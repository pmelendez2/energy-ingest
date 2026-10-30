<!-- SPDX-License-Identifier: MIT -->
# Sanitization graph integrity integration

Audience: maintainers of `fixture_privacy`.

Purpose: bind the graph verifier to generated candidates rather than
accepting an arbitrary user-supplied correspondence map.

## Integration

Add to `crates/fixture_privacy/src/lib.rs` with existing module declarations:

```rust
mod sanitization_graph;
pub use sanitization_graph::{
    VerifiedCandidateSet,
    sanitize_set_verified,
};
```

The implementation is additive: it does not replace the existing
`sanitize_set`, `prepare-set`, or `verify-set` commands. To integrate with
set generation, collect source XML into memory and call
`sanitize_set_verified(&originals, secret)?` **before writing any files**.
Use `verified.candidates` in place of the existing raw candidate bytes.
Do not write private correspondence mappings or source XML to reports.

The function obtains expected mappings by passing source IDs and link
hrefs through the same deterministic UUID sanitizer as the source XML.
It verifies exact direct Atom entry links and rejects changed results.
All files must use the same secret. No source file is modified.

## Important limits

* Exact graph checks cover entry IDs and direct entry links only.
* The workflow does not establish that all PII was sanitized.
* A UUID-only transformation may leave sensitive URL path segments,
  account numbers, time-series readings, or other personal data.
* A privacy-safe report must not include source IDs, hrefs, secrets,
  or the correspondence mapping.
* `publication_authorized` remains false; owner and independent
  reviewer approval remain mandatory under ADR-0028.
* The six original SDG&E alias conflicts are an unresolved observation,
  not an established provider defect.

## Validation on sparta

```bash
cargo fmt --all
cargo check -p fixture_privacy --all-targets
cargo test -p fixture_privacy
cargo clippy -p fixture_privacy --all-targets -- -D warnings
git diff --check
```
