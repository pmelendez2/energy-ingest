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
print(f"Release notes validated: docs/releases/{version}.md")
