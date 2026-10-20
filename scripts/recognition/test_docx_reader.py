"""Real package reader and exact numbering-source mapping checks."""
import hashlib
import io
from pathlib import Path
import tempfile
import unittest
import zipfile

from build_scheme_fixtures import official_docx
from docx_reader import DocxEvidenceError, read_docx
from schemes import OFFICIAL, propose

SETTINGS = {"language": "zh", "script": "Hans", "region": "CN"}


def fixture():
    with tempfile.TemporaryDirectory() as name:
        path = Path(name) / "TEST.docx"
        official_docx(path)
        return path.read_bytes()


def replace(data, part, old, new):
    output = io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(data)) as source, zipfile.ZipFile(output, "w") as target:
        for item in source.infolist():
            raw = source.read(item.filename)
            target.writestr(item, raw.replace(old.encode(), new.encode()) if item.filename == part else raw)
    return output.getvalue()


class DocxReaderTests(unittest.TestCase):
    def test_reads_real_styles_numbering_and_source_not_plain_text_claims(self):
        data = fixture()
        original = read_docx(data, hashlib.sha256(data).hexdigest())
        self.assertEqual(original["status"], "complete")
        self.assertEqual(original["input"]["bytes"], len(data))
        self.assertEqual(original["docxBlocks"][0]["style"], "Title")
        self.assertEqual(original["docxBlocks"][5]["outlineLevel"], 1)
        self.assertEqual(original["docxBlocks"][6]["outlineLevel"], 2)
        self.assertTrue(original["docxBlocks"][5]["numbering"]["proven"])
        self.assertEqual(original["docxBlocks"][5]["numbering"]["pattern"], "%1、")
        self.assertNotIn("numbering", original["docxBlocks"][7])
        fields = propose(OFFICIAL, original, SETTINGS)["fields"]
        self.assertEqual(fields["issuingAgency"]["value"], "TEST测试局")
        self.assertEqual(fields["issueDate"]["value"], "2026-10-18")

    def test_keep_remove_rebuild_native_counter_and_literal_prefix_mapping(self):
        data = fixture()
        data = replace(data, "word/document.xml", "<w:t>测试事项</w:t>", "<w:t>2026. 测试事项</w:t>")
        source = read_docx(data)
        for choice, action in (("keep", "preserve"), ("remove", "omit"), ("rebuild", "replace")):
            result = propose(OFFICIAL, source, {**SETTINGS, "numbering": choice,
                "selectedStructureIds": ["structure:docx:p:5"]})
            item = next(item for item in result["structures"] if item["id"] == "structure:docx:p:5")
            self.assertEqual(item["nativeNumberingAction"], action)
            self.assertEqual(item["proposedText"], "2026. 测试事项")
            self.assertTrue(item["literalPrefixPreserved"])
            self.assertEqual(item["contentSource"], {"blockId": "docx:p:5", "quote": "2026. 测试事项",
                "start": 0, "end": 10, "offsetUnit": "unicode-code-point", "sourceMethod": "docx",
                "blockIndex": 5, "part": "word/document.xml", "paragraphIndex": 5})
            unselected = next(item for item in result["structures"] if item["id"] == "structure:docx:p:7")
            self.assertNotIn("proposedText", unselected)
        self.assertEqual(source["docxBlocks"][5]["text"], "2026. 测试事项")

    def test_unresolved_numbering_does_not_prove_native_list(self):
        data = replace(fixture(), "word/document.xml", 'w:numId w:val="5"', 'w:numId w:val="99"')
        original = read_docx(data)
        self.assertEqual(original["status"], "partial")
        self.assertFalse(original["docxBlocks"][5]["numbering"]["proven"])
        self.assertFalse(any(item["proof"] == "docx-numbering" for item in propose(OFFICIAL, original, SETTINGS)["structures"]))

    def test_digest_malformed_package_and_entities_are_rejected(self):
        with self.assertRaises(DocxEvidenceError):
            read_docx(fixture(), "0" * 64)
        with self.assertRaises(DocxEvidenceError):
            read_docx(b"not a package")
        data = replace(fixture(), "word/document.xml", "<w:document", '<!DOCTYPE a [<!ENTITY x "bad">]><w:document')
        with self.assertRaises(DocxEvidenceError):
            read_docx(data)
        data = replace(fixture(), "word/document.xml", "<w:body>", "<w:body>" + "<w:sdt>" * 130)
        data = replace(data, "word/document.xml", "</w:body>", "</w:sdt>" * 130 + "</w:body>")
        with self.assertRaisesRegex(DocxEvidenceError, "complexity budget"):
            read_docx(data)

    def test_revisions_are_disclosed_not_silently_accepted(self):
        data = replace(fixture(), "word/document.xml", "<w:t>子事项</w:t>", '<w:ins><w:r><w:t>新增</w:t></w:r></w:ins>')
        original = read_docx(data)
        self.assertEqual(original["status"], "partial")
        self.assertIn("ins", original["issues"][-1]["elements"])

    def test_unlinked_or_external_numbering_part_never_proves_a_counter(self):
        data = replace(fixture(), "word/_rels/document.xml.rels", 'Target="numbering.xml"', 'Target="numbering.xml" TargetMode="External"')
        with self.assertRaises(DocxEvidenceError):
            read_docx(data)
        data = replace(fixture(), "word/_rels/document.xml.rels", '/relationships/numbering"', '/relationships/unknown"')
        original = read_docx(data)
        self.assertEqual(original["status"], "partial")
        self.assertFalse(original["docxBlocks"][5]["numbering"]["proven"])


if __name__ == "__main__":
    unittest.main()
