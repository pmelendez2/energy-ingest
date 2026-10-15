#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Check local Markdown links and basic project doc metadata."""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]
LINK = re.compile(r"(?<!!)\[[^\]]+\]\(([^)]+)\)")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--strict-metadata", action="store_true")
    args = ap.parse_args()
    failures: list[str] = []
    warnings: list[str] = []
    paths = sorted(ROOT.rglob("*.md"))
    paths = [p for p in paths if not any(
        item in {"target", ".git", "dist", "node_modules"}
        for item in p.relative_to(ROOT).parts
    )]
    for path in paths:
        content = path.read_text(encoding="utf-8")
        rel = path.relative_to(ROOT)
        if not rel.parts[0] == "docs" and rel.parts[0] != "profiles":
            pass
        # ADRs have their own record format; templates and generated
        # changelogs are not required to use user-facing doc metadata.
        is_adr = len(rel.parts) > 1 and rel.parts[:2] == (
            "docs", "adr"
        )
        if (rel.parts[0] in {"docs", "profiles"} and not is_adr
                and rel.name != "README.md"):
            if not re.search(
                r"(?im)^(?:#{2,6}\s+Purpose\b|[^\n]*\bPurpose\s*:)",
                content,
            ):
                warnings.append(f"{rel}: missing Purpose")
            if not re.search(
                r"(?im)^(?:#{2,6}\s+(?:Intended Audience|Audience)\b|[^\n]*\b(?:Intended Audience|Audience)\s*:)",
                content,
            ):
                warnings.append(f"{rel}: missing Intended Audience")
        for raw in LINK.findall(content):
            dest = raw.strip().split()[0].strip("<>")
            if (not dest or dest.startswith((
                "#", "https://", "http://", "mailto:", "data:"
            ))):
                continue
            target = unquote(dest.split("#", 1)[0].split("?", 1)[0])
            if target and not (path.parent / target).exists():
                failures.append(f"{rel}: broken link {dest}")
    print(f"Scanned {len(paths)} Markdown files")
    for item in failures:
        print(f"ERROR: {item}")
    for item in warnings:
        print(f"WARN: {item}")
    if failures or (args.strict_metadata and warnings):
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
