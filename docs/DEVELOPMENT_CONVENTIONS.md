<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# Development Conventions

> **Status:** Project convention
>
> **Audience:** Maintainers, contributors, and AI coding assistants
>
> **Purpose:** Re-establish the conventions used to design and implement this project so that future work remains consistent even when the original development context is unavailable.
>
> **Last Updated:** 2026-10-02

## 1. How to Use This Document

Read this document before making implementation changes.

For AI-assisted work, provide this file together with the relevant ADRs, source files, migrations, and task context. Treat it as a project-level convention document, not as a substitute for the source code or ADRs.

When guidance conflicts, use this precedence:

1. Current source code and database migrations define implemented behavior.
2. Accepted ADRs define architectural intent and constraints.
3. This document defines development conventions.
4. Runbooks and operational documentation define deployment and maintenance procedures.
5. Historical discussions and generated notes are informative only.

Do not silently invent behavior that is not supported by the source code, migrations, or accepted ADRs.

---

## 2. Project Philosophy

Use a KISS approach.

Prefer:

- simple, explicit implementations;
- source code as the primary definition of behavior;
- database migrations as the primary definition of schema;
- automated tests as executable verification;
- ADRs for architectural decisions and rationale;
- runbooks for operational procedures;
- small, reviewable implementation chunks.

Avoid duplicating detailed behavior in multiple documentation files.

Documentation should explain why a structure exists, how components interact, how to operate them, and how to verify them.

---

## 3. Naming Conventions

Use names consistently across code, configuration, runtime services, and environment variables.

### Rust and configuration identifiers

Use `snake_case`.

Examples:

```text
energy_ingest
energy_store
energy_test
interval_reading
ingestion_source
```

### Runtime and service names

Use hyphenated names.

Examples:

```text
energy-ingest
energy-test
```

### Environment variables

Use uppercase snake case.

Examples:

```text
ENGY_T_DB_URL
ENGY_T_DB_PWD
ENGY_T_DB_ROOT_PWD
```

### Terminology

Prefer plain, provider-independent language in shared application and persistence layers.

Use:

- `ingestion_source`
- `source`
- `history`
- `revision`
- `coverage`
- `gap`

Avoid unnecessarily specialized or obscure terminology.

Do not use XML-specific names outside the XML parser/provider layer.

Do not use the old `provenance` terminology for the ingestion-source API.

---

## 4. Licensing

The project uses the MIT License unless explicitly changed later.

Every source file should begin with a concise SPDX header when the file format supports comments:

```text
SPDX-License-Identifier: MIT
```

For Rust:

```rust
// SPDX-License-Identifier: MIT
```

Do not add per-file copyright lines.

Keep the full MIT license text in the repository `LICENSE` file.

---

## 5. Rust Documentation Conventions

All Rust source, including tests, should be documented.

### Module-level documentation

Use `//!`.

Recommended structure:

```rust
//! Short module description.
//!
//! # Purpose
//!
//! Explain what this module is responsible for.
//!
//! # Architecture
//!
//! Reference the ADRs that materially define this module.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.
```

Use `# Maintainer`, not `# Programmer`.

### Public functions

Use `///` and include sections where applicable:

```text
# Purpose
# Inputs
# Returns
# Errors
# Notes
```

Do not add sections merely to satisfy a template. Keep documentation useful and specific.

### Tests

Document test functions with `///`.

Recommended structure:

```text
# Test Goal
# Inputs
# Expected Outcomes
```

Tests should explain the architectural behavior or invariant they prove, not merely restate that an insert or function call succeeds.

### Types and fields

Public structs, enums, newtypes, and public fields should have useful `///` documentation.

Private helpers should also be documented when their behavior is not obvious.

### Maintainer wording

Use:

```text
Project Contributors. Individual changes are tracked by Git history.
```

Do not embed personal names or private identity information in public project source unless explicitly required.

---

## 6. ADR Conventions

Accepted ADRs define architectural intent.

When implementing a feature:

1. identify the ADRs that materially apply;
2. reference them in module-level documentation or focused comments;
3. write tests that verify important ADR-defined invariants;
4. update the architecture conformance documentation when needed.

