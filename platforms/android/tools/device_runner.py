"""Internal OS-test driver for scripts/android.py; no standalone product entry.

prepare_plan performs no SDK execution. run_driver must be invoked by that entry
inside its exclusive Windows Job, never by an unguarded terminal command.
"""
from __future__ import annotations

import ctypes
from ctypes import wintypes
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import re
import socket
import struct
import subprocess
import time
import uuid
import zipfile

_REPOSITORY = Path(__file__).resolve().parents[3]
_ADB_PORT, _CONSOLE_PORT, _DEVICE_PORT = 5047, 5580, 5581
_SERIAL = "emulator-5580"
_IMAGE = "system-images;android-35;google_apis;x86_64"
_IDENTIFIER = re.compile(r"[A-Za-z_][A-Za-z_0-9]*(?:\.[A-Za-z_$][A-Za-z_0-9$]*)+")
_MAX_LOG = 16 * 1024 * 1024
_ENVIRONMENT_UNSET = (
    "ANDROID_AVD_HOME", "ANDROID_USER_HOME", "ANDROID_EMULATOR_HOME", "ANDROID_SDK_HOME", "ADB_VENDOR_KEYS",
    "ADB_SERVER_SOCKET", "ANDROID_ADB_SERVER_PORT", "ANDROID_SERIAL", "ADBHOST", "ADB_MDNS_AUTO_CONNECT",
    "ADB_LOCAL_TRANSPORT_MAX_PORT", "GRADLE_OPTS", "JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS",
    "ADB_TRACE", "ANDROID_LOG_TAGS", "ANDROID_VERBOSE", "ANDROID_DEBUG", "ANDROID_EMULATOR_DEBUG",
    "ANDROID_ADB_LOG_PATH", "ADB_LOG_PATH", "GRPC_TRACE", "QT_LOGGING_RULES", "ANDROID_PREFS_ROOT",
    "ANDROID_EMU_ENABLE_CRASH_REPORTING", "LOCALAPPDATA", "NETSIM_ARGS", "TMPDIR", "ANDROID_TMP",
)


class SdkReadOnlyFiles:
    """Metadata-only handles deny write/delete sharing; credential data is never read here."""
    def __init__(self):
        if os.name != "nt": raise ValueError("Windows SDK profile qualification is required")
        self.kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        self.handles: list[tuple[str, Path, int]] = []
        self.initial: list[dict] = []
        shell = ctypes.WinDLL("shell32", use_last_error=True)
        profile = ctypes.create_unicode_buffer(32768)
        folder = shell.SHGetFolderPathW
        folder.argtypes = [wintypes.HWND, ctypes.c_int, wintypes.HANDLE, wintypes.DWORD, wintypes.LPWSTR]
        folder.restype = ctypes.c_long
        if folder(None, 0x28, None, 0, profile) != 0: raise ValueError("Windows profile KnownFolder unavailable")
        paths = [("consoleToken", _path(Path(profile.value) / ".emulator_console_auth_token")),
                 ("adbUserKey", _path(Path(profile.value) / ".android" / "adbkey"))]
        self.absent_paths = [("adbUserPublicKey", Path(profile.value) / ".android" / "adbkey.pub"),
                             ("adbKnownHosts", Path(profile.value) / ".android" / "adb_known_hosts.pb")]
        create = self.kernel.CreateFileW
        create.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.c_void_p,
                           wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE]
        create.restype = wintypes.HANDLE
        try:
            for kind, path in paths:
                handle = create(str(path), 0x80, 1, None, 3, 0x00200000, None)
                if handle == ctypes.c_void_p(-1).value: raise ctypes.WinError(ctypes.get_last_error())
                self.handles.append((kind, path, handle))
            self.initial = self._metadata()
        except BaseException:
            self.close(); raise

    def _metadata(self) -> list[dict]:
        class Info(ctypes.Structure):
            _fields_ = [("attributes", wintypes.DWORD), ("created", wintypes.FILETIME),
                        ("accessed", wintypes.FILETIME), ("modified", wintypes.FILETIME),
                        ("volume", wintypes.DWORD), ("sizeHigh", wintypes.DWORD), ("sizeLow", wintypes.DWORD),
                        ("links", wintypes.DWORD), ("idHigh", wintypes.DWORD), ("idLow", wintypes.DWORD)]
        query = self.kernel.GetFileInformationByHandle
        query.argtypes = [wintypes.HANDLE, ctypes.POINTER(Info)]; query.restype = wintypes.BOOL
        result = []
        for kind, path, handle in self.handles:
            info = Info()
            if not query(handle, ctypes.byref(info)): raise ctypes.WinError(ctypes.get_last_error())
            size = (info.sizeHigh << 32) | info.sizeLow
            if info.attributes & (0x10 | 0x400) or not 1 <= size <= 16 * 1024:
                raise ValueError("SDK credential must already be a bounded nonempty regular non-reparse file")
            result.append({"kind": kind, "path": str(path), "volumeSerial": f"{info.volume:08x}",
                           "fileId": f"{info.idHigh:08x}{info.idLow:08x}", "bytes": size,
                           "createdFiletime100ns": str((info.created.dwHighDateTime << 32) | info.created.dwLowDateTime),
                           "lastWriteFiletime100ns": str((info.modified.dwHighDateTime << 32) | info.modified.dwLowDateTime),
                           "attributes": info.attributes, "links": info.links, "runnerReadsCredentialData": False,
                           "requestedAccess": "FILE_READ_ATTRIBUTES", "sharing": "FILE_SHARE_READ"})
        for kind, path in self.absent_paths:
            _path(path.parent)
            if os.path.lexists(path):
                raise ValueError("An unapproved global SDK public-key/known-host file exists")
            result.append({"kind": kind, "path": str(path), "exists": False, "runnerReadsCredentialData": False})
        return result

    def verify(self, expected: list[dict]) -> list[dict]:
        actual = self._metadata()
        if actual != self.initial or actual != expected:
            raise ValueError("Reviewed read-only SDK credential metadata changed")
        return actual

    def close(self) -> None:
        close = self.kernel.CloseHandle; close.argtypes = [wintypes.HANDLE]; close.restype = wintypes.BOOL
        for _, _, handle in self.handles: close(handle)
        self.handles = []

    def __enter__(self): return self
    def __exit__(self, kind, value, traceback): self.close()


def _utc() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat()


