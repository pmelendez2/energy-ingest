#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Source after the caller has changed to deploy/docker-compose.
# Adds LAN binding only when explicitly configured in .env.prod.
engy_api_compose() {
    local lan_ip
    lan_ip="$(sed -n 's/^ENGY_API_LAN_BIND_IP=//p' .env.prod | tail -n 1)"
    ENGY_API_COMPOSE=(
        docker compose -p energy-ingest --env-file .env.prod
        -f compose.prod.yml
    )
    if [[ -n "$lan_ip" ]]; then
        [[ "$lan_ip" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
            echo 'Invalid ENGY_API_LAN_BIND_IP' >&2
            return 1
        }
        export ENGY_API_LAN_BIND_IP="$lan_ip"
        ENGY_API_COMPOSE+=(-f compose.api-lan.yml)
    fi
}
