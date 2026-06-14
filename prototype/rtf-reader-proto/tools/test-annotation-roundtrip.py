#!/usr/bin/env python3
"""
Minimal annotation roundtrip test.

Generates a small RTF with a Word-compatible \annotation control word,
converts it to ODT with LibreOffice, and checks that the annotation
(author + text) survives the roundtrip.

Usage:
    python3 tools/test-annotation-roundtrip.py
"""
import os
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "test-data"
OUT_DIR.mkdir(exist_ok=True)

RTF_PATH = OUT_DIR / "annotation-roundtrip.rtf"
ODT_PATH = OUT_DIR / "annotation-roundtrip.odt"

EXPECTED_AUTHOR = "Reviewer"
EXPECTED_TEXT = "This is a roundtrip annotation comment."

RTF_TEMPLATE = r"""{{\rtf1\ansi\ansicpg1252\deff0\deflang1033
{{\fonttbl{{\f0\fnil\fcharset0 Arial;}}}}
{{\info{{\author RTF Reader Proto}}}}
\pard\plain\f0\fs24 
This is sample text with an {{\*\atrfstart 0}}annotated{{\*\atrfend 0}}{{\*\atnid 1}}{{\*\atnauthor {author}}}\chatn {{\*\annotation {{\*\atnref 0}}
\pard\plain\f0\fs20 
{text}
}} word in the middle.
\par
}}
"""


def generate_rtf() -> None:
    rtf = RTF_TEMPLATE.format(author=EXPECTED_AUTHOR, text=EXPECTED_TEXT)
    RTF_PATH.write_text(rtf, encoding="ascii")
    print(f"Generated {RTF_PATH}")


def convert_to_odt() -> None:
    soffice = shutil.which("soffice") or shutil.which("libreoffice")
    if not soffice:
        if sys.platform == "darwin":
            soffice = "/Applications/LibreOffice.app/Contents/MacOS/soffice"
        else:
            raise RuntimeError("LibreOffice not found in PATH")

    cmd = [
        soffice,
        "--headless",
        "--convert-to",
        "odt",
        "--outdir",
        str(OUT_DIR),
        str(RTF_PATH),
    ]
    subprocess.run(cmd, check=True, capture_output=True, text=True)
    print(f"Converted to {ODT_PATH}")


def extract_annotations() -> list[dict]:
    ns = {
        "office": "urn:oasis:names:tc:opendocument:xmlns:office:1.0",
        "text": "urn:oasis:names:tc:opendocument:xmlns:text:1.0",
        "dc": "http://purl.org/dc/elements/1.1/",
    }
    annotations = []
    with zipfile.ZipFile(ODT_PATH) as zf:
        with zf.open("content.xml") as f:
            tree = ET.parse(f)

    for elem in tree.iter("{urn:oasis:names:tc:opendocument:xmlns:office:1.0}annotation"):
        author = elem.findtext("dc:creator", namespaces=ns) or ""
        # Only collect text from text:p paragraphs inside the annotation.
        body_parts = []
        for p in elem.iter("{urn:oasis:names:tc:opendocument:xmlns:text:1.0}p"):
            body_parts.append("".join(p.itertext()))
        body = "\n".join(body_parts).strip()
        annotations.append({"author": author, "text": body})

    return annotations


def main() -> int:
    generate_rtf()
    convert_to_odt()
    annotations = extract_annotations()

    print(f"Found {len(annotations)} annotation(s) in ODT")
    for a in annotations:
        print(f"  author={a['author']!r} text={a['text']!r}")

    if not annotations:
        print("FAIL: no annotations found")
        return 1

    first = annotations[0]
    if first["author"] != EXPECTED_AUTHOR or EXPECTED_TEXT not in first["text"]:
        print("FAIL: annotation content mismatch")
        return 1

    print("PASS: annotation roundtrip works")
    return 0


if __name__ == "__main__":
    sys.exit(main())
