<!-- SPDX-License-Identifier: MIT -->
<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-21. -->

# Canonical Meter Validation Contract — Initial Implementation

- **Purpose:** Define the first shared, read-only validation summary.
- **Intended Audience:** Energy Ingest and Home Assistant developers.

## Contract and consumers

ADR-0031 requires Energy Ingest to determine canonical stream validity.
This incremental implementation exposes the existing conservative resolver
through the same summary function for CLI and authenticated HTTP consumers.
It does **not** alter how source readings are selected.

- Rust: `summarize_meter_validation(meter_id, &resolution)`.
- CLI: `energy-store-admin meter validate METER_ID`.
- HTTP: `GET /v1/meters/{id}/validation` (bearer authentication required).

Both consumers expose contract version `2`, physical meter ID, status,
candidate count, selected interval count, deterministic content fingerprint,
and stable issue codes.
The CLI renders text; the API returns JSON. Both derive from the same
`energy_store::MeterValidation` and `resolve_physical_meter_stream`.

### Status semantics

- `no_data`: No observations are attached to that meter.
- `validated`: Candidates resolved without conflicts. Duplicates may
  produce fewer selected intervals than candidates.
- `unresolved`: One or more validation issues remain. No selected
  intervals or usage totals are certified by this summary.

An empty stream is **not** a successful historical validation.
A validation success is scoped to the observations currently attached;
this summary does not claim that meter coverage is complete or gap-free.
Missing intervals remain missing and are never synthesized.

### Example JSON (illustrative, not production readings)

```json
{
  "contract_version": 2,
  "physical_meter_id": 3,
  "status": "unresolved",
  "candidate_count": 2,
  "selected_interval_count": 0,
  "content_fingerprint": "<64 lowercase hex characters>",
  "issues": [
    {
      "kind": "conflicting_duplicate",
      "interval_reading_id": 7,
      "other_interval_reading_id": 6
    }
  ]
}
```

## Boundaries and planned follow-up

This summary is a first slice of ADR-0031, **not full ADR compliance**.
It does not yet include explicit gap coverage, a stream-level revision
cursor, a snapshot-consistent read, or historical HA Recorder sync.
The existing `/v1/meters/{id}/window` endpoint retains its bounded
31-day range and successful-window fingerprint, and must not be treated
as a globally ordered revision cursor. The existing `/stream` response
remains supported for current consumers. Do not derive usage totals from
candidate counts or infer missing readings. No database changes are made.

Run `bash deploy/docker-compose/test.sh` in the isolated repository test
environment before publishing. Runtime parity should subsequently be
verified with the same physical meter ID using both CLI and HTTP APIs.

## Content fingerprint (contract v2)

`content_fingerprint` is a deterministic SHA-256 digest of the meter ID,
number of candidates, resolved intervals (including accepted revision IDs),
and unresolved issue codes and source interval IDs. Source ordering does not
change this digest. CLI and API format the same canonical field.

A changed fingerprint means the **validation result** changed, not which
historical range changed. A stable fingerprint is not proof that unselected
source observations have not changed. It is **not** a monotonic cursor, a
transactionally consistent database snapshot, a gap-coverage certificate,
or permission to build historical Recorder totals. Clients must not use it
as a substitute for window fingerprints or periodic re-fetching. Later
work must add a durable change feed or snapshot-consistent cursor and
an affected-window reconciliation protocol before automating history sync.
