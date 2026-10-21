#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Regression tests for guided commodity matching.
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT

awk '
    /^setup_kind\(\) \{/ { copy = 1 }
    copy { print }
    copy && /^}$/ { exit }
' "$root/scripts/energy-import.sh" > "$tmp/selection.sh"
grep -q '^setup_kind() {' "$tmp/selection.sh"
tail -n 1 "$tmp/selection.sh" | grep -qx '}'
bash -n "$tmp/selection.sh"
# shellcheck source=/dev/null
source "$tmp/selection.sh"

SCRIPT_DIR="$tmp"
source_custodian_id=1
TEST_ATTACH_LOG="$tmp/attached"
export TEST_ATTACH_LOG

cat > "$tmp/meter-prod.sh" <<'SCRIPT'
#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
printf '%s\n' "$*" >> "$TEST_ATTACH_LOG"
SCRIPT
chmod +x "$tmp/meter-prod.sh"

meter_rows=''
run_readonly_sql() {
    case "$1" in
        *'SELECT id, display_name FROM physical_meter'*)
            printf '%s' "$meter_rows"
            ;;
        *)
            printf 'Unexpected SQL in guided test\n' >&2
            return 1
            ;;
    esac
}
die() {
    printf 'ERROR: %s\n' "$*" >&2
    exit 1
}
# No initial confirmation should occur in setup_kind().
confirm() {
    printf 'Unexpected extra confirmation prompt\n' >&2
    return 1
}

# No resources: no query, prompt, or association.
meter_rows=$'11\tElectric\n'
setup_kind electric 1 Electricity 0 > "$tmp/output"
[[ ! -e "$tmp/attached" ]]

# One electric meter: delegate exactly once to the preview/apply CLI.
setup_kind electric 1 Electricity 6 > "$tmp/output"
grep -qx 'attach-compatible --meter 11' "$tmp/attached"
[[ "$(wc -l < "$tmp/attached")" -eq 1 ]]
grep -q \
    'Review imported data before confirming the meter match.' \
    "$tmp/output"

# Natural gas uses the same one-delegation behavior.
: > "$tmp/attached"
meter_rows=$'22\tGas\n'
setup_kind gas 7 'Natural gas' 2 > "$tmp/output"
grep -qx 'attach-compatible --meter 22' "$tmp/attached"
[[ "$(wc -l < "$tmp/attached")" -eq 1 ]]

# Multiple same-commodity meters: fail closed, never guess ownership.
: > "$tmp/attached"
meter_rows=$'11\tMain meter\n12\tSecond meter\n'
if (setup_kind electric 1 Electricity 2) > "$tmp/output" 2>&1; then
    echo 'Multiple-meter association was allowed' >&2
    exit 1
fi
[[ ! -s "$tmp/attached" ]]
grep -q \
    'No automatic meter matches will be made.' \
    "$tmp/output"

# No registered meter: fail closed.
meter_rows=''
if (setup_kind gas 7 'Natural gas' 2) > "$tmp/output" 2>&1; then
    echo 'Association with no registered meter was allowed' >&2
    exit 1
fi
[[ ! -s "$tmp/attached" ]]

# Malformed resource count: fail closed.
if (setup_kind electric 1 Electricity bad) > "$tmp/output" 2>&1; then
    echo 'Malformed count was allowed' >&2
    exit 1
fi
[[ ! -s "$tmp/attached" ]]

echo 'Guided single-meter preview and fail-closed selection: PASS'
