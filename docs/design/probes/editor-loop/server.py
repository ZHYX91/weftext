"""Local, bounded editor experiment. Only edits its dedicated experiment file.

Never point --data at a user library. This is not the production Core/backend.
"""
import argparse
import collections
import ctypes
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

HERE = Path(__file__).resolve().parent
DEMO = '= 中文编辑实验\n\n甲*乙*丙。中文正文。\n\n- 第一项\n- 第二项\n'
SAFE_SYNTAX = {'Markup','Text','Space','Parbreak','Linebreak','Strong','Emph',
             'Heading','HeadingMarker','ListItem','ListMarker','EnumItem','EnumMarker',
             'Escape','Shorthand','SmartQuote','LineComment','BlockComment','Raw','Link',
             'RawLang','RawDelim','RawTrimmed','Star','Underscore'}
SAFE_PARENTS = {'Markup','Strong','Emph','Heading','ListItem','EnumItem'}


def read_text(path):
    return Path(path).read_bytes().decode('utf-8')


def atomic_json(path, value):
    tmp = path.with_suffix('.pending')
    with tmp.open('wb') as f:
        f.write(json.dumps(value,ensure_ascii=False).encode('utf-8'))
        f.flush(); os.fsync(f.fileno())
    os.replace(tmp, path)


class Probe:
    def __init__(self, binary, data):
        self.binary = str(Path(binary).resolve())
        self.data = Path(data).resolve()
        marker = self.data/'editor-loop-owned.json'
        if self.data.exists() and any(self.data.iterdir()) and not marker.exists():
            raise ValueError('Use an empty dedicated experiment directory')
        self.data.mkdir(parents=True, exist_ok=True)
        if not marker.exists(): atomic_json(marker, {'purpose':'disposable editor experiment'})
        self.note = self.data/'note.typ'
        if not self.note.exists(): self.note.write_bytes(DEMO.encode('utf-8'))
        self.lock = threading.Lock()
        self.compile_lock = threading.Lock()

    def opened(self):
        with self.note.open('rb') as f:
            raw=f.read().decode('utf-8'); st=os.fstat(f.fileno())
        return {'source':raw,'file_id':[st.st_dev,st.st_ino],'name':'note.typ'}

    def save(self, request, fault=False):
        if os.name != 'nt': raise ValueError('This experiment tests Windows share-zero only')
        with self.lock:
            journal=self.data/'save-journal.json'
            if journal.exists():
                prior=json.loads(read_text(journal))
                if prior['operation']==request['operation']:
                    if prior['after'] != request['source'] or prior['before'] != request['base']:
                        raise ValueError('Operation ID reused with different contents')
                    return {'state':prior['state'],'source':prior['after']}
                if prior['state'] != 'committed':
                    raise ValueError('Pending recovery retained; inspect original operation first')
            # Private experiment target only; ordinary save protects this same handle.
            import msvcrt
            kernel=ctypes.WinDLL('kernel32',use_last_error=True)
            kernel.CreateFileW.argtypes=[ctypes.c_wchar_p,ctypes.c_uint32,ctypes.c_uint32,ctypes.c_void_p,ctypes.c_uint32,ctypes.c_uint32,ctypes.c_void_p]
            kernel.CreateFileW.restype=ctypes.c_void_p
            handle=kernel.CreateFileW(str(self.note),0xC0000000,0,None,3,0x80,None)
            if handle == ctypes.c_void_p(-1).value:
                raise OSError(ctypes.get_last_error(),'File busy or unavailable')
            fd=msvcrt.open_osfhandle(handle,os.O_RDWR|os.O_BINARY)
            with os.fdopen(fd,'r+b') as f:
                st=os.fstat(f.fileno()); before=f.read().decode('utf-8')
                if [st.st_dev,st.st_ino] != request['file_id'] or before != request['base']:
                    raise ValueError('External change: original and proposed drafts retained')
                record={'operation':request['operation'],'before':before,'after':request['source'],'state':'prepared'}
                atomic_json(journal,record)
                f.seek(0);f.write(request['source'].encode('utf-8'));f.truncate();f.flush();os.fsync(f.fileno())
                if fault: raise RuntimeError('Injected interruption after file flush')
                record['state']='committed';atomic_json(journal,record)
            return {'state':'committed',**self.opened()}

    def recovery(self):
        path=self.data/'save-journal.json'
        if not path.exists(): return {'state':'none'}
        j=json.loads(read_text(path));current=read_text(self.note)
        return {**j,'current':current,'file_state':'after' if current==j['after'] else 'before' if current==j['before'] else 'third'}

    def compile(self, source, serial):
        if len(source.encode('utf-8')) > 200000: raise ValueError('Experiment input limit')
        with self.compile_lock, tempfile.TemporaryDirectory(prefix='compile-',dir=self.data) as tmp:
            root=Path(tmp)
            (root/'note.typ').write_bytes(source.encode('utf-8'))
            # Transparent application entry, not source metadata or a new author file.
            (root/'entry.typ').write_text('#set text(font: ("Microsoft YaHei", "New Computer Modern"))\n#include "note.typ"',encoding='utf-8')
            (root/'child.typ').write_text('被引用的子文档。',encoding='utf-8')
            log=root/'native.jsonl';log.write_text('',encoding='utf-8')
            env=os.environ.copy();env['WEFTEXT_PROBE_LOG']=str(log);env['WEFTEXT_PROBE_MODE']='world-only';env['TYPST_HTML_STYLE_PROBE']='1'
            run=subprocess.run([self.binary,'compile',str(root/'entry.typ'),str(root/'output.html'),'--root',str(root),'--features','html','--creation-timestamp','0','--jobs','1'],env=env,capture_output=True,timeout=40)
            response={'serial':serial,'source':source,'ok':run.returncode==0,'diagnostic':run.stderr.decode('utf-8',errors='replace'),'writer_profile':'literal-markup-only'}
            if run.returncode: return response
            events=[json.loads(line) for line in read_text(log).splitlines()]
            trees=[e['payload'] for e in events if e.get('stage')=='final_html_tree']
            if not trees: raise ValueError('Compiler has no source-origin hook')
            tree=trees[-1]
            leaves=[]
            def walk(n):
                if n.get('kind')=='text': leaves.append(n)
                for child in n.get('children',[]): walk(child)
            walk(tree)
            keys=collections.Counter((o.get('file'),o.get('start'),o.get('end')) for n in leaves if (o:=n.get('origin')))
            raw=source.encode('utf-8');candidates=[]
            for index,n in enumerate(leaves):
                n['occurrence']=index;o=n.get('origin') or {};a=o.get('start',0);b=o.get('end',0)
                main=o.get('file','').strip('"')=='/note.typ'
                ancestors=o.get('ancestors',[])
                # Deliberately bounded: no code/show qualification is inferred from string equality.
                kinds=set(o.get('syntax_kinds',[]))
                safe=bool(kinds) and kinds<=SAFE_SYNTAX and all(p['kind'] in SAFE_PARENTS for p in ancestors)
                match=main and o.get('kind')=='Text' and raw[a:b].decode('utf-8',errors='replace')==n['text']
                n['writable']=bool(safe and match and keys[(o.get('file'),a,b)]==1)
                if main:
                    n['from']=len(raw[:a].decode('utf-8').encode('utf-16-le'))//2
                    n['to']=len(raw[:b].decode('utf-8').encode('utf-16-le'))//2
                if n['writable']: candidates.append({'occurrence':index,'from':n['from'],'to':n['to'],'text':n['text'],'origin':o})
            response.update(tree=tree,candidates=candidates)
            return response


