// Actual table controls with controlled native-map/controller facts. This checks
// routing and asynchronous ownership; test_tables_browser.mjs checks real Core
// transformations and shared author Undo on the selected candidate.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

assert.equal(process.argv.length, 3, 'Usage: node scripts/test_table_actions_components_browser.mjs ABSOLUTE_NEW_OUTPUT');
const repo = path.resolve(import.meta.dirname, '..'), out = path.resolve(process.argv[2]);
assert.ok(path.isAbsolute(process.argv[2]));
assert.ok(path.relative(repo, out).startsWith('..' + path.sep));
await mkdir(out, { recursive: false });
const inputs = ['app/src/table-grid.ts', 'app/src/table-clipboard.ts', 'app/src/writing.ts', 'app/src/ui-locale.ts', 'app/src/ui-messages.ts', 'scripts/test_table_actions_components_browser.mjs'];
const inventory = async () => Promise.all(inputs.map(async name => ({ path: name, sha256: createHash('sha256').update(await readFile(path.join(repo, name))).digest('hex') })));
const result = { schemaVersion: 1, sourceBefore: await inventory(), startedUtc: new Date().toISOString(), cases: [], pageErrors: [], completed: false,
    scope: 'Actual table controls, dialogs and event routing with controlled native-map/controller inputs',
    limits: ['No Core transformation, persistence or real shared Undo is qualified by this component check.', 'Composition uses a synthetic state gate, not real OS IME.'] };
