#!/bin/sh
# SPDX-License-Identifier: MIT
# Production CLI wrapper: credentials remain in Docker secret files.
set -eu
: "${ENGY_DB_NAME:?database name is required}"
: "${ENGY_DB_USER:?database user is required}"
case "$ENGY_DB_NAME" in
    *[!A-Za-z0-9_]*|'') echo 'invalid database name' >&2; exit 1;;
esac
case "$ENGY_DB_USER" in
    *[!A-Za-z0-9_]*|'') echo 'invalid database user' >&2; exit 1;;
esac
secret=/run/secrets/energy_db_pwd
[ -r "$secret" ] || { echo 'database secret missing' >&2; exit 1; }
pwd="$(cat "$secret")"
case "$pwd" in
    *[!A-Fa-f0-9]*|'') echo 'invalid database secret' >&2; exit 1;;
esac
export ENGY_DB_URL="mysql://${ENGY_DB_USER}:${pwd}@mariadb:3306/${ENGY_DB_NAME}"
exec gosu energy:energy /usr/local/bin/energy-store-import "$@"
