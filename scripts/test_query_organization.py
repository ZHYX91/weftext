"""Owned real-program acceptance of ordinary root presets and native Query organization."""
from _creation_result import created_document
import json
from pathlib import Path
import sys
import tempfile
import unittest
import uuid
import test_hosted
import test_hosted_query

def uid(): return str(uuid.uuid4())
def plan(): return {'scope':{'descendants':True,'lifecycle':'live'},'types':[],'select':[],'order':[]}

class QueryOrganization(unittest.TestCase):
    def setUp(self):
        self.folder=Path(tempfile.mkdtemp(prefix='weftext-org-')); print('Query organization evidence:',self.folder,flush=True)
        self.data=self.folder/'state'; self.root=self.folder/'Library'; self.server=test_hosted.Server(['--no-browser','--data-dir',str(self.data)]); self.addCleanup(lambda:self.server.close()); self.serial=0
        self.options={'initialRoot':{'title':'我的首页😀','queries':['tasks','events']}}
        self.operation=uid(); self.receipt=self.call('/api/library/create',{'path':str(self.root),'operationId':self.operation,'options':self.options})
        self.doc=self.call('/api/open',{'root':str(self.root),'path':self.receipt['rootNotePath']})
    def call(self,route,body=None,status=200):
        code,value,_=self.server.call(route,body); self.assertEqual(code,status,(route,value)); return value
    def basis(self,source=None):
        self.serial+=1; return {'sessionId':self.doc['sessionId'],'source':self.doc['source'] if source is None else source,'serial':self.serial}
    def discover(self,source=None):
        basis=self.basis(source); return basis,self.call('/api/query/embedded/discover',basis)['occurrences']
    def test_root_presets_have_real_schema_original_keys_and_complete_empty_typed_results(self):
        self.assertEqual(Path(self.receipt['rootNotePath']),self.root/'Library.typ')
        self.assertEqual(self.doc['kind'],'managed'); self.assertEqual(self.call('/api/library/create',{'path':str(self.root),'operationId':self.operation,'options':self.options}),self.receipt)
        schema=self.call('/api/data/schema',{'root':str(self.root)}); self.assertFalse(schema['builtinDefault'])
        basis,queries=self.discover(); self.assertEqual(len(queries),2)
        for query in queries:
            self.assertEqual(query['binding']['revision'],schema['revision']); self.assertTrue(query['runnable'])
            run=self.call('/api/query/embedded/run',{**basis,'occurrenceId':query['id'],'parameters':{}})['result']
            self.assertEqual(run['rows'],[]); self.assertEqual(run['coverage'],'complete'); self.assertEqual(run['snapshotMode'],'admittedFrozen'); self.assertEqual(run['freshness'],'stale')
        self.assertEqual(Path(self.doc['path']).read_text('utf-8'),self.doc['source'])
        # The ordinary root and its initialized style must also work in the
        # knowledge/citation consumer; actual derived entries are not files.
        self.call('/api/knowledge/read',{'context':{**self.basis(),'baseId':self.doc['baseId'],'dirty':False}})
    def test_reorder_remove_and_stale_proposals_preserve_unselected_source_without_saving(self):
        original=Path(self.doc['path']).read_bytes(); basis,queries=self.discover(); first,last=queries
        moved=self.call('/api/query/embedded/edit',{**basis,'occurrenceId':last['id'],'action':'up'})
        self.assertLess(moved['source'].index(last['label']),moved['source'].index(first['label']))
        self.assertIn(first['range']['raw'],moved['source']); self.assertIn(last['range']['raw'],moved['source'])
        newer,found=self.discover(moved['source'])
        self.call('/api/query/embedded/edit',{**basis,'occurrenceId':last['id'],'action':'up'},409)
        removed=self.call('/api/query/embedded/edit',{**newer,'occurrenceId':found[0]['id'],'action':'remove'})
        self.assertNotIn(found[0]['range']['raw'],removed['source']); self.assertIn(found[1]['range']['raw'],removed['source']); self.assertEqual(Path(self.doc['path']).read_bytes(),original)
    def test_shared_call_pinned_edit_and_copy_keep_real_owner_and_do_not_rewrite_it(self):
        owner_source=self.doc['source']; owner=f"weftext-node://{self.doc['workspaceId']}/{self.doc['nodeId']}"
        definitions=self.call('/api/query/definitions',{'root':str(self.root),'owner':owner}); target=definitions['definitions'][0]
        self.doc=created_document(self.call('/api/note/create',{'root':str(self.root),'parent':str(self.root),'name':'Caller','operationId':uid()})); self.serial=0
        call=plan(); call['advanced']={'call':{'owner':owner,'selection':{'mode':'current','key':target['key']},'arguments':{}}}
        operation=uid(); basis=self.basis(); proposal=self.call('/api/query/definition/draft',{**basis,'baseId':self.doc['baseId'],'label':'weftext-query-共享调用','plan':call,'operationId':operation})
        self.call('/api/save',{'sessionId':self.doc['sessionId'],'baseId':self.doc['baseId'],'source':proposal['source'],'operationId':operation})
        self.doc=self.call('/api/open',{'root':str(self.root),'path':self.doc['path']}); self.serial=0
        basis,queries=self.discover(); self.assertEqual(queries[0]['kind'],'call'); self.assertTrue(queries[0]['builderSupported'])
        changed=queries[0]['plan']; changed['advanced']['call']['selection']={'mode':'pinned','key':target['key'],'revision':definitions['ownerRevision']}
        edited=self.call('/api/query/embedded/edit',{**basis,'occurrenceId':queries[0]['id'],'plan':changed}); self.assertEqual(Path(self.root/'Library.typ').read_text('utf-8'),owner_source)
        self.call('/api/save',{'sessionId':self.doc['sessionId'],'baseId':self.doc['baseId'],'source':edited['source'],'operationId':uid()})
        intent={'kind':'copyNode','source':str(self.root/'Caller'),'parent':str(self.root),'name':'CopiedCaller'}
        refused=self.call('/api/operations/prepare',{'root':str(self.root),'operationId':uid(),'intent':intent},409); self.assertEqual(refused['error']['code'],'copyLosses')
        preparation=self.call('/api/operations/prepare',{'root':str(self.root),'operationId':uid(),'intent':{**intent,'allowLosses':True}})
        self.assertTrue(any(loss['code']=='pinnedHistoricalDefinition' for loss in preparation['losses']))
        self.call('/api/operations/commit',{'operationId':preparation['operationId'],'planId':preparation['planId']})
        self.doc=self.call('/api/open',{'root':str(self.root),'path':str(self.root/'CopiedCaller/CopiedCaller.typ')}); self.serial=0
        basis,copied=self.discover(); self.assertNotEqual(copied[0]['key'],queries[0]['key']); self.assertEqual(copied[0]['plan']['advanced']['call']['owner'],owner)
        result=self.call('/api/query/embedded/run',{**basis,'occurrenceId':copied[0]['id'],'parameters':{}})['result']; self.assertEqual(result['rows'],[])
    def test_root_user_changes_survive_restart_and_original_creation_retry(self):
        owner=self.doc['nodeId']; source=f'#metadata((id: "{owner}", types: ())) <weftext>\n\n用户清空预设后自己的正文😀\n'
        self.call('/api/save',{'sessionId':self.doc['sessionId'],'baseId':self.doc['baseId'],'source':source,'operationId':uid()})
        self.server.close(); self.server=test_hosted.Server(['--no-browser','--data-dir',str(self.data)])
        receipt=self.call('/api/library/create',{'path':str(self.root),'operationId':self.operation,'options':self.options}); self.assertEqual(receipt,self.receipt)
        self.doc=self.call('/api/open',{'root':str(self.root),'path':receipt['rootNotePath']}); self.assertEqual(self.doc['source'],source); self.serial=0
        self.assertEqual(self.discover()[1],[])
    def test_root_preset_sync_clone_keeps_schema_keys_and_exact_author_source(self):
        envelope=(self.root/'.weftext-workspace.json').read_bytes()
        binding=self.call('/api/sync/enable',{'root':str(self.root),'operationId':uid()}); identity=binding['identity']; objects=[{'kind':'node','id':self.doc['nodeId']},{'kind':'schema'}]; exchange=self.folder/'exchange'
        self.call('/api/sync/export',{'root':str(self.root),'directory':str(exchange),'objects':objects})
        replica=test_hosted.Server(['--no-browser','--data-dir',str(self.folder/'replica-state')]); self.addCleanup(replica.close); destination=self.folder/'Replica'
        def call(route,body):
            code,value,_=replica.call(route,body); self.assertEqual(code,200,(route,value)); return value
        call('/api/sync/receive',{'identity':identity,'directory':str(exchange)})
        prepared=call('/api/operations/prepare',{'root':str(destination),'operationId':uid(),'intent':{'kind':'cloneSync','identity':identity,'objects':objects}})
        call('/api/operations/commit',{'operationId':prepared['operationId'],'planId':prepared['planId']})
        owner=f"weftext-node://{self.doc['workspaceId']}/{self.doc['nodeId']}"; opened=call('/api/data/open',{'root':str(destination),'entityKey':owner,'lifecycle':'live'})
        self.assertEqual(opened['source'],self.doc['source'])
        basis={'sessionId':opened['sessionId'],'source':opened['source'],'serial':0}; found=call('/api/query/embedded/discover',basis)['occurrences']; self.assertEqual(len(found),2)
        for query in found:
            result=call('/api/query/embedded/run',{**basis,'occurrenceId':query['id'],'parameters':{}})['result']; self.assertEqual(result['rows'],[]); self.assertEqual(result['coverage'],'complete')
        changed=opened['source']+'\n\n副本自己的普通正文编辑😀\n'
        call('/api/save',{'sessionId':opened['sessionId'],'baseId':opened['baseId'],'source':changed,'operationId':uid()})
        call('/api/sync/export',{'root':str(destination),'directory':str(exchange),'objects':objects})
        self.call('/api/sync/receive',{'identity':identity,'directory':str(exchange)})
        apply=self.call('/api/operations/prepare',{'root':str(self.root),'operationId':uid(),'intent':{'kind':'applySync','identity':identity,'objects':objects}})
        self.call('/api/operations/commit',{'operationId':apply['operationId'],'planId':apply['planId']})
        self.assertEqual(Path(self.doc['path']).read_text('utf-8'),changed); self.assertEqual((self.root/'.weftext-workspace.json').read_bytes(),envelope)

