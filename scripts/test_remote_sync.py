"""Actual two-replica + hosted-relay acceptance. All roots/state are owned
disposable fixtures, retained for diagnosis; no desktop or external service.
"""
from _cli_serve import serve_command
from _creation_result import created_document
import base64
import copy
import hashlib
import json
from pathlib import Path
import secrets
import sys
import tempfile
import time
import unittest
import subprocess
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from test_hosted import EXE, Server, uid

def sha(data): return hashlib.sha256(data).hexdigest()

class RemoteSync(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.evidence = Path(tempfile.mkdtemp(prefix='weftext-remote-sync-'))
        print('Retained remote sync evidence:', cls.evidence, flush=True)

    def setUp(self):
        self.folder = self.evidence / self._testMethodName; self.folder.mkdir()
        self.a = Server(['--data-dir',str(self.folder/'state-a')]); self.addCleanup(lambda:self.a.close())
        self.b = Server(['--data-dir',str(self.folder/'state-b')]); self.addCleanup(lambda:self.b.close())
        self.ra, self.rb = self.folder/'a', self.folder/'b'
        self.local(self.a,'/api/library/create',path=str(self.ra),operationId=uid())
        self.note = created_document(self.local(self.a,'/api/note/create',root=str(self.ra),parent=str(self.ra),name='Note',operationId=uid()))
        self.other = created_document(self.local(self.a,'/api/note/create',root=str(self.ra),parent=str(self.ra),name='Other',operationId=uid()))
        self.key = {'kind':'node','id':self.note['nodeId']}; self.other_key = {'kind':'node','id':self.other['nodeId']}
        self.identity = self.local(self.a,'/api/sync/enable',root=str(self.ra),operationId=uid())['identity']
        self.tokens = {name:secrets.token_hex(32) for name in ['admin','owner','owner2','author']}
        self.relay_root = self.folder/'relay-files'; self.relay_root.mkdir(); (self.relay_root/'shared.typ').write_text('服务端独立正文',encoding='utf-8')
        self.config = {'deployment':'remote-test','bind':'127.0.0.1:0','root':str(self.relay_root),'adminTokenSha256':sha(self.tokens['admin'].encode()),
            'syncIdentity':self.identity,'files':{'shared':{'path':'shared.typ','title':'共同文档','editable':True}},
            'users':{name:{'enabled':True,'tokenSha256':sha(self.tokens[name].encode()),'syncOwner':name.startswith('owner'),
                     'grants':{'shared':{'read':True,'write':True,'history':True,'download':True}}} for name in ['owner','owner2','author']}}
        self.config_path=self.folder/'server.json'; self.config_path.write_text(json.dumps(self.config),encoding='utf-8')
        self.server = Server(['--data-dir',str(self.folder/'relay-state'),'--server-config',str(self.config_path)],hosted=True)
        self.addCleanup(lambda:self.server.close())

    def local(self, server, route, expected=200, **data):
        code, value, _ = server.call(route, data); self.assertEqual(code,expected,(route,value)); return value

    def remote(self, route, data=None, actor='owner', expected=200):
        code,value,headers=self.server.call('/api/hosted/'+route,data,token=self.tokens[actor]); self.assertEqual(code,expected,(route,value)); self.assertEqual(headers['Cache-Control'],'no-store'); return value

    def connection(self, actor='owner'):
        return {'endpoint':self.server.url,'token':self.tokens[actor],'identity':self.identity}

    def transfer(self, local, direction, root=None, objects=None):
        req={'requestId':uid(),'connection':self.connection(),'direction':direction,'root':str(root) if root else None,'objects':objects}
        original=self.local(local,'/api/sync/remote/start',**req)
        again=self.local(local,'/api/sync/remote/start',**req)
        self.assertEqual(original['requestId'],again['requestId'])
        deadline=time.monotonic()+100
        while time.monotonic()<deadline:
            state=self.local(local,'/api/sync/remote/job',requestId=req['requestId'])
            if state['status']!='running':
                self.assertEqual(state['status'],'complete',state); return state
            time.sleep(.03)
        self.fail('Transfer did not finish')

    def seed_clone(self):
        self.transfer(self.a,'push',self.ra,[self.key])
        self.transfer(self.b,'pull',objects=[self.key])
        self.plan(self.b,self.rb,{'kind':'cloneSync','identity':self.identity,'objects':[self.key]})
        self.assertFalse((self.rb/'Other').exists())

    def plan(self, server, root, intent):
        preview=self.local(server,'/api/operations/prepare',operationId=uid(),root=str(root),intent=intent)
        return self.local(server,'/api/operations/commit',operationId=preview['operationId'],planId=preview['planId'])

    def save(self, server, root, name, suffix):
        doc=self.local(server,'/api/open',root=str(root),path=str(root/name/(name+'.typ')))
        source=doc['source']+'\n'+suffix
        self.local(server,'/api/save',sessionId=doc['sessionId'],baseId=doc['baseId'],source=source,operationId=uid()); return source

    def policy(self, change):
        policy=self.remote('admin/policy',actor='admin'); change(policy['users'])
        return self.remote('admin/policy',{'expectedGeneration':policy['generation'],'users':policy['users']},actor='admin')

    def test_three_programs_partial_offline_conflicts_and_unrelated_progress(self):
        self.seed_clone()
        self.assertEqual((self.rb/'Note/Note.typ').read_bytes(),(self.ra/'Note/Note.typ').read_bytes())
        a=self.save(self.a,self.ra,'Note','甲离线中文😀'); b=self.save(self.b,self.rb,'Note','乙离线中文')
        other=self.save(self.a,self.ra,'Other','无关对象继续')
        self.transfer(self.a,'push',self.ra,[self.key,self.other_key]); self.transfer(self.b,'push',self.rb,[self.key])
        self.transfer(self.b,'pull',objects=[self.key,self.other_key])
        status=self.local(self.b,'/api/sync/status',identity=self.identity)
        view=next(o['view'] for o in status['objects'] if o['object']==self.key)
        self.assertEqual(len(view['versions']),2)
        texts=[self.local(self.b,'/api/sync/file',identity=self.identity,object=self.key,revision=v['revision'],path='Note.typ')['text'] for v in view['versions']]
        self.assertEqual(set(texts),{a,b})
        self.plan(self.b,self.rb,{'kind':'applySync','identity':self.identity,'objects':[self.other_key]})
        self.assertEqual((self.rb/'Other/Other.typ').read_text(encoding='utf-8'),other)
        self.assertEqual((self.rb/'Note/Note.typ').read_text(encoding='utf-8'),b)
        self.assertEqual((self.relay_root/'shared.typ').read_text(encoding='utf-8'),'服务端独立正文')
        branch=view['versions'][texts.index(a)]
        self.plan(self.b,self.rb,{'kind':'mergeSync','identity':self.identity,'resolutions':[{'object':self.key,'revisions':[v['revision'] for v in view['versions']],'state':branch['state']}],'blobs':{}})
        self.transfer(self.b,'push',self.rb,[self.key]);self.transfer(self.a,'pull',objects=[self.key])
        self.plan(self.a,self.ra,{'kind':'applySync','identity':self.identity,'objects':[self.key]})
        self.assertEqual((self.ra/'Note/Note.typ').read_text(encoding='utf-8'),a)

    def test_upload_retry_revoke_regrant_restart_and_integrity(self):
        data=b'A'*65536+'尾部'.encode('utf-8')
        record={'manifest':False,'id':sha(data),'bytes':len(data)}
        upload=self.remote('sync/upload',{'identity':self.identity,'record':record})
        first={'ticket':upload['ticket'],'offset':0,'bytes':base64.b64encode(data[:65536]).decode()}
        reply=self.remote('sync/append',first);self.assertFalse(reply['complete'])
        self.assertEqual(reply,self.remote('sync/append',first))
        wrong={**first,'bytes':base64.b64encode(b'B'*65536).decode()};self.remote('sync/append',wrong,expected=400)
        resumed=self.remote('sync/upload',{'identity':self.identity,'record':record});self.assertEqual(resumed['offset'],65536);self.assertEqual(resumed['ticket'],upload['ticket'])
        self.policy(lambda u:u['owner'].update(syncOwner=False))
        self.remote('sync/append',{'ticket':upload['ticket'],'offset':65536,'bytes':base64.b64encode(data[65536:]).decode()},expected=404)
        self.policy(lambda u:u['owner'].update(syncOwner=True))
        self.remote('sync/append',first,expected=404)
        new=self.remote('sync/upload',{'identity':self.identity,'record':record});self.assertEqual(new['offset'],0)
        self.remote('sync/append',{**first,'ticket':new['ticket']})
        final={'ticket':new['ticket'],'offset':65536,'bytes':base64.b64encode(data[65536:]).decode()}
        done=self.remote('sync/append',final);self.assertTrue(done['complete']);self.assertEqual(done,self.remote('sync/append',final))
        self.remote('sync/append',{**final,'bytes':base64.b64encode(b'X'*len(data[65536:])).decode()},expected=400)
        self.server.close();self.server=Server(['--data-dir',str(self.folder/'relay-state'),'--server-config',str(self.config_path)],hosted=True)
        self.remote('sync/append',final,expected=404)
        complete=self.remote('sync/upload',{'identity':self.identity,'record':record});self.assertTrue(complete['complete']);self.assertIsNone(complete['ticket'])
        damaged={'manifest':False,'id':sha(b'good'),'bytes':4}
        bad=self.remote('sync/upload',{'identity':self.identity,'record':damaged})
        self.remote('sync/append',{'ticket':bad['ticket'],'offset':0,'bytes':base64.b64encode(b'evil').decode()},expected=409)
        fresh=self.remote('sync/upload',{'identity':self.identity,'record':damaged});self.assertEqual(fresh['offset'],0)
        self.assertTrue(self.remote('sync/append',{'ticket':fresh['ticket'],'offset':0,'bytes':base64.b64encode(b'good').decode()})['complete'])

    def test_frozen_download_and_every_chunk_revocation_do_not_affect_another_owner(self):
        self.seed_clone()
        prepared=self.remote('sync/prepare',{'identity':self.identity,'objects':[self.key]})
        other=self.remote('sync/prepare',{'identity':self.identity,'objects':[self.key]},actor='owner2')
        page=self.remote('sync/catalog',{'ticket':prepared['ticket'],'offset':0}); old=copy.deepcopy(page)
        self.save(self.a,self.ra,'Note','newer');self.transfer(self.a,'push',self.ra,[self.key])
        self.assertEqual(old,self.remote('sync/catalog',{'ticket':prepared['ticket'],'offset':0}))
        chunk=self.remote('sync/chunk',{'ticket':prepared['ticket'],'record':0,'offset':0})
        self.policy(lambda u:u['owner'].update(syncOwner=False))
        for route,body in [('sync/catalog',{'ticket':prepared['ticket'],'offset':0}),('sync/chunk',{'ticket':prepared['ticket'],'record':0,'offset':0}),('sync/status',{'identity':self.identity})]:self.remote(route,body,expected=404)
        self.assertEqual(chunk,self.remote('sync/chunk',{'ticket':other['ticket'],'record':0,'offset':0},actor='owner2'))
        self.policy(lambda u:u['owner'].update(syncOwner=True))
        self.remote('sync/chunk',{'ticket':prepared['ticket'],'record':0,'offset':0},expected=404)
        self.remote('sync/chunk',{'ticket':other['ticket'],'record':0,'offset':0},actor='owner',expected=404)

    def test_document_grants_do_not_imply_sync_and_bad_pair_never_discloses(self):
        self.remote('open',{'resource':'shared'},actor='author')
        for route in ['status','prepare','upload']:
            body={'identity':self.identity}
            if route=='upload':body['record']={'manifest':False,'id':sha(b''),'bytes':0}
            denied=self.remote('sync/'+route,body,actor='author',expected=404)
            body['identity']={**self.identity,'workspaceId':uid()}
            self.assertEqual(denied,self.remote('sync/'+route,body,actor='author',expected=404))
        for route,body in [('chunk',{'ticket':uid(),'record':0,'offset':0}),('catalog',{'ticket':uid(),'offset':0}),('append',{'ticket':uid(),'offset':0,'bytes':''}),('close',{'ticket':uid()})]:self.remote('sync/'+route,body,actor='author',expected=404)
        self.remote('sync/status',{'identity':{**self.identity,'genesis':'0'*64}},expected=404)
        self.remote('sync/status',{'identity':self.identity},actor='admin',expected=401)
        for endpoint in ['http://remote.invalid','http://127.0.0.1/path','https://user:secret@example.invalid','https://example.invalid/?token=x']:
            self.local(self.a,'/api/sync/remote/inspect',expected=400,**{**self.connection(),'endpoint':endpoint})

    def test_hosted_author_saves_publish_to_bound_replica_without_sync_permission(self):
        # The same Core writer may serve authorized document edits and own its
        # existing replica outbox. Merely sharing a raw tree is never used.
        self.server.close(); self.a.close()
        self.config['root']=str(self.ra);self.config['files']['shared']['path']='Note/Note.typ'
        self.config_path.write_text(json.dumps(self.config),encoding='utf-8')
        self.server=Server(['--data-dir',str(self.folder/'state-a'),'--server-config',str(self.config_path)],hosted=True)
        doc=self.remote('open',{'resource':'shared'},actor='author')
        source=doc['source']+'\n服务端作者保存'
        result=self.remote('save',{'sessionId':doc['sessionId'],'baseId':doc['baseId'],'source':source,'operationId':uid()},actor='author')
        self.assertEqual(result['status'],'committed')
        self.transfer(self.b,'pull',objects=[self.key])
        self.plan(self.b,self.rb,{'kind':'cloneSync','identity':self.identity,'objects':[self.key]})
        self.assertEqual((self.rb/'Note/Note.typ').read_text(encoding='utf-8'),source)
        self.remote('sync/status',{'identity':self.identity},actor='author',expected=404)

    def test_no_redirect_credential_forwarding_and_bounded_remote_response(self):
        hits=[]
        class Handler(BaseHTTPRequestHandler):
            def do_POST(handler):
                handler.rfile.read(int(handler.headers.get('Content-Length',0)))
                hits.append(handler.path)
                if handler.path.endswith('/status'):
                    handler.send_response(307);handler.send_header('Location',f'http://127.0.0.1:{handler.server.server_port}/leak');handler.end_headers()
                else: handler.send_response(200);handler.end_headers()
            def log_message(self,*args):pass
        server=ThreadingHTTPServer(('127.0.0.1',0),Handler)
        thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
        self.addCleanup(server.server_close);self.addCleanup(server.shutdown)
        result=self.local(self.a,'/api/sync/remote/inspect',expected=502,**{**self.connection(),'endpoint':f'http://127.0.0.1:{server.server_port}'})
        self.assertEqual(hits,['/api/hosted/sync/status']);self.assertNotIn(self.tokens['owner'],json.dumps(result))
        def huge(handler):
            handler.rfile.read(int(handler.headers.get('Content-Length',0)));handler.send_response(200);handler.send_header('Content-Length',str(17*1024*1024));handler.end_headers()
        Handler.do_POST=huge
        self.local(self.a,'/api/sync/remote/inspect',expected=400,**{**self.connection(),'endpoint':f'http://127.0.0.1:{server.server_port}'})

    def test_stop_between_chunks_and_restart_reuses_only_complete_records(self):
        data=b'bounded-chunk-'+b'x'*70000
        record={'manifest':False,'id':sha(data),'bytes':len(data)}
        identity=self.identity; entered=threading.Event(); release=threading.Event(); chunks=[]
        class Handler(BaseHTTPRequestHandler):
            def do_POST(handler):
                req=json.loads(handler.rfile.read(int(handler.headers['Content-Length'])))
                route=handler.path.rsplit('/',1)[-1]
                if route=='prepare': result={'ticket':'test-ticket','identity':identity,'records':1,'bytes':len(data),'chunkBytes':65536}
                elif route=='catalog': result={'offset':0,'next':1,'complete':True,'records':[record]}
                elif route=='chunk':
                    offset=req['offset']; chunks.append(offset)
                    if len(chunks)==1:
                        entered.set();release.wait(10)
                    end=min(offset+65536,len(data))
                    result={'record':record,'offset':offset,'next':end,'complete':end==len(data),'bytes':base64.b64encode(data[offset:end]).decode()}
                else: result={'status':'closed'}
                body=json.dumps(result).encode();handler.send_response(200);handler.send_header('Content-Length',str(len(body)));handler.end_headers();handler.wfile.write(body)
            def log_message(self,*args):pass
        server=ThreadingHTTPServer(('127.0.0.1',0),Handler)
        threading.Thread(target=server.serve_forever,daemon=True).start()
        self.addCleanup(server.server_close);self.addCleanup(server.shutdown);self.addCleanup(release.set)
        connection={**self.connection(),'endpoint':f'http://127.0.0.1:{server.server_port}'}
        def start():
            identifier=uid();self.local(self.b,'/api/sync/remote/start',requestId=identifier,connection=connection,direction='pull');return identifier
        def finished(identifier, expected):
            for _ in range(200):
                result=self.local(self.b,'/api/sync/remote/job',requestId=identifier)
                if result['status']!='running':self.assertEqual(result['status'],expected,result);return result
                time.sleep(.03)
            self.fail('Transfer did not settle')
        identifier=start();self.assertTrue(entered.wait(5))
        self.local(self.b,'/api/sync/remote/stop',requestId=identifier);release.set()
        self.assertEqual(finished(identifier,'stopped')['recordsDone'],0);self.assertEqual(chunks,[0]);self.assertFalse(self.rb.exists())
        # A stopped partial blob is not a complete reusable immutable record.
        completed=start();self.assertEqual(finished(completed,'complete')['recordsDone'],1)
        self.assertEqual(chunks,[0,0,65536]);self.assertFalse(self.rb.exists())
        self.b.close();self.b=Server(['--data-dir',str(self.folder/'state-b')])
        self.assertEqual(self.local(self.b,'/api/sync/remote/job',requestId=completed)['status'],'unknown')
        finished(start(),'complete');self.assertEqual(chunks,[0,0,65536]);self.assertFalse(self.rb.exists())

    def test_pairing_is_bound_to_persisted_policy_and_old_grants_do_not_expand(self):
        self.server.close()
        changed=copy.deepcopy(self.config);changed['syncIdentity']={**self.identity,'workspaceId':uid()}
        self.config_path.write_text(json.dumps(changed),encoding='utf-8')
        rejected=subprocess.run(serve_command(str(EXE), ['--data-dir', str(self.folder / 'relay-state'), '--server-config', str(self.config_path)]),capture_output=True,text=True,encoding='utf-8',timeout=20,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
        self.assertNotEqual(rejected.returncode,0);self.assertIn('Persisted sync pairing',rejected.stderr)
        legacy=copy.deepcopy(self.config);legacy.pop('syncIdentity')
        for user in legacy['users'].values():user['syncOwner']=False
        self.config_path.write_text(json.dumps(legacy),encoding='utf-8')
        self.server=Server(['--data-dir',str(self.folder/'legacy-state'),'--server-config',str(self.config_path)],hosted=True)
        self.server.close();self.config_path.write_text(json.dumps(self.config),encoding='utf-8')
        self.server=Server(['--data-dir',str(self.folder/'legacy-state'),'--server-config',str(self.config_path)],hosted=True)
        self.remote('sync/status',{'identity':self.identity},expected=404)
        self.policy(lambda users:users['owner'].update(syncOwner=True))
        self.assertEqual(self.remote('sync/status',{'identity':self.identity})['identity'],self.identity)

if __name__=='__main__': unittest.main(argv=[sys.argv[0]],verbosity=2)
