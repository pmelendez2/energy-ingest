<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0007: Use OAuth 2.0 as Configurable Data Custodian Authentication

- Status: Accepted
- Date: 2026-09-30

## Context

Retail Customer Energy Usage Information may be protected by Data Custodian authentication and authorization requirements.

Green Button acquisition commonly occurs in two different ways:

- with Download My Data (DMD), the Retail Customer authenticates to the Data Custodian and obtains a Green Button file for later local processing;
- with Connect My Data (CMD), the Retail Customer authorizes a Third Party to retrieve Green Button/ESPI data from the Data Custodian, commonly through OAuth-based authorization.

These stages must remain distinct from subsequent parsing and normalization of an already acquired Green Button source.

Data Custodians may differ in OAuth details such as:

- authorization and token endpoints;
- scopes;
- client authentication;
- token lifetime;
- refresh-token behavior;
- redirect URI requirements;
- revocation behavior;
- and profile-specific parameters.

Authentication and authorization are acquisition concerns and must not be embedded in the Green Button parser, canonical energy model, persistence consumers, or Home Assistant integration.

## Decision

The Data Custodian acquisition/authentication boundary will support OAuth 2.0 as a configurable authentication and authorization mechanism for integrations that require it.

OAuth support will be centralized and reusable rather than independently reimplemented by each Data Custodian adapter.

`energy_ingest` should rely on maintained open-source OAuth implementations for standards-level OAuth mechanics where suitable, rather than implementing the protocol from scratch.

Data Custodian adapters may provide integration-specific configuration or behavior needed to use that common OAuth capability.

Static OAuth configuration may include:

- client identifier;
- client secret where required;
- authorization endpoint;
- token endpoint;
- redirect URI;
- requested scopes;
- and Data Custodian-specific OAuth parameters.

Static credentials follow [ADR-0018](0018-security-identities-credentials-and-secret-management-policy.md).

Mutable authorization state such as access tokens, refresh tokens, expiration, granted scopes, refresh behavior, persistence, and revocation follows [ADR-0021](0021-mutable-oauth-token-persistence-and-lifecycle.md).

The Green Button/ESPI parser has no responsibility for OAuth processing.

Home Assistant and other local API clients do not manage or receive Data Custodian OAuth credentials.

Loss, expiration, or revocation of Data Custodian authorization stops protected future acquisition that requires that authorization, but does not automatically delete previously imported data.

Retention of previously acquired data is governed by [ADR-0009](0009-separate-data-retention-from-provider-authorization.md) and backup/retention behavior by [ADR-0023](0023-source-data-retention-backup-export-and-restore.md).

A locally available Green Button file may continue to be processed without another Data Custodian login when local processing is otherwise permitted.

The architecture does not assume that every supported acquisition mechanism uses OAuth. Another authentication mechanism may be supported through the authentication/acquisition boundary if a concrete Data Custodian requires it, while the Green Button/ESPI ingestion boundary remains unchanged.

## Consequences

OAuth-capable Data Custodian integrations share common authorization and lifecycle behavior.

Data Custodian adapters remain focused on integration-specific acquisition rather than implementing generic OAuth protocol mechanics.

Static secrets and mutable OAuth state have separate ownership and lifecycle rules.

The ingestion core, Green Button parser, canonical persistence model, and Home Assistant integration remain independent of Data Custodian authentication details.

Revoking Data Custodian authorization can stop future retrieval without destroying historical local data.

OAuth introduces lifecycle and security complexity, including refresh, expiration, revocation, and reauthorization, but those details are centralized by [ADR-0021](0021-mutable-oauth-token-persistence-and-lifecycle.md).

Deterministic OAuth testing follows [ADR-0015](0015-use-mocked-oauth-provider-for-authentication-testing.md) and the overall testing policy in [ADR-0020](0020-standardized-testing-and-provider-test-data-policy.md).
