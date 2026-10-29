<!-- SPDX-License-Identifier: MIT -->
# Production release verification

**Purpose:** Describe required checks and publication procedures for
production container releases.

**Intended Audience:** Release maintainers and CI/CD operators.

This checklist is not an end-user installation guide. For installation
see [Installation](INSTALLATION.md).

## Mandatory checks before tagging

1. Clean Git tree, code review, and `scripts/check-publication.sh`.
2. `./deploy/docker-compose/test.sh` (isolated Rust test suite).
3. `bash scripts/check-prod-package.sh` to validate the intended
   production installer ZIP, image reference, executable modes, and scripts.
4. `docker build -f docker/Dockerfile.prod -t energy-ingest:release-check .`.
5. `./scripts/check-prod-image.sh energy-ingest:release-check`.
6. `./scripts/audit-prod-release.sh energy-ingest:release-check`.
7. `ENGY_TEST_IMAGE=energy-ingest:release-check ./scripts/test-prod-runtime.sh`.
8. `ENGY_TEST_IMAGE=energy-ingest:release-check \
   bash scripts/test-prod-e2e.sh` using approved sanitized XML.
   Confirm identical reimport, utility meter-number persistence/privacy,
   and rejection of conflicting resource ownership.
9. ARM64 Buildx build and administrative CLI/root-denial smoke tests.
10. Review critical CVE findings, build inputs, SBOM, and provenance.
11. Verify the source is on `main` and GitHub checks have passed.

## Publishing

The intended stable release version is stored in `release/VERSION`.
Stable production packages and public release tags must match this
version. Run `bash scripts/check-prod-package.sh` without arguments
to validate the intended stable package.

Private release candidates use an explicit `X.Y.Z-rc.N` version.
The private RC publisher validates the requested version, requires
a clean and synchronized `main` checkout, and checks that the
requested registry tag does not already exist.

Historical release reproduction requires the appropriate source
checkout and release metadata. Do not treat a current source
checkout as equivalent to a historical release.

Publish only by pushing an annotated `energy-ingest-vX.Y.Z` tag.
The Docker workflow pushes AMD64 and ARM64 version tags, not `latest`.
Afterward inspect the image digest and both platform manifests, then
validate on a native Raspberry Pi 5 before production deployment.

## Historical release 0.1.0 evidence

GitHub Actions run `38649034462` passed isolated verification, Trivy,
MariaDB-backed runtime checks, sanitized XML import, an authenticated
API query, ARM64 QEMU smoke tests, and multi-platform publication.
The published image was separately pulled and executed natively on
Raspberry Pi 5 `piha` (ARM64); the full database deployment on `piha`
was not exercised. See
[release validation](RELEASE_0_1_0_VALIDATION.md).

Inspect the published index with:

```bash
docker buildx imagetools inspect \
  docker.io/pmelendez2/gb-energy-ingest:0.1.0
```

Expected digest for the immutable `0.1.0` index:

```text
sha256:d0eb788c6342f6b90e1d15a4048889cf2fb44d8f9b832505ca29ea8d18bffb62
```

## Next-release CI/CD improvement

The current working-tree release workflow uses a native ARM64 runner
for its smoke job and configures a BuildKit cache. These changes are
post-`0.1.0` and do not alter the previously published image. Verify
that they are committed and active on the release branch before relying
on them. Benchmark a subsequent release before claiming reduced build
duration; cache reuse does not ensure that the published image is
bit-for-bit identical to the separately tested smoke image.

## Guided multi-meter selection

Run `bash scripts/test-guided-meter-selection.sh` to check customer
selection of a registered meter, skip behavior, refusal of invalid
selections, and refusal of unconfirmed associations. This test uses
mocked database reads and validates the actual Bash selection function;
it does not replace the disposable MariaDB production E2E test.
