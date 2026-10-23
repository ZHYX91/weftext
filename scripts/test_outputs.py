"""Real HTTP checks for frozen outputs and original-preserving imports.

Uses only newly owned disposable fixtures. Products, templates, original uploads
and server logs remain for independent Office application inspection. This suite
does not open native windows or claim Word/LibreOffice interaction acceptance.
"""
import argparse
import base64
import hashlib
import io
import json
from pathlib import Path
import posixpath
import sys
import unittest
import xml.etree.ElementTree as ET
import zipfile

import test_local_features as features


W = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'
M = 'http://schemas.openxmlformats.org/officeDocument/2006/math'
X = 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'
R = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships'
P = 'http://schemas.openxmlformats.org/package/2006/relationships'
CT = 'http://schemas.openxmlformats.org/package/2006/content-types'


def b64(value):
    return base64.b64encode(value).decode('ascii')


def package(parts):
    output = io.BytesIO()
    with zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        for name, value in parts.items():
            archive.writestr(name, value.encode() if isinstance(value, str) else value)
    return output.getvalue()


def unpack(value):
    with zipfile.ZipFile(io.BytesIO(value)) as archive:
        return {name: archive.read(name) for name in archive.namelist()}


def xml_text(node, namespace=W):
    return ''.join(text.text or '' for text in node.iter(f'{{{namespace}}}t'))


def docx_template():
    def slot(name):
        return (f'<w:sdt><w:sdtPr><w:tag w:val="{name}"/><w:alias w:val="{name}"/>'
                f'</w:sdtPr><w:sdtContent><w:p><w:r><w:t>OLD {name}</w:t>'
                '</w:r></w:p></w:sdtContent></w:sdt>')
    footer = (f'<w:ftr xmlns:w="{W}"><w:p><w:r><w:t>原模板页脚 FOOTER 2026'
              '</w:t></w:r></w:p></w:ftr>').encode()
    colors = {'dk1': '000000', 'lt1': 'FFFFFF', 'dk2': '1F497D', 'lt2': 'EEECE1',
              'accent1': '4F81BD', 'accent2': 'C0504D', 'accent3': '9BBB59',
              'accent4': '8064A2', 'accent5': '4BACC6', 'accent6': 'F79646',
              'hlink': '0000FF', 'folHlink': '800080'}
    scheme = ''.join(f'<a:{name}><a:srgbClr val="{color}"/></a:{name}>' for name, color in colors.items())
    fonts = ''.join(f'<a:{name}Font><a:latin typeface="{font}"/><a:ea typeface=""/><a:cs typeface=""/></a:{name}Font>'
                    for name, font in [('major', 'Cambria'), ('minor', 'Calibri')])
    fills = '<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>' * 3
    lines = ''.join(f'<a:ln w="{width}" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/><a:miter lim="800000"/></a:ln>'
                    for width in [9525, 25400, 38100])
    effects = '<a:effectStyle><a:effectLst/></a:effectStyle>' * 3
    theme = (f'<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Retained template theme"><a:themeElements><a:clrScheme name="Output QA">{scheme}</a:clrScheme><a:fontScheme name="Output QA">{fonts}</a:fontScheme><a:fmtScheme name="Output QA"><a:fillStyleLst>{fills}</a:fillStyleLst><a:lnStyleLst>{lines}</a:lnStyleLst><a:effectStyleLst>{effects}</a:effectStyleLst><a:bgFillStyleLst>{fills}</a:bgFillStyleLst></a:fmtScheme></a:themeElements><a:objectDefaults/><a:extraClrSchemeLst/></a:theme>').encode()
    parts = {
        '[Content_Types].xml': f'<Types xmlns="{CT}"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/footer1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/><Override PartName="/word/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/></Types>',
        '_rels/.rels': f'<Relationships xmlns="{P}"><Relationship Id="officeDocument" Type="{R}/officeDocument" Target="word/document.xml"/></Relationships>',
        'word/document.xml': f'<w:document xmlns:w="{W}" xmlns:r="{R}"><w:body>{slot("TitleSlot")}{slot("BodySlot")}{slot("LabelSlot")}<w:sectPr><w:footerReference w:type="default" r:id="footer"/><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:bottom="1440" w:left="1440" w:right="1440"/></w:sectPr></w:body></w:document>',
        'word/_rels/document.xml.rels': f'<Relationships xmlns="{P}"><Relationship Id="styles" Type="{R}/styles" Target="styles.xml"/><Relationship Id="footer" Type="{R}/footer" Target="footer1.xml"/><Relationship Id="theme" Type="{R}/theme" Target="theme/theme1.xml"/></Relationships>',
        'word/styles.xml': f'<w:styles xmlns:w="{W}"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Calibri" w:eastAsia="Microsoft YaHei"/><w:sz w:val="22"/></w:rPr></w:rPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style><w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:basedOn w:val="Normal"/></w:style></w:styles>',
        'word/footer1.xml': footer,
        'word/theme/theme1.xml': theme,
    }
    return package(parts), footer, theme


