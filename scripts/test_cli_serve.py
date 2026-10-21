"""Protocol/controller boundaries only; fake processes are not product acceptance."""
import ctypes
from ctypes import wintypes
import json
import os
from pathlib import Path
import queue
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from _cli_serve import OwnedServe, _creation_time, parse_ready, parse_shutdown, serve_command

CREATED = 134055000000000001  # Deliberately larger than JavaScript Number precision.
IDENTITY = {'pid': 41, 'created': CREATED, 'incarnation': 'opaque-incarnation',
            'profileIdentity': 'opaque-qualified-profile', 'role': 'local'}
TOKEN = 'a' * 64


def ready(identity=IDENTITY, *, hosted=False):
    return json.dumps({'schemaVersion': 1, 'kind': 'ready',
                       'url': 'http://127.0.0.1:12345/' + ('' if hosted else '#token=' + TOKEN),
                       'ownerIdentity': identity}).encode() + b'\n'


def receipt(identity=IDENTITY):
    return json.dumps({'schemaVersion': 1, 'call': 'ownerStop',
                       'result': {'owner': identity, 'exitCode': 0}}).encode() + b'\n'


class Pipe:
    def __init__(self, initial=None):
        self.chunks = queue.Queue()
        if initial is not None:
            self.chunks.put(initial)

    def readline(self, limit):
        return self.chunks.get()

    def read1(self, limit):
        return self.chunks.get()

    def close(self):
        pass


class ServeFixture:
    pid = 41
    def __init__(self, initial):
        self.stdout, self.stderr = Pipe(initial), Pipe()
        self.returncode = None

    def poll(self):
        return self.returncode

    def finish(self, code):
        if self.returncode is None:
            self.returncode = code
            self.stdout.chunks.put(b'')
            self.stderr.chunks.put(b'')

    def wait(self, timeout=None):
        if self.returncode is None:
            raise AssertionError('Fixture only exits after the control action')
        return self.returncode


class ControlFixture:
    pid = 42
    def __init__(self, callback):
        self.callback = callback

    def wait(self, timeout=None):
        return self.callback()


class Fixture:
    def __init__(self, *, initial=ready(), stop_code=0, owner_exit=0, stopped_identity=IDENTITY,
                 status_identity=IDENTITY):
        self.service = ServeFixture(initial)
        self.stop_code, self.owner_exit = stop_code, owner_exit
        self.stopped_identity, self.status_identity = stopped_identity, status_identity
        self.calls, self.owner = [], None

    def popen(self, command, **kwargs):
        self.calls.append(command)
        if command[1] == 'serve':
            return self.service
        if command[1] == 'owner-status':
            payload = {'schemaVersion': 1, 'call': 'ownerStatus', 'ownerIdentity': self.status_identity}
            kwargs['stdout'].write(json.dumps(payload).encode() + b'\n')
            return ControlFixture(lambda: 0)
        if command[1] == 'owner-stop':
            kwargs['stdout'].write(receipt(self.stopped_identity) if self.stop_code == 0 else b'')
            def result():
                if self.stop_code == 0:
                    self.service.finish(self.owner_exit)
                return self.stop_code
            return ControlFixture(result)
        raise AssertionError('No other process command is allowed')

    def launch(self, root):
        with patch('_cli_serve.subprocess.Popen', side_effect=self.popen), patch('_cli_serve._creation_time', return_value=CREATED):
            self.owner = OwnedServe(root / 'weftext-cli.exe', ['--data-dir', str(root / 'state')])
        return self.owner

    def close_fixture(self):
        self.service.finish(3)
        if self.owner:
            self.owner._join()


class ReadyAndReceipt(unittest.TestCase):
    def test_hosted_bind_comes_from_operator_config_without_an_added_port(self):
        executable = Path(__file__).absolute().parent / 'weftext-cli.exe'
        for config_option in (['--server-config', 'operator.json'], ['--server-config=operator.json']):
            command = serve_command(executable, config_option)
            self.assertNotIn('--port', command)
            explicit = serve_command(executable, [*config_option, '--port', '12'])
            self.assertEqual(explicit[-2:], ['--port', '12'])
        self.assertEqual(serve_command(executable, ['--data-dir', 'state'])[-2:], ['--port', '0'])

    def test_exact_integer_and_actual_url_credential(self):
        parsed = parse_ready(ready(), 41)
        self.assertEqual(parsed['ownerIdentity']['created'], CREATED)
        self.assertEqual(parsed['base'], 'http://127.0.0.1:12345')
        self.assertEqual(parsed['token'], TOKEN)
        hosted = {**IDENTITY, 'role': 'hosted'}
        self.assertIsNone(parse_ready(ready(hosted, hosted=True), 41, True)['token'])

    def test_wrong_process_role_keys_and_legacy_url_are_rejected(self):
        for identity in ({**IDENTITY, 'pid': 42}, {**IDENTITY, 'role': 'hosted'},
                         {**IDENTITY, 'created': True}, {**IDENTITY, 'extra': 'ignored'}):
            with self.subTest(identity=identity), self.assertRaises(ValueError):
                parse_ready(ready(identity), 41)
        for payload in (b'http://127.0.0.1:12345/#token=' + TOKEN.encode() + b'\n',
                        ready().replace(b'"schemaVersion": 1,', b'"schemaVersion": 1, "schemaVersion": 1,')):
            with self.assertRaises(ValueError):
                parse_ready(payload, 41)

    def test_protocol_json_does_not_autodetect_non_utf8_encodings(self):
        for encoding in ('utf-16-be', 'utf-16-le', 'utf-32-be', 'utf-32-le'):
            with self.subTest(encoding=encoding), self.assertRaises(ValueError):
                parse_ready(ready().decode('utf-8').encode(encoding), 41)

    def test_local_ready_cannot_redirect_http_checks_to_an_external_service(self):
        for host in ('remote.invalid', '@127.0.0.1'):
            with self.subTest(host=host), self.assertRaises(ValueError):
                parse_ready(ready().replace(b'127.0.0.1', host.encode()), 41)

    def test_shutdown_requires_the_same_complete_identity(self):
        self.assertEqual(parse_shutdown(receipt(), IDENTITY)['exitCode'], 0)
        for key, value in (('pid', 42), ('created', CREATED + 1), ('incarnation', 'another'),
                           ('profileIdentity', 'another'), ('role', 'hosted')):
            with self.subTest(key=key), self.assertRaises(ValueError):
                parse_shutdown(receipt({**IDENTITY, key: value}), IDENTITY)

    @unittest.skipUnless(os.name == 'nt', 'Read-only GetProcessTimes check requires Windows')
    def test_creation_time_uses_a_held_handle(self):
        current = ctypes.WinDLL('kernel32.dll', use_last_error=True, winmode=0x800).GetCurrentProcess
        current.argtypes, current.restype = [], wintypes.HANDLE
        process = type('CurrentProcess', (), {'_handle': current()})()
        self.assertGreater(_creation_time(process), 2 ** 53)


