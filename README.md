<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# energy_ingest

Local-first residential energy-data ingestion service.

The initial implementation targets Green Button / ESPI usage data, with
SDG&E Green Button Download My Data XML serving as the first real-world
provider and regression-test source.

The project is intended to support additional energy-data providers and
acquisition mechanisms through provider adapters, including OAuth 2.0 APIs.

**Audience:** Users, contributors, developers, and maintainers.

**Purpose:** Introduce the project and direct readers to authoritative guides.

## Initial goals

The following are project goals, not a declaration that each capability
is complete. Home Assistant integration is planned separately.

- Parse Green Button / ESPI XML correctly.
- Preserve source measurement metadata and reading quality.
- Support electricity and gas independently.
- Support multiple measurement streams and flow directions.
- Import historical and incremental usage data.
- Detect gaps and overlapping coverage.
- Handle revisions and corrections idempotently.
- Provide normalized data to downstream consumers such as Home Assistant.
- Support provider-specific authentication, including OAuth 2.0.
- Keep credentials outside the source repository.

## Platform

Currently supported development and runtime platform:

- Ubuntu 26.04
- Linux
- Docker/container-oriented deployment

Not currently supported or tested:

- Windows
- macOS

Portability should not be intentionally prevented in the Rust core where it
does not add significant complexity.

## Security

Static credentials must not be committed to this repository.

Local deployments may provide credentials through SOPS/age-backed secrets or
other secure runtime mechanisms.

See `SECURITY.md` and the architecture decision records under `docs/adr/`.

## Project status

Production images are published for `linux/amd64` and `linux/arm64`.
The current development release target is recorded once in
[`release/VERSION`](release/VERSION). It is not necessarily published yet.
For installation, select the most recent stable
[GitHub release](https://github.com/pmelendez2/energy_ingest/releases).

- Image: `docker.io/pmelendez2/gb-energy-ingest:<release-version>`.
- [Installation](docs/INSTALLATION.md) and
  [Linux setup](docs/INSTALL_LINUX.md).
- [Published release evidence](docs/RELEASE_0_1_0_VALIDATION.md).
- [Production release checklist](docs/PRODUCTION_RELEASE_CHECKLIST.md).

Consult source and tests for current feature readiness.

## Documentation

- [`docs/GLOSSARY.md`](docs/GLOSSARY.md) — canonical terms, including
  distinctions among schema changes, data movement, and backup/restore.
- [`docs/DEVELOPMENT_ENVIRONMENT.md`](docs/DEVELOPMENT_ENVIRONMENT.md)
  — local development and verification with Docker Compose.
- [`profiles/README.md`](profiles/README.md) — provider profiles,
  validation, and contributions.
- [`docs/PROFILE_RELEASES.md`](docs/PROFILE_RELEASES.md) — releases of
  the standalone `gb_profile` utility, not provider-profile revisions.
- [`docs/PROJECT_TASKS.md`](docs/PROJECT_TASKS.md) — active and deferred work.
- [`docs/adr/`](docs/adr/) — accepted architecture decisions.
- [`docs/testing/TEST_STRATEGY.md`](docs/testing/TEST_STRATEGY.md) and
  [`docs/testing/TEST_PLAN.md`](docs/testing/TEST_PLAN.md) — testing policy.
- [`docs/AIUSAGE.md`](docs/AIUSAGE.md) — AI-assisted development records.
- [`docs/I18N.md`](docs/I18N.md) — internationalization policy.
- [`docs/LEGAL_NOTICES.md`](docs/LEGAL_NOTICES.md) — licenses and marks.
- [`SECURITY.md`](SECURITY.md) — security reporting and handling.

## License

This project is licensed under the MIT License.

See `LICENSE` for the complete license text.