def xlsx_template():
    parts = {
        '[Content_Types].xml': f'<Types xmlns="{CT}"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/></Types>',
        '_rels/.rels': f'<Relationships xmlns="{P}"><Relationship Id="officeDocument" Type="{R}/officeDocument" Target="xl/workbook.xml"/></Relationships>',
        'xl/workbook.xml': f'<workbook xmlns="{X}" xmlns:r="{R}"><sheets><sheet name="报表" sheetId="1" r:id="sheet1"/></sheets><definedNames><definedName name="ClientName">\'报表\'!$D$1</definedName><definedName name="ReportRows">\'报表\'!$A$3:$C$5</definedName></definedNames></workbook>',
        'xl/_rels/workbook.xml.rels': f'<Relationships xmlns="{P}"><Relationship Id="sheet1" Type="{R}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="styles" Type="{R}/styles" Target="styles.xml"/></Relationships>',
        'xl/styles.xml': f'<styleSheet xmlns="{X}"><fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>',
        'xl/worksheets/sheet1.xml': f'<worksheet xmlns="{X}"><sheetData><row r="1"><c r="D1" t="inlineStr"><is><t>OLD LABEL</t></is></c><c r="E1"><f>1+2</f><v>3</v></c><c r="F1" t="inlineStr"><is><t>UNCHANGED 模板</t></is></c></row><row r="3"><c r="A3" t="inlineStr"><is><t>OLD A</t></is></c></row></sheetData></worksheet>',
    }
    return package(parts)


def xlsx_sheets(parts):
    workbook = ET.fromstring(parts['xl/workbook.xml'])
    relationships = ET.fromstring(parts['xl/_rels/workbook.xml.rels'])
    paths = {rel.attrib['Id']: posixpath.normpath(posixpath.join('xl', rel.attrib['Target']))
             for rel in relationships}
    return {sheet.attrib['name']: paths[sheet.attrib[f'{{{R}}}id']]
            for sheet in workbook.find(f'{{{X}}}sheets')}


def xlsx_cells(parts, path):
    shared = []
    if 'xl/sharedStrings.xml' in parts:
        shared = [''.join(t.text or '' for t in item.iter(f'{{{X}}}t'))
                  for item in ET.fromstring(parts['xl/sharedStrings.xml'])]
    output = {}
    for cell in ET.fromstring(parts[path]).iter(f'{{{X}}}c'):
        if cell.attrib.get('t') == 'inlineStr':
            value = ''.join(t.text or '' for t in cell.iter(f'{{{X}}}t'))
        else:
            value = cell.findtext(f'{{{X}}}v', '')
            if cell.attrib.get('t') == 's':
                value = shared[int(value)]
        output[cell.attrib['r']] = (value, cell)
    return output


def xlsx_structure(parts):
    path = next(path for name, path in xlsx_sheets(parts).items()
                if name.startswith('Weftext structure'))
    cells = xlsx_cells(parts, path)
    grouped = {}
    for address, (item, _) in cells.items():
        if address.startswith('A') and address != 'A1':
            row = address[1:]
            grouped.setdefault(item, []).append((int(cells[f'B{row}'][0]), cells[f'D{row}'][0]))
    return {key: json.loads(''.join(chunk for _, chunk in sorted(chunks)))
            for key, chunks in grouped.items()}


