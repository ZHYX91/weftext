"""Real HTTP proofreading/Word review checks; native Office is a separate gate."""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import unittest
import xml.etree.ElementTree as ET
import test_local_features as features
import test_outputs as outputs

W, R, P, CT = outputs.W, outputs.R, outputs.P, outputs.CT


def fixture():
    return outputs.package({
        '[Content_Types].xml': f'<Types xmlns="{CT}"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/comments.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml"/></Types>',
        '_rels/.rels': f'<Relationships xmlns="{P}"><Relationship Id="doc" Type="{R}/officeDocument" Target="word/document.xml"/></Relationships>',
        'word/_rels/document.xml.rels': f'<Relationships xmlns="{P}"><Relationship Id="comments" Type="{R}/comments" Target="comments.xml"/></Relationships>',
        'word/document.xml': f'<w:document xmlns:w="{W}"><w:body><w:p><w:commentRangeStart w:id="0"/><w:r><w:t>原文😀</w:t></w:r><w:commentRangeEnd w:id="0"/><w:r><w:commentReference w:id="0"/></w:r><w:ins w:id="1" w:author="甲" w:date="2026-10-14T00:00:00Z"><w:r><w:t>新增</w:t></w:r></w:ins><w:r><w:t>后文</w:t></w:r><w:del w:id="2" w:author="乙" w:date="2026-10-14T00:00:00Z"><w:r><w:delText>删除</w:delText></w:r></w:del></w:p><w:sectPr/></w:body></w:document>',
        'word/comments.xml': f'<w:comments xmlns:w="{W}"><w:comment w:id="0" w:author="原审阅人" w:date="2026-10-14T00:00:00Z"><w:p><w:r><w:t>请核对原文</w:t></w:r></w:p></w:comment></w:comments>',
    })


