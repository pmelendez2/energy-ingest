// SPDX-License-Identifier: MIT

//! Conservative read-only selection of physical-meter interval streams.
//!
//! No usage totals are emitted until source metadata and overlapping
//! intervals are reconciled. Raw values and revision IDs are preserved.

use crate::StoreError;
use sqlx::{FromRow, MySql, Pool, types::chrono::NaiveDateTime};

/// A source interval with its currently accepted revision and measurement type.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct StreamCandidate {
    pub meter_reading_id: u64,
    pub interval_reading_id: u64,
    pub accepted_revision_id: Option<u64>,
    pub interval_start: NaiveDateTime,
    pub interval_duration_seconds: u32,
    pub reading_value: Option<i64>,
    pub reading_quality: Option<String>,
    pub accumulation_behaviour: Option<u32>,
    pub commodity: Option<u32>,
    pub data_qualifier: Option<u32>,
    pub flow_direction: Option<u32>,
    pub kind: Option<u32>,
    pub phase: Option<u32>,
    pub power_of_ten_multiplier: Option<i16>,
    pub unit_of_measure: Option<u32>,
}

/// An accepted source interval, retaining the raw scale and provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedInterval {
    pub start: NaiveDateTime,
    pub duration_seconds: u32,
    pub raw_value: i64,
    pub meter_reading_id: u64,
    pub accepted_revision_id: u64,
    pub power_of_ten_multiplier: i16,
    pub unit_of_measure: u32,
}

/// Reason that candidate observations cannot form an unambiguous stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamIssueKind {
    MissingAcceptedRevision,
    IncompleteMeasurementType,
    IncompatibleMeasurementType,
    ConflictingDuplicate,
    OverlappingIntervals,
    InvalidDuration,
}

/// Issue associated with one or two source interval identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamIssue {
    pub kind: StreamIssueKind,
    pub interval_reading_id: u64,
    pub other_interval_reading_id: Option<u64>,
}

/// Read-only resolution of a customer-approved physical meter association.
///
/// `intervals` is empty whenever `issues` is nonempty: consumers must not
/// accidentally interpret a partial selection as validated usage.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamResolution {
    pub candidate_count: usize,
    pub intervals: Vec<SelectedInterval>,
    pub issues: Vec<StreamIssue>,
}

/// Loads accepted observations for explicitly attached resources.
///
/// The query never joins rejected revisions and never mutates storage.
/// `NotFound` distinguishes an absent meter from a meter with no readings.
pub async fn resolve_physical_meter_stream(
    pool: &Pool<MySql>,
    physical_meter_id: u64,
) -> Result<StreamResolution, StoreError> {
    let exists: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM physical_meter WHERE id = ?")
            .bind(physical_meter_id)
            .fetch_optional(pool)
            .await?;
    if exists.is_none() {
        return Err(StoreError::NotFound);
    }

    let candidates = sqlx::query_as::<_, StreamCandidate>(
        r#"
        SELECT mr.id AS meter_reading_id,
               ir.id AS interval_reading_id,
               ir.accepted_revision_id,
               ir.interval_start, ir.interval_duration_seconds,
               rev.reading_value, rev.reading_quality,
               rt.accumulation_behaviour, rt.commodity,
               rt.data_qualifier, rt.flow_direction,
               rt.kind, rt.phase, rt.power_of_ten_multiplier,
               rt.unit_of_measure
        FROM physical_meter_resource AS pmr
        JOIN meter_reading AS mr
          ON mr.id = pmr.meter_reading_id
        JOIN reading_type AS rt
          ON rt.id = mr.reading_type_id
        JOIN interval_reading AS ir
          ON ir.meter_reading_id = mr.id
        LEFT JOIN interval_reading_revision AS rev
          ON rev.id = ir.accepted_revision_id
        WHERE pmr.physical_meter_id = ?
        ORDER BY ir.interval_start, ir.interval_duration_seconds,
                 ir.id
        "#,
    )
    .bind(physical_meter_id)
    .fetch_all(pool)
    .await?;

    Ok(resolve_candidates(candidates))
}

/// Upper bound on candidates evaluated for one historical window.
pub const MAX_WINDOW_CANDIDATES: i64 = 10_000;

/// Resolves readings beginning inside a half-open UTC window.
///
/// A limit is enforced *before* resolution: an overfull window fails rather
/// than returning a partial stream or silently truncating duplicates.
/// Callers must request smaller windows if the bound is exceeded.
/// This is not a revision-watermark or snapshot-consistent retrieval API.
pub async fn resolve_meter_window(
    pool: &Pool<MySql>,
    physical_meter_id: u64,
    start: NaiveDateTime,
    end: NaiveDateTime,
) -> Result<Option<StreamResolution>, StoreError> {
    let exists: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM physical_meter WHERE id = ?")
            .bind(physical_meter_id)
            .fetch_optional(pool)
            .await?;
    if exists.is_none() {
        return Ok(None);
    }

    let candidates = sqlx::query_as::<_, StreamCandidate>(
        r#"
        SELECT mr.id AS meter_reading_id,
               ir.id AS interval_reading_id,
               ir.accepted_revision_id,
               ir.interval_start, ir.interval_duration_seconds,
               rev.reading_value, rev.reading_quality,
               rt.accumulation_behaviour, rt.commodity,
               rt.data_qualifier, rt.flow_direction,
               rt.kind, rt.phase, rt.power_of_ten_multiplier,
               rt.unit_of_measure
        FROM physical_meter_resource AS pmr
        JOIN meter_reading AS mr
          ON mr.id = pmr.meter_reading_id
        JOIN reading_type AS rt
          ON rt.id = mr.reading_type_id
        JOIN interval_reading AS ir
          ON ir.meter_reading_id = mr.id
        LEFT JOIN interval_reading_revision AS rev
          ON rev.id = ir.accepted_revision_id
        WHERE pmr.physical_meter_id = ?
          AND ir.interval_start >= ?
          AND ir.interval_start < ?
        ORDER BY ir.interval_start, ir.interval_duration_seconds,
                 ir.id
        LIMIT ?
        "#,
    )
    .bind(physical_meter_id)
    .bind(start)
    .bind(end)
    .bind(MAX_WINDOW_CANDIDATES + 1)
    .fetch_all(pool)
    .await?;
    if candidates.len() > MAX_WINDOW_CANDIDATES as usize {
        return Ok(Some(StreamResolution {
            candidate_count: candidates.len(),
            ..StreamResolution::default()
        }));
    }
    Ok(Some(resolve_candidates(candidates)))
}

