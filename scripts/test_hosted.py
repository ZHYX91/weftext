"""Real hosted HTTP boundaries against owned disposable libraries; no desktop input."""
from _cli_serve import launch_serve
from _creation_result import created_document
import copy
import hashlib
import json
from pathlib import Path
import secrets
import tempfile
import unittest
from urllib.error import HTTPError
from urllib.request import Request, urlopen
import uuid
import sys
import time
from concurrent.futures import ThreadPoolExecutor

EXE = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else None
def uid(): return str(uuid.uuid4())
def digest(text): return hashlib.sha256(text.encode()).hexdigest()

class Server:
    def __init__(self, args, hosted=False, *, expected_default_data_dir=None, evidence_parent=None):
        self.owner = launch_serve(EXE, args, hosted=hosted, expected_default_data_dir=expected_default_data_dir,
                                  evidence_parent=evidence_parent)
        self.process = self.owner.process
        self.url, self.token = self.owner.ready['base'], self.owner.ready['token']
        self.hosted = hosted

    def close(self):
        return self.owner.stop()

    def call(self, route, data=None, token=None, headers=None):
        auth = {'Authorization': 'Bearer ' + (token or '')} if self.hosted else {'X-Weftext-Token': self.token}
        req = Request(self.url + route, data=None if data is None else json.dumps(data).encode(),
                      headers={'Content-Type':'application/json', **auth, **(headers or {})})
        try:
            response = urlopen(req, timeout=40)
        except HTTPError as e:
            response = e
        with response:
            raw = response.read()
            value = json.loads(raw) if 'json' in response.headers.get('Content-Type','') else raw
            return response.status, value, response.headers

