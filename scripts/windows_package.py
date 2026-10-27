"""Windows ZIP/MSIX generation. Called only by mvp.py; never installs or signs."""
import base64
from datetime import datetime, timezone
import io
import json
import os
from pathlib import Path
import re
import struct
import subprocess
import uuid
import xml.etree.ElementTree as ET
import zipfile

from package_inputs import dependency_licenses, inventory, parse_json, read_checked, sha256, zip_bytes

NS = {
    '': 'http://schemas.microsoft.com/appx/manifest/foundation/windows10',
    'uap': 'http://schemas.microsoft.com/appx/manifest/uap/windows10',
    'uap5': 'http://schemas.microsoft.com/appx/manifest/uap/windows10/5',
    'uap10': 'http://schemas.microsoft.com/appx/manifest/uap/windows10/10',
    'desktop': 'http://schemas.microsoft.com/appx/manifest/desktop/windows10',
    'com': 'http://schemas.microsoft.com/appx/manifest/com/windows10',
    'virtualization': 'http://schemas.microsoft.com/appx/manifest/virtualization/windows10',
    'rescap': 'http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities',
}
for prefix, uri in NS.items():
    ET.register_namespace(prefix, uri)


def closed(value, keys, label):
    if not isinstance(value, dict) or set(value) != set(keys):
        raise ValueError(f'{label} has missing/unknown fields')


def quad(value):
    if not isinstance(value, str) or not re.fullmatch(r'(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*)){3}', value):
        raise ValueError('Expected a canonical four-part Windows version')
    parts = tuple(map(int, value.split('.')))
    if any(part > 65535 for part in parts):
        raise ValueError('Windows version component exceeds 65535')
    return parts


def msix_version(semver):
    match = re.fullmatch(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-rc\.(0|[1-9][0-9]*))?', semver)
    if not match:
        raise ValueError('Local MSIX versions support only stable or canonical -rc.N SemVer')
    major, minor, patch = map(int, match.group(1, 2, 3))
    revision = int(match.group(4)) if match.group(4) is not None else 65535
    if match.group(4) is not None and revision >= 65535:
        raise ValueError('rc.N must be less than 65535; stable reserves revision 65535')
    value = f'{major}.{minor}.{patch}.{revision}'
    quad(value)
    return value


def load_contract(data):
    value = parse_json(data)
    closed(value, ('schemaVersion', 'identity', 'platform', 'roles', 'paths', 'activation', 'webview2'), 'Host contract')
    if type(value['schemaVersion']) is not int or value['schemaVersion'] != 1:
        raise ValueError('Unsupported host contract version')
    closed(value['identity'], ('name', 'publisher', 'applicationId'), 'Identity')
    closed(value['platform'], ('architecture', 'minVersion', 'maxVersionTested'), 'Platform')
    closed(value['roles'], ('desktop', 'cli'), 'Roles')
    for role, bin_name, subsystem in (('desktop', 'weftext', 2), ('cli', 'weftext-cli', 3)):
        expected = {'cargoBin': bin_name, 'filename': bin_name + '.exe', 'peSubsystem': subsystem}
        closed(value['roles'][role], expected, f'Role {role}')
        if value['roles'][role] != expected or type(value['roles'][role].get('peSubsystem')) is not int:
            raise ValueError(f'Invalid executable role: {role}')
    identity = value['identity']
    if not isinstance(identity['name'], str) or not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9.-]{2,49}', identity['name']):
        raise ValueError('Invalid package identity name')
    if not isinstance(identity['publisher'], str) or not re.fullmatch(r'CN=[A-Za-z0-9 .-]{1,100}', identity['publisher']):
        raise ValueError('Invalid local package publisher subject')
    if not isinstance(identity['applicationId'], str) or not re.fullmatch(r'[A-Za-z][A-Za-z0-9]{0,63}', identity['applicationId']):
        raise ValueError('Invalid application Id')
    platform = value['platform']
    if platform['architecture'] != 'x64' or quad(platform['minVersion']) < (10, 0, 22000, 0) or quad(platform['maxVersionTested']) < quad(platform['minVersion']):
        raise ValueError('This package requires the Win11 x64 precise-virtualization contract')
    # MakeAppx accepts some invalid KnownFolder values. Keep this security/data
    # boundary closed rather than accepting arbitrary paths from a caller.
    if value['paths'] != {'persistentKnownFolder': 'LocalAppData', 'persistentRelativeRoot': 'Weftext-MVP',
                          'coreRelativePath': 'core', 'hostStateRelativePath': 'host-state',
                          'browserDataKnownFolder': 'LocalAppData', 'browserDataRelativeRoot': 'Weftext/Cache'}:
        raise ValueError('Persistent Core/host-state and browser-data paths differ from the supported contract')
    activation = value['activation']
    closed(activation, ('portableAumid', 'startupTaskId', 'startupArgument', 'toastClsid', 'toastArgument'), 'Activation')
    if activation['startupArgument'] != '--login-startup' or activation['toastArgument'] != '--toast-server':
        raise ValueError('Unsupported activation arguments')
    if not isinstance(activation['portableAumid'], str) or not re.fullmatch(r'[A-Za-z0-9.]{3,128}', activation['portableAumid']):
        raise ValueError('Invalid portable AUMID')
    if not isinstance(activation['startupTaskId'], str) or not re.fullmatch(r'[A-Za-z][A-Za-z0-9]{0,63}', activation['startupTaskId']):
        raise ValueError('Invalid StartupTask Id')
    if str(uuid.UUID(activation['toastClsid'])) != activation['toastClsid']:
        raise ValueError('Toast CLSID must be a canonical UUID')
    closed(value['webview2'], ('minimumVersion',), 'WebView2')
    quad(value['webview2']['minimumVersion'])
    return value


