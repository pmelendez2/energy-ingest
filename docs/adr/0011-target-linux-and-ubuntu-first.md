<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0011: Use a Portable Core and Container-First Deployment

- Status: Accepted
- Date: 2026-09-29

## Context

`energy_ingest` is developed primarily in a Linux Home Lab environment and is expected to run on both x86-64 and ARM64 systems.

The project should avoid unnecessary coupling to one host distribution while still defining a practical, testable baseline.

Containerized deployment provides a consistent runtime boundary for services such as the application and MariaDB.

## Decision

The application core will target portable Rust behavior and avoid unnecessary operating-system-specific dependencies.

Linux is the primary runtime family.

The primary tested development baseline will be Ubuntu, with exact tested releases recorded outside this ADR according to [ADR-0019](0019-development-environment-source-control-and-ai-assisted-development-policy.md).

The initial supported runtime architectures are:

- `linux/amd64`;
- `linux/arm64`.

ARM64 support includes Raspberry Pi 5 class systems.

Normal deployment will be container-first where practical.

The architecture must not require a proprietary container runtime or cloud platform.

Native local development remains supported when the required Rust toolchain and dependencies are available.

A platform must not be described as tested merely because Rust or container tooling is theoretically portable to it.

Actual tested environments and release validation belong in `docs/TESTED_ENVIRONMENTS.md` under ADR-0019.

The exact CI matrix, container image tags, package prerequisites, and host setup instructions are development/deployment documentation rather than ADR content.

## Consequences

The project has a clear primary environment without claiming universal platform validation.

Containerized deployment reduces host-specific runtime variation.

Supporting both amd64 and arm64 requires dependencies and images that work on both architectures.

Additional operating systems or architectures may be added later when they are actually validated.
