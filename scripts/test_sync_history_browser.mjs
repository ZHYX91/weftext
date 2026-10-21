import { createdDocument } from './_creation_result.mjs';
// Two owned programs and headless Chrome. No desktop input or borrowed state.
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
const result = { exe, sha256: createHash('sha256').update(await readFile(exe)).digest('hex'), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(), cases: [], pageErrors: [], routeErrors: [], completed: false, ownedProcessesClosed: false };
const servers = []; let browser, pageA, pageB, stderr = '';
const pause = ms => new Promise(r => setTimeout(r, ms));
async function start(args) {
  const child = spawn(exe, args, { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] }); servers.push(child);
  let output = ''; child.stdout.on('data', b => output += b); child.stderr.on('data', b => stderr += b);
  for (let i = 0; i < 300; i++) {
    const m = output.match(/(http:\/\/127\.0\.0\.1:\d+)\/#token=(\w+)/);
    if (m) return { child, base: m[1], token: m[2] };
    if (child.exitCode !== null) throw new Error(stderr); await pause(100);
  }
  throw new Error('startup timeout');
}
async function stop(child) { if (child && child.exitCode === null && child.signalCode === null) { const exited = new Promise(r => child.once('exit', r)); child.kill(); await exited; } }
async function api(server, route, data) {
  const response = await fetch(server.base + route, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': server.token }, body: JSON.stringify(data) });
  const body = await response.json(); assert.ok(response.ok, `${route}: ${JSON.stringify(body)}`); return body;
}
async function sync(page) {
  await page.getByRole('button', { name: '同步', exact: true }).click();
  const panel = page.getByRole('dialog', { name: '资料库同步' }); await panel.waitFor();
  await panel.getByRole('status').filter({ hasText: '操作已完成' }).waitFor(); return panel;
}
async function click(panel, name) {
  await panel.getByRole('button', { name, exact: true }).click();
  await panel.getByRole('status').filter({ hasText: '操作已完成' }).waitFor();
}
async function close(panel) { await panel.getByRole('button', { name: '关闭同步', exact: true }).click(); }
async function sourceEditor(page) {
  await page.getByRole('button', { name: '源码', exact: true }).click();
  const editor = page.locator('.cm-content[contenteditable=true]'); await editor.waitFor(); return editor;
}
const text = async editor => (await editor.locator('.cm-line').allTextContents()).join('\n');
async function enterDraft(page, editor, source) {
  // Exercise CodeMirror's real input transaction and its personal-Draft write,
  // rather than treating a contenteditable DOM replacement as editor state.
  const persisted = page.waitForResponse(response => {
    if (!response.url().endsWith('/api/draft') || response.request().method() !== 'POST') return false;
    return response.request().postDataJSON()?.source === source && response.ok();
  });
  await editor.press('ControlOrMeta+a'); await page.keyboard.insertText(source);
  await persisted; assert.equal(await text(editor), source);
}
try {
  const ra = path.join(out, 'library-a'), rb = path.join(out, 'library-b'), da = path.join(out, 'state-a'), db = path.join(out, 'state-b'), exchange = path.join(out, 'exchange');
  let a = await start(['--no-browser', '--data-dir', da]), b = await start(['--no-browser', '--data-dir', db]);
  await api(a, '/api/library/create', { path: ra, operationId: randomUUID() });
  const note = createdDocument(await api(a, '/api/note/create', { root: ra, parent: ra, name: 'Note', operationId: randomUUID() }));
  const key = { kind: 'node', id: note.nodeId };
  const { identity } = await api(a, '/api/sync/enable', { root: ra, operationId: randomUUID() });
  async function transfer() {
    await api(a, '/api/sync/export', { root: ra, directory: exchange, objects: [key] });
    return api(b, '/api/sync/receive', { identity, directory: exchange });
  }
  await transfer();
  const clone = await api(b, '/api/operations/prepare', { root: rb, operationId: randomUUID(), intent: { kind: 'cloneSync', identity, objects: [key] } });
  await api(b, '/api/operations/commit', { operationId: clone.operationId, planId: clone.planId });
  const history = await api(b, '/api/sync/history', { identity, object: key });
  const original = history.versions[0], initial = note.source;
  for (const suffix of ['历史二 中文', '历史三 😀']) {
    const opened = await api(a, '/api/open', { root: ra, path: note.path });
    await api(a, '/api/save', { sessionId: opened.sessionId, baseId: opened.baseId, source: `${opened.source}\n${suffix}`, operationId: randomUUID() });
    await transfer();
  }
  await stop(a.child); await stop(b.child);
  a = await start(['--no-browser', '--data-dir', da, '--open', note.path, '--root', ra]);
  const bPath = path.join(rb, 'Note', 'Note.typ');
  b = await start(['--no-browser', '--data-dir', db, '--open', bPath, '--root', rb]);
  browser = await chromium.launch({ channel: 'chrome', headless: true }); result.browser = browser.version();
  pageA = await (await browser.newContext({ viewport: { width: 1440, height: 1000 } })).newPage();
  pageB = await (await browser.newContext({ viewport: { width: 1440, height: 1000 } })).newPage();
  for (const p of [pageA, pageB]) p.on('pageerror', e => result.pageErrors.push(String(e)));
  await pageB.goto(`${b.base}/#token=${b.token}`);
  const editorB = await sourceEditor(pageB), draftB = initial + '\n读历史期间的个人草稿';
  await enterDraft(pageB, editorB, draftB);
  let writesDuringHistory = 0;
  const count = request => { if (/\/api\/operations\/(prepare|commit)$/.test(new URL(request.url()).pathname)) writesDuringHistory++; };
  pageB.on('request', count);
  const pb = await sync(pageB), row = pb.locator('.sync-object').filter({ hasText: key.id });
  await row.getByRole('button', { name: '查看收到的全部历史', exact: true }).click();
  await pb.getByRole('status').filter({ hasText: '操作已完成' }).waitFor();
  const historyPanel = pb.getByRole('region', { name: '收到的对象历史' });
  const entry = historyPanel.locator('details').filter({ has: pageB.locator('summary', { hasText: original.revision }) });
  await entry.locator('summary').click();
  assert.match(await entry.innerText(), /parents/); assert.match(await entry.innerText(), /before/);
  await click(pb, `只读查看历史 ${original.revision}`);
  assert.equal(await pb.getByRole('button', { name: '预览采用此完整分支', exact: true }).count(), 0);
  await pb.locator('details').filter({ has: pageB.locator('summary', { hasText: /^Note.typ$/ }) }).locator('summary').click();
  await click(pb, '读取 Note.typ');
  const filePreview = pb.locator('details').filter({ has: pageB.locator('summary', { hasText: /^Note.typ$/ }) });
  assert.equal(await filePreview.locator('pre').innerText(), initial);
  assert.match(await historyPanel.innerText(), /本机未安装此版本/);
  assert.equal(writesDuringHistory, 0);
  assert.equal(await readFile(bPath, 'utf8'), initial);
  await pb.screenshot({ path: path.join(out, 'history-readonly.png') }); await close(pb);
  assert.equal(await text(editorB), draftB); pageB.off('request', count);
  result.cases.push('Exact received ancestor outside heads/common bases is read-only; real causal metadata and installed state shown; no author operation, source overwrite or Draft loss');

  await pageA.goto(`${a.base}/#token=${a.token}`);
  const editorA = await sourceEditor(pageA), beforeExternal = await readFile(note.path, 'utf8');
  const draftA = beforeExternal + '\n仍在编辑的私有草稿'; await enterDraft(pageA, editorA, draftA);
  const external1 = beforeExternal + '\n外部当前内容一'; await writeFile(note.path, external1, 'utf8');
  let pa = await sync(pageA); await click(pa, '预览采纳当前外部修改');
  assert.match(await pa.innerText(), /adoptSyncExternal/);
  assert.equal(await readFile(note.path, 'utf8'), external1);
  const external2 = beforeExternal + '\n外部第三状态'; await writeFile(note.path, external2, 'utf8');
  await pa.getByRole('button', { name: '确认应用此预览', exact: true }).click();
  await pa.getByRole('status').filter({ hasText: '原操作记录保留' }).waitFor();
  assert.equal(await readFile(note.path, 'utf8'), external2);
  await click(pa, '检查原操作结果'); assert.match(await pa.innerText(), /原同步操作：prepared/);
  await click(pa, '取消尚未开始写入的原提案'); await click(pa, '结束已确认操作');
  await click(pa, '预览采纳当前外部修改');
  let committed; const commitDone = new Promise(r => committed = r);
  await pageA.route('**/api/operations/commit', async route => {
    try { const response = await route.fetch(); assert.equal(response.status(), 200); committed(); await route.abort('failed'); }
    catch (error) { result.routeErrors.push(String(error)); committed(); await route.abort('failed').catch(() => {}); }
  });
  await pa.getByRole('button', { name: '确认应用此预览', exact: true }).click(); await commitDone;
  await pa.getByRole('status').filter({ hasText: '原操作记录保留' }).waitFor();
  await pageA.unroute('**/api/operations/commit'); await close(pa); await pageA.reload();
  pa = await sync(pageA); assert.match(await pa.innerText(), /adoptSyncExternal/);
  await click(pa, '检查原操作结果'); assert.match(await pa.innerText(), /原同步操作：committed/);
  assert.equal(await readFile(note.path, 'utf8'), external2);
  await pa.screenshot({ path: path.join(out, 'adopt-original-recovery.png') });
  await click(pa, '结束已确认操作'); await close(pa);
  const retained = await api(a, '/api/open', { root: ra, path: note.path });
  assert.equal(retained.source, external2); assert.equal(retained.recoveredDraft, draftA);
  await api(a, '/api/sync/publish', { root: ra });
  const adopted = await api(a, '/api/sync/history', { identity, object: key });
  const contents = await Promise.all(adopted.versions.map(v => api(a, '/api/sync/file', { identity, object: key, revision: v.revision, path: 'Note.typ' })));
  assert.ok(contents.some(v => v.text === external2)); assert.ok(!contents.some(v => v.text === external1));
  result.cases.push('Explicit external-content adoption refuses a changed original preview, preserves the third state, then recovers the actual committed original after lost response/reload while preserving private Draft');
  assert.deepEqual(result.pageErrors, []); assert.deepEqual(result.routeErrors, []); result.completed = true;
} catch (error) {
  result.failure = String(error.stack ?? error);
  for (const [name, page] of [['a', pageA], ['b', pageB]]) if (page) await page.screenshot({ path: path.join(out, `failure-${name}.png`), fullPage: true }).catch(() => {});
  throw error;
} finally {
  await browser?.close(); for (const server of servers) await stop(server);
  result.ownedProcessesClosed = servers.every(s => s.exitCode !== null || s.signalCode !== null);
  await writeFile(path.join(out, 'results.json'), JSON.stringify(result, null, 2));
  await writeFile(path.join(out, 'server.stderr.log'), stderr);
  console.log(JSON.stringify({ out, completed: result.completed, cases: result.cases, pageErrors: result.pageErrors, routeErrors: result.routeErrors, ownedProcessesClosed: result.ownedProcessesClosed }));
}
