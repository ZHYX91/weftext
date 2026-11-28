"""Measure local Query requests against a real program and an admitted synthetic library."""

import argparse
import base64
import ctypes
from ctypes import wintypes
import hashlib
import io
import json
import math
import os
from pathlib import Path
import platform
import statistics
import subprocess
import time
import uuid
import zipfile

import test_hosted
from _creation_result import created_document
from test_hosted import Server, uid


class ProcessIo(ctypes.Structure):
    _fields_ = [(name, ctypes.c_uint64) for name in (
        'reads', 'writes', 'other', 'readBytes', 'writeBytes', 'otherBytes')]


def process_counters(pid):
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    kernel.OpenProcess.restype = wintypes.HANDLE
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    kernel.GetProcessTimes.argtypes = [wintypes.HANDLE] + [ctypes.POINTER(wintypes.FILETIME)] * 4
    kernel.GetProcessIoCounters.argtypes = [wintypes.HANDLE, ctypes.POINTER(ProcessIo)]
    handle = kernel.OpenProcess(0x1000, False, pid)  # PROCESS_QUERY_LIMITED_INFORMATION
    if not handle:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        times = [wintypes.FILETIME() for _ in range(4)]
        io = ProcessIo()
        if (not kernel.GetProcessTimes(handle, *(ctypes.byref(t) for t in times))
                or not kernel.GetProcessIoCounters(handle, ctypes.byref(io))):
            raise ctypes.WinError(ctypes.get_last_error())
        kernel_ms, user_ms = [(t.dwHighDateTime << 32 | t.dwLowDateTime) / 10_000 for t in times[2:]]
        return {'kernelMs': kernel_ms, 'userMs': user_ms,
                **{name: getattr(io, name) for name, _ in io._fields_}}
    finally:
        kernel.CloseHandle(handle)


def profile_query(server, root, case, predicate, expected):
    counters = process_counters(server.process.pid)
    start = time.perf_counter_ns()
    status, outcome, headers = server.call('/api/query/run', {
        'root': str(root), 'plan': {'types': ['task'], 'where': predicate,
            'select': [{'name': 'status', 'expr': 'status'}], 'take': 100},
    }, headers={'X-Weftext-Query-Profile': '1'})
    elapsed = (time.perf_counter_ns() - start) / 1_000_000
    end = process_counters(server.process.pid)
    if status != 200 or outcome['coverage'] != 'complete' or outcome['totalKnown'] != expected:
        raise ValueError(f'Profile control {case} returned unexpected status, coverage or count')
    result = {'case': case, 'ms': elapsed,
              'serverCounters': {name: value - counters[name] for name, value in end.items()},
              'serverTiming': headers.get('Server-Timing')}
    if case == 'profile-body':
        result['index'] = next(v for v in outcome['inputVersions'] if v.get('role') == 'nativeQueryIndex')
        if result['index']['misses'] or result['index']['built']:
            raise ValueError(f'Profile control {case} was not warm')
    return result


def fixture_id(key):
    return str(uuid.UUID(bytes=hashlib.sha256(key.encode()).digest()[:16], version=4))


def synthetic_package(schema, size, *, full_text=False):
    workspace = fixture_id(f"weftext-query-fixture/{size}" + ("/full-text" if full_text else ""))
    files = {'.weftext-schema.json': schema.encode('utf-8')}
    entries = {'.weftext-schema.json': {'role': 'schema'}}
    for index in range(size):
        node = fixture_id(f'{workspace}/{index}')
        name = f'Task-{index:06}'
        status = 'open' if index % 2 == 0 else 'done'
        path = f'{name}/{name}.typ'
        source = (f'#metadata((id: "{node}", types: ("task",), status: "{status}")) <weftext>\n'
                  f'#set document(title: [任务 Task {index}])\n#title()\n\n'
                  + '会议纪要 meeting notes. ' * 8 + '\n')
        if full_text:
            source += f'词条 term{index:06} ' + ('needle search' if index % 2 == 0 else 'haystack') + '\n'
            if index in (0, size - 1):
                source += '#include "../shared-query-text.typ"\n'
        files[path] = source.encode('utf-8')
        entries[path] = {'role': 'source', 'nodeId': node}

    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, 'w') as archive:
        for path, value in files.items():
            entries[path].update(bytes=len(value), sha256=hashlib.sha256(value).hexdigest())
            archive.writestr(zipfile.ZipInfo(path), value, compress_type=zipfile.ZIP_DEFLATED)
        manifest = dict(format='weftext.native-package', version=2,
                        packageId=fixture_id(f'{workspace}/package'), workspaceId=workspace,
                        scope='library', schemaPath='.weftext-schema.json', losses=[], entries=entries)
        archive.writestr(zipfile.ZipInfo('manifest.json'), json.dumps(manifest), compress_type=zipfile.ZIP_DEFLATED)
    return buffer.getvalue()


