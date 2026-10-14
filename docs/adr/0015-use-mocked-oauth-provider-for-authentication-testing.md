<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0015: Use an Open-Source OAuth Test Server and Data Custodian Stubs

- Status: Accepted
- Date: 2026-09-30

## Context

[ADR-0007](0007-use-oauth2-as-configurable-provider-authentication.md) establishes OAuth 2.0 as a configurable authentication and authorization mechanism for Data Custodian acquisition.

OAuth lifecycle behavior includes more than a single successful authorization. Tests must be able to reproduce conditions such as:

- authorization success or denial;
- token expiration;
- refresh and refresh-token rotation;
- invalid credentials or scopes;
- revocation;
- reauthorization;
- transient OAuth failures;
- and Data Custodian API failures.

Required automated tests must not depend on real Retail Customer credentials, production Data Custodian accounts, or the continued availability of an external service.

The project also should not implement a general-purpose OAuth authorization server solely for testing when maintained open-source implementations can provide the needed protocol behavior.

OAuth protocol behavior and Data Custodian API behavior are related but distinct test boundaries.

## Decision

The project will use a maintained open-source OAuth 2.0 or OIDC test server for deterministic OAuth authentication and authorization testing.

The test server will run locally or in a container and must not require Internet access or real Data Custodian credentials for the required automated test suite.

Data Custodian API behavior will be simulated separately using local Data Custodian stubs or mock HTTP services.

Conceptually:

```text
energy_ingest
   |       |
   |       +----------------------+
   |                              |
   v                              v
OAuth test server          Data Custodian stub
                                  |
                                  +-- Green Button/ESPI responses
                                  +-- pagination
                                  +-- revisions and gaps
                                  +-- HTTP/API errors
                                  +-- timeouts
                                  +-- malformed supported-format responses
```

The OAuth test server is responsible for standards-oriented OAuth behavior needed by supported flows, such as:

- authorization-code behavior;
- token issuance;
- access-token expiration;
- refresh-token issuance and rotation;
- scopes;
- client authentication;
- revocation or simulated revocation;
- and deterministic OAuth errors.

The Data Custodian stub is responsible for integration behavior such as:

- Green Button CMD endpoints;
- supported Green Button/ESPI resources;
- pagination;
- source publication metadata;
- revision and ordering behavior;
- rate limiting;
- temporary failures;
- authorization failures;
- malformed or truncated Green Button/ESPI payloads;
- missing intervals;
- and corrected intervals.

The Data Custodian stub must not introduce proprietary non-Green-Button formats into the supported ingestion contract. The Green Button/ESPI scope is defined by [ADR-0020](0020-standardized-testing-and-provider-test-data-policy.md).

Data Custodian-specific scenarios belong in Data Custodian-specific fixtures or stub configuration rather than in the generic OAuth server unless the behavior is genuinely part of OAuth.

Production code must not contain test-only branches solely to force these scenarios.

Tests use only synthetic or test-only credentials and data. Real Retail Customer or deployment credentials must not be committed to the repository.

Tests against a real Data Custodian sandbox, test environment, or authorized endpoint may be added as optional supplemental validation, but they are not required for ordinary deterministic test success.

The exact OAuth test server and HTTP mocking implementation are development-tool choices and may change as long as the required deterministic behavior remains available.

## Consequences

OAuth behavior can be tested without implementing an authorization server inside `energy_ingest`.

Data Custodian API behavior can evolve independently from OAuth test infrastructure.

OAuth failures remain distinguishable from Data Custodian API, Green Button parsing, and ingestion failures.

Required tests remain reproducible without production credentials or external-service availability.

Containerized test infrastructure adds development dependencies, but avoids reliance on proprietary hosted testing services.

A standards-compliant OAuth test server cannot guarantee that every Data Custodian behaves identically, so Data Custodian-specific stubs and optional live/test-environment validation remain useful.

Detailed test cases and fixture organization follow [ADR-0020](0020-standardized-testing-and-provider-test-data-policy.md) and development testing documentation.
