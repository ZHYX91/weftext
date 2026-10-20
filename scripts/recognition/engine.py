"""Offline single-page implementation; invoked only by worker.py's bounded child."""
from __future__ import annotations

from collections import defaultdict
import base64
import csv
import io
import math
from pathlib import Path
import subprocess
import tempfile
import time

import pypdfium2 as pdfium
from PIL import Image, ImageDraw, ImageOps

MAX_PIXELS = 40_000_000
MAX_CHARACTERS = 500_000
MAX_WORDS = 100_000
Image.MAX_IMAGE_PIXELS = MAX_PIXELS


def bbox_union(rects):
    return [min(r[0] for r in rects), min(r[1] for r in rects), max(r[2] for r in rects), max(r[3] for r in rects)]


def overlap(a, b):
    return min(a[2], b[2]) > max(a[0], b[0]) and min(a[3], b[3]) > max(a[1], b[1])


def contained(a, b):
    return a[0] >= b[0] and a[1] >= b[1] and a[2] <= b[2] and a[3] <= b[3]


def rounded(rect):
    return [round(float(value), 5) for value in rect]


def affine(fn):
    origin, x, y = fn(0, 0), fn(1, 0), fn(0, 1)
    return [x[0] - origin[0], x[1] - origin[1], y[0] - origin[0], y[1] - origin[1], *origin]


def mapped_box(rect, transform):
    points = [transform(x, y) for x in (rect[0], rect[2]) for y in (rect[1], rect[3])]
    return rounded([min(p[0] for p in points), min(p[1] for p in points), max(p[0] for p in points), max(p[1] for p in points)])


def inverse_rotation(rotation, width, height):
    return {0: lambda x, y: (x, y), 90: lambda x, y: (y, height - x),
            180: lambda x, y: (width - x, height - y), 270: lambda x, y: (width - y, x)}[rotation]


