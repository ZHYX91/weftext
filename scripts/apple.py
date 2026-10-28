#!/usr/bin/env python3
"""Build/check the native Apple host library with the installed Apple SDKs only."""
from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import signal
import subprocess
import sys
import time
import uuid
import zipfile

ROOT = Path(__file__).resolve().parents[1]
PACKAGE = ROOT / "platforms" / "apple"
SIMULATOR_RUNTIME = "com.apple.CoreSimulator.SimRuntime.iOS-18-5"
SIMULATOR_DEVICE_TYPE = "com.apple.CoreSimulator.SimDeviceType.iPhone-16"


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
                "Tests/WeftextAppleHostTests/NativeWebSurfaceTests.swift",
                "HarnessTests/test_simulator.py"]
    for relative in required:
        if not (PACKAGE / relative).is_file():
            raise ValueError(f"Missing Apple host source: {relative}")
    # Source inventory only. Swift and WebKit semantics require the actual Mac build below.
    for p in PACKAGE.rglob("*.swift"):
        p.read_text(encoding="utf-8", errors="strict")
    subprocess.run([sys.executable, str(PACKAGE / "HarnessTests" / "test_simulator.py")],
                   cwd=ROOT, check=True, timeout=30)
    print("Apple source inventory and pure ownership guards passed; Swift/SDK/runtime validation was not run.")


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
        cleanup_errors = []

        def cleanup_failure(phase, error):
            cleanup_errors.append(error)
            detail = {"phase": phase, "type": type(error).__name__, "message": str(error)}
            if isinstance(error, OSError):
                detail["errno"] = error.errno
            entry.setdefault("cleanupErrors", []).append(detail)
            # A cleanup failure is additional evidence, never a replacement for
            # the original command deadline, exit or interruption.
            entry.setdefault("terminationReason", "cleanupUnknown")

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
            entry["ownedGroupGone"] = False
            entry["ownedLeaderReaped"] = False
            phase = "groupProbe"
            try:
                try:
                    os.killpg(process.pid, 0)
                except ProcessLookupError:
                    entry["ownedGroupGone"] = True
                else:
                    if "terminationReason" not in entry:
                        entry["terminationReason"] = "descendantsAfterLauncherExit"
                    # Exactly one signal to this original group. A permission
                    # rejection is retained, not retried or bypassed with sudo.
                    phase = "groupSignal"
                    os.killpg(process.pid, signal.SIGKILL)
                    deadline = time.monotonic() + 10
                    while time.monotonic() < deadline:
                        phase = "leaderPoll"
                        process.poll()
                        phase = "groupProbeAfterSignal"
                        try:
                            os.killpg(process.pid, 0)
                        except ProcessLookupError:
                            entry["ownedGroupGone"] = True
                            break
                        time.sleep(0.05)
            except BaseException as error:
                cleanup_failure(phase, error)
            finally:
                # Even failed group probing/signalling must observe the held
                # original Popen; no numeric-PID reopen or replacement process.
                try:
                    code = process.wait(timeout=10)
                    entry["ownedLeaderReaped"] = True
                except BaseException as error:
                    cleanup_failure("leaderWait", error)
                entry["cleanupUnknown"] = bool(cleanup_errors) or not (
                    entry["ownedGroupGone"] and entry["ownedLeaderReaped"])
                if entry["cleanupUnknown"]:
                    entry.setdefault("terminationReason", "cleanupUnknown")
        entry.update(exitCode=code, durationSeconds=round(time.monotonic() - started, 3),
                     logSha256=hashlib.sha256(log.read_bytes()).hexdigest())
    if caught:
        entry["state"] = "failed"
        save(run, record)
        raise caught
    if code or entry.get("terminationReason") or entry["cleanupUnknown"]:
        entry["state"] = "failed"
        save(run, record)
        # Preserve explicit interruption semantics when there was no original
        # deadline to retain; ordinary cleanup errors remain structured below.
        interrupted = next((error for error in cleanup_errors
                            if isinstance(error, (SystemExit, KeyboardInterrupt))), None)
        if interrupted is not None and entry.get("terminationReason") != "deadline":
            raise interrupted
        reason = entry.get("terminationReason", f"exit {code}")
        raise RuntimeError(f"Apple {label} failed ({reason}); preserve {log}") from (
            cleanup_errors[0] if cleanup_errors else None)
    entry["state"] = "passed"
    save(run, record)
    return log.read_text(encoding="utf-8", errors="replace")

