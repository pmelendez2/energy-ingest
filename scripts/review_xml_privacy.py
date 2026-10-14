#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# AI-assisted: ChatGPT / GPT-6; 2026-10-09.
"""Local Green Button XML privacy screening and hash-bound approvals.

Heuristic findings require human interpretation. This tool never certifies
anonymity; audit reports can contain customer data and must remain private.
"""

import argparse
from collections import Counter, defaultdict
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from xml.parsers import expat

MAX_XML_BYTES = 25 * 1024 * 1024
MAX_VALUE = 240
MAX_GROUPS = 300
MAX_SAMPLES = 3
EMAIL = re.compile(r"\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b", re.I)
PHONE = re.compile(r"(?<!\d)(?:\+?1[ .-]?)?\(?\d{3}\)?[ .-]?\d{3}[ .-]?\d{4}(?!\d)")
CREDENTIAL = re.compile(
    r"(?:bearer\s+[a-z0-9._-]{12,}|"
    r"(?:password|api[_-]?key|access[_-]?token|client[_-]?secret)"
    r"\s*[:=]\s*\S+)", re.I,
)
URL_QUERY = re.compile(r"https?://[^\s\"'<>?]+\?[^\s\"'<>]+", re.I)
UUID = re.compile(
    r"\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-"
    r"[0-9a-f]{4}-[0-9a-f]{12}\b", re.I,
)
LONG_NUMBER = re.compile(r"(?<!\d)\d{9,}(?!\d)")
# Green Button / ESPI numerical data is not automatically a phone number.
TECHNICAL = frozenset({
    "start", "duration", "value", "cost", "timestamp", "updated",
    "published", "time", "offset", "interval", "readingtype",
    "multiplier", "poweroftentenmultiplier", "uom", "unit", "kind",
    "flowdirection", "accumulationbehaviour", "commodity", "status",
    "intervalblock", "intervalreading", "currency", "tou", "phase",
})
SENSITIVE = re.compile(
    r"customer|account|meter|service.?address|street|postal|zip.?code|"
    r"email|phone|first.?name|last.?name|authorization|token|secret|"
    r"usage.?point|service.?point|retail.?customer|subscription", re.I,
)
IDENTITY_FIELDS = re.compile(
    r"account|meter|customer|address|email|phone|name|identifier|"
    r"usage.?point|service.?point|subscription", re.I,
)


def git_fixtures(repo: Path, project: Path) -> list[Path]:
    prefix = project.relative_to(repo).as_posix()
    raw = subprocess.check_output([
        "git", "-C", str(repo), "ls-files", "-z", "--", prefix,
    ])
    found = []
    for item in raw.split(b"\0"):
        if not item:
            continue
        path = repo / os.fsdecode(item)
        rel = path.relative_to(project)
        if (len(rel.parts) >= 3 and rel.parts[0] == "profiles"
                and "fixtures" in rel.parts and path.suffix.lower() == ".xml"
                and path.is_file() and not path.is_symlink()):
            found.append(path)
    return sorted(found)


