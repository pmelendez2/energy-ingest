#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Scan all Markdown for stale current-release claims; preserve history."""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION_RE = re.compile(r'(?<!\d)(?:v)?(\d+\.\d+\.\d+)(?!\d|\.\d)')
CURRENT_RE = re.compile(
    r'\b(?:current|latest|most recent)\s+(?:stable\s+)?release\b'
    r'|\b(?:current|latest)\s+(?:production\s+)?version\b',
    re.IGNORECASE,
)
SKIP_PARTS = {'.git', 'target', 'dist', 'node_modules'}


def check_text(text: str, version: str) -> list[tuple[int, str]]:
    errors = []
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if CURRENT_RE.search(line):
            # Allow wrapped sentences (e.g. "The current stable release is" on
            # one line and the number on the next).
            segment = ' '.join(lines[index:index + 2])
            for match in VERSION_RE.finditer(segment):
                if match.group(1) != version:
                    errors.append((index + 1, match.group(1)))
    return errors


def main() -> int:
    version = (ROOT / 'release/VERSION').read_text().strip()
    if not re.fullmatch(r'\d+\.\d+\.\d+', version):
        print('ERROR: release/VERSION must contain a stable semver')
        return 1
    count = 0
    problems = []
    for path in sorted(ROOT.rglob('*.md')):
        if any(p in SKIP_PARTS for p in path.relative_to(ROOT).parts):
            continue
        count += 1
        for line, stale in check_text(path.read_text(), version):
            problems.append(f'{path.relative_to(ROOT)}:{line}: '
                            f'current release {stale}, expected {version}')
    print(f'Checked {count} Markdown files against release/VERSION={version}')
    for msg in problems:
        print('ERROR: ' + msg)
    return int(bool(problems))


if __name__ == '__main__':
    raise SystemExit(main())
