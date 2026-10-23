"""Actual hosted directory and Trash scopes over explicitly owned fixtures.

Local Core routes provision admissions and structural operations while the hosted
server is stopped. Query, definition and output consumers use hosted HTTP only.
"""
from _creation_result import created_document
import copy
import json
import shutil
import sys
import time
import unittest
from concurrent.futures import ThreadPoolExecutor
from owned_worker_observer import OwnedWorkerObservation, event_time_ns, timed_call
from pathlib import Path

from test_hosted import uid
from test_hosted_definition import HostedDefinition
from test_hosted_office import HostedOffice
from test_hosted_query import HostedQuery


class HostedScopes(HostedQuery):
    def prepare_scopes(self):
        self.prepare_query()
        self.refs = {}

        def provision(api):
            for key, directory in [('root', self.root), ('empty', self.root / 'Empty')]:
                if directory != self.root:
                    directory.mkdir()
                prepared = api('/api/navigation/admit-directory', {
                    'root': str(self.root), 'path': str(directory), 'operationId': uid()})
                self.commit_local(api, prepared['preview'])
                self.refs[key] = prepared['targetRef']
            for key, directory in [('shared', self.root / 'Shared'), ('secret', self.root / 'Secret')]:
                self.refs[key] = api('/api/navigation/capture', {
                    'root': str(self.root), 'path': str(directory)})['targetRef']
            child = created_document(api('/api/note/create', {'root': str(self.root),
                'parent': str(self.root / 'Shared'), 'name': 'Child', 'operationId': uid()}))
            source = (f'#metadata((id: "{child["nodeId"]}", types: ("task",), '
                      'status: "open", private: "PRIVATE_CHILD")) <weftext>\n= 子笔记😀\n子正文\n')
            api('/api/save', {'sessionId': child['sessionId'], 'baseId': child['baseId'],
                             'source': source, 'operationId': uid()})
            self.docs['Child'] = child
            self.refs['child'] = api('/api/navigation/capture', {
                'root': str(self.root), 'path': str(self.root / 'Shared/Child')})['targetRef']
            self.config['files']['Child'] = {
                'path': 'Shared/Child/Child.typ', 'title': 'Child', 'editable': True}
            self.config['directories'] = {
                key: {'path': path, 'title': key, 'reference': self.refs[key]}
                for key, path in [('root', ''), ('shared', 'Shared'), ('empty', 'Empty'), ('secret', 'Secret')]}

        self.local(provision)
        self.policy(lambda users: users['fields']['grants'].update(
            Child={'fields': {'tasks/status': {'read': True}}}))

    def commit_local(self, api, preview):
        result = api('/api/operations/commit', {
            'operationId': preview['operationId'], 'planId': preview['planId']})
        self.assertEqual(result['status'], 'committed')
        return result

    def operation_local(self, api, intent):
        return self.commit_local(api, api('/api/operations/prepare', {
            'root': str(self.root), 'operationId': uid(), 'intent': intent}))

    def grant_scopes(self, directories=(), trash=False, actor='fields'):
        return self.policy(lambda users: users[actor].update(
            queryScopes={'directories': list(directories), 'trash': trash}))

    def scope_plan(self, directory=None, descendants=True, lifecycle='live', expr='status'):
        scope = {'descendants': descendants, 'lifecycle': lifecycle}
        if directory is not None:
            scope['directory'] = directory
        return {'scope': scope, 'select': [self.column(expr)]}

    def rows(self, plan, **extra):
        output = self.query(plan, **extra)
        self.assertEqual(output['result']['coverage'], 'complete', output)
        self.no_paths(output)
        return output['result']['rows']

    def no_paths(self, output):
        text = json.dumps(output, ensure_ascii=False).replace('\\\\', '\\')
        for path in (self.root, self.data):
            self.assertNotIn(str(path), text)
            self.assertNotIn(path.as_posix(), text)
        self.assertNotIn('Volume{', text)
        self.assertNotIn('PRIVATE_', text)

    def unavailable(self, plan, **extra):
        code, output, _ = self.server.call('/api/hosted/query', {
            'fields': ['tasks/status'], 'plan': plan, **extra}, self.tokens['fields'])
        if code == 200:
            self.assertEqual(output['result']['coverage'], 'partial', output)
        else:
            self.assertIn(code, [400, 404, 409], output)
            self.assertNotIn('queryId', output)
        self.no_paths(output)
        return code, output

    def restart_config(self):
        self.server.close()
        self.start()

    def test_scope_independent_catalog_hidden_unknown_and_empty(self):
        self.prepare_scopes()
        catalog = self.call('query/catalog', actor='fields')
        self.assertEqual(catalog['directories'], [])
        self.assertFalse(catalog['trash'])
        hidden = self.query(self.scope_plan('secret'), resources=[], status=404)
        missing = self.query(self.scope_plan('never-configured'), resources=[], status=404)
        self.assertEqual(hidden, missing)
        self.grant_scopes(['root', 'shared', 'empty'])
        catalog = self.call('query/catalog', actor='fields')
        self.assertEqual({item['id'] for item in catalog['directories']}, {'root', 'shared', 'empty'})
        self.assertTrue(all(item['status'] == 'available' for item in catalog['directories']))
        self.assertEqual(catalog['scopeCoverage'], 'complete')
        self.no_paths(catalog)
        self.assertNotIn(self.refs['secret'], json.dumps(catalog))
        self.call('open', {'resource': 'Shared'}, 'fields', 404)
        self.call('download/prepare', {'resource': 'Shared'}, 'fields', 404)
        self.assertEqual(self.rows(self.scope_plan('shared'), resources=[]), [])
        self.assertEqual(self.rows(self.scope_plan('empty')), [])
        self.query(self.scope_plan(lifecycle='trash'), resources=[], status=404)

    def test_scope_node_descendants_stable_ref_and_unrelated_hidden_damage(self):
        self.prepare_scopes()
        self.grant_scopes(['root', 'shared'])
        self.assertEqual([r['entityKey'] for r in self.rows(self.scope_plan('shared', False))], ['Shared'])
        self.assertEqual({r['entityKey'] for r in self.rows(self.scope_plan('shared'))}, {'Shared', 'Child'})
        self.assertEqual({r['entityKey'] for r in self.rows(self.scope_plan(self.refs['shared']))}, {'Shared', 'Child'})
        self.assertEqual({r['entityKey'] for r in self.rows(self.scope_plan('root'))}, {'Shared', 'Child', 'Other'})
        hidden = self.root / 'Secret/Secret.typ'
        original = hidden.read_bytes()
        hidden.write_bytes(b'UNRELATED_INVALID_PRIVATE_SOURCE')
        self.assertEqual({r['entityKey'] for r in self.rows(self.scope_plan('root'))}, {'Shared', 'Child', 'Other'})
        self.assertEqual(hidden.read_bytes(), b'UNRELATED_INVALID_PRIVATE_SOURCE')
        self.assertNotEqual(hidden.read_bytes(), original)
        self.config['directories']['shared']['path'] = 'sHaReD'
        self.restart_config()
        self.assertEqual({r['entityKey'] for r in self.rows(self.scope_plan('shared'))}, {'Shared', 'Child'})

    def test_scope_copied_marker_and_raw_directory_replacement_are_unavailable(self):
        self.prepare_scopes()
        self.grant_scopes(['empty'])
        empty, held = self.root / 'Empty', self.root / 'OwnedHeldEmpty'
        for path in (empty, held):
            self.assertTrue(path.resolve().is_relative_to(self.root.resolve()))
        self.assertFalse(held.exists())
        # A same-volume move of this owned fixture is retained, never deleted.
        empty.rename(held)
        shutil.copytree(held, empty)
        catalog = self.call('query/catalog', actor='fields')
        entry = next(item for item in catalog['directories'] if item['id'] == 'empty')
        self.assertEqual(entry['status'], 'unavailable')
        self.assertEqual(catalog['scopeCoverage'], 'partial')
        self.unavailable(self.scope_plan('empty'), resources=[])
        self.unavailable(self.scope_plan(self.refs['empty']))
        self.assertTrue((held / 'weftext.directory.json').exists())
        self.assertTrue((empty / 'weftext.directory.json').exists())

    def test_scope_proven_move_keeps_ref_and_old_address_reoccupation_cannot_rebind(self):
        self.prepare_scopes()
        self.grant_scopes(['shared'])
        def move(api):
            self.operation_local(api, {'kind': 'renameNode', 'source': str(self.root / 'Shared'), 'name': 'Moved'})
            self.operation_local(api, {'kind': 'copyNode', 'source': str(self.root / 'Moved'),
                                       'parent': str(self.root), 'name': 'Shared', 'allowLosses': True})
            self.config['files']['Shared']['path'] = 'Moved/Moved.typ'
            self.config['files']['Child']['path'] = 'Moved/Child/Child.typ'
            self.config['directories']['shared']['path'] = 'Moved'
        self.local(move)
        self.assertEqual({r['entityKey'] for r in self.rows(self.scope_plan(self.refs['shared']))}, {'Shared', 'Child'})
        self.config['directories']['shared']['path'] = 'Shared'
        self.restart_config()
        self.unavailable(self.scope_plan('shared'), resources=[])
        self.unavailable(self.scope_plan(self.refs['shared']))
        self.config['directories']['shared']['path'] = 'Moved'
        self.restart_config()
        self.assertEqual({r['entityKey'] for r in self.rows(self.scope_plan('shared'))}, {'Shared', 'Child'})

    def test_scope_nested_current_pinned_and_union_recheck_actual_scope_grants(self):
        self.prepare_scopes()
        definition = self.add_definition(self.scope_plan(self.refs['shared']))
        self.grant_query(definitions=True, history=True)
        key = definition['definition']['key']
        call = {'advanced': {'call': {'owner': 'Other', 'selection': {'mode': 'current', 'key': key}}}}
        self.query(call, status=404)
        self.grant_scopes(['shared'])
        self.assertEqual({r['entityKey'] for r in self.rows(call)}, {'Shared', 'Child'})
        pinned = copy.deepcopy(call)
        pinned['advanced']['call']['selection'] = {
            'mode': 'pinned', 'key': key, 'revision': definition['definitionRevision']}
        self.assertEqual({r['entityKey'] for r in self.rows(pinned)}, {'Shared', 'Child'})
        union = {'advanced': {'unionAll': [call, self.scope_plan(self.refs['shared'], False)]}}
        self.assertEqual(len(self.rows(union)), 3)
        self.grant_scopes([])
        for plan in (call, pinned, union):
            self.query(plan, resources=[], status=404)

    def test_scope_definition_prepare_canonicalizes_and_save_rechecks_without_losing_source(self):
        self.prepare_scopes()
        self.policy(lambda users: [users['alice']['grants'][resource].update(
            fields={'tasks/status': {'read': True}}, query={'definitions': True, 'definitionHistory': True})
            for resource in ['Shared', 'Other']])
        doc = self.open('Other')
        self.call('editor/map', {'sessionId': doc['sessionId'], 'source': doc['source'], 'serial': 1})
        request = dict(resource='Other', sessionId=doc['sessionId'], baseId=doc['baseId'],
            source=doc['source'], serial=1, operationId=uid(), label='weftext-query-scoped',
            plan=self.scope_plan('shared'))
        self.call('query/definition/prepare', request, status=404)
        self.grant_scopes(['shared'], actor='alice')
        # A policy change ends the old actor epoch and its open session.
        # Explicitly reopen before starting the now-authorized proposal.
        doc = self.open('Other')
        self.call('editor/map', {'sessionId': doc['sessionId'], 'source': doc['source'], 'serial': 1})
        request.update(sessionId=doc['sessionId'], baseId=doc['baseId'], operationId=uid())
        prepared = self.call('query/definition/prepare', request)
        self.assertEqual(prepared['status'], 'reserved')
        self.assertEqual(prepared['definition']['plan']['scope']['directory'], self.refs['shared'])
        self.assertEqual(self.open('Other')['source'], doc['source'])
        self.grant_scopes([], actor='alice')
        current = self.open('Other')
        HostedDefinition.definition_save(self, current, prepared['source'], [request['operationId']], status=404)
        self.assertEqual(self.open('Other')['source'], doc['source'])
        self.grant_scopes(['shared'], actor='alice')
        current = self.open('Other')
        committed, _ = HostedDefinition.definition_save(self, current, prepared['source'], [request['operationId']])
        self.assertEqual(committed['status'], 'committed')
        self.assertIn(self.refs['shared'], self.open('Other')['source'])

    def test_scope_office_keeps_exact_result_and_scope_only_revoke_expires_delivery(self):
        self.prepare_scopes()
        self.grant_scopes(['shared'])
        self.policy(lambda users: [users['fields']['grants'][resource].update(
            office={'export': True, 'download': True}) for resource in ('Shared', 'Child', 'Other')])
        output = self.query(self.scope_plan('shared'))
        request = {'outputLocale': 'zh-CN', 'requestId': uid(), 'queryId': output['queryId'], 'format': 'xlsx',
                   'reviewMode': 'none', 'includeResolved': False, 'stylePolicy': 'template', 'bindings': {}}
        prepared = self.call('office/export/prepare', request, 'fields')
        original = HostedOffice.deliver(self, prepared, 'fields')
        # A different later Query cannot replace the prepared result's bytes.
        self.rows(self.scope_plan('shared', False))
        self.assertEqual(HostedOffice.deliver(self, prepared, 'fields'), original)
        for xml in HostedOffice.parts(original).values():
            self.assertNotIn(str(self.root), xml)
            self.assertNotIn('Volume{', xml)
            self.assertNotIn('PRIVATE_', xml)
        self.grant_scopes([])
        self.assertEqual(self.call('office/export/status', {'requestId': request['requestId']}, 'fields')['status'], 'expired')
        self.call('office/export/chunk', {'requestId': request['requestId'], 'offset': 0}, 'fields', 409)
        self.grant_scopes(['shared'])
        self.assertEqual(self.call('office/export/status', {'requestId': request['requestId']}, 'fields')['status'], 'expired')
        self.assertEqual({r['entityKey'] for r in self.rows(self.scope_plan('shared'))}, {'Shared', 'Child'})

    def test_scope_rules_change_blocks_new_query_and_original_definition_save(self):
        self.prepare_scopes()
        self.grant_scopes(['shared'])
        self.grant_scopes(['shared'], actor='alice')
        self.policy(lambda users: [users['alice']['grants'][resource].update(
            fields={'tasks/status': {'read': True}}, query={'definitions': True})
            for resource in ('Shared', 'Other')])
        doc = self.open('Other')
        prepared, original = HostedDefinition.proposal(self, doc, plan=self.scope_plan('shared'))
        rules = self.root / '.weftext-rules'
        self.assertTrue(rules.exists(), 'New library must have its original admitted default rules')
        changed = json.dumps({'version': 1, 'ignored': ['Shared'], 'unmanaged': []}).encode()
        rules.write_bytes(changed)
        self.unavailable(self.scope_plan('shared'), resources=[])
        code, error, _ = self.server.call('/api/hosted/save', {
            'sessionId': doc['sessionId'], 'baseId': doc['baseId'], 'source': prepared['source'],
            'operationId': uid(), 'definitionReservations': [original['operationId']]}, self.tokens['alice'])
        self.assertIn(code, [404, 409], error)
        self.assertEqual((self.root / 'Other/Other.typ').read_text('utf-8'), doc['source'])
        self.assertEqual(rules.read_bytes(), changed)

    def trash_shared(self):
        def readonly(users):
            for user in users.values():
                for resource in ('Shared', 'Child'):
                    if resource not in user['grants']:
                        continue
                    rights = user['grants'][resource]
                    rights.update(write=False, history=False, download=False)
                    rights.pop('annotations', None)
                    rights.pop('office', None)
                    for field in rights.get('fields', {}).values():
                        field['write'] = False
        self.policy(readonly)
        self.config['users'] = copy.deepcopy(self.call('admin/policy', actor='admin')['users'])
        def trash(api):
            receipt = self.operation_local(api, {'kind': 'trashNode', 'source': str(self.root / 'Shared')})
            target = Path(receipt['path'])
            self.trash_operation = receipt['operationId']
            for resource, leaf in [('Shared', 'Shared.typ'), ('Child', 'Child/Child.typ')]:
                workspace, node = self.refs[resource.lower()].split('://', 1)[1].split('/')
                self.config['files'][resource].update(path=(target / leaf).relative_to(self.root).as_posix(),
                    editable=False, importable=False, lifecycle='trash',
                    expectedNode={'workspaceId': workspace, 'nodeId': node})
            self.config['directories']['shared'].update(
                path=target.relative_to(self.root).as_posix(), lifecycle='trash')
        self.local(trash)

    def test_scope_trash_is_explicit_readonly_and_restore_invalidates_old_qualification(self):
        self.prepare_scopes()
        self.trash_shared()
        self.query(self.scope_plan(lifecycle='trash'), resources=[], status=404)
        self.grant_scopes(['shared'], trash=True)
        rows = self.rows(self.scope_plan('shared', lifecycle='trash'))
        self.assertEqual({row['entityKey'] for row in rows}, {'Shared', 'Child'})
        self.assertTrue(all(row['readOnly'] for row in rows))
        self.assertEqual({row['entityKey'] for row in self.rows(self.scope_plan())}, {'Other'})
        selected = ['Shared', 'Child', 'Other']
        self.assertEqual({row['entityKey'] for row in self.rows(self.scope_plan(), resources=selected)}, {'Other'})
        self.assertEqual({row['entityKey'] for row in self.rows(
            self.scope_plan(lifecycle='trash'), resources=selected)}, {'Shared', 'Child'})
        self.call('open', {'resource': 'Shared'}, status=404)
        self.call('fields/write', {'resource': 'Shared', 'fieldId': 'tasks/status',
            'basis': uid(), 'value': {'kind': 'text', 'value': 'done'}, 'operationId': uid()}, 'fields', 404)
        self.policy(lambda users: [users['fields']['grants'][resource].update(
            office={'export': True, 'download': True}) for resource in ('Shared', 'Child')])
        result = self.query(self.scope_plan('shared', lifecycle='trash'))
        request = {'outputLocale': 'zh-CN', 'requestId': uid(), 'queryId': result['queryId'], 'format': 'xlsx',
                   'reviewMode': 'none', 'includeResolved': False, 'stylePolicy': 'template', 'bindings': {}}
        output = self.call('office/export/prepare', request, 'fields')
        artifact = HostedOffice.deliver(self, output, 'fields')
        self.assertTrue(artifact.startswith(b'PK'))
        self.grant_scopes([], trash=False)
        self.assertEqual(self.call('office/export/status', {'requestId': request['requestId']}, 'fields')['status'], 'expired')
        self.assertEqual({row['entityKey'] for row in self.rows(self.scope_plan())}, {'Other'})
        self.grant_scopes(['shared'], trash=True)
        def restore(api):
            self.operation_local(api, {'kind': 'restoreNode', 'trashOperationId': self.trash_operation,
                                       'parent': str(self.root), 'name': 'Restored'})
        self.local(restore)
        self.unavailable(self.scope_plan('shared', lifecycle='trash'), resources=[])
        self.assertTrue((self.root / 'Restored/Restored.typ').exists())

    def test_scope_live_saved_call_does_not_capture_unused_trash_objects(self):
        self.prepare_scopes()
        definition = self.add_definition(self.scope_plan())
        self.grant_query(resources=('Other',), definitions=True, history=True)
        self.trash_shared()
        self.grant_scopes(trash=True)
        selected = self.root / self.config['files']['Shared']['path']
        held = selected.with_name('OwnedHeldShared.typ')
        for path in (selected, held):
            self.assertTrue(path.resolve().is_relative_to(self.root.resolve()))
        self.assertFalse(held.exists())
        # Preserve the original owned file, but make this unused Trash slot
        # unreadable as a source. Live callees must not capture it at all.
        selected.rename(held)
        selected.mkdir()
        key = definition['definition']['key']
        for selection in ({'mode': 'current', 'key': key},
                          {'mode': 'pinned', 'key': key,
                           'revision': definition['definitionRevision']}):
            plan = {'advanced': {'call': {'owner': 'Other', 'selection': selection}}}
            self.assertEqual([row['entityKey'] for row in self.rows(plan)], ['Other'])
        self.unavailable(self.scope_plan(lifecycle='trash'))
        self.assertTrue(held.is_file())
        self.assertTrue(selected.is_dir())

    def test_scope_revocation_fences_inflight_embedded_delivery(self):
        self.prepare_scopes()
        definition = self.add_definition(self.scope_plan(self.refs['root'], expr='body'))
        self.grant_query(actor='alice', definitions=True, products=['body'])
        self.grant_scopes(['root'], actor='alice')
        doc = self.open('Other')
        slow = doc['source'] + '\n#for i in range(4000) [scope delivery paragraph #parbreak()]'
        basis = {'sessionId': doc['sessionId'], 'source': slow, 'serial': 1}
        found = self.call('query/embedded/discover', basis)
        self.assertTrue(self.call('editor/render', basis)['ok'])
        occurrence = next(item for item in found['occurrences'] if item['key'] == definition['definition']['key'])
        deadline = time.monotonic() + 40
        with ThreadPoolExecutor(max_workers=1) as pool:
            with OwnedWorkerObservation(self.server.owner) as observation:
                self.assertLess(time.monotonic(), deadline, 'Original request/mutation budget exhausted')
                submitted = event_time_ns()
                pending = pool.submit(timed_call, self.server.call, '/api/hosted/query/embedded/run', {**basis, 'occurrenceId': occurrence['id']}, self.tokens['alice'])
                # The whole input set is captured before its first worker.
                # This does not identify a particular row or all workers.
                worker = observation.wait(pending, deadline=deadline)
                observation.assert_inflight(pending)
                self.assertLess(time.monotonic(), deadline, 'Original request/mutation budget exhausted before change')
                self.grant_scopes([], actor='alice')
                changed = event_time_ns()
                (code, result, _), completed = pending.result(timeout=max(0, deadline-time.monotonic()))
        trace = {'submittedMonotonicNs': submitted, 'worker': worker,
                 'changeCompletedMonotonicNs': changed, 'requestCompletedMonotonicNs': completed,
                 'status': code, 'errorCode': result.get('error', {}).get('code'),
                 'explanationKey': result.get('error', {}).get('explanation', {}).get('key')}
        with (self.folder/'scope-revocation-inflight.json').open('x', encoding='utf-8') as evidence:
            evidence.write(json.dumps(trace, ensure_ascii=False, indent=2))
        self.assertLessEqual(submitted, worker['observedMonotonicNs'])
        self.assertLess(worker['observedMonotonicNs'], changed)
        self.assertLess(changed, completed, 'Concurrent change must finish before request delivery')
        self.assertEqual(code, 401, result)
        self.assertEqual(result['error']['code'], 'authentication')
        self.assertEqual(result['error']['explanation']['key'],
                         'error.api.hosted.authenticationExpired')
        self.assertNotIn('queryId', result)


if __name__ == '__main__':
    suite = unittest.TestSuite(HostedScopes(name) for name in
        unittest.defaultTestLoader.getTestCaseNames(HostedScopes) if name.startswith('test_scope_'))
    sys.exit(0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1)
