<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-08. -->

# ADR-0026: Standalone Green Button™ Profile Generator and Distribution

- Status: Accepted
- Date: 2026-10-08

## Context

`energy_ingest` uses provider profiles to retain sanitized Green Button
fixtures and regression expectations. Profiles improve regression coverage;
they are not a prerequisite for ingesting otherwise supported Green Button
data.

ADR-0025 establishes jurisdiction-independent provider identities, stable
profile revisions, independent submission identities, and non-destructive
reconciliation of contributions. The current `gb_profile` utility
is run with Cargo inside the source checkout and resolves its profile root
relative to the crate's build-time manifest directory. This is useful for
development but is not suitable as the only interface for contributors who
do not have Git, Rust, MariaDB, or an `energy_ingest` checkout.

Contributors need a local-first workflow that can inspect authorized Green
Button XML, prepare candidate profiles, review privacy-sensitive material,
and export a portable contribution package. Experienced contributors may
instead use a GitHub pull request. Hosted intake, automated notification,
and publication coordination are separate concerns.

## Decision

### Standalone Tool and Shared Validation

`gb_profile` shall be distributable as a standalone Rust command-line
program. Its normal contributor operations shall not require Cargo, the
source repository, MariaDB, a Docker daemon, or a network connection.

The standalone CLI and the repository's automated tests shall use the
same reusable profile parsing and validation logic. A profile registry is
not a prerequisite for generic Green Button parsing or ingestion.

### Distribution

Versioned prebuilt executable releases shall be the primary distribution
method for supported Linux platforms, initially x86-64 and ARM64, subject to
release validation. Releases shall include integrity verification metadata
and document platform compatibility.

An optional, versioned OCI container image may provide the same tool, with
read-only input mounts, separate writable output, non-root execution, and no
routine requirement for network access. Building from source remains
supported for developers, not required for contributors.

### Workspace Independence

The `gb_profile` CLI shall operate on selected input and output
locations, without relying on a source-checkout layout or paths embedded by
`CARGO_MANIFEST_DIR`. An unambiguous documented workspace default may be
provided. Data-bearing operations shall not silently overwrite existing
profiles, fixtures, raw inputs, or output packages.

A workspace shall separate private customer-originated inputs and transient
material from candidate profile files and exportable artifacts. Private
customer XML must not be committed or packaged. Local operations shall be
usable offline without a hosted submission registry.

### Profile Creation and Compatibility

The `gb_profile` CLI shall use the provider identity, relationship,
and profile revision model accepted in ADR-0025,
including provider roles beyond California Community Choice Aggregation.
A contribution to an existing provider configuration may add coverage rather
than creating another canonical profile. Local generation shall not assign
an authoritative new published profile identity or revision when conflicts
require later intake reconciliation.

The `gb_profile` CLI shall support explicit contributor review of
sanitized fixtures, expected results, and relevant evidence. Automated
sanitization and validation are safeguards, not guarantees that material is
safe for public release. The CLI shall distinguish structural validation,
fixture behavior validation, and human privacy review in its reporting.

### Local Standards Validation and External GBA Evidence

The `gb_profile` CLI shall support offline XML well-formedness
checks and validation against applicable, locally available ESPI XML
Schema Definition (XSD) files for explicitly supported schema versions.
Schema resources shall be versioned and distributed or installed under
documented licensing and provenance terms. Validation reports shall
identify the specific schemas and versions used. Unsupported schema
versions shall be reported as unverified, not silently treated as valid.

Local validation shall separately report XML schema conformance,
`energy_ingest` parsing compatibility, golden-fixture expectations,
and regression results. A passing parser test alone shall not imply
full ESPI conformance.

The Green Button Alliance (GBA) provides a Download My Data (DMD)
File Validator and separate formal certification testing services.
The DMD File Validator is a preliminary schema-oriented assessment;
passing it does not establish GBA certification.

The `gb_profile` CLI may associate independently obtained
GBA DMD File Validator reports with individual sanitized fixtures as
optional evidence. Such evidence shall record the source, test date,
relevant standard or server version, results, and a digest identifying
the exact fixture assessed. It shall not be represented as project
certification or proof that every provider capability was tested.

