"""Pure CLI fixture/diagnostic checks. No Win32, relay, worker or owner starts."""
import contextlib
import hashlib
import io
import json
import ntpath
from pathlib import Path
import types
import unittest
from unittest import mock

import _cli_contract_diagnostics as diagnostics
from _cli_contract_fixtures import RemoteContractText, remote_contract_text
import test_cli_contract as contract


def machine(code='remoteConnection', **extra):
    return {'schemaVersion': 1, 'error': {'code': code, 'exitCode': 6, **extra}}


def line(value):
    return json.dumps(value, ensure_ascii=False, separators=(',', ':')).encode('utf-8')


class FixtureTests(unittest.TestCase):
    def test_exact_existing_authored_content_and_fixed_bounds(self):
        original = '作者原文\r\n#metadata((kind: "original")) <original>\n😀'
        fixture = remote_contract_text(original)
        tokens = [hashlib.sha256(str(index).encode('ascii')).hexdigest() for index in range(2048)]
        original_builder = original + '\n\n' + '\n\n'.join(
            ' '.join(tokens[index:index + 16]) for index in range(0, len(tokens), 16)) + '\n中文😀\n'
        self.assertIsInstance(fixture, RemoteContractText)
        self.assertEqual(fixture.source.encode('utf-8'), original_builder.encode('utf-8'))
        self.assertEqual(fixture.tokens, tuple(tokens))
        self.assertEqual(len(set(fixture.tokens)), 2048)
        self.assertEqual(fixture.maximum_paragraph_tokens, 16)
        paragraphs = fixture.source[len(original) + 2:].removesuffix('\n中文😀\n').split('\n\n')
        self.assertEqual(len(paragraphs), 128)
        self.assertEqual([len(p.split(' ')) for p in paragraphs], [16] * 128)
        self.assertFalse(any(hasattr(fixture, key) for key in ('sessionId', 'baseId', 'serial', 'passed')))


