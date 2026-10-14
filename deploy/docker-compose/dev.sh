#!/usr/bin/env bash
# SPDX-License-Identifier: MIT

set -euo pipefail

SCRIPT_DIR="$(
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1
    pwd
)"
cd "$SCRIPT_DIR"

exec docker compose \
    -p energy-ingest-dev \
    --env-file .env.dev \
    -f compose.dev.yml \
    exec dev \
    bash -c '
        export ENGY_DB_URL
        ENGY_DB_URL="mysql://${ENGY_DB_USER}:$(cat /run/secrets/energy_db_pwd)"
        ENGY_DB_URL="${ENGY_DB_URL}@mariadb:3306/${ENGY_DB_NAME}"
        exec bash
    '
