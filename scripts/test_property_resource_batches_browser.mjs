// Owned end-to-end check against the selected CLI's embedded workspace and real Core.
// No Vite/mock server, desktop input, real IME or assistive-technology claim.
import assert from 'node:assert/strict';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { deflateSync } from 'node:zlib';
import { workspaceCommand, workspaceMode, workspaceTab } from './_workspace_browser.mjs';
import { spawnServe, serveReady, stopServe, stringifyServeJson } from './_cli_serve.mjs';
import { createdDocument } from './_creation_result.mjs';

const repo = path.resolve(import.meta.dirname, '..');
const exe = process.env.WEFTEXT_TEST_EXECUTABLE;
const out = process.env.WEFTEXT_BROWSER_REPORT;
assert.ok(exe && path.isAbsolute(exe) && out && path.isAbsolute(out), 'Set absolute WEFTEXT_TEST_EXECUTABLE and new WEFTEXT_BROWSER_REPORT');
assert.equal(process.argv.length, 2, 'This check uses environment variables and the embedded frontend only');
assert.match(path.basename(exe), /^weftext-cli\.exe$/i, 'Use the Windows CLI role, not the desktop executable');
const relativeOut = path.relative(repo, out);
assert.ok(path.isAbsolute(relativeOut) || relativeOut.startsWith('..' + path.sep), 'The fresh private report must be outside the product repository');
await mkdir(out, { recursive: false });
const sha = value => createHash('sha256').update(value).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const batch = path.dirname(exe), catalogPath = path.join(batch, 'artifact-catalog.json'), receiptPath = path.join(batch, 'build-input-receipt.json');
const executableBytes = await readFile(exe), catalogBytes = await readFile(catalogPath), receiptBytes = await readFile(receiptPath);
const executableSha256 = sha(executableBytes), catalog = JSON.parse(catalogBytes), receipt = JSON.parse(receiptBytes);
assert.equal(catalog.schemaVersion, 1); assert.equal(receipt.schemaVersion, 1);
assert.equal(receipt.scope, 'same-checkout-frozen-artifacts-not-reproducible-build-or-check-results');
assert.equal(path.resolve(receipt.sourceRoot).toLowerCase(), repo.toLowerCase());
assert.equal(receipt.batchId, path.basename(batch));
assert.equal(path.resolve(receipt.targetRoot, 'weftext-build-artifacts', receipt.batchId).toLowerCase(), path.resolve(batch).toLowerCase());
assert.equal(receipt.target, 'x86_64-pc-windows-msvc');
assert.deepEqual(receipt.features, []);
assert.equal(receipt.catalog?.filename, 'artifact-catalog.json');
assert.equal(receipt.catalog?.bytes, catalogBytes.length); assert.equal(receipt.catalog?.sha256, sha(catalogBytes));
for (const record of [catalog.roles?.cli, receipt.roles?.cli]) {
  assert.equal(record?.filename, path.basename(exe)); assert.equal(record?.bytes, executableBytes.length); assert.equal(record?.sha256, executableSha256);
}
assert.equal(catalog.roles.cli.peSubsystem, 3);
assert.equal(executableBytes.subarray(0, 2).toString('ascii'), 'MZ');
const peOffset = executableBytes.readUInt32LE(0x3c);
assert.ok(peOffset > 0 && peOffset + 94 <= executableBytes.length);
assert.equal(executableBytes.subarray(peOffset, peOffset + 4).toString('hex'), '50450000');
assert.equal(executableBytes.readUInt16LE(peOffset + 4), 0x8664);
assert.equal(executableBytes.readUInt16LE(peOffset + 24), 0x20b);
assert.equal(executableBytes.readUInt16LE(peOffset + 92), 3, 'The executable itself must be a console-subsystem PE');
if (process.env.WEFTEXT_TEST_EXECUTABLE_SHA256) assert.equal(executableSha256, process.env.WEFTEXT_TEST_EXECUTABLE_SHA256.toLowerCase());
const inputs = [
  'app/src/data-panel.ts', 'app/src/field-transfer.ts', 'app/src/field-transfer-state.ts', 'app/src/native-value-form.ts',
  'app/src/resource-import-batch.ts', 'app/src/resource-import-batch-state.ts', 'app/src/workspace-tools.ts', 'app/src/ui-messages.ts',
  'crates/weftext-app/src/data_api.rs', 'crates/weftext-app/src/field_transfer_api.rs', 'crates/weftext-app/src/resource_import_api.rs',
  'crates/weftext-core/src/operations/resource_import.rs', 'crates/weftext-data/tests/field_transfer.rs',
];
const localInputs = ['scripts/test_property_resource_batches_browser.mjs', 'scripts/_workspace_browser.mjs', 'scripts/_cli_serve.mjs', 'scripts/_creation_result.mjs'];
const hashes = {};
for (const file of [...inputs, ...localInputs]) hashes[file] = sha(await readFile(path.join(repo, file)));
for (const file of inputs) {
  const entries = receipt.source.filter(entry => entry.path === file);
  assert.equal(entries.length, 1, `Build receipt must identify ${file}`);
  assert.equal(entries[0].sha256, hashes[file], `Embedded candidate does not match current ${file}`);
}
const result = {
  schemaVersion: 1, startedUtc: new Date().toISOString(), exe, executableSha256,
  artifacts: { batch, buildId: catalog.buildId, batchId: receipt.batchId, catalogSha256: sha(catalogBytes), receiptSha256: sha(receiptBytes) },
  sourceInputs: hashes, frontend: 'selected CLI embedded workspace', cases: [], requests: [], pageErrors: [], services: [], completed: false,
  limits: ['Owned synthetic library and headless Chrome only; no native desktop, real OS IME or assistive-technology acceptance.', 'No author save after fixture setup; component recovery and negative paths are covered separately.'],
};
await writeFile(path.join(out, 'inputs.json'), stringifyServeJson(result, 2), { flag: 'wx' });
const { chromium } = createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const root = path.join(out, 'library'), data = path.join(out, 'state');
let child, browser, page, base, token, stderr = '', doc, initialBytes;
const ownerServices = new WeakMap(), requests = new Map(), responseTasks = [];
const traced = new Set(['/api/data/edit', '/api/data/transfer', '/api/resources/prepare-import-batch', '/api/operations/commit', '/api/operations/status', '/api/resources/import-batch/status', '/api/editor-map', '/api/resources/import-batch/insert', '/api/draft', '/api/save', '/api/save-as']);

