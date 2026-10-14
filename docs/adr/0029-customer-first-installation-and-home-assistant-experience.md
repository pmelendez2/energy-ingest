<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-12. -->

# ADR-0029: Customer-First Installation and Home Assistant Experience

- Status: Accepted
- Date: 2026-10-10

## Context

`energy_ingest` acquires and maintains canonical electricity and natural-gas
usage records from customer-authorized sources. `ha_energy_usage` makes those
records useful within Home Assistant. Customers seek usable energy information,
not a database administration or software development experience.

The expected customer may already use Home Assistant but have no familiarity
with Rust, database schemas, source repositories, or internal identifiers.
The product therefore needs an explicit customer-facing architectural boundary
between installing the service, providing source data, and consuming readings.

ADR-0019 defines the independence of production, development, and test
environments. This ADR defines the customer experience that production must
support; it does not prescribe a user-interface framework, component layout,
or project implementation plan.

## Decision

### Customer journey

The reference journey is **Install -> Connect -> Import -> View**:

1. Install and initialize Energy Usage using a supported distribution.
2. Connect Home Assistant to Energy Usage through an authenticated local
   interface.
3. Import customer-authorized usage data from an EDC and/or CCA.
4. View supported electricity and natural-gas usage in Home Assistant.

Connection and data import may occur in either order. An installation with no
readings remains valid and healthy and must report an understandable empty-data
state.

### Default installation and initial data state

The first usable release must provide a readily obtainable production
deployment with an **empty, initialized Energy Usage database**. Empty means a
usable schema and required non-customer metadata, but no imported customer
readings or customer account records. This capability is a first-release
prerequisite, not an optional development utility.

The default installation must establish required persistent storage and
application configuration with minimal user input. Repeated installation,
restart, or upgrade must protect existing customer data. It must not silently
reset storage, rotate credentials, or replace user configuration. Destructive
operations require informed, explicit consent.

The distribution may use multiple cooperating services while presenting a
single supported installation experience. Production operation does not
require a source checkout, compilation, development tooling, a test database,
or manual schema administration.

An externally managed database is an advanced deployment option. The
application must respect its ownership boundaries and never remove or reset
that external database as part of ordinary application lifecycle operations.

### Configuration and customer-facing identity

A normal customer must not need to enter internal acquisition-source IDs,
database connection strings, or implementation-specific identifiers. The
application supplies secure defaults, clearly distinguishes required user
choices from managed settings, and provides understandable confirmation of
significant configuration changes.

Advanced deployment configuration must not change the ingestion model or
complicate the ordinary customer journey. Production secrets and authorized
customer data remain separate from development and test material.

### Customer-controlled import

The initial release must offer a straightforward customer-facing way to select
and import one or more authorized Green Button XML files without using an
internal administrative CLI. A separate directory for each download is not
required. Filenames and directory structures must not determine meter identity
or acceptance of a reading.

Import must validate supported data and relationships, preserve provenance,
handle repeated and overlapping data, retain legitimate correction history,
and provide actionable, privacy-conscious feedback. Failed imports must not
commit partial changes to the affected import unit.

Retention of uploaded originals requires a defined customer-visible privacy
policy. Retrieval from an EDC or CCA on a schedule is an optional,
provider-dependent capability and is not a prerequisite for manual import.

### Home Assistant integration boundary

Home Assistant communicates with Energy Usage through an authenticated local
application interface, never through direct database access. Energy Usage
owns ingestion, canonical readings, provenance, database access, and any
provider authorization or retrieval scheduling. Home Assistant owns the
presentation of entities, statistics, and dashboards.

The integration must support a successfully connected application that has no
imported readings yet. Provider credentials must not be required by Home
Assistant merely to display previously imported energy data.

### First-release architectural boundary

A usable first release is defined by the complete customer journey: install
an empty, initialized service; connect Home Assistant securely; import
customer-authorized XML through a customer-facing interface; and display
supported readings. These are product acceptance capabilities, not an ordered
engineering task list.

Advanced database topology and automated provider retrieval may evolve
independently without blocking that default journey or requiring a second
application architecture.

## Consequences

- The first release must provide a genuinely usable initialized application,
  rather than source code and database setup instructions alone.
- The customer-facing import and local API are core product boundaries.
- Installation and upgrade behavior must preserve customer data and secrets.
- Ordinary usage must not depend on development or test infrastructure.
- Advanced infrastructure choices must preserve the same application behavior.
- Automated provider access remains distinct from manual XML import and
  must not be represented as universally available.

## Alternatives considered

- **Require manual database and source registration:** rejected for the normal
  customer journey because it exposes internal implementation details.
- **Require development infrastructure:** rejected under ADR-0019.
- **Distribute customer-populated database state:** rejected because customers
  must begin with private, independently managed data.
- **Require distinct folders for each XML download:** rejected because import
  identity must be based on source provenance and content, not file layout.
- **Connect Home Assistant directly to the database:** rejected because it
  exposes database access and couples the integration to internal schema.
- **Make automatic provider retrieval mandatory:** rejected because provider
  authorization and capabilities vary; manual import must work independently.

## Related decisions

- ADR-0019: separation of production, development, and test environments.
- ADR-0016: canonical storage and database ownership.
- ADR-0018: identities, credentials, and secrets.
- ADR-0024: local API client authentication.

Container images, orchestration choices, commands, configuration variables,
release tasks, and troubleshooting procedures belong in implementation and
operational documentation rather than this decision record.
