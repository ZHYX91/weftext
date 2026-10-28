// Real native forms and DataPanel with a bounded JSON host; no Core writes.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

assert.equal(process.argv.length, 3, 'Usage: node scripts/test_property_order_components_browser.mjs ABSOLUTE_NEW_OUTPUT');
assert.ok(path.isAbsolute(process.argv[2]));
const repo = path.resolve(import.meta.dirname, '..'), out = path.resolve(process.argv[2]);
assert.ok(path.relative(repo, out).startsWith('..' + path.sep), 'Evidence belongs outside the product repository');
await mkdir(out, { recursive: false });
const inputs = ['app/src/native-value-form.ts', 'app/src/native-control-identity.ts', 'app/src/data-panel.ts', 'app/src/data-field-preference.ts', 'app/src/data-projection-read.ts', 'app/src/data-view.css', 'app/src/ui-locale.ts', 'app/src/ui-messages.ts', 'app/package.json', 'app/package-lock.json', 'scripts/test_property_order_components_browser.mjs'];
const inventory = async () => Promise.all(inputs.map(async name => { const bytes = await readFile(path.join(repo, name)); return { path: name, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') }; }));
const result = { startedUtc: new Date().toISOString(), developmentOnly: true,
  scope: 'Current Vite native form and DataPanel; real DOM focus and keyboard, bounded exact-source/schema projection host.',
  limits: ['No actual Core transform or durable save, native desktop, real OS IME or assistive-technology acceptance.', 'One bounded host patch is not proof of the full author Undo stack.'],
  cases: [], pageErrors: [], completed: false, sourceBefore: await inventory() };
const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
const require = createRequire(import.meta.url), { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
let server, browser;
async function check(name, run) { const entry = { name, passed: false }; result.cases.push(entry); await run(); entry.passed = true; }
try {
  const fixtureHtml = `<!doctype html><meta charset="utf-8"><link rel="stylesheet" href="/src/data-view.css"><style>body{margin:0;font:14px sans-serif}main{display:flex;gap:20px;padding:12px}#native{width:400px}.data-panel{width:410px;height:780px}</style><main><section id="native"></section><section id="panel"></section></main><script type="module">
      import {valueEditor} from '/src/native-value-form.ts';import {DataPanel} from '/src/data-panel.ts';import {locale,uiTextValue,message} from '/src/ui-locale.ts';
      const text=value=>({kind:'text',value}),emptySchema={definitions:{},fields:[],types:[],aliases:{}};
      const fixture={changes:[],calls:[],patches:[],notifications:[],locale,uiTextValue,message};window.fixture=fixture;
      fixture.native=(integers=false)=>{fixture.changes=[];const items=integers?{kind:'scalar',semantic:'int'}:{kind:'scalar',semantic:'text'};
        const initial={kind:'array',value:integers?[{kind:'int',value:'1'},{kind:'int',value:'2'},{kind:'int',value:'3'}]:[text('first'),text('second'),text('third')]};
        const editor=valueEditor({kind:'list',items,min:0,max:null},initial,emptySchema,value=>fixture.changes.push(structuredClone(value)),'TEST native field',{fieldId:'fixture/list',path:[]});document.querySelector('#native').replaceChildren(editor);fixture.nativeElement=editor;};
      fixture.panel=async()=>{fixture.calls=[];fixture.patches=[];fixture.notifications=[];
        const basis={sessionId:'fixture-session',source:'TEST exact author source',serial:1,baseId:'fixture-base',path:'D:/property-component/Note/Note.typ',root:'D:/property-component',inputId:'input-1'};
        const definitions=[['fixture/title','title','text',true],['fixture/names','names','list',false],['fixture/phones','phones','list',false],['fixture/priority','priority','int',false],['fixture/invalid','invalidValue','int',false],['fixture/imported','legacyRecord','record',false]];
        const shapeFor=kind=>kind==='list'?{kind:'list',items:{kind:'scalar',semantic:'text'},min:0,max:null}:kind==='record'?{kind:'record',members:{number:{shape:{kind:'scalar',semantic:'phone'},required:true,nullable:false}},unknown:'preserve'}:{kind:'scalar',semantic:kind};
        const schema={definitions:{},aliases:{},fields:definitions.map(([id,key,kind])=>({id,key,shape:shapeFor(kind),nullable:false})),
          types:[{id:'fixture/person',requires:[],conflicts:[],fields:definitions.map(([id,_key,_kind,required])=>({id,required}))}]};
        const schemaSource='TEST schema source',values={title:text('TEST title'),names:{kind:'array',value:[text('重复😀 first'),text('重复😀 second'),text('third')]},phones:{kind:'array',value:[text('00138'),text('00138')]},priority:{kind:'int',value:'1'},legacyRecord:{kind:'record',value:[{key:'number',value:text('00138')},{key:'legacy',value:text('TEST unbound preserved member')}]}};
        const digest=async value=>[...new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(value)))].map(n=>n.toString(16).padStart(2,'0')).join('');
        const host={getDocument:()=>basis,canEdit:()=>true,notify:(text,error)=>fixture.notifications.push({text:uiTextValue(text),error:!!error}),openDocument:async()=>{throw Error('No navigation in this component')},
          applyPatch:async(patch,original)=>{if(original.source!==basis.source||original.serial!==basis.serial)throw Error('stale bounded patch');fixture.patches.push(structuredClone(patch));basis.source=basis.source.slice(0,patch.from)+patch.text+basis.source.slice(patch.to);basis.serial++;basis.inputId='input-'+basis.serial;},
          request:async(route,body)=>{fixture.calls.push({route,body:JSON.parse(JSON.stringify(body))});if(route==='/api/data/schema')return{schema,source:schemaSource,revision:null,builtinDefault:false,canInstall:false};
            if(route==='/api/data/project')return{serial:body.serial,sourceHash:await digest(body.source),schemaHash:await digest(schemaSource),schemaRevision:null,readOnly:false,projection:{qualification:'qualified',id:null,types:['fixture/person'],resolvedTypes:['fixture/person'],unknownFields:[],diagnostics:[],budgetExhausted:false,fields:definitions.map(([fieldId,key,_kind,required])=>({fieldId,key,cell:key==='invalidValue'?{state:'invalid',value:null,diagnostics:[]}:{state:'value',value:values[key]},sourceRange:null,raw:null,writable:true,required,unbound:key==='legacyRecord'?['legacy']:[]}))}};
            if(route==='/api/data/edit'){values[body.key]=structuredClone(body.value);return{from:0,to:basis.source.length,text:JSON.stringify(body.value)};}
            throw Error('Unexpected bounded route '+route);}};
        fixture.basis=basis;fixture.owner=new DataPanel(host);document.querySelector('#panel').replaceChildren(fixture.owner.element);await fixture.owner.refresh();fixture.panelReady=true;};
      fixture.native();window.fixtureReady=true;
    </script>`;
  const fixtureModule=fixtureHtml.match(/<script type="module">([\s\S]*?)<\/script>/)?.[1];assert.ok(fixtureModule);
  const syntax=spawnSync(process.execPath,['--input-type=module','--check'],{input:fixtureModule,encoding:'utf8',windowsHide:true});
  assert.equal(syntax.status,0,syntax.stderr || syntax.error?.message || 'Inline fixture module must parse');
  server = await createServer({ root: path.join(repo, 'app'), configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false, watch: { ignored: ['**/*'] } }, plugins: [{ name: 'property-order-fixture', configureServer(vite) { vite.middlewares.use('/__property_order_check', (_request, response) => {
    response.setHeader('Content-Type', 'text/html; charset=utf-8');response.end(fixtureHtml);
  }); } }] });
  await server.listen();
  browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE || 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
  result.browserVersion = browser.version();
  const page = await browser.newPage({ viewport: { width: 1050, height: 900 } });
  page.on('pageerror', error => result.pageErrors.push(error.message));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/__property_order_check`);
  await page.waitForFunction(() => window.fixtureReady);
  const native = page.locator('#native');
  const translated = key => page.evaluate(key => window.fixture.uiTextValue(window.fixture.message(key)), key);
  const row = index => native.locator('.native-list-item').nth(index);
  const advanced = async index => { const details = row(index).locator('.native-list-order'); if (!await details.evaluate(node => node.open)) await details.locator('summary').click(); return details; };
  await check('first, last and exact position move the precise repeated item and retain actual keyboard focus', async () => {
    await (await advanced(2)).locator('[data-order-action=first]').click();
    assert.deepEqual(await page.evaluate(() => window.fixture.changes.at(-1).value.map(item => item.value)), ['third', 'first', 'second']);
    assert.equal(await row(0).evaluate(node => node.contains(document.activeElement)), true);
    await (await advanced(0)).locator('[data-order-action=last]').click();
    assert.deepEqual(await page.evaluate(() => window.fixture.changes.at(-1).value.map(item => item.value)), ['first', 'second', 'third']);
    const position = (await advanced(2)).locator('[data-order-action=position]');await position.fill('2');await position.press('Enter');
    assert.deepEqual(await page.evaluate(() => window.fixture.changes.at(-1).value.map(item => item.value)), ['first', 'third', 'second']);
    assert.equal(await row(1).locator('[data-order-action=position]').evaluate(node => node === document.activeElement), true);
  });
  await check('invalid native input and invalid target remain exact and prevent a destructive reorder', async () => {
    await page.evaluate(() => window.fixture.native(true));
    const editor = row(0).locator('.native-value-editor > input');await editor.fill('-');
    const original = await editor.elementHandle();await (await advanced(0)).locator('[data-order-action=last]').click();
    assert.equal(await editor.inputValue(), '-');assert.equal(await editor.evaluate(node => node === document.activeElement), true);
    assert.equal(await original.evaluate(node => node.isConnected), true);assert.equal(await page.evaluate(() => window.fixture.changes.length), 0);
    await editor.fill('1');const position=(await advanced(0)).locator('[data-order-action=position]');await position.fill('2.5');await position.press('Enter');
    assert.equal(await position.inputValue(), '2.5');assert.equal(await position.getAttribute('aria-invalid'), 'true');
    assert.equal(await page.evaluate(() => window.fixture.changes.length), 1, 'only completing the scalar input changed the local value');
  });
  await check('composition guards and language changes preserve the same form without implicit moves', async () => {
    await page.evaluate(() => window.fixture.native());
    const editor=row(0).locator('textarea');await editor.focus();const original=await editor.elementHandle();
    await editor.evaluate(node=>node.dispatchEvent(new CompositionEvent('compositionstart',{bubbles:true,data:'中'})));
    await (await advanced(2)).locator('[data-order-action=first]').click();assert.equal(await page.evaluate(()=>window.fixture.changes.length),0);
    await editor.evaluate(node=>node.dispatchEvent(new CompositionEvent('compositionend',{bubbles:true,data:'中'})));
    await page.evaluate(()=>window.fixture.locale.set('en-US'));assert.equal(await original.evaluate(node=>node.isConnected),true);
    await page.evaluate(()=>window.fixture.locale.set('zh-CN'));
  });
  await check('field placement and sorting retain actual editor nodes, text and selection without author requests', async () => {
    await page.evaluate(() => window.fixture.panel());await page.waitForFunction(()=>window.fixture.panelReady);
    const field=key=>page.locator(`.data-property[data-field-key="${key}"]`);
    const names=field('names').locator('textarea').first();await names.fill('未应用😀 draft');await names.focus();await names.evaluate(node=>node.setSelectionRange(1,4));
    const original=await names.elementHandle();
    await field('names').locator('.data-field-placement > summary').click();await field('names').locator('[data-field-placement=top]').click();
    assert.equal(await original.evaluate(node=>node.isConnected&&node===document.querySelector('.data-property[data-field-key="names"] textarea')),true);
    await names.focus();await names.evaluate(node=>node.setSelectionRange(1,4));
    await page.evaluate(()=>{const select=document.querySelector('#property-presentation-sort');select.value='name';select.dispatchEvent(new Event('change',{bubbles:true}));});
    assert.deepEqual(await names.evaluate(node=>[node.value,node.selectionStart,node.selectionEnd,node===document.activeElement]),['未应用😀 draft',1,4,true]);
    await field('names').locator('.data-field-placement > summary').click();await field('names').locator('[data-field-placement=hidden]').click();assert.equal(await field('names').isVisible(),true,'unapplied field remains discoverable');
    if(!await field('legacyRecord').isVisible())await page.locator('.data-field-group.advanced > summary').click();
    for(const key of ['title','invalidValue','legacyRecord']){await field(key).locator('.data-field-placement > summary').click();await field(key).locator('[data-field-placement=hidden]').click();assert.equal(await field(key).isVisible(),true);}
    assert.equal(await field('legacyRecord').locator('.native-unknown').isVisible(),true,'a value-state record with Core unbound members must expose the preserved unknown-member notice');
    assert.match(await field('legacyRecord').locator('.native-unknown').innerText(),/TEST unbound preserved member/);
    await field('phones').locator('.data-field-placement > summary').click();const phone=await field('phones').locator('textarea').first().elementHandle();await field('phones').locator('[data-field-placement=hidden]').click();assert.equal(await field('phones').isVisible(),false);
    await page.locator('.data-field-presentation-settings > summary').click();await page.locator('#property-presentation-hidden').check();assert.equal(await field('phones').isVisible(),true);assert.equal(await phone.evaluate(node=>node.isConnected),true);
    assert.equal(await page.evaluate(()=>window.fixture.calls.filter(call=>call.route==='/api/data/edit').length),0);assert.deepEqual(await page.evaluate(()=>window.fixture.patches),[]);
    await page.locator('#panel').screenshot({path:path.join(out,'field-presentation.png')});
  });
  await check('reordering applies one exact native field value through one bounded patch', async () => {
    const names=page.locator('.data-property[data-field-key="names"]');
    const details=names.locator('.native-list-item').nth(2).locator('.native-list-order');await details.locator('summary').click();await details.locator('[data-order-action=first]').click();
    await names.getByRole('button',{name:await translated('queryEdit.applyDraft'),exact:true}).click();
    await page.waitForFunction(()=>window.fixture.patches.length===1);
    const writes=await page.evaluate(()=>window.fixture.calls.filter(call=>call.route==='/api/data/edit'));
    assert.equal(writes.length,1);assert.equal(writes[0].body.key,'names');assert.deepEqual(writes[0].body.path,[]);
    assert.deepEqual(writes[0].body.value.value.map(item=>item.value),['third','未应用😀 draft','重复😀 second']);
    assert.equal(await page.evaluate(()=>window.fixture.notifications.some(item=>item.error)),false);
  });
  result.sourceAfter=await inventory();assert.deepEqual(result.sourceAfter,result.sourceBefore);assert.deepEqual(result.pageErrors,[]);result.completed=true;
} catch(error) { result.error={message:error.message,stack:error.stack};process.exitCode=1; }
finally { await browser?.close();await server?.close();result.finishedUtc=new Date().toISOString();await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2),{flag:'wx'});console.log(JSON.stringify({completed:result.completed,cases:result.cases,error:result.error?.message})); }
