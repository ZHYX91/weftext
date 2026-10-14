"""Portable documentation gate; optional bounded models run in disposable copies."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]

def check():
    count=0
    files = []
    for directory, children, names in os.walk(ROOT):
        children[:] = [name for name in children if name not in
                       {'.git', 'node_modules', 'target', 'dist', '.workspace', '__pycache__'}]
        files.extend(Path(directory) / name for name in names if name.endswith('.md'))
    for p in sorted(files):
        text=p.read_text(encoding='utf-8')
        plain=re.sub(r'```.*?```','',text,flags=re.S)
        for target in re.findall(r'\]\(([^\n)]+)\)',plain):
            target=target.strip('<>').split(' "')[0]
            if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'):
                continue
            local=unquote(target.split('#')[0])
            assert (p.parent/local).exists(), f'{p.relative_to(ROOT)}: missing {local}'
        if p.name.endswith('.zh-CN.md'):
            peer=p.with_name(p.name.replace('.zh-CN.md','.md'))
            if p.name=='LICENSE.zh-CN.md':
                peer=p.with_name('LICENSE')
            assert peer.exists(),f'Missing English peer: {peer}'
        count+=1
    for p in (ROOT/'docs').rglob('*.json'):
        def unique(pairs):
            d={}
            for k,v in pairs:
                assert k not in d,f'Duplicate JSON member {k}: {p}'
                d[k]=v
            return d
        json.loads(p.read_text(encoding='utf-8-sig'),object_pairs_hook=unique)
    for lang in ['.zh-CN','']:
        req=(ROOT/f'docs/REQUIREMENTS{lang}.md').read_text(encoding='utf-8')
        assert set(re.findall(r'^\| (RQ\d+) /',req,re.M))=={f'RQ{i:02}' for i in range(1,28)}
        assert len(re.findall(r'^\| P[12]-\d+ \|',req,re.M))==21
    print(f'PASS: {count} Markdown files, local file links, bilingual peers, JSON, requirement inventory')
    print('Inventory checks do not establish semantics or product behavior.')

def models():
    with tempfile.TemporaryDirectory(prefix='weftext-check-') as temp:
        copied=Path(temp)/'examples'
        shutil.copytree(ROOT/'docs/design/examples',copied)
        subprocess.run([sys.executable,str(copied/'resolve_builtin_schema.py'),'--check'],check=True,cwd=copied)
        for script in ['test_builtin_schema.py','validate_repair.py','precision_relation_model.py']:
            subprocess.run([sys.executable,str(copied/script)],check=True,cwd=copied)
    print('PASS: bounded design models; no product acceptance claimed; source evidence unchanged')

if __name__=='__main__':
    p=argparse.ArgumentParser()
    p.add_argument('--models',action='store_true')
    args=p.parse_args()
    check()
    if args.models:
        models()
