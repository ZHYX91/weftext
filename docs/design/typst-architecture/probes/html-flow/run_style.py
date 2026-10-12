"""Bounded author experiment. Uses own binary/cache, not frozen earlier probes.
Usage: python run_style.py --binary /path/typst --official /path/official-typst
"""
import argparse, os, json, subprocess, hashlib, collections
from pathlib import Path
ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--official',required=True);a=ap.parse_args()
base=Path(__file__).resolve().parent;out=base/'results';out.mkdir(exist_ok=True)
cases=['plain-entry','override-entry','font-style-entry','standard-matrix','runin-list-table-math','whitespace-break','whitespace-strong','show-context','simple-cell','unsupported','target-branch','frame-target','measure-discard','repeat-include','same-source-different-show','measured-par-used']
events=[];results=[]
def run(cmd,env):
 p=subprocess.run(list(map(str,cmd)),env=env,capture_output=True,text=True,encoding='utf-8',errors='replace');events.append({'command':list(map(str,cmd)),'returncode':p.returncode,'stdout':p.stdout,'stderr':p.stderr});return p.returncode

def load(p):return [json.loads(l) for l in p.read_text(encoding='utf-8').splitlines()] if p.exists() else []
def body_tree(n):
 if n.get('tag')=='body':return n
 for c in n.get('children',[]):
  r=body_tree(c)
  if r:return r
 return None
def leaves(n):
 if n['kind']=='text':yield n['text']
 elif n['kind']=='element':
  for c in n['children']:yield from leaves(c)
def nodes(n):
 yield n
 if n['kind']=='element':
  for c in n['children']:yield from nodes(c)
def truncated(v):
 if isinstance(v,dict):return v.get('truncated') is True or any(truncated(x) for x in v.values())
 if isinstance(v,list):return any(truncated(x) for x in v)
 return False
for case in cases:
 rec={'case':case};logs={};trees={};codes={}
 for mode in ['off','on','official','jobs4']:
  env=os.environ.copy();env.pop('TYPST_HTML_STYLE_PROBE',None);env.pop('WEFTEXT_PROBE_LOG',None)
  if mode!='official':
   log=out/f'{case}.{mode}.jsonl';log.write_text('');env['WEFTEXT_PROBE_LOG']=str(log);env['WEFTEXT_PROBE_MODE']='world-only'
   if mode in ['on','jobs4']:env['TYPST_HTML_STYLE_PROBE']='1'
  exe=a.official if mode=='official' else a.binary
  html=out/f'{case}.{mode}.html';codes[mode]=run([exe,'compile',base/'fixtures'/f'{case}.typ',html,'--features','html','--creation-timestamp','0','--ignore-system-fonts','--jobs','4' if mode=='jobs4' else '1'],env)
  if mode!='official':
   logs[mode]=load(log);t=[e['payload'] for e in logs[mode] if e['stage']=='final_html_tree'];trees[mode]=t[-1] if t else None
   if t:(out/f'{case}.{mode}.final-tree.json').write_text(json.dumps(t[-1],ensure_ascii=False,indent=2),encoding='utf-8')
 rec['introspection']={m:[e['payload'] for e in logs[m] if e['stage']=='final_introspection'] for m in logs}
 rec['exit_codes']=codes
 if all(x==0 for x in codes.values()):
  rec['off_matches_official_html_bytes']=(out/f'{case}.off.html').read_bytes()==(out/f'{case}.official.html').read_bytes()
  rec['on_matches_jobs4_html_bytes']=(out/f'{case}.on.html').read_bytes()==(out/f'{case}.jobs4.html').read_bytes()
  rec['final_tree_on_matches_jobs4']=trees['on']==trees['jobs4']
  world=lambda m:[(e['stage'],e['payload']) for e in logs[m] if e['stage'].startswith('world_')]
  rec['world_on_off_equal_ordered']=world('on')==world('off');rec['world_counts']={m:dict(collections.Counter(s for s,p in world(m))) for m in logs}
  rec['display_text']={m:''.join(leaves(body_tree(t))) for m,t in trees.items()}
  rec['elements']={m:dict(collections.Counter(n['tag'] for n in nodes(t) if n['kind']=='element')) for m,t in trees.items()}
  rec['links']={m:[dict(n['attrs']).get('href') for n in nodes(t) if n['kind']=='element' and n['tag']=='a'] for m,t in trees.items()}
  rec['frame_count']={m:sum(n['kind']=='frame' for n in nodes(t)) for m,t in trees.items()}
  rec['any_truncated']=any(truncated(t) for t in trees.values()) or truncated(rec['introspection'])
 # Paged must be totally unchanged; official print remains separate target.
 pc={}
 for mode in ['off','on','official']:
  env=os.environ.copy();env.pop('WEFTEXT_PROBE_LOG',None);env.pop('TYPST_HTML_STYLE_PROBE',None)
  if mode=='on':env['TYPST_HTML_STYLE_PROBE']='1'
  exe=a.official if mode=='official' else a.binary
  pdf=out/f'{case}.{mode}.pdf';pc[mode]=run([exe,'compile',base/'fixtures'/f'{case}.typ',pdf,'--features','html','--creation-timestamp','0','--ignore-system-fonts','--jobs','1'],env)
 rec['paged_exit_codes']=pc
 if all(x==0 for x in pc.values()):rec['paged_pdf_bytes_equal_three']=len({hashlib.sha256((out/f'{case}.{m}.pdf').read_bytes()).hexdigest() for m in pc})==1
 results.append(rec);print(case,codes,pc,flush=True)
 (out/'results.json').write_text(json.dumps(results,ensure_ascii=False,indent=2),encoding='utf-8');(out/'events.json').write_text(json.dumps(events,ensure_ascii=False,indent=2),encoding='utf-8')
