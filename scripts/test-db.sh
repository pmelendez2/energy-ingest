#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#

set -euo pipefail

AGE_IDENTITY="${ENERGY_INGEST_AGE_IDENTITY:-}"
CONFIG_HOME="${XDG_CONFIG_HOME:-${HOME}/.config}"
AGE_FILE="${ENERGY_INGEST_DEV_SECRETS:-${CONFIG_HOME}/energy_ingest/dev_secrets.age}"

DB_HOST="127.0.0.1"
DB_PORT="3307"
DB_USER="energy_test"
DB_NAME="energy_test"
DB_PASSWORD_KEY="ENGY_T_DB_PWD"

for command in age mariadb awk mktemp; do
    if ! command -v "${command}" >/dev/null 2>&1; then
        echo "error: required command not found: ${command}" >&2
        exit 1
    fi
done

if [[ -z "${AGE_IDENTITY}" ]]; then
    echo "error: ENERGY_INGEST_AGE_IDENTITY must identify the local AGE private key" >&2
    exit 1
fi

if [[ ! -f "${AGE_IDENTITY}" ]]; then
    echo "error: AGE identity file not found" >&2
    exit 1
fi

if [[ ! -f "${AGE_FILE}" ]]; then
    echo "error: encrypted secrets file not found: ${AGE_FILE}" >&2
    exit 1
fi

DB_PASSWORD=$(
    age -d \
        -i "${AGE_IDENTITY}" \
        "${AGE_FILE}" |
    awk -F= -v key="${DB_PASSWORD_KEY}" '
        $1 == key {
            sub(/^[^=]*=/, "")
            print
            found = 1
            exit
        }
        END {
            if (!found) {
                exit 1
            }
        }
    '
)

if [[ -z "${DB_PASSWORD}" ]]; then
    echo "error: ${DB_PASSWORD_KEY} is missing or empty" >&2
    exit 1
fi

if [[ "${DB_PASSWORD}" == *$'\n'* ]]; then
    echo "error: ${DB_PASSWORD_KEY} contains an unsupported newline" >&2
    exit 1
fi

umask 077

CLIENT_CONFIG=$(mktemp /dev/shm/energy_ingest-mariadb.XXXXXX.cnf)

cleanup() {
    rm -f "${CLIENT_CONFIG}"
    unset DB_PASSWORD
}

trap cleanup EXIT

ESCAPED_PASSWORD="${DB_PASSWORD//\\/\\\\}"
ESCAPED_PASSWORD="${ESCAPED_PASSWORD//\"/\\\"}"

cat >"${CLIENT_CONFIG}" <<EOF
[client]
host=${DB_HOST}
port=${DB_PORT}
user=${DB_USER}
password="${ESCAPED_PASSWORD}"
database=${DB_NAME}
protocol=TCP
EOF

unset DB_PASSWORD
unset ESCAPED_PASSWORD

mariadb \
    --defaults-extra-file="${CLIENT_CONFIG}" \
    "$@"
