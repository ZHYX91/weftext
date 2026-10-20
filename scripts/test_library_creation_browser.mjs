// Real local initialization UI over owned fixtures. Never uses desktop input.
// The supplied binary and frontend mode are recorded; working builds are not
// promoted to final candidates by this focused acceptance.
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile, readdir } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]);
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const dev = process.argv.includes('--dev'), storageKey = 'weftext-library-creation-v1';
const browserExecutable = process.env.WEFTEXT_BROWSER_EXECUTABLE;
await mkdir(out, { recursive: false });
const result = { executable: exe, sha256: createHash('sha256').update(await readFile(exe)).digest('hex'),
  version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(),
  frontend: dev ? 'current Vite sources; component integration only' : 'embedded executable frontend',
  cases: [], requests: [], pageErrors: [], completed: false };
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
let server, browser, vite, page, stderr = '', mode = 'normal', defaultsCalls = 0;
const creationRequests = [];
async function readOwnedFile(file, encoding = 'utf8') {
  // Core's short protected observation may overlap an external fixture read.
  // Retry only an actual share conflict; all other failures remain failures.
  for (let index = 0; ; index++) {
    try { return await readFile(file, encoding); }
    catch (error) { if (!['EBUSY', 'EPERM'].includes(error.code) || index >= 30) throw error; await pause(100); }
  }
}

