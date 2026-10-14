#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# AI-assisted: ChatGPT / GPT-6; 2026-10-09.
#
# Create a repeatable source review ZIP for the energy_ingest project.
#
# Usage:
#   bash scripts/create_review_zip.sh
#   bash scripts/create_review_zip.sh --output /tmp/review.zip
#   bash scripts/create_review_zip.sh --audit-fixtures
#   bash scripts/create_review_zip.sh --audit-fixtures \
#       --report-output /tmp/private-xml-review.txt
#   bash scripts/create_review_zip.sh --approve-fixtures
#   bash scripts/create_review_zip.sh --include-fixtures
#
# The default archive contains Git-tracked source, docs, examples, and
# .gitignore rules, but excludes XML fixtures and private/runtime files.
# XML fixtures require per-file local privacy audit and hash-based approval.
# Audit reports contain source XML values and stay OUTSIDE this ZIP.
# No automated detector can prove the absence of identifying information.

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_root="$(cd -- "$script_dir/.." && pwd)"
repo_root="$(git -C "$project_root" rev-parse --show-toplevel)"

if [[ ! -f "$project_root/Cargo.toml" ]]; then
    echo 'Missing energy_ingest/Cargo.toml' >&2
    exit 1
fi

output="$project_root/energy_ingest_review.zip"
include_fixtures=0
audit_fixtures=0
approve_fixtures=0
report_output=""

while (($#)); do
    case "$1" in
        --output)
            if (($# < 2)); then
                echo '--output requires a filename' >&2
                exit 2
            fi
            output="$2"
            shift 2
            ;;
        --report-output)
            if (($# < 2)) || [[ -z "$2" ]]; then
                echo '--report-output requires a filename' >&2
                exit 2
            fi
            report_output="$2"
            shift 2
            ;;
        --include-fixtures)
            include_fixtures=1
            shift
            ;;
        --audit-fixtures)
            audit_fixtures=1
            shift
            ;;
        --approve-fixtures)
            approve_fixtures=1
            shift
            ;;
        --help|-h)
            sed -n '4,17p' "$0"
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            exit 2
            ;;
    esac
done

if [[ -n "$report_output" ]] &&    (( ! audit_fixtures && ! approve_fixtures )); then
    echo '--report-output requires --audit-fixtures or --approve-fixtures' >&2
    exit 2
fi

privacy_tool="$script_dir/review_xml_privacy.py"
if ((audit_fixtures || approve_fixtures || include_fixtures)); then
    if [[ ! -f "$privacy_tool" ]]; then
        echo "Missing XML privacy scanner: $privacy_tool" >&2
        exit 2
    fi
fi
if ((audit_fixtures && approve_fixtures)); then
    echo 'Choose --audit-fixtures or --approve-fixtures, not both.' >&2
    exit 2
fi
if ((audit_fixtures)); then
    privacy_args=(--audit)
    if [[ -n "$report_output" ]]; then
        privacy_args+=(--report-output "$report_output")
    fi
    python3 "$privacy_tool" "${privacy_args[@]}"
    if (( ! include_fixtures )); then
        exit 0
    fi
fi
if ((approve_fixtures)); then
    privacy_args=(--approve)
    if [[ -n "$report_output" ]]; then
        privacy_args+=(--report-output "$report_output")
    fi
    python3 "$privacy_tool" "${privacy_args[@]}"
    if (( ! include_fixtures )); then
        exit 0
    fi
fi
if ((include_fixtures)); then
    echo 'Fixture inclusion disabled pending ADR-0028 review gate.' >&2
    echo 'Generate a source-only review ZIP instead.' >&2
    exit 2
fi

python3 - "$project_root" "$repo_root" "$output" \
    "$include_fixtures" <<'PY'
# SPDX-License-Identifier: MIT
import hashlib
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import zipfile

project = Path(sys.argv[1]).resolve()
repo = Path(sys.argv[2]).resolve()
output = Path(sys.argv[3]).expanduser().absolute()
include_fixtures = sys.argv[4] == "1"
prefix = project.relative_to(repo).as_posix()

# Only tracked regular files are eligible. Retain .gitignore files even
# inside secret directories, but never copy those directories' contents.
excluded_dirs = {
    ".git", "target", "dist", "test-output", "coverage",
    "__pycache__", ".venv", ".tox", ".pytest_cache",
}
secret_dirs = {"dev-secrets", "prod-secrets", "secrets"}
private_suffixes = {
    ".age", ".key", ".pem", ".p12", ".pfx", ".p8",
    ".sqlite", ".sqlite3", ".db", ".zip", ".csv",
}
private_names = {
    ".env", ".env.dev", ".env.prod", "id_rsa", "id_ed25519",
    ".npmrc", ".pypirc", "credentials", "credentials.json",
}

