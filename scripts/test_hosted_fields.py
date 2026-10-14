"""Narrow fields/Query through the actual server, durable journal and ACL."""
import copy
import json
import secrets
import sys
import unittest
from test_hosted import uid, digest, Server
from test_hosted_editor import HostedEditor


class HostedFields(HostedEditor):
    def prepare_fields(self):
        self.server.close()
        local = Server(['--no-browser', '--data-dir', str(self.data)])
        try:
            code, schema, _ = local.call('/api/data/schema', {'root': str(self.root)})
            self.assertEqual(code, 200, schema)
            op = uid()
            code, plan, _ = local.call('/api/data/schema/prepare', {'root': str(self.root), 'source': schema['source'], 'beforeSource': schema['source'], 'beforeRevision': schema['revision'], 'operationId': op})
            self.assertEqual(code, 200, plan)
            code, result, _ = local.call('/api/data/schema/install', {'operationId': op, 'planId': plan['planId']})
            self.assertEqual(code, 200, result)
        finally: local.close()
        self.start()
        for name in ['Shared', 'Other', 'Secret']:
            doc = self.open(name)
            source = doc['source'].replace('types: ()', 'types: ("task",), status: "open", due: unknown(), aliases: 42, private: read("hidden-value")')
            self.save(doc, source)
        self.tokens['fields'] = secrets.token_urlsafe(32)
        self.tokens['fieldreader'] = secrets.token_urlsafe(32)
        def change(users):
            users['fields'] = {'enabled': True, 'tokenSha256': digest(self.tokens['fields']), 'grants': {
                n: {'fields': {'tasks/status': {'read': True, 'write': True}}} for n in ['Shared', 'Other']}}
            users['fieldreader'] = {'enabled': True, 'tokenSha256': digest(self.tokens['fieldreader']), 'grants': {
                'Shared': {'fields': {'tasks/status': {'read': True}}}}}
        self.policy(change)

    def field(self, name='Shared', actor='fields'):
        return self.call('fields/read', {'resource': name, 'fieldId': 'tasks/status'}, actor)

    def proposal(self, profile, value='done', op=None):
        return {'resource': profile['resource'], 'fieldId': 'tasks/status', 'basis': profile['basis'], 'value': {'kind': 'text', 'value': value}, 'operationId': op or uid()}

    def test_fields_keep_current_hidden_source_and_durable_original_receipt(self):
        self.prepare_fields()
        self.policy(lambda users: users['alice']['grants']['Shared'].update(fields={'tasks/status': {'read': True, 'write': True}}))
        personal = self.open()
        self.call('draft', {'sessionId': personal['sessionId'], 'source': personal['source'] + '\nMY_PRIVATE_UNSAVED_DRAFT'})
        own = self.field(actor='alice')
        self.call('fields/write', self.proposal(own, 'cancelled'), 'alice')
        reopened = self.open()
        self.assertIn('MY_PRIVATE_UNSAVED_DRAFT', reopened['recoveredDraft'])
        # Bob can change the author source without consuming Alice's draft.
        bob = self.open(actor='bob')
        self.save(bob, bob['source'].replace('status: "cancelled"', 'status: "open"'), 'bob')
        profile = self.field()
        self.assertTrue(profile['basis']); self.assertTrue(profile['field']['writable'])
        encoded = json.dumps(profile)
        for hidden in ['初始共同正文', 'hidden-value', 'unknown()', 'sourceRange', 'baseId', 'sha256', 'sourceHash', str(self.root), 'raw', 'nodeId', 'types']:
            self.assertNotIn(hidden, encoded)
        reader = self.field(actor='fieldreader'); self.assertIsNone(reader['basis']); self.assertFalse(reader['field']['writable'])
        self.call('fields/write', self.proposal(profile), 'fieldreader', 404)
        doc = self.open(actor='bob')
        updated = doc['source'].replace('private: read("hidden-value")', 'private: (notEvaluated: "different")') + '\n#let f(x) = x * 2\n#context query(heading)\n😀新正文\n'
        self.save(doc, updated, 'bob')
        req = self.proposal(profile)
        receipt = self.call('fields/write', req, 'fields')
        self.assertEqual(receipt['status'], 'committed')
        expected = updated.replace('status: "open"', 'status: "done"')
        self.assertEqual((self.root/'Shared/Shared.typ').read_text('utf-8'), expected)
        self.assertEqual(self.call('fields/write', req, 'fields'), receipt)
        self.server.close(); self.start()
        self.assertEqual(self.call('fields/write', req, 'fields'), receipt)
        self.call('fields/write', {**req, 'value': {'kind': 'text', 'value': 'cancelled'}}, 'fields', 409)
        self.assertEqual((self.root/'Shared/Shared.typ').read_text('utf-8'), expected)
        self.assertNotIn('baseId', receipt); self.assertNotIn('source', receipt)

    def test_fields_aba_external_change_revoke_and_settlement(self):
        self.prepare_fields()
        old = self.field(); other = self.field('Other')
        doc = self.open(); self.save(doc, doc['source'].replace('status: "open"', 'status: "done"'))
        doc = self.open(); self.save(doc, doc['source'].replace('status: "done"', 'status: "open"'))
        self.call('fields/write', self.proposal(old), 'fields', 409)
        pending = self.proposal(self.field())
        operation = {k: pending[k] for k in ['resource', 'fieldId', 'operationId']}
        self.assertEqual(self.call('fields/operation', operation, 'fields')['status'], 'notRegistered')
        self.assertEqual(self.call('fields/settle', operation, 'fields')['status'], 'cancelled')
        self.server.close(); self.start()
        self.assertEqual(self.call('fields/write', pending, 'fields')['status'], 'cancelled')
        before = (self.root/'Shared/Shared.typ').read_text('utf-8')
        (self.root/'Shared/Shared.typ').write_text(before+'\n外部未接纳内容', 'utf-8')
        self.call('fields/read', {'resource': 'Shared', 'fieldId': 'tasks/status'}, 'fields', 409)
        # Another document still progresses; the current external bytes survive.
        # Restart invalidated ephemeral field bases, so explicitly reacquire Other.
        other = self.field('Other'); self.call('fields/write', self.proposal(other), 'fields')
        self.assertTrue((self.root/'Shared/Shared.typ').read_text('utf-8').endswith('外部未接纳内容'))
        latest = self.field('Other')
        self.policy(lambda users: users['fields']['grants']['Other']['fields']['tasks/status'].update(read=False, write=False))
        self.call('fields/write', self.proposal(latest), 'fields', 404)
        self.policy(lambda users: users['fields']['grants']['Other']['fields']['tasks/status'].update(read=True, write=True))
        self.call('fields/write', self.proposal(latest), 'fields', 409)

    def test_field_acl_query_scope_hidden_schema_and_endpoint_isolation(self):
        self.prepare_fields()
        self.assertEqual(self.call('documents', actor='fields')['documents'], [])
        catalog = self.call('fields', actor='fields')['fields']
        self.assertEqual({v['resource'] for v in catalog}, {'Shared', 'Other'})
        hidden = self.call('fields/read', {'resource': 'Secret', 'fieldId': 'tasks/status'}, 'fields', 404)
        missing = self.call('fields/read', {'resource': 'NotThere', 'fieldId': 'tasks/status'}, 'fields', 404)
        self.assertEqual(hidden, missing)
        self.call('open', {'resource': 'Shared'}, 'fields', 404)
        self.call('download/prepare', {'resource': 'Shared'}, 'fields', 404)
        request = {'fields': ['tasks/status'], 'plan': {'where': 'status == "open"'}}
        result = self.call('query', request, 'fields')['result']
        self.assertEqual({r['entityKey'] for r in result['rows']}, {'Shared', 'Other'})
        self.assertTrue(all(r['cells']['status']['value']['value'] == 'open' for r in result['rows']))
        for hidden in ['Secret', '初始共同正文', 'private', 'hidden-value', str(self.root)]:
            self.assertNotIn(hidden, json.dumps(result))
        query = copy.deepcopy(request); query['plan'] = {'select': [{'name': x, 'expr': x} for x in ['body', 'title', 'types', 'self']]}
        unavailable = self.call('query', query, 'fields')['result']
        self.assertTrue(all(c['state'] == 'unavailable' for r in unavailable['rows'] for c in r['cells'].values()))
        for expr in ['due', 'field("core/aliases")', 'field("made-up/hidden")']:
            query['plan'] = {'select': [{'name': 'value', 'expr': expr}]}
            denied = self.call('query', query, 'fields', 400)
            self.assertNotIn('aliases', json.dumps(denied))
        # Literal inputs cannot turn a status-only grant into a shared temporal
        # registry oracle. Pure CEL/native date operations remain available.
        for expr in ['compareTemporal({"calendar": "private"}, {"calendar": "private"})',
                     'rangeDuration({"start": "2026-10-14", "end_exclusive": "2026-10-15"})']:
            query['plan'] = {'select': [{'name': 'value', 'expr': expr}]}
            denied = self.call('query', query, 'fields', 400)
            self.assertNotIn('private', json.dumps(denied))
        query['plan'] = {'select': [{'name': 'value', 'expr': 'date("2026-10-14") == date("2026-10-14") && status == "open"'}]}
        native = self.call('query', query, 'fields')['result']
        self.assertTrue(all(r['cells']['value']['value']['value'] for r in native['rows']))
        # An authorized field which the current Type does not contribute is
        # absent, not a reason to lose the whole row or decode its unknown key.
        other = self.open('Other')
        self.save(other, other['source'].replace('types: ("task",)', 'types: ()').replace('status: "open"', 'status: read("HIDDEN_UNBOUND_VALUE")'))
        absent = self.call('query', {'fields': ['tasks/status'], 'resources': ['Other'], 'plan': {}}, 'fields')['result']
        self.assertEqual(len(absent['rows']), 1)
        self.assertEqual(absent['rows'][0]['cells']['status']['state'], 'absent')
        self.assertNotIn('HIDDEN_UNBOUND_VALUE', json.dumps(absent))
        other = self.open('Other')
        self.save(other, other['source'].replace('types: ()', 'types: ("task",)').replace('status: read("HIDDEN_UNBOUND_VALUE")', 'status: "open"'))
        self.call('query', {**request, 'resources': ['Secret']}, 'fields', 404)
        for field in ['core/aliases', 'invented/hidden']:
            self.call('query', {'fields': [field], 'resources': [], 'plan': {}}, 'fields', 404)
        self.policy(lambda users: users['fields']['grants'].pop('Shared'))
        remaining = self.call('query', request, 'fields')['result']
        self.assertEqual([r['entityKey'] for r in remaining['rows']], ['Other'])
        self.assertEqual(len(self.call('query', request, 'fieldreader')['result']['rows']), 1)

    def test_narrow_write_requires_protected_admitted_schema(self):
        doc = self.open()
        self.save(doc, doc['source'].replace('types: ()', 'types: ("task",), status: "open"'))
        self.policy(lambda users: users['alice']['grants']['Shared'].update(fields={'tasks/status': {'read': True, 'write': True}}))
        default = self.field(actor='alice')
        self.assertEqual(default['field']['cell']['value']['value'], 'open')
        self.assertIsNone(default['basis']); self.assertFalse(default['field']['writable'])
        self.prepare_fields()
        installed = self.field()
        schema = self.root/'.weftext-schema.json'
        old = schema.read_bytes()
        schema.write_bytes(old+b'\n')
        unchanged = (self.root/'Shared/Shared.typ').read_bytes()
        self.call('fields/write', self.proposal(installed), 'fields', 409)
        self.assertEqual((self.root/'Shared/Shared.typ').read_bytes(), unchanged)


if __name__ == '__main__': unittest.main(argv=[sys.argv[0]], defaultTest='HostedFields', verbosity=2)