def pe_info(data):
    """Inspect actual PE bytes, without loading code or guessing from filename."""
    def number(fmt, offset):
        if offset < 0 or offset + struct.calcsize(fmt) > len(data):
            raise ValueError('Truncated PE header')
        return struct.unpack_from(fmt, data, offset)
    if len(data) < 64 or data[:2] != b'MZ':
        raise ValueError('Expected a Windows PE executable')
    pe, = number('<I', 60)
    if data[pe:pe + 4] != b'PE\0\0':
        raise ValueError('Invalid PE signature')
    machine, sections = number('<HH', pe + 4)
    size, characteristics = number('<HH', pe + 20)
    optional = pe + 24
    magic, = number('<H', optional)
    if magic not in (0x10b, 0x20b) or size < (96 if magic == 0x10b else 112) or not 1 <= sections <= 96:
        raise ValueError('Invalid PE optional header')
    number(f'<{size}s', optional)
    subsystem, = number('<H', optional + 68)
    entrypoint, = number('<I', optional + 16)
    directory = optional + (96 if magic == 0x10b else 112)
    directory_count, = number('<I', directory - 4)
    if directory_count > (optional + size - directory) // 8:
        raise ValueError('Invalid PE data directory count')
    table = optional + size
    entries = []
    for index in range(sections):
        entry = table + 40 * index
        virtual_size, rva, raw_size, raw = number('<IIII', entry + 8)
        if raw_size and raw + raw_size > len(data):
            raise ValueError('Truncated PE section')
        flags, = number('<I', entry + 36)
        entries.append((rva, virtual_size, raw_size, raw, flags))
    if not any(base <= entrypoint < base + min(virtual, raw_size) and flags & 0x20000000
               for base, virtual, raw_size, raw, flags in entries):
        raise ValueError('PE entrypoint is not backed by executable section bytes')
    def offset(rva, length=1):
        for base, virtual, raw_size, raw, flags in entries:
            if base <= rva and rva - base + length <= raw_size:
                return raw + rva - base
        raise ValueError('PE import RVA lies outside a file-backed section')
    imports = []
    def imported_name(rva):
        start = offset(rva)
        end = data.find(b'\0', start, min(start + 257, len(data)))
        if end < 0:
            raise ValueError('Unterminated PE import name')
        offset(rva, end - start + 1)  # The complete name must be mapped, not PE overlay bytes.
        name = data[start:end].decode('ascii').lower()
        if not re.fullmatch(r'[a-z0-9_.-]+\.dll', name):
            raise ValueError('Invalid PE import name')
        imports.append(name)
    if directory_count > 1:
        import_rva, import_size = number('<II', directory + 8)
        if import_rva:
            if import_size > 1024 * 1024 or import_size < 20:
                raise ValueError('Invalid PE import table size')
            for index in range(min(import_size // 20, 1024)):
                descriptor = number('<IIIII', offset(import_rva + index * 20, 20))
                if not any(descriptor):
                    break
                imported_name(descriptor[3])
            else:
                raise ValueError('Unterminated PE import table')
    if directory_count > 13:
        delay_rva, delay_size = number('<II', directory + 13 * 8)
        if delay_rva:
            if delay_size > 1024 * 1024 or delay_size < 32:
                raise ValueError('Invalid delay-import table size')
            for index in range(min(delay_size // 32, 1024)):
                descriptor = number('<IIIIIIII', offset(delay_rva + index * 32, 32))
                if not any(descriptor):
                    break
                if descriptor[0] != 1:
                    raise ValueError('Unsupported non-RVA delay-import descriptor')
                imported_name(descriptor[1])
            else:
                raise ValueError('Unterminated delay-import table')
    return {'machine': machine, 'optionalMagic': magic, 'subsystem': subsystem, 'isDll': bool(characteristics & 0x2000), 'executableImage': bool(characteristics & 2), 'imports': sorted(set(imports))}


SYSTEM_DLLS = set('advapi32 bcrypt bcryptprimitives cfgmgr32 combase comctl32 comdlg32 crypt32 cryptbase d2d1 dbghelp dnsapi dwmapi dwrite dxgi gdi32 imm32 iphlpapi kernel32 kernelbase msimg32 ncrypt netapi32 normaliz ntdll ole32 oleaut32 powrprof profapi propsys psapi rpcrt4 secur32 setupapi shell32 shlwapi synchronization ucrtbase user32 userenv usp10 uxtheme version winhttp wininet winmm winspool wintrust ws2_32 wtsapi32'.split())


VC_DLLS = set('vcruntime140 vcruntime140_1 msvcp140 msvcp140_1 msvcp140_2 msvcp140_atomic_wait msvcp140_codecvt_ids concrt140 vccorlib140 vcamp140 vcomp140'.split())


def validate_roles(binaries, contract):
    if set(binaries) != {'desktop', 'cli'}:
        raise ValueError('Both actual desktop and console Cargo artifacts are required')
    result = {}
    for role, policy in contract['roles'].items():
        info = pe_info(binaries[role])
        if info['machine'] != 0x8664 or info['optionalMagic'] != 0x20b or info['isDll'] or not info['executableImage'] or info['subsystem'] != policy['peSubsystem']:
            raise ValueError(f'{role} is not the required real x64 PE role/subsystem')
        for dependency in info['imports']:
            stem = dependency[:-4]
            if dependency == 'webview2loader.dll':
                raise ValueError('This product contract requires the qualified static WebView2 Loader')
            if stem not in SYSTEM_DLLS | VC_DLLS and not stem.startswith(('api-ms-win-', 'ext-ms-win-')):
                raise ValueError(f'Unqualified native dependency in {role}: {dependency}')
        result[role] = {**info, 'filename': policy['filename'], 'bytes': len(binaries[role]), 'sha256': sha256(binaries[role]), 'workers': ['--render-worker', '--notes-edit-worker']}
    return result


def artifact_catalog(source, binaries, contract):
    """The native host consumes this exact closed shape and LF digest rule."""
    facts = validate_roles(binaries, contract)
    source_digest = sha256(zip_bytes(source))
    contract_digest = sha256(source['packaging/windows/host-contract.json'])
    roles = {
        role: {key: facts[role][key] for key in ('filename', 'sha256', 'bytes', 'subsystem')}
        for role in ('desktop', 'cli')
    }
    for item in roles.values():
        item['peSubsystem'] = item.pop('subsystem')
    # ASCII lowercase hex digests separated by exactly three LF bytes; no LF
    # follows the CLI hash. No JSON serializer order participates in buildId.
    identity = '\n'.join((source_digest, contract_digest, roles['desktop']['sha256'], roles['cli']['sha256']))
    value = {
        'schemaVersion': 1, 'buildId': sha256(identity.encode('ascii')),
        'sourceSha256': source_digest, 'hostContractSha256': contract_digest,
        'architecture': 'x64', 'roles': roles,
    }
    return json.dumps(value, indent=2).encode('utf-8')


def manifest(contract, version):
    def tag(name):
        prefix, local = name.split(':', 1) if ':' in name else ('', name)
        return f'{{{NS[prefix]}}}{local}'
    def add(parent, name, attrs=None, text=None):
        node = ET.SubElement(parent, tag(name), {tag(key) if ':' in key else key: str(value) for key, value in (attrs or {}).items()})
        node.text = text
        return node
    package = ET.Element(tag('Package'), {'IgnorableNamespaces': ' '.join(key for key in NS if key)})
    identity, platform, activation = (contract[key] for key in ('identity', 'platform', 'activation'))
    gui, cli = (contract['roles'][role]['filename'] for role in ('desktop', 'cli'))
    add(package, 'Identity', {'Name': identity['name'], 'Publisher': identity['publisher'], 'Version': version, 'ProcessorArchitecture': platform['architecture']})
    properties = add(package, 'Properties')
    for key, text in (('DisplayName', 'ms-resource:DisplayName'), ('PublisherDisplayName', 'Weftext'), ('Description', 'ms-resource:Description'), ('Logo', 'Assets/StoreLogo.png')):
        add(properties, key, text=text)
    excluded = add(add(properties, 'virtualization:FileSystemWriteVirtualization'), 'virtualization:ExcludedDirectories')
    paths = contract['paths']
    # Both roots contain durable author recovery; the historical browser path
    # is unchanged. Do not unvirtualize all AppData or unrelated registry data.
    for folder, root in (('persistentKnownFolder', 'persistentRelativeRoot'),
                         ('browserDataKnownFolder', 'browserDataRelativeRoot')):
        relative_root = paths[root].replace('/', '\\')
        add(excluded, 'virtualization:ExcludedDirectory', text=f"$(KnownFolder:{paths[folder]})\\{relative_root}")
    add(add(package, 'Dependencies'), 'TargetDeviceFamily', {'Name': 'Windows.Desktop', 'MinVersion': platform['minVersion'], 'MaxVersionTested': platform['maxVersionTested']})
    resources = add(package, 'Resources')
    for language in ('zh-CN', 'en-US'):
        add(resources, 'Resource', {'Language': language})
    application = add(add(package, 'Applications'), 'Application', {'Id': identity['applicationId'], 'Executable': gui, 'uap10:RuntimeBehavior': 'packagedClassicApp', 'uap10:TrustLevel': 'mediumIL', 'uap10:SupportsMultipleInstances': 'true'})
    add(application, 'uap:VisualElements', {'DisplayName': 'ms-resource:DisplayName', 'Description': 'ms-resource:Description', 'BackgroundColor': '#163B51', 'Square150x150Logo': 'Assets/Square150x150Logo.png', 'Square44x44Logo': 'Assets/Square44x44Logo.png'})
    extensions = add(application, 'Extensions')
    startup = add(extensions, 'desktop:Extension', {'Category': 'windows.startupTask', 'Executable': gui, 'uap10:RuntimeBehavior': 'packagedClassicApp', 'uap10:TrustLevel': 'mediumIL', 'uap10:Parameters': activation['startupArgument']})
    add(startup, 'desktop:StartupTask', {'TaskId': activation['startupTaskId'], 'Enabled': 'false', 'DisplayName': 'ms-resource:DisplayName'})
    server = add(add(add(extensions, 'com:Extension', {'Category': 'windows.comServer'}), 'com:ComServer'), 'com:ExeServer', {'Executable': gui, 'Arguments': activation['toastArgument'], 'DisplayName': 'Weftext'})
    add(server, 'com:Class', {'Id': activation['toastClsid'], 'DisplayName': 'Weftext'})
    add(add(extensions, 'desktop:Extension', {'Category': 'windows.toastNotificationActivation'}), 'desktop:ToastNotificationActivation', {'ToastActivatorCLSID': activation['toastClsid']})
    alias = add(extensions, 'uap5:Extension', {'Category': 'windows.appExecutionAlias', 'Executable': cli, 'uap10:RuntimeBehavior': 'packagedClassicApp', 'uap10:TrustLevel': 'mediumIL'})
    add(add(alias, 'uap5:AppExecutionAlias', {'uap10:Subsystem': 'console'}), 'uap5:ExecutionAlias', {'Alias': cli})
    capabilities = add(package, 'Capabilities')
    for capability in ('runFullTrust', 'unvirtualizedResources'):
        add(capabilities, 'rescap:Capability', {'Name': capability})
    ET.indent(package)
    return ET.tostring(package, encoding='utf-8', xml_declaration=True)


def sdk_tools(directory):
    directory = Path(directory).absolute()
    result = {}
    for tool in ('makeappx.exe', 'makepri.exe', 'rc.exe'):
        path = directory / tool
        data = read_checked(path, directory)
        info = pe_info(data)
        if info['machine'] != 0x8664:
            raise ValueError('Use the x64 Windows SDK packaging tools')
        from windows_trust import file_version
        result[tool] = {'path': path, 'sha256': sha256(data), 'bytes': len(data), 'fileVersion': list(file_version(path))}
    return result


def run_sdk(tools, tool, args, directory, label):
    log = directory / 'sdk-logs' / label
    log.mkdir(parents=True, exist_ok=False)
    path = tools[tool]['path']
    if sha256(read_checked(path, path.parent)) != tools[tool]['sha256']:
        raise ValueError('SDK tool changed after qualification')
    command = [str(path), *map(str, args)]
    record = {'command': command, 'toolSha256': tools[tool]['sha256'], 'startedUtc': datetime.now(timezone.utc).isoformat()}
    (log / 'intent.json').write_text(json.dumps(record, indent=2), encoding='utf-8')
    with (log / 'stdout.bin').open('xb') as out, (log / 'stderr.bin').open('xb') as err:
        process = subprocess.Popen(command, cwd=directory, stdin=subprocess.DEVNULL, stdout=out, stderr=err)
        record['pid'] = process.pid
        try:
            record['exitCode'] = process.wait()
        except BaseException:
            record['interrupted'] = True
            try:
                if process.poll() is None:
                    process.terminate()
                try:
                    record['exitCode'] = process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    record['exitCode'] = process.wait(timeout=10)
                record['ownedProcessStopped'] = True
            except BaseException as cleanup_error:
                record['ownedProcessStopped'] = False
                record['cleanupError'] = repr(cleanup_error)
                raise
            finally:
                record['endedUtc'] = datetime.now(timezone.utc).isoformat()
                (log / 'result.json').write_text(json.dumps(record, indent=2), encoding='utf-8')
            raise
    record['endedUtc'] = datetime.now(timezone.utc).isoformat()
    (log / 'result.json').write_text(json.dumps(record, indent=2), encoding='utf-8')
    if record['exitCode']:
        raise RuntimeError(f'{tool} {label} failed; original output retained at {log}')


def verify_containers(common, manifest_bytes, msix, portable):
    """Check complete uncompressed bytes, not just archive names or CRCs."""
    with zipfile.ZipFile(msix) as packed, zipfile.ZipFile(portable) as zipped:
        if len(zipped.namelist()) != len(set(zipped.namelist())) or set(zipped.namelist()) != set(common):
            raise ValueError('Portable ZIP inventory differs from frozen payload')
        expected_package = set(common) | {'AppxManifest.xml', 'AppxBlockMap.xml', '[Content_Types].xml'}
        if len(packed.namelist()) != len(set(packed.namelist())) or set(packed.namelist()) != expected_package:
            raise ValueError('MSIX has unqualified or duplicate container members')
        for name, data in common.items():
            if packed.read(name) != data or zipped.read(name) != data:
                raise ValueError(f'Container payload differs from frozen bytes: {name}')
        if packed.read('AppxManifest.xml') != manifest_bytes or 'AppxSignature.p7x' in packed.namelist():
            raise ValueError('Unexpected signed/changed package manifest')
        blockmap = ET.fromstring(packed.read('AppxBlockMap.xml'))
        if blockmap.attrib['HashMethod'] != 'http://www.w3.org/2001/04/xmlenc#sha256':
            raise ValueError('Unexpected blockmap hash method')
        names = set()
        for member in blockmap:
            name = member.attrib['Name'].replace('\\', '/')
            data = packed.read(name)
            if name in names or len(data) != int(member.attrib['Size']):
                raise ValueError('Invalid blockmap file inventory')
            names.add(name)
            blocks = member.findall('{http://schemas.microsoft.com/appx/2010/blockmap}Block')
            expected = [data[index:index + 65536] for index in range(0, len(data), 65536)]
            if len(blocks) != len(expected):
                raise ValueError('Invalid blockmap block count')
            for block, content in zip(blocks, expected):
                if base64.b64decode(block.attrib['Hash'], validate=True).hex() != sha256(content):
                    raise ValueError(f'Invalid blockmap hash: {name}')
            full = member.find('{http://schemas.microsoft.com/appx/2021/blockmap}FileHash')
            if full is not None and base64.b64decode(full.attrib['Hash'], validate=True).hex() != sha256(data):
                raise ValueError(f'Invalid complete-file blockmap hash: {name}')
        if names != set(common) | {'AppxManifest.xml'}:
            raise ValueError('MSIX contains an unexpected payload inventory')


def package_windows(root, artifacts, version, contract, materials, tools, *, output_root):
    if os.name != 'nt':
        raise ValueError('Windows packaging requires Windows and the qualified x64 SDK')
    artifacts.verify_inputs(root)
    roles = validate_roles(artifacts.binaries, contract)
    if (artifacts.catalog_path is None or artifacts.catalog_path.name != 'artifact-catalog.json'
            or any(path.parent != artifacts.catalog_path.parent for path in artifacts.paths.values())
            or artifacts.catalog is None or artifacts.catalog != artifact_catalog(artifacts.source, artifacts.binaries, contract)):
        raise ValueError('A complete candidate requires the exact fixed-build artifact catalog')
    vc_version = next(record['version'] for record in materials['records'] if record['id'] == 'vc-runtime')
    if artifacts.msvc_version is None or quad(vc_version) < artifacts.msvc_version:
        raise ValueError('Offline VC Runtime does not meet the exact MSVC linker used by this build')
    package_version = msix_version(version)
    common, packages = dependency_licenses(root, artifacts.metadata, artifacts.built_package_ids)
    common.update(materials['files'])
    common['artifact-catalog.json'] = artifacts.catalog
    for role, policy in contract['roles'].items():
        common[policy['filename']] = artifacts.binaries[role]
    for name in ('LICENSE', 'NOTICE', 'app/example.typ'):
        common[Path(name).name] = artifacts.source[name]
    for language in ('', '.zh-CN'):
        common[f'README{language}.txt'] = artifacts.source[f'packaging/windows/README{language}.txt']
    for name, data in artifacts.source.items():
        if name.startswith('packaging/windows/assets/'):
            common['Assets/' + name.rsplit('/', 1)[-1]] = data
    required_assets = {'Assets/StoreLogo.png', 'Assets/Square150x150Logo.png', 'Assets/Square44x44Logo.png', 'Assets/weftext.ico'}
    if not required_assets <= set(common):
        raise ValueError('Missing production Windows icon assets')
    common['source.zip'] = zip_bytes(artifacts.source)
    common['DEPENDENCIES.json'] = json.dumps({'scope': 'Actual Cargo build participants and installed npm build dependencies; includes build/test licenses, not a claim that every package is loaded at runtime.', 'packages': packages, 'offline': materials['records']}, ensure_ascii=False, indent=2).encode()
    build = {
        'schemaVersion': 1,
        'status': 'unsigned local candidate; installation, trust and DSK09/10 remain unqualified',
        'version': version, 'msixVersion': package_version, 'profile': artifacts.profile, 'roles': roles,
        'msvcLinkerVersion': list(artifacts.msvc_version),
        'sourceSha256': sha256(common['source.zip']),
        'sourceInventory': inventory(artifacts.source),
        'embeddedFrontend': inventory(artifacts.frontend),
        'hostContractSha256': sha256(artifacts.source['packaging/windows/host-contract.json']),
        'artifactCatalogSha256': sha256(artifacts.catalog),
        'buildId': parse_json(artifacts.catalog)['buildId'],
        'recognition': {
            'runtimeIncluded': False, 'setup': 'source.zip/scripts/recognition/README.md',
            'runtimeLockSha256': sha256(artifacts.source['resources/recognition/runtime-lock.json']),
        },
        'offline': materials['records'], 'upgradeQualified': False,
    }
    common['BUILD.json'] = json.dumps(build, ensure_ascii=False, indent=2).encode()
    manifest_bytes = manifest(contract, package_version)
    # All input/role/license/material checks precede this create-new boundary.
    stamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')
    destination = output_root / 'dist' / f'weftext-{version}-local-{stamp}'
    destination.mkdir(parents=True, exist_ok=False)
    try:
        payload = destination / 'payload'
        for name, data in common.items():
            path = payload / name
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open('xb') as stream:
                stream.write(data)
        (destination / 'AppxManifest.xml').write_bytes(manifest_bytes)
        resource_input = destination / 'resource-input'
        for language in ('zh-CN', 'en-US'):
            target = resource_input / 'Strings' / language / 'Resources.resw'
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(artifacts.source[f'packaging/windows/strings/{language}/Resources.resw'])
        config = destination / 'priconfig.xml'
        config.write_bytes(artifacts.source['packaging/windows/priconfig.xml'])
        run_sdk(tools, 'makepri.exe', ['new', '/pr', resource_input, '/cf', config, '/mn', destination / 'AppxManifest.xml', '/of', payload / 'resources.pri'], destination, 'resources')
        common['resources.pri'] = read_checked(payload / 'resources.pri', payload)
        mapping = destination / 'mapping.txt'
        rows = ['[Files]', f'"{destination / "AppxManifest.xml"}" "AppxManifest.xml"']
        rows.extend(f'"{payload / name}" "{name.replace(chr(47), chr(92))}"' for name in sorted(common))
        mapping.write_text('\n'.join(rows) + '\n', encoding='utf-8')
        msix = destination / f'weftext-{version}-x64.msix'
        run_sdk(tools, 'makeappx.exe', ['pack', '/f', mapping, '/p', msix, '/no', '/h', 'SHA256'], destination, 'package')
        portable = destination / f'weftext-{version}-x64.zip'
        with portable.open('xb') as stream:
            stream.write(zip_bytes(common))
        verify_containers(common, manifest_bytes, msix, portable)
        receipt = {
            'schemaVersion': 1,
            'status': 'unsigned generated candidate; no installation or publication',
            'identity': contract['identity'], 'version': package_version, 'roles': roles,
            'buildId': build['buildId'], 'artifactCatalogSha256': sha256(artifacts.catalog),
            'commonPayload': inventory(common), 'manifestSha256': sha256(manifest_bytes),
            'containers': inventory({msix.name: msix.read_bytes(), portable.name: portable.read_bytes()}),
            'sdkTools': {
                name: {key: str(value) if isinstance(value, Path) else value for key, value in record.items()}
                for name, record in tools.items()
            },
            'signed': False, 'installed': False, 'upgradeQualified': False,
            'dataPreservationQualified': False,
        }
        (destination / 'PACKAGE.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2), encoding='utf-8')
    except BaseException as error:
        (destination / 'INCOMPLETE.json').write_text(json.dumps({'status': 'incomplete; preserve original outputs, do not install', 'error': str(error)}, ensure_ascii=False, indent=2), encoding='utf-8')
        raise
    print(f'Unsigned local candidate: {destination}')
    return destination
