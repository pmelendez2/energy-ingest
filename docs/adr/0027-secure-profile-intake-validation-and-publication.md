<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-08. -->

# ADR-0027: Secure Provider Profile Intake, Validation, and Publication

- Status: Accepted
- Date: 2026-10-08

## Context

`energy_ingest` uses Green Button™ provider profiles to preserve reviewed,
sanitized examples of real-world provider behavior and strengthen regression
coverage. Profiles are not an ingestion allowlist: supported XML may be
validated and ingested without a registered profile.

ADR-0025 defines provider identity, profile revisions, and collision
reconciliation. ADR-0026 defines the standalone `gb_profile` CLI, local
validation, and portable contribution packages. A contribution intake and
publication architecture is still needed for people without GitHub experience
and for experienced contributors submitting pull requests. Submissions are
untrusted, may contain customer-derived data, and may overlap or conflict.
The profile collection must also remain maintainable as it grows.

## Decision

### Contribution Channels

The default contribution path shall accept portable ZIP packages without
requiring Git or GitHub knowledge. Direct GitHub pull requests shall be an
alternative for experienced contributors. Both channels shall follow equivalent
validation and publication policies.

The architecture shall accommodate initially maintainer-mediated ZIP intake
and subsequently a hosted intake service. Public intake shall not begin until
appropriate security, privacy, abuse prevention, and operational controls exist.

### Trust and Privacy Boundaries

Untrusted submissions shall be isolated from the published repository and
privileged build or publication environments until reviewed and cleared.
Private customer data, submission archives, diagnostics, and contact information
shall not be exposed in public Git history or CI results. Submission processing
shall enforce security and resource limits and have explicit access and
retention policies.

Automated checks cannot establish that customer-derived data is safe to
publish. Public release requires affirmative human privacy review and
maintainer approval, regardless of submission channel.

### Shared Validation and Package Integrity

Standalone validation, intake validation, and CI shall reuse the same core
versioned validation logic where applicable. Results shall distinguish
standards/schema checks, application compatibility, regression coverage, and
external certification; unsupported checks shall not be represented as passed.
External Green Button Alliance validation evidence remains optional under
ADR-0026.

Each contribution package shall identify the `gb_profile` generator version,
package format, profile schema, and applicable profile revision independently.
A SHA-256 digest shall identify the exact ZIP bytes and shall be verified by
intake; fixture-level fingerprints may support content deduplication. Detached
cryptographic signatures may be supported but shall remain optional for ordinary
contributors. Neither a checksum nor a valid signature replaces independent
validation, privacy review, or trust decisions.

Contributors shall receive a submission receipt and understandable validation
outcomes. GitHub checks or an intake status service may deliver results;
optional notifications must protect contributor privacy.

### Publication and Reconciliation

Submissions shall remain independently traceable. The published profile state
shall follow ADR-0025's canonical identity, collision resolution, revision,
and concurrency rules. Independent validation may occur concurrently, but
publication must reconcile against the latest approved revision and prevent
lost updates.

The repository is authoritative for published profile contents; any private
intake registry tracks submissions rather than replacing published profile
history. Public GitHub validation of contributor-controlled content shall be
least-privileged and separated from trusted publication actions. Passing tests
alone shall never authorize publication.

### Scalability and Fixture Retention

The architecture shall support thousands of profiles without loading XML
fixtures during routine metadata discovery. It shall permit selective and full
regression validation, parallelizing independent work where appropriate.
Generated indexes and caches, if used, are rebuildable and do not supersede
profile metadata.

Published fixtures shall prioritize distinct, representative compatibility
behavior rather than retention of complete submitted datasets. Deduplication
and coverage-preserving minimization may propose changes, but removal of
accepted regression cases requires review and a recorded profile revision.
Compressed, integrity-checked fixture archives may be used when beneficial;
small readable files may remain uncompressed. Raw input, private submissions,
published fixtures, and retired material shall have distinct retention rules.

### Profile-Independent Ingestion

No published provider profile, submission registration, hosted intake service,
or GitHub workflow shall be required to ingest otherwise supported Green Button
XML. Ingestion shall depend on the application's ordinary validation,
authorization, and persistence requirements, not profile membership.

## Alternatives Considered

- **GitHub-only contributions:** Simpler to operate, but excludes contributors
  without GitHub experience.
- **Automatic publication after tests:** Faster, but disregards privacy and
  unresolved compatibility conflicts.
- **Privileged processing of untrusted contributions:** Simplifies workflows
  but violates the security boundary.
- **Retain all submitted data indefinitely:** Increases privacy, storage, and
  testing costs without proportional regression value.
- **Require provider registration before ingestion:** Turns regression profiles
  into an inappropriate provider allowlist.

## Consequences

The project gains a ZIP-first contribution path, equivalent checks for direct
pull requests, independent validation, reviewable publication, and scalable
regression fixtures. It also accepts the operational cost of private intake,
contributor feedback, secure validation, and maintainer review. These may be
introduced incrementally without changing the approved trust boundaries.

Specific archive limits, manifest fields, hashing formats, signature trust
policies, intake states, email delivery, GitHub Actions configuration,
fixture-minimization algorithms, performance budgets, and deployment details
belong in implementation specifications rather than this ADR.

## Related Decisions

- [ADR-0025: Provider Profile Identity and Contribution Lifecycle](0025-green-button-provider-profile-identity-and-contribution-lifecycle.md)
- [ADR-0026: Standalone Profile Generator and Distribution](0026-standalone-gb_profile-generator-and-distribution.md)
- [ADR-0020: Standardized Testing and Provider Test Data](0020-standardized-testing-and-provider-test-data-policy.md)
