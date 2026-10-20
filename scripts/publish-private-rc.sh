#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# AI-assisted: ChatGPT / GPT-6; 2026-10-20.
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
# shellcheck source=scripts/prod-rc-registry.sh
source scripts/prod-rc-registry.sh

usage() {
    echo 'Usage: publish-private-rc.sh X.Y.Z-rc.N' >&2
    exit 2
}
[[ $# -eq 1 ]] || usage
version="$1"
pattern='^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.'
pattern+='(0|[1-9][0-9]*)-rc\.[1-9][0-9]*$'
[[ "$version" =~ $pattern ]] || usage

registry="${ENGY_RC_REGISTRY:-tecoreg.tecocllc.home.arpa:5001}"
repository="${ENGY_RC_REPOSITORY:-energy-ingest}"
[[ "$registry" =~ ^[a-zA-Z0-9.-]+:[0-9]+$ ]] || usage
[[ "$repository" =~ ^[a-z0-9]+([._/-][a-z0-9]+)*$ ]] || usage
image="$registry/$repository:$version"
export ENGY_RELEASE_IMAGE="$image"

[[ "$(git branch --show-current)" == main ]] || {
    echo 'Publication requires main.' >&2
    exit 1
}
[[ -z "$(git status --porcelain)" ]] || {
    echo 'Publication requires a clean worktree.' >&2
    exit 1
}
git fetch origin main
[[ "$(git rev-parse HEAD)" == "$(git rev-parse origin/main)" ]] || {
    echo 'Local main must match origin/main.' >&2
    exit 1
}

# This HTTP registry carries Basic credentials in cleartext over the LAN.
# Read secrets interactively; do not commit or print them.
read -r -p 'Registry username: ' registry_user
read -r -s -p 'Registry password: ' registry_password
printf '\n'
[[ -n "$registry_user" && -n "$registry_password" ]] || {
    echo 'Registry credentials are required.' >&2
    exit 1
}
rc_tag_is_absent "$registry" "$repository" "$version" \
    "$registry_user" "$registry_password" || exit 1
unset registry_password

bash scripts/test-prod-release-versions.sh
bash scripts/test-private-rc-registry.sh
bash scripts/check-prod-package.sh "$version"
mkdir -p dist
bash scripts/package-prod-release.sh "$version" dist

docker buildx inspect --bootstrap
docker buildx build \
    --platform linux/amd64,linux/arm64 \
    --file docker/Dockerfile.prod \
    --tag "$image" \
    --provenance=mode=min \
    --sbom=true \
    --push .
docker buildx imagetools inspect "$image"
echo "Published: $image"
