"""Read bounded OOXML paragraph evidence without rendering or executing content.

The Core must freeze/authorize the original bytes before calling ``read_docx``.
This is structure evidence for schemes, not a complete DOCX body importer.
"""
from __future__ import annotations

import hashlib
import io
from pathlib import Path
import posixpath
import xml.etree.ElementTree as ET
import zipfile

MEDIA = "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
NS = {"w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main"}
W = "{" + NS["w"] + "}"
MAX_INPUT = 16 * 1024 * 1024
MAX_XML = 32 * 1024 * 1024


class DocxEvidenceError(ValueError):
    """Unsupported, malformed, or excessive package; no structural claims."""


def _value(node, path):
    item = node.find(path, NS) if node is not None else None
    return item.get(W + "val") if item is not None else None


def _integer(value, low=0, high=2**31 - 1):
    try:
        number = int(value)
    except (ValueError, TypeError):
        return None
    return number if low <= number <= high else None


def read_docx(source: bytes | Path, expected_sha256: str | None = None):
    """Return real paragraphs with immutable part / ordinal / text provenance.

    ``blockIndex`` is the zero-based paragraph ordinal in document.xml (including
    paragraphs in tables), not a guessed page or an XML byte offset. Revisions,
    textboxes, fields and non-text content are disclosed as partial evidence.
    """
    data = source.read_bytes() if isinstance(source, Path) else source
    if not isinstance(data, bytes) or not data or len(data) > MAX_INPUT:
        raise DocxEvidenceError("DOCX input exceeds the fixed 16 MiB gate or is empty")
    digest = hashlib.sha256(data).hexdigest()
    if expected_sha256 is not None and digest != expected_sha256:
        raise DocxEvidenceError("original digest mismatch")
    try:
        archive = zipfile.ZipFile(io.BytesIO(data))
        infos = archive.infolist()
        names = [item.filename for item in infos]
        if len(infos) > 4096 or len(set(names)) != len(names):
            raise DocxEvidenceError("duplicate or excessive package parts")
        if any(item.flag_bits & 1 for item in infos) or sum(item.file_size for item in infos) > MAX_XML:
            raise DocxEvidenceError("encrypted or oversized expanded package")

        xml_nodes = 0

        def xml(part, required=False):
            nonlocal xml_nodes
            if part not in names:
                if required:
                    raise DocxEvidenceError("missing " + part)
                return None
            raw = archive.read(part)
            if b"<!DOCTYPE" in raw.upper() or b"<!ENTITY" in raw.upper():
                raise DocxEvidenceError("DTD and entities are not supported")
            root, depth = None, 0
            for event, node in ET.iterparse(io.BytesIO(raw), events=("start", "end")):
                if event == "start":
                    xml_nodes += 1
                    depth += 1
                    if xml_nodes > 200_000 or depth > 128:
                        raise DocxEvidenceError("DOCX XML complexity budget exceeded")
                    if root is None:
                        root = node
                else:
                    depth -= 1
            return root

        def related(relationships, suffix, base=""):
            matches = [] if relationships is None else [item for item in relationships
                if item.get("Type", "").endswith("/" + suffix)]
            if not matches:
                return None
            if len(matches) != 1 or matches[0].get("TargetMode", "Internal") != "Internal":
                raise DocxEvidenceError("ambiguous or external structural relationship")
            target = matches[0].get("Target", "")
            if not target or "\\" in target or ":" in target or "#" in target:
                raise DocxEvidenceError("invalid structural relationship target")
            part = posixpath.normpath(target.lstrip("/") if target.startswith("/") else posixpath.join(base, target))
            if part.startswith("../") or part not in names:
                raise DocxEvidenceError("structural relationship part is unavailable")
            return part

        main_part = related(xml("_rels/.rels", True), "officeDocument")
        if main_part != "word/document.xml":
            raise DocxEvidenceError("unsupported main document part")
        document = xml(main_part, True)
        relationships = xml("word/_rels/document.xml.rels")
        styles_part = related(relationships, "styles", "word")
        numbering_part = related(relationships, "numbering", "word")
        styles_xml = xml(styles_part) if styles_part else None
        numbering_xml = xml(numbering_part) if numbering_part else None
        styles = {}
        if styles_xml is not None:
            for style in styles_xml.findall("w:style", NS):
                key = style.get(W + "styleId")
                if key in styles:
                    raise DocxEvidenceError("duplicate style definition")
                styles[key] = style

        def style_chain(key):
            chain, seen = [], set()
            while key:
                if key in seen or key not in styles:
                    return chain, False
                seen.add(key)
                chain.append(styles[key])
                key = _value(styles[key], "w:basedOn")
            return chain, True

        nums, abstracts = {}, {}
        if numbering_xml is not None:
            for tag, attr, target in (("num", "numId", nums), ("abstractNum", "abstractNumId", abstracts)):
                for item in numbering_xml.findall("w:" + tag, NS):
                    key = item.get(W + attr)
                    if key in target:
                        raise DocxEvidenceError("duplicate numbering definition")
                    target[key] = item
        blocks, issues, text_bytes = [], [], 0
        for index, paragraph in enumerate(document.findall(".//w:p", NS)):
            # Read the literal w:t sequence; do not accept/reject revisions or
            # execute field instructions to fabricate the rendered paragraph.
            text = "".join(item.text or "" if item.tag == W + "t" else "\t" if item.tag == W + "tab" else "\n"
                           for item in paragraph.iter() if item.tag in (W + "t", W + "tab", W + "br", W + "cr"))
            block = {"id": f"docx:p:{index}", "sourceMethod": "docx", "blockIndex": index,
                     "text": text, "part": "word/document.xml", "paragraphIndex": index}
            ppr = paragraph.find("w:pPr", NS)
            style_id = _value(ppr, "w:pStyle")
            chain, proven = style_chain(style_id)
            props = [ppr, *(style.find("w:pPr", NS) for style in chain if proven)]
            if style_id:
                block["styleId"] = style_id
                if proven:
                    block["style"] = _value(chain[0], "w:name") or style_id
                    block["styleSourcePart"] = styles_part
                else:
                    issues.append({"code": "unresolved-docx-style", "blockId": block["id"]})
            outline = next((_value(prop, "w:outlineLvl") for prop in props if _value(prop, "w:outlineLvl") is not None), None)
            level = _integer(outline, high=8)
            if level is not None:
                block["outlineLevel"] = level + 1
            num_id = next((_value(prop, "w:numPr/w:numId") for prop in props if _value(prop, "w:numPr/w:numId") is not None), None)
            ilvl = next((_value(prop, "w:numPr/w:ilvl") for prop in props if _value(prop, "w:numPr/w:ilvl") is not None), "0")
            if num_id and num_id != "0":
                num = nums.get(num_id)
                abstract_id = _value(num, "w:abstractNumId")
                abstract = abstracts.get(abstract_id)
                definitions = [] if abstract is None else [item for item in abstract.findall("w:lvl", NS) if item.get(W + "ilvl") == ilvl]
                overrides = [] if num is None else [item for item in num.findall("w:lvlOverride", NS) if item.get(W + "ilvl") == ilvl]
                replacement = overrides[0].find("w:lvl", NS) if len(overrides) == 1 else None
                definition = replacement if replacement is not None else definitions[0] if len(definitions) == 1 else None
                format_name = _value(definition, "w:numFmt")
                pattern = _value(definition, "w:lvlText")
                native_level = _integer(ilvl, high=8)
                valid = num is not None and definition is not None and native_level is not None and format_name is not None and pattern is not None and len(overrides) <= 1
                block["numbering"] = {"proven": valid, "numId": num_id, "abstractNumId": abstract_id,
                    "level": native_level + 1 if native_level is not None else None,
                    "format": format_name, "pattern": pattern,
                    "start": _integer(_value(definition, "w:start")),
                    "startOverride": _integer(_value(overrides[0], "w:startOverride")) if len(overrides) == 1 else None,
                    "restartAfterLevel": _integer(_value(definition, "w:lvlRestart"), high=9),
                    "sourcePart": numbering_part}
                if not valid:
                    issues.append({"code": "unresolved-docx-numbering", "blockId": block["id"]})
            blocks.append(block)
            text_bytes += len(text.encode("utf-8"))
            if len(blocks) > 20_000 or text_bytes > 4 * 1024 * 1024:
                raise DocxEvidenceError("DOCX paragraph evidence budget exceeded")
        unsupported = {"ins", "del", "moveFrom", "moveTo", "txbxContent", "instrText", "drawing", "object", "altChunk"}
        found = sorted({node.tag[len(W):] for node in document.iter() if node.tag.startswith(W) and node.tag[len(W):] in unsupported})
        if found:
            issues.append({"code": "docx-content-requires-original-body-importer", "elements": found})
        if any(name.endswith("vbaProject.bin") for name in names):
            issues.append({"code": "docx-active-content-not-executed"})
        return {"version": 1, "status": "partial" if issues else "complete",
            "input": {"mediaType": MEDIA, "sha256": digest, "bytes": len(data)}, "pages": [],
            "docxBlocks": blocks, "issues": issues,
            "adapter": {"id": "weftext-ooxml-paragraph-evidence", "version": 1,
                        "blockIndexMeaning": "word/document.xml paragraph ordinal, zero-based"}}
    except (zipfile.BadZipFile, ET.ParseError, KeyError, RuntimeError) as error:
        raise DocxEvidenceError("malformed DOCX package") from error
