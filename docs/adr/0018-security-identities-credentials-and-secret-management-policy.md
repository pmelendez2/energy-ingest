<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0018: Security, Identities, Credentials, and Secret Management Policy

- Status: Accepted
- Date: 2026-09-30

## Context

`energy_ingest` uses several credential domains, including Data Custodian authentication, database access, local API authentication, administration, and backup/recovery.

These credentials have different owners and lifecycles and must not become interchangeable merely for convenience.

The security model must remain practical for a residential Home Lab rather than requiring enterprise secret-management infrastructure by default.

## Decision

The project will follow these security invariants:

- secrets are not committed to source control;
- credentials are provided only to components that need them;
- normal runtime identities use minimum practical privilege;
- administrative credentials are not used for routine runtime access;
- distinct credential domains do not share credentials merely for convenience;
- mutable runtime authorization state is separate from static deployment secrets;
- secrets are not exposed through logs, diagnostics, normal APIs, or documentation;
- real credentials are separated from test credentials;
- and credentials can be replaced or revoked according to their underlying mechanism.

Project terminology follows `docs/GLOSSARY.md`.

## Credential Categories

The architecture distinguishes at least:

- static deployment credentials;
- mutable runtime credentials;
- administrative credentials;
- application client credentials.

Examples include MariaDB credentials, OAuth client secrets, OAuth access/refresh tokens, local API credentials, encryption keys where used, and backup credentials.

Each credential must have a clear owner and consumer.

## Credential-Domain Separation

Data Custodian authentication, database authentication, local API authentication, administrative access, and backup/recovery are separate credential domains.

For example, Home Assistant receives only its local API credential and must not receive MariaDB or Data Custodian credentials.

## Secret Storage

Secrets must not be stored in:

- source code;
- Git-tracked configuration containing real values;
- public fixtures;
- container images;
- or ordinary documentation.

A deployment may use protected secret files, orchestration secrets, protected environment injection, or another suitable mechanism.

No single secret-storage product is mandatory.

Mutable secrets that change during normal operation must be stored through the subsystem that owns their lifecycle rather than by rewriting static deployment configuration.

## Exposure and Logging

Secret values must not appear in normal or debug logs, ordinary error responses, normal application APIs, source-control history, public test artifacts, or routine command output.

Where diagnostics are needed, secrets should be omitted or redacted while retaining non-secret identifiers and status information.

## Test and Development Credentials

Tests must not depend on real deployment or Retail Customer credentials.

Test-only credentials may be used in isolated test environments.

Test-data policy follows [ADR-0020](0020-standardized-testing-and-provider-test-data-policy.md).

## Rotation, Revocation, and Recovery

The project does not require automated rotation for every credential.

Rotation and replacement behavior should follow the risk and capabilities of the credential type.

The underlying system must support replacement or revocation where practical.

Backup/recovery policy must distinguish credentials that are preserved, regenerated, reissued, or intentionally excluded.

Detailed procedures belong in operations documentation.

## Encryption

The project does not require application-level encryption of every secret by default.

If application-level encryption is introduced, established cryptographic libraries and separately protected keys must be used.

The project must not invent custom cryptography.

## External Secret Management

External systems such as Vault may be integrated when useful, but are not required by the initial architecture.

## Related Boundaries

Mutable OAuth lifecycle is defined by [ADR-0021](0021-mutable-oauth-token-persistence-and-lifecycle.md).

Local API credentials are defined by [ADR-0024](0024-local-api-client-authentication.md).

Backup/recovery treatment is defined by [ADR-0023](0023-source-data-retention-backup-export-and-restore.md).

## Consequences

Credential ownership and boundaries are explicit.

The design avoids one shared credential spanning unrelated services.

The Home Lab deployment remains compatible with simple protected secret mechanisms.

The maintainer must document and manage more than one credential domain, but only where separation provides meaningful security value.
