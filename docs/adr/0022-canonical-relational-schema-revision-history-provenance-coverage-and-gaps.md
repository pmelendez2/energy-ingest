<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0022: Canonical Relational Schema, Revision History, Provenance, Coverage, and Gaps

- Status: Accepted
- Date: 2026-09-30
- Updated: 2026-10-02

## Context

`energy_ingest` requires a durable relational representation for normalized
Green Button/ESPI energy data and the application state needed to explain,
re-import, revise, and validate that data.

The canonical store must preserve:

- Data Custodian and Acquisition Source identity;
- Green Button Usage Points;
- Reading Types;
- Meter Readings;
- Interval Blocks;
- canonical interval readings;
- revision history;
- ingestion source tracking;
- retrieval coverage;
- known gaps;
- ingestion runs;
- and mutable OAuth authorization state when OAuth is used.

The database is not intended to be a byte-for-byte mirror of Green Button XML.

The canonical model preserves the Green Button concepts and source identifiers
required for correct interpretation, reconciliation, revision handling, and
future ingestion.

The initial project scope is residential.

Business, commercial, aggregator, multi-property commercial, and multi-tenant
account models are out of scope for the initial schema.

## Decision

The canonical relational store will use MariaDB through the `energy-store`
persistence boundary defined by
[ADR-0016](0016-use-mariadb-for-persistent-relational-storage.md).

Migration SQL is the authoritative schema definition.

The schema will maintain application-controlled numeric identifiers while
preserving source-provided identifiers where needed for reconciliation and
re-import.

The initial schema is organized around:

- `data_custodian`;
- `acquisition_source`;
- `ingestion_run`;
- `usage_point`;
- `reading_type`;
- `meter_reading`;
- `interval_block`;
- `interval_reading`;
- `interval_reading_revision`;
- `retrieval_coverage`;
- `retrieval_coverage_revision`;
- `retrieval_gap`;
- and `oauth_authorization`.

The exact column definitions, indexes, foreign keys, checks, and comments remain
defined by the version-controlled migration SQL.

## Data Custodian and Acquisition Source

A Data Custodian represents the external organization or system responsible for
the source data.

An Acquisition Source represents the configured source through which
`energy_ingest` acquires data from that Data Custodian.

Application-controlled identifiers are used internally.

Source-provided identifiers are preserved where needed for reconciliation and
re-import.

An ingestion run records one execution of source acquisition/import processing.

## Green Button Canonical Entities

The canonical schema preserves Green Button concepts needed for correct
interpretation of Energy Usage Information.

### Usage Point

A Usage Point is scoped to an Acquisition Source and preserves its
source-provided identifier.

A single Acquisition Source may contain multiple Usage Points.

Usage Points may represent different meters, service points, or commodities,
including electric and gas usage.

Usage Point identity is independent of OAuth authorization identity.

### Reading Type

Reading Type preserves the Green Button numeric codes needed to interpret a
reading, including applicable fields such as:

- accumulation behaviour;
- commodity;
- data qualifier;
- flow direction;
- kind;
- phase;
- power-of-ten multiplier;
- and unit of measure.

The persistence layer stores the numeric Green Button representation rather than
inventing provider-specific text values.

### Meter Reading

A Meter Reading associates a Usage Point with a Reading Type and preserves its
source-provided identifier.

### Interval Block

An Interval Block belongs to a Meter Reading and preserves its source identity
and time range.

## Canonical Interval Identity

A canonical interval reading is identified by:

- Meter Reading;
- interval start;
- interval duration.

Repeated imports of the same canonical interval must resolve to the same
canonical interval identity.

A zero-valued reading is a real reading and must remain distinguishable from
missing data.

Canonical timestamps follow
[ADR-0004](0004-store-canonical-timestamps-in-utc.md).

## Revision History

Interval values may change when a Data Custodian republishes or corrects data.

The schema therefore separates canonical interval identity from observed
revision values.

Each observed revision records enough information to explain the observation,
including:

- canonical interval identity;
- ingestion run;
- source Interval Block when available;
- source-provided identifier when available;
- source publication/update timestamps when available;
- raw reading value;
- reading quality when available;
- and storage timestamp.

The schema must permit more than one observed revision in a single ingestion
run.

A canonical interval may identify one accepted revision.

The database must prevent an interval from accepting a revision belonging to a
different canonical interval.

Revision-aware import behavior follows
[ADR-0005](0005-use-revision-aware-interval-imports.md).

## Ingestion Source Tracking

The canonical store must retain enough source information to explain where
stored data came from without copying every field from the original source
payload.

This includes the Data Custodian, Acquisition Source, ingestion run, preserved
external identifiers, and source timestamps where useful.

Shared persistence APIs should use plain ingestion-source terminology rather
than exposing XML-specific terminology.

## Retrieval Coverage

The application must track what time ranges were requested or observed so that
absence of a reading is not automatically interpreted as absence of data.

Coverage is scoped to:

- Acquisition Source; and
- optionally a Usage Point.

A `NULL` Usage Point represents source-wide coverage.

The schema must enforce uniqueness for both Usage Point-scoped and source-wide
coverage ranges.

Coverage range end must be later than range start.

Coverage and idempotent re-import behavior follow
[ADR-0010](0010-handle-reimports-overlaps-and-gaps-idempotently.md).

