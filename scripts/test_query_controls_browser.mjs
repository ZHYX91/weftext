// Real QueryPanel controls in an owned headless browser with a bounded host.
// This checks UI/request integration, not Core query evaluation or native input.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

assert.equal(process.argv.length, 3, 'Usage: node scripts/test_query_controls_browser.mjs ABSOLUTE_NEW_OUTPUT');
assert.ok(path.isAbsolute(process.argv[2]), 'Output must be absolute');
const repo = path.resolve(import.meta.dirname, '..'), out = path.resolve(process.argv[2]);
assert.ok(path.relative(repo, out).startsWith('..' + path.sep), 'Keep private evidence outside the product repository');
await mkdir(out, { recursive: false });
const inputs = ['app/src/query-panel.ts', 'app/src/query-refresh.ts', 'app/src/query-control-state.ts', 'app/src/tag-tree.ts', 'app/src/query-result.test-support.ts', 'app/src/ui-locale.ts', 'app/src/ui-messages.ts', 'app/package.json', 'app/package-lock.json', 'scripts/test_query_controls_browser.mjs'];
const inventory = async () => Promise.all(inputs.map(async name => { const bytes = await readFile(path.join(repo, name)); return { path: name, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') }; }));
const result = { startedUtc: new Date().toISOString(), frontend: 'current Vite source, isolated QueryPanel component', developmentOnly: true,
  limits: ['Bounded host returns schema, registry and a strict local Query result; no Core query evaluation or persisted author operation.', 'No native desktop, OS IME, assistive technology, performance or whole-product acceptance.'],
  cases: [], pageErrors: [], completed: false, sourceBefore: await inventory() };
const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
const require = createRequire(import.meta.url), { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
let server, browser;
async function check(name, run) { const entry = { name, passed: false }; result.cases.push(entry); await run(); entry.passed = true; }
try {
  server = await createServer({ root: path.join(repo, 'app'), configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false, watch: { ignored: ['**/*'] } }, plugins: [{ name: 'query-controls-fixture', configureServer(server) { server.middlewares.use('/__query_controls_check', (_request, response) => {
    response.setHeader('Content-Type', 'text/html; charset=utf-8');
    response.end(`<!doctype html><meta charset="utf-8"><link rel="stylesheet" href="/src/data-view.css"><style>html,body{height:100%;margin:0;font:14px sans-serif}main{height:100%;display:flex}.query-panel{flex:1;min-height:0}</style><main></main><script type="module">
      import {QueryPanel} from '/src/query-panel.ts';import {locale,uiTextValue} from '/src/ui-locale.ts';import {localQueryResult} from '/src/query-result.test-support.ts';
      const calls=[],notifications=[],root='D:/query-component',workspaceId='query-fixture',tagRef='weftext-tag://query-fixture/root',tagLabel='研究 😀';
      const schema={definitions:{},aliases:{},types:[],fields:[{id:'tags-field',key:'tags',shape:{kind:'list',items:{kind:'scalar',semantic:'tag_ref'},min:0,max:null},nullable:false}]};
      const host={getDocument:()=>null,async applyPatch(){throw Error('No author writer in this component check')},async openDocument(){throw Error('No author navigation in this component check')},notify(text,error){notifications.push({text:uiTextValue(text),error:!!error})},async request(route,body){
        // Model the actual JSON transport boundary, retaining the original body.
        calls.push({route,body:JSON.parse(JSON.stringify(body))});
        if(route==='/api/data/schema')return schema;
        if(route==='/api/navigation/tags')return {baseId:'registry-fixture',complete:true,tags:[{tagRef,tagId:'root',parentTagId:null,displayName:tagLabel},{tagRef:'weftext-tag://query-fixture/child',tagId:'child',parentTagId:'root',displayName:'子级'}]};
        if(route==='/api/query/run'){const n=calls.filter(call=>call.route===route).length;return localQueryResult({cutId:'component-cut-'+n,columns:['title'],rows:[{key:'row',entityKey:null,path:null,revision:null,readOnly:true,cells:{title:{state:'value',value:{kind:'text',value:'Component result'}}}}],totalKnown:1});}
        throw Error('Unexpected component route '+route);
      }};
      locale.set('zh-CN');const panel=new QueryPanel(host);document.querySelector('main').append(panel.element);
      window.fixture={panel,locale,calls,notifications,tagRef,tagLabel};panel.setLibrary({root,workspaceId});window.fixtureReady=true;
    </script>`);
  }); } }] });
  await server.listen();
  browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE || 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
  result.browserVersion = browser.version();
  const context = await browser.newContext({ viewport: { width: 900, height: 850 } }), page = await context.newPage();
  page.on('pageerror', error => result.pageErrors.push(error.message));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/__query_controls_check`);
  await page.waitForFunction(() => window.fixtureReady);
  const panel = page.locator('.query-panel'), chips = panel.locator('.query-chips'), search = panel.locator('.query-search-form input');
  const queryCount = () => page.evaluate(() => window.fixture.calls.filter(call => call.route === '/api/query/run').length);
  const ready = async n => { await page.waitForFunction(n => window.fixture.calls.filter(call => call.route === '/api/query/run').length === n && document.querySelector('.query-results')?.getAttribute('aria-busy') === 'false', n); assert.match(await panel.locator('.query-basis').first().innerText(), new RegExp('component-cut-' + n)); assert.equal(await panel.locator('.query-card').count(), 1); };
  const lastPlan = () => page.evaluate(() => window.fixture.calls.filter(call => call.route === '/api/query/run').at(-1).body.plan);
  const tagRow = panel.locator('.registered-tag-row').first();
  await ready(1);
  await check('registered exact tag creates a real local display chip and sends only native Query state', async () => {
    await panel.getByText('登记标签与目录入口', { exact: true }).click();
    await panel.getByRole('button', { name: '读取登记标签', exact: true }).click();
    await tagRow.getByRole('button', { name: '仅此标签', exact: true }).click(); await ready(2);
    assert.match(await chips.innerText(), /标签：.*研究 😀.*仅此标签/);
    const plan = await lastPlan();
    assert.match(plan.where, /t\.ref == tagRef\("weftext-tag:\/\/query-fixture\/root"\)/);
    assert.doesNotMatch(plan.where, /tagInSubtree/);
    assert.doesNotMatch(JSON.stringify(await page.evaluate(() => window.fixture.calls.filter(call => call.route === '/api/query/run').at(-1).body)), /queryFilter\.tagChip|tagNavigation|研究/);
    assert.match(await page.evaluate(() => { try { JSON.stringify(window.fixture.panel.conditions[0].label); return 'guard missing'; } catch (error) { return error.message; } }), /local display message cannot be sent/);
  });
  await check('exact/subtree toggle reaches the original Query host and updates its native predicate', async () => {
    await chips.getByRole('button', { name: '改为此标签及子级', exact: true }).click(); await ready(3);
    assert.match(await chips.innerText(), /标签：.*研究 😀.*此标签及子级/);
    assert.equal(await chips.locator('.query-chip').count(), 1);
    assert.match((await lastPlan()).where, /tagInSubtree\(t\.ref, tagRef\("weftext-tag:\/\/query-fixture\/root"\)\)/);
    await chips.getByRole('button', { name: '改为仅此标签', exact: true }).click(); await ready(4);
    assert.match((await lastPlan()).where, /t\.ref == tagRef\(/);
  });
  await check('locale changes relabel retained chips without rebuilding focused inputs or triggering a Query', async () => {
    await search.fill('中文 😀 retained');
    await search.evaluate(element => { element.setSelectionRange(3, 5); window.originalQueryInput = element; window.originalChip = document.querySelector('.query-chip'); });
    const count = await queryCount();
    await page.evaluate(() => { window.fixture.locale.set('en-US'); window.fixture.panel.documentChanged(); });
    assert.match(await chips.innerText(), /Tag:.*研究 😀.*This tag only/);
    assert.equal(await search.evaluate(element => element === window.originalQueryInput && element === document.activeElement && document.querySelector('.query-chip') === window.originalChip), true);
    assert.deepEqual(await search.evaluate(element => [element.value, element.selectionStart, element.selectionEnd]), ['中文 😀 retained', 3, 5]);
    assert.equal(await queryCount(), count);
    await page.evaluate(() => window.fixture.locale.set('zh-CN'));
    assert.match(await chips.innerText(), /标签：.*研究 😀.*仅此标签/);
    assert.equal(await queryCount(), count);
    await panel.locator('.query-search-form button[type="submit"]').click(); await ready(5);
    assert.match((await lastPlan()).where, /中文 😀 retained/);
  });
  await check('invalid shortcut recovery displays its local warning, retains the original record and permits Query refresh', async () => {
    await page.evaluate(() => {
      const key = 'weftext-query-shortcuts:weftext-workspace://query-broken', raw = '{"version":2,"items":[{"kind":"tag"}]}';
      localStorage.setItem(key, raw); window.brokenShortcut = { key, raw };
      window.fixture.panel.setLibrary({ root: 'D:/query-component-broken', workspaceId: 'query-broken' });
    }); await ready(6);
    const shortcuts = panel.locator('.query-shortcuts');
    await shortcuts.locator('summary').click();
    assert.match(await shortcuts.locator('.data-warning').innerText(), /快捷入口恢复失败/);
    assert.equal(await shortcuts.locator('pre').innerText(), await page.evaluate(() => window.brokenShortcut.raw));
    assert.equal(await page.evaluate(() => localStorage.getItem(window.brokenShortcut.key) === window.brokenShortcut.raw), true);
    await search.focus();
    await page.evaluate(() => { window.originalWarning = document.querySelector('.query-shortcuts .data-warning'); window.fixture.locale.set('en-US'); window.fixture.panel.documentChanged(); });
    assert.match(await shortcuts.locator('.data-warning').innerText(), /Shortcut recovery failed/);
    assert.equal(await shortcuts.locator('.data-warning').evaluate(element => element === window.originalWarning), true);
    assert.equal(await search.evaluate(element => element === document.activeElement), true);
    assert.equal(await queryCount(), 6);
    await shortcuts.getByRole('button', { name: 'Back up original record and reset invalid entries', exact: true }).click();
    assert.deepEqual(await page.evaluate(() => {
      const { key, raw } = window.brokenShortcut, backups = Object.keys(localStorage).filter(item => item.startsWith(key + ':preserved:'));
      return { reset: JSON.parse(localStorage.getItem(key)), backupCount: backups.length, originalPreserved: backups.length === 1 && localStorage.getItem(backups[0]) === raw };
    }), { reset: { version: 2, items: [] }, backupCount: 1, originalPreserved: true });
    assert.equal(await shortcuts.locator('.data-warning').count(), 0);
    await panel.locator('.query-heading').getByRole('button', { name: 'Refresh', exact: true }).click(); await ready(7);
  });
  result.requests = await page.evaluate(() => window.fixture.calls);
  result.notifications = await page.evaluate(() => window.fixture.notifications);
  assert.deepEqual(result.notifications, []);
  result.sourceAfter = await inventory(); assert.deepEqual(result.sourceAfter, result.sourceBefore, 'Source must remain fixed during the component check');
  assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) { result.error = { message: error.message, stack: error.stack }; process.exitCode = 1; }
finally {
  await browser?.close(); await server?.close(); result.finishedUtc = new Date().toISOString();
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2), { flag: 'wx' });
  console.log(JSON.stringify({ completed: result.completed, cases: result.cases, error: result.error?.message }));
}
