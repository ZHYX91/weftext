// Owned end-to-end check against the selected CLI's embedded workspace and real Core.
// No Vite/mock server, desktop input, real IME or assistive-technology claim.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { workspaceCommand, workspaceMode } from './_workspace_browser.mjs';
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
// A package build owns a separate frozen source root. Bind its selected source
// entries below; do not relabel that build as one from the harness checkout.
assert.ok(path.isAbsolute(receipt.sourceRoot));
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
const productionCommit = execFileSync('git', ['rev-parse', `${process.env.WEFTEXT_TEST_SOURCE_COMMIT || 'HEAD'}^{commit}`], { cwd: repo, encoding: 'utf8', windowsHide: true }).trim();
const inputs = [
  'app/src/birthday.ts', 'app/src/native-value-form.ts', 'app/src/author-input.ts', 'app/src/personal-control-input.ts',
  'app/src/capture-controller.ts', 'app/src/builtin-property-labels.ts', 'app/src/data-panel.ts', 'app/src/data-view.css',
  'app/src/main.ts', 'app/src/ui-messages.ts', 'assets/ui-messages.json', 'app/src/editor.ts', 'app/src/session.ts',
  'app/src/original-operation.ts', 'crates/weftext-app/src/data_api.rs', 'docs/design/examples/schema.json',
];
const localInputs = ['scripts/test_birthday_persistence_browser.mjs', 'scripts/_workspace_browser.mjs', 'scripts/_cli_serve.mjs', 'scripts/_creation_result.mjs'];
const hashes = {}, receiptSourceBindings = {};
for (const file of localInputs) hashes[file] = sha(await readFile(path.join(repo, file)));
for (const file of inputs) {
  const entries = receipt.source.filter(entry => entry.path === file);
  assert.equal(entries.length, 1, `Build receipt must identify ${file}`);
  const committed = execFileSync('git', ['show', `${productionCommit}:${file}`], { cwd: repo, maxBuffer: 8 * 1024 * 1024 });
  hashes[file] = sha(committed);
  // The receipt records physical Windows bytes, while Git text blobs use LF.
  // Accept only the exact fixed blob or its uniform CRLF checkout bytes; current
  // production worktree changes cannot qualify an older embedded executable.
  const variants = [committed, Buffer.from(committed.toString('utf8').replace(/\r?\n/g, '\r\n'))];
  const matched = variants.findIndex(bytes => bytes.length === entries[0].bytes && sha(bytes) === entries[0].sha256);
  assert.ok(matched >= 0, `Receipt differs from frozen ${productionCommit}:${file}, including supported checkout line endings`);
  receiptSourceBindings[file] = { gitBlobSha256: hashes[file], receiptSha256: entries[0].sha256, checkoutBytes: matched === 0 ? 'fixed Git blob' : 'fixed Git blob with CRLF checkout newlines' };
}
const result = {
  schemaVersion: 1, startedUtc: new Date().toISOString(), exe, executableSha256,
  artifacts: { batch, buildId: catalog.buildId, batchId: receipt.batchId, receiptSourceRoot: receipt.sourceRoot, catalogSha256: sha(catalogBytes), receiptSha256: sha(receiptBytes) },
  productionCommit, receiptSourceBindings, sourceInputs: hashes, knownIssues: productionCommit.startsWith("7423588") ? ["The fixed 7423588 UI focus helper can focus Remove instead of date input; this test does not activate that shortcut.", "The fixed 7423588 birthday summary does not distinguish leap_month; this fixture uses Gregorian year/month records, not leap-month claims."] : [], frontend: 'selected CLI embedded workspace', cases: [], requests: [], pageErrors: [], services: [], completed: false,
  limits: ['Owned synthetic library and headless Chrome only; no native desktop, real OS IME or assistive-technology acceptance.', 'Only an owned synthetic library is used. Save response loss is deliberately injected after a real Core commit; this is not an internal IO fault or physical crash.'],
};
await writeFile(path.join(out, 'inputs.json'), stringifyServeJson(result, 2), { flag: 'wx' });
const { chromium } = createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const root = path.join(out, 'library'), data = path.join(out, 'state');
let child, browser, page, base, token, stderr = '', doc, initialBytes;
const ownerServices = new WeakMap(), requests = new Map(), responseTasks = [];
const traced = new Set(['/api/data/edit', '/api/data/project', '/api/operations/status', '/api/draft', '/api/save', '/api/save-as']);

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
const sourceSelector = '.source-host .cm-content';
async function source() {
  return page.locator(sourceSelector).evaluate(element => {
    const view = element.cmTile?.root?.view;
    if (view?.contentDOM !== element) throw new Error('Actual source editor unavailable');
    return view.state.doc.toString();
  });
}
async function waitSource(expected) {
  await page.waitForFunction(({ selector, expected }) => {
    const element = document.querySelector(selector), view = element?.cmTile?.root?.view;
    return view?.contentDOM === element && view.state.doc.toString() === expected;
  }, { selector: sourceSelector, expected });
  assert.equal(await source(), expected);
}
async function snapshot(name, text) {
  await writeFile(path.join(out, `${name}.typ`), text, { flag: 'wx' });
  return { file: `${name}.typ`, sha256: sha(text) };
}
function calls(route) { return result.requests.filter(item => item.path === route); }
const text = value => ({ kind: 'text', value }), bool = value => ({ kind: 'bool', value }), int = value => ({ kind: 'int', value: String(value) });
const record = value => ({ kind: 'record', value: Object.entries(value).map(([key, value]) => ({ key, value })) });
const list = (...value) => ({ kind: 'array', value });
const member = (value, key) => value.value.find(item => item.key === key)?.value;
const calendar = (year, month) => record({ calendar: text('gregorian'), version: text('proleptic-gregorian.v1'), precision: text(month ? 'month' : 'year'), year: int(year), ...(month ? { month: int(month) } : {}) });
const evidence = name => list(record({ reference: record({ url: text('https://example.invalid/birthday/' + name) }), locator: text('Synthetic page ' + name), note: text('Retain source note ' + name) }));
const initialValue = list(
  record({ kind: text('birth'), event_time: calendar(2001), selected: bool(true), note: text('Birth A retained 😀'), sources: evidence('a'), unknownEvidence: record({ code: text('retain-a') }) }),
  record({ kind: text('death'), event_time: calendar(2090, 9), selected: bool(true), note: text('Independent selected death'), sources: evidence('death') }),
  record({ kind: text('birth'), event_time: calendar(2004, 7), selected: bool(true), note: text('Birth B retained'), sources: evidence('b'), unknownEvidence: record({ code: text('retain-b') }) }),
);
const expectedValue = structuredClone(initialValue);
member(expectedValue.value[0], 'selected').value = false;
function nativeSource(value) {
  if (value.kind === 'record') return '(' + value.value.map(item => `${item.key}: ${nativeSource(item.value)}`).join(', ') + ',)';
  if (value.kind === 'array') return '(' + value.value.map(nativeSource).join(', ') + (value.value.length ? ',' : '') + ')';
  if (value.kind === 'text') return JSON.stringify(value.value);
  if (value.kind === 'int' || value.kind === 'bool') return String(value.value);
  throw new Error('Unsupported synthetic fixture value ' + value.kind);
}
async function openWorkspace(expected) {
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
  await page.goto(base + '/#token=' + token); await workspaceMode(page, '源码'); await waitSource(expected);
  await workspaceCommand(page, 'knowledge', '属性');
  const field = page.locator('.data-panel .data-property[data-field-id="people/life_events"]');
  await field.waitFor({ state: 'attached' });
  const advanced = page.locator('.data-panel .data-field-group.advanced');
  if (await advanced.count() && !await advanced.evaluate(element => element.open)) await advanced.locator(':scope > summary').click();
  await field.locator('.native-birthday').waitFor({ state: 'visible' });
  return field;
}
async function projectSaved(pathname) {
  const opened = await api('/api/open', { path: pathname, root });
  const projected = await api('/api/data/project', { sessionId: opened.sessionId, source: opened.source, serial: 1 });
  assert.equal(projected.projection.qualification, 'qualified');
  const field = projected.projection.fields.find(field => field.fieldId === 'people/life_events');
  assert.equal(field?.cell.state, 'value', JSON.stringify(projected));
  return { source: opened.source, value: field.cell.value, projection: projected };
}