def blank_image(image):
    small = image.convert("L")
    small.thumbnail((1000, 1000))
    histogram = small.histogram()
    # Only an almost entirely white raster is classified blank; empty OCR on a
    # diagram/photograph is partial, never successful blank recognition.
    return sum(histogram[:245]) <= max(1, small.width * small.height // 100_000)


def order_candidates(blocks, width, height):
    lines = [block for block in blocks if block["text"].strip()]
    left = [b for b in lines if b["bbox"][2] < width * .49]
    right = [b for b in lines if b["bbox"][0] > width * .51]
    wide = [b for b in lines if b not in left and b not in right]
    two = len(left) >= 2 and len(right) >= 2 and not any(b["bbox"][1] > min(c["bbox"][1] for c in left + right) and b["bbox"][3] < max(c["bbox"][3] for c in left + right) for b in wide)
    if two:
        top = min(b["bbox"][1] for b in left + right)
        before = sorted([b for b in wide if b["bbox"][1] <= top], key=lambda b: (b["bbox"][1], b["bbox"][0]))
        after = sorted([b for b in wide if b not in before], key=lambda b: (b["bbox"][1], b["bbox"][0]))
        proposed = before + sorted(left, key=lambda b: b["bbox"][1]) + sorted(right, key=lambda b: b["bbox"][1]) + after
    else:
        # Font sizes vary: normalizing each top by its own height moves large
        # headings ahead of earlier body lines. Keep physical top-to-bottom order.
        proposed = sorted(lines, key=lambda b: (b["bbox"][1], b["bbox"][0]))
    return {"kind": "simple-two-column-candidate" if two else "single-column-candidate", "requiresConfirmation": True,
            "original": [b["id"] for b in blocks], "suggested": [b["id"] for b in proposed],
            "headerFooterCandidates": [b["id"] for b in lines if b["bbox"][3] < height * .1 or b["bbox"][1] > height * .9]}


def grid_geometry(image):
    gray = image.convert("L")
    scale = min(1., 1800 / max(gray.size))
    if scale < 1:
        gray = gray.resize((max(1, round(gray.width * scale)), max(1, round(gray.height * scale))))
    data, width, height = gray.tobytes(), gray.width, gray.height
    xs = [x for x in range(width) if sum(data[y * width + x] < 90 for y in range(height)) > height * .65]
    ys = [y for y in range(height) if sum(value < 90 for value in data[y * width:(y + 1) * width]) > width * .65]
    def groups(values):
        result = []
        for value in values:
            if result and value - result[-1][-1] <= 2:
                result[-1].append(value)
            else:
                result.append([value])
        return [(group[0] / scale, group[-1] / scale) for group in result]
    xgroups, ygroups = groups(xs), groups(ys)
    xs, ys = [(left + right) / 2 for left, right in xgroups], [(top + bottom) / 2 for top, bottom in ygroups]
    if not 3 <= len(xs) <= 21 or not 3 <= len(ys) <= 51:
        return None
    # Every line must span the outer rectangle. Missing grid segments indicate
    # merges or complex layout; never reconstruct them as ordinary cells.
    px = gray.load()
    for x in xs:
        if any(px[min(width - 1, round(x * scale)), y] >= 90 for y in range(math.ceil(ys[0] * scale), math.floor(ys[-1] * scale) + 1)):
            return None
    for y in ys:
        if any(px[x, min(height - 1, round(y * scale))] >= 90 for x in range(math.ceil(xs[0] * scale), math.floor(xs[-1] * scale) + 1)):
            return None
    return xs, ys, xgroups, ygroups


def grid_candidate(image, words, to_visible):
    """Only complete straight rectangular grids; no merged/cross-page claims."""
    geometry = grid_geometry(image)
    if not geometry:
        return []
    xs, ys, _, _ = geometry
    cells = []
    assigned = set()
    for row in range(len(ys) - 1):
        for column in range(len(xs) - 1):
            box = mapped_box([xs[column], ys[row], xs[column + 1], ys[row + 1]], to_visible)
            indices = [i for i, word in enumerate(words) if box[0] < (word["bbox"][0] + word["bbox"][2]) / 2 < box[2] and box[1] < (word["bbox"][1] + word["bbox"][3]) / 2 < box[3]]
            if any(not contained(words[i]["bbox"], box) for i in indices):
                return []
            assigned.update(indices)
            cells.append({"row": row, "column": column, "bbox": box, "wordIndices": indices,
                          "text": ("" if all(words[i]["confidence"] is None for i in indices) else " ").join(words[i]["text"] for i in indices)})
    within = [i for i, word in enumerate(words) if contained(word["bbox"], mapped_box([xs[0], ys[0], xs[-1], ys[-1]], to_visible))]
    if set(within) != assigned:
        return []
    return [{"kind": "ruled-rectangular-grid", "requiresConfirmation": True, "rows": len(ys) - 1, "columns": len(xs) - 1,
             "bbox": mapped_box([xs[0], ys[0], xs[-1], ys[-1]], to_visible), "cells": cells,
             "scope": "single-region-straight-grid-without-merged-cells"}]


def ocr_region(image, selection, page_number, runtime, languages, to_visible):
    rotation = selection.get("rotation", 0)
    original_size = image.size
    rotated = image.rotate(-rotation, expand=True) if rotation else image
    geometry = grid_geometry(rotated)
    processed = rotated
    if geometry:
        # Continuous grid rules cause Tesseract to classify the whole table as a
        # graphic. Remove only the proven rules in this derived raster, then run
        # one OCR route. Text and word coordinates still map to the original.
        processed = rotated.copy()
        draw = ImageDraw.Draw(processed)
        xs, ys, xgroups, ygroups = geometry
        for left, right in xgroups:
            draw.rectangle((math.floor(left) - 1, math.floor(ys[0]) - 2, math.ceil(right) + 1, math.ceil(ys[-1]) + 2), fill="white")
        for top, bottom in ygroups:
            draw.rectangle((math.floor(xs[0]) - 2, math.floor(top) - 1, math.ceil(xs[-1]) + 2, math.ceil(bottom) + 1), fill="white")
    ocr_scale = selection.get("ocrScale", 1)
    if ocr_scale != 1:
        processed = processed.resize((max(1, round(processed.width * ocr_scale)), max(1, round(processed.height * ocr_scale))), Image.Resampling.LANCZOS)
    inverse = inverse_rotation(rotation, *original_size)
    def original_transform(x, y):
        return to_visible(*inverse(x, y))
    def transform(x, y):
        return original_transform(x * rotated.width / processed.width, y * rotated.height / processed.height)
    # The bounded page's TEMP is inside its parent's private job directory.
    # If the page is killed, parent cleanup owns every derived raster and TSV.
    # An antivirus/indexer may briefly hold a completed diagnostic on Windows.
    # Leave an owned private temporary file on cleanup failure; never replace the
    # actual OCR/budget outcome with a cleanup PermissionError.
    with tempfile.TemporaryDirectory(prefix="ocr-", ignore_cleanup_errors=True) as directory:
        input_file = Path(directory) / "input.png"
        processed.save(input_file)
        segmentation = selection.get("segmentation", "auto")
        psm = {"auto": "3", "sparse": "11", "block": "6", "line": "7"}[segmentation]
        command = [str(runtime / "tesseract" / "tesseract.exe"), str(input_file), "stdout", "--tessdata-dir", str(runtime / "tessdata"), "-l", "+".join(languages), "--oem", "1", "--psm", psm, "-c", "tessedit_create_tsv=1"]
        # The page process is already in the parent-owned memory/time/job limit.
        # Capture to bounded files so communicate() cannot allocate unbounded output.
        output_file, diagnostic_file = Path(directory) / "output.tsv", Path(directory) / "diagnostic"
        with output_file.open("wb") as output, diagnostic_file.open("wb") as diagnostic:
            completed = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=output, stderr=diagnostic, creationflags=0x08000000 if __import__("os").name == "nt" else 0)
            started = time.monotonic()
            try:
                while completed.poll() is None:
                    if time.monotonic() - started > 55:
                        raise ValueError("ocr-time-budget")
                    if output_file.stat().st_size > 24 * 1024 * 1024 or diagnostic_file.stat().st_size > 256 * 1024:
                        raise ValueError("ocr-output-budget")
                    time.sleep(.02)
            finally:
                if completed.poll() is None:
                    completed.kill()
                completed.wait()
        if completed.returncode or output_file.stat().st_size > 24 * 1024 * 1024 or diagnostic_file.stat().st_size > 256 * 1024:
            raise ValueError("ocr-process-failed")
        raw_tsv = output_file.read_text(encoding="utf-8-sig")
    words, grouped = [], defaultdict(list)
    for record in csv.DictReader(io.StringIO(raw_tsv), delimiter="\t", quoting=csv.QUOTE_NONE):
        if record["level"] != "5" or not record.get("text", "").strip():
            continue
        x, y, width, height = (int(record[key]) for key in ("left", "top", "width", "height"))
        confidence = float(record["conf"])
        if not 0 <= confidence <= 100 or min(x, y, width, height) < 0 or x + width > processed.width or y + height > processed.height:
            raise ValueError("ocr-geometry-invalid")
        word = {"text": record["text"], "bbox": mapped_box([x, y, x + width, y + height], transform), "confidence": confidence}
        words.append(word)
        grouped[(record["block_num"], record["par_num"], record["line_num"])].append(word)
        if len(words) > MAX_WORDS:
            raise ValueError("word-budget")
    blocks = []
    for index, line in enumerate(grouped.values()):
        blocks.append({"id": f"p{page_number}-{selection['id']}-b{index + 1}", "page": page_number, "text": " ".join(word["text"] for word in line),
                       "bbox": bbox_union([word["bbox"] for word in line]), "sourceMethod": "ocr", "confidence": min(word["confidence"] for word in line), "words": line})
    losses = []
    if any(word["confidence"] < 50 for word in words):
        losses.append("low-confidence-words-require-review")
    if not words and not blank_image(rotated):
        losses.append("nonblank-region-without-recognized-text")
    tables = grid_candidate(rotated, words, original_transform)
    return {"id": selection["id"], "rect": selection["rect"], "method": "ocr", "status": "partial" if losses else "complete" if words else "blank",
            "rawText": "\n".join(block["text"] for block in blocks), "blocks": blocks,
            "ocrGeometry": {"width": processed.width, "height": processed.height, "rotation": rotation, "ocrScale": ocr_scale, "segmentation": selection.get("segmentation", "auto"), "preprocessing": "complete-straight-grid-rules-removed" if geometry else "none", "ocrToVisible": affine(transform)},
            "tables": tables, "losses": losses}


