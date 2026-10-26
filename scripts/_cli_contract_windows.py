"""Existing held Win32 identity and owned ConPTY mechanisms; no eager process work."""
import ctypes
from ctypes import wintypes
import os
from pathlib import Path
import re
import shutil
import subprocess
import threading
import time
import traceback

from _cli_contract_common import MAX_LOG_READ, Nonrun, require, utc

CONPTY_SOURCE = 'https://learn.microsoft.com/windows/console/creating-a-pseudoconsole-session'


class Win32:
    """Only process identity/wait and private pseudoconsole primitives."""
    def __init__(self):
        if os.name != 'nt':
            raise Nonrun('This dual-PE/Windows owner contract requires Windows')
        self.k = ctypes.WinDLL('kernel32', use_last_error=True)
        self.k.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        self.k.OpenProcess.restype = wintypes.HANDLE
        self.k.CloseHandle.argtypes = [wintypes.HANDLE]
        self.k.CloseHandle.restype = wintypes.BOOL
        self.k.GetProcessTimes.argtypes = [wintypes.HANDLE] + [ctypes.POINTER(wintypes.FILETIME)] * 4
        self.k.GetProcessTimes.restype = wintypes.BOOL
        self.k.QueryFullProcessImageNameW.argtypes = [wintypes.HANDLE, wintypes.DWORD,
                                                    wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD)]
        self.k.QueryFullProcessImageNameW.restype = wintypes.BOOL
        self.k.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
        self.k.WaitForSingleObject.restype = wintypes.DWORD
        self.k.GetExitCodeProcess.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)]
        self.k.GetExitCodeProcess.restype = wintypes.BOOL

    @staticmethod
    def check(ok):
        if not ok:
            raise ctypes.WinError(ctypes.get_last_error())

    def identity(self, handle, pid):
        values = [wintypes.FILETIME() for _ in range(4)]
        self.check(self.k.GetProcessTimes(handle, *(ctypes.byref(v) for v in values)))
        created = (int(values[0].dwHighDateTime) << 32) | int(values[0].dwLowDateTime)
        size = wintypes.DWORD(32768)
        image = ctypes.create_unicode_buffer(size.value)
        self.check(self.k.QueryFullProcessImageNameW(handle, 0, image, ctypes.byref(size)))
        return {'pid': int(pid), 'created': created, 'image': str(Path(image.value).resolve())}

    @staticmethod
    def local_app_data():
        # FOLDERID_LocalAppData; no environment-variable fallback or create flag.
        folder_id = (ctypes.c_ubyte * 16).from_buffer_copy(
            bytes.fromhex('8527b3f1ba6fcf4f9d557b8e7f157091'))
        shell = ctypes.WinDLL('shell32.dll', winmode=0x800)
        ole = ctypes.WinDLL('ole32.dll', winmode=0x800)
        query = shell.SHGetKnownFolderPath
        query.argtypes = [ctypes.c_void_p, wintypes.DWORD, wintypes.HANDLE,
                          ctypes.POINTER(ctypes.c_void_p)]
        query.restype = ctypes.c_long
        free = ole.CoTaskMemFree
        free.argtypes = [ctypes.c_void_p]
        free.restype = None
        value = ctypes.c_void_p()
        result = query(ctypes.byref(folder_id), 0, None, ctypes.byref(value))
        try:
            if result < 0:
                raise OSError(result, 'LocalAppData Known Folder lookup failed')
            if not value.value:
                raise OSError('LocalAppData Known Folder returned no path')
            return Path(ctypes.wstring_at(value))
        finally:
            free(value)

    def open_owner(self, expected, executable):
        handle = self.k.OpenProcess(0x1000 | 0x100000, False, expected['pid'])
        self.check(handle)
        try:
            identity = self.identity(handle, expected['pid'])
            require(identity['created'] == expected['created'], 'Owner PID creation identity differs')
            require(os.path.normcase(identity['image']) == os.path.normcase(str(executable)),
                    'Owner image is not the supplied GUI role artifact')
            require(self.k.WaitForSingleObject(handle, 0) == 258, 'Owner already exited')
            return handle, identity
        except Exception:
            self.k.CloseHandle(handle)
            raise

    def exit_code(self, handle):
        value = wintypes.DWORD()
        self.check(self.k.GetExitCodeProcess(handle, ctypes.byref(value)))
        return int(value.value)

