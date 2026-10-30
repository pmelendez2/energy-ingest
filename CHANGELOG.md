<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

## 0.1.8 — 2026-10-30 (release preparation)

### Added

- Authenticated acquisition-source API responses now include
  `utility_company`, resolved from the registered data custodian.
- Rust dependency auditing in CI and the pre-push workflow.
- Docker Hub publication verification for the expected image digest and
  both `linux/amd64` and `linux/arm64` platforms.
- A GitHub Actions publication summary emitted only after verification.

### Verification

- 209 tests passed in the isolated Docker verification environment.
- RustSec dependency audit completed without reported vulnerabilities.
- PR #31 passed CI and secret scanning and was merged into `main`.
- Registry publication and production deployment are pending; this entry
  does not assert that v0.1.8 has been released.
