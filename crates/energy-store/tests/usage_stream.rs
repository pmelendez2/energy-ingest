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
        source_published_at: None,
        source_updated_at: None,
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

fn dated(mut c: StreamCandidate, day: u32) -> StreamCandidate {
    c.source_updated_at = Some(
        NaiveDateTime::parse_from_str(
            &format!("2026-10-{day:02} 12:00:00"),
            "%Y-%m-%d %H:%M:%S",
        )
        .unwrap(),
    );
    c
}

#[test]
fn newer_source_revision_wins_independent_of_resource_id() {
    let result = resolve_candidates(vec![
        dated(candidate(20, 0, 3600, 150), 6),
        dated(candidate(3, 0, 3600, 175), 27),
    ]);
    assert!(result.issues.is_empty());
    assert_eq!(result.intervals[0].raw_value, 175);
    assert_eq!(result.intervals[0].meter_reading_id, 3);
}

#[test]
fn late_import_of_older_source_does_not_reverse_correction() {
    let result = resolve_candidates(vec![
        dated(candidate(3, 0, 3600, 175), 27),
        dated(candidate(99, 0, 3600, 150), 6),
    ]);
    assert!(result.issues.is_empty());
    assert_eq!(result.intervals[0].raw_value, 175);
}

#[test]
fn unknown_or_equal_source_order_blocks_differing_values() {
    let unknown = resolve_candidates(vec![
        candidate(1, 0, 3600, 150),
        dated(candidate(2, 0, 3600, 175), 27),
    ]);
    assert!(unknown.intervals.is_empty());
    let equal = resolve_candidates(vec![
        dated(candidate(1, 0, 3600, 150), 27),
        dated(candidate(2, 0, 3600, 175), 27),
    ]);
    assert!(equal.intervals.is_empty());
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

fn gas_day(id: u64, utc: &str, value: i64) -> StreamCandidate {
    let mut c = candidate(id, 0, 86_400, value);
    c.interval_start =
        NaiveDateTime::parse_from_str(utc, "%Y-%m-%d %H:%M:%S").unwrap();
    c.commodity = Some(7);
    c.unit_of_measure = Some(169);
    c.flow_direction = Some(0);
    c
}

#[test]
fn spring_dst_gas_requires_verified_zone() {
    use energy_store::resolve_candidates_with_gas_zone;
    let data = vec![
        gas_day(29390, "2026-03-08 07:59:00", 0),
        gas_day(29391, "2026-03-09 06:59:00", 1023),
        gas_day(29786, "2026-03-09 06:59:00", 1023),
    ];
    assert!(!resolve_candidates(data.clone()).issues.is_empty());
    let zone = "America/Los_Angeles".parse().unwrap();
    let result = resolve_candidates_with_gas_zone(data, Some(zone));
    assert!(result.issues.is_empty());
    assert_eq!(result.intervals.len(), 2);
    assert_eq!(result.intervals[1].raw_value, 1023);
}

#[test]
fn autumn_dst_gas_does_not_fabricate_usage() {
    use energy_store::resolve_candidates_with_gas_zone;
    let zone = "America/Los_Angeles".parse().unwrap();
    let data = vec![
        gas_day(1, "2026-11-01 06:59:00", 42),
        gas_day(2, "2026-11-02 07:59:00", 43),
    ];
    let result = resolve_candidates_with_gas_zone(data, Some(zone));
    assert!(result.issues.is_empty());
    assert_eq!(result.intervals.len(), 2);
    assert_eq!(result.intervals[0].raw_value, 42);
    assert_eq!(result.intervals[1].raw_value, 43);
}

#[test]
fn gas_conflicting_duplicate_is_still_rejected() {
    use energy_store::resolve_candidates_with_gas_zone;
    let zone = "America/Los_Angeles".parse().unwrap();
    let data = vec![
        gas_day(1, "2026-03-09 06:59:00", 1023),
        gas_day(2, "2026-03-09 06:59:00", 1024),
    ];
    let result = resolve_candidates_with_gas_zone(data, Some(zone));
    assert!(result.intervals.is_empty());
    assert_eq!(result.issues[0].kind, StreamIssueKind::ConflictingDuplicate);
}

#[test]
fn unconfigured_and_non_gas_intervals_remain_strict() {
    use energy_store::resolve_candidates_with_gas_zone;
    let zone = "America/Los_Angeles".parse().unwrap();
    let data = vec![candidate(1, 0, 86_400, 50), candidate(2, 1, 3_600, 10)];
    let result = resolve_candidates_with_gas_zone(data, Some(zone));
    assert!(result.intervals.is_empty());
    assert_eq!(result.issues[0].kind, StreamIssueKind::OverlappingIntervals);
}
