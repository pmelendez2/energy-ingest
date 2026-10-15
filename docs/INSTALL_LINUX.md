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

## Obtain the installation files

Release `energy-ingest-v0.1.0` identifies source commit `41798b7`.
The published application image is
`docker.io/pmelendez2/gb-energy-ingest:0.1.0` (AMD64/ARM64).

The installation source is hosted in the
[GitHub repository](https://github.com/pmelendez2/energy_ingest).
Use a Git checkout to obtain the exact Compose and script files:

```bash
git clone https://github.com/pmelendez2/energy_ingest.git
cd energy_ingest
git checkout energy-ingest-v0.1.0
```

This fetches source as well as the scripts, but customers do not need
to build or compile it. The Compose production template selects the
published image. Verify the repository URL and public tag before
relying on this method; their accessibility was not live-checked in
this static documentation review.

No separate installation ZIP is asserted as published here. A future
release may provide a smaller, verified installation bundle.

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
