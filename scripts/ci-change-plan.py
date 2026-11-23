#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Conservatively classify PR files for the stable isolated CI check."""
from __future__ import annotations

import argparse
import subprocess
from pathlib import PurePosixPath


def classify(status_paths: list[tuple[str, list[str]]]) -> str:
    if not status_paths:
        return 'full'
    paths = [p for _, group in status_paths for p in group]
    if any(not status or status[0] not in 'ACDMRT'
           for status, _ in status_paths):
        return 'full'
    if any(not p or p.startswith('/') or '..' in PurePosixPath(p).parts
           for p in paths):
        return 'full'
    # Changes to documentation content only. Unknown and deleted paths are
    # deliberately treated as full; renames are checked on both ends.
    def documentation(p: str) -> bool:
        return p.endswith('.md') or (
            p.startswith('docs/') and p.endswith('.txt')
        )

    def workflow(p: str) -> bool:
        return p.startswith('.github/workflows/') and p.endswith(('.yml', '.yaml'))

    if all(documentation(p) for p in paths):
        return 'docs'
    if all(workflow(p) for p in paths):
        return 'workflow'
    return 'full'


def parse_diff(data: bytes) -> list[tuple[str, list[str]]]:
    fields = data.decode('utf-8').split('\0')
    if fields and fields[-1] == '':
        fields.pop()
    out = []
    i = 0
    while i < len(fields):
        status = fields[i]
        i += 1
        count = 2 if status[:1] in ('R', 'C') else 1
        if i + count > len(fields):
            raise ValueError('malformed git diff name/status output')
        paths = fields[i:i + count]
        i += count
        out.append((status, paths))
    return out


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument('--base', required=True)
    parser.add_argument('--head', required=True)
    parser.add_argument('--github-output')
    args = parser.parse_args()
    diff = subprocess.check_output([
        'git', 'diff', '--name-status', '-z', '--find-renames',
        f'{args.base}...{args.head}',
    ])
    plan = classify(parse_diff(diff))
    print(f'CI plan: {plan}')
    if args.github_output:
        with open(args.github_output, 'a', encoding='utf-8') as file:
            file.write(f'plan={plan}\n')


if __name__ == '__main__':
    main()