try {
  await start();
  const libraryOperationId = randomUUID(); const libraryReceipt = await api('/api/library/create', { path: root, operationId: libraryOperationId });
  const schema = await api('/api/data/schema', { root });
  await writeFile(path.join(out, 'schema-builtin.json'), schema.source, { flag: 'wx' });
  assert.equal(JSON.parse(schema.source).fields.find(field => field.id === 'people/life_events').key, 'life_events');
  const creationId = randomUUID(); doc = createdDocument(await api('/api/note/create', { root, parent: root, name: 'Birthday persistence', operationId: creationId }), creationId);
  assert.ok(doc.source.includes('types: ()'));
  const unknown = 'unknown_fact: (opaque: "保留原文😀",)', initial = '// retained outside metadata 😀\n' + doc.source.replace('types: ()', `types: ("person",), life_events: ${nativeSource(initialValue)}, ${unknown}`) + '\nOriginal body retained 中文😀。\n';
  result.sources = { initial: await snapshot('source-initial', initial) };
  const setupOperationId = randomUUID(), setupReceipt = await api('/api/save', { sessionId: doc.sessionId, baseId: doc.baseId, source: initial, operationId: setupOperationId });
  assert.ok(['saved', 'committed'].includes(setupReceipt.status));
  assert.deepEqual((await projectSaved(doc.path)).value, initialValue);
  result.fixture = { root, path: doc.path, state: data, creationId, libraryOperationId, libraryReceipt, setupOperationId, setupReceipt, schemaSha256: sha(schema.source), initialValue, expectedValue };
  await stop(); initialBytes = await readFile(doc.path); assert.deepEqual(initialBytes, Buffer.from(initial));
  result.cases.push('Real Core creates and saves the retained builtin-schema synthetic person with two selected birth records and an independently selected death.');

  await start(['--open', doc.path, '--root', root]);
  browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE || 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true }); result.browser = browser.version();
  const field = await openWorkspace(initial), panel = field.locator('.native-birthday');
  assert.match(await panel.innerText(), /多个选中的出生事件/);
  assert.equal(await panel.locator('[data-birth-index]').count(), 2);
  assert.match(await panel.locator('[data-birth-index="0"]').innerText(), /年精度/);
  assert.match(await panel.locator('[data-birth-index="2"]').innerText(), /月精度/);
  await panel.locator('[data-birth-index="2"]').getByRole('button', { name: '选为生日来源', exact: true }).click();
  assert.equal(calls('/api/data/edit').length, 0); assert.equal(calls('/api/save').length, 0);
  assert.equal(await source(), initial); assert.deepEqual(await readFile(doc.path), initialBytes);
  const [editResponse] = await Promise.all([
    page.waitForResponse(response => new URL(response.url()).pathname === '/api/data/edit' && response.request().method() === 'POST'),
    field.getByRole('button', { name: '应用到草稿', exact: true }).click(),
  ]);
  assert.equal(editResponse.status(), 200); const editRequest = editResponse.request().postDataJSON(), editWire = await editResponse.json();
  assert.equal(editWire.patchUnit, 'utf16'); assert.equal(editWire.sourceHash, sha(initial)); assert.equal(editWire.patches.length, 1);
  const patch = editWire.patches[0];
  assert.equal(editRequest.key, 'life_events'); assert.deepEqual(editRequest.path, []); assert.deepEqual(editRequest.value, expectedValue);
  assert.equal(typeof patch.from, 'number'); assert.equal(typeof patch.to, 'number'); assert.equal(typeof patch.text, 'string');
  assert.equal(patch.expected, initial.slice(patch.from, patch.to));
  assert.equal(patch.byteFrom, Buffer.byteLength(initial.slice(0, patch.from))); assert.equal(patch.byteTo, Buffer.byteLength(initial.slice(0, patch.to)));
  const applied = initial.slice(0, patch.from) + patch.text + initial.slice(patch.to); assert.equal(applied, editWire.source);
  assert.ok(applied.includes(unknown)); assert.ok(applied.endsWith('Original body retained 中文😀。\n'));
  await waitSource(applied); assert.deepEqual(await readFile(doc.path), initialBytes, 'Apply changes only author Draft until Save');
  await page.locator(sourceSelector).press('Control+z'); await waitSource(initial);
  await page.locator(sourceSelector).press('Control+Shift+Z'); await waitSource(applied);
  assert.equal(calls('/api/data/edit').length, 1, 'Undo/Redo uses the shared source history, not another server field write');
  result.sources.applied = await snapshot('source-applied', applied);
  result.cases.push('Actual embedded DataPanel explicitly selects birth B and one Core-generated field patch preserves event order, independent selected death, notes, sources, unknown values and exact year/month precision; shared-source Undo/Redo and unapplied file bytes verified.');

  let droppedRequest, committedResponse, deliveryError;
  let resolveLost, rejectLost; const lost = new Promise((resolve, reject) => { resolveLost = resolve; rejectLost = reject; }); lost.catch(() => {});
  await page.route('**/api/save', async route => {
    try {
      assert.equal(droppedRequest, undefined, 'Exactly one author save is delivered');
      droppedRequest = route.request().postDataJSON();
      const response = await route.fetch(); assert.equal(response.status(), 200); committedResponse = await response.json();
      assert.equal(droppedRequest.source, applied); assert.ok(['saved', 'committed'].includes(committedResponse.status));
      assert.equal(committedResponse.operationId, droppedRequest.operationId);
      await route.abort('failed'); resolveLost();
    } catch (error) { deliveryError = String(error.stack ?? error); rejectLost(error); await route.abort('failed').catch(() => {}); }
  });
  const saveButton = page.locator('.top-actions [data-command="save"]');
  const recoveryResponsePromise = page.waitForResponse(response => new URL(response.url()).pathname === '/api/operations/status' && droppedRequest && response.request().postDataJSON().operationId === droppedRequest.operationId);
  const requestFailed = page.waitForEvent('requestfailed', request => new URL(request.url()).pathname === '/api/save');
  // Attach every rejection handler before the click. A pre-save failure must
  // reach catch/finally even when the save route is never entered.
  let deadline;
  const timeout = new Promise((_, reject) => { deadline = setTimeout(() => reject(new Error('Save response-loss/recovery did not complete within 30 seconds')), 30000); });
  let recoveryResponse;
  try {
    [recoveryResponse] = await Promise.race([
      Promise.all([recoveryResponsePromise, requestFailed, lost, saveButton.click()]),
      timeout,
    ]);
  } finally { clearTimeout(deadline); }
  await page.waitForFunction(() => { const button = document.querySelector('.top-actions [data-command="save"]'); return button && !button.disabled; });
  assert.equal(deliveryError, undefined);
  assert.deepEqual(await readFile(doc.path), Buffer.from(applied), 'Original real Core save committed before its response was deliberately lost');
  await waitSource(applied); await page.unroute('**/api/save');
  const recoveryRequest = recoveryResponse.request().postDataJSON(), recovery = await recoveryResponse.json();
  assert.equal(recoveryResponse.status(), 200); assert.equal(recoveryRequest.operationId, droppedRequest.operationId);
  assert.equal(recovery.operationId, droppedRequest.operationId); assert.ok(['saved', 'committed'].includes(recovery.status));
  await page.waitForFunction(() => { const button = document.querySelector('.top-actions [data-command="save"]'); return button && !button.disabled; });
  await waitSource(applied); assert.equal(calls('/api/save').length, 1); assert.equal(calls('/api/save-as').length, 0);
  assert.deepEqual((await projectSaved(doc.path)).value, expectedValue);
  result.saveRecovery = { mode: 'controlled response loss after a real Core commit', request: droppedRequest, committedResponse, recoveryRequest, recovery };
  result.sources.saved = await snapshot('source-saved', await readFile(doc.path, 'utf8'));
  result.cases.push('One actual author Save commits to disk despite controlled response loss; the existing UI resolves that exact original operation through status without sending another author Save.');
  await panel.screenshot({ path: path.join(out, 'birthday-saved-embedded.png') });
  await browser.close(); browser = undefined; await stop();

  await start(['--open', doc.path, '--root', root]);
  browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE || 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
  const reopened = await openWorkspace(applied), reopenedPanel = reopened.locator('.native-birthday');
  assert.doesNotMatch(await reopenedPanel.innerText(), /多个选中的出生事件/);
  assert.equal(await reopenedPanel.locator('[data-birth-index]').count(), 2);
  assert.equal(await reopenedPanel.locator('[data-birth-index="2"]').getByRole('button', { name: '选为生日来源', exact: true }).count(), 0);
  assert.equal(await reopenedPanel.locator('[data-birth-index="0"]').getByRole('button', { name: '选为生日来源', exact: true }).count(), 1);
  const readback = await projectSaved(doc.path); assert.equal(readback.source, applied); assert.deepEqual(readback.value, expectedValue);
  assert.deepEqual(await readFile(doc.path), Buffer.from(applied));
  await writeFile(path.join(out, 'readback-projection.json'), JSON.stringify(readback.projection, null, 2), { flag: 'wx' });
  await reopenedPanel.screenshot({ path: path.join(out, 'birthday-reopened-embedded.png') });
  assert.notEqual(result.services.at(-1).ownerIdentity.incarnation, result.services.at(-2).ownerIdentity.incarnation);
  result.cases.push('After exact ownerStop, a fresh Core owner and fresh embedded browser reopen the saved author file and reproject identical life_events with only birth B selected and no inferred month/day parts.');
  result.nativeFollowup = { executableRole: 'root-supplied desktop sibling of this qualified CLI', data, root, open: doc.path, schema: path.join(out, 'schema-builtin.json'), savedSource: path.join(out, 'source-saved.typ'), note: 'Retained synthetic fixture only. CLI is stopped before handoff; this result does not claim native WebView2 acceptance.' };
  await Promise.all(responseTasks); assert.deepEqual(result.pageErrors, []);
  await browser.close(); browser = undefined; await stop();
  for (const file of localInputs) assert.equal(sha(await readFile(path.join(repo, file))), hashes[file], `Harness changed: ${file}`);
  assert.equal(sha(await readFile(exe)), executableSha256); assert.equal(sha(await readFile(catalogPath)), sha(catalogBytes)); assert.equal(sha(await readFile(receiptPath)), sha(receiptBytes));
  result.authorFileAfterSha256 = sha(await readFile(doc.path)); result.completed = true;
} catch (error) {
  result.failure = String(error.stack ?? error); process.exitCode = 1;
  if (page) {
    await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {});
    await writeFile(path.join(out, 'failure-ui.txt'), await page.locator('body').innerText().catch(() => 'unavailable'), { flag: 'wx' }).catch(() => {});
  }
} finally {
  if (browser) { try { await browser.close(); } catch (error) { result.browserCleanupError = String(error); result.completed = false; process.exitCode = 1; } }
  if (child) { try { await stop(); } catch (error) { result.ownerCleanupError = String(error); result.completed = false; process.exitCode = 1; } }
  try { await Promise.all(responseTasks); } catch (error) { result.responseCollectionError = String(error); result.completed = false; process.exitCode = 1; }
  result.stderr = stderr; result.finishedUtc = new Date().toISOString();
  await writeFile(path.join(out, 'result.json'), stringifyServeJson(result, 2), { flag: 'wx' });
}
console.log(JSON.stringify({ completed: result.completed, cases: result.cases.length, failure: result.failure, out }));
