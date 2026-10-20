"""Test-only Windows observation of an owned backend's exact executable child.

This observes the existing compiler boundary without changing worker execution.
Other platforms must report only the dependent race subcase as unverified.
"""
import os
from pathlib import Path
import time


SUPPORTED = os.name == 'nt'

if SUPPORTED:
    import ctypes as c
    from ctypes import wintypes as w

    class Entry(c.Structure):
        _fields_ = [('size', w.DWORD), ('usage', w.DWORD), ('pid', w.DWORD),
                    ('heap', c.c_size_t), ('module', w.DWORD), ('threads', w.DWORD),
                    ('parent', w.DWORD), ('priority', w.LONG), ('flags', w.DWORD),
                    ('name', w.WCHAR * 260)]

    kernel = c.WinDLL('kernel32', use_last_error=True)
    kernel.CreateToolhelp32Snapshot.argtypes = [w.DWORD, w.DWORD]
    kernel.CreateToolhelp32Snapshot.restype = w.HANDLE
    kernel.Process32FirstW.argtypes = [w.HANDLE, c.POINTER(Entry)]
    kernel.Process32NextW.argtypes = [w.HANDLE, c.POINTER(Entry)]
    kernel.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
    kernel.OpenProcess.restype = w.HANDLE
    kernel.CloseHandle.argtypes = [w.HANDLE]
    kernel.GetProcessTimes.argtypes = [w.HANDLE, c.POINTER(w.FILETIME),
                                     c.POINTER(w.FILETIME), c.POINTER(w.FILETIME),
                                     c.POINTER(w.FILETIME)]
    kernel.QueryFullProcessImageNameW.argtypes = [w.HANDLE, w.DWORD, w.LPWSTR,
                                               c.POINTER(w.DWORD)]
    kernel.GetSystemTimePreciseAsFileTime.argtypes = [c.POINTER(w.FILETIME)]

    def ticks(value):
        return (value.dwHighDateTime << 32) | value.dwLowDateTime

    def identity(pid):
        handle = kernel.OpenProcess(0x1000, False, pid)
        if not handle:
            return None
        try:
            image = c.create_unicode_buffer(32768)
            size = w.DWORD(len(image))
            if not kernel.QueryFullProcessImageNameW(handle, 0, image, c.byref(size)):
                return None
            created, exited, cpu_kernel, cpu_user = (w.FILETIME() for _ in range(4))
            if not kernel.GetProcessTimes(handle, c.byref(created), c.byref(exited),
                                          c.byref(cpu_kernel), c.byref(cpu_user)):
                return None
            if ticks(exited):
                return None
            return {'pid': pid, 'image': str(Path(image.value).resolve()),
                    'creationFileTime': ticks(created)}
        finally:
            kernel.CloseHandle(handle)

    def children(parent):
        snapshot = kernel.CreateToolhelp32Snapshot(2, 0)
        if snapshot == c.c_void_p(-1).value:
            raise c.WinError(c.get_last_error())
        try:
            entry = Entry()
            entry.size = c.sizeof(entry)
            more = kernel.Process32FirstW(snapshot, c.byref(entry))
            result = []
            while more:
                if entry.parent == parent:
                    result.append(entry.pid)
                more = kernel.Process32NextW(snapshot, c.byref(entry))
            return result
        finally:
            kernel.CloseHandle(snapshot)


def creation_boundary():
    """Capture before request submission so an older child cannot satisfy it."""
    if not SUPPORTED:
        raise RuntimeError('Owned compiler observation is only verified on Windows')
    value = w.FILETIME()
    kernel.GetSystemTimePreciseAsFileTime(c.byref(value))
    return ticks(value)


def wait_for_owned_worker(process, pending, not_before, timeout=5):
    """Return only after freeze has spawned this request's compiler child.

    A direct same-executable child is the existing backend worker boundary.
    PID, image and creation time are checked; no process is suspended or killed.
    """
    if not SUPPORTED:
        raise RuntimeError('Owned compiler observation is only verified on Windows')
    expected_image = os.path.normcase(str(Path(process.args[0]).resolve()))
    root = identity(process.pid)
    assert root and os.path.normcase(root['image']) == expected_image, 'Owned backend identity changed'
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        assert not pending.done(), 'Render completed before an owned compiler worker was observed'
        assert process.poll() is None and identity(process.pid) == root, 'Owned backend ended or changed'
        for pid in children(process.pid):
            worker = identity(pid)
            if (worker and os.path.normcase(worker['image']) == expected_image
                    and worker['creationFileTime'] >= not_before):
                assert not pending.done(), 'Render completed before the concurrent action'
                return {**worker, 'parentPid': process.pid,
                        'observedMonotonicNs': time.monotonic_ns()}
        # Polling interval is not an assumption about how long compilation takes.
        time.sleep(.002)
    raise AssertionError('No owned compiler worker observed before the bounded deadline')


def timed_call(function, *args):
    result = function(*args)
    return result, time.monotonic_ns()
