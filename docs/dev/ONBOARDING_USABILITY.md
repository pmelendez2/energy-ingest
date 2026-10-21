<!-- SPDX-License-Identifier: MIT -->
<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-20. -->

# Guided Import Usability

**Purpose:** Explain how guided meter registration and meter matching
work when importing electricity and natural-gas readings.

**Intended Audience:** Maintainers and developers.

## Meter registration

The guided importer collects a friendly meter name and an actual utility
meter number, then presents a single confirmation before writing either.
The meter number is sent through standard input, never a command argument.
The administrative `meter-prod.sh set-number` command retains its own
interactive confirmation for standalone use.

A failed number update must not be treated as a successful registration.
The created meter can remain in the database; maintainers must safely
reconcile it before repeating an import. Never delete usage observations
or invent a meter number to repair the partial registration.

## Import terminology

The guided UI uses *Complete utility data import* and *meter matching*.
Database/API terms such as association and internal resource IDs retain
their established meanings. Where multiple physical meters of the same
commodity exist, the application must not guess which meter owns readings.

## Validation and missing data

Successful XML import and meter matching do not themselves prove that
usage streams are valid. Use `scripts/analyze-prod-data.sh` to inspect
coverage and overlaps before relying on totals. Missing observations
remain missing; no interpolation or synthetic gap filling is permitted.

## Release and testing

Run the repository's normal isolated verification and review any guided
import test expectations affected by updated customer-facing wording.
Production packages must include the updated guided importer and helper.