class Handler(BaseHTTPRequestHandler):
    def log_message(self,*args): pass
    def local_host(self):
        return self.headers.get('Host') in {f'127.0.0.1:{self.server.server_port}',f'localhost:{self.server.server_port}'}
    def reply(self,value,status=200,kind='application/json'):
        data=value if isinstance(value,bytes) else json.dumps(value,ensure_ascii=False).encode('utf-8')
        self.send_response(status);self.send_header('Content-Type',kind+'; charset=utf-8');self.send_header('Cache-Control','no-store');self.end_headers();self.wfile.write(data)
    def do_GET(self):
        if not self.local_host(): return self.reply({'error':'host'},403)
        if self.path=='/open': return self.reply(self.server.probe.opened())
        if self.path=='/recovery': return self.reply(self.server.probe.recovery())
        files={'/':'index.html','/session.mjs':'session.mjs','/app.mjs':'app.mjs'}
        if self.path not in files: return self.reply({'error':'not found'},404)
        return self.reply((HERE/files[self.path]).read_bytes(),kind='text/html' if self.path=='/' else 'text/javascript')
    def do_POST(self):
        if not self.local_host(): return self.reply({'error':'host'},403)
        origin=self.headers.get('Origin')
        if origin and origin != 'http://'+self.headers.get('Host',''): return self.reply({'error':'origin'},403)
        if self.headers.get('Content-Type')!='application/json': return self.reply({'error':'content type'},415)
        length=int(self.headers.get('Content-Length','0'))
        if length>1000000: return self.reply({'error':'limit'},413)
        try:
            req=json.loads(self.rfile.read(length))
            if self.path=='/compile': result=self.server.probe.compile(req['source'],req['serial'])
            elif self.path=='/save': result=self.server.probe.save(req)
            else: raise ValueError('Unknown operation')
            return self.reply(result)
        except Exception as e: return self.reply({'error':str(e)},409)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--data',required=True);p.add_argument('--port',type=int,default=8765);a=p.parse_args()
    server=ThreadingHTTPServer(('127.0.0.1',a.port),Handler);server.probe=Probe(a.binary,a.data)
    print(f'Editor experiment: http://127.0.0.1:{a.port}',flush=True);server.serve_forever()
