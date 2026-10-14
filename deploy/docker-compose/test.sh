#!/usr/bin/env bash
# SPDX-License-Identifier: MIT

set -euo pipefail

SCRIPT_DIR="$(
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1
    pwd
)"
cd "$SCRIPT_DIR"

PROJECT="energy-ingest-test"
PROJECT_ROOT="$(cd ../.. && pwd)"
ENGY_REPO_ROOT="$(git -C "$PROJECT_ROOT" rev-parse --show-toplevel)"
ENGY_PROJECT_RELATIVE="$(
    realpath --relative-to="$ENGY_REPO_ROOT" "$PROJECT_ROOT"
)"
export ENGY_REPO_ROOT
export ENGY_PROJECT_RELATIVE
TMP_DIR="$(mktemp -d)"

cleanup() {
    printf 'Cleaning up ephemeral test environment...\n'
    docker compose \
        -p "$PROJECT" \
        -f compose.test.yml \
        down -v --remove-orphans >/dev/null 2>&1 || true
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT HUP INT TERM

generate_secret() {
    local path="$1"

    if command -v openssl >/dev/null 2>&1; then
        openssl rand -hex 32 >"$path"
    else
        python3 - "$path" <<'PY'
import secrets
import sys

with open(sys.argv[1], "w", encoding="utf-8") as handle:
    handle.write(secrets.token_hex(32))
PY
    fi

    chmod 600 "$path"
}

umask 077
generate_secret "$TMP_DIR/db_pwd"
generate_secret "$TMP_DIR/root_pwd"

export ENGY_TEST_DB_NAME=energy_ingest_test
export ENGY_TEST_DB_USER=energy_ingest_test
export ENGY_TEST_DB_PWD_FILE="$TMP_DIR/db_pwd"
export ENGY_TEST_DB_ROOT_PWD_FILE="$TMP_DIR/root_pwd"

ENGY_TEST_DB_PWD="$(cat "$TMP_DIR/db_pwd")"
export ENGY_TEST_DB_PWD

ENGY_TEST_DB_ROOT_PWD="$(cat "$TMP_DIR/root_pwd")"
export ENGY_TEST_DB_ROOT_PWD

ENGY_DB_ADMIN_URL="mysql://root:${ENGY_TEST_DB_ROOT_PWD}"
ENGY_DB_ADMIN_URL+="@mariadb:3306/${ENGY_TEST_DB_NAME}"
export ENGY_DB_ADMIN_URL

ENGY_T_DB_URL="$ENGY_DB_ADMIN_URL"
export ENGY_T_DB_URL

ENGY_DB_URL="mysql://${ENGY_TEST_DB_USER}:${ENGY_TEST_DB_PWD}"
ENGY_DB_URL+="@mariadb:3306/${ENGY_TEST_DB_NAME}"
export ENGY_DB_URL

docker compose \
    -p "$PROJECT" \
    -f compose.test.yml \
    build test

printf 'Starting ephemeral test database...\n'
docker compose \
    -p "$PROJECT" \
    -f compose.test.yml \
    up -d --wait mariadb

printf 'Running verification container...\n'
docker compose \
    -p "$PROJECT" \
    -f compose.test.yml \
    run --rm --no-deps test

printf 'Verification completed successfully.\n'
