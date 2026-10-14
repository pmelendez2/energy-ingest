// SPDX-License-Identifier: MIT
//! End-to-end tests for private verified-set generation and rechecking.

use fixture_privacy::{generate_verified_set, verify_generated_set};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
const SECRET: &str = "testing-only-private-secret-32-chars-minimum";

struct FixtureDirs(PathBuf);

impl FixtureDirs {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "fixture-privacy-verified-{}-{id}",
            std::process::id(),
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("source")).unwrap();
        Self(root)
    }

    fn source(&self) -> PathBuf {
        self.0.join("source")
    }

    fn output(&self) -> PathBuf {
        self.0.join("output")
    }
}

impl Drop for FixtureDirs {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn example(id: &str) -> String {
    format!(
        "<feed><entry><id>urn:uuid:{id}</id>\
        <link rel=\"self\" href=\"https://example.invalid/UsagePoint/{id}\"/>\
        </entry></feed>"
    )
}

#[test]
fn creates_and_rechecks_private_candidate_set() {
    let dirs = FixtureDirs::new();
    fs::write(dirs.source().join("a.xml"), example("first")).unwrap();
    fs::write(dirs.source().join("b.xml"), example("second")).unwrap();
    let report =
        generate_verified_set(&dirs.source(), &dirs.output(), SECRET).unwrap();
    assert!(report.graph.exact_relationships_preserved);
    assert!(!report.publication_authorized);
    assert!(!report.privacy_review_complete);
    verify_generated_set(&dirs.source(), &dirs.output(), SECRET).unwrap();
    assert!(
        generate_verified_set(&dirs.source(), &dirs.output(), SECRET,).is_err()
    );
}

#[test]
fn modification_rejects_reverification() {
    let dirs = FixtureDirs::new();
    fs::write(dirs.source().join("a.xml"), example("first")).unwrap();
    generate_verified_set(&dirs.source(), &dirs.output(), SECRET).unwrap();
    fs::write(dirs.output().join("a.xml"), example("tampered")).unwrap();
    assert!(
        verify_generated_set(&dirs.source(), &dirs.output(), SECRET,).is_err()
    );
}

#[test]
fn malformed_source_does_not_create_destination() {
    let dirs = FixtureDirs::new();
    fs::write(dirs.source().join("bad.xml"), b"<feed>").unwrap();
    assert!(
        generate_verified_set(&dirs.source(), &dirs.output(), SECRET,).is_err()
    );
    assert!(!dirs.output().exists());
}

#[test]
fn incomplete_secret_fails_without_output() {
    let dirs = FixtureDirs::new();
    fs::write(dirs.source().join("a.xml"), example("first")).unwrap();
    assert!(
        generate_verified_set(&dirs.source(), &dirs.output(), "short",)
            .is_err()
    );
    assert!(!dirs.output().exists());
}

/// An output inside the source must be rejected without modifying input.
#[test]
fn nested_output_is_rejected() {
    let dirs = FixtureDirs::new();
    fs::write(dirs.source().join("a.xml"), example("first")).unwrap();
    let nested = dirs.source().join("generated");
    assert!(generate_verified_set(&dirs.source(), &nested, SECRET,).is_err());
    assert!(!nested.exists());
}

/// A relative output containing parent traversal must not bypass checks.
#[test]
fn parent_traversal_output_is_rejected() {
    let dirs = FixtureDirs::new();
    fs::write(dirs.source().join("a.xml"), example("first")).unwrap();
    let traversing = dirs.source().join("..").join("output");
    assert!(
        generate_verified_set(&dirs.source(), &traversing, SECRET,).is_err()
    );
    assert!(!dirs.output().exists());
}

#[test]
fn review_summary_is_non_authorizing_and_rechecked() {
    let dirs = FixtureDirs::new();
    fs::write(dirs.source().join("a.xml"), example("first")).unwrap();
    generate_verified_set(&dirs.source(), &dirs.output(), SECRET).unwrap();
    let path = dirs.output().join("shareable_review_summary.json");
    let summary: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(summary["publication_authorized"], false);
    assert_eq!(summary["privacy_review_complete"], false);
    assert_eq!(summary["documents"], 1);
    assert!(!fs::read_to_string(&path).unwrap().contains("first"));
    let mut tampered = summary;
    tampered["publication_authorized"] = true.into();
    fs::write(&path, serde_json::to_vec(&tampered).unwrap()).unwrap();
    assert!(
        verify_generated_set(&dirs.source(), &dirs.output(), SECRET,).is_err()
    );
}

#[cfg(unix)]
#[test]
fn private_permissions_are_enforced() {
    use std::os::unix::fs::PermissionsExt;
    let dirs = FixtureDirs::new();
    fs::write(dirs.source().join("a.xml"), example("first")).unwrap();
    generate_verified_set(&dirs.source(), &dirs.output(), SECRET).unwrap();
    let mode =
        fs::metadata(dirs.output()).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
    for name in
        ["a.xml", "verified_set_evidence.json", "shareable_review_summary.json"]
    {
        let mode = fs::metadata(dirs.output().join(name))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
}
