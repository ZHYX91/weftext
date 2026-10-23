"""Real HTTP combinations of new participants and installed Node relocations.

The Rules cases deliberately clone only the four original Nodes; the later
external-resource adoption still reviews the complete sender installed cut.
New-parent cases use the original public create/move/rename/save producers.
Native cases also import into an already enabled sender and clone its exact
wrapper/Node/resource closure, preserving the uploaded original package.
There are no synthetic received states, private-record rewrites or failpoints.
Controlled interrupted-install recovery is tested separately by Core.

Run: python scripts/test_sync_structure.py <candidate.exe> [exact_test_name ...]
Only this class's structure cases run. Unknown selections fail before startup;
unsupported planner refusals cannot count as successful negative cases.
"""
from _creation_result import created_document
import base64
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import sys
import unittest
import zipfile

from test_hosted import uid
from test_sync_compound import SyncCompound, evidence_directory, physical_identity, png, tree_state


def native_nested_parent(schema):
    """A legal authored Native upload, with no foreign Core or Sync records."""
    workspace, node, resource = (uid() for _ in range(3))
    image = png((80, 130, 240))
    files = {
        '.weftext-schema.json': schema.encode(),
        'NewParent/NewParent.typ': (
            f'#metadata((id: "{node}", types: ())) <weftext>\n'
            '= Native nested parent\nNATIVE NESTED RESOURCE SOURCE\n'
            '#image("assets/icons/nested.png", width: 12pt)\n').encode(),
        'NewParent/assets/icons/nested.png': image,
        'NewParent/weftext.resources.json': json.dumps({
            'version': 1, 'resources': [{'resourceId': resource, 'path': 'assets/icons/nested.png'}]
        }).encode(),
    }
    roles = {
        '.weftext-schema.json': {'role': 'schema'},
        'NewParent/NewParent.typ': {'role': 'source', 'nodeId': node},
        'NewParent/assets/icons/nested.png': {'role': 'resource', 'ownerNodeId': node, 'resourceId': resource},
        'NewParent/weftext.resources.json': {'role': 'resourceManifest', 'ownerNodeId': node},
    }
    manifest = {'format': 'weftext.native-package', 'version': 2, 'packageId': uid(),
                'workspaceId': workspace, 'scope': 'library', 'schemaPath': '.weftext-schema.json',
                'losses': [], 'entries': {path: {**roles[path], 'bytes': len(data),
                    'sha256': hashlib.sha256(data).hexdigest()} for path, data in files.items()}}
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, 'w', zipfile.ZIP_STORED) as archive:
        archive.writestr('manifest.json', json.dumps(manifest))
        for path, data in files.items():
            archive.writestr(path, data)
    return buffer.getvalue(), node, resource, image


