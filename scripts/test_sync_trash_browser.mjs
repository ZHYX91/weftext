import { spawnServe, serveReady, stopServe, stringifyServeJson } from './_cli_serve.mjs';
import { createdDocument } from './_creation_result.mjs';
// Real public producers and the local Trash UI, with owned replicas and Chrome.
// Route interception only drops an actual completed response; it never invents
// an object, version, plan, receipt, or successful HTTP payload.
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile, appendFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import assert from 'node:assert/strict';

if (!process.argv[2] || !process.argv[3]) throw new Error('Usage: node test_sync_trash_browser.mjs EXE NEW_OUT');
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]);
await mkdir(out, { recursive: false });
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const result = { exe, sha256: sha(await readFile(exe)), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(), cases: [], pageErrors: [], routeErrors: [], completed: false };
const servers = [], requests = [], responses = [], observed = [];
let browser, page;
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
async function until(read, predicate, label, milliseconds = 30000) {
  const end = Date.now() + milliseconds; let value;
  while (Date.now() < end) { value = await read(); if (predicate(value)) return value; await pause(60); }
  throw new Error(`${label} timeout; last=${JSON.stringify(value)}`);
}
async function start(label, args) {
  const child = spawnServe(exe, [...args], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  const server = { child, label, stdout: '', stderr: '' }; servers.push(server);
  child.stdout.on('data', bytes => server.stdout += bytes); child.stderr.on('data', bytes => server.stderr += bytes);
  await until(() => serveReady(child), Boolean, `${label} startup`);
  const match = serveReady(child);
  return Object.assign(server, { base: match.base, token: match.token });
}
async function stop(server) { await stopServe(server?.child); }
async function api(server, route, request, expected = 200) {
  const response = await fetch(server.base + route, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': server.token }, body: JSON.stringify(request), signal: AbortSignal.timeout(45000) });
  const value = await response.json();
  await appendFile(path.join(out, 'http.ndjson'), JSON.stringify({ program: server.label, route, request, status: response.status, response: value }) + '\n');
  assert.equal(response.status, expected, `${route}: ${JSON.stringify(value)}`); return value;
}
async function operation(server, root, intent, operationId = randomUUID()) {
  const plan = await api(server, '/api/operations/prepare', { operationId, root, intent });
  const receipt = await api(server, '/api/operations/commit', { operationId, planId: plan.planId });
  assert.equal(receipt.status, 'committed'); return { plan, receipt };
}
const normalize = value => path.resolve(value).replaceAll('\\', '/');
async function exists(location) { try { await stat(location); return true; } catch (error) { if (error.code === 'ENOENT') return false; throw error; } }
const sourceText = async editor => (await editor.locator('.cm-line').allTextContents()).join('\n');
async function sourceEditor() { await page.getByRole('button', { name: '源码', exact: true }).click(); const editor = page.locator('.cm-content[contenteditable=true]'); await editor.waitFor(); return editor; }
async function enterDraft(editor, source) {
  const persisted = page.waitForResponse(response => response.url().endsWith('/api/draft') && response.request().postDataJSON()?.source === source && response.ok());
  await editor.press('ControlOrMeta+a'); await page.keyboard.insertText(source); await persisted;
  assert.equal(await sourceText(editor), source);
}
async function trashPanel() {
  const loaded = page.waitForResponse(response => response.url().endsWith('/api/knowledge/trash') && response.ok());
  await page.getByRole('button', { name: '回收站', exact: true }).click();
  const panel = page.getByRole('dialog', { name: '资料库回收站' }); await panel.waitFor();
  const response = await loaded; await panel.locator('article.knowledge-card').first().waitFor();
  return { panel, entries: (await response.json()).trash };
}
function card(panel, entry) { return panel.locator('article.knowledge-card').filter({ has: page.getByText(entry.path, { exact: true }) }); }
async function refresh(panel) {
  const prior = await panel.locator('article.knowledge-card').first().elementHandle();
  const loaded = page.waitForResponse(response => response.url().endsWith('/api/knowledge/trash') && response.ok());
  await panel.getByRole('button', { name: '刷新回收记录', exact: true }).click();
  const entries = (await (await loaded).json()).trash;
  if (prior) await page.waitForFunction(element => !element.isConnected, prior);
  await until(() => panel.locator('article.knowledge-card').count(), count => count === entries.length, 'real Trash refresh'); return entries;
}
async function setDraft(panel, entry, name, parent) {
  await card(panel, entry).getByLabel('恢复名称', { exact: true }).fill(name);
  await card(panel, entry).getByLabel('目标父目录（完整路径）', { exact: true }).fill(parent);
}
async function assertDraft(panel, entry, name, parent) {
  assert.equal(await card(panel, entry).getByLabel('恢复名称', { exact: true }).inputValue(), name);
  assert.equal(await card(panel, entry).getByLabel('目标父目录（完整路径）', { exact: true }).inputValue(), parent);
}
async function tap(route, action, drop = false) {
  let value, failure;
  const handler = async intercepted => {
    try {
      const request = intercepted.request().postDataJSON(), response = await intercepted.fetch({ timeout: 45000 });
      const body = await response.json(); assert.equal(response.status(), 200, JSON.stringify(body));
      const original = { route, request, response: body, dropped: drop };
      if (drop) await intercepted.abort('failed'); else await intercepted.fulfill({ response });
      observed.push(original); value = original;
    } catch (error) { failure = error; result.routeErrors.push(String(error.stack ?? error)); await intercepted.abort('failed').catch(() => {}); }
  };
  await page.route(`**${route}`, handler);
  try { await action(); await until(() => value ?? failure, Boolean, route, 50000); if (failure) throw failure; return value; }
  finally { await page.unroute(`**${route}`, handler); }
}

try {
  const ra = path.join(out, 'sender-library'), rb = path.join(out, 'receiver-library');
  const da = path.join(out, 'sender-private'), db = path.join(out, 'receiver-private');
  const a = await start('sender', ['--data-dir', da]); let b = await start('receiver-setup', ['--data-dir', db]);
  await api(a, '/api/library/create', { path: ra, operationId: randomUUID() });
  const notes = {};
  for (const [name, parent] of [['Parent', ra], ['Child', path.join(ra, 'Parent')], ['Other', ra], ['Live', ra], ['Reorder', ra]]) {
    const document = createdDocument(await api(a, '/api/note/create', { root: ra, parent, name, operationId: randomUUID() }));
    const source = document.source + `\nORIGINAL ${name} 中文😀\n`;
    await api(a, '/api/save', { sessionId: document.sessionId, baseId: document.baseId, source, operationId: randomUUID() });
    notes[name] = { ...document, source, key: { kind: 'node', id: document.nodeId } };
  }
  const child = await api(a, '/api/open', { root: ra, path: notes.Child.path });
  const resource = await api(a, '/api/resources/prepare-import', { sessionId: child.sessionId, name: 'original.bin', bytes: Buffer.from([0, 255, 7]).toString('base64'), operationId: randomUUID() });
  await api(a, '/api/operations/commit', { operationId: resource.operationId, planId: resource.planId });
  const binding = await api(a, '/api/sync/enable', { root: ra, operationId: randomUUID() });
  const identity = binding.identity, objects = binding.installedObjects;
  for (const name of ['Parent', 'Other']) await operation(a, ra, { kind: 'trashNode', source: path.join(ra, name) });
  await api(a, '/api/sync/publish', { root: ra });
  const exchange = path.join(out, 'exchange');
  await api(a, '/api/sync/export', { root: ra, directory: exchange, objects });
  await api(b, '/api/sync/receive', { identity, directory: exchange });
  // This later public Trash operation sorts before Clone and really shifts
  // the existing UI rows; no HTTP response or DOM order is fabricated.
  const cloneId = `80000000-0000-4000-8000-${randomUUID().slice(-12)}`;
  const clone = await operation(b, rb, { kind: 'cloneSync', identity, objects }, cloneId);
  const entries = (await api(b, '/api/knowledge/trash', { root: rb })).trash;
  assert.equal(entries.length, 2); assert.ok(entries.every(entry => entry.operationId === cloneId));
  const parentEntry = entries.find(entry => entry.nodeId === notes.Parent.nodeId), otherEntry = entries.find(entry => entry.nodeId === notes.Other.nodeId);
  assert.ok(parentEntry && otherEntry); assert.ok(!entries.some(entry => entry.nodeId === notes.Child.nodeId));
  assert.equal(await readFile(path.join(parentEntry.path, 'Parent.typ'), 'utf8'), notes.Parent.source);
  assert.equal(await readFile(path.join(parentEntry.path, 'Child', 'Child.typ'), 'utf8'), notes.Child.source);
  assert.deepEqual(await readFile(path.join(parentEntry.path, 'Child', 'original.bin')), Buffer.from([0, 255, 7]));
  const received = await api(b, '/api/sync/status', { identity }), installed = await api(b, '/api/sync/binding', { root: rb });
  for (const [object, revision] of installed.installedVersions) {
    const view = received.objects.find(item => JSON.stringify(item.object) === JSON.stringify(object)).view;
    assert.equal(view.versions.length, 1); assert.equal(view.versions[0].revision, revision);
  }
  const missing = await api(b, '/api/operations/prepare', { root: rb, operationId: randomUUID(), intent: { kind: 'restoreNode', trashOperationId: cloneId, parent: rb, name: 'Never guessed' } }, 409);
  assert.equal(missing.error.code, 'trashSelectionRequired');
  await writeFile(path.join(out, 'fixture.json'), JSON.stringify({ identity, objects, notes, clone, entries, received, installed }, null, 2));
  await stop(b); b = await start('receiver-ui', ['--data-dir', db, '--open', path.join(rb, 'Live', 'Live.typ'), '--root', rb]);
  browser = await chromium.launch({ channel: 'chrome', headless: true }); result.browser = browser.version();
  page = await (await browser.newContext({ viewport: { width: 1500, height: 1050 } })).newPage(); page.setDefaultTimeout(30000);
  page.on('pageerror', error => result.pageErrors.push(String(error)));
  page.on('request', request => { const route = new URL(request.url()).pathname; if (route.startsWith('/api/')) requests.push({ route, body: request.postData() ? request.postDataJSON() : null }); });
  page.on('response', response => { const route = new URL(response.url()).pathname; if (route.startsWith('/api/')) responses.push({ route, status: response.status() }); });
  await page.goto(`${b.base}/#token=${b.token}`);
  let editor = await sourceEditor(); assert.equal(await sourceText(editor), notes.Live.source);
  const draft = notes.Live.source + '\nPRIVATE LIVE DRAFT — keep original Undo 🌿';
  await enterDraft(editor, draft); await editor.evaluate(element => window.__trashOriginalEditor = element);
  let { panel } = await trashPanel();
  const parentName = 'Restored Parent', parentDestination = path.join(rb, 'Live'), otherName = 'Restored Other';
  await setDraft(panel, parentEntry, parentName, parentDestination); await setDraft(panel, otherEntry, otherName, rb);
  await refresh(panel); await assertDraft(panel, parentEntry, parentName, parentDestination); await assertDraft(panel, otherEntry, otherName, rb);
  await panel.getByRole('button', { name: '关闭', exact: true }).click(); ({ panel } = await trashPanel());
  await assertDraft(panel, parentEntry, parentName, parentDestination); await assertDraft(panel, otherEntry, otherName, rb);
  const beforeOrder = await panel.locator('article.knowledge-card > .knowledge-address').allTextContents();
  await operation(b, rb, { kind: 'trashNode', source: path.join(rb, 'Reorder') }, `10000000-0000-4000-8000-${randomUUID().slice(-12)}`);
  const reordered = await refresh(panel), afterOrder = await panel.locator('article.knowledge-card > .knowledge-address').allTextContents();
  assert.equal(reordered.length, 3); assert.ok(beforeOrder.every(location => afterOrder.indexOf(location) === beforeOrder.indexOf(location) + 1));
  await assertDraft(panel, parentEntry, parentName, parentDestination); await assertDraft(panel, otherEntry, otherName, rb);
  await panel.screenshot({ path: path.join(out, 'independent-drafts-after-real-reorder.png') });
  result.cases.push('A real retired Clone exposes two exact roots under one operation; distinct UI name/parent drafts survive refresh, close/reopen and a real earlier-sorted Trash insertion');

  const preparedOther = await tap('/api/operations/prepare', () => card(panel, otherEntry).getByRole('button', { name: '准备恢复', exact: true }).click(), true);
  assert.deepEqual(preparedOther.request.intent, { kind: 'restoreNode', trashOperationId: cloneId, nodeId: notes.Other.nodeId, parent: rb, name: otherName });
  await panel.getByText(/原请求已保留，检查原操作即可/).waitFor();
  await setDraft(panel, otherEntry, 'Later field input must not replace original', parentDestination);
  const retry = await api(b, '/api/operations/prepare', preparedOther.request);
  assert.equal(retry.operationId, preparedOther.request.operationId); assert.equal(retry.planId, preparedOther.response.planId);
  const statusCount = requests.filter(item => item.route === '/api/operations/status').length;
  await panel.getByRole('button', { name: '关闭', exact: true }).click(); ({ panel } = await trashPanel());
  assert.ok(requests.slice().filter(item => item.route === '/api/operations/status').slice(statusCount).some(item => item.body.operationId === preparedOther.request.operationId));
  await panel.getByRole('heading', { name: '恢复预览', exact: true }).waitFor();
  assert.ok((await panel.innerText()).includes(`恢复名称：${otherName}`));
  const restoredOther = await tap('/api/operations/commit', () => panel.getByRole('button', { name: '确认恢复', exact: true }).click());
  assert.deepEqual(restoredOther.request, { operationId: preparedOther.request.operationId, planId: preparedOther.response.planId });
  assert.equal(restoredOther.response.document.nodeId, notes.Other.nodeId);
  const otherPath = path.join(rb, otherName, `${otherName}.typ`);
  assert.equal(normalize(restoredOther.response.document.path), normalize(otherPath)); assert.equal(await readFile(otherPath, 'utf8'), notes.Other.source);
  await until(() => card(panel, otherEntry).count(), count => count === 0, 'only selected Other root removed from UI');
  assert.equal(await exists(otherEntry.path), false); assert.equal(await exists(parentEntry.path), true);
  const afterOther = (await api(b, '/api/knowledge/trash', { root: rb })).trash;
  assert.ok(afterOther.some(entry => entry.operationId === cloneId && entry.nodeId === notes.Parent.nodeId)); assert.ok(!afterOther.some(entry => entry.nodeId === notes.Other.nodeId));
  await assertDraft(panel, parentEntry, parentName, parentDestination);
  assert.equal(requests.filter(item => item.route === '/api/operations/prepare' && item.body.operationId === preparedOther.request.operationId).length, 1);
  assert.equal(requests.filter(item => item.route === '/api/operations/commit' && item.body.operationId === preparedOther.request.operationId).length, 1);
  result.cases.push('Dropped real prepare response retains selected Other root and original request; exact same-ID retry returns the same Plan, reopening only inspects it, and explicit confirmation restores only that root');

  const preparedParent = await tap('/api/operations/prepare', () => card(panel, parentEntry).getByRole('button', { name: '准备恢复', exact: true }).click());
  assert.deepEqual(preparedParent.request.intent, { kind: 'restoreNode', trashOperationId: cloneId, nodeId: notes.Parent.nodeId, parent: parentDestination, name: parentName });
  const restoredParent = await tap('/api/operations/commit', () => panel.getByRole('button', { name: '确认恢复', exact: true }).click(), true);
  assert.equal(restoredParent.response.document.nodeId, notes.Parent.nodeId);
  await panel.getByText(/恢复结果未知，请检查原操作/).waitFor();
  await panel.getByRole('button', { name: '关闭', exact: true }).click(); ({ panel } = await trashPanel());
  await until(() => card(panel, parentEntry).count(), count => count === 0, 'committed Parent status inspected without another write');
  const parentPath = path.join(parentDestination, parentName, `${parentName}.typ`), childPath = path.join(parentDestination, parentName, 'Child', 'Child.typ');
  assert.equal(await readFile(parentPath, 'utf8'), notes.Parent.source); assert.equal(await readFile(childPath, 'utf8'), notes.Child.source);
  assert.deepEqual(await readFile(path.join(parentDestination, parentName, 'Child', 'original.bin')), Buffer.from([0, 255, 7]));
  const childNow = await api(b, '/api/data/open', { root: rb, entityKey: `weftext-node://${identity.workspaceId}/${notes.Child.nodeId}`, lifecycle: 'live' });
  assert.equal(childNow.nodeId, notes.Child.nodeId); assert.equal(normalize(childNow.path), normalize(childPath)); assert.equal(childNow.source, notes.Child.source);
  const retainedParent = await api(b, '/api/operations/prepare', preparedParent.request);
  assert.equal(retainedParent.planId, preparedParent.response.planId); assert.equal(retainedParent.status, 'committed');
  assert.equal(requests.filter(item => item.route === '/api/operations/prepare' && item.body.operationId === preparedParent.request.operationId).length, 1);
  assert.equal(requests.filter(item => item.route === '/api/operations/commit' && item.body.operationId === preparedParent.request.operationId).length, 1);
  const remaining = (await api(b, '/api/knowledge/trash', { root: rb })).trash;
  assert.equal(remaining.length, 1); assert.equal(remaining[0].name, 'Reorder'); assert.equal(await exists(parentEntry.path), false);
  await panel.screenshot({ path: path.join(out, 'original-parent-commit-recovered.png') });
  await panel.getByRole('button', { name: '关闭', exact: true }).click();
  await page.locator('.document-tab').filter({ has: page.locator('.tab-name', { hasText: /^Live\.typ$/ }) }).click(); editor = await sourceEditor();
  assert.equal(await sourceText(editor), draft); assert.equal(await editor.evaluate(element => element === window.__trashOriginalEditor), true);
  for (const [keys, source] of [['ControlOrMeta+z', notes.Live.source], ['ControlOrMeta+Shift+z', draft]]) {
    const persisted = page.waitForResponse(response => response.url().endsWith('/api/draft') && response.request().postDataJSON()?.source === source && response.ok());
    await editor.click(); await editor.press(keys); await until(() => sourceText(editor), value => value === source, 'original Live editor Undo/Redo'); await persisted;
  }
  const liveNow = await api(b, '/api/open', { root: rb, path: path.join(rb, 'Live', 'Live.typ') });
  assert.equal(liveNow.source, notes.Live.source); assert.equal(liveNow.recoveredDraft, draft);
  await page.screenshot({ path: path.join(out, 'retained-live-draft-and-undo.png'), fullPage: true });
  result.cases.push('Dropped real Parent commit response is recovered only by original status; Parent and inherited Child restore together with exact sources/resource, and the original unrelated live editor, private Draft and real Undo/Redo survive both restores');
  assert.deepEqual(result.pageErrors, []); assert.deepEqual(result.routeErrors, []); assert.ok(servers.every(server => server.stderr === ''), 'Owned server stderr must be empty'); result.completed = true;
} catch (error) {
  result.failure = String(error.stack ?? error); if (page && !page.isClosed()) await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {}); throw error;
} finally {
  await browser?.close(); for (const server of servers) await stop(server);
  result.ownedProcessesClosed = servers.every(server => server.child.exitCode !== null || server.child.signalCode !== null) && !browser?.isConnected();
  result.serverStderrBytes = servers.reduce((sum, server) => sum + Buffer.byteLength(server.stderr), 0);
  for (const [index, server] of servers.entries()) { await writeFile(path.join(out, `server-${index}-${server.label}.stdout.log`), server.stdout); await writeFile(path.join(out, `server-${index}-${server.label}.stderr.log`), server.stderr); }
  await writeFile(path.join(out, 'browser-requests.json'), JSON.stringify(requests, null, 2)); await writeFile(path.join(out, 'browser-responses.json'), JSON.stringify(responses, null, 2));
  await writeFile(path.join(out, 'observed-original-operations.json'), JSON.stringify(observed, null, 2)); await writeFile(path.join(out, 'results.json'), stringifyServeJson(result, 2));
  console.log(JSON.stringify({ out, completed: result.completed, cases: result.cases, pageErrors: result.pageErrors, routeErrors: result.routeErrors, ownedProcessesClosed: result.ownedProcessesClosed }));
}
