<!-- SPDX-License-Identifier: MIT -->

# Green Button electricity flow foundations

**Purpose:** Document foundational Green Button solar import/export handling, measurement-direction safeguards, and regression testing.

**Intended Audience:** Energy Ingest developers, contributors, and maintainers.

## Scope

The initial Energy Ingest release handles electricity consumption and natural
 gas usage. Solar production and grid-export reporting are not yet supported.

## Existing preservation

The Green Button parser and MariaDB `reading_type` already preserve
`flow_direction`. Accepted revisions and reading-type provenance are retained.
No new database migration is needed for this safety guard.

## Consumption guard

For electricity (`commodity = 1`), the physical-meter consumption resolver
accepts only `flow_direction = 1` (forward). Reverse (`19`), total (`20`),
unspecified (`0`), and other directions produce an explicit
`unsupported_electricity_flow` issue. If an interval has an issue, the
resolver returns no consumption intervals for that window. This prevents
mixing exported kWh into import consumption; it does **not** delete or
reclassify the stored readings. Natural gas is unchanged.

## Follow-up before solar reporting

- Obtain an authorized, representative provider Green Button XML sample
  containing distinct import and reverse/export streams.
- Verify `ReadingType` units, flow direction, interval associations, and
  meter metadata; do not assume export equals onsite solar production.
- Introduce an explicit stream selection and API model for import, export,
  and net readings without changing the meaning of the consumption API.
- Add source-derived and synthetic Green Button XML fixtures and tests
  for gaps, updates, duplicate streams, and mixed cadence.
- Never synthesize missing observations or monetary sell-back amounts.

The guard is intentionally conservative. Any legitimate provider electricity
record with a different direction needs a documented semantic decision before
it is used as consumption.
