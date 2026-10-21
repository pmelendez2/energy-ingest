#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# AI-assisted: ChatGPT / GPT-6; 2026-10-20.
# Read-only stream discovery and evidence collection.
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
usage() {
    cat <<'HELP'
Usage: scripts/stream-validation-preflight.sh [--details]

Runs read-only resource discovery and meter inventory through the existing
production entry scripts. No totals or stream-approval decisions are made.
The current production database is not modified.
HELP
}
case "${1:-}" in
    '') [[ $# -eq 0 ]] || { usage >&2; exit 2; } ;;
    --details) [[ $# -eq 1 ]] || { usage >&2; exit 2; } ;;
    --help|-h) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
esac

for script in scripts/analyze-prod-data.sh scripts/meter-prod.sh; do
    if [[ ! -x "$root/$script" ]]; then
        echo "Missing executable production script: $script" >&2
        exit 2
    fi
done

printf '%s\n' 'Energy Usage — stream validation preflight'
printf '%s\n' '------------------------------------------'
printf '%s\n' 'Mode: read-only; no meter reassignment, no totals'
printf '\n%s\n' 'Physical meters and attached resource counts:'
"$root/scripts/meter-prod.sh" list
printf '\n%s\n' 'Imported stream coverage and overlapping readings:'
if [[ "${1:-}" == '--details' ]]; then
    "$root/scripts/analyze-prod-data.sh" --details
else
    "$root/scripts/analyze-prod-data.sh"
fi
printf '\n%s\n' 'STREAM VALIDATION: NOT YET ESTABLISHED'
printf '%s\n' 'Discovery alone cannot approve hourly/daily/monthly usage totals.'
printf '%s\n' 'Do not sum overlapping data or fill missing periods.'