tracked_raw = subprocess.check_output([
    "git", "-C", str(repo), "ls-files", "-z", "--", prefix,
])
tracked = sorted(os.fsdecode(s) for s in tracked_raw.split(b"\0") if s)

included = []
excluded = []
for entry in tracked:
    path = repo / entry
    if not path.is_relative_to(project):
        continue
    rel = path.relative_to(project)
    name = rel.as_posix()
    parts = rel.parts
    lower = path.name.lower()
    in_secrets = any(p in secret_dirs for p in parts[:-1])
    is_fixture = (
        path.suffix.lower() == ".xml"
        and "fixtures" in parts
        and len(parts) >= 3
        and parts[0] == "profiles"
    )

    reason = None
    if path.is_symlink() or not path.is_file():
        reason = "not a regular file"
    elif path.resolve() == output.resolve():
        reason = "output archive"
    elif any(p in excluded_dirs for p in parts[:-1]):
        reason = "generated directory"
    elif in_secrets and lower != ".gitignore":
        reason = "secret directory"
    elif lower in private_names:
        reason = "private filename"
    elif lower.startswith(".env") and not lower.endswith(".example"):
        reason = "runtime environment file"
    elif path.suffix.lower() in private_suffixes:
        reason = "private or generated extension"
    elif path.suffix.lower() == ".xml" and not (
        is_fixture and include_fixtures
    ):
        reason = "XML needs explicit privacy review"

    if reason:
        excluded.append((name, reason))
    else:
        included.append((path, name))

if not included:
    raise SystemExit("No eligible Git-tracked files found")

head = subprocess.check_output([
    "git", "-C", str(repo), "rev-parse", "HEAD",
], text=True).strip()
changes = subprocess.check_output([
    "git", "-C", str(repo), "status", "--porcelain", "--",
    prefix,
], text=True)

manifest = [
    "# energy_ingest review archive manifest",
    f"git_revision: {head}",
    f"tracked_worktree_changes: {str(bool(changes)).lower()}",
    f"xml_fixtures_included: {str(include_fixtures).lower()}",
    "# SHA-256 digest  path (digest is of archived file bytes)",
]
for path, name in included:
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    manifest.append(f"{digest}  {name}")

output.parent.mkdir(parents=True, exist_ok=True)
fd, temp = tempfile.mkstemp(
    prefix=".review-", suffix=".zip", dir=output.parent,
)
os.close(fd)
try:
    with zipfile.ZipFile(temp, "w", zipfile.ZIP_DEFLATED) as archive:
        for path, name in included:
            archive.write(path, arcname=name)
        archive.writestr("REVIEW_MANIFEST.txt", "\n".join(manifest) + "\n")
    with zipfile.ZipFile(temp) as archive:
        invalid = archive.testzip()
        if invalid:
            raise RuntimeError(f"ZIP integrity check failed: {invalid}")
    os.replace(temp, output)
finally:
    if os.path.exists(temp):
        os.unlink(temp)

print(f"Archive: {output}")
print(f"Git revision: {head[:12]}")
print(f"Tracked worktree changes: {'yes' if changes else 'no'}")
print(f"XML fixtures included: {'yes' if include_fixtures else 'no'}")
print(f"Included tracked files: {len(included)}")
print(f"Excluded tracked files: {len(excluded)}")
print(f"Archive size: {output.stat().st_size:,} bytes")
print("ZIP integrity: OK")
print("\nIncluded files:")
for _, name in included:
    print(f"  {name}")
print("\nExcluded files (reason):")
for name, reason in excluded:
    print(f"  {name}: {reason}")

untracked_raw = subprocess.check_output([
    "git", "-C", str(repo), "ls-files", "--others",
    "--exclude-standard", "-z", "--", prefix,
])
untracked = [os.fsdecode(s) for s in untracked_raw.split(b"\0") if s]
if untracked:
    print("\nWARNING: Untracked files were not included:")
    for entry in untracked:
        print(f"  {entry}")

print("\nWARNING: Filename filtering is not a secrets scan.")
print("Inspect the archive and XML fixtures before uploading it.")
PY
