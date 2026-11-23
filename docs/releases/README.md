# Release notes

**Purpose:** Maintain human-reviewed release summaries for public Energy Ingest
releases.

**Intended Audience:** Users, release maintainers, and contributors.

Each stable release has a matching `X.Y.Z.md` file. The version must match
`release/VERSION` at the release tag.

Start with `TEMPLATE.md`, fill in user-visible changes and verification
results, and review the notes in the release pull request. Do not publish
private XML, secrets, hostnames, or personal information in release notes.

The GitHub release workflow uses the version-matched Markdown file as the
published release description. It must not silently generate generic notes or
replace an existing release's assets.

Older notes are historical records. Do not revise an already-published release
to imply that new work was present at that time.

The `scripts/check-release-notes.py` check validates a release note's required
sections and version identity before publication.
