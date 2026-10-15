<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

<!-- SPDX-License-Identifier: MIT -->

# Containerized Development and Test Environments

## Purpose

**Intended Audience:** Developers, contributors, and CI maintainers.

`energy_ingest` uses Docker Compose as the reference development and test
environment.

The host requires:

- Docker Engine;
- Docker Compose v2;
- Git; and
- Bash.

Rust, Cargo, MariaDB, and project build dependencies are provided by
containers. A host-installed Rust toolchain or MariaDB server is not required.

No project-specific homelab deployment manager is required.

## Repository Location

The repository may be cloned at any filesystem location.

No parent repository, parent directory name, or local directory hierarchy is
required. In particular, the public development workflow does not depend on a
`homelab-tools` parent repository or a path such as:

```text
~/src/hlt/energy_ingest
```

Valid examples include:

```text
~/energy_ingest
~/projects/energy_ingest
/opt/src/energy_ingest
/tmp/energy_ingest
```

The supported workflow is:

```text
clone repository anywhere
        |
        v
enter the repository
        |
        v
run project tooling
```

The helper scripts derive repository paths from Git metadata and their current
project context rather than from a fixed host path.

A standalone clone is therefore the normal public-release model.

## Environment Model

The project keeps development and test environments separate.

Development:

```text
Compose project: energy-ingest-dev
Database:        energy_ingest_dev
Database state:  persistent
Cargo caches:    persistent
Usage:           interactive development
```

Test:

```text
Compose project: energy-ingest-test
Database:        energy_ingest_test
Database state:  ephemeral
Credentials:     generated per run
Usage:           automated verification
```

Production is a separate deployment target and must not reuse development or
test credentials or database state.

## Cargo workspace and command verification

The workspace contains packages `gb` and `gb_profile`, along with
`energy-store`, `energy-api`, and other crates. There is no package
called `greenbutton`. For profile commands use, from the project root:

```bash
cargo run -p gb_profile -- list
```

Check `Cargo.toml` package names before documenting new Cargo commands.

## Development Files

The development environment is defined by:

```text
docker/Dockerfile.dev
deploy/docker-compose/.env.dev.example
deploy/docker-compose/compose.dev.yml
deploy/docker-compose/10-runtime-grants.sh
deploy/docker-compose/setup-dev.sh
deploy/docker-compose/dev.sh
deploy/docker-compose/status-dev.sh
deploy/docker-compose/dev-secrets/.gitignore
```

`.env.dev.example` is a tracked release artifact. A clean clone must
contain this file because `setup-dev.sh` uses it to create the local
`.env.dev` configuration during first-time setup.

Local files that are intentionally excluded from source control include:

```text
deploy/docker-compose/.env.dev
deploy/docker-compose/dev-secrets/energy_db_pwd
deploy/docker-compose/dev-secrets/energy_db_root_pwd
```

## Test Files

The test environment is defined by:

```text
deploy/docker-compose/compose.test.yml
deploy/docker-compose/test.sh
```

The test environment uses the same development image definition from:

```text
docker/Dockerfile.dev
```

## Git Repository Mount

The helper scripts detect the host Git top-level and mount that repository at:

```text
/workspace/repo
```

The project working directory is selected relative to that Git root.

This supports both:

- a standalone `energy_ingest` clone; and
- an `energy_ingest` directory inside a larger local Git repository.

The larger-repository case is a local development convenience only. It is not
a requirement for release users.

The containers use Git process configuration to mark `/workspace/repo` as a
safe directory. This allows Git commands to operate on the bind-mounted
repository when host and container ownership differ.

The test repository mount is read/write because `scripts/verify.sh` updates:

```text
.test-status.md
.test-history.csv
```

Database state remains isolated from the source repository.

## Container Working Directory

The development image uses the neutral default:

```text
/workspace
```

The image does not encode the repository name or a host directory hierarchy.

Docker Compose sets the effective working directory after mounting the detected
repository. This keeps the image reusable regardless of where the source was
cloned on the host.

## Development Image

The development image is based on the configured Rust image and includes the
components and packages required by the current verification workflow.

The image includes:

- Cargo;
- `rustfmt`;
- Clippy;
- CA certificates;
- a MariaDB client; and
- `pkg-config`.

The image is built from:

```text
docker/Dockerfile.dev
```

## First-Time Development Setup

From any clone location, enter the deployment directory:

```bash
cd deploy/docker-compose
./setup-dev.sh
```

If no development environment exists, the script reports the planned actions
before changing anything.

A first-time setup creates:

- `.env.dev` from `.env.dev.example`;
- URL-safe development database credentials;
- the Rust development image;
- the Rust development container;
- the MariaDB development container;
- a persistent development database volume;
- Cargo cache volumes; and
- the Compose network.

An unrelated Docker Swarm environment on the same host is not part of this
Compose project and is not modified by this workflow.

## Development Lifecycle

`setup-dev.sh` detects the current state before presenting actions.

The validated menu is:

```text
1. Continue using this environment
2. Refresh development containers
3. Recreate development database
4. Remove development environment
5. Cancel
```

### Option 1: Continue

Option 1 preserves:

