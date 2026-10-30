#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
if [[ "${1:-}" == --install ]]; then
  if ! cargo audit --version >/dev/null 2>&1; then
    cargo install --locked cargo-audit
  fi
fi
if ! cargo audit --version >/dev/null 2>&1; then
  echo 'ERROR: cargo-audit missing. Install: cargo install --locked cargo-audit' >&2
  exit 1
fi
cargo audit --file Cargo.lock --deny warnings
