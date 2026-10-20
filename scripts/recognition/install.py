#!/usr/bin/env python3
"""Install the locked, independent Windows recognition runtime.

Only this explicit installation entry point uses the network. It statically
extracts upstream installers; it never executes either installer, changes PATH,
creates shortcuts or falls back to packages from another environment.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import struct
import subprocess
import sys
import urllib.parse
import urllib.request
import uuid
import zipfile

REPO = Path(__file__).resolve().parents[2]
LOCK = REPO / "resources" / "recognition" / "runtime-lock.json"
MARKER = ".weftext-recognition-runtime.json"
OWNER = "weftext-recognition-runtime"
MAX_DOWNLOAD = 128 * 1024 * 1024
SYSTEM_DLLS = {
    "advapi32.dll", "bcrypt.dll", "crypt32.dll", "gdi32.dll", "iphlpapi.dll",
    "kernel32.dll", "msvcrt.dll", "secur32.dll", "user32.dll", "wldap32.dll",
    "ws2_32.dll",
}


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_json(path: Path, value: object) -> None:
    no_reparse(path)
    temporary = path.with_name(path.name + "." + uuid.uuid4().hex + ".part")
    with temporary.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write("\n")
    temporary.replace(path)


def no_reparse(path: Path) -> None:
    for current in (path, *path.parents):
        if current.exists() and (
            current.is_symlink()
            or getattr(current.lstat(), "st_file_attributes", 0) & 0x400
        ):
            raise ValueError(f"Reparse/symlink path is not accepted: {current}")


def own_root(path: Path) -> None:
    no_reparse(path)
    marker = path / MARKER
    if path.exists() and any(path.iterdir()):
        if not marker.is_file():
            raise ValueError(f"Nonempty runtime has no Weftext ownership marker: {path}")
        value = json.loads(marker.read_text(encoding="utf-8"))
        if value.get("owner") != OWNER or value.get("version") != 1:
            raise ValueError("Runtime ownership marker does not match")
    else:
        path.mkdir(parents=True, exist_ok=True)
        write_json(marker, {"version": 1, "owner": OWNER})


def environment(root: Path) -> dict[str, str]:
    env = os.environ.copy()
    for name in tuple(env):
        if name.upper().startswith(("PYTHON", "PIP_")) or name in {
            "VIRTUAL_ENV", "TESSDATA_PREFIX", "PDFIUM_BINARY", "PDFIUM_PLATFORM"
        }:
            env.pop(name, None)
    system_root = Path(os.environ.get("SystemRoot", r"C:\Windows"))
    env["PATH"] = os.pathsep.join((str(system_root / "System32"), str(system_root)))
    env["OMP_THREAD_LIMIT"] = "1"
    env["TMP"] = str(root / "work")
    env["TEMP"] = env["TMP"]
    no_reparse(root / "work")
    (root / "work").mkdir(exist_ok=True)
    return env


def run(argv: list[str], root: Path, *, timeout: int = 120) -> str:
    no_reparse(Path(argv[0]))
    result = subprocess.run(
        argv, cwd=root, env=environment(root), capture_output=True,
        encoding="utf-8", errors="replace", timeout=timeout,
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
    )
    if result.returncode:
        raise RuntimeError(
            f"Command failed ({result.returncode}): {Path(argv[0]).name}\n"
            + (result.stderr + result.stdout)[-8000:]
        )
    return result.stdout


def download(asset: dict, cache: Path, offline: bool) -> Path:
    no_reparse(cache)
    name = asset["filename"]
    if Path(name).name != name or name in {".", ".."}:
        raise ValueError("Invalid locked asset filename")
    target = cache / name
    if target.exists():
        no_reparse(target)
        if digest(target) != asset["sha256"] or target.stat().st_size != asset["size"]:
            raise ValueError(f"Cached asset differs from the lock; preserved: {target}")
        return target
    if offline:
        raise ValueError(f"Offline cache is missing a locked asset: {name}")
    if urllib.parse.urlsplit(asset["url"]).scheme != "https":
        raise ValueError("Locked downloads must use HTTPS")
    temporary = cache / (name + "." + uuid.uuid4().hex + ".part")
    request = urllib.request.Request(
        asset["url"], headers={"User-Agent": "Weftext-recognition-installer/1"}
    )
    total = 0
    with urllib.request.urlopen(request, timeout=120) as response:
        if urllib.parse.urlsplit(response.url).scheme != "https":
            raise ValueError("Download redirected away from HTTPS")
        with temporary.open("xb") as stream:
            while block := response.read(1024 * 1024):
                total += len(block)
                if total > MAX_DOWNLOAD or total > asset["size"]:
                    raise ValueError(f"Locked download exceeded its byte budget: {name}")
                stream.write(block)
    if total != asset["size"] or digest(temporary) != asset["sha256"]:
        raise ValueError(f"Download differs from the lock; partial file preserved: {temporary}")
    temporary.replace(target)
    return target


def pe_imports(path: Path) -> list[str]:
    """Read PE imports without loading a DLL into the installation process."""
    data = path.read_bytes()
    if data[:2] != b"MZ":
        raise ValueError(f"Not a PE binary: {path.name}")
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe:pe + 4] != b"PE\0\0":
        raise ValueError(f"Invalid PE header: {path.name}")
    count = struct.unpack_from("<H", data, pe + 6)[0]
    optional = pe + 24
    magic = struct.unpack_from("<H", data, optional)[0]
    if magic not in (0x10B, 0x20B):
        raise ValueError("Unsupported PE optional header")
    directories = optional + (112 if magic == 0x20B else 96)
    import_rva = struct.unpack_from("<I", data, directories + 8)[0]
    optional_size = struct.unpack_from("<H", data, pe + 20)[0]
    sections = []
    for i in range(count):
        offset = optional + optional_size + 40 * i
        virtual_size, address, raw_size, raw_offset = struct.unpack_from(
            "<IIII", data, offset + 8
        )
        sections.append((address, max(virtual_size, raw_size), raw_offset))

    def position(rva: int) -> int:
        for address, size, offset in sections:
            if address <= rva < address + size:
                return offset + rva - address
        raise ValueError(f"Invalid PE RVA in {path.name}")

    if not import_rva:
        return []
    offset = position(import_rva)
    imports = []
    for _ in range(1024):
        if not any(data[offset:offset + 20]):
            return imports
        name_rva = struct.unpack_from("<I", data, offset + 12)[0]
        name_offset = position(name_rva)
        name_end = data.index(0, name_offset, name_offset + 256)
        imports.append(data[name_offset:name_end].decode("ascii"))
        offset += 20
    raise ValueError("PE import table exceeded its bound")


def copy_exact(source: Path, target: Path) -> None:
    no_reparse(target)
    if target.exists():
        if digest(source) != digest(target):
            raise ValueError(f"Existing runtime file differs; preserved: {target}")
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)


def install_tesseract(root: Path, lock: dict, assets: dict[str, Path]) -> dict:
    extractor = root / "tools" / "7zip"
    no_reparse(extractor)
    extractor.mkdir(parents=True, exist_ok=True)
    bootstrap = assets[lock["extractor"]["bootstrap"]["filename"]]
    archive = assets[lock["extractor"]["archive"]["filename"]]
    # Each input archive was verified above. Neither executable installer runs.
    run([str(bootstrap), "x", str(archive), "-o" + str(extractor), "-y",
         "--", "7z.exe", "7z.dll", "License.txt", "readme.txt"], root)
    for name, expected in lock["extractor"]["files"].items():
        if digest(extractor / name) != expected["sha256"]:
            raise ValueError(f"Extracted 7-Zip file differs from the lock: {name}")
    seven = extractor / "7z.exe"
    stage = root / "downloads" / "tesseract-extracted"
    no_reparse(stage)
    run([str(seven), "e", str(assets[lock["tesseract"]["filename"]]),
         "-o" + str(stage), "-y", "--", "tesseract.exe", "*.dll",
         r"doc\LICENSE", r"doc\AUTHORS", r"doc\README.md"], root)
    available = {p.name.lower(): p for p in stage.glob("*.dll")}
    available["tesseract.exe"] = stage / "tesseract.exe"
    pending = ["tesseract.exe"]
    included: dict[str, dict] = {}
    system = set()
    destination = root / "tesseract"
    no_reparse(destination)
    destination.mkdir(exist_ok=True)
    expected_names = {name.lower() for name in lock["tesseract"]["files"]}
    for existing in destination.iterdir():
        if existing.name.lower() not in expected_names:
            raise ValueError(f"Unexpected existing Tesseract runtime file: {existing}")
    while pending:
        name = pending.pop().lower()
        if name in included:
            continue
        source = available[name]
        expected = next((item for filename, item in lock["tesseract"]["files"].items()
                         if filename.lower() == name), None)
        if expected is None or digest(source) != expected["sha256"]:
            raise ValueError(f"Extracted Tesseract binary differs from the lock: {name}")
        dependencies = pe_imports(source)
        target = destination / source.name
        copy_exact(source, target)
        included[name] = {
            "path": str(target), "sha256": digest(target), "imports": dependencies
        }
        for dependency in dependencies:
            normalized = dependency.lower()
            if normalized in available:
                pending.append(normalized)
            elif normalized in SYSTEM_DLLS:
                system.add(dependency)
            else:
                raise ValueError(f"Unresolved non-system DLL dependency: {dependency}")
    if set(included) != {name.lower() for name in lock["tesseract"]["files"]}:
        raise ValueError("Tesseract dependency closure differs from the lock")
    for name in ("LICENSE", "AUTHORS", "README.md"):
        copy_exact(stage / name, root / "licenses" / "tesseract" / name)
    return {"files": included, "systemDlls": sorted(system),
            "dependencyEvidence": "PE transitive import closure and execution probes"}


def copy_wheel_notices(wheel: Path, root: Path, package: str) -> None:
    with zipfile.ZipFile(wheel) as archive:
        for item in archive.infolist():
            if ".dist-info/licenses/" not in item.filename or item.is_dir():
                continue
            suffix = item.filename.split(".dist-info/licenses/", 1)[1]
            relative = Path(suffix)
            if relative.is_absolute() or ".." in relative.parts:
                raise ValueError("Unsafe wheel notice path")
            target = root / "licenses" / package / relative
            no_reparse(target)
            target.parent.mkdir(parents=True, exist_ok=True)
            data = archive.read(item)
            if target.exists() and target.read_bytes() != data:
                raise ValueError(f"Existing license differs: {target}")
            target.write_bytes(data)


PROBE = r'''
import json,sys,subprocess,os
from pathlib import Path
import pypdfium2 as pdfium
from PIL import Image,ImageDraw,ImageFont,__version__ as pillow_version
root=Path(sys.argv[1])
doc=pdfium.PdfDocument.new()
page=doc.new_page(144,144)
page.render(scale=1).to_pil().save(root/'work'/'pdfium-probe.png')
page.close();doc.close()
image=Image.new('RGB',(1200,160),'white')
ImageDraw.Draw(image).text((20,30),'Weftext OCR 12345',font=ImageFont.load_default(size=58),fill='black')
png=root/'work'/'tesseract-probe.png';image.save(png)
tesseract=root/'tesseract'/'tesseract.exe'
base=[str(tesseract),str(png),'stdout','--tessdata-dir',str(root/'tessdata'),'--oem','1','--psm','7']
results={}
for combination in ('eng','chi_sim','chi_tra','chi_sim+eng','chi_tra+eng'):
 result=subprocess.run(base+['-l',combination],capture_output=True,encoding='utf-8',errors='replace',timeout=60,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
 if result.returncode: raise RuntimeError(combination+': '+result.stderr)
 results[combination]=result.stdout.strip()
 if combination=='eng' and '12345' not in result.stdout: raise RuntimeError('English OCR smoke probe did not recognize the known digits')
print(json.dumps({'pythonVersion':sys.version.split()[0],'pypdfium2Version':pdfium.PYPDFIUM_INFO.version,'pdfiumVersion':pdfium.PDFIUM_INFO.version,'pillowVersion':pillow_version,'ocrProbes':results}))
'''


def probe(root: Path, lock: dict) -> dict:
    python = root / "venv" / "Scripts" / "python.exe"
    packages = root / "venv" / "Lib" / "site-packages"
    for package in lock["packages"]:
        for name, expected_file in package["runtimeFiles"].items():
            path = packages / name
            no_reparse(path)
            if digest(path) != expected_file["sha256"]:
                raise ValueError(f"Installed package file differs from the product lock: {name}")
    for name, expected_binary in lock["tesseract"]["files"].items():
        path = root / "tesseract" / name
        no_reparse(path)
        if digest(path) != expected_binary["sha256"]:
            raise ValueError(f"Installed binary differs from the product lock: {name}")
    details = json.loads(run([str(python), "-I", "-c", PROBE, str(root)], root, timeout=360))
    expected = {item["name"].lower(): item["version"] for item in lock["packages"]}
    if details["pypdfium2Version"] != expected["pypdfium2"]:
        raise ValueError("Installed pypdfium2 version does not match")
    if details["pillowVersion"] != expected["pillow"]:
        raise ValueError("Installed Pillow version does not match")
    executable = root / "tesseract" / "tesseract.exe"
    version = run([str(executable), "--version"], root)
    if version.splitlines()[0] != lock["tesseract"]["versionOutputFirstLine"]:
        raise ValueError("Tesseract version does not match")
    languages = run([str(executable), "--list-langs", "--tessdata-dir",
                     str(root / "tessdata")], root)
    for language, asset in lock["models"]["files"].items():
        if language not in languages.splitlines():
            raise ValueError(f"Tesseract did not load language {language}")
        if digest(root / "tessdata" / asset["filename"]) != asset["sha256"]:
            raise ValueError(f"Model bytes differ: {language}")
    details["tesseractVersionOutput"] = version
    details["tesseractLanguagesOutput"] = languages
    return details


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime-dir", type=Path, required=True)
    parser.add_argument("--python", type=Path, help="Absolute, non-venv CPython 3.12 x64")
    parser.add_argument("--offline", action="store_true", help="Use only verified cached assets")
    parser.add_argument("--verify", action="store_true", help="Verify and probe an installed runtime without downloads")
    options = parser.parse_args()
    if sys.platform != "win32" or platform.machine().lower() not in {"amd64", "x86_64"}:
        parser.error("The locked runtime currently supports Windows x64 only")
    if not options.runtime_dir.is_absolute():
        parser.error("--runtime-dir must be absolute")
    no_reparse(options.runtime_dir)
    root = options.runtime_dir.resolve()
    if options.verify and not (root / MARKER).is_file():
        raise ValueError("Verification requires an existing owned runtime")
    own_root(root)
    lock = json.loads(LOCK.read_text(encoding="utf-8"))
    lock_sha = digest(LOCK)
    manifest_path = root / "installation.json"
    no_reparse(manifest_path)
    if manifest_path.exists():
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        if manifest.get("runtimeLockSha256") != lock_sha:
            raise ValueError("Existing runtime uses a different lock; choose a new owned directory")
        for binary in manifest["tesseract"]["files"].values():
            if digest(Path(binary["path"])) != binary["sha256"]:
                raise ValueError("Installed Tesseract binary changed")
        verified = probe(root, lock)
        print(json.dumps({"status": "verified", "runtimeDir": str(root),
                          "probe": verified}, ensure_ascii=False, indent=2))
        return 0
    if options.verify:
        raise ValueError("Runtime has no completed installation manifest")
    if not options.python or not options.python.is_absolute():
        parser.error("An initial install requires absolute --python")
    base_python = options.python.resolve()
    base = json.loads(run([str(base_python), "-I", "-c",
        "import sys,struct,json;print(json.dumps({'version':list(sys.version_info[:3]),'bits':struct.calcsize('P')*8,'isVenv':sys.prefix!=sys.base_prefix}))"], root))
    if base["version"][:2] != [3, 12] or base["bits"] != 64 or base["isVenv"]:
        raise ValueError("Base interpreter must be non-venv CPython 3.12 x64")
    cache = root / "downloads"
    no_reparse(cache)
    cache.mkdir(exist_ok=True)
    asset_list = [*lock["packages"], lock["tesseract"],
                  lock["extractor"]["bootstrap"], lock["extractor"]["archive"],
                  *lock["models"]["files"].values(), *lock["notices"]]
    assets = {asset["filename"]: download(asset, cache, options.offline) for asset in asset_list}
    no_reparse(root / "licenses")
    (root / "licenses").mkdir(exist_ok=True)
    tesseract = install_tesseract(root, lock, assets)
    venv = root / "venv"
    no_reparse(venv)
    if not venv.exists():
        run([str(base_python), "-I", "-m", "venv", str(venv)], root)
    config = (venv / "pyvenv.cfg").read_text(encoding="utf-8")
    settings = dict(line.split(" = ", 1) for line in config.splitlines() if " = " in line)
    if (Path(settings.get("home", "")).resolve() != base_python.parent
            or settings.get("include-system-site-packages", "").lower() != "false"):
        raise ValueError("Existing venv does not use the selected independent base interpreter")
    python = venv / "Scripts" / "python.exe"
    requirements = root / "downloads" / "requirements-locked.txt"
    requirements.write_text("\n".join(
        f"{item['name']}=={item['version']} --hash=sha256:{item['sha256']}"
        for item in lock["packages"]
    ) + "\n", encoding="utf-8")
    run([str(python), "-I", "-m", "pip", "--isolated", "install", "--no-index",
         "--no-deps", "--require-hashes", "--only-binary=:all:",
         "--find-links", str(cache), "-r", str(requirements)], root)
    models = {}
    for language, asset in lock["models"]["files"].items():
        target = root / "tessdata" / asset["filename"]
        copy_exact(assets[asset["filename"]], target)
        models[language] = {"path": str(target), "sha256": asset["sha256"],
                            "version": lock["models"]["version"],
                            "commit": lock["models"]["commit"]}
    for item in lock["packages"]:
        copy_wheel_notices(assets[item["filename"]], root, item["name"])
    for item in lock["notices"]:
        copy_exact(assets[item["filename"]], root / "licenses" / item["filename"])
    copy_exact(root / "tools" / "7zip" / "License.txt", root / "licenses" / "7zip-LICENSE.txt")
    details = probe(root, lock)
    executable = root / "tesseract" / "tesseract.exe"
    notices = [{"path": str(p.relative_to(root)), "sha256": digest(p)}
               for p in sorted((root / "licenses").rglob("*")) if p.is_file()]
    manifest = {
        "version": 1, "runtimeDir": str(root), "platform": lock["platform"],
        "installedAt": dt.datetime.now(dt.timezone.utc).isoformat(),
        "runtimeLockSha256": lock_sha,
        "python": {"path": str(python), "version": details["pythonVersion"],
                   "basePath": str(base_python), "baseVersion": ".".join(map(str, base["version"]))},
        "tesseract": {"path": str(executable), "version": lock["tesseract"]["version"],
                      "sha256": digest(executable), "installerExecuted": False,
                      "installMode": "archive-extraction-only", **tesseract},
        "pdfium": {"pypdfium2Version": details["pypdfium2Version"],
                   "pdfiumVersion": details["pdfiumVersion"]},
        "pillow": {"version": details["pillowVersion"]},
        "models": models, "modelCombinations": lock["models"]["combinations"],
        "engineMode": 1, "budgets": lock["budgets"], "probe": details,
        "licenses": notices, "sources": lock["sources"],
        "distributionBoundary": lock["distributionBoundary"],
        "registryModified": False, "pathModified": False,
    }
    write_json(manifest_path, manifest)
    print(json.dumps({"status": "installed", "manifest": str(manifest_path),
                      "python": str(python), "tesseract": str(executable),
                      "probe": details}, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired,
            zipfile.BadZipFile) as error:
        print(f"Recognition runtime installation stopped: {error}", file=sys.stderr)
        raise SystemExit(1)
