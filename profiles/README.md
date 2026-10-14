<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-08. -->

# Green Button™ Provider Profiles

## Why Profiles Exist

**Profiles improve regression coverage; they are not a prerequisite for
 ingesting otherwise supported Green Button data.**

`energy_ingest` accepts supported XML independently of whether its energy
provider has a registered profile. Profiles preserve representative, reviewed
XML behavior as regression fixtures. They help detect changes to parser and
ingestion behavior across providers, commodities, reporting intervals, and
schema variations without becoming a provider whitelist.

## Structure and Scope

Every immediate subdirectory of `profiles/` is a tested provider
configuration. Each contains exactly one `profile.toml`, a `fixtures/`
directory, and an optional explanatory `README.md`. The TOML manifest
identifies the provider relationship and defines human-reviewed expectations.
Provider names are descriptive and do not determine parsing eligibility.

The initial `sdge/` profile covers examples of electricity and natural gas
from San Diego Gas & Electric. The model is intended to encompass other
energy data custodians, aggregators, and suppliers, without assuming a
California-specific CCA relationship.

## Running Commands

From the **`energy_ingest` repository root**, developers can run:

```bash
cargo run -p greenbutton -- list
cargo run -p greenbutton -- validate-all
```

A standalone executable requires neither Cargo nor a repository checkout.
Run it from the root of a local workspace containing `profiles/`, or use
`--workspace DIR` explicitly:

```bash
gb_profile --workspace /path/to/workspace list
gb_profile --workspace /path/to/workspace init example
gb_profile --workspace /path/to/workspace validate example
```

The `init` command creates an empty profile; it is not validated until reviewed
fixtures and expectations are registered. The `add` command accepts a separate
expectation TOML and requires `--reviewed`, which acknowledges review but does
not sanitize customer information.

## Profile Contributions

Do not commit or package original customer XML. Inspect it privately, create
representative sanitized fixtures, review relationships and expected values,
and validate the profile. Once reviewed, the standalone CLI can create a ZIP
and SHA-256 checksum:

```bash
gb_profile --workspace /path/to/workspace \
  package example example-profile.zip
```

The archive contains a generator/version manifest and profile files; it is
not proof of privacy review or independent certification. Package creation
is an initial contributor capability. Automated sanitization, XSD checks,
Ed25519 signatures, public intake, and publication remain separate planned
work governed by accepted ADR-0025 through ADR-0027.

Maintainers run the repository's full regression and database-backed tests
before accepting submitted fixtures. Accepted profile updates preserve
historical compatibility coverage and are reviewed for sensitive information.

## Trademark Notice

Green Button and associated marks belong to their respective trademark
holders, including the Green Button Alliance and/or U.S. government agencies.
This project does not claim Green Button Alliance certification or endorsement.
