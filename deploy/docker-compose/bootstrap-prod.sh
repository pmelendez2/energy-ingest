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
ROOT_SECRET="prod-secrets/energy_db_root_pwd"

COMPOSE=(
    docker compose
    -p "$PROJECT"
    --env-file "$ENV_FILE"
    -f compose.prod.yml
)

die() {
    printf 'Error: %s\n' "$*" >&2
    exit 1
}

require_environment() {
    command -v docker >/dev/null 2>&1 ||
        die "docker is not installed or not in PATH."

    docker version >/dev/null 2>&1 ||
        die "Docker Engine is not available."

    docker compose version >/dev/null 2>&1 ||
        die "Docker Compose v2 is not available."

    [ -f "$ENV_FILE" ] ||
        die "Missing $ENV_FILE. Run ./setup-prod.sh first."

    [ -f "$ROOT_SECRET" ] ||
        die "Missing production administrative database credential."

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

    [ -n "$cid" ] || {
        printf 'not-created\n'
        return
    }

    format='{{if .State.Health}}{{.State.Health.Status}}'
    format+='{{else}}none{{end}}'

    docker inspect --format "$format" "$cid" 2>/dev/null ||
        printf 'unknown\n'
}

main() {
    require_environment

    if [ "$(mariadb_health)" != "healthy" ]; then
        die "Production MariaDB is not healthy. Run ./setup-prod.sh first."
    fi

    "${COMPOSE[@]}" run \
        --rm \
        --pull always \
        --no-deps \
        migrate \
        database bootstrap
}

main "$@"
