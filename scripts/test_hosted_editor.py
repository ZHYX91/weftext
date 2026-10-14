"""Hosted native draft consumers using the actual executable and owned roots."""
import json
import sys
import time
import unittest
from concurrent.futures import ThreadPoolExecutor
from test_hosted import Hosted

class HostedEditor(Hosted):
    def test_native_edit_map_exact_basis_permissions_and_original_save(self):
        a=self.call('open',{'resource':'Shared'})
        source=a['source']+'\n= 标题\n\n中文😀正文\n'
        basis={'sessionId':a['sessionId'],'source':source,'serial':1}
        mapped=self.call('editor/map',basis)
        region=next(r for r in mapped['literalRegions'] if '中文' in r['text'])
        request={**basis,'sourceHash':mapped['sourceHash'],'intent':{'kind':'replaceLiteral','range':region['range'],'text':'甲中文😀修改'}}
        edited=self.call('editor/edit',request)
        self.assertIn('修改',edited['source']);self.assertEqual((self.root/'Shared/Shared.typ').read_text(encoding='utf-8'),self.docs['Shared']['source'])
        self.call('editor/edit',request,'bob',410)
        reader=self.call('open',{'resource':'Shared'},'reader')
        readbasis={**basis,'sessionId':reader['sessionId']}
        readmap=self.call('editor/map',readbasis,'reader')
        self.assertFalse(any(r['range']['writable'] for r in readmap['literalRegions']))
        self.call('editor/edit',{**request,'sessionId':reader['sessionId']},'reader',404)
        self.call('editor/map',{**basis,'serial':2,'source':source+'新输入'})
        self.call('editor/edit',request,status=409)
        self.call('editor/map',{**basis,'serial':2},status=409)
        # Same bytes with a different serial cannot revive the old map.
        self.call('editor/map',{**basis,'serial':3})
        self.call('editor/edit',request,status=409)
        self.call('save',{'sessionId':a['sessionId'],'baseId':a['baseId'],'source':edited['source'],'operationId':__import__('uuid').uuid4().__str__()})
        self.assertEqual((self.root/'Shared/Shared.typ').read_text(encoding='utf-8'),edited['source'])

    def test_projection_has_only_virtual_main_origins_and_frozen_authorized_inputs(self):
        a=self.call('open',{'resource':'Shared'})
        result=self.call('editor/render',{'sessionId':a['sessionId'],'source':'= 中文\n\n正文😀','serial':1})
        self.assertTrue(result['ok'],result)
        def origins(tree):
            if tree.get('origin'): yield tree['origin']
            for child in tree.get('children',[]):yield from origins(child)
        self.assertTrue(list(origins(result['tree'])))
        self.assertTrue(all(o['path']=='resource:Shared' and o['isMain'] for o in origins(result['tree'])))
        self.assertNotIn(str(self.root),json.dumps(result));self.assertNotIn('dependencies',result)
        hidden=self.call('editor/render',{'sessionId':a['sessionId'],'source':'#read("../Secret/Secret.typ")','serial':2})
        self.assertFalse(hidden['ok']);self.assertNotIn('HIDDEN_PAYLOAD',json.dumps(hidden))
        reader=self.call('open',{'resource':'Shared'},'reader')
        result=self.call('editor/render',{'sessionId':reader['sessionId'],'source':'只读正文','serial':1},'reader')
        def writable(tree):
            yield tree.get('writable',False)
            for child in tree.get('children',[]):yield from writable(child)
        self.assertFalse(any(writable(result['tree'])))
        self.server.close();self.config['files']['Shared']['inputs']=['Secret'];self.start()
        a=self.call('open',{'resource':'Shared'})
        result=self.call('editor/render',{'sessionId':a['sessionId'],'source':'#include "../Secret/Secret.typ"','serial':1})
        self.assertTrue(result['ok'])
        def text(tree): return (tree.get('text') or '')+''.join(text(child) for child in tree.get('children',[]))
        self.assertIn('HIDDEN_PAYLOAD',text(result['tree']));self.assertFalse(list(origins(result['tree'])))
        dependency=self.call('open',{'resource':'Secret'})
        with ThreadPoolExecutor() as pool:
            pending=pool.submit(self.server.call,'/api/hosted/editor/render',{'sessionId':a['sessionId'],'source':'#include "../Secret/Secret.typ"\n#for i in range(20000) [delayed paragraph #parbreak()]','serial':2},self.tokens['alice'])
            time.sleep(.2)
            self.save(dependency,dependency['source']+'\n新的依赖正文')
            code,result,_=pending.result(timeout=40);self.assertEqual(code,409,result);self.assertEqual(result['error']['code'],'staleEnvironment')
        b=self.call('open',{'resource':'Shared'},'bob')
        self.call('editor/render',{'sessionId':b['sessionId'],'source':'普通文字','serial':1},'bob',404)

    def test_late_projection_cannot_revive_old_serial_or_revoked_session(self):
        a=self.call('open',{'resource':'Shared'})
        slow={'sessionId':a['sessionId'],'source':'#for i in range(20000) [delayed paragraph #parbreak()]','serial':1}
        with ThreadPoolExecutor() as pool:
            pending=pool.submit(self.server.call,'/api/hosted/editor/render',slow,self.tokens['alice'])
            time.sleep(.2)
            self.call('editor/map',{**slow,'source':'更新后的草稿','serial':2})
            code,result,_=pending.result(timeout=40);self.assertEqual(code,409,result)
        with ThreadPoolExecutor() as pool:
            pending=pool.submit(self.server.call,'/api/hosted/editor/render',{**slow,'serial':3},self.tokens['alice'])
            time.sleep(.2)
            policy=self.call('admin/policy',actor='admin');policy['users']['alice']['grants']['Shared']['write']=False
            self.call('admin/policy',{'expectedGeneration':policy['generation'],'users':policy['users']},actor='admin')
            code,result,_=pending.result(timeout=40);self.assertEqual(code,410,result)

if __name__=='__main__': unittest.main(argv=[sys.argv[0]],defaultTest='HostedEditor',verbosity=2)