def scan(path: Path) -> dict:
    data = path.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    groups = {}
    technical = Counter()
    blocked = False
    xml_lines = data.decode("utf-8", errors="replace").splitlines()

    def add(level, kind, field, value, line):
        # Group repetitive values, while preserving a few source locations.
        shown = str(value).strip().replace("\n", " ")[:MAX_VALUE]
        key = (level, kind, field.lower(), shown)
        if key not in groups:
            groups[key] = {"level": level, "type": kind,
                           "field": field, "value": shown,
                           "count": 0, "lines": []}
        rec = groups[key]
        rec["count"] += 1
        if line not in rec["lines"] and len(rec["lines"]) < MAX_SAMPLES:
            rec["lines"].append(line)

    if len(data) > MAX_XML_BYTES:
        add("BLOCKED", "XML exceeds audit size limit", "XML", len(data), 1)
        blocked = True
    if re.search(rb"<!\s*(?:DOCTYPE|ENTITY)\b", data, re.I):
        add("BLOCKED", "DOCTYPE or ENTITY declaration", "XML", "present", 1)
        blocked = True
    if b"\x00" in data:
        add("BLOCKED", "NUL byte", "XML", "present", 1)
        blocked = True

    if not blocked:
        parser = expat.ParserCreate()
        parser.SetParamEntityParsing(expat.XML_PARAM_ENTITY_PARSING_NEVER)
        stack = []

        def check(value, line, field):
            trimmed = value.strip()
            if not trimmed:
                return
            short = field.lower()
            if short in TECHNICAL:
                technical[short] += 1
                return
            if CREDENTIAL.search(trimmed):
                add("CRITICAL", "possible credential", field,
                    trimmed, line)
            if EMAIL.search(trimmed):
                add("HIGH", "email address", field, trimmed, line)
            if URL_QUERY.search(trimmed):
                add("HIGH", "URL with query parameters", field,
                    trimmed, line)
            # Only recognize a phone in phone-like XML contexts or with
            # unmistakable phone formatting; exclude plain 10-digit epochs.
            phone_context = "phone" in short or "tel" in short
            phone_format = any(char in trimmed for char in "+()- ")
            if PHONE.search(trimmed) and (phone_context or phone_format):
                add("HIGH", "possible telephone number", field,
                    trimmed, line)
            if UUID.search(trimmed):
                add("MEDIUM", "UUID / technical identifier", field,
                    trimmed, line)
            if IDENTITY_FIELDS.search(short) and LONG_NUMBER.search(trimmed):
                add("MEDIUM", "possible customer-specific ID", field,
                    trimmed, line)
            matched = (CREDENTIAL.search(trimmed) or
                       EMAIL.search(trimmed) or
                       URL_QUERY.search(trimmed) or
                       UUID.search(trimmed))
            if (SENSITIVE.search(short) and len(trimmed) <= 120
                    and not matched):
                add("MEDIUM", "sensitive field value", field,
                    trimmed, line)

        def start(name, attrs):
            short = name.rsplit(":", 1)[-1]
            stack.append(short)
            line = parser.CurrentLineNumber
            if SENSITIVE.search(short):
                # Structural names are grouped, not repeated per reading.
                technical[f"sensitive_element:{short}"] += 1
            for key, value in attrs.items():
                field = key.rsplit(":", 1)[-1]
                check(value, line, field)

        def end(_name):
            if stack:
                stack.pop()

        def chars(value):
            if value.strip():
                check(value, parser.CurrentLineNumber,
                      stack[-1] if stack else "text")

        def reject_doctype(*_args):
            raise ValueError("DOCTYPE is prohibited")

        parser.StartElementHandler = start
        parser.EndElementHandler = end
        parser.CharacterDataHandler = chars
        parser.StartDoctypeDeclHandler = reject_doctype
        try:
            parser.Parse(data, True)
        except (expat.ExpatError, ValueError) as exc:
            add("BLOCKED", "invalid/prohibited XML", "XML",
                str(exc), parser.CurrentLineNumber or 1)
            blocked = True

    findings = sorted(groups.values(), key=lambda x: (
        {"BLOCKED": 0, "CRITICAL": 1, "HIGH": 2,
         "MEDIUM": 3}.get(x["level"], 4),
        x["type"], x["field"], x["value"],
    ))
    return {"sha256": digest, "size": len(data),
            "blocked": blocked, "findings": findings,
            "technical": technical, "line_count": len(xml_lines)}


def write_private_report(path: Path, repo: Path, scans, project: Path):
    # Resolve parent to prevent ../ and parent-symlink escapes into Git.
    path = path.expanduser().absolute()
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    if path.is_symlink():
        raise ValueError("Report destination must not be a symlink")
    real_path = path.parent.resolve() / path.name
    real_repo = repo.resolve()
    if real_path == real_repo or real_repo in real_path.parents:
        raise ValueError("Private report must be outside the repository")

    summary = Counter()
    report = [
        "Green Button XML fixture privacy screening",
        "PRIVATE REPORT: MAY CONTAIN CUSTOMER INFORMATION.",
        "Do not commit, upload, email, or paste this report.",
        "Heuristic findings are NOT proof of disclosure or anonymity.",
        f"Files scanned: {len(scans)}", "",
    ]
    for p, result in scans:
        summary.update(f["level"] for f in result["findings"])
        if result["blocked"]:
            summary["blocked_files"] += 1
    state = ("BLOCKED" if summary["blocked_files"] else
             "REVIEW REQUIRED" if any(r["findings"] for _, r in scans)
             else "NO HEURISTIC FINDINGS — MANUAL REVIEW REQUIRED")
    report.append(f"OVERALL RESULT: {state}")
    for level in ("BLOCKED", "CRITICAL", "HIGH", "MEDIUM"):
        report.append(f"Unique {level} finding groups: {summary[level]}")
    report.append("")

    for source, record in scans:
        relative = source.relative_to(project).as_posix()
        findings = record["findings"]
        report.extend([
            "=" * 64, f"FILE: {relative}",
            f"SHA-256: {record['sha256']}",
            f"Size: {record['size']:,} bytes; lines: {record['line_count']:,}",
            f"Blocked: {record['blocked']}",
            f"Unique finding groups: {len(findings)}",
            f"Finding occurrences: {sum(f['count'] for f in findings)}",
        ])
        for finding in findings[:MAX_GROUPS]:
            report.append(
                f"[{finding['level']}] {finding['type']} "
                f"in <{finding['field']}>; "
                f"occurrences={finding['count']}; "
                f"sample lines={finding['lines']}"
            )
            report.append(f"  VALUE: {finding['value']}")
        if len(findings) > MAX_GROUPS:
            report.append(
                f"WARNING: {len(findings) - MAX_GROUPS} additional unique "
                "groups omitted; inspect XML directly before approval."
            )
        if not findings:
            report.append("NO HEURISTIC FINDINGS DETECTED.")
            report.append("Manual privacy review remains required.")
        report.append("Expected technical/structural element counts:")
        for key, count in record["technical"].most_common(30):
            report.append(f"  {key}: {count}")
        report.append("Review meter IDs, account relationships, usage patterns,")
        report.append("timestamps and URLs for reidentification risk.")
        report.append("")

    # Avoid exposing data if a report already exists with permissive mode.
    flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC
    flags |= getattr(os, "O_NOFOLLOW", 0)
    fd = os.open(real_path, flags, 0o600)
    with os.fdopen(fd, "w", encoding="utf-8") as out:
        os.fchmod(out.fileno(), 0o600)
        out.write("\n".join(report) + "\n")
    size = real_path.stat().st_size
    if size == 0:
        raise RuntimeError("ERROR: privacy report is empty")
    print(f"PRIVATE local report (contains values): {real_path}")
    print(f"Report size: {size:,} bytes")
    print(f"Overall result: {state}")
    return real_path


