"""Pure CI association/evidence tests; no product, network or SDK execution."""
from contextlib import ExitStack
import json
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import MagicMock, call, patch
import zipfile

import ci_windows_package as ci
from package_inputs import inventory


class CandidateAssociation(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='weftext-ci-unit-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.dist = self.root / 'dist'
        self.dist.mkdir()
        self.old = self.dist / 'old'
        self.old.mkdir()
        (self.old / 'PACKAGE.json').write_text('{}', encoding='utf-8')
        self.new = self.dist / 'new'
        (self.new / 'payload').mkdir(parents=True)
        self.identity = {'buildId': 'a' * 64, 'artifactCatalogSha256': 'b' * 64}
        self.blobs = {'weftext-unit.zip': b'fixture zip; never execute',
                      'weftext-unit.msix': b'fixture msix; never install'}
        for name, data in self.blobs.items():
            (self.new / name).write_bytes(data)
        ci.write_json(self.new / 'payload/BUILD.json', self.identity)
        self.receipt = {**self.identity, 'containers': inventory(self.blobs), 'signed': False,
            'commonPayload': inventory({'BUILD.json': (self.new / 'payload/BUILD.json').read_bytes()}),
            'installed': False, 'upgradeQualified': False, 'dataPreservationQualified': False}
        ci.write_json(self.new / 'PACKAGE.json', self.receipt)

    def rewrite(self):
        (self.new / 'PACKAGE.json').write_text(json.dumps(self.receipt), encoding='utf-8')

    def test_exact_new_package_matches_returned_batch(self):
        selected, receipt = ci.match_package(self.dist, ['old'], self.identity)
        self.assertEqual(selected, self.new)
        self.assertEqual(receipt, self.receipt)

    def test_missing_new_candidate_never_adopts_old(self):
        with self.assertRaisesRegex(ValueError, 'exactly one'):
            ci.match_package(self.dist, ['old', 'new'], self.identity)

    def test_multiple_new_candidates_are_ambiguous(self):
        (self.dist / 'other').mkdir()
        with self.assertRaisesRegex(ValueError, 'exactly one'):
            ci.match_package(self.dist, ['old'], self.identity)

    def test_receipt_cannot_associate_another_build(self):
        with self.assertRaisesRegex(ValueError, 'exact returned'):
            ci.match_package(self.dist, ['old'], {**self.identity, 'buildId': 'c' * 64})

    def test_incomplete_output_never_qualifies(self):
        (self.new / 'INCOMPLETE.json').write_bytes(b'{}')
        with self.assertRaisesRegex(ValueError, 'Incomplete'):
            ci.match_package(self.dist, ['old'], self.identity)

    def test_changed_container_fails_readback(self):
        (self.new / 'weftext-unit.zip').write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError, 'container changed'):
            ci.match_package(self.dist, ['old'], self.identity)

    def test_changed_build_json_fails_association(self):
        (self.new / 'payload/BUILD.json').write_text(json.dumps(
            {**self.identity, 'buildId': 'd' * 64}), encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'BUILD'):
            ci.match_package(self.dist, ['old'], self.identity)

    def test_build_with_same_identity_but_other_changed_fields_is_not_original(self):
        (self.new / 'payload/BUILD.json').write_text(json.dumps(
            {**self.identity, 'status': 'different claim'}), encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'BUILD original bytes changed'):
            ci.match_package(self.dist, ['old'], self.identity)

    def test_unexpected_container_paths_or_claims_fail(self):
        for row in ({**self.receipt['containers'][0], 'path': '../outside.zip'},
                    {**self.receipt['containers'][0], 'path': 'C:\\outside.zip'}):
            with self.subTest(row=row):
                self.receipt['containers'][0] = row
                self.rewrite()
                with self.assertRaisesRegex(ValueError, 'container names'):
                    ci.match_package(self.dist, ['old'], self.identity)
        self.receipt['containers'] = inventory(self.blobs)
        self.receipt['installed'] = True
        self.rewrite()
        with self.assertRaisesRegex(ValueError, 'unsigned review'):
            ci.match_package(self.dist, ['old'], self.identity)


