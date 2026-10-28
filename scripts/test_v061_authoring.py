"""Real HTTP acceptance for v0.6.1 authoring on the supplied Windows candidate.

Reuses Server and retained Windows identity helpers from the existing hosted/
sync harnesses. Every library, private state and transfer directory belongs to
this run and is retained. No desktop input, cleanup, source journal rewriting or
fallback to an older executable is performed. HTTP checks do not establish real
keyboard/IME, clipboard, Undo UI or Word acceptance.

Run: python scripts/test_v061_authoring.py CANDIDATE.exe --evidence-dir DIRECTORY
     python scripts/test_v061_authoring.py --list
Optional positional test names select exact cases; unknown selections fail.
"""
import uuid
from _creation_result import created_document, library_fact
import argparse
from concurrent.futures import ThreadPoolExecutor
import copy
import hashlib
import json
import os
from pathlib import Path
import secrets
import sys
import tempfile
import time
import unittest

import test_hosted
from test_hosted import Server, digest, uid
from test_sync_compound import physical_identity, tree_state


EVIDENCE = None
TABLE_BODY = ('\n= Before\n\n#table(columns: 2, [same], [same], [中文😀], [])'
              '\n\nAfter exact bytes 🌿\n')
NOTES_BODY = ('\n= Heading <first>\n\nText #footnote[old note] <original-note>\n\n'
              '#table(columns: 2, [A], [B])\n\n$ x + y $\n')
INITIAL_STYLE = ('// INITIAL_STYLE_PRIVATE_SOURCE_COMMENT\n'
                 '#let library-style(body) = {\n'
                 '  set text(lang: "zh", region: "CN")\n'
                 '  [INITIAL_STYLE_EXECUTED]\n  body\n}\n')


def tree_text(tree):
    if not isinstance(tree, dict):
        return ''
    return (tree.get('text') or '') + ''.join(tree_text(child) for child in tree.get('children', []))


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')


