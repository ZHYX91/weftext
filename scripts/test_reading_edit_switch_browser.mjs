// Chromium desktop-entry test. Core products are injected; no packaged host
// or OS input behavior is claimed by this browser check.
import { createServer } from '../app/node_modules/vite/dist/node/index.js';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const app = fileURLToPath(new URL('../app/', import.meta.url));
const vite = await createServer({root:app,configFile:false,server:{host:'127.0.0.1',port:0,hmr:false}});
const browser=await chromium.launch({executablePath:process.env.WEFTEXT_BROWSER_EXECUTABLE,headless:true});
try {
 await vite.listen();
 const url='http://127.0.0.1:'+vite.httpServer.address().port;
 const page=await browser.newPage({ viewport:{width:1120,height:760}, locale:'zh-CN' });
 page.setDefaultTimeout(8000);
 const errors=[];
 page.on('pageerror',e=>errors.push(e.message));
 await page.addInitScript(()=>localStorage.setItem('weftext-view','read'));
 const title='Chapter target';
 const filler=Array.from({length:90},(_,i)=>`Paragraph ${i+1} before the chapter.`).join('\n\n');
 const source=`${filler}\n\n= ${title}\n\nThe end.\n`;
 const sourceHash=createHash('sha256').update(source).digest('hex');
 const from=source.indexOf(title),to=from+title.length;
 const doc={sessionId:'reading-fixture',viewInputId:'input-one',path:'/tmp/synthetic/reading.typ',root:'/tmp/synthetic',kind:'ordinary',source,baseId:sourceHash};
 const inventory={root:doc.root,status:'ordinary',entries:[],warnings:[],complete:true};
 const text=(key,str)=>({key,kind:'text',tag:null,text:str,children:[],origin:(key==='heading-text'?{path:doc.path,from,to,utf16From:from,utf16To:to,raw:str,syntaxKind:'text',isMain:true}:null),writable:false,attrs:{}});
 const element=(key,tag,children,attrs={})=>({key,tag,kind:'element',text:null,children,attrs,origin:null,writable:false});
 const heading={key:'test-heading',kind:'heading',text:title,level:1,generated:false,range:{utf16From:from,utf16To:to}};
 const tree=element('doc','div',[...filler.split('\n\n').map((s,i)=>element('p'+i,'p',[text('t'+i,s)])),element(heading.key,'h1',[text('heading-text',title)],{'data-weftext-role':'heading','data-weftext-level':'1'}),element('p-last','p',[text('end','The end.')])]);
 const role={status:'absent',text:null,range:null,construct:null,encoding:null};
 const map={sourceHash,syntaxValid:true,literalRegions:[],structures:[],title:role,subtitle:role,bodyInsertion:null,unavailableReason:null,notes:{sourceHash,syntaxValid:true,entries:[]},headingNumbering:{sourceHash,settings:null,editable:true,frozenEndnotesRequired:false,reason:null},bodyBlocks:[]};
 await page.route('**/api/**',async route=>{
   const path=new URL(route.request().url()).pathname;
   const body=route.request().postDataJSON()??{};
   const fixtures={
    '/api/bootstrap':{inventory,initialDocument:doc,recoveries:[]},
    '/api/library/open':inventory,
    '/api/render':{ok:true,html:'',tree,diagnostics:[],sourceInfo:{syntaxValid:true,identityStatus:'ordinary',types:[],headings:[],links:[],syntaxTokens:[]},dependencies:[],fontFamilies:[],compilerVersion:'injected outline browser fixture',serial:body.serial,source:body.source,headingStructure:{coverage:'complete',truncated:false,items:[heading]}},
    '/api/editor-map':{...map,serial:body.serial},
    '/api/query/embedded/discover':{serial:body.serial,source:body.source,sourceHash,syntaxValid:true,occurrences:[]},
   };
   return route.fulfill(path in fixtures ? {json:fixtures[path]}:{status:404,json:{}});
 });
 await page.goto(url);
 await page.locator('.mode-read .flow-document [data-document-node-key="test-heading"]').waitFor();
 const ribbon=page.locator('.workspace-ribbon');
 await ribbon.locator('[data-tab="view"][role="tab"]').click();
 const existing=page.locator('[data-dock-tab="outline"]');
 if(await existing.isVisible()) await existing.click();
 else {
   await ribbon.locator('.workspace-dock-view-menu summary').click();
   await ribbon.locator('[data-show-view="outline"]').click();
 }
 const search=page.locator('[data-outline-control="search"]');
 await search.fill(title);
 await search.press('Enter');
 const target=page.locator('.mode-read .flow-document [data-document-node-key="test-heading"]');
 assert.equal(await target.evaluate(e=>document.activeElement===e),true);
 assert.equal(await page.locator('.source-host .cm-content').count(),0);
 assert.ok(await page.locator('.mode-read .flow-pane').evaluate(e=>e.scrollTop)>0);
 const reader = page.locator('.mode-read .flow-pane');
 const readScroll = await reader.evaluate(e=>e.scrollTop);
 const edit = page.locator('.view-caption .read-edit-shortcut');
 await edit.click();
 await page.locator('.mode-source .cm-content').waitFor();
 const sourcePosition = () => page.locator('.mode-source .cm-content').evaluate(e=>{
   const view=e.cmTile?.root?.view;
   if (view?.contentDOM !== e) throw new Error('Actual CodeMirror view is unavailable');
   const {from,to}=view.state.selection.main;
   return { from,to, viewport: view.viewport, active:view.hasFocus };
 });
 let position=await sourcePosition();
 assert.equal(position.from,from);
 assert.equal(position.to,from);
 assert.equal(position.active,true);
 await page.waitForFunction(pos=>[...document.querySelectorAll('.mode-source .cm-line')].some(e=>e.textContent.includes(pos)),title);
 await page.locator('.view-caption .read-edit-shortcut').click();
 await reader.waitFor();
 await page.waitForFunction(scroll=>Math.abs(document.querySelector('.mode-read .flow-pane').scrollTop-scroll)<2,readScroll);
 await target.evaluate(e=>{
   const range=document.createRange();
   range.setStart(e.firstChild,0);range.setEnd(e.firstChild,7);
   window.getSelection().removeAllRanges();window.getSelection().addRange(range);
 });
 await edit.click();
 await page.locator('.mode-source .cm-content').waitFor();
 position=await sourcePosition();
 assert.deepEqual([position.from,position.to],[from,from+7]);
 await page.locator('.view-caption .read-edit-shortcut').press('Enter');
 await reader.waitFor();
 await page.waitForFunction(scroll=>Math.abs(document.querySelector('.mode-read .flow-pane').scrollTop-scroll)<2,readScroll);
 assert.equal(await page.locator('.view-caption .read-edit-shortcut').evaluate(e=>e===document.activeElement),true);
 await page.setViewportSize({width:740,height:640});
 assert.equal(await edit.isVisible(),true);
 assert.equal(await edit.evaluate(e=>e.getBoundingClientRect().right<=window.innerWidth),true);
 assert.deepEqual(errors,[]);
 console.log('PASS actual workspace: direct edit uses focused heading or selected reading text; returning restores previous reading scroll');
} finally {await browser.close(); await vite.close();}
