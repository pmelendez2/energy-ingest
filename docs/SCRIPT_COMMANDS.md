# Energy Usage Command-Line Operations

**Purpose:** Reference supported production administration scripts.

**Intended Audience:** Production operators.

The scripts locate `deploy/docker-compose` relative to their own file
locations. **The command examples below assume the project root as the
current working directory** because they start with `./scripts/`.
From another directory, invoke a script using its absolute path.
A host with Docker Engine and Docker Compose is required.

## Set up Energy Usage

```bash
./scripts/setup-prod.sh
./scripts/verify-prod.sh
```

The installation initializes a database without requiring XML downloads.
Select start/update for routine startup, refresh to recreate application
containers while retaining data, or the explicitly labeled reset/uninstall
options when you intend to remove data or resources.

## Register and import utility usage data

```bash
./scripts/register-prod.sh \
  --utility 'SDG&E' \
  --source 'Manual XML Download'

./scripts/import-prod.sh \
  --import-dir "$HOME/energy_usage/data" \
  --file '*.xml' --list-files

./scripts/import-prod.sh \
  --import-dir "$HOME/energy_usage/data" \
  --file '*.xml'
```

The importer selects the only registered acquisition source automatically.
With multiple sources, specify `--source-id` to disambiguate.
File download may happen before or after installation.

## Analyze and manage meters

```bash
./scripts/analyze-prod-data.sh
./scripts/analyze-prod-data.sh --details
./scripts/meter-prod.sh list
./scripts/meter-prod.sh attach-compatible --meter 1
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
These commands are a local administrative interface, not HTTP clients.
The separate authenticated API is not used by these scripts.

Credentials reside in Compose-mounted secret files, not these scripts.
The CLI migration/bootstrap operation remains an administrative operation
outside the ordinary application API.

## Developer verification

From the repository root, use the isolated test workflow:

```bash
./deploy/docker-compose/test.sh
```

The Compose wrapper creates and removes a temporary MariaDB test
environment. Running `./scripts/verify.sh` directly requires a working
test database and `ENGY_T_DB_URL`. Verification includes strict
Markdown documentation validation.
