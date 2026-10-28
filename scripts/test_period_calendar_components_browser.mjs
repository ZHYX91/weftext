// Actual native date controls and CalendarPanel ownership with controlled
// schema/resolve/creation responses. This is not Core or desktop acceptance.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

assert.equal(process.argv.length, 3, 'Usage: node scripts/test_period_calendar_components_browser.mjs ABSOLUTE_NEW_OUTPUT');
const repo = path.resolve(import.meta.dirname, '..'), out = path.resolve(process.argv[2]);
assert.ok(path.isAbsolute(process.argv[2]));
assert.ok(path.relative(repo, out).startsWith('..' + path.sep));
await mkdir(out, { recursive: false });
const inputs = ['app/src/period-navigation.ts', 'app/src/period-navigation.css', 'app/src/workspace-tools.ts', 'app/src/query-panel.ts', 'app/src/ui-locale.ts', 'app/src/ui-messages.ts', 'assets/ui-messages.json', 'scripts/test_period_calendar_components_browser.mjs'];
const inventory = async () => Promise.all(inputs.map(async name => ({ path: name, sha256: createHash('sha256').update(await readFile(path.join(repo, name))).digest('hex') })));
const result = { schemaVersion: 1, sourceBefore: await inventory(), startedUtc: new Date().toISOString(), cases: [], pageErrors: [], completed: false,
    scope: 'Actual CalendarPanel DOM, native date badInput, owner close/departure and controlled original-operation delivery',
    limits: ['The host schema, date resolution and committed creation receipts are controlled component facts.', 'No Core calendar calculation, author persistence, OS IME or native desktop acceptance is claimed.'] };
