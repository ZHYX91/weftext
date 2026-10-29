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
const knowledgeReads = () => result.requests.filter(request => request.route === '/api/knowledge/read').length;
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
  // Admit this owned fixture's root explicitly so its actual Parent/Sibling
  // nodes can participate in shared author ordering through the real UI.
  const rootAdmission = await api('/api/navigation/admit-directory', { root, path: root, operationId: randomUUID() });
  const rootAdmissionResult = await api('/api/operations/commit', { operationId: rootAdmission.preview.operationId, planId: rootAdmission.preview.planId });
  assert.equal(rootAdmissionResult.status, 'committed');
  result.orderFixture = { parentRef: rootAdmission.targetRef, operationId: rootAdmission.preview.operationId };
  const parent = createdDocument(await api('/api/note/create', { root, parent: root, name: 'Parent', operationId: randomUUID() }));
  const child = createdDocument(await api('/api/note/create', { root, parent: path.dirname(parent.path), name: 'Child', operationId: randomUUID() }));
  const sibling = createdDocument(await api('/api/note/create', { root, parent: root, name: 'Sibling', operationId: randomUUID() }));
  const parentFolder = path.dirname(parent.path), imagePath = path.join(parentFolder, 'layout-resource.png'), ordinaryPath = path.join(parentFolder, 'layout-resource.txt');
  await writeFile(imagePath, png(), { flag: 'wx' });
  await writeFile(ordinaryPath, 'Owned ordinary resource\n', { flag: 'wx' });
  const parentSource = parent.source + '\n\n= Layout TEST\n\nLayout selection TEST\n\n== Layout child heading\n\n#table(columns: 2, [A], [B], [one], [two])\n\n#figure(rect(width: 10pt, height: 10pt), kind: image, caption: [Layout figure caption])\n';
  await api('/api/save', { sessionId: parent.sessionId, baseId: parent.baseId, source: parentSource, operationId: randomUUID() });
  const fixtureRender = await api('/api/render', { sessionId: parent.sessionId, source: parentSource, serial: 1 });
  assert.equal(fixtureRender.ok, true, 'Owned layout fixture must actually compile before any UI result is assessed: ' + JSON.stringify(fixtureRender.diagnostics));
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

  await treeRow('layout-resource.txt').click();
  const resourceDialog = page.locator('.inventory-resource-dialog[open]');
  await resourceDialog.waitFor();
  await until(async () => (await resourceDialog.textContent()).includes('Owned ordinary resource'), 'Files view did not open the exact ordinary resource');
  await resourceDialog.getByRole('button', { name: '关闭', exact: true }).click();
  await expectSource(parentSource);
  pass('Files opens ordinary resources through the same revision-scoped read-only viewer without changing the author');

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

  // The ordinary Outline must consume the same accepted native compile rather
  // than opening the whole-library knowledge/relationship discovery path.
  const outlineKnowledgeReads = knowledgeReads();
  assert.equal(outlineKnowledgeReads, 0, 'Opening and navigating the ordinary Outline started a full knowledge scan');
  await selectTab('view');
  await ribbon().getByRole('button', { name: '源码', exact: true }).click();
  await page.locator('.document-body.mode-source').waitFor();
  await ribbon().locator('.preview-toggle').click();
  assert.equal(await page.locator('.preview-pane').count(), 0, 'The hidden-preview check retained a native preview host');
  assert.equal(await dockView('outline').isVisible(), true);
  const hiddenHeadingText = 'Layout hidden-preview heading', hiddenSource = parentSource + '= ' + hiddenHeadingText;
  const hiddenRenderResponse = page.waitForResponse(response => {
    if (new URL(response.url()).pathname !== '/api/render' || response.status() !== 200) return false;
    try { return response.request().postDataJSON()?.source === hiddenSource; } catch { return false; }
  });
  const hiddenEditor = await source().elementHandle();
  await source().focus(); await page.keyboard.press('Control+End');
  await page.keyboard.type('= ' + hiddenHeadingText); await expectSource(hiddenSource);
  const hiddenRender = await (await hiddenRenderResponse).json();
  assert.equal(hiddenRender.ok, true);
  const hiddenItems = hiddenRender.headingStructure?.items?.filter(item => item.kind === 'heading' && item.text === hiddenHeadingText);
  assert.equal(hiddenItems?.length, 1, 'The actual native render did not publish the authored hidden-preview heading');
  const nativeHiddenHeading = hiddenItems[0];
  assert.ok(nativeHiddenHeading.range, 'The literal native heading did not provide an exact main source range');
  const hiddenRow = () => outline().locator(`li[data-heading-key=${JSON.stringify(nativeHiddenHeading.key)}]`);
  await until(async () => await hiddenRow().count() === 1, 'Outline did not automatically publish the accepted heading while preview was hidden');
  assert.equal(await page.locator('.preview-pane').count(), 0);
  assert.equal(knowledgeReads(), outlineKnowledgeReads, 'Outline updating with preview hidden started a knowledge scan');
  await hiddenRow().locator('.outline-jump').click();
  const hiddenSelection = await editorSnapshot();
  assert.deepEqual([hiddenSelection.from, hiddenSelection.to], [nativeHiddenHeading.range.utf16From, nativeHiddenHeading.range.utf16To], 'Hidden-preview Outline did not select the actual native range');
  assert.equal(hiddenSelection.source, hiddenSource);
  assert.equal(await source().evaluate((element, original) => element === original, hiddenEditor), true);
  await source().focus(); await page.keyboard.press('Control+z'); await expectSource(parentSource);
  await until(async () => await outline().locator('[data-outline-kind="figure"]').count() === 1 && await outline().locator('.outline-row').filter({ hasText: hiddenHeadingText }).count() === 0, 'Outline did not retire the undone heading while preview stayed hidden');
  assert.equal(knowledgeReads(), outlineKnowledgeReads);
  await ribbon().locator('.preview-toggle').click();
  await page.locator('.preview-pane').waitFor({ state: 'visible' });
  await selectTab('home');
  result.outlineProjectionChecks = { knowledgeReadsBefore: outlineKnowledgeReads, knowledgeReadsAfter: knowledgeReads(), previewHidden: true,
    nativeRange: nativeHiddenHeading.range, selectedRange: { from: hiddenSelection.from, to: hiddenSelection.to } };
  pass('Outline updates from the accepted native render without knowledge scans or a preview host and preserves exact source navigation and Undo');
  await page.screenshot({ path: path.join(out, 'workspace-initial-1440.png'), fullPage: false });
  await observeEditorInput();

  // Exercise the actual main.ts host callback twice on the same mounted owner.
  // The second explicit reorder restores the initial shared order; document
  // text/selection/history and the independent filesystem tree stay intact.
  const sortSentinel = 'Layout sort Undo TEST', sortingSource = parentSource + sortSentinel;
  await source().focus(); await page.keyboard.press('Control+End'); await page.keyboard.type(sortSentinel); await expectSource(sortingSource);
  await selectText('Layout selection TEST');
  const sortingEditor = await source().elementHandle(), sortingSelection = await editorSnapshot();
  await revealView('nodes');
  const orderResponse = (route, predicate = () => true) => page.waitForResponse(response => {
    if (new URL(response.url()).pathname !== route || response.request().method() !== 'POST') return false;
    try { return predicate(response.request().postDataJSON()); } catch { return false; }
  });
  const initialOrderResponse = orderResponse('/api/navigation/order/read', body => body.parentRef === rootAdmission.targetRef);
  await nodePath().getByRole('button', { name: path.basename(root), exact: true }).click();
  const initialOrderHttp = await initialOrderResponse; assert.equal(initialOrderHttp.status(), 200);
  let freshOrder = await initialOrderHttp.json();
  assert.equal(freshOrder.complete, true);
  const parentRef = `weftext-node://${parent.workspaceId}/${parent.nodeId}`, siblingRef = `weftext-node://${sibling.workspaceId}/${sibling.nodeId}`;
  assert.deepEqual(new Set(freshOrder.orderedRefs), new Set([parentRef, siblingRef]));
  const initialAuthorOrder = [...freshOrder.orderedRefs];
  await nodes().getByLabel('节点内容个人排序', { exact: true }).selectOption('authored');
  const sortingOwner = await nodes().locator('.node-contents').elementHandle();
  const orderDialog = page.getByRole('dialog', { name: '作者同级顺序变更', exact: true });
  const orderNodeTitles = () => nodes().locator('[data-contents-section="nodes"] .node-content-row [data-control-role="title"]').allTextContents();
  const orderNames = refs => refs.map(ref => `${ref === parentRef ? 'Parent' : 'Sibling'} · 笔记`);
  const orderCheckpoints = [];
  for (let step = 0; step < 2; step++) {
    const prior = freshOrder, movedRef = prior.orderedRefs[1], movedName = movedRef === parentRef ? 'Parent' : 'Sibling';
    await until(async () => JSON.stringify(await orderNodeTitles()) === JSON.stringify(orderNames(prior.orderedRefs)), 'Nodes did not install its exact fresh author order');
    await nodes().getByRole('button', { name: `${movedName} 作者顺序上移`, exact: true }).click();
    const preparedResponse = orderResponse('/api/navigation/order/prepare', body => body.parentRef === rootAdmission.targetRef);
    await orderDialog.getByRole('button', { name: '准备此顺序变更', exact: true }).click();
    const preparedHttp = await preparedResponse; assert.equal(preparedHttp.status(), 200);
    const prepared = await preparedHttp.json(), prepareRequest = preparedHttp.request().postDataJSON();
    assert.equal(prepared.status, 'prepared'); assert.ok(prepared.changes?.length > 0);
    assert.equal(prepareRequest.baseId, prior.baseId, 'The next reorder did not use its actual fresh read basis');
    await orderDialog.waitFor({ state: 'hidden' });
    await nodes().getByRole('button', { name: '查看原顺序变更并确认', exact: true }).click();
    for (const change of prepared.changes) assert.ok((await orderDialog.textContent()).includes(change.path), 'Confirmation omitted the actual prepared file impact');
    const committedResponse = orderResponse('/api/operations/commit', body => body.operationId === prepared.operationId && body.planId === prepared.planId);
    const freshResponse = orderResponse('/api/navigation/order/read', body => body.parentRef === rootAdmission.targetRef);
    await orderDialog.getByRole('button', { name: '确认提交原作者顺序', exact: true }).click();
    const committedHttp = await committedResponse; assert.equal(committedHttp.status(), 200);
    assert.equal((await committedHttp.json()).status, 'committed');
    const freshHttp = await freshResponse; assert.equal(freshHttp.status(), 200); freshOrder = await freshHttp.json();
    assert.equal(freshOrder.complete, true); assert.equal(freshOrder.parentRef, rootAdmission.targetRef);
    assert.deepEqual(freshOrder.orderedRefs, [...prior.orderedRefs].reverse());
    await orderDialog.waitFor({ state: 'hidden' });
    await until(() => page.evaluate(parent => !localStorage.getItem(`weftext-node-contents:${encodeURIComponent(parent)}:operation`), rootAdmission.targetRef), 'Normal sorting did not automatically retire its original operation');
    await until(async () => JSON.stringify(await orderNodeTitles()) === JSON.stringify(orderNames(freshOrder.orderedRefs)), 'Nodes did not show the committed fresh author order');
    assert.equal(await nodes().getByRole('button', { name: '结束已确认顺序操作', exact: true }).count(), 0);
    assert.equal(await nodes().locator('.node-contents').evaluate((element, original) => element === original, sortingOwner), true, 'The real host inventory refresh replaced the sorting owner');
    assert.deepEqual(await page.locator('.file-tree .tree-row .tree-name').allTextContents(), originalOrder, 'Shared author sorting changed the independent filesystem tree');
    // Nodes is the active tab. Inspect the retained Files DOM explicitly; this
    // checks its identity projection without claiming the hidden view is shown.
    for (const [name, filename] of [['Parent', parent.path], ['Sibling', sibling.path]]) {
      const retained = page.locator('.file-tree').getByRole('button', { name, exact: true, includeHidden: true });
      assert.equal(path.normalize((await retained.getAttribute('title')).split('\n')[0]), path.normalize(filename), 'Refreshed file tree lost the exact admitted main note');
    }
    await assertSameEditor(sortingEditor, sortingSelection);
    orderCheckpoints.push({ operationId: prepared.operationId, planId: prepared.planId, prepareBaseId: prepareRequest.baseId, children: prepareRequest.children, freshBaseId: freshOrder.baseId, freshRefs: freshOrder.orderedRefs });
  }
  assert.deepEqual(freshOrder.orderedRefs, initialAuthorOrder, 'The integration check did not restore the original author order');
  assert.notEqual(orderCheckpoints[0].operationId, orderCheckpoints[1].operationId);
  await source().focus(); await page.keyboard.press('Control+z'); await expectSource(parentSource);
  await page.keyboard.press('Control+Shift+z'); await expectSource(sortingSource);
  await page.keyboard.press('Control+z'); await expectSource(parentSource);
  await nodes().locator('.node-browser-toolbar button').first().click();
  assert.equal(path.normalize(await nodePath().getAttribute('title')), path.normalize(parentFolder));
  await showFiles(); await sortingOwner.dispose(); await sortingEditor.dispose();
  result.authorOrderChecks = { parentRef: rootAdmission.targetRef, initialAuthorOrder, operations: orderCheckpoints, restoredAuthorOrder: freshOrder.orderedRefs, exactEditorAndSelectionRetained: true, realUndoRedoRetained: true };
  pass('Two real main-host author reorders retain prepared-impact confirmation, automatically retire with fresh order, preserve the owner/tree/editor and restore the original order and Undo history');

  await selectText('Layout selection TEST');
  const handle = await source().elementHandle(), stable = await editorSnapshot();
  const opens = () => result.requests.filter(item => ['/api/open', '/api/data/open', '/api/bootstrap'].includes(item.route)).length;
  const openCount = opens(), navigations = navigationCount;
  for (const id of ['nodes', 'query', 'timeline', 'outline', 'files']) {
    await revealView(id); await assertSameEditor(handle, stable);
  }
  // Query has the same narrow, vertically shared space as the other dock
  // views. Exercise an actual changed search and its original Core result,
  // rather than proving only that the panel can be mounted at desktop width.
  await revealView('query');
  const queryPanel = dockView('query').locator('.query-panel'), queryResults = queryPanel.locator('.query-results');
  await until(async () => await queryResults.getAttribute('aria-busy') === 'false', 'Initial dock Query did not settle', 60000);
  const wholeLibrary = queryPanel.locator('.query-scope-row').getByRole('button', { name: '全库', exact: true });
  if (await wholeLibrary.isVisible()) {
    await wholeLibrary.click();
    await until(async () => await queryResults.getAttribute('aria-busy') === 'false', 'Whole-library Query scope did not settle', 60000);
  }
  const titleSearch = queryPanel.getByRole('searchbox', { name: '查询标题文字', exact: true });
  await titleSearch.fill('Parent');
  const exactQueryResponse = page.waitForResponse(response => {
    if (new URL(response.url()).pathname !== '/api/query/run' || response.request().method() !== 'POST') return false;
    try { return response.request().postDataJSON()?.plan?.where?.includes('Parent'); } catch { return false; }
  }, { timeout: 60000 });
  await queryPanel.locator('.query-search-form').getByRole('button', { name: '查询', exact: true }).click();
  const actualQueryResponse = await exactQueryResponse;
  assert.equal(actualQueryResponse.status(), 200);
  const actualQuery = await actualQueryResponse.json(), actualQueryRequest = actualQueryResponse.request().postDataJSON();
  assert.equal(actualQuery.rows.length, 1);
  assert.equal(actualQuery.rows[0].entityKey, `weftext-node://${parent.workspaceId}/${parent.nodeId}`);
  assert.equal(actualQuery.coverage, 'complete');
  assert.notEqual(actualQuery.superseded, true);
  await until(async () => await queryResults.getAttribute('aria-busy') === 'false'
    && (await queryResults.locator('.query-basis').allTextContents()).includes(`读取版本：${actualQuery.cutId}`)
    && await queryResults.locator('.query-card').count() === 1, 'Dock Query did not visibly install its exact original result');
  const widthHandle = page.locator('.navigation-sidebar-resizer'), queryLayouts = [];
  const setSidebarWidth = async width => {
    await widthHandle.focus(); await page.keyboard.press('Enter');
    assert.equal(Number(await widthHandle.getAttribute('aria-valuenow')), 264);
    for (let current = 264; current < width; current += 12) await page.keyboard.press('ArrowRight');
    assert.equal(Number(await widthHandle.getAttribute('aria-valuenow')), width);
  };
  for (const width of [264, 420]) {
    await setSidebarWidth(width);
    await titleSearch.focus();
    await queryPanel.locator('.query-heading h2').scrollIntoViewIfNeeded();
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    assert.equal(await titleSearch.evaluate(element => document.activeElement === element), true, 'Narrow Query layout lost search focus');
    const layout = await queryPanel.evaluate(panel => {
      const controls = panel.querySelector('.query-controls'), results = panel.querySelector('.query-results'), heading = panel.querySelector('.query-heading');
      const rect = element => { const r = element.getBoundingClientRect(); return { left: r.left, right: r.right, top: r.top, bottom: r.bottom, width: r.width, height: r.height }; };
      const title = heading.querySelector('h2'), range = document.createRange(); range.selectNodeContents(title);
      return { width: rect(panel).width, panel: rect(panel), controls: rect(controls), results: rect(results),
        title: { ...rect(title), textHeight: range.getBoundingClientRect().height, lineHeight: parseFloat(getComputedStyle(title).lineHeight) },
        buttons: [...heading.querySelectorAll('button')].map(button => ({ text: button.textContent, ...rect(button), fontSize: parseFloat(getComputedStyle(button).fontSize), whiteSpace: getComputedStyle(button).whiteSpace })),
        documentWidth: document.documentElement.scrollWidth, viewportWidth: innerWidth, resultCount: results.querySelectorAll('.query-card').length };
    });
    queryLayouts.push({ requestedWidth: width, ...layout });
    assert.ok(layout.width <= width + 1, 'Query retains a minimum width beyond its dock');
    assert.ok(layout.controls.height > 40 && layout.results.height >= 90, 'Query controls starve the vertically shared result area: ' + JSON.stringify(layout));
    assert.ok(layout.title.textHeight <= layout.title.lineHeight * 1.2, 'Narrow Query title wraps vertically');
    assert.ok(layout.title.top >= layout.controls.top && layout.title.bottom <= layout.controls.bottom, 'Narrow Query title is clipped by its local controls scroller');
    for (const button of layout.buttons) {
      assert.ok(button.fontSize >= 12, 'Narrow Query heading button becomes unreadably small');
      assert.ok(button.left >= layout.panel.left && button.right <= layout.panel.right + 1, 'Narrow Query heading button overflows its dock');
      assert.ok(button.height <= 48, 'Narrow Query heading button text is vertically crushed');
      assert.ok(button.top >= layout.controls.top && button.bottom <= layout.controls.bottom, 'Narrow Query heading button is clipped by the controls scroller');
    }
    assert.ok(layout.documentWidth <= layout.viewportWidth + 1, 'Narrow Query creates global horizontal overflow');
    const cardTitle = queryResults.locator('.query-card .query-row-title'); await cardTitle.scrollIntoViewIfNeeded();
    const titleVisible = await cardTitle.evaluate(element => {
      const title = element.getBoundingClientRect(), viewport = element.closest('.query-results').getBoundingClientRect();
      return Math.min(title.bottom, viewport.bottom) - Math.max(title.top, viewport.top);
    });
    assert.ok(titleVisible >= 18, 'Actual Query card title cannot be reached inside the local result scroll area');
    // A below-the-fold advanced control remains reachable through the local
    // controls scroller while the independent results area stays available.
    const advanced = queryPanel.locator('.query-advanced-plan');
    await advanced.locator('summary').focus(); await page.keyboard.press('Enter');
    await page.keyboard.press('Tab'); await page.keyboard.press('Tab');
    const advancedInput = advanced.locator('textarea.query-plan-input');
    assert.equal(await advancedInput.evaluate(element => document.activeElement === element && element.closest('.query-controls').scrollTop > 0), true);
    assert.ok(await queryResults.evaluate(element => element.getBoundingClientRect().height >= 90));
    await advanced.locator('summary').focus(); await page.keyboard.press('Enter');
    await titleSearch.focus();
    await queryPanel.locator('.query-heading h2').scrollIntoViewIfNeeded();
    await assertSameEditor(handle, stable);
    await page.screenshot({ path: path.join(out, `workspace-query-${width}.png`), fullPage: false });
  }
  await setSidebarWidth(264); await showFiles();
  result.queryLayoutChecks = { request: actualQueryRequest, cutId: actualQuery.cutId, entityKey: actualQuery.rows[0].entityKey, layouts: queryLayouts };
  assert.equal(opens(), openCount); assert.equal(navigationCount, navigations);
  pass('Actual narrow Search keeps horizontal readable controls, local advanced-control scrolling and exact original Query results without replacing the editor');
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
  assert.equal(await dockView('data').evaluate(element => element.isConnected && !!element.closest('[hidden][inert]')), true, 'Hidden property owner was detached rather than parked');
  await assertSameEditor(handle, crossSideEditor);
  assert.equal(await page.locator('.workspace-dock-right .workspace-dock-group').count(), 0, 'Hiding the last right view retained a visible view group');
  // The user may retain an empty drop area. Closing the side is a separate action.
  if (await page.locator('.workspace-dock-right').isVisible()) {
    await selectTab('view');
    await ribbon().locator('[data-toggle-side="right"]').click();
  }
  await page.locator('.workspace-dock-right').waitFor({ state: 'hidden' });
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
  const resultRow = workPanel.locator(`.work-panel-item[data-state="result"][data-id="${actualOutput.id}"]`);
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
  assert.equal(await dockView('outline').isVisible(), true, 'The final Outline stability check requires the visible view');
  await until(async () => await outline().locator('[data-outline-kind="figure"]').count() === 1 && await outline().locator('[data-outline-kind="heading"]').count() >= 2, 'Visible Outline did not recover the current accepted native headings/caption after ordinary edits and Undo');
  pass('Visible Outline recovers current native headings and captions after ordinary editing and Undo');
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
  // Core's active author handle can deny external reads. Drain the actual owned
  // browser and typed Core owner before reading original files; no lock bypass.
  browserClosing = true;
  await browser.close(); browser = undefined;
  await stop();
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
