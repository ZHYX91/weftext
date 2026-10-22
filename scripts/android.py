#!/usr/bin/env python3
"""One Android host source/build entry. Never installs SDKs or accepts licenses."""
from __future__ import annotations

import argparse
import ctypes
from ctypes import wintypes
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import uuid
import time
import xml.etree.ElementTree as ET
import importlib.util

ROOT = Path(__file__).resolve().parents[1]
ANDROID = ROOT / "platforms" / "android"
GRADLE_VERSION = "8.13"
TASKS = [":host:assembleDebug", ":host:lintDebug", ":host:testDebugUnitTest", ":host:compileDebugAndroidTestKotlin",
         ":host:assembleDebugAndroidTest"]


def run_owned_windows_tree(command: list[str], environment: dict[str, str], output, record: dict, report: Path) -> int:
    """Admit the suspended launcher into a non-breakaway Job before any child can execute."""
    if os.name != "nt":
        raise ValueError("This scheduled build requires the Windows owned-tree supervisor")
    import msvcrt
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    pointer = ctypes.c_size_t

    class BasicLimits(ctypes.Structure):
        _fields_ = [("processTime", ctypes.c_int64), ("jobTime", ctypes.c_int64),
                    ("flags", wintypes.DWORD), ("minWorking", pointer), ("maxWorking", pointer),
                    ("activeLimit", wintypes.DWORD), ("affinity", pointer),
                    ("priority", wintypes.DWORD), ("scheduling", wintypes.DWORD)]

    class IoCounters(ctypes.Structure):
        _fields_ = [(name, ctypes.c_uint64) for name in ("readOps", "writeOps", "otherOps", "read", "write", "other")]

    class ExtendedLimits(ctypes.Structure):
        _fields_ = [("basic", BasicLimits), ("io", IoCounters), ("processMemory", pointer),
                    ("jobMemory", pointer), ("peakProcess", pointer), ("peakJob", pointer)]

    class Accounting(ctypes.Structure):
        _fields_ = [(name, ctypes.c_int64) for name in ("user", "kernel", "periodUser", "periodKernel")] + [
            (name, wintypes.DWORD) for name in ("pageFaults", "total", "active", "terminated")]

    class MemoryStatus(ctypes.Structure):
        _fields_ = [("length", wintypes.DWORD), ("load", wintypes.DWORD)] + [
            (name, ctypes.c_uint64) for name in ("physical", "freePhysical", "pagefile", "freePagefile", "virtual", "freeVirtual", "extended")]

    class Startup(ctypes.Structure):
        _fields_ = [("cb", wintypes.DWORD), ("reserved", wintypes.LPWSTR), ("desktop", wintypes.LPWSTR),
                    ("title", wintypes.LPWSTR)] + [(name, wintypes.DWORD) for name in
                    ("x", "y", "xsize", "ysize", "xcount", "ycount", "fill", "flags")] + [
                    ("show", wintypes.WORD), ("reservedSize", wintypes.WORD), ("reservedBytes", ctypes.c_void_p),
                    ("stdin", wintypes.HANDLE), ("stdout", wintypes.HANDLE), ("stderr", wintypes.HANDLE)]

    class ProcessInfo(ctypes.Structure):
        _fields_ = [("process", wintypes.HANDLE), ("thread", wintypes.HANDLE), ("pid", wintypes.DWORD), ("tid", wintypes.DWORD)]

    def api(name, arguments, result=wintypes.BOOL):
        function = getattr(kernel, name); function.argtypes = arguments; function.restype = result
        return function

    create_job = api("CreateJobObjectW", [ctypes.c_void_p, wintypes.LPCWSTR], wintypes.HANDLE)
    set_job = api("SetInformationJobObject", [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD])
    query_job = api("QueryInformationJobObject", [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD, ctypes.c_void_p])
    assign_job = api("AssignProcessToJobObject", [wintypes.HANDLE, wintypes.HANDLE])
    terminate_job = api("TerminateJobObject", [wintypes.HANDLE, wintypes.UINT])
    create_process = api("CreateProcessW", [wintypes.LPCWSTR, wintypes.LPWSTR, ctypes.c_void_p, ctypes.c_void_p,
                         wintypes.BOOL, wintypes.DWORD, ctypes.c_void_p, wintypes.LPCWSTR,
                         ctypes.POINTER(Startup), ctypes.POINTER(ProcessInfo)])
    resume = api("ResumeThread", [wintypes.HANDLE], wintypes.DWORD)
    wait = api("WaitForSingleObject", [wintypes.HANDLE, wintypes.DWORD], wintypes.DWORD)
    exit_code = api("GetExitCodeProcess", [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)])
    terminate_process = api("TerminateProcess", [wintypes.HANDLE, wintypes.UINT])
    close = api("CloseHandle", [wintypes.HANDLE])
    memory_status = api("GlobalMemoryStatusEx", [ctypes.POINTER(MemoryStatus)])
    system_directory = api("GetSystemDirectoryW", [wintypes.LPWSTR, wintypes.UINT], wintypes.UINT)
    job = create_job(None, None)
    if not job: raise ctypes.WinError(ctypes.get_last_error())
    limits = ExtendedLimits(); limits.basic.flags = 0x2000 | 0x200  # kill-on-close, aggregate job memory limit
    limits.jobMemory = 4 * 1024**3
    process = ProcessInfo()
    assigned = False
    start = time.monotonic()
    reason = None
    peak = 0
    minimum_free = None
    try:
        if not set_job(job, 9, ctypes.byref(limits), ctypes.sizeof(limits)):
            raise ctypes.WinError(ctypes.get_last_error())
        available = MemoryStatus(); available.length = ctypes.sizeof(available)
        if not memory_status(ctypes.byref(available)):
            raise ctypes.WinError(ctypes.get_last_error())
        if available.freePhysical <= 4 * 1024**3:
            raise ValueError("Host has at most 4 GiB free; owned build was not launched")
        system = ctypes.create_unicode_buffer(32768)
        if not system_directory(system, len(system)): raise ctypes.WinError(ctypes.get_last_error())
        shell = str(Path(system.value) / "cmd.exe")
        line = ctypes.create_unicode_buffer('"' + shell + '" /d /s /c "' + subprocess.list2cmdline(command) + '"')
        block = ctypes.create_unicode_buffer("\0".join(key + "=" + value for key, value in sorted(environment.items())) + "\0\0")
        with open(os.devnull, "rb") as input_stream:
            out_fd = output.fileno(); in_fd = input_stream.fileno()
            os.set_inheritable(out_fd, True); os.set_inheritable(in_fd, True)
            startup = Startup(); startup.cb = ctypes.sizeof(startup); startup.flags = 0x100 | 1
            startup.stdin = msvcrt.get_osfhandle(in_fd)
            startup.stdout = startup.stderr = msvcrt.get_osfhandle(out_fd)
            try:
                if not create_process(shell, line, None, None, True, 0x4 | 0x08000000 | 0x400,
                                      block, str(ANDROID), ctypes.byref(startup), ctypes.byref(process)):
                    raise ctypes.WinError(ctypes.get_last_error())
            finally:
                os.set_inheritable(out_fd, False); os.set_inheritable(in_fd, False)
        if not assign_job(job, process.process):
            raise ctypes.WinError(ctypes.get_last_error())
        assigned = True
        record.update(state="running", pid=process.pid, ownedTree="windowsJob", deadlineSeconds=1500,
                      jobMemoryLimit=limits.jobMemory, startedUtc=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()))
        report.write_text(json.dumps(record, ensure_ascii=False, indent=2), encoding="utf-8")
        if resume(process.thread) == 0xffffffff: raise ctypes.WinError(ctypes.get_last_error())
        while True:
            accounting = Accounting()
            if not query_job(job, 1, ctypes.byref(accounting), ctypes.sizeof(accounting), None):
                raise ctypes.WinError(ctypes.get_last_error())
            if not query_job(job, 9, ctypes.byref(limits), ctypes.sizeof(limits), None):
                raise ctypes.WinError(ctypes.get_last_error())
            peak = max(peak, limits.peakJob)
            if not memory_status(ctypes.byref(available)): raise ctypes.WinError(ctypes.get_last_error())
            minimum_free = available.freePhysical if minimum_free is None else min(minimum_free, available.freePhysical)
            if accounting.active == 0: break
            launcher_failed = False
            if wait(process.process, 0) == 0:
                observed_exit = wintypes.DWORD()
                if not exit_code(process.process, ctypes.byref(observed_exit)):
                    raise ctypes.WinError(ctypes.get_last_error())
                record["launcherExitCode"] = observed_exit.value
                launcher_failed = observed_exit.value != 0
            if reason is None and (launcher_failed or time.monotonic() - start >= 1500 or available.freePhysical <= 4 * 1024**3):
                reason = ("launcherFailedWithChildren" if launcher_failed else
                          "deadline" if time.monotonic() - start >= 1500 else "hostMemoryGuard")
                if not terminate_job(job, 125): raise ctypes.WinError(ctypes.get_last_error())
            time.sleep(1)
        code = wintypes.DWORD()
        if wait(process.process, 0) != 0 or not exit_code(process.process, ctypes.byref(code)):
            raise ValueError("Owned launcher exit was not observed")
        record.update(ownedTreeActiveProcesses=0, ownedTreeTotalProcesses=accounting.total,
                      peakJobMemoryBytes=peak, minimumHostFreeBytes=minimum_free,
                      durationSeconds=round(time.monotonic() - start, 3), terminationReason=reason)
        return 125 if reason else code.value
    finally:
        if process.process:
            if assigned:
                # An observation error must not escape and release the caller's SDK
                # read-only guards while an owned SDK process may still be alive.
                # Retain this exact Job/handles until the OS confirms ActiveProcesses=0.
                # No deadline or failed query substitutes for that observation.
                drain_errors = 0
                while True:
                    terminated = bool(terminate_job(job, 125))
                    termination_error = None if terminated else ctypes.get_last_error()
                    accounting = Accounting()
                    if not query_job(job, 1, ctypes.byref(accounting), ctypes.sizeof(accounting), None):
                        query_error = ctypes.get_last_error()
                    else:
                        query_error = None
                        if accounting.active == 0:
                            record["ownedTreeActiveProcesses"] = 0
                            record["ownedTreeDrained"] = True
                            break
                    if termination_error is not None or query_error is not None:
                        drain_errors += 1
                        record.update(ownedTreeDrained=False, ownedTreeDrainObservation="unknown",
                                      ownedTreeDrainErrors=drain_errors,
                                      ownedTreeDrainLastWin32={"terminate": termination_error, "query": query_error})
                        try:
                            report.write_text(json.dumps(record, ensure_ascii=False, indent=2), encoding="utf-8")
                        except OSError:
                            pass  # Keep the ownership/guard lifetime even if reporting fails.
                    time.sleep(0.1)
            else:  # Still suspended, therefore has never been able to spawn a child.
                terminate_process(process.process, 125); wait(process.process, 0xffffffff)
            close(process.thread); close(process.process)
        close(job)


