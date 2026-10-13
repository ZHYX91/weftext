import json
from pathlib import Path
b=Path(__file__).resolve().parent/'results'
r=json.loads((b/'results.json').read_text()); report={}
def tree(c,m='on'):return json.loads((b/f'{c}.{m}.final-tree.json').read_text())
def nodes(n):
 yield n
 for c in n.get('children',[]):yield from nodes(c)
def body(n):return next(v for v in nodes(n) if v.get('tag')=='body')
def styles(n,anc=()):
 if n['kind']=='text':yield n['text'],anc[-1]['css'] if anc else ''
 elif n['kind']=='element':
  for c in n['children']:yield from styles(c,anc+(n,))
def css(c,t):return next(s for text,s in styles(body(tree(c))) if text==t)
def semantic(n,skip_marker=False):
 # Test-only projection. Drop only the marker's explicit producer ID. Actual
 # consumers always retain every final child path; no span/string dedup.
 if n['kind']=='tag-start' or n['kind']=='tag-end':return []
 if n['kind']=='text':return [('text',n['text'])]
 if n['kind']=='frame':return [('frame',n['frame'])]
 if skip_marker and dict(n.get('attrs',[])).get('data-probe-marker')=='true':return []
 children=[v for c in n.get('children',[]) for v in semantic(c,skip_marker)]
 if n.get('tag')=='span' and dict(n.get('attrs',[])).get('data-probe-text')=='true':return children
 attrs=[(k,v) for k,v in n.get('attrs',[]) if k!='style']
 return [('open',n.get('tag'),attrs)]+children+[('close',n.get('tag'))]
def merge(v):
 out=[]
 for x in v:
  if x[0]=='text' and out and out[-1][0]=='text':out[-1]=('text',out[-1][1]+x[1])
  else:out.append(x)
 return out
for x in r:
 c=x['case'];assert all(v==0 for v in x['exit_codes'].values()) and all(v==0 for v in x['paged_exit_codes'].values())
 for k in ['off_matches_official_html_bytes','on_matches_jobs4_html_bytes','final_tree_on_matches_jobs4','paged_pdf_bytes_equal_three']:assert x[k],(c,k)
 assert not x['any_truncated'],c
 assert x['links']['on']==x['links']['off'],c
 assert merge(semantic(body(tree(c)),True))==merge(semantic(body(tree(c,'off')))),c
 # Controlled fixtures must not supply our test-only carrier/marker attributes.
 for n in nodes(body(tree(c,'off'))):
  assert not ({'data-probe-text','data-probe-marker'} & set(dict(n.get('attrs',[])))),c
 # All ordinary content/role/break paths match stock except explicit marker
 # production and style carriers. Frame values themselves must also match.
report['counts']={'cases':len(r),'off_release_html_equal':16,'on_jobs4_html_and_tree_equal':16,'paged_pdf_equal_three':16,'body_semantic_projection_preserved':16,'world_order_equal':sum(x['world_on_off_equal_ordered'] for x in r)}
for c,t,expected in [('plain-entry','DefaultBody.','16pt'),('plain-entry','ListBody.','9pt'),('override-entry','OverrideBody.','12pt'),('override-entry','OverrideList.','15pt'),('override-entry','LocalBody.','18pt'),('override-entry','AfterLocal.','12pt'),('font-style-entry','LibraryFontBody.','14pt'),('font-style-entry','LibraryListFont.','9pt'),('font-style-entry','NoteFontBody.','12pt'),('font-style-entry','LocalFontBody.','18pt')]:
 assert 'font-size: '+expected in css(c,t),(c,t)
