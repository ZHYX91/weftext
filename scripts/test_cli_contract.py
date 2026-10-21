"""Actual dual-PE/CLI contract acceptance in create-new, retained TEST directories.

The caller supplies both Cargo-reported role artifacts. This script never builds,
guesses an executable, launches the default GUI, or borrows workspace fixtures.
PASS means this invocation ran that case. NONRUN is an incomplete check and makes
the script fail. All original inputs, process receipts and output bytes survive.

Worker fixtures follow test_diagram_docx.py:48 and the public engine integration
test notes_editing.rs::frozen_endnotes_use_exact_authorized_module_actual_callsite_and_one_draft.
ConPTY follows https://learn.microsoft.com/windows/console/creating-a-pseudoconsole-session.
It attaches owned interactive cmd/PowerShell hosts; tested commands subsequently
arrive through the private input pipe. It never uses the desktop's keyboard,
terminal window, clipboard or existing console session. Windows PowerShell 5
redirection's UTF-16LE is recorded as a shell transformation, separately from
the product's raw UTF-8 and create-only --output bytes:
https://learn.microsoft.com/powershell/module/microsoft.powershell.core/about/about_character_encoding
"""
from __future__ import annotations

import argparse
import base64
import ctypes
from ctypes import wintypes
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import threading
import time
import traceback
import uuid


SOURCE_ROOT = Path(__file__).resolve().parents[1]
CREATE_NO_WINDOW = 0x08000000
MAX_LOG_READ = 26 * 1024 * 1024
CONPTY_SOURCE = 'https://learn.microsoft.com/windows/console/creating-a-pseudoconsole-session'
CASES = (
    'artifact-roles', 'dual-pe-frozen-workers', 'actual-local-owner',
    'library-read-literal-save-original', 'query-confirmed-artifact-download',
    'bibliography-import-original-plan',
    'local-sync-portable-records',
    'conflict-preserves-committed-source', 'unknown-original-durable-fence',
    'partial-result-and-input-budget', 'cross-profile-original-rejection',
    'original-save-after-process-restart', 'real-broken-pipe-original-commit',
    'actual-cmd-and-powershell', 'actual-interactive-shells-conpty', 'actual-hosted-http-cli',
)


class Nonrun(RuntimeError):
    """An explicit missing qualification, never silently treated as PASS."""


def utc():
    return dt.datetime.now(dt.timezone.utc).isoformat()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def file_digest(path):
    with path.open('rb') as file:
        return hashlib.file_digest(file, 'sha256').hexdigest()


def uid():
    return str(uuid.uuid4())


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def unique_json(data):
    def pairs(items):
        out = {}
        for key, value in items:
            if key in out:
                raise ValueError('duplicate JSON key: ' + key)
            out[key] = value
        return out
    return json.loads(data.decode('utf-8'), object_pairs_hook=pairs)


def wire(value):
    return json.dumps(value, ensure_ascii=False, separators=(',', ':')).encode('utf-8')


def keep_json(path, value):
    with path.open('x', encoding='utf-8', newline='\n') as file:
        json.dump(value, file, ensure_ascii=False, indent=2)
        file.write('\n')


def read_small(path):
    with path.open('rb') as file:
        data = file.read(MAX_LOG_READ + 1)
    require(len(data) <= MAX_LOG_READ, f'Output exceeded the bounded test read: {path}')
    return data


def pe_identity(path):
    with path.open('rb') as file:
        dos = file.read(64)
        require(dos[:2] == b'MZ', f'Not a PE file: {path}')
        offset = struct.unpack_from('<I', dos, 0x3c)[0]
        require(64 <= offset <= 1024 * 1024, 'Invalid PE header offset')
        file.seek(offset)
        header = file.read(24 + 70)
    require(header[:4] == b'PE\0\0', 'Invalid PE signature')
    return {'path': str(path), 'sha256': file_digest(path),
            'length': path.stat().st_size, 'machine': struct.unpack_from('<H', header, 4)[0],
            'optionalMagic': struct.unpack_from('<H', header, 24)[0],
            'subsystem': struct.unpack_from('<H', header, 24 + 68)[0]}


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
                clean = strip_vt(bytes(self.data))
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


