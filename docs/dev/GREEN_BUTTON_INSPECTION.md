<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

<!-- SPDX-License-Identifier: MIT -->

# Green Button XML Inspection

## Purpose

`gb-inspect` provides a human-readable summary of Green Button / ESPI
XML.

The command is intended for:

- development;
- fixture validation;
- troubleshooting;
- profile creation and review; and
- confirming the characteristics of downloaded Green Button XML.

The inspector determines measurement characteristics from the XML itself.
Provider filenames, portal labels, and user descriptions are not used as
parsing rules.

**Intended Audience:** Developers, profile maintainers, and operators inspecting XML.

## Green Button Terminology

The inspector uses existing Green Button terminology wherever possible.

### MeterReading

A `MeterReading` is the Green Button resource used as the primary unit of
inspection output.

For each `MeterReading`, the inspector resolves its related `ReadingType` and
associated `IntervalBlock` resources.

The Rust type `MeterReadingSummary` is an `energy_ingest` summary of those
Green Button resources. It is not a new Green Button resource type.

Terms such as "measurement stream" and "measurement series" are intentionally
not used because existing Green Button terminology is sufficient.

### ReadingType

A `ReadingType` describes characteristics associated with readings in a
`MeterReading`.

The inspector currently uses `ReadingType` values including:

- commodity;
- flow direction;
- unit of measure; and
- interval length.

A Green Button document may contain multiple `ReadingType` resources even
when only one is related to a specific `MeterReading`.

The inspector follows Atom/ESPI relationships to select the applicable
`ReadingType`. It does not select the first `ReadingType` found in the file.

### IntervalBlock

An `IntervalBlock` contains interval readings associated with a
`MeterReading`.

The inspector follows the `MeterReading` related link to the corresponding
`IntervalBlock` relationship instead of assuming that an `IntervalBlock`
directly references the `MeterReading` self URL.

This distinction is important for the SDG&E Green Button XML represented by
the golden fixtures.

### IntervalReading

An `IntervalReading` contains a measured value and its time period.

The inspector uses the actual `IntervalReading.timePeriod.start` and
`IntervalReading.timePeriod.duration` values to determine:

- observed interval length;
- reading count;
- first reading time; and
- last reading time.

## Human-Readable Output

Green Button uses numeric enumeration values for many fields.

The inspector retains the raw Green Button values internally while displaying
known human-readable labels with the original code in parentheses.

Examples:

```text
Commodity: Electricity
Flow direction: Forward (1)
Unit: Wh (72)
```

For natural gas, an example is:

```text
Commodity: Natural gas
Flow direction: Forward (1)
Unit: therm (169)
```

Unknown codes are not guessed. They remain visible as raw values so the
provider data can still be inspected and researched.

Example:

```text
Unit: Unknown (456)
```

## Interval Display

Known interval durations are formatted for humans while preserving their
numeric value.

Examples:

```text
Expected interval: 1 hour (3600 seconds)
Observed interval: 1 hour (3600 seconds)
```

and:

```text
Expected interval: 1 day (86400 seconds)
Observed interval: 1 day (86400 seconds)
```

The expected interval comes from `ReadingType.intervalLength`.

The observed interval comes from the distinct
`IntervalReading.timePeriod.duration` values found in the XML.

## Expected vs Observed Interval Validation

The output explicitly identifies what is being compared:

```text
Expected vs Observed interval validation: Match
```

`Match` does not mean that the XML merely parsed successfully.

It means that:

1. the related `ReadingType` provides an expected interval length;
2. one uniform interval duration was observed in the associated
   `IntervalReading` records; and
3. the expected and observed durations are equal.

Possible results are:

- `Match` - the expected and observed interval durations are equal;
- `Mismatch` - one observed duration exists but differs from the expected
  interval;
- `Mixed` - multiple observed interval durations exist; and
- `Unknown` - insufficient information exists to make the comparison.

## Date and Time Handling

Green Button timestamps are retained internally as exact Unix/UTC instants.

Human-readable output uses the process environment's local time zone through
the operating system.

The program does not set or change `TZ`.

Development, test, and production environments are responsible for providing
the correct local time-zone configuration.

The fixture manifest therefore does not contain a time-zone setting.

This keeps time-zone configuration in the runtime environment rather than
duplicating it in profile metadata.