class Pseudoconsole:
    """Real ConPTY with a concurrently drained output channel and owned shell."""
    def __init__(self, api, shell, cwd, output, arguments=('/D', '/Q', '/K'), environment=None):
        self.api, self.output = api, output
        self.input_write = self.output_read = self.pc = None
        self.process = self.thread_handle = None
        self.reader = None
        self.data = bytearray()
        self.read_error = None
        self.changed = threading.Condition()
        k = api.k
        if not hasattr(k, 'CreatePseudoConsole'):
            raise Nonrun('CreatePseudoConsole is unavailable on the actual Windows host')

        class Coord(ctypes.Structure):
            _fields_ = [('X', ctypes.c_short), ('Y', ctypes.c_short)]

        class Startup(ctypes.Structure):
            _fields_ = [('cb', wintypes.DWORD), ('lpReserved', wintypes.LPWSTR),
                        ('lpDesktop', wintypes.LPWSTR), ('lpTitle', wintypes.LPWSTR),
                        ('dwX', wintypes.DWORD), ('dwY', wintypes.DWORD),
                        ('dwXSize', wintypes.DWORD), ('dwYSize', wintypes.DWORD),
                        ('dwXCountChars', wintypes.DWORD), ('dwYCountChars', wintypes.DWORD),
                        ('dwFillAttribute', wintypes.DWORD), ('dwFlags', wintypes.DWORD),
                        ('wShowWindow', wintypes.WORD), ('cbReserved2', wintypes.WORD),
                        ('lpReserved2', ctypes.POINTER(ctypes.c_byte)),
                        ('hStdInput', wintypes.HANDLE), ('hStdOutput', wintypes.HANDLE),
                        ('hStdError', wintypes.HANDLE)]

        class StartupEx(ctypes.Structure):
            _fields_ = [('StartupInfo', Startup), ('lpAttributeList', ctypes.c_void_p)]

        class ProcessInfo(ctypes.Structure):
            _fields_ = [('hProcess', wintypes.HANDLE), ('hThread', wintypes.HANDLE),
                        ('dwProcessId', wintypes.DWORD), ('dwThreadId', wintypes.DWORD)]

        k.CreatePipe.argtypes = [ctypes.POINTER(wintypes.HANDLE), ctypes.POINTER(wintypes.HANDLE),
                                 ctypes.c_void_p, wintypes.DWORD]
        k.CreatePipe.restype = wintypes.BOOL
        k.CreatePseudoConsole.argtypes = [Coord, wintypes.HANDLE, wintypes.HANDLE,
                                          wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)]
        k.CreatePseudoConsole.restype = ctypes.c_long
        k.ClosePseudoConsole.argtypes = [wintypes.HANDLE]
        k.ClosePseudoConsole.restype = None
        k.InitializeProcThreadAttributeList.argtypes = [ctypes.c_void_p, wintypes.DWORD,
                                                       wintypes.DWORD, ctypes.POINTER(ctypes.c_size_t)]
        k.InitializeProcThreadAttributeList.restype = wintypes.BOOL
        k.UpdateProcThreadAttribute.argtypes = [ctypes.c_void_p, wintypes.DWORD, ctypes.c_size_t,
                                                ctypes.c_void_p, ctypes.c_size_t,
                                                ctypes.c_void_p, ctypes.c_void_p]
        k.UpdateProcThreadAttribute.restype = wintypes.BOOL
        k.DeleteProcThreadAttributeList.argtypes = [ctypes.c_void_p]
        k.DeleteProcThreadAttributeList.restype = None
        k.CreateProcessW.argtypes = [wintypes.LPCWSTR, wintypes.LPWSTR, ctypes.c_void_p,
                                     ctypes.c_void_p, wintypes.BOOL, wintypes.DWORD,
                                     ctypes.c_void_p, wintypes.LPCWSTR,
                                     ctypes.POINTER(Startup), ctypes.POINTER(ProcessInfo)]
        k.CreateProcessW.restype = wintypes.BOOL
        k.ReadFile.argtypes = [wintypes.HANDLE, ctypes.c_void_p, wintypes.DWORD,
                               ctypes.POINTER(wintypes.DWORD), ctypes.c_void_p]
        k.ReadFile.restype = wintypes.BOOL
        k.WriteFile.argtypes = list(k.ReadFile.argtypes)
        k.WriteFile.restype = wintypes.BOOL
        k.GetStdHandle.argtypes = [wintypes.DWORD]
        k.GetStdHandle.restype = wintypes.HANDLE
        k.SetStdHandle.argtypes = [wintypes.DWORD, wintypes.HANDLE]
        k.SetStdHandle.restype = wintypes.BOOL
        input_read, input_write = wintypes.HANDLE(), wintypes.HANDLE()
        output_read, output_write = wintypes.HANDLE(), wintypes.HANDLE()
        pc = wintypes.HANDLE()
        attribute = None
        attribute_ready = False
        failed = False
        try:
            api.check(k.CreatePipe(ctypes.byref(input_read), ctypes.byref(input_write), None, 0))
            self.input_write = input_write.value
            api.check(k.CreatePipe(ctypes.byref(output_read), ctypes.byref(output_write), None, 0))
            self.output_read = output_read.value
            hr = k.CreatePseudoConsole(Coord(4000, 80), input_read, output_write, 0, ctypes.byref(pc))
            require(hr == 0, f'CreatePseudoConsole failed: HRESULT {hr & 0xffffffff:08x}')
            self.pc = pc.value
            self.reader = threading.Thread(target=self._read, name='weftext-conpty-drain', daemon=True)
            self.reader.start()
            required = ctypes.c_size_t()
            k.InitializeProcThreadAttributeList(None, 1, 0, ctypes.byref(required))
            require(required.value > 0, 'No ConPTY attribute allocation size')
            attribute = ctypes.create_string_buffer(required.value)
            api.check(k.InitializeProcThreadAttributeList(attribute, 1, 0, ctypes.byref(required)))
            attribute_ready = True
            api.check(k.UpdateProcThreadAttribute(attribute, 0, 0x00020016, pc,
                                                  ctypes.sizeof(pc), None, None))
            startup, process = StartupEx(), ProcessInfo()
            startup.StartupInfo.cb = ctypes.sizeof(StartupEx)
            startup.StartupInfo.dwFlags = 1  # SW_HIDE, no interactive desktop window.
            startup.StartupInfo.wShowWindow = 0
            startup.lpAttributeList = ctypes.cast(attribute, ctypes.c_void_p)
            # /K is interactive. In particular, this is not cmd /c under Popen.
            command = ctypes.create_unicode_buffer(subprocess.list2cmdline([str(shell), *arguments]))
            # The check entry itself normally inherits redirected/EOF pipes.
            # Do not let an interactive shell treat those as its stdin. Save
            # only this harness process's std-handle table, briefly clear it
            # while creating the HPC-attached child, then restore immediately.
            # No caller/tool/user handle is closed, inherited, or reconfigured.
            standard = (0xfffffff6, 0xfffffff5, 0xfffffff4)
            saved = [(which, k.GetStdHandle(which)) for which in standard]
            environment_block = None
            if environment is not None:
                environment_block = ctypes.create_unicode_buffer('\0'.join(
                    f'{key}={value}' for key, value in sorted(environment.items(), key=lambda item: item[0].upper())
                ) + '\0\0')
            try:
                for which, _ in saved:
                    api.check(k.SetStdHandle(which, None))
                api.check(k.CreateProcessW(str(shell), command, None, None, False, 0x00080000 | 0x400,
                                           environment_block, str(cwd), ctypes.byref(startup.StartupInfo),
                                           ctypes.byref(process)))
                self.process, self.thread_handle = process.hProcess, process.hThread
            finally:
                restore_failures = []
                for which, handle in saved:
                    if not k.SetStdHandle(which, handle):
                        restore_failures.append(ctypes.get_last_error())
                require(not restore_failures, 'Harness std-handle restoration failed: ' + str(restore_failures))
            self.identity = api.identity(self.process, process.dwProcessId)
        except BaseException:
            failed = True
            raise
        finally:
            if attribute_ready:
                k.DeleteProcThreadAttributeList(attribute)
            for handle in (input_read.value, output_write.value):
                if handle:
                    k.CloseHandle(handle)
            if failed:
                # No product command has been sent. Drain the pseudoconsole
                # while closing this incompletely initialized owned shell.
                if self.pc and self.output_read:
                    if self.reader is None:
                        self.reader = threading.Thread(target=self._read,
                            name='weftext-conpty-failed-drain', daemon=True)
                        self.reader.start()
                    self.close()
                else:
                    for handle in (self.input_write, self.output_read):
                        if handle:
                            k.CloseHandle(handle)

    def _read(self):
        try:
            with self.output.open('xb') as log:
                buffer = ctypes.create_string_buffer(8192)
                while True:
                    received = wintypes.DWORD()
                    ok = self.api.k.ReadFile(self.output_read, buffer, len(buffer),
                                             ctypes.byref(received), None)
                    if not ok:
                        code = ctypes.get_last_error()
                        if code not in (109, 232, 995):
                            raise ctypes.WinError(code)
                        break
                    if not received.value:
                        break
                    part = buffer.raw[:received.value]
                    log.write(part)
                    log.flush()
                    with self.changed:
                        if len(self.data) + len(part) <= MAX_LOG_READ:
                            self.data.extend(part)
                        else:
                            self.read_error = 'ConPTY output exceeded the bounded transcript budget'
                        self.changed.notify_all()
        except Exception:
            self.read_error = traceback.format_exc()
        finally:
            with self.changed:
                self.changed.notify_all()

    def send(self, text):
        data = text.encode('utf-8')
        sent = wintypes.DWORD()
        self.api.check(self.api.k.WriteFile(self.input_write, data, len(data), ctypes.byref(sent), None))
        require(sent.value == len(data), 'ConPTY input was incomplete')

    def wait_marker(self, marker, timeout=60):
        deadline = time.monotonic() + timeout
        pattern = re.compile(rb'(?m)^' + re.escape(marker.encode('ascii')) + rb':([0-9]+)\r*$')
        with self.changed:
            while True:
                if self.read_error:
                    raise AssertionError(self.read_error)
                # ConPTY can move to the next prompt with an absolute cursor
                # command instead of CRLF. Preserve boundaries only around
                # this exact result marker; JSON wrapping remains untouched.
                observed = bytes(self.data)
                marker_bytes = re.escape(marker.encode('ascii'))
                observed = re.sub(rb'\x1b\[[0-9]+;1[Hf](?=' + marker_bytes + rb':[0-9]+)',
                                  b'\n', observed)
                observed = re.sub(rb'(' + marker_bytes + rb':[0-9]+\r*)\x1b\[[0-9]+;1[Hf]',
                                  rb'\1\n', observed)
                clean = strip_vt(observed)
                found = pattern.search(clean)
                if found:
                    return clean, found
                if self.api.k.WaitForSingleObject(self.process, 0) == 0:
                    raise AssertionError('The interactive shell exited before its result marker')
                left = deadline - time.monotonic()
                if left <= 0:
                    raise TimeoutError('The real interactive cmd did not produce its result marker')
                self.changed.wait(min(0.25, left))

    def close(self):
        receipt = {'process': getattr(self, 'identity', None), 'startedAt': utc()}
        if self.process:
            try:
                self.send('exit 0\r\n')
                waited = self.api.k.WaitForSingleObject(self.process, 15000)
                receipt['naturalShellExit'] = waited == 0
                if waited == 0:
                    receipt['exitCode'] = self.api.exit_code(self.process)
            except BaseException:
                receipt['error'] = traceback.format_exc()
        # ClosePseudoConsole may emit a last frame: the independent reader stays
        # alive through close. Only this owned, read-only shell is attached here.
        closed = threading.Event()
        def finish_pc():
            if self.pc:
                self.api.k.ClosePseudoConsole(self.pc)
            closed.set()
        closer = threading.Thread(target=finish_pc, name='weftext-conpty-close', daemon=True)
        closer.start()
        receipt['pseudoconsoleClosed'] = closed.wait(15)
        if self.reader and receipt['pseudoconsoleClosed']:
            self.reader.join(timeout=5)
            receipt['outputDrained'] = not self.reader.is_alive()
        for handle in (self.input_write, self.output_read, self.thread_handle, self.process):
            if handle and receipt['pseudoconsoleClosed']:
                self.api.k.CloseHandle(handle)
        receipt['completedAt'] = utc()
        return receipt