class Outputs(features.LocalFeatures):
    def keep(self, name, value):
        directory = self.root / 'output-products'
        directory.mkdir(exist_ok=True)
        path = directory / name
        path.write_bytes(value.encode('utf-8') if isinstance(value, str) else value)
        return path

    def prepare_output(self, doc, fmt, source=None, serial=1, **options):
        request = {'outputLocale': 'zh-CN', 'requestId': features.uid(), 'sessionId': doc['sessionId'],
                   'baseId': doc['baseId'], 'source': doc['source'] if source is None else source,
                   'serial': serial, 'format': fmt, 'stylePolicy': 'template', **options}
        output = self.api('/api/export/prepare', request)
        self.assertEqual(output['status'], 'prepared', output)
        self.assertEqual(output['requestId'], request['requestId'])
        self.keep(f'{fmt}-{request["requestId"][:8]}-preview.json', json.dumps(output, ensure_ascii=False, indent=2))
        return request, output

    def download_output(self, output, name, accept=True):
        confirmed = self.api('/api/export/confirm', {'id': output['id'], 'acceptLosses': accept})
        self.assertEqual(confirmed['status'], 'confirmed')
        _, payload, headers = self.raw('/api/export/download', {'id': output['id']})
        self.assertEqual(len(payload), output['length'])
        self.assertEqual(hashlib.sha256(payload).hexdigest(), output['digest'])
        self.assertEqual(headers['Cache-Control'], 'no-store')
        self.keep(name, payload)
        return payload

    def prepare_exchange(self, doc, root, name, fmt, payload, operation_id=None, status=200):
        request = {'operationId': operation_id or features.uid(), 'sessionId': doc['sessionId'],
                   'baseId': doc['baseId'], 'root': str(root), 'parent': str(root),
                   'name': name, 'format': fmt, 'bytesBase64': b64(payload)}
        return request, self.api('/api/exchange/prepare', request, status=status)

    def commit_exchange(self, preview, accept=True, status=200):
        response = self.api('/api/exchange/commit', {'operationId': preview['operationId'],
                            'planId': preview['planId'], 'acceptLosses': accept}, status=status)
        if status == 200:
            self.assertEqual(response['status'], 'committed')
            self.assertEqual(response['planId'], preview['planId'])
        return response

    def rich_document(self):
        doc = self.ordinary('= Run-in heading\n\nRun-in body *bold* and _italic_.\n\n$ x^2 + y^2 = z^2 $\n')
        source, _ = self.edit(doc, doc['source'], 1, lambda m: {'kind': 'setTitle', 'text': '输出标题🌿'})
        source, _ = self.edit(doc, source, 2, lambda m: {'kind': 'setSubtitle', 'text': '真实副标题'})
        source, _ = self.edit(doc, source, 3, lambda m: {'kind': 'setRunIn',
                         'range': next(s['range'] for s in m['structures'] if s['kind'] == 'heading'),
                         'enabled': True})
        self.keep('rich-author-source.typ', source)
        return doc, source

    def query_fixture(self):
        root, _ = self.library()
        self.install_amount_schema(root)
        decimal = {'kind': 'decimal', 'value': '9007199254740993.120000000000000001'}
        doc, _ = self.capture(root, 'Exact task', ['task'], {'tasks/status': features.text('open'), 'acceptance/amount': decimal})
        plan = {'types': ['task'], 'select': [{'name': 'amount', 'expr': 'amount'},
                    {'name': 'integer', 'expr': '9223372036854775807'},
                    {'name': 'formula-text', 'expr': "'=1+1'"}]}
        query = self.api('/api/query/run', {'root': str(root), 'plan': plan})
        self.assertEqual(query['rows'][0]['cells']['amount']['value'], decimal)
        self.assertIn('queryId', query)
        return root, doc, plan, query, decimal

    def test_01_exact_draft_same_id_ranges_empty_and_token(self):
        doc = self.ordinary('saved original\n')
        exact = '\ufeff#title[准确草稿🌿]\r\n\r\nUnsaved draft *exact*.\r\n'
        request, output = self.prepare_output(doc, 'source', exact, serial=1)
        self.raw('/api/export/download', {'id': output['id']}, status=409)
        payload = self.download_output(output, 'exact-draft.typ', accept=False)
        self.assertEqual(payload, exact.encode())
        self.assertEqual(Path(doc['path']).read_bytes(), b'saved original\n')
        self.saved(doc, 'later saved state\n')
        replay = self.api('/api/export/prepare', request)
        self.assertEqual(replay['status'], 'confirmed')
        self.assertEqual(replay['digest'], output['digest'])
        self.assertEqual(self.api('/api/export/status', {'requestId': request['requestId']})['digest'], output['digest'])
        reuse = self.api('/api/export/prepare', {**request, 'source': 'different'}, status=409)
        self.assertEqual(reuse['error']['code'], 'requestReuse')
        _, part, headers = self.raw('/api/export/download', {'id': output['id'], 'offset': 2, 'length': 5}, status=206)
        self.assertEqual(part, payload[2:7])
        self.assertEqual(headers['Content-Range'], f'bytes 2-6/{len(payload)}')
        overflow = self.api('/api/export/download', {'id': output['id'], 'offset': 1, 'length': 2**64 - 1}, status=416)
        self.assertEqual(overflow['error']['code'], 'invalidRange')
        self.raw('/api/export/download', {'id': output['id']}, status=403, token=False)
        self.api('/api/export/status', {'requestId': output['id']}, status=403, headers={'X-Weftext-Token': 'invalid'})
        empty = self.ordinary('')
        _, empty_output = self.prepare_output(empty, 'source')
        self.assertEqual(self.download_output(empty_output, 'empty-source.typ', accept=False), b'')

    def test_02_docx_real_roles_run_in_and_native_equations(self):
        doc, source = self.rich_document()
        _, output = self.prepare_output(doc, 'docx', source, serial=4)
        self.assertTrue(any(loss['code'] == 'run-in-toc-whole-paragraph' for loss in output['losses']))
        self.api('/api/export/confirm', {'id': output['id'], 'acceptLosses': False}, status=409)
        parts = unpack(self.download_output(output, 'rich-editable.docx'))
        document = ET.fromstring(parts['word/document.xml'])
        all_text = xml_text(document)
        for expected in ['输出标题🌿', '真实副标题', 'Run-in heading', 'Run-in body', 'bold', 'italic']:
            self.assertIn(expected, all_text)
        paragraphs = list(document.iter(f'{{{W}}}p'))
        title = next(p for p in paragraphs if '输出标题' in xml_text(p))
        self.assertEqual(title.find(f'{{{W}}}pPr/{{{W}}}pStyle').attrib[f'{{{W}}}val'], 'Title')
        heading = next(p for p in paragraphs if 'Run-in heading' in xml_text(p))
        self.assertEqual(heading.find(f'{{{W}}}pPr/{{{W}}}pStyle').attrib[f'{{{W}}}val'], 'Heading1')
        self.assertIn('Run-in body', xml_text(heading))
        self.assertTrue(any(True for _ in document.iter(f'{{{M}}}oMath')), 'Equation is not native editable OMML')
        self.assertIn('word/numbering.xml', parts)
        numbered = self.ordinary(
            '#set heading(numbering: "1.1")\n#title[编号与引用真实检查]\n'
            '= First section <first>\n\nFirst section body.\n'
            '== Nested section <nested>\n\nNested body.\n'
            '#counter(heading).update(4)\n'
            '#show <reset>: it => context {\n'
            '  strong[\n'
            '    #if it.numbering != none {\n'
            '      counter(heading).display(it.numbering)\n'
            '      text(" ")\n'
            '    }\n'
            '    #it.body\n'
            '  ]\n'
            '  text(" ")\n'
            '}\n'
            '= Reset run-in target <reset>\nRun-in continuation *bold* and _italic_.\n'
            'See @first, @nested and @reset.\n\n'
            '#table(columns: 2, [Column A], [Column B], [First cell], [Second cell])\n')
        # Advanced counter resets use authored native Source; the rich editor
        # correctly refuses to rewrite this unsupported scope.
        source = numbered['source']
        self.keep('numbered-ref-source.typ', source)
        _, output = self.prepare_output(numbered, 'docx', source, serial=1)
        self.keep('numbered-ref-preview.json', json.dumps(output, ensure_ascii=False, indent=2))
        parts = unpack(self.download_output(output, 'numbered-ref.docx'))
        document = ET.fromstring(parts['word/document.xml'])
        styles = {style.attrib[f'{{{W}}}val'] for style in document.iter(f'{{{W}}}pStyle')}
        self.assertTrue({'Heading1', 'Heading2'}.issubset(styles))
        self.assertTrue(any(True for _ in document.iter(f'{{{W}}}tbl')))
        self.assertTrue(any(' REF ' in field.attrib.get(f'{{{W}}}instr', '')
                            for field in document.iter(f'{{{W}}}fldSimple')))
        reset = next(p for p in document.iter(f'{{{W}}}p') if 'Reset run-in target' in xml_text(p))
        self.assertIn('Run-in continuation', xml_text(reset))
        numbering = ET.fromstring(parts['word/numbering.xml'])
        self.assertTrue(any(start.attrib.get(f'{{{W}}}val') == '5'
                            for start in numbering.iter(f'{{{W}}}startOverride')))
        leading = self.ordinary(source.replace('= Reset run-in target <reset>',
            'Before run-in body.\n= Reset run-in target <reset>'))
        _, output = self.prepare_output(leading, 'docx')
        self.assertTrue(any(loss['code'] == 'run-in-leading-body-loss' for loss in output['losses']))
        self.keep('leading-run-in-preview.json', json.dumps(output, ensure_ascii=False, indent=2))
        self.download_output(output, 'leading-run-in.docx')

    def test_03_docx_standard_controls_preserve_template_footer_and_theme(self):
        doc, source = self.rich_document()
        template, footer, theme = docx_template()
        self.keep('word-standard-controls-template.docx', template)
        inspection = self.api('/api/export/template', {'format': 'docx', 'bytesBase64': b64(template)})
        slots = {slot['name']: slot for slot in inspection['slots']}
        self.assertEqual(set(slots), {'TitleSlot', 'BodySlot', 'LabelSlot'})
        self.assertTrue(all(slot['supported'] for slot in slots.values()))
        _, output = self.prepare_output(doc, 'docx', source, serial=4,
            template={'name': 'word-standard-controls-template.docx', 'bytesBase64': b64(template)},
            bindings={'TitleSlot': {'kind': 'title'}, 'BodySlot': {'kind': 'body'},
                      'LabelSlot': {'kind': 'text', 'text': '固定标签🌿'}})
        parts = unpack(self.download_output(output, 'template-filled.docx'))
        self.assertEqual(parts['word/footer1.xml'], footer)
        self.assertEqual(parts['word/theme/theme1.xml'], theme)
        body = xml_text(ET.fromstring(parts['word/document.xml']))
        self.assertIn('输出标题🌿', body)
        self.assertIn('Run-in body', body)
        self.assertIn('固定标签🌿', body)
        self.assertNotIn('OLD BodySlot', body)

    def test_04_xlsx_uses_cached_query_exact_values_and_formula_text(self):
        root, doc, plan, query, decimal = self.query_fixture()
        changed = self.api('/api/data/edit', {**self.snapshot(doc, serial=2), 'key': 'amount',
                          'value': {'kind': 'decimal', 'value': '10.0'}})
        self.map_source(doc, changed['source'], serial=3)
        current = self.saved(doc, changed['source'], root)
        fresh = self.api('/api/query/run', {'root': str(root), 'plan': plan})
        self.assertNotEqual(fresh['rows'][0]['cells']['amount']['value'], decimal)
        _, output = self.prepare_output(current, 'xlsx', serial=3, queryId=query['queryId'])
        self.assertEqual(output['preview']['query']['queryId'], query['queryId'])
        parts = unpack(self.download_output(output, 'frozen-query-exact.xlsx'))
        structure = xlsx_structure(parts)['sheet:0']
        self.assertEqual(structure['columns'], ['amount', 'integer', 'formula-text'])
        self.assertEqual(structure['result']['rows'], query['rows'])
        self.assertEqual(structure['result']['rows'][0]['cells']['amount']['value'], decimal)
        data_path = xlsx_sheets(parts)['资料']
        cells = xlsx_cells(parts, data_path)
        self.assertEqual(cells['A2'][0], decimal['value'])
        self.assertEqual(cells['B2'][0], '9223372036854775807')
        self.assertEqual(cells['C2'][0], '=1+1')
        self.assertIsNone(cells['C2'][1].find(f'{{{X}}}f'))
        self.assertEqual(cells['C2'][1].attrib.get('t'), 'inlineStr')

    def test_05_xlsx_defined_names_fill_bound_ranges_and_keep_other_cells(self):
        _, doc, _, query, decimal = self.query_fixture()
        template = xlsx_template()
        self.keep('excel-defined-names-template.xlsx', template)
        inspection = self.api('/api/export/template', {'format': 'xlsx', 'bytesBase64': b64(template)})
        slots = {slot['name']: slot for slot in inspection['slots']}
        self.assertEqual(set(slots), {'ClientName', 'ReportRows'})
        self.assertTrue(all(slot['supported'] for slot in slots.values()))
        _, output = self.prepare_output(doc, 'xlsx', queryId=query['queryId'],
            template={'name': 'excel-defined-names-template.xlsx', 'bytesBase64': b64(template)},
            bindings={'ClientName': {'kind': 'text', 'text': '=HYPERLINK("never execute")'},
                      'ReportRows': {'kind': 'query'}})
        parts = unpack(self.download_output(output, 'template-filled.xlsx'))
        cells = xlsx_cells(parts, xlsx_sheets(parts)['报表'])
        self.assertEqual(cells['D1'][0], '=HYPERLINK("never execute")')
        self.assertIsNone(cells['D1'][1].find(f'{{{X}}}f'))
        self.assertEqual(cells['A4'][0], decimal['value'])
        self.assertEqual(cells['B4'][0], '9223372036854775807')
        self.assertEqual(cells['C4'][0], '=1+1')
        self.assertEqual(cells['F1'][0], 'UNCHANGED 模板')
        self.assertEqual(cells['E1'][1].findtext(f'{{{X}}}f'), '1+2')
        self.assertIsNone(cells['E1'][1].find(f'{{{X}}}v'), 'Stale formula cache was retained')

    def test_06_native_library_package_fresh_import_same_plan_and_invalid_zip(self):
        origin, library = self.library()
        note = self.note(origin, 'Original')
        source = note['source'] + f'\r\n#link("weftext-node://{library["workspaceId"]}/{note["nodeId"]}")[Self]\r\n'
        note = self.saved(note, source, origin)
        png = features.tiny_png()
        resource = self.api('/api/resources/prepare-import', {'sessionId': note['sessionId'],
                   'name': 'owned.png', 'bytes': b64(png), 'operationId': features.uid()})
        self.commit(resource)
        (origin / 'unknown-original.txt').write_bytes(b'not registered; keep original')
        _, output = self.prepare_output(note, 'native')
        self.assertEqual(output['preview']['scope'], 'savedWholeLibrary')
        self.assertTrue(any(loss['code'] == 'unregisteredFileOmitted' for loss in output['losses']))
        payload = self.download_output(output, 'saved-library.weftext.zip')
        parts = unpack(payload)
        self.assertEqual(parts['Original/Original.typ'], source.encode())
        self.assertEqual(parts['Original/owned.png'], png)
        self.assertNotIn('.weftext-mvp-commit.json', parts)
        target, target_library = self.library()
        anchor = self.note(target, 'Anchor')
        request, preview = self.prepare_exchange(anchor, target, 'Imported', 'native', payload)
        self.assertEqual(preview['kind'], 'importNative')
        self.assertNotEqual(preview['nodeMap'][note['nodeId']], note['nodeId'])
        self.assertFalse((target / 'Imported').exists())
        # Changing the context source cannot cause the original Plan to be rebuilt.
        self.saved(anchor, anchor['source'] + '\nchanged context\n', target)
        repeated = self.api('/api/exchange/prepare', request)
        self.assertEqual(repeated['planId'], preview['planId'])
        reuse = self.api('/api/exchange/prepare', {**request, 'bytesBase64': b64(payload + b'changed')}, status=409)
        self.assertEqual(reuse['error']['code'], 'operationConflict')
        self.commit_exchange(preview, accept=False, status=409)
        receipt = self.commit_exchange(preview)
        self.assertEqual(Path(receipt['path']), target / 'Imported')
        self.assertEqual(self.commit_exchange(preview)['nodeIds'], receipt['nodeIds'])
        imported = self.api('/api/open', {'path': str(target / 'Imported/Original/Original.typ'), 'root': str(target)})
        self.assertEqual(imported['nodeId'], preview['nodeMap'][note['nodeId']])
        self.assertIn(f'weftext-node://{target_library["workspaceId"]}/{imported["nodeId"]}', imported['source'])
        self.assertEqual((target / 'Imported/.weftext-import-originals/original.weftext').read_bytes(), payload)
        self.assertEqual((origin / 'unknown-original.txt').read_bytes(), b'not registered; keep original')
        resources = self.api('/api/resources/list', {'sessionId': imported['sessionId']})['resources']
        self.assertIsNotNone(resources[0]['revision'])
        self.assertEqual((Path(imported['path']).parent / resources[0]['name']).read_bytes(), png)
        anchor = self.api('/api/open', {'path': anchor['path'], 'root': str(target)})
        _, bad = self.prepare_exchange(anchor, target, 'Bad ZIP', 'native', b'not a zip', status=409)
        self.assertEqual(bad['error']['code'], 'packageZip')
        self.assertFalse((target / 'Bad ZIP').exists())

    def test_07_bibliography_inspect_fresh_works_and_virtual_bib_compile(self):
        root, library = self.library()
        anchor = self.note(root, 'Bibliography anchor')
        uploads = {
            'bibtex': b'@article{external, title={BIB imported}, author={Alpha One and Beta Two}, year={2026}, journal={Journal}, doi={10.1234/bib}}\n',
            'csl-json': json.dumps([{'id': 'same-doi', 'type': 'article-journal', 'title': 'CSL imported',
                'DOI': '10.1234/csl', 'author': [{'literal': 'Alpha'}, {'literal': 'Beta'}],
                'issued': {'date-parts': [[2026, 9]]}}]).encode(),
            'ris': b'TY  - JOUR\nTI  - RIS imported\nAU  - Alpha One\nAU  - Beta Two\nPY  - 2026\nDO  - 10.1234/ris\nER  - \n',
        }
        created = {}
        for fmt, payload in uploads.items():
            self.keep(f'original-input-{fmt}.txt', payload)
            proposal = self.api('/api/exchange/inspect', {'format': fmt, 'bytesBase64': b64(payload)})
            self.assertEqual(len(proposal['records']), 1)
            self.assertFalse(proposal['records'][0]['conflicts'])
            self.assertTrue(proposal['records'][0]['sources'])
            _, preview = self.prepare_exchange(anchor, root, f'Imported {fmt}', fmt, payload)
            self.assertEqual(preview['kind'], 'importBibliography')
            self.assertEqual(len(preview['nodeMap']), 1)
            self.commit_exchange(preview)
            path = root / f'Imported {fmt}/paper-0001/paper-0001.typ'
            doc = self.api('/api/open', {'path': str(path), 'root': str(root)})
            projection = self.api('/api/data/project', self.snapshot(doc))['projection']
            self.assertIn('library/paper', projection['resolvedTypes'])
            self.assertIn('library/work', projection['resolvedTypes'])
            resources = self.api('/api/resources/list', {'sessionId': doc['sessionId']})['resources']
            original = next(r for r in resources if r['name'] != 'original-record.json')
            self.assertIsNotNone(original['revision'])
            self.assertEqual((path.parent / original['name']).read_bytes(), payload)
            created[fmt] = doc
        _, repeated = self.prepare_exchange(anchor, root, 'Repeated DOI', 'csl-json', uploads['csl-json'])
        self.commit_exchange(repeated)
        self.assertNotIn(created['csl-json']['nodeId'], repeated['nodeMap'].values())
        cited = self.note(root, 'Citing note')
        work_ref = f'weftext-node://{library["workspaceId"]}/{created["csl-json"]["nodeId"]}'
        source = cited['source'] + (f'\n#metadata((kind: "weftext.citations", entries: ((key: "selectedWork", work: "{work_ref}"),))) <weftext-citations>\n'
                  'See @selectedWork; #cite(<selectedWork>).\n#bibliography("weftext.citations.bib")\n')
        cited = self.saved(cited, source, root)
        rendered = self.api('/api/render', self.snapshot(cited))
        self.assertTrue(rendered['ok'], rendered.get('diagnostics'))
        self.assertEqual(rendered['citations']['works'][0]['work']['entityRef'], work_ref)
        self.assertTrue(any(d['path'].endswith('weftext.citations.bib') and d['status'] == 'loaded'
                            for d in rendered['dependencies']))
        self.assertFalse(Path(cited['path']).with_name('weftext.citations.bib').exists())
        self.keep('virtual-citation.bib', bytes(rendered['citations']['bibliography']['bytes']))
        for fmt, extension in [('bibtex', 'bib'), ('csl-json', 'json'), ('ris', 'ris')]:
            _, output = self.prepare_output(cited, fmt)
            payload = self.download_output(output, f'citation-works.{extension}')
            proposal = self.api('/api/exchange/inspect', {'format': fmt, 'bytesBase64': b64(payload)})
            self.assertEqual(len(proposal['records']), 1)
            self.assertFalse(proposal['records'][0]['conflicts'])
        # The declaration's authored key remains stable while copied Work refs
        # enter the same fresh map as the actual paper notes and owned resources.
        _, native_output = self.prepare_output(cited, 'native')
        native_bytes = self.download_output(native_output, 'citation-library.weftext.zip')
        native_parts = unpack(native_bytes)
        self.assertEqual(native_parts['Citing note/Citing note.typ'], source.encode())
        self.assertIn(b'CSL imported', native_parts['Citing note/weftext.citations.bib'])
        target, target_library = self.library()
        target_anchor = self.note(target, 'Native citation target')
        _, native_preview = self.prepare_exchange(target_anchor, target, 'Native copied citations', 'native', native_bytes)
        self.commit_exchange(native_preview)
        copied_citing = self.api('/api/open', {'path': str(target / 'Native copied citations/Citing note/Citing note.typ'), 'root': str(target)})
        new_work_ref = f'weftext-node://{target_library["workspaceId"]}/{native_preview["nodeMap"][created["csl-json"]["nodeId"]]}'
        self.assertIn(new_work_ref, copied_citing['source'])
        self.assertIn('key: "selectedWork"', copied_citing['source'])
        copied_render = self.api('/api/render', self.snapshot(copied_citing))
        self.assertTrue(copied_render['ok'], copied_render.get('diagnostics'))
        self.assertEqual(copied_render['citations']['works'][0]['work']['entityRef'], new_work_ref)
        copied_work = self.api('/api/open', {'path': str(target / 'Native copied citations/Imported csl-json/paper-0001/paper-0001.typ'), 'root': str(target)})
        projected = self.api('/api/data/project', self.snapshot(copied_work))['projection']
        publication = next(field['cell']['value'] for field in projected['fields'] if field['key'] == 'publication')
        for member in publication['value']:
            if member['key'] == 'citation_key':
                member['value'] = features.text('ChangedSuggestion')
                break
        else:
            publication['value'].append({'key': 'citation_key', 'value': features.text('ChangedSuggestion')})
        changed = self.api('/api/data/edit', {**self.snapshot(copied_work, serial=2), 'key': 'publication', 'value': publication})
        self.saved(copied_work, changed['source'], target)
        after_change = self.api('/api/render', self.snapshot(copied_citing, serial=2))
        self.assertTrue(after_change['ok'], after_change.get('diagnostics'))
        self.assertEqual(after_change['citations']['declaration']['entries'][0]['key'], 'selectedWork')
        self.assertEqual(after_change['citations']['works'][0]['work']['entityRef'], new_work_ref)

    def test_08_bibliography_exact_extension_conflict_is_explicit_and_has_no_effect(self):
        root, _ = self.library()
        anchor = self.note(root, 'Conflict anchor')
        upload = json.dumps([{'type': 'article-journal', 'title': 'Original formal title',
                             'author': [{'literal': 'Original Author'}]}]).encode()
        _, preview = self.prepare_exchange(anchor, root, 'Paper source', 'csl-json', upload)
        self.commit_exchange(preview)
        doc = self.api('/api/open', {'path': str(root / 'Paper source/paper-0001/paper-0001.typ'), 'root': str(root)})
        _, output = self.prepare_output(doc, 'csl-json')
        exact = json.loads(self.download_output(output, 'original-with-exact-extension.json'))
        exact[0]['title'] = 'External changed title'
        changed = json.dumps(exact).encode()
        self.keep('conflicting-csl-input.json', changed)
        proposal = self.api('/api/exchange/inspect', {'format': 'csl-json', 'bytesBase64': b64(changed)})
        self.assertTrue(any(c['key'] == 'title' for c in proposal['records'][0]['conflicts']))
        _, failure = self.prepare_exchange(anchor, root, 'Refused conflict', 'csl-json', changed, status=(400, 409, 422))
        self.assertEqual(failure['error']['code'], 'bibliographyConflict')
        self.assertFalse((root / 'Refused conflict').exists())
        self.assertIn('Original formal title', Path(doc['path']).read_text(encoding='utf-8'))

    def test_09_native_saved_whole_library_freezes_citation_inputs_and_rejects_ambiguity(self):
        root, library = self.library()
        anchor = self.note(root, 'Current context')
        upload = json.dumps([{'type': 'article-journal', 'title': 'Exact saved title',
                             'author': [{'literal': 'Original Author'}]}]).encode()
        _, imported = self.prepare_exchange(anchor, root, 'Saved Work', 'csl-json', upload)
        self.commit_exchange(imported)
        work = self.api('/api/open', {'path': str(root / 'Saved Work/paper-0001/paper-0001.typ'),
                                     'root': str(root)})
        citing = self.note(root, 'Other saved owner')
        ref = f'weftext-node://{library["workspaceId"]}/{work["nodeId"]}'
        source = citing['source'] + (
            f'\n#metadata((kind: "weftext.citations", entries: ((key: "exact", work: "{ref}"),))) <weftext-citations>\n'
            '@exact\n#bibliography("weftext.citations.bib")\n')
        citing = self.saved(citing, source, root)
        request, output = self.prepare_output(anchor, 'native')
        payload = self.download_output(output, 'whole-library-with-derived-citations.weftext.zip')
        parts = unpack(payload)
        manifest = json.loads(parts['manifest.json'])
        bib_path = 'Other saved owner/weftext.citations.bib'
        bib = parts[bib_path]
        self.assertEqual(parts['Other saved owner/Other saved owner.typ'], source.encode())
        self.assertIn(b'Exact saved title', bib)
        self.assertEqual(manifest['entries'][bib_path]['role'], 'generatedBibliography')
        self.assertEqual(manifest['entries'][bib_path]['ownerNodeId'], citing['nodeId'])
        self.assertEqual(manifest['entries'][bib_path]['sha256'], hashlib.sha256(bib).hexdigest())
        self.assertFalse(Path(citing['path']).with_name('weftext.citations.bib').exists())
        work = self.saved(work, work['source'].replace('Exact saved title', 'Later saved title'), root)
        _, newer = self.prepare_output(anchor, 'native')
        later = unpack(self.download_output(newer, 'later-whole-library.weftext.zip'))
        self.assertIn(b'Later saved title', later[bib_path])
        # Original IDs and artifacts retain their exact initial saved cut.
        replay = self.api('/api/export/prepare', request)
        self.assertEqual(replay['id'], output['id'])
        self.assertEqual(replay['digest'], output['digest'])
        _, old, _ = self.raw('/api/export/download', {'id': output['id']})
        self.assertEqual(old, payload)
        bad_ref = f'weftext-node://{library["workspaceId"]}/{anchor["nodeId"]}'
        for changed, expected in [
            (source.replace(ref, bad_ref), 'citationWorkType'),
            (source + '\n#bibliography("external.bib")\n', 'citationDeclarationUnavailable'),
        ]:
            citing = self.saved(citing, changed, root)
            failure = self.api('/api/export/prepare', {**request, 'requestId': features.uid()})
            self.assertEqual(failure['status'], 'failed')
            self.assertEqual(failure['code'], expected)
            self.raw('/api/export/download', {'id': failure['id']}, status=409)
        citing = self.saved(citing, source, root)
        occupied = Path(citing['path']).with_name('weftext.citations.bib')
        occupied.write_bytes(b'original author file; never replace')
        failure = self.api('/api/export/prepare', {**request, 'requestId': features.uid()})
        self.assertEqual(failure['status'], 'failed')
        self.assertEqual(failure['code'], 'packagePath')
        self.raw('/api/export/download', {'id': failure['id']}, status=409)
        self.assertEqual(occupied.read_bytes(), b'original author file; never replace')
        self.assertEqual(Path(citing['path']).read_bytes(), source.encode())


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path)
    parser.add_argument('--artifacts', '--artifacts-dir', type=Path, dest='artifacts')
    parser.add_argument('--only', action='append', default=[], metavar='TEST')
    args = parser.parse_args()
    features.EXE = args.executable.resolve(strict=True)
    executable_digest = hashlib.sha256(features.EXE.read_bytes()).hexdigest()
    if args.artifacts:
        args.artifacts.mkdir(parents=True, exist_ok=True)
        features.ARTIFACT_PARENT = args.artifacts.resolve(strict=True)
    names = sorted(name for name in Outputs.__dict__ if name.startswith('test_'))
    if args.only:
        selected = {name.split('.')[-1] for name in args.only}
        unknown = selected.difference(names)
        if unknown:
            parser.error(f'Unknown new output scenarios: {sorted(unknown)}')
        names = [name for name in names if name in selected]
    # Inherited LocalFeatures methods are helpers only; its 18 scenarios are not rerun.
    suite = unittest.TestSuite(Outputs(name) for name in names)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if hasattr(Outputs, 'root'):
        (Outputs.root / 'results.json').write_text(json.dumps({
            'executable': str(features.EXE), 'testsRun': result.testsRun,
            'executableSha256AtStart': executable_digest,
            'failures': [{'test': str(test), 'traceback': trace} for test, trace in result.failures],
            'errors': [{'test': str(test), 'traceback': trace} for test, trace in result.errors],
            'successful': result.wasSuccessful(),
            'nativeOfficeInteraction': 'Not run; retained products require independent application QA',
            'products': sorted(str(path.relative_to(Outputs.root)) for path in
                               (Outputs.root / 'output-products').glob('*')),
        }, ensure_ascii=False, indent=2), encoding='utf-8')
    sys.exit(0 if result.wasSuccessful() else 1)
