"""Minimal Windows ctypes helpers for namespace_probe.py; standard library only."""
import ctypes as C
from ctypes import wintypes as W
import json
from pathlib import Path
import subprocess
import sys

K = C.WinDLL('kernel32', use_last_error=True)
INVALID = C.c_void_p(-1).value
READ, WRITE, SHARE_ALL = 0x80000000, 0x40000000, 7

def api(name, args, result):
    f = getattr(K, name)
    f.argtypes, f.restype = args, result
    return f

CreateFile = api('CreateFileW', [W.LPCWSTR, W.DWORD, W.DWORD, C.c_void_p, W.DWORD, W.DWORD, W.HANDLE], W.HANDLE)
CloseHandle = api('CloseHandle', [W.HANDLE], W.BOOL)
ReadFile = api('ReadFile', [W.HANDLE, C.c_void_p, W.DWORD, C.POINTER(W.DWORD), C.c_void_p], W.BOOL)
WriteFile = api('WriteFile', [W.HANDLE, C.c_void_p, W.DWORD, C.POINTER(W.DWORD), C.c_void_p], W.BOOL)
Seek = api('SetFilePointerEx', [W.HANDLE, C.c_longlong, C.POINTER(C.c_longlong), W.DWORD], W.BOOL)
End = api('SetEndOfFile', [W.HANDLE], W.BOOL)
Flush = api('FlushFileBuffers', [W.HANDLE], W.BOOL)
Move = api('MoveFileExW', [W.LPCWSTR, W.LPCWSTR, W.DWORD], W.BOOL)
SetInfo = api('SetFileInformationByHandle', [W.HANDLE, W.DWORD, C.c_void_p, W.DWORD], W.BOOL)

class INFO(C.Structure):
    _fields_ = [('attr', W.DWORD), ('creation', W.FILETIME), ('access', W.FILETIME), ('write', W.FILETIME), ('volume', W.DWORD), ('size_hi', W.DWORD), ('size_lo', W.DWORD), ('links', W.DWORD), ('index_hi', W.DWORD), ('index_lo', W.DWORD)]
GetInfo = api('GetFileInformationByHandle', [W.HANDLE, C.POINTER(INFO)], W.BOOL)

def check(ok):
    if not ok:
        raise C.WinError(C.get_last_error())

def handle(path, access=READ | WRITE, share=0, create=3):
    h = CreateFile(str(path), access, share, None, create, 0x80, None)
    if h == INVALID:
        raise C.WinError(C.get_last_error())
    return h

def info(h):
    x = INFO()
    check(GetInfo(h, C.byref(x)))
    return {'volume': x.volume, 'file_index': (x.index_hi << 32) | x.index_lo}

def read(h):
    check(Seek(h, 0, None, 0))
    chunks = []
    while True:
        buf, n = C.create_string_buffer(4096), W.DWORD()
        check(ReadFile(h, buf, 4096, C.byref(n), None))
        if not n.value:
            return b''.join(chunks)
        chunks.append(buf.raw[:n.value])

def write(h, data):
    check(Seek(h, 0, None, 0))
    n = W.DWORD()
    check(WriteFile(h, data, len(data), C.byref(n), None))
    if n.value != len(data):
        raise RuntimeError('short WriteFile')
    check(End(h))
    check(Flush(h))

def durable(path, data):
    h = handle(path, create=2)
    try:
        write(h, data)
    finally:
        check(CloseHandle(h))

def observe(call):
    try:
        if call() is False:
            raise C.WinError(C.get_last_error())
        return {'ok': True}
    except OSError as e:
        return {'ok': False, 'winerror': e.winerror}

def external(path, kind, other=None):
    if kind == 'write_open':
        def attempt():
            h = handle(path, WRITE, SHARE_ALL)
            check(CloseHandle(h))
        return observe(attempt)
    if kind == 'rename_dir':
        return observe(lambda: bool(Move(str(path), str(other), 8)))
    if kind == 'insert_child':
        return observe(lambda: durable(path / 'external-child.typ', b'EXTERNAL-INSERT\n'))
    raise ValueError(kind)

