// SPDX-License-Identifier: MIT
//! Verified private candidate-set workflow.
//!
//! Generation runs sanitization and exact Atom graph comparison in memory
//! before writing. Evidence deliberately excludes original identifiers,
//! private mappings, credentials, and input paths. Neither successful
//! generation nor verification authorizes publication under ADR-0028.

use crate::{
    Evidence, Result, evaluate, minimize_interval_readings,
    regression_sampling_limit, sanitize_set_verified, sha256, verify_candidate,
    verify_unchanged_graph,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Documented selection policy for a private sampled fixture set.
/// Defaults retain a week of hourly/daily data and three monthly points.
/// A short complete resource is retained rather than arbitrarily truncated.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SamplingPolicy {
    pub hourly_days: usize,
    pub daily_days: usize,
    pub monthly_months: usize,
}

impl Default for SamplingPolicy {
    fn default() -> Self {
        Self { hourly_days: 7, daily_days: 7, monthly_months: 3 }
    }
}

impl SamplingPolicy {
    fn validate(self) -> Result<()> {
        if self.hourly_days == 0
            || self.daily_days == 0
            || self.monthly_months == 0
        {
            return Err("sampling limits must be positive".into());
        }
        Ok(())
    }
}

/// Aggregate sampling evidence; never includes meter readings or IDs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SamplingEvidence {
    pub policy: SamplingPolicy,
    pub retained_readings: usize,
    pub removed_readings: usize,
    pub minimized_set_sha256: String,
}

/// Private per-file work record for an auditable fixture preparation run.
/// Filenames stay in the private evidence, never the shareable summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileProcessingResult {
    pub filename: String,
    pub readings_found: usize,
    pub readings_retained: usize,
    pub readings_removed: usize,
    pub operations: Vec<String>,
}

/// Named XML documents held in memory during candidate generation.
type XmlDocumentSet = Vec<(String, Vec<u8>)>;

/// Select readings before sanitization, then compare the entire Atom
/// entry/link multiset before and after selection. Unsupported durations
/// fail closed rather than silently leaving an unminimized history.
fn minimize_set(
    source: &[(String, Vec<u8>)],
    policy: SamplingPolicy,
) -> Result<(XmlDocumentSet, SamplingEvidence, Vec<FileProcessingResult>)> {
    policy.validate()?;
    let mut outputs = Vec::new();
    let mut kept = 0usize;
    let mut removed = 0usize;
    let mut rows = Vec::new();
    for (name, xml) in source {
        let (_, limit, _) = regression_sampling_limit(
            xml,
            policy.hourly_days,
            policy.daily_days,
            policy.monthly_months,
        )?;
        let (candidate, summary) = minimize_interval_readings(xml, limit)?;
        kept = kept
            .checked_add(summary.retained)
            .ok_or("reading count overflow")?;
        removed = removed
            .checked_add(summary.removed)
            .ok_or("reading count overflow")?;
        rows.push(FileProcessingResult {
            filename: name.clone(),
            readings_found: summary.retained + summary.removed,
            readings_retained: summary.retained,
            readings_removed: summary.removed,
            operations: vec![
                "history minimized".into(),
                "identifiers pseudonymized".into(),
                "Atom graph verified".into(),
            ],
        });
        outputs.push((name.clone(), candidate));
    }
    let originals: Vec<&[u8]> =
        source.iter().map(|(_, xml)| xml.as_slice()).collect();
    let minimized: Vec<&[u8]> =
        outputs.iter().map(|(_, xml)| xml.as_slice()).collect();
    // The minimizer must not erase or rewire Atom resource entries.
    if !verify_unchanged_graph(&originals, &minimized)? {
        return Err("sampling changed Atom relationships".into());
    }
    let evidence = SamplingEvidence {
        policy,
        retained_readings: kept,
        removed_readings: removed,
        minimized_set_sha256: set_digest(&outputs),
    };
    Ok((outputs, evidence, rows))
}

/// The evidence format is tied to the crate version, not a second policy
/// version. Hashes detect changes; they do not demonstrate privacy safety.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedSetEvidence {
    pub tool_version: String,
    pub source_set_sha256: String,
    pub candidate_set_sha256: String,
    pub graph: crate::GraphVerificationReport,
    #[serde(default)]
    pub sampling: Option<SamplingEvidence>,
    pub candidates: BTreeMap<String, Evidence>,
    pub file_results: Vec<FileProcessingResult>,
    pub privacy_review_complete: bool,
    pub publication_authorized: bool,
}

/// A deterministic digest of file names and contents, preventing a hash
/// collision caused by ambiguous concatenation of file boundaries.
fn set_digest(files: &[(String, Vec<u8>)]) -> String {
    let mut stream = Vec::new();
    for (name, content) in files {
        stream.extend_from_slice(&(name.len() as u64).to_be_bytes());
        stream.extend_from_slice(name.as_bytes());
        stream.extend_from_slice(&(content.len() as u64).to_be_bytes());
        stream.extend_from_slice(content);
    }
    sha256(&stream)
}

