// SPDX-License-Identifier: MIT
//! Registered provider fixture profiles and validation.
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::{Commodity, MeterReadingSummary, inspect_file};
use chrono::{DateTime, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use toml::value::Datetime;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Profile {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub provider: Provider,
    #[serde(default, rename = "fixture")]
    pub fixtures: Vec<Fixture>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Provider {
    pub edc: String,
    #[serde(default)]
    pub cca: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Fixture {
    pub file: String,
    pub commodity: String,
    pub flow_direction: u32,
    pub unit_of_measure: u32,
    pub interval_seconds: u64,
    pub reading_count: u64,
    pub start_date: Datetime,
    pub end_date: Datetime,
    #[serde(default)]
    pub require_zero: bool,
}

#[derive(Debug)]
pub struct LoadedProfile {
    pub directory: PathBuf,
    pub metadata: Profile,
}

pub fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|b| {
            b.is_ascii_lowercase()
                || b.is_ascii_digit()
                || b == b'_'
                || b == b'-'
        })
}

fn date(value: &Datetime) -> Result<NaiveDate> {
    let d = value.date.ok_or("expected TOML date")?;
    NaiveDate::from_ymd_opt(
        i32::from(d.year),
        u32::from(d.month),
        u32::from(d.day),
    )
    .ok_or_else(|| "invalid calendar date".into())
}

fn secure_fixture_file(dir: &Path, name: &str) -> Result<PathBuf> {
    let path = Path::new(name);
    if path.as_os_str().is_empty()
        || path.extension().and_then(|x| x.to_str()) != Some("xml")
        || !path.components().all(|p| matches!(p, Component::Normal(_)))
    {
        return Err(format!("invalid fixture path: {name}").into());
    }
    let base = fs::canonicalize(dir.join("fixtures"))?;
    let source = fs::canonicalize(base.join(path))?;
    if !source.starts_with(base) || !source.is_file() {
        return Err(format!("fixture escapes profile: {name}").into());
    }
    Ok(source)
}

pub fn load(dir: &Path) -> Result<LoadedProfile> {
    let content = fs::read_to_string(dir.join("profile.toml"))?;
    let metadata: Profile = toml::from_str(&content)?;
    let dirname = dir
        .file_name()
        .and_then(|x| x.to_str())
        .ok_or("invalid directory name")?;
    if metadata.schema_version != 1
        || !valid_id(&metadata.id)
        || metadata.id != dirname
        || metadata.name.trim().is_empty()
        || !valid_id(&metadata.provider.edc)
    {
        return Err(format!("invalid profile: {dirname}").into());
    }
    let mut ccas = HashSet::new();
    for cca in &metadata.provider.cca {
        if !valid_id(cca) || !ccas.insert(cca) {
            return Err("invalid or duplicate CCA".into());
        }
    }
    let mut names = HashSet::new();
    for f in &metadata.fixtures {
        if !names.insert(&f.file) {
            return Err("duplicate fixture".into());
        }
        secure_fixture_file(dir, &f.file)?;
        date(&f.start_date)?;
        date(&f.end_date)?;
        if !matches!(f.commodity.as_str(), "electricity" | "natural_gas") {
            return Err("unknown fixture commodity".into());
        }
    }
    Ok(LoadedProfile { directory: dir.to_path_buf(), metadata })
}

pub fn discover(root: &Path) -> Result<Vec<LoadedProfile>> {
    let mut directories = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            directories.push(entry.path());
        }
    }
    directories.sort();
    let mut profiles = Vec::new();
    for dir in directories {
        profiles.push(load(&dir)?);
    }
    Ok(profiles)
}

pub fn init(root: &Path, id: &str) -> Result<()> {
    if !valid_id(id) {
        return Err("invalid profile ID".into());
    }
    let dir = root.join(id);
    if dir.exists() {
        return Err("profile already exists".into());
    }
    fs::create_dir_all(dir.join("fixtures"))?;
    fs::write(
        dir.join("profile.toml"),
        format!(
            "# SPDX-License-Identifier: MIT\n\nschema_version = 1\nid = \"{id}\"\nname = \"{id}\"\n\n[provider]\nedc = \"{id}\"\ncca = []\n"
        ),
    )?;
    Ok(())
}

pub fn add(dir: &Path, file: &Path) -> Result<()> {
    let mut loaded = load(dir)?;
    let text = fs::read_to_string(file)?;
    let fixture: Fixture = toml::from_str(&text)?;
    secure_fixture_file(dir, &fixture.file)?;
    if loaded.metadata.fixtures.iter().any(|x| x.file == fixture.file) {
        return Err("fixture already registered".into());
    }
    loaded.metadata.fixtures.push(fixture);
    let updated = toml::to_string_pretty(&loaded.metadata)?;
    fs::write(
        dir.join("profile.toml"),
        format!("# SPDX-License-Identifier: MIT\n\n{updated}"),
    )?;
    Ok(())
}

fn local_date(seconds: i64) -> Option<NaiveDate> {
    DateTime::<Utc>::from_timestamp(seconds, 0)
        .map(|x| x.with_timezone(&Local).date_naive())
}

fn final_date(reading: &MeterReadingSummary) -> Option<NaiveDate> {
    let [interval] = reading.observed_interval_seconds.as_slice() else {
        return None;
    };
    let end = reading.end_unix_exclusive?;
    local_date(end.checked_sub(i64::try_from(*interval).ok()?)?)
}

pub fn validate(profile: &LoadedProfile) -> Result<usize> {
    if profile.metadata.fixtures.is_empty() {
        return Err("profile has no golden fixtures".into());
    }
    for f in &profile.metadata.fixtures {
        let source = secure_fixture_file(&profile.directory, &f.file)?;
        let report = inspect_file(&source)?;
        let [reading] = report.meter_readings.as_slice() else {
            return Err(
                format!("{}: expected one meter reading", f.file).into()
            );
        };
        let commodity = match f.commodity.as_str() {
            "electricity" => Commodity::Electricity,
            "natural_gas" => Commodity::NaturalGas,
            _ => return Err("invalid commodity".into()),
        };
        let start = reading.start_unix.and_then(local_date);
        if reading.commodity != commodity
            || reading.flow_direction != Some(f.flow_direction)
            || reading.unit_of_measure != Some(f.unit_of_measure)
            || reading.nominal_interval_seconds != Some(f.interval_seconds)
            || reading.observed_interval_seconds != vec![f.interval_seconds]
            || reading.reading_count != f.reading_count
            || start != Some(date(&f.start_date)?)
            || final_date(reading) != Some(date(&f.end_date)?)
            || !reading.interval_matches_reading_type()
        {
            return Err(format!("{}: expectation mismatch", f.file).into());
        }
    }
    Ok(profile.metadata.fixtures.len())
}
