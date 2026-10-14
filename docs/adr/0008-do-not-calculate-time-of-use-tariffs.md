<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0008: Do not calculate Time-of-Use tariffs

- Status: Accepted
- Date: 2026-09-29

## Context

Time-of-Use (TOU) pricing assigns different electricity rates based on when
energy is consumed.

TOU schedules may depend on factors such as:

- time of day;
- weekday or weekend;
- season;
- holidays;
- tariff effective dates;
- customer rate plan;
- service territory;
- critical-peak events;
- consumption tiers;
- other provider-specific rules.

Determining the applicable TOU period and calculating charges can therefore
require substantially more information than an interval timestamp and energy
quantity.

Green Button / ESPI data may include TOU-related metadata, tariff
classifications, consumption tiers, reading metadata, and monetary amounts
supplied by the data custodian or provider.

Those values are source data and may be useful to downstream consumers.

However, independently interpreting tariff schedules and reproducing utility
billing calculations is a different responsibility from ingesting and
normalizing energy-usage data.

Dedicated tariff-processing projects may address that problem separately.

For example, projects such as `ca-tariff-parse` are designed to parse and
structure published electricity tariff information. That type of tariff
interpretation is outside the responsibility of this project.

## Decision

This project will not calculate Time-of-Use tariffs.

The ingestion system will not independently:

- determine which TOU period applies to an interval;
- classify an interval as on-peak, off-peak, super-off-peak, or another tariff
  period based on time alone;
- interpret utility rate schedules;
- determine which tariff or rate plan applies to a customer;
- calculate electricity charges from energy usage and tariff schedules;
- recalculate or validate customer bills;
- determine whether a provider-calculated charge is correct.

When TOU-related information is supplied by the source, the ingestion system
may preserve it as source data and provenance.

This may include, when present:

- TOU classification or code;
- consumption tier;
- critical-peak or similar event classification;
- tariff-related identifiers;
- provider-calculated monetary amounts;
- other related source metadata.

Provider-supplied calculated amounts will be treated as reported source values,
not as calculations performed or independently verified by this project.

The ingestion system will preserve source timestamps and local-time context
needed to interpret the source data correctly, but it will not use that
information to derive TOU classifications.

Questions or disputes concerning:

- tariff applicability;
- TOU classification;
- electricity rates;
- provider-calculated amounts;
- billed charges;

are outside the scope of this project and should be addressed with the
applicable utility, EDC, CCA, energy supplier, or other responsible provider.

Future integration with a dedicated tariff-calculation or tariff-parsing system
may be supported through a separate interface.

Any such external calculation must remain distinguishable from values supplied
by the original energy-data source.

## Consequences

The ingestion system remains focused on acquiring, normalizing, preserving, and
serving energy-usage data.

TOU-related metadata supplied by providers is not discarded merely because the
project does not calculate tariffs.

Provider-supplied monetary values can be retained without implying that this
project has reproduced or validated the provider's billing calculation.

The project avoids duplicating complex and jurisdiction-specific tariff logic.

Changes to utility rate schedules do not require changes to the core ingestion
engine solely for tariff calculation.

A future tariff engine can consume normalized interval data and preserved
source metadata without becoming part of the ingestion core.

Canonical UTC timestamps remain independent of tariff interpretation. Any
future TOU calculation system will be responsible for applying the appropriate
local-time, calendar, tariff, and effective-date rules.

Documentation and user interfaces must clearly distinguish:

- measured or reported energy usage;
- provider-supplied tariff or monetary information;
- externally calculated tariff results, if supported in the future.

Testing must verify that TOU metadata and provider-supplied monetary values are
preserved without causing the ingestion system to derive or recalculate tariff
results.
