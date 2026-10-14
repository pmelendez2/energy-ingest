<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0024: Local API Client Authentication

- Status: Accepted
- Date: 2026-09-30

## Context

`energy_ingest` exposes a local application API for Home Assistant and other trusted Home Lab clients.

The API should not assume that every process or device on the local network is automatically authorized.

The authentication model must be simple, machine-friendly, independently revocable per client, and separate from database and Data Custodian credentials.

## Decision

Application API access requires authentication unless an endpoint is explicitly documented as unauthenticated.

The initial architecture uses one credential per API client.

All normal API client credentials initially have the same application permissions.

The initial architecture does not define role-based access control.

Security-sensitive administrative operations remain outside the normal client API until a concrete requirement justifies exposing them there.

## Default Credential

The default API credential is an opaque bearer token containing 256 bits of cryptographically secure random material.

The random bytes are encoded using base64url without padding and prefixed with:

`ei_`

Clients send the token using the standard HTTP `Authorization: Bearer` mechanism.

The initial architecture does not require JWT, OAuth for local clients, session cookies, client certificates, or an external identity provider.

## Credential Storage

The plaintext token is shown only when created or deliberately replaced.

The server stores a one-way verifier rather than the recoverable plaintext credential.

The default verifier is SHA-256 of the complete issued token.

Because the token has high cryptographic entropy, password-specific slow hashing is not required for this credential type.

A stored client record may include non-secret metadata such as client identity, creation time, revocation/disabled state, and operational timestamps.

Exact table/column design is an implementation detail.

## One Credential per Client

Each API client receives its own token and tokens are not shared between clients.

The initial deployment may have only one client, such as Home Assistant, and therefore only one token.

Per-client credentials add modest configuration overhead but allow independent replacement, revocation, and client identification without changing unrelated clients.

## No Initial RBAC

Per-client identity is used for credential lifecycle and attribution, not for different permission sets initially.

All normal API clients receive the same normal API capabilities.

If a real use case later requires different application permissions, authorization roles/permissions should be introduced through a separate architectural decision.

## Administrative Boundary

Operations such as creating clients, issuing/replacing/revoking credentials, or managing Data Custodian authorization are initially handled through a separately trusted local administrative interface such as CLI tooling.

They are not exposed to ordinary API clients merely to avoid building RBAC prematurely.

## Credential-Domain Separation

Local API credentials are distinct from:

- MariaDB credentials;
- Data Custodian/OAuth credentials;
- SSH credentials;
- and unrelated Home Lab secrets.

Home Assistant receives only its `energy_ingest` API credential and consumes canonical data through the API rather than accessing MariaDB directly.

General credential policy follows [ADR-0018](0018-security-identities-credentials-and-secret-management-policy.md).

## Exposure and Transport

Raw API tokens must not appear in normal logs, URLs, ordinary API responses, or documentation.

Existing raw tokens are not retrievable through the normal application API; loss of a plaintext token requires replacement.

Authentication remains required even when the API is reachable only on the local network.

Network/firewall/reverse-proxy controls complement authentication rather than replace it.

Protected transport such as HTTPS should be used where the deployment boundary requires it; exact TLS and reverse-proxy configuration are deployment concerns.

## Public Endpoints

A small number of endpoints may intentionally remain unauthenticated when they expose no sensitive information, for example a minimal health endpoint.

Such endpoints must be explicitly documented and must not expose Energy Usage Information, Retail Customer information, credentials, or administrative state.

## Testing

Authentication behavior is tested according to [ADR-0020](0020-standardized-testing-and-provider-test-data-policy.md).

Detailed test matrices and provisioning commands belong outside this ADR.

## Consequences

The initial authentication architecture is explicit and simple.

Clients can be revoked independently without requiring RBAC.

The database does not store recoverable plaintext API tokens.

Each additional client requires one additional credential configuration.

If different client permissions become necessary later, a new authorization design will be required.
