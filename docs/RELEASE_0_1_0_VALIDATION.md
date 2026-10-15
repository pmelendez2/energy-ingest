# Release 0.1.0 — published validation

**Purpose:** Preserve historical evidence for the published 0.1.0 image.

**Intended Audience:** Release maintainers and auditors.

<!-- SPDX-License-Identifier: MIT -->

## Published artifact

- Git source tag: `energy-ingest-v0.1.0`.
- Release commit: `41798b7973800adce6ba3c7cd26a282466273ed1`.
- Docker Hub: `docker.io/pmelendez2/gb-energy-ingest:0.1.0`.
- Multi-platform index digest:
  `sha256:d0eb788c6342f6b90e1d15a4048889cf2fb44d8f9b832505ca29ea8d18bffb62`.
- Platforms: `linux/amd64`, `linux/arm64`.
- Additional `unknown/unknown` entries in the index are attestation
  manifests, not runnable application platforms.
- GitHub Actions publication run: `38649034462` (success).

## Verification evidence

The successful release workflow passed:

- Isolated Rust and database tests.
- AMD64 production image inspection and privacy/security audit.
- Trivy scan for fixable critical vulnerabilities.
- Production runtime and protected-secret checks.
- Sanitized Green Button XML import and authenticated API query.
- ARM64 smoke build, inspection, and entrypoint execution under QEMU.
- AMD64/ARM64 multi-platform publication.

Native ARM64 validation on Raspberry Pi 5 host `piha` also passed:

- Docker Hub pull using `--platform linux/arm64`.
- Docker image inspection reported `linux/arm64`.
- The published image ran `energy-store-admin --help` successfully.

A full MariaDB-backed deployment on `piha` was **not** tested as part
of that native smoke check.

## CI/CD baseline (release 0.1.0)

All timestamps are from GitHub Actions run `38649034462`.

| Step | Duration |
|---|---:|
| Isolated verification | 2m 12s |
| AMD64 production image build | 1m 31s |
| Trivy vulnerability scan | 12s |
| ARM64 QEMU build | 33m 17s |
| Multi-platform build and publish | 37m 53s |
| Entire release workflow | 76m 30s |

The following CI/CD optimization is **post-0.1.0** and does not alter
any published `0.1.0` image: use a native ARM64 runner and export its
BuildKit cache for reuse by the publication job. Cache reuse is not
identical to transferring a previously validated image digest; retain
all existing release gates and benchmark a subsequent release before
claiming any time savings.
