"""Bounded, offline proposal rules over genuine recognition / DOCX evidence.

This module never performs OCR, writes a business record, executes author text,
or confirms a proposed value. Source offsets use Unicode code points; boxes are
the original page boxes supplied by the reader, not reconstructed coordinates.
"""

from __future__ import annotations

import copy
import datetime as dt
import re
from decimal import Decimal, InvalidOperation, localcontext
from typing import Any

INVOICE = "invoice-cn-mainland-digital-ordinary-single-page"
OFFICIAL = "official-document-cn-mainland-hans"
VERSION = 1
SCHEMES = {
    INVOICE: {
        "id": INVOICE,
        "version": VERSION,
        "name": "中国大陆电子发票（普通发票）标准单页",
        "language": "zh",
        "script": "Hans",
        "region": "CN",
        "mediaTypes": ["application/pdf", "image/png", "image/jpeg"],
        "layout": "single-page-without-specific-business-label",
        "reference": "https://fgk.chinatax.gov.cn/zcfgk/c100012/c5236067/content.html",
    },
    OFFICIAL: {
        "id": OFFICIAL,
        "version": VERSION,
        "name": "中国大陆简体中文公文候选",
        "language": "zh",
        "script": "Hans",
        "region": "CN",
        "mediaTypes": [
            "application/pdf", "image/png", "image/jpeg",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ],
    },
}
MAX_BLOCKS = 20_000
MAX_TEXT = 4 * 1024 * 1024
_SPACE = r"[ \t\u3000]*"
_COLON = rf"{_SPACE}[:：]{_SPACE}"
def _flex(pattern):
    """Allow OCR word separation inside fixed Han labels, preserving raw text."""
    return re.sub(r"([\u4e00-\u9fff])", lambda match: match[0] + _SPACE, pattern)


_TITLE = re.compile(_flex(r"电子发票\s*[（(]\s*普通发票\s*[）)]"))
_SPECIAL = (
    "建筑服务", "成品油", "报废产品收购", "货物运输服务", "不动产销售",
    "不动产经营租赁服务", "旅客运输服务", "农产品收购", "光伏收购",
)
_PARTY_LABEL = r"名称|统一社会信用代码\s*[/／、]?\s*纳税人识别号|统一社会信用代码|纳税人识别号"
_PARTY = re.compile(rf"(?<![A-Za-z\u4e00-\u9fff])(?P<label>{_flex(_PARTY_LABEL)}){_COLON}")
_TOTAL_LABEL = r"价税合计(?:[（(][^）)\r\n]{1,24}[）)])?"


class SchemeInputError(ValueError):
    """Malformed or excessive evidence; no proposal was produced."""


def _bbox(value: Any) -> list[float] | None:
    if value is None:
        return None
    if not isinstance(value, (list, tuple)) or len(value) != 4:
        raise SchemeInputError("invalid source box")
    if any(isinstance(x, bool) or not isinstance(x, (int, float)) for x in value):
        raise SchemeInputError("invalid source box")
    box = [float(x) for x in value]
    if any(not (-1e8 < x < 1e8) for x in box) or box[0] > box[2] or box[1] > box[3]:
        raise SchemeInputError("invalid source box")
    return box