class ReviewInterchange(outputs.Outputs):
    def scan(self, doc, rules, serial=1, **options):
        context = self.context(doc, serial=serial)
        render = self.api('/api/render', {'sessionId': doc['sessionId'], 'source': doc['source'], 'serial': serial})
        self.assertTrue(render['ok'], render.get('diagnostics'))
        settings = {'characters': True, 'typos': True, 'pairs': True, 'sensitive': True,
                    'rules': rules, 'allowlist': [], 'selection': None, **options}
        result = self.api('/api/knowledge/proofread', {'context': context, 'options': settings})
        return context, settings, result

    def batch(self, context, options, findings, **extra):
        request = {'operationId': features.uid(), 'context': context,
                   'intent': {'kind': 'annotationProofread', 'options': options, 'findings': findings, **extra}}
        preview = self.api('/api/knowledge/prepare', request)
        self.commit(preview, context)
        return request, preview

    def imported(self, payload=None, name='Word导入'):
        root, _ = self.library()
        doc = self.note(root, '入口')
        original = fixture() if payload is None else payload
        self.keep('word-original.docx' if payload is None else name+'-original.docx', original)
        request, preview = self.prepare_exchange(doc, root, name, 'docx', original)
        self.assertEqual(preview['kind'], 'importDocx')
        if payload is None:
            self.assertEqual(preview['contentPreview']['comments'], 1)
            self.assertEqual(preview['contentPreview']['revisions'], 2)
            self.assertIn('原文😀后文删除', preview['contentPreview']['source'])
        self.commit_exchange(preview)
        self.assertEqual(self.api('/api/exchange/prepare', request)['planId'], preview['planId'])
        path = root / name / 'document/document.typ'
        new = self.api('/api/open', {'root': str(root), 'path': str(path)})
        self.assertEqual((path.parent / 'original.docx').read_bytes(), original)
        return root, new

    def test_01_checks_batch_deduplicates_and_keeps_replies(self):
        root, _ = self.library()
        doc = self.note(root, '校对', '这是错字。再次错字。\n\n交叉 ([)]。\n\n`错字`\n')
        rules = [{'id': 'typo.one', 'version': '1', 'category': 'typo', 'text': '错字', 'message': '疑似词语', 'replacement': '正字'}]
        context, options, scan = self.scan(doc, rules)
        typos = [f for f in scan['findings'] if f['category'] == 'typo']
        self.assertEqual(len(typos), 2, scan)
        self.assertTrue(all(f['sourceRange'] for f in typos), scan)
        ids = [f['id'] for f in scan['findings'] if f['sourceRange']]
        request, preview = self.batch(context, options, ids)
        self.assertEqual(self.api('/api/knowledge/prepare', request)['planId'], preview['planId'])
        annotation_path = Path(doc['path']).parent / 'weftext.annotations.json'
        first = json.loads(annotation_path.read_text('utf-8'))
        doc = self.api('/api/open', {'root': str(root), 'path': doc['path']})
        context, options, scan = self.scan(doc, rules)
        self.assertEqual(set(scan['existing']), set(ids))
        self.batch(context, options, ids)
        self.assertEqual(json.loads(annotation_path.read_text('utf-8')), first)
        doc = self.api('/api/open', {'root': str(root), 'path': doc['path']})
        context = self.context(doc)
        record = first['annotations'][0]
        reply = self.api('/api/knowledge/prepare', {'operationId': features.uid(), 'context': context,
                         'intent': {'kind': 'annotationReply', 'annotationId': record['annotationId'], 'body': '人工复核意见'}})
        self.commit(reply, context)
        doc = self.api('/api/open', {'root': str(root), 'path': doc['path']})
        context, options, scan = self.scan(doc, rules)
        self.batch(context, options, ids)
        records = json.loads(annotation_path.read_text('utf-8'))['annotations']
        self.assertEqual(len(records), len(first['annotations']) + 1)
        self.assertEqual(records[-1]['body'], '人工复核意见')
        doc = self.api('/api/open', {'root': str(root), 'path': doc['path']})
        _, result = self.prepare_output(doc, 'docx', reviewMode='comments')
        payload = self.download_output(result, 'proofreading-comments.docx')
        parts = outputs.unpack(payload)
        comments = ET.fromstring(parts['word/comments.xml'])
        self.assertEqual(len(comments.findall(f'{{{W}}}comment')), len(records))
        self.assertIn('人工复核意见', outputs.xml_text(comments))
        _, reimport = self.prepare_exchange(doc, root, '批注重入', 'docx', payload)
        self.commit_exchange(reimport)
        after = json.loads((root / '批注重入/document/weftext.annotations.json').read_text('utf-8'))['annotations']
        reply = next(r for r in after if r['body'] == '人工复核意见')
        parent = next(r for r in after if r['annotationId'] == reply['replyTo'])
        self.assertEqual(reply['target'], parent['target'])
        self.assertEqual(parent['target']['anchor']['sourceRange']['raw'], '错字')

    def test_02_cross_format_and_old_results(self):
        root, _ = self.library()
        doc = self.note(root, '跨格式', '中文错*别*字。\n\n独立段落错\n\n别字。')
        rules = [{'id': 'typo.formatted', 'version': '1', 'category': 'typo', 'text': '错别字', 'message': '核对', 'replacement': '正确文字'}]
        context, options, scan = self.scan(doc, rules)
        hits = [f for f in scan['findings'] if f['ruleId'] == 'typo.formatted']
        self.assertEqual(len(hits), 1, scan)
        self.assertIsNotNone(hits[0]['sourceRange'], scan)
        self.assertFalse(hits[0]['executable'])
        changed = self.saved(doc, doc['source'] + '\n\n新的正文', root)
        self.api('/api/knowledge/prepare', {'operationId': features.uid(), 'context': context,
                 'intent': {'kind': 'annotationProofread', 'options': options, 'findings': [hits[0]['id']]}}, status=(409, 400))
        self.assertEqual(Path(changed['path']).read_text('utf-8'), changed['source'])

    def test_03_word_import_and_tracked_export_preserve_pending(self):
        root, doc = self.imported()
        self.assertIn('原文😀后文删除', doc['source'])
        self.assertNotIn('新增', doc['source'])
        records = json.loads((Path(doc['path']).parent / 'weftext.annotations.json').read_text('utf-8'))['annotations']
        self.assertEqual(len(records), 3)
        self.assertTrue(all(not r['suggestion']['requiresReconfirmation'] for r in records if r['suggestion']))
        _, result = self.prepare_output(doc, 'docx', reviewMode='changes')
        payload = self.download_output(result, 'word-imported-review.docx')
        parts = outputs.unpack(payload)
        main = ET.fromstring(parts['word/document.xml'])
        self.assertEqual(''.join(outputs.xml_text(n) for n in main.iter(f'{{{W}}}ins')), '新增')
        self.assertEqual(''.join(n.text or '' for n in main.iter(f'{{{W}}}delText')), '删除')
        self.assertEqual(len(ET.fromstring(parts['word/comments.xml']).findall(f'{{{W}}}comment')), 3)
        self.assertEqual(Path(doc['path']).read_text('utf-8'), doc['source'])

    def test_04_accept_insert_and_reject_delete(self):
        root, doc = self.imported()
        context = self.context(doc)
        path = Path(doc['path']).parent / 'weftext.annotations.json'
        records = json.loads(path.read_text('utf-8'))['annotations']
        insertion = next(r for r in records if r.get('wordImport', {}).get('revisionKind') == 'insert')
        deletion = next(r for r in records if r.get('wordImport', {}).get('revisionKind') == 'delete')
        rejected = self.api('/api/knowledge/prepare', {'operationId': features.uid(), 'context': context,
                            'intent': {'kind': 'annotationReject', 'annotationId': deletion['annotationId']}})
        self.commit(rejected, context)
        accepted = self.api('/api/knowledge/prepare', {'operationId': features.uid(), 'context': context,
                            'intent': {'kind': 'annotationAccept', 'annotationId': insertion['annotationId']}})
        self.commit(accepted, context)
        self.assertIn('原文😀新增后文删除', Path(doc['path']).read_text('utf-8'))
        stored = json.loads(path.read_text('utf-8'))['annotations']
        self.assertEqual(next(r for r in stored if r['annotationId'] == insertion['annotationId'])['suggestion']['state'], 'accepted')
        self.assertEqual(next(r for r in stored if r['annotationId'] == deletion['annotationId'])['suggestion']['state'], 'rejected')

    def test_05_sensitive_selection_and_persistent_ignore(self):
        root, _ = self.library()
        doc = self.note(root, '敏感与白名单', '公开 秘密 😀秘密 秘密。\n')
        rules = [{'id':'sensitive.one','version':'1','category':'sensitive','text':'秘密','message':'按当前词表核查','replacement':None}]
        start = doc['source'].encode().index('😀秘密'.encode())
        context, options, scan = self.scan(doc, rules, selection=[start, start + len('😀秘密'.encode())])
        self.assertEqual(len(scan['findings']), 1, scan)
        self.assertFalse(scan['findings'][0]['executable'])
        self.batch(context, options, [scan['findings'][0]['id']], ignore=True)
        doc = self.api('/api/open', {'root':str(root),'path':doc['path']})
        _, _, again = self.scan(doc, rules)
        self.assertEqual(len(again['existing']), 1)
        self.assertEqual(len(again['findings']), 3)
        stored = json.loads((Path(doc['path']).parent/'weftext.annotations.json').read_text('utf-8'))['annotations']
        self.assertEqual(stored[0]['reviewState'], 'resolved')
        self.assertIsNone(stored[0]['suggestion'])
        self.assertEqual(Path(doc['path']).read_text('utf-8'), doc['source'])
        _, _, allowed = self.scan(doc, rules, allowlist=['秘密'])
        self.assertFalse(allowed['findings'])

    def test_06_stale_insertion_requires_explicit_current_point(self):
        root, doc = self.imported()
        records = json.loads((Path(doc['path']).parent/'weftext.annotations.json').read_text('utf-8'))['annotations']
        insertion = next(r for r in records if r.get('wordImport',{}).get('revisionKind')=='insert')
        doc = self.saved(doc, doc['source']+'\n\n新段落', root)
        context = self.context(doc)
        self.api('/api/knowledge/prepare', {'operationId':features.uid(),'context':context,'intent':{'kind':'annotationAccept','annotationId':insertion['annotationId']}},status=(400,409))
        at = len(doc['source'].split('后文')[0].encode('utf-16-le')) // 2
        preview = self.api('/api/knowledge/prepare', {'operationId':features.uid(),'context':context,'intent':{'kind':'annotationReconfirm','annotationId':insertion['annotationId'],'target':{'kind':'source_point','from':at,'to':at},'replacement':'新确认'}})
        self.commit(preview,context)
        doc = self.api('/api/open', {'root':str(root),'path':doc['path']}); context=self.context(doc)
        preview=self.api('/api/knowledge/prepare', {'operationId':features.uid(),'context':context,'intent':{'kind':'annotationAccept','annotationId':insertion['annotationId']}})
        self.commit(preview,context)
        self.assertIn('原文😀新确认后文删除',Path(doc['path']).read_text('utf-8'))

    def test_07_adjacent_replacement_and_empty_body_insertion(self):
        for name, body, expected, count in [
            ('adjacent-replacement', '<w:r><w:t>前文</w:t></w:r><w:del w:id="1"><w:r><w:delText>旧字</w:delText></w:r></w:del><w:ins w:id="2"><w:r><w:t>新字</w:t></w:r></w:ins><w:r><w:t>后文</w:t></w:r>', '新字', 2),
            ('empty-body-insertion', '<w:ins w:id="1"><w:r><w:t>全新正文😀</w:t></w:r></w:ins>', '全新正文😀', 1),
        ]:
            parts=outputs.unpack(fixture())
            parts['word/document.xml']=f'<w:document xmlns:w="{W}"><w:body><w:p>{body}</w:p><w:sectPr/></w:body></w:document>'.encode()
            parts['word/comments.xml']=f'<w:comments xmlns:w="{W}"/>'.encode()
            root, doc=self.imported(outputs.package(parts), name)
            _, output=self.prepare_output(doc,'docx',reviewMode='changes')
            exported=outputs.unpack(self.download_output(output,name+'.docx'))
            main=ET.fromstring(exported['word/document.xml'])
            self.assertEqual(''.join(outputs.xml_text(e) for e in main.iter(f'{{{W}}}ins')),expected)
            self.assertEqual(len(list(main.iter(f'{{{W}}}ins')))+len(list(main.iter(f'{{{W}}}del'))),count)
            if count==1:
                records=json.loads((Path(doc['path']).parent/'weftext.annotations.json').read_text('utf-8'))['annotations']
                context=self.context(doc)
                preview=self.api('/api/knowledge/prepare',{'operationId':features.uid(),'context':context,'intent':{'kind':'annotationAccept','annotationId':records[0]['annotationId']}})
                self.commit(preview,context)
                self.assertTrue(Path(doc['path']).read_text('utf-8').endswith(expected))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path)
    parser.add_argument('--artifacts', type=Path)
    parser.add_argument('--only', action='append', default=[])
    args = parser.parse_args()
    features.EXE = args.executable.resolve(strict=True)
    if args.artifacts:
        args.artifacts.mkdir(parents=True, exist_ok=True)
        features.ARTIFACT_PARENT = args.artifacts.resolve()
    names = sorted(n for n in ReviewInterchange.__dict__ if n.startswith('test_'))
    if args.only:
        names = [n for n in names if n in args.only]
        if not names:
            parser.error('No tests match --only; pass full method names')
    result = unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite(ReviewInterchange(n) for n in names))
    if hasattr(ReviewInterchange, 'root'):
        (ReviewInterchange.root / 'results.json').write_text(json.dumps({'testsRun': result.testsRun, 'successful': result.wasSuccessful(),
            'executableSha256': hashlib.sha256(features.EXE.read_bytes()).hexdigest(), 'failures': [(str(t), s) for t, s in result.failures],
            'errors': [(str(t), s) for t, s in result.errors], 'nativeOffice': 'not run by HTTP suite'}, ensure_ascii=False, indent=2), 'utf-8')
    sys.exit(0 if result.wasSuccessful() else 1)
