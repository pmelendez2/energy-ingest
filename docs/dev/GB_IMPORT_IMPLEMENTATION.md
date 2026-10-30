<!-- SPDX-License-Identifier: MIT -->

# Green Button XML ingestion: architecture and operation

**Intended Audience:** Maintainers and contributors. This is not the homeowner's
installation guide. **Status:** development implementation; not a released
production container or Home Assistant integration.

## Purpose

A homeowner should be able to install the production `energy-ingest` services
using a supported container deployment, configure a provider, and upload Green
Button electricity or natural-gas XML without installing Rust, provisioning a
development database, or learning internal record identifiers. The application
will manage its database, provenance records, authentication, and migrations.
Home Assistant will eventually read canonical usage through a local
authenticated API. Automated retrieval may be added where an EDC or CCA
provides an authorized supported interface. **Those user-facing components
are planned and are not provided by this implementation chunk.**

This document explains the current importer and the engineering work needed
to deliver that experience. Developers use isolated MariaDB for validation;
production users must not need that environment.

## Components and data ownership

| Component | Responsibility |
| --- | --- |
| `gb` | Interpret Green Button/ESPI XML into normalized resources |
| `energy-store` | Validate references and persist canonical data in MariaDB |
| MariaDB | Retain resource identities, readings, revisions, and provenance |
| `energy-store-import` | Developer-oriented manual import command |
| Future production service | Handle setup, uploads, migrations and authentication |
| Future local API | Supply validated electricity and gas history to Home Assistant |

The important relationships are a Usage Point, its Meter Readings and Reading
Types, Interval Blocks, and interval observations. The store identifies each
canonical interval by meter, UTC start and duration. A utility correction
creates another revision and changes the accepted pointer; importing the same
observation again does not create a duplicate revision. Raw values are kept
with the source Reading Type metadata so units can be interpreted correctly.

## Current transactional import

`energy_store::import_gb_document_atomic` performs full supported-shape
preflight before starting a MariaDB transaction. For each supported Meter
Reading and associated Interval Block it:

1. Locks the acquisition-source row (`SELECT ... FOR UPDATE`) to serialize
   cooperating imports of the same source.
2. Resolves or creates Usage Points, Reading Types, Meter Readings, blocks,
   and canonical interval identities within the transaction.
3. Locks each canonical interval's accepted-revision pointer before comparing
   its value, quality and source metadata with the new observation.
4. Keeps matching observations unchanged, or inserts and accepts a new
   revision while retaining previous history.
5. Commits the file only after all its supported readings have succeeded.

A database error aborts the transaction and rolls back its uncommitted
changes. This means *per-file resource and interval writes* are atomic. Each
file is a separate transaction: if file three fails, files one and two that
already committed are not rolled back. Ingestion-run creation currently
occurs outside this transaction. A failed file can therefore retain its run
record, which operations and auditing must interpret correctly. The source
lock works for importers using this entry point; legacy pool-based APIs do
not participate in that serialization contract.

Only `import_gb_document_atomic` is exported as a document importer.
The pre-release non-atomic entry point has been removed. Lower-level store
functions remain available for controlled administrative and test use.

### Supported inputs and explicit limits

The document must contain unambiguously linked Meter Readings, Usage Points,
Reading Types, and Interval Blocks. Blocks must contain nonempty readings with
positive durations and representable UTC timestamps. Invalid links fail
preflight without database writes. The API preserves zero-valued observations
and tracks source metadata. It does not yet validate every possible ESPI
extension, normalize across separate linked XML exports, or implement an
upload queue and recovery/status interface. Additional database-backed
concurrency, correction, and rollback evidence is required before release.

## Development and integration testing — not production prerequisites

These prerequisites apply **only to contributors** exercising the current
Rust CLI and database integration tests:

1. Set up an isolated MariaDB database following
   [`DEVELOPMENT_ENVIRONMENT.md`](DEVELOPMENT_ENVIRONMENT.md).
2. Apply schema migrations following
   [`DATABASE_BOOTSTRAP.md`](../DATABASE_BOOTSTRAP.md).
3. Load the private test connection URL into `ENGY_DB_URL`; keep credentials
   outside Git and do not paste them into issue reports.
4. Install the documented development Rust toolchain.
5. Use authorized local XML samples. Never commit customer exports. Consult
   [`PRIVACY_SANITIZATION.md`](../PRIVACY_SANITIZATION.md) before considering
   fixtures for publication.
6. Register a custodian and acquisition source using `energy-store-admin`,
   if your development database does not already contain them.

### What is an acquisition-source ID?

`--source-id` refers to the positive **numeric** primary key assigned by the
application database to a source of imported data. Example `42` is a sample
only, not a provider code, utility account number, UUID or URL. Valid CLI
syntax is a positive base-10 unsigned 64-bit integer. The record must exist
in the selected database, and the ingestion run must belong to it. A
production-facing application should create and resolve this relationship
without exposing `SOURCE_ID` to homeowners.

Development-only examples, after the database exists and migrations run:

```bash
cargo run -p energy-store --bin energy-store-admin -- \
  custodian create "Example Utility"

# Substitute the actual custodian ID printed by the previous command.
cargo run -p energy-store --bin energy-store-admin -- \
  source create 1 "Manual Green Button XML"

cargo run -p energy-store --bin energy-store-admin -- \
  source list 1
```

