"""Synthetic frozen-artifact boundaries; no compiler, npm install or product."""
import copy
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import uuid

import build_receipt as receipt
from package_inputs import inventory, snapshot
from test_windows_package import CONTRACT, CONTRACT_BYTES, pe
from windows_package import artifact_catalog


class Fixture:
    def __init__(self, directory):
        self.root = Path(directory) / 'source'
        self.target = Path(directory) / 'target'
        (self.root / 'packaging/windows').mkdir(parents=True)
        (self.root / 'app/dist').mkdir(parents=True)
        (self.root / 'scripts').mkdir()
        (self.root / 'packaging/windows/host-contract.json').write_bytes(CONTRACT_BYTES)
        (self.root / 'README.md').write_bytes(b'current documentation\n')
        (self.root / 'scripts/check.py').write_bytes(b'current check\n')
        (self.root / 'app/dist/index.html').write_bytes(b'embedded frontend\n')
        self.batch = self.target / 'weftext-build-artifacts' / str(uuid.uuid4())
        self.batch.mkdir(parents=True)
        self.path = self.batch / receipt.RECEIPT
        self.metadata = {'target_directory': str(self.target), 'workspace_root': str(self.root),
                         'packages': [{'id': 'path+file:///fixture#weftext-app@0.7.0'}]}
        self.qualification = {'tools': {'fixture': 'actual tool identity'},
                              'declaredConfig': {'sha256': 'a' * 64, 'sdkObservation': 'implicit-unobserved'}}
        source, frontend = snapshot(self.root, [self.target]), snapshot(self.root / 'app/dist')
        binaries = {'desktop': pe(2), 'cli': pe(3)}
        paths = {role: self.batch / policy['filename'] for role, policy in CONTRACT['roles'].items()}
        for role, path in paths.items():
            path.write_bytes(binaries[role])
        catalog = artifact_catalog(source, binaries, CONTRACT)
        catalog_path = self.batch / 'artifact-catalog.json'
        catalog_path.write_bytes(catalog)
        self.artifacts = receipt.QualifiedArtifacts(
            self.root, paths, binaries, source, frontend, self.metadata,
            frozenset([self.metadata['packages'][0]['id']]), self.target, 'debug',
            catalog_path=catalog_path, catalog=catalog,
            receipt_path=self.path, qualification=self.qualification)
        receipt.publish_receipt(self.artifacts)

    def load(self, **kwargs):
        with patch.object(receipt, 'qualify', return_value=self.qualification):
            return receipt.load_receipt(kwargs.get('path', self.path), kwargs.get('root', self.root),
                                        kwargs.get('metadata', self.metadata), kwargs.get('profile', 'debug'), {}, None)

    def change_receipt(self, mutation):
        value = json.loads(self.path.read_text())
        mutation(value)
        self.path.write_text(json.dumps(value), encoding='utf-8')


