// SPDX-License-Identifier: MIT

//! Golden-fixture tests for generic Green Button parsing and inspection.
//!
//! # Purpose
//!
//! Verifies provider-independent XML interpretation, normalized ingestion
//! fields, relationship resolution, and human-readable inspection behavior.
//!
//! # Architecture
//!
//! Sanitized fixtures provide regression evidence under ADR-0020 while the
//! manifest keeps expected calendar and measurement characteristics reviewable.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

use chrono::{DateTime, Local, NaiveDate, Utc};
use gb::{Commodity, MeterReadingSummary, inspect_file, parse_reader};
use serde::Deserialize;
use toml::value::Datetime;

/// Human-reviewed expectations for all golden fixtures.
#[derive(Debug, Deserialize)]
struct FixtureManifest {
    fixture: Vec<ExpectedFixture>,
}

/// Expected characteristics for one sanitized golden fixture.
#[derive(Debug, Deserialize)]
struct ExpectedFixture {
    /// Fixture filename relative to the profile fixture directory.
    file: String,

    /// Human-readable commodity expectation.
    commodity: String,

    /// Expected Green Button flow-direction code.
    flow_direction: u32,

    /// Expected Green Button unit-of-measure code.
    unit_of_measure: u32,

    /// Expected interval duration in seconds.
    interval_seconds: u64,

    /// Expected number of source interval readings.
    reading_count: u64,

    /// Expected local calendar date of the first reading.
    start_date: Datetime,

    /// Expected local calendar date of the last reading.
    end_date: Datetime,
}

/// Verifies every golden fixture against the human-reviewed manifest.
///
/// # Test Goal
///
/// Prove that XML-derived commodity, interval, reading count, and calendar
/// coverage match the fixture expectations.
///
/// # Inputs
///
/// Sanitized SDG&E golden fixtures and `manifest.toml`.
///
/// # Expected Outcomes
///
/// Each fixture resolves exactly one MeterReading and matches all declared
/// measurement and coverage expectations.
#[test]
fn golden_fixtures_match_manifest() {
    for (directory, expected) in all_fixtures() {
        let path = directory.join("fixtures").join(&expected.file);

        let report = inspect_file(&path).unwrap_or_else(|error| {
            panic!("{} should inspect successfully: {error}", path.display())
        });

        assert_eq!(
            report.meter_readings.len(),
            1,
            "{} should contain one MeterReading",
            expected.file
        );

        let reading = &report.meter_readings[0];

        assert_eq!(
            reading.commodity,
            expected_commodity(&expected.commodity),
            "{} commodity",
            expected.file
        );

        assert_eq!(
            reading.nominal_interval_seconds,
            Some(expected.interval_seconds),
            "{} nominal interval",
            expected.file
        );

        assert_eq!(
            reading.observed_interval_seconds,
            vec![expected.interval_seconds],
            "{} observed interval",
            expected.file
        );

        assert_eq!(
            reading.reading_count, expected.reading_count,
            "{} reading count",
            expected.file
        );

        assert_eq!(
            first_reading_date(reading),
            Some(manifest_date(expected.start_date)),
            "{} start date",
            expected.file
        );

        assert_eq!(
            last_reading_date(reading),
            Some(manifest_date(expected.end_date)),
            "{} end date",
            expected.file
        );

        assert!(
            reading.interval_matches_reading_type(),
            "{} interval should match ReadingType",
            expected.file
        );
    }
}

