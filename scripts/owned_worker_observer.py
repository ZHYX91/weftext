"""Test-only Windows observation of a new child of an exact held serve owner.

The caller isolates one request whose production path freezes inputs before
spawning its compiler. This observes that existing boundary, without changing
worker execution. Other platforms remain unverified for dependent race cases.
"""
import os
import copy
import json
from pathlib import Path
import sys
import time


SUPPORTED = os.name == 'nt'
_QUERY_SYNCHRONIZE = 0x1000 | 0x100000


def _require(condition, message):
    if not condition:
        raise AssertionError(message)


def _close_preserving(close, handles, primary):
    cleanup_error = None
    for handle in handles:
        try:
            close(handle)
        except Exception as error:
            if primary is not None:
                primary.add_note('Owned observation handle close failed: ' + type(error).__name__)
            elif cleanup_error is None:
                cleanup_error = error
            else:
                cleanup_error.add_note('Another observation handle close failed: ' + type(error).__name__)
    if primary is None and cleanup_error is not None:
        raise cleanup_error


class _WindowsAPI:
    def __init__(self):
        import ctypes as c
        from ctypes import wintypes as w

        class Entry(c.Structure):
            _fields_ = [('size', w.DWORD), ('usage', w.DWORD), ('pid', w.DWORD),
                        ('heap', c.c_size_t), ('module', w.DWORD), ('threads', w.DWORD),
                        ('parent', w.DWORD), ('priority', w.LONG), ('flags', w.DWORD),
                        ('name', w.WCHAR * 260)]

        self.c, self.w, self.Entry = c, w, Entry
        self.kernel = kernel = c.WinDLL('kernel32.dll', use_last_error=True, winmode=0x800)
        kernel.CreateToolhelp32Snapshot.argtypes = [w.DWORD, w.DWORD]
        kernel.CreateToolhelp32Snapshot.restype = w.HANDLE
        kernel.Process32FirstW.argtypes = [w.HANDLE, c.POINTER(Entry)]
        kernel.Process32FirstW.restype = w.BOOL
        kernel.Process32NextW.argtypes = [w.HANDLE, c.POINTER(Entry)]
        kernel.Process32NextW.restype = w.BOOL
        kernel.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
        kernel.OpenProcess.restype = w.HANDLE
        kernel.CloseHandle.argtypes = [w.HANDLE]
        kernel.CloseHandle.restype = w.BOOL
        kernel.GetProcessId.argtypes = [w.HANDLE]
        kernel.GetProcessId.restype = w.DWORD
        kernel.GetProcessTimes.argtypes = [w.HANDLE, *([c.POINTER(w.FILETIME)] * 4)]
        kernel.GetProcessTimes.restype = w.BOOL
        kernel.QueryFullProcessImageNameW.argtypes = [w.HANDLE, w.DWORD, w.LPWSTR,
                                                   c.POINTER(w.DWORD)]
        kernel.QueryFullProcessImageNameW.restype = w.BOOL
        kernel.WaitForSingleObject.argtypes = [w.HANDLE, w.DWORD]
        kernel.WaitForSingleObject.restype = w.DWORD
        kernel.GetSystemTimePreciseAsFileTime.argtypes = [c.POINTER(w.FILETIME)]
        kernel.GetSystemTimePreciseAsFileTime.restype = None

    @staticmethod
    def ticks(value):
        return (value.dwHighDateTime << 32) | value.dwLowDateTime

    def identity(self, handle):
        c, w, kernel = self.c, self.w, self.kernel
        pid = kernel.GetProcessId(handle)
        if not pid:
            raise c.WinError(c.get_last_error())
        image = c.create_unicode_buffer(32768)
        size = w.DWORD(len(image))
        if not kernel.QueryFullProcessImageNameW(handle, 0, image, c.byref(size)):
            raise c.WinError(c.get_last_error())
        times = [w.FILETIME() for _ in range(4)]
        if not kernel.GetProcessTimes(handle, *(c.byref(value) for value in times)):
            raise c.WinError(c.get_last_error())
        return {'pid': int(pid), 'image': str(Path(image.value).resolve()),
                'creationFileTime': self.ticks(times[0])}

    def active(self, handle):
        status = self.kernel.WaitForSingleObject(handle, 0)
        if status == 258:
            return True
        if status == 0:
            return False
        raise self.c.WinError(self.c.get_last_error())

    def open_child(self, pid):
        handle = self.kernel.OpenProcess(_QUERY_SYNCHRONIZE, False, pid)
        if not handle:
            raise self.c.WinError(self.c.get_last_error())
        return handle

    def close(self, handle):
        if not self.kernel.CloseHandle(handle):
            raise self.c.WinError(self.c.get_last_error())

    def children(self, parent):
        c, kernel = self.c, self.kernel
        snapshot = kernel.CreateToolhelp32Snapshot(2, 0)
        if not snapshot or snapshot == c.c_void_p(-1).value:
            raise c.WinError(c.get_last_error())
        try:
            entry = self.Entry()
            entry.size = c.sizeof(entry)
            result = []
            c.set_last_error(0)
            more = kernel.Process32FirstW(snapshot, c.byref(entry))
            while more:
                if entry.parent == parent:
                    result.append(int(entry.pid))
                c.set_last_error(0)
                more = kernel.Process32NextW(snapshot, c.byref(entry))
            if c.get_last_error() != 18:  # ERROR_NO_MORE_FILES, not an incomplete snapshot.
                raise c.WinError(c.get_last_error())
            return result
        finally:
            _close_preserving(self.close, [snapshot], sys.exception())

    def creation_boundary(self):
        value = self.w.FILETIME()
        self.kernel.GetSystemTimePreciseAsFileTime(self.c.byref(value))
        return self.ticks(value)


