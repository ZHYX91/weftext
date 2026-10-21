"""Real local HTTP tag forests, Query closure and one-plan 5,000-tag import.

Run against an explicitly built executable. Fixtures, original uploads and
per-request timings are retained in fresh directories; no existing library or
private journal is rewritten. --artifacts-dir selects the current workspace's
temporary parent; the portable default stays inside this repository's target.
"""
from _creation_result import created_document
import argparse
import base64
import copy
import hashlib
import http.client
import io
import json
from pathlib import Path
import tempfile
import time
import unittest
from urllib.error import HTTPError
from urllib.parse import urlsplit
from urllib.request import Request, urlopen
import zipfile

import test_hosted
from test_hosted import Server, uid

ARTIFACTS = None


def native_tags(schema, count=3, mutate=None):
    """A real complete upload, with no foreign Core records or claimed custody."""
    workspace, node = uid(), uid()
    ids = [uid() for _ in range(count)]
    document = {'version': 2, 'tags': [dict(tagId=tag, displayName='同名/path',
        parentTagId=ids[index - 1] if index else None) for index, tag in enumerate(ids)]}
    if mutate:
        mutate(document)
    leaf = f'weftext-tag://{workspace}/{ids[-1]}'
    source = (f'#metadata((id: "{node}", types: ("person",), tags: ((ref: "{leaf}",), '
              f'(text: "{leaf}",)))) <weftext>\n原始引用文字保留😀\n')
    files = {'.weftext-schema.json': schema.encode(),
             '.weftext-tags.json': json.dumps(document, ensure_ascii=False).encode(),
             'Imported/Imported.typ': source.encode()}
    roles = {'.weftext-schema.json': {'role': 'schema'},
             '.weftext-tags.json': {'role': 'tags'},
             'Imported/Imported.typ': {'role': 'source', 'nodeId': node}}
    manifest = dict(format='weftext.native-package', version=1, packageId=uid(),
        workspaceId=workspace, scope='library', schemaPath='.weftext-schema.json', losses=[],
        entries={name: {**roles[name], 'bytes': len(value),
                       'sha256': hashlib.sha256(value).hexdigest()} for name, value in files.items()})
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, 'w', zipfile.ZIP_DEFLATED) as archive:
        archive.writestr('manifest.json', json.dumps(manifest))
        for name, value in files.items():
            archive.writestr(name, value)
    return buffer.getvalue(), document, manifest, source


