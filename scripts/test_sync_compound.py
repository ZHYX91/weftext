"""Real two-program HTTP checks for installed, disjoint Node relocations.

Only the first compound slice is exercised: two installed Nodes, an installed
target parent, content/resources/annotations, and an unmoved content update.
No production failpoint or private-record rewrite is used. Restart checks the
original prepared/committed operation; interrupted partial installs belong to
the Core's controlled tests. Fixtures and HTTP transcripts are retained.

Run: python scripts/test_sync_compound.py <candidate.exe> [exact_test_name ...]
Unknown selections fail before either program starts. Every positive prepare
must return 200; an older candidate's syncStructureRequired is never a pass.
"""
import base64
import ctypes
from ctypes import wintypes
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import sys
import tempfile
import unittest
import zlib

from test_hosted import Server, uid


def evidence_directory(prefix):
    """Allow each run to retain evidence at an explicit fresh workspace path."""
    selected = os.environ.get('WEFTEXT_SYNC_EVIDENCE_DIR')
    if not selected:
        return Path(tempfile.mkdtemp(prefix=prefix))
    directory = Path(selected).resolve()
    directory.mkdir(parents=True, exist_ok=False)
    return directory


def png(rgb):
    """A valid two-by-two RGB input, requiring no external imaging tool."""
    def chunk(kind, data):
        return (struct.pack('>I', len(data)) + kind + data
                + struct.pack('>I', zlib.crc32(kind + data) & 0xffffffff))

    rows = (b'\x00' + bytes(rgb) * 2) * 2
    return (b'\x89PNG\r\n\x1a\n'
            + chunk(b'IHDR', struct.pack('>IIBBBBB', 2, 2, 8, 2, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b''))


def physical_identity(path):
    """Windows volume/index/creation-time identity, including directories."""
    class Information(ctypes.Structure):
        _fields_ = [('attributes', wintypes.DWORD),
                    ('creation', wintypes.FILETIME), ('access', wintypes.FILETIME),
                    ('write', wintypes.FILETIME), ('volume', wintypes.DWORD),
                    ('size_high', wintypes.DWORD), ('size_low', wintypes.DWORD),
                    ('links', wintypes.DWORD), ('index_high', wintypes.DWORD),
                    ('index_low', wintypes.DWORD)]

    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    kernel.CreateFileW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD,
                                  ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD,
                                  wintypes.HANDLE]
    kernel.CreateFileW.restype = wintypes.HANDLE
    kernel.GetFileInformationByHandle.argtypes = [wintypes.HANDLE,
                                                   ctypes.POINTER(Information)]
    kernel.GetFileInformationByHandle.restype = wintypes.BOOL
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    kernel.CloseHandle.restype = wintypes.BOOL
    # Read attributes, share read/write/delete, open existing, open directory
    # and reparse-point itself. The owned fixtures contain no reparse points.
    handle = kernel.CreateFileW(str(path), 0x80, 7, None, 3, 0x02200000, None)
    if handle == wintypes.HANDLE(-1).value:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        value = Information()
        if not kernel.GetFileInformationByHandle(handle, ctypes.byref(value)):
            raise ctypes.WinError(ctypes.get_last_error())
        return (value.volume, (value.index_high << 32) | value.index_low,
                (value.creation.dwHighDateTime << 32) | value.creation.dwLowDateTime)
    finally:
        kernel.CloseHandle(handle)


def tree_state(root):
    """Exact author-tree content and object identities, excluding access time."""
    result = {'.': ('directory', physical_identity(root))}
    for path in sorted(root.rglob('*')):
        relative = path.relative_to(root).as_posix()
        if path.is_dir():
            result[relative] = ('directory', physical_identity(path))
        else:
            data = path.read_bytes()
            result[relative] = ('file', physical_identity(path), len(data),
                                hashlib.sha256(data).hexdigest())
    return result


