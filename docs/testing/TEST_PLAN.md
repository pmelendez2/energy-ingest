<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# Test Plan

**Purpose:** Define verification scope, entry criteria, and exit criteria.

**Intended Audience:** Test authors, developers, and release maintainers.

## Scope

The test plan covers the energy ingestion application, provider adapters,
Green Button / ESPI parsing, persistence, coverage tracking, revisions, and
public interfaces.

## Entry criteria

- Code builds successfully.
- Required test fixtures are available.
- Configuration required by the tests is isolated from production secrets.

## Required test areas

- XML parsing
- ESPI relationship resolution
- reading metadata
- unit normalization
- interval handling
- electricity and gas
- duplicate imports
- overlapping imports
- revisions and corrections
- gap detection
- coverage computation
- persistence
- provider adapters
- authentication/OAuth
- error handling
- security

## Exit criteria

A release candidate must pass all required automated tests and any identified
manual release tests.

Known failures must be documented before release.

## Physical-meter logical usage streams

The source of truth for selection behavior is the Rust implementation in
`crates/energy-store/src/usage_stream.rs` and its executable tests.

- `crates/energy-store/tests/usage_stream.rs` validates deterministic, in-memory selection.
- `crates/energy-store/tests/usage_stream_db.rs` validates physical-meter associations,
  accepted revisions, deduplication, overlap rejection, measurement
  compatibility, commodity isolation, and missing-meter handling.
- Conflicting or incomplete source observations must not yield a
  partially selected stream or fabricated zero-valued readings.
- Preserve accepted revision and resource provenance in selected results.
- Use isolated synthetic data and the ephemeral MariaDB test runner.
- Mixed-cadence prioritization, unit conversion, reliable gap reporting,
  and concurrent-import consistency need separate validation before
  customer-facing usage totals can be released.

For development verification, `ENGY_SKIP_FMT=1` can bypass formatting
while preserving compilation, Clippy, tests, and ADR conformance. Such
runs record formatting as `SKIPPED`, not `PASS`, and do not satisfy the
normal complete-verification requirement for releases.

## HTTP API contract

Each published HTTP endpoint must have an OpenAPI operation describing
its parameters, successful responses, errors, and authentication.
The contract is generated from Rust handlers and response types using
`utoipa`. Automated tests verify that all currently published routes
appear in the specification, that protected operations require bearer
authentication, and that the developer documentation is disabled by
default. The optional Swagger UI is restricted to local development.
