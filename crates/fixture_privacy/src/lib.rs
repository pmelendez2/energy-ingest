// SPDX-License-Identifier: MIT
//! Shared fail-closed privacy checks for Green Button XML fixtures.
//! No operation in this crate authorizes publication automatically.
//! Architectural rationale: ADR-0028.

use quick_xml::Writer;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::Path;

mod calendar;
mod graph_verification;
mod relationships;
mod sanitization_graph;
mod workflow;

pub use calendar::{
    CoverageReport, CoverageStatus, MonthCoverage, assess_daily_coverage,
    calendar_summary, coverage_report_summary, daily_month_coverage,
};
pub use graph_verification::{
    GraphVerificationReport, IdentityMap, verify_graph, verify_unchanged_graph,
};
pub use relationships::{
    RelationshipComparison, RelationshipReport, compare_relationships,
    inspect_relationships,
};
pub use sanitization_graph::{VerifiedCandidateSet, sanitize_set_verified};
pub use workflow::{
    SamplingEvidence, SamplingPolicy, ShareableReviewSummary,
    VerifiedSetEvidence, generate_verified_minimized_set,
    generate_verified_set, verify_generated_set,
};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;
pub const MAX_XML_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Version supplied by Cargo, not a separately maintained policy number.
    pub tool_version: String,
    pub candidate_sha256: String,
    pub detected_identifiers: usize,
    pub residual_risks: Vec<String>,
    pub publication_authorized: bool,
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Validate the complete XML syntax and reject entity declarations.
/// The check is run both before and after rewriting.
pub fn validate_xml(xml: &[u8]) -> Result<()> {
    if xml.len() > MAX_XML_BYTES {
        return Err("XML exceeds maximum input size".into());
    }
    let mut reader = Reader::from_reader(xml);
    let mut depth = 0usize;
    let mut root_seen = false;
    let mut roots = 0usize;
    loop {
        match reader.read_event()? {
            Event::DocType(_) => return Err("DTD is forbidden".into()),
            Event::Start(_) => {
                if depth == 0 {
                    roots += 1;
                    root_seen = true;
                }
                depth += 1;
            }
            Event::Empty(_) => {
                if depth == 0 {
                    roots += 1;
                    root_seen = true;
                }
            }
            Event::End(_) => {
                if depth == 0 {
                    return Err("unbalanced XML".into());
                }
                depth -= 1;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !root_seen || roots != 1 || depth != 0 {
        return Err("XML must have exactly one root".into());
    }
    Ok(())
}

fn is_hex(b: u8) -> bool {
    b.is_ascii_hexdigit()
}

fn uuid_at(bytes: &[u8], pos: usize) -> bool {
    if pos + 36 > bytes.len() {
        return false;
    }
    for i in 0..36 {
        if [8, 13, 18, 23].contains(&i) {
            if bytes[pos + i] != b'-' {
                return false;
            }
        } else if !is_hex(bytes[pos + i]) {
            return false;
        }
    }
    if pos > 0 && is_hex(bytes[pos - 1]) {
        return false;
    }
    if pos + 36 < bytes.len() && is_hex(bytes[pos + 36]) {
        return false;
    }
    true
}

/// RFC 2104 HMAC-SHA256, domain-separated for identifier mapping.
/// This avoids concatenating a secret into an ordinary hash.
fn keyed_digest(key: &[u8], value: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut block = [0u8; BLOCK];
    if key.len() > BLOCK {
        block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut inner = [0x36u8; BLOCK];
    let mut outer = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        inner[i] ^= block[i];
        outer[i] ^= block[i];
    }
    let mut hash = Sha256::new();
    hash.update(inner);
    hash.update(value);
    let first = hash.finalize();
    let mut hash = Sha256::new();
    hash.update(outer);
    hash.update(first);
    hash.finalize().into()
}

/// Produce a stable synthetic UUID for the same secret and original value.
/// The mapping key must stay private and must not enter public reports.
fn synthetic(uuid: &str, secret: &str) -> String {
    let canonical = uuid.to_ascii_lowercase();
    let mut input = b"fixture-privacy:uuid:v1:".to_vec();
    input.extend_from_slice(canonical.as_bytes());
    let digest = keyed_digest(secret.as_bytes(), &input);
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15],
    )
}

/// Rewrite UUID references across all XML occurrences, including href/id
/// attributes and urn:uuid values. All files use the same private secret.
/// Return a count of replacements. No source file is modified.
pub fn sanitize_uuid_references(
    xml: &[u8],
    secret: &str,
) -> Result<(Vec<u8>, usize)> {
    if secret.len() < 32 {
        return Err("private mapping secret must be 32+ characters".into());
    }
    validate_xml(xml)?;
    let mut result = Vec::with_capacity(xml.len());
    let mut count = 0;
    let mut i = 0;
    while i < xml.len() {
        if uuid_at(xml, i) {
            let original = std::str::from_utf8(&xml[i..i + 36])?;
            result.extend_from_slice(synthetic(original, secret).as_bytes());
            i += 36;
            count += 1;
        } else {
            result.push(xml[i]);
            i += 1;
        }
    }
    validate_xml(&result)?;
    Ok((result, count))
}

/// Independent residual risk check. Unknown customer-related fields and
/// consumption histories are deliberately not marked as privacy-safe.
pub fn evaluate(xml: &[u8]) -> Result<Evidence> {
    validate_xml(xml)?;
    let lower = String::from_utf8_lossy(xml).to_ascii_lowercase();
    let mut risks = Vec::new();
    for (needle, risk) in [
        ("mds.sempra.com", "Live provider resource hostname"),
        ("<email", "Potential email field"),
        ("<address", "Potential address field"),
        ("<account", "Potential account field"),
        ("authorization", "Potential authorization material"),
        ("<intervalreading", "Consumption history requires review"),
        ("<reading", "Reading history requires review"),
    ] {
        if lower.contains(needle) {
            risks.push(risk.to_string());
        }
    }
    // Structure-dependent risks remain, even when none of the heuristics fire.
    risks.push(
        "Reidentification from usage/time patterns must be assessed".into(),
    );
    Ok(Evidence {
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        candidate_sha256: sha256(xml),
        detected_identifiers: (0..xml.len())
            .filter(|&i| uuid_at(xml, i))
            .count(),
        residual_risks: risks,
        publication_authorized: false,
    })
}

pub fn save_candidate(
    source: &Path,
    output: &Path,
    secret: &str,
) -> Result<Evidence> {
    if source == output || output.exists() {
        return Err("refusing overwrite of source or candidate".into());
    }
    let bytes = fs::read(source)?;
    let (candidate, _) = sanitize_uuid_references(&bytes, secret)?;
    let evidence = evaluate(&candidate)?;
    fs::write(output, candidate)?;
    Ok(evidence)
}

pub fn verify_candidate(path: &Path, evidence: &Evidence) -> Result<()> {
    if evidence.tool_version != env!("CARGO_PKG_VERSION") {
        return Err("privacy tool version mismatch".into());
    }
    let bytes = fs::read(path)?;
    let actual = evaluate(&bytes)?;
    if actual.candidate_sha256 != evidence.candidate_sha256 {
        return Err("stale or mismatched privacy evidence".into());
    }
    if actual.residual_risks != evidence.residual_risks {
        return Err("privacy risk report mismatch".into());
    }
    if evidence.publication_authorized {
        return Err("automatic publication authorization is forbidden".into());
    }
    Ok(())
}

/// Counts of XML readings retained and removed during minimization.
/// Values are aggregate counts only; no customer data is recorded.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MinimizationSummary {
    pub retained: usize,
    pub removed: usize,
}

