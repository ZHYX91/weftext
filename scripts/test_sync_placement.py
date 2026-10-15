"""Actual complex-placement HTTP consumers of one original ApplySync Plan.

The sender uses only public, empty-target RenameNode/MoveNode operations.
The receiver reviews and installs their final received cut once. Positive
cases require a successful original Plan, not an unsupported-scope refusal.
Core owns controlled namespace checkpoint coverage; these cases cover the
real HTTP request, local identities, resources, drafts and restart consumers.

Run: python scripts/test_sync_placement.py EXE [exact_test_name ...]
Set WEFTEXT_SYNC_EVIDENCE_DIR to a fresh owned output directory.
"""
import hashlib
import json
import os
from pathlib import Path
import sys
import unittest

from test_hosted import uid
from test_sync_compound import SyncCompound, evidence_directory, physical_identity, png, tree_state


class SyncPlacement(SyncCompound):
    @classmethod
    def setUpClass(cls):
        if os.name != 'nt':
            raise RuntimeError('This harness requires the Windows candidate and physical identity API')
        cls.evidence = evidence_directory('weftext-sync-placement-')
        print('Retained complex placement evidence:', cls.evidence, flush=True)

    def prepare_source_before_sync(self):
        # Extend the existing real-producer fixture before its public enable/
        # clone. No received manifest, admission or private state is edited.
        self.nested = 'child' in self.placement_test_name
        names = ['Still']
        if self.nested:
            doc = self.call(self.a, '/api/note/create', root=str(self.ra), parent=str(self.ra / 'Left'),
                            name='Child', operationId=uid())
            self.docs['Child'] = doc
            self.keys['Child'] = {'kind': 'node', 'id': doc['nodeId']}
            self.save(self.a, self.ra, Path('Left/Child/Child.typ'), 'Initial child source 中文😀')
            names.append('Child')
        for name in names:
            relative = Path('Left/Child/Child.typ') if name == 'Child' else Path(f'{name}/{name}.typ')
            self.import_resource(self.a, self.ra, relative, 'baseline.png', self.baseline_png)
            self.annotate(self.a, self.ra, relative, f'初始 {name} 原讨论😀')

    def setUp(self):
        # The inherited fixture owns its processes and files. Keep its Windows
        # journal paths short while retaining the full unittest identity.
        self.placement_test_name = self._testMethodName
        names = sorted(name for name in type(self).__dict__ if name.startswith('test_sync_placement_'))
        self._testMethodName = f'c{names.index(self.placement_test_name) + 1:02d}'
        try:
            super().setUp()
        finally:
            self._testMethodName = self.placement_test_name
        (self.folder / 'case.json').write_text(json.dumps({'test': self.placement_test_name}) + '\n', encoding='utf-8')
        self.original_paths = {name: Path(f'{name}/{name}.typ') for name in self.keys}
        if self.nested:
            self.original_paths['Child'] = Path('Left/Child/Child.typ')
        for name in ['Still'] + (['Child'] if self.nested else []):
            doc = self.open(self.b, self.rb, self.original_paths[name])
            self.drafts[name] = doc['source'] + f'\nB PRIVATE PLACEMENT DRAFT {name} 🌿'
            self.call(self.b, '/api/draft', sessionId=doc['sessionId'], baseId=doc['baseId'], source=self.drafts[name])
            history = self.call(self.b, '/api/history/list', sessionId=doc['sessionId'])
            self.history[name] = {item['versionId']: self.call(self.b, '/api/history/read',
                                  sessionId=doc['sessionId'], versionId=item['versionId'])['source']
                                  for item in history['versions'] if item['status'] == 'available'}
            self.initial_resources[name] = self.resources(self.b, doc)['baseline.png']
        for name, relative in self.original_paths.items():
            directory = (self.rb / relative).parent
            if not (directory / 'personal.raw').exists():
                (directory / 'personal.raw').write_bytes(f'B RAW {name} 中文😀'.encode())
            opaque = directory / 'opaque'
            opaque.mkdir()
            (opaque / 'keep.raw').write_bytes(f'B OPAQUE {name} 🌿'.encode())
        self.before = tree_state(self.rb)
        self.selected_names, self.moved_names, self.after_paths = [], [], {}
        self.expected, self.sender_resources = {}, {}

    def rename(self, source, name):
        plan = self.call(self.a, '/api/operations/prepare', root=str(self.ra), operationId=uid(),
                         intent={'kind': 'renameNode', 'source': str(self.ra / source), 'name': name})
        self.commit(self.a, plan)

    def move(self, source, parent):
        plan = self.call(self.a, '/api/operations/prepare', root=str(self.ra), operationId=uid(),
                         intent={'kind': 'moveNode', 'source': str(self.ra / source),
                                 'parent': str(self.ra / parent)})
        self.commit(self.a, plan)

    def produce(self, layout):
        if layout == 'exchange':
            self.rename('Left', 'Holding')
            self.rename('Right', 'Left')
            self.rename('Holding', 'Right')
            directories = {'Left': 'Right', 'Right': 'Left'}
        elif layout == 'rotate':
            self.rename('Left', 'Holding')
            self.rename('Still', 'Left')
            self.rename('Right', 'Still')
            self.rename('Holding', 'Right')
            directories = {'Left': 'Right', 'Still': 'Left', 'Right': 'Still'}
        elif layout == 'extract':
            self.move('Left/Child', '')
            self.rename('Left', 'RenamedLeft')
            directories = {'Left': 'RenamedLeft', 'Child': 'Child'}
        elif layout == 'nested':
            # Child's real operation must observe the final parent head.
            self.move('Left', 'Container')
            self.rename('Container/Left/Child', 'RenamedChild')
            directories = {'Left': 'Container/Left', 'Child': 'Container/Left/RenamedChild'}
        else:
            self.fail('Unknown placement layout')
        self.assertFalse((self.ra / 'Holding').exists())
        self.moved_names = list(directories)
        self.selected_names = self.moved_names + ([] if 'Still' in directories else ['Still'])
        self.after_paths = {name: Path(directory) / f'{Path(directory).name}.typ'
                            for name, directory in directories.items()}
        if 'Still' not in self.after_paths:
            self.after_paths['Still'] = Path('Still/Still.typ')
        # Final ancestors finish all content producers before descendants read
        # their parent head. Container has no later author edit in this fixture.
        producer_order = sorted(self.selected_names, key=lambda name: len(self.after_paths[name].parts))
        for index, name in enumerate(producer_order):
            relative = self.after_paths[name]
            self.save(self.a, self.ra, relative, f'A final {layout} source for original {name} 中文😀')
            filenames = [relative.name]
            if name in self.moved_names:
                self.import_resource(self.a, self.ra, relative, 'added.png', png((20, index, 255)))
                self.annotate(self.a, self.ra, relative, f'A final {name} discussion {layout} 😀')
                filenames += ['baseline.png', 'added.png', 'weftext.resources.json', 'weftext.annotations.json']
                self.sender_resources[name] = self.resources(self.a, self.open(self.a, self.ra, relative))
            self.expected[name] = {leaf: ((self.ra / relative).parent / leaf).read_bytes() for leaf in filenames}
        received = self.transfer(self.a, self.ra, self.b, [self.keys[name] for name in self.selected_names])
        self.received = {entry['object']['id']: entry['view'] for entry in received['objects']
                         if entry['object']['kind'] == 'node'}
        self.assertEqual(tree_state(self.rb), self.before, 'Receive cannot install any author effect')
        self.assert_received_sources()
        (self.folder / 'placement-fixture.json').write_text(json.dumps({
            'layout': layout, 'keys': self.keys,
            'originalPaths': {name: str(relative) for name, relative in self.original_paths.items()},
            'afterPaths': {name: str(relative) for name, relative in self.after_paths.items()},
            'selectedNames': self.selected_names,
        }, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')

    def request(self):
        objects = sorted([self.keys[name] for name in self.selected_names], key=lambda key: key['id'])
        return {'operationId': uid(), 'root': str(self.rb), 'intent': {
            'kind': 'applySync', 'identity': self.identity, 'objects': objects}}

    def prepare(self, request=None):
        self.requested = request or self.request()
        current = tree_state(self.rb)
        plan = self.call(self.b, '/api/operations/prepare', **self.requested)
        self.assertEqual((plan['kind'], plan['status'], plan['operationId']),
                         ('applySync', 'prepared', self.requested['operationId']))
        for name in self.selected_names:
            self.assertEqual(plan['nodeMap'][self.keys[name]['id']], self.keys[name]['id'])
        self.assertEqual(tree_state(self.rb), current, 'Prepare must not create parking or change author paths')
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.requested), plan)
        self.assertEqual(self.call(self.b, '/api/sync/operation', operationId=plan['operationId']),
                         {'request': self.requested, 'plan': plan})
        self.plan = plan
        return plan

    def assert_received_sources(self):
        for name in self.selected_names:
            view = self.received[self.keys[name]['id']]
            self.assertEqual(len(view['versions']), 1)
            source = self.call(self.b, '/api/sync/file', identity=self.identity, object=self.keys[name],
                               revision=view['versions'][0]['revision'], path=self.after_paths[name].name)
            self.assertEqual(source['text'].encode(), self.expected[name][self.after_paths[name].name])

    def assert_installed(self):
        for name in self.selected_names:
            relative, original = self.after_paths[name], self.original_paths[name]
            directory, old_directory = relative.parent, original.parent
            for leaf, expected in self.expected[name].items():
                self.assertEqual((self.rb / directory / leaf).read_bytes(), expected)
            self.assertEqual(physical_identity(self.rb / directory), self.before[old_directory.as_posix()][1])
            self.assertEqual(physical_identity(self.rb / relative), self.before[original.as_posix()][1])
            doc = self.open(self.b, self.rb, relative)
            self.assertEqual(doc['nodeId'], self.keys[name]['id'])
            resolved = self.call(self.b, '/api/data/open', root=str(self.rb),
                                 entityKey=f'weftext-node://{self.identity["workspaceId"]}/{self.keys[name]["id"]}')
            self.assertEqual(Path(resolved['path']), self.rb / relative)
            self.assertEqual(doc['recoveredDraft'], self.drafts[name])
            self.assertTrue(doc['recoveredDraftConflict'])
            for version, source in self.history[name].items():
                self.assertEqual(self.call(self.b, '/api/history/read', sessionId=doc['sessionId'],
                                          versionId=version)['source'], source)
            for leaf in ['personal.raw', 'opaque', 'opaque/keep.raw']:
                before = self.before[(old_directory / leaf).as_posix()]
                actual = self.rb / directory / leaf
                self.assertEqual(physical_identity(actual), before[1])
                if actual.is_file():
                    self.assertEqual(hashlib.sha256(actual.read_bytes()).hexdigest(), before[3])
            if name in self.moved_names:
                for leaf in ['baseline.png', 'weftext.resources.json', 'weftext.annotations.json']:
                    self.assertEqual(physical_identity(self.rb / directory / leaf), self.before[(old_directory / leaf).as_posix()][1])
                resources = self.resources(self.b, doc)
                self.assertEqual(set(resources), {'baseline.png', 'added.png'})
                self.assertEqual(resources['baseline.png']['resourceId'], self.initial_resources[name]['resourceId'])
                self.assertEqual(resources['baseline.png']['revision'], self.initial_resources[name]['revision'])
                for leaf, resource in resources.items():
                    self.assertEqual(resource['resourceId'], self.sender_resources[name][leaf]['resourceId'])
                    self.assertEqual(resource['currentness'], 'accepted')
                    self.assertEqual(self.call(self.b, '/api/resources/read', sessionId=doc['sessionId'],
                                              resource=resource['resource'], revision=resource['revision']),
                                     self.expected[name][leaf])
                annotations = json.loads((self.rb / directory / 'weftext.annotations.json').read_text(encoding='utf-8'))
                self.assertEqual({record['body'] for record in annotations['annotations']},
                                 {f'初始 {name} 原讨论😀', f'A final {name} discussion {self.layout} 😀'})
        self.assertEqual(physical_identity(self.rb), self.before['.'][1])
        for leaf in ['personal.raw', 'opaque/keep.raw']:
            self.assertEqual(physical_identity(self.rb / 'Container' / leaf), self.before[f'Container/{leaf}'][1])
        self.assertFalse((self.rb / 'Holding').exists(), 'A sender transit name is not an installed final object')
        self.assert_received_sources()

    def positive(self, layout, restart=False, late_raw=False):
        self.layout = layout
        self.produce(layout)
        plan = self.prepare()
        if restart:
            self.restart_b()
            self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.requested), plan)
        if late_raw:
            (self.rb / 'Left/late.raw').write_bytes(b'LATE LOCAL RAW; follows original Left')
            (self.rb / 'Left/late-directory').mkdir()
            (self.rb / 'Left/late-directory/keep.raw').write_bytes(b'LATE LOCAL CHILD')
            late = {str(path.relative_to(self.rb / 'Left')): (physical_identity(path), path.read_bytes() if path.is_file() else None)
                    for path in [self.rb / 'Left/late.raw', self.rb / 'Left/late-directory', self.rb / 'Left/late-directory/keep.raw']}
        receipt = self.commit(self.b, plan)
        self.assertEqual(self.commit(self.b, plan), receipt)
        self.assert_installed()
        if late_raw:
            for relative, (identity, data) in late.items():
                path = self.rb / self.after_paths['Left'].parent / relative
                self.assertEqual(physical_identity(path), identity)
                if data is not None:
                    self.assertEqual(path.read_bytes(), data)
        if restart:
            installed = tree_state(self.rb)
            self.restart_b()
            self.assertEqual(self.call(self.b, '/api/operations/status', operationId=plan['operationId']), receipt)
            self.assertEqual(self.call(self.b, '/api/operations/prepare', **self.requested)['planId'], plan['planId'])
            self.assertEqual(self.commit(self.b, plan), receipt)
            self.assertEqual(tree_state(self.rb), installed)
            self.assert_installed()

    def reject_current(self):
        current = tree_state(self.rb)
        self.commit(self.b, self.plan, expected=409)
        self.assertEqual(tree_state(self.rb), current, 'Every current identity and byte must survive refusal')
        original = self.call(self.b, '/api/sync/operation', operationId=self.plan['operationId'])
        self.assertEqual(original, {'request': self.requested, 'plan': self.plan})
        recovery = self.call(self.b, '/api/recovery/inspect', operationId=self.plan['operationId'])
        self.assertIn(recovery['coherence'], ['thirdState', 'unavailable'])
        self.assertEqual(recovery['allowedActions'], [])
        for action in ['adoptAfterImages', 'resumeOriginalPlan', 'keepCurrent']:
            self.call(self.b, '/api/recovery/resolve', expected=409, operationId=self.plan['operationId'], action=action)
        self.assertEqual(tree_state(self.rb), current)
        self.assert_received_sources()

    def test_sync_placement_exchange_original_request_restart_and_late_raw_preservation(self):
        self.positive('exchange', restart=True, late_raw=True)

    def test_sync_placement_three_root_cycle_with_resources_annotations_and_private_drafts(self):
        self.positive('rotate')

    def test_sync_placement_extract_child_then_parent_rename_preserves_independent_objects(self):
        self.positive('extract')

    def test_sync_placement_nested_parent_move_and_child_rename_use_final_parent(self):
        self.positive('nested')

    def test_sync_placement_exchange_late_foreign_participant_preserves_displaced_original(self):
        self.layout = 'exchange'
        self.produce(self.layout)
        self.prepare()
        retained = self.folder / 'retained-original-left'
        self.retain_original(self.rb / 'Left', retained)
        original = tree_state(retained)
        (self.rb / 'Left').mkdir()
        (self.rb / 'Left/foreign.raw').write_bytes(b'FOREIGN OWNER IN ORIGINAL PARTICIPANT SLOT')
        self.reject_current()
        self.assertEqual(tree_state(retained), original)

    def test_sync_placement_exchange_same_bytes_resource_replacement_refuses_every_effect(self):
        self.layout = 'exchange'
        self.produce(self.layout)
        self.prepare()
        source, retained = self.rb / 'Right/baseline.png', self.folder / 'retained-original-resource.png'
        original = physical_identity(source)
        self.retain_original(source, retained)
        source.write_bytes(retained.read_bytes())
        self.assertNotEqual(physical_identity(source), original)
        self.reject_current()
        self.assertEqual(physical_identity(retained), original)
        self.assertEqual(source.read_bytes(), retained.read_bytes())

    def test_sync_placement_extract_child_late_target_occupancy_blocks_parent_first_effect(self):
        self.layout = 'extract'
        self.produce(self.layout)
        self.prepare()
        (self.rb / 'Child').mkdir()
        (self.rb / 'Child/foreign.raw').write_bytes(b'FOREIGN CHILD TARGET OWNER')
        self.reject_current()
        self.assertFalse((self.rb / 'RenamedLeft').exists())


