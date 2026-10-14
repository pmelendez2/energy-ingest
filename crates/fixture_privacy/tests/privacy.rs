// SPDX-License-Identifier: MIT
//! Integration tests for privacy screening and identifier replacement.
//! All identifiers and XML documents are synthetic.

use fixture_privacy::{evaluate, sanitize_uuid_references, validate_xml};

/// Synthetic UUID repeated in XML resource references.
const TEST_RESOURCE_UUID: &str = "12345678-1234-1234-1234-123456789abc";

#[test]
fn repeated_resource_identifiers_are_replaced_consistently() {
    let xml = format!(
        "<feed><id>urn:uuid:{TEST_RESOURCE_UUID}</id><link href=\"https://x/{TEST_RESOURCE_UUID}\"/></feed>"
    );
    let (a, count) =
        sanitize_uuid_references(xml.as_bytes(), &"s".repeat(32)).unwrap();
    let (b, _) =
        sanitize_uuid_references(xml.as_bytes(), &"s".repeat(32)).unwrap();
    assert_eq!(count, 2);
    assert_eq!(a, b);
    assert!(!String::from_utf8_lossy(&a).contains(TEST_RESOURCE_UUID));
    assert_eq!(evaluate(&a).unwrap().detected_identifiers, 2);
}

#[test]
fn rejects_doctype() {
    assert!(
        validate_xml(b"<!DOCTYPE feed [<!ENTITY secret SYSTEM 'file:///etc/passwd'>]><feed/>")
            .is_err()
    );
}

#[test]
fn refuses_short_secret() {
    assert!(sanitize_uuid_references(b"<feed/>", "short").is_err());
}

#[test]
fn flags_consumption_history() {
    let risk = evaluate(b"<feed><IntervalReading/></feed>").unwrap();
    assert!(!risk.residual_risks.is_empty());
    assert!(!risk.publication_authorized);
}

/// A candidate report must become invalid after the XML changes.
#[test]
fn changed_candidate_invalidates_evidence() {
    use fixture_privacy::verify_candidate;
    use std::fs;

    let candidate = std::env::temp_dir().join(format!(
        "fixture_privacy_test_{}_{}.xml",
        std::process::id(),
        "changed_candidate",
    ));
    let original = b"<feed><id>synthetic</id></feed>";
    fs::write(&candidate, original).unwrap();
    let evidence = evaluate(original).unwrap();
    fs::write(&candidate, b"<feed><id>altered</id></feed>").unwrap();
    assert!(verify_candidate(&candidate, &evidence).is_err());
    fs::remove_file(&candidate).unwrap();
}

/// Evidence that claims approval must never be accepted by the verifier.
#[test]
fn rejects_forged_publication_authorization() {
    use fixture_privacy::verify_candidate;
    use std::fs;

    let candidate = std::env::temp_dir().join(format!(
        "fixture_privacy_test_{}_{}.xml",
        std::process::id(),
        "forged_authorization",
    ));
    let xml = b"<feed/>";
    fs::write(&candidate, xml).unwrap();
    let mut evidence = evaluate(xml).unwrap();
    evidence.publication_authorized = true;
    assert!(verify_candidate(&candidate, &evidence).is_err());
    fs::remove_file(&candidate).unwrap();
}

/// Cargo is the only source of the tool version used in evidence.
#[test]
fn evidence_version_comes_from_cargo() {
    let evidence = evaluate(b"<feed/>").unwrap();
    assert_eq!(evidence.tool_version, env!("CARGO_PKG_VERSION"));
}

/// Check that one dataset key preserves cross-file resource identities.
#[test]
fn candidate_set_preserves_cross_file_uuid_links() {
    use fixture_privacy::{sanitize_set, verify_directory};
    use std::fs;
    let base = std::env::temp_dir().join(format!(
        "fixture-privacy-test-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("case")
    ));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let first = base.join("one.xml");
    let second = base.join("two.xml");
    let input = format!("<root><id>{TEST_RESOURCE_UUID}</id></root>");
    fs::write(&first, &input).unwrap();
    fs::write(&second, &input).unwrap();
    let destination = base.join("candidates");
    sanitize_set(
        &[first, second],
        &destination,
        "synthetic-test-only-key-is-at-least-32-characters",
    )
    .unwrap();
    verify_directory(&destination).unwrap();
    let left = fs::read(destination.join("one.xml")).unwrap();
    let right = fs::read(destination.join("two.xml")).unwrap();
    assert_eq!(left, right);
    assert!(!String::from_utf8(left).unwrap().contains(TEST_RESOURCE_UUID));
    fs::remove_dir_all(base).unwrap();
}

/// Unknown and missing candidates must not be accepted by verification.
#[test]
fn set_evidence_cannot_omit_or_add_candidates() {
    use fixture_privacy::{evaluate, verify_set};
    use std::collections::BTreeMap;
    use std::path::Path;
    let evidence = evaluate(b"<root/>").unwrap();
    let mut reports = BTreeMap::new();
    reports.insert("absent.xml".to_string(), evidence);
    assert!(verify_set(&[], &reports).is_err());
    assert!(verify_set(&[Path::new("missing.xml")], &reports).is_err());
}

