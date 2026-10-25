"""Content-qualified hints for the single product entry's shared Cargo target.

The product lock is acquired before invoking Cargo (which owns its own lock).
This is a cache hint, not acceptance evidence or a replacement for Cargo's
feature/profile/unit fingerprints. Missing, interrupted or changed hints force
only local/path package entry points; registry dependencies remain Cargo-owned.
"""
from contextlib import contextmanager
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import threading
import time
import uuid

from package_inputs import is_reparse, parse_json, read_checked, sha256, snapshot

# Compile-time inputs outside their package directory. Keep these beside the
# actual include/build-script consumers when adding another external asset.
EXTRA_INPUTS = {
    'weftext-messages': ('assets/ui-messages.json',),
    'weftext-data': ('docs/design/examples/schema.authoring.json', 'docs/design/examples/native-cases/saved-query.typ'),
    'weftext-core': ('docs/design/examples/schema.authoring.json',
                     'app/src/capture-author-wire.fixture.json'),
    'weftext-engine': ('resources/fonts', 'crates/weftext-export/assets'),
    'weftext-export': ('docs/design/examples/schema.authoring.json',),
    'weftext-app': ('app/dist', 'packaging/windows/host-contract.json',
                    'packaging/windows/application.manifest', 'packaging/windows/assets/weftext.ico',
                    'scripts/recognition/worker.py', 'scripts/recognition/engine.py',
                    'scripts/recognition/install.py', 'scripts/recognition/schemes.py',
                    'scripts/recognition/docx_reader.py', 'scripts/recognition/product_worker.py',
                    'resources/recognition/runtime-lock.json', 'resources/recognition/request.schema.json',
                    'docs/design/examples/schema.authoring.json',
                    'crates/weftext-export/assets'),
}
STATE = '.weftext-cargo-inputs.json'


def digest_json(value):
    return sha256(json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=True).encode())


def tool_identity(command, root, environment):
    version = subprocess.run([command, '-vV'], cwd=root, env=environment,
                             stdin=subprocess.DEVNULL, capture_output=True, check=True).stdout.decode()
    requested = Path(command).stem
    executable = Path(shutil.which(command, path=environment.get('PATH')) or command).resolve(strict=True)
    data = read_checked(executable, executable.parent)
    rustup = shutil.which('rustup', path=environment.get('PATH'))
    rustup = None if rustup is None else Path(rustup).resolve(strict=True)
    if rustup and sha256(data) == sha256(read_checked(rustup, rustup.parent)):
        # Qualify the pinned actual tool behind a hard-linked/symlinked shim.
        executable = Path(subprocess.run([str(rustup), 'which', requested], cwd=root, env=environment,
                                        stdin=subprocess.DEVNULL, capture_output=True, check=True).stdout.decode().strip())
        data = read_checked(executable, executable.parent)
    return {'version': version, 'bytes': len(data), 'sha256': sha256(data)}