def source_files() -> list[Path]:
    return sorted(path for path in ANDROID.rglob("*") if path.is_file()
                  and not any(part in {".gradle", "build", ".kotlin"} for part in path.relative_to(ANDROID).parts))


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def check() -> dict:
    files = source_files()
    if not files:
        raise ValueError("Android sources are absent")
    manifest = ET.parse(ANDROID / "host/src/main/AndroidManifest.xml").getroot()
    if manifest.findall("uses-permission") or manifest.findall("application/activity") or manifest.findall("application/service"):
        raise ValueError("Host library must not register a placeholder product or grant permissions")
    properties = (ANDROID / "gradle.properties").read_text(encoding="utf-8")
    for required in ("android.builder.sdkDownload=false", "org.gradle.daemon=false", "org.gradle.workers.max=2"):
        if required not in properties.splitlines():
            raise ValueError("Android build isolation property missing: " + required)
    for name in ("SurfaceGuard", "NativeWebSurface", "NativeHostActivity", "SafDocuments", "FrozenWorker"):
        if not (ANDROID / f"host/src/main/java/org/weftext/android/host/{name}.kt").is_file():
            raise ValueError("Android OS layer missing: " + name)
    return {"schemaVersion": 1, "kind": "androidHostSourceCheck", "scope": "hostLibrary",
            "compiled": False, "runtimeValidated": False,
            "sources": {path.relative_to(ROOT).as_posix(): digest(path) for path in files}}