def _sha(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def _write_new(path: Path, data: dict) -> None:
    with path.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(data, stream, ensure_ascii=False, indent=2)
        stream.write("\n")


def _write(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def _path(value: Path | str, *, exists: bool = True) -> Path:
    path = Path(value)
    if not path.is_absolute() or ".." in path.parts:
        raise ValueError("An absolute non-traversing path is required")
    if any(c in str(path) for c in '&|<>^%!\r\n"'):
        raise ValueError("Shell metacharacters are not permitted in SDK/state paths")
    for component in (path, *path.parents):
        if component.exists() or component.is_symlink():
            info = component.lstat()
            if component.is_symlink() or getattr(info, "st_file_attributes", 0) & 0x400:
                raise ValueError("Reparse/symlink path rejected: " + str(component))
    if exists and not path.exists():
        raise ValueError("Required path is absent: " + str(path))
    return path.resolve(strict=exists)


def _properties(path: Path) -> dict[str, str]:
    content = path.read_text(encoding="utf-8")
    if len(content) > 128 * 1024:
        raise ValueError("Oversized SDK properties")
    return {key.strip(): value.strip() for line in content.splitlines()
            if "=" in line and not line.lstrip().startswith("#")
            for key, value in [line.split("=", 1)]}


def _reserve_ports() -> list[socket.socket]:
    held: list[socket.socket] = []
    try:
        for port in (_ADB_PORT, _CONSOLE_PORT, _DEVICE_PORT):
            listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            held.append(listener)
            if hasattr(socket, "SO_EXCLUSIVEADDRUSE"):
                listener.setsockopt(socket.SOL_SOCKET, socket.SO_EXCLUSIVEADDRUSE, 1)
            listener.bind(("127.0.0.1", port))
            listener.listen(1)
        return held
    except BaseException:
        for listener in held:
            listener.close()
        raise


def _release_ports(held: list[socket.socket]) -> None:
    for listener in held:
        listener.close()


def _strings(data: bytes, offset: int, size: int, header: int) -> list[str]:
    if header < 28 or size < header:
        raise ValueError("Invalid APK string pool")
    count, style_count, flags, start, styles_start = struct.unpack_from("<IIIII", data, offset + 8)
    if count > 100_000 or header + 4 * (count + style_count) > size:
        raise ValueError("Invalid APK string offsets")
    end = styles_start if styles_start else size
    if not header <= start <= end <= size:
        raise ValueError("Invalid APK string region")
    result: list[str] = []
    for index in range(count):
        relative, = struct.unpack_from("<I", data, offset + header + index * 4)
        cursor = offset + start + relative
        boundary = offset + end
        if not offset + start <= cursor < boundary:
            raise ValueError("Invalid APK string address")

        def length8() -> int:
            nonlocal cursor
            if cursor >= boundary: raise ValueError("Truncated UTF-8 string")
            value = data[cursor]; cursor += 1
            if value & 0x80:
                if cursor >= boundary: raise ValueError("Truncated UTF-8 length")
                value = ((value & 0x7f) << 8) | data[cursor]; cursor += 1
            return value

        def length16() -> int:
            nonlocal cursor
            if cursor + 2 > boundary: raise ValueError("Truncated UTF-16 string")
            value, = struct.unpack_from("<H", data, cursor); cursor += 2
            if value & 0x8000:
                if cursor + 2 > boundary: raise ValueError("Truncated UTF-16 length")
                second, = struct.unpack_from("<H", data, cursor); cursor += 2
                value = ((value & 0x7fff) << 16) | second
            return value

        if flags & 0x100:
            units, length = length8(), length8()
            if cursor + length + 1 > boundary or data[cursor + length] != 0:
                raise ValueError("Truncated APK UTF-8 content")
            value = data[cursor:cursor + length].decode("utf-8", errors="strict")
            if len(value.encode("utf-16-le")) // 2 != units:
                raise ValueError("APK string length mismatch")
        else:
            units = length16(); length = units * 2
            if cursor + length + 2 > boundary or data[cursor + length:cursor + length + 2] != b"\0\0":
                raise ValueError("Truncated APK UTF-16 content")
            value = data[cursor:cursor + length].decode("utf-16-le", errors="strict")
        result.append(value)
    return result


def _apk_manifest(apk: Path) -> dict[str, str]:
    """Pure bounded preview only; aapt2 independently verifies it in the driver."""
    if apk.stat().st_size > 256 * 1024 * 1024:
        raise ValueError("Instrumentation APK budget exceeded")
    with zipfile.ZipFile(apk) as archive:
        manifests = [item for item in archive.infolist() if item.filename == "AndroidManifest.xml"]
        if len(manifests) != 1 or manifests[0].file_size > 1024 * 1024:
            raise ValueError("One bounded actual APK Manifest is required")
        data = archive.read(manifests[0])
    if len(data) < 8 or struct.unpack_from("<HHI", data, 0) != (3, 8, len(data)):
        raise ValueError("Actual binary Android Manifest required")
    pool: list[str] | None = None
    package: str | None = None
    instruments: list[dict[str, str]] = []
    cursor = 8
    while cursor < len(data):
        if cursor + 8 > len(data): raise ValueError("Truncated Manifest chunk")
        kind, header, size = struct.unpack_from("<HHI", data, cursor)
        if header < 8 or size < header or cursor + size > len(data):
            raise ValueError("Invalid Manifest chunk")
        if kind == 1:
            if pool is not None: raise ValueError("Multiple Manifest string pools")
            pool = _strings(data, cursor, size, header)
        elif kind == 0x102:
            if pool is None or header != 16 or size < 36:
                raise ValueError("Invalid Manifest element")
            name_index, = struct.unpack_from("<I", data, cursor + 20)
            start, stride, count = struct.unpack_from("<HHH", data, cursor + 24)
            if name_index >= len(pool) or start < 20 or stride < 20 or 16 + start + count * stride > size:
                raise ValueError("Invalid Manifest attributes")
            attributes: dict[str, str] = {}
            for index in range(count):
                attr = cursor + 16 + start + index * stride
                namespace, name, raw, value_size, reserved, value_type, value = struct.unpack_from("<IIIHBBI", data, attr)
                if name >= len(pool) or value_size != 8 or reserved:
                    raise ValueError("Invalid Manifest attribute")
                if namespace != 0xffffffff and (namespace >= len(pool) or pool[namespace] != "http://schemas.android.com/apk/res/android"):
                    continue
                if raw != 0xffffffff:
                    if raw >= len(pool): raise ValueError("Invalid Manifest raw value")
                    text = pool[raw]
                elif value_type == 3:
                    if value >= len(pool): raise ValueError("Invalid Manifest string value")
                    text = pool[value]
                else:
                    continue
                if pool[name] in attributes: raise ValueError("Duplicate Manifest attribute")
                attributes[pool[name]] = text
            if pool[name_index] == "manifest":
                if package is not None: raise ValueError("Multiple Manifest roots")
                package = attributes.get("package")
            elif pool[name_index] == "instrumentation":
                instruments.append(attributes)
        cursor += size
    if package is None or not _IDENTIFIER.fullmatch(package) or len(instruments) != 1:
        raise ValueError("One actual instrumentation package/runner required")
    runner = instruments[0].get("name", "")
    runner = package + runner if runner.startswith(".") else runner if "." in runner else package + "." + runner
    target = instruments[0].get("targetPackage", "")
    if not _IDENTIFIER.fullmatch(runner) or target != package:
        raise ValueError("This library OS test requires a self-targeting instrumentation APK")
    return {"package": package, "runner": runner, "targetPackage": target}


def _sdk_inputs(sdk: Path, java_home: Path) -> dict[str, str]:
    paths = [sdk / "platform-tools/adb.exe", sdk / "platform-tools/source.properties",
             sdk / "emulator/emulator.exe", sdk / "emulator/source.properties",
             sdk / "cmdline-tools/latest/bin/avdmanager.bat", sdk / "cmdline-tools/latest/source.properties",
             sdk / "build-tools/35.0.0/aapt2.exe", sdk / "build-tools/35.0.0/source.properties",
             sdk / "system-images/android-35/google_apis/x86_64/source.properties",
             sdk / "system-images/android-35/google_apis/x86_64/package.xml",
             sdk / "system-images/android-35/google_apis/x86_64/system.img",
             sdk / "system-images/android-35/google_apis/x86_64/vendor.img",
             sdk / "system-images/android-35/google_apis/x86_64/ramdisk.img",
             sdk / "system-images/android-35/google_apis/x86_64/kernel-ranchu",
             sdk / "system-images/android-35/google_apis/x86_64/encryptionkey.img",
             sdk / "system-images/android-35/google_apis/x86_64/data/empty_data_disk",
             sdk / "system-images/android-35/google_apis/x86_64/advancedFeatures.ini",
             sdk / "emulator/qemu/windows-x86_64/qemu-system-x86_64-headless.exe",
             java_home / "bin/java.exe", java_home / "release"]
    for path in paths:
        if not _path(path).is_file(): raise ValueError("Required installed SDK/JDK file absent")
    image = _properties(paths[8])
    if image.get("AndroidVersion.ApiLevel") != "35" or image.get("SystemImage.Abi") != "x86_64" or image.get("SystemImage.TagId") != "google_apis":
        raise ValueError("Installed API 35 Google APIs x86_64 image required")
    factory_marker = sdk / "system-images/android-35/google_apis/x86_64/data/empty_data_disk"
    if "keep the data partition empty" not in factory_marker.read_text(encoding="utf-8"):
        raise ValueError("The selected official image's empty-data factory marker is unrecognized")
    if not re.search(r'^JAVA_VERSION="21(?:\.|\")', paths[-1].read_text(encoding="utf-8"), re.MULTILINE):
        raise ValueError("The explicit JDK must be version 21")
    # Hash binaries and small metadata; large immutable image sizes are retained without rereading GiB.
    return {str(path): _sha(path) for path in paths if path.suffix != ".img"}


def _commands(run: Path, sdk: Path, apk: Path, manifest: dict, name: str, test_class: str) -> dict[str, list[str]]:
    adb = str(sdk / "platform-tools/adb.exe")
    device = [adb, "-P", str(_ADB_PORT), "-s", _SERIAL]
    return {
        "verifyManifest": [str(sdk / "build-tools/35.0.0/aapt2.exe"), "dump", "xmltree", str(apk), "--file", "AndroidManifest.xml"],
        "createTestKey": [adb, "keygen", str(run / "emulator-home" / "adbkey")],
        "createAvd": [str(sdk / "cmdline-tools/latest/bin/avdmanager.bat"), "create", "avd", "--name", name,
                      "--package", _IMAGE, "--device", "pixel_6", "--path", str(run / "avd")],
        # The private server must neither claim an unrelated USB device nor scan other AVD ports.
        "adbServer": [adb, "--one-device", "weftext-no-usb-" + name.removeprefix("weftext-android-os-"),
                      "-L", "tcp:127.0.0.1:5047", "nodaemon", "server"],
        "emulator": [str(sdk / "emulator/emulator.exe"), "-avd", name, "-no-window", "-no-audio", "-no-boot-anim",
                     "-no-snapshot", "-no-metrics", "-gpu", "swiftshader", "-memory", "2048", "-cores", "2", "-port", "5580",
                     "-feature", "-BluetoothEmulation", "-feature", "-Uwb", "-feature", "-WiFiPacketStream"],
        "boot": [*device, "shell", "getprop", "sys.boot_completed"],
        "avdIdentity": [*device, "shell", "getprop", "ro.boot.qemu.avd_name"],
        "install": [*device, "install", "--no-streaming", "-t", str(apk)],
        "instrument": [*device, "shell", "am", "instrument", "-w", "-r", "-e", "class", test_class,
                       manifest["package"] + "/" + manifest["runner"]],
        "stopEmulator": [*device, "emu", "kill"],
        "stopAdbServer": [adb, "-P", str(_ADB_PORT), "kill-server"],
    }


def _environment(run: Path, sdk: Path, java: Path) -> dict[str, str]:
    return {"ANDROID_AVD_HOME": str(run / "avd-home"), "ANDROID_USER_HOME": str(run / "android-user-home"),
            "ANDROID_EMULATOR_HOME": str(run / "emulator-home"), "ANDROID_ADB_SERVER_PORT": "5047",
            "ADB_SERVER_SOCKET": "tcp:127.0.0.1:5047", "ADB_MDNS_AUTO_CONNECT": "0",
            "ADB_LOCAL_TRANSPORT_MAX_PORT": "0", "JAVA_HOME": str(java),
            "ANDROID_HOME": str(sdk), "ANDROID_SDK_ROOT": str(sdk),
            "ANDROID_ADB_LOG_PATH": str(run / "adb-sdk.log"), "ANDROID_EMU_ENABLE_CRASH_REPORTING": "0",
            "ADB_VENDOR_KEYS": str(run / "emulator-home" / "adbkey"),
            "TEMP": str(run / "temp"), "TMP": str(run / "temp"),
            "TMPDIR": str(run / "temp"), "ANDROID_TMP": str(run / "temp")}


def _build_proof(result_path: Path, apk: Path) -> tuple[dict, str]:
    if result_path.stat().st_size > 1024 * 1024: raise ValueError("Build result budget exceeded")
    result = json.loads(result_path.read_text(encoding="utf-8"))
    run = _path(result["runDirectory"])
    if result_path.parent != run or result.get("state") != "passed" or result.get("exitCode") != 0 or not result.get("sourcesUnchanged"):
        raise ValueError("A successful immutable actual Android build result is required")
    matches = [sha for name, sha in result.get("artifacts", {}).items() if _path(run / name) == apk]
    if len(matches) != 1 or not apk.is_relative_to(run) or matches[0] != _sha(apk):
        raise ValueError("Exact APK path and SHA must come from the actual build artifact map")
    if ":host:assembleDebugAndroidTest" not in result.get("tasks", []):
        raise ValueError("The mapped APK must have been assembled by the product entry")
    if not result.get("sources") or len(result["sources"]) > 256:
        raise ValueError("The actual build source manifest is absent or oversized")
    for name, digest in result["sources"].items():
        source = _path(_REPOSITORY / name)
        if not source.is_relative_to(_REPOSITORY) or _sha(source) != digest:
            raise ValueError("Compiled Android source no longer matches its actual build")
    if result.get("entrySha256") != _sha(_REPOSITORY / "scripts/android.py"):
        raise ValueError("Compiled product entry no longer matches its actual build")
    return result, matches[0]


def _versions(sdk: Path, java_home: Path) -> dict:
    paths = ("platform-tools/source.properties", "emulator/source.properties", "cmdline-tools/latest/source.properties",
             "build-tools/35.0.0/source.properties", "system-images/android-35/google_apis/x86_64/source.properties")
    result = {name: _properties(sdk / name) for name in paths}
    if result["build-tools/35.0.0/source.properties"].get("Pkg.Revision") != "35.0.0":
        raise ValueError("Installed build tools must actually be 35.0.0")
    result["jdkRelease"] = (java_home / "release").read_text(encoding="utf-8")
    result["systemImageFiles"] = {name: {"bytes": path.stat().st_size, "mtimeNs": path.stat().st_mtime_ns,
        "device": path.stat().st_dev, "fileId": path.stat().st_ino}
        for name in ("system.img", "vendor.img", "ramdisk.img", "kernel-ranchu", "encryptionkey.img")
        for path in [sdk / "system-images/android-35/google_apis/x86_64" / name]}
    result["factoryDataMode"] = "emptyDataDiskMarker"
    return result


def prepare_plan(*, workspace: Path, repository: Path, sdk: Path, java_home: Path,
                 state_root: Path, apk: Path, build_result: Path, test_class: str,
                 expected_test_count: int) -> dict:
    """Pure preflight plus create-new plan. Does not execute any SDK command."""
    workspace, repository = _path(workspace), _path(repository)
    if repository != _REPOSITORY or not (workspace / "repos/weftext/README.md").is_file() or not (workspace / "infra/README.md").is_file():
        raise ValueError("This repository's independent Weftext workspace is required")
    sdk, java_home = _path(sdk), _path(java_home)
    state_root = _path(state_root, exists=False)
    if not state_root.is_relative_to(workspace / ".workspace/build") or state_root == workspace / ".workspace/build":
        raise ValueError("Owned device state must be below this workspace's .workspace/build")
    if not _IDENTIFIER.fullmatch(test_class) or not 1 <= expected_test_count <= 128:
        raise ValueError("An exact test class and bounded expected test count are required")
    apk, build_result = _path(apk), _path(build_result)
    result, apk_sha = _build_proof(build_result, apk)
    if not _path(result["runDirectory"]).is_relative_to(workspace / ".workspace/build"):
        raise ValueError("Build proof must belong to this workspace")
    tool_inputs = _sdk_inputs(sdk, java_home)
    manifest = _apk_manifest(apk)
    with SdkReadOnlyFiles() as sdk_files:
        sdk_readonly_files = sdk_files.verify(sdk_files.initial)
    held = _reserve_ports(); _release_ports(held)
    identity = str(uuid.uuid4())
    run = state_root / identity
    state_root.mkdir(parents=True, exist_ok=True)
    run.mkdir(exist_ok=False)
    name = "weftext-android-os-" + identity
    plan = {"schemaVersion": 1, "kind": "androidOwnedOsTestPlan", "createdUtc": _utc(),
            "id": identity, "workspace": str(workspace), "repository": str(repository), "runDirectory": str(run),
            "sdk": str(sdk), "javaHome": str(java_home), "apk": str(apk), "apkSha256": apk_sha,
            "buildResult": str(build_result), "buildResultSha256": _sha(build_result),
            "buildSources": result["sources"], "toolInputs": tool_inputs, "toolVersions": _versions(sdk, java_home),
            "helperSha256": _sha(Path(__file__)), "manifest": manifest,
            "sdkReadOnlyFiles": sdk_readonly_files,
            "testKey": {"privatePath": str(run / "emulator-home" / "adbkey"),
                        "publicPath": str(run / "emulator-home" / "adbkey.pub"),
                        "createOnce": True, "runnerReadsPrivateKeyData": False,
                        "unknownResult": "retainAndReconcileMetadataWithoutRetry"},
            "consoleGreeting": {"host": "127.0.0.1", "port": _CONSOLE_PORT, "maxBytes": 4096,
                                "requiredFirstLine": "Android Console: Authentication required"},
            "testClass": test_class, "expectedTestCount": expected_test_count,
            "avdName": name, "systemImage": _IMAGE, "serial": _SERIAL,
            "ports": {"adbServer": _ADB_PORT, "console": _CONSOLE_PORT, "device": _DEVICE_PORT},
            "environment": _environment(run, sdk, java_home),
            "environmentUnset": list(_ENVIRONMENT_UNSET),
            "createAvdStdin": "no\n", "limits": {"externalWindowsJobRequired": True, "deadlineSeconds": 1500,
            "jobMemoryBytes": 4 * 1024**3, "hostFreeMinimumBytes": 4 * 1024**3, "guestMemoryMiB": 2048, "guestCores": 2},
            "commands": _commands(run, sdk, apk, manifest, name, test_class),
            "runtimeValidated": False, "scope": "hostLibraryOsMechanismsOnly"}
    plan_path = run / "plan.json"
    _write_new(plan_path, plan)
    return {"planPath": str(plan_path), "planSha256": _sha(plan_path), "plan": plan}


def _owned_job_pids() -> set[int]:
    if os.name != "nt": raise ValueError("Exclusive Windows Job supervision is required")
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    query = kernel.QueryInformationJobObject
    query.argtypes = [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD, ctypes.c_void_p]
    query.restype = wintypes.BOOL
    class BasicLimits(ctypes.Structure):
        _fields_ = [("processTime", ctypes.c_int64), ("jobTime", ctypes.c_int64), ("flags", wintypes.DWORD),
                    ("minWorking", ctypes.c_size_t), ("maxWorking", ctypes.c_size_t), ("activeLimit", wintypes.DWORD),
                    ("affinity", ctypes.c_size_t), ("priority", wintypes.DWORD), ("scheduling", wintypes.DWORD)]
    class IoCounters(ctypes.Structure):
        _fields_ = [(name, ctypes.c_uint64) for name in ("readOps", "writeOps", "otherOps", "read", "write", "other")]
    class ExtendedLimits(ctypes.Structure):
        _fields_ = [("basic", BasicLimits), ("io", IoCounters), ("processMemory", ctypes.c_size_t),
                    ("jobMemory", ctypes.c_size_t), ("peakProcess", ctypes.c_size_t), ("peakJob", ctypes.c_size_t)]
    limits = ExtendedLimits()
    if not query(None, 9, ctypes.byref(limits), ctypes.sizeof(limits), None):
        raise ctypes.WinError(ctypes.get_last_error())
    if limits.basic.flags & (0x2000 | 0x200) != (0x2000 | 0x200) or limits.basic.flags & (0x800 | 0x1000) or limits.jobMemory != 4 * 1024**3:
        raise ValueError("Driver requires actual kill-on-close/non-breakaway/4-GiB Job limits")
    buffer = ctypes.create_string_buffer(8 + ctypes.sizeof(ctypes.c_size_t) * 4096)
    if not query(None, 3, buffer, len(buffer), None): raise ctypes.WinError(ctypes.get_last_error())
    assigned, count = struct.unpack_from("<II", buffer.raw)
    if assigned != count or count > 4096: raise ValueError("Owned Job process list was not complete")
    item = "<Q" if ctypes.sizeof(ctypes.c_size_t) == 8 else "<I"
    values = {struct.unpack_from(item, buffer.raw, 8 + i * ctypes.sizeof(ctypes.c_size_t))[0] for i in range(count)}
    if os.getpid() not in values: raise ValueError("Driver is not in the supervising Job")
    return values


def _port_owners(port: int) -> set[int]:
    library = ctypes.WinDLL("iphlpapi", use_last_error=True)
    query = library.GetExtendedTcpTable
    query.argtypes = [ctypes.c_void_p, ctypes.POINTER(wintypes.DWORD), wintypes.BOOL, wintypes.ULONG, ctypes.c_int, wintypes.ULONG]
    query.restype = wintypes.DWORD
    size = wintypes.DWORD()
    if query(None, ctypes.byref(size), False, socket.AF_INET, 3, 0) not in (0, 122):
        raise ValueError("Unable to inspect TCP listener ownership")
    buffer = ctypes.create_string_buffer(size.value)
    if query(buffer, ctypes.byref(size), False, socket.AF_INET, 3, 0) != 0:
        raise ValueError("Unable to inspect TCP listener ownership")
    count, = struct.unpack_from("<I", buffer.raw)
    if 4 + count * 24 > len(buffer): raise ValueError("Truncated TCP listener table")
    return {row[5] for index in range(count)
            for row in [struct.unpack_from("<IIIIII", buffer.raw, 4 + index * 24)]
            if socket.ntohs(row[2] & 0xffff) == port}


def _process_identity(pid: int) -> dict:
    """Read exact live owned process creation time and executable, never argv or memory."""
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    open_process = kernel.OpenProcess
    open_process.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    open_process.restype = wintypes.HANDLE
    query = kernel.QueryFullProcessImageNameW
    query.argtypes = [wintypes.HANDLE, wintypes.DWORD, wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD)]
    query.restype = wintypes.BOOL
    times = kernel.GetProcessTimes
    times.argtypes = [wintypes.HANDLE, *[ctypes.POINTER(wintypes.FILETIME)] * 4]
    times.restype = wintypes.BOOL
    close = kernel.CloseHandle
    close.argtypes = [wintypes.HANDLE]; close.restype = wintypes.BOOL
    handle = open_process(0x1000, False, pid)
    if not handle: raise ctypes.WinError(ctypes.get_last_error())
    try:
        path, length = ctypes.create_unicode_buffer(32768), wintypes.DWORD(32768)
        created, exited, system, user = (wintypes.FILETIME() for _ in range(4))
        if not query(handle, 0, path, ctypes.byref(length)) or not times(handle, ctypes.byref(created), ctypes.byref(exited), ctypes.byref(system), ctypes.byref(user)):
            raise ctypes.WinError(ctypes.get_last_error())
        return {"pid": pid, "executable": str(_path(path.value)),
                "createdFiletime100ns": str((created.dwHighDateTime << 32) | created.dwLowDateTime)}
    finally:
        close(handle)


def _system_directory() -> Path:
    query = ctypes.WinDLL("kernel32", use_last_error=True).GetSystemDirectoryW
    query.argtypes = [wintypes.LPWSTR, wintypes.UINT]; query.restype = wintypes.UINT
    value = ctypes.create_unicode_buffer(32768)
    size = query(value, len(value))
    if not size or size >= len(value): raise ctypes.WinError(ctypes.get_last_error())
    return _path(value.value)


def _verify_dump(text: str, expected: dict[str, str]) -> None:
    package = None
    instrument: dict[str, str] = {}
    in_instrument = False
    instrument_indent = 0
    for line in text.splitlines():
        element = re.match(r"(\s*)E: (\S+)", line)
        if element:
            if in_instrument and len(element[1]) <= instrument_indent: in_instrument = False
            if element[2] == "instrumentation":
                if instrument: raise ValueError("SDK reported multiple instrumentation elements")
                in_instrument = True; instrument_indent = len(element[1])
        attr = re.match(r'\s*A: (?:android:)?([A-Za-z]+)(?:\(0x[0-9a-fA-F]+\))?="([^"\r\n]*)"', line)
        if attr:
            if attr[1] == "package": package = attr[2]
            if in_instrument and attr[1] in ("name", "targetPackage"): instrument[attr[1]] = attr[2]
    runner = instrument.get("name", "")
    runner = str(package) + runner if runner.startswith(".") else runner if "." in runner else str(package) + "." + runner
    actual = {"package": package, "runner": runner, "targetPackage": instrument.get("targetPackage")}
    if actual != expected:
        raise ValueError("SDK Manifest observation disagrees with the reviewed exact plan")


def _instrumentation(text: str, expected_class: str, expected_count: int) -> dict:
    statuses: list[dict] = []
    current: dict[str, str] = {}
    terminal: list[int] = []
    for line in text.splitlines():
        if line.startswith("INSTRUMENTATION_STATUS: ") and "=" in line:
            key, value = line[len("INSTRUMENTATION_STATUS: "):].split("=", 1); current[key] = value
        elif line.startswith("INSTRUMENTATION_STATUS_CODE: "):
            code = int(line.split(":", 1)[1]); statuses.append({"code": code, "values": current}); current = {}
        elif line.startswith("INSTRUMENTATION_CODE: "):
            terminal.append(int(line.split(":", 1)[1]))
        elif line.startswith(("INSTRUMENTATION_FAILED", "INSTRUMENTATION_ABORTED")) or "FAILURES!!!" in line:
            raise ValueError("Instrumentation reported failure")
    completed = [status["values"] for status in statuses if status["code"] == 0]
    names = {(value.get("class"), value.get("test")) for value in completed}
    if terminal != [-1] or len(completed) != expected_count or len(names) != expected_count:
        raise ValueError("Instrumentation terminal/test-count mismatch")
    if any(status["code"] < 0 for status in statuses): raise ValueError("Instrumentation test failed/skipped")
    if any(value.get("class") != expected_class or not value.get("test") or int(value.get("numtests", "0")) != expected_count for value in completed):
        raise ValueError("Instrumentation class/count does not match reviewed plan")
    if re.search(r"INSTRUMENTATION_RESULT: (?:shortMsg|longMsg|Error)=", text):
        raise ValueError("Instrumentation process reported an error")
    return {"tests": expected_count, "failures": 0, "testNames": sorted(name[1] for name in names),
            "terminalCode": -1, "statusEvents": statuses}


class _Run:
    def __init__(self, plan: dict, environment: dict[str, str]):
        self.plan, self.environment = plan, environment
        self.root = Path(plan["runDirectory"])
        self.record = {"schemaVersion": 1, "kind": "androidOwnedOsTest", "startedUtc": _utc(),
                       "scope": "hostLibraryOsMechanismsOnly", "runtimeValidated": False,
                       "state": "running", "steps": [], "children": []}
        self.processes: list[tuple[subprocess.Popen, dict]] = []
        self.base_job_pids = _owned_job_pids()
        self.record["initialOwnedJobPids"] = sorted(self.base_job_pids)
        self.server: subprocess.Popen | None = None
        self.emulator: subprocess.Popen | None = None
        self._save()

    def _save(self) -> None:
        _write(self.root / "result.json", self.record)

    def spawn(self, label: str, command: list[str], *, input_bytes: bytes | None = None) -> tuple[subprocess.Popen, dict]:
        prefix = f"{len(self.record['steps']):03d}-{label}"
        out, err = self.root / (prefix + ".stdout"), self.root / (prefix + ".stderr")
        executable = None
        launched: list[str] | str = command
        if Path(command[0]).suffix.lower() == ".bat":
            # .bat execution has one explicit fixed system-shell boundary. SDK/state
            # paths already reject metacharacters; all other arguments are fixed/typed.
            executable = str(_system_directory() / "cmd.exe")
            launched = '"' + executable + '" /d /s /c "' + subprocess.list2cmdline(command) + '"'
        with out.open("xb") as stdout, err.open("xb") as stderr:
            process = subprocess.Popen(launched, executable=executable, cwd=self.root, env=self.environment,
                                       stdin=subprocess.PIPE if input_bytes is not None else subprocess.DEVNULL,
                                       stdout=stdout, stderr=stderr, shell=False,
                                       creationflags=subprocess.CREATE_NO_WINDOW)
        step = {"label": label, "argv": command, "pid": process.pid, "startedUtc": _utc(),
                "stdout": str(out), "stderr": str(err), "state": "running",
                "creationFlags": ["CREATE_NO_WINDOW"], "launcherExecutable": executable,
                "windowsCommandLine": launched if isinstance(launched, str) else subprocess.list2cmdline(launched)}
        self.processes.append((process, step)); self.record["steps"].append(step); self._save()
        if input_bytes is not None:
            assert process.stdin is not None
            process.stdin.write(input_bytes); process.stdin.close()
        return process, step

    def wait(self, process: subprocess.Popen, step: dict, seconds: int, *, required: bool = True) -> str:
        deadline = time.monotonic() + seconds
        while process.poll() is None:
            if any(Path(step[name]).stat().st_size > _MAX_LOG for name in ("stdout", "stderr")):
                step["state"] = "logBudgetExceeded"; self._save(); raise ValueError("SDK output exceeded its budget")
            if time.monotonic() >= deadline:
                step["state"] = "timeout"; self._save(); raise TimeoutError("SDK stage deadline: " + step["label"])
            time.sleep(0.25)
        step.update(state="exited", exitCode=process.returncode, endedUtc=_utc(),
                    stdoutSha256=_sha(Path(step["stdout"])), stderrSha256=_sha(Path(step["stderr"])))
        self._save()
        for name in ("stdout", "stderr"):
            if Path(step[name]).stat().st_size > _MAX_LOG: raise ValueError("SDK output budget exceeded")
        if required and process.returncode != 0: raise ValueError("SDK stage failed: " + step["label"])
        return Path(step["stdout"]).read_text(encoding="utf-8", errors="strict") if required else ""

    def command(self, label: str, seconds: int = 30, *, required: bool = True) -> str:
        process, step = self.spawn(label, self.plan["commands"][label])
        return self.wait(process, step, seconds, required=required)

    def test_key_metadata(self) -> list[dict]:
        """Own test-key identity only, never open/read/hash private key contents."""
        result = []
        for kind, name in (("private", "adbkey"), ("public", "adbkey.pub")):
            path = self.root / "emulator-home" / name
            if not os.path.lexists(path):
                result.append({"kind": kind, "path": str(path), "exists": False})
                continue
            _path(path)
            info = path.stat()
            if not path.is_file() or not 1 <= info.st_size <= 16 * 1024:
                raise ValueError("Owned test-key output is not a bounded regular file")
            result.append({"kind": kind, "path": str(path), "exists": True, "bytes": info.st_size,
                           "device": info.st_dev, "fileId": info.st_ino,
                           "mtimeNs": info.st_mtime_ns, "mode": info.st_mode,
                           "attributes": getattr(info, "st_file_attributes", 0),
                           "runnerReadsPrivateKeyData": False})
        self.record["testKeyMetadata"] = result
        self._save()
        return result

    def create_test_key(self) -> None:
        if any(os.path.lexists(self.root / "emulator-home" / name) for name in ("adbkey", "adbkey.pub")):
            raise ValueError("Owned test-key paths must both be absent before their single keygen attempt")
        try:
            self.command("createTestKey", 30)
        finally:
            metadata = self.test_key_metadata()
        if not all(item["exists"] for item in metadata):
            raise ValueError("Owned test-key output pair is incomplete; no retry or replacement is permitted")

    def own_server(self) -> None:
        if self.server is None or self.server.poll() is not None or _port_owners(_ADB_PORT) != {self.server.pid}:
            raise ValueError("Private foreground ADB server ownership is unconfirmed")

    def own_device(self) -> None:
        self.own_server()
        owned = _owned_job_pids()
        for port in (_CONSOLE_PORT, _DEVICE_PORT):
            pids = _port_owners(port)
            if not pids or not pids.issubset(owned): raise ValueError("Emulator listener is not in this owned Job")
            for pid in pids:
                identity = _process_identity(pid)
                expected = _path(Path(self.plan["sdk"]) / "emulator/qemu/windows-x86_64/qemu-system-x86_64-headless.exe")
                if _path(identity["executable"]) != expected:
                    raise ValueError("Owned emulator listener is not the reviewed official headless executable")
                if identity not in self.record["children"]:
                    self.record["children"].append(identity)
                    self._save()

    def adb(self, label: str, seconds: int = 30, *, required: bool = True) -> str:
        self.own_device()
        return self.command(label, seconds, required=required)

    def verify_console_auth(self) -> None:
        self.own_device()
        received = bytearray()
        with socket.create_connection(("127.0.0.1", _CONSOLE_PORT), timeout=5) as console:
            while b"\n" not in received and len(received) < 4096:
                block = console.recv(min(512, 4096 - len(received)))
                if not block: break
                received.extend(block)
        first = bytes(received).split(b"\n", 1)[0].rstrip(b"\r")
        if first != b"Android Console: Authentication required":
            raise ValueError("Owned console did not confirm nonempty credential authentication")
        # Never read the credential file, send an auth command, or retain the greeting's path text.
        self.record["consoleAuthenticationRequired"] = True
        self._save()

    def stop(self) -> bool:
        errors: list[str] = []
        if self.emulator is not None:
            try: self.adb("stopEmulator", 20)
            except (ValueError, OSError, TimeoutError) as error: errors.append(str(error))
            try: self.emulator.wait(timeout=30)
            except subprocess.TimeoutExpired: errors.append("Owned emulator exit was not observed")
        if self.server is not None and self.server.poll() is None:
            try: self.own_server(); self.command("stopAdbServer", 15)
            except (ValueError, OSError, TimeoutError) as error: errors.append(str(error))
            try: self.server.wait(timeout=20)
            except subprocess.TimeoutExpired: errors.append("Owned foreground ADB exit was not observed")
        remaining = []
        for process, step in self.processes:
            if process.poll() is None: remaining.append({"pid": process.pid, "label": step["label"]})
            elif step["state"] != "exited":
                self.wait(process, step, 0, required=False)
        self.record.update(cleanupErrors=errors, unconfirmedChildren=remaining)
        deadline = time.monotonic() + 20
        job_remaining = _owned_job_pids() - self.base_job_pids
        while job_remaining and time.monotonic() < deadline:
            time.sleep(0.25)
            job_remaining = _owned_job_pids() - self.base_job_pids
        self.record["unconfirmedJobPids"] = sorted(job_remaining)
        self._save()
        return not errors and not remaining and not job_remaining


def validate_plan(plan_path: Path | str, *, allow_supervisor_files: bool = False) -> dict:
    """Pure one-shot plan validation, also usable by the outer entry before its Job/log exist."""
    plan_path = _path(plan_path)
    if plan_path.stat().st_size > 1024 * 1024: raise ValueError("Plan budget exceeded")
    plan = json.loads(plan_path.read_text(encoding="utf-8"))
    if plan.get("schemaVersion") != 1 or plan.get("kind") != "androidOwnedOsTestPlan": raise ValueError("Invalid OS test plan")
    repository, workspace = _path(plan["repository"]), _path(plan["workspace"])
    run, sdk, java = _path(plan["runDirectory"]), _path(plan["sdk"]), _path(plan["javaHome"])
    if repository != _REPOSITORY or not (workspace / "repos/weftext/README.md").is_file(): raise ValueError("Wrong repository/workspace")
    identity = str(uuid.UUID(plan["id"]))
    if identity != plan["id"] or run.name != identity or plan_path != run / "plan.json" or not run.is_relative_to(workspace / ".workspace/build"):
        raise ValueError("Unowned plan path")
    allowed = {"plan.json", "owned-tree-result.json", "owned-tree.log"} if allow_supervisor_files else {"plan.json"}
    if (run / "result.json").exists() or any(path.name not in allowed for path in run.iterdir()):
        raise ValueError("A prepared plan is one-shot; prior runtime state cannot be reused")
    if not _IDENTIFIER.fullmatch(plan["testClass"]) or not 1 <= plan["expectedTestCount"] <= 128:
        raise ValueError("Invalid planned tests")
    apk, build = _path(plan["apk"]), _path(plan["buildResult"])
    result, apk_sha = _build_proof(build, apk)
    manifest = _apk_manifest(apk)
    if apk_sha != plan["apkSha256"] or _sha(build) != plan["buildResultSha256"] or result["sources"] != plan["buildSources"] or manifest != plan["manifest"]:
        raise ValueError("Reviewed build/APK identity changed")
    if _sdk_inputs(sdk, java) != plan["toolInputs"] or _versions(sdk, java) != plan["toolVersions"]:
        raise ValueError("Reviewed SDK/JDK inputs changed")
    if _sha(Path(__file__)) != plan["helperSha256"]: raise ValueError("Reviewed helper source changed")
    with SdkReadOnlyFiles() as sdk_files:
        sdk_files.verify(plan["sdkReadOnlyFiles"])
    if plan["testKey"] != {"privatePath": str(run / "emulator-home" / "adbkey"),
                          "publicPath": str(run / "emulator-home" / "adbkey.pub"),
                          "createOnce": True, "runnerReadsPrivateKeyData": False,
                          "unknownResult": "retainAndReconcileMetadataWithoutRetry"}:
        raise ValueError("Reviewed owned test-key scope changed")
    if plan["avdName"] != "weftext-android-os-" + identity or plan["commands"] != _commands(run, sdk, apk, manifest, plan["avdName"], plan["testClass"]):
        raise ValueError("Reviewed exact SDK commands changed")
    expected_environment = _environment(run, sdk, java)
    if plan["environment"] != expected_environment or plan["environmentUnset"] != list(_ENVIRONMENT_UNSET) or plan["createAvdStdin"] != "no\n":
        raise ValueError("Reviewed environment changed")
    if plan["serial"] != _SERIAL or plan["systemImage"] != _IMAGE or plan["ports"] != {"adbServer": _ADB_PORT, "console": _CONSOLE_PORT, "device": _DEVICE_PORT}:
        raise ValueError("Reviewed device/ports changed")
    if plan["limits"] != {"externalWindowsJobRequired": True, "deadlineSeconds": 1500,
            "jobMemoryBytes": 4 * 1024**3, "hostFreeMinimumBytes": 4 * 1024**3, "guestMemoryMiB": 2048, "guestCores": 2}:
        raise ValueError("Reviewed resource limits changed")
    if plan["consoleGreeting"] != {"host": "127.0.0.1", "port": _CONSOLE_PORT, "maxBytes": 4096,
                                  "requiredFirstLine": "Android Console: Authentication required"}:
        raise ValueError("Reviewed console qualification changed")
    return plan


def run_driver(plan_path: Path | str) -> int:
    """Execute only an already-reviewed one-shot plan inside the caller's exclusive Job."""
    plan = validate_plan(Path(plan_path), allow_supervisor_files=True)
    _owned_job_pids()
    reservations = _reserve_ports()
    run = Path(plan["runDirectory"])
    environment = os.environ.copy()
    for key in _ENVIRONMENT_UNSET:
        environment.pop(key, None)
    environment.update(plan["environment"])
    system = _system_directory()
    environment["PATH"] = os.pathsep.join((str(Path(plan["javaHome"]) / "bin"), str(system), str(system.parent)))
    for name in ("avd-home", "android-user-home", "emulator-home", "temp"):
        (run / name).mkdir(exist_ok=False)
    owned = _Run(plan, environment)
    code = 1
    try:
        _verify_dump(owned.command("verifyManifest"), plan["manifest"])
        owned.create_test_key()
        avd, avd_step = owned.spawn("createAvd", plan["commands"]["createAvd"], input_bytes=b"no\n")
        owned.wait(avd, avd_step, 120)
        if not (run / "avd/config.ini").is_file() or not (run / "avd-home" / (plan["avdName"] + ".ini")).is_file():
            raise ValueError("Owned AVD configuration was not created")
        reservations[0].close()
        owned.server, _ = owned.spawn("adbServer", plan["commands"]["adbServer"])
        deadline = time.monotonic() + 20
        while True:
            if owned.server.poll() is not None: raise ValueError("Foreground private ADB server exited")
            owners = _port_owners(_ADB_PORT)
            if owners == {owned.server.pid}: break
            if owners or time.monotonic() >= deadline: raise ValueError("Private ADB port ownership rejected")
            time.sleep(0.25)
        _release_ports(reservations[1:]); reservations = []
        owned.emulator, _ = owned.spawn("emulator", plan["commands"]["emulator"])
        deadline = time.monotonic() + 300
        while True:
            owned.own_server()
            if owned.emulator.poll() is not None: raise ValueError("Owned emulator exited during boot")
            ports = [_port_owners(port) for port in (_CONSOLE_PORT, _DEVICE_PORT)]
            known = _owned_job_pids()
            if any(value and not value.issubset(known) for value in ports): raise ValueError("Emulator port was claimed by a foreign process")
            if all(ports):
                # Only a nonzero boot-query exit may mean the newly-owned device is still offline.
                owned.own_device()
                process, step = owned.spawn("boot", plan["commands"]["boot"])
                owned.wait(process, step, 10, required=False)
                if process.returncode == 0 and Path(step["stdout"]).read_text(encoding="utf-8").strip() == "1": break
            if time.monotonic() >= deadline: raise TimeoutError("Owned emulator boot deadline")
            time.sleep(2)
        if owned.adb("avdIdentity").strip() != plan["avdName"]:
            raise ValueError("The guest did not identify the exact newly-owned AVD")
        owned.verify_console_auth()
        installed = owned.adb("install", 120)
        if not re.search(r"(?m)^Success\s*$", installed): raise ValueError("Exact instrumentation APK installation not confirmed")
        output = owned.adb("instrument", 300)
        tests = _instrumentation(output, plan["testClass"], plan["expectedTestCount"])
        owned.record.update(tests=tests, testResultsValidated=True, state="testsPassed")
        code = 0
    except (ValueError, OSError, TimeoutError, subprocess.SubprocessError) as error:
        owned.record.update(state="failed", errorType=type(error).__name__, error=str(error))
    finally:
        _release_ports(reservations)
        if not owned.stop(): code = 125
        owned.record.update(state="passed" if code == 0 else "failed", exitCode=code, endedUtc=_utc(),
                            planSha256=_sha(Path(plan_path)), apkSha256=plan["apkSha256"],
                            productCoreValidated=False, runtimeValidated=code == 0)
        owned._save()
    return code
