<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0016: Use MariaDB for Persistent Relational Storage

- Status: Accepted
- Date: 2026-09-30

## Context

`energy_ingest` requires durable relational storage for canonical readings, revisions, provenance, ingestion state, coverage, gaps, and related application state.

The persistence layer must support transactions and relational integrity while remaining practical for a residential Home Lab deployment on amd64 and arm64 systems.

## Decision

MariaDB will be the initial persistent relational backend for `energy_ingest`.

MariaDB is an implementation choice for the initial architecture, not a permanent guarantee that no other backend can ever be supported.

A dedicated application persistence boundary, `energy-store`, will own database interaction.

Other components, including Data Custodian adapters, Home Assistant, and ordinary API clients, will not access MariaDB directly.

The persistence boundary owns:

- SQL and database-specific behavior;
- transaction boundaries;
- schema migrations;
- relational constraints;
- and interpretation of persisted application state.

The schema must preserve the architectural invariants defined elsewhere, including:

- revision history;
- provenance;
- durable ingestion state;
- coverage;
- gaps;
- and idempotent reprocessing.

Logical updates that must remain consistent will use database transactions.

Relational invariants that can be enforced clearly by MariaDB should use appropriate database constraints rather than relying only on application code.

The schema will be versioned and evolved through explicit migrations.

The normal runtime database identity will use the minimum practical privileges needed for application operation and will not use the MariaDB root account.

Database credentials are deployment secrets governed by [ADR-0018](0018-security-identities-credentials-and-secret-management-policy.md).

MariaDB will use durable storage independent of the lifecycle of the database container.

Container replacement must not itself destroy application data.

MariaDB deployment must support the project's initial `linux/amd64` and `linux/arm64` runtime targets from [ADR-0011](0011-target-linux-and-ubuntu-first.md).

The exact tables, columns, keys, indexes, transaction boundaries, migration tooling, image version, storage path, and backup commands are implementation or operations details.

Canonical relational requirements are defined by [ADR-0022](0022-canonical-relational-schema-revision-history-provenance-coverage-and-gaps.md).

Backup and recovery policy is defined by [ADR-0023](0023-source-data-retention-backup-export-and-restore.md).

## Consequences

The project gains a relational backend with transactions, constraints, and mature operational tooling.

Database-specific behavior is contained behind the persistence boundary rather than exposed to application clients.

The project must operate and back up a MariaDB service.

Future support for another database would require deliberate persistence-layer work rather than assuming SQL portability automatically.
