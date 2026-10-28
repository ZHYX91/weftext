"""Actual hosted review transactions, authorization and restart recovery."""
import uuid
import json
import secrets
import sys
import time
import unittest
from concurrent.futures import ThreadPoolExecutor
from owned_worker_observer import (OwnedWorkerObservation, event_time_ns, timed_call,
                                   freeze_policy_revocation, apply_inflight_revocation,
                                   verify_revocation_delivery)
from test_hosted import uid, digest
from test_hosted_fields import HostedFields


class HostedReview(HostedFields):
    def prepare_review(self):
        for actor in ['commenter', 'targeter', 'annreader']:
            self.tokens[actor] = secrets.token_urlsafe(32)
        def update(users):
            for resource in ['Shared', 'Other']:
                users['alice']['grants'][resource]['annotations'] = dict(read=True, write=True, target=True, accept=True, proofread=True)
            for actor, grant in [('commenter', dict(read=True, write=True)), ('targeter', dict(read=True, target=True)), ('annreader', dict(read=True))]:
                users[actor] = {'enabled': True, 'tokenSha256': digest(self.tokens[actor]), 'grants': {'Shared': {'annotations': grant}}}
        self.policy(update)

    def review(self, resource='Shared', actor='alice'):
        return self.call('review/read', {'resource': resource}, actor)

    def target(self, source, text='共同正文'):
        start = source.index(text)
        return {'kind': 'source_range', 'from': len(source[:start].encode('utf-16-le'))//2,
                'to': len(source[:start+len(text)].encode('utf-16-le'))//2, 'visibleExcerpt': text}

    def prepare_annotation(self, profile, intent, actor='alice', operation=None):
        request = dict(resource=profile['resource'], basis=profile['basis'], operationId=operation or uid(), intent=intent)
        return self.call('review/prepare', request, actor), request

    def commit_annotation(self, plan, resource='Shared', actor='alice', status=200):
        return self.call('review/commit', dict(resource=resource, operationId=plan['operationId'], planId=plan['planId']), actor, status)

    def create_annotation(self, purpose='comment', replacement='', resource='Shared'):
        source = self.open(resource)['source']
        plan, request = self.prepare_annotation(self.review(resource), dict(kind='annotationCreate', purpose=purpose, body='保留这条讨论😀', replacement=replacement, target=self.target(source)))
        self.assertEqual(plan['status'], 'prepared')
        self.assertEqual(self.commit_annotation(plan, resource)['status'], 'committed')
        return self.review(resource)['annotations'][0], request, plan

    def test_review_independent_permissions_exact_targets_and_original_restart(self):
        self.prepare_review()
        self.assertEqual(self.call('review', actor='bob')['resources'], [])
        self.call('review/read', {'resource': 'Shared'}, 'bob', 404)
        self.call('open', {'resource': 'Shared'}, 'commenter', 404)
        before = (self.root/'Shared/Shared.typ').read_bytes()
        history = self.call('history', {'sessionId': self.open()['sessionId']})['versions']
        annotation, request, plan = self.create_annotation()
        self.assertEqual((self.root/'Shared/Shared.typ').read_bytes(), before)
        self.assertEqual(self.call('history', {'sessionId': self.open()['sessionId']})['versions'], history)
        limited = self.review(actor='commenter')
        encoded = json.dumps(limited, ensure_ascii=False)
        for hidden in ['共同正文', 'sourceLocation', 'sourceHash', str(self.root), 'weftext-node://']:
            self.assertNotIn(hidden, encoded)
        self.assertEqual(limited['annotations'][0]['body'], '保留这条讨论😀')
        target = self.review(actor='targeter')
        self.assertEqual(target['annotations'][0]['visibleExcerpt'], '共同正文')
        self.assertEqual(target['annotations'][0]['targetResolution'], 'exact')
        self.assertNotIn('sourceHash', target)
        self.call('review/prepare', {**request, 'operationId': uid()}, 'annreader', 404)
        self.server.close(); self.start()
        self.assertEqual(self.call('review/prepare', request)['status'], 'committed')
        self.assertEqual(self.commit_annotation(plan)['status'], 'committed')
        wrong = {**request, 'intent': {**request['intent'], 'body': 'different intent'}}
        self.call('review/prepare', wrong, status=409)
        self.assertEqual(self.call('review/operation', {'resource': 'Shared', 'operationId': plan['operationId']}, 'commenter')['status'], 'notRegistered')
        self.assertEqual(len(self.review()['annotations']), 1)

    def test_review_prepared_restart_keeps_original_preview_and_commit(self):
        self.prepare_review()
        source = self.open()['source']
        plan, request = self.prepare_annotation(self.review(), dict(kind='annotationCreate', purpose='comment', body='原准备稿😀', target=self.target(source)))
        self.assertEqual(plan['status'], 'prepared')
        self.assertFalse((self.root/'Shared/weftext.annotations.json').exists())
        self.server.close(); self.start()
        recovered = self.call('review/prepare', request)
        self.assertEqual(recovered['planId'], plan['planId'])
        self.assertEqual(recovered['preview'], plan['preview'])
        self.assertEqual(self.commit_annotation(recovered)['status'], 'committed')
        self.assertEqual(self.review()['annotations'][0]['body'], '原准备稿😀')
        self.assertEqual((self.root/'Shared/Shared.typ').read_text('utf-8'), source)

    def test_review_reply_state_without_target_and_keep_unknown_members(self):
        self.prepare_review()
        annotation, _, _ = self.create_annotation()
        path = self.root/'Shared/weftext.annotations.json'
        original = json.loads(path.read_text('utf-8'))
        original['retainedFutureExtension'] = {'sentinel': 'KEEP_UNKNOWN'}
        path.write_text(json.dumps(original, ensure_ascii=False), 'utf-8')
        profile = self.review(actor='commenter')
        plan, _ = self.prepare_annotation(profile, dict(kind='annotationReply', annotationId=annotation['id'], body='独立批注权回复', attribution='spoofed'), 'commenter')
        # A discussion-only plan is independent from the target source preimage.
        doc = self.open(); self.save(doc, doc['source']+'\n源另有修改')
        self.assertEqual(self.commit_annotation(plan, actor='commenter')['status'], 'committed')
        replies = self.review(actor='commenter')['annotations'][0]['replies']
        self.assertEqual(replies[0]['body'], '独立批注权回复')
        self.assertEqual(replies[0]['author'], 'commenter')
        self.assertEqual(json.loads(path.read_text('utf-8'))['retainedFutureExtension'], original['retainedFutureExtension'])
        plan, _ = self.prepare_annotation(self.review(actor='commenter'), dict(kind='annotationResolve', annotationId=annotation['id'], resolved=True), 'commenter')
        self.commit_annotation(plan, actor='commenter')
        self.assertEqual(self.review(actor='annreader')['annotations'][0]['reviewState'], 'resolved')

    def test_review_suggestion_atomic_source_and_state_preserves_draft_and_stale_plan(self):
        self.prepare_review()
        annotation, _, _ = self.create_annotation('suggestion', '校对后的正文😀')
        doc = self.open(); self.call('draft', {'inputId': str(uuid.uuid4()), 'baseId': doc['baseId'], 'inputRevision': 1, 'sessionId': doc['sessionId'], 'source': doc['source']+'\nMY_PRIVATE_DRAFT'})
        source_before = (self.root/'Shared/Shared.typ').read_text('utf-8')
        sidecar_before = (self.root/'Shared/weftext.annotations.json').read_bytes()
        plan, _ = self.prepare_annotation(self.review(), dict(kind='annotationAccept', annotationId=annotation['id']))
        self.assertEqual((self.root/'Shared/Shared.typ').read_text('utf-8'), source_before)
        self.assertEqual((self.root/'Shared/weftext.annotations.json').read_bytes(), sidecar_before)
        result = self.commit_annotation(plan); self.assertEqual(result['status'], 'committed')
        self.assertEqual((self.root/'Shared/Shared.typ').read_text('utf-8'), source_before.replace('共同正文', '校对后的正文😀'))
        self.assertEqual(self.review()['annotations'][0]['suggestionState'], 'accepted')
        self.assertIn('MY_PRIVATE_DRAFT', self.open()['recoveredDraft'])
        # A stale sidecar before-image refuses the full new operation.
        p = self.review(actor='commenter')
        first, _ = self.prepare_annotation(p, dict(kind='annotationReply', annotationId=annotation['id'], body='first'), 'commenter')
        second, _ = self.prepare_annotation(p, dict(kind='annotationReply', annotationId=annotation['id'], body='second'), 'commenter')
        self.commit_annotation(first, actor='commenter'); self.commit_annotation(second, actor='commenter', status=409)
        self.assertEqual([r['body'] for r in self.review()['annotations'][0]['replies']], ['first'])

    def test_review_revoke_regrant_original_settlement_and_unrelated_progress(self):
        self.prepare_review()
        annotation, _, _ = self.create_annotation()
        plan, req = self.prepare_annotation(self.review(), dict(kind='annotationReply', annotationId=annotation['id'], body='must not appear'))
        self.policy(lambda users: users['alice']['grants']['Shared']['annotations'].update(write=False, accept=False))
        self.commit_annotation(plan, status=404)
        self.policy(lambda users: users['alice']['grants']['Shared']['annotations'].update(write=True, accept=True))
        self.commit_annotation(plan, status=409)
        self.assertEqual(self.call('review/settle', {'resource': 'Shared', 'operationId': plan['operationId']})['status'], 'cancelled')
        self.assertEqual(self.call('review/prepare', req)['status'], 'cancelled')
        profile = self.review(); operation = uid()
        self.assertEqual(self.call('review/settle', {'resource': 'Shared', 'operationId': operation})['status'], 'cancelled')
        delayed, _ = self.prepare_annotation(profile, dict(kind='annotationReply', annotationId=annotation['id'], body='late'), operation=operation)
        self.assertEqual(delayed['status'], 'cancelled')
        # A different configured object remains independently writable.
        source = self.open('Other')['source']
        plan, _ = self.prepare_annotation(self.review('Other'), dict(kind='annotationCreate', purpose='comment', body='other', target={'kind': 'whole_node'}))
        self.commit_annotation(plan, 'Other')
        self.assertEqual((self.root/'Other/Other.typ').read_text('utf-8'), source)
        self.assertEqual(self.review()['annotations'][0]['replies'], [])

    def test_review_proofread_uses_frozen_visible_text_and_batch_no_implicit_recheck(self):
        self.prepare_review()
        doc = self.open(); source = doc['source']+'\n\n错*别*字 和 错别字，配对（未结束。\n'
        self.save(doc, source)
        options = dict(characters=True, typos=True, pairs=True, sensitive=True, allowlist=[], selection=None,
                       rules=[dict(id='owned.typo', version='1', category='typo', text='错别字', message='请核对', replacement='正确字', scope={'universal': True})])
        profile = self.review()
        scan = self.call('review/proofread', dict(resource='Shared', basis=profile['basis'], options=options))
        typos = [f for f in scan['findings'] if f['ruleId']=='owned.typo']
        self.assertEqual(len(typos), 2)
        self.assertTrue(any(f['sourceRange'] for f in typos))
        findings = [f['id'] for f in scan['findings'] if f['sourceRange']]
        intent = {'annotationLocale': 'zh-CN', 'kind': 'annotationProofread', 'options': options, 'findings': findings, 'scanId': scan['scanId'], 'ignore': False}
        self.call('review/prepare', dict(resource='Shared', basis=profile['basis'], operationId=uid(), intent={key: value for key, value in intent.items() if key != 'scanId'}), status=400)
        plan, _ = self.prepare_annotation(profile, intent)
        self.commit_annotation(plan)
        self.assertEqual((self.root/'Shared/Shared.typ').read_text('utf-8'), source)
        self.assertEqual(len(self.review()['annotations']), len(findings))
        profile = self.review(); scan = self.call('review/proofread', dict(resource='Shared', basis=profile['basis'], options=options))
        self.assertTrue(set(findings).issubset(set(scan['existing'])))
        updated = self.open(); self.save(updated, updated['source']+'\n后来改稿')
        self.call('review/prepare', dict(resource='Shared', basis=profile['basis'], operationId=uid(), intent={**intent, 'scanId':scan['scanId']}), status=409)
        self.call('review/proofread', dict(resource='Shared', basis=profile['basis'], options=options), 'commenter', 404)

    def test_review_frozen_dependencies_revocation_and_configuration_binding(self):
        self.prepare_review()
        annotation, _, _ = self.create_annotation()
        original, _ = self.prepare_annotation(self.review(), dict(kind='annotationReply', annotationId=annotation['id'], body='ORIGINAL_PRIVATE_REVIEW'))
        before = (self.root/'Shared/weftext.annotations.json').read_bytes()
        self.server.close()
        # A newly configured alias cannot acquire the old resource's preview or writer.
        self.config['files']['Shared']['path'], self.config['files']['Other']['path'] = self.config['files']['Other']['path'], self.config['files']['Shared']['path']
        self.start()
        status = self.call('review/operation', {'resource': 'Shared', 'operationId': original['operationId']})
        self.assertEqual(status['status'], 'prepared')
        self.assertFalse(status['commitAllowed'])
        self.assertNotIn('ORIGINAL_PRIVATE_REVIEW', json.dumps(status))
        self.commit_annotation(original, status=409)
        self.assertEqual((self.root/'Shared/weftext.annotations.json').read_bytes(), before)
        self.call('review/settle', {'resource': 'Shared', 'operationId': original['operationId']})
        self.server.close()
        self.config['files']['Shared']['path'], self.config['files']['Other']['path'] = self.config['files']['Other']['path'], self.config['files']['Shared']['path']
        self.config['files']['Shared']['inputs'] = ['Secret']
        self.start()
        doc = self.open(); self.save(doc, doc['source']+'\n错别字\n#include "../Secret/Secret.typ"')
        options = dict(characters=True, typos=True, pairs=True, sensitive=True, allowlist=[], selection=None,
                       rules=[dict(id='owned.typo', version='1', category='typo', text='错别字', message='请核对', replacement='正确字', scope={'universal': True})])
        profile = self.review(); scan = self.call('review/proofread', dict(resource='Shared', basis=profile['basis'], options=options))
        findings = [f['id'] for f in scan['findings'] if f['sourceRange']]
        self.assertTrue(findings)
        prepared, _ = self.prepare_annotation(profile, {'annotationLocale': 'zh-CN', 'kind': 'annotationProofread', 'options': options, 'findings': findings, 'scanId': scan['scanId'], 'ignore': False})
        sidecar = (self.root/'Shared/weftext.annotations.json').read_bytes()
        dependency = self.open('Secret'); self.save(dependency, dependency['source']+'\nDEPENDENCY_CHANGED')
        self.commit_annotation(prepared, status=409)
        self.assertEqual((self.root/'Shared/weftext.annotations.json').read_bytes(), sidecar)
        self.call('review/settle', {'resource': 'Shared', 'operationId': prepared['operationId']})
        self.policy(lambda users: users['bob']['grants']['Shared'].update(annotations=dict(read=True, target=True, proofread=True)))
        b = self.review(actor='bob')
        self.call('review/proofread', dict(resource='Shared', basis=b['basis'], options=options), 'bob', 404)
        # An in-flight compilation may not deliver its old text after revocation.
        doc = self.open()
        included = '#include "../Secret/Secret.typ"'
        self.assertEqual(doc['source'].count(included), 1)
        # Retain the real frozen dependency as text: a whole managed note's
        # document set rule cannot execute inside the compiled content container.
        source = doc['source'].replace(included, '#read("../Secret/Secret.typ")', 1)
        # Construct 20,000 native paragraph contents, but project only their
        # count and a sample: this is a delivery race, not an output-capacity test.
        # The full projection exceeds the unchanged tree/worker-output budgets.
        workload = ('\n#let inflight-paragraphs = range(20000).map(i => [delayed paragraph#parbreak()])\n'
                    '#text(str(inflight-paragraphs.len()))\ndelayed paragraph')
        self.save(doc, source+workload)
        profile = self.review()
        target = ('alice', 'grants', 'Shared', 'annotations', 'proofread')
        policy_request = freeze_policy_revocation(self.call('admin/policy', actor='admin'), target)
        deadline = time.monotonic() + 40
        with ThreadPoolExecutor(max_workers=1) as pool:
            with OwnedWorkerObservation(self.server.owner) as observation:
                self.assertLess(time.monotonic(), deadline, 'Original request/mutation budget exhausted')
                submitted = event_time_ns()
                pending = pool.submit(timed_call, self.server.call, '/api/hosted/review/proofread', dict(resource='Shared', basis=profile['basis'], options=options), self.tokens['alice'])
                worker = observation.wait(pending, deadline=deadline)
                observation.assert_inflight(pending)
                self.assertLess(time.monotonic(), deadline, 'Original request/mutation budget exhausted before change')
                mutation = apply_inflight_revocation(observation, pending, policy_request,
                    lambda update: self.call('admin/policy', update, 'admin'), deadline)
                (code, response, _), completed = pending.result(timeout=max(0, deadline-time.monotonic()))
        self.assertLess(time.monotonic(), deadline, 'Original request/mutation budget exhausted before readback')
        readback = self.call('admin/policy', actor='admin')
        self.assertLess(time.monotonic(), deadline, 'Original request/mutation budget exhausted during readback')
        trace = {'submittedMonotonicNs': submitted, 'worker': worker,
                 **mutation, 'requestCompletedMonotonicNs': completed,
                 'policyReadbackGeneration': readback.get('generation'),
                 'targetAuthorizationAfter': readback['users']['alice']['grants']['Shared']['annotations']['proofread'],
                 'status': code, 'errorCode': response.get('error', {}).get('code'),
                 'explanationKey': response.get('error', {}).get('explanation', {}).get('key')}
        with (self.folder/'proofread-revocation-inflight.json').open('x', encoding='utf-8') as evidence:
            evidence.write(json.dumps(trace, ensure_ascii=False, indent=2))
        verify_revocation_delivery(policy_request, target, readback, trace, response,
                                   (404, 'unavailable', 'error.api.hosted.contentUnavailable', 'findings'))


if __name__ == '__main__': unittest.main(argv=[sys.argv[0]], defaultTest='HostedReview', verbosity=2)
