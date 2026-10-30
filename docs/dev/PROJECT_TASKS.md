<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

<!-- SPDX-License-Identifier: MIT -->

# Project Tasks

**Audience:** Maintainers and contributors.

**Purpose:** Track active and deferred work without duplicating ADRs or
operational instructions.

This document tracks implementation work that is known but intentionally
deferred or awaiting a later project milestone.

It is not a substitute for source code, ADRs, tests, or operational
documentation. Its purpose is to prevent agreed future work from being lost
while keeping the current implementation focused.

## Review Policy

Review this task list:

- after each major implementation chunk;
- before beginning a new subsystem;
- before a release candidate;
- before the first production deployment; and
- whenever a deferred dependency or prerequisite becomes available.

During each review:

1. remove completed tasks;
2. move tasks into active work when their prerequisites are satisfied;
3. revise tasks whose design has changed;
4. remove tasks that are no longer required; and
5. add newly identified deferred work.

## Completed release checkpoint (2026-10-14)

Production Docker image `0.1.0` was published for AMD64 and ARM64.
Run `38649034462` passed the security, runtime, XML-to-API, and ARM64
QEMU release gates. The released image was also smoke-tested natively
on Raspberry Pi 5 `piha`. See
[release validation](../testing/RELEASE_0_1_0_VALIDATION.md).

CI/CD follow-up: evaluate native ARM64 builds, BuildKit cache reuse,
and eventually publish-by-digest so tested images are not rebuilt.
Measure against the baseline 76m 30s, and keep protected-branch and
Docker Hub publication requirements intact. Production MariaDB-backed
validation on Pi 5 and Windows/WSL2 validation remain outstanding.

## Current Work

### Expand and validate Green Button ingestion coverage

Status: Partially implemented; further coverage and production validation
remain open. The `0.1.0` release workflow passed a sanitized XML import
followed by an authenticated API query. That does not establish complete
coverage of every ingestion case or deployment environment.

Connect the shared Green Button parsing path to the canonical
`energy-store` persistence API.

The existing release E2E fixture is the approved sanitized SDG&E
`sdge-electric-one-day-hourly.xml` fixture. The following items are
coverage requirements to reconcile against current tests, not a claim
that all are missing.

The implementation must:

- parse the Green Button XML without relying on provider filenames;
- preserve Green Button source identifiers needed for reconciliation;
- persist the applicable Usage Point, Reading Type, Meter Reading, and
  Interval Block;
- persist canonical interval readings using UTC interval identity;
- record observed interval-reading revisions with ingestion provenance;
- preserve zero-valued readings as real readings;
- verify persisted reading count, timestamps, durations, and values;
- re-import the same fixture without duplicating canonical intervals; and
- run through the normal ephemeral database verification workflow.

The Green Button inspection summary is not the persistence model. The
`gb` parser exposes normalized records through `GreenButtonDocument`
so ingestion can pass source data to `energy-store` without coupling persistent
storage to XML parsing.

After the one-day electricity fixture passes, validate representative larger
electricity and natural-gas fixtures through the same ingestion path.

## Deferred Tasks

### CI, GitHub Actions, and Test Runner hardening

**Status:** Deferred until the ADR-0028 fixture-privacy implementation is
complete and independently reviewed. This is a separate implementation chunk.

**Goal:** Establish one coherent verification policy for local development,
GitHub Actions, and releases, with maintainable test evidence.

Acceptance criteria:

- Reuse one authoritative verification contract across environments.
- Run Rust formatting, Clippy, unit/integration/doc tests, and ADR checks.
- Include database-backed tests in the supported container test environment.
- Add dependency/license and documentation checks with actionable failures.
- Verify x86-64 and ARM64 release artifacts, versions, checksums, and source
  provenance; distinguish development, prerelease, and stable channels.
- Integrate ADR-0028 privacy verification when that implementation is ready.
- Prevent publication on failed required checks and keep sensitive data out
  of logs, test fixtures, and published artifacts.

### Host-managed backup and restore

Status: Production-readiness work under ADR-0023.

Use an existing protected OS/host backup workflow where feasible, rather than
introducing a separate application-specific backup service. Implement and
validate a consistent recoverable database state and an actual restore.

Validate:

- consistent production database backup;
- restoration into an empty database;
- schema and version validation after restore;
- application-data validation after restore;
- retained source-data recovery where applicable; and
- clear recovery failure behavior.

Keep the Home Lab workflow simple and inspectable. Do not introduce a backup
control plane or additional service unless a concrete requirement justifies it.

### Complete formal testing documentation

Status: Open documentation-maintenance work. Release `0.1.0` has
been published; reconcile these documents with implemented test gates.

Review and complete the existing testing documentation, including:

- `docs/testing/TEST_PLAN.md`;
- `docs/testing/TEST_STRATEGY.md`;
- test categories and integration-test infrastructure;
- synthetic and sanitized Green Button test data;
- verification commands;
- release verification expectations; and
- interpretation of test results.

Do not create another testing document solely to duplicate material already
covered by the existing testing documentation. Source code and automated tests
remain the behavioral source of truth.

### OAuth operational documentation

Status: Deferred until OAuth persistence and provider integration are
implemented.

Document operational procedures for:

- authorization setup;
- token refresh;
- authorization revocation;
- credential replacement;
- failed authorization recovery; and
- provider-specific troubleshooting.

### Local API operational details

Status: Partially implemented. Client authentication and
administration code exists in `crates/energy-api` and
`crates/energy-store`; review outstanding rotation, revocation,
and operational documentation separately.

