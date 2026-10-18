#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Repository-wide developer verification entry point.
#
# If formatting, compilation, Clippy, any test category, or ADR conformance
# fails, the script stops and does not append a successful history row. That
# keeps .test-history.csv as a history of completed successful verification
# runs.

set -euo pipefail

cd "$(dirname "$0")/.."

STATUS_FILE=".test-status.md"
HISTORY_FILE=".test-history.csv"

VERIFY_START=$SECONDS
TEMP_DIR=$(mktemp -d)
UNIT_OUTPUT="$TEMP_DIR/unit-tests.txt"
INTEGRATION_OUTPUT="$TEMP_DIR/integration-tests.txt"
DOC_OUTPUT="$TEMP_DIR/doc-tests.txt"
ADR_OUTPUT="$TEMP_DIR/adr-conformance.txt"
MARKDOWN_OUTPUT="$TEMP_DIR/markdown-docs.txt"

cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

test_count() {
    local result_word=$1
    local output_file=$2

    awk -v result_word="$result_word" '
        /^test result:/ {
            for (i = 1; i <= NF; i++) {
                if ($i == result_word) {
                    value = $(i - 1)
                    gsub(/[^0-9]/, "", value)
                    total += value
                }
            }
        }
        END { print total + 0 }
    ' "$output_file"
}

# Keep detailed command output out of the normal terminal summary.
# On failure, display a short excerpt and preserve the full log if desired.
run_check() {
    local label="$1"
    local output="$2"
    shift 2

    local started=$SECONDS
    printf '%s ... ' "$label"
    if "$@" >"$output" 2>&1; then
        printf "PASS (%ss)\n" "$((SECONDS - started))"
    else
        local rc=$?
        printf "FAIL (exit %s; %ss)\n" \
            "$rc" "$((SECONDS - started))"
        tail -n 15 "$output" >&2
        echo "Full output was captured during this run." >&2
        exit "$rc"
    fi
}

FMT_OUTPUT="$TEMP_DIR/format.txt"
CHECK_OUTPUT="$TEMP_DIR/check.txt"
CLIPPY_OUTPUT="$TEMP_DIR/clippy.txt"

if [[ "${ENGY_SKIP_FMT:-0}" == "1" ]]; then
    echo "Formatting ... SKIPPED (ENGY_SKIP_FMT=1; 0s)"
    fmt_result="SKIPPED"
else
    run_check "Formatting" "$FMT_OUTPUT" \
        cargo fmt --all --check
    fmt_result="PASS"
fi

run_check "Compilation" "$CHECK_OUTPUT" cargo check --workspace
check_result="PASS"

run_check "Clippy" "$CLIPPY_OUTPUT" \
    cargo clippy --workspace --all-targets -- -D warnings
clippy_result="PASS"

run_check "Unit tests" "$UNIT_OUTPUT" \
    cargo test --workspace --lib
unit_result="PASS"

unit_passed=$(test_count "passed;" "$UNIT_OUTPUT")
unit_failed=$(test_count "failed;" "$UNIT_OUTPUT")
unit_ignored=$(test_count "ignored;" "$UNIT_OUTPUT")

# Database tests share a disposable MariaDB instance. Run sequentially.
run_check "Integration tests" "$INTEGRATION_OUTPUT" \
    cargo test --workspace --tests -- --test-threads=1
integration_result="PASS"

integration_passed=$(test_count "passed;" "$INTEGRATION_OUTPUT")
integration_failed=$(test_count "failed;" "$INTEGRATION_OUTPUT")
integration_ignored=$(test_count "ignored;" "$INTEGRATION_OUTPUT")

run_check "Documentation tests" "$DOC_OUTPUT" \
    cargo test --workspace --doc
doc_result="PASS"

doc_passed=$(test_count "passed;" "$DOC_OUTPUT")
doc_failed=$(test_count "failed;" "$DOC_OUTPUT")
doc_ignored=$(test_count "ignored;" "$DOC_OUTPUT")

run_check "ADR conformance" "$ADR_OUTPUT" \
    python3 scripts/check_adr_conformance.py
adr_result="PASS"

run_check "Markdown documentation" "$MARKDOWN_OUTPUT" \
    python3 scripts/check-docs.py --strict-metadata
markdown_result="PASS"

# Validate the shipped customer ZIP and fail-closed CLI entry behavior.
PACKAGE_OUTPUT="$TEMP_DIR/package.txt"
GUIDED_OUTPUT="$TEMP_DIR/guided-import.txt"
run_check "Production installer package" "$PACKAGE_OUTPUT" \
    bash scripts/check-prod-package.sh 0.1.4
