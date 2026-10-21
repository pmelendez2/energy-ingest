#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Maintain customer-defined physical meter profiles.
# Meter numbers are not passed as command-line arguments or logged.
set -euo pipefail
# Resolve deployment files relative to this script, not the shell cwd.
SCRIPT_DIR="$(
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd
)"
cd -- "$SCRIPT_DIR/../deploy/docker-compose"

usage() {
    cat <<'HELP'
Usage: ./meter-prod.sh list
       ./meter-prod.sh create [--custodian ID] --kind electric|gas \
           --name NAME
       ./meter-prod.sh set-number --meter ID
       ./meter-prod.sh rename --meter ID --name NAME
       ./meter-prod.sh attach --meter ID --resource ID
       ./meter-prod.sh attach-compatible --meter ID [--yes]
       ./meter-prod.sh resources --meter ID

Profiles are separate from Green Button resource identities.
Associations require an explicit attach operation.
HELP
}
[[ $# -gt 0 ]] || { usage; exit 2; }
verb=$1
shift
args=()
case "$verb" in
    list) [[ $# -eq 0 ]] || { usage >&2; exit 2; }
        args=(meter list) ;;
    create)
        custodian='' kind='' name=''
        while [[ $# -gt 0 ]]; do
            case "$1" in
                --custodian) [[ $# -ge 2 ]] || exit 2
                    custodian=$2; shift 2 ;;
                --kind) [[ $# -ge 2 ]] || exit 2
                    kind=$2; shift 2 ;;
                --name) [[ $# -ge 2 ]] || exit 2
                    name=$2; shift 2 ;;
                *) usage >&2; exit 2 ;;
            esac
        done
        case "$kind" in
            electric) code=1 ;;
            gas) code=7 ;;
            *) usage >&2; exit 2 ;;
        esac
        [[ -n "$name" ]] || { usage >&2; exit 2; }
        if [[ -n "$custodian" ]]; then
            args=(meter create "$custodian" "$code" "$name")
        else
            args=(meter create-auto "$code" "$name")
        fi ;;
    set-number)
        meter_id=''
        stdin_mode=0
        [[ $# -ge 2 && "$1" == '--meter' ]] || {
            usage >&2; exit 2
        }
        meter_id="$2"
        shift 2
        if [[ $# -eq 1 && "$1" == '--stdin' ]]; then
            stdin_mode=1
        elif [[ $# -ne 0 ]]; then
            usage >&2; exit 2
        fi
        [[ "$meter_id" =~ ^[1-9][0-9]*$ ]] || exit 2
        [[ -f .env.prod ]] || {
            echo 'Error: run setup-prod.sh first.' >&2
            exit 2
        }
        if [[ "$stdin_mode" -eq 1 ]]; then
            IFS= read -r number || {
                echo 'Error: missing meter number on stdin.' >&2
                exit 2
            }
        else
            [[ -t 0 ]] || {
                echo 'Error: interactive terminal required.' >&2
                exit 2
            }
            read -r -p 'Utility meter number: ' number
        fi
        [[ -n "${number//[[:space:]]/}" &&
            ${#number} -le 255 ]] || {
            echo 'Error: invalid meter number.' >&2
            exit 2
        }
        if [[ "$stdin_mode" -eq 0 ]]; then
            printf 'Entered meter number: %s\n' "$number"
            read -r -p 'Save this meter number? [y/N]: ' answer
            case "$answer" in
                y|Y|yes|YES) ;;
                *) unset number; echo 'Meter number not saved.'; exit 0 ;;
            esac
        fi
        # Pipe the number to stdin; never put it on the command line.
        printf '%s\n' "$number" | docker compose -p energy-ingest \
            --env-file .env.prod -f compose.prod.yml \
            run -T --rm --no-deps admin meter set-number \
                "$meter_id" --stdin
        unset number
        exit 0 ;;
    rename)
        meter='' name=''
        while [[ $# -gt 0 ]]; do
            case "$1" in
                --meter) [[ $# -ge 2 ]] || exit 2
                    meter=$2; shift 2 ;;
                --name) [[ $# -ge 2 ]] || exit 2
                    name=$2; shift 2 ;;
                *) usage >&2; exit 2 ;;
            esac
        done
        [[ -n "$meter" && -n "$name" ]] || exit 2
        args=(meter rename "$meter" "$name") ;;
    attach)
        meter='' resource=''
        while [[ $# -gt 0 ]]; do
            case "$1" in
                --meter) [[ $# -ge 2 ]] || exit 2
                    meter=$2; shift 2 ;;
                --resource) [[ $# -ge 2 ]] || exit 2
                    resource=$2; shift 2 ;;
                *) usage >&2; exit 2 ;;
            esac
        done
        [[ -n "$meter" && -n "$resource" ]] || exit 2
        args=(meter attach "$meter" "$resource") ;;
    attach-compatible)
        meter='' yes=0
        while [[ $# -gt 0 ]]; do
            case "$1" in
                --meter) [[ $# -ge 2 ]] || exit 2
                    meter=$2; shift 2 ;;
                --yes) yes=1; shift ;;
                *) usage >&2; exit 2 ;;
            esac
        done
        [[ "$meter" =~ ^[1-9][0-9]*$ ]] || {
            echo 'Error: --meter requires a positive ID.' >&2
            exit 2
        }
        [[ -f .env.prod ]] || {
            echo 'Error: run setup-prod.sh first.' >&2
            exit 2
        }
        compose=(docker compose -p energy-ingest
            --env-file .env.prod -f compose.prod.yml)
        # Preview does not modify the database.
        "${compose[@]}" run --rm --no-deps admin \
            meter attach-compatible "$meter"
        if [[ "$yes" -eq 0 ]]; then
            read -r -p 'Associate the eligible file data? [y/N]: ' answer
            [[ "$answer" == [yY] ]] || {
                echo 'No associations changed.'
                exit 0
            }
        fi
        # Eligibility and ownership are rechecked inside a transaction.
        "${compose[@]}" run --rm --no-deps admin \
            meter attach-compatible "$meter" --apply
        exit 0 ;;
    resources)
        [[ $# -eq 2 && "$1" == '--meter' ]] || exit 2
        args=(meter resources "$2") ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
esac
[[ -f .env.prod ]] || {
    echo 'Error: run setup-prod.sh first.' >&2
    exit 2
}
docker compose -p energy-ingest \
    --env-file .env.prod -f compose.prod.yml \
    run --rm --no-deps admin "${args[@]}"
