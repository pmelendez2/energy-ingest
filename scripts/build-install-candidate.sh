#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Build a private ARM64 installation candidate; never publish an image.
set -euo pipefail

usage() {
    cat <<'HELP'
Usage: scripts/build-install-candidate.sh [OUTPUT_DIRECTORY]

Builds a local linux/arm64 production image and saves it with an
installation ZIP whose image setting points to that local image.
Requires Docker Buildx, Python 3, zip utilities and sha256sum.
Does not push images, modify running containers, or copy files to Pi.
HELP
}
if [[ ${1:-} == '--help' ]]; then usage; exit 0; fi
[[ $# -le 1 ]] || { usage >&2; exit 2; }
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
out="${1:-$root/dist/install-review-0.1.4}"
mkdir -p "$out"
out="$(cd "$out" && pwd)"
image='energy-ingest:pre-0.1.4-arm64'
archive='energy-ingest-install-review-0.1.4.zip'
image_tar='energy-ingest-pre-0.1.4-arm64.tar'

for program in docker python3 sha256sum unzip; do
    command -v "$program" >/dev/null || {
        echo "Missing required program: $program" >&2
        exit 1
    }
done
if [[ -e "$out/$archive" || -e "$out/$image_tar" ]]; then
    echo 'Output already exists; use a new or empty output directory.' >&2
    exit 1
fi

echo 'Validating normal customer production package...'
bash "$root/scripts/check-prod-package.sh" 0.1.4

echo 'Building local ARM64 release image (not publishing)...'
docker buildx build --platform linux/arm64 --load \
    -f "$root/docker/Dockerfile.prod" \
    -t "$image" "$root"
arch="$(docker image inspect "$image" --format '{{.Architecture}}')"
[[ "$arch" == arm64 ]] || {
    echo "Expected arm64, got: $arch" >&2
    exit 1
}

tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
bash "$root/scripts/package-prod-release.sh" 0.1.4 "$tmp"

# Rewrite only the installer example's image reference. The official
# release package builder remains unchanged for subsequent publication.
python3 - "$tmp/energy-ingest-prod-0.1.4.zip" \
    "$out/$archive" "$image" <<'PY'
# SPDX-License-Identifier: MIT
import pathlib
import sys
import zipfile

src, dst, image = map(str, sys.argv[1:])
key = 'deploy/docker-compose/.env.prod.example'
with zipfile.ZipFile(src) as original, zipfile.ZipFile(
    dst, 'w', compression=zipfile.ZIP_DEFLATED
) as candidate:
    for info in original.infolist():
        data = original.read(info.filename)
        if info.filename == key:
            lines = data.decode('utf-8').splitlines()
            found = False
            for index, line in enumerate(lines):
                if line.startswith('ENGY_APP_IMAGE='):
                    lines[index] = 'ENGY_APP_IMAGE=' + image
                    found = True
            if not found:
                raise SystemExit('Image variable missing from installer')
            data = ('\n'.join(lines) + '\n').encode('utf-8')
        candidate.writestr(info, data)
with zipfile.ZipFile(dst) as check:
    lines = check.read(key).decode('utf-8').splitlines()
    if 'ENGY_APP_IMAGE=' + image not in lines:
        raise SystemExit('Review installer image mismatch')
    if 'scripts/energy-import.sh' not in check.namelist():
        raise SystemExit('Guided importer missing')
print('Pre-release installer image and guided importer: PASS')
PY

echo 'Saving the exact local ARM64 image...'
docker save -o "$out/$image_tar" "$image"
(
    cd "$out"
    sha256sum "$archive" "$image_tar" > SHA256SUMS
    sha256sum -c SHA256SUMS
    unzip -tq "$archive" >/dev/null
)
cat > "$out/READ_ME_FIRST.txt" <<'NOTES'
LOCAL REVIEW ONLY — NOT AN OFFICIAL v0.1.4 RELEASE

Copy the installation ZIP, ARM64 image tar, and SHA256SUMS together.
On piha, before installation, verify the files with:
    sha256sum -c SHA256SUMS
Then load the locally built pre-release image:
    docker load -i energy-ingest-pre-0.1.4-arm64.tar
Extract the ZIP into a FRESH installation directory and follow
its docs/INSTALL_LINUX.md and docs/INSTALLATION.md.

Do not use this bundle to upgrade or overwrite an existing database.
Use only on a disposable/clean Energy Ingest installation after
explicitly deciding how existing data will be handled.
The image name energy-ingest:pre-0.1.4-arm64 is local only.
No GitHub/Docker Hub publication occurs in this process.
NOTES
printf '\nCandidate artifacts are ready in: %s\n' "$out"