def _blocks(document: dict[str, Any]) -> list[dict[str, Any]]:
    output: list[dict[str, Any]] = []
    text_size = 0
    seen: set[str] = set()

    def add(raw: Any, page: int | None, region: str | None) -> None:
        nonlocal text_size
        if not isinstance(raw, dict) or not isinstance(raw.get("text"), str):
            raise SchemeInputError("source block requires original text")
        ident = raw.get("id")
        if not isinstance(ident, str) or not ident or ident in seen:
            raise SchemeInputError("source block IDs must be unique")
        seen.add(ident)
        text_size += len(raw["text"].encode("utf-8"))
        if len(output) >= MAX_BLOCKS or text_size > MAX_TEXT:
            raise SchemeInputError("scheme evidence budget exceeded")
        method = raw.get("sourceMethod")
        if method not in ("text-layer", "ocr", "docx"):
            raise SchemeInputError("source method must be explicit")
        source = {"id": ident, "text": raw["text"], "sourceMethod": method}
        if raw.get("confidence") is not None:
            confidence = raw["confidence"]
            if isinstance(confidence, bool) or not isinstance(confidence, (int, float)) or not 0 <= confidence <= 100:
                raise SchemeInputError("invalid original confidence")
            source["confidence"] = confidence
        if method == "docx":
            index = raw.get("blockIndex")
            if isinstance(index, bool) or not isinstance(index, int) or index < 0:
                raise SchemeInputError("DOCX block requires its original block index")
            source["blockIndex"] = index
            for key in ("style", "styleId", "outlineLevel", "numbering", "part", "paragraphIndex"):
                if key in raw:
                    source[key] = copy.deepcopy(raw[key])
        else:
            if isinstance(page, bool) or not isinstance(page, int) or page < 1:
                raise SchemeInputError("source page is required")
            source.update(page=page, regionId=region, bbox=_bbox(raw.get("bbox")))
            # Words may refine a quote's box only when they align exactly to raw
            # text with whitespace-only gaps. No search/reordering fallback.
            ranges, cursor = [], 0
            words = raw.get("words", [])
            if isinstance(words, list):
                for word in words:
                    if not isinstance(word, dict) or not isinstance(word.get("text"), str) or not word["text"]:
                        ranges = []
                        break
                    start = raw["text"].find(word["text"], cursor)
                    if start < 0 or raw["text"][cursor:start].strip():
                        ranges = []
                        break
                    end = start + len(word["text"])
                    word_box = _bbox(word.get("bbox"))
                    if word_box is None:
                        ranges = []
                        break
                    confidence = word.get("confidence")
                    if confidence is not None and (isinstance(confidence, bool) or not isinstance(confidence, (int, float)) or not 0 <= confidence <= 100):
                        raise SchemeInputError("invalid original word confidence")
                    ranges.append((start, end, word_box, confidence))
                    cursor = end
                if ranges and not raw["text"][cursor:].strip():
                    source["wordRanges"] = ranges
        output.append(source)

    pages = document.get("pages", [])
    if not isinstance(pages, list):
        raise SchemeInputError("pages must be an array")
    for page in pages:
        if not isinstance(page, dict):
            raise SchemeInputError("invalid page")
        for region in page.get("regions", []):
            if not isinstance(region, dict):
                raise SchemeInputError("invalid region")
            for block in region.get("blocks", []):
                add(block, page.get("page"), region.get("id"))
    for block in document.get("docxBlocks", []):
        add(block, None, None)
    return output


def _source(block: dict[str, Any], start: int = 0, end: int | None = None) -> dict[str, Any]:
    if end is None:
        end = len(block["text"])
    result = {
        "blockId": block["id"], "quote": block["text"][start:end],
        "start": start, "end": end, "offsetUnit": "unicode-code-point",
        "sourceMethod": block["sourceMethod"],
    }
    if "confidence" in block:
        result["confidence"] = block["confidence"]
    if block["sourceMethod"] == "docx":
        result["blockIndex"] = block["blockIndex"]
        for key in ("part", "paragraphIndex"):
            if key in block:
                result[key] = block[key]
    else:
        result.update(page=block["page"], regionId=block["regionId"], bbox=block["bbox"])
        result["positionPrecision"] = "block" if block["bbox"] is not None else "unavailable"
        ranges = [item for item in block.get("wordRanges", []) if item[0] < end and item[1] > start]
        if ranges:
            result["bbox"] = [min(item[2][0] for item in ranges), min(item[2][1] for item in ranges),
                              max(item[2][2] for item in ranges), max(item[2][3] for item in ranges)]
            result["positionPrecision"] = "word"
            confidences = [item[3] for item in ranges if item[3] is not None]
            if confidences:
                result["confidence"] = min(confidences)
    return result


def _candidate(block: dict[str, Any], start: int, end: int, parser=None, **extra) -> dict[str, Any]:
    raw = block["text"][start:end]
    value = raw.strip()
    parsed, error = (value, None) if parser is None else parser(value)
    if not value:
        parsed, error = None, "empty-value"
    candidate = {
        "raw": raw, "parsed": parsed, "parseStatus": "parsed" if error is None else "parse-failure",
        "sources": [_source(block, start, end)], "confirmed": False, **extra,
    }
    if error is not None:
        candidate["reason"] = error
    return candidate