/// Load a nonrecursive XML directory in reproducible filename order.
/// Symlinks and nonregular XML entries are refused rather than followed.
fn read_set(dir: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let mut files = Vec::new();
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "XML filename must be UTF-8")?;
        if !name.ends_with(".xml") {
            continue;
        }
        if !entry.file_type()?.is_file() {
            return Err("XML input must be a regular file".into());
        }
        if !names.insert(name.clone()) {
            return Err("duplicate XML filename".into());
        }
        files.push((name, fs::read(entry.path())?));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    if files.is_empty() {
        return Err("no XML files found".into());
    }
    Ok(files)
}

/// Resolve an output whose final component does not exist. Reject symlinked
/// source directories and existing output paths before any file is created.
/// Lexical `..` in the output is refused to prevent ambiguous paths.
fn ensure_disjoint(input: &Path, output: &Path) -> Result<PathBuf> {
    if fs::symlink_metadata(input)?.file_type().is_symlink() {
        return Err("source directory symlink refused".into());
    }
    let source = fs::canonicalize(input)?;
    if !fs::metadata(&source)?.is_dir() {
        return Err("source must be a directory".into());
    }
    if output.components().any(|part| part == std::path::Component::ParentDir) {
        return Err("output may not contain parent traversal".into());
    }
    if fs::symlink_metadata(output).is_ok() {
        return Err("destination already exists".into());
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = fs::canonicalize(parent)?;
    let name = output.file_name().ok_or("invalid destination name")?;
    let resolved = parent.join(name);
    if resolved.starts_with(&source) || source.starts_with(&resolved) {
        return Err("source and destination must be separate".into());
    }
    Ok(resolved)
}

/// Shareable review status: deliberately excludes all source names, digests,
/// raw identifiers, original URLs, and private mapping information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareableReviewSummary {
    pub tool_version: String,
    pub documents: usize,
    pub relationship_integrity_passed: bool,
    pub files_requiring_privacy_review: usize,
    pub sampling_applied: bool,
    pub readings_removed: usize,
    pub privacy_review_complete: bool,
    pub publication_authorized: bool,
    pub blockers: Vec<String>,
}

