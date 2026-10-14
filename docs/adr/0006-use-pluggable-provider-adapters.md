<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0006: Use Pluggable Data Custodian Adapters

- Status: Accepted
- Date: 2026-09-30

## Context

Energy Usage Information may be acquired from more than one Data Custodian or through more than one supported acquisition mechanism.

Examples include:

- local Green Button DMD files;
- Green Button CMD interfaces;
- Data Custodian APIs that return supported Green Button/ESPI data;
- and other supported mechanisms that ultimately provide Green Button/ESPI input.

Data Custodians may differ in:

- authentication;
- request and download workflow;
- pagination;
- update frequency;
- revision semantics;
- rate limits;
- identifiers;
- available metadata;
- and supported Green Button/ESPI capabilities.

[ADR-0001](0001-separate-provider-acquisition-from-normalization.md) separates Data Custodian acquisition from Green Button/ESPI parsing and normalization.

The ingestion core should not contain Data Custodian-specific acquisition logic throughout normalization, persistence, or downstream integrations.

## Decision

`energy_ingest` will use pluggable **Data Custodian adapters**.

A Data Custodian adapter is a project software component. It is not an additional Green Button industry actor.

Each adapter is responsible for Data Custodian-specific behavior needed to obtain and describe supported source data.

An adapter may handle:

- source discovery;
- file or API acquisition;
- authentication integration;
- pagination;
- Data Custodian-specific identifiers;
- source creation or publication metadata;
- revision and ordering metadata;
- capability discovery;
- and transport or API errors.

A Data Custodian adapter that obtains Green Button/ESPI XML must pass that content to the shared Green Button/ESPI parser rather than implementing a separate canonical parsing path.

Adapters do not translate proprietary non-Green-Button formats into the canonical model. Such formats are outside the current project scope according to [ADR-0020](0020-standardized-testing-and-provider-test-data-policy.md).

Adapters may expose supported capabilities explicitly where doing so prevents the core from guessing Data Custodian behavior.

Capabilities may include whether an integration supports:

- electricity or other supported commodities;
- import measurements;
- export measurements;
- historical backfill;
- incremental retrieval;
- revisions;
- explicit source ordering;
- OAuth;
- and file-based import.

The adapter boundary does not require every Data Custodian to support every capability.

Data Custodian credentials and mutable authentication state remain outside the canonical energy-data model.

The exact Rust trait or module interface may evolve without changing this decision.

## Consequences

New Data Custodian integrations can be added without embedding integration-specific acquisition logic into storage or Home Assistant integration layers.

Multiple Data Custodians that provide Green Button/ESPI data can reuse the same parser and normalization logic.

Data Custodian-specific revision semantics may remain isolated in the adapter while the canonical ingestion layer applies the common revision rules defined by [ADR-0005](0005-use-revision-aware-interval-imports.md).

Testing can separate:

- Data Custodian adapter behavior;
- Green Button/ESPI parsing;
- canonical normalization;
- and persistence.

Adapters add an abstraction layer and may differ significantly in complexity.

The project must avoid allowing Data Custodian-specific fields to leak into the canonical model unless they represent useful source metadata, provenance, or another accepted canonical concern.

Data Custodian adapters should remain replaceable and independently testable.
