<!-- SPDX-License-Identifier: MIT -->

# Install on Linux

## Requirements

A Linux computer with Docker Engine and Docker Compose v2, access to
container images, and persistent local storage. The account running
setup must be permitted to operate Docker. Docker access grants broad
host privileges; follow Docker's security guidance.

## Installation

1. Download an Energy Usage installation archive from the project's
   distribution location and extract it to a private directory.
2. Open a terminal and change to `deploy/docker-compose` inside it.
3. Run the setup and check the database status:

```bash
bash setup-prod.sh
bash status-prod.sh
```

Follow the displayed setup prompts. Credentials are generated locally.
The normal setup path preserves an existing database volume.

Return to [Installation](INSTALLATION.md) for utility registration and
XML import. For an alternative input directory, use an absolute Linux
path with `--import-dir`, or set `ENGY_IMPORT_DIR` in `.env.prod`.
