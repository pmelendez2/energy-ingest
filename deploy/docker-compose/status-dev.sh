#!/usr/bin/env bash
# SPDX-License-Identifier: MIT

set -euo pipefail

SCRIPT_DIR="$(
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1
    pwd
)"
cd "$SCRIPT_DIR"

PROJECT="energy-ingest-dev"
DB_VOLUME="${PROJECT}_energy-db-data"

container_state() {
    local service="$1"
    local cid

    cid="$(
        docker ps -a \
            --filter "label=com.docker.compose.project=${PROJECT}" \
            --filter "label=com.docker.compose.service=${service}" \
            --format '{{.ID}}' |
        head -n 1
    )"

    if [ -z "$cid" ]; then
        printf 'not-created'
        return
    fi

    docker inspect --format '{{.State.Status}}' "$cid"
}

mariadb_health() {
    local cid
    local format

    cid="$(
        docker ps -a \
            --filter "label=com.docker.compose.project=${PROJECT}" \
            --filter "label=com.docker.compose.service=mariadb" \
            --format '{{.ID}}' |
        head -n 1
    )"

    if [ -z "$cid" ]; then
        printf 'not-created'
        return
    fi

    format='{{if .State.Health}}{{.State.Health.Status}}'
    format+='{{else}}none{{end}}'
    docker inspect --format "$format" "$cid"
}

printf 'Development environment status\n'
printf '%s\n' '------------------------------'
printf 'Compose project:   %s\n' "$PROJECT"
printf 'Settings:          %s\n' \
    "$([ -f .env.dev ] && printf present || printf missing)"
printf 'Runtime secret:    %s\n' \
    "$([ -f dev-secrets/energy_db_pwd ] &&
        printf present || printf missing)"
printf 'Root secret:       %s\n' \
    "$([ -f dev-secrets/energy_db_root_pwd ] &&
        printf present || printf missing)"
printf 'DB volume:         %s\n' \
    "$(docker volume inspect "$DB_VOLUME" >/dev/null 2>&1 &&
        printf present || printf missing)"
printf 'Dev container:     %s\n' "$(container_state dev)"
printf 'MariaDB container: %s\n' "$(container_state mariadb)"
printf 'MariaDB health:    %s\n' "$(mariadb_health)"
