"""Pure acquisition/selection tests; never access network or native tool APIs."""
from copy import deepcopy
from email.message import Message
import hashlib
from io import BytesIO
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch
from urllib.request import Request

import prepare_windows_package as ENTRY


def record_for(data):
    return {
        'id': 'vc-runtime', 'kind': 'vc-redist', 'filename': 'vc_redist.x64.exe',
        'sha256': hashlib.sha256(data).hexdigest(), 'size': len(data),
        'version': '14.44.35211.0', 'payloadArchitectures': ['x64'],
        'sourceUrl': 'https://aka.ms/vs/17/release/vc_redist.x64.exe',
        'finalUrl': 'https://download.visualstudio.microsoft.com/download/pr/locked/vc_redist.x64.exe',
        'publisherSubject': 'C=US, S=Washington, L=Redmond, O=Microsoft Corporation, CN=Microsoft Corporation',
        'signerCertificateSha256': 'a' * 64,
        'licenseFiles': [{'file': 'packaging/windows/licenses/vc-runtime/LICENSE.txt',
                          'sha256': 'b' * 64, 'size': 7,
                          'sourceUrl': 'https://www.microsoft.com/legal/'}],
    }


class Response:
    def __init__(self, data, url, *, lengths=(), status=200, fail_after=None):
        self.body = BytesIO(data)
        self.url, self.status, self.fail_after = url, status, fail_after
        self.headers = Message()
        for length in lengths:
            self.headers.add_header('Content-Length', str(length))
        self.read_sizes = []
        self.closed = False

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.closed = True

    def geturl(self):
        return self.url

    def read(self, size):
        self.read_sizes.append(size)
        if self.fail_after is not None and self.body.tell() >= self.fail_after:
            raise OSError('TEST original reply failed')
        return self.body.read(size)


class AcquisitionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.data = b'TEST locked installer bytes'
        self.record = record_for(self.data)

    def saved(self):
        return json.loads((self.root / 'vc-runtime.result.json').read_text('utf-8'))

    def exercise(self, response, *, record=None, clock=lambda: 0):
        opener = Mock(return_value=response)
        result = ENTRY.download(record or self.record, self.root, opener=opener, clock=clock)
        return result, opener

    def test_exact_original_uses_pinned_final_url_and_keeps_real_response_provenance(self):
        original = deepcopy(self.record)
        redirected = 'https://download.microsoft.com/locked/vc_redist.x64.exe'
        response = Response(self.data, redirected, lengths=[len(self.data)])
        result, opener = self.exercise(response)
        request = opener.call_args.args[0]
        self.assertEqual(request.full_url, self.record['finalUrl'])
        self.assertNotEqual(request.full_url, self.record['sourceUrl'])
        self.assertEqual(request.get_header('Accept-encoding'), 'identity')
        opener.assert_called_once()
        self.assertEqual((self.root / self.record['filename']).read_bytes(), self.data)
        self.assertEqual(result, self.saved())
        self.assertEqual(result['responseUrl'], redirected)
        self.assertEqual(result['downloadedSha256'], self.record['sha256'])
        self.assertEqual(result['status'], 'downloadedExactOriginal')
        self.assertFalse(result['installerExecuted'])
        self.assertTrue(response.closed)
        self.assertEqual(self.record, original)

    def test_chunked_exact_original_without_content_length_is_still_bounded(self):
        response = Response(self.data, self.record['finalUrl'])
        result, _ = self.exercise(response)
        self.assertEqual(result['downloadedBytes'], len(self.data))
        self.assertLessEqual(max(response.read_sizes), len(self.data) + 1)
        self.assertEqual(response.body.tell(), len(self.data))

    def test_redirect_handler_rejects_nonofficial_or_nonhttps_targets(self):
        handler = ENTRY.OfficialRedirects()
        request = Request(self.record['finalUrl'])
        for url in ('https://example.invalid/file.exe', 'http://download.microsoft.com/file.exe',
                    'https://user:secret@download.microsoft.com/file.exe',
                    'https://download.microsoft.com/file.exe#fragment'):
            with self.subTest(url=url), self.assertRaises(ValueError):
                handler.redirect_request(request, None, 302, 'Found', {}, url)
        redirected = handler.redirect_request(request, None, 302, 'Found', {},
                                             'https://download.microsoft.com/file.exe')
        self.assertEqual(redirected.full_url, 'https://download.microsoft.com/file.exe')

    def test_final_response_url_is_rechecked_even_with_a_substituted_opener(self):
        response = Response(self.data, 'https://example.invalid/file.exe')
        with self.assertRaises(ValueError):
            self.exercise(response)
        self.assertEqual(response.read_sizes, [])
        self.assertFalse((self.root / self.record['filename']).exists())
        self.assertEqual(self.saved()['status'], 'failedOriginalRetained')

    def test_http_error_does_not_create_or_read_installer(self):
        response = Response(self.data, self.record['finalUrl'], status=503)
        with self.assertRaises(ValueError):
            self.exercise(response)
        self.assertFalse((self.root / self.record['filename']).exists())
        self.assertEqual(response.read_sizes, [])
        self.assertEqual(self.saved()['downloadedBytes'], 0)

    def test_conflicting_or_duplicate_lengths_fail_before_body_read(self):
        for index, lengths in enumerate(([len(self.data) + 1], [len(self.data), len(self.data)])):
            folder = self.root / str(index)
            folder.mkdir()
            response = Response(self.data, self.record['finalUrl'], lengths=lengths)
            with self.subTest(lengths=lengths), self.assertRaises(ValueError):
                ENTRY.download(self.record, folder, opener=lambda _: response, clock=lambda: 0)
            self.assertEqual(response.read_sizes, [])
            self.assertFalse((folder / self.record['filename']).exists())
            self.assertEqual(json.loads((folder / 'vc-runtime.result.json').read_text('utf-8'))['status'],
                             'failedOriginalRetained')

    def test_short_reply_retains_its_actual_partial_bytes(self):
        partial = self.data[:-3]
        response = Response(partial, self.record['finalUrl'], lengths=[len(self.data)])
        with self.assertRaises(ValueError):
            self.exercise(response)
        self.assertEqual((self.root / self.record['filename']).read_bytes(), partial)
        self.assertEqual(self.saved()['downloadedBytes'], len(partial))
        self.assertEqual(self.saved()['downloadedSha256'], hashlib.sha256(partial).hexdigest())
        self.assertEqual(self.saved()['status'], 'failedOriginalRetained')

    def test_oversized_reply_consumes_only_one_excess_byte_and_retains_failure(self):
        response = Response(self.data + b'EXCESS' * 1000, self.record['finalUrl'])
        with self.assertRaises(ValueError):
            self.exercise(response)
        consumed = self.data + b'E'
        self.assertEqual(response.body.tell(), len(self.data) + 1)
        self.assertEqual((self.root / self.record['filename']).read_bytes(), consumed)
        self.assertEqual(self.saved()['downloadedBytes'], len(consumed))
        self.assertEqual(self.saved()['status'], 'failedOriginalRetained')

    def test_equal_length_wrong_hash_is_not_an_accepted_installer(self):
        wrong = b'X' * len(self.data)
        response = Response(wrong, self.record['finalUrl'], lengths=[len(wrong)])
        with self.assertRaises(ValueError):
            self.exercise(response)
        self.assertEqual((self.root / self.record['filename']).read_bytes(), wrong)
        self.assertEqual(self.saved()['downloadedSha256'], hashlib.sha256(wrong).hexdigest())
        self.assertEqual(self.saved()['status'], 'failedOriginalRetained')

    def test_reply_exception_preserves_partial_and_does_not_retry(self):
        response = Response(self.data, self.record['finalUrl'], fail_after=4)
        opener = Mock(return_value=response)
        with patch.object(ENTRY, 'CHUNK', 4), self.assertRaisesRegex(OSError, 'original reply failed'):
            ENTRY.download(self.record, self.root, opener=opener, clock=lambda: 0)
        opener.assert_called_once()
        self.assertEqual((self.root / self.record['filename']).read_bytes(), self.data[:4])
        self.assertEqual(self.saved()['downloadedBytes'], 4)
        self.assertEqual(self.saved()['status'], 'failedOriginalRetained')
        originals = {path.name: path.read_bytes() for path in self.root.iterdir()}
        forbidden = Mock(side_effect=AssertionError('must not issue another request'))
        with self.assertRaises(FileExistsError):
            ENTRY.download(self.record, self.root, opener=forbidden, clock=lambda: 0)
        forbidden.assert_not_called()
        self.assertEqual({path.name: path.read_bytes() for path in self.root.iterdir()}, originals)

    def test_existing_file_is_never_overwritten_or_adopted(self):
        previous = b'TEST existing unknown original'
        (self.root / self.record['filename']).write_bytes(previous)
        response = Response(self.data, self.record['finalUrl'])
        opener = Mock(return_value=response)
        with self.assertRaises(FileExistsError):
            ENTRY.download(self.record, self.root, opener=opener, clock=lambda: 0)
        opener.assert_not_called()
        self.assertEqual((self.root / self.record['filename']).read_bytes(), previous)
        self.assertEqual(response.read_sizes, [])
        self.assertEqual([path.name for path in self.root.iterdir()], [self.record['filename']])

    def test_deadline_does_not_reset_when_the_response_opens(self):
        response = Response(self.data, self.record['finalUrl'])
        ticks = iter([0, ENTRY.DOWNLOAD_SECONDS + 1])
        with self.assertRaises(TimeoutError):
            self.exercise(response, clock=lambda: next(ticks))
        self.assertEqual(response.read_sizes, [])
        self.assertEqual(self.saved()['downloadedBytes'], 0)
        self.assertEqual(self.saved()['status'], 'failedOriginalRetained')

    def test_readback_mismatch_cannot_promote_received_bytes_to_success(self):
        response = Response(self.data, self.record['finalUrl'])
        with patch.object(ENTRY, 'read_checked', return_value=b'TEST changed after write'):
            with self.assertRaisesRegex(ValueError, 'readback differs'):
                self.exercise(response)
        self.assertEqual(self.saved()['status'], 'failedOriginalRetained')
        self.assertEqual((self.root / self.record['filename']).read_bytes(), self.data)


class LockedRecordTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.records = []
        for kind in ('vc-runtime', 'webview2'):
            record = record_for(b'TEST ' + kind.encode())
            if kind == 'webview2':
                record.update(id=kind, kind='evergreen-standalone',
                              filename='MicrosoftEdgeWebView2RuntimeInstallerX64.exe',
                              version='155.0.3530.61')
            license_path = self.root / 'packaging/windows/licenses' / kind / 'LICENSE.txt'
            license_path.parent.mkdir(parents=True)
            license_bytes = ('TEST original license ' + kind).encode()
            license_path.write_bytes(license_bytes)
            record['licenseFiles'] = [{
                'file': license_path.relative_to(self.root).as_posix(),
                'sha256': hashlib.sha256(license_bytes).hexdigest(), 'size': len(license_bytes),
                'sourceUrl': 'https://www.microsoft.com/legal/'}]
            self.records.append(record)
        self.lock = self.root / 'packaging/windows/offline-materials.json'
        self.write_lock()

    def write_lock(self):
        self.lock.write_text(json.dumps({'schemaVersion': 1, 'materials': self.records}), 'utf-8')

    def test_locked_set_binds_the_original_license_bytes_before_acquisition(self):
        data, records = ENTRY.locked_records(self.root)
        self.assertEqual(data, self.lock.read_bytes())
        self.assertEqual({record['id'] for record in records}, {'vc-runtime', 'webview2'})
        path = self.root / self.records[0]['licenseFiles'][0]['file']
        path.write_bytes(b'TEST changed original license')
        with self.assertRaisesRegex(ValueError, 'license original changed'):
            ENTRY.locked_records(self.root)

    def test_duplicate_material_cannot_stand_in_for_the_missing_other_prerequisite(self):
        self.records[1] = deepcopy(self.records[0])
        self.write_lock()
        with self.assertRaisesRegex(ValueError, 'complete locked material set'):
            ENTRY.locked_records(self.root)


class InstalledToolTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def linker(self, name, *, libraries=True, headers=True):
        tools = self.root / 'VS/VC/Tools/MSVC' / name
        path = tools / 'bin/Hostx64/x64/link.exe'
        path.parent.mkdir(parents=True)
        path.write_bytes(('TEST linker ' + name).encode())
        if libraries:
            (tools / 'lib/x64').mkdir(parents=True)
        if headers:
            (tools / 'include').mkdir()
        return path

    def sdk(self, version, *, omit=()):
        kits = self.root / 'Kits'
        directory = kits / 'bin' / version / 'x64'
        directory.mkdir(parents=True)
        for name in ('makeappx.exe', 'makepri.exe', 'rc.exe'):
            if name not in omit:
                (directory / name).write_bytes(('TEST SDK ' + name).encode())
        for name in ('ucrt', 'um', 'shared', 'winrt'):
            if 'include/' + name not in omit:
                (kits / 'Include' / version / name).mkdir(parents=True)
        for name in ('ucrt', 'um'):
            if 'lib/' + name not in omit:
                (kits / 'Lib' / version / name / 'x64').mkdir(parents=True)
        return kits, directory

    def test_linker_selection_uses_actual_version_not_directory_name(self):
        old = self.linker('99-misleading-old')
        selected = self.linker('00-actual-compatible')
        newer = self.linker('14-runtime-too-new')
        x86 = self.linker('15-wrong-architecture')
        incomplete = self.linker('16-no-libraries', libraries=False)
        versions = {old: (14, 42, 1, 0), selected: (14, 44, 35207, 0),
                    newer: (14, 44, 35212, 0), x86: (14, 44, 35210, 0),
                    incomplete: (14, 43, 1, 0)}
        images = {path.read_bytes(): (0x14c if path == x86 else 0x8664) for path in versions}
        with patch.object(ENTRY, 'file_version', side_effect=versions.__getitem__), \
                patch.object(ENTRY, 'pe_info', side_effect=lambda data: {'machine': images[data]}):
            actual, rejected = ENTRY.select_linker([self.root / 'VS'], '14.44.35211.0')
        self.assertEqual(actual['path'], str(selected))
        self.assertEqual(actual['fileVersion'], list(versions[selected]))
        self.assertEqual(actual['sha256'], hashlib.sha256(selected.read_bytes()).hexdigest())
        self.assertEqual({row['path'] for row in rejected}, {str(newer), str(x86), str(incomplete)})

    def test_linker_requires_both_headers_and_x64_libraries(self):
        self.linker('no-headers', headers=False)
        self.linker('no-libraries', libraries=False)
        with patch.object(ENTRY, 'file_version', return_value=(14, 42, 1, 0)), \
                patch.object(ENTRY, 'pe_info', return_value={'machine': 0x8664}):
            with self.assertRaisesRegex(ValueError, 'No installed x64 MSVC'):
                ENTRY.select_linker([self.root / 'VS'], '14.44.35211.0')

    def test_linker_actual_version_query_error_is_not_an_eligible_fallback(self):
        self.linker('unknown-version')
        with patch.object(ENTRY, 'file_version', side_effect=OSError('TEST version query denied')), \
                patch.object(ENTRY, 'pe_info', return_value={'machine': 0x8664}):
            with self.assertRaisesRegex(OSError, 'version query denied'):
                ENTRY.select_linker([self.root / 'VS'], '14.44.35211.0')

    def test_sdk_chooses_newest_complete_candidate_and_qualifies_its_real_tools(self):
        kits, _ = self.sdk('10.0.22000.0')
        _, selected = self.sdk('10.0.22621.0')
        self.sdk('10.0.26100.0', omit=['rc.exe'])
        self.sdk('10.0.27000.0', omit=['include/winrt'])
        self.sdk('10.0.28000.0', omit=['lib/ucrt'])
        self.sdk('not-a-version')
        qualification = {name: {'path': selected / name, 'fileVersion': [10, 0, 22621, 1],
                                 'bytes': (selected / name).stat().st_size,
                                 'sha256': hashlib.sha256((selected / name).read_bytes()).hexdigest()}
                         for name in ('makeappx.exe', 'makepri.exe', 'rc.exe')}
        with patch.object(ENTRY, 'sdk_tools', return_value=qualification) as qualify:
            actual = ENTRY.select_sdk(kits)
        qualify.assert_called_once_with(selected)
        self.assertEqual(actual['directory'], str(selected))
        self.assertEqual(actual['version'], '10.0.22621.0')
        self.assertEqual(set(actual['tools']), {'makeappx.exe', 'makepri.exe', 'rc.exe'})
        self.assertEqual(actual['tools']['makeappx.exe']['path'], str(selected / 'makeappx.exe'))

    def test_no_complete_sdk_cannot_call_tool_qualification(self):
        kits, _ = self.sdk('10.0.26100.0', omit=['include/shared'])
        with patch.object(ENTRY, 'sdk_tools') as qualify:
            with self.assertRaisesRegex(ValueError, 'No complete installed'):
                ENTRY.select_sdk(kits)
        qualify.assert_not_called()

    def test_selected_sdk_native_qualification_failure_remains_a_failure(self):
        kits, _ = self.sdk('10.0.26100.0')
        with patch.object(ENTRY, 'sdk_tools', side_effect=ValueError('TEST wrong PE architecture')):
            with self.assertRaisesRegex(ValueError, 'wrong PE architecture'):
                ENTRY.select_sdk(kits)

    def test_compiler_environment_contains_only_selected_headers_and_libraries(self):
        link = self.linker('selected')
        kits, sdk_bin = self.sdk('10.0.26100.0')
        tools = link.parents[3]
        cppwinrt = kits / 'Include/10.0.26100.0/cppwinrt'
        cppwinrt.mkdir()
        incoming = {'PATH': 'TEST original PATH', 'LIB': 'TEST unrelated old library',
                    'INCLUDE': 'TEST unrelated old include', 'UNRELATED': 'retained by caller'}
        before = deepcopy(incoming)
        result = ENTRY.compiler_environment(
            {'path': str(link), 'toolsRoot': str(tools)},
            {'directory': str(sdk_bin), 'kitsRoot': str(kits), 'version': '10.0.26100.0'}, incoming)
        self.assertEqual(incoming, before)
        self.assertEqual(result['CARGO_BUILD_TARGET'], 'x86_64-pc-windows-msvc')
        self.assertEqual(result['CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER'], str(link))
        self.assertEqual(result['WEFTEXT_WINDOWS_SDK_BIN'], str(sdk_bin))
        self.assertEqual(result['PATH'], os.pathsep.join([str(link.parent), str(sdk_bin), incoming['PATH']]))
        self.assertEqual(result['LIB'].split(os.pathsep), list(map(str, [
            tools / 'lib/x64', kits / 'Lib/10.0.26100.0/ucrt/x64', kits / 'Lib/10.0.26100.0/um/x64'])))
        self.assertEqual(result['INCLUDE'].split(os.pathsep), list(map(str, [
            tools / 'include', *[kits / 'Include/10.0.26100.0' / name
                                for name in ('ucrt', 'um', 'shared', 'winrt')], cppwinrt])))
        self.assertNotIn('UNRELATED', result)


if __name__ == '__main__':
    unittest.main()
