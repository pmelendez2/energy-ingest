<!-- SPDX-License-Identifier: MIT -->

# Verify an Energy Usage installation

## Purpose

Explain how to check an installed Energy Usage deployment.

**Intended Audience:** Users and operators troubleshooting an installation.

Use these checks after installation or when investigating an import
problem. They do not require database administration tools.

## Check the installed services

From the repository root on the host, run:

```bash
./scripts/verify-prod.sh
```

The command checks the local configuration, protected credential files,
Docker connectivity, Compose configuration, and MariaDB health. It does
not change the database.

If a check fails, review the displayed message and use
`./scripts/setup-prod.sh` to continue a normal installation. Do not choose
an option that deletes or recreates stored data during troubleshooting.

## Register your utility

To register SDG&E without entering a name interactively:

```bash
./scripts/register-prod.sh \
  --utility 'SDG&E' \
  --source 'Manual XML Download'
```

The command displays a **Source ID**. Keep it for advanced multi-source imports. Registration
writes source records to the selected database; it is not a read-only
verification step. Avoid repeating registration unnecessarily.

## Import a utility export

Place the downloaded XML file in your configured private import folder.
The default is `deploy/docker-compose/private-imports`.

```bash
./scripts/import-prod.sh --file usage-export.xml
```

Replace the filename with the downloaded XML filename. If you use a
custom folder, add `--import-dir /path/to/your/folder`.

The original XML file is mounted read-only. Preserve it privately for
possible reimport. Successful reimport of the same data should not
create duplicate canonical readings; the importer manages revisions
and overlap.

## Review import results

Confirm the import command exited successfully and examine its reported
statistics. If an import fails, review the message before retrying.
Do not share raw utility XML or credentials in public issue reports.

## Home Assistant

The database verification above does not check a Home Assistant
connection. The Home Assistant integration is planned for a separate project.
It will use the authenticated application API, not MariaDB access.

## Preview and import multiple files

Preview the files selected from your download directory:

```bash
./scripts/import-prod.sh \
  --import-dir "$HOME/energy_usage/data" \
  --file "*.xml" \
  --list-files
```

After reviewing the list, remove `--list-files` to import them.
Only use a source ID that matches the provider and acquisition source
for every selected file. Split files by source if necessary.

Reimporting an existing XML file should report unchanged observations
for matching resource identities. Equivalent consumption under distinct
resource IDs is retained separately and requires stream selection
before aggregation in Home Assistant.
