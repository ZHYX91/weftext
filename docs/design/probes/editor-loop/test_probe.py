"""Actual compiler round trips and isolated Windows file/recovery checks."""
import argparse
import json
from pathlib import Path
import tempfile
from server import Probe, DEMO, read_text

p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--output',required=True);args=p.parse_args()
rows=[]
def check(name, value):
    rows.append({'name':name,'pass':bool(value)})
    assert value,name

with tempfile.TemporaryDirectory(prefix='weftext-editor-loop-') as tmp:
    probe=Probe(args.binary,tmp)
    result=probe.compile(DEMO,0)
    check('actual-final-native-origins',result['ok'] and len(result['candidates'])>=5)
    c=next(c for c in result['candidates'] if c['text']=='乙')
    raw=DEMO.encode('utf-8');a=c['origin']['start'];b=c['origin']['end']
    after=(raw[:a]+'改好的中文'.encode('utf-8')+raw[b:]).decode('utf-8')
    second=probe.compile(after,1)
    check('strong-chinese-roundtrip-keeps-outside-bytes',second['ok'] and '甲*改好的中文*丙' in after and any(c['text']=='改好的中文' for c in second['candidates']))
    import re
    special='中文*#=+-~@[]。'
    encoded=re.sub(r'[!-/:-@\[-`{-~]',lambda m:'\\'+m[0],special)
    literal=probe.compile(encoded,2)
    def texts(n):
        return (n.get('text','') if n.get('kind')=='text' else '')+''.join(texts(c) for c in n.get('children',[]))
    check('escaped-punctuation-keeps-literal-rendering',literal['ok'] and special in texts(literal['tree']))
    for name,source in [
        ('repeated-include-no-writer','#include "child.typ"\n#include "child.typ"'),
        ('show-rewrite-no-guess','#show strong: [替换]\n甲*乙*丙'),
        ('generated-function-no-guess','#let word = [同字]\n#word #word'),
        ('nested-code-no-writer','= 标题 #strong[插入]\n\n普通文字'),
        ('table-is-source-only-in-this-profile','#table(columns: 2, [甲], [乙])')]:
        r=probe.compile(source,2);check(name,r['ok'] and not r['candidates'])
    invalid=probe.compile('#let broken = (',3);check('invalid-source-retained',not invalid['ok'] and invalid['source']=='#let broken = (')
    empty=probe.compile('',4);check('empty-source-compiles',empty['ok'])
    opened=probe.opened();request={'operation':'one','base':opened['source'],'source':after,'file_id':opened['file_id']}
    saved=probe.save(request);check('same-handle-save',saved['state']=='committed' and read_text(probe.note)==after)
    check('same-operation-retry',probe.save(request)['state']=='committed')
    baseline=probe.opened();probe.note.write_bytes('外部新稿'.encode('utf-8'))
    try: probe.save({'operation':'two','base':baseline['source'],'source':'不得覆盖','file_id':baseline['file_id']}); conflict=False
    except ValueError: conflict=True
    check('external-change-preserved',conflict and read_text(probe.note)=='外部新稿')

with tempfile.TemporaryDirectory(prefix='weftext-editor-recovery-') as tmp:
    probe=Probe(args.binary,tmp);opened=probe.opened()
    request={'operation':'interrupted','base':opened['source'],'source':'写完但未决','file_id':opened['file_id']}
    try: probe.save(request,fault=True)
    except RuntimeError: pass
    reopened=Probe(args.binary,tmp);r=reopened.recovery()
    check('restart-preserves-undecided-before-after',r['state']=='prepared' and r['file_state']=='after' and r['before']==DEMO)
    reopened.note.write_bytes('第三方新稿'.encode('utf-8'))
    r=reopened.recovery();check('recovery-preserves-third-state',r['file_state']=='third' and read_text(reopened.note)=='第三方新稿')

Path(args.output).write_text(json.dumps({'passed':all(r['pass'] for r in rows),'cases':rows,'scope':'literal markup origin experiment; injected exception is not a power-loss test; real OS IME and production writer remain unrun'},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(f'PASS: {len(rows)} bounded compiler/file cases')
