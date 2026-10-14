// SPDX-License-Identifier: MIT
//! Synthetic tests for offline XML reference diagnostics.

use fixture_privacy::{compare_relationships, inspect_relationships};

#[test]
fn linked_fragments_are_resolved_across_documents() {
    let a = br#"<feed><entry><id>meter-1</id></entry></feed>"#;
    let b = br##"<feed><link href="#meter-1"/></feed>"##;
    let result = inspect_relationships(&[a, b]).unwrap();
    assert_eq!(result.documents, 2);
    assert_eq!(result.references, 1);
    assert_eq!(result.unresolved_local_references, 0);
}

#[test]
fn missing_local_reference_is_reported_without_exposing_value() {
    let xml = br##"<feed><link href="#private-account-id"/></feed>"##;
    let report = inspect_relationships(&[xml]).unwrap();
    assert_eq!(report.unresolved_local_references, 1);
    let public = serde_json::to_string(&report).unwrap();
    assert!(!public.contains("private-account-id"));
}

#[test]
fn provider_urls_are_not_assumed_to_be_missing_local_ids() {
    let xml =
        br#"<feed><link href="https://example.invalid/UsagePoint/1"/></feed>"#;
    let report = inspect_relationships(&[xml]).unwrap();
    assert_eq!(report.external_references, 1);
    assert_eq!(report.unresolved_local_references, 0);
}

#[test]
fn malformed_xml_is_rejected() {
    assert!(inspect_relationships(&[b"<feed><id>x</feed>"]).is_err());
}

/// Absolute URLs can refer to resources in the same offline export.
#[test]
fn absolute_resource_links_resolve_across_documents() {
    let resource = br#"<feed><entry><id>https://example.invalid/UsagePoint/42</id></entry></feed>"#;
    let links =
        br#"<feed><link href="https://example.invalid/UsagePoint/42"/></feed>"#;
    let report = inspect_relationships(&[resource, links]).unwrap();
    assert_eq!(report.resolved_references, 1);
    assert_eq!(report.external_references, 0);
}

/// Provider references absent from partial exports remain out-of-set.
#[test]
fn unrepresented_absolute_links_remain_external() {
    let xml =
        br#"<feed><link href="https://example.invalid/UsagePoint/99"/></feed>"#;
    let report = inspect_relationships(&[xml]).unwrap();
    assert_eq!(report.external_references, 1);
    assert_eq!(report.unresolved_local_references, 0);
}

/// URI-valued identifiers should be supported without exposing them.
#[test]
fn urn_identifiers_resolve_when_present() {
    let xml = br#"<feed><entry><id>urn:uuid:11111111-1111-1111-1111-111111111111</id><link href="urn:uuid:11111111-1111-1111-1111-111111111111"/></entry></feed>"#;
    let report = inspect_relationships(&[xml]).unwrap();
    assert_eq!(report.resolved_references, 1);
    assert!(!serde_json::to_string(&report).unwrap().contains("11111111"));
}

/// The Atom UUID and `self` HTTPS URL describe the same resource.
#[test]
fn self_url_is_indexed_as_an_alias_for_entry_id() {
    let a = br#"<feed><entry><id>urn:uuid:test-one</id><link rel="self" href="https://example.invalid/UsagePoint/1"/></entry></feed>"#;
    let b = br#"<feed><entry><id>urn:uuid:test-two</id><link rel="up" href="https://example.invalid/UsagePoint/1"/></entry></feed>"#;
    let report = inspect_relationships(&[a, b]).unwrap();
    assert_eq!(report.resolved_references, 2);
    assert_eq!(report.external_references, 0);
}

/// Repeated exports of the same entry are not conflicting by themselves.
#[test]
fn repeated_identical_resource_is_not_a_conflict() {
    let xml = br#"<feed><entry><id>urn:uuid:test-one</id><link rel="self" href="https://example.invalid/UsagePoint/1"/></entry></feed>"#;
    let report = inspect_relationships(&[xml, xml]).unwrap();
    assert_eq!(report.duplicate_identifiers, 1);
    assert_eq!(report.conflicting_identifiers, 0);
    assert_eq!(report.conflicting_self_links, 0);
}

/// Conflicting identity relationships require review, not auto-correction.
#[test]
fn conflicting_resource_aliases_are_counted_without_disclosure() {
    let a = br#"<feed><entry><id>urn:uuid:a</id><link rel="self" href="https://example.invalid/UsagePoint/1"/></entry></feed>"#;
    let b = br#"<feed><entry><id>urn:uuid:a</id><link rel="self" href="https://example.invalid/UsagePoint/2"/></entry></feed>"#;
    let c = br#"<feed><entry><id>urn:uuid:b</id><link rel="self" href="https://example.invalid/UsagePoint/1"/></entry></feed>"#;
    let report = inspect_relationships(&[a, b, c]).unwrap();
    assert_eq!(report.conflicting_identifiers, 1);
    assert_eq!(report.conflicting_self_links, 1);
    let public = serde_json::to_string(&report).unwrap();
    assert!(!public.contains("example.invalid"));
    assert!(!public.contains("urn:uuid"));
}

