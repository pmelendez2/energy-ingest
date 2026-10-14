// SPDX-License-Identifier: MIT

//! Command-line Green Button XML inspection.

use std::env;
use std::path::Path;
use std::process::ExitCode;

use chrono::{DateTime, Local, Utc};
use gb::{MeterReadingSummary, inspect_file};

fn main() -> ExitCode {
    let mut args = env::args_os();

    let program = args.next().unwrap_or_else(|| "gb-inspect".into());

    let Some(path) = args.next() else {
        usage(&program);
        return ExitCode::FAILURE;
    };

    if args.next().is_some() {
        usage(&program);
        return ExitCode::FAILURE;
    }

    match inspect_file(&path) {
        Ok(report) => {
            println!("Green Button Usage Summary");
            println!();

            let total = report.meter_readings.len();

            for (index, reading) in report.meter_readings.iter().enumerate() {
                print_meter_reading(index + 1, total, reading);
            }

            println!();
            println!("XML Resources");
            println!("  Reading types: {}", report.reading_type_count);
            println!("  Meter readings: {}", report.meter_reading_count);
            println!("  Interval blocks: {}", report.interval_block_count);

            ExitCode::SUCCESS
        }

        Err(error) => {
            eprintln!("gb-inspect: {error}");
            ExitCode::FAILURE
        }
    }
}

fn usage(program: &std::ffi::OsStr) {
    eprintln!("usage: {} <green-button.xml>", Path::new(program).display());
}

fn print_meter_reading(
    index: usize,
    total: usize,
    reading: &MeterReadingSummary,
) {
    println!("MeterReading {index} of {total}");
    println!("  Commodity: {}", reading.commodity);

    print_code_value(
        "Flow direction",
        reading.flow_direction,
        reading.flow_direction_label(),
    );

    print_code_value(
        "Unit",
        reading.unit_of_measure,
        reading.unit_of_measure_label(),
    );

    println!(
        "  Expected interval: {}",
        format_optional_interval(reading.nominal_interval_seconds,)
    );

    println!(
        "  Observed interval: {}",
        format_observed_intervals(&reading.observed_interval_seconds,)
    );

    println!("  Interval readings: {}", reading.reading_count);

    print_time("First reading", reading.start_unix);

    print_last_reading(reading);

    println!(
        "  Expected vs Observed interval validation: {}",
        interval_validation(reading)
    );
}

fn print_code_value(label: &str, code: Option<u32>, description: Option<&str>) {
    match (code, description) {
        (Some(code), Some(description)) => {
            println!("  {label}: {description} ({code})");
        }

        (Some(code), None) => {
            println!("  {label}: Unknown ({code})");
        }

        (None, _) => {
            println!("  {label}: Unknown");
        }
    }
}

fn format_optional_interval(seconds: Option<u64>) -> String {
    match seconds {
        Some(seconds) => format_interval(seconds),
        None => "Unknown".to_string(),
    }
}

fn format_observed_intervals(intervals: &[u64]) -> String {
    match intervals {
        [] => "Unknown".to_string(),

        [seconds] => format_interval(*seconds),

        values => values
            .iter()
            .map(|seconds| format_interval(*seconds))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn format_interval(seconds: u64) -> String {
    let description = match seconds {
        60 => "1 minute".to_string(),
        900 => "15 minutes".to_string(),
        1_800 => "30 minutes".to_string(),
        3_600 => "1 hour".to_string(),
        86_400 => "1 day".to_string(),

        seconds if seconds % 3_600 == 0 => {
            format!("{} hours", seconds / 3_600)
        }

        seconds if seconds % 60 == 0 => {
            format!("{} minutes", seconds / 60)
        }

        _ => format!("{seconds} seconds"),
    };

    format!("{description} ({seconds} seconds)")
}

fn print_time(label: &str, value: Option<i64>) {
    match value.and_then(unix_time_local) {
        Some(value) => {
            println!(
                "  {label}: {}",
                value.format("%Y-%m-%d %H:%M:%S %Z (%:z)",)
            );
        }

        None => println!("  {label}: Unknown"),
    }
}

fn print_last_reading(reading: &MeterReadingSummary) {
    let interval = match reading.observed_interval_seconds.as_slice() {
        [seconds] => i64::try_from(*seconds).ok(),
        _ => None,
    };

    let last_start = match (reading.end_unix_exclusive, interval) {
        (Some(end), Some(interval)) => end.checked_sub(interval),

        _ => None,
    };

    print_time("Last reading", last_start);
}

fn unix_time_local(seconds: i64) -> Option<DateTime<Local>> {
    DateTime::<Utc>::from_timestamp(seconds, 0)
        .map(|value| value.with_timezone(&Local))
}

fn interval_validation(reading: &MeterReadingSummary) -> &'static str {
    if reading.observed_interval_seconds.is_empty()
        || reading.nominal_interval_seconds.is_none()
    {
        "Unknown"
    } else if !reading.has_uniform_observed_interval() {
        "Mixed"
    } else if reading.interval_matches_reading_type() {
        "Match"
    } else {
        "Mismatch"
    }
}
