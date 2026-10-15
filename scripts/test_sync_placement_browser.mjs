// Actual swap consumer: same active Node, editor, Draft/Undo and explicit rebind.
// The Python fixture supplies real public producers and owns both HTTP programs.
import { createRequire } from 'node:module';
import { spawn, execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';

if (process.argv.length !== 4) throw new Error('Usage: node test_sync_placement_browser.mjs EXE NEW_OUT');
const require = createRequire(import.meta.url), { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]); await mkdir(out, { recursive: false });
const result = { exe, sha256: createHash('sha256').update(await readFile(exe)).digest('hex'), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(), completed: false, pageErrors: [], routeErrors: [], observations: [] };
const bridge = spawn(process.env.WEFTEXT_PYTHON || 'python', [path.join(import.meta.dirname, 'test_sync_placement.py'), exe, '--browser-fixture', path.join(out, 'http')], { windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] });
let stdout = '', stderr = '', browser, context, page, fixture, pendingRequest, preview;
bridge.stdout.on('data', chunk => { stdout += chunk; }); bridge.stderr.on('data', chunk => { stderr += chunk; });
bridge.stdin.on('error', () => {});
const bridgeExited = () => bridge.exitCode !== null || bridge.signalCode !== null;
const marker = prefix => stdout.split(/\r?\n/).slice(0, -1).find(line => line.startsWith(prefix));
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
async function until(read, predicate, label, milliseconds = 30000) { const deadline = Date.now() + milliseconds; let value; while (Date.now() < deadline) { value = await read(); if (predicate(value)) return value; await delay(60); } throw new Error(`${label} timeout; last=${JSON.stringify(value)}`); }
const sourceText = async () => (await page.locator('.cm-content[contenteditable=true] .cm-line').allTextContents()).join('\n');
const pending = async () => page.evaluate(() => JSON.parse(localStorage.getItem('weftext-sync-operation-v1') || 'null'));
async function command(value) { await new Promise((resolve, reject) => { bridge.stdin.write(JSON.stringify(value) + '\n', error => error ? reject(error) : resolve()); }); }
async function api(route, body) { const response = await fetch(fixture.url + route, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': fixture.token }, body: JSON.stringify(body), signal: AbortSignal.timeout(45000) }); const value = await response.json(); assert.equal(response.status, 200, `${route}: ${JSON.stringify(value)}`); return value; }
async function observeSource(label) {
  result.observations.push({ label, source: await sourceText(), focus: await page.evaluate(() => ({ tag: document.activeElement?.tagName, className: document.activeElement?.className,
    originalEditor: document.activeElement === window.__placementOriginalEditor, selection: document.getSelection()?.toString() })) });
}
async function waitDraft(source, action) {
  await observeSource('before Draft action');
  const saved = page.waitForResponse(response => response.url().endsWith('/api/draft') && response.request().postDataJSON()?.source === source && response.ok());
  try { await action(); await observeSource('after Draft action'); await saved; await until(sourceText, value => value === source, 'shared Draft'); }
  finally { await observeSource('settled Draft action'); }
}
try {
  const ready = await until(() => ({ line: marker('WEFTEXT_PLACEMENT_FIXTURE '), exited: bridgeExited() }), value => value.line || value.exited, 'real HTTP fixture', 180000);
  assert.equal(bridge.exitCode, null, `Fixture failed: ${stderr}`);
  assert.ok(ready.line, `Fixture failed: ${stderr}`); fixture = JSON.parse(ready.line.slice('WEFTEXT_PLACEMENT_FIXTURE '.length));
  browser = await chromium.launch({ channel: 'chrome', headless: true }); result.browser = browser.version(); context = await browser.newContext({ viewport: { width: 1500, height: 1050 } }); page = await context.newPage(); page.setDefaultTimeout(30000);
  await context.addInitScript(() => {
    window.__placementKeyEvents = [];
    for (const capture of [true, false]) for (const type of ['keydown', 'keyup']) document.addEventListener(type, event => {
      if (!['Control', 'Meta', 'Shift', 'z', 'Z', 'y', 'Y'].includes(event.key)) return;
      window.__placementKeyEvents.push({ phase: capture ? 'capture' : 'bubble', type, key: event.key, code: event.code, ctrl: event.ctrlKey, meta: event.metaKey,
        shift: event.shiftKey, composing: event.isComposing, prevented: event.defaultPrevented, target: event.target?.className, active: document.activeElement?.className,
        originalEditor: event.target === window.__placementOriginalEditor, source: [...document.querySelectorAll('.cm-content[contenteditable=true] .cm-line')].map(line => line.textContent).join('\n') });
    }, capture);
  });
  page.on('pageerror', error => result.pageErrors.push(String(error)));
  const requests = [], responses = []; result.requests = requests; result.responses = responses;
  page.on('request', request => { if (new URL(request.url()).pathname.startsWith('/api/')) requests.push({ route: new URL(request.url()).pathname, body: request.postData() ? request.postDataJSON() : null }); });
  page.on('response', response => { if (new URL(response.url()).pathname.startsWith('/api/')) responses.push({ route: new URL(response.url()).pathname, status: response.status() }); });
  const launched = page.waitForResponse(response => response.url().endsWith('/api/bootstrap'));
  await page.goto(`${fixture.url}/#token=${fixture.token}`); const initial = (await (await launched).json()).initialDocument;
  assert.equal(initial.nodeId, fixture.nodeId); assert.equal(initial.source, fixture.baseSource);
  await page.getByRole('button', { name: '源码', exact: true }).click(); const editor = page.locator('.cm-content[contenteditable=true]'); await editor.waitFor(); assert.equal(await sourceText(), fixture.initialDraft);
  const draft = fixture.initialDraft + '\nBROWSER LATER PRIVATE SWAP INPUT 中文😀';
  await waitDraft(draft, async () => { await editor.press('ControlOrMeta+a'); await page.keyboard.insertText(draft); }); await editor.evaluate(element => { window.__placementOriginalEditor = element; });
  await page.getByRole('button', { name: '同步', exact: true }).click(); const panel = page.getByRole('dialog', { name: '资料库同步' }); await panel.waitFor(); await panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor();
  const rows = panel.locator('.sync-object'); for (let i = 0; i < await rows.count(); i++) await rows.nth(i).locator('input[type=checkbox]').uncheck();
  for (const name of fixture.selectedNames) await rows.filter({ hasText: fixture.keys[name].id }).locator('input[type=checkbox]').check();
  await page.route('**/api/operations/prepare', async intercepted => {
    try { const body = intercepted.request().postDataJSON(); assert.deepEqual((await pending()).request, body); const response = await intercepted.fetch(); preview = await response.json(); assert.equal(response.status(), 200, JSON.stringify(preview)); pendingRequest = body; await intercepted.fulfill({ response }); }
    catch (error) { result.routeErrors.push(String(error.stack ?? error)); await intercepted.abort('failed'); }
  });
  await panel.getByRole('button', { name: '预览应用所选对象', exact: true }).click(); await panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor(); assert.ok(preview && pendingRequest); assert.equal(preview.kind, 'applySync'); assert.equal(preview.status, 'prepared');
  assert.equal(await readFile(path.join(fixture.root, 'Left', 'Left.typ'), 'utf8'), fixture.baseSource); assert.equal(await sourceText(), draft);
  const committed = page.waitForResponse(response => response.url().endsWith('/api/operations/commit'));
  await panel.getByRole('button', { name: '确认应用此预览', exact: true }).click(); const response = await committed, receipt = await response.json(); assert.equal(response.status(), 200, JSON.stringify(receipt)); assert.equal(receipt.status, 'committed'); assert.equal(receipt.planId, preview.planId);
  await panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor(); assert.equal(await sourceText(), draft); assert.equal(await editor.evaluate(element => element === window.__placementOriginalEditor), true);
  assert.equal(await readFile(fixture.afterPath, 'utf8'), fixture.expectedSource);
  const original = await api('/api/sync/operation', { operationId: pendingRequest.operationId }); assert.deepEqual(original.request, pendingRequest); assert.equal(original.plan.planId, preview.planId); assert.equal(original.plan.status, 'committed');
  await command({ action: 'verify', draft }); await until(() => ({ verified: marker('WEFTEXT_PLACEMENT_VERIFIED '), exited: bridgeExited() }), value => value.verified || value.exited, 'original FI/resources/history/private facts'); assert.ok(marker('WEFTEXT_PLACEMENT_VERIFIED '), stderr);
  await panel.getByRole('button', { name: '结束已确认操作', exact: true }).click(); await panel.getByRole('button', { name: '关闭同步', exact: true }).click();
  // A physical shifted Z has key="Z". Playwright's lowercase Shift+z emits
  // key="z", which can resolve to Undo while another Undo entry remains.
  await waitDraft(fixture.initialDraft, async () => { await editor.click(); await editor.press('ControlOrMeta+z'); }); await waitDraft(draft, () => editor.press('ControlOrMeta+Shift+Z'));
  await page.getByRole('button', { name: '比较磁盘内容', exact: true }).click(); const comparison = page.getByRole('dialog').filter({ has: page.getByRole('heading', { name: '比较草稿与磁盘内容', exact: true }) }); await comparison.waitFor();
  assert.equal(await comparison.getByLabel('草稿原始版本', { exact: true }).inputValue(), fixture.baseSource); assert.equal(await comparison.getByLabel('当前磁盘内容', { exact: true }).inputValue(), fixture.expectedSource); assert.equal(await comparison.getByLabel('合并后的草稿（可编辑）', { exact: true }).inputValue(), draft);
  await comparison.screenshot({ path: path.join(out, 'exact-swap-comparison.png') });
  const merged = fixture.expectedSource + '\nEXPLICIT MERGED PRIVATE SWAP DRAFT 中文😀\n'; await comparison.getByLabel('合并后的草稿（可编辑）', { exact: true }).fill(merged);
  const rendered = page.waitForResponse(value => value.url().endsWith('/api/render') && value.request().postDataJSON()?.source === merged && value.ok()); await comparison.getByRole('button', { name: '应用合并后的草稿', exact: true }).click(); const freshRender = await rendered; assert.equal((await freshRender.json()).ok, true); assert.notEqual(freshRender.request().postDataJSON().sessionId, initial.sessionId);
  assert.equal(await sourceText(), merged); assert.equal(await editor.evaluate(element => element === window.__placementOriginalEditor), true); assert.equal(await readFile(fixture.afterPath, 'utf8'), fixture.expectedSource);
  await waitDraft(draft, () => editor.press('ControlOrMeta+z')); await waitDraft(merged, () => editor.press('ControlOrMeta+Shift+Z'));
  const redoKeys = await page.evaluate(() => window.__placementKeyEvents.filter(event => event.phase === 'capture' && event.type === 'keydown' && event.code === 'KeyZ' && event.shift && (event.ctrl || event.meta)));
  assert.equal(redoKeys.length, 2); assert.ok(redoKeys.every(event => event.key === 'Z' && event.originalEditor));
  const fresh = await api('/api/data/open', { root: fixture.root, entityKey: `weftext-node://${fixture.identity.workspaceId}/${fixture.nodeId}` }); assert.equal(path.resolve(fresh.path), path.resolve(fixture.afterPath)); assert.equal(fresh.source, fixture.expectedSource); assert.equal(fresh.recoveredDraft, merged);
  assert.equal(requests.filter(value => value.route === '/api/operations/prepare').length, 1); assert.equal(requests.filter(value => value.route === '/api/operations/commit').length, 1);
  assert.deepEqual(result.pageErrors, []); assert.deepEqual(result.routeErrors, []); await page.screenshot({ path: path.join(out, 'swap-merged-draft.png'), fullPage: true }); result.completed = true;
} catch (error) { result.failure = String(error.stack ?? error); if (page && !page.isClosed()) await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {}); throw error; }
finally {
  if (page && !page.isClosed()) result.keyboard = await page.evaluate(() => window.__placementKeyEvents ?? []).catch(() => []);
  await context?.close(); await browser?.close();
  if (!bridgeExited()) { await command({ action: 'stop' }).catch(() => {}); bridge.stdin.end(); await until(bridgeExited, Boolean, 'owned fixture cleanup', 45000); }
  result.fixtureExitCode = bridge.exitCode; result.ownedProcessesClosed = bridge.exitCode === 0 && !browser?.isConnected();
  await writeFile(path.join(out, 'fixture.stdout.log'), stdout); await writeFile(path.join(out, 'fixture.stderr.log'), stderr); await writeFile(path.join(out, 'results.json'), JSON.stringify({ ...result, fixture: fixture ? { ...fixture, token: undefined } : null, pendingRequest, preview }, null, 2));
  console.log(JSON.stringify({ out, completed: result.completed, ownedProcessesClosed: result.ownedProcessesClosed }));
  if (bridge.exitCode !== 0) throw new Error(`Fixture cleanup failed: ${stderr}`);
}