/// Keep at most `limit` IntervalReading elements in each IntervalBlock.
///
/// This is a structural sample, not a privacy clearance. It preserves
/// the parent block, metadata, and remaining reading elements. The caller
/// must still validate parser semantics and review identification risks.
/// Source bytes are never modified in place.
pub fn minimize_interval_readings(
    xml: &[u8],
    limit: usize,
) -> Result<(Vec<u8>, MinimizationSummary)> {
    if limit == 0 {
        return Err("reading limit must be greater than zero".into());
    }
    validate_xml(xml)?;
    let mut reader = Reader::from_reader(xml);
    let mut writer = Writer::new(Vec::with_capacity(xml.len()));
    let mut block_counts: Vec<usize> = Vec::new();
    let mut skip_depth = 0usize;
    let mut summary = MinimizationSummary::default();
    loop {
        let event = reader.read_event()?;
        let local = match &event {
            Event::Start(e) | Event::Empty(e) => {
                Some(e.local_name().as_ref().to_owned())
            }
            Event::End(e) => Some(e.local_name().as_ref().to_owned()),
            _ => None,
        };
        if skip_depth > 0 {
            match &event {
                Event::Start(_) => skip_depth += 1,
                Event::End(_) => skip_depth -= 1,
                Event::Eof => return Err("unexpected XML end".into()),
                _ => {}
            }
            continue;
        }
        match &event {
            Event::Start(_)
                if local.as_deref().is_some_and(|n| n == "IntervalBlock") =>
            {
                block_counts.push(0);
            }
            Event::End(_)
                if local.as_deref().is_some_and(|n| n == "IntervalBlock") =>
            {
                block_counts.pop();
            }
            Event::Start(_) | Event::Empty(_)
                if local.as_deref().is_some_and(|n| n == "IntervalReading")
                    && !block_counts.is_empty() =>
            {
                let count = block_counts.last_mut().unwrap();
                *count += 1;
                if *count > limit {
                    summary.removed += 1;
                    if matches!(event, Event::Start(_)) {
                        skip_depth = 1;
                    }
                    continue;
                }
                summary.retained += 1;
            }
            Event::Eof => break,
            _ => {}
        }
        writer.write_event(event.into_owned())?;
    }
    let output = writer.into_inner();
    validate_xml(&output)?;
    Ok((output, summary))
}

