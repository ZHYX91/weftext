// Real local workspace tasks in an owned, headless Chrome profile.
// --dev deliberately combines the selected CLI with the current Vite frontend;
// it is visual/development evidence, never an embedded candidate acceptance.
import assert from 'node:assert/strict';
import { createHash, randomUUID } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawnServe, serveReady, stopServe, stringifyServeJson } from './_cli_serve.mjs';
import { createdDocument } from './_creation_result.mjs';

const args = process.argv.slice(2);
assert.ok(args.length === 2 || args.length === 3 && args[2] === '--dev',
  'Usage: node scripts/test_workspace_layout_browser.mjs ABSOLUTE_CLI ABSOLUTE_NEW_OUTPUT [--dev]');
assert.ok(path.isAbsolute(args[0]) && path.isAbsolute(args[1]), 'Executable and new output paths must be absolute');
const exe = path.resolve(args[0]), out = path.resolve(args[1]), dev = args[2] === '--dev';
const repo = path.resolve(import.meta.dirname, '..');
assert.ok(path.relative(repo, out).startsWith('..' + path.sep), 'Keep private test evidence outside the public repository');
await mkdir(out, { recursive: false });
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds));
const result = {
  schemaVersion: 1, startedUtc: new Date().toISOString(), exe,
  executableSha256: hash(await readFile(exe)),
  frontend: dev ? 'current Vite development source' : 'selected executable embedded frontend',
  developmentOnly: dev, cases: [], pageErrors: [], consoleErrors: [], requests: [], requestFailures: [], httpErrors: [],
  ownerLifecycles: [], layouts: [], completed: false,
  limits: [
    'Owned headless Chrome only; no user browser, native window, real OS IME or assistive-technology acceptance.',
    'Synthetic composing-key checks cover shortcut routing only.',
    'Narrow CSS viewports model zoom reflow; they do not establish actual browser or OS zoom behavior.',
    'This task check does not establish the complete product, package or cross-platform acceptance.',
  ],
};
const batch = path.dirname(exe), catalogBytes = await readFile(path.join(batch, 'artifact-catalog.json'));
const receiptBytes = await readFile(path.join(batch, 'build-input-receipt.json'));
const catalog = JSON.parse(catalogBytes), receipt = JSON.parse(receiptBytes);
assert.equal(catalog.schemaVersion, 1);
assert.equal(catalog.roles?.cli?.filename, path.basename(exe));
assert.equal(catalog.roles.cli.sha256, result.executableSha256);
assert.equal(receipt.roles?.cli?.sha256, result.executableSha256);
result.artifacts = { batch, catalogSha256: hash(catalogBytes), receiptSha256: hash(receiptBytes), buildId: catalog.buildId };
result.scriptSha256 = hash(await readFile(new URL(import.meta.url)));
result.helperSha256 = hash(await readFile(path.join(repo, 'scripts/_cli_serve.mjs')));
result.version = execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim();

async function frontendInventory() {
  const files = [];
  async function visit(directory) {
    for (const item of await readdir(directory, { withFileTypes: true })) {
      const filename = path.join(directory, item.name);
      assert.ok(!item.isSymbolicLink(), 'Development frontend inventory refuses symlinks');
      if (item.isDirectory()) await visit(filename);
      else if (item.isFile()) {
        const bytes = await readFile(filename);
        files.push({ path: path.relative(repo, filename).replaceAll(path.sep, '/'), bytes: bytes.length, sha256: hash(bytes) });
      }
    }
  }
  await visit(path.join(repo, 'app/src'));
  for (const name of ['app/index.html', 'app/package.json', 'app/package-lock.json', 'assets/ui-messages.json']) {
    const bytes = await readFile(path.join(repo, name));
    files.push({ path: name, bytes: bytes.length, sha256: hash(bytes) });
  }
  return files.sort((a, b) => a.path.localeCompare(b.path));
}
if (dev) {
  result.developmentSourceBefore = await frontendInventory();
  await writeFile(path.join(out, 'development-source-before.json'), JSON.stringify(result.developmentSourceBefore, null, 2), { flag: 'wx' });
}
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const root = path.join(out, 'TEST-library'), data = path.join(out, 'state');
const originals = new Map(), responseTasks = new Set(), allOwners = [];
let owner, base, token, browser, page, vite, apiOrdinal = 0, navigationCount = 0, browserClosing = false, expectedSource;
const pass = name => { result.cases.push(name); console.log('PASS', name); };

