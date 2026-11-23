<!-- SPDX-License-Identifier: MIT -->

# Energy Ingest — Unreleased Changes

## Purpose

Document changes planned for the next Energy Ingest release,
including bug fixes, dependency maintenance, CI improvements,
and compatibility requirements.

## Intended Audience

Energy Ingest users, maintainers, contributors, and release
reviewers.


These changes are intended for the next release after v0.1.9.
The release version and publication date have not yet been assigned.

## Added

- Added synthetic Green Button XML regression coverage for
  repeated UsagePoint definitions in NEM datasets.
- Added database regression coverage verifying that consumption
  and export channels remain distinct and reimport is idempotent.
- Added version-aware documentation and release-note validation.
- Added a release-note template and release-note publishing checks.
- Added regression tests for change-aware CI classification.
- Added shared UTC, calendar-date, and local-time helpers using
  maintained Rust date/time dependencies.

## Changed

- Replaced Chrono and Chrono-TZ with `time` and Jiff across
  Green Button parsing, MariaDB persistence, API handling,
  and physical-meter stream validation.
- Updated SQLx from 0.9.0 to 0.9.1.
- Updated fixture contribution documentation with privacy
  review and maintainer approval requirements.
- Updated current release references to v0.1.9.
- Updated CI to select appropriate verification for
  documentation, workflow, and source changes.
- Preserved the required `isolated` GitHub CI check.
- Changed release publication to use version-specific notes.

## Fixed

- Accept equivalent repeated Green Button UsagePoint definitions
  while rejecting conflicting resource identities or metadata.
- Preserve separate NEM consumption and export observations.
- Correct outdated documentation release-version references.
- Correct the vulnerable URL dependency version.
- Correct the vulnerable SQLite library dependency version.

## Security

- Updated `url` from 2.5.8 to 2.5.9.
- Updated `libsqlite3-sys` from 0.37.0 to 0.39.1.
- Updated SQLx to 0.9.1 for compatible dependency resolution.
- Removed `chrono` and `chrono-tz`, resolving their
  unmaintained-dependency advisories.
- Preserved strict dependency auditing without advisory
  exceptions or disabled security checks.
- Preserved secret scanning and signed-commit requirements.

## Compatibility

- Existing MariaDB schema and stored UTC datetime semantics
  are unchanged.
- API UTC timestamps retain their existing wire format.
- Timestamp fingerprints continue to use Unix seconds.
- Daily natural-gas reporting retains local-date and DST
  reconciliation behavior.
- Missing observations are never fabricated.
- Solar-export data is retained distinctly; this release does
  not add Home Assistant solar-production reporting.

## Verification

- Formatting: PASS.
- Compilation: PASS.
- Clippy with warnings denied: PASS.
- Isolated MariaDB verification: 223 tests passed.
- Documentation checks: PASS.
- ADR conformance: PASS.
- Production installer packaging: PASS.
- Strict RustSec dependency audit: PASS.

## Related Issues

- #41 — NEM duplicate UsagePoint import handling.
- #42 — Documentation and release consistency.
- #43 — CI improvements.
- #45 — Migration to maintained datetime dependencies.

## Release Status

Not yet published. Assign the next release version only
after the protected-main pull request, required CI checks,
and publication verification have completed.
