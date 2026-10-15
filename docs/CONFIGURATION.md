<!-- SPDX-License-Identifier: MIT -->

# Energy Usage Configuration

## Purpose

This guide describes configuration for the Energy Usage deployment
with its bundled MariaDB database. For installation and startup,
see [Installation](INSTALLATION.md). For database maintenance,
see [Production Database](PRODUCTION_DATABASE.md).

The standard setup creates the necessary configuration and credentials.
You do not need to install Rust, create database accounts manually,
or export database connection strings in your shell.

**Intended Audience:** Users and operators configuring an Energy Usage installation.

## Prerequisites

- Docker Engine and Docker Compose v2.
- Storage for the database's persistent Docker volume.
- A private location for utility XML files you intend to import.

Run the setup workflow from the deployment directory:

```bash
./scripts/setup-prod.sh
```

The setup workflow preserves existing configuration and database data
unless you explicitly choose a destructive operation. Review its
confirmation prompts before proceeding.

## Settings managed by setup

The setup workflow creates `.env.prod` from `.env.prod.example` when
needed. Docker Compose reads this file; the values are not commands
to enter in your shell.

| Setting | Purpose | Example | Normal user action |
| --- | --- | --- | --- |
| `ENGY_APP_IMAGE` | Production application image | `docker.io/pmelendez2/gb-energy-ingest:VERSION` | Keep at a verified release |
| `ENGY_DB_IMAGE` | MariaDB image | `mariadb:lts` | None |
| `ENGY_DB_NAME` | Database name | `energy_ingest` | None |
| `ENGY_DB_USER` | Database application account | `energy_ingest` | None |
| `ENGY_DB_PWD_FILE` | Application password file path | `./prod-secrets/energy_db_pwd` | None |
| `ENGY_DB_ROOT_PWD_FILE` | Database initialization password file path | `./prod-secrets/energy_db_root_pwd` | None |

The examples describe the bundled database setup. To inspect the
actual values on your installation, open `.env.prod` in the deployment
directory. **Do not share secret files or credentials.**

## Database credentials

Setup generates separate credentials for database initialization and
normal application access. Passwords are kept in files under
`prod-secrets/`, with restricted filesystem permissions. The Compose
services mount only the credentials needed for their responsibilities.

The following settings are created internally by the container
entrypoints; they are **not** configuration fields for a standard user:

| Internal setting | Purpose |
| --- | --- |
| `ENGY_DB_ADMIN_URL` | Privileged connection for schema initialization |
| `ENGY_DB_URL` | Application connection for registration and imports |

Do not paste full database URLs, passwords, or provider account
information into issue reports or committed files.

## Persistent storage

MariaDB data is stored in the `energy-db-data` Docker named volume,
managed by the Compose deployment. Restarting containers or repeating
the normal setup continuation does not intentionally clear that volume.

Keep a backup of data you wish to preserve before performing database
maintenance, replacing storage, or selecting a destructive setup option.
See [Production Database](PRODUCTION_DATABASE.md) for operational details.

## Utility XML files

The default private input directory is:

```text
deploy/docker-compose/private-imports/
```

You may choose another directory on the host. To configure a default,
add the following line to `.env.prod`:

```dotenv
ENGY_IMPORT_DIR=/home/user/energy-xml
```

The path must exist and be accessible to Docker. You may also choose
a directory for an individual import using `--import-dir`:

```bash
./scripts/import-prod.sh --source-id 42 \
    --import-dir /home/user/energy-xml \
    --file usage-export.xml
```

The import helper mounts the selected directory read-only in the
container. Multiple XML files may share a directory, and imports do not
delete the original downloads. Run `./scripts/import-prod.sh --help` for
command-line options. Paths on Windows are described in
[Windows installation](INSTALL_WINDOWS.md).

## Troubleshooting configuration

If setup cannot start the database, check the Compose service status
and health before modifying credentials or data:

```bash
cd deploy/docker-compose
docker compose -p energy-ingest \
  --env-file .env.prod \
  -f compose.prod.yml ps
```

Use [Installation](INSTALLATION.md) for setup troubleshooting and
[Production Database](PRODUCTION_DATABASE.md) for database operations.

## HTTP API development documentation

The read-only API has an optional interactive OpenAPI reference. It is
**disabled by default**. Developers running the API directly may set
`ENGY_API_DEV_DOCS=1` only when `ENGY_API_LISTEN` is a loopback address.

When enabled, the developer routes are:

- `/api-docs/openapi.json` — generated OpenAPI 3 specification.
- `/swagger-ui/` — interactive reference and request explorer.

Do not enable the interactive explorer on network-facing deployments.
Ordinary API calls still require bearer authentication; see
[API Client Administration](API_CLIENT_ADMIN.md).

The supported application routes and schemas are defined by the Rust
handlers and the generated OpenAPI specification, rather than by a
separate manually maintained HTTP endpoint document.