/// Create candidates from a representative subset of interval readings.
/// All files use one key so repeated identifiers remain consistent.
/// This function does not authorize sharing or publication.
pub fn prepare_representative_set(
    inputs: &[std::path::PathBuf],
    destination: &Path,
    secret: &str,
    readings_per_block: usize,
) -> Result<BTreeMap<String, Evidence>> {
    if secret.len() < 32 || inputs.is_empty() || readings_per_block == 0 {
        return Err("input set, mapping key and positive limit required".into());
    }
    if destination.exists() {
        return Err("destination already exists".into());
    }
    let mut candidates = Vec::new();
    let mut reports = BTreeMap::new();
    for input in inputs {
        let name = input
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("invalid input filename")?
            .to_string();
        if !name.ends_with(".xml") || reports.contains_key(&name) {
            return Err("distinct XML filenames are required".into());
        }
        let bytes = fs::read(input)?;
        let (minimized, _) =
            minimize_interval_readings(&bytes, readings_per_block)?;
        let (candidate, _) = sanitize_uuid_references(&minimized, secret)?;
        reports.insert(name.clone(), evaluate(&candidate)?);
        candidates.push((name, candidate));
    }
    fs::create_dir(destination)?;
    let result = (|| -> Result<()> {
        for (name, bytes) in candidates {
            fs::write(destination.join(name), bytes)?;
        }
        fs::write(
            destination.join("privacy_evidence.json"),
            serde_json::to_vec_pretty(&reports)?,
        )?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(destination);
    }
    result?;
    Ok(reports)
}

/// Validate every reported candidate, rejecting duplicate basenames and
/// extra reports. Basename collisions must not alias privacy evidence.
pub fn verify_set(
    paths: &[&Path],
    reports: &BTreeMap<String, Evidence>,
) -> Result<()> {
    if paths.len() != reports.len() {
        return Err("candidate and evidence counts differ".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for path in paths {
        let name = path
            .file_name()
            .ok_or("missing filename")?
            .to_string_lossy()
            .to_string();
        if !seen.insert(name.clone()) {
            return Err("duplicate candidate filename".into());
        }
        let evidence = reports.get(&name).ok_or("missing evidence")?;
        verify_candidate(path, evidence)?;
    }
    Ok(())
}

/// Sanitize multiple XML inputs with one key and fail before writing if
/// any candidate path or source overlaps. Mapping consistency derives from
/// HMAC's shared secret and canonical UUID encoding.
pub fn sanitize_set(
    inputs: &[std::path::PathBuf],
    destination: &Path,
    secret: &str,
) -> Result<BTreeMap<String, Evidence>> {
    if secret.len() < 32 || inputs.is_empty() {
        return Err("nonempty input set and 32+ character key required".into());
    }
    if destination.exists() {
        return Err("destination already exists".into());
    }
    let mut candidates = Vec::new();
    let mut reports = BTreeMap::new();
    for input in inputs {
        let name = input
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("invalid input filename")?
            .to_owned();
        if !name.ends_with(".xml") || reports.contains_key(&name) {
            return Err("XML input filenames must be distinct".into());
        }
        let data = fs::read(input)?;
        let (candidate, _) = sanitize_uuid_references(&data, secret)?;
        let evidence = evaluate(&candidate)?;
        reports.insert(name.clone(), evidence);
        candidates.push((name, candidate));
    }
    // The output directory is intentionally created only after all inputs
    // have passed validation; no customer data is modified in place.
    fs::create_dir(destination)?;
    let write_result = (|| -> Result<()> {
        for (name, bytes) in &candidates {
            fs::write(destination.join(name), bytes)?;
        }
        fs::write(
            destination.join("privacy_evidence.json"),
            serde_json::to_vec_pretty(&reports)?,
        )?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_dir_all(destination);
    }
    write_result?;
    Ok(reports)
}

/// Verify every XML candidate in a staging directory against recomputed
/// checks. This checks freshness, NOT permission to publish the dataset.
pub fn verify_directory(destination: &Path) -> Result<()> {
    let reports: BTreeMap<String, Evidence> = serde_json::from_slice(
        &fs::read(destination.join("privacy_evidence.json"))?,
    )?;
    if reports.is_empty() {
        return Err("empty privacy evidence".into());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(destination)? {
        let entry = entry?;
        let file = entry.path();
        if file.extension().is_some_and(|ext| ext == "xml") {
            if !entry.file_type()?.is_file() {
                return Err("XML candidate must be a regular file".into());
            }
            files.push(file);
        }
    }
    let paths: Vec<&Path> = files.iter().map(|p| p.as_path()).collect();
    verify_set(&paths, &reports)
}

/// Determine a conservative reading limit from XML measurement duration.
///
/// Frequency is read from the XML itself, never inferred from filenames.
/// Mixed, unsupported, or absent durations fail closed for manual review.
pub fn sampling_limit(
    xml: &[u8],
    hourly_days: usize,
    daily_days: usize,
    monthly_months: usize,
) -> Result<(&'static str, usize)> {
    validate_xml(xml)?;
    let mut reader = Reader::from_reader(xml);
    let mut inside_period = false;
    let mut in_duration = false;
    let mut durations = std::collections::BTreeSet::new();
    loop {
        match reader.read_event()? {
            Event::Start(e) => match e.local_name().as_ref() {
                "timePeriod" => inside_period = true,
                "duration" if inside_period => in_duration = true,
                _ => {}
            },
            Event::Text(t) if in_duration => {
                let value = t.as_ref().trim().parse::<u64>()?;
                durations.insert(value);
            }
            Event::End(e) => match e.local_name().as_ref() {
                "duration" => in_duration = false,
                "timePeriod" => inside_period = false,
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    if durations.len() != 1 {
        return Err("XML reading frequency is missing or mixed".into());
    }
    let duration = *durations.iter().next().unwrap();
    // Month lengths vary, so permit 28–31 days.
    match duration {
        3600 => Ok((
            "hourly",
            hourly_days.checked_mul(24).ok_or("hourly limit overflow")?,
        )),
        86400 => Ok(("daily", daily_days)),
        2419200..=2678400 => Ok(("monthly", monthly_months)),
        _ => Err("unsupported XML reading frequency".into()),
    }
}

/// Choose a regression-aware reading limit using only XML contents.
///
/// Keep complete short samples (up to 31 days for hourly/daily,
/// three readings for monthly data). Longer histories use the user's
/// representative defaults. This is a coverage heuristic, not privacy
/// verification or a guarantee that a calendar month is represented.
/// Every XML block is considered separately: multiple short resources
/// must not be mistaken for one long history.
pub fn regression_sampling_limit(
    xml: &[u8],
    hourly_days: usize,
    daily_days: usize,
    monthly_months: usize,
) -> Result<(&'static str, usize, &'static str)> {
    let (frequency, standard) =
        sampling_limit(xml, hourly_days, daily_days, monthly_months)?;
    let mut reader = Reader::from_reader(xml);
    let mut depth = 0usize;
    let mut current = 0usize;
    let mut maximum = 0usize;
    loop {
        match reader.read_event()? {
            Event::Start(e) => match e.local_name().as_ref() {
                "IntervalBlock" => {
                    depth += 1;
                    if depth == 1 {
                        current = 0;
                    }
                }
                "IntervalReading" if depth > 0 => current += 1,
                _ => {}
            },
            Event::Empty(e) if depth > 0 => {
                if e.local_name().as_ref() == "IntervalReading" {
                    current += 1;
                }
            }
            Event::End(e) if e.local_name().as_ref() == "IntervalBlock" => {
                if depth == 0 {
                    return Err("unbalanced IntervalBlock".into());
                }
                if depth == 1 {
                    maximum = maximum.max(current);
                }
                depth -= 1;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if depth != 0 || maximum == 0 {
        return Err("no interval readings in XML blocks".into());
    }
    let short_limit = match frequency {
        "hourly" => 31 * 24,
        "daily" => 31,
        "monthly" => 3,
        _ => return Err("unsupported measurement frequency".into()),
    };
    if maximum <= short_limit {
        Ok((frequency, maximum, "complete short coverage"))
    } else {
        Ok((frequency, standard, "representative long history"))
    }
}

/// Prepare a related set containing different measurement frequencies.
/// A single key preserves repeated UUID replacements across every file.
pub fn prepare_representative_mixed_set(
    inputs: &[std::path::PathBuf],
    destination: &Path,
    secret: &str,
    limits: &[usize],
) -> Result<BTreeMap<String, Evidence>> {
    if inputs.is_empty()
        || inputs.len() != limits.len()
        || secret.len() < 32
        || limits.contains(&0)
    {
        return Err("invalid input, secret or sampling limits".into());
    }
    if destination.exists() {
        return Err("destination already exists".into());
    }
    let mut candidates = Vec::new();
    let mut reports = BTreeMap::new();
    for (input, limit) in inputs.iter().zip(limits) {
        let name = input
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("invalid input filename")?
            .to_string();
        if !name.ends_with(".xml") || reports.contains_key(&name) {
            return Err("unique XML filenames required".into());
        }
        let bytes = fs::read(input)?;
        let (minimized, _) = minimize_interval_readings(&bytes, *limit)?;
        let (candidate, _) = sanitize_uuid_references(&minimized, secret)?;
        reports.insert(name.clone(), evaluate(&candidate)?);
        candidates.push((name, candidate));
    }
    fs::create_dir(destination)?;
    let result = (|| -> Result<()> {
        for (name, bytes) in candidates {
            fs::write(destination.join(name), bytes)?;
        }
        fs::write(
            destination.join("privacy_evidence.json"),
            serde_json::to_vec_pretty(&reports)?,
        )?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(destination);
    }
    result?;
    Ok(reports)
}
