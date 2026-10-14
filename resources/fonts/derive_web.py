"""Reproduce the two browser-only Weftext Web Serif derivatives.

The complete, pinned original inputs remain in web-sources. Native Typst keeps
its original typst-assets fonts. Install requirements-web.txt only to regenerate
or fully compare these assets; ordinary integrity checks use the standard library.
"""
from __future__ import annotations

import argparse
import hashlib
from io import BytesIO
import json
from pathlib import Path

import brotli
import fontTools
from fontTools.pens.recordingPen import DecomposingRecordingPen
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parent
FONTTOOLS_VERSION = "4.61.1"
BROTLI_VERSION = "1.2.0"
FAMILY = "Weftext Web Serif"
MODIFICATION_DATE = "2026-10-13"


def digest(value: object) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def geometry(font: TTFont) -> dict:
    order, glyphs = font.getGlyphOrder(), font.getGlyphSet()
    outlines = []
    for name in order:
        pen = DecomposingRecordingPen(glyphs)
        glyphs[name].draw(pen)
        outlines.append((name, pen.value))
    cmap = [(table.platformID, table.platEncID, table.format, table.language, table.cmap) for table in font["cmap"].tables]
    return {
        "glyphs": len(order), "glyphOrderSha256": digest(order),
        "outlinesSha256": digest(outlines),
        "metricsSha256": digest([(name, font["hmtx"].metrics[name]) for name in order]),
        "cmapSha256": digest(cmap), "unitsPerEm": font["head"].unitsPerEm,
    }


def derive(entry: dict, style: str) -> tuple[dict, bytes]:
    source = ROOT / "web-sources" / entry["file"]
    original = source.read_bytes()
    if len(original) != entry["bytes"] or hashlib.sha256(original).hexdigest() != entry["sha256"]:
        raise ValueError(f"{source.name}: original input differs from pinned typst-assets")
    font = TTFont(BytesIO(original), recalcTimestamp=False)
    before = geometry(font)
    unchanged = {tag: font.getTableData(tag) for tag in font.keys() if tag not in ("GlyphOrder", "CFF ", "name", "head")}
    font["CFF "].cff.desubroutinize()
    # Chromium OTS rejects the original hint programs. Only these CFF hint
    # instructions are removed; all outlines, glyphs and metrics are compared.
    font["CFF "].cff.remove_hints()
    postscript = f"WeftextWebSerif-{style}"
    names = {
        1: FAMILY, 2: style, 3: f"Weftext:WebSerif:1.0:{style}",
        4: f"{FAMILY} {style}", 5: font["name"].getDebugName(5) + "; Weftext Web 1.0",
        6: postscript, 16: FAMILY, 17: style,
    }
    for record in font["name"].names:
        if record.nameID in names:
            record.string = names[record.nameID].encode(record.getEncoding())
    font["name"].setName(
        f"Modified version, {MODIFICATION_DATE}: Weftext browser derivative; CFF desubroutinized and unhinted; WOFF2 encoded. See NOTICE-WeftextWebFonts.txt for changes and original notices.",
        10, 3, 1, 0x409,
    )
    cff = font["CFF "].cff
    cff.fontNames = [postscript]
    cff.topDictIndex[0].FamilyName = FAMILY
    cff.topDictIndex[0].FullName = f"{FAMILY} {style}"
    font.flavor = "woff2"
    buffer = BytesIO()
    font.save(buffer, reorderTables=True)
    data = buffer.getvalue()
    reopened = TTFont(BytesIO(data), recalcTimestamp=False)
    after = geometry(reopened)
    if before != after or any(reopened.getTableData(tag) != value for tag, value in unchanged.items()):
        raise ValueError(f"{style}: glyph geometry, cmap, metrics or preserved native tables changed")
    file = f"{postscript}.woff2"
    result = {
        "nativeFile": entry["file"], "file": file, "url": f"/fonts/{file}",
        "family": entry["family"], "internalFamily": FAMILY,
        "role": entry["role"], "weight": entry["weight"], "style": entry["style"],
        "version": reopened["name"].getDebugName(5),
        "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(),
        "license": entry["license"],
        "licenseFiles": ["NOTICE-TypstAssets.txt", "NOTICE-WeftextWebFonts.txt"] + (["LICENSE-LPPL-1.3c.txt"] if style == "Italic" else []),
        "source": {
            "file": f"web-sources/{entry['file']}", "bytes": entry["bytes"], "sha256": entry["sha256"],
            "package": entry["sourcePackage"], "commit": entry["sourceCommit"], "url": entry["sourceUrl"],
            "completeUpstreamWork": "https://ctan.org/pkg/newcomputermodern",
        },
        "transformations": ["CFF desubroutinization", "CFF hint instruction removal", "font/file renaming and modified-version notice", "WOFF2 encoding"],
        "equivalence": after, "unchangedSfntTables": list(unchanged),
    }
    return result, data


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Regenerate in memory and compare without writes")
    args = parser.parse_args()
    if fontTools.__version__ != FONTTOOLS_VERSION or brotli.__version__ != BROTLI_VERSION:
        raise ValueError("Use the exact generation tools in requirements-web.txt")
    builtins = json.loads((ROOT / "builtin-manifest.json").read_text(encoding="utf-8"))
    manifest = {
        "schemaVersion": 1, "fontSetVersion": "weftext-web-newcm-1.0",
        "generator": {"file": "derive_web.py", "fontTools": FONTTOOLS_VERSION, "brotli": BROTLI_VERSION},
        "fonts": [],
    }
    for style in ("Regular", "Italic"):
        entry = next(item for item in builtins["fonts"] if item["file"] == f"NewCM10-{style}.otf")
        metadata, data = derive(entry, style)
        manifest["fonts"].append(metadata)
        target = ROOT / metadata["file"]
        if args.check:
            if target.read_bytes() != data:
                raise ValueError(f"{target.name}: generated bytes differ")
        else:
            target.write_bytes(data)
        print(f"{target.name}: {metadata['equivalence']['glyphs']} complete glyphs; cmap/outlines/metrics and 12 other tables preserved")
    target = ROOT / "web-manifest.json"
    if args.check:
        if json.loads(target.read_text(encoding="utf-8")) != manifest:
            raise ValueError("web-manifest.json: derivation facts differ")
    else:
        target.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
