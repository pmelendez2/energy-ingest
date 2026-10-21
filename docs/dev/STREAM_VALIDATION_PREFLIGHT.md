<!-- SPDX-License-Identifier: MIT -->
<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-20. -->

# Stream Validation Preflight

## Purpose

Provide a read-only, production-safe evidence collection step before
selecting utility consumption streams for Home Assistant history.

**Intended Audience:** Maintainers and developers.

## Scope

The preflight lists physical meters and reports imported resource coverage,
measurement units, cadences, and overlapping observations using existing
production entry scripts. It does not approve stream selection, generate
energy totals, alter accepted readings, or modify meter assignments.

Run on the production installation after import and physical-meter matching:

```bash
scripts/stream-validation-preflight.sh --details
```

This is a diagnostic command; do not treat exit code zero as proof that
physical-meter consumption streams have been validated. The operator must
review how overlapping hourly, daily, and monthly readings relate and select
a source only when metadata and policy support that choice.

## Safeguards

- Never infer, interpolate, or fill missing readings.
- Never add together overlapping hourly, daily, and monthly observations.
- Preserve original values, accepted corrections, and source provenance.
- Exclude ambiguous electricity flow and export from consumption totals.
- Do not expose physical meter numbers or API credentials in reports.
- Keep billing amounts, tariffs, and solar compensation out of scope.
- Historical Recorder statistics remain disabled until stream selection is
  separately validated and tested.

## Verification

```bash
bash scripts/test-stream-validation-preflight.sh
bash -n scripts/stream-validation-preflight.sh
bash -n scripts/test-stream-validation-preflight.sh
```

The regression test mocks the production entry scripts, checking argument
handling, read-only invocation, and fail-closed errors. Integration testing
against actual imported streams remains a separate acceptance milestone.
