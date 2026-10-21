"""Actual advanced hosted Query authority, native definitions and frozen output.

Uses owned disposable libraries and real HTTP/Core commits. No desktop input.
The local process is used only to provision admitted fixtures while the hosted
process is stopped; consumer requests always use the hosted boundary.
"""
import copy
import json
import sys
import time
import unittest
from concurrent.futures import ThreadPoolExecutor
from test_hosted import Server, digest, uid
from test_hosted_fields import HostedFields
from test_hosted_office import HostedOffice


class HostedQuery(HostedFields):
    def prepare_query(self):
        self.prepare_fields()
        for name, status in [('Shared', 'open'), ('Other', 'done'), ('Secret', 'open')]:
            doc = self.open(name)
            node = self.docs[name]['nodeId']
            source = (f'#metadata((id: "{node}", types: ("task",), status: "{status}", '
                      f'private: "PRIVATE_{name}_UNREADABLE")) <weftext>\n'
                      f'#set document(title: [文档{name}])\n#title()\n'
                      f'= 标题{name}😀\n\n正文{name}🌿\n')
            self.save(doc, source)

    def local(self, action):
        self.server.close()
        local = Server(['--no-browser', '--data-dir', str(self.data)])
        def api(route, data=None):
            code, result, _ = local.call(route, data)
            self.assertEqual(code, 200, str(result)[:1500])
            return result
        try:
            return action(api)
        finally:
            local.close()
            self.start()

    def add_definition(self, plan, name='Other', label='weftext-query-test', key=None):
        def create(api):
            doc = api('/api/open', {'path': str(self.root/name/(name+'.typ')), 'root': str(self.root)})
            if doc.get('recoveredDraftConflict'):
                api('/api/draft/resolve', {'sessionId': doc['sessionId'], 'action': 'mergeRecovered', 'source': doc['source']})
            return api('/api/query/definition/save', {
                'sessionId': doc['sessionId'], 'baseId': doc['baseId'],
                'source': doc['source'], 'sourceHash': digest(doc['source']), 'serial': 1,
                'operationId': uid(), 'key': key, 'label': label, 'plan': plan})
        return self.local(create)

    @staticmethod
    def column(expr='status', name='status'):
        return {'name': name, 'expr': expr}

    def query(self, plan, actor='fields', fields=None, status=200, **extra):
        return self.call('query', {'fields': ['tasks/status'] if fields is None else fields,
                                  'plan': plan, **extra}, actor, status)

    def grant_query(self, actor='fields', resources=('Shared', 'Other'), products=(),
                    definitions=False, history=False, relations=()):
        rights = {'definitions': definitions, 'definitionHistory': history,
                  'products': {name: True for name in products}, 'relations': list(relations)}
        self.policy(lambda users: [users[actor]['grants'].setdefault(resource, {}).update(query=copy.deepcopy(rights))
                                  for resource in resources])

    def test_query_parameters_union_distinct_and_static_denial(self):
        self.prepare_query()
        base = {'select': [self.column()], 'where': 'status == wanted',
                'advanced': {'parameters': [{'name': 'wanted',
                    'shape': {'kind': 'scalar', 'semantic': 'text'}}]}}
        result = self.query(base, parameters={'wanted': {'kind': 'text', 'value': 'open'}})
        self.assertEqual([row['entityKey'] for row in result['result']['rows']], ['Shared'])
        self.assertEqual(result['columns'], ['status'])
        self.query(base, parameters={'wanted': {'kind': 'int', 'value': '1'}}, status=400)
        self.query(base, parameters={}, status=400)
        self.query(base, parameters={'wanted': {'kind': 'text', 'value': 'open'},
                                     'extra': {'kind': 'text', 'value': 'unused'}}, status=400)
        branch = {'select': [self.column()], 'where': 'status == "open"'}
        union = {'advanced': {'unionAll': [branch, copy.deepcopy(branch)]}}
        rows = self.query(union)['result']['rows']
        self.assertEqual(len(rows), 2)
        self.assertNotEqual(rows[0]['key'], rows[1]['key'])
        self.assertEqual([row['cells']['status']['value']['value'] for row in rows], ['open', 'open'])
        union['advanced']['distinct'] = [self.column('status', 'uniqueStatus')]
        distinct = self.query(union)['result']
        self.assertEqual(len(distinct['rows']), 1)
        self.assertTrue(distinct['rows'][0]['readOnly'])
        self.assertIsNone(distinct['rows'][0]['entityKey'])
        # A producer column named body is a terminal number, not a request to
        # compile private document bodies through a misleading name match.
        literal = {'select': [self.column('1', 'body')]}
        terminal = self.query({'where': 'body == 1',
                               'advanced': {'unionAll': [literal, copy.deepcopy(literal)]}})['result']
        self.assertEqual(len(terminal['rows']), 4)
        self.assertTrue(all(row['cells']['body']['value'] == {'kind': 'int', 'value': '1'}
                            for row in terminal['rows']))
        incompatible = {'advanced': {'unionAll': [branch, {'select': [self.column('1')]}]}}
        self.query(incompatible, status=400)
        for expr in ['due', 'field("invented/hidden")', 'field("core/aliases")']:
            denied = self.query({'advanced': {'unionAll': [branch, {'select': [self.column(expr)]}]}}, status=400)
            self.assertNotIn('aliases', json.dumps(denied))
        for hidden in ['PRIVATE_', 'Secret', '正文', str(self.root)]:
            self.assertNotIn(hidden, json.dumps(result, ensure_ascii=False))

    def test_query_products_are_independent_and_do_not_deliver_source(self):
        self.prepare_query()
        plan = {'select': [self.column('title', 'title'), self.column('body', 'body'),
                           self.column('headings', 'headings')]}
        requested = ['title', 'body', 'headings']
        # Existing source read is not a grant for this separate product consumer.
        self.query(plan, actor='alice', fields=[], products=requested, resources=['Shared'], status=404)
        self.grant_query(products=requested)
        output = self.query(plan, fields=[], products=requested)
        self.assertEqual(output['columns'], requested)
        rows = {row['entityKey']: row for row in output['result']['rows']}
        self.assertEqual(set(rows), {'Shared', 'Other'})
        for resource, row in rows.items():
            self.assertEqual(row['cells']['title']['value']['value'], '文档'+resource)
            self.assertIn('正文'+resource+'🌿', row['cells']['body']['value']['value'])
            self.assertIn('标题'+resource, json.dumps(row['cells']['headings'], ensure_ascii=False))
        for hidden in ['PRIVATE_', 'Secret', str(self.root), '#metadata']:
            self.assertNotIn(hidden, json.dumps(output, ensure_ascii=False))
        self.call('open', {'resource': 'Shared'}, 'fields', 404)
        self.call('download/prepare', {'resource': 'Shared'}, 'fields', 404)
        self.policy(lambda users: users['fields']['grants']['Shared']['query']['products'].update(body=False))
        self.query(plan, fields=[], products=requested, resources=['Shared'], status=404)
        # A requested empty scope still cannot type-check an ungranted product.
        self.policy(lambda users: users['fields']['grants']['Other']['query']['products'].update(body=False))
        self.query(plan, fields=[], products=requested, resources=[], status=404)

    def test_query_saved_current_pinned_calls_need_exact_definition_authority(self):
        self.prepare_query()
        plan = {'where': 'status == "open"', 'select': [self.column()]}
        first = self.add_definition(plan)
        key = first['definition']['key']
        revision = first['definitionRevision']
        call = {'advanced': {'call': {'owner': 'Other', 'selection': {'mode': 'current', 'key': key}}}}
        self.query(call, status=404)
        self.grant_query(definitions=True)
        catalog = self.call('query/catalog', actor='fields')
        self.assertEqual({item['resource'] for item in catalog['resources']}, {'Shared', 'Other'})
        self.assertEqual([item['key'] for item in catalog['definitions']], [key])
        for hidden in ['PRIVATE_', '正文', 'Secret', str(self.root)]:
            self.assertNotIn(hidden, json.dumps(catalog, ensure_ascii=False))
        listed = self.call('query/definitions', {'resource': 'Other'}, 'fields')
        self.assertEqual([item['key'] for item in listed['definitions']], [key])
        before = self.query(call)
        self.assertEqual([row['entityKey'] for row in before['result']['rows']], ['Shared'])
        workspace = first['definition']['binding']['schema'].split('://', 1)[1].split('/', 1)[0]
        nested_plan = copy.deepcopy(call)
        nested_plan['advanced']['call']['owner'] = f"weftext-node://{workspace}/{self.docs['Other']['nodeId']}"
        nested = self.add_definition(nested_plan, name='Shared', label='weftext-query-nested')
        nested_call = {'advanced': {'call': {'owner': 'Shared', 'selection': {
            'mode': 'current', 'key': nested['definition']['key']}}}}
        # The called definition owner is outside the explicit data row scope.
        self.assertEqual([row['entityKey'] for row in self.query(nested_call, resources=['Shared'])['result']['rows']], ['Shared'])
        self.call('open', {'resource': 'Other'}, 'fields', 404)
        for hidden in ['PRIVATE_', '正文', 'Secret', str(self.root)]:
            self.assertNotIn(hidden, json.dumps(before, ensure_ascii=False))
        changed = {**plan, 'where': 'status == "done"'}
        self.add_definition(changed, key=key)
        self.assertEqual([row['entityKey'] for row in self.query(call)['result']['rows']], ['Other'])
        pinned = copy.deepcopy(call)
        pinned['advanced']['call']['selection'] = {'mode': 'pinned', 'key': key, 'revision': revision}
        self.query(pinned, status=404)
        self.grant_query(definitions=True, history=True)
        selected = self.call('query/definitions', {'resource': 'Other',
            'selection': {'mode': 'pinned', 'key': key, 'revision': revision}}, 'fields')
        self.assertEqual(selected['ownerRevision'], revision)
        self.assertEqual([row['entityKey'] for row in self.query(pinned)['result']['rows']], ['Shared'])
        self.server.close(); self.start()
        self.assertEqual([row['entityKey'] for row in self.query(pinned)['result']['rows']], ['Shared'])
        wrong = copy.deepcopy(pinned)
        wrong['advanced']['call']['selection']['revision'] = uid()
        self.query(wrong, status=409)
        hidden = copy.deepcopy(call)
        hidden['advanced']['call']['owner'] = 'Secret'
        missing = copy.deepcopy(hidden)
        missing['advanced']['call']['owner'] = 'NotThere'
        self.assertEqual(self.query(hidden, status=404), self.query(missing, status=404))

    def test_query_related_uses_granted_fields_types_identity_and_actual_refs(self):
        self.prepare_query()
        library = self.local(lambda api: api('/api/library/open', {'path': str(self.root)}))
        refs = {name: f"weftext-node://{library['workspaceId']}/{doc['nodeId']}"
                for name, doc in self.docs.items()}
        for name in ['Shared', 'Other', 'Secret']:
            doc = self.open(name)
            parent = f', parents: ((target: (ref: "{refs["Other"]}"),),)' if name == 'Shared' else ''
            if name == 'Secret': parent = ', parents: ((target: (text: "Other"),),)'
            source = (f'#metadata((id: "{self.docs[name]["nodeId"]}", types: ("person",)'
                      f'{parent})) <weftext>\nPRIVATE_BODY_{name}')
            self.save(doc, source)
        installed = json.loads((self.root/'.weftext-schema.json').read_text('utf-8'))
        relation = next(item['id'] for item in installed['relations'] if item['field'] == 'people/parents')
        self.policy(lambda users: [users['fields']['grants'][name]['fields'].update(
            {'people/parents': {'read': True}}) for name in ['Shared', 'Other']])
        self.grant_query(products=['self', 'types'], relations=[relation])
        seed = {'where': 'self == origin', 'advanced': {'parameters': [{
            'name': 'origin', 'shape': {'kind': 'scalar', 'semantic': 'node_ref'},
            'default': {'kind': 'text', 'value': refs['Shared']}}]}}
        plan = {'select': [self.column('self', 'self')], 'advanced': {'related': {
            'relation': relation, 'from': seed, 'direction': 'forward'}}}
        request = dict(fields=['people/parents'], products=['self', 'types'], relations=[relation])
        forward = self.query(plan, **request)['result']
        self.assertEqual([row['entityKey'] for row in forward['rows']], ['Other'])
        self.assertEqual(forward['rows'][0]['cells']['self']['value']['value'], refs['Other'])
        inverse = copy.deepcopy(plan)
        inverse['advanced']['related']['direction'] = 'inverse'
        inverse['advanced']['related']['from']['advanced']['parameters'][0]['default']['value'] = refs['Other']
        self.assertEqual([row['entityKey'] for row in self.query(inverse, **request)['result']['rows']], ['Shared'])
        self.policy(lambda users: users['fields']['grants']['Shared']['query'].update(relations=[]))
        self.query(plan, resources=['Shared', 'Other'], status=404, **request)
        for hidden in ['PRIVATE_BODY_', 'Secret', str(self.root)]:
            self.assertNotIn(hidden, json.dumps(forward, ensure_ascii=False))

    def test_query_catalog_is_partial_when_one_definition_owner_is_unavailable(self):
        self.prepare_query()
        saved = self.add_definition({'select': [self.column()]})
        self.grant_query(definitions=True)
        self.assertEqual(self.call('query/catalog', actor='fields')['coverage'], 'complete')
        path = self.root/'Other/Other.typ'
        external = path.read_text('utf-8') + '\nEXTERNAL_UNADMITTED_DEFINITION_OWNER'
        path.write_text(external, 'utf-8')
        catalog = self.call('query/catalog', actor='fields')
        self.assertEqual(catalog['coverage'], 'partial')
        self.assertEqual(catalog['unavailableDefinitions'], [{'resource': 'Other', 'status': 'unavailable'}])
        self.assertNotIn(saved['definition']['key'], json.dumps(catalog))
        self.assertNotIn('EXTERNAL_UNADMITTED', json.dumps(catalog))
        self.assertEqual([row['entityKey'] for row in self.query(
            {'select': [self.column()]}, resources=['Shared'])['result']['rows']], ['Shared'])
        self.call('query/definitions', {'resource': 'Other'}, 'fields', 409)
        self.assertEqual(path.read_text('utf-8'), external)

    def test_query_embedded_exact_draft_proposals_preserve_source_and_personal_input(self):
        self.prepare_query()
        definition = self.add_definition({'where': 'status == "open"', 'select': [self.column()]})
        self.policy(lambda users: [users['alice']['grants'][name].update(fields={'tasks/status': {'read': True}})
                                  for name in ['Shared', 'Other']])
        self.grant_query(actor='alice', definitions=True)
        doc = self.open('Other')
        draft = doc['source'] + '\nPERSONAL_QUERY_DRAFT😀'
        snapshot = {'sessionId': doc['sessionId'], 'source': draft, 'serial': 4, 'sourceHash': digest(draft)}
        original = (self.root/'Other/Other.typ').read_bytes()
        if doc.get('recoveredDraftConflict'):
            self.call('draft/resolve', {'sessionId': doc['sessionId'], 'source': draft})
        self.call('draft', {'sessionId': doc['sessionId'], 'source': draft})
        found = self.call('query/embedded/discover', snapshot)
        occurrence = next(item for item in found['occurrences'] if item['key'] == definition['definition']['key'])
        request = {**snapshot, 'occurrenceId': occurrence['id']}
        rendered = self.call('editor/render', snapshot)
        self.assertTrue(rendered['ok'], rendered)
        def tags(tree):
            if tree.get('kind') == 'tag':
                yield tree
            for child in tree.get('children', []):
                yield from tags(child)
        query_tags = [tag for tag in tags(rendered['tree'])
                      if tag.get('attrs', {}).get('data-weftext-query-label')]
        self.assertTrue(query_tags, 'Native Query carriers must remain mountable in the hosted projection')
        for tag in tags(rendered['tree']):
            self.assertTrue(set(tag.get('attrs', {})) <= {
                'data-weftext-query-label', 'data-weftext-query-occurrence'})
        output = self.call('query/embedded/run', request)
        self.assertTrue(output['draftPreview'])
        self.assertEqual([row['entityKey'] for row in output['result']['rows']], ['Shared'])
        changed = copy.deepcopy(occurrence['plan']); changed['where'] = 'status == "done"'
        proposal = self.call('query/embedded/edit', {**request, 'plan': changed})
        self.assertIn('PERSONAL_QUERY_DRAFT😀', proposal['source'])
        self.assertEqual((self.root/'Other/Other.typ').read_bytes(), original)
        self.assertIn('PERSONAL_QUERY_DRAFT😀', self.open('Other')['recoveredDraft'])
        # An equal-byte return to the old draft remains a newer serial, so the
        # old range cannot silently reacquire a writer after a newer discovery.
        self.call('query/embedded/discover', {**snapshot, 'serial': 5, 'source': draft+'new', 'sourceHash': digest(draft+'new')})
        self.call('query/embedded/discover', {**snapshot, 'serial': 6})
        self.call('query/embedded/edit', {**request, 'plan': changed}, status=409)
        self.call('query/embedded/discover', snapshot, actor='bob', status=410)
        # Source-only readers may inspect their source but cannot execute its
        # data dependencies or acquire any edit entry.
        self.policy(lambda users: users['reader']['grants'].update(Other={'read': True}))
        reader = self.open('Other', actor='reader')
        readonly = {'sessionId': reader['sessionId'], 'source': reader['source'], 'serial': 1}
        viewed = self.call('query/embedded/discover', readonly, actor='reader')
        item = viewed['occurrences'][0]
        self.assertFalse(item['writable']); self.assertFalse(item['runnable'])
        self.call('query/embedded/edit', {**readonly, 'occurrenceId': item['id'], 'replacement': item['range']['raw']}, 'reader', 404)
        self.call('query/embedded/run', {**readonly, 'occurrenceId': item['id']}, 'reader', 404)

    def test_query_late_semantic_run_rechecks_serial_and_revoked_authority(self):
        self.prepare_query()
        definition = self.add_definition({'select': [self.column('body', 'body')]})
        self.grant_query(actor='alice', definitions=True, products=['body'])
        doc = self.open('Other')
        slow = doc['source'] + '\n#for i in range(4000) [delayed paragraph #parbreak()]'
        basis = {'sessionId': doc['sessionId'], 'source': slow, 'serial': 1}
        found = self.call('query/embedded/discover', basis)
        self.assertTrue(self.call('editor/render', basis)['ok'])
        occurrence = next(item for item in found['occurrences'] if item['key'] == definition['definition']['key'])
        request = {**basis, 'occurrenceId': occurrence['id']}
        with ThreadPoolExecutor() as pool:
            pending = pool.submit(self.server.call, '/api/hosted/query/embedded/run', request, self.tokens['alice'])
            time.sleep(.2)
            self.call('editor/map', {**basis, 'source': slow+'\nNEWER_DRAFT', 'serial': 2})
            code, result, _ = pending.result(timeout=40)
            self.assertEqual(code, 409, result)
            self.assertNotIn('queryId', result)
        with ThreadPoolExecutor() as pool:
            pending = pool.submit(self.server.call, '/api/hosted/query/embedded/run',
                                  {**request, 'serial': 3}, self.tokens['alice'])
            time.sleep(.2)
            self.policy(lambda users: users['alice']['grants']['Other']['query']['products'].update(body=False))
            code, result, _ = pending.result(timeout=40)
            self.assertEqual(code, 401, result)
            self.assertEqual(result['error']['code'], 'authentication')
            self.assertNotIn('queryId', result)

    def test_query_frozen_semantic_output_and_every_consumer_revocation(self):
        self.prepare_query()
        self.grant_query(products=['body'])
        self.policy(lambda users: [grant.update(office={'export': True, 'download': True})
                                  for grant in users['fields']['grants'].values()])
        result = self.query({'select': [self.column('body', 'body')]}, fields=[], products=['body'])
        request = {'requestId': uid(), 'format': 'xlsx', 'reviewMode': 'none',
                   'includeResolved': False, 'stylePolicy': 'template', 'bindings': {}, 'queryId': result['queryId']}
        doc = self.open()
        self.save(doc, doc['source'].replace('正文Shared🌿', 'NEWER_BODY_AFTER_QUERY'))
        output, _ = HostedOffice.output(self, request, 'fields')
        workbook = HostedOffice.deliver(self, output, 'fields')
        xml = '\n'.join(HostedOffice.parts(workbook).values())
        self.assertIn('正文Shared🌿', xml)
        self.assertNotIn('NEWER_BODY_AFTER_QUERY', xml)
        self.assertNotIn('PRIVATE_', xml)
        self.policy(lambda users: users['fields']['grants']['Shared']['query']['products'].update(body=False))
        self.assertEqual(self.call('office/export/status', {'requestId': request['requestId']}, 'fields')['status'], 'expired')
        self.call('office/export/chunk', {'requestId': request['requestId'], 'offset': 0}, 'fields', 409)
        self.call('office/export/prepare', {**request, 'requestId': uid()}, 'fields', 410)
        self.grant_query(products=['body'])
        self.assertEqual(self.call('office/export/status', {'requestId': request['requestId']}, 'fields')['status'], 'expired')

    def test_query_capacity_expires_only_own_old_handles_and_keeps_prepared_output(self):
        self.prepare_query()
        self.policy(lambda users: [grant.update(office={'export': True, 'download': True})
                                  for actor in ['fields', 'fieldreader']
                                  for grant in users[actor]['grants'].values()])
        reader = self.query({'select': [self.column()]}, actor='fieldreader')
        first = self.query({'where': 'status == "open"', 'select': [self.column()]})
        request = {'requestId': uid(), 'format': 'xlsx', 'reviewMode': 'none',
                   'includeResolved': False, 'stylePolicy': 'template', 'bindings': {},
                   'queryId': first['queryId']}
        frozen, _ = HostedOffice.output(self, request, 'fields')
        latest = None
        for index in range(10):
            latest = self.query({'select': [self.column(str(index), 'ordinal')]})
        active = self.call('office', actor='fields')['queries']
        self.assertEqual(len(active), 8)
        self.assertNotIn(first['queryId'], [item['queryId'] for item in active])
        self.assertIn(latest['queryId'], [item['queryId'] for item in active])
        self.call('office/export/prepare', {**request, 'requestId': uid()}, 'fields', 410)
        self.assertIn(reader['queryId'], [item['queryId'] for item in self.call('office', actor='fieldreader')['queries']])
        # Expiring a Query handle cannot replace or invalidate an already
        # prepared output's exact bytes. Delivery still rechecks its authority.
        self.assertEqual(self.call('office/export/status', {'requestId': request['requestId']}, 'fields')['digest'], frozen['digest'])
        xml = '\n'.join(HostedOffice.parts(HostedOffice.deliver(self, frozen, 'fields')).values())
        self.assertIn('open', xml)
        self.assertNotIn('ordinal', xml)
        self.query({'take': 501}, status=400)
        self.assertEqual({item['queryId'] for item in self.call('office', actor='fields')['queries']},
                         {item['queryId'] for item in active})

    def test_query_tag_structure_static_zero_scope_stale_frozen_output_and_revocation(self):
        self.prepare_query()
        self.policy(lambda users: users['fields'].update(navigation={'tags': {
            'read': True, 'create': True, 'rename': True, 'move': True}}))

        def tag_action(kind, **arguments):
            basis = self.call('navigation/registry', actor='fields')['tags']['basis']
            prepared = self.call('navigation/registry/prepare', {'operationId': uid(),
                'intent': {'kind': kind, 'basis': basis, **arguments}}, 'fields')
            committed = self.call('navigation/registry/commit', {key: prepared[key]
                for key in ['operationId', 'planId']}, 'fields')
            return committed['navigationRef']

        root = tag_action('createTag', displayName='same/path', parentRef=None)
        leaf = tag_action('createTag', displayName='same/path', parentRef=root)
        predicate = f'tagInSubtree(tagRef("{leaf}"), tagRef("{root}"))'
        structural = {'where': predicate, 'select': [self.column()]}
        exact = {'where': f'tagRef("{leaf}") == tagRef("{root}")', 'select': [self.column()]}
        self.query(structural, actor='fieldreader', status=404, resources=[])
        self.assertEqual(self.query(exact, actor='fieldreader')['result']['rows'], [])
        zero = self.query(structural, resources=[])['result']
        self.assertEqual(zero['rows'], [])
        self.assertEqual(zero['coverage'], 'complete')
        missing = f'weftext-tag://{root.split("/")[-2]}/{uid()}'
        self.query({'where': f'true || !tagInSubtree(tagRef("{leaf}"), tagRef("{missing}"))',
                    'select': [self.column()]}, resources=[], status=400)
        displayed = self.query(structural, resources=['Shared'])
        self.assertEqual(len(displayed['result']['rows']), 1)
        self.assertEqual(displayed['result']['freshness'], 'current')
        tag_action('moveTag', reference=leaf, parentRef=None)
        self.assertEqual(self.query(structural, resources=['Shared'])['result']['rows'], [])
        stale = next(row for row in self.call('office', actor='fields')['queries']
                     if row['queryId'] == displayed['queryId'])
        self.assertEqual(stale['freshness'], 'stale')
        self.assertEqual(stale['rowCount'], 1)
        self.policy(lambda users: [grant.update(office={'export': True, 'download': True})
                                  for grant in users['fields']['grants'].values()])
        # Policy generation changes invalidate old handles; repeat under the
        # current epoch, then verify a new tree change keeps the displayed cut.
        tag_action('moveTag', reference=leaf, parentRef=root)
        displayed = self.query(structural, resources=['Shared'])
        tag_action('moveTag', reference=leaf, parentRef=None)
        request = {'requestId': uid(), 'format': 'xlsx', 'reviewMode': 'none',
                   'includeResolved': False, 'stylePolicy': 'template', 'bindings': {},
                   'queryId': displayed['queryId']}
        output, _ = HostedOffice.output(self, request, 'fields')
        self.assertEqual(output['preview']['query']['freshness'], 'stale')
        self.assertEqual(output['preview']['query']['rowCount'], 1)
        self.policy(lambda users: users['fields']['navigation']['tags'].update(
            read=False, create=False, rename=False, move=False))
        self.query(structural, status=404, resources=[])
        self.assertEqual(self.query(exact)['result']['rows'], [])
        self.call('office/export/chunk', {'requestId': request['requestId'], 'offset': 0}, 'fields', 409)

    def test_query_definition_static_tag_roots_validate_even_with_deferred_parameters(self):
        self.prepare_query()
        self.grant_query('alice', definitions=True, history=True)
        self.policy(lambda users: users['alice'].update(navigation={'tags': {
            'read': True, 'create': True}}))
        basis = self.call('navigation/registry')['tags']['basis']
        tag = self.call('navigation/registry/prepare', {'operationId': uid(), 'intent': {
            'kind': 'createTag', 'parentRef': None, 'basis': basis, 'displayName': 'Root'}})
        self.call('navigation/registry/commit', {key: tag[key] for key in ['operationId', 'planId']})
        root = tag['navigationRef']
        missing = f'weftext-tag://{root.split("/")[-2]}/{uid()}'
        doc = self.open()
        original = (self.root / 'Shared/Shared.typ').read_bytes()
        plan = {'where': f'false && tagInSubtree(tagRef("{root}"), tagRef("{missing}"))',
                'select': [self.column('true', 'value')], 'advanced': {'parameters': [
                    {'name': 'root', 'shape': {'kind': 'scalar', 'semantic': 'tag_ref'}}]}}
        request = dict(resource='Shared', sessionId=doc['sessionId'], baseId=doc['baseId'],
                       source=doc['source'], sourceHash=digest(doc['source']), serial=1,
                       operationId=uid(), label='weftext-query-tree', plan=plan)
        self.call('editor/map', {key: request[key] for key in ['sessionId', 'source', 'sourceHash', 'serial']})
        self.call('query/definition/prepare', request, status=404)
        self.assertEqual((self.root / 'Shared/Shared.typ').read_bytes(), original)
        self.assertEqual(self.call('query/definitions', {'resource': 'Shared'})['definitions'], [])
        deferred = copy.deepcopy(request)
        deferred['operationId'] = uid()
        deferred['plan']['where'] = f'false && tagInSubtree(tagRef("{root}"), root)'
        proposal = self.call('query/definition/prepare', deferred)
        self.assertTrue(proposal['draftOnly'])
        self.assertEqual((self.root / 'Shared/Shared.typ').read_bytes(), original)
        self.policy(lambda users: users['alice']['navigation']['tags'].update(read=False, create=False))
        fresh = self.open()
        deferred.update(operationId=uid(), sessionId=fresh['sessionId'], baseId=fresh['baseId'],
                        source=fresh['source'], sourceHash=digest(fresh['source']))
        self.call('editor/map', {key: deferred[key] for key in ['sessionId', 'source', 'sourceHash', 'serial']})
        self.call('query/definition/prepare', deferred, status=404)


if __name__ == '__main__':
    suite = unittest.TestSuite(HostedQuery(name) for name in unittest.defaultTestLoader.getTestCaseNames(HostedQuery)
                              if name.startswith('test_query_'))
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    sys.exit(not result.wasSuccessful())