class Harness:
    def __init__(self, args):
        self.args = args
        self.test, self.output = args.test_root, args.output_dir
        self.api = None
        self.owners, self.pending, self.attempted_profiles = {}, [], set()
        self.shutdown_attempted = set()
        self.sequence = 0
        self.report = {'schemaVersion': 1, 'startedAt': utc(), 'passed': False,
                       'testRoot': str(self.test), 'outputDir': str(self.output),
                       'scope': 'Actual dual-PE workers, CLI business and owned-shell contract; no default GUI launch',
                       'unknownEvidenceScope': {
                           'neverSubmittedOriginal': 'unknown with no registered Plan, durable fence, then late prepare rejected',
                           'committedOutputLoss': 'closed stdout, exit 7, then exact original commit lookup',
                           'acceptedTransportReplyLoss': 'NONRUN: no product transport fault case integrated'},
                       'cases': [{'name': name, 'status': 'NONRUN', 'reason': 'Not reached'}
                                 for name in CASES], 'processes': [], 'ownerShutdown': [],
                       'workerObservations': [{'role': role, 'worker': worker, 'status': 'NONRUN'}
                           for role in ('gui', 'cli') for worker in ('render', 'notes')]}

    def artifact(self, name, data=None):
        path = self.output / name
        if data is not None:
            with path.open('xb') as file:
                file.write(data)
        return path

    def json_file(self, name, value):
        path = self.output / name
        keep_json(path, value)
        return path

    def case(self, name, function, requires=()):
        item = next(item for item in self.report['cases'] if item['name'] == name)
        item.update(startedAt=utc(), status='NONRUN')
        item.pop('reason', None)
        try:
            passed = {case['name'] for case in self.report['cases'] if case['status'] == 'PASS'}
            if not set(requires) <= passed:
                raise Nonrun('Required earlier case did not pass: ' + ', '.join(set(requires) - passed))
            detail = function()
            item.update(status='PASS', detail=detail)
        except Nonrun as error:
            item.update(status='NONRUN', reason=str(error))
        except Exception:
            item.update(status='FAIL', error=traceback.format_exc())
        item['completedAt'] = utc()
        print(f"{item['status']} {name}", flush=True)

    def run_process(self, label, command, *, input_bytes=None, expected=0,
                    env=None, timeout=120, worker=False, broken_stdout=False,
                    executable=None):
        self.sequence += 1
        stem = f'{self.sequence:03d}-{label}'
        out, err = self.artifact(stem + '.stdout.bin'), self.artifact(stem + '.stderr.bin')
        if input_bytes is not None:
            self.artifact(stem + '.stdin.bin', input_bytes)
        metadata = {'label': label, 'command': command, 'startedAt': utc(),
                    'inputLength': len(input_bytes) if input_bytes is not None else None,
                    'inputSha256': digest(input_bytes) if input_bytes is not None else None,
                    'stdout': out.name, 'stderr': err.name, 'brokenStdout': broken_stdout}
        self.report['processes'].append(metadata)
        start = time.monotonic()
        with out.open('xb') as stdout, err.open('xb') as stderr:
            process = subprocess.Popen(command, executable=executable, cwd=self.test,
                stdin=subprocess.PIPE if input_bytes is not None else subprocess.DEVNULL,
                stdout=subprocess.PIPE if broken_stdout else stdout, stderr=stderr,
                env=env, creationflags=CREATE_NO_WINDOW if os.name == 'nt' else 0)
            metadata['pid'] = process.pid
            try:
                if self.api:
                    metadata['identity'] = self.api.identity(int(process._handle), process.pid)
                if broken_stdout:
                    process.stdout.close()
                    process.stdout = None  # communicate must not read the closed end.
                process.communicate(input=input_bytes, timeout=timeout)
            except subprocess.TimeoutExpired:
                metadata['timedOut'] = True
                if worker:
                    # An exact owned isolated compiler has no Core/author work.
                    process.kill()
                    process.communicate(timeout=15)
                    metadata['isolatedWorkerTerminated'] = True
                else:
                    self.pending.append({'process': process, 'identity': metadata.get('identity'),
                                         'label': label})
                raise
            except BaseException:
                if process.poll() is None:
                    self.pending.append({'process': process, 'identity': metadata.get('identity'),
                                         'label': label})
                raise
            finally:
                metadata['exitCode'] = process.poll()
                metadata['elapsedSeconds'] = time.monotonic() - start
                metadata['completedAt'] = utc()
                keep_json(self.output / (stem + '.process.json'), metadata)
        data, errors = read_small(out), read_small(err)
        allowed = (expected,) if isinstance(expected, int) else tuple(expected)
        require(process.returncode in allowed,
                f'{label}: exit {process.returncode}, expected {allowed}; see {err.name}')
        return data, errors, metadata

    def cli(self, label, arguments, *, profile=None, expected=0, input_bytes=None,
            broken_stdout=False, env=None):
        command = [str(self.args.cli_executable)]
        if profile is not None:
            command += ['--data-dir', str(profile)]
        command += [str(arg) for arg in arguments]
        data, errors, metadata = self.run_process(label, command, expected=expected,
            input_bytes=input_bytes, broken_stdout=broken_stdout, env=env)
        if broken_stdout:
            require(metadata['exitCode'] == 7, 'Broken output must remain a delivery failure')
            return None
        if data or metadata['exitCode'] in (0, 8):
            value = unique_json(data)
            require(isinstance(value, dict) and value.get('schemaVersion') == 1,
                    'CLI stdout is not one complete versioned JSON result')
            return value
        diagnostics = [unique_json(line) for line in errors.splitlines() if line.strip()]
        require(any(isinstance(line, dict) and line.get('schemaVersion') == 1
                    and line.get('error', {}).get('exitCode') == metadata['exitCode']
                    for line in diagnostics), 'No matching machine error diagnostic for a local CLI failure')
        return {'exitCode': metadata['exitCode'], 'stdoutBytes': len(data),
                'stderrBytes': len(errors)}

    def capabilities(self, profile, label):
        self.attempted_profiles.add(profile)
        value = self.cli(label, ['capabilities'], profile=profile)
        self.register_owner(profile, value)
        return value

    def register_owner(self, profile, value):
        owner = value.get('ownerIdentity')
        require(isinstance(owner, dict)
                and set(owner) == {'pid', 'created', 'incarnation', 'profileIdentity', 'role'},
                'No complete typed actual OwnerIdentity')
        require(isinstance(owner['pid'], int) and owner['pid'] > 0
                and isinstance(owner['created'], int) and owner['created'] > 0,
                'Invalid process identity')
        require(str(uuid.UUID(owner['incarnation'])) == owner['incarnation'], 'Invalid owner incarnation')
        require(owner['role'] in ('local', 'hosted'), 'Unknown owner role')
        if owner_key(owner) in self.shutdown_attempted:
            raise Nonrun('Shutdown was already sent to this exact owner; no automatic control retry')
        if 'target' in value:
            require(value['target'] == {'mode': 'local', 'profileIdentity': owner['profileIdentity']},
                    'CLI target and actual owner profile differ')
        handle, actual = self.api.open_owner(owner, self.args.gui_executable)
        key = str(profile)
        if key in self.owners:
            self.api.k.CloseHandle(handle)
            require(self.owners[key]['owner'] == owner, 'CLI started another Core owner')
        else:
            self.owners[key] = {'owner': owner, 'handle': handle, 'actual': actual, 'profile': profile}

    def setup(self):
        self.api = Win32()
        gui, cli = pe_identity(self.args.gui_executable), pe_identity(self.args.cli_executable)
        self.report['artifacts'] = {'gui': gui, 'cli': cli}
        require(all(item['machine'] == 0x8664 and item['optionalMagic'] == 0x20b
                    for item in (gui, cli)), 'Role artifacts are not the qualified AMD64 PE32+ pair')
        require(gui['subsystem'] == 2, 'GUI role PE is not IMAGE_SUBSYSTEM_WINDOWS_GUI')
        require(cli['subsystem'] == 3, 'CLI role PE is not IMAGE_SUBSYSTEM_WINDOWS_CUI')
        require(gui['path'] != cli['path'] and gui['sha256'] != cli['sha256'], 'Role artifacts are not distinct')
        self.profile = self.test / 'profile-a'
        self.library = self.test / 'TEST-library'
        self.profile_b = self.test / 'profile-b'
        return self.report['artifacts']

    def workers(self):
        source = '= 中文标题\n\n正文 *强调* 😀\n\n$ x^2 $\n'
        module = (SOURCE_ROOT / 'crates/weftext-export/assets/weftext-endnotes.typ').read_bytes()
        style = (SOURCE_ROOT / 'crates/weftext-engine/resources/default-style.typ').read_bytes().decode('utf-8')
        note_source = '#metadata((id:"8f3fe32e-e7ce-47f7-a731-dda1eb15480f",types:())) <weftext>\n正文'
        results = []
        for role, executable in [('gui', self.args.gui_executable), ('cli', self.args.cli_executable)]:
            root = self.test / ('worker-' + role)
            root.mkdir()
            request = {'source': source, 'path': str(root / 'render.typ'), 'root': str(root),
                       'paged': True, 'frozen_files': {}}
            environment = dict(os.environ)
            # A mistakenly entered user launch must stay detectable in this
            # owned root, rather than touching the user's ordinary profile.
            environment['LOCALAPPDATA'] = str(root / 'UNEXPECTED-user-profile')
            observed = next(item for item in self.report['workerObservations']
                            if item['role'] == role and item['worker'] == 'render')
            observed.update(status='FAIL', startedAt=utc())
            data, _, _ = self.run_process(role + '-render-worker', [str(executable), '--render-worker'],
                                         input_bytes=wire(request), timeout=60, worker=True,
                                         env=environment)
            rendered = unique_json(data)
            require(rendered.get('ok') is True and rendered.get('pageCount', 0) > 0,
                    'Actual frozen render did not compile')
            pdf = base64.b64decode(rendered['pdfBase64'], validate=True)
            require(pdf.startswith(b'%PDF-') and b'%%EOF' in pdf[-128:], 'No complete actual PDF')
            self.artifact(role + '-worker.pdf', pdf)
            observed.update(status='PASS', completedAt=utc(), pdfSha256=digest(pdf))
            size, units = len(note_source.encode('utf-8')), len(note_source.encode('utf-16-le')) // 2
            notes = {'input': {'source': note_source, 'path': str(root / 'note.typ'),
                     'root': str(root), 'paged': False,
                     'frozen_files': {'note.typ': base64.b64encode(note_source.encode('utf-8')).decode('ascii'),
                                      'weftext-endnotes.typ': base64.b64encode(module).decode('ascii')},
                     'style': {'path': 'default-style.typ', 'source': style,
                               'enabled': True, 'entryFunction': 'library-style'}},
                     'request': {'sourceHash': digest(note_source.encode('utf-8')),
                     'intent': {'kind': 'nativeNote', 'action': {'kind': 'insertEndnote',
                         'at': {'from': size, 'to': size, 'utf16From': units, 'utf16To': units,
                                'raw': '', 'writable': True},
                         'body': {'mode': 'source', 'value': '中文 *强调*\n\n第二段'}}}}}
            observed = next(item for item in self.report['workerObservations']
                            if item['role'] == role and item['worker'] == 'notes')
            observed.update(status='FAIL', startedAt=utc())
            data, _, _ = self.run_process(role + '-notes-worker', [str(executable), '--notes-edit-worker'],
                                         input_bytes=wire(notes), timeout=60, worker=True,
                                         env=environment)
            value = unique_json(data)
            edit = value['edit']
            require(edit['sourceHash'] == digest(edit['source'].encode('utf-8')), 'Notes hash differs')
            definitions = [entry for entry in edit['map']['notes']['entries']
                           if entry['kind'] == 'endnote-definition']
            require(len(definitions) == 1 and definitions[0]['writable'] is True,
                    'No unique compiled writable endnote source')
            require('#box[#endnote(id:' in edit['source'], 'No actual endnote insertion')
            patched = note_source.encode('utf-8')
            for patch in reversed(edit['patches']):
                require(patched[patch['from']:patch['to']].decode('utf-8') == patch['expected'],
                        'Notes patch did not address the original source')
                patched = patched[:patch['from']] + patch['insert'].encode('utf-8') + patched[patch['to']:]
            require(patched.decode('utf-8') == edit['source'], 'Notes patches did not reproduce exact output')
            require(any(item['path'] == 'weftext-endnotes.typ' and item['status'] == 'loaded'
                        for item in value['dependencies']), 'Notes module was not an actual loaded dependency')
            require(not list(root.iterdir()), 'Isolated workers unexpectedly wrote files/profile state')
            observed.update(status='PASS', completedAt=utc(), notesSourceSha256=edit['sourceHash'])
            results.append({'role': role, 'renderPdfSha256': digest(pdf),
                            'notesSourceSha256': edit['sourceHash'], 'moduleSha256': digest(module)})
        return results

    def owner_start(self):
        self.cap = self.capabilities(self.profile, 'owner-capabilities')
        return self.cap['ownerIdentity']

    def create_and_write(self):
        options = self.json_file('library-options.json', {'initialRoot': {'title': 'TEST 首页', 'queries': ['notes']}})
        library_request = self.output / 'library.weftext-request.json'
        created = self.cli('library-create', ['library', 'create', '--path', self.library,
            '--options', options, '--request-file', library_request], profile=self.profile)
        require(created['result']['root'] == str(self.library), 'Wrong actual library location')
        request = self.output / 'note-create.weftext-request.json'
        created = self.cli('note-create', ['note', 'create', '--root', self.library,
            '--parent', self.library, '--name', 'TEST-中文😀', '--request-file', request], profile=self.profile)
        self.note = created['result']['document']
        self.note_path = Path(self.note['path'])
        before = self.cli('note-read-basis', ['note', 'read', '--path', self.note_path,
                          '--root', self.library], profile=self.profile)
        basis = self.json_file('note-basis.json', before)
        self.saved_source = before['result']['source'].split('\n', 1)[0] + '\r\n\r\n中文 CRLF 🌿😀\r\n= 标题\r\n正文 *强调*\r\n'
        self.artifact('literal-source.typ', self.saved_source.encode('utf-8'))
        self.save_request = self.output / 'save.weftext-request.json'
        saved = self.cli('note-write', ['note', 'write', '--basis', basis, '--source', '-',
            '--request-file', self.save_request], profile=self.profile,
            input_bytes=self.saved_source.encode('utf-8'))
        require(saved['result']['status'] in ('saved', 'committed'), 'Save was not committed')
        require(self.note_path.read_bytes() == self.saved_source.encode('utf-8'), 'Literal source bytes changed')
        frozen = unique_json(self.save_request.read_bytes())
        require(frozen['operationId'] == saved['result']['operationId'], 'Original saved ID differs')
        fact = self.cli('save-original-status', ['operation', 'inspect', '--request', self.save_request],
                        profile=self.profile)
        require(fact['result']['status'] == 'committed'
                and fact['result']['operationId'] == frozen['operationId'], 'Original save not recovered')
        self.capabilities(self.profile, 'same-owner-after-business')
        return {'nodeId': self.note['nodeId'], 'sourceSha256': digest(self.note_path.read_bytes()),
                'originalOperationId': frozen['operationId']}

    def query_and_export(self):
        catalog = self.cli('query-catalog', ['query', 'catalog', '--root', self.library], profile=self.profile)
        require(isinstance(catalog['result'], dict), 'No actual schema catalog')
        query = self.json_file('query.json', {'root': str(self.library), 'plan': {
            'scope': {'descendants': True, 'lifecycle': 'live'}, 'types': [], 'select': [], 'order': []}})
        value = self.cli('query-run', ['query', 'run', '--input', query], profile=self.profile)
        require(value['result']['coverage'] == 'complete' and value['result']['rows'], 'No complete real Query rows')
        basis = self.json_file('export-basis.json', self.cli('export-note-read',
            ['note', 'read', '--path', self.note_path, '--root', self.library], profile=self.profile))
        request = self.output / 'export-source.weftext-request.json'
        prepared = self.cli('export-source-prepare', ['export', 'prepare', '--basis', basis,
            '--kind', 'source', '--request-file', request], profile=self.profile)
        result = prepared['result']
        require(result['status'] == 'prepared' and result['length'] > 0, 'No actual prepared source artifact')
        confirmed = self.cli('export-source-confirm', ['export', 'confirm', '--request', request,
            '--digest', result['digest'], '--request-file', self.output / 'export-confirm.weftext-request.json'],
            profile=self.profile)
        require(confirmed['result']['status'] == 'confirmed', 'Artifact was not explicitly confirmed')
        target = self.output / 'downloaded-source.typ'
        downloaded = self.cli('export-source-download', ['export', 'download', '--request', request,
            '--destination', target], profile=self.profile)
        require(target.read_bytes() == self.saved_source.encode('utf-8'), 'Downloaded bytes differ from confirmed source')
        require(downloaded['result']['sha256'] == result['digest'] == digest(target.read_bytes()),
                'Confirmed/delivered digest differs')
        return {'queryRows': len(value['result']['rows']), 'artifactSha256': result['digest']}

    def bibliography_import(self):
        # Current product fixture: scripts/test_outputs.py bibliography import.
        # Inspect is read-only; commit applies the exact original preview/plan.
        source_before = self.note_path.read_bytes()
        original = self.artifact('cli-import-original.bib',
            b'@article{external, title={BIB imported}, author={Alpha One and Beta Two}, '
            b'year={2026}, journal={Journal}, doi={10.1234/bib}}\n')
        original_sha = file_digest(original)
        inspected = self.cli('bibliography-import-inspect',
            ['import', 'inspect', '--file', original, '--kind', 'bibtex'], profile=self.profile)
        require(inspected['result']['records'], 'Actual bibliography inspection returned no records')
        basis = self.json_file('bibliography-import-basis.json', self.cli('bibliography-import-read',
            ['note', 'read', '--path', self.note_path, '--root', self.library], profile=self.profile))
        name = 'TEST-cli-imported-bibliography'
        destination = self.library / name
        request = self.output / 'bibliography-import.weftext-request.json'
        prepared = self.cli('bibliography-import-prepare', ['import', 'prepare', '--file', original,
            '--kind', 'bibtex', '--basis', basis, '--parent', self.library, '--name', name,
            '--request-file', request], profile=self.profile)
        frozen = unique_json(request.read_bytes())
        preview = prepared['result']
        require(preview['status'] == 'prepared' and preview['kind'] == 'importBibliography'
                and preview['operationId'] == frozen['operationId'] and preview['planId'],
                'Import did not return the exact original bibliography Plan')
        require(not destination.exists() and self.note_path.read_bytes() == source_before,
                'Import preview modified authored content before explicit commit')
        command = ['import', 'commit', '--request', request, '--plan-id', preview['planId'],
                   '--request-file', self.output / 'bibliography-import-commit.weftext-request.json']
        if preview.get('losses'):
            command.append('--accept-losses')
        committed = self.cli('bibliography-import-commit', command, profile=self.profile)
        require(committed['result']['operationId'] == frozen['operationId']
                and committed['result']['status'] == 'committed', 'Original import was not committed')
        status = self.cli('bibliography-import-original-status',
            ['import', 'status', '--request', request], profile=self.profile)
        require(status['result']['operationId'] == frozen['operationId']
                and status['result']['status'] == 'committed', 'Import status lost the original commit')
        require(destination.is_dir() and self.note_path.read_bytes() == source_before
                and file_digest(original) == original_sha,
                'Import did not preserve the upload and existing author source')
        return {'operationId': frozen['operationId'], 'planId': preview['planId'],
                'originalUploadSha256': original_sha, 'existingSourceSha256': digest(source_before),
                'lossesExplicitlyAccepted': bool(preview.get('losses')), 'status': 'committed'}

    def local_sync(self):
        source_before = self.note_path.read_bytes()
        binding = self.cli('sync-binding-before-enable',
            ['sync', 'binding', '--root', self.library], profile=self.profile)
        require(binding['result'] is None, 'TEST library unexpectedly already has a sync pairing')
        enable_request = self.output / 'sync-enable.weftext-request.json'
        enabled = self.cli('sync-enable', ['sync', 'enable', '--root', self.library,
            '--request-file', enable_request], profile=self.profile)
        identity = enabled['result']['identity']
        require(set(identity) == {'workspaceId', 'genesis'}, 'No exact paired SyncIdentity')
        observed = self.cli('sync-binding-after-enable',
            ['sync', 'binding', '--root', self.library], profile=self.profile)
        require(observed['result']['identity'] == identity, 'Pairing changed across CLI commands')
        selector = self.json_file('sync-identity-input.json', {'identity': identity})
        status = self.cli('sync-status', ['sync', 'status', '--input', selector], profile=self.profile)
        require(status['result']['identity'] == identity and status['result']['objects'],
                'No real initial portable sync objects')
        published = self.cli('sync-publish', ['sync', 'publish', '--root', self.library,
            '--request-file', self.output / 'sync-publish.weftext-request.json'], profile=self.profile)
        require(published['result']['identity'] == identity, 'Publication belongs to another pairing')
        portable = self.test / 'TEST-portable-sync-records'
        export_input = self.json_file('sync-export-input.json', {
            'root': str(self.library), 'directory': str(portable),
            'objects': [item['object'] for item in published['result']['objects']]})
        exported = self.cli('sync-export-portable', ['sync', 'export', '--input', export_input,
            '--request-file', self.output / 'sync-export.weftext-request.json'], profile=self.profile)
        require(exported['result']['identity'] == identity and exported['result']['manifests'] > 0
                and unique_json((portable / 'identity.json').read_bytes()) == identity,
                'No actual immutable portable sync transfer was exported')
        records_before = {str(path.relative_to(portable)): file_digest(path)
                          for path in portable.rglob('*') if path.is_file()}
        receive_input = self.json_file('sync-receive-input.json', {
            'identity': identity, 'directory': str(portable)})
        received = self.cli('sync-receive-portable', ['sync', 'receive', '--input', receive_input,
            '--request-file', self.output / 'sync-receive.weftext-request.json'], profile=self.profile)
        require(received['result']['identity'] == identity, 'Receive changed the explicit pairing')
        require(self.note_path.read_bytes() == source_before
                and records_before == {str(path.relative_to(portable)): file_digest(path)
                                       for path in portable.rglob('*') if path.is_file()},
                'Portable transport modified author source or immutable input records')
        return {'identity': identity, 'portableDirectory': str(portable),
                'manifestCount': exported['result']['manifests'],
                'blobCount': exported['result']['blobs'], 'records': records_before,
                'enableReplyObserved': True,
                'scope': 'Observed enable reply and portable copy; no invented per-copy operation receipt'}

    def conflict(self):
        create = self.cli('conflict-note-create', ['note', 'create', '--root', self.library,
            '--parent', self.library, '--name', 'TEST-conflict', '--request-file',
            self.output / 'conflict-create.weftext-request.json'], profile=self.profile)
        path = Path(create['result']['document']['path'])
        args = ['note', 'read', '--path', path, '--root', self.library]
        a = self.json_file('conflict-basis-a.json', self.cli('conflict-read-a', args, profile=self.profile))
        b_value = self.cli('conflict-read-b', args, profile=self.profile)
        b = self.json_file('conflict-basis-b.json', b_value)
        first = b_value['result']['source'] + '\nFIRST_COMMITTED_中文😀'
        second = b_value['result']['source'] + '\nSECOND_MUST_NOT_OVERWRITE'
        a_source = self.artifact('conflict-first.typ', first.encode('utf-8'))
        b_source = self.artifact('conflict-second.typ', second.encode('utf-8'))
        self.cli('conflict-first-save', ['note', 'write', '--basis', a, '--source', a_source,
            '--request-file', self.output / 'conflict-first.weftext-request.json'], profile=self.profile)
        rejected = self.cli('conflict-stale-save', ['note', 'write', '--basis', b, '--source', b_source,
            '--request-file', self.output / 'conflict-second.weftext-request.json'], profile=self.profile, expected=5)
        require(rejected['result']['error']['code'] == 'conflict',
                'Stale save did not return the real original file conflict')
        require(path.read_bytes() == first.encode('utf-8'), 'Stale writer overwrote the original')
        return {'savedSha256': digest(path.read_bytes()), 'rejectedInputSha256': digest(second.encode('utf-8'))}

    def fence(self):
        operation = uid()
        body = {'operationId': operation, 'root': str(self.library),
                'intent': {'kind': 'newNode', 'parent': str(self.library), 'name': 'TEST-fenced'}}
        # Order every object explicitly so the later CLI Value encoder has the
        # same wire under either serde_json Map feature configuration. Check the
        # actual retained late wire below rather than assuming semantic equality.
        encoded = json.dumps(body, ensure_ascii=False, separators=(',', ':'), sort_keys=True).encode('utf-8')
        body = unique_json(encoded)
        original = self.json_file('never-submitted.weftext-request.json', {
            'version': 1, 'target': self.cap['target'], 'kind': 'operationPrepare',
            'requestId': uid(), 'operationId': operation, 'wireSha256': digest(encoded),
            'wireBase64': base64.b64encode(encoded).decode('ascii')})
        missing = self.cli('unknown-before-fence', ['operation', 'inspect', '--request', original],
                           profile=self.profile, expected=6)
        require(missing['result']['operationId'] == operation and missing['result']['status'] == 'unknown',
                'A never-submitted original did not return the exact real unknown fact')
        ended = self.cli('durable-original-fence', ['operation', 'settle', '--request', original,
            '--request-file', self.output / 'fence.weftext-request.json'], profile=self.profile)
        require(ended['result']['status'] in ('resolved', 'cancelled', 'notCommitted'), 'No durable original fence')
        late = self.json_file('late-original-input.json', body)
        rejected = self.cli('late-original-rejected', ['operation', 'prepare', '--input', late,
            '--request-file', self.output / 'late-original.weftext-request.json'], profile=self.profile, expected=5)
        require(rejected['result']['error']['code'] == 'operationCancelled',
                'The delayed original was not rejected by its durable attempt fence')
        require(unique_json((self.output / 'late-original.weftext-request.json').read_bytes())['wireSha256']
                == digest(encoded), 'The delayed CLI request was not the exact original frozen wire')
        require(not (self.library / 'TEST-fenced').exists(), 'Fenced original created an author object')
        return {'operationId': operation, 'lateAuthorPathAbsent': True,
                'scope': 'Never-submitted original fence; no accepted transport reply was dropped'}

    def partial_and_budget(self):
        created = self.cli('partial-note-create', ['note', 'create', '--root', self.library,
            '--parent', self.library, '--name', 'TEST-partial', '--request-file',
            self.output / 'partial-create.weftext-request.json'], profile=self.profile)
        note = created['result']['document']
        read = self.cli('partial-note-read', ['note', 'read', '--path', note['path'], '--root', self.library],
                        profile=self.profile)
        basis = self.json_file('partial-basis.json', read)
        source = self.artifact('partial-source.typ',
            (read['result']['source'].split('\n', 1)[0] + '\n#panic("TEST deliberate compiler failure")\n').encode('utf-8'))
        self.cli('partial-note-write', ['note', 'write', '--basis', basis, '--source', source,
            '--request-file', self.output / 'partial-save.weftext-request.json'], profile=self.profile)
        query = self.json_file('partial-query.json', {'root': str(self.library), 'plan': {
            'select': [{'name': 'title', 'expr': 'title'}, {'name': 'body', 'expr': 'body'}]}})
        value = self.cli('partial-query-exit', ['query', 'run', '--input', query],
                         profile=self.profile, expected=8)
        require(value['result']['coverage'] == 'partial', 'Compiler failure was falsely complete')
        allowed = self.cli('partial-query-acknowledged', ['--allow-partial', 'query', 'run', '--input', query],
                           profile=self.profile)
        require(allowed['result']['coverage'] == 'partial', 'Partial acknowledgement changed the facts')
        oversized = self.artifact('over-budget-source.typ', b'x' * (2 * 1024 * 1024 + 1))
        request = self.output / 'over-budget.weftext-request.json'
        before = self.note_path.read_bytes()
        self.cli('budget-before-submit', ['note', 'write', '--basis', self.output / 'export-basis.json',
            '--source', oversized, '--request-file', request], profile=self.profile, expected=2)
        require(not request.exists() and self.note_path.read_bytes() == before, 'Oversized input was submitted')
        return {'coverage': value['result']['coverage'], 'oversizedSubmitted': False}

    def broken_pipe(self):
        read = self.cli('broken-pipe-note-read', ['note', 'read', '--path', self.note_path,
                        '--root', self.library], profile=self.profile)
        basis = self.json_file('broken-pipe-basis.json', read)
        source = read['result']['source'] + '\nAFTER_BROKEN_OUTPUT_中文😀\n'
        path = self.artifact('broken-pipe-source.typ', source.encode('utf-8'))
        request = self.output / 'broken-pipe.weftext-request.json'
        self.cli('real-closed-stdout', ['note', 'write', '--basis', basis, '--source', path,
            '--request-file', request], profile=self.profile, expected=7, broken_stdout=True)
        frozen = unique_json(request.read_bytes())
        fact = self.cli('broken-output-original-status', ['operation', 'inspect', '--request', request],
                        profile=self.profile)
        require(fact['result']['operationId'] == frozen['operationId']
                and fact['result']['status'] == 'committed', 'Broken output lost the original commit')
        require(self.note_path.read_bytes() == source.encode('utf-8'), 'Original commit bytes differ')
        return {'originalOperationId': frozen['operationId'], 'status': 'committed', 'deliveryExit': 7}

    def cross_profile(self):
        other = self.capabilities(self.profile_b, 'second-profile-capabilities')
        require(other['target'] != self.cap['target'], 'Independent profiles have equal identity')
        before = self.note_path.read_bytes()
        self.cli('cross-profile-original-refused', ['operation', 'inspect', '--request', self.save_request],
                 profile=self.profile_b, expected=5)
        require(self.note_path.read_bytes() == before, 'Cross-profile lookup changed author bytes')
        return {'targetMismatchRejected': True}

    def restart_original(self):
        previous = self.cap['ownerIdentity']
        original_target = self.cap['target']
        source = self.profile
        destination = self.test / 'profile-a-moved'
        self.stop_owner(str(self.profile), 'original-restart')
        # The exact original process has exited. Only this invocation's owned
        # private profile moves; author data and unrelated worktrees never do.
        source = source.resolve(strict=True)
        destination = destination.absolute()
        require(source.parent == destination.parent == self.test.resolve(strict=True)
                and source.is_relative_to(self.test) and destination.is_relative_to(self.test),
                'Profile relocation left the single owned TEST parent')
        require(source.is_dir() and not source.is_symlink()
                and not (getattr(source.lstat(), 'st_file_attributes', 0) & 0x400)
                and not destination.exists(), 'Profile relocation targets are not the owned normal/absent directories')
        os.rename(source, destination)  # One same-parent, same-volume move, no copy/delete fallback.
        self.profile = destination
        self.cap = self.capabilities(self.profile, 'restarted-owner-capabilities')
        require(self.cap['ownerIdentity'] != previous, 'Restart retained the old process incarnation')
        require(self.cap['target'] == original_target, 'A moved physical profile lost its stable target identity')
        frozen = unique_json(self.save_request.read_bytes())
        fact = self.cli('original-save-after-process-restart',
            ['operation', 'inspect', '--request', self.save_request], profile=self.profile)
        require(fact['result']['operationId'] == frozen['operationId']
                and fact['result']['status'] == 'committed', 'Restart lost the original operation')
        require(self.note_path.read_bytes() == self.saved_source.encode('utf-8'),
                'Original-result query rewrote the saved file')
        require(not source.exists(), 'Original profile path unexpectedly reappeared before explicit creation')
        recreated = self.capabilities(source, 'new-empty-original-path-profile')
        require(recreated['target'] != original_target, 'New profile at the old path inherited the moved profile identity')
        self.cli('old-request-not-new-path-journal', ['operation', 'inspect', '--request', self.save_request],
                 profile=source, expected=5)
        receipt = {'originalPath': str(source), 'movedPath': str(destination),
                   'sameOwnedParent': str(self.test), 'previousOwner': previous,
                   'currentOwner': self.cap['ownerIdentity'], 'stableTarget': self.cap['target'],
                   'newProfileAtOriginalPath': recreated['ownerIdentity'],
                   'operationId': frozen['operationId'], 'status': fact['result']['status'],
                   'originalRequestRejectedByNewProfile': True}
        self.json_file('profile-relocation.json', receipt)
        return receipt

    def shells(self):
        command = cmd_product(self.args.cli_executable, self.profile)
        shell = system_cmd()
        cmdline = f'"{shell}" /D /S /C "{command}"'
        raw, _, _ = self.run_process('cmd-script-shell', cmdline, executable=str(shell))
        require(unique_json(raw)['ownerIdentity'] == self.cap['ownerIdentity'], 'cmd used another owner')
        # Actual native stdout/stderr and exit status through anonymous pipes.
        invalid, errors, meta = self.run_process('console-invalid-arguments',
            [str(self.args.cli_executable), '--definitely-invalid-test-option'], expected=2)
        require(not invalid and errors and meta['exitCode'] == 2, 'Argument failure did not separate stdout/stderr')
        redirected = self.output / 'cmd-native-redirect.json'
        redirect_command = command + ' > ' + cmd_quote(redirected)
        require(not redirected.exists(), 'Redirection destination already exists')
        self.run_process('cmd-native-redirection', f'"{shell}" /D /S /C "{redirect_command}"',
                         executable=str(shell))
        data = read_small(redirected)
        require(not data.startswith((b'\xff\xfe', b'\xfe\xff', b'\xef\xbb\xbf')),
                'cmd native redirection changed the UTF-8 stream')
        require(unique_json(data)['ownerIdentity'] == self.cap['ownerIdentity'], 'cmd redirected the wrong result')
        failed_command = cmd_quote(self.args.cli_executable) + ' --definitely-invalid-test-option'
        raw, errors, _ = self.run_process('cmd-argument-exit-and-streams',
            f'"{shell}" /D /S /C "{failed_command}"', executable=str(shell), expected=2)
        require(not raw and errors, 'cmd did not preserve the CLI argument error streams')

        # Product --output/source-output bypass shell text transcoding entirely.
        destination, source_copy = self.output / 'product-output.json', self.output / 'product-source-output.typ'
        raw, _, _ = self.run_process('product-create-only-output', [str(self.args.cli_executable),
            '--data-dir', str(self.profile), '--output', str(destination), 'note', 'read',
            '--path', str(self.note_path), '--root', str(self.library), '--source-output', str(source_copy)])
        product = read_small(destination)
        require(not raw and product.startswith(b'{') and product.endswith(b'\n'),
                'Product --output was not one raw UTF-8 JSON artifact')
        value = unique_json(product)
        require(source_copy.read_bytes() == self.note_path.read_bytes()
                == value['result']['source'].encode('utf-8'), 'Product source output changed literal author bytes')
        self.run_process('product-output-refuses-existing', [str(self.args.cli_executable),
            '--data-dir', str(self.profile), '--output', str(destination), 'capabilities'], expected=2)
        require(read_small(destination) == product, 'Existing output was overwritten')

        results = [{'shell': 'cmd', 'status': 'PASS', 'direct': 'PASS', 'redirected': 'PASS', 'encoding': 'utf-8'}]
        self.report['shellObservations'] = results
        for kind, powershell in installed_powershells():
            if powershell is None:
                results.append({'shell': kind, 'status': 'NONRUN', 'reason': 'No installed shell executable'})
                continue
            info_script = ('[pscustomobject]@{version=$PSVersionTable.PSVersion.ToString();'
                           'edition=$PSVersionTable.PSEdition}|ConvertTo-Json -Compress')
            version, _, _ = self.run_process(kind + '-version', [str(powershell), '-NoLogo',
                '-NoProfile', '-NonInteractive', '-Command', info_script])
            info = unique_json(version)
            product_command = (f'& {ps_quote(self.args.cli_executable)} --data-dir '
                               f'{ps_quote(self.profile)} capabilities')
            prefix = '[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false);$OutputEncoding=[Console]::OutputEncoding;'
            script = prefix + product_command + '; exit $LASTEXITCODE'
            raw, _, _ = self.run_process(kind + '-script-shell', [str(powershell), '-NoLogo', '-NoProfile',
                                                        '-NonInteractive', '-Command', script])
            require(unique_json(raw)['ownerIdentity'] == self.cap['ownerIdentity'], kind + ' used another owner')
            script = (prefix + f'& {ps_quote(self.args.cli_executable)} --definitely-invalid-test-option;'
                      'exit $LASTEXITCODE')
            raw, errors, _ = self.run_process(kind + '-argument-exit-and-streams', [str(powershell),
                '-NoLogo', '-NoProfile', '-NonInteractive', '-Command', script], expected=2)
            require(not raw and errors, kind + ' did not preserve the argument failure streams')
            path = self.output / (kind + '-native-redirect.json')
            require(not path.exists(), 'Shell redirection destination already exists')
            script = prefix + product_command + f' > {ps_quote(path)}; exit $LASTEXITCODE'
            self.run_process(kind + '-native-redirection', [str(powershell), '-NoLogo', '-NoProfile',
                                                          '-NonInteractive', '-Command', script])
            shell_bytes = read_small(path)
            encoding = ('utf-16-le-bom' if shell_bytes.startswith(b'\xff\xfe') else
                        'utf-8-bom' if shell_bytes.startswith(b'\xef\xbb\xbf') else 'utf-8')
            decoded = shell_bytes.decode('utf-16' if encoding == 'utf-16-le-bom' else 'utf-8-sig')
            require(json.loads(decoded)['ownerIdentity'] == self.cap['ownerIdentity'], 'Shell redirection lost the JSON result')
            if kind == 'winps' and info['version'].split('.')[0] == '5':
                require(encoding == 'utf-16-le-bom', 'Windows PowerShell 5 redirection did not use its documented UTF-16 output')
            pipeline = (prefix + product_command
                        + ' | ConvertFrom-Json | ConvertTo-Json -Depth 32 -Compress; exit $LASTEXITCODE')
            raw, _, _ = self.run_process(kind + '-real-object-pipeline', [str(powershell), '-NoLogo',
                '-NoProfile', '-NonInteractive', '-Command', pipeline])
            require(unique_json(raw)['ownerIdentity'] == self.cap['ownerIdentity'], 'Shell pipeline lost the actual result')
            results.append({'shell': kind, 'status': 'PASS', 'version': info,
                            'nativeDecode': 'explicit UTF-8 in this owned shell',
                            'redirectedEncoding': encoding, 'redirectionIsShellTransformation': kind == 'winps',
                            'pipeline': 'actual ConvertFrom-Json/ConvertTo-Json'})
        self.report['shellObservations'] = results
        if any(item.get('status') == 'NONRUN' for item in results):
            raise Nonrun('An installed-shell mechanism was not available; see shellObservations')
        return {'shells': results, 'invalidArgumentsExit': 2,
                'productOutputSha256': digest(product), 'sourceOutputSha256': file_digest(source_copy)}

    def conpty(self):
        results = []
        self.report['interactiveObservations'] = results
        for kind, executable in [('cmd', system_cmd()), *installed_powershells()]:
            if executable is None:
                results.append({'shellKind': kind, 'status': 'NONRUN', 'reason': 'No installed shell executable'})
                continue
            try:
                result = self.interactive_shell(kind, executable)
                results.append({'shellKind': kind, 'status': 'PASS', 'detail': result})
            except Nonrun as error:
                results.append({'shellKind': kind, 'status': 'NONRUN', 'reason': str(error)})
            except Exception:
                results.append({'shellKind': kind, 'status': 'FAIL', 'error': traceback.format_exc()})
        self.report['interactiveObservations'] = results
        require(not any(item['status'] == 'FAIL' for item in results), 'A real interactive shell failed')
        if any(item['status'] == 'NONRUN' for item in results):
            raise Nonrun('A real interactive shell was not run; see interactiveObservations')
        return results

    def interactive_shell(self, kind, executable):
        # -Command initializes only this owned PowerShell's prompt/history
        # behavior. Every product command is subsequently sent through ConPTY
        # input, after an independently observed ready marker.
        environment = dict(os.environ)
        shell_home = self.test / ('interactive-' + kind + '-profile')
        shell_home.mkdir()
        for variable in ('APPDATA', 'LOCALAPPDATA', 'USERPROFILE', 'HOME'):
            environment[variable] = str(shell_home)
        environment.update(POWERSHELL_UPDATECHECK='Off', POWERSHELL_TELEMETRY_OPTOUT='1')
        initialization = ("Import-Module PSReadLine -ErrorAction SilentlyContinue;"
                          "if(Get-Module PSReadLine){Set-PSReadLineOption -HistorySaveStyle SaveNothing "
                          f"-HistorySavePath {ps_quote(shell_home / 'history.txt')};Remove-Module PSReadLine}};"
                          "function global:prompt{'WEFTEXT_TEST> '}")
        arguments = ('/D', '/Q', '/K') if kind == 'cmd' else (
            '-NoLogo', '-NoProfile', '-NoExit', '-Command', initialization)
        shell = Pseudoconsole(self.api, executable, self.test,
                              self.output / ('interactive-' + kind + '.conpty.bin'), arguments, environment)
        receipt = {'source': CONPTY_SOURCE, 'shell': shell.identity, 'arguments': list(arguments)}
        try:
            ready = 'WEFTEXT_READY_' + uuid.uuid4().hex
            if kind == 'cmd':
                shell.send('chcp 65001\r\n')
                shell.send(f'echo {ready}:0\r\n')
                command = cmd_product(self.args.cli_executable, self.profile)
            else:
                shell.send('[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false);'
                           '$OutputEncoding=[Console]::OutputEncoding\r\n')
                shell.send(f"Write-Output '{ready}:0'\r\n")
                command = (f'& {ps_quote(self.args.cli_executable)} --data-dir '
                           f'{ps_quote(self.profile)} capabilities')
            shell.wait_marker(ready)
            before = len(shell.data)
            marker = 'WEFTEXT_AFTER_CLI_' + uuid.uuid4().hex
            shell.send(command + '\r\n')
            shell.send(f'echo {marker}:%ERRORLEVEL%\r\n' if kind == 'cmd' else
                       f"Write-Output ('{marker}:' + $LASTEXITCODE)\r\n")
            clean, found = shell.wait_marker(marker)
            require(found.group(1) == b'0', 'Interactive cmd observed a failing CLI exit')
            # Locate a complete JSON result before the following shell command's
            # output. The shell command line itself is not a completion witness.
            decoder = json.JSONDecoder()
            text = clean[:found.start()].decode('utf-8')
            results = []
            for opening in re.finditer(r'\{', text):
                try:
                    value, _ = decoder.raw_decode(text[opening.start():])
                    if isinstance(value, dict) and value.get('call') == 'capabilities':
                        results.append(value)
                except ValueError:
                    pass
            require(results and results[-1]['ownerIdentity'] == self.cap['ownerIdentity'],
                    'No complete CLI result preceded the real interactive shell continuation')
            failed = 'WEFTEXT_AFTER_INVALID_' + uuid.uuid4().hex
            invalid = (cmd_quote(self.args.cli_executable) + ' --definitely-invalid-test-option' if kind == 'cmd'
                       else f'& {ps_quote(self.args.cli_executable)} --definitely-invalid-test-option')
            shell.send(invalid + '\r\n')
            shell.send(f'echo {failed}:%ERRORLEVEL%\r\n' if kind == 'cmd' else
                       f"Write-Output ('{failed}:' + $LASTEXITCODE)\r\n")
            _, invalid_result = shell.wait_marker(failed)
            require(invalid_result.group(1) == b'2', 'Interactive shell did not wait for argument exit 2')
            receipt.update(marker=marker, markerExit=0, transcriptBytes=len(shell.data),
                           initialTranscriptBytes=before, completeResultBeforeShellContinuation=True,
                           invalidArgumentsExit=2, streams='ConPTY merges the console streams; direct pipes check separation')
        finally:
            receipt['shutdown'] = shell.close()
            keep_json(self.output / ('interactive-' + kind + '.process.json'), receipt)
        require(receipt['shutdown'].get('naturalShellExit')
                and receipt['shutdown'].get('exitCode') == 0
                and receipt['shutdown'].get('pseudoconsoleClosed')
                and receipt['shutdown'].get('outputDrained'), 'Owned ConPTY shell did not drain normally')
        return receipt

    def remote(self):
        # Do not start an accepted Core without a real shutdown contract. The
        # existing test_hosted.Server uses terminate(), which cannot qualify this
        # requested graceful-owner test. N1's role dispatcher/hosted control must
        # be integrated before adding an exact startup/control invocation here.
        raise Nonrun('Actual hosted HTTP CLI chain has no integrated typed graceful hosted-stop '
                     'contract in this harness; no hosted process or author operation was started')

    def stop_owner(self, key, label):
        # Remove this owned incarnation before sending shutdown. An unknown
        # acknowledgement must never cause an automatic second shutdown attempt.
        item = self.owners.pop(key)
        self.shutdown_attempted.add(owner_key(item['owner']))
        receipt = {'owner': item['owner'], 'verifiedProcess': item['actual'],
                   'startedAt': utc(), 'status': 'UNCONFIRMED'}
        self.report['ownerShutdown'].append(receipt)
        try:
            identity = self.json_file(label + '-owner-identity.json', item['owner'])
            value = self.cli(label + '-typed-stop', ['owner-stop', '--owner-identity', identity],
                             profile=item['profile'])
            require(value['result']['owner'] == item['owner'], 'Stop receipt belongs to a different owner')
            require(value['result']['exitCode'] == 0, 'Owner did not exit successfully')
            require(self.api.k.WaitForSingleObject(item['handle'], 1000) == 0, 'Exact owner is still live')
            receipt.update(status='PASS', receipt=value['result'],
                           independentlyObservedExitCode=self.api.exit_code(item['handle']))
            return receipt
        except Exception:
            receipt.update(status='FAIL', error=traceback.format_exc())
            raise
        finally:
            self.api.k.CloseHandle(item['handle'])
            receipt['completedAt'] = utc()

    def stop_owners(self):
        # A timed-out client can still finish qualification and start its owner.
        # Drain clients before the final existing-only scan, never after it.
        # If any remains live, retain precise failure evidence and leave owner
        # shutdown unconfirmed instead of racing that client with stop/restart.
        live = []
        for pending in self.pending:
            process = pending['process']
            observation = {'pid': process.pid, 'identity': pending['identity'], 'label': pending['label']}
            try:
                process.wait(timeout=5)
                observation['exitCode'] = process.returncode
            except Exception:
                observation['status'] = 'UNCONFIRMED'
                observation['error'] = traceback.format_exc()
                live.append(observation)
            self.report.setdefault('delayedClientDrain', []).append(observation)
        if live:
            self.report['ownerShutdown'].append({
                'status': 'UNCONFIRMED', 'reason': 'A timed-out client is still live; no shutdown race',
                'clients': live, 'owners': [item['owner'] for item in self.owners.values()]})
            return
        if self.api:
            # A capabilities failure may have happened after the owner was
            # started. This lookup is explicitly existing-only and cannot
            # create another Core while trying to recover its identity.
            for profile in sorted(self.attempted_profiles, key=str):
                if str(profile) not in self.owners:
                    try:
                        value = self.cli('existing-owner-after-failure', ['owner-status'],
                                         profile=profile, expected=(0, 3))
                        if 'ownerIdentity' in value:
                            self.register_owner(profile, value)
                        else:
                            self.report.setdefault('ownerDiscovery', []).append({
                                'profile': str(profile), 'status': 'UNAVAILABLE', 'reply': value})
                    except BaseException:
                        self.report.setdefault('ownerDiscovery', []).append({
                            'profile': str(profile), 'status': 'UNCONFIRMED',
                            'error': traceback.format_exc()})
        for index, key in enumerate(reversed(list(self.owners))):
            try:
                self.stop_owner(key, f'final-owner-{index}')
            except Exception:
                pass  # The exact failure and identity remain in ownerShutdown.

    def run(self):
        try:
            self.case('artifact-roles', self.setup)
            self.case('dual-pe-frozen-workers', self.workers, ['artifact-roles'])
            self.case('actual-local-owner', self.owner_start, ['artifact-roles'])
            self.case('library-read-literal-save-original', self.create_and_write, ['actual-local-owner'])
            business = ['library-read-literal-save-original']
            self.case('query-confirmed-artifact-download', self.query_and_export, business)
            self.case('bibliography-import-original-plan', self.bibliography_import, business)
            self.case('local-sync-portable-records', self.local_sync, business)
            self.case('conflict-preserves-committed-source', self.conflict, business)
            self.case('unknown-original-durable-fence', self.fence, business)
            self.case('partial-result-and-input-budget', self.partial_and_budget,
                      ['query-confirmed-artifact-download'])
            self.case('cross-profile-original-rejection', self.cross_profile, business)
            self.case('original-save-after-process-restart', self.restart_original, business)
            self.case('real-broken-pipe-original-commit', self.broken_pipe,
                      ['original-save-after-process-restart'])
            self.case('actual-cmd-and-powershell', self.shells, ['original-save-after-process-restart'])
            self.case('actual-interactive-shells-conpty', self.conpty, ['original-save-after-process-restart'])
            self.case('actual-hosted-http-cli', self.remote, ['artifact-roles'])
        finally:
            try:
                self.stop_owners()
            except BaseException:
                self.report['ownerShutdown'].append({
                    'status': 'UNCONFIRMED', 'reason': 'Unexpected cleanup failure',
                    'error': traceback.format_exc()})
            self.report['completedAt'] = utc()
            self.report['passed'] = (all(case['status'] == 'PASS' for case in self.report['cases'])
                and all(owner['status'] == 'PASS' for owner in self.report['ownerShutdown'])
                and not any(item.get('status') == 'UNCONFIRMED'
                            for item in self.report.get('delayedClientDrain', [])
                            + self.report.get('ownerDiscovery', [])))
            self.report['counts'] = {status: sum(item['status'] == status for item in self.report['cases'])
                                     for status in ('PASS', 'FAIL', 'NONRUN')}
            self.report['originalMechanismMapping'] = self.mechanism_mapping()
            keep_json(self.output / 'result.json', self.report)
        return 0 if self.report['passed'] else 1

    def mechanism_mapping(self):
        """The original 5+7+7 modes, without claiming a GUI-as-CLI retest."""
        mapped = []
        header_passed = next(case for case in self.report['cases']
                             if case['name'] == 'artifact-roles')['status'] == 'PASS'
        for shell in ('cmd', 'pwsh', 'winps'):
            interactive = next((item['status'] for item in self.report.get('interactiveObservations', [])
                                if item['shellKind'] == shell), 'NONRUN')
            scripting = next((item.get('status', 'NONRUN') for item in self.report.get('shellObservations', [])
                              if item['shell'] == shell), 'NONRUN')
            modes = ['console-direct', 'gui-direct', 'gui-attach-direct',
                     'console-redirected', 'gui-redirected']
            if shell != 'cmd':
                modes += ['console-pipeline', 'gui-pipeline']
            for mode in modes:
                replacement = mode.startswith('gui-')
                proof = interactive if mode.endswith('direct') else scripting
                status = ('ROLE_REPLACED' if header_passed and proof == 'PASS' else 'NONRUN') \
                    if replacement else proof
                mapped.append({'shell': shell, 'originalMode': mode, 'status': status,
                    'currentRole': 'formal CLI / CUI 3', 'mechanismExecuted': not replacement and proof == 'PASS',
                    'evidenceCase': 'actual-interactive-shells-conpty' if mode.endswith('direct')
                                    else 'actual-cmd-and-powershell',
                    'meaning': ('GUI-as-CLI/AttachConsole is replaced by the actual separate CUI role; '
                                'native GUI windows remain N1 acceptance, outside this script')
                               if replacement else 'Actual formal CLI under the named shell mechanism'})
        return mapped


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


