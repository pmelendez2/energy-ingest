// SPDX-License-Identifier: MIT

//! Conservative read-only selection of physical-meter interval streams.
//!
//! No usage totals are emitted until source metadata and overlapping
//! intervals are reconciled. Raw values and revision IDs are preserved.

use crate::StoreError;
use sha2::{Digest, Sha256};
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
    UnsupportedElectricityFlow,
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

/// Validation state shared by CLI and API consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationStatus {
    NoData,
    Validated,
    Unresolved,
}

impl ValidationStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoData => "no_data",
            Self::Validated => "validated",
            Self::Unresolved => "unresolved",
        }
    }
}

impl StreamIssueKind {
    /// Machine-readable identifier shared by all presentation surfaces.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingAcceptedRevision => "missing_accepted_revision",
            Self::IncompleteMeasurementType => "incomplete_measurement_type",
            Self::UnsupportedElectricityFlow => "unsupported_electricity_flow",
            Self::IncompatibleMeasurementType => {
                "incompatible_measurement_type"
            }
            Self::ConflictingDuplicate => "conflicting_duplicate",
            Self::OverlappingIntervals => "overlapping_intervals",
            Self::InvalidDuration => "invalid_duration",
        }
    }
}

/// Deterministic summary of a physical meter's existing stream resolution.
///
/// No totals, inferred gaps, or partial intervals are exposed. The summary
/// is not a global change cursor or a snapshot-consistent revision token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeterValidation {
    pub physical_meter_id: u64,
    pub status: ValidationStatus,
    pub candidate_count: usize,
    pub selected_interval_count: usize,
    pub content_fingerprint: String,
    pub issues: Vec<StreamIssue>,
}

/// Derive canonical validation facts from the same result used by the API.
pub fn summarize_meter_validation(
    physical_meter_id: u64,
    result: &StreamResolution,
) -> MeterValidation {
    let status = if !result.issues.is_empty() {
        ValidationStatus::Unresolved
    } else if result.candidate_count == 0 {
        ValidationStatus::NoData
    } else {
        ValidationStatus::Validated
    };
    MeterValidation {
        physical_meter_id,
        status,
        candidate_count: result.candidate_count,
        selected_interval_count: result.intervals.len(),
        content_fingerprint: validation_fingerprint(physical_meter_id, result),
        issues: result.issues.clone(),
    }
}