@contextmanager
def target_lock(target):
    target.mkdir(parents=True, exist_ok=True)
    path = target / '.weftext-cargo-inputs.lock'
    if is_reparse(target.lstat()) or (path.exists() and is_reparse(path.lstat())):
        raise ValueError('Cargo input hints refuse a reparse target/lock')
    with path.open('a+b') as lock:
        if lock.seek(0, 2) == 0:
            lock.write(b'\0')
            lock.flush()
        while True:
            lock.seek(0)
            try:
                if os.name == 'nt':
                    import msvcrt
                    msvcrt.locking(lock.fileno(), msvcrt.LK_NBLCK, 1)
                else:
                    import fcntl
                    fcntl.flock(lock.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                time.sleep(.1)
            except OSError as error:
                if os.name != 'nt' or error.errno not in (13, 36):
                    raise
                time.sleep(.1)
        try:
            yield
        finally:
            lock.seek(0)
            if os.name == 'nt':
                import msvcrt
                msvcrt.locking(lock.fileno(), msvcrt.LK_UNLCK, 1)
            else:
                import fcntl
                fcntl.flock(lock.fileno(), fcntl.LOCK_UN)


def compiler_inputs(root, target, metadata, extra_inputs=EXTRA_INPUTS):
    files = snapshot(root, [target])
    if (root / 'app/dist').is_dir():
        files.update({'app/dist/' + name: data for name, data in snapshot(root / 'app/dist').items()})
    packages = {}
    for package in metadata['packages']:
        if package.get('source') is not None:
            continue
        manifest = Path(package['manifest_path']).absolute()
        key = manifest.relative_to(root).as_posix()  # Outside path dependencies are not public product input.
        folder = manifest.parent.relative_to(root).as_posix()
        entries = sorted({Path(item['src_path']).absolute() for item in package['targets']})
        for entry in entries:
            if entry.relative_to(root).as_posix() not in files:
                raise ValueError(f'Local Cargo entry is absent from selected public source: {entry}')
        packages[key] = {'id': package['id'], 'folder': folder, 'entries': entries,
                         'customBuild': any('custom-build' in item['kind'] for item in package['targets']),
                         'name': package['name'], 'files': {}}
    common = {name: sha256(data) for name, data in files.items()
              if name in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain', 'rust-toolchain.toml') or name.startswith('.cargo/')}
    for name, data in files.items():
        owners = [(len(item['folder']), key) for key, item in packages.items()
                  if item['folder'] == '.' or name.startswith(item['folder'] + '/')]
        if owners:
            packages[max(owners)[1]]['files'][name] = sha256(data)
        for item in packages.values():
            if any(name == prefix or name.startswith(prefix + '/') for prefix in extra_inputs.get(item['name'], ())):
                item['files'][name] = sha256(data)
    for item in packages.values():
        item['sha256'] = digest_json({'common': common, 'package': item.pop('files')})
    return packages


def stream_owned_process(command, root, environment, consume):
    """Do not return or raise while the exact owned child is still running."""
    process = subprocess.Popen(command, cwd=root, env=environment, stdin=subprocess.DEVNULL,
                               stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    failure = None
    deferred, previous_handler = False, None
    def remember_failure():
        nonlocal failure, deferred, previous_handler
        failure = failure or sys.exc_info()
        if not deferred and threading.current_thread() is threading.main_thread():
            previous_handler = signal.signal(signal.SIGINT, signal.SIG_IGN)
            deferred = True
    try:
        try:
            for line in process.stdout:
                consume(line)
        except BaseException:
            remember_failure()
        # Popen.__exit__ only briefly waits after KeyboardInterrupt. In
        # particular, a signal delivered only to Python need not stop Cargo.
        # Drain without invoking the failed consumer again, retaining the lock
        # and the first exception even if another interrupt arrives meanwhile.
        try:
            while True:
                try:
                    for _line in process.stdout:
                        pass
                    break
                except (KeyboardInterrupt, SystemExit):
                    remember_failure()
        except BaseException:
            remember_failure()
        finally:
            while True:
                try:
                    code = process.wait()
                    break
                except (KeyboardInterrupt, SystemExit):
                    remember_failure()
        if failure is not None:
            raise failure[1].with_traceback(failure[2])
        return code
    finally:
        try:
            process.stdout.close()
        finally:
            if deferred:
                signal.signal(signal.SIGINT, previous_handler)


class CargoCache:
    """Hold the product lock through compilation and immutable role copying."""
    def __init__(self, root, target, environment, *, extra_inputs=EXTRA_INPUTS):
        self.root, self.target = Path(root).absolute(), Path(target).absolute()
        self.environment = dict(environment)
        self.extra_inputs = extra_inputs
        self.lock = None

    def __enter__(self):
        if self.lock is not None:
            raise ValueError('Cargo product target lock is already held by this session')
        lock = target_lock(self.target)
        lock.__enter__()
        self.lock = lock
        return self

    def __exit__(self, *error):
        try:
            return self.lock.__exit__(*error)
        finally:
            self.lock = None

    def _write(self, state):
        temporary = self.target / f'{STATE}.{uuid.uuid4()}.tmp'
        with temporary.open('xb') as output:
            output.write(json.dumps(state, sort_keys=True, separators=(',', ':')).encode())
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, self.target / STATE)

    def _state(self):
        try:
            state = parse_json(read_checked(self.target / STATE, self.target, max_bytes=4 * 1024**2))
            if (set(state) == {'schemaVersion', 'pending', 'modes'} and type(state['schemaVersion']) is int
                    and state['schemaVersion'] == 1 and state['pending'] is False and isinstance(state['modes'], dict)
                    and all(isinstance(item, dict) and set(item) == {'packages'} and isinstance(item['packages'], dict)
                            for item in state['modes'].values())):
                return state
        except (OSError, ValueError, TypeError):
            pass
        return {'schemaVersion': 1, 'pending': False, 'modes': {}}

    def _witness(self, names):
        result = []
        for name in sorted(set(names)):
            path = Path(name).absolute()
            relative = path.relative_to(self.target).as_posix()
            data = read_checked(path, self.target)
            result.append({'path': relative, 'bytes': len(data), 'sha256': sha256(data)})
        return result

    def _matches(self, witnesses):
        if not isinstance(witnesses, list) or not witnesses:
            return False
        try:
            for item in witnesses:
                path = self.target / item['path']
                data = read_checked(path, self.target)
                if len(data) != item['bytes'] or sha256(data) != item['sha256']:
                    return False
            return True
        except (OSError, ValueError, TypeError, KeyError):
            return False

    def _output_tree(self, directory):
        directory = Path(directory).absolute()
        relative = directory.relative_to(self.target)
        if '..' in relative.parts:
            raise ValueError('Generated Cargo output escapes the target')
        current = self.target
        for part in relative.parts:
            current /= part
            if is_reparse(current.lstat()):
                raise ValueError('Generated Cargo output is a reparse path')
        if not directory.is_dir():
            raise ValueError('Missing actual Cargo generated-output directory')
        files, used = {}, 0
        for parent, children, names in os.walk(directory, followlinks=False):
            for name in children:
                if is_reparse((Path(parent) / name).lstat()):
                    raise ValueError('Generated Cargo output contains a reparse directory')
            for name in names:
                path = Path(parent) / name
                data = read_checked(path, self.target, max_bytes=512 * 1024**2 - used)
                used += len(data)
                if len(files) >= 50000:
                    raise ValueError('Generated Cargo output exceeds its explicit inventory budget')
                files[path.relative_to(directory).as_posix()] = {'bytes': len(data), 'sha256': sha256(data)}
        return digest_json(files)

    def _generated(self, events):
        result = []
        for event in events:
            path = Path(event['out_dir']).absolute()
            result.append({'path': path.relative_to(self.target).as_posix(), 'sha256': self._output_tree(path),
                           'declarations': digest_json({key: value for key, value in event.items()
                                                       if key not in ('reason', 'package_id', 'out_dir')})})
        return sorted(result, key=lambda item: (item['path'], item['declarations']))

    def _matches_generated(self, records, required):
        if not isinstance(records, list) or (required and not records):
            return False
        try:
            return all(self._output_tree(self.target / item['path']) == item['sha256'] for item in records)
        except (OSError, ValueError, TypeError, KeyError):
            return False

    def _invalidate_local_fingerprints(self, inputs, forced, metadata, command):
        # Cargo can retain target-relative dep-info paths into a previous frozen
        # source copy. Touching the new entry then cannot dirty that old unit.
        # Cargo explicitly supports an empty short fingerprint after a failed
        # rebuild: it compares unequal and requires compilation. Invalidate only
        # generated short hashes for the unqualified local package names, never
        # source copies, dependency outputs or the whole shared target.
        names = {inputs[key]['name'] for key in forced}
        if names.intersection(package['name'] for package in metadata['packages']
                              if package.get('source') is not None):
            raise ValueError('Cannot invalidate ambiguous local/registry package fingerprints')
        policy = re.compile(r'(?:' + '|'.join(re.escape(name) for name in sorted(names)) + r')-[0-9a-f]{16}')
        if not names:
            return
        if '..' in self.target.parts or is_reparse(self.target.lstat()):
            raise ValueError('Cargo fingerprint target is not a qualified ordinary directory')
        markers = []
        # Do not glob the target: it also owns frozen public source copies.
        # Only Cargo's selected host/target profile locations are writable.
        profile = 'debug'
        targets = []
        configured = self.environment.get('CARGO_BUILD_TARGET')
        if configured:
            targets.append(configured)
        iterator = iter(command[2:])
        explicit_target = False
        for argument in iterator:
            if argument == '--':
                break
            if argument == '--release':
                profile = 'release'
            elif argument == '--profile' or argument.startswith('--profile='):
                profile = next(iterator) if argument == '--profile' else argument.partition('=')[2]
                profile = {'dev': 'debug', 'test': 'debug', 'bench': 'release'}.get(profile, profile)
            elif argument == '--target' or argument.startswith('--target='):
                if not explicit_target:
                    targets = []
                    explicit_target = True
                targets.append(next(iterator) if argument == '--target' else argument.partition('=')[2])
        folders = [Path(profile)]
        for target in targets:
            if target.endswith('.json'):
                target = Path(target).stem
            folders.append(Path(target) / profile)
        if any(not re.fullmatch(r'[A-Za-z0-9_-]+', part) for folder in folders for part in folder.parts):
            raise ValueError('Unsupported Cargo fingerprint target/profile layout')
        roots = sorted({self.target / folder / '.fingerprint' for folder in folders})
        for root in roots:
            if not root.exists():
                continue
            current = self.target
            for part in root.relative_to(self.target).parts:
                current /= part
                if is_reparse(current.lstat()):
                    raise ValueError('Cargo fingerprint path is a reparse point')
            for directory in root.iterdir():
                if not policy.fullmatch(directory.name):
                    continue
                if is_reparse(directory.lstat()) or not directory.is_dir():
                    raise ValueError('Cargo local fingerprint directory is not an ordinary directory')
                for marker in directory.iterdir():
                    if marker.suffix or not marker.with_name(marker.name + '.json').is_file():
                        continue
                    data = read_checked(marker, self.target, max_bytes=16)
                    if data and not re.fullmatch(b'[0-9a-f]{16}', data):
                        raise ValueError('Unsupported Cargo local short fingerprint')
                    sidecar = parse_json(read_checked(marker.with_name(marker.name + '.json'), self.target,
                                                      max_bytes=1024**2))
                    if not isinstance(sidecar, dict):
                        raise ValueError('Unsupported Cargo local fingerprint sidecar')
                    info = marker.stat()
                    if info.st_nlink != 1:
                        raise ValueError('Cargo local fingerprint has multiple hard links')
                    markers.append((marker, info.st_dev, info.st_ino, data))
        # Validate every selected marker before writing any of them. Recheck the
        # exact opened file so a substituted hard link cannot truncate source.
        for marker, device, inode, data in markers:
            if read_checked(marker, self.target, max_bytes=16) != data:
                raise ValueError('Cargo local fingerprint changed before invalidation')
            with marker.open('r+b') as stream:
                opened = os.fstat(stream.fileno())
                if (opened.st_dev, opened.st_ino, opened.st_nlink) != (device, inode, 1):
                    raise ValueError('Cargo local fingerprint identity changed before invalidation')
                if os.name == 'nt':
                    import ctypes
                    import msvcrt
                    from ctypes import wintypes
                    function = ctypes.WinDLL('kernel32', use_last_error=True).GetFinalPathNameByHandleW
                    function.argtypes = [wintypes.HANDLE, wintypes.LPWSTR, wintypes.DWORD, wintypes.DWORD]
                    function.restype = wintypes.DWORD
                    buffer = ctypes.create_unicode_buffer(32768)
                    length = function(msvcrt.get_osfhandle(stream.fileno()), buffer, len(buffer), 0)
                    if not length or length >= len(buffer):
                        raise OSError(ctypes.get_last_error(), 'Cannot qualify Cargo fingerprint write handle')
                    actual = buffer.value
                    if actual.startswith('\\\\?\\UNC\\'):
                        actual = '\\\\' + actual[8:]
                    elif actual.startswith('\\\\?\\'):
                        actual = actual[4:]
                    if Path(actual) != marker:
                        raise ValueError('Cargo fingerprint write handle escaped its qualified path')
                current = self.target
                for part in (None, *marker.relative_to(self.target).parts):
                    if part is not None:
                        current /= part
                    if is_reparse(current.lstat()):
                        raise ValueError('Cargo fingerprint changed to a reparse path')
                stream.truncate(0)

    def run(self, command, *, on_message=None, output=None):
        if self.lock is None or len(command) < 2 or command[1] not in ('build', 'test', 'check', 'clippy'):
            raise ValueError('A supported compile command requires the held product target lock')
        if not any(value.startswith('--message-format=json') for value in command):
            raise ValueError('Cargo source qualification requires compiler-artifact JSON messages')
        feature_args = []
        iterator = iter(command[2:])
        for argument in iterator:
            if argument == '--':
                break
            if argument == '--features':
                feature_args.extend([argument, next(iterator)])
            elif argument.startswith('--features=') or argument in ('--all-features', '--no-default-features'):
                feature_args.append(argument)
        metadata = json.loads(subprocess.run([command[0], 'metadata', '--locked', '--format-version', '1', *feature_args],
                              cwd=self.root, env=self.environment, stdin=subprocess.DEVNULL,
                              capture_output=True, check=True).stdout)
        if Path(metadata['target_directory']).absolute() != self.target:
            raise ValueError('Cargo changed the qualified target directory')
        inputs = compiler_inputs(self.root, self.target, metadata, self.extra_inputs)
        tools = [tool_identity(tool, self.root, self.environment)
                 for tool in (command[0], self.environment.get('RUSTC', 'rustc'))]
        environment = {key: value for key, value in self.environment.items()
                       if key.startswith(('RUST', 'CARGO_BUILD_', 'CARGO_TARGET_')) and key != 'CARGO_TARGET_DIR'}
        mode = digest_json({'command': command, 'tools': tools, 'environment': environment,
                            'sourceRoot': str(self.root.resolve(strict=True))})
        state = self._state()
        previous = state['modes'].get(mode, {})
        qualified = previous.get('packages', {}) if isinstance(previous, dict) else {}
        forced = set()
        for key, item in inputs.items():
            old = qualified.get(key, {})
            if (not isinstance(old, dict) or old.get('sha256') != item['sha256']
                    or not self._matches(old.get('artifacts'))
                    or not self._matches_generated(old.get('generated'), item['customBuild'])):
                forced.add(key)
        # Invalidate success before any touch/compiler can alter shared outputs.
        # A failed or interrupted command leaves pending, never old A-qualified
        # state after partially compiling B. The next attempt is conservative.
        self._write({'schemaVersion': 1, 'pending': True, 'modes': {}})
        self._invalidate_local_fingerprints(inputs, forced, metadata, command)
        for key in forced:
            for entry in inputs[key]['entries']:
                before = read_checked(entry, self.root)
                os.utime(entry, None)
                if read_checked(entry, self.root) != before:
                    raise ValueError('Local entry changed while invalidating Cargo mtime')
        artifacts = []
        build_outputs = []
        log = None if output is None else Path(output).open('xb')
        try:
            def consume(line):
                if log:
                    log.write(line)
                    log.flush()
                try:
                    message = json.loads(line)
                except (ValueError, UnicodeError):
                    print(line.decode('utf-8', errors='replace'), end='', file=sys.stderr)
                    return
                if not isinstance(message, dict):
                    return
                if message.get('reason') == 'compiler-artifact':
                    artifacts.append(message)
                    if on_message:
                        on_message(message)
                elif message.get('reason') == 'build-script-executed':
                    build_outputs.append(message)
                elif message.get('reason') == 'compiler-message' and message.get('message', {}).get('rendered'):
                    print(message['message']['rendered'], end='', file=sys.stderr)
            code = stream_owned_process(command, self.root, self.environment, consume)
            if code:
                raise subprocess.CalledProcessError(code, command)
            current = compiler_inputs(self.root, self.target, metadata, self.extra_inputs)
            if {key: item['sha256'] for key, item in current.items()} != {key: item['sha256'] for key, item in inputs.items()}:
                raise ValueError('Compiler inputs changed during Cargo; source hint remains pending')
            successful = {}
            for key, item in inputs.items():
                messages = [message for message in artifacts if message['package_id'] == item['id']]
                if not messages:
                    continue  # An unselected local package is not qualified by this command.
                for message in messages:
                    if Path(message['target']['src_path']).absolute() not in item['entries']:
                        raise ValueError('Cargo local target differs from actual metadata entry')
                    if key in forced and message.get('fresh') is not False:
                        raise ValueError(f'Cargo reused an unqualified local unit: {key}')
                names = [name for message in messages for name in message['filenames']]
                generated = self._generated([event for event in build_outputs if event['package_id'] == item['id']])
                if item['customBuild'] and not generated:
                    raise ValueError(f'Cargo did not qualify actual local build-script outputs: {key}')
                if key not in forced and generated != qualified[key].get('generated'):
                    raise ValueError(f'Local build-script outputs changed without qualified inputs: {key}')
                successful[key] = {'sha256': item['sha256'], 'artifacts': self._witness(names), 'generated': generated}
            modes = dict(state['modes'])
            modes[mode] = {'packages': successful}
            while len(modes) > 16:
                del modes[next(iter(modes))]
            self._write({'schemaVersion': 1, 'pending': False, 'modes': modes})
            return {'artifacts': artifacts, 'forced': sorted(forced), 'mode': mode}
        finally:
            if log:
                log.close()