class RetainedHTTP(unittest.TestCase):
    def setUp(self):
        if os.name != 'nt':
            self.fail('Windows physical-identity acceptance requires Windows; it is not a skip/pass')
        self.folder = EVIDENCE / f'c{CASES.index(self._testMethodName) + 1:02}'
        self.folder.mkdir()
        write_json(self.folder / 'case.json', {'test': self._testMethodName})
        self.servers = []

    def server(self, label, state, config=None):
        args = [ '--data-dir', str(state)]
        if config is not None:
            args += ['--server-config', str(config)]
        server = Server(args, hosted=config is not None)
        server.evidence_label = label
        server.evidence_index = len(self.servers)
        server.evidence_closed = False
        self.servers.append(server)
        self.addCleanup(self.close_server, server)
        return server

    def close_server(self, server):
        if server.evidence_closed:
            return
        stdout, stderr = server.close()
        server.evidence_closed = True
        stem = f'server-{server.evidence_index:02}-{server.evidence_label}'
        (self.folder / f'{stem}.stdout.log').write_text(stdout, encoding='utf-8')
        (self.folder / f'{stem}.stderr.log').write_text(stderr, encoding='utf-8')
        with (self.folder / 'processes.ndjson').open('a', encoding='utf-8') as log:
            log.write(json.dumps({'program': server.evidence_label, 'pid': server.process.pid,
                                  'closed': server.process.poll() is not None}) + '\n')
        self.assertIsNotNone(server.process.poll())
        self.assertEqual(stderr, '', f'{server.evidence_label} stderr')

    def call(self, server, route, data=None, status=200, token=None):
        started = time.monotonic()
        counters = self.__dict__.setdefault('_draft_counters', {})
        key = (id(server), data.get('sessionId')) if data else None
        if route.endswith('/draft') and data and status == 200:
            data['inputRevision'] = max(data.get('inputRevision', 1), counters.get(key, 0) + 1)
        code, result, headers = server.call(route, data, token=token)
        if route.endswith('/draft') and code == 200:
            counters[key] = result['inputRevision']
        with (self.folder / 'http.ndjson').open('a', encoding='utf-8') as log:
            log.write(json.dumps({'program': server.evidence_label, 'route': route,
                                  'request': data, 'status': code, 'response': result,
                                  'elapsedSeconds': time.monotonic() - started},
                                 ensure_ascii=False) + '\n')
        self.assertEqual(code, status, f'{route}: {result}')
        if server.hosted:
            self.assertEqual(headers.get('Cache-Control'), 'no-store')
        return result

    def create_library(self, server, root):
        return self.call(server, '/api/library/create', {'path': str(root), 'operationId': uid()})

    def create_note(self, server, root, name='Note'):
        return created_document(self.call(server, '/api/note/create', {
            'root': str(root), 'parent': str(root), 'name': name, 'operationId': uid()}))

    def local_fixture(self):
        server = self.server('local', self.folder / 'state')
        root = self.folder / 'lib'
        self.create_library(server, root)
        note = self.create_note(server, root)
        return server, root, note

    def hosted_fixture(self):
        local, root, note = self.local_fixture()
        other = self.create_note(local, root, 'Other')
        self.close_server(local)
        tokens = {name: secrets.token_urlsafe(32) for name in ('alice', 'bob', 'reader', 'admin')}
        full = {'read': True, 'write': True, 'history': True, 'download': True}
        config = {'deployment': 'v061-authoring', 'bind': '127.0.0.1:0', 'root': str(root),
                  'adminTokenSha256': digest(tokens['admin']),
                  'files': {name: {'path': f'{name}/{name}.typ', 'title': name, 'editable': True}
                            for name in ('Note', 'Other')},
                  'users': {name: {'enabled': True, 'tokenSha256': digest(tokens[name]),
                                   'grants': {key: full.copy() for key in ('Note', 'Other')}}
                            for name in ('alice', 'bob')}}
        config['users']['reader'] = {'enabled': True, 'tokenSha256': digest(tokens['reader']),
                                      'grants': {'Note': {'read': True}}}
        config_path = self.folder / 'server.json'
        write_json(config_path, config)
        server = self.server('hosted', self.folder / 'state', config_path)
        return server, root, note, other, tokens

    def view(self, server, root, name='Note', token=None):
        if server.hosted:
            return self.call(server, '/api/hosted/open', {'resource': name}, token=token)
        return self.call(server, '/api/open', {'root': str(root),
                         'path': str(root / name / f'{name}.typ')})

    def endpoint(self, server, operation):
        return ({'map': '/api/editor-map', 'edit': '/api/edit', 'render': '/api/render',
                 'save': '/api/save', 'draft': '/api/draft'}[operation] if not server.hosted
                else '/api/hosted/' + {'map': 'editor/map', 'edit': 'editor/edit',
                                      'render': 'editor/render', 'save': 'save', 'draft': 'draft'}[operation])

    def observe(self, server, doc, source, serial, token=None):
        basis = {'sessionId': doc['sessionId'], 'source': source, 'serial': serial}
        mapped = self.call(server, self.endpoint(server, 'map'), basis, token=token)
        self.assertTrue(mapped['syntaxValid'], mapped)
        self.assertIsNone(mapped['unavailableReason'], mapped)
        self.assertEqual(mapped['sourceHash'], digest(source))
        self.assertEqual(mapped['serial'], serial)
        return basis, mapped

    def table(self, mapped):
        structures = [entry for entry in mapped['structures'] if entry['kind'] == 'table']
        self.assertEqual(len(structures), 1, structures)
        self.assertTrue(structures[0]['table']['topologyWritable'], structures[0])
        return structures[0]

    def topology(self, mapped, rows, columns):
        info = self.table(mapped)['table']
        self.assertEqual((info['rows'], info['columns']), (rows, columns))
        coverage = set()
        for cell in info['logicalCells']:
            for row in range(cell['row'], cell['row'] + cell['rowspan']):
                for column in range(cell['column'], cell['column'] + cell['colspan']):
                    self.assertNotIn((row, column), coverage)
                    coverage.add((row, column))
        self.assertEqual(coverage, {(row, column) for row in range(rows) for column in range(columns)})
        return info

    def propose(self, server, basis, mapped, intent, token=None, status=200):
        result = self.call(server, self.endpoint(server, 'edit'), {
            **basis, 'sourceHash': mapped['sourceHash'], 'intent': intent}, status, token)
        if status != 200:
            self.assertEqual(result['error']['code'], 'unsupported_edit')
            return result
        raw = basis['source'].encode('utf-8')
        for patch in reversed(result['patches']):
            self.assertEqual(raw[patch['from']:patch['to']], patch['expected'].encode('utf-8'))
            raw = raw[:patch['from']] + patch['insert'].encode('utf-8') + raw[patch['to']:]
        self.assertEqual(raw.decode('utf-8'), result['source'])
        self.assertEqual(result['sourceHash'], digest(result['source']))
        return result

    def save_exact(self, server, doc, source, token=None):
        revision = self.__dict__.setdefault('_draft_counters', {}).get((id(server), doc['sessionId']), 0) + 1
        ack = self.call(server, self.endpoint(server, 'draft'), {'sessionId': doc['sessionId'], 'baseId': doc['baseId'], 'source': source, 'inputId': uid(), 'inputRevision': revision}, token=token)
        doc['_draftRevision'] = ack['inputRevision']
        request = {'sessionId': doc['sessionId'], 'baseId': doc['baseId'],
                   'source': source, 'operationId': uid(), 'inputRevision': ack['inputRevision']}
        receipt = self.call(server, self.endpoint(server, 'save'), request, token=token)
        self.assertEqual(receipt['status'], 'committed' if server.hosted else 'saved')
        self.assertEqual(receipt['operationId'], request['operationId'])
        self.assertEqual(self.call(server, self.endpoint(server, 'save'), request, token=token), receipt)
        return receipt, request

    def table_workflow(self, hosted):
        if hosted:
            server, root, _, _, tokens = self.hosted_fixture()
            token = tokens['alice']
        else:
            server, root, _ = self.local_fixture()
            token = None
        doc = self.view(server, root, token=token)
        path = root / 'Note/Note.typ'
        original_tree = tree_state(root)
        source, serial = doc['source'] + TABLE_BODY, 1
        basis, mapped = self.observe(server, doc, source, serial, token)
        self.topology(mapped, 2, 2)
        originals = {'directory': physical_identity(path.parent), 'file': physical_identity(path)}

        def action(command, rows, columns, error=False):
            nonlocal source, serial, basis, mapped
            structure = self.table(mapped)
            intent = {'kind': 'tableEdit', 'range': structure['range'], 'action': command}
            result = self.propose(server, basis, mapped, intent, token, 422 if error else 200)
            self.assertEqual(tree_state(root), original_tree, 'An editor proposal must not save author bytes')
            if error:
                return
            before = source.encode('utf-8')
            start, end = structure['range']['from'], structure['range']['to']
            self.assertTrue(result['source'].encode('utf-8').startswith(before[:start]))
            self.assertTrue(result['source'].encode('utf-8').endswith(before[end:]))
            source, serial = result['source'], serial + 1
            basis, mapped = self.observe(server, doc, source, serial, token)
            return self.topology(mapped, rows, columns)

        info = action({'kind': 'moveRow', 'from': 0, 'to': 1}, 2, 2)
        self.assertEqual([cell['body']['raw'] for cell in info['logicalCells']], ['中文😀', '', 'same', 'same'])
        info = action({'kind': 'moveColumn', 'from': 0, 'to': 1}, 2, 2)
        self.assertEqual([cell['body']['raw'] for cell in info['logicalCells']], ['', '中文😀', 'same', 'same'])
        action({'kind': 'insertRow', 'index': 1}, 3, 2)
        action({'kind': 'insertColumn', 'index': 1}, 3, 3)
        action({'kind': 'removeRow', 'index': 1}, 2, 3)
        action({'kind': 'removeColumn', 'index': 1}, 2, 2)
        info = action({'kind': 'clearRange', 'top': 1, 'left': 0, 'bottom': 1, 'right': 1}, 2, 2)
        self.assertEqual([cell['body']['raw'] for cell in info['logicalCells']][2:], ['', ''])
        info = action({'kind': 'pasteRange', 'row': 0, 'column': 0,
                       'text': '"A\tB"\t"line\nnext"\n\t尾😀', 'expand': False}, 2, 2)
        self.assertEqual([cell['body']['raw'] for cell in info['logicalCells']],
                         ['A\\u{9}B', 'line\\\nnext', '', '尾😀'])
        multiline_preview = self.call(server, self.endpoint(server, 'render'), basis, token=token)
        self.assertTrue(multiline_preview['ok'], multiline_preview)

        def tags(tree):
            yield tree.get('tag')
            for child in tree.get('children', []):
                yield from tags(child)

        self.assertIn('br', list(tags(multiline_preview['tree'])),
                      'Quoted clipboard newlines must produce real native line breaks')
        action({'kind': 'paste', 'row': 0, 'column': 0, 'text': 'first\tsecond'}, 2, 2)
        action({'kind': 'pasteRange', 'row': 0, 'column': 0, 'text': 'a\tb\nc', 'expand': False}, 2, 2, True)
        action({'kind': 'pasteRange', 'row': 0, 'column': 0, 'text': '\t' * 1000, 'expand': True}, 2, 2, True)
        action({'kind': 'pasteRange', 'row': 2**64 - 1, 'column': 0, 'text': 'a\nb', 'expand': True}, 2, 2, True)
        action({'kind': 'pasteCells', 'row': 0, 'column': 0, 'rows': 1, 'columns': 2,
                'cells': [{'row': 0, 'column': 0, 'rowspan': 1, 'colspan': 2,
                           'text': 'merged', 'header': False}], 'expand': False}, 2, 2)
        action({'kind': 'clearRange', 'top': 0, 'left': 1, 'bottom': 0, 'right': 1}, 2, 2, True)
        action({'kind': 'split', 'row': 0, 'column': 0}, 2, 2)
        action({'kind': 'merge', 'top': 1, 'left': 0, 'bottom': 1, 'right': 1}, 2, 2)
        info = action({'kind': 'setHeader', 'rows': 1, 'columns': 0}, 2, 2)
        self.assertEqual(info['headerRows'], 1)
        self.assertTrue(all(cell['header'] == (cell['row'] == 0) for cell in info['logicalCells']))
        action({'kind': 'merge', 'top': 0, 'left': 0, 'bottom': 1, 'right': 1}, 2, 2, True)
        action({'kind': 'setHeader', 'rows': 0, 'columns': 0}, 2, 2)
        action({'kind': 'transpose'}, 2, 2)
        action({'kind': 'split', 'row': 0, 'column': 1}, 2, 2)
        action({'kind': 'pasteRange', 'row': 1, 'column': 1, 'text': 'X\tY\nZ\tW', 'expand': False}, 2, 2, True)
        info = action({'kind': 'pasteRange', 'row': 1, 'column': 1, 'text': 'X\tY\nZ\tW', 'expand': True}, 3, 3)
        cells = {(cell['row'], cell['column']): cell['body']['raw'] for cell in info['logicalCells']}
        self.assertEqual([cells[pos] for pos in ((1, 1), (1, 2), (2, 1), (2, 2))], ['X', 'Y', 'Z', 'W'])
        preview = self.call(server, self.endpoint(server, 'render'), basis, token=token)
        self.assertTrue(preview['ok'], preview)
        self.assertEqual(tree_state(root), original_tree)
        receipt, request = self.save_exact(server, doc, source, token)
        self.assertEqual(path.read_bytes(), source.encode('utf-8'))
        self.assertEqual(physical_identity(path), originals['file'])
        self.assertEqual(physical_identity(path.parent), originals['directory'])
        reopened = self.view(server, root, token=token)
        self.assertEqual(reopened['source'], source)
        if hosted:
            self.assertEqual(self.call(server, '/api/hosted/operation', {'resource': doc['resource'], 'operationId': request['operationId']}, token=token), receipt)
            self.assertEqual(self.call(server, '/api/hosted/operation', {'resource': doc['resource'], 'operationId': request['operationId']}, token=tokens['bob'])['status'], 'notRegistered')
        else:
            self.assertEqual(reopened['nodeId'], doc['nodeId'])
            self.assertEqual(reopened['workspaceId'], doc['workspaceId'])

    def notes_workflow(self, hosted):
        if hosted:
            server, root, _, _, tokens = self.hosted_fixture()
            token = tokens['alice']
        else:
            server, root, _ = self.local_fixture()
            token = None
        doc = self.view(server, root, token=token)
        source, serial = doc['source'] + NOTES_BODY, 1
        original = tree_state(root)
        basis, mapped = self.observe(server, doc, source, serial, token)

        def edit(intent):
            nonlocal source, serial, basis, mapped
            output = self.propose(server, basis, mapped, intent, token)
            self.assertEqual(tree_state(root), original)
            source, serial = output['source'], serial + 1
            basis, mapped = self.observe(server, doc, source, serial, token)

        definition = next(entry for entry in mapped['notes']['entries'] if entry['kind'] == 'footnote-definition')
        edit({'kind': 'nativeNote', 'action': {'kind': 'setNoteBody', 'construct': definition['construct'],
              'body': {'mode': 'literal', 'value': 'new # note 中文😀'}}})
        self.assertIn('new\\u{20}\\#\\u{20}note\\u{20}中文😀', source)
        definition = next(entry for entry in mapped['notes']['entries'] if entry['kind'] == 'footnote-definition')
        edit({'kind': 'nativeNote', 'action': {'kind': 'repeatFootnote', 'at': mapped['bodyInsertion'],
                                               'definition': definition['construct']}})
        self.assertEqual(len([entry for entry in mapped['notes']['entries'] if entry['kind'] == 'footnote-reference']), 1)
        definition = next(entry for entry in mapped['notes']['entries'] if entry['kind'] == 'footnote-definition')
        edit({'kind': 'nativeNote', 'action': {'kind': 'removeNote', 'construct': definition['construct'], 'allReferences': False}})
        self.assertEqual(len([entry for entry in mapped['notes']['entries'] if entry['kind'] == 'footnote-definition']), 1)
        self.assertEqual(len([entry for entry in mapped['notes']['entries'] if entry['kind'] == 'footnote-reference']), 0)
        definition = next(entry for entry in mapped['notes']['entries'] if entry['kind'] == 'footnote-definition')
        edit({'kind': 'nativeNote', 'action': {'kind': 'repeatFootnote', 'at': mapped['bodyInsertion'],
                                               'definition': definition['construct']}})
        definition = next(entry for entry in mapped['notes']['entries'] if entry['kind'] == 'footnote-definition')
        edit({'kind': 'nativeNote', 'action': {'kind': 'removeNote', 'construct': definition['construct'], 'allReferences': True}})
        self.assertFalse(any(entry['kind'].startswith('footnote-') for entry in mapped['notes']['entries']))
        edit({'kind': 'nativeNote', 'action': {'kind': 'insertFootnote', 'at': mapped['bodyInsertion'],
                                               'body': {'mode': 'literal', 'value': 'inserted native note'}}})
        self.assertEqual(len([entry for entry in mapped['notes']['entries'] if entry['kind'] == 'footnote-definition']), 1)
        table = self.table(mapped)
        old_table = table['range']['raw']
        edit({'kind': 'nativeNote', 'action': {'kind': 'wrapCaption', 'object': table['range'],
              'body': {'mode': 'literal', 'value': '表格题注'}, 'category': 'table',
              'position': 'top', 'numbering': {'kind': 'pattern', 'pattern': '1'}}})
        self.assertIn(old_table, source)
        self.assertNotIn('##figure', source)
        figure = next(entry for entry in mapped['notes']['entries'] if entry['kind'] == 'figure')
        edit({'kind': 'nativeNote', 'action': {'kind': 'setCaption', 'construct': figure['construct'],
              'body': {'mode': 'literal', 'value': '修改题注'}, 'position': 'bottom', 'numbering': {'kind': 'none'}}})
        math = next(entry for entry in mapped['structures'] if entry['kind'] == 'displayMath')
        edit({'kind': 'nativeNote', 'action': {'kind': 'setEquationNumbering', 'construct': math['range'],
                                               'numbering': {'kind': 'pattern', 'pattern': '(1)'}}})
        tail = source
        settings = {'scheme': 'decimal', 'start': 3, 'maxLevel': 6, 'excludeLabels': ['first']}
        edit({'kind': 'headingNumbering', 'action': settings})
        self.assertTrue(source.endswith(tail), 'Numbering must keep all original author bytes')
        self.assertEqual(mapped['headingNumbering']['settings'], settings)
        edit({'kind': 'headingNumbering', 'action': {**settings, 'scheme': 'chinese', 'start': 1, 'excludeLabels': []}})
        self.assertTrue(source.endswith(tail))
        self.assertEqual(mapped['headingNumbering']['settings']['scheme'], 'chinese')
        preview = self.call(server, self.endpoint(server, 'render'), basis, token=token)
        self.assertTrue(preview['ok'], preview)
        self.assertEqual(tree_state(root), original)
        self.save_exact(server, doc, source, token)
        self.assertEqual((root / 'Note/Note.typ').read_bytes(), source.encode('utf-8'))

    def test_local_table_topology_draft_and_original_save(self):
        self.table_workflow(False)

    def test_hosted_table_topology_draft_and_original_save(self):
        self.table_workflow(True)

    def test_local_native_notes_caption_and_heading_numbering(self):
        self.notes_workflow(False)

    def test_hosted_native_notes_caption_and_heading_numbering(self):
        self.notes_workflow(True)

    def test_local_dirty_knowledge_read_and_native_target_selector_use_the_current_draft(self):
        server, root, note = self.local_fixture()
        other = self.create_note(server, root, 'Other')
        owner_ref = f'weftext-node://{note["workspaceId"]}/{note["nodeId"]}'
        other_ref = f'weftext-node://{other["workspaceId"]}/{other["nodeId"]}'
        source = (note['source'] + '\n#set heading(numbering: "1")\n'
                  '= 未保存目标😀 <draft-only>\n\n'
                  f'#link("{other_ref}")[跨笔记草稿链接]\n'
                  f'#link("{owner_ref}#draft-only")[本篇未保存标签]\n')
        original = tree_state(root)
        basis, _ = self.observe(server, note, source, 1)
        preview = self.call(server, '/api/render', basis)
        self.assertTrue(preview['ok'], preview)
        context = {**basis, 'baseId': note['baseId'], 'dirty': True}
        result = self.call(server, '/api/knowledge/read', {'context': context})
        citations = result['citations']
        self.assertEqual(citations['displayStatus'], 'nativeCompileAvailable')
        self.assertEqual(citations['editor']['sourceHash'], digest(source))
        self.assertTrue(any(choice['reference'] == other_ref for choice in citations['choices']))
        targets = [target for target in citations['editor']['targets'] if target['label'] == 'draft-only']
        self.assertEqual(len(targets), 1, targets)
        self.assertTrue(targets[0]['unique'])
        self.assertIn('未保存目标😀', targets[0]['range']['raw'])
        fragments = [edge for edge in result['links']['outgoing']
                     if edge['occurrence']['target'] == owner_ref + '#draft-only']
        self.assertEqual(len(fragments), 1, result['links'])
        self.assertEqual(fragments[0]['resolution']['status'], 'resolved')
        proposal = self.call(server, '/api/knowledge/prepare', {
            'context': context, 'operationId': uid(),
            'intent': {'kind': 'nativeReferenceInsert', 'generation': citations['generation'],
                       'at': len(source.encode('utf-8')), 'label': 'draft-only', 'form': 'normal'}})
        self.assertTrue(proposal['draftOnly'], proposal)
        self.assertEqual(proposal['source'], source + '#ref(<draft-only>)')
        self.assertEqual(proposal['context'], context)
        self.assertEqual(tree_state(root), original, 'Knowledge reads and selector proposals must not save the Draft')
        self.assertEqual((root / 'Note/Note.typ').read_bytes(), note['source'].encode('utf-8'))
        self.observe(server, note, source + '\n后续输入保留\n', 2)
        self.call(server, '/api/knowledge/read', {'context': context}, 409)
        self.assertEqual(tree_state(root), original)

    def test_initial_style_original_request_and_sync_inherit_native_resources(self):
        a = self.server('initial-style-source', self.folder / 'state-a')
        root = self.folder / 'library'
        creation = {'path': str(root), 'operationId': uid(),
                    'options': {'initialStyle': {'source': INITIAL_STYLE, 'enabled': True}}}
        receipt = self.call(a, '/api/library/create', creation)
        self.assertEqual(library_fact(self.call(a, '/api/library/create', creation)), library_fact(receipt))
        changed = copy.deepcopy(creation)
        changed['options']['initialStyle']['enabled'] = False
        self.assertEqual(self.call(a, '/api/library/create', changed, 409)['error']['code'], 'operationConflict')
        note = self.create_note(a, root)
        source = note['source'] + '\nInherited native document language 中文😀\n'
        self.save_exact(a, note, source)
        self.assertNotIn('#set text', source)
        self.assertNotIn('library-style', source)
        paths = ['Styles/Styles.typ', 'Styles/library-style.typ',
                 'Styles/weftext.resources.json', '.weftext-settings.json', 'Note/Note.typ']
        originals = {path: (root / path).read_bytes() for path in paths}
        identities = {path: physical_identity(root / path) for path in paths}
        self.assertEqual(originals['Styles/library-style.typ'], INITIAL_STYLE.encode('utf-8'))
        settings = json.loads(originals['.weftext-settings.json'])
        self.assertTrue(settings['defaultStyle']['enabled'])
        self.assertNotIn('locale', settings)
        current = self.view(a, root)
        rendered = self.call(a, '/api/render', {'sessionId': current['sessionId'], 'source': source, 'serial': 1})
        self.assertTrue(rendered['ok'], rendered)
        self.assertIn('INITIAL_STYLE_EXECUTED', tree_text(rendered['tree']))

        binding = self.call(a, '/api/sync/enable', {'root': str(root), 'operationId': uid()})
        objects = binding['installedObjects']
        self.assertTrue(any(item['kind'] == 'settings' for item in objects), objects)
        self.assertGreaterEqual(sum(item['kind'] == 'node' for item in objects), 2, objects)
        exchange = self.folder / 'exchange'
        self.call(a, '/api/sync/export', {'root': str(root), 'directory': str(exchange), 'objects': objects})
        b = self.server('initial-style-receiver', self.folder / 'state-b')
        self.call(b, '/api/sync/receive', {'identity': binding['identity'], 'directory': str(exchange)})
        replica = self.folder / 'replica'
        plan = self.call(b, '/api/operations/prepare', {'root': str(replica), 'operationId': uid(),
                         'intent': {'kind': 'cloneSync', 'identity': binding['identity'], 'objects': objects}})
        self.assertFalse(replica.exists())
        self.commit_original(b, plan)
        for path, raw in originals.items():
            self.assertEqual((replica / path).read_bytes(), raw, path)
        receiver = self.view(b, replica)
        self.assertEqual(receiver['nodeId'], note['nodeId'])
        self.assertEqual(receiver['workspaceId'], note['workspaceId'])
        self.assertEqual(receiver['source'], source)
        self.assertNotEqual(physical_identity(replica / 'Note/Note.typ'), identities['Note/Note.typ'])
        replica_before = tree_state(replica)
        rendered = self.call(b, '/api/render', {'sessionId': receiver['sessionId'], 'source': source, 'serial': 1})
        self.assertTrue(rendered['ok'], rendered)
        self.assertIn('INITIAL_STYLE_EXECUTED', tree_text(rendered['tree']))
        self.assertEqual(tree_state(replica), replica_before)
        for path, raw in originals.items():
            self.assertEqual((root / path).read_bytes(), raw, path)
            self.assertEqual(physical_identity(root / path), identities[path], path)

        # A restart queries the original creation, never installs fresh IDs.
        self.close_server(a)
        a = self.server('initial-style-source-restart', self.folder / 'state-a')
        self.assertEqual(library_fact(self.call(a, '/api/library/create', creation)), library_fact(receipt))
        cancelled = {'path': str(self.folder / 'cancelled-library'), 'operationId': uid(),
                     'options': copy.deepcopy(creation['options'])}
        self.assertEqual(self.call(a, '/api/library/create-cancel', cancelled)['status'], 'resolved')
        self.close_server(a)
        a = self.server('initial-style-cancel-restart', self.folder / 'state-a')
        self.assertEqual(self.call(a, '/api/recovery/inspect', {'operationId': cancelled['operationId']})['status'], 'resolved')
        self.assertEqual(self.call(a, '/api/library/create', cancelled, 409)['error']['code'], 'operationCanceled')
        self.assertFalse(Path(cancelled['path']).exists())

    def test_initial_style_hosted_execution_source_read_and_note_rights_are_independent(self):
        local = self.server('initial-style-local', self.folder / 'state')
        root = self.folder / 'library'
        self.call(local, '/api/library/create', {'path': str(root), 'operationId': uid(),
                  'options': {'initialStyle': {'source': INITIAL_STYLE, 'enabled': True}}})
        note = self.create_note(local, root)
        saved = note['source'] + '\nShared author content 中文😀\n'
        self.save_exact(local, note, saved)
        self.close_server(local)
        original = tree_state(root)
        tokens = {name: secrets.token_urlsafe(32) for name in ('alice', 'reader', 'style-reader', 'admin')}
        config = {'deployment': 'v061-initial-style', 'bind': '127.0.0.1:0', 'root': str(root),
                  'adminTokenSha256': digest(tokens['admin']),
                  'files': {'Note': {'path': 'Note/Note.typ', 'title': 'Note', 'editable': True}},
                  'users': {
                      'alice': {'enabled': True, 'tokenSha256': digest(tokens['alice']),
                                'grants': {'Note': {'read': True, 'write': True}}, 'library': {'styleUse': True}},
                      'reader': {'enabled': True, 'tokenSha256': digest(tokens['reader']),
                                 'grants': {'Note': {'read': True}}},
                      'style-reader': {'enabled': True, 'tokenSha256': digest(tokens['style-reader']),
                                       'grants': {}, 'library': {'style': {'read': True}}}}}
        config_path = self.folder / 'server.json'
        write_json(config_path, config)
        server = self.server('initial-style-hosted', self.folder / 'state', config_path)
        alice = self.view(server, root, token=tokens['alice'])
        basis = {'sessionId': alice['sessionId'], 'source': saved, 'serial': 1}
        rendered = self.call(server, '/api/hosted/editor/render', basis, token=tokens['alice'])
        self.assertTrue(rendered['ok'], rendered)
        self.assertIn('INITIAL_STYLE_EXECUTED', tree_text(rendered['tree']))
        for hidden in ('INITIAL_STYLE_PRIVATE_SOURCE_COMMENT', str(root), str(self.folder / 'state')):
            self.assertNotIn(hidden, json.dumps(rendered, ensure_ascii=False))
        self.call(server, '/api/hosted/library/read', {'role': 'style'}, 404, tokens['alice'])
        attack = {**basis, 'serial': 2, 'source': saved + '\n#read("/Styles/library-style.typ")\n'}
        denied = self.call(server, '/api/hosted/editor/render', attack, token=tokens['alice'])
        self.assertFalse(denied['ok'], denied)
        self.assertNotIn('INITIAL_STYLE_PRIVATE_SOURCE_COMMENT', json.dumps(denied))
        personal = saved + '\nAlice retained personal input\n'
        self.call(server, '/api/hosted/draft', {'inputId': str(uuid.uuid4()), 'baseId': alice['baseId'], 'inputRevision': 1, 'sessionId': alice['sessionId'], 'source': personal}, token=tokens['alice'])

        reader = self.view(server, root, token=tokens['reader'])
        reader_basis = {'sessionId': reader['sessionId'], 'source': saved, 'serial': 1}
        self.call(server, '/api/hosted/editor/render', reader_basis, 404, tokens['reader'])
        style = self.call(server, '/api/hosted/library/read', {'role': 'style'}, token=tokens['style-reader'])
        self.assertEqual(style['source'], INITIAL_STYLE)
        self.call(server, '/api/hosted/open', {'resource': 'Note'}, 404, tokens['style-reader'])
        policy = self.call(server, '/api/hosted/admin/policy', token=tokens['admin'])
        users = copy.deepcopy(policy['users'])
        users['reader']['library'] = {'styleUse': True}
        self.call(server, '/api/hosted/admin/policy', {'expectedGeneration': policy['generation'], 'users': users}, token=tokens['admin'])
        reader = self.view(server, root, token=tokens['reader'])
        rendered = self.call(server, '/api/hosted/editor/render', {**reader_basis, 'sessionId': reader['sessionId']}, token=tokens['reader'])
        self.assertTrue(rendered['ok'], rendered)
        self.assertIn('INITIAL_STYLE_EXECUTED', tree_text(rendered['tree']))

        # The cached successful image/tree is no longer deliverable to the old
        # actor session after style execution is revoked; its Draft survives.
        policy = self.call(server, '/api/hosted/admin/policy', token=tokens['admin'])
        users = copy.deepcopy(policy['users'])
        users['alice']['library']['styleUse'] = False
        self.call(server, '/api/hosted/admin/policy', {'expectedGeneration': policy['generation'], 'users': users}, token=tokens['admin'])
        self.call(server, '/api/hosted/editor/render', {**basis, 'source': attack['source'], 'serial': 2}, 410, tokens['alice'])
        reopened = self.view(server, root, token=tokens['alice'])
        self.assertEqual(reopened['recoveredDraft'], personal)
        self.call(server, '/api/hosted/editor/render', {'sessionId': reopened['sessionId'], 'source': personal, 'serial': 1}, 404, tokens['alice'])
        self.assertEqual(tree_state(root), original)

    def test_local_old_serial_and_late_projection_are_rejected(self):
        server, root, _ = self.local_fixture()
        self.old_serial_and_late_projection(server, root)

    def test_hosted_old_serial_readonly_and_revocation(self):
        server, root, _, _, tokens = self.hosted_fixture()
        self.old_serial_and_late_projection(server, root, tokens['alice'])
        reader = self.view(server, root, token=tokens['reader'])
        source = reader['source'] + TABLE_BODY + '\n#footnote[reader note]\n'
        basis, mapped = self.observe(server, reader, source, 1, tokens['reader'])
        table = next(entry for entry in mapped['structures'] if entry['kind'] == 'table')
        self.assertFalse(table['table']['topologyWritable'])
        self.assertFalse(mapped['headingNumbering']['editable'])

        def assert_readonly(value):
            if isinstance(value, dict):
                for key, item in value.items():
                    if key in ('writable', 'gridWritable', 'topologyWritable', 'levelWritable'):
                        self.assertFalse(item, f'Reader flag {key}')
                    assert_readonly(item)
            elif isinstance(value, list):
                for item in value:
                    assert_readonly(item)

        assert_readonly(mapped)
        request = {**basis, 'sourceHash': mapped['sourceHash'], 'intent': {
            'kind': 'tableEdit', 'range': table['range'], 'action': {'kind': 'transpose'}}}
        self.call(server, '/api/hosted/editor/edit', request, 404, tokens['reader'])
        alice = self.view(server, root, token=tokens['alice'])
        alice_basis, alice_map = self.observe(server, alice, source, 1, tokens['alice'])
        alice_request = {**request, **alice_basis, 'sourceHash': alice_map['sourceHash']}
        self.call(server, '/api/hosted/editor/edit', alice_request, 410, tokens['bob'])
        policy = self.call(server, '/api/hosted/admin/policy', token=tokens['admin'])
        users = copy.deepcopy(policy['users'])
        users['alice']['grants']['Note']['write'] = False
        self.call(server, '/api/hosted/admin/policy', {'expectedGeneration': policy['generation'], 'users': users}, token=tokens['admin'])
        self.call(server, '/api/hosted/editor/edit', alice_request, 410, tokens['alice'])
        self.call(server, '/api/hosted/editor/map', alice_basis, 410, tokens['alice'])
        bob = self.view(server, root, 'Other', tokens['bob'])
        self.save_exact(server, bob, bob['source'] + '\nUnrelated actor still saves\n', tokens['bob'])

    def old_serial_and_late_projection(self, server, root, token=None):
        doc = self.view(server, root, token=token)
        source = doc['source'] + TABLE_BODY
        before = tree_state(root)
        basis, mapped = self.observe(server, doc, source, 1, token)
        intent = {'kind': 'tableEdit', 'range': self.table(mapped)['range'], 'action': {'kind': 'transpose'}}
        request = {**basis, 'sourceHash': mapped['sourceHash'], 'intent': intent}
        self.observe(server, doc, source + '\nnew draft\n', 2, token)
        self.call(server, self.endpoint(server, 'edit'), request, 409, token)
        self.call(server, self.endpoint(server, 'map'), {**basis, 'serial': 2}, 409, token)
        self.observe(server, doc, source, 3, token)
        self.call(server, self.endpoint(server, 'edit'), request, 409, token)
        slow = {**basis, 'source': '#for i in range(3000) [delayed paragraph #parbreak()]', 'serial': 4}
        with ThreadPoolExecutor(max_workers=1) as pool:
            pending = pool.submit(self.call, server, self.endpoint(server, 'render'), slow, 409, token)
            time.sleep(.2)
            self.observe(server, doc, source + '\nnewest input\n', 5, token)
            result = pending.result(timeout=45)
            self.assertIn('error', result)
        self.assertEqual(tree_state(root), before)

    def test_sync_keeps_personal_draft_identity_and_rejects_old_ranges(self):
        a, root_a, note = self.local_fixture()
        b = self.server('receiver', self.folder / 'state-b')
        root_b = self.folder / 'replica'
        source = note['source'] + TABLE_BODY
        initial, _ = self.save_exact(a, note, source)
        sender = initial['document']
        self.assertEqual(sender['sessionId'], note['sessionId'])
        self.assertEqual(sender['source'], source)
        self.assertEqual(sender['baseId'], initial['baseId'])
        key = {'kind': 'node', 'id': note['nodeId']}
        binding = self.call(a, '/api/sync/enable', {'root': str(root_a), 'operationId': uid()})
        identity = binding['identity']
        number = 0

        def transfer():
            nonlocal number
            number += 1
            directory = self.folder / f'exchange-{number:02}'
            self.call(a, '/api/sync/export', {'root': str(root_a), 'directory': str(directory), 'objects': [key]})
            return self.call(b, '/api/sync/receive', {'identity': identity, 'directory': str(directory)})

        transfer()
        clone = self.call(b, '/api/operations/prepare', {'root': str(root_b), 'operationId': uid(),
                          'intent': {'kind': 'cloneSync', 'identity': identity, 'objects': [key]}})
        self.assertFalse(root_b.exists())
        self.commit_original(b, clone)
        receiver = self.view(b, root_b)
        self.assertEqual(receiver['nodeId'], note['nodeId'])
        self.assertEqual(receiver['workspaceId'], note['workspaceId'])
        old_basis, old_map = self.observe(b, receiver, source, 1)
        old_range = self.table(old_map)['range']
        draft = source + '\nB personal retained input 中文😀\n'
        self.call(b, '/api/draft', {'inputId': str(uuid.uuid4()), 'inputRevision': 1, 'sessionId': receiver['sessionId'], 'baseId': receiver['baseId'], 'source': draft})
        path = root_b / 'Note/Note.typ'
        file_identity = physical_identity(path)
        directory_identity = physical_identity(path.parent)
        raw = path.parent / 'personal.raw'
        raw.write_bytes(b'owned private raw')
        raw_identity = physical_identity(raw)
        sender_basis, sender_map = self.observe(a, sender, source, 1)
        updated = self.propose(a, sender_basis, sender_map, {'kind': 'tableEdit', 'range': self.table(sender_map)['range'],
                               'action': {'kind': 'moveRow', 'from': 0, 'to': 1}})['source']
        updated = updated.replace('\n#table(', '\nSynchronized prefix moves the exact source range\n\n#table(', 1)
        self.observe(a, sender, updated, 2)
        self.save_exact(a, sender, updated)
        before_receive = tree_state(root_b)
        transfer()
        self.assertEqual(tree_state(root_b), before_receive, 'Receive may not install author bytes')
        plan = self.call(b, '/api/operations/prepare', {'root': str(root_b), 'operationId': uid(),
                         'intent': {'kind': 'applySync', 'identity': identity, 'objects': [key]}})
        receipt = self.commit_original(b, plan)
        self.assertEqual(path.read_bytes(), updated.encode('utf-8'))
        self.assertEqual(physical_identity(path), file_identity)
        self.assertEqual(physical_identity(path.parent), directory_identity)
        self.assertEqual(physical_identity(raw), raw_identity)
        self.assertEqual(raw.read_bytes(), b'owned private raw')
        reopened = self.view(b, root_b)
        self.assertEqual(reopened['nodeId'], note['nodeId'])
        self.assertEqual(reopened['recoveredDraft'], draft)
        self.assertEqual(reopened['recoveredDraftBaseSource'], source)
        self.assertTrue(reopened['recoveredDraftConflict'])
        current_basis, current_map = self.observe(b, reopened, updated, 2)
        self.propose(b, current_basis, current_map, {'kind': 'tableEdit', 'range': old_range,
                                                  'action': {'kind': 'transpose'}}, status=422)
        self.call(b, '/api/edit', {**current_basis, 'sourceHash': old_map['sourceHash'],
                  'intent': {'kind': 'tableEdit', 'range': old_range, 'action': {'kind': 'transpose'}}}, 409)
        self.call(b, '/api/save', {'inputRevision': 1, 'sessionId': receiver['sessionId'], 'baseId': receiver['baseId'],
                  'source': draft, 'operationId': uid()}, 409)
        self.assertEqual(path.read_bytes(), updated.encode('utf-8'))
        self.close_server(b)
        b = self.server('receiver-restart', self.folder / 'state-b')
        self.assertEqual(self.call(b, '/api/operations/status', {'operationId': plan['operationId']}), receipt)
        recovered = self.call(b, '/api/sync/operation', {'operationId': plan['operationId']})
        self.assertEqual(recovered['plan']['planId'], plan['planId'])
        self.assertEqual(recovered['request']['intent']['kind'], 'applySync')
        self.assertEqual(self.commit_original(b, plan), receipt)
        self.assertEqual(physical_identity(path), file_identity)
        self.assertEqual(self.view(b, root_b)['recoveredDraft'], draft)

    def commit_original(self, server, plan):
        receipt = self.call(server, '/api/operations/commit', {'operationId': plan['operationId'], 'planId': plan['planId']})
        self.assertEqual(receipt['status'], 'committed')
        self.assertEqual(receipt['operationId'], plan['operationId'])
        self.assertEqual(receipt['planId'], plan['planId'])
        return receipt