def save(run: Path, record: dict) -> None:
    (run / "result.json").write_text(json.dumps(record, ensure_ascii=False, indent=2) + "\n",
                                     encoding="utf-8")


def strict_inventory(text: str) -> dict:
    def unique(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError("Duplicate key in original simulator inventory")
            result[key] = value
        return result
    value = json.loads(text, object_pairs_hook=unique)
    if not isinstance(value, dict):
        raise ValueError("Expected a simulator inventory object")
    return value


def simulator_udid(value: str) -> str:
    if not isinstance(value, str) or str(uuid.UUID(value)) != value.lower():
        raise ValueError("Expected an exact canonical simulator UDID")
    return value.upper()


def inventory_devices(inventory: dict) -> list[tuple[str, dict]]:
    groups = inventory.get("devices")
    if not isinstance(groups, dict) or not all(isinstance(items, list) for items in groups.values()):
        raise ValueError("Expected devices grouped by actual runtime identifier")
    rows = [(runtime, item) for runtime, items in groups.items() for item in items]
    if not all(isinstance(runtime, str) and isinstance(item, dict) for runtime, item in rows):
        raise ValueError("Malformed original simulator device inventory")
    return rows


def owned_device(inventory: dict, udid: str, name: str) -> dict:
    matching = [(runtime, item) for runtime, item in inventory_devices(inventory)
                if isinstance(item.get("udid"), str) and item["udid"].upper() == udid]
    if len(matching) != 1:
        raise ValueError("Original simulator UDID is missing or ambiguous")
    runtime, item = matching[0]
    if (runtime != SIMULATOR_RUNTIME or item.get("name") != name or
            item.get("deviceTypeIdentifier") != SIMULATOR_DEVICE_TYPE or
            item.get("isAvailable") is not True or simulator_udid(item["udid"]) != udid):
        raise ValueError("Original simulator ownership tuple changed")
    return item


def reconcile_created_device(inventory: dict, before: dict, name: str) -> str:
    matches = [(runtime, item) for runtime, item in inventory_devices(inventory) if item.get("name") == name]
    if len(matches) != 1 or matches[0][0] != SIMULATOR_RUNTIME:
        raise ValueError("Unknown create outcome: exact original name/runtime is not unique")
    udid = simulator_udid(matches[0][1].get("udid"))
    if any(isinstance(item.get("udid"), str) and item["udid"].upper() == udid
           for _, item in inventory_devices(before)):
        raise ValueError("Create outcome refers to a device that existed before this attempt")
    owned_device(inventory, udid, name)
    return udid


def archive_test_result(run: Path, record: dict) -> None:
    source = run / "ios-simulator.xcresult"
    if not source.is_dir():
        if record["simulator"]["testsPassed"]:
            raise ValueError("Successful simulator tests have no original xcresult bundle")
        return
    target = run / "ios-simulator.xcresult.zip"
    files = sorted(p for p in source.rglob("*") if p.is_file() or p.is_symlink())
    if not files or len(files) > 100_000 or any(p.is_symlink() for p in files):
        raise ValueError("Unsupported original xcresult archive shape; original retained")
    total = sum(p.stat().st_size for p in files)
    if total > 512 * 1024 * 1024:
        raise ValueError("Original xcresult exceeds archive budget; original retained")
    with zipfile.ZipFile(target, "x", compression=zipfile.ZIP_DEFLATED) as archive:
        for path in files:
            if not path.resolve().is_relative_to(source):
                raise ValueError("xcresult member escaped the original result bundle")
            before = path.stat()
            archive.write(path, path.relative_to(run).as_posix())
            after = path.stat()
            if (before.st_size, before.st_mtime_ns) != (after.st_size, after.st_mtime_ns):
                raise ValueError("Original xcresult changed during archival")
    record["simulator"]["resultBundleArchive"] = {"source": source.relative_to(run).as_posix(),
        "archive": target.relative_to(run).as_posix(), "bytes": target.stat().st_size,
        "sha256": hashlib.sha256(target.read_bytes()).hexdigest(), "originalLogicalBytes": total}


def run_simulator_tests(run: Path, package: Path, scheme: str, env: dict[str, str], record: dict) -> None:
    """One new, exactly qualified UDID; only its observed Shutdown settles cleanup."""
    original = {"name": "Weftext-" + str(uuid.uuid4()), "runtime": SIMULATOR_RUNTIME,
        "deviceType": SIMULATOR_DEVICE_TYPE, "createAttempted": False, "udid": None,
        "cleanupUnknown": False, "testsPassed": False}
    record["simulator"] = original
    save(run, record)
    base = ["/usr/bin/xcrun", "simctl"]
    for command in ("create", "bootstatus", "shutdown"):
        run_command(base + ["help", command], run, "simulator-help-" + command, env, 30, record, package)
    runtimes = strict_inventory(run_command(base + ["list", "runtimes", "--json"], run,
                               "simulator-runtimes", env, 30, record, package))
    matches = [item for item in runtimes.get("runtimes", [])
               if isinstance(item, dict) and item.get("identifier") == SIMULATOR_RUNTIME]
    if (len(matches) != 1 or matches[0].get("isAvailable") is not True or
            matches[0].get("version") != "18.5" or not isinstance(matches[0].get("buildversion"), str) or
            not isinstance(matches[0].get("supportedArchitectures"), list) or
            "arm64" not in matches[0].get("supportedArchitectures", []) or
            not any(item.get("identifier") == SIMULATOR_DEVICE_TYPE
                    for item in matches[0].get("supportedDeviceTypes", []) if isinstance(item, dict))):
        raise ValueError("The installed iOS18.5 runtime/device/architecture is not qualified")
    original["runtimeBuild"] = matches[0]["buildversion"]
    types = strict_inventory(run_command(base + ["list", "devicetypes", "--json"], run,
                            "simulator-device-types", env, 30, record, package))
    if sum(isinstance(item, dict) and item.get("identifier") == SIMULATOR_DEVICE_TYPE
           for item in types.get("devicetypes", [])) != 1:
        raise ValueError("Expected one installed iPhone16 device type")
    def devices(label):
        return strict_inventory(run_command(base + ["list", "devices", "--json"], run,
                                label, env, 30, record, package))
    before = devices("simulator-before-create")
    if any(item.get("name") == original["name"] for _, item in inventory_devices(before)):
        raise ValueError("Unique owned simulator name already exists; no replacement")
    primary_failure = None
    try:
        original.update(createAttempted=True, cleanupUnknown=True)
        save(run, record)
        # The raw log is saved before parsing. This create call is never automatically repeated.
        reply = run_command(base + ["create", original["name"], SIMULATOR_DEVICE_TYPE, SIMULATOR_RUNTIME],
                            run, "simulator-create", env, 60, record, package)
        udid = simulator_udid(reply.strip())
        original["udid"] = udid
        created = devices("simulator-after-create")
        if reconcile_created_device(created, before, original["name"]) != udid:
            raise ValueError("Create receipt and original simulator tuple disagree")
        if owned_device(created, udid, original["name"]).get("state") != "Shutdown":
            raise ValueError("New owned simulator was not observed Shutdown before boot")
        save(run, record)
        run_command(base + ["boot", udid], run, "simulator-boot", env, 60, record, package)
        run_command(base + ["bootstatus", udid, "-b"], run, "simulator-boot-status", env, 180, record, package)
        if owned_device(devices("simulator-after-boot"), udid, original["name"]).get("state") != "Booted":
            raise ValueError("Original simulator did not produce a Booted observation")
        output = run_command(["/usr/bin/xcodebuild", "-jobs", "2", "-scheme", scheme, "-sdk", "iphonesimulator",
            "-destination", "platform=iOS Simulator,id=" + udid, "-destination-timeout", "60",
            "-parallel-testing-enabled", "NO", "-maximum-concurrent-test-simulator-destinations", "1",
            "-derivedDataPath", str(run / "ios-tests"), "-resultBundlePath", str(run / "ios-simulator.xcresult"),
            "-clonedSourcePackagesDirPath", str(run / "packages"), "CODE_SIGNING_ALLOWED=NO", "test"],
            run, "ios-simulator-tests", env, 600, record, package)
        planned = [name for source in sorted((package / "Tests").rglob("*.swift"))
                   for name in re.findall(r"\bfunc\s+(test[A-Za-z0-9_]+)\(", source.read_text(encoding="utf-8"))]
        if not planned or len(planned) != len(set(planned)) or not all(
                re.search(r"Test Case .*" + re.escape(name) + r"[^\n]* passed", output) for name in planned):
            raise ValueError("The actual planned iOS XCTest cases did not all execute and pass")
        original.update(testsPassed=True, actualPassedCases=planned)
        record["mechanismValidation"]["iOSSimulatorRuntime"] = {"udid": udid, "passedCases": planned}
    except BaseException as error:
        primary_failure = error
        original["failure"] = f"{type(error).__name__}: {error}"
        raise
    finally:
        if original["createAttempted"]:
            try:
                current = devices("simulator-before-cleanup")
                udid = original["udid"]
                if udid is None:
                    udid = reconcile_created_device(current, before, original["name"])
                    original.update(udid=udid, createOutcomeReconciled=True)
                device = owned_device(current, udid, original["name"])
                shutdown_failure = None
                if device.get("state") != "Shutdown":
                    try:
                        run_command(base + ["shutdown", udid], run, "simulator-shutdown", env, 60, record, package)
                    except BaseException as error:
                        shutdown_failure = error
                        original["shutdownCommandFailure"] = f"{type(error).__name__}: {error}"
                final = owned_device(devices("simulator-after-cleanup"), udid, original["name"])
                if final.get("state") != "Shutdown":
                    raise ValueError("Original simulator cleanup has no observed Shutdown receipt")
                original.update(cleanupUnknown=False, finalState="Shutdown")
                if shutdown_failure:
                    raise shutdown_failure
            except BaseException as error:
                original["cleanupFailure"] = f"{type(error).__name__}: {error}"
                if primary_failure is None:
                    raise
            finally:
                save(run, record)
                try:
                    archive_test_result(run, record)
                except BaseException as error:
                    original["resultArchiveFailure"] = f"{type(error).__name__}: {error}"
                    if primary_failure is None and "cleanupFailure" not in original:
                        raise
                finally:
                    save(run, record)


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
    if record.get("simulatorTestsRequested"):
        test_executables = []
        for bundle in (run / "ios-tests" / "Build" / "Products").rglob("*.xctest"):
            info = bundle / "Info.plist"
            if not info.is_file() or info.stat().st_size > 64 * 1024:
                continue
            executable = plistlib.loads(info.read_bytes()).get("CFBundleExecutable")
            if not isinstance(executable, str) or not re.fullmatch(r"[A-Za-z0-9_.-]{1,128}", executable) or executable in {".", ".."}:
                raise ValueError("Unsupported original iOS test-bundle executable metadata")
            test_executables.append(bundle / executable)
        selections.append(("ios-simulator-test-bundle", test_executables, {"BUNDLE", "EXECUTE"}, {"arm64"}))
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
    result_bundle = record.get("simulator", {}).get("resultBundleArchive")
    if result_bundle is not None:
        source = run / result_bundle["archive"]
        destination = role_directory / source.name
        if not source.resolve().is_relative_to(run) or destination.exists():
            raise ValueError("Invalid fixed simulator result archive role")
        shutil.copyfile(source, destination)
        if (destination.stat().st_size != result_bundle["bytes"] or
                hashlib.sha256(destination.read_bytes()).hexdigest() != result_bundle["sha256"]):
            raise ValueError("Simulator result archive changed before role freeze")
        result_bundle["artifact"] = destination.relative_to(run).as_posix()
    if require_complete and record["unavailableRoles"]:
        raise ValueError("A required actual Apple artifact role is missing or ambiguous")


def build(state_root: Path, developer_dir: Path, simulator_tests: bool = False) -> int:
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
              "commands": [], "runtimeValidated": False, "productValidated": False,
              "simulatorTestsRequested": simulator_tests}
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
        if simulator_tests:
            run_simulator_tests(run, package_input, actual[0], env, record)
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
    compile_parser.add_argument("--run-simulator-tests", action="store_true",
                                help="Create one owned installed-iOS18.5 Simulator and run the same actual XCTest cases")
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
        return build(args.state_root, args.developer_dir, args.run_simulator_tests)
    finally:
        for value, handler in previous.items():
            signal.signal(value, handler)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
