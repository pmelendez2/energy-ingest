#!/bin/sh
# SPDX-License-Identifier: MIT
#
# Production administration entrypoint.
#
# Constructs the appropriate database connection from mounted
# credentials and delegates execution to energy-store-admin.
#
# Schema migrations require administrative credentials.
# Registration operations use restricted application credentials.
set -eu

# Require an unprivileged application identity.
: "${ENGY_RUN_USER:?Runtime user is required}"
: "${ENGY_RUN_GROUP:?Runtime group is required}"

run_uid=$(gosu \
    "${ENGY_RUN_USER}:${ENGY_RUN_GROUP}" id -u)

if [ "$run_uid" -eq 0 ]; then
    echo "ERROR: Application must not run as root" >&2
    exit 1
fi

if [ "${1:-}" = '--help' ] || [ "${1:-}" = '-h' ] ||
   [ "$#" -eq 0 ]; then
    exec gosu "${ENGY_RUN_USER}:${ENGY_RUN_GROUP}" \
         /usr/local/bin/energy-store-admin --help
fi

: "${ENGY_DB_NAME:?database name is required}"
case "$ENGY_DB_NAME" in
    ''|*[!A-Za-z0-9_]*)
        echo 'Invalid database name.' >&2
        exit 2
        ;;
esac

case "${1:-} ${2:-}" in
    'database bootstrap')
        secret=/run/secrets/energy_db_root_pwd
        user=root
        variable=ENGY_DB_ADMIN_URL
        ;;
    'custodian create'|'source create'|'source list'|\
    'stream discover'|'stream analyze'|\
    'meter create'|'meter create-auto'|'meter rename'|\
    'meter list'|'meter attach'|'meter attach-compatible'|\
    'meter resources'|\
    'api-token create'|'api-token list'|'api-token revoke')
        secret=/run/secrets/energy_db_pwd
        user="${ENGY_DB_USER:?database user is required}"
        variable=ENGY_DB_URL
        case "$user" in
            ''|*[!A-Za-z0-9_]*)
                echo 'Invalid database user.' >&2
                exit 2
                ;;
        esac
        ;;
    *)
        # Let the Rust executable report invalid syntax without secrets.
        exec gosu "${ENGY_RUN_USER}:${ENGY_RUN_GROUP}" \
             /usr/local/bin/energy-store-admin "$@"
        ;;
esac

if [ ! -r "$secret" ]; then
    echo 'Required database credential is not available.' >&2
    exit 1
fi
password="$(cat "$secret")"
case "$password" in
    ''|*[!A-Fa-f0-9]*)
        echo 'Invalid database credential format.' >&2
        exit 1
        ;;
esac

url="mysql://${user}:${password}@mariadb:3306/${ENGY_DB_NAME}"
if [ "$variable" = 'ENGY_DB_ADMIN_URL' ]; then
    export ENGY_DB_ADMIN_URL="$url"
else
    export ENGY_DB_URL="$url"
fi
unset password url
if [ "${1:-}" = "api-token" ]; then
    shift
    exec gosu "${ENGY_RUN_USER}:${ENGY_RUN_GROUP}" \
         /usr/local/bin/energy-api-token "$@"
fi
exec gosu "${ENGY_RUN_USER}:${ENGY_RUN_GROUP}" \
     /usr/local/bin/energy-store-admin "$@"
