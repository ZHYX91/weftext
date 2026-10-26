"""Actual owned offline Cargo regressions without third-party packages."""
import argparse
import copy
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import tomllib
import unittest
from unittest import mock
import uuid

from cargo_cache import CargoCache, EXTRA_INPUTS, STATE
import cargo_cache as cache_module
from package_inputs import snapshot

ROOT = Path(__file__).resolve().parents[1]
options, remaining = argparse.ArgumentParser(add_help=False), None
options.add_argument('--fixture-root', type=Path)
arguments, remaining = options.parse_known_args()
BASE = (arguments.fixture_root or ROOT / 'target/weftext-checks' / ('cargo-inputs-' + str(uuid.uuid4()))).absolute()
BASE.mkdir(parents=True, exist_ok=False)
OLD = 1577836800


def memory_gate():
    if os.name == 'nt':
        class Memory(ctypes.Structure):
            _fields_ = [('length', ctypes.c_ulong), ('load', ctypes.c_ulong), *[(name, ctypes.c_ulonglong) for name in ('total', 'free', 'pageTotal', 'pageFree', 'virtualTotal', 'virtualFree', 'extendedFree')]]
        value = Memory()
        value.length = ctypes.sizeof(value)
        if not ctypes.WinDLL('kernel32', use_last_error=True).GlobalMemoryStatusEx(ctypes.byref(value)):
            raise ctypes.WinError(ctypes.get_last_error())
        free = value.free
    else:
        free = os.sysconf('SC_AVPHYS_PAGES') * os.sysconf('SC_PAGE_SIZE')
    if free < 4 * 1024**3:
        raise RuntimeError(f'NONRUN: host free below 4 GiB ({free} bytes)')
    return free


class Fixture:
    def __init__(self, name, *, sources_in_target=False):
        self.root = BASE / name
        self.root.mkdir()
        self.target = self.root / 'target'
        self.sources_in_target = sources_in_target
        self.environment = {**os.environ, 'CARGO_TARGET_DIR': str(self.target), 'CARGO_NET_OFFLINE': 'true', 'CARGO_BUILD_JOBS': '1'}
        self.environment.pop('CARGO_BUILD_TARGET', None)  # This fixture executes its own host binary.
        self.records = []
        for variant in ('a', 'b'):
            source = self.source(variant)
            files = {
                'Cargo.toml': '[workspace]\nmembers=["data","app"]\nexclude=["vendor/stable"]\nresolver="2"\n[profile.dev]\ndebug=0\n',
                'rust-toolchain.toml': (ROOT / 'rust-toolchain.toml').read_text(encoding='utf-8'),
                'data/Cargo.toml': '[package]\nname="cache-fixture-data"\nversion="0.1.0"\nedition="2024"\n[features]\nextra=[]\n[dependencies]\ncache-fixture-stable={path="../vendor/stable"}\n',
                'data/src/lib.rs': self.data(variant),
                'data/build.rs': 'fn main(){println!("cargo:rerun-if-changed=build-value.txt");std::fs::write(std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("value.txt"),std::fs::read("build-value.txt").unwrap()).unwrap();}\n',
                'data/build-value.txt': 'owned-build-asset-' + variant,
                'shared/suffix.txt': '',
                'vendor/stable/Cargo.toml': '[package]\nname="cache-fixture-stable"\nversion="0.1.0"\nedition="2024"\n',
                'vendor/stable/src/lib.rs': 'pub fn value()->u32 {1}\n',
                'app/Cargo.toml': '[package]\nname="cache-fixture-app"\nversion="0.1.0"\nedition="2024"\n[features]\nextra=["cache-fixture-data/extra"]\n[dependencies]\ncache-fixture-data={path="../data"}\n',
                'app/src/main.rs': 'fn main(){println!("{}:{}",cache_fixture_data::' + ('old_symbol' if variant == 'a' else 'new_symbol') + '(),cache_fixture_data::asset());}\n',
                'Cargo.lock': '# Owned, third-party-free fixture\nversion=4\n[[package]]\nname="cache-fixture-app"\nversion="0.1.0"\ndependencies=["cache-fixture-data"]\n[[package]]\nname="cache-fixture-data"\nversion="0.1.0"\ndependencies=["cache-fixture-stable"]\n[[package]]\nname="cache-fixture-stable"\nversion="0.1.0"\n',
            }
            for name, value in files.items():
                path = source / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(value, encoding='utf-8', newline='\n')
                os.utime(path, (OLD, OLD))

    def source(self, variant):
        return (self.target / 'frozen-sources' if self.sources_in_target else self.root) / ('checkout-' + variant)

    def data(self, variant, value=22):
        return ('pub fn old_symbol()->u32 {11+cache_fixture_stable::value()}\n'
                + (f'pub fn new_symbol()->u32 {{{value}+cache_fixture_stable::value()}}\n' if variant == 'b' else '')
                + 'pub fn asset()->String {format!("{}{}",include_str!(concat!(env!("OUT_DIR"),"/value.txt")),include_str!("../../shared/suffix.txt"))}\n'
                + '#[test] fn actual_source(){assert_eq!(' + ('new_symbol(),23' if variant == 'b' else 'old_symbol(),12') + ');}\n')

    def command(self, action='build', package='cache-fixture-app', feature=False):
        return ['cargo', action, '--locked', '--offline', '-j1', '-p', package,
                '--message-format=json-render-diagnostics', *(['--features=extra'] if feature else [])]

    def run(self, variant='b', *, wrapped=True, action='build', package='cache-fixture-app', feature=False, mutate=None, expected=0):
        command = self.command(action, package, feature)
        before = memory_gate()
        index = len(self.records)
        log = self.root / f'{index:02d}.cargo.log'
        record = {'command': command, 'variant': variant, 'wrapped': wrapped, 'hostFreeBefore': before}
        try:
            if wrapped:
                with CargoCache(self.source(variant), self.target, self.environment,
                                extra_inputs={'cache-fixture-data': ('shared',)}) as cache:
                    result = cache.run(command, output=log, on_message=mutate)
                record['result'] = {'forced': result['forced'], 'mode': result['mode'],
                                    'artifacts': result['artifacts']}
                code = 0
            else:
                with log.open('xb') as output:
                    with subprocess.Popen(command, cwd=self.source(variant), env=self.environment,
                                          stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT) as process:
                        code = process.wait(timeout=60)
                        record['pid'] = process.pid
                result = None
            record['exitCode'] = code
            if code != expected:
                raise AssertionError(record)
            return result
        except BaseException as error:
            record['error'] = repr(error)
            raise
        finally:
            record['hostFreeAfter'] = memory_gate()
            self.records.append(record)
            (self.root / 'RUNTIME.json').write_text(json.dumps({'records': self.records, 'productBuild': False,
                                    'allReturned': True, 'target': str(self.target)}, indent=2), encoding='utf-8')

    def value(self):
        exe = self.target / 'debug' / ('cache-fixture-app.exe' if os.name == 'nt' else 'cache-fixture-app')
        return subprocess.run([str(exe)], stdin=subprocess.DEVNULL, capture_output=True, check=True).stdout.decode().strip()


