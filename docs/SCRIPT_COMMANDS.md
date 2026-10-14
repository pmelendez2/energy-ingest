# Energy Usage Command-Line Operations

Use the commands in the project `scripts/` directory. They work from any
working directory; each locates `deploy/docker-compose` automatically.
Run each command from the unpacked Energy Usage distribution on a host
with Docker Engine and Docker Compose available.

## Set up Energy Usage

```bash
bash scripts/setup-prod.sh
bash scripts/verify-prod.sh
```

The installation initializes a database without requiring XML downloads.
Select start/update for routine startup, refresh to recreate application
containers while retaining data, or the explicitly labeled reset/uninstall
options when you intend to remove data or resources.

## Register and import utility usage data

```bash
bash scripts/register-prod.sh \
  --utility 'SDG&E' \
  --source 'Manual XML Download'

bash scripts/import-prod.sh \
  --import-dir "$HOME/energy_usage/data" \
  --file '*.xml' --list-files

bash scripts/import-prod.sh \
  --import-dir "$HOME/energy_usage/data" \
  --file '*.xml'
```

The importer selects the only registered acquisition source automatically.
With multiple sources, specify `--source-id` to disambiguate.
File download may happen before or after installation.

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
