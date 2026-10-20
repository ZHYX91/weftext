"""Real offline PDFium/Tesseract cases, using self-created TEST material only.

Run with the private recognition runtime's Python; --evidence must name an owned
directory. The script keeps originals/results, never mutates source documents.
"""
from __future__ import annotations

import argparse
import base64
import ctypes
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

import pypdfium2 as pdfium
from PIL import Image, ImageDraw, ImageFont


def sha(data):
    return hashlib.sha256(data).hexdigest()


def compact(text):
    return "".join(text.split())


def distance(a, b):
    row = list(range(len(b) + 1))
    for i, left in enumerate(a, 1):
        after = [i]
        for j, right in enumerate(b, 1):
            after.append(min(after[-1] + 1, row[j] + 1, row[j - 1] + (left != right)))
        row = after
    return row[-1]


def sample_image(text, font_path):
    image = Image.new("RGB", (2000, 500), "white")
    draw = ImageDraw.Draw(image)
    font = ImageFont.truetype(str(font_path), 62)
    draw.text((70, 80), text, font=font, fill="black")
    draw.text((70, 240), "TEST 2026 12345", font=font, fill="black")
    return image


def add_text(document, page, text, x, y, size=20):
    font = pdfium.raw.FPDFText_LoadStandardFont(document, b"Helvetica")
    obj = pdfium.raw.FPDFPageObj_CreateTextObj(document, font, size)
    data = ctypes.create_string_buffer(text.encode("utf-16-le") + b"\0\0")
    assert pdfium.raw.FPDFText_SetText(obj, ctypes.cast(data, ctypes.POINTER(ctypes.c_ushort)))
    wrapper = pdfium.PdfObject(obj, pdf=document)
    wrapper.transform(pdfium.PdfMatrix(e=x, f=y))
    page.insert_obj(wrapper)
    return font


def make_pdf(path, *, image=None, overlap=False, rotation=0, crop=False, blank=False):
    document = pdfium.PdfDocument.new()
    page = document.new_page(600, 400)
    fonts = []
    if not blank:
        fonts.append(add_text(document, page, "TEST TEXT LAYER 2026", 40, 340))
        fonts.append(add_text(document, page, "SECOND SOURCE LINE", 40, 300))
    if image:
        obj = pdfium.PdfImage.new(document)
        bitmap = pdfium.PdfBitmap.from_pil(image)
        obj.set_bitmap(bitmap)
        obj.set_matrix(pdfium.PdfMatrix(a=500, d=125, e=40, f=270 if overlap else 100))
        page.insert_obj(obj)
    if crop:
        page.set_cropbox(20, 30, 580, 380)
    if rotation:
        page.set_rotation(rotation)
    page.gen_content()
    document.save(path)
    for font in fonts:
        pdfium.raw.FPDFFont_Close(font)
    document.close()


def invoke(runtime, path, model_languages, pages=None, **overrides):
    source = path.read_bytes()
    media = "application/pdf" if path.suffix == ".pdf" else "image/png" if path.suffix == ".png" else "image/jpeg"
    request = {"version": 1, "inputPath": str(path.resolve()), "inputSha256": sha(source), "mediaType": media, "languages": model_languages, "pages": pages or [{"page": 1}], "dpi": 300, **overrides}
    if request.get("operation") == "inspect":
        request.pop("pages")
    start = time.monotonic()
    completed = subprocess.run([sys.executable, "-I", str(Path(__file__).with_name("worker.py")), "--runtime", str(runtime)], input=json.dumps(request).encode(), stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=75)
    assert completed.returncode == 0, completed.stderr[:200]
    result = json.loads(completed.stdout)
    assert source == path.read_bytes(), "original modified"
    if "input" in result:
        assert result["input"] == {"sha256": sha(source), "bytes": len(source), "mediaType": media}, "original provenance mismatch"
    assert str(runtime) not in completed.stdout.decode(), "operator path disclosed"
    return result, round(time.monotonic() - start, 3)


