<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0004: Store canonical timestamps in UTC

- Status: Accepted
- Date: 2026-09-29

## Context

Energy interval data must remain unambiguous across time zones and daylight
saving time transitions.

Green Button / ESPI represents interval times using TimeType values.

TimeType is a signed 64-bit value representing the number of seconds since
1970-01-01T00:00:00 UTC.

Green Button also defines LocalTimeParameters to describe how UTC timestamps
relate to local civil time.

LocalTimeParameters may include:

- standard time-zone offset;
- daylight saving time offset;
- daylight saving time start rule;
- daylight saving time end rule.

This separates the timestamp of the measurement from its local-time
presentation.

Converting source timestamps to local wall-clock time before persistence would
introduce ambiguity.

For example, during a daylight saving time transition, a local clock time may
occur twice or may not occur at all. The underlying UTC timestamps remain
unambiguous.

Timestamp representation also affects:

- interval identity;
- deduplication;
- revision handling;
- gap detection;
- aggregation;
- Home Assistant statistics;
- Data Custodian interoperability.

Atom metadata such as published and updated timestamps may use ISO 8601
date-time values rather than ESPI TimeType values. These timestamps must also
be normalized to an unambiguous UTC representation before use in canonical
ingestion logic.

## Decision

The ingestion system will use UTC as the canonical time basis for stored
timestamps.

Green Button / ESPI TimeType values will be interpreted as seconds since
1970-01-01T00:00:00 UTC.

Canonical interval records will preserve:

- interval start time as an unambiguous UTC timestamp;
- interval duration separately from the start timestamp;
- source time metadata required to reproduce or validate the interpretation.

The ingestion system will not convert interval timestamps to local wall-clock
time before persistence.

LocalTimeParameters will be preserved when supplied by the source.

Time-zone and daylight saving time information will be used when local-time
interpretation is required, but it will not change the canonical UTC identity
of an interval.

Atom or Data Custodian metadata timestamps used for ordering, revisions, or
provenance will also be converted to an unambiguous UTC representation while
preserving relevant source metadata.

Local-time conversion belongs at presentation, reporting, aggregation, or
Data Custodian-specific boundaries where local civil time is actually required.

The canonical model will not assume a particular local time zone such as
Pacific Time.

## Consequences

Interval timestamps remain globally unambiguous.

Daylight saving time transitions do not create duplicate or nonexistent
canonical timestamps.

Intervals can be compared and deduplicated consistently regardless of the
machine or time zone performing the import.

Revision-aware imports can identify historical intervals without depending on
local wall-clock interpretation.

Home Assistant and other consumers can convert canonical UTC timestamps to the
appropriate local time for display.

Calendar-based aggregation such as local days, billing periods, or
time-of-use periods will require explicit local-time information rather than
assuming that UTC calendar boundaries are equivalent to local calendar
boundaries.

The parser and storage layers must distinguish between:

- absolute timestamps;
- interval durations;
- local-time configuration;
- source metadata timestamps.

Tests must include daylight saving time transitions and verify that local-time
conversion does not alter canonical interval identity.