def load_json(path: Path):
    if not path.exists():
        return {}
    if path.is_symlink():
        raise ValueError(f"Refusing symlinked state file: {path}")
    return json.loads(path.read_text(encoding="utf-8"))


def save_json(path: Path, data):
    if path.is_symlink():
        raise ValueError(f"Refusing symlinked state file: {path}")
    flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC
    flags |= getattr(os, "O_NOFOLLOW", 0)
    fd = os.open(path, flags, 0o600)
    with os.fdopen(fd, "w", encoding="utf-8") as out:
        os.fchmod(out.fileno(), 0o600)
        json.dump(data, out, indent=2, sort_keys=True)
        out.write("\n")


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    modes = ap.add_mutually_exclusive_group(required=True)
    modes.add_argument("--audit", action="store_true")
    modes.add_argument("--approve", action="store_true")
    modes.add_argument("--verify", action="store_true")
    ap.add_argument("--report-output", type=Path)
    args = ap.parse_args()
    if args.verify and args.report_output:
        ap.error("--report-output cannot be used with --verify")

    project = Path(__file__).resolve().parent.parent
    repo = Path(subprocess.check_output([
        "git", "-C", str(project), "rev-parse", "--show-toplevel",
    ], text=True).strip())
    fixtures = git_fixtures(repo, project)
    if not fixtures:
        sys.exit("ERROR: No tracked XML fixtures found")
    state_dir = Path.home() / ".local/state/energy_ingest"
    state_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
    os.chmod(state_dir, 0o700)
    approvals_path = state_dir / "xml_fixture_approvals.json"
    audited_path = state_dir / "xml_fixture_audits.json"
    approvals = load_json(approvals_path)
    audited = load_json(audited_path)
    scans = [(p, scan(p)) for p in fixtures]

    if args.audit or args.approve:
        report_path = args.report_output or (
            state_dir / "xml_privacy_report.txt"
        )
        write_private_report(report_path, repo, scans, project)
        audited = {
            p.relative_to(project).as_posix(): r["sha256"]
            for p, r in scans
        }
        save_json(audited_path, audited)
        for p, r in scans:
            print(f"  {p.name}: {len(r['findings'])} unique groups; "
                  f"blocked={r['blocked']}")

    if args.approve:
        if not sys.stdin.isatty():
            sys.exit("ERROR: Approval requires an interactive terminal")
        print("Review the private report and the complete XML source.")
        print("Approval attests your review; it is not certification.")
        for path, record in scans:
            rel = path.relative_to(project).as_posix()
            if record["blocked"]:
                approvals.pop(rel, None)
                print(f"BLOCKED: {rel}")
                continue
            print(f"\n{rel}: {len(record['findings'])} finding groups")
            answer = input("Manually reviewed and safe to share? [yes/no] ")
            if answer.strip() == "yes":
                approvals[rel] = {
                    "sha256": record["sha256"],
                    "approved_utc": datetime.now(timezone.utc).isoformat(),
                }
            else:
                approvals.pop(rel, None)
        save_json(approvals_path, approvals)
        print(f"Local approvals: {approvals_path}")

    if args.verify:
        failures = []
        for path, record in scans:
            rel = path.relative_to(project).as_posix()
            if (record["blocked"] or
                audited.get(rel) != record["sha256"] or
                approvals.get(rel, {}).get("sha256") != record["sha256"]):
                failures.append(rel)
        if failures:
            print("BLOCKED: stale/missing audit or file approvals:",
                  file=sys.stderr)
            for entry in failures:
                print(f"  {entry}", file=sys.stderr)
            return 1
        print(f"Approved fixture hashes verified: {len(scans)}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        sys.exit(f"ERROR: {exc}")
