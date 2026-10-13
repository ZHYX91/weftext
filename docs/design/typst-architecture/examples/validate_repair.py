"""Bounded design checks; not the Rust schema installer or Typst AST producer."""
from pathlib import Path
import copy, json, re, calendar, datetime, subprocess
ROOT=Path(__file__).parent
schema_file=ROOT/'schema.json'
def strict_json(text):
 def pairs(items):
  out={}
  for k,v in items:
   assert k not in out,'duplicate JSON key'
   out[k]=v
  return out
 return json.loads(text,object_pairs_hook=pairs)
schema=strict_json(schema_file.read_text(encoding='utf-8'))
abi=json.loads((ROOT/'builtin-abi.design.json').read_text(encoding='utf-8'))
CODECS=set('text nonempty_text phone email identifier doi url language_tag citation_key node_ref resource_ref tag_ref schema_ref config_revision bool int float decimal exact_decimal duration exact_seconds date time local_datetime instant'.split())
BUILTINS={'weftext.core.rules.v1':1,'weftext.temporal.rules.v2':2,'weftext.units.rules.v1':1,'weftext.calendar.gregorian.v2':2,'weftext.calendar.table.v2':2,'weftext.zone.table.v1':1}
COUNTS=[]
def exact(x,keys):
 assert isinstance(x,dict) and set(x)==set(keys),f'closed members {set(x)} != {set(keys)}'