async function start(args = []) {
  assert.equal(child, undefined);
  child = spawnServe(exe, ['--data-dir', data, ...args], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  const service = { pid: child.pid, startedUtc: new Date().toISOString() }; result.services.push(service); ownerServices.set(child, service);
  child.stderr.on('data', bytes => { stderr += bytes.toString(); });
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    const ready = serveReady(child);
    if (ready) { base = ready.base; token = ready.token; service.ownerIdentity = ready.ownerIdentity; return; }
    if (child.exitCode !== null) throw new Error(`Owned CLI exited before Ready: ${stderr}`);
    await pause(80);
  }
  throw new Error('Owned CLI Ready timeout; retain the owner and report, never guess an identity');
}
async function stop() {
  const owner = child; child = undefined;
  if (!owner) return;
  const service = ownerServices.get(owner); assert.ok(service && !service.stopAttempted);
  service.stopAttempted = true;
  try { service.stopReceipt = await stopServe(owner); assert.equal(owner.exitCode, 0); assert.equal(owner.signalCode, null); }
  catch (error) { service.stopError = String(error.stack ?? error); throw error; }
  finally { service.actualExitCode = owner.exitCode; service.actualSignal = owner.signalCode; service.stopObservedUtc = new Date().toISOString(); }
}
async function api(route, body) {
  const response = await fetch(base + route, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': token }, body: JSON.stringify(body), signal: AbortSignal.timeout(45000) });
  const value = await response.json(); assert.equal(response.status, 200, `${route}: ${JSON.stringify(value)}`); return value;
}
function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) { crc ^= byte; for (let i = 0; i < 8; i++) crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1)); }
  return (crc ^ 0xffffffff) >>> 0;
}
function pngChunk(name, payload) {
  const type = Buffer.from(name), size = Buffer.alloc(4), checksum = Buffer.alloc(4);
  size.writeUInt32BE(payload.length); checksum.writeUInt32BE(crc32(Buffer.concat([type, payload])));
  return Buffer.concat([size, type, payload, checksum]);
}
function fixturePng() {
  const header = Buffer.alloc(13); header.writeUInt32BE(2, 0); header.writeUInt32BE(2, 4); header[8] = 8; header[9] = 6;
  const rows = Buffer.from([0, 255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 0, 255, 255, 255, 255, 0, 255]);
  return Buffer.concat([Buffer.from('89504e470d0a1a0a', 'hex'), pngChunk('IHDR', header), pngChunk('IDAT', deflateSync(rows)), pngChunk('IEND', Buffer.alloc(0))]);
}
const sourceSelector = '.source-host .cm-content';
async function source() {
  return page.locator(sourceSelector).evaluate(element => { const view = element.cmTile?.root?.view; if (view?.contentDOM !== element) throw new Error('Actual source editor unavailable'); return view.state.doc.toString(); });
}
async function waitSource(expected) {
  await page.waitForFunction(({ selector, expected }) => { const element = document.querySelector(selector), view = element?.cmTile?.root?.view; return view?.contentDOM === element && view.state.doc.toString() === expected; }, { selector: sourceSelector, expected });
  assert.equal(await source(), expected);
}
async function snapshot(name, text) { await writeFile(path.join(out, `${name}.typ`), text, { flag: 'wx' }); return { file: `${name}.typ`, sha256: sha(text) }; }
function patchResult(before, wire) {
  assert.equal(wire.patchUnit, 'utf16'); assert.equal(wire.sourceHash, sha(before)); assert.equal(wire.patches.length, 1);
  const patch = wire.patches[0]; assert.equal(patch.expected, before.slice(patch.from, patch.to));
  assert.equal(patch.byteFrom, Buffer.byteLength(before.slice(0, patch.from))); assert.equal(patch.byteTo, Buffer.byteLength(before.slice(0, patch.to)));
  assert.equal(wire.source, before.slice(0, patch.from) + patch.text + before.slice(patch.to)); return wire.source;
}
function calls(route) { return result.requests.filter(item => item.path === route); }
async function undo(expected) { await page.locator(sourceSelector).press('Control+z'); await waitSource(expected); }
async function redo(expected) { await page.locator(sourceSelector).press('Control+Shift+Z'); await waitSource(expected); }
async function listValues(key, expected) {
  await page.waitForFunction(({ key, expected }) => {
    const field = document.querySelector(`.data-property[data-field-key="${key}"]`);
    const actual = field && [...field.querySelectorAll('.native-list-item textarea')].map(input => input.value);
    return actual && JSON.stringify(actual) === JSON.stringify(expected);
  }, { key, expected });
}
async function waitTransferReady() {
  await page.waitForFunction(() => {
    const panel = document.querySelector('.data-panel'), transfer = panel?.querySelector('.data-field-transfer');
    const apply = transfer?.querySelector('[data-transfer-control="apply"]');
    return panel && panel.getAttribute('aria-busy') !== 'true' && transfer?.getAttribute('aria-busy') === 'false' && apply && !apply.disabled;
  });
}
async function readResources(names, png, textBytes) {
  const directory = path.dirname(doc.path), files = [await readFile(path.join(directory, names[0])), await readFile(path.join(directory, names[1]))];
  assert.deepEqual(files[0], png); assert.deepEqual(files[1], textBytes);
  const manifestBytes = await readFile(path.join(directory, 'weftext.resources.json')), manifest = JSON.parse(manifestBytes);
  assert.equal(manifest.version, 1); assert.ok(Array.isArray(manifest.resources));
  for (const name of names) assert.equal(manifest.resources.filter(item => item.path === name).length, 1);
  return { pngSha256: sha(files[0]), textSha256: sha(files[1]), manifestSha256: sha(manifestBytes), manifest, directory };
}

