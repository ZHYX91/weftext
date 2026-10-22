#!/usr/bin/env python3
"""Build/check the native Apple host library with the installed Apple SDKs only."""
from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
PACKAGE = ROOT / "platforms" / "apple"


def sources() -> dict[str, str]:
    paths = [p for p in PACKAGE.rglob("*") if p.is_file() and
             not any(part in {".build", ".swiftpm"} for part in p.relative_to(PACKAGE).parts)]
    paths.append(Path(__file__).resolve())
    return {p.relative_to(ROOT).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(paths)}


def static_check() -> None:
    required = ["Package.swift", "Sources/WeftextAppleHost/SurfaceGate.swift",
                "Sources/WeftextAppleHost/BundledAssets.swift",
                "Sources/WeftextAppleHost/NativeWebSurface.swift",
                "Tests/WeftextAppleHostTests/SurfaceGateTests.swift",
                "Tests/WeftextAppleHostTests/NativeWebSurfaceTests.swift"]
    for relative in required:
        if not (PACKAGE / relative).is_file():
            raise ValueError(f"Missing Apple host source: {relative}")
    # Source inventory only. Swift and WebKit semantics require the actual Mac build below.
    for p in PACKAGE.rglob("*.swift"):
        p.read_text(encoding="utf-8", errors="strict")
    print("Apple source inventory is complete; Swift/SDK/runtime validation was not run.")


def run_command(argv: list[str], run: Path, label: str, env: dict[str, str], timeout: int,
                record: dict, cwd: Path = PACKAGE) -> str:
    log = run / f"{label}.log"
    started = time.monotonic()
    entry = {"label": label, "argv": argv, "log": log.name, "deadlineSeconds": timeout,
             "state": "starting"}
    record["commands"].append(entry)
    save(run, record)
    with log.open("xb") as output:
        process = subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                                   stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
        code = None
        caught = None
        try:
            entry.update(pid=process.pid, processGroup=process.pid, state="running")
            save(run, record)
            code = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            entry["terminationReason"] = "deadline"
        except BaseException as error:
            caught = error
            entry["terminationReason"] = type(error).__name__
        finally:
            try:
                os.killpg(process.pid, 0)
            except ProcessLookupError:
                entry["ownedGroupGone"] = True
            else:
                entry["ownedGroupGone"] = False
                if "terminationReason" not in entry:
                    entry["terminationReason"] = "descendantsAfterLauncherExit"
                # Only the group created here; timeout or launcher exit is not a cleanup receipt.
                os.killpg(process.pid, signal.SIGKILL)
                deadline = time.monotonic() + 10
                while time.monotonic() < deadline:
                    # Reap our launcher, allowing adopted descendants to be reaped by the OS.
                    process.poll()
                    try:
                        os.killpg(process.pid, 0)
                    except ProcessLookupError:
                        entry["ownedGroupGone"] = True
                        break
                    time.sleep(0.05)
            try:
                code = process.wait(timeout=10)
                entry["ownedLeaderReaped"] = True
            except subprocess.TimeoutExpired:
                entry["ownedLeaderReaped"] = False
                entry["terminationReason"] = "cleanupUnknown"
        entry.update(exitCode=code, durationSeconds=round(time.monotonic() - started, 3),
                     logSha256=hashlib.sha256(log.read_bytes()).hexdigest())
    if caught:
        entry["state"] = "failed"
        save(run, record)
        raise caught
    if code or entry.get("terminationReason") or not entry["ownedGroupGone"]:
        entry["state"] = "failed"
        save(run, record)
        raise RuntimeError(f"Apple {label} failed; preserve {log}")
    entry["state"] = "passed"
    save(run, record)
    return log.read_text(encoding="utf-8", errors="replace")


def save(run: Path, record: dict) -> None:
    (run / "result.json").write_text(json.dumps(record, ensure_ascii=False, indent=2) + "\n",
                                     encoding="utf-8")


