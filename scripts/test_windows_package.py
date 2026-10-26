"""Packaging boundary regressions. Synthetic bytes are never a product candidate."""
import base64
import copy
import io
import json
import os
from pathlib import Path, PureWindowsPath
import struct
import subprocess
import tempfile
import tracemalloc
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch
import xml.etree.ElementTree as ET

import mvp
from _cli_serve import serve_command
from package_inputs import BuildArtifacts, collect_artifact, dependency_licenses, read_checked, require_checkout, sha256, snapshot, validate_target, zip_bytes
from windows_materials import freeze_materials, validate_record
from windows_package import NS, artifact_catalog, load_contract, manifest, msix_version, pe_info, run_sdk, validate_roles, verify_containers

ROOT = Path(__file__).resolve().parents[1]
CONTRACT_BYTES = (ROOT / 'packaging/windows/host-contract.json').read_bytes()
CONTRACT = load_contract(CONTRACT_BYTES)


def pe(subsystem, machine=0x8664, dll=False):
    """Minimal parse fixture only. It is not loaded or packaged for execution."""
    data = bytearray(1024)
    data[:2] = b'MZ'
    struct.pack_into('<I', data, 60, 128)
    data[128:132] = b'PE\0\0'
    struct.pack_into('<HH', data, 132, machine, 1)
    struct.pack_into('<HH', data, 148, 240, 0x22 | (0x2000 if dll else 0))
    optional = 152
    struct.pack_into('<H', data, optional, 0x20b)
    struct.pack_into('<I', data, optional + 16, 0x1000)
    struct.pack_into('<I', data, optional + 56, 0x2000)
    struct.pack_into('<I', data, optional + 60, 512)
    struct.pack_into('<H', data, optional + 68, subsystem)
    struct.pack_into('<I', data, optional + 108, 16)
    section = optional + 240
    data[section:section + 5] = b'.text'
    struct.pack_into('<IIII', data, section + 8, 512, 0x1000, 512, 512)
    struct.pack_into('<I', data, section + 36, 0x60000020)
    data[512] = 0xc3
    return bytes(data)


def containers(common, manifest_bytes, *, extra=False, corrupt_hash=False):
    uri = 'http://schemas.microsoft.com/appx/2010/blockmap'
    blockmap = ET.Element('BlockMap', {'xmlns': uri, 'HashMethod': 'http://www.w3.org/2001/04/xmlenc#sha256'})
    for name, data in {**common, 'AppxManifest.xml': manifest_bytes}.items():
        node = ET.SubElement(blockmap, 'File', {'Name': name, 'Size': str(len(data)), 'LfhSize': '30'})
        for offset in range(0, len(data), 65536):
            digest = bytes.fromhex(sha256(data[offset:offset + 65536]))
            ET.SubElement(node, 'Block', {'Hash': base64.b64encode(b'0' * 32 if corrupt_hash else digest).decode()})
        ET.SubElement(node, '{http://schemas.microsoft.com/appx/2021/blockmap}FileHash', {'Hash': base64.b64encode(bytes.fromhex(sha256(data))).decode()})
    files = {**common, 'AppxManifest.xml': manifest_bytes, 'AppxBlockMap.xml': ET.tostring(blockmap), '[Content_Types].xml': b'<Types/>'}
    if extra:
        files['unqualified.dll'] = b'not in the approved payload'
    return io.BytesIO(zip_bytes(files)), io.BytesIO(zip_bytes(common))


