#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Guided import of Green Button XML files into the production database.
# Uses the existing production import and meter-management scripts.

set -euo pipefail

# Resolve paths relative to this script, not the caller's directory.
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
COMPOSE_DIR="$ROOT_DIR/deploy/docker-compose"
DEFAULT_IMPORT_DIR="$COMPOSE_DIR/private-imports"


# Display customer-facing command-line usage.
usage() {
    cat <<'HELP'
Usage: ./scripts/energy-import.sh [--import-dir DIR] [--file GLOB]
                                  [--source-id ID]

Guided, interactive import. The default file pattern is '*.xml'.
For noninteractive imports, use import-prod.sh with prepared files.
HELP
}


# Report an error and stop without attempting another operation.
die() {
    printf 'Error: %s\n' "$*" >&2
    exit 2
}


# Require affirmative consent; Enter, No, or EOF means decline.
confirm() {
    local answer

    read -r -p "$1 [y/N]: " answer || return 1
    [[ "$answer" == [yY] || "$answer" == [yY][eE][sS] ]]
}


# Parse the guided-import options.
import_dir="$DEFAULT_IMPORT_DIR"
file_pattern='*.xml'
source_id=''

while (($#)); do
    case "$1" in
        --help|-h)
            usage
            exit 0
            ;;
        --import-dir|--file|--source-id)
            (($# >= 2)) || die "Missing value for $1"

            case "$1" in
                --import-dir) import_dir="$2" ;;
                --file) file_pattern="$2" ;;
                --source-id) source_id="$2" ;;
            esac

            shift 2
            ;;
        *)
            die "Unknown option: $1"
            ;;
    esac
done


# Validate the execution environment and requested XML selection.
[[ -t 0 && -t 1 ]] || die 'An interactive terminal is required.'
[[ -f "$COMPOSE_DIR/.env.prod" ]] ||
    die 'Complete setup-prod.sh first.'
