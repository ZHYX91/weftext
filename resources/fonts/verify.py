"""Verify the shipped, unmodified offline fonts without system font discovery.

This is an asset/cmap/advance check, not a renderer or platform acceptance test.
Only Python's standard library is required. Run from any directory.
"""

from __future__ import annotations

import hashlib
import json
import re
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parent
SAMPLE = "简体中文繁體漢字總價單價數量，。！？“”‘’（）【】《》—…ABCxyz0123"


def u16(data: bytes, offset: int) -> int:
    return struct.unpack_from(">H", data, offset)[0]


def u32(data: bytes, offset: int) -> int:
    return struct.unpack_from(">I", data, offset)[0]


class Font:
    def __init__(self, path: Path):
        self.data = path.read_bytes()
        if self.data[:4] not in (b"OTTO", b"\x00\x01\x00\x00"):
            raise ValueError(f"{path.name}: expected static OpenType font")
        self.tables = {}
        for index in range(u16(self.data, 4)):
            offset = 12 + 16 * index
            name = self.data[offset : offset + 4].decode("ascii")
            start, length = struct.unpack_from(">II", self.data, offset + 8)
            if start + length > len(self.data):
                raise ValueError(f"{path.name}: invalid table bounds")
            self.tables[name] = self.data[start : start + length]
        self.cmap = self._cmap()

    def _cmap(self) -> dict[int, int]:
        table = self.tables["cmap"]
        result = {}
        for index in range(u16(table, 2)):
            platform, encoding, offset = struct.unpack_from(">HHI", table, 4 + index * 8)
            if platform != 0 and not (platform == 3 and encoding in (1, 10)):
                continue
            sub = table[offset:]
            fmt = u16(sub, 0)
            if fmt == 12:
                for group in range(u32(sub, 12)):
                    first, last, glyph = struct.unpack_from(">III", sub, 16 + 12 * group)
                    for codepoint in range(first, last + 1):
                        result[codepoint] = glyph + codepoint - first
            elif fmt == 4:
                count = u16(sub, 6) // 2
                ends, starts, deltas, ranges = 14, 16 + 2 * count, 16 + 4 * count, 16 + 6 * count
                for segment in range(count):
                    first = u16(sub, starts + 2 * segment)
                    last = u16(sub, ends + 2 * segment)
                    delta = u16(sub, deltas + 2 * segment)
                    distance = u16(sub, ranges + 2 * segment)
                    for codepoint in range(first, last + 1):
                        if codepoint == 0xFFFF:
                            continue
                        glyph = (codepoint + delta) & 0xFFFF
                        if distance:
                            glyph = u16(sub, ranges + 2 * segment + distance + 2 * (codepoint - first))
                            glyph = (glyph + delta) & 0xFFFF if glyph else 0
                        result[codepoint] = glyph
        return {codepoint: glyph for codepoint, glyph in result.items() if glyph}

    def name(self, name_id: int) -> str:
        table = self.tables["name"]
        base = u16(table, 4)
        for index in range(u16(table, 2)):
            platform, encoding, language, ident, length, offset = struct.unpack_from(">HHHHHH", table, 6 + index * 12)
            if ident == name_id and platform == 3 and language in (0, 0x409):
                return table[base + offset : base + offset + length].decode("utf-16-be")
        raise ValueError(f"name {name_id} unavailable")

    def advance(self, character: str) -> int:
        glyph = self.cmap[ord(character)]
        count = u16(self.tables["hhea"], 34)
        return u16(self.tables["hmtx"], 4 * min(glyph, count - 1))

    def report(self) -> dict:
        missing = sorted({character for character in SAMPLE if ord(character) not in self.cmap})
        return {
            "family": self.name(1),
            "subfamily": self.name(2),
            "copyright": self.name(0),
            "fontVersion": self.name(5),
            "mappedCodepoints": len(self.cmap),
            "unitsPerEm": u16(self.tables["head"], 18),
            "missingSampleCharacters": missing,
            "asciiAdvances": {character: self.advance(character) for character in " AiW0"},
            "cjkAdvances": {character: self.advance(character) for character in "中漢，。"},
        }


