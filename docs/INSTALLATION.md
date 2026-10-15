<!-- SPDX-License-Identifier: MIT -->

# Install Energy Usage

**Purpose:** Guide customers through production setup, utility registration,
and Green Button XML import without requiring a Rust toolchain.

**Intended Audience:** Linux customers and installation administrators.
Windows/WSL2 users should review its unvalidated platform guidance.

## Published release and prerequisites

The initial production image is
`docker.io/pmelendez2/gb-energy-ingest:0.1.0` for Linux AMD64 and ARM64.
The published image is not itself a complete installation bundle.
The setup scripts and Compose files must also be obtained from a
verified source checkout or published release archive.

Follow [Linux installation](INSTALL_LINUX.md) or
[Windows guidance](INSTALL_WINDOWS.md) first. Windows/WSL2 installation
has not been validated for this release. No host Rust installation is
needed to run the production image.

Run the commands below **from the project root**. Scripts in `scripts/`
locate their Compose configuration automatically. The `.sh` scripts have
Bash shebangs; use direct execution when their executable bits are retained.

## 1. Set up the database

```bash
./scripts/setup-prod.sh
./scripts/verify-prod.sh
```

The interactive setup prepares credentials and starts MariaDB and the
schema migration service. Normal setup preserves existing database data.
Review any recreate/remove prompt carefully: those operations can delete data.
XML exports are not needed for initial setup.

## 2. Store Green Button XML on the host

Download Green Button XML exports from your utility provider and store
copies privately. The **default host operating-system directory**, relative
to the project checkout or extracted installation directory, is:

```text
deploy/docker-compose/private-imports/
```

`./scripts/setup-prod.sh` creates this directory during initialization,
so no manual directory creation is normally required. To open it from a
terminal at the project root:

```bash
cd deploy/docker-compose/private-imports
pwd
```

Move or copy your downloaded `.xml` files into that host folder using
your preferred file manager or standard file copy operation. Return to
the project root before running the commands below. The import script
uses a read-only Docker container mount for the selected host folder;
it does not modify or delete the originals.

You may alternatively use `--import-dir` with an absolute host path,
or configure `ENGY_IMPORT_DIR` in `deploy/docker-compose/.env.prod`.
The script resolves relative paths from `deploy/docker-compose/`, not
from your shell's current directory. See [Configuration](CONFIGURATION.md).

## 3. Register an acquisition source

```bash
./scripts/register-prod.sh
```

Follow the prompts to identify the utility and the source of the XML
export. Retain the returned source ID if you use more than one source.

## 4. Preview and import XML files

From the **project root**:

```bash
./scripts/import-prod.sh --file '*.xml' --list-files
./scripts/import-prod.sh --file '*.xml'
```

To use a different host directory, add:

```text
--import-dir /absolute/host/path
```

A single registered acquisition source is selected automatically. If
there are multiple sources, pass `--source-id ID`. Do not combine exports
from different acquisition sources under one ID. Keep XML exports private;
never attach raw customer data to public support requests.

## 5. Verify and maintain

```bash
./scripts/verify-prod.sh
```

See [Production Database](PRODUCTION_DATABASE.md) for secrets, storage,
backups, and maintenance. See [Command Reference](SCRIPT_COMMANDS.md)
for additional operations.

## Planned: Home Assistant integration

A separate `ha_energy_usage` Home Assistant integration is planned.
It is **not** part of this validated installation procedure. The planned
integration will use the application's authenticated API, not direct
MariaDB access. Do not expose MariaDB to Home Assistant or the internet.
