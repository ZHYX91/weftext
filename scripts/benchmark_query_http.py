"""Measure local Query requests against a real program and an admitted synthetic library."""

import argparse
import base64
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


def fixture_id(key):
    return str(uuid.UUID(bytes=hashlib.sha256(key.encode()).digest()[:16], version=4))


def synthetic_package(schema, size):
    workspace = fixture_id(f"weftext-query-fixture/{size}")
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


def provision(data, root, size):
    server = Server(['--data-dir', str(data)])
    try:
        request(server, '/api/library/create', {'path': str(root), 'operationId': uid()})
        anchor = created_document(request(server, '/api/note/create', {
            'root': str(root), 'parent': str(root), 'name': 'Fixture anchor', 'operationId': uid(),
        }))
        schema = request(server, '/api/data/schema', {'root': str(root)})['source']
        archive = synthetic_package(schema, size)
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path)
    parser.add_argument('output', type=Path, help='New directory outside the source checkout')
    parser.add_argument('--notes', default=1000, type=int)
    parser.add_argument('--samples', default=20, type=int)
    args = parser.parse_args()
    if not 1 <= args.notes <= 2000 or not 1 <= args.samples <= 100:
        parser.error('Use 1–2000 notes and 1–100 samples under the current inventory budget')
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
        result['fixtureArchiveSha256'] = provision(data, root, args.notes)
        measure(data, root, args.notes, args.samples, result['cases'])
        result['complete'] = True
    finally:
        (evidence / 'query-timings.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')


if __name__ == '__main__':
    main()