def absolute_directory(value: str) -> Path:
    path = Path(value)
    if not path.is_absolute() or not path.is_dir():
        raise ValueError("Explicit existing absolute directory required")
    # Gradle's Windows batch entry is the only shell boundary; reject metacharacters.
    if any(character in str(path) for character in '&|<>^%!\r\n"'):
        raise ValueError("Build path contains shell metacharacters")
    return path.resolve()


def workspace() -> Path:
    for candidate in ROOT.parents:
        if (candidate / "repos/weftext/README.md").is_file() and (candidate / "infra/README.md").is_file():
            return candidate
    raise ValueError("This entry requires its independently owned Weftext workspace")


def build(arguments: argparse.Namespace) -> int:
    checked = check()
    gradle = absolute_directory(arguments.gradle_home)
    sdk = absolute_directory(arguments.sdk)
    java = absolute_directory(arguments.java_home)
    required = [gradle / ("bin/gradle.bat" if os.name == "nt" else "bin/gradle"),
                sdk / "platforms/android-35/android.jar", sdk / "build-tools/35.0.0/source.properties",
                java / ("bin/java.exe" if os.name == "nt" else "bin/java")]
    if not all(path.is_file() for path in required) or not (gradle / f"lib/gradle-launcher-{GRADLE_VERSION}.jar").is_file():
        raise ValueError("Pinned Gradle 8.13, SDK 35/build-tools 35.0.0 and JDK are required; no auto-install")
    state = Path(arguments.state_root)
    if not state.is_absolute() or any(character in str(state) for character in '&|<>^%!\r\n"'):
        raise ValueError("Explicit absolute owned state root required")
    state = state.resolve()
    allowed = workspace() / ".workspace" / "build"
    if not state.is_relative_to(allowed) or state == allowed:
        raise ValueError("Android state must be a child of this workspace's .workspace/build")
    state.mkdir(parents=True, exist_ok=True)
    marker = state / "android-build-owner.json"
    owner = {"schemaVersion": 1, "kind": "weftextAndroidBuild", "workspace": str(workspace())}
    if marker.exists():
        if json.loads(marker.read_text(encoding="utf-8")) != owner:
            raise ValueError("Android state ownership mismatch")
    else:
        if any(state.iterdir()):
            raise ValueError("Unowned state directory is not empty")
        with marker.open("x", encoding="utf-8") as output:
            json.dump(owner, output, indent=2)
    run = state / "runs" / str(uuid.uuid4())
    run.mkdir(parents=True, exist_ok=False)
    command = [str(required[0]), "--no-daemon", "--max-workers=2", "--console=plain", "--stacktrace",
               "--project-dir", str(ANDROID), "--project-cache-dir", str(run / "project-cache"),
               "-PweftextBuildRoot=" + str(run / "build"),
               "-Pkotlin.project.persistent.dir=" + str(run / "kotlin-state"),
               "-Pkotlin.project.persistent.dir.gradle.disableWrite=true"]
    if not arguments.allow_dependency_downloads:
        command.append("--offline")
    command.extend(TASKS)
    environment = os.environ.copy()
    environment.update(JAVA_HOME=str(java), ANDROID_HOME=str(sdk), ANDROID_SDK_ROOT=str(sdk),
                       GRADLE_USER_HOME=str(state / "gradle-user-home"),
                       ANDROID_USER_HOME=str(state / "android-user-home"))
    temporary = run / "temp"
    temporary.mkdir(exist_ok=False)
    system = Path(os.environ["SystemRoot"])
    environment.update(TEMP=str(temporary), TMP=str(temporary),
                       PATH=os.pathsep.join((str(java / "bin"), str(system / "System32"), str(system))))
    # Do not inherit JVM/Gradle injection from an unrelated developer environment.
    for key in ("GRADLE_OPTS", "JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS", "GRADLE_RO_DEP_CACHE",
                "ANDROID_SDK_HOME", "ANDROID_PREFS_ROOT", "ANDROID_AVD_HOME", "ADB_VENDOR_KEYS", "ANDROID_SERIAL"):
        environment.pop(key, None)
    record = {**checked, "kind": "androidHostBuild", "argv": command, "javaHome": str(java),
              "sdk": str(sdk), "gradleHome": str(gradle), "runDirectory": str(run), "tasks": TASKS,
              "dependencyDownloads": arguments.allow_dependency_downloads, "state": "starting"}
    record["entrySha256"] = digest(Path(__file__).resolve())
    report = run / "result.json"
    report.write_text(json.dumps(record, ensure_ascii=False, indent=2), encoding="utf-8")
    with (run / "gradle.log").open("xb") as output:
        try:
            code = run_owned_windows_tree(command, environment, output, record, report)
        except BaseException:
            record.update(state="failed", compiled=False)
            report.write_text(json.dumps(record, ensure_ascii=False, indent=2), encoding="utf-8")
            raise
    record.update(state="passed" if code == 0 else "failed", exitCode=code, compiled=code == 0)
    try:
        record["sourcesUnchanged"] = checked["sources"] == check()["sources"] and record["entrySha256"] == digest(Path(__file__).resolve())
        if not record["sourcesUnchanged"]:
            code = 2
            record.update(state="failed", exitCode=code, compiled=False, failure="sourceChangedDuringBuild")
        # Only final task outputs: recursively scanning AGP dex transforms can hit Windows path limits.
        outputs = run / "build" / "host" / "outputs"
        built = list((outputs / "aar").glob("*.aar"))
        apks = []
        for metadata in outputs.rglob("output-metadata.json"):
            actual = json.loads(metadata.read_text(encoding="utf-8"))
            if actual.get("artifactType", {}).get("type") != "APK": continue
            for element in actual.get("elements", []):
                name = element.get("outputFile", "")
                if not isinstance(name, str) or Path(name).name != name or Path(name).suffix != ".apk":
                    raise ValueError("Invalid actual Gradle APK output metadata")
                apk = metadata.parent / name
                if not apk.is_file(): raise ValueError("Declared Gradle APK is absent")
                apks.append(apk)
        built.extend(apks)
        record["artifacts"] = {path.relative_to(run).as_posix(): digest(path) for path in sorted(built)}
        if code == 0 and len(apks) != 1:
            code = 2
            record.update(state="failed", exitCode=code, compiled=False, failure="ambiguousOsInstrumentationArtifact")
        record["artifactRoles"] = {} if len(apks) != 1 else {
            "osInstrumentation": {"path": apks[0].relative_to(run).as_posix(), "sha256": digest(apks[0])}}
        record["logSha256"] = digest(run / "gradle.log")
    except BaseException as error:
        record.update(state="failed", exitCode=2, compiled=False, failure="postBuildCollection",
                      errorType=type(error).__name__, error=str(error))
        report.write_text(json.dumps(record, ensure_ascii=False, indent=2), encoding="utf-8")
        raise
    report.write_text(json.dumps(record, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"schemaVersion": 1, "kind": "androidHostBuild", "exitCode": code,
                      "report": str(report), "runtimeValidated": False}, ensure_ascii=False))
    return code


