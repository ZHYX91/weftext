"""Pure fake-native tests; no real process, worker, host or desktop is started."""
from pathlib import Path
import copy
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

import owned_worker_observer as observer


WINDOWS_API = observer._WindowsAPI
CREATED = (1 << 63) + 37
IMAGE = str(Path(__file__).resolve().parent / 'fake-weftext.exe')


def _fake_require(condition, message='Fake native invariant failed'):
    if not condition:
        raise AssertionError(message)


class Pending:
    completed = False

    def done(self):
        return self.completed


class FakeNative:
    def __init__(self):
        self.parent = {'pid': 11, 'creationFileTime': CREATED, 'image': IMAGE}
        self.identities = {22: {'pid': 22, 'creationFileTime': CREATED + 200, 'image': IMAGE}}
        self.baseline, self.candidates, self.after_hold = [], [22], None
        self.snapshot_calls = 0
        self.opened, self.closed, self.handles = [], [], {}
        self.close_attempts, self.close_failure_pids = [], set()
        self.inactive = set()
        self.boundary_action = self.after_hold_action = None
        self.fail_identity = set()

    def identity(self, handle):
        _fake_require(handle not in self.closed, 'Query after observation handle closed')
        if handle == 1:
            return dict(self.parent)
        pid = self.handles[handle]
        if pid in self.fail_identity:
            raise OSError('fake query refused')
        return dict(self.identities[pid])

    def active(self, handle):
        _fake_require(handle not in self.closed, 'Fake native invariant failed')
        return (11 if handle == 1 else self.handles[handle]) not in self.inactive

    def open_child(self, pid):
        _fake_require(pid != 11, 'The numeric owner PID must never be reopened')
        handle = 100 + len(self.opened)
        self.opened.append((pid, handle))
        self.handles[handle] = pid
        return handle

    def close(self, handle):
        _fake_require(handle != 1, 'The borrowed Popen owner handle must never be closed')
        _fake_require(handle not in self.closed, 'Observation handle closed twice')
        self.close_attempts.append(handle)
        if self.handles[handle] in self.close_failure_pids:
            raise OSError('fake handle close failed')
        self.closed.append(handle)

    def children(self, parent):
        _fake_require(parent == 11, 'Fake native invariant failed')
        self.snapshot_calls += 1
        if self.snapshot_calls == 1:
            return list(self.baseline)
        if self.snapshot_calls >= 3:
            if self.after_hold_action:
                self.after_hold_action()
            if self.after_hold is not None:
                return list(self.after_hold)
        return list(self.candidates)

    def creation_boundary(self):
        if self.boundary_action:
            self.boundary_action()
        return CREATED + 100