/// Verifies that normalized parsing retains ingestion-critical source data.
///
/// # Test Goal
///
/// Prove that resource relationships, ReadingType numeric metadata, block
/// timing, reading quality, and a zero-valued reading survive parsing.
///
/// # Inputs
///
/// A compact synthetic Green Button XML document.
///
/// # Expected Outcomes
///
/// The resulting `GbDocument` exposes all fields needed by the
/// persistence integration path without relying on provider filenames.
#[test]
fn normalized_parser_retains_ingestion_fields() {
    let xml = r#"
<feed xmlns="http://www.w3.org/2005/Atom"
      xmlns:espi="http://naesb.org/espi">
  <entry>
    <link rel="self" href="usage-point-1" />
    <link rel="related" href="usage-point-1/MeterReading" />
    <content>
      <espi:UsagePoint>
        <espi:ServiceCategory>
          <espi:kind>0</espi:kind>
        </espi:ServiceCategory>
      </espi:UsagePoint>
    </content>
  </entry>
  <entry>
    <link rel="self" href="reading-type-1" />
    <content>
      <espi:ReadingType>
        <espi:accumulationBehaviour>4</espi:accumulationBehaviour>
        <espi:commodity>1</espi:commodity>
        <espi:dataQualifier>12</espi:dataQualifier>
        <espi:flowDirection>1</espi:flowDirection>
        <espi:intervalLength>3600</espi:intervalLength>
        <espi:kind>12</espi:kind>
        <espi:phase>0</espi:phase>
        <espi:powerOfTenMultiplier>0</espi:powerOfTenMultiplier>
        <espi:uom>72</espi:uom>
      </espi:ReadingType>
    </content>
  </entry>
  <entry>
    <link rel="self" href="meter-reading-1" />
    <link rel="up" href="usage-point-1/MeterReading" />
    <link rel="related"
          href="meter-reading-1/IntervalBlock" />
    <link rel="related" href="reading-type-1" />
    <content>
      <espi:MeterReading />
    </content>
  </entry>
  <entry>
    <link rel="self" href="interval-block-1" />
    <link rel="up"
          href="meter-reading-1/IntervalBlock" />
    <content>
      <espi:IntervalBlock>
        <espi:interval>
          <espi:duration>86400</espi:duration>
          <espi:start>1767254400</espi:start>
        </espi:interval>
        <espi:IntervalReading>
          <espi:ReadingQuality>
            <espi:quality>16</espi:quality>
          </espi:ReadingQuality>
          <espi:timePeriod>
            <espi:duration>3600</espi:duration>
            <espi:start>1767254400</espi:start>
          </espi:timePeriod>
          <espi:value>0</espi:value>
        </espi:IntervalReading>
      </espi:IntervalBlock>
    </content>
    <updated>2026-01-01T08:00:00Z</updated>
    <published>2026-01-01T08:00:00Z</published>
  </entry>
</feed>
"#;

    let document = parse_reader(Cursor::new(xml.as_bytes()))
        .expect("synthetic Green Button XML should parse");

    assert_eq!(document.usage_points.len(), 1);
    assert_eq!(document.reading_types.len(), 1);
    assert_eq!(document.meter_readings.len(), 1);
    assert_eq!(document.interval_blocks.len(), 1);

    let usage_point = &document.usage_points[0];
    assert_eq!(usage_point.external_id, "usage-point-1");
    assert_eq!(usage_point.service_category_kind, Some(0));

    let reading_type = &document.reading_types[0];
    assert_eq!(reading_type.accumulation_behaviour, Some(4));
    assert_eq!(reading_type.commodity, Some(1));
    assert_eq!(reading_type.data_qualifier, Some(12));
    assert_eq!(reading_type.flow_direction, Some(1));
    assert_eq!(reading_type.interval_length_seconds, Some(3600));
    assert_eq!(reading_type.kind, Some(12));
    assert_eq!(reading_type.phase, Some(0));
    assert_eq!(reading_type.power_of_ten_multiplier, Some(0));
    assert_eq!(reading_type.unit_of_measure, Some(72));

    let meter_reading = &document.meter_readings[0];
    assert_eq!(meter_reading.external_id, "meter-reading-1");
    assert_eq!(
        meter_reading.up_href.as_deref(),
        Some("usage-point-1/MeterReading")
    );
    assert!(
        meter_reading.related_hrefs.iter().any(|href| href == "reading-type-1")
    );

    let block = &document.interval_blocks[0];
    assert_eq!(block.external_id, "interval-block-1");
    assert_eq!(block.up_href, "meter-reading-1/IntervalBlock");
    assert_eq!(block.start_unix, 1767254400);
    assert_eq!(block.duration_seconds, 86400);
    assert!(block.published_unix.is_some());
    assert!(block.updated_unix.is_some());
    assert_eq!(block.readings.len(), 1);

    let reading = &block.readings[0];
    assert_eq!(reading.start_unix, 1767254400);
    assert_eq!(reading.duration_seconds, 3600);
    assert_eq!(reading.value, 0);
    assert_eq!(reading.quality, Some(16));
}

/// Verifies that interval semantics come from XML rather than filenames.
///
/// # Test Goal
///
/// Prevent provider naming conventions from becoming parsing rules.
///
/// # Inputs
///
/// Sanitized golden fixtures and their manifest expectations.
///
/// # Expected Outcomes
///
/// Observed interval duration always matches XML content.
#[test]
fn filenames_do_not_define_xml_interval() {
    for (directory, expected) in all_fixtures() {
        let path = directory.join("fixtures").join(&expected.file);

        let report = inspect_file(&path).unwrap_or_else(|error| {
            panic!("{} should inspect successfully: {error}", path.display())
        });

        assert_eq!(
            report.meter_readings[0].observed_interval_seconds,
            vec![expected.interval_seconds],
            "{} interval must come from XML contents",
            expected.file
        );
    }
}

