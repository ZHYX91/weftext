"""Bounded offline recognition process. Paths are operator inputs, never HTTP inputs.

Use the independently installed runtime's Python. One page is processed per child
process; PDFium is never called concurrently in threads. No job downloads tools.
"""
from __future__ import annotations

import argparse
from copy import deepcopy
import ctypes
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

MAX_INPUT = 16 * 1024 * 1024
MAX_OUTPUT = 32 * 1024 * 1024
MAX_REQUEST = 128 * 1024
MAX_PAGES = 100
PAGE_SECONDS = 60
MAX_PIXELS = 40_000_000
MEMORY_BYTES = 1024 * 1024 * 1024


class Refusal(Exception):
    def __init__(self, code: str):
        self.code = code
        super().__init__(code)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def checked_json(data: bytes):
    def unique_object(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate-json-key")
            result[key] = value
        return result
    try:
        return json.loads(data.decode("utf-8"), object_pairs_hook=unique_object, parse_constant=lambda _: (_ for _ in ()).throw(ValueError()))
    except (ValueError, UnicodeError, RecursionError) as exc:
        raise Refusal("invalid-json") from exc


def read_file(path: Path, limit: int) -> bytes:
    try:
        with path.open("rb") as stream:
            before = os.fstat(stream.fileno())
            if not path.is_file() or before.st_size > limit:
                raise Refusal("input-budget")
            data = stream.read(limit + 1)
            after = os.fstat(stream.fileno())
        if len(data) > limit:
            raise Refusal("input-budget")
        if (before.st_size, before.st_mtime_ns, before.st_ino) != (after.st_size, after.st_mtime_ns, after.st_ino):
            raise Refusal("input-changed")
        return data
    except OSError as exc:
        raise Refusal("input-unavailable") from exc


def runtime_manifest(root: Path, languages: list[str]) -> dict:
    try:
        try:
            manifest = checked_json(read_file(root / "installation.json", 256 * 1024))
        except Refusal as exc:
            raise Refusal("runtime-unavailable") from exc
        if manifest["version"] != 1:
            raise Refusal("runtime-version")
        if Path(manifest["python"]["path"]).resolve() != Path(sys.executable).resolve():
            raise Refusal("runtime-python-mismatch")
        lock_path = Path(__file__).resolve().parents[2] / "resources" / "recognition" / "runtime-lock.json"
        lock_bytes = read_file(lock_path, 256 * 1024)
        lock = checked_json(lock_bytes)
        if digest(lock_bytes) != manifest["runtimeLockSha256"]:
            raise Refusal("runtime-lock-mismatch")
        site_packages = root / "venv" / "Lib" / "site-packages"
        for package in lock["packages"]:
            for filename, expected in package["runtimeFiles"].items():
                installed = site_packages / filename
                if not installed.resolve().is_relative_to(site_packages.resolve()) or digest(read_file(installed, 64 * 1024 * 1024)) != expected["sha256"]:
                    raise Refusal("runtime-package-changed")
        binary = Path(manifest["tesseract"]["path"])
        if binary.resolve() != (root / "tesseract" / "tesseract.exe").resolve():
            raise Refusal("runtime-path")
        if manifest["tesseract"]["sha256"] != lock["tesseract"]["binarySha256"] or digest(read_file(binary, 64 * 1024 * 1024)) != lock["tesseract"]["binarySha256"]:
            raise Refusal("runtime-binary-changed")
        for filename, expected in lock["tesseract"]["files"].items():
            if digest(read_file(root / "tesseract" / filename, 64 * 1024 * 1024)) != expected["sha256"]:
                raise Refusal("runtime-dependency-changed")
        models = {}
        for language in languages:
            model = manifest["models"].get(language)
            if not model:
                raise Refusal("model-unavailable")
            model_path = Path(model["path"])
            if model_path.resolve() != (root / "tessdata" / f"{language}.traineddata").resolve():
                raise Refusal("runtime-path")
            expected_model = lock["models"]["files"][language]
            try:
                actual_model_hash = digest(read_file(model_path, 64 * 1024 * 1024))
            except Refusal as exc:
                raise Refusal("model-unavailable") from exc
            if model["sha256"] != expected_model["sha256"] or actual_model_hash != expected_model["sha256"]:
                raise Refusal("model-changed")
            models[language] = {key: value for key, value in model.items() if key != "path"}
        # Do not disclose operator paths in job evidence or Server responses.
        return {"version": 1, "python": manifest["python"]["version"],
                "tesseract": {"version": manifest["tesseract"]["version"], "sha256": manifest["tesseract"]["sha256"]},
                "pdfium": manifest["pdfium"], "pillow": manifest.get("pillow"),
                "models": models, "runtimeLockSha256": manifest["runtimeLockSha256"]}
    except (KeyError, TypeError, ValueError) as exc:
        raise Refusal("runtime-unavailable") from exc


def validate_request(request: dict) -> dict:
    if not isinstance(request, dict) or set(request) - {"version", "operation", "inputPath", "inputSha256", "mediaType", "languages", "pages", "dpi"}:
        raise Refusal("request-shape")
    operation = request.get("operation", "recognize")
    if not isinstance(operation, str) or operation not in {"recognize", "preview", "inspect"}:
        raise Refusal("unsupported-operation")
    if operation == "inspect" and "pages" in request:
        raise Refusal("inspect-selection")
    if type(request.get("version")) is not int or request.get("version") != 1 or not isinstance(request.get("mediaType"), str) or request.get("mediaType") not in {"application/pdf", "image/png", "image/jpeg"}:
        raise Refusal("unsupported-input")
    path = request.get("inputPath")
    if not isinstance(path, str) or not Path(path).is_absolute():
        raise Refusal("input-path")
    sha = request.get("inputSha256")
    if not isinstance(sha, str) or len(sha) != 64 or any(c not in "0123456789abcdef" for c in sha):
        raise Refusal("input-hash")
    languages = request.get("languages", [])
    if languages not in [["eng"], ["chi_sim"], ["chi_tra"], ["chi_sim", "eng"], ["chi_tra", "eng"]]:
        raise Refusal("unsupported-languages")
    dpi = request.get("dpi", 300)
    if type(dpi) is not int or not 72 <= dpi <= 300:
        raise Refusal("dpi-budget")
    pages = [{"page": 1}] if operation == "inspect" else request.get("pages")
    if not isinstance(pages, list) or not 1 <= len(pages) <= MAX_PAGES:
        raise Refusal("page-budget")
    seen = set()
    for page in pages:
        if not isinstance(page, dict) or set(page) - {"page", "regions"}:
            raise Refusal("page-shape")
        number = page.get("page")
        if type(number) is not int or number < 1 or number > MAX_PAGES or number in seen:
            raise Refusal("page-selection")
        seen.add(number)
        regions = page.get("regions")
        if regions is None:
            continue
        if not isinstance(regions, list) or not 1 <= len(regions) <= 64:
            raise Refusal("region-budget")
        ids, rects = set(), []
        for region in regions:
            if not isinstance(region, dict) or set(region) - {"id", "rect", "mode", "rotation", "segmentation", "ocrScale"}:
                raise Refusal("region-shape")
            rid, rect = region.get("id"), region.get("rect")
            if not isinstance(rid, str) or not 1 <= len(rid) <= 64 or rid in ids or rid.startswith("__weftext_auto_"):
                raise Refusal("region-id")
            ids.add(rid)
            if not isinstance(rect, list) or len(rect) != 4 or any(type(n) not in (int, float) or not 0 <= n < 100_000 for n in rect):
                raise Refusal("region-geometry")
            if rect[0] >= rect[2] or rect[1] >= rect[3]:
                raise Refusal("region-geometry")
            if any(min(rect[2], old[2]) > max(rect[0], old[0]) and min(rect[3], old[3]) > max(rect[1], old[1]) for old in rects):
                raise Refusal("overlapping-selection")
            rects.append(rect)
            if not isinstance(region.get("mode", "auto"), str) or region.get("mode", "auto") not in {"auto", "text", "ocr"} or type(region.get("rotation", 0)) is not int or region.get("rotation", 0) not in {0, 90, 180, 270}:
                raise Refusal("region-mode")
            if not isinstance(region.get("segmentation", "auto"), str) or region.get("segmentation", "auto") not in {"auto", "sparse", "block", "line"}:
                raise Refusal("region-segmentation")
            if type(region.get("ocrScale", 1)) not in (int, float) or region.get("ocrScale", 1) not in {.5, .75, 1}:
                raise Refusal("region-ocr-scale")
    return {**request, "operation": operation, "pages": pages, "dpi": dpi}


class ChildJob:
    """Kill the child and all OCR descendants on timeout or parent exit on Windows."""
    def __init__(self, process, *, suspended=False):
        self.process, self.handle = process, None
        if os.name != "nt":
            return
        from ctypes import wintypes
        class IO(ctypes.Structure):
            _fields_ = [(key, ctypes.c_ulonglong) for key in ("ReadOperationCount", "WriteOperationCount", "OtherOperationCount", "ReadTransferCount", "WriteTransferCount", "OtherTransferCount")]
        class BASIC(ctypes.Structure):
            _fields_ = [("PerProcessUserTimeLimit", ctypes.c_longlong), ("PerJobUserTimeLimit", ctypes.c_longlong), ("LimitFlags", wintypes.DWORD), ("MinimumWorkingSetSize", ctypes.c_size_t), ("MaximumWorkingSetSize", ctypes.c_size_t), ("ActiveProcessLimit", wintypes.DWORD), ("Affinity", ctypes.c_size_t), ("PriorityClass", wintypes.DWORD), ("SchedulingClass", wintypes.DWORD)]
        class EXTENDED(ctypes.Structure):
            _fields_ = [("BasicLimitInformation", BASIC), ("IoInfo", IO), ("ProcessMemoryLimit", ctypes.c_size_t), ("JobMemoryLimit", ctypes.c_size_t), ("PeakProcessMemoryUsed", ctypes.c_size_t), ("PeakJobMemoryUsed", ctypes.c_size_t)]
        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel.CreateJobObjectW.restype = wintypes.HANDLE
        kernel.SetInformationJobObject.argtypes = [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD]
        kernel.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
        kernel.CloseHandle.argtypes = [wintypes.HANDLE]
        self.kernel = kernel
        handle = kernel.CreateJobObjectW(None, None)
        limits = EXTENDED()
        limits.BasicLimitInformation.LimitFlags = 0x2000 | 0x200  # KILL_ON_JOB_CLOSE | JOB_MEMORY
        limits.JobMemoryLimit = MEMORY_BYTES
        if not handle or not kernel.SetInformationJobObject(handle, 9, ctypes.byref(limits), ctypes.sizeof(limits)) or not kernel.AssignProcessToJobObject(handle, wintypes.HANDLE(process._handle)):
            if handle:
                kernel.CloseHandle(handle)
            process.kill()
            process.wait()
            raise Refusal("process-isolation-unavailable")
        self.handle = handle
        if suspended:
            # Start suspended so no native worker can spawn OCR before belonging
            # to the kill-on-close Job. Popen closes its thread handle; resume the
            # process through the native process handle after assignment.
            ntdll = ctypes.WinDLL("ntdll")
            ntdll.NtResumeProcess.argtypes = [wintypes.HANDLE]
            ntdll.NtResumeProcess.restype = ctypes.c_long
            if ntdll.NtResumeProcess(wintypes.HANDLE(process._handle)) != 0:
                self.close()
                raise Refusal("process-isolation-unavailable")

    def close(self):
        if self.handle:
            self.kernel.CloseHandle(self.handle)
            self.handle = None
        if self.process.poll() is None:
            self.process.kill()
        self.process.wait()


def execute(request: dict, runtime: Path, progress=None) -> dict:
    request = validate_request(request)
    active_languages = request["languages"] if request["operation"] == "recognize" else []
    versions = runtime_manifest(runtime, active_languages)
    original = read_file(Path(request["inputPath"]), MAX_INPUT)
    if digest(original) != request["inputSha256"]:
        raise Refusal("input-changed")
    sniff = "application/pdf" if original.startswith(b"%PDF-") else "image/png" if original.startswith(b"\x89PNG\r\n\x1a\n") else "image/jpeg" if original.startswith(b"\xff\xd8\xff") else None
    if sniff != request["mediaType"]:
        raise Refusal("media-type-mismatch")
    sources = [(name, read_file(Path(__file__).with_name(name), 256 * 1024)) for name in ("worker.py", "engine.py")]
    implementation_hash = digest(b"".join(name.encode() + b"\0" + data for name, data in sources))
    result = {"version": 1, "operation": request["operation"], "input": {"sha256": request["inputSha256"], "bytes": len(original), "mediaType": sniff},
              "runtime": versions, "implementation": {"version": 1, "sha256": implementation_hash}, "selection": {"pages": request["pages"], "dpi": request["dpi"], "languages": request["languages"]},
              "budgets": {"inputBytes": MAX_INPUT, "pages": MAX_PAGES, "dpi": 300, "pixelsPerPage": MAX_PIXELS, "outputBytes": MAX_OUTPUT, "secondsPerPage": PAGE_SECONDS, "childMemoryBytes": MEMORY_BYTES},
              "pages": [], "losses": []}
    def report_progress():
        if progress is None:
            return
        # Host-owned provisional recovery evidence, never a second editable
        # authority. The callback cannot mutate the in-flight result/selection.
        snapshot = deepcopy(result)
        counts = {page["pageCount"] for page in snapshot["pages"] if "pageCount" in page}
        snapshot["pageCount"] = next(iter(counts)) if len(counts) == 1 else None
        statuses = [page["status"] for page in snapshot["pages"]]
        selected_complete = request["operation"] == "inspect" or len(snapshot["pages"]) == len(request["pages"])
        snapshot["status"] = "partial" if not selected_complete else "complete" if all(s in {"complete", "blank"} for s in statuses) else "failed" if all(s == "failed" for s in statuses) else "partial"
        progress(snapshot)
    jobs = runtime / "jobs"
    jobs.mkdir(exist_ok=True)
    # Only private files created by this invocation are removed by TemporaryDirectory.
    with tempfile.TemporaryDirectory(prefix="job-", dir=jobs, ignore_cleanup_errors=True) as directory:
        work = Path(directory)
        input_file = work / "original"
        input_file.write_bytes(original)
        for name, data in sources:
            (work / name).write_bytes(data)
        system_root = Path(os.environ.get("SystemRoot", "C:/Windows"))
        # Explicit runtime environment: parent credentials, proxies and unrelated
        # backend overrides never enter a native document/OCR process.
        env = {"SystemRoot": str(system_root), "WINDIR": str(system_root), "OMP_THREAD_LIMIT": "1",
               "TESSDATA_PREFIX": str(runtime / "tessdata"), "PYTHONNOUSERSITE": "1", "PYTHONUTF8": "1"}
        env["PATH"] = os.pathsep.join(str(path) for path in (system_root / "System32", system_root))
        env["TEMP"] = env["TMP"] = str(work)
        for selection in request["pages"]:
            config = work / f"page-{selection['page']}.json"
            config.write_text(json.dumps({"operation": request["operation"], "input": str(input_file), "mediaType": sniff, "selection": selection, "dpi": request["dpi"], "languages": request["languages"], "runtime": str(runtime), "versions": versions}), encoding="utf-8")
            output, diagnostics = work / "page.json", work / "diagnostics"
            with output.open("wb") as out, diagnostics.open("wb") as err:
                process = subprocess.Popen([sys.executable, "-I", str(work / "worker.py"), "--page-job", str(config)], stdin=subprocess.DEVNULL, stdout=out, stderr=err, env=env, creationflags=0x08000004 if os.name == "nt" else 0)
                job = ChildJob(process, suspended=os.name == "nt")
                started, failure = time.monotonic(), None
                try:
                    while process.poll() is None:
                        if time.monotonic() - started > PAGE_SECONDS:
                            failure = "page-time-budget"
                            break
                        if output.stat().st_size > MAX_OUTPUT or diagnostics.stat().st_size > 256 * 1024:
                            failure = "page-output-budget"
                            break
                        time.sleep(0.02)
                finally:
                    job.close()
            if failure is not None or process.returncode != 0:
                page = {"page": selection["page"], "status": "failed", "regions": [], "losses": [failure or "page-process-failed"]}
            else:
                try:
                    page = checked_json(read_file(output, MAX_OUTPUT))
                    if not isinstance(page, dict) or page.get("page") != selection["page"] or page.get("status") not in {"complete", "blank", "partial", "failed"} or not isinstance(page.get("regions"), list):
                        raise Refusal("page-result-invalid")
                except Refusal:
                    page = {"page": selection["page"], "status": "failed", "regions": [], "losses": ["page-result-invalid"]}
            if request["operation"] == "inspect" and "inspectionPages" in page:
                result["pages"].extend(page.pop("inspectionPages"))
            else:
                result["pages"].append(page)
            if len(json.dumps(result, ensure_ascii=False).encode("utf-8")) > MAX_OUTPUT - 65536:
                result["pages"].pop()
                result["pages"].append({"page": selection["page"], "status": "failed", "regions": [], "losses": ["job-output-budget"]})
                report_progress()
                for remaining in request["pages"][len(result["pages"]):]:
                    result["pages"].append({"page": remaining["page"], "status": "failed", "regions": [], "losses": ["job-output-budget"]})
                    report_progress()
                break
            report_progress()
    if runtime_manifest(runtime, active_languages) != versions:
        raise Refusal("runtime-changed-during-job")
    counts = {page["pageCount"] for page in result["pages"] if "pageCount" in page}
    result["pageCount"] = next(iter(counts)) if len(counts) == 1 else None
    statuses = [p["status"] for p in result["pages"]]
    result["status"] = "complete" if all(s in {"complete", "blank"} for s in statuses) else "failed" if all(s == "failed" for s in statuses) else "partial"
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runtime", type=Path)
    parser.add_argument("--page-job", type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args()
    try:
        if args.page_job:
            # -I isolates ambient Python paths; only this product directory is added.
            sys.path.insert(0, str(Path(__file__).resolve().parent))
            from engine import recognize_page
            result = recognize_page(checked_json(read_file(args.page_job, MAX_REQUEST)))
        else:
            if args.runtime is None or not args.runtime.is_absolute():
                raise Refusal("runtime-path")
            data = sys.stdin.buffer.read(MAX_REQUEST + 1)
            if len(data) > MAX_REQUEST:
                raise Refusal("request-budget")
            result = execute(checked_json(data), args.runtime.resolve())
    except Refusal as exc:
        result = {"version": 1, "status": "failed", "error": exc.code, "pages": []}
    except Exception:
        # Native errors may include private paths/text. Diagnostics are operator-only.
        result = {"version": 1, "status": "failed", "error": "recognition-failed", "pages": []}
    encoded = json.dumps(result, ensure_ascii=False, allow_nan=False, separators=(",", ":")).encode("utf-8")
    if len(encoded) > MAX_OUTPUT:
        encoded = b'{"version":1,"status":"failed","error":"job-output-budget","pages":[]}'
    sys.stdout.buffer.write(encoded)


if __name__ == "__main__":
    main()
