<!-- SPDX-License-Identifier: MIT -->

# Install Energy Usage

**Purpose:** Set up Energy Ingest from its production release and import
Green Button XML without installing Rust or cloning the source repository.

**Intended audience:** Linux customers and administrators. The Linux
installation is currently the validated production path.

## 1. Download, initialize, and verify

Follow [Install on Linux](INSTALL_LINUX.md) first. That guide downloads the
verified production release, initializes MariaDB and its schema, and runs
verification. **Do not repeat setup after returning to this guide.**

If you already see `Deployment checks passed`, continue with step 2.
Do not recreate the database or its credentials.

## 2. Keep the original Green Button XML files in your own directory

Download Green Button XML exports from your utility provider. Keep the
original files in a **private directory you control, outside**
`$HOME/energy-ingest-prod` and its `deploy/docker-compose/` directory.
Use a full (absolute) Linux path for that directory, for example:

```text
/home/user/utility-xml
```

The directory must already exist and contain readable `.xml` files. Do not
commit or attach original customer XML to public issues or repositories.
You do not need to copy your originals into `private-imports` yourself.

## 3. Run the guided utility registration and XML import

The standard first-time workflow is the interactive guided importer:

```bash
"$HOME/energy-ingest-prod/scripts/energy-import.sh" \
  --import-dir "/absolute/path/to/your/utility-xml"
```

Replace the example path with the location of your original XML files.
The default selection is `*.xml`. To select a subset, use a quoted file
pattern, for example `--file 'electric-*.xml'`. Run
`"$HOME/energy-ingest-prod/scripts/energy-import.sh" --help` for options.

The guided installer will:

1. Show the selected XML files and ask permission to copy them into
   protected application storage for import. Your originals remain in
   your own directory and are not changed.
2. Register or select your utility and download source by name.
3. Register your physical electricity and gas meters, including their
   utility-assigned meter numbers, before importing XML.
4. Import the selected XML files after your confirmation.
5. Review imported file data and confirm association with the correct
   physical meters. When multiple meters of the same commodity are
   registered, automatic association is disabled. Individual
   associations must be reviewed using the meter administration tool.

**Before starting:** Find the actual utility-assigned meter numbers on
your bill or online utility account and keep them available for meter
identification. These are different from Energy Ingest database IDs.
Green Button XML may not include the utility meter numbers.

The guided workflow requests the actual utility-assigned meter number,
displays the entered value for confirmation, and stores it separately
from the friendly meter name.

Physical meter registration requires an administration image supporting
the `meter set-number` command, available beginning with v0.1.4.
Earlier releases do not support this onboarding procedure.

## 4. Verify the installation and import

Recheck deployment health:

```bash
"$HOME/energy-ingest-prod/scripts/verify-prod.sh"
```

Verify physical meters and their assignments with the supplied meter
administration tool:

```bash
"$HOME/energy-ingest-prod/scripts/meter-prod.sh" list
```

The guided script reports whether association finished. A successful import
is not proof that historical readings are gap-free or that usage streams
are conflict-free; review the results before using any totals.

For database and backup operations, see
[Production Database](PRODUCTION_DATABASE.md). For advanced manual operations,
see [Command Reference](SCRIPT_COMMANDS.md).

## 5. Additional settings and advanced imports

`deploy/docker-compose/private-imports/` is the application-managed staging
location. It is **not** the recommended permanent home of original utility
exports. The optional `ENGY_IMPORT_DIR` field in
`deploy/docker-compose/.env.prod` configures the Compose-side default import
path for manual import operations. It does not relocate or manage your
original downloads, and it is not necessary with guided `--import-dir`.

If Compose environment or bind-mount settings are edited, **recreate the
affected Compose service** for the change to take effect; simply restarting
an existing container does not apply changed Compose configuration. Do not
recreate database volumes. See [Configuration](CONFIGURATION.md).

## Home Assistant integration

The separate `ha_energy_usage` Home Assistant integration is not part of
this backend installer. It communicates through the authenticated Energy
Ingest API; Home Assistant never connects directly to MariaDB. The HA
integration's installation and runtime validation are a separate stage.