def freeze_roles(run: Path, package: Path, env: dict[str, str], record: dict,
                 require_complete: bool) -> None:
    """Fix actual executable/relocatable bytes, not a count of compiler intermediates."""
    role_directory = run / "roles"
    role_directory.mkdir()
    record["roles"] = {}
    record["unavailableRoles"] = []
    selections = [("macos-test-bundle", list((run / "swift").rglob(
        "WeftextAppleHostPackageTests.xctest/Contents/MacOS/WeftextAppleHostPackageTests")),
        {"BUNDLE", "EXECUTE"}, None)]
    for source in sorted((package / "Sources" / "WeftextAppleHost").glob("*.swift")):
        selections.append(("macos-host-" + source.stem + ".o", list((run / "swift").rglob(
            "WeftextAppleHost.build/" + source.name + ".o")), {"OBJECT"}, None))
    selections.append(("ios-simulator-host.o", [run / "ios" / "Build" / "Products" /
                       "Debug-iphonesimulator" / "WeftextAppleHost.o"], {"OBJECT"}, {"arm64", "x86_64"}))
    magic_values = {"feedface", "cefaedfe", "feedfacf", "cffaedfe", "cafebabe", "bebafeca",
                    "cafebabf", "bfbafeca"}
    for name, candidates, file_types, required_architectures in selections:
        available = [p for p in candidates if p.is_file()]
        if len(available) != 1:
            record["unavailableRoles"].append({"role": name, "availableCandidates": len(available)})
            continue
        source = available[0]
        if not source.resolve().is_relative_to(run):
            raise ValueError("Artifact role escaped the owned run")
        data = source.read_bytes()
        if not 16 <= len(data) <= 128 * 1024 * 1024 or data[:4].hex() not in magic_values:
            raise ValueError(f"Expected a bounded actual Mach-O role: {name}")
        description = run_command(["/usr/bin/file", "-b", str(source)], run,
                                  "role-" + name + "-file", env, 15, record, package).strip()
        architectures = set(run_command(["/usr/bin/xcrun", "lipo", "-archs", str(source)], run,
                                       "role-" + name + "-architectures", env, 15, record, package).split())
        headers = run_command(["/usr/bin/xcrun", "otool", "-hv", str(source)], run,
                              "role-" + name + "-headers", env, 15, record, package)
        if not architectures or (required_architectures is not None and architectures != required_architectures):
            raise ValueError(f"Unexpected architecture qualification for {name}")
        if not any(re.search(r"\b(?:MH_)?" + kind + r"\b", headers) for kind in file_types):
            raise ValueError(f"Unexpected actual Mach-O file type for {name}")
        destination = role_directory / name
        with destination.open("xb") as output:
            output.write(data)
        record["roles"][name] = {"source": source.relative_to(run).as_posix(),
            "artifact": destination.relative_to(run).as_posix(), "bytes": len(data),
            "sha256": hashlib.sha256(data).hexdigest(), "machOMagic": data[:4].hex(),
            "architectures": sorted(architectures), "fileDescription": description,
            "headersLog": "role-" + name + "-headers.log"}
    if require_complete and record["unavailableRoles"]:
        raise ValueError("A required actual Apple artifact role is missing or ambiguous")