The project does not currently have a verified, supported automated
integration with the GBA validator. The standalone workflow shall not
require access to GBA services, and neither local profile creation nor
ingestion of otherwise supported Green Button data shall depend upon
external validation or certification.

External submission of customer-derived XML requires an explicit
user-initiated action and privacy review of the exact file to be
transmitted. The CLI shall not automatically upload customer XML,
including sanitized candidates, to the GBA or any other external
service. GBA validation reports must themselves be reviewed for
sensitive information before inclusion in contribution packages.

Future automated integration may be evaluated separately only if an
officially supported interface and its privacy, security, and usage
conditions are confirmed.

### Portable Contribution Packages

The `gb_profile` CLI shall export a self-contained, versioned
contribution package suitable for controlled ZIP intake or for inclusion in a pull request.
Packages shall use repository-relative profile paths and contain only
allowlisted publishable files, together with metadata needed to identify
the proposed provider configuration, tool/schema versions, compatibility
observations, and validation provenance. No raw customer inputs,
credentials, private logs, or temporary workspace files may be included.

Package generation shall check paths, sizes, contents, and repeatable
integrity metadata. Packaging is not publication approval. The recipient
shall independently validate the package and reconcile it against the
latest published profile state under ADR-0025.

### Exit Status and Automation

The CLI shall expose stable command behavior, actionable diagnostic messages,
nonzero exit status on validation failures, and a machine-readable report
format in addition to human-readable output. Local and CI checks shall be
able to apply equivalent validation rules without parsing presentation text.

### Separation from Hosted Intake

This ADR defines the standalone contributor tool and its distribution and
packaging boundaries. A future hosted ZIP intake service, GitHub Actions
workflows, contributor email notifications, submission concurrency registry,
and maintainer publication controls shall be specified separately. Neither
the CLI nor its package format may presume that a contributor has GitHub
knowledge or an account.

## Alternatives Considered

- **Require a full source checkout and Cargo:** Simple for maintainers, but
  excludes contributors without a development environment.
- **Require Docker for everyone:** Convenient on some systems, but creates an
  unnecessary runtime dependency for users who can run a native binary.
- **Use a browser-only hosted generator:** Reduces local installation but
  introduces sensitive XML transfer and continuous service operation.
- **Put submission and publication logic inside the local CLI:** Couples
  offline profile creation to hosting, authorization, and repository policy.
- **Allow packages to contain unsanitized originals for later processing:**
  Increases sensitive-data exposure and is rejected.
- **Require GBA-hosted validation on every submission:** Would depend on an
  external service and risk transmitting customer data; rejected.
- **Treat parser success as ESPI schema compliance:** Conflates application
  compatibility with standards validation; rejected.

## Consequences

Contributors can prepare reviewable provider profiles offline without a
software-development environment. Native and container distributions share
one implementation and one validation contract. The project must maintain
release builds, compatibility testing, package schema evolution, and clear
separation of private inputs from publishable fixtures.

Successful standalone validation, including optional GBA DMD validation,
does not establish provider certification,
privacy clearance, or permission to merge. Maintainer review and accepted
profile publication remain governed by ADR-0025.

Exact CLI flags, workspace defaults, package manifest fields, sanitization
techniques, target platform matrix, checksums/signatures, and onboarding
instructions belong in subsequent implementation specifications and tests.

This decision builds on [ADR-0025](0025-green-button-provider-profile-identity-and-contribution-lifecycle.md)
and complements [ADR-0013](0013-validate-source-data-independent-of-provider-certification.md)
and [ADR-0020](0020-standardized-testing-and-provider-test-data-policy.md).

## External References

- [GBA DMD File Validator](https://dmdvalidator.greenbuttonalliance.org/)
- [GBA Testing and Certification](https://www.greenbuttonalliance.org/testing)
- [GBA Certification Preparation](https://www.greenbuttonalliance.org/certification-preparation-dc)
