#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Supplementary release audit for production images and source paths.
# This is not a substitute for a vulnerability or image-layer scan.

set -euo pipefail

readonly IMAGE="${1:-energy-ingest:release-check}"
readonly IGNORE_FILE='docker/Dockerfile.prod.dockerignore'
readonly SAFE_SECRET_PLACEHOLDER='deploy/docker-compose/prod-secrets/.gitignore'

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

pass() {
    printf 'PASS: %s\n' "$*"
}

require_image() {
    docker image inspect "$IMAGE" >/dev/null 2>&1 ||
        fail "Production image not found: $IMAGE"
}

check_tracked_paths() {
    local path
    local -a unsafe_paths=()

    # Inspect filenames only. Never open or print sensitive file data.
    while IFS= read -r -d '' path; do
        # This file contains ignore rules, not production credentials.
        if [[ "$path" == "$SAFE_SECRET_PLACEHOLDER" ]]; then
            continue
        fi

        if [[ "$path" =~ (^|/)(prod-secrets|original-downloads)/ ]] ||
           [[ "$path" =~ (^|/)\.env\.prod$ ]] ||
           [[ "$path" =~ \.(age|p12|pfx)$ ]]; then
            unsafe_paths+=("$path")
        fi
    done < <(git ls-files -z)

    if (( ${#unsafe_paths[@]} > 0 )); then
        printf 'Disallowed tracked paths:\n' >&2
        printf '  %s\n' "${unsafe_paths[@]}" >&2
        fail 'Sensitive paths are tracked by Git'
    fi

    pass 'No disallowed sensitive paths tracked by Git'
}

require_ignore_rule() {
    local pattern="$1"
    local explanation="$2"

    # Match an actual Docker ignore rule, not a comment or substring.
    if ! grep -Fxq -- "$pattern" "$IGNORE_FILE"; then
        fail "Missing $explanation exclusion: $pattern"
    fi
}

check_build_context() {
    [[ -f "$IGNORE_FILE" ]] ||
        fail "Missing production Docker ignore file: $IGNORE_FILE"

    # Docker recognizes Dockerfile-specific ignore files for builds
    # using docker/Dockerfile.prod as their Dockerfile.
    require_ignore_rule '.git' 'Git metadata'
    require_ignore_rule 'deploy/docker-compose/prod-secrets' \
        'production credential directory'
    require_ignore_rule 'deploy/docker-compose/.env*' \
        'production and other local environment files'

    pass 'Required production build-context exclusions exist'
}

check_runtime_identity() {
    # Validate the configured user is non-root without assuming its name.
    docker run --rm --network none \
        --entrypoint /bin/sh "$IMAGE" -ec '
            : "${ENGY_RUN_USER:?Missing runtime user}"
            : "${ENGY_RUN_GROUP:?Missing runtime group}"

            uid="$(gosu \
                "$ENGY_RUN_USER:$ENGY_RUN_GROUP" id -u)"
            test "$uid" -ne 0
        ' || fail 'Application runtime identity is root or invalid'

    pass 'Configured application identity is non-root'
}

check_image_history() {
    local history

    # Inspect build commands without printing potential credential data.
    history="$(docker history --no-trunc \
        --format '{{.CreatedBy}}' "$IMAGE")" ||
        fail 'Unable to inspect image history'

    if grep -Eiq \
        '(password|secret|token)[[:space:]]*=[[:space:]]*[^ ]+' \
        <<<"$history"; then
        fail 'Credential-like assignment found in Docker history'
    fi

    pass 'No obvious credential assignments in image history'
}

main() {
    require_image
    check_tracked_paths
    check_build_context
    check_runtime_identity
    check_image_history

    pass 'Production release supplementary audit complete'
}

main "$@"