/// Verifies labels for Green Button codes declared by the fixture manifest.
///
/// # Test Goal
///
/// Confirm that known flow-direction and unit codes are presented with stable
/// human-readable labels while fixture selection remains data-driven.
///
/// # Inputs
///
/// Sanitized golden fixtures and their manifest-declared numeric codes.
///
/// # Expected Outcomes
///
/// Every fixture's raw codes match the manifest and any known code has the
/// expected stable human-readable label.
#[test]
fn known_green_button_codes_have_labels() {
    for (directory, expected) in all_fixtures() {
        let path = directory.join("fixtures").join(&expected.file);
        let report = inspect_file(&path).unwrap_or_else(|error| {
            panic!("{} should inspect successfully: {error}", path.display(),)
        });

        let reading = report
            .meter_readings
            .first()
            .expect("fixture should contain a MeterReading");

        assert_eq!(
            reading.flow_direction,
            Some(expected.flow_direction),
            "{} flow-direction code",
            expected.file,
        );
        assert_eq!(
            reading.unit_of_measure,
            Some(expected.unit_of_measure),
            "{} unit-of-measure code",
            expected.file,
        );
        assert_eq!(
            reading.flow_direction_label(),
            expected_flow_direction_label(expected.flow_direction),
            "{} flow-direction label",
            expected.file,
        );
        assert_eq!(
            reading.unit_of_measure_label(),
            expected_unit_of_measure_label(expected.unit_of_measure),
            "{} unit-of-measure label",
            expected.file,
        );
    }
}

/// Returns the expected human-readable label for a known flow-direction code.
fn expected_flow_direction_label(code: u32) -> Option<&'static str> {
    match code {
        0 => Some("N/A"),
        1 => Some("Forward"),
        19 => Some("Reverse"),
        20 => Some("Total"),
        21 => Some("Total by phase"),
        _ => None,
    }
}

/// Returns the expected human-readable label for a known unit code.
fn expected_unit_of_measure_label(code: u32) -> Option<&'static str> {
    match code {
        31 => Some("J"),
        42 => Some("m3"),
        72 => Some("Wh"),
        119 => Some("ft3"),
        132 => Some("Btu"),
        169 => Some("therm"),
        _ => None,
    }
}

/// Maps a manifest commodity label to the inspection enum.
///
/// # Inputs
///
/// - `value`: Supported manifest label.
///
/// # Returns
///
/// Returns the matching [`Commodity`] and panics for unsupported labels.
fn expected_commodity(value: &str) -> Commodity {
    match value {
        "electricity" => Commodity::Electricity,
        "natural_gas" => Commodity::NaturalGas,

        other => panic!("unsupported fixture commodity in manifest: {other}"),
    }
}

/// Converts a TOML calendar date into `chrono::NaiveDate`.
fn manifest_date(value: Datetime) -> NaiveDate {
    let date = value.date.expect("manifest value must be a TOML date");

    NaiveDate::from_ymd_opt(
        i32::from(date.year),
        u32::from(date.month),
        u32::from(date.day),
    )
    .expect("manifest date must be valid")
}

/// Returns the local calendar date of the first observed reading.
fn first_reading_date(reading: &MeterReadingSummary) -> Option<NaiveDate> {
    reading.start_unix.and_then(unix_date_local)
}

/// Returns the local calendar date of the final observed reading start.
fn last_reading_date(reading: &MeterReadingSummary) -> Option<NaiveDate> {
    let interval = match reading.observed_interval_seconds.as_slice() {
        [seconds] => i64::try_from(*seconds).ok()?,
        _ => return None,
    };

    let end = reading.end_unix_exclusive?;
    let last_start = end.checked_sub(interval)?;

    unix_date_local(last_start)
}

/// Converts Unix seconds to the process-local calendar date.
fn unix_date_local(seconds: i64) -> Option<NaiveDate> {
    DateTime::<Utc>::from_timestamp(seconds, 0)
        .map(|value| value.with_timezone(&Local).date_naive())
}

/// Every golden fixture in every discovered provider profile.
fn all_fixtures() -> Vec<(PathBuf, ExpectedFixture)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../profiles");
    let profiles = gb::profile::discover(&root)
        .expect("provider profile discovery must succeed");
    assert!(!profiles.is_empty(), "no provider profiles found");
    let mut result = Vec::new();
    for profile in profiles {
        let path = profile.directory.join("profile.toml");
        let content = fs::read_to_string(&path)
            .expect("profile manifest should be readable");
        let manifest: FixtureManifest = toml::from_str(&content)
            .expect("fixture expectations should parse");
        assert!(
            !manifest.fixture.is_empty(),
            "{} has no fixtures",
            path.display()
        );
        for expectation in manifest.fixture {
            result.push((profile.directory.clone(), expectation));
        }
    }
    result
}