class RegistryFixture(Fixture):
    """Owned offline directory source, retaining actual registry package IDs."""
    def __init__(self):
        self.root = BASE / 'reg'
        self.root.mkdir()
        self.target = self.root / 'target'
        self.sources_in_target = True
        self.records = []
        self.environment = {**os.environ, 'CARGO_TARGET_DIR': str(self.target),
                            'CARGO_NET_OFFLINE': 'true', 'CARGO_BUILD_JOBS': '1'}
        self.environment.pop('CARGO_BUILD_TARGET', None)
        registry = self.root / 'registry'
        checksums = {}
        for name, code, dependencies in (
                ('owned-parent', 'pub use owned_leaf::Token; pub fn root()->Token {owned_leaf::root()}\n',
                 '\n[dependencies]\nowned-leaf="0.1.0"\n'),
                ('owned-other', 'pub fn value()->u32 {7}\n', '')):
            folder = registry / name
            files = {'Cargo.toml': f'[package]\nname="{name}"\nversion="0.1.0"\nedition="2024"\n' + dependencies,
                     'src/lib.rs': code}
            for relative, content in files.items():
                path = folder / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(content.encode())
            checksum = hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest()
            checksums[name] = checksum
            (folder / '.cargo-checksum.json').write_text(json.dumps({
                'package': checksum, 'files': {key: hashlib.sha256(value.encode()).hexdigest()
                                               for key, value in files.items()}}))
        lock = '# Owned offline registry fixture\nversion=4\n'
        for name, dependencies in [('cache-fixture-app', ['owned-parent', 'owned-other']),
                                   ('owned-leaf', []), ('owned-parent', ['owned-leaf']), ('owned-other', [])]:
            lock += f'\n[[package]]\nname="{name}"\nversion="0.1.0"\n'
            if name in checksums:
                lock += f'source="registry+https://github.com/rust-lang/crates.io-index"\nchecksum="{checksums[name]}"\n'
            if dependencies:
                lock += 'dependencies=' + json.dumps(dependencies) + '\n'
        for variant in ('a', 'b'):
            files = {
                'Cargo.toml': '[workspace]\nmembers=["app"]\nexclude=["vendor/leaf"]\nresolver="2"\n[profile.dev]\ndebug=0\n[patch.crates-io]\nowned-leaf={path="vendor/leaf"}\n',
                'rust-toolchain.toml': (ROOT / 'rust-toolchain.toml').read_text(),
                'Cargo.lock': lock,
                '.cargo/config.toml': '[source.crates-io]\nreplace-with="owned"\n[source.owned]\ndirectory=' + json.dumps(str(registry)) + '\n',
                'app/Cargo.toml': '[package]\nname="cache-fixture-app"\nversion="0.1.0"\nedition="2024"\n[dependencies]\nowned-parent="0.1.0"\nowned-other="0.1.0"\n',
                'app/src/main.rs': 'fn main(){println!("{}:{}",owned_parent::root().0,owned_other::value());}\n',
                'vendor/leaf/Cargo.toml': '[package]\nname="owned-leaf"\nversion="0.1.0"\nedition="2024"\n',
                'vendor/leaf/src/lib.rs': 'pub struct Token(pub &' + "'static str); pub fn root()->Token {Token(env!(\"CARGO_MANIFEST_DIR\"))}\n",
            }
            for relative, content in files.items():
                path = self.source(variant) / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(content.encode())


