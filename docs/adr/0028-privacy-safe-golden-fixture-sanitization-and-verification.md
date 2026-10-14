<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-09. -->

# ADR-0028: Privacy-Safe Golden Fixture Sanitization and Verification

- Status: Accepted
- Date: 2026-10-09

## Context

`energy_ingest` uses Green Button™ XML fixtures to test parsing, provider
profiles, and ingestion. Customer-supplied exports can contain identifying
references, account relationships, and distinctive usage histories. Valid XML
and passing tests do not establish that data is safe to share. ADR-0025
through ADR-0027 establish profile contribution and publication boundaries;
a consistent privacy boundary is also required.

## Decision

### Shared privacy policy across workflows

Privacy screening, sanitization, and verification shall follow one consistent,
versioned set of rules and evidence semantics across local fixture preparation,
repository review packaging, and provider-profile contributions. Workflows may
have different user interfaces or implementations but must not enforce
contradictory privacy criteria.

### Distinct operations and source protection

Auditing identifies potential disclosures without changing inputs.
Sanitization produces separate candidate outputs while preserving originals.
Verification independently evaluates candidates and their evidence. Customer
exports remain local by default; neither screening nor sanitization requires
transmitting source data to a third-party service.

### Relationship-aware transformation and validation

Transformations shall preserve required resource relationships and the
semantics needed for regression coverage. Identifiers and references that
identify or link to customers must be handled consistently across related
documents. Unknown or ambiguous fields and residual identification risks
must be surfaced rather than silently treated as safe.

### Transparent evidence and controlled disclosure

The process shall explain its privacy rules and transformations, record what
changed, and produce evidence linked to the exact candidate content and
policy version. Detailed reports may show original values and mappings to
the person reviewing their own data. Material that can reconnect published
fixtures to original customer records shall not accompany public fixtures
or contribution packages. Shareable evidence must avoid exposing source
identifiers, credentials, or sensitive replacement material.

### Data-provider responsibility and publication boundary

The data owner or authorized provider is responsible for reviewing sanitized
content and authorizing its submission. Automated privacy controls assist
that review but cannot guarantee anonymity or transfer responsibility to
the project. Submission does not guarantee acceptance or publication;
maintainers independently review contributions under ADR-0027.

Previously accepted or published fixtures may be proposed for removal
through the project's normal GitHub Pull Request process, subject to
maintainer review. Removing a fixture from the current repository does
not guarantee removal from Git history, forks, clones, or previously
distributed copies. No separate withdrawal mechanism or guaranteed
removal outcome is established by this decision.

Detailed energy consumption patterns, time ranges, and cross-file linkages
may remain identifying after direct identifiers are removed. Relevant
structural and regression validation and an explicit assessment of residual
privacy risk are prerequisites for sharing. Stale or mismatched evidence
cannot authorize publication.

## Alternatives Considered

- **Unrelated privacy checks in each workflow:** rejected because different
  rules can lead to inconsistent outcomes.
- **Regex-only substitutions:** rejected because they can damage resource
  relationships and misclassify ordinary technical data.
- **Attestation without automated assistance:** rejected because it leaves
  common exposures harder to identify consistently.
- **Automatic approval based on a clean scan:** rejected because usage
  patterns and contextual linkages can remain identifying.

## Consequences

- A shared privacy policy and evidence boundary must be maintained across
  participating workflows regardless of implementation technology.
- A data provider reviews and authorizes sharing; project maintainers
  separately decide whether to accept and publish submitted fixtures.
- Existing fixtures are not automatically approved or rewritten by this ADR.
- Technology choices, module names, CLI flags, field rules, report formats,
  and user procedures belong in implementation and operational documentation.

## Related Decisions

- ADR-0013 — Validate source data independently of provider certification
- ADR-0018 — Security, identities, credentials, and secret management
- ADR-0020 — Standardized testing and provider test data
- ADR-0025 — Provider profile identity and contribution lifecycle
- ADR-0026 — Standalone profile generator and distribution
- ADR-0027 — Secure profile intake, validation, and publication