## Golden Fixture Validation

The SDG&E golden fixtures provide regression coverage for the first tested
Green Button provider profile.

The tests currently verify that:

- fixture expectations come from a human-readable TOML manifest;
- interval characteristics are derived from XML rather than filenames;
- Green Button relationships resolve the correct `ReadingType`;
- `IntervalBlock` readings are associated through the Green Button links;
- expected and observed interval durations agree;
- expected reading counts agree;
- expected local calendar coverage agrees; and
- known Green Button codes map to expected human-readable labels.

The fixture manifest uses native TOML calendar dates, for example:

```toml
start_date = 2026-01-01
end_date = 2026-01-31
```

Unix timestamps are intentionally not used in the human-reviewed manifest.

## Example Electricity Output

```text
Green Button Usage Summary

MeterReading 1 of 1
  Commodity: Electricity
  Flow direction: Forward (1)
  Unit: Wh (72)
  Expected interval: 1 hour (3600 seconds)
  Observed interval: 1 hour (3600 seconds)
  Interval readings: 744
  First reading: 2026-01-01 00:00:00 -08:00 (-08:00)
  Last reading: 2026-01-31 23:00:00 -08:00 (-08:00)
  Expected vs Observed interval validation: Match

XML Resources
  Reading types: 5
  Meter readings: 1
  Interval blocks: 1
```

## Example Natural Gas Output

```text
Green Button Usage Summary

MeterReading 1 of 1
  Commodity: Natural gas
  Flow direction: Forward (1)
  Unit: therm (169)
  Expected interval: 1 day (86400 seconds)
  Observed interval: 1 day (86400 seconds)
  Interval readings: 31
  First reading: 2026-01-01 23:59:00 -08:00 (-08:00)
  Last reading: 2026-01-31 23:59:00 -08:00 (-08:00)
  Expected vs Observed interval validation: Match

XML Resources
  Reading types: 5
  Meter readings: 1
  Interval blocks: 1
```

## Relationship to Ingestion

Green Button inspection and Green Button ingestion are related but distinct
operations.

Inspection reads supported Green Button/ESPI XML, resolves resource
relationships, and produces human-readable summaries useful for validation and
troubleshooting.

Inspection does not persist Energy Usage Information.

The inspection-specific `GreenButtonInspection` and `MeterReadingSummary`
types are reporting types. They are not the canonical persistence model.

The shared parser also exposes `GreenButtonDocument` through `parse_file` and
`parse_reader`. That normalized document uses the same relationship resolution
as inspection while retaining the source data required to populate the
canonical `energy-store` model, including:

- source resource identifiers;
- Usage Point relationships;
- Reading Type numeric fields;
- Meter Reading relationships;
- Interval Block identity and time range;
- Interval Reading start and duration;
- reading value;
- source revision metadata when available; and
- ingestion provenance.

Canonical interval identity and revision history remain responsibilities of
`energy-store`.

Golden fixtures are used both for parser regression testing and for selected
database integration tests. Parsing a golden fixture successfully does not by
itself mean that the fixture was persisted.

### End-to-End Ingestion Validation

The initial database-integration target is the sanitized
`sdge-electric-one-day-hourly.xml` fixture.

The end-to-end test should verify that the shared Green Button parsing path can:

- persist the applicable Usage Point, Reading Type, Meter Reading, and
  Interval Block;
- persist canonical interval readings using UTC interval identity;
- record observed interval-reading revisions with ingestion provenance;
- preserve zero-valued readings as real readings;
- verify persisted counts, timestamps, durations, and values; and
- re-import the same fixture without duplicating canonical intervals.

After that test passes, representative larger electricity and natural-gas
fixtures should be validated through the same ingestion path.

## Profile Workflow Direction

The SDG&E fixtures are the first model for a repeatable Green Button profile
workflow.

A future EDC and/or CCA profile contribution should:

1. begin with customer-obtained Green Button XML;
2. sanitize customer-specific identifiers and usage values locally;
3. retain provider-specific XML structure and relationships;
4. generate reusable golden fixtures and human-readable expectations;
5. run the generic Green Button validation path;
6. allow the contributor to review the sanitized result; and
7. contribute the reviewed profile data for inclusion in a future release.

A profile records tested evidence. It must not become a gatekeeper that
prevents otherwise valid supported Green Button XML from being imported.