def pdf_characters(page, converter, scale):
    text = page.get_textpage()
    count = text.count_chars()
    if count > MAX_CHARACTERS:
        raise ValueError("character-budget")
    characters, losses, positions = [], [], defaultdict(list)
    for index in range(count):
        codepoint = pdfium.raw.FPDFText_GetUnicode(text, index)
        if codepoint == 0 or codepoint > 0x10ffff or 0xd800 <= codepoint <= 0xdfff:
            losses.append("unmapped-pdf-character")
            continue
        value = chr(codepoint)
        # PDFium synthesizes inter-object layout spaces (including Latin/Han
        # transitions) which are not Unicode characters in the source PDF.
        # Keep explicit source whitespace and generated line boundaries; omit
        # only generated horizontal whitespace rather than rewriting field text.
        if value.isspace() and value not in "\r\n" and pdfium.raw.FPDFText_IsGenerated(text, index) == 1:
            continue
        box = text.get_charbox(index)
        points = [converter.to_bitmap(x, y) for x in (box[0], box[2]) for y in (box[1], box[3])]
        rect = rounded([min(p[0] for p in points) / scale, min(p[1] for p in points) / scale, max(p[0] for p in points) / scale, max(p[1] for p in points) / scale])
        # Some scanned PDFs contain the same invisible OCR layer more than once.
        # Only identical glyphs at the same geometry are duplicate evidence;
        # repeated words elsewhere and legitimate adjacent glyphs are retained.
        key = (value, round(rect[0], 1), round(rect[1], 1))
        if not value.isspace() and any(all(abs(a - b) <= .1 for a, b in zip(rect, old)) for old in positions[key]):
            continue
        positions[key].append(rect)
        characters.append({"text": value, "bbox": rect})
    text.close()
    return characters, list(dict.fromkeys(losses))


