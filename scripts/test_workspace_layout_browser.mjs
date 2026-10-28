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
import { deflateSync } from 'node:zlib';
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
const dockTab = id => page.locator(`[data-dock-tab="${id}"]`);
const dockView = id => page.locator(`#workspace-view-${id}`);
async function revealView(id) {
  if (!await dockTab(id).isVisible()) {
    await selectTab('view');
    const menu = ribbon().locator('.workspace-dock-view-menu');
    await menu.locator('summary').click();
    await menu.locator(`[data-show-view="${id}"]`).click();
  } else await dockTab(id).click();
  await dockView(id).waitFor({ state: 'visible' });
}
async function viewAction(id, label) {
  await dockTab(id).focus(); await page.keyboard.press('Shift+F10');
  await page.locator('.workspace-dock-menu').getByRole('menuitem', { name: label, exact: true }).click();
}
async function showFiles() { await revealView('files'); }
function png() {
  function crc(bytes) { let value = 0xffffffff; for (const byte of bytes) { value ^= byte; for (let bit = 0; bit < 8; bit++) value = (value >>> 1) ^ (value & 1 ? 0xedb88320 : 0); } return (value ^ 0xffffffff) >>> 0; }
  function chunk(kind, bytes) { const type = Buffer.from(kind), length = Buffer.alloc(4), checksum = Buffer.alloc(4); length.writeUInt32BE(bytes.length); checksum.writeUInt32BE(crc(Buffer.concat([type, bytes]))); return Buffer.concat([length, type, bytes, checksum]); }
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(2, 0); ihdr.writeUInt32BE(2, 4); ihdr[8] = 8; ihdr[9] = 2;
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', deflateSync(Buffer.from([0, 255, 0, 0, 0, 255, 0, 0, 255, 0, 0, 0, 255, 0]))), chunk('IEND', Buffer.alloc(0))]);
}
async function selectTab(id) {
  await tab(id).click();
  assert.equal(await tab(id).getAttribute('aria-selected'), 'true');
  assert.equal(await ribbon().locator('[role="tabpanel"]:visible').count(), 1);
  assert.equal(await ribbon().locator('[role="tabpanel"]:visible').getAttribute('data-tab'), id);
}
async function keyboardRebuild(control, key, label) {
  const previous = await control.elementHandle(); assert.ok(previous, `${label}: original control is missing`);
  try {
    await control.focus(); await page.keyboard.press(key);
    await until(() => previous.evaluate(element => !element.isConnected), `${label}: original control was not rebuilt`);
    assert.equal(await control.evaluate(element => document.activeElement === element), true, `${label}: rebuilt control lost keyboard focus`);
  } finally { await previous.dispose(); }
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
  const parentFolder = path.dirname(parent.path), imagePath = path.join(parentFolder, 'layout-resource.png'), ordinaryPath = path.join(parentFolder, 'layout-resource.txt');
  await writeFile(imagePath, png(), { flag: 'wx' });
  await writeFile(ordinaryPath, 'Owned ordinary resource\n', { flag: 'wx' });
  const parentSource = parent.source + '\n\n= Layout TEST\n\nLayout selection TEST\n\n== Layout child heading\n\n#table(columns: 2, [A], [B], [one], [two])\n\n#figure(rect(width: 10pt, height: 10pt), kind: "figure", caption: [Layout figure caption])\n';
  await api('/api/save', { sessionId: parent.sessionId, baseId: parent.baseId, source: parentSource, operationId: randomUUID() });
  for (const document of [parent, child, sibling]) originals.set(document.path, await readFile(document.path));
  for (const filename of [imagePath, ordinaryPath]) originals.set(filename, await readFile(filename));
  assert.equal(originals.get(parent.path).toString('utf8'), parentSource);
  await stop();
  await start(['--open', parent.path, '--root', root]);
  browser = await chromium.launch({ headless: true,
    ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : { channel: 'chrome' }) });
  result.browserVersion = browser.version();
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

  // Directory browsing has its own view and never narrows the filesystem tree.
  await treeWrapper('Parent').locator(':scope > .tree-scope-button').filter({ has: page.locator('svg') }).click();
  await dockView('nodes').waitFor({ state: 'visible' });
  const nodes = () => dockView('nodes').locator('.workspace-nodes-view');
  const nodePath = () => nodes().locator('.node-browser-path');
  assert.equal(await nodes().getAttribute('data-follow'), 'false');
  assert.equal(path.normalize(await nodePath().getAttribute('title')), path.normalize(parentFolder));
  for (const section of ['nodes', 'album', 'files'])
    assert.equal(await nodes().locator(`[data-contents-section="${section}"]`).count(), 1);
  assert.equal(await nodes().locator('[data-contents-section="nodes"] .node-content-row').count(), 1);
  const imageRow = nodes().locator('.node-content-row').filter({ has: page.locator('[data-control-role="title"]', { hasText: 'layout-resource.png' }) });
  const fileRow = nodes().locator('.node-content-row').filter({ has: page.locator('[data-control-role="title"]', { hasText: 'layout-resource.txt' }) });
  await imageRow.locator('[data-control-role="open"]').click();
  const resourceDialog = page.locator('.inventory-resource-dialog[open]');
  await resourceDialog.waitFor();
  await until(() => resourceDialog.locator('img').evaluateAll(images => images.some(image => image.naturalWidth === 2 && image.naturalHeight === 2)), 'Ordinary image did not display its actual owned pixels');
  await resourceDialog.getByRole('button', { name: '关闭', exact: true }).click();
  await fileRow.locator('[data-control-role="open"]').click();
  await resourceDialog.waitFor();
  await until(async () => (await resourceDialog.textContent()).includes('Owned ordinary resource'), 'Ordinary file did not display its exact owned text');
  await resourceDialog.getByRole('button', { name: '关闭', exact: true }).click();
  const albumToggle = nodes().locator('[data-section-control="album-toggle"]');
  await albumToggle.focus(); await page.keyboard.press('Enter');
  assert.equal(await albumToggle.getAttribute('aria-expanded'), 'false');
  assert.equal(await albumToggle.evaluate(element => document.activeElement === element), true);
  await page.keyboard.press('Enter');
  await nodes().locator('.node-browser-filter').fill('layout-resource.png');
  assert.equal(await nodes().locator('.node-content-row').count(), 1);
  await showFiles();
  assert.deepEqual(await page.locator('.file-tree .tree-row .tree-name').allTextContents(), originalOrder, 'Nodes filter changed the independent file tree');
  await revealView('nodes'); await nodes().locator('.node-browser-filter').fill('');
  await nodes().locator('.node-browser-toolbar button').first().click();
  assert.equal(await nodes().getAttribute('data-follow'), 'true');
  await showFiles(); await treeRow('Sibling').click(); await expectSource(sibling.source);
  await revealView('nodes');
  assert.equal(path.normalize(await nodePath().getAttribute('title')), path.normalize(path.dirname(sibling.path)));
  await nodes().locator('.node-browser-toolbar button').first().click();
  await showFiles(); await treeRow('Parent').click(); await expectSource(parentSource);
  await revealView('nodes');
  assert.equal(path.normalize(await nodePath().getAttribute('title')), path.normalize(path.dirname(sibling.path)), 'Pinned node browsing followed another editor');
  await nodes().locator('.node-browser-toolbar button').first().click();
  assert.equal(path.normalize(await nodePath().getAttribute('title')), path.normalize(parentFolder));
  await showFiles();
  pass('Independent Nodes sections open exact ordinary resources, retain filter/fold focus and distinguish follow from pinned browsing');

  await revealView('timeline');
  const timeline = dockView('timeline');
  await until(async () => await timeline.locator('[data-history-version]').count() > 0, 'Timeline did not show actual retained saves');
  assert.equal(path.normalize(await timeline.locator('.timeline-path').textContent()), path.normalize(parent.path));
  await timeline.locator('.timeline-version-summary').first().click();
  assert.equal(await timeline.locator('[data-history-version]').first().getAttribute('open'), '');
  await timeline.getByRole('button', { name: '比较与恢复…', exact: true }).click();
  const history = page.locator('.history-panel[open]'); await history.waitFor();
  await history.getByRole('button', { name: '关闭时间线', exact: true }).click();
  await expectSource(parentSource); await showFiles();
  pass('Timeline reads actual saved versions and routes comparison to the existing guarded history owner');

  await revealView('outline');
  const outline = () => dockView('outline').locator('.outline-view');
  await until(async () => await outline().locator('[data-outline-kind="figure"]').count() === 1, 'Native figure caption did not enter the independent Outline');
  const outlineHandle = await source().elementHandle();
  await outline().locator('[data-outline-control="search"]').fill('Layout figure caption');
  assert.equal(await outline().locator('[data-outline-kind="figure"]').count(), 1);
  assert.ok((await outline().locator('.outline-match-controls').textContent()).includes('1 个大纲项命中'));
  assert.equal(await outline().locator('[data-outline-control="search"]').evaluate(element => document.activeElement === element), true);
  await outline().locator('[data-outline-control="search"]').fill('');
  await outline().locator('[data-outline-control="filter"]').selectOption('caption');
  assert.equal(await outline().locator('[data-outline-kind="heading"]').count(), 0);
  await outline().locator('[data-outline-kind="figure"] .outline-jump').click();
  const captionSelection = await editorSnapshot();
  assert.ok(captionSelection.source.slice(captionSelection.from, captionSelection.to).includes('Layout figure caption'), 'Caption jump did not select its exact native author range');
  await outline().locator('[data-outline-control="filter"]').selectOption('all');
  await outline().locator('[data-outline-control="map"]').click();
  await outline().locator('svg').first().waitFor();
  await outline().locator('[data-outline-control="list"]').click();
  assert.equal(await source().evaluate((element, original) => element === original, outlineHandle), true);
  await expectSource(parentSource); await showFiles();
  pass('Independent Outline lists native headings and captions, preserves filter focus, navigates exact source and offers a heading map');
  await page.screenshot({ path: path.join(out, 'workspace-initial-1440.png'), fullPage: false });
  await observeEditorInput();

  await selectText('Layout selection TEST');
  const handle = await source().elementHandle(), stable = await editorSnapshot();
  const opens = () => result.requests.filter(item => ['/api/open', '/api/data/open', '/api/bootstrap'].includes(item.route)).length;
  const openCount = opens(), navigations = navigationCount;
  for (const id of ['nodes', 'query', 'timeline', 'outline', 'files']) {
    await revealView(id); await assertSameEditor(handle, stable);
  }
  const groupsBefore = await page.locator('.workspace-dock-left .workspace-dock-group').count();
  await viewAction('nodes', '拆分到下方');
  assert.equal(await page.locator('.workspace-dock-left .workspace-dock-group').count(), groupsBefore + 1);
  await assertSameEditor(handle, stable);
  const separator = page.locator('.workspace-dock-left .workspace-dock-divider').first();
  const initialSize = Number(await separator.getAttribute('aria-valuenow'));
  await separator.focus(); await page.keyboard.press('ArrowDown');
  assert.notEqual(Number(await separator.getAttribute('aria-valuenow')), initialSize);
  await viewAction('nodes', '合并到上方分区');
  assert.equal(await page.locator('.workspace-dock-left .workspace-dock-group').count(), groupsBefore);
  await showFiles(); await assertSameEditor(handle, stable);
  assert.equal(opens(), openCount); assert.equal(navigationCount, navigations);
  pass('Independent sidebar views, keyboard split/merge and vertical resize retain the real editor and selection without reopening source');
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
  const sourceMode = ribbon().getByRole('button', { name: '源码', exact: true });
  await keyboardRebuild(sourceMode, 'Enter', 'Source mode'); await expectSource(parentSource);
  const layoutChoice = ribbon().locator('select.layout-choice');
  await keyboardRebuild(layoutChoice, 'End', 'Paged layout');
  await until(async () => await layoutChoice.inputValue() === 'paged', 'Keyboard did not choose paged layout');
  await keyboardRebuild(layoutChoice, 'Home', 'Continuous layout');
  await until(async () => await layoutChoice.inputValue() === 'continuous', 'Keyboard did not restore continuous layout');
  const previewToggle = ribbon().locator('.preview-toggle'), previewLabel = await previewToggle.textContent();
  await keyboardRebuild(previewToggle, 'Enter', 'Preview toggle');
  await until(async () => await previewToggle.textContent() !== previewLabel, 'Keyboard preview toggle did not settle');
  await keyboardRebuild(previewToggle, 'Enter', 'Restore preview preference');
  await until(async () => await previewToggle.textContent() === previewLabel, 'Keyboard did not restore preview preference');
  await selectTab('knowledge');
  const properties = ribbon().getByRole('button', { name: '属性', exact: true });
  await properties.focus(); await page.keyboard.press('Enter');
  await dockView('data').waitFor({ state: 'visible' });
  const crossSideEditor = await editorSnapshot(), propertyElement = await dockView('data').elementHandle();
  const rightGroup = page.locator('.workspace-dock-right .workspace-dock-group').first();
  const targetRect = await rightGroup.boundingBox(); assert.ok(targetRect);
  await dockTab('nodes').dragTo(rightGroup, { targetPosition: { x: targetRect.width / 2, y: targetRect.height / 2 } });
  await page.locator('.workspace-dock-right [data-dock-tab="nodes"]').waitFor({ state: 'visible' });
  await assertSameEditor(handle, crossSideEditor);
  assert.equal(await dockView('data').evaluate((element, original) => element === original, propertyElement), true);
  await viewAction('nodes', '移到左侧栏');
  await revealView('data'); await viewAction('data', '隐藏视图');
  await page.locator('.workspace-dock-right').waitFor({ state: 'hidden' });
  assert.equal(await dockView('data').evaluate(element => element.isConnected && !!element.closest('[hidden][inert]')), true, 'Hidden property owner was detached rather than parked');
  await showFiles();
  await selectTab('home');
  pass('Five modes remain reachable; cross-side drag and hidden property owners preserve mounted controls and the author editor');

  const workPanel = page.locator('.work-panel'), workStatus = page.locator('.work-panel-status-button');
  assert.equal(await workPanel.getAttribute('hidden'), '');
  await source().focus(); await page.keyboard.press('Control+End');
  const invalidSuffix = '#layout_test_missing(';
  await page.keyboard.type(invalidSuffix); await expectSource(parentSource + invalidSuffix);
  await until(async () => await workPanel.locator('.work-panel-item[data-state="error"]').count() > 0, 'Actual invalid native source did not report a bottom-panel problem');
  assert.equal(await workPanel.getAttribute('hidden'), '', 'Automatic compiler reports opened the work panel');
  assert.equal(await source().evaluate(element => document.activeElement === element), true, 'Automatic diagnostics stole author focus');
  await workStatus.click();
  await workPanel.locator('.work-panel-tab[data-tab="problems"]').click();
  await workPanel.locator('.work-panel-item[data-state="error"] .work-panel-item-actions button').first().click();
  const diagnosticSelection = await editorSnapshot();
  assert.ok(diagnosticSelection.from >= parentSource.length && diagnosticSelection.to <= parentSource.length + invalidSuffix.length, 'Diagnostic navigation did not stay within the actual failing source fragment');
  await source().focus(); await page.keyboard.press('Control+z'); await expectSource(parentSource);
  await until(async () => await workPanel.locator('.work-panel-item[data-state="error"]').count() === 0, 'Current clean projection retained stale diagnostics');
  await workPanel.locator('.work-panel-collapse').click();
  pass('Actual compile problems update the collapsed bottom panel without stealing focus, jump to source and retire after Undo');

  await selectTab('file');
  await ribbon().getByRole('button', { name: '输出与导入', exact: true }).click();
  const output = page.locator('.output-panel[open]'); await output.waitFor();
  await output.getByLabel('输出格式', { exact: true }).selectOption('source');
  const preparedResponse = page.waitForResponse(response => new URL(response.url()).pathname === '/api/export/prepare' && response.request().method() === 'POST');
  await output.getByRole('button', { name: '生成并查看预览', exact: true }).click();
  const outputResponse = await preparedResponse;
  assert.equal(outputResponse.status(), 200);
  const actualOutput = await outputResponse.json(), originalOutputRequest = outputResponse.request().postDataJSON();
  assert.equal(actualOutput.requestId, originalOutputRequest.requestId);
  await until(async () => await workPanel.locator(`.work-panel-item[data-id="${actualOutput.requestId}"][data-state="waiting"]`).count() === 1, 'Generated output did not retain its actual original request in Tasks');
  assert.equal(await workPanel.getAttribute('hidden'), '', 'Automatic output progress opened the work panel');
  await output.getByRole('button', { name: '关闭', exact: true }).first().click();
  await workStatus.click();
  await workPanel.locator('.work-panel-tab[data-tab="results"]').click();
  const resultRow = workPanel.locator(`.work-panel-item[data-id="${actualOutput.id}"]`);
  await resultRow.waitFor();
  assert.ok((await resultRow.textContent()).includes(actualOutput.filename));
  await resultRow.getByRole('button', { name: '查看详情', exact: true }).click();
  await output.waitFor();
  await output.getByRole('button', { name: '确认本次文件', exact: true }).click();
  await until(() => output.getByRole('button', { name: '下载已确认文件', exact: true }).isEnabled(), 'Exact generated source was not confirmed');
  const downloadPromise = page.waitForEvent('download');
  await output.getByRole('button', { name: '下载已确认文件', exact: true }).click();
  const download = await downloadPromise, downloadPath = path.join(out, 'actual-output.typ');
  await download.saveAs(downloadPath);
  assert.equal((await readFile(downloadPath)).toString('utf8'), parentSource);
  await until(async () => await workPanel.locator(`.work-panel-item[data-id="${actualOutput.requestId}"][data-state="succeeded"]`).count() === 1, 'Downloaded original output did not become a completed task');
  await output.getByRole('button', { name: '关闭', exact: true }).first().click();
  await workPanel.locator('.work-panel-collapse').click();
  await selectTab('home'); await expectSource(parentSource);
  result.actualOutput = { requestId: actualOutput.requestId, id: actualOutput.id, filename: actualOutput.filename, digest: actualOutput.digest, length: actualOutput.length, downloadedSha256: hash(await readFile(downloadPath)) };
  pass('Actual output tasks and generated results retain their original IDs and route review, confirmation and exact download to the original owner');

  const leftWidthHandle = page.locator('.navigation-sidebar-resizer');
  const previousWidth = Number(await leftWidthHandle.getAttribute('aria-valuenow'));
  await leftWidthHandle.focus(); await page.keyboard.press('ArrowRight');
  assert.equal(Number(await leftWidthHandle.getAttribute('aria-valuenow')), previousWidth + 12);
  await page.keyboard.press('ArrowLeft');
  assert.equal(Number(await leftWidthHandle.getAttribute('aria-valuenow')), previousWidth);
  await expectSource(parentSource);
  pass('Sidebar width geometry remains keyboard-adjustable independently of Dock visibility and grouping');

  await until(async () => (await page.locator('.status-right').textContent()).includes('草稿预览已更新'), 'Current author source did not finish its preview before layout capture');
  for (const [width, label] of [[1440, 'desktop'], [1024, 'compact-desktop'], [768, 'tablet'], [720, '1440-at-200-percent-equivalent'], [390, 'phone'], [320, '1280-at-400-percent-equivalent']]) {
    await page.setViewportSize({ width, height: 900 });
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    if (width <= 768 && !await page.locator('.sidebar').evaluate(element => element.hidden))
      await page.locator('.sidebar-overlay-close').click();
    if (width <= 768) await until(() => page.locator('.sidebar').evaluate(element => element.hidden && element.inert), 'Dock did not hide the explicitly collapsed sidebar');
    const observed = await page.evaluate(() => {
      const visible = element => element.getClientRects().length > 0 && getComputedStyle(element).visibility !== 'hidden' && !element.closest('[hidden],[inert]');
      const controls = [...document.querySelectorAll('.topbar button,.workspace-ribbon button,.workspace-ribbon input,.workspace-ribbon select,.workspace-ribbon textarea,.sidebar button,.sidebar input,.workspace-inspector button,.workspace-inspector input,.workspace-inspector select,.workspace-inspector textarea')].filter(visible);
      const sidebar = document.querySelector('.sidebar');
      return {
        viewport: innerWidth, documentWidth: document.documentElement.scrollWidth, bodyWidth: document.body.scrollWidth,
        smallControls: controls.filter(element => parseFloat(getComputedStyle(element).fontSize) < 12).map(element => ({ text: element.textContent, fontSize: getComputedStyle(element).fontSize })),
        sidebarHidden: sidebar?.hidden, sidebarInert: sidebar?.inert,
      };
    });
    result.layouts.push({ width, label, ...observed });
    assert.ok(observed.documentWidth <= width + 1 && observed.bodyWidth <= width + 1, `Workspace overflows horizontally at ${width}px: ${JSON.stringify(observed)}`);
    assert.deepEqual(observed.smallControls, [], `Interactive controls shrink below 12px at ${width}px`);
    if (width <= 768) {
      assert.equal(observed.sidebarHidden, true); assert.equal(observed.sidebarInert, true);
      assert.ok(await page.locator('.header-sidebar-toggle').isVisible());
    }
    if (width === 390) {
      await page.locator('.header-sidebar-toggle').click();
      assert.equal(await page.locator('.sidebar').evaluate(element => element.hidden || element.inert), false);
      await page.locator('.sidebar-overlay-close').click();
      assert.equal(await page.locator('.sidebar').evaluate(element => element.hidden && element.inert), true);
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