def _field(candidates: list[dict[str, Any]], *, ambiguous: bool = False) -> dict[str, Any]:
    result: dict[str, Any] = {"status": "missing", "candidates": candidates, "confirmed": False}
    if not candidates:
        return result
    if ambiguous:
        result["status"] = "ambiguous"
    elif len(candidates) > 1:
        comparable = {(item["parseStatus"], item["parsed"], item.get("reason")) for item in candidates}
        result["status"] = "duplicate" if len(comparable) == 1 else "ambiguous"
    elif candidates[0]["parseStatus"] == "parse-failure":
        result["status"] = "parse-failure"
    else:
        result["status"] = "present"
        result["value"] = candidates[0]["parsed"]
    return result


def _number(raw: str) -> tuple[str | None, str | None]:
    if re.fullmatch(r"[0-9]{20}", raw):
        return raw, None
    return None, "invoice-number-must-contain-exactly-20-ascii-digits"


def _date(raw: str) -> tuple[str | None, str | None]:
    match = re.fullmatch(r"([0-9]{4})\s*年\s*([0-9]{1,2})\s*月\s*([0-9]{1,2})\s*日", raw)
    if match is None:
        match = re.fullmatch(r"([0-9]{4})-([0-9]{2})-([0-9]{2})", raw)
    if match is None:
        return None, "date-format-unknown-or-ambiguous"
    try:
        return dt.date(*(int(part) for part in match.groups())).isoformat(), None
    except ValueError:
        return None, "invalid-calendar-date"


def _money(raw: str) -> tuple[str | None, str | None]:
    # The symbol is retained in raw evidence. It does not supply a currency.
    raw = re.sub(r"^(?:人民币|CNY|RMB|[¥￥])\s*", "", raw).strip()
    if len(raw) > 128:
        return None, "amount-exceeds-supported-precision-budget"
    if not re.fullmatch(r"-?(?:[0-9]+|[1-9][0-9]{0,2}(?:,[0-9]{3})+)\.[0-9]{2}", raw):
        return None, "amount-requires-unambiguous-two-decimal-place-number"
    try:
        return format(Decimal(raw.replace(",", "")), ".2f"), None
    except InvalidOperation:
        return None, "invalid-decimal"


def _tax_id(raw: str) -> tuple[str | None, str | None]:
    # Preserve text and leading zeroes; this is syntax, not validity checking.
    if re.fullmatch(r"[0-9A-Z]{15,20}", raw):
        return raw, None
    return None, "tax-id-not-an-unambiguous-uppercase-alphanumeric-string"


def _currency(raw: str) -> tuple[str | None, str | None]:
    if re.sub(r"\s+", "", raw) in ("人民币", "CNY", "RMB"):
        return "CNY", None
    return None, "currency-not-explicitly-supported"


def _line_values(blocks, label: str, parser=None):
    result = []
    pattern = re.compile(rf"(?:{_flex(label)}){_COLON}")
    for block in blocks:
        for match in pattern.finditer(block["text"]):
            # A following known label on the same line has its own evidence.
            start = match.end()
            value = re.split(r"[\r\n]", block["text"][start:], maxsplit=1)[0]
            stop = re.search(_flex(r"\s+(?:发票号码|开票日期|名称|纳税人识别号|统一社会信用代码|金额合计|税额合计|" + _TOTAL_LABEL + r"|币种|发文机关|发文字号|文号|成文日期|发文日期|主送(?:机关)?)\s*[:：]"), value)
            end = start + (stop.start() if stop else len(value))
            result.append(_candidate(block, start, end, parser))
    return result


def _summary_amounts(blocks, fields, issues):
    """Use the real two amount columns, never text order, for an unlabeled total."""
    def segment(block, match):
        source = _source(block, match.start(), match.end())
        if source.get("bbox") is None:
            return None
        if source["positionPrecision"] == "word" or block["text"].strip() == match[0].strip():
            return source["bbox"]
        return None

    rows = []
    headers = {"amount": [], "tax": []}
    for block in blocks:
        for match in re.finditer(r"^\s*合\s*计", block["text"]):
            if (box := segment(block, match)) is not None:
                rows.append(box)
        for name, label in (("amount", "金额"), ("tax", "税额")):
            for match in re.finditer(rf"(?<![\w\u4e00-\u9fff]){_flex(label)}(?![\u4e00-\u9fff：:])", block["text"]):
                if (box := segment(block, match)) is not None:
                    headers[name].append(box)
    if not rows or any(len(hits) != 1 for hits in headers.values()):
        return
    a, t = headers["amount"][0], headers["tax"][0]
    if a[2] >= t[0] or abs(a[1] - t[1]) > 25:
        issues.append({"code": "summary-columns-ambiguous", "severity": "review-required"})
        return
    split = (a[2] + t[0]) / 2
    admitted = set()
    for box in rows:
        if box[1] <= max(a[3], t[3]):
            continue
        for block in blocks:
            for match in re.finditer(r"(?<![0-9A-Za-z.,])(?:[¥￥]\s*)?-?[0-9][0-9,]*\.[0-9]{2}(?![0-9A-Za-z.,])", block["text"]):
                value_box = segment(block, match)
                if value_box is None or abs((value_box[1] + value_box[3] - box[1] - box[3]) / 2) > max(5, (box[3] - box[1]) / 2):
                    continue
                name = "amount" if a[0] - 30 <= value_box[0] and value_box[2] < split else "tax" if value_box[0] > split else None
                ident = (name, block["id"], match.start(), match.end())
                if name and ident not in admitted:
                    admitted.add(ident)
                    fields[name].append(_candidate(block, match.start(), match.end(), _money,
                                                   columnEvidence="explicit-table-header-and-summary-row"))


