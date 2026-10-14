#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# End-to-end production image test using only sanitized public fixtures.
# No production containers, named volumes, credentials, or XML are used.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="${ENGY_TEST_IMAGE:-energy-ingest:release-check}"
DB_IMAGE="${ENGY_TEST_DB_IMAGE:-mariadb:lts}"
CURL_IMAGE="${ENGY_TEST_CURL_IMAGE:-curlimages/curl:8.16.0}"
FIXTURE_DEFAULT="$ROOT/profiles/sdge/fixtures/"
FIXTURE_DEFAULT+="sdge-electric-one-day-hourly.xml"
FIXTURE="${ENGY_E2E_FIXTURE:-$FIXTURE_DEFAULT}"
DB_NAME=energy_ingest_e2e
DB_USER=energy_ingest_e2e
WORK="$(mktemp -d)"
NAME="engy-e2e-$(openssl rand -hex 5)"
NET="$NAME"
DB="$NAME-db"
API="$NAME-api"

cleanup() {
    docker rm -f "$API" "$DB" >/dev/null 2>&1 || true
    docker network rm "$NET" >/dev/null 2>&1 || true
    rm -rf -- "$WORK"
}
trap cleanup EXIT

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

for cmd in docker openssl python3; do
    command -v "$cmd" >/dev/null || fail "Missing $cmd"
done