- `.env.dev`;
- development credentials;
- database data;
- Cargo caches; and
- the existing development configuration.

It starts or reconciles the development containers and waits for MariaDB to
become healthy.

### Option 2: Refresh

Option 2 rebuilds and recreates the development containers while preserving:

- `.env.dev`;
- credentials;
- the development database volume and data; and
- Cargo cache volumes.

### Option 3: Recreate Database

Option 3 permanently deletes the development database volume after explicit
confirmation and creates a new empty database volume.

The workflow separately asks whether to:

- generate new database credentials; and
- reset `.env.dev` to defaults.

Both preservation and regeneration paths have been validated.

### Option 4: Remove Environment

Option 4 removes the development containers and Compose network.

The workflow separately asks whether to delete:

- persistent development database data;
- `.env.dev`; and
- development database credentials.

A temporary teardown can preserve all persistent state.

A full removal deletes the database volume, settings, and credentials. After a
full removal, a subsequent `setup-dev.sh` run detects a new environment and
performs first-time setup again.

Cargo cache volumes and the built image are not treated as database state by
this removal workflow.

### Option 5: Cancel

Option 5 exits without modifying development resources.

## Development Status

Run:

```bash
./status-dev.sh
```

The status script reports:

- Compose project name;
- `.env.dev` presence;
- runtime secret presence;
- root secret presence;
- development database volume presence;
- development container state;
- MariaDB container state; and
- MariaDB health.

It can report state even when development containers have been removed.

## Enter the Development Container

Run:

```bash
./dev.sh
```

The development shell runs inside the Rust container.

`dev.sh` constructs the runtime database connection URL inside the container
from the mounted runtime database secret. The password is not stored in
`compose.dev.yml`.

Normal development commands include:

```bash
cargo fmt
cargo check --workspace
cargo clippy --workspace
cargo test --workspace
# Full database-backed verification is run from the host; see below.
```

## Development MariaDB Initialization

Development MariaDB uses a persistent named volume.

The official MariaDB entrypoint reads the configured `_FILE` variables during
initialization and exports the root password before executing scripts from:

```text
/docker-entrypoint-initdb.d
```

`10-runtime-grants.sh` uses that exported root password. It does not reopen the
file-backed root secret after MariaDB has switched to the `mysql` user.

The script reduces the normal runtime database user to:

```text
SELECT
INSERT
UPDATE
DELETE
```

Schema bootstrap remains an administrative operation.

The MariaDB health check uses:

```text
healthcheck.sh --connect --innodb_initialized
```

A 60-second start period allows normal first-time initialization to complete
before health failures are counted.

## Test Workflow

From the repository root on the host, run:

```bash
./deploy/docker-compose/test.sh
```

This is the recommended full verification command. Running
`./scripts/verify.sh` directly requires an available isolated test
database and `ENGY_T_DB_URL`. The Compose wrapper configures both.

The workflow:

1. detects the host Git root;
2. creates temporary URL-safe database credentials;
3. builds the test runner image;
4. starts the ephemeral MariaDB test service;
5. waits for MariaDB to become healthy;
6. runs the verification container;
7. returns the verification container exit status; and
8. removes the ephemeral test environment and temporary credentials.

The MariaDB test data directory uses `tmpfs`.

Test credentials are generated for each run and are not reused by development
or production.

The integration-test variable:

```text
ENGY_T_DB_URL
```

uses the administrative test database connection because database bootstrap
tests must execute schema migrations.

## Verification

The test container runs:

```text
scripts/verify.sh
```

The current verification workflow covers:

- formatting;
- compilation;
- Clippy;
- unit tests;
- integration tests;
- documentation tests; and
- ADR conformance; and
- strict Markdown documentation validation.

A successful run ends with output equivalent to:

```text
All verification checks passed.
Updated .test-status.md
Updated .test-history.csv
Verification completed successfully.
Cleaning up ephemeral test environment...
```

The one-shot test container must exit with status `0`.

## Cross-Host Portability Test

A portability test should use a clean clone at a path that does not reproduce
the maintainer's normal local hierarchy.

For example:

```bash
git clone <repository-url> ~/energy-ingest-portability-test
cd ~/energy-ingest-portability-test/deploy/docker-compose

./setup-dev.sh
./status-dev.sh
./test.sh
```

A Raspberry Pi 5 running a supported 64-bit Linux and Docker installation is a
useful ARM64 portability target.

The test should verify that no command requires:

```text
homelab-tools
~/src/hlt
a fixed repository parent
a fixed repository clone path
```

## Environment Isolation

The reference names are intentionally distinct:

```text
Development Compose project: energy-ingest-dev
Development database:        energy_ingest_dev

Test Compose project:        energy-ingest-test
Test database:               energy_ingest_test

Production Compose project:  energy-ingest
Production database:         energy_ingest
```

Development and test resources must not depend on a Docker Swarm deployment or
private homelab management scripts.

## Tested Environments

Validated operating systems, architectures, and portability checks are recorded
in:

```text
docs/TESTED_ENVIRONMENTS.md
```

Update that file when a development or release-validation workflow is exercised
on a new environment.
