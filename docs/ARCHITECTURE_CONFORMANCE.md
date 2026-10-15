<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# Architecture Conformance

**Purpose:** Index implementation evidence for accepted ADRs.

**Intended Audience:** Maintainers and architecture reviewers.

This document provides a lightweight human-audit index for verifying that the implementation remains consistent with the project's accepted Architecture Decision Records (ADRs).

The ADRs define architectural decisions and constraints. This document does not duplicate those decisions and does not itself establish architectural requirements.

The source code, database migrations, automated tests, and operational procedures provide the implementation evidence used to verify conformance.

## Conformance Review

For each accepted ADR, reviewers should be able to identify where its architectural consequences are implemented or verified.

Evidence may include:

- source-code or crate boundaries;
- database migrations;
- automated unit or integration tests;
- configuration;
- operational runbooks;
- generated API specifications;
- or explicit human review where meaningful automated verification is not practical.

Not every ADR requires every evidence type.

Automated conformance checking verifies that this audit trail remains internally consistent. It does **not** prove that the implementation satisfies the intent of an ADR. Human review remains required.

## Review States

The following terms describe the state of implementation evidence:

- **Pending** — implementation has not yet reached the point where meaningful conformance evidence exists.
- **Automated** — one or more automated checks or tests provide evidence for important architectural constraints.
- **Review** — conformance is primarily verified through human inspection of source, design, or operational behavior.
- **Automated + Review** — automated evidence exists, but human review is also required.

These states describe verification evidence, not ADR lifecycle status.

## ADR Conformance Index

The index below is intentionally maintained at the ADR level rather than assigning individual requirement identifiers.

| ADR | Evidence | Verification |
| --- | --- | --- |
| ADR-0000 | Pending | Pending |
| ADR-0001 | Pending | Pending |
| ADR-0002 | Pending | Pending |
| ADR-0003 | Pending | Pending |
| ADR-0004 | Pending | Pending |
| ADR-0005 | Pending | Pending |
| ADR-0006 | Pending | Pending |
| ADR-0007 | Pending | Pending |
| ADR-0008 | Pending | Pending |
| ADR-0009 | Pending | Pending |
| ADR-0010 | Pending | Pending |
| ADR-0011 | Pending | Pending |
| ADR-0012 | `crates/energy-store/src/bin/energy-store-admin.rs`; `crates/energy-store/src/bootstrap.rs`; `crates/energy-store/tests/bootstrap_cli.rs`; `docs/DATABASE_BOOTSTRAP.md` | Automated + Review |
| ADR-0013 | Pending | Pending |
| ADR-0014 | Pending | Pending |
| ADR-0015 | Pending | Pending |
| ADR-0016 | `crates/energy-store/src/bootstrap.rs`; `crates/energy-store/migrations/`; `crates/energy-store/tests/bootstrap.rs`; `crates/energy-store/tests/migration.rs`; `docs/DATABASE_BOOTSTRAP.md` | Automated + Review |
| ADR-0017 | `docs/adr/` | Review |
| ADR-0018 | `crates/energy-store/src/bin/energy-store-admin.rs`; `crates/energy-store/src/bootstrap.rs`; `crates/energy-store/tests/bootstrap_cli.rs`; `docs/DATABASE_BOOTSTRAP.md` | Automated + Review |
| ADR-0019 | Project source, documentation, and development workflow | Review |
| ADR-0020 | Pending | Pending |
| ADR-0021 | `crates/energy-store/src/oauth_authorization.rs`; `crates/energy-store/migrations/0005_create_oauth_authorization.sql`; `crates/energy-store/tests/oauth_authorization.rs`; `crates/energy-store/tests/migration.rs` | Automated + Review |
| ADR-0022 | `crates/energy-store/migrations/0001_create_ingestion_foundation.sql` through `0005_create_oauth_authorization.sql`; `crates/energy-store/src/interval_reading.rs`; `crates/energy-store/src/retrieval_coverage.rs`; `crates/energy-store/src/oauth_authorization.rs`; `crates/energy-store/tests/migration.rs`; `crates/energy-store/tests/interval_reading.rs`; `crates/energy-store/tests/retrieval_coverage.rs`; `crates/energy-store/tests/oauth_authorization.rs` | Automated + Review |
| ADR-0023 | Pending | Pending |
| ADR-0024 | `crates/energy-store/src/api_client.rs`; `crates/energy-store/tests/api_client_db.rs`; `crates/energy-api/tests/http.rs` | Automated + Review |
| ADR-0025 | Pending | Pending |
| ADR-0026 | Pending | Pending |
| ADR-0027 | Pending | Pending |
| ADR-0028 | Pending | Pending |
| ADR-0029 | Pending | Pending |

## Review Process

During implementation, update the relevant ADR entry when meaningful evidence is introduced.

Examples include:

- adding a migration that implements an ADR-defined persistence constraint;
- adding an integration test for idempotency or revision behavior;
- establishing a crate boundary that enforces separation of responsibilities;
- adding an OAuth lifecycle test;
- adding a backup-and-restore runbook;
- or introducing an API authentication implementation and its tests.

Run:

```bash
./scripts/check_adr_conformance.py
```

after each coherent design, implementation, migration, testing, or operational-documentation change and before committing the change.

A successful automated check means that the conformance audit trail is structurally consistent. It does not replace review of the ADR's architectural intent.
