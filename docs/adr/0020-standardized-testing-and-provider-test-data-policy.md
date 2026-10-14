<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0020: Standardized Testing and Green Button Test-Data Policy

- Status: Accepted
- Date: 2026-09-30

## Context

`energy_ingest` acquires Energy Usage Information through supported Green Button/ESPI mechanisms.

The acquisition mechanism may vary, but the ingestion data contract remains supported Green Button/ESPI XML.

Data Custodian implementations may differ in authentication, API behavior, pagination, resource population, revisions, missing data, and supported ESPI versions.

Testing must therefore validate both common Green Button/ESPI behavior and Data Custodian-specific acquisition behavior without depending on live Retail Customer accounts.

Real Green Button data may contain sensitive Retail Customer information, so test-data handling is also a security boundary.

## Decision

`energy_ingest` will use a layered, deterministic testing strategy based primarily on:

- standard Rust testing facilities;
- synthetic Green Button data;
- sanitized fixtures where necessary;
- local simulations of Data Custodian behavior;
- disposable integration dependencies;
- and optional live validation against external systems.

The required deterministic test suite must not depend on a live Data Custodian.

Detailed test commands, framework configuration, fixture layout, and CI matrices belong in testing documentation rather than this ADR.

## Green Button / ESPI Scope

The project supports only Green Button/ESPI representations explicitly supported by `energy_ingest`.

It does not claim to ingest arbitrary utility XML, JSON, CSV, or proprietary formats.

Tests must distinguish valid supported input from malformed, unsupported-version, semantically incomplete, or non-Green-Button input.

Non-Green-Button data must not be heuristically treated as supported Energy Usage Information.

## Version Support

The initial normative ESPI baseline is NAESB REQ.21 ESPI 4.1.

Other versions are described as tested only after representative validation exists.

Successful XML parsing alone is not sufficient evidence of version compatibility.

## Test Levels

The project will use the lowest practical test level that meaningfully validates the behavior.

Relevant levels may include unit, component, Data Custodian adapter contract, database integration, API, migration, and selected end-to-end tests.

Not every behavior requires every level.

## Data Custodian Adapter Testing

A Data Custodian adapter is a project software component, not an industry actor.

Adapters should be testable without the real Data Custodian through controlled HTTP/OAuth simulations and representative Green Button fixtures.

Data Custodian-specific behavior must remain testable independently from common parser/normalization behavior.

## Database and Integration Testing

Behavior that depends on MariaDB semantics must be validated against MariaDB rather than only through mocks.

Tests must use isolated/disposable state and must not operate against production or normal user data.

## Regression Testing

A reproducible defect should normally result in an automated regression test at the lowest meaningful level.

The project does not require a unit test specifically for every bug.

## Determinism

Required tests should avoid unnecessary dependence on wall-clock time, local timezone, uncontrolled randomness, external network availability, execution order, or persistent state from earlier runs.

## Test Data

Synthetic Green Button data is preferred when it can represent the behavior adequately.

Sanitized fixtures derived from real structures may be used when necessary to represent actual Data Custodian behavior.

Raw Retail Customer-specific Green Button data must not be committed to the repository.

Sanitized fixtures require human review to ensure both test validity and removal of sensitive information.

Required automated tests must not depend on real deployment or Retail Customer credentials.

## Live Validation

Authorized validation against a real Data Custodian or official test/sandbox environment may supplement the deterministic suite.

Live validation is never a prerequisite for routine automated testing.

The architecture does not assume every Data Custodian provides an accessible sandbox or certification environment.

## Code Coverage

The project does not initially require a numerical coverage threshold.

Coverage may be used diagnostically, but meaningful behavioral protection is preferred over optimizing a percentage.

## Release Validation

Public releases should identify environments and ESPI versions actually exercised and any material validation limitations.

Environment reporting follows [ADR-0019](0019-development-environment-source-control-and-ai-assisted-development-policy.md).

## Security and AI Review

Test credentials and fixture handling follow [ADR-0018](0018-security-identities-credentials-and-secret-management-policy.md).

AI-assisted tests and fixtures follow the human-review requirements in [ADR-0019](0019-development-environment-source-control-and-ai-assisted-development-policy.md).

## Consequences

Routine tests remain reproducible without live external dependencies.

Green Button/ESPI scope is explicit and testable.

Representative fixtures require maintenance and careful sanitization.

Live Data Custodian changes may still require supplemental integration validation.
