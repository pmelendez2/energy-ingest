<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-08. -->

# AI Usage

## Purpose

This document records how AI-assisted tools were used during the design,
implementation, testing, documentation, and maintenance of this project.

The goal is transparency for maintainers, contributors, and users.

**Intended Audience:** Maintainers, contributors, and users reviewing AI provenance.

## General policy

AI-generated or AI-assisted output is not considered authoritative solely
because it was produced by an AI system.

Architectural decisions, source-code changes, documentation, tests, and
external technical claims must be reviewed and verified before acceptance.

Accepted ADRs are project decisions made by maintainers, regardless of whether
AI assisted in their discussion or drafting.

## File-level AI provenance

File-level annotations complement release-level AI history. They do not
establish authorship, copyright, verification, or maintainer approval.

New AI-assisted Markdown may use:

```markdown
<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-08. -->
```

Existing Markdown with unknown origins must not be falsely labeled
AI-generated or AI-assisted. Bulk annotation may use a neutral record:

```markdown
<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->
```

For source code, preserve the SPDX license line separately and use the
language's normal comment syntax. Record future substantive AI-assisted
changes in annotations or the applicable release history.

## Release history

| Release | AI Tool | Model | Date Range |
|---|---|---|---|
| 0.1.8 (preparation) | ChatGPT | GPT-6 | 2026-10-30 |
| 0.1.5-rc.1 | ChatGPT | GPT-6 | 2026-10-20 |
| Unreleased / 0.1.0-dev | ChatGPT | GPT-5.6 Sol | 2026-09-28 - present |

### 0.1.8 (release preparation)

* AI Tool: ChatGPT
* Model: GPT-6
* Date Range: 2026-10-30
* Maintainer Review: Required

#### Areas of AI assistance

- Acquisition-source API response and utility-company naming changes.
- Rust dependency audit integration in hooks and CI.
- Docker Hub digest and multi-platform publication verification.
- Release documentation and verification instructions.

#### Verification

The maintainer reported 209 passing tests in the isolated Docker suite,
a successful dependency audit, successful PR #31 CI and secret scans,
and a verified signed merge commit on `main`. The v0.1.8 release tag,
Docker Hub publication, native production upgrade, and Home Assistant
integration validation have not yet been completed.

### 0.1.5-rc.1

* AI Tool: ChatGPT
* Model: GPT-6
* Date Range: 2026-10-20
* Maintainer Review: Required

#### Areas of AI assistance

AI assistance was used for:

- extending production package version validation to support prerelease versions;
- making release image references configurable;
- preserving compatibility with existing stable-release packaging;
- developing the private Docker registry publication workflow;
- implementing registry authentication and tag-existence safeguards;
- developing automated regression tests for release versions and registry behavior;
- preparing private release-candidate documentation.

#### Verification

The maintainer executed the following checks:

- shell syntax validation;
- production package generation and SHA-256 verification;
- stable and prerelease version regression tests;
- private registry safety regression tests;
- Markdown documentation metadata validation;
- full isolated development verification, including 150 passing tests;
- Git whitespace and executable-permission checks.

Final verification of Docker image publication, multi-platform manifests,
private registry access, and production deployment remains pending.

#### Maintenance

The release candidate requires maintainer review, successful CI checks,
and approval before publication. The existing stable Docker Hub release
workflow remains separate from private release-candidate publication.

### Unreleased / 0.1.0-dev

* AI Tool: ChatGPT
* Model: GPT-5.6 Sol
* Date Range: 2026-09-28 - present
* Maintainer Review: Required

#### Areas of AI assistance

AI assistance has been used for:

- initial project architecture;
- Green Button / ESPI research;
- evaluation of existing open-source projects;
- provider and OAuth architecture;
- canonical data-model discussions;
- ADR planning and drafting;
- repository structure;
- test strategy and CI/CD planning;
- documentation planning;
- future Home Assistant integration planning.

#### Verification

AI-assisted conclusions should be verified against authoritative sources where
appropriate, including:

- NAESB ESPI XML schemas;
- Green Button Alliance documentation;
- provider documentation;
- upstream source repositories;
- actual test results;
- sanitized or synthetic regression fixtures.

#### Maintenance

Update this document for every project release.

Each release entry should record:

- project release/version;
- AI product/tool;
- AI model/version;
- date or date range;
- areas of AI assistance;
- relevant human review and verification.
