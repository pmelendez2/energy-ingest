<!-- SPDX-License-Identifier: MIT -->

# Install on Linux

**Purpose:** Download the verified production installer and initialize the
Energy Ingest backend on Linux, then continue to guided XML onboarding.

**Intended audience:** Linux customers and system administrators.

## Requirements

Install Docker Engine, Docker Compose v2, `curl`, `unzip`, and `sha256sum`.
The account running setup needs permission to operate Docker (a privileged
capability). Access to Docker Hub is required. No Rust compiler, Git clone,
or host MariaDB installation is required.

## Download the official production release

Select the current stable version from the
[official releases](https://github.com/pmelendez2/energy_ingest/releases).
Set `version` to that **published** release number. The development target
recorded in `release/VERSION` may not be published yet. Do not use it until
the corresponding GitHub release and container image exist.

```bash
version=REPLACE_WITH_PUBLISHED_VERSION
base="https://github.com/pmelendez2/energy_ingest/releases/download/energy-ingest-v${version}"
mkdir -p "$HOME/energy-ingest-prod"
cd "$HOME/energy-ingest-prod"
curl -fLO "$base/energy-ingest-prod-${version}.zip"
curl -fLO "$base/energy-ingest-prod-${version}.zip.sha256"
sha256sum -c "energy-ingest-prod-${version}.zip.sha256"
unzip "energy-ingest-prod-${version}.zip"
```

Stop if the download or checksum verification fails. Production setup uses
the version-matched Docker image defined in the extracted `.env` template.
Do not replace it with a locally built development image.

## Initialize and verify MariaDB (one-time step)

These commands may be executed from any working directory:

```bash
"$HOME/energy-ingest-prod/scripts/setup-prod.sh"
"$HOME/energy-ingest-prod/scripts/verify-prod.sh"
```

Read the interactive setup prompts carefully. On first installation, setup
creates database credentials, a persistent MariaDB volume, and the schema.
Normal subsequent setup preserves existing data; recreate/remove options can
destroy it. The database is not exposed on host port 3306.

After `Deployment checks passed`, **do not repeat setup**. Continue directly
to **step 2** of [Installation](INSTALLATION.md) to locate original utility
XML and run guided import.

Secrets are generated in
`$HOME/energy-ingest-prod/deploy/docker-compose/prod-secrets/`.
Do not publish their contents.
