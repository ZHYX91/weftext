"""Actual existing-replica retirement/restore HTTP over original public writers.

Local-owner personal Drafts are checked here; separate actor isolation and
controlled interrupted-install checkpoints remain Core test responsibilities.
No received versions, private records or Trash destinations are fabricated.

Run: python scripts/test_sync_retirement.py EXE [exact_test_name ...]
WEFTEXT_SYNC_EVIDENCE_DIR selects a fresh retained directory.
"""
from _creation_result import created_document
import hashlib
import json
import os
from pathlib import Path
import sys
import unittest

from test_hosted import uid
from test_sync_compound import SyncCompound, evidence_directory, physical_identity, tree_state


class SyncRetirement(SyncCompound):
    @classmethod
    def setUpClass(cls):
        if os.name != 'nt':
            raise RuntimeError('This harness requires Windows file identity evidence')
        cls.evidence = evidence_directory('weftext-sync-retirement-')
        print('Retained retirement sync evidence:', cls.evidence, flush=True)
        executable = Path(sys.argv[1]).resolve(strict=True)
        (cls.evidence / 'candidate.json').write_text(json.dumps({
            'exe': str(executable), 'sha256': hashlib.sha256(executable.read_bytes()).hexdigest(),
        }, indent=2), encoding='utf-8')

    def setUp(self):
        names = sorted(name for name in type(self).__dict__ if name.startswith('test_sync_retirement_'))
        self.folder = self.evidence / f'c{names.index(self._testMethodName) + 1:02}'
        self.folder.mkdir()
        self.servers, self.transfer_number = [], 0
        self.a = self.start_server([ '--data-dir', str(self.folder / 'pa')], 'A')
        self.b = self.start_server([ '--data-dir', str(self.folder / 'pb')], 'B')
        self.ra, self.rb = self.folder / 'a', self.folder / 'b'
        self.call(self.a, '/api/library/create', path=str(self.ra), operationId=uid())
        self.initial_paths = {name: Path(relative) for name, relative in [
            ('Parent', 'Parent/Parent.typ'), ('Child', 'Parent/Child/Child.typ'),
            ('Other', 'Other/Other.typ'), ('Live', 'Live/Live.typ')]}
        self.keys, self.resource_bytes = {}, {}
        for name, relative in self.initial_paths.items():
            doc = created_document(self.call(self.a, '/api/note/create', root=str(self.ra),
                            parent=str((self.ra / relative).parent.parent), name=name, operationId=uid()))
            self.keys[name] = {'kind': 'node', 'id': doc['nodeId']}
            self.save(self.a, self.ra, relative, f'ORIGINAL {name} 中文😀')
            data = f'original registered resource {doc["nodeId"]}'.encode()
            self.resource_bytes[name] = data
            self.import_resource(self.a, self.ra, relative, 'owned.bin', data)
            self.annotate(self.a, self.ra, relative, f'Original {name} discussion 中文😀')
        # All child Save/import/annotation producers precede enable. A later
        # child Save would create a real old-Parent dependency requiring its
        # own explicit semantic decision, rather than this clean writer case.
        self.binding = self.call(self.a, '/api/sync/enable', root=str(self.ra), operationId=uid())
        self.identity = self.binding['identity']
        self.objects = self.binding['installedObjects']
        self.assertTrue(all(key in self.objects for key in self.keys.values()))
        self.transfer(self.a, self.ra, self.b, self.objects)
        clone = self.call(self.b, '/api/operations/prepare', root=str(self.rb), operationId=uid(),
                          intent={'kind': 'cloneSync', 'identity': self.identity, 'objects': self.objects})
        self.assertFalse(self.rb.exists())
        self.commit(self.b, clone)
        for name in ['Parent', 'Other', 'Live']:
            self.save(self.b, self.rb, self.initial_paths[name], 'B retained root history 中文😀')
        self.transfer(self.b, self.rb, self.a, self.objects)
        self.commit(self.a, self.call(self.a, '/api/operations/prepare', root=str(self.ra), operationId=uid(),
                    intent={'kind': 'applySync', 'identity': self.identity, 'objects': self.objects}))
        for relative in self.initial_paths.values():
            doc = self.open(self.a, self.ra, relative)
            if doc.get('recoveredDraftConflict'):
                self.assertEqual(doc['recoveredDraft'], doc['recoveredDraftBaseSource'])
                self.call(self.a, '/api/draft/resolve', sessionId=doc['sessionId'],
                          action='mergeRecovered', source=doc['source'])
        self.drafts, self.history, self.resources_before = {}, {}, {}
        for name, relative in self.initial_paths.items():
            doc = self.open(self.b, self.rb, relative)
            self.drafts[name] = doc['source'] + f'\nB OWNER PRIVATE {name} 🌿'
            self.call(self.b, '/api/draft', inputRevision=1, sessionId=doc['sessionId'], source=self.drafts[name])
            versions = self.call(self.b, '/api/history/list', sessionId=doc['sessionId'])['versions']
            self.history[name] = {version['versionId']: self.call(self.b, '/api/history/read',
                sessionId=doc['sessionId'], versionId=version['versionId'])['source']
                for version in versions if version['status'] == 'available'}
            self.assertTrue(self.history[name], 'Original receiver Clone/Save history must be retained')
            self.resources_before[name] = self.resources(self.b, doc)['owned.bin']
            (self.rb / relative).with_name('personal.raw').write_bytes(f'B RAW {name} 🌿'.encode())
        self.before = tree_state(self.rb)
        self.root_retirement_operations = {}

    def restart_b(self):
        self.close_server(self.b)
        self.b = self.start_server([ '--data-dir', str(self.folder / 'pb')], 'B-restart')

    def local_operation(self, intent):
        plan = self.call(self.a, '/api/operations/prepare', root=str(self.ra), operationId=uid(), intent=intent)
        return self.commit(self.a, plan)

    def trash(self, server, root):
        return self.call(server, '/api/knowledge/trash', root=str(root))['trash']

    def current_states(self, server):
        status = self.call(server, '/api/sync/status', identity=self.identity)
        states = {}
        for name, key in self.keys.items():
            view = next(item['view'] for item in status['objects'] if item['object'] == key)
            self.assertEqual(len(view['versions']), 1, f'{name} must have one actual received head')
            states[name] = view['versions'][0]
        return states

    def exchange(self):
        before = tree_state(self.rb)
        self.call(self.a, '/api/sync/publish', root=str(self.ra))
        self.expected = self.current_states(self.a)
        self.transfer(self.a, self.ra, self.b, self.objects)
        self.assertEqual(tree_state(self.rb), before, 'Receive must not install author effects')
        self.assertEqual(self.current_states(self.b), self.expected)
        self.expected_sources = {}
        for name, version in self.expected.items():
            state = version['state']
            canonical = state['name'] + '.typ'
            exact = self.call(self.b, '/api/sync/file', identity=self.identity, object=self.keys[name],
                              revision=version['revision'], path=canonical)
            self.assertEqual(exact['digest'], state['files'][canonical])
            self.assertEqual(exact['fileRevision'], state['fileRevisions'][canonical])
            self.assertEqual(hashlib.sha256(exact['text'].encode()).hexdigest(), exact['digest'])
            self.expected_sources[name] = exact['text']

    def retired(self, name):
        by_id = {self.keys[key]['id']: version['state'] for key, version in self.expected.items()}
        state, seen = self.expected[name]['state'], set()
        while True:
            if state['retired']:
                return True
            parent = state.get('parent')
            if not parent or parent.get('id') not in by_id:
                return False
            self.assertNotIn(parent['id'], seen)
            seen.add(parent['id'])
            state = by_id[parent['id']]

    def assert_installed(self):
        binding = self.call(self.b, '/api/sync/binding', root=str(self.rb))
        installed = {json.dumps(key, sort_keys=True): revision for key, revision in binding['installedVersions']}
        self.assertEqual(binding['pendingPublications'], 0, 'Applying received versions must not publish another edit')
        roots = {name for name, version in self.expected.items() if version['state']['retired']}
        entries = self.trash(self.b, self.rb)
        self.assertEqual({entry['nodeId'] for entry in entries}, {self.keys[name]['id'] for name in roots})
        self.assertNotIn(self.keys['Child']['id'], {entry['nodeId'] for entry in entries}, 'Inherited Child is not an independent Trash root')
        rows = {}
        for lifecycle in ['live', 'trash']:
            result = self.call(self.b, '/api/query/run', root=str(self.rb), parameters={},
                plan={'scope': {'lifecycle': lifecycle, 'descendants': True}, 'select': [{'name': 'constant', 'expr': '1'}]})
            self.assertEqual(result['coverage'], 'complete')
            expected = {f'weftext-node://{self.identity["workspaceId"]}/{self.keys[name]["id"]}'
                        for name in self.keys if self.retired(name) == (lifecycle == 'trash')}
            self.assertEqual({row['entityKey'] for row in result['rows']}, expected)
            self.assertTrue(all(row['lifecycle'] == lifecycle for row in result['rows']))
            if lifecycle == 'trash':
                self.assertTrue(all(row['readOnly'] for row in result['rows']))
            rows.update({row['entityKey']: row for row in result['rows']})
        for name, version in self.expected.items():
            self.assertEqual(installed[json.dumps(self.keys[name], sort_keys=True)], version['revision'])
            lifecycle = 'trash' if self.retired(name) else 'live'
            entity_key = f'weftext-node://{self.identity["workspaceId"]}/{self.keys[name]["id"]}'
            doc = self.call(self.b, '/api/data/open', root=str(self.rb),
                            entityKey=entity_key, lifecycle=lifecycle, revision=rows[entity_key]['revision'])
            location = Path(doc['path'])
            self.assertTrue(location.is_relative_to(self.rb))
            self.assertEqual(doc['nodeId'], self.keys[name]['id'])
            self.assertEqual(doc['source'], self.expected_sources[name])
            self.assertEqual(doc['kind'], 'managedTrash' if lifecycle == 'trash' else 'managed')
            self.assertEqual(doc['recoveredDraft'], self.drafts[name])
            initial = self.initial_paths[name]
            self.assertEqual(physical_identity(location), self.before[initial.as_posix()][1])
            self.assertEqual(physical_identity(location.parent), self.before[initial.parent.as_posix()][1])
            for leaf in ['owned.bin', 'weftext.resources.json', 'weftext.annotations.json', 'personal.raw']:
                actual = location.with_name(leaf)
                self.assertEqual(physical_identity(actual), self.before[(initial.parent / leaf).as_posix()][1])
                self.assertEqual(hashlib.sha256(actual.read_bytes()).hexdigest(), self.before[(initial.parent / leaf).as_posix()][3])
            # Independent API assertions still fail the test, while allowing
            # later history/restart/Restore writers to expose their own result.
            with self.subTest(node=name, lifecycle=lifecycle, consumer='resources'):
                resource = self.resources(self.b, doc)['owned.bin']
                for field in ['resourceId', 'resource', 'nodeId', 'revision']:
                    self.assertEqual(resource[field], self.resources_before[name][field])
                self.assertEqual(resource['currentness'], 'accepted')
                self.assertEqual(self.call(self.b, '/api/resources/read', sessionId=doc['sessionId'],
                    resource=resource['resource'], revision=resource['revision']), self.resource_bytes[name])
            with self.subTest(node=name, lifecycle=lifecycle, consumer='history'):
                versions = self.call(self.b, '/api/history/list', sessionId=doc['sessionId'])['versions']
                self.assertTrue(set(self.history[name]).issubset({v['versionId'] for v in versions if v['status'] == 'available'}))
                for version, source in self.history[name].items():
                    self.assertEqual(self.call(self.b, '/api/history/read', sessionId=doc['sessionId'], versionId=version)['source'], source)
            if name in roots:
                entry = next(entry for entry in entries if entry['nodeId'] == self.keys[name]['id'])
                self.assertEqual(Path(entry['path']), location.parent)
                self.assertEqual(entry['operationId'], self.root_retirement_operations[name])

    def apply_original(self):
        before = tree_state(self.rb)
        request = {'operationId': uid(), 'root': str(self.rb), 'intent': {'kind': 'applySync',
            'identity': self.identity, 'objects': sorted(self.keys.values(), key=lambda key: key['id'])}}
        plan = self.call(self.b, '/api/operations/prepare', **request)
        self.assertEqual(plan['status'], 'prepared')
        self.assertEqual(plan['kind'], 'applySync')
        self.assertEqual(tree_state(self.rb), before)
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **request), plan)
        self.restart_b()
        self.assertEqual(tree_state(self.rb), before)
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **request), plan)
        original = self.call(self.b, '/api/sync/operation', operationId=request['operationId'])
        self.assertEqual(original['request'], request)
        self.assertEqual(original['plan'], plan)
        receipt = self.commit(self.b, plan)
        for name, version in self.expected.items():
            if version['state']['retired'] and name not in self.root_retirement_operations:
                self.root_retirement_operations[name] = request['operationId']
        self.assert_installed()
        self.assertEqual(self.commit(self.b, plan), receipt)
        installed = tree_state(self.rb)
        self.restart_b()
        self.assertEqual(tree_state(self.rb), installed)
        original = self.call(self.b, '/api/sync/operation', operationId=request['operationId'])
        self.assertEqual(original['request'], request)
        self.assertEqual(original['plan']['planId'], plan['planId'])
        self.assertEqual(original['plan']['status'], 'committed')
        self.assertEqual(self.commit(self.b, plan), receipt)
        self.assertEqual(tree_state(self.rb), installed)
        self.assert_installed()
        (self.folder / f'operation-{request["operationId"]}.json').write_text(json.dumps({
            'request': request, 'prepared': plan, 'receipt': receipt, 'expected': self.expected,
        }, ensure_ascii=False, indent=2), encoding='utf-8')

    def produce(self, multiple):
        self.local_operation({'kind': 'trashNode', 'source': str(self.ra / 'Parent')})
        if multiple:
            self.local_operation({'kind': 'trashNode', 'source': str(self.ra / 'Other')})
        else:
            self.local_operation({'kind': 'renameNode', 'source': str(self.ra / 'Other'), 'name': 'MovedOther'})
            self.save(self.a, self.ra, Path('Live/Live.typ'), 'A live content in one retirement transaction 中文😀')
        self.exchange()

    def test_sync_retirement_multiple_existing_roots_original_restart_and_private_facts(self):
        self.produce(multiple=True)
        self.apply_original()

    def test_sync_retirement_mixed_live_move_and_content_keep_inherited_child_lifecycle(self):
        self.produce(multiple=False)
        self.apply_original()

    def test_sync_retirement_restore_then_live_move_preserves_original_retirement_record(self):
        self.produce(multiple=True)
        self.apply_original()
        entry = next(entry for entry in self.trash(self.a, self.ra) if entry['nodeId'] == self.keys['Parent']['id'])
        self.local_operation({'kind': 'restoreNode', 'trashOperationId': entry['operationId'],
            'nodeId': entry['nodeId'], 'parent': str(self.ra), 'name': 'RestoredParent'})
        self.local_operation({'kind': 'renameNode', 'source': str(self.ra / 'Live'), 'name': 'MovedLive'})
        self.exchange()
        self.apply_original()


if __name__ == '__main__':
    available = {name for name in SyncRetirement.__dict__ if name.startswith('test_sync_retirement_')}
    selected = set(sys.argv[2:])
    unknown = selected - available
    if unknown:
        sys.exit('Unknown test selection: ' + ', '.join(sorted(unknown)))
    if len(sys.argv) < 2 or not Path(sys.argv[1]).is_file():
        sys.exit('Usage: test_sync_retirement.py <candidate.exe> [exact_test_name ...]')
    suite = unittest.TestSuite(SyncRetirement(name) for name in sorted(selected or available))
    sys.exit(not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful())