[[ -d "$import_dir" ]] || die "Directory not found: $import_dir"
[[ "$file_pattern" != */* && "$file_pattern" == *.xml ]] ||
    die 'Use an XML basename or a quoted *.xml glob.'
[[ -z "$source_id" || "$source_id" =~ ^[1-9][0-9]*$ ]] ||
    die 'Invalid source ID.'

import_dir="$(cd -- "$import_dir" && pwd -P)"
mkdir -p -- "$DEFAULT_IMPORT_DIR"
[[ ! -L "$DEFAULT_IMPORT_DIR" ]] ||
    die 'The managed import folder must not be a symlink.'
staging_dir="$(cd -- "$DEFAULT_IMPORT_DIR" && pwd -P)"

# Select only regular XML files; do not follow source-file symlinks.
shopt -s nullglob
files=()

for path in "$import_dir"/*; do
    name="${path##*/}"
    [[ "$name" == $file_pattern && -f "$path" && ! -L "$path" ]] ||
        continue
    files+=("$name")
done

((${#files[@]})) || die 'No regular XML files selected.'

printf '\nEnergy Ingest — Guided Import\n'
printf 'Selected %s XML files from %s\n' \
    "${#files[@]}" "$import_dir"
printf '  %s\n' "${files[@]}"
printf '\nHave utility meter numbers available.\n'
printf 'Keep original XML files outside the installation folder.\n'
printf 'Installation cleanup can remove staged copies.\n'


# Production build defines the application user/group. Resolve numeric IDs
# from that identity in the published image; do not duplicate Dockerfile
# build defaults in this customer-facing wrapper.
cd -- "$COMPOSE_DIR"
compose=(
    docker compose -p energy-ingest
    --env-file .env.prod
    -f compose.prod.yml
)

runtime_identity="$("${compose[@]}" run --quiet --rm --no-deps \
    --entrypoint /bin/sh import -c '
        id -u "$ENGY_RUN_USER"
        id -g "$ENGY_RUN_USER"
    ')" || die 'Unable to determine container runtime identity.'

mapfile -t runtime_ids <<< "$runtime_identity"
((${#runtime_ids[@]} == 2)) || die 'Invalid runtime identity.'
run_uid="${runtime_ids[0]}"
run_gid="${runtime_ids[1]}"
[[ "$run_uid" =~ ^[0-9]+$ && "$run_gid" =~ ^[0-9]+$ ]] ||
    die 'Invalid container UID/GID.'

# Managed staging must be on local storage. Do not change permissions on
# NFS files or source directories, even if they are writable.
filesystem_type="$(findmnt -n -T "$staging_dir" -o FSTYPE)"
[[ "$filesystem_type" != nfs* ]] ||
    die 'Managed staging must be on a local filesystem.'

# Grant the effective container group access to the managed directory.
# For copies created/managed by this workflow, staging consent includes
# secure permissions. Files placed directly here by the customer require
# a separate authorization before any permission changes.
set_managed_directory_access() {
    if [[ "$(stat -c %g "$staging_dir")" != "$run_gid" ]]; then
        sudo chgrp -- "$run_gid" "$staging_dir" ||
            die 'Unable to set staging group.'
    fi
    chmod 750 -- "$staging_dir" || die 'Unable to protect staging.'
}

set_managed_file_access() {
    local name="$1" path="$staging_dir/$1"
    [[ -f "$path" && ! -L "$path" ]] ||
        die "Unexpected staged file: $name"
    if [[ "$(stat -c %g "$path")" != "$run_gid" ]]; then
        sudo chgrp -- "$run_gid" "$path" ||
            die "Unable to set staging group: $name"
    fi
    chmod 640 -- "$path" || die "Unable to protect staged file: $name"
}

# Stage external files locally only after the customer's approval.
# Original NFS or other external files are never changed.
if [[ "$import_dir" != "$staging_dir" ]]; then
    printf '\nFiles require secure staging in %s\n' "$staging_dir"
    printf 'Original files will not be modified.\n'
    printf 'Staging applies private file permissions.\n'
    printf 'Staged copies may be deleted with the installation folder.\n'
    printf 'Declining staging cancels the import.\n'

    if ! confirm 'Copy files into managed staging?'; then
        printf 'Cancelled: no database import attempted.\n'
        exit 0
    fi

    # Reuse identical staged files when importing the same downloads.
    # Replacements require a separate decision and never alter originals.
    changed=()

    for name in "${files[@]}"; do
        [[ ! -L "$staging_dir/$name" ]] ||
            die "Staged file is a symlink: $name"

        if [[ -e "$staging_dir/$name" ]]; then
            [[ -f "$staging_dir/$name" ]] ||
                die "Staged path is not a regular file: $name"

            if cmp -s -- "$import_dir/$name" "$staging_dir/$name"; then
                printf 'Reusing identical staged file: %s\n' "$name"
            else
                changed+=("$name")
            fi
        fi
    done

    if ((${#changed[@]})); then
        printf '\n%s staged file(s) have different contents:\n' \
            "${#changed[@]}"
        printf '  %s\n' "${changed[@]}"
        printf 'Replacing them changes ONLY the local staged copies.\n'
        printf 'Original downloaded XML files will not be modified.\n'
        printf 'Declining cancels the import; no database import occurs.\n'

        if ! confirm 'Replace the changed staged copies?'; then
            printf 'Cancelled: no database import attempted.\n'
            exit 0
        fi
    fi

    set_managed_directory_access

    # Copy via a temporary file; rename only after a complete copy.
    # Permissions are configured before publishing the staged copy.
    for name in "${files[@]}"; do
        if [[ -e "$staging_dir/$name" ]] &&
            cmp -s -- "$import_dir/$name" "$staging_dir/$name"; then
            set_managed_file_access "$name"
            continue
        fi

        staged_tmp="$(mktemp "$staging_dir/.energy-import.XXXXXXXX")"
        if ! install -m 600 -- "$import_dir/$name" "$staged_tmp"; then
            rm -f -- "$staged_tmp"
            die "Unable to stage: $name"
        fi
        if [[ "$(stat -c %g "$staged_tmp")" != "$run_gid" ]]; then
            sudo chgrp -- "$run_gid" "$staged_tmp" || {
                rm -f -- "$staged_tmp"
                die "Unable to protect staged copy: $name"
            }
        fi
        chmod 640 -- "$staged_tmp"
        mv -f -- "$staged_tmp" "$staging_dir/$name"
    done
fi

# Verify readability as the actual application user. Compose may apply
# overrides; testing the effective UID/GID avoids host-root false passes.
can_read_files() {
    local name

    for name in "${files[@]}"; do
        ENGY_IMPORT_DIR="$staging_dir" "${compose[@]}" \
            run --quiet --rm --no-deps \
            --user "$run_uid:$run_gid" \
            --entrypoint /bin/sh import \
            -c 'test -r "/imports/$1"' sh "$name" || return 1
    done
}

# Files placed directly in private-imports are customer-managed until
# they authorize access changes. Do not silently rewrite their modes.
if ! can_read_files; then
    if [[ "$import_dir" != "$staging_dir" ]]; then
        die 'Staged files are inaccessible. No database import attempted.'
    fi

    printf '\nThe import container cannot read all selected XML files.\n'
    printf 'Proposed local changes: group %s, directory 750,\n' \
        "$run_gid"
    printf 'XML files group %s, mode 640.\n' "$run_gid"
    printf 'Originals outside this folder will not be modified.\n'
    printf 'IF DECLINED: the XML import WILL NOT proceed or succeed.\n'

    if ! confirm 'Authorize local permission changes?'; then
        printf 'Cancelled: no database import attempted.\n'
        exit 0
    fi

    set_managed_directory_access
    for name in "${files[@]}"; do
        set_managed_file_access "$name"
    done

    can_read_files ||
        die 'Files remain inaccessible. No database import attempted.'
fi

# Resolve acquisition-source registration as part of guided onboarding.
# Keep database registration after file-access checks, so a declined
# permission request cannot create a partially registered import.
list_sources() {
    "${compose[@]}" exec -T mariadb sh -c '
        export MYSQL_PWD="$(cat /run/secrets/energy_db_pwd)"
        mariadb --batch --skip-column-names \
            -u "$MARIADB_USER" "$MARIADB_DATABASE" -e "
            SELECT s.id, c.name, s.name
            FROM acquisition_source AS s
            JOIN data_custodian AS c
              ON c.id = s.data_custodian_id
            ORDER BY s.id;
        "
    '
}

# Source names can contain whitespace; use tab-separated database rows.
# Reject control characters in names when registering new records.
source_rows="$(list_sources)" || die 'Cannot list acquisition sources.'
source_records=()
if [[ -n "$source_rows" ]]; then
    mapfile -t source_records <<< "$source_rows"
fi

printf '\nUtility registration\n'
if ((${#source_records[@]} == 0)); then
    [[ -z "$source_id" ]] ||
        die 'No sources exist; the supplied --source-id is invalid.'

    printf 'No utility is registered yet.\n'
    printf 'Enter the utility providing the XML and how you got it.\n'
    read -r -p 'Utility name (e.g., SDG&E): ' utility || exit 2
    read -r -p 'Source label (e.g., Green Button Download): ' \
        source_name || exit 2

    [[ -n "${utility//[[:space:]]/}" ]] ||
        die 'Utility name cannot be blank.'
    [[ -n "${source_name//[[:space:]]/}" ]] ||
        die 'Source name cannot be blank.'
    [[ "$utility" != *[$'\t\r\n']* ]] || die 'Invalid utility name.'
    [[ "$source_name" != *[$'\t\r\n']* ]] ||
        die 'Invalid source name.'

    printf 'Utility: %s\nSource: %s\n' "$utility" "$source_name"
    if ! confirm 'Register this utility and import method?'; then
        printf 'Cancelled: no database import attempted.\n'
        exit 0
    fi

    # Reuse the existing registration command, which manages the
    # database connection and application permissions.
    # A previously registered custodian may have no acquisition source.
    # Reuse it rather than attempting to create a duplicate custodian.
    custodian_rows="$("${compose[@]}" exec -T mariadb sh -c '
        export MYSQL_PWD="$(cat /run/secrets/energy_db_pwd)"
        mariadb --batch --skip-column-names \
            -u "$MARIADB_USER" "$MARIADB_DATABASE" -e \
            "SELECT id, name FROM data_custodian ORDER BY id"
    ')" || die 'Cannot inspect registered utilities.'

    custodian_id=''
    while IFS=$'\t' read -r id name; do
        if [[ "$name" == "$utility" ]]; then
            custodian_id="$id"
            break
        fi
    done <<< "$custodian_rows"

    if [[ -n "$custodian_id" ]]; then
        printf 'Using existing utility registration: %s\n' "$utility"
        "${compose[@]}" run --rm --no-deps admin \
            source create "$custodian_id" "$source_name" ||
            die 'Unable to register acquisition source.'
    else
        "$SCRIPT_DIR/register-prod.sh" \
            --utility "$utility" --source "$source_name" ||
            die 'Registration incomplete; review sources before retry.'
    fi

    source_rows="$(list_sources)" || die 'Cannot verify registration.'
    mapfile -t source_records <<< "$source_rows"
fi

if [[ -n "$source_id" ]]; then
    selected=''
    for record in "${source_records[@]}"; do
        IFS=$'\t' read -r id utility source_name <<< "$record"
        if [[ "$id" == "$source_id" ]]; then
            selected="$record"
            break
        fi
    done
    [[ -n "$selected" ]] || die 'Selected source ID is not registered.'
elif ((${#source_records[@]} == 1)); then
    selected="${source_records[0]}"
    IFS=$'\t' read -r source_id utility source_name <<< "$selected"
else
    printf 'Multiple acquisition sources are registered:\n'
    for record in "${source_records[@]}"; do
        IFS=$'\t' read -r id utility source_name <<< "$record"
        printf '  %s — %s (source ID %s)\n' \
            "$utility" "$source_name" "$id"
    done

    read -r -p 'Enter the ID shown beside the correct utility: ' \
        source_id || exit 2
    [[ "$source_id" =~ ^[1-9][0-9]*$ ]] ||
        die 'Choose a valid positive source ID.'
    selected=''
    for record in "${source_records[@]}"; do
        IFS=$'\t' read -r id utility source_name <<< "$record"
        if [[ "$id" == "$source_id" ]]; then
            selected="$record"
            break
        fi
    done
    [[ -n "$selected" ]] || die 'Source ID is not registered.'
fi

IFS=$'\t' read -r source_id utility source_name <<< "$selected"
printf 'Import destination:\n'
printf '  Utility: %s\n' "$utility"
printf '  Acquisition source: %s\n' "$source_name"
printf '  Selected: %s — %s\n' \
    "$utility" "$source_name"

# Database IDs are internal. Never print them during guided onboarding.
run_readonly_sql() {
    local query="$1"
    "${compose[@]}" exec -T mariadb sh -c '
        export MYSQL_PWD="$(cat /run/secrets/energy_db_pwd)"
        exec mariadb --batch --skip-column-names \
            -u "$MARIADB_USER" "$MARIADB_DATABASE"
    ' <<< "$query"
}

source_custodian_id="$(run_readonly_sql "
    SELECT data_custodian_id FROM acquisition_source
    WHERE id = $source_id;
")"
[[ "$source_custodian_id" =~ ^[1-9][0-9]*$ ]] ||
    die 'Cannot resolve utility for meter registration.'

# Register customer-defined physical meters before the import transaction.
# Do not infer meter identity or commodity from XML filenames.
register_meters() {
    local kind="$1" code="$2" label="$3"
    local rows count id name meter_id name_hex

    printf '\n%s meter registration\n' "$label"
    printf '%s\n' '--------------------------'
    rows="$(run_readonly_sql "
        SELECT id, display_name FROM physical_meter
        WHERE data_custodian_id = $source_custodian_id
          AND commodity = $code ORDER BY id;
    ")" || die 'Cannot list registered meters.'
    count=0
    if [[ -n "$rows" ]]; then
        while IFS=$'\t' read -r id name; do
            [[ "$id" =~ ^[1-9][0-9]*$ ]] ||
                die 'Invalid meter record.'
            printf '  Existing meter: %s\n' "$name"
            count=$((count + 1))
        done <<< "$rows"
    fi
    if ((count == 0)); then
        if ! confirm "Register a $label meter for this utility?"; then
            printf 'Skipped %s registration.\n' "$label"
            return 0
        fi
    else
        printf '%s registered %s meter(s).\n' "$count" "$label"
        if ! confirm "Register another $label meter?"; then
            return 0
        fi
    fi

    while :; do
        read -r -p "Friendly $label meter name: " name ||
            die 'Meter registration cancelled.'
        [[ -n "${name//[[:space:]]/}" && ${#name} -le 255 ]] ||
            die 'Provide a friendly meter name (up to 255 characters).'
        printf 'Register %s meter: %s\n' "$label" "$name"
        confirm 'Create this meter?' ||
            die 'Meter registration declined; import not started.'
        "$SCRIPT_DIR/meter-prod.sh" create \
            --custodian "$source_custodian_id" \
            --kind "$kind" --name "$name" >/dev/null ||
            die 'Could not create meter.'
        # Determine the record by customer-provided unique name.
        # Do not ask the customer to handle database IDs.
        name_hex="$(printf %s "$name" | od -An -tx1 | tr -d ' \n')"
        rows="$(run_readonly_sql "
            SELECT id FROM physical_meter
            WHERE data_custodian_id = $source_custodian_id
              AND commodity = $code
              AND display_name =
                  CONVERT(UNHEX('$name_hex') USING utf8mb4);
        ")" || die 'Cannot verify new meter.'
        [[ "$rows" =~ ^[1-9][0-9]*$ ]] ||
            die 'Cannot uniquely identify newly registered meter.'
        meter_id="$rows"
        printf 'Enter the actual number from your utility records.\n'
        "$SCRIPT_DIR/meter-prod.sh" set-number --meter "$meter_id" ||
            die 'Utility meter number was not saved.'
        printf '%s meter registered.\n' "$label"
        if ! confirm "Register another $label meter?"; then
            break
        fi
    done
}

printf '\nUtility and physical-meter registration\n'
printf '%s\n' '---------------------------------------'
printf 'Utility: %s\n' "$utility"
printf 'Import method: %s\n' "$source_name"
printf 'Use actual meter numbers from your utility records.\n'
register_meters electric 1 Electricity
register_meters gas 7 'Natural gas'

# Confirm the database operation separately from filesystem access.
printf '\nAccess verified. Ready to import %s files.\n' \
    "${#files[@]}"

if ! confirm 'Proceed with database import?'; then
    printf 'Cancelled: no database import attempted.\n'
    exit 0
fi

import_args=(--import-dir "$staging_dir")
import_args+=(--source-id "$source_id")

for name in "${files[@]}"; do
    import_args+=(--file "$name")
done

"$SCRIPT_DIR/import-prod.sh" "${import_args[@]}"


# Physical-meter association is required to complete guided onboarding.
# The user confirms identity. Matching consumption values alone never
# establish that two resources came from the same physical meter.
printf '\n\nPhysical-meter setup (required)\n'
printf '%s\n' '--------------------------------'
printf 'Use the meter numbers shown on utility records.\n'
printf 'Utility: %s — %s\n' "$utility" "$source_name"
printf 'Imported XML file data must be assigned to the\n'
printf 'correct physical meter before usage is ready for display.\n'
printf 'This step requires customer confirmation of meter identity.\n'
printf '\n'

# Query unassigned resources for the selected acquisition source,
# broken down by commodity. Unsupported kinds must be reviewed.
unassigned_summary() {
    run_readonly_sql "
        SELECT COALESCE(rt.commodity, 0), COUNT(*)
        FROM meter_reading AS mr
        JOIN usage_point AS up ON up.id = mr.usage_point_id
        JOIN reading_type AS rt ON rt.id = mr.reading_type_id
        LEFT JOIN physical_meter_resource AS pmr
          ON pmr.meter_reading_id = mr.id
        WHERE up.acquisition_source_id = $source_id
          AND pmr.meter_reading_id IS NULL
        GROUP BY COALESCE(rt.commodity, 0)
        ORDER BY COALESCE(rt.commodity, 0);
    "
}

# Prompt only when this source has unmapped resources for the kind.
# If more than one physical meter of the same kind is registered,
# attach-compatible is unsafe: it would mix separate physical meters.
setup_kind() {
    local kind="$1" code="$2" label="$3" count="$4"
    local rows meter_id meter_count id display_name
    [[ "$count" =~ ^[0-9]+$ ]] || die 'Invalid resource count.'
    ((count > 0)) || return 0

    printf '\n%s file data association\n' "$label"
    printf '%s\n' '-----------------------------'
    printf 'Unassociated file data entries: %s\n' "$count"
    rows="$(run_readonly_sql "
        SELECT id, display_name FROM physical_meter
        WHERE data_custodian_id = $source_custodian_id
          AND commodity = $code ORDER BY id;
    ")" || die 'Unable to list physical meters.'
    meter_count=0
    meter_id=''
    if [[ -n "$rows" ]]; then
        while IFS=$'\t' read -r id display_name; do
            [[ "$id" =~ ^[1-9][0-9]*$ ]] ||
                die 'Invalid meter record.'
            printf '  Registered meter: %s\n' "$display_name"
            meter_count=$((meter_count + 1))
            meter_id="$id"
        done <<< "$rows"
    fi
    if ((meter_count == 0)); then
        die "Register a $label meter before associating resources."
    fi
    if ((meter_count > 1)); then
        printf 'More than one %s meter is registered.\n' "$label"
        printf 'No automatic associations will be made.\n'
        printf 'Use meter-prod.sh to review individual file data entries.\n'
        die 'Individual meter association is required.'
    fi
    printf 'Review the imported file data before confirming association.\n'
    printf 'An XML file may contain multiple data entries.\n'
    "$SCRIPT_DIR/meter-prod.sh" attach-compatible \
        --meter "$meter_id" ||
        die 'Meter association failed; import is preserved.'
}

summary="$(unassigned_summary)" ||
    die 'Cannot determine unassociated imported resources.'
electric_count=0
gas_count=0
unsupported_count=0

if [[ -n "$summary" ]]; then
    while IFS=$'\t' read -r code count; do
        case "$code" in
            1) electric_count="$count" ;;
            7) gas_count="$count" ;;
            *) unsupported_count=$((unsupported_count + count)) ;;
        esac
    done <<< "$summary"
fi

((unsupported_count == 0)) ||
    die 'Unsupported commodity detected; onboard manually.'

setup_kind electric 1 Electricity "$electric_count"
setup_kind gas 7 'Natural gas' "$gas_count"

remaining="$(unassigned_summary)" ||
    die 'Unable to verify resource mapping.'
if [[ -n "$remaining" ]]; then
    printf '\nSome imported file data remains unassociated:\n%s\n' "$remaining"
    die 'Onboarding incomplete. Resolve meter associations before usage.'
fi

printf '\n\nImport and meter association completed.\n'
printf 'All imported file data is associated with meters.\n'
printf 'Stream validation is still required before using totals.\n'