def strip_vt(data):
    # Retain the original transcript separately; this copy is only for locating
    # complete product JSON and actual shell result lines, not display testing.
    data = re.sub(rb'\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)', b'', data)
    data = re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]', b'', data)
    return data.replace(b'\x1b=', b'').replace(b'\x1b>', b'')

def system_cmd():
    value = Path(os.environ.get('SystemRoot', r'C:\Windows')) / 'System32/cmd.exe'
    if not value.is_file():
        raise Nonrun('The actual system cmd.exe is unavailable')
    return value.resolve(strict=True)

def installed_powershells():
    pwsh = shutil.which('pwsh.exe')
    winps = Path(os.environ.get('SystemRoot', r'C:\Windows')) / 'System32/WindowsPowerShell/v1.0/powershell.exe'
    return [('pwsh', Path(pwsh).resolve(strict=True) if pwsh else None),
            ('winps', winps.resolve(strict=True) if winps.is_file() else None)]

def ps_quote(value):
    return "'" + str(value).replace("'", "''") + "'"

def cmd_quote(value):
    # Shell tests consume known owned paths only. Reject shell expansion syntax
    # rather than assembling a command from an accidentally active path string.
    if any(char in str(value) for char in '"%!&|<>^\r\n'):
        raise Nonrun('Owned paths contain cmd expansion syntax; shell case was not executed')
    return '"' + str(value) + '"'

def cmd_product(executable, profile):
    return cmd_quote(executable) + ' --data-dir ' + cmd_quote(profile) + ' capabilities'