class RetainedOriginGuardTests(unittest.TestCase):
    """Only the acceptance guards: sockets, owners and all evidence reads are fake."""

    @contextlib.contextmanager
    def fixture(self, *, unexpected_ready=False, controls=None,
                io_kind='PermissionDenied', os_code=10013, error_schema=1,
                canonical_locator=None, serialized_locator=None, after_port=None):
        test = Path.cwd() / 'UNCREATED-origin-guard-test'
        output = Path.cwd() / 'UNCREATED-origin-guard-output'
        profile = test / 'retained-origin-profile'
        resolved = profile if canonical_locator is None else canonical_locator
        harness = contract.Harness(types.SimpleNamespace(
            test_root=test, output_dir=output, cli_executable=test / 'weftext-cli.exe'))
        harness.api = types.SimpleNamespace(local_app_data=mock.Mock(return_value=test / 'known-folder'))
        identity = {'profileIdentity': 'weftext.profile.physical.v2:test'}
        ready = {'token': 'test-only-token', 'base': 'http://127.0.0.1:12345',
                 'ownerIdentity': identity}
        first = types.SimpleNamespace(
            directory=test / 'first-control', ready=ready,
            record={'shutdownReceipt': {'owner': identity, 'exitCode': 0}},
            wait_ready=mock.Mock(), stop=mock.Mock(),
            process=types.SimpleNamespace(poll=mock.Mock(return_value=0)))
        blocked = types.SimpleNamespace(
            directory=test / 'blocked-control', ready=ready if unexpected_ready else None,
            record={'shutdownAttempted': False, 'controls': controls or {}},
            wait_ready=mock.Mock(side_effect=None if unexpected_ready else RuntimeError('no Ready')),
            stop=mock.Mock(), _join=mock.Mock(), _save=mock.Mock(), _write_logs=mock.Mock(),
            _stream_errors=[], process=types.SimpleNamespace(
                wait=mock.Mock(return_value=3), poll=mock.Mock(return_value=3)))
        diagnostic = {'code': 'localOriginUnavailable',
                      'message': 'The retained local origin is unavailable; no changed-origin fallback was used',
                      'phase': 'retainedBind', 'ioKind': io_kind, 'osCode': os_code}
        catalog = {'schemaVersion': 1, 'origins': [{
            'physicalIdentity': identity['profileIdentity'],
            'locator': str(resolved) if serialized_locator is None else serialized_locator, 'port': 12345}]}
        reads = {
            'host-contract.json': line({'paths': {'persistentKnownFolder': 'LocalAppData',
                'persistentRelativeRoot': 'Weftext-MVP', 'hostStateRelativePath': 'host-state'}}),
            'host-routing.json': line(catalog), 'serve.stdout.bin': b'',
            'serve.stderr.bin': line({'schemaVersion': 1, 'diagnostic': diagnostic}) + b'\n'
                + line({'schemaVersion': error_schema, 'error': {'code': 'localOriginUnavailable', 'exitCode': 3}}) + b'\n',
        }
        catalog_reads = 0

        def read(path):
            nonlocal catalog_reads
            if path.name == 'host-routing.json':
                catalog_reads += 1
                if after_port is not None and catalog_reads > 1:
                    return line({**catalog, 'origins': [{**catalog['origins'][0], 'port': after_port}]})
            return reads[path.name]

        blocker = mock.MagicMock()
        blocker.__enter__.return_value = blocker
        blocker.getsockname.return_value = ('127.0.0.1', 12345)
        with contextlib.ExitStack() as stack:
            stack.enter_context(mock.patch.object(contract.subprocess, 'Popen',
                side_effect=AssertionError('Pure guard tests must not start a process')))
            stack.enter_context(mock.patch.object(contract.socket, 'socket', return_value=blocker))
            stack.enter_context(mock.patch.object(contract.socket, 'SO_EXCLUSIVEADDRUSE', 4, create=True))
            launches = stack.enter_context(mock.patch.object(contract.cli_serve, 'OwnedServe',
                side_effect=(first, blocked)))
            stack.enter_context(mock.patch.object(contract, 'read_small', side_effect=read))
            stack.enter_context(mock.patch.object(Path, 'exists', return_value=False))
            stack.enter_context(mock.patch.object(Path, 'resolve', return_value=resolved))
            yield harness, first, blocked, launches

    def test_natural_refusal_accepts_only_matching_windows_io_pairs(self):
        for kind, code in (('PermissionDenied', 10013), ('AddrInUse', 10048)):
            with self.subTest(kind=kind), self.fixture(io_kind=kind, os_code=code,
                    controls={'owner-status': {'exitCode': 3}}) as (harness, first, blocked, launches):
                proof = harness.retained_origin_refusal()
                self.assertTrue(proof['originalOriginUnchanged'])
                self.assertEqual(proof['observedExitCode'], 3)
                self.assertEqual(launches.call_count, 2)
                first.stop.assert_called_once_with()
                blocked.stop.assert_not_called()
        with self.fixture(io_kind='AddrInUse', os_code=10013) as (harness, _, blocked, _):
            with self.assertRaisesRegex(AssertionError, 'actual Windows refusal'):
                harness.retained_origin_refusal()
            blocked.stop.assert_not_called()

    def test_unexpected_ready_is_stopped_once_and_still_fails(self):
        with self.fixture(unexpected_ready=True) as (harness, first, blocked, launches):
            with self.assertRaisesRegex(AssertionError, 'unexpectedly emitted Ready'):
                harness.retained_origin_refusal()
            first.stop.assert_called_once_with()
            blocked.stop.assert_called_once_with()
            self.assertEqual(launches.call_count, 2)
            blocked._save.assert_called_once_with()

    def test_windows_extended_locators_match_the_same_original(self):
        for plain, extended in ((r'C:\owned\state', r'\\?\C:\owned\state'),
                                (r'\\server\share\state', r'\\?\UNC\server\share\state')):
            with self.subTest(locator=extended), self.fixture(canonical_locator=plain,
                    serialized_locator=extended) as (harness, _, _, _):
                with mock.patch.object(contract.os.path, 'normcase', ntpath.normcase), \
                        mock.patch.object(contract.os.path, 'normpath', ntpath.normpath):
                    self.assertTrue(harness.retained_origin_refusal()['originalOriginUnchanged'])

    def test_boolean_standard_error_schema_does_not_equal_integer_one(self):
        with self.fixture(error_schema=True) as (harness, _, blocked, _):
            with self.assertRaisesRegex(AssertionError, 'standard error contract'):
                harness.retained_origin_refusal()
            blocked.stop.assert_not_called()

    def test_refused_start_cannot_change_its_original_origin(self):
        with self.fixture(after_port=12346) as (harness, _, blocked, _):
            with self.assertRaisesRegex(AssertionError, 'differs from the actual Ready/profile'):
                harness.retained_origin_refusal()
            blocked.stop.assert_not_called()
            self.assertNotIn('originalOriginUnchanged', harness.report['retainedOriginRefusal'])

    def test_unknown_control_cannot_pass_after_original_child_exits_three(self):
        for control in ({'outcome': 'unknown; no retry or kill'},
                        {'exitCode': 3, 'outcome': 'unknown; no retry or kill'}, {}):
            with self.subTest(control=control), self.fixture(
                    controls={'owner-status': control}) as (harness, first, blocked, launches):
                with self.assertRaisesRegex(AssertionError, 'control outcome is unknown'):
                    harness.retained_origin_refusal()
                blocked.process.wait.assert_called_once_with(timeout=15)
                first.stop.assert_called_once_with()
                blocked.stop.assert_not_called()
                self.assertEqual(launches.call_count, 2)
                self.assertEqual(blocked.record['observedExitCode'], 3)