def device_module():
    source = ANDROID / "tools" / "device_runner.py"
    spec = importlib.util.spec_from_file_location("weftext_android_device_runner", source)
    if spec is None or spec.loader is None:
        raise ValueError("Android device runner source is absent")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    original_bytecode = sys.dont_write_bytecode
    try:
        sys.dont_write_bytecode = True
        spec.loader.exec_module(module)
    finally:
        sys.dont_write_bytecode = original_bytecode
    return module


def prepare_device(arguments: argparse.Namespace) -> int:
    build_result = Path(arguments.build_result)
    if not build_result.is_absolute() or not build_result.is_file():
        raise ValueError("Explicit actual build result required")
    built = json.loads(build_result.read_text(encoding="utf-8"))
    if built.get("state") != "passed" or not built.get("sourcesUnchanged"):
        raise ValueError("Device plan requires a passed immutable-source build")
    if built.get("sources") != check()["sources"] or built.get("entrySha256") != digest(Path(__file__).resolve()):
        raise ValueError("Current source does not match the selected build")
    role = built["artifactRoles"]["osInstrumentation"]
    apk = (build_result.parent / role["path"]).resolve()
    if not apk.is_relative_to(build_result.parent.resolve()) or digest(apk) != role["sha256"]:
        raise ValueError("Instrumentation role does not match its actual artifact")
    plan = device_module().prepare_plan(workspace=workspace(), repository=ROOT,
        sdk=absolute_directory(arguments.sdk), java_home=absolute_directory(arguments.java_home),
        state_root=Path(arguments.state_root), apk=apk, build_result=build_result,
        test_class="org.weftext.android.host.FrozenWorkerBoundaryTest", expected_test_count=6)
    print(json.dumps(plan, ensure_ascii=False, indent=2))
    return 0


