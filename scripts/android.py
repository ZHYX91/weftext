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

ROOT = Path(__file__).resolve().parents[1]
ANDROID = ROOT / "platforms" / "android"
GRADLE_VERSION = "8.13"
TASKS = [":host:assembleDebug", ":host:lintDebug", ":host:testDebugUnitTest", ":host:compileDebugAndroidTestKotlin"]


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
            if reason is None and (time.monotonic() - start >= 1500 or available.freePhysical <= 4 * 1024**3):
                reason = "deadline" if time.monotonic() - start >= 1500 else "hostMemoryGuard"
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
                terminate_job(job, 125)
                while True:
                    accounting = Accounting()
                    if not query_job(job, 1, ctypes.byref(accounting), ctypes.sizeof(accounting), None):
                        raise ctypes.WinError(ctypes.get_last_error())
                    if accounting.active == 0: break
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
    for key in ("GRADLE_OPTS", "JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS", "GRADLE_RO_DEP_CACHE"):
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
    record["sourcesUnchanged"] = checked["sources"] == check()["sources"] and record["entrySha256"] == digest(Path(__file__).resolve())
    if not record["sourcesUnchanged"]:
        code = 2
        record.update(state="failed", exitCode=code, compiled=False, failure="sourceChangedDuringBuild")
    record["artifacts"] = {path.relative_to(run).as_posix(): digest(path)
                           for path in (run / "build").rglob("*.aar")}
    record["logSha256"] = digest(run / "gradle.log")
    report.write_text(json.dumps(record, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"schemaVersion": 1, "kind": "androidHostBuild", "exitCode": code,
                      "report": str(report), "runtimeValidated": False}, ensure_ascii=False))
    return code


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("check", help="Check host source inventory without compiling or changing OS state")
    compiler = commands.add_parser("build", help="Build/lint/test only the real Android host AAR")
    for name in ("gradle-home", "sdk", "java-home", "state-root"):
        compiler.add_argument("--" + name, required=True)
    compiler.add_argument("--allow-dependency-downloads", action="store_true")
    arguments = parser.parse_args()
    try:
        if arguments.command == "check":
            print(json.dumps(check(), ensure_ascii=False, indent=2))
            return 0
        return build(arguments)
    except (ValueError, OSError, ET.ParseError) as error:
        print(json.dumps({"schemaVersion": 1, "kind": "androidEntryError", "message": str(error)}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