def request(server, route, payload):
    status, response, _ = server.call(route, payload)
    if status != 200:
        raise RuntimeError(f'{route} returned {status}: {str(response)[:800]}')
    return response


def provision(data, root, size, *, full_text=False):
    server = Server(['--data-dir', str(data)])
    try:
        request(server, '/api/library/create', {'path': str(root), 'operationId': uid()})
        anchor = created_document(request(server, '/api/note/create', {
            'root': str(root), 'parent': str(root), 'name': 'Fixture anchor', 'operationId': uid(),
        }))
        schema = request(server, '/api/data/schema', {'root': str(root)})['source']
        archive = synthetic_package(schema, size, full_text=full_text)
        plan = request(server, '/api/exchange/prepare', {
            'operationId': uid(), 'sessionId': anchor['sessionId'], 'baseId': anchor['baseId'],
            'root': str(root), 'parent': str(root), 'name': 'Synthetic', 'format': 'native',
            'bytesBase64': base64.b64encode(archive).decode('ascii'),
        })
        committed = request(server, '/api/exchange/commit', {
            'operationId': plan['operationId'], 'planId': plan['planId'], 'acceptLosses': True,
        })
        if committed['status'] != 'committed':
            raise RuntimeError('The synthetic package was not admitted')
        if full_text:
            (root / 'Synthetic/shared-query-text.typ').write_text('shared baseline\n', encoding='utf-8')
        return hashlib.sha256(archive).hexdigest()
    finally:
        server.close()


