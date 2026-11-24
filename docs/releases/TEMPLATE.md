# Energy Ingest X.Y.Z

**Purpose:** Provide a consistent template for future Energy Ingest release notes.

**Intended Audience:** Release maintainers and contributors.

## Highlights

Describe the most important user-visible changes.

## Fixes

Describe corrected behavior and security fixes.

## Issues resolved

List each fixed issue using its direct GitHub link, e.g.:

- [#123 — Short description](https://github.com/pmelendez2/energy-ingest/issues/123):
  Explain the user-visible fix.

For releases without resolved issues, use `None` on its own line.
Do not claim that a merged issue or PR shipped before its release.

## Related pull requests

Link implementation and release-preparation PRs, or state `None`.

## Compatibility and upgrade notes

Describe configuration, database, API, and behavior changes. State "None"
when there are no special upgrade steps.

## Verification

Summarize the tests and release checks actually completed. Do not claim
results that have not been verified.

## Known limitations

Document known limitations and remaining work. State "None reported"
if none have been identified.

## Installation

Download the release ZIP and SHA-256 checksum from the official GitHub
release. Follow `docs/INSTALL_LINUX.md`. For production upgrades, preserve
the existing database, credentials, and imported readings.