async function until(check, message, milliseconds = 30000) {
  const end = Date.now() + milliseconds;
  let last;
  do {
    try { const value = await check(); if (value) return value; }
    catch (error) { last = error; }
    await pause(100);
  } while (Date.now() < end);
  throw new Error(message, { cause: last });
}
async function start(extra = []) {
  assert.equal(owner, undefined, 'An existing owner cannot be replaced');
  const child = spawnServe(exe, ['--data-dir', data, ...extra], { windowsHide: true });
  owner = child; allOwners.push(child);
  const ready = await until(() => serveReady(child), 'Original owned CLI did not become Ready', 45000);
  base = ready.base; token = ready.token;
  result.ownerLifecycles.push({ pid: child.pid, ownerIdentity: ready.ownerIdentity, phase: 'Ready' });
}
async function stop() {
  // Clear before awaiting: an unknown control outcome is never retried by finally.
  const child = owner;
  owner = undefined;
  if (!child) return;
  const stopped = await stopServe(child);
  result.ownerLifecycles.push({ pid: child.pid, receipt: stopped, actualExitCode: child.exitCode, phase: 'normally stopped' });
}
async function api(route, body) {
  assert.ok(owner && owner.exitCode === null && owner.signalCode === null, 'The original owner is no longer live');
  const response = await fetch(base + route, {
    method: body === undefined ? 'GET' : 'POST',
    headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': token },
    body: body === undefined ? undefined : JSON.stringify(body), signal: AbortSignal.timeout(40000),
  });
  const bytes = await response.text();
  const prefix = String(++apiOrdinal).padStart(2, '0');
  await writeFile(path.join(out, `${prefix}-api.json`), JSON.stringify({ route, body, status: response.status, response: JSON.parse(bytes) }, null, 2), { flag: 'wx' });
  assert.ok(response.ok, `${route} failed: ${bytes}`);
  return JSON.parse(bytes);
}
async function frontend() {
  if (!dev) return base;
  const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
  const proxy = { target: base, changeOrigin: true, configure(instance) {
    instance.on('proxyReq', (request, incoming) => { if (incoming.headers.origin) request.setHeader('Origin', base); });
  } };
  vite = await createServer({ root: path.join(repo, 'app'), configFile: false,
    server: { host: '127.0.0.1', port: 0, hmr: false, proxy: { '/api': proxy, '/fonts': proxy } } });
  await vite.listen();
  return 'http://127.0.0.1:' + vite.httpServer.address().port;
}
const ribbon = () => page.locator('.workspace-ribbon');
const tab = id => ribbon().locator(`[role="tab"][data-tab="${id}"]`);
const source = () => page.locator('.source-host .cm-content');
const treeRow = name => page.locator('.file-tree').getByRole('button', { name, exact: true });
const treeWrapper = name => treeRow(name).locator('..');
async function selectTab(id) {
  await tab(id).click();
  assert.equal(await tab(id).getAttribute('aria-selected'), 'true');
  assert.equal(await ribbon().locator('[role="tabpanel"]:visible').count(), 1);
  assert.equal(await ribbon().locator('[role="tabpanel"]:visible').getAttribute('data-tab'), id);
}
async function editorSnapshot() {
  return source().evaluate(element => {
    const view = element.cmTile?.root?.view;
    if (view?.contentDOM !== element) throw new Error('Actual CodeMirror view is unavailable');
    const selection = view.state.selection.main;
    return { source: view.state.doc.toString(), length: view.state.doc.length,
      from: selection.from, to: selection.to, anchor: selection.anchor, head: selection.head,
      composing: view.composing, contentFocused: document.activeElement === element,
      scrollTop: view.scrollDOM.scrollTop };
  });
}
async function expectSource(expected) {
  expectedSource = expected;
  await until(async () => (await editorSnapshot()).source === expected, 'Actual editor source did not match the expected original change');
}
async function expectEndCaret(expected, label) {
  expectedSource = expected;
  const actual = await editorSnapshot();
  (result.editorCheckpoints ??= []).push({ label, ...actual });
  assert.equal(actual.source, expected, `${label}: original source changed`);
  assert.equal(actual.from, expected.length, `${label}: caret is not at the actual document end`);
  assert.equal(actual.to, expected.length, `${label}: selection is not collapsed at the actual document end`);
}
async function observeEditorInput() {
  await source().evaluate(element => {
    const records = window.__weftextLayoutInputEvents = [];
    for (const type of ['keydown', 'keyup', 'beforeinput', 'input']) element.addEventListener(type, event => {
      const view = element.cmTile?.root?.view, selection = view?.state.selection.main;
      records.push({ type, key: event.key ?? null, code: event.code ?? null,
        inputType: event.inputType ?? null, data: event.data ?? null,
        ctrlKey: event.ctrlKey ?? false, shiftKey: event.shiftKey ?? false,
        isTrusted: event.isTrusted, isComposing: event.isComposing ?? false,
        defaultPreventedAtCapture: event.defaultPrevented,
        sourceLengthAtCapture: view?.state.doc.length,
        fromAtCapture: selection?.from, toAtCapture: selection?.to });
      if (records.length > 256) records.shift();
    }, { capture: true });
  });
}
async function selectText(text) {
  await source().click();
  await page.keyboard.press('Control+f');
  const search = page.locator('.cm-search input[name="search"]');
  await search.fill(text); await search.press('End'); await search.press('Enter'); await search.press('Escape');
  const actual = await editorSnapshot();
  assert.equal(actual.source.slice(actual.from, actual.to), text);
}
async function assertSameEditor(handle, expected) {
  assert.ok(await source().evaluate((element, original) => element === original, handle), 'Command selection replaced the active editor DOM');
  const actual = await editorSnapshot();
  assert.equal(actual.source, expected.source);
  assert.deepEqual([actual.from, actual.to], [expected.from, expected.to]);
}