class HierarchicalTags(unittest.TestCase):
    def setUp(self):
        self.folder = Path(tempfile.mkdtemp(prefix='tag-', dir=ARTIFACTS))
        (self.folder / 'test-case.txt').write_text(self._testMethodName, 'utf-8')
        self.data, self.root = self.folder / 'state', self.folder / 'Library'
        self.timings = []
        self.server = Server(['--no-browser', '--data-dir', str(self.data)])
        self.library = self.call('/api/library/create', {'path': str(self.root), 'operationId': uid()})
        self.doc = self.note('Anchor')
        print('Hierarchical tags evidence:', self.folder, flush=True)

    def tearDown(self):
        self.server.close()
        (self.folder / 'timings.json').write_text(json.dumps(self.timings, indent=2), 'utf-8')

    def call(self, route, body=None, status=200, timeout=300):
        request = Request(self.server.url + route,
            data=None if body is None else json.dumps(body).encode(),
            headers={'Content-Type': 'application/json', 'X-Weftext-Token': self.server.token})
        started = time.perf_counter()
        try:
            response = urlopen(request, timeout=timeout)
        except HTTPError as error:
            response = error
        with response:
            value = json.loads(response.read())
            self.timings.append({'route': route, 'status': response.status,
                                 'seconds': time.perf_counter() - started})
            self.assertEqual(response.status, status, (route, value))
            return value

    def refused(self, route, body, codes):
        request = Request(self.server.url + route, data=json.dumps(body).encode(),
            headers={'Content-Type': 'application/json', 'X-Weftext-Token': self.server.token})
        with self.assertRaises(HTTPError) as caught:
            urlopen(request, timeout=300)
        error = json.loads(caught.exception.read())
        self.assertIn(error['error']['code'], codes, error)
        return error

    def note(self, name):
        return created_document(self.call('/api/note/create', {'root': str(self.root), 'parent': str(self.root),
                                            'name': name, 'operationId': uid()}))

    def registry(self):
        value = self.call('/api/navigation/tags', {'root': str(self.root)})
        self.assertTrue(value['complete'], value)
        return value

    def commit(self, prepared):
        preview = prepared.get('preview', prepared)
        return self.call('/api/operations/commit', {key: preview[key] for key in ['operationId', 'planId']})

    def tag_request(self, action, **extra):
        return {'root': str(self.root), 'baseId': self.registry()['baseId'], 'operationId': uid(), **extra}

    def tag(self, name='同名/path', parent=None):
        prepared = self.call('/api/navigation/create-tag', self.tag_request('create', displayName=name, parentRef=parent))
        self.commit(prepared)
        return prepared['targetRef']

    def move(self, reference, parent=None):
        prepared = self.call('/api/navigation/move-tag', self.tag_request('move', tagRef=reference, parentRef=parent))
        self.commit(prepared)

    def save(self, source):
        self.call('/api/save', {'sessionId': self.doc['sessionId'], 'baseId': self.doc['baseId'],
                               'source': source, 'operationId': uid()})
        self.doc = self.call('/api/open', {'root': str(self.root), 'path': self.doc['path']})

    def query(self, root, mode='subtree', **extra):
        member = f'tagInSubtree(t.ref, tagRef("{root}"))' if mode == 'subtree' else f't.ref == tagRef("{root}")'
        plan = {'where': f'isAbsent(tags) ? false : tags.exists(t, has(t.ref) ? {member} : false)',
                'select': [{'name': 'self', 'expr': 'self'}], **extra}
        return self.call('/api/query/run', {'root': str(self.root), 'plan': plan})

    def assign(self, *references):
        values = ', '.join(f'(ref: "{ref}",)' for ref in references)
        self.save(f'#metadata((id: "{self.doc["nodeId"]}", types: ("person",), tags: ({values},))) <weftext>\n原始正文😀\n')

    def schema(self, install=False):
        schema = self.call('/api/data/schema', {'root': str(self.root)})
        if install:
            plan = self.call('/api/data/schema/prepare', {'root': str(self.root),
                'source': schema['source'], 'beforeSource': schema['source'],
                'beforeRevision': schema['revision'], 'operationId': uid()})
            self.call('/api/data/schema/install', {key: plan[key] for key in ['operationId', 'planId']})
        return schema['source']

    def import_request(self, payload, name):
        return dict(operationId=uid(), sessionId=self.doc['sessionId'], baseId=self.doc['baseId'],
            root=str(self.root), parent=str(self.root), name=name, format='native',
            bytesBase64=base64.b64encode(payload).decode())

    def test_registry_move_rename_exact_subtree_and_distinct_note_count(self):
        parent = self.tag()
        branch = self.tag(parent=parent)
        first, second = self.tag(parent=branch), self.tag(parent=branch)
        self.tag('é/name')
        self.tag('e\u0301/name')
        self.assign(first, second)
        before = Path(self.doc['path']).read_bytes()
        original_ids = {tag['tagId'] for tag in self.registry()['tags']}
        registry_path = self.root / '.weftext-tags.json'
        original_object = (registry_path.stat().st_dev, registry_path.stat().st_ino)
        self.assertEqual(self.query(parent, 'exact')['rows'], [])
        self.assertEqual(len(self.query(parent)['rows']), 1)
        grouped = self.query(parent, select=[], group={'keys': [], 'aggregates': [{'name': 'notes', 'op': 'count'}]})
        self.assertEqual(grouped['rows'][0]['cells']['notes']['value']['value'], '1')
        rename = self.call('/api/navigation/rename-tag', self.tag_request('rename', tagRef=parent, displayName='已改名/😀'))
        self.commit(rename)
        self.assertEqual(rename['targetRef'], parent)
        self.move(branch)
        self.assertEqual(self.query(parent)['rows'], [])
        self.assertEqual(len(self.query(branch)['rows']), 1)
        self.assertEqual(Path(self.doc['path']).read_bytes(), before)
        self.assertEqual({tag['tagId'] for tag in self.registry()['tags']}, original_ids)
        self.assertEqual((registry_path.stat().st_dev, registry_path.stat().st_ino), original_object)
        self.refused('/api/navigation/move-tag', self.tag_request('move', tagRef=branch, parentRef=first), {'tagRegistry'})
        self.refused('/api/navigation/move-tag', self.tag_request('move', tagRef=branch, parentRef=branch), {'tagRegistry'})
        self.refused('/api/navigation/move-tag', self.tag_request('move', tagRef=branch,
            parentRef=f'weftext-tag://{uid()}/{uid()}'), {'invalidReference'})
        document = json.loads(registry_path.read_text('utf-8'))
        self.assertEqual(document['version'], 2)
        self.assertTrue(all(set(tag) == {'tagId', 'displayName', 'parentTagId'} for tag in document['tags']))
        self.assign(parent)
        self.assertEqual(len(self.query(parent, 'exact')['rows']), 1)

    def test_saved_current_pinned_and_zero_scope_static_roots(self):
        self.schema(install=True)
        parent, leaf = self.tag(), None
        leaf = self.tag(parent=parent)
        missing = f'weftext-tag://{self.library["workspaceId"]}/{uid()}'
        bad = {'where': f'false && tagInSubtree(tagRef("{leaf}"), tagRef("{missing}"))',
               'select': [{'name': 'value', 'expr': 'true'}]}
        self.refused('/api/query/run', {'root': str(self.root), 'plan': bad}, {'invalid_data'})
        self.assign(leaf)
        selected = f'tags.exists(t, has(t.ref) ? tagInSubtree(t.ref, tagRef("{parent}")) : false)'
        plan = {'where': f'isAbsent(tags) ? false : {selected}', 'select': [{'name': 'self', 'expr': 'self'}]}
        request = dict(sessionId=self.doc['sessionId'], baseId=self.doc['baseId'], source=self.doc['source'],
                       serial=1, operationId=uid(), label='weftext-query-tree', plan=plan)
        saved = self.call('/api/query/definition/save', request)
        owner = f'weftext-node://{self.library["workspaceId"]}/{self.doc["nodeId"]}'
        key, revision = saved['definition']['key'], saved['definitionRevision']
        def run(mode):
            selection = {'mode': mode, 'key': key}
            if mode == 'pinned':
                selection['revision'] = revision
            return self.call('/api/query/definition/run', {'root': str(self.root), 'owner': owner,
                'selection': selection, 'parameters': {}})['result']
        self.assertEqual(len(run('current')['rows']), 1)
        self.assertEqual(len(run('pinned')['rows']), 1)
        self.move(leaf)
        self.assertEqual(run('current')['rows'], [])
        self.assertEqual(run('pinned')['rows'], [])
        self.move(leaf, parent)
        self.assertEqual(len(run('pinned')['rows']), 1)
        self.doc = self.call('/api/open', {'root': str(self.root), 'path': self.doc['path']})
        rejected = {**request, 'sessionId': self.doc['sessionId'], 'baseId': self.doc['baseId'],
                    'source': self.doc['source'], 'key': key, 'operationId': uid(), 'plan': bad}
        before = Path(self.doc['path']).read_bytes()
        self.refused('/api/query/definition/save', rejected, {'invalidQueryDefinition'})
        self.assertEqual(Path(self.doc['path']).read_bytes(), before)

    def test_empty_library_negation_and_literal_root_still_bind_complete_tree(self):
        root = str(self.folder / 'Empty')
        library = self.call('/api/library/create', {'path': root, 'operationId': uid()})
        basis = self.call('/api/navigation/tags', {'root': root})
        tag = self.call('/api/navigation/create-tag', {'root': root, 'baseId': basis['baseId'],
            'operationId': uid(), 'displayName': 'Empty subtree', 'parentRef': None})
        self.commit(tag)
        reference = tag['targetRef']
        plan = {'where': f'!tagInSubtree(tagRef("{reference}"), tagRef("{reference}"))',
                'select': [{'name': 'value', 'expr': 'true'}]}
        result = self.call('/api/query/run', {'root': root, 'plan': plan})
        self.assertEqual(result['rows'], [])
        self.assertEqual(result['coverage'], 'complete')
        self.assertEqual(result['membershipIds'], [])
        self.assertTrue(any(version.get('role') == 'tags' for version in result['inputVersions']))
        missing = f'weftext-tag://{library["workspaceId"]}/{uid()}'
        plan['where'] = f'true || tagInSubtree(tagRef("{reference}"), tagRef("{missing}"))'
        self.refused('/api/query/run', {'root': root, 'plan': plan}, {'invalid_data'})

    def test_cancel_and_discarded_commit_response_recover_original_identity_after_restart(self):
        source_before = Path(self.doc['path']).read_bytes()
        request = self.tag_request('create', displayName='取消', parentRef=None)
        proposal = self.call('/api/navigation/create-tag', request)
        preview = proposal['preview']
        self.call('/api/navigation/cancel', {'root': str(self.root), **{key: preview[key]
                  for key in ['operationId', 'planId']}})
        self.assertEqual(self.registry()['tags'], [])
        self.refused('/api/operations/commit', {key: preview[key] for key in ['operationId', 'planId']}, {'recoveryPending'})
        cancelled = self.call('/api/navigation/create-tag', request)
        self.assertEqual(cancelled['targetRef'], proposal['targetRef'])
        self.assertNotEqual(cancelled['preview']['status'], 'committed')
        request = self.tag_request('create', displayName='原操作', parentRef=None)
        proposal = self.call('/api/navigation/create-tag', request)
        preview = proposal['preview']
        url = urlsplit(self.server.url)
        connection = http.client.HTTPConnection(url.hostname, url.port, timeout=300)
        connection.request('POST', '/api/operations/commit', json.dumps({key: preview[key]
            for key in ['operationId', 'planId']}), {'Content-Type': 'application/json',
            'X-Weftext-Token': self.server.token})
        response = connection.getresponse()
        self.assertEqual(response.status, 200)
        # Discard the response body rather than accepting its receipt.
        connection.close()
        self.server.close()
        self.server = Server(['--no-browser', '--data-dir', str(self.data)])
        status = self.call('/api/operations/status', {'operationId': preview['operationId']})
        self.assertEqual(status['status'], 'committed')
        replay = self.call('/api/navigation/create-tag', request)
        self.assertEqual(replay['targetRef'], proposal['targetRef'])
        self.assertEqual(len(self.registry()['tags']), 1)
        self.assertEqual(self.registry()['tags'][0]['tagRef'], proposal['targetRef'])
        self.assertEqual(Path(self.doc['path']).read_bytes(), source_before)

    def test_native_strict_forest_refuses_invalid_members_without_installing_partial_tree(self):
        schema = self.schema()
        cases = {
            'missing_member': lambda d: d['tags'][0].pop('parentTagId'),
            'missing_parent': lambda d: d['tags'][1].update(parentTagId=uid()),
            'self_parent': lambda d: d['tags'][0].update(parentTagId=d['tags'][0]['tagId']),
            'cycle': lambda d: d['tags'][0].update(parentTagId=d['tags'][-1]['tagId']),
            'duplicate': lambda d: d['tags'][1].update(tagId=d['tags'][0]['tagId']),
            'unknown_member': lambda d: d['tags'][0].update(children=[]),
            'old_successor': lambda d: d.update(version=1),
        }
        for name, mutate in cases.items():
            with self.subTest(name=name):
                payload, _, _, _ = native_tags(schema, mutate=mutate)
                (self.folder / (name + '.weftext.zip')).write_bytes(payload)
                self.refused('/api/exchange/prepare', self.import_request(payload, name), {'packageTags'})
                self.assertFalse((self.root / name).exists())
                self.assertEqual(self.registry()['tags'], [])

    def test_native_5000_longest_chain_single_plan_fresh_mapping_and_5001_refusal(self):
        schema = self.schema()
        payload, source_tree, manifest, original_source = native_tags(schema, 5000)
        (self.folder / 'chain-5000.weftext.zip').write_bytes(payload)
        started = time.perf_counter()
        request = self.import_request(payload, 'FiveThousand')
        plan = self.call('/api/exchange/prepare', request)
        prepare_seconds = time.perf_counter() - started
        self.assertEqual(plan['kind'], 'importNative')
        self.assertFalse((self.root / 'FiveThousand').exists())
        receipt = self.call('/api/exchange/commit', {key: plan[key] for key in ['operationId', 'planId']} | {'acceptLosses': True})
        total_seconds = time.perf_counter() - started
        self.assertEqual(receipt['status'], 'committed')
        registry = self.registry()['tags']
        self.assertEqual(len(registry), 5000)
        source_ids = {tag['tagId'] for tag in source_tree['tags']}
        target = {tag['tagId']: tag for tag in registry}
        self.assertFalse(set(target) & source_ids)
        roots = [tag for tag in registry if tag['parentTagId'] is None]
        self.assertEqual(len(roots), 1)
        children = {}
        for tag in registry:
            if tag['parentTagId'] is not None:
                self.assertNotIn(tag['parentTagId'], children)
                children[tag['parentTagId']] = tag['tagId']
        cursor, depth = roots[0]['tagId'], 1
        while cursor in children:
            cursor, depth = children[cursor], depth + 1
        self.assertEqual(depth, 5000)
        source_node = next(entry['nodeId'] for entry in manifest['entries'].values() if entry['role'] == 'source')
        imported = self.call('/api/open', {'root': str(self.root),
            'path': str(self.root / 'FiveThousand/Imported/Imported.typ')})
        self.assertEqual(imported['nodeId'], plan['nodeMap'][source_node])
        self.assertIn(f'ref: "{target[cursor]["tagRef"]}"', imported['source'])
        original_leaf = f'weftext-tag://{manifest["workspaceId"]}/{source_tree["tags"][-1]["tagId"]}'
        self.assertIn(f'text: "{original_leaf}"', imported['source'])
        self.assertEqual((self.root / 'FiveThousand/.weftext-import-originals/original.weftext').read_bytes(), payload)
        self.assertEqual(len(self.query(roots[0]['tagRef'])['rows']), 1)
        self.assertEqual(self.query(roots[0]['tagRef'], 'exact')['rows'], [])
        self.assertEqual(len(self.query(target[cursor]['tagRef'], 'exact')['rows']), 1)
        before = Path(imported['path']).read_bytes()
        self.move(target[cursor]['tagRef'])
        self.assertEqual(self.query(roots[0]['tagRef'])['rows'], [])
        self.assertEqual(Path(imported['path']).read_bytes(), before)
        self.refused('/api/navigation/create-tag', self.tag_request('create', displayName='5001', parentRef=None), {'budget'})
        print(json.dumps({'tags': 5000, 'depth': depth, 'prepareSeconds': prepare_seconds,
                          'prepareAndCommitSeconds': total_seconds, 'samples': 1}), flush=True)
        # An oversized source forest must fail its own closed package contract,
        # independent of the already-full destination's separate budget.
        payload, _, _, _ = native_tags(schema, 5001)
        (self.folder / 'chain-5001.weftext.zip').write_bytes(payload)
        self.refused('/api/exchange/prepare', self.import_request(payload, 'TooMany'), {'packageTags'})
        self.assertFalse((self.root / 'TooMany').exists())
        self.assertEqual(len(self.registry()['tags']), 5000)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path)
    parser.add_argument('tests', nargs='*', help='Optional exact test method names')
    parser.add_argument('--artifacts-dir', type=Path,
        default=Path(__file__).resolve().parents[1] / 'target/test-fixtures/hierarchical-tags')
    arguments = parser.parse_args()
    names = unittest.defaultTestLoader.getTestCaseNames(HierarchicalTags)
    if set(arguments.tests) - set(names):
        parser.error('Unknown exact tests: ' + ', '.join(sorted(set(arguments.tests) - set(names))))
    ARTIFACTS = arguments.artifacts_dir.resolve()
    ARTIFACTS.mkdir(parents=True, exist_ok=True)
    test_hosted.EXE = arguments.executable.resolve(strict=True)
    print('Candidate:', test_hosted.EXE, hashlib.sha256(test_hosted.EXE.read_bytes()).hexdigest(), flush=True)
    suite = unittest.TestSuite(HierarchicalTags(name) for name in (arguments.tests or names))
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    raise SystemExit(not result.wasSuccessful())
