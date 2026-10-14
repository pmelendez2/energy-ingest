// SPDX-License-Identifier: MIT
//! Private identity correspondence and exact edge preservation tests.

use fixture_privacy::{IdentityMap, verify_graph};

fn map() -> IdentityMap {
    [
        ("source-a", "candidate-a"),
        ("source-b", "candidate-b"),
        ("https://example.invalid/UsagePoint/1", "urn:test:one"),
        ("https://example.invalid/UsagePoint/2", "urn:test:two"),
    ]
    .into_iter()
    .map(|(a, b)| (a.into(), b.into()))
    .collect()
}

#[test]
fn mapped_graph_is_preserved() {
    let source = br#"<feed>
      <entry><id>source-a</id><link rel="self"
        href="https://example.invalid/UsagePoint/1"/></entry>
      <entry><id>source-b</id><link rel="up"
        href="https://example.invalid/UsagePoint/1"/></entry>
    </feed>"#;
    let candidate = br#"<feed>
      <entry><id>candidate-b</id><link rel="up"
        href="urn:test:one"/></entry>
      <entry><id>candidate-a</id><link rel="self"
        href="urn:test:one"/></entry>
    </feed>"#;
    let report = verify_graph(&[source], &[candidate], &map()).unwrap();
    assert!(report.exact_relationships_preserved);
    assert!(!report.publication_authorized);
}

#[test]
fn swapped_links_with_equal_counts_are_rejected() {
    let source = br#"<feed>
      <entry><id>source-a</id><link rel="related"
        href="https://example.invalid/UsagePoint/1"/></entry>
      <entry><id>source-b</id><link rel="related"
        href="https://example.invalid/UsagePoint/2"/></entry>
    </feed>"#;
    let swapped = br#"<feed>
      <entry><id>candidate-a</id><link rel="related"
        href="urn:test:two"/></entry>
      <entry><id>candidate-b</id><link rel="related"
        href="urn:test:one"/></entry>
    </feed>"#;
    let report = verify_graph(&[source], &[swapped], &map()).unwrap();
    assert_eq!(report.source_links, report.candidate_links);
    assert!(!report.exact_relationships_preserved);
}

#[test]
fn missing_mapping_fails_without_leaking_identifiers() {
    let source = br#"<feed><entry><id>private-id</id></entry></feed>"#;
    let candidate = br#"<feed><entry><id>candidate</id></entry></feed>"#;
    let error =
        verify_graph(&[source], &[candidate], &map()).unwrap_err().to_string();
    assert!(!error.contains("private-id"));
}

#[test]
fn shared_self_url_ambiguity_is_preserved() {
    let source = br#"<feed>
      <entry><id>source-a</id><link rel="self"
        href="https://example.invalid/UsagePoint/1"/></entry>
      <entry><id>source-b</id><link rel="self"
        href="https://example.invalid/UsagePoint/1"/></entry>
    </feed>"#;
    let candidate = br#"<feed>
      <entry><id>candidate-a</id><link rel="self"
        href="urn:test:one"/></entry>
      <entry><id>candidate-b</id><link rel="self"
        href="urn:test:one"/></entry>
    </feed>"#;
    assert!(
        verify_graph(&[source], &[candidate], &map())
            .unwrap()
            .exact_relationships_preserved
    );
}
