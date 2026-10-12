from pathlib import Path
import os, subprocess, json, hashlib, collections, difflib
import argparse
ap=argparse.ArgumentParser()
ap.add_argument('--instrumented-cli',type=Path,required=True)
ap.add_argument('--official-cli',type=Path,required=True)
ap.add_argument('--output',type=Path,default=Path('probe-results'))
args=ap.parse_args()
base=Path(__file__).parent.resolve()
binary=args.instrumented_cli.resolve()
official=args.official_cli.resolve()
out=args.output.resolve();out.mkdir(exist_ok=True)
events=[]
def run(cmd,env=None):
 p=subprocess.run(list(map(str,cmd)),env=env,capture_output=True,text=True)
 events.append({'cmd':list(map(str,cmd)),'exit':p.returncode,'stdout':p.stdout,'stderr':p.stderr})
 if p.returncode: raise RuntimeError(p.stderr)
 return p.stdout

def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def objects(obj):
 if isinstance(obj,dict):
  if 'elem' in obj:yield obj
  for v in obj.values():yield from objects(v)
 elif isinstance(obj,list):
  for v in obj:yield from objects(v)
def frame_text(obj):
 if isinstance(obj,dict):
  if obj.get('kind')=='text':yield obj['text']
  for k,v in obj.items():
   if k!='glyph_spans':yield from frame_text(v)
 elif isinstance(obj,list):
  for v in obj:yield from frame_text(v)

summaries=[]
for name in ['default','show-rewrite','show-remove','show-strip-strong','repeat-include','generated','runin-list-table-math','duplicated-variable','target-branch','resource','computed-join']:
 paths={}
 for mode,exe in [('on',binary),('off',binary),('official',official)]:
  env=os.environ.copy();env.pop('WEFTEXT_PROBE_LOG',None);env.pop('WEFTEXT_PROBE_MODE',None)
  log=out/f'{name}.{mode}.jsonl';log.unlink(missing_ok=True)
  if mode!='official':
   env['WEFTEXT_PROBE_LOG']=str(log)
   if mode=='off':env['WEFTEXT_PROBE_MODE']='world-only'
  pdf=out/f'{name}.{mode}.pdf';paths[mode]=pdf
  run([exe,'compile',base/'fixtures'/f'{name}.typ',pdf,'--creation-timestamp','0','--ignore-system-fonts','--jobs','1'],env)
 logs={mode:[json.loads(s) for s in (out/f'{name}.{mode}.jsonl').read_text(encoding='utf-8').splitlines()] for mode in ['on','off']}
 final=next(e['payload'] for e in logs['on'] if e['stage']=='final_introspection')
 frames={mode:next(e['payload'] for e in logs[mode] if e['stage']=='final_frames') for mode in logs}
 resources={mode:[(e['stage'],e['payload']) for e in logs[mode] if e['stage'].startswith('world_')] for mode in logs}
 rounds=[e['payload'] for e in logs['on'] if e['stage']=='round_trace']
 for r in rounds:
  start=next(i for i,e in enumerate(logs['on']) if e['stage']=='round_start' and e['payload']['round']==r['round'])
  end=next(i for i,e in enumerate(logs['on']) if e['stage']=='round_end' and e['payload']['round']==r['round'])
  r['physical_counts']=dict(collections.Counter(e['stage'] for e in logs['on'][start+1:end] if e['stage'] in ['show_return','realize_result']))
 stable=next(r for r in rounds if r['stable'])
 # Publish successful stabilized tracked trace; earlier rounds retained as diagnostic logs only.
 (out/f'{name}.final-trace.json').write_text(json.dumps(stable,ensure_ascii=False,indent=2),encoding='utf-8')
 source_map={}
 for e in logs['on']:
  if e['stage']=='source_snapshot':
   for node in e['payload']['nodes']:source_map[node['span']]={'file':e['payload']['file'],**node}
 occurrences=[]
 for node in objects(next(e['payload']['content'] for e in logs['on'] if e['stage']=='evaluated')):
  if node['elem']=='text':
   origin=source_map.get(node['span'])
   occurrences.append({'text':node['fields'].get('text'),'span':node['span'],'source_node':origin})
 (out/f'{name}.evaluated-origins.json').write_text(json.dumps(occurrences,ensure_ascii=False,indent=2),encoding='utf-8')
 physical=[e for e in logs['on'] if e['stage'] in ['show_return','realize_result']]
 src=''.join(frame_text(frames['on']))
 result={'sample':name,'pdf_hashes':{m:digest(p) for m,p in paths.items()},'pdf_bytes_equal_all_three':len({digest(p) for p in paths.values()})==1,'world_requests_equal_ordered':resources['on']==resources['off'],'world_request_counts':{m:dict(collections.Counter(stage for stage,_ in v)) for m,v in resources.items()},'final_frames_equal':frames['on']==frames['off'],'frame_text_native_traversal':list(frame_text(frames['on'])),'heading_count':len(final['headings']),'heading_locations':[h['location'] for h in final['headings']],'metadata':final['metadata'],'rounds':[{'round':r['round'],'stable':r['stable'],'physical_counts':r['physical_counts'],'tracked_counts':dict(collections.Counter(e['stage'] for e in r['events']))} for r in rounds],'final_show_events':[e['payload'] for e in stable['events'] if e['stage']=='show_return' and e['payload']['input']['elem'] in ['heading','strong','context']],'evaluated_origin_counts':dict(collections.Counter(n['span'] for n in occurrences))}
 assert result['pdf_bytes_equal_all_three'],name
 assert not any(n.get('truncated') for n in objects(stable)),name
 assert result['world_requests_equal_ordered'],name
 assert result['final_frames_equal'],name
 counters=[row['number'] for m in final['metadata'] for row in m['fields'].get('value',[]) if isinstance(row,dict) and 'number' in row]
 result['final_counter_values']=counters
 if name in ['repeat-include','generated']:assert counters==[[1],[2]],name
 if name=='default':assert len([e for e in stable['events'] if e['stage']=='realize_result']) > stable['physical_counts'].get('realize_result',0),name
 if name=='show-remove':assert len(final['headings'])==1 and 'GoneHeading' not in src
 if name=='show-rewrite':assert 'PREFIX' in src and 'SUFFIX' in src
 if name=='repeat-include':assert len(final['headings'])==2 and len(set(result['heading_locations']))==2
 if name=='target-branch':assert 'PAGEDBranch' in src and 'HTMLBranch' not in src
 summaries.append(result)
 print(name,'PDF/source/font-order/frame equality PASS; final',result['rounds'][-1],flush=True)
(out/'summary.json').write_text(json.dumps(summaries,ensure_ascii=False,indent=2),encoding='utf-8')
(out/'events.json').write_text(json.dumps(events,ensure_ascii=False,indent=2),encoding='utf-8')
print('ALL',len(summaries),'PASSED')
