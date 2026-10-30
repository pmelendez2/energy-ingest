<!-- SPDX-License-Identifier: MIT -->

# Install on Windows

**Purpose:** Document a potential Docker Desktop/WSL2 installation path.

**Intended Audience:** Windows customers evaluating an unvalidated path.

## Requirements

Docker Desktop with Linux containers and Compose v2, a supported WSL2
setup, and sufficient disk space for a persistent MariaDB volume.
The supplied setup utilities are Bash scripts: execute them inside
a WSL2 Linux shell with Docker Desktop integration enabled. Windows
PowerShell alone cannot execute these scripts directly.

## Release validation status

The `0.1.0` container image includes Linux AMD64 and ARM64 platforms.
The Windows/WSL2 installation path below is guidance only; it has not
been validated for this release. See
[release validation](testing/RELEASE_0_1_0_VALIDATION.md).

## Installation

1. Obtain the installation files from the verified release source
   described in [Linux installation](INSTALL_LINUX.md).
2. Open the extracted project folder from your WSL2 shell. For
   example, Windows `C:\Users\YourName\EnergyUsage` is accessible
   as `/mnt/c/Users/YourName/EnergyUsage` in a typical WSL2 setup.
3. From the installation root in the WSL2 shell, run:

```bash
./scripts/setup-prod.sh
./scripts/verify-prod.sh
```

Docker Desktop must be running. Keep production secrets and private
XML files in an appropriately access-controlled location. File
permissions on Windows-mounted drives differ from Linux filesystem
permissions; for secrets, a private directory in the WSL2 filesystem
is preferable. Do not store credentials in a shared/public directory.

To import from a user directory, supply a **WSL2-accessible path**.
For example:

```bash
./scripts/energy-import.sh --source-id 42 \
    --import-dir /mnt/c/Users/YourName/EnergyXML \
    --file usage-export.xml
```

Replace the example path and source ID with your values. Windows
installation and path mapping should be validated on the intended
Docker Desktop/WSL2 version before customer distribution.

Return to [Installation](INSTALLATION.md) for the remaining steps.