report['style_values']={c:{t:css(c,t) for t in ts} for c,ts in [('plain-entry',['DefaultBody.','ListBody.']),('override-entry',['OverrideBody.','OverrideList.','LocalBody.','AfterLocal.']),('font-style-entry',['LibraryFontBody.','LibraryListFont.','NoteFontBody.','LocalFontBody.']),('whitespace-strong',['Bold','Emph','Local','DeltaZero','Outer','Inner']),('show-context',['PREFIX','HOne','SUFFIX','StrongBody','Emphasized'])]}
assert '"new computer modern"' in css('font-style-entry','NoteFontBody.')
assert 'font-weight: 700' in css('whitespace-strong','Bold')
assert 'font-weight: 400' in css('whitespace-strong','DeltaZero')
assert 'font-style: normal' in css('whitespace-strong','Inner')
assert 'font-style: italic' in css('whitespace-strong','Outer')
assert 'font-weight: 400' in css('show-context','StrongBody')
def els(c,tag):return [n for n in nodes(body(tree(c))) if n.get('tag')==tag]
report['table_css']={c:[n['css'] for n in els(c,'td')] for c in ['plain-entry','override-entry','simple-cell','unsupported']}
assert '1.3pt solid #ff4136' in report['table_css']['plain-entry'][0]
assert '2pt solid #2ecc40' in report['table_css']['override-entry'][0]
assert 'text-align: center' in report['table_css']['simple-cell'][0]
assert 'vertical-align: middle' in report['table_css']['simple-cell'][0]
assert 'text-align: right' in report['table_css']['simple-cell'][1]
assert 'vertical-align: bottom' in report['table_css']['simple-cell'][1]
assert all('border-' not in css for css in report['table_css']['unsupported'])
report['markers']={c:[''.join(v['text'] for v in nodes(n) if v['kind']=='text') for n in nodes(body(tree(c))) if dict(n.get('attrs',[])).get('data-probe-marker')=='true'] for c in ['plain-entry','override-entry','whitespace-strong','unsupported']}
assert report['markers']['plain-entry']==['LIB-V2'] and report['markers']['override-entry']==['NOTE-M']
assert report['markers']['whitespace-strong']==['ONE','TWO']
assert report['markers']['unsupported']==[]
report['heading_introspection']={x['case']:x['introspection']['on'][-1] for x in r if x['case'] in ['show-context','repeat-include','runin-list-table-math']}
assert len(report['heading_introspection']['show-context']['headings'])==2
assert 'GoneHeading' not in ''.join(t for t,s in styles(body(tree('show-context'))))
assert any('https://example.org/table'==dict(n['attrs']).get('href') for n in els('standard-matrix','a'))
report['standard_table_links']=[dict(n['attrs']) for n in els('standard-matrix','a')]
report['breaks']={c:len(els(c,'br')) for c in ['whitespace-break','whitespace-strong','measured-par-used','show-context']}
assert report['breaks']['whitespace-break']==1 and report['breaks']['measured-par-used']==1
assert ''.join(t for t,s in styles(body(tree('measure-discard'))))=='VisibleOnly'
assert len(els('repeat-include','h2'))==2
assert len(els('measured-par-used','p'))==1
assert len(els('same-source-different-show','div'))==2
report['frame_evidence']={}
def frame_items(f):
 for v in f['items']:
  i=v['item'];yield i
  if i['kind']=='group':yield from frame_items(i['frame'])
for c in ['frame-target','unsupported']:
 frames=[n for n in nodes(body(tree(c))) if n['kind']=='frame'];assert len(frames)==1
 items=list(frame_items(frames[0]['frame']));texts=[i['text'] for i in items if i['kind']=='text']
 report['frame_evidence'][c]={'target':frames[0]['target'],'text_items_in_native_order':texts,'link_items':[i['repr'] for i in items if i['kind']=='other' and i['repr'].startswith('Link(')],'dashed_shape_count':sum(i['kind']=='other' and 'dash: Some' in i.get('repr','') for i in items)}
 assert any('paged' in t for t in texts)
assert report['frame_evidence']['unsupported']['dashed_shape_count']>0
report['unsupported_world_call_exception']=next(x['world_counts'] for x in r if x['case']=='unsupported')
(b/'analysis.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8');print(json.dumps(report['counts']))