def child_result(args):
    p = subprocess.run([sys.executable, str(Path(__file__)), '--external', *map(str, args)], capture_output=True, text=True, timeout=15, check=True)
    return json.loads(p.stdout)

class Rename(C.Structure):
    _fields_ = [('replace', W.BOOL), ('root', W.HANDLE), ('length', W.DWORD), ('name', W.WCHAR * 1)]

def rename_by_handle(h, dest):
    raw = str(dest).encode('utf-16-le')
    size = max(C.sizeof(Rename), Rename.name.offset + len(raw) + 2)
    buf = C.create_string_buffer(size)
    header = C.cast(buf, C.POINTER(Rename)).contents
    header.replace, header.root, header.length = 0, None, len(raw)
    C.memmove(C.addressof(buf) + Rename.name.offset, raw, len(raw))
    return observe(lambda: bool(SetInfo(h, 3, buf, size)))

class IOSB(C.Structure):
    _fields_ = [('status_or_pointer', C.c_void_p), ('information', C.c_size_t)]
class UnicodeString(C.Structure):
    _fields_ = [('length', W.USHORT), ('maximum_length', W.USHORT), ('buffer', W.LPWSTR)]
class ObjectAttributes(C.Structure):
    _fields_ = [('length', W.ULONG), ('root', W.HANDLE), ('name', C.POINTER(UnicodeString)), ('attributes', W.ULONG), ('security', C.c_void_p), ('qos', C.c_void_p)]
NT = C.WinDLL('ntdll', use_last_error=True)
NtCreate = NT.NtCreateFile
NtCreate.argtypes = [C.POINTER(W.HANDLE), W.DWORD, C.POINTER(ObjectAttributes), C.POINTER(IOSB), C.c_void_p, W.ULONG, W.ULONG, W.ULONG, W.ULONG, C.c_void_p, W.ULONG]
NtCreate.restype = C.c_long

def create_directory(parent_handle, single_name):
    """FILE_CREATE returns the original directory handle; never mkdir/reopen."""
    if not single_name or single_name in ('.', '..') or any(c in single_name for c in '\\/:'):
        raise ValueError('single relative directory component required')
    buf = C.create_unicode_buffer(single_name)
    name = UnicodeString(len(single_name.encode('utf-16-le')), C.sizeof(buf), C.cast(buf, W.LPWSTR))
    attrs = ObjectAttributes(C.sizeof(ObjectAttributes), parent_handle, C.pointer(name), 0x40, None, None)
    h, ios = W.HANDLE(), IOSB()
    # LIST_DIRECTORY|READ_ATTRIBUTES|DELETE|SYNCHRONIZE; share READ|WRITE,
    # FILE_CREATE; DIRECTORY_FILE|SYNCHRONOUS_IO_NONALERT|OPEN_REPARSE_POINT.
    status = NtCreate(C.byref(h), 0x1|0x80|0x10000|0x100000, C.byref(attrs), C.byref(ios), None, 0x10, 3, 2, 0x1|0x20|0x200000, None, 0)
    if status != 0:
        raise OSError('NtCreateFile status ' + hex(status & 0xffffffff))
    if ios.information != 2:
        check(CloseHandle(h))
        raise RuntimeError('FILE_CREATED required')
    return h.value

if __name__ == '__main__':
    if len(sys.argv) < 4 or sys.argv[1] != '--external':
        raise SystemExit('helper is invoked by namespace_probe.py')
    base = Path(__file__).resolve().parent
    paths = [Path(sys.argv[2]).resolve()]
    if len(sys.argv) > 4:
        paths.append(Path(sys.argv[4]).resolve())
    if any(not p.is_relative_to(base) or not p.relative_to(base).parts[0].startswith('namespace-run-') for p in paths):
        raise SystemExit('external probe path outside its newly created fixture')
    print(json.dumps(external(paths[0], sys.argv[3], paths[1] if len(paths) > 1 else None)))