def _role_header(block):
    # A business name containing 购买方/销售方 does not establish a role.
    first_value = _PARTY.search(block["text"])
    return block["text"][:first_value.start()] if first_value else block["text"]


def _party_role(block, anchors, page_width, quote_box=None) -> tuple[str | None, str | None]:
    text = re.sub(r"\s+", "", _role_header(block))
    explicit = [role for role, label in (("buyer", "购买方"), ("seller", "销售方")) if label in text]
    for role, label in (("buyer", "购买方"), ("seller", "销售方")):
        declared = rf"(?:^|\s){_flex(label + r'(?:信息)?')}\s*(?=(?:{_flex(_PARTY_LABEL)}){_COLON})"
        if re.search(declared, block["text"]) and role not in explicit:
            explicit.append(role)
    if len(explicit) == 1:
        return explicit[0], "explicit-label-in-source-block"
    box = quote_box if quote_box is not None else block.get("bbox")
    if box is None or set(anchors) != {"buyer", "seller"}:
        return None, "party-column-geometry-unavailable"
    left, right = sorted(anchors.items(), key=lambda pair: pair[1][0])
    a, b = left[1], right[1]
    if abs(a[1] - b[1]) > 30 or a[2] >= b[0]:
        return None, "party-column-geometry-ambiguous"
    if not isinstance(page_width, (int, float)) or isinstance(page_width, bool) or page_width <= 0:
        return None, "original-page-width-unavailable"
    midpoint = page_width / 2
    if not (a[2] < midpoint <= b[0]):
        return None, "party-anchors-do-not-prove-standard-left-right-columns"
    # Original geometry is in PDF points or image pixels. A fixed 96-pixel
    # band wrongly excluded 300-DPI tax IDs; normalize the named layout's
    # bounded header band by the original page width, without changing boxes.
    scale = page_width / 900
    top, bottom = min(a[1], b[1]) - 2 * scale, max(a[3], b[3]) + 96 * scale
    if box[1] < top or box[3] > bottom:
        return None, "source-is-outside-bounded-party-header-band"
    if box[2] < midpoint:
        return left[0], "two-explicit-party-columns"
    if box[0] > midpoint:
        return right[0], "two-explicit-party-columns"
    return None, "source-spans-both-party-columns"


