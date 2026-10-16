<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-15. -->

# Historical Usage: Development and Verification

## Scope

`history.py` is a pure Python validation and normalization boundary.
It does **not** backfill Home Assistant Recorder, connect to MariaDB,
or alter the Energy Dashboard. It consumes `resolved` physical-meter
intervals from the Rust API and rejects unresolved or inconsistent data.

Electricity code 1, UOM 72 (Wh) is normalized to kWh. Gas code 7,
UOM 169 (therm) preserves the source multiplier to produce therm.
Do not infer gas hourly buckets from the daily SDG&E reading timestamp;
verify provider period semantics before statistics aggregation.

## Run checks

From the `ha_energy_usage` repository root:

```bash
python3 -m compileall -q custom_components/ha_energy_usage/history.py
python3 -m unittest discover -s tests -p 'test_history.py' -v
```

## Required next integration increment

1. Implement a **bounded** stream API with stable pagination or revision
   watermark, server-side selection, and explicit conflict information.
2. Define the statistics bucket/time convention and correction rules.
3. Add a persistent, idempotent synchronization cursor and reconciliation.
4. Add Recorder integration using a Core 2026.10.1-compatible API.
5. Test integration lifecycle, restart, errors, and historical correction
   on a separate Home Assistant instance before use on `piha`.

Reference: [Home Assistant Recorder statistics API changes](https://developers.home-assistant.io/blog/2025/10/16/recorder-statistics-api-changes/).
