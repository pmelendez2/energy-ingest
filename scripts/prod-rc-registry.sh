#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# AI-assisted: ChatGPT / GPT-6; 2026-10-20.
# Authenticated Docker Registry V2 tag check. Only a 404 is available.

rc_tag_is_absent() {
    local registry="$1" repo="$2" version="$3"
    local user="$4" password="$5"
    local config status rc
    config="$(mktemp)" || return 1
    chmod 600 "$config"
    # Curl configuration is private, unlike CLI --user arguments.
    # Reject control characters and escape config string delimiters.
    if [[ "$user" == *[$'\n'$'\r']* || \
          "$password" == *[$'\n'$'\r']* ]]; then
        rm -f -- "$config"
        echo 'Registry credentials contain a newline.' >&2
        return 1
    fi
    local auth="${user}:${password}"
    auth="${auth//\\/\\\\}"
    auth="${auth//\"/\\\"}"
    printf 'user = "%s"\n' "$auth" > "$config"

    local accept
    accept='application/vnd.oci.image.index.v1+json'
    accept+=', application/vnd.docker.distribution.manifest.list.v2+json'
    accept+=', application/vnd.oci.image.manifest.v1+json'
    accept+=', application/vnd.docker.distribution.manifest.v2+json'
    rc=0
    status="$(curl --silent --show-error --max-time 15 \
        --config "$config" --output /dev/null \
        --write-out '%{http_code}' \
        --header "Accept: $accept" \
        --url "http://${registry}/v2/${repo}/manifests/${version}")" || rc=$?
    rm -f -- "$config"
    if ((rc != 0)); then
        echo "Registry request failed (curl exit $rc)." >&2
        return 1
    fi
    case "$status" in
        404) return 0 ;;
        200)
            echo 'Release-candidate tag already exists.' >&2
            return 1 ;;
        *)
            echo "Registry preflight failed (HTTP $status)." >&2
            return 1 ;;
    esac
}
