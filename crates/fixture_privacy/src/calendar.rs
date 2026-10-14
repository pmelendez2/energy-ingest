// SPDX-License-Identifier: MIT
//! Calendar coverage inspection for daily Green Button readings.
//!
//! All timestamps are read from XML. No filename or UI label is used.
//! This is a regression-coverage diagnostic, not privacy clearance.

use crate::{Result, validate_xml};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Calendar-month coverage found in daily XML measurements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonthCoverage {
    pub year: i32,
    pub month: u32,
    pub expected_days: u32,
    pub observed_days: usize,
    pub duplicate_days: usize,
    pub missing_days: Vec<u32>,
    pub complete: bool,
}

fn leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Convert Unix days to Gregorian civil date (UTC), including leap years.
/// Based on the standard 400-year Gregorian calendar decomposition.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    if m <= 2 {
        y += 1;
    }
    (y as i32, m as u32, d as u32)
}

#[derive(Default)]
struct ReadingTime {
    start: Option<i64>,
    duration: Option<u64>,
}

/// Why a calendar result could not be established.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoverageStatus {
    Verified,
    NoDailyReadings,
    TimezoneUnresolved,
    DaylightSavingRulesUnsupported,
}

/// Distinguishes successful checks from missing timezone information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub status: CoverageStatus,
    pub months: Vec<MonthCoverage>,
    pub timezone_source: String,
}

/// A fixed UTC offset given in Green Button LocalTimeParameters.
/// Green Button's tzOffset and dstOffset values are seconds.
#[derive(Default)]
struct TimezoneMetadata {
    standard: Option<i64>,
    daylight: Option<i64>,
}

