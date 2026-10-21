"""Actual local HTTP semantic revalidation through the original operation writer.

Two independent programs create every version through public Save/configuration/
sync producers. The reviewed object states and exact cuts come from the real
read-only APIs, never edited graph records or client proof flags. Fixtures,
transcripts and owned-process evidence stay in a fresh explicit output directory.

Run: python scripts/test_sync_revalidation.py EXE [exact_test_name ...]
Set WEFTEXT_SYNC_EVIDENCE_DIR to a new workspace output directory.
"""
from _creation_result import created_document
import copy
import hashlib
import json
import os
from pathlib import Path
import sys
import time
import unittest
from urllib.error import HTTPError
from urllib.request import Request, urlopen

from test_hosted import uid
from test_sync_compound import SyncCompound, evidence_directory, physical_identity, tree_state


CUT = '/api/sync/revalidation-cut'
REVIEW = '/api/sync/revalidation-review'
NATIVE_GOOD = ('#let generated = [精确标题]\n#title(generated)\n'
               '#if false [= 不存在标题]\n#heading(level:12)[实际深标题]\n'
               '#for n in (1,2) [循环#n]\n')
NATIVE_PROVIDER = ("nextCivilDay(title == '精确标题' && body.contains('循环2') "
                   "&& !body.contains('不存在标题') "
                   "&& headings.exists(h, h.level == 12 && h.generated) "
                   "? date('2026-01-01') : date('9999-12-31'))")


class BoundedSemanticCall:
    """Use a finite longer wait for exact semantic prepare and runtime calls."""
    def __init__(self, server):
        self.server = server
        self.evidence_name = server.evidence_name

    def call(self, route, data):
        request = Request(self.server.url + route, data=json.dumps(data).encode(), headers={
            'Content-Type': 'application/json', 'X-Weftext-Token': self.server.token})
        try:
            response = urlopen(request, timeout=90)
        except HTTPError as error:
            response = error
        with response:
            raw = response.read()
            result = json.loads(raw) if 'json' in response.headers.get('Content-Type', '') else raw
            return response.status, result, response.headers


