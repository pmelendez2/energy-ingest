#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Isolated production-image smoke test. Never uses production secrets/data.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="${ENGY_TEST_IMAGE:-energy-ingest:nonroot-test}"
DB_IMAGE="${ENGY_TEST_DB_IMAGE:-mariadb:lts}"
TMP_DIR="$(mktemp -d)"
NAME="engy-rt-$(openssl rand -hex 5)"
NET="$NAME"
DB="${NAME}-db"
API="${NAME}-api"

cleanup() {
    docker rm -f "$API" "$DB" >/dev/null 2>&1 || true
    docker network rm "$NET" >/dev/null 2>&1 || true
    rm -rf -- "$TMP_DIR"
}
trap cleanup EXIT

for tool in docker openssl; do
    command -v "$tool" >/dev/null || {
        echo "Missing required tool: $tool" >&2
        exit 1
    }
done

docker image inspect "$IMAGE" >/dev/null
umask 077
openssl rand -hex 32 >"$TMP_DIR/energy_db_pwd"
openssl rand -hex 32 >"$TMP_DIR/energy_db_root_pwd"
chmod 600 "$TMP_DIR"/*

docker network create --internal "$NET" >/dev/null

echo 'Starting isolated MariaDB ...'
docker run -d --name "$DB" --network "$NET" \
    --network-alias mariadb \
    --mount "type=bind,src=$TMP_DIR/energy_db_pwd,\
dst=/run/secrets/energy_db_pwd,readonly" \
    --mount "type=bind,src=$TMP_DIR/energy_db_root_pwd,\
dst=/run/secrets/energy_db_root_pwd,readonly" \
    -e MARIADB_DATABASE=energy_ingest_runtime_test \
    -e MARIADB_USER=energy_ingest_runtime_test \
    -e MARIADB_PASSWORD_FILE=/run/secrets/energy_db_pwd \
    -e MARIADB_ROOT_PASSWORD_FILE=/run/secrets/energy_db_root_pwd \
    --health-cmd='healthcheck.sh --connect --innodb_initialized' \
    --health-interval=2s --health-retries=30 \
    "$DB_IMAGE" >/dev/null

healthy=false
for _ in $(seq 1 60); do
    status="$(docker inspect -f '{{.State.Health.Status}}' "$DB")"
    if [ "$status" = healthy ]; then
        healthy=true
        break
    fi
    if [ "$status" = unhealthy ]; then
        break
    fi
    sleep 2
done
if [ "$healthy" != true ]; then
    echo 'MariaDB did not become healthy.' >&2
    docker logs --tail 30 "$DB" >&2
    exit 1
fi

# MariaDB's standard init grants the application account privileges.
# We then apply the restricted grant policy used by production.
docker cp "$ROOT/deploy/docker-compose/10-runtime-grants.sh" \
    "$DB:/tmp/10-runtime-grants.sh"
docker exec -e MARIADB_DATABASE=energy_ingest_runtime_test \
    -e MARIADB_USER=energy_ingest_runtime_test \
    "$DB" sh -c '
        export MARIADB_ROOT_PASSWORD="$(cat \
            /run/secrets/energy_db_root_pwd)"
        sh /tmp/10-runtime-grants.sh
    '

root_mount="type=bind,src=$TMP_DIR/energy_db_root_pwd,\
dst=/run/secrets/energy_db_root_pwd,readonly"
app_mount="type=bind,src=$TMP_DIR/energy_db_pwd,\
dst=/run/secrets/energy_db_pwd,readonly"

echo 'Testing privileged migration with protected root secret ...'
docker run --rm --network "$NET" \
    --mount "$root_mount" \
    -e ENGY_DB_NAME=energy_ingest_runtime_test \
    "$IMAGE" database bootstrap

echo 'Testing application authentication with protected secret ...'
docker run --rm --network "$NET" \
    --mount "$app_mount" \
    -e ENGY_DB_NAME=energy_ingest_runtime_test \
    -e ENGY_DB_USER=energy_ingest_runtime_test \
    "$IMAGE" custodian create RuntimeTest

echo 'Testing application authentication ...'
docker run --rm --network "$NET" \
    --mount "$app_mount" \
    -e ENGY_DB_NAME=energy_ingest_runtime_test \
    -e ENGY_DB_USER=energy_ingest_runtime_test \
    "$IMAGE" source list 1

echo 'Testing importer entrypoint ...'
docker run --rm --network "$NET" \
    --mount "$app_mount" \
    -e ENGY_DB_NAME=energy_ingest_runtime_test \
    -e ENGY_DB_USER=energy_ingest_runtime_test \
    --entrypoint /usr/local/bin/engy-st-imp-prod-ep \
    "$IMAGE" --help >/dev/null

echo 'Testing default identity and root rejection ...'
docker run --rm --network none \
    "$IMAGE" --help >/dev/null
if docker run --rm --network none \
    -e ENGY_RUN_USER=root -e ENGY_RUN_GROUP=root \
    "$IMAGE" --help >/dev/null 2>&1; then
    echo 'ERROR: root identity was accepted.' >&2
    exit 1
fi

echo 'PASS: isolated database, secrets, migration, app and identity'
