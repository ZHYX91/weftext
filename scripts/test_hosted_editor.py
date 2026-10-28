"""Hosted native draft consumers using the actual executable and owned roots."""
import json
import copy
import sys
import time
import unittest
from concurrent.futures import ThreadPoolExecutor
from test_hosted import Hosted, uid, digest
from owned_worker_observer import (SUPPORTED, OwnedWorkerObservation, event_time_ns, timed_call,
    apply_inflight_mutation, verify_inflight_change, freeze_policy_changes,
    apply_inflight_revocation, verify_policy_changes)

PURE_COMPUTE = '#let checksum = range(500000).fold(0, (acc, i) => acc + i)\nFinal #checksum'

class HostedEditor(Hosted):
    def assert_inflight_projection_change(self, basis, change, expected, evidence_name, error_code=None,
                                         *, verify_change=None, policy_request=None, policy_paths=None, deadline=None):
        if not SUPPORTED:
            self.skipTest('In-flight compiler race is unverified without the Windows owned-worker boundary')
        deadline = deadline if deadline is not None else time.monotonic() + 40
        with ThreadPoolExecutor(max_workers=1) as pool:
            with OwnedWorkerObservation(self.server.owner) as observation:
                submitted = event_time_ns()
                pending = pool.submit(timed_call, self.server.call, '/api/hosted/editor/render', basis, self.tokens['alice'])
                worker = observation.wait(pending, deadline=deadline)
                self.assertFalse(pending.done())
                observation.assert_inflight(pending)
                if policy_request is not None:
                    mutation = apply_inflight_revocation(observation, pending, policy_request,
                        lambda update: self.call('admin/policy', update, 'admin'), deadline)
                    acknowledgment = None
                else:
                    acknowledgment, mutation = apply_inflight_mutation(observation, pending, change, deadline)
                (code, result, _), completed = pending.result(timeout=max(0, deadline-time.monotonic()))
        trace = {'submittedMonotonicNs':submitted, 'worker':worker,
                 **mutation, 'requestCompletedMonotonicNs':completed,
                 'status':code, 'errorCode':result.get('error',{}).get('code')}
        (self.folder/(evidence_name+'.json')).write_text(json.dumps(trace,ensure_ascii=False,indent=2),'utf-8')
        verify_inflight_change(trace)
        self.assertLess(time.monotonic(), deadline, 'Original mutation/readback budget exhausted')
        if policy_request is not None:
            readback = self.call('admin/policy', actor='admin')
            verify_policy_changes(policy_request, policy_paths, readback, trace)
        elif verify_change is not None:
            verify_change(acknowledgment)
        self.assertLess(time.monotonic(), deadline, 'Original mutation/readback budget exhausted')
        self.assertEqual(code,expected,result)
        if error_code is not None:self.assertEqual(result['error']['code'],error_code)

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
        with self.subTest(race='dependency-currentness'):
            # Reading the authorized note avoids executing its document set rules
            # inside a content container. Pure computation keeps output small.
            basis={'sessionId':a['sessionId'],'source':'#read("../Secret/Secret.typ")\n'+PURE_COMPUTE,'serial':2}
            source = dependency['source']+'\n新的依赖正文'; operation = uid()
            deadline=time.monotonic()+40
            acknowledged=self.call('draft',{'sessionId':dependency['sessionId'],'baseId':dependency['baseId'],
                'source':source,'inputId':uid(),'inputRevision':1})
            self.assertEqual(acknowledged['status'],'inputAcknowledged')
            request={'sessionId':dependency['sessionId'],'baseId':dependency['baseId'],'source':source,
                     'operationId':operation,'inputRevision':acknowledged['inputRevision']}
            frozen=copy.deepcopy(request)
            def verify_save(status):
                self.assertEqual(request,frozen)
                self.committed_document(dependency, source, status, request)
            # Supplying the actual ACK revision avoids call('save') preparing a
            # hidden Draft POST inside the measured mutation interval.
            self.assert_inflight_projection_change(basis,lambda:self.call('save',request),409,
                'projection-dependency-race','staleEnvironment',verify_change=verify_save,deadline=deadline)
        b=self.call('open',{'resource':'Shared'},'bob')
        self.call('editor/render',{'sessionId':b['sessionId'],'source':'普通文字','serial':1},'bob',404)

    def test_late_projection_cannot_revive_old_serial_or_revoked_session(self):
        a=self.call('open',{'resource':'Shared'})
        slow={'sessionId':a['sessionId'],'source':PURE_COMPUTE,'serial':1}
        with self.subTest(race='newer-serial'):
            newer = {**slow,'source':'更新后的草稿','serial':2}
            def verify_map(result):
                self.assertEqual(result['serial'],newer['serial']); self.assertEqual(result['sourceHash'],digest(newer['source']))
                self.call('editor/map',slow,status=409)
                actual = self.call('editor/map',newer)
                self.assertEqual(actual['serial'],2); self.assertEqual(actual['sourceHash'],digest(newer['source']))
            self.assert_inflight_projection_change(slow,lambda:self.call('editor/map',newer),409,
                'projection-serial-race',verify_change=verify_map)
        with self.subTest(race='revoked-session'):
            deadline=time.monotonic()+40
            target=('alice','grants','Shared','write')
            request=freeze_policy_changes(self.call('admin/policy',actor='admin'),[(target,False)])
            self.assert_inflight_projection_change({**slow,'serial':3},None,410,'projection-revoked-race',
                policy_request=request,policy_paths=[target],deadline=deadline)

if __name__=='__main__': unittest.main(argv=[sys.argv[0]],defaultTest='HostedEditor',verbosity=2)
