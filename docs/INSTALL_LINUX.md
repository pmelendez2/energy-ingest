<!-- SPDX-License-Identifier: MIT -->

# Install on Linux

**Purpose:** Obtain the published source release containing production
Compose definitions and setup scripts, then initialize Energy Usage.

**Intended Audience:** Linux customers and system administrators.

## Requirements

Install Docker Engine, Docker Compose v2, Git, and access to Docker Hub.
The account running setup must be authorized to operate Docker. Docker
access is privileged; follow Docker's security guidance. No host Rust
compiler or MariaDB server is required.

## Obtain the official release archive

Open the [GitHub Releases page](https://github.com/pmelendez2/energy_ingest/releases)
and choose the version you intend to install. The release must provide an
`energy-ingest-prod-VERSION.zip` file and its `.sha256` checksum.
For example, for release `0.1.2`, run these commands on your **Linux host**:

```bash
version=0.1.2
base="https://github.com/pmelendez2/energy_ingest/releases/download/energy-ingest-v${version}"
mkdir -p "$HOME/energy-ingest-prod"
cd "$HOME/energy-ingest-prod"
curl -fLO "$base/energy-ingest-prod-${version}.zip"
curl -fLO "$base/energy-ingest-prod-${version}.zip.sha256"
sha256sum -c "energy-ingest-prod-${version}.zip.sha256"
unzip "energy-ingest-prod-${version}.zip"
```

If any download or checksum step fails, **stop**. Do not use a
privately assembled archive or substitute source files. The archive
contains production scripts, Compose configuration, and documentation;
Docker Compose subsequently pulls the version-matched image from Docker Hub.
No Git checkout or Rust toolchain is required.

## Initialize and verify

From the project root, run:

```bash
./scripts/setup-prod.sh
./scripts/verify-prod.sh
```

Setup generates local credentials and initializes MariaDB. Follow its
interactive prompts; ordinary startup preserves existing database data.

Continue with [Installation](INSTALLATION.md) for the host-side import
folder, registration, and XML import instructions. Production secrets
are generated under `deploy/docker-compose/prod-secrets/`.
