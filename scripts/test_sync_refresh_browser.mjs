import { createdDocument } from './_creation_result.mjs';
// Two owned replicas; real sidecar synchronization and delayed UI delivery.
// Composition events test the browser gate, not an operating-system IME.
import { createRequire } from 'node:module';
import { spawn, execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import assert from 'node:assert/strict';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]);
await mkdir(out, { recursive: false });
const result = { exe, sha256: createHash('sha256').update(await readFile(exe)).digest('hex'), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(), cases: [], pageErrors: [], routeErrors: [], completed: false };
const servers = []; let browser, page, stderr = '', release;
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
async function start(args) {
  const child = spawn(exe, args, { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] }); servers.push(child);
  let output = ''; child.stdout.on('data', bytes => output += bytes); child.stderr.on('data', bytes => stderr += bytes);
  for (let i = 0; i < 300; i++) {
    const match = output.match(/(http:\/\/127\.0\.0\.1:\d+)\/#token=(\w+)/);
    if (match) return { child, base: match[1], token: match[2] };
    if (child.exitCode !== null) throw new Error(stderr); await pause(100);
  }
  throw new Error('startup timeout');
}
async function stop(child) { if (child.exitCode === null && child.signalCode === null) { const done = new Promise(resolve => child.once('exit', resolve)); child.kill(); await done; } }
async function api(server, route, data) {
  const response = await fetch(server.base + route, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': server.token }, body: JSON.stringify(data) });
  const value = await response.json(); assert.equal(response.status, 200, JSON.stringify(value)); return value;
}
async function commit(server, plan) { return api(server, '/api/operations/commit', { operationId: plan.operationId, planId: plan.planId }); }
async function click(panel, name) { await panel.getByRole('button', { name, exact: true }).click(); await panel.getByRole('status').filter({ hasText: '操作已完成' }).waitFor(); }
try {
  const ra = path.join(out, 'library-a'), rb = path.join(out, 'library-b'), da = path.join(out, 'state-a'), db = path.join(out, 'state-b'), exchange = path.join(out, 'exchange');
  let a = await start(['--no-browser', '--data-dir', da]);
  const b = await start(['--no-browser', '--data-dir', db]);
  await api(a, '/api/library/create', { path: ra, operationId: randomUUID() });
  let note = createdDocument(await api(a, '/api/note/create', { root: ra, parent: ra, name: 'Note', operationId: randomUUID() }));
  const savedSource = note.source + '\n= 同步正文\n\n准确正文😀\n';
  await api(a, '/api/save', { sessionId: note.sessionId, baseId: note.baseId, source: savedSource, operationId: randomUUID() });
  const { identity } = await api(a, '/api/sync/enable', { root: ra, operationId: randomUUID() });
  const object = { kind: 'node', id: note.nodeId };
  await api(a, '/api/sync/export', { root: ra, directory: exchange, objects: [object] });
  await api(b, '/api/sync/receive', { identity, directory: exchange });
  await commit(b, await api(b, '/api/operations/prepare', { root: rb, operationId: randomUUID(), intent: { kind: 'cloneSync', identity, objects: [object] } }));
  await stop(a.child); a = await start(['--no-browser', '--data-dir', da, '--open', note.path, '--root', ra]);
  browser = await chromium.launch({ channel: 'chrome', headless: true }); result.browser = browser.version();
  page = await browser.newPage({ viewport: { width: 1500, height: 1050 } }); page.setDefaultTimeout(30000);
  page.on('pageerror', error => result.pageErrors.push(String(error)));
  const calls = []; page.on('request', request => { const url = new URL(request.url()); if (url.pathname.startsWith('/api/')) calls.push(url.pathname); });
  await page.goto(`${a.base}/#token=${a.token}`);
  await page.getByRole('button', { name: '源码', exact: true }).click();
  const editor = page.locator('.cm-content[contenteditable=true]');
  const initialEditorSource = (await editor.locator('.cm-line').allTextContents()).join('\n');
  assert.equal(initialEditorSource, savedSource);
  await editor.press('ControlOrMeta+End'); await page.keyboard.insertText('保留个人草稿');
  const draft = (await editor.locator('.cm-line').allTextContents()).join('\n');
  assert.equal(draft, savedSource + '保留个人草稿');
  await page.getByRole('button', { name: '链接与批注', exact: true }).click();
  await page.getByRole('tab', { name: '批注', exact: true }).click();
  await page.locator('.knowledge-current').waitFor();
  const remote = await api(b, '/api/open', { root: rb, path: path.join(rb, 'Note', 'Note.typ') });
  const context = { sessionId: remote.sessionId, baseId: remote.baseId, source: remote.source, serial: 0, path: remote.path, root: rb, nodeId: remote.nodeId, workspaceId: remote.workspaceId, dirty: false };
  const marker = '仅批注同步：正文没有改变';
  const annotation = await api(b, '/api/knowledge/prepare', { operationId: randomUUID(), context, intent: { kind: 'annotationCreate', body: marker, target: { kind: 'whole_document' } } });
  await api(b, '/api/operations/commit', { operationId: annotation.operationId, planId: annotation.planId, context });
  await api(b, '/api/sync/export', { root: rb, directory: exchange, objects: [object] });
  await page.getByRole('button', { name: '同步', exact: true }).click();
  const panel = page.getByRole('dialog', { name: '资料库同步' });
  await panel.getByLabel('交换目录路径（新目录或已配对目录）', { exact: true }).fill(exchange);
  await click(panel, '接收交换目录'); await click(panel, '预览应用所选对象');
  let received; const held = new Promise(resolve => received = resolve), resumed = new Promise(resolve => release = resolve);
  await page.route('**/api/operations/commit', async route => {
    try { const response = await route.fetch(); assert.equal(response.status(), 200); received(); await resumed; await route.fulfill({ response }); }
    catch (error) { result.routeErrors.push(String(error)); received(); await route.abort('failed').catch(() => {}); }
  });
  await panel.getByRole('button', { name: '确认应用此预览', exact: true }).click(); await held;
  await editor.evaluate(element => { window.__syncOriginalEditor = element; element.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true, data: '中' })); });
  const beforeDelivery = calls.length; release(); await pause(350);
  assert.equal(await page.evaluate(() => window.__syncOriginalEditor.isConnected), true);
  assert.match(await panel.locator('p[role="status"]:not([aria-label])').innerText(), /正在读取或处理/);
  assert.equal(calls.slice(beforeDelivery).includes('/api/data/open'), false, 'receipt must wait for preedit before reading/replacing the current document');
  await editor.evaluate(element => element.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true, data: '' })));
  await panel.getByRole('status').filter({ hasText: '操作已完成' }).waitFor();
  await page.unroute('**/api/operations/commit');
  await click(panel, '结束已确认操作'); await panel.getByRole('button', { name: '关闭同步', exact: true }).click();
  await page.getByText(marker, { exact: true }).waitFor();
  const renewed = calls.slice(beforeDelivery);
  for (const route of ['/api/render', '/api/editor-map', '/api/knowledge/read', '/api/data/schema', '/api/query/run']) assert.ok(renewed.includes(route), 'missing actual consumer refresh: ' + route);
  const saved = await api(a, '/api/open', { root: ra, path: note.path });
  assert.equal(saved.source, savedSource); assert.equal(saved.recoveredDraft, draft);
  result.cases.push('actual sidecar-only sync refreshes annotations and native/query/Schema consumers while saved source bytes and the personal Draft remain unchanged');
  result.cases.push('a held committed sync receipt waits for browser composition end before rebuilding the source surface');
  assert.equal((await editor.locator('.cm-line').allTextContents()).join('\n'), draft);
  await editor.press('ControlOrMeta+z');
  assert.equal((await editor.locator('.cm-line').allTextContents()).join('\n'), savedSource);
  await editor.press('ControlOrMeta+Shift+z');
  assert.equal((await editor.locator('.cm-line').allTextContents()).join('\n'), draft);
  result.cases.push('the original shared editor Undo/Redo survives synchronized sidecar refresh without any source save or extra edit');
  await page.screenshot({ path: path.join(out, 'success-sidecar-refresh.png'), fullPage: true });
  assert.deepEqual(result.pageErrors, []); assert.deepEqual(result.routeErrors, []); result.completed = true;
} catch (error) {
  result.failure = String(error.stack ?? error); if (page) await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {}); throw error;
} finally {
  release?.(); await browser?.close(); for (const server of servers) await stop(server);
  result.ownedProcessesClosed = servers.every(server => server.exitCode !== null || server.signalCode !== null); result.stderrBytes = Buffer.byteLength(stderr);
  await writeFile(path.join(out, 'results.json'), JSON.stringify(result, null, 2)); await writeFile(path.join(out, 'server.stderr.log'), stderr);
  console.log(JSON.stringify({ out, completed: result.completed, cases: result.cases, pageErrors: result.pageErrors }));
}
