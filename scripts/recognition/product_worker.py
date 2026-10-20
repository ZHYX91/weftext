"""Product-owned read-only bridge; all paths and options are host frozen inputs."""
from __future__ import annotations
import argparse
import json
import copy
import os
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import worker
import schemes
import docx_reader

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--runtime', required=True, type=Path)
    parser.add_argument('--progress', type=Path)
    args = parser.parse_args()
    try:
        envelope = worker.checked_json(sys.stdin.buffer.read(256 * 1024 + 1))
        request = envelope['request']
        original = worker.read_file(Path(request['inputPath']), worker.MAX_INPUT)
        if worker.digest(original) != request['inputSha256']:
            raise worker.Refusal('input-changed')
        if envelope.get('purpose') == 'resource':
            result = {'version': 1, 'status': 'complete', 'input': {'sha256': worker.digest(original), 'bytes': len(original), 'mediaType': request['mediaType']}, 'pages': [], 'pageCount': None, 'retentionOnly': True}
        elif envelope['format'] == 'docx':
            if request.get('operation', 'recognize') in {'inspect', 'preview'}:
                raise worker.Refusal('docx-page-preview-unavailable')
            result = docx_reader.read_docx(original, request['inputSha256'])
        else:
            def progress(snapshot):
                if args.progress is None:
                    return
                data = json.dumps({'result': copy.deepcopy(snapshot), 'scheme': None}, ensure_ascii=False, allow_nan=False).encode('utf-8')
                if len(data) > worker.MAX_OUTPUT:
                    raise worker.Refusal('checkpoint-budget')
                temporary = args.progress.with_suffix('.pending')
                with temporary.open('wb') as stream:
                    stream.write(data)
                    stream.flush()
                    os.fsync(stream.fileno())
                os.replace(temporary, args.progress)
            result = worker.execute(request, args.runtime.resolve(), progress=progress)
        scheme = None
        selected = envelope.get('scheme', 'none')
        if selected != 'none' and request.get('operation', 'recognize') == 'recognize' and envelope.get('purpose') != 'resource':
            if not selected.endswith('@1') or selected[:-2] not in schemes.SCHEMES:
                raise worker.Refusal('unsupported-scheme-version')
            scheme = schemes.propose(selected[:-2], result, envelope.get('schemeOptions', {}))
        reply = {'result': result, 'scheme': scheme}
    except worker.Refusal as exc:
        reply = {'result': {'version': 1, 'status': 'failed', 'error': exc.code, 'pages': []}, 'scheme': None}
    except Exception:
        reply = {'result': {'version': 1, 'status': 'failed', 'error': 'recognition-failed', 'pages': []}, 'scheme': None}
    data = json.dumps(reply, ensure_ascii=False, allow_nan=False, separators=(',', ':')).encode('utf-8')
    if len(data) > worker.MAX_OUTPUT:
        data = b'{"result":{"version":1,"status":"failed","error":"job-output-budget","pages":[]},"scheme":null}'
    sys.stdout.buffer.write(data)

if __name__ == '__main__':
    main()
