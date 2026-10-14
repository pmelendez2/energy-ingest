#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Report imported electricity and gas data without modifying it.
set -euo pipefail
# Resolve deployment files relative to this script, not the shell cwd.
SCRIPT_DIR="$(
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd
)"
cd -- "$SCRIPT_DIR/../deploy/docker-compose"

usage() {
    cat <<'HELP'
Usage: ./analyze-prod-data.sh [--details] [--help]

Summarize imported data, coverage, and overlapping observations.
Use --details for diagnostic database resource IDs and pairwise results.
This command never modifies stored observations or meter associations.
HELP
}

case "${1:-}" in
    -h|--help) usage; exit 0 ;;
    '' ) (( $# == 0 )) || exit 2 ;;
    --details) (( $# == 1 )) || { usage >&2; exit 2; } ;;
    *) usage >&2; exit 2 ;;
esac
[[ -f .env.prod ]] || {
    echo 'Error: run setup-prod.sh first.' >&2
    exit 2
}

args=(stream analyze)
if [[ "${1:-}" == '--details' ]]; then
    args+=(--details)
fi
docker compose -p energy-ingest \
    --env-file .env.prod \
    -f compose.prod.yml \
    run --rm --no-deps admin "${args[@]}"