Define implementation-level details for ADR-0024, including:

- client-token creation;
- verifier persistence;
- token rotation;
- token revocation;
- administrative commands; and
- operational testing.

Do not introduce RBAC unless a new requirement justifies an architectural
change.

### Dedicated development, test, and integration host

Status: Deferred until additional Home Lab hardware is available.

Reevaluate moving development support services, ephemeral test infrastructure,
and integration infrastructure from the current development workstation to a
dedicated host.

Do not change the current test environment solely in anticipation of future
hardware.

### Integration environment

Status: Deferred until enough application components exist to justify a
persistent full-system environment.

Establish an integration environment using synthetic/test data for
cross-component testing.

Keep the environment distinction:

```text
Development -> Test -> Integration -> Release gate -> Production
```

Test remains ephemeral. Integration may be semi-persistent.

### Public repository privacy and security audit

Status: Recurring audit requirement. Public release `0.1.0` exists;
retain this audit as a continuing release gate instead of describing
it as exclusively future pre-publication work.

Review all tracked project content for unnecessary local or private
infrastructure information.

Check for:

- usernames and personal account names;
- private hostnames and internal DNS domains;
- absolute home-directory paths;
- SSH and AGE key filenames or locations;
- email addresses not intended for publication;
- private IP addresses and network topology;
- credential, token, password, and secret values;
- secret-file names that expose private infrastructure details;
- provider account identifiers;
- development-only configuration accidentally committed; and
- examples copied from the local Home Lab that should instead use generic
  placeholders.

Public project source and documentation should use generic configuration
interfaces and examples rather than encode a contributor's local environment.

Run this audit periodically during development and as a mandatory first-release
check.

## Completed Tasks

### Standalone production database deployment

Implemented and validated the pre-release production MariaDB deployment using
standard Docker Compose.

The deployment provides:

- Compose project `energy-ingest`;
- database `energy_ingest`;
- persistent MariaDB storage;
- production runtime and administrative database credentials;
- file-backed Compose secrets;
- an internal-only database network;
- MariaDB health checking;
- production lifecycle setup and status scripts; and
- production artifacts independent of development and test artifacts.

The pre-release lifecycle was validated for:

- first-time creation;
- repeat-safe continue/start;
- container refresh;
- database recreation;
- credential regeneration;
- settings reset;
- temporary teardown with state preservation;
- restoration from preserved state;
- full removal;
- NEW-state detection; and
- cancellation without changes.

MariaDB port `3306` is not published to the host by the reference deployment.

Operational behavior is documented in:

```text
docs/PRODUCTION_DATABASE.md
```

### Production database schema bootstrap

Implemented and validated a standalone production bootstrap path.

The production administrative image is built from:

```text
docker/Dockerfile.prod
```

and does not depend on development or test artifacts.

The bootstrap command is exposed through:

```text
deploy/docker-compose/bootstrap-prod.sh
```

The workflow:

- uses the application's embedded SQLx migration path;
- obtains administrative database credentials from the deployment environment;
- connects through the internal production Docker network;
- validates and applies SQLx migrations;
- verifies the expected application schema;
- supports repeated execution safely; and
- does not print database credentials.

Production validation completed repeated bootstrap runs successfully with:

```text
Database bootstrap complete: 13 application tables verified.
```

Operational behavior is documented in:

```text
docs/DATABASE_BOOTSTRAP.md
```

### Runtime database privilege boundary

Validated that the production runtime database account has only:

```text
SELECT
INSERT
UPDATE
DELETE
```

on the application database.

A direct `CREATE TABLE` test using the runtime identity was rejected by
MariaDB.

### Containerized development and test environments

Implemented standalone Docker Compose development and test environments.

The development environment uses persistent database and Cargo-cache state.

The test environment uses an ephemeral MariaDB database and generated
credentials.

The workflows do not require Docker Swarm or project-specific deployment
managers.

### Development environment portability

Validated the containerized development and test workflows on:

- x86_64 Linux; and
- ARM64 / Raspberry Pi 5.

A clean ARM64 clone was tested from a path that did not reproduce the
maintainer's normal local directory hierarchy.

### Database installation/bootstrap workflow

Implemented the administrative database bootstrap workflow using the
application's embedded SQLx migration path.

The workflow:

- obtains administrative database credentials through the deployment
  environment rather than source-controlled configuration;
- connects to MariaDB;
- validates and applies SQLx migrations;
- detects incompatible migration history through SQLx;
- verifies the expected application schema;
- supports repeated execution safely; and
- reports bootstrap failures without printing database credentials.

The administrative command is:

```text
energy-store-admin database bootstrap
```

### Canonical database schema

Implemented and validated the initial `energy-store` relational schema through
version-controlled SQLx migrations.

The schema covers:

- ingestion source tracking;
- Green Button canonical entities;
- interval readings and revision history;
- retrieval coverage and gaps; and
- OAuth authorization state.

Migration SQL remains the source of truth for the database schema.

### Developer verification tooling

Implemented `scripts/verify.sh` as the repository-wide developer verification
entry point.

The verification workflow covers:

- formatting;
- compilation;
- Clippy;
- unit tests;
- integration tests;
- documentation tests; and
- ADR conformance.

Local verification status and history files remain uncommitted developer
state.

### Initial database integration test coverage

Expanded database integration testing to validate the canonical schema and its
primary relational constraints.

### ADR conformance checker

Implemented automated ADR evidence-integrity checking while retaining human
review for architectural intent.