Do not scatter ADR numbers across every small helper.

Reference ADRs where they explain a meaningful architectural choice or invariant.

For persistence work, commonly relevant ADRs include:

- ADR-0004 — UTC handling;
- ADR-0005 — revision-aware imports;
- ADR-0016 — MariaDB persistence boundary;
- ADR-0018 — security, identities, and secrets;
- ADR-0019 — development environment and AI-assisted development;
- ADR-0020 — testing and Green Button test data;
- ADR-0022 — canonical schema, revisions, coverage, and gaps;
- ADR-0024 — local API client authentication.

Use the current ADR files as authoritative; this list is not a replacement for them.

---

## 7. Database and Persistence Conventions

The SQL migration files are the authoritative schema definition.

The Rust persistence layer should match the SQL types rather than inventing alternative representations.

Examples:

- MariaDB `INT UNSIGNED` -> Rust unsigned integer type appropriate to the range;
- MariaDB `BIGINT UNSIGNED` -> Rust `u64`;
- MariaDB `BIGINT` -> Rust `i64`;
- nullable columns -> Rust `Option<T>`;
- `DATETIME(6)` -> `NaiveDateTime` at the persistence boundary.

### Time

Canonical timestamps are UTC according to ADR-0004.

MariaDB `DATETIME(6)` is used so session timezone conversion does not silently alter stored values.

### Schema comments

Create table-level and field-level MariaDB comments in the original `CREATE TABLE` DDL while migrations are still pre-release and mutable.

Comments should explain:

- the purpose of the table;
- relationships represented by foreign keys;
- non-obvious numeric code fields;
- UTC timestamp meaning;
- application-controlled versus source-provided identifiers.

Keep comments concise and useful.

### Repository boundary

SQLx-specific database access belongs inside `energy-store`.

Callers should receive project-level types and `StoreError`, not raw SQLx errors.

### Typed IDs

Use small typed identifier newtypes at repository boundaries when they improve clarity and prevent mixing unrelated IDs.

Example:

```rust
pub struct IntervalReadingId(pub u64);
```

### Canonical identities

Use database uniqueness constraints as the authoritative definition of canonical identity.

Repeated imports should be idempotent where the architecture requires a stable canonical identity.

### Revision history

Do not overwrite earlier observed source values when ADR-0005 requires revision history.

Store revisions separately and retain the accepted/current revision relationship explicitly.

---

## 8. Error Handling

Translate SQLx errors at the `energy-store` persistence boundary.

Current project-level error categories include:

```text
Database
NotFound
Conflict
```

Do not invent new error variants unless a real caller needs to distinguish that condition.

Duplicate/constraint conditions that callers need to recognize should map to `StoreError::Conflict`.

Foreign-key and other database-enforced invariants may remain `StoreError::Database` unless there is a demonstrated application need for a more specific error.

Higher application/API layers should translate `StoreError` again at their own boundary.

---

## 9. Testing Conventions

Use automated tests as executable proof of behavior and architecture.

### Test data

Routine tests should use synthetic or sanitized data.

Do not depend on live utility/provider systems for normal development verification.

### Database integration tests

Integration tests may use the ephemeral `energy-test` MariaDB environment.

Assume test database contents are disposable.

Tests should be restartable and safe to rerun.

Do not rely on long-lived state in `energy-test`.

### Cleanup

Integration tests should clean up records they create when practical.

Delete in foreign-key-safe order.

When accepted/current revision references create cycles, clear those references before deleting revision rows.

### Constraint testing

Test important database invariants directly, including:

- uniqueness;
- foreign-key relationships;
- valid ranges;
- accepted-revision ownership;
- idempotent canonical identity behavior.

Do not duplicate the entire schema in Rust test logic; test the important invariants.

---

## 10. Security and Privacy

Do not commit:

- passwords;
- API tokens;
- private keys;
- passphrases;
- private IP addresses unless intentionally public/project-generic;
- usernames;
- hostnames from a private environment;
- absolute private home-directory paths;
- local SSH key paths;
- organization/domain names that are not intended for the public repository.

Secrets should be supplied at runtime through the approved secrets workflow.

Do not hardcode AGE identity paths or credential locations into public source.

