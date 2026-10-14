#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
#

"""Check lightweight ADR conformance evidence.

This tool does not prove architectural correctness.

It verifies that:
- Accepted ADRs are represented in the architecture conformance index.
- ADR references in the conformance index refer to existing ADRs.
- referenced implementation, test, and documentation paths exist.
- obvious conformance-index drift is reported before changes are committed.

Human review remains responsible for determining whether the implementation
actually satisfies the architectural intent of an ADR.
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
ADR_DIR = REPO_ROOT / "docs" / "adr"
CONFORMANCE_FILE = REPO_ROOT / "docs" / "ARCHITECTURE_CONFORMANCE.md"

ADR_FILENAME_RE = re.compile(r"^(?P<number>\d{4})-.*\.md$")
ADR_REF_RE = re.compile(r"\bADR-(?P<number>\d{4})\b")
STATUS_RE = re.compile(
    r"^- Status:\s*(?P<status>.+?)\s*$",
    re.MULTILINE | re.IGNORECASE,
)

PATH_RE = re.compile(r"`(?P<path>[^`\n]+)`")


@dataclass(frozen=True)
class Adr:
    number: str
    path: Path
    status: str

    @property
    def reference(self) -> str:
        return f"ADR-{self.number}"


def load_adrs() -> dict[str, Adr]:
    adrs: dict[str, Adr] = {}

    if not ADR_DIR.is_dir():
        raise RuntimeError(f"ADR directory does not exist: {ADR_DIR}")

    for path in sorted(ADR_DIR.glob("*.md")):
        match = ADR_FILENAME_RE.match(path.name)
        if match is None:
            continue

        text = path.read_text(encoding="utf-8")
        status_match = STATUS_RE.search(text)

        status = (
            status_match.group("status").strip()
            if status_match is not None
            else "UNKNOWN"
        )

        number = match.group("number")

        adrs[number] = Adr(
            number=number,
            path=path,
            status=status,
        )

    return adrs


def accepted_adrs(adrs: dict[str, Adr]) -> dict[str, Adr]:
    return {
        number: adr
        for number, adr in adrs.items()
        if adr.status.casefold() == "accepted"
    }


def repository_path_exists(raw_path: str) -> bool:
    """Return whether a backtick-delimited repository path exists.

    Ignore strings that clearly are not repository-relative paths.
    """

    raw_path = raw_path.strip()

    if not raw_path:
        return True

    if raw_path.startswith(("http://", "https://")):
        return True

    if raw_path.startswith(("ADR-", "cargo ", "python", "./")):
        return True

    if " " in raw_path:
        return True

    candidate = REPO_ROOT / raw_path

    return candidate.exists()


def check_conformance_index(
    adrs: dict[str, Adr],
    accepted: dict[str, Adr],
) -> tuple[list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []

    if not CONFORMANCE_FILE.is_file():
        errors.append(
            "Missing docs/ARCHITECTURE_CONFORMANCE.md"
        )
        return errors, warnings

    text = CONFORMANCE_FILE.read_text(encoding="utf-8")

    referenced_numbers = set(ADR_REF_RE.findall(text))

    for number, adr in accepted.items():
        if number not in referenced_numbers:
            errors.append(
                f"{adr.reference} is Accepted but is not represented "
                "in docs/ARCHITECTURE_CONFORMANCE.md"
            )

    for number in sorted(referenced_numbers):
        if number not in adrs:
            errors.append(
                f"ARCHITECTURE_CONFORMANCE.md references unknown ADR-{number}"
            )

    for match in PATH_RE.finditer(text):
        raw_path = match.group("path")

        if "/" not in raw_path:
            continue

        if not repository_path_exists(raw_path):
            warnings.append(
                f"Referenced path does not exist: {raw_path}"
            )

    return errors, warnings


def print_report(
    adrs: dict[str, Adr],
    accepted: dict[str, Adr],
    errors: list[str],
    warnings: list[str],
) -> None:
    print("ADR Conformance Review")
    print("======================")
    print()
    print(f"ADRs found:       {len(adrs)}")
    print(f"Accepted ADRs:    {len(accepted)}")
    print(f"Errors:           {len(errors)}")
    print(f"Warnings:         {len(warnings)}")
    print()

    if errors:
        print("Errors")
        print("------")
        for error in errors:
            print(f"[ERROR] {error}")
        print()

    if warnings:
        print("Warnings")
        print("--------")
        for warning in warnings:
            print(f"[WARN]  {warning}")
        print()

    if not errors and not warnings:
        print("[OK] Automated ADR conformance checks passed.")
    elif not errors:
        print("[OK] No blocking ADR conformance errors.")

    print()
    print(
        "Human review is still required to verify architectural intent."
    )


def main() -> int:
    try:
        adrs = load_adrs()
    except RuntimeError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    accepted = accepted_adrs(adrs)

    errors, warnings = check_conformance_index(adrs, accepted)

    print_report(
        adrs=adrs,
        accepted=accepted,
        errors=errors,
        warnings=warnings,
    )

    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
