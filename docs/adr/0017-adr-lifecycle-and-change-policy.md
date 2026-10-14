<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# ADR-0017: ADR Lifecycle and Change Policy

- Status: Accepted
- Date: 2026-09-30

## Context

`energy_ingest` is still in its pre-implementation architecture phase and is initially maintained by one person.

The project needs stable architectural records without creating process overhead that makes early corrections unnecessarily expensive.

ADRs should preserve decisions and rationale, not become implementation specifications, test plans, runbooks, or user manuals.

## Decision

The initial ADR lifecycle is:

`Proposed -> Accepted`

Additional lifecycle states will be introduced only when a concrete need appears.

### Proposed ADRs

A Proposed ADR may be revised, shortened, expanded, combined, or abandoned while architecture is still being developed.

Git history preserves that evolution.

### Accepted ADRs

Accepted means the decision has been reviewed sufficiently to guide implementation.

Before acceptance, the ADR should identify:

- the problem or context;
- the architectural decision;
- important boundaries or invariants;
- significant alternatives;
- and major consequences or tradeoffs.

### Pre-Implementation Revision

During the current pre-implementation phase, an Accepted ADR may be reopened when review discovers:

- a wrong assumption;
- unnecessary complexity;
- excessive implementation detail;
- duplication;
- inconsistent terminology;
- or a better architectural boundary.

A substantive revision should normally:

1. change the ADR to `Proposed`;
2. retain the same ADR number;
3. revise the ADR;
4. review the result;
5. return it to `Accepted` when approved.

Minor editorial changes that do not alter the decision do not require reopening the ADR.

### Changes After Implementation or Release

Once implementation, persistent data, public APIs, or released behavior materially depend on an Accepted ADR, substantive architectural changes should normally be recorded in a new ADR rather than silently rewriting history.

Formal supersession or deprecation states can be added when first needed.

## ADR Scope

ADRs should focus on:

- why a decision is needed;
- what architectural choice was made;
- important boundaries and invariants;
- major tradeoffs;
- and significant alternatives.

ADRs should not simultaneously serve as:

- implementation specifications;
- exhaustive test plans;
- operational runbooks;
- security checklists;
- command references;
- release procedures;
- or user manuals.

Implementation details belong in code and design documentation.

Testing details belong in testing documentation and automated tests.

Operational details belong in deployment/operations documentation.

## Identification and Git History

ADR numbers are stable and must not be reused or renumbered merely because an earlier ADR is revised or abandoned.

Git history is the record of how ADR text evolved during the pre-implementation phase.

## Review Cadence

ADR review is event-driven rather than scheduled.

Review is appropriate when implementation planning, architecture review, or new evidence exposes a conflict, missing boundary, or unnecessary complexity.

## Governance

During the sole-maintainer phase, explicit maintainer review and status change are sufficient approval.

Pull requests, committees, or multiple approvers are not required merely for process ceremony.

## Consequences

The architecture can be corrected while change is still inexpensive.

ADR numbers and Git history remain stable.

The project avoids both premature immutability and indefinite rewriting of decisions already relied upon by released behavior.

The maintainer must still exercise judgment about whether a change is editorial or architectural.