CASES = sorted(name for name in RetainedHTTP.__dict__ if name.startswith('test_'))


def main():
    global EVIDENCE
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('exe', nargs='?', help='Exact candidate executable to exercise')
    parser.add_argument('tests', nargs='*', help='Exact test names from --list')
    parser.add_argument('--evidence-dir', type=Path, help='Existing delivery evidence parent; a fresh owned child is retained')
    parser.add_argument('--list', action='store_true', help='List exact cases without starting a program')
    args = parser.parse_args()
    if args.list:
        print('\n'.join(CASES))
        return 0
    if not args.exe:
        parser.error('exe is required unless --list is used')
    unknown = set(args.tests) - set(CASES)
    if unknown:
        parser.error('Unknown test names: ' + ', '.join(sorted(unknown)))
    exe = Path(args.exe).resolve(strict=True)
    if not exe.is_file():
        parser.error('exe must be a file')
    test_hosted.EXE = exe
    selected = args.tests or CASES
    parent = args.evidence_dir or (Path(os.environ['WEFTEXT_V061_EVIDENCE_DIR'])
                                    if os.environ.get('WEFTEXT_V061_EVIDENCE_DIR') else None)
    if parent is not None:
        parent = parent.resolve()
        parent.mkdir(parents=True, exist_ok=True)
        EVIDENCE = parent / ('authoring-' + uid()[:8])
        EVIDENCE.mkdir()
    else:
        EVIDENCE = Path(tempfile.mkdtemp(prefix='weftext-v061-authoring-'))
    write_json(EVIDENCE / 'candidate.json', {'exe': str(exe), 'sha256': hashlib.sha256(exe.read_bytes()).hexdigest(),
               'bytes': exe.stat().st_size, 'tests': selected, 'python': sys.version, 'platform': sys.platform,
               'evidenceScope': 'actual HTTP only; keyboard/IME/clipboard/Undo UI/Office are separate acceptance'})
    print('Retained v0.6.1 authoring evidence:', EVIDENCE, flush=True)
    suite = unittest.TestSuite(RetainedHTTP(name) for name in selected)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    write_json(EVIDENCE / 'result.json', {'successful': result.wasSuccessful(), 'testsRun': result.testsRun,
               'failures': [{'test': str(test), 'traceback': detail} for test, detail in result.failures],
               'errors': [{'test': str(test), 'traceback': detail} for test, detail in result.errors],
               'skipped': [{'test': str(test), 'reason': reason} for test, reason in result.skipped]})
    return 0 if result.wasSuccessful() else 1


if __name__ == '__main__':
    raise SystemExit(main())
