"""Bounded design oracle over actual schema paths; not a production provider/AST decoder."""
from pathlib import Path
import calendar, copy, datetime as dt, json, re
from fractions import Fraction

class Invalid(ValueError): pass
class Unavailable(ValueError): pass

class Model:
 def __init__(self,schema):self.s=schema;self.d=schema['definitions']
 def endpoint(self,x):
  if type(x) is dt.date:
   return self.endpoint(dict(calendar='gregorian',version='proleptic-gregorian.v1',precision='day',year=x.year,month=x.month,day=x.day))
  if isinstance(x,str):
   m=re.fullmatch(r'(\d{4})-(\d\d)-(\d\d)T(\d\d):(\d\d):(\d\d)(\.\d+)?(Z|[+-]\d\d:\d\d)',x)
   if not m or x.endswith('-00:00'):raise Invalid('instant syntax')
   y,mo,da,h,mi,se=map(int,m.group(1,2,3,4,5,6))
   if h>23 or mi>59 or se>59:raise Invalid('instant clock')
   try:ordinal=dt.date(y,mo,da).toordinal()
   except ValueError as e:raise Invalid('instant date') from e
   z=m[8];off=0
   if z!='Z':
    oh,om=map(int,z[1:].split(':'))
    if oh>23 or om>59:raise Invalid('offset')
    off=(oh*3600+om*60)*(1 if z[0]=='+' else -1)
   n=Fraction(ordinal*86400+h*3600+mi*60+se-off)+Fraction(m[7] or '0')
   return ('instant','UTC','exact','instant',n,n,n)
  if not isinstance(x,dict):raise Invalid('endpoint domain')
  c,v,g=x.get('calendar'),x.get('version'),x.get('precision')
  if g not in ['year','month','day'] or type(x.get('year')) is not int:raise Invalid('precision')
  if g=='year' and any(k in x for k in ['month','day','leap_month']):raise Invalid('year members')
  if g=='month' and 'day'in x:raise Invalid('month members')
  if g!='year' and type(x.get('month')) is not int:raise Invalid('month required')
  if g=='day' and type(x.get('day')) is not int:raise Invalid('day required')
  ps=[p for p in self.s['providers'] if p['family']=='calendar' and p['data'].get('key')==c and p['data'].get('version')==v]
  if len(ps)!=1:raise Unavailable('calendar provider')
  data=ps[0]['data'];y=x['year']
  if data['kind']=='gregorian':
   if not data['min_year']<=y<=data['max_year']:raise Unavailable('coverage')
   if x.get('leap_month',False):raise Invalid('Gregorian leap month')
   try:
    lo=dt.date(y,1,1).toordinal();hi=dt.date(y,12,31).toordinal();rank=y
    if g!='year':
     mo=x['month'];n=calendar.monthrange(y,mo)[1];lo=dt.date(y,mo,1).toordinal();hi=lo+n-1;rank=(y-1)*12+mo-1
    if g=='day':lo=hi=dt.date(y,mo,x['day']).toordinal();rank=lo
   except (ValueError,calendar.IllegalMonthError) as e:raise Invalid('calendar date') from e
  else:
   years=data['years'];ys=[a for a in years if a['year']==y]
   if not ys:raise Unavailable('coverage')
   before=[a for a in years if a['year']<y]
   lo=dt.date.fromisoformat(data['epoch_date']).toordinal()+sum(m['days'] for a in before for m in a['months'])
   months=ys[0]['months'];hi=lo+sum(m['days'] for m in months)-1;rank=y
   if g!='year':
    indices=[i for i,m in enumerate(months) if m['number']==x['month'] and m['leap']==x.get('leap_month',False)]
    if len(indices)!=1:raise Invalid('calendar month')
    i=indices[0];lo+=sum(m['days'] for m in months[:i]);hi=lo+months[i]['days']-1
    rank=sum(len(a['months']) for a in before)+i
   if g=='day':
    day=x['day']
    if not 1<=day<=months[i]['days']:raise Invalid('calendar day')
    lo=hi=lo+day-1;rank=lo
  return ('civil',c,v,g,rank,lo,hi)
 def relation(self,a,b,strict=False):
  a,b=self.endpoint(a),self.endpoint(b)
  if a[0]!=b[0]:raise Invalid('civil/instant mixed')
  same=a[1:4]==b[1:4]
  if same:return a[4]<b[4] if strict else a[4]<=b[4]
  if a[0]=='civil' and a[1:3]!=b[1:3] and (a[3]!='day' or b[3]!='day'):raise Unavailable('coarse mixed calendar')
  if strict:
   if a[6]<b[5]:return True
   if a[5]>=b[6]:return False
  else:
   if a[6]<=b[5]:return True
   if a[5]>b[6]:return False
  return None
 def compare(self,a,b):
  av,bv=self.endpoint(a),self.endpoint(b)
  if av[0]!=bv[0]:raise Invalid('civil/instant mixed')
  if av[1:4]==bv[1:4]:return (av[4]>bv[4])-(av[4]<bv[4])
  if av[0]=='civil' and av[1:3]!=bv[1:3] and (av[3]!='day' or bv[3]!='day'):raise Unavailable('coarse mixed calendar')
  if av[6]<bv[5]:return -1
  if av[5]>bv[6]:return 1
  if av[3]==bv[3]=='day' and av[5]==bv[5]:return 0
  raise Unavailable('overlapping precision buckets')
 def interval(self,x,during=False):
  if not isinstance(x,dict):raise Invalid('interval')
  keys=['start','end'] if during else ['start','end_exclusive']
  bounds=[]
  for k in keys:
   if k in x:
    if x[k] is None:
     if not during:raise Invalid('Range none')
    else:bounds.append(x[k]);self.endpoint(x[k])
  if not during and not bounds:raise Invalid('Range no bound')
  if len(bounds)==2:
   ordered=self.relation(bounds[0],bounds[1],strict=not during)
   if ordered is False:raise Invalid('interval order')
   if ordered is None:raise Unavailable('unproved interval order')
  return True
 @staticmethod
 def conjunction(a,b):
  if a is False or b is False:return False
  if a is True and b is True:return True
  raise Unavailable('boundary precision')
 def contains(self,x,p):
  self.endpoint(p);self.interval(x,True)
  if not all(k in x for k in ['start','end']):raise Unavailable('unknown endpoint')
  a=True if x['start'] is None else self.relation(x['start'],p)
  b=True if x['end'] is None else self.relation(p,x['end'])
  return self.conjunction(a,b)
 def overlap(self,a,b):
  self.interval(a);self.interval(b)
  v=True if 'start' not in a or 'end_exclusive' not in b else self.relation(a['start'],b['end_exclusive'],True)
  w=True if 'start' not in b or 'end_exclusive' not in a else self.relation(b['start'],a['end_exclusive'],True)
  return self.conjunction(v,w)
 def duration(self,x,during=False):
  self.interval(x,during);end='end' if during else 'end_exclusive'
  if any(k not in x or x[k] is None for k in ['start',end]):raise Unavailable('unbounded/unknown duration')
  a,b=self.endpoint(x['start']),self.endpoint(x[end])
  if a[0]=='instant':return {'value':b[4]-a[4],'unit':'seconds'}
  if a[1:4]!=b[1:4]:
   if a[3]!='day' or b[3]!='day':raise Unavailable('mixed duration')
   return {'value':b[5]-a[5]+int(during),'unit':'days','calendar':'gregorian','version':'proleptic-gregorian.v1'}
  return {'value':b[4]-a[4]+int(during),'unit':{'year':'years','month':'months','day':'days'}[a[3]],'calendar':a[1],'version':a[2]}
 def exact_event(self,x):
  self.interval(x)
  if not all(k in x for k in ['start','end_exclusive']):raise Invalid('recurrence bounded template')
  if any(isinstance(x[k],dict) for k in ['start','end_exclusive']):raise Invalid('recurrence exact template')
  return True
 def decode(self,shape,value):
  """Small schema-driven model, not official native decoding or all scalar codecs."""
  k=shape['kind']
  if k=='use':return self.decode(self.d[shape['name']],value)
  if k=='scalar':
   n=shape['semantic']
   if n=='date':
    if type(value) is not dt.date:raise Invalid('not native date')
   elif n=='instant':
    if not isinstance(value,str):raise Invalid('not instant')
    self.endpoint(value)
   elif n=='int':
    if type(value) is not int:raise Invalid('not int')
   elif n=='bool':
    if type(value) is not bool:raise Invalid('not bool')
   else:
    if not isinstance(value,str) or (n=='nonempty_text' and not value):raise Invalid('text')
   return value
  if k=='enum':
   if not isinstance(value,str) or (not shape['open'] and value not in shape['values']):raise Invalid('preset code')
   return value
  if k=='list':
   if not isinstance(value,list):raise Invalid('not list')
   return [self.decode(shape['items'],v) for v in value]
  if k=='union':
   disc=shape['discriminator']
   if disc and disc['kind']=='member':
    present=[i for i,key in enumerate(disc['keys']) if isinstance(value,dict) and key in value]
    if len(present)!=1:raise Invalid('branch conflict/missing')
    return self.decode(shape['variants'][present[0]],value)
   good=[]
   for v in shape['variants']:
    try:good.append(self.decode(v,value))
    except Invalid:pass
   if len(good)!=1:raise Invalid('native union domain')
   return good[0]
  if k!='record' or not isinstance(value,dict):raise Invalid('not record')
  if shape['unknown']=='reject' and set(value)-set(shape['members']):raise Invalid('unknown branch member')
  for key,m in shape['members'].items():
   if key not in value:
    if m['required']:raise Invalid('required '+key)
   elif value[key] is None:
    if not m['nullable']:raise Invalid('null '+key)
   else:self.decode(m['shape'],value[key])
  for c in shape['constraints']:
   v=c['validator']
   if v=='temporal/calendar-date':self.endpoint(value)
   elif v=='temporal/during':self.interval(value,True)
   elif v=='temporal/range':self.interval(value)
  return value
 def field(self,key,value):
  f=next(f for f in self.s['fields'] if f['key']==key)
  return self.decode(f['shape'],value)