def browser_fixture(output):
    """Owned live fixture for the separate swap UI consumer, not a product API.

    The browser drives the actual application. A verify command only runs the
    same read-only identity/resource/history checks after its real commit.
    Closing the input pipe always closes both original owned processes.
    """
    os.environ['WEFTEXT_SYNC_EVIDENCE_DIR'] = str(Path(output).resolve())
    SyncPlacement.setUpClass()
    case = SyncPlacement('test_sync_placement_exchange_original_request_restart_and_late_raw_preservation')
    try:
        case.setUp()
        case.layout = 'exchange'
        case.produce(case.layout)
        case.close_server(case.b)
        case.b = case.start_server(['--no-browser', '--data-dir', str(case.folder / 'state-b'),
                                   '--open', str(case.rb / 'Left/Left.typ'), '--root', str(case.rb)], 'B-ui')
        base = case.open(case.b, case.rb, Path('Left/Left.typ'))
        fixture = {'folder': str(case.folder), 'root': str(case.rb), 'url': case.b.url, 'token': case.b.token,
                   'identity': case.identity, 'keys': case.keys, 'selectedNames': case.selected_names,
                   'baseSource': base['source'], 'initialDraft': case.drafts['Left'],
                   'expectedSource': case.expected['Left']['Right.typ'].decode(),
                   'afterPath': str(case.rb / 'Right/Right.typ'), 'nodeId': case.keys['Left']['id']}
        print('WEFTEXT_PLACEMENT_FIXTURE ' + json.dumps(fixture, ensure_ascii=True), flush=True)
        for line in sys.stdin:
            command = json.loads(line)
            if command['action'] == 'stop':
                break
            if command['action'] != 'verify':
                raise RuntimeError('Unknown fixture command')
            case.drafts['Left'] = command['draft']
            case.assert_installed()
            print('WEFTEXT_PLACEMENT_VERIFIED ' + json.dumps({'checked': True}), flush=True)
    finally:
        if not case.doCleanups():
            raise RuntimeError('An owned placement process did not close cleanly')


if __name__ == '__main__':
    if len(sys.argv) > 2 and sys.argv[2] == '--browser-fixture':
        if len(sys.argv) != 4 or not Path(sys.argv[1]).is_file():
            sys.exit('Usage: test_sync_placement.py <candidate.exe> --browser-fixture <fresh_output>')
        browser_fixture(sys.argv[3])
        sys.exit(0)
    available = {name for name in SyncPlacement.__dict__ if name.startswith('test_sync_placement_')}
    selected = set(sys.argv[2:])
    unknown = selected - available
    if unknown:
        sys.exit('Unknown test selection: ' + ', '.join(sorted(unknown)))
    if len(sys.argv) < 2 or not Path(sys.argv[1]).is_file():
        sys.exit('Usage: test_sync_placement.py <candidate.exe> [exact_test_name ...]')
    suite = unittest.TestSuite(SyncPlacement(name) for name in sorted(selected or available))
    sys.exit(not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful())