/// Minimization retains structure and representative readings per block.
#[test]
fn representative_readings_are_limited_per_block() {
    use fixture_privacy::minimize_interval_readings;
    let xml = br#"<feed xmlns:e="urn:espi"><e:IntervalBlock>
      <e:IntervalReading><e:value>1</e:value></e:IntervalReading>
      <e:IntervalReading><e:value>2</e:value></e:IntervalReading>
      <e:IntervalReading><e:value>3</e:value></e:IntervalReading>
    </e:IntervalBlock><e:IntervalBlock>
      <e:IntervalReading><e:value>4</e:value></e:IntervalReading>
      <e:IntervalReading><e:value>5</e:value></e:IntervalReading>
    </e:IntervalBlock></feed>"#;
    let (output, summary) = minimize_interval_readings(xml, 1).unwrap();
    let result = String::from_utf8(output).unwrap();
    assert_eq!(summary.retained, 2);
    assert_eq!(summary.removed, 3);
    assert!(result.contains("<e:value>1</e:value>"));
    assert!(result.contains("<e:value>4</e:value>"));
    assert!(!result.contains("<e:value>2</e:value>"));
    assert!(!result.contains("<e:value>5</e:value>"));
    assert_eq!(result.matches("<e:IntervalBlock>").count(), 2);
}

/// A zero limit must not silently produce empty consumption coverage.
#[test]
fn representative_limit_must_be_positive() {
    use fixture_privacy::minimize_interval_readings;
    assert!(minimize_interval_readings(b"<feed/>", 0).is_err());
}

/// Reading-like elements outside IntervalBlock are preserved.
#[test]
fn minimization_preserves_unrelated_xml_elements() {
    use fixture_privacy::minimize_interval_readings;
    let input = b"<feed><IntervalReading>metadata</IntervalReading></feed>";
    let (output, counts) = minimize_interval_readings(input, 1).unwrap();
    assert_eq!(counts.removed, 0);
    assert_eq!(output, input);
}

/// Frequency is derived from Green Button XML reading timestamps,
/// never inferred from file names supplied by a user.
#[test]
fn sampling_detects_hourly_daily_and_monthly() {
    use fixture_privacy::sampling_limit;
    for (seconds, name, limit) in
        [(3600, "hourly", 168), (86400, "daily", 7), (2592000, "monthly", 3)]
    {
        let xml = format!(
            "<feed><IntervalReading><timePeriod>\
             <duration>{seconds}</duration></timePeriod>\
             </IntervalReading></feed>"
        );
        assert_eq!(
            sampling_limit(xml.as_bytes(), 7, 7, 3).unwrap(),
            (name, limit)
        );
    }
}

#[test]
fn unknown_frequency_fails_instead_of_guessing() {
    use fixture_privacy::sampling_limit;
    assert!(sampling_limit(b"<feed/>", 7, 7, 3).is_err());
}

/// Selection must preserve a whole short scenario, regardless of name.
#[test]
fn regression_sampling_keeps_short_xml_scenarios() {
    use fixture_privacy::regression_sampling_limit;
    for (seconds, count, kind) in
        [(3600, 744, "hourly"), (86400, 31, "daily"), (2592000, 3, "monthly")]
    {
        let readings = (0..count)
            .map(|_| {
                format!(
                    "<IntervalReading><timePeriod><duration>{seconds}</duration>\
             </timePeriod></IntervalReading>"
                )
            })
            .collect::<String>();
        let xml =
            format!("<feed><IntervalBlock>{readings}</IntervalBlock></feed>");
        let (detected, kept, reason) =
            regression_sampling_limit(xml.as_bytes(), 7, 7, 3).unwrap();
        assert_eq!(detected, kind);
        assert_eq!(kept, count);
        assert_eq!(reason, "complete short coverage");
    }
}

/// Large histories should be trimmed to the requested representative
/// default, independently of any user-supplied file naming convention.
#[test]
fn regression_sampling_minimizes_long_xml_histories() {
    use fixture_privacy::regression_sampling_limit;
    let reading = "<IntervalReading><timePeriod><duration>3600</duration>\
                   </timePeriod></IntervalReading>";
    let xml = format!(
        "<feed><IntervalBlock>{}</IntervalBlock></feed>",
        reading.repeat(800),
    );
    let (kind, limit, reason) =
        regression_sampling_limit(xml.as_bytes(), 7, 7, 3).unwrap();
    assert_eq!(kind, "hourly");
    assert_eq!(limit, 168);
    assert_eq!(reason, "representative long history");
}

/// Multiple resources containing short blocks must not be trimmed just
/// because their combined reading count exceeds one month's data.
#[test]
fn regression_sampling_uses_per_block_coverage() {
    use fixture_privacy::regression_sampling_limit;
    let reading = "<IntervalReading><timePeriod><duration>86400</duration>\
                   </timePeriod></IntervalReading>";
    let block =
        format!("<IntervalBlock>{}</IntervalBlock>", reading.repeat(31));
    let xml = format!("<feed>{block}{block}</feed>");
    assert_eq!(
        regression_sampling_limit(xml.as_bytes(), 7, 7, 3).unwrap().1,
        31
    );
}

