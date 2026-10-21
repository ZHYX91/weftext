"""Owned HTTP-test console processes, exact Ready and typed owner shutdown."""
import json
import os
from pathlib import Path
import queue
import re
import subprocess
import threading
import uuid
from urllib.parse import parse_qs, urlsplit

from package_inputs import parse_json

MAX_MESSAGE = 16 * 1024
MAX_CAPTURE = 16 * 1024 * 1024


def serve_command(executable, arguments):
    if not Path(executable).is_absolute():
        raise ValueError('The actual CLI executable must be an absolute path')
    arguments = [str(value) for value in arguments]
    if '--no-browser' in arguments or arguments[:1] == ['serve']:
        raise ValueError('Pass only formal console serve options, without a legacy GUI flag or subcommand')
    if any(value == '--root' or value.startswith('--root=') for value in arguments) and not any(value == '--open' or value.startswith('--open=') for value in arguments):
        raise ValueError('serve --root requires an explicit --open initial document')
    hosted = any(value == '--server-config' or value.startswith('--server-config=') for value in arguments)
    if not hosted and not any(value == '--port' or value.startswith('--port=') for value in arguments):
        arguments.extend(['--port', '0'])
    return [str(executable), 'serve', *arguments]


def _closed(value, fields, label):
    if not isinstance(value, dict) or set(value) != set(fields):
        raise ValueError(f'{label} has missing or unknown fields')


def _identity(value, *, pid=None, hosted=None):
    _closed(value, ('pid', 'created', 'incarnation', 'profileIdentity', 'role'), 'Owner identity')
    if type(value['pid']) is not int or not 0 < value['pid'] <= 0xffffffff:
        raise ValueError('Owner PID is not a Windows process ID')
    if type(value['created']) is not int or not 0 <= value['created'] <= 0xffffffffffffffff:
        raise ValueError('Owner creation time is not an exact FILETIME u64')
    for key in ('incarnation', 'profileIdentity'):
        if not isinstance(value[key], str) or not 0 < len(value[key]) <= 4096:
            raise ValueError(f'Owner {key} is not an opaque identity string')
    if value['role'] not in ('local', 'hosted'):
        raise ValueError('Unknown owner role')
    if pid is not None and value['pid'] != pid:
        raise ValueError('Ready belongs to another process, not the owned serve child')
    if hosted is not None and value['role'] != ('hosted' if hosted else 'local'):
        raise ValueError('Ready owner role differs from this test launch')
    return value


def parse_ready(data, pid, hosted=False):
    if len(data) > MAX_MESSAGE:
        raise ValueError('Ready exceeds its bounded message size')
    value = parse_json(data)
    _closed(value, ('schemaVersion', 'kind', 'url', 'ownerIdentity'), 'Serve Ready')
    if type(value['schemaVersion']) is not int or value['schemaVersion'] != 1 or value['kind'] != 'ready':
        raise ValueError('Unsupported Serve Ready schema/kind')
    identity = _identity(value['ownerIdentity'], pid=pid, hosted=hosted)
    if not isinstance(value['url'], str):
        raise ValueError('Ready URL is not a string')
    parsed = urlsplit(value['url'])
    if (parsed.scheme != 'http' or not parsed.hostname or parsed.username is not None or parsed.password is not None
            or parsed.path not in ('', '/') or parsed.query or not parsed.port):
        raise ValueError('Ready does not contain the actual HTTP listener root')
    if not hosted and parsed.hostname != '127.0.0.1':
        raise ValueError('Local Ready differs from the loopback LocalService origin')
    base = f'{parsed.scheme}://{parsed.netloc}'
    token = None
    if hosted:
        if parsed.fragment:
            raise ValueError('Hosted Ready must not expose an operator secret')
    else:
        fragment = parse_qs(parsed.fragment, keep_blank_values=True, strict_parsing=True)
        if set(fragment) != {'token'} or len(fragment['token']) != 1:
            raise ValueError('Local Ready does not carry exactly one browser credential')
        token = fragment['token'][0]
        if not re.fullmatch('[0-9a-f]{64}', token):
            raise ValueError('Local browser credential differs from LocalService')
    return {'url': value['url'], 'base': base, 'token': token, 'ownerIdentity': identity}


