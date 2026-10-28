"""Actual local HTTP received-history reads; never install an inspected version.

Own fixtures and processes use the independent product harness. Historical
content is compared to the exact originally saved bytes, not a newer head.
"""
import uuid
import hashlib
import shutil
import sys
import unittest

from test_hosted import Server, uid
from test_sync import Sync


class ReceivedHistory(Sync):
    def history(self, server=None, object_key=None, **page):
        return self.call(server or self.b, '/api/sync/history', identity=self.identity,
                         object=object_key or self.key, **page)

    def test_received_retained_ancestor_pagination_exact_bytes_and_draft(self):
        original = self.open(self.b, self.rb)
        draft = original['source'] + '\n历史查看期间保留的草稿 😀'
        self.call(self.b, '/api/draft', inputId=str(uuid.uuid4()), inputRevision=1, sessionId=original['sessionId'],
                  baseId=original['baseId'], source=draft)
        before_binding = self.call(self.b, '/api/sync/binding', root=str(self.rb))
        initial = self.history()['versions'][0]
        expected = {initial['revision']: original['source']}
        writer = self.open(self.a, self.ra)
        for suffix in ['历史二 中文', '历史三 😀', '当前头']:
            source = writer['source'] + '\n' + suffix
            operation = uid()
            saved = self.call(self.a, '/api/save', sessionId=writer['sessionId'],
                              baseId=writer['baseId'], source=source, operationId=operation)
            self.assertEqual(saved['status'], 'saved')
            self.assertEqual(saved['operationId'], operation)
            self.assertNotIn('documentError', saved)
            self.assertTrue(saved['documentSourceMatchesCommit'])
            current = saved['document']
            self.assertEqual(current['sessionId'], writer['sessionId'])
            self.assertEqual(current['source'], source)
            self.assertEqual(current['baseId'], saved['baseId'])
            self.assertEqual((self.ra / 'Note/Note.typ').read_bytes(), source.encode())
            writer = current
            received = self.transfer(self.a, self.ra, self.b, [self.key])
            head = next(o for o in received['objects'] if o['object'] == self.key)['view']['versions'][0]
            expected[head['revision']] = source
        view = next(o for o in received['objects'] if o['object'] == self.key)['view']
        self.assertNotIn(initial['revision'], [v['revision'] for v in view['versions'] + view['commonBases']])
        records, cursor = [], None
        while True:
            page = self.history(limit=1, afterCommit=cursor)
            self.assertEqual(page['coverage'], 'complete')
            self.assertEqual(len(page['versions']), 1)
            records.extend(page['versions'])
            cursor = page['nextCommit']
            if cursor is None:
                break
        self.assertEqual(len(records), len(expected))
        self.assertEqual([v['commit'] for v in records], sorted(v['commit'] for v in records))
        for version in records:
            self.assertEqual(version['admission']['status'], 'accepted')
            result = self.call(self.b, '/api/sync/file', identity=self.identity,
                               object=self.key, revision=version['revision'], path='Note.typ')
            source = expected[version['revision']]
            self.assertEqual(result['text'], source)
            self.assertEqual(result['digest'], hashlib.sha256(source.encode()).hexdigest())
            self.assertEqual(result['fileRevision'], version['state']['fileRevisions']['Note.typ'])
        self.assertEqual(self.call(self.b, '/api/sync/binding', root=str(self.rb)), before_binding)
        reopened = self.open(self.b, self.rb)
        self.assertEqual(reopened['source'], original['source'])
        self.assertEqual(reopened['recoveredDraft'], draft)
        self.b.close()
        self.b = Server([ '--data-dir', str(self.folder / 'state-b')])
        self.assertEqual(self.history()['versions'], records)
        self.assertEqual(self.open(self.b, self.rb)['recoveredDraft'], draft)

    def test_received_branches_expose_actual_causal_links_without_merge(self):
        original = self.history()['versions'][0]
        a = self.save(self.a, self.ra, '甲分支')
        b = self.save(self.b, self.rb, '乙分支')
        self.transfer(self.b, self.rb, self.a, [self.key])
        received = self.transfer(self.a, self.ra, self.b, [self.key])
        versions = self.history()['versions']
        self.assertEqual(len(versions), 3)
        children = [v for v in versions if v['revision'] != original['revision']]
        for child in children:
            self.assertEqual(child['before'], [original['revision']])
            self.assertIn(original['commit'], child['parents'])
        sources = [self.call(self.b, '/api/sync/file', identity=self.identity, object=self.key,
                             revision=v['revision'], path='Note.typ')['text'] for v in children]
        self.assertEqual(set(sources), {a, b})
        view = next(o for o in received['objects'] if o['object'] == self.key)['view']
        self.assertEqual(len(view['versions']), 2)
        self.assertEqual(self.open(self.b, self.rb)['source'], b)
        self.assertEqual(len(self.history()['versions']), 3)

    def test_received_staged_history_refuses_exact_read_until_hydrated(self):
        original = self.open(self.b, self.rb)['source']
        after = self.save(self.a, self.ra, '仅收到提交，尚未收到正文')
        self.call(self.a, '/api/sync/export', root=str(self.ra), directory=str(self.exchange), objects=[self.key])
        partial = self.folder / 'manifests-only'
        partial.mkdir()
        shutil.copyfile(self.exchange / 'identity.json', partial / 'identity.json')
        shutil.copytree(self.exchange / 'manifests', partial / 'manifests')
        (partial / 'blobs').mkdir()
        self.call(self.b, '/api/sync/receive', identity=self.identity, directory=str(partial))
        page = self.history()
        self.assertEqual(page['coverage'], 'partial')
        staged = [v for v in page['versions'] if v['admission']['status'] != 'accepted']
        self.assertEqual(len(staged), 1)
        revision = staged[0]['revision']
        self.assertTrue(staged[0]['admission'].get('missingBlobs'))
        error = self.call(self.b, '/api/sync/file', expected=409, identity=self.identity,
                          object=self.key, revision=revision, path='Note.typ')
        self.assertEqual(error['error']['code'], 'syncIncomplete')
        self.assertEqual(self.open(self.b, self.rb)['source'], original)
        self.b.close()
        self.b = Server([ '--data-dir', str(self.folder / 'state-b')])
        self.assertEqual(self.history(), page)
        self.call(self.b, '/api/sync/receive', identity=self.identity, directory=str(self.exchange))
        self.assertEqual(self.history()['coverage'], 'complete')
        exact = self.call(self.b, '/api/sync/file', identity=self.identity,
                          object=self.key, revision=revision, path='Note.typ')
        self.assertEqual(exact['text'], after)
        self.assertEqual(self.open(self.b, self.rb)['source'], original)

    def test_received_history_bounds_identity_and_auth_are_not_fallbacks(self):
        original = self.history()['versions'][0]
        for body in [dict(limit=0), dict(limit=101), dict(afterCommit='not-a-commit'),
                     dict(afterCommit='a' * 64)]:
            self.call(self.b, '/api/sync/history', expected=409, identity=self.identity, object=self.key, **body)
        wrong = {**self.identity, 'workspaceId': uid()}
        self.call(self.b, '/api/sync/history', expected=409, identity=wrong, object=self.key)
        for object_key, revision, path in [(self.other_key, original['revision'], 'Note.typ'),
                                           (self.key, uid(), 'Note.typ'),
                                           (self.key, original['revision'], '../Note.typ')]:
            self.call(self.b, '/api/sync/file', expected=409, identity=self.identity,
                      object=object_key, revision=revision, path=path)
        for route in ['history', 'file']:
            for headers in [{'X-Weftext-Token': ''}, {'Origin': 'https://invalid.example'}]:
                code, _, _ = self.b.call('/api/sync/' + route, {}, headers=headers)
                self.assertEqual(code, 403)
        self.assertEqual(self.history()['versions'][0]['revision'], original['revision'])

    def adoption_cut(self):
        status = self.call(self.a, '/api/sync/status', identity=self.identity)
        binding = self.call(self.a, '/api/sync/binding', root=str(self.ra))
        objects = [o['object'] for o in status['objects'] if o['object'] in binding['installedObjects']]
        return self.call(self.a, '/api/sync/adoption-cut', root=str(self.ra), identity=self.identity, objects=objects)

    def test_adoption_is_explicit_exact_and_recovers_original_after_restart(self):
        doc = self.open(self.a, self.ra)
        draft = doc['source'] + '\n未保存的私人草稿'
        self.call(self.a, '/api/draft', inputId=str(uuid.uuid4()), inputRevision=1, sessionId=doc['sessionId'], baseId=doc['baseId'], source=draft)
        original = self.call(self.a, '/api/sync/history', identity=self.identity, object=self.key)['versions']
        path = self.ra / 'Note/Note.typ'
        external = doc['source'] + '\n明确采纳外部当前内容'
        path.write_bytes(external.encode('utf-8'))
        # Ordinary Save must not silently turn an external edit into a portable commit.
        self.call(self.a, '/api/save', expected=409, inputRevision=1, sessionId=doc['sessionId'], baseId=doc['baseId'],
                  source=draft, operationId=uid())
        cut = self.adoption_cut()
        self.assertTrue(cut['expectedInstalled'])
        self.assertEqual(len(cut['changes']), 1)
        self.assertEqual(bytes(cut['changes'][0]['before']), external.encode())
        self.assertEqual(cut['changes'][0]['before'], cut['changes'][0]['after'])
        self.assertTrue(cut['changes'][0]['baseId'])
        request = dict(operationId=uid(), root=str(self.ra), intent={'kind': 'adoptSyncExternal', **cut})
        plan = self.call(self.a, '/api/operations/prepare', **request)
        self.assertEqual(path.read_text(encoding='utf-8'), external)
        self.assertEqual(self.call(self.a, '/api/sync/history', identity=self.identity, object=self.key)['versions'], original)
        self.assertEqual(plan, self.call(self.a, '/api/operations/prepare', **request))
        receipt = self.commit(self.a, plan)
        self.assertEqual(receipt['status'], 'committed')
        self.a.close()
        self.a = Server([ '--data-dir', str(self.folder / 'state-a')])
        restored = self.call(self.a, '/api/sync/operation', operationId=request['operationId'])
        self.assertEqual(restored['request'], request)
        self.assertEqual(restored['plan']['planId'], plan['planId'])
        self.assertEqual(restored['plan']['status'], 'committed')
        self.assertEqual(self.commit(self.a, plan), receipt)
        reopened = self.open(self.a, self.ra)
        self.assertEqual(reopened['source'], external)
        self.assertEqual(reopened['recoveredDraft'], draft)
        self.call(self.a, '/api/sync/publish', root=str(self.ra))
        versions = self.call(self.a, '/api/sync/history', identity=self.identity, object=self.key)['versions']
        self.assertEqual(len(versions), len(original) + 1)
        adopted = next(v for v in versions if v['revision'] not in [o['revision'] for o in original])
        self.assertEqual(adopted['before'], [original[0]['revision']])
        exact = self.call(self.a, '/api/sync/file', identity=self.identity, object=self.key,
                          revision=adopted['revision'], path='Note.typ')
        self.assertEqual(exact['text'], external)

    def test_adoption_stale_cut_preserves_third_state_and_cannot_rewrite_content(self):
        doc = self.open(self.a, self.ra)
        path = self.ra / 'Note/Note.typ'
        external = doc['source'] + '\n外部内容一'
        path.write_bytes(external.encode('utf-8'))
        cut = self.adoption_cut()
        request = dict(operationId=uid(), root=str(self.ra), intent={'kind': 'adoptSyncExternal', **cut})
        plan = self.call(self.a, '/api/operations/prepare', **request)
        third = doc['source'] + '\n外部第三态'
        path.write_bytes(third.encode('utf-8'))
        self.assertEqual(plan, self.call(self.a, '/api/operations/prepare', **request))
        self.call(self.a, '/api/operations/commit', expected=409,
                  operationId=plan['operationId'], planId=plan['planId'])
        self.assertEqual(path.read_text(encoding='utf-8'), third)
        cancelled = self.call(self.a, '/api/sync/cancel-prepared', operationId=plan['operationId'], planId=plan['planId'])
        self.assertEqual(cancelled['status'], 'resolved')
        self.call(self.a, '/api/operations/commit', expected=409,
                  operationId=plan['operationId'], planId=plan['planId'])
        changed = self.adoption_cut()
        changed['changes'][0]['after'] = list((third + '\n不得借采纳任意改写').encode())
        self.call(self.a, '/api/operations/prepare', expected=409, operationId=uid(), root=str(self.ra),
                  intent={'kind': 'adoptSyncExternal', **changed})
        self.assertEqual(path.read_text(encoding='utf-8'), third)


if __name__ == '__main__':
    # Only this file's new cases; the imported base owns separate regression tests.
    suite = unittest.TestSuite(ReceivedHistory(name) for name in sorted(vars(ReceivedHistory))
                               if name.startswith('test_'))
    sys.exit(not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful())
