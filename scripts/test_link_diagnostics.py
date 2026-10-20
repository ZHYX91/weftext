"""Stage-three read-only diagnostics, exact fragment proposals and current-cut refusal."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
import uuid
from test_hosted import Server

def uid(): return str(uuid.uuid4())

class LinkDiagnostics(unittest.TestCase):
    def setUp(self):
        self.folder=Path(tempfile.mkdtemp(prefix='weftext-links-'))
        self.root=self.folder/'library'
        self.server=Server(['--no-browser','--data-dir',str(self.folder/'state')])
        self.call('/api/library/create',{'path':str(self.root),'operationId':uid()})
        self.target=self.note('目标','= 路线图 <route>\n\n目标正文')
        self.current=self.note('来源','路线图 route\n\n#link("'+self.ref(self.target)+'#lost")[片段失效]\n#link("weftext-node://'+self.target['workspaceId']+'/'+uid()+'")[缺失]\n')
        self.serial=0
        print('Link diagnostics evidence:',self.folder,flush=True)
    def tearDown(self): self.server.close()
    def call(self,route,body=None,status=200):
        code,value,_=self.server.call(route,body);self.assertEqual(code,status,value);return value
    def note(self,name,body):
        doc=self.call('/api/note/create',{'root':str(self.root),'parent':str(self.root),'name':name,'operationId':uid()})
        source=doc['source']+'\n\n'+body
        self.call('/api/save',{'sessionId':doc['sessionId'],'baseId':doc['baseId'],'source':source,'operationId':uid()})
        return self.call('/api/open',{'root':str(self.root),'path':doc['path']})
    def ref(self,doc): return f"weftext-node://{doc['workspaceId']}/{doc['nodeId']}"
    def read(self):
        self.serial+=1
        context={**{k:self.current[k] for k in ('sessionId','baseId','source')},'dirty':False,'serial':self.serial}
        render=self.call('/api/render',{'sessionId':context['sessionId'],'source':context['source'],'serial':context['serial']})
        self.assertTrue(render['ok'],render['diagnostics'])
        return context,self.call('/api/knowledge/read',{'context':context})
    def test_issues_keep_fragment_connection_and_scan_does_not_write(self):
        before={p:p.read_bytes() for p in self.root.rglob('*.typ')}
        context,profile=self.read();d=profile['diagnostics']
        self.assertEqual({i['resolution']['status'] for i in d['issues']},{'fragmentMissing','missing'})
        self.assertEqual(next(n for n in d['nodes'] if n['node']['nodeId']==self.target['nodeId'])['incoming'],1)
        self.assertTrue(all(i['sourceHash'] for i in d['issues']))
        self.assertEqual(before,{p:p.read_bytes() for p in before})
    def test_heading_fragment_conversion_is_exact_and_old_dictionary_refuses(self):
        context,profile=self.read()
        hits=[(f,h,c) for f in profile['mentions']['fragments'] for h in f['hits'] for c in h['candidates'] if c['provenance']=='heading' and c['fragment']=='route']
        self.assertTrue(hits,profile['mentions']);fragment,hit,candidate=hits[0]
        intent=dict(kind='mentionConvert',generation=profile['mentions']['generation'],fragmentId=fragment['id'],startScalar=hit['startScalar'],endScalar=hit['endScalar'],target=candidate['target'],fragment='route')
        request=dict(context=context,operationId=uid(),intent=intent)
        proposal=self.call('/api/knowledge/prepare',request)
        self.assertTrue(proposal['draftOnly']);self.assertIn(self.ref(self.target)+'#route',proposal['source'])
        self.assertEqual(Path(self.current['path']).read_text('utf-8'),self.current['source'])
        self.call('/api/save',{'sessionId':self.target['sessionId'],'baseId':self.target['baseId'],'source':self.target['source'].replace('<route>','<changed>'),'operationId':uid()})
        self.call('/api/knowledge/prepare',{**request,'operationId':uid()},409)
        _,changed=self.read()
        self.assertFalse(any(c['fragment']=='route' for f in changed['mentions']['fragments'] for h in f['hits'] for c in h['candidates']))
    def test_duplicate_label_never_becomes_a_fragment_candidate(self):
        self.call('/api/save',{'sessionId':self.target['sessionId'],'baseId':self.target['baseId'],'source':self.target['source']+'\n= 第二处 <route>','operationId':uid()})
        _,profile=self.read()
        self.assertFalse(any(c['fragment']=='route' for f in profile['mentions']['fragments'] for h in f['hits'] for c in h['candidates']))

if __name__=='__main__': unittest.main(argv=[sys.argv[0],*sys.argv[2:]],verbosity=2)
