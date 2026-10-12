"""Official Typst citation syntax experiment; not application role validation."""
from pathlib import Path
import json, subprocess, sys
ROOT=Path(__file__).parent
tool=Path(sys.argv[1])
EX=ROOT/'native-cases'
bib='@article{chen2026, author = {Zhang, San and Unknown, Author}, title = {Native structured notes}, journal = {Fixture Journal}, year = {2026}, volume = {1}, pages = {1--2}}\n'
(EX/'weftext.citations.bib').write_text(bib,encoding='utf-8')
head='#metadata((id: "a9201319-d7ea-4ab1-9b89-401c8818d175", types: ())) <weftext>\n#metadata((kind: "weftext.citations", entries: ((key: "chen2026", work: "weftext-node://2fd54310-65f7-4d3c-8668-2f1e344a56c0/adbbf750-b81a-40b9-9c51-e43efab5bb30"),))) <weftext-citations>\n'
cases={'citation':head+'Native reference @chen2026. Explicit #cite(<chen2026>).\n#bibliography("weftext.citations.bib")\n','citation-label-collision':head+'#set heading(numbering: "1.")\n= Heading <chen2026>\n@chen2026. #cite(<chen2026>)\n#bibliography("weftext.citations.bib")\n','heading-reference':'#set heading(numbering: "1.")\n= Heading <only-heading>\n@only-heading\n'}
results=[]
for name,source in cases.items():
 (EX/f'{name}.typ').write_text(source,encoding='utf-8')
 p=subprocess.run([str(tool),'compile',str(EX/f'{name}.typ'),str(EX/f'{name}.pdf')],capture_output=True,text=True,encoding='utf-8')
 results.append({'case':name,'exit':p.returncode,'stderr':p.stderr.replace(str(EX),'[native-cases]')})
assert results[0]['exit']==0 and results[1]['exit']!=0 and results[2]['exit']==0
(ROOT/'citation-experiments.json').write_text(json.dumps({'evidence':'Official CLI compilation of hand-supplied generated-resource fixture only; not Paper lookup, metadata authentication, citation relation extraction or product resource generation. The bibliography is experiment output, not a second author authority.','tool_version':subprocess.run([str(tool),'--version'],capture_output=True,text=True).stdout.strip(),'results':results},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps(results,ensure_ascii=False))
