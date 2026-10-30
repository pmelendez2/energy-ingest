<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-08. -->

# `gb_profile` Utility Releases

**Audience:** Release maintainers and developers.

**Purpose:** Explain versioning and distribution of the standalone
`gb_profile` utility; these are not provider-profile revisions.

**Status:** Pre-release; official release workflow validation pending.

`gb_profile` is an independent Cargo package (`crates/gb_profile`).
The `gb` crate (`crates/gb`) is the shared parsing/profile library.
Its library version does not determine the CLI release version.

## Version policy

Use SemVer: `0.2.0-dev.1`, `0.2.0-rc.1`, then `0.2.0`.
The version in `crates/gb_profile/Cargo.toml` is authoritative.
`--version`, `version --verbose`, and `submission.toml` refer to
that same embedded version. Source revision and build channel are
captured at compile time. A local `--release` build remains a
**development** build, not a trusted official release.

## Local build and validation

From the `energy_ingest` repository root:

```bash
cargo test --workspace
cargo run -p gb_profile -- --version
cargo run -p gb_profile -- version --verbose
./scripts/release_gb_profile.sh
```

The script produces `dist/gb_profile-vVERSION-linux-ARCH.tar.gz`
and a corresponding `.sha256` file. It uses the host's native
architecture (`x86_64` or `aarch64`). A local archive is not
an approved production release.

## Official release

1. Set and commit the stable SemVer in `crates/gb_profile/Cargo.toml`.
2. Run format, clippy, full tests, and ADR checks.
3. Tag the reviewed commit: `gb_profile-vVERSION`.
4. Push the tag. GitHub Actions builds x86-64 and ARM64 archives.
5. Confirm workflow success, both checksums, and the GitHub Release.

The release workflow uses `GB_PROFILE_OFFICIAL_RELEASE=1` and
rejects nonstable versions, dirty tracked files, and a mismatched
tag. It uploads two architecture-specific archives and publishes
them as GitHub Release assets. The tag and artifacts are not a
substitute for human release approval or downstream verification.

**Repository layout:** The workflow operates from the standalone
`energy_ingest` repository root. The executable package is in
`crates/gb_profile`; do not introduce an extra `energy_ingest/` path.

## Installed usage

Extract the archive and run `gb_profile` directly; the executable
does not require Cargo, MariaDB, or a repository checkout. It does
require a Linux architecture compatible with the chosen binary.
The operator provides a writable `--workspace` directory for local
profiles. No private XML is uploaded by the release workflow.

Checksums verify transfer integrity only; they do not establish
publisher identity. Verify the source and release publisher before
using a downloaded binary.
