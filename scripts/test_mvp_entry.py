"""Entry selection and frozen role routing; subprocess execution is recorded only."""
from contextlib import contextmanager, redirect_stdout
import io
from pathlib import Path
import shutil
import tempfile
from types import SimpleNamespace
import unittest
import uuid
from unittest.mock import Mock, patch

import mvp
import build_receipt as receipt
from build_receipt import GROUPS
from test_build_receipt import Fixture


class NpmFixture:
    node = Path('selected-node')

    def environment(self, environment):
        return dict(environment)

    def command(self, *args):
        return ['selected-node', 'real-npm-cli.js', *map(str, args)]


class EntrySelection(unittest.TestCase):
    def setUp(self):
        self.original_license_collector = mvp.dependency_licenses
        collector = patch.object(mvp, 'dependency_licenses', side_effect=AssertionError('License preflight is package-only'))
        self.license_collector = collector.start()
        self.addCleanup(collector.stop)

    def test_tag_fixture_default_is_independent_of_deep_frozen_source(self):
        self._tag_fixture_parent(False)

    def test_tag_fixture_explicit_owned_parent_remains_selected(self):
        self._tag_fixture_parent(True)

    def _tag_fixture_parent(self, explicit):
        import test_hierarchical_tags as tags
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executable = root / 'fake-cli.exe'
            executable.write_bytes(b'entry-selection-only; never executed')
            deep_source = root / ('frozen-' * 45) / 'source/scripts/test_hierarchical_tags.py'
            selected = unittest.defaultTestLoader.getTestCaseNames(tags.HierarchicalTags)[0]
            arguments = [str(executable), selected]
            parent = root / 'evidence'
            if explicit:
                arguments.extend(['--artifacts-dir', str(parent)])
            result = SimpleNamespace(wasSuccessful=lambda: True)
            runner = SimpleNamespace(run=Mock(return_value=result))
            with patch.object(tags, '__file__', str(deep_source)), \
                    patch.object(tags, 'ARTIFACTS', None), \
                    patch.object(tags.test_hosted, 'EXE', None), \
                    patch.object(tags.unittest, 'TextTestRunner', return_value=runner), \
                    redirect_stdout(io.StringIO()):
                self.assertEqual(tags.main(arguments), 0)
                self.assertEqual(tags.ARTIFACTS, parent.resolve() if explicit else None)
                self.assertEqual([test._testMethodName for test in runner.run.call_args.args[0]], [selected])
                case = tags.HierarchicalTags(selected)
                server = SimpleNamespace(close=Mock())
                with patch.object(tags, 'Server', return_value=server), \
                        patch.object(case, 'call', return_value={}), \
                        patch.object(case, 'note', return_value={}):
                    case.setUp()
                self.assertEqual(case.folder.parent, parent.resolve() if explicit else Path(tempfile.gettempdir()).resolve())
                self.assertFalse(case.folder.is_relative_to(deep_source.parent))
                # Exercise the exact longest control-output name that failed
                # under frozen-source/target, without starting any process.
                control = case.folder / ('server-control-' + str(uuid.uuid4()))
                control.mkdir()
                output = control / 'owner-stop.stdout.bin'
                with output.open('xb') as stream:
                    stream.write(b'owned fixture, no process')
                self.assertEqual(output.read_bytes(), b'owned fixture, no process')
                case.tearDown()
                server.close.assert_called_once_with()

    def test_package_copy_uses_unique_nonredacted_token_and_never_overwrites(self):
        with tempfile.TemporaryDirectory() as directory:
            root, target = Path(directory) / 'checkout', Path(directory) / 'target'
            root.mkdir()
            target.mkdir()
            frozen = {'packaging/windows/host-contract.json': b'contract',
                      'app/package.json': b'{"name":"owned-source"}',
                      'original.txt': b'original source bytes'}
            tokens = [uuid.uuid4(), uuid.uuid4()]
            observed = []
            stopped = RuntimeError('stop before npm install or build')

            def before_install(*args, **kwargs):
                source = kwargs['cwd']
                observed.append(source)
                self.assertEqual(args, tuple(NpmFixture().command('--prefix', 'app', 'ci', '--ignore-scripts')))
                self.assertEqual({name: (source / name).read_bytes() for name in frozen}, frozen)
                raise stopped

            with patch.object(mvp, 'ROOT', root), \
                    patch.object(mvp.NpmTool, 'discover', return_value=NpmFixture()), \
                    patch.object(mvp, 'metadata', return_value={'target_directory': str(target)}), \
                    patch.object(mvp, 'snapshot', return_value=frozen), \
                    patch.object(mvp, 'load_contract', return_value={}), \
                    patch.object(mvp, 'qualify', return_value={}), \
                    patch.object(mvp, 'run', side_effect=before_install), \
                    patch.object(mvp.uuid, 'uuid4', side_effect=[*tokens, tokens[0]]):
                for _ in tokens:
                    with self.assertRaises(RuntimeError) as caught:
                        mvp.build(True, package_input=True)
                    self.assertIs(caught.exception, stopped)
                with self.assertRaises(FileExistsError):
                    mvp.build(True, package_input=True)
            self.assertEqual(observed, [target / 'weftext-package-inputs' / token.hex / 'source'
                                        for token in tokens])
            self.assertNotEqual(observed[0], observed[1])
            self.assertEqual((observed[0] / 'original.txt').read_bytes(), frozen['original.txt'])

    def test_source_only_needs_no_node_cargo_git_or_product(self):
        calls = []
        with patch.object(mvp, 'metadata', side_effect=AssertionError('Cargo forbidden')), \
                patch.object(mvp, 'build', side_effect=AssertionError('PE build forbidden')), \
                patch.object(mvp, 'require_checkout', side_effect=AssertionError('Git forbidden')), \
                patch.object(mvp.NpmTool, 'discover', side_effect=AssertionError('npm forbidden')), \
                patch.object(mvp, 'selected_node', side_effect=AssertionError('Node forbidden')), \
                patch.object(mvp, 'run', side_effect=lambda *args, **_: calls.append(args)), redirect_stdout(io.StringIO()) as output:
            mvp.main(['check', '--group', 'source'])
        self.assertEqual(calls, [(mvp.sys.executable, 'scripts/check.py', '--models')])
        self.assertIn('partial', output.getvalue())
        self.assertNotIn('Full product', output.getvalue())

    def test_frontend_only_uses_same_explicit_node_cli_without_cargo_or_pe_build(self):
        calls = []
        with patch.object(mvp, 'metadata', side_effect=AssertionError('Cargo forbidden')), \
                patch.object(mvp, 'build', side_effect=AssertionError('PE build forbidden')), \
                patch.object(mvp.NpmTool, 'discover', return_value=NpmFixture()), \
                patch.object(mvp, 'run', side_effect=lambda *args, **_: calls.append(args)), redirect_stdout(io.StringIO()) as output:
            mvp.main(['check', '--group', 'frontend'])
        self.assertEqual(calls, [tuple(NpmFixture().command('--prefix', 'app', *args))
                                 for args in [('ci', '--ignore-scripts'), ('test',), ('run', 'build')]])
        self.assertIn('partial', output.getvalue())

    def test_duplicate_and_reordered_groups_use_fixed_order(self):
        self.assertEqual(mvp.selected_groups(['http', 'source', 'http', 'cli']), ('source', 'cli', 'http'))
        with self.assertRaises(ValueError):
            mvp.selected_groups(['unknown'])

    def test_build_and_package_reject_groups_or_receipts_before_any_work(self):
        with patch.object(mvp, 'build') as build, patch.object(mvp, 'require_checkout') as checkout:
            for action in ('build', 'package'):
                for extra in (['--group', 'source'], ['--artifacts', '/not-used.json']):
                    with self.subTest(action=action, extra=extra), redirect_stdout(io.StringIO()), \
                            patch('sys.stderr', new=io.StringIO()), self.assertRaises(SystemExit):
                        mvp.main([action, *extra])
            build.assert_not_called()
            checkout.assert_not_called()

    def test_default_check_builds_once_and_requests_all_original_groups(self):
        artifact = object()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.assertFalse((root / '.git').exists())
            with patch.object(mvp, 'ROOT', root), \
                    patch.object(mvp, 'require_checkout', side_effect=AssertionError('Standalone check needs no Git')), \
                    patch.object(mvp, 'build', return_value=artifact) as build, patch.object(mvp, 'check') as check:
                result = mvp.main(['check'])
        self.assertIs(result, artifact)
        build.assert_called_once_with(False, package_input=False, linker=None, windows_sdk=None)
        check.assert_called_once_with(artifact, GROUPS)

    def test_plain_build_still_accepts_standalone_source_without_a_git_preflight(self):
        artifact = object()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.assertFalse((root / '.git').exists())
            with patch.object(mvp, 'ROOT', root), \
                    patch.object(mvp, 'require_checkout', side_effect=AssertionError('Standalone build needs no Git')), \
                    patch.object(mvp, 'build', return_value=artifact) as build, patch.object(mvp, 'check') as check:
                result = mvp.main(['build', '--release'])
        self.assertIs(result, artifact)
        build.assert_called_once_with(True, package_input=False, linker=None, windows_sdk=None)
        check.assert_not_called()

    def test_receipt_source_group_still_qualifies_before_partial_execution(self):
        artifact = object()
        meta = {'target_directory': '/fixture-target'}
        with patch.object(mvp, 'require_checkout') as checkout, \
                patch.object(mvp, 'metadata', return_value=meta) as metadata, \
                patch.object(mvp, 'validate_target', return_value=Path('/fixture-target')), \
                patch.object(mvp.NpmTool, 'discover', return_value=NpmFixture()), \
                patch.object(mvp, 'load_receipt', return_value=artifact) as load, \
                patch.object(mvp, 'build', side_effect=AssertionError('No rebuild')), \
                patch.object(mvp, 'check') as check:
            mvp.main(['check', '--release', '--artifacts', '/fixture-receipt.json', '--group', 'source'])
        checkout.assert_not_called()
        self.assertEqual(metadata.call_count, 2)
        self.assertEqual(load.call_args.args[3], 'release')
        check.assert_called_once_with(artifact, ('source',))

    def test_receipt_without_group_requests_full_check_without_build(self):
        artifact = object()
        with patch.object(mvp, 'require_checkout') as checkout, \
                patch.object(mvp, 'metadata', return_value={'target_directory': '/fixture-target'}), \
                patch.object(mvp, 'validate_target', return_value=Path('/fixture-target')), \
                patch.object(mvp.NpmTool, 'discover', return_value=NpmFixture()), \
                patch.object(mvp, 'load_receipt', return_value=artifact), \
                patch.object(mvp, 'build', side_effect=AssertionError('Receipt must not rebuild')), \
                patch.object(mvp, 'check') as check:
            mvp.main(['check', '--artifacts', '/fixture-receipt.json'])
        checkout.assert_not_called()
        check.assert_called_once_with(artifact, GROUPS)

    def test_actual_gitless_receipt_uses_the_cli_entry_without_rebuilding(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            original = fixture.path.read_bytes()
            self.assertFalse((fixture.root / '.git').exists())
            with patch.object(mvp, 'ROOT', fixture.root), \
                    patch.object(mvp, 'require_checkout', side_effect=AssertionError('Receipt source needs no Git')), \
                    patch.object(mvp, 'metadata', return_value=fixture.metadata) as metadata, \
                    patch.object(mvp.NpmTool, 'discover', return_value=NpmFixture()), \
                    patch.object(receipt, 'qualify', return_value=fixture.qualification) as qualify, \
                    patch.object(mvp, 'build', side_effect=AssertionError('Receipt must not rebuild')), \
                    patch.object(mvp, 'check') as check:
                result = mvp.main(['check', '--artifacts', str(fixture.path), '--group', 'cli'])
            self.assertIsInstance(result, receipt.QualifiedArtifacts)
            self.assertEqual(result.source_root, fixture.root)
            self.assertEqual(result.receipt_path, fixture.path)
            self.assertEqual(result.binaries, fixture.artifacts.binaries)
            self.assertEqual(metadata.call_count, 2)
            qualify.assert_called_once()
            check.assert_called_once_with(result, ('cli',))
            self.assertFalse((fixture.root / '.git').exists())
            self.assertEqual(fixture.path.read_bytes(), original)

    def test_actual_cli_rejects_identical_other_source_before_qualification_or_checks(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            other = Path(directory) / 'another-source'
            shutil.copytree(fixture.root, other)
            self.assertFalse((other / '.git').exists())
            self.assertEqual(receipt.snapshot(other, [fixture.target]), fixture.artifacts.source)
            self.assertEqual(receipt.snapshot(other / 'app/dist'), fixture.artifacts.frontend)
            original = fixture.path.read_bytes()
            with patch.object(mvp, 'ROOT', other), \
                    patch.object(mvp, 'require_checkout', side_effect=AssertionError('Receipt source needs no Git')), \
                    patch.object(mvp, 'metadata', return_value=fixture.metadata), \
                    patch.object(mvp.NpmTool, 'discover', return_value=NpmFixture()), \
                    patch.object(receipt, 'qualify', return_value=fixture.qualification) as qualify, \
                    patch.object(mvp, 'build', side_effect=AssertionError('Receipt must not rebuild')), \
                    patch.object(mvp, 'check') as check:
                with self.assertRaisesRegex(ValueError, 'Receipt version, checkout, target, batch or profile does not match'):
                    mvp.main(['check', '--artifacts', str(fixture.path), '--group', 'cli'])
            qualify.assert_not_called()
            check.assert_not_called()
            self.assertFalse((other / '.git').exists())
            self.assertEqual(fixture.path.read_bytes(), original)

    def test_package_git_preflight_failure_stops_before_tools_materials_or_build(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            failure = ValueError('Packaging requires an actual Git product checkout')
            with patch.object(mvp, 'ROOT', root), patch.object(mvp, 'os', SimpleNamespace(name='nt')), \
                    patch.object(mvp, 'require_checkout', side_effect=failure) as checkout, \
                    patch.object(mvp, 'read_checked', side_effect=AssertionError('No input read before Git')), \
                    patch.object(mvp, 'sdk_tools') as sdk, patch.object(mvp, 'freeze_materials') as materials, \
                    patch.object(mvp, 'build') as build, patch.object(mvp, 'check') as check:
                with self.assertRaises(ValueError) as caught:
                    mvp.main(['package', '--release', '--windows-sdk', str(root / 'sdk'),
                              '--offline-materials', str(root / 'materials'), '--msvc-linker', str(root / 'link.exe')])
            self.assertIs(caught.exception, failure)
            checkout.assert_called_once_with(root)
            sdk.assert_not_called()
            materials.assert_not_called()
            build.assert_not_called()
            check.assert_not_called()
            self.license_collector.assert_not_called()

    def test_package_preflight_uses_the_same_original_license_owner_as_final_packaging(self):
        self.assertIs(self.original_license_collector, mvp.package_windows.__globals__['dependency_licenses'])

    def test_package_still_qualifies_the_checkout_tools_and_materials_before_build(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sources = {'packaging/windows/host-contract.json': b'contract',
                       'packaging/windows/offline-materials.json': b'locked materials',
                       'crates/weftext-app/Cargo.toml': b'[package]\nversion = "0.7.0"\n'}
            artifact = SimpleNamespace(source_root=root / 'frozen', source=sources,
                                       metadata={'packages': [{'id': 'selected-build-participant'}]},
                                       built_package_ids=frozenset({'selected-build-participant'}))
            contract, tools = {'fixture': 'contract'}, {'fixture': 'SDK tools'}
            materials = {'lockSha256': mvp.sha256(sources['packaging/windows/offline-materials.json'])}
            events = []
            sdk, linker, originals = root / 'sdk', root / 'link.exe', root / 'materials'
            with patch.object(mvp, 'ROOT', root), patch.object(mvp, 'os', SimpleNamespace(name='nt')), \
                    patch.object(mvp, 'require_checkout', side_effect=lambda _: events.append('checkout')) as checkout, \
                    patch.object(mvp, 'read_checked', side_effect=lambda path, _: sources[path.relative_to(root).as_posix()]), \
                    patch.object(mvp, 'load_contract', return_value=contract), \
                    patch.object(mvp, 'sdk_tools', side_effect=lambda _: events.append('sdk') or tools), \
                    patch.object(mvp, 'freeze_materials', side_effect=lambda *_: events.append('materials') or materials), \
                    patch.object(mvp, 'build', side_effect=lambda *_, **__: events.append('build') or artifact) as build, \
                    patch.object(mvp, 'dependency_licenses', side_effect=lambda *_: events.append('license-preflight')) as licenses, \
                    patch.object(mvp, 'check', side_effect=lambda *_: events.append('all-checks')) as check, \
                    patch.object(mvp, 'package_windows', side_effect=lambda *_, **__: events.append('package')) as package:
                result = mvp.main(['package', '--release', '--windows-sdk', str(sdk),
                                   '--offline-materials', str(originals), '--msvc-linker', str(linker)])
            self.assertIs(result, artifact)
            self.assertEqual(events, ['checkout', 'sdk', 'materials', 'build', 'license-preflight', 'all-checks', 'package'])
            checkout.assert_called_once_with(root)
            build.assert_called_once_with(True, package_input=True, linker=linker, windows_sdk=sdk)
            licenses.assert_called_once_with(artifact.source_root, artifact.metadata, artifact.built_package_ids)
            check.assert_called_once_with(artifact, GROUPS)
            package.assert_called_once_with(artifact.source_root, artifact, '0.7.0', contract, materials, tools, output_root=root)

    def test_package_missing_license_keeps_the_original_error_and_stops_all_checks_and_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sources = {'packaging/windows/host-contract.json': b'contract',
                       'crates/weftext-app/Cargo.toml': b'[package]\nversion = "0.7.0"\n'}
            artifact = SimpleNamespace(source_root=root / 'frozen',
                                       metadata={'packages': [{'id': 'selected-build-participant'}]},
                                       built_package_ids=frozenset({'selected-build-participant'}))
            failure = ValueError('Missing original license text: cargo selected-build-participant 0.7.0')
            events = []

            def missing(*_):
                events.append('license-preflight')
                raise failure

            with patch.object(mvp, 'ROOT', root), patch.object(mvp, 'os', SimpleNamespace(name='nt')), \
                    patch.object(mvp, 'require_checkout'), \
                    patch.object(mvp, 'read_checked', side_effect=lambda path, _: sources[path.relative_to(root).as_posix()]), \
                    patch.object(mvp, 'load_contract', return_value={}), patch.object(mvp, 'sdk_tools'), \
                    patch.object(mvp, 'freeze_materials'), \
                    patch.object(mvp, 'build', side_effect=lambda *_, **__: events.append('build') or artifact) as build, \
                    patch.object(mvp, 'dependency_licenses', side_effect=missing) as licenses, \
                    patch.object(mvp, 'check') as check, patch.object(mvp, 'package_windows') as package:
                with self.assertRaises(ValueError) as caught:
                    mvp.main(['package', '--release', '--windows-sdk', str(root / 'sdk'),
                              '--offline-materials', str(root / 'materials'), '--msvc-linker', str(root / 'link.exe')])
            self.assertIs(caught.exception, failure)
            self.assertEqual(events, ['build', 'license-preflight'])
            self.assertEqual(build.call_count, 1)
            licenses.assert_called_once_with(artifact.source_root, artifact.metadata, artifact.built_package_ids)
            check.assert_not_called()
            package.assert_not_called()


class FrozenRouting(unittest.TestCase):
    permit_test = ('reminder_device_state::integration_tests::capacity::'
                   'reminder_64_member_permit_checks_within_its_real_monotonic_lifetime')
    rust_commands = [
        ['cargo', 'clippy', '--locked', '--workspace', '--all-targets',
         '--message-format=json-render-diagnostics', '--', '-D', 'warnings'],
        ['cargo', 'test', '--locked', '--workspace', '--exclude', 'weftext-app',
         '--message-format=json-render-diagnostics', '--', '--exact', '--skip', permit_test],
        ['cargo', 'test', '--locked', '-p', 'weftext-core', '--lib',
         '--message-format=json-render-diagnostics', '--', permit_test, '--exact', '--test-threads=1'],
        ['cargo', 'test', '--locked', '-p', 'weftext-app',
         '--message-format=json-render-diagnostics', '--', '--test-threads=1'],
    ]

    def test_full_schedule_preserves_original_order_workers_roles_and_all_40_http(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            calls, cargo_calls, environments, timeline = [], [], [], []

            def run(*args, **kwargs):
                calls.append(tuple(str(arg) for arg in args))
                environments.append(kwargs['env'])
                timeline.append(('run', str(args[1]) if len(args) > 1 else ''))

            def cargo(args):
                cargo_calls.append(args)
                timeline.append(('cargo', len(cargo_calls)))

            @contextmanager
            def cache(root, target, environment):
                self.assertEqual(root, fixture.artifacts.source_root)
                self.assertEqual(target, fixture.artifacts.target_directory)
                self.assertEqual(environment, environments[-1])
                yield SimpleNamespace(run=cargo)

            with patch.object(mvp.NpmTool, 'discover', return_value=NpmFixture()), \
                    patch.object(mvp, 'run', side_effect=run), patch.object(mvp, 'CargoCache', side_effect=cache), \
                    patch.object(mvp, 'verify_qualified', side_effect=lambda *_: timeline.append(('verify', 0))) as verify, \
                    redirect_stdout(io.StringIO()) as output:
                mvp.check(fixture.artifacts)
            names = [call[1] if len(call) > 1 else '' for call in calls]
            self.assertLess(names.index('scripts/check.py'), names.index('scripts/test_cargo_cache.py'))
            self.assertLess(names.index('scripts/test_cli_contract.py'), names.index('scripts/test_mvp.py'))
            self.assertLess(next(i for i, call in enumerate(calls) if call[:2] == ('cargo', 'fmt')),
                            names.index('scripts/test_cli_contract.py'))
            self.assertEqual(cargo_calls, self.rust_commands)
            self.assertLess(timeline.index(('run', 'scripts/test_cli_contract.py')), timeline.index(('cargo', 1)))
            self.assertLess(timeline.index(('cargo', 4)), timeline.index(('run', 'scripts/test_mvp.py')))
            cli_index = timeline.index(('run', 'scripts/test_cli_contract.py'))
            self.assertEqual(timeline[cli_index - 1:cli_index + 10], [
                ('verify', 0), ('run', 'scripts/test_cli_contract.py'), ('verify', 0),
                ('verify', 0), ('cargo', 1), ('cargo', 2), ('cargo', 3), ('cargo', 4),
                ('verify', 0), ('verify', 0), ('run', 'scripts/test_mvp.py'),
            ])
            http = [call for call in calls if len(call) == 3 and call[2] == str(fixture.artifacts.paths['cli'])]
            self.assertEqual(len(http), 40)
            self.assertEqual([call[1] for call in http], ['scripts/' + name for name in mvp.CHECK_SCRIPTS])
            cli = calls[names.index('scripts/test_cli_contract.py')]
            self.assertEqual(cli[cli.index('--gui-executable') + 1], str(fixture.artifacts.paths['desktop']))
            self.assertEqual(cli[cli.index('--cli-executable') + 1], str(fixture.artifacts.paths['cli']))
            self.assertTrue(all(env['WEFTEXT_TEST_EXECUTABLE'] == str(fixture.artifacts.paths['cli']) for env in environments))
            self.assertEqual([call for call in calls if call[:2] == ('selected-node', 'real-npm-cli.js')],
                             [('selected-node', 'real-npm-cli.js', '--prefix', 'app', 'test')])
            self.assertEqual(verify.call_count, 15)  # before/after seven original phases, then final
            self.assertIn('Full product checks passed', output.getvalue())

    def test_each_rust_failure_stops_later_tests_and_http_after_cli_and_rechecks_inputs(self):
        for failed in range(len(self.rust_commands)):
            with self.subTest(failed=failed), tempfile.TemporaryDirectory() as directory:
                fixture = Fixture(directory)
                calls, cargo_calls, timeline = [], [], []
                primary = RuntimeError('original Rust check failure')

                def run(*args, **_):
                    calls.append(tuple(str(arg) for arg in args))
                    timeline.append('run:' + str(args[1]))

                def cargo(args):
                    cargo_calls.append(args)
                    timeline.append('cargo')
                    if len(cargo_calls) == failed + 1:
                        raise primary

                @contextmanager
                def cache(*_):
                    yield SimpleNamespace(run=cargo)

                with patch.object(mvp.NpmTool, 'discover', return_value=NpmFixture()), \
                        patch.object(mvp, 'run', side_effect=run), \
                        patch.object(mvp, 'CargoCache', side_effect=cache), \
                        patch.object(mvp, 'verify_qualified', side_effect=lambda *_: timeline.append('verify')) as verify, \
                        redirect_stdout(io.StringIO()), \
                        self.assertRaises(RuntimeError) as caught:
                    mvp.check(fixture.artifacts, ['cli', 'rust', 'http'])
                self.assertIs(caught.exception, primary)
                self.assertEqual(cargo_calls, self.rust_commands[:failed + 1])
                self.assertEqual([call[1] for call in calls], ['fmt', 'scripts/test_cli_contract.py'])
                self.assertLess(timeline.index('run:scripts/test_cli_contract.py'), timeline.index('cargo'))
                self.assertEqual(timeline[-1], 'verify')
                self.assertEqual(verify.call_count, 6)

    def test_same_batch_selected_group_executes_again_without_pass_cache(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            with patch.object(mvp.NpmTool, 'discover', return_value=NpmFixture()), \
                    patch.object(mvp, 'run') as run, patch.object(mvp, 'verify_qualified') as verify, \
                    redirect_stdout(io.StringIO()) as output:
                mvp.check(fixture.artifacts, ['source'])
                mvp.check(fixture.artifacts, ['source'])
            self.assertEqual(run.call_count, 2)
            self.assertEqual(verify.call_count, 6)
            self.assertEqual(output.getvalue().count('Partial checks passed'), 2)
            self.assertNotIn('Full product', output.getvalue())

    def test_group_failure_stops_later_work_and_preserves_primary_if_postcheck_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Fixture(directory)
            primary = RuntimeError('original check failure')
            with patch.object(mvp.NpmTool, 'discover', return_value=NpmFixture()), \
                    patch.object(mvp, 'run', side_effect=primary) as run, \
                    patch.object(mvp, 'verify_qualified', side_effect=[None, ValueError('changed frozen inputs')]), \
                    self.assertRaises(RuntimeError) as caught:
                mvp.check(fixture.artifacts, ['source', 'frontend'])
            self.assertIs(caught.exception, primary)
            self.assertEqual(run.call_count, 1)
            self.assertEqual(len(primary.__notes__), 1)


if __name__ == '__main__':
    unittest.main()
