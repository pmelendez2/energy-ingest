<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

<!-- SPDX-License-Identifier: MIT -->

# Tested Environments

## Purpose

Record platforms and workflows with documented verification evidence.

**Intended Audience:** Maintainers, contributors, and release reviewers.

This file records environments in which the current `energy_ingest`
development and verification workflows have been exercised successfully.

It is a validation record, not a guarantee that unlisted environments are
unsupported.

## 2026-10-06

### x86_64 Linux

Validated on a 64-bit x86 Linux development workstation.

Validated workflows:

```text
containerized development setup
development environment lifecycle
persistent MariaDB development database
ephemeral MariaDB test database
Rust workspace verification
ADR conformance
clean test cleanup
```

Development lifecycle coverage included:

- first-time creation;
- reuse;
- refresh;
- database recreation;
- credential regeneration;
- credential preservation;
- settings reset and preservation;
- temporary teardown and restoration;
- full teardown;
- NEW-state detection; and
- cancellation without changes.

The full `test.sh` workflow completed successfully.


### Pre-Release Production Database Validation

Validated the production-shaped Docker Compose deployment on the x86_64 Linux
development workstation.

Production lifecycle coverage included:

- first-time creation;
- production settings and secret generation;
- persistent production MariaDB volume creation;
- MariaDB health validation;
- confirmation that port `3306` is not published to the host;
- repeat-safe continue/start;
- MariaDB container refresh;
- database recreation with new credentials;
- `.env.prod` reset from the tracked template;
- temporary teardown while preserving database state;
- restoration from preserved state;
- full database/settings/credential removal;
- NEW-state detection; and
- cancellation without changes.

The production runtime account was verified to have only:

```text
SELECT
INSERT
UPDATE
DELETE
```

on the `energy_ingest` database.

A direct `CREATE TABLE` attempt using the runtime account was rejected by
MariaDB.

The standalone production administrative image was built successfully from:

```text
docker/Dockerfile.prod
```

without depending on development or test artifacts.

A production-specific Docker ignore file reduced the clean image build context
from approximately 2.40 GB to approximately 9.02 kB.

The production schema bootstrap completed repeatedly with:

```text
Database bootstrap complete: 13 application tables verified.
```

The final simplified bootstrap path was executed from the
`deploy/docker-compose/` directory:

```bash
./bootstrap-prod.sh
```

From the repository root, the equivalent invocation is
`./deploy/docker-compose/bootstrap-prod.sh`.

and did not rebuild the administrative image during each bootstrap run.

Backup and restore validation remain outstanding before the production
database work is considered release-ready.

### ARM64 / Raspberry Pi 5

Validated on a Raspberry Pi 5 running 64-bit Linux (`aarch64`).

The repository was cloned into:

```text
~/energy-portability-test/energy_ingest
```

This location intentionally did not reproduce the normal workstation path or
parent-directory hierarchy.

The following commands completed successfully:

```bash
cd deploy/docker-compose
./setup-dev.sh
./status-dev.sh
./test.sh
```

Validated behavior included:

- clean Git clone;
- repository path independence;
- no dependency on a `homelab-tools` parent directory name;
- ARM64 Rust development-image build;
- ARM64 MariaDB image pull and startup;
- persistent development database volume;
- persistent Cargo cache volumes;
- generated development credentials;
- MariaDB health check;
- ephemeral MariaDB test environment;
- formatting checks;
- compilation;
- Clippy;
- unit tests;
- integration tests;
- documentation tests;
- ADR conformance; and
- automatic test-environment cleanup.

The full verification run completed with:

```text
ADRs found:       25
Accepted ADRs:    25
Errors:           0
Warnings:         0
```

and ended with:

```text
All verification checks passed.
Updated .test-status.md
Updated .test-history.csv
Verification completed successfully.
Cleaning up ephemeral test environment...
```

### Portability Issue Found During Validation

The initial clean ARM64 clone exposed that:

```text
deploy/docker-compose/.env.dev.example
```

was required by `setup-dev.sh` but had not been committed.

The file was added as a tracked release artifact and the clean-clone setup was
then rerun successfully.

This validation confirms that the development tooling must continue to avoid
dependencies on:

```text
~/src/hlt
homelab-tools as a required parent directory
a fixed repository clone path
a fixed CPU architecture
```

## 2026-10-14 — Production Docker release 0.1.0

- Source: tag `energy-ingest-v0.1.0`, commit `41798b797380`.
- Image: `docker.io/pmelendez2/gb-energy-ingest:0.1.0`.
- Platforms published: `linux/amd64` and `linux/arm64`.
- Multi-platform digest:
  `sha256:d0eb788c6342f6b90e1d15a4048889cf2fb44d8f9b832505ca29ea8d18bffb62`.
- GitHub Actions run `38649034462`: verify, ARM64 QEMU smoke test,
  and multi-platform publish succeeded.
- Verification included an isolated database test suite, production
  runtime and secret checks, critical-CVE Trivy gate, sanitized XML
  import, and authenticated API query.
- Raspberry Pi 5 `piha`: successful ARM64 image pull, architecture
  inspection (`linux/arm64`), and administrative `--help` execution.
- Limitations: no full MariaDB-backed production deployment was run on
  `piha`; no Windows/WSL2 or macOS validation is claimed here.
- Release workflow baseline: 76m 30s; ARM64 QEMU build 33m 17s and
  multi-platform build/publish 37m 53s.

See [complete 0.1.0 evidence](RELEASE_0_1_0_VALIDATION.md).

## Release Validation Policy

Before a release, update this file with the environments actually exercised for
that release.

At minimum, record:

- operating system and architecture;
- container runtime;
- development setup result;
- verification result;
- database environment result; and
- any release-specific limitations or deviations.

Do not infer support from this file for environments that were not actually
tested.

## 2026-10-30 — v0.1.8 release preparation

The maintainer ran the isolated Docker verification workflow on the
`sparta` development host: 209 tests passed, including compilation,
formatting, Clippy, unit and integration tests, documentation checks,
ADR conformance, and production package validation. The RustSec
`cargo-audit` scan reported no vulnerabilities. PR #31 CI and secret
scanning passed before merge commit `9522d05`.

This evidence applies to the source changes merged in PR #31. The
v0.1.8 registry image, native Raspberry Pi deployment, and Home
Assistant compatibility have not yet been verified.
