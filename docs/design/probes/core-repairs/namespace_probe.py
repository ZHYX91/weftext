"""Windows namespace experiment; creates only fresh namespace-run-* fixtures."""
import sys
from pathlib import Path
import ctypes as C
from ctypes import wintypes as W
import json
import uuid
from datetime import datetime, timezone
HERE = Path(__file__).resolve().parent
import win32_probe as P
import win32_probe as R

SetInfo = P.api('SetFileInformationByHandle', [W.HANDLE, W.DWORD, C.c_void_p, W.DWORD], W.BOOL)
DELETE, READ_ATTR, DIR_FLAGS = 0x10000, 0x80, 0x02000000
NT = C.WinDLL('ntdll', use_last_error=True)
class IOSB(C.Structure):
    _fields_ = [('status_or_pointer', C.c_void_p), ('information', C.c_size_t)]
NtSet = NT.NtSetInformationFile
NtSet.argtypes = [W.HANDLE, C.POINTER(IOSB), C.c_void_p, W.ULONG, W.ULONG]
NtSet.restype = C.c_long

def native_rename_relative(h, parent, name):
    raw = name.encode('utf-16-le')
    size = max(C.sizeof(R.Rename), R.Rename.name.offset + len(raw) + 2)
    buf = C.create_string_buffer(size)
    header = C.cast(buf, C.POINTER(R.Rename)).contents
    header.replace, header.root, header.length = 0, parent, len(raw)
    C.memmove(C.addressof(buf) + R.Rename.name.offset, raw, len(raw))
    ios = IOSB()
    status = NtSet(h, C.byref(ios), buf, size, 10)
    return {'ok': status == 0, 'ntstatus': hex(status & 0xffffffff)}

def directory(path, delete=False):
    h = P.CreateFile(str(path), READ_ATTR | 0x1 | (DELETE if delete else 0), 3, None, 3, DIR_FLAGS, None)
    if h == P.INVALID:
        raise C.WinError(C.get_last_error())
    return h

def dispose(h):
    flag = C.c_ubyte(1)
    return P.observe(lambda: bool(SetInfo(h, 4, C.byref(flag), 1)))

def main():
    root = HERE / ('namespace-run-' + uuid.uuid4().hex[:12])
    root.mkdir()
    w, a, b = root/'W', root/'W'/'A', root/'W'/'B'
    a.mkdir(parents=True); b.mkdir()
    child = a/'N'; child.mkdir()
    P.durable(child/'N.typ', b'ORIGINAL\n')
    handles = [directory(w), directory(a), directory(b), directory(child, True)]
    result = {'utc_time': datetime.now(timezone.utc).isoformat(), 'run_directory':root.name}
    try:
        result['source_ancestor_external_rename'] = P.child_result([a,'rename_dir',w/'A-other'])
        result['target_parent_external_rename'] = P.child_result([b,'rename_dir',w/'B-other'])
        result['new_sibling_not_excluded'] = P.child_result([b,'insert_child'])
        result['win32_absolute_move_with_parent_guards'] = R.rename_by_handle(handles[-1], b/'N')
        result['own_source_same_handle_move_with_parent_guards'] = result['win32_absolute_move_with_parent_guards']
        if not result['own_source_same_handle_move_with_parent_guards']['ok']:
            result['native_relative_move_with_parent_guards'] = native_rename_relative(handles[-1],handles[2],'N')
            result['own_source_same_handle_move_with_parent_guards'] = result['native_relative_move_with_parent_guards']
    finally:
        for h in reversed(handles): P.check(P.CloseHandle(h))
    if not result['own_source_same_handle_move_with_parent_guards']['ok']:
        print(json.dumps(result,indent=2),flush=True)
        raise RuntimeError('guarded directory move did not succeed')
    result['moved_original_preserved'] = (b/'N'/'N.typ').read_bytes() == b'ORIGINAL\n'

    copy = w/'Copied'
    parent = directory(w)
    try:
        dh = P.create_directory(parent, 'Copied')
        created_identity = P.info(dh)
        try:
            duplicate = P.create_directory(parent, 'Copied')
        except OSError as e:
            result['directory_create_collision'] = str(e)
        else:
            P.CloseHandle(duplicate)
            raise AssertionError('FILE_CREATE must not open existing directory')
        result['directory_creation_handle_retained'] = P.info(dh) == created_identity
    finally:
        P.check(P.CloseHandle(parent))
    fh = P.handle(copy/'Copied.typ', create=1)
    try:
        P.write(fh, b'FROZEN-COPY\n')
        result['planned_created_file_external_write'] = P.child_result([copy/'Copied.typ','write_open'])
        result['copy_unknown_child_insert'] = P.child_result([copy,'insert_child'])
        result['planned_created_file_exact'] = P.read(fh) == b'FROZEN-COPY\n'
        result['copy_root_external_rename'] = P.child_result([copy,'rename_dir',w/'OtherCopy'])
    finally:
        P.check(P.CloseHandle(fh)); P.check(P.CloseHandle(dh))
    result['copy_unknown_preserved'] = (copy/'external-child.typ').read_bytes() == b'EXTERNAL-INSERT\n'

    trash = w/'Trash'; trash.mkdir()
    P.durable(trash/'known.typ', b'KNOWN\n')
    th = directory(trash, True)
    known = P.handle(trash/'known.typ', access=P.READ|P.WRITE|DELETE, share=0)
    try:
        result['purge_unknown_insert'] = P.child_result([trash,'insert_child'])
        result['known_handle_dispose'] = dispose(known)
    finally:
        P.check(P.CloseHandle(known))
    result['known_removed'] = not (trash/'known.typ').exists()
    result['root_delete_nonempty'] = dispose(th)
    P.check(P.CloseHandle(th))
    result['unknown_preserved_after_nonempty_failure'] = (trash/'external-child.typ').read_bytes() == b'EXTERNAL-INSERT\n'
    # Explicitly selected empty directory, no recursion or unknown-byte removal.
    empty = w/'Empty'
    parent = directory(w)
    try:
        eh = P.create_directory(parent, 'Empty')
    finally:
        P.check(P.CloseHandle(parent))
    result['empty_directory_handle_dispose'] = dispose(eh)
    P.check(P.CloseHandle(eh))
    result['empty_directory_removed'] = not empty.exists()
    assert not result['source_ancestor_external_rename']['ok']
    assert not result['target_parent_external_rename']['ok']
    assert result['own_source_same_handle_move_with_parent_guards']['ok']
    assert result['moved_original_preserved']
    assert not result['planned_created_file_external_write']['ok']
    assert result['copy_unknown_preserved'] and result['planned_created_file_exact']
    assert not result['copy_root_external_rename']['ok']
    assert result['known_removed']
    assert not result['root_delete_nonempty']['ok']
    assert result['unknown_preserved_after_nonempty_failure']
    assert result['empty_directory_removed']
    assert result['directory_creation_handle_retained']
    result['assertions_passed'] = True
    print(json.dumps(result, indent=2))

if __name__ == '__main__': main()
