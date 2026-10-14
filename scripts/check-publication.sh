#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

if [[ -n "$(git status --porcelain)" ]]; then
  echo 'ERROR: commit or discard local changes before publishing.' >&2
  exit 1
fi

# Exclude explicitly private artifacts even if a developer force-added them.
if git ls-files | grep -Ei \
  '(^|/)(secrets|private|scratch|tmp)/|(^|/)\.env($|\.[^.]*$)|\.(age|pem|key|p12|pfx|sqlite|db|zip)$' \
  | grep -Ev '\.env\.[^.]+\.example$'; then
  echo 'ERROR: prohibited tracked paths (inspect above).' >&2
  exit 1
fi

if ! command -v docker >/dev/null; then
  echo 'ERROR: Docker is required to scan Git history.' >&2
  exit 1
fi

echo 'Scanning local Git history for secrets...'
docker run --rm \
  -v "$PWD:/work:ro" -w /work \
  ghcr.io/gitleaks/gitleaks:v8.30.1 \
  git --redact=100 .

echo 'Checking whitespace...'
git diff HEAD --check

echo 'Publication checks passed. Review commits before pushing.'
