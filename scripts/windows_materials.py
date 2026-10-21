"""Freeze repository-qualified offline installers; never fetch or execute them."""
from pathlib import Path
import re
from urllib.parse import urlsplit

from package_inputs import parse_json, read_checked, sha256
from windows_package import closed, quad
from windows_trust import verify_material

MATERIAL_KEYS = ('id', 'kind', 'filename', 'sha256', 'size', 'version', 'payloadArchitectures', 'sourceUrl', 'finalUrl', 'publisherSubject', 'signerCertificateSha256', 'licenseFiles')
REQUIRED = {'webview2': 'evergreen-standalone', 'vc-runtime': 'vc-redist'}
OFFICIAL_HOSTS = {'aka.ms', 'go.microsoft.com', 'download.microsoft.com', 'download.visualstudio.microsoft.com', 'msedge.sf.dl.delivery.mp.microsoft.com'}


def validate_record(record):
    closed(record, MATERIAL_KEYS, 'Offline material')
    if record['id'] not in REQUIRED or record['kind'] != REQUIRED[record['id']]:
        raise ValueError('Unknown offline material kind/id')
    if not isinstance(record['filename'], str) or not re.fullmatch(r'[A-Za-z0-9_.-]+\.exe', record['filename']):
        raise ValueError('Offline material filename must be one leaf EXE name')
    for key in ('sha256', 'signerCertificateSha256'):
        if not isinstance(record[key], str) or not re.fullmatch('[0-9a-f]{64}', record[key]):
            raise ValueError('Offline material requires an exact canonical SHA-256')
    if type(record['size']) is not int or not 0 < record['size'] <= 1024 * 1024 * 1024:
        raise ValueError('Offline material has an invalid locked length')
    quad(record['version'])
    if not isinstance(record['payloadArchitectures'], list) or 'x64' not in record['payloadArchitectures'] or len(set(record['payloadArchitectures'])) != len(record['payloadArchitectures']) or set(record['payloadArchitectures']) - {'x64', 'arm64'}:
        raise ValueError('Offline installer lacks a qualified x64 payload')
    for key in ('sourceUrl', 'finalUrl'):
        url = urlsplit(record[key])
        if url.scheme != 'https' or url.hostname not in OFFICIAL_HOSTS or url.username or url.password or url.fragment:
            raise ValueError('Offline material needs its exact official HTTPS provenance')
    if record['publisherSubject'] != 'C=US, S=Washington, L=Redmond, O=Microsoft Corporation, CN=Microsoft Corporation':
        raise ValueError('Offline material is not the reviewed Microsoft publisher')
    if not isinstance(record['licenseFiles'], list) or not record['licenseFiles']:
        raise ValueError('Offline material is missing original license texts')
    for license_file in record['licenseFiles']:
        closed(license_file, ('file', 'sha256', 'size', 'sourceUrl'), 'Offline license')
        path = Path(license_file['file'])
        if path.is_absolute() or '..' in path.parts or not path.as_posix().startswith('packaging/windows/licenses/'):
            raise ValueError('Offline license must be a repository-owned original')
        if not re.fullmatch('[0-9a-f]{64}', license_file['sha256']) or type(license_file['size']) is not int or license_file['size'] <= 0:
            raise ValueError('Offline license requires exact bytes and SHA-256')


def freeze_materials(root, directory, contract):
    lock_bytes = read_checked(root / 'packaging/windows/offline-materials.json', root)
    lock = parse_json(lock_bytes)
    closed(lock, ('schemaVersion', 'materials'), 'Offline material lock')
    if type(lock['schemaVersion']) is not int or lock['schemaVersion'] != 1 or not isinstance(lock['materials'], list):
        raise ValueError('Unsupported offline material lock')
    records = lock['materials']
    for record in records:
        validate_record(record)
    if len(records) != len(REQUIRED) or {item['id'] for item in records} != set(REQUIRED):
        missing = sorted(set(REQUIRED) - {item['id'] for item in records})
        raise ValueError(f'No complete offline Windows candidate: missing qualified repository material(s): {", ".join(missing)}')
    files, verified = {}, []
    directory = Path(directory).absolute()
    for record in records:
        path = directory / record['filename']
        # This first qualification rejects reparse inputs before the native
        # same-handle signature reader. Only that reader's bytes are packaged.
        qualified = read_checked(path, directory)
        if len(qualified) != record['size'] or sha256(qualified) != record['sha256']:
            raise ValueError('Offline installer does not match its reviewed lock')
        data, signature = verify_material(path, record['sha256'], record['size'], record['publisherSubject'])
        if signature['leafCertificateSha256'] != record['signerCertificateSha256']:
            raise ValueError('Offline installer certificate differs from its reviewed original')
        if record['id'] == 'webview2' and quad(record['version']) < quad(contract['webview2']['minimumVersion']):
            raise ValueError('Locked offline WebView2 is older than the native host contract')
        installer_name = 'prerequisites/' + record['filename']
        if installer_name in files:
            raise ValueError('Duplicate offline installer filename')
        files[installer_name] = data
        for item in record['licenseFiles']:
            original = read_checked(root / item['file'], root)
            if sha256(original) != item['sha256'] or len(original) != item['size']:
                raise ValueError('Offline license original changed')
            license_name = 'prerequisites/licenses/' + record['id'] + '/' + Path(item['file']).name
            if license_name in files:
                raise ValueError('Duplicate offline license filename')
            files[license_name] = original
        verified.append({**record, 'signature': signature, 'installationTested': False, 'onlineRevocationChecked': False})
    return {'files': files, 'records': verified, 'lockSha256': sha256(lock_bytes)}
