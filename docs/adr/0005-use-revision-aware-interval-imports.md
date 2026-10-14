<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0005: Use revision-aware interval imports

- Status: Accepted
- Date: 2026-09-29

## Context

Data Custodians may publish interval data more than once.

A later file or API response may overlap data that was already imported and may
contain:

- the same value again;
- a revised value for the same interval;
- updated reading-quality information;
- additional intervals that fill a previous gap;
- Data Custodian-specific correction or revision metadata.

Some Data Custodian workflows explicitly require consumers to process source files in
creation order because later files may supersede earlier values for the same
interval.

UtilityAPI's supplier-oriented bulk workflow was reviewed during project
research as one example of this behavior. That workflow is Data Custodian-specific
and is not treated as a Green Button / ESPI requirement.

The ingestion system must therefore support both duplicate data and later
corrections without assuming that an interval becomes immutable after its first
import.

Using only the newest imported timestamp as a cursor is not sufficient. A later
source may contain a correction for an older interval or may fill a gap that
occurs before the newest stored timestamp.

## Decision

Interval ingestion will be revision-aware and idempotent.

The system will identify a logical interval using stable measurement identity
and interval characteristics rather than import order alone.

The identity will include enough information to distinguish separate
measurement streams and the interval being measured. This may include:

- Data Custodian or source identity;
- UsagePoint or equivalent service-point identity;
- MeterReading or measurement-stream identity;
- ReadingType or equivalent measurement type;
- interval start time;
- interval duration.

When the same logical interval is received more than once:

- an identical repeated reading will not create a duplicate interval;
- a later authoritative reading may replace or supersede the active value;
- updated reading-quality information may update the active interval;
- provenance of the source that supplied the active value will be preserved;
- earlier values should remain traceable where practical when they are
  superseded.

Data Custodian adapters are responsible for exposing ordering and revision metadata
when their Data Custodian defines such semantics.

When available, authoritative source ordering may include:

- Data Custodian creation timestamps;
- sequence numbers;
- revision identifiers;
- publication timestamps;
- other Data Custodian-defined ordering information.

The system will not rely solely on:

- file names;
- filesystem modification times;
- local import time;
- the greatest interval timestamp already stored.

When a Data Custodian does not supply explicit revision-order information, the
ingestion layer will use a documented fallback policy.

Coverage tracking and revision handling are separate concerns. Filling an older
gap must remain possible even when newer intervals have already been imported.

## Consequences

Repeated imports can be processed safely without double-counting energy usage.

Historical corrections can be accepted without deleting and rebuilding all
later data.

A later source can repair an older gap even when the system already contains
newer readings.

Data Custodian-specific revision semantics remain isolated in Data Custodian adapters
rather than becoming assumptions in the canonical data model.

The persistent store must maintain sufficient identity and provenance to
compare new readings with existing intervals.

Storage and ingestion logic will be more complex than an append-only model.

The project will need tests for:

- identical re-imports;
- overlapping imports;
- corrections to older values;
- quality-only revisions;
- out-of-order source delivery;
- gap filling;
- conflicting revisions;
- Data Custodian sources with and without explicit revision metadata.

The canonical persistence requirements for revision history are defined by
[ADR-0022](0022-canonical-relational-schema-revision-history-provenance-coverage-and-gaps.md).