fn shareable_summary(e: &VerifiedSetEvidence) -> ShareableReviewSummary {
    ShareableReviewSummary {
        tool_version: e.tool_version.clone(),
        documents: e.candidates.len(),
        relationship_integrity_passed: e.graph.exact_relationships_preserved,
        files_requiring_privacy_review: e
            .candidates
            .values()
            .filter(|item| !item.residual_risks.is_empty())
            .count(),
        sampling_applied: e.sampling.is_some(),
        readings_removed: e
            .sampling
            .as_ref()
            .map_or(0, |sampling| sampling.removed_readings),
        privacy_review_complete: false,
        publication_authorized: false,
        blockers: vec![
            "Residual customer-identification review incomplete".into(),
            "Usage-pattern reidentification review incomplete".into(),
            "ESPI resource semantics and coverage review incomplete".into(),
            "Human reviewer authorization is required".into(),
        ],
    }
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Produce a complete private set or return an error without leaving a
/// partially usable output. Parent directories must already exist.
///
/// A successful result proves Atom-graph consistency only. The residual
/// risk review, ESPI semantics, and human approval are still mandatory.
/// The destination must not already exist.
pub fn generate_verified_set(
    source_dir: &Path,
    destination: &Path,
    secret: &str,
) -> Result<VerifiedSetEvidence> {
    generate_with_policy(source_dir, destination, secret, None)
}

/// Generate a private sampled-and-verified candidate set.
/// Publication remains blocked regardless of the reported risk count.
pub fn generate_verified_minimized_set(
    source_dir: &Path,
    destination: &Path,
    secret: &str,
    policy: SamplingPolicy,
) -> Result<VerifiedSetEvidence> {
    generate_with_policy(source_dir, destination, secret, Some(policy))
}

fn generate_with_policy(
    source_dir: &Path,
    destination: &Path,
    secret: &str,
    policy: Option<SamplingPolicy>,
) -> Result<VerifiedSetEvidence> {
    if secret.len() < 32 {
        return Err("private secret must have at least 32 characters".into());
    }
    let destination = ensure_disjoint(source_dir, destination)?;
    let source = read_set(source_dir)?;
    let (selected, sampling, mut file_results) = if let Some(policy) = policy {
        let (selected, evidence, rows) = minimize_set(&source, policy)?;
        (selected, Some(evidence), rows)
    } else {
        (source.clone(), None, Vec::new())
    };
    let originals: Vec<Vec<u8>> =
        selected.iter().map(|(_, bytes)| bytes.clone()).collect();
    let verified = sanitize_set_verified(&originals, secret)?;
    let mut candidates = BTreeMap::new();
    let mut outputs = Vec::new();
    for ((name, _), xml) in source.iter().zip(verified.candidates) {
        candidates.insert(name.clone(), evaluate(&xml)?);
        outputs.push((name.clone(), xml));
    }
    // All files have now passed in-memory sanitization and graph checks.
    // Use the same private names as the source input set.
    if file_results.is_empty() {
        file_results = source
            .iter()
            .map(|(name, _)| FileProcessingResult {
                filename: name.clone(),
                readings_found: 0,
                readings_retained: 0,
                readings_removed: 0,
                operations: vec![
                    "identifiers pseudonymized".into(),
                    "Atom graph verified".into(),
                ],
            })
            .collect();
    }
    let evidence = VerifiedSetEvidence {
        tool_version: env!("CARGO_PKG_VERSION").into(),
        source_set_sha256: set_digest(&source),
        candidate_set_sha256: set_digest(&outputs),
        graph: verified.graph_report,
        sampling,
        candidates,
        file_results,
        privacy_review_complete: false,
        publication_authorized: false,
    };
    if !evidence.graph.exact_relationships_preserved {
        return Err("exact relationship graph changed".into());
    }
    // Create an unpublished staging directory in the same parent filesystem.
    // `create_dir` never replaces a preexisting destination.
    let staging = destination.with_file_name(format!(
        ".{}.private-staging-{}",
        destination.file_name().ok_or("invalid destination")?.to_string_lossy(),
        std::process::id(),
    ));
    if fs::symlink_metadata(&staging).is_ok() {
        return Err("staging path already exists".into());
    }
    fs::create_dir(&staging)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(error) =
            fs::set_permissions(&staging, fs::Permissions::from_mode(0o700))
        {
            let _ = fs::remove_dir(&staging);
            return Err(error.into());
        }
    }
    let result = (|| -> Result<()> {
        for (name, xml) in &outputs {
            write_private(&staging.join(name), xml)?;
        }
        let json = serde_json::to_vec_pretty(&evidence)?;
        write_private(&staging.join("verified_set_evidence.json"), &json)?;
        let public = serde_json::to_vec_pretty(&shareable_summary(&evidence))?;
        write_private(&staging.join("shareable_review_summary.json"), &public)?;
        if fs::symlink_metadata(&destination).is_ok() {
            return Err("destination appeared during staging".into());
        }
        fs::rename(&staging, &destination)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result?;
    Ok(evidence)
}

/// Verify a persisted private set against its original input directory.
/// This recomputes sanitizer output and the exact graph, not just counts.
/// Both sets must be local; never supply private identifiers to logs.
pub fn verify_generated_set(
    source_dir: &Path,
    destination: &Path,
    secret: &str,
) -> Result<VerifiedSetEvidence> {
    let destination = fs::canonicalize(destination)?;
    let source = fs::canonicalize(source_dir)?;
    if destination.starts_with(&source) || source.starts_with(&destination) {
        return Err("source and destination overlap".into());
    }
    let evidence: VerifiedSetEvidence = serde_json::from_slice(&fs::read(
        destination.join("verified_set_evidence.json"),
    )?)?;
    if evidence.tool_version != env!("CARGO_PKG_VERSION")
        || evidence.publication_authorized
        || evidence.privacy_review_complete
    {
        return Err("invalid or unauthorized verification evidence".into());
    }
    let source = read_set(&source)?;
    let candidates = read_set(&destination)?;
    if source.iter().map(|(name, _)| name).collect::<Vec<_>>()
        != candidates.iter().map(|(name, _)| name).collect::<Vec<_>>()
    {
        return Err("candidate filenames differ from source set".into());
    }
    if set_digest(&source) != evidence.source_set_sha256
        || set_digest(&candidates) != evidence.candidate_set_sha256
    {
        return Err("source or candidate set changed".into());
    }
    let published_summary: ShareableReviewSummary = serde_json::from_slice(
        &fs::read(destination.join("shareable_review_summary.json"))?,
    )?;
    let expected_summary = shareable_summary(&evidence);
    if serde_json::to_value(published_summary)?
        != serde_json::to_value(expected_summary)?
    {
        return Err("shareable review summary mismatch".into());
    }
    let selected = if let Some(sampling) = &evidence.sampling {
        let (selected, recalculated, rows) =
            minimize_set(&source, sampling.policy)?;
        if rows != evidence.file_results {
            return Err("file operation evidence mismatch".into());
        }
        if recalculated != *sampling {
            return Err("sampling evidence mismatch".into());
        }
        selected
    } else {
        if evidence.file_results.len() != source.len() {
            return Err("file operation evidence mismatch".into());
        }
        source.clone()
    };
    let originals: Vec<Vec<u8>> =
        selected.iter().map(|(_, bytes)| bytes.clone()).collect();
    let regenerated = sanitize_set_verified(&originals, secret)?;
    for ((name, bytes), expected) in
        candidates.iter().zip(regenerated.candidates)
    {
        if *bytes != expected {
            return Err("candidate differs from verified sanitization".into());
        }
        let record = evidence
            .candidates
            .get(name)
            .ok_or("candidate missing privacy evidence")?;
        verify_candidate(&destination.join(name), record)?;
    }
    if evidence.candidates.len() != candidates.len()
        || !regenerated.graph_report.exact_relationships_preserved
        || serde_json::to_value(&regenerated.graph_report)?
            != serde_json::to_value(&evidence.graph)?
    {
        return Err("relationship evidence mismatch".into());
    }
    Ok(evidence)
}