/// Repeated copies of the same resource do not create alias conflicts.
#[test]
fn repeated_pair_is_distinguished_from_alias_conflict() {
    let a = br#"<feed><entry><id>urn:uuid:test</id><link rel="self" href="https://example.invalid/A"/></entry></feed>"#;
    let report = inspect_relationships(&[a, a]).unwrap();
    assert_eq!(report.repeated_resource_pairs, 1);
    assert_eq!(report.self_urls_with_multiple_ids, 0);
    assert_eq!(report.ids_with_multiple_self_urls, 0);
}

/// One URL shared by different IDs is distinct from a repeated pair.
#[test]
fn conflicting_aliases_are_classified_separately() {
    let a = br#"<feed><entry><id>urn:uuid:first</id><link rel="self" href="https://example.invalid/A"/></entry></feed>"#;
    let b = br#"<feed><entry><id>urn:uuid:second</id><link rel="self" href="https://example.invalid/A"/></entry></feed>"#;
    let report = inspect_relationships(&[a, b]).unwrap();
    assert_eq!(report.repeated_resource_pairs, 0);
    assert_eq!(report.self_urls_with_multiple_ids, 1);
    assert_eq!(report.ids_with_multiple_self_urls, 0);
}

/// Aggregate equality does not constitute privacy or graph clearance.
#[test]
fn same_counts_still_require_semantic_review() {
    let a = br#"<feed><entry><id>id-a</id><link rel="self" href="https://example.invalid/A"/></entry></feed>"#;
    let b = br#"<feed><entry><id>id-b</id><link rel="self" href="https://example.invalid/B"/></entry></feed>"#;
    let comparison = compare_relationships(&[a], &[b]).unwrap();
    assert!(comparison.structural_counts_match);
    assert!(comparison.regression_review_required);
    let serialized = serde_json::to_string(&comparison).unwrap();
    assert!(!serialized.contains("example.invalid"));
}

#[test]
fn missing_relationship_is_detected_across_sets() {
    let before = br#"<feed><entry><id>id-a</id><link rel="self" href="https://example.invalid/A"/></entry></feed>"#;
    let after = br#"<feed><entry><id>id-a</id></entry></feed>"#;
    let comparison = compare_relationships(&[before], &[after]).unwrap();
    assert!(!comparison.structural_counts_match);
}

/// Classify a repeated URL without revealing the associated Atom IDs.
#[test]
fn self_link_conflicts_are_grouped_by_resource_type() {
    let xml = br#"<feed>
      <entry><id>urn:uuid:first</id>
        <link rel="self"
          href="https://example.invalid/UsagePoint/42"/></entry>
      <entry><id>urn:uuid:second</id>
        <link rel="self"
          href="https://example.invalid/UsagePoint/42"/></entry>
    </feed>"#;
    let report = inspect_relationships(&[xml]).unwrap();
    assert_eq!(report.self_urls_with_multiple_ids, 1);
    assert_eq!(report.conflicting_self_link_types.get("UsagePoint"), Some(&1),);
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("example.invalid"));
    assert!(!json.contains("urn:uuid"));
}

/// Provider-specific path segments must never appear in public JSON.
#[test]
fn unknown_resource_paths_do_not_leak() {
    let xml = br#"<feed>
      <entry><id>urn:uuid:first</id>
        <link rel="self"
          href="https://example.invalid/private-account/42"/></entry>
      <entry><id>urn:uuid:second</id>
        <link rel="self"
          href="https://example.invalid/private-account/42"/></entry>
    </feed>"#;
    let report = inspect_relationships(&[xml]).unwrap();
    assert_eq!(report.conflicting_self_link_types.get("other"), Some(&1));
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("private-account"));
    assert!(!json.contains("example.invalid"));
}

/// The before/after comparison considers category changes too.
#[test]
fn changed_conflict_resource_type_is_detected() {
    let a = br#"<feed>
      <entry><id>id-1</id>
        <link rel="self" href="https://example.invalid/UsagePoint/1"/></entry>
      <entry><id>id-2</id>
        <link rel="self" href="https://example.invalid/UsagePoint/1"/></entry>
    </feed>"#;
    let b = br#"<feed>
      <entry><id>id-1</id>
        <link rel="self" href="https://example.invalid/ReadingType/1"/></entry>
      <entry><id>id-2</id>
        <link rel="self" href="https://example.invalid/ReadingType/1"/></entry>
    </feed>"#;
    let comparison = compare_relationships(&[a], &[b]).unwrap();
    assert!(!comparison.structural_counts_match);
    assert!(comparison.regression_review_required);
}
