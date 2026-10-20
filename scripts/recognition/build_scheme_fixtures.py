"""Build owned public TEST fixtures; never download or borrow user examples.

Raster fixtures require an explicitly selected official Typst CLI and Pillow;
the OOXML fixture uses only the standard library. Font bytes come from this
repository's licensed Noto assets and are not copied into fixtures.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import zipfile

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def official_docx(path):
    """A real OOXML structure fixture, not OCR text mislabeled as DOCX proof."""
    parts = {
        "[Content_Types].xml": '''<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/></Types>''',
        "_rels/.rels": '''<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>''',
        "word/_rels/document.xml.rels": '''<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/></Relationships>''',
        "word/styles.xml": '''<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults><w:rPrDefault><w:rPr><w:lang w:val="zh-CN" w:eastAsia="zh-CN"/></w:rPr></w:rPrDefault></w:docDefaults><w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/></w:style><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:style><w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:pPr><w:outlineLvl w:val="1"/></w:pPr></w:style></w:styles>''',
        "word/numbering.xml": '''<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:abstractNum w:abstractNumId="1"><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="chineseCounting"/><w:lvlText w:val="%1、"/></w:lvl><w:lvl w:ilvl="1"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%2."/></w:lvl></w:abstractNum><w:num w:numId="5"><w:abstractNumId w:val="1"/></w:num></w:numbering>''',
        "word/document.xml": '''<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>
<w:p><w:pPr><w:pStyle w:val="Title"/></w:pPr><w:r><w:t>TEST测试局关于测试事项的通知</w:t></w:r></w:p>
<w:p><w:r><w:t>发文机关：TEST测试局</w:t></w:r></w:p>
<w:p><w:r><w:t>发文字号：测字〔2026〕1号</w:t></w:r></w:p>
<w:p><w:r><w:t>成文日期：2026年10月18日</w:t></w:r></w:p>
<w:p><w:r><w:t>主送机关：TEST各测试单位</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Heading1"/><w:numPr><w:ilvl w:val="0"/><w:numId w:val="5"/></w:numPr></w:pPr><w:r><w:t>测试事项</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Heading2"/><w:numPr><w:ilvl w:val="1"/><w:numId w:val="5"/></w:numPr></w:pPr><w:r><w:t>子事项</w:t></w:r></w:p>
<w:p><w:r><w:t>1. 普通数字段首，未经确认不能算原生列表。</w:t></w:r></w:p>
<w:p><w:r><w:t>TEST ONLY — 合成测试，非正式公文</w:t></w:r></w:p>
</w:body></w:document>''',
    }
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        for name, content in parts.items():
            info = zipfile.ZipInfo(name, date_time=(2026, 10, 18, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, content.encode("utf-8"))


def build(typst: Path, output: Path):
    from PIL import Image

    root = Path(__file__).resolve().parents[2]
    fixtures = Path(__file__).resolve().parent / "fixtures"
    output.mkdir(parents=True, exist_ok=True)
    manifest = {"version": 1, "kind": "self-authored-public-TEST-fixtures", "files": [],
                "invoiceExpected": {"invoiceNumber": "00123456789012345678", "issueDate": "2026-10-18",
                    "buyerName": "TEST BUYER LTD", "buyerTaxId": "001234567890123456",
                    "sellerName": "TEST SELLER LTD", "sellerTaxId": "991234567890123456",
                    "amount": "1000.00", "tax": "130.00", "total": "1130.00", "currency": "CNY"}}
    invoice_text = (fixtures / "invoice-test.typ").read_text(encoding="utf-8")
    variants = {
        "invoice-wrong-title": (invoice_text.replace("电子发票（普通发票）", "电子发票（增值税专用发票）"), {"applicability": "not-applicable"}),
        "invoice-missing-date": (invoice_text.replace('#at(590pt, 94pt)[开票日期：2026年10月18日]', ''), {"fieldStatus": {"issueDate": "missing"}}),
        "invoice-ambiguous-date": (invoice_text.replace("2026年10月18日", "01/02/2026"), {"fieldStatus": {"issueDate": "parse-failure"}}),
        "invoice-invalid-date": (invoice_text.replace("2026年10月18日", "2026年02月30日"), {"fieldStatus": {"issueDate": "parse-failure"}}),
        "invoice-arithmetic": (invoice_text.replace("1,130.00", "1,131.00"), {"issue": "amount-plus-tax-does-not-equal-total", "fieldValue": {"total": "1131.00"}}),
        "invoice-duplicate-number": (invoice_text + '\n#at(25pt, 68pt)[发票号码：00123456789012345678]\n', {"fieldStatus": {"invoiceNumber": "duplicate"}}),
        "invoice-competing-number": (invoice_text + '\n#at(25pt, 68pt)[发票号码：99123456789012345678]\n', {"applicability": "unsupported", "reason": "competing-invoice-numbers-not-grouped"}),
        "invoice-multiple-title": (invoice_text + '\n#at(25pt, 50pt)[电子发票（普通发票）]\n', {"applicability": "unsupported", "reason": "multiple-invoices-not-grouped"}),
    }
    manifest["qualificationCases"] = []
    for name in ("invoice-test", "official-test", *variants):
        source = fixtures / f"{name}.typ"
        if name in variants:
            source = output / f"{name}.typ"
            source.write_text(variants[name][0], encoding="utf-8")
            manifest["qualificationCases"].append({"name": name, "scheme": "invoice", "expect": variants[name][1], "formats": ["pdf", "png", "jpeg"]})
        pdf, png = output / f"{name}.pdf", output / f"{name}.png"
        arguments = [str(typst), "compile", "--font-path", str(root / "resources" / "fonts"), str(source)]
        subprocess.run([*arguments, str(pdf)], check=True, stdin=subprocess.DEVNULL)
        subprocess.run([*arguments, "--format", "png", "--ppi", "300", str(png)], check=True, stdin=subprocess.DEVNULL)
        with Image.open(png) as image:
            image.convert("RGB").save(output / f"{name}.jpeg", format="JPEG", quality=96, subsampling=0)
        manifest["files"].append({"source": source.name, "sourceSha256": sha(source),
            "outputs": [{"name": path.name, "sha256": sha(path), "bytes": path.stat().st_size}
                        for path in (pdf, png, output / f"{name}.jpeg")]})
    docx = output / "official-test.docx"
    official_docx(docx)
    manifest["files"].append({"name": docx.name, "sha256": sha(docx), "bytes": docx.stat().st_size})
    (output / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return manifest


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--typst", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(build(args.typst, args.output_dir), ensure_ascii=False))
