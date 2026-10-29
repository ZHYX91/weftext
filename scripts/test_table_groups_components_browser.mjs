// Real table controls with controlled map/controller facts. This checks grouped
// routing, selection and gesture ownership; it does not qualify Core or Undo.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

assert.equal(process.argv.length, 3, 'Usage: node scripts/test_table_groups_components_browser.mjs ABSOLUTE_NEW_OUTPUT');
const repo = path.resolve(import.meta.dirname, '..'), out = path.resolve(process.argv[2]);
assert.ok(path.isAbsolute(process.argv[2]));
assert.ok(path.relative(repo, out).startsWith('..' + path.sep));
await mkdir(out, { recursive: false });
const inputs = ['app/src/table-grid.ts', 'app/src/table-axis.ts', 'app/src/table-clipboard.ts', 'app/src/writing.ts', 'app/src/writing.css', 'app/src/ui-locale.ts', 'app/src/ui-messages.ts', 'scripts/test_table_groups_components_browser.mjs'];
const inventory = async () => Promise.all(inputs.map(async name => ({ path: name, sha256: createHash('sha256').update(await readFile(path.join(repo, name))).digest('hex') })));
const result = { schemaVersion: 1, sourceBefore: await inventory(), startedUtc: new Date().toISOString(), cases: [], pageErrors: [], completed: false,
    scope: 'Actual table group controls and trusted native drag gestures with controlled map/controller facts',
    limits: ['The controlled execute records one proposed operation without transforming source; Core, persistence and shared Undo are not qualified.', 'Composition and source retirement use controlled state gates; selection invalidation and Escape use the actual grid key handler with synthetic key events. No real OS IME is qualified.'] };