class Hosted(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.evidence = Path(tempfile.mkdtemp(prefix='weftext-hosted-'))
        print('Hosted evidence:', cls.evidence, flush=True)

    def setUp(self):
        self.folder = self.evidence / self._testMethodName
        self.folder.mkdir()
        self.data = self.folder / 'state'
        self.root = self.folder / 'library'
        local = Server(['--data-dir',str(self.data)])
        try:
            self.assertEqual(local.call('/api/library/create', {'path':str(self.root),'operationId':uid()})[0],200)
            self.docs = {}
            for name, content in [('Shared','初始共同正文'),('Other','独立正文'),('Secret','HIDDEN_PAYLOAD_987')]:
                code, doc, _ = local.call('/api/note/create',{'root':str(self.root),'parent':str(self.root),'name':name,'operationId':uid()})
                self.assertEqual(code,200,doc)
                doc = created_document(doc)
                doc['source'] += content
                code, _, _ = local.call('/api/save',{'sessionId':doc['sessionId'],'baseId':doc['baseId'],'source':doc['source'],'operationId':uid()})
                self.assertEqual(code,200)
                self.docs[name] = doc
        finally:
            local.close()
        self.tokens = {id:secrets.token_urlsafe(32) for id in ['alice','bob','reader','admin']}
        full = {'read':True,'write':True,'history':True,'download':True}
        self.config = {'deployment':'test','bind':'127.0.0.1:0','root':str(self.root),
                       'adminTokenSha256':digest(self.tokens['admin']),
                       'files':{name:{'path':f'{name}/{name}.typ','title':name,'editable':True} for name in self.docs},
                       'users':{
                           'alice':{'enabled':True,'tokenSha256':digest(self.tokens['alice']),'grants':{name:full.copy() for name in self.docs}},
                           'bob':{'enabled':True,'tokenSha256':digest(self.tokens['bob']),'grants':{name:full.copy() for name in ['Shared','Other']}},
                           'reader':{'enabled':True,'tokenSha256':digest(self.tokens['reader']),'grants':{'Shared':{'read':True}}}}}
        self.config_path = self.folder/'server.json'
        self.start()

    def start(self):
        self.config_path.write_text(json.dumps(self.config),'utf-8')
        self.server = Server(['--data-dir',str(self.data),'--server-config',str(self.config_path)],hosted=True)

    def tearDown(self): self.server.close()
    def call(self, route, data=None, actor='alice', status=200, headers=None):
        code, result, returned = self.server.call('/api/hosted/'+route,data,self.tokens.get(actor,'x'*40),headers)
        self.assertEqual(code,status,str(result)[:1500])
        if status == 200: self.assertEqual(returned.get('Cache-Control'),'no-store')
        return result
    def open(self,name='Shared',actor='alice'): return self.call('open',{'resource':name},actor)
    def save(self,doc,source,actor='alice',status=200,op=None):
        req={'sessionId':doc['sessionId'],'baseId':doc['baseId'],'source':source,'operationId':op or uid()}
        return self.call('save',req,actor,status),req
    def policy(self, transform):
        old=self.call('admin/policy',actor='admin')
        users=copy.deepcopy(old['users']);transform(users)
        return self.call('admin/policy',{'expectedGeneration':old['generation'],'users':users},'admin')

    def test_repeated_close_releases_sessions_without_discarding_personal_drafts(self):
        original = self.open()
        retained = original['source'] + '\n个人恢复稿 😀\n'
        self.call('draft', {'sessionId': original['sessionId'], 'source': retained})
        for _ in range(40):
            opened = self.open()
            self.call('session/keepalive', {'sessionId': opened['sessionId']}, 'bob', 410)
            self.call('close', {'sessionId': opened['sessionId']}, 'bob', 410)
            self.call('session/keepalive', {'sessionId': opened['sessionId']})
            self.call('close', {'sessionId': opened['sessionId']})
        self.call('close', {'sessionId': original['sessionId']})
        expired = self.call('session/keepalive', {'sessionId': original['sessionId']}, status=410)
        self.assertEqual(expired['error']['code'], 'sessionExpired')
        reopened = self.open()
        self.assertEqual(reopened['recoveredDraft'], retained)
        self.assertEqual(reopened['source'], original['source'])

    def test_boundary_and_hidden_existence(self):
        self.call('me',actor='unknown',status=401)
        self.call('me',headers={'Origin':'https://foreign.invalid'},status=403)
        self.call('me',headers={'Host':'foreign.invalid'},status=403)
        self.call('admin/policy',actor='alice',status=401)
        files=self.call('documents',actor='bob')['documents']
        self.assertEqual({r['resource'] for r in files},{'Shared','Other'})
        no=self.call('open',{'resource':'NotThere'},'bob',404)
        hidden=self.call('open',{'resource':'Secret'},'bob',404)
        self.assertEqual(no,hidden)
        self.assertEqual(self.call('search',{'text':'HIDDEN_PAYLOAD'},'bob')['rows'],[])
        self.assertNotIn(str(self.root),json.dumps(files))
        for route in ['/api/open','/api/recovery','/api/library/open','/api/export/prepare','/api/knowledge/annotations','/index.html', '/api/sync/binding', '/api/sync/enable', '/api/sync/status', '/api/sync/publish', '/api/sync/receive', '/api/sync/export', '/api/sync/file', '/api/sync/cancel-prepared', '/api/sync/operation']:
            code,body,_=self.server.call(route,{} if route.startswith('/api/') else None,self.tokens['alice'])
            self.assertEqual(code,404,(route,body))

    def test_read_does_not_grant_write_history_download_or_other_sessions(self):
        a=self.open();r=self.open(actor='reader')
        self.save(r,r['source']+'unauthorized','reader',404)
        self.call('history',{'sessionId':r['sessionId']},'reader',404)
        self.call('download/prepare',{'resource':'Shared'},'reader',404)
        self.call('draft',{'sessionId':a['sessionId'],'source':'stolen'},'bob',410)
        self.call('preview',{'sessionId':a['sessionId'],'source':'stolen'},'bob',410)

    def test_concurrent_writers_keep_both_drafts_and_unrelated_document(self):
        a=self.open();b=self.open(actor='bob');other=self.open('Other','bob')
        self.call('draft',{'sessionId':a['sessionId'],'source':a['source']+'ALICE_DRAFT'})
        self.call('draft',{'sessionId':b['sessionId'],'source':b['source']+'BOB_DRAFT'},'bob')
        receipt,req=self.save(a,a['source']+'ALICE_SAVED')
        self.assertEqual(receipt,self.call('save',req))
        self.save(b,b['source']+'BOB_CONFLICT','bob',409)
        self.save(other,other['source']+'UNRELATED_OK','bob')
        current=(self.root/'Shared/Shared.typ').read_text('utf-8')
        self.assertIn('ALICE_SAVED',current);self.assertNotIn('BOB_CONFLICT',current)
        recovered=self.open(actor='bob')
        self.assertIn('BOB_CONFLICT',recovered['recoveredDraft'])
        self.assertTrue(recovered['recoveredDraftConflict'])
        self.assertNotIn('ALICE_DRAFT',json.dumps(recovered))
        merged=recovered['source']+'\nBOB_MERGED'
        self.call('draft/resolve',{'sessionId':recovered['sessionId'],'source':merged},'bob')
        self.save(recovered,merged,'bob')
        self.assertIn('ALICE_SAVED',(self.root/'Shared/Shared.typ').read_text('utf-8'))

    def test_retries_status_actor_namespace_and_restart(self):
        a=self.open();b=self.open(actor='bob');op=uid()
        saved,req=self.save(a,a['source']+'saved',op=op)
        other_op=self.call('operation',{'operationId':op},'bob')
        self.assertEqual(other_op['status'],'notRegistered')
        self.save(b,b['source']+'conflict','bob',409,op)
        self.call('save',{**req,'source':req['source']+'changed'},status=409)
        self.server.close();self.start()
        self.assertEqual(self.call('operation',{'operationId':op}),saved)
        self.assertEqual(self.call('operation',{'operationId':op},'bob')['status'],'notCommitted')
        reopened=self.open(actor='bob');self.assertIn('conflict',reopened['recoveredDraft'])

    def test_history_authorized_without_other_users_drafts(self):
        a=self.open();b=self.open(actor='bob')
        self.call('draft',{'sessionId':b['sessionId'],'source':b['source']+'BOB_PRIVATE'},'bob')
        self.save(a,a['source']+'saved history')
        history=self.call('history',{'sessionId':a['sessionId']})
        self.assertNotIn('BOB_PRIVATE',json.dumps(history));self.assertNotIn(str(self.root),json.dumps(history))
        version=next(v for v in history['versions'] if v['status']=='available')
        self.assertIsInstance(self.call('history/read',{'sessionId':a['sessionId'],'versionId':version['versionId']})['source'],str)
        self.policy(lambda u:u['alice']['grants']['Shared'].update(history=False))
        a=self.open();self.call('history/read',{'sessionId':a['sessionId'],'versionId':version['versionId']},status=404)

    def test_frozen_download_same_bytes_and_revocation_every_chunk(self):
        a=self.open();body=a['source']+'X'*150000
        self.save(a,body)
        ticket=self.call('download/prepare',{'resource':'Shared'})
        self.call('download/chunk',{'ticket':ticket['ticket'],'offset':0},'bob',404)
        first=self.call('download/chunk',{'ticket':ticket['ticket'],'offset':0})
        b=self.open(actor='bob');self.save(b,b['source']+'later','bob')
        chunks=[first]+[self.call('download/chunk',{'ticket':ticket['ticket'],'offset':i}) for i in range(65536,ticket['length'],65536)]
        self.assertEqual(hashlib.sha256(b''.join(chunks)).hexdigest(),ticket['sha256'])
        self.policy(lambda u:u['alice']['grants']['Shared'].update(download=False))
        self.call('download/chunk',{'ticket':ticket['ticket'],'offset':65536},status=404)
        self.policy(lambda u:u['alice']['grants']['Shared'].update(download=True))
        self.call('download/chunk',{'ticket':ticket['ticket'],'offset':0},status=404)

    def test_revoke_survives_restart_and_unrelated_actor_can_save(self):
        a=self.open();b=self.open(actor='bob')
        before=self.call('me',actor='bob')['generation']
        self.policy(lambda u:u['alice'].update(enabled=False))
        self.call('me',status=401)
        self.save(b,b['source']+'continued','bob')
        self.assertEqual(self.call('me',actor='bob')['generation'],before)
        self.server.close();self.start()
        self.call('me',status=401)
        self.assertIn('continued',self.open(actor='bob')['source'])

    def test_frozen_compiler_never_fills_missing_or_hidden_inputs(self):
        a=self.open(actor='bob')
        allowed=self.call('preview',{'sessionId':a['sessionId'],'source':'= 中文预览\n正文😀'},'bob')
        self.assertTrue(allowed['ok'],allowed)
        self.assertIn('正文',allowed['html'])
        for source in ['#read("../Secret/Secret.typ")','#include "../Secret/Secret.typ"',f'#read("/{self.root.as_posix()}/Secret/Secret.typ")']:
            rendered=self.call('preview',{'sessionId':a['sessionId'],'source':source},'bob')
            self.assertFalse(rendered['ok']);self.assertNotIn('HIDDEN_PAYLOAD',json.dumps(rendered))
            self.assertNotIn(str(self.root),json.dumps(rendered))

    def test_explicit_dependency_authorization_before_read(self):
        # New configured inputs become active only on an operator restart.
        self.server.close();self.config['files']['Shared']['inputs']=['Secret'];self.start()
        a=self.open(actor='bob')
        self.call('preview',{'sessionId':a['sessionId'],'source':'ordinary'},'bob',404)
        a=self.open()
        rendered=self.call('preview',{'sessionId':a['sessionId'],'source':'#read("../Secret/Secret.typ")'})
        self.assertTrue(rendered['ok']);self.assertIn('HIDDEN',rendered['html'])

    def test_unknown_request_is_fenced_before_a_new_proposal(self):
        a=self.open();op=uid()
        self.assertEqual(self.call('operation',{'operationId':op})['status'],'notRegistered')
        self.assertEqual(self.call('operation/settle',{'resource':'Shared','operationId':op})['status'],'cancelled')
        self.save(a,a['source']+'late original',status=409,op=op)
        self.assertNotIn('late original',(self.root/'Shared/Shared.typ').read_text('utf-8'))
        result,req=self.save(a,a['source']+'new explicit proposal')
        self.assertEqual(self.call('operation/settle',{'resource':'Shared','operationId':req['operationId']}),result)

    def test_inflight_preview_rechecks_revocation_before_delivery(self):
        a=self.open()
        with ThreadPoolExecutor(max_workers=1) as pool:
            pending=pool.submit(self.server.call,'/api/hosted/preview',{'sessionId':a['sessionId'],'source':'#for i in range(20000) [delayed preview paragraph #parbreak()]'},self.tokens['alice'])
            time.sleep(0.1)
            self.policy(lambda u:u['alice'].update(enabled=False))
            code,body,_=pending.result(timeout=30)
        self.assertIn(code,[401,410],str(body)[:500])
        self.assertNotIn('html',body)

if __name__ == '__main__':
    unittest.main(argv=[sys.argv[0]],verbosity=2)
