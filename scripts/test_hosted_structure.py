"""Original hosted structure operations over owned, independently granted trees.

Local APIs only establish the fixture's admissions. All tested effects and new
object entry points use hosted HTTP without rewriting operator configuration.
"""
import base64
import copy
import json
import sys
import unittest

from test_hosted import uid
from test_hosted_history import HostedHistory
from test_hosted_library import visible_html_text
from test_local_features import tiny_png, units


class HostedStructure(HostedHistory):
    def prepare_structure(self):
        self.directory_refs = {}

        def provision(api):
            schema = api('/api/data/schema', {'root': str(self.root)})
            plan = api('/api/data/schema/prepare', {'root': str(self.root), 'source': schema['source'], 'beforeSource': schema['source'], 'beforeRevision': schema['revision'], 'operationId': uid()})
            api('/api/data/schema/install', {'operationId': plan['operationId'], 'planId': plan['planId']})
            for key, directory in [('root', self.root), ('destination', self.root / 'Destination')]:
                if directory != self.root:
                    directory.mkdir()
                prepared = api('/api/navigation/admit-directory', {
                    'root': str(self.root), 'path': str(directory), 'operationId': uid()})
                plan = prepared['preview']
                api('/api/operations/commit', {'operationId': plan['operationId'], 'planId': plan['planId']})
                self.directory_refs[key] = prepared['targetRef']
            self.config['directories'] = {
                'root': {'path': '', 'title': 'Approved root', 'reference': self.directory_refs['root']},
                'destination': {'path': 'Destination', 'title': 'Approved destination', 'reference': self.directory_refs['destination']}}
            slots = {'Created': ('root', 'Created'), 'Copied': ('root', 'Copied'),
                     'Renamed': ('root', 'Renamed'), 'Moved': ('destination', 'Shared'),
                     'Restored': ('root', 'Restored'), 'Template': ('root', 'Template')}
            self.config['structureSlots'] = {key: {'parent': parent, 'name': name} for key, (parent, name) in slots.items()}
            for key, (parent, name) in slots.items():
                prefix = '' if parent == 'root' else 'Destination/'
                self.config['files'][key] = {'path': f'{prefix}{name}/{name}.typ', 'title': key, 'editable': True}
                self.config['users']['alice']['grants'][key] = {'read': True, 'write': True, 'history': True}
            self.config['users']['reader']['grants']['Created'] = {'read': True}

        self.local(provision)
        # Startup configuration never silently enlarges persisted policy.
        # The fixture's operator explicitly grants these future entries.
        def authorize_slots(users):
            for key in self.config['structureSlots']:
                users['alice']['grants'][key] = {'read': True, 'write': True, 'history': True}
            users['reader']['grants']['Created'] = {'read': True}
        self.policy(authorize_slots)

    def grant(self, *, sources=None, destinations=None, actor='alice', schema=False):
        def edit(users):
            users[actor]['structure'] = {'sources': sources or {}, 'destinations': destinations or {}}
            if schema:
                users[actor].setdefault('library', {})['schema'] = {'read': True}
        return self.policy(edit)

    def request(self, kind, resource, **extra):
        return {'resource': resource, 'operationId': uid(), 'kind': kind, **extra}

    def prepare(self, request, actor='alice', status=200):
        result = self.call('structure/prepare', request, actor, status)
        self.no_physical_paths(result)
        return result

    def execute(self, request, actor='alice', accept_losses=False):
        plan = self.prepare(request, actor)
        self.assertEqual(plan['status'], 'prepared', plan)
        self.assertTrue(plan['commitAllowed'], plan)
        result = self.call('structure/commit', {
            'resource': request['resource'], 'operationId': request['operationId'],
            'planId': plan['planId'], 'acceptLosses': accept_losses}, actor)
        self.assertEqual(result['status'], 'committed', result)
        self.no_physical_paths(result)
        return result

    def original(self, request, route='operation', actor='alice', status=200):
        return self.call('structure/' + route, {
            'resource': request['resource'], 'operationId': request['operationId']}, actor, status)

    def test_structure_independent_create_destination_and_post_create_read(self):
        self.prepare_structure()
        request = self.request('create', 'Created', destination='Created', capture={'title': '创建😀', 'body': '#not-code 中文', 'bodyMode': 'text'})
        self.prepare(request, status=404)
        catalog = self.call('structure', actor='reader')
        self.assertEqual(catalog['slots'], [])
        self.grant(destinations={'Created': {'create': True}})
        for code in ['#metadata((types: ("tasks/task",), status: "done")) <weftext>',
                     '#eval("metadata((types: ())) <weftext>", mode: "markup")',
                     '$eval("metadata")$']:
            self.prepare(self.request('create', 'Created', destination='Created',
                                      capture={'body': code, 'bodyMode': 'markup'}), status=400)
        self.assertFalse((self.root / 'Created').exists())
        self.policy(lambda users: users['alice']['grants']['Created'].update(read=False, write=False, history=False))
        result = self.execute(request)
        self.assertFalse(result['result']['canOpen'])
        self.call('open', {'resource': 'Created'}, status=404)
        opened = self.open('Created', 'reader')
        self.assertIn('创建😀', opened['source'])
        rendered = self.call('preview', {'sessionId': opened['sessionId'], 'source': opened['source']}, 'reader')
        self.assertTrue(rendered['ok'], rendered)
        self.assertIn('#not-code 中文', visible_html_text(rendered['html']))
        self.save(opened, opened['source'] + 'forbidden', 'reader', 404)
        self.assertEqual(self.config['files']['Created']['path'], 'Created/Created.typ')

    def test_structure_original_request_restart_and_unknown_settlement(self):
        self.prepare_structure()
        self.grant(destinations={'Created': {'create': True}})
        request = self.request('create', 'Created', destination='Created', capture={'body': '原请求正文'})
        plan = self.prepare(request)
        self.server.close(); self.start()
        replay = self.prepare(request)
        self.assertEqual(replay['planId'], plan['planId'])
        changed = copy.deepcopy(request); changed['capture']['body'] = '换输入'
        self.prepare(changed, status=409)
        result = self.call('structure/commit', {'resource': 'Created', 'operationId': request['operationId'], 'planId': plan['planId']})
        self.assertEqual(result['status'], 'committed')
        source = self.open('Created')['source']
        self.server.close(); self.start()
        self.assertEqual(self.original(request)['status'], 'committed')
        self.assertEqual(self.open('Created')['source'], source)
        unknown = self.request('create', 'Copied', destination='Copied')
        self.grant(destinations={'Created': {'create': True}, 'Copied': {'create': True}})
        settled = self.original(unknown, 'settle')
        self.assertEqual(settled['status'], 'cancelled')
        self.assertEqual(self.prepare(unknown)['status'], 'cancelled')
        self.assertFalse((self.root / 'Copied').exists())

    def test_structure_copy_requires_explicit_tree_read_export_and_fresh_identity(self):
        self.prepare_structure()
        def child(api):
            note = api('/api/note/create', {'root': str(self.root), 'parent': str(self.root / 'Shared'), 'name': 'Child', 'operationId': uid()})
            self.child_source = note['source'] + '\nWHOLE_TREE_OPT_IN_PRIVATE_CHILD'
            api('/api/save', {'sessionId': note['sessionId'], 'baseId': note['baseId'], 'source': self.child_source, 'operationId': uid()})
        self.local(child)
        original = self.open()
        original_ref = self.call('navigation/ref', {'sessionId': original['sessionId']})['reference']
        request = self.request('copy', 'Copied', source='Shared', destination='Copied')
        self.grant(sources={'Shared': {'copy': True}}, destinations={'Copied': {'receive': True}}, schema=True)
        self.prepare(request, status=404)
        self.grant(sources={'Shared': {'copy': True, 'treeRead': True}}, destinations={'Copied': {'receive': True}}, schema=True)
        self.prepare(request, status=404)
        self.grant(sources={'Shared': {'copy': True, 'treeRead': True, 'treeExport': True}}, destinations={'Copied': {'receive': True}}, schema=True)
        result = self.execute(request, accept_losses=True)
        self.assertTrue(result['result']['canOpen'])
        copied = self.open('Copied')
        self.assertNotEqual(self.call('navigation/ref', {'sessionId': copied['sessionId']})['reference'], original_ref)
        self.assertNotIn('WHOLE_TREE_OPT_IN_PRIVATE_CHILD', json.dumps(result))
        self.assertIn('WHOLE_TREE_OPT_IN_PRIVATE_CHILD', (self.root / 'Copied/Child/Child.typ').read_text('utf-8'))
        self.assertEqual(self.open()['source'], original['source'])

    def test_structure_rename_move_keep_original_reference_and_new_entry(self):
        self.prepare_structure()
        before = self.open()
        reference = self.call('navigation/ref', {'sessionId': before['sessionId']})['reference']
        alice_draft = before['source'] + '\nALICE_UNSAVED_BEFORE_MOVE'
        self.call('draft', {'sessionId': before['sessionId'], 'source': alice_draft})
        bob = self.open(actor='bob'); bob_draft = bob['source'] + '\nBOB_PRIVATE_BEFORE_MOVE'
        self.call('draft', {'sessionId': bob['sessionId'], 'source': bob_draft}, 'bob')
        self.policy(lambda users: users['bob']['grants'].update(Moved={'read': True}))
        self.grant(sources={'Shared': {'move': True}}, destinations={'Moved': {'receive': True}})
        result = self.execute(self.request('move', 'Shared', source='Shared', destination='Moved'))
        self.assertTrue(result['result']['canOpen'])
        moved = self.open('Moved')
        self.assertEqual(moved['source'], before['source'])
        self.assertEqual(moved['recoveredDraft'], alice_draft)
        self.assertEqual(self.open('Moved', 'bob')['recoveredDraft'], bob_draft)
        self.assertNotIn('BOB_PRIVATE', json.dumps(moved))
        self.assertEqual(self.call('navigation/ref', {'sessionId': moved['sessionId']})['reference'], reference)
        self.assertFalse((self.root / 'Shared').exists())
        self.server.close(); self.start()
        self.assertEqual(self.open('Moved')['source'], before['source'])
        # Rename uses the approved slot in the same parent.
        self.grant(sources={'Other': {'rename': True}}, destinations={'Renamed': {'receive': True}})
        other = self.open('Other')
        renamed = self.execute(self.request('rename', 'Other', source='Other', destination='Renamed'))
        self.assertTrue(renamed['result']['canOpen'])
        self.assertEqual(self.open('Renamed')['source'], other['source'])

    def test_structure_trash_restore_original_receipt_and_independent_right(self):
        self.prepare_structure()
        source = self.open()['source']
        self.grant(sources={'Shared': {'trash': True, 'restore': True}}, destinations={'Restored': {'receive': True}})
        trash = self.request('trash', 'Shared', source='Shared')
        self.execute(trash)
        self.assertFalse((self.root / 'Shared').exists())
        invalid = self.request('restore', 'Shared', source='Shared', destination='Restored', trashOperationId=uid())
        self.prepare(invalid, status=404)
        restored = self.execute(self.request('restore', 'Shared', source='Shared', destination='Restored', trashOperationId=trash['operationId']))
        self.assertTrue(restored['result']['canOpen'])
        self.assertEqual(self.open('Restored')['source'], source)
        self.assertEqual(self.original(trash)['status'], 'committed')

    def test_structure_stale_target_never_overwrites_an_independent_object(self):
        self.prepare_structure()
        # A legitimate local Core birth is still not the original hosted
        # producer authorized for a future destination slot.
        self.local(lambda api: api('/api/note/create', {
            'root': str(self.root), 'parent': str(self.root), 'name': 'Copied', 'operationId': uid()}))
        self.call('open', {'resource': 'Copied'}, status=404)
        self.grant(destinations={'Created': {'create': True}})
        request = self.request('create', 'Created', destination='Created')
        plan = self.prepare(request)
        occupied = self.root / 'Created'; occupied.mkdir()
        raw = occupied / 'Created.typ'; raw.write_text('INDEPENDENT_EXTERNAL_OCCUPANT', encoding='utf-8')
        self.call('structure/commit', {'resource': 'Created', 'operationId': request['operationId'], 'planId': plan['planId']}, status=409)
        self.assertEqual(raw.read_text('utf-8'), 'INDEPENDENT_EXTERNAL_OCCUPANT')
        self.call('open', {'resource': 'Created'}, status=404)
        other = self.open('Other'); self.save(other, other['source'] + '\nUNRELATED_CONTINUES')

    def test_structure_revocation_preserves_draft_original_plan_and_other_note(self):
        self.prepare_structure()
        self.grant(sources={'Shared': {'rename': True}}, destinations={'Renamed': {'receive': True}})
        doc = self.open(); draft = doc['source'] + '\nPRIVATE_CURRENT_DRAFT'
        self.call('draft', {'sessionId': doc['sessionId'], 'source': draft})
        request = self.request('rename', 'Shared', source='Shared', destination='Renamed')
        plan = self.prepare(request)
        self.policy(lambda users: users['alice']['structure']['sources']['Shared'].update(rename=False))
        self.call('structure/commit', {'resource': 'Shared', 'operationId': request['operationId'], 'planId': plan['planId']}, status=404)
        self.assertTrue((self.root / 'Shared/Shared.typ').exists())
        self.assertEqual(self.open()['recoveredDraft'], draft)
        other = self.open('Other', 'bob'); self.save(other, other['source'] + '\nBOB_INDEPENDENT', 'bob')

    def test_structure_builtin_template_is_native_safe_and_slot_bound(self):
        self.prepare_structure()
        self.grant(destinations={'Template': {'create': True}})
        request = self.request('template', 'Template', destination='Template', template='native-note', parameters={'title': '#panic("not executed")😀', 'body': '模板正文'})
        result = self.execute(request, accept_losses=True)
        self.assertTrue(result['result']['canOpen'])
        doc = self.open('Template')
        self.assertIn('模板正文', doc['source'])
        rendered = self.call('preview', {'sessionId': doc['sessionId'], 'source': doc['source']})
        self.assertTrue(rendered['ok'], rendered)
        self.assertIn('#panic("not executed")😀', visible_html_text(rendered['html']))

    def test_structure_typed_capture_requires_schema_type_and_field_authority(self):
        self.prepare_structure()
        self.config['structureSlots']['Created']['types'] = ['tasks/task']
        self.server.close(); self.start()
        self.grant(destinations={'Created': {'create': True}})
        request = self.request('create', 'Created', destination='Created', capture={
            'title': '有类型任务', 'body': '当前字段与正文', 'types': ['tasks/task'],
            'fields': {'tasks/status': {'kind': 'text', 'value': 'open'}}})
        self.prepare(request, status=404)
        self.grant(destinations={'Created': {'create': True}}, schema=True)
        self.prepare(request, status=404)
        self.policy(lambda users: users['alice']['grants']['Created'].update(
            fields={'tasks/status': {'read': True, 'write': True}}))
        result = self.execute(request)
        self.assertTrue(result['result']['canOpen'])
        profile = self.call('fields/read', {'resource': 'Created', 'fieldId': 'tasks/status'})
        self.assertEqual(profile['field']['cell']['value'], {'kind': 'text', 'value': 'open'})
        self.assertIn('当前字段与正文', self.open('Created')['source'])

    def test_structure_registered_template_copies_resources_into_pregranted_collection(self):
        self.prepare_structure()
        def provision(api):
            doc = api('/api/open', {'root': str(self.root), 'path': str(self.root / 'Shared/Shared.typ')})
            source = doc['source'] + '\n模板参数槽😀\n#image("模板资源.png", width: 20pt)\n'
            api('/api/save', {'sessionId': doc['sessionId'], 'baseId': doc['baseId'],
                             'source': source, 'operationId': uid()})
            doc = api('/api/open', {'root': str(self.root), 'path': doc['path']})
            resource = api('/api/resources/prepare-import', {'sessionId': doc['sessionId'],
                'name': '模板资源.png', 'bytes': base64.b64encode(tiny_png()).decode(), 'operationId': uid()})
            api('/api/operations/commit', {'operationId': resource['operationId'], 'planId': resource['planId']})
            context = dict(sessionId=doc['sessionId'], baseId=doc['baseId'], source=doc['source'],
                serial=0, path=doc['path'], root=str(self.root), nodeId=doc['nodeId'],
                workspaceId=doc['workspaceId'], dirty=False)
            marker = '模板参数槽😀'; start = source.index(marker)
            registration = api('/api/knowledge/prepare', {'operationId': uid(), 'context': context,
                'intent': {'kind': 'templateRegister', 'title': '准确原生模板', 'parameters': [{
                    'name': 'body', 'kind': 'markup', 'from': units(source[:start]),
                    'to': units(source[:start + len(marker)])}]}})
            api('/api/operations/commit', {'operationId': registration['operationId'],
                                         'planId': registration['planId'], 'context': context})
            self.config['structureTemplates'] = {'registered': {'source': 'Shared',
                'templateId': registration['preview']['templateAfter']['id']}}
            self.config['resourceCollections'] = {'template-assets': {
                'owner': 'Template', 'fromStructureSlot': True, 'title': 'Created template assets'}}
            self.config['files']['Template']['inputs'] = ['template-assets/*']
        self.local(provision)
        self.grant(sources={'Shared': {'template': True, 'treeRead': True, 'treeExport': True}},
                   destinations={'Template': {'receive': True}}, schema=True)
        self.policy(lambda users: users['alice'].update(resourceCollections={
            'template-assets': {'read': True, 'download': True}}))
        catalog = self.call('structure')
        self.assertEqual(next(t for t in catalog['templates'] if t['id'] == 'registered')['status'], 'ready')
        request = self.request('template', 'Template', source='Shared', destination='Template',
                               template='registered', parameters={'body': '#panic("literal")🌿'})
        self.execute(request, accept_losses=True)
        doc = self.open('Template')
        self.assertIn('literal', doc['source'])
        self.assertNotIn('模板参数槽', doc['source'])
        rendered = self.call('preview', {'sessionId': doc['sessionId'], 'source': doc['source']})
        self.assertTrue(rendered['ok'], rendered)
        catalog = self.call('media')
        collection = next(c for c in catalog['collections'] if c['collection'] == 'template-assets')
        self.assertEqual(collection['status'], 'ready')
        self.assertTrue(any(a['asset'].startswith('template-assets/') for a in catalog['resources']))
        self.assertEqual((self.root / 'Template/模板资源.png').read_bytes(), tiny_png())
        self.assertIn('模板参数槽', self.open()['source'])
        self.assertFalse(any(a['asset'].startswith('template-assets/') for a in self.call('media', actor='reader')['resources']))

    def test_structure_parent_namespace_does_not_grant_reference_content(self):
        self.prepare_structure()
        doc = self.open()
        link = '#link("../Secret/Secret.typ")[作者原有的链接]'
        self.save(doc, doc['source'] + '\n' + link)
        original = self.docs['Secret']
        reference = f"weftext-node://{original['workspaceId']}/{original['nodeId']}"
        self.server.close()
        self.config['directories']['secret-parent'] = {
            'path': 'Secret', 'reference': reference, 'title': 'Authorized destination only'}
        for key in ['ParentCopy', 'WithInput']:
            self.config['structureSlots'][key] = {'parent': 'secret-parent', 'name': key}
            self.config['files'][key] = {'path': f'Secret/{key}/{key}.typ', 'title': key, 'editable': True}
        self.start()
        def policy(users):
            users['alice']['grants']['Secret'] = {'read': False, 'write': False, 'history': False, 'download': False}
            for key in ['ParentCopy', 'WithInput']:
                users['alice']['grants'][key] = {'read': True}
        self.policy(policy)
        self.grant(sources={'Shared': {'copy': True, 'treeRead': True, 'treeExport': True}},
            destinations={'ParentCopy': {'receive': True}, 'WithInput': {'receive': True}}, schema=True)
        request = self.request('copy', 'ParentCopy', source='Shared', destination='ParentCopy')
        plan = self.prepare(request)
        self.assertTrue(any(loss['code'] == 'structure-copy-loss' for loss in plan['preview']['losses']))
        self.assertNotIn('HIDDEN_PAYLOAD_987', json.dumps(plan))
        result = self.call('structure/commit', {'resource': 'ParentCopy', 'operationId': request['operationId'],
            'planId': plan['planId'], 'acceptLosses': True})
        self.assertEqual(result['status'], 'committed')
        self.assertIn(link, self.open('ParentCopy')['source'])
        denied = self.request('copy', 'WithInput', source='Shared', destination='WithInput', inputs=['Secret'])
        self.prepare(denied, status=404)
        self.policy(lambda users: users['alice']['grants']['Secret'].update(read=True))
        self.execute(denied, accept_losses=True)
        copied = self.open('WithInput')['source']
        self.assertIn(reference, copied)
        self.assertNotIn('../Secret/Secret.typ', copied)


if __name__ == '__main__':
    selected = set(sys.argv[2:])
    suite = unittest.TestSuite(HostedStructure(name) for name in sorted(HostedStructure.__dict__)
                              if name.startswith('test_structure_') and (not selected or name in selected))
    sys.exit(not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful())
