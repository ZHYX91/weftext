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
 page.setDefaultTimeout(18000);
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
 const text=(currentSource,key,str)=>{
   const start=currentSource.indexOf(str),end=start+str.length;
   assert.ok(start>=0 && currentSource.lastIndexOf(str)===start,'rendered fixture text must have one exact source origin');
   return {key,kind:'text',tag:null,text:str,children:[],origin:{path:doc.path,from:start,to:end,utf16From:start,utf16To:end,raw:str,syntaxKind:'text',isMain:true},writable:false,attrs:{}};
 };
 const element=(key,tag,children,attrs={})=>({key,tag,kind:'element',text:null,children,attrs,origin:null,writable:false});
 const project = currentSource => {
   const start=currentSource.indexOf(title);
   const heading={key:'test-heading',kind:'heading',text:title,level:1,generated:false,range:{utf16From:start,utf16To:start+title.length}};
   const children=currentSource.trim().split('\n\n').map((str,i)=>str===`= ${title}`
     ? element(heading.key,'h1',[text(currentSource,'heading-text',title)],{'data-weftext-role':'heading','data-weftext-level':'1'})
     : element('p'+i,'p',[text(currentSource,'t'+i,str)]));
   return {heading,tree:element('doc','div',children)};
 };
 const role={status:'absent',text:null,range:null,construct:null,encoding:null};
 const map={sourceHash,syntaxValid:true,literalRegions:[],structures:[],title:role,subtitle:role,bodyInsertion:null,unavailableReason:null,notes:{sourceHash,syntaxValid:true,entries:[]},headingNumbering:{sourceHash,settings:null,editable:true,frozenEndnotesRequired:false,reason:null},bodyBlocks:[]};
 let holdRender = null;
 await page.route('**/api/**',async route=>{
   const path=new URL(route.request().url()).pathname;
   const body=route.request().postDataJSON()??{};
   const currentSource=body.source??source;
   if (path==='/api/render' && currentSource===holdRender?.source) await holdRender.promise;
   const currentHash=createHash('sha256').update(currentSource).digest('hex');
   const {heading,tree}=project(currentSource);
   const fixtures={
    '/api/bootstrap':{inventory,initialDocument:doc,recoveries:[]},
    '/api/library/open':inventory,
    '/api/render':{ok:true,html:'',tree,diagnostics:[],sourceInfo:{syntaxValid:true,identityStatus:'ordinary',types:[],headings:[],links:[],syntaxTokens:[]},dependencies:[],fontFamilies:[],compilerVersion:'injected outline browser fixture',serial:body.serial,source:body.source,headingStructure:{coverage:'complete',truncated:false,items:[heading]}},
    '/api/editor-map':{...map,sourceHash:currentHash,serial:body.serial},
    '/api/query/embedded/discover':{serial:body.serial,source:body.source,sourceHash:currentHash,syntaxValid:true,occurrences:[]},
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
 const paragraphText='Paragraph 45 before the chapter.';
 const paragraph=reader.getByText(paragraphText,{exact:true});
 // Scroll using the normal continuous reader, with no outline jump or text selection.
 await reader.evaluate(e=>{e.scrollTop=0;});
 await page.waitForFunction(()=>document.querySelector('.mode-read .flow-pane').scrollTop===0);
 const readerBox=await reader.boundingBox(), paragraphBox=await paragraph.boundingBox();
 const readingLine=readerBox.y+readerBox.height/3;
 await page.mouse.move(readerBox.x+readerBox.width/2,readingLine);
 await page.mouse.wheel(0,Math.round(paragraphBox.y-readingLine));
 await page.waitForFunction(text=>{
   const pane=document.querySelector('.mode-read .flow-pane');
   const p=[...pane.querySelectorAll('p')].find(e=>e.textContent===text);
   const viewport=pane.getBoundingClientRect();
   return Math.abs(p.getBoundingClientRect().top-(viewport.top+viewport.height/3))<6;
 },paragraphText);
 assert.equal(await paragraph.evaluate(e=>document.activeElement===e || !window.getSelection()?.isCollapsed),false);
 await edit.click();
 await page.locator('.mode-source .cm-content').waitFor();
 position=await sourcePosition();
 const paragraphStart=source.indexOf(paragraphText);
 assert.deepEqual([position.from,position.to],[paragraphStart,paragraphStart]);
 assert.equal(position.active,true);
 const change = spec => page.locator('.mode-source .cm-content').evaluate((e,spec)=>{
   const view=e.cmTile?.root?.view;
   if (view?.contentDOM !== e) throw new Error('Actual CodeMirror view is unavailable');
   view.dispatch({changes:spec,userEvent:'input.type'});
 },spec);
 const observedRender = expected => page.waitForResponse(async reply=>
    reply.url().endsWith('/api/render') && reply.request().postDataJSON()?.source===expected && reply.ok());
 const relativeTop=locator=>locator.evaluate(e=>{
   const range=document.createRange();
   range.selectNodeContents(e.firstChild);
   return range.getBoundingClientRect().top-e.closest('.flow-pane').getBoundingClientRect().top;
 });
 // Record the real viewport click position before layout changes, not a guessed offset.
 await edit.press('Enter');
 await reader.waitFor();
 await page.waitForFunction(text=>{
   const pane=document.querySelector('.mode-read .flow-pane');
   const p=[...pane.querySelectorAll('p')].find(e=>e.textContent===text);
   return p && Math.abs(p.getBoundingClientRect().top-(pane.getBoundingClientRect().top+pane.clientHeight/3))<6;
 },paragraphText);
 const originalTop=await relativeTop(paragraph);
 await edit.click();
 const insertion=Array.from({length:20},(_,i)=>`Inserted paragraph ${i+1} above the reading position.`).join('\n\n')+'\n\n';
 const added=observedRender(insertion+source);
 await change({from:0,insert:insertion});
 await added;
 await edit.press('Enter');
 await reader.getByText('Inserted paragraph 20 above the reading position.',{exact:true}).waitFor({state:'attached'});
 const expectParagraphAtOriginalTop = () => page.waitForFunction(({text,top})=>{
   const p=[...document.querySelectorAll('.mode-read .flow-pane p')].find(e=>e.textContent===text);
   if (!p) return false;
   const range=document.createRange();
   range.selectNodeContents(p.firstChild);
   return Math.abs(range.getBoundingClientRect().top-p.closest('.flow-pane').getBoundingClientRect().top-top)<3;
 },{text:paragraphText,top:originalTop});
 await expectParagraphAtOriginalTop();
 // Native editor history must move the same paragraph in both directions.
 await edit.click();
 const undone=observedRender(source);
 await page.locator('.mode-source .cm-content').press('ControlOrMeta+z');
 await undone;
 await edit.press('Enter');
 await reader.getByText('Inserted paragraph 20 above the reading position.',{exact:true}).waitFor({state:'detached'});
 await expectParagraphAtOriginalTop();
 await edit.click();
 const redone=observedRender(insertion+source);
 await page.locator('.mode-source .cm-content').press('ControlOrMeta+Shift+z');
 await redone;
 await edit.press('Enter');
 await reader.getByText('Inserted paragraph 20 above the reading position.',{exact:true}).waitFor({state:'attached'});
 await expectParagraphAtOriginalTop();
 const fallbackScroll=await reader.evaluate(e=>e.scrollTop);
 await edit.click();
 const fromDelete=insertion.length+paragraphStart;
 const toDelete=fromDelete+paragraphText.length+2;
 const afterDeletion=(insertion+source).slice(0,fromDelete)+(insertion+source).slice(toDelete);
 const deleted=observedRender(afterDeletion);
 await change({from:fromDelete,to:toDelete});
 await deleted;
 await edit.press('Enter');
 await paragraph.waitFor({state:'detached'});
 await page.waitForFunction(scroll=>Math.abs(document.querySelector('.mode-read .flow-pane').scrollTop-scroll)<2,fallbackScroll);
 // Return while the new render is held. A real wheel action must win over any
 // delayed restoration, even if the accepted projection still contains the run.
 await edit.click();
 const extra='An extra paragraph for the delayed render.\n\n';
 let release;
 holdRender={source:extra+afterDeletion,promise:new Promise(resolve=>{release=resolve;})};
 await change({from:0,insert:extra});
 await edit.press('Enter');
 await reader.locator('.preview-stale').waitFor({state:'attached'});
 await page.waitForFunction(scroll=>Math.abs(document.querySelector('.mode-read .flow-pane').scrollTop-scroll)<2,fallbackScroll);
 const waitingBox=await reader.boundingBox();
 await page.mouse.move(waitingBox.x+waitingBox.width/2,waitingBox.y+waitingBox.height/3);
 await page.mouse.wheel(0,-500);
 await page.waitForFunction(scroll=>document.querySelector('.mode-read .flow-pane').scrollTop<scroll-450,fallbackScroll);
 const userScroll=await reader.evaluate(e=>e.scrollTop);
 release();
 await reader.getByText('An extra paragraph for the delayed render.',{exact:true}).waitFor({state:'attached'});
 await reader.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
 assert.ok(Math.abs(await reader.evaluate(e=>e.scrollTop)-userScroll)<3,'a delayed accepted render reset an explicit wheel scroll');
 await page.setViewportSize({width:740,height:640});
 assert.equal(await edit.isVisible(),true);
 assert.equal(await edit.evaluate(e=>e.getBoundingClientRect().right<=window.innerWidth),true);
 assert.deepEqual(errors,[]);
 console.log('PASS actual workspace: direct edit uses focused heading, selected text, or the paragraph in the viewport; returning retains the same text after edits above it and falls back after deletion');
} finally {await browser.close(); await vite.close();}