Then, with `ENGY_DB_URL` loaded privately and the actual source ID:

```bash
cargo run -p energy-store --bin energy-store-import -- \
  --source-id 42 /private/path/electricity.xml
```

This command is for supervised development use, not the intended homeowner
installation method. It parses all requested files before writing, reports
observations, unchanged readings and accepted revisions, and imports each
file in its own transaction. The per-file counts describe observations, not
all database rows created. `accepted` includes both initial and corrected
readings. Avoid using customer-specific filenames in publicly shared logs.

### Validation and error recovery

Run formatting, workspace checks, tests, Clippy and ADR conformance using the
existing project verification workflow. The important tests for this
milestone are first import, repeat import, correction, invalid-resource
preflight, database-failure rollback, and concurrent imports of the same
source. An isolated MariaDB is required for database integration tests; unit
tests alone cannot establish transaction behavior.

If a file fails, check its error against the import report and ingestion-run
history. Correct the source or the environment and retry. Do not modify
canonical rows or delete revision history manually to force an import to
pass. Never include private XML, credentials or customer identifiers in
public bug reports.

## Delivery boundaries and post-release tracking

Before a production release, engineering must additionally deliver a
containerized runtime with persistent storage, automatic migrations, source
registration managed by the application, supported XML upload, credentials,
health/status reporting, authenticated local API and Home Assistant
integration. A complete test matrix must cover rollback, contention,
provider exports, overlap/corrections, and recovery. These are **project
engineering requirements**, not extra installation steps for homeowners.

`docs/dev/PROJECT_TASKS.md` is temporary pre-first-release planning material. It
must not ship as a permanent public-release guide. After the first release,
work items and defects should be tracked through GitHub Issues (and release
milestones as appropriate), not a maintained backlog in this document.

For implementation changes, review the specific owning module and accepted
ADR that governs the change: `gb` for parsing, `energy-store` migrations and
`gb_import.rs` for persistence, ADR-0004/0005 for UTC and revisions,
ADR-0016/0022 for storage, and ADR-0018/0024 for secrets and client access.
Maintainers do not need to read every ADR to conduct a routine import test.

## Performance diagnosis for maintainers

The golden-fixture test currently exercises two low-level, per-reading
persistence passes as well as the supported atomic importer. Consequently,
its total runtime alone does not measure production import throughput.
The `ENGY_T_PROFILE` opt-in prints elapsed time for parsing, the two legacy
test passes, subsequent atomic reimports, and cleanup, per fixture.

```bash
ENGY_T_PROFILE=1 cargo test -p energy-store --test gb_ingest \
  -- --nocapture
```

These are diagnostic durations, **not hardware-dependent pass/fail limits**.
Compare the atomic phase against the low-level test passes. Check existing
unique constraints and indexes in the migration SQL before proposing new
indexes. Use `EXPLAIN` for specific measured slow queries; do not add a
schema migration based only on elapsed test time. Database round trips,
foreign-key cleanup, and test structure can dominate small fixtures.

The remaining low-level test path intentionally verifies repository APIs;
it is not a supported alternative document importer. Production installation
does not require this developer profiling or an isolated test environment.

## Golden-fixture test structure and performance diagnostics

The `gb_ingest` integration test exercises the **supported transactional
importer** for the initial observation, an unchanged reimport, and a
single-reading correction. Low-level repository behavior is covered in its
own store tests. Source-scoped fixture cleanup uses set-oriented SQL within
one transaction, rather than repeating individual deletes per reading.

The test verifies the fixture manifest's reading count and duration, required
zero-valued observations, the importer report for each operation, the count of
canonical interval identities, the count of historical revisions, and the
accepted correction. It uses only isolated database connections and removes
records associated with the disposable acquisition source it creates.

For optional phase timing during development:

```bash
ENGY_T_PROFILE=1 cargo test -p energy-store \
  --test gb_ingest -- --nocapture
```

The report separates XML parsing, source setup, initial atomic import,
unchanged reimport, correction, database assertions, and cleanup. Compare
phase timings **on the same machine and database configuration**; do not use
fixed wall-clock thresholds to determine correctness. If a phase remains
slow, inspect the corresponding SQL and use MariaDB `EXPLAIN` and existing
schema indexes before proposing a new migration. Do not make production
indexing changes based solely on integration-test wall-clock durations.

## Authenticated HTTP import contract

The authenticated `POST /v1/imports/green-button?source_id=ID`
endpoint accepts one complete Green Button XML document as
`application/xml` with the existing API bearer token. The
request limit is 8 MiB. `source_id` identifies an existing
acquisition source, not a meter. Successful imports report
`observations`, `unchanged`, and `accepted` counts. The canonical
revision-aware importer rejects invalid XML and source IDs; it
does not fabricate missing observations. This is a write endpoint:
review token permissions and deployment network boundaries before
production use. Home Assistant handles browser file selection and
Recorder synchronization separately; do not store utility login
credentials in Energy Ingest. Test invalid sources, repeat imports,
accepted corrections, authorization, and bounded payloads.
