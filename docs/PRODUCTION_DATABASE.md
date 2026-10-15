<!-- SPDX-License-Identifier: MIT -->

# Production Database Deployment

## Purpose

**Intended Audience:** Production operators and maintainers.


This guide explains how to operate the Energy Usage MariaDB deployment,
including installation, database initialization, credentials, storage,
maintenance, and access from the application services.

For initial setup, follow [Installation](INSTALLATION.md). For configuration
values and credential locations, see [Configuration](CONFIGURATION.md).

## Database initialization and XML ingestion

`setup-prod.sh` starts MariaDB and executes the `migrate` job after the
database becomes healthy. The migration job initializes or updates the
database schema. The `import` service reads XML from a private, read-only
input mount and uses the restricted application credential. Database and
maintenance services do not publish network ports.

For the commands to register sources and import files, see
[Installation](INSTALLATION.md).

## Production Files

The deployment files include:

```text
docker/Dockerfile.prod
docker/es-adm-prod.sh
deploy/docker-compose/compose.prod.yml
deploy/docker-compose/.env.prod.example
scripts/setup-prod.sh
scripts/verify-prod.sh
deploy/docker-compose/bootstrap-prod.sh
deploy/docker-compose/prod-secrets/.gitignore
deploy/docker-compose/10-runtime-grants.sh
```

Local files created by `setup-prod.sh` include:

```text
deploy/docker-compose/.env.prod
deploy/docker-compose/prod-secrets/energy_db_pwd
deploy/docker-compose/prod-secrets/energy_db_root_pwd
```

The local configuration and credential files remain excluded from source
control.

## Required Host Software

The host requires:

- Docker Engine; and
- Docker Compose v2.

No host MariaDB or Rust installation is required for this deployment.

## Configuration

The tracked template is:

```text
.env.prod.example
```

The configuration template uses:

```text
ENGY_DB_IMAGE=mariadb:lts
ENGY_DB_NAME=energy_ingest
ENGY_DB_USER=energy_ingest
ENGY_DB_PWD_FILE=./prod-secrets/energy_db_pwd
ENGY_DB_ROOT_PWD_FILE=./prod-secrets/energy_db_root_pwd
```

Use a tested, maintained MariaDB image version for your deployment.

## Secrets

The deployment uses two file-backed Compose secrets:

```text
energy_db_pwd
energy_db_root_pwd
```

The normal runtime identity uses `energy_db_pwd`.

Administrative maintenance uses `energy_db_root_pwd`.

The normal runtime database account remains limited to:

```text
SELECT
INSERT
UPDATE
DELETE
```

Schema creation and migrations remain administrative operations.

## Persistent Storage

The production database uses:

```text
energy-ingest_energy-db-data
```

mounted at:

```text
/var/lib/mysql
```

Do not select database recreation or removal unless you intend to remove
stored usage history. Back up important data before maintenance.

Persistent storage alone is not a backup.

## Network Exposure

MariaDB joins the internal Compose network:

```text
energy-db
```

Port `3306` is not published to the host.

Production application and maintenance containers join the same internal
network and connect to:

```text
mariadb:3306
```

## Production Lifecycle

Run:

```bash
cd deploy/docker-compose
./setup-prod.sh
```

The setup menu includes:

```text
1) Continue/start production environment
2) Refresh production containers
3) Recreate production database
4) Remove production environment
5) Cancel
```

## Verification

Run:

```bash
./scripts/verify-prod.sh
```

Run from the repository or installation root.

It reports settings, credential presence, database volume state, MariaDB
container state, and health without displaying credential values.

## Published production image

The production Compose template selects the released multi-platform
image `docker.io/pmelendez2/gb-energy-ingest:0.1.0` using
`ENGY_APP_IMAGE`. It is published for AMD64 and ARM64 Linux containers;
production users do not need to build the Rust image locally.
The full release checks, native Pi 5 smoke-test scope, and digest are
recorded in [release validation](RELEASE_0_1_0_VALIDATION.md).

## Production Administrative Image

Administrative schema work uses a production-only image built from:

```text
docker/Dockerfile.prod
```

The image contains the release build of:

```text
energy-store-admin
```

It does not depend on `docker/Dockerfile.dev` or any test artifact.

The production entrypoint is:

```text
docker/es-adm-prod.sh
```

The entrypoint reads the administrative database credential from the production
secret, constructs `ENGY_DB_ADMIN_URL` inside the container, and executes
`energy-store-admin`.

The setup generates hexadecimal database credentials.
The entrypoint rejects a non-hex administrative credential instead of placing
an unescaped value into the database URL.

## Schema Bootstrap

Run:

```bash
./bootstrap-prod.sh
```

The wrapper:

1. verifies Docker and Compose are available;
2. verifies `.env.prod` and the administrative credential exist;
3. verifies production MariaDB is healthy;
4. builds the production administrative image if required;
5. starts a temporary `migrate` container on the internal database network;
6. runs `energy-store-admin database bootstrap`; and
7. removes the temporary container when the command exits.

The `admin` Compose service is reserved for maintenance and registration;
it is not started as a long-running application service.

You can rerun bootstrap when needed. The schema initialization workflow
is designed to preserve existing application data. Review errors before
attempting manual database changes.

## Backup and recovery

The database volume preserves data across normal container restarts, but
it is not a backup. Maintain a backup and recovery procedure suited to
your storage environment before relying on the service for long-term
history. Preserve any source XML you need for recovery.

Do not delete the database volume or rotate database credentials as a
recovery attempt. Database restoration and credential changes require
an explicit, reviewed procedure.

## Source registration and privilege separation

The operator can run `register-prod.sh` after database bootstrap. It
registers a utility and source without requiring SQL or database secrets
on the command line. The `admin` Compose job mounts only restricted runtime
credentials for registration. The separate `migrate` service mounts the
privileged migration credential for schema bootstrap. Do not use the
root credential for routine XML imports or provider registration.

See [Configuration](CONFIGURATION.md) for settings, default values, and
credential ownership. This Compose configuration uses the bundled MariaDB
service. An external database requires a separately configured deployment.

## Source XML retention and database recovery

The importer reads files from a read-only mount; it does not erase
the original downloads. Retained XML exports can be imported again to
reconstruct the usage readings they contain. A database backup remains
useful for preserving source registrations, import provenance, revision
history, and information absent from the retained XML files.

Backups and restores should be validated before relying on them for
recovery. Do not delete an existing database volume as a troubleshooting
step.
