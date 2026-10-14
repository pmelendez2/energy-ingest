<!-- SPDX-License-Identifier: MIT -->
# Rust privacy engine: implementation status

**Audience:** Developers, data providers, and maintainers.

**Purpose:** Describe executable privacy checks and current limitations.

This patch introduces the **real Rust** `fixture_privacy` crate, not a
Python prototype. It uses `quick-xml` to validate XML syntax, SHA-256
for fingerprints and secret-keyed deterministic replacement of UUID
references across separate documents.

Build and test from the repository root:

```bash
cargo test -p fixture_privacy
cargo run -p fixture_privacy -- audit input.xml
FIXTURE_PRIVACY_SECRET="$(openssl rand -hex 32)" \
  cargo run -p fixture_privacy -- sanitize input.xml \
  candidate.xml candidate.report.json
cargo run -p fixture_privacy -- verify \
  candidate.xml candidate.report.json
```

For related documents, retain the same private secret during preparation.
Do not place the secret in source control, reports, or archives.
The report records the Cargo package version (`tool_version`), candidate
hashes, remaining risk categories and a
`publication_authorized: false` marker. It **does not** authorize sharing.

**Not yet sufficient for ADR-0028 completion:** Customer-number segments
within provider URLs, customer names, all account/contact metadata, high
resolution consumption/time-series fingerprinting, complete XML namespace
semantics, dataset-wide referential integrity, and a secure evidence-based
publication gate require additional implementation and tests. The present
transformation preserves UUID link equality but does not by itself
establish privacy safety or Green Button semantic correctness.

Original SDG&E XML files must remain private and unchanged. Do not use
`--include-fixtures` or publish candidate fixtures until the above gaps
are closed, the data owner has authorized submission, and maintainers
have independently reviewed the result.

## Architectural references and software versioning

ADR-0028 is a reference in source documentation, not a runtime
constant. It is superseded by a later ADR if the architecture changes.
There is no independent privacy policy version. Evidence records
`tool_version` from Cargo's `CARGO_PKG_VERSION`, plus a SHA-256 digest of
the candidate. Verification rejects evidence with an incompatible tool
version, recalculates the digest and runs the current risk checks. Tool
version does not by itself prove privacy safety or authentic provenance.

Maintainers update `crates/fixture_privacy/Cargo.toml` when preparing a
new published software version, under the existing release process.
Behavior changes require regression tests and new verification evidence;
the team does not separately increment a manually maintained policy ID.
