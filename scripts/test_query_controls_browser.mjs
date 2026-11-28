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
const inputs = ['app/src/query-panel.ts', 'app/src/query-predicate.ts', 'app/src/query-plan-controls.ts', 'app/src/query-argument-controls.ts', 'app/src/query-relation-controls.ts', 'app/src/query-plan.ts', 'app/src/native-value-form.ts', 'app/src/query-refresh.ts', 'app/src/query-control-state.ts', 'app/src/tag-tree.ts', 'app/src/query-result.test-support.ts', 'app/src/ui-locale.ts', 'app/src/ui-messages.ts', 'app/package.json', 'app/package-lock.json', 'scripts/test_query_controls_browser.mjs'];
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
        if(route==='/api/query/definitions' && window.fixture.savedDefinition)
          return {definitions:[window.fixture.savedDefinition],ownerRevision:'current-revision'};
        if(route==='/api/query/definition/run' && window.fixture.savedDefinition) {
          if(!body.parameters.wanted)throw Error('The required parameter is missing');
          return {definition:window.fixture.savedDefinition,definitionRevision:'retained-revision',result:localQueryResult({cutId:'saved-cut',columns:['title'],rows:[],totalKnown:0})};
        }
        if(route==='/api/query/run'){const n=calls.filter(call=>call.route===route).length;return localQueryResult({cutId:'component-cut-'+n,columns:['title','count'],rows:[{key:'row',entityKey:null,path:null,revision:null,readOnly:true,cells:{title:{state:'value',value:{kind:'text',value:'Component result'}},count:{state:'value',value:{kind:'int',value:'12'}}}}],totalKnown:1});}
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
  await check('table, board, calendar, chart and list use one frozen result and its declared output columns', async () => {
    const view = panel.locator('.query-presentation select').first();
    const initial = await page.evaluate(() => window.fixture.panel.result);
    for (const choice of ['table','board','calendar','chart','list']) {
      await view.selectOption(choice);
      if (choice === 'chart') {
        assert.equal(await panel.locator('.query-chart-value').innerText(), '12');
        const output = panel.locator('.query-view-field');
        await output.evaluate(element => { element.value = 'title'; element.dispatchEvent(new Event('change')); });
        assert.equal(await panel.locator('.query-chart-value').count(), 0);
      }
      assert.equal(await queryCount(), 1);
      assert.deepEqual(await page.evaluate(() => window.fixture.panel.result), initial);
    }
  });
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
  await check('shared definition controls round-trip inclusive bounds, state-only filters and independent CEL drafts', async () => {
    const actual = await page.evaluate(async () => {
      const { queryPlanControls } = await import('/src/query-plan-controls.ts');
      const plan = { scope: { descendants: true, lifecycle: 'live' }, types: [], where: 'title == "before"', select: [], order: [] };
      const schema = { definitions: {}, aliases: {}, types: [], fields: [{ id: 'price', key: 'price', shape: { kind: 'scalar', semantic: 'decimal' }, nullable: true }] };
      const controls = queryPlanControls(plan, schema, () => {});
      document.body.append(controls);
      const [, field, operator] = controls.querySelectorAll('section > select');
      const select = (control, value) => { control.value = value; control.dispatchEvent(new Event('change')); };
      select(field, '1');
      const options = [...operator.options].map(option => [option.value, option.textContent]);
      const add = controls.querySelector('section > button'), source = controls.querySelector('textarea');
      select(operator, '<=');
      const value = operator.nextElementSibling.querySelector('input');
      value.value = '9007199254740993.01'; value.dispatchEvent(new Event('input')); add.click();
      const bound = plan.where, synced = source.value;
      source.value = 'title.startsWith("draft")'; source.dispatchEvent(new Event('input'));
      select(operator, 'invalid'); add.click();
      const final = { options, bound, synced, draft: source.value, combined: plan.where, operandAbsent: !operator.nextElementSibling.querySelector('input') };
      controls.remove(); return final;
    });
    assert.ok(actual.options.some(([value, label]) => value === '<=' && label !== '≤'));
    assert.ok(actual.options.some(([value]) => value === '>='));
    assert.equal(actual.options.some(([value]) => value === 'contains'), false);
    assert.equal(actual.bound, '(title == "before") && (field("price") <= decimal("9007199254740993.01"))');
    assert.equal(actual.synced, actual.bound);
    assert.equal(actual.draft, 'title.startsWith("draft")');
    assert.equal(actual.combined, '(' + actual.bound + ') && (isInvalid(field("price")))');
    assert.ok(actual.operandAbsent);
  });
  await check('condition groups preserve edited leaves, nesting and order in the native plan', async () => {
    await page.evaluate(async () => {
      window.fixture.locale.set('zh-CN');
      const { queryPlanControls } = await import('/src/query-plan-controls.ts');
      const plan = { scope: { descendants: true, lifecycle: 'live' }, types: [], order: [], select: [],
        where: { op: 'any', conditions: ['title == "first"', 'title == "second"'] } };
      window.fixture.conditionPlan = plan;
      const form = queryPlanControls(plan, null, () => {});
      form.id = 'condition-group-fixture';
      document.body.append(form);
    });
    const controls = page.locator('#condition-group-fixture');
    const group = controls.locator('.query-condition-chips > fieldset');
    const second = group.locator(':scope > fieldset').nth(1);
    await second.getByRole('textbox').fill('title == "edited"');
    await second.locator(':scope > select').selectOption('not');
    await second.getByRole('button', { name: '放入新条件组' }).click();
    await group.locator(':scope > fieldset').nth(1).locator(':scope > button').last().evaluate(button =>
      button.previousElementSibling.click());
    const actual = await page.evaluate(async () => {
      const { parseQueryPlan } = await import('/src/query-plan.ts');
      return parseQueryPlan(JSON.stringify(window.fixture.conditionPlan)).where;
    });
    assert.deepEqual(actual, { op: 'any', conditions: [{ op: 'not', conditions: ['title == "edited"'] }, 'title == "first"'] });
    await controls.evaluate(node => node.remove());
  });
  await check('relation controls keep the original seed and capture two distinct directed hops', async () => {
    await page.evaluate(async () => {
      const { queryRelationControls } = await import('/src/query-relation-controls.ts');
      const container = document.createElement('div');
      container.id = 'relation-fixture';
      const draw = plan => {
        window.fixture.relationPlan = plan;
        container.replaceChildren(queryRelationControls(plan, ['parents', 'authors'], draw));
      };
      draw({ scope: { descendants: true, lifecycle: 'live' }, types: ['person'],
        where: 'self == nodeRef("weftext-node://test/seed")', select: [], order: [] });
      document.body.append(container);
    });
    const relations = page.locator('#relation-fixture');
    const add = relations.getByRole('button', { name: '添加一跳', exact: true });
    await add.click();
    await relations.getByRole('combobox', { name: '下一跳的关系类型' }).selectOption('authors');
    await relations.getByRole('combobox', { name: '关系方向', exact: true }).last().selectOption('inverse');
    await add.click();
    assert.equal(await add.count(), 0);
    const actual = await page.evaluate(() => window.fixture.relationPlan.advanced.related);
    assert.equal(actual.relation, 'authors');
    assert.equal(actual.direction, 'inverse');
    const first = actual.from.advanced.related;
    assert.equal(first.relation, 'parents');
    assert.equal(first.direction, 'forward');
    assert.equal(first.from.where, 'self == nodeRef("weftext-node://test/seed")');
    await relations.getByRole('button', { name: '移除最后一跳并返回该跳之前的查询' }).click();
    assert.equal(await add.count(), 1);
    await relations.evaluate(node => node.remove());
  });
  await check('typed argument controls preserve exact values, default omission and an independently edited draft', async () => {
    await page.evaluate(async () => {
      window.fixture.locale.set('zh-CN');
      const { queryArgumentControls } = await import('/src/query-argument-controls.ts');
      const plan = { parameters: [
        { name: 'amount', shape: { kind: 'scalar', semantic: 'decimal' } },
        { name: 'start', shape: { kind: 'scalar', semantic: 'date' }, nullable: true,
          default: { kind: 'date', value: '2026-11-28' } },
      ] };
      const draft = document.createElement('textarea');
      draft.value = '{}';
      const form = queryArgumentControls(draft, () => plan, () => null);
      const host = document.createElement('div');
      host.id = 'typed-arguments-fixture';
      host.append(draft, form);
      document.body.append(host);
    });
    const form = page.locator('#typed-arguments-fixture');
    const draft = form.locator(':scope > textarea');
    await form.locator('summary').click();
    await form.locator('fieldset, [role="status"]:not(:empty)').first().waitFor();
    assert.equal(await form.getByRole('status').innerText(), '');
    const apply = form.getByRole('button', { name: '采用这些实参' });
    const amountMode = form.getByRole('combobox', { name: 'amount 的实参来源' });
    const startMode = form.getByRole('combobox', { name: 'start 的实参来源' });
    await apply.click();
    assert.equal(await draft.inputValue(), '{}');
    await amountMode.selectOption('value');
    await form.getByRole('textbox', { name: 'amount', exact: true }).fill('9007199254740993.01');
    await apply.click();
    const amount = { kind: 'decimal', value: '9007199254740993.01' };
    assert.deepEqual(JSON.parse(await draft.inputValue()), { amount });
    await startMode.selectOption('null');
    await apply.click();
    assert.deepEqual(JSON.parse(await draft.inputValue()), { amount, start: { kind: 'none' } });
    await draft.fill('{"amount":{"kind":"decimal","value":"3.20"}}');
    await apply.click();
    assert.match(await form.getByRole('status').innerText(), /已变化/);
    assert.equal(JSON.parse(await draft.inputValue()).amount.value, '3.20');
    await form.evaluate(element => element.remove());
  });
  await check('saved query loads the pinned parameter declaration before the first successful run', async () => {
    await page.evaluate(async () => {
      window.fixture.savedDefinition = {
        key: 'saved-key', label: 'weftext-query-fixture',
        plan: { scope: { descendants: true, lifecycle: 'live' }, select: [], order: [], types: [],
          parameters: [{ name: 'wanted', shape: { kind: 'scalar', semantic: 'text' } }] },
      };
      await window.fixture.panel.runSaved({
        owner: 'weftext-node://query-fixture/saved', key: 'saved-key', label: 'Fixture',
        mode: 'pinned', revision: 'retained-revision',
      });
    });
    const advanced = panel.locator('.query-advanced-plan');
    await advanced.locator(':scope > summary').click();
    const args = advanced.locator('.query-argument-controls');
    await args.locator('summary').click();
    await args.getByRole('combobox', { name: 'wanted 的实参来源' }).selectOption('value');
    await args.getByRole('textbox', { name: 'wanted', exact: true }).fill('open');
    await args.getByRole('button', { name: '采用这些实参' }).click();
    const routes = await page.evaluate(() => window.fixture.calls.filter(call => call.route.startsWith('/api/query/definition')));
    assert.deepEqual(routes.map(call => call.route), ['/api/query/definitions', '/api/query/definition/run']);
    assert.deepEqual(routes[0].body.selection, { key: 'saved-key', mode: 'pinned', revision: 'retained-revision' });
    const parameters = { wanted: { kind: 'text', value: 'open' } };
    assert.deepEqual(JSON.parse(await advanced.locator('textarea.query-expression').inputValue()), parameters);
    await advanced.getByRole('button', { name: '按本次参数重跑' }).click();
    await panel.locator('.query-basis').filter({ hasText: 'saved-cut' }).first().waitFor();
    const actual = await page.evaluate(() => window.fixture.calls.at(-1));
    assert.equal(actual.route, '/api/query/definition/run');
    assert.deepEqual(actual.body.parameters, parameters);
    assert.deepEqual(actual.body.selection, routes[0].body.selection);
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