class VersionAndPolicy(unittest.TestCase):
    def test_duplicate_authoritative_json_fields_are_rejected(self):
        duplicate = CONTRACT_BYTES.decode().replace('"schemaVersion": 1,', '"schemaVersion": 1, "schemaVersion": 1,', 1)
        self.assertNotEqual(duplicate, CONTRACT_BYTES.decode())
        with self.assertRaisesRegex(ValueError, 'Duplicate JSON field'):
            load_contract(duplicate)

    def test_rc_stable_order_and_bounds(self):
        self.assertEqual(msix_version('0.7.0-rc.0'), '0.7.0.0')
        self.assertEqual(msix_version('0.7.0-rc.65534'), '0.7.0.65534')
        self.assertEqual(msix_version('0.7.0'), '0.7.0.65535')
        self.assertLess(tuple(map(int, msix_version('0.7.0').split('.'))), tuple(map(int, msix_version('0.7.1-rc.0').split('.'))))
        for value in ('0.7.0-rc.65535', '65536.0.0', '0.7.0-beta.1', '0.7.0+new', '0.7.0-rc.01'):
            with self.subTest(value=value), self.assertRaises(ValueError):
                msix_version(value)

    def test_persistent_scope_is_closed_even_if_sdk_accepts_profile(self):
        for key, value in (('persistentKnownFolder', 'Profile'), ('persistentRelativeRoot', '../Private'), ('browserDataRelativeRoot', 'Weftext-MVP/cache')):
            changed = copy.deepcopy(CONTRACT)
            changed['paths'][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                load_contract(json.dumps(changed))
        changed = copy.deepcopy(CONTRACT)
        changed['additionalWriter'] = True
        with self.assertRaises(ValueError):
            load_contract(json.dumps(changed))


    def test_persistent_browser_data_keeps_one_original_physical_root(self):
        paths = CONTRACT['paths']
        self.assertEqual(paths['browserDataKnownFolder'], 'LocalAppData')
        self.assertEqual(paths['browserDataRelativeRoot'], 'Weftext/Cache')
        for known_folder in ('Profile', 'RoamingAppData', 'LocalAppData/..', ''):
            changed = copy.deepcopy(CONTRACT)
            changed['paths']['browserDataKnownFolder'] = known_folder
            with self.subTest(known_folder=known_folder), self.assertRaises(ValueError):
                load_contract(json.dumps(changed))
        for relative_root in ('', '.', '..', 'Weftext', '../Weftext/Cache',
                              '/Weftext/Cache', 'C:/Weftext/Cache',
                              'Weftext/Cache/child', 'Weftext/../Cache'):
            changed = copy.deepcopy(CONTRACT)
            changed['paths']['browserDataRelativeRoot'] = relative_root
            with self.subTest(relative_root=relative_root), self.assertRaises(ValueError):
                load_contract(json.dumps(changed))
        # There is one current contract, not an old/new field alias or fallback.
        for current, retired in (('browserDataKnownFolder', 'cacheKnownFolder'),
                                 ('browserDataRelativeRoot', 'cacheRelativeRoot')):
            changed = copy.deepcopy(CONTRACT)
            changed['paths'][retired] = changed['paths'].pop(current)
            with self.subTest(retired=retired), self.assertRaises(ValueError):
                load_contract(json.dumps(changed))

    def test_core_and_browser_data_roots_cannot_overlap_in_either_direction(self):
        for changes in (
            {'browserDataRelativeRoot': 'Weftext-MVP/core'},
            {'browserDataRelativeRoot': 'Weftext-MVP/core/browser-data'},
            {'browserDataRelativeRoot': 'Weftext-MVP'},
            {'persistentRelativeRoot': 'Weftext/Cache'},
            {'persistentRelativeRoot': 'Weftext/Cache/profile'},
        ):
            changed = copy.deepcopy(CONTRACT)
            changed['paths'].update(changes)
            paths = changed['paths']
            core = PureWindowsPath(paths['persistentRelativeRoot']) / paths['coreRelativePath']
            browser = PureWindowsPath(paths['browserDataRelativeRoot'])
            self.assertTrue(core.is_relative_to(browser) or browser.is_relative_to(core))
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                load_contract(json.dumps(changed))

    def test_core_and_private_host_state_are_required_disjoint_contract_children(self):
        self.assertEqual(CONTRACT['paths']['coreRelativePath'], 'core')
        self.assertEqual(CONTRACT['paths']['hostStateRelativePath'], 'host-state')
        for key in ('coreRelativePath', 'hostStateRelativePath'):
            changed = copy.deepcopy(CONTRACT)
            del changed['paths'][key]
            with self.subTest(missing=key), self.assertRaises(ValueError):
                load_contract(json.dumps(changed))
            for value in ('', '.', '..', '../core', '/core', 'core', 'core/child', 'host-state', 'host-state/child'):
                if value == CONTRACT['paths'][key]:
                    continue
                changed = copy.deepcopy(CONTRACT)
                changed['paths'][key] = value
                with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                    load_contract(json.dumps(changed))
        changed = copy.deepcopy(CONTRACT)
        changed['paths']['coreRelativePath'] = changed['paths']['hostStateRelativePath']
        with self.assertRaises(ValueError):
            load_contract(json.dumps(changed))

    def test_real_manifest_has_correct_activation_and_no_global_unvirtualization(self):
        tree = ET.fromstring(manifest(CONTRACT, '0.7.0.1'))
        ns = {key or 'f': value for key, value in NS.items()}
        excluded = tree.findall('.//virtualization:ExcludedDirectory', ns)
        self.assertEqual([item.text for item in excluded], [
            '$(KnownFolder:LocalAppData)\\Weftext-MVP',
            '$(KnownFolder:LocalAppData)\\Weftext\\Cache',
        ])
        # Two exact owned trees, never all LocalAppData or a registry opt-out.
        self.assertIsNone(tree.find('.//virtualization:RegistryWriteVirtualization', ns))
        desktop6 = 'http://schemas.microsoft.com/appx/manifest/desktop/windows10/6'
        for property_name in ('FileSystemWriteVirtualization', 'RegistryWriteVirtualization'):
            self.assertIsNone(tree.find(f'.//{{{desktop6}}}{property_name}'))
        capabilities = tree.findall('.//rescap:Capability', ns)
        self.assertEqual({item.attrib['Name'] for item in capabilities},
                         {'runFullTrust', 'unvirtualizedResources'})
        extensions = tree.findall('.//desktop:Extension', ns)
        startup = next(node for node in extensions if node.attrib['Category'] == 'windows.startupTask')
        self.assertEqual(startup.attrib['{' + NS['uap10'] + '}Parameters'], '--login-startup')
        self.assertEqual(startup.find('desktop:StartupTask', ns).attrib['Enabled'], 'false')
        self.assertEqual(tree.find('.//com:ExeServer', ns).attrib['Arguments'], '--toast-server')
        self.assertEqual(tree.find('.//com:Class', ns).attrib['Id'], tree.find('.//desktop:ToastNotificationActivation', ns).attrib['ToastActivatorCLSID'])
        self.assertEqual(tree.find('.//uap5:AppExecutionAlias', ns).attrib['{' + NS['uap10'] + '}Subsystem'], 'console')
        self.assertEqual(tree.find('.//uap5:Extension', ns).attrib['Executable'], 'weftext-cli.exe')


class ActualRoleSelection(unittest.TestCase):
    def test_catalog_binds_source_contract_and_both_actual_roles(self):
        source = {'packaging/windows/host-contract.json': CONTRACT_BYTES, 'author.rs': b'frozen source'}
        binaries = {'desktop': pe(2), 'cli': pe(3)}
        catalog = json.loads(artifact_catalog(source, binaries, CONTRACT))
        digests = [sha256(zip_bytes(source)), sha256(CONTRACT_BYTES), sha256(binaries['desktop']), sha256(binaries['cli'])]
        self.assertEqual(catalog['buildId'], sha256('\n'.join(digests).encode('ascii')))
        self.assertEqual(catalog['roles']['cli'], {'filename': 'weftext-cli.exe', 'sha256': digests[3], 'bytes': len(binaries['cli']), 'peSubsystem': 3})
        changed = {**source, 'author.rs': b'new source'}
        self.assertNotEqual(artifact_catalog(source, binaries, CONTRACT), artifact_catalog(changed, binaries, CONTRACT))
        changed_binaries = {**binaries, 'cli': binaries['cli'] + b'final resource bytes'}
        self.assertNotEqual(artifact_catalog(source, binaries, CONTRACT), artifact_catalog(source, changed_binaries, CONTRACT))

    def test_explicit_console_serve_keeps_initial_document_and_port(self):
        exe = ROOT / 'owned/weftext-cli.exe'
        arguments = ['--data-dir', str(ROOT / 'owned/state'), '--open', 'initial.typ', '--root', 'library', '--port=19']
        self.assertEqual(serve_command(exe, arguments), [str(exe), 'serve', *arguments])
        self.assertEqual(serve_command(exe, ['--data-dir', 'state'])[-2:], ['--port', '0'])
        for arguments in (['--no-browser'], ['serve'], ['--root=library']):
            with self.subTest(arguments=arguments), self.assertRaises(ValueError):
                serve_command(exe, arguments)
        with self.assertRaises(ValueError):
            serve_command('guessed.exe', [])

    def test_cargo_identity_and_order_cannot_select_another_package(self):
        paths = {}
        def message(package, target, filename):
            return {'reason': 'compiler-artifact', 'package_id': package, 'target': {'name': target, 'kind': ['bin']}, 'executable': str(Path(filename).absolute())}
        collect_artifact(message('other', 'weftext', 'wrong.exe'), 'actual', CONTRACT['roles'], paths)
        collect_artifact(message('actual', 'weftext-cli', 'console.exe'), 'actual', CONTRACT['roles'], paths)
        collect_artifact(message('actual', 'weftext', 'desktop.exe'), 'actual', CONTRACT['roles'], paths)
        self.assertEqual(set(paths), {'desktop', 'cli'})
        self.assertEqual(paths['desktop'].name, 'desktop.exe')
        with self.assertRaises(ValueError):
            collect_artifact(message('actual', 'weftext', 'conflicting.exe'), 'actual', CONTRACT['roles'], paths)

    def test_renaming_does_not_change_real_pe_role(self):
        validate_roles({'desktop': pe(2), 'cli': pe(3)}, CONTRACT)
        for roles in ({'desktop': pe(2)}, {'desktop': pe(3), 'cli': pe(2)}, {'desktop': pe(2, machine=0x14c), 'cli': pe(3)}, {'desktop': pe(2, dll=True), 'cli': pe(3)}):
            with self.subTest(), self.assertRaises(ValueError):
                validate_roles(roles, CONTRACT)

    def test_truncated_and_unbacked_entrypoints_fail(self):
        for data in (b'MZ', pe(2)[:200], pe(2)[:800]):
            with self.subTest(length=len(data)), self.assertRaises(ValueError):
                pe_info(data)
        data = bytearray(pe(2))
        struct.pack_into('<I', data, 152 + 16, 0x6000)
        with self.assertRaises(ValueError):
            pe_info(data)

    def test_dynamic_loader_and_unqualified_delay_import_are_rejected(self):
        for name in (b'WebView2Loader.dll\0', b'private-runtime.dll\0', b'vcruntime140-private.dll\0'):
            data = bytearray(pe(2))
            struct.pack_into('<II', data, 152 + 112 + 13 * 8, 0x1040, 64)
            struct.pack_into('<II', data, 512 + 64, 1, 0x10a0)
            data[512 + 160:512 + 160 + len(name)] = name
            with self.subTest(name=name), self.assertRaises(ValueError):
                validate_roles({'desktop': bytes(data), 'cli': pe(3)}, CONTRACT)

    def test_system_com_runtime_import_is_qualified_for_both_actual_roles(self):
        for role, subsystem in (('desktop', 2), ('cli', 3)):
            for delayed in (False, True):
                data = bytearray(pe(subsystem))
                directory = 13 if delayed else 1
                struct.pack_into('<II', data, 152 + 112 + directory * 8, 0x1040, 64 if delayed else 40)
                if delayed:
                    struct.pack_into('<II', data, 512 + 64, 1, 0x10a0)
                else:
                    struct.pack_into('<IIIII', data, 512 + 64, 0, 0, 0, 0x10a0, 0)
                name = b'COMBASE.dll\0'
                data[512 + 160:512 + 160 + len(name)] = name
                binaries = {'desktop': pe(2), 'cli': pe(3), role: bytes(data)}
                with self.subTest(role=role, delayed=delayed):
                    facts = validate_roles(binaries, CONTRACT)
                    self.assertEqual(facts[role]['imports'], ['combase.dll'])

    def test_import_name_cannot_continue_into_unmapped_pe_overlay(self):
        data = bytearray(pe(2)) + bytearray(32)
        struct.pack_into('<II', data, 152 + 112 + 8, 0x1040, 40)
        struct.pack_into('<IIIII', data, 512 + 64, 0, 0, 0, 0x11fe, 0)
        name = b'kernel32.dll\0'
        data[1022:1022 + len(name)] = name
        with self.assertRaisesRegex(ValueError, 'file-backed section'):
            pe_info(data)


class InputAndContainerBoundaries(unittest.TestCase):
    def test_git_source_uses_current_public_bytes_without_reading_ignored_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(['git', 'init', '-q', str(root)], stdin=subprocess.DEVNULL, check=True, capture_output=True)
            (root / '.gitignore').write_bytes(b'secret.env\ncache/\n')
            tracked = root / 'tracked.txt'
            tracked.write_bytes(b'index bytes')
            deleted = root / 'removed.txt'
            deleted.write_bytes(b'old deleted source')
            replaced = root / 'old-file'
            replaced.write_bytes(b'old indexed file')
            subprocess.run(['git', '-C', str(root), 'add', 'tracked.txt', 'removed.txt', 'old-file'], stdin=subprocess.DEVNULL, check=True, capture_output=True)
            tracked.write_bytes(b'current uncommitted bytes')
            deleted.unlink()
            replaced.unlink()
            replaced.mkdir()
            (replaced / 'new.rs').write_bytes(b'public source in replacement directory')
            (root / 'public-new.txt').write_bytes(b'visible untracked public source')
            (root / 'secret.env').write_bytes(b'owned ignored private fixture')
            (root / 'cache').mkdir()
            (root / 'cache/private.txt').write_bytes(b'owned ignored cache fixture')
            read_paths = []
            def only_public(path, qualified_root, **options):
                relative = Path(path).relative_to(qualified_root).as_posix()
                self.assertNotIn(relative, {'secret.env', 'cache/private.txt'})
                read_paths.append(relative)
                return read_checked(path, qualified_root, **options)
            with patch('package_inputs.read_checked', side_effect=only_public):
                files = snapshot(root)
            self.assertEqual(files, {'.gitignore': b'secret.env\ncache/\n', 'tracked.txt': b'current uncommitted bytes', 'public-new.txt': b'visible untracked public source', 'old-file/new.rs': b'public source in replacement directory'})
            self.assertEqual(set(read_paths), set(files))
            self.assertEqual((root / 'secret.env').read_bytes(), b'owned ignored private fixture')
            self.assertEqual((root / 'cache/private.txt').read_bytes(), b'owned ignored cache fixture')
            require_checkout(root)
            with self.assertRaises(ValueError):
                require_checkout(root / 'cache')

    def test_small_file_does_not_allocate_its_large_policy_budget(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / 'license.txt'
            path.write_bytes(b'a' * 1024)
            tracemalloc.start()
            try:
                self.assertEqual(len(read_checked(path, root, max_bytes=8 * 1024 * 1024)), 1024)
                self.assertLess(tracemalloc.get_traced_memory()[1], 1024 * 1024)
            finally:
                tracemalloc.stop()

    def test_interrupted_sdk_is_stopped_before_logs_close(self):
        for needs_kill in (False, True):
            with self.subTest(needs_kill=needs_kill), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                tool = root / 'owned-sdk.exe'
                tool.write_bytes(b'owned test fixture; never executed')
                process = Mock(pid=12345)
                process.poll.return_value = None
                process.wait.side_effect = [KeyboardInterrupt(), *([subprocess.TimeoutExpired('owned-sdk', 10)] if needs_kill else []), 1]
                tools = {'owned-sdk': {'path': tool, 'sha256': sha256(tool.read_bytes())}}
                with patch('windows_package.subprocess.Popen', return_value=process):
                    with self.assertRaises(KeyboardInterrupt):
                        run_sdk(tools, 'owned-sdk', [], root, 'interrupted')
                process.terminate.assert_called_once()
                self.assertEqual(process.kill.call_count, int(needs_kill))
                record = json.loads((root / 'sdk-logs/interrupted/result.json').read_bytes())
                self.assertTrue(record['ownedProcessStopped'])
                self.assertEqual(record['exitCode'], 1)

    def test_lexical_escape_and_oversized_input_are_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            outer = Path(directory)
            root = outer / 'source'
            root.mkdir()
            private = outer / 'private.txt'
            private.write_bytes(b'PRIVATE')
            with self.assertRaises(ValueError):
                read_checked(root / '../private.txt', root)
            (root / 'large.txt').write_bytes(b'12345')
            with self.assertRaisesRegex(ValueError, 'byte budget'):
                read_checked(root / 'large.txt', root, max_bytes=4)

    def test_illegal_target_overlap_is_refused(self):
        root = ROOT.absolute()
        for target in (root, root / 'app/generated', root / 'crates/cache'):
            with self.subTest(target=target), self.assertRaises(ValueError):
                validate_target(root, target)
        self.assertEqual(validate_target(root, root / 'target/isolated'), root / 'target/isolated')

    def test_source_and_executable_changes_after_freeze_are_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'app/dist').mkdir(parents=True)
            (root / 'app/dist/ui.js').write_bytes(b'embedded')
            (root / 'author.py').write_bytes(b'original')
            target = root / 'target'
            target.mkdir()
            exe = target / 'cli.exe'
            exe.write_bytes(pe(3))
            artifact = BuildArtifacts(root, {'cli': exe}, {'cli': exe.read_bytes()}, snapshot(root, [target]), snapshot(root / 'app/dist'), {}, frozenset(), target, 'debug')
            artifact.verify_inputs(root)
            catalog_path = target / 'artifact-catalog.json'
            catalog_path.write_bytes(b'fixed catalog fixture')
            artifact = BuildArtifacts(root, artifact.paths, artifact.binaries, artifact.source, artifact.frontend, {}, frozenset(), target, 'debug', catalog_path=catalog_path, catalog=b'fixed catalog fixture')
            catalog_path.write_bytes(b'changed catalog fixture')
            with self.assertRaisesRegex(ValueError, 'catalog changed'):
                artifact.verify_inputs(root)
            catalog_path.write_bytes(artifact.catalog)
            exe.write_bytes(pe(2))
            with self.assertRaises(ValueError):
                artifact.verify_inputs(root)
            exe.write_bytes(artifact.binaries['cli'])
            (root / 'author.py').write_bytes(b'new source')
            with self.assertRaises(ValueError):
                artifact.verify_inputs(root)

    def test_source_links_never_read_their_target(self):
        with tempfile.TemporaryDirectory() as directory:
            outer = Path(directory)
            root = outer / 'source'
            root.mkdir()
            private = outer / 'original.private'
            private.write_bytes(b'PRIVATE ORIGINAL')
            try:
                (root / 'linked.txt').symlink_to(private)
            except OSError as error:
                if os.name != 'nt':
                    self.skipTest(f'OS did not permit an owned symbolic-link fixture: {error}')
                # A junction needs no Developer Mode or changed privileges.
                # Both ends are new, owned fixture directories; no user path is
                # read or modified. shutil's Windows cleanup unlinks junctions.
                target = outer / 'private-directory'
                target.mkdir()
                (target / 'original.private').write_bytes(private.read_bytes())
                environment = {**os.environ, 'WEFTEXT_PACKAGE_TEST_LINK': str(root / 'linked-directory'), 'WEFTEXT_PACKAGE_TEST_TARGET': str(target)}
                code = "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:WEFTEXT_PACKAGE_TEST_LINK -Value $env:WEFTEXT_PACKAGE_TEST_TARGET | Out-Null"
                encoded = base64.b64encode(code.encode('utf-16-le')).decode()
                subprocess.run(['powershell.exe', '-NoProfile', '-NonInteractive', '-EncodedCommand', encoded], env=environment, stdin=subprocess.DEVNULL, check=True, capture_output=True)
            with self.assertRaises(ValueError):
                snapshot(root)
            self.assertEqual(private.read_bytes(), b'PRIVATE ORIGINAL')

    def test_missing_license_original_is_not_an_empty_success(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            dependency = root / 'dependency'
            dependency.mkdir()
            metadata = {'packages': [{'id': 'actual', 'name': 'missing-license', 'version': '1', 'source': 'registry', 'manifest_path': str(dependency / 'Cargo.toml'), 'license': 'MIT'}]}
            with self.assertRaisesRegex(ValueError, 'Missing original license'):
                dependency_licenses(root, metadata, {'actual'})

    def test_external_cargo_path_dependency_is_not_omitted(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'source'
            root.mkdir()
            metadata = {'packages': [{'id': 'external', 'name': 'outside', 'version': '1', 'source': None, 'manifest_path': str(root.parent / 'dependency/Cargo.toml')}]}
            with self.assertRaisesRegex(ValueError, 'outside the frozen source'):
                dependency_licenses(root, metadata, {'external'})

    def test_complete_bytes_and_2021_file_hash_are_checked(self):
        common, xml = {'weftext.exe': b'frozen bytes' * 10000}, b'<Package/>'
        packed, portable = containers(common, xml)
        verify_containers(common, xml, packed, portable)
        for option in ('extra', 'corrupt_hash'):
            packed, portable = containers(common, xml, **{option: True})
            with self.subTest(option=option), self.assertRaises(ValueError):
                verify_containers(common, xml, packed, portable)
        packed, portable = containers(common, xml)
        with self.assertRaises(ValueError):
            verify_containers({'weftext.exe': b'different bytes'}, xml, packed, portable)


class OriginalLicenseBoundaries(unittest.TestCase):
    """Synthetic originals exercise the full collector, including fonts and SDK."""

    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.provenance_path = self.root / 'packaging/windows/licenses/provenance.json'
        self.cargo_folder = self.root / 'registry/same-name'
        self.npm_folder = self.root / 'app/node_modules/same-name'
        self.cargo_folder.mkdir(parents=True)
        self.npm_folder.mkdir(parents=True)
        self._put('app/node_modules/same-name/package.json', json.dumps({'name': 'same-name', 'version': '1.2.3', 'license': 'MIT OR Apache-2.0'}).encode())
        self._put('app/package-lock.json', json.dumps({'packages': {'': {}, 'node_modules/same-name': {'version': '1.2.3'}}}).encode())
        self._put('resources/fonts/LICENSE.txt', b'synthetic font original\r\n')
        for name in ('manifest.json', 'builtin-manifest.json', 'web-manifest.json'):
            self._put('resources/fonts/' + name, b'{}\n')
        sdk_folder = self.root / 'registry/webview2-com-sys'
        self._put('registry/webview2-com-sys/LICENSE.txt', b'synthetic Rust binding license\n')
        loader = self._put('registry/webview2-com-sys/x64/WebView2LoaderStatic.lib', b'synthetic loader only')
        self.metadata = {'packages': [
            {'id': 'actual', 'name': 'same-name', 'version': '1.2.3', 'source': 'registry', 'manifest_path': str(self.cargo_folder / 'Cargo.toml'), 'license': 'MIT OR Apache-2.0'},
            {'id': 'sdk', 'name': 'webview2-com-sys', 'version': '0.39.1', 'source': 'registry', 'manifest_path': str(sdk_folder / 'Cargo.toml'), 'license': 'MIT'},
        ]}
        self.built = {'actual', 'sdk'}
        self.provenance = {
            'schemaVersion': 2,
            'rust': {'same-name-1.2.3': {'files': [
                self._record('packaging/windows/licenses/upstream/rust/LICENSE.txt', b'synthetic Rust license\r\n'),
                self._record('packaging/windows/licenses/upstream/rust/NOTICE.txt', b'synthetic Rust notice\n'),
            ]}},
            'npm': {'same-name-1.2.3': {'files': [
                self._record('packaging/windows/licenses/upstream/npm/LICENSE.txt', b'synthetic npm license\n'),
                self._record('packaging/windows/licenses/upstream/npm/NOTICE.txt', b'synthetic npm notice\r\n'),
            ]}},
            'webview2Sdk': {
                'version': 'synthetic-1', 'sysCrateVersion': '0.39.1',
                'staticLoaderX64Sha256': sha256(loader),
                'files': {
                    'LICENSE.txt': self._record('packaging/windows/licenses/sdk/LICENSE.txt', b'synthetic Microsoft license\n'),
                    'NOTICE.txt': self._record('packaging/windows/licenses/sdk/NOTICE.txt', b'synthetic Microsoft notice\n'),
                },
            },
        }

    def _put(self, relative, data):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        return data

    def _record(self, relative, data):
        self._put(relative, data)
        return {'file': relative, 'sha256': sha256(data), 'size': len(data), 'sourceUrl': 'https://example.invalid/synthetic-original'}

    def _collect(self, provenance=None):
        self.provenance_path.parent.mkdir(parents=True, exist_ok=True)
        self.provenance_path.write_text(json.dumps(self.provenance if provenance is None else provenance), encoding='utf-8')
        return dependency_licenses(self.root, self.metadata, self.built)

    def test_cargo_and_npm_multifile_fallbacks_preserve_each_original_byte(self):
        files, packages = self._collect()
        for ecosystem, section in (('cargo', 'rust'), ('npm', 'npm')):
            expected = []
            for record in self.provenance[section]['same-name-1.2.3']['files']:
                target = f'third-party/{ecosystem}-same-name-1.2.3/{Path(record["file"]).name}'
                expected.append(target)
                self.assertEqual(files[target], (self.root / record['file']).read_bytes())
            package = next(value for value in packages if value['ecosystem'] == ecosystem and value['name'] == 'same-name')
            self.assertEqual(package['licenseFiles'], expected)
        self.assertEqual(files['third-party/microsoft-webview2-sdk-synthetic-1/NOTICE.txt'], b'synthetic Microsoft notice\n')
        self.assertIn('FONTS-web-manifest.json', files)

    def test_existing_nonempty_originals_are_not_replaced_by_fallbacks(self):
        for folder in (self.cargo_folder, self.npm_folder):
            (folder / 'LICENSE.local').write_bytes(b'actual installed original\r\n')
        files, packages = self._collect()
        for ecosystem in ('cargo', 'npm'):
            prefix = f'third-party/{ecosystem}-same-name-1.2.3/'
            self.assertEqual(files[prefix + 'LICENSE.local'], b'actual installed original\r\n')
            self.assertNotIn(prefix + 'LICENSE.txt', files)
            package = next(value for value in packages if value['ecosystem'] == ecosystem and value['name'] == 'same-name')
            self.assertEqual(package['licenseFiles'], [prefix + 'LICENSE.local'])

    def test_explicit_license_selection_preserves_declaration_and_original_bytes(self):
        changed = copy.deepcopy(self.provenance)
        for section in ('rust', 'npm'):
            changed[section]['same-name-1.2.3'].update({
                'selectedLicense': 'Apache-2.0',
                'declarationSourceUrls': ['https://example.invalid/upstream/declaration'],
                'sourceKind': 'synthetic-official-original-and-upstream-declaration',
            })
        files, packages = self._collect(changed)
        for ecosystem, section in (('cargo', 'rust'), ('npm', 'npm')):
            package = next(value for value in packages if value['ecosystem'] == ecosystem and value['name'] == 'same-name')
            self.assertEqual(package['license'], 'MIT OR Apache-2.0')
            self.assertEqual(package['selectedLicense'], 'Apache-2.0')
            for record in changed[section]['same-name-1.2.3']['files']:
                target = f'third-party/{ecosystem}-same-name-1.2.3/{Path(record["file"]).name}'
                self.assertEqual(files[target], (self.root / record['file']).read_bytes())

    def test_invalid_explicit_license_selection_is_rejected(self):
        for section in ('rust', 'npm'):
            for value in (None, False, '', ' \r\n\t', 42, []):
                changed = copy.deepcopy(self.provenance)
                changed[section]['same-name-1.2.3']['selectedLicense'] = value
                with self.subTest(section=section, value=value), self.assertRaisesRegex(ValueError, 'Invalid selected upstream license'):
                    self._collect(changed)

    def test_missing_or_other_version_never_qualifies_a_package(self):
        for section in ('rust', 'npm'):
            for key in ('same-name', 'same-name-9.9.9'):
                changed = copy.deepcopy(self.provenance)
                changed[section][key] = changed[section].pop('same-name-1.2.3')
                with self.subTest(section=section, key=key), self.assertRaisesRegex(ValueError, 'Missing original license'):
                    self._collect(changed)

    def test_other_ecosystem_does_not_supply_a_missing_original(self):
        for section in ('rust', 'npm'):
            changed = copy.deepcopy(self.provenance)
            changed[section] = {}
            with self.subTest(section=section), self.assertRaisesRegex(ValueError, 'Missing original license'):
                self._collect(changed)

    def test_missing_provenance_and_empty_defaults_do_not_invent_originals(self):
        with self.assertRaisesRegex(ValueError, 'Missing original license'):
            dependency_licenses(self.root, self.metadata, self.built)
        for value in ({'schemaVersion': 2}, {'schemaVersion': 2, 'rust': {}, 'npm': {}}):
            with self.subTest(value=value), self.assertRaisesRegex(ValueError, 'Missing original license'):
                self._collect(value)

    def test_schema_one_missing_or_noninteger_version_is_rejected(self):
        for version in (None, 1, True, '2', 2.0):
            changed = copy.deepcopy(self.provenance)
            if version is None:
                del changed['schemaVersion']
            else:
                changed['schemaVersion'] = version
            with self.subTest(version=version), self.assertRaisesRegex(ValueError, 'provenance schema'):
                self._collect(changed)

    def test_sections_and_files_records_have_one_current_shape(self):
        for section in ('rust', 'npm'):
            for value in (None, [], 'not a section'):
                changed = copy.deepcopy(self.provenance)
                changed[section] = value
                with self.subTest(section=section, value=value), self.assertRaisesRegex(ValueError, 'provenance section'):
                    self._collect(changed)
            scalar = self.provenance[section]['same-name-1.2.3']['files'][0]
            for value in (scalar, {'files': [], **scalar}, {'files': []}, {'files': 'LICENSE'}, {'files': [None]}):
                changed = copy.deepcopy(self.provenance)
                changed[section]['same-name-1.2.3'] = value
                with self.subTest(section=section, value=value), self.assertRaises(ValueError):
                    self._collect(changed)

    def test_second_original_wrong_size_or_hash_is_not_hidden_by_first(self):
        for section in ('rust', 'npm'):
            for field, value in (('size', 9999), ('sha256', '0' * 64)):
                changed = copy.deepcopy(self.provenance)
                changed[section]['same-name-1.2.3']['files'][1][field] = value
                with self.subTest(section=section, field=field), self.assertRaisesRegex(ValueError, 'Changed upstream license original'):
                    self._collect(changed)

    def test_bad_size_and_hash_types_are_rejected(self):
        for section in ('rust', 'npm'):
            for field, values in (('size', (True, 0, -1, '1', 1.0)), ('sha256', (None, 'short', 'G' * 64, 'A' * 64))):
                for value in values:
                    changed = copy.deepcopy(self.provenance)
                    changed[section]['same-name-1.2.3']['files'][1][field] = value
                    with self.subTest(section=section, field=field, value=value), self.assertRaisesRegex(ValueError, 'Invalid upstream original'):
                        self._collect(changed)

    def test_second_original_missing_or_directory_is_not_hidden_by_first(self):
        for section in ('rust', 'npm'):
            for relative in ('packaging/windows/licenses/missing/NOTICE.txt', 'resources/fonts'):
                changed = copy.deepcopy(self.provenance)
                changed[section]['same-name-1.2.3']['files'][1]['file'] = relative
                with self.subTest(section=section, relative=relative), self.assertRaises((ValueError, OSError)):
                    self._collect(changed)

    def test_matching_whitespace_second_original_is_rejected(self):
        for section in ('rust', 'npm'):
            changed = copy.deepcopy(self.provenance)
            second = changed[section]['same-name-1.2.3']['files'][1]
            path = self.root / second['file']
            original = path.read_bytes()
            try:
                second.update(self._record(second['file'], b' \t\r\n'))
                with self.subTest(section=section), self.assertRaisesRegex(ValueError, 'Changed upstream license original'):
                    self._collect(changed)
            finally:
                path.write_bytes(original)

    def test_escaping_or_noncanonical_original_paths_are_rejected(self):
        for section in ('rust', 'npm'):
            for value in ('../LICENSE', '/LICENSE', 'C:/outside/LICENSE', '//server/share/LICENSE', r'folder\LICENSE', 'folder/../LICENSE', './LICENSE'):
                changed = copy.deepcopy(self.provenance)
                changed[section]['same-name-1.2.3']['files'][1]['file'] = value
                with self.subTest(section=section, value=value), self.assertRaisesRegex(ValueError, 'Invalid upstream original path'):
                    self._collect(changed)

    def test_second_original_read_errors_are_not_hidden(self):
        for section in ('rust', 'npm'):
            target = self.root / self.provenance[section]['same-name-1.2.3']['files'][1]['file']
            original_read = read_checked
            def deny_selected(path, root, **keywords):
                if Path(path) == target:
                    raise PermissionError('synthetic original read denial')
                return original_read(path, root, **keywords)
            with self.subTest(section=section), patch('package_inputs.read_checked', side_effect=deny_selected), self.assertRaisesRegex(PermissionError, 'synthetic original read denial'):
                self._collect()

    def test_dependency_directory_enumeration_denial_cannot_become_fallback(self):
        original_scandir = os.scandir
        for folder in (self.cargo_folder, self.npm_folder):
            def deny_selected(path):
                if Path(path) == folder:
                    raise PermissionError('synthetic dependency directory enumeration denial')
                return original_scandir(path)
            with self.subTest(folder=folder), patch('package_inputs.os.scandir', side_effect=deny_selected), self.assertRaisesRegex(PermissionError, 'synthetic dependency directory enumeration denial'):
                self._collect()

    def test_snapshot_keeps_original_bytes_and_excludes_installed_dependencies(self):
        files = snapshot(self.root)
        self.assertEqual(files['resources/fonts/LICENSE.txt'], b'synthetic font original\r\n')
        self.assertEqual(files['app/package-lock.json'], (self.root / 'app/package-lock.json').read_bytes())
        self.assertFalse(any(name.startswith('app/node_modules/') for name in files))

    def test_snapshot_root_and_nested_enumeration_denials_propagate(self):
        original_scandir = os.scandir
        for folder in (self.root, self.root / 'resources/fonts'):
            def deny_selected(path):
                if Path(path) == folder:
                    raise PermissionError('synthetic source directory enumeration denial')
                return original_scandir(path)
            with self.subTest(folder=folder), patch('package_inputs.os.scandir', side_effect=deny_selected), self.assertRaisesRegex(PermissionError, 'synthetic source directory enumeration denial'):
                snapshot(self.root)

    def test_second_original_reparse_is_not_followed(self):
        import package_inputs
        for section in ('rust', 'npm'):
            target = self.root / self.provenance[section]['same-name-1.2.3']['files'][1]['file']
            inode = target.lstat().st_ino
            original_reparse = package_inputs.is_reparse
            def selected_reparse(info):
                return original_reparse(info) or info.st_ino == inode
            with self.subTest(section=section), patch('package_inputs.is_reparse', side_effect=selected_reparse), self.assertRaisesRegex(ValueError, 'reparse input'):
                self._collect()

    def test_duplicate_casefold_payload_basename_is_rejected(self):
        for section in ('rust', 'npm'):
            changed = copy.deepcopy(self.provenance)
            changed[section]['same-name-1.2.3']['files'][1] = self._record('packaging/windows/licenses/other/license.TXT', b'independent original\n')
            with self.subTest(section=section), self.assertRaisesRegex(ValueError, 'Duplicate upstream license payload basename'):
                self._collect(changed)

    def test_an_existing_empty_license_is_not_masked_by_valid_fallbacks(self):
        for folder in (self.cargo_folder, self.npm_folder):
            path = folder / 'LICENSE.empty'
            try:
                path.write_bytes(b' \r\n\t')
                with self.subTest(folder=folder), self.assertRaisesRegex(ValueError, 'Empty original license text'):
                    self._collect()
            finally:
                path.unlink()

    def test_sdk_actual_binding_and_loader_requirements_still_apply(self):
        changed = copy.deepcopy(self.provenance)
        changed['webview2Sdk']['sysCrateVersion'] = 'other'
        with self.assertRaisesRegex(ValueError, 'Built WebView2 binding'):
            self._collect(changed)
        changed = copy.deepcopy(self.provenance)
        changed['webview2Sdk']['staticLoaderX64Sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'Built static WebView2 Loader'):
            self._collect(changed)


class OfflinePreflight(unittest.TestCase):
    def test_package_without_git_checkout_fails_before_sdk_or_build(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.object(mvp, 'ROOT', root), patch.object(mvp, 'os', SimpleNamespace(name='nt')), patch.object(mvp, 'sdk_tools') as sdk, patch.object(mvp, 'build') as build:
                with self.assertRaisesRegex(ValueError, 'actual Git product checkout'):
                    mvp.main(['package', '--windows-sdk', str(root / 'sdk'), '--offline-materials', str(root / 'materials'), '--msvc-linker', str(root / 'link.exe')])
                sdk.assert_not_called()
                build.assert_not_called()
            self.assertFalse((root / 'dist').exists())

    def test_missing_standalone_fails_before_build_or_candidate_output(self):
        record = json.loads((ROOT / 'packaging/windows/offline-materials.json').read_text())
        record['materials'] = [item for item in record['materials'] if item['id'] == 'vc-runtime']
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'packaging/windows').mkdir(parents=True)
            (root / 'crates/weftext-app').mkdir(parents=True)
            (root / 'packaging/windows/host-contract.json').write_bytes(CONTRACT_BYTES)
            (root / 'packaging/windows/offline-materials.json').write_text(json.dumps(record))
            (root / 'crates/weftext-app/Cargo.toml').write_text('[package]\nversion="0.7.0-rc.1"\n')
            with patch.object(mvp, 'ROOT', root), patch.object(mvp, 'os', SimpleNamespace(name='nt')), patch.object(mvp, 'require_checkout'), patch.object(mvp, 'sdk_tools', return_value={}), patch.object(mvp, 'build') as build:
                with self.assertRaisesRegex(ValueError, 'missing qualified repository material.*webview2'):
                    mvp.main(['package', '--windows-sdk', str(root / 'sdk'), '--offline-materials', str(root / 'materials'), '--msvc-linker', str(root / 'link.exe')])
                build.assert_not_called()
            self.assertFalse((root / 'dist').exists())

    def test_material_type_architecture_and_provenance_are_closed(self):
        original = next(item for item in json.loads((ROOT / 'packaging/windows/offline-materials.json').read_text())['materials'] if item['id'] == 'vc-runtime')
        validate_record(original)
        for key, value in (('kind', 'bootstrapper'), ('filename', '../runtime.exe'), ('sha256', 'unknown'), ('payloadArchitectures', ['arm64']), ('sourceUrl', 'https://unrelated.invalid/runtime.exe')):
            item = copy.deepcopy(original)
            item[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate_record(item)


if __name__ == '__main__':
    unittest.main()
