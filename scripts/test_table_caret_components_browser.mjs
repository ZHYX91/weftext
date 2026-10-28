// Real browser hit testing over the actual table component with controlled
// native-map/controller inputs. This is not Core, IME, or desktop acceptance.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

assert.equal(process.argv.length, 3, 'Usage: node scripts/test_table_caret_components_browser.mjs ABSOLUTE_NEW_OUTPUT');
const repo = path.resolve(import.meta.dirname, '..'), out = path.resolve(process.argv[2]);
assert.ok(path.isAbsolute(process.argv[2]));
assert.ok(path.relative(repo, out).startsWith('..' + path.sep));
await mkdir(out, { recursive: false });
const inputs = ['app/src/table-grid.ts', 'app/src/table-cell-caret.ts', 'app/src/writing.ts', 'app/src/writing.css', 'app/src/ui-locale.ts', 'app/src/ui-messages.ts', 'assets/ui-messages.json', 'scripts/test_table_caret_components_browser.mjs'];
const inventory = async () => Promise.all(inputs.map(async name => ({ path: name, sha256: createHash('sha256').update(await readFile(path.join(repo, name))).digest('hex') })));
const result = { schemaVersion: 1, sourceBefore: await inventory(), startedUtc: new Date().toISOString(), cases: [], pageErrors: [], completed: false,
    scope: 'Actual table DOM, pointer hit test, focus and native textarea selection with synthetic map/controller facts only',
    limits: ['No Core source transformation, persistence, author Undo or native desktop operation is qualified here.', 'Composition coverage is a synthetic state gate, not real OS IME.'] };