class OwnedObservationTests(unittest.TestCase):
    def setUp(self):
        self.native, self.pending = FakeNative(), Pending()
        self.process = SimpleNamespace(pid=11, _handle=1, poll=lambda: None)
        ready = {'pid': 11, 'created': CREATED, 'incarnation': 'opaque-incarnation',
                 'profileIdentity': 'opaque-profile', 'role': 'hosted'}
        self.owner = SimpleNamespace(process=self.process, command=[IMAGE, 'serve'],
                                     ready={'ownerIdentity': ready}, created=CREATED, hosted=True)
        native_patch = patch.object(observer, '_WindowsAPI', return_value=self.native)
        support_patch = patch.object(observer, 'SUPPORTED', True)
        native_patch.start()
        support_patch.start()
        self.addCleanup(native_patch.stop)
        self.addCleanup(support_patch.stop)

    def observation(self):
        return observer.OwnedWorkerObservation(self.owner)

    def assert_all_observation_handles_closed(self):
        self.assertCountEqual(self.native.closed, [handle for _, handle in self.native.opened])
        self.assertNotIn(1, self.native.closed)

    def test_original_held_owner_and_live_child_are_retained_until_context_exit(self):
        with self.observation() as observation:
            worker = observation.wait(self.pending)
            held = self.native.opened[-1][1]
            self.assertNotIn(held, self.native.closed)
            self.assertEqual(worker['parentCreationFileTime'], CREATED)
            self.assertEqual(worker['creationFileTime'], CREATED + 200)
            self.assertTrue(worker['heldChildParentRechecked'])
            self.assertEqual(self.native.snapshot_calls, 3)
            observation.assert_inflight(self.pending)
        self.assert_all_observation_handles_closed()

    def test_ready_creation_mismatch_cannot_reopen_numeric_owner(self):
        self.owner.ready['ownerIdentity']['created'] += 1
        with self.assertRaisesRegex(AssertionError, 'Ready creation'):
            with self.observation():
                self.fail('Unqualified owner was accepted')
        self.assertEqual(self.native.opened, [])

    def test_held_handle_pid_creation_image_and_ready_role_are_all_required(self):
        for field, value in [('pid', 99), ('creationFileTime', CREATED + 1), ('image', IMAGE + '.other')]:
            with self.subTest(field=field):
                original = self.native.parent[field]
                self.native.parent[field] = value
                with self.assertRaises(AssertionError):
                    with self.observation():
                        self.fail('Wrong held owner accepted')
                self.native.parent[field] = original
        self.owner.ready['ownerIdentity']['role'] = 'local'
        with self.assertRaisesRegex(AssertionError, 'role'):
            with self.observation():
                self.fail('Wrong Ready role accepted')
        self.assertEqual(self.native.opened, [])

    def test_baseline_identity_is_rejected_even_if_its_time_would_pass(self):
        self.native.baseline = [22]
        with patch.object(observer.time, 'sleep', side_effect=lambda _: setattr(self.pending, 'completed', True)):
            with self.assertRaisesRegex(AssertionError, 'Render completed'):
                with self.observation() as observation:
                    observation.wait(self.pending)
        self.assertEqual(len(self.native.opened), 2)
        self.assert_all_observation_handles_closed()

    def test_same_pid_with_new_creation_is_a_new_identity_not_the_baseline(self):
        self.native.baseline = [22]
        self.native.identities[22]['creationFileTime'] = CREATED + 50
        self.native.boundary_action = lambda: self.native.identities[22].update(creationFileTime=CREATED + 200)
        with self.observation() as observation:
            worker = observation.wait(self.pending)
            self.assertEqual(worker['baselineChildIdentities'], [[22, CREATED + 50]])
            self.assertEqual(worker['creationFileTime'], CREATED + 200)
        self.assert_all_observation_handles_closed()

    def test_nonbaseline_child_before_filetime_boundary_is_rejected(self):
        self.native.identities[22]['creationFileTime'] = CREATED + 99
        with patch.object(observer.time, 'sleep', side_effect=lambda _: setattr(self.pending, 'completed', True)):
            with self.assertRaisesRegex(AssertionError, 'Render completed'):
                with self.observation() as observation:
                    observation.wait(self.pending)
        self.assert_all_observation_handles_closed()

    def test_different_child_image_cannot_satisfy_the_compiler_boundary(self):
        self.native.identities[22]['image'] = IMAGE + '.other'
        with patch.object(observer.time, 'sleep', side_effect=lambda _: setattr(self.pending, 'completed', True)):
            with self.assertRaisesRegex(AssertionError, 'Render completed'):
                with self.observation() as observation:
                    observation.wait(self.pending)
        self.assert_all_observation_handles_closed()

    def test_multiple_new_live_matching_children_are_ambiguous_and_all_closed(self):
        self.native.identities[33] = {'pid': 33, 'creationFileTime': CREATED + 300, 'image': IMAGE}
        self.native.candidates = [22, 33]
        with self.assertRaisesRegex(AssertionError, 'Multiple new'):
            with self.observation() as observation:
                observation.wait(self.pending)
        self.assert_all_observation_handles_closed()

    def test_parent_relationship_is_rechecked_after_child_handle_is_held(self):
        self.native.after_hold = []
        with self.assertRaisesRegex(AssertionError, 'parent relationship'):
            with self.observation() as observation:
                observation.wait(self.pending)
        self.assert_all_observation_handles_closed()

    def test_additional_child_appearing_during_parent_recheck_is_not_silently_ignored(self):
        self.native.identities[33] = {'pid': 33, 'creationFileTime': CREATED + 300, 'image': IMAGE}
        self.native.after_hold = [22, 33]
        with self.assertRaisesRegex(AssertionError, 'New child appeared'):
            with self.observation() as observation:
                observation.wait(self.pending)
        self.assert_all_observation_handles_closed()

    def test_close_failure_keeps_primary_rejection_and_attempts_all_other_handles(self):
        self.native.identities[33] = {'pid': 33, 'creationFileTime': CREATED + 300, 'image': IMAGE}
        self.native.candidates = [22, 33]
        self.native.close_failure_pids.add(22)
        with self.assertRaisesRegex(AssertionError, 'Multiple new') as caught:
            with self.observation() as observation:
                observation.wait(self.pending)
        self.assertCountEqual(self.native.close_attempts, [handle for _, handle in self.native.opened])
        self.assertTrue(any('close failed: OSError' in note for note in caught.exception.__notes__))
        self.assertEqual(self.native.closed, [self.native.opened[1][1]])

    def test_close_failure_without_primary_rejects_and_closes_the_selected_handle(self):
        self.native.identities[33] = {'pid': 33, 'creationFileTime': CREATED + 300, 'image': IMAGE + '.other'}
        self.native.candidates = [22, 33]
        self.native.close_failure_pids.add(33)
        with self.assertRaisesRegex(OSError, 'handle close failed'):
            with self.observation() as observation:
                observation.wait(self.pending)
        self.assertCountEqual(self.native.close_attempts, [handle for _, handle in self.native.opened])
        self.assertEqual(self.native.closed, [self.native.opened[0][1]])

    def test_context_close_failure_keeps_the_original_action_error_without_retry(self):
        original = RuntimeError('original action failed')
        self.native.close_failure_pids.add(22)
        with self.assertRaises(RuntimeError) as caught:
            with self.observation() as observation:
                observation.wait(self.pending)
                raise original
        self.assertIs(caught.exception, original)
        self.assertEqual(len(self.native.close_attempts), 1)
        self.assertTrue(any('close failed: OSError' in note for note in original.__notes__))

    def test_observation_cannot_be_reentered_after_context_exit(self):
        observation = self.observation()
        with observation:
            pass
        with self.assertRaisesRegex(AssertionError, 'cannot be entered twice'):
            with observation:
                self.fail('Reentered a used observation')

    def test_unqueryable_candidate_cannot_be_skipped_and_its_handle_is_closed(self):
        self.native.fail_identity.add(22)
        with self.assertRaisesRegex(OSError, 'query refused'):
            with self.observation() as observation:
                observation.wait(self.pending)
        self.assert_all_observation_handles_closed()

    def test_baseline_query_and_close_failure_preserve_the_original_query_object(self):
        self.native.baseline = [22]
        self.native.close_failure_pids.add(22)
        original = OSError('original baseline identity query failed')
        identity = self.native.identity
        def fail_child(handle):
            if handle != 1:
                raise original
            return identity(handle)
        self.native.identity = fail_child
        with self.assertRaises(OSError) as caught:
            with self.observation():
                self.fail('Unqualified baseline accepted')
        self.assertIs(caught.exception, original)
        self.assertEqual(len(self.native.close_attempts), 1)
        self.assertTrue(any('close failed: OSError' in note for note in original.__notes__))

    def test_wait_query_and_close_failure_preserve_the_primary_and_try_all_handles(self):
        self.native.identities[33] = {'pid': 33, 'creationFileTime': CREATED + 300, 'image': IMAGE}
        self.native.candidates = [22, 33]
        self.native.close_failure_pids.add(22)
        original = OSError('original candidate identity query failed')
        identity = self.native.identity
        def fail_second(handle):
            if handle != 1 and self.native.handles[handle] == 33:
                raise original
            return identity(handle)
        self.native.identity = fail_second
        with self.assertRaises(OSError) as caught:
            with self.observation() as observation:
                observation.wait(self.pending)
        self.assertIs(caught.exception, original)
        self.assertCountEqual(self.native.close_attempts, [handle for _, handle in self.native.opened])
        self.assertTrue(any('close failed: OSError' in note for note in original.__notes__))

    def test_snapshot_enumeration_and_close_failure_preserve_the_original_error_object(self):
        native = object.__new__(WINDOWS_API)
        original = OSError('original process enumeration failed')
        state = SimpleNamespace(error=0)
        def first(*_args):
            state.error = 5
            return False
        native.c = SimpleNamespace(c_void_p=lambda _: SimpleNamespace(value=-1),
                                   sizeof=lambda _: 1, byref=lambda value: value,
                                   set_last_error=lambda value: setattr(state, 'error', value),
                                   get_last_error=lambda: state.error, WinError=lambda _: original)
        native.Entry = lambda: SimpleNamespace(size=0)
        native.kernel = SimpleNamespace(CreateToolhelp32Snapshot=lambda *_: 99,
                                        Process32FirstW=first)
        native.close = Mock(side_effect=OSError('fake snapshot close failed'))
        with self.assertRaises(OSError) as caught:
            native.children(11)
        self.assertIs(caught.exception, original)
        native.close.assert_called_once_with(99)
        self.assertTrue(any('close failed: OSError' in note for note in original.__notes__))

    def test_original_owner_must_stay_active_during_post_hold_recheck(self):
        self.native.after_hold_action = lambda: self.native.inactive.add(11)
        with self.assertRaisesRegex(AssertionError, 'owner is no longer active'):
            with self.observation() as observation:
                observation.wait(self.pending)
        self.assert_all_observation_handles_closed()

    def test_request_completion_during_post_hold_recheck_prevents_mutation(self):
        self.native.after_hold_action = lambda: setattr(self.pending, 'completed', True)
        with self.assertRaisesRegex(AssertionError, 'Render completed'):
            with self.observation() as observation:
                observation.wait(self.pending)
        self.assert_all_observation_handles_closed()

    def test_child_exit_and_late_request_completion_are_rechecked_before_mutation(self):
        with self.observation() as observation:
            observation.wait(self.pending)
            self.native.inactive.add(22)
            with self.assertRaisesRegex(AssertionError, 'child exited'):
                observation.assert_inflight(self.pending)
            self.native.inactive.clear()
            self.pending.completed = True
            with self.assertRaisesRegex(AssertionError, 'Render completed'):
                observation.assert_inflight(self.pending)
        self.assert_all_observation_handles_closed()

    def test_action_failure_still_closes_only_the_observation_child(self):
        original = RuntimeError('original concurrent action failed')
        with self.assertRaises(RuntimeError) as caught:
            with self.observation() as observation:
                observation.wait(self.pending)
                raise original
        self.assertIs(caught.exception, original)
        self.assert_all_observation_handles_closed()

    def test_supplied_absolute_deadline_is_not_replaced_with_another_five_seconds(self):
        self.native.candidates = []
        clock = SimpleNamespace(value=0.0)
        def advance(seconds):
            clock.value += seconds
        with patch.object(observer.time, 'monotonic', side_effect=lambda: clock.value), \
                patch.object(observer.time, 'sleep', side_effect=advance):
            with self.assertRaisesRegex(AssertionError, 'bounded deadline'):
                with self.observation() as observation:
                    observation.wait(self.pending, deadline=.003)
        self.assertEqual(clock.value, .003)
        self.assert_all_observation_handles_closed()

    def test_observation_native_open_requests_only_query_and_synchronize(self):
        native = object.__new__(WINDOWS_API)
        native.kernel = SimpleNamespace(OpenProcess=Mock(return_value=17))
        self.assertEqual(native.open_child(22), 17)
        native.kernel.OpenProcess.assert_called_once_with(0x1000 | 0x100000, False, 22)

    def test_unsupported_platform_is_explicitly_unverified_without_a_fallback(self):
        with patch.object(observer, 'SUPPORTED', False):
            with self.assertRaisesRegex(RuntimeError, 'only verified on Windows'):
                observer.OwnedWorkerObservation(self.owner)