class FingerprintInvalidation(unittest.TestCase):
    def test_validate_all_markers_before_writing_and_reject_hardlinks(self):
        root = BASE / 'marker-safety'
        target = root / 'target'
        folder = target / 'debug/.fingerprint/local-0123456789abcdef'
        folder.mkdir(parents=True)
        first, second = folder / 'lib-local', folder / 'test-lib-local'
        for marker in (first, second):
            marker.write_bytes(b'0123456789abcdef')
            marker.with_name(marker.name + '.json').write_text('{}')
        cache = CargoCache(root, target, {})
        inputs, forced = {'local/Cargo.toml': {'name': 'local', 'id': 'local'}}, {'local/Cargo.toml'}
        metadata = {'packages': [{'id': 'local', 'name': 'local', 'source': None}]}
        second.with_name(second.name + '.json').write_text('[]')
        with self.assertRaisesRegex(ValueError, 'sidecar'):
            cache._invalidate_local_fingerprints(inputs, forced, metadata, ['cargo', 'build'])
        self.assertEqual(first.read_bytes(), b'0123456789abcdef')
        second.with_name(second.name + '.json').write_text('{}')
        linked = root / 'owned-source-original'
        os.link(second, linked)
        with self.assertRaisesRegex(ValueError, 'hard links'):
            cache._invalidate_local_fingerprints(inputs, forced, metadata, ['cargo', 'build'])
        self.assertEqual(first.read_bytes(), b'0123456789abcdef')
        self.assertEqual(linked.read_bytes(), b'0123456789abcdef')

    def test_empty_marker_is_dirty_and_registry_ambiguity_is_rejected(self):
        root = BASE / 'marker-empty'
        target = root / 'target'
        folder = target / 'host-triple/release/.fingerprint/local-0123456789abcdef'
        folder.mkdir(parents=True)
        marker = folder / 'lib-local'
        marker.write_bytes(b'')
        marker.with_name(marker.name + '.json').write_text('{}')
        cache = CargoCache(root, target, {})
        inputs, forced = {'local/Cargo.toml': {'name': 'local', 'id': 'local'}}, {'local/Cargo.toml'}
        metadata = {'packages': [{'id': 'local', 'name': 'local', 'source': None}]}
        with self.assertRaisesRegex(ValueError, 'ambiguous'):
            cache._invalidate_local_fingerprints(inputs, forced,
                                                {'packages': [*metadata['packages'], {'id': 'registry', 'name': 'local', 'source': 'registry+test'}]}, ['cargo', 'build'])
        cache._invalidate_local_fingerprints(inputs, forced, metadata,
                                                ['cargo', 'build', '--target', 'host-triple', '--release'])
        self.assertEqual(marker.read_bytes(), b'')
        marker.write_bytes(b'not-a-valid-hash')
        with self.assertRaisesRegex(ValueError, 'short fingerprint'):
            cache._invalidate_local_fingerprints(inputs, forced, metadata,
                                                ['cargo', 'build', '--target', 'host-triple', '--release'])


class InputInventory(unittest.TestCase):
    def test_actual_literal_external_includes_are_declared_compiler_inputs(self):
        # Independently inventory current public Rust consumers. This detects a
        # new literal outside-package include omitted from the content policy;
        # it does not pretend to compile Core/App or replace dynamic build rules.
        files = snapshot(ROOT)
        packages = {}
        for name, data in files.items():
            if name.startswith('crates/') and name.endswith('/Cargo.toml'):
                manifest = tomllib.loads(data.decode('utf-8'))
                if 'package' in manifest:
                    packages[name.removesuffix('/Cargo.toml')] = manifest['package']['name']
        literal = re.compile(r'\binclude_(?:str|bytes)!\(\s*"([^"\\]*)"')
        for name, data in files.items():
            if not name.startswith('crates/') or not name.endswith('.rs'):
                continue
            owners = [folder for folder in packages if name.startswith(folder + '/')]
            if not owners:
                continue
            folder = max(owners, key=len)
            for included in literal.findall(data.decode('utf-8')):
                target = (ROOT / name).parent.joinpath(included).resolve()
                if target.is_relative_to(ROOT / folder):
                    continue
                relative = target.relative_to(ROOT).as_posix()
                self.assertTrue(any(relative == prefix or relative.startswith(prefix + '/')
                                    for prefix in EXTRA_INPUTS.get(packages[folder], ())),
                                f'Undeclared external compiler input: {name} -> {relative}')