def all_text(result):
    return "\n".join(region.get("rawText", "") for page in result["pages"] for region in page["regions"])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runtime", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--font", type=Path, default=Path("C:/Windows/Fonts/msyh.ttc"))
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    cases = []
    def record(name, result, elapsed, **extra):
        (args.evidence / f"{name}.result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
        cases.append({"name": name, "status": result["status"], "seconds": elapsed, **extra})
    # Frozen before execution: these clear printed non-private samples must meet
    # CER <= 5%, selected coverage 100%, duplicate output 0, original SHA exact.
    quality = {"characterErrorRateMaximum": .05, "selectedCoverageRequired": 1., "duplicateOutputMaximum": 0., "originalHashRequired": "exact"}
    (args.evidence / "frozen-quality.json").write_text(json.dumps(quality, indent=2), encoding="utf-8")
    combinations = [("eng", ["eng"], "OFFLINE DOCUMENT SAMPLE"), ("hans", ["chi_sim"], "简体中文测试 文件资料保存"),
                    ("hant", ["chi_tra"], "繁體中文測試 文件資料保存"), ("hans-eng", ["chi_sim", "eng"], "简体中文测试 Weftext SAMPLE"),
                    ("hant-eng", ["chi_tra", "eng"], "繁體中文測試 Weftext SAMPLE")]
    for name, languages, expected in combinations:
        image = sample_image(expected, args.font)
        path = args.evidence / (f"{name}.jpg" if name == "hans-eng" else f"{name}.png")
        image.save(path, quality=95)
        image.close()
        result, elapsed = invoke(args.runtime, path, languages)
        actual, expected_full = compact(all_text(result)), compact(expected + "TEST 2026 12345")
        cer = distance(expected_full, actual) / len(expected_full)
        record(name, result, elapsed, cer=cer, inputSha256=sha(path.read_bytes()))
        assert result["status"] in {"complete", "partial"} and cer <= quality["characterErrorRateMaximum"], (name, actual, cer)
    scan_image = sample_image("SCANNED PAGE SAMPLE", args.font)
    scan_pdf = args.evidence / "scan.pdf"
    scan_image.save(scan_pdf, resolution=240)
    result, elapsed = invoke(args.runtime, scan_pdf, ["eng"])
    record("scan", result, elapsed)
    assert "SCANNED PAGE SAMPLE" in all_text(result), result
    for name, options in [("text", {}), ("mixed-disjoint", {"image": scan_image}), ("mixed-overlay", {"image": scan_image, "overlap": True}),
                          ("rotated-cropped", {"rotation": 90, "crop": True}), ("blank", {"blank": True})]:
        path = args.evidence / f"{name}.pdf"
        make_pdf(path, **options)
        result, elapsed = invoke(args.runtime, path, ["eng"])
        record(name, result, elapsed)
        if name == "blank":
            assert result["pages"][0]["status"] == "blank", result
        else:
            assert all_text(result).count("TEST TEXT LAYER 2026") == 1, result
        if name == "mixed-disjoint":
            assert "SCANNED PAGE SAMPLE" in all_text(result), result
        if name == "mixed-overlay":
            assert result["status"] == "partial" and "SCANNED PAGE SAMPLE" not in all_text(result), result
        if name == "rotated-cropped":
            page = result["pages"][0]
            assert page["rotation"] == 90 and page["cropBox"] == [20, 30, 580, 380], result
            for region in page["regions"]:
                for block in region["blocks"]:
                    assert 0 <= block["bbox"][0] <= block["bbox"][2] <= page["width"]
                    assert 0 <= block["bbox"][1] <= block["bbox"][3] <= page["height"]
            block = next(block for region in page["regions"] for block in region["blocks"] if block["text"].startswith("TEST TEXT"))
            a, b, c, d, e, f = page["renderToPdf"]
            points = [(a * x * page["renderWidth"] / page["width"] + c * y * page["renderHeight"] / page["height"] + e,
                       b * x * page["renderWidth"] / page["width"] + d * y * page["renderHeight"] / page["height"] + f)
                      for x in (block["bbox"][0], block["bbox"][2]) for y in (block["bbox"][1], block["bbox"][3])]
            assert abs(min(p[0] for p in points) - 40) < 2 and abs(min(p[1] for p in points) - 340) < 2, points
    scan_image.close()
    base = args.evidence / "eng.png"
    for name, kwargs, expected_error in [
        ("stale-hash", {"inputSha256": "0" * 64}, "input-changed"),
        ("wrong-type", {"mediaType": "image/jpeg"}, "media-type-mismatch"),
        ("unsupported-language", {"languages": ["fra"]}, "unsupported-languages"),
        ("dpi-budget", {"dpi": 301}, "dpi-budget"),
        ("overlap-selection", {"pages": [{"page": 1, "regions": [{"id": "one", "rect": [0, 0, 100, 100]}, {"id": "two", "rect": [50, 50, 200, 200]}]}]}, "overlapping-selection"),
    ]:
        result, elapsed = invoke(args.runtime, base, ["eng"], **kwargs)
        record(name, result, elapsed)
        assert result.get("error") == expected_error, result
    missing, elapsed = invoke(args.runtime / "missing-runtime", base, ["eng"])
    record("missing-runtime", missing, elapsed)
    assert missing.get("error") == "runtime-unavailable", missing
    result, elapsed = invoke(args.runtime, base, ["eng"], pages=[{"page": 1}, {"page": 2}])
    record("partial-page-selection", result, elapsed)
    assert result["status"] == "partial" and result["pageCount"] == 1 and result["pages"][1]["status"] == "failed", result
    result, elapsed = invoke(args.runtime, base, ["eng"], pages=[{"page": 1, "regions": [{"id": "outside", "rect": [1900, 0, 2100, 100]}]}])
    record("outside-region", result, elapsed)
    assert result["status"] == "failed" and result["pages"][0]["losses"] == ["region-outside-visible-page"], result
    corrupted = args.evidence / "corrupt.pdf"
    corrupted.write_bytes(b"%PDF-1.7\nTEST invalid data\n")
    result, elapsed = invoke(args.runtime, corrupted, ["eng"])
    record("corrupt-pdf", result, elapsed)
    assert result["status"] == "failed" and "corrupt-pdf" in result["pages"][0]["losses"], result
    large = args.evidence / "pixel-budget.png"
    oversized = Image.new("RGB", (6400, 6400), "white")
    oversized.save(large)
    oversized.close()
    result, elapsed = invoke(args.runtime, large, ["eng"])
    record("pixel-budget", result, elapsed)
    assert result["status"] == "failed" and "image-budget-or-format" in result["pages"][0]["losses"], result
    table = Image.new("RGB", (1000, 600), "white")
    draw, font = ImageDraw.Draw(table), ImageFont.truetype(str(args.font), 48)
    for x in (20, 500, 980):
        draw.line((x, 20, x, 580), fill="black", width=3)
    for y in (20, 300, 580):
        draw.line((20, y, 980, y), fill="black", width=3)
    for x, y, text in [(90, 100, "ALPHA"), (570, 100, "BETA"), (90, 380, "GAMMA"), (570, 380, "DELTA")]:
        draw.text((x, y), text, font=font, fill="black")
    table_path = args.evidence / "table.png"
    table.save(table_path)
    table.close()
    result, elapsed = invoke(args.runtime, table_path, ["eng"])
    record("basic-ruled-table", result, elapsed)
    tables = result["pages"][0]["regions"][0].get("tables", [])
    assert len(tables) == 1 and tables[0]["rows"] == tables[0]["columns"] == 2, result
    assert [cell["text"] for cell in tables[0]["cells"]] == ["ALPHA", "BETA", "GAMMA", "DELTA"], result
    multipage = args.evidence / "noncontinuous.pdf"
    document, fonts = pdfium.PdfDocument.new(), []
    for index in range(4):
        page = document.new_page(600, 400)
        fonts.append(add_text(document, page, f"TEST SELECTED PAGE {index + 1}", 40, 340))
        page.gen_content()
    document.save(multipage)
    for font in fonts:
        pdfium.raw.FPDFFont_Close(font)
    document.close()
    result, elapsed = invoke(args.runtime, multipage, ["eng"], pages=[{"page": 4}, {"page": 2}])
    record("noncontinuous-pages", result, elapsed)
    assert result["status"] == "complete" and result["pageCount"] == 4 and [page["page"] for page in result["pages"]] == [4, 2], result
    assert "PAGE 1" not in all_text(result) and "PAGE 3" not in all_text(result), result
    result, elapsed = invoke(args.runtime, multipage, ["eng"], operation="inspect")
    record("inspect-all-pages", result, elapsed)
    assert result["status"] == "complete" and result["pageCount"] == 4 and len(result["pages"]) == 4
    assert all(page["width"] == 600 and page["height"] == 400 and page["regions"] == [] for page in result["pages"])
    result, elapsed = invoke(args.runtime, args.evidence / "rotated-cropped.pdf", ["eng"], operation="preview", dpi=144)
    record("preview-rotated-cropped", result, elapsed)
    page = result["pages"][0]
    rendered = base64.b64decode(page["preview"]["dataBase64"], validate=True)
    (args.evidence / "preview-rotated-cropped.png").write_bytes(rendered)
    assert result["status"] == "complete" and rendered.startswith(b"\x89PNG\r\n\x1a\n")
    assert page["preview"]["width"] == page["renderWidth"] == 700 and page["preview"]["height"] == page["renderHeight"] == 1120
    duplicate = args.evidence / "duplicate-text-layer.pdf"
    document = pdfium.PdfDocument.new()
    page = document.new_page(600, 400)
    fonts = [add_text(document, page, "TEST DUPLICATE LAYER", 40, 340) for _ in range(2)]
    page.gen_content()
    document.save(duplicate)
    for font in fonts:
        pdfium.raw.FPDFFont_Close(font)
    document.close()
    result, elapsed = invoke(args.runtime, duplicate, ["eng"])
    record("duplicate-text-layer", result, elapsed)
    assert all_text(result).count("TEST DUPLICATE LAYER") == 1, result
    text_table = args.evidence / "text-table.pdf"
    document = pdfium.PdfDocument.new()
    page = document.new_page(1000, 600)
    for x1, y1, x2, y2 in [(x, 20, x, 580) for x in (20, 500, 980)] + [(20, y, 980, y) for y in (20, 300, 580)]:
        obj = pdfium.raw.FPDFPageObj_CreateNewPath(x1, y1)
        assert pdfium.raw.FPDFPath_LineTo(obj, x2, y2)
        assert pdfium.raw.FPDFPath_SetDrawMode(obj, 0, True)
        assert pdfium.raw.FPDFPageObj_SetStrokeWidth(obj, 2)
        page.insert_obj(pdfium.PdfObject(obj, pdf=document))
    fonts = [add_text(document, page, value, x, y, 32) for x, y, value in [(90, 450, "ALPHA"), (570, 450, "BETA"), (90, 170, "GAMMA"), (570, 170, "DELTA")]]
    page.gen_content()
    document.save(text_table)
    for font in fonts:
        pdfium.raw.FPDFFont_Close(font)
    document.close()
    result, elapsed = invoke(args.runtime, text_table, ["eng"])
    record("basic-text-layer-table", result, elapsed)
    tables = result["pages"][0]["regions"][0]["tables"]
    assert len(tables) == 1 and [cell["text"] for cell in tables[0]["cells"]] == ["ALPHA", "BETA", "GAMMA", "DELTA"], result
    columns = args.evidence / "two-columns.pdf"
    document = pdfium.PdfDocument.new()
    page = document.new_page(600, 400)
    fonts = [add_text(document, page, value, x, y, 16) for x, y, value in [(40, 340, "TEST LEFT FIRST"), (330, 340, "TEST RIGHT FIRST"),
                                                                       (40, 300, "TEST LEFT SECOND"), (330, 300, "TEST RIGHT SECOND")]]
    page.gen_content()
    document.save(columns)
    for font in fonts:
        pdfium.raw.FPDFFont_Close(font)
    document.close()
    result, elapsed = invoke(args.runtime, columns, ["eng"], dpi=72)
    record("simple-two-column-order", result, elapsed)
    page = result["pages"][0]
    blocks = {block["id"]: block["text"] for region in page["regions"] for block in region["blocks"]}
    assert [blocks[bid] for bid in page["readingOrder"]["suggested"]] == ["TEST LEFT FIRST", "TEST LEFT SECOND", "TEST RIGHT FIRST", "TEST RIGHT SECOND"], result
    result, elapsed = invoke(args.runtime, args.evidence / "eng.png", ["eng"], pages=[{"page": 1, "regions": [{"id": "scaled", "rect": [0, 0, 2000, 500], "mode": "ocr", "ocrScale": .5}]}])
    record("explicit-ocr-scale", result, elapsed)
    geometry = result["pages"][0]["regions"][0]["ocrGeometry"]
    assert compact(all_text(result)) == "OFFLINEDOCUMENTSAMPLETEST202612345", result
    assert geometry["width"] == 1000 and geometry["height"] == 250 and geometry["ocrToVisible"] == [2, 0, 0, 2, 0, 0], geometry
    transparent = args.evidence / "transparent.png"
    with Image.new("RGBA", (1800, 400), (0, 0, 0, 0)) as image:
        ImageDraw.Draw(image).text((70, 80), "TEST TRANSPARENT PNG", font=ImageFont.truetype(str(args.font), 62), fill=(0, 0, 0, 255))
        image.save(transparent)
    result, elapsed = invoke(args.runtime, transparent, ["eng"])
    record("transparent-png-composited", result, elapsed)
    assert compact(all_text(result)) == "TESTTRANSPARENTPNG", result
    summary = {"version": 1, "kind": "recognition-runtime-integration", "scope": "Private runtime worker; no HTTP/Core/UI/Office acceptance", "quality": quality, "cases": cases, "passed": True}
    (args.evidence / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"passed": len(cases), "summary": str(args.evidence / "summary.json")}, ensure_ascii=False))


if __name__ == "__main__":
    main()