def owner_key(owner):
    return (owner['pid'], owner['created'], owner['incarnation'], owner['profileIdentity'], owner['role'])


def cmd_quote(value):
    # Shell tests consume known owned paths only. Reject shell expansion syntax
    # rather than assembling a command from an accidentally active path string.
    if any(char in str(value) for char in '"%!&|<>^\r\n'):
        raise Nonrun('Owned paths contain cmd expansion syntax; shell case was not executed')
    return '"' + str(value) + '"'


def cmd_product(executable, profile):
    return cmd_quote(executable) + ' --data-dir ' + cmd_quote(profile) + ' capabilities'


def absolute_path(value):
    path = Path(value)
    if not path.is_absolute():
        raise argparse.ArgumentTypeError('An explicit absolute path is required')
    return path.resolve()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--gui-executable', required=True, type=absolute_path)
    parser.add_argument('--cli-executable', required=True, type=absolute_path)
    parser.add_argument('--test-root', required=True, type=absolute_path)
    parser.add_argument('--output-dir', required=True, type=absolute_path)
    args = parser.parse_args()
    for path in (args.gui_executable, args.cli_executable):
        if not path.is_file():
            parser.error('The supplied actual role artifact is not a file: ' + str(path))
    roots = (args.test_root, args.output_dir)
    if roots[0] == roots[1] or any(a in b.parents for a, b in (roots, tuple(reversed(roots)))):
        parser.error('TEST and output leaves must be distinct, non-nested directories')
    for path in roots:
        if path.exists() or not path.parent.is_dir():
            parser.error('Each leaf must be absent under an existing parent: ' + str(path))
    # Preserve both leaves, including partial/failure evidence. No temp cleanup.
    args.test_root.mkdir(exist_ok=False)
    args.output_dir.mkdir(exist_ok=False)
    return Harness(args).run()


if __name__ == '__main__':
    raise SystemExit(main())
