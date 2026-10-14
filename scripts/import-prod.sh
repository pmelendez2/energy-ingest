#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Import selected utility XML files from a private host directory.
set -euo pipefail
# Resolve deployment files relative to this script, not the shell cwd.
SCRIPT_DIR="$(
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd
)"
cd -- "$SCRIPT_DIR/../deploy/docker-compose"

usage() {
    cat <<'HELP'
Usage: ./import-prod.sh --file 'PATTERN' [OPTIONS]
       ./import-prod.sh --file '*.xml' --list-files

Options:
  -s, --source-id ID    Override automatic source selection (positive ID).
  -f, --file PATTERN    XML filename or wildcard pattern; repeatable.
  -d, --import-dir DIR  Directory containing XML files.
                        Default: ENGY_IMPORT_DIR in .env.prod,
                        or ./private-imports if unset.
      --list-files      Show matched files without importing.
  -h, --help            Show this help.

Quote patterns (for example, '*.xml') to prevent shell expansion.
A single registered source is selected automatically.
For multiple sources, explicitly supply --source-id.
Each file must belong to the selected acquisition source.
The import container reads files without modifying them.
HELP
}

die() {
    printf 'Error: %s\n' "$*" >&2
    exit 2
}

source_id=''
import_dir=''
patterns=()
list_files=false
while (( $# > 0 )); do
    case "$1" in
        -h|--help) usage; exit 0 ;;
        --list-files) list_files=true; shift ;;
        -s|--source-id|-d|--import-dir|-f|--file)
            (( $# >= 2 )) || die "Missing value for $1"
            case "$1" in
                -s|--source-id) source_id="$2" ;;
                -d|--import-dir) import_dir="$2" ;;
                -f|--file) patterns+=("$2") ;;
            esac
            shift 2 ;;
        *) die "Unknown argument: $1 (see --help)" ;;
    esac
done

if [[ -n "$source_id" ]]; then
    [[ "$source_id" =~ ^[0-9]+$ && "$source_id" =~ [1-9] ]] ||
        die 'Source ID must be a positive integer.'
fi
(( ${#patterns[@]} > 0 )) || die 'Provide at least one --file.'
[[ -f .env.prod ]] || die 'Run setup-prod.sh before importing.'

# Read the optional setting as data, without sourcing the env file.
if [[ -z "$import_dir" ]]; then
    import_dir="$(sed -n 's/^ENGY_IMPORT_DIR=//p' .env.prod |
        tail -n 1)"
    import_dir="${import_dir:-./private-imports}"
fi
[[ -n "$import_dir" ]] || die 'Empty import directory.'
if [[ "$import_dir" != /* ]]; then
    import_dir="$(pwd)/${import_dir#./}"
fi
[[ -d "$import_dir" ]] || die "Import directory missing: $import_dir"
import_dir="$(cd -- "$import_dir" && pwd -P)"

# Match against individual basenames. This avoids unsafe word splitting,
# supports spaces, and never follows directory traversal patterns.
shopt -s nullglob
candidates=("$import_dir"/*)
files=()
declare -A selected=()
for pattern in "${patterns[@]}"; do
    [[ "$pattern" != */* && "$pattern" == *.xml ]] ||
        die 'Use an .xml filename/pattern without a directory path.'
    count=0
    for path in "${candidates[@]}"; do
        name="${path##*/}"
        [[ "$name" == $pattern ]] || continue
        [[ -f "$path" && ! -L "$path" ]] || continue
        (( count += 1 ))
        if [[ -z "${selected[$name]+yes}" ]]; then
            files+=("$name")
            selected["$name"]=1
        fi
    done
    (( count > 0 )) || die "No XML files match: $pattern"
done
(( ${#files[@]} > 0 )) || die 'No regular XML files selected.'

if [[ "$list_files" == true ]]; then
    printf 'Import directory: %s\n' "$import_dir"
    if [[ -n "$source_id" ]]; then
        printf 'Source ID: %s\n' "$source_id"
    else
        printf 'Source: automatic selection at import time\n'
    fi
    printf 'Matching XML files (%s):\n' "${#files[@]}"
    printf '  %s\n' "${files[@]}"
    printf 'No files imported.\n'
    exit 0
fi

# Resolve source IDs only for real imports, never during a file preview.
# SQL returns only internal numeric IDs; no customer identifiers.
if [[ -z "$source_id" ]]; then
    source_rows="$(docker compose -p energy-ingest \
        --env-file .env.prod -f compose.prod.yml \
        exec -T mariadb sh -c '
            MYSQL_PWD="$(cat /run/secrets/energy_db_pwd)"
            export MYSQL_PWD
            mariadb --batch --skip-column-names \
                -u "$MARIADB_USER" "$MARIADB_DATABASE" \
                -e "SELECT id FROM acquisition_source ORDER BY id"
        ')" || die 'Unable to look up registered import sources.'
    mapfile -t source_ids <<<"$source_rows"
    if [[ -z "$source_rows" ]]; then
        die 'No import source registered. Run register-prod.sh first.'
    fi
    if (( ${#source_ids[@]} != 1 )); then
        die 'Multiple import sources exist; specify --source-id.'
    fi
    source_id="${source_ids[0]}"
    [[ "$source_id" =~ ^[1-9][0-9]*$ ]] ||
        die 'Invalid registered import source returned by database.'
fi

args=(--source-id "$source_id")
for file in "${files[@]}"; do
    args+=("/imports/$file")
done
ENGY_IMPORT_DIR="$import_dir" docker compose -p energy-ingest \
    --env-file .env.prod -f compose.prod.yml \
    run --rm --no-deps import "${args[@]}"
