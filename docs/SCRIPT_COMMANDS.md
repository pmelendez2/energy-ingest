# Energy Usage Command-Line Operations

**Purpose:** Document the supported Energy Ingest production
administration commands and guided XML import workflow.

**Intended Audience:** Customers, system administrators, and
operators managing an Energy Ingest installation.

Use the commands in the project `scripts/` directory. They work from any
working directory; each locates `deploy/docker-compose` automatically.
Run each command from the unpacked Energy Usage distribution on a host
with Docker Engine and Docker Compose available.

## Set up Energy Usage

```bash
"$HOME/energy-ingest-prod/scripts/setup-prod.sh"
"$HOME/energy-ingest-prod/scripts/verify-prod.sh"
```

The installation initializes a database without requiring XML downloads.
Select start/update for routine startup, refresh to recreate application
containers while retaining data, or the explicitly labeled reset/uninstall
options when you intend to remove data or resources.

## Guided registration and import (recommended)

**Keep the original XML exports in a private directory outside the Energy ingest installation.** Removing the installation directory also removes any XML files stored within it.  The guided importer preserves your original files. Register or choose a named source, and associate meters using one guided command:

```bash
"$HOME/energy-ingest-prod/scripts/energy-import.sh" \
  --import-dir "/absolute/path/to/your/utility-xml"
```

The script stages selected files in application-managed `private-imports`
after confirmation, but **does not sanitize** the XML. It never deletes the
customer's original downloads. Names are presented for source selection;
numeric IDs are internal and not required in the default onboarding flow.
Meter numbers are entered privately and sent to the Rust backend on stdin.

`energy-import.sh` is the only supported customer XML import command.
Its internal import job is not a separate user-facing workflow.
`register-prod.sh` and `meter-prod.sh` remain administrative helpers.

## Guided import options and results

Use `--file 'name.xml'` to select a single XML file or a quoted pattern
such as `--file 'electric-*.xml'` to select a subset. The default is
`*.xml`. The guided importer normally chooses or registers a utility
source interactively. Advanced operators can specify an existing source
with `--source-id ID`; review the utility and source names displayed
before approving the import. Source IDs do not identify physical meters.

The import report distinguishes **Observed**, **Unchanged**, and
**Accepted** readings. Importing an unchanged file again should report
unchanged observations, not duplicate stored usage. If an import fails
partway through multiple files, earlier files may have been saved.
Correct the issue and retry safely without erasing the database.

When two physical meters have the same commodity, guided import must
not guess the association. Review the resource-to-meter assignments
with the administrative meter tools and confirm each belongs to the
selected meter. Missing observations remain missing.

## Analyze and manage meters

```bash
bash scripts/analyze-prod-data.sh
bash scripts/analyze-prod-data.sh --details
bash scripts/meter-prod.sh list
bash scripts/meter-prod.sh attach-compatible --meter 1
```

Meter profile and Green Button resource IDs are internal identifiers.
Bulk association previews eligible resources before asking for approval.
The user must verify that the selected resources belong to the same
physical meter. Imported readings are not merged by association.

## Runtime boundaries

The shell commands call local Docker Compose jobs, which invoke Rust
administrative and import executables. They do not call the authenticated
Energy Usage HTTP API. Container entrypoints and Compose descriptors stay
in `docker/` and `deploy/docker-compose/`, respectively.

The accepted ADR-0012 specifies that the CLI and local API share reusable
application logic; they must not implement different business rules.
Until an authenticated API is implemented, these commands remain a local
administrative interface rather than an HTTP client.

Credentials reside in Compose-mounted secret files, not these scripts.
The CLI migration/bootstrap operation remains an administrative operation
outside the ordinary application API.
