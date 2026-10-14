#!/usr/bin/env python3
# SPDX-License-Identifier: MIT

"""
Convert plain ADR references in Markdown files into relative Markdown links.

Example:

    ADR-0016

becomes:

    [ADR-0016](0016-use-mariadb-for-persistent-relational-storage.md)

The script:

- discovers ADR filenames dynamically from docs/adr;
- does not hardcode the ADR filename mapping;
- ignores fenced Markdown code blocks;
- ignores inline code;
- ignores references already inside Markdown links;
- does not link an ADR to itself;
- reports forward references to ADRs that do not yet exist;
- treats missing references at or below the highest assigned ADR as errors;
- is safe to rerun;
- defaults to dry-run mode;
- modifies files only when --write is supplied.

Usage:

    ./scripts/link_adr_references.py
    ./scripts/link_adr_references.py --write
    ./scripts/link_adr_references.py --adr-dir docs/adr
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


ADR_FILE_RE = re.compile(r"^(?P<number>\d{4})-.+\.md$")
ADR_REF_RE = re.compile(r"\bADR-(?P<number>\d{4})\b")

# Markdown links/images.
#
# This intentionally handles the normal ADR link forms used by this
# repository rather than attempting to implement a complete Markdown parser.
MARKDOWN_LINK_RE = re.compile(
    r"!?\[[^\]]*\]\([^)]+\)"
)

# Inline code delimited with one or more backticks.
INLINE_CODE_RE = re.compile(
    r"(`+)(.+?)\1"
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Convert plain ADR-NNNN references to Markdown links."
    )

    parser.add_argument(
        "--adr-dir",
        type=Path,
        default=Path("docs/adr"),
        help="ADR directory (default: docs/adr)",
    )

    parser.add_argument(
        "--write",
        action="store_true",
        help="Write changes. Without this option, perform a dry run.",
    )

    return parser.parse_args()


def build_adr_map(adr_dir: Path) -> dict[str, str]:
    """
    Build a mapping of ADR numbers to filenames.

    Example:

        {
            "0016":
                "0016-use-mariadb-for-persistent-relational-storage.md",
            "0017":
                "0017-adr-lifecycle-immutability-and-supersession-policy.md",
        }
    """

    adr_map: dict[str, str] = {}

    for path in sorted(adr_dir.glob("*.md")):
        match = ADR_FILE_RE.match(path.name)

        if match is None:
            continue

        number = match.group("number")

        if number in adr_map:
            raise RuntimeError(
                f"duplicate ADR number {number}: "
                f"{adr_map[number]} and {path.name}"
            )

        adr_map[number] = path.name

    return adr_map


def protected_ranges(line: str) -> list[tuple[int, int]]:
    """
    Return character ranges that should not be modified.

    Protected content includes:

    - existing Markdown links/images;
    - inline code.
    """

    ranges: list[tuple[int, int]] = []

    for match in MARKDOWN_LINK_RE.finditer(line):
        ranges.append(match.span())

    for match in INLINE_CODE_RE.finditer(line):
        ranges.append(match.span())

    return ranges


def is_protected(
    start: int,
    end: int,
    ranges: list[tuple[int, int]],
) -> bool:
    """
    Return True if the requested range overlaps protected Markdown content.
    """

    for protected_start, protected_end in ranges:
        if start < protected_end and end > protected_start:
            return True

    return False


def replace_references(
    line: str,
    adr_map: dict[str, str],
    current_file: Path,
) -> tuple[str, list[str], list[str]]:
    """
    Replace unlinked ADR references on one line.

    Returns:

        modified line
        list of replacements
        list of unresolved ADR numbers
    """

    ranges = protected_ranges(line)

    output: list[str] = []
    replacements: list[str] = []
    unresolved: list[str] = []

    cursor = 0

    for match in ADR_REF_RE.finditer(line):
        start, end = match.span()
        number = match.group("number")
        reference = match.group(0)

        if is_protected(start, end, ranges):
            continue

        filename = adr_map.get(number)

        if filename is None:
            unresolved.append(number)
            continue

        # Do not link an ADR's own identifier to itself.
        if filename == current_file.name:
            continue

        output.append(line[cursor:start])
        output.append(f"[{reference}]({filename})")

        replacements.append(reference)
        cursor = end

    if not replacements:
        return line, replacements, unresolved

    output.append(line[cursor:])

    return "".join(output), replacements, unresolved


def process_file(
    path: Path,
    adr_map: dict[str, str],
) -> tuple[str, int, set[str]]:
    """
    Process one Markdown ADR file.

    Fenced code blocks are copied without modification.
    """

    original = path.read_text(encoding="utf-8")

    lines = original.splitlines(keepends=True)

    output: list[str] = []
    changes = 0
    unresolved: set[str] = set()

    in_fence = False
    fence_marker: str | None = None

    for line in lines:
        stripped = line.lstrip()

        # Track ``` and ~~~ fenced blocks.
        if stripped.startswith("```") or stripped.startswith("~~~"):
            marker = stripped[:3]

            if not in_fence:
                in_fence = True
                fence_marker = marker

            elif marker == fence_marker:
                in_fence = False
                fence_marker = None

            output.append(line)
            continue

        if in_fence:
            output.append(line)
            continue

        new_line, replacements, line_unresolved = replace_references(
            line,
            adr_map,
            path,
        )

        changes += len(replacements)
        unresolved.update(line_unresolved)

        output.append(new_line)

    return "".join(output), changes, unresolved


def classify_unresolved_references(
    unresolved_refs: dict[Path, set[str]],
    highest_adr: int,
) -> tuple[dict[Path, set[str]], dict[Path, set[str]]]:
    """
    Divide unresolved ADR references into two categories.

    Future references:
        ADR number is greater than the highest ADR currently present.

        These are allowed because the referenced ADR may simply not have
        been created yet.

    Missing existing references:
        ADR number is less than or equal to the highest ADR currently
        present, but no corresponding ADR file exists.

        These are considered errors because they may indicate a typo,
        deleted ADR, or broken ADR sequence.
    """

    missing_existing: dict[Path, set[str]] = {}
    future_refs: dict[Path, set[str]] = {}

    for path, numbers in unresolved_refs.items():
        for number in numbers:
            target = int(number)

            if target > highest_adr:
                future_refs.setdefault(path, set()).add(number)
            else:
                missing_existing.setdefault(path, set()).add(number)

    return missing_existing, future_refs


def print_reference_group(
    heading: str,
    references: dict[Path, set[str]],
) -> None:
    """
    Print grouped ADR references by source file.
    """

    if not references:
        return

    print(heading)

    for path, numbers in sorted(
        references.items(),
        key=lambda item: item[0].name,
    ):
        refs = ", ".join(
            f"ADR-{number}"
            for number in sorted(numbers)
        )

        print(f"  {path.name}: {refs}")

    print()


def main() -> int:
    args = parse_args()

    adr_dir = args.adr_dir.resolve()

    if not adr_dir.is_dir():
        print(
            f"ERROR: ADR directory does not exist: {adr_dir}",
            file=sys.stderr,
        )
        return 1

    try:
        adr_map = build_adr_map(adr_dir)
    except RuntimeError as exc:
        print(
            f"ERROR: {exc}",
            file=sys.stderr,
        )
        return 1

    if not adr_map:
        print(
            f"ERROR: no ADR Markdown files found in {adr_dir}",
            file=sys.stderr,
        )
        return 1

    highest_adr = max(
        int(number)
        for number in adr_map
    )

    print(f"ADR directory: {adr_dir}")
    print(f"Discovered {len(adr_map)} ADR files")
    print(f"Highest assigned ADR: ADR-{highest_adr:04d}")
    print()

    total_changes = 0

    unresolved_refs: dict[Path, set[str]] = {}

    changed_files: list[
        tuple[Path, str, int]
    ] = []

    for path in sorted(adr_dir.glob("*.md")):
        new_content, changes, unresolved = process_file(
            path,
            adr_map,
        )

        if unresolved:
            unresolved_refs[path] = unresolved

        if changes == 0:
            continue

        changed_files.append(
            (
                path,
                new_content,
                changes,
            )
        )

        total_changes += changes

        action = (
            "UPDATE"
            if args.write
            else "WOULD UPDATE"
        )

        print(
            f"{action}: {path.name}: "
            f"{changes} reference(s)"
        )

    print()

    missing_existing, future_refs = classify_unresolved_references(
        unresolved_refs,
        highest_adr,
    )

    print_reference_group(
        "Future/planned ADR references:",
        future_refs,
    )

    print_reference_group(
        "ERROR: references to missing existing ADRs:",
        missing_existing,
    )

    if args.write:
        for path, content, _ in changed_files:
            path.write_text(
                content,
                encoding="utf-8",
            )

        print(
            f"Updated {len(changed_files)} file(s), "
            f"{total_changes} reference(s)."
        )

    else:
        print(
            f"Dry run: {len(changed_files)} file(s) would change, "
            f"{total_changes} reference(s)."
        )

        if changed_files:
            print()
            print(
                "Run again with --write to apply the changes."
            )

    if future_refs:
        print()
        print(
            "NOTE: Future ADR references remain unchanged until "
            "the referenced ADR files are created."
        )
        print(
            "Rerun this script after creating those ADRs to "
            "convert the references automatically."
        )

    if missing_existing:
        print()
        print(
            "ERROR: one or more references point to ADR numbers "
            "that should already exist.",
            file=sys.stderr,
        )
        return 2

    return 0


if __name__ == "__main__":
    sys.exit(main())