def unique(items):assert len(items)==len(set(items))
def install(s):
 exact(s,['version','definitions','fields','types','aliases','relations','validators','providers','series_policies']);assert s['version']==1
 fields={x['id']:x for x in s['fields']};types={x['id']:x for x in s['types']};validators={x['id']:x for x in s['validators']};providers={x['id']:x for x in s['providers']}
 unique([x['id'] for x in s['fields']]);unique([x['key'] for x in s['fields']]);unique([x['id'] for x in s['types']]);unique([x['id'] for x in s['validators']]);unique([x['id'] for x in s['providers']]);unique([x['id'] for x in s['relations']]);unique([x['id'] for x in s['series_policies']]);unique([x['key'] for x in s['series_policies']])
 def deref(x,stack=()):
  if x['kind']=='use':
   name=x['name'];assert name in s['definitions'] and name not in stack;return deref(s['definitions'][name],stack+(name,))
  return x
 def path(x,steps):
  candidates=[deref(x)]
  for step in steps:
   new=[]
   for z in candidates:
    variants=z['variants'] if z['kind']=='union' else [z]
    for z in variants:
     z=deref(z)
     if step['kind']=='each':
      exact(step,['kind']);assert z['kind']=='list';new.append(deref(z['items']))
     else:
      exact(step,['kind','key']);assert z['kind']=='record'
      if step['key'] in z['members']:new.append(deref(z['members'][step['key']]['shape']))
   assert new,'path has no schema branch';candidates=new
  return candidates
 def constraints(cs,shape=None,type_id=None):
  for c in cs:
   exact(c,['validator','inputs','parameters']);assert c['validator'] in validators
   v=validators[c['validator']];assert set(c['inputs'])==set(v['inputs'])
   parameter_members=deref(v['parameters'])['members'];assert set(c['parameters'])==set(parameter_members)
   for selector in c['inputs'].values():
    if selector['kind']=='local':exact(selector,['kind','path']);assert shape is not None;path(shape,selector['path'])
    elif selector['kind']=='field':
     exact(selector,['kind','id','path']);assert type_id in types
     def owned_fields(id,stack=()):
      assert id not in stack
      return {x['id'] for x in types[id]['fields']}|set().union(*(owned_fields(z,stack+(id,)) for z in types[id]['requires']))
     assert selector['id'] in owned_fields(type_id);path(fields[selector['id']]['shape'],selector['path'])
    else:exact(selector,['kind','name']);assert selector['kind']=='context' and selector['name'] in ['owner','resource_metadata']
 def shape(x,stack=()):
  k=x['kind']
  if k=='scalar':exact(x,['kind','semantic']);assert x['semantic'] in CODECS
  elif k=='enum':exact(x,['kind','values','open']);assert isinstance(x['open'],bool);unique(x['values']);assert all(isinstance(y,str) for y in x['values'])
  elif k=='use':
   exact(x,['kind','name']);assert x['name'] in s['definitions'] and x['name'] not in stack;shape(s['definitions'][x['name']],stack+(x['name'],))
  elif k=='list':exact(x,['kind','items','min','max']);assert x['min']>=0 and(x['max'] is None or x['max']>=x['min']);shape(x['items'],stack)
  elif k=='record':
   exact(x,['kind','members','unknown','constraints']);assert x['unknown'] in ['preserve','reject']
   for m in x['members'].values():exact(m,['shape','required','nullable']);assert isinstance(m['required'],bool) and isinstance(m['nullable'],bool);shape(m['shape'],stack)
   constraints(x['constraints'],shape=x)
  elif k=='union':
   exact(x,['kind','variants','discriminator']);assert len(x['variants'])>=2
   for y in x['variants']:shape(y,stack)
   d=x['discriminator']
   if d is not None:
    if d['kind']=='member':exact(d,['kind','keys']);assert len(d['keys'])==len(x['variants']);unique(d['keys'])
    elif d['kind']=='tag':exact(d,['kind','key'])
    else:exact(d,['kind','key','present','absent']);assert d['kind']=='record_presence' and d['present']!=d['absent'];assert all(deref(x['variants'][d[y]])['kind']=='record' for y in ['present','absent'])
  else:raise AssertionError('unknown shape')
 for p in providers.values():
  exact(p,['id','family','implementation','data','exports','limits']);exact(p['implementation'],['kind','id','abi']);assert p['implementation']['kind']=='builtin' and p['implementation']['id'] in BUILTINS and p['implementation']['abi']==BUILTINS[p['implementation']['id']]
  assert p['exports']==abi['implementations'][p['implementation']['id']]['exports']
  exact(p['limits'],['max_steps','max_bytes','max_expansion']);assert all(isinstance(x,int) and x>0 for x in p['limits'].values());unique([x['name'] for x in p['exports']])
  for e in p['exports']:exact(e,['name','inputs','result','effect','query_name']);assert e['effect']=='pure';assert e['query_name'] is None or e['query_name'] in ['compareTemporal','duringDuration','containsDate','nextCivilDay','rangeOverlap','rangeDuration','periodBounds','expandRecurrence']
  d=p['data'];kind=d['kind']
  if kind=='none':exact(d,['kind'])
  elif kind=='unit-table':
   exact(d,['kind','version','units']);unique([u['key'] for u in d['units']]);
   for u in d['units']:exact(u,['key','dimension']);assert u['dimension'] in ['length','mass']
  elif kind=='gregorian':exact(d,['kind','key','version','min_year','max_year']);assert 1<=d['min_year']<=d['max_year']<=9999
  elif kind=='calendar-table':
   exact(d,['kind','key','version','epoch_date','years']);assert d['years'];ys=[y['year'] for y in d['years']];assert ys==list(range(min(ys),max(ys)+1));datetime.date.fromisoformat(d['epoch_date'])
   for y in d['years']:
    exact(y,['year','months']);unique([(m['number'],m['leap']) for m in y['months']]);assert y['months']
    for m in y['months']:exact(m,['number','leap','days']);assert m['days']>0
  elif kind=='zone-table':
   exact(d,['kind','key','version','from','until','initial_offset_seconds','transitions']);assert d['from']<d['until'];ats=[t['at'] for t in d['transitions']];assert ats==sorted(set(ats))
   for t in d['transitions']:exact(t,['at','offset_seconds']);assert d['from']<=t['at']<d['until'] and abs(t['offset_seconds'])<=86400
  else:raise AssertionError('unknown provider data family')
 for v in validators.values():
  exact(v,['id','provider','operation','inputs','parameters','result']);assert v['provider'] in providers;assert v['operation'] in [e['name'] for e in providers[v['provider']]['exports']];assert v['result']=='valid-invalid-unavailable';shape(v['parameters'])
  assert {k:v[k] for k in ['provider','operation','inputs','parameters','result']}==abi['validators'][v['id']]
 unique([e['query_name'] for p in providers.values() for e in p['exports'] if e['query_name'] is not None])
 for d in s['definitions'].values():shape(d)
 for f in fields.values():
  exact(f,['id','key','shape','nullable','default']);assert f['key'] not in ['id','types'] and not f['key'].startswith('weftext-');shape(f['shape']);assert f['default']=={'kind':'absent'}
 for t in types.values():
  exact(t,['id','fields','requires','conflicts','constraints']);unique([f['id'] for f in t['fields']]);assert all(f['id'] in fields for f in t['fields']);assert all(id in types for id in t['requires']+t['conflicts'])
  constraints(t['constraints'],type_id=t['id'])
 def closure(id,stack=()):
  assert id not in stack
  return {id}|set().union(*(closure(z,stack+(id,)) for z in types[id]['requires']))
 for id in types:closure(id)
 assert all(id in types for id in s['aliases'].values())
 for r in s['relations']:
  exact(r,['id','source_type','field','target_path','target_domain','target_types','direction','inverse','max_assertions','reject_self','graph_projection','cycle_policy','resolution','delete'])
  assert r['source_type'] in types and r['field'] in {f['id'] for f in types[r['source_type']]['fields']};assert r['target_domain']=='node_ref';assert all(x=={'kind':'scalar','semantic':'node_ref'} for x in path(fields[r['field']]['shape'],r['target_path']))
  assert r['direction'] in ['directed','symmetric'] and r['graph_projection'] in ['general','ranked','family'] and r['cycle_policy'] in ['allow','reject'];assert r['direction']!='symmetric' or r['inverse'] is None
  z=r['target_types'];assert z=={'kind':'ordinary'} or(z['kind']=='any_of' and all(id in types for id in z['ids']))
 for p in s['series_policies']:
  exact(p,['id','key','calendar','zone','kinds','rule','scope','multiplicity']);assert p['calendar'] in providers and providers[p['calendar']]['family']=='calendar';assert p['zone'] in providers and providers[p['zone']]['family']=='time-zone';assert p['multiplicity'] in ['unique','many']
  expected_rule='iso8601' if providers[p['calendar']]['data']['kind']=='gregorian' else 'table-ordinal-v1'
  assert p['rule']==expected_rule
  assert p['kinds'] and set(p['kinds'])<=set(['day','week','month','quarter','year']);unique(p['kinds'])
  if p['scope']['kind']=='library':exact(p['scope'],['kind'])
  else:exact(p['scope'],['kind','root']);assert p['scope']['kind']=='node_subtree' and re.fullmatch(r'weftext-node://[0-9a-f-]{36}/[0-9a-f-]{36}',p['scope']['root'])
 return True