def parse_shutdown(data, identity):
    if len(data) > MAX_MESSAGE:
        raise ValueError('Owner-stop receipt exceeds its bounded message size')
    value = parse_json(data)
    _closed(value, ('schemaVersion', 'call', 'result'), 'Owner-stop response')
    if type(value['schemaVersion']) is not int or value['schemaVersion'] != 1 or value['call'] != 'ownerStop':
        raise ValueError('Unsupported owner-stop response')
    _closed(value['result'], ('owner', 'exitCode'), 'Shutdown receipt')
    if _identity(value['result']['owner']) != identity:
        raise ValueError('Shutdown receipt belongs to another owner')
    if type(value['result']['exitCode']) is not int or value['result']['exitCode'] != 0:
        raise ValueError('Owner did not exit successfully after drain')
    return value['result']


def _option(arguments, name):
    values = []
    for index, argument in enumerate(arguments):
        if argument == name:
            if index + 1 == len(arguments):
                raise ValueError(f'Missing {name} value')
            values.append(arguments[index + 1])
        elif argument.startswith(name + '='):
            values.append(argument[len(name) + 1:])
    if len(values) != 1:
        raise ValueError(f'An owned serve launch requires exactly one {name}')
    return values[0]


def _creation_time(process):
    if os.name != 'nt':
        return None
    import ctypes
    from ctypes import wintypes
    # CPython retains CreateProcess's original hp as Popen._handle. Query that
    # held handle, never reopen a process selected only by its numeric PID.
    function = ctypes.WinDLL('kernel32.dll', use_last_error=True, winmode=0x800).GetProcessTimes
    function.argtypes = [wintypes.HANDLE, *([ctypes.POINTER(wintypes.FILETIME)] * 4)]
    function.restype = wintypes.BOOL
    times = [wintypes.FILETIME() for _ in range(4)]
    if not function(int(process._handle), *(ctypes.byref(value) for value in times)):
        raise ctypes.WinError(ctypes.get_last_error())
    return times[0].dwLowDateTime | (times[0].dwHighDateTime << 32)


