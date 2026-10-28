"""Acquire locked official inputs and select installed tools; never run installers."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from urllib.parse import urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener

from package_inputs import is_reparse, parse_json, read_checked, sha256
from windows_materials import OFFICIAL_HOSTS, REQUIRED, freeze_materials, validate_record
from windows_package import closed, load_contract, pe_info, quad, sdk_tools
from windows_trust import file_version

ROOT = Path(__file__).resolve().parents[1]
CHUNK = 1024 * 1024
DOWNLOAD_SECONDS = 600


def official(url):
    value = urlsplit(url)
    if (value.scheme != 'https' or value.hostname not in OFFICIAL_HOSTS
            or value.username or value.password or value.fragment):
        raise ValueError('Acquisition requires official Microsoft HTTPS URLs')
    return url


class OfficialRedirects(HTTPRedirectHandler):
    def redirect_request(self, request, response, code, message, headers, new_url):
        return super().redirect_request(request, response, code, message, headers,
                                        official(new_url))


def open_url(request):
    return build_opener(OfficialRedirects()).open(request, timeout=60)


def new_json(path, value):
    with path.open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def ordinary_parent(path):
    if not path.is_absolute() or '..' in path.parts:
        raise ValueError('Use an absolute ordinary output path')
    for item in (path, *path.parents):
        if item.exists() and is_reparse(item.lstat()):
            raise ValueError('Preparation output cannot use reparse points')


def locked_records(root):
    lock_bytes = read_checked(root / 'packaging/windows/offline-materials.json', root)
    lock = parse_json(lock_bytes)
    closed(lock, ('schemaVersion', 'materials'), 'Offline material lock')
    if type(lock['schemaVersion']) is not int or lock['schemaVersion'] != 1 or not isinstance(lock['materials'], list):
        raise ValueError('Unsupported offline material lock')
    records = lock['materials']
    for record in records:
        validate_record(record)
        for license_file in record['licenseFiles']:
            original = read_checked(root / license_file['file'], root)
            if len(original) != license_file['size'] or sha256(original) != license_file['sha256']:
                raise ValueError('Offline license original changed')
    if len(records) != len(REQUIRED) or {record['id'] for record in records} != set(REQUIRED):
        raise ValueError('A complete locked material set is required')
    return lock_bytes, records


def download(record, directory, *, opener=open_url, clock=time.monotonic):
    """One request, bounded bytes, no retry/adoption; failures retain partials."""
    validate_record(record)
    destination = directory / record['filename']
    if destination.exists() or destination.is_symlink():
        raise FileExistsError('An original material attempt already exists; no adoption or retry')
    result = {'id': record['id'], 'requestedUrl': official(record['finalUrl']),
              'lockedSourceUrl': record['sourceUrl'], 'filename': record['filename'],
              'lockedBytes': record['size'], 'lockedSha256': record['sha256'],
              'lockedPayloadVersion': record['version'], 'downloadedBytes': 0,
              'socketTimeoutSeconds': 60, 'betweenReadsTimeBudgetSeconds': DOWNLOAD_SECONDS,
              'startedUtc': datetime.now(timezone.utc).isoformat(), 'status': 'started',
              'installerExecuted': False}
    new_json(directory / (record['id'] + '.intent.json'), result)
    digest = hashlib.sha256()
    started = clock()
    try:
        request = Request(record['finalUrl'], headers={
            'User-Agent': 'Weftext-Windows-Package-Inputs/1', 'Accept-Encoding': 'identity'})
        with opener(request) as response:
            result['responseUrl'] = official(response.geturl())
            if response.status != 200:
                raise ValueError('Locked material request did not return HTTP200')
            lengths = response.headers.get_all('Content-Length') or []
            if len(lengths) > 1 or lengths and int(lengths[0]) != record['size']:
                raise ValueError('HTTP length differs from the locked original')
            with destination.open('xb') as stream:
                while True:
                    # The budget is observed between reads; the current socket
                    # read can take at most its separate 60-second timeout.
                    if clock() - started > DOWNLOAD_SECONDS:
                        raise TimeoutError('Locked material acquisition deadline expired')
                    # Observe at most one excess byte; never accept or consume an
                    # unbounded response merely because it came from an official URL.
                    block = response.read(min(CHUNK, record['size'] - result['downloadedBytes'] + 1))
                    if not block:
                        break
                    stream.write(block)
                    digest.update(block)
                    result['downloadedBytes'] += len(block)
                    if result['downloadedBytes'] > record['size']:
                        raise ValueError('Material exceeds its locked length')
                stream.flush()
                os.fsync(stream.fileno())
        result['downloadedSha256'] = digest.hexdigest()
        if result['downloadedBytes'] != record['size'] or result['downloadedSha256'] != record['sha256']:
            raise ValueError('Material differs from its locked bytes/SHA256')
        if sha256(read_checked(destination, directory)) != record['sha256']:
            raise ValueError('Downloaded material readback differs')
        result['status'] = 'downloadedExactOriginal'
        return result
    except BaseException as error:
        result['status'] = 'failedOriginalRetained'
        result['error'] = repr(error)
        result['downloadedSha256'] = digest.hexdigest()
        raise
    finally:
        result['endedUtc'] = datetime.now(timezone.utc).isoformat()
        new_json(directory / (record['id'] + '.result.json'), result)


def installed_visual_studio(environment):
    locator = Path(environment['ProgramFiles(x86)']) / 'Microsoft Visual Studio/Installer/vswhere.exe'
    raw = subprocess.check_output([str(locator), '-all', '-products', '*', '-requires',
        'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', '-format', 'json', '-utf8'], timeout=30)
    rows = parse_json(raw)
    if not isinstance(rows, list):
        raise ValueError('Visual Studio locator did not return its installation list')
    return [Path(row['installationPath']) for row in rows]


def select_linker(installations, runtime_version):
    eligible, rejected = [], []
    for installation in installations:
        for path in installation.glob('VC/Tools/MSVC/*/bin/Hostx64/x64/link.exe'):
            data = read_checked(path, path.parent)
            version = file_version(path)
            if pe_info(data)['machine'] != 0x8664 or version > quad(runtime_version):
                rejected.append({'path': str(path), 'fileVersion': list(version), 'reason': 'architecture/runtime compatibility'})
                continue
            tools = path.parents[3]
            if not (tools / 'lib/x64').is_dir() or not (tools / 'include').is_dir():
                rejected.append({'path': str(path), 'fileVersion': list(version), 'reason': 'installed libraries/headers missing'})
                continue
            eligible.append((version, str(path), {'path': str(path), 'fileVersion': list(version),
                'sha256': sha256(data), 'bytes': len(data), 'toolsRoot': str(tools)}))
    if not eligible:
        raise ValueError('No installed x64 MSVC linker is compatible with the locked VC Runtime')
    return max(eligible, key=lambda item: item[:2])[2], rejected


def select_sdk(kits):
    candidates = []
    for directory in (kits / 'bin').glob('*/x64'):
        try:
            version = quad(directory.parent.name)
        except ValueError:
            continue
        include = kits / 'Include' / directory.parent.name
        lib = kits / 'Lib' / directory.parent.name
        if (all((directory / name).is_file() for name in ('makeappx.exe', 'makepri.exe', 'rc.exe'))
                and all((include / name).is_dir() for name in ('ucrt', 'um', 'shared', 'winrt'))
                and all((lib / name / 'x64').is_dir() for name in ('ucrt', 'um'))):
            candidates.append((version, directory))
    if not candidates:
        raise ValueError('No complete installed x64 Windows SDK is available')
    directory = max(candidates, key=lambda item: item[0])[1]
    qualified = sdk_tools(directory)
    return {'directory': str(directory), 'kitsRoot': str(kits), 'version': directory.parent.name,
            'tools': {name: {key: str(value) if isinstance(value, Path) else value
                             for key, value in record.items()} for name, record in qualified.items()}}


def compiler_environment(linker, sdk, incoming):
    tools, kits = Path(linker['toolsRoot']), Path(sdk['kitsRoot'])
    include = kits / 'Include' / sdk['version']
    libraries = kits / 'Lib' / sdk['version']
    headers = [tools / 'include', *[include / name for name in ('ucrt', 'um', 'shared', 'winrt')]]
    if (include / 'cppwinrt').is_dir():
        headers.append(include / 'cppwinrt')
    return {'CARGO_BUILD_TARGET': 'x86_64-pc-windows-msvc',
        'CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER': linker['path'],
        'WEFTEXT_WINDOWS_SDK_BIN': sdk['directory'],
        'LIB': os.pathsep.join(map(str, [tools / 'lib/x64', libraries / 'ucrt/x64', libraries / 'um/x64'])),
        'INCLUDE': os.pathsep.join(map(str, headers)),
        'PATH': os.pathsep.join([str(Path(linker['path']).parent), sdk['directory'], incoming.get('PATH', '')])}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--materials', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args(argv)
    if os.name != 'nt':
        parser.error('Preparation requires the actual Windows tool/signature APIs')
    materials, output = args.materials.absolute(), args.output.absolute()
    ordinary_parent(materials)
    ordinary_parent(output.parent)
    if output.exists() or output.is_symlink():
        raise FileExistsError('Preparation output already exists')
    if not materials.parent.is_dir() or not output.parent.is_dir():
        raise ValueError('Preparation parents must already exist')
    materials.mkdir(exist_ok=False)
    lock_bytes, records = locked_records(ROOT)
    result = {'schemaVersion': 1, 'scope': 'acquisition and installed-tool selection; no product acceptance',
              'startedUtc': datetime.now(timezone.utc).isoformat(), 'materialsDirectory': str(materials),
              'lockSha256': sha256(lock_bytes), 'downloads': [], 'installerExecuted': False,
              'trustChanged': False, 'status': 'preparing'}
    try:
        for record in records:
            result['downloads'].append(download(record, materials))
        contract = load_contract(read_checked(ROOT / 'packaging/windows/host-contract.json', ROOT))
        frozen = freeze_materials(ROOT, materials, contract)
        result['materialQualification'] = frozen['records']
        runtime = next(record['version'] for record in records if record['id'] == 'vc-runtime')
        result['linker'], result['rejectedLinkers'] = select_linker(installed_visual_studio(os.environ), runtime)
        result['sdk'] = select_sdk(Path(os.environ['ProgramFiles(x86)']) / 'Windows Kits/10')
        result['environment'] = compiler_environment(result['linker'], result['sdk'], os.environ)
        result['status'] = 'preparedInputsOnly'
    except BaseException as error:
        result['status'] = 'failedOriginalsRetained'
        result['error'] = repr(error)
        raise
    finally:
        result['endedUtc'] = datetime.now(timezone.utc).isoformat()
        new_json(output, result)


if __name__ == '__main__':
    main()