results=[]
def test(name,fn,expect=True):
 try: actual=bool(fn())
 except (AssertionError,KeyError,ValueError,TypeError,StopIteration):actual=False
 assert actual==expect,(name,actual,expect)
 results.append({'case':name,'expected':expect,'actual':actual,'passed':True})
test('complete-schema-closes-and-paths-resolve',lambda:install(schema))
for name,mutate in [
 ('unknown-top-member',lambda s:s.update(extra=True)),
 ('unregistered-validator',lambda s:s['definitions']['Range']['constraints'][0].update(validator='future/rule')),
 ('unknown-provider-implementation',lambda s:s['providers'][0]['implementation'].update(id='author/script')),
 ('forged-provider-export-signature',lambda s:s['providers'][0]['exports'][0].update(result='text')),
 ('forged-validator-input-signature',lambda s:s['validators'][0]['inputs'].update(value='text')),
 ('relation-path-wrong-key',lambda s:s['relations'][0].update(target_path=[{'kind':'each'},{'kind':'member','key':'secret'}])),
 ('query-binding-unregistered-codec',lambda s:s['definitions']['QueryBinding']['members']['schema']['shape'].update(semantic='guess_config')),
 ('missing-event-type',lambda s:s['types'].remove(next(t for t in s['types'] if t['id']=='calendar/event'))),
 ('constraint-read-missing-member',lambda s:s['definitions']['During']['constraints'][0]['inputs']['value'].update(path=[{'kind':'member','key':'end_exclusive'}])),
 ('unknown-rule-data-family',lambda s:s['providers'][3]['data'].update(kind='eval-calendar')),
 ('unregistered-series-rule',lambda s:s['series_policies'][0].update(rule='author/script')),
]:
 x=copy.deepcopy(schema);mutate(x);test(name,lambda x=x:install(x),False)