class SyncRevalidation(SyncCompound):
    @classmethod
    def setUpClass(cls):
        if os.name != 'nt':
            raise RuntimeError('This harness requires the Windows candidate and physical identity API')
        cls.evidence = evidence_directory('weftext-sync-revalidation-')
        print('Retained semantic sync evidence:', cls.evidence, flush=True)

    def setUp(self):
        # Keep owned Windows journal paths short; retain the full case name.
        names = sorted(name for name in type(self).__dict__
                       if name.startswith('test_sync_revalidation_'))
        self.folder = self.evidence / f'c{names.index(self._testMethodName) + 1:02d}'
        self.folder.mkdir()
        (self.folder / 'case.json').write_text(
            json.dumps({'test': self._testMethodName}) + '\n', encoding='utf-8')
        self.servers, self.transfer_number = [], 0
        self.a = self.start_server(['--no-browser', '--data-dir', str(self.folder / 'state-a')], 'A')
        self.b = self.start_server(['--no-browser', '--data-dir', str(self.folder / 'state-b')], 'B')
        self.ra, self.rb = self.folder / 'library-a', self.folder / 'library-b'
        self.call(self.a, '/api/library/create', path=str(self.ra), operationId=uid())
        self.with_schema = 'absent_schema' not in self._testMethodName
        if self.with_schema:
            self.install_schema(self.a, self.ra, 'semantic_baseline_title')
        doc = created_document(self.call(self.a, '/api/note/create', root=str(self.ra), parent=str(self.ra),
                        name='Note', operationId=uid()))
        self.key = {'kind': 'node', 'id': doc['nodeId']}
        self.keys = {'Note': self.key}
        if 'missing_annotation_target' in self._testMethodName:
            # This is an actual external authored sidecar before sync enable,
            # captured by that public producer. No graph or admission record is
            # fabricated, and the unresolved typed target gains no authority.
            self.annotation_id = uid()
            self.missing_target = {'workspaceId': doc['workspaceId'], 'nodeId': uid()}
            self.annotation_bytes = (json.dumps({'version': 1, 'annotations': [{
                'annotationId': self.annotation_id, 'replyTo': None, 'attribution': 'author',
                'createdAt': '2026-10-15T00:00:00Z', 'purpose': 'comment',
                'body': 'Preserve original discussion with missing typed target 中文😀',
                'labels': [], 'appearance': {}, 'reviewState': 'open', 'suggestion': None,
                'target': {'kind': 'wholeDocument', 'node': self.missing_target},
            }]}, ensure_ascii=False, indent=2) + '\n').encode()
            (self.ra / 'Note/weftext.annotations.json').write_bytes(self.annotation_bytes)
            (self.folder / 'original-authored-annotations.json').write_bytes(self.annotation_bytes)
        self.binding = self.call(self.a, '/api/sync/enable', root=str(self.ra), operationId=uid())
        self.identity = self.binding['identity']
        objects = self.binding['installedObjects']
        self.assertIn(self.key, objects)
        self.assertEqual({'kind': 'schema'} in objects, self.with_schema)
        self.transfer(self.a, self.ra, self.b, objects)
        clone = self.call(self.b, '/api/operations/prepare', operationId=uid(), root=str(self.rb),
                          intent={'kind': 'cloneSync', 'identity': self.identity, 'objects': objects})
        self.assertFalse(self.rb.exists())
        self.commit(self.b, clone)

        # A genuine B Save provides local retained history and a common base.
        self.save(self.b, self.rb, Path('Note/Note.typ'), 'B retained exact semantic base 中文😀')
        self.transfer(self.b, self.rb, self.a, [self.key])
        common = self.call(self.a, '/api/operations/prepare', operationId=uid(), root=str(self.ra),
                           intent={'kind': 'applySync', 'identity': self.identity, 'objects': [self.key]})
        self.commit(self.a, common)
        sender = self.open(self.a, self.ra, Path('Note/Note.typ'))
        if sender.get('recoveredDraftConflict'):
            self.assertEqual(sender['recoveredDraft'], sender['recoveredDraftBaseSource'])
            self.call(self.a, '/api/draft/resolve', sessionId=sender['sessionId'],
                      action='mergeRecovered', source=sender['source'])
        original = self.open(self.b, self.rb, Path('Note/Note.typ'))
        self.original_source = original['source']
        self.draft = self.original_source + '\nB PRIVATE SEMANTIC DRAFT 🌿'
        self.call(self.b, '/api/draft', sessionId=original['sessionId'],
                  baseId=original['baseId'], source=self.draft)
        timeline = self.call(self.b, '/api/history/list', sessionId=original['sessionId'])
        self.history = {item['versionId']: self.call(
            self.b, '/api/history/read', sessionId=original['sessionId'], versionId=item['versionId'])['source']
            for item in timeline['versions'] if item['status'] == 'available'}
        self.assertTrue(self.history)
        (self.rb / 'Note/personal.raw').write_bytes(b'PRIVATE RAW SEMANTIC OWNER')
        self.before = tree_state(self.rb)

    def install_schema(self, server, root, alias):
        before = self.call(server, '/api/data/schema', root=str(root))
        schema = json.loads(before['source'])
        # Aliases name Types, so this real configuration edit uses the bundled
        # task Type rather than a FieldId such as core/title.
        schema['aliases'][alias] = 'tasks/task'
        source = json.dumps(schema, ensure_ascii=False, indent=2) + '\n'
        plan = self.call(server, '/api/data/schema/prepare', root=str(root), operationId=uid(),
                         source=source, beforeSource=before['source'], beforeRevision=before['revision'])
        receipt = self.call(server, '/api/data/schema/install', operationId=plan['operationId'], planId=plan['planId'])
        self.assertEqual(receipt['status'], 'committed')
        result = self.call(server, '/api/data/schema', root=str(root))
        self.assertEqual(result['source'], source)
        return result

    def call(self, server, route, expected=200, **data):
        semantic_prepare = (route == '/api/operations/prepare'
                            and data.get('intent', {}).get('kind') in ['revalidateSync', 'adoptSyncExternal'])
        if not semantic_prepare and route not in [REVIEW, '/api/query/definition/run', '/api/query/run']:
            return super().call(server, route, expected=expected, **data)
        started = time.monotonic()
        try:
            return super().call(BoundedSemanticCall(server), route, expected=expected, **data)
        finally:
            with (self.folder / 'semantic-http-timings.ndjson').open('a', encoding='utf-8') as log:
                log.write(json.dumps({'program': server.evidence_name, 'route': route,
                                      'operationId': data.get('operationId'), 'timeoutSeconds': 90,
                                      'elapsedSeconds': time.monotonic() - started}) + '\n')

    def current_view(self, server, object_key):
        status = self.call(server, '/api/sync/status', identity=self.identity)
        return next(entry['view'] for entry in status['objects'] if entry['object'] == object_key)

    def changed_schema_received_head(self, source=None):
        if source is None:
            self.saved = self.save(self.a, self.ra, Path('Note/Note.typ'), 'A exact received semantic source 😀')['source']
        else:
            self.saved = source
        self.schema = self.install_schema(self.b, self.rb, 'semantic_current_title')
        self.call(self.b, '/api/sync/publish', root=str(self.rb))
        current = tree_state(self.rb)
        self.transfer(self.a, self.ra, self.b, [self.key])
        self.assertEqual(tree_state(self.rb), current, 'Receive must not change the author tree')
        view = self.current_view(self.b, self.key)
        self.assertEqual(len(view['versions']), 1, 'Explicit revalidation reviews one actual received head')
        # A plain Save does not claim a causal Schema dependency. The user can
        # still request typed revalidation against B's changed current Schema;
        # review and prepare must freeze the real reads they actually perform.
        self.assertEqual(tree_state(self.rb), current)
        return current

    def definition(self):
        doc = self.open(self.a, self.ra, Path('Note/Note.typ'))
        plan = {'select': [{'name': 'closed', 'expr': "nextCivilDay(date('2026-01-01'))"},
                           {'name': 'unknown', 'expr': 'spare'}],
                'advanced': {'parameters': [{'name': 'spare', 'shape': {'kind': 'scalar', 'semantic': 'int'},
                                             'nullable': False}]}}
        result = self.call(self.a, '/api/query/definition/save', sessionId=doc['sessionId'],
                           source=doc['source'], sourceHash=hashlib.sha256(doc['source'].encode()).hexdigest(),
                           serial=1, baseId=doc['baseId'], key=None, label='weftext-query-semantic',
                           plan=plan, operationId=uid())
        self.definition_key = result['definition']['key']
        self.definition_binding = result['definition']['binding']
        self.assertTrue(self.definition_binding['revision'])
        return result['document']

    def native_definition(self, body=NATIVE_GOOD, expression=NATIVE_PROVIDER,
                          unrelated=False, include=False, lifecycle='live', defer_retirement=False):
        """Admit real row sources before saving the real native Query owner.

        The optional raw include is explicitly authored in both owned trees;
        only the Core's actual frozen compiler read can qualify it. Neither
        the source metadata nor the HTTP client supplies compiled products.
        """
        names = ['Task', 'RetiredTask'] if lifecycle == 'mixed' else ['Task']
        if unrelated:
            names.append('Unrelated')
        self.native_keys = {}
        for name in names:
            doc = created_document(self.call(self.a, '/api/note/create', root=str(self.ra), parent=str(self.ra),
                            name=name, operationId=uid()))
            key = {'kind': 'node', 'id': doc['nodeId']}
            self.native_keys[name] = key
            metadata = f'#metadata((id: "{doc["nodeId"]}"'
            if name != 'Unrelated':
                metadata += ', types: ("task",), status: "open"'
            source = metadata + ')) <weftext>\n' + (
                '#undefined-body-function()' if name == 'Unrelated' else body)
            if include and name == 'Task':
                source += '\n#include "../shared.typ"\n'
            self.call(self.a, '/api/save', sessionId=doc['sessionId'], baseId=doc['baseId'],
                      source=source, operationId=uid())
        objects = list(self.native_keys.values())
        self.transfer(self.a, self.ra, self.b, objects)
        self.commit(self.b, self.call(self.b, '/api/operations/prepare', root=str(self.rb),
                    operationId=uid(), intent={'kind': 'applySync', 'identity': self.identity,
                                              'objects': objects}))
        if include:
            self.native_include = self.rb / 'shared.typ'
            self.native_include_bytes = '原始共享输入中文😀\n'.encode()
            (self.ra / 'shared.typ').write_bytes(self.native_include_bytes)
            self.native_include.write_bytes(self.native_include_bytes)
        if lifecycle in ['trash', 'mixed'] and not defer_retirement:
            name = 'RetiredTask' if lifecycle == 'mixed' else 'Task'
            retired = self.call(self.a, '/api/operations/prepare', root=str(self.ra), operationId=uid(),
                                intent={'kind': 'trashNode', 'source': str(self.ra / name)})
            self.commit(self.a, retired)
            self.transfer(self.a, self.ra, self.b, [self.native_keys[name]])
            self.commit(self.b, self.call(self.b, '/api/operations/prepare', root=str(self.rb),
                        operationId=uid(), intent={'kind': 'applySync', 'identity': self.identity,
                                                  'objects': [self.native_keys[name]]}))
            self.assertTrue(self.current_view(self.b, self.native_keys[name])['versions'][0]['state']['retired'])
        plan = {'types': ['task'], 'scope': {'lifecycle': 'trash' if lifecycle == 'trash' else 'live'},
                'select': [{'name': 'actual', 'expr': expression}]}
        if lifecycle == 'mixed':
            trash_plan = copy.deepcopy(plan)
            trash_plan['scope']['lifecycle'] = 'trash'
            plan = {'advanced': {'unionAll': [plan, trash_plan]}}
        doc = self.open(self.a, self.ra, Path('Note/Note.typ'))
        result = self.call(self.a, '/api/query/definition/save', sessionId=doc['sessionId'],
                           source=doc['source'], sourceHash=hashlib.sha256(doc['source'].encode()).hexdigest(),
                           serial=1, baseId=doc['baseId'], key=None, label='weftext-query-native-proof',
                           plan=plan, operationId=uid())
        self.definition_key = result['definition']['key']
        self.definition_binding = result['definition']['binding']
        self.native_plan, self.native_lifecycle = plan, lifecycle
        self.before = tree_state(self.rb)
        (self.folder / 'native-fixture.json').write_text(json.dumps({
            'body': body, 'expression': expression, 'plan': plan,
            'keys': self.native_keys, 'definition': result['definition'],
            'unrelatedInvalidBody': unrelated, 'localRawInclude': include,
        }, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
        if defer_retirement:
            self.assertEqual(lifecycle, 'trash')
            retired = self.call(self.a, '/api/operations/prepare', root=str(self.ra), operationId=uid(),
                                intent={'kind': 'trashNode', 'source': str(self.ra / 'Task')})
            self.commit(self.a, retired)
            self.transfer(self.a, self.ra, self.b, [self.native_keys['Task']])
            self.assertTrue((self.rb / 'Task/Task.typ').is_file(),
                            'The receiver must still be live before the single revalidation request')
        return result['document']

    def native_case(self, **options):
        doc = self.native_definition(**options)
        self.changed_schema_received_head(doc['source'])
        self.resolution()
        self.native_before = tree_state(self.rb)

    def native_review(self):
        result = self.review()
        for key in self.native_keys.values():
            expected = {'object': key, 'revisions': [self.current_view(self.b, key)['versions'][0]['revision']]}
            self.assertIn(expected, result['reviewedVersions'], 'Native row versions belong to the exact reviewed cut')
        self.assertEqual(tree_state(self.rb), self.native_before)
        return result

    def assert_native_runtime(self):
        output = self.call(self.b, '/api/query/definition/run', root=str(self.rb),
                           owner=f'weftext-node://{self.identity["workspaceId"]}/{self.key["id"]}',
                           selection={'mode': 'current', 'key': self.definition_key}, parameters={})
        result = output['result']
        self.assertEqual(result['coverage'], 'complete', 'Actual native runtime coverage is not weakened for this fixture')
        expected_names = ['Task', 'RetiredTask'] if self.native_lifecycle == 'mixed' else ['Task']
        self.assertEqual(len(result['rows']), len(expected_names))
        self.assertEqual({row['entityKey'] for row in result['rows']}, {
            f'weftext-node://{self.identity["workspaceId"]}/{self.native_keys[name]["id"]}'
            for name in expected_names})
        for row in result['rows']:
            self.assertEqual(row['cells']['actual']['value'], {'kind': 'date', 'value': '2026-01-02'})
        if self.native_lifecycle == 'trash':
            self.assertTrue(all(row['readOnly'] and row['lifecycle'] == 'trash' for row in result['rows']))
        if self.native_lifecycle == 'mixed':
            self.assertEqual({row['lifecycle'] for row in result['rows']}, {'live', 'trash'})
            self.assertTrue(all(row['readOnly'] for row in result['rows'] if row['lifecycle'] == 'trash'))

    def assert_native_inputs_preserved(self):
        current = tree_state(self.rb)
        for relative, original in self.native_before.items():
            if relative != 'Note/Note.typ':
                self.assertEqual(current[relative], original, f'Native qualification preserves {relative}')

    def assert_native_review_refused(self):
        result = self.call(self.b, REVIEW, expected=409, root=str(self.rb), identity=self.identity,
                           resolutions=[self.proposal], blobs=self.blobs)
        self.assertEqual(result['error']['code'], 'queryInvocationUnavailable')
        self.assertEqual(tree_state(self.rb), self.native_before)
        self.assertEqual(self.assert_private_and_retained()['source'], self.original_source)

    def bound_native_review(self, choices):
        before = tree_state(self.rb)
        operation_id = uid()
        review = self.call(self.b, REVIEW, root=str(self.rb), identity=self.identity,
                           operationId=operation_id, resolutions=choices, blobs=self.blobs)
        self.assertEqual(review['operationId'], operation_id)
        self.assertEqual(len(review['resolutions']), len(choices))
        for expected in choices:
            actual = next(item for item in review['resolutions'] if item['object'] == expected['object'])
            for field in ['object', 'revisions', 'decision', 'state']:
                self.assertEqual(actual[field], expected[field])
        self.assertEqual(tree_state(self.rb), before)
        self.assertEqual(self.call(self.b, '/api/operations/status', operationId=operation_id)['status'], 'unknown')
        self.reviewed = review
        self.requested = {'operationId': operation_id, 'root': str(self.rb), 'intent': {
            'kind': 'revalidateSync', 'identity': self.identity,
            'expectedInstalled': review['expectedInstalled'], 'reviewedVersions': review['reviewedVersions'],
            'resolutions': review['resolutions'], 'blobs': copy.deepcopy(self.blobs)}}
        return review

    def receive_native_owner(self, source):
        self.saved = source
        self.transfer(self.a, self.ra, self.b, [self.key])
        self.schema = self.call(self.b, '/api/data/schema', root=str(self.rb))
        self.resolution()
        self.native_before = tree_state(self.rb)

    def native_independent_directory(self):
        directory = self.ra / 'Independent'
        directory.mkdir()
        admitted = self.call(self.a, '/api/navigation/admit-directory', root=str(self.ra),
                             path=str(directory), operationId=uid())
        self.commit(self.a, admitted['preview'])
        key = {'kind': 'directory', 'id': json.loads((directory / 'weftext.directory.json').read_bytes())['directoryId']}
        self.transfer(self.a, self.ra, self.b, [key])
        self.commit(self.b, self.call(self.b, '/api/operations/prepare', root=str(self.rb), operationId=uid(),
                    intent={'kind': 'applySync', 'identity': self.identity, 'objects': [key]}))
        return key

    def install_native_style(self):
        original = self.call(self.a, '/api/styles/read', root=str(self.ra))
        prepared = self.call(self.a, '/api/styles/prepare', root=str(self.ra), operationId=uid(),
                             baseId=original['baseId'], source=original['source'] + '\n// Actual native style dependency\n',
                             enabled=True)
        styled = self.call(self.a, '/api/styles/apply', root=str(self.ra), operationId=prepared['operationId'],
                           planId=prepared['planId'])
        self.assertTrue(styled['enabled'])
        self.assertEqual(styled['operationReceipt']['status'], 'committed')
        module = Path(styled['path'])
        owner = self.open(self.a, self.ra, module.parent.relative_to(self.ra) / 'Styles.typ')
        self.style_key = {'kind': 'node', 'id': owner['nodeId']}
        keys = [self.style_key, {'kind': 'settings'}]
        self.transfer(self.a, self.ra, self.b, keys)
        self.commit(self.b, self.call(self.b, '/api/operations/prepare', root=str(self.rb), operationId=uid(),
                    intent={'kind': 'applySync', 'identity': self.identity, 'objects': keys}))
        self.style_relative = module.relative_to(self.ra)
        self.assertEqual((self.rb / self.style_relative).read_bytes(), module.read_bytes())
        self.assertNotEqual(physical_identity(self.rb / self.style_relative), physical_identity(module))

    def native_adoption_case(self, include=False, invalid_provider=False, change_task=False):
        # A real earlier commit installs the admitted definition. The operation
        # tested below is exclusively AdoptSyncExternal over current raw bytes;
        # neither a received-head decision nor a semantic-review response can
        # stand in for the physical adoption-cut and its original snapshot bases.
        self.native_case(include=include)
        self.native_review()
        self.prepare_revalidation()
        self.commit(self.b, self.plan)
        self.assert_applied()
        self.native_adoption_sender_tree = tree_state(self.ra)
        self.native_adoption_predecessors = {
            name: copy.deepcopy(self.current_view(self.b, key)['versions'][0])
            for name, key in {'Note': self.key, **self.native_keys}.items()}
        self.external_source = self.expected_source + '\n真实外部采纳正文 中文😀\n'
        if invalid_provider:
            self.assertEqual(self.external_source.count("date('2026-01-01')"), 1)
            self.external_source = self.external_source.replace(
                "date('2026-01-01')", "date('9999-12-31')")
        (self.rb / 'Note/Note.typ').write_bytes(self.external_source.encode())
        self.external_paths = {'Note/Note.typ': self.external_source.encode()}
        if change_task:
            task_path = self.rb / 'Task/Task.typ'
            # The final generated body is part of the adopted after image. The
            # actual provider still consumes its title, loop2 and deep heading.
            self.external_paths['Task/Task.typ'] = task_path.read_bytes() + (
                '\n#for n in (3,) [外部新增循环#n]\n').encode()
            task_path.write_bytes(self.external_paths['Task/Task.typ'])
        self.external_tree = tree_state(self.rb)
        binding = self.call(self.b, '/api/sync/binding', root=str(self.rb))
        objects = binding['installedObjects']
        self.assertTrue(all(key in objects for key in [self.key, *self.native_keys.values()]))
        cut = self.call(self.b, '/api/sync/adoption-cut', root=str(self.rb),
                        identity=self.identity, objects=objects)
        self.assertEqual(cut['identity'], self.identity)
        self.assertEqual({json.dumps(dep['object'], sort_keys=True) for dep in cut['expectedInstalled']},
                         {json.dumps(key, sort_keys=True) for key in objects})
        self.assertEqual(len(cut['changes']), len(self.external_paths))
        for change in cut['changes']:
            relative = Path(change['path']).relative_to(self.rb).as_posix()
            self.assertEqual(bytes(change['before']), self.external_paths[relative])
            self.assertEqual(change['before'], change['after'])
            self.assertTrue(change['baseId'])
        self.adoption_cut = cut
        self.adoption_request = {'operationId': uid(), 'root': str(self.rb),
                                 'intent': {'kind': 'adoptSyncExternal', **cut}}
        self.assertEqual(tree_state(self.rb), self.external_tree)
        self.assertEqual(self.assert_private_and_retained()['source'], self.external_source)

    def prepare_native_adoption(self):
        plan = self.call(self.b, '/api/operations/prepare', **self.adoption_request)
        self.assertEqual(plan['kind'], 'adoptSyncExternal')
        self.assertEqual(plan['status'], 'prepared')
        self.assertEqual(plan['operationId'], self.adoption_request['operationId'])
        self.assertEqual(tree_state(self.rb), self.external_tree,
                         'Native adoption prepare must preserve all observed bytes and physical identities')
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.adoption_request), plan)
        self.assertEqual(self.call(self.b, '/api/sync/operation', operationId=plan['operationId']),
                         {'request': self.adoption_request, 'plan': plan})
        self.adoption_plan = plan
        return plan

    def assert_native_adoption_originals(self, expected_tree=None):
        self.assertEqual(tree_state(self.rb), self.external_tree if expected_tree is None else expected_tree)
        self.assertEqual(tree_state(self.ra), self.native_adoption_sender_tree)
        self.assertEqual(self.assert_private_and_retained()['source'], self.external_source)
        for relative, expected in self.external_paths.items():
            self.assertEqual((self.rb / relative).read_bytes(), expected)
        for name, key in {'Note': self.key, **self.native_keys}.items():
            predecessor = self.native_adoption_predecessors[name]
            relative = f'{name}.typ'
            retained = self.call(self.b, '/api/sync/file', identity=self.identity, object=key,
                                 revision=predecessor['revision'], path=relative)
            self.assertEqual(retained['digest'], predecessor['state']['files'][relative])
            self.assertEqual(hashlib.sha256(retained['text'].encode()).hexdigest(), retained['digest'])

    def assert_native_final_products(self):
        self.assert_native_runtime()
        result = self.call(self.b, '/api/query/run', root=str(self.rb), parameters={}, plan={
            'types': ['task'], 'select': [{'name': 'title', 'expr': 'title'},
                                        {'name': 'body', 'expr': 'body'},
                                        {'name': 'headings', 'expr': 'headings'}]})
        self.assertEqual(result['coverage'], 'complete')
        self.assertEqual(len(result['rows']), 1)
        self.assertEqual(result['rows'][0]['entityKey'],
                         f'weftext-node://{self.identity["workspaceId"]}/{self.native_keys["Task"]["id"]}')
        cells = result['rows'][0]['cells']
        self.assertEqual(cells['title']['value'], {'kind': 'text', 'value': '精确标题'})
        self.assertIn('循环2', cells['body']['value']['value'])
        self.assertNotIn('不存在标题', cells['body']['value']['value'])
        self.assertIn('外部新增循环3', cells['body']['value']['value'])
        self.assertTrue(any(heading['kind'] == 'record'
                            and {'key': 'level', 'value': {'kind': 'int', 'value': '12'}} in heading['value']
                            and {'key': 'generated', 'value': {'kind': 'bool', 'value': True}} in heading['value']
                            for heading in cells['headings']['value']['value']))

    def removed_member_case(self):
        # Register the resource with the real importer and install that exact
        # predecessor at both replicas before observing an external removal.
        self.removed_bytes = b'original registered member bytes'
        self.import_resource(self.a, self.ra, Path('Note/Note.typ'), 'asset.bin', self.removed_bytes)
        self.transfer(self.a, self.ra, self.b, [self.key])
        self.commit(self.b, self.call(self.b, '/api/operations/prepare', operationId=uid(), root=str(self.rb),
                    intent={'kind': 'applySync', 'identity': self.identity, 'objects': [self.key]}))
        heads = self.current_view(self.a, self.key)['versions']
        self.assertEqual(len(heads), 1)
        self.removed_predecessor = copy.deepcopy(heads[0])
        self.assertIn('asset.bin', self.removed_predecessor['state']['files'])
        sender = self.open(self.a, self.ra, Path('Note/Note.typ'))
        self.removed_source = sender['source']
        self.removed_draft = self.removed_source + '\nA PRIVATE MEMBER REMOVAL DRAFT 🌿'
        self.call(self.a, '/api/draft', sessionId=sender['sessionId'], baseId=sender['baseId'], source=self.removed_draft)
        history = self.call(self.a, '/api/history/list', sessionId=sender['sessionId'])
        self.removed_history = {item['versionId']: self.call(
            self.a, '/api/history/read', sessionId=sender['sessionId'], versionId=item['versionId'])['source']
            for item in history['versions'] if item['status'] == 'available'}
        self.assertTrue(self.removed_history)
        (self.ra / 'Note/member-private.raw').write_bytes(b'ORIGINAL SENDER RAW')
        self.removed_asset = self.ra / 'Note/asset.bin'
        self.removed_identity = physical_identity(self.removed_asset)
        self.removed_retained = self.folder / 'retained-original-asset.bin'
        self.removed_manifest = self.ra / 'Note/weftext.resources.json'
        manifest_identity = physical_identity(self.removed_manifest)
        self.retain_original(self.removed_asset, self.removed_retained)
        self.assertEqual(physical_identity(self.removed_retained), self.removed_identity)
        manifest = json.loads(self.removed_manifest.read_bytes())
        original_entries = len(manifest['resources'])
        manifest['resources'] = [entry for entry in manifest['resources'] if entry['path'] != 'asset.bin']
        self.assertEqual(len(manifest['resources']), original_entries - 1)
        self.removed_manifest.write_bytes((json.dumps(manifest, ensure_ascii=False, indent=2) + '\n').encode())
        self.assertEqual(physical_identity(self.removed_manifest), manifest_identity)
        self.removed_observed = tree_state(self.ra)
        self.removed_receiver = tree_state(self.rb)
        binding = self.call(self.a, '/api/sync/binding', root=str(self.ra))
        cut = self.call(self.a, '/api/sync/adoption-cut', root=str(self.ra), identity=self.identity,
                        objects=binding['installedObjects'])
        self.assertEqual(cut['identity'], self.identity)
        self.assertEqual(len(cut['removedMembers']), 1)
        member = cut['removedMembers'][0]
        self.assertEqual(Path(member['path']), self.removed_asset)
        self.assertTrue(member['baseId'])
        self.assertEqual(set(member), {'path', 'baseId'}, 'An absent member has no fabricated after bytes')
        self.assertEqual(len(cut['changes']), 1)
        self.assertEqual(Path(cut['changes'][0]['path']), self.removed_manifest)
        self.assertEqual(bytes(cut['changes'][0]['before']), self.removed_manifest.read_bytes())
        self.assertEqual(cut['changes'][0]['before'], cut['changes'][0]['after'])
        self.removed_cut = cut
        self.removed_request = {'operationId': uid(), 'root': str(self.ra),
                                'intent': {'kind': 'adoptSyncExternal', **cut}}
        (self.folder / 'removed-member-fixture.json').write_text(json.dumps({
            'predecessor': self.removed_predecessor, 'cut': cut, 'request': self.removed_request,
            'originalPhysicalIdentity': self.removed_identity, 'retainedOriginal': str(self.removed_retained),
            'observedSenderTree': self.removed_observed, 'unchangedReceiverTree': self.removed_receiver,
        }, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
        self.assert_removed_originals()

    def assert_removed_originals(self, sender_tree=None):
        self.assertEqual(tree_state(self.ra), self.removed_observed if sender_tree is None else sender_tree)
        self.assertEqual(tree_state(self.rb), self.removed_receiver)
        self.assertEqual(self.removed_retained.read_bytes(), self.removed_bytes)
        self.assertEqual(physical_identity(self.removed_retained), self.removed_identity)
        sender = self.open(self.a, self.ra, Path('Note/Note.typ'))
        self.assertEqual(sender['source'], self.removed_source)
        self.assertEqual(sender['recoveredDraft'], self.removed_draft)
        for version, source in self.removed_history.items():
            self.assertEqual(self.call(self.a, '/api/history/read', sessionId=sender['sessionId'],
                                       versionId=version)['source'], source)
        receiver = self.open(self.b, self.rb, Path('Note/Note.typ'))
        self.assertEqual(receiver['source'], self.original_source)
        self.assertEqual(receiver['recoveredDraft'], self.draft)
        for version, source in self.history.items():
            self.assertEqual(self.call(self.b, '/api/history/read', sessionId=receiver['sessionId'],
                                       versionId=version)['source'], source)
        original = self.call(self.a, '/api/sync/file', identity=self.identity, object=self.key,
                             revision=self.removed_predecessor['revision'], path='asset.bin')
        self.assertEqual(original['text'].encode(), self.removed_bytes)

    def restart_a(self):
        self.close_server(self.a)
        self.a = self.start_server(['--no-browser', '--data-dir', str(self.folder / 'state-a')], 'A-restart')

    def new_received_source(self, suffix):
        self.saved = self.save(self.a, self.ra, Path('Note/Note.typ'), suffix)['source']
        self.transfer(self.a, self.ra, self.b, [self.key])

    def resolution(self, repair_source=None):
        current = tree_state(self.rb)
        cut = self.call(self.b, CUT, root=str(self.rb), identity=self.identity, objects=[self.key])
        self.assertEqual(cut['coverage'], 'complete')
        row = next(row for row in cut['objects'] if row['object'] == self.key)
        self.assertEqual(len(row['view']['versions']), 1)
        head = row['view']['versions'][0]
        retained = self.call(self.b, '/api/sync/file', identity=self.identity, object=self.key,
                             revision=head['revision'], path='Note.typ')
        self.assertEqual(retained['text'], self.saved)
        self.received_head = copy.deepcopy(head)
        state = copy.deepcopy(head['state'])
        files = [entry for entry in cut['files'] if entry['object'] == self.key
                 and entry['objectRevision'] == head['revision'] and entry['relativePath'] == 'Note.typ']
        self.assertEqual(len(files), 1)
        self.assertEqual(files[0]['fileRevision'], state['fileRevisions']['Note.typ'])
        self.assertEqual(files[0]['digest'], hashlib.sha256(self.saved.encode()).hexdigest())
        self.expected_source, self.blobs = self.saved, {}
        decision = {'kind': 'preserve', 'revision': head['revision']}
        if repair_source is not None:
            self.expected_source = repair_source
            digest = hashlib.sha256(repair_source.encode()).hexdigest()
            state['files']['Note.typ'] = digest
            state['fileRevisions']['Note.typ'] = uid()
            self.blobs[digest] = list(repair_source.encode())
            decision = {'kind': 'repair', 'baseRevision': head['revision']}
        self.proposal = {'object': self.key, 'revisions': [head['revision']], 'decision': decision,
                         'state': state, 'reviewedDependencies': []}
        self.assertEqual(tree_state(self.rb), current)
        return cut

    def review(self):
        current = tree_state(self.rb)
        result = self.call(self.b, REVIEW, root=str(self.rb), identity=self.identity,
                           resolutions=[self.proposal], blobs=self.blobs)
        self.assertEqual(len(result['resolutions']), 1)
        actual = result['resolutions'][0]
        for field in ['object', 'revisions', 'decision', 'state']:
            self.assertEqual(actual[field], self.proposal[field])
        self.assertTrue(actual['reviewedDependencies'])
        self.assertEqual(tree_state(self.rb), current, 'Review must have no author namespace or content effect')
        self.reviewed = result
        self.requested = {'operationId': uid(), 'root': str(self.rb), 'intent': {
            'kind': 'revalidateSync', 'identity': self.identity,
            'expectedInstalled': result['expectedInstalled'], 'reviewedVersions': result['reviewedVersions'],
            'resolutions': result['resolutions'], 'blobs': self.blobs}}
        return result

    def prepare_revalidation(self):
        before = tree_state(self.rb)
        plan = self.call(self.b, '/api/operations/prepare', **self.requested)
        self.assertEqual(plan['kind'], 'revalidateSync')
        self.assertEqual(plan['status'], 'prepared')
        self.assertEqual(plan['operationId'], self.requested['operationId'])
        self.assertEqual(tree_state(self.rb), before)
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.requested), plan)
        retained = self.call(self.b, '/api/sync/operation', operationId=plan['operationId'])
        self.assertEqual(retained['request'], self.requested)
        self.assertEqual(retained['plan'], plan)
        self.plan = plan
        return plan

    def assert_private_and_retained(self):
        doc = self.open(self.b, self.rb, Path('Note/Note.typ'))
        self.assertEqual(doc['recoveredDraft'], self.draft)
        self.assertEqual((self.rb / 'Note/personal.raw').read_bytes(), b'PRIVATE RAW SEMANTIC OWNER')
        for relative in ['Note', 'Note/Note.typ', 'Note/personal.raw']:
            self.assertEqual(physical_identity(self.rb / relative), self.before[relative][1])
        for version, original in self.history.items():
            self.assertEqual(self.call(self.b, '/api/history/read', sessionId=doc['sessionId'],
                                       versionId=version)['source'], original)
        retained = self.call(self.b, '/api/sync/file', identity=self.identity, object=self.key,
                             revision=self.received_head['revision'], path='Note.typ')
        self.assertEqual(retained['text'], self.saved)
        return doc

    def assert_applied(self):
        doc = self.assert_private_and_retained()
        self.assertEqual(doc['source'], self.expected_source)
        self.assertTrue(doc['recoveredDraftConflict'])
        # The committed original Plan owns publication; make that retained
        # publication visible to the read-only received graph before querying it.
        self.call(self.b, '/api/sync/publish', root=str(self.rb))
        head = self.current_view(self.b, self.key)['versions']
        self.assertEqual(len(head), 1)
        self.assertNotEqual(head[0]['revision'], self.received_head['revision'])
        self.assertEqual(head[0]['state']['files'], self.proposal['state']['files'])
        self.assertEqual(head[0]['state']['fileRevisions'], self.proposal['state']['fileRevisions'])
        self.assertEqual(self.call(self.b, '/api/sync/file', identity=self.identity, object=self.key,
                                   revision=head[0]['revision'], path='Note.typ')['text'], self.expected_source)
        self.assertEqual((self.ra / 'Note/Note.typ').read_bytes(), self.saved.encode())
        if self.with_schema:
            self.assertEqual((self.rb / '.weftext-schema.json').read_bytes(), self.schema['source'].encode())

    def assert_stale_without_effect(self, prepared=False):
        current = tree_state(self.rb)
        if prepared:
            self.commit(self.b, self.plan, expected=409)
            retained = self.call(self.b, '/api/sync/operation', operationId=self.plan['operationId'])
            self.assertEqual(retained['request'], self.requested)
            self.assertEqual(retained['plan'], self.plan)
        else:
            self.call(self.b, '/api/operations/prepare', expected=409, **self.requested)
        self.assertEqual(tree_state(self.rb), current, 'A changed reviewed cut must refuse every author effect')
        self.assertEqual(self.assert_private_and_retained()['source'], self.original_source)

    def test_sync_revalidation_single_head_preserve_exact_dependencies_and_file_revision(self):
        self.changed_schema_received_head()
        self.resolution()
        review = self.review()
        schema = self.current_view(self.b, {'kind': 'schema'})
        schema_dependency = {'object': {'kind': 'schema'}, 'revisions': [schema['versions'][0]['revision']]}
        self.assertIn(schema_dependency, review['resolutions'][0]['reviewedDependencies'])
        self.prepare_revalidation()
        receipt = self.commit(self.b, self.plan)
        self.assert_applied()
        installed = tree_state(self.rb)
        self.assertEqual(self.commit(self.b, self.plan), receipt)
        self.assertEqual(tree_state(self.rb), installed)

    def test_sync_revalidation_explicit_repair_preserves_original_and_allows_unresolved_ordinary_link(self):
        self.changed_schema_received_head()
        source = self.saved + f'\n#link("weftext-node://{self.identity["workspaceId"]}/{uid()}")[ordinary unresolved link]\n明确修复正文😀\n'
        self.resolution(source)
        self.assertNotEqual(self.proposal['state']['fileRevisions']['Note.typ'],
                            self.received_head['state']['fileRevisions']['Note.typ'])
        self.review()
        self.prepare_revalidation()
        self.commit(self.b, self.plan)
        self.assert_applied()

    def test_sync_revalidation_original_request_and_committed_receipt_survive_restart(self):
        self.changed_schema_received_head()
        self.resolution()
        self.review()
        self.prepare_revalidation()
        self.restart_b()
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.requested), self.plan)
        # The actual response is ignored by this simulated HTTP client; no new
        # request or operation is prepared after the committed process restarts.
        receipt = self.commit(self.b, self.plan)
        installed = tree_state(self.rb)
        self.restart_b()
        status = self.call(self.b, '/api/operations/status', operationId=self.plan['operationId'])
        self.assertEqual(status, receipt)
        self.assertEqual(self.commit(self.b, self.plan), receipt)
        self.assertEqual(tree_state(self.rb), installed)
        self.assert_applied()

    def test_sync_revalidation_late_received_head_invalidates_original_review(self):
        self.changed_schema_received_head()
        self.resolution()
        self.review()
        original_head = self.received_head['revision']
        self.new_received_source('A newer received head after the exact review')
        self.assertNotEqual(self.current_view(self.b, self.key)['versions'][0]['revision'], original_head)
        # Retained assertions continue to read the exact previously reviewed
        # bytes, rather than substituting the newer received source.
        latest = self.saved
        self.saved = self.call(self.b, '/api/sync/file', identity=self.identity, object=self.key,
                               revision=original_head, path='Note.typ')['text']
        self.assertNotEqual(self.saved, latest)
        self.assert_stale_without_effect()

    def test_sync_revalidation_late_received_head_invalidates_original_plan(self):
        self.changed_schema_received_head()
        self.resolution()
        self.review()
        self.prepare_revalidation()
        original = self.saved
        self.new_received_source('A newer head after original Plan preparation')
        self.saved = original
        self.assert_stale_without_effect(prepared=True)

    def test_sync_revalidation_absent_schema_appearing_after_review_is_not_silently_selected(self):
        self.new_received_source('A exact source with originally absent author Schema')
        self.assertFalse((self.rb / '.weftext-schema.json').exists())
        self.resolution()
        self.review()
        self.schema = self.install_schema(self.b, self.rb, 'semantic_late_schema_title')
        self.call(self.b, '/api/sync/publish', root=str(self.rb))
        self.assert_stale_without_effect()

    def test_sync_revalidation_absent_schema_appearing_after_prepare_refuses_original_commit(self):
        self.new_received_source('A exact source with absent Schema at original prepare')
        self.assertFalse((self.rb / '.weftext-schema.json').exists())
        self.resolution()
        self.review()
        self.prepare_revalidation()
        self.schema = self.install_schema(self.b, self.rb, 'semantic_late_schema_title')
        self.call(self.b, '/api/sync/publish', root=str(self.rb))
        self.assert_stale_without_effect(prepared=True)

    def test_sync_revalidation_required_parameter_allows_valid_deferred_definition_without_invented_values(self):
        doc = self.definition()
        self.changed_schema_received_head(doc['source'])
        self.resolution()
        self.review()
        self.prepare_revalidation()
        self.commit(self.b, self.plan)
        self.assert_applied()
        # A value is supplied only for this separate, explicit runtime call;
        # semantic review and prepare above never fabricate a root parameter.
        result = self.call(self.b, '/api/query/definition/run', root=str(self.rb),
                           owner=f'weftext-node://{self.identity["workspaceId"]}/{self.key["id"]}',
                           selection={'mode': 'current', 'key': self.definition_key},
                           parameters={'spare': {'kind': 'int', 'value': '7'}})
        self.assertEqual(result['result']['coverage'], 'complete')
        self.assertTrue(result['result']['rows'])
        for row in result['result']['rows']:
            self.assertEqual(row['cells']['unknown']['value'], {'kind': 'int', 'value': '7'})
            self.assertEqual(row['cells']['closed']['value'], {'kind': 'date', 'value': '2026-01-02'})

    def test_sync_revalidation_required_parameter_cannot_hide_closed_invalid_provider_expression(self):
        doc = self.definition()
        source = doc['source'].replace('2026-01-01', '9999-12-31')
        self.assertNotEqual(source, doc['source'])
        self.call(self.a, '/api/save', sessionId=doc['sessionId'], baseId=doc['baseId'], source=source, operationId=uid())
        self.changed_schema_received_head(source)
        self.resolution()
        current = tree_state(self.rb)
        result = self.call(self.b, REVIEW, expected=409, root=str(self.rb), identity=self.identity,
                           resolutions=[self.proposal], blobs=self.blobs)
        self.assertEqual(result['error']['code'], 'queryInvocationUnavailable')
        self.assertEqual(tree_state(self.rb), current)
        self.assertEqual(self.assert_private_and_retained()['source'], self.original_source)

    def test_sync_revalidation_missing_annotation_target_preserves_original_typed_unavailable(self):
        self.changed_schema_received_head()
        self.resolution()
        review = self.review()
        targets = [item for item in review['annotationTargets'] if item['annotationId'] == self.annotation_id]
        self.assertEqual(len(targets), 1)
        self.assertEqual(targets[0]['owner'], {'workspaceId': self.identity['workspaceId'],
                                            'nodeId': self.key['id']})
        self.assertEqual(targets[0]['state'], 'unavailable')
        self.assertEqual(targets[0]['reason'], 'preserved-awaiting-exact-target-qualification')
        self.prepare_revalidation()
        losses = [loss for loss in self.plan['losses'] if loss['code'] == 'annotationTargetUnavailable']
        self.assertEqual(len(losses), 1)
        self.assertEqual(losses[0]['path'],
                         f'weftext-node://{self.identity["workspaceId"]}/{self.key["id"]}')
        self.assertIn(self.annotation_id, losses[0]['detail'])
        self.assertIn('preserved-target-not-in-authorized-read-cut', losses[0]['detail'])
        self.commit(self.b, self.plan)
        self.assert_applied()
        self.assertEqual((self.rb / 'Note/weftext.annotations.json').read_bytes(), self.annotation_bytes)
        self.assertEqual((self.ra / 'Note/weftext.annotations.json').read_bytes(), self.annotation_bytes)
        self.assertEqual(physical_identity(self.rb / 'Note/weftext.annotations.json'),
                         self.before['Note/weftext.annotations.json'][1])
        annotations = json.loads(self.annotation_bytes)
        self.assertEqual(annotations['annotations'][0]['target']['node'], self.missing_target)

    def test_sync_revalidation_native_live_title_deep_heading_body_drive_actual_provider(self):
        self.native_case()
        self.native_review()
        self.prepare_revalidation()
        self.commit(self.b, self.plan)
        self.assert_applied()
        self.assert_native_inputs_preserved()
        self.assert_native_runtime()

    def test_sync_revalidation_native_unrelated_invalid_body_does_not_replace_selected_task_rows(self):
        self.native_case(unrelated=True)
        self.native_review()
        self.prepare_revalidation()
        self.commit(self.b, self.plan)
        self.assert_applied()
        self.assert_native_inputs_preserved()
        self.assertIn('#undefined-body-function()', (self.rb / 'Unrelated/Unrelated.typ').read_text())
        # The actual semantic invocation is Task-typed. This does not claim a
        # complete general-purpose Query over the independently invalid row.

    def test_sync_revalidation_native_wrong_final_title_refuses_provider_not_static_empty_result(self):
        self.native_case(body=NATIVE_GOOD.replace('精确标题', '错误标题'))
        self.assert_native_review_refused()

    def test_sync_revalidation_native_ambiguous_document_and_native_title_refuse_actual_read(self):
        self.native_case(body='#set document(title:[早期标题])\n#title()\n'
                              '#set document(title:[后期元标题])\n正文', expression='title')
        self.assert_native_review_refused()

    def test_sync_revalidation_native_frame_body_unavailable_refuses_direct_body_read(self):
        self.native_case(body='#html.frame[无法承诺流式文字]', expression='body')
        self.assert_native_review_refused()

    def test_sync_revalidation_native_compile_failure_refuses_direct_title_read(self):
        self.native_case(body='#undefined-body-function()', expression='title')
        self.assert_native_review_refused()

    def test_sync_revalidation_native_prepared_include_same_bytes_new_identity_refuses_after_restart(self):
        self.native_case(include=True)
        self.native_review()
        self.prepare_revalidation()
        self.restart_b()
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.requested), self.plan)
        original_identity = physical_identity(self.native_include)
        retained = self.folder / 'retained-original-shared.typ'
        self.retain_original(self.native_include, retained)
        self.native_include.write_bytes(self.native_include_bytes)
        self.assertNotEqual(physical_identity(self.native_include), original_identity)
        self.assertEqual(retained.read_bytes(), self.native_include.read_bytes())
        before = tree_state(self.rb)
        refused = self.commit(self.b, self.plan, expected=409)
        self.assertEqual(refused['error']['code'], 'conflict')
        self.assertEqual(tree_state(self.rb), before)
        self.assertEqual(self.call(self.b, '/api/sync/operation', operationId=self.plan['operationId']),
                         {'request': self.requested, 'plan': self.plan})
        self.assertEqual(self.assert_private_and_retained()['source'], self.original_source)

    def test_sync_revalidation_native_prepared_include_late_bytes_preserve_both_originals(self):
        self.native_case(include=True)
        self.native_review()
        self.prepare_revalidation()
        self.native_include.write_bytes(self.native_include_bytes + '晚到的原生依赖输入\n'.encode())
        before = tree_state(self.rb)
        refused = self.commit(self.b, self.plan, expected=409)
        self.assertEqual(refused['error']['code'], 'conflict')
        self.assertEqual(tree_state(self.rb), before)
        self.assertEqual(self.assert_private_and_retained()['source'], self.original_source)
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.requested), self.plan)

    def test_sync_revalidation_native_prepared_absent_settings_appearance_refuses_original_plan(self):
        self.native_case()
        settings = self.rb / '.weftext-settings.json'
        self.assertFalse(settings.exists())
        self.native_review()
        self.prepare_revalidation()
        original = b'{"version":1}\n'
        settings.write_bytes(original)
        before = tree_state(self.rb)
        refused = self.commit(self.b, self.plan, expected=409)
        self.assertEqual(refused['error']['code'], 'scopeChanged')
        self.assertEqual(tree_state(self.rb), before)
        self.assertEqual(settings.read_bytes(), original)
        self.assertEqual(self.assert_private_and_retained()['source'], self.original_source)

    def test_sync_revalidation_native_late_received_task_refuses_review_and_preserves_original_request(self):
        self.native_case()
        self.native_review()
        original = copy.deepcopy(self.requested)
        self.save(self.a, self.ra, Path('Task/Task.typ'), 'late actual native row')
        self.transfer(self.a, self.ra, self.b, [self.native_keys['Task']])
        before = tree_state(self.rb)
        refused = self.call(self.b, '/api/operations/prepare', expected=409, **original)
        self.assertEqual(refused['error']['code'], 'syncConflictChanged')
        self.assertEqual(self.requested, original)
        self.assertEqual(tree_state(self.rb), before)
        status = self.call(self.b, '/api/operations/status', operationId=original['operationId'])
        self.assertEqual(status['status'], 'unknown')
        self.assertEqual(self.assert_private_and_retained()['source'], self.original_source)

    def test_sync_revalidation_native_cancelled_original_retries_same_plan_and_rejects_changed_request(self):
        self.native_case()
        self.native_review()
        self.prepare_revalidation()
        changed = copy.deepcopy(self.requested)
        changed['intent']['resolutions'][0]['decision']['revision'] = uid()
        before = tree_state(self.rb)
        refused = self.call(self.b, '/api/operations/prepare', expected=409, **changed)
        self.assertEqual(refused['error']['code'], 'operationConflict')
        resolved = self.call(self.b, '/api/sync/cancel-prepared', operationId=self.plan['operationId'],
                             planId=self.plan['planId'])
        self.assertEqual(resolved['status'], 'resolved')
        self.assertEqual(resolved['planId'], self.plan['planId'])
        self.restart_b()
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.requested), resolved)
        self.assertEqual(self.call(self.b, '/api/sync/operation', operationId=self.plan['operationId']),
                         {'request': self.requested, 'plan': resolved})
        self.assertEqual(tree_state(self.rb), before)
        refused = self.commit(self.b, self.plan, expected=409)
        self.assertEqual(refused['error']['code'], 'recoveryPending')
        self.assertEqual(tree_state(self.rb), before)
        self.assertEqual(self.assert_private_and_retained()['source'], self.original_source)

    def test_sync_revalidation_native_trash_products_use_original_retired_rows(self):
        self.native_case(lifecycle='trash')
        self.native_review()
        self.prepare_revalidation()
        self.commit(self.b, self.plan)
        self.assert_applied()
        self.assert_native_inputs_preserved()
        self.assert_native_runtime()

    def test_sync_revalidation_native_mixed_live_trash_products_keep_both_row_lifecycles(self):
        self.native_case(lifecycle='mixed')
        self.native_review()
        self.prepare_revalidation()
        self.commit(self.b, self.plan)
        self.assert_applied()
        self.assert_native_inputs_preserved()
        self.assert_native_runtime()

    def test_sync_revalidation_native_adopt_external_after_products_keep_exact_bytes_and_original_restart(self):
        self.native_adoption_case(change_task=True)
        self.prepare_native_adoption()
        changed = copy.deepcopy(self.adoption_request)
        changed['intent']['changes'][0]['after'].append(10)
        refused = self.call(self.b, '/api/operations/prepare', expected=409, **changed)
        self.assertEqual(refused['error']['code'], 'operationConflict')
        self.assert_native_adoption_originals()
        self.restart_b()
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.adoption_request),
                         self.adoption_plan)
        receipt = self.commit(self.b, self.adoption_plan)
        self.assert_native_adoption_originals()
        self.call(self.b, '/api/sync/publish', root=str(self.rb))
        for name, key in {'Note': self.key, **self.native_keys}.items():
            heads = self.current_view(self.b, key)['versions']
            self.assertEqual(len(heads), 1)
            self.assertNotEqual(heads[0]['revision'], self.native_adoption_predecessors[name]['revision'])
            relative = f'{name}.typ'
            self.assertEqual(heads[0]['state']['files'][relative],
                             hashlib.sha256(self.external_paths[f'{name}/{relative}']).hexdigest())
            exact = self.call(self.b, '/api/sync/file', identity=self.identity, object=key,
                              revision=heads[0]['revision'], path=relative)
            self.assertEqual(exact['text'].encode(), self.external_paths[f'{name}/{relative}'])
        self.assert_native_final_products()
        self.restart_b()
        retained = self.call(self.b, '/api/sync/operation', operationId=self.adoption_plan['operationId'])
        self.assertEqual(retained['request'], self.adoption_request)
        self.assertEqual(retained['plan']['planId'], self.adoption_plan['planId'])
        self.assertEqual(retained['plan']['status'], 'committed')
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.adoption_request), retained['plan'])
        self.assertEqual(self.commit(self.b, self.adoption_plan), receipt)
        self.assert_native_adoption_originals()

    def test_sync_revalidation_native_adopt_invalid_actual_provider_refuses_without_plan_or_rewrite(self):
        self.native_adoption_case(invalid_provider=True)
        refused = self.call(self.b, '/api/operations/prepare', expected=409, **self.adoption_request)
        self.assertEqual(refused['error']['code'], 'queryInvocationUnavailable')
        self.assertEqual(self.call(self.b, '/api/operations/status',
                                   operationId=self.adoption_request['operationId'])['status'], 'unknown')
        self.assert_native_adoption_originals()

    def test_sync_revalidation_native_adopt_late_include_identity_refuses_original_plan_after_restart(self):
        self.native_adoption_case(include=True)
        self.prepare_native_adoption()
        original_identity = physical_identity(self.native_include)
        retained = self.folder / 'retained-adoption-shared.typ'
        self.retain_original(self.native_include, retained)
        self.native_include.write_bytes(self.native_include_bytes)
        self.assertNotEqual(physical_identity(self.native_include), original_identity)
        self.assertEqual(physical_identity(retained), original_identity)
        self.assertEqual(retained.read_bytes(), self.native_include.read_bytes())
        before = tree_state(self.rb)
        self.restart_b()
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.adoption_request),
                         self.adoption_plan)
        refused = self.commit(self.b, self.adoption_plan, expected=409)
        self.assertEqual(refused['error']['code'], 'conflict')
        self.assert_native_adoption_originals(before)
        self.assertEqual(self.call(self.b, '/api/sync/operation', operationId=self.adoption_plan['operationId']),
                         {'request': self.adoption_request, 'plan': self.adoption_plan})
        self.assertEqual(physical_identity(retained), original_identity)
        self.assertEqual(retained.read_bytes(), self.native_include_bytes)

    def test_sync_revalidation_native_same_request_retire_and_new_definition_keep_original_operation(self):
        expression = "nextCivilDay(title == '精确标题' ? date('2026-01-01') : date('9999-12-31'))"
        doc = self.native_definition(lifecycle='trash', defer_retirement=True, expression=expression)
        self.changed_schema_received_head(doc['source'])
        self.resolution()
        task = self.native_keys['Task']
        heads = self.current_view(self.b, task)['versions']
        self.assertEqual(len(heads), 1)
        retired = copy.deepcopy(heads[0])
        self.assertTrue(retired['state']['retired'])
        self.assertTrue((self.rb / 'Task/Task.typ').is_file())
        original_binding = self.call(self.b, '/api/sync/binding', root=str(self.rb))
        installed = next(revision for key, revision in original_binding['installedVersions'] if key == task)
        self.assertNotEqual(installed, retired['revision'])
        (self.rb / 'Task/retire-private.raw').write_bytes(b'ORIGINAL LIVE TASK RAW')
        task_tree = tree_state(self.rb / 'Task')
        current = tree_state(self.rb)
        operation_id = uid()
        target = self.rb / '.weftext-trash' / operation_id
        choices = [self.proposal, {'object': task, 'revisions': [retired['revision']],
                   'decision': {'kind': 'preserve', 'revision': retired['revision']},
                   'state': retired['state'], 'reviewedDependencies': []}]
        review = self.call(self.b, REVIEW, root=str(self.rb), identity=self.identity,
                           operationId=operation_id, resolutions=choices, blobs={})
        self.assertEqual(review['operationId'], operation_id)
        self.assertEqual(len(review['resolutions']), 2)
        for expected in choices:
            actual = next(item for item in review['resolutions'] if item['object'] == expected['object'])
            for field in ['object', 'revisions', 'decision', 'state']:
                self.assertEqual(actual[field], expected[field])
        self.assertIn({'object': task, 'revisions': [installed]}, review['expectedInstalled'])
        self.assertIn({'object': task, 'revisions': [retired['revision']]}, review['reviewedVersions'])
        self.assertEqual(tree_state(self.rb), current, 'A bound native review cannot retire or allocate author paths')
        self.assertFalse(target.exists())
        self.assertEqual(self.call(self.b, '/api/operations/status', operationId=operation_id)['status'], 'unknown')
        self.requested = {'operationId': operation_id, 'root': str(self.rb), 'intent': {
            'kind': 'revalidateSync', 'identity': self.identity,
            'expectedInstalled': review['expectedInstalled'], 'reviewedVersions': review['reviewedVersions'],
            'resolutions': review['resolutions'], 'blobs': {}}}
        self.prepare_revalidation()
        self.assertEqual(tree_state(self.rb), current)
        self.assertFalse(target.exists())
        self.restart_b()
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.requested), self.plan)
        receipt = self.commit(self.b, self.plan)
        self.assertFalse((self.rb / 'Task').exists())
        self.assertEqual(tree_state(target), task_tree,
                         'The one original Plan must move the live directory, source and private raw with their exact identities')
        self.assert_applied()
        applied = self.current_view(self.b, task)['versions']
        self.assertEqual(len(applied), 1)
        self.assertTrue(applied[0]['state']['retired'])
        for field in ['files', 'fileRevisions']:
            self.assertEqual(applied[0]['state'][field], retired['state'][field])
        self.assert_native_runtime()
        self.restart_b()
        original = self.call(self.b, '/api/sync/operation', operationId=operation_id)
        self.assertEqual(original['request'], self.requested)
        self.assertEqual(original['plan']['status'], 'committed')
        self.assertEqual(original['plan']['planId'], self.plan['planId'])
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.requested), original['plan'])
        self.assertEqual(self.commit(self.b, self.plan), receipt)
        self.assertEqual(tree_state(target), task_tree)
        self.assertEqual(self.assert_private_and_retained()['source'], self.expected_source)

    def test_sync_revalidation_removed_registered_resource_adopts_exact_absence_and_original_restart(self):
        self.removed_member_case()
        plan = self.call(self.a, '/api/operations/prepare', **self.removed_request)
        self.assertEqual(plan['kind'], 'adoptSyncExternal')
        self.assertEqual(plan['status'], 'prepared')
        self.assertEqual(plan['operationId'], self.removed_request['operationId'])
        self.assert_removed_originals()
        self.assertFalse(self.removed_asset.exists())
        self.restart_a()
        self.assertEqual(self.call(self.a, '/api/operations/prepare', **self.removed_request), plan)
        self.assertEqual(self.call(self.a, '/api/sync/operation', operationId=plan['operationId']),
                         {'request': self.removed_request, 'plan': plan})
        changed = copy.deepcopy(self.removed_request)
        changed['intent']['removedMembers'][0]['baseId'] = uid()
        refused = self.call(self.a, '/api/operations/prepare', expected=409, **changed)
        self.assertEqual(refused['error']['code'], 'operationConflict')
        receipt = self.commit(self.a, plan)
        self.assert_removed_originals()
        self.call(self.a, '/api/sync/publish', root=str(self.ra))
        heads = self.current_view(self.a, self.key)['versions']
        self.assertEqual(len(heads), 1)
        self.assertNotEqual(heads[0]['revision'], self.removed_predecessor['revision'])
        self.assertNotIn('asset.bin', heads[0]['state']['files'])
        self.assertNotIn('asset.bin', heads[0]['state']['fileRevisions'])
        for field in ['files', 'fileRevisions']:
            self.assertEqual(heads[0]['state'][field]['Note.typ'], self.removed_predecessor['state'][field]['Note.typ'])
        self.assertEqual(heads[0]['state']['files']['weftext.resources.json'],
                         hashlib.sha256(self.removed_manifest.read_bytes()).hexdigest())
        self.restart_a()
        original = self.call(self.a, '/api/sync/operation', operationId=plan['operationId'])
        self.assertEqual(original['request'], self.removed_request)
        self.assertEqual(original['plan']['planId'], plan['planId'])
        self.assertEqual(original['plan']['status'], 'committed')
        self.assertEqual(self.call(self.a, '/api/operations/prepare', **self.removed_request), original['plan'])
        self.assertEqual(self.commit(self.a, plan), receipt)
        self.assertFalse(self.removed_asset.exists())
        self.assert_removed_originals()

    def test_sync_revalidation_removed_member_reappears_with_same_bytes_refuses_original_absence_cut(self):
        self.removed_member_case()
        self.removed_asset.write_bytes(self.removed_bytes)
        self.assertNotEqual(physical_identity(self.removed_asset), self.removed_identity)
        current = tree_state(self.ra)
        refused = self.call(self.a, '/api/operations/prepare', expected=409, **self.removed_request)
        self.assertEqual(refused['error']['code'], 'staleBase')
        self.assertEqual(self.call(self.a, '/api/operations/status',
                                   operationId=self.removed_request['operationId'])['status'], 'unknown')
        self.assert_removed_originals(current)
        self.assertEqual(self.removed_asset.read_bytes(), self.removed_bytes)

    def test_sync_revalidation_native_actual_directory_read_requires_explicit_concurrent_choice(self):
        key = self.native_independent_directory()
        document = self.native_definition(body=NATIVE_GOOD + '\n#read("../Independent/weftext.directory.json")\n')
        self.receive_native_owner(document['source'])
        for server, root, branch in [(self.a, self.ra, 'AuthorBranch'), (self.b, self.rb, 'ReceiverBranch')]:
            for old, new in [('Independent', branch), (branch, 'Independent')]:
                moved = self.call(server, '/api/operations/prepare', root=str(root), operationId=uid(),
                                  intent={'kind': 'relocateDirectory', 'source': str(root / old),
                                          'parent': str(root), 'name': new})
                self.commit(server, moved)
            self.call(server, '/api/sync/publish', root=str(root))
        self.transfer(self.a, self.ra, self.b, [key])
        before = tree_state(self.rb)
        failed_id = uid()
        refused = self.call(self.b, REVIEW, expected=409, root=str(self.rb), identity=self.identity,
                            operationId=failed_id, resolutions=[self.proposal], blobs=self.blobs)
        self.assertEqual(refused['error']['code'], 'syncConflict')
        self.assertEqual(tree_state(self.rb), before)
        self.assertEqual(self.call(self.b, '/api/operations/status', operationId=failed_id)['status'], 'unknown')
        cut = self.call(self.b, CUT, root=str(self.rb), identity=self.identity, objects=[self.key])
        directory = next(item for item in cut['objects'] if item['object'] == key)
        versions = directory['view']['versions']
        self.assertEqual(len(versions), 2)
        self.assertEqual(versions[0]['state'], versions[1]['state'])
        binding = self.call(self.b, '/api/sync/binding', root=str(self.rb))
        installed = next(revision for object_key, revision in binding['installedVersions'] if object_key == key)
        chosen = next(version for version in versions if version['revision'] == installed)
        revisions = sorted(version['revision'] for version in versions)
        decision = {'object': key, 'revisions': revisions,
                    'decision': {'kind': 'preserve', 'revision': chosen['revision']},
                    'state': copy.deepcopy(chosen['state']), 'reviewedDependencies': []}
        review = self.bound_native_review([self.proposal, decision])
        note = next(item for item in review['resolutions'] if item['object'] == self.key)
        self.assertIn({'object': key, 'revisions': revisions}, note['reviewedDependencies'])
        self.assertIn({'object': key, 'revisions': revisions}, review['reviewedVersions'])
        self.prepare_revalidation()
        self.restart_b()
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.requested), self.plan)
        receipt = self.commit(self.b, self.plan)
        self.assert_applied()
        self.assert_native_inputs_preserved()
        self.assert_native_runtime()
        heads = self.current_view(self.b, key)['versions']
        self.assertEqual(len(heads), 1)
        self.assertEqual(heads[0]['state'], chosen['state'])
        current = tree_state(self.rb)
        self.restart_b()
        self.assertEqual(self.call(self.b, '/api/sync/operation', operationId=self.plan['operationId'])['request'], self.requested)
        self.assertEqual(self.commit(self.b, self.plan), receipt)
        self.assertEqual(tree_state(self.rb), current)

    def test_sync_revalidation_native_default_style_and_late_settings_keep_original_plan(self):
        self.install_native_style()
        document = self.native_definition()
        self.receive_native_owner(document['source'])
        review = self.bound_native_review([self.proposal])
        for object_key in [self.style_key, {'kind': 'settings'}]:
            versions = self.current_view(self.b, object_key)['versions']
            self.assertEqual(len(versions), 1)
            dependency = {'object': object_key, 'revisions': [versions[0]['revision']]}
            self.assertIn(dependency, review['reviewedVersions'])
            self.assertIn(dependency, review['resolutions'][0]['reviewedDependencies'])
        self.prepare_revalidation()
        self.restart_b()
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.requested), self.plan)
        self.commit(self.b, self.plan)
        self.assert_applied()
        self.assert_native_inputs_preserved()
        self.assert_native_runtime()
        self.resolution()
        self.bound_native_review([self.proposal])
        self.prepare_revalidation()
        settings = self.rb / '.weftext-settings.json'
        original = settings.read_bytes()
        (self.folder / 'retained-original-style-settings.json').write_bytes(original)
        identity = physical_identity(settings)
        changed = json.loads(original)
        self.assertTrue(changed['defaultStyle']['enabled'])
        changed['defaultStyle']['enabled'] = False
        settings.write_bytes((json.dumps(changed, ensure_ascii=False, indent=2) + '\n').encode())
        self.assertEqual(physical_identity(settings), identity)
        current = tree_state(self.rb)
        self.restart_b()
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.requested), self.plan)
        refused = self.commit(self.b, self.plan, expected=409)
        self.assertEqual(refused['error']['code'], 'conflict')
        self.assertEqual(tree_state(self.rb), current)
        self.assertEqual(self.call(self.b, '/api/sync/operation', operationId=self.plan['operationId']),
                         {'request': self.requested, 'plan': self.plan})
        self.assertEqual((self.folder / 'retained-original-style-settings.json').read_bytes(), original)
        self.assertEqual(self.assert_private_and_retained()['source'], self.saved)

    def test_sync_revalidation_native_moved_registered_include_keeps_same_plan_and_file_identity(self):
        document = self.native_definition(body=NATIVE_GOOD + '\n#include "part.typ"\n')
        part = '原生同次移动输入 中文😀\n'.encode()
        self.import_resource(self.a, self.ra, Path('Task/Task.typ'), 'part.typ', part)
        task = self.native_keys['Task']
        self.transfer(self.a, self.ra, self.b, [task])
        self.commit(self.b, self.call(self.b, '/api/operations/prepare', root=str(self.rb), operationId=uid(),
                    intent={'kind': 'applySync', 'identity': self.identity, 'objects': [task]}))
        (self.rb / 'Task/move-private.raw').write_bytes(b'ORIGINAL MOVING NATIVE RAW')
        original_task = tree_state(self.rb / 'Task')
        renamed = self.call(self.a, '/api/operations/prepare', root=str(self.ra), operationId=uid(),
                            intent={'kind': 'renameNode', 'source': str(self.ra / 'Task'), 'name': 'Renamed'})
        self.commit(self.a, renamed)
        self.transfer(self.a, self.ra, self.b, [task])
        self.receive_native_owner(document['source'])
        incoming = self.current_view(self.b, task)['versions']
        self.assertEqual(len(incoming), 1)
        self.assertEqual(incoming[0]['state']['name'], 'Renamed')
        review = self.bound_native_review([self.proposal])
        self.assertIn({'object': task, 'revisions': [incoming[0]['revision']]}, review['reviewedVersions'])
        self.assertFalse((self.rb / 'Renamed').exists())
        self.assertEqual(tree_state(self.rb / 'Task'), original_task)
        self.prepare_revalidation()
        self.assertFalse((self.rb / 'Renamed').exists())
        self.restart_b()
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.requested), self.plan)
        receipt = self.commit(self.b, self.plan)
        self.assert_applied()
        self.assertFalse((self.rb / 'Task').exists())
        expected = {('Renamed.typ' if relative == 'Task.typ' else relative): value
                    for relative, value in original_task.items()}
        self.assertEqual(tree_state(self.rb / 'Renamed'), expected)
        self.assertEqual((self.rb / 'Renamed/part.typ').read_bytes(), part)
        self.assert_native_runtime()
        actual = self.call(self.b, '/api/query/run', root=str(self.rb), parameters={}, plan={
            'types': ['task'], 'select': [{'name': 'body', 'expr': 'body'}]})
        self.assertEqual(actual['coverage'], 'complete')
        self.assertEqual(len(actual['rows']), 1)
        self.assertIn(part.decode().strip(), actual['rows'][0]['cells']['body']['value']['value'])
        current = tree_state(self.rb)
        self.restart_b()
        original = self.call(self.b, '/api/sync/operation', operationId=self.plan['operationId'])
        self.assertEqual(original['request'], self.requested)
        self.assertEqual(original['plan']['planId'], self.plan['planId'])
        self.assertEqual(original['plan']['status'], 'committed')
        self.assertEqual(self.commit(self.b, self.plan), receipt)
        self.assertEqual(tree_state(self.rb), current)


if __name__ == '__main__':
    available = {name for name in SyncRevalidation.__dict__ if name.startswith('test_sync_revalidation_')}
    selected = set(sys.argv[2:])
    unknown = selected - available
    if unknown:
        sys.exit('Unknown test selection: ' + ', '.join(sorted(unknown)))
    if len(sys.argv) < 2 or not Path(sys.argv[1]).is_file():
        sys.exit('Usage: test_sync_revalidation.py <candidate.exe> [exact_test_name ...]')
    suite = unittest.TestSuite(SyncRevalidation(name) for name in sorted(selected or available))
    sys.exit(not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful())
