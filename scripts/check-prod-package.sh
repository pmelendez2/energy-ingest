#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Validate the actual customer installation archive before publication.
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
[[ $# -le 1 ]] || {
    echo "Usage: $0 [VERSION]" >&2
    exit 2
}
version="${1:-$(<"$root/release/VERSION")}"
[[ "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-rc\.[1-9][0-9]*)?$ ]] || {
    printf 'Expected version X.Y.Z or X.Y.Z-rc.N\n' >&2
    exit 2
}
image="${ENGY_RELEASE_IMAGE:-docker.io/pmelendez2/gb-energy-ingest:$version}"
[[ "$image" == *":$version" ]] || exit 2
out="$(mktemp -d)"
trap 'rm -rf -- "$out"' EXIT
bash "$root/scripts/package-prod-release.sh" "$version" "$out"
archive="$out/energy-ingest-prod-${version}.zip"
( cd "$out" && sha256sum -c "${archive##*/}.sha256" )
python3 - "$archive" "$version" "$image" <<'PY'
# SPDX-License-Identifier: MIT
import pathlib
import subprocess
import sys
import zipfile

archive = pathlib.Path(sys.argv[1])
version = sys.argv[2]
image = sys.argv[3]
required = {
    'docs/INSTALLATION.md',
    'docs/INSTALL_LINUX.md',
    'docs/CONFIGURATION.md',
    'docs/SCRIPT_COMMANDS.md',
    'docs/PRODUCTION_DATABASE.md',
    'scripts/energy-import.sh',
    'scripts/setup-prod.sh',
    'scripts/verify-prod.sh',
    'scripts/prod-api-common.sh',
    'scripts/internal/import-files.sh',
    'scripts/register-prod.sh',
    'scripts/meter-prod.sh',
    'scripts/analyze-prod-data.sh',
    'deploy/docker-compose/compose.prod.yml',
    'deploy/docker-compose/compose.api-lan.yml',
    'deploy/docker-compose/.env.prod.example',
    'deploy/docker-compose/bootstrap-prod.sh',
    'deploy/docker-compose/10-runtime-grants.sh',
    'deploy/docker-compose/prod-secrets/.gitignore',
}
with zipfile.ZipFile(archive) as z:
    names = z.namelist()
    if len(names) != len(set(names)):
        raise SystemExit('Duplicate ZIP members detected')
    if set(names) != required:
        raise SystemExit(
            f'Package file mismatch: missing={sorted(required-set(names))}, '
            f'extra={sorted(set(names)-required)}'
        )
    for name in names:
        path = pathlib.PurePosixPath(name)
        if path.is_absolute() or '..' in path.parts:
            raise SystemExit(f'Unsafe ZIP member: {name}')
        info = z.getinfo(name)
        mode = (info.external_attr >> 16) & 0o777
        expected = 0o755 if name.endswith('.sh') else 0o644
        if mode != expected:
            raise SystemExit(f'Unexpected permissions for {name}: {mode:o}')
        if name.endswith('.sh'):
            contents = z.read(name)
            if not contents.startswith((b'#!/usr/bin/env bash',
                                        b'#!/bin/sh')):
                raise SystemExit(f'Invalid shell script header: {name}')
            subprocess.run(['bash', '-n'], input=contents, check=True)
    settings = z.read('deploy/docker-compose/.env.prod.example').decode()
    expected_image = 'ENGY_APP_IMAGE=' + image
    if expected_image not in settings.splitlines():
        raise SystemExit('Package refers to wrong production image version')
    for doc in ('docs/INSTALLATION.md', 'docs/INSTALL_LINUX.md'):
        content = z.read(doc).decode()
        if (doc.endswith('INSTALLATION.md')
                and 'energy-import.sh' not in content):
            raise SystemExit('Guided importer omitted from installation guide')
    print(f'Package inventory and permissions: PASS ({len(names)} files)')
PY
printf 'Customer production package: PASS\n'
