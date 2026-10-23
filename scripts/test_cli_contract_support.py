"""Pure CLI fixture/diagnostic checks. No Win32, relay, worker or owner starts."""
import contextlib
import hashlib
import io
import json
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
        called = ('setup', 'workers', 'owner_start', 'query_and_export', 'bibliography_import',
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
        self.assertEqual(cases['library-read-literal-save-original']['status'], 'FAIL')
        self.assertIn('original-author-failure', cases['library-read-literal-save-original']['error'])
        self.assertEqual(cases['query-confirmed-artifact-download']['status'], 'NONRUN')
        self.assertEqual(cases['actual-hosted-http-cli']['status'], 'PASS')


if __name__ == '__main__':
    unittest.main()
