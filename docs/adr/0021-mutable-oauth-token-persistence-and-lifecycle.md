<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0021: Mutable OAuth Token Persistence and Lifecycle

- Status: Accepted
- Date: 2026-09-30
- Updated: 2026-10-02

## Context

`energy_ingest` may acquire Green Button/ESPI data from Data Custodians using
OAuth-protected interfaces such as Green Button Connect My Data (CMD).

OAuth introduces mutable authorization state such as access tokens, refresh
tokens, expiration, granted scopes, authorization status, and related runtime
metadata.

This state differs from static deployment credentials such as an OAuth client
secret. Static deployment credentials remain governed by
[ADR-0018](0018-security-identities-credentials-and-secret-management-policy.md).

The initial project scope is residential and local-first.

A residential utility login may expose multiple Usage Points, meters, and
commodities through one authorization. For example, one authorization may
provide access to both electric and gas Usage Points.

Business, commercial, multi-property commercial, aggregator, and multi-tenant
authorization requirements are out of scope for the initial project.

The project should not add authorization complexity for those scenarios until
a demonstrated requirement exists.

## Decision

`energy_ingest` will treat OAuth authorization state as sensitive, mutable
application state.

For the initial residential scope:

- an Acquisition Source may have zero or one active OAuth authorization;
- one OAuth authorization may provide access to multiple Usage Points, meters,
  and commodities;
- OAuth authorization is an acquisition concern and is not part of canonical
  Usage Point identity;
- token refresh updates the existing authorization state;
- reauthorization replaces or updates the same logical authorization state;
- the project will not model multiple concurrent OAuth authorizations for one
  Acquisition Source;
- business, commercial, aggregator, and multi-tenant authorization models are
  out of scope.

The application component responsible for OAuth authorization owns:

- token storage;
- token refresh;
- token expiration handling;
- token replacement;
- revocation handling;
- and authorization lifecycle state.

Other components must not independently store or refresh OAuth tokens.

OAuth protocol behavior should be implemented using a maintained OAuth
implementation/library rather than project-specific protocol code.

## Relationship to Energy Data

OAuth authorization and energy-data identity are separate concerns.

The project must not assume:

- one OAuth authorization per Usage Point;
- one OAuth authorization per meter;
- one OAuth authorization per commodity;
- or one OAuth authorization per downloaded Green Button resource.

A single authorization may expose multiple Green Button resources.

The canonical energy model remains responsible for Usage Point, Meter Reading,
Reading Type, Interval Block, interval reading, revision, coverage, and gap
identity.

The OAuth model only governs access to a Data Custodian.

## Persistence

OAuth authorization state must be persisted when required to preserve valid
authorization across normal application or host restarts.

Persistent runtime token storage must use protected application-controlled
storage.

For the current architecture, mutable OAuth state may be stored through the
`energy-store` persistence boundary in MariaDB.

This decision does not require a separate OAuth database or external
credential-management platform.

The project must not create unmanaged ad hoc token files when application
persistence is already available.

If future deployment requirements justify a different protected persistence
mechanism, that change should be reviewed separately.

## Static and Mutable OAuth Credentials

Static OAuth configuration may include:

- client identifier;
- client secret;
- redirect URI;
- authorization endpoint;
- token endpoint;
- expected scopes;
- and other deployment configuration.

Static secrets follow
[ADR-0018](0018-security-identities-credentials-and-secret-management-policy.md).

Mutable OAuth authorization state may include:

- access token;
- refresh token;
- expiration;
- granted scopes;
- token type;
- provider authorization identifier;
- authorization status;
- authorization timestamp;
- last successful refresh;
- last successful use;
- revocation timestamp;
- and other non-secret lifecycle metadata required for operation.

Mutable state must not require editing static configuration each time a token
changes.

## Token Refresh

Where refresh is supported, refresh should occur before an access token becomes
unusable when expiration metadata is available.

Refresh must:

1. identify the current authorization state;
2. perform the refresh through the maintained OAuth implementation;
3. validate the response;
4. persist replacement token state;
5. update expiration and scope metadata as applicable;
6. stop using obsolete token state.

If a Data Custodian rotates refresh tokens, the replacement state must be
persisted atomically.

A failed refresh must not silently destroy the last known authorization state
before the failure is classified.

## Reauthorization

When authorization can no longer be refreshed or is revoked, the application
must transition to a state requiring reauthorization.

