<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0001: Separate Data Custodian Acquisition from Normalization

- Status: Accepted
- Date: 2026-09-30

## Context

`energy_ingest` may obtain Energy Usage Information through more than one acquisition mechanism.

Supported acquisition mechanisms may include:

- local Green Button Download My Data (DMD) XML files;
- Green Button Connect My Data (CMD) interfaces;
- Data Custodian APIs;
- bulk-download workflows;
- and other mechanisms that ultimately provide a Green Button/ESPI representation supported by the project.

Acquisition mechanisms may differ in:

- transport;
- authentication and authorization;
- pagination;
- request and retrieval workflow;
- update frequency;
- revision behavior;
- and Data Custodian-specific metadata.

The core normalization and persistence behavior should not depend on how supported Green Button/ESPI data was obtained.

The project is intentionally not a general-purpose utility-data format translator. The supported ingestion data contract is Green Button/ESPI as established by [ADR-0020](0020-standardized-testing-and-provider-test-data-policy.md).

## Decision

Data Custodian acquisition will be implemented as a separate layer from Green Button/ESPI parsing, normalization, and persistence.

A Data Custodian adapter is responsible for acquisition concerns such as:

- obtaining the source data;
- authentication or authorization integration;
- request construction;
- pagination;
- retries and transport behavior;
- Data Custodian-specific identifiers;
- and acquisition metadata needed for provenance or revision handling.

After acquisition, supported Green Button/ESPI content is passed to the shared Green Button/ESPI parsing and normalization path.

The Data Custodian adapter must not define an independent canonical energy-data format.

Data that does not conform to a Green Button/ESPI version supported by `energy_ingest` is outside the current ingestion scope unless a later architectural decision explicitly expands that scope.

Authentication belongs to the acquisition/authentication boundary rather than the Green Button parser.

OAuth behavior follows [ADR-0007](0007-use-oauth2-as-configurable-provider-authentication.md) and mutable OAuth state follows [ADR-0021](0021-mutable-oauth-token-persistence-and-lifecycle.md).

Persistent storage and downstream consumers, including Home Assistant, operate on the canonical application model rather than Data Custodian responses.

## Consequences

A new Data Custodian integration can be added without changing canonical storage or downstream consumers solely because it uses a different transport or authentication mechanism.

Green Button XML obtained from a local file and Green Button data obtained through an OAuth-protected API can reuse the same Green Button/ESPI parsing and normalization logic.

Data Custodian-specific authentication, URLs, paging behavior, and retrieval metadata remain outside the canonical model except where metadata is intentionally retained for provenance or revision semantics.

The separation adds an acquisition boundary, but avoids coupling the ingestion core to a particular Data Custodian.

The architecture does not support arbitrary Data Custodian-specific XML, JSON, CSV, or proprietary formats as alternate ingestion contracts.
