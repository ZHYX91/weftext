"""Read-only, cache-only Windows Authenticode checks; no installer execution."""
import ctypes as C
from ctypes import wintypes as W
import hashlib
from pathlib import Path
import uuid


class GUID(C.Structure):
    _fields_ = [('Data1', W.DWORD), ('Data2', W.WORD), ('Data3', W.WORD), ('Data4', C.c_ubyte * 8)]


class FILE_INFO(C.Structure):
    _fields_ = [('size', W.DWORD), ('path', W.LPCWSTR), ('handle', W.HANDLE), ('subject', C.c_void_p)]


class TRUST_DATA(C.Structure):
    _fields_ = [('size', W.DWORD), ('policy', C.c_void_p), ('sip', C.c_void_p), ('ui', W.DWORD), ('revocation', W.DWORD), ('choice', W.DWORD), ('file', C.c_void_p), ('action', W.DWORD), ('state', W.HANDLE), ('url', W.LPCWSTR), ('flags', W.DWORD), ('context', W.DWORD), ('settings', C.c_void_p)]


class SIGNER_PREFIX(C.Structure):
    _fields_ = [('size', W.DWORD), ('time', W.FILETIME), ('certificates', W.DWORD)]


class PROVIDER_CERT_PREFIX(C.Structure):
    _fields_ = [('size', W.DWORD), ('certificate', C.c_void_p)]


class CERT_CONTEXT(C.Structure):
    _fields_ = [('encoding', W.DWORD), ('der', C.c_void_p), ('length', W.DWORD), ('info', C.c_void_p), ('store', W.HANDLE)]


def dll(name):
    return C.WinDLL(name, use_last_error=True, winmode=0x800)  # System32 only.


def function(library, name, arguments, result):
    value = getattr(library, name)
    value.argtypes, value.restype = arguments, result
    return value


def file_version(path):
    library = dll('version.dll')
    size_api = function(library, 'GetFileVersionInfoSizeW', [W.LPCWSTR, C.POINTER(W.DWORD)], W.DWORD)
    read_api = function(library, 'GetFileVersionInfoW', [W.LPCWSTR, W.DWORD, W.DWORD, C.c_void_p], W.BOOL)
    query_api = function(library, 'VerQueryValueW', [C.c_void_p, W.LPCWSTR, C.POINTER(C.c_void_p), C.POINTER(W.UINT)], W.BOOL)
    ignored = W.DWORD()
    size = size_api(str(path), C.byref(ignored))
    if not 0 < size <= 16 * 1024 * 1024:
        raise ValueError('Missing or oversized Windows version resource')
    buffer = C.create_string_buffer(size)
    if not read_api(str(path), 0, size, buffer):
        raise C.WinError(C.get_last_error())
    pointer, length = C.c_void_p(), W.UINT()
    if not query_api(buffer, '\\', C.byref(pointer), C.byref(length)) or length.value < 52:
        raise ValueError('Missing fixed Windows file version')
    info = C.cast(pointer, C.POINTER(W.DWORD * 13)).contents
    if info[0] != 0xFEEF04BD:
        raise ValueError('Invalid Windows version signature')
    return (info[2] >> 16, info[2] & 65535, info[3] >> 16, info[3] & 65535)