try {
  await start(); await api('/api/library/create', { path: root, operationId: randomUUID() });
  const baseSchema = await api('/api/data/schema', { root }), schema = JSON.parse(baseSchema.source);
  const list = (id, key) => ({ id, key, shape: { kind: 'list', items: { kind: 'scalar', semantic: 'text' }, min: 0, max: null }, nullable: false, default: { kind: 'absent' } });
  schema.fields.push(list('batch/left_values', 'left_values'), list('batch/right_values', 'right_values'), { id: 'batch/rating', key: 'rating', shape: { kind: 'scalar', semantic: 'int' }, nullable: false, default: { kind: 'absent' } });
  schema.types.push({ id: 'batch/type', fields: ['batch/left_values', 'batch/right_values', 'batch/rating'].map(id => ({ id, required: false })), requires: [], conflicts: [], constraints: [] });
  schema.aliases.batchtype = 'batch/type';
  const schemaSource = JSON.stringify(schema, null, 2), schemaOperation = randomUUID();
  const schemaPlan = await api('/api/data/schema/prepare', { root, source: schemaSource, beforeSource: baseSchema.source, beforeRevision: baseSchema.revision, operationId: schemaOperation });
  const schemaReceipt = await api('/api/data/schema/install', { operationId: schemaPlan.operationId, planId: schemaPlan.planId });
  assert.equal(schemaReceipt.status, 'committed'); assert.equal(schemaReceipt.operationId, schemaOperation);
  result.schema = { operationId: schemaOperation, planId: schemaPlan.planId, sourceSha256: sha(schemaSource), receipt: schemaReceipt };
  const creationId = randomUUID(); doc = createdDocument(await api('/api/note/create', { root, parent: root, name: 'Property Resource Batches', operationId: creationId }), creationId);
  assert.ok(doc.source.includes('types: ()'));
  const unknown = 'unknown_fact: (opaque: "保留原文😀",)', initial = '// outside-before 😀\n' + doc.source.replace('types: ()', `types: ("batchtype",), left_values: ("same", "same", "tail",), right_values: ("dest",), rating: 7, ${unknown}`) + '\n正文原件中文😀。\n';
  result.sources = { initial: await snapshot('source-initial', initial) };
  await api('/api/save', { sessionId: doc.sessionId, baseId: doc.baseId, source: initial, operationId: randomUUID() }); await stop();
  initialBytes = await readFile(doc.path); assert.deepEqual(initialBytes, Buffer.from(initial));
  result.fixture = { root, path: doc.path, creationId, initialSha256: sha(initialBytes) };
  await start(['--open', doc.path, '--root', root]);
  browser = await chromium.launch({ channel: 'chrome', headless: true }); result.browser = browser.version();
  page = await browser.newPage({ viewport: { width: 1500, height: 1100 } }); page.setDefaultTimeout(30000);
  page.on('pageerror', error => result.pageErrors.push(String(error.stack ?? error)));
  page.on('request', request => {
    const route = new URL(request.url()).pathname;
    if (request.method() !== 'POST' || !traced.has(route)) return;
    const item = { path: route, request: request.postDataJSON() }; requests.set(request, item); result.requests.push(item);
  });
  page.on('response', response => {
    const item = requests.get(response.request()); if (!item) return;
    const task = (async () => { item.httpStatus = response.status(); item.response = await response.json(); })();
    void task.catch(() => {}); responseTasks.push(task);
  });
  await page.goto(base + '/#token=' + token); await workspaceMode(page, '源码'); await waitSource(initial);
  await workspaceCommand(page, 'knowledge', '属性');
  const panel = page.locator('.data-panel'), field = key => panel.locator(`.data-property[data-field-key="${key}"]`), transfer = panel.locator('.data-field-transfer');
  await field('left_values').waitFor({ state: 'attached' });
  const advanced = panel.locator('.data-field-group.advanced');
  if (await advanced.count() && !await advanced.evaluate(element => element.open)) await advanced.locator('summary').click();
  await listValues('left_values', ['same', 'same', 'tail']); await listValues('right_values', ['dest']);
  await field('left_values').locator('[data-field-preset="batch/left_values"]').click();
  const dialog = page.locator('dialog.data-value-preset-dialog'); await dialog.waitFor();
  await dialog.getByLabel('预设名称', { exact: true }).fill('独立追加');
  await dialog.locator('.native-value-editor').first().getByRole('textbox').first().fill('preset');
  await dialog.getByRole('button', { name: '命名并保存预设', exact: true }).click();
  const presetSelect = dialog.getByLabel('已保存的预设', { exact: true });
  await presetSelect.locator('option').filter({ hasText: '独立追加' }).waitFor({ state: 'attached' });
  await page.waitForFunction(select => select instanceof HTMLSelectElement && select.isConnected && select.value !== ''
    && select.selectedOptions[0]?.textContent === '独立追加' && select.closest('details')?.getAttribute('aria-busy') === 'false', await presetSelect.elementHandle());
  await dialog.getByRole('button', { name: '追加预设项', exact: true }).click();
  assert.equal(calls('/api/data/edit').length, 0); assert.equal(await source(), initial);
  await dialog.getByRole('button', { name: '关闭', exact: true }).click(); await dialog.waitFor({ state: 'detached' });
  await listValues('left_values', ['same', 'same', 'tail', 'preset']); assert.equal(calls('/api/data/edit').length, 0);
  const [editResponse] = await Promise.all([page.waitForResponse(response => new URL(response.url()).pathname === '/api/data/edit' && response.request().method() === 'POST'), field('left_values').getByRole('button', { name: '应用到草稿', exact: true }).click()]);
  assert.equal(editResponse.status(), 200); const editRequest = editResponse.request().postDataJSON(), editWire = await editResponse.json();
  assert.equal(editRequest.source, initial); assert.equal(editRequest.key, 'left_values'); assert.deepEqual(editRequest.path, []);
  assert.deepEqual(editRequest.value, { kind: 'array', value: ['same', 'same', 'tail', 'preset'].map(value => ({ kind: 'text', value })) });
  const afterPreset = patchResult(initial, editWire); assert.ok(afterPreset.includes(unknown)); await waitSource(afterPreset);
  await waitTransferReady();
  assert.equal(calls('/api/data/edit').length, 1); result.sources.preset = await snapshot('source-preset', afterPreset);
  await undo(initial); await listValues('left_values', ['same', 'same', 'tail']);
  await redo(afterPreset); await listValues('left_values', ['same', 'same', 'tail', 'preset']);
  await waitTransferReady();
  assert.equal(calls('/api/data/edit').length, 1); result.cases.push('Personal preset stays staged until one Apply; one source Undo and Redo preserve the author source.');

  await transfer.locator('[data-transfer-control="fromField"]').selectOption('batch/left_values');
  await transfer.locator('[data-transfer-control="toField"]').selectOption('batch/right_values');
  await transfer.locator('[data-transfer-control="first"]').fill('2'); await transfer.locator('[data-transfer-control="last"]').fill('3'); await transfer.locator('[data-transfer-control="position"]').fill('2');
  const [transferResponse] = await Promise.all([page.waitForResponse(response => new URL(response.url()).pathname === '/api/data/transfer' && response.request().method() === 'POST'), transfer.locator('[data-transfer-control="apply"]').click()]);
  assert.equal(transferResponse.status(), 200); const transferRequest = transferResponse.request().postDataJSON(), transferWire = await transferResponse.json();
  const selection = { fromField: 'batch/left_values', toField: 'batch/right_values', start: 1, end: 2, to: 1 };
  for (const [key, value] of Object.entries(selection)) assert.equal(transferRequest[key], value);
  assert.deepEqual(transferWire.transfer, selection); assert.equal(transferRequest.source, afterPreset); assert.equal(transferRequest.sourceHash, sha(afterPreset));
  const afterTransfer = patchResult(afterPreset, transferWire); assert.ok(afterTransfer.includes(unknown)); await waitSource(afterTransfer);
  await listValues('left_values', ['same', 'preset']); await listValues('right_values', ['dest', 'same', 'tail']);
  await waitTransferReady();
  result.sources.transfer = await snapshot('source-transfer', afterTransfer); assert.equal(calls('/api/data/transfer').length, 1);
  await undo(afterPreset); await listValues('left_values', ['same', 'same', 'tail', 'preset']); await listValues('right_values', ['dest']);
  await redo(afterTransfer); await listValues('left_values', ['same', 'preset']); await listValues('right_values', ['dest', 'same', 'tail']);
  await waitTransferReady();
  assert.equal(calls('/api/data/transfer').length, 1); result.cases.push('One exact ordinal transfer moves duplicate occurrences in one patch; one Undo restores both fields and Redo restores the transfer.');

  const png = fixturePng(), textBytes = Buffer.from('Synthetic resource 中文 1029\n');
  const uploads = [path.join(out, 'picked-image.png'), path.join(out, 'picked-text.txt')];
  await writeFile(uploads[0], png, { flag: 'wx' }); await writeFile(uploads[1], textBytes, { flag: 'wx' });
  const insertTab = await workspaceTab(page, 'insert'); assert.equal(await insertTab.locator('input.resource-batch-picker[multiple]').count(), 1);
  const [chooser] = await Promise.all([page.waitForEvent('filechooser'), insertTab.getByRole('button', { name: '导入资源', exact: true }).click()]);
  await chooser.setFiles(uploads);
  const resource = page.locator('dialog.resource-import-batch'); await resource.waitFor();
  const control = name => resource.locator(`[data-resource-import-control="${name}"]`), names = ['owned-renamed.png', 'owned-renamed.txt'];
  await resource.locator('[data-resource-import-name="0"]').fill(names[0]); await resource.locator('[data-resource-import-name="1"]').fill(names[1]);
  await control('mode').selectOption('autoAppend'); assert.equal(await source(), afterTransfer);
  result.resourceLayouts = [];
  for (const width of [1200, 390]) {
    await page.setViewportSize({ width, height: 900 });
    const layout = await resource.evaluate(element => ({ viewport: innerWidth, documentWidth: document.documentElement.scrollWidth, dialogWidth: element.clientWidth, dialogScrollWidth: element.scrollWidth }));
    const screenshot = `resource-dialog-${width}.png`; await page.screenshot({ path: path.join(out, screenshot), fullPage: true });
    result.resourceLayouts.push({ ...layout, screenshot });
    assert.ok(layout.documentWidth <= layout.viewport, `Document overflows at ${width}px: ${JSON.stringify(layout)}`);
    assert.ok(layout.dialogScrollWidth <= layout.dialogWidth, `Resource dialog overflows at ${width}px: ${JSON.stringify(layout)}`);
  }
  await page.setViewportSize({ width: 1500, height: 1100 });
  const [preparedResponse] = await Promise.all([page.waitForResponse(response => new URL(response.url()).pathname === '/api/resources/prepare-import-batch' && response.request().method() === 'POST'), control('prepare').click()]);
  assert.equal(preparedResponse.status(), 200); const preparedRequest = preparedResponse.request().postDataJSON(), prepared = await preparedResponse.json();
  assert.deepEqual(preparedRequest.files.map(file => file.name), names); assert.deepEqual(preparedRequest.files.map(file => Buffer.from(file.bytes, 'base64')), [png, textBytes]);
  assert.equal(prepared.operation.operationId, preparedRequest.operationId); assert.equal(prepared.operation.status, 'prepared'); assert.equal(prepared.operation.kind, 'importResources');
  assert.equal(prepared.imports.length, 2); assert.deepEqual(prepared.imports.map(item => item.name), names); assert.equal(prepared.operation.changes.length, 3);
  const expectedPaths = [...names, 'weftext.resources.json'].map(name => path.resolve(path.dirname(doc.path), name).toLowerCase()).sort();
  assert.deepEqual(prepared.operation.changes.map(change => path.resolve(change.path).toLowerCase()).sort(), expectedPaths);
  await control('confirm').waitFor(); assert.equal(await resource.locator('[data-resource-import-changes] li').count(), 3); assert.equal(await source(), afterTransfer);
  const [commitResponse, insertResponse] = await Promise.all([
    page.waitForResponse(response => new URL(response.url()).pathname === '/api/operations/commit' && response.request().method() === 'POST'),
    page.waitForResponse(response => new URL(response.url()).pathname === '/api/resources/import-batch/insert' && response.request().method() === 'POST'),
    control('confirm').click(),
  ]);
  assert.equal(commitResponse.status(), 200); const commitRequest = commitResponse.request().postDataJSON(), commit = await commitResponse.json();
  assert.deepEqual(commitRequest, { operationId: preparedRequest.operationId, planId: prepared.operation.planId }); assert.equal(commit.status, 'committed'); assert.equal(commit.operationId, preparedRequest.operationId);
  assert.equal(insertResponse.status(), 200); const insertRequest = insertResponse.request().postDataJSON(), insertion = await insertResponse.json();
  assert.equal(insertRequest.operationId, preparedRequest.operationId); assert.equal(insertRequest.mode, 'auto'); assert.equal(insertRequest.source, afterTransfer); assert.equal(insertRequest.sourceHash, sha(afterTransfer));
  assert.equal(insertion.operationId, preparedRequest.operationId); assert.deepEqual(insertion.imports, prepared.imports);
  assert.equal(insertion.edit.serial, insertRequest.serial);
  assert.equal(insertion.edit.patches.length, 1); const patch = insertion.edit.patches[0];
  assert.equal(patch.utf16From, patch.utf16To); assert.equal(patch.expected, '');
  assert.equal(patch.utf16From, afterTransfer.length); assert.equal(patch.from, Buffer.byteLength(afterTransfer));
  assert.equal(insertion.edit.source, afterTransfer.slice(0, patch.utf16From) + patch.insert + afterTransfer.slice(patch.utf16To));
  assert.equal(insertion.edit.sourceHash, sha(insertion.edit.source)); assert.ok(patch.insert.includes(names[0])); assert.ok(patch.insert.includes(names[1]));
  assert.ok(patch.insert.includes(insertion.imports[1].resourceRef)); assert.ok(insertion.edit.source.includes(unknown));
  const afterInsert = insertion.edit.source; await waitSource(afterInsert); result.sources.insert = await snapshot('source-insert', afterInsert);
  await control('finish').waitFor(); assert.ok(await control('finish').isEnabled());
  const beforeUndoFiles = await readResources(names, png, textBytes);
  for (let i = 0; i < names.length; i++) {
    const resourceId = insertion.imports[i].resourceRef.split('/').at(-1);
    assert.equal(beforeUndoFiles.manifest.resources.find(item => item.path === names[i]).resourceId, resourceId);
  }
  await control('finish').click(); await resource.waitFor({ state: 'hidden' });
  await Promise.all(responseTasks);
  const initialDraft = calls('/api/draft').filter(item => item.request.source === afterInsert && item.httpStatus === 200 && item.response?.evidence === 'authorDraft').at(-1);
  assert.ok(initialDraft, 'The original insertion must acknowledge the unsaved author Draft');
  await undo(afterTransfer); const afterUndoFiles = await readResources(names, png, textBytes); assert.deepEqual(afterUndoFiles, beforeUndoFiles);
  const finalDraftPromise = page.waitForResponse(response => {
    if (new URL(response.url()).pathname !== '/api/draft' || response.request().method() !== 'POST') return false;
    const body = response.request().postDataJSON(); return body.source === afterInsert && body.inputRevision > initialDraft.request.inputRevision;
  });
  await redo(afterInsert); const finalDraftResponse = await finalDraftPromise, finalDraftRequest = finalDraftResponse.request().postDataJSON(), finalDraft = await finalDraftResponse.json();
  assert.equal(finalDraftResponse.status(), 200); assert.equal(finalDraft.evidence, 'authorDraft'); assert.equal(finalDraft.inputRevision, finalDraftRequest.inputRevision);
  assert.equal(finalDraftRequest.sessionId, insertRequest.sessionId);
  const afterRedoFiles = await readResources(names, png, textBytes); assert.deepEqual(afterRedoFiles, beforeUndoFiles);
  assert.equal(calls('/api/resources/prepare-import-batch').length, 1); assert.equal(calls('/api/operations/commit').length, 1); assert.equal(calls('/api/resources/import-batch/insert').length, 1);
  await Promise.all(responseTasks);
  const commitIndex = result.requests.findIndex(item => item.path === '/api/operations/commit' && item.request.operationId === preparedRequest.operationId);
  const insertIndex = result.requests.findIndex(item => item.path === '/api/resources/import-batch/insert' && item.request.operationId === preparedRequest.operationId);
  const qualifiedMaps = result.requests.filter((item, index) => index > commitIndex && index < insertIndex && item.path === '/api/editor-map' && item.request.source === afterTransfer && item.request.serial === insertRequest.serial);
  assert.ok(qualifiedMaps.length >= 1, 'An observed map must qualify the exact insertion input');
  assert.ok(qualifiedMaps.some(item => item.httpStatus === 200 && item.response?.sourceHash === sha(afterTransfer) && item.response.syntaxValid && item.response.bodyInsertion?.writable && item.response.bodyInsertion.utf16From === patch.utf16From && item.response.bodyInsertion.utf16To === patch.utf16To));
  result.resource = { operationId: preparedRequest.operationId, planId: prepared.operation.planId, imports: insertion.imports, files: beforeUndoFiles, patch, qualifiedMapRequests: qualifiedMaps.length,
    finalDraft: { sourceSha256: sha(finalDraftRequest.source), inputId: finalDraftRequest.inputId, inputRevision: finalDraftRequest.inputRevision, evidence: finalDraft.evidence } };
  result.cases.push('One real picker batch is prepared and committed once, then one source insertion Undo/Redo retains the imported resource files and manifest.');

  await Promise.all(responseTasks); assert.equal(calls('/api/save').length, 0); assert.equal(calls('/api/save-as').length, 0); assert.deepEqual(result.pageErrors, []);
  await browser.close(); browser = undefined; await stop();
  assert.deepEqual(await readFile(doc.path), initialBytes, 'The source Draft was never saved over the author file');
  assert.deepEqual(await readResources(names, png, textBytes), beforeUndoFiles);
  for (const [file, digest] of Object.entries(hashes)) assert.equal(sha(await readFile(path.join(repo, file))), digest, `Input changed: ${file}`);
  assert.equal(sha(await readFile(exe)), executableSha256); assert.equal(sha(await readFile(catalogPath)), sha(catalogBytes)); assert.equal(sha(await readFile(receiptPath)), sha(receiptBytes));
  result.authorFileAfterSha256 = sha(await readFile(doc.path)); result.completed = true;
} catch (error) {
  result.failure = String(error.stack ?? error); process.exitCode = 1;
  if (page) {
    await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {});
    await writeFile(path.join(out, 'failure-ui.txt'), await page.locator('body').innerText().catch(() => 'unavailable'), { flag: 'wx' }).catch(() => {});
  }
}
finally {
  if (browser) { try { await browser.close(); } catch (error) { result.browserCleanupError = String(error); result.completed = false; process.exitCode = 1; } }
  if (child) { try { await stop(); } catch (error) { result.ownerCleanupError = String(error); result.completed = false; process.exitCode = 1; } }
  try { await Promise.all(responseTasks); } catch (error) { result.responseCollectionError = String(error); result.completed = false; process.exitCode = 1; }
  result.stderr = stderr; result.finishedUtc = new Date().toISOString();
  await writeFile(path.join(out, 'result.json'), stringifyServeJson(result, 2), { flag: 'wx' });
}
console.log(JSON.stringify({ completed: result.completed, cases: result.cases.length, out }));