/// Content identity for one meter-resolution result, not a database cursor.
///
/// Changes to accepted values, revision provenance, selected intervals, or
/// validation issues change the digest. It must be read again to detect
/// changes; concurrent database writes are not snapshot-isolated here.
pub fn validation_fingerprint(
    physical_meter_id: u64,
    result: &StreamResolution,
) -> String {
    let mut hash = Sha256::new();
    hash.update(b"energy-ingest-meter-validation-v2\0");
    hash.update(physical_meter_id.to_be_bytes());
    hash.update((result.candidate_count as u64).to_be_bytes());
    let mut intervals = result.intervals.clone();
    intervals
        .sort_by_key(|i| (i.start, i.duration_seconds, i.meter_reading_id));
    hash.update((intervals.len() as u64).to_be_bytes());
    for interval in intervals {
        hash.update(interval.start.and_utc().timestamp().to_be_bytes());
        hash.update(interval.duration_seconds.to_be_bytes());
        hash.update(interval.raw_value.to_be_bytes());
        hash.update(interval.meter_reading_id.to_be_bytes());
        hash.update(interval.accepted_revision_id.to_be_bytes());
        hash.update(interval.power_of_ten_multiplier.to_be_bytes());
        hash.update(interval.unit_of_measure.to_be_bytes());
    }
    let mut issues = result.issues.clone();
    issues.sort_by_key(|i| {
        (i.kind.as_str(), i.interval_reading_id, i.other_interval_reading_id)
    });
    hash.update((issues.len() as u64).to_be_bytes());
    for issue in issues {
        let kind = issue.kind.as_str().as_bytes();
        hash.update((kind.len() as u64).to_be_bytes());
        hash.update(kind);
        hash.update(issue.interval_reading_id.to_be_bytes());
        match issue.other_interval_reading_id {
            Some(id) => {
                hash.update([1]);
                hash.update(id.to_be_bytes());
            }
            None => hash.update([0]),
        }
    }
    format!("{:x}", hash.finalize())
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
        // Grid exports and ambiguous electricity flow are not consumption.
        // Retain the raw observations in storage for a future export API.
        if candidate.commodity == Some(1) && candidate.flow_direction != Some(1)
        {
            issue(
                &mut result,
                StreamIssueKind::UnsupportedElectricityFlow,
                candidate.interval_reading_id,
                None,
            );
            continue;
        }
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

#[cfg(test)]
mod validation_contract_tests {
    use super::*;

    #[test]
    fn empty_stream_is_not_validated() {
        let summary =
            summarize_meter_validation(2, &StreamResolution::default());
        assert_eq!(summary.physical_meter_id, 2);
        assert_eq!(summary.status.as_str(), "no_data");
        assert_eq!(summary.selected_interval_count, 0);
    }

    #[test]
    fn validated_stream_only_counts_selected_intervals() {
        let sample = SelectedInterval {
            start: NaiveDateTime::parse_from_str(
                "2026-01-01T00:00:00",
                "%Y-%m-%dT%H:%M:%S",
            )
            .unwrap(),
            duration_seconds: 3600,
            raw_value: 25,
            meter_reading_id: 1,
            accepted_revision_id: 3,
            power_of_ten_multiplier: 0,
            unit_of_measure: 72,
        };
        let resolution = StreamResolution {
            candidate_count: 2,
            intervals: vec![sample],
            issues: Vec::new(),
        };
        let summary = summarize_meter_validation(5, &resolution);
        assert_eq!(summary.status, ValidationStatus::Validated);
        assert_eq!(summary.candidate_count, 2);
        assert_eq!(summary.selected_interval_count, 1);
    }

    #[test]
    fn unresolved_stream_does_not_certify_partial_intervals() {
        let resolution = StreamResolution {
            candidate_count: 2,
            intervals: Vec::new(),
            issues: vec![StreamIssue {
                kind: StreamIssueKind::ConflictingDuplicate,
                interval_reading_id: 9,
                other_interval_reading_id: Some(8),
            }],
        };
        let summary = summarize_meter_validation(5, &resolution);
        assert_eq!(summary.status, ValidationStatus::Unresolved);
        assert_eq!(summary.selected_interval_count, 0);
        assert_eq!(summary.issues[0].kind.as_str(), "conflicting_duplicate");
    }

    #[test]
    fn all_issue_names_stable() {
        let kinds = [
            StreamIssueKind::MissingAcceptedRevision,
            StreamIssueKind::IncompleteMeasurementType,
            StreamIssueKind::UnsupportedElectricityFlow,
            StreamIssueKind::IncompatibleMeasurementType,
            StreamIssueKind::ConflictingDuplicate,
            StreamIssueKind::OverlappingIntervals,
            StreamIssueKind::InvalidDuration,
        ];
        let mut names = std::collections::HashSet::new();
        for kind in kinds {
            assert!(names.insert(kind.as_str()));
            assert!(!kind.as_str().is_empty());
        }
    }

    #[test]
    fn validation_fingerprint_is_stable_for_same_resolution() {
        let resolution = StreamResolution::default();
        let first = summarize_meter_validation(1, &resolution);
        let second = summarize_meter_validation(1, &resolution);
        assert_eq!(first.content_fingerprint, second.content_fingerprint);
        assert_eq!(first.content_fingerprint.len(), 64);
        assert_ne!(
            first.content_fingerprint,
            summarize_meter_validation(2, &resolution).content_fingerprint
        );
    }

    #[test]
    fn validation_fingerprint_changes_when_issues_change() {
        let mut resolution = StreamResolution {
            candidate_count: 2,
            intervals: Vec::new(),
            issues: vec![StreamIssue {
                kind: StreamIssueKind::ConflictingDuplicate,
                interval_reading_id: 42,
                other_interval_reading_id: Some(41),
            }],
        };
        let original = validation_fingerprint(1, &resolution);
        resolution.issues[0].interval_reading_id = 43;
        assert_ne!(original, validation_fingerprint(1, &resolution));
    }
}
