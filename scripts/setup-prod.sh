#!/usr/bin/env bash
# SPDX-License-Identifier: MIT

set -euo pipefail

SCRIPT_DIR="$(
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1
    pwd
)"
# Setup owns the Compose deployment, not the caller's cwd.
cd "$SCRIPT_DIR/../deploy/docker-compose"

PROJECT="energy-ingest"
ENV_FILE=".env.prod"
ENV_EXAMPLE=".env.prod.example"
SECRETS_DIR="prod-secrets"
DB_SECRET="${SECRETS_DIR}/energy_db_pwd"
ROOT_SECRET="${SECRETS_DIR}/energy_db_root_pwd"
DB_VOLUME="${PROJECT}_energy-db-data"
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

prompt_yes_no() {
    local prompt="$1"
    local answer

    read -r -p "$prompt [y/N]: " answer || true

    case "$answer" in
        y|Y|yes|YES|Yes)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

require_tools() {
    command -v docker >/dev/null 2>&1 ||
        die "docker is not installed or not in PATH."

    docker version >/dev/null 2>&1 ||
        die "Docker Engine is not available."

    docker compose version >/dev/null 2>&1 ||
        die "Docker Compose v2 is not available."

    [ -f compose.prod.yml ] || die "Missing compose.prod.yml."
    [ -f "$ENV_EXAMPLE" ] || die "Missing $ENV_EXAMPLE."
    [ -f 10-runtime-grants.sh ] ||
        die "Missing 10-runtime-grants.sh."
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

container_running() {
    local cid

    cid="$(container_id)"
    [ -n "$cid" ] || return 1

    [ "$(docker inspect \
        --format '{{.State.Running}}' "$cid" 2>/dev/null || true)" = "true" ]
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

generate_secret() {
    local path="$1"

    mkdir -p "$SECRETS_DIR"
    chmod 700 "$SECRETS_DIR"
    umask 077

    if command -v openssl >/dev/null 2>&1; then
        openssl rand -hex 32 >"$path"
    elif command -v python3 >/dev/null 2>&1; then
        python3 - "$path" <<'PY'
import secrets
import sys

with open(sys.argv[1], "w", encoding="utf-8") as handle:
    handle.write(secrets.token_hex(32))
PY
    else
        die "openssl or python3 is required to generate credentials."
    fi

    chmod 600 "$path"
}

write_env() {
    cp "$ENV_EXAMPLE" "$ENV_FILE"
    chmod 600 "$ENV_FILE"
}

state() {
    local env=false
    local db_secret=false
    local root_secret=false
    local volume=false
    local container=false

    [ -f "$ENV_FILE" ] && env=true
    [ -f "$DB_SECRET" ] && db_secret=true
    [ -f "$ROOT_SECRET" ] && root_secret=true
    volume_exists && volume=true
    [ -n "$(container_id)" ] && container=true

    if [ "$env" = false ] &&
       [ "$db_secret" = false ] &&
       [ "$root_secret" = false ] &&
       [ "$volume" = false ] &&
       [ "$container" = false ]; then
        printf 'NEW\n'
        return
    fi

    if [ "$env" = true ] &&
       [ "$db_secret" = true ] &&
       [ "$root_secret" = true ]; then
        if container_running; then
            printf 'RUNNING\n'
        elif volume_exists; then
            printf 'STOPPED\n'
        else
            printf 'PREPARED\n'
        fi
        return
    fi

    printf 'INCOMPLETE\n'
}

summary() {
    printf '\nEnergy Usage — Database Management\n'
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
    printf 'MariaDB health:    %s\n' "$(health_status)"
    printf '\n'
}

validate() {
    "${COMPOSE[@]}" config >/dev/null
}

start_database() {
    validate
    "${COMPOSE[@]}" up -d mariadb
}

wait_healthy() {
    local attempts=60
    local health

    while [ "$attempts" -gt 0 ]; do
        health="$(health_status)"

        case "$health" in
            healthy)
                printf 'MariaDB is healthy.\n'
                return 0
                ;;
            unhealthy)
                "${COMPOSE[@]}" ps
                return 1
                ;;
        esac

        attempts=$((attempts - 1))
        sleep 2
    done

    printf 'Timed out waiting for MariaDB health.\n' >&2
    return 1
}

migrate_database() {
    # Pull the versioned application image; never build production from
    # local source. Migrations are safe on repeat runs.
    printf 'Applying and verifying production schema...\n'
    "${COMPOSE[@]}" run --rm --pull always --no-deps \
        migrate database bootstrap
}