class ReceiptChildRouting(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='weftext-ci-receipt-')
        self.addCleanup(self.temp.cleanup)
        folder = Path(self.temp.name)
        self.root, self.target, self.evidence = folder / 'checkout', folder / 'target', folder / 'evidence'
        self.root.mkdir()
        self.evidence.mkdir()
        self.source = self.target / 'weftext-package-inputs/fixed/source'
        self.source.mkdir(parents=True)
        source = {'packaging/windows/host-contract.json': b'{}',
            'packaging/windows/offline-materials.json': b'locked original materials',
            'crates/weftext-app/Cargo.toml': b'[package]\nversion = "0.7.0-rc.1"\n'}
        for name, data in source.items():
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        self.batch = self.target / 'weftext-build-artifacts/fixed'
        self.batch.mkdir(parents=True)
        self.receipt = self.batch / 'build-input-receipt.json'
        self.receipt.write_bytes(b'original mocked build receipt; never execute')
        self.build_environment = {'CARGO_TARGET_DIR': str(self.target),
            'CARGO_BUILD_TARGET': 'x86_64-pc-windows-msvc',
            'CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER': str(folder / 'installed/link.exe'),
            'WEFTEXT_WINDOWS_SDK_BIN': str(folder / 'installed/sdk')}
        self.artifacts = ci.mvp.QualifiedArtifacts(self.source, {}, {}, source, {},
            {'target_directory': str(self.target), 'packages': []}, frozenset(), self.target, 'release',
            build_environment=self.build_environment, receipt_path=self.receipt,
            catalog=b'{"buildId":"actual-canonical-build"}')
        ci.write_json(self.evidence / 'provenance.json', {'targetRoot': str(self.target), 'distBefore': []})
        self.preparation = self.evidence / 'preparation.json'
        ci.write_json(self.preparation, {'status': 'preparedInputsOnly',
            'sdk': {'directory': self.build_environment['WEFTEXT_WINDOWS_SDK_BIN']},
            'materialsDirectory': str(folder / 'materials'),
            'linker': {'path': self.build_environment['CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER']}})
        self.timeline, self.invocations = [], []

    def child(self, exit_code=0, waits=None):
        process = MagicMock()
        process.__enter__.return_value = process
        process.__exit__.return_value = False
        process.pid = 27182
        process.wait.side_effect = waits
        process.wait.return_value = exit_code
        def launch(command, **kwargs):
            self.assertTrue((self.evidence / 'receipt-cli-intent.json').is_file())
            self.assertFalse(kwargs['stdout'].closed)
            self.assertFalse(kwargs['stderr'].closed)
            kwargs['stdout'].write(b'original stdout\x00\xff\r\n')
            kwargs['stderr'].write(b'original stderr\x80\r\n')
            self.invocations.append((command, kwargs))
            self.timeline.append('receipt-child')
            return process
        return process, launch

    def canonical(self, launch):
        stack = ExitStack()
        self.addCleanup(stack.close)
        stack.enter_context(patch.object(ci, 'ROOT', self.root))
        stack.enter_context(patch.object(ci.mvp, 'ROOT', self.root))
        stack.enter_context(patch.object(ci.mvp, 'require_checkout'))
        stack.enter_context(patch.object(ci.mvp, 'sdk_tools', return_value={'qualified': 'fixture'}))
        stack.enter_context(patch.object(ci.mvp, 'freeze_materials', return_value={
            'lockSha256': ci.sha256(self.artifacts.source['packaging/windows/offline-materials.json'])}))
        stack.enter_context(patch.object(ci.mvp, 'load_contract', return_value={'identity': 'fixture'}))
        def build(*args, **kwargs):
            self.timeline.append('canonical-build')
            return self.artifacts
        def check(artifacts, groups):
            self.assertIs(artifacts, self.artifacts)
            self.assertEqual(groups, ci.mvp.GROUPS)
            record = json.loads((self.evidence / 'receipt-cli-result.json').read_text(encoding='utf-8'))
            self.assertEqual(record['status'], 'passedPartialCliOnly')
            self.timeline.append('original-full-checks')
        def packaged(source, artifacts, *args, **kwargs):
            self.assertEqual(source, self.source)
            self.assertIs(artifacts, self.artifacts)
            self.timeline.append('prepare-containers')
            kwargs['before_finalize']()
            self.timeline.append('finalize-package')
        def licenses(source, metadata, built_package_ids):
            self.assertEqual(source, self.source)
            self.assertIs(metadata, self.artifacts.metadata)
            self.assertIs(built_package_ids, self.artifacts.built_package_ids)
            self.timeline.append('license-preflight')
            return {'original': b'license fixture'}, ['package fixture']
        build_mock = stack.enter_context(patch.object(ci.mvp, 'build', side_effect=build))
        self.licenses = stack.enter_context(patch.object(ci.mvp, 'dependency_licenses', side_effect=licenses))
        check_mock = stack.enter_context(patch.object(ci.mvp, 'check', side_effect=check))
        package_mock = stack.enter_context(patch.object(ci.mvp, 'package_windows', side_effect=packaged))
        stack.enter_context(patch.object(ci, 'match_package', return_value=(self.root / 'dist/new', {'containers': []})))
        stack.enter_context(patch.object(ci.subprocess, 'Popen', side_effect=launch))
        return build_mock, check_mock, package_mock

    def test_exact_package_batch_is_checked_in_a_child_before_original_full_continuation(self):
        process, launch = self.child()
        build, check, package = self.canonical(launch)
        with patch.dict(os.environ, {'CARGO_TARGET_DIR': 'must be replaced by actual build target',
                'PATH': 'the selected Node and npm path', 'TEMP': 'same attempt temp', 'TMP': 'same attempt temp'}):
            ci.package(self.evidence, self.preparation)
            self.assertEqual(os.environ['CARGO_TARGET_DIR'], 'must be replaced by actual build target')
        self.assertEqual(self.timeline, ['canonical-build', 'receipt-child', 'license-preflight',
            'prepare-containers', 'original-full-checks', 'finalize-package'])
        build.assert_called_once()
        self.licenses.assert_called_once_with(self.source, self.artifacts.metadata, self.artifacts.built_package_ids)
        check.assert_called_once_with(self.artifacts, ci.mvp.GROUPS)
        package.assert_called_once()
        self.assertIs(ci.mvp.build, build)
        self.assertIs(ci.mvp.check, check)
        command, kwargs = self.invocations[0]
        self.assertEqual(command, [ci.sys.executable, str(self.source / 'scripts/mvp.py'),
            'check', '--release', '--group', 'cli', '--artifacts', str(self.receipt)])
        self.assertTrue(Path(command[0]).is_absolute())
        self.assertEqual(kwargs['cwd'], self.source)
        self.assertEqual(kwargs['stdin'], ci.subprocess.DEVNULL)
        self.assertNotIn('timeout', kwargs)
        self.assertEqual({key: kwargs['env'][key] for key in self.build_environment}, self.build_environment)
        self.assertEqual(kwargs['env']['PATH'], 'the selected Node and npm path')
        self.assertEqual(kwargs['env']['TEMP'], 'same attempt temp')
        self.assertEqual(kwargs['env']['TMP'], 'same attempt temp')
        self.assertTrue(kwargs['stdout'].closed)
        self.assertTrue(kwargs['stderr'].closed)
        self.assertEqual((self.evidence / 'receipt-cli.stdout.bin').read_bytes(), b'original stdout\x00\xff\r\n')
        self.assertEqual((self.evidence / 'receipt-cli.stderr.bin').read_bytes(), b'original stderr\x80\r\n')
        record = json.loads((self.evidence / 'receipt-cli-result.json').read_text(encoding='utf-8'))
        self.assertEqual(record['exitCode'], 0)
        self.assertEqual(record['receipt'], str(self.receipt))
        self.assertEqual(record['sourceRoot'], str(self.source))
        self.assertEqual(record['pid'], process.pid)
        self.assertTrue((self.evidence / 'package-success.json').is_file())
        self.assertFalse((self.source / '.git').exists())

    def test_nonzero_child_retains_exit_and_streams_and_stops_original_full_checks(self):
        _, launch = self.child(exit_code=23)
        build, check, package = self.canonical(launch)
        with self.assertRaises(ci.subprocess.CalledProcessError) as raised:
            ci.package(self.evidence, self.preparation)
        self.assertEqual(raised.exception.returncode, 23)
        self.assertEqual(self.timeline, ['canonical-build', 'receipt-child'])
        self.assertIs(ci.mvp.build, build)
        check.assert_not_called()
        package.assert_not_called()
        self.licenses.assert_not_called()
        self.assertTrue((self.evidence / 'package-batch.json').is_file())
        self.assertTrue((self.evidence / 'cargo-metadata.json').is_file())
        self.assertFalse((self.evidence / 'package-success.json').exists())
        record = json.loads((self.evidence / 'receipt-cli-result.json').read_text(encoding='utf-8'))
        self.assertEqual(record['exitCode'], 23)
        self.assertEqual(record['status'], 'failedOriginalsRetained')
        self.assertEqual((self.evidence / 'receipt-cli.stderr.bin').read_bytes(), b'original stderr\x80\r\n')

    def test_missing_original_license_after_child_success_stops_full_checks_and_package(self):
        _, launch = self.child()
        build, check, package = self.canonical(launch)
        original = ValueError('Missing original license text: cargo cel 0.15.0')
        def missing(*args):
            self.timeline.append('license-preflight')
            raise original
        self.licenses.side_effect = missing
        with self.assertRaises(ValueError) as raised:
            ci.package(self.evidence, self.preparation)
        self.assertIs(raised.exception, original)
        self.assertEqual(self.timeline, ['canonical-build', 'receipt-child', 'license-preflight'])
        self.licenses.assert_called_once_with(self.source, self.artifacts.metadata, self.artifacts.built_package_ids)
        self.assertIs(ci.mvp.build, build)
        check.assert_not_called()
        package.assert_not_called()
        self.assertTrue((self.evidence / 'package-batch.json').is_file())
        self.assertFalse((self.evidence / 'package-success.json').exists())
        child = json.loads((self.evidence / 'receipt-cli-result.json').read_text(encoding='utf-8'))
        self.assertEqual(child['status'], 'passedPartialCliOnly')

    def test_spawn_failure_preserves_original_error_empty_streams_and_no_false_exit(self):
        original = FileNotFoundError('selected interpreter disappeared')
        build, check, package = self.canonical(MagicMock(side_effect=original))
        with self.assertRaises(FileNotFoundError) as raised:
            ci.package(self.evidence, self.preparation)
        self.assertIs(raised.exception, original)
        self.assertIs(ci.mvp.build, build)
        check.assert_not_called()
        package.assert_not_called()
        self.licenses.assert_not_called()
        record = json.loads((self.evidence / 'receipt-cli-result.json').read_text(encoding='utf-8'))
        self.assertIsNone(record['exitCode'])
        self.assertNotIn('pid', record)
        self.assertEqual((self.evidence / 'receipt-cli.stdout.bin').read_bytes(), b'')
        self.assertEqual((self.evidence / 'receipt-cli.stderr.bin').read_bytes(), b'')
        self.assertFalse((self.evidence / 'package-success.json').exists())

    def test_interruption_waits_for_exact_child_exit_then_preserves_interrupt_without_full_continuation(self):
        original = KeyboardInterrupt('first original interruption')
        process, launch = self.child(waits=[original, 0])
        build, check, package = self.canonical(launch)
        with patch.object(ci.signal, 'signal', return_value='previous handler') as handler:
            with self.assertRaises(KeyboardInterrupt) as raised:
                ci.package(self.evidence, self.preparation)
        self.assertIs(raised.exception, original)
        self.assertEqual(process.wait.call_count, 2)
        self.assertEqual(handler.call_args_list, [
            call(ci.signal.SIGINT, ci.signal.SIG_IGN), call(ci.signal.SIGINT, 'previous handler')])
        self.assertIs(ci.mvp.build, build)
        check.assert_not_called()
        package.assert_not_called()
        self.licenses.assert_not_called()
        record = json.loads((self.evidence / 'receipt-cli-result.json').read_text(encoding='utf-8'))
        self.assertEqual(record['exitCode'], 0)
        self.assertEqual(record['status'], 'failedOriginalsRetained')
        self.assertFalse((self.evidence / 'package-success.json').exists())

    def test_result_write_failure_does_not_replace_nonzero_child_error(self):
        _, launch = self.child(exit_code=17)
        original_write = ci.write_json
        def write(path, value):
            if path.name == 'receipt-cli-result.json':
                raise OSError('result write failed')
            return original_write(path, value)
        with patch.object(ci.subprocess, 'Popen', side_effect=launch), patch.object(ci, 'write_json', side_effect=write):
            with self.assertRaises(ci.subprocess.CalledProcessError) as raised:
                ci.verify_receipt(self.evidence, self.artifacts)
        self.assertEqual(raised.exception.returncode, 17)
        self.assertTrue(any('result write failed' in note for note in raised.exception.__notes__))
        self.assertTrue((self.evidence / 'receipt-cli-intent.json').is_file())
        self.assertTrue((self.evidence / 'receipt-cli.stdout.bin').is_file())

    def test_handler_restore_failure_does_not_replace_the_original_interrupt(self):
        original = SystemExit('original interruption')
        process, launch = self.child(waits=[original, 0])
        with patch.object(ci.subprocess, 'Popen', side_effect=launch), \
                patch.object(ci.signal, 'signal', side_effect=['previous handler', OSError('restore failed')]):
            with self.assertRaises(SystemExit) as raised:
                ci.verify_receipt(self.evidence, self.artifacts)
        self.assertIs(raised.exception, original)
        self.assertEqual(process.wait.call_count, 2)
        self.assertTrue(any('restore failed' in note for note in original.__notes__))
        record = json.loads((self.evidence / 'receipt-cli-result.json').read_text(encoding='utf-8'))
        self.assertEqual(record['exitCode'], 0)
        self.assertEqual(record['status'], 'failedOriginalsRetained')

    def test_initialize_records_attempt_without_any_build_or_check(self):
        runner = self.evidence / 'runner'
        runner.mkdir()
        outputs = self.evidence / 'outputs'
        environment = {'RUNNER_TEMP': str(runner), 'GITHUB_RUN_ID': '123', 'GITHUB_RUN_ATTEMPT': '2',
            'GITHUB_OUTPUT': str(outputs), 'GITHUB_REPOSITORY': 'owner/weftext', 'GITHUB_SHA': 'checkout-sha'}
        with patch.object(ci, 'ROOT', self.root), patch.dict(os.environ, environment), \
                patch.object(ci.mvp, 'metadata', return_value={'target_directory': str(self.target)}), \
                patch.object(ci.subprocess, 'check_output', side_effect=['commit', 'tree']), \
                patch.object(ci.mvp, 'main', side_effect=AssertionError('No product invocation')), \
                patch.object(ci.mvp, 'build', side_effect=AssertionError('No build')), \
                patch.object(ci.mvp, 'check', side_effect=AssertionError('No check')):
            ci.initialize()
        evidence = runner / 'weftext-ci-123-2'
        record = json.loads((evidence / 'provenance.json').read_text(encoding='utf-8'))
        self.assertEqual(record['targetRoot'], str(self.target))
        self.assertEqual(record['checkoutCommit'], 'commit')
        self.assertEqual(record['checkoutTree'], 'tree')
        self.assertEqual(set(path.name for path in evidence.iterdir()), {'provenance.json', 'temp'})
        self.assertNotIn('receipt=', outputs.read_text(encoding='utf-8'))