def build(state_root: Path, developer_dir: Path) -> int:
    if sys.platform != "darwin":
        raise ValueError("Apple SDK compilation requires an actual macOS host; no cross-platform pass")
    if not state_root.is_absolute() or not developer_dir.is_absolute():
        raise ValueError("State root and DEVELOPER_DIR must be explicit absolute paths")
    if not (developer_dir / "usr" / "bin" / "xcodebuild").is_file():
        raise ValueError("The selected installed Xcode is unavailable; no SDK download or license fallback")
    state_root = state_root.resolve()
    if state_root == ROOT or state_root.is_relative_to(PACKAGE):
        raise ValueError("Build state must be outside Apple source")
    state_root.mkdir(parents=True, exist_ok=True)
    run = state_root / str(uuid.uuid4())
    run.mkdir()
    env = dict(os.environ)
    env["DEVELOPER_DIR"] = str(developer_dir)
    env["TMPDIR"] = str(run / "tmp")
    Path(env["TMPDIR"]).mkdir()
    env["CLANG_MODULE_CACHE_PATH"] = str(run / "clang-modules")
    before = sources()
    # Xcode/SwiftPM may create .swiftpm beside Package.swift. Compile only this run's fixed input.
    package_input = run / "source"
    package_input.mkdir()
    for relative, expected in before.items():
        source = ROOT / relative
        if source.is_relative_to(PACKAGE):
            data = source.read_bytes()
            if hashlib.sha256(data).hexdigest() != expected:
                raise ValueError("Apple source changed before input freeze")
            destination = package_input / source.relative_to(PACKAGE)
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(data)
    record = {"schemaVersion": 1, "kind": "appleHostLibraryBuild", "startedUtc":
              datetime.datetime.now(datetime.timezone.utc).isoformat(), "state": "running",
              "sources": before, "sourceSnapshot": str(package_input), "developerDirectory": str(developer_dir),
              "commands": [], "runtimeValidated": False, "productValidated": False}
    save(run, record)
    print(run, flush=True)
    try:
        for label, argv in [
            ("os", ["/usr/bin/sw_vers"]), ("architecture", ["/usr/bin/uname", "-m"]),
            ("xcode", ["/usr/bin/xcodebuild", "-version"]),
            ("first-launch", ["/usr/bin/xcodebuild", "-checkFirstLaunchStatus"]),
            ("sdks", ["/usr/bin/xcodebuild", "-showsdks"]),
            ("swift", ["/usr/bin/xcrun", "swift", "--version"]),
            ("runtimes", ["/usr/bin/xcrun", "simctl", "list", "runtimes", "--json"]),
        ]:
            run_command(argv, run, label, env, 60, record)
        macos_output = run_command(["/usr/bin/xcrun", "swift", "test", "--package-path", str(package_input),
                     "--scratch-path", str(run / "swift"), "--cache-path", str(state_root / "cache"),
                     "--jobs", "2"], run, "macos-tests", env, 600, record, package_input)
        mechanism_source = package_input / "Tests" / "WeftextAppleHostTests" / "NativeWebSurfaceTests.swift"
        mechanism_tests = re.findall(r"\bfunc\s+(test[A-Za-z0-9_]+)\(", mechanism_source.read_text(encoding="utf-8"))
        if not mechanism_tests or not all(re.search(r"Test Case .*" + re.escape(name) +
                                                   r"[^\n]* passed", macos_output)
                                           for name in mechanism_tests):
            raise ValueError("The planned actual WebKit tests did not all produce passed XCTest results")
        record["mechanismValidation"] = {"macosWebKitTestsPassed": mechanism_tests,
                                         "iOSSimulatorRuntime": "not run", "product": "not validated"}
        schemes_text = run_command(["/usr/bin/xcodebuild", "-list", "-json"], run, "schemes", env, 90, record, package_input)
        # Xcode may print a short preamble before its JSON inventory.
        schemes = json.loads(schemes_text[schemes_text.index("{"):])
        actual = [name for section in schemes.values() if isinstance(section, dict)
                  for name in section.get("schemes", []) if name == "WeftextAppleHost"]
        if len(actual) != 1:
            raise ValueError("Expected one actual WeftextAppleHost scheme; inspect preserved inventory")
        run_command(["/usr/bin/xcodebuild", "-jobs", "2", "-scheme", actual[0], "-sdk", "iphonesimulator",
                     "-destination", "generic/platform=iOS Simulator", "-derivedDataPath", str(run / "ios"),
                     "-clonedSourcePackagesDirPath", str(run / "packages"),
                     "CODE_SIGNING_ALLOWED=NO", "build"], run, "ios-sdk-build", env, 600, record, package_input)
        record["sourcesUnchanged"] = sources() == before
        if not record["sourcesUnchanged"]:
            raise ValueError("Apple source changed during compilation")
        record["state"] = "passed"
        return 0
    except BaseException as error:
        record.update(state="failed", failure=f"{type(error).__name__}: {error}", sourcesUnchanged=sources() == before)
        raise
    finally:
        failed_before_role_collection = record["state"] != "passed"
        try:
            freeze_roles(run, package_input, env, record, require_complete=not failed_before_role_collection)
        except BaseException as error:
            record.update(state="failed", roleCollectionError=f"{type(error).__name__}: {error}")
            if not failed_before_role_collection:
                raise
        finally:
            save(run, record)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("check")
    compile_parser = commands.add_parser("build")
    compile_parser.add_argument("--state-root", type=Path, required=True)
    compile_parser.add_argument("--developer-dir", type=Path, required=True)
    args = parser.parse_args()
    static_check()
    if args.command == "check":
        return 0
    previous = {}
    def terminate(signum, _frame):
        # Complete the bounded finally path once, even if the owner repeats the same signal.
        signal.signal(signum, signal.SIG_IGN)
        raise SystemExit(128 + signum)
    try:
        for name in ("SIGTERM", "SIGHUP"):
            value = getattr(signal, name, None)
            if value is not None:
                previous[value] = signal.signal(value, terminate)
        return build(args.state_root, args.developer_dir)
    finally:
        for value, handler in previous.items():
            signal.signal(value, handler)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