def range_ok(x):
 vals=[x.get(k) for k in ['start','end_exclusive'] if k in x];assert vals
 assert all(isinstance(y,str) for y in vals)
 domain=lambda y:'instant' if 'T'in y else 'date'
 assert len({domain(y) for y in vals})==1
 if len(vals)==2:assert vals[0]<vals[1]
 return True
test('open-range',lambda:range_ok({'start':'2026-10-12'}))
test('closed-half-open-range',lambda:range_ok({'start':'2026-10-12','end_exclusive':'2026-10-13'}))
test('empty-unbounded-range',lambda:range_ok({}),False)
test('equal-range-ends',lambda:range_ok({'start':'2026-10-12','end_exclusive':'2026-10-12'}),False)
test('mixed-date-instant-range',lambda:range_ok({'start':'2026-10-12','end_exclusive':'2026-10-13T00:00:00Z'}),False)
def calendar_ok(x):
 d=next(p['data'] for p in schema['providers'] if p['data'].get('key')==x['calendar']);assert x['version']==d['version']
 if d['kind']=='calendar-table':
  y=next(y for y in d['years'] if y['year']==x['year']);m=next(m for m in y['months'] if m['number']==x.get('month') and m['leap']==x.get('leap_month',False)) if x['precision']!='year' else None
  if x['precision']=='day':assert 1<=x['day']<=m['days']
 else:
  assert 1<=x['year']<=9999
  if x['precision']=='day':datetime.date(x['year'],x['month'],x['day'])
 if x['precision']=='year':assert 'month'not in x and 'day'not in x
 elif x['precision']=='month':assert 'month'in x and 'day'not in x
 elif x['precision']=='day':assert 'month'in x and 'day'in x
 else:raise AssertionError('unknown precision')
 return True
test('non-gregorian-finite-pack-day',lambda:calendar_ok({'calendar':'test-calendar','version':'test-calendar.v1','precision':'day','year':1,'month':2,'day':30}))
test('non-gregorian-outside-pack-range',lambda:calendar_ok({'calendar':'test-calendar','version':'test-calendar.v1','precision':'day','year':2,'month':1,'day':1}),False)
test('year-precision-does-not-fill-month',lambda:calendar_ok({'calendar':'gregorian','version':'proleptic-gregorian.v1','precision':'year','year':2026}))
test('year-precision-rejects-extra-known-month',lambda:calendar_ok({'calendar':'gregorian','version':'proleptic-gregorian.v1','precision':'year','year':2026,'month':10}),False)
baseline=copy.deepcopy(schema);current=copy.deepcopy(schema);current['aliases']['person']='organizations/organization'
binding={'schema':'weftext-config://2fd54310-65f7-4d3c-8668-2f1e344a56c0/schema','revision':'6d446e07-5bfd-47c8-8f1f-307df7ee6a0f','language':'weftext.query.cel.v1'}
archive={(binding['schema'],binding['revision']):baseline}
def expanded_shape(s,x):
 if isinstance(x,list):return [expanded_shape(s,y) for y in x]
 if not isinstance(x,dict):return x
 if x.get('kind')=='use':return expanded_shape(s,s['definitions'][x['name']])
 return {k:expanded_shape(s,v) for k,v in x.items()}