async function start() {
  const child = spawn(exe, ['--no-browser', '--data-dir', path.join(out, 'private-state')],
    { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  let stdout = '';
  child.stdout.on('data', value => stdout += value);
  child.stderr.on('data', value => stderr += value);
  for (let index = 0; index < 300; index++) {
    const found = stdout.match(/(http:\/\/127\.0\.0\.1:\d+)\/#token=(\w+)/);
    if (found) return { child, base: found[1], token: found[2] };
    if (child.exitCode !== null) throw new Error(`Owned server exited: ${stderr}`);
    await pause(100);
  }
  child.kill(); throw new Error('Owned server startup timed out');
}
async function stop() {
  if (server?.child.exitCode === null && server.child.signalCode === null) {
    const exited = new Promise(resolve => server.child.once('exit', resolve));
    server.child.kill(); await exited;
  }
}
async function frontend() {
  if (!dev) return server.base;
  const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
  const proxy = { target: server.base, changeOrigin: true, configure(instance) {
    instance.on('proxyReq', (request, incoming) => { if (incoming.headers.origin) request.setHeader('Origin', server.base); });
  } };
  vite = await createServer({ root: path.join(repo, 'app'), configFile: false,
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: { ignored: ['**/*'] }, proxy: { '/api': proxy, '/fonts': proxy } } });
  await vite.listen(); return `http://127.0.0.1:${vite.httpServer.address().port}`;
}
async function api(route, body, expected = 200) {
  const response = await fetch(server.base + route, { method: body === undefined ? 'GET' : 'POST',
    headers: { 'X-Weftext-Token': server.token, 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body) });
  const value = await response.json(); assert.equal(response.status, expected, JSON.stringify(value)); return value;
}
const click = (root, name) => root.getByRole('button', { name, exact: true }).click();
const dialog = () => page.getByRole('dialog', { name: '创建资料库与文档排版语言', exact: true });
const retained = () => page.evaluate(key => JSON.parse(localStorage.getItem(key)), storageKey);
async function open() {
  if (page.viewportSize().width < 780 && !await page.locator('.shell').evaluate(element => element.classList.contains('sidebar-open'))) {
    await page.getByRole('button', { name: '显示资料库导航', exact: true }).click();
  }
  await page.getByRole('button', { name: '创建资料库', exact: true }).first().click();
  await dialog().waitFor(); return dialog();
}
async function preview(root, language, region) {
  const panel = await open();
  await panel.getByRole('textbox', { name: '新资料库完整路径', exact: true }).fill(root);
  await panel.getByRole('combobox', { name: '初始文档排版语言', exact: true }).selectOption(language);
  await panel.getByRole('combobox', { name: '初始文档排版地区', exact: true }).selectOption(region);
  await click(panel, '预览创建');
  assert.equal((await retained()).request, null);
  return panel;
}
async function waitUnknown() {
  await page.waitForFunction(key => !!JSON.parse(localStorage.getItem(key) || '{}').request, storageKey);
  await dialog().getByRole('status').filter({ hasText: '原操作' }).waitFor();
  await dialog().getByRole('button', { name: '确认同一创建操作', exact: true }).waitFor({ state: 'visible' });
  await page.waitForFunction(() => !document.querySelector('.library-creation-dialog button:last-child')?.disabled);
}
async function verifyFiles(request) {
  const stylePath = path.join(request.path, 'Styles/library-style.typ');
  assert.equal(await readOwnedFile(stylePath), request.options.initialStyle.source);
  const settings = JSON.parse(await readOwnedFile(path.join(request.path, '.weftext-settings.json')));
  assert.deepEqual(Object.keys(settings).sort(), ['defaultStyle', 'version']);
  const resource = JSON.parse(await readOwnedFile(path.join(request.path, 'Styles/weftext.resources.json'))).resources[0];
  assert.ok(settings.defaultStyle.resource.endsWith('/' + resource.resourceId));
  const plan = JSON.parse(await readOwnedFile(path.join(out, `private-state/journal/${request.operationId}.plan.json`)));
  assert.equal(plan.kind, 'createLibrary');
  assert.deepEqual(plan.transaction.request.intent.options, request.options);
  assert.equal(Object.keys(plan.transaction.preview.nodeMap).length, 1);
  assert.equal(Object.keys(plan.transaction.preview.resourceMap).length, 1);
  return { source: request.options.initialStyle.source, resource: settings.defaultStyle.resource };
}
async function assertActionsReachable(panel) {
  for (const button of await panel.locator('.modal-actions button').all()) {
    await button.scrollIntoViewIfNeeded();
    const reachable = await button.evaluate(button => {
      const r = button.getBoundingClientRect(), target = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
      return r.x >= 0 && r.right <= innerWidth && r.y >= 0 && r.bottom <= innerHeight && button.contains(target);
    });
    assert.ok(reachable, `Action is not pointer reachable after scrolling: ${await button.textContent()}`);
  }
  const expected = await panel.locator('.modal-actions button:enabled').allTextContents(), reached = new Set();
  for (let index = 0; index < 24; index++) {
    await page.keyboard.press('Tab');
    reached.add(await page.evaluate(() => document.activeElement?.textContent));
  }
  for (const label of expected) assert.ok(reached.has(label), `Keyboard cannot reach ${label}`);
}

try {
  server = await start();
  browser = await chromium.launch({ headless: true, ...(browserExecutable ? { executablePath: browserExecutable } : {}) });
  result.browser = { version: browser.version(), executable: browserExecutable || 'Playwright bundled Chromium' };
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
  page = await context.newPage(); page.on('pageerror', error => result.pageErrors.push(String(error)));
  await page.route('**/api/library/create-defaults', async route => { defaultsCalls++; await route.continue(); });
  await page.route('**/api/styles/prepare', () => { throw new Error('Initialization must not use a second style transaction'); });
  await page.route('**/api/styles/apply', () => { throw new Error('Initialization must not use a second style transaction'); });
  await page.route('**/api/library/create', async route => {
    const request = route.request().postDataJSON(); creationRequests.push(request);
    assert.deepEqual((await retained()).request, request, 'Exact request must be durable before author IO');
    if (mode === 'lose-committed-response') {
      const response = await route.fetch(); assert.equal(response.status(), 200);
      result.lostCommittedReceipt = await response.json(); await route.abort('failed');
    } else if (mode === 'lose-before-server') await route.abort('failed');
    else await route.continue();
  });
  const base = await frontend(); await page.goto(`${base}/#token=${server.token}`);
  await page.getByRole('button', { name: '创建资料库', exact: true }).first().waitFor();

  let panel = await open();
  const language = panel.getByRole('combobox', { name: '初始文档排版语言', exact: true });
  const region = panel.getByRole('combobox', { name: '初始文档排版地区', exact: true });
  assert.equal(await language.inputValue(), '"zh"'); assert.equal(await region.inputValue(), '"CN"');
  assert.ok((await language.locator('option').allTextContents()).includes('English · en'));
  assert.ok((await region.locator('option').allTextContents()).includes('未指定地区 · none'));
  assert.ok((await region.locator('option').allTextContents()).includes('英国 · GB'));
  await click(panel, '关闭'); result.cases.push('native initialization has explicit friendly language/region choices');

  panel = await preview(path.join(out, 'english-unknown-region'), '"en"', 'none');
  await assertActionsReachable(panel);
  assert.match(await panel.locator('pre').textContent(), /lang: "en"/);
  assert.match(await panel.locator('pre').textContent(), /region: none/);
  await click(panel, '创建资料库'); await panel.waitFor({ state: 'hidden' });
  const first = creationRequests.at(-1); await verifyFiles(first);
  assert.equal(await retained(), null);
  result.cases.push('English and unknown region are created as exact native resource in the original creation Plan');

  mode = 'lose-committed-response';
  panel = await preview(path.join(out, 'lost-committed-result'), '"en"', '"GB"');
  await click(panel, '创建资料库'); await waitUnknown();
  const lost = structuredClone((await retained()).request), count = defaultsCalls;
  await verifyFiles(lost);
  assert.equal(result.lostCommittedReceipt.operationId, lost.operationId);
  assert.equal(await panel.getByRole('textbox', { name: '新资料库完整路径' }).isDisabled(), true);
  await click(panel, '关闭'); await page.reload(); panel = await open();
  assert.equal(defaultsCalls, count, 'Recovery must retain original native bytes without replacing template');
  assert.deepEqual((await retained()).request, lost);
  assert.equal(await panel.getByRole('combobox', { name: '初始文档排版地区' }).inputValue(), '"GB"');
  mode = 'normal'; await click(panel, '确认同一创建操作'); await panel.waitFor({ state: 'hidden' });
  assert.deepEqual(creationRequests.at(-1), lost); await verifyFiles(lost);
  const commits = (await readdir(path.join(out, 'private-state/journal'))).filter(name => name === `${lost.operationId}.commit.json`);
  assert.equal(commits.length, 1);
  result.cases.push('lost committed response survives reload and retries exact original ID, source and choices');

  mode = 'lose-before-server';
  await page.setViewportSize({ width: 480, height: 800 });
  panel = await preview(path.join(out, 'unknown-never-created'), '"zh"', '"TW"');
  await assertActionsReachable(panel);
  await click(panel, '创建资料库'); await waitUnknown();
  const unknown = structuredClone((await retained()).request);
  await click(panel, '关闭'); await page.reload(); panel = await open();
  await assertActionsReachable(panel);
  await click(panel, '查看原操作恢复信息');
  await panel.getByRole('status').filter({ hasText: '原操作' }).waitFor();
  assert.deepEqual((await retained()).request, unknown);
  assert.equal(await panel.getByRole('textbox', { name: '新资料库完整路径' }).isDisabled(), true);
  mode = 'normal'; await click(panel, '结束尚未开始的原创建');
  await page.waitForFunction(key => JSON.parse(localStorage.getItem(key)).request === null, storageKey);
  assert.equal(await panel.getByRole('textbox', { name: '新资料库完整路径' }).isEnabled(), true);
  const late = await api('/api/library/create', unknown, 409); assert.equal(late.error.code, 'operationCanceled');
  assert.equal((await api('/api/recovery/inspect', { operationId: unknown.operationId })).status, 'resolved');
  await click(panel, '关闭'); await page.reload(); panel = await open();
  assert.equal(await panel.getByRole('textbox', { name: '新资料库完整路径' }).isEnabled(), true);
  assert.equal((await retained()).request, null);
  await click(panel, '关闭');
  result.cancelledOriginal = unknown;
  result.cases.push('unknown request stays locked after inspection; durable Core cancellation releases it and rejects late creation');

  panel = await preview(path.join(out, 'new-after-cancellation'), '"ja"', '"JP"');
  await click(panel, '创建资料库'); await panel.waitFor({ state: 'hidden' });
  const fresh = creationRequests.at(-1); assert.notEqual(fresh.operationId, unknown.operationId); await verifyFiles(fresh);
  assert.match(fresh.options.initialStyle.source, /lang: "ja"/);
  assert.match(fresh.options.initialStyle.source, /region: "JP"/);
  result.cases.push('new inputs become a fresh operation only after confirmed cancellation');
  result.cases.push('all wide and narrow initialization/recovery actions are pointer and keyboard reachable');
  assert.deepEqual(result.pageErrors, []);
  result.requests = creationRequests; result.completed = true;
} catch (error) {
  result.error = String(error.stack || error);
  if (page) {
    result.failureBody = await page.locator('body').innerText().catch(() => 'unavailable');
    await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {});
  }
  throw error;
} finally {
  await browser?.close(); await vite?.close(); await stop();
  result.requests = creationRequests;
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2));
  await writeFile(path.join(out, 'server.stderr.log'), stderr);
}
process.stdout.write(JSON.stringify({ completed: result.completed, sha256: result.sha256,
  frontend: result.frontend, cases: result.cases, pageErrors: result.pageErrors,
  evidence: path.join(out, 'result.json') }, null, 2));