class EvidencePreservation(unittest.TestCase):
    def test_archive_retains_original_bytes_and_only_selected_controls(self):
        with tempfile.TemporaryDirectory(prefix='weftext-ci-unit-') as directory:
            evidence = Path(directory)
            root = evidence / 'check'
            selected = root / 'scenario/server-control-owned'
            selected.mkdir(parents=True)
            original = b'original native bytes\x00\xff\r\n'
            (selected / 'owner-stop.stdout.bin').write_bytes(original)
            cli = root / 'cli-results'
            cli.mkdir()
            (cli / 'results.json').write_bytes(b'{"original":true}')
            (root / 'irrelevant.txt').write_text('not a control', encoding='utf-8')
            ci.archive_controls(evidence, [('new-check', root)])
            with zipfile.ZipFile(evidence / 'process-controls.zip') as archive:
                self.assertEqual(set(archive.namelist()), {
                    'new-check/scenario/server-control-owned/owner-stop.stdout.bin',
                    'new-check/cli-results/results.json'})
                self.assertEqual(archive.read('new-check/scenario/server-control-owned/owner-stop.stdout.bin'), original)

    def test_collect_excludes_cached_batches_and_keeps_partial_package_unqualified(self):
        with tempfile.TemporaryDirectory(prefix='weftext-ci-unit-') as directory:
            root = Path(directory)
            evidence, target = root / 'evidence', root / 'target'
            evidence.mkdir()
            (evidence / 'temp').mkdir()
            batches = target / 'weftext-build-artifacts'
            (batches / 'cached').mkdir(parents=True)
            (batches / 'new').mkdir()
            (root / 'dist/old').mkdir(parents=True)
            (root / 'dist/partial/sdk-logs').mkdir(parents=True)
            (root / 'dist/partial/INCOMPLETE.json').write_bytes(b'{"failed":true}')
            ci.write_json(evidence / 'provenance.json', {'targetRoot': str(target),
                'checkRootsBefore': [], 'batchesBefore': ['cached'], 'distBefore': ['old'],
                'fixedControlRootsBefore': {}})
            outputs = root / 'outputs'
            with patch.object(ci, 'ROOT', root), patch.dict(os.environ, {'GITHUB_OUTPUT': str(outputs)}):
                ci.collect(evidence)
            listed = outputs.read_text(encoding='utf-8').splitlines()
            self.assertIn(str(batches / 'new'), listed)
            self.assertNotIn(str(batches / 'cached'), listed)
            self.assertIn(str(root / 'dist/partial'), listed)
            self.assertNotIn(str(root / 'dist/old'), listed)
            self.assertIn(str(evidence / 'receipt-cli.stdout.bin'), listed)
            self.assertIn(str(evidence / 'receipt-cli.stderr.bin'), listed)
            self.assertFalse((evidence / 'package-success.json').exists())

    def test_failed_package_restores_actual_build_without_replacing_checks(self):
        with tempfile.TemporaryDirectory(prefix='weftext-ci-unit-') as directory:
            evidence = Path(directory)
            ci.write_json(evidence / 'provenance.json', {'targetRoot': str(evidence / 'target'), 'distBefore': []})
            prep = evidence / 'preparation.json'
            ci.write_json(prep, {'status': 'preparedInputsOnly', 'sdk': {'directory': 'sdk'},
                'materialsDirectory': 'materials', 'linker': {'path': 'link'}})
            original_build, original_check = ci.mvp.build, ci.mvp.check
            with patch.object(ci.mvp, 'main', side_effect=RuntimeError('original full gate failed')) as main:
                with self.assertRaisesRegex(RuntimeError, 'original full gate failed'):
                    ci.package(evidence, prep)
            self.assertIs(ci.mvp.build, original_build)
            self.assertIs(ci.mvp.check, original_check)
            self.assertEqual(main.call_args.args[0], ['package', '--release', '--windows-sdk', 'sdk',
                '--offline-materials', 'materials', '--msvc-linker', 'link'])
            self.assertFalse((evidence / 'package-success.json').exists())

    def test_collection_failure_keeps_selected_roots_before_the_archive_error(self):
        with tempfile.TemporaryDirectory(prefix='weftext-ci-unit-') as directory:
            evidence = Path(directory)
            (evidence / 'temp').mkdir()
            ci.write_json(evidence / 'provenance.json', {'targetRoot': str(evidence / 'target'),
                'checkRootsBefore': [], 'batchesBefore': [], 'distBefore': [],
                'fixedControlRootsBefore': {}})
            with patch.object(ci, 'archive_controls', side_effect=OSError('original archive error')):
                with self.assertRaisesRegex(OSError, 'original archive error'):
                    ci.collect(evidence)
            selected = json.loads((evidence / 'process-roots.json').read_text(encoding='utf-8'))
            self.assertEqual(selected['selectedRoots'], [{'label': 'http-temp', 'path': str(evidence / 'temp')}])

    def test_windows_long_io_path_does_not_change_logical_spelling(self):
        if os.name != 'nt':
            self.skipTest('actual Windows extended-path filesystem')
        with tempfile.TemporaryDirectory(prefix='weftext-ci-unit-', dir=ci._io_path(tempfile.gettempdir())) as directory:
            evidence = Path(directory)
            root = evidence / ('frozen-' * 20) / ('source-' * 15)
            control = root / 'server-control-owned'
            ci._io_path(control).mkdir(parents=True)
            ci._io_path(control / 'owner.stdout.bin').write_bytes(b'long original')
            ci.archive_controls(evidence, [('frozen-check', root)])
            with zipfile.ZipFile(evidence / 'process-controls.zip') as archive:
                self.assertEqual(archive.namelist(), ['frozen-check/server-control-owned/owner.stdout.bin'])
                self.assertEqual(archive.read(archive.namelist()[0]), b'long original')


if __name__ == '__main__':
    unittest.main()
