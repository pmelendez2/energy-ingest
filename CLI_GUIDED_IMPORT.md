<!-- SPDX-License-Identifier: MIT -->

# Import Green Button XML Files

Use the guided import command to bring electricity or natural gas usage
files into Energy Usage. The guide will show the selected files, check
whether the application can read them, and ask for approval before
copying files, changing access permissions, or importing data.

You do not need to know Docker user IDs or change file permissions
manually. Read each prompt before answering.

## Before you begin

1. Complete the [Energy Usage installation](docs/INSTALLATION.md)
   and [initial production database setup](docs/PRODUCTION_DATABASE.md).
   For operating-system instructions, use
   [Linux](docs/INSTALL_LINUX.md) or
   [Windows](docs/INSTALL_WINDOWS.md).
2. Download **Green Button XML** usage files from your utility provider.
   Keep the original files in a private, backed-up folder **outside**
   the Energy Usage installation directory.
3. Record the **physical meter number** for each electricity or gas
   service. It may appear on your utility bill or online account.
   Green Button XML files do not necessarily include this number.
4. Open a terminal on the computer running Energy Usage. The guided
   command checks utility and acquisition-source registration for you.
   If no source exists, it offers to register one before importing.

Meter numbers are used later to associate imported resources with the
correct physical meter. Do not publish your meter numbers, customer
identifiers, or original XML files.

## 1. Open the installation directory

Run commands from the folder where you extracted the Energy Usage
installation package:

```bash
cd ~/energy-ingest-prod
```

If you installed Energy Usage elsewhere, use that directory instead.

## 2. Choose the files to import

**Option A — Files are already staged in the application folder:**

Put the XML files in:

```text
deploy/docker-compose/private-imports/
```

Then run:

```bash
./scripts/energy-import.sh
```

**Option B — Recommended: keep original files in another folder:**

Point the command to the folder containing your downloads:

```bash
./scripts/energy-import.sh \
    --import-dir "$HOME/my-utility-xml"
```

Replace `my-utility-xml` with your own persistent directory. The
application offers to copy the selected files into its local private
staging folder; it does not alter files in the original directory.

### Keep originals outside the installation directory

`deploy/docker-compose/private-imports/` belongs to the Energy Usage
installation. If you later delete or replace the installation directory
(for example, while uninstalling or performing a clean reinstall),
**the staged XML files inside it may be deleted with the directory**.
Simply stopping Docker containers does not necessarily delete them;
the outcome depends on the removal procedure. Do not rely on this
folder as your only copy of utility data.

Keep original downloads in a separate private directory and include
them in your normal backups. For example:

```text
~/utility-data/green-button/
```

Import XML files from that location using `--import-dir`. Energy Ingest
copies the selected files into its managed staging directory:

```text
deploy/docker-compose/private-imports/
```

For a standard installation under `~/energy-ingest-prod`, this is
`~/energy-ingest-prod/deploy/docker-compose/private-imports/`.
The import container reads these copies through `/imports/`.
Original files and their permissions remain unchanged. Staging requires
additional disk space. Deleting the installation directory may delete
these copies, so preserve the original downloads in a separate,
backed-up location.

By default, the command selects regular `*.xml` files in the chosen
folder. To select one file, use its filename:

```bash
./scripts/energy-import.sh \
    --file 'electricity-export.xml'
```

### How the utility and acquisition source are selected

Every imported XML reading is associated with an **acquisition source**
(the utility and the method of obtaining its data). This identifies
where the readings came from, **not** which physical meter supplied
those readings. Meter association is required later in this guide.

The guided CLI reads the registered acquisition sources before import:

- **No sources:** Enter the utility name (for example, `SDG&E`) and
  source label (for example, `Green Button Download`). Review the
  proposed registration and answer `y` to approve. The command
  registers the missing source and displays the selected destination.
  Declining prevents the import from proceeding.
- **One source:** The command displays the **utility name**, acquisition
  source name, and internal source ID together, then selects it
  automatically. Confirm that
  the XML files came from that source before approving the import.
- **Multiple sources:** The command lists each registered utility and
  acquisition source by name, with the ID beside it. Enter the ID
  corresponding to the correct names.
- **Explicit `--source-id`:** The command validates the selected ID
  against existing registration and displays its utility/source names.

Example when you already know the source ID and the XML directory:

```bash
./scripts/energy-import.sh \
    --import-dir "$HOME/my-utility-xml" \
    --file '*.xml' \
    --source-id 1
```

This selects XML files in the specified folder and associates their
**imported observations with acquisition source ID 1**. The `--file`
option is optional because `*.xml` is the default. Replace the
example folder and ID with your own values. This command does **not**
automatically associate those observations with a physical meter.

If there is only one registered source, normally run the command
without `--source-id` and verify the named destination displayed:

```text
Import destination:
  Utility: SDG&E
  Acquisition source: Green Button Download
  Selection: SDG&E — Green Button Download (source ID 1)
```

