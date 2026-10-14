# ADR-0025: Green Button Provider Identity, Relationships, and Contribution Lifecycle

<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-08. -->
<!-- Maintainer review required; see docs/AIUSAGE.md. -->

- Status: Accepted
- Date: 2026-10-08

## Context

`energy_ingest` maintains tested Green Button/ESPI provider profiles with
sanitized golden fixtures. A profile describes observed compatibility for an
Energy Data Custodian (EDC) and its relevant participating providers,
such as a community aggregator, retail supplier, or distribution utility.
Community Choice Aggregation (CCA) is one jurisdiction-specific arrangement,
not a mandatory identity component. Profiles improve regression coverage; they are not a
prerequisite for ingesting otherwise supported Green Button data.

Future contributors may generate profiles outside the source repository and
submit portable ZIP packages through a controlled intake process. Experienced
contributors may submit pull requests directly. Multiple submissions can refer
to the same providers, overlap in coverage, contain duplicates, disagree in
expectations, or arrive concurrently. Directory names and submission IDs alone
cannot safely determine whether contributions represent new profiles.

The project needs stable identity, non-destructive reconciliation, and an
approval boundary before these submission mechanisms are implemented.

## Decision

### Profile and Provider Identity

A published profile represents a canonical tested provider configuration.
Identity is based on stable, project-maintained provider identifiers, their
relevant roles, and relationships, not on names, jurisdiction-specific
terminology, or contributor-selected directory names. An organization can
fulfill more than one role. Only relationships material to tested data behavior
contribute to canonical profile identity.

CCA is one supported role or arrangement, not a mandatory field. Other
arrangements, including municipal aggregation and competitive retail supply,
must be representable without modifying generic Green Button parsing.

Provider roles and relationships are normalized before comparison. Where a
relationship comprises multiple participants, ordering cannot create distinct
identities. Matching must not rely on free-text name similarity. Canonical
identity rules are defined in a separate versioned profile specification.

Published profiles continue to use one `profiles/<profile-id>/profile.toml`
per profile. A published profile ID is stable even if a provider is renamed.
Former names become aliases and are not competing identities. Mergers,
successorship, and materially changed relationships require explicit identity
review rather than automatic aliasing. Provider identity, profile identity,
and submission identity remain distinct.

An explicit compatibility variant may be created only when evidence shows
that otherwise matching provider configurations require meaningfully distinct
validation behavior that cannot reasonably coexist in one profile. Variants
are deliberately assigned and reviewed, not generated from collisions.

### Profile Revisions and Standards Compatibility

Every accepted change to the published profile's metadata, fixture corpus,
or compatibility contract advances a monotonically increasing integer profile
revision. Duplicate submissions and unrelated repository commits do not
automatically advance it. Each published revision is traceable to the exact
Git state and an accompanying human-readable profile changelog.

Profile revision, the project's profile metadata `schema_version`, and the
external Green Button/ESPI specification version are independent identifiers.
Compatibility records distinguish features observed in fixtures, features
validated by project tests, and externally established certification claims.
Successful parsing does not establish compliance or certification with an
entire ESPI version.

A provider name change normally updates metadata and aliases in the same
profile. New XML elements, different intervals, and changed Green Button
structures normally extend fixture coverage in a new profile revision; they
do not automatically create a new profile. Historical fixtures representing
behavior still supported by the project are retained. Removing or retiring
coverage requires explicit review and a documented rationale.

Submissions identify the published profile revision and exact baseline Git
state they used when known. Older or stale submissions are reconciled against
the latest accepted revision and revalidated before publication. Lack of a
known baseline is not grounds for rejecting an otherwise usable standalone
submission.

### Contributions to Existing Profiles

A submission is an independent contribution, not necessarily a request to
create a new profile. An existing canonical provider configuration normally maps
to its published profile. Valid new observations or sanitized fixtures may
extend that profile rather than create a competing profile directory.

Matching provider identity must not cause automatic rejection: submissions
may cover different commodities, time ranges, intervals, structures, or other
useful cases. Equally, contributions must not silently replace approved fixtures
or alter human-reviewed expectations. Divergent observations and expectation
conflicts require explicit review.

