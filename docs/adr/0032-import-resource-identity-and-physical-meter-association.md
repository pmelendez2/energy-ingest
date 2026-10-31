<!-- SPDX-License-Identifier: MIT -->
<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-30. -->

# ADR-0032: Import Resource Identity and Physical Meter Association

- Status: Accepted
- **Date:** 2026-10-30

## Context

A successful Green Button import may introduce meter-reading resources that
are not yet associated with a physical meter. Their accepted observations
then exist in Energy Ingest but do not appear in that meter's validated usage
history or downstream Home Assistant Recorder statistics. Re-importing an
identical XML document can leave the same association work outstanding even
when the import accepts no new observations.

<a href="0025-green-button-provider-profile-identity-and-contribution-lifecycle.md">ADR-0025</a> governs provider identity, <a href="0027-secure-profile-intake-validation-and-publication.md">ADR-0027</a> governs intake validation,
<a href="0030-home-assistant-integration-and-distributed-deployment.md">ADR-0030</a> defines the Home Assistant integration boundary, and <a href="0031-canonical-usage-validation-and-cli-home-assistant-consistency.md">ADR-0031</a>
makes Energy Ingest authoritative for canonical stream resolution. The
shared transactional meter-association storage operation is already used by
administrative and HTTP interfaces. None of those decisions establishes the
resource-identity response and association workflow for each import.

## Decision

### Import identity and results

1. After importing a Green Button XML file, Energy Ingest MUST identify the meter-reading resources contained in that file and return their internal IDs.
2. Re-importing the same file MUST return the existing resource IDs, even when no new readings are added. It MUST NOT create duplicate resources.
3. Resource identities are derived from the validated document and canonical
   persisted identity model, not from a broad query for resources belonging
   to the same acquisition source, commodity, or import time range.
4. The response distinguishes import acceptance and observation outcomes
   from association outcomes. An import's success does not imply that every
   resource has been associated or that Recorder synchronization succeeded.
5. The API contract is versioned and exposes enough identity and status
   information for a client to present a safe mapping decision. Endpoint
   paths and field-level serialization belong in the API specification.

### Association authority and safety

1. Energy Ingest is the authority for physical-meter identity and persistent
   resource-to-meter association. Clients may propose a selection but may
   not bypass backend validation.
2. Existing associations are retained and reported. Repeated association
   with the same meter is idempotent; association with a different meter is
   a conflict requiring explicit resolution, not an automatic reassignment.
3. An unassociated resource may be assigned automatically only when a
   previously confirmed, persistent, unambiguous identity rule identifies
   the same physical meter. Commodity alone, display-name similarity, or
   the presence of a single candidate meter is insufficient.
4. Unknown, conflicting, or ambiguous identities remain unassociated until
   a user confirms the intended meter. The backend validates provider or
   custodian identity, commodity, and other applicable identity constraints.
5. Administrative and HTTP association paths use the same transactional
   storage operation. Concurrent attempts must not attach one resource to
   different physical meters; the database enforces the final uniqueness
   invariant.
6. Association changes must invalidate or advance relevant canonical usage
   revision information so consumers can reconcile affected history.

### Transaction and recovery boundaries

1. Canonical observation import and physical-meter association are distinct
   outcomes. A successfully committed import is not rolled back solely
   because association or downstream synchronization fails.
2. Clients can retry resource discovery and association after partial
   failure without duplicating observations.
3. The backend reports enough per-resource status to distinguish associated,
   already associated, unresolved, and conflicting outcomes. It does not
   claim downstream Home Assistant Recorder persistence.

## Consequences

- The import API and its documentation require a backward-compatible or
  explicitly versioned response extension.
- Re-import tests must verify stable resource identity, not only zero
  changes to observations.
- Database-backed tests must cover identity mismatches, idempotency,
  competing concurrent associations, and failure/retry behavior.
- An unresolved association can delay downstream visibility while preserving
  all accepted utility observations.
- No fabricated gap readings, billing calculations, or direct Home Assistant
  database access are introduced.

## Alternatives considered

- **Infer the meter from commodity or candidate count:** Rejected because
  multiple physical meters can share a commodity and utility identity.
- **Associate every resource from the same acquisition source:** Rejected
  because it can attach unrelated resources from earlier imports.
- **Treat import and Recorder synchronization as one transaction:** Rejected
  because they are independently persisted in separate systems.

## Related decisions

- <a href="0025-green-button-provider-profile-identity-and-contribution-lifecycle.md">ADR-0025</a>: Green Button provider profile identity and contribution lifecycle
- <a href="0027-secure-profile-intake-validation-and-publication.md">ADR-0027</a>: Secure profile intake, validation, and publication
- <a href="0030-home-assistant-integration-and-distributed-deployment.md">ADR-0030</a>: Home Assistant integration and distributed deployment
- <a href="0031-canonical-usage-validation-and-cli-home-assistant-consistency.md">ADR-0031</a>: Canonical usage validation and CLI/Home Assistant consistency
- `ha_energy_usage` <a href="https://github.com/pmelendez2/ha-energy-usage/blob/main/docs/adr/0001-historical-utility-usage-recorder-synchronization.md">ADR-0001</a>, <a href="https://github.com/pmelendez2/ha-energy-usage/blob/main/docs/adr/0002-utility-data-acquisition-and-synchronization.md">ADR-0002</a>, and proposed <a href="https://github.com/pmelendez2/ha-energy-usage/blob/main/docs/adr/0003-guided-physical-meter-mapping-and-import-synchronization.md">ADR-0003</a>