Use generic environment-variable-driven configuration.

Before a public release, perform the required repository privacy/security audit.

---

## 11. AI-Assisted Development

AI-generated code must be understandable, maintainable, and reviewed by a human.

An AI assistant should be given:

1. this document;
2. relevant accepted ADRs;
3. the current source files being modified;
4. current SQL migrations when persistence is involved;
5. current project task context;
6. relevant verification output when troubleshooting.

AI output must follow the same project conventions as human-written code.

Do not accept generated code solely because it compiles.

Review for:

- architectural consistency;
- schema/type correctness;
- security and privacy;
- naming;
- error boundaries;
- documentation quality;
- unnecessary complexity;
- test coverage.

Maintain the project's AI-usage record according to project policy.

---

## 12. Verification Workflow

Before committing an implementation chunk, run the relevant focused tests first.

Then run the full project verification:

```bash
cargo fmt --all --check
scripts/verify.sh
```

The verification script is expected to cover:

- formatting;
- compile/check;
- Clippy;
- unit tests;
- integration tests;
- documentation tests;
- ADR conformance checks.

Treat successful automated ADR conformance as a coarse check only.

Human review is still required to confirm architectural intent.

---

## 13. Git and Commit Conventions

Keep commits focused and reviewable.

Do not stage unrelated working-tree changes.

Before committing:

```bash
git status --short
git diff --cached --stat
git diff --cached
```

Use commit messages that describe the implementation chunk rather than every individual file change.

Keep generated/local verification artifacts out of commits unless the project explicitly tracks them.

Use Git history for individual authorship and change history rather than embedding author names throughout source files.

---

## 14. Project Task Discipline

Review the project task list:

- after a major implementation chunk;
- before starting a new subsystem;
- before a release candidate;
- before production deployment;
- whenever prerequisites materially change.

Keep ongoing work active rather than marking an entire area complete because one milestone passed.

For example, initial integration-test coverage may be a completed milestone while database integration testing remains an active project activity.

---

## 15. Maintainer Development Environment

A maintainer setup document should complement this convention file rather than duplicate it.

Recommended separate file:

```text
docs/MAINTAINER_SETUP.md
```

That document should describe:

- required Rust toolchain;
- MariaDB/test environment;
- environment variables;
- secrets setup;
- how to deploy/reset `energy-test`;
- how to run focused tests;
- how to run `scripts/verify.sh`;
- how ADR conformance is checked;
- how to prepare AI context for project work.

This file defines **how development should be done**.

`MAINTAINER_SETUP.md` should define **how to prepare a machine and developer environment to do it**.

---

## 16. Periodic Convention Review

`DEVELOPMENT_CONVENTIONS.md` is a living maintainer-context document. Review it
periodically when meaningful project work has occurred since the **Last Updated**
date.

The goal is to preserve stable conventions that help a future human or AI
maintainer continue development in substantially the same style.

Do not add:

- routine decisions;
- temporary implementation choices;
- historical discussion that has no practical maintenance value;
- details already adequately captured by source code or ADRs.

Add or revise a convention only when it has become a stable project practice or
when the additional context would materially help future maintenance.

### Suggested AI Review Prompt

Use the following prompt when refreshing this document:

> Here is the current `DEVELOPMENT_CONVENTIONS.md`. Review our discussions and
> project work since the **Last Updated** date in this file. Determine whether
> any stable development conventions have changed or emerged that would
> materially help a future human or AI maintainer continue the project in the
> same style. Do not add routine decisions, temporary choices, or history unless
> they serve a practical maintenance purpose. Do not duplicate details already
> adequately captured by ADRs or source code. Generate a complete updated
> `DEVELOPMENT_CONVENTIONS.md` that can replace the existing file. If no
> meaningful updates are needed, return the existing document unchanged and say
> that no update is required.

After an approved update, set **Last Updated** to the date of that review.

---

## 17. Convention Changes

Change these conventions deliberately.

When a convention affects architecture, persistence boundaries, security, release policy, or another significant design decision, consider whether an ADR is required.

For smaller development-process conventions, update this document in the same commit as the convention change when practical.

Avoid undocumented convention drift.
