#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Fail-closed entry checks without Docker, secrets, or real XML files.
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
script="$root/scripts/energy-import.sh"
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
bash -n "$script"
help_text="$(bash "$script" --help)"
[[ "$help_text" == *'--import-dir'* ]] || {
    echo 'Guided CLI help does not mention --import-dir' >&2
    exit 1
}
# Bad options must be rejected before filesystem or database access.
expect_failure() {
    if bash "$script" "$@" >"$tmp/output" 2>&1; then
        printf 'Unexpected successful guided import entry: %s\n' "$*" >&2
        exit 1
    fi
}
expect_failure --not-an-option
expect_failure --import-dir
expect_failure --source-id nope
printf 'Guided importer entry safeguards: PASS\n'