def resolve_bound_symbols(binding,archive,field_symbols,type_symbols,current):
 # The declared symbol lists stand in for an independently parsed CEL tree;
 # this checks only persistent schema identity resolution, not the CEL parser.
 assert binding['language']=='weftext.query.cel.v1'
 base=archive[(binding['schema'],binding['revision'])]
 fields={x['key']:x for x in base['fields']};now={x['id']:x for x in current['fields']}
 out={'fields':{},'types':{}}
 for key in field_symbols:
  f=fields[key];n=now[f['id']];assert expanded_shape(base,f['shape'])==expanded_shape(current,n['shape']) and f['nullable']==n['nullable'];out['fields'][key]=f['id']
 for name in type_symbols:
  id=base['aliases'].get(name,name);assert id in {t['id'] for t in base['types']} and id in {t['id'] for t in current['types']};out['types'][name]=id
 return out
test('cold-query-shortname-uses-persisted-baseline',lambda:resolve_bound_symbols(binding,archive,['phones'],['person'],current)=={'fields':{'phones':'contacts/phones'},'types':{'person':'people/person'}})
test('missing-baseline-cannot-fallback-current',lambda:resolve_bound_symbols(binding,{},['phones'],['person'],current),False)
changed=copy.deepcopy(current);next(f for f in changed['fields'] if f['id']=='contacts/phones')['shape']={'kind':'scalar','semantic':'text'}
test('incompatible-current-field-not-namesake',lambda:resolve_bound_symbols(binding,archive,['phones'],['person'],changed),False)
changed_member=copy.deepcopy(current);changed_member['definitions']['Phone']['members']['number']['shape']={'kind':'scalar','semantic':'int'}
test('incompatible-nested-definition-not-same-use-name',lambda:resolve_bound_symbols(binding,archive,['phones'],['person'],changed_member),False)
test('duplicate-json-member-rejected',lambda:strict_json('{"version":1,"version":1}'),False)
def decision(original,exdates,exceptions):
 xs=[x for x in exceptions if x['onset']==original]
 assert len(xs)<=1
 if xs:return None if xs[0]['action']=='cancel' else xs[0]['replacement']
 return None if original in exdates else original
test('replacement-wins-original-exdate',lambda:decision('2026-10-12',{'2026-10-12'},[{'onset':'2026-10-12','action':'replace','replacement':'2026-10-14'}])=='2026-10-14')
test('duplicate-cancel-replace-invalid',lambda:decision('2026-10-12',set(),[{'onset':'2026-10-12','action':'cancel'},{'onset':'2026-10-12','action':'replace','replacement':'2026-10-14'}]),False)
test('nonmatching-exdate-retains-existing-template',lambda:decision('2026-10-12',{'2026-10-13'},[])=='2026-10-12')
test('same-instant-different-offset-key',lambda:datetime.datetime.fromisoformat('2026-10-12T08:00:00+08:00')==datetime.datetime.fromisoformat('2026-10-12T00:00:00+00:00'))
test('copy-retains-exact-baseline-contract',lambda:resolve_bound_symbols(copy.deepcopy(binding),archive,['phones'],['person'],current)['types']['person']=='people/person')
COUNTS={'fields':len(schema['fields']),'types':len(schema['types']),'definitions':len(schema['definitions']),'relations':len(schema['relations']),'validators':len(schema['validators']),'providers':len(schema['providers'])}
(ROOT/'design-checks.json').write_text(json.dumps({'evidence':'Bounded Python design checks only; not product installer, official AST role authentication, Rust provider/CEL implementation, permissions or transactions. Query checks exercise schema identity resolution from caller-supplied symbol lists, not CEL parsing.','schema_counts':COUNTS,'results':results},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'passed':len(results),'counts':COUNTS}))
