// SPDX-License-Identifier: MIT
//! Synthetic Net Energy Metering regression; no private utility data.
use gb::{inspect_file, parse_file};
use std::path::PathBuf;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../profiles/sdge/fixtures/")
        .join("sdge-nem-duplicate-usage-point-synthetic.xml")
}

#[test]
fn parser_keeps_both_nem_channels_and_duplicate_usage_point() {
    let document = parse_file(fixture()).expect("synthetic NEM XML parses");
    assert_eq!(document.usage_points.len(), 2);
    assert_eq!(document.usage_points[0], document.usage_points[1]);
    assert_eq!(document.meter_readings.len(), 2);
    assert_eq!(document.reading_types.len(), 2);
    assert_eq!(document.interval_blocks.len(), 2);
    assert_eq!(document.reading_types[0].flow_direction, Some(1));
    assert_eq!(document.reading_types[1].flow_direction, Some(19));
    assert_eq!(document.interval_blocks[0].readings[0].value, 1500);
    assert_eq!(document.interval_blocks[0].readings[1].value, 1750);
    assert_eq!(document.interval_blocks[1].readings[0].value, 400);
    assert_eq!(document.interval_blocks[1].readings[1].value, 600);
    assert_ne!(
        document.meter_readings[0].external_id,
        document.meter_readings[1].external_id
    );
}

#[test]
fn inspection_reports_separate_forward_and_reverse_flows() {
    let summary = inspect_file(fixture()).expect("synthetic NEM XML inspects");
    assert_eq!(summary.meter_readings.len(), 2);
    let flows: Vec<_> = summary
        .meter_readings
        .iter()
        .map(|r| (r.flow_direction, r.reading_count))
        .collect();
    assert!(flows.contains(&(Some(1), 2)));
    assert!(flows.contains(&(Some(19), 2)));
}
