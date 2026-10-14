// SPDX-License-Identifier: MIT

//! Provider-independent ingestion of normalized GB interval documents.
//!
//! # Purpose
//!
//! Resolve GB resource relationships and persist canonical intervals while
//! preserving revisions only for observations that have changed.
//!
//! # Transaction boundary
//!
//! The supported import entry point commits all file resource and revision
//! writes as a single MariaDB transaction. The ingestion-run record is
//! created separately by its caller.

use crate::{AcquisitionSourceId, IngestionRunId, StoreError};
use gb::{GbDocument, IntervalBlockRecord, MeterReadingRecord};
use sqlx::{
    MySql, Pool,
    types::chrono::{DateTime, NaiveDateTime, Utc},
};
use thiserror::Error;

/// Error returned when source relationships cannot be imported safely.
#[derive(Debug, Error)]
pub enum GbImportError {
    /// Source document is outside the supported ingestion shape.
    #[error("unsupported or ambiguous GB resource relationship")]
    InvalidRelationships,
    /// A source timestamp cannot be represented in canonical UTC storage.
    #[error("GB timestamp cannot be represented in UTC")]
    InvalidTimestamp,
    /// A database operation failed.
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Number of observations handled by one import.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GbImportReport {
    /// Total intervals across all supported meters and blocks.
    pub observations: usize,
    /// Number of observations matching the accepted revision.
    pub unchanged: usize,
    /// Number of observations for which a revision was accepted.
    /// Includes initial observations of previously unseen intervals.
    pub accepted: usize,
}

/// A fully validated interval with its canonical UTC start.
struct PreparedReading<'a> {
    source: &'a gb::IntervalReadingRecord,
    start: NaiveDateTime,
}

/// One resolved and validated block, with no database identifiers yet.
struct PreparedBlock<'a> {
    source: &'a IntervalBlockRecord,
    start: NaiveDateTime,
    published: Option<NaiveDateTime>,
    updated: Option<NaiveDateTime>,
    readings: Vec<PreparedReading<'a>>,
}

/// A Meter Reading with its unambiguous parent resources and blocks.
struct PreparedMeter<'a> {
    meter: &'a MeterReadingRecord,
    usage: &'a gb::UsagePointRecord,
    reading_type: &'a gb::ReadingTypeRecord,
    blocks: Vec<PreparedBlock<'a>>,
}

/// Converts Unix seconds to canonical UTC without a database operation.
fn utc(seconds: i64) -> Result<NaiveDateTime, GbImportError> {
    DateTime::<Utc>::from_timestamp(seconds, 0)
        .map(|dt| dt.naive_utc())
        .ok_or(GbImportError::InvalidTimestamp)
}

/// Validates the complete supported GB relationship graph before writes.
///
/// Every Meter Reading needs one matching Usage Point and Reading Type.
/// Every Interval Block must belong to exactly one Meter Reading. This
/// avoids silently skipping malformed or unmatched source resources.
fn prepare(
    document: &GbDocument,
) -> Result<Vec<PreparedMeter<'_>>, GbImportError> {
    if document.meter_readings.is_empty() || document.interval_blocks.is_empty()
    {
        return Err(GbImportError::InvalidRelationships);
    }
    let mut prepared = Vec::with_capacity(document.meter_readings.len());
    let mut assigned_blocks = 0;

    for meter in &document.meter_readings {
        let up_href = meter
            .up_href
            .as_deref()
            .ok_or(GbImportError::InvalidRelationships)?;
        let mut usages = document.usage_points.iter().filter(|item| {
            item.related_hrefs.iter().any(|href| href == up_href)
        });
        let usage = usages.next().ok_or(GbImportError::InvalidRelationships)?;
        if usages.next().is_some() {
            return Err(GbImportError::InvalidRelationships);
        }

        let mut types = document
            .reading_types
            .iter()
            .filter(|item| meter.related_hrefs.contains(&item.external_id));
        let reading_type =
            types.next().ok_or(GbImportError::InvalidRelationships)?;
        if types.next().is_some() {
            return Err(GbImportError::InvalidRelationships);
        }

        let mut blocks = Vec::new();
        for block in &document.interval_blocks {
            if !meter.related_hrefs.contains(&block.up_href) {
                continue;
            }
            if block.readings.is_empty() || block.duration_seconds == 0 {
                return Err(GbImportError::InvalidRelationships);
            }
            let mut readings = Vec::with_capacity(block.readings.len());
            for reading in &block.readings {
                if reading.duration_seconds == 0 {
                    return Err(GbImportError::InvalidRelationships);
                }
                readings.push(PreparedReading {
                    source: reading,
                    start: utc(reading.start_unix)?,
                });
            }
            blocks.push(PreparedBlock {
                source: block,
                start: utc(block.start_unix)?,
                published: block.published_unix.map(utc).transpose()?,
                updated: block.updated_unix.map(utc).transpose()?,
                readings,
            });
            assigned_blocks += 1;
        }
        if blocks.is_empty() {
            return Err(GbImportError::InvalidRelationships);
        }
        prepared.push(PreparedMeter { meter, usage, reading_type, blocks });
    }

    // Distinct source blocks cannot be assigned to multiple meters, and
    // none may be ignored because its parent is absent from the export.
    if assigned_blocks != document.interval_blocks.len() {
        return Err(GbImportError::InvalidRelationships);
    }
    Ok(prepared)
}

