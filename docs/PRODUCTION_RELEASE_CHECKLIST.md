<!-- SPDX-License-Identifier: MIT -->
# Production release verification

## Mandatory checks before tagging

1. Clean Git tree, code review, and `scripts/check-publication.sh`.
2. `bash deploy/docker-compose/test.sh` (isolated Rust test suite).
3. `docker build -f docker/Dockerfile.prod -t energy-ingest:release-check .`.
4. `bash scripts/check-prod-image.sh energy-ingest:release-check`.
5. `bash scripts/audit-prod-release.sh energy-ingest:release-check`.
6. `ENGY_TEST_IMAGE=energy-ingest:release-check bash scripts/test-prod-runtime.sh`.
7. ARM64 Buildx build and administrative CLI/root-denial smoke tests.
8. Review critical CVE findings, build inputs, SBOM, and provenance.
9. Verify the source is on `main` and GitHub checks have passed.

## Publishing

Publish only by pushing an annotated `energy-ingest-vX.Y.Z` tag.
The Docker workflow pushes AMD64 and ARM64 version tags, not `latest`.
Afterward inspect the image digest and both platform manifests, then
validate on a native Raspberry Pi 5 before production deployment.

## Outstanding end-to-end gate

The runtime suite currently covers MariaDB initialization, generated
credentials, the importer help path, API health, and unauthorized denial.
It does **not** yet perform a real sanitized XML import followed by an
authenticated API query. That check must be completed before v0.1.0.
