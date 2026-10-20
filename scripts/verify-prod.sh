#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Read-only checks for the local Energy Usage Docker deployment.
set -euo pipefail
# Resolve deployment files relative to this script, not the shell cwd.
SCRIPT_DIR="$(
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd
)"
cd -- "$SCRIPT_DIR/../deploy/docker-compose"

usage() {
    cat <<'HELP'
Usage: ./verify-prod.sh [--help]

Check local configuration, credentials, Docker Compose, and the
running MariaDB service without modifying any database records.
HELP
}
if (( $# > 0 )); then
    if (( $# == 1 )) && [[ "$1" == '--help' || "$1" == '-h' ]]; then
        usage
        exit 0
    fi
    usage >&2
    exit 2
fi

errors=0
pass() { printf '[OK] %s\n' "$*"; }
fail() { printf '[FAIL] %s\n' "$*" >&2; errors=$((errors + 1)); }

[[ -f .env.prod ]] && pass 'Configuration present' ||
    fail 'Missing .env.prod: run setup-prod.sh'
[[ -s prod-secrets/energy_db_pwd ]] &&
    pass 'Application credential present' ||
    fail 'Missing application credential'
[[ -s prod-secrets/energy_db_root_pwd ]] &&
    pass 'Administrative credential present' ||
    fail 'Missing administrative credential'

if ! command -v docker >/dev/null 2>&1; then
    fail 'Docker is not installed or not on PATH'
    exit 1
fi
if docker version >/dev/null 2>&1; then
    pass 'Docker Engine accessible'
else
    fail 'Docker Engine is not accessible'
fi
if docker compose version >/dev/null 2>&1; then
    pass 'Docker Compose available'
else
    fail 'Docker Compose is unavailable'
fi

if [[ -f .env.prod ]] && docker compose -p energy-ingest \
    --env-file .env.prod -f compose.prod.yml config --quiet; then
    pass 'Compose configuration valid'
else
    fail 'Compose configuration invalid'
fi

cid="$(docker ps -a \
    --filter label=com.docker.compose.project=energy-ingest \
    --filter label=com.docker.compose.service=mariadb \
    --format '{{.ID}}' | head -n 1)" || cid=''
if [[ -z "$cid" ]]; then
    fail 'MariaDB container not found'
else
    health="$(docker inspect --format \
        '{{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}' \
        "$cid" 2>/dev/null)" || health='unknown'
    if [[ "$health" == healthy ]]; then
        pass 'MariaDB healthy'
    else
        fail "MariaDB health: $health"
    fi
fi

# Check the HTTP endpoint, not just the container running state.
if [[ -s prod-secrets/ha_energy_dev_api_token ]]; then
    pass 'Home Assistant API token file present'
else
    fail 'Home Assistant API token missing'
fi
if command -v curl >/dev/null 2>&1; then
    code="$(curl -sS -o /dev/null -w '%{http_code}' \
        --max-time 5 http://127.0.0.1:8080/v1/meters)" || code=000
    [[ "$code" == 401 ]] && pass 'API rejects anonymous requests' || \
        fail "Local HTTP API not reachable/authenticated: $code"
else
    fail 'curl required to verify HTTP API'
fi

if (( errors > 0 )); then
    printf '\nVerification found %s problem(s).\n' "$errors" >&2
    exit 1
fi
printf '\nDeployment checks passed.\n'
