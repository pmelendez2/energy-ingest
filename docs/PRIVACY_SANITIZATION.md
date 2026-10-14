<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-10. -->

# Preparing Green Button XML test fixtures

**Audience:** Developers, data contributors, profile reviewers, and maintainers.

**Purpose:** Transform real Green Button XML exports into privacy-reviewed,
reusable test fixtures **for publication in the public `energy_ingest` GitHub
repository**, so that current and future releases can run automated
regression tests against realistic utility data formats.

## Why we do this

Utilities and community choice aggregators (CCAs) can implement Green Button
XML differently. Valid regression fixtures help prevent changes to the
`energy_ingest` parser and ingestion pipeline from breaking compatibility
with electric distribution companies (EDCs) or CCAs. Real exports are useful
starting points but can expose customer, meter, location, and detailed energy
usage information. **Never publish original exports or private candidates.**

The desired end state is an independently approved, minimized, anonymized
fixture committed to public GitHub and exercised by automated tests across
future releases. A valid XML document is not necessarily a safe public fixture.

## Workflow

1. **Prepare** from local original XML. The tool samples representative
   history, pseudonymizes supported identifiers, and checks Atom relationships.
2. **Verify** against the original XML, private secret, and saved candidate
   files and reports. A mismatch fails verification.
3. **Review** the candidate XML and the private and shareable reports.
   Assess any residual customer information and identifying usage patterns.
4. **Approve** only after the authorized data provider and an independent
   project maintainer have completed the required reviews.
5. **Publish** only the explicitly approved test fixtures to the public
   GitHub repository, where continuous integration and release tests use them.

**Neither `prepare` nor `verify` authorizes publication.** The current CLI
has no automatic public-release approval command.

## Commands

The currently tested execution environment is Linux. The published XML files
are platform-independent and may be consumed by developers on Linux, macOS,
or Windows. Support for running this particular CLI on macOS or Windows is
not yet validated.

### First-time setup: privacy secret

On Linux, generate a cryptographically random secret once in a private file.
The command below will not overwrite an existing secret:

```bash
umask 077
mkdir -p ~/.config/energy_ingest
if [ ! -e ~/.config/energy_ingest/fixture_privacy_secret ]; then
    openssl rand -hex 32 > \
      ~/.config/energy_ingest/fixture_privacy_secret
fi
chmod 600 ~/.config/energy_ingest/fixture_privacy_secret
export FIXTURE_PRIVACY_SECRET="$(
    cat ~/.config/energy_ingest/fixture_privacy_secret
)"
```

Keep the same secret when verifying a prepared set. Do not print, commit,
or share the secret. The environment variable is scoped to the current
shell session; it is not stored in the repository. If the variable is
missing, the CLI error points to this section.

```bash
fixture_privacy prepare SOURCE_DIR OUTPUT_DIR
fixture_privacy verify SOURCE_DIR OUTPUT_DIR
```

`SOURCE_DIR` contains the original private XML exports. `OUTPUT_DIR` is a new,
private directory that must not already exist and must not overlap the source.

The default sampling policy retains seven days of hourly readings, seven days
of daily readings, or three monthly readings where supported, while keeping
complete shorter histories. Unknown reading frequencies fail rather than
being guessed. Selection uses XML content, not filenames. Calendar-aware
coverage and provider-specific daylight-saving interpretation still require
additional validation.

The CLI intentionally has no compatibility aliases: this project has not
released a public fixture-preparation interface yet. `verify` checks a full
prepared directory; `verify-file` checks a single XML candidate against a
single-file report. `audit`, `sanitize`, and `relationships` are diagnostics,
not alternative fixture publication workflows.

## Verification results and review reports

The CLI prints a **private per-file processing table** after `prepare`
and `verify`. The table names each XML file and reports readings found,
retained, removed, and operations performed. For example:

```text
File                               Found     Kept  Removed  Operations
----------------------------------------------------------------------------
electric-example.xml                 240      168       72  history minimized, identifiers pseudonymized, Atom graph verified
gas-example.xml                       31       31        0  history minimized, identifiers pseudonymized, Atom graph verified
2 files processed
Privacy review: REQUIRED | Publication: NOT AUTHORIZED
```

These are synthetic illustrative counts. Filenames and per-file results
are **private** and must not be included in public issues or logs.
A separate aggregate shareable report omits filenames.

The generated directory contains candidate XML and two reports:

- `verified_set_evidence.json` is **private**. It contains per-file names,
  digests, verification details, and detected risk information. Never commit
  or share it publicly.
- `shareable_review_summary.json` contains aggregate results without original
  identifiers or private source digests. It is suitable for **review**, not
  automatic publication; a reviewer must still assess contextual disclosure.

Passing relationship checks means that the implemented Atom entry/link
comparison was satisfied. It does **not** prove complete ESPI semantics,
privacy safety, or platform compatibility. Remaining risks include unknown
XML fields, detailed usage fingerprints, nested references, and incomplete
calendar interpretation. No report is a certificate of anonymity.

## Safety and ownership

Original exports must stay unchanged and outside a public Git checkout.
On **Linux**, output directories are restricted to permission mode `0700`,
and output files to `0600`. The workflow uses a private staging directory
and attempts to remove it after ordinary write failures. Power loss or
unexpected interruption can leave a staging directory that requires private
inspection. A filesystem rename alone does not guarantee safe concurrent
writers or crash durability on every filesystem.

The source data provider decides whether sanitized material may be submitted.
An independent maintainer decides whether the submitted fixture is appropriate
for public release. Automated checks supplement but cannot replace these
reviews. Follow ADR-0028 and `docs/PROFILE_RELEASES.md` for release policy.

## SDG&E observation

Ten original and ten derived SDG&E XML exports have identical aggregate
relationship statistics, including six `self` URLs associated with multiple
Atom IDs (five classified as `ReadingType`, one as `other`). This is a
suspicious but **unconfirmed** interoperability inconsistency, not a proven
provider defect. Preserve those relationships while investigating them.

## Reading the terminal results

`prepare` and `verify` display a compact table of privately processed XML
files. File names may be shortened **only in terminal output**. Numbered
rows make shortened names distinguishable; original names remain in the
private review report and on disk.

- **Found / Kept / Removed**: counts of XML interval readings, not unique
  utility measurements across overlapping downloads.
- **Min**: history-selection step was recorded. A short sample may keep all
  readings; a checkmark does not imply readings were deleted.
- **ID**: UUID pseudonymization was recorded; it does not establish that
  all personal data was removed.
- **Graph**: exact Atom relationship verification was recorded.
- **✓**: step completed; **-**: no completed operation recorded.

The review-risk count reflects the existing automated risk heuristics,
not an independent privacy clearance. Do not publish candidate XML merely
because all table columns show checkmarks.