class OwnedWorkerObservation:
    """Borrow the original owner handle and retain only the selected child.

    Enter before submitting the caller's one isolated request. A successful
    wait retains a query/SYNCHRONIZE child handle until context exit. Neither
    owner nor child is stopped, killed, suspended or otherwise controlled.
    """
    def __init__(self, owner):
        if not SUPPORTED:
            raise RuntimeError('Owned compiler observation is only verified on Windows')
        self.owner, self.process = owner, owner.process
        self.native = _WindowsAPI()
        self.parent_handle = int(self.process._handle)
        self.expected_image = os.path.normcase(str(Path(owner.command[0]).resolve()))
        self.parent = self.ready_identity = None
        self.baseline = set()
        self.not_before = None
        self.worker = None
        self._worker_handle = None
        self._entered = False
        self._used = False

    def _assert_owner(self):
        _require(self.owner.process is self.process, 'Original owned serve process changed')
        _require(int(self.process._handle) == self.parent_handle, 'Original held owner handle changed')
        _require(self.owner.ready['ownerIdentity'] == self.ready_identity, 'Verified Ready identity changed')
        _require(self.owner.created == self.parent['creationFileTime'], 'Original owner creation changed')
        _require(self.process.poll() is None, 'Original owned backend ended')
        _require(self.native.identity(self.parent_handle) == self.parent, 'Original held owner identity changed')
        _require(self.native.active(self.parent_handle), 'Original held owner is no longer active')

    def __enter__(self):
        _require(not self._used, 'Owned worker observation cannot be entered twice')
        self._used = True
        ready = self.owner.ready['ownerIdentity']
        _require(type(ready['pid']) is int and ready['pid'] == self.process.pid, 'Ready PID differs from owned process')
        _require(type(ready['created']) is int and type(self.owner.created) is int, 'Missing exact owner creation')
        _require(ready['created'] == self.owner.created, 'Ready creation differs from original held owner')
        _require(ready['role'] == ('hosted' if self.owner.hosted else 'local'), 'Ready owner role differs')
        self.ready_identity = dict(ready)
        self.parent = self.native.identity(self.parent_handle)
        _require(self.parent['pid'] == ready['pid'], 'Original handle names another owner PID')
        _require(self.parent['creationFileTime'] == ready['created'], 'Original handle creation differs from Ready')
        _require(os.path.normcase(self.parent['image']) == self.expected_image, 'Original owner image differs')
        self._assert_owner()
        for pid in self.native.children(self.parent['pid']):
            handle = self.native.open_child(pid)
            try:
                identity = self.native.identity(handle)
                _require(identity['pid'] == pid, 'Baseline child handle names another PID')
                self.baseline.add((pid, identity['creationFileTime']))
            finally:
                _close_preserving(self.native.close, [handle], sys.exception())
        self.not_before = self.native.creation_boundary()
        self._assert_owner()
        self._entered = True
        return self

    def wait(self, pending, timeout=5, *, deadline=None):
        """Observe a live new direct same-image child within the original budget.

        The default five seconds is retained for the editor's existing case.
        A caller with one global budget supplies its absolute monotonic deadline.
        """
        _require(self._entered and self._worker_handle is None, 'Observation is not awaiting its first worker')
        if deadline is None:
            deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            _require(not pending.done(), 'Render completed before an owned compiler worker was observed')
            self._assert_owner()
            opened, candidates, keep = [], [], None
            try:
                pids = self.native.children(self.parent['pid'])
                _require(len(pids) == len(set(pids)), 'Owned child snapshot contains duplicate PIDs')
                for pid in pids:
                    handle = self.native.open_child(pid)
                    opened.append(handle)
                    worker = self.native.identity(handle)
                    _require(worker['pid'] == pid, 'Observed child handle names another PID')
                    if ((pid, worker['creationFileTime']) not in self.baseline
                            and worker['creationFileTime'] >= self.not_before
                            and os.path.normcase(worker['image']) == self.expected_image
                            and self.native.active(handle)):
                        candidates.append((handle, worker))
                _require(len(candidates) <= 1, 'Multiple new owned compiler children are ambiguous')
                if candidates:
                    handle, worker = candidates[0]
                    # Re-enumerate after holding the actual child, so a stale
                    # numeric PID snapshot cannot qualify a replacement object.
                    rechecked = self.native.children(self.parent['pid'])
                    _require(len(rechecked) == len(set(rechecked)), 'Held child snapshot contains duplicate PIDs')
                    _require(worker['pid'] in rechecked, 'Held child parent relationship changed')
                    _require(not set(rechecked).difference(pids), 'New child appeared during held-child parent recheck')
                    self._assert_owner()
                    _require(self.native.identity(handle) == worker, 'Held compiler child identity changed')
                    _require(self.native.active(handle), 'Compiler child exited before the concurrent action')
                    _require(not pending.done(), 'Render completed before the concurrent action')
                    _require(time.monotonic() < deadline, 'Owned worker observation budget exhausted')
                    self.worker = dict(worker)
                    self._worker_handle = keep = handle
                    return {**worker, 'parentPid': self.parent['pid'],
                            'parentCreationFileTime': self.parent['creationFileTime'],
                            'creationBoundaryFileTime': self.not_before,
                            'baselineChildIdentities': [list(value) for value in sorted(self.baseline)],
                            'heldChildParentRechecked': True,
                            'observedMonotonicNs': event_time_ns()}
            finally:
                _close_preserving(self.native.close, [handle for handle in opened if handle != keep],
                                  sys.exception())
            # Polling observes an event; no duration is a presumed compile phase.
            time.sleep(min(.002, max(0, deadline - time.monotonic())))
        raise AssertionError('No owned compiler worker observed before the bounded deadline')

    def assert_inflight(self, pending):
        _require(self._entered and self._worker_handle is not None, 'No held compiler child was observed')
        self._assert_owner()
        _require(self.native.identity(self._worker_handle) == self.worker, 'Held compiler child identity changed')
        _require(self.native.active(self._worker_handle), 'Compiler child exited before the concurrent action')
        _require(not pending.done(), 'Render completed before the concurrent action')

    def __exit__(self, exc_type, exc, tb):
        handle, self._worker_handle = self._worker_handle, None
        self._entered = False
        if handle is not None:
            _close_preserving(self.native.close, [handle], exc)
        return False


