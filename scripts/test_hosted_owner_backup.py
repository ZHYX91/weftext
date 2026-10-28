"""Actual independently authorized recovery-backup HTTP on owned fixtures.

ZIP integrity is not automatic cross-device restoration or Office acceptance.
"""
import uuid
import base64
import hashlib
import io
import json
from pathlib import Path
import sys
import unittest
import zipfile

from test_hosted import Hosted, uid, digest


class HostedOwnerBackup(Hosted):
    def enable_backup(self):
        self.spool = self.folder / 'backup-output'
        self.spool.mkdir()
        self.server.close()
        self.config['ownerBackupDirectory'] = str(self.spool)
        self.start()
        self.policy(lambda users: users['alice'].update(
            ownerBackup={'read': True, 'export': True, 'download': True}))

    def prepare_backup(self, request=None, status=200):
        request = request or uid()
        return self.call('owner-backup/prepare', {'requestId': request}, status=status)

    def commit_backup(self, prepared, status=200):
        return self.call('owner-backup/commit', {
            'requestId': prepared['requestId'], 'planId': prepared['planId']}, status=status)

    def result(self, request, actor='alice'):
        return self.call('owner-backup/status', {'requestId': request}, actor)

    def download_backup(self, committed):
        artifact = committed['artifact']
        parts = []
        offset = 0
        while True:
            value = self.call('owner-backup/download', {
                'requestId': committed['requestId'], 'digest': artifact['digest'], 'offset': offset})
            self.assertEqual(value['offset'], offset)
            part = base64.b64decode(value['base64'], validate=True)
            self.assertLessEqual(len(part), 65536)
            self.assertEqual(value['nextOffset'], offset + len(part))
            parts.append(part)
            if value['done']:
                break
            self.assertGreater(value['nextOffset'], offset)
            offset = value['nextOffset']
        data = b''.join(parts)
        self.assertEqual(len(data), artifact['length'])
        self.assertEqual(hashlib.sha256(data).hexdigest(), artifact['digest'])
        return data

    def test_independent_operator_and_read_export_download_grants(self):
        self.assertFalse(self.call('owner-backup')['enabled'])
        self.prepare_backup(status=404)
        self.policy(lambda users: users['alice'].update(
            ownerBackup={'read': True, 'export': True, 'download': True}))
        self.assertFalse(self.call('owner-backup')['enabled'])
        self.prepare_backup(status=404)
        self.enable_backup()
        self.assertTrue(self.call('owner-backup')['enabled'])
        self.assertFalse(self.call('owner-backup', actor='bob')['enabled'])
        self.call('owner-backup/prepare', {'requestId': uid()}, 'bob', 404)
        self.policy(lambda users: users['reader'].update(ownerBackup={'read': True}))
        self.assertTrue(self.call('owner-backup', actor='reader')['enabled'])
        self.call('owner-backup/prepare', {'requestId': uid()}, 'reader', 404)
        self.call('owner-backup/prepare', {'requestId': uid(), 'root': str(self.root)}, status=422)
        self.prepare_backup('not-an-operation', status=400)
        self.assertEqual(list(self.spool.iterdir()), [])

    def test_exact_complete_materials_actor_drafts_fences_raw_and_empty_directories(self):
        self.enable_backup()
        alice = self.open()
        bob = self.open(actor='bob')
        self.call('draft', {'inputId': str(uuid.uuid4()), 'baseId': alice['baseId'], 'inputRevision': 1, 'sessionId': alice['sessionId'], 'source': alice['source'] + '\nALICE_BACKUP_DRAFT'})
        self.call('draft', {'inputId': str(uuid.uuid4()), 'baseId': bob['baseId'], 'inputRevision': 1, 'sessionId': bob['sessionId'], 'source': bob['source'] + '\nBOB_PRIVATE_BACKUP_DRAFT'}, 'bob')
        fences = [uid(), uid()]
        self.call('operation/settle', {'operationId': fences[0], 'resource': 'Shared'})
        self.call('operation/settle', {'operationId': fences[1], 'resource': 'Shared'}, 'bob')
        (self.root / 'raw.bin').write_bytes(b'\x00RAW_BACKUP\xff')
        long_name = '长' * 100 + '.txt'
        long_file = self.root / long_name
        if sys.platform == 'win32':
            # Preserve the real NTFS name even under a long owned fixture root.
            long_file = Path('\\\\?\\' + str(long_file.resolve()))
        long_file.write_bytes(b'VALID_NTFS_LONG_NAME')
        (self.root / 'empty-raw-folder').mkdir()
        # Whole-library owner authority is independent of all ordinary grants.
        self.policy(lambda users: users['alice'].update(grants={}))
        prepared = self.prepare_backup()
        self.assertEqual(prepared['status'], 'prepared')
        self.assertTrue(prepared['commitAllowed'])
        self.assertFalse(prepared['automaticRestore'])
        committed = self.commit_backup(prepared)
        self.assertEqual(committed['status'], 'committed')
        self.assertFalse(committed['deliveryUnavailable'])
        data = self.download_backup(committed)
        (self.folder / 'complete-backup.zip').write_bytes(data)
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            names = archive.namelist()
            self.assertIn('library/empty-raw-folder/', names)
            self.assertEqual(archive.read('library/raw.bin'), b'\x00RAW_BACKUP\xff')
            self.assertEqual(archive.read('library/' + long_name), b'VALID_NTFS_LONG_NAME')
            self.assertNotIn('manifest.json', names)
            manifest = json.loads(archive.read('backup.json'))
            self.assertEqual(manifest['kind'], 'weftext.local-backup.v1')
            self.assertEqual(manifest['version'], 1)
            for entry in manifest['files']:
                raw = archive.read(entry['path'].replace('\\', '/'))
                self.assertEqual(len(raw), entry['bytes'])
                self.assertEqual(hashlib.sha256(raw).hexdigest(), entry['sha256'])
            all_bytes = b'\n'.join(archive.read(name) for name in names if not name.endswith('/'))
            for marker in [b'ALICE_BACKUP_DRAFT', b'BOB_PRIVATE_BACKUP_DRAFT', b'HIDDEN_PAYLOAD_987']:
                self.assertIn(marker, all_bytes)
            for operation in fences:
                self.assertTrue(any(name.endswith('/' + operation + '.json') for name in names))
            self.assertTrue(any(name.startswith('private/actors/') and '/drafts/' in name for name in names))
            self.assertFalse(any('hosted-policy' in name for name in names))
            for token in self.tokens.values():
                self.assertNotIn(token.encode(), all_bytes)
        self.assertEqual(self.result(prepared['requestId'])['artifact'], committed['artifact'])
        self.assertEqual(self.commit_backup(prepared)['artifact'], committed['artifact'])
        self.assertEqual(len(list(self.spool.iterdir())), 1)

    def test_late_other_actor_fence_invalidates_cut_before_any_backup_effect(self):
        self.enable_backup()
        prepared = self.prepare_backup()
        self.call('operation/settle', {'resource': 'Shared', 'operationId': uid()}, 'bob')
        failed = self.commit_backup(prepared, status=409)
        self.assertEqual(failed['error']['code'], 'ownerBackupUnavailable')
        self.assertEqual(list(self.spool.iterdir()), [])
        self.assertEqual(self.result(prepared['requestId'])['status'], 'prepared')
        settled = self.call('owner-backup/settle', {'requestId': prepared['requestId']})
        self.assertEqual(settled['status'], 'cancelled')
        self.assertEqual(self.prepare_backup(prepared['requestId'])['status'], 'cancelled')

    def test_prepared_restart_original_plan_and_committed_restart_never_recaptures(self):
        self.enable_backup()
        prepared = self.prepare_backup()
        self.server.close()
        self.start()
        again = self.prepare_backup(prepared['requestId'])
        self.assertEqual(again['planId'], prepared['planId'])
        committed = self.commit_backup(again)
        original_digest = committed['artifact']['digest']
        self.download_backup(committed)
        self.server.close()
        self.start()
        restored = self.result(prepared['requestId'])
        self.assertEqual(restored['status'], 'committed')
        self.assertTrue(restored['deliveryUnavailable'])
        self.assertIsNone(restored['artifact'])
        self.assertEqual(self.commit_backup(prepared)['status'], 'committed')
        self.call('owner-backup/download', {'requestId': prepared['requestId'], 'digest': original_digest, 'offset': 0}, status=404)
        self.assertEqual(len(list(self.spool.iterdir())), 1)
        unknown = uid()
        self.assertEqual(self.call('owner-backup/settle', {'requestId': unknown})['status'], 'cancelled')
        self.assertEqual(self.prepare_backup(unknown)['status'], 'cancelled')

    def test_each_chunk_actor_epoch_download_right_and_close_preserve_original(self):
        self.enable_backup()
        self.policy(lambda users: users['bob'].update(
            ownerBackup={'read': True, 'export': True, 'download': True}))
        (self.root / 'large-raw.bin').write_bytes(b'PRIVATE_RAW' * 18000)
        prepared = self.prepare_backup()
        committed = self.commit_backup(prepared)
        request = {'requestId': prepared['requestId'], 'digest': committed['artifact']['digest'], 'offset': 0}
        self.assertGreater(committed['artifact']['length'], 65536)
        self.assertEqual(self.result(prepared['requestId'], 'bob')['status'], 'notRegistered')
        self.call('owner-backup/download', request, 'bob', 404)
        self.assertEqual(self.call('owner-backup/settle', {'requestId': prepared['requestId']}, 'bob')['status'], 'cancelled')
        self.assertEqual(self.result(prepared['requestId'])['status'], 'committed')
        self.assertFalse(self.call('owner-backup/download', request)['done'])
        self.policy(lambda users: users['alice']['ownerBackup'].update(download=False))
        self.call('owner-backup/download', {**request, 'offset': 65536}, status=404)
        self.policy(lambda users: users['alice']['ownerBackup'].update(download=True))
        self.call('owner-backup/download', request, status=404)
        retained = self.result(prepared['requestId'])
        self.assertEqual(retained['status'], 'committed')
        self.assertTrue(retained['deliveryUnavailable'])
        self.call('owner-backup/close', {'requestId': prepared['requestId']})
        self.assertEqual(self.result(prepared['requestId'])['status'], 'committed')
        self.assertEqual(len(list(self.spool.iterdir())), 1)

    def test_close_live_artifact_never_reexecutes_or_rebuilds_original(self):
        self.enable_backup()
        prepared = self.prepare_backup()
        committed = self.commit_backup(prepared)
        self.assertIsNotNone(committed['artifact'])
        self.call('owner-backup/close', {'requestId': prepared['requestId']})
        for value in [self.result(prepared['requestId']), self.prepare_backup(prepared['requestId']), self.commit_backup(prepared)]:
            self.assertEqual(value['status'], 'committed')
            self.assertIsNone(value['artifact'])
            self.assertTrue(value['deliveryUnavailable'])
        self.call('owner-backup/download', {'requestId': prepared['requestId'], 'digest': committed['artifact']['digest'], 'offset': 0}, status=404)
        self.assertEqual(len(list(self.spool.iterdir())), 1)

    def test_prepared_epoch_revocation_and_config_rebind_do_not_retarget_original(self):
        self.enable_backup()
        prepared = self.prepare_backup()
        self.policy(lambda users: users['alice']['ownerBackup'].update(export=False, download=False))
        self.assertFalse(self.result(prepared['requestId'])['commitAllowed'])
        self.commit_backup(prepared, status=404)
        self.policy(lambda users: users['alice']['ownerBackup'].update(export=True, download=True))
        self.commit_backup(prepared, status=409)
        current = self.prepare_backup()
        destination = self.folder / 'new-backup-output'
        destination.mkdir()
        self.server.close()
        self.config['ownerBackupDirectory'] = str(destination)
        self.start()
        self.assertFalse(self.result(current['requestId'])['commitAllowed'])
        self.commit_backup(current, status=409)
        self.prepare_backup(current['requestId'], status=409)
        self.assertEqual(list(self.spool.iterdir()), [])
        self.assertEqual(list(destination.iterdir()), [])

    def test_legacy_unscoped_output(self):
        self.enable_backup()
        self.server.close()
        actor = 'test:unscoped-old-output'
        folder = self.data / 'actors' / digest(actor) / 'operations' / 'outputs'
        folder.mkdir(parents=True)
        original = json.dumps({'actor': actor, 'fingerprint': digest('original')}).encode()
        path = folder / (uid() + '.json')
        path.write_bytes(original)
        self.start()
        result = self.prepare_backup(status=409)
        message = json.dumps(result)
        for secret in [actor, str(path), str(self.data), str(self.root)]:
            self.assertNotIn(secret, message)
        self.assertEqual(path.read_bytes(), original)
        self.assertEqual(list(self.spool.iterdir()), [])


if __name__ == '__main__':
    names = set(sys.argv[2:])
    available = {name for name in HostedOwnerBackup.__dict__ if name.startswith('test_')}
    if names - available:
        sys.exit('Unknown test selection: ' + ', '.join(sorted(names - available)))
    selected = unittest.TestSuite(HostedOwnerBackup(name) for name in sorted(names or available))
    result = unittest.TextTestRunner(verbosity=2).run(selected)
    sys.exit(not result.wasSuccessful())
