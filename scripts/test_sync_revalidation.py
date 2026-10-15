"""Actual local HTTP semantic revalidation through the original operation writer.

Two independent programs create every version through public Save/configuration/
sync producers. The reviewed object states and exact cuts come from the real
read-only APIs, never edited graph records or client proof flags. Fixtures,
transcripts and owned-process evidence stay in a fresh explicit output directory.

Run: python scripts/test_sync_revalidation.py EXE [exact_test_name ...]
Set WEFTEXT_SYNC_EVIDENCE_DIR to a new workspace output directory.
"""
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
        doc = self.call(self.a, '/api/note/create', root=str(self.ra), parent=str(self.ra),
                        name='Note', operationId=uid())
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
                            and data.get('intent', {}).get('kind') == 'revalidateSync')
        if not semantic_prepare and route != '/api/query/definition/run':
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
