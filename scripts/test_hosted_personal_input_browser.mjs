import { workspaceTab, workspaceCommand, workspaceMode } from './_workspace_browser.mjs';
import { spawnServe, serveReady, stopServe, stringifyServeJson } from './_cli_serve.mjs';
import { createdDocument } from './_creation_result.mjs';
import { personalInputSnapshot, abortPersonalInputWrites } from './_personal_input_browser.mjs';
// Real embedded hosted UI regression. Pass EXE and a new owned evidence directory.
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash, randomBytes, randomUUID } from 'node:crypto';
import path from 'node:path';
import assert from 'node:assert/strict';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]); await mkdir(out, { recursive: false });
const sha = value => createHash('sha256').update(value).digest('hex'), pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const result = { exe, exeSha256: sha(await readFile(exe)), harnessSha256: sha(await readFile(new URL(import.meta.url))), personalInputHelperSha256: sha(await readFile(new URL('./_personal_input_browser.mjs', import.meta.url))), workspaceBrowserHelperSha256: sha(await readFile(new URL('./_workspace_browser.mjs', import.meta.url))), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(), completed: false, cases: [], pageErrors: [], processes: [] };
const tokens = Object.fromEntries(['alice', 'bob', 'admin'].map(actor => [actor, randomBytes(32).toString('hex')]));
let base, localToken, local, server, browser;
const pendingReleases = [];
async function bounded(promise, label) { let timer; try { return await Promise.race([promise, new Promise((_, reject) => { timer = setTimeout(() => reject(Error(label + ' timeout')), 30000); })]); } finally { clearTimeout(timer); } }
async function start(args, hosted = false) {
  const child = spawnServe(exe, args, { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] }), record = { pid: child.pid, args: child.spawnargs.slice(1), stdout: '', stderr: '' }; result.processes.push(record);
  child.stdout.on('data', bytes => record.stdout += bytes.toString()); child.stderr.on('data', bytes => record.stderr += bytes.toString()); child.once('exit', (code, signal) => { record.exit = { code, signal }; });
  for (let i = 0; i < 300; i++) { const match = serveReady(child); if (match) { base = match.base; localToken = match.token; record.base = base; return child; } if (child.exitCode !== null) throw new Error(record.stderr); await pause(100); }
  await stop(child); throw new Error('Owned test backend startup timed out');
}
async function stop(child) {
  if (!child) return;
  const record = result.processes.find(record => record.pid === child.pid);
  record.ownerStop = await stopServe(child);
  assert.equal(child.exitCode, 0);
}
async function api(route, data, actor = 'alice') {
  const response = await fetch(base + route, { method: data === undefined ? 'GET' : 'POST', headers: { 'Content-Type': 'application/json', ...(actor ? { Authorization: 'Bearer ' + tokens[actor] } : { 'X-Weftext-Token': localToken }) }, body: data === undefined ? undefined : JSON.stringify(data) });
  const body = await response.json(); assert.ok(response.ok, JSON.stringify(body)); return body;
}
async function policy(edit) { const current = await api('/api/hosted/admin/policy', undefined, 'admin'); edit(current.users); return api('/api/hosted/admin/policy', { expectedGeneration: current.generation, users: current.users }, 'admin'); }
async function login(page, actor, navigate = true) {
  if (navigate) await page.goto(base); if (await page.locator('#workspace').isVisible()) await workspaceTab(page, 'file'); if (await page.locator('#logout').isVisible()) await page.locator('#logout').click();
  await page.getByLabel('访问凭据', { exact: true }).fill(tokens[actor]); await page.getByRole('button', { name: '连接', exact: true }).click();
  await page.getByRole('button', { name: 'TEST共同文档', exact: true }).click(); await workspaceMode(page, 'V01 源码'); await page.locator('#source .cm-content').waitFor(); await workspaceCommand(page, 'knowledge', '知识链接'); await page.locator('#stable-link:not([hidden])').waitFor();
}
async function freshPage() {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } }), page = await context.newPage();
  page.on('pageerror', error => result.pageErrors.push(String(error))); page.on('dialog', dialog => dialog.accept());
  await page.route('**/api/hosted/draft', route => route.abort('failed')); await login(page, 'alice'); return { page, context };
}
async function source(page) { return (await page.locator('#source .cm-line').allTextContents()).join('\n'); }
async function append(page, text) { const editor = page.locator('#source .cm-content'); await editor.click(); await editor.press('ControlOrMeta+End'); await page.keyboard.insertText(text); }
async function ownStorage(page) { const snapshot = await personalInputSnapshot(page); return { audiences: JSON.stringify(snapshot.audiences), texts: JSON.stringify(snapshot.texts) }; }
async function assertOnlyOwn(page, marker, secret) { const storage = await ownStorage(page), values = Object.values(storage).join('\n'); if (!(values.includes(marker) || values.includes(JSON.stringify(marker).slice(1, -1)))) { result.inputFailure = { storage, source: await source(page), status: await page.locator('#status').textContent() }; } assert.ok(values.includes(marker) || values.includes(JSON.stringify(marker).slice(1, -1)), 'new input must be retained'); assert.ok(!values.includes(secret), 'protected Base must never enter personal input storage'); for (const token of Object.values(tokens)) assert.ok(!values.includes(token), 'credentials must never enter personal input storage'); }
async function revoke(page, disabled = false) {
  await policy(users => { if (disabled) users.alice.enabled = false; else users.alice.grants = {}; }); await page.locator('#refresh').click();
  await page.waitForFunction(() => !document.querySelector('#source .cm-content'));
  assert.deepEqual(await page.evaluate(() => Object.keys(sessionStorage).filter(key => key.startsWith('weftext-hosted-draft:') || key.startsWith('weftext-hosted-draft-text:'))), []);
}
async function reauthorize(page) { await policy(users => { users.alice.enabled = true; users.alice.grants = { shared: { read: true, write: true, history: true, download: true } }; }); await login(page, 'alice', false); }
async function recoveryRow(page, marker) {
  await workspaceCommand(page, 'home', '取回本人未上传输入');
  const rows = page.locator('#personal-inputs > section');
  for (let i = 0; i < await rows.count(); i++) if ((await rows.nth(i).locator('textarea').inputValue()).includes(marker)) return rows.nth(i);
  throw new Error('Expected pure personal input record was not shown');
}
try {
  const data = path.join(out, 'state'), root = path.join(out, 'TEST-library');
  local = await start(['--data-dir', data]);
  await api('/api/library/create', { path: root, operationId: randomUUID() }, null);
  const note = createdDocument(await api('/api/note/create', { root, parent: root, name: 'TEST-Shared', operationId: randomUUID() }, null));
  const secret = 'PRIVATE_PROTECTED_BASE_NOT_PERSONAL', initial = note.source + '\n' + secret + '\n';
  await api('/api/save', { sessionId: note.sessionId, baseId: note.baseId, source: initial, operationId: randomUUID() }, null); await stop(local); local = null;
  const rights = { read: true, write: true, history: true, download: true };
  const config = { deployment: 'TEST-personal-input', bind: '127.0.0.1:0', root, adminTokenSha256: sha(tokens.admin), files: { shared: { path: 'TEST-Shared/TEST-Shared.typ', title: 'TEST共同文档', editable: true } }, users: Object.fromEntries(['alice', 'bob'].map(actor => [actor, { enabled: true, tokenSha256: sha(tokens[actor]), grants: { shared: rights } }])) };
  const configPath = path.join(out, 'server.json'); await writeFile(configPath, JSON.stringify(config, null, 2)); server = await start(['--data-dir', data, '--server-config', configPath], true);
  browser = await chromium.launch({ channel: 'chrome', headless: true }); result.browser = browser.version();
  {
    const { page, context } = await freshPage(), marker = 'OWN_UNUPLOADED_中文😀'; await append(page, marker); await pause(900); await assertOnlyOwn(page, marker, secret);
    await revoke(page); await assertOnlyOwn(page, marker, secret); await reauthorize(page); assert.equal(await source(page), initial);
    const row = await recoveryRow(page, marker); await row.getByRole('button', { name: '核对原基线并恢复为草稿', exact: true }).click();
    await page.waitForFunction(marker => document.querySelector('#source')?.textContent.includes(marker), marker); assert.equal(await source(page), initial + marker);
    await page.locator('#source .cm-content').click(); await page.locator('#source .cm-content').press('ControlOrMeta+z'); assert.equal(await source(page), initial);
    await page.locator('#source .cm-content').press('ControlOrMeta+Shift+KeyZ'); assert.equal(await source(page), initial + marker);
    assert.equal(await readFile(path.join(root, 'TEST-Shared', 'TEST-Shared.typ'), 'utf8'), initial); await assertOnlyOwn(page, marker, secret);
    await page.screenshot({ path: path.join(out, 'plain-restored-draft.png'), fullPage: true }); result.cases.push('actual draft-network failure + policy revocation + fresh reauthentication retains only author input and restores exact draft with CM Undo/Redo, without saving'); await context.close();
  }
  {
    const { page, context } = await freshPage(), marker = 'OWN_401_AND_ACTOR'; await append(page, marker); await revoke(page, true); await page.locator('#login:not([hidden])').waitFor();
    await login(page, 'bob'); await workspaceTab(page, 'home'); assert.equal(await page.getByRole('button', { name: '取回本人未上传输入', exact: true }).isVisible(), false); assert.equal(await page.locator('#personal-inputs textarea').count(), 0);
    await reauthorize(page); await recoveryRow(page, marker); await assertOnlyOwn(page, marker, secret); result.cases.push('real credential 401 locks personal input until fresh original actor authentication; another actor does not see it'); await context.close();
  }
  {
    const { page, context } = await freshPage(), marker = 'OWN_RICH_WAITING_😀';
    await page.locator('#source [role=status]').filter({ hasText: '预览对应当前草稿' }).waitFor({ timeout: 30000 }); await workspaceMode(page, 'V04 可视写作');
    let observed; const pending = new Promise(resolve => observed = resolve); let release; const held = new Promise(resolve => release = resolve); pendingReleases.push(release);
    await page.route('**/api/hosted/editor/edit', async route => { try { const response = await route.fetch(); observed({ status: response.status() }); await held; await route.fulfill({ response }).catch(() => {}); } catch (error) { observed({ error: String(error) }); await route.abort('failed').catch(() => {}); } });
    const literal = page.getByRole('textbox', { name: '编辑段落文字', exact: true }).last(); await literal.click(); await literal.press('End'); await page.keyboard.insertText(marker); assert.deepEqual(await bounded(pending, 'held native edit'), { status: 200 }); await assertOnlyOwn(page, marker, secret);
    await revoke(page); await assertOnlyOwn(page, marker, secret); release(); await reauthorize(page); await assertOnlyOwn(page, marker, secret); const row = await recoveryRow(page, marker); const pure = await row.locator('textarea').inputValue(); assert.ok(pure.includes(marker)); assert.ok(!pure.includes(secret));
    await row.getByRole('button', { name: '核对原基线并恢复为草稿', exact: true }).click(); await row.getByRole('status').filter({ hasText: /未能准确映射|原目标或准确基线不同/ }).waitFor(); assert.equal(await source(page), initial);
    result.cases.push('held native rich result preserves only actual input and cannot reinstall old protected source or pretend automatic mapping'); await context.close();
  }
  {
    const { page, context } = await freshPage(), marker = 'OWN_APPLIED_TITLE_中文';
    await page.locator('#source [role=status]').filter({ hasText: '预览对应当前草稿' }).waitFor({ timeout: 30000 }); await workspaceMode(page, 'V04 可视写作');
    await workspaceCommand(page, 'insert', '标题'); const dialog = page.getByRole('dialog', { name: '文档标题', exact: true });
    const titleInput = dialog.getByRole('textbox'); await titleInput.fill('A');
    await titleInput.evaluate(input => { input.value = 'A' + 'PROGRAMMATIC_SECRET'; }); await titleInput.press('End'); await page.keyboard.insertText('X');
    assert.ok(!Object.values(await ownStorage(page)).join().includes('PROGRAMMATIC_SECRET'));
    // Synthetic IME lifecycle is an ownership negative case, not OS IME acceptance.
    await titleInput.fill('A'); await titleInput.evaluate(input => { input.setSelectionRange(1, 1); input.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true, data: '' })); input.value = 'AIME_PROGRAMMATIC_SECRETZ'; input.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true, data: 'Z' })); });
    assert.ok(!Object.values(await ownStorage(page)).join().includes('IME_PROGRAMMATIC_SECRET'));
    await titleInput.fill(marker); await dialog.getByRole('button', { name: '应用', exact: true }).click(); await dialog.waitFor({ state: 'hidden' });
    await revoke(page); await reauthorize(page); const row = await recoveryRow(page, marker); assert.ok(!(await row.locator('textarea').inputValue()).includes(secret));
    await row.getByRole('button', { name: '核对原基线并恢复为草稿', exact: true }).click(); await row.getByRole('status').filter({ hasText: /未能准确映射|原目标或准确基线不同/ }).waitFor();
    result.cases.push('applied native title proposal remains independently recoverable without retaining its initial field, native source or assuming a replay map'); await context.close();
  }
  {
    const { page, context } = await freshPage(), marker = 'OWN_TRANSACTION_REFUSED';
    const original = await personalInputSnapshot(page); await abortPersonalInputWrites(page);
    await append(page, marker); await revoke(page); await reauthorize(page); const row = await recoveryRow(page, marker); assert.ok((await row.locator('textarea').inputValue()).includes(marker));
    await page.locator('#personal-inputs [role=alert]').filter({ hasText: /内存|写入/ }).waitFor();
    assert.ok(await page.evaluate(() => window.personalInputWriteAborts > 0));
    assert.deepEqual(await personalInputSnapshot(page), original);
    result.cases.push('actual IndexedDB transaction abort preserves the original committed snapshot and latest pure input in memory across authorization clearing, with explicit unpersisted status'); await context.close();
  }
  {
    const { page, context } = await freshPage(), marker = 'OWN_PENDING_ORIGINAL'; await append(page, marker);
    let originalOperation, writes = 0; await page.route('**/api/hosted/save', async route => { writes++; originalOperation = route.request().postDataJSON().operationId; const response = await route.fetch(); assert.equal(response.status(), 200); await route.abort('failed'); });
    await page.locator('#save').click(); await page.locator('#save').waitFor({ state: 'visible' }); await page.waitForFunction(() => document.querySelector('#status').textContent.length > 0); await pause(200);
    await revoke(page); await reauthorize(page); await workspaceCommand(page, 'home', '取回本人未上传输入');
    const row = page.locator('#personal-inputs > section').filter({ hasText: originalOperation }); await row.getByRole('button', { name: '查询并核定原保存操作', exact: true }).click();
    await page.locator('#personal-inputs > section').filter({ hasText: originalOperation }).filter({ hasText: 'committed' }).waitFor(); assert.equal(writes, 1); assert.ok((await source(page)).includes(marker));
    result.originalOperation = originalOperation; result.cases.push('lost committed save response retains and queries the same operation ID after revocation; no replacement save request is sent'); await context.close();
  }
  assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) { result.failure = String(error.stack ?? error); throw error; }
finally {
  for (const release of pendingReleases) release();
  await browser?.close(); await stop(local); await stop(server); for (const process of result.processes) process.stdout = process.stdout.replace(/#token=\w+/g, '#token=[TEST credential redacted]');
  result.closeMethod = 'Owned browser.close, exact typed owner-stop receipts and actual original owner exits zero';
  await writeFile(path.join(out, 'result.json'), stringifyServeJson(result, 2)); console.log(JSON.stringify({ out, completed: result.completed, cases: result.cases, failure: result.failure }));
}
