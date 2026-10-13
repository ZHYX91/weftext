"""Compile portable design fixtures with an explicitly supplied official Typst binary."""
from pathlib import Path
import argparse,json,subprocess,tempfile
ROOT=Path(__file__).parent
parser=argparse.ArgumentParser()
parser.add_argument('--typst',required=True)
args=parser.parse_args()
results=[]
names=['person','organization','task','work','paper','period','range','event','numeric','root-dynamic-body','saved-query','negative-range-empty','negative-org-literal','negative-person-two-branches','citation','heading-reference','citation-label-collision']
names += ['precision-periods','precision-range','precision-task','general-relations','negative-precision-recurrence','negative-general-relations']
with tempfile.TemporaryDirectory() as tmp:
 for name in names:
  p=subprocess.run([args.typst,'compile',str(ROOT/'native-cases'/f'{name}.typ'),str(Path(tmp)/f'{name}.pdf')],capture_output=True,text=True,encoding='utf-8')
  expected=1 if name=='citation-label-collision' else 0
  assert (p.returncode!=0)==(expected!=0),(name,p.stderr)
  results.append({'case':name,'exit':p.returncode,'expected_compile_failure':bool(expected),'stderr':p.stderr.replace(str(ROOT/'native-cases'),'[native-cases]')})
(ROOT/'syntax-experiments.json').write_text(json.dumps({'tool_version':subprocess.run([args.typst,'--version'],capture_output=True,text=True).stdout.strip(),'evidence':'Official native-language compilation only. Semantic negatives intentionally compile. Node/config identities are fictitious and not Core-admitted. Citation Bib is supplied experiment output, not proof of Work lookup/resource generation.','results':results},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'native_compile_cases':len(results),'expected_collision_error':True}))