class OwnedProcessLifecycle(unittest.TestCase):
    def test_parent_only_sigint_keeps_lock_until_owned_child_exits(self):
        # Exercise the very same Popen helper as Cargo, with a real owned Python
        # child. This controlled parent signal is not a terminal/GUI Ctrl+C test.
        for stage in ('stdout', 'wait'):
            with self.subTest(stage=stage):
                root = BASE / ('parent-sigint-' + stage)
                root.mkdir()
                scripts = {
                    'child.py': '''import json,os,sys,time
from pathlib import Path
root=Path(sys.argv[1]); stage=sys.argv[2]
(root/'child-ready.json').write_text(json.dumps({'pid':os.getpid()}))
if stage=='wait':
    os.close(1); os.close(2)
deadline=time.monotonic()+30
streaming=stage=='stdout'
output_closed_early=False
while not (root/'release').exists() and time.monotonic()<deadline:
    if streaming:
        try:
            sys.stdout.buffer.write(b'x'*65536+b'\\n'); sys.stdout.buffer.flush()
        except OSError:
            output_closed_early=True
            streaming=False
    time.sleep(.01)
code=0 if (root/'release').exists() else 9
(root/'child-exit.json').write_text(json.dumps({'pid':os.getpid(),'exitCode':code,'outputClosedEarly':output_closed_early}))
os._exit(code)
''',
                    'parent.py': '''import inspect,json,os,signal,sys,threading,time
from pathlib import Path
from cargo_cache import STATE,stream_owned_process,target_lock
root=Path(sys.argv[1]); stage=sys.argv[2]; target=root/'target'
count=0; owned=None
def interrupt(_signal,_frame):
    global count
    count+=1
    (root/('signal-'+str(count)+'.json')).write_text(json.dumps({'pid':os.getpid(),'childPid':owned.pid,'childLive':owned.poll() is None}))
    raise KeyboardInterrupt('owned-parent-interrupt-'+str(count))
signal.signal(signal.SIGINT,interrupt)
lines,start=inspect.getsourcelines(stream_owned_process)
wait_lines={start+i for i,line in enumerate(lines) if 'process.wait()' in line}
output_lines={start+i for i,line in enumerate(lines) if 'consume(line)' in line or 'for line in process.stdout' in line}
def trace(frame,event,_arg):
    global owned
    if event=='line' and frame.f_code is stream_owned_process.__code__:
        process=frame.f_locals.get('process')
        if process is not None and (root/'child-ready.json').exists() and process.poll() is None:
            owned=process
            first=count==0 and frame.f_lineno in (output_lines if stage=='stdout' else wait_lines)
            if first:
                signal.raise_signal(signal.SIGINT)
    return trace
def repeat_signal():
    deadline=time.monotonic()+20
    while not (root/'interrupt-again').exists() and time.monotonic()<deadline:
        time.sleep(.01)
    if (root/'interrupt-again').exists():
        signal.raise_signal(signal.SIGINT)
        (root/'repeat-signal-returned').write_text('SIGINT raised only in parent')
if stage=='stdout':
    threading.Thread(target=repeat_signal,daemon=True).start()
with target_lock(target):
    (target/STATE).write_text(json.dumps({'schemaVersion':1,'pending':True,'modes':{}}))
    try:
        sys.settrace(trace)
        stream_owned_process([sys.executable,str(root/'child.py'),str(root),stage],root,dict(os.environ),lambda _line: None)
    except BaseException as error:
        sys.settrace(None)
        (root/'propagated.json').write_text(json.dumps({'error':str(error),'type':type(error).__name__,'childPid':owned.pid,'childExitCode':owned.poll(),'signalsHandled':count,'handlerRestored':signal.getsignal(signal.SIGINT) is interrupt}))
        raise
    finally:
        sys.settrace(None)
''',
                    'waiter.py': '''import json,sys,time
from pathlib import Path
from cargo_cache import target_lock
root=Path(sys.argv[1])
(root/'waiter-waiting').write_text('waiting')
with target_lock(root/'target'):
    (root/'waiter-acquired.json').write_text(json.dumps({'childExitPresent':(root/'child-exit.json').exists()}))
''',
                }
                for name, content in scripts.items():
                    (root / name).write_text(content, encoding='utf-8', newline='\n')
                environment = {**os.environ, 'PYTHONPATH': str(ROOT / 'scripts')}
                before = memory_gate()
                parent = waiter = None
                observed = {}
                def await_file(name, timeout=10):
                    deadline = time.monotonic() + timeout
                    while not (root / name).exists() and time.monotonic() < deadline:
                        time.sleep(.01)
                    self.assertTrue((root / name).exists(), name)
                with (root / 'parent-output.log').open('xb') as parent_log, (root / 'waiter-output.log').open('xb') as waiter_log:
                    try:
                        parent = subprocess.Popen([sys.executable, str(root / 'parent.py'), str(root), stage],
                                                  env=environment, stdin=subprocess.DEVNULL,
                                                  stdout=parent_log, stderr=subprocess.STDOUT)
                        await_file('signal-1.json')
                        waiter = subprocess.Popen([sys.executable, str(root / 'waiter.py'), str(root)],
                                                  env=environment, stdin=subprocess.DEVNULL,
                                                  stdout=waiter_log, stderr=subprocess.STDOUT)
                        await_file('waiter-waiting')
                        time.sleep(.4)  # Exceed Popen.__exit__'s short SIGINT grace.
                        observed.update(parentLiveAfterInterrupt=parent.poll() is None,
                                        childStillGated=not (root / 'child-exit.json').exists(),
                                        waiterBlocked=not (root / 'waiter-acquired.json').exists())
                        if stage == 'stdout' and observed['parentLiveAfterInterrupt']:
                            (root / 'interrupt-again').touch()
                            await_file('repeat-signal-returned')
                            time.sleep(.2)
                            observed['waiterBlockedAfterRepeatedSignal'] = not (root / 'waiter-acquired.json').exists()
                    finally:
                        (root / 'release').touch()
                        if parent is not None:
                            parent.wait(timeout=30)
                        if waiter is not None:
                            waiter.wait(timeout=30)
                        if parent is not None:
                            await_file('child-exit.json')
                        (root / 'RUNTIME.json').write_text(json.dumps({
                            'mechanism': 'actual owned Python child; controlled parent-only signal.raise_signal(SIGINT)',
                            'stage': stage, 'observed': observed, 'parentPid': None if parent is None else parent.pid,
                            'parentExitCode': None if parent is None else parent.returncode,
                            'waiterPid': None if waiter is None else waiter.pid,
                            'waiterExitCode': None if waiter is None else waiter.returncode,
                            'hostFreeBefore': before, 'hostFreeAfter': memory_gate(),
                            'productBuild': False, 'parentAndWaiterReturned': True,
                            'childExitMarkerPresent': (root / 'child-exit.json').exists(),
                        }, indent=2), encoding='utf-8')
                self.assertTrue(all(observed.values()), observed)
                self.assertNotEqual(parent.returncode, 0)
                self.assertEqual(waiter.returncode, 0)
                self.assertEqual(json.loads((root / 'child-exit.json').read_bytes())['exitCode'], 0)
                self.assertFalse(json.loads((root / 'child-exit.json').read_bytes())['outputClosedEarly'])
                self.assertTrue(json.loads((root / 'signal-1.json').read_bytes())['childLive'])
                propagated = json.loads((root / 'propagated.json').read_bytes())
                self.assertEqual(propagated['type'], 'KeyboardInterrupt')
                self.assertEqual(propagated['error'], 'owned-parent-interrupt-1')
                self.assertEqual(propagated['childExitCode'], 0)
                self.assertEqual(propagated['signalsHandled'], 1)
                self.assertTrue(propagated['handlerRestored'])
                self.assertTrue(json.loads((root / 'waiter-acquired.json').read_bytes())['childExitPresent'])
                self.assertTrue(json.loads((root / 'target' / STATE).read_bytes())['pending'])