/// Imports one fully validated GB document in a single MariaDB transaction.
///
/// The source row is locked first, serializing imports for that source.
/// A failed SQL statement drops the transaction without committing changes.
/// The caller must not share a `run_id` across different sources.
pub async fn import_gb_document_atomic(
    pool: &Pool<MySql>,
    source_id: AcquisitionSourceId,
    run_id: IngestionRunId,
    document: &GbDocument,
) -> Result<GbImportReport, GbImportError> {
    let meters = prepare(document)?;
    let mut tx = pool.begin().await.map_err(StoreError::from)?;

    // Lock one stable parent row for this source before reading or writing.
    // This serializes cooperating importers without a global lock.
    let source: Option<u64> = sqlx::query_scalar(
        "SELECT id FROM acquisition_source WHERE id = ? FOR UPDATE",
    )
    .bind(source_id.0)
    .fetch_optional(&mut *tx)
    .await
    .map_err(StoreError::from)?;
    if source.is_none() {
        return Err(GbImportError::Store(StoreError::NotFound));
    }
    let run_source: Option<u64> = sqlx::query_scalar(
        "SELECT acquisition_source_id FROM ingestion_run WHERE id = ?",
    )
    .bind(run_id.0)
    .fetch_optional(&mut *tx)
    .await
    .map_err(StoreError::from)?;
    if run_source != Some(source_id.0) {
        return Err(GbImportError::InvalidRelationships);
    }

    let mut report = GbImportReport::default();
    for linked in meters {
        let up = sqlx::query(
            "INSERT INTO usage_point \
             (acquisition_source_id, external_id, service_category_kind) \
             VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE \
             id = LAST_INSERT_ID(id)",
        )
        .bind(source_id.0)
        .bind(&linked.usage.external_id)
        .bind(linked.usage.service_category_kind)
        .execute(&mut *tx)
        .await
        .map_err(StoreError::from)?
        .last_insert_id();

        let t = linked.reading_type;
        let rt = sqlx::query(
            "INSERT INTO reading_type (acquisition_source_id, external_id, \
             accumulation_behaviour, commodity, data_qualifier, \
             flow_direction, kind, phase, power_of_ten_multiplier, \
             unit_of_measure) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE id = LAST_INSERT_ID(id)",
        )
        .bind(source_id.0)
        .bind(&t.external_id)
        .bind(t.accumulation_behaviour)
        .bind(t.commodity)
        .bind(t.data_qualifier)
        .bind(t.flow_direction)
        .bind(t.kind)
        .bind(t.phase)
        .bind(t.power_of_ten_multiplier)
        .bind(t.unit_of_measure)
        .execute(&mut *tx)
        .await
        .map_err(StoreError::from)?
        .last_insert_id();

        let meter = sqlx::query(
            "INSERT INTO meter_reading \
             (usage_point_id, reading_type_id, external_id) \
             VALUES (?, ?, ?) \
             ON DUPLICATE KEY UPDATE id = LAST_INSERT_ID(id)",
        )
        .bind(up)
        .bind(rt)
        .bind(&linked.meter.external_id)
        .execute(&mut *tx)
        .await
        .map_err(StoreError::from)?
        .last_insert_id();

        // A stable external identity must never silently change parent type.
        let existing_type: u64 = sqlx::query_scalar(
            "SELECT reading_type_id FROM meter_reading WHERE id = ?",
        )
        .bind(meter)
        .fetch_one(&mut *tx)
        .await
        .map_err(StoreError::from)?;
        if existing_type != rt {
            return Err(GbImportError::InvalidRelationships);
        }

        for block in linked.blocks {
            let block_id = sqlx::query(
                "INSERT INTO interval_block \
                 (meter_reading_id, external_id, interval_start, \
                 interval_duration_seconds) VALUES (?, ?, ?, ?) \
                 ON DUPLICATE KEY UPDATE id = LAST_INSERT_ID(id)",
            )
            .bind(meter)
            .bind(&block.source.external_id)
            .bind(block.start)
            .bind(block.source.duration_seconds)
            .execute(&mut *tx)
            .await
            .map_err(StoreError::from)?
            .last_insert_id();

            for prepared in block.readings {
                let reading = prepared.source;
                let id = sqlx::query(
                    "INSERT INTO interval_reading \
                     (meter_reading_id, interval_start, \
                     interval_duration_seconds) VALUES (?, ?, ?) \
                     ON DUPLICATE KEY UPDATE id = LAST_INSERT_ID(id)",
                )
                .bind(meter)
                .bind(prepared.start)
                .bind(reading.duration_seconds)
                .execute(&mut *tx)
                .await
                .map_err(StoreError::from)?
                .last_insert_id();

                // The accepted pointer is locked for the compare-and-swap.
                let accepted: Option<u64> = sqlx::query_scalar(
                    "SELECT accepted_revision_id FROM interval_reading \
                     WHERE id = ? FOR UPDATE",
                )
                .bind(id)
                .fetch_one(&mut *tx)
                .await
                .map_err(StoreError::from)?;
                let quality = reading.quality.map(|q| q.to_string());
                let unchanged = if let Some(revision_id) = accepted {
                    let existing = sqlx::query_as::<
                        _,
                        (
                            i64,
                            Option<String>,
                            Option<u64>,
                            Option<String>,
                            Option<NaiveDateTime>,
                            Option<NaiveDateTime>,
                        ),
                    >(
                        "SELECT reading_value, reading_quality, \
                         interval_block_id, source_external_id, \
                         source_published_at, source_updated_at \
                         FROM interval_reading_revision WHERE id = ?",
                    )
                    .bind(revision_id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(StoreError::from)?;
                    existing.0 == reading.value
                        && existing.1 == quality
                        && existing.2 == Some(block_id)
                        && existing.3.as_deref()
                            == Some(block.source.external_id.as_str())
                        && existing.4 == block.published
                        && existing.5 == block.updated
                } else {
                    false
                };
                report.observations += 1;
                if unchanged {
                    report.unchanged += 1;
                    continue;
                }
                let revision = sqlx::query(
                    "INSERT INTO interval_reading_revision \
                     (interval_reading_id, ingestion_run_id, \
                     interval_block_id, source_external_id, \
                     source_published_at, source_updated_at, \
                     reading_value, reading_quality) \
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(id)
                .bind(run_id.0)
                .bind(block_id)
                .bind(&block.source.external_id)
                .bind(block.published)
                .bind(block.updated)
                .bind(reading.value)
                .bind(quality)
                .execute(&mut *tx)
                .await
                .map_err(StoreError::from)?
                .last_insert_id();
                sqlx::query(
                    "UPDATE interval_reading \
                     SET accepted_revision_id = ? WHERE id = ?",
                )
                .bind(revision)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(StoreError::from)?;
                report.accepted += 1;
            }
        }
    }
    tx.commit().await.map_err(StoreError::from)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_document_without_database_access() {
        let doc = GbDocument {
            usage_points: vec![],
            reading_types: vec![],
            meter_readings: vec![],
            interval_blocks: vec![],
        };
        assert!(matches!(
            prepare(&doc),
            Err(GbImportError::InvalidRelationships)
        ));
    }

    #[test]
    fn rejects_orphan_block_before_database_access() {
        let doc = GbDocument {
            usage_points: vec![gb::UsagePointRecord {
                external_id: "usage".into(),
                related_hrefs: vec!["meter-parent".into()],
                service_category_kind: None,
            }],
            reading_types: vec![gb::ReadingTypeRecord {
                external_id: "type".into(),
                accumulation_behaviour: None,
                commodity: None,
                data_qualifier: None,
                flow_direction: None,
                kind: None,
                phase: None,
                power_of_ten_multiplier: None,
                unit_of_measure: None,
                interval_length_seconds: None,
            }],
            meter_readings: vec![gb::MeterReadingRecord {
                external_id: "meter".into(),
                up_href: Some("meter-parent".into()),
                related_hrefs: vec!["type".into(), "block-collection".into()],
            }],
            interval_blocks: vec![gb::IntervalBlockRecord {
                external_id: "orphan".into(),
                up_href: "unmatched".into(),
                start_unix: 0,
                duration_seconds: 3600,
                published_unix: None,
                updated_unix: None,
                readings: vec![gb::IntervalReadingRecord {
                    start_unix: 0,
                    duration_seconds: 3600,
                    value: 0,
                    quality: None,
                }],
            }],
        };
        assert!(matches!(
            prepare(&doc),
            Err(GbImportError::InvalidRelationships)
        ));
    }

    #[test]
    fn rejects_unrepresentable_timestamp() {
        assert!(matches!(utc(i64::MAX), Err(GbImportError::InvalidTimestamp)));
    }
}
