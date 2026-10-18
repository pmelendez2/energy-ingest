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
    docker run --rm -i --network "$NET" \
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

# The guided customer workflow registers real meters before importing XML.
# Two same-kind meters must not trigger a guessed resource association.
printf 'Registering two meters before XML import ...\n'
app meter create "$custodian_id" 1 'E2E Electric' \
    >"$WORK/meter-created"
meter_id="$(sed -n 's/.*(\([0-9][0-9]*\))$/\1/p' \
    "$WORK/meter-created")"
[ -n "$meter_id" ] || fail 'Could not parse meter ID'
app meter create "$custodian_id" 1 'E2E Second Electric' \
    >"$WORK/second-meter-created"
second_meter_id="$(sed -n 's/.*(\([0-9][0-9]*\))$/\1/p' \
    "$WORK/second-meter-created")"
[ -n "$second_meter_id" ] || fail 'Could not parse second meter ID'

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

printf 'Reimporting identical fixture to verify idempotency ...\n'
docker run --rm --network "$NET" \
    --mount "$app_mount" \
    --mount "type=bind,src=$fixture_dir,dst=/fixtures,readonly" \
    -e ENGY_DB_NAME="$DB_NAME" \
    -e ENGY_DB_USER="$DB_USER" \
    --entrypoint /usr/local/bin/engy-st-imp-prod-ep \
    "$IMAGE" --source-id "$source_id" "/fixtures/$fixture_file" \
    >"$WORK/reimport-report"
python3 - "$WORK/reimport-report" <<'PY_REIMPORT'
import re
import sys
from pathlib import Path

report = Path(sys.argv[1]).read_text()
match = re.search(r"^TOTAL\s+(\d+)\s+(\d+)\s+(\d+)",
                  report, re.MULTILINE)
if not match:
    raise SystemExit("FAIL: reimport summary is missing")
observed, unchanged, accepted = map(int, match.groups())
if observed == 0 or unchanged == 0 or accepted != 0:
    raise SystemExit(
        "FAIL: identical reimport was not idempotent: "
        f"observed={observed}, unchanged={unchanged}, "
        f"accepted={accepted}"
    )
print(f"PASS: {unchanged} unchanged observations on reimport")
PY_REIMPORT

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

printf 'Verifying imported resources are unassociated ...\n'
app meter resources "$meter_id" >"$WORK/initial-meter-resources"
app meter resources "$second_meter_id" \
    >"$WORK/initial-second-resources"
if grep -Eq '^[0-9]+$' "$WORK/initial-meter-resources" || \
   grep -Eq '^[0-9]+$' "$WORK/initial-second-resources"; then
    fail 'An imported resource was associated without confirmation'
fi
printf 'PASS: no automatic association with multiple meters\n'

# Read the imported resource identifiers from the disposable database.
# This is diagnostic test logic, never an instruction to end users.
resource_ids="$(docker exec -i "$DB" sh -c '
    export MYSQL_PWD="$(cat /run/secrets/energy_db_pwd)"
    exec mariadb --batch --skip-column-names \
        -u "$MARIADB_USER" "$MARIADB_DATABASE"
' <<SQL_RESOURCES
SELECT mr.id FROM meter_reading mr
JOIN usage_point up ON up.id = mr.usage_point_id
WHERE up.acquisition_source_id = $source_id
ORDER BY mr.id;
SQL_RESOURCES
)"
[ -n "$resource_ids" ] || fail 'Import produced no meter resources'
resource_id="$(printf '%s\n' "$resource_ids" | head -n 1)"
[[ "$resource_id" =~ ^[1-9][0-9]*$ ]] || \
    fail 'Invalid imported resource identifier'

# Model customer Skip: don't call attach at all, assert zero mappings.
printf 'Verifying a skipped resource remains unassociated ...\n'
app meter resources "$second_meter_id" \
    >"$WORK/skipped-resources"
if grep -Eq '^[0-9]+$' "$WORK/skipped-resources"; then
    fail 'Skipped resource was unexpectedly assigned'
fi
printf 'PASS: skipped meter selection did not change ownership\n'

# An explicit customer selection maps the chosen imported resource.
printf 'Associating selected resource with confirmed meter ...\n'
app meter attach "$meter_id" "$resource_id" \
    >"$WORK/selected-attach"
app meter resources "$meter_id" >"$WORK/attached-resources"
grep -qx "$resource_id" "$WORK/attached-resources" || \
    fail 'Confirmed meter association was not persisted'
printf 'PASS: explicitly selected meter owns imported readings\n'

printf 'Verifying private meter number and ownership isolation ...\n'
number='E2E-METER-PRIVATE-0294'
printf '%s\n' "$number" | \
    app meter set-number "$meter_id" --stdin \
    >"$WORK/meter-number-output" \
    2>"$WORK/meter-number-error"
if grep -Fq "$number" "$WORK/meter-number-output" || \
   grep -Fq "$number" "$WORK/meter-number-error"; then
    fail 'Utility meter number was exposed in administrative output'
fi
# Read the persisted value privately from disposable MariaDB; do not
# echo the number into test logs.
persisted_number="$(docker exec -i "$DB" sh -c '
    export MYSQL_PWD="$(cat /run/secrets/energy_db_pwd)"
    exec mariadb --batch --skip-column-names \
        -u "$MARIADB_USER" "$MARIADB_DATABASE"
' <<SQL_NUMBER
SELECT meter_number FROM physical_meter WHERE id = $meter_id;
SQL_NUMBER
)"
[ "$persisted_number" = "$number" ] || \
    fail 'Utility meter number was not persisted correctly'
printf 'PASS: utility meter number persisted without logging it\n'


# A second meter of the same commodity must not be allowed to take
# ownership of an already associated XML resource.
if app meter attach "$second_meter_id" "$resource_id" \
    >"$WORK/conflict-out" 2>"$WORK/conflict-err"; then
    fail 'Resource could be reassigned to a different physical meter'
fi
app meter resources "$second_meter_id" \
    >"$WORK/second-meter-resources"
if grep -Eq '^[0-9]+$' "$WORK/second-meter-resources"; then
    fail 'Conflicting resource was added to the second meter'
fi
printf 'PASS: duplicate association rejected; second meter remains empty\n'


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