[ -f "$FIXTURE" ] || fail "Approved test fixture not found: $FIXTURE"
case "$FIXTURE" in
    "$ROOT"/profiles/sdge/fixtures/*.xml) ;;
    *) fail 'Fixture must be under profiles/sdge/fixtures' ;;
esac

docker image inspect "$IMAGE" >/dev/null || fail "Image missing: $IMAGE"

umask 077
openssl rand -hex 32 >"$WORK/energy_db_pwd"
openssl rand -hex 32 >"$WORK/energy_db_root_pwd"
chmod 600 "$WORK"/*

app_mount="type=bind,src=$WORK/energy_db_pwd,\
dst=/run/secrets/energy_db_pwd,readonly"
root_mount="type=bind,src=$WORK/energy_db_root_pwd,\
dst=/run/secrets/energy_db_root_pwd,readonly"

# These wrappers always use temporary credentials and the internal network.
app() {
    docker run --rm --network "$NET" \
        --mount "$app_mount" \
        -e ENGY_DB_NAME="$DB_NAME" \
        -e ENGY_DB_USER="$DB_USER" \
        "$IMAGE" "$@"
}

http_get() {
    local path="$1"
    local output="$2"
    local with_token="${3:-false}"

    if [ "$with_token" = true ]; then
        # Token lives only in a temporary curl config, not process argv.
        docker run --rm --network "$NET" \
            --user 0:0 \
            --mount "type=bind,src=$WORK/curl-auth.conf,\
dst=/tmp/curl-auth.conf,readonly" \
            "$CURL_IMAGE" -fsS --max-time 10 \
            --config /tmp/curl-auth.conf \
            "http://$API:8080$path" >"$output"
    else
        docker run --rm --network "$NET" \
            "$CURL_IMAGE" -fsS --max-time 10 \
            "http://$API:8080$path" >"$output"
    fi
}

printf 'Starting disposable MariaDB ...\n'
docker network create --internal "$NET" >/dev/null

docker run -d --name "$DB" --network "$NET" \
    --network-alias mariadb \
    --mount "$app_mount" --mount "$root_mount" \
    -e MARIADB_DATABASE="$DB_NAME" \
    -e MARIADB_USER="$DB_USER" \
    -e MARIADB_PASSWORD_FILE=/run/secrets/energy_db_pwd \
    -e MARIADB_ROOT_PASSWORD_FILE=/run/secrets/energy_db_root_pwd \
    --health-cmd='healthcheck.sh --connect --innodb_initialized' \
    --health-interval=2s --health-retries=30 \
    "$DB_IMAGE" >/dev/null

ready=false
for _ in $(seq 1 60); do
    state="$(docker inspect -f '{{.State.Health.Status}}' "$DB")"
    if [ "$state" = healthy ]; then
        ready=true
        break
    fi
    [ "$state" != unhealthy ] || break
    sleep 2
done
[ "$ready" = true ] || fail 'Disposable MariaDB did not become healthy'

# Apply the same restricted grants used by the production installer.
docker cp "$ROOT/deploy/docker-compose/10-runtime-grants.sh" \
    "$DB:/tmp/10-runtime-grants.sh"
docker exec -e MARIADB_DATABASE="$DB_NAME" \
    -e MARIADB_USER="$DB_USER" "$DB" sh -c '
        export MARIADB_ROOT_PASSWORD="$(cat \
            /run/secrets/energy_db_root_pwd)"
        sh /tmp/10-runtime-grants.sh
    '

printf 'Bootstrapping schema and registering disposable source ...\n'
docker run --rm --network "$NET" --mount "$root_mount" \
    -e ENGY_DB_NAME="$DB_NAME" \
    "$IMAGE" database bootstrap

custodian_output="$(app custodian create 'E2E SDGE Fixture')"
custodian_id="$(printf '%s\n' "$custodian_output" | \
    sed -n 's/^Custodian created: id=\([0-9][0-9]*\)$/\1/p')"
[ -n "$custodian_id" ] || fail 'Could not parse custodian ID'

source_output="$(app source create "$custodian_id" 'e2e-golden')"
source_id="$(printf '%s\n' "$source_output" | \
    sed -n 's/^Source created: id=\([0-9][0-9]*\)$/\1/p')"
[ -n "$source_id" ] || fail 'Could not parse source ID'

printf 'Importing one sanitized XML fixture ...\n'
fixture_dir="$(dirname -- "$FIXTURE")"
fixture_file="$(basename -- "$FIXTURE")"
docker run --rm --network "$NET" \
    --mount "$app_mount" \
    --mount "type=bind,src=$fixture_dir,dst=/fixtures,readonly" \
    -e ENGY_DB_NAME="$DB_NAME" \
    -e ENGY_DB_USER="$DB_USER" \
    --entrypoint /usr/local/bin/engy-st-imp-prod-ep \
    "$IMAGE" --source-id "$source_id" "/fixtures/$fixture_file" \
    >"$WORK/import-report"
cat "$WORK/import-report"

# A valid import must accept at least one observation, not just parse XML.
python3 - "$WORK/import-report" <<'PY'
import re
import sys
from pathlib import Path

report = Path(sys.argv[1]).read_text()
match = re.search(r"^TOTAL\s+(\d+)\s+(\d+)\s+(\d+)",
                  report, re.MULTILINE)
if not match or int(match.group(3)) < 1:
    raise SystemExit("FAIL: import accepted zero observations")
print(f"PASS: {match.group(3)} observations accepted")
PY

printf 'Creating disposable API token ...\n'
# Only stdout contains the token. Never print or persist it in Git/logs.
app api-token create e2e-release-check >"$WORK/api-token" \
    2>"$WORK/token-status"
chmod 600 "$WORK/api-token"
[ -s "$WORK/api-token" ] || fail 'Token generation returned empty data'

# Hex/base64url bearer-token alphabet is safe inside quoted curl config.
printf 'header = "Authorization: Bearer %s"\n' \
    "$(cat "$WORK/api-token")" >"$WORK/curl-auth.conf"
chmod 600 "$WORK/curl-auth.conf"

printf 'Creating a meter and attaching compatible imported readings ...\n'
app meter create "$custodian_id" 1 'E2E Electric' \
    >"$WORK/meter-created"
meter_id="$(sed -n 's/.*(\([0-9][0-9]*\))$/\1/p' \
    "$WORK/meter-created")"
[ -n "$meter_id" ] || fail 'Could not parse meter ID'
app meter attach-compatible "$meter_id" --apply \
    >"$WORK/meter-attach"

printf 'Starting authenticated production API ...\n'
docker run -d --name "$API" --network "$NET" \
    --mount "$app_mount" \
    -e ENGY_DB_NAME="$DB_NAME" \
    -e ENGY_DB_USER="$DB_USER" \
    -e ENGY_API_LISTEN=0.0.0.0:8080 \
    --entrypoint /usr/local/bin/engy-api-prod-ep \
    "$IMAGE" >/dev/null

api_ready=false
for _ in $(seq 1 30); do
    if docker run --rm --network "$NET" "$CURL_IMAGE" \
        -fsS --max-time 3 "http://$API:8080/healthz" \
        >/dev/null 2>&1; then
        api_ready=true
        break
    fi
    sleep 2
done
[ "$api_ready" = true ] || fail 'API did not become healthy'

http_get /v1/meters "$WORK/meters.json" true
http_get "/v1/meters/$meter_id/stream" \
    "$WORK/stream.json" true

python3 - "$WORK/meters.json" "$WORK/stream.json" "$meter_id" <<'PY'
import json
import sys
from pathlib import Path

meters = json.loads(Path(sys.argv[1]).read_text())
stream = json.loads(Path(sys.argv[2]).read_text())
meter_id = int(sys.argv[3])

assert any(item["id"] == meter_id for item in meters), \
    "meter absent from authenticated API response"
assert stream["physical_meter_id"] == meter_id, \
    "stream references the wrong meter"
assert stream["intervals"], "no imported intervals returned by API"
print(f"PASS: authenticated API returned "
      f"{len(stream['intervals'])} intervals")
PY

printf 'PASS: sanitized XML ingestion and authenticated API query\n'
