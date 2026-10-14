"""Hosted library configuration and actual native-style consumers.

Uses only owned disposable fixtures. Consumer requests never use local routes.
The shared harness retains all original files and evidence after failures.
"""
import json
import sys
import time
import unittest
import xml.etree.ElementTree as ET
from concurrent.futures import ThreadPoolExecutor
from html.parser import HTMLParser
from test_hosted import uid
from test_hosted_query import HostedQuery
from test_hosted_office import HostedOffice


def visible_html_text(markup):
    class Text(HTMLParser):
        def __init__(self):
            super().__init__()
            self.parts = []

        def handle_data(self, data):
            self.parts.append(data)
    parser = Text()
    parser.feed(markup)
    return ''.join(parser.parts)


class HostedLibrary(HostedQuery):
    ROLES = ('schema', 'settings', 'rules', 'style')

    def grant_library(self, roles=ROLES, actor='alice', write=True, style_use=True):
        self.policy(lambda users: users[actor].update(library={
            **{role: {'read': True, 'write': write} for role in roles},
            'styleUse': style_use}))

    def library_read(self, role, actor='alice'):
        result = self.call('library/read', {'role': role}, actor)
        self.assertEqual(result['role'], role)
        self.assertIsInstance(result['basis'], str)
        self.assertIsInstance(result['source'], str)
        self.assertNotIn(str(self.root), json.dumps(result))
        self.assertNotIn(str(self.data), json.dumps(result))
        return result

    def library_prepare(self, role, source=None, actor='alice', basis=None, **extra):
        before = basis or self.library_read(role, actor)
        request = dict(role=role, basis=before['basis'], operationId=uid(), **extra)
        if source is not None:
            request['source'] = source
        return self.call('library/prepare', request, actor), request

    def library_commit(self, plan, actor='alice', status=200):
        return self.call('library/commit', {key: plan[key] for key in
                         ('role', 'operationId', 'planId')}, actor, status)

    def install_style(self, size='12pt', marker='HOSTED_STYLE_MARKER'):
        source = ('// PRIVATE_STYLE_SOURCE_COMMENT\n'
                  f'#let library-style(body) = {{\n  set text(size: {size})\n'
                  f'  [{marker}]\n  body\n}}\n')
        plan, request = self.library_prepare('style', source, enabled=True)
        self.assertEqual(self.library_commit(plan)['status'], 'committed')
        return plan, request

    def test_library_closed_permissions_and_safe_catalog(self):
        self.assertEqual(self.call('library')['roles'], [])
        for role in self.ROLES:
            denied = self.call('library/read', {'role': role}, 'alice', 404)
            unknown = self.call('library/read', {'role': 'not-configured'}, 'alice', 404)
            self.assertEqual(denied, unknown)
        self.grant_library(roles=('schema',), actor='reader', write=False, style_use=False)
        catalog = self.call('library', actor='reader')
        self.assertEqual([r['role'] for r in catalog['roles']], ['schema'])
        self.assertFalse(catalog['roles'][0]['write'])
        schema = self.library_read('schema', 'reader')
        self.call('library/prepare', dict(role='schema', basis=schema['basis'],
                  source=schema['source'], operationId=uid()), 'reader', 404)
        self.grant_library(roles=('style',), actor='bob', style_use=False)
        self.policy(lambda users: users['bob'].update(grants={}))
        self.library_read('style', 'bob')
        self.call('open', {'resource': 'Shared'}, 'bob', 404)
        self.assertFalse(self.call('library', actor='bob')['styleUse'])

    def test_library_exact_original_settings_plan_restart_and_actor_isolation(self):
        self.grant_library()
        before = {name: (self.root/name/(name+'.typ')).read_bytes() for name in self.docs}
        doc = self.open()
        self.call('draft', {'sessionId': doc['sessionId'], 'source': doc['source']+'\nPERSONAL_DRAFT'})
        source = json.dumps({'version': 1, 'customExtension': {'keep': ['未知', 7]}}, ensure_ascii=False)
        plan, request = self.library_prepare('settings', source)
        self.assertEqual(plan['status'], 'prepared')
        self.assertFalse((self.root/'.weftext-settings.json').exists())
        self.assertEqual(plan['preview']['afterSource'], source)
        self.assertEqual(self.call('library/prepare', request)['planId'], plan['planId'])
        self.call('library/prepare', {**request, 'source': source+' '}, status=409)
        self.server.close(); self.start()
        original = self.call('library/operation', {'role': 'settings', 'operationId': request['operationId']})
        self.assertEqual(original['planId'], plan['planId'])
        self.assertEqual(original['preview'], plan['preview'])
        self.assertEqual(self.library_commit(original)['status'], 'committed')
        self.assertEqual(self.call('library/prepare', request)['status'], 'committed')
        self.assertEqual((self.root/'.weftext-settings.json').read_text('utf-8'), source)
        self.grant_library(roles=('settings',), actor='bob')
        self.assertEqual(self.call('library/operation', {'role': 'settings', 'operationId': request['operationId']}, 'bob')['status'], 'notRegistered')
        self.call('library/operation', {'role': 'rules', 'operationId': request['operationId']}, status=409)
        for name, content in before.items():
            self.assertEqual((self.root/name/(name+'.typ')).read_bytes(), content)
        self.assertIn('PERSONAL_DRAFT', self.open()['recoveredDraft'])

    def test_library_conflict_settlement_and_epoch_do_not_overwrite(self):
        self.grant_library()
        first, request = self.library_prepare('settings', '{"version":1,"choice":"first"}')
        second, _ = self.library_prepare('settings', '{"version":1,"choice":"second"}')
        self.assertEqual(self.library_commit(first)['status'], 'committed')
        self.library_commit(second, status=409)
        self.assertEqual(json.loads((self.root/'.weftext-settings.json').read_text('utf-8'))['choice'], 'first')
        current = self.library_read('settings')
        late = dict(role='settings', basis=current['basis'], operationId=uid(), source='{"version":1,"choice":"late"}')
        op = {key: late[key] for key in ('role', 'operationId')}
        self.assertEqual(self.call('library/operation', op)['status'], 'notRegistered')
        self.assertEqual(self.call('library/settle', op)['status'], 'cancelled')
        self.server.close(); self.start()
        self.assertEqual(self.call('library/prepare', late)['status'], 'cancelled')
        prepared, _ = self.library_prepare('settings', '{"version":1,"choice":"revoked"}')
        self.grant_library(roles=('settings',), write=False)
        self.grant_library(roles=('settings',))
        self.assertFalse(self.call('library/operation', {key: prepared[key] for key in ('role', 'operationId')})['commitAllowed'])
        self.library_commit(prepared, status=409)
        self.assertEqual(json.loads((self.root/'.weftext-settings.json').read_text('utf-8'))['choice'], 'first')

    def test_library_schema_install_supplies_original_field_baseline(self):
        self.grant_library(roles=('schema',))
        doc = self.open()
        source = doc['source'].replace('types: ()', 'types: ("task",), status: "open"')
        self.save(doc, source)
        self.policy(lambda users: users['alice']['grants']['Shared'].update(fields={'tasks/status': {'read': True, 'write': True}}))
        before = self.call('fields/read', {'resource': 'Shared', 'fieldId': 'tasks/status'})
        self.assertFalse(before['field']['writable'])
        schema = self.library_read('schema')
        self.assertFalse(schema['installed'])
        plan, _ = self.library_prepare('schema', schema['source'], basis=schema)
        self.assertEqual(self.library_commit(plan)['status'], 'committed')
        field = self.call('fields/read', {'resource': 'Shared', 'fieldId': 'tasks/status'})
        self.assertTrue(field['field']['writable'])
        write = dict(resource='Shared', fieldId='tasks/status', basis=field['basis'],
                     operationId=uid(), value={'kind': 'text', 'value': 'done'})
        self.assertEqual(self.call('fields/write', write)['status'], 'committed')
        current = self.library_read('schema')
        changed = json.loads(current['source'])
        changed['fields'].append({'id': 'acceptance/library', 'key': 'libraryValue',
            'shape': {'kind': 'scalar', 'semantic': 'text'}, 'nullable': False, 'default': {'kind': 'absent'}})
        old = self.call('fields/read', {'resource': 'Shared', 'fieldId': 'tasks/status'})
        update, _ = self.library_prepare('schema', json.dumps(changed), basis=current)
        self.assertEqual(self.library_commit(update)['status'], 'committed')
        # Narrow field bases deliberately ignore unrelated Schema additions.
        self.assertEqual(self.call('fields/write', {**write, 'basis': old['basis'],
                         'operationId': uid()})['status'], 'committed')
        old = self.call('fields/read', {'resource': 'Shared', 'fieldId': 'tasks/status'})
        current = self.library_read('schema')
        changed = json.loads(current['source'])
        next(f for f in changed['fields'] if f['id'] == 'tasks/status')['shape']['values'].append('queued')
        update, _ = self.library_prepare('schema', json.dumps(changed), basis=current)
        self.assertEqual(self.library_commit(update)['status'], 'committed')
        self.call('fields/write', {**write, 'basis': old['basis'], 'operationId': uid(),
                                  'value': {'kind': 'text', 'value': 'open'}}, status=409)
        self.assertIn('status: "done"', (self.root/'Shared/Shared.typ').read_text('utf-8'))

    def test_library_settings_cannot_bypass_style_permission(self):
        self.grant_library()
        self.install_style()
        self.grant_library(roles=('settings',), actor='bob', style_use=False)
        original = self.library_read('settings', 'bob')
        changed = json.loads(original['source'])
        changed['defaultStyle']['enabled'] = False
        self.call('library/prepare', dict(role='settings', basis=original['basis'], operationId=uid(),
                                         source=json.dumps(changed)), 'bob', 404)
        kept = json.loads(original['source'])
        kept['customExtension'] = {'keep': '设置自己的未知数据😀'}
        plan, _ = self.library_prepare('settings', json.dumps(kept, ensure_ascii=False), 'bob', original)
        self.assertEqual(self.library_commit(plan, 'bob')['status'], 'committed')
        self.assertTrue(self.library_read('style')['enabled'])
        plan, _ = self.library_prepare('style', changes={'text.size': '18pt'}, enabled=False)
        self.assertEqual(self.library_commit(plan)['status'], 'committed')
        self.assertEqual(json.loads(self.library_read('settings')['source'])['customExtension'], kept['customExtension'])

    def test_library_style_execution_is_independent_and_frozen_office_stays_original(self):
        self.grant_library()
        self.install_style()
        self.policy(lambda users: users['alice']['grants']['Shared'].update(office={'export': True, 'download': True}))
        doc = self.open()
        original = (self.root/'Shared/Shared.typ').read_bytes()
        rendered = self.call('preview', {'sessionId': doc['sessionId'], 'source': doc['source']})
        self.assertTrue(rendered['ok'], rendered)
        self.assertIn('HOSTED_STYLE_MARKER', visible_html_text(rendered['html']))
        self.assertNotIn('PRIVATE_STYLE_SOURCE_COMMENT', rendered['html'])
        bob = self.open(actor='bob')
        self.call('preview', {'sessionId': bob['sessionId'], 'source': bob['source']}, 'bob', 404)
        self.grant_library(roles=(), actor='bob', style_use=True)
        bob = self.open(actor='bob')
        self.assertTrue(self.call('preview', {'sessionId': bob['sessionId'], 'source': bob['source']}, 'bob')['ok'])
        self.call('library/read', {'role': 'style'}, 'bob', 404)
        req = dict(requestId=uid(), resource='Shared', source=doc['source'], format='docx',
                   reviewMode='none', includeResolved=False, stylePolicy='source', bindings={})
        artifact = self.call('office/export/prepare', req)
        self.install_style(size='20pt', marker='NEW_STYLE_MARKER')
        output = HostedOffice.deliver(self, artifact)
        document = ET.fromstring(HostedOffice.parts(output)['word/document.xml'])
        text = ''.join(node.text or '' for node in document.iter('{http://schemas.openxmlformats.org/wordprocessingml/2006/main}t'))
        self.assertIn('HOSTED_STYLE_MARKER', text)
        self.assertNotIn('NEW_STYLE_MARKER', text)
        doc = self.open()
        newer = self.call('preview', {'sessionId': doc['sessionId'], 'source': doc['source']})
        self.assertIn('NEW_STYLE_MARKER', visible_html_text(newer['html']))
        self.assertEqual((self.root/'Shared/Shared.typ').read_bytes(), original)
        self.grant_library(style_use=False)
        self.assertEqual(self.call('office/export/status', {'requestId': artifact['requestId']})['status'], 'expired')

    def test_library_rules_are_reviewed_and_invalid_input_cannot_replace_original(self):
        self.grant_library(roles=('rules',))
        original = (self.root/'.weftext-rules').read_bytes()
        before = self.library_read('rules')
        for value in ('[]', '{"version":2,"ignored":[],"unmanaged":[]}',
                      '{"version":1,"ignored":["../outside"],"unmanaged":[]}'):
            self.call('library/prepare', dict(role='rules', basis=before['basis'], operationId=uid(), source=value), status=400)
            self.assertEqual((self.root/'.weftext-rules').read_bytes(), original)
        source = '{"version":1,"ignored":["Secret"],"unmanaged":[]}'
        plan, _ = self.library_prepare('rules', source, basis=before)
        self.assertEqual((self.root/'.weftext-rules').read_bytes(), original)
        self.assertEqual(self.library_commit(plan)['status'], 'committed')
        self.assertEqual((self.root/'.weftext-rules').read_text('utf-8'), source)
        self.assertTrue((self.root/'Secret/Secret.typ').exists())

    def test_library_style_query_and_proofreading_use_same_explicit_environment(self):
        self.prepare_query()
        self.grant_library()
        self.install_style()
        self.grant_query(products=('body',))
        # Static fields do not execute the native module or inherit that right.
        static = self.query({'select': [self.column()]})
        self.assertEqual(len(static['result']['rows']), 2)
        product = {'select': [self.column('body', 'body')]}
        self.query(product, fields=[], products=['body'], resources=['Shared'], status=404)
        self.grant_library(roles=(), actor='fields', style_use=True)
        result = self.query(product, fields=[], products=['body'], resources=['Shared'])
        body = result['result']['rows'][0]['cells']['body']['value']['value']
        self.assertIn('HOSTED_STYLE_MARKER', body)
        self.assertNotIn('PRIVATE_STYLE_SOURCE_COMMENT', json.dumps(result))
        self.call('library/read', {'role': 'style'}, 'fields', 404)
        self.call('open', {'resource': 'Shared'}, 'fields', 404)
        self.policy(lambda users: users['alice']['grants']['Shared'].update(
            annotations=dict(read=True, target=True, proofread=True)))
        doc = self.open()
        self.save(doc, doc['source']+'\n错别字\n')
        options = dict(characters=True, typos=True, pairs=True, sensitive=True,
            allowlist=[], selection=None, rules=[dict(id='library.typo', version='1',
            category='typo', text='错别字', message='请核对', replacement='正确字')])
        profile = self.call('review/read', {'resource': 'Shared'})
        scan = self.call('review/proofread', dict(resource='Shared', basis=profile['basis'], options=options))
        self.assertTrue(any(f['ruleId'] == 'library.typo' for f in scan['findings']))
        self.grant_library(style_use=False)
        profile = self.call('review/read', {'resource': 'Shared'})
        self.call('review/proofread', dict(resource='Shared', basis=profile['basis'], options=options), status=404)

    def test_library_style_external_change_fences_inflight_render(self):
        self.grant_library()
        self.install_style()
        doc = self.open()
        source = doc['source']+'\n#for i in range(12000) [style boundary #parbreak()]'
        original = (self.root/'Shared/Shared.typ').read_bytes()
        module = self.root/'Styles/library-style.typ'
        before = module.read_bytes()
        (self.folder/'original-style.typ').write_bytes(before)
        with ThreadPoolExecutor() as pool:
            pending = pool.submit(self.server.call, '/api/hosted/editor/render',
                dict(sessionId=doc['sessionId'], source=source, serial=1), self.tokens['alice'])
            time.sleep(.2)
            module.write_bytes(before+b'\n// external style change\n')
            code, result, _ = pending.result(timeout=40)
        self.assertEqual(code, 409, result)
        self.assertNotIn('tree', result)
        self.assertEqual((self.root/'Shared/Shared.typ').read_bytes(), original)
        self.assertEqual(module.read_bytes(), before+b'\n// external style change\n')

    def test_library_proofread_plan_retains_original_style_dependency_after_restart(self):
        self.grant_library()
        self.install_style()
        self.policy(lambda users: users['alice']['grants']['Shared'].update(
            annotations=dict(read=True, write=True, target=True, proofread=True)))
        doc = self.open()
        self.save(doc, doc['source']+'\n错别字\n')
        options = dict(characters=True, typos=True, pairs=True, sensitive=True,
            allowlist=[], selection=None, rules=[dict(id='library.plan', version='1',
            category='typo', text='错别字', message='请核对', replacement='正确字')])
        profile = self.call('review/read', {'resource': 'Shared'})
        scan = self.call('review/proofread', dict(resource='Shared', basis=profile['basis'], options=options))
        findings = [f['id'] for f in scan['findings'] if f['sourceRange']]
        self.assertTrue(findings)
        request = dict(resource='Shared', basis=profile['basis'], operationId=uid(),
            intent=dict(kind='annotationProofread', options=options, findings=findings,
                        scanId=scan['scanId'], ignore=False))
        plan = self.call('review/prepare', request)
        sidecar = self.root/'Shared/weftext.annotations.json'
        original = sidecar.read_bytes() if sidecar.exists() else None
        self.server.close(); self.start()
        recovered = self.call('review/operation', {'resource': 'Shared', 'operationId': request['operationId']})
        self.assertEqual(recovered['planId'], plan['planId'])
        self.install_style(marker='CHANGED_AFTER_PROOFREAD')
        self.call('review/commit', dict(resource='Shared', operationId=request['operationId'],
                                       planId=plan['planId']), status=409)
        self.assertEqual(sidecar.read_bytes() if sidecar.exists() else None, original)
        self.call('review/settle', {'resource': 'Shared', 'operationId': request['operationId']})

    def test_library_style_execution_cannot_read_private_module_or_ambient_inputs(self):
        self.grant_library()
        source = ('// PRIVATE_STYLE_SOURCE_COMMENT\n'
                  '#let private-value = "STYLE_SECRET_CONSTANT"\n'
                  '#let library-style(body) = body\n')
        plan, _ = self.library_prepare('style', source, enabled=True)
        self.assertEqual(self.library_commit(plan)['status'], 'committed')
        self.grant_library(roles=(), actor='bob', style_use=True)
        doc = self.open(actor='bob')
        for attack in ('#read("/Styles/library-style.typ")',
                       '#import "/Styles/library-style.typ" as styles\n#styles.private-value'):
            rendered = self.call('preview', {'sessionId': doc['sessionId'],
                'source': doc['source']+'\n'+attack}, 'bob')
            self.assertFalse(rendered['ok'], rendered)
            for hidden in ('PRIVATE_STYLE_SOURCE_COMMENT', 'STYLE_SECRET_CONSTANT', str(self.root)):
                self.assertNotIn(hidden, json.dumps(rendered))
        self.assertIn('STYLE_SECRET_CONSTANT', self.library_read('style')['source'])
        # Even the style executor may use only explicit independently readable inputs.
        plan, _ = self.library_prepare('style',
            '#let library-style(body) = { read("../Secret/Secret.typ"); body }\n', enabled=True)
        self.assertEqual(self.library_commit(plan)['status'], 'committed')
        doc = self.open()
        missing = self.call('preview', {'sessionId': doc['sessionId'], 'source': doc['source']})
        self.assertFalse(missing['ok'], missing)
        self.assertNotIn('HIDDEN_PAYLOAD_987', json.dumps(missing))
        self.server.close()
        self.config['styleInputs'] = ['Secret']
        self.start()
        doc = self.open()
        allowed = self.call('preview', {'sessionId': doc['sessionId'], 'source': doc['source']})
        self.assertTrue(allowed['ok'], allowed)
        self.assertIn('HIDDEN_PAYLOAD_987', allowed['html'])
        bob = self.open(actor='bob')
        self.call('preview', {'sessionId': bob['sessionId'], 'source': bob['source']}, 'bob', 404)


if __name__ == '__main__':
    suite = unittest.TestSuite(HostedLibrary(name) for name in
        unittest.defaultTestLoader.getTestCaseNames(HostedLibrary) if name.startswith('test_library_'))
    sys.exit(0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1)
