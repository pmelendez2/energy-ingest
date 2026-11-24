# Release notes

**Purpose:** Maintain human-reviewed, issue-linked release summaries for public
Energy Ingest releases.

**Intended Audience:** Users, release maintainers, and contributors.

Each stable release has a matching `X.Y.Z.md` file. The version must match
`release/VERSION` at the release tag. Start with `TEMPLATE.md`.

For changes that resolve GitHub issues, add each issue to the target version
milestone, link it from the implementing PR using `Fixes #NUMBER`, and include
a direct issue URL in the **Issues resolved** section of the release notes.
If there are no resolved issues, explicitly write `None`. List related PRs
separately. Issue closure on main does not mean a tagged release exists.

Before publication, review user-visible changes, compatibility and verification.
Do not publish private XML, secrets, hostnames, or personal information.

The GitHub release workflow uses the version-matched Markdown file as its
published description. It must not generate generic notes or replace an
existing release's assets.

Older notes are historical records. Do not revise an already-published release
to imply that new work was present at that time.

`scripts/check-release-notes.py` validates required sections, release identity,
and issue-link formatting. It cannot prove that an issue is fixed or has the
correct milestone; reviewers must verify those claims in GitHub.