### Submission Identity and Deduplication

Each received contribution has a separately tracked submission identity,
regardless of intake channel. The intake process preserves the relationship
between submissions, their contributors where consented, and the published
profile revisions they propose to extend.

Byte-identical package detection and fixture-content equivalence are distinct
checks. Equivalent packages or fixtures may share validation work, but a
submission must remain traceable and must receive a meaningful outcome.
Deduplication does not imply that identical provider identity means identical
fixture coverage. Content fingerprints must be calculated only from material
that is appropriate to retain for that processing stage.

### Concurrent Validation and Publication

Independent submissions may be validated concurrently. Acceptance or
publication changes affecting the same canonical profile must be reconciled
against the latest approved state and coordinated so that one submission
cannot silently overwrite another.

A submission validated against an earlier profile revision must be reassessed
when the relevant published profile changes. Publication is conditional on
validation against the final combined candidate and an unchanged expected
revision; conflicts cause reconciliation and revalidation, not last-writer-wins.
The specific locking, transaction, and queue mechanisms are implementation
details of the intake and publication system.

### Lifecycle and Acceptance Boundary

The system distinguishes receipt, validation, review, and publication.
Automated validation determines technical readiness, not permission to make
customer-derived data public. Publication requires explicit maintainer approval
and suitable privacy review. Failed or conflicting submissions can be corrected
or withdrawn without corrupting an already published profile.

The profile collection is maintained by the project, not owned exclusively by
its first submitter. Contributor attribution is optional and subject to consent.

### Submission Channels and Independence

The ordinary contributor path is local profile generation followed by a
portable submission package delivered through a controlled intake process,
without requiring GitHub knowledge. A direct GitHub pull request is an
alternative for experienced contributors. Both channels must use equivalent
validation and approval rules.

Local profile generation and validation must remain usable without an intake
service, submission registry, or database. The future hosted intake service
may maintain transactional submission state for identity, deduplication,
concurrency, and notifications. It must not be the source of truth for the
published profile contents, which remain versioned in the repository.


### Privacy Boundary

Raw Retail Customer Green Button data must not enter the public profile
repository or public CI as part of the contribution workflow. Sanitization,
validation, and automated privacy checks cannot by themselves guarantee that
customer-derived fixtures are safe to publish; explicit review is required.
Private intake storage, diagnostic retention, and notification data must be
separate from the public profile collection.

## Alternatives Considered

- **One profile per submission:** Simple intake, but duplicates provider
  configurations and fragments regression coverage.
- **Reject a submission when the provider combination exists:** Avoids naming
  collisions but discards potentially valuable additional coverage.
- **Automatically merge all passing submissions:** Faster publication but
  cannot safely resolve conflicting expectations or privacy concerns.
- **Serialize all validation:** Avoids concurrency but unnecessarily slows
  independent submissions; coordination is needed only for shared publication
  state.
- **Require GitHub pull requests:** Simplifies intake infrastructure but
  excludes contributors unfamiliar with Git or GitHub.

## Consequences

The project gains jurisdiction-independent identity, stable profile revision
history, reusable contributions, deterministic collision handling, and a
non-destructive publication boundary.

A separate contribution identity and revision-aware reconciliation add design
and operational complexity to a future hosted intake service. Normalizing
provider identifiers and evaluating compatibility variants require maintainer
judgment. The standalone profile utility can remain simple and local-first.

Exact canonical identifier rules, role vocabulary, manifest fields, fingerprint algorithms,
submission package format, lifecycle state names, API contracts, test cases,
GitHub automation, and email delivery belong in subsequent design documents
or architectural decisions rather than this ADR.

The AI provenance convention is cross-cutting and should also be documented
in the project contribution rules rather than being limited to profiles.

This decision complements [ADR-0013](0013-validate-source-data-independent-of-provider-certification.md)
and [ADR-0020](0020-standardized-testing-and-provider-test-data-policy.md).