/// Selects exact duplicates once; refuses ambiguous or overlapping data.
///
/// This intentionally does not resample hourly/daily/monthly sources,
/// choose quality winners, or sum values across resource boundaries.
/// A future policy must explicitly authorize those operations.
pub fn resolve_candidates(
    mut candidates: Vec<StreamCandidate>,
) -> StreamResolution {
    candidates.sort_by_key(|c| {
        (c.interval_start, c.interval_duration_seconds, c.interval_reading_id)
    });
    let mut result = StreamResolution {
        candidate_count: candidates.len(),
        ..StreamResolution::default()
    };
    let mut selected: Vec<(&StreamCandidate, NaiveDateTime)> = Vec::new();
    let mut signature: Option<[i64; 8]> = None;

    for candidate in &candidates {
        if candidate.accepted_revision_id.is_none()
            || candidate.reading_value.is_none()
        {
            issue(
                &mut result,
                StreamIssueKind::MissingAcceptedRevision,
                candidate.interval_reading_id,
                None,
            );
            continue;
        }
        if candidate.interval_duration_seconds == 0 {
            issue(
                &mut result,
                StreamIssueKind::InvalidDuration,
                candidate.interval_reading_id,
                None,
            );
            continue;
        }
        let Some(end) = candidate.interval_start.checked_add_signed(
            chrono::Duration::seconds(i64::from(
                candidate.interval_duration_seconds,
            )),
        ) else {
            issue(
                &mut result,
                StreamIssueKind::InvalidDuration,
                candidate.interval_reading_id,
                None,
            );
            continue;
        };
        let fields = [
            candidate.accumulation_behaviour.map(i64::from),
            candidate.commodity.map(i64::from),
            candidate.data_qualifier.map(i64::from),
            candidate.flow_direction.map(i64::from),
            candidate.kind.map(i64::from),
            candidate.phase.map(i64::from),
            candidate.power_of_ten_multiplier.map(i64::from),
            candidate.unit_of_measure.map(i64::from),
        ];
        let Some(metadata) = fields.into_iter().collect::<Option<Vec<i64>>>()
        else {
            issue(
                &mut result,
                StreamIssueKind::IncompleteMeasurementType,
                candidate.interval_reading_id,
                None,
            );
            continue;
        };
        let metadata: [i64; 8] = metadata.try_into().expect("eight fields");
        if let Some(first) = signature {
            if first != metadata {
                issue(
                    &mut result,
                    StreamIssueKind::IncompatibleMeasurementType,
                    candidate.interval_reading_id,
                    None,
                );
                continue;
            }
        } else {
            signature = Some(metadata);
        }
        // An interval is a half-open range [start, end).
        if let Some((previous, previous_end)) = selected.last()
            && candidate.interval_start < *previous_end
        {
            let exact = candidate.interval_start == previous.interval_start
                && end == *previous_end;
            if exact
                && candidate.reading_value == previous.reading_value
                && candidate.reading_quality == previous.reading_quality
            {
                continue;
            }
            issue(
                &mut result,
                if exact {
                    StreamIssueKind::ConflictingDuplicate
                } else {
                    StreamIssueKind::OverlappingIntervals
                },
                candidate.interval_reading_id,
                Some(previous.interval_reading_id),
            );
            continue;
        }
        selected.push((candidate, end));
    }
    if result.issues.is_empty() {
        result.intervals = selected
            .into_iter()
            .map(|(c, _)| SelectedInterval {
                start: c.interval_start,
                duration_seconds: c.interval_duration_seconds,
                raw_value: c.reading_value.expect("validated"),
                meter_reading_id: c.meter_reading_id,
                accepted_revision_id: c
                    .accepted_revision_id
                    .expect("validated"),
                power_of_ten_multiplier: c
                    .power_of_ten_multiplier
                    .expect("validated"),
                unit_of_measure: c.unit_of_measure.expect("validated"),
            })
            .collect();
    }
    result
}

fn issue(
    result: &mut StreamResolution,
    kind: StreamIssueKind,
    interval_reading_id: u64,
    other_interval_reading_id: Option<u64>,
) {
    result.issues.push(StreamIssue {
        kind,
        interval_reading_id,
        other_interval_reading_id,
    });
}
