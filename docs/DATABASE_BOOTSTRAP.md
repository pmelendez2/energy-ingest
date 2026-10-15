<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

<!-- SPDX-License-Identifier: MIT -->

# Database Bootstrap

## Purpose

`energy-store-admin database bootstrap` initializes and verifies the
`energy_ingest` application schema in an already provisioned MariaDB database.

The command uses the same SQLx migration path compiled into `energy-store`.
Migration files under `crates/energy-store/migrations/` remain the
authoritative schema definition.

This command does not create the MariaDB server, database account, or database
itself. Deployment provisioning remains a separate operational concern.

**Intended Audience:** Developers and database administrators.

## Command

Run from a development environment with administrative database access:

```bash
cargo run   -p energy-store   --bin energy-store-admin   --   database bootstrap
```

The command requires:

```text
ENGY_DB_ADMIN_URL
```

in its environment.

`ENGY_DB_ADMIN_URL` contains the MariaDB connection URL for credentials allowed
to apply the application's schema migrations. The value must be supplied by the
deployment environment and must not be committed to source control, placed
directly on the command line, or written to normal logs.

Administrative database credentials are intended only for explicit bootstrap
or migration operations. Normal runtime services should use separate
credentials with the minimum practical privileges required for routine
operation.

## Behavior

The bootstrap command:

1. connects to the configured MariaDB database;
2. runs the SQLx migration set embedded in `energy-store`;
3. lets SQLx validate the previously applied migration history;
4. applies only pending migrations;
5. verifies that the expected application tables exist; and
6. reports success only after schema verification completes.

SQLx migration-history or checksum incompatibilities are reported as bootstrap
failures rather than being ignored or repaired automatically.

A successful bootstrap currently reports:

```text
Database bootstrap complete: 13 application tables verified.
```

## Repeatability

The command is designed to be rerun safely.

Already-applied migrations are tracked by SQLx and are not reapplied. A repeat
run still performs schema verification so an operator receives confirmation
that the current application schema is present.

Repeatability is covered by automated integration tests and has also been
validated against the pre-release production Docker Compose environment.

## Production Bootstrap

The production deployment provides:

```text
deploy/docker-compose/bootstrap-prod.sh
```

Run:

```bash
cd deploy/docker-compose
./bootstrap-prod.sh
```

The wrapper uses the production-only administrative image defined by:

```text
docker/Dockerfile.prod
```

and the production Compose service:

```text
admin
```

The temporary `admin` container:

- joins the production internal database network;
- receives the production administrative database secret;
- constructs `ENGY_DB_ADMIN_URL` inside the container;
- runs `energy-store-admin database bootstrap`; and
- is removed after the command exits.

The production bootstrap path does not depend on development or test
Dockerfiles, Compose files, scripts, images, or volumes.

The administrative image is built separately from bootstrap. This keeps
`bootstrap-prod.sh` focused on one task and avoids rebuilding the image on every
schema verification.

Build it with:

```bash
docker compose \\
  -p energy-ingest \\
  --env-file .env.prod \\
  -f compose.prod.yml \\
  build admin
```

The production build context is reduced by:

```text
docker/Dockerfile.prod.dockerignore
```

so local build outputs, secrets, Git metadata, and development/test artifacts
are not sent unnecessarily to the production image build.

## Production Validation

On 2026-10-06, the production bootstrap path was exercised repeatedly against
the pre-release production MariaDB deployment.

Two consecutive bootstrap runs completed successfully with:

```text
Database bootstrap complete: 13 application tables verified.
```

A subsequent run using the simplified `bootstrap-prod.sh` path also completed
successfully without rebuilding the administrative image.

The normal runtime database account was separately verified to have only:

```text
SELECT
INSERT
UPDATE
DELETE
```

on the application database.

A direct `CREATE TABLE` attempt using the runtime account was rejected by
MariaDB, confirming that schema-changing operations remain administrative.

## Failure Behavior

The command exits unsuccessfully when:

- `ENGY_DB_ADMIN_URL` is missing;
- the database connection fails;
- SQLx detects incompatible migration history;
- a pending migration cannot be applied;
- schema verification queries fail; or
- expected application tables are missing after migration.

Failure diagnostics must not print the database URL or other secret values.

## Home Lab Design

The production bootstrap workflow follows the KISS principle.

The reference deployment intentionally uses:

- one MariaDB service;
- one production administrative image;
- one temporary administrative container when explicitly requested; and
- one small wrapper script for the common bootstrap operation.

Home Lab operators may still inspect, modify, test, or manually run
administrative operations as needed.

Additional deployment complexity should be introduced only when a concrete
requirement justifies it.
