// SPDX-License-Identifier: MIT

//! Deterministic unit-style coverage without a live database.

use energy_store::{StreamCandidate, StreamIssueKind, resolve_candidates};
use sqlx::types::chrono::NaiveDateTime;

fn candidate(id: u64, hour: u32, duration: u32, value: i64) -> StreamCandidate {
    let start = NaiveDateTime::parse_from_str(
        &format!("2026-01-01 {hour:02}:00:00"),
        "%Y-%m-%d %H:%M:%S",
    )
    .unwrap();
    StreamCandidate {
        meter_reading_id: id,
        interval_reading_id: id,
        accepted_revision_id: Some(id + 100),
        interval_start: start,
        interval_duration_seconds: duration,
        reading_value: Some(value),
        reading_quality: Some("verified".into()),
        accumulation_behaviour: Some(4),
        commodity: Some(1),
        data_qualifier: Some(0),
        flow_direction: Some(1),
        kind: Some(12),
        phase: Some(0),
        power_of_ten_multiplier: Some(0),
        unit_of_measure: Some(72),
    }
}

#[test]
fn identical_resources_are_not_double_counted() {
    let result = resolve_candidates(vec![
        candidate(2, 0, 3600, 500),
        candidate(1, 0, 3600, 500),
        candidate(3, 1, 3600, 600),
    ]);
    assert!(result.issues.is_empty());
    assert_eq!(result.candidate_count, 3);
    assert_eq!(result.intervals.len(), 2);
    assert_eq!(result.intervals[0].meter_reading_id, 1);
}

#[test]
fn differing_duplicate_values_block_entire_selection() {
    let result = resolve_candidates(vec![
        candidate(1, 0, 3600, 500),
        candidate(2, 0, 3600, 501),
    ]);
    assert!(result.intervals.is_empty());
    assert_eq!(result.issues[0].kind, StreamIssueKind::ConflictingDuplicate);
}

#[test]
fn overlapping_cadences_block_entire_selection() {
    let result = resolve_candidates(vec![
        candidate(1, 0, 86400, 500),
        candidate(2, 1, 3600, 200),
    ]);
    assert!(result.intervals.is_empty());
    assert_eq!(result.issues[0].kind, StreamIssueKind::OverlappingIntervals);
}

#[test]
fn adjacent_intervals_are_valid() {
    let result = resolve_candidates(vec![
        candidate(1, 0, 3600, 500),
        candidate(2, 1, 3600, 200),
    ]);
    assert_eq!(result.intervals.len(), 2);
    assert!(result.issues.is_empty());
}

#[test]
fn incompatible_units_block_entire_selection() {
    let mut second = candidate(2, 1, 3600, 200);
    second.unit_of_measure = Some(99);
    let result = resolve_candidates(vec![candidate(1, 0, 3600, 500), second]);
    assert!(result.intervals.is_empty());
    assert_eq!(
        result.issues[0].kind,
        StreamIssueKind::IncompatibleMeasurementType
    );
}

#[test]
fn missing_accepted_revision_blocks_selection() {
    let mut second = candidate(2, 1, 3600, 200);
    second.accepted_revision_id = None;
    second.reading_value = None;
    let result = resolve_candidates(vec![candidate(1, 0, 3600, 500), second]);
    assert!(result.intervals.is_empty());
    assert_eq!(result.issues[0].kind, StreamIssueKind::MissingAcceptedRevision);
}

#[test]
fn missing_measurement_fields_block_selection() {
    let mut first = candidate(1, 0, 3600, 500);
    first.kind = None;
    let result = resolve_candidates(vec![first]);
    assert_eq!(
        result.issues[0].kind,
        StreamIssueKind::IncompleteMeasurementType
    );
}

#[test]
fn empty_input_is_not_claimed_to_have_usage() {
    let result = resolve_candidates(vec![]);
    assert!(result.intervals.is_empty());
    assert!(result.issues.is_empty());
    assert_eq!(result.candidate_count, 0);
}

#[test]
fn reverse_export_is_not_considered_consumption() {
    let mut export = candidate(2, 1, 3600, 900);
    export.flow_direction = Some(19);
    let result = resolve_candidates(vec![candidate(1, 0, 3600, 400), export]);
    assert!(result.intervals.is_empty());
    assert_eq!(result.issues.len(), 1);
    assert_eq!(
        result.issues[0].kind,
        StreamIssueKind::UnsupportedElectricityFlow
    );
}

#[test]
fn export_only_is_not_treated_as_consumption() {
    let mut export = candidate(1, 0, 3600, 1200);
    export.flow_direction = Some(19);
    let result = resolve_candidates(vec![export]);
    assert!(result.intervals.is_empty());
    assert_eq!(
        result.issues[0].kind,
        StreamIssueKind::UnsupportedElectricityFlow
    );
}

#[test]
fn ambiguous_electricity_flow_is_not_consumption() {
    let mut net = candidate(1, 0, 3600, 500);
    net.flow_direction = Some(20);
    let result = resolve_candidates(vec![net]);
    assert!(result.intervals.is_empty());
    assert_eq!(
        result.issues[0].kind,
        StreamIssueKind::UnsupportedElectricityFlow
    );
}

#[test]
fn natural_gas_is_not_reclassified_as_solar() {
    let mut gas = candidate(1, 0, 86400, 3);
    gas.commodity = Some(7);
    gas.flow_direction = Some(0);
    gas.unit_of_measure = Some(169);
    let result = resolve_candidates(vec![gas]);
    assert_eq!(result.intervals.len(), 1);
    assert!(result.issues.is_empty());
}

#[test]
fn forward_electricity_still_resolves() {
    let result = resolve_candidates(vec![candidate(1, 0, 3600, 500)]);
    assert_eq!(result.intervals.len(), 1);
    assert!(result.issues.is_empty());
}
