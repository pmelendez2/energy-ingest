<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0009: Separate Data Retention from Data Custodian Authorization

- Status: Accepted
- Date: 2026-09-29

## Context

`energy_ingest` may continue to hold Energy Usage Information after the authorization used to acquire it has expired, been revoked, or otherwise become unavailable.

Authorization lifecycle and data-retention lifecycle are different concerns.

A local-first system should not require continued Data Custodian access merely to retain or use data that was previously acquired.

At the same time, energy-usage data is sensitive and retention must be deliberate rather than an accidental consequence of authentication state.

## Decision

Retention of previously acquired energy data will be managed independently from Data Custodian authentication and authorization state.

Loss, expiration, or revocation of Data Custodian authorization will stop future protected acquisition where applicable, but will not automatically delete previously acquired data.

The project will distinguish, where applicable, between:

- canonical normalized data;
- retained raw source artifacts;
- provenance and revision history;
- operational metadata;
- and backup copies.

Deleting one category of data must not silently imply deletion of unrelated categories.

For example:

- deleting retained raw source artifacts does not automatically delete canonical readings;
- deleting canonical readings does not automatically revoke Data Custodian authorization;
- revoking authorization does not automatically erase retained historical data;
- deleting live application data does not imply immediate removal from historical backups.

By default, successfully ingested canonical energy data may be retained until the user or administrator explicitly removes it or a configured retention policy applies.

The exact retention durations and administrative controls are operational policy rather than architectural decisions.

Authentication credentials must not be embedded in canonical energy records or provenance merely to preserve data retention.

Detailed source-retention, backup, export, and restore policy is defined by [ADR-0023](0023-source-data-retention-backup-export-and-restore.md).

## Consequences

Historical data can remain usable after Data Custodian access ends.

The local data history may be longer than the history still available from the Data Custodian.

Storage consumption may grow until a retention or deletion policy removes data.

Deletion behavior must clearly distinguish active application data, retained source artifacts, authorization state, and backup copies.

Authorization cannot be used as a substitute for an explicit local deletion policy.