def _invoice(document, blocks, result):
    fields = {name: [] for name in (
        "invoiceNumber", "issueDate", "buyerName", "buyerTaxId", "sellerName", "sellerTaxId",
        "amount", "tax", "total", "currency",
    )}
    pages = document.get("pages", [])
    if len(pages) != 1 or document.get("pageCount") != 1:
        result.update(applicability="unsupported", reason="scheme-requires-one-original-page")
        return result
    title_hits = [(block, match) for block in blocks for match in _TITLE.finditer(block["text"])]
    if len(title_hits) != 1:
        result.update(applicability="unsupported" if title_hits else "not-applicable",
                      reason="multiple-invoices-not-grouped" if title_hits else "required-invoice-title-not-found")
        return result
    # Specific-business labels are only meaningful in the title/header band.
    title_block, title_match = title_hits[0]
    title_y = title_block.get("bbox", [0, 0, 0, 0])
    title_y = title_y[3] if title_y else None
    for block in blocks:
        compact = re.sub(r"\s+", "", block["text"])
        box = block.get("bbox")
        header = block is title_block or (box is not None and title_y is not None and box[1] < title_y + 35)
        if header and (any(label in compact for label in _SPECIAL) or "特定业务标签" in compact):
            result.update(applicability="unsupported", reason="specific-business-layout-not-supported")
            return result
    result["titleSource"] = _source(title_block, title_match.start(), title_match.end())
    fields["invoiceNumber"] = _line_values(blocks, "发票号码", _number)
    distinct_numbers = {item["parsed"] for item in fields["invoiceNumber"] if item["parseStatus"] == "parsed"}
    if len(distinct_numbers) > 1:
        result.update(applicability="unsupported", reason="competing-invoice-numbers-not-grouped")
        result["unassignedInvoiceNumbers"] = fields["invoiceNumber"]
        return result
    fields["issueDate"] = _line_values(blocks, "开票日期", _date)
    fields["amount"] = _line_values(blocks, "金额合计|合计金额", _money)
    fields["tax"] = _line_values(blocks, "税额合计|合计税额", _money)
    fields["total"] = _line_values(blocks, _TOTAL_LABEL + r"|[（(]小写[）)]", _money)
    if not fields["total"]:
        for block in blocks:
            for match in re.finditer(r"[（(]小写[）)]\s*[:：]?\s*(?P<value>[^\r\n]+)", block["text"]):
                fields["total"].append(_candidate(block, match.start("value"), match.end("value"), _money))
    _summary_amounts(blocks, fields, result["issues"])
    fields["currency"] = _line_values(blocks, "币种", _currency)
    # Reject competing invoice headers rather than arbitrarily choosing one.
    if len(fields["invoiceNumber"]) > 1:
        result["issues"].append({"code": "competing-invoice-numbers", "severity": "review-required"})
    anchors = {}
    for role, label in (("buyer", "购买方"), ("seller", "销售方")):
        hits = []
        for block in blocks:
            for match in re.finditer(_flex(label + "(?:信息)?"), _role_header(block)):
                source = _source(block, match.start(), match.end())
                label_only = re.sub(r"\s+", "", block["text"]) in (label, label + "信息")
                if source.get("bbox") is not None and (label_only or source["positionPrecision"] == "word"):
                    hits.append(source["bbox"])
        if len(hits) == 1:
            anchors[role] = hits[0]
    unresolved = {"Name": [], "TaxId": []}
    for block in blocks:
        for match in _PARTY.finditer(block["text"]):
            value_start = match.end()
            value = re.split(r"[\r\n]", block["text"][value_start:], maxsplit=1)[0]
            value_end = value_start + len(value)
            role_then_label = rf"{_flex(r'(?:购买方|销售方)(?:信息)?')}\s*(?=(?:{_flex(_PARTY_LABEL)}){_COLON})"
            next_label = re.search(rf"(?:{_flex(_PARTY_LABEL)}){_COLON}|{role_then_label}", value)
            if next_label:
                value_end = value_start + next_label.start()
            key = "Name" if re.sub(r"\s+", "", match.group("label")) == "名称" else "TaxId"
            quote_source = _source(block, value_start, value_end)
            role, proof = _party_role(block, anchors, pages[0].get("width"), quote_source.get("bbox"))
            candidate = _candidate(block, value_start, value_end, None if key == "Name" else _tax_id,
                                   roleEvidence=proof)
            if role is None:
                unresolved[key].append(candidate)
            else:
                fields[role + key].append(candidate)
    result["fields"] = {key: _field(value) for key, value in fields.items()}
    for suffix, items in unresolved.items():
        if items:
            for role in ("buyer", "seller"):
                key = role + suffix
                # Even an explicit candidate cannot discard competing unassigned text.
                result["fields"][key] = _field(fields[key] + copy.deepcopy(items), ambiguous=True)
    for key, field in result["fields"].items():
        field["valueKind"] = "decimal" if key in ("amount", "tax", "total") else "date" if key == "issueDate" else "text"
    if result["fields"]["currency"]["status"] == "missing":
        result["issues"].append({"code": "currency-not-stated", "severity": "review-required"})
    parsed = [result["fields"][key].get("value") for key in ("amount", "tax", "total")]
    if all(value is not None for value in parsed):
        with localcontext() as context:
            context.prec = 260  # Every admitted amount has at most 128 source characters.
            if Decimal(parsed[0]) + Decimal(parsed[1]) != Decimal(parsed[2]):
                result["issues"].append({"code": "amount-plus-tax-does-not-equal-total", "severity": "review-required"})
    result["unsupportedFields"] = ["lineItems", "taxAuthenticity", "businessValidity"]
    return result


