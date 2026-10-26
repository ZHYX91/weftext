"""Owned, immutable packaging inputs shared by the build and package entry point."""
from dataclasses import dataclass, field
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import stat
import subprocess
import sys
import zipfile

IGNORED = {'.git', 'node_modules', 'target', 'dist', '.workspace', '__pycache__'}


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def parse_json(data):
    if isinstance(data, (bytes, bytearray)):
        data = bytes(data).decode('utf-8')  # Protocol/config JSON is UTF-8, not auto-detected UTF-16/32.
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f'Duplicate JSON field: {key}')
            result[key] = value
        return result
    def invalid_constant(value):
        raise ValueError(f'Invalid JSON constant: {value}')
    return json.loads(data, object_pairs_hook=unique, parse_constant=invalid_constant)


def is_reparse(info):
    return stat.S_ISLNK(info.st_mode) or bool(getattr(info, 'st_file_attributes', 0) & 0x400)


def require_checkout(root):
    root = Path(root).absolute()
    marker = root / '.git'
    if not marker.exists() or is_reparse(root.lstat()) or is_reparse(marker.lstat()):
        raise ValueError('Packaging requires an actual Git product checkout')
    result = subprocess.run(['git', '-C', str(root), 'rev-parse', '--show-toplevel'],
                            stdin=subprocess.DEVNULL, capture_output=True, check=True, timeout=30,
                            env={**os.environ, 'GIT_OPTIONAL_LOCKS': '0'})
    top = Path(result.stdout.decode('utf-8').rstrip('\r\n')).resolve(strict=True)
    if top != root.resolve(strict=True):
        raise ValueError('Git source selection does not identify this product checkout')


def source_files(root, excluded=()):
    root = Path(root).absolute()
    if not (root / '.git').exists():
        # Frozen source copies/source.zip have no Git metadata. Their only
        # inputs are the already selected source files; build/check remain
        # independent of the repository and package requires a checkout.
        yield from walk_files(root, excluded)
        return
    require_checkout(root)
    result = subprocess.run(['git', '-C', str(root), 'ls-files', '-c', '-o', '--exclude-standard', '-z'],
                            stdin=subprocess.DEVNULL, capture_output=True, check=True, timeout=30,
                            env={**os.environ, 'GIT_OPTIONAL_LOCKS': '0'})
    if len(result.stdout) > 64 * 1024 * 1024:
        raise ValueError('Git source-name inventory exceeds its explicit budget')
    excluded = {Path(path).absolute() for path in excluded}
    seen = set()
    for raw in result.stdout.split(b'\0'):
        if not raw:
            continue
        name = raw.decode('utf-8')
        relative = PurePosixPath(name)
        if relative.is_absolute() or relative.as_posix() != name or '..' in relative.parts or '\\' in name or ':' in name:
            raise ValueError('Git returned a noncanonical product source path')
        if name in seen:
            continue
        seen.add(name)
        path = root.joinpath(*relative.parts)
        if (not set(relative.parts).isdisjoint(IGNORED) or path.suffix in {'.pyc', '.pyo'}
                or any(path.is_relative_to(directory) for directory in excluded)):
            continue
        try:
            info = path.lstat()
        except FileNotFoundError:
            # The index also lists tracked files deliberately deleted in this
            # working tree. They are absent source, not the old index bytes.
            # Later disappearance during qualification/read remains an error.
            continue
        if stat.S_ISDIR(info.st_mode) and not is_reparse(info):
            if (path / '.git').exists():
                raise ValueError('Packaging does not admit a nested Git checkout or submodule as a source file')
            # A tracked file can become a directory in the current working
            # tree. Git still lists its stale index name and separately lists
            # its visible new children. Select the children, not old bytes.
            continue
        yield path


