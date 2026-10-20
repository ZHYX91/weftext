"""Protocol rejection, source geometry and real descendant cancellation checks."""
from __future__ import annotations

import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from PIL import Image
from engine import inverse_rotation, mapped_box, ocr_region, order_candidates, text_region
from worker import ChildJob, Refusal, checked_json, validate_request


class BoundaryTests(unittest.TestCase):
    def request(self):
        return {"version": 1, "inputPath": str(Path(__file__).resolve()), "inputSha256": "a" * 64, "mediaType": "application/pdf", "languages": ["chi_sim", "eng"], "pages": [{"page": 1}]}

    def test_strict_selection_rejects_duplicate_unknown_nonfinite_and_overlap(self):
        variants = [{"version": True}, {"operation": []}, {"mediaType": {}}, {"unrecognized": "ignored"}, {"pages": [{"page": 1}, {"page": 1}]}, {"pages": [{"page": 1}] * 101},
                    {"pages": [{"page": 1, "regions": [{"id": "r", "rect": [0, 0, float("nan"), 20]}]}]},
                    {"pages": [{"page": 1, "regions": [{"id": "r", "rect": [0, 0, 20, 20], "segmentation": "custom-config"}]}]}]
        for variant in variants:
            with self.subTest(variant=variant), self.assertRaises(Refusal):
                validate_request({**self.request(), **variant})

    def test_clockwise_ocr_boxes_return_to_original_cropped_region(self):
        # A real 120x80 source crop, rotated clockwise. Its upper-left 20x10
        # rotated area came from the original lower-left 10x20 area.
        self.assertEqual(mapped_box([0, 0, 20, 10], inverse_rotation(90, 120, 80)), [0., 60., 10., 80.])
        self.assertEqual(mapped_box([0, 0, 20, 10], inverse_rotation(180, 120, 80)), [100., 70., 120., 80.])
        self.assertEqual(mapped_box([0, 0, 20, 10], inverse_rotation(270, 120, 80)), [110., 0., 120., 20.])

    def test_reading_order_retains_varied_font_sizes_and_all_blocks(self):
        blocks = [{"id": "body", "text": "earlier body", "bbox": [10, 20, 400, 30]},
                  {"id": "heading", "text": "later large heading", "bbox": [10, 80, 400, 120]}]
        self.assertEqual(order_candidates(blocks, 600, 800)["suggested"], ["body", "heading"])

    def test_inspect_selection_and_preview_operation_contract(self):
        request = self.request()
        request.pop("pages")
        self.assertEqual(validate_request({**request, "operation": "inspect"})["pages"], [{"page": 1}])
        with self.assertRaisesRegex(Refusal, "inspect-selection"):
            validate_request({**self.request(), "operation": "inspect"})
        with self.assertRaisesRegex(Refusal, "unsupported-operation"):
            validate_request({**self.request(), "operation": "execute-command"})

    def test_duplicate_json_keys_rejected_before_request_validation(self):
        with self.assertRaisesRegex(Refusal, "invalid-json"):
            checked_json(b'{"version":1,"version":2}')

    def test_geometric_column_split_preserves_words(self):
        characters = [{"text": value, "bbox": box} for value, box in [("A", [10, 10, 20, 20]), (" ", [20, 10, 200, 20]), ("B", [200, 10, 210, 20])]]
        result = text_region(characters, {"id": "test", "rect": [0, 0, 600, 400]}, 1)
        self.assertEqual([block["text"] for block in result["blocks"]], ["A", "B"])

    def test_actual_ocr_like_diagnostic_flood_is_stopped(self):
        original_popen = subprocess.Popen
        original_temporary_directory = tempfile.TemporaryDirectory
        processes, locks = [], []
        def flood(_command, **options):
            process = original_popen([sys.executable, "-c", "import os,time;os.write(2,b'x'*300000);time.sleep(20)"], **options)
            processes.append(process)
            # CRT file handles deny deletion on Windows. Keep an actual reader
            # open so cleanup must preserve the OCR budget exception even when
            # an indexer/AV-like lock retains the private diagnostic.
            locks.append(open(options["stderr"].name, "rb"))
            return process
        with original_temporary_directory(prefix="weftext-flood-test-", ignore_cleanup_errors=True) as directory:
            def private_temporary_directory(*args, **options):
                return original_temporary_directory(*args, **{**options, "dir": directory})
            try:
                with Image.new("RGB", (100, 50), "white") as image, patch("engine.subprocess.Popen", side_effect=flood), patch("engine.tempfile.TemporaryDirectory", side_effect=private_temporary_directory):
                    started = time.monotonic()
                    with self.assertRaisesRegex(ValueError, "ocr-output-budget"):
                        ocr_region(image, {"id": "test", "rect": [0, 0, 100, 50]}, 1, Path(__file__).parent, ["eng"], lambda x, y: (x, y))
                    self.assertLess(time.monotonic() - started, 5)
                    self.assertTrue(all(process.poll() is not None for process in processes))
                    if os.name == "nt":
                        self.assertTrue(any(Path(directory).glob("ocr-*/diagnostic")), "locked private diagnostic must be retained")
            finally:
                for lock in locks:
                    lock.close()

    @unittest.skipUnless(os.name == "nt", "Windows process Job contract")
    def test_job_close_kills_actual_ocr_like_grandchild(self):
        with tempfile.TemporaryDirectory(prefix="weftext-job-test-") as directory:
            root = Path(directory)
            ready, unwanted = root / "ready", root / "late-write"
            grandchild = "import pathlib,time;time.sleep(2);pathlib.Path(" + repr(str(unwanted)) + ").write_text('late')"
            child = "import subprocess,sys,pathlib,time; subprocess.Popen([sys.executable,'-c'," + repr(grandchild) + "]); pathlib.Path(" + repr(str(ready)) + ").write_text('ready'); time.sleep(20)"
            process = subprocess.Popen([sys.executable, "-c", child], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, creationflags=0x08000004)
            job = ChildJob(process, suspended=True)
            try:
                deadline = time.monotonic() + 5
                while not ready.exists() and time.monotonic() < deadline:
                    time.sleep(.02)
                self.assertTrue(ready.exists(), "child failed before creating descendant")
            finally:
                job.close()
            time.sleep(2.2)
            self.assertFalse(unwanted.exists(), "descendant survived job cancellation")


if __name__ == "__main__":
    unittest.main()