class ActualCargo(unittest.TestCase):
    def test_registry_parent_of_local_patch_rebuilds_but_independent_registry_stays_warm(self):
        fixture = RegistryFixture()
        fixture.run('a')
        original = {path.relative_to(fixture.source('a')).as_posix():
                    (path.read_bytes(), path.stat().st_mtime_ns, path.stat().st_ino)
                    for path in fixture.source('a').rglob('*') if path.is_file()}
        self.assertEqual(fixture.value(), str(fixture.source('a') / 'vendor/leaf') + ':7')
        invalidate = CargoCache._invalidate_local_fingerprints
        def local_only(cache, inputs, forced, metadata, command):
            return invalidate(cache, inputs, forced, {**metadata, 'resolve': {'nodes': []}}, command)
        with mock.patch.object(CargoCache, '_invalidate_local_fingerprints', local_only):
            with self.assertRaises(subprocess.CalledProcessError):
                fixture.run('b')
        events = []
        for line in (fixture.root / '01.cargo.log').read_bytes().splitlines():
            try:
                events.append(json.loads(line))
            except ValueError:
                pass
        old_artifacts = [event for event in events if event.get('reason') == 'compiler-artifact']
        self.assertTrue(next(event for event in old_artifacts if '#owned-parent@' in event['package_id'])['fresh'])
        self.assertFalse(next(event for event in old_artifacts if '/vendor/leaf#' in event['package_id'])['fresh'])
        self.assertIn(b'error[E0460]:', (fixture.root / '01.cargo.log').read_bytes())
        self.assertTrue(json.loads((fixture.target / STATE).read_bytes())['pending'])
        repaired = fixture.run('b')
        parent = next(event for event in repaired['artifacts'] if '#owned-parent@' in event['package_id'])
        other = next(event for event in repaired['artifacts'] if '#owned-other@' in event['package_id'])
        self.assertTrue(parent['package_id'].startswith('registry+'))
        self.assertFalse(parent['fresh'])
        self.assertTrue(other['fresh'])
        self.assertEqual(fixture.value(), str(fixture.source('b') / 'vendor/leaf') + ':7')
        self.assertEqual(original, {path.relative_to(fixture.source('a')).as_posix():
                                   (path.read_bytes(), path.stat().st_mtime_ns, path.stat().st_ino)
                                   for path in fixture.source('a').rglob('*') if path.is_file()})
        warm = fixture.run('b')
        self.assertEqual(warm['forced'], [])
        self.assertTrue(all(event['fresh'] for event in warm['artifacts']))

    def test_target_contained_source_switch_rebuilds_vendor_without_touching_original(self):
        fixture = Fixture('in-target', sources_in_target=True)
        for variant in ('a', 'b'):
            source = fixture.source(variant)
            vendor = source / 'vendor/stable/src/lib.rs'
            vendor.write_text('pub fn value()->u32 {1}\npub fn root()-> &' + "'static str {env!(\"CARGO_MANIFEST_DIR\")}\n", encoding='utf-8')
            data = source / 'data/src/lib.rs'
            data.write_text(data.read_text() + 'pub fn vendor_root()-> &' + "'static str {cache_fixture_stable::root()}\n", encoding='utf-8')
            app = source / 'app/src/main.rs'
            app.write_text(app.read_text().replace('fn main(){', 'fn main(){println!("{}",cache_fixture_data::vendor_root());'), encoding='utf-8')
        fixture.run('a')
        # A public source may itself contain a fingerprint-shaped fixture.
        # It is not a Cargo output location, even though A is inside target.
        source_marker = fixture.source('a') / '.fingerprint/cache-fixture-stable-0123456789abcdef/lib-stable'
        source_marker.parent.mkdir(parents=True)
        source_marker.write_bytes(b'0123456789abcdef')
        source_marker.with_name(source_marker.name + '.json').write_text('{}')
        original = {path.relative_to(fixture.source('a')).as_posix():
                    (path.read_bytes(), path.stat().st_mtime_ns, path.stat().st_ino)
                    for path in fixture.source('a').rglob('*') if path.is_file()}
        self.assertEqual(fixture.value().splitlines()[0], str(fixture.source('a') / 'vendor/stable'))
        # Reproduce the old entry-touch policy on the actual shared fixture
        # target. The strict gate must reject Cargo's retained vendor unit.
        with mock.patch.object(CargoCache, '_invalidate_local_fingerprints', return_value=None):
            with self.assertRaisesRegex(ValueError, 'Cargo reused an unqualified local unit'):
                fixture.run('b')
        self.assertTrue(json.loads((fixture.target / STATE).read_bytes())['pending'])
        repaired = fixture.run('b')
        self.assertTrue(all(message['fresh'] is False for message in repaired['artifacts']))
        self.assertEqual(fixture.value().splitlines(),
                         [str(fixture.source('b') / 'vendor/stable'), '23:owned-build-asset-b'])
        self.assertEqual(original, {path.relative_to(fixture.source('a')).as_posix():
                                   (path.read_bytes(), path.stat().st_mtime_ns, path.stat().st_ino)
                                   for path in fixture.source('a').rglob('*') if path.is_file()})
        warm = fixture.run('b')
        self.assertEqual(warm['forced'], [])
        self.assertTrue(all(message['fresh'] for message in warm['artifacts']))

    def test_same_target_product_lock_precedes_cargo_and_success_hint(self):
        fixture = Fixture('concurrent-product-lock')
        script = '''import json,sys
from pathlib import Path
from cargo_cache import CargoCache
print("waiting",flush=True)
with CargoCache(Path(sys.argv[1]),Path(sys.argv[2]),dict(__import__("os").environ),extra_inputs={"cache-fixture-data":("shared",)}) as cache:
    result=cache.run(json.loads(sys.argv[3]),output=Path(sys.argv[4]))
print(json.dumps({"forced":result["forced"]}),flush=True)
'''
        environment = {**fixture.environment, 'PYTHONPATH': str(ROOT / 'scripts')}
        free_before = memory_gate()
        session = CargoCache(fixture.source('a'), fixture.target, fixture.environment)
        with session:
            child = subprocess.Popen([sys.executable, '-c', script, str(fixture.source('b')), str(fixture.target),
                                      json.dumps(fixture.command()), str(fixture.root / 'child-cargo.log')],
                                     cwd=fixture.source('b'), env=environment, stdin=subprocess.DEVNULL,
                                     stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            self.assertEqual(child.stdout.readline(), b'waiting\r\n' if os.name == 'nt' else b'waiting\n')
            time.sleep(.2)
            self.assertIsNone(child.poll())
            self.assertFalse((fixture.target / STATE).exists(), 'A waiting command must not write source success/pending')
            memory_gate()
        out, _ = child.communicate(timeout=60)
        (fixture.root / 'child-output.log').write_bytes(out)
        self.assertEqual(child.returncode, 0, out.decode(errors='replace'))
        self.assertFalse(json.loads((fixture.target / STATE).read_bytes())['pending'])
        (fixture.root / 'RUNTIME.json').write_text(json.dumps({
            'productBuild': False, 'allReturned': True, 'childPid': child.pid, 'exitCode': child.returncode,
            'cargoCommand': fixture.command(), 'target': str(fixture.target),
            'hostFreeBefore': free_before, 'hostFreeAfter': memory_gate(),
            'observedWaitingUnderProductLock': True, 'hintAbsentWhileWaiting': True,
        }, indent=2), encoding='utf-8')
        with self.assertRaises(ValueError):
            session.run(fixture.command())

    def test_two_checkouts_old_mtime_test_and_normal_then_current_source_and_warm_reuse(self):
        fixture = Fixture('normal-test-source-switch')
        fixture.run('a', wrapped=False)
        fixture.run('b', wrapped=False, action='test', package='cache-fixture-data')
        os.utime(fixture.source('b') / 'app/src/main.rs', None)
        fixture.run('b', wrapped=False, expected=101)
        original = fixture.root / 'unqualified-originals'
        original.mkdir()
        # Preserve the actual stale unit bytes before the wrapper changes them.
        for directory in (fixture.target / 'debug/deps', fixture.target / 'debug/.fingerprint'):
            for path in directory.rglob('*'):
                if path.is_file() and ('cache_fixture_data' in path.name or 'cache-fixture-data' in path.as_posix()):
                    target = original / path.relative_to(fixture.target)
                    if os.name == 'nt':
                        # The owned fixture paths are absolute local paths.
                        # Explicit extended paths also work when the host has
                        # not enabled the process-wide long-path policy.
                        path = Path('\\\\?\\' + str(path))
                        target = Path('\\\\?\\' + str(target))
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(path, target)
                    self.assertEqual(target.read_bytes(), path.read_bytes())
        result = fixture.run()
        self.assertIn('data/Cargo.toml', result['forced'])
        self.assertEqual(fixture.value(), '23:owned-build-asset-b')
        warm = fixture.run()
        self.assertEqual(warm['forced'], [])
        self.assertTrue(all(message['fresh'] for message in warm['artifacts']))
        # A changed current Rust file is newer in content but deliberately old
        # in mtime. Only this local package needs explicit invalidation.
        path = fixture.source('b') / 'data/src/lib.rs'
        path.write_text(fixture.data('b', 33), encoding='utf-8', newline='\n')
        os.utime(path, (OLD, OLD))
        changed = fixture.run()
        self.assertEqual(changed['forced'], ['data/Cargo.toml'])
        self.assertEqual(fixture.value(), '34:owned-build-asset-b')
        self.assertTrue(next(message for message in changed['artifacts'] if 'cache-fixture-stable' in message['package_id'])['fresh'])
        # A build-script input also participates, even with a regressed mtime.
        asset = fixture.source('b') / 'data/build-value.txt'
        asset.write_text('updated-build-asset', encoding='utf-8')
        os.utime(asset, (OLD, OLD))
        self.assertEqual(fixture.run()['forced'], ['data/Cargo.toml'])
        self.assertEqual(fixture.value(), '34:updated-build-asset')
        external = fixture.source('b') / 'shared/suffix.txt'
        external.write_text(':external-input', encoding='utf-8')
        os.utime(external, (OLD, OLD))
        self.assertEqual(fixture.run()['forced'], ['data/Cargo.toml'])
        self.assertEqual(fixture.value(), '34:updated-build-asset:external-input')

    def test_failed_compile_and_changed_inputs_never_commit_success(self):
        fixture = Fixture('failure-and-change')
        fixture.run()
        path = fixture.source('b') / 'data/src/lib.rs'
        original = path.read_bytes()
        path.write_bytes(b'this is deliberately invalid Rust\n')
        os.utime(path, (OLD, OLD))
        with self.assertRaises(subprocess.CalledProcessError):
            fixture.run()
        self.assertTrue(json.loads((fixture.target / STATE).read_bytes())['pending'])
        path.write_bytes(original)
        os.utime(path, (OLD, OLD))
        self.assertIn('data/Cargo.toml', fixture.run()['forced'])
        done = False
        def mutate(message):
            nonlocal done
            if not done and 'cache-fixture-data' in message['package_id']:
                path.write_bytes(original + b'// changed after a real compiler-artifact\n')
                os.utime(path, (OLD, OLD))
                done = True
        with self.assertRaisesRegex(ValueError, 'changed during Cargo'):
            fixture.run(feature=True, mutate=mutate)
        self.assertTrue(done)
        self.assertTrue(json.loads((fixture.target / STATE).read_bytes())['pending'])
        fixture.run(feature=True)
        self.assertFalse(json.loads((fixture.target / STATE).read_bytes())['pending'])
        def reject_artifact(_message):
            raise RuntimeError('owned artifact consumer refused')
        with self.assertRaisesRegex(RuntimeError, 'consumer refused'):
            fixture.run(feature=True, mutate=reject_artifact)
        self.assertTrue(json.loads((fixture.target / STATE).read_bytes())['pending'])

    def test_features_and_external_artifact_change_are_not_source_success(self):
        fixture = Fixture('features-and-witness')
        fixture.run()
        feature = fixture.run(feature=True)
        self.assertIn('data/Cargo.toml', feature['forced'])
        self.assertEqual(fixture.run(feature=True)['forced'], [])
        # An unmanaged Cargo command replaces the same unit from checkout A.
        # The recorded artifact witness must refuse the previous B hint.
        os.utime(fixture.source('a') / 'data/src/lib.rs', None)
        os.utime(fixture.source('a') / 'app/src/main.rs', None)
        fixture.run('a', wrapped=False, feature=True)
        result = fixture.run(feature=True)
        self.assertIn('data/Cargo.toml', result['forced'])
        self.assertEqual(fixture.value(), '23:owned-build-asset-b')
        hint = json.loads((fixture.target / STATE).read_bytes())
        hint['modes'][result['mode']]['packages'] = 'unknown malformed hint'
        (fixture.target / STATE).write_text(json.dumps(hint), encoding='utf-8')
        self.assertEqual(len(fixture.run(feature=True)['forced']), 3)
        # Mutable generated Rust inputs can be replaced by another unit before
        # the old normal rlib changes. They belong to the artifact witness too.
        for generated in (fixture.target / 'debug/build').glob('cache-fixture-data-*/out/value.txt'):
            generated.write_text('unqualified-generated-cache', encoding='utf-8')
        repaired = fixture.run(feature=True)
        self.assertIn('data/Cargo.toml', repaired['forced'])
        self.assertEqual(fixture.value(), '23:owned-build-asset-b')


class RuntimeModeQualification(unittest.TestCase):
    """In-memory Cargo artifact qualification; no Cargo, lock or target IO."""
    def setUp(self):
        self.root, self.target = BASE, BASE / 'unopened-target'
        self.entry = BASE / 'logical-source.rs'
        self.package = 'path+file:///owned#mode-fixture@0.1.0'
        self.key = 'owned/Cargo.toml'
        self.state = {'schemaVersion': 1, 'pending': False, 'modes': {}}
        self.writes, self.executions, self.invalidations, self.metadata_commands = [], [], [], []
        self.source_token, self.tool_token = 'source-a', 'tool-a'
        self.artifact_token, self.generated_token = 'artifact-a', 'generated-a'
        self.exit_code, self.forced, self.current_command = 0, set(), None
        self.cache = CargoCache(self.root, self.target, {'CARGO_BUILD_JOBS': '1'})
        self.cache.lock = object()  # Held ownership is mocked; never enter a real lock.
        def metadata(command, **kwargs):
            self.metadata_commands.append(copy.deepcopy(command))
            return subprocess.CompletedProcess(command, 0, stdout=json.dumps({
                'target_directory': str(self.target)}).encode())
        def inputs(*_):
            return {self.key: {'id': self.package, 'sha256': self.source_token,
                'entries': [self.entry], 'customBuild': True}}
        def write(value):
            self.writes.append(copy.deepcopy(value))
            self.state = copy.deepcopy(value)
        def state():
            return copy.deepcopy(self.state if self.state['pending'] is False else
                {'schemaVersion': 1, 'pending': False, 'modes': {}})
        def invalidate(inputs, forced, metadata, command):
            self.assertIs(command, self.current_command)
            self.forced = set(forced)
            self.invalidations.append((copy.deepcopy(command), set(forced)))
            return {self.package} if forced else set()
        def stream(command, root, environment, consume):
            self.assertIs(command, self.current_command)
            self.assertEqual(self.writes[-1], {'schemaVersion': 1, 'pending': True, 'modes': {}})
            self.executions.append(copy.deepcopy(command))
            if self.forced:
                self.artifact_token = f'artifact-{len(self.executions)}-{self.source_token}'
                self.generated_token = f'generated-{len(self.executions)}-{self.source_token}'
            for event in ({'reason': 'compiler-artifact', 'package_id': self.package,
                    'target': {'src_path': str(self.entry)}, 'filenames': [str(self.target / 'unit')],
                    'fresh': not bool(self.forced)},
                    {'reason': 'build-script-executed', 'package_id': self.package,
                     'out_dir': str(self.target / 'logical-generated')}):
                consume(json.dumps(event).encode() + b'\n')
            return self.exit_code
        def generated(events):
            self.assertEqual(len(events), 1)
            self.assertEqual(events[0]['package_id'], self.package)
            return self.generated_witness()
        patches = [mock.patch.object(cache_module.subprocess, 'run', side_effect=metadata),
            mock.patch.object(cache_module, 'compiler_inputs', side_effect=inputs),
            mock.patch.object(cache_module, 'tool_identity', side_effect=lambda *_: {'identity': self.tool_token}),
            mock.patch.object(cache_module, 'read_checked', side_effect=lambda *_: self.source_token.encode()),
            mock.patch.object(cache_module.os, 'utime'),
            mock.patch.object(cache_module, 'stream_owned_process', side_effect=stream),
            mock.patch.object(self.cache, '_state', side_effect=state),
            mock.patch.object(self.cache, '_write', side_effect=write),
            mock.patch.object(self.cache, '_invalidate_local_fingerprints', side_effect=invalidate),
            mock.patch.object(self.cache, '_matches', side_effect=lambda values: values == self.artifact_witness()),
            mock.patch.object(self.cache, '_witness', side_effect=lambda _: self.artifact_witness()),
            mock.patch.object(self.cache, '_matches_generated', side_effect=lambda values, required:
                required is True and values == self.generated_witness()),
            mock.patch.object(self.cache, '_generated', side_effect=generated)]
        for patcher in patches:
            patcher.start()
            self.addCleanup(patcher.stop)

    def artifact_witness(self):
        return [{'path': 'unit', 'bytes': len(self.artifact_token), 'sha256': self.artifact_token}]

    def generated_witness(self):
        return [{'path': 'logical-generated', 'sha256': self.generated_token, 'declarations': 'owned'}]

    def command(self, *tail, verb='test', options=()):
        return ['cargo', verb, '--locked', '-p', 'mode-fixture', '--lib',
            '--message-format=json-render-diagnostics', *options, *tail]

    def invoke(self, command):
        original = copy.deepcopy(command)
        self.current_command = command
        try:
            return self.cache.run(command)
        finally:
            self.assertEqual(command, original)

    def test_runtime_filters_exact_skip_and_threads_share_qualification_but_execute_every_original_argv(self):
        first = self.invoke(self.command('--', 'first', '--exact', '--test-threads=1'))
        second = self.invoke(self.command('--', '--skip', 'first', '--test-threads=8', '--nocapture'))
        self.assertEqual(first['mode'], second['mode'])
        self.assertEqual(first['forced'], [self.key])
        self.assertEqual(second['forced'], [])
        self.assertTrue(second['artifacts'][0]['fresh'])
        self.assertEqual(len(self.executions), 2)
        self.assertEqual(self.executions, [item[0] for item in self.invalidations])
        self.assertEqual(self.executions[1][-5:], ['--', '--skip', 'first', '--test-threads=8', '--nocapture'])

    def test_warm_runtime_failure_keeps_original_command_and_pending_then_requires_requalification(self):
        first = self.invoke(self.command('--', 'first', '--exact'))
        self.exit_code = 101
        failed = self.command('--', 'failing-assertion', '--test-threads=1')
        with self.assertRaises(subprocess.CalledProcessError) as raised:
            self.invoke(failed)
        self.assertEqual(raised.exception.returncode, 101)
        self.assertIs(raised.exception.cmd, failed)
        self.assertEqual(self.invalidations[-1][1], set())
        self.assertEqual(self.state, {'schemaVersion': 1, 'pending': True, 'modes': {}})
        self.exit_code = 0
        recovered = self.invoke(self.command('--', '--skip', 'failing-assertion'))
        self.assertEqual(first['mode'], recovered['mode'])
        self.assertEqual(recovered['forced'], [self.key])
        self.assertEqual(len(self.executions), 3)

    def test_compile_prefix_features_profile_target_package_and_target_kind_remain_distinct(self):
        commands = [self.command('--', 'first'),
            self.command('--', 'first', options=('--features=extra',)),
            self.command('--', 'first', options=('--release',)),
            self.command('--', 'first', options=('--target', 'other-triple')),
            [part if part != 'mode-fixture' else 'another-package' for part in self.command('--', 'first')],
            [part if part != '--lib' else '--bins' for part in self.command('--', 'first')]]
        modes = [self.invoke(command)['mode'] for command in commands]
        self.assertEqual(len(set(modes)), len(modes))
        self.assertEqual(self.metadata_commands[1][-1], '--features=extra')

    def test_non_test_compiler_tail_is_retained_and_unsupported_rustc_stays_rejected(self):
        for verb in ('build', 'check', 'clippy'):
            with self.subTest(verb=verb):
                deny = self.invoke(self.command('--', '-D', 'warnings', verb=verb))
                allow = self.invoke(self.command('--', '-A', 'warnings', verb=verb))
                self.assertNotEqual(deny['mode'], allow['mode'])
        before = len(self.executions)
        with self.assertRaisesRegex(ValueError, 'supported compile command'):
            self.invoke(self.command('--', '-C', 'opt-level=3', verb='rustc'))
        self.assertEqual(len(self.executions), before)

    def test_first_delimiter_empty_tail_multiple_delimiters_and_no_delimiter_share_test_context(self):
        commands = [self.command(), self.command('--'), self.command('--', 'first'),
            self.command('--', '--', 'second', '--exact')]
        results = [self.invoke(command) for command in commands]
        self.assertEqual(len({result['mode'] for result in results}), 1)
        self.assertTrue(all(not result['forced'] for result in results[1:]))
        self.assertEqual(self.executions, commands)

    def test_source_tool_environment_and_source_root_keep_their_existing_identity_boundaries(self):
        first = self.invoke(self.command('--', 'first'))
        self.source_token = 'source-b'
        source = self.invoke(self.command('--', 'second'))
        self.assertEqual(first['mode'], source['mode'])
        self.assertEqual(source['forced'], [self.key])
        self.tool_token = 'tool-b'
        tool = self.invoke(self.command('--', 'third'))
        self.assertNotEqual(source['mode'], tool['mode'])
        self.assertEqual(tool['forced'], [self.key])
        self.cache.environment['RUSTFLAGS'] = '--cfg=owned'
        environment = self.invoke(self.command('--', 'fourth'))
        self.assertNotEqual(tool['mode'], environment['mode'])
        self.assertEqual(environment['forced'], [self.key])
        self.cache.root = BASE.parent
        root = self.invoke(self.command('--', 'fifth'))
        self.assertNotEqual(environment['mode'], root['mode'])
        self.assertEqual(root['forced'], [self.key])

    def test_artifact_and_generated_witness_changes_still_force_the_same_context(self):
        first = self.invoke(self.command('--', 'first'))
        self.artifact_token = 'changed by another compiler'
        artifact = self.invoke(self.command('--', 'second'))
        self.assertEqual(first['mode'], artifact['mode'])
        self.assertEqual(artifact['forced'], [self.key])
        self.generated_token = 'changed generated input'
        generated = self.invoke(self.command('--', 'third'))
        self.assertEqual(first['mode'], generated['mode'])
        self.assertEqual(generated['forced'], [self.key])


if __name__ == '__main__':
    print(f'Owned Cargo fixture evidence: {BASE}', flush=True)
    unittest.main(argv=[sys.argv[0], *remaining])
