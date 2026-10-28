// Real WorkPanel component in an owned headless browser. Synthetic caller facts,
// no Core/backend, desktop input, native IME, reader or complete task acceptance.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

assert.equal(process.argv.length, 3, 'Usage: node scripts/test_work_panel_browser.mjs ABSOLUTE_NEW_OUTPUT');
assert.ok(path.isAbsolute(process.argv[2]), 'Output must be absolute');
const repo = path.resolve(import.meta.dirname, '..'), out = path.resolve(process.argv[2]);
assert.ok(path.relative(repo, out).startsWith('..' + path.sep), 'Keep private evidence outside the product repository');
await mkdir(out, { recursive: false });
const inputs = ['app/src/work-panel.ts', 'app/src/work-panel-state.ts', 'app/src/work-panel.css', 'app/src/ui-locale.ts', 'app/src/ui-messages.ts', 'assets/ui-messages.json', 'app/package.json', 'app/package-lock.json', 'scripts/test_work_panel_browser.mjs'];
const inventory = async () => Promise.all(inputs.map(async name => { const bytes = await readFile(path.join(repo, name)); return { path: name, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') }; }));
const result = { schemaVersion: 1, startedUtc: new Date().toISOString(), frontend: 'current Vite source, isolated WorkPanel component', developmentOnly: true, cases: [], pageErrors: [], completed: false,
    limits: ['Synthetic caller diagnostics/tasks/results exercise the real presentation component only.', 'No Core/backend, native desktop input, actual OS IME or assistive-technology acceptance.', 'No import, export, recognition, sync or durable author operation is executed by this component check.'], sourceBefore: await inventory() };
const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
const require = createRequire(import.meta.url), { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
let server, browser;
async function check(name, run) { const entry = { name, passed: false }; result.cases.push(entry); await run(); entry.passed = true; }
try {
    server = await createServer({ root: path.join(repo, 'app'), configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false, watch: { ignored: ['**/*'] } }, plugins: [{ name: 'owned-work-panel-fixture', configureServer(server) { server.middlewares.use('/__work_panel_check', (_request, response) => {
        response.setHeader('Content-Type', 'text/html; charset=utf-8');
        response.end(`<!doctype html><html><head><meta charset="utf-8"><style>html,body{margin:0;height:100%;font:14px sans-serif}.test-workspace{height:100%;display:flex;flex-direction:column}#author{flex:1;min-height:80px;margin:12px}footer{height:28px;display:flex;align-items:center;border-top:1px solid #ddd}</style></head><body><main class="test-workspace"><textarea id="author" aria-label="Author source"></textarea><footer></footer></main><script type="module">import {WorkPanel} from '/src/work-panel.ts';import {locale} from '/src/ui-locale.ts';import '/src/work-panel.css';const panel=new WorkPanel({storage:localStorage,notify:text=>{throw new Error(String(text))}});panel.setContext('work-panel-fixture:A');document.querySelector('footer').before(panel.element);document.querySelector('footer').append(panel.statusButton);window.fixture={panel,locale};window.fixtureReady=true;</script></body></html>`);
    }); } }] });
    await server.listen();
    browser = await chromium.launch({ executablePath: process.env.WEFTEXT_CHROME || 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
    result.browserVersion = browser.version();
    const context = await browser.newContext({ viewport: { width: 1100, height: 850 } }), page = await context.newPage();
    page.on('pageerror', error => result.pageErrors.push(error.message));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/__work_panel_check`);
    await page.waitForFunction(() => window.fixtureReady);
    const panel = page.locator('.work-panel'), author = page.getByRole('textbox', { name: 'Author source' });
    await check('automatic facts retain author focus and a collapsed panel', async () => {
        await author.fill('中文输入保留 😀');
        await page.evaluate(() => {
            const panel = window.fixture.panel;
            panel.selectProblemScope('doc:A');
            panel.setProblems('doc:A', [{ id: 'error:A', severity: 'error', title: 'Synthetic compile error', actions: [{ id: 'jump', label: 'Jump to exact source', run: () => { window.jumped = 'doc:A:12'; } }] }]);
            panel.reportTask({ id: 'original-operation:A', title: 'Synthetic import', status: 'waiting', actions: [{ id: 'original', label: 'Inspect original operation', run: () => { window.original = 'A'; } }] });
            panel.addResult({ id: 'result:A', title: 'Synthetic generated file', detail: 'fixture-output.pdf' });
        });
        assert.equal(await author.evaluate(element => element === document.activeElement), true);
        assert.equal(await author.inputValue(), '中文输入保留 😀');
        assert.equal(await panel.isVisible(), false);
        assert.equal(await panel.getAttribute('inert'), '');
        assert.equal(await page.getByRole('tab').count(), 0);
        assert.match(await page.locator('.work-panel-status-button').innerText(), /1.*1.*1/);
    });
    await check('original task updates retain the same focused action and waiting semantics', async () => {
        await page.evaluate(() => window.fixture.panel.reveal('tasks'));
        assert.equal(await author.evaluate(element => element === document.activeElement), true);
        const action = page.getByRole('button', { name: 'Inspect original operation', exact: true });
        await action.focus(); await action.evaluate(element => { window.originalAction = element; });
        await page.evaluate(() => window.fixture.panel.reportTask({ id: 'original-operation:A', title: 'Synthetic import updated', detail: 'Result pending confirmation', status: 'waiting', actions: [{ id: 'original', label: 'Inspect original operation', run: () => { window.original = 'A-updated'; } }] }));
        assert.equal(await action.evaluate(element => element === window.originalAction && element === document.activeElement), true);
        assert.equal(await panel.locator('.work-panel-item[data-id="original-operation:A"]').getAttribute('data-state'), 'waiting');
        await action.click(); await page.waitForFunction(() => window.original === 'A-updated');
        await page.evaluate(() => window.fixture.panel.finishTask('original-operation:A', 'cancelled'));
        assert.match(await page.locator('.work-panel-status-button').innerText(), /1.*0.*1/);
        assert.equal(await panel.locator('.work-panel-item[data-id="original-operation:A"]').getAttribute('data-state'), 'cancelled');
    });
    await check('problems follow an exact document scope and supplied source action', async () => {
        await author.focus();
        await page.evaluate(() => { window.fixture.panel.selectProblemScope('doc:B'); window.fixture.panel.reveal('problems'); });
        assert.equal(await author.evaluate(element => element === document.activeElement), true);
        assert.equal(await panel.locator('.work-panel-content:not([hidden]) .work-panel-item').count(), 0);
        await page.evaluate(() => window.fixture.panel.selectProblemScope('doc:A'));
        await page.getByRole('button', { name: 'Jump to exact source', exact: true }).click();
        await page.waitForFunction(() => window.jumped === 'doc:A:12');
    });
    await check('height keyboard and pointer controls retain bounded per-context geometry', async () => {
        const separator = page.getByRole('separator');
        await separator.focus();
        const initial = (await panel.boundingBox()).height;
        await separator.press('ArrowUp');
        assert.equal((await panel.boundingBox()).height, initial + 12);
        await separator.press('End');
        assert.ok((await panel.boundingBox()).height <= 850 * .6);
        await separator.press('Enter');
        const before = (await panel.boundingBox()).height, box = await separator.boundingBox();
        await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2); await page.mouse.down();
        await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2 - 30); await page.mouse.up();
        const after = (await panel.boundingBox()).height;
        assert.equal(after, before + 30);
        await page.evaluate(() => window.fixture.panel.setContext('work-panel-fixture:B'));
        assert.equal(await panel.isVisible(), false);
        await page.evaluate(() => window.fixture.panel.setContext('work-panel-fixture:A'));
        assert.equal((await panel.boundingBox()).height, after);
        const preferences = await page.evaluate(() => JSON.parse(localStorage.getItem('weftext-work-panel:v1:' + encodeURIComponent('work-panel-fixture:A'))));
        assert.deepEqual(Object.keys(preferences).sort(), ['collapsed', 'height', 'selected']);
        assert.equal(JSON.stringify(preferences).includes('Synthetic'), false);
    });
    await check('tab roving keys respect composition and collapse removes panel focus targets', async () => {
        const tasks = panel.locator('.work-panel-tab[data-tab="tasks"]');
        await tasks.focus(); await tasks.press('ArrowRight');
        assert.equal(await page.evaluate(() => window.fixture.panel.selected), 'results');
        await panel.locator('.work-panel-tab[data-tab="results"]').evaluate(element => element.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', isComposing: true, bubbles: true, cancelable: true })));
        assert.equal(await page.evaluate(() => window.fixture.panel.selected), 'results');
        await panel.locator('.work-panel-collapse').click();
        assert.equal(await panel.isVisible(), false);
        assert.equal(await page.locator('.work-panel-status-button').evaluate(element => element === document.activeElement), true);
        assert.equal(await page.getByRole('tab').count(), 0);
        await author.focus(); await author.press('Tab');
        assert.equal(await page.locator('.work-panel-status-button').evaluate(element => element === document.activeElement), true);
    });
    await check('localized labels and existing facts remain consistent on reopening', async () => {
        await page.evaluate(() => { window.fixture.locale.set('en-US'); window.fixture.panel.reveal('tasks'); });
        assert.match(await page.locator('.work-panel-status-button').innerText(), /^Work panel:/);
        assert.equal(await page.getByRole('tab', { name: 'Tasks (1)', exact: true }).count(), 1);
        assert.match(await panel.locator('.work-panel-content:not([hidden])').innerText(), /Cancelled/);
        await page.screenshot({ path: path.join(out, 'work-panel.png'), fullPage: true });
        await page.evaluate(() => window.fixture.panel.dispose());
        assert.equal(await page.locator('.work-panel').count(), 0);
        assert.equal(await page.locator('.work-panel-status-button').count(), 0);
    });
    result.sourceAfter = await inventory();
    assert.deepEqual(result.sourceAfter, result.sourceBefore, 'The component source must stay fixed during its check');
    assert.deepEqual(result.pageErrors, []);
    result.completed = true;
} catch (error) { result.error = { message: error.message, stack: error.stack }; process.exitCode = 1; }
finally {
    await browser?.close(); await server?.close();
    result.finishedUtc = new Date().toISOString();
    await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2), { flag: 'wx' });
    console.log(JSON.stringify({ completed: result.completed, cases: result.cases, error: result.error?.message }));
}