run_check "Guided import entry checks" "$GUIDED_OUTPUT" \
    bash scripts/test-guided-import-entry.sh
run_check "Guided meter selection" "$GUIDED_OUTPUT" \
    bash scripts/test-guided-meter-selection.sh

adr_count=$(
    awk -F: '/^ADRs found:/ {
        gsub(/[[:space:]]/, "", $2)
        print $2
    }' "$ADR_OUTPUT"
)

adr_errors=$(
    awk -F: '/^Errors:/ {
        gsub(/[[:space:]]/, "", $2)
        print $2
    }' "$ADR_OUTPUT"
)

adr_warnings=$(
    awk -F: '/^Warnings:/ {
        gsub(/[[:space:]]/, "", $2)
        print $2
    }' "$ADR_OUTPUT"
)

tests_passed=$((unit_passed + integration_passed + doc_passed))
tests_failed=$((unit_failed + integration_failed + doc_failed))
tests_ignored=$((unit_ignored + integration_ignored + doc_ignored))
tests_total=$((tests_passed + tests_failed + tests_ignored))

timestamp=$(date --iso-8601=seconds)
branch=$(git branch --show-current)
commit=$(git rev-parse --short HEAD)

if [[ -n "$(git status --porcelain)" ]]; then
    worktree="dirty"
else
    worktree="clean"
fi

cat > "$STATUS_FILE" <<EOF
# Developer Verification Status

**Last successful verification:** $timestamp
**Git branch:** \`$branch\`
**Git commit:** \`$commit\`
**Working tree:** \`$worktree\`

## Verification Results

| Category | Command | Result |
|---|---|---|
| Formatting | \`cargo fmt --all --check\` | $fmt_result |
| Compilation | \`cargo check --workspace\` | $check_result |
| Linting | \`cargo clippy --workspace --all-targets -- -D warnings\` | $clippy_result |
| Unit tests | \`cargo test --workspace --lib\` | $unit_result |
| Integration tests | \`cargo test --workspace --tests -- --test-threads=1\` | $integration_result |
| Documentation tests | \`cargo test --workspace --doc\` | $doc_result |
| ADR conformance | \`python3 scripts/check_adr_conformance.py\` | $adr_result |
| Markdown docs | \`python3 scripts/check-docs.py --strict-metadata\` | $markdown_result |

## Test Counts

| Category | Passed | Failed | Ignored |
|---|---:|---:|---:|
| Unit tests | $unit_passed | $unit_failed | $unit_ignored |
| Integration tests | $integration_passed | $integration_failed | $integration_ignored |
| Documentation tests | $doc_passed | $doc_failed | $doc_ignored |
| **Total** | **$tests_passed** | **$tests_failed** | **$tests_ignored** |

## ADR Conformance

| Metric | Count |
|---|---:|
| ADRs found | $adr_count |
| Errors | $adr_errors |
| Warnings | $adr_warnings |

## Result

**All required developer verification checks passed.**

**Formatting:** $fmt_result (SKIPPED is not a formatting pass).

> This is a local development verification record.
> Formal release testing will be documented separately in \`docs/testing/TEST_PLAN.md\`.
EOF

if [[ ! -f "$HISTORY_FILE" ]]; then
    echo "timestamp,branch,commit,worktree,fmt,check,clippy,unit_tests,unit_passed,unit_failed,unit_ignored,integration_tests,integration_passed,integration_failed,integration_ignored,doc_tests,doc_passed,doc_failed,doc_ignored,adr_conformance,adr_count,adr_errors,adr_warnings,total_tests,total_passed,total_failed,total_ignored" \
        > "$HISTORY_FILE"
fi

echo "$timestamp,$branch,$commit,$worktree,$fmt_result,$check_result,$clippy_result,$unit_result,$unit_passed,$unit_failed,$unit_ignored,$integration_result,$integration_passed,$integration_failed,$integration_ignored,$doc_result,$doc_passed,$doc_failed,$doc_ignored,$adr_result,$adr_count,$adr_errors,$adr_warnings,$tests_total,$tests_passed,$tests_failed,$tests_ignored" \
    >> "$HISTORY_FILE"

echo
printf "Verification passed: %s tests; formatting: %s; ADR errors: %s.\n" \
    "$tests_passed" "$fmt_result" "$adr_errors"
echo "Markdown documentation: $markdown_result"
echo "Production installer package: PASS"
echo "Guided import entry checks: PASS"
echo "Guided meter selection: PASS"
echo "Updated $STATUS_FILE"
echo "Updated $HISTORY_FILE"
printf "Verification elapsed: %ss\n" \
    "$((SECONDS - VERIFY_START))"