For the initial residential model, reauthorization updates or replaces the
existing logical authorization for the Acquisition Source rather than creating
another concurrent authorization relationship.

Previously imported energy data remains independent of OAuth authorization
lifecycle and must not be deleted merely because authorization is removed or
replaced.

## Revocation and Removal

OAuth authorization may be revoked by the Retail Customer, Data Custodian,
expiration policy, security action, or another provider-defined mechanism.

When revocation is definitive:

- normal acquisition using the authorization must stop;
- refresh attempts should stop when they cannot succeed;
- the authorization should be marked unusable;
- and reauthorization should be required when appropriate.

Local authorization removal and deletion of previously imported energy data are
separate operations.

## Storage Protection

OAuth token values must not be stored in:

- source control;
- committed configuration;
- command-line arguments;
- application logs;
- container images;
- or normal diagnostic output.

Database access must remain restricted to application components that require
it.

Normal API clients must not receive raw OAuth token values.

The project must not invent proprietary encryption or key-management schemes.

Application-level field encryption may be added later if the threat model
justifies it and established cryptographic libraries are used.

## Backup and Recovery

OAuth authorization state is sensitive mutable application state.

Backup policy must explicitly determine whether OAuth state is:

- included in protected backups;
- excluded and expected to require reauthorization after recovery;
- or protected separately.

Restored authorization state must be treated as potentially stale and validated
through normal OAuth behavior before being assumed usable.

Obsolete bearer credentials should not be retained indefinitely.

## Testing

OAuth lifecycle behavior must be tested according to
[ADR-0020](0020-standardized-testing-and-provider-test-data-policy.md).

Tests should cover, where applicable:

- initial authorization persistence;
- access-token expiration;
- successful refresh;
- refresh-token rotation;
- scope persistence;
- restart behavior;
- invalid or revoked authorization;
- reauthorization-required state;
- atomic replacement of mutable token state;
- secret redaction;
- and the invariant that one Acquisition Source has at most one active OAuth
  authorization in the initial residential model.

Tests must use synthetic or test-only OAuth credentials.

Real Retail Customer OAuth credentials must not be committed to fixtures.

## Operational Simplicity

The OAuth design must remain appropriate for a residential/local-first
deployment.

The project should avoid requiring:

- enterprise secret-management infrastructure;
- a dedicated OAuth credential database;
- external hosted token brokers;
- complex multi-tenant authorization models;
- commercial account/property hierarchies;
- or manual copying of refreshed tokens into configuration files.

Additional OAuth complexity should be introduced only when a supported use case
demonstrates the need.

## Consequences

### Positive

- The authorization model matches the initial residential use case.
- One authorization can expose multiple Usage Points and commodities.
- Token lifecycle ownership remains clear.
- Durable authorization can survive normal restarts.
- OAuth remains separated from canonical energy-data identity.
- Commercial and aggregator complexity is not introduced prematurely.

### Negative

- The initial model does not support multiple concurrent authorizations for one
  Acquisition Source.
- A future business, commercial, aggregator, or multi-tenant requirement may
  require a schema and ADR update.
- Durable OAuth state remains sensitive persistent data requiring careful
  protection.

### Neutral

- This ADR does not define the OAuth protocol itself.
- This ADR does not require every Data Custodian to support refresh tokens.
- This ADR does not require application-level encryption of every token.
- Exact table and column definitions remain governed by
  [ADR-0022](0022-canonical-relational-schema-revision-history-provenance-coverage-and-gaps.md).

## Alternatives Considered

### Multiple OAuth authorizations per Acquisition Source

The earlier design allowed multiple authorization relationships for one
Acquisition Source.

It was simplified because the initial project scope is residential and the
observed residential model allows one utility authorization to expose multiple
Usage Points and commodities.

Supporting hypothetical business, commercial, or aggregator scenarios would add
schema and lifecycle complexity without a current requirement.

### Store tokens only in memory

Not selected because normal application or host restarts could require
unnecessary reauthorization.

### Store refreshed tokens in static configuration

Not selected because runtime OAuth state is mutable application data and should
not require rewriting deployment configuration.

### Require an external secret-management platform

Not selected because it adds operational complexity that is not justified for
the initial residential/local-first deployment.

### Treat authorization removal as data deletion

Rejected because future acquisition authorization and retention of already
imported energy data are separate lifecycle concerns.
