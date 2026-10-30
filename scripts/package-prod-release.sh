#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

usage() {
  echo 'Usage: scripts/package-prod-release.sh VERSION OUTPUT_DIR' >&2
  exit 2
}

[[ $# -eq 2 ]] || usage
version="$1"
out_dir="$2"
[[ "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-rc\.[1-9][0-9]*)?$ ]] || usage
image="${ENGY_RELEASE_IMAGE:-docker.io/pmelendez2/gb-energy-ingest:$version}"
[[ "$image" == *":$version" ]] || usage
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
expected_version="$(<"$root/release/VERSION")"

if [[ "$version" != *-rc.* ]] &&
   [[ "$version" != "$expected_version" ]]; then
    printf 'ERROR: requested version %s differs from release/VERSION %s\n' \
        "$version" "$expected_version" >&2
    exit 1
fi
mkdir -p "$out_dir"
out_dir="$(cd "$out_dir" && pwd)"
archive="energy-ingest-prod-${version}.zip"
files=(
  scripts/setup-prod.sh
  scripts/prod-api-common.sh
  scripts/verify-prod.sh
  scripts/register-prod.sh
  scripts/internal/import-files.sh
  scripts/energy-import.sh
  scripts/meter-prod.sh
  scripts/analyze-prod-data.sh
  deploy/docker-compose/compose.prod.yml
  deploy/docker-compose/compose.api-lan.yml
  deploy/docker-compose/.env.prod.example
  deploy/docker-compose/10-runtime-grants.sh
  deploy/docker-compose/bootstrap-prod.sh
  deploy/docker-compose/prod-secrets/.gitignore
  docs/INSTALLATION.md
  docs/INSTALL_LINUX.md
  docs/CONFIGURATION.md
  docs/SCRIPT_COMMANDS.md
  docs/PRODUCTION_DATABASE.md
)
for path in "${files[@]}"; do
  [[ -f "$root/$path" ]] || {
    echo "Missing required release file: $path" >&2
    exit 1
  }
done

# Avoid exposing host-local runtime files, raw XML, or Git metadata.
# zipfile writes fixed timestamps for reproducible archives.
RELEASE_ROOT="$root" RELEASE_OUT="$out_dir/$archive" \
RELEASE_VERSION="$version" RELEASE_IMAGE="$image" python3 - "${files[@]}" <<'PY'
import os
import pathlib
import sys
import zipfile

root = pathlib.Path(os.environ['RELEASE_ROOT'])
target = pathlib.Path(os.environ['RELEASE_OUT'])
version = os.environ['RELEASE_VERSION']
image = os.environ['RELEASE_IMAGE']
script_paths = set(sys.argv[1:])
with zipfile.ZipFile(target, 'w', zipfile.ZIP_DEFLATED) as z:
    for name in sorted(script_paths):
        data = (root / name).read_bytes()
        if name == 'deploy/docker-compose/.env.prod.example':
            lines = data.decode().splitlines(keepends=True)
            found = False
            for i, line in enumerate(lines):
                if line.startswith('ENGY_APP_IMAGE='):
                    lines[i] = (
                        'ENGY_APP_IMAGE=' + image + '\n'
                    )
                    found = True
            if not found:
                raise SystemExit('ENGY_APP_IMAGE is missing')
            data = ''.join(lines).encode()
        info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        info.create_system = 3
        mode = 0o755 if name.endswith('.sh') else 0o644
        info.external_attr = (0o100000 | mode) << 16
        z.writestr(info, data)
print(target)
PY
(
  cd "$out_dir"
  sha256sum "$archive" > "${archive}.sha256"
  unzip -tq "$archive" >/dev/null
  sha256sum -c "${archive}.sha256"
)
