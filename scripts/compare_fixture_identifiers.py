#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Private, value-free comparison of XML identity tokens.

This is a screening aid, not certification that fixtures are anonymous.
Do not upload source documents or detailed scan results.
"""

import argparse
from collections import Counter
from pathlib import Path
import re
import sys
from urllib.parse import urlsplit
from xml.etree import ElementTree as ET

UUID = re.compile(
    r"(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-"
    r"[0-9a-f]{4}-[0-9a-f]{12}\b"
)
LONG_DIGITS = re.compile(r"(?<!\d)\d{7,}(?!\d)")
SENSITIVE_FIELD = re.compile(
    r"(?i)account|meter|customer|service.?point|usage.?point|"
    r"identifier|serial|address|telephone|phone|email|name"
)
PII_FIELD = re.compile(
    r"(?i)address|telephone|phone|email|first.?name|last.?name|"
    r"full.?name|street|postal"
)
IDENTITY_FIELD = {"id", "href", "self", "up", "mrid"}


def local_name(tag):
    return tag.rsplit("}", 1)[-1].rsplit(":", 1)[-1].lower()


def extract(path):
    """Return opaque identity tokens and count of explicit PII fields."""
    if b"<!DOCTYPE" in path.read_bytes().upper():
        raise ValueError("DOCTYPE is not allowed")
    tokens = set()
    pii_fields = 0
    # ET.iterparse avoids retaining large usage-reading trees.
    context = ET.iterparse(path, events=("start", "end"))
    for event, elem in context:
        if event != "end":
            continue
        field = local_name(elem.tag)
        pairs = [(field, elem.text or "")]
        pairs.extend((local_name(k), v) for k, v in elem.attrib.items())
        for name, value in pairs:
            value = value.strip()
            if not value:
                continue
            if PII_FIELD.search(name):
                pii_fields += 1
            if name not in IDENTITY_FIELD and not SENSITIVE_FIELD.search(name):
                continue
            tokens.update(("uuid", m.lower()) for m in UUID.findall(value))
            # Compare longer numbers only when used in identity context.
            tokens.update(("numeric", m) for m in LONG_DIGITS.findall(value))
            if value.startswith(("http://", "https://")):
                # Individual URL path components can expose real IDs.
                parts = urlsplit(value).path.split("/")
                markers = {"retailcustomer", "usagepoint",
                           "serviceagreement", "meter", "account"}
                for index, part in enumerate(parts[:-1]):
                    candidate = parts[index + 1]
                    if (part.lower() in markers and
                            len(candidate) >= 6 and
                            re.fullmatch(r"[a-zA-Z0-9_-]+", candidate) and
                            not UUID.fullmatch(candidate)):
                        tokens.add(("path", candidate))
            elif len(value) >= 12 and len(value) <= 200:
                # Compare complete IDs; do not inspect meter readings.
                if not UUID.search(value):
                    tokens.add(("full", value))
        elem.clear()
    return tokens, pii_fields


def xml_files(root):
    return sorted(root.rglob("*.xml"))


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--originals", required=True, type=Path)
    ap.add_argument("--fixtures", required=True, type=Path)
    args = ap.parse_args()
    originals = args.originals.resolve()
    fixtures = args.fixtures.resolve()
    if not originals.is_dir() or not fixtures.is_dir():
        ap.error("both paths must be directories")
    if originals == fixtures or originals in fixtures.parents or \
            fixtures in originals.parents:
        ap.error("originals and fixtures must be separate directories")
    src_files = xml_files(originals)
    dst_files = xml_files(fixtures)
    if not src_files or not dst_files:
        ap.error("both directories must contain XML files")

    original_tokens = set()
    for file in src_files:
        tokens, _ = extract(file)
        original_tokens.update(tokens)

    collisions = Counter()
    total_pii_fields = 0
    for file in dst_files:
        tokens, pii = extract(file)
        total_pii_fields += pii
        for kind, _ in tokens & original_tokens:
            collisions[kind] += 1

    print(f"Original XML documents: {len(src_files)}")
    print(f"Fixture XML documents: {len(dst_files)}")
    print(f"Fixture fields with PII-related names: {total_pii_fields}")
    print("Shared identity tokens (file occurrences; no values):")
    for kind in ("uuid", "numeric", "path", "full"):
        print(f"  {kind}: {collisions[kind]}")
    print("Result: " + (
        "REVIEW REQUIRED: original identity tokens present"
        if sum(collisions.values()) else
        "NO SHARED TOKENS FOUND; manual review still required"
    ))
    # Fail closed if there is overlap or explicit PII-bearing fields.
    return 1 if sum(collisions.values()) or total_pii_fields else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ET.ParseError, ValueError) as exc:
        print(f"ERROR: comparison could not complete ({type(exc).__name__})",
              file=sys.stderr)
        sys.exit(2)
