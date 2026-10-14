#!/bin/sh
# SPDX-License-Identifier: MIT
# Construct application DB URL from the mounted secret.
set -eu
: "${ENGY_DB_NAME:?database name required}"
: "${ENGY_DB_USER:?database user required}"
for value in "$ENGY_DB_NAME" "$ENGY_DB_USER"; do
    case "$value" in
        ''|*[!A-Za-z0-9_]*)
            echo 'Invalid database identifier.' >&2
            exit 2
            ;;
    esac
done
secret=/run/secrets/energy_db_pwd
if [ ! -r "$secret" ]; then
    echo 'Database credential unavailable.' >&2
    exit 1
fi
password="$(cat "$secret")"
case "$password" in
    ''|*[!A-Fa-f0-9]*)
        echo 'Invalid database credential format.' >&2
        exit 1
        ;;
esac
export ENGY_DB_URL="mysql://${ENGY_DB_USER}:${password}@mariadb:3306/${ENGY_DB_NAME}"
unset password
exec gosu energy:energy /usr/local/bin/energy-api
