// SPDX-License-Identifier: MIT
//! UTC and local-time helpers replacing deprecated Chrono dependencies.
//!
//! Persisted timestamps remain naive UTC (MySQL DATETIME semantics).
use jiff::{Timestamp, tz::TimeZone};
use time::macros::format_description;
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime};

pub type NaiveDateTime = PrimitiveDateTime;
pub type NaiveDate = Date;

/// Parse only the formats already accepted by the application.
pub fn parse_datetime(
    value: &str,
    format: &str,
) -> Result<NaiveDateTime, time::error::Parse> {
    match format {
        "%Y-%m-%d %H:%M:%S" => PrimitiveDateTime::parse(
            value,
            &format_description!(
                "[year]-[month]-[day] [hour]:[minute]:[second]"
            ),
        ),
        "%Y-%m-%dT%H:%M:%S" => PrimitiveDateTime::parse(
            value,
            &format_description!(
                "[year]-[month]-[day]T[hour]:[minute]:[second]"
            ),
        ),
        "%Y-%m-%dT%H:%M:%SZ" => PrimitiveDateTime::parse(
            value,
            &format_description!(
                "[year]-[month]-[day]T[hour]:[minute]:[second]Z"
            ),
        ),
        _ => PrimitiveDateTime::parse(
            value,
            &format_description!("[year]-[month]-[day]"),
        ),
    }
}

/// The exact wire encoding previously used for UTC API timestamps.
pub fn format_utc(value: NaiveDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        value.year(),
        u8::from(value.month()),
        value.day(),
        value.hour(),
        value.minute(),
        value.second(),
    )
}

pub fn date_from_ymd(year: i32, month: u32, day: u32) -> Option<Date> {
    Date::from_calendar_date(
        year,
        Month::try_from(u8::try_from(month).ok()?).ok()?,
        u8::try_from(day).ok()?,
    )
    .ok()
}

pub fn utc_seconds(seconds: i64) -> Option<NaiveDateTime> {
    let dt = OffsetDateTime::from_unix_timestamp(seconds).ok()?;
    Some(PrimitiveDateTime::new(dt.date(), dt.time()))
}

pub fn local_date(seconds: i64) -> Option<Date> {
    date_in_zone(seconds, &TimeZone::system())
}

pub fn date_in_zone(seconds: i64, zone: &TimeZone) -> Option<Date> {
    let date = zone.to_datetime(Timestamp::from_second(seconds).ok()?).date();
    date_from_ymd(date.year() as i32, date.month() as u32, date.day() as u32)
}

/// Convert an instant to its local hour/minute/second and calendar date.
pub fn local_datetime_fields(
    seconds: i64,
    zone: &TimeZone,
) -> Option<(Date, u8, u8, u8)> {
    let local = zone.to_datetime(Timestamp::from_second(seconds).ok()?);
    let date = date_from_ymd(
        local.date().year() as i32,
        local.date().month() as u32,
        local.date().day() as u32,
    )?;
    Some((date, local.hour() as u8, local.minute() as u8, local.second() as u8))
}

pub fn format_local(seconds: i64) -> Option<String> {
    let ts = Timestamp::from_second(seconds).ok()?;
    Some(
        ts.to_zoned_datetime(TimeZone::system())
            .strftime("%Y-%m-%d %H:%M:%S %Z (%:z)")
            .to_string(),
    )
}