fn read_timezone(xml: &[u8]) -> Result<TimezoneMetadata> {
    let mut reader = Reader::from_reader(xml);
    let mut metadata = TimezoneMetadata::default();
    let mut inside = false;
    let mut field: Option<&str> = None;
    loop {
        match reader.read_event()? {
            Event::Start(e) => match e.local_name().as_ref() {
                "LocalTimeParameters" => inside = true,
                "tzOffset" if inside => field = Some("standard"),
                "dstOffset" if inside => field = Some("daylight"),
                _ => {}
            },
            Event::Text(t) if inside => match field {
                Some("standard") => {
                    let value = t.as_ref().trim().parse::<i64>()?;
                    if value.abs() > 86_400 {
                        return Err("invalid Green Button tzOffset".into());
                    }
                    metadata.standard = Some(value);
                }
                Some("daylight") => {
                    let value = t.as_ref().trim().parse::<i64>()?;
                    if value.abs() > 86_400 {
                        return Err("invalid Green Button dstOffset".into());
                    }
                    metadata.daylight = Some(value);
                }
                _ => {}
            },
            Event::End(e) => match e.local_name().as_ref() {
                "LocalTimeParameters" => inside = false,
                "tzOffset" | "dstOffset" => field = None,
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(metadata)
}

/// Assess calendar coverage independently from privacy verification.
///
/// XML provides Unix timestamps. A fixed local offset may be used only
/// when it is supplied with no nonzero daylight-saving adjustment.
/// Provider DST rules require separate time-zone interpretation and are
/// deliberately reported as unsupported rather than guessed.
pub fn assess_daily_coverage(xml: &[u8]) -> Result<CoverageReport> {
    validate_xml(xml)?;
    let tz = read_timezone(xml)?;
    let mut reader = Reader::from_reader(xml);
    let mut reading_depth = 0usize;
    let mut period_depth = 0usize;
    let mut active_field: Option<&str> = None;
    let mut current = ReadingTime::default();
    let mut starts = Vec::new();
    loop {
        match reader.read_event()? {
            Event::Start(e) => match e.local_name().as_ref() {
                "IntervalReading" => {
                    reading_depth += 1;
                    if reading_depth == 1 {
                        current = ReadingTime::default();
                    }
                }
                "timePeriod" if reading_depth > 0 => period_depth += 1,
                "start" | "duration"
                    if reading_depth > 0 && period_depth > 0 =>
                {
                    active_field =
                        Some(if e.local_name().as_ref() == "start" {
                            "start"
                        } else {
                            "duration"
                        });
                }
                _ => {}
            },
            Event::Text(t) if active_field.is_some() => match active_field {
                Some("start") => {
                    current.start = Some(t.as_ref().trim().parse()?);
                }
                Some("duration") => {
                    current.duration = Some(t.as_ref().trim().parse()?);
                }
                _ => {}
            },
            Event::End(e) => match e.local_name().as_ref() {
                "start" | "duration" => active_field = None,
                "timePeriod" if period_depth > 0 => period_depth -= 1,
                "IntervalReading" if reading_depth > 0 => {
                    reading_depth -= 1;
                    if reading_depth == 0 && current.duration == Some(86_400) {
                        starts.push(
                            current.start.ok_or(
                                "daily reading has no start timestamp",
                            )?,
                        );
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    let (offset, source) = if let Some(standard) = tz.standard {
        if tz.daylight.unwrap_or(0) != 0 {
            return Ok(CoverageReport {
                status: CoverageStatus::DaylightSavingRulesUnsupported,
                months: vec![],
                timezone_source: "XML local time metadata (DST)".into(),
            });
        }
        (standard, "XML local time fixed offset")
    } else {
        (0, "UTC timestamps; no local time metadata")
    };
    if starts.is_empty() {
        return Ok(CoverageReport {
            status: CoverageStatus::NoDailyReadings,
            months: vec![],
            timezone_source: source.into(),
        });
    }
    if starts.iter().any(|start| (start + offset).rem_euclid(86_400) != 0) {
        return Ok(CoverageReport {
            status: CoverageStatus::TimezoneUnresolved,
            months: vec![],
            timezone_source: source.into(),
        });
    }
    let mut months: BTreeMap<(i32, u32), BTreeMap<u32, usize>> =
        BTreeMap::new();
    for start in starts {
        let (year, month, day) =
            civil_from_days((start + offset).div_euclid(86_400));
        *months.entry((year, month)).or_default().entry(day).or_default() += 1;
    }
    let mut results = Vec::new();
    for ((year, month), observed) in months {
        let expected_days = days_in_month(year, month);
        let missing_days: Vec<u32> = (1..=expected_days)
            .filter(|day| !observed.contains_key(day))
            .collect();
        let duplicate_days =
            observed.values().map(|count| count.saturating_sub(1)).sum();
        results.push(MonthCoverage {
            year,
            month,
            expected_days,
            observed_days: observed.len(),
            duplicate_days,
            complete: missing_days.is_empty() && duplicate_days == 0,
            missing_days,
        });
    }
    Ok(CoverageReport {
        status: CoverageStatus::Verified,
        months: results,
        timezone_source: source.into(),
    })
}

/// Preserve the earlier API for consumers needing only verified months.
pub fn daily_month_coverage(xml: &[u8]) -> Result<Vec<MonthCoverage>> {
    Ok(assess_daily_coverage(xml)?.months)
}

/// Compact output that names the reason when verification is unavailable.
pub fn coverage_report_summary(report: &CoverageReport) -> String {
    match report.status {
        CoverageStatus::Verified => format!(
            "{} (timezone: {})",
            calendar_summary(&report.months),
            report.timezone_source,
        ),
        CoverageStatus::NoDailyReadings => {
            "calendar coverage not applicable: no daily readings".into()
        }
        CoverageStatus::TimezoneUnresolved => format!(
            "calendar coverage not verified: timestamp alignment or timezone \
metadata unresolved ({})",
            report.timezone_source,
        ),
        CoverageStatus::DaylightSavingRulesUnsupported => {
            "calendar coverage not verified: daylight-saving rules \
require provider-specific interpretation"
                .into()
        }
    }
}

/// Summarize verified month counts without exposing measured values.
pub fn calendar_summary(months: &[MonthCoverage]) -> String {
    if months.is_empty() {
        return "calendar coverage not verified".into();
    }
    let complete = months.iter().filter(|m| m.complete).count();
    let gaps: usize = months.iter().map(|m| m.missing_days.len()).sum();
    let duplicates: usize = months.iter().map(|m| m.duplicate_days).sum();
    format!(
        "calendar months: {} complete of {}, {} missing days, {} duplicates",
        complete,
        months.len(),
        gaps,
        duplicates,
    )
}
