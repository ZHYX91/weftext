"""Pure CI association/evidence tests; no product, network or SDK execution."""
import json
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch
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
