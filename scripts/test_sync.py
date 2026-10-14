"""Two actual local programs, separate private state and author trees. Owned
fixtures are retained; this never drives a desktop or borrows another project.
"""
import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
from test_hosted import Server, uid


class Sync(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.evidence = Path(tempfile.mkdtemp(prefix='weftext-sync-'))
        print('Retained sync evidence:', cls.evidence, flush=True)

    def setUp(self):
        self.folder = self.evidence / self._testMethodName
        self.folder.mkdir()
        self.a = Server(['--no-browser', '--data-dir', str(self.folder / 'state-a')])
        self.addCleanup(lambda: self.a.close())
        self.b = Server(['--no-browser', '--data-dir', str(self.folder / 'state-b')])
        self.addCleanup(lambda: self.b.close())
        self.ra, self.rb = self.folder / 'library-a', self.folder / 'library-b'
        self.call(self.a, '/api/library/create', path=str(self.ra), operationId=uid())
        self.note = self.call(self.a, '/api/note/create', root=str(self.ra), parent=str(self.ra), name='Note', operationId=uid())
        self.other = self.call(self.a, '/api/note/create', root=str(self.ra), parent=str(self.ra), name='Other', operationId=uid())
        self.key = {'kind': 'node', 'id': self.note['nodeId']}
        self.other_key = {'kind': 'node', 'id': self.other['nodeId']}
        operation_id = uid()
        self.binding = self.call(self.a, '/api/sync/enable', root=str(self.ra), operationId=operation_id)
        self.assertEqual(self.binding, self.call(self.a, '/api/sync/enable', root=str(self.ra), operationId=operation_id))
        self.identity = self.binding['identity']
        self.exchange = self.folder / 'exchange'
        self.transfer(self.a, self.ra, self.b, [self.key])
        request = dict(operationId=uid(), root=str(self.rb), intent={'kind':'cloneSync','identity':self.identity,'objects':[self.key]})
        plan = self.call(self.b, '/api/operations/prepare', **request)
        self.assertFalse(self.rb.exists())
        self.assertEqual(plan, self.call(self.b, '/api/operations/prepare', **request))
        self.commit(self.b, plan)
        self.assertFalse((self.rb / 'Other').exists())

    def call(self, server, route, expected=200, **data):
        code, result, _ = server.call(route, data)
        self.assertEqual(code, expected, f'{route}: {result}')
        return result

    def commit(self, server, plan):
        return self.call(server, '/api/operations/commit', operationId=plan['operationId'], planId=plan['planId'])

    def transfer(self, sender, root, receiver, objects):
        self.call(sender, '/api/sync/export', root=str(root), directory=str(self.exchange), objects=objects)
        return self.call(receiver, '/api/sync/receive', identity=self.identity, directory=str(self.exchange))

    def open(self, server, root, name='Note'):
        return self.call(server, '/api/open', root=str(root), path=str(root / name / f'{name}.typ'))

    def save(self, server, root, suffix, name='Note'):
        opened = self.open(server, root, name)
        source = opened['source'] + '\n' + suffix
        self.call(server, '/api/save', sessionId=opened['sessionId'], baseId=opened['baseId'], source=source, operationId=uid())
        return source

    def apply(self, server, root, objects):
        plan = self.call(server, '/api/operations/prepare', operationId=uid(), root=str(root), intent={'kind':'applySync','identity':self.identity,'objects':objects})
        return self.commit(server, plan)

    def test_offline_conflict_partial_hydration_merge_and_retained_draft(self):
        a = self.save(self.a, self.ra, '甲离线提交 😀')
        b = self.save(self.b, self.rb, '乙离线提交 中文')
        other = self.save(self.a, self.ra, '无关对象仍可前进', 'Other')
        binding = self.call(self.b, '/api/sync/binding', root=str(self.rb))
        self.assertTrue(binding['partial'])
        self.assertGreater(binding['pendingPublications'], 0)
        doc = self.open(self.b, self.rb)
        draft = b + '\n尚未保存的个人草稿'
        self.call(self.b, '/api/draft', sessionId=doc['sessionId'], baseId=doc['baseId'], source=draft)
        self.transfer(self.b, self.rb, self.a, [self.key])
        received = self.transfer(self.a, self.ra, self.b, [self.key, self.other_key])
        view = next(o['view'] for o in received['objects'] if o['object'] == self.key)
        self.assertEqual(len(view['versions']), 2)
        self.assertTrue(view['commonBases'])
        sources = []
        for version in view['versions']:
            file = self.call(self.b, '/api/sync/file', identity=self.identity, object=self.key, revision=version['revision'], path='Note.typ')
            sources.append(file['text'])
        self.assertEqual(set(sources), {a, b})
        self.apply(self.b, self.rb, [self.other_key])
        self.assertEqual((self.rb / 'Other/Other.typ').read_text(encoding='utf-8'), other)
        self.assertEqual((self.rb / 'Note/Note.typ').read_text(encoding='utf-8'), b)
        self.call(self.b, '/api/operations/prepare', expected=409, operationId=uid(), root=str(self.rb), intent={'kind':'applySync','identity':self.identity,'objects':[self.key]})
        choose = view['versions'][sources.index(a)]
        resolution = {'object': self.key, 'revisions': [v['revision'] for v in view['versions']], 'state': choose['state']}
        bad = copy.deepcopy(resolution); bad['revisions'].pop()
        self.call(self.b, '/api/operations/prepare', expected=409, operationId=uid(), root=str(self.rb), intent={'kind':'mergeSync','identity':self.identity,'resolutions':[bad],'blobs':{}})
        plan = self.call(self.b, '/api/operations/prepare', operationId=uid(), root=str(self.rb), intent={'kind':'mergeSync','identity':self.identity,'resolutions':[resolution],'blobs':{}})
        self.commit(self.b, plan)
        opened = self.open(self.b, self.rb)
        self.assertEqual(opened['source'], a)
        self.assertEqual(opened['recoveredDraft'], draft)
        self.assertTrue(opened['recoveredDraftConflict'])
        self.transfer(self.b, self.rb, self.a, [self.key])
        self.apply(self.a, self.ra, [self.key])
        self.assertEqual(self.open(self.a, self.ra)['source'], a)

    def test_original_receipt_after_response_loss_restart_and_stale_preview(self):
        source = self.save(self.a, self.ra, '接收版本')
        self.transfer(self.a, self.ra, self.b, [self.key])
        plan = self.call(self.b, '/api/operations/prepare', operationId=uid(), root=str(self.rb), intent={'kind':'applySync','identity':self.identity,'objects':[self.key]})
        # Do not use the first receipt; restart recovers the original durable decision.
        self.commit(self.b, plan)
        self.b.close()
        self.b = Server(['--no-browser', '--data-dir', str(self.folder / 'state-b')])
        result = self.call(self.b, '/api/operations/status', operationId=plan['operationId'])
        self.assertEqual(result['status'], 'committed')
        recovered = self.call(self.b, '/api/sync/operation', operationId=plan['operationId'])
        self.assertEqual(recovered['request']['intent']['kind'], 'applySync')
        self.assertEqual(recovered['plan']['planId'], plan['planId'])
        self.assertEqual(self.commit(self.b, plan), result)
        self.assertEqual(self.open(self.b, self.rb)['source'], source)
        self.save(self.a, self.ra, '下一个版本')
        self.transfer(self.a, self.ra, self.b, [self.key])
        stale = self.call(self.b, '/api/operations/prepare', operationId=uid(), root=str(self.rb), intent={'kind':'applySync','identity':self.identity,'objects':[self.key]})
        path = self.rb / 'Note/Note.typ'; path.write_text(source + '\n外部修改', encoding='utf-8')
        self.call(self.b, '/api/operations/commit', expected=409, operationId=stale['operationId'], planId=stale['planId'])
        self.assertEqual(path.read_text(encoding='utf-8'), source + '\n外部修改')
        self.call(self.b, '/api/sync/cancel-prepared', operationId=stale['operationId'], planId=stale['planId'])
        self.assertEqual(self.call(self.b, '/api/operations/status', operationId=stale['operationId'])['status'], 'resolved')
        self.call(self.b, '/api/operations/commit', expected=409, operationId=stale['operationId'], planId=stale['planId'])

    def test_pairing_auth_corruption_and_unowned_transport_refused(self):
        for route in ['binding', 'enable', 'status', 'publish', 'receive', 'export', 'file', 'cancel-prepared', 'operation']:
            code, _, _ = self.a.call('/api/sync/' + route, {}, headers={'X-Weftext-Token':''})
            self.assertEqual(code, 403)
            code, _, _ = self.a.call('/api/sync/' + route, {}, headers={'Origin':'https://invalid.example'})
            self.assertEqual(code, 403)
        wrong = {**self.identity, 'workspaceId':uid()}
        self.call(self.b, '/api/sync/receive', expected=409, identity=wrong, directory=str(self.exchange))
        existing = self.folder / 'existing'; existing.mkdir()
        self.call(self.a, '/api/sync/export', expected=409, root=str(self.ra), directory=str(existing), objects=[self.key])
        self.assertFalse(any(existing.iterdir()))
        self.call(self.a, '/api/sync/export', expected=409, root=str(self.ra), directory=str(self.ra / 'bad-transport'), objects=[self.key])
        self.call(self.b, '/api/sync/file', expected=409, identity=self.identity, object=self.key, revision=uid(), path='../Note.typ')
        blob = next((self.exchange / 'blobs').glob('*.blob')); blob.write_bytes(b'corrupt')
        before = self.open(self.b, self.rb)['source']
        self.call(self.b, '/api/sync/receive', expected=409, identity=self.identity, directory=str(self.exchange))
        self.assertEqual(self.open(self.b, self.rb)['source'], before)


if __name__ == '__main__':
    unittest.main(argv=[sys.argv[0]], verbosity=2)
