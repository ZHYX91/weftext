import { workspaceMode } from './_workspace_browser.mjs';
import { spawnServe, serveReady, stopServe, stringifyServeJson } from './_cli_serve.mjs';
import { createdDocument } from './_creation_result.mjs';
// Real Core/DOM group operations through actual toolbar, Shift-click and mouse
// drag gestures. No manufactured drag or clipboard events and no GUI role.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import path from 'node:path';

const dev = process.argv.includes('--dev'), hosted = process.argv.includes('--hosted');
assert.equal(process.argv.length, 4 + Number(dev) + Number(hosted), 'Usage: node scripts/test_table_groups_browser.mjs CLI NEW_OUTPUT [--dev] [--hosted]');
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]);
assert.ok(/^weftext-cli(?:\.exe)?$/i.test(path.basename(exe)), 'Pass the qualified console role, never the desktop executable');
await mkdir(out, { recursive: false });
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const { chromium } = createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const frontendInputs = ['app/src/table-grid.ts', 'app/src/table-axis.ts', 'app/src/writing.ts', 'app/src/writing.css', 'app/src/ui-messages.ts'];
const frontendIdentity = () => Promise.all(frontendInputs.map(async file => ({ path: file, sha256: sha(await readFile(path.resolve(import.meta.dirname, '..', file))) })));
const result = { schemaVersion: 1, startedUtc: new Date().toISOString(), exe, sha256: sha(await readFile(exe)),
  harness: { path: import.meta.filename, sha256: sha(await readFile(import.meta.filename)) }, frontend: dev ? 'current Vite source with separately identified qualified backend' : 'embedded candidate',
  client: hosted ? 'controlled hosted writer' : 'local owner', cases: [], nativeCalls: [], dragEvents: [], pageErrors: [], services: [], saveRequests: [], completed: false,
  limits: ['Headless Chrome with an owned synthetic library; no native desktop, real OS IME, mobile or assistive-technology acceptance.', 'Positive group interactions and complete-axis deletion rejection are covered here; merged/header/budget negative cases remain Engine tests.'] };
