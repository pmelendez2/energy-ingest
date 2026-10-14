<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0012: Use CLI-first administration with a stable local API

- Status: Accepted
- Date: 2026-09-29

## Context

`energy_ingest` needs an operational interface for:

- configuring Data Custodian integrations;
- importing local files;
- triggering Data Custodian acquisition;
- checking coverage and gaps;
- viewing import results;
- inspecting Data Custodian authorization state;
- diagnosing failures;
- administering retained data.

The initial users of the project are expected to be comfortable with command-line
tools and declarative configuration.

A command-line interface is therefore a practical first administration surface.

However, placing business logic directly inside CLI commands would make future
interfaces harder to add.

Possible future interfaces include:

- a local web administration UI;
- Home Assistant configuration or status views;
- automation through scripts;
- external management tools;
- other local applications.

The Home Assistant Energy Dashboard is intended to provide energy visualization.
`energy_ingest` should not duplicate that visualization functionality merely to
provide administration.

The project therefore needs a stable separation between:

- core ingestion and Data Custodian integration logic;
- configuration;
- administrative interfaces;
- downstream consumers.

## Decision

`energy_ingest` will use a CLI-first administration model.

The CLI will be a client of shared application and service logic rather than the
owner of that logic.

Core behavior such as:

- Data Custodian acquisition;
- Green Button parsing;
- normalization;
- revision handling;
- gap detection;
- persistence;
- authorization lifecycle;
- retention operations;

will be implemented in reusable libraries or service components.

The project will support declarative configuration using a documented local
configuration format.

Configuration will describe non-secret settings such as:

- Data Custodian integration type;
- source locations;
- API endpoints where applicable;
- acquisition behavior;
- storage settings;
- local service settings.

Secrets will remain outside ordinary checked-in configuration and will follow
the authentication and secret-handling decisions defined elsewhere.

The application will expose a stable local API for administrative and downstream
integration.

The API should support operations needed by clients such as:

- health and readiness checks;
- Data Custodian status;
- import requests;
- import summaries;
- coverage and gap status;
- access to normalized energy data;
- retention and purge operations when implemented;
- diagnostics.

The exact transport and API schema may evolve during implementation, but the
architecture will preserve the separation between clients and core business
logic.

The CLI will use the same underlying application or API contracts available to
other clients where practical.

A future administration UI may be added without moving business logic into that
UI.

The project will not build a separate energy-visualization dashboard as part of
the initial administration interface.

Energy visualization is expected to be handled by downstream systems such as
Home Assistant.

## Consequences

The project can provide useful administration early without requiring a web UI.

Command-line workflows remain scriptable and suitable for automated local
operations.

Business logic can be tested independently from the CLI.

A future web interface can be added without rewriting Data Custodian integration, parsing, storage,
or retention logic.

Home Assistant can integrate through a defined local interface rather than
depending on internal database structures.

The project will need to maintain compatibility expectations for its local API.

Configuration format changes may require migration or backward-compatibility
handling.

The CLI and future UI must present consistent interpretations of Data Custodian state,
import results, gaps, revisions, and retention behavior.

The local API increases the need for explicit security boundaries.

The implementation must decide:

- which network interfaces the service listens on;
- whether authentication is required for local API access;
- how container networking is configured;
- how API compatibility is versioned.

Those security and protocol details may require separate architecture decisions.

Tests must verify that:

- CLI operations use shared application logic;
- core functionality can be exercised without invoking the CLI;
- API and CLI results remain consistent;
- Home Assistant can consume the supported local interface without direct access
  to the ingestion database.
