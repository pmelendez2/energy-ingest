<!-- SPDX-License-Identifier: MIT -->
# Sanitized XML-to-API release verification

Run `scripts/test-prod-e2e.sh` using the freshly built production image.
The script generates disposable MariaDB credentials and a disposable API token,
imports an approved sanitized SDG&E XML fixture, registers a test physical
meter, attaches compatible reading resources, and retrieves its intervals
through the authenticated HTTP API.

```bash
ENGY_TEST_IMAGE=energy-ingest:release-check \
  bash scripts/test-prod-e2e.sh
```

The default test fixture is
`profiles/sdge/fixtures/sdge-electric-one-day-hourly.xml`.
To select another **approved, sanitized** XML fixture inside that same folder,
set `ENGY_E2E_FIXTURE` to its absolute path. Never use original customer XML.

The test rejects imports that accept no observations and responses containing
no meter intervals. It never publishes the test API outside its internal Docker
network and destroys its test containers, network, and credentials on exit.

Passing this test does not establish all meter-profile correctness rules or
independently certify the underlying fixture as sanitized. Fixture review and
privacy approval remain separate release gates.
