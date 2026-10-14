<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0019: Development Environment, Source Control, and AI-Assisted Development Policy

- Status: Accepted
- Date: 2026-09-30

## Context

`energy_ingest` is an open-source Rust project with a local-first development workflow and may use substantial AI assistance.

The project needs reproducible development, human ownership of accepted work, and a lightweight source-control process suitable for a sole maintainer.

## Decision

Development will be local-first, Git-based, and based on open-source tooling.

The project will maintain an explicit tested development baseline rather than implying that every platform on which Rust may run is tested.

Environment claims use these project terms:

- **tested** — explicitly exercised by the maintainer or CI;
- **supported** — intended to remain compatible;
- **unverified** — may work but has not been sufficiently validated.

Current and release-specific tested environments are recorded in:

`docs/TESTED_ENVIRONMENTS.md`

The Git repository is the authoritative source for project-controlled source, tests, migrations, documentation, ADRs, scripts, CI configuration, and safe-to-commit configuration.

Secrets, mutable runtime credentials, generated local state, and customer-specific data do not belong in the repository.

## Runtime and Environment Separation

`energy_ingest` is a local-first application distributed for production use
through supported prebuilt containers. Operating the production application
must not require a Rust toolchain, source checkout, contributor test database,
or knowledge of internal database identifiers.

Production, development, and automated testing are separate operating
contexts with independent configuration, database storage, and credentials.
Production data, credentials, and customer XML must never be copied into
non-production environments implicitly.

Production installation owns initialization and migration of its persistent
schema and the creation of application-managed metadata needed for routine
imports. End users must not be required to run contributor bootstrap tools,
manual SQL statements, or fixture preparation workflows.

The same maintained application logic and migrations govern all contexts.
Testing can use synthetic or appropriately sanitized fixtures and disposable
infrastructure; development tooling and destructive test capabilities are
excluded from production images by default. Production retains appropriate
health and diagnostic capabilities without embedding a test environment.

The deployment topology, supported container images, onboarding interface,
and provider-specific automated retrieval options belong in implementation
and operations documents, not in this architectural policy.

**Tradeoff:** Separate runtime configurations and release packaging increase
validation work but reduce accidental production data exposure, remove
contributor-tooling dependencies for users, and make deployment behavior
more predictable.

## Source Control

The sole-maintainer workflow will remain simple.

Small, well-understood changes may be committed directly to the primary branch.

Short-lived branches may be used when isolation provides practical value.

The project does not require Git Flow, mandatory pull requests for the sole maintainer, or multiple approvals during the pre-1.0 development phase unless a demonstrated need arises. These practices may be introduced after the first stable release or earlier if collaboration, release management, or governance requirements justify the additional overhead

Commits should represent coherent, understandable changes.

Detailed branch naming, commit conventions, and release workflow belong in contributor/development documentation.

## Human Review of AI-Assisted Work

All source code generated or substantially modified with AI assistance must be explicitly reviewed by a human before acceptance.

Human review requires reasonable understanding of:

- what the change does;
- why it exists;
- how it fits the architecture;
- important error/state/security behavior;
- how it is validated;
- and how it can be maintained later.

Compilation or passing tests alone is not sufficient acceptance criteria.

AI-assisted code must remain understandable without access to the originating AI conversation.

The human maintainer remains responsible for correctness, security, privacy, maintainability, testing, dependencies, licensing, documentation, and operational behavior.

AI-generated tests and documentation require human review appropriate to their role.

## AI Usage Transparency

Significant AI assistance will be recorded in:

`docs/AIUSAGE.md`

The record should identify the AI product/tool, model/version where known, date, affected work area, and general nature of assistance.

This is a transparency record and does not transfer responsibility away from the human maintainer.

## Build and Dependency Policy

The project will use standard Rust ecosystem tooling where practical, including Cargo, `cargo fmt`, and `cargo clippy`.

Dependencies require human review for suitability, maintenance, licensing, security, architecture support, and unnecessary transitive impact.

Exact command matrices and CI enforcement belong in development/testing documentation.

## Licensing

The project uses the MIT License unless explicitly changed later.

The full license text is kept in `LICENSE`.

Source and configuration files that support comments should include the project SPDX identifier where applicable:

`SPDX-License-Identifier: MIT`

AI assistance does not change licensing requirements.

## Sensitive Data During Development

Development and AI workflows must respect the credential and privacy boundaries from [ADR-0018](0018-security-identities-credentials-and-secret-management-policy.md).

Synthetic or sanitized data should be used when it can satisfy the development need.

Testing and fixture policy follows [ADR-0020](0020-standardized-testing-and-provider-test-data-policy.md).

## Architecture Compliance

Implementation must follow Accepted ADRs.

If implementation reveals that an Accepted ADR is no longer appropriate, it is revisited according to [ADR-0017](0017-adr-lifecycle-and-change-policy.md).

AI-generated implementation does not override architectural decisions.

## Consequences

Development remains reproducible without requiring proprietary hosted tooling.

AI assistance can be used productively while human ownership remains explicit.

The project accepts some documentation and review overhead in exchange for traceability and maintainability.

More formal collaboration processes may be introduced if the contributor base grows.