## Coverage Revisions and Gaps

A retrieval coverage range may have observed states such as:

- available;
- partial;
- unavailable.

Coverage state is revisioned by ingestion run.

Known unavailable subranges are stored as retrieval gaps.

Each gap records:

- the coverage revision in which it was observed;
- gap start;
- gap end;
- and an optional reason.

Gap end must be later than gap start.

Missing data and a known unavailable range are not the same thing.

## OAuth Authorization State

OAuth authorization state is an acquisition concern and is separate from
canonical energy-data identity.

For the initial residential scope:

- an Acquisition Source may have zero or one OAuth authorization record;
- one OAuth authorization may expose multiple Usage Points, meters, and
  commodities;
- no direct one-to-one relationship is defined between OAuth authorization and
  Usage Point;
- token refresh updates the existing authorization state;
- reauthorization replaces or updates the same logical authorization state;
- multiple concurrent authorization records for one Acquisition Source are not
  required.

The OAuth table therefore represents the current mutable authorization state for
an Acquisition Source rather than a collection of independent authorization
relationships.

The schema should enforce at most one OAuth authorization row per Acquisition
Source.

An external/provider authorization identifier may be retained when supplied,
but it is not the application's primary identity.

Business, commercial, aggregator, multi-property commercial, delegated, and
multi-tenant authorization relationships are out of scope.

OAuth lifecycle behavior follows
[ADR-0021](0021-mutable-oauth-token-persistence-and-lifecycle.md).

## Residential Scope

The initial schema is designed for residential/local-first energy ingestion.

The project may support:

- multiple Usage Points;
- multiple meters;
- multiple commodities;
- electric and gas data;
- repeated imports;
- multiple Data Custodians or Acquisition Sources configured by the local
  deployment.

The initial schema does not attempt to model:

- business or commercial account hierarchies;
- commercial property portfolios;
- landlord/tenant billing relationships;
- aggregator customer hierarchies;
- delegated multi-user access;
- enterprise multi-tenancy;
- or multiple concurrent OAuth authorizations for one Acquisition Source.

Those capabilities require demonstrated use cases before they are added.

## Database-Enforced Invariants

Important canonical invariants should be enforced in MariaDB where practical.

Examples include:

- source-scoped uniqueness of external identifiers;
- positive interval durations;
- unique canonical interval identity;
- accepted revision ownership;
- valid coverage ranges;
- valid coverage status values;
- valid gap ranges;
- and one OAuth authorization row per Acquisition Source for the initial
  residential model.

Rust repository APIs should expose typed application boundaries, but the
migration SQL remains authoritative for relational constraints.

## Provider Independence

The persistence model is provider-independent.

Provider adapters may contain Data Custodian-specific acquisition behavior, but
provider-specific fields should not leak into canonical persistence unless they
represent an accepted canonical concern.

XML-specific details belong in Green Button parsing/provider layers rather than
generic persistence APIs.

## Testing

Schema behavior must be tested according to
[ADR-0020](0020-standardized-testing-and-provider-test-data-policy.md).

Automated integration tests should verify important invariants, including:

- migration completeness;
- source-scoped uniqueness;
- Green Button entity relationships;
- numeric Green Button code storage;
- canonical interval idempotency;
- revision history;
- accepted-revision ownership;
- coverage identity;
- gap constraints;
- OAuth source uniqueness;
- and foreign-key relationships.

Tests should use synthetic or sanitized data and must not require routine live
Data Custodian access.

## Consequences

### Positive

- The schema remains understandable and relational.
- Source identity and revision history remain explainable.
- Re-import behavior is deterministic.
- Coverage and gaps are represented explicitly.
- One authorization can expose multiple residential Usage Points and
  commodities without coupling OAuth to meter identity.
- Commercial and aggregator complexity is avoided until needed.
- Database constraints protect important invariants.

### Negative

- Future business, commercial, aggregator, or multi-tenant requirements may
  require schema expansion.
- Future support for multiple concurrent OAuth authorizations per Acquisition
  Source would require revisiting ADR-0021 and this ADR.
- The relational model requires explicit migrations as canonical concepts
  evolve.

### Neutral

- This ADR does not define provider API behavior.
- This ADR does not define OAuth protocol implementation.
- This ADR does not define backup/restore policy.
- This ADR does not define local API authentication.
- Detailed DDL remains in the migration SQL rather than duplicated here.

## Alternatives Considered

### Model business/commercial account hierarchies now

Not selected.

The initial supported use case is residential. Adding commercial properties,
account hierarchies, delegated users, or aggregator relationships now would add
complexity without a demonstrated requirement.

### Allow multiple OAuth authorization records per Acquisition Source

The earlier schema direction allowed multiple authorization relationships.

It was simplified because the initial residential model requires at most one
active authorization per Acquisition Source, while that authorization may expose
many Usage Points and commodities.

The schema can be extended later if a supported use case demonstrates the need.

### Mirror Green Button XML directly in relational tables

Not selected.

The database stores a canonical relational representation rather than a
byte-for-byte XML structure.

### Store only current interval values

Not selected.

Revision history is required so corrected or republished source values do not
destroy prior observations.

### Infer coverage only from stored readings

Not selected.

The absence of readings does not prove that a time range was requested,
available, or unavailable.
