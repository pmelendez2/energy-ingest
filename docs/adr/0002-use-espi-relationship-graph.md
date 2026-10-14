<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0002: Use ESPI relationships to resolve Green Button data

- Status: Accepted
- Date: 2026-09-29

## Context

Green Button / ESPI XML contains multiple related resource types, including
UsagePoint, MeterReading, ReadingType, IntervalBlock, and IntervalReading.

The meaning of an interval reading depends on metadata held in related
resources.

For example, ReadingType may define:

- commodity;
- flow direction;
- interval length;
- unit of measure;
- power-of-ten multiplier;
- default reading quality;
- time-of-use classification.

A Green Button XML document may contain multiple ReadingType resources.

Selecting the first ReadingType found within a Green Button XML document, or
otherwise relying on XML element order within that document, can associate
readings with the wrong metadata.

The SDG&E Green Button files used during initial project research demonstrate
this condition. Electricity and gas XML files may contain several ReadingType
definitions even though only specific ReadingType resources are related to
particular MeterReading resources.

Green Button / ESPI uses Atom links and resource identifiers to express these
relationships.

The parser therefore needs to preserve and resolve those relationships rather
than treating the XML as a flat collection of elements.

This ADR addresses relationships and ordering within a single Green Button XML
document. Ordering, revisions, and supersession across multiple source files
are separate ingestion concerns and are addressed by other architecture
decisions.

## Decision

The Green Button parser will resolve ESPI resources using their defined
relationships.

The parser will build or otherwise maintain sufficient relationship information
to associate:

- UsagePoint with MeterReading;
- MeterReading with ReadingType;
- MeterReading with IntervalBlock;
- IntervalBlock with IntervalReading;
- other related ESPI resources when required.

Resource relationships will be resolved using ESPI / Atom identifiers and links
rather than XML element order within a source document.

Reading metadata will be taken from the ReadingType associated with the
MeterReading containing the readings.

The parser will not assume that:

- there is only one ReadingType in an XML document;
- the first ReadingType applies to all readings;
- electricity and gas use the same ReadingType;
- all measurements represent consumption;
- all readings use the same interval length or unit.

The relationship model should preserve enough source identity to support later
validation, deduplication, revisions, diagnostics, and debugging.

## Consequences

The parser more closely follows the Green Button / ESPI resource model.

Electricity, gas, import, export, net usage, and other measurement streams can
be interpreted using the metadata actually associated with each stream.

XML files containing multiple ReadingType resources can be parsed without
relying on element-order assumptions.

The implementation will be more complex than a flat XML parser.

Tests must include XML documents containing multiple related resource types and
must verify that the correct relationships are resolved.

Malformed or incomplete relationship graphs must produce clear validation or
diagnostic results rather than silently selecting unrelated metadata.

The internal parser implementation may use a graph, indexed resource maps, or
another equivalent structure. This ADR defines the required relationship
semantics, not a specific Rust data structure.
