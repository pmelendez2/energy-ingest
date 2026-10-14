<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0010: Handle Reimports, Overlaps, and Gaps Idempotently

- Status: Accepted
- Date: 2026-09-29

## Context

Green Button/ESPI imports may overlap data already stored.

This may occur when:

- the same source is processed again;
- a later retrieval overlaps an earlier date range;
- a Data Custodian republishes previously supplied readings;
- a later source corrects a prior value;
- or a later source fills a previously missing interval.

Repeated ingestion must not double-count energy usage.

A missing interval is also not equivalent to a reported interval with a value of zero.

[ADR-0005](0005-use-revision-aware-interval-imports.md) defines revision-aware handling of changed readings.

## Decision

Ingestion will be idempotent at the logical reading/interval level.

Reprocessing equivalent source data must not create duplicate canonical readings or cause usage to be counted more than once.

For an incoming logical interval, the system must be able to distinguish at least:

- new data;
- unchanged data;
- revised data;
- and invalid or rejected data.

An exact reimport is valid and leaves the canonical current state unchanged.

Overlapping imports must preserve valid previously stored readings that are not superseded by a later authoritative revision.

The absence of an interval from a later overlapping source does not, by itself, delete an earlier stored reading.

The system must not fabricate zero-valued readings for intervals absent from the source.

When the expected cadence or coverage is known, missing expected intervals may be represented as gaps.

A gap does not necessarily make the entire ingestion fail; valid readings from the same source may still be accepted when the source is otherwise usable.

Gap and coverage state must be revisable when later acquisition fills previously missing data.

Coverage and gap persistence are defined more fully by [ADR-0022](0022-canonical-relational-schema-revision-history-provenance-coverage-and-gaps.md).

The exact comparison algorithms, import summary formats, retry policy, and optimization of already-seen source artifacts are implementation concerns.

## Consequences

Repeated imports are safe.

Overlapping retrieval windows do not require replacement of an entire historical range.

Corrections can be represented through revision history instead of destructive overwrite.

Missing data remains distinguishable from zero usage.

Later retrievals can fill earlier gaps without duplicating unaffected readings.
