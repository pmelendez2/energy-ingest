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
| Unreleased / 0.1.0-dev | ChatGPT | GPT-5.6 Sol | 2026-09-28 - present |

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
