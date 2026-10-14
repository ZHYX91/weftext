"""Hosted links/mentions on owned HTTP fixtures; provision through existing Core.

Run only against a freshly built executable containing knowledge_links::router.
These checks do not claim browser Undo, native OS IME or other-platform acceptance.
"""
import json
import secrets
import sys
import time
import unittest
from concurrent.futures import ThreadPoolExecutor

from test_hosted import digest, uid
from test_hosted_history import HostedHistory


class HostedKnowledgeLinks(HostedHistory):
    def replace_body(self, resource, title, body, aliases=()):
        doc = self.open(resource)
        node = self.call('navigation/ref', {'sessionId': doc['sessionId']})['nodeRef']['nodeId']
        terms = ', '.join(json.dumps(term, ensure_ascii=False) for term in aliases)
        source = f'#metadata((id: "{node}", types: ("work",), aliases: ({terms}{"," if terms else ""}))) <weftext>\n'
        source += f'#set document(title: [{title}])\n#title()\n\n{body}\n'
        self.save(doc, source)
        return self.open(resource)

    def basis(self, doc, source=None, serial=1, actor='alice'):
        source = doc['source'] if source is None else source
        req = dict(sessionId=doc['sessionId'], source=source, sourceHash=digest(source), serial=serial)
        self.call('editor/map', req, actor)
        return req

    def links(self, basis, actor='alice', status=200):
        result = self.call('knowledge-links/read', basis, actor, status)
        self.no_physical_paths(result)
        if status == 200:
            self.assertEqual(result['basis']['sourceHash'], basis['sourceHash'])
            self.assertEqual(result['basis']['serial'], basis['serial'])
            self.assertEqual(result['scope'], 'configuredAuthorizedManagedSources')
        return result

    @staticmethod
    def occurrence(edge):
        return {key: edge['occurrence'][key] for key in ('from', 'to', 'target')}

    def convert_link(self, basis, profile, edge, form, actor='alice', status=200):
        return self.call('knowledge-links/convert', dict(basis=basis, snapshot=profile['basis']['snapshot'],
            intent=dict(kind='link', occurrence=self.occurrence(edge), form=form)), actor, status)

    def test_links_authored_and_backlinks_keep_exact_ranges_and_do_not_save(self):
        shared = self.open()
        shared_ref = self.call('navigation/ref', {'sessionId': shared['sessionId']})['reference']
        other = self.replace_body('Other', '白云山', f'#link("{shared_ref}")[ID 回链]\n#link("../Shared/Shared.typ")[路径回链]')
        ref = self.call('navigation/ref', {'sessionId': other['sessionId']})['reference']
        source = shared['source'] + f'\n#link("{ref}")[*保持显示😀文字*]\n'
        basis = self.basis(shared, source, 4)
        profile = self.links(basis)
        self.assertEqual(len(profile['links']['incomingId']), 1)
        self.assertEqual(len(profile['links']['incomingPath']), 1)
        edge = profile['links']['outgoing'][-1]
        self.assertEqual(edge['resolution']['target']['resource'], 'Other')
        proposal = self.convert_link(basis, profile, edge, 'relative')
        self.assertFalse(proposal['saved'])
        patch = proposal['patch']
        expected = source.encode()[:patch['range']['from']] + patch['replacement'].encode() + source.encode()[patch['range']['to']:]
        self.assertEqual(expected.decode(), patch['after'])
        self.assertTrue(patch['after'].endswith(')[*保持显示😀文字*]\n'))
        self.assertEqual((self.root/'Shared/Shared.typ').read_text('utf-8'), shared['source'])
        copy = self.call('knowledge-links/copy', dict(basis=basis, snapshot=profile['basis']['snapshot'], occurrence=self.occurrence(edge), form='workspace'))
        self.assertIn('Other/Other.typ', copy['address'])
        self.assertTrue(copy['fullLink'].endswith(')[*保持显示😀文字*]'))
        self.no_physical_paths([proposal, copy])

    def test_links_overlapping_aliases_and_equal_terms_keep_all_authorized_choices(self):
        self.policy(lambda users: users['alice']['grants']['Secret'].update(fields={'core/aliases': {'read': True}}))
        self.replace_body('Other', '白云山', '可读目标正文')
        self.replace_body('Secret', '山', '隐藏目标正文', aliases=('山南路',))
        shared = self.replace_body('Shared', '当前笔记', '白云山南路')
        basis = self.basis(shared, serial=5)
        profile = self.links(basis)
        hits = [(fragment, hit) for fragment in profile['mentions']['fragments'] for hit in fragment['hits']]
        terms = {term for _, hit in hits for term in hit['terms']}
        self.assertTrue({'白云山', '山', '山南路'}.issubset(terms), profile['mentions'])
        self.assertGreaterEqual(len(hits), 3)
        fragment, hit = next((f, h) for f, h in hits if '山南路' in h['terms'])
        target = hit['targets'][0]
        proposal = self.call('knowledge-links/convert', dict(basis=basis, snapshot=profile['basis']['snapshot'],
            intent=dict(kind='mention', fragmentId=fragment['id'], startScalar=hit['startScalar'], endScalar=hit['endScalar'], target=target['nodeRef'])))
        self.assertIn('#link(', proposal['patch']['after'])
        self.assertEqual(proposal['patch']['range']['raw'], '山南路')
        bob = self.open(actor='bob')
        limited = self.links(self.basis(bob, serial=5, actor='bob'), 'bob')
        for fragment in limited['mentions']['fragments']:
            for hit in fragment['hits']:
                self.assertNotIn('山南路', hit['terms'])
                self.assertNotIn('Secret', [target['resource'] for target in hit['targets']])
        self.assertNotIn('隐藏目标正文', json.dumps(limited, ensure_ascii=False))

    def test_links_source_words_do_not_gain_private_schema_meanings_without_field_read(self):
        self.replace_body('Other', '可见标题', '普通正文', aliases=('OnlySchemaName',))
        shared = self.replace_body('Shared', '当前笔记', 'OnlySchemaName')
        profile = self.links(self.basis(shared, serial=1))
        self.assertFalse(any('OnlySchemaName' in hit['terms'] for f in profile['mentions']['fragments'] for hit in f['hits']))
        self.assertEqual(profile['propertyReferences']['items'], [])
        # Reading the complete Schema still does not grant the separate field
        # product; the exact alias string was already present in source bytes.
        self.policy(lambda users: users['alice'].update(library={'schema': {'read': True}}))
        shared = self.open()
        profile = self.links(self.basis(shared, serial=2))
        self.assertFalse(any('OnlySchemaName' in hit['terms'] for f in profile['mentions']['fragments'] for hit in f['hits']))
        self.policy(lambda users: users['alice']['grants']['Other'].update(fields={'core/aliases': {'read': True}}))
        shared = self.open()
        profile = self.links(self.basis(shared, serial=3))
        self.assertTrue(any('OnlySchemaName' in hit['terms'] for f in profile['mentions']['fragments'] for hit in f['hits']), profile['mentions'])

    def test_links_current_draft_fragments_keep_original_encoding_and_reject_missing_duplicate(self):
        target = self.replace_body('Other', '目标', '= 章节 <章>\n\n= 重复一 <dup>\n\n= 重复二 <dup>')
        reference = self.call('navigation/ref', {'sessionId': target['sessionId']})['reference']
        doc = self.open()
        source = doc['source'] + f'\n#link("{reference}#%E7%AB%A0")[*显示文字*]\n#link("{reference}#dup")[duplicate]\n#link("{reference}#missing")[missing]\n'
        basis = self.basis(doc, source, 8)
        profile = self.links(basis)
        edges = {edge['occurrence']['target'].rsplit('#', 1)[-1]: edge for edge in profile['links']['outgoing']}
        self.assertEqual(edges['%E7%AB%A0']['resolution']['status'], 'resolved')
        self.assertEqual(edges['dup']['resolution']['status'], 'ambiguous')
        self.assertEqual(edges['missing']['resolution']['status'], 'fragmentMissing')
        proposal = self.convert_link(basis, profile, edges['%E7%AB%A0'], 'relative')
        self.assertIn('#%E7%AB%A0', proposal['patch']['after'])
        self.assertIn(')[*显示文字*]', proposal['patch']['after'])
        for name in ('dup', 'missing'):
            self.assertEqual(self.convert_link(basis, profile, edges[name], 'relative', status=422)['error']['code'], 'knowledgeLinksUnavailable')

    def test_links_hidden_unknown_readonly_copy_and_server_file_addresses_stay_separate(self):
        secret = self.open('Secret')
        hidden = self.call('navigation/ref', {'sessionId': secret['sessionId']})['reference']
        unknown = f'weftext-node://{uid()}/{uid()}'
        reader = self.open(actor='reader')
        source = reader['source'] + f'\n#link("{hidden}")[hidden]\n#link("{unknown}")[unknown]\n#link("file:///D:/authored/source.typ#%E7%AB%A0")[*原文字*]\n'
        basis = self.basis(reader, source, 2, 'reader')
        profile = self.links(basis, 'reader')
        edges = profile['links']['outgoing']
        self.assertEqual(edges[0]['resolution'], edges[1]['resolution'])
        self.assertFalse(profile['writable'])
        edge = edges[-1]
        copied = self.call('knowledge-links/copy', dict(basis=basis, snapshot=profile['basis']['snapshot'], occurrence=self.occurrence(edge), form='original'), 'reader')
        self.assertEqual(copied['address'], 'file:///D:/authored/source.typ#%E7%AB%A0')
        self.assertEqual(copied['fullLink'], '#link("file:///D:/authored/source.typ#%E7%AB%A0")[*原文字*]')
        self.call('knowledge-links/copy', dict(basis=basis, snapshot=profile['basis']['snapshot'], occurrence=self.occurrence(edge), form='file'), 'reader', 422)
        self.convert_link(basis, profile, edge, 'id', 'reader', 404)

    def test_links_old_serial_scope_and_real_move_need_fresh_displayed_basis(self):
        other = self.open('Other')
        reference = self.call('navigation/ref', {'sessionId': other['sessionId']})['reference']
        doc = self.open()
        source = doc['source'] + f'\n#link("{reference}")[current target]\n'
        basis = self.basis(doc, source, 6)
        profile = self.links(basis)
        edge = profile['links']['outgoing'][-1]
        self.basis(doc, source+'later', 7)
        self.basis(doc, source, 8)
        self.convert_link(basis, profile, edge, 'relative', status=409)
        current = self.basis(doc, source, 8)
        profile = self.links(current)
        other = self.open('Other')
        self.save(other, other['source']+'\nchanged target')
        self.convert_link(current, profile, profile['links']['outgoing'][-1], 'relative', status=409)
        def move(api):
            plan = api('/api/operations/prepare', dict(root=str(self.root), operationId=uid(), intent=dict(kind='renameNode', source=str(self.root/'Other'), name='Moved')))
            api('/api/operations/commit', dict(operationId=plan['operationId'], planId=plan['planId']))
            self.config['files']['Other']['path'] = 'Moved/Moved.typ'
        self.local(move)
        doc = self.open()
        fresh = self.links(self.basis(doc, source, 9))
        self.assertEqual(fresh['links']['outgoing'][-1]['resolution']['target']['resource'], 'Other')
        self.assertEqual(fresh['links']['outgoing'][-1]['resolution']['status'], 'resolved')

    def test_links_inflight_revocation_does_not_deliver_old_graph_or_mentions(self):
        doc = self.open()
        source = doc['source'] + '\n#for i in range(20000) [delayed paragraph #parbreak()]'
        basis = self.basis(doc, source, 20)
        with ThreadPoolExecutor() as pool:
            pending = pool.submit(self.server.call, '/api/hosted/knowledge-links/read', basis, self.tokens['alice'])
            time.sleep(.2)
            self.policy(lambda users: users['alice'].update(enabled=False))
            status, response, _ = pending.result(timeout=40)
        self.assertEqual(status, 401, response)
        self.assertEqual(response['error']['code'], 'authentication')
        self.assertNotIn('links', response)
        self.assertNotIn('mentions', response)

    def test_links_configured_readonly_managed_notes_open_without_admitting_compiler_inputs(self):
        shared = self.open()
        shared_ref = self.call('navigation/ref', {'sessionId': shared['sessionId']})['reference']
        other = self.replace_body('Other', 'ReadOnlyTitle', f'#link("{shared_ref}")[readonly backlink]', aliases=('ReadOnlySemanticName',))
        other_ref = self.call('navigation/ref', {'sessionId': other['sessionId']})['reference']
        def make_readonly(users):
            for user in users.values():
                if 'Other' in user['grants']:
                    user['grants']['Other']['write'] = False
            users['alice']['grants']['Other']['fields'] = {'core/aliases': {'read': True}}
        self.policy(make_readonly)
        # Restart validates persisted authority against the new file flag.
        # Bootstrap users do not overwrite an already persisted policy.
        self.config['users'] = self.call('admin/policy', actor='admin')['users']
        self.server.close()
        self.config['files']['Other']['editable'] = False
        for user in self.config['users'].values():
            if 'Other' in user['grants']:
                user['grants']['Other']['write'] = False
        self.config['users']['alice']['grants']['Other']['fields'] = {'core/aliases': {'read': True}}
        (self.root/'Compiler').mkdir()
        (self.root/'Compiler/Compiler.typ').write_text('#let compiler_only = "CompilerInputPayload"\n', 'utf-8')
        (self.root/'Data.json').write_text('{"private": "JsonInputPayload"}', 'utf-8')
        for name, path in [('CompilerInput', 'Compiler/Compiler.typ'), ('JsonInput', 'Data.json')]:
            self.config['files'][name] = {'path': path, 'title': name, 'editable': False}
            self.config['users']['alice']['grants'][name] = {'read': True}
        self.config['files']['Shared']['inputs'] = ['CompilerInput', 'JsonInput']
        self.tokens['fieldOnly'] = secrets.token_urlsafe(32)
        self.config['users']['fieldOnly'] = {'enabled': True, 'tokenSha256': digest(self.tokens['fieldOnly']),
            'grants': {'Other': {'fields': {'core/aliases': {'read': True}}}}}
        self.start()
        def install_read_grants(users):
            for name in ['CompilerInput', 'JsonInput']:
                users['alice']['grants'][name] = {'read': True}
            users['fieldOnly'] = self.config['users']['fieldOnly']
        self.policy(install_read_grants)
        documents = self.call('documents')['documents']
        self.assertIn('Other', {row['resource'] for row in documents})
        for name in ['CompilerInput', 'JsonInput']:
            self.assertNotIn(name, {row['resource'] for row in documents})
            self.call('open', {'resource': name}, status=404)
        self.assertEqual(self.call('search', {'text': 'InputPayload'})['rows'], [])
        readonly = self.open('Other')
        self.assertFalse(readonly['rights']['write'])
        self.assertEqual(self.call('navigation/ref', {'sessionId': readonly['sessionId']})['reference'], other_ref)
        self.assertTrue(self.call('navigation/resolve', {'reference': other_ref})['canOpen'])
        timeline = self.timeline(readonly)
        available = next(row['versionId'] for row in timeline['versions'] if row['status'] == 'available'
            and self.read_version(readonly, row['versionId'])['source'] == readonly['source'])
        self.assertIn('ReadOnlyTitle', self.read_version(readonly, available)['source'])
        self.call('history/restore', self.restore_request(readonly, available), status=404)
        self.save(readonly, readonly['source']+'must-not-write', status=404)
        field = self.call('fields/read', {'resource': 'Other', 'fieldId': 'core/aliases'}, 'fieldOnly')
        self.assertFalse(field['field']['writable'])
        self.assertIsNone(field['basis'])
        self.assertIn('ReadOnlySemanticName', json.dumps(field))
        self.assertNotIn('readonly backlink', json.dumps(field))
        self.call('open', {'resource': 'Other'}, 'fieldOnly', 404)
        shared = self.open()
        source = shared['source'] + f'\n#link("{other_ref}")[read-only target]\nReadOnlySemanticName\n'
        basis = self.basis(shared, source, 2)
        profile = self.links(basis)
        edge = profile['links']['outgoing'][-1]
        self.assertTrue(edge['resolution']['canOpen'])
        self.assertEqual(profile['links']['incomingId'][0]['source']['resource'], 'Other')
        self.assertTrue(any('ReadOnlySemanticName' in hit['terms'] for f in profile['mentions']['fragments'] for hit in f['hits']))
        self.assertFalse(self.convert_link(basis, profile, edge, 'relative')['saved'])


if __name__ == '__main__':
    selected = set(sys.argv[2:])
    suite = unittest.TestSuite(HostedKnowledgeLinks(name) for name in sorted(HostedKnowledgeLinks.__dict__)
        if name.startswith('test_links_') and (not selected or name in selected))
    sys.exit(not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful())