class OwnedServe:
    """A controller around a real Popen, not a replacement for Popen's API.

    Raw streams and control receipts stay in an owned create-new evidence
    directory. A failed/unknown shutdown is cached and is never retried or
    replaced with TerminateProcess. Such failures explicitly retain the PID.
    """
    def __init__(self, executable, arguments, *, hosted=False, log_prefix=None):
        self.command = serve_command(executable, arguments)
        self.data_dir = Path(_option(self.command[2:], '--data-dir'))
        if not self.data_dir.is_absolute():
            raise ValueError('The owned TEST data directory must be absolute')
        self.directory = self.data_dir.parent / f'server-control-{uuid.uuid4()}'
        self.directory.mkdir(exist_ok=False)
        self.log_prefix = None if log_prefix is None else Path(log_prefix)
        self.hosted, self.ready, self._owner_identity = hosted, None, None
        self._lines, self._streams = queue.Queue(), {'stdout': bytearray(), 'stderr': bytearray()}
        self._stream_errors, self._stop_attempted, self._stop_error = [], False, None
        self._startup_cleanup_attempted = False
        self._threads, self._ready_bytes, self._logs_written = [], 0, False
        self._first_stdout, self._extra_stdout = True, False
        self.record = {'command': self.command, 'hosted': hosted, 'ready': False, 'shutdownAttempted': False}
        self._save()
        self.process = subprocess.Popen(self.command, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                        stderr=subprocess.PIPE, creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        self.record['pid'] = self.process.pid
        try:
            self.created = _creation_time(self.process)
            self.record['created'] = self.created
        except (OSError, AttributeError, TypeError) as error:
            self.created = None
            self.record['creationQueryError'] = repr(error)
        self._save()
        for name in ('stdout', 'stderr'):
            thread = threading.Thread(target=self._read, args=(name,), daemon=True)
            thread.start()
            self._threads.append(thread)

    def _save(self):
        (self.directory / 'lifecycle.json').write_text(json.dumps(self.record, indent=2), encoding='utf-8')

    def _read(self, name):
        stream = getattr(self.process, name)
        try:
            with (self.directory / f'serve.{name}.bin').open('xb') as output:
                while True:
                    chunk = stream.readline(MAX_MESSAGE + 1) if name == 'stdout' else stream.read1(65536)
                    if not chunk:
                        break
                    output.write(chunk)
                    output.flush()
                    if len(self._streams[name]) + len(chunk) <= MAX_CAPTURE:
                        self._streams[name].extend(chunk)
                    elif not any(message.startswith(name + ':') for message in self._stream_errors):
                        self._stream_errors.append(name + ': captured output exceeds limit; raw file retained')
                    if name == 'stdout':
                        if self._first_stdout:
                            self._lines.put(chunk)
                            self._first_stdout = False
                        else:
                            self._extra_stdout = True
        except BaseException as error:
            self._stream_errors.append(f'{name}: {error!r}')
        finally:
            stream.close()
            if name == 'stdout':
                self._lines.put(None)

    def wait_ready(self, timeout=45):
        try:
            line = self._lines.get(timeout=timeout)
            if line is None:
                raise RuntimeError('Owned serve child exited without Ready')
            if not line.endswith(b'\n'):
                raise ValueError('Ready is not a complete JSON line')
            self.ready = parse_ready(line, self.process.pid, self.hosted)
            if self.created is not None and self.ready['ownerIdentity']['created'] != self.created:
                raise ValueError('Ready creation FILETIME differs from the original owned OS handle')
            self._owner_identity = self.ready['ownerIdentity']
            self._ready_bytes = len(line)
            self.record['ready'] = True
            self.record['ownerIdentity'] = self.ready['ownerIdentity']
            self._save()
            return self
        except BaseException as error:
            if (self.process.poll() is None and self.created is not None
                    and not self._startup_cleanup_attempted):
                self._startup_cleanup_attempted = True
                try:
                    self._cleanup_unready(timeout)
                except BaseException as cleanup_error:
                    self.record['startupCleanupError'] = repr(cleanup_error)
            self.record.update({'startupError': repr(error), 'observedExitCode': self.process.poll(),
                                'stopped': self.process.poll() is not None})
            if self.process.poll() is not None:
                self._join()
            self._save()
            self._write_logs()
            raise RuntimeError(f'Serve Ready failed; owned PID {self.process.pid}, originals: {self.directory}') from error

    def _control(self, name, arguments, timeout):
        command = [self.command[0], *arguments]
        record = {'command': command}
        self.record.setdefault('controls', {})[name] = record
        self._save()
        with (self.directory / f'{name}.stdout.bin').open('xb') as out, (self.directory / f'{name}.stderr.bin').open('xb') as err:
            control = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=out, stderr=err,
                                       creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
            record['pid'] = control.pid
            self._save()
            try:
                record['exitCode'] = control.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                record['outcome'] = 'unknown; no retry or kill'
                self._save()
                raise
        self._save()
        path = self.directory / f'{name}.stdout.bin'
        if path.stat().st_size > MAX_MESSAGE:
            raise ValueError(f'{name} output exceeds its bounded message size')
        return record['exitCode'], path.read_bytes()

    def _cleanup_unready(self, timeout):
        # N1's existing-only status qualifies this exact data-dir/profile. The
        # returned identity must also match the original owned OS handle before
        # one typed stop is permitted. Never turn a missing broker into a start.
        code, data = self._control('owner-status', ['owner-status', '--data-dir', str(self.data_dir)], timeout)
        if code != 0:
            raise RuntimeError(f'Existing-only owner-status exit {code}; no alternative cleanup')
        status = parse_json(data)
        _closed(status, ('schemaVersion', 'call', 'ownerIdentity'), 'Owner-status response')
        if type(status['schemaVersion']) is not int or status['schemaVersion'] != 1 or status['call'] != 'ownerStatus':
            raise ValueError('Unsupported owner-status response')
        identity = _identity(status['ownerIdentity'], pid=self.process.pid, hosted=self.hosted)
        if identity['created'] != self.created:
            raise ValueError('Existing owner differs from the original held process handle')
        self._owner_identity = identity
        self.record['startupCleanupIdentity'] = identity
        self.stop(control_timeout=timeout)

    def _join(self):
        for thread in self._threads:
            thread.join(timeout=2)
        if any(thread.is_alive() for thread in self._threads):
            raise RuntimeError('Owned process streams did not drain after process exit')

    def output(self):
        return (bytes(self._streams['stdout'][self._ready_bytes:]).decode('utf-8'),
                bytes(self._streams['stderr']).decode('utf-8'))

    def _write_logs(self):
        if self.log_prefix is None or self._logs_written:
            return
        for name, data in self._streams.items():
            text = bytes(data).decode('utf-8', errors='replace')
            if self.ready and self.ready['token']:
                text = text.replace(self.ready['token'], '<redacted>')
            with Path(str(self.log_prefix) + f'.{name}.log').open('a', encoding='utf-8') as output:
                output.write(text)
        self._logs_written = True

    def stop(self, control_timeout=45, exit_timeout=15):
        if self._stop_attempted:
            if self._stop_error is not None:
                raise self._stop_error
            return self.output()
        self._stop_attempted = True
        self.record['shutdownAttempted'] = True
        try:
            if self._owner_identity is None:
                raise RuntimeError('No verified Ready identity; do not guess an owner to stop')
            if self.process.poll() is not None:
                raise RuntimeError('Owned serve child exited before a typed shutdown receipt')
            identity_file = self.directory / 'owner-identity.json'
            identity_bytes = json.dumps(self._owner_identity, ensure_ascii=False, separators=(',', ':')).encode('utf-8')
            if len(identity_bytes) > MAX_MESSAGE:
                raise ValueError('Verified owner identity exceeds the CLI input limit')
            with identity_file.open('xb') as output:
                output.write(identity_bytes)
            code, data = self._control('owner-stop', ['owner-stop', '--data-dir', str(self.data_dir),
                                                    '--owner-identity', str(identity_file)], control_timeout)
            if code != 0:
                raise RuntimeError(f'owner-stop exit {code}; no retry or owner kill, outcome may be unknown')
            self.record['shutdownReceipt'] = parse_shutdown(data, self._owner_identity)
            if self.process.wait(timeout=exit_timeout) != 0:
                raise RuntimeError('Owned serve child did not exit successfully')
            self._join()
            if self._stream_errors:
                raise RuntimeError('; '.join(self._stream_errors))
            if self._extra_stdout:
                raise RuntimeError('Serve emitted additional stdout after its single Ready')
            self.record.update({'stopped': True, 'exitCode': self.process.returncode})
            return self.output()
        except BaseException as error:
            self.record.update({'shutdownError': repr(error), 'observedExitCode': self.process.poll(),
                                'stopped': self.process.poll() is not None})
            self._stop_error = RuntimeError(f'Typed shutdown failed; no retry/kill; owned PID {self.process.pid}, originals: {self.directory}')
            raise self._stop_error from error
        finally:
            self._save()
            self._write_logs()


def launch_serve(executable, arguments, *, hosted=False, log_prefix=None, ready_timeout=45):
    return OwnedServe(executable, arguments, hosted=hosted, log_prefix=log_prefix).wait_ready(ready_timeout)
