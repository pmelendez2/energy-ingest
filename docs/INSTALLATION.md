<!-- SPDX-License-Identifier: MIT -->

# Install Energy Usage

Energy Usage stores electricity and gas usage readings from Green Button
XML files in a local database. This guide explains how to install the
Docker deployment, register a utility, and import your files.

## 1. Choose your operating system

Follow the setup instructions for your computer:

- [Linux installation](INSTALL_LINUX.md)
- [Windows installation](INSTALL_WINDOWS.md)

Both guides use Docker Compose to run Energy Usage and its MariaDB database.
The installation package includes the configuration and setup scripts.
You do not need to install Rust, use Git, or create database tables manually.

## 2. Start Energy Usage

Open a terminal as described in your operating-system guide. From the
installation package's `deploy/docker-compose` directory, run:

```bash
bash setup-prod.sh
```

Follow the prompts. The setup creates local database credentials,
initializes the database, and starts the database service. Existing
stored data is preserved during normal startup; review any prompt
about resetting or removing data carefully.

Check its status with:

```bash
bash verify-prod.sh
```

For details about storage, credentials, and maintenance, see
[Production Database](PRODUCTION_DATABASE.md).

## 3. Download utility XML files when convenient

You can download XML before installation or after installation.
No XML files are required to start an empty Energy Usage database.

## 4. Keep your utility XML files

Sign in to your electricity or gas provider's website and download
available Green Button XML usage exports. Keep a copy of each export in
a private folder on your computer.

The default import folder is:

```text
deploy/docker-compose/private-imports/
```

You may instead use any folder accessible to Docker. You do not need to
create a separate directory for each XML download. Imports read the
source XML without changing or deleting it.

Retained XML files allow usage readings to be imported again if needed.
A database backup additionally preserves registration information,
import history, and other records that may not be recoverable from XML.

## 5. Register your utility

Once you are ready to import XML files, run:

```bash
bash register-prod.sh
```

Enter the utility name and an import source label, for example
**SDG&E** and **Manual XML Download**. The application records an
internal source identifier. You do not normally need to enter it.

## 6. Import your usage data

Downloaded XML files are optional during installation. You can import
now or return to this step later.

To preview all XML files in your preferred directory:

```bash
bash import-prod.sh \
  --import-dir "$HOME/energy_usage/data" \
  --file "*.xml" \
  --list-files
```

Remove `--list-files` to import them. Omit `--import-dir` to use the
configured default. To select one file, use its name instead of
`"*.xml"`.

If there is exactly one registered acquisition source, Energy Usage
selects it automatically. For multiple sources, specify
`--source-id "<SOURCE_ID>"` using the ID shown at registration. Only
combine files obtained through the same acquisition source.

Files are read without modification. Overlapping Green Button
resources must not be summed before logical stream validation.

## 7. Connect Home Assistant

Energy Usage and Home Assistant have separate responsibilities: Energy
Usage stores and manages usage readings, while Home Assistant displays
them through its integration. The database installation described here
does not itself install Home Assistant entities or connect directly to
Home Assistant.

When configuring the Home Assistant integration, use the Energy Usage
application's authenticated API rather than MariaDB credentials. Do not
expose the database service directly to Home Assistant or the internet.

## Configuration and help

See [Configuration](CONFIGURATION.md) for import directory and other
supported settings. See [Production Database](PRODUCTION_DATABASE.md)
for service responsibilities, persistent storage, and recovery guidance.

Keep utility XML exports and generated credential files private. Do not
include them in public support requests.
