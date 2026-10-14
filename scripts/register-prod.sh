#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Register a utility and XML acquisition source using application access.
set -euo pipefail
# Resolve deployment files relative to this script, not the shell cwd.
SCRIPT_DIR="$(
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd
)"
cd -- "$SCRIPT_DIR/../deploy/docker-compose"

usage() {
    cat <<'HELP'
Usage: ./register-prod.sh [--utility NAME] [--source NAME]
       ./register-prod.sh --help

Options:
  -u, --utility NAME  Utility name, such as 'SDG&E'.
  -s, --source NAME   Import source label.
  -h, --help          Show this help.

Missing values are requested interactively.
Imports select the source automatically when only one exists.
HELP
}

die() {
    printf 'Error: %s\n' "$*" >&2
    exit 2
}

utility=''
source=''
while (( $# > 0 )); do
    case "$1" in
        -h|--help) usage; exit 0 ;;
        -u|--utility|-s|--source)
            (( $# >= 2 )) || die "Missing value for $1"
            case "$1" in
                -u|--utility) utility="$2" ;;
                -s|--source) source="$2" ;;
            esac
            shift 2 ;;
        *) die "Unknown option: $1 (see --help)" ;;
    esac
done

[[ -f .env.prod ]] || die 'Run setup-prod.sh first.'
[[ -f prod-secrets/energy_db_pwd ]] ||
    die 'Application database credential is missing.'

if [[ -z "$utility" ]]; then
    read -r -p 'Utility name (for example, SDG&E): ' utility
fi
if [[ -z "$source" ]]; then
    read -r -p 'Source label (for example, Manual XML Download): ' source
fi
[[ -n "${utility//[[:space:]]/}" ]] || die 'Utility name is required.'
[[ -n "${source//[[:space:]]/}" ]] || die 'Source label is required.'
[[ ${#utility} -le 255 && ${#source} -le 255 ]] ||
    die 'Names must not exceed 255 characters.'

compose=(docker compose -p energy-ingest --env-file .env.prod
    -f compose.prod.yml)

# Only the application credential is mounted in the admin service.
# The custodian command returns a machine-readable numeric ID.
output="$("${compose[@]}" run --rm --no-deps admin \
    custodian create "$utility")" || die 'Utility registration failed.'
if [[ "$output" =~ Custodian\ created:\ id=([0-9]+) ]]; then
    custodian_id="${BASH_REMATCH[1]}"
else
    die 'Utility registration returned an unexpected response.'
fi

output="$("${compose[@]}" run --rm --no-deps admin \
    source create "$custodian_id" "$source")" ||
    die 'Source registration failed; utility record may exist.'
if [[ "$output" =~ Source\ (created|already\ exists):\ id=([0-9]+) ]]; then
    source_id="${BASH_REMATCH[2]}"
else
    die 'Source registration returned an unexpected response.'
fi

printf '\nUtility registered: %s\n' "$utility"
printf 'Import source:      %s\n' "$source"
printf 'Source ID:          %s\n' "$source_id"
printf '\nNormal imports select the only registered source automatically.\n'
