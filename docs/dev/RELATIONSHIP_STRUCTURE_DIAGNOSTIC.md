# Read-only Green Button relationship structure diagnostic

**Audience:** Profile developers and EDC/CCA regression-test contributors.

**Purpose:** Compare the structure of original downloads with derived fixtures,
without printing customer identifiers or resource URLs.

Run on the private original downloads:

```bash
cargo run -p fixture_privacy --bin relationship_structure -- \
  /home/pme991/energy_usage/data
```

Run on the derived repository fixtures:

```bash
cargo run -p fixture_privacy --bin relationship_structure -- \
  profiles/sdge/fixtures
```

Compare **aggregate counts**, including Atom entries, entry IDs, self links,
link relationships, and recognized Green Button resource path categories.
Do not interpret `self_link_differs_from_entry_id` as a broken relationship:
an Atom `urn:uuid:` ID and an HTTPS resource URL can validly describe the
same resource. The counts help determine whether an ESPI-specific identity
mapping is required.

This tool is offline, read-only and does not print identifier values, URLs,
or XML content. It does not sanitize files, prove semantic relationship
integrity, or grant publication approval. Avoid publishing raw XML inputs.
