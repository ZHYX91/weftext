import { workspaceCommand, workspaceMode } from './_workspace_browser.mjs';
import { spawnServe, serveReady, stopServe, stringifyServeJson } from './_cli_serve.mjs';
import { createdDocument } from './_creation_result.mjs';
// An ordinary Directory moves through real public producers, then the real
// receiver Sync UI. No synthetic Plan, response, editor state or private record.
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile, appendFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import assert from 'node:assert/strict';

if (!process.argv[2] || !process.argv[3]) throw new Error('Usage: node test_sync_directory_browser.mjs EXE NEW_OUT');
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]);
await mkdir(out, { recursive: false });
const result = { exe, sha256: createHash('sha256').update(await readFile(exe)).digest('hex'), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(), cases: [], pageErrors: [], completed: false };
const servers = [], calls = [];
let browser, page;
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
async function until(read, predicate, label, ms = 30000) {
  const deadline = Date.now() + ms; let last;
  while (Date.now() < deadline) { last = await read(); if (predicate(last)) return last; await pause(60); }
  throw new Error(`${label} timeout: ${JSON.stringify(last)}`);
}
async function start(label, args) {
  const child = spawnServe(exe, [...args], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  const server = { label, child, stdout: '', stderr: '' }; servers.push(server);
  child.stdout.on('data', bytes => server.stdout += bytes); child.stderr.on('data', bytes => server.stderr += bytes);
  await until(() => serveReady(child), Boolean, `${label} start`);
  const match = serveReady(child);
  return Object.assign(server, { base: match.base, token: match.token });
}
async function stop(server) { await stopServe(server?.child); }
async function api(server, route, request) {
  const response = await fetch(server.base + route, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': server.token }, body: JSON.stringify(request), signal: AbortSignal.timeout(45000) });
  const body = await response.json();
  await appendFile(path.join(out, 'http.ndjson'), JSON.stringify({ server: server.label, route, request, status: response.status, body }) + '\n');
  assert.equal(response.status, 200, `${route}: ${JSON.stringify(body)}`); return body;
}
async function commit(server, plan) {
  const receipt = await api(server, '/api/operations/commit', { operationId: plan.operationId, planId: plan.planId });
  assert.equal(receipt.status, 'committed'); return receipt;
}
async function operation(server, root, intent) {
  const plan = await api(server, '/api/operations/prepare', { root, operationId: randomUUID(), intent });
  return { plan, receipt: await commit(server, plan) };
}
async function physical(location) {
  const value = await stat(location, { bigint: true }); assert.notEqual(value.ino, 0n);
  return { dev: String(value.dev), ino: String(value.ino), birth: String(value.birthtimeNs) };
}
const normalize = value => path.resolve(value).replaceAll('\\', '/').toLowerCase();
const sourceText = editor => editor.locator('.cm-line').allTextContents().then(lines => lines.join('\n'));
function responseFor(route, predicate = () => true) {
  const promise = page.waitForResponse(response => response.url().endsWith(route) && predicate(response));
  promise.catch(() => {}); return promise;
}
async function clickDone(panel, name) {
  await panel.getByRole('button', { name, exact: true }).click();
  await panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor();
}

try {
  const ra = path.join(out, 'a'), rb = path.join(out, 'b'), da = path.join(out, 'da'), db = path.join(out, 'db');
  const a = await start('sender', ['--data-dir', da]); let b = await start('receiver-setup', ['--data-dir', db]);
  await api(a, '/api/library/create', { path: ra, operationId: randomUUID() });
  const folder = path.join(ra, 'Folder'); await mkdir(folder);
  const admitted = await api(a, '/api/navigation/admit-directory', { root: ra, path: folder, operationId: randomUUID() });
  await commit(a, admitted.preview);
  const marker = JSON.parse(await readFile(path.join(folder, 'weftext.directory.json'), 'utf8'));
  const directoryKey = { kind: 'directory', id: marker.directoryId };
  const child = createdDocument(await api(a, '/api/note/create', { root: ra, parent: folder, name: 'Child', operationId: randomUUID() }));
  const savedSource = child.source + '\n= 目录子笔记\n原始正文 中文😀\n';
  await api(a, '/api/save', { sessionId: child.sessionId, baseId: child.baseId, source: savedSource, operationId: randomUUID() });
  const container = createdDocument(await api(a, '/api/note/create', { root: ra, parent: ra, name: 'Container', operationId: randomUUID() }));
  const childKey = { kind: 'node', id: child.nodeId }, containerKey = { kind: 'node', id: container.nodeId };
  const binding = await api(a, '/api/sync/enable', { root: ra, operationId: randomUUID() }), identity = binding.identity;
  const objects = [directoryKey, childKey, containerKey];
  const initial = path.join(out, 'initial');
  await api(a, '/api/sync/export', { root: ra, directory: initial, objects });
  await api(b, '/api/sync/receive', { identity, directory: initial });
  await operation(b, rb, { kind: 'cloneSync', identity, objects });
  const original = path.join(rb, 'Folder'), childRelative = path.join('Child', 'Child.typ');
  const before = {};
  for (const relative of ['', 'weftext.directory.json', 'Child', childRelative]) before[relative] = await physical(path.join(original, relative));
  await stop(b); b = await start('receiver-ui', ['--data-dir', db, '--open', path.join(original, childRelative), '--root', rb]);
  browser = await chromium.launch({ channel: 'chrome', headless: true }); result.browser = browser.version();
  page = await (await browser.newContext({ viewport: { width: 1500, height: 1050 } })).newPage(); page.setDefaultTimeout(30000);
  page.on('pageerror', error => result.pageErrors.push(String(error)));
  page.on('request', request => { if (new URL(request.url()).pathname.startsWith('/api/')) calls.push({ route: new URL(request.url()).pathname, body: request.postData() ? request.postDataJSON() : null }); });
  await page.goto(`${b.base}/#token=${b.token}`);
  await workspaceMode(page, '源码');
  const editor = page.locator('.cm-content[contenteditable=true]'); await editor.waitFor();
  assert.equal(await sourceText(editor), savedSource);
  const draft = savedSource + '\n目录移动时必须保留的未保存稿 🌿';
  const draftSaved = responseFor('/api/draft', response => response.ok() && response.request().postDataJSON()?.source === draft);
  await editor.press('ControlOrMeta+a'); await page.keyboard.insertText(draft); await draftSaved;
  await editor.evaluate(element => window.__directoryOriginalEditor = element);
  const reference = `weftext-directory://${identity.workspaceId}/${marker.directoryId}`;
  const produced = await operation(a, ra, { kind: 'relocateDirectory', source: folder, parent: path.join(ra, 'Container'), name: 'Moved' });
  assert.equal(produced.plan.kind, 'relocateDirectory');
  const exchange = path.join(out, 'moved');
  await api(a, '/api/sync/export', { root: ra, directory: exchange, objects });
  await workspaceCommand(page, 'knowledge', '同步');
  const panel = page.getByRole('dialog', { name: '资料库同步' }); await panel.waitFor();
  await panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor();
  await panel.getByLabel('交换目录路径（新目录或已配对目录）', { exact: true }).fill(exchange);
  await clickDone(panel, '接收交换目录');
  const rows = panel.locator('.sync-object');
  for (let index = 0; index < await rows.count(); index++) await rows.nth(index).locator('input[type=checkbox]').uncheck();
  for (const key of objects) await rows.filter({ hasText: `(${key.kind}:${key.id})` }).locator('input[type=checkbox]').check();
  const prepareResponse = responseFor('/api/operations/prepare');
  await clickDone(panel, '预览应用所选对象');
  const response = await prepareResponse, prepared = await response.json(); assert.equal(response.status(), 200, JSON.stringify(prepared));
  const request = response.request().postDataJSON(); assert.equal(request.intent.kind, 'applySync');
  assert.equal(await readFile(path.join(original, childRelative), 'utf8'), savedSource);
  const commitResponse = responseFor('/api/operations/commit');
  await clickDone(panel, '确认应用此预览');
  const committedResponse = await commitResponse, receipt = await committedResponse.json();
  assert.equal(committedResponse.status(), 200, JSON.stringify(receipt)); assert.equal(receipt.status, 'committed');
  assert.deepEqual(committedResponse.request().postDataJSON(), { operationId: prepared.operationId, planId: prepared.planId });
  await clickDone(panel, '结束已确认操作'); await panel.getByRole('button', { name: '关闭同步', exact: true }).click();
  assert.equal(await editor.evaluate(element => element === window.__directoryOriginalEditor), true);
  assert.equal(await sourceText(editor), draft);
  for (const [keys, expected] of [['ControlOrMeta+z', savedSource], ['ControlOrMeta+Shift+z', draft]]) {
    const persisted = responseFor('/api/draft', value => value.ok() && value.request().postDataJSON()?.source === expected);
    await editor.click(); await editor.press(keys); await until(() => sourceText(editor), source => source === expected, 'original Undo/Redo'); await persisted;
  }
  const target = path.join(rb, 'Container', 'Moved'), finalChild = path.join(target, childRelative);
  for (const [relative, stamp] of Object.entries(before)) assert.deepEqual(await physical(path.join(target, relative)), stamp);
  await assert.rejects(stat(original), { code: 'ENOENT' }); await assert.rejects(stat(path.join(target, 'Moved.typ')), { code: 'ENOENT' });
  const resolved = await api(b, '/api/navigation/resolve', { root: rb, targetRef: reference });
  assert.equal(resolved.status, 'managed'); assert.equal(normalize(resolved.path), normalize(target));
  const opened = await api(b, '/api/data/open', { root: rb, entityKey: `weftext-node://${identity.workspaceId}/${child.nodeId}` });
  assert.equal(normalize(opened.path), normalize(finalChild)); assert.equal(opened.source, savedSource); assert.equal(opened.recoveredDraft, draft);
  await until(() => page.locator('.tree-row[title]').evaluateAll(elements => elements.map(element => element.title.split('\n')[0].replaceAll('\\', '/').toLowerCase())), titles => titles.includes(normalize(finalChild)), 'moved child sidebar');
  await page.getByRole('button', { name: '比较磁盘内容', exact: true }).click();
  const comparison = page.getByRole('dialog').filter({ has: page.getByRole('heading', { name: '比较草稿与磁盘内容', exact: true }) }); await comparison.waitFor();
  assert.equal(await comparison.getByLabel('草稿原始版本', { exact: true }).inputValue(), savedSource);
  assert.equal(await comparison.getByLabel('当前磁盘内容', { exact: true }).inputValue(), savedSource);
  assert.equal(await comparison.getByLabel('合并后的草稿（可编辑）', { exact: true }).inputValue(), draft);
  await comparison.screenshot({ path: path.join(out, 'directory-child-exact-comparison.png') });
  await comparison.getByRole('button', { name: '应用合并后的草稿', exact: true }).click(); await comparison.waitFor({ state: 'detached' });
  const saved = responseFor('/api/save', value => value.ok() && value.request().postDataJSON()?.source === draft);
  await page.getByRole('button', { name: '保存', exact: true }).click(); await saved;
  assert.equal(await readFile(finalChild, 'utf8'), draft);
  assert.equal(await editor.evaluate(element => element === window.__directoryOriginalEditor), true);
  assert.equal(calls.filter(call => call.route === '/api/operations/prepare').length, 1);
  assert.equal(calls.filter(call => call.route === '/api/operations/commit').length, 1);
  await writeFile(path.join(out, 'fixture.json'), JSON.stringify({ identity, reference, objects, before, produced, request, prepared, receipt, savedSource, draft, finalChild }, null, 2));
  result.cases.push('Ordinary Directory relocation retains marker/child FI and stable refs; actual Sync UI keeps original editor/Draft/Undo, refreshes navigation and saves only after explicit comparison');
  assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) {
  result.failure = String(error.stack ?? error);
  if (page && !page.isClosed()) await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {});
  throw error;
} finally {
  await browser?.close(); for (const server of servers) await stop(server);
  result.ownedProcessesClosed = servers.every(server => server.child.exitCode !== null || server.child.signalCode !== null) && !browser?.isConnected();
  result.serverStderrBytes = servers.reduce((sum, server) => sum + Buffer.byteLength(server.stderr), 0);
  for (const [index, server] of servers.entries()) {
    await writeFile(path.join(out, `${index}-${server.label}.stdout.log`), server.stdout);
    await writeFile(path.join(out, `${index}-${server.label}.stderr.log`), server.stderr);
  }
  await writeFile(path.join(out, 'browser-requests.json'), JSON.stringify(calls, null, 2));
  await writeFile(path.join(out, 'results.json'), stringifyServeJson(result, 2));
  console.log(JSON.stringify(result));
  if (result.completed) {
    assert.equal(result.ownedProcessesClosed, true);
    assert.equal(result.serverStderrBytes, 0);
  }
}
