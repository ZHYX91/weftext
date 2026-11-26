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
 for (const width of [1120, 740, 640, 390, 320]) {
 const page=await browser.newPage({ viewport:{width,height:760}, locale:'zh-CN' });
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
 const text=(key,str)=>{
   const start=source.indexOf(str),end=start+str.length;
   assert.ok(start>=0 && source.lastIndexOf(str)===start,'rendered fixture text must have one exact source origin');
   return {key,kind:'text',tag:null,text:str,children:[],origin:{path:doc.path,from:start,to:end,utf16From:start,utf16To:end,raw:str,syntaxKind:'text',isMain:true},writable:false,attrs:{}};
 };
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
 const ribbon = page.locator('.workspace-ribbon');
 if (width <= 780) await page.locator('.sidebar-overlay-close').click();
 const view = ribbon.locator('[data-tab="view"][role="tab"]');
 const trigger = ribbon.locator('.workspace-dock-controls [popovertarget="workspace-dock-view-list"]');
 const list = page.locator('#workspace-dock-view-list');
 const visible = () => list.evaluate(el => el.matches(':popover-open'));
 const clickExact = async locator => {
   const rect = await locator.boundingBox();
   assert.ok(rect);
   const center = { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
   assert.ok(await locator.evaluate((el, p) => el.contains(document.elementFromPoint(p.x, p.y)), center),
     `physical click target clipped at width ${width}: ${await locator.textContent()}`);
   await page.mouse.click(center.x,center.y);
 };
 await view.click();
 await trigger.scrollIntoViewIfNeeded();
 await clickExact(trigger);
 assert.equal(await visible(),true);
 const buttons=list.getByRole('button');
 assert.ok(await buttons.count() >= 8);
 for (const choice of await buttons.all()) {
   const rect=await choice.boundingBox();
   assert.ok(rect);
   assert.ok(await choice.evaluate((el,r)=>el.contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2)),rect),
     `choice clipped at width ${width}: ${await choice.textContent()}`);
 }
 await page.keyboard.press('Escape');
 assert.equal(await visible(),false);
 assert.equal(await trigger.evaluate(el=>el===document.activeElement),true);
 await page.keyboard.press('Enter');
 assert.equal(await visible(),true);
 assert.equal(await buttons.first().evaluate(el=>el===document.activeElement),true);
 await page.keyboard.press('Tab');
 assert.equal(await buttons.nth(1).evaluate(el=>el===document.activeElement),true);
 await page.keyboard.press('Escape');
 assert.equal(await visible(),false);
 assert.equal(await trigger.evaluate(el=>el===document.activeElement),true);
 await clickExact(trigger);
 await page.mouse.click(width - 10, 700);
 assert.equal(await visible(),false);
 if (width === 320) {
   await page.setViewportSize({width,height:325});
   await trigger.scrollIntoViewIfNeeded();
   await clickExact(trigger);
   const layout=await list.evaluate(el=>({bottom:el.getBoundingClientRect().bottom, scrollHeight:el.scrollHeight, clientHeight:el.clientHeight}));
   assert.ok(layout.bottom<=325 && layout.scrollHeight>layout.clientHeight);
   for (let index=1; index<await buttons.count(); index++) await page.keyboard.press('Tab');
   const last=buttons.last();
   assert.equal(await last.evaluate(el=>el===document.activeElement),true);
   const focused=await last.boundingBox();
   assert.ok(focused && focused.y<325 && focused.y+focused.height>0);
   await page.keyboard.press('Escape');
   assert.equal(await trigger.evaluate(el=>el===document.activeElement),true);
   await page.setViewportSize({width,height:760});
   await trigger.scrollIntoViewIfNeeded();
 }
 await clickExact(trigger);
 await ribbon.locator('.workspace-ribbon-toggle').click();
 assert.equal(await visible(),false);
 await view.click();
 await trigger.scrollIntoViewIfNeeded();
 await clickExact(trigger);
 await clickExact(list.locator('[data-show-view="query"]'));
 assert.equal(await visible(),false);
 await page.locator('#workspace-view-query').waitFor({state:'visible'});
 assert.equal(await page.locator('.mode-read .flow-document').count(),1);
 assert.deepEqual(errors,[]);
 console.log(`width ${width}: all choices hit, keyboard focus and dismiss, ribbon, Search, reading retained`);
 await page.close();
 }
} finally { await browser.close(); await vite.close(); }
