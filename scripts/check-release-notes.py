#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Validate version-matched human-reviewed release notes."""
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
version = (ROOT / "release/VERSION").read_text(encoding="utf-8").strip()
if not re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)",
                    version):
    sys.exit("Invalid stable release/VERSION")

path = ROOT / "docs/releases" / f"{version}.md"
if not path.is_file():
    sys.exit(f"Missing version-matched release notes: {path.relative_to(ROOT)}")
content = path.read_text(encoding="utf-8")
if not content.startswith(f"# Energy Ingest {version}\n"):
    sys.exit("Release notes heading does not match release/VERSION")
sections = (
    "Highlights",
    "Fixes",
    "Compatibility and upgrade notes",
    "Verification",
    "Known limitations",
    "Installation",
)
for section in sections:
    if not re.search(rf"(?m)^## {re.escape(section)}\s*$", content):
        sys.exit(f"Missing release-notes section: {section}")
if re.search(r"\b(?:TODO|TBD|FIXME|REPLACE_WITH_PUBLISHED_VERSION)\b",
             content, re.I):
    sys.exit("Release notes contain an unfilled placeholder")
# All subsequent releases must identify resolved issues, or explicitly say
# None. Keep published notes immutable; the existing v0.1.10 notes comply.
match = re.search(
    r"(?ms)^## Issues resolved\s*\n(.*?)(?=^## |\Z)", content
)
if match is None:
    sys.exit("Missing release-notes section: Issues resolved")
issues_section = match.group(1)
if re.search(r"(?im)^\s*(?:None|[-*] None)\s*\.?\s*$",
             issues_section):
    # Do not silently allow a mixture of a None declaration and issue links.
    if re.search(r"https?://|#[0-9]+", issues_section):
        sys.exit("Issues resolved: None conflicts with issue references")
else:
    links = re.findall(
        r"\[#([1-9][0-9]*)[^\]\n]*\]\("
        r"https://github\.com/pmelendez2/energy-ingest/issues/"
        r"([1-9][0-9]*)\)", issues_section
    )
    if not links:
        sys.exit("Issues resolved must contain GitHub Markdown issue links "
                 "or explicitly say None")
    if any(label != target for label, target in links):
        sys.exit("Issue link label and target number disagree")
    if len({target for _, target in links}) != len(links):
        sys.exit("Duplicate issue links in release notes")
    if re.search(r"(?<![\w/])#[0-9]+", re.sub(
            r"\[#([0-9]+)[^\]\n]*\]\(https://github\.com/"
            r"pmelendez2/energy-ingest/issues/[0-9]+\)",
            "", issues_section)):
        sys.exit("Bare issue numbers in Issues resolved; use links")
print(f"Release notes validated: docs/releases/{version}.md")