def read_checked(path, root, *, max_bytes=1024 * 1024 * 1024):
    """Never follow a source/material link into private or unrelated files."""
    root = Path(root).absolute()
    relative = Path(path).absolute().relative_to(root)
    if '..' in relative.parts:
        raise ValueError('Packaging input must remain inside its qualified root')
    qualified_root = root.resolve(strict=True)
    path = qualified_root / relative
    current = root
    for part in (None, *relative.parts):
        if part is not None:
            current /= part
        if is_reparse(current.lstat()):
            raise ValueError(f'Packaging refuses a symlink/reparse input: {current}')
    before = path.stat()
    if not stat.S_ISREG(before.st_mode):
        raise ValueError(f'Packaging input is not a regular file: {path}')
    if before.st_size > max_bytes:
        raise ValueError(f'Packaging input exceeds its explicit byte budget: {path}')
    with path.open('rb') as stream:
        opened = os.fstat(stream.fileno())
        if (before.st_dev, before.st_ino) != (opened.st_dev, opened.st_ino):
            raise ValueError(f'Packaging input changed while opening: {path}')
        # An ancestor could have been replaced since lstat. Inspect the opened
        # Windows handle before reading any bytes, rather than trusting the name.
        if os.name == 'nt':
            import ctypes
            import msvcrt
            from ctypes import wintypes
            function = ctypes.WinDLL('kernel32', use_last_error=True).GetFinalPathNameByHandleW
            function.argtypes = [wintypes.HANDLE, wintypes.LPWSTR, wintypes.DWORD, wintypes.DWORD]
            function.restype = wintypes.DWORD
            buffer = ctypes.create_unicode_buffer(32768)
            length = function(msvcrt.get_osfhandle(stream.fileno()), buffer, len(buffer), 0)
            if not length or length >= len(buffer):
                raise OSError(ctypes.get_last_error(), 'Cannot qualify packaging input handle')
            actual = buffer.value
            if actual.startswith('\\\\?\\UNC\\'):
                actual = '\\\\' + actual[8:]
            elif actual.startswith('\\\\?\\'):
                actual = actual[4:]
            if Path(actual) != path:
                raise ValueError(f'Packaging input no longer names its qualified file: {path}')
            Path(actual).relative_to(qualified_root)
        # BufferedReader.read(n) can allocate n bytes even for a tiny file. Use
        # the actual opened length, plus one byte to detect growth, not the
        # potentially 1-GiB policy ceiling for every individual license/source.
        data = stream.read(min(opened.st_size, max_bytes) + 1)
        if len(data) > max_bytes or len(data) != opened.st_size:
            raise ValueError(f'Packaging input grew or changed beyond its qualified length: {path}')
        after = os.fstat(stream.fileno())
    if (opened.st_size, opened.st_mtime_ns, opened.st_ctime_ns) != (after.st_size, after.st_mtime_ns, after.st_ctime_ns):
        raise ValueError(f'Packaging input changed while reading: {path}')
    return data


def walk_files(root, excluded=()):
    root = Path(root).absolute()
    if is_reparse(root.lstat()):
        raise ValueError(f'Packaging refuses a reparse root: {root}')
    excluded = {Path(path).absolute() for path in excluded}
    def onerror(error):
        raise error
    for directory, children, names in os.walk(root, followlinks=False, onerror=onerror):
        allowed = []
        for name in sorted(children):
            path = Path(directory) / name
            if name in IGNORED or path in excluded:
                continue
            if is_reparse(path.lstat()):
                raise ValueError(f'Packaging refuses a reparse directory: {path}')
            allowed.append(name)
        children[:] = allowed
        for name in sorted(names):
            path = Path(directory) / name
            if name not in IGNORED and path.suffix not in {'.pyc', '.pyo'}:
                yield path


def snapshot(root, excluded=(), byte_budget=512 * 1024 * 1024):
    files = {}
    used = 0
    folded = set()
    for path in source_files(root, excluded):
        name = path.relative_to(root).as_posix()
        if name.casefold() in folded:
            raise ValueError(f'Case-colliding Windows source names: {name}')
        folded.add(name.casefold())
        if path.lstat().st_size > byte_budget - used:
            raise ValueError('Packaging source inventory exceeds its explicit byte budget')
        data = read_checked(path, root, max_bytes=byte_budget - used)
        used += len(data)
        if used > byte_budget or len(files) >= 50000:
            raise ValueError('Packaging source inventory exceeds its explicit budget')
        files[name] = data
    return files


def inventory(files):
    return [{'path': name, 'bytes': len(data), 'sha256': sha256(data)} for name, data in sorted(files.items())]


def zip_bytes(files):
    output = io.BytesIO()
    with zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        for name, data in sorted(files.items()):
            entry = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            entry.compress_type = zipfile.ZIP_DEFLATED
            entry.external_attr = 0o100644 << 16
            archive.writestr(entry, data)
    return output.getvalue()


def validate_target(root, target):
    root, target = Path(root).resolve(), Path(target).absolute()
    if target.is_relative_to(root) and not target.is_relative_to(root / 'target'):
        raise ValueError('Packaging requires target/ or a Cargo target directory outside the source tree')
    return target


