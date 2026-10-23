"""Background Mermaid -> real local DOCX acceptance with retained owned inputs.

Usage: python scripts/test_diagram_docx.py EXE NEW_OUTPUT --evidence DIAGRAM_DIR
The evidence directory supplies diagram.typ and diagram.png from the separate
browser renderer acceptance. This script neither opens Office nor regenerates
that PNG. It checks the executable's real output chain and the product's native
source repair helper; no desktop input, network fonts or external workspace is
used. Rerun the same script on the final packaged executable in a new directory.
"""
import argparse
import base64
import hashlib
import io
import json
from pathlib import Path
import posixpath
import re
import shutil
import struct
import subprocess
import sys
import unittest
import xml.etree.ElementTree as ET
import zipfile

import test_local_features as features


NS = {
    'w': 'http://schemas.openxmlformats.org/wordprocessingml/2006/main',
    'wp': 'http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing',
    'a': 'http://schemas.openxmlformats.org/drawingml/2006/main',
    'r': 'http://schemas.openxmlformats.org/officeDocument/2006/relationships',
    'rel': 'http://schemas.openxmlformats.org/package/2006/relationships',
}
REPORT = {}
ARGS = None


def sha(value):
    return hashlib.sha256(value).hexdigest()


def keep_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding='utf-8')