// Verify the selected role against both records before invoking any executable.
const batch = path.dirname(exe), catalogBytes = await readFile(path.join(batch, 'artifact-catalog.json')), receiptBytes = await readFile(path.join(batch, 'build-input-receipt.json'));
const catalog = JSON.parse(catalogBytes), receipt = JSON.parse(receiptBytes);
assert.equal(catalog.schemaVersion, 1); assert.equal(receipt.schemaVersion, 1);
assert.equal(catalog.roles?.cli?.filename, path.basename(exe)); assert.equal(receipt.roles?.cli?.filename, path.basename(exe));
assert.equal(catalog.roles.cli.sha256, result.sha256); assert.equal(receipt.roles.cli.sha256, result.sha256);
result.artifacts = { batch, catalogSha256: sha(catalogBytes), receiptSha256: sha(receiptBytes), buildId: catalog.buildId, batchId: receipt.batchId };
result.version = execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim();
if (dev) result.frontendBefore = await frontendIdentity();
const root = path.join(out, 'library'), data = path.join(out, 'state'), hostedToken = randomUUID(), adminToken = randomUUID();
let child, browser, page, vite, base, token, stderr = '', filePath, initial, originalBytes;
const requests = new Map(), responseTasks = [], ownerServices = new WeakMap();
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
async function start(args = []) {
  assert.equal(child, undefined, 'An existing owner cannot be replaced');
  child = spawnServe(exe, ['--data-dir', data, ...args], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  const service = { pid: child.pid, startedUtc: new Date().toISOString() }; result.services.push(service);
  ownerServices.set(child, service);
  child.stderr.on('data', bytes => stderr += bytes);
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    const ready = serveReady(child);
    if (ready) { base = ready.base; token = ready.token; service.ownerIdentity = ready.ownerIdentity; return; }
    if (child.exitCode !== null) throw new Error(stderr);
    await pause(80);
  }
  throw new Error('Owned CLI startup timeout');
}
async function stop() {
  // Take the owner before awaiting. An unknown typed stop is never reissued by finally.
  const owner = child; child = undefined;
  if (!owner) return;
  const service = ownerServices.get(owner);
  assert.ok(service && !service.stopAttempted, 'An original owner stop may be attempted only once');
  service.stopAttempted = true; service.stopAttemptedUtc = new Date().toISOString();
  try { service.stopReceipt = await stopServe(owner); }
  catch (error) { service.stopError = String(error.stack ?? error); throw error; }
  finally { service.actualExitCode = owner.exitCode; service.actualSignal = owner.signalCode; service.stopObservedUtc = new Date().toISOString(); }
}
async function api(route, body) {
  const response = await fetch(base + route, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': token }, body: JSON.stringify(body) }), value = await response.json();
  assert.equal(response.status, 200, route + ': ' + JSON.stringify(value)); return value;
}
const mode = name => workspaceMode(page, hosted ? name === '源码' ? 'V01 源码' : 'V04 可视写作' : name);
const sourceSelector = hosted ? '#source .cm-content' : '.source-host .cm-content';
const surface = () => page.locator('.writing-table-surface').first();
const grid = () => surface().getByRole('grid');
const handle = (axis, index) => surface().getByRole('button', { name: `选择第 ${index + 1} ${axis === 'row' ? '行' : '列'}`, exact: true });
const tool = label => surface().getByRole('button', { name: label, exact: true });
async function source() {
  return page.locator(sourceSelector).evaluate(element => { const view = element.cmTile?.root?.view; if (view?.contentDOM !== element) throw new Error('Actual source editor unavailable'); return view.state.doc.toString(); });
}
async function verifySource(expected) {
  await mode('源码');
  await page.waitForFunction(({ expected, selector }) => { const element = document.querySelector(selector), view = element?.cmTile?.root?.view; return view?.contentDOM === element && view.state.doc.toString() === expected; }, { expected, selector: sourceSelector });
  assert.equal(await source(), expected); assert.deepEqual(result.saveRequests, [], 'Table editing and source Undo/Redo must not send a real save request');
}
async function visual() { await mode('可视编辑'); await grid().waitFor(); await surface().locator('.writing-table-tools[aria-busy="false"]').waitFor(); }
async function matrix() {
  return grid().evaluate(table => {
    const rows = Number(table.getAttribute('aria-rowcount')), columns = Number(table.getAttribute('aria-colcount')), cells = [...table.querySelectorAll('[data-cell-row][data-cell-column]')];
    if (cells.length !== rows * columns || cells.some(cell => cell.rowSpan !== 1 || cell.colSpan !== 1)) throw new Error('Expected a complete plain fixture grid');
    return Array.from({ length: rows }, (_, row) => Array.from({ length: columns }, (_, column) => {
      const cell = cells.find(cell => Number(cell.dataset.cellRow) === row && Number(cell.dataset.cellColumn) === column), input = cell?.querySelector('textarea');
      if (!input) throw new Error('Every fixture cell must have its native literal input'); return input.value;
    }));
  });
}
async function verifyMatrix(expected) {
  await page.waitForFunction(expected => {
    const table = document.querySelector('.writing-table-surface [role="grid"]');
    if (!table || Number(table.getAttribute('aria-rowcount')) !== expected.length || Number(table.getAttribute('aria-colcount')) !== expected[0].length) return false;
    return expected.every((row, r) => row.every((text, c) => table.querySelector(`[data-cell-row="${r}"][data-cell-column="${c}"] textarea`)?.value === text));
  }, expected);
  assert.deepEqual(await matrix(), expected);
}
async function verifySelection(axis, start, end, expectedMatrix) {
  const selected = await grid().locator('[data-cell-row][data-cell-column][aria-selected="true"]').evaluateAll(cells => cells.map(cell => [Number(cell.dataset.cellRow), Number(cell.dataset.cellColumn)]));
  const expected = expectedMatrix.flatMap((row, r) => row.flatMap((_text, c) => (axis === 'row' ? r : c) >= start && (axis === 'row' ? r : c) <= end ? [[r, c]] : []));
  assert.deepEqual(selected, expected, 'The complete moved/inserted group must remain selected');
  assert.deepEqual(await grid().locator(`[data-table-axis="${axis}"][aria-pressed="true"]`).evaluateAll(handles => handles.map(handle => Number(handle.dataset.tableIndex))), Array.from({ length: end - start + 1 }, (_, offset) => start + offset));
}
async function selectGroup(axis, start, end) {
  await handle(axis, start).click();
  await surface().locator('.writing-table-tools[aria-busy="false"]').waitFor();
  await grid().locator(`[data-table-axis="${axis}"][data-table-index="${start}"][aria-pressed="true"]`).waitFor();
  if (end !== start) { await handle(axis, end).click({ modifiers: ['Shift'] }); await surface().locator('.writing-table-tools[aria-busy="false"]').waitFor(); }
  await grid().locator(`[data-table-axis="${axis}"][data-table-index="${end}"][aria-pressed="true"]`).waitFor();
  await verifySelection(axis, start, end, await matrix());
}
const expected = (rowOrder = [0, 1, 2, 3, 4, 5], columnOrder = [0, 1, 2, 3, 4]) => rowOrder.map(row => columnOrder.map(column => row === null || column === null ? '' : `R${row + 1}C${column + 1}`));
const initialMatrix = expected();
const tableRequest = request => request.method() === 'POST' && ['/api/edit', '/api/hosted/editor/edit'].includes(new URL(request.url()).pathname) && request.postDataJSON()?.intent?.kind === 'tableEdit';
async function runSuccess({ name, axis, start = 1, end = 2, label, dragTarget, action, output, selectedStart, selectedEnd }) {
  const item = { name, passed: false }; result.cases.push(item);
  await verifySource(initial); await visual(); await verifyMatrix(initialMatrix); await selectGroup(axis, start, end);
  const before = result.nativeCalls.length, dragBefore = result.dragEvents.length;
  const [response] = await Promise.all([page.waitForResponse(response => tableRequest(response.request())), label ? tool(label).click() : handle(axis, start).dragTo(handle(axis, dragTarget))]);
  assert.equal(response.status(), 200); const body = await response.json(), request = response.request().postDataJSON();
  assert.deepEqual(request.intent.action, action); assert.equal(body.patches.length, 1, 'One group action must produce one bounded native patch');
  const patch = body.patches[0];
  assert.equal(patch.utf16From, request.intent.range.utf16From); assert.equal(patch.utf16To, request.intent.range.utf16To);
  assert.equal(patch.expected, initial.slice(patch.utf16From, patch.utf16To));
  assert.equal(body.source, initial.slice(0, patch.utf16From) + patch.insert + initial.slice(patch.utf16To), 'Prefix, suffix and the following table stay byte-for-byte outside the sole patch');
  await surface().locator('.writing-table-tools[aria-busy="false"]').waitFor(); await verifyMatrix(output); await verifySelection(axis, selectedStart, selectedEnd, output);
  assert.equal(await page.getByRole('dialog').count(), 0, 'Explicit bounded axis operations require no extra confirmation');
  assert.equal(result.nativeCalls.length, before + 1, 'Group operations cannot loop independent single-axis requests');
  if (dragTarget !== undefined) {
    result.dragEvents = await page.evaluate(() => window.__tableGroupDrags);
    const dragEvents = result.dragEvents.slice(dragBefore); assert.ok(dragEvents.some(event => event.type === 'dragstart' && event.trusted)); assert.ok(dragEvents.some(event => event.type === 'drop' && event.trusted));
  }
  await verifySource(body.source); await page.locator(sourceSelector).press('Control+z'); await verifySource(initial);
  await page.locator(sourceSelector).press('Control+Shift+Z'); await verifySource(body.source); await visual(); await verifyMatrix(output);
  await mode('源码'); await page.locator(sourceSelector).press('Control+z'); await verifySource(initial);
  assert.equal(result.nativeCalls.length, before + 1, 'Source Undo/Redo must not rerun the native structural request');
  item.callIndex = before; item.resultSourceSha256 = sha(body.source); item.expectedRows = output.length; item.expectedColumns = output[0].length;
  item.sourceBefore = 'source-before.typ'; item.sourceAfter = `source-case-${result.cases.length.toString().padStart(2, '0')}.typ`;
  await writeFile(path.join(out, item.sourceAfter), body.source, { flag: 'wx' }); item.passed = true;
}
async function rejectCompleteAxis(axis) {
  const item = { name: `Deleting every ${axis} is rejected without a successful edit or author change`, passed: false }; result.cases.push(item);
  await verifySource(initial); await visual(); await selectGroup(axis, 0, axis === 'row' ? 5 : 4);
  const before = result.nativeCalls.length;
  const [response] = await Promise.all([page.waitForResponse(response => tableRequest(response.request())), tool(axis === 'row' ? '删除所选行' : '删除所选列').click()]);
  assert.ok(response.status() >= 400); assert.ok(!(await response.json()).patches?.length);
  await surface().getByRole('alert').waitFor(); assert.ok((await surface().getByRole('alert').innerText()).length > 0);
  await surface().locator('.writing-table-tools[aria-busy="false"]').waitFor(); await verifyMatrix(initialMatrix); await verifySource(initial);
  assert.equal(result.nativeCalls.length, before + 1); item.callIndex = before; item.sourceBefore = item.sourceAfter = 'source-before.typ'; item.passed = true;
}
try {
  await start(); await api('/api/library/create', { path: root, operationId: randomUUID() });
  const doc = createdDocument(await api('/api/note/create', { root, parent: root, name: 'Table Groups', operationId: randomUUID() })); filePath = doc.path;
  initial = doc.source + '\n前方原件😀。\n\n#table(columns: 5,\n' + initialMatrix.map(row => '  ' + row.map(text => `[${text}]`).join(', ')).join(',\n') + '\n)\n\n后方原件😀。\n\n#table(columns: 2, [SECOND_A], [SECOND_B])\n';
  await writeFile(path.join(out, 'source-before.typ'), initial, { flag: 'wx' });
  await api('/api/save', { sessionId: doc.sessionId, baseId: doc.baseId, source: initial, operationId: randomUUID() }); await stop();
  originalBytes = await readFile(filePath);
  assert.deepEqual(originalBytes, Buffer.from(initial, 'utf8'), 'The normally stopped fixture setup must have saved the exact initial source');
  result.originalBefore = { path: filePath, physicalSha256: sha(originalBytes), bytes: originalBytes.length, checkedAfterSetupOwnerStop: true };
  if (hosted) {
    const config = { deployment: 'table-groups', bind: '127.0.0.1:0', root, adminTokenSha256: sha(adminToken), files: { table: { path: path.relative(root, doc.path).replaceAll('\\', '/'), title: '表格成组文档', editable: true } }, users: { writer: { enabled: true, tokenSha256: sha(hostedToken), grants: { table: { read: true, write: true } } } } };
    const configPath = path.join(out, 'server.json'); await writeFile(configPath, JSON.stringify(config)); await start(['--server-config', configPath]);
  } else await start(['--open', doc.path, '--root', root]);
  let frontend = base;
  if (dev) {
    const app = path.resolve(import.meta.dirname, '../app'), { createServer } = await import(pathToFileURL(path.join(app, 'node_modules/vite/dist/node/index.js')).href);
    const proxy = { target: base, changeOrigin: true, configure(instance) { instance.on('proxyReq', (request, incoming) => { if (incoming.headers.origin) request.setHeader('Origin', base); }); } };
    vite = await createServer({ root: app, configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false, watch: { ignored: ['**/*'] }, proxy: { '/api': proxy, '/fonts': proxy } } });
    await vite.listen(); frontend = 'http://127.0.0.1:' + vite.httpServer.address().port;
  }
  browser = await chromium.launch({ channel: 'chrome', headless: true }); result.browser = browser.version();
  page = await browser.newPage({ viewport: { width: 1500, height: 1100 } }); page.setDefaultTimeout(30000); page.on('pageerror', error => result.pageErrors.push(String(error)));
  page.on('request', request => {
    const pathname = new URL(request.url()).pathname;
    if (request.method() === 'POST' && ['/api/save', '/api/save-as', '/api/hosted/save', '/api/hosted/save-as'].includes(pathname)) result.saveRequests.push({ path: pathname });
    if (!tableRequest(request)) return;
    const body = request.postDataJSON(), call = { path: new URL(request.url()).pathname, action: body.intent.action, requestSourceSha256: sha(body.source), requestSourceLength: body.source.length };
    requests.set(request, call); result.nativeCalls.push(call);
  });
  page.on('response', response => {
    const call = requests.get(response.request()); if (!call) return;
    const task = (async () => {
      call.status = response.status(); const body = await response.json();
      call.patchCount = body.patches?.length ?? 0;
      if (body.source !== undefined) call.resultSourceSha256 = sha(body.source);
      if (body.patches) call.patches = body.patches.map(patch => ({ from: patch.from, to: patch.to, utf16From: patch.utf16From, utf16To: patch.utf16To, expectedSha256: sha(patch.expected), insertSha256: sha(patch.insert) }));
      else call.error = body;
    })();
    // Observe rejection immediately so a failed response cannot bypass finally.
    // Keep the original Promise: the later Promise.all still fails this check.
    void task.catch(() => {});
    responseTasks.push(task);
  });
  await page.goto(frontend + (hosted ? '/hosted.html' : '/#token=' + token));
  if (hosted) { await page.getByLabel('访问凭据', { exact: true }).fill(hostedToken); await page.getByRole('button', { name: '连接', exact: true }).click(); await page.getByRole('button', { name: '表格成组文档', exact: true }).waitFor(); await page.waitForLoadState('networkidle'); await page.getByRole('button', { name: '表格成组文档', exact: true }).click(); await page.locator('#source .cm-content[contenteditable=true]').waitFor(); }
  await page.evaluate(() => { window.__tableGroupDrags = []; for (const type of ['dragstart', 'drop']) document.addEventListener(type, event => { const handle = event.target?.closest?.('[data-table-axis]'); if (handle) window.__tableGroupDrags.push({ type, trusted: event.isTrusted, axis: handle.dataset.tableAxis, index: Number(handle.dataset.tableIndex) }); }, true); });
  for (const entry of [
    { name: 'Insert two rows above a Shift-selected group', axis: 'row', label: '上方加行', action: { kind: 'insertRows', index: 1, count: 2 }, output: expected([0, null, null, 1, 2, 3, 4, 5]), selectedStart: 1, selectedEnd: 2 },
    { name: 'Insert two rows below a Shift-selected group', axis: 'row', label: '下方加行', action: { kind: 'insertRows', index: 3, count: 2 }, output: expected([0, 1, 2, null, null, 3, 4, 5]), selectedStart: 3, selectedEnd: 4 },
    { name: 'Delete two nonempty rows as one operation', axis: 'row', label: '删除所选行', action: { kind: 'removeRows', start: 1, end: 2 }, output: expected([0, 3, 4, 5]), selectedStart: 1, selectedEnd: 1 },
    { name: 'Move the selected row group up with the toolbar', axis: 'row', label: '行上移', action: { kind: 'moveRows', start: 1, end: 2, to: 0 }, output: expected([1, 2, 0, 3, 4, 5]), selectedStart: 0, selectedEnd: 1 },
    { name: 'Move the selected row group down with the toolbar', axis: 'row', label: '行下移', action: { kind: 'moveRows', start: 1, end: 2, to: 2 }, output: expected([0, 3, 1, 2, 4, 5]), selectedStart: 2, selectedEnd: 3 },
    { name: 'Drag a selected row handle downward and move the entire group', axis: 'row', dragTarget: 5, action: { kind: 'moveRows', start: 1, end: 2, to: 4 }, output: expected([0, 3, 4, 5, 1, 2]), selectedStart: 4, selectedEnd: 5 },
    { name: 'Drag a selected row group upward', axis: 'row', start: 3, end: 4, dragTarget: 0, action: { kind: 'moveRows', start: 3, end: 4, to: 0 }, output: expected([3, 4, 0, 1, 2, 5]), selectedStart: 0, selectedEnd: 1 },
    { name: 'Insert two columns to the left of a Shift-selected group', axis: 'column', label: '左侧加列', action: { kind: 'insertColumns', index: 1, count: 2 }, output: expected(undefined, [0, null, null, 1, 2, 3, 4]), selectedStart: 1, selectedEnd: 2 },
    { name: 'Insert two columns to the right of a Shift-selected group', axis: 'column', label: '右侧加列', action: { kind: 'insertColumns', index: 3, count: 2 }, output: expected(undefined, [0, 1, 2, null, null, 3, 4]), selectedStart: 3, selectedEnd: 4 },
    { name: 'Delete two nonempty columns as one operation', axis: 'column', label: '删除所选列', action: { kind: 'removeColumns', start: 1, end: 2 }, output: expected(undefined, [0, 3, 4]), selectedStart: 1, selectedEnd: 1 },
    { name: 'Move the selected column group left with the toolbar', axis: 'column', label: '列左移', action: { kind: 'moveColumns', start: 1, end: 2, to: 0 }, output: expected(undefined, [1, 2, 0, 3, 4]), selectedStart: 0, selectedEnd: 1 },
    { name: 'Move the selected column group right with the toolbar', axis: 'column', label: '列右移', action: { kind: 'moveColumns', start: 1, end: 2, to: 2 }, output: expected(undefined, [0, 3, 1, 2, 4]), selectedStart: 2, selectedEnd: 3 },
    { name: 'Drag a selected column handle rightward and move the entire group', axis: 'column', dragTarget: 4, action: { kind: 'moveColumns', start: 1, end: 2, to: 3 }, output: expected(undefined, [0, 3, 4, 1, 2]), selectedStart: 3, selectedEnd: 4 },
    { name: 'Drag a selected column group leftward', axis: 'column', start: 2, end: 3, dragTarget: 0, action: { kind: 'moveColumns', start: 2, end: 3, to: 0 }, output: expected(undefined, [2, 3, 0, 1, 4]), selectedStart: 0, selectedEnd: 1 },
  ]) await runSuccess(entry);
  await rejectCompleteAxis('row'); await rejectCompleteAxis('column');
  await Promise.all(responseTasks); assert.equal(result.nativeCalls.filter(call => call.status === 200).length, 14); assert.equal(result.nativeCalls.filter(call => call.status >= 400).length, 2);
  assert.deepEqual(result.pageErrors, []); if (dev) { result.frontendAfter = await frontendIdentity(); assert.deepEqual(result.frontendAfter, result.frontendBefore); }
  // Windows may deny external reads of an active Core author handle. Check the
  // saved original only after the actual browser and typed owner have drained.
  await browser.close(); browser = undefined;
  await stop();
  assert.deepEqual(result.saveRequests, [], 'No real save request may be sent during the task');
  const physicalBytes = await readFile(filePath);
  assert.deepEqual(physicalBytes, originalBytes, 'Group edits and Undo/Redo must leave the original saved file byte-for-byte unchanged');
  result.originalFile = { path: filePath, expectedSha256: sha(originalBytes), physicalSha256: sha(physicalBytes), bytes: physicalBytes.length, checkedAfterOwnerStop: true };
  result.completed = true;
} catch (error) {
  result.failure = String(error.stack ?? error); process.exitCode = 1;
  if (page) { await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {}); await writeFile(path.join(out, 'failure-ui.txt'), await page.locator('body').innerText().catch(() => 'unavailable')); }
} finally {
  result.cleanupErrors = [];
  for (const [name, close] of [['browser', () => browser?.close()], ['vite', () => vite?.close()], ['owned CLI', stop]]) {
    try { await close(); } catch (error) { result.cleanupErrors.push({ name, error: String(error.stack ?? error) }); process.exitCode = 1; }
  }
  await Promise.allSettled(responseTasks);
  result.ownedProcessesClosed = result.services.every(service => service.stopAttempted && service.stopReceipt && service.actualExitCode === 0 && !service.actualSignal) && (!browser || !browser.isConnected());
  if (!result.completed || !result.ownedProcessesClosed || result.cleanupErrors.length > 0) process.exitCode = 1;
  result.finishedUtc = new Date().toISOString(); await writeFile(path.join(out, 'result.json'), stringifyServeJson(result, 2)); await writeFile(path.join(out, 'server.stderr.log'), stderr);
}
console.log(JSON.stringify({ out, completed: result.completed, cases: result.cases.length, ownedProcessesClosed: result.ownedProcessesClosed, error: result.failure }));
