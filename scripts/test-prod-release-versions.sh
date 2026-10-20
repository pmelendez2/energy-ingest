#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

generator='scripts/package-prod-release.sh'
validator='scripts/check-prod-package.sh'
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT

stable='1.2.3'
rc='1.2.4-rc.1'
registry='example.invalid:5001'
rc_image="$registry/energy-ingest:$rc"

expect_fail() {
    if "$@" >"$tmp/rejected.log" 2>&1; then
        echo "Unexpected success: $*" >&2
        exit 1
    fi
}

echo 'Checking stable package.'
env -u ENGY_RELEASE_IMAGE \
    bash "$validator" "$stable" >/dev/null

echo 'Checking prerelease package.'
ENGY_RELEASE_IMAGE="$rc_image" \
    bash "$validator" "$rc" >/dev/null

echo 'Checking embedded image.'
ENGY_RELEASE_IMAGE="$rc_image" \
    bash "$generator" "$rc" "$tmp" >/dev/null

python3 - "$tmp/energy-ingest-prod-$rc.zip" \
    "$rc_image" <<'PY'
# SPDX-License-Identifier: MIT
import sys
import zipfile

archive, expected = sys.argv[1:]
with zipfile.ZipFile(archive) as package:
    data = package.read(
        "deploy/docker-compose/.env.prod.example"
    ).decode()

if f"ENGY_APP_IMAGE={expected}" not in data.splitlines():
    raise SystemExit("Wrong image embedded in package")
PY

echo 'Checking invalid version rejection.'
for version in \
    0.1 \
    0.1.5-rc.0 \
    0.1.5-rc.foo \
    0.1.5-rc.1-extra \
    0.1.5-rc.1/invalid
do
    expect_fail env -u ENGY_RELEASE_IMAGE \
        bash "$generator" "$version" "$tmp"
    expect_fail env -u ENGY_RELEASE_IMAGE \
        bash "$validator" "$version"
done

echo 'Checking image/version mismatch rejection.'
expect_fail env ENGY_RELEASE_IMAGE="$rc_image" \
    bash "$generator" "$stable" "$tmp"
expect_fail env ENGY_RELEASE_IMAGE="$rc_image" \
    bash "$validator" "$stable"

echo 'Release version regression tests: PASS'
