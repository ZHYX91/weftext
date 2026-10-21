"""Real hosted history restore and stable Node navigation acceptance.

Each case owns a disposable library and private state directory. Local APIs are
used only to provision structural moves while the hosted process is stopped.
Consumer behavior always goes through the hosted authorization boundary.
"""
import json
import sys
import unittest

from test_hosted import Hosted, Server, digest, uid


class HostedHistory(Hosted):
    def timeline(self, doc, actor='alice'):
        return self.call('history/list', {'sessionId': doc['sessionId']}, actor)

    def read_version(self, doc, version, actor='alice', status=200):
        return self.call('history/read', {'sessionId': doc['sessionId'],
                                         'versionId': version}, actor, status)

    def versions(self, resource='Shared'):
        doc = self.open(resource)
        metadata = doc['source'].split('\n', 1)[0]
        sources = []
        for letter in 'ABC':
            source = metadata + f'\r\n\r\n= 版本 {letter}🌿\r\n准确正文 {letter}。\r\n'
            self.save(doc, source)
            sources.append(source)
            doc = self.open(resource)
        timeline = self.timeline(doc)
        # Hosted operation IDs are actor scoped. Locate the actual version by
        # its exact bytes, never by wall-clock order or the external request ID.
        found = {}
        for row in timeline['versions']:
            if row['status'] == 'available':
                source = self.read_version(doc, row['versionId'])['source']
                if source in sources:
                    found[source] = row['versionId']
        self.assertEqual(set(found), set(sources))
        return doc, [{'source': source, 'id': found[source]} for source in sources]

    def restore_request(self, doc, version, *, source=None, serial=1,
                        action='none', operation=None, actor='alice'):
        source = doc['source'] if source is None else source
        request = dict(resource=doc['resource'], sessionId=doc['sessionId'],
                       baseId=doc['baseId'], versionId=version, source=source,
                       sourceHash=digest(source), serial=serial,
                       operationId=operation or uid(), draftAction=action)
        self.call('editor/map', {key: request[key] for key in
                                ['sessionId', 'source', 'sourceHash', 'serial']}, actor)
        return request

    def restore_status(self, operation, resource='Shared', actor='alice'):
        return self.call('history/status', {'resource': resource, 'operationId': operation}, actor)

    def no_physical_paths(self, value):
        serialized = json.dumps(value, ensure_ascii=False).replace('\\\\', '\\')
        for path in (self.root, self.data):
            self.assertNotIn(str(path), serialized)
            self.assertNotIn(path.as_posix(), serialized)

    def local(self, action):
        self.server.close()
        local = Server([ '--data-dir', str(self.data)])
        def api(route, data):
            code, result, _ = local.call(route, data)
            self.assertEqual(code, 200, str(result)[:1500])
            return result
        try:
            return action(api)
        finally:
            local.close()
            self.start()

    def test_history_consumer_exact_versions_and_comparisons_preserve_both_drafts(self):
        doc, versions = self.versions()
        alice = doc['source'] + '\nALICE_PRIVATE_PERSISTED😀'
        bob = doc['source'] + '\nBOB_PRIVATE_PERSISTED🌿'
        self.call('draft', {'sessionId': doc['sessionId'], 'source': alice})
        b = self.open(actor='bob')
        self.call('draft', {'sessionId': b['sessionId'], 'source': bob}, 'bob')
        before = self.timeline(doc)
        self.assertEqual(before['path'], 'resource:Shared')
        self.assertEqual(before['currentness'], 'saved')
        self.assertEqual(before['currentSavedVersionId'], versions[2]['id'])
        for version in versions:
            read = self.read_version(doc, version['id'])
            self.assertEqual(read['source'], version['source'])
            self.assertTrue(read['readOnly'])
            self.assertEqual(read['previewStatus'], 'sourceOnly')
        saved = self.call('history/compare', {'sessionId': doc['sessionId'], 'versionId': versions[0]['id']})
        self.assertEqual(saved['comparison'], 'currentSaved')
        self.assertEqual(saved['selected']['source'], versions[0]['source'])
        self.assertEqual(saved['current']['source'], versions[2]['source'])
        current_buffer = alice + '\nEXACT_UNSAVED_BUFFER'
        self.call('editor/map', dict(sessionId=doc['sessionId'], source=current_buffer,
                  serial=8, sourceHash=digest(current_buffer)))
        draft = self.call('history/compare-draft', dict(sessionId=doc['sessionId'],
                          versionId=versions[0]['id'], source=current_buffer,
                          serial=8, sourceHash=digest(current_buffer)))
        self.assertEqual(draft['comparison'], 'draftSnapshot')
        self.assertEqual(draft['current']['source'], current_buffer)
        self.assertEqual(draft['current']['serial'], 8)
        self.assertEqual(draft['current']['sourceHash'], digest(current_buffer))
        self.assertFalse(draft['equal'])
        self.assertEqual(self.open()['source'], versions[2]['source'])
        self.assertEqual(self.open()['recoveredDraft'], alice)
        self.assertEqual(self.open(actor='bob')['recoveredDraft'], bob)
        self.assertEqual(self.timeline(doc)['versions'], before['versions'])
        self.assertNotIn('BOB_PRIVATE', json.dumps([before, saved, draft]))
        self.no_physical_paths([before, saved, draft])

    def test_history_consumer_restore_protects_persisted_and_buffer_and_replays_after_restart(self):
        doc, versions = self.versions()
        persisted = doc['source'] + '\nALICE_OLD_DRAFT'
        buffer = persisted + '\nALICE_LATEST_BUFFER😀'
        self.call('draft', {'sessionId': doc['sessionId'], 'source': persisted})
        b = self.open(actor='bob')
        bob = b['source'] + '\nBOB_PRIVATE_RETAINED'
        self.call('draft', {'sessionId': b['sessionId'], 'source': bob}, 'bob')
        before = self.timeline(doc)
        request = self.restore_request(doc, versions[0]['id'], source=buffer, action='retain')
        restored = self.call('history/restore', request)
        self.assertEqual(restored['status'], 'committed')
        self.assertEqual(restored['operationId'], request['operationId'])
        result = restored['result']
        self.assertEqual(result['receipt']['operationId'], request['operationId'])
        self.assertEqual(result['source'], versions[0]['source'])
        self.assertEqual({item['source'] for item in result['protectedDrafts']}, {persisted, buffer})
        self.assertTrue(all(item['baseKnown'] for item in result['protectedDrafts']))
        self.assertNotIn('BOB_PRIVATE', json.dumps(restored))
        self.no_physical_paths(restored)
        self.assertEqual(self.open()['source'], versions[0]['source'])
        after = self.timeline(self.open())
        self.assertEqual(len(after['versions']), len(before['versions']) + 1)
        self.assertEqual(self.open(actor='bob')['recoveredDraft'], bob)
        for version in versions:
            self.assertEqual(self.read_version(self.open(), version['id'])['source'], version['source'])
        self.server.close()
        self.start()
        status = self.restore_status(request['operationId'])
        self.assertEqual(status['status'], 'committed')
        self.assertEqual(status['result'], result)
        self.assertEqual(self.call('history/restore', request)['result'], result)
        self.assertEqual(self.timeline(self.open())['versions'], after['versions'])
        self.call('history/restore', {**request, 'versionId': versions[1]['id']}, status=409)
        self.assertEqual(self.restore_status(request['operationId'], actor='bob')['status'], 'notRegistered')

    def test_history_consumer_dirty_none_and_stale_buffer_never_install(self):
        doc, versions = self.versions()
        dirty = doc['source'] + '\nPROTECT_ME'
        self.call('draft', {'sessionId': doc['sessionId'], 'source': dirty})
        request = self.restore_request(doc, versions[0]['id'], source=dirty)
        denied = self.call('history/restore', request, status=409)
        self.assertEqual(denied['error']['code'], 'draftProtectionRequired')
        self.assertEqual(self.open()['source'], versions[2]['source'])
        self.assertEqual(self.open()['recoveredDraft'], dirty)
        newer = dirty + '\nNEWER_BUFFER'
        self.call('editor/map', dict(sessionId=doc['sessionId'], source=newer,
                                    sourceHash=digest(newer), serial=3))
        self.call('history/restore', {**request, 'operationId': uid(), 'draftAction': 'retain'}, status=409)
        self.call('history/compare-draft', dict(sessionId=doc['sessionId'],
                  versionId=versions[0]['id'], source=dirty, sourceHash=digest(dirty), serial=1), status=409)
        self.assertEqual(self.open()['source'], versions[2]['source'])

    def test_history_consumer_browse_and_restore_permissions_are_separate(self):
        doc, versions = self.versions()
        self.policy(lambda users: users['reader']['grants']['Shared'].update(history=True))
        reader = self.open(actor='reader')
        self.assertEqual(self.read_version(reader, versions[0]['id'], 'reader')['source'], versions[0]['source'])
        self.assertEqual(self.timeline(reader, 'reader')['currentness'], 'saved')
        request = self.restore_request(reader, versions[0]['id'], actor='reader')
        self.call('history/restore', request, 'reader', 404)
        self.call('history/settle', {'resource': 'Shared', 'operationId': uid()}, 'reader', 404)
        self.policy(lambda users: users['bob']['grants']['Shared'].update(history=False))
        bob = self.open(actor='bob')
        self.call('history/list', {'sessionId': bob['sessionId']}, 'bob', 404)
        request = self.restore_request(bob, versions[0]['id'], actor='bob')
        self.call('history/restore', request, 'bob', 404)
        self.assertEqual(self.open()['source'], versions[2]['source'])

    def test_history_consumer_external_change_preserves_external_and_personal_draft(self):
        doc, versions = self.versions()
        draft = doc['source'] + '\nALICE_BEFORE_EXTERNAL'
        self.call('draft', {'sessionId': doc['sessionId'], 'source': draft})
        request = self.restore_request(doc, versions[0]['id'], source=draft, action='retain')
        external = doc['source'] + '\nEXTERNAL_CURRENT🌿'
        (self.root / 'Shared/Shared.typ').write_bytes(external.encode())
        denied = self.call('history/restore', request, status=409)
        self.assertEqual(denied['error']['code'], 'staleBase')
        reopened = self.open()
        self.assertEqual(reopened['source'], external)
        self.assertEqual(reopened['recoveredDraft'], draft)
        self.assertTrue(reopened['recoveredDraftConflict'])
        self.assertEqual(self.timeline(reopened)['currentness'], 'external')
        self.assertEqual(self.read_version(reopened, versions[0]['id'])['source'], versions[0]['source'])
        self.call('history/compare', {'sessionId': reopened['sessionId'], 'versionId': versions[0]['id']}, status=409)

    def test_history_consumer_unknown_settlement_and_operation_categories_fence_late_writes(self):
        doc, versions = self.versions()
        request = self.restore_request(doc, versions[0]['id'])
        operation = request['operationId']
        self.assertEqual(self.restore_status(operation)['status'], 'notRegistered')
        settled = self.call('history/settle', {'resource': 'Shared', 'operationId': operation})
        self.assertEqual(settled['status'], 'cancelled')
        self.server.close()
        self.start()
        self.assertEqual(self.restore_status(operation)['status'], 'cancelled')
        self.assertEqual(self.call('history/restore', request)['status'], 'cancelled')
        self.assertEqual(self.open()['source'], versions[2]['source'])
        fresh = self.open()
        saved, save_request = self.save(fresh, fresh['source'] + '\nCATEGORY_SAVE')
        self.assertEqual(saved['status'], 'committed')
        fresh = self.open()
        collide = self.restore_request(fresh, versions[0]['id'], operation=save_request['operationId'])
        self.call('history/restore', collide, status=409)
        # Category exclusivity also applies in the other direction; settling a
        # never-sent restore cannot later become an ordinary source save.
        self.save(fresh, fresh['source'] + '\nFORBIDDEN_LATE_SAVE', status=409, op=operation)
        self.assertNotIn('FORBIDDEN', self.open()['source'])

    def test_history_consumer_committed_fact_survives_newer_save_and_owner_remap(self):
        doc, versions = self.versions()
        request = self.restore_request(doc, versions[0]['id'])
        original = self.call('history/restore', request)
        self.assertEqual(original['status'], 'committed')
        self.assertEqual(original['resultBasis'], 'current')
        self.save(self.open(), versions[0]['source'])
        self.assertEqual(self.restore_status(request['operationId'])['resultBasis'], 'superseded')
        fresh = self.open()
        self.save(fresh, fresh['source'] + '\nLATER_SAVED_F')
        superseded = self.restore_status(request['operationId'])
        self.assertEqual(superseded['status'], 'committed')
        self.assertEqual(superseded['resultBasis'], 'superseded')
        self.assertEqual(superseded['result']['source'], versions[0]['source'])
        # Identical author bytes in a distinct committed revision do not revive
        # the original restore receipt as the current basis.
        self.save(self.open(), versions[0]['source'])
        self.assertEqual(self.restore_status(request['operationId'])['resultBasis'], 'superseded')
        self.server.close()
        original_path = self.config['files']['Shared']['path']
        self.config['files']['Shared']['path'] = self.config['files']['Other']['path']
        self.config['files']['Other']['path'] = original_path
        self.start()
        remapped = self.restore_status(request['operationId'])
        self.assertEqual(remapped['status'], 'committed')
        self.assertEqual(remapped['resultStatus'], 'unavailable')
        self.assertNotIn('result', remapped)
        self.assertNotIn(versions[0]['source'], json.dumps(remapped, ensure_ascii=False))
        self.assertIn('独立正文', self.open()['source'])

    def test_history_consumer_foreign_version_and_revocation_do_not_deliver_private_contents(self):
        doc, versions = self.versions()
        other = self.open('Other')
        self.read_version(other, versions[0]['id'], status=409)
        request = self.restore_request(other, versions[0]['id'])
        self.call('history/restore', request, status=409)
        self.assertEqual(self.open('Other')['source'], other['source'])
        request = self.restore_request(doc, versions[0]['id'])
        self.call('history/restore', request)
        self.policy(lambda users: users['alice']['grants']['Shared'].update(history=False))
        self.call('history/status', {'resource': 'Shared', 'operationId': request['operationId']}, status=404)
        self.call('history/restore', request, status=404)
        current = self.open()
        self.call('history/read', {'sessionId': current['sessionId'], 'versionId': versions[0]['id']}, status=404)
        self.policy(lambda users: users['alice']['grants']['Shared'].update(history=True))
        self.assertEqual(self.restore_status(request['operationId'])['status'], 'committed')
        self.call('history/read', {'sessionId': current['sessionId'], 'versionId': versions[0]['id']}, status=410)

    def test_history_consumer_navigation_requires_target_read_and_qualified_identity(self):
        doc = self.open()
        reference = self.call('navigation/ref', {'sessionId': doc['sessionId']})
        self.assertEqual(reference['resource'], 'Shared')
        self.assertTrue(reference['reference'].startswith('weftext-node://'))
        self.assertEqual(reference['nodeRef']['nodeId'], self.docs['Shared']['nodeId'])
        reader = self.open(actor='reader')
        self.assertEqual(self.call('navigation/ref', {'sessionId': reader['sessionId']}, 'reader')['reference'], reference['reference'])
        for target in (reference['reference'], reference['nodeRef']):
            resolved = self.call('navigation/resolve', {'reference': target}, 'reader')
            self.assertEqual(resolved['resource'], 'Shared')
            self.assertTrue(resolved['canOpen'])
            self.assertEqual(resolved['scope'], 'configuredAuthorizedSources')
            self.assertEqual(resolved['coverage'], 'complete')
            self.no_physical_paths(resolved)
        link_source = (doc['source'] + f'\n#link("{reference["reference"]}")[稳定笔记]\n'
                       '#link("https://example.test/reference")[外部资料]\n')
        rendered = self.call('editor/render', dict(sessionId=doc['sessionId'], source=link_source,
                             sourceHash=digest(link_source), serial=1))
        self.assertTrue(rendered['ok'], rendered)
        def links(node):
            if not isinstance(node, dict):
                return []
            return ([node['attrs']['href']] if node.get('tag') == 'a' and node.get('attrs', {}).get('href') else []) + [
                href for child in node.get('children', []) for href in links(child)]
        self.assertIn(reference['reference'], links(rendered['tree']))
        self.assertIn('https://example.test/reference', links(rendered['tree']))
        self.assertEqual(self.open()['source'], doc['source'])
        secret = self.call('navigation/ref', {'sessionId': self.open('Secret')['sessionId']})
        hidden = self.call('navigation/resolve', {'reference': secret['reference']}, 'bob', 404)
        absent = self.call('navigation/resolve', {'reference': {
            **reference['nodeRef'], 'nodeId': uid()}}, 'bob', 404)
        self.assertEqual(hidden, absent)
        path = self.root / 'Shared/Shared.typ'
        path.write_bytes(doc['source'].replace(self.docs['Shared']['nodeId'], uid()).encode())
        replaced = self.call('navigation/resolve', {'reference': reference['reference']}, 'bob', 404)
        self.assertEqual(replaced, hidden)
        self.assertNotIn('HIDDEN_PAYLOAD', json.dumps([hidden, absent, replaced]))

    def test_history_consumer_navigation_and_history_follow_proven_move_not_copy(self):
        doc, versions = self.versions()
        reference = self.call('navigation/ref', {'sessionId': doc['sessionId']})
        def rename_and_copy(api):
            preview = api('/api/operations/prepare', {'root': str(self.root), 'operationId': uid(),
                'intent': {'kind': 'renameNode', 'source': str(self.root / 'Shared'), 'name': 'Moved'}})
            moved = api('/api/operations/commit', {'operationId': preview['operationId'], 'planId': preview['planId']})
            self.assertEqual(moved['status'], 'committed')
            preview = api('/api/operations/prepare', {'root': str(self.root), 'operationId': uid(),
                'intent': {'kind': 'copyNode', 'source': str(self.root / 'Moved'),
                           'parent': str(self.root), 'name': 'Shared'}})
            copied = api('/api/operations/commit', {'operationId': preview['operationId'], 'planId': preview['planId']})
            self.assertEqual(copied['status'], 'committed')
            self.config['files']['Shared']['path'] = 'Moved/Moved.typ'
            self.config['files']['Other']['path'] = 'Shared/Shared.typ'
        self.local(rename_and_copy)
        resolved = self.call('navigation/resolve', {'reference': reference['reference']})
        self.assertEqual(resolved['resource'], 'Shared')
        moved = self.open()
        self.assertEqual(self.call('navigation/ref', {'sessionId': moved['sessionId']})['reference'], reference['reference'])
        for version in versions:
            self.assertEqual(self.read_version(moved, version['id'])['source'], version['source'])
        copied = self.open('Other')
        copy_ref = self.call('navigation/ref', {'sessionId': copied['sessionId']})
        self.assertNotEqual(copy_ref['nodeRef']['nodeId'], reference['nodeRef']['nodeId'])
        self.read_version(copied, versions[0]['id'], status=409)
        self.assertEqual(self.call('navigation/resolve', {'reference': copy_ref['reference']})['resource'], 'Other')
        self.policy(lambda users: users['bob']['grants']['Shared'].update(read=False, write=False, history=False, download=False))
        self.call('navigation/resolve', {'reference': reference['reference']}, 'bob', 404)
        self.assertEqual(self.call('navigation/resolve', {'reference': copy_ref['reference']}, 'bob')['resource'], 'Other')


if __name__ == '__main__':
    suite = unittest.TestSuite(HostedHistory(name) for name in
        unittest.defaultTestLoader.getTestCaseNames(HostedHistory)
        if name.startswith('test_history_consumer_'))
    sys.exit(0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1)
