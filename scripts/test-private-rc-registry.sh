#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# AI-assisted: ChatGPT / GPT-6; 2026-10-20.
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/prod-rc-registry.sh
source "$root/scripts/prod-rc-registry.sh"
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
cat > "$tmp/curl" <<'MOCK'
#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail
config=''
while (($#)); do
    case "$1" in
        --config) config="$2"; shift 2 ;;
        *) shift ;;
    esac
done
[[ -f "$config" ]] || exit 12
[[ "$(stat -c '%a' "$config")" == 600 ]] || exit 13
if [[ "${FAKE_CURL_EXIT:-0}" != 0 ]]; then
    exit "$FAKE_CURL_EXIT"
fi
printf '%s' "${FAKE_HTTP_CODE:-500}"
MOCK
chmod 755 "$tmp/curl"
export PATH="$tmp:$PATH"
for code in 200 401 403 500 000; do
    if FAKE_HTTP_CODE="$code" rc_tag_is_absent \
        'registry.example:5001' energy-ingest 1.2.3-rc.1 \
        testuser testpassword >/dev/null 2>&1; then
        echo "FAIL: HTTP $code permitted publication" >&2
        exit 1
    fi
done
if FAKE_CURL_EXIT=7 rc_tag_is_absent \
    'registry.example:5001' energy-ingest 1.2.3-rc.1 \
    testuser testpassword >/dev/null 2>&1; then
    echo 'FAIL: connection error permitted publication' >&2
    exit 1
fi
FAKE_HTTP_CODE=404 rc_tag_is_absent \
    'registry.example:5001' energy-ingest 1.2.3-rc.1 \
    testuser testpassword
printf 'Private RC registry safety tests: PASS\n'