try {
  await start();
  await api('/api/library/create', { path: root, operationId: randomUUID() });
  const parent = createdDocument(await api('/api/note/create', { root, parent: root, name: 'Parent', operationId: randomUUID() }));
  const child = createdDocument(await api('/api/note/create', { root, parent: path.dirname(parent.path), name: 'Child', operationId: randomUUID() }));
  const sibling = createdDocument(await api('/api/note/create', { root, parent: root, name: 'Sibling', operationId: randomUUID() }));
  const parentSource = parent.source + '\n\n= Layout TEST\n\nLayout selection TEST\n\n#table(columns: 2, [A], [B], [one], [two])\n';
  await api('/api/save', { sessionId: parent.sessionId, baseId: parent.baseId, source: parentSource, operationId: randomUUID() });
  for (const document of [parent, child, sibling]) originals.set(document.path, await readFile(document.path));
  assert.equal(originals.get(parent.path).toString('utf8'), parentSource);
  await stop();
  await start(['--open', parent.path, '--root', root]);
  browser = await chromium.launch({ headless: true,
    ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : { channel: 'chrome' }) });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, locale: 'zh-CN' });
  page = await context.newPage();
  page.setDefaultTimeout(30000);
  page.on('pageerror', error => result.pageErrors.push(String(error.stack || error)));
  page.on('console', message => { if (message.type() === 'error') result.consoleErrors.push(message.text()); });
  page.on('framenavigated', frame => { if (frame === page.mainFrame()) navigationCount++; });
  page.on('request', request => { const route = new URL(request.url()).pathname; if (route.startsWith('/api/')) result.requests.push({ method: request.method(), route }); });
  page.on('requestfailed', request => result.requestFailures.push({ route: new URL(request.url()).pathname,
    method: request.method(), error: request.failure()?.errorText, retired: browserClosing }));
  page.on('response', response => {
    const route = new URL(response.url()).pathname;
    if (!route.startsWith('/api/') || response.status() < 400) return;
    const task = response.text().then(body => result.httpErrors.push({ route, status: response.status(), body }))
      .catch(error => (result.responseReadFailures ??= []).push({ route, retired: browserClosing, error: String(error) }));
    responseTasks.add(task); void task.finally(() => responseTasks.delete(task));
  });
  await page.goto((await frontend()) + '/#token=' + token, { waitUntil: 'domcontentloaded' });
  await expectSource(parentSource);
  await page.evaluate(() => document.fonts.ready);
  assert.equal(await page.locator('nav.tabs[aria-label="打开的文档"]').count(), 1);
  assert.equal(await ribbon().getByRole('tablist', { name: '文档功能区', exact: true }).count(), 1);
  assert.equal(await tab('home').getAttribute('aria-selected'), 'true');
  pass('Document tabs and command categories are separate, with Home selected initially');

  // The initial document can mount before bootstrap finishes its library read.
  await treeRow('Parent').waitFor({ state: 'visible' });
  await treeRow('Child').waitFor({ state: 'visible' });
  await treeRow('Sibling').waitFor({ state: 'visible' });
  const originalOrder = await page.locator('.file-tree .tree-row .tree-name').allTextContents();
  assert.equal(await treeWrapper('Parent').locator(':scope > .tree-expand-button').count(), 1);
  for (const name of ['Child', 'Sibling']) assert.equal(await treeWrapper(name).locator(':scope > .tree-expand-button').count(), 0);
  assert.equal(await page.locator('.file-tree .tree-chevron').count(), 0);
  await treeWrapper('Parent').locator(':scope > .tree-expand-button').click();
  await until(async () => await treeRow('Child').count() === 0, 'Collapsed child still appeared');
  assert.ok(await treeRow('Parent').isVisible());
  assert.ok(await treeRow('Sibling').isVisible());
  assert.equal(await treeWrapper('Parent').locator(':scope > .tree-expand-button').getAttribute('aria-expanded'), 'false');
  await treeWrapper('Parent').locator(':scope > .tree-expand-button').click();
  await treeRow('Child').waitFor({ state: 'visible' });
  assert.deepEqual(await page.locator('.file-tree .tree-row .tree-name').allTextContents(), originalOrder);
  await treeRow('Sibling').click();
  await until(async () => (await editorSnapshot()).source === sibling.source, 'Name click did not open the sibling source');
  await treeRow('Parent').click(); await expectSource(parentSource);
  assert.equal(await page.locator('nav.tabs .document-tab[aria-current="page"] .tab-name').textContent(), 'Parent.typ');
  pass('One disclosure keeps its parent, hides descendants only, retains order and opens the real selected source');
  await page.screenshot({ path: path.join(out, 'workspace-initial-1440.png'), fullPage: false });
  await observeEditorInput();

  await selectText('Layout selection TEST');
  const handle = await source().elementHandle(), stable = await editorSnapshot();
  const opens = () => result.requests.filter(item => ['/api/open', '/api/data/open', '/api/bootstrap'].includes(item.route)).length;
  const openCount = opens(), navigations = navigationCount;
  for (const id of ['file', 'insert', 'knowledge', 'review', 'view', 'home']) {
    await selectTab(id); await assertSameEditor(handle, stable);
  }
  assert.equal(opens(), openCount); assert.equal(navigationCount, navigations);
  pass('All six command tabs preserve the actual editor, source, selection and original open session');

  await tab('home').focus(); await page.keyboard.press('ArrowRight');
  assert.equal(await tab('insert').getAttribute('aria-selected'), 'true');
  await page.keyboard.press('End'); assert.equal(await tab('view').getAttribute('aria-selected'), 'true');
  await page.keyboard.press('Home'); assert.equal(await tab('file').getAttribute('aria-selected'), 'true');
  const composingCancelled = await tab('file').evaluate(element => !element.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, cancelable: true, isComposing: true })));
  assert.equal(composingCancelled, false); assert.equal(await tab('file').getAttribute('aria-selected'), 'true');
  await selectTab('home'); await assertSameEditor(handle, stable);
  await ribbon().getByRole('button', { name: '收起功能区', exact: true }).click();
  assert.equal(await ribbon().locator('[role="tabpanel"]:visible').count(), 0);
  await ribbon().getByRole('button', { name: '展开功能区', exact: true }).click();
  await assertSameEditor(handle, stable);
  await ribbon().getByRole('button', { name: '收起功能区', exact: true }).click();
  await page.locator('.status-mode').click();
  assert.equal(await tab('view').getAttribute('aria-selected'), 'true');
  assert.equal(await ribbon().locator('[role="tabpanel"]:visible').count(), 1);
  await selectTab('home'); await assertSameEditor(handle, stable);
  pass('Roving command tabs, composition-key guard and collapsing preserve the editor and selection');

  // CDP insertText is a contenteditable insertion API, not an Enter key. Verify
  // real newline handling separately before using ordinary ASCII key events to
  // test Undo across ribbon changes. Every expected source remains exact.
  await source().click(); await page.keyboard.press('Control+End');
  await expectEndCaret(parentSource, 'before-real-Enter');
  await page.keyboard.press('Enter'); await expectSource(parentSource + '\n');
  await expectEndCaret(parentSource + '\n', 'after-real-Enter');
  await page.keyboard.press('Control+z'); await expectSource(parentSource);
  await page.keyboard.press('Control+y'); await expectSource(parentSource + '\n');
  await page.keyboard.press('Control+z'); await expectSource(parentSource);
  pass('A real Enter preserves the exact newline and original-source Undo/Redo');
  await page.keyboard.press('Control+End'); await expectEndCaret(parentSource, 'before-ASCII-probe');
  await page.keyboard.type('UNDO-PROBE-TEST');
  await expectSource(parentSource + 'UNDO-PROBE-TEST');
  for (const id of ['insert', 'review', 'view', 'home']) await selectTab(id);
  await source().click(); await page.keyboard.press('Control+z'); await expectSource(parentSource);
  await page.keyboard.press('Control+y'); await expectSource(parentSource + 'UNDO-PROBE-TEST');
  await page.keyboard.press('Control+z'); await expectSource(parentSource);
  pass('Undo and Redo retain the actual author change across command tabs');
  await selectText('Layout selection TEST'); await selectTab('home');
  await ribbon().locator('.format-bold').click();
  await expectSource(parentSource.replace('Layout selection TEST', '*Layout selection TEST*'));
  await source().click(); await page.keyboard.press('Control+z'); await expectSource(parentSource);
  pass('Home formatting preserves the original selected range and one Undo restores exact source');

  await selectTab('view');
  for (const name of ['源码', '样式源码', '按需语法', '可视编辑', '阅读']) assert.equal(await ribbon().getByRole('button', { name, exact: true }).count(), 1);
  await ribbon().getByRole('button', { name: '源码', exact: true }).click(); await expectSource(parentSource);
  await selectTab('knowledge');
  await ribbon().getByRole('button', { name: '属性', exact: true }).click();
  await page.locator('.workspace-inspector').waitFor({ state: 'visible' });
  await page.locator('.workspace-inspector .inspector-header').getByRole('button', { name: '关闭', exact: true }).click();
  await page.locator('.workspace-inspector').waitFor({ state: 'hidden' });
  await selectTab('home');
  pass('View exposes all five modes and the authorized property inspector opens and closes independently');

  await until(async () => (await page.locator('.status-right').textContent()).includes('草稿预览已更新'), 'Current author source did not finish its preview before layout capture');
  for (const [width, label] of [[1440, 'desktop'], [1024, 'compact-desktop'], [768, 'tablet'], [720, '1440-at-200-percent-equivalent'], [390, 'phone'], [320, '1280-at-400-percent-equivalent']]) {
    await page.setViewportSize({ width, height: 900 });
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    if (width <= 768) await until(() => page.locator('.sidebar').evaluate(element => getComputedStyle(element).visibility === 'hidden' || element.getBoundingClientRect().right <= 0), 'Collapsed sidebar transition did not finish');
    const observed = await page.evaluate(() => {
      const visible = element => element.getClientRects().length > 0 && getComputedStyle(element).visibility !== 'hidden' && !element.closest('[hidden],[inert]');
      const controls = [...document.querySelectorAll('.topbar button,.workspace-ribbon button,.workspace-ribbon input,.workspace-ribbon select,.workspace-ribbon textarea,.sidebar button,.sidebar input,.workspace-inspector button,.workspace-inspector input,.workspace-inspector select,.workspace-inspector textarea')].filter(visible);
      const sidebar = document.querySelector('.sidebar');
      return {
        viewport: innerWidth, documentWidth: document.documentElement.scrollWidth, bodyWidth: document.body.scrollWidth,
        smallControls: controls.filter(element => parseFloat(getComputedStyle(element).fontSize) < 12).map(element => ({ text: element.textContent, fontSize: getComputedStyle(element).fontSize })),
        sidebarHidden: sidebar?.getAttribute('aria-hidden'), sidebarInert: sidebar?.inert,
      };
    });
    result.layouts.push({ width, label, ...observed });
    assert.ok(observed.documentWidth <= width + 1 && observed.bodyWidth <= width + 1, `Workspace overflows horizontally at ${width}px: ${JSON.stringify(observed)}`);
    assert.deepEqual(observed.smallControls, [], `Interactive controls shrink below 12px at ${width}px`);
    if (width <= 768) {
      assert.equal(observed.sidebarHidden, 'true'); assert.equal(observed.sidebarInert, true);
      assert.ok(await page.locator('.header-sidebar-toggle').isVisible());
    }
    if (width === 1440 || width === 390) await page.screenshot({ path: path.join(out, `workspace-${width}.png`), fullPage: false });
  }
  pass('Responsive workspace keeps readable controls and local overflow at desktop, narrow and zoom-equivalent CSS widths');
  for (const [filename, bytes] of originals) assert.deepEqual(await readFile(filename), bytes, 'Navigation/formatting Undo changed an original saved author file');
  assert.equal(hash(await readFile(exe)), result.executableSha256);
  await Promise.all([...responseTasks]);
  result.expectedHttpStates = []; result.unexpectedHttpErrors = [];
  for (const event of result.httpErrors) {
    let code; try { code = JSON.parse(event.body).error?.code; } catch { /* Retain the original non-JSON body. */ }
    const expected = event.status === 409 && code === 'stale_draft' && ['/api/render', '/api/knowledge/read'].includes(event.route);
    (expected ? result.expectedHttpStates : result.unexpectedHttpErrors).push({ ...event, code });
  }
  assert.deepEqual(result.pageErrors, []); assert.deepEqual(result.unexpectedHttpErrors, []);
  assert.deepEqual((result.responseReadFailures ?? []).filter(item => !item.retired), []);
  if (dev) {
    result.developmentSourceAfter = await frontendInventory();
    result.developmentSourceUnchanged = JSON.stringify(result.developmentSourceBefore) === JSON.stringify(result.developmentSourceAfter);
    assert.ok(result.developmentSourceUnchanged, 'Development source changed during the browser run');
  }
  result.completed = true;
} catch (error) {
  result.failure = String(error.stack || error);
  result.expectedSource = expectedSource ?? null;
  if (page && !page.isClosed()) {
    result.actualEditor = await editorSnapshot().catch(snapshotError => ({ unavailable: String(snapshotError) }));
    result.lastInputEvents = await page.evaluate(() => window.__weftextLayoutInputEvents ?? [])
      .catch(eventError => [{ unavailable: String(eventError) }]);
    await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: false }).catch(() => {});
    await writeFile(path.join(out, 'failure.html'), await page.content().catch(() => ''), { flag: 'wx' });
  }
} finally {
  browserClosing = true;
  try { await browser?.close(); } catch (error) { result.browserCloseFailure = String(error); }
  await Promise.all([...responseTasks]);
  try { await vite?.close(); } catch (error) { result.frontendCloseFailure = String(error); }
  try { await stop(); } catch (error) { result.lifecycleFailure = String(error.stack || error); }
  result.ownedProcessesClosed = allOwners.every(child => child.exitCode === 0 && child.signalCode === null);
  result.endedUtc = new Date().toISOString();
  await writeFile(path.join(out, 'results.json'), stringifyServeJson(result, 2), { flag: 'wx' });
  console.log(JSON.stringify({ out, completed: result.completed, cases: result.cases.length, failure: result.failure, lifecycleFailure: result.lifecycleFailure }));
}
if (!result.completed || !result.ownedProcessesClosed || result.lifecycleFailure || result.browserCloseFailure || result.frontendCloseFailure) process.exitCode = 1;