const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
const require = createRequire(import.meta.url), { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
let vite, browser, page;
const button = name => page.getByRole('button', { name, exact: true });
const grid = () => page.getByRole('grid');
const intents = () => page.evaluate(() => window.fixture.intents);
const idle = () => page.locator('.writing-table-tools[aria-busy="false"]').waitFor();
async function check(name, run) { const item = { name, passed: false }; result.cases.push(item); await run(); item.passed = true; }
async function mount(options = {}) { await page.evaluate(options => window.mountTable(options), options); }
async function paste(html) { await grid().evaluate((element, html) => { const clipboardData = new DataTransfer(); clipboardData.setData('text/html', html); element.dispatchEvent(new ClipboardEvent('paste', { bubbles: true, cancelable: true, clipboardData })); }, html); }
try {
    vite = await createServer({ root: path.join(repo, 'app'), configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false, watch: { ignored: ['**/*'] } }, plugins: [{ name: 'owned-table-action-fixture', configureServer(server) { server.middlewares.use('/__table_action_check', (_request, response) => {
        response.setHeader('Content-Type', 'text/html; charset=utf-8');
        response.end(`<!doctype html><html><head><meta charset="utf-8"></head><body><main id="test"></main><script type="module">
        import {createTableGrid,disposeTableGrid} from '/src/table-grid.ts';
        import {writingRange} from '/src/writing.ts';
        import '/src/writing.css';
        window.mountTable=({header=false}={})=>{
            if(window.fixture)disposeTableGrid(window.fixture.element);
            const source='#table(columns:2, [甲], [乙], [丙], [丁])';
            const regions=['甲','乙','丙','丁'].map(text=>{const at=source.indexOf(text);return{range:writingRange(source,at,at+text.length),text,role:'tableCell',paragraphBreaks:[]};});
            const cells=regions.map((region,index)=>({row:Math.floor(index/2),column:index%2,rowspan:1,colspan:1,body:region.range,range:region.range,header:header&&index<2}));
            const table={columns:2,rows:2,cells:regions.map(region=>region.range),logicalCells:cells,gridWritable:true,topologyWritable:true,unavailableReason:null,headerRows:header?1:0,headerColumns:0};
            const structure={kind:'table',range:writingRange(source,0,source.length),table},snapshot={sessionId:'controlled-table-action',source,serial:0,generation:0};
            const fixture={snapshot,current:true,intents:[],errors:[],element:null,holdFlush:false,holdExecute:false,resumeFlush:null,resumeExecute:null};
            const controller={snapshot,map:{structures:[structure],literalRegions:regions,sourceHash:'controlled'},state:{composing:false,pending:false,busy:false},sourceSelection:{from:0,to:0},
                isSnapshotCurrent:basis=>fixture.current&&basis.sessionId===snapshot.sessionId&&basis.source===snapshot.source&&basis.serial===snapshot.serial&&basis.generation===snapshot.generation,
                selectRange:()=>{},surfaceStateChanged:()=>{},observeSurface:()=>()=>{},
                flush:async()=>{if(fixture.holdFlush){fixture.holdFlush=false;await new Promise(resolve=>fixture.resumeFlush=resolve);}},
                execute:async build=>{if(fixture.holdExecute){fixture.holdExecute=false;await new Promise(resolve=>fixture.resumeExecute=resolve);}fixture.intents.push(build());},
                bindLiteral:(input,region)=>{input.value=region.text;return()=>{};},
            };
            fixture.controller=controller;fixture.element=createTableGrid(controller,structure,{onError:error=>fixture.errors.push(error.message)});
            document.querySelector('#test').replaceChildren(fixture.element);window.fixture=fixture;
        };
        window.ready=true;
        </script></body></html>`);
    }); } }] });
    await vite.listen();
    browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE || process.env.WEFTEXT_CHROME || 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
    result.browserVersion = browser.version(); page = await browser.newPage({ viewport: { width: 1100, height: 900 } });
    page.on('pageerror', error => result.pageErrors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/__table_action_check`); await page.waitForFunction(() => window.ready);
    await check('explicit lossless merge, split and whole transpose dispatch once with no dialog', async () => {
        for (const [label, action] of [['合并所选格', { kind: 'merge', top: 0, left: 0, bottom: 0, right: 1 }], ['拆分当前格', { kind: 'split', row: 0, column: 0 }], ['整张表格转置', { kind: 'transpose' }]]) {
            await mount();
            if (action.kind === 'merge') await button('选择第 1 行').click();
            await button(label).click(); await idle();
            assert.equal(await page.getByRole('dialog').count(), 0); assert.deepEqual((await intents()).map(intent => intent.action), [action]);
            assert.ok(await button(label).getAttribute('aria-description'));
        }
    });
    await check('header-bearing whole transpose remains refused without a partial command', async () => {
        await mount({ header: true }); await button('整张表格转置').click(); await idle();
        assert.deepEqual(await intents(), []); assert.match(await page.getByRole('alert').innerText(), /转置前请明确取消表头/);
    });
    await check('lossless in-bounds paste dispatches once without a dialog', async () => {
        await mount(); await paste('<table><tr><td>新甲</td><td>新乙</td></tr></table>'); await idle();
        assert.equal(await page.getByRole('dialog').count(), 0); const actual = await intents(); assert.equal(actual.length, 1);
        assert.deepEqual(actual[0].action, { kind: 'pasteCells', row: 0, column: 0, rows: 1, columns: 2, cells: [{ row: 0, column: 0, rowspan: 1, colspan: 1, text: '新甲', header: false }, { row: 0, column: 1, rowspan: 1, colspan: 1, text: '新乙', header: false }], expand: false });
    });
    await check('table expansion remains an explicit cancellable choice', async () => {
        const html = '<table><tr><td>一</td></tr><tr><td>二</td></tr><tr><td>三</td></tr></table>';
        await mount(); await paste(html); const dialog = page.getByRole('dialog', { name: '粘贴表格范围', exact: true }); await dialog.waitFor();
        assert.deepEqual(await intents(), []); await dialog.getByRole('button', { name: '取消', exact: true }).click(); await idle(); assert.deepEqual(await intents(), []);
        await paste(html); await dialog.getByRole('button', { name: '扩展并粘贴', exact: true }).click(); await idle();
        assert.equal((await intents()).length, 1); assert.equal((await intents())[0].action.expand, true);
    });
    await check('HTML formatting loss still requires confirmation before writing', async () => {
        await mount(); await paste('<table><tr><td><strong>强调会降级</strong></td></tr></table>'); const dialog = page.getByRole('dialog', { name: '粘贴表格范围', exact: true }); await dialog.waitFor();
        assert.deepEqual(await intents(), []); await dialog.getByRole('button', { name: '取消', exact: true }).click(); await idle(); assert.deepEqual(await intents(), []);
    });
    await check('selection changes while a direct command awaits execution reject its original target', async () => {
        await mount(); await page.evaluate(() => window.fixture.holdExecute = true); await button('拆分当前格').click(); await page.waitForFunction(() => typeof window.fixture.resumeExecute === 'function');
        await grid().press('ArrowRight'); await page.evaluate(() => window.fixture.resumeExecute()); await idle();
        assert.deepEqual(await intents(), []); assert.match(await page.getByRole('alert').innerText(), /选区|变化|过期/);
    });
    await check('source retirement while a direct command awaits execution refuses all writes', async () => {
        await mount(); await page.evaluate(() => window.fixture.holdExecute = true); await button('整张表格转置').click(); await page.waitForFunction(() => typeof window.fixture.resumeExecute === 'function');
        await page.evaluate(() => { window.fixture.current = false; window.fixture.resumeExecute(); }); await idle(); assert.deepEqual(await intents(), []); await page.getByRole('alert').waitFor();
    });
    await check('composition beginning during settle cannot dispatch the pending structural command', async () => {
        await mount(); await page.evaluate(() => window.fixture.holdFlush = true); await button('拆分当前格').click(); await page.waitForFunction(() => typeof window.fixture.resumeFlush === 'function');
        await page.evaluate(() => { window.fixture.controller.state.composing = true; window.fixture.resumeFlush(); }); await idle(); assert.deepEqual(await intents(), []); assert.match(await page.getByRole('alert').innerText(), /输入法|组合/);
    });
    await check('selection changes behind an expansion preview invalidate its approval', async () => {
        await mount(); await paste('<table><tr><td>一</td><td>二</td><td>三</td></tr></table>'); const dialog = page.getByRole('dialog', { name: '粘贴表格范围', exact: true }); await dialog.waitFor();
        await page.evaluate(() => document.querySelector('.writing-table-grid').dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true })));
        await dialog.getByRole('button', { name: '扩展并粘贴', exact: true }).click(); await idle(); assert.deepEqual(await intents(), []); assert.match(await page.getByRole('alert').innerText(), /过期|变化/);
    });
    result.sourceAfter = await inventory(); assert.deepEqual(result.sourceAfter, result.sourceBefore); assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) {
    result.error = { message: error.message, stack: error.stack }; process.exitCode = 1;
    await page?.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {});
} finally {
    await browser?.close(); await vite?.close(); result.finishedUtc = new Date().toISOString();
    await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2), { flag: 'wx' });
    console.log(JSON.stringify({ completed: result.completed, cases: result.cases, error: result.error?.message }));
}
