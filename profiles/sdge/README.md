<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-08. -->

# SDG&E Provider Profile

**Intended Audience:** Profile contributors and maintainers.

## Purpose and Context

This is the initial Green Button™ regression profile for San Diego Gas &
Electric (SDG&E), an energy data custodian. It documents tested XML patterns
for electricity and natural gas usage. It does not establish a CCA combination,
assert certification, or restrict ingestion from any other data provider.

## Golden Coverage

`profile.toml` records three reviewed golden expectations:

| Fixture | Commodity | Interval | Coverage | Readings |
|---|---|---|---|---:|
| `sdge-electric-one-day-hourly.xml` | Electricity | Hourly | 2026-10-06 | 24 |
| `sdge-electric-one-month.xml` | Electricity | Hourly | January 2026 | 744 |
| `sdge-gas-one-month.xml` | Natural gas | Daily | January 2026 | 31 |

Additional XML files in `fixtures/` are available as reference examples;
only the fixtures registered in `profile.toml` participate in the current
profile validation. The dates above describe fixture coverage, not provider
service availability or universal reporting capability.

## What Tests Establish

The golden expectations check reported commodity, flow, units, interval
lengths, reading count, and coverage. The wider repository test suite also
checks XML relationships and repeatable MariaDB ingestion. Passing tests
establish application compatibility with these examples, not compliance with
all Green Button schemas or formal Green Button Alliance certification.

## Maintaining the Profile

From the **`energy_ingest` repository root**:

```bash
cargo run -p gb_profile -- validate sdge
bash deploy/docker-compose/test.sh
```

Review all fixture changes for customer-identifying content and preserve
previously tested behavior when adding new XML characteristics. Update
manifest expectations and profile revision history with accepted changes.

## Trademark Notice

Green Button and associated marks belong to their respective trademark
holders, including the Green Button Alliance and/or U.S. government agencies.
No affiliation, certification, or endorsement is implied.
