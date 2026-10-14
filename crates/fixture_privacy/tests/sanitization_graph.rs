// SPDX-License-Identifier: MIT
use fixture_privacy::sanitize_set_verified;

const SECRET: &str = "synthetic-test-only-secret-at-least-32-bytes";

fn document(id: &str, href: &str) -> Vec<u8> {
    format!(
        "<feed><entry><id>urn:uuid:{id}</id>\
         <link rel=\"self\" href=\"{href}\"/>\
         </entry></feed>"
    )
    .into_bytes()
}

#[test]
fn two_documents_with_shared_resource_remain_valid() {
    let url = "https://example.invalid/ReadingType/\
        11111111-1111-4111-8111-111111111111";
    let inputs = vec![
        document("22222222-2222-4222-8222-222222222222", url),
        document("33333333-3333-4333-8333-333333333333", url),
    ];
    let result = sanitize_set_verified(&inputs, SECRET).unwrap();
    assert_eq!(result.graph_report.source_entries, 2);
    assert_eq!(result.graph_report.source_links, 2);
    assert!(result.graph_report.exact_relationships_preserved);
    assert!(!result.graph_report.publication_authorized);
    let output = String::from_utf8(result.candidates[0].clone()).unwrap();
    assert!(!output.contains("22222222-2222-4222-8222-222222222222"));
    assert!(!output.contains("11111111-1111-4111-8111-111111111111"));
}

#[test]
fn incomplete_set_does_not_return_partial_candidates() {
    let valid = document(
        "22222222-2222-4222-8222-222222222222",
        "https://example.invalid/UsagePoint/4",
    );
    let invalid = b"<!DOCTYPE feed><feed/>".to_vec();
    assert!(sanitize_set_verified(&[valid, invalid], SECRET).is_err());
}

#[test]
fn weak_secret_is_rejected() {
    let valid = document(
        "22222222-2222-4222-8222-222222222222",
        "https://example.invalid/UsagePoint/4",
    );
    assert!(sanitize_set_verified(&[valid], "weak").is_err());
}