class PolicyRevocationTests(unittest.TestCase):
    def setUp(self):
        self.path = ('alice', 'enabled')
        self.original = {'generation': 4, 'users': {'alice': {'enabled': True}, 'other': {'enabled': True}}}
        self.request = observer.freeze_policy_revocation(self.original, self.path)
        self.readback = {'generation': 5, 'users': copy.deepcopy(self.request['users'])}
        self.trace = {'submittedMonotonicNs': 10, 'worker': {'observedMonotonicNs': 20},
                      'changeStartedMonotonicNs': 30, 'changeAcknowledgedMonotonicNs': 50,
                      'requestCompletedMonotonicNs': 40, 'policyGeneration': 5, 'status': 401}
        self.body = {'error': {'code': 'authentication', 'explanation': {'key': 'error.api.hosted.authenticationExpired'}}}
        self.denial = (401, 'authentication', 'error.api.hosted.authenticationExpired', 'html')

    def verify(self):
        observer.verify_revocation_delivery(self.request, self.path, self.readback, self.trace, self.body, self.denial)

    def test_acknowledgment_receipt_before_or_after_denial_is_not_publication(self):
        for acknowledged in (35, 50):
            with self.subTest(acknowledged=acknowledged):
                self.trace['changeAcknowledgedMonotonicNs'] = acknowledged
                self.verify()
        self.assertTrue(self.original['users']['alice']['enabled'])
        self.assertTrue(self.request['users']['other']['enabled'])

    def test_proofread_denial_and_exact_nested_authorization_are_supported(self):
        self.path = ('alice', 'grants', 'Shared', 'annotations', 'proofread')
        policy = {'generation': 4, 'users': {'alice': {'grants': {'Shared': {'annotations': {'proofread': True}}}}}}
        self.request = observer.freeze_policy_revocation(policy, self.path)
        self.readback['users'] = copy.deepcopy(self.request['users'])
        self.trace['status'] = 404
        self.body = {'error': {'code': 'unavailable', 'explanation': {'key': 'error.api.hosted.contentUnavailable'}}}
        self.denial = (404, 'unavailable', 'error.api.hosted.contentUnavailable', 'findings')
        self.verify()
        self.body['findings'] = []
        with self.assertRaisesRegex(AssertionError, 'without content'):
            self.verify()

    def test_completed_request_or_worker_after_mutation_cannot_claim_inflight(self):
        for key, value in [('requestCompletedMonotonicNs', 30), ('requestCompletedMonotonicNs', 29),
                           ('changeStartedMonotonicNs', 20), ('submittedMonotonicNs', 21),
                           ('changeAcknowledgedMonotonicNs', 29)]:
            with self.subTest(key=key, value=value):
                trace = copy.deepcopy(self.trace)
                self.trace[key] = value
                with self.assertRaises(AssertionError):
                    self.verify()
                self.trace = trace

    def test_wrong_generation_or_unrevoked_target_cannot_be_a_publication_witness(self):
        for readback in [{'generation': 6, 'users': self.readback['users']},
                         {'generation': True, 'users': self.readback['users']},
                         {'generation': 5, 'users': self.original['users']},
                         {'generation': 5, 'users': {}}]:
            with self.subTest(readback=readback):
                self.readback = readback
                with self.assertRaises(AssertionError):
                    self.verify()

    def test_success_wrong_error_or_content_is_never_a_revocation_denial(self):
        for body, status in [(self.body, 200), ({'error': {'code': 'other'}}, 401),
                             ({**self.body, 'html': 'old private output'}, 401)]:
            with self.subTest(status=status, body=body):
                self.body, self.trace['status'] = body, status
                with self.assertRaises(AssertionError):
                    self.verify()

    def test_only_a_true_original_grant_can_be_frozen(self):
        for value in (False, 1, None):
            with self.subTest(value=value):
                policy = copy.deepcopy(self.original)
                policy['users']['alice']['enabled'] = value
                with self.assertRaises(AssertionError):
                    observer.freeze_policy_revocation(policy, self.path)

    def test_post_is_immediately_preceded_by_inflight_check_and_requires_exact_ack(self):
        for acknowledgment in [{'status': 'applied', 'generation': 5},
                               {'status': 'conflict', 'generation': 5},
                               {'status': 'applied', 'generation': 6}]:
            with self.subTest(acknowledgment=acknowledgment):
                events = []
                observation = SimpleNamespace(assert_inflight=lambda pending: events.append('inflight'))
                def post(request):
                    self.assertEqual(request, self.request)
                    self.assertEqual(events, ['inflight'])
                    events.append('post')
                    return acknowledgment
                with patch.object(observer, 'event_time_ns', side_effect=[30, 50]), \
                        patch.object(observer.time, 'monotonic', side_effect=[1, 2]):
                    if acknowledgment == {'status': 'applied', 'generation': 5}:
                        result = observer.apply_inflight_revocation(observation, Pending(), self.request, post, 40)
                        self.assertEqual(result, {key: self.trace[key] for key in
                                                  ['changeStartedMonotonicNs', 'changeAcknowledgedMonotonicNs', 'policyGeneration']})
                    else:
                        with self.assertRaisesRegex(AssertionError, 'acknowledged'):
                            observer.apply_inflight_revocation(observation, Pending(), self.request, post, 40)

    def test_original_deadline_is_enforced_before_and_after_the_policy_post(self):
        for times in ([40], [1, 40]):
            with self.subTest(times=times):
                post = Mock(return_value={'status': 'applied', 'generation': 5})
                observation = SimpleNamespace(assert_inflight=Mock())
                with patch.object(observer.time, 'monotonic', side_effect=times), \
                        patch.object(observer, 'event_time_ns', side_effect=[30, 50]):
                    with self.assertRaisesRegex(AssertionError, 'budget exhausted'):
                        observer.apply_inflight_revocation(observation, Pending(), self.request, post, 40)
                self.assertEqual(post.call_count, int(len(times) == 2))


class EventClockTests(unittest.TestCase):
    def test_adjacent_events_remain_ordered_when_the_ordinary_tick_is_equal(self):
        with patch.object(observer.time, 'monotonic_ns', return_value=0), \
                patch.object(observer.time, 'perf_counter_ns', side_effect=[101, 102]):
            before = observer.event_time_ns()
            result, after = observer.timed_call(lambda: 'original result')
        self.assertEqual(result, 'original result')
        self.assertLess(before, after)

    def test_an_unqualified_event_counter_cannot_claim_event_order(self):
        with patch.object(observer.time, 'get_clock_info', return_value=SimpleNamespace(monotonic=False)):
            with self.assertRaisesRegex(AssertionError, 'not monotonic'):
                observer.event_time_ns()


if __name__ == '__main__':
    unittest.main(verbosity=2)