_PREFIXES = (
    (1, re.compile(r"^[一二三四五六七八九十百]+\s*[、．.]\s*")),
    (2, re.compile(r"^[（(]\s*[一二三四五六七八九十百]+\s*[）)]\s*")),
    (3, re.compile(r"^[0-9]+[.．、]\s*")),
    (4, re.compile(r"^[（(][0-9]+[）)]\s*")),
)


def _official(document, blocks, result, options):
    mode = options.get("numbering", "keep")
    if mode not in ("keep", "remove", "rebuild"):
        raise SchemeInputError("numbering must be keep, remove, or rebuild")
    selected = options.get("selectedStructureIds", [])
    if not isinstance(selected, list) or any(not isinstance(item, str) for item in selected):
        raise SchemeInputError("selected structure IDs must be strings")
    if len(set(selected)) != len(selected):
        raise SchemeInputError("selected structure IDs must be unique")
    result["fields"] = {
        "issuingAgency": _field(_line_values(blocks, "发文机关")),
        "documentNumber": _field(_line_values(blocks, "发文字号|文号")),
        "issueDate": _field(_line_values(blocks, "成文日期|发文日期", _date)),
        "mainRecipient": _field(_line_values(blocks, "主送(?:机关)?")),
    }
    # Typical printed documents omit labels. These remain sourced proposals,
    # without deriving an agency from a title or filling a missing date.
    fallback = {key: [] for key in result["fields"]}
    first_heading = next((i for i, b in enumerate(blocks) if any(pattern.match(b["text"]) for _, pattern in _PREFIXES)), len(blocks))
    for index, block in enumerate(blocks):
        text = block["text"].strip()
        start = len(block["text"]) - len(block["text"].lstrip())
        end = start + len(text)
        if re.fullmatch(r"[^\s：:]{1,20}〔[0-9]{4}〕[0-9]+\s*号", text):
            fallback["documentNumber"].append(_candidate(block, start, end, proof="unlabeled-document-number-pattern"))
        if _date(text)[1] is None:
            fallback["issueDate"].append(_candidate(block, start, end, _date, proof="unlabeled-complete-date-pattern"))
        if index >= max(1, len(blocks) - 5) and re.fullmatch(r"[^：:。；;\r\n]{2,50}(?:人民政府|委员会|办公室|局|厅|部|院)", text):
            fallback["issuingAgency"].append(_candidate(block, start, end, proof="signature-name-pattern"))
        if index < first_heading and re.fullmatch(r"(?:各|有关)[^：:。；;\r\n]{1,80}[：:]", text):
            fallback["mainRecipient"].append(_candidate(block, start, end - 1, proof="unlabeled-recipient-pattern"))
    for key, candidates in fallback.items():
        if result["fields"][key]["status"] == "missing" and candidates:
            result["fields"][key] = _field(candidates)
    for key, field in result["fields"].items():
        field["valueKind"] = "date" if key == "issueDate" else "text"
    result["numbering"] = {"choice": mode, "selectedStructureIds": selected}
    result["structures"] = []
    for index, block in enumerate(blocks):
        text = block["text"]
        if not text.strip():
            continue
        style = block.get("style")
        level = block.get("outlineLevel")
        numbering = block.get("numbering")
        docx_heading = block["sourceMethod"] == "docx" and isinstance(level, int) and not isinstance(level, bool) and 1 <= level <= 9
        native_list = block["sourceMethod"] == "docx" and isinstance(numbering, dict) and numbering.get("proven") is True
        prefix = next(((depth, match) for depth, pattern in _PREFIXES if (match := pattern.match(text))), None)
        is_title = block["sourceMethod"] == "docx" and style in ("Title", "标题")
        if not (docx_heading or native_list or prefix or is_title):
            if index < 3 and re.search(r"(?:通知|通报|意见|决定|报告|函|公告)$", text.strip()):
                result["structures"].append({
                    "id": f"structure:{block['id']}", "kind": "title-candidate", "raw": text,
                    "content": text.strip(), "sources": [_source(block)], "proof": "text-pattern",
                    "confirmed": False,
                })
            continue
        kind = "title" if is_title else "heading" if docx_heading else "list" if native_list else "heading-candidate"
        proof = "docx-style" if is_title or docx_heading else "docx-numbering" if native_list else "text-pattern"
        ident = f"structure:{block['id']}"
        # Native Word counters are separate from paragraph text. A literal
        # numeric body prefix is never proven redundant by native numbering.
        removable_prefix = prefix if block["sourceMethod"] != "docx" else None
        content = text[removable_prefix[1].end():] if removable_prefix else text
        proposal = {
            "id": ident, "kind": kind, "raw": text, "content": content,
            "sources": [_source(block)], "proof": proof, "confirmed": False,
            "level": level if docx_heading else (numbering.get("level") if native_list else prefix[0] if prefix else None),
            "visiblePrefix": prefix[1].group(0) if prefix else None,
            "selected": ident in selected,
            "contentSource": _source(block, removable_prefix[1].end() if removable_prefix else 0),
        }
        if native_list:
            proposal["nativeNumbering"] = copy.deepcopy(numbering)
        if ident in selected:
            proposal["numberingChoice"] = mode
            if native_list:
                proposal["nativeNumberingAction"] = {"keep": "preserve", "remove": "omit", "rebuild": "replace"}[mode]
            if mode == "remove":
                proposal["proposedText"] = content
            elif mode == "keep":
                proposal["proposedText"] = text
            else:
                # The final authoring adapter applies native numbering once.
                proposal["proposedText"] = content
                proposal["nativeNumberingRequired"] = True
        if prefix and block["sourceMethod"] == "docx":
            proposal["literalPrefixPreserved"] = True
        result["structures"].append(proposal)
    known = {item["id"] for item in result["structures"]}
    if set(selected) - known:
        raise SchemeInputError("selected structure is not in this exact evidence")
    result["issues"].append({"code": "fields-and-text-pattern-structures-require-confirmation", "severity": "review-required"})
    if any(_TITLE.search(block["text"]) for block in blocks):
        result.update(applicability="not-applicable", reason="invoice-is-not-an-official-document")
    elif not result["structures"] and all(field["status"] == "missing" for field in result["fields"].values()):
        result.update(applicability="not-applicable", reason="official-document-markers-not-found")
    elif all(field["status"] == "missing" for field in result["fields"].values()) and not any(
            item["kind"] in ("title", "title-candidate") for item in result["structures"]):
        result.update(applicability="unknown", reason="numbered-text-alone-does-not-establish-official-document-type")
    return result