/// Build a synthetic daily fixture with explicit UTC starts.
/// It contains no customer data and is independent of filenames.
fn daily_fixture(starts: &[i64]) -> Vec<u8> {
    let mut xml = String::from("<feed><IntervalBlock>");
    for start in starts {
        xml.push_str(&format!(
            "<IntervalReading><timePeriod><start>{start}</start>\
             <duration>86400</duration></timePeriod></IntervalReading>"
        ));
    }
    xml.push_str("</IntervalBlock></feed>");
    xml.into_bytes()
}

#[test]
fn daily_calendar_handles_every_month_length() {
    use fixture_privacy::daily_month_coverage;
    // 2025 February 28; 2024 February 29; April 30; January 31.
    for (start, days, year, month) in [
        (1738368000_i64, 28, 2025, 2),
        (1706745600_i64, 29, 2024, 2),
        (1711929600_i64, 30, 2024, 4),
        (1704067200_i64, 31, 2024, 1),
    ] {
        let timestamps: Vec<_> =
            (0..days).map(|day| start + i64::from(day) * 86_400).collect();
        let months = daily_month_coverage(&daily_fixture(&timestamps)).unwrap();
        assert_eq!(months.len(), 1);
        assert_eq!((months[0].year, months[0].month), (year, month));
        assert_eq!(months[0].expected_days, days);
        assert!(months[0].complete);
    }
}

#[test]
fn daily_calendar_reports_gaps_and_duplicates() {
    use fixture_privacy::daily_month_coverage;
    // 2024-02-01 twice, followed by 2024-02-03; day 2 is missing.
    let months = daily_month_coverage(&daily_fixture(&[
        1706745600, 1706745600, 1706918400,
    ]))
    .unwrap();
    assert_eq!(months.len(), 1);
    assert!(!months[0].complete);
    assert_eq!(months[0].duplicate_days, 1);
    assert!(months[0].missing_days.contains(&2));
}

#[test]
fn daily_calendar_distinguishes_year_boundaries() {
    use fixture_privacy::daily_month_coverage;
    // 2023-12-31 and 2024-01-01 belong to separate partial months.
    let months =
        daily_month_coverage(&daily_fixture(&[1703980800, 1704067200]))
            .unwrap();
    assert_eq!(months.len(), 2);
    assert_eq!((months[0].year, months[0].month), (2023, 12));
    assert_eq!((months[1].year, months[1].month), (2024, 1));
    assert!(months.iter().all(|month| !month.complete));
}

/// Provider-local midnight must not stop a privacy preparation dry run.
/// Without a validated timezone, no calendar-completeness claim is made.
#[test]
fn daily_calendar_local_midnight_is_not_a_fatal_error() {
    use fixture_privacy::{calendar_summary, daily_month_coverage};
    // 2024-01-01 08:00 UTC can represent local midnight (UTC-8).
    let months = daily_month_coverage(&daily_fixture(&[1704096000])).unwrap();
    assert!(months.is_empty());
    assert!(calendar_summary(&months).contains("not verified"));
}

/// A fixed UTC offset supplied by XML can resolve local midnight.
#[test]
fn calendar_uses_xml_fixed_timezone_offset() {
    use fixture_privacy::{CoverageStatus, assess_daily_coverage};
    let xml = b"<feed><LocalTimeParameters><tzOffset>-28800</tzOffset>\
<dstOffset>0</dstOffset></LocalTimeParameters>\
<IntervalReading><timePeriod><start>1704096000</start>\
<duration>86400</duration></timePeriod></IntervalReading></feed>";
    let report = assess_daily_coverage(xml).unwrap();
    assert_eq!(report.status, CoverageStatus::Verified);
    assert_eq!((report.months[0].year, report.months[0].month), (2024, 1));
}

/// DST metadata is not a license to guess a provider's transition rules.
#[test]
fn calendar_refuses_unresolved_daylight_saving_rules() {
    use fixture_privacy::{CoverageStatus, assess_daily_coverage};
    let xml = b"<feed><LocalTimeParameters><tzOffset>-28800</tzOffset>\
<dstOffset>3600</dstOffset></LocalTimeParameters>\
<IntervalReading><timePeriod><start>1704096000</start>\
<duration>86400</duration></timePeriod></IntervalReading></feed>";
    let report = assess_daily_coverage(xml).unwrap();
    assert_eq!(report.status, CoverageStatus::DaylightSavingRulesUnsupported);
}

/// Missing local timezone metadata remains a diagnostic, not an error.
#[test]
fn calendar_explains_unresolved_local_timestamp() {
    use fixture_privacy::{CoverageStatus, assess_daily_coverage};
    let report = assess_daily_coverage(&daily_fixture(&[1704096000])).unwrap();
    assert_eq!(report.status, CoverageStatus::TimezoneUnresolved);
}
