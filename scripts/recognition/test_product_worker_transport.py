"""Private request framing and real DOCX subprocess checks; no OCR backend."""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest
import zipfile

import product_worker
import worker

LIMIT = 256 * 1024
RUNTIME = None
EVIDENCE = None


def frame(payload, *, magic=b"WFRQ", version=1, length=None):
    return struct.pack("<4sII", magic, version, len(payload) if length is None else length) + payload


class ShortReads(io.BytesIO):
    def read(self, size=-1):
        return super().read(min(size, 3) if size >= 0 else 3)


class RequestFrameTests(unittest.TestCase):
    def refused(self, data, code="invalid-request-frame"):
        with self.assertRaises(worker.Refusal) as caught:
            product_worker.read_request(io.BytesIO(data))
        self.assertEqual(caught.exception.code, code)

    def test_complete_frame_needs_neither_eof_nor_a_full_read(self):
        payload = b'{"request":{"text":"TEST"}}'
        stream = ShortReads(frame(payload) + b"unconsumed-next-bytes")
        self.assertEqual(product_worker.read_request(stream), {"request": {"text": "TEST"}})
        self.assertEqual(stream.tell(), 12 + len(payload))

    def test_invalid_header_is_rejected_before_reading_a_body(self):
        for magic, version, length in ((b"NOPE", 1, 1), (b"WFRQ", 2, 1),
                                       (b"WFRQ", 1, 0), (b"WFRQ", 1, LIMIT + 1)):
            with self.subTest(magic=magic, version=version, length=length):
                class HeaderOnly(io.BytesIO):
                    def read(self, size=-1):
                        if self.tell() >= 12:
                            raise AssertionError("Invalid header must not request a body")
                        return super().read(size)
                with self.assertRaises(worker.Refusal) as caught:
                    product_worker.read_request(HeaderOnly(struct.pack("<4sII", magic, version, length)))
                self.assertEqual(caught.exception.code, "invalid-request-frame")

    def test_every_truncated_header_and_a_truncated_body_are_rejected(self):
        valid = frame(b'{"request":{}}')
        for length in range(12):
            with self.subTest(headerBytes=length):
                self.refused(valid[:length])
        self.refused(valid[:-1])

    def test_payload_limit_is_inclusive(self):
        payload = b'{"padding":"' + b"x" * (LIMIT - len(b'{"padding":""}')) + b'"}'
        self.assertEqual(len(payload), LIMIT)
        self.assertEqual(len(product_worker.read_request(io.BytesIO(frame(payload)))["padding"]),
                         LIMIT - len(b'{"padding":""}'))

    def test_malformed_json_keeps_its_existing_error(self):
        for payload in (b"{", b"\xff", b'{"value":1,"value":2}'):
            with self.subTest(payload=payload):
                self.refused(frame(payload), "invalid-json")


def docx(path):
    with zipfile.ZipFile(path, "x", zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("[Content_Types].xml", '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>')
        archive.writestr("_rels/.rels", '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>')
        archive.writestr("word/document.xml", '<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>TEST FRAME TRANSPORT</w:t></w:r></w:p></w:body></w:document>')


class ProductSubprocessTests(unittest.TestCase):
    def setUp(self):
        self.temporary = None
        if EVIDENCE is None:
            self.temporary = tempfile.TemporaryDirectory(prefix="weftext-product-frame-")
            self.directory = Path(self.temporary.name)
        else:
            self.directory = EVIDENCE / self._testMethodName
            self.directory.mkdir()
        self.original = self.directory / "TEST-transport.docx"
        docx(self.original)
        self.original_bytes = self.original.read_bytes()
        self.runtime = RUNTIME if RUNTIME is not None else self.directory

    def tearDown(self):
        if self.temporary is not None:
            self.temporary.cleanup()

    def invoke(self, data, *, keep_stdin_open, name):
        command = [sys.executable, "-I", "-B", str(Path(product_worker.__file__).resolve()),
                   "--runtime", str(self.runtime)]
        output = self.directory / (name + "-stdout.json")
        errors = self.directory / (name + "-stderr.txt")
        (self.directory / (name + "-input.bin")).write_bytes(data)
        system_root = os.environ.get("SystemRoot", r"C:\Windows")
        environment = {"SystemRoot": system_root, "TEMP": str(self.directory),
                       "TMP": str(self.directory), "PYTHONUTF8": "1"}
        if os.name != "nt":
            environment["PATH"] = os.defpath
        job = None
        with output.open("xb") as stdout, errors.open("xb") as stderr:
            child = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=stdout, stderr=stderr,
                                     cwd=self.directory, env=environment,
                                     creationflags=0x08000004 if os.name == "nt" else 0)
            try:
                job = worker.ChildJob(child, suspended=os.name == "nt")
                child.stdin.write(data)
                child.stdin.flush()
                if not keep_stdin_open:
                    child.stdin.close()
                child.wait(timeout=5)
                self.assertEqual(child.stdin.closed, not keep_stdin_open)
                self.assertEqual(child.returncode, 0)
            finally:
                if job is not None:
                    job.close()
                elif child.poll() is None:
                    child.kill()
                    child.wait(timeout=5)
                if child.stdin is not None:
                    child.stdin.close()
        self.assertEqual(self.original.read_bytes(), self.original_bytes)
        self.assertEqual(errors.read_bytes(), b"")
        return json.loads(output.read_bytes())

    def test_docx_finishes_while_the_parent_keeps_stdin_open(self):
        envelope = {"format": "docx", "purpose": "note", "scheme": "none", "request": {
            "inputPath": str(self.original), "inputSha256": hashlib.sha256(self.original_bytes).hexdigest(),
            "mediaType": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "operation": "recognize"}}
        reply = self.invoke(frame(json.dumps(envelope, separators=(",", ":")).encode()),
                            keep_stdin_open=True, name="complete")
        self.assertEqual(reply["result"]["status"], "complete")
        self.assertEqual([item["text"] for item in reply["result"]["docxBlocks"]], ["TEST FRAME TRANSPORT"])
        self.assertIsNone(reply["scheme"])

    def test_invalid_and_oversized_headers_fail_without_waiting_for_eof(self):
        for name, data in (("magic", frame(b"", magic=b"NOPE", length=1)),
                           ("budget", frame(b"", length=LIMIT + 1))):
            with self.subTest(name=name):
                reply = self.invoke(data, keep_stdin_open=True, name=name)
                self.assertEqual(reply["result"]["status"], "failed")
                self.assertEqual(reply["result"]["error"], "invalid-request-frame")

    def test_truncated_body_fails_after_the_actual_input_eof(self):
        reply = self.invoke(frame(b"{", length=2), keep_stdin_open=False, name="truncated")
        self.assertEqual(reply["result"]["status"], "failed")
        self.assertEqual(reply["result"]["error"], "invalid-request-frame")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", type=Path)
    parser.add_argument("--evidence", type=Path)
    options, remaining = parser.parse_known_args()
    RUNTIME = options.runtime.resolve() if options.runtime is not None else None
    EVIDENCE = options.evidence.resolve() if options.evidence is not None else None
    if EVIDENCE is not None:
        EVIDENCE.mkdir(parents=True, exist_ok=False)
    unittest.main(argv=[sys.argv[0], *remaining])
