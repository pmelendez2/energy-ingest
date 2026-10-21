#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# AI-assisted: ChatGPT / GPT-6; 2026-10-20.
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
mkdir -p "$tmp/scripts"
cp "$root/scripts/stream-validation-preflight.sh" "$tmp/scripts/"
cat > "$tmp/scripts/meter-prod.sh" <<'STUB'
#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
printf 'METER_ARGS:%s\n' "$*"
STUB
cat > "$tmp/scripts/analyze-prod-data.sh" <<'STUB'
#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
printf 'ANALYZE_ARGS:%s\n' "$*"
STUB
chmod +x "$tmp/scripts/"*.sh
out="$(bash "$tmp/scripts/stream-validation-preflight.sh")"
grep -qx 'METER_ARGS:list' <<< "$out"
grep -qx 'ANALYZE_ARGS:' <<< "$out"
grep -q 'NOT YET ESTABLISHED' <<< "$out"
out="$(bash "$tmp/scripts/stream-validation-preflight.sh" --details)"
grep -qx 'ANALYZE_ARGS:--details' <<< "$out"
if bash "$tmp/scripts/stream-validation-preflight.sh" --unsafe \
    > /dev/null 2>&1; then
    echo 'Unknown options must fail.' >&2
    exit 1
fi
if bash "$tmp/scripts/stream-validation-preflight.sh" --details bad \
    > /dev/null 2>&1; then
    echo 'Extra arguments must fail.' >&2
    exit 1
fi
# Fail closed if the authoritative data analyzer fails.
cat > "$tmp/scripts/analyze-prod-data.sh" <<'STUB'
#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
exit 17
STUB
chmod +x "$tmp/scripts/analyze-prod-data.sh"
if bash "$tmp/scripts/stream-validation-preflight.sh" \
    > /dev/null 2>&1; then
    echo 'Analysis failure must propagate.' >&2
    exit 1
fi
printf '%s\n' 'Stream validation preflight tests: PASS'