class SyncStructure(SyncCompound):
    @classmethod
    def setUpClass(cls):
        if os.name != 'nt':
            raise RuntimeError('This harness requires the Windows candidate and physical identity API')
        cls.evidence = evidence_directory('weftext-sync-structure-')
        print('Retained sync structure evidence:', cls.evidence, flush=True)

    def initial_objects(self):
        # This is a real partial clone selection, without intercepting any
        # response or removing subsequent portable dependencies. Its Rules
        # hydration plus relocations reproduces the retained stage-two cut.
        objects = list(self.keys.values())
        self.assertEqual(len(objects), 4)
        self.assertTrue(all(key['kind'] == 'node' for key in objects))
        self.assertNotIn({'kind': 'rules'}, objects)
        if hasattr(self, 'native_wrapper_key'):
            # Native's real original CreateTree now admits its wrapper. That
            # pre-existing, empty Directory is an installed baseline member;
            # NewParent has already moved out and remains genuinely missing.
            self.assertIn(self.native_wrapper_key, self.binding['installedObjects'])
            objects.append(self.native_wrapper_key)
        print('Initial real Clone selection:', json.dumps(objects, sort_keys=True), flush=True)
        return objects

    def prepare_source_before_sync(self):
        if self._testMethodName != 'test_sync_structure_new_parent_with_native_nested_resource_and_two_relocations':
            return
        schema = self.call(self.a, '/api/data/schema', root=str(self.ra))['source']
        payload, original_node, original_resource, image = native_nested_parent(schema)
        (self.folder / 'native-nested-parent.weftext.zip').write_bytes(payload)
        anchor = self.open(self.a, self.ra, Path('Still/Still.typ'))
        before = tree_state(self.ra)
        request = {'operationId': uid(), 'sessionId': anchor['sessionId'], 'baseId': anchor['baseId'],
                   'root': str(self.ra), 'parent': str(self.ra), 'name': 'NativeUpload', 'format': 'native',
                   'bytesBase64': base64.b64encode(payload).decode()}
        plan = self.call(self.a, '/api/exchange/prepare', **request)
        self.assertEqual(plan['kind'], 'importNative')
        self.assertEqual(tree_state(self.ra), before)
        self.assertEqual(self.call(self.a, '/api/exchange/prepare', **request), plan)
        receipt = self.call(self.a, '/api/exchange/commit', operationId=plan['operationId'],
                            planId=plan['planId'], acceptLosses=True)
        self.assertEqual(receipt['status'], 'committed')
        self.assertNotEqual(plan['nodeMap'][original_node], original_node)
        self.assertEqual((self.ra / 'NativeUpload/.weftext-import-originals/original.weftext').read_bytes(), payload)
        self.native_original_payload = payload
        marker = json.loads((self.ra / 'NativeUpload/weftext.directory.json').read_bytes())
        self.native_wrapper_key = {'kind': 'directory', 'id': marker['directoryId']}
        imported = self.open(self.a, self.ra, Path('NativeUpload/NewParent/NewParent.typ'))
        self.assertEqual(imported['nodeId'], plan['nodeMap'][original_node])
        resource = self.resources(self.a, imported)['nested.png']
        self.assertNotEqual(resource['resourceId'], original_resource)
        self.assertEqual(self.call(self.a, '/api/resources/read', sessionId=imported['sessionId'],
                                   resource=resource['resource'], revision=resource['revision']), image)
        paths = ['NewParent.typ', 'weftext.resources.json', 'assets', 'assets/icons', 'assets/icons/nested.png']
        identities = {path: physical_identity(self.ra / 'NativeUpload/NewParent' / path) for path in paths}
        directory_identity = physical_identity(self.ra / 'NativeUpload/NewParent')
        moved = self.call(self.a, '/api/operations/prepare', operationId=uid(), root=str(self.ra),
                          intent={'kind': 'moveNode', 'source': str(self.ra / 'NativeUpload/NewParent'),
                                  'parent': str(self.ra)})
        self.commit(self.a, moved)
        self.assertFalse((self.ra / 'NativeUpload/NewParent').exists())
        self.assertEqual(physical_identity(self.ra / 'NewParent'), directory_identity)
        for path, identity in identities.items():
            self.assertEqual(physical_identity(self.ra / 'NewParent' / path), identity)
        self.nested_parent_doc = self.open(self.a, self.ra, Path('NewParent/NewParent.typ'))
        self.assertEqual(self.nested_parent_doc['nodeId'], imported['nodeId'])
        self.nested_image = image

    def setUp(self):
        # Shorten physical paths without changing the method identity used by
        # the real Native producer in prepare_source_before_sync.
        names = sorted(name for name in type(self).__dict__ if name.startswith('test_sync_structure_'))
        self.fixture_directory_name = f'c{names.index(self._testMethodName) + 1:02d}'
        super().setUp()
        binding = self.call(self.b, '/api/sync/binding', root=str(self.rb))
        self.assertNotIn({'kind': 'rules'}, binding['installedObjects'])
        self.assertFalse((self.rb / '.weftext-rules').exists())
        self.assertNotIn('.weftext-rules', self.before)
        if hasattr(self, 'native_wrapper_key'):
            self.assertIn(self.native_wrapper_key, binding['installedObjects'])
            self.assertFalse((self.rb / 'NewParent').exists(), 'The genuine initial wrapper Clone must not install NewParent')
            self.assertNotIn({'kind': 'node', 'id': self.nested_parent_doc['nodeId']}, binding['installedObjects'])
        self.rules_bytes = (self.ra / '.weftext-rules').read_bytes()

    def assert_rules_installed(self):
        path = self.rb / '.weftext-rules'
        self.assertEqual(path.read_bytes(), self.rules_bytes)
        self.assertNotEqual(physical_identity(path), physical_identity(self.ra / '.weftext-rules'))
        binding = self.call(self.b, '/api/sync/binding', root=str(self.rb))
        self.assertIn({'kind': 'rules'}, binding['installedObjects'])
        history = self.call(self.b, '/api/sync/history', identity=self.identity, object={'kind': 'rules'})
        self.assertEqual(history['coverage'], 'complete')
        self.assertTrue(history['versions'])
        for version in history['versions']:
            file = self.call(self.b, '/api/sync/file', identity=self.identity, object={'kind': 'rules'},
                             revision=version['revision'], path='.weftext-rules')
            self.assertEqual(file['text'].encode(), self.rules_bytes)

    def assert_original_prepared_after_restart(self, request, plan):
        self.restart_b()
        self.assertEqual(tree_state(self.rb), self.before)
        original = self.call(self.b, '/api/sync/operation', operationId=plan['operationId'])
        self.assertEqual(original['request'], request)
        self.assertEqual(original['plan'], plan)
        self.assertEqual(self.call(self.b, '/api/operations/status', operationId=plan['operationId'])['status'], 'prepared')
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **request), plan)

    def new_parent_changes(self, existing_sender_parent=None):
        """A creates the missing parent and all versions through real HTTP."""
        self.new_parent = existing_sender_parent or created_document(self.call(
            self.a, '/api/note/create', root=str(self.ra), parent=str(self.ra), name='NewParent', operationId=uid()))
        self.new_parent_key = {'kind': 'node', 'id': self.new_parent['nodeId']}
        self.keys['NewParent'] = self.new_parent_key
        self.save(self.a, self.ra, Path('NewParent/NewParent.typ'), 'A 新父节点原生正文😀')
        self.new_parent_bytes = (self.ra / 'NewParent/NewParent.typ').read_bytes()
        renamed = self.call(self.a, '/api/operations/prepare', operationId=uid(), root=str(self.ra),
                            intent={'kind': 'renameNode', 'source': str(self.ra / 'Left'), 'name': 'Renamed'})
        self.commit(self.a, renamed)
        moved = self.call(self.a, '/api/operations/prepare', operationId=uid(), root=str(self.ra),
                          intent={'kind': 'moveNode', 'source': str(self.ra / 'Right'),
                                  'parent': str(self.ra / 'NewParent')})
        self.commit(self.a, moved)
        self.after_paths['Right'] = Path('NewParent/Right/Right.typ')
        for number, name in enumerate(['Left', 'Right']):
            relative = self.after_paths[name]
            self.save(self.a, self.ra, relative, f'A {name} 同次 Apply 正文更新😀')
            self.adopt_resource(relative, png((0, 255, 50 + number)))
            self.import_resource(self.a, self.ra, relative, 'added.png', png((20, number, 255)))
            self.annotate(self.a, self.ra, relative, f'A {name} 随移动的新讨论😀')
        self.save(self.a, self.ra, self.after_paths['Still'], 'A 未移动对象同次内容更新😀')
        for name, relative in self.after_paths.items():
            directory = (self.ra / relative).parent
            filenames = [relative.name]
            if name != 'Still':
                filenames += ['baseline.png', 'added.png', 'weftext.resources.json', 'weftext.annotations.json']
            self.expected[name] = {filename: (directory / filename).read_bytes() for filename in filenames}
        received = self.transfer(self.a, self.ra, self.b,
                                 [self.keys[name] for name in ['Left', 'Right', 'Still']])
        self.received = {entry['object']['id']: entry['view'] for entry in received['objects']
                         if entry['object']['kind'] == 'node'}
        self.assertTrue({self.keys[name]['id'] for name in ['Left', 'Right', 'Still', 'NewParent']}.issubset(self.received))
        self.assertEqual(tree_state(self.rb), self.before, 'Receive must not create the parent or relocate any member')
        self.assertFalse((self.rb / 'NewParent').exists())
        self.assertFalse((self.rb / '.weftext-rules').exists())
        self.assert_received_sources()
        view = self.received[self.new_parent_key['id']]
        self.assertEqual(len(view['versions']), 1)
        incoming = self.call(self.b, '/api/sync/file', identity=self.identity, object=self.new_parent_key,
                             revision=view['versions'][0]['revision'], path='NewParent.typ')
        self.assertEqual(incoming['text'].encode(), self.new_parent_bytes)

    def assert_new_parent_installed(self):
        self.assert_installed()
        self.assert_rules_installed()
        parent = self.rb / 'NewParent'
        source = parent / 'NewParent.typ'
        self.assertEqual(source.read_bytes(), self.new_parent_bytes)
        doc = self.open(self.b, self.rb, Path('NewParent/NewParent.typ'))
        self.assertEqual(doc['nodeId'], self.new_parent_key['id'])
        self.assertEqual(doc['workspaceId'], self.identity['workspaceId'])
        self.assertNotEqual(physical_identity(parent), physical_identity(self.ra / 'NewParent'))
        self.assertNotEqual(physical_identity(source), physical_identity(self.ra / 'NewParent/NewParent.typ'))
        binding = self.call(self.b, '/api/sync/binding', root=str(self.rb))
        self.assertIn(self.new_parent_key, binding['installedObjects'])

    def prepare_native_after_enable(self):
        """Import into an already sync-enabled library through the real upload."""
        schema = self.call(self.a, '/api/data/schema', root=str(self.ra))['source']
        payload, original_node, original_resource, image = native_nested_parent(schema)
        (self.folder / 'original-after-enable.weftext.zip').write_bytes(payload)
        anchor = self.open(self.a, self.ra, Path('Still/Still.typ'))
        request = {'operationId': uid(), 'sessionId': anchor['sessionId'], 'baseId': anchor['baseId'],
                   'root': str(self.ra), 'parent': str(self.ra), 'name': 'NativeAfterEnable',
                   'format': 'native', 'bytesBase64': base64.b64encode(payload).decode()}
        before = tree_state(self.ra)
        plan = self.call(self.a, '/api/exchange/prepare', **request)
        self.assertEqual(plan['kind'], 'importNative')
        self.assertEqual(plan['status'], 'prepared')
        self.assertEqual(plan['operationId'], request['operationId'])
        self.assertNotEqual(plan['nodeMap'][original_node], original_node)
        self.assertEqual(tree_state(self.ra), before)
        self.assertFalse((self.ra / request['name']).exists())
        self.assertEqual(self.call(self.a, '/api/exchange/prepare', **request), plan)
        self.assertEqual(tree_state(self.ra), before)
        return {'request': request, 'plan': plan, 'before': before, 'payload': payload,
                'originalNode': original_node, 'originalResource': original_resource, 'image': image}

    def commit_native(self, case, expected=200):
        plan = case['plan']
        receipt = self.call(self.a, '/api/exchange/commit', expected=expected,
                            operationId=plan['operationId'], planId=plan['planId'], acceptLosses=True)
        if expected == 200:
            self.assertEqual(receipt['operationId'], plan['operationId'])
            self.assertEqual(receipt['planId'], plan['planId'])
            self.assertEqual(receipt['status'], 'committed')
        return receipt

    def restart_a(self):
        self.close_server(self.a)
        self.a = self.start_server([ '--data-dir', str(self.folder / 'state-a')], 'A-restart')

    def assert_native_after_enable(self, case):
        target = self.ra / case['request']['name']
        original = target / '.weftext-import-originals/original.weftext'
        self.assertEqual(original.read_bytes(), case['payload'])
        marker = json.loads((target / 'weftext.directory.json').read_bytes())
        directory_key = {'kind': 'directory', 'id': marker['directoryId']}
        reference = f'weftext-directory://{self.identity["workspaceId"]}/{marker["directoryId"]}'
        resolved = self.call(self.a, '/api/navigation/resolve', root=str(self.ra), targetRef=reference)
        self.assertEqual(resolved['status'], 'managed')
        self.assertEqual(Path(resolved['path']), target)
        binding = self.call(self.a, '/api/sync/binding', root=str(self.ra))
        node_key = {'kind': 'node', 'id': case['plan']['nodeMap'][case['originalNode']]}
        self.assertIn(directory_key, binding['installedObjects'])
        self.assertIn(node_key, binding['installedObjects'])
        current = tree_state(self.ra)
        unchanged = self.call(self.a, '/api/sync/adoption-cut', expected=409,
                              root=str(self.ra), identity=self.identity, objects=[directory_key, node_key])
        self.assertEqual(unchanged['error']['code'], 'noChange',
                         'Original imported wrapper and Node observations must remain readable, with no external edit')
        self.assertEqual(tree_state(self.ra), current)
        relative = Path(case['request']['name']) / 'NewParent/NewParent.typ'
        imported = self.open(self.a, self.ra, relative)
        self.assertEqual(imported['nodeId'], node_key['id'])
        self.assertEqual(imported['workspaceId'], self.identity['workspaceId'])
        resource = self.resources(self.a, imported)['nested.png']
        self.assertNotEqual(resource['resourceId'], case['originalResource'])
        resource_path = target / 'NewParent/assets/icons/nested.png'
        mapped = [reference for location, reference in case['plan']['resourceMap'].items()
                  if Path(location) == resource_path]
        self.assertEqual(mapped, [resource['resource']],
                         'The real target path must retain its exact frozen resource reference')
        self.assertEqual(resource['nodeId'], node_key['id'])
        self.assertEqual(resource['currentness'], 'accepted')
        self.assertEqual(resource['metadata'], {'kind': 'image', 'width': 2, 'height': 2})
        self.assertTrue(resource['annotationReady'])
        self.assertIsNotNone(resource['revision'])
        self.assertEqual(self.call(self.a, '/api/resources/read', sessionId=imported['sessionId'],
                                   resource=resource['resource'], revision=resource['revision']), case['image'])
        self.assertFalse((target / 'NewParent/assets/weftext.directory.json').exists(),
                         'Owned resource folders are not independent navigation identities')
        self.assertEqual(tree_state(self.rb), self.before, 'Independent receiver originals must remain untouched')
        return {'relative': relative, 'doc': imported, 'resource': resource,
                'nodeKey': node_key, 'directoryKey': directory_key, 'target': target}

    def clone_native_after_enable(self, case, imported):
        clone_root = self.folder / 'library-native-clone'
        clone = self.start_server([ '--data-dir', str(self.folder / 'state-native-clone')], 'C-clone')
        received = self.transfer(self.a, self.ra, clone, [imported['nodeKey']])
        received_keys = [entry['object'] for entry in received['objects']]
        self.assertIn(imported['nodeKey'], received_keys)
        self.assertIn(imported['directoryKey'], received_keys,
                      'Real export must retain the imported Node wrapper dependency')
        request = {'operationId': uid(), 'root': str(clone_root), 'intent': {
            'kind': 'cloneSync', 'identity': self.identity, 'objects': [imported['nodeKey']]}}
        plan = self.call(clone, '/api/operations/prepare', **request)
        self.assertFalse(clone_root.exists())
        self.assertEqual(plan['nodeMap'][imported['nodeKey']['id']], imported['nodeKey']['id'])
        receipt = self.commit(clone, plan)
        doc = self.open(clone, clone_root, imported['relative'])
        self.assertEqual(doc['nodeId'], imported['doc']['nodeId'])
        self.assertEqual(doc['workspaceId'], self.identity['workspaceId'])
        self.assertEqual(doc['source'], imported['doc']['source'])
        resource = self.resources(clone, doc)['nested.png']
        for field in ['resourceId', 'resource', 'nodeId', 'revision', 'metadata']:
            self.assertEqual(resource[field], imported['resource'][field])
        self.assertEqual(resource['currentness'], 'accepted')
        self.assertEqual(self.call(clone, '/api/resources/read', sessionId=doc['sessionId'],
                                   resource=resource['resource'], revision=resource['revision']), case['image'])
        relative_target = Path(case['request']['name'])
        self.assertEqual((clone_root / relative_target / 'weftext.directory.json').read_bytes(),
                         (imported['target'] / 'weftext.directory.json').read_bytes())
        binding = self.call(clone, '/api/sync/binding', root=str(clone_root))
        self.assertIn(imported['directoryKey'], binding['installedObjects'])
        self.assertIn(imported['nodeKey'], binding['installedObjects'])
        for relative in [relative_target, imported['relative'], relative_target / 'NewParent/assets',
                         relative_target / 'NewParent/assets/icons', relative_target / 'NewParent/assets/icons/nested.png']:
            self.assertNotEqual(physical_identity(clone_root / relative), physical_identity(self.ra / relative))
        installed = tree_state(clone_root)
        self.assertEqual(self.commit(clone, plan), receipt)
        self.assertEqual(tree_state(clone_root), installed, 'Clone replay must preserve each original materialized object')
        self.assertEqual((imported['target'] / '.weftext-import-originals/original.weftext').read_bytes(), case['payload'])

    def test_sync_structure_native_after_enable_wrapper_and_nested_resource_clone(self):
        case = self.prepare_native_after_enable()
        receipt = self.commit_native(case)
        imported = self.assert_native_after_enable(case)
        installed = tree_state(self.ra)
        self.assertEqual(self.commit_native(case), receipt)
        self.assertEqual(tree_state(self.ra), installed, 'Import retry must preserve wrapper, IDs, resources and original ZIP')
        self.clone_native_after_enable(case, imported)

    def test_sync_structure_native_after_enable_restart_keeps_original_plan_and_receipt(self):
        case = self.prepare_native_after_enable()
        self.restart_a()
        self.assertEqual(tree_state(self.ra), case['before'])
        self.assertEqual(self.call(self.a, '/api/operations/status', operationId=case['plan']['operationId']), case['plan'])
        self.assertEqual(self.call(self.a, '/api/exchange/prepare', **case['request']), case['plan'])
        # Ignore the actual committed response; the next process must recover it.
        receipt = self.commit_native(case)
        imported = self.assert_native_after_enable(case)
        installed = tree_state(self.ra)
        self.restart_a()
        status = self.call(self.a, '/api/operations/status', operationId=case['plan']['operationId'])
        self.assertEqual(status, {**receipt, 'kind': case['plan']['kind']})
        self.assertEqual(self.commit_native(case), receipt)
        repeated = self.call(self.a, '/api/exchange/prepare', **case['request'])
        self.assertEqual(repeated, {**case['plan'], 'status': 'committed'})
        self.assertEqual(repeated['nodeMap'], case['plan']['nodeMap'])
        self.assertEqual(tree_state(self.ra), installed)
        after = self.assert_native_after_enable(case)
        self.assertEqual(after['directoryKey'], imported['directoryKey'])
        self.assertTrue(after['resource']['readBasis'])
        self.assertEqual({key: value for key, value in after['resource'].items() if key != 'readBasis'},
                         {key: value for key, value in imported['resource'].items() if key != 'readBasis'})
        self.clone_native_after_enable(case, after)

    def test_sync_structure_native_after_enable_late_target_occupied_preserves_every_effect(self):
        case = self.prepare_native_after_enable()
        self.restart_a()
        self.assertEqual(self.call(self.a, '/api/exchange/prepare', **case['request']), case['plan'])
        target = self.ra / case['request']['name']
        target.mkdir()
        original = b'FOREIGN NATIVE UPLOAD TARGET; preserve original owner'
        (target / 'external.raw').write_bytes(original)
        identity = physical_identity(target)
        current = tree_state(self.ra)
        self.commit_native(case, expected=409)
        self.assertEqual(tree_state(self.ra), current)
        self.assertEqual(physical_identity(target), identity)
        self.assertEqual((target / 'external.raw').read_bytes(), original)
        self.assertFalse((target / 'weftext.directory.json').exists())
        self.assertFalse((target / 'NewParent').exists())
        self.assertFalse((target / '.weftext-import-originals').exists())
        self.assertEqual(self.call(self.a, '/api/operations/status', operationId=case['plan']['operationId']), case['plan'])
        inspected = self.call(self.a, '/api/recovery/inspect', operationId=case['plan']['operationId'])
        self.assertIn(inspected['coherence'], ('thirdState', 'unavailable'))
        self.assertEqual(inspected['allowedActions'], [])
        for action in ['adoptAfterImages', 'resumeOriginalPlan', 'keepCurrent']:
            self.call(self.a, '/api/recovery/resolve', expected=409,
                      operationId=case['plan']['operationId'], action=action)
        self.assertEqual(tree_state(self.ra), current)
        self.assertEqual(tree_state(self.rb), self.before)

    def test_sync_structure_missing_rules_two_relocations_content_and_private_facts(self):
        # Reuse the already accepted source/resource/annotation/Still, physical
        # identity, raw companion, personal Draft and exact history assertions.
        self.test_sync_compound_two_moves_content_resources_annotations_and_private_facts()
        self.assert_rules_installed()

    def test_sync_structure_directory_move(self):
        """Directory relocation revalidates its child's exact older dependency."""
        directory = self.ra / 'Folder'
        directory.mkdir()
        admission = self.call(self.a, '/api/navigation/admit-directory', root=str(self.ra),
                              path=str(directory), operationId=uid())
        self.commit(self.a, admission['preview'])
        marker = json.loads((directory / 'weftext.directory.json').read_bytes())
        key = {'kind': 'directory', 'id': marker['directoryId']}
        reference = f'weftext-directory://{self.identity["workspaceId"]}/{marker["directoryId"]}'
        move_child = self.call(self.a, '/api/operations/prepare', operationId=uid(), root=str(self.ra),
                               intent={'kind': 'moveNode', 'source': str(self.ra / 'Left'),
                                       'parent': str(directory)})
        self.commit(self.a, move_child)
        objects = [key, self.keys['Left']]
        self.transfer(self.a, self.ra, self.b, objects)
        first = self.call(self.b, '/api/operations/prepare', operationId=uid(), root=str(self.rb),
                          intent={'kind': 'applySync', 'identity': self.identity, 'objects': objects})
        self.commit(self.b, first)
        self.assertFalse((self.rb / 'Left').exists())
        before = tree_state(self.rb / 'Folder')
        original_marker = (self.rb / 'Folder/weftext.directory.json').read_bytes()
        current = self.open(self.b, self.rb, Path('Folder/Left/Left.typ'))
        self.assertEqual(current['recoveredDraft'], self.drafts['Left'])
        self.assertEqual(current['nodeId'], self.keys['Left']['id'])
        original_history = self.call(self.b, '/api/history/list', sessionId=current['sessionId'])
        original_versions = {version['versionId'] for version in original_history['versions']}
        resource_before = self.resources(self.b, current)['baseline.png']

        sender = self.open(self.a, self.ra, Path('Folder/Left/Left.typ'))
        sender_draft = sender['source'] + '\nDirectory moved with unsaved sender draft 中文😀'
        self.call(self.a, '/api/draft', sessionId=sender['sessionId'], baseId=sender['baseId'],
                  source=sender_draft)
        sender_before = tree_state(directory)
        request = {'operationId': uid(), 'root': str(self.ra),
                   'intent': {'kind': 'relocateDirectory', 'source': str(directory),
                              'parent': str(self.ra / 'Container'), 'name': 'MovedFolder'}}
        planned = self.call(self.a, '/api/operations/prepare', **request)
        self.assertEqual(planned['kind'], 'relocateDirectory')
        self.assertEqual(tree_state(directory), sender_before)
        self.assertEqual(self.call(self.a, '/api/operations/prepare', **request), planned)
        receipt = self.commit(self.a, planned)
        moved = self.ra / 'Container/MovedFolder'
        self.assertFalse(directory.exists())
        self.assertEqual(tree_state(moved), sender_before)
        self.assertFalse((moved / 'MovedFolder.typ').exists())
        effects = receipt.get('affectedDocuments', [])
        rebound = next(effect['document'] for effect in effects
                       if effect['priorSessionId'] == sender['sessionId'])
        self.assertEqual(Path(rebound['path']), moved / 'Left/Left.typ')
        self.assertEqual(rebound['nodeId'], sender['nodeId'])
        self.assertEqual(rebound['recoveredDraft'], sender_draft)
        target = self.call(self.a, '/api/navigation/resolve', root=str(self.ra), targetRef=reference)
        self.assertEqual(target['status'], 'managed')
        self.assertEqual(Path(target['path']), moved)
        self.assertEqual(self.commit(self.a, planned)['operationId'], planned['operationId'])
        self.assertEqual(tree_state(moved), sender_before)

        self.transfer(self.a, self.ra, self.b, objects)
        receiver_before = tree_state(self.rb)
        apply_request = {'operationId': uid(), 'root': str(self.rb),
                         'intent': {'kind': 'applySync', 'identity': self.identity, 'objects': objects}}
        refused = self.call(self.b, '/api/operations/prepare', expected=409, **apply_request)
        self.assertEqual(refused['error']['code'], 'syncDependencyConflict',
                         'The installed child still names the original Folder revision; Apply must not silently upgrade it')
        self.assertEqual(tree_state(self.rb), receiver_before)
        self.assertEqual(self.call(self.b, '/api/operations/status',
                                   operationId=apply_request['operationId'])['status'], 'unknown')
        cut = self.call(self.b, '/api/sync/revalidation-cut', root=str(self.rb),
                        identity=self.identity, objects=objects)
        self.assertEqual(cut['coverage'], 'complete')
        choices = []
        for object_key in objects:
            entry = next(item for item in cut['objects'] if item['object'] == object_key)
            self.assertEqual(len(entry['view']['versions']), 1)
            version = entry['view']['versions'][0]
            choices.append({'object': object_key, 'revisions': [version['revision']],
                            'decision': {'kind': 'preserve', 'revision': version['revision']},
                            'state': copy.deepcopy(version['state']), 'reviewedDependencies': []})
            if object_key == self.keys['Left']:
                self.assertIn(key, entry['unsettledDependencies'])
                source = self.call(self.b, '/api/sync/file', identity=self.identity, object=object_key,
                                   revision=version['revision'], path='Left.typ')
                self.assertEqual(source['text'], current['source'])
                self.assertEqual(source['fileRevision'], version['state']['fileRevisions']['Left.typ'])
                self.assertEqual(source['digest'], version['state']['files']['Left.typ'])
                original_child = copy.deepcopy(version)
        operation_id = uid()
        review = self.call(self.b, '/api/sync/revalidation-review', root=str(self.rb), identity=self.identity,
                           operationId=operation_id, resolutions=choices, blobs={})
        self.assertEqual(review['operationId'], operation_id)
        self.assertEqual(len(review['resolutions']), len(choices))
        for expected in choices:
            actual = next(item for item in review['resolutions'] if item['object'] == expected['object'])
            for field in ['object', 'revisions', 'decision', 'state']:
                self.assertEqual(actual[field], expected[field])
        self.assertEqual(tree_state(self.rb), receiver_before, 'Explicit dependency review has no author effect')
        self.assertEqual(self.call(self.b, '/api/operations/status', operationId=operation_id)['status'], 'unknown')
        received_request = {'operationId': operation_id, 'root': str(self.rb), 'intent': {
            'kind': 'revalidateSync', 'identity': self.identity,
            'expectedInstalled': review['expectedInstalled'], 'reviewedVersions': review['reviewedVersions'],
            'resolutions': review['resolutions'], 'blobs': {}}}
        (self.folder / 'directory-revalidation.json').write_text(json.dumps({
            'rejectedApply': apply_request, 'review': review, 'request': received_request,
            'receiverBefore': receiver_before, 'folderBefore': before,
        }, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
        try:
            received_plan = self.call(self.b, '/api/operations/prepare', **received_request)
        except AssertionError:
            self.assertEqual(tree_state(self.rb), receiver_before, 'An unsuccessful prepare must preserve the exact author tree')
            self.assertEqual(self.call(self.b, '/api/operations/status', operationId=operation_id)['status'], 'unknown')
            raise
        self.assertEqual(received_plan['kind'], 'revalidateSync')
        self.assertEqual(received_plan['operationId'], operation_id)
        self.assertEqual(received_plan['status'], 'prepared')
        self.assertEqual(tree_state(self.rb), receiver_before)
        self.assertEqual(tree_state(self.rb / 'Folder'), before)
        self.restart_b()
        self.assertEqual(tree_state(self.rb), receiver_before)
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **received_request), received_plan)
        self.assertEqual(self.call(self.b, '/api/sync/operation', operationId=operation_id),
                         {'request': received_request, 'plan': received_plan})
        self.commit(self.b, received_plan)
        receiver_target = self.rb / 'Container/MovedFolder'
        self.assertFalse((self.rb / 'Folder').exists())
        self.assertEqual(tree_state(receiver_target), before)
        self.assertEqual((receiver_target / 'weftext.directory.json').read_bytes(), original_marker)
        self.assertFalse((receiver_target / 'MovedFolder.typ').exists())
        target = self.call(self.b, '/api/navigation/resolve', root=str(self.rb), targetRef=reference)
        self.assertEqual(target['status'], 'managed')
        self.assertEqual(Path(target['path']), receiver_target)
        child = self.open(self.b, self.rb, Path('Container/MovedFolder/Left/Left.typ'))
        self.assertEqual(child['nodeId'], self.keys['Left']['id'])
        self.assertEqual(child['recoveredDraft'], self.drafts['Left'])
        history = self.call(self.b, '/api/history/list', sessionId=child['sessionId'])
        self.assertEqual({version['versionId'] for version in history['versions']}, original_versions)
        for version, source in self.history['Left'].items():
            self.assertEqual(self.call(self.b, '/api/history/read', sessionId=child['sessionId'],
                                       versionId=version)['source'], source)
        resource = self.resources(self.b, child)['baseline.png']
        for field in ['resourceId', 'resource', 'nodeId', 'revision']:
            self.assertEqual(resource[field], resource_before[field])
        self.assertEqual(self.call(self.b, '/api/resources/read', sessionId=child['sessionId'],
                                   resource=resource['resource'], revision=resource['revision']), self.baseline_png)
        self.call(self.b, '/api/sync/publish', root=str(self.rb))
        state = self.call(self.b, '/api/sync/status', identity=self.identity)
        child_versions = next(item['view']['versions'] for item in state['objects']
                              if item['object'] == self.keys['Left'])
        self.assertEqual(len(child_versions), 1)
        self.assertNotEqual(child_versions[0]['revision'], original_child['revision'])
        for field in ['files', 'fileRevisions']:
            self.assertEqual(child_versions[0]['state'][field], original_child['state'][field])
        after = tree_state(self.rb)
        for relative, original in receiver_before.items():
            if relative != 'Folder' and not relative.startswith('Folder/'):
                self.assertEqual(after[relative], original, f'Directory revalidation preserves unrelated {relative}')
        installed = tree_state(self.rb)
        self.restart_b()
        status = self.call(self.b, '/api/operations/status', operationId=received_plan['operationId'])
        self.assertEqual(status['status'], 'committed')
        original = self.call(self.b, '/api/sync/operation', operationId=operation_id)
        self.assertEqual(original['request'], received_request)
        self.assertEqual(original['plan']['planId'], received_plan['planId'])
        self.assertEqual(original['plan']['status'], 'committed')
        self.assertEqual(self.commit(self.b, received_plan), status)
        self.assertEqual(tree_state(self.rb), installed)

    def test_sync_structure_missing_rules_restart_keeps_original_plan_and_receipt(self):
        self.test_sync_compound_restart_preserves_original_id_plan_and_committed_receipt()
        self.assert_rules_installed()

    def test_sync_structure_rules_slot_occupied_after_prepare_rejects_every_effect(self):
        self.changes()
        request = self.request()
        plan = self.prepare(request)
        self.assert_original_prepared_after_restart(request, plan)
        slot = self.rb / '.weftext-rules'
        self.assertFalse(slot.exists())
        external = json.loads(self.rules_bytes)
        external['ignored'] = [*external['ignored'], 'EXTERNAL_RULES_OWNER_KEEP']
        original = json.dumps(external, ensure_ascii=False, indent=2).encode() + b'\n'
        # The occupied file is valid Rules syntax and an actual new physical
        # member, rather than a forged installed revision or malformed parser
        # fixture. The original create-only slot must preserve its owner.
        with slot.open('xb') as handle:
            handle.write(original)
        identity = physical_identity(slot)
        current = tree_state(self.rb)
        self.assert_rejected_without_effect(plan, current)
        self.assertEqual(slot.read_bytes(), original)
        self.assertEqual(physical_identity(slot), identity)
        self.assertFalse((self.rb / 'Container/Right').exists())

    def structure_request(self, objects):
        order = {'node': 0, 'directory': 1, 'rootDirectory': 2, 'schema': 3,
                 'settings': 4, 'tags': 5, 'rules': 6}
        return {'operationId': uid(), 'root': str(self.rb), 'intent': {
            'kind': 'applySync', 'identity': self.identity,
            'objects': sorted(objects, key=lambda key: (order[key['kind']], key.get('id', '')))}}

    def configuration_move_case(self):
        for leaf in ['.weftext-schema.json', '.weftext-settings.json', '.weftext-rules']:
            self.assertFalse((self.rb / leaf).exists())
        schema = self.call(self.a, '/api/data/schema', root=str(self.ra))
        value = json.loads(schema['source'])
        value['aliases']['compound_configuration'] = 'tasks/task'
        source = json.dumps(value, ensure_ascii=False, indent=2) + '\n'
        planned = self.call(self.a, '/api/data/schema/prepare', root=str(self.ra), operationId=uid(),
                            source=source, beforeSource=schema['source'], beforeRevision=schema['revision'])
        self.assertEqual(self.call(self.a, '/api/data/schema/install', operationId=planned['operationId'],
                                   planId=planned['planId'])['status'], 'committed')
        style = self.call(self.a, '/api/styles/read', root=str(self.ra))
        planned = self.call(self.a, '/api/styles/prepare', root=str(self.ra), operationId=uid(),
                            baseId=style['baseId'], source=style['source'] + '\n// Explicit disabled configuration fixture\n',
                            enabled=False)
        installed = self.call(self.a, '/api/styles/apply', root=str(self.ra), operationId=planned['operationId'],
                              planId=planned['planId'])
        self.assertFalse(installed['enabled'])
        self.assertEqual(installed['operationReceipt']['status'], 'committed')
        self.changes()
        keys = [{'kind': kind} for kind in ['schema', 'settings', 'rules']]
        self.transfer(self.a, self.ra, self.b,
                      [self.keys[name] for name in ['Left', 'Right', 'Still']] + keys)
        self.configuration_bytes = {leaf: (self.ra / leaf).read_bytes() for leaf in [
            '.weftext-schema.json', '.weftext-settings.json', '.weftext-rules']}
        return self.structure_request([self.keys[name] for name in ['Left', 'Right', 'Still']] + keys)

    def test_sync_structure_missing_schema_settings_moves_share_original_plan(self):
        request = self.configuration_move_case()
        plan = self.prepare(request)
        self.assert_original_prepared_after_restart(request, plan)
        receipt = self.commit(self.b, plan)
        self.assert_installed()
        self.assert_rules_installed()
        graph = self.call(self.a, '/api/sync/status', identity=self.identity)
        for kind, leaf in [('schema', '.weftext-schema.json'), ('settings', '.weftext-settings.json'),
                           ('rules', '.weftext-rules')]:
            self.assertEqual((self.rb / leaf).read_bytes(), self.configuration_bytes[leaf])
            self.assertNotEqual(physical_identity(self.rb / leaf), physical_identity(self.ra / leaf))
            versions = next(item['view']['versions'] for item in graph['objects'] if item['object'] == {'kind': kind})
            self.assertEqual(len(versions), 1)
            self.assertEqual(plan['revisions'][str(self.rb / leaf)], versions[0]['state']['fileRevisions'][leaf])
        current = tree_state(self.rb)
        self.restart_b()
        self.assertEqual(self.call(self.b, '/api/sync/operation', operationId=plan['operationId'])['request'], request)
        self.assertEqual(self.commit(self.b, plan), receipt)
        self.assertEqual(tree_state(self.rb), current)

    def test_sync_structure_late_settings_slot_refuses_configuration_and_moves(self):
        request = self.configuration_move_case()
        plan = self.prepare(request)
        self.assert_original_prepared_after_restart(request, plan)
        slot = self.rb / '.weftext-settings.json'
        with slot.open('xb') as handle:
            handle.write(self.configuration_bytes['.weftext-settings.json'])
        foreign = physical_identity(slot)
        current = tree_state(self.rb)
        self.assert_rejected_without_effect(plan, current)
        self.assertEqual(slot.read_bytes(), self.configuration_bytes['.weftext-settings.json'])
        self.assertEqual(physical_identity(slot), foreign)
        self.assertFalse((self.rb / '.weftext-schema.json').exists())
        self.assertFalse((self.rb / '.weftext-rules').exists())
        self.assertEqual(self.call(self.b, '/api/sync/operation', operationId=plan['operationId'])['request'], request)

    def test_sync_structure_new_nodes_in_moving_tree_keep_births_and_resource(self):
        self.changes()
        child = created_document(self.call(self.a, '/api/note/create', root=str(self.ra), parent=str(self.ra / 'Renamed'),
                          name='NewChild', operationId=uid()))
        nested = created_document(self.call(self.a, '/api/note/create', root=str(self.ra), parent=str(self.ra / 'Renamed/NewChild'),
                           name='Nested', operationId=uid()))
        self.save(self.a, self.ra, Path('Renamed/NewChild/NewChild.typ'), 'Actual new child within moving tree 中文😀')
        self.import_resource(self.a, self.ra, Path('Renamed/NewChild/Nested/Nested.typ'), 'nested.bin', bytes([7, 0, 255]))
        created = [(child, Path('Renamed/NewChild/NewChild.typ')),
                   (nested, Path('Renamed/NewChild/Nested/Nested.typ'))]
        keys = [self.keys[name] for name in ['Left', 'Right', 'Still']] + [
            {'kind': 'node', 'id': doc['nodeId']} for doc, _ in created]
        self.transfer(self.a, self.ra, self.b, keys)
        request = self.structure_request(keys)
        plan = self.prepare(request)
        self.assertFalse((self.rb / 'Left/NewChild').exists())
        self.assertFalse((self.rb / 'Renamed').exists())
        self.assert_original_prepared_after_restart(request, plan)
        receipt = self.commit(self.b, plan)
        self.assert_installed()
        self.assert_rules_installed()
        for doc, relative in created:
            actual = self.open(self.b, self.rb, relative)
            self.assertEqual(actual['nodeId'], doc['nodeId'])
            self.assertEqual(actual['source'].encode(), (self.ra / relative).read_bytes())
            self.assertNotEqual(physical_identity(self.rb / relative), physical_identity(self.ra / relative))
            self.assertNotEqual(physical_identity((self.rb / relative).parent), physical_identity((self.ra / relative).parent))
            history = self.call(self.b, '/api/history/list', sessionId=actual['sessionId'])
            births = [version for version in history['versions'] if version['operationId'] == plan['operationId']]
            self.assertEqual(len(births), 1)
            self.assertEqual(self.call(self.b, '/api/history/read', sessionId=actual['sessionId'],
                                       versionId=births[0]['versionId'])['source'], actual['source'])
        owner = self.open(self.b, self.rb, created[1][1])
        resources = self.resources(self.b, owner)
        self.assertEqual(set(resources), {'nested.bin'})
        resource = resources['nested.bin']
        self.assertEqual(resource['currentness'], 'accepted')
        self.assertEqual(self.call(self.b, '/api/resources/read', sessionId=owner['sessionId'],
                                   resource=resource['resource'], revision=resource['revision']), bytes([7, 0, 255]))
        current = tree_state(self.rb)
        self.restart_b()
        self.assertEqual(self.commit(self.b, plan), receipt)
        self.assertEqual(tree_state(self.rb), current)

    def test_sync_structure_new_ordinary_directory_parent_and_moves_keep_identity(self):
        self.changes()
        directory = self.ra / 'NewContainer'
        directory.mkdir()
        admitted = self.call(self.a, '/api/navigation/admit-directory', root=str(self.ra),
                             path=str(directory), operationId=uid())
        self.commit(self.a, admitted['preview'])
        marker = (directory / 'weftext.directory.json').read_bytes()
        directory_id = json.loads(marker)['directoryId']
        key = {'kind': 'directory', 'id': directory_id}
        moved = self.call(self.a, '/api/operations/prepare', root=str(self.ra), operationId=uid(),
                          intent={'kind': 'moveNode', 'source': str(self.ra / 'Container/Right'),
                                  'parent': str(directory)})
        self.commit(self.a, moved)
        self.after_paths['Right'] = Path('NewContainer/Right/Right.typ')
        objects = [self.keys[name] for name in ['Left', 'Right', 'Still']] + [key]
        received = self.transfer(self.a, self.ra, self.b, objects)
        self.received = {entry['object']['id']: entry['view'] for entry in received['objects'] if entry['object']['kind'] == 'node'}
        self.assert_received_sources()
        request = self.structure_request(objects)
        plan = self.prepare(request)
        self.assertFalse((self.rb / 'NewContainer').exists())
        self.assert_original_prepared_after_restart(request, plan)
        receipt = self.commit(self.b, plan)
        self.assert_installed()
        self.assert_rules_installed()
        self.assertEqual((self.rb / 'NewContainer/weftext.directory.json').read_bytes(), marker)
        self.assertFalse((self.rb / 'NewContainer/NewContainer.typ').exists())
        self.assertNotEqual(physical_identity(self.rb / 'NewContainer'), physical_identity(directory))
        target = self.call(self.b, '/api/navigation/resolve', root=str(self.rb),
                           targetRef=f'weftext-directory://{self.identity["workspaceId"]}/{directory_id}')
        self.assertEqual(target['status'], 'managed')
        self.assertEqual(Path(target['path']), self.rb / 'NewContainer')
        self.assertIn(key, self.call(self.b, '/api/sync/binding', root=str(self.rb))['installedObjects'])
        current = tree_state(self.rb)
        self.restart_b()
        self.assertEqual(self.commit(self.b, plan), receipt)
        self.assertEqual(tree_state(self.rb), current)

    def member_retirement_case(self, mixed):
        # Keep the original partial-clone fixture, then genuinely install its
        # complete predecessor before the operation under test. The receiver
        # starts with every removed Resource registered at its original FI.
        for name in ['Left', 'Right']:
            self.import_resource(self.a, self.ra, Path(name) / f'{name}.typ', 'keep.bin',
                                 f'ORIGINAL KEEP {name} 中文'.encode())
        self.import_resource(self.a, self.ra, Path('Left/Left.typ'), 'extra.bin', b'ORIGINAL EXTRA LEFT')
        binding = self.call(self.a, '/api/sync/binding', root=str(self.ra))
        self.transfer(self.a, self.ra, self.b, binding['installedObjects'])
        baseline = self.call(self.b, '/api/operations/prepare', **self.structure_request(binding['installedObjects']))
        self.commit(self.b, baseline)
        self.member_owners = {}
        self.member_originals = []
        status = self.call(self.b, '/api/sync/status', identity=self.identity)
        for name, removed in [('Left', ['baseline.png', 'extra.bin']), ('Right', ['baseline.png'])]:
            doc = self.open(self.b, self.rb, Path(name) / f'{name}.typ')
            resources = self.resources(self.b, doc)
            history = self.call(self.b, '/api/history/list', sessionId=doc['sessionId'])
            old = next(item['view']['versions'] for item in status['objects'] if item['object'] == self.keys[name])
            self.assertEqual(len(old), 1)
            self.member_owners[name] = {
                'removed': {leaf: {'resource': copy.deepcopy(resources[leaf]),
                                   'identity': physical_identity(self.rb / name / leaf),
                                   'bytes': (self.rb / name / leaf).read_bytes()} for leaf in removed},
                'keep': copy.deepcopy(resources['keep.bin']),
                'tree': tree_state(self.rb / name), 'predecessor': copy.deepcopy(old[0]),
                'history': {version['versionId']: self.call(self.b, '/api/history/read', sessionId=doc['sessionId'],
                            versionId=version['versionId'])['source'] for version in history['versions']
                            if version['status'] == 'available'},
            }
            for leaf in removed:
                original = self.ra / name / leaf
                retained = self.folder / f'original-A-{name}-{leaf}'
                identity = physical_identity(original)
                data = original.read_bytes()
                self.retain_original(original, retained)
                self.member_originals.append({'path': retained, 'identity': identity, 'bytes': data})
            manifest = self.ra / name / 'weftext.resources.json'
            identity = physical_identity(manifest)
            value = json.loads(manifest.read_bytes())
            value['resources'] = [entry for entry in value['resources'] if entry['path'] not in removed]
            self.assertEqual([entry['path'] for entry in value['resources']], ['keep.bin'])
            manifest.write_bytes((json.dumps(value, ensure_ascii=False, indent=2) + '\n').encode())
            self.assertEqual(physical_identity(manifest), identity)
        observed = tree_state(self.ra)
        cut = self.call(self.a, '/api/sync/adoption-cut', root=str(self.ra), identity=self.identity,
                        objects=binding['installedObjects'])
        self.assertEqual(len(cut['removedMembers']), 3)
        self.assertEqual(len(cut['changes']), 2)
        for member in cut['removedMembers']:
            self.assertEqual(set(member), {'path', 'baseId'})
            self.assertTrue(member['baseId'])
        adopt_request = {'operationId': uid(), 'root': str(self.ra), 'intent': {'kind': 'adoptSyncExternal', **cut}}
        adopted = self.call(self.a, '/api/operations/prepare', **adopt_request)
        self.assertEqual(tree_state(self.ra), observed)
        self.assertEqual(self.call(self.a, '/api/operations/prepare', **adopt_request), adopted)
        self.commit(self.a, adopted)
        self.assertEqual(tree_state(self.ra), observed, 'Sender adoption records exact observed absence without rewriting files')
        if mixed:
            renamed = self.call(self.a, '/api/operations/prepare', root=str(self.ra), operationId=uid(),
                                intent={'kind': 'renameNode', 'source': str(self.ra / 'Left'), 'name': 'MovedLeft'})
            self.commit(self.a, renamed)
            self.save(self.a, self.ra, Path('Right/Right.typ'), 'A content change shares Resource retirement 中文😀')
        self.member_paths = {'Left': Path('MovedLeft/MovedLeft.typ') if mixed else Path('Left/Left.typ'),
                             'Right': Path('Right/Right.typ')}
        self.member_sender_drafts = {}
        for name, relative in self.member_paths.items():
            doc = self.open(self.a, self.ra, relative)
            draft = doc['source'] + f'\nA ORIGINAL UNSAVED {name} RESOURCE DRAFT 🌿'
            self.call(self.a, '/api/draft', sessionId=doc['sessionId'], baseId=doc['baseId'], source=draft)
            self.member_sender_drafts[name] = draft
        self.member_sender_tree = tree_state(self.ra)
        self.member_before = tree_state(self.rb)
        received = self.transfer(self.a, self.ra, self.b, binding['installedObjects'])
        self.assertEqual(tree_state(self.rb), self.member_before)
        self.member_after = {}
        for name in self.member_paths:
            view = next(item['view'] for item in received['objects'] if item['object'] == self.keys[name])
            self.assertEqual(len(view['versions']), 1)
            state = view['versions'][0]['state']
            for leaf in self.member_owners[name]['removed']:
                self.assertNotIn(leaf, state['files'])
                self.assertNotIn(leaf, state['fileRevisions'])
            self.member_after[name] = copy.deepcopy(view['versions'][0])
        request = self.structure_request(binding['installedObjects'])
        plan = self.call(self.b, '/api/operations/prepare', **request)
        self.assertEqual(plan['status'], 'prepared')
        self.assertEqual(plan['kind'], 'applySync')
        self.assertEqual(tree_state(self.rb), self.member_before)
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **request), plan)
        self.member_targets = [Path(change['path']) for change in plan['changes'] if change['action'] == 'retainOriginal']
        retired = [Path(change['path']) for change in plan['changes'] if change['action'] == 'retireMember']
        self.assertEqual(len(self.member_targets), 3)
        self.assertEqual(len(set(self.member_targets)), 3)
        self.assertEqual(set(retired), {(self.rb / self.member_paths[name]).parent / leaf
                                       for name, owner in self.member_owners.items() for leaf in owner['removed']})
        for target in self.member_targets:
            self.assertTrue(target.resolve().is_relative_to(self.rb.resolve()))
            self.assertFalse(target.exists())
        (self.folder / 'resource-retirement-plan.json').write_text(json.dumps({
            'senderAdoption': adopt_request, 'request': request, 'plan': plan,
            'targets': [str(target) for target in self.member_targets], 'before': self.member_before,
        }, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
        return request, plan

    def assert_member_retirement_installed(self):
        self.assertEqual(tree_state(self.ra), self.member_sender_tree)
        targets = {physical_identity(target): target for target in self.member_targets}
        self.assertEqual(len(targets), 3)
        graph = self.call(self.b, '/api/sync/status', identity=self.identity)
        binding = self.call(self.b, '/api/sync/binding', root=str(self.rb))
        for original in self.member_originals:
            self.assertEqual(physical_identity(original['path']), original['identity'])
            self.assertEqual(original['path'].read_bytes(), original['bytes'])
        for name, relative in self.member_paths.items():
            owner = self.member_owners[name]
            directory = (self.rb / relative).parent
            heads = next(item['view']['versions'] for item in graph['objects'] if item['object'] == self.keys[name])
            self.assertEqual(heads, [self.member_after[name]])
            self.assertIn([self.keys[name], self.member_after[name]['revision']], binding['installedVersions'])
            source = self.open(self.b, self.rb, relative)
            self.assertEqual(source['nodeId'], self.keys[name]['id'])
            self.assertEqual(source['source'].encode(), (self.ra / relative).read_bytes())
            self.assertEqual(source['recoveredDraft'], self.drafts[name])
            self.assertEqual(physical_identity(directory), owner['tree']['.'][1])
            self.assertEqual(physical_identity(self.rb / relative), owner['tree'][f'{name}.typ'][1])
            for leaf in ['weftext.resources.json', 'weftext.annotations.json', 'personal.raw', 'keep.bin']:
                self.assertEqual(physical_identity(directory / leaf), owner['tree'][leaf][1])
            self.assertEqual(hashlib.sha256((directory / 'weftext.annotations.json').read_bytes()).hexdigest(),
                             owner['tree']['weftext.annotations.json'][3])
            self.assertEqual((directory / 'personal.raw').read_bytes(), f'B RAW {name} 中文😀'.encode())
            for version, text in owner['history'].items():
                self.assertEqual(self.call(self.b, '/api/history/read', sessionId=source['sessionId'],
                                           versionId=version)['source'], text)
            resources = self.resources(self.b, source)
            self.assertEqual(set(resources), {'keep.bin'})
            for field in ['resourceId', 'resource', 'revision']:
                self.assertEqual(resources['keep.bin'][field], owner['keep'][field])
            self.assertEqual(resources['keep.bin']['currentness'], 'accepted')
            self.assertEqual(self.call(self.b, '/api/resources/read', sessionId=source['sessionId'],
                                       resource=resources['keep.bin']['resource'], revision=resources['keep.bin']['revision']),
                             f'ORIGINAL KEEP {name} 中文'.encode())
            for leaf, asset in owner['removed'].items():
                self.assertFalse((directory / leaf).exists())
                target = targets[asset['identity']]
                self.assertEqual(target.read_bytes(), asset['bytes'])
                refused = self.call(self.b, '/api/resources/read', expected=409, sessionId=source['sessionId'],
                                    resource=asset['resource']['resource'], revision=asset['resource']['revision'])
                self.assertEqual(refused['error']['code'], 'resourceUnavailable',
                                 'Physical preservation cannot grant the retired ref a current owner role')
                old = self.call(self.b, '/api/sync/file', identity=self.identity, object=self.keys[name],
                                revision=owner['predecessor']['revision'], path=leaf)
                self.assertEqual(old['digest'], hashlib.sha256(asset['bytes']).hexdigest())
            sender = self.open(self.a, self.ra, relative)
            self.assertEqual(sender['recoveredDraft'], self.member_sender_drafts[name])
        current = tree_state(self.rb)
        for relative, original in self.member_before.items():
            if relative not in ['Left', 'Right'] and not relative.startswith(('Left/', 'Right/')):
                self.assertEqual(current[relative], original, f'Resource retirement preserves unrelated {relative}')
        self.assertEqual(self.call(self.b, '/api/knowledge/trash', root=str(self.rb))['trash'], [],
                         'Preserving retired Resource files must not invent retired Nodes')

    def member_retirement_commit_and_restart(self, mixed):
        request, plan = self.member_retirement_case(mixed)
        self.restart_b()
        self.assertEqual(tree_state(self.rb), self.member_before)
        self.assertEqual(self.call(self.b, '/api/sync/operation', operationId=plan['operationId']),
                         {'request': request, 'plan': plan})
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **request), plan)
        receipt = self.commit(self.b, plan)
        self.assert_member_retirement_installed()
        binding = self.call(self.b, '/api/sync/binding', root=str(self.rb))
        self.assertEqual(binding['pendingPublications'], 0)
        current = tree_state(self.rb)
        self.restart_b()
        self.assert_member_retirement_installed()
        original = self.call(self.b, '/api/sync/operation', operationId=plan['operationId'])
        self.assertEqual(original['request'], request)
        self.assertEqual(original['plan']['planId'], plan['planId'])
        self.assertEqual(original['plan']['status'], 'committed')
        self.assertEqual(self.commit(self.b, plan), receipt)
        self.assertEqual(tree_state(self.rb), current)

    def test_sync_structure_member_retirement_multi_owner(self):
        self.member_retirement_commit_and_restart(False)

    def test_sync_structure_member_retirement_move_content(self):
        self.member_retirement_commit_and_restart(True)

    def test_sync_structure_new_node_parent_two_relocations_content_and_private_facts(self):
        self.new_parent_changes()
        plan = self.prepare()
        self.assertEqual(plan['nodeMap'][self.new_parent_key['id']], self.new_parent_key['id'])
        self.assertFalse((self.rb / 'NewParent').exists())
        self.assertFalse((self.rb / '.weftext-rules').exists())
        receipt = self.commit(self.b, plan)
        self.assert_new_parent_installed()
        installed = tree_state(self.rb)
        self.assertEqual(self.commit(self.b, plan), receipt)
        self.assertEqual(tree_state(self.rb), installed, 'Replay must preserve the new parent objects too')

    def test_sync_structure_new_node_parent_restart_keeps_original_plan_and_receipt(self):
        self.new_parent_changes()
        request = self.request()
        plan = self.prepare(request)
        self.assertEqual(plan['nodeMap'][self.new_parent_key['id']], self.new_parent_key['id'])
        self.assert_original_prepared_after_restart(request, plan)
        self.commit(self.b, plan)
        installed = tree_state(self.rb)
        self.restart_b()
        self.assertEqual(tree_state(self.rb), installed)
        status = self.call(self.b, '/api/operations/status', operationId=plan['operationId'])
        self.assertEqual(status['status'], 'committed')
        self.assertEqual(status['planId'], plan['planId'])
        original = self.call(self.b, '/api/sync/operation', operationId=plan['operationId'])
        self.assertEqual(original['request'], request)
        self.assertEqual(original['plan']['planId'], plan['planId'])
        self.assertEqual(self.commit(self.b, plan), status)
        self.assertEqual(tree_state(self.rb), installed)
        self.assert_new_parent_installed()

    def test_sync_structure_new_node_parent_occupied_after_prepare_rejects_every_effect(self):
        self.new_parent_changes()
        plan = self.prepare()
        self.assertEqual(plan['nodeMap'][self.new_parent_key['id']], self.new_parent_key['id'])
        parent = self.rb / 'NewParent'
        self.assertFalse(parent.exists())
        parent.mkdir()
        original = b'EXTERNAL NEW PARENT OWNER; preserve this actual object'
        slot = parent / 'external.raw'
        slot.write_bytes(original)
        identity = physical_identity(parent)
        self.assert_rejected_without_effect(plan, tree_state(self.rb))
        self.assertEqual(physical_identity(parent), identity)
        self.assertEqual(slot.read_bytes(), original)
        self.assertFalse((parent / 'Right').exists())
        self.assertFalse((self.rb / '.weftext-rules').exists(), 'Rules must not be the first partial effect')

    def test_sync_structure_new_parent_with_native_nested_resource_and_two_relocations(self):
        # The Native import and its move happened before sender enable. This
        # proves a new-to-B Node with existing nested resources, not a producer
        # that adds new resource directories to an already installed Node.
        sender = self.open(self.a, self.ra, Path('NewParent/NewParent.typ'))
        self.assertEqual(sender['nodeId'], self.nested_parent_doc['nodeId'])
        self.new_parent_changes(existing_sender_parent=sender)
        sender = self.open(self.a, self.ra, Path('NewParent/NewParent.typ'))
        expected_resource = self.resources(self.a, sender)['nested.png']
        self.assertEqual(expected_resource['currentness'], 'accepted')
        self.assertIsNotNone(expected_resource['revision'])
        plan = self.prepare()
        self.assertEqual(plan['nodeMap'][self.new_parent_key['id']], self.new_parent_key['id'])
        self.assertFalse((self.rb / 'NewParent').exists())
        receipt = self.commit(self.b, plan)
        self.assert_new_parent_installed()
        opened = self.open(self.b, self.rb, Path('NewParent/NewParent.typ'))
        resource = self.resources(self.b, opened)['nested.png']
        for field in ['resourceId', 'resource', 'nodeId', 'revision', 'metadata']:
            self.assertEqual(resource[field], expected_resource[field])
        self.assertEqual(resource['currentness'], 'accepted')
        self.assertTrue(resource['annotationReady'])
        self.assertEqual(resource['metadata'], {'kind': 'image', 'width': 2, 'height': 2})
        self.assertEqual(self.call(self.b, '/api/resources/read', sessionId=opened['sessionId'],
                                   resource=resource['resource'], revision=resource['revision']), self.nested_image)
        relative = Path('NewParent/assets/icons/nested.png')
        self.assertEqual((self.rb / relative).read_bytes(), self.nested_image)
        manifest = json.loads((self.rb / 'NewParent/weftext.resources.json').read_bytes())
        self.assertEqual(manifest['resources'], [{'resourceId': resource['resourceId'],
                                                  'path': 'assets/icons/nested.png'}])
        for path in [Path('NewParent/assets'), Path('NewParent/assets/icons'), relative]:
            self.assertNotEqual(physical_identity(self.rb / path), physical_identity(self.ra / path))
        for relative in ['NativeUpload', 'NativeUpload/weftext.directory.json']:
            self.assertEqual(physical_identity(self.rb / relative), self.before[relative][1])
        self.assertEqual((self.ra / 'NativeUpload/.weftext-import-originals/original.weftext').read_bytes(),
                         self.native_original_payload)
        # Receiver resources and their containing directories are new physical
        # objects once; replay must not replace them with same-byte objects.
        installed = tree_state(self.rb)
        self.assertEqual(self.commit(self.b, plan), receipt)
        self.assertEqual(tree_state(self.rb), installed)


if __name__ == '__main__':
    available = {name for name in SyncStructure.__dict__ if name.startswith('test_sync_structure_')}
    selected = set(sys.argv[2:])
    unknown = selected - available
    if unknown:
        sys.exit('Unknown test selection: ' + ', '.join(sorted(unknown)))
    if len(sys.argv) < 2 or not Path(sys.argv[1]).is_file():
        sys.exit('Usage: test_sync_structure.py <candidate.exe> [exact_test_name ...]')
    suite = unittest.TestSuite(SyncStructure(name) for name in sorted(selected or available))
    sys.exit(not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful())