For background on registration and installation, see
[Installation](docs/INSTALLATION.md). For normal first-time imports,
no separate registration command is required.

## 3. Review the files and staging request

The command shows how many XML files it found and lists their filenames.
Check that these are the files you intend to import.

If you selected files from outside the application's private import
folder, you will be asked whether to copy them into local staging:

```text
Files require secure staging in .../private-imports
Original files will not be modified.
Declining staging cancels the import.
Copy files into managed staging? [y/N]:
```

Enter `y` to approve or `N` to cancel. Pressing Enter also cancels.

### Importing files again

You can run the guided import repeatedly. The application checks the
selected XML files against any existing staged copies:

- **Same filename, identical contents:** the staged copy is reused.
  You can continue importing; previously accepted observations should
  appear as **Unchanged** in the report.
- **Same filename, different contents:** the application lists the
  changed files and asks for separate approval before replacing the
  local staged copies. Declining cancels the import. It does not
  change the original files or import any data.
- **New filename:** the file is staged normally, with your approval.

If you approve replacement, only the staged copy is replaced. The
original download is preserved. The importer then handles changed
observations through its revision-aware storage model; it does not
simply add every reading as a duplicate. Keep the original exports
available for recovery. Staging changes and database imports are
separate operations, so a later failure may leave updated staged copies
without completing the database import.

If your files are already inside `private-imports`, the application
uses them directly without copying or comparing against another source.
To import all local copies again, run:

```bash
./scripts/energy-import.sh --file '*.xml'
```

## 4. File staging and permissions

When importing XML files from outside the Energy Ingest installation,
you approve copying the files into protected local staging. This approval
also includes configuring the staged copies so the import service can
read them. No additional permission confirmation is required for those
application-created copies. The originals and their permissions are
not changed.

If you place files directly into `private-imports`, Energy Ingest
checks whether the import service can read them. If changes to those
customer-managed files are required, the command explains the proposed
permissions and asks for approval. If you decline, **the import is
cancelled and no database import occurs**. Administrative credentials
may be requested when applying approved changes.

Keep original Green Button downloads in a separate, backed-up location.
Staged copies may be removed when the application directory is deleted.

## 5. Confirm the database import

After the file-access check, review the acquisition-source destination
shown by the command. Confirm that the utility and source are correct
for every selected XML file. When only one source exists, this is
selected automatically. If several sources exist, you must choose.

After that, you will see:

```text
Access verified. Ready to import 10 files.
Proceed with database import? [y/N]:
```

The number of files will reflect your selection. Enter `y` to begin.
Enter `N`, or press Enter, to cancel without starting an import.

The completion report includes these counts:

| Field | Meaning |
| --- | --- |
| Observed | Usage observations read from the selected XML files |
| Unchanged | Observations already present with the same accepted value |
| Accepted | New observations and accepted revisions stored |

A repeat import of unchanged files should normally show observations
as **Unchanged**, rather than accept duplicates.

If an import fails after it begins, earlier work may already have been
saved. Follow the error message, correct the issue, and retry. Do not
assume that an interrupted multi-file import automatically rolls back
all earlier files.

## 6. Confirm physical-meter associations

Physical-meter association is **required** to complete onboarding.
After importing XML, the guided command checks whether resources from
this acquisition source are associated with physical meters.

For each detected electricity or natural gas service:

1. If one matching physical meter exists, review its friendly name.
2. If no meter exists, provide a friendly label and approve registration.
   Keep your meter number in your private records. Do not use an account
   number as a publicly shared diagnostic label.
3. Confirm that the resources belong to the same physical meter.
4. Review the compatible-resource preview and approve attachment.
5. The command verifies that no resources for the selected source remain
   unmapped. If verification fails, onboarding is incomplete.

If multiple physical meters of the same kind are registered, the guided
command **stops rather than grouping their data together**. Ask an
administrator to review each resource using `meter-prod.sh attach`,
then run the guided command again to verify the mapping.

Only `--kind electric` and `--kind gas` are currently supported. Solar
production is not yet validated. Do not assume a solar installation
is represented by an ordinary electricity consumption resource.

Declining required identity confirmation cancels meter setup. Previously
accepted XML observations remain in the database, but onboarding is
**incomplete** and those readings must not be used as a finalized
household consumption stream.

## 7. Repeat imports and troubleshooting

You can run the guided command again with the same XML files. Already
accepted observations should be reported as **Unchanged**. Identical
staged copies are reused. If the original XML changes while keeping
its filename, the script requests approval before replacing the local
staged copy; the original is never modified.

If meter setup was interrupted after a successful import, retry the same
command. Import idempotency avoids adding duplicate observations, and
existing meter mappings are retained.

After onboarding, review usage with the project's stream diagnostic
commands. Mapping resources to meters does not itself establish a
nonduplicated canonical consumption series.
