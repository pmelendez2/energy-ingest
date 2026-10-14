<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0003: Preserve source reading metadata

- Status: Accepted
- Date: 2026-09-29

## Context

Energy readings are not fully described by a timestamp and numeric value.

Green Button / ESPI ReadingType and IntervalReading resources may provide
additional information needed to interpret a measurement correctly, including:

- commodity;
- flow direction;
- interval length;
- unit of measure;
- power-of-ten multiplier;
- reading quality;
- time-of-use classification;
- consumption tier;
- cost;
- source resource identifiers.

A Data Custodian may also supply metadata that is useful for determining provenance,
revision order, or data quality.

Normalizing source data too early can discard information that may later be
needed to:

- determine whether a reading was estimated, validated, or revenue-quality;
- distinguish electricity import, export, and net measurements;
- reproduce or verify unit conversion;
- diagnose incorrect parsing;
- compare revised readings with earlier values;
- support future Data Custodian integrations or measurement types.

The initial SDG&E Green Button files demonstrate that electricity and gas use
different interval lengths and ReadingType metadata. Individual readings also
contain ReadingQuality values.

## Decision

The ingestion system will preserve both the source representation needed for
auditability and a normalized representation suitable for downstream use.

For interval measurements, the canonical model will preserve, when available:

- interval start time;
- interval duration;
- raw source value;
- normalized value;
- unit of measure;
- power-of-ten multiplier;
- service or commodity type;
- flow direction;
- ReadingType identity;
- UsagePoint and MeterReading identities or equivalent source references;
- all ReadingQuality values associated with the reading;
- Data Custodian/source identity;
- source-file or source-request provenance.

ReadingQuality will support more than one quality value for an interval because
ESPI permits multiple ReadingQuality elements.

The ingestion system will not discard the raw source value after unit
normalization.

Data Custodian-specific metadata that is required to determine revision order,
quality, or provenance should also be retained or represented in a form that
allows the original interpretation to be reconstructed.

Normalized values will be derived from source metadata rather than replacing it.

Downstream consumers such as Home Assistant may use only a subset of this
metadata, but the canonical ingestion layer will preserve the additional
information.

## Consequences

Stored energy data can be audited against its original source representation.

Unit conversions can be reproduced and tested.

Reading-quality information remains available for future display, filtering,
diagnostics, or correction handling.

The model can distinguish measurement streams such as electricity import,
electricity export, net usage, and gas consumption without inferring their
meaning from names or units alone.

The persistent data model will contain more fields than are immediately needed
by Home Assistant.

Storage requirements will be slightly larger.

Data Custodian adapters and parsers must preserve metadata rather than returning only
timestamp/value pairs.

Schema changes may be required in the future when a Data Custodian exposes useful
metadata not represented by the initial canonical model.

Preserving source metadata does not require retaining every byte of every raw
Data Custodian response indefinitely. Raw-source retention, backup, and recovery
are governed by [ADR-0023](0023-source-data-retention-backup-export-and-restore.md).