const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
const require = createRequire(import.meta.url), { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
let server, browser, page;
async function check(name, run) { const item = { name, passed: false }; result.cases.push(item); await run(); item.passed = true; }
const date = () => page.locator('#calendar input[type="date"]');
const series = () => page.locator('#calendar input[list]');
const kind = () => page.locator('#calendar .period-navigation-form > label select');
const lookup = () => page.locator('#calendar .period-date-form button[type="submit"]');
async function mount() { await page.evaluate(() => window.mountCalendar()); }
async function choose(value = 'day') { await series().fill('journal'); await kind().selectOption(value); await date().fill('2021-01-01'); }
async function settled(count) { await page.waitForFunction(count => window.fixture.requests.filter(item => item.route.endsWith('/resolve')).length === count && !window.fixture.owner.loading, count); }
try {
    server = await createServer({ root: path.join(repo, 'app'), configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false, watch: { ignored: ['**/*'] } } });
    await server.listen();
    browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE || process.env.WEFTEXT_CHROME || 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
    result.browserVersion = browser.version();
    page = await browser.newPage({ viewport: { width: 900, height: 900 }, locale: 'en-US' });
    page.on('pageerror', error => result.pageErrors.push(error.message));
    await page.route('**/__period_calendar_check', route => route.fulfill({ contentType: 'text/html', body: '<!doctype html><html lang="en-US"><meta charset="utf-8"><link rel="stylesheet" href="/src/data-view.css"><link rel="stylesheet" href="/src/period-navigation.css"><style>body{margin:12px;font:14px sans-serif}#calendar{width:330px;height:700px;border:1px solid #aaa}</style><textarea id="author" aria-label="Author source"></textarea><button id="try-close">Close workspace</button><button id="try-switch">Switch library</button><main id="calendar"></main></html>' }));
    await page.goto('http://127.0.0.1:' + server.httpServer.address().port + '/__period_calendar_check');
    await page.evaluate(async () => {
        const [{ CalendarPanel }, { uiTextValue, errorUiText }] = await Promise.all([import('/src/period-navigation.ts'), import('/src/ui-locale.ts')]);
        const native = (kind, value) => ({ kind, value });
        const record = value => native('record', Object.entries(value).map(([key, value]) => ({ key, value })));
        const cases = {
            day: ['2021-01-01', '2021-01-01', '2021-01-02'],
            week: ['2020-W53', '2020-12-28', '2021-01-04'],
            month: ['2021-01', '2021-01-01', '2021-02-01'],
            quarter: ['2021-Q1', '2021-01-01', '2021-04-01'],
            year: ['2021', '2021-01-01', '2022-01-01'],
        };
        window.mountCalendar = async () => {
            const fixture = { panel: null, requests: [], notices: [], delivered: [], environments: [], hold: false, release: null, coverage: 'complete', created: false, closeBlocked: 0, closed: false, switchErrors: [], author: '作者原稿 😀', schema: {
                fields: [{ id: 'calendar/period', key: 'period', nullable: false, shape: { kind: 'record', members: {}, unknown: 'preserve' } }], types: [], definitions: {}, aliases: {},
                series_policies: [{ key: 'journal', kinds: Object.keys(cases), scope: { kind: 'library' }, multiplicity: 'unique' }, { key: 'daily-only', kinds: ['day'], scope: { kind: 'library' }, multiplicity: 'many' }],
                providers: [{ id: 'calendar', implementation: { id: 'table', abi: 1 }, data: { key: 'controlled', version: 'one', years: [1, 2] } }],
            } };
            const response = body => {
                const [key, start, end] = cases[body.kind];
                return { period: record({ key: native('text', key) }), bounds: record({ start: native('date', start), end_exclusive: native('date', end) }), policy: { multiplicity: 'unique' }, matches: fixture.created ? [{ key: 'created', entityKey: 'weftext-node://component/created', path: '/library/created.typ', cells: {}, readOnly: false }] : [], coverage: fixture.coverage, canCreate: fixture.coverage === 'complete' && !fixture.created };
            };
            fixture.panel = new CalendarPanel({
                request: async (route, body) => {
                    fixture.requests.push({ route, body: structuredClone(body) });
                    if (route.endsWith('/schema')) return { schema: structuredClone(fixture.schema) };
                    if (route.endsWith('/resolve')) {
                        const output = response(body);
                        if (fixture.hold) { fixture.hold = false; await new Promise(resolve => fixture.release = resolve); }
                        return output;
                    }
                    if (route.endsWith('/create')) { fixture.created = true; return { status: 'committed', operationId: body.operationId, document: { path: '/library/created.typ', sessionId: 'controlled-created-session', source: body.body } }; }
                    throw new Error('Unexpected component request ' + route);
                },
                getDocument: () => null, applyPatch: async () => { throw new Error('Calendar lookup must not patch the author'); },
                openDocument: async () => {}, notify: text => fixture.notices.push(uiTextValue(text)),
                environmentChanged: root => { fixture.environments.push(root); fixture.panel.environmentChanged(root); },
                created: async (document, operationId) => fixture.delivered.push({ document, operationId }),
            });
            Object.defineProperty(fixture, 'owner', { get: () => fixture.panel.current.navigator });
            document.querySelector('#calendar').replaceChildren(fixture.panel.element);
            document.querySelector('#author').value = fixture.author;
            document.querySelector('#try-close').onclick = () => { if (fixture.panel.freezeForClose()) fixture.closed = true; else fixture.closeBlocked++; };
            document.querySelector('#try-switch').onclick = () => { try { fixture.panel.setLibrary({ root: '/other', workspaceId: 'other' }, null); } catch (error) { fixture.switchErrors.push(uiTextValue(errorUiText(error))); } };
            fixture.panel.setLibrary({ root: '/library', workspaceId: 'original' }, '/library/folder');
            await fixture.panel.refresh();
            window.fixture = fixture;
        };
        await window.mountCalendar();
    });
    await check('real incomplete date buffer blocks close and library departure until explicitly cleared', async () => {
        await date().focus();
        await date().press('ArrowLeft'); await date().press('ArrowLeft'); await date().press('ArrowLeft'); await date().press('1');
        assert.deepEqual(await date().evaluate(input => ({ value: input.value, badInput: input.validity.badInput, valid: input.validity.valid })), { value: '', badInput: true, valid: false });
        await date().evaluate(input => window.originalDate = input);
        await page.locator('#try-close').click(); await page.locator('#try-switch').click();
        assert.deepEqual(await page.evaluate(() => [window.fixture.closed, window.fixture.closeBlocked, window.fixture.switchErrors.length, window.fixture.panel.current.root]), [false, 1, 1, '/library']);
        assert.equal(await date().evaluate(input => input === window.originalDate && input.isConnected), true);
        await date().fill('');
        await page.locator('#try-close').click();
        assert.equal(await page.evaluate(() => window.fixture.closed), true);
    });
    for (const selected of ['day', 'week', 'month', 'quarter', 'year']) {
        await check('actual ' + selected + ' controls resolve a date read-only and retain the Core-returned key', async () => {
            await mount(); await choose(selected); await lookup().click(); await settled(1);
            const state = await page.evaluate(() => ({ requests: window.fixture.requests, key: window.fixture.owner.key.value, author: document.querySelector('#author').value, result: window.fixture.owner.result }));
            const request = state.requests.find(item => item.route.endsWith('/resolve')).body;
            assert.equal(request.date, '2021-01-01'); assert.equal(request.kind, selected); assert.equal(Object.hasOwn(request, 'key'), false);
            assert.equal(state.key, { day: '2021-01-01', week: '2020-W53', month: '2021-01', quarter: '2021-Q1', year: '2021' }[selected]);
            assert.ok(state.result); assert.equal(state.requests.filter(item => item.route.endsWith('/create')).length, 0); assert.equal(state.author, '作者原稿 😀');
            assert.match(await page.locator('.period-lookup-result').innerText(), /不含结束日/);
        });
    }
    await check('installed series choices retain an unsupported kind without a hidden lookup or silent replacement', async () => {
        await mount(); await choose('week'); await series().fill('daily-only');
        assert.deepEqual(await page.locator('#calendar datalist option').evaluateAll(options => options.map(option => option.value)), ['journal', 'daily-only']);
        assert.equal(await kind().inputValue(), 'week');
        await lookup().click();
        assert.equal(await page.evaluate(() => window.fixture.requests.filter(item => item.route.endsWith('/resolve')).length), 0);
        assert.equal(await kind().inputValue(), 'week');
        assert.match(await page.locator('.data-warning').innerText(), /不支持所选周期/);
    });
    await check('late date resolution preserves the later input, author focus and an empty current result', async () => {
        await mount(); await choose(); await page.evaluate(() => window.fixture.hold = true);
        await lookup().click(); await page.waitForFunction(() => typeof window.fixture.release === 'function');
        await date().fill('2021-01-02'); await page.locator('#author').focus();
        await page.evaluate(() => window.fixture.release()); await settled(1);
        assert.equal(await date().inputValue(), '2021-01-02');
        assert.deepEqual(await page.evaluate(() => [window.fixture.owner.result, document.activeElement.id, window.fixture.requests.filter(item => item.route.endsWith('/create')).length]), [null, 'author', 0]);
    });
    await check('provider-only schema refresh retires old bounds while preserving the exact date control and raw input', async () => {
        await mount(); await choose('week'); await lookup().click(); await settled(1);
        await date().evaluate(input => window.originalDate = input);
        await page.evaluate(async () => { window.fixture.schema.providers[0].data.version = 'two'; await window.fixture.panel.refresh(); });
        assert.equal(await page.evaluate(() => window.fixture.owner.result), null);
        assert.doesNotMatch(await page.locator('.period-lookup-result').innerText(), /2020-12-28/);
        assert.equal(await date().inputValue(), '2021-01-01');
        assert.equal(await date().evaluate(input => input === window.originalDate), true);
    });
    await check('partial results stay read-only and complete lookup enables one explicit creation with its original operation', async () => {
        await mount(); await choose('week'); await page.evaluate(() => window.fixture.coverage = 'partial');
        await lookup().click(); await settled(1);
        const create = page.locator('.period-create-form > button');
        await page.locator('.period-create-form > summary').click();
        assert.equal(await create.isDisabled(), true);
        await page.evaluate(() => window.fixture.coverage = 'complete'); await lookup().click(); await settled(2);
        await page.getByLabel('新周期笔记名称', { exact: true }).fill('Created-period');
        await page.getByLabel('周期笔记原生 Typst 正文', { exact: true }).fill('显式新建正文 😀');
        await create.click();
        await page.waitForFunction(() => window.fixture.delivered.length === 1 && !window.fixture.owner.loading);
        const state = await page.evaluate(() => ({ writes: window.fixture.requests.filter(item => item.route.endsWith('/create')), delivered: window.fixture.delivered, author: document.querySelector('#author').value }));
        assert.equal(state.writes.length, 1);
        assert.equal(state.writes[0].body.key, '2020-W53'); assert.equal(Object.hasOwn(state.writes[0].body, 'date'), false);
        assert.equal(state.delivered[0].operationId, state.writes[0].body.operationId);
        assert.equal(state.delivered[0].document.source, '显式新建正文 😀'); assert.equal(state.author, '作者原稿 😀');
    });
    await page.locator('#calendar').evaluate(element => element.style.width = '264px');
    await page.screenshot({ path: path.join(out, 'calendar-264.png') });
    assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) {
    result.error = String(error?.stack ?? error); process.exitCode = 1;
    if (page) await page.screenshot({ path: path.join(out, 'failure.png') }).catch(() => {});
} finally {
    await browser?.close(); await server?.close(); result.ownedProcessesClosed = true;
    result.sourceAfter = await inventory(); result.inputsUnchanged = JSON.stringify(result.sourceBefore) === JSON.stringify(result.sourceAfter);
    if (!result.inputsUnchanged) process.exitCode = 1;
    result.finishedUtc = new Date().toISOString(); await writeFile(path.join(out, 'results.json'), JSON.stringify(result, null, 2) + '\n');
    console.log(JSON.stringify({ cases: result.cases.length, passed: result.cases.filter(item => item.passed).length, completed: result.completed, inputsUnchanged: result.inputsUnchanged, ownedProcessesClosed: result.ownedProcessesClosed }));
}
