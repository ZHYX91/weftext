// Current-source, real headless DOM checks over a controlled inventory. This is
// not full-server, native-host, OS IME or authorization acceptance.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
const out = path.resolve(process.argv[2]);
assert.ok(path.isAbsolute(process.argv[2]), 'Use an absolute, new private output directory');
await mkdir(out, { recursive: false });
const repo = path.resolve(import.meta.dirname, '..');
const require = createRequire(import.meta.url), { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
const vite = await createServer({ root: path.join(repo, 'app'), configFile: false,
  server: { host: '127.0.0.1', port: 0, hmr: false }, plugins: [{ name: 'selection-fixture', configureServer(server) {
    server.middlewares.use('/__selection_test__', (_request, response) => {
      response.setHeader('Content-Type', 'text/html');
      response.end('<!doctype html><meta charset="utf-8"><textarea aria-label="Independent author">Preserved author</textarea><section aria-label="Contents"></section>');
    });
  } }] });
const result = { cases: [], pageErrors: [], completed: false,
  limits: ['Controlled inventory/current-source DOM checks only; no full-server, native host, real OS IME or new authorization qualification.'] };
let browser, page;
const pass = message => result.cases.push(message);
try {
  await vite.listen();
  browser = await chromium.launch({ headless: true, ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : { channel: 'chrome' }) });
  result.browser = browser.version();
  page = await browser.newPage({ viewport: { width: 1100, height: 1000 } });
  page.on('pageerror', error => result.pageErrors.push(String(error)));
  await page.goto('http://127.0.0.1:' + vite.httpServer.address().port + '/__selection_test__');
  await page.evaluate(async () => {
    const { mountNodeContents } = await import('/src/node-contents.ts');
    const root = 'D:/Owned/Selection', folder = root + '/Parent';
    const entry = (name, kind = 'file', identityStatus = 'ordinary') => ({ path: folder + '/' + name, parentPath: folder, relativePath: 'Parent/' + name, name, kind, identityStatus });
    const entries = [entry('X-conflict.typ', 'node', 'bindingConflict'), entry('Y-node', 'directory', 'unmanaged'), entry('Z-node', 'directory', 'unmanaged'), entry('a-image.png'), entry('b-blocked.png', 'blocked', 'unsupported'),
      ...Array.from({ length: 26 }, (_, index) => entry('file-' + String(index).padStart(2, '0') + '.txt'))];
    for (const name of ['Y-node', 'Z-node']) entries.push({ ...entry(name + '.typ', 'node', 'admitted'), path: folder + '/' + name + '/' + name + '.typ', parentPath: folder + '/' + name, nodeId: 'owned-' + name });
    const fixture = window.selectionFixture = { root, folder, entries, selections: new Set(), opened: [], menus: [], menuScopes: [], requests: [], controller: null };
    const inventory = { root, workspaceId: 'owned-selection-workspace', status: 'localMvp', complete: false, warnings: [], entries };
    window.mountSelection = query => {
      fixture.controller?.dispose();
      fixture.controller = mountNodeContents(document.querySelector('section'), inventory, folder, query || '', fixture.selections, {
        request: async (route, body) => { fixture.requests.push({ route, body }); if (route === '/api/navigation/capture') return { status: 'unmanaged' }; throw Error('Unexpected selection route ' + route); },
        readBlob: async () => { throw Error('Selection must not read an author or resource'); },
        open: entry => fixture.opened.push(structuredClone(entry)), browse: () => {},
        menu: (entries, _folder, isSelectionCurrent) => { fixture.menus.push(structuredClone(entries)); fixture.menuScopes.push(isSelectionCurrent); }, preview: () => {}, changed: async () => {}, notify: () => {},
      });
    };
    window.mountSelection();
  });
  const rows = () => page.locator('.node-content-row');
  const row = name => rows().filter({ has: page.locator('[data-control-role="title"]', { hasText: name + ' ·' }) });
  const title = name => row(name).locator('[data-control-role="title"]');
  const selected = () => page.evaluate(() => [...window.selectionFixture.selections].map(value => value.slice(window.selectionFixture.folder.length + 1)).sort());
  const clear = () => page.getByRole('button', { name: '清除选择', exact: true }).click();
  const all = () => page.getByRole('button', { name: '选择全部已显示合格项', exact: true }).click();
  const actions = () => page.getByRole('button', { name: '已显示选中项操作', exact: true }).click();
  await row('Y-node').getByRole('checkbox').click();
  await title('a-image.png').click({ modifiers: ['Shift'] });
  assert.deepEqual(await selected(), ['Y-node', 'Z-node', 'a-image.png']);
  assert.equal(await page.evaluate(() => window.selectionFixture.opened.length), 0);
  await title('a-image.png').click({ modifiers: ['Control'] });
  assert.deepEqual(await selected(), ['Y-node', 'Z-node']);
  await title('Z-node').click({ modifiers: ['Meta'] });
  assert.deepEqual(await selected(), ['Y-node']);
  await title('Z-node').click();
  assert.deepEqual(await selected(), ['Z-node']);
  assert.deepEqual(await page.evaluate(() => window.selectionFixture.opened.map(entry => entry.path)), ['D:/Owned/Selection/Parent/Z-node/Z-node.typ']);
  for (const modifier of ['Control', 'Meta']) {
    if ((await selected()).length) await clear();
    await title('Z-node').dblclick({ modifiers: [modifier] });
    assert.deepEqual(await selected(), []);
    assert.equal(await page.evaluate(() => window.selectionFixture.opened.length), 1);
  }
  pass('Shift follows actual section display order; Ctrl/Meta toggle without opening, while an ordinary healthy-node click opens its exact admitted source.');
  await row('Y-node').getByRole('checkbox').click();
  await title('a-image.png').focus(); await page.keyboard.press('Shift+Space');
  assert.deepEqual(await selected(), ['Y-node', 'Z-node', 'a-image.png']);
  await title('file-00.txt').focus(); await page.keyboard.press('Control+Shift+Space');
  assert.deepEqual(await selected(), ['Y-node', 'Z-node', 'a-image.png', 'file-00.txt']);
  await title('Y-node').press('Shift+F10');
  assert.deepEqual(await page.evaluate(() => window.selectionFixture.menus.at(-1).map(entry => entry.name)), ['Y-node.typ', 'Z-node.typ', 'a-image.png', 'file-00.txt']);
  assert.equal(await page.evaluate(() => window.selectionFixture.opened.length), 1);
  pass('Keyboard Shift+Space extends once, Ctrl+Shift+Space adds a range, and Shift+F10 preserves the qualified multi-selection.');
  await clear(); await row('X-conflict.typ').getByRole('checkbox').click();
  await title('a-image.png').click({ modifiers: ['Shift'] });
  assert.deepEqual(await selected(), ['a-image.png']);
  await clear(); await title('file-00.txt').focus(); await page.keyboard.press('Control+a');
  assert.equal((await selected()).includes('X-conflict.typ'), false);
  assert.equal((await selected()).includes('b-blocked.png'), false);
  assert.equal((await selected()).includes('file-25.txt'), false);
  await actions();
  const firstBatch = await page.evaluate(() => window.selectionFixture.menus.at(-1).map(entry => entry.name));
  assert.deepEqual(firstBatch.slice(0, 3), ['Y-node.typ', 'Z-node.typ', 'a-image.png']);
  assert.equal(firstBatch.length, 26);
  assert.equal(firstBatch.includes('file-23.txt'), false);
  assert.ok((await page.locator('[data-control-role="selection-summary"]').textContent()).includes('已显示 28 项，选中 26 项，其中 26 项'));
  assert.ok((await page.locator('.node-selection-controls').textContent()).includes('当前清单扫描不完整'));
  pass('Select-all is bounded by the displayed page and excludes blocked or conflicted identities; partial inventory and exact displayed counts stay explicit.');
  await page.evaluate(() => window.mountSelection('file-25'));
  assert.equal(await rows().count(), 1);
  await all(); await actions();
  assert.deepEqual(await page.evaluate(() => window.selectionFixture.menus.at(-1).map(entry => entry.name)), ['file-25.txt']);
  assert.equal((await selected()).length, 27);
  assert.ok((await page.locator('[data-control-role="selection-summary"]').textContent()).includes('另保留 26 个未显示的选择'));
  await clear(); assert.deepEqual(await selected(), []);
  pass('Filtered-out selections are retained and counted but never enter the displayed bulk payload; Clear removes both shown and retained selections.');
  await page.evaluate(() => window.mountSelection());
  await row('Y-node').getByRole('checkbox').click(); await row('a-image.png').getByRole('checkbox').click();
  await page.locator('[data-section-control="album-toggle"]').click();
  await title('Y-node').press('Shift+F10');
  assert.deepEqual(await page.evaluate(() => window.selectionFixture.menus.at(-1).map(entry => entry.name)), ['Y-node.typ']);
  assert.deepEqual(await selected(), ['Y-node', 'a-image.png']);
  await page.locator('[data-section-control="nodes-toggle"]').click();
  await title('file-00.txt').click({ modifiers: ['Shift'] });
  assert.deepEqual(await selected(), ['file-00.txt']);
  pass('Collapsed sections are excluded from row menus and range anchors even though their preserved rows remain mounted in the DOM.');
  await page.locator('[data-section-control="nodes-toggle"]').click();
  await page.locator('[data-section-control="album-toggle"]').click();
  await page.locator('[data-section-control="files-more"]').click();
  await all(); await actions();
  assert.ok(await page.evaluate(() => window.selectionFixture.menus.at(-1).some(entry => entry.name === 'file-25.txt')));
  await clear(); await row('X-conflict.typ').getByRole('checkbox').click();
  await title('X-conflict.typ').press('Shift+F10');
  assert.deepEqual(await page.evaluate(() => window.selectionFixture.menus.at(-1).map(entry => entry.identityStatus)), ['bindingConflict']);
  assert.equal(await page.getByRole('button', { name: '已显示选中项操作', exact: true }).isDisabled(), true);
  pass('Explicitly displayed additional pages join select-all; an anomalous single item remains inspectable without becoming a qualified bulk target.');
  const beforeIme = await selected();
  for (const options of [{ key: 'a', ctrlKey: true, isComposing: true }, { key: 'a', ctrlKey: true, keyCode: 229 }])
    await title('file-00.txt').evaluate((element, options) => element.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...options })), options);
  assert.deepEqual(await selected(), beforeIme);
  assert.equal(await page.getByRole('textbox', { name: 'Independent author', exact: true }).inputValue(), 'Preserved author');
  assert.equal(await page.evaluate(() => window.selectionFixture.requests.every(item => item.route === '/api/navigation/capture')), true);
  assert.deepEqual(result.pageErrors, []);
  pass('Composition shortcut guards leave selection and the independent author intact; selection issued no reads, prepares or author writes.');
  await clear(); await row('Y-node').getByRole('checkbox').click(); await actions();
  const toolbarScope = await page.evaluate(() => window.selectionFixture.menuScopes.length - 1);
  assert.equal(await page.evaluate(index => window.selectionFixture.menuScopes[index](), toolbarScope), true);
  await page.locator('input[data-preference="summaries"]').check();
  assert.equal(await page.evaluate(index => window.selectionFixture.menuScopes[index](), toolbarScope), true, 'An equivalent row redraw must not retire a valid selection');
  await title('Y-node').press('Shift+F10');
  const rowScope = await page.evaluate(() => window.selectionFixture.menuScopes.length - 1);
  assert.equal(await page.evaluate(index => window.selectionFixture.menuScopes[index](), toolbarScope), true, 'Opening a menu on an already selected row is not a new selection intent');
  assert.equal(await page.evaluate(index => window.selectionFixture.menuScopes[index](), rowScope), true);
  pass('Both toolbar and row menus capture live selection scopes; identical redraws and opening the selected row menu preserve their validity.');
  await page.evaluate(index => {
    const fixture = window.selectionFixture, current = fixture.menuScopes[index];
    fixture.lateDelivery = new Promise(resolve => { fixture.completeLateDelivery = () => resolve(current()); });
  }, rowScope);
  await title('Y-node').click({ modifiers: ['Control'] }); await title('Y-node').click({ modifiers: ['Control'] });
  assert.deepEqual(await selected(), ['Y-node']);
  assert.equal(await page.evaluate(index => window.selectionFixture.menuScopes[index](), toolbarScope), false);
  await page.evaluate(() => window.selectionFixture.completeLateDelivery());
  assert.equal(await page.evaluate(() => window.selectionFixture.lateDelivery), false, 'A late result must be rejected even after selection ABA restores the same paths');
  await actions(); const displayScope = await page.evaluate(() => window.selectionFixture.menuScopes.length - 1);
  await page.locator('[data-section-control="album-toggle"]').click(); await page.locator('[data-section-control="album-toggle"]').click();
  assert.equal(await page.evaluate(index => window.selectionFixture.menuScopes[index](), displayScope), false, 'Displayed-list ABA must not revive a captured batch');
  pass('Explicit selection ABA and displayed-list ABA retire old scopes permanently, and an awaited late delivery sees the original scope as stale.');
  await all(); await actions(); const intentScope = await page.evaluate(() => window.selectionFixture.menuScopes.length - 1);
  await all();
  assert.equal(await page.evaluate(index => window.selectionFixture.menuScopes[index](), intentScope), false, 'An explicit select-all intent invalidates an older batch even when the set already matches');
  await actions(); const filteredScope = await page.evaluate(() => window.selectionFixture.menuScopes.length - 1);
  await page.evaluate(() => window.mountSelection('file-25'));
  assert.equal(await page.evaluate(index => window.selectionFixture.menuScopes[index](), filteredScope), false);
  await all(); await actions(); const disposedScope = await page.evaluate(() => window.selectionFixture.menuScopes.length - 1);
  await page.evaluate(() => window.selectionFixture.controller.dispose());
  assert.equal(await page.evaluate(index => window.selectionFixture.menuScopes[index](), disposedScope), false);
  pass('No-op explicit select-all, filter replacement and owner disposal cannot keep an old asynchronous menu batch current.');
  assert.deepEqual(result.pageErrors, []);
  await page.screenshot({ path: path.join(out, 'selection.png'), fullPage: true });
  result.completed = true;
} catch (error) {
  result.failure = String(error.stack || error);
  if (page) await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {});
  throw error;
} finally {
  await browser?.close(); await vite.close();
  await writeFile(path.join(out, 'results.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify({ out, completed: result.completed, cases: result.cases.length, failure: result.failure }));
}