def collect_artifact(message, package_id, roles, paths):
    if message.get('reason') != 'compiler-artifact' or message.get('package_id') != package_id:
        return
    target = message.get('target', {})
    if 'bin' not in target.get('kind', []) or not message.get('executable'):
        return
    for role, policy in roles.items():
        if target.get('name') == policy['cargoBin']:
            path = Path(message['executable']).absolute()
            if role in paths and paths[role] != path:
                raise ValueError(f'Cargo reported conflicting executables for {role}')
            paths[role] = path


@dataclass(frozen=True)
class BuildArtifacts:
    source_root: Path
    paths: dict
    binaries: dict
    source: dict
    frontend: dict
    metadata: dict
    built_package_ids: frozenset
    target_directory: Path
    profile: str
    msvc_version: tuple | None = None
    build_environment: dict = field(default_factory=dict)
    catalog_path: Path | None = None
    catalog: bytes | None = None

    def verify_inputs(self, root):
        if snapshot(root, [self.target_directory]) != self.source:
            raise ValueError('Source changed after the build input snapshot; rebuild before packaging')
        if snapshot(root / 'app/dist') != self.frontend:
            raise ValueError('Embedded frontend changed after the build; rebuild before packaging')
        for role, path in self.paths.items():
            if read_checked(path, self.target_directory) != self.binaries[role]:
                raise ValueError(f'The built {role} executable changed before packaging')
        if self.catalog_path is not None and read_checked(self.catalog_path, self.target_directory) != self.catalog:
            raise ValueError('The fixed build artifact catalog changed before packaging')


