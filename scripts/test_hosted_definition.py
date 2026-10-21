"""Actual hosted saved-definition reservations, explicit saves and recovery.

These cases use owned fixtures and the real Core journal/HTTP boundary. Local
routes only provision the initial library/schema while its server is stopped.
"""
from _creation_result import created_document
import json
import sys
import unittest

from test_hosted import digest, uid
from test_hosted_query import HostedQuery


class HostedDefinition(HostedQuery):
    def provision(self):
        self.prepare_query()
        self.policy(lambda users: [users[actor]['grants'][resource].update(
            fields={'tasks/status': {'read': True}},
            query={'definitions': True, 'definitionHistory': True})
            for actor in ['alice', 'bob'] for resource in ['Shared', 'Other']])

    def proposal(self, doc, *, source=None, serial=1, operation=None,
                 label='weftext-query-created', plan=None, actor='alice'):
        source = doc['source'] if source is None else source
        request = dict(resource=doc['resource'], sessionId=doc['sessionId'],
                       baseId=doc['baseId'], source=source, sourceHash=digest(source),
                       serial=serial, operationId=operation or uid(), label=label,
                       plan=plan or {'select': [self.column()], 'where': 'status == "open"'})
        self.call('editor/map', {key: request[key] for key in ['sessionId', 'source', 'sourceHash', 'serial']}, actor)
        return self.call('query/definition/prepare', request, actor), request

    def definition_save(self, doc, source, proposals, actor='alice', status=200, operation=None):
        request = dict(sessionId=doc['sessionId'], baseId=doc['baseId'], source=source,
                       operationId=operation or uid(), definitionReservations=proposals)
        return self.call('save', request, actor, status), request

    def definitions(self, resource='Shared', actor='alice'):
        return self.call('query/definitions', {'resource': resource}, actor)['definitions']

    def receipt(self, resource, operation, actor='alice'):
        return self.call('query/definition/operation', {'resource': resource, 'operationId': operation}, actor)

    def test_definition_prepare_is_draft_only_and_explicit_save_admits_original_key(self):
        self.provision()
        doc = self.open()
        personal = doc['source'] + '\nPERSONAL_DRAFT_BEFORE_QUERY😀'
        self.call('draft', {'sessionId': doc['sessionId'], 'source': personal})
        prepared, request = self.proposal(doc, source=personal)
        self.assertEqual(prepared['status'], 'reserved')
        self.assertTrue(prepared['draftOnly'])
        self.assertEqual(prepared['patchUnit'], 'utf16')
        key = prepared['definition']['key']
        self.assertIn(key, prepared['source'])
        self.assertIn('PERSONAL_DRAFT_BEFORE_QUERY😀', prepared['source'])
        self.assertEqual(self.open()['source'], doc['source'])
        self.assertEqual(self.open()['recoveredDraft'], personal)
        self.assertEqual(self.definitions(), [])
        replay = self.call('query/definition/prepare', request)
        self.assertEqual(replay['definition'], prepared['definition'])
        self.assertEqual(replay['source'], prepared['source'])
        self.assertEqual(replay['patches'], prepared['patches'])
        changed = {**request, 'label': 'weftext-query-different'}
        self.call('query/definition/prepare', changed, status=409)
        result, save = self.definition_save(doc, prepared['source'], [request['operationId']])
        self.assertEqual(result['status'], 'committed')
        self.assertEqual([item['key'] for item in self.definitions()], [key])
        self.assertEqual(self.receipt('Shared', request['operationId'])['status'], 'admitted')
        self.assertEqual(self.call('save', save)['status'], 'committed')
        current = self.open()['source']
        self.assertEqual(current.count(key), 1)
        self.assertEqual(current, prepared['source'])
        # Admission is historical identity, so later ordinary Undo/save/Redo
        # does not need the allocation ID again or mint another key.
        self.save(self.open(), personal)
        self.assertEqual(self.definitions(), [])
        self.save(self.open(), prepared['source'])
        self.assertEqual([item['key'] for item in self.definitions()], [key])
        self.assertEqual(self.receipt('Shared', request['operationId'])['status'], 'admitted')

    def test_definition_multiple_drafts_undo_save_redo_and_restart_keep_original_keys(self):
        self.provision()
        doc = self.open()
        first, one = self.proposal(doc, label='weftext-query-one')
        second, two = self.proposal(doc, source=first['source'], serial=2,
                                    label='weftext-query-two', plan={'advanced': {'call': {
                                        'owner': 'Shared', 'selection': {'mode': 'current', 'key': first['definition']['key']}}}})
        # Undo removes both carriers. Saving that actual source does not admit
        # either key and does not cancel their durable reservations.
        self.definition_save(doc, doc['source'] + '\nSAVED_WHILE_UNDONE', [one['operationId'], two['operationId']])
        self.assertEqual(self.definitions(), [])
        self.assertEqual(self.receipt('Shared', one['operationId'])['status'], 'reserved')
        self.server.close()
        self.start()
        current = self.open()
        listed = self.call('query/definition/reservations', {'sessionId': current['sessionId']})
        self.assertEqual({item['operationId'] for item in listed['reservations']}, {one['operationId'], two['operationId']})
        # Old prepare recovery is by the original ID/request even though its
        # editor session is gone. It never writes the returned historical patch.
        replay = self.call('query/definition/prepare', one)
        self.assertEqual(replay['definition']['key'], first['definition']['key'])
        self.assertEqual(replay['source'], first['source'])
        final_source = second['source'] + '\nSAVED_WHILE_UNDONE\nREDO_EXPLICIT_SAVE'
        saved, request = self.definition_save(current, final_source, [one['operationId'], two['operationId']])
        self.assertEqual(saved['status'], 'committed')
        self.assertEqual({item['key'] for item in self.definitions()},
                         {first['definition']['key'], second['definition']['key']})
        self.server.close()
        self.start()
        self.assertEqual(self.call('operation', {'operationId': request['operationId']})['status'], 'committed')
        self.assertEqual(self.receipt('Shared', two['operationId'])['status'], 'admitted')
        self.assertEqual(self.open()['source'], final_source)
        call = {'advanced': {'call': {'owner': 'Shared', 'selection': {'mode': 'current', 'key': second['definition']['key']}}}}
        self.assertEqual([row['cells']['status']['value']['value'] for row in self.query(call, actor='alice')['result']['rows']], ['open'])

    def test_definition_plain_source_save_cannot_admit_unbound_or_foreign_keys(self):
        self.provision()
        doc = self.open()
        prepared, proposal = self.proposal(doc)
        key = prepared['definition']['key']
        # Valid author source can be retained without a definition admission.
        # Neither the exposed UUID nor a manually invented UUID is evidence.
        self.save(doc, prepared['source'])
        self.assertEqual(self.definitions(), [])
        self.assertEqual(self.receipt('Shared', proposal['operationId'])['status'], 'reserved')
        current = self.open()
        self.definition_save(current, prepared['source'] + '\nEXPLICIT_ADMISSION', [proposal['operationId']])
        self.assertEqual([item['key'] for item in self.definitions()], [key])
        current = self.open()
        unbound, _ = self.proposal(current, label='weftext-query-not-admitted')
        self.save(current, unbound['source'])
        partial = self.call('query/definitions', {'resource': 'Shared'})
        self.assertEqual(partial['coverage'], 'partial')
        self.assertEqual([item['key'] for item in partial['definitions']], [key])
        self.assertIn(unbound['definition']['key'], [item['key'] for item in partial['unavailable']])
        catalog = self.call('query/catalog')
        self.assertEqual(catalog['coverage'], 'partial')
        self.assertIn(key, [item['key'] for item in catalog['definitions']])
        other = self.open('Other')
        foreign = prepared['source'].replace(self.docs['Shared']['nodeId'], self.docs['Other']['nodeId'])
        self.definition_save(other, foreign, [proposal['operationId']], status=409)
        self.assertEqual(self.open('Other')['source'], other['source'])
        self.save(other, foreign)
        self.assertEqual(self.definitions('Other'), [])
        # A different actor cannot admit an allocation by learning its public
        # key/proposal ID, even with equivalent rights over the same resource.
        bob = self.open('Other', 'bob')
        self.definition_save(bob, bob['source'] + '\nBOB_ATTEMPT', [proposal['operationId']], 'bob', 409)
        self.assertEqual(self.open('Other')['source'], foreign)

    def test_definition_settlement_fences_unknown_and_prepared_requests(self):
        self.provision()
        doc = self.open()
        unknown = uid()
        self.assertEqual(self.call('query/definition/settle', {'resource': 'Shared', 'operationId': unknown})['status'], 'cancelled')
        delayed, _ = self.proposal(doc, operation=unknown)
        self.assertEqual(delayed['status'], 'cancelled')
        self.assertNotIn('patches', delayed)
        self.assertNotIn('definition', delayed)
        self.assertEqual(self.open()['source'], doc['source'])
        prepared, proposal = self.proposal(doc, serial=2, label='weftext-query-settled')
        settled = self.call('query/definition/settle', {'resource': 'Shared', 'operationId': proposal['operationId']})
        self.assertEqual(settled['status'], 'cancelled')
        self.definition_save(doc, prepared['source'], [proposal['operationId']], status=409)
        self.server.close()
        self.start()
        self.assertEqual(self.call('query/definition/prepare', proposal)['status'], 'cancelled')
        self.assertEqual(self.receipt('Shared', unknown)['status'], 'cancelled')
        self.assertEqual(self.open()['source'], doc['source'])
        self.assertEqual(self.definitions(), [])

    def test_definition_unfinished_author_source_retains_draft_without_admitting_then_repairs(self):
        self.provision()
        doc = self.open()
        prepared, proposal = self.proposal(doc)
        unfinished = prepared['source'] + '\n#let still_editing = (\n'
        _, request = self.definition_save(doc, unfinished, [proposal['operationId']], status=409)
        retained = self.open()
        self.assertEqual(retained['source'], doc['source'])
        self.assertEqual(retained['recoveredDraft'], unfinished)
        self.assertEqual(self.call('operation', {'operationId': request['operationId']})['status'], 'notCommitted')
        self.assertIn(self.call('operation/settle', {'resource': 'Shared', 'operationId': request['operationId']})['status'], ['cancelled', 'resolved'])
        self.assertEqual(self.receipt('Shared', proposal['operationId'])['status'], 'reserved')
        self.server.close()
        self.start()
        current = self.open()
        self.assertEqual(current['source'], doc['source'])
        self.assertEqual(current['recoveredDraft'], unfinished)
        self.assertEqual(self.receipt('Shared', proposal['operationId'])['status'], 'reserved')
        repaired = unfinished + ')\n'
        self.definition_save(current, repaired, [proposal['operationId']])
        self.assertEqual(self.open()['source'], repaired)
        self.assertEqual([item['key'] for item in self.definitions()], [prepared['definition']['key']])
        self.assertEqual(self.receipt('Shared', proposal['operationId'])['status'], 'admitted')

    def test_definition_authority_exact_draft_and_revoke_before_save_preserve_input(self):
        self.provision()
        doc = self.open()
        prepared, proposal = self.proposal(doc)
        stale = {**proposal, 'operationId': uid(), 'label': 'weftext-query-stale'}
        changed = doc['source'] + '\nNEWER_SERIAL'
        self.call('editor/map', {'sessionId': doc['sessionId'], 'source': changed, 'serial': 2})
        self.call('editor/map', {'sessionId': doc['sessionId'], 'source': doc['source'], 'serial': 3})
        self.call('query/definition/prepare', stale, status=409)
        self.call('query/definition/prepare', {**stale, 'serial': 3, 'sourceHash': '0' * 64}, status=409)
        self.call('query/definition/prepare', {**stale, 'serial': 3}, 'bob', 410)
        self.policy(lambda users: users['alice']['grants']['Shared']['query'].update(definitions=False, definitionHistory=False))
        self.call('query/definition/prepare', proposal, status=404)
        reopened = self.open()
        self.definition_save(reopened, prepared['source'], [proposal['operationId']], status=404)
        self.assertEqual(self.open()['source'], doc['source'])
        self.assertEqual(self.call('query/definition/operation', {'resource': 'Shared', 'operationId': proposal['operationId']}, status=404),
                         self.call('query/definition/operation', {'resource': 'NotThere', 'operationId': proposal['operationId']}, status=404))

    def test_definition_hidden_and_unknown_fields_share_one_refusal(self):
        self.provision()
        doc = self.open()
        self.call('editor/map', {'sessionId': doc['sessionId'], 'source': doc['source'], 'serial': 1})
        responses = []
        for expression in ['field("tasks/due")', 'field("private/nonexistent-field")', 'due', 'nonexistent_field']:
            operation = uid()
            request = dict(resource='Shared', sessionId=doc['sessionId'], baseId=doc['baseId'],
                           source=doc['source'], serial=1, operationId=operation,
                           label='weftext-query-hidden-probe', plan={'select': [self.column(expression, 'probe')]})
            responses.append(self.call('query/definition/prepare', request, status=404))
            self.assertEqual(self.receipt('Shared', operation)['status'], 'notRegistered')
        self.assertTrue(all(response == responses[0] for response in responses))
        self.assertEqual(self.open()['source'], doc['source'])

    def test_definition_current_schema_and_callee_changes_are_revalidated_at_save(self):
        self.provision()
        callee = self.add_definition({'select': [self.column()]})
        doc = self.open()
        call = {'advanced': {'call': {'owner': 'Other', 'selection': {'mode': 'current', 'key': callee['definition']['key']}}},
                'where': 'status == "open"'}
        prepared, proposal = self.proposal(doc, plan=call)
        stored_owner = prepared['definition']['plan']['advanced']['call']['owner']
        self.assertTrue(stored_owner.startswith('weftext-node://'))
        self.assertTrue(stored_owner.endswith('/' + self.docs['Other']['nodeId']))
        # The originally valid string column becomes an integer. A stale
        # preparation cannot install a caller under the old callee contract.
        self.add_definition({'select': [self.column('1')]}, key=callee['definition']['key'])
        current = self.open()
        self.definition_save(current, prepared['source'], [proposal['operationId']], status=404)
        self.assertEqual(self.open()['source'], doc['source'])
        simple, simple_request = self.proposal(current, label='weftext-query-schema')
        def change_schema(api):
            config = api('/api/data/schema', {'root': str(self.root)})
            schema = json.loads(config['source'])
            field = next(item for item in schema['fields'] if item['id'] == 'tasks/status')
            field['shape'] = {'kind': 'scalar', 'semantic': 'int'}
            operation = uid()
            plan = api('/api/data/schema/prepare', {'root': str(self.root), 'source': json.dumps(schema),
                       'beforeSource': config['source'], 'beforeRevision': config['revision'], 'operationId': operation})
            api('/api/data/schema/install', {'operationId': operation, 'planId': plan['planId']})
        self.local(change_schema)
        current = self.open()
        self.definition_save(current, simple['source'], [simple_request['operationId']], status=409)
        self.assertEqual(self.open()['source'], doc['source'])

    def test_definition_required_parameters_are_static_and_products_need_separate_grants(self):
        self.provision()
        doc = self.open()
        parameterized = {'where': 'status == wanted', 'select': [self.column()],
                         'advanced': {'parameters': [{'name': 'wanted', 'shape': {'kind': 'scalar', 'semantic': 'text'}}]}}
        prepared, proposal = self.proposal(doc, plan=parameterized)
        self.definition_save(doc, prepared['source'], [proposal['operationId']])
        call = {'advanced': {'call': {'owner': 'Shared', 'selection': {'mode': 'current', 'key': prepared['definition']['key']},
                                      'arguments': {'wanted': '"open"'}}}}
        # Definition construction checks declared parameter shapes without
        # executing a parameterless result or inventing sample values.
        self.assertEqual(len(self.definitions()), 1)
        self.assertEqual([row['cells']['status']['value']['value'] for row in self.query(call, actor='alice')['result']['rows']], ['open'])
        doc = self.open()
        request = dict(resource='Shared', sessionId=doc['sessionId'], baseId=doc['baseId'],
                       source=doc['source'], sourceHash=digest(doc['source']), serial=1,
                       operationId=uid(), label='weftext-query-body', plan={'select': [self.column('body', 'body')]})
        self.call('editor/map', {key: request[key] for key in ['sessionId', 'source', 'sourceHash', 'serial']})
        self.call('query/definition/prepare', request, status=404)
        self.assertEqual(self.open()['source'], doc['source'])
        self.assertEqual(self.receipt('Shared', request['operationId'])['status'], 'notRegistered')
        self.grant_query(actor='alice', definitions=True, history=True, products=['body'])
        doc = self.open()
        body, allocated = self.proposal(doc, plan=request['plan'], label=request['label'])
        self.definition_save(doc, body['source'], [allocated['operationId']])
        self.assertEqual(len(self.definitions()), 2)

    def test_definition_resource_remapping_never_delivers_or_admits_old_owner_proposal(self):
        self.provision()
        doc = self.open()
        prepared, request = self.proposal(doc, source=doc['source'] + '\nORIGINAL_OWNER_PRIVATE_PROPOSAL😀')
        replacement = self.local(lambda api: created_document(api('/api/note/create', {'root': str(self.root), 'parent': str(self.root),
                                                                     'name': 'Replacement', 'operationId': uid()})))
        original_bytes = (self.root / 'Shared/Shared.typ').read_bytes()
        other_bytes = (self.root / 'Replacement/Replacement.typ').read_bytes()
        self.server.close()
        self.config['files']['Shared']['path'] = 'Replacement/Replacement.typ'
        self.start()
        for route, body in [('query/definition/prepare', request),
                            ('query/definition/operation', {'resource': 'Shared', 'operationId': request['operationId']})]:
            code, value, _ = self.server.call('/api/hosted/' + route, body, self.tokens['alice'])
            self.assertIn(code, [404, 409], value)
            self.assertNotIn('ORIGINAL_OWNER_PRIVATE_PROPOSAL', json.dumps(value))
            self.assertNotIn(prepared['source'], json.dumps(value, ensure_ascii=False))
        current = self.open()
        self.assertEqual(self.call('query/definition/reservations', {'sessionId': current['sessionId']})['reservations'], [])
        substituted = prepared['source'].replace(self.docs['Shared']['nodeId'], replacement['nodeId'])
        self.definition_save(current, substituted, [request['operationId']], status=409)
        self.assertEqual((self.root / 'Shared/Shared.typ').read_bytes(), original_bytes)
        self.assertEqual((self.root / 'Replacement/Replacement.typ').read_bytes(), other_bytes)


if __name__ == '__main__':
    suite = unittest.TestSuite(HostedDefinition(name) for name in unittest.defaultTestLoader.getTestCaseNames(HostedDefinition)
                              if name.startswith('test_definition_'))
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    sys.exit(not result.wasSuccessful())
