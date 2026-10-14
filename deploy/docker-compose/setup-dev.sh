#!/usr/bin/env bash
# SPDX-License-Identifier: MIT

set -euo pipefail

SCRIPT_DIR="$(
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1
    pwd
)"
cd "$SCRIPT_DIR"

PROJECT="energy-ingest-dev"
COMPOSE=(docker compose -p "$PROJECT" --env-file .env.dev -f compose.dev.yml)
ENV_FILE=".env.dev"
ENV_EXAMPLE=".env.dev.example"
SECRETS_DIR="dev-secrets"
DB_SECRET="${SECRETS_DIR}/energy_db_pwd"
ROOT_SECRET="${SECRETS_DIR}/energy_db_root_pwd"
DB_VOLUME="${PROJECT}_energy-db-data"

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
    command -v git >/dev/null 2>&1 ||
        die "git is not installed or not in PATH."

    command -v docker >/dev/null 2>&1 ||
        die "docker is not installed or not in PATH."

    docker version >/dev/null 2>&1 ||
        die "Docker Engine is not available."

    docker compose version >/dev/null 2>&1 ||
        die "Docker Compose v2 is not available."

    [ -f compose.dev.yml ] || die "Missing compose.dev.yml."
    [ -f "$ENV_EXAMPLE" ] || die "Missing $ENV_EXAMPLE."
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

configure_repo_mount() {
    local project_root
    local repo_root
    local relative

    project_root="$(cd ../.. && pwd)"
    repo_root="$(git -C "$project_root" rev-parse --show-toplevel)"
    relative="$(realpath --relative-to="$repo_root" "$project_root")"

    sed -i \
        '/^ENGY_REPO_ROOT=/d; /^ENGY_PROJECT_RELATIVE=/d' \
        "$ENV_FILE"

    {
        printf '\nENGY_REPO_ROOT=%s\n' "$repo_root"
        printf 'ENGY_PROJECT_RELATIVE=%s\n' "$relative"
    } >>"$ENV_FILE"
}

ensure_files() {
    if [ ! -f "$ENV_FILE" ]; then
        cp "$ENV_EXAMPLE" "$ENV_FILE"
        chmod 600 "$ENV_FILE"
        printf 'Created %s from %s.\n' "$ENV_FILE" "$ENV_EXAMPLE"
    fi

    if [ ! -f "$DB_SECRET" ]; then
        generate_secret "$DB_SECRET"
        printf 'Created development runtime DB credential.\n'
    fi

    if [ ! -f "$ROOT_SECRET" ]; then
        generate_secret "$ROOT_SECRET"
        printf 'Created development administrative DB credential.\n'
    fi

    configure_repo_mount
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
       [ "$root_secret" = true ] &&
       [ "$volume" = true ]; then
        if container_running; then
            printf 'RUNNING\n'
        else
            printf 'STOPPED\n'
        fi
        return
    fi

    printf 'INCOMPLETE\n'
}

summary() {
    printf '\nDevelopment environment\n'
    printf '%s\n' '-----------------------'
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

start() {
    "${COMPOSE[@]}" build dev
    "${COMPOSE[@]}" up -d mariadb dev
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

assert_volume_owned() {
    local label

    volume_exists || return 0

    label="$(
        docker volume inspect \
            --format '{{index .Labels "com.docker.compose.project"}}' \
            "$DB_VOLUME" 2>/dev/null || true
    )"

    [ "$label" = "$PROJECT" ] ||
        die "Refusing to delete volume not owned by $PROJECT."
}

first_setup() {
    cat <<EOF

No existing containerized development environment was found.

This will create:

  Compose project:  ${PROJECT}
  Rust dev container
  MariaDB dev container
  Persistent development database volume
  Cargo cache volumes
  Local .env.dev configuration
  Generated development database credentials

The existing Docker Swarm test environment is not modified.

EOF

    prompt_yes_no "Create the development environment?" || return

    ensure_files
    validate
    start
    wait_healthy
    summary
}

reuse() {
    validate
    start
    wait_healthy
    summary
}

refresh() {
    cat <<'EOF'

Refresh recreates the development containers and rebuilds the Rust dev image.

Preserved:
  .env.dev
  database credentials
  development database volume and data
  Cargo cache volumes

EOF

    prompt_yes_no "Refresh development containers?" || return

    "${COMPOSE[@]}" build --no-cache dev
    "${COMPOSE[@]}" up -d --force-recreate mariadb dev
    wait_healthy
    summary
}

recreate() {
    cat <<EOF

Recreate development environment

This permanently deletes the development database volume:

  ${DB_VOLUME}

It preserves .env.dev and database credentials by default.

EOF

    prompt_yes_no "Delete and recreate the development database?" || return

    assert_volume_owned
    "${COMPOSE[@]}" down

    if volume_exists; then
        docker volume rm "$DB_VOLUME"
    fi

    if prompt_yes_no "Generate new development database credentials?"; then
        rm -f "$DB_SECRET" "$ROOT_SECRET"
        ensure_files
    fi

    if prompt_yes_no "Reset .env.dev to defaults?"; then
        cp "$ENV_EXAMPLE" "$ENV_FILE"
        chmod 600 "$ENV_FILE"
        configure_repo_mount
    fi

    validate
    start
    wait_healthy
    summary
}

remove_env() {
    local delete_data=false
    local delete_env=false
    local delete_secrets=false

    cat <<'EOF'

Remove development environment

Containers and the Compose network will be removed.

You will separately choose whether to delete:
  persistent development database data
  local .env.dev settings
  development database credentials

EOF

    prompt_yes_no "Remove development containers and network?" || return

    if prompt_yes_no "Delete persistent development database data?"; then
        delete_data=true
        assert_volume_owned
    fi

    if prompt_yes_no "Delete local .env.dev settings?"; then
        delete_env=true
    fi

    if prompt_yes_no "Delete development database credentials?"; then
        delete_secrets=true
    fi

    "${COMPOSE[@]}" down

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

    while true; do
        summary
        cat <<'EOF'
What would you like to do?

  1. Continue using this environment
     Keep settings, credentials, database data, and Cargo caches.
     Ensure the development containers are running.

  2. Refresh development containers
     Rebuild and recreate containers.
     Preserve settings, credentials, database data, and Cargo caches.

  3. Recreate development database
     Delete the development database and initialize it again.
     You can also choose new credentials or default settings.

  4. Remove development environment
     Remove containers and network.
     Choose whether persistent data, settings, and credentials are kept.

  5. Cancel
     Exit without making changes.

EOF
        read -r -p "Selection: " choice
        case "$choice" in
            1)
                reuse
                return
                ;;
            2)
                refresh
                return
                ;;
            3)
                recreate
                return
                ;;
            4)
                remove_env
                return
                ;;
            5)
                return
                ;;
            *)
                printf 'Invalid selection.\n'
                ;;
        esac
    done
}

repair() {
    cat <<'EOF'

The development environment is incomplete.

The repair operation preserves any existing database volume and existing
local files. It creates only missing settings or credentials, then starts the
containers.

EOF

    prompt_yes_no "Attempt non-destructive repair?" || return

    ensure_files
    validate
    start
    wait_healthy
    summary
}

main() {
    local current

    require_tools
    current="$(state)"

    case "$current" in
        NEW)
            first_setup
            ;;
        RUNNING|STOPPED)
            menu
            ;;
        INCOMPLETE)
            summary
            repair
            ;;
        *)
            die "Unknown state: $current"
            ;;
    esac
}

main "$@"
