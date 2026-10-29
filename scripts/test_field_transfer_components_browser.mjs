// Actual FieldTransfer DOM with controlled host facts and readonly proposal
// responses. No Core, author writer, persistence or real Undo is qualified.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

assert.equal(process.argv.length, 3, 'Usage: node scripts/test_field_transfer_components_browser.mjs ABSOLUTE_NEW_OUTPUT');
const repo = path.resolve(import.meta.dirname, '..'), out = path.resolve(process.argv[2]);
assert.ok(path.isAbsolute(process.argv[2]));
assert.ok(path.relative(repo, out).startsWith('..' + path.sep));
await mkdir(out, { recursive: false });
const inputs = ['app/src/field-transfer.ts', 'app/src/field-transfer-state.ts', 'app/src/source-input-basis.ts', 'app/src/native-value-form.ts', 'app/src/data-projection.ts', 'app/src/ui-locale.ts', 'app/src/ui-messages.ts', 'assets/ui-messages.json', 'scripts/test_field_transfer_components_browser.mjs'];
const inventory = async () => Promise.all(inputs.map(async file => ({ path: file, sha256: createHash('sha256').update(await readFile(path.join(repo, file))).digest('hex') })));
const result = { schemaVersion: 1, startedUtc: new Date().toISOString(), sourceBefore: await inventory(), cases: [], pageErrors: [], completed: false,
    scope: 'Actual FieldTransfer form, selection routing and asynchronous host authority gates with controlled readonly responses',
    limits: ['Synthetic schema/projection and response data only; no Core transform, author write, save or shared Undo is qualified.', 'The mock applyPatch only records its patch and old basis, and never changes fixture author bytes or values.', 'IME and retired-control events are controlled event/state gates, not real OS IME acceptance.'] };
