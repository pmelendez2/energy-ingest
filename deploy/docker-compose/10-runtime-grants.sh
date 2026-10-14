#!/bin/sh
# SPDX-License-Identifier: MIT

set -eu

case "${MARIADB_DATABASE:-}" in
    ''|*[!A-Za-z0-9_]*)
        echo "invalid MARIADB_DATABASE" >&2
        exit 1
        ;;
esac

case "${MARIADB_USER:-}" in
    ''|*[!A-Za-z0-9_]*)
        echo "invalid MARIADB_USER" >&2
        exit 1
        ;;
esac

admin_cnf="$(mktemp)"
trap 'rm -f "$admin_cnf"' EXIT HUP INT TERM
chmod 600 "$admin_cnf"

root_password="${MARIADB_ROOT_PASSWORD:?MARIADB_ROOT_PASSWORD is not set}"
escaped_password="$(
    printf '%s' "$root_password" |
    sed 's/\\/\\\\/g; s/"/\\"/g'
)"

cat >"$admin_cnf" <<EOF
[client]
user=root
password="${escaped_password}"
protocol=socket
EOF

mariadb \
    --defaults-extra-file="$admin_cnf" \
    --database=mysql <<EOF
REVOKE ALL PRIVILEGES, GRANT OPTION
FROM '${MARIADB_USER}'@'%';

GRANT SELECT, INSERT, UPDATE, DELETE
ON \`${MARIADB_DATABASE}\`.*
TO '${MARIADB_USER}'@'%';

FLUSH PRIVILEGES;
EOF
