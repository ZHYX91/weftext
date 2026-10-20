"""Actual-page recovery checkpoints for an operator-selected private runtime."""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import pypdfium2 as pdfium

from test_worker import add_text, all_text, sha
from worker import execute


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runtime", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    original = args.evidence / "TEST-progress-two-pages.pdf"
    document, fonts = pdfium.PdfDocument.new(), []
    for number in (1, 2):
        page = document.new_page(600, 400)
        fonts.append(add_text(document, page, f"TEST PROGRESS PAGE {number}", 40, 340))
        page.gen_content()
    document.save(original)
    for font in fonts:
        pdfium.raw.FPDFFont_Close(font)
    document.close()
    source = original.read_bytes()
    request = {"version": 1, "inputPath": str(original.resolve()), "inputSha256": sha(source),
               "mediaType": "application/pdf", "languages": ["eng"], "pages": [{"page": 1}, {"page": 2}], "dpi": 72}
    snapshots = []
    def checkpoint(snapshot):
        snapshots.append(json.loads(json.dumps(snapshot)))
        (args.evidence / f"progress-{len(snapshots)}.json").write_text(json.dumps(snapshot, ensure_ascii=False, indent=2), encoding="utf-8")
        # Deliberately hostile in-process consumer: checkpoint state is isolated.
        snapshot["pages"].clear()
        snapshot["selection"]["pages"].clear()
        snapshot["runtime"]["models"].clear()
    result = execute(request, args.runtime, progress=checkpoint)
    (args.evidence / "complete.json").write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    assert len(snapshots) == 2
    assert snapshots[0]["status"] == "partial" and snapshots[0]["pageCount"] == 2
    assert [page["page"] for page in snapshots[0]["pages"]] == [1]
    assert snapshots[0]["selection"]["pages"] == [{"page": 1}, {"page": 2}]
    assert snapshots[1]["status"] == "complete" and len(snapshots[1]["pages"]) == 2
    assert result["status"] == "complete" and len(result["pages"]) == 2 and len(result["selection"]["pages"]) == 2
    assert "eng" in result["runtime"]["models"]
    assert "TEST PROGRESS PAGE 1" in all_text(result) and "TEST PROGRESS PAGE 2" in all_text(result)
    inspected = []
    inspection = execute({key: value for key, value in {**request, "operation": "inspect"}.items() if key != "pages"}, args.runtime, progress=inspected.append)
    (args.evidence / "inspect.json").write_text(json.dumps(inspection, indent=2), encoding="utf-8")
    assert len(inspected) == 1 and inspected[0]["pageCount"] == 2 and len(inspected[0]["pages"]) == 2 and inspected[0]["status"] == "complete"
    class Interrupted(Exception):
        pass
    def stop_after_first(snapshot):
        (args.evidence / "interrupted-checkpoint.json").write_text(json.dumps(snapshot, indent=2), encoding="utf-8")
        assert len(snapshot["pages"]) == 1 and snapshot["pageCount"] == 2 and snapshot["status"] == "partial"
        raise Interrupted()
    try:
        execute(request, args.runtime, progress=stop_after_first)
        raise AssertionError("callback interruption was swallowed")
    except Interrupted:
        pass
    assert original.read_bytes() == source and result["input"]["sha256"] == sha(source)
    summary = {"version": 1, "passed": True, "cases": ["two-page-checkpoints", "snapshot-mutation-isolation", "inspect-completion", "callback-interruption"],
               "implementation": result["implementation"], "scope": "Private worker progress callback; no host timeout/restart/product acceptance"}
    (args.evidence / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary))


if __name__ == "__main__":
    main()