class ReceiptBoundary(unittest.TestCase):
    def test_roundtrip_actual_pe_roles_no_passes_and_mtime_is_not_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            original = fixture.path.read_bytes()
            os.utime(fixture.root / 'README.md', (1, 1))
            result = fixture.load()
            self.assertEqual(result.paths, fixture.artifacts.paths)
            self.assertEqual(result.binaries, fixture.artifacts.binaries)
            self.assertEqual(result.receipt_path, fixture.path)
            self.assertEqual(original, fixture.path.read_bytes())
            self.assertFalse(any('pass' in key.lower() for key in json.loads(original)))

    def test_every_source_and_frontend_change_requires_new_batch(self):
        mutations = (
            lambda f: (f.root / 'README.md').write_bytes(b'changed docs'),
            lambda f: (f.root / 'scripts/check.py').write_bytes(b'changed check'),
            lambda f: (f.root / 'added.txt').write_bytes(b'new source'),
            lambda f: (f.root / 'README.md').unlink(),
            lambda f: (f.root / 'app/dist/index.html').write_bytes(b'new embedded bytes'),
            lambda f: f.artifacts.paths['cli'].write_bytes(pe(3) + b'changed'),
            lambda f: f.artifacts.paths['desktop'].write_bytes(pe(3)),
            lambda f: f.artifacts.catalog_path.write_bytes(b'changed catalog'),
        )
        for mutation in mutations:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                fixture = Fixture(directory)
                mutation(fixture)
                with self.assertRaises(ValueError):
                    fixture.load()

    def test_closed_shape_type_inventory_roles_and_cargo_participants(self):
        mutations = (
            lambda v: v.update(unknown=True),
            lambda v: v.update(schemaVersion=True),
            lambda v: v.update(schemaVersion=2),
            lambda v: v['roles'].pop('desktop'),
            lambda v: v['roles']['cli'].update(filename='../elsewhere.exe'),
            lambda v: v['roles']['cli'].update(bytes=True),
            lambda v: v['source'][0].update(unknown=1),
            lambda v: v.update(features=['not-built']),
            lambda v: v.update(metadataSha256='0' * 64),
            lambda v: v.update(builtPackageIds=['unknown-cargo-package']),
            lambda v: v.update(builtPackageIds=[True]),
            lambda v: v['qualification'].update(passCache=True),
        )
        for mutation in mutations:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                fixture = Fixture(directory)
                fixture.change_receipt(mutation)
                with self.assertRaises(ValueError):
                    fixture.load()

    def test_duplicate_nonfinite_and_incomplete_receipts_are_rejected(self):
        for payload in (b'{"schemaVersion":1,"schemaVersion":1}', b'{"x":NaN}', b'{'):
            with self.subTest(payload=payload), tempfile.TemporaryDirectory() as directory:
                fixture = Fixture(directory)
                fixture.path.write_bytes(payload)
                with self.assertRaises(ValueError):
                    fixture.load()

    def test_profile_target_checkout_and_batch_must_match(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            for kwargs in ({'profile': 'release'}, {'root': fixture.target},
                           {'metadata': {**fixture.metadata, 'target_directory': str(fixture.target / 'another')}},
                           {'path': Path(receipt.RECEIPT)},
                           {'path': fixture.path.with_name('other.json')}):
                with self.subTest(kwargs=kwargs), self.assertRaises((ValueError, OSError)):
                    fixture.load(**kwargs)
            other = fixture.target / 'weftext-build-artifacts' / str(uuid.uuid4())
            other.mkdir()
            (other / receipt.RECEIPT).write_bytes(fixture.path.read_bytes())
            with self.assertRaises(ValueError):
                fixture.load(path=other / receipt.RECEIPT)

    def test_old_batch_without_receipt_is_not_signed_or_recovered(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            fixture.path.unlink()
            with self.assertRaises(FileNotFoundError):
                fixture.load()
            self.assertFalse(fixture.path.exists())

    def test_publication_is_create_new_and_incomplete_publication_is_not_usable(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            original = fixture.path.read_bytes()
            with self.assertRaises(FileExistsError):
                receipt.publish_receipt(fixture.artifacts)
            self.assertEqual(fixture.path.read_bytes(), original)
            with self.assertRaisesRegex(ValueError, 'publication is incomplete'):
                fixture.load()

    def test_top_level_tools_and_declared_configuration_change_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            for key in ('tools', 'declaredConfig'):
                changed = copy.deepcopy(fixture.qualification)
                changed[key]['changed'] = 'a new input'
                with self.subTest(key=key), patch.object(receipt, 'qualify', return_value=changed):
                    with self.assertRaises(ValueError):
                        receipt.load_receipt(fixture.path, fixture.root, fixture.metadata, 'debug', {}, None)
            # Implicit SDK is deliberately not a fabricated complete environment identity.
            self.assertEqual(fixture.load().qualification['declaredConfig']['sdkObservation'], 'implicit-unobserved')

    def test_reparse_classification_rejects_role_before_open(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            inode = fixture.artifacts.paths['cli'].lstat().st_ino
            with patch('package_inputs.is_reparse', side_effect=lambda info: info.st_ino == inode):
                with self.assertRaisesRegex(ValueError, 'symlink/reparse'):
                    fixture.load()


class NpmAndConfiguration(unittest.TestCase):
    def installed(self, directory):
        root = Path(directory) / 'node_modules/npm'
        (root / 'bin').mkdir(parents=True)
        (root / 'lib').mkdir()
        (root / 'node_modules/nested').mkdir(parents=True)
        (root / 'package.json').write_text(json.dumps({'name': 'npm', 'version': '11.17.0', 'bin': {'npm': 'bin/npm-cli.js'}}))
        (root / 'bin/npm-cli.js').write_text("require('../lib/cli.js')\n")
        (root / 'lib/cli.js').write_text('original actual code\n')
        (root / 'node_modules/nested/index.js').write_text('original bundled code\n')
        node = Path(directory) / 'node.exe'
        node.write_bytes(b'actual selected Node fixture')
        return receipt.NpmTool(node, root / 'bin/npm-cli.js', root)

    def test_nested_npm_implementation_changes_without_shim_or_version_change(self):
        with tempfile.TemporaryDirectory() as directory:
            tool = self.installed(directory)
            output = lambda command, *_: 'v24.19.0' if len(command) == 2 else '11.17.0'
            with patch.object(receipt, '_output', side_effect=output):
                before = tool.identity(Path(directory), {})
                (tool.root / 'node_modules/nested/index.js').write_text('changed bundled implementation')
                after = tool.identity(Path(directory), {})
            self.assertNotEqual(before['npm']['implementation'], after['npm']['implementation'])
            self.assertEqual(before['npm']['cli'], after['npm']['cli'])
            self.assertEqual(before['npm']['version'], after['npm']['version'])

    def test_unreadable_npm_subtree_cannot_be_accepted_as_a_complete_implementation(self):
        with tempfile.TemporaryDirectory() as directory:
            tool = self.installed(directory)
            hidden = tool.root / 'node_modules/nested'
            original_scandir = os.scandir
            def denied(path):
                if not isinstance(path, int) and Path(path) == hidden:
                    raise PermissionError('owned npm fixture subtree')
                return original_scandir(path)
            with patch.object(receipt.os, 'scandir', side_effect=denied):
                for content in ('first implementation', 'changed hidden implementation'):
                    (hidden / 'index.js').write_text(content)
                    with self.assertRaises(PermissionError):
                        tool.identity(Path(directory), {})

    def test_windows_standard_layout_resolves_real_cli_and_explicit_selected_node(self):
        with tempfile.TemporaryDirectory() as directory:
            tool = self.installed(directory)
            cmd = Path(directory) / 'npm.cmd'
            cmd.write_text('this shim is not executed')
            with patch.object(receipt, 'selected_node', return_value=tool.node), \
                    patch.object(receipt, 'os', type('Windows', (), {'name': 'nt'})), \
                    patch.object(receipt.shutil, 'which', return_value=str(cmd)):
                actual = receipt.NpmTool.discover({})
            self.assertEqual(actual.command('ci'), [str(tool.node), str(tool.cli), 'ci'])
            manifest = json.loads((tool.root / 'package.json').read_text())
            manifest['bin']['npm'] = 'custom.js'
            (tool.root / 'package.json').write_text(json.dumps(manifest))
            with patch.object(receipt, 'selected_node', return_value=tool.node), \
                    patch.object(receipt, 'os', type('Windows', (), {'name': 'nt'})), \
                    patch.object(receipt.shutil, 'which', return_value=str(cmd)):
                with self.assertRaises(ValueError):
                    receipt.NpmTool.discover({})

    def test_posix_resolved_real_cli_accepts_package_bin_and_rejects_custom_wrapper(self):
        with tempfile.TemporaryDirectory() as directory:
            tool = self.installed(directory)
            with patch.object(receipt, 'selected_node', return_value=tool.node), \
                    patch.object(receipt, 'os', type('Posix', (), {'name': 'posix'})), \
                    patch.object(receipt.shutil, 'which', return_value=str(tool.cli)):
                actual = receipt.NpmTool.discover({})
            self.assertEqual(actual.cli, tool.cli.resolve())
            wrapper = Path(directory) / 'custom-bin/npm'
            wrapper.parent.mkdir()
            wrapper.write_text('a custom shell wrapper')
            with patch.object(receipt, 'selected_node', return_value=tool.node), \
                    patch.object(receipt, 'os', type('Posix', (), {'name': 'posix'})), \
                    patch.object(receipt.shutil, 'which', return_value=str(wrapper)):
                with self.assertRaises((ValueError, OSError)):
                    receipt.NpmTool.discover({})

    def test_config_digest_binds_flags_profile_files_and_no_secret_values(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tool = self.installed(directory)
            values = {key: 'null' for key in receipt.NPM_CONFIG_KEYS}
            values.update(userconfig=str(root / '.npmrc'), globalconfig=str(root / 'etc/npmrc'))
            config_output = '\n'.join(f'{key}={value}' for key, value in values.items())
            with patch.object(receipt, '_output', return_value=config_output):
                before = receipt.declared_config(root, {}, tool)
                for env in ({'RUSTFLAGS': 'secret-build-flag'}, {'CARGO_PROFILE_DEV_DEBUG': '1'}, {'NODE_OPTIONS': 'secret-option'}):
                    with self.subTest(env=tuple(env)):
                        changed = receipt.declared_config(root, env, tool)
                        self.assertNotEqual(before['sha256'], changed['sha256'])
                        self.assertNotIn('secret', json.dumps(changed))
                (root / '.cargo').mkdir()
                (root / '.cargo/config.toml').write_text('[build]\njobs=2\n')
                self.assertNotEqual(before['sha256'], receipt.declared_config(root, {}, tool)['sha256'])
                (root / '.npmrc').write_text('//registry.invalid/:_authToken=secret-token\n')
                result = receipt.declared_config(root, {}, tool)
                self.assertNotIn('secret-token', json.dumps(result))
                self.assertNotIn('PATH', json.dumps(result))


class DirectCargoToolQualification(unittest.TestCase):
    def tools(self, root, environment):
        cargo = root / ('cargo.exe' if os.name == 'nt' else 'cargo')
        cargo.write_bytes(b'owned Cargo fixture')
        cargo.chmod(0o755)
        environment = {**environment, 'PATH': str(root)}
        class Npm:
            def identity(self, *_):
                return {'node': 'owned fixture', 'npm': 'owned fixture'}
        def identity(command, *_):
            path = Path(command)
            return {'command': str(command),
                    'bytes': path.read_bytes().hex() if path.is_file() else 'default tool'}
        with patch.object(receipt, 'tool_identity', side_effect=identity), \
                patch.object(receipt, 'declared_config', return_value={'sha256': 'fixture'}):
            return receipt.qualify(root, environment, Npm())

    def test_cargo_build_rustc_binds_actual_bytes_and_rustc_has_precedence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            selected, override = root / 'selected.exe', root / 'override.exe'
            selected.write_bytes(b'first actual compiler')
            override.write_bytes(b'explicit override compiler')
            environment = {'CARGO_HOME': str(root / 'cargo'), 'CARGO_BUILD_RUSTC': str(selected)}
            before = self.tools(root, environment)
            selected.write_bytes(b'replaced same-path compiler')
            after = self.tools(root, environment)
            self.assertNotEqual(before['tools']['rustc'], after['tools']['rustc'])
            self.assertEqual(after['tools']['rustc']['command'], str(selected))
            chosen = self.tools(root, {**environment, 'RUSTC': str(override)})
            self.assertEqual(chosen['tools']['rustc']['command'], str(override))
            with self.assertRaises(ValueError):
                self.tools(root, {**environment, 'RUSTC': ''})

    def test_unresolved_config_tools_refuse_qualification_until_explicitly_selected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / '.cargo').mkdir()
            config = root / '.cargo/config.toml'
            environment = {'CARGO_HOME': str(root / 'cargo')}
            cases = (
                ('[build]\nrustc="custom-compiler"\n', 'RUSTC', 'rustc'),
                ('[build]\nrustc-wrapper="custom-wrapper"\n', 'RUSTC_WRAPPER', ''),
                ('[build]\nrustc-workspace-wrapper="custom-wrapper"\n', 'RUSTC_WORKSPACE_WRAPPER', ''),
                ('[target.x86_64-pc-windows-msvc]\nlinker="custom-linker"\n',
                 'CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER', 'explicit-linker'),
            )
            for text, key, value in cases:
                config.write_text(text)
                with self.subTest(key=key), self.assertRaises(ValueError):
                    receipt._require_direct_cargo_tools(root, environment)
                receipt._require_direct_cargo_tools(root, {**environment, key: value})

    def test_includes_conditional_linkers_and_config_env_tools_never_silently_use_defaults(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / '.cargo').mkdir()
            config = root / '.cargo/config.toml'
            environment = {'CARGO_HOME': str(root / 'cargo'), 'RUSTC': 'rustc'}
            for text in ('include=["extra.toml"]\n',
                         '[target.\'cfg(windows)\']\nlinker="conditional"\n',
                         '[env]\nRUSTC="forced-compiler"\n',
                         '[env]\nRUSTUP_TOOLCHAIN={value="other",force=true}\n',
                         '[env]\nRUSTUP_HOME={value="other",force=true}\n',
                         '[env]\nCARGO_HOME={value="other",force=true}\n'):
                config.write_text(text)
                with self.subTest(text=text), self.assertRaises(ValueError):
                    receipt._require_direct_cargo_tools(root, environment)

    def test_relative_external_inputs_are_refused_across_distinct_caller_and_cargo_roots(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            root, caller = base / 'source', base / 'caller'
            root.mkdir()
            caller.mkdir()
            for folder in (root, caller):
                (folder / 'tools').mkdir()
                (folder / 'tools/wrapper.exe').write_bytes(str(folder).encode())
            tool = NpmAndConfiguration().installed(base)
            values = {key: 'null' for key in receipt.NPM_CONFIG_KEYS}
            values.update(userconfig=str(base / '.npmrc'), globalconfig=str(base / 'etc/npmrc'))
            output = '\n'.join(f'{key}={value}' for key, value in values.items())
            environment = {'CARGO_HOME': str(base / 'cargo'), 'PATH': str(root / 'tools')}
            previous = Path.cwd()
            try:
                os.chdir(caller)
                with patch.object(receipt, '_output', return_value=output):
                    for key, value in (
                            ('CARGO_HOME', '../cargo'), ('RUSTC_WRAPPER', 'tools/wrapper.exe'),
                            ('CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER', 'tools/wrapper.exe'),
                            ('WEFTEXT_WINDOWS_SDK_BIN', 'tools')):
                        with self.subTest(key=key), self.assertRaisesRegex(ValueError, 'absolute'):
                            receipt.declared_config(root, {**environment, key: value}, tool)
                    with self.assertRaisesRegex(ValueError, 'absolute'):
                        self.tools(root, {**environment, 'RUSTC': 'tools/wrapper.exe'})
                    with self.assertRaisesRegex(ValueError, 'absolute'):
                        receipt._direct_tool('wrapper', {'PATH': 'tools'})
                    selected = {**environment, 'RUSTC_WRAPPER': str(root / 'tools/wrapper.exe')}
                    before = receipt.declared_config(root, selected, tool)
                    (root / 'tools/wrapper.exe').write_bytes(b'replaced actual root wrapper')
                    after = receipt.declared_config(root, selected, tool)
                    self.assertNotEqual(before, after)
                    (base / 'cargo').mkdir()
                    config = base / 'cargo/config.toml'
                    config.write_text('[build]\nrustflags=["--cfg=first"]\n')
                    before = receipt.declared_config(root, selected, tool)
                    config.write_text('[build]\nrustflags=["--cfg=second"]\n')
                    self.assertNotEqual(before, receipt.declared_config(root, selected, tool))
                    if os.name == 'nt':
                        # No program runs: actual Windows lookup finds a different
                        # source-root executable when the parent caller differs.
                        (root / 'wrapper.exe').write_bytes(b'root shadow')
                        with self.assertRaisesRegex(ValueError, 'shadowing'):
                            receipt._direct_tool('wrapper', environment, root)
            finally:
                os.chdir(previous)


if __name__ == '__main__':
    unittest.main()