def propose(scheme_id: str, document: dict[str, Any], options: dict[str, Any]) -> dict[str, Any]:
    """Return unconfirmed proposals; never mutate ``document`` or ``options``.

    ``options.language/script/region`` are explicit operation settings. Missing
    or different values do not activate a region-specific rule. ``document`` is
    the runtime's original result with pages -> regions -> blocks, or a DOCX
    adapter result containing docxBlocks with actual structure provenance.
    """
    if scheme_id not in SCHEMES:
        raise SchemeInputError("unknown recognition scheme")
    if not isinstance(document, dict) or not isinstance(options, dict):
        raise SchemeInputError("scheme input and options must be objects")
    scheme = SCHEMES[scheme_id]
    result = {
        "scheme": copy.deepcopy(scheme), "applicability": "supported", "fields": {},
        "issues": [], "confirmed": False,
    }
    actual = {key: options.get(key) for key in ("language", "script", "region")}
    result["processing"] = actual
    if any(value is None for value in actual.values()):
        result.update(applicability="unknown", reason="explicit-language-script-region-required")
        return result
    if any(actual[key] != scheme[key] for key in actual):
        result.update(applicability="not-applicable", reason="processing-scope-does-not-match")
        return result
    input_meta = document.get("input", {})
    if not isinstance(input_meta, dict) or input_meta.get("mediaType") not in scheme["mediaTypes"]:
        result.update(applicability="unsupported", reason="input-format-not-supported")
        return result
    if document.get("status") == "failed":
        result.update(applicability="unavailable", reason="recognition-failed")
        return result
    blocks = _blocks(document)
    result["input"] = {key: input_meta[key] for key in ("sha256", "mediaType", "bytes") if key in input_meta}
    result["sourceStatus"] = document.get("status", "unknown")
    if document.get("status") != "complete":
        result["issues"].append({"code": "source-coverage-not-complete", "severity": "review-required"})
    if scheme_id == INVOICE:
        return _invoice(document, blocks, result)
    return _official(document, blocks, result, options)