class DiagnosticTests(unittest.TestCase):
    def setUp(self):
        self.secret = 'REGISTERED_CREDENTIAL_0123456789'
        self.hidden = 'AUTHOR_BODY_DO_NOT_PRINT'
        self.output = Path('owned-output')

    def test_stdout_http_error_and_stderr_error_only_closed_fields(self):
        public_error = {'code': 'previewUnavailable', 'explanation': {
            'key': 'error.api.hostedEditor.authorizedPreviewUnavailable',
            'params': {'author': self.hidden}}, 'diagnostics': [self.hidden], 'causes': [self.hidden]}
        stdout = line({'schemaVersion': 1, 'httpStatus': 422,
                       'result': {'error': public_error, 'source': self.hidden},
                       'argv': [self.secret], 'token': self.secret})
        stderr = line({'schemaVersion': 1, 'event': 'requestRetained', 'source': self.hidden}) + b'\n' + line(
            machine(description={'key': 'error.worker.failure', 'params': {'source': self.hidden}},
                    message=self.hidden, source=self.secret))
        with mock.patch.object(diagnostics, 'read_small', side_effect=(stdout, stderr)):
            out = diagnostics.stream_diagnostic(Path('out'), 'stdout', {self.secret})
            err = diagnostics.stream_diagnostic(Path('err'), 'stderr', {self.secret})
        self.assertEqual(out['machineErrorCount'], 1)
        self.assertEqual(json.loads(out['recentMachineErrors'][0]['text']), {
            'code': 'previewUnavailable', 'explanationKey': 'error.api.hostedEditor.authorizedPreviewUnavailable',
            'httpStatus': 422})
        self.assertEqual(err['machineErrorCount'], 1)
        self.assertEqual(err['unparsedLines'], 1)  # Valid JSON event is not a machine error.
        self.assertEqual(json.loads(err['recentMachineErrors'][0]['text']), {
            'code': 'remoteConnection', 'exitCode': 6, 'descriptionKey': 'error.worker.failure'})
        encoded = json.dumps([out, err])
        self.assertNotIn(self.secret, encoded)
        self.assertNotIn(self.hidden, encoded)
        self.assertEqual(out['sha256'], hashlib.sha256(stdout).hexdigest())

    def test_invalid_duplicate_oversize_success_and_nonobject_are_not_raw_fallbacks(self):
        rows = [b'not-json-' + self.hidden.encode(),
                b'{"schemaVersion":1,"schemaVersion":1,"error":{"code":"bad"}}',
                b'"' + b'x' * 16385 + b'"', b'\xff', b'[]',
                line({'schemaVersion': 1, 'result': {'source': self.hidden}, 'httpStatus': 200}),
                line({'schemaVersion': True, 'error': {'code': 'bad'}})]
        raw = b'\n'.join(rows)
        with mock.patch.object(diagnostics, 'read_small', return_value=raw):
            result = diagnostics.stream_diagnostic(Path('out'), 'stdout', {self.secret})
        self.assertEqual(result['machineErrorCount'], 0)
        self.assertEqual(result['unparsedLines'], len(rows))
        self.assertEqual(result['recentMachineErrors'], [])
        self.assertNotIn(self.hidden, json.dumps(result))

    def test_only_four_recent_errors_and_full_redaction_before_truncation(self):
        raw = b'\n'.join(line(machine(code='error_' + str(i))) for i in range(20))
        with mock.patch.object(diagnostics, 'read_small', return_value=raw):
            result = diagnostics.stream_diagnostic(Path('err'), 'stderr', {self.secret})
        self.assertEqual(result['machineErrorCount'], 20)
        self.assertEqual([json.loads(x['text'])['code'] for x in result['recentMachineErrors']],
                         ['error_16', 'error_17', 'error_18', 'error_19'])
        observed = diagnostics.diagnostic_text('prefix-' + self.secret + '-suffix', {self.secret}, 12)
        self.assertTrue(observed['truncated'])
        self.assertNotIn(self.secret[-5:], observed['text'])
        self.assertNotIn(self.secret[:5], observed['text'])

    def harness(self):
        return contract.Harness(types.SimpleNamespace(test_root=Path('owned-test'), output_dir=self.output))

    def test_main_entry_and_all_loaded_support_sources_are_bound(self):
        harness = self.harness()
        identity = harness.report['cliSourceIdentity']
        self.assertEqual(Path(identity['entry']['path']), Path(contract.__file__).resolve())
        self.assertEqual(identity['entry']['sha256'], hashlib.sha256(Path(contract.__file__).read_bytes()).hexdigest())
        self.assertEqual(set(identity['supportModules']), {
            '_cli_contract_common.py', '_cli_contract_windows.py', '_cli_contract_relay.py',
            '_cli_contract_diagnostics.py', '_cli_contract_fixtures.py', '_cli_serve.py', '_creation_result.py'})
        for record in identity['supportModules'].values():
            self.assertEqual(record['sha256'], hashlib.sha256(Path(record['path']).read_bytes()).hexdigest())

    def test_original_failure_and_process_slice_survive_unavailable_streams(self):
        harness = self.harness()
        harness.redactions.add(self.secret)
        earlier = {'label': 'previous-case', 'stderr': 'old.err', 'stdout': 'old.out'}
        harness.report['processes'].append(earlier)
        current = [{'label': 'current-' + str(i), 'exitCode': 3, 'expectedExitCodes': [0],
                    'stdout': str(i) + '.out', 'stderr': str(i) + '.err', 'command': self.hidden,
                    'environment': self.secret} for i in range(6)]
        preserved = json.loads(json.dumps(current))

        def fail():
            harness.report['processes'].extend(current)
            raise ValueError('original failure ' + self.secret)

        stdout, stderr = io.StringIO(), io.StringIO()
        with mock.patch.object(diagnostics, 'read_small', side_effect=PermissionError('hidden ' + self.secret)):
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                harness.case('actual-hosted-http-cli', fail)
                harness.case('artifact-roles', lambda: {'original': True})
        failed = next(x for x in harness.report['cases'] if x['name'] == 'actual-hosted-http-cli')
        self.assertEqual(failed['status'], 'FAIL')
        self.assertIn(self.secret, failed['error'])
        self.assertEqual([p['label'] for p in failed['failureDiagnostic']['processes']],
                         ['current-2', 'current-3', 'current-4', 'current-5'])
        self.assertNotIn(self.secret, stderr.getvalue())
        self.assertNotIn(self.hidden, stderr.getvalue())
        self.assertEqual(harness.report['cases'][0]['status'], 'PASS')
        self.assertEqual(current, preserved)
        self.assertEqual(harness.report['processes'][0], earlier)

    def test_write_and_flush_failure_cannot_replace_case_or_block_next_case(self):
        class BrokenWrite:
            def write(self, _):
                raise BrokenPipeError('broken write')

            def flush(self):
                raise BrokenPipeError('broken flush')

        class BrokenFlush:
            def __init__(self):
                self.flushes = 0

            def write(self, value):
                return len(value)

            def flush(self):
                self.flushes += 1
                raise BrokenPipeError('broken flush')

        for sink in (BrokenWrite(), BrokenFlush()):
            with self.subTest(sink=type(sink).__name__):
                harness = self.harness()
                with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(sink):
                    harness.case('actual-hosted-http-cli', lambda: (_ for _ in ()).throw(ValueError('original-fact')))
                    harness.case('artifact-roles', lambda: {'next': True})
                failed = next(x for x in harness.report['cases'] if x['name'] == 'actual-hosted-http-cli')
                self.assertEqual(failed['status'], 'FAIL')
                self.assertIn('original-fact', failed['error'])
                self.assertEqual(harness.report['cases'][0]['status'], 'PASS')
                if isinstance(sink, BrokenFlush):
                    self.assertEqual(sink.flushes, 2)

    def test_original_runner_keeps_failure_independent_case_and_final_cleanup(self):
        harness = self.harness()
        called = ('setup', 'workers', 'retained_origin_refusal', 'owner_start',
                  'query_and_export', 'bibliography_import',
                  'local_sync', 'conflict', 'fence', 'partial_and_budget', 'cross_profile',
                  'restart_original', 'broken_pipe', 'shells', 'conpty', 'remote')
        with contextlib.ExitStack() as stack:
            for name in called:
                stack.enter_context(mock.patch.object(harness, name, return_value={'observed': name}))
            stack.enter_context(mock.patch.object(harness, 'create_and_write', side_effect=ValueError('original-author-failure')))
            cleanup = stack.enter_context(mock.patch.object(harness, 'stop_owners'))
            stack.enter_context(mock.patch.object(harness, 'mechanism_mapping', return_value={}))
            retained = stack.enter_context(mock.patch.object(contract, 'keep_json'))
            stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
            stack.enter_context(contextlib.redirect_stderr(io.StringIO()))
            exit_code = harness.run()
        cleanup.assert_called_once_with()
        retained.assert_called_once_with(self.output / 'result.json', harness.report)
        self.assertEqual(exit_code, 1)
        self.assertFalse(harness.report['passed'])
        cases = {item['name']: item for item in harness.report['cases']}
        self.assertEqual(len(cases), 17)
        self.assertEqual(cases['retained-origin-startup-refusal']['status'], 'PASS')
        self.assertEqual(cases['retained-origin-startup-refusal']['detail'],
                         {'observed': 'retained_origin_refusal'})
        self.assertEqual(cases['library-read-literal-save-original']['status'], 'FAIL')
        self.assertIn('original-author-failure', cases['library-read-literal-save-original']['error'])
        self.assertEqual(cases['query-confirmed-artifact-download']['status'], 'NONRUN')
        self.assertEqual(cases['actual-hosted-http-cli']['status'], 'PASS')


if __name__ == '__main__':
    unittest.main()