def text_region(characters, selection, page_number):
    rect, lines, current, losses = selection["rect"], [], [], []
    for char in characters:
        box, value = char["bbox"], char["text"]
        if value in "\r\n":
            if current:
                lines.append(current)
                current = []
            continue
        # Half-open selections assign a glyph centered exactly on a shared edge
        # to one region only. Adjacent nonoverlapping selections cannot duplicate.
        if not rect[0] <= (box[0] + box[2]) / 2 < rect[2] or not rect[1] <= (box[1] + box[3]) / 2 < rect[3]:
            continue
        if not value.isspace() and not contained(box, rect):
            losses.append("text-crosses-selection-boundary")
        current.append(char)
    if current:
        lines.append(current)
    # PDFium may concatenate separate column text objects into one visual row.
    # Split only a large proven geometric gap, retaining all actual text. This
    # gives the separately reviewable order candidate real column blocks.
    runs = []
    for line in lines:
        current, previous = [], None
        for char in line:
            if not char["text"].isspace() and previous is not None:
                height = max(previous["bbox"][3] - previous["bbox"][1], char["bbox"][3] - char["bbox"][1])
                gap = char["bbox"][0] - previous["bbox"][2]
                same_row = min(char["bbox"][3], previous["bbox"][3]) > max(char["bbox"][1], previous["bbox"][1])
                if same_row and gap > max(height * 3, (rect[2] - rect[0]) * .04):
                    while current and current[-1]["text"].isspace():
                        current.pop()
                    if current:
                        runs.append(current)
                    current = []
            current.append(char)
            if not char["text"].isspace():
                previous = char
        if current:
            runs.append(current)
    blocks = []
    for line in runs:
        value = "".join(char["text"] for char in line)
        visible = [char for char in line if not char["text"].isspace()]
        if not visible:
            continue
        blocks.append({"id": f"p{page_number}-{selection['id']}-b{len(blocks) + 1}", "page": page_number, "text": value,
                       "bbox": bbox_union([char["bbox"] for char in visible]), "sourceMethod": "text-layer", "confidence": None,
                       "words": [{**char, "confidence": None} for char in line]})
    return {"id": selection["id"], "rect": rect, "method": "text-layer", "status": "partial" if losses else "complete" if blocks else "blank",
            "rawText": "\n".join(block["text"] for block in blocks), "blocks": blocks, "tables": [], "losses": list(dict.fromkeys(losses))}


