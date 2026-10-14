<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0000: Record architecture decisions

- Status: Accepted
- Date: 2026-09-29

## Context

This project includes architectural decisions involving Green Button / ESPI
parsing, Data Custodian acquisition, authentication, data normalization, revision
handling, persistence, security, deployment, and interfaces to downstream
systems.

These decisions affect correctness and long-term maintainability. Their intent
should remain understandable to maintainers who were not involved when the
decision was made.

## Decision

Architecturally significant decisions will be recorded as numbered Markdown
files in `docs/adr/`.

Each ADR will contain:

- Status
- Date
- Context
- Decision
- Consequences

Accepted ADRs will not be rewritten to reflect later architectural changes.

When an accepted decision changes, a new ADR will document the new decision
and supersede the previous ADR.

Routine implementation details do not require an ADR.

## Consequences

Architectural intent and tradeoffs remain visible to future maintainers.

Changes to significant architecture require consideration of whether a new ADR
is appropriate.

Maintaining ADRs adds a small documentation cost, but avoids requiring future
maintainers to reconstruct architectural reasoning from source code and commit
history.