def verify_material(path, expected_sha256, expected_size, expected_subject):
    """Return exactly the verified handle's bytes, never a later path copy.

    CACHE_ONLY_URL_RETRIEVAL prevents chain/CRL network retrieval. This is not
    a claim of current online revocation status. A missing cached chain fails.
    """
    kernel, trust, crypt = (dll(name) for name in ('kernel32.dll', 'wintrust.dll', 'crypt32.dll'))
    create = function(kernel, 'CreateFileW', [W.LPCWSTR, W.DWORD, W.DWORD, C.c_void_p, W.DWORD, W.DWORD, W.HANDLE], W.HANDLE)
    read = function(kernel, 'ReadFile', [W.HANDLE, C.c_void_p, W.DWORD, C.POINTER(W.DWORD), C.c_void_p], W.BOOL)
    seek = function(kernel, 'SetFilePointerEx', [W.HANDLE, C.c_longlong, C.POINTER(C.c_longlong), W.DWORD], W.BOOL)
    close = function(kernel, 'CloseHandle', [W.HANDLE], W.BOOL)
    final_path = function(kernel, 'GetFinalPathNameByHandleW', [W.HANDLE, W.LPWSTR, W.DWORD, W.DWORD], W.DWORD)
    verify = function(trust, 'WinVerifyTrust', [W.HWND, C.POINTER(GUID), C.POINTER(TRUST_DATA)], W.LONG)
    provider = function(trust, 'WTHelperProvDataFromStateData', [W.HANDLE], C.c_void_p)
    signer = function(trust, 'WTHelperGetProvSignerFromChain', [C.c_void_p, W.DWORD, W.BOOL, W.DWORD], C.c_void_p)
    certificate = function(trust, 'WTHelperGetProvCertFromChain', [C.c_void_p, W.DWORD], C.c_void_p)
    name_api = function(crypt, 'CertGetNameStringW', [C.c_void_p, W.DWORD, W.DWORD, C.c_void_p, W.LPWSTR, W.DWORD], W.DWORD)
    path = Path(path).absolute()
    expected_path = path.resolve(strict=True)
    # Caller qualifies the material tree first. No write/delete sharing while
    # hashing and WinVerifyTrust use this same handle.
    handle = create(str(path), 0x80000000, 1, None, 3, 0x00200000, None)
    if handle in (None, C.c_void_p(-1).value):
        raise C.WinError(C.get_last_error())
    state = TRUST_DATA()
    state.size, state.ui, state.choice, state.action = C.sizeof(state), 2, 1, 1
    state.flags = 0x3010  # CACHE_ONLY | REVOCATION_CHECK_NONE | DISABLE_MD2_MD4.
    file_info = FILE_INFO(C.sizeof(FILE_INFO), str(path), handle, None)
    state.file = C.cast(C.pointer(file_info), C.c_void_p)
    action = GUID.from_buffer_copy(uuid.UUID('00aac56b-cd44-11d0-8cc2-00c04fc295ee').bytes_le)
    attempted = False
    close_status = None
    def content():
        if not seek(handle, 0, None, 0):
            raise C.WinError(C.get_last_error())
        result = bytearray()
        buffer = C.create_string_buffer(1024 * 1024)
        while True:
            count = W.DWORD()
            if not read(handle, buffer, len(buffer), C.byref(count), None):
                raise C.WinError(C.get_last_error())
            if not count.value:
                return bytes(result)
            result.extend(buffer.raw[:count.value])
            if len(result) > expected_size:
                raise ValueError('Material exceeds its locked byte length')
    try:
        buffer = C.create_unicode_buffer(32768)
        length = final_path(handle, buffer, len(buffer), 0)
        if not 0 < length < len(buffer):
            raise ValueError('Cannot qualify material handle path')
        actual = buffer.value
        actual = '\\\\' + actual[8:] if actual.startswith('\\\\?\\UNC\\') else actual.removeprefix('\\\\?\\')
        if Path(actual) != expected_path:
            raise ValueError('Material path changed before verification')
        before = content()
        if len(before) != expected_size or hashlib.sha256(before).hexdigest() != expected_sha256:
            raise ValueError('Material differs from the reviewed repository lock')
        if not seek(handle, 0, None, 0):
            raise C.WinError(C.get_last_error())
        attempted = True
        status = verify(C.c_void_p(-1), C.byref(action), C.byref(state))
        if status != 0:
            raise ValueError(f'Offline Authenticode trust not established: 0x{status & 0xffffffff:08x}')
        data_pointer = provider(state.state)
        signer_pointer = signer(data_pointer, 0, False, 0) if data_pointer else None
        if not signer_pointer:
            raise ValueError('Verified signer is unavailable')
        header = C.cast(signer_pointer, C.POINTER(SIGNER_PREFIX)).contents
        if header.size < C.sizeof(SIGNER_PREFIX) or not 0 < header.certificates <= 32:
            raise ValueError('Invalid verified signer bounds')
        entry = certificate(signer_pointer, 0)
        if not entry:
            raise ValueError('Verified leaf certificate is missing')
        leaf = C.cast(entry, C.POINTER(PROVIDER_CERT_PREFIX)).contents
        if leaf.size < C.sizeof(PROVIDER_CERT_PREFIX) or not leaf.certificate:
            raise ValueError('Invalid verified leaf certificate')
        form = W.DWORD(3)  # CERT_X500_NAME_STR, CERT_NAME_RDN_TYPE.
        name_length = name_api(leaf.certificate, 2, 0, C.byref(form), None, 0)
        if not 1 < name_length <= 65536:
            raise ValueError('Missing verified publisher subject')
        subject = C.create_unicode_buffer(name_length)
        if name_api(leaf.certificate, 2, 0, C.byref(form), subject, name_length) != name_length or subject.value != expected_subject:
            raise ValueError('Verified publisher differs from the repository material lock')
        cert = C.cast(leaf.certificate, C.POINTER(CERT_CONTEXT)).contents
        if not cert.der or not 0 < cert.length <= 1024 * 1024:
            raise ValueError('Invalid certificate bounds')
        certificate_hash = hashlib.sha256(C.string_at(cert.der, cert.length)).hexdigest()
        if content() != before:
            raise ValueError('Material changed during offline signature verification')
        result = {'publisherSubject': subject.value, 'leafCertificateSha256': certificate_hash, 'winVerifyTrustStatus': 0, 'cacheOnly': True, 'onlineRevocationChecked': False}
    finally:
        try:
            if attempted:
                state.action = 2
                close_status = verify(C.c_void_p(-1), C.byref(action), C.byref(state))
        finally:
            closed = close(handle)
    if close_status != 0 or not closed:
        raise ValueError('Offline verification resources did not close successfully')
    return before, result
