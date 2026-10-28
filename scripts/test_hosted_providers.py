"""Provider Query grants and frozen versions through the actual hosted server.

Uses owned libraries and the existing local installer for fixture provisioning;
all consumer requests use hosted HTTP. No host desktop input or external data.
"""
import copy
import json
import sys
import time
import unittest
from concurrent.futures import ThreadPoolExecutor
from owned_worker_observer import OwnedWorkerObservation, event_time_ns, timed_call, freeze_policy_changes, apply_inflight_revocation, verify_policy_changes
from test_hosted import uid
from test_hosted_definition import HostedDefinition
from test_hosted_query import HostedQuery
from test_hosted_office import HostedOffice


class HostedProviders(HostedQuery):
    def install_schema(self, change):
        def install(api):
            before = api('/api/data/schema', {'root': str(self.root)})
            schema = json.loads(before['source'])
            change(schema)
            operation = uid()
            plan = api('/api/data/schema/prepare', {
                'root': str(self.root), 'source': json.dumps(schema, ensure_ascii=False),
                'beforeSource': before['source'], 'beforeRevision': before['revision'],
                'operationId': operation})
            api('/api/data/schema/install', {'operationId': operation, 'planId': plan['planId']})
            return schema
        return self.local(install)

    def prepare_providers(self):
        self.prepare_query()
        def change(schema):
            calendar = copy.deepcopy(next(p for p in schema['providers'] if p['id'] == 'calendars/gregorian'))
            calendar['id'] = 'calendars/private'
            calendar['implementation']['id'] = 'weftext.calendar.table.v2'
            calendar['data'] = {'kind': 'calendar-table', 'key': 'private-calendar', 'version': 'private-v1',
                'epoch_date': '2026-01-01', 'years': [
                    {'year': year, 'months': [{'number': 1, 'leap': False, 'days': 30}]}
                    for year in [1, 2]]}
            schema['providers'].append(calendar)
            series = copy.deepcopy(schema['series_policies'][0])
            series.update(id='calendar/private', key='private-series', calendar='calendars/private',
                          rule='table-ordinal-v1', kinds=['day'])
            schema['series_policies'].append(series)
            schema['definitions']['PrivateDecision'] = {'kind': 'enum', 'values': ['ONLY_INTERNAL_CHOICE'], 'open': False}
        self.install_schema(change)

    def grant_providers(self, providers=(), series=(), actor='fields'):
        self.policy(lambda users: users[actor].update(queryProviders={
            'providers': list(providers), 'seriesPolicies': list(series)}))

    def selections(self, providers=(), series=(), actor='fields'):
        catalog = self.call('query/catalog', actor=actor)
        known = {p['id']: p for p in catalog['providers']}
        policies = {p['id']: p for p in catalog['seriesPolicies']}
        return {'providers': {key: known[key]['fingerprint'] for key in providers},
                'seriesPolicies': {key: policies[key]['fingerprint'] for key in series}}

    @staticmethod
    def private_date(calendar='private-calendar', version='private-v1', year=1):
        return {'calendar': calendar, 'version': version, 'precision': 'day', 'year': year, 'month': 1, 'day': 1}

    @staticmethod
    def native_record(values):
        return {'kind': 'record', 'value': [{'key': key, 'value': {
            'kind': 'int' if isinstance(value, int) else 'text', 'value': str(value)}} for key, value in values.items()]}

    def require_unavailable(self, plan, **extra):
        code, result, _ = self.server.call('/api/hosted/query', {
            'fields': ['tasks/status'], 'plan': plan, **extra}, self.tokens['fields'])
        if code == 200:
            self.assertEqual(result['result']['coverage'], 'partial', result)
        else:
            self.assertIn(code, [400, 404, 409], result)
            self.assertNotIn('queryId', result)
        self.assertNotIn('epoch_date', json.dumps(result))
        self.assertNotIn(str(self.root), json.dumps(result))
        return code, result

    def test_provider_grants_are_independent_and_catalog_does_not_disclose_data(self):
        self.prepare_providers()
        for actor in ['alice', 'fields', 'fieldreader']:
            catalog = self.call('query/catalog', actor=actor)
            self.assertEqual(catalog['providers'], [])
            self.assertEqual(catalog['seriesPolicies'], [])
            encoded = json.dumps(catalog)
            for hidden in ['private-calendar', 'private-v1', 'private-series', 'epoch_date']:
                self.assertNotIn(hidden, encoded)
        plan = {'select': [self.column('nextCivilDay(date("2026-10-14"))', 'tomorrow')]}
        self.query(plan, status=400)
        # An explicitly requested ungranted descriptor cannot discover whether
        # the provider ID exists, even when there are no requested result rows.
        hidden = self.query(plan, resources=[], providers={'calendars/private': '0' * 64}, status=404)
        missing = self.query(plan, resources=[], providers={'calendars/not-there': '0' * 64}, status=404)
        self.assertEqual(hidden, missing)
        self.grant_providers(['temporal/rules', 'calendars/private'])
        catalog = self.call('query/catalog', actor='fields')
        self.assertEqual({p['id'] for p in catalog['providers']}, {'temporal/rules', 'calendars/private'})
        self.assertEqual(catalog['seriesPolicies'], [])
        encoded = json.dumps(catalog)
        for hidden in ['epoch_date', 'years', 'private-series', str(self.root), 'PRIVATE_']:
            self.assertNotIn(hidden, encoded)
        # Granting the registry still grants no author-source/definition rights.
        self.call('open', {'resource': 'Shared'}, 'fields', 404)
        self.call('query/definitions', {'resource': 'Shared'}, 'fields', 404)

    def test_provider_temporal_calls_use_selected_profile_and_preserve_source(self):
        self.prepare_providers()
        self.grant_providers(['temporal/rules'])
        choice = self.selections(['temporal/rules'])
        paths = [self.root / name / (name + '.typ') for name in ['Shared', 'Other', 'Secret']]
        originals = [path.read_bytes() for path in paths]
        expression = ('containsDate({"start":date("2026-10-01"),"end":date("2026-10-31")},day)'
                      ' && nextCivilDay(day)==date("2026-10-15") && status=="open"')
        plan = {'select': [self.column(expression, 'matches')],
                'advanced': {'parameters': [{'name': 'day', 'shape': {'kind': 'scalar', 'semantic': 'date'}}]}}
        result = self.query(plan, parameters={'day': {'kind': 'date', 'value': '2026-10-14'}}, **choice)['result']
        self.assertEqual(result['coverage'], 'complete')
        rows = {row['entityKey']: row for row in result['rows']}
        self.assertEqual(set(rows), {'Shared', 'Other'})
        self.assertTrue(rows['Shared']['cells']['matches']['value']['value'])
        self.assertFalse(rows['Other']['cells']['matches']['value']['value'])
        self.assertEqual([path.read_bytes() for path in paths], originals)
        self.query(plan, parameters={'day': {'kind': 'date', 'value': '2026-10-14'}}, status=400)

    def test_provider_exact_table_parameter_shape_and_pinned_baseline(self):
        self.prepare_providers()
        ids = ['temporal/rules', 'calendars/private']
        self.grant_providers(ids)
        self.grant_query(definitions=True, history=True)
        choice = self.selections(ids)
        literal = 'compareTemporal(' + json.dumps(self.private_date()) + ', date("2026-01-01"))'
        plan = {'select': [self.column(literal, 'order')]}
        first = self.query(plan, **choice)
        self.assertEqual(first['result']['coverage'], 'complete')
        self.assertTrue(all(row['cells']['order']['value'] == {'kind': 'int', 'value': '0'}
                            for row in first['result']['rows']))
        parameter_plan = {'select': [self.column('compareTemporal(day,date("2026-01-01"))', 'order')],
            'advanced': {'parameters': [{'name': 'day', 'shape': {'kind': 'use', 'name': 'CalendarDate'}}]}}
        parameter = self.query(parameter_plan, parameters={'day': self.native_record(self.private_date())}, **choice)
        self.assertEqual(parameter['result']['coverage'], 'complete')
        self.assertEqual(parameter['result']['rows'][0]['cells']['order']['value']['value'], '0')
        definition = self.add_definition(plan)
        pinned = {'advanced': {'call': {'owner': 'Other', 'selection': {
            'mode': 'pinned', 'key': definition['definition']['key'], 'revision': definition['definitionRevision']}}}}
        self.assertEqual(self.query(pinned, **choice)['result']['rows'][0]['cells']['order']['value']['value'], '0')
        self.install_schema(lambda schema: next(p for p in schema['providers'] if p['id'] == 'calendars/private')['data'].update(epoch_date='2026-01-02'))
        self.query(plan, **choice, status=409)
        fresh = self.selections(ids)
        self.assertNotEqual(fresh['providers']['calendars/private'], choice['providers']['calendars/private'])
        self.assertEqual(self.query(plan, **fresh)['result']['rows'][0]['cells']['order']['value']['value'], '1')
        # An exact historical definition keeps its own old semantic baseline;
        # today's different data cannot make its old Provider contract current.
        self.query(pinned, **fresh, status=409)

    def test_provider_empty_scope_and_unavailable_versions_are_not_complete_zero(self):
        self.prepare_providers()
        self.grant_providers(['temporal/rules'])
        temporal = self.selections(['temporal/rules'])
        hidden = 'compareTemporal(' + json.dumps(self.private_date()) + ',date("2026-01-01"))'
        absent = 'compareTemporal(' + json.dumps(self.private_date(calendar='never-installed')) + ',date("2026-01-01"))'
        hidden_results = []
        for expression in [hidden, absent]:
            hidden_results.append((
                self.require_unavailable({'select': [self.column(expression)]}, resources=[], **temporal),
                self.require_unavailable({'where': expression + ' == 0 || true', 'select': [self.column()]},
                                         resources=[], **temporal)))
        self.assertEqual(hidden_results[0], hidden_results[1])
        ids = ['temporal/rules', 'calendars/private']
        self.grant_providers(ids)
        choice = self.selections(ids)
        for value in [self.private_date(version='missing-v0'), self.private_date(year=100)]:
            expression = 'compareTemporal(' + json.dumps(value) + ',date("2026-01-01"))'
            self.require_unavailable({'select': [self.column(expression)]}, **choice)
            self.require_unavailable({'select': [self.column(expression)]}, resources=[], **choice)
        parameter_plan = {'select': [self.column('compareTemporal(day,date("2026-01-01"))')],
            'advanced': {'parameters': [{'name': 'day', 'shape': {'kind': 'use', 'name': 'CalendarDate'}}]}}
        self.require_unavailable(parameter_plan, resources=[],
            parameters={'day': self.native_record(self.private_date(version='missing-v0'))}, **choice)

    def test_provider_series_policy_is_separately_selected_and_granted(self):
        self.prepare_providers()
        ids = ['temporal/rules', 'calendars/gregorian', 'zones/utc']
        self.grant_providers(ids)
        choice = self.selections(ids)
        period = {'series_key': 'main', 'kind': 'day', 'calendar': 'gregorian',
            'calendar_version': 'proleptic-gregorian.v1', 'rule': 'iso8601', 'key': '2026-10-14',
            'zone': 'UTC', 'zone_version': 'fixed-utc.v1'}
        plan = {'select': [self.column('periodBounds(' + json.dumps(period) + ')', 'bounds')]}
        self.require_unavailable(plan, resources=[], **choice)
        self.grant_providers(ids, ['calendar/main'])
        catalog = self.call('query/catalog', actor='fields')
        self.assertEqual([p['id'] for p in catalog['seriesPolicies']], ['calendar/main'])
        self.assertNotIn('private-series', json.dumps(catalog))
        result = self.query(plan, **self.selections(ids, ['calendar/main']))['result']
        self.assertEqual(result['coverage'], 'complete')
        bounds = result['rows'][0]['cells']['bounds']
        self.assertEqual(bounds['state'], 'value')
        members = {item['key']: item['value'] for item in bounds['value']['value']}
        self.assertEqual(members['start'], {'kind': 'date', 'value': '2026-10-14'})
        self.assertEqual(members['end_exclusive'], {'kind': 'date', 'value': '2026-10-15'})

    def test_provider_frozen_office_bytes_and_revocation(self):
        self.prepare_providers()
        ids = ['temporal/rules', 'calendars/private']
        self.grant_providers(ids)
        self.policy(lambda users: [grant.update(office={'export': True, 'download': True})
                                  for grant in users['fields']['grants'].values()])
        choice = self.selections(ids)
        plan = {'select': [self.column('compareTemporal(' + json.dumps(self.private_date()) +
                                      ',date("2026-01-01"))', 'originalOrder')]}
        frozen = self.query(plan, **choice)
        request = {'outputLocale': 'zh-CN', 'requestId': uid(), 'format': 'xlsx', 'reviewMode': 'none',
                   'includeResolved': False, 'stylePolicy': 'template', 'bindings': {}, 'queryId': frozen['queryId']}
        output, _ = HostedOffice.output(self, request, 'fields')
        before = HostedOffice.deliver(self, output, 'fields')
        workbook_parts = '\n'.join(HostedOffice.parts(before).values())
        for hidden in ['epoch_date', 'PRIVATE_', str(self.root)]:
            self.assertNotIn(hidden, workbook_parts)
        # A raw external schema change is intentionally unadmitted. It must
        # invalidate new capture while already prepared bytes stay unchanged.
        schema_path = self.root / '.weftext-schema.json'
        schema = json.loads(schema_path.read_text('utf-8'))
        next(p for p in schema['providers'] if p['id'] == 'calendars/private')['data']['epoch_date'] = '2026-01-02'
        schema_path.write_text(json.dumps(schema, ensure_ascii=False), encoding='utf-8')
        self.query(plan, **choice, status=409)
        self.assertEqual(self.call('office/export/status', {'requestId': request['requestId']}, 'fields')['digest'], output['digest'])
        self.assertEqual(HostedOffice.deliver(self, output, 'fields'), before)
        self.grant_providers([])
        self.assertEqual(self.call('office/export/status', {'requestId': request['requestId']}, 'fields')['status'], 'expired')
        self.call('office/export/chunk', {'requestId': request['requestId'], 'offset': 0}, 'fields', 409)
        self.call('office/export/prepare', {**request, 'requestId': uid()}, 'fields', 410)
        self.grant_providers(ids)
        self.assertEqual(self.call('office/export/status', {'requestId': request['requestId']}, 'fields')['status'], 'expired')

    def test_provider_definition_prepare_and_save_rederive_minimal_authority(self):
        self.prepare_providers()
        self.policy(lambda users: [users['alice']['grants'][resource].update(
            fields={'tasks/status': {'read': True}},
            query={'definitions': True, 'definitionHistory': True})
            for resource in ['Shared', 'Other']])
        self.grant_providers(['temporal/rules'], actor='alice')
        doc = self.open()
        plan = {'select': [self.column('compareTemporal(' + json.dumps(self.private_date()) +
                                       ',date("2026-01-01"))', 'originalOrder')]}
        self.call('editor/map', {'sessionId': doc['sessionId'], 'source': doc['source'], 'serial': 1})
        denied = dict(resource='Shared', sessionId=doc['sessionId'], baseId=doc['baseId'],
                      source=doc['source'], serial=1, operationId=uid(),
                      label='weftext-query-provider-denied', plan=plan)
        denied_result = self.call('query/definition/prepare', denied, status=404)
        self.assertEqual(self.call('query/definition/operation', {
            'resource': 'Shared', 'operationId': denied['operationId']})['status'], 'notRegistered')
        missing = {**denied, 'operationId': uid(), 'plan': {'select': [self.column(
            'compareTemporal(' + json.dumps(self.private_date(calendar='never-installed')) +
            ',date("2026-01-01"))', 'originalOrder')]}}
        self.assertEqual(self.call('query/definition/prepare', missing, status=404), denied_result)
        self.assertEqual(self.call('query/definition/operation', {
            'resource': 'Shared', 'operationId': missing['operationId']})['status'], 'notRegistered')
        ids = ['temporal/rules', 'calendars/private']
        self.grant_providers(ids, actor='alice')
        doc = self.open()
        prepared, proposal = HostedDefinition.proposal(self, doc, plan=plan)
        self.assertEqual(prepared['status'], 'reserved')
        self.assertEqual(self.open()['source'], doc['source'])
        self.assertNotIn('providers', proposal)
        self.grant_providers(['temporal/rules'], actor='alice')
        current = self.open()
        HostedDefinition.definition_save(self, current, prepared['source'], [proposal['operationId']], status=404)
        self.assertEqual(self.open()['source'], doc['source'])
        self.grant_providers(ids, actor='alice')
        current = self.open()
        saved, _ = HostedDefinition.definition_save(self, current, prepared['source'], [proposal['operationId']])
        self.assertEqual(saved['status'], 'committed')
        call = {'advanced': {'call': {'owner': 'Shared', 'selection': {
            'mode': 'current', 'key': prepared['definition']['key']}}}}
        choice = self.selections(ids, actor='alice')
        result = self.query(call, actor='alice', **choice)['result']
        self.assertEqual(result['coverage'], 'complete')
        self.assertEqual(result['rows'][0]['cells']['originalOrder']['value']['value'], '0')
        self.query(call, actor='alice', status=400)

    def test_provider_revocation_fences_inflight_embedded_delivery(self):
        self.prepare_providers()
        plan = {'select': [self.column('body', 'body'),
                           self.column('nextCivilDay(date("2026-10-14"))', 'tomorrow')]}
        definition = self.add_definition(plan)
        self.grant_query(actor='alice', definitions=True, products=['body'])
        self.grant_providers(['temporal/rules'], actor='alice')
        choice = self.selections(['temporal/rules'], actor='alice')
        doc = self.open('Other')
        slow = doc['source'] + '\n#for i in range(4000) [delayed provider paragraph #parbreak()]'
        basis = {'sessionId': doc['sessionId'], 'source': slow, 'serial': 1}
        found = self.call('query/embedded/discover', basis)
        self.assertTrue(self.call('editor/render', basis)['ok'])
        occurrence = next(item for item in found['occurrences'] if item['key'] == definition['definition']['key'])
        deadline = time.monotonic() + 40
        changes = [(('alice', 'queryProviders'), {'providers': [], 'seriesPolicies': []})]
        policy_request = freeze_policy_changes(self.call('admin/policy', actor='admin'), changes)
        with ThreadPoolExecutor(max_workers=1) as pool:
            with OwnedWorkerObservation(self.server.owner) as observation:
                self.assertLess(time.monotonic(), deadline, 'Original request/mutation budget exhausted')
                submitted = event_time_ns()
                pending = pool.submit(timed_call, self.server.call, '/api/hosted/query/embedded/run', {**basis, 'occurrenceId': occurrence['id'], **choice}, self.tokens['alice'])
                # The whole input set is captured before its first worker.
                # This does not identify a particular row or all workers.
                worker = observation.wait(pending, deadline=deadline)
                observation.assert_inflight(pending)
                self.assertLess(time.monotonic(), deadline, 'Original request/mutation budget exhausted before change')
                mutation = apply_inflight_revocation(observation, pending, policy_request,
                    lambda update: self.call('admin/policy', update, 'admin'), deadline)
                (code, result, _), completed = pending.result(timeout=max(0, deadline-time.monotonic()))
        self.assertLess(time.monotonic(), deadline, 'Original budget exhausted before policy readback')
        readback = self.call('admin/policy', actor='admin')
        self.assertLess(time.monotonic(), deadline, 'Original budget exhausted during policy readback')
        trace = {'submittedMonotonicNs': submitted, 'worker': worker,
                 **mutation, 'requestCompletedMonotonicNs': completed,
                 'policyReadbackGeneration': readback['generation'],
                 'status': code, 'errorCode': result.get('error', {}).get('code'),
                 'explanationKey': result.get('error', {}).get('explanation', {}).get('key')}
        with (self.folder/'provider-revocation-inflight.json').open('x', encoding='utf-8') as evidence:
            evidence.write(json.dumps(trace, ensure_ascii=False, indent=2))
        verify_policy_changes(policy_request, [path for path, _ in changes], readback, trace)
        self.assertEqual(code, 401, result)
        self.assertEqual(result['error']['code'], 'authentication')
        self.assertEqual(result['error']['explanation']['key'],
                         'error.api.hosted.authenticationExpired')
        self.assertNotIn('queryId', result)

    def test_provider_named_parameter_shapes_do_not_probe_private_schema_definitions(self):
        self.prepare_providers()
        self.policy(lambda users: [users['alice']['grants'][resource].update(
            fields={'tasks/status': {'read': True}},
            query={'definitions': True, 'definitionHistory': True})
            for resource in ['Shared', 'Other']])
        doc = self.open()
        self.call('editor/map', {'sessionId': doc['sessionId'], 'source': doc['source'], 'serial': 1})
        results = []
        for name in ['PrivateDecision', 'NeverInstalledDecision']:
            plan = {'select': [self.column('wanted', 'choice')], 'advanced': {'parameters': [
                {'name': 'wanted', 'shape': {'kind': 'use', 'name': name}}]}}
            direct = self.query(plan, parameters={'wanted': {'kind': 'text', 'value': 'ONLY_INTERNAL_CHOICE'}}, status=400)
            empty = self.query(plan, resources=[], parameters={'wanted': {'kind': 'text', 'value': 'ONLY_INTERNAL_CHOICE'}}, status=400)
            operation = uid()
            prepare = self.call('query/definition/prepare', dict(resource='Shared', sessionId=doc['sessionId'],
                baseId=doc['baseId'], source=doc['source'], serial=1, operationId=operation,
                label='weftext-query-shape-probe', plan=plan), status=404)
            self.assertEqual(self.call('query/definition/operation', {
                'resource': 'Shared', 'operationId': operation})['status'], 'notRegistered')
            results.append((direct, empty, prepare))
        self.assertEqual(results[0], results[1])
        self.assertEqual(self.open()['source'], doc['source'])


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit('usage: test_hosted_providers.py EXE')
    suite = unittest.TestSuite(HostedProviders(name) for name in unittest.defaultTestLoader.getTestCaseNames(HostedProviders)
                               if name.startswith('test_provider_'))
    raise SystemExit(not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful())
