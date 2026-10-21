<!-- SPDX-License-Identifier: MIT -->
<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-21. -->

# Checking Historical Usage for Changes

- **Purpose:** Explain how Energy Ingest decides which periods of previously imported electricity or natural gas data need to be checked again.
- **Intended Audience:** Energy Ingest and Home Assistant developers, including contributors who are new to historical data synchronization.

## Why this is needed

A utility may publish a reading late or correct a reading it supplied earlier.
For example, data downloaded in October may include a correction to a reading
from September. We want Home Assistant to eventually show the accepted data
without counting the same usage twice or making up missing readings.

Energy Ingest includes a small planning tool that helps identify periods that
need to be retrieved again. It **does not** update Home Assistant by itself.

## Definitions

| Term | Plain-English meaning |
| --- | --- |
| **Calendar date** | A day selected on a calendar, such as October 3, 2026. A customer does not enter hours or timestamps to select that day. |
| **Date range** | A start date and an end date, with both calendar dates included. For example, October 1–7 includes October 1, October 7, and every day in between. |
| **Reading** | An electricity or natural gas usage observation received from the utility. Electricity readings may cover an hour; gas readings may cover a day. |
| **Historical data** | Readings for earlier dates, including readings imported before the latest download. |
| **Correction** | A later utility-provided observation that changes what was previously recorded for a period. |
| **Missing data (gap)** | A period for which no accepted utility reading is available. Missing does **not** mean zero usage. |
| **Validated / resolved** | Energy Ingest can consistently select the available readings for the period. This does **not** guarantee that every hour or day has a reading. |
| **Unresolved** | Energy Ingest cannot safely select the readings, for example because there are conflicting observations. |
| **Fingerprint** | A short digital identifier calculated from a validated result. Comparing old and new fingerprints helps detect a changed result; the identifier is not the usage value itself. |
| **Refresh** | Retrieve the latest validated readings for the affected period. This planning step does not itself rewrite Home Assistant history. |
| **Application** | Software using Energy Ingest's results, such as its command-line tools or the Home Assistant integration. |

## What customers select versus what the software stores

A customer selects **calendar dates**, just as on the SDG&E download page.
There is no need to enter a precise timestamp.

For example, a customer might select:

- **Start date:** October 1, 2026
- **End date:** October 7, 2026

Both days are included in the requested data.

Inside Energy Ingest, readings already carry precise times supplied in the
utility data. The software uses these times to prevent overlapping periods
from being counted twice and to compare the same period on later checks.
That technical representation is called a **window** in the existing Rust code.
In this document, we usually call it a **date range** or **period**.

**Developer note:** An internal window is defined by an exact start and end
instant, not necessarily by whole calendar days. Its start is included and
its end is excluded. For instance, a request covering October 1–7 may be
represented internally as starting October 1 at midnight and ending at the
start of October 8, after applying the applicable time-zone rules. Internal
window boundaries are represented in UTC (Coordinated Universal Time).
Handling local calendar dates correctly, including daylight-saving changes,
requires explicit conversion policy; the planner does not perform it.

## How Energy Ingest decides what needs another check

The planning function, `energy_store::plan_history_refresh`, compares a
fingerprint saved during an earlier check with a fingerprint from the latest
validated result for the **same period**.

It returns one of these results:

| Result | What it means | What happens next |
| --- | --- | --- |
| **`unchanged`** | The previous and current fingerprints match. | The application can skip fetching the same readings again. |
| **`refresh`** | This is the first check, or the validated result changed. | The application should retrieve the latest validated readings for that period. |
| **`blocked`** | The current readings cannot be safely resolved. | Do not publish replacement usage statistics from this unresolved result. Investigate or check again later. |

### Example: A corrected electricity reading

1. Energy Ingest previously checked October 1–7 and recorded a fingerprint.
2. The utility later supplies an accepted correction for October 3.
3. Energy Ingest checks the same period again. Its validated result has a
   different fingerprint.
4. The planner returns **`refresh`** for that period.
5. A future synchronization service can retrieve the revised readings and
   update the affected Home Assistant history according to its own safety
   checks. The current planner does not perform that update.

If no relevant validated data changed, the result is **`unchanged`**. If the
utility supplied conflicting readings that Energy Ingest cannot resolve, the
result is **`blocked`**. A previously blocked period that later becomes
resolved needs a **`refresh`**.

A blocked result does not automatically delete readings already shown in Home
Assistant. How to handle previously published statistics is a separate
implementation decision.

## Important safeguards

- **Missing readings stay missing.** The planner never estimates, fills in,
  or treats gaps as zero usage.
- **No double counting.** Which readings are authoritative is decided by
  Energy Ingest's existing validation logic, not by this planner or by Home
  Assistant independently.
- **An uncertain period stays blocked.** In the Rust representation, an
  unresolved current period has no fingerprint (`None`). A previous
  fingerprint cannot make that period valid.
- **Compare like with like.** The old and current checks must cover exactly
  the same set of internal periods. The periods cannot overlap, be repeated,
  or have an ending time earlier than or equal to their starting time.
  Otherwise the planner stops with an error instead of guessing.
- **Only trusted results count.** A fingerprint supplied by another
  application does not prove that the underlying readings were validated.
  The inputs must come from Energy Ingest's validated period resolver.

## What is not implemented yet

This is a **planning tool**, not a complete historical synchronization system.
It does not select readings, compute consumption totals, calculate bills,
identify every affected date range automatically, or write to Home Assistant.

The planner is not yet connected to a CLI command or API endpoint. There is
no persistent synchronization cursor, guaranteed consistent snapshot across
multiple periods, automatic change discovery, or Home Assistant Recorder
writer. Applications still need to request specific bounded periods and
check them again later.

The existing `/v1/meters/{id}/window` API allows at most **31 days per
request**. This planner does not change that limit. Splitting selected
calendar dates into internal periods, including the effects of local time
and daylight-saving transitions, needs separate policy and tests.

## Developer verification

Format Rust code before running the full isolated test suite:

```bash
cargo fmt --all
cargo fmt --all -- --check
bash deploy/docker-compose/test.sh
```

The planner's tests cover first-time checks, no-change cases, revisions,
previously blocked periods that become resolved, currently blocked periods,
incorrect boundaries, overlapping or repeated periods, and mismatched sets
of periods.
