"""Create a private hosted bootstrap config and one-time credentials, without starting a server."""
import argparse
import hashlib
import json
from pathlib import Path
import secrets

parser = argparse.ArgumentParser()
parser.add_argument('--root',type=Path,required=True)
parser.add_argument('--out',type=Path,required=True)
parser.add_argument('--document',action='append',default=[],metavar='ID=RELATIVE.typ')
parser.add_argument('--user',action='append',required=True,help='Initial full-source author ID')
parser.add_argument('--deployment',default='writing-space')
parser.add_argument('--bind',default='127.0.0.1:47821')
parser.add_argument('--origin')
parser.add_argument('--sync-workspace-id')
parser.add_argument('--sync-genesis')
parser.add_argument('--sync-owner',action='append',default=[],help='Explicit full-history sync user; independent of document grants')
args = parser.parse_args()
root = args.root.resolve(strict=True)
output = args.out.resolve()
if output.is_relative_to(root):
    parser.error('Keep the configuration and credentials outside the served document root')
files={}
for declaration in args.document:
    identifier, separator, relative = declaration.partition('=')
    if not separator or identifier in files:
        parser.error('Each document needs a unique ID=RELATIVE.typ')
    path = (root / relative).resolve(strict=True)
    if not path.is_relative_to(root) or path.suffix != '.typ':
        parser.error('Each source must be a .typ file inside root')
    files[identifier]={'path':path.relative_to(root).as_posix(),'title':path.stem,'editable':True}
if len(set(args.user))!=len(args.user) or 'admin' in args.user:
    parser.error('Choose distinct users; admin is reserved by this provisioning helper')
if bool(args.sync_workspace_id) != bool(args.sync_genesis) or (args.sync_owner and not args.sync_workspace_id):
    parser.error('Sync requires both the explicit workspace ID and genesis digest')
if not set(args.sync_owner).issubset(args.user):
    parser.error('Each sync owner must be an explicitly listed user')
tokens={key:secrets.token_urlsafe(32) for key in ['admin',*args.user]}
sha=lambda text:hashlib.sha256(text.encode()).hexdigest()
config={'deployment':args.deployment,'bind':args.bind,'origin':args.origin,'root':str(root),
        'adminTokenSha256':sha(tokens['admin']),'files':files,
        'users':{key:{'tokenSha256':sha(tokens[key]),'enabled':True,
                      'syncOwner':key in args.sync_owner,
                      'grants':{id:{'read':True,'write':True,'history':True,'download':True} for id in files}}
                 for key in args.user}}
if args.sync_workspace_id:
    config['syncIdentity']={'workspaceId':args.sync_workspace_id,'genesis':args.sync_genesis}
output.parent.mkdir(parents=True,exist_ok=True)
with output.open('x',encoding='utf-8') as stream:
    json.dump(config,stream,ensure_ascii=False,indent=2)
print('Configuration created:',output)
print('Initial tokens (copy privately; not stored in the configuration):')
for user,token in tokens.items(): print(user+': '+token)
print('The administrator token manages grants; use a personal token in the WebUI.')
