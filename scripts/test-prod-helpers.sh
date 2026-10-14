#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Validate relocated customer commands without a running database.
set -euo pipefail

SCRIPT_DIR="$(
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd
)"

for script in setup-prod.sh verify-prod.sh import-prod.sh \
    register-prod.sh analyze-prod-data.sh meter-prod.sh; do
    bash -n "$SCRIPT_DIR/$script"
    printf '[OK] %s syntax\n' "$script"
done

for script in import-prod.sh register-prod.sh \
    analyze-prod-data.sh meter-prod.sh verify-prod.sh; do
    bash "$SCRIPT_DIR/$script" --help >/dev/null
    printf '[OK] %s help\n' "$script"
done

if bash "$SCRIPT_DIR/import-prod.sh" --source-id \
    >/dev/null 2>&1; then
    echo '[FAIL] missing option value accepted' >&2
    exit 1
fi
printf '[OK] missing option value rejected\n'

# A disposable project layout tests the real relative-path resolution.
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
mkdir -p "$tmp/scripts" "$tmp/deploy/docker-compose/imports"
cp "$SCRIPT_DIR/import-prod.sh" "$tmp/scripts/"
printf 'ENGY_IMPORT_DIR=./imports\n' \
    >"$tmp/deploy/docker-compose/.env.prod"
touch "$tmp/deploy/docker-compose/imports/Gas daily.xml"
touch "$tmp/deploy/docker-compose/imports/Electric monthly.xml"
touch "$tmp/deploy/docker-compose/imports/notes.txt"

result="$(cd / && bash "$tmp/scripts/import-prod.sh" \
    --file '*.xml' --list-files)"
[[ "$result" == *'Matching XML files (2)'* ]]
[[ "$result" == *'Gas daily.xml'* ]]
[[ "$result" == *'Electric monthly.xml'* ]]

if bash "$tmp/scripts/import-prod.sh" --file 'missing-*.xml' \
    --list-files >/dev/null 2>&1; then
    echo '[FAIL] unmatched wildcard accepted' >&2
    exit 1
fi
printf '[OK] relocated wildcard preview and spaced filenames\n'

for phrase in 'Start or update Energy Usage' \
    'Refresh application containers' \
    'DELETE ALL STORED USAGE DATA' \
    'Uninstall Energy Usage'; do
    grep -Fq "$phrase" "$SCRIPT_DIR/setup-prod.sh"
done
printf '[OK] management menu text\n'
