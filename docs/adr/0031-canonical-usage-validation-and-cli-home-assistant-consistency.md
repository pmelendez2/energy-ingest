<!-- SPDX-License-Identifier: MIT -->
<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-20. -->

# ADR-0031: Canonical Usage Validation and CLI/Home Assistant Consistency

- **Status:** Accepted
- **Date:** 2026-10-20

## Context

Energy Ingest imports Green Button electricity and natural gas observations
from multiple files and resources. These may contain identical overlapping
intervals, later corrections, gaps, or incompatible measurements. Import
acceptance, physical-meter matching, and a report showing matching overlap
values do not by themselves certify a usable consumption stream.

Energy Ingest exposes command-line administration and analysis, an authenticated
HTTP API, and a separate Home Assistant integration (`ha_energy_usage`). If each
consumer independently selects readings, resolves overlap, derives usage totals,
or handles corrections, they can present conflicting histories for the same
physical meter. Home Assistant Recorder has its own storage and aggregation
requirements, but those requirements must not redefine accepted utility data.

This decision extends the established backend ownership and deployment boundary
in ADR-0030 and the canonical history, provenance, and gap policies in
ADR-0022. It does not replace those decisions.

## Decision

### One authoritative backend

The Rust Energy Ingest backend is the sole authority for canonical
physical-meter identity, accepted observation revisions, measurement
compatibility, stream resolution, coverage, gaps, validation outcomes, and
usage-source selection. The backend must expose shared, reusable Rust domain
functions for these decisions. CLI commands and HTTP handlers call those
functions rather than implementing independent selection or validation rules.

The `ha_energy_usage` integration consumes the backend's versioned API results.
It must not independently resolve competing Green Button resources or
reinterpret unresolved intervals as valid usage. The CLI and Home Assistant
may format or aggregate **the same validated intervals** for their respective
user interfaces, but neither may silently change the backend's chosen source,
status, units, flow direction, or interval boundaries.

### Validation contract

The backend shall provide a machine-readable, versioned contract with:

- stable physical-meter and stream identity;
- commodity, measurement kind, unit, and electricity flow direction;
- interval start and end in UTC, with explicit interval semantics;
- selected actual observations and their accepted revision/provenance;
- unambiguous validation status, including unresolved conflicts and gaps;
- deterministic issue codes usable by CLI and HA;
- a revision token or data watermark that changes when accepted data,
  corrections, matching, or validation decisions affect the result; and
- a declared schema/contract version for compatible consumers.

The representation may be split into summary, diagnostics, and paginated or
bounded interval endpoints. All views must derive from the same canonical
validation implementation and report the same underlying validation revision.
The exact endpoint paths, serialization schema, and cache strategy belong in
API specifications and implementation documentation, not this ADR.

### Conservative usage semantics

Only genuinely utility-supplied, accepted observations can contribute to a
validated consumption stream. Identical overlapping observations contribute
once; incompatible or genuinely conflicting observations remain unresolved
unless a documented backend rule deterministically resolves them. No consumer
may infer, interpolate, synthesize, or automatically fill missing intervals.

Do not merge different interval resolutions by summing overlapping data.
Unknown or reverse electricity flow must not be classified as import
consumption. Units and measurement kinds must be compatible before aggregation.
Later accepted corrections must become visible in both CLI and HA history,
including past periods, while preserving provenance and without double-counting.

Validation must distinguish imported-but-unvalidated, validated, partially
validated, and unresolved results as appropriate. A diagnostic overlap report
with no differing pairs is not sufficient evidence of complete validation.
The API must not silently present unresolved totals as authoritative.

### Home Assistant projection and synchronization

Home Assistant Recorder statistics are a derived projection, not a second
source of truth. The HA integration may transform validated backend intervals
into Home Assistant's required statistics representation, units, and display
cadence, subject to documented deterministic mapping rules. Those mappings
must not change which utility observations were selected.

The integration shall track backend revision/watermark information, reconcile
historical corrections and newly supplied observations idempotently, and
revisit affected past windows when they change. Gaps remain missing. If a
window cannot be represented safely, the integration shall withhold or flag
its derived statistics rather than fabricate usage. A successful HA write does
not retroactively establish backend validation.

CLI and HA must present customer-friendly language while preserving precise
internal issue codes. A CLI analysis report and HA status view may differ in
layout and available detail, but shared facts must agree for the same meter,
window, and backend revision.

### Testing and compatibility

Automated tests must exercise the same source fixtures through the shared Rust
validation path, CLI output, and HTTP response. Contract tests for
`ha_energy_usage` shall verify it renders the authoritative status and
intervals, correctly handles pagination/bounds and revision changes, and does
not duplicate source-selection logic.

Test scenarios include exact duplicates, overlapping cadences, conflicting
values, missing data, corrected historical observations, unit incompatibility,
flow-direction ambiguity, multiple physical meters, empty streams, and stale
HA projections. Cross-surface parity assertions shall compare meter identity,
validation state, intervals, units, provenance/revision, and eligible totals
where totals are actually valid.

Contract changes require explicit versioning and compatibility testing across
backend and HA integration releases. A temporarily unavailable or incompatible
backend must produce a clear unavailable/error state, not a stale value
misrepresented as current verified usage.

### Scope

This decision covers electricity and natural gas usage and historical trends.
It does not introduce tariff, billing, charge, credit, or solar-export monetary
calculations. Future additional commodities or measurement types require their
own compatible validation rules and API contract extension.

## Consequences

### Benefits

- CLI and HA report consistent facts at the same backend revision.
- Corrections, gaps, conflicts, and provenance are handled in one place.
- The backend can be independently audited and tested without HA.
- Home Assistant remains a replaceable presentation and statistics consumer.

### Costs and constraints

- The API requires stable schema/version handling and revision semantics.
- HA needs reconciliation logic for corrected historical windows.
- Backend and HA releases need explicit compatibility and parity tests.
- Canonical validation can block totals until ambiguity is resolved.

## Alternatives considered

1. **Duplicate stream-resolution logic in HA.** Rejected: creates divergent
   correction, conflict, and gap behavior.
2. **Treat Recorder history as canonical.** Rejected: Recorder is a derived
   presentation store and cannot replace utility provenance and revisions.
3. **Share only CLI-formatted text reports.** Rejected: loses stable,
   machine-readable identities, statuses, and revision semantics.

## Related decisions

- ADR-0022: Canonical Relational Schema, Revision History, Provenance,
  Coverage, and Gaps.
- ADR-0024: Local API Client Authentication.
- ADR-0029: Customer-First Installation and Home Assistant Experience.
- ADR-0030: Home Assistant Integration and Distributed Deployment.

## Acceptance criteria

The decision is implementable when the backend publishes a documented canonical
validation contract; CLI and API expose consistent results from the same Rust
implementation; HA consumes those results without separate stream resolution;
and tests prove parity for clean, duplicate, conflicting, corrected, and
missing-data histories. No totals are certified solely by an import or overlap
analysis report.
