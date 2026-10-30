#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail
if [[ $# -ne 2 ]]; then
  echo 'Usage: verify-published-image.sh IMAGE TAG_DIGEST' >&2
  exit 2
fi
image="$1"
expected="$2"
[[ "$expected" =~ ^sha256:[0-9a-f]{64}$ ]] || {
  echo 'ERROR: invalid expected digest' >&2
  exit 1
}
command -v docker >/dev/null || {
  echo 'ERROR: Docker is required' >&2
  exit 1
}
raw="$(mktemp)"
trap 'rm -f "$raw"' EXIT
# Docker Hub registry must return the manifest successfully.
docker buildx imagetools inspect --raw "$image" > "$raw"
python3 - "$raw" <<'PYIN'
import json
import sys
with open(sys.argv[1], encoding='utf-8') as handle:
    data = json.load(handle)
platforms = {
    (m.get('platform', {}).get('os'),
     m.get('platform', {}).get('architecture'))
    for m in data.get('manifests', [])
}
required = {('linux', 'amd64'), ('linux', 'arm64')}
missing = required - platforms
if missing:
    sys.exit('ERROR: published image missing platforms: ' + str(missing))
print('[OK] Registry manifest includes linux/amd64 and linux/arm64')
PYIN
# Verify that the published tag resolves to the digest emitted by build-push.
actual="$(docker buildx imagetools inspect "$image" | sed -n   's/^[[:space:]]*Digest:[[:space:]]*\(sha256:[0-9a-f]*\).*$/\1/p' | head -1)"
if [[ "$actual" != "$expected" ]]; then
  echo "ERROR: published digest mismatch: $actual != $expected" >&2
  exit 1
fi
printf '[OK] Published digest verified: %s\n' "$actual"
