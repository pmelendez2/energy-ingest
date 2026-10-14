<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0023: Source-Data Retention, Backup, Export, and Restore

- Status: Accepted
- Date: 2026-09-30

## Context

`energy_ingest` stores canonical application data in MariaDB and may retain original Green Button/ESPI source artifacts.

The project needs a simple recovery policy that identifies what must be protected without turning the architecture into a backup runbook.

## Decision

Persistent data is classified into a small number of recovery categories:

1. canonical MariaDB application state;
2. retained raw Green Button source artifacts;
3. deployment configuration that cannot simply be recreated from source control;
4. mutable authorization state;
5. static deployment secrets;
6. temporary or reproducible data.

The architecture requires recoverability, not a particular backup product.

An existing protected Home Lab backup system may be used when it safely covers the required state.

## MariaDB

The MariaDB database is the primary persistent application state and must be included in the recovery design.

The backup method must produce a recoverable, consistent database state.

Exact database commands and schedules are operational details.

## Retained Source Data

If raw Green Button/ESPI source artifacts are retained as part of the application's source-data set, they should be included in backup according to the deployment retention policy.

Raw source artifacts are useful for replay and investigation but are not the normal query database.

Canonical application operation must not require indefinite retention of raw source files.

## Configuration and Credentials

Configuration needed to reconstruct the deployment must either be safely version controlled or included in protected recovery material.

Static secrets follow [ADR-0018](0018-security-identities-credentials-and-secret-management-policy.md).

The deployment must document whether each important secret is backed up, regenerated, reissued, or intentionally excluded.

Mutable OAuth state follows [ADR-0021](0021-mutable-oauth-token-persistence-and-lifecycle.md).

Restored OAuth state is treated as potentially stale and is validated through normal OAuth behavior before use.

## Data Not Normally Backed Up

Temporary or reproducible data does not normally require backup, including caches, build output, transient scratch files, and container layers.

Operational documentation may identify exceptions.

## Restore

A backup is useful only if it can reconstruct a usable installation.

Restore must be able to recover the selected durable state and validate that the database/schema is usable and required configuration is present.

The project should periodically verify that important backup data can actually be restored.

The exact restore procedure belongs in operations documentation.

## Retention and Replay

Raw source retention duration is an operational policy.

Deleting an old raw source artifact does not automatically delete its canonical readings.

Retained sources may be replayed through current ingestion logic, subject to normal idempotency and revision rules.

## Export

Export is separate from backup.

Backup exists to reconstruct an `energy_ingest` installation.

Export exists to provide a portable representation of user-owned canonical energy data.

Exports must not include passwords, OAuth tokens, client secrets, or other authentication credentials.

Exact export formats are implementation choices and should favor open, documented formats.

An export must not be described as a complete backup unless it has actually been designed and tested to reconstruct all required state.

## Authorization and Retention

Loss or revocation of Data Custodian authorization does not automatically delete previously acquired data, consistent with [ADR-0009](0009-separate-data-retention-from-provider-authorization.md).

Authorization state, canonical data, raw source retention, and backup retention are separate lifecycles.

## Consequences

The recovery model remains simple and compatible with existing Home Lab backup infrastructure.

MariaDB is clearly the primary backup requirement.

Retained source artifacts remain useful without becoming mandatory permanent storage.

Backup and export have distinct purposes.

Deployment documentation still needs to identify actual paths, commands, schedules, and retention settings.