class SyncCompound(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if os.name != 'nt':
            raise RuntimeError('This harness requires the Windows candidate and physical identity API')
        cls.evidence = evidence_directory('weftext-sync-compound-')
        print('Retained compound sync evidence:', cls.evidence, flush=True)

    def initial_objects(self):
        """The first slice installs the original complete published baseline."""
        objects = self.binding['installedObjects']
        self.assertTrue(any(key['kind'] == 'rules' for key in objects))
        return objects

    def prepare_source_before_sync(self):
        """Optional real source producers; the original fixture has none."""
        return None

    def setUp(self):
        self.folder = self.evidence / self._testMethodName
        self.folder.mkdir()
        self.transfer_number = 0
        self.servers = []
        self.a = self.start_server(['--no-browser', '--data-dir', str(self.folder / 'state-a')], 'A')
        self.b = self.start_server(['--no-browser', '--data-dir', str(self.folder / 'state-b')], 'B')
        self.ra, self.rb = self.folder / 'library-a', self.folder / 'library-b'
        self.call(self.a, '/api/library/create', path=str(self.ra), operationId=uid())
        self.docs = {}
        for name in ['Left', 'Right', 'Container', 'Still']:
            doc = self.call(self.a, '/api/note/create', root=str(self.ra),
                            parent=str(self.ra), name=name, operationId=uid())
            self.docs[name] = doc
            self.save(self.a, self.ra, Path(name) / f'{name}.typ', f'初始 {name} 中文😀')
        self.keys = {name: {'kind': 'node', 'id': doc['nodeId']}
                     for name, doc in self.docs.items()}
        self.baseline_png = png((255, 0, 0))
        for name in ['Left', 'Right']:
            self.import_resource(self.a, self.ra, Path(name) / f'{name}.typ',
                                 'baseline.png', self.baseline_png)
            self.annotate(self.a, self.ra, Path(name) / f'{name}.typ',
                          f'初始 {name} 原讨论😀')
        self.prepare_source_before_sync()
        self.binding = self.call(self.a, '/api/sync/enable', root=str(self.ra), operationId=uid())
        self.identity = self.binding['identity']
        # The first slice starts with every participant already installed,
        # including Rules used by the later full external-adoption cut. A
        # Node-only clone plus new Rules hydration and two relocations is a
        # separate, intentionally unsupported combination in this phase.
        all_objects = self.initial_objects()
        self.assertTrue(all(key in all_objects for key in self.keys.values()))
        self.transfer(self.a, self.ra, self.b, all_objects)
        clone = self.call(self.b, '/api/operations/prepare', operationId=uid(), root=str(self.rb),
                          intent={'kind': 'cloneSync', 'identity': self.identity,
                                  'objects': all_objects})
        self.assertFalse(self.rb.exists(), 'Clone prepare must not create the author tree')
        self.commit(self.b, clone)

        # Establish a genuine B-side retained Save, then share that head with A
        # before A makes any branch. A's later changes have this common base.
        self.save(self.b, self.rb, Path('Left/Left.typ'), 'B 的准确保留历史😀')
        self.transfer(self.b, self.rb, self.a, [self.keys['Left']])
        baseline = self.call(self.a, '/api/operations/prepare', operationId=uid(), root=str(self.ra),
                             intent={'kind': 'applySync', 'identity': self.identity,
                                     'objects': [self.keys['Left']]})
        self.commit(self.a, baseline)
        # A's retained buffer still describes its old saved source. Compare
        # that buffer to its original base before explicitly accepting the
        # now-common source. The real resolver retains the reviewed originals;
        # this does not touch the separate B drafts established below.
        sender = self.open(self.a, self.ra, Path('Left/Left.typ'))
        if sender.get('recoveredDraftConflict'):
            self.assertEqual(sender['recoveredDraft'], sender['recoveredDraftBaseSource'],
                             'Only the sender fixture buffer with no unsaved edits is settled')
            self.assertTrue(sender['source'].startswith(sender['recoveredDraft']))
            self.assertIn('B 的准确保留历史😀', sender['source'])
            self.call(self.a, '/api/draft/resolve', sessionId=sender['sessionId'],
                      action='mergeRecovered', source=sender['source'])
            settled = self.open(self.a, self.ra, Path('Left/Left.typ'))
            self.assertEqual(settled['source'], sender['source'])
            self.assertFalse(settled['recoveredDraftConflict'])
        self.transfer(self.a, self.ra, self.b, all_objects)
        self.drafts, self.history = {}, {}
        for name in ['Left', 'Right']:
            doc = self.open(self.b, self.rb, Path(name) / f'{name}.typ')
            self.drafts[name] = doc['source'] + f'\nB PRIVATE DRAFT {name} 🌿'
            self.call(self.b, '/api/draft', sessionId=doc['sessionId'],
                      baseId=doc['baseId'], source=self.drafts[name])
            timeline = self.call(self.b, '/api/history/list', sessionId=doc['sessionId'])
            available = [version for version in timeline['versions'] if version['status'] == 'available']
            self.history[name] = {version['versionId']: self.call(
                self.b, '/api/history/read', sessionId=doc['sessionId'],
                versionId=version['versionId'])['source'] for version in available}
        self.assertTrue(self.history['Left'], 'A genuine B-side Save must remain readable')

        # Personal raw companions have no portable resource identity and must
        # stay the same local objects while their owning directory moves.
        for name in ['Left', 'Right', 'Container']:
            (self.rb / name / 'personal.raw').write_bytes(f'B RAW {name} 中文😀'.encode())
        self.before = tree_state(self.rb)
        self.initial_resources = {}
        for name in ['Left', 'Right']:
            doc = self.open(self.b, self.rb, Path(name) / f'{name}.typ')
            self.initial_resources[name] = self.resources(self.b, doc)['baseline.png']
        self.after_paths = {'Left': Path('Renamed/Renamed.typ'),
                            'Right': Path('Container/Right/Right.typ'),
                            'Still': Path('Still/Still.typ')}
        self.expected = {}
        self.received = None

    def start_server(self, args, label):
        server = Server(args)
        server.evidence_name = label
        server.evidence_index = len(self.servers)
        server.evidence_closed = False
        self.servers.append(server)
        self.addCleanup(self.close_server, server)
        return server

    def close_server(self, server):
        if server.evidence_closed:
            return
        if server.process.poll() is None:
            server.process.terminate()
        stdout, stderr = server.process.communicate(timeout=15)
        server.evidence_closed = True
        stem = f'server-{server.evidence_index:02}-{server.evidence_name}'
        (self.folder / f'{stem}.stdout.log').write_text(stdout, encoding='utf-8')
        (self.folder / f'{stem}.stderr.log').write_text(stderr, encoding='utf-8')
        with (self.folder / 'processes.ndjson').open('a', encoding='utf-8') as log:
            log.write(json.dumps({'program': server.evidence_name, 'pid': server.process.pid,
                                  'closed': server.process.poll() is not None,
                                  'stderrBytes': len(stderr.encode())}) + '\n')
        self.assertIsNotNone(server.process.poll(), 'Owned HTTP server must be closed')
        self.assertEqual(stderr, '', f'{server.evidence_name} server stderr')

    def call(self, server, route, expected=200, **data):
        code, result, _ = server.call(route, data)
        record = {'program': server.evidence_name, 'route': route,
                  'request': data, 'status': code,
                  'response': result if not isinstance(result, bytes)
                  else {'base64': base64.b64encode(result).decode()}}
        with (self.folder / 'http.ndjson').open('a', encoding='utf-8') as log:
            log.write(json.dumps(record, ensure_ascii=False) + '\n')
        self.assertEqual(code, expected, f'{route}: {result}')
        if expected == 409 and isinstance(result, dict):
            self.assertNotEqual(result.get('error', {}).get('code'), 'syncStructureRequired',
                                'An unsupported old compound planner is not this negative case')
        return result

    def commit(self, server, plan, expected=200, context=None):
        request = {'operationId': plan['operationId'], 'planId': plan['planId']}
        if context is not None:
            request['context'] = context
        receipt = self.call(server, '/api/operations/commit', expected=expected, **request)
        if expected == 200:
            self.assertEqual(receipt['status'], 'committed')
            self.assertEqual(receipt['operationId'], plan['operationId'])
            self.assertEqual(receipt['planId'], plan['planId'])
        return receipt

    def transfer(self, sender, root, receiver, objects):
        self.transfer_number += 1
        exchange = self.folder / f'exchange-{self.transfer_number:03}'
        self.call(sender, '/api/sync/export', root=str(root), directory=str(exchange), objects=objects)
        return self.call(receiver, '/api/sync/receive', identity=self.identity, directory=str(exchange))

    def open(self, server, root, relative):
        return self.call(server, '/api/open', root=str(root), path=str(root / relative))

    def save(self, server, root, relative, suffix):
        doc = self.open(server, root, relative)
        source = doc['source'] + '\n' + suffix + '\n'
        self.call(server, '/api/save', sessionId=doc['sessionId'], baseId=doc['baseId'],
                  source=source, operationId=uid())
        return self.open(server, root, relative)

    def resources(self, server, doc):
        value = self.call(server, '/api/resources/list', sessionId=doc['sessionId'])
        self.assertEqual(value['warnings'], [])
        return {resource['name']: resource for resource in value['resources']}

    def import_resource(self, server, root, relative, name, data):
        doc = self.open(server, root, relative)
        prepared = self.call(server, '/api/resources/prepare-import', sessionId=doc['sessionId'],
                             name=name, bytes=base64.b64encode(data).decode(), operationId=uid())
        self.commit(server, prepared)

    def annotate(self, server, root, relative, body):
        doc = self.open(server, root, relative)
        context = {'sessionId': doc['sessionId'], 'baseId': doc['baseId'],
                   'source': doc['source'], 'serial': 1, 'dirty': False}
        prepared = self.call(server, '/api/knowledge/prepare', context=context, operationId=uid(),
                             intent={'kind': 'annotationCreate', 'purpose': 'comment',
                                     'body': body, 'target': {'kind': 'wholeDocument'}})
        self.commit(server, prepared, context=context)

    def adopt_resource(self, relative, data):
        """Publish an in-place external edit through the original exact cut."""
        # Generic UpdateFiles correctly does not replace an admitted Resource.
        # Finish the real earlier producers, review the full installed cut,
        # and adopt only the actual same-object external bytes, without editing
        # the returned proposal or inventing a replacement-resource API.
        self.call(self.a, '/api/sync/publish', root=str(self.ra))
        binding = self.call(self.a, '/api/sync/binding', root=str(self.ra))
        status = self.call(self.a, '/api/sync/status', identity=self.identity)
        objects = [entry['object'] for entry in status['objects']
                   if entry['object'] in binding['installedObjects']]
        self.assertEqual({json.dumps(key, sort_keys=True) for key in objects},
                         {json.dumps(key, sort_keys=True) for key in binding['installedObjects']})
        self.assertTrue(all(self.keys[name] in objects for name in self.keys))
        path = (self.ra / relative).with_name('baseline.png')
        self.assertEqual(path.read_bytes(), self.baseline_png)
        identity = physical_identity(path)
        path.write_bytes(data)
        self.assertEqual(physical_identity(path), identity)
        current = tree_state(self.ra)
        cut = self.call(self.a, '/api/sync/adoption-cut', root=str(self.ra),
                        identity=self.identity, objects=objects)
        self.assertEqual(cut['identity'], self.identity)
        self.assertEqual({json.dumps(item['object'], sort_keys=True) for item in cut['expectedInstalled']},
                         {json.dumps(key, sort_keys=True) for key in objects})
        self.assertEqual(len(cut['changes']), 1)
        change = cut['changes'][0]
        self.assertEqual(Path(change['path']), path)
        self.assertEqual(bytes(change['before']), data)
        self.assertEqual(change['before'], change['after'])
        self.assertTrue(change['baseId'])
        request = {'operationId': uid(), 'root': str(self.ra),
                   'intent': {'kind': 'adoptSyncExternal', **cut}}
        prepared = self.call(self.a, '/api/operations/prepare', **request)
        self.assertEqual(tree_state(self.ra), current, 'Adoption prepare must preserve observed bytes and identities')
        self.commit(self.a, prepared)
        self.assertEqual(tree_state(self.ra), current, 'Adoption must record the observed edit without rewriting it')

    def changes(self):
        """Generate every sender version through original public producers."""
        renamed = self.call(self.a, '/api/operations/prepare', operationId=uid(), root=str(self.ra),
                            intent={'kind': 'renameNode', 'source': str(self.ra / 'Left'),
                                    'name': 'Renamed'})
        self.commit(self.a, renamed)
        moved = self.call(self.a, '/api/operations/prepare', operationId=uid(), root=str(self.ra),
                          intent={'kind': 'moveNode', 'source': str(self.ra / 'Right'),
                                  'parent': str(self.ra / 'Container')})
        self.commit(self.a, moved)
        for number, name in enumerate(['Left', 'Right']):
            relative = self.after_paths[name]
            self.save(self.a, self.ra, relative, f'A {name} 同次 Apply 正文更新😀')
            changed_png = png((0, 255, 50 + number))
            self.adopt_resource(relative, changed_png)
            self.import_resource(self.a, self.ra, relative, 'added.png', png((20, number, 255)))
            self.annotate(self.a, self.ra, relative, f'A {name} 随移动的新讨论😀')
        self.save(self.a, self.ra, self.after_paths['Still'], 'A 未移动对象同次内容更新😀')
        for name, relative in self.after_paths.items():
            directory = (self.ra / relative).parent
            names = [relative.name]
            if name != 'Still':
                names += ['baseline.png', 'added.png', 'weftext.resources.json', 'weftext.annotations.json']
            self.expected[name] = {filename: (directory / filename).read_bytes() for filename in names}
        objects = [self.keys[name] for name in ['Left', 'Right', 'Still']]
        received = self.transfer(self.a, self.ra, self.b, objects)
        # Receive keeps the complete dependency closure. Only this assertion
        # projects Node views; Schema/Rules/Settings keys need not have an id.
        self.received = {entry['object']['id']: entry['view'] for entry in received['objects']
                         if entry['object']['kind'] == 'node'}
        self.assertTrue({self.keys[name]['id'] for name in ['Left', 'Right', 'Still']}.issubset(self.received))
        self.assertEqual(tree_state(self.rb), self.before, 'Receiving must not install any author member')
        self.assert_received_sources()

    def request(self, objects=None):
        chosen = objects if objects is not None else [self.keys[name] for name in ['Left', 'Right', 'Still']]
        # The original request stores a BTreeSet. Use its Node-id order so
        # replay compares the durable request without relying on random UUID
        # creation order or treating an unordered selection as a new decision.
        chosen = sorted(chosen, key=lambda key: key['id'])
        return {'operationId': uid(), 'root': str(self.rb), 'intent': {
            'kind': 'applySync', 'identity': self.identity,
            'objects': chosen}}

    def prepare(self, request=None):
        request = request or self.request()
        before = tree_state(self.rb)
        plan = self.call(self.b, '/api/operations/prepare', **request)
        self.assertEqual(plan['operationId'], request['operationId'])
        self.assertEqual(plan['kind'], 'applySync')
        self.assertEqual(plan['status'], 'prepared')
        selected_ids = {self.keys[name]['id'] for name in ['Left', 'Right', 'Still']}
        self.assertTrue(selected_ids.issubset(plan['nodeMap']))
        for node_id in selected_ids:
            self.assertEqual(plan['nodeMap'][node_id], node_id)
        self.assertEqual(tree_state(self.rb), before, 'Prepare must have no namespace or content effect')
        self.assertEqual(plan, self.call(self.b, '/api/operations/prepare', **request))
        self.assertEqual(tree_state(self.rb), before, 'Idempotent prepare must have no effect')
        original = self.call(self.b, '/api/sync/operation', operationId=plan['operationId'])
        self.assertEqual(original['request'], request)
        self.assertEqual(original['plan'], plan)
        return plan

    def restart_b(self):
        self.close_server(self.b)
        self.b = self.start_server(['--no-browser', '--data-dir', str(self.folder / 'state-b')], 'B-restart')

    def retain_original(self, source, retained):
        """Move only the current test's originals to an unused owned path."""
        fixture = self.folder.resolve(strict=True)
        resolved_source = source.resolve(strict=True)
        resolved_retained = retained.resolve(strict=False)
        self.assertTrue(resolved_source.is_relative_to(fixture), 'Source must remain inside this owned fixture')
        self.assertTrue(resolved_retained.is_relative_to(fixture), 'Retained target must remain inside this owned fixture')
        self.assertNotEqual(resolved_source, fixture)
        self.assertNotEqual(resolved_retained, fixture)
        self.assertFalse(retained.exists() or retained.is_symlink(), 'The retained target must be unused')
        resolved_source.rename(resolved_retained)

    def assert_received_sources(self):
        for name in ['Left', 'Right', 'Still']:
            view = self.received[self.keys[name]['id']]
            self.assertEqual(len(view['versions']), 1, f'{name}: unsettled fixture heads {view}')
            version = view['versions'][0]
            relative = self.after_paths[name]
            file = self.call(self.b, '/api/sync/file', identity=self.identity, object=self.keys[name],
                             revision=version['revision'], path=relative.name)
            self.assertEqual(file['text'].encode(), self.expected[name][relative.name])

    def assert_installed(self):
        for name, relative in self.after_paths.items():
            directory = (self.rb / relative).parent
            for filename, data in self.expected[name].items():
                self.assertEqual((directory / filename).read_bytes(), data, f'{name}/{filename}')
            doc = self.open(self.b, self.rb, relative)
            self.assertEqual(doc['nodeId'], self.keys[name]['id'])
            if name == 'Still':
                self.assertEqual(physical_identity(self.rb / relative), self.before['Still/Still.typ'][1])
                continue
            self.assertFalse((self.rb / name).exists())
            self.assertEqual(physical_identity(directory), self.before[name][1])
            self.assertEqual(physical_identity(self.rb / relative), self.before[f'{name}/{name}.typ'][1])
            for filename in ['baseline.png', 'weftext.resources.json', 'weftext.annotations.json', 'personal.raw']:
                self.assertEqual(physical_identity(directory / filename), self.before[f'{name}/{filename}'][1])
            self.assertEqual((directory / 'personal.raw').read_bytes(), f'B RAW {name} 中文😀'.encode())
            self.assertEqual(doc['recoveredDraft'], self.drafts[name])
            self.assertTrue(doc['recoveredDraftConflict'])
            timeline = self.call(self.b, '/api/history/list', sessionId=doc['sessionId'])
            available = {version['versionId'] for version in timeline['versions'] if version['status'] == 'available'}
            self.assertTrue(set(self.history[name]).issubset(available))
            for version_id, source in self.history[name].items():
                retained = self.call(self.b, '/api/history/read', sessionId=doc['sessionId'], versionId=version_id)
                self.assertEqual(retained['source'], source)
            actual_resources = self.resources(self.b, doc)
            self.assertEqual(set(actual_resources), {'baseline.png', 'added.png'})
            self.assertEqual(actual_resources['baseline.png']['resourceId'], self.initial_resources[name]['resourceId'])
            self.assertNotEqual(actual_resources['baseline.png']['revision'], self.initial_resources[name]['revision'])
            for resource in actual_resources.values():
                self.assertEqual(resource['currentness'], 'accepted')
                payload = self.call(self.b, '/api/resources/read', sessionId=doc['sessionId'],
                                    resource=resource['resource'], revision=resource['revision'])
                self.assertEqual(payload, self.expected[name][resource['name']])
            annotations = json.loads((directory / 'weftext.annotations.json').read_text(encoding='utf-8'))
            self.assertEqual({record['body'] for record in annotations['annotations']},
                             {f'初始 {name} 原讨论😀', f'A {name} 随移动的新讨论😀'})
        for relative in ['.', 'Container', 'Container/Container.typ', 'Container/personal.raw']:
            path = self.rb if relative == '.' else self.rb / relative
            self.assertEqual(physical_identity(path), self.before[relative][1])
            if not path.is_dir():
                data = path.read_bytes()
                self.assertEqual(hashlib.sha256(data).hexdigest(), self.before[relative][3])

    def assert_rejected_without_effect(self, plan, current):
        self.commit(self.b, plan, expected=409)
        self.assertEqual(tree_state(self.rb), current, 'Rejection must preserve all current objects and bytes')
        self.assertTrue((self.rb / 'Left/Left.typ').is_file())
        self.assertFalse((self.rb / 'Renamed').exists(), 'The first relocation must not have happened')
        self.assertEqual(physical_identity(self.rb / 'Left'), self.before['Left'][1])
        original = self.call(self.b, '/api/sync/operation', operationId=plan['operationId'])
        self.assertEqual(original['plan'], plan, 'A rejection must retain the same original plan')
        inspected = self.call(self.b, '/api/recovery/inspect', operationId=plan['operationId'])
        self.assertIn(inspected['coherence'], ('thirdState', 'unavailable'))
        self.assertEqual(inspected['allowedActions'], [])
        for action in ['adoptAfterImages', 'resumeOriginalPlan', 'keepCurrent']:
            self.call(self.b, '/api/recovery/resolve', expected=409,
                      operationId=plan['operationId'], action=action)
        self.assertEqual(tree_state(self.rb), current, 'Recovery must not overwrite a third state')
        self.assert_received_sources()

    def test_sync_compound_two_moves_content_resources_annotations_and_private_facts(self):
        self.changes()
        plan = self.prepare()
        inspected = self.call(self.b, '/api/recovery/inspect', operationId=plan['operationId'])
        self.assertEqual(inspected['status'], 'prepared')
        self.assertEqual(inspected['coherence'], 'knownBefore')
        self.assertEqual(inspected['allowedActions'], ['keepCurrent'])
        self.assertEqual(tree_state(self.rb), self.before)
        receipt = self.commit(self.b, plan)
        self.assertEqual(self.commit(self.b, plan), receipt)
        self.assert_installed()

    def test_sync_compound_restart_preserves_original_id_plan_and_committed_receipt(self):
        self.changes()
        request = self.request()
        plan = self.prepare(request)
        self.restart_b()
        self.assertEqual(tree_state(self.rb), self.before)
        recovered = self.call(self.b, '/api/sync/operation', operationId=plan['operationId'])
        self.assertEqual(recovered['request'], request)
        self.assertEqual(recovered['plan'], plan)
        self.assertEqual(self.call(self.b, '/api/operations/status', operationId=plan['operationId'])['status'], 'prepared')
        self.assertEqual(self.call(self.b, '/api/operations/prepare', **request), plan)
        # Ignore this response. A new process must reconstruct the same durable
        # committed receipt, with no new prepare or replacement operation ID.
        self.commit(self.b, plan)
        self.restart_b()
        status = self.call(self.b, '/api/operations/status', operationId=plan['operationId'])
        self.assertEqual(status['status'], 'committed')
        self.assertEqual(status['planId'], plan['planId'])
        recovered = self.call(self.b, '/api/sync/operation', operationId=plan['operationId'])
        self.assertEqual(recovered['request'], request)
        self.assertEqual(recovered['plan']['planId'], plan['planId'])
        self.assertEqual(self.commit(self.b, plan), status)
        self.assert_installed()

    def test_sync_compound_second_target_occupied_before_prepare_rejects_all(self):
        self.changes()
        target = self.rb / 'Container/Right'
        target.mkdir()
        (target / 'external.raw').write_bytes(b'OWNED EXTERNAL TARGET; preserve this')
        current = tree_state(self.rb)
        self.call(self.b, '/api/operations/prepare', expected=409, **self.request())
        self.assertEqual(tree_state(self.rb), current)
        self.assertFalse((self.rb / 'Renamed').exists())
        self.assertEqual(physical_identity(self.rb / 'Left'), self.before['Left'][1])
        self.assert_received_sources()

    def test_sync_compound_second_target_occupied_after_prepare_rejects_before_first_effect(self):
        self.changes()
        plan = self.prepare()
        target = self.rb / 'Container/Right'
        target.mkdir()
        (target / 'external.raw').write_bytes(b'LATE TARGET OWNER; preserve this')
        self.assert_rejected_without_effect(plan, tree_state(self.rb))

    def test_sync_compound_late_second_source_preimage_rejects_before_first_effect(self):
        self.changes()
        plan = self.prepare()
        source = self.rb / 'Right/Right.typ'
        source.write_bytes(source.read_bytes() + b'\nEXTERNAL LATE SECOND SOURCE\n')
        self.assert_rejected_without_effect(plan, tree_state(self.rb))

    def test_sync_compound_same_bytes_second_source_replacement_rejects_before_first_effect(self):
        self.changes()
        plan = self.prepare()
        source = self.rb / 'Right/Right.typ'
        original = source.read_bytes()
        retained = self.folder / 'retained-original-right.typ'
        original_identity = physical_identity(source)
        self.retain_original(source, retained)
        source.write_bytes(original)
        self.assertEqual(source.read_bytes(), original)
        self.assertNotEqual(physical_identity(source), original_identity)
        self.assert_rejected_without_effect(plan, tree_state(self.rb))
        self.assertEqual(retained.read_bytes(), original)
        self.assertEqual(physical_identity(retained), original_identity)

    def test_sync_compound_same_bytes_target_parent_replacement_rejects_before_first_effect(self):
        self.changes()
        plan = self.prepare()
        parent = self.rb / 'Container'
        retained = self.folder / 'retained-original-container'
        original_identity = physical_identity(parent)
        self.retain_original(parent, retained)
        shutil.copytree(retained, parent)
        self.assertNotEqual(physical_identity(parent), original_identity)
        self.assertEqual({p.relative_to(parent).as_posix(): p.read_bytes() for p in parent.rglob('*') if p.is_file()},
                         {p.relative_to(retained).as_posix(): p.read_bytes() for p in retained.rglob('*') if p.is_file()})
        self.assert_rejected_without_effect(plan, tree_state(self.rb))
        self.assertEqual(physical_identity(retained), original_identity)


if __name__ == '__main__':
    available = {name for name in SyncCompound.__dict__ if name.startswith('test_sync_compound_')}
    selected = set(sys.argv[2:])
    unknown = selected - available
    if unknown:
        sys.exit('Unknown test selection: ' + ', '.join(sorted(unknown)))
    if len(sys.argv) < 2 or not Path(sys.argv[1]).is_file():
        sys.exit('Usage: test_sync_compound.py <candidate.exe> [exact_test_name ...]')
    suite = unittest.TestSuite(SyncCompound(name) for name in sorted(selected or available))
    sys.exit(not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful())