def measure(data, root, size, samples, output):
    status = {'name': 'status', 'expr': 'status'}
    order = [{'expr': 'status', 'descending': False, 'nulls': 'last'}]
    cases = [
        ('scalar-filter', {'select': [status], 'where': 'status == "open"'}, (size + 1) // 2),
        ('ordered-projection', {'select': [status], 'order': order}, size),
        ('grouped-count', {'group': {'keys': [status], 'aggregates': [{'name': 'notes', 'op': 'count'}]}},
         min(2, size)),
    ]
    for name, plan, expected in cases:
        server = Server(['--data-dir', str(data)])
        try:
            timings = []
            for _ in range(samples + 1):
                start = time.perf_counter_ns()
                result = request(server, '/api/query/run', {
                    'root': str(root), 'plan': {'types': ['task'], 'take': 100, **plan},
                })
                timings.append((time.perf_counter_ns() - start) / 1_000_000)
                if (result['coverage'] != 'complete' or result.get('externalPending')
                        or result['totalKnown'] != expected or len(result['rows']) != min(expected, 100)):
                    output.append({
                        'case': name, 'requestMs': timings[-1], 'coverage': result['coverage'],
                        'totalKnown': result['totalKnown'], 'expectedRowsBeforeTake': expected,
                        'externalPending': result.get('externalPending'),
                        'diagnostics': [item.get('explanation', {}).get('key')
                                        for item in result.get('diagnostics', [])[:12]],
                    })
                    raise RuntimeError(f'{name} returned unexpected coverage or row totals')
                if name == 'grouped-count':
                    counts = {row['cells']['status']['value']['value']:
                              int(row['cells']['notes']['value']['value']) for row in result['rows']}
                    expected_counts = {'open': (size + 1) // 2}
                    if size > 1:
                        expected_counts['done'] = size // 2
                    if counts != expected_counts:
                        raise RuntimeError(f'{name} returned unexpected group counts')
            peak = subprocess.check_output([
                'powershell.exe', '-NoProfile', '-Command',
                f'(Get-Process -Id {server.process.pid}).PeakWorkingSet64',
            ], text=True, timeout=10)
        finally:
            server.close()
        warm = sorted(timings[1:])
        output.append({'case': name, 'firstRequestMs': timings[0], 'warmSamplesMs': timings[1:],
                       'p50Ms': statistics.median(warm), 'p95Ms': warm[math.ceil(.95 * samples) - 1],
                       'serverPeakWorkingSetBytes': int(peak), 'expectedRowsBeforeTake': expected})


def measure_full_text(data, root, size, samples, output, *, profile=False):
    if size < 40:
        raise ValueError('Use at least 40 notes to exercise coverage beyond the former limit')

    def measured(server, name, terms, expected, expected_misses):
        start = time.perf_counter_ns()
        outcome = request(server, '/api/query/run', {'root': str(root), 'plan': {
            'types': ['task'], 'where': f'textMatch(body, "{terms}", "phrase")',
            'select': [{'name': 'status', 'expr': 'status'}], 'take': 100,
        }})
        elapsed = (time.perf_counter_ns() - start) / 1_000_000
        index = next((v for v in outcome.get('inputVersions', []) if v.get('role') == 'nativeQueryIndex'), {})
        sample = {'case': name, 'ms': elapsed, 'coverage': outcome['coverage'],
                  'matches': outcome['totalKnown'], 'expectedMatches': expected,
                  'index': index, 'expectedMisses': expected_misses}
        output.append(sample)
        if (outcome['coverage'] != 'complete' or outcome.get('externalPending')
                or outcome['totalKnown'] != expected or len(outcome['rows']) != min(expected, 100)
                or index.get('misses') != expected_misses or index.get('built') != expected_misses):
            raise ValueError(f'Unexpected full-text coverage or incremental work for {name}: {sample}')
        return elapsed

    server = Server(['--data-dir', str(data)])
    try:
        expected = (size + 1) // 2
        measured(server, 'cold', 'needle search', expected, size + 1)
        warm = [measured(server, 'warm', 'needle search', expected, 0) for _ in range(samples)]
        if profile:
            controls = [('profile-scalar', 'status == "open"'),
                        ('profile-body', 'textMatch(body, "needle search", "phrase")')]
            for serial in range(3):
                for case, predicate in controls[::1 if serial % 2 == 0 else -1]:
                    output.append(profile_query(server, root, case, predicate, expected))
        measured(server, 'last-document', f'term{size-1:06}', 1, 0)
        name = f'Task-{size-1:06}'
        path = root / 'Synthetic' / name / f'{name}.typ'
        document = request(server, '/api/open', {'root': str(root), 'path': str(path)})
        request(server, '/api/save', {'sessionId': document['sessionId'], 'baseId': document['baseId'],
                                      'source': document['source'] + '\nnew unique phrase\n',
                                      'operationId': uid()})
        measured(server, 'single-edit', 'new unique phrase', 1, 1)
        (root / 'Synthetic/shared-query-text.typ').write_text('shared revision\n', encoding='utf-8')
        measured(server, 'shared-dependency-edit', 'shared revision', 2, 2)
        measured(server, 'complete-no-match', 'shared baseline', 0, 0)
        peak = subprocess.check_output([
            'powershell.exe', '-NoProfile', '-Command',
            f'(Get-Process -Id {server.process.pid}).PeakWorkingSet64',
        ], text=True, timeout=10)
    finally:
        server.close()
    server = Server(['--data-dir', str(data)])
    try:
        measured(server, 'restart-warm', 'shared revision', 2, 0)
    finally:
        server.close()
    if size == 40:
        (data / 'query-products.json').write_bytes(b'{"format":"damaged synthetic index"}')
        server = Server(['--data-dir', str(data)])
        try:
            measured(server, 'rebuild-corrupt-index', 'shared revision', 2, size + 1)
            measured(server, 'rebuilt-index-reused', 'shared revision', 2, 0)
        finally:
            server.close()
    ordered = sorted(warm)
    return {'p50Ms': statistics.median(warm), 'p95Ms': ordered[math.ceil(.95 * samples) - 1],
            'serverPeakWorkingSetBytes': int(peak)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path)
    parser.add_argument('output', type=Path, help='New directory outside the source checkout')
    parser.add_argument('--notes', default=1000, type=int)
    parser.add_argument('--samples', default=20, type=int)
    parser.add_argument('--full-text', action='store_true', help='Exercise native body coverage and incremental index changes')
    parser.add_argument('--profile', action='store_true', help='Record paired Windows process counters for scalar and warm native reads')
    args = parser.parse_args()
    if not 1 <= args.notes <= 2000 or not 1 <= args.samples <= 100:
        parser.error('Use 1–2000 notes and 1–100 samples under the current inventory budget')
    if args.profile and (not args.full_text or os.name != 'nt'):
        parser.error('--profile requires a Windows full-text run')
    evidence = args.output.resolve()
    if evidence.is_relative_to(Path(__file__).resolve().parents[1]):
        parser.error('Store benchmark evidence outside the source checkout')
    test_hosted.EXE = args.executable.resolve(strict=True)
    with test_hosted.EXE.open('rb') as binary:
        executable_hash = hashlib.file_digest(binary, 'sha256').hexdigest()
    evidence.mkdir()
    data, root = evidence / 'state', evidence / 'Library'
    result = {'executableSha256': executable_hash, 'platform': platform.platform(), 'notes': args.notes,
              'logicalCpuCount': os.cpu_count(), 'samplesPerCase': args.samples, 'cases': [], 'complete': False}
    try:
        result['fixtureArchiveSha256'] = provision(data, root, args.notes, full_text=args.full_text)
        if args.full_text:
            result['fullText'] = measure_full_text(data, root, args.notes, args.samples, result['cases'], profile=args.profile)
        else:
            measure(data, root, args.notes, args.samples, result['cases'])
        result['complete'] = True
    finally:
        (evidence / 'query-timings.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')


if __name__ == '__main__':
    main()
