"""Real offline worker -> scheme fixture qualification, separate from HTTP/GUI."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

from schemes import INVOICE, OFFICIAL, propose
from docx_reader import read_docx


def exact_sources(document, proposal):
    """Independently check every quoted range against retained original blocks."""
    blocks = {block["id"]: block for page in document.get("pages", [])
              for region in page.get("regions", []) for block in region.get("blocks", [])}
    blocks.update({block["id"]: block for block in document.get("docxBlocks", [])})
    sources = []

    def visit(value):
        if isinstance(value, dict):
            if {"blockId", "quote", "start", "end"} <= set(value):
                sources.append(value)
            for child in value.values():
                visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)

    visit(proposal)
    return all(source["blockId"] in blocks and
               blocks[source["blockId"]]["text"][source["start"]:source["end"]] == source["quote"]
               and source["offsetUnit"] == "unicode-code-point" for source in sources)


def expected_checks(proposal, expected):
    checks = {}
    for key in ("applicability", "reason"):
        if key in expected:
            checks[key] = proposal.get(key) == expected[key]
    for key, value in expected.get("fieldStatus", {}).items():
        checks[key + "Status"] = proposal.get("fields", {}).get(key, {}).get("status") == value
    for key, value in expected.get("fieldValue", {}).items():
        checks[key + "Value"] = proposal.get("fields", {}).get(key, {}).get("value") == value
    if "issue" in expected:
        checks["reviewIssue"] = any(item.get("code") == expected["issue"] for item in proposal.get("issues", []))
    return checks


def run(runtime: Path, fixtures: Path, evidence: Path, selections: dict | None = None,
        formats: list[str] | None = None, names: list[str] | None = None):
    runtime, fixtures, evidence = runtime.resolve(), fixtures.resolve(), evidence.resolve()
    evidence.mkdir(parents=True, exist_ok=True)
    manifest = json.loads((fixtures / "manifest.json").read_text(encoding="utf-8"))
    expected = manifest["invoiceExpected"]
    by_name = {item["name"]: item for group in manifest["files"] for item in group.get("outputs", [])}
    python = runtime / "venv" / "Scripts" / "python.exe"
    worker = Path(__file__).resolve().parent / "worker.py"
    results = []
    cases = [{"name": "invoice-test", "scheme": "invoice"}, {"name": "official-test", "scheme": "official"}, *manifest.get("qualificationCases", [])]
    for case in cases:
        name, scheme = case["name"], INVOICE if case["scheme"] == "invoice" else OFFICIAL
        if names is not None and name not in names:
            continue
        for extension, media in (("pdf", "application/pdf"), ("png", "image/png"), ("jpeg", "image/jpeg")):
            if formats is not None and extension not in formats:
                continue
            if extension not in case.get("formats", ["pdf", "png", "jpeg"]):
                continue
            source = fixtures / f"{name}.{extension}"
            digest = hashlib.sha256(source.read_bytes()).hexdigest()
            if digest != by_name[source.name]["sha256"]:
                raise ValueError("synthetic fixture changed after generation")
            request = {"version": 1, "inputPath": str(source.resolve()), "inputSha256": digest,
                       "mediaType": media, "languages": ["chi_sim", "eng"], "pages": [{"page": 1}], "dpi": 300}
            if selections and source.name in selections:
                request["pages"] = selections[source.name]
            process = subprocess.run([str(python), "-I", str(worker), "--runtime", str(runtime)],
                input=json.dumps(request).encode("utf-8"), stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                timeout=90, check=True)
            recognized = json.loads(process.stdout)
            proposal = propose(scheme, recognized, {"language": "zh", "script": "Hans", "region": "CN"})
            checks = expected_checks(proposal, case["expect"]) if "expect" in case else {key: proposal.get("fields", {}).get(key, {}).get("value") == value for key, value in expected.items()} if scheme == INVOICE else {
                "issuingAgency": proposal.get("fields", {}).get("issuingAgency", {}).get("value") == "TEST测试局",
                "issueDate": proposal.get("fields", {}).get("issueDate", {}).get("value") == "2026-10-18",
                "numberedCandidates": len([item for item in proposal.get("structures", []) if item.get("visiblePrefix")]) == 3,
            }
            checks.update(exactSources=exact_sources(recognized, proposal),
                          exactOriginal=recognized.get("input", {}).get("sha256") == digest,
                          unconfirmed=proposal.get("confirmed") is False and all(field.get("confirmed") is False for field in proposal.get("fields", {}).values()))
            (evidence / f"{name}-{extension}-request.json").write_text(json.dumps(request, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            for suffix, value in (("recognized", recognized), ("proposal", proposal)):
                (evidence / f"{name}-{extension}-{suffix}.json").write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            results.append({"input": source.name, "sha256": digest, "recognitionStatus": recognized.get("status"),
                            "schemeApplicability": proposal["applicability"], "checks": checks,
                            "passed": recognized.get("status") == "complete" and ("expect" in case or proposal["applicability"] == "supported") and all(checks.values())})
    docx = fixtures / "official-test.docx"
    if docx.is_file() and (formats is None or "docx" in formats) and (names is None or "official-test" in names):
        digest = hashlib.sha256(docx.read_bytes()).hexdigest()
        declared = next(item["sha256"] for item in manifest["files"] if item.get("name") == docx.name)
        if digest != declared:
            raise ValueError("DOCX fixture changed after generation")
        recognized = read_docx(docx, digest)
        checks = {}
        proposals = {}
        for choice, action in (("keep", "preserve"), ("remove", "omit"), ("rebuild", "replace")):
            proposal = propose(OFFICIAL, recognized, {"language": "zh", "script": "Hans", "region": "CN", "numbering": choice,
                "selectedStructureIds": ["structure:docx:p:5", "structure:docx:p:6"]})
            proposals[choice] = proposal
            selected = [item for item in proposal["structures"] if item.get("selected")]
            checks[choice] = len(selected) == 2 and all(item.get("nativeNumberingAction") == action and
                item["proposedText"] == recognized["docxBlocks"][item["sources"][0]["blockIndex"]]["text"] for item in selected) and exact_sources(recognized, proposal)
        (evidence / "official-test-docx-recognized.json").write_text(json.dumps(recognized, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        (evidence / "official-test-docx-proposals.json").write_text(json.dumps(proposals, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        results.append({"input": docx.name, "sha256": digest, "recognitionStatus": recognized["status"], "checks": checks, "passed": recognized["status"] == "complete" and all(checks.values())})
    summary = {"version": 1, "scope": "real-offline-worker-and-pure-scheme-component-not-product-acceptance",
               "results": results, "passed": all(item["passed"] for item in results)}
    (evidence / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return summary


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--runtime", type=Path, required=True)
    parser.add_argument("--fixtures-dir", type=Path, required=True)
    parser.add_argument("--evidence-dir", type=Path, required=True)
    parser.add_argument("--selections", type=Path, help="Explicit original-page region selections keyed by fixture filename")
    parser.add_argument("--formats", nargs="+", choices=["pdf", "png", "jpeg", "docx"])
    parser.add_argument("--names", nargs="+", help="Bound a rerun to exact manifest case names")
    args = parser.parse_args()
    summary = run(args.runtime, args.fixtures_dir, args.evidence_dir,
                  json.loads(args.selections.read_text(encoding="utf-8")) if args.selections else None,
                  args.formats, args.names)
    print(json.dumps(summary, ensure_ascii=False))
    sys.exit(0 if summary["passed"] else 1)