def run(schema):
 m=Model(schema);results=[]
 def check(name,fn,expected):
  try:actual=fn()
  except Invalid:actual='invalid'
  except Unavailable:actual='unavailable'
  assert actual==expected,(name,actual,expected)
  results.append({'case':name,'expected':expected,'actual':actual,'passed':True})
 def c(y,mo=None,day=None,cal='gregorian',version='proleptic-gregorian.v1'):
  x=dict(calendar=cal,version=version,precision='day' if day else 'month' if mo else 'year',year=y)
  if mo:x['month']=mo
  if day:x['day']=day
  return x
 month=dict(start=c(2020,3),end=None);year=dict(start=c(2019),end=c(2021))
 check('actual-engagement-month-start-ongoing',lambda:m.field('engagements',[{'employer':{'text':'研究所'},'during':month}])==[{'employer':{'text':'研究所'},'during':month}],True)
 check('actual-state-year-history',lambda:m.field('nationality_states',[{'value':'声明值','during':year}]) is not None,True)
 check('actual-task-start-year',lambda:m.field('start',c(2020))==c(2020),True)
 check('actual-task-due-month',lambda:m.field('due',c(2020,3))==c(2020,3),True)
 check('completed-at-stays-exact',lambda:m.field('completed_at',c(2020,3)),'invalid')
 check('proof-bucket-does-not-fill-author-day',lambda:m.contains(month,dt.date(2020,4,1)) and 'day' not in month['start'],True)
 check('instant-duration-preserves-nanosecond',lambda:m.duration(dict(start='2020-03-01T00:00:00Z',end_exclusive='2020-03-01T00:00:00.000000001Z'))['value']==Fraction(1,1000000000),True)
 check('compare-year-usual-order',lambda:m.compare(c(2020),c(2021)),-1)
 check('compare-month-usual-order',lambda:m.compare(c(2020,3),c(2020,4)),-1)
 check('compare-native-date-compatible-record-day',lambda:m.compare(dt.date(2020,3,1),c(2020,3,1)),0)
 check('compare-mixed-precision-disjoint',lambda:m.compare(c(2020),c(2021,3)),-1)
 check('compare-mixed-precision-overlapping',lambda:m.compare(c(2020),c(2020,3)),'unavailable')
 check('during-same-month-contained',lambda:m.contains(month,c(2020,3)),True)
 check('during-full-day-inside-later-month',lambda:m.contains(month,dt.date(2020,4,1)),True)
 check('during-full-day-near-unknown-start',lambda:m.contains(month,dt.date(2020,3,15)),'unavailable')
 check('during-full-day-definitely-before',lambda:m.contains(month,dt.date(2020,2,29)),False)
 check('during-year-closed-duration',lambda:m.duration(year,True)['value'],3)
 check('during-month-closed-duration',lambda:m.duration(dict(start=c(2020,3),end=c(2020,5)),True)['value'],3)
 for grain,a,b,n in [('year',c(2020),c(2022),2),('month',c(2020,3),c(2020,5),2)]:
  x=dict(start=a,end_exclusive=b)
  check('actual-range-'+grain,lambda x=x:m.field('range',x)==x,True)
  check('range-'+grain+'-duration',lambda x=x:m.duration(x)['value'],n)
  check('range-'+grain+'-adjacent-no-overlap',lambda x=x,b=b:m.overlap(x,dict(start=b)),False)
 check('range-month-day-interior-overlap',lambda:m.overlap(dict(start=c(2020,3),end_exclusive=c(2020,5)),dict(start=dt.date(2020,4,1),end_exclusive=dt.date(2020,4,2))),True)
 check('range-month-boundary-overlap-unproved',lambda:m.overlap(dict(start=c(2020,3),end_exclusive=c(2020,5)),dict(start=dt.date(2020,3,15),end_exclusive=dt.date(2020,3,16))),'unavailable')
 check('range-month-far-after-no-overlap',lambda:m.overlap(dict(start=c(2020,3),end_exclusive=c(2020,5)),dict(start=dt.date(2020,6,1),end_exclusive=dt.date(2020,6,2))),False)
 check('range-month-reversed',lambda:m.field('range',dict(start=c(2020,5),end_exclusive=c(2020,3))),'invalid')
 check('range-month-equal-empty',lambda:m.field('range',dict(start=c(2020,3),end_exclusive=c(2020,3))),'invalid')
 check('range-mixed-valid-disjoint-years',lambda:m.field('range',dict(start=c(2020),end_exclusive=c(2021,3))) is not None,True)
 check('range-mixed-order-unproved',lambda:m.field('range',dict(start=c(2020),end_exclusive=c(2020,3))),'unavailable')
 check('mixed-precision-duration-unavailable',lambda:m.duration(dict(start=c(2020),end_exclusive=c(2021,3))),'unavailable')
 check('during-absent-end-unknown',lambda:m.contains(dict(start=c(2020,3)),c(2021,1)),'unavailable')
 check('during-explicit-unbounded-year-point',lambda:m.contains(dict(start=None,end=None),c(2020)),True)
 check('range-none-still-invalid',lambda:m.field('range',dict(start=c(2020,3),end_exclusive=None)),'invalid')
 check('native-Gregorian-date-and-record-day',lambda:m.duration(dict(start=dt.date(2020,3,1),end_exclusive=c(2020,3,2)))['value'],1)
 check('civil-instant-remains-invalid',lambda:m.field('range',dict(start=c(2020,3),end_exclusive='2021-01-01T00:00:00Z')),'invalid')
 check('missing-calendar-provider-in-during',lambda:m.field('engagements',[{'employer':{'text':'X'},'during':dict(start=c(2020,3,cal='missing'),end=None)}]),'unavailable')
 non=c(1,1,cal='test-calendar',version='test-calendar.v1')
 check('different-calendar-full-day-same-ordinal',lambda:m.compare(c(1,1,1,cal='test-calendar',version='test-calendar.v1'),dt.date(2026,1,1)),0)
 check('actual-non-Gregorian-month-range',lambda:m.field('range',dict(start=non,end_exclusive=c(1,2,cal='test-calendar',version='test-calendar.v1'))) is not None,True)
 check('non-Gregorian-month-duration-unit',lambda:m.duration(dict(start=non,end_exclusive=c(1,2,cal='test-calendar',version='test-calendar.v1')))['unit'],'months')
 check('mixed-calendar-coarse-unavailable',lambda:m.interval(dict(start=non,end_exclusive=c(2026,3))),'unavailable')
 check('recurrence-month-anchor-invalid',lambda:m.exact_event(dict(start=c(2020,3),end_exclusive=c(2020,4))),'invalid')
 check('recurrence-exact-native-date-valid',lambda:m.exact_event(dict(start=dt.date(2020,3,1),end_exclusive=dt.date(2020,3,2))),True)
 check('replacement-partial-range-rejected-by-actual-shape',lambda:m.decode(m.d['Exception'],dict(onset=dt.date(2020,3,1),action='replace',range=dict(start=c(2020,3),end_exclusive=c(2020,4)))),'invalid')
 # Assert the actual field mapping, not merely isolated descriptor enum checks.
 for field,codes in [('family_relations',['extended-relative']),('social_relations',['acquaintance','classmate','friend']),('professional_relations',['colleague'])]:
  for code in codes:
   row={'target':{'text':'甲'},'relationship':{'preset':code}}
   check(field+'-preset-'+code,lambda field=field,row=row:m.field(field,[row])==[row],True)
  custom={'target':{'text':'甲'},'relationship':{'custom':codes[0]}}
  check(field+'-same-text-stays-custom',lambda field=field,custom=custom:m.field(field,[custom])[0]['relationship']=={'custom':custom['relationship']['custom']},True)
  both={'target':{'text':'甲'},'relationship':{'preset':codes[0],'custom':codes[0]}}
  check(field+'-two-branches-invalid',lambda field=field,both=both:m.field(field,[both]),'invalid')
  check(field+'-classification-required',lambda field=field:m.field(field,[{'target':{'text':'甲'}}]),'invalid')
  check(field+'-empty-custom-invalid',lambda field=field:m.field(field,[{'target':{'text':'甲'},'relationship':{'custom':''}}]),'invalid')
 check('family-cross-domain-friend-invalid',lambda:m.field('family_relations',[{'target':{'text':'甲'},'relationship':{'preset':'friend'}}]),'invalid')
 check('social-cross-domain-colleague-invalid',lambda:m.field('social_relations',[{'target':{'text':'甲'},'relationship':{'preset':'colleague'}}]),'invalid')
 check('professional-cross-domain-friend-invalid',lambda:m.field('professional_relations',[{'target':{'text':'甲'},'relationship':{'preset':'friend'}}]),'invalid')
 check('social-unknown-preset-invalid',lambda:m.field('social_relations',[{'target':{'text':'甲'},'relationship':{'preset':'new-friend'}}]),'invalid')
 check('legacy-role-customLabel-not-classification',lambda:m.field('social_relations',[{'target':{'text':'甲'},'role':'friend','customLabel':'friend'}]),'invalid')
 for key,value in [('engagements',[{'employer':{'text':'X'},'during':month}]),('social_relations',[{'target':{'text':'甲'},'relationship':{'custom':' friend '}}])]:
  transported=json.loads(json.dumps(value,ensure_ascii=False))
  check(key+'-exact-structure-roundtrip',lambda key=key,transported=transported,value=value:m.field(key,transported)==value,True)
 return results

if __name__=='__main__':
 root=Path(__file__).parent;s=json.loads((root/'schema.json').read_text(encoding='utf-8'));results=run(s)
 (root/'precision-relation-checks.json').write_text(json.dumps({'evidence':'Bounded Python design model using actual During/Range and person relation schema paths; not official AST parsing, Rust providers/CEL, Core authorization, forms or DOCX/XLSX reimport. Exact JSON roundtrips prove model transport only.','results':results},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
 print(json.dumps({'design_model_checks':len(results),'passed':True}))
