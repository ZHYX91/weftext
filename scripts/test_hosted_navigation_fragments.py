"""Real authorized compiled-label navigation, using owned HTTP fixtures."""
import uuid
import json
import sys
import time
import unittest
from concurrent.futures import ThreadPoolExecutor
from owned_worker_observer import OwnedWorkerObservation, event_time_ns, timed_call
from urllib.parse import quote

from test_hosted import digest, uid
from test_hosted_history import HostedHistory


class HostedNavigationFragments(HostedHistory):
    def labelled(self, body='= Target heading <target>\n\nTarget body.', resource='Shared'):
        doc = self.open(resource)
        source = doc['source'].split('\n', 1)[0] + '\n\n' + body + '\n'
        self.save(doc, source)
        doc = self.open(resource)
        reference = self.call('navigation/ref', {'sessionId': doc['sessionId']})['reference']
        return doc, reference

    def resolve_fragment(self, reference, actor='alice', status=200):
        result = self.call('navigation/resolve', {'reference': reference}, actor, status)
        self.no_physical_paths(result)
        return result

    def fragment_request(self, doc, reference, source=None, serial=1, actor='alice'):
        source = doc['source'] if source is None else source
        request = dict(sessionId=doc['sessionId'], source=source, sourceHash=digest(source),
                       serial=serial, reference=reference)
        self.call('editor/map', {key: request[key] for key in ('sessionId', 'source', 'sourceHash', 'serial')}, actor)
        return request

    def locate(self, request, actor='alice', status=200):
        result = self.call('navigation/fragment', request, actor, status)
        self.no_physical_paths(result)
        return result

    def exact_origin(self, fragment, source, label):
        self.assertEqual(fragment['status'], 'found')
        self.assertEqual(fragment['label'], label)
        self.assertEqual(fragment['sourceHash'], digest(source))
        origin = fragment['origin']
        self.assertTrue(origin['isMain'])
        text = source.encode('utf-16-le')[origin['utf16From'] * 2:origin['utf16To'] * 2].decode('utf-16-le')
        self.assertEqual(text, origin['raw'])
        self.assertTrue(text)
        self.assertEqual(source.encode()[origin['from']:origin['to']].decode(), text)

    def test_fragment_strict_identity_encoding_and_actual_utf16_range(self):
        doc, reference = self.labelled('😀 before\n\n= 中文标题 <章节一>\n\n= Hash label <hash-name>')
        for suffix in ['章节一', quote('章节一')]:
            result = self.resolve_fragment(reference + '#' + suffix)
            self.assertEqual(result['resource'], 'Shared')
            self.assertTrue(result['canOpen'])
            self.exact_origin(result['fragment'], doc['source'], '章节一')
        self.assertIsNone(self.resolve_fragment(reference).get('fragment'))
        self.exact_origin(self.resolve_fragment(reference + '#hash-name')['fragment'], doc['source'], 'hash-name')
        for suffix in ['', '%', '%GG', '%FF', '%00', 'one#two', '%0A', '%C0%AF']:
            self.resolve_fragment(reference + '#' + suffix, status=400)
        self.resolve_fragment(reference + '?label=章节一', status=400)

    def test_fragment_compiled_presence_and_ambiguity_do_not_fall_back(self):
        doc, reference = self.labelled('= First <duplicate>\n\n= Second <duplicate>\n\n#if false [= Hidden <not-executed>]')
        for label, expected in [('duplicate', 'ambiguous'), ('not-executed', 'missing'), ('never-here', 'missing')]:
            result = self.resolve_fragment(reference + '#' + label)
            self.assertEqual(result['fragment']['status'], expected)
            self.assertFalse(result['canOpen'])
            self.assertIsNone(result['fragment'].get('origin'))
        source = doc['source'] + '\n#unknown_function()'
        self.save(doc, source)
        result = self.resolve_fragment(reference + '#duplicate')
        self.assertEqual(result['fragment']['status'], 'unavailable')
        self.assertFalse(result['canOpen'])

    def test_fragment_hidden_unknown_and_readonly_have_independent_authority(self):
        doc, reference = self.labelled()
        result = self.resolve_fragment(reference + '#target', 'reader')
        self.exact_origin(result['fragment'], doc['source'], 'target')
        hidden_doc, hidden = self.labelled('= Secret <secret-label>', 'Secret')
        missing = f'weftext-node://{uid()}/{uid()}#secret-label'
        denied = self.resolve_fragment(hidden + '#secret-label', 'bob', 404)
        self.assertEqual(denied, self.resolve_fragment(hidden + '#not-here', 'bob', 404))
        self.assertEqual(denied, self.resolve_fragment(missing, 'bob', 404))
        other = self.open('Other', 'bob')
        request = self.fragment_request(other, hidden + '#secret-label', actor='bob')
        self.locate(request, 'bob', 404)
        self.assertNotIn('Secret', json.dumps(denied))
        self.assertNotIn(hidden_doc['source'], json.dumps(denied))

    def test_fragment_current_draft_not_saved_range_and_late_serial_is_rejected(self):
        doc, reference = self.labelled()
        changed = doc['source'].replace('Target heading', 'Much later 😀 heading').replace('<target>', '<personal>')
        self.call('draft', {'inputId': str(uuid.uuid4()), 'baseId': doc['baseId'], 'inputRevision': 1, 'sessionId': doc['sessionId'], 'source': changed})
        request = self.fragment_request(doc, reference + '#personal', changed, serial=11)
        self.exact_origin(self.locate(request)['fragment'], changed, 'personal')
        self.assertEqual(self.resolve_fragment(reference + '#personal')['fragment']['status'], 'missing')
        old_target = self.fragment_request(doc, reference + '#target', changed, serial=12)
        self.assertEqual(self.locate(old_target)['fragment']['status'], 'missing')
        newer = changed + '\nNEWER_UNSAVED'
        self.fragment_request(doc, reference + '#personal', newer, serial=13)
        self.locate(request, status=409)
        self.assertEqual((self.root/'Shared/Shared.typ').read_text('utf-8'), doc['source'])
        self.assertEqual(self.open()['recoveredDraft'], changed)

    def test_fragment_follows_original_move_and_restart_but_not_replacement(self):
        doc, reference = self.labelled()
        def move(api):
            preview = api('/api/operations/prepare', {'root': str(self.root), 'operationId': uid(),
                'intent': {'kind': 'renameNode', 'source': str(self.root/'Shared'), 'name': 'Moved'}})
            result = api('/api/operations/commit', {'operationId': preview['operationId'], 'planId': preview['planId']})
            self.assertEqual(result['status'], 'committed')
            self.config['files']['Shared']['path'] = 'Moved/Moved.typ'
        self.local(move)
        current = self.open()
        self.exact_origin(self.resolve_fragment(reference + '#target')['fragment'], current['source'], 'target')
        self.server.close(); self.start()
        self.assertTrue(self.resolve_fragment(reference + '#target')['canOpen'])
        source = self.root/'Moved/Moved.typ'
        held = source.with_name('OwnedOriginal.typ')
        original = source.read_bytes(); source.rename(held); source.write_bytes(original)
        self.resolve_fragment(reference + '#target', status=404)
        self.assertEqual(held.read_bytes(), original)

    def test_fragment_inflight_revocation_never_delivers_old_origin(self):
        doc, reference = self.labelled()
        source = doc['source'] + '\n#for i in range(20000) [delayed paragraph #parbreak()]'
        request = self.fragment_request(doc, reference + '#target', source, serial=21)
        deadline = time.monotonic() + 40
        with ThreadPoolExecutor(max_workers=1) as pool:
            with OwnedWorkerObservation(self.server.owner) as observation:
                self.assertLess(time.monotonic(), deadline, 'Original request/mutation budget exhausted')
                submitted = event_time_ns()
                pending = pool.submit(timed_call, self.server.call, '/api/hosted/navigation/fragment', request, self.tokens['alice'])
                worker = observation.wait(pending, deadline=deadline)
                observation.assert_inflight(pending)
                self.assertLess(time.monotonic(), deadline, 'Original request/mutation budget exhausted before change')
                self.policy(lambda users: users['alice']['grants']['Shared'].update(read=False, write=False, history=False, download=False))
                changed = event_time_ns()
                (status, response, _), completed = pending.result(timeout=max(0, deadline-time.monotonic()))
        trace = {'submittedMonotonicNs': submitted, 'worker': worker,
                 'changeCompletedMonotonicNs': changed, 'requestCompletedMonotonicNs': completed,
                 'status': status, 'errorCode': response.get('error', {}).get('code'),
                 'explanationKey': response.get('error', {}).get('explanation', {}).get('key')}
        with (self.folder/'fragment-revocation-inflight.json').open('x', encoding='utf-8') as evidence:
            evidence.write(json.dumps(trace, ensure_ascii=False, indent=2))
        self.assertLessEqual(submitted, worker['observedMonotonicNs'])
        self.assertLess(worker['observedMonotonicNs'], changed)
        self.assertLess(changed, completed, 'Concurrent change must finish before request delivery')
        self.assertEqual(status, 401, response)
        self.assertEqual(response['error']['code'], 'authentication')
        self.assertEqual(response['error']['explanation']['key'],
                         'error.api.hosted.authenticationExpired')
        self.assertNotIn('fragment', response)
        self.assertNotIn('Target heading', json.dumps(response))


if __name__ == '__main__':
    selected = set(sys.argv[2:])
    suite = unittest.TestSuite(HostedNavigationFragments(name) for name in sorted(HostedNavigationFragments.__dict__)
                              if name.startswith('test_fragment_') and (not selected or name in selected))
    sys.exit(not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful())