class DiagramDocx(features.LocalFeatures):
    def compile(self, source, name, paged=False):
        request = {'source': source, 'path': str(self.author_root / 'diagram.typ'),
                   'root': str(self.author_root), 'frozen_files': {}, 'paged': paged}
        completed = subprocess.run([str(features.EXE), '--render-worker'],
                                   input=json.dumps(request, ensure_ascii=False),
                                   capture_output=True, text=True, encoding='utf-8',
                                   timeout=60, creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        (self.root / f'{name}.stderr.log').write_text(completed.stderr, encoding='utf-8')
        self.assertEqual(completed.returncode, 0, completed.stderr)
        result = json.loads(completed.stdout)
        keep_json(self.root / f'{name}.json', result)
        return result

    def generate(self, doc, source, fmt, name, serial=1):
        request = {'outputLocale': 'zh-CN', 'requestId': features.uid(), 'sessionId': doc['sessionId'],
                   'baseId': doc['baseId'], 'source': source, 'serial': serial,
                   'format': fmt, 'stylePolicy': 'template', 'bindings': {}}
        result = self.api('/api/export/prepare', request)
        keep_json(self.root / f'{name}-preview.json', result)
        self.assertEqual(result['status'], 'prepared', result)
        self.assertEqual(result['requestId'], request['requestId'])
        confirmed = self.api('/api/export/confirm', {'id': result['id'], 'acceptLosses': True})
        self.assertEqual(confirmed['status'], 'confirmed')
        _, data, headers = self.raw('/api/export/download', {'id': result['id']})
        self.assertEqual(len(data), result['length'])
        self.assertEqual(sha(data), result['digest'])
        self.assertEqual(headers['Cache-Control'], 'no-store')
        (self.root / name).write_bytes(data)
        return request, result, data

    def inspect_docx(self, data, png, author_source, name):
        with zipfile.ZipFile(io.BytesIO(data)) as package:
            parts = {entry: package.read(entry) for entry in package.namelist()}
        media = [path for path in parts if path.startswith('word/media/')]
        self.assertEqual(len(media), 1, media)
        self.assertTrue(media[0].endswith('.png'), media)
        self.assertEqual(parts[media[0]], png, 'DOCX must contain the exact confirmed browser PNG')
        width, height = struct.unpack('>II', png[16:24])
        document = ET.fromstring(parts['word/document.xml'])
        relationships = ET.fromstring(parts['word/_rels/document.xml.rels'])
        inline = document.findall('.//wp:inline', NS)
        self.assertEqual(len(inline), 1)
        properties = inline[0].find('wp:docPr', NS)
        self.assertIsNotNone(properties)
        # XML 1.0 normalizes literal attribute line breaks/tabs to spaces.
        # DOCX is the required picture fallback, not a Mermaid-source format;
        # exact editable author text is verified in the native .typ export.
        normalized_alt = author_source.translate(str.maketrans({'\n': ' ', '\r': ' ', '\t': ' '}))
        self.assertEqual(properties.attrib.get('descr'), normalized_alt)
        blip = inline[0].find('.//a:blip', NS)
        self.assertIsNotNone(blip)
        relationship_id = blip.attrib[f'{{{NS["r"]}}}embed']
        image_rel = next(rel for rel in relationships if rel.attrib['Id'] == relationship_id)
        self.assertEqual(image_rel.attrib['Type'], NS['r'] + '/image')
        self.assertNotEqual(image_rel.attrib.get('TargetMode'), 'External')
        resolved = posixpath.normpath(posixpath.join('word', image_rel.attrib['Target']))
        self.assertEqual(resolved, media[0])
        types = ET.fromstring(parts['[Content_Types].xml'])
        self.assertTrue(any(item.attrib.get('ContentType') == 'image/png'
                            and (item.attrib.get('PartName') == '/' + media[0]
                                 or item.attrib.get('Extension') == 'png') for item in types))
        extent = inline[0].find('wp:extent', NS)
        cx, cy = int(extent.attrib['cx']), int(extent.attrib['cy'])
        self.assertGreater(cx, 0); self.assertGreater(cy, 0)
        self.assertLessEqual(cx, 5_486_400)
        self.assertAlmostEqual(cx / cy, width / height, delta=0.0001)
        image_extent = inline[0].find('.//a:xfrm/a:ext', NS)
        self.assertEqual(image_extent.attrib, extent.attrib)
        visible_text = ''.join(node.text or '' for node in document.findall('.//w:t', NS))
        self.assertIn('原稿', visible_text)
        self.assertNotIn('let source', visible_text)
        self.assertNotIn('89504e47', visible_text)
        self.assertEqual(len(document.findall('.//w:drawing', NS)), 1)
        (self.root / f'{name}-media.png').write_bytes(parts[media[0]])
        (self.root / f'{name}-document.xml').write_bytes(parts['word/document.xml'])
        (self.root / f'{name}-relationships.xml').write_bytes(parts['word/_rels/document.xml.rels'])
        return {'docxSha256': sha(data), 'docxBytes': len(data), 'mediaPath': media[0],
                'pngSha256': sha(png), 'pngBytes': len(png), 'width': width, 'height': height,
                'drawingCount': 1, 'alt': properties.attrib['descr'],
                'altMatchesXmlNormalizedAuthorSource': properties.attrib['descr'] == normalized_alt,
                'altRetainsExactSourceWhitespace': properties.attrib['descr'] == author_source,
                'extentEmu': {'cx': cx, 'cy': cy}, 'visibleText': visible_text}

    def test_mermaid_docx_and_failed_source_repair(self):
        # Author input must not include the running server's private state or
        # its locked recovery files in the compilation root.
        self.author_root = self.root / 'author-input'
        self.author_root.mkdir()
        source_bytes = (ARGS.evidence / 'diagram.typ').read_bytes()
        source = source_bytes.decode('utf-8')
        png = (ARGS.evidence / 'diagram.png').read_bytes()
        self.assertEqual(png[:8], b'\x89PNG\r\n\x1a\n')
        self.assertEqual(source.count('  let source = '), 1)
        self.assertEqual(source.count('  let confirmed = '), 1)
        literal = re.search(r'^  let source = ("(?:\\.|[^"\\])*")$', source, re.M)
        confirmed = re.search(r'^  let confirmed = ("(?:\\.|[^"\\])*")$', source, re.M)
        author_source = json.loads(literal[1])
        self.assertEqual(author_source, json.loads(confirmed[1]))
        embedded = re.search(r'image\(bytes\("([a-f0-9]+)"\.matches', source)
        self.assertEqual(bytes.fromhex(embedded[1]), png)
        (self.author_root / 'diagram.typ').write_bytes(source_bytes)
        (self.root / 'confirmed-browser.png').write_bytes(png)
        REPORT['fixture'] = {'evidence': str(ARGS.evidence), 'sourceSha256': sha(source_bytes),
                             'pngSha256': sha(png), 'pngBytes': len(png),
                             'size': list(struct.unpack('>II', png[16:24])),
                             'editableAuthorSourceBindings': 1, 'confirmedWitnessBindings': 1}
        html = self.compile(source, 'current-html')
        self.assertTrue(html['ok'], html.get('diagnostics'))
        data_uri = re.search(r'data:image/png;base64,([A-Za-z0-9+/=]+)', html['html'])
        self.assertIsNotNone(data_uri)
        self.assertEqual(base64.b64decode(data_uri[1]), png)
        paged = self.compile(source, 'current-paged', paged=True)
        self.assertTrue(paged['ok'], paged.get('diagnostics'))
        doc = self.api('/api/open', {'path': str(self.author_root / 'diagram.typ'), 'root': str(self.author_root)})
        request, output, data = self.generate(doc, source, 'docx', 'mermaid.docx')
        REPORT['docx'] = self.inspect_docx(data, png, author_source, 'mermaid')
        REPORT['docx']['losses'] = output['losses']
        _, source_output, raw_source = self.generate(doc, source, 'source', 'exact-source.typ')
        self.assertEqual(raw_source, source_bytes)
        self.assertEqual((self.author_root / 'diagram.typ').read_bytes(), source_bytes)
        REPORT['exactSourceExport'] = {'sha256': source_output['digest'], 'bytes': len(raw_source),
                                       'matchesOriginal': True}
        stale_author_source = author_source.replace('flowchart', 'graph', 1)
        self.assertNotEqual(stale_author_source, author_source)
        stale = source[:literal.start(1)] + json.dumps(stale_author_source, ensure_ascii=False) + source[literal.end(1):]
        (self.root / 'stale.typ').write_text(stale, encoding='utf-8')
        rejected = self.compile(stale, 'stale-html')
        self.assertFalse(rejected['ok'])
        self.assertTrue(any('Mermaid 图源已改变' in raw['text'] for item in rejected['diagnostics'] for raw in item['diagnostics']))
        bad_request = {**request, 'requestId': features.uid(), 'source': stale, 'serial': 2}
        status, payload, _ = self.raw('/api/export/prepare', bad_request, status=(200, 400, 409, 422))
        failed = json.loads(payload)
        keep_json(self.root / 'stale-docx-response.json', failed)
        if status == 200:
            self.assertEqual(failed['status'], 'failed', failed)
        self.assertFalse(failed.get('digest'))
        self.assertEqual((self.author_root / 'diagram.typ').read_bytes(), source_bytes)
        # A native compile failure must not make the exact product diagram
        # occurrence unwritable. Repair uses the same helper as the UI, on the
        # stale basis, and restores the already-rendered confirmed source/PNG.
        node_program = """
import { diagramOccurrences, diagramPatch } from './src/diagram-state.ts';
let input = ''; for await (const chunk of process.stdin) input += chunk;
const value = JSON.parse(input), occurrences = diagramOccurrences(value.stale);
if (occurrences.length !== 1 || !occurrences[0].stale) throw Error('stale occurrence is not editable');
const basis = { sessionId: 'owned', baseId: 'base', serial: 2, path: 'owned.typ', root: 'owned', source: value.stale };
const patch = diagramPatch(basis, { source: value.authorSource, png: Uint8Array.from(Buffer.from(value.pngBase64, 'base64')), width: value.width, height: value.height }, occurrences[0]);
const repaired = basis.source.slice(0, patch.from) + patch.text + basis.source.slice(patch.to);
const after = diagramOccurrences(repaired); if (after.length !== 1 || after[0].stale) throw Error('repair did not confirm the original occurrence');
process.stdout.write(JSON.stringify({ repaired, idBefore: occurrences[0].id, idAfter: after[0].id, authorSource: after[0].source, editableCount: after.length }));
"""
        repair_input = {'stale': stale, 'authorSource': author_source,
                        'pngBase64': base64.b64encode(png).decode(),
                        'width': REPORT['fixture']['size'][0], 'height': REPORT['fixture']['size'][1]}
        repaired_run = subprocess.run([ARGS.node, '--import', 'tsx', '--input-type=module', '-e', node_program],
                                      cwd=Path(__file__).resolve().parents[1] / 'app',
                                      input=json.dumps(repair_input, ensure_ascii=False),
                                      capture_output=True, encoding='utf-8', timeout=40,
                                      creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        self.assertEqual(repaired_run.returncode, 0, repaired_run.stderr)
        repaired = json.loads(repaired_run.stdout)
        self.assertEqual(repaired['idBefore'], repaired['idAfter'])
        self.assertEqual(repaired['repaired'], source)
        (self.root / 'repaired.typ').write_text(repaired['repaired'], encoding='utf-8')
        repair_render = self.compile(repaired['repaired'], 'repaired-html')
        self.assertTrue(repair_render['ok'], repair_render.get('diagnostics'))
        _, repaired_output, repaired_docx = self.generate(doc, repaired['repaired'], 'docx', 'repaired.docx', serial=3)
        REPORT['repairedDocx'] = self.inspect_docx(repaired_docx, png, author_source, 'repaired')
        # Previously confirmed bytes remain the exact same artifact after a
        # separate failed source snapshot and a successful repaired proposal.
        _, downloaded_again, _ = self.raw('/api/export/download', {'id': output['id']})
        self.assertEqual(downloaded_again, data)
        REPORT['repair'] = {'staleRenderRejected': True, 'staleExportHttpStatus': status,
                            'staleExportResponse': failed, 'sameOccurrenceId': repaired['idAfter'],
                            'editableCount': repaired['editableCount'], 'sourceRestoredExactly': True,
                            'repairedArtifactId': repaired_output['id'],
                            'originalConfirmedDownloadUnchanged': True,
                            'method': 'Product diagramPatch on stale native source, restoring the existing browser-confirmed source/PNG; no new PNG render claimed'}
        REPORT['completed'] = True


def main():
    global ARGS
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--evidence', type=Path, required=True)
    parser.add_argument('--node', default=shutil.which('node'))
    ARGS = parser.parse_args()
    features.EXE = ARGS.executable.resolve(strict=True)
    ARGS.evidence = ARGS.evidence.resolve(strict=True)
    ARGS.output = ARGS.output.resolve()
    ARGS.output.mkdir(parents=True, exist_ok=False)
    features.ARTIFACT_PARENT = ARGS.output
    REPORT.update(executable=str(features.EXE), executableSha256=sha(features.EXE.read_bytes()),
                  python=sys.version, node=ARGS.node, completed=False,
                  boundaries=['No native Word/WPS/LibreOffice interaction',
                              'No fresh PNG regeneration; consumes the separately accepted browser PNG',
                              'DOCX alt uses XML-normalized whitespace; exact editable Mermaid text remains in native Typst',
                              'No cross-platform, remote authorization or synchronization claim'])
    result = unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite([
        DiagramDocx('test_mermaid_docx_and_failed_source_repair')]))
    REPORT['successful'] = result.wasSuccessful()
    REPORT['failures'] = [{'test': str(case), 'traceback': trace} for case, trace in result.failures]
    REPORT['errors'] = [{'test': str(case), 'traceback': trace} for case, trace in result.errors]
    if hasattr(DiagramDocx, 'root'):
        REPORT['ownedRunDirectory'] = str(DiagramDocx.root)
        REPORT['serverStopped'] = DiagramDocx.process.poll() is not None
        REPORT['serverPid'] = DiagramDocx.process.pid
    keep_json(ARGS.output / 'result.json', REPORT)
    print(json.dumps(REPORT, ensure_ascii=False, indent=2))
    return 0 if result.wasSuccessful() else 1


if __name__ == '__main__':
    raise SystemExit(main())