const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
const require = createRequire(import.meta.url), { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
let vite, browser, page;
async function check(name, run) { const item = { name, passed: false }; result.cases.push(item); await run(); item.passed = true; }
async function mount(text) { await page.evaluate(text => window.mountTable(text), text); }
async function clickAt(offset) {
    const point = await page.locator('.table-cell-display').evaluate((display, at) => {
        const range = document.createRange(); range.setStart(display.firstChild, at); range.collapse(true);
        const rect = range.getBoundingClientRect();
        return { x: rect.left, y: rect.top + rect.height / 2 };
    }, offset);
    await page.mouse.click(point.x, point.y);
}
const input = () => page.locator('.table-cell-input');
try {
    vite = await createServer({ root: path.join(repo, 'app'), configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false, watch: { ignored: ['**/*'] } }, plugins: [{ name: 'owned-table-caret-fixture', configureServer(server) { server.middlewares.use('/__table_caret_check', (_request, response) => {
        response.setHeader('Content-Type', 'text/html; charset=utf-8');
        response.end(`<!doctype html><html><head><meta charset="utf-8"><style>body{margin:20px;font:16px sans-serif}#test{width:650px}</style></head><body><main id="test"></main><script type="module">
        import {createTableGrid,disposeTableGrid} from '/src/table-grid.ts';
        import {writingRange,encodeWritingTableLiteral} from '/src/writing.ts';
        import '/src/writing.css';
        window.mountTable=text=>{
            if(window.fixture)disposeTableGrid(window.fixture.element);
            const raw=encodeWritingTableLiteral(text),source='#table(columns:1, ['+raw+'])',from=source.indexOf('[')+1;
            const body=writingRange(source,from,from+raw.length),range=writingRange(source,0,source.length),region={range:body,text,role:'tableCell',paragraphBreaks:[]};
            const cell={row:0,column:0,rowspan:1,colspan:1,body,range:body,header:false};
            const table={columns:1,rows:1,cells:[body],logicalCells:[cell],gridWritable:true,topologyWritable:true,unavailableReason:null,headerRows:0,headerColumns:0};
            const structure={kind:'table',range,table},snapshot={sessionId:'controlled-table',source,serial:0,generation:0};
            const fixture={source,snapshot,current:true,inputs:[],errors:[],element:null,holdFlush:false,resume:null};
            const controller={snapshot,map:{structures:[structure],literalRegions:[region],sourceHash:'controlled'},state:{composing:false,pending:false,busy:false},sourceSelection:{from:0,to:0},
                isSnapshotCurrent:basis=>fixture.current&&basis.sessionId===snapshot.sessionId&&basis.source===snapshot.source&&basis.serial===snapshot.serial&&basis.generation===snapshot.generation,
                selectRange:()=>{},surfaceStateChanged:()=>{},observeSurface:()=>()=>{},
                flush:async()=>{if(fixture.holdFlush){fixture.holdFlush=false;await new Promise(resolve=>fixture.resume=resolve);}},
                bindLiteral:(input,region)=>{input.value=region.text;const read=()=>fixture.inputs.push(input.value);input.addEventListener('input',read);return()=>input.removeEventListener('input',read);},
            };
            fixture.controller=controller;fixture.element=createTableGrid(controller,structure,{onError:error=>fixture.errors.push(error.message)});
            document.querySelector('#test').replaceChildren(fixture.element);window.fixture=fixture;
        };
        window.ready=true;
        </script></body></html>`);
    }); } }] });
    await vite.listen();
    browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE || process.env.WEFTEXT_CHROME || 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
    result.browserVersion = browser.version(); page = await browser.newPage({ viewport: { width: 1000, height: 900 } });
    page.on('pageerror', error => result.pageErrors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/__table_caret_check`); await page.waitForFunction(() => window.ready);
    for (const [text, offset] of [['甲乙丙', 1], ['a  b', 3], ['甲\n乙丙', 3], ['甲😀乙', 3], ['同 同 同', 2]]) {
        await check(`actual pointer caret preserves exact UTF-16 position ${offset} in ${JSON.stringify(text)}`, async () => {
            await mount(text); await clickAt(offset); await input().waitFor({ state: 'visible' });
            assert.deepEqual(await input().evaluate(element => [element.selectionStart, element.selectionEnd]), [offset, offset]);
            await page.keyboard.insertText('新');
            assert.equal(await input().inputValue(), text.slice(0, offset) + '新' + text.slice(offset));
            assert.equal(await page.evaluate(() => window.fixture.inputs.at(-1)), text.slice(0, offset) + '新' + text.slice(offset));
        });
    }
    await check('empty placeholder enters an empty caret without selecting replacement text', async () => {
        await mount(''); await page.locator('.table-cell-display').click(); await input().waitFor({ state: 'visible' });
        assert.deepEqual(await input().evaluate(element => [element.selectionStart, element.selectionEnd]), [0, 0]);
        await page.keyboard.insertText('空格新文'); assert.equal(await input().inputValue(), '空格新文');
    });
    await check('unavailable browser hit testing falls back to the end, with an explicit all-text alternative', async () => {
        await mount('原文字😀');
        await page.evaluate(() => { for (const name of ['caretPositionFromPoint', 'caretRangeFromPoint']) Object.defineProperty(document, name, { configurable: true, value: () => null }); });
        await page.locator('.table-cell-display').click(); await input().waitFor({ state: 'visible' });
        assert.deepEqual(await input().evaluate(element => [element.selectionStart, element.selectionEnd]), [5, 5]);
        assert.match(await page.locator('.writing-table-surface > .writing-help').first().innerText(), /无法确认点击字符位置/);
        await page.keyboard.insertText('追加'); assert.equal(await input().inputValue(), '原文字😀追加');
        await mount('明确选择'); await page.getByRole('button', { name: '全选格内文字', exact: true }).click();
        assert.deepEqual(await input().evaluate(element => [element.selectionStart, element.selectionEnd]), [0, 4]);
        await page.keyboard.insertText('替换'); assert.equal(await input().inputValue(), '替换');
        await page.evaluate(() => { delete document.caretPositionFromPoint; delete document.caretRangeFromPoint; });
    });
    await check('a mismatched display never supplies a caret to the native input', async () => {
        await mount('真实原文'); await page.locator('.table-cell-display').evaluate(element => element.textContent = '不相符的展示文字');
        await page.locator('.table-cell-display').click(); await input().waitFor({ state: 'visible' });
        assert.deepEqual(await input().evaluate(element => [element.value, element.selectionStart, element.selectionEnd]), ['真实原文', 4, 4]);
    });
    await check('retirement during the original settle cannot authorize a captured click', async () => {
        await mount('保留文字'); await page.evaluate(() => window.fixture.holdFlush = true);
        await clickAt(1); await page.waitForFunction(() => typeof window.fixture.resume === 'function');
        await page.evaluate(() => { window.fixture.current = false; window.fixture.resume(); });
        await page.getByRole('alert').waitFor();
        assert.equal(await input().isHidden(), true); assert.deepEqual(await page.evaluate(() => window.fixture.inputs), []);
        assert.match(await page.getByRole('alert').innerText(), /来源|源|映射/);
    });
    await check('synthetic composition owns pointer entry and leaves the text unmodified', async () => {
        await mount('组合输入保留'); await page.evaluate(() => window.fixture.controller.state.composing = true);
        await page.locator('.table-cell-display').click();
        assert.equal(await input().isHidden(), true); assert.deepEqual(await page.evaluate(() => window.fixture.inputs), []);
    });
    result.sourceAfter = await inventory(); assert.deepEqual(result.sourceAfter, result.sourceBefore);
    assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) {
    result.error = { message: error.message, stack: error.stack }; process.exitCode = 1;
    await page?.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {});
} finally {
    await browser?.close(); await vite?.close(); result.finishedUtc = new Date().toISOString();
    await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2), { flag: 'wx' });
    console.log(JSON.stringify({ completed: result.completed, cases: result.cases, error: result.error?.message }));
}
