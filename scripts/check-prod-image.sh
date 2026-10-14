#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Read-only pre-publication checks for the local production image.
set -euo pipefail

image="${1:-energy-ingest:stage2}"
fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf 'PASS: %s\n' "$*"; }

command -v docker >/dev/null || fail 'Docker unavailable'
docker image inspect "$image" >/dev/null 2>&1 ||
    fail "Image not found: $image"

# Verify the application and entrypoints without starting MariaDB.
docker run --rm --network none --entrypoint /bin/sh \
    "$image" -ec '
    for path in \
        /usr/local/bin/energy-api \
        /usr/local/bin/energy-api-token \
        /usr/local/bin/energy-store-admin \
        /usr/local/bin/energy-store-import \
        /usr/local/bin/engy-api-prod-ep \
        /usr/local/bin/es-adm-prod; do
        test -x "$path"
    done
' || fail 'Required executables absent'
pass 'Required executables present'

# Scan the *final image filesystem* for disallowed file extensions.
# This is supplementary; it does not prove an image has no secrets.
if docker run --rm --network none --entrypoint /bin/sh \
    "$image" -ec '
    find / -type f \( -name "*.xml" -o -name "*.age" \
        -o -name ".env" -o -name "*.pem" \
        -o -name "*.sqlite" \) 2>/dev/null | grep -q .
'; then
    fail 'Final image contains XML/credential-like file paths'
fi
pass 'No selected sensitive file patterns in final filesystem'

# Avoid false assurance: shell inspection does not examine layer history.
cat <<'MSG'
Remaining release gates:
  - Inspect image layers/history and image vulnerability scan.
  - Check intended linux/amd64 and linux/arm64 manifests.
  - Test the non-root runtime and mounted-secret permissions.
  - Verify exact base-image digests and release provenance.
  - Do not push publicly until all release gates are satisfied.
MSG