create_missing_local_files() {
    mkdir -p "$SECRETS_DIR"
    chmod 700 "$SECRETS_DIR"

    mkdir -p private-imports

    if [ ! -f "$ENV_FILE" ]; then
        write_env
        printf 'Created %s from %s.\n' "$ENV_FILE" "$ENV_EXAMPLE"
    fi

    if [ ! -f "$DB_SECRET" ]; then
        generate_secret "$DB_SECRET"
        printf 'Created production runtime DB credential.\n'
    fi

    if [ ! -f "$ROOT_SECRET" ]; then
        generate_secret "$ROOT_SECRET"
        printf 'Created production administrative DB credential.\n'
    fi
}

first_setup() {
    cat <<EOF

No Energy Usage database environment was found.

This will create:

  Compose project:  ${PROJECT}
  MariaDB container
  Persistent production database volume
  Local .env.prod configuration
  Generated production database credentials

MariaDB port 3306 will not be published to the host.

Existing data is preserved during normal startup.

EOF

    prompt_yes_no "Create the Energy Usage environment?" || return

    create_missing_local_files
    start_database
    wait_healthy
    migrate_database
    summary
}

continue_environment() {
    create_missing_local_files
    start_database
    wait_healthy
    migrate_database
    summary
}

refresh_containers() {
    cat <<'EOF'

Refresh application containers

This recreates the Compose service using published images, preserving:

  .env.prod
  production credentials
  production database volume and data

EOF

    prompt_yes_no "Refresh application containers?" || return

    validate
    "${COMPOSE[@]}" up -d --force-recreate mariadb
    wait_healthy
    migrate_database
    summary
}

recreate_database() {
    cat <<'EOF'

Reset Energy Usage database

This permanently deletes the current production database volume.

This permanently removes stored database records. Retain a verified
backup before choosing this option.

EOF

    prompt_yes_no "Delete all stored usage data and reset the database?" || return

    "${COMPOSE[@]}" down

    if volume_exists; then
        docker volume rm "$DB_VOLUME"
    fi

    if prompt_yes_no "Generate new production credentials?"; then
        rm -f "$DB_SECRET" "$ROOT_SECRET"
        generate_secret "$DB_SECRET"
        generate_secret "$ROOT_SECRET"
        printf 'Generated new production database credentials.\n'
    fi

    if prompt_yes_no "Reset .env.prod from the tracked template?"; then
        write_env
        printf 'Reset %s from %s.\n' "$ENV_FILE" "$ENV_EXAMPLE"
    fi

    create_missing_local_files
    start_database
    wait_healthy
    migrate_database
    summary
}

remove_environment() {
    cat <<'EOF'

Uninstall Energy Usage

Containers and the Compose network can be removed.

You will separately choose whether to delete:

  persistent production database data
  local .env.prod settings
  production database credentials

Review every data-deletion choice before confirming.

EOF

    prompt_yes_no "Remove application containers and network?" || return

    local delete_data=false
    local delete_env=false
    local delete_secrets=false

    prompt_yes_no "Delete persistent production database data?" &&
        delete_data=true

    prompt_yes_no "Delete local .env.prod settings?" &&
        delete_env=true

    prompt_yes_no "Delete production database credentials?" &&
        delete_secrets=true

    if [ -f "$ENV_FILE" ] &&
       [ -f "$DB_SECRET" ] &&
       [ -f "$ROOT_SECRET" ]; then
        "${COMPOSE[@]}" down
    else
        local cid
        cid="$(container_id)"
        [ -z "$cid" ] || docker rm -f "$cid"
    fi

    if [ "$delete_data" = true ] && volume_exists; then
        docker volume rm "$DB_VOLUME"
    fi

    if [ "$delete_env" = true ]; then
        rm -f "$ENV_FILE"
    fi

    if [ "$delete_secrets" = true ]; then
        rm -f "$DB_SECRET" "$ROOT_SECRET"
    fi

    summary
}

menu() {
    local choice

    summary

    cat <<'EOF'
Select an action:

  1) Start or update Energy Usage
     Keep settings and usage data; check database schema.

  2) Refresh application containers
     Recreate containers; keep settings, credentials, and data.

  3) Reset database — DELETE ALL STORED USAGE DATA
     Initialize an empty database; retain source XML files.

  4) Uninstall Energy Usage
     Choose whether to delete database, settings, and credentials.

  5) Exit without changes

EOF

    read -r -p "Selection: " choice || true

    case "$choice" in
        1)
            continue_environment
            ;;
        2)
            refresh_containers
            ;;
        3)
            recreate_database
            ;;
        4)
            remove_environment
            ;;
        5)
            printf 'Exit without making changes.\n'
            ;;
        *)
            die "Invalid selection."
            ;;
    esac
}

main() {
    local current

    require_tools
    current="$(state)"

    case "$current" in
        NEW)
            first_setup
            ;;
        PREPARED|RUNNING|STOPPED|INCOMPLETE)
            menu
            ;;
        *)
            die "Unknown state: $current"
            ;;
    esac
}

main "$@"