def verify(builtin_dir: Path | None = None) -> None:
    manifest = json.loads((ROOT / "manifest.json").read_text(encoding="utf-8"))
    builtins = json.loads((ROOT / "builtin-manifest.json").read_text(encoding="utf-8"))
    web = json.loads((ROOT / "web-manifest.json").read_text(encoding="utf-8"))
    failures = []
    for asset in manifest["fonts"]:
        path = ROOT / asset["file"]
        font = Font(path)
        report = font.report()
        if hashlib.sha256(font.data).hexdigest() != asset["sha256"]:
            failures.append(f"{path.name}: SHA-256 differs from pinned asset")
        git_blob = hashlib.sha1(f"blob {len(font.data)}\0".encode() + font.data).hexdigest()
        if git_blob != asset["sourceBlobSha1"]:
            failures.append(f"{path.name}: differs from official pinned Git blob")
        if len(font.data) != asset["bytes"]:
            failures.append(f"{path.name}: length differs")
        if report != asset["inspection"]:
            failures.append(f"{path.name}: SFNT inspection differs")
        if report["missingSampleCharacters"]:
            failures.append(f"{path.name}: sample has missing cmap glyphs")
        if asset["role"] == "code":
            ascii_widths = set(report["asciiAdvances"].values())
            cjk_widths = set(report["cjkAdvances"].values())
            if len(ascii_widths) != 1 or len(cjk_widths) != 1 or next(iter(cjk_widths)) != 2 * next(iter(ascii_widths)):
                failures.append(f"{path.name}: expected half-width ASCII and full-width CJK advances")
        print(f"{path.name}: {report['fontVersion']}, {report['mappedCodepoints']} mapped codepoints, sample glyphs present")
    for license_entry in manifest["licenses"]:
        path = ROOT / license_entry["file"]
        data = path.read_bytes()
        if hashlib.sha256(data).hexdigest() != license_entry["sha256"]:
            failures.append(f"{path.name}: pinned license differs")
        if hashlib.sha1(f"blob {len(data)}\0".encode() + data).hexdigest() != license_entry["sourceBlobSha1"]:
            failures.append(f"{path.name}: differs from official pinned license Git blob")
    native_faces = {entry["file"]: entry for entry in manifest["fonts"] + builtins["fonts"]}
    browser_overrides = {}
    for entry in web["fonts"]:
        native = native_faces.get(entry["nativeFile"])
        if not native or any(entry[key] != native[key] for key in ("family", "role", "weight", "style", "license")):
            failures.append(f"{entry['file']}: browser derivative has no matching actual native owner")
            continue
        if entry["nativeFile"] in browser_overrides:
            failures.append(f"{entry['file']}: duplicate browser owner")
        browser_overrides[entry["nativeFile"]] = entry
        if entry["url"] != f"/fonts/{entry['file']}" or not re.fullmatch(r"[A-Za-z0-9_-]+\.woff2", entry["file"]):
            failures.append(f"{entry['file']}: invalid local browser delivery path")
        data = (ROOT / entry["file"]).read_bytes()
        if len(data) != entry["bytes"] or hashlib.sha256(data).hexdigest() != entry["sha256"]:
            failures.append(f"{entry['file']}: derived byte identity differs")
        if data[:8] != b"wOF2OTTO" or u32(data, 8) != len(data) or u16(data, 14) != 0:
            failures.append(f"{entry['file']}: invalid WOFF2 OpenType header")
        source = entry["source"]
        original = ROOT / "web-sources" / entry["nativeFile"]
        original_font = Font(original)
        if source["file"] != str(original.relative_to(ROOT)).replace("\\", "/") or source["sha256"] != native["sha256"] or source["bytes"] != native["bytes"] or hashlib.sha256(original_font.data).hexdigest() != native["sha256"]:
            failures.append(f"{entry['file']}: original source identity differs")
        proof = entry["equivalence"]
        if proof["glyphs"] != u16(original_font.tables["maxp"], 4) or proof["unitsPerEm"] != u16(original_font.tables["head"], 18) or any(not re.fullmatch(r"[0-9a-f]{64}", proof[key]) for key in ("glyphOrderSha256", "outlinesSha256", "metricsSha256", "cmapSha256")):
            failures.append(f"{entry['file']}: complete geometry proof is malformed")
        if not all((ROOT / name).is_file() for name in entry["licenseFiles"]):
            failures.append(f"{entry['file']}: derivative license notice missing")
        print(f"{entry['file']}: pinned browser derivative; {proof['glyphs']} glyphs; original input and recorded geometry proof verified")
    if set(browser_overrides) != {"NewCM10-Regular.otf", "NewCM10-Italic.otf"}:
        failures.append("browser derivative scope differs from the two proved CFF compatibility repairs")
    expected_faces = {browser_overrides.get(entry["file"], entry)["file"]: browser_overrides.get(entry["file"], entry) for entry in native_faces.values()}
    seen_faces = set()
    for face in re.findall(r"@font-face\s*\{([^}]+)\}", (ROOT / "fonts.css").read_text(encoding="utf-8")):
        family = re.search(r'font-family:\s*"([^"]+)"', face)
        filename = re.search(r'src:\s*url\("/fonts/([^"]+)"\)', face)
        weight = re.search(r"font-weight:\s*(\d+)", face)
        style = re.search(r"font-style:\s*(\w+)", face)
        if not all((family, filename, weight, style)):
            failures.append("invalid same-origin font face declaration")
            continue
        entry = expected_faces.get(filename[1])
        if not entry or family[1] != entry["family"] or int(weight[1]) != entry["weight"] or style[1] != entry.get("style", "normal"):
            failures.append(f"{filename[1]}: font face does not match actual native/browser delivery manifest")
        if filename[1] in seen_faces:
            failures.append(f"{filename[1]}: duplicate font face")
        seen_faces.add(filename[1])
    if seen_faces != set(expected_faces):
        failures.append("font faces and actual fixed font set differ")
    if builtin_dir is not None:
        for entry in builtins["fonts"]:
            path = builtin_dir / entry["file"]
            font = Font(path)
            data = font.data
            if len(data) != entry["bytes"] or hashlib.sha256(data).hexdigest() != entry["sha256"] or hashlib.sha1(f"blob {len(data)}\0".encode() + data).hexdigest() != entry["sourceBlobSha1"]:
                failures.append(f"{path.name}: embedded bytes differ from official pinned manifest")
            if font.name(5) != entry["version"] or ("MATH" in font.tables) != entry["inspection"]["hasMathTable"]:
                failures.append(f"{path.name}: native version/math table differs")
        print(f"{len(builtins['fonts'])} compiler-embedded font byte hashes/versions/math tables verified.")
    if failures:
        raise SystemExit("\n".join(failures))
    print("Asset integrity, CJK sample cmap, code advances and all 27 native/27 browser faces (two explicit derivatives) verified. Full geometry regeneration and platform rendering remain separate checks.")


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--builtin-dir", type=Path, help="Optional pinned typst-assets 0.15.1 files/fonts directory; read only")
    verify(parser.parse_args().builtin_dir)
