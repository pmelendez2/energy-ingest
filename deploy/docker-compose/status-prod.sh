#!/usr/bin/env bash
# SPDX-License-Identifier: MIT

set -euo pipefail

SCRIPT_DIR="$(
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1
    pwd
)"
cd "$SCRIPT_DIR"

PROJECT="energy-ingest"
ENV_FILE=".env.prod"
SECRETS_DIR="prod-secrets"
DB_SECRET="${SECRETS_DIR}/energy_db_pwd"
ROOT_SECRET="${SECRETS_DIR}/energy_db_root_pwd"
DB_VOLUME="${PROJECT}_energy-db-data"

command -v docker >/dev/null 2>&1 || {
    printf 'Error: docker is not installed or not in PATH.\n' >&2
    exit 1
}

volume_exists() {
    docker volume inspect "$DB_VOLUME" >/dev/null 2>&1
}

container_id() {
    docker ps -a \
        --filter "label=com.docker.compose.project=${PROJECT}" \
        --filter "label=com.docker.compose.service=mariadb" \
        --format '{{.ID}}' |
    head -n 1
}

container_state() {
    local cid

    cid="$(container_id)"
    [ -n "$cid" ] || {
        printf 'not-created\n'
        return
    }

    docker inspect --format '{{.State.Status}}' "$cid" 2>/dev/null ||
        printf 'unknown\n'
}

health_status() {
    local cid
    local format

    cid="$(container_id)"
    [ -n "$cid" ] || {
        printf 'not-created\n'
        return
    }

    format='{{if .State.Health}}{{.State.Health.Status}}'
    format+='{{else}}none{{end}}'

    docker inspect --format "$format" "$cid" 2>/dev/null ||
        printf 'unknown\n'
}

printf 'Pre-release production database\n'
printf '%s\n' '-------------------------------'
printf 'Compose project:   %s\n' "$PROJECT"
printf 'Settings:          %s\n' \
    "$([ -f "$ENV_FILE" ] && printf present || printf missing)"
printf 'Runtime secret:    %s\n' \
    "$([ -f "$DB_SECRET" ] && printf present || printf missing)"
printf 'Root secret:       %s\n' \
    "$([ -f "$ROOT_SECRET" ] && printf present || printf missing)"
printf 'Database volume:   %s\n' \
    "$(volume_exists && printf present || printf missing)"
printf 'MariaDB container: %s\n' "$(container_state)"
printf 'MariaDB health:    %s\n' "$(health_status)"