def run_device(arguments: argparse.Namespace) -> int:
    plan_path = Path(arguments.plan)
    if not plan_path.is_absolute() or not plan_path.is_file() or digest(plan_path) != arguments.plan_sha256:
        raise ValueError("Explicit reviewed plan and exact SHA256 required")
    module = device_module()
    plan = module.validate_plan(plan_path)
    if not plan_path.resolve().is_relative_to(workspace() / ".workspace" / "build"):
        raise ValueError("Device plan is outside this workspace's owned build tree")
    run = plan_path.parent
    report = run / "owned-tree-result.json"
    with report.open("x", encoding="utf-8") as output:
        json.dump({"kind": "androidOsOwnedTree", "state": "starting", "planSha256": arguments.plan_sha256}, output)
    command = [sys.executable, str(Path(__file__).resolve()), "_device-driver", "--plan", str(plan_path)]
    environment = os.environ.copy()
    record = {"schemaVersion": 1, "kind": "androidOsOwnedTree", "scope": "hostOsProbe",
              "state": "starting", "planSha256": arguments.plan_sha256, "argv": command,
              "productCoreValidated": False}
    try:
        with module.SdkReadOnlyFiles() as sdk_files:
            sdk_files.verify(plan["sdkReadOnlyFiles"])
            with (run / "owned-tree.log").open("xb") as output:
                code = run_owned_windows_tree(command, environment, output, record, report)
            record["sdkReadOnlyFilesUnchanged"] = sdk_files.verify(plan["sdkReadOnlyFiles"]) == plan["sdkReadOnlyFiles"]
    except BaseException as error:
        record.update(state="failed", productCoreValidated=False, failureType=type(error).__name__,
                      failure=str(error))
        report.write_text(json.dumps(record, ensure_ascii=False, indent=2), encoding="utf-8")
        raise
    record.update(state="passed" if code == 0 else "failed", exitCode=code)
    report.write_text(json.dumps(record, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"kind": "androidOsOwnedTree", "exitCode": code, "report": str(report),
                      "productCoreValidated": False}, ensure_ascii=False))
    return code


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("check", help="Check host source inventory without compiling or changing OS state")
    compiler = commands.add_parser("build", help="Build/lint/test only the real Android host AAR")
    for name in ("gradle-home", "sdk", "java-home", "state-root"):
        compiler.add_argument("--" + name, required=True)
    compiler.add_argument("--allow-dependency-downloads", action="store_true")
    planner = commands.add_parser("device-plan", help="Prepare exact isolated OS-probe commands without starting a device")
    for name in ("sdk", "java-home", "state-root", "build-result"):
        planner.add_argument("--" + name, required=True)
    device = commands.add_parser("device-run", help="Run a reviewed exact OS-probe plan inside an owned Windows Job")
    device.add_argument("--plan", required=True); device.add_argument("--plan-sha256", required=True)
    driver = commands.add_parser("_device-driver", help=argparse.SUPPRESS)
    driver.add_argument("--plan", required=True)
    arguments = parser.parse_args()
    try:
        if arguments.command == "check":
            print(json.dumps(check(), ensure_ascii=False, indent=2))
            return 0
        if arguments.command == "build": return build(arguments)
        if arguments.command == "device-plan": return prepare_device(arguments)
        if arguments.command == "device-run": return run_device(arguments)
        return device_module().run_driver(Path(arguments.plan))
    except (ValueError, OSError, ET.ParseError) as error:
        print(json.dumps({"schemaVersion": 1, "kind": "androidEntryError", "message": str(error)}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