def event_time_ns():
    # A nanosecond unit does not imply a fine Windows monotonic tick. Keep
    # event order on the high-resolution counter, independently of deadlines.
    _require(time.get_clock_info('perf_counter').monotonic, 'Event counter is not monotonic')
    return time.perf_counter_ns()


def timed_call(function, *args):
    result = function(*args)
    return result, event_time_ns()


def _policy_value(users, path):
    value = users
    for key in path:
        _require(isinstance(value, dict) and key in value, 'Missing exact policy authorization target')
        value = value[key]
    return value


def freeze_policy_revocation(policy, path):
    _require(path and _policy_value(policy.get('users'), path) is True, 'Original grant must be true')
    return freeze_policy_changes(policy, [(path, False)])


def freeze_policy_changes(policy, changes):
    """Freeze exact boolean, list or structured policy targets before the worker.

    This is only a policy POST payload; it cannot identify a publication instant.
    """
    generation = policy.get('generation')
    _require(type(generation) is int and generation >= 0, 'Invalid original policy generation')
    _require(changes, 'Missing exact policy changes')
    users = copy.deepcopy(policy['users'])
    seen = set()
    for path, replacement in changes:
        _require(path and tuple(path) not in seen, 'Missing or duplicate exact policy target')
        seen.add(tuple(path))
        old = _policy_value(users, path)
        _require(type(old) is not type(replacement) or old != replacement, 'Policy target must actually change')
        parent = users
        for key in path[:-1]:
            parent = parent[key]
        parent[path[-1]] = copy.deepcopy(replacement)
    return {'expectedGeneration': generation, 'users': users}


