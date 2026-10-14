// SPDX-License-Identifier: MIT
//! End-to-end checks for private sampled-set generation and reverification.

use fixture_privacy::{
    SamplingPolicy, generate_verified_minimized_set, verify_generated_set,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);
const SECRET: &str = "testing-only-private-secret-32-chars-minimum";

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let number = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("fixture-sample-{}-{number}", std::process::id(),));
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("input")).unwrap();
        Self(path)
    }
    fn input(&self) -> PathBuf {
        self.0.join("input")
    }
    fn output(&self) -> PathBuf {
        self.0.join("output")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn document(readings: usize, duration: usize) -> String {
    let reading = format!(
        "<IntervalReading><timePeriod><duration>{duration}</duration>\
         </timePeriod><value>1</value></IntervalReading>"
    );
    format!(
        "<feed><entry><id>urn:uuid:first</id>\
         <link rel=\"self\" href=\"https://example.invalid/UsagePoint/1\"/>\
         <content><IntervalBlock>{}</IntervalBlock></content>\
         </entry></feed>",
        reading.repeat(readings),
    )
}

#[test]
fn large_history_is_minimized_and_reverified() {
    let work = Workspace::new();
    fs::write(work.input().join("electric.xml"), document(750, 3600)).unwrap();
    let report = generate_verified_minimized_set(
        &work.input(),
        &work.output(),
        SECRET,
        SamplingPolicy::default(),
    )
    .unwrap();
    let selection = report.sampling.unwrap();
    assert_eq!(selection.retained_readings, 168);
    assert_eq!(selection.removed_readings, 582);
    let verified =
        verify_generated_set(&work.input(), &work.output(), SECRET).unwrap();
    assert!(!verified.publication_authorized);
    assert_eq!(verified.sampling.unwrap(), selection);
}

#[test]
fn unsupported_frequency_fails_without_writing() {
    let work = Workspace::new();
    fs::write(work.input().join("x.xml"), document(10, 12345)).unwrap();
    assert!(
        generate_verified_minimized_set(
            &work.input(),
            &work.output(),
            SECRET,
            SamplingPolicy::default(),
        )
        .is_err()
    );
    assert!(!work.output().exists());
}

#[test]
fn invalid_policy_fails_without_writing() {
    let work = Workspace::new();
    fs::write(work.input().join("x.xml"), document(12, 3600)).unwrap();
    assert!(
        generate_verified_minimized_set(
            &work.input(),
            &work.output(),
            SECRET,
            SamplingPolicy { hourly_days: 0, ..SamplingPolicy::default() },
        )
        .is_err()
    );
    assert!(!work.output().exists());
}

#[test]
fn modified_candidate_fails_reverification() {
    let work = Workspace::new();
    fs::write(work.input().join("x.xml"), document(750, 3600)).unwrap();
    generate_verified_minimized_set(
        &work.input(),
        &work.output(),
        SECRET,
        SamplingPolicy::default(),
    )
    .unwrap();
    fs::write(work.output().join("x.xml"), document(750, 3600)).unwrap();
    assert!(
        verify_generated_set(&work.input(), &work.output(), SECRET,).is_err()
    );
}

#[test]
fn per_file_operations_are_saved_and_reverified() {
    let work = Workspace::new();
    fs::write(work.input().join("electric.xml"), document(750, 3600)).unwrap();
    let result = generate_verified_minimized_set(
        &work.input(),
        &work.output(),
        SECRET,
        SamplingPolicy::default(),
    )
    .unwrap();
    assert_eq!(result.file_results.len(), 1);
    let item = &result.file_results[0];
    assert_eq!(item.filename, "electric.xml");
    assert_eq!(item.readings_found, 750);
    assert_eq!(item.readings_retained + item.readings_removed, 750);
    assert!(item.operations.iter().any(|s| s.contains("graph")));
    assert_eq!(
        result.file_results,
        verify_generated_set(&work.input(), &work.output(), SECRET)
            .unwrap()
            .file_results,
    );
}

#[test]
fn tampered_file_operations_fail_verification() {
    let work = Workspace::new();
    fs::write(work.input().join("electric.xml"), document(750, 3600)).unwrap();
    generate_verified_minimized_set(
        &work.input(),
        &work.output(),
        SECRET,
        SamplingPolicy::default(),
    )
    .unwrap();
    let path = work.output().join("verified_set_evidence.json");
    let mut json: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    json["file_results"][0]["readings_removed"] = serde_json::json!(0);
    fs::write(&path, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    assert!(
        verify_generated_set(&work.input(), &work.output(), SECRET,).is_err()
    );
}