def recognize_page(config):
    runtime = Path(config["runtime"])
    number, media_type, dpi = config["selection"]["page"], config["mediaType"], config["dpi"]
    versions = config["versions"]
    operation = config.get("operation", "recognize")
    if str(pdfium.PYPDFIUM_INFO.version) != versions["pdfium"]["pypdfium2Version"] or str(pdfium.PDFIUM_INFO.version) != versions["pdfium"]["pdfiumVersion"]:
        raise ValueError("pdfium-version-mismatch")
    document, bitmap, page, image = None, None, None, None
    try:
        if media_type == "application/pdf":
            document = pdfium.PdfDocument(Path(config["input"]).read_bytes())
            page_count = len(document)
            if page_count > 100 or number > page_count:
                raise ValueError("page-budget-or-selection")
            if operation == "inspect":
                inspected = []
                for index in range(page_count):
                    current = document[index]
                    try:
                        w, h = current.get_size()
                        inspected.append({"page": index + 1, "pageCount": page_count, "status": "complete", "width": w, "height": h,
                                          "coordinateSpace": "visible-page-top-left", "units": "pt", "rotation": current.get_rotation(),
                                          "cropBox": list(current.get_cropbox()), "mediaBox": list(current.get_mediabox()), "regions": [], "losses": []})
                    finally:
                        current.close()
                return {"page": number, "status": "complete", "regions": [], "losses": [], "inspectionPages": inspected}
            page = document[number - 1]
            width, height = page.get_size()
            scale = dpi / 72
            pixel_width, pixel_height = math.ceil(width * scale), math.ceil(height * scale)
            if pixel_width * pixel_height > MAX_PIXELS:
                raise ValueError("pixel-budget")
            converter = pdfium.PdfPosConv(page, (0, 0, pixel_width, pixel_height, 0))
            # Use the actual integer render dimensions for exact visible-page mapping.
            sx, sy = pixel_width / width, pixel_height / height
            characters, character_losses = pdf_characters(page, converter, 1) if operation == "recognize" else ([], [])
            for char in characters:
                char["bbox"] = rounded([char["bbox"][0] / sx, char["bbox"][1] / sy, char["bbox"][2] / sx, char["bbox"][3] / sy])
            image_rects, object_losses = [], []
            for index, obj in enumerate(page.get_objects(filter=[pdfium.raw.FPDF_PAGEOBJ_IMAGE], max_depth=10)) if operation == "recognize" else []:
                if index >= 1000:
                    object_losses.append("image-object-budget")
                    break
                if obj.level > 0:
                    object_losses.append("nested-image-geometry-requires-explicit-regions")
                    continue
                box = obj.get_bounds()
                image_rects.append(mapped_box([box[0], box[1], box[2], box[3]], lambda x, y: (converter.to_bitmap(x, y)[0] / sx, converter.to_bitmap(x, y)[1] / sy)))
            geometry = {"rotation": page.get_rotation(), "cropBox": list(page.get_cropbox()), "mediaBox": list(page.get_mediabox()),
                        "renderWidth": pixel_width, "renderHeight": pixel_height, "renderToPdf": affine(converter.to_page)}
            units = "pt"
            def get_image():
                nonlocal bitmap, image
                if image is None:
                    bitmap = page.render(scale=scale)
                    if bitmap.width != pixel_width or bitmap.height != pixel_height:
                        raise ValueError("render-geometry-mismatch")
                    image = bitmap.to_pil().convert("RGB")
                return image
        else:
            if number != 1:
                raise ValueError("image-page-selection")
            opened = Image.open(config["input"])
            if opened.format not in {"PNG", "JPEG"} or getattr(opened, "n_frames", 1) != 1 or opened.width * opened.height > MAX_PIXELS:
                raise ValueError("image-budget-or-format")
            orientation = opened.getexif().get(274, 1)
            raw_width, raw_height = opened.size
            oriented = ImageOps.exif_transpose(opened)
            if "A" in oriented.getbands() or "transparency" in oriented.info:
                rgba = oriented.convert("RGBA")
                image = Image.new("RGB", rgba.size, "white")
                with rgba.getchannel("A") as alpha:
                    image.paste(rgba, mask=alpha)
                rgba.close()
            else:
                image = oriented.convert("RGB")
            oriented.close()
            opened.close()
            width, height = image.size
            sx, sy = 1, 1
            page_count, characters, character_losses, image_rects, object_losses = 1, [], [], [[0, 0, width, height]], []
            visible_to_raw = {1: lambda x, y: (x, y), 2: lambda x, y: (raw_width - x, y), 3: lambda x, y: (raw_width - x, raw_height - y),
                              4: lambda x, y: (x, raw_height - y), 5: lambda x, y: (y, x), 6: lambda x, y: (y, raw_height - x),
                              7: lambda x, y: (raw_width - y, raw_height - x), 8: lambda x, y: (raw_width - y, x)}.get(orientation)
            if visible_to_raw is None:
                raise ValueError("exif-orientation-invalid")
            geometry, units = {"rotation": 0, "exifOrientation": orientation, "rawWidth": raw_width, "rawHeight": raw_height, "visibleToRaw": affine(visible_to_raw)}, "px"
            def get_image():
                return image
        if operation in {"preview", "inspect"}:
            result = {"page": number, "pageCount": page_count, "status": "complete", "width": width, "height": height,
                      "coordinateSpace": "visible-page-top-left", "units": units, **geometry, "regions": [], "losses": []}
            if operation == "preview":
                buffer = io.BytesIO()
                get_image().save(buffer, format="PNG")
                if buffer.tell() > 23 * 1024 * 1024:
                    raise ValueError("preview-output-budget")
                result["preview"] = {"mediaType": "image/png", "dataBase64": base64.b64encode(buffer.getvalue()).decode("ascii"),
                                     "width": get_image().width, "height": get_image().height}
            return result
        full = [0, 0, width, height]
        selections = config["selection"].get("regions") or [{"id": "page", "rect": full, "mode": "auto"}]
        regions, losses = [], character_losses + object_losses
        text_boxes = [char["bbox"] for char in characters if not char["text"].isspace()]
        for selection in selections:
            rect = selection["rect"]
            if not contained(rect, full):
                raise ValueError("region-outside-visible-page")
            has_text = any(overlap(rect, box) for box in text_boxes)
            mode = selection.get("mode", "auto")
            if media_type != "application/pdf" and mode == "text":
                regions.append({"id": selection["id"], "rect": rect, "method": "text-layer", "status": "failed", "rawText": "", "blocks": [], "losses": ["image-has-no-text-layer"]})
                continue
            if mode == "text" or mode == "auto" and has_text and not character_losses:
                region = text_region(characters, selection, number)
                left, top, right, bottom = [round(rect[0] * sx), round(rect[1] * sy), round(rect[2] * sx), round(rect[3] * sy)]
                with get_image().crop((left, top, right, bottom)) as table_image:
                    region["tables"] = grid_candidate(table_image, [word for block in region["blocks"] for word in block["words"]],
                                                       lambda x, y, l=left, t=top: ((x + l) / sx, (y + t) / sy))
                regions.append(region)
                if not region["blocks"] and not blank_image(get_image().crop((round(rect[0] * sx), round(rect[1] * sy), round(rect[2] * sx), round(rect[3] * sy)))):
                    region["status"] = "partial"
                    region["losses"].append("nonblank-region-without-text-layer")
                if mode == "auto":
                    # Keep authoritative text. OCR only fully disjoint visible image
                    # rectangles. Overlays need an explicit review selection.
                    selected_images = [box for box in image_rects if overlap(box, rect)]
                    for index, box in enumerate(selected_images):
                        box = [max(box[0], rect[0]), max(box[1], rect[1]), min(box[2], rect[2]), min(box[3], rect[3])]
                        if any(overlap(box, old) for old in text_boxes) or any(overlap(box, old) for old in selected_images[:index]):
                            region["status"] = "partial"
                            region["losses"].append("mixed-image-text-overlap-unresolved")
                            continue
                        auto_selection = {"id": f"__weftext_auto_{selection['id']}-image-{index + 1}", "rect": box, "rotation": selection.get("rotation", 0), "segmentation": selection.get("segmentation", "auto"), "ocrScale": selection.get("ocrScale", 1)}
                        region.setdefault("excludedRects", []).append(box)
                        left, top, right, bottom = [round(box[0] * sx), round(box[1] * sy), round(box[2] * sx), round(box[3] * sy)]
                        cropped = get_image().crop((left, top, right, bottom))
                        regions.append(ocr_region(cropped, auto_selection, number, runtime, config["languages"], lambda x, y, l=left, t=top: ((x + l) / sx, (y + t) / sy)))
            else:
                left, top, right, bottom = [round(rect[0] * sx), round(rect[1] * sy), round(rect[2] * sx), round(rect[3] * sy)]
                cropped = get_image().crop((left, top, right, bottom))
                region = ocr_region(cropped, selection, number, runtime, config["languages"], lambda x, y, l=left, t=top: ((x + l) / sx, (y + t) / sy))
                if character_losses:
                    region["losses"].append("invalid-text-layer-replaced-by-explicit-single-ocr-route")
                regions.append(region)
        all_blocks = [block for region in regions for block in region["blocks"]]
        statuses = [region["status"] for region in regions]
        status = "complete" if all(s in {"complete", "blank"} for s in statuses) and not losses else "failed" if all(s == "failed" for s in statuses) else "partial"
        if status == "complete" and all(s == "blank" for s in statuses):
            status = "blank"
        return {"page": number, "pageCount": page_count, "status": status, "width": width, "height": height, "coordinateSpace": "visible-page-top-left", "units": units,
                **geometry, "regions": regions, "readingOrder": order_candidates(all_blocks, width, height), "losses": list(dict.fromkeys(losses))}
    except Exception as exc:
        # Keep exact failure categories without serializing library messages, paths,
        # document passwords, raw text or model diagnostics.
        code = str(exc) if type(exc) is ValueError and str(exc) in {"page-budget-or-selection", "pixel-budget", "character-budget", "word-budget", "region-outside-visible-page", "image-page-selection", "image-budget-or-format", "exif-orientation-invalid", "pdfium-version-mismatch", "render-geometry-mismatch", "ocr-process-failed", "ocr-geometry-invalid", "preview-output-budget", "ocr-time-budget", "ocr-output-budget"} else "page-recognition-failed"
        if isinstance(exc, pdfium.PdfiumError):
            code = {pdfium.raw.FPDF_ERR_PASSWORD: "encrypted-pdf-requires-password", pdfium.raw.FPDF_ERR_FORMAT: "corrupt-pdf"}.get(exc.err_code, "pdf-decoding-failed")
        return {"page": number, "status": "failed", "regions": [], "losses": [code]}
    finally:
        if image is not None:
            image.close()
        if bitmap is not None:
            bitmap.close()
        if page is not None:
            page.close()
        if document is not None:
            document.close()
