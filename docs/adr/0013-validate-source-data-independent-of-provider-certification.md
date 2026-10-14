<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0013: Validate Source Data Independently of Data Custodian Certification

- Status: Accepted
- Date: 2026-09-29

## Context

A Data Custodian may describe an implementation or payload as Green Button compliant or certified.

Certification or self-description is useful provenance, but it does not prove that every individual payload is syntactically valid, structurally complete, semantically usable, or compatible with the Green Button/ESPI versions supported by `energy_ingest`.

Conversely, absence of known certification does not automatically prove that otherwise valid supported Green Button/ESPI data is unusable.

## Decision

`energy_ingest` will validate source data independently of Data Custodian certification status.

Acceptance of Green Button/ESPI input will be based on the source data and the project's supported Green Button/ESPI contract rather than solely on certification claims.

Validation must be sufficient to determine whether the source is usable by the ingestion pipeline.

This includes, as applicable:

- well-formed XML;
- supported Green Button/ESPI structure;
- required relationship resolution;
- supported version/profile expectations;
- and semantic consistency required by normalization.

Certification or compatibility claims may be retained as provenance when reliably available, but they do not bypass validation.

Malformed or unsupported input must not be accepted merely because the Data Custodian is certified or claims compatibility.

Likewise, otherwise valid supported input must not be rejected solely because certification status is unknown.

The exact validation library, schema-validation commands, error taxonomy, and test corpus belong in implementation and testing documentation.

Testing policy follows [ADR-0020](0020-standardized-testing-and-provider-test-data-policy.md).

## Consequences

The ingestion boundary remains under project control rather than delegated to external certification status.

Certification can still be recorded as useful provenance.

The project must maintain its own validation behavior for the Green Button/ESPI versions it claims to support.