def apply_inflight_mutation(observation, pending, post, deadline):
    """Call only the actual mutating POST here, never prepare/GET/readback."""
    _require(time.monotonic() < deadline, 'Original request/mutation budget exhausted before change')
    observation.assert_inflight(pending)
    started = event_time_ns()
    acknowledgment = post()
    acknowledged = event_time_ns()
    _require(time.monotonic() < deadline, 'Original request/mutation budget exhausted during change')
    return acknowledgment, {'changeStartedMonotonicNs': started, 'changeAcknowledgedMonotonicNs': acknowledged}


def apply_inflight_revocation(observation, pending, request, post, deadline):
    acknowledgment, interval = apply_inflight_mutation(observation, pending, lambda: post(request), deadline)
    _require(acknowledgment.get('status') == 'applied'
             and type(acknowledgment.get('generation')) is int
             and acknowledgment['generation'] == request['expectedGeneration'] + 1,
             'Exact policy update was not acknowledged as applied')
    # The server publishes under its shared policy/delivery mutex before either
    # response is sent. Client ACK receipt is an interval endpoint, not that
    # publication instant; an authorized denial may reach its client first.
    return {**interval, 'policyGeneration': acknowledgment['generation']}


def verify_inflight_change(trace):
    _require(trace['submittedMonotonicNs'] <= trace['worker']['observedMonotonicNs']
             < trace['changeStartedMonotonicNs'] < trace['requestCompletedMonotonicNs'],
             'Request must remain in flight when the mutating POST starts')
    _require(trace['changeStartedMonotonicNs'] <= trace['changeAcknowledgedMonotonicNs'],
             'Mutation acknowledgment precedes its request')


def verify_policy_changes(request, paths, readback, trace):
    _require(paths, 'Missing exact policy readback targets')
    _require(type(readback.get('generation')) is int
             and readback['generation'] == trace['policyGeneration'] == request['expectedGeneration'] + 1,
             'Policy readback differs from the exact acknowledged generation')
    for path in paths:
        expected = _policy_value(request['users'], path)
        actual = _policy_value(readback.get('users'), path)
        _require(json.dumps(actual, sort_keys=True) == json.dumps(expected, sort_keys=True),
                 'Exact policy target differs on readback')
    verify_inflight_change(trace)


def verify_revocation_delivery(request, path, readback, trace, body, denial):
    verify_policy_changes(request, [path], readback, trace)
    _require(_policy_value(request['users'], path) is False
             and _policy_value(readback.get('users'), path) is False,
             'Exact target authorization was not revoked')
    status, code, key, forbidden = denial
    error = body.get('error', {})
    _require(trace['status'] == status and error.get('code') == code
             and error.get('explanation', {}).get('key') == key and forbidden not in body,
             'Original request did not return the exact authorization denial without content')
