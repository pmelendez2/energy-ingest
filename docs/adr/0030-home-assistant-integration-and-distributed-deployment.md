<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-15. -->

# ADR-0030: Home Assistant Integration and Distributed Deployment

- Status: Accepted
- Date: 2026-10-15

## Context

Energy Ingest holds revision-aware Green Button usage data and approved
physical-meter resource mappings. Customers need configuration, historical
usage, and import status in Home Assistant. Production deployments may place
Home Assistant, the Energy Ingest API, and MariaDB on different hosts.

The current target validation installation is Home Assistant Container
Core 2026.10.1, Frontend 20260930.3. This target may change in the
future and does not establish minimum supported Home Assistant versions.

## Decision

### Component responsibility

The `ha_energy_usage` repository (`energy_usage` domain) shall use
Home Assistant's Python integration framework for config entries, entities, coordination, and
Recorder statistics. Green Button processing, source registration, physical
meter relationships, revision handling, and stream resolution belong in
Rust `energy_ingest`. Home Assistant shall not directly access MariaDB.

### Two-stage installation

Stage 1 deploys Energy Ingest independently, with managed MariaDB
or a supported existing database. Home Assistant Container does not
provide a Supervisor-based database installation service. The
standalone installer remains available when Home Assistant is absent.

Stage 2 installs `ha_energy_usage` through HACS and configures the
Energy Ingest API connection in Home Assistant Devices & Services.
The integration must explain when Stage 1 is incomplete. HACS only
installs or updates integration code; configuration is performed
through Home Assistant config entries.

Database connection parameters for an existing local or remote
MariaDB server may be configured in a later Home Assistant-guided
administrative setup using a separately authorized Rust setup API.
This is not a direct Home Assistant-to-MariaDB connection, and it
is not a requirement that HACS provision MariaDB.

### Deployment topology

Home Assistant connects to an authenticated Energy Ingest HTTP(S) API.
Energy Ingest connects to MariaDB using privately configured credentials.
All three services may reside on separate hosts. The remote database mode
requires explicit configuration, limited database privileges, and secure
transport; it is not automatically supported by existing Compose defaults.

### Customer experience

Home Assistant shall use native config entries and provide UI-based
setup, reauthentication on token failure, and reconfiguration of the
API endpoint. Changing the endpoint to a *different* service identity
must not silently redirect an existing meter/device configuration.
The initial URL-based unique identity requires a stable server
identifier in a future API contract for safe host migration.


Use Home Assistant config flows for API endpoint and token validation.
Support reauthentication and reconfiguration, avoiding delete/reinstall.
Expose physical meters as devices with appropriate entities. HACS is the
intended integration distribution path; public listing is a separate
release requirement.

### Historical statistics

Home Assistant's Recorder statistics interface is the intended historical
backfill mechanism. Implement bounded, restart-safe synchronization that
preserves authoritative meter identity and accepted source revisions.
Never sum overlapping Green Button exports. Resolve issues in Rust before
emitting statistics. Handle corrected readings and incomplete coverage
explicitly; do not manufacture zero consumption for missing intervals.

Validate commodity-specific units, timestamp conventions, and meter
semantics. Solar/export readings require separate analysis and are not
assumed to behave like consumption. Native Energy Dashboard compatibility
must not require a custom Lovelace card.

Energy Ingest provides reliable, revision-aware historical usage through
its authenticated API. The Home Assistant integration translates this
data into Home Assistant statistics and handles historical corrections
without duplicating consumption. Specific synchronization algorithms
and endpoint designs belong in implementation specifications and tests.

### Quality and testing

Follow Home Assistant Integration Quality Scale guidance. Maintain unit
and config-flow tests; verify sensor/statistics semantics against the
supported Core release. Test API auth failures, connectivity, restart,
revisions, duplicate imports, timezone boundaries, and data gaps.

## Rationale and official references

- [Config flow architecture](https://developers.home-assistant.io/docs/core/integration/config_flow/)
- [Reconfigure flow](https://developers.home-assistant.io/docs/core/integration-quality-scale/rules/reconfiguration-flow/)
- [Integration Quality Scale](https://developers.home-assistant.io/docs/core/integration-quality-scale/)
- [Configuration flow](https://developers.home-assistant.io/docs/core/integration-quality-scale/rules/config-flow/)
- [Test before configure](https://developers.home-assistant.io/docs/core/integration-quality-scale/rules/test-before-configure/)
- [Reauthentication](https://developers.home-assistant.io/docs/core/integration-quality-scale/rules/reauthentication-flow/)
- [Reconfiguration](https://developers.home-assistant.io/docs/core/integration-quality-scale/rules/reconfiguration-flow/)
- [Recorder statistics API changes](https://developers.home-assistant.io/blog/2025/10/16/recorder-statistics-api-changes/)
- [Sensor entities](https://developers.home-assistant.io/docs/core/entity/sensor/)

### Administrative boundaries

Database bootstrap, migration, secret management, and remote TLS
validation remain in Rust. Administrative setup requires a distinct
authorization boundary, explicit confirmation, and credential
lifecycle handling. Ordinary API bearer tokens must not authorize
database administration or arbitrary host provisioning.

## Consequences

Two independently released repositories and compatible API contracts must
be maintained. The Python layer remains thin. Home Assistant may guide configuration of an existing local or remote
database through a future separately
authorized Energy Ingest administration API, without connecting directly
to MariaDB or storing database administrator credentials.
Initial releases may expose diagnostics without enabling history import
until the API and Recorder contracts have been verified.

## Related Decisions

- [ADR-0018 — Security, Identities, Credentials, and Secret Management](0018-security-identities-credentials-and-secret-management-policy.md)
- [ADR-0019 — Development Environment and Isolation](0019-development-environment-source-control-and-ai-assisted-development-policy.md)
- [ADR-0024 — Local API Client Authentication](0024-local-api-client-authentication.md)
- [ADR-0029 — Customer-First Installation and Home Assistant Experience](0029-customer-first-installation-and-home-assistant-experience.md)