def dependency_licenses(root, metadata, built_package_ids):
    """Freeze exact published license texts; missing originals are not a pass."""
    files, packages = {}, []
    overrides_path = root / 'packaging/windows/licenses/provenance.json'
    overrides = parse_json(read_checked(overrides_path, root)) if overrides_path.exists() else {'schemaVersion': 2}
    if not isinstance(overrides, dict) or type(overrides.get('schemaVersion')) is not int or overrides['schemaVersion'] != 2:
        raise ValueError('Unsupported dependency license provenance schema')
    for section in ('rust', 'npm'):
        entries = overrides.get(section, {})
        if not isinstance(entries, dict):
            raise ValueError(f'Invalid {section} license provenance section')
        for key, item in entries.items():
            if (not isinstance(key, str) or not isinstance(item, dict) or 'files' not in item
                    or {'file', 'sha256', 'size'}.intersection(item)):
                raise ValueError(f'Invalid upstream license files record: {section} {key}')
            if not isinstance(item['files'], list) or not item['files']:
                raise ValueError(f'Empty upstream license files: {section} {key}')
            if 'selectedLicense' in item and (not isinstance(item['selectedLicense'], str) or not item['selectedLicense'].strip()):
                raise ValueError(f'Invalid selected upstream license: {section} {key}')
            basenames = set()
            for record in item['files']:
                if not isinstance(record, dict):
                    raise ValueError(f'Invalid upstream license file: {section} {key}')
                name = record.get('file')
                if not isinstance(name, str) or not name:
                    raise ValueError(f'Invalid upstream original path: {section} {key}')
                relative = PurePosixPath(name)
                if (relative.is_absolute() or relative.as_posix() != name or '..' in relative.parts
                        or '\\' in name or ':' in name or not relative.parts):
                    raise ValueError(f'Invalid upstream original path: {section} {key}')
                if type(record.get('size')) is not int or record['size'] <= 0:
                    raise ValueError(f'Invalid upstream original size: {section} {key}')
                digest = record.get('sha256')
                if not isinstance(digest, str) or len(digest) != 64 or any(value not in '0123456789abcdef' for value in digest):
                    raise ValueError(f'Invalid upstream original SHA256: {section} {key}')
                basename = relative.name.casefold()
                if basename in basenames:
                    raise ValueError(f'Duplicate upstream license payload basename: {section} {key}')
                basenames.add(basename)

    def collect(name, version, folder, license_id, repository, ecosystem):
        label = f'{ecosystem}-{name.replace("/", "-").replace("@", "")}-{version}'
        copies = []
        selected_license = None
        for path in walk_files(folder):
            if path.name.lower().startswith(('license', 'licence', 'copying', 'notice', 'ofl')):
                target = f'third-party/{label}/{path.relative_to(folder).as_posix()}'
                original = read_checked(path, folder)
                if not original.strip():
                    raise ValueError(f'Empty original license text: {path}')
                files[target] = original
                copies.append(target)
        if not copies and ecosystem in ('cargo', 'npm'):
            section = 'rust' if ecosystem == 'cargo' else 'npm'
            item = overrides.get(section, {}).get(f'{name}-{version}')
            if item is not None:
                selected_license = item.get('selectedLicense')
                for record in item['files']:
                    data = read_checked(root / record['file'], root)
                    if sha256(data) != record['sha256'] or len(data) != record['size'] or not data.strip():
                        raise ValueError(f'Changed upstream license original: {ecosystem} {name} {version}')
                    target = f'third-party/{label}/{PurePosixPath(record["file"]).name}'
                    if target in files:
                        raise ValueError(f'Duplicate upstream license payload target: {target}')
                    files[target] = data
                    copies.append(target)
        if not copies:
            raise ValueError(f'Missing original license text: {ecosystem} {name} {version}')
        package = {'ecosystem': ecosystem, 'name': name, 'version': version, 'license': license_id, 'repository': repository, 'licenseFiles': copies}
        if selected_license is not None:
            package['selectedLicense'] = selected_license
        packages.append(package)

    for item in metadata['packages']:
        if item['id'] not in built_package_ids:
            continue
        manifest = Path(item['manifest_path']).absolute()
        if not item.get('source') and not manifest.is_relative_to(root):
            raise ValueError(f'Cargo path dependency is outside the frozen source: {manifest}')
        if item.get('source') or '/vendor/' in manifest.as_posix():
            collect(item['name'], item['version'], Path(item['manifest_path']).parent, item.get('license'), item.get('repository'), 'cargo')
    lock = parse_json(read_checked(root / 'app/package-lock.json', root))
    def applicable(spec, current):
        if not isinstance(spec, list) or any(not isinstance(entry, str) for entry in spec):
            raise ValueError('Invalid npm platform constraints')
        positive = [entry for entry in spec if not entry.startswith('!')]
        return f'!{current}' not in spec and '!any' not in spec and (not positive or current in positive or 'any' in positive)
    for relative, item in lock['packages'].items():
        if not relative:
            continue
        folder = root / 'app' / relative
        if not folder.exists():
            system = 'win32' if os.name == 'nt' else sys.platform
            incompatible = 'os' in item and not applicable(item['os'], system)
            incompatible |= 'cpu' in item and not applicable(item['cpu'], 'x64')
            if item.get('optional') and incompatible:
                continue  # Explicitly inapplicable platform package, omitted by npm ci.
            raise ValueError(f'Missing installed npm build dependency: {relative}')
        manifest = parse_json(read_checked(folder / 'package.json', folder))
        if manifest['version'] != item['version']:
            raise ValueError(f'Installed npm dependency differs from its locked version: {relative}')
        collect(manifest['name'], manifest['version'], folder, manifest.get('license'), manifest.get('repository'), 'npm')
    collect('weftext-offline-fonts', 'bundled', root / 'resources/fonts', 'See exact font manifests and originals', None, 'assets')
    for filename in ('manifest.json', 'builtin-manifest.json', 'web-manifest.json'):
        files[f'FONTS-{filename}'] = read_checked(root / 'resources/fonts' / filename, root)
    sdk = overrides.get('webview2Sdk')
    if not sdk:
        raise ValueError('Missing exact Microsoft WebView2 SDK license provenance')
    bindings = [item for item in metadata['packages'] if item['id'] in built_package_ids and item['name'] == 'webview2-com-sys']
    if len(bindings) != 1 or bindings[0]['version'] != sdk['sysCrateVersion']:
        raise ValueError('Built WebView2 binding differs from the qualified SDK license provenance')
    loader_root = Path(bindings[0]['manifest_path']).parent
    if sha256(read_checked(loader_root / 'x64/WebView2LoaderStatic.lib', loader_root)) != sdk['staticLoaderX64Sha256']:
        raise ValueError('Built static WebView2 Loader differs from its qualified Microsoft original')
    for name in ('LICENSE.txt', 'NOTICE.txt'):
        record = sdk['files'][name]
        data = read_checked(root / record['file'], root)
        if sha256(data) != record['sha256'] or len(data) != record['size'] or not data.strip():
            raise ValueError(f'Changed Microsoft WebView2 SDK {name}')
        files[f'third-party/microsoft-webview2-sdk-{sdk["version"]}/{name}'] = data
    return files, packages