const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
const require = createRequire(import.meta.url), { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
let vite, browser, page;
const button = name => page.getByRole('button', { name, exact: true });
const grid = () => page.getByRole('grid');
const handle = (axis, index) => button(`选择第 ${index + 1} ${axis === 'row' ? '行' : '列'}`);
const intents = () => page.evaluate(() => window.fixture.intents);
const actions = async () => (await intents()).map(intent => intent.action);
const idle = () => page.locator('.writing-table-tools[aria-busy="false"]').waitFor();
async function check(name, run) { const item = { name, passed: false }; result.cases.push(item); await run(); item.passed = true; }
async function mount() { await page.evaluate(() => window.mountTable()); }
async function selected(axis) { return grid().locator(`[data-table-axis="${axis}"][aria-pressed="true"]`).evaluateAll(elements => elements.map(element => Number(element.dataset.tableIndex))); }
async function group(axis, start, end) {
    await handle(axis, start).click(); await idle();
    if (end !== start) { await handle(axis, end).click({ modifiers: ['Shift'] }); await idle(); }
    assert.deepEqual(await selected(axis), Array.from({ length: Math.abs(end - start) + 1 }, (_, i) => Math.min(start, end) + i));
}
async function drag(axis, source, target, mutation = '') {
    await page.evaluate(mutation => window.fixture.dragMutation = mutation, mutation);
    await handle(axis, source).dragTo(handle(axis, target)); await idle();
    const events = await page.evaluate(() => window.fixture.dragEvents);
    assert.ok(events.some(event => event.type === 'dragstart' && event.trusted), 'The browser must start a trusted native drag');
    if (!mutation) assert.ok(events.some(event => event.type === 'drop' && event.trusted), 'The browser must deliver a trusted native drop');
    else {
        const mutations = await page.evaluate(() => window.fixture.dragMutationLog);
        assert.equal(mutations.length, 1); assert.equal(mutations[0].kind, mutation);
        if (mutation === 'selection') assert.notEqual(mutations[0].beforeActive, mutations[0].afterActive, 'The invalidating key must actually change the active cell');
    }
    return events;
}
try {
    vite = await createServer({ root: path.join(repo, 'app'), configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false, watch: { ignored: ['**/*'] } }, plugins: [{ name: 'owned-table-groups-fixture', configureServer(server) { server.middlewares.use('/__table_groups_check', (_request, response) => {
        response.setHeader('Content-Type', 'text/html; charset=utf-8');
        response.end(`<!doctype html><html><head><meta charset="utf-8"></head><body><main id="test"></main><script type="module">
        import {createTableGrid,disposeTableGrid} from '/src/table-grid.ts';
        import {writingRange} from '/src/writing.ts';
        import '/src/writing.css';
        window.ownerStats={mounted:0,disposed:0,observers:0,bindings:0};
        window.unmountTable=()=>{if(window.fixture){disposeTableGrid(window.fixture.element);window.fixture.element.remove();window.fixture=null;window.ownerStats.disposed++;}};
        window.mountTable=()=>{
            window.unmountTable();
            const texts=Array.from({length:20},(_,index)=>'单元'+index),source='#table(columns:4, '+texts.map(text=>'['+text+']').join(', ')+')';
            const regions=texts.map(text=>{const at=source.indexOf('['+text+']')+1;return{range:writingRange(source,at,at+text.length),text,role:'tableCell',paragraphBreaks:[]};});
            const cells=regions.map((region,index)=>({row:Math.floor(index/4),column:index%4,rowspan:1,colspan:1,body:region.range,range:region.range,header:false}));
            const table={columns:4,rows:5,cells:regions.map(region=>region.range),logicalCells:cells,gridWritable:true,topologyWritable:true,unavailableReason:null,headerRows:0,headerColumns:0};
            const structure={kind:'table',range:writingRange(source,0,source.length),table},snapshot={sessionId:'controlled-table-groups',source,serial:0,generation:0};
            const fixture={snapshot,current:true,intents:[],errors:[],element:null,holdFlush:false,holdExecute:false,resumeFlush:null,resumeExecute:null,dragEvents:[],dragMutation:'',dragMutationLog:[]};
            const controller={snapshot,map:{structures:[structure],literalRegions:regions,sourceHash:'controlled-groups'},state:{composing:false,pending:false,busy:false},sourceSelection:{from:0,to:0},
                isSnapshotCurrent:basis=>fixture.current&&basis.sessionId===snapshot.sessionId&&basis.source===snapshot.source&&basis.serial===snapshot.serial&&basis.generation===snapshot.generation,
                selectRange:()=>{},surfaceStateChanged:()=>{},observeSurface:()=>{window.ownerStats.observers++;return()=>window.ownerStats.observers--;},
                flush:async()=>{if(fixture.holdFlush){fixture.holdFlush=false;await new Promise(resolve=>fixture.resumeFlush=resolve);}},
                execute:async build=>{if(fixture.holdExecute){fixture.holdExecute=false;await new Promise(resolve=>fixture.resumeExecute=resolve);}fixture.intents.push(build());},
                bindLiteral:(input,region)=>{input.value=region.text;window.ownerStats.bindings++;let active=true;return()=>{if(active){active=false;window.ownerStats.bindings--;}};},
            };
            fixture.controller=controller;fixture.element=createTableGrid(controller,structure,{onError:error=>fixture.errors.push(error.message)});
            for(const type of ['dragstart','drop','dragend'])fixture.element.addEventListener(type,event=>{
                fixture.dragEvents.push({type,trusted:event.isTrusted});
                if(type==='dragstart'&&fixture.dragMutation)queueMicrotask(()=>{
                    const grid=fixture.element.querySelector('[role="grid"]');
                    const beforeActive=grid.getAttribute('aria-activedescendant');
                    if(fixture.dragMutation==='retire')fixture.current=false;
                    else if(fixture.dragMutation==='selection')grid.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowLeft',bubbles:true,cancelable:true}));
                    else if(fixture.dragMutation==='composition')controller.state.composing=true;
                    else if(fixture.dragMutation==='escape')grid.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true,cancelable:true}));
                    fixture.dragMutationLog.push({kind:fixture.dragMutation,beforeActive,afterActive:grid.getAttribute('aria-activedescendant')});
                });
            });
            document.querySelector('#test').replaceChildren(fixture.element);window.fixture=fixture;window.ownerStats.mounted++;
        };
        window.remountSameTable=()=>{
            const fixture=window.fixture,controller=fixture.controller,map=controller.map,snapshot=controller.snapshot,source=snapshot.source;
            disposeTableGrid(fixture.element);fixture.element.remove();window.ownerStats.disposed++;
            fixture.element=createTableGrid(controller,map.structures[0],{onError:error=>fixture.errors.push(error.message)});
            document.querySelector('#test').replaceChildren(fixture.element);window.ownerStats.mounted++;
            return{sameController:fixture.controller===controller,sameMap:controller.map===map,sameSnapshot:controller.snapshot===snapshot,sameSource:controller.snapshot.source===source};
        };
        window.ready=true;
        </script></body></html>`);
    }); } }] });
    await vite.listen();
    browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE || process.env.WEFTEXT_CHROME || 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
    result.browserVersion = browser.version(); page = await browser.newPage({ viewport: { width: 1440, height: 1200 } });
    page.on('pageerror', error => result.pageErrors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/__table_groups_check`); await page.waitForFunction(() => window.ready);
    await check('Shift handle selection routes two rows or columns to one grouped move and restores the entire selection', async () => {
        for (const [axis, label, action, restored] of [
            ['row', '行下移', { kind: 'moveRows', start: 1, end: 2, to: 2 }, [2, 3]],
            ['row', '行上移', { kind: 'moveRows', start: 1, end: 2, to: 0 }, [0, 1]],
            ['column', '列右移', { kind: 'moveColumns', start: 1, end: 2, to: 2 }, [2, 3]],
            ['column', '列左移', { kind: 'moveColumns', start: 1, end: 2, to: 0 }, [0, 1]],
        ]) {
            await mount(); await group(axis, 1, 2);
            assert.match(await button(label).getAttribute('title'), /2/); assert.match(await button(label).getAttribute('aria-description'), /2/);
            await button(label).click(); await idle(); assert.deepEqual(await actions(), [action]); assert.deepEqual(await selected(axis), restored);
            assert.equal(await page.getByRole('dialog').count(), 0);
        }
    });
    await check('group insertion and removal route once with the complete count or inclusive interval', async () => {
        for (const [axis, label, action] of [
            ['row', '上方加行', { kind: 'insertRows', index: 1, count: 2 }],
            ['row', '下方加行', { kind: 'insertRows', index: 3, count: 2 }],
            ['column', '左侧加列', { kind: 'insertColumns', index: 1, count: 2 }],
            ['column', '右侧加列', { kind: 'insertColumns', index: 3, count: 2 }],
            ['row', '删除所选行', { kind: 'removeRows', start: 1, end: 2 }],
            ['column', '删除所选列', { kind: 'removeColumns', start: 1, end: 2 }],
        ]) { await mount(); await group(axis, 1, 2); await button(label).click(); await idle(); assert.deepEqual(await actions(), [action]); }
    });
    await check('Shift Arrow on axis handles extends row and column groups through keyboard controls', async () => {
        for (const [axis, key] of [['row', 'Shift+ArrowDown'], ['column', 'Shift+ArrowRight']]) {
            await mount(); await group(axis, 1, 1); await handle(axis, 1).focus(); await handle(axis, 1).press(key); await idle();
            assert.deepEqual(await selected(axis), [1, 2]); assert.deepEqual(await actions(), []);
        }
    });
    await check('same-controller source-stable remount preserves the axis anchor for continued Shift group expansion', async () => {
        result.remountRoutes = [];
        for (const axis of ['row', 'column']) {
            await mount(); await group(axis, 1, 2);
            const identity = await page.evaluate(() => window.remountSameTable());
            assert.deepEqual(identity, { sameController: true, sameMap: true, sameSnapshot: true, sameSource: true });
            assert.deepEqual(await selected(axis), [1, 2]);
            await handle(axis, 3).click({ modifiers: ['Shift'] }); await idle();
            assert.deepEqual(await selected(axis), [1, 2, 3]); assert.deepEqual(await actions(), []);
            result.remountRoutes.push({ axis, identity, restored: [1, 2], expanded: [1, 2, 3] });
        }
    });
    await check('trusted native drag moves complete row and column groups in both directions using final-start coordinates', async () => {
        result.dragRoutes = [];
        for (const [axis, start, end, target, to] of [['row', 1, 2, 4, 3], ['row', 2, 3, 0, 0], ['column', 1, 2, 3, 2], ['column', 2, 3, 0, 0]]) {
            await mount(); await group(axis, start, end); const events = await drag(axis, start, target);
            assert.deepEqual(await actions(), [{ kind: axis === 'row' ? 'moveRows' : 'moveColumns', start, end, to }]);
            assert.deepEqual(await selected(axis), [to, to + 1]); result.dragRoutes.push({ axis, start, end, target, to, events });
        }
    });
    await check('source retirement or selection changes after native drag start invalidate its frozen target', async () => {
        for (const mutation of ['retire', 'selection']) {
            await mount(); await group('row', 1, 2); await drag('row', 1, 4, mutation); assert.deepEqual(await actions(), []);
            assert.ok((await page.evaluate(() => window.fixture.dragEvents)).some(event => event.type === 'dragstart' && event.trusted));
        }
    });
    await check('composition or Escape after native drag start prevents every grouped write', async () => {
        for (const mutation of ['composition', 'escape']) {
            await mount(); await group('row', 1, 2); await drag('row', 1, 4, mutation); assert.deepEqual(await actions(), []);
        }
    });
    await check('selection changes while a grouped command awaits execution reject the frozen interval', async () => {
        await mount(); await group('row', 1, 2); await page.evaluate(() => window.fixture.holdExecute = true); await button('行下移').click();
        await page.waitForFunction(() => typeof window.fixture.resumeExecute === 'function'); await grid().press('ArrowLeft'); assert.deepEqual(await selected('row'), []); await page.evaluate(() => window.fixture.resumeExecute()); await idle();
        assert.deepEqual(await actions(), []); assert.match(await page.getByRole('alert').innerText(), /选区|变化|过期/);
    });
    await check('composition beginning during grouped-command settle refuses the entire operation', async () => {
        await mount(); await group('column', 1, 2); await page.evaluate(() => window.fixture.holdFlush = true); await button('删除所选列').click();
        await page.waitForFunction(() => typeof window.fixture.resumeFlush === 'function'); await page.evaluate(() => { window.fixture.controller.state.composing = true; window.fixture.resumeFlush(); }); await idle();
        assert.deepEqual(await actions(), []); assert.match(await page.getByRole('alert').innerText(), /输入法|组合/);
    });
    await check('composition blocks handle expansion without changing the selected group or routing writes', async () => {
        await mount(); await group('row', 1, 1); await handle('row', 1).focus(); await page.evaluate(() => window.fixture.controller.state.composing = true);
        await handle('row', 1).press('Shift+ArrowDown'); assert.deepEqual(await selected('row'), [1]); assert.deepEqual(await actions(), []);
    });
    result.ownerStats = await page.evaluate(() => { window.unmountTable(); return window.ownerStats; });
    assert.equal(result.ownerStats.observers, 0); assert.equal(result.ownerStats.bindings, 0); assert.equal(result.ownerStats.mounted, result.ownerStats.disposed);
    result.sourceAfter = await inventory(); assert.deepEqual(result.sourceAfter, result.sourceBefore); assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) {
    result.error = { message: error.message, stack: error.stack }; process.exitCode = 1;
    await page?.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {});
    if (page) result.failureFixture = await page.evaluate(() => window.fixture ? { intents: window.fixture.intents, errors: window.fixture.errors, dragEvents: window.fixture.dragEvents, dragMutationLog: window.fixture.dragMutationLog } : null).catch(() => null);
} finally {
    await browser?.close(); await vite?.close(); result.finishedUtc = new Date().toISOString();
    result.ownedProcessesClosed = (!browser || !browser.isConnected()) && (!vite?.httpServer || !vite.httpServer.listening) && (!page || page.isClosed());
    await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2), { flag: 'wx' });
    console.log(JSON.stringify({ out, completed: result.completed, cases: result.cases, ownedProcessesClosed: result.ownedProcessesClosed, error: result.error?.message }));
}
