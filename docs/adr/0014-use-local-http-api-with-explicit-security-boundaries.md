<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0014: Use a Local HTTP API with Explicit Security Boundaries

- Status: Accepted
- Date: 2026-09-29

## Context

`energy_ingest` needs a stable application boundary for Home Assistant and other local consumers.

Direct database access would expose the physical schema, distribute database credentials, and couple clients to persistence implementation details.

The API also needs a clear separation from Data Custodian authentication and from administrative operations.

## Decision

`energy_ingest` will expose a versioned local HTTP API as the normal integration boundary for application clients.

Clients such as Home Assistant will use the application API rather than access MariaDB directly.

The API may expose canonical application information such as:

- energy readings;
- coverage and gap state;
- ingestion status;
- and non-secret Data Custodian status.

The API must maintain explicit security boundaries:

- local API authentication is separate from Data Custodian OAuth;
- API clients do not receive database credentials;
- API clients do not receive Data Custodian access or refresh tokens;
- sensitive administrative operations are not implicitly granted to ordinary API clients;
- sensitive data is not intentionally exposed through health or diagnostic endpoints.

The API is intended primarily for local/private-network use.

Network placement, firewalling, and reverse-proxy controls provide defense in depth but do not replace application authentication.

Internet exposure is not required by the architecture.

The concrete local API authentication mechanism is defined by [ADR-0024](0024-local-api-client-authentication.md).

Data Custodian OAuth lifecycle follows [ADR-0021](0021-mutable-oauth-token-persistence-and-lifecycle.md).

The exact endpoint paths, response schemas, bind addresses, reverse-proxy configuration, health-check implementation, logging fields, and TLS deployment are API or operations documentation rather than ADR content.

## Consequences

Home Assistant and other clients depend on an application contract rather than the relational schema.

Database and Data Custodian credentials remain behind application boundaries.

The local API can evolve independently from MariaDB schema changes when the API contract is preserved.

Additional network security controls can be added without redefining the application boundary.
