"""Stage-one exact native edits and original local Query reservation recovery.

Own fixtures are retained; no foreground input, real libraries or cleanup.
"""
import json
from pathlib import Path
import sys
import tempfile
import unittest
import uuid
from test_hosted import Server

def uid(): return str(uuid.uuid4())

class Everyday(unittest.TestCase):
    def setUp(self):
        self.folder = Path(tempfile.mkdtemp(prefix='weftext-everyday-'))
        self.root = self.folder/'library'
        self.data = self.folder/'state'
        self.server = Server(['--no-browser','--data-dir',str(self.data)])
        self.call('/api/library/create',{'path':str(self.root),'operationId':uid()})
        self.doc = self.call('/api/note/create',{'root':str(self.root),'parent':str(self.root),'name':'TEST-note','operationId':uid()})
        self.serial = 0
        print('Everyday evidence:',self.folder,flush=True)
    def tearDown(self): self.server.close()
    def call(self,route,body=None,status=200):
        code,value,_=self.server.call(route,body)
        self.assertEqual(code,status,value)
        return value
    def basis(self,source=None):
        self.serial+=1
        return {'sessionId':self.doc['sessionId'],'source':self.doc['source'] if source is None else source,'serial':self.serial}
    def install_schema(self):
        read=self.call('/api/data/schema',{'root':str(self.root)})
        op=uid();plan=self.call('/api/data/schema/prepare',{'root':str(self.root),'source':read['source'],'beforeSource':read['source'],'beforeRevision':read.get('revision'),'operationId':op})
        self.call('/api/data/schema/install',{'operationId':op,'planId':plan['planId']})
    def proposal(self,basis,operation=None):
        request={**basis,'baseId':self.doc['baseId'],'operationId':operation or uid(),'label':'weftext-query-test','plan':{'scope':{'descendants':True,'lifecycle':'live'},'types':[],'select':[{'name':'title','expr':'title'}],'order':[],'take':100}}
        return request,self.call('/api/query/definition/draft',request)
    def test_exact_block_commands_stale_refusal_and_no_implicit_save(self):
        original=Path(self.doc['path']).read_bytes()
        source=self.doc['source']+'\n\n第一段😀\n\n= 第二段\n\n尾段'
        basis=self.basis(source);mapping=self.call('/api/editor-map',basis)
        block=next(b for b in mapping['bodyBlocks'] if b['range']['raw']=='第一段😀')
        request={**basis,'sourceHash':mapping['sourceHash'],'intent':{'kind':'convertBodyBlock','range':block['range'],'target':'heading','level':2}}
        changed=self.call('/api/edit',request)
        self.assertEqual(changed['source'],source.replace('第一段😀','== 第一段😀'))
        next_basis=self.basis(changed['source']);mapped=self.call('/api/editor-map',next_basis)
        block=next(b for b in mapped['bodyBlocks'] if b['range']['raw']=='= 第二段')
        moved=self.call('/api/edit',{**next_basis,'sourceHash':mapped['sourceHash'],'intent':{'kind':'moveBodyBlock','range':block['range'],'direction':'up'}})
        self.assertIn('= 第二段\n\n== 第一段😀',moved['source'])
        self.call('/api/edit',request,409)
        self.assertEqual(Path(self.doc['path']).read_bytes(),original)
    def test_query_draft_original_key_recovery_after_restart_and_normal_save(self):
        self.install_schema();original=Path(self.doc['path']).read_bytes();basis=self.basis()
        request,proposal=self.proposal(basis)
        replay=self.call('/api/query/definition/draft',request)
        self.assertEqual(proposal['key'],replay['key']);self.assertEqual(proposal['source'],replay['source'])
        self.assertTrue(proposal['draftOnly']);self.assertEqual(Path(self.doc['path']).read_bytes(),original)
        pending=self.call('/api/query/definition/pending',self.basis(proposal['source']))
        self.assertEqual(pending['operationId'],request['operationId'])
        self.server.close();self.server=Server(['--no-browser','--data-dir',str(self.data)])
        self.doc=self.call('/api/open',{'path':self.doc['path'],'root':str(self.root)});self.serial=0
        pending=self.call('/api/query/definition/pending',self.basis(proposal['source']))
        self.assertEqual(pending['operationId'],request['operationId'])
        self.call('/api/save',{'sessionId':self.doc['sessionId'],'baseId':self.doc['baseId'],'source':proposal['source'],'operationId':pending['operationId']})
        owner=f"weftext-node://{self.doc['workspaceId']}/{self.doc['nodeId']}"
        definitions=self.call('/api/query/definitions',{'root':str(self.root),'owner':owner})
        self.assertEqual([d['key'] for d in definitions['definitions']],[proposal['key']])
        self.assertIsNone(self.call('/api/query/definition/pending',self.basis(proposal['source']))['operationId'])
    def test_undo_query_then_save_other_text_does_not_consume_original_key(self):
        self.install_schema();request,proposal=self.proposal(self.basis())
        ordinary=self.doc['source']+'\n\n独立正文'
        self.assertIsNone(self.call('/api/query/definition/pending',self.basis(ordinary))['operationId'])
        self.call('/api/save',{'sessionId':self.doc['sessionId'],'baseId':self.doc['baseId'],'source':ordinary,'operationId':uid()})
        self.doc=self.call('/api/open',{'path':self.doc['path'],'root':str(self.root)});self.serial=0
        restored=proposal['source']+'\n\n独立正文'
        pending=self.call('/api/query/definition/pending',self.basis(restored));self.assertEqual(pending['operationId'],request['operationId'])
        self.call('/api/save',{'sessionId':self.doc['sessionId'],'baseId':self.doc['baseId'],'source':restored,'operationId':pending['operationId']})
        self.assertEqual(Path(self.doc['path']).read_text('utf-8'),restored)
    def test_missing_schema_and_foreign_key_do_not_obtain_definition_birth(self):
        before=Path(self.doc['path']).read_bytes();basis=self.basis()
        request={**basis,'baseId':self.doc['baseId'],'operationId':uid(),'label':'weftext-query-test','plan':{'scope':{'descendants':True,'lifecycle':'live'},'types':[],'select':[{'name':'title','expr':'title'}],'order':[]}}
        code,value,_=self.server.call('/api/query/definition/draft',request);self.assertGreaterEqual(code,400,value)
        self.install_schema()
        proposal=self.call('/api/query/definition/draft',request)
        foreign=proposal['source'].replace(proposal['key'],uid())
        self.assertEqual(Path(self.doc['path']).read_bytes(),before)
        pending=self.call('/api/query/definition/pending',self.basis(foreign))
        self.assertIsNone(pending['operationId']);self.assertEqual(pending['queryState'],'unadmitted')
        self.call('/api/save',{'sessionId':self.doc['sessionId'],'baseId':self.doc['baseId'],'source':foreign,'operationId':uid()})
        owner=f"weftext-node://{self.doc['workspaceId']}/{self.doc['nodeId']}"
        definitions=self.call('/api/query/definitions',{'root':str(self.root),'owner':owner})
        self.assertEqual(definitions['definitions'],[])
        self.assertEqual(Path(self.doc['path']).read_text('utf-8'),foreign)
        unparsed=foreign.replace('"expr": "title"','"expr": 42')
        unresolved=self.call('/api/query/definition/pending',self.basis(unparsed))
        self.assertIsNone(unresolved['operationId']);self.assertEqual(unresolved['queryState'],'unparsed')

if __name__=='__main__': unittest.main(argv=[sys.argv[0],*sys.argv[2:]])