class HostedOrganization(unittest.TestCase):
    def setUp(self):
        self.fixture=test_hosted_query.HostedQuery('runTest'); self.fixture.evidence=Path(tempfile.mkdtemp(prefix='weftext-org-hosted-')); print('Hosted organization evidence:',self.fixture.evidence,flush=True); self.fixture.setUp(); self.addCleanup(self.fixture.tearDown)
    def test_form_catalog_exposes_only_independently_granted_shapes_and_arrangement_cannot_gain_write(self):
        h=self.fixture
        h.policy(lambda users: users['reader']['grants']['Shared'].update(fields={'tasks/status':{'read':True,'write':False}}))
        code,catalog,_=h.server.call('/api/hosted/query/catalog',token=h.tokens['reader'])
        self.assertEqual(code,200,catalog)
        allowed={field for resource in catalog['resources'] for field in resource['fields']}
        self.assertTrue({field['id'] for field in (catalog.get('formSchema') or {}).get('fields',[])} <= allowed)
        self.assertEqual({field['id'] for field in catalog['formSchema']['fields']},{'tasks/status'})
        self.assertEqual((catalog.get('formSchema') or {}).get('types',[]),[])
        code,doc,_=h.server.call('/api/hosted/open',{'resource':'Shared'},token=h.tokens['reader']); self.assertEqual(code,200,doc)
        code,error,_=h.server.call('/api/hosted/query/embedded/edit',{'sessionId':doc['sessionId'],'source':doc['source'],'serial':0,'occurrenceId':'forged','action':'remove'},token=h.tokens['reader'])
        self.assertEqual(code,404,error)
        h.policy(lambda users: users['reader'].update(grants={}))
        code,catalog,_=h.server.call('/api/hosted/query/catalog',token=h.tokens['reader']); self.assertEqual(code,200,catalog); self.assertEqual(catalog['resources'],[]); self.assertIsNone(catalog.get('formSchema'))
    def test_hosted_arrangement_is_draft_only_and_group_edit_keeps_original_authorization(self):
        h=self.fixture; h.prepare_query(); h.grant_query('alice',resources=('Shared','Other'),products=('title','types'),definitions=True,history=True)
        basic=plan(); basic['select']=[{'name':'title','expr':'title'}]
        h.add_definition(basic,'Shared','weftext-query-one'); h.add_definition(basic,'Shared','weftext-query-two')
        doc=h.open('Shared'); original=(h.root/'Shared/Shared.typ').read_bytes(); basis={'sessionId':doc['sessionId'],'source':doc['source'],'serial':1}
        found=h.call('query/embedded/discover',basis)['occurrences']; self.assertEqual(len(found),2)
        moved=h.call('query/embedded/edit',{**basis,'occurrenceId':found[1]['id'],'action':'up'}); self.assertLess(moved['source'].index(found[1]['label']),moved['source'].index(found[0]['label']))
        next_basis={**basis,'source':moved['source'],'serial':2}; newer=h.call('query/embedded/discover',next_basis)['occurrences']; grouped=newer[0]['plan']; grouped['group']={'keys':[],'aggregates':[{'name':'count','op':'count'}]}
        edited=h.call('query/embedded/edit',{**next_basis,'occurrenceId':newer[0]['id'],'plan':grouped}); self.assertIn(newer[1]['range']['raw'],edited['source']); self.assertEqual((h.root/'Shared/Shared.typ').read_bytes(),original)
        h.policy(lambda users: users['alice']['grants']['Shared'].update(write=False))
        h.call('query/embedded/edit',{**next_basis,'occurrenceId':newer[0]['id'],'action':'remove'},status=410)
        readonly=h.open('Shared'); h.call('query/embedded/edit',{'sessionId':readonly['sessionId'],'source':readonly['source'],'serial':0,'occurrenceId':found[0]['id'],'action':'remove'},status=404)

if __name__=='__main__':
    test_hosted.EXE=Path(sys.argv.pop(1)).resolve(); unittest.main(verbosity=2)