const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
const { chromium } = createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
let vite, browser, page;
const control = name => page.locator(`[data-transfer-control="${name}"]`);
const idle = () => page.locator('.data-field-transfer[aria-busy="false"]').waitFor();
async function mount(options = {}) { await page.evaluate(options => window.mountTransfer(options), options); }
async function selectRange() { await control('first').fill('2'); await control('last').fill('3'); await control('position').fill('2'); }
async function requestReady() { await page.waitForFunction(() => window.fixture.requests.length === 1 && typeof window.fixture.resolveRequest === 'function'); }
async function submit() { await selectRange(); await control('apply').click(); await requestReady(); }
async function releaseRequest() { await page.evaluate(() => window.fixture.releaseRequest()); await idle(); }
async function unchanged() {
    assert.equal(await page.evaluate(() => window.fixture.authorSource), await page.evaluate(() => window.fixture.originalSource));
    assert.deepEqual(await page.evaluate(() => window.fixture.authorValues), [["same", "same", "tail"], ["dest"], ["other"]]);
}
async function check(name, run) {
    const item = { name, passed: false }; result.cases.push(item);
    try { await run(); await unchanged(); item.passed = true; }
    catch (error) {
        item.error = String(error.stack ?? error);
        item.observed = await page.evaluate(() => window.fixture && ({ requests: window.fixture.requests, applies: window.fixture.applies, notifications: window.fixture.notifications, events: window.fixture.events, runActive: window.fixture.runActive,
            controls: Object.fromEntries([...window.fixture.component.element.querySelectorAll('[data-transfer-control]')].map(element => [element.dataset.transferControl, { value: element.value, disabled: element.disabled }])) })).catch(() => null);
        await page.screenshot({ path: path.join(out, `failure-${String(result.cases.length).padStart(2, '0')}.png`), fullPage: true }).catch(() => {});
    }
}
const source = '// 原件😀\r\n#metadata((id: "6f593c9c-1bf4-46d3-937d-81287fbd58c4", types: ("transfer/test",), from_values: ("same", "same", "tail"), to_values: ("dest",), other_values: ("other",), unknown_root: "保留")) <weftext>\r\n正文不改';
try {
    vite = await createServer({ root: path.join(repo, 'app'), configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false, watch: { ignored: ['**/*'] } }, plugins: [{ name: 'owned-field-transfer-fixture', configureServer(server) { server.middlewares.use('/__field_transfer_check', (_request, response) => {
        response.setHeader('Content-Type', 'text/html; charset=utf-8');
        response.end(`<!doctype html><html><head><meta charset="utf-8"><style>body{font:16px sans-serif;margin:20px;max-width:900px}label,input,select{display:block;margin:8px 0}input,select{min-height:30px}</style></head><body><main id="test"></main><script type="module">
        import {FieldTransfer} from '/src/field-transfer.ts';
        import {disposeUiTree,uiTextValue} from '/src/ui-locale.ts';
        const originalSource=${JSON.stringify(source)};
        const text=value=>({kind:'text',value}),array=values=>({state:'value',value:{kind:'array',value:values.map(text)}});
        const list={kind:'list',items:{kind:'scalar',semantic:'text'},min:0,max:null};
        const fields=[['transfer/from','from_values',['same','same','tail']],['transfer/into','to_values',['dest']],['transfer/other','other_values',['other']]];
        function context(){
            const schema={definitions:{},fields:fields.map(([id,key])=>({id,key,shape:structuredClone(list),nullable:false})),types:[{id:'transfer/test',fields:fields.map(([id])=>({id,required:false})),requires:[],conflicts:[]}],aliases:{}};
            return {basis:{inputId:'original-input',sessionId:'session',source:originalSource,serial:7,baseId:'base',path:'D:/owned/A/A.typ',root:'D:/owned',workspaceId:'workspace',nodeId:'node'},
                schemaRead:{schema,source:JSON.stringify(schema),revision:'schema-revision',semanticDigest:'a'.repeat(64),builtinDefault:false,canInstall:true},
                projection:{qualification:'qualified',id:'6f593c9c-1bf4-46d3-937d-81287fbd58c4',types:['transfer/test'],resolvedTypes:['transfer/test'],fields:fields.map(([fieldId,key,values])=>({fieldId,key,cell:array(values),writable:true,sourceRange:{from:1,to:2},raw:'static fixture list',unbound:[]})),unknownFields:[],diagnostics:[],budgetExhausted:false,readOnly:false}};
        }
        const same=(left,right)=>left&&right&&JSON.stringify(left.basis)===JSON.stringify(right.basis)&&left.schemaRead.source===right.schemaRead.source&&left.schemaRead.revision===right.schemaRead.revision&&left.schemaRead.semanticDigest===right.schemaRead.semanticDigest&&left.projection.readOnly===right.projection.readOnly&&JSON.stringify(left.projection.fields.map(field=>[field.fieldId,field.writable,field.unbound]))===JSON.stringify(right.projection.fields.map(field=>[field.fieldId,field.writable,field.unbound]));
        window.ownerStats={mounted:0,disposed:0};
        window.disposeTransfer=async()=>{
            const fixture=window.fixture;if(!fixture)return;
            fixture.component.retire();fixture.component.element.remove();
            fixture.resolveRequest?.(fixture.response());fixture.resolveApply?.();fixture.resolveFinal?.();
            for(let i=0;i<100&&fixture.runActive;i++)await new Promise(resolve=>setTimeout(resolve,0));
            if(fixture.runActive!==0)throw new Error('A controlled proposal owner did not settle');
            disposeUiTree(fixture.component.element);window.ownerStats.disposed++;window.fixture=null;
        };
        window.mountTransfer=async(options={})=>{
            await window.disposeTransfer();
            const fixture={context:context(),allowed:options.allowed!==false,current:true,requests:[],applies:[],notifications:[],events:[],runActive:0,holdApply:!!options.holdApply,holdFinal:!!options.holdFinal,resolveRequest:null,resolveApply:null,resolveFinal:null,mutation:options.mutation??'',authorSource:originalSource,originalSource,authorValues:[['same','same','tail'],['dest'],['other']]};
            fixture.response=()=>{
                const body=fixture.requests.at(-1)?.body;
                if(!body)return undefined;
                const from=body.source.indexOf('("same", "same", "tail")'),to=body.source.indexOf('("dest",)')+'("dest",)'.length,text='("same",), to_values: ("dest", "same", "tail",)';
                const wire={source:body.source.slice(0,from)+text+body.source.slice(to),patches:[{from,to,text,byteFrom:new TextEncoder().encode(body.source.slice(0,from)).length,byteTo:new TextEncoder().encode(body.source.slice(0,to)).length,expected:body.source.slice(from,to)}],patchUnit:'utf16',serial:body.serial,sourceHash:body.sourceHash,schemaRevision:body.schemaRevision,semanticDigest:body.semanticDigest,transfer:{fromField:body.fromField,toField:body.toField,start:body.start,end:body.end,to:body.to}};
                if(fixture.mutation==='multiple')wire.patches.push({...wire.patches[0]});
                if(fixture.mutation==='expected')wire.patches[0].expected='wrong original';
                if(fixture.mutation==='schema')wire.semanticDigest='b'.repeat(64);
                if(fixture.mutation==='echo')wire.transfer.start++;
                if(fixture.mutation==='source')wire.source+='unrelated edit';
                return wire;
            };
            fixture.releaseRequest=()=>{if(typeof fixture.resolveRequest!=='function')throw new Error('Original readonly request is not pending');const release=fixture.resolveRequest;fixture.resolveRequest=null;release(fixture.response());};
            const host={getContext:()=>fixture.context,isCurrent:original=>fixture.current&&same(original,fixture.context),canTransfer:()=>fixture.allowed,
                request:async(path,body)=>{fixture.events.push('request');fixture.requests.push({path,body:structuredClone(body),runActive:fixture.runActive});return new Promise(resolve=>fixture.resolveRequest=resolve);},
                applyPatch:async(patch,basis)=>{fixture.events.push('apply:start');fixture.applies.push({patch:structuredClone(patch),basis:structuredClone(basis),runActive:fixture.runActive});if(fixture.holdApply)await new Promise(resolve=>fixture.resolveApply=resolve);fixture.events.push('apply:end');},
                notify:(value,error=false)=>fixture.notifications.push({text:uiTextValue(value),error}),
                run:async operation=>{fixture.runActive++;fixture.events.push('run:start');try{await operation();fixture.events.push('refresh:start');if(fixture.holdFinal)await new Promise(resolve=>fixture.resolveFinal=resolve);fixture.events.push('refresh:end');}finally{fixture.runActive--;fixture.events.push('run:end');}}
            };
            fixture.component=new FieldTransfer(host);document.querySelector('#test').replaceChildren(fixture.component.element);fixture.component.refresh();window.fixture=fixture;window.ownerStats.mounted++;
        };
        window.ready=true;
        </script></body></html>`);
    }); } }] });
    await vite.listen();
    browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE || 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
    result.browserVersion = browser.version(); page = await browser.newPage({ viewport: { width: 1100, height: 1000 } }); page.setDefaultTimeout(30000);
    page.on('pageerror', error => result.pageErrors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/__field_transfer_check`); await page.waitForFunction(() => window.ready);
    await check('visible duplicate ordinals produce one readonly request and one patch against the exact old basis', async () => {
        await mount(); await submit();
        const request = await page.evaluate(() => window.fixture.requests[0]);
        assert.equal(request.path, '/api/data/transfer'); assert.equal(request.runActive, 1);
        assert.deepEqual({ fromField: request.body.fromField, toField: request.body.toField, start: request.body.start, end: request.body.end, to: request.body.to }, { fromField: 'transfer/from', toField: 'transfer/into', start: 1, end: 2, to: 1 });
        assert.match(await page.locator('.data-transfer-preview').innerText(), /第 2–3 项/);
        await releaseRequest();
        const fixture = await page.evaluate(() => ({ requests: window.fixture.requests, applies: window.fixture.applies, runActive: window.fixture.runActive }));
        assert.equal(fixture.requests.length, 1); assert.equal(fixture.applies.length, 1); assert.equal(fixture.applies[0].basis.source, source);
        assert.equal(fixture.applies[0].basis.inputId, 'original-input'); assert.equal(fixture.applies[0].runActive, 1); assert.equal(fixture.runActive, 0);
        assert.equal(fixture.requests.filter(request => request.path.includes('save')).length, 0);
    });
    await check('the real owner guard and busy gate span delayed request, patch delivery and final refresh', async () => {
        await mount({ holdApply: true, holdFinal: true }); await submit();
        await control('apply').evaluate(element => element.click()); assert.equal(await page.evaluate(() => window.fixture.requests.length), 1);
        await page.evaluate(() => window.fixture.releaseRequest()); await page.waitForFunction(() => typeof window.fixture.resolveApply === 'function');
        assert.equal(await page.evaluate(() => window.fixture.runActive), 1); assert.equal(await page.locator('.data-field-transfer').getAttribute('aria-busy'), 'true');
        await page.evaluate(() => window.fixture.resolveApply()); await page.waitForFunction(() => typeof window.fixture.resolveFinal === 'function');
        assert.equal(await page.evaluate(() => window.fixture.runActive), 1); assert.equal(await page.locator('.data-field-transfer').getAttribute('aria-busy'), 'true');
        await page.evaluate(() => window.fixture.resolveFinal()); await idle(); assert.equal(await page.evaluate(() => window.fixture.runActive), 0);
        assert.deepEqual(await page.evaluate(() => window.fixture.events), ['run:start', 'request', 'apply:start', 'apply:end', 'refresh:start', 'refresh:end', 'run:end']);
    });
    await check('unapplied authority blocks the actual form before any request', async () => {
        await mount({ allowed: false }); assert.equal(await control('apply').isDisabled(), true); await control('apply').evaluate(element => element.click());
        assert.deepEqual(await page.evaluate(() => window.fixture.requests), []); assert.deepEqual(await page.evaluate(() => window.fixture.applies), []);
    });
    await check('composition owns the actual ordinal input and prevents submission', async () => {
        await mount(); await control('first').evaluate(element => element.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true, data: '二' })));
        assert.equal(await control('apply').isDisabled(), true); await control('apply').evaluate(element => element.click()); assert.deepEqual(await page.evaluate(() => window.fixture.requests), []);
        await control('first').evaluate(element => element.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true, data: '' }))); assert.equal(await control('apply').isEnabled(), true);
    });
    for (const mutation of ['source', 'serial', 'input', 'base', 'scope', 'node', 'digest', 'revision', 'authority']) await check(`late ${mutation} change rejects the readonly response without applying either list`, async () => {
        await mount(); await submit();
        await page.evaluate(mutation => {
            const fixture = window.fixture; fixture.context = structuredClone(fixture.context);
            if (mutation === 'source') fixture.context.basis.source += 'new input';
            if (mutation === 'serial') fixture.context.basis.serial++;
            if (mutation === 'input') fixture.context.basis.inputId = 'new input';
            if (mutation === 'base') fixture.context.basis.baseId = 'new base';
            if (mutation === 'scope') fixture.context.basis.root = 'D:/different-library';
            if (mutation === 'node') fixture.context.basis.nodeId = 'new node';
            if (mutation === 'digest') fixture.context.schemaRead.semanticDigest = 'b'.repeat(64);
            if (mutation === 'revision') fixture.context.schemaRead.revision = 'new revision';
            if (mutation === 'authority') fixture.allowed = false;
        }, mutation);
        await releaseRequest(); assert.deepEqual(await page.evaluate(() => window.fixture.applies), []); assert.equal(await control('first').inputValue(), '2'); assert.equal(await control('last').inputValue(), '3');
    });
    await check('a changed ordinal event while transport is pending invalidates the captured selection', async () => {
        await mount(); await submit(); await control('first').evaluate(element => { element.value = '1'; element.dispatchEvent(new Event('input', { bubbles: true })); });
        await releaseRequest(); assert.deepEqual(await page.evaluate(() => window.fixture.applies), []); assert.equal(await control('first').inputValue(), '1');
    });
    await check('composition beginning after request dispatch prevents late proposal delivery', async () => {
        await mount(); await submit(); await control('last').evaluate(element => element.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true, data: '三' })));
        await releaseRequest(); assert.deepEqual(await page.evaluate(() => window.fixture.applies), []);
        await control('last').evaluate(element => element.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true, data: '' })));
    });
    for (const mutation of ['multiple', 'expected', 'schema', 'echo', 'source']) await check(`a malformed ${mutation} response cannot become a patch`, async () => {
        await mount({ mutation }); await submit(); await releaseRequest(); assert.deepEqual(await page.evaluate(() => window.fixture.applies), []); assert.equal(await page.getByRole('alert').isVisible(), true);
    });
    for (const mutation of ['retire', 'disconnect', 'refresh']) await check(`late response after ${mutation} never applies to another form owner`, async () => {
        await mount(); await submit(); await page.evaluate(mutation => {
            const fixture = window.fixture;
            if (mutation === 'retire') fixture.component.retire();
            if (mutation === 'disconnect') fixture.component.element.remove();
            if (mutation === 'refresh') { fixture.context = structuredClone(fixture.context); fixture.context.basis.inputId = 'replacement input'; fixture.component.refresh(); }
        }, mutation);
        await page.evaluate(() => window.fixture.releaseRequest()); await page.waitForFunction(() => window.fixture.runActive === 0);
        assert.deepEqual(await page.evaluate(() => window.fixture.applies), []);
    });
    await check('a benign same-input refresh retains the visible selection and remains usable', async () => {
        await mount(); await selectRange(); await page.evaluate(() => { window.fixture.context = structuredClone(window.fixture.context); window.fixture.component.refresh(); });
        assert.equal(await control('first').inputValue(), '2'); assert.equal(await control('last').inputValue(), '3'); assert.equal(await control('apply').isEnabled(), true);
        await control('apply').click(); await requestReady(); await releaseRequest(); assert.equal(await page.evaluate(() => window.fixture.applies.length), 1);
    });
    await check('retired ordinal controls cannot change the new visible selection or its request', async () => {
        await mount(); await page.evaluate(() => {
            const fixture = window.fixture, oldFirst = fixture.component.element.querySelector('[data-transfer-control="first"]'), oldLast = fixture.component.element.querySelector('[data-transfer-control="last"]');
            fixture.context = structuredClone(fixture.context); fixture.context.basis.inputId = 'new visible owner'; fixture.component.refresh();
            oldFirst.value = '2'; oldFirst.dispatchEvent(new Event('input', { bubbles: true })); oldLast.value = '3'; oldLast.dispatchEvent(new Event('input', { bubbles: true }));
        });
        assert.equal(await control('first').inputValue(), '1'); assert.equal(await control('last').inputValue(), '1');
        await control('apply').click(); await requestReady(); const request = await page.evaluate(() => window.fixture.requests[0].body);
        assert.equal(request.start, 0); assert.equal(request.end, 0);
        await page.evaluate(() => window.fixture.component.retire()); await releaseRequest(); assert.deepEqual(await page.evaluate(() => window.fixture.applies), []);
    });
    await check('retired destination selectors cannot retarget the new visible destination field', async () => {
        await mount(); await page.evaluate(() => {
            const fixture = window.fixture, oldTarget = fixture.component.element.querySelector('[data-transfer-control="toField"]');
            fixture.context = structuredClone(fixture.context); fixture.context.basis.inputId = 'new visible owner'; fixture.component.refresh();
            oldTarget.value = 'transfer/other'; oldTarget.dispatchEvent(new Event('change', { bubbles: true }));
        });
        assert.equal(await control('toField').inputValue(), 'transfer/into');
        await control('apply').click(); await requestReady(); assert.equal(await page.evaluate(() => window.fixture.requests[0].body.toField), 'transfer/into');
        await page.evaluate(() => window.fixture.component.retire()); await releaseRequest(); assert.deepEqual(await page.evaluate(() => window.fixture.applies), []);
    });
    await check('retiring an IME owner permits a fresh context without an old compositionend', async () => {
        await mount(); await control('first').evaluate(element => element.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true, data: '二' })));
        await page.evaluate(() => {
            const fixture = window.fixture; fixture.oldImeInput = fixture.component.element.querySelector('[data-transfer-control="first"]');
            fixture.component.retire(); fixture.component.element.remove(); fixture.context = structuredClone(fixture.context); fixture.context.basis.inputId = 'fresh context';
            document.querySelector('#test').append(fixture.component.element); fixture.component.refresh();
        });
        assert.equal(await page.evaluate(() => window.fixture.oldImeInput !== window.fixture.component.element.querySelector('[data-transfer-control="first"]')), true);
        assert.equal(await control('apply').isEnabled(), true); assert.deepEqual(await page.evaluate(() => window.fixture.requests), []);
    });
    await page.evaluate(() => window.disposeTransfer()); result.ownerStats = await page.evaluate(() => window.ownerStats);
    assert.equal(result.ownerStats.mounted, result.ownerStats.disposed);
    result.sourceAfter = await inventory(); result.inputsUnchanged = JSON.stringify(result.sourceAfter) === JSON.stringify(result.sourceBefore);
    result.completed = result.cases.every(item => item.passed) && result.pageErrors.length === 0 && result.inputsUnchanged;
} catch (error) { result.failure = String(error.stack ?? error); }
finally {
    result.cleanupErrors = [];
    try { if (page && !page.isClosed()) await page.evaluate(() => window.disposeTransfer?.()); } catch (error) { result.cleanupErrors.push(String(error)); }
    try { await browser?.close(); } catch (error) { result.cleanupErrors.push(String(error)); }
    try { await vite?.close(); } catch (error) { result.cleanupErrors.push(String(error)); }
    result.ownedProcessesClosed = (!browser || !browser.isConnected()) && result.cleanupErrors.length === 0;
    result.finishedUtc = new Date().toISOString(); await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2), { flag: 'wx' });
    if (!result.completed || !result.ownedProcessesClosed) process.exitCode = 1;
}
console.log(JSON.stringify({ completed: result.completed, cases: result.cases.length, failed: result.cases.filter(item => !item.passed).map(item => item.name), inputsUnchanged: result.inputsUnchanged, ownedProcessesClosed: result.ownedProcessesClosed, failure: result.failure }));