class Controller(unittest.TestCase):
    def test_single_typed_stop_preserves_raw_and_exact_identity(self):
        fixture = Fixture()
        with tempfile.TemporaryDirectory() as directory:
            try:
                owner = fixture.launch(Path(directory)).wait_ready(.5)
                with patch('_cli_serve.subprocess.Popen', side_effect=fixture.popen):
                    self.assertEqual(owner.stop(), ('', ''))
                    self.assertEqual(owner.stop(), ('', ''))
                self.assertEqual([call[1] for call in fixture.calls], ['serve', 'owner-stop'])
                self.assertEqual(json.loads((owner.directory / 'owner-identity.json').read_bytes()), IDENTITY)
                self.assertEqual((owner.directory / 'serve.stdout.bin').read_bytes(), ready())
                record = json.loads((owner.directory / 'lifecycle.json').read_bytes())
                self.assertTrue(record['stopped'])
                self.assertEqual(record['shutdownReceipt']['owner']['created'], CREATED)
            finally:
                fixture.close_fixture()

    def test_unknown_stop_is_not_retried_or_replaced_with_kill(self):
        fixture = Fixture(stop_code=6)
        with tempfile.TemporaryDirectory() as directory:
            try:
                owner = fixture.launch(Path(directory)).wait_ready(.5)
                with patch('_cli_serve.subprocess.Popen', side_effect=fixture.popen):
                    for _ in range(2):
                        with self.assertRaisesRegex(RuntimeError, 'no retry/kill'):
                            owner.stop()
                self.assertEqual([call[1] for call in fixture.calls], ['serve', 'owner-stop'])
                self.assertIsNone(fixture.service.poll())
                self.assertFalse(json.loads((owner.directory / 'lifecycle.json').read_bytes())['stopped'])
            finally:
                fixture.close_fixture()

    def test_receipt_alone_does_not_replace_actual_child_exit(self):
        fixture = Fixture(owner_exit=3)
        with tempfile.TemporaryDirectory() as directory:
            try:
                owner = fixture.launch(Path(directory)).wait_ready(.5)
                with patch('_cli_serve.subprocess.Popen', side_effect=fixture.popen):
                    with self.assertRaisesRegex(RuntimeError, 'Typed shutdown failed'):
                        owner.stop()
            finally:
                fixture.close_fixture()

    def test_unready_cleanup_qualifies_pid_and_creation_then_stops_once(self):
        for matches in (True, False):
            fixture = Fixture(initial=None, status_identity={**IDENTITY, 'created': CREATED if matches else CREATED + 1})
            with self.subTest(matches=matches), tempfile.TemporaryDirectory() as directory:
                try:
                    owner = fixture.launch(Path(directory))
                    with patch('_cli_serve.subprocess.Popen', side_effect=fixture.popen):
                        for _ in range(2):
                            with self.assertRaisesRegex(RuntimeError, 'Serve Ready failed'):
                                owner.wait_ready(0)
                    self.assertEqual([call[1] for call in fixture.calls], ['serve', 'owner-status'] + (['owner-stop'] if matches else []))
                    self.assertEqual(fixture.service.poll(), 0 if matches else None)
                finally:
                    fixture.close_fixture()

    def test_malformed_ready_still_fails_after_one_qualified_fault_cleanup(self):
        fixture = Fixture(initial=b'not a Ready JSON\n')
        with tempfile.TemporaryDirectory() as directory:
            try:
                owner = fixture.launch(Path(directory))
                with patch('_cli_serve.subprocess.Popen', side_effect=fixture.popen):
                    with self.assertRaisesRegex(RuntimeError, 'Serve Ready failed'):
                        owner.wait_ready(.5)
                self.assertEqual([call[1] for call in fixture.calls], ['serve', 'owner-status', 'owner-stop'])
                self.assertFalse(owner.record['ready'])
                self.assertTrue(owner.record['stopped'])
                self.assertIn('startupError', owner.record)
                self.assertEqual((owner.directory / 'serve.stdout.bin').read_bytes(), b'not a Ready JSON\n')
            finally:
                fixture.close_fixture()


if __name__ == '__main__':
    unittest.main()
