// Actual semantic-review UI, original pending operation, owned replicas and headless Chrome.
import { createRequire } from 'node:module';
import { spawn, execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile, appendFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import assert from 'node:assert/strict';

if (!process.argv[2] || !process.argv[3]) throw new Error('Usage: node test_sync_revalidation_browser.mjs EXE NEW_OUT [--only=1,2,3,4,5]');
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]);
const only = process.argv.slice(4).find(value => value.startsWith('--only='));
const selected = only ? only.slice(7).split(',').map(Number) : [1, 2, 3, 4, 5];
assert.ok(selected.length && new Set(selected).size === selected.length && selected.every(id => Number.isInteger(id) && id >= 1 && id <= 5));
await mkdir(out, { recursive: false });
const sha = value => createHash('sha256').update(value).digest('hex');
const result = { exe, sha256: sha(await readFile(exe)), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(), selectedCases: selected, cases: [], pageErrors: [], routeErrors: [], completed: false };
const servers = [];
let browser, active;
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
async function until(read, predicate, label, milliseconds = 30000) {
  const end = Date.now() + milliseconds;
  let value;
  while (Date.now() < end) { value = await read(); if (predicate(value)) return value; await delay(60); }
  throw new Error(`${label} timeout; last=${JSON.stringify(value)}`);
}
async function start(s, label, args) {
  const child = spawn(exe, ['--no-browser', ...args], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  const server = { child, label, folder: s.folder, stdout: '', stderr: '' }; servers.push(server);
  child.stdout.on('data', value => server.stdout += value); child.stderr.on('data', value => server.stderr += value);
  await until(() => server.stdout, value => /http:\/\/127\.0\.0\.1:\d+\/#token=\w+/.test(value), `${label} startup`);
  const match = server.stdout.match(/(http:\/\/127\.0\.0\.1:\d+)\/#token=(\w+)/);
  return Object.assign(server, { base: match[1], token: match[2], port: new URL(match[1]).port });
}
async function stop(server) {
  if (!server || server.child.exitCode !== null || server.child.signalCode !== null) return;
  server.child.kill();
  await until(() => server.child.exitCode !== null || server.child.signalCode !== null, Boolean, `${server.label} owned process close`, 15000);
}
async function api(s, server, route, request, expected = 200) {
  const response = await fetch(server.base + route, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': server.token }, body: JSON.stringify(request), signal: AbortSignal.timeout(45000) });
  const value = await response.json();
  await appendFile(path.join(s.folder, 'http.ndjson'), JSON.stringify({ program: server.label, route, request, status: response.status, response: value }) + '\n');
  assert.equal(response.status, expected, `${route}: ${JSON.stringify(value)}`); return value;
}
async function commit(s, server, plan) { const receipt = await api(s, server, '/api/operations/commit', { operationId: plan.operationId, planId: plan.planId }); assert.equal(receipt.status, 'committed'); return receipt; }
const sourceText = async editor => (await editor.locator('.cm-line').allTextContents()).join('\n');
const pending = page => page.evaluate(() => JSON.parse(localStorage.getItem('weftext-sync-operation-v1') || 'null'));
async function physical(location) {
  const value = await stat(location, { bigint: true }); assert.notEqual(value.ino, 0n);
  return { dev: String(value.dev), ino: String(value.ino), birthtimeNs: String(value.birthtimeNs) };
}
async function editor(s) { await s.page.getByRole('button', { name: '源码', exact: true }).click(); s.editor = s.page.locator('.cm-content[contenteditable=true]'); await s.editor.waitFor(); }
async function done(panel, name) { await panel.getByRole('button', { name, exact: true }).click(); await panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor(); }
async function sync(s) { await s.page.getByRole('button', { name: '同步', exact: true }).click(); const panel = s.page.getByRole('dialog', { name: '资料库同步' }); await panel.waitFor(); await panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor(); return panel; }
async function transfer(s, server, root, receiver, objects) {
  s.transfer++; const directory = path.join(s.folder, `exchange-${s.transfer}`);
  await api(s, server, '/api/sync/export', { root, directory, objects });
  return api(s, receiver, '/api/sync/receive', { identity: s.identity, directory });
}
async function saveSchema(s, server, root, alias) {
  const before = await api(s, server, '/api/data/schema', { root }), value = JSON.parse(before.source); value.aliases[alias] = 'people/person';
  const source = JSON.stringify(value, null, 2) + '\n';
  const plan = await api(s, server, '/api/data/schema/prepare', { root, operationId: randomUUID(), source, beforeSource: before.source, beforeRevision: before.revision });
  await api(s, server, '/api/data/schema/install', { operationId: plan.operationId, planId: plan.planId }); return source;
}
async function setup(id) {
  const folder = path.join(out, `c${id}`); await mkdir(folder);
  const s = { id, folder, ra: path.join(folder, 'a'), rb: path.join(folder, 'b'), da: path.join(folder, 'sa'), db: path.join(folder, 'sb'), transfer: 0, requests: [], responses: [], observations: [] }; active = s;
  s.a = await start(s, 'a', ['--data-dir', s.da]); s.b = await start(s, 'b-initial', ['--data-dir', s.db]);
  await api(s, s.a, '/api/library/create', { path: s.ra, operationId: randomUUID() }); await saveSchema(s, s.a, s.ra, 'semantic_baseline_title');
  const note = await api(s, s.a, '/api/note/create', { root: s.ra, parent: s.ra, name: 'Note', operationId: randomUUID() }); s.nodeId = note.nodeId; s.key = { kind: 'node', id: note.nodeId };
  const binding = await api(s, s.a, '/api/sync/enable', { root: s.ra, operationId: randomUUID() }); s.identity = binding.identity;
  await transfer(s, s.a, s.ra, s.b, binding.installedObjects);
  await commit(s, s.b, await api(s, s.b, '/api/operations/prepare', { root: s.rb, operationId: randomUUID(), intent: { kind: 'cloneSync', identity: s.identity, objects: binding.installedObjects } }));
  let base = await api(s, s.b, '/api/open', { root: s.rb, path: path.join(s.rb, 'Note', 'Note.typ') });
  await api(s, s.b, '/api/save', { sessionId: base.sessionId, baseId: base.baseId, source: base.source + '\nB retained common history 中文😀\n', operationId: randomUUID() });
  await transfer(s, s.b, s.rb, s.a, [s.key]); await commit(s, s.a, await api(s, s.a, '/api/operations/prepare', { root: s.ra, operationId: randomUUID(), intent: { kind: 'applySync', identity: s.identity, objects: [s.key] } }));
  const sender = await api(s, s.a, '/api/open', { root: s.ra, path: path.join(s.ra, 'Note', 'Note.typ') });
  if (sender.recoveredDraftConflict) { assert.equal(sender.recoveredDraft, sender.recoveredDraftBaseSource); await api(s, s.a, '/api/draft/resolve', { sessionId: sender.sessionId, action: 'mergeRecovered', source: sender.source }); }
  base = await api(s, s.b, '/api/open', { root: s.rb, path: path.join(s.rb, 'Note', 'Note.typ') }); s.baseSource = base.source; s.draft = base.source + '\nB PRIVATE UNSAVED SEMANTIC DRAFT 🌿';
  const history = await api(s, s.b, '/api/history/list', { sessionId: base.sessionId }); s.history = {};
  for (const item of history.versions.filter(value => value.status === 'available')) s.history[item.versionId] = (await api(s, s.b, '/api/history/read', { sessionId: base.sessionId, versionId: item.versionId })).source;
  assert.ok(Object.keys(s.history).length);
  s.originalPhysical = { directory: await physical(path.join(s.rb, 'Note')), source: await physical(path.join(s.rb, 'Note', 'Note.typ')) };
  const port = s.b.port; await stop(s.b); s.b = await start(s, 'b-ui', ['--data-dir', s.db, '--port', port, '--open', path.join(s.rb, 'Note', 'Note.typ'), '--root', s.rb]);
  s.context = await browser.newContext({ viewport: { width: 1500, height: 1050 } }); s.page = await s.context.newPage(); s.page.setDefaultTimeout(30000);
  s.page.on('pageerror', error => result.pageErrors.push(`c${id}: ${String(error)}`));
  s.page.on('request', request => { const route = new URL(request.url()).pathname; if (route.startsWith('/api/')) s.requests.push({ route, body: request.postData() ? request.postDataJSON() : null }); });
  s.page.on('response', response => { const route = new URL(response.url()).pathname; if (route.startsWith('/api/')) s.responses.push({ route, status: response.status() }); });
  await s.page.goto(`${s.b.base}/#token=${s.b.token}`); await editor(s); assert.equal(await sourceText(s.editor), s.baseSource);
  const drafted = s.page.waitForResponse(response => response.url().endsWith('/api/draft') && response.request().postDataJSON()?.source === s.draft && response.ok());
  await s.editor.press('ControlOrMeta+a'); await s.page.keyboard.insertText(s.draft); await drafted;
  await s.editor.evaluate(element => window.__semanticOriginalEditor = element);
  const opened = await api(s, s.a, '/api/open', { root: s.ra, path: path.join(s.ra, 'Note', 'Note.typ') });
  const authoredSource = opened.source + '\nA exact single-head received source 😀\n';
  // Use an actual saved definition and its admitted historical Schema binding.
  if (id <= 2) {
    const defined = await api(s, s.a, '/api/query/definition/save', {
      sessionId: opened.sessionId, baseId: opened.baseId, source: authoredSource,
      sourceHash: sha(authoredSource), serial: 1, key: null,
      label: 'weftext-query-semantic', plan: { select: [{ name: 'literal', expr: '1' }] },
      operationId: randomUUID(),
    });
    s.receivedSource = defined.document.source;
  } else {
    await api(s, s.a, '/api/save', { sessionId: opened.sessionId, baseId: opened.baseId,
      source: authoredSource, operationId: randomUUID() });
    s.receivedSource = authoredSource;
  }
  s.schemaSource = await saveSchema(s, s.b, s.rb, 'semantic_current_title'); await api(s, s.b, '/api/sync/publish', { root: s.rb });
  s.exchange = path.join(folder, 'ui-exchange'); await api(s, s.a, '/api/sync/export', { root: s.ra, directory: s.exchange, objects: [s.key] });
  s.panel = await sync(s); await s.panel.getByLabel('交换目录路径（新目录或已配对目录）', { exact: true }).fill(s.exchange); await done(s.panel, '接收交换目录');
  const rows = s.panel.locator('.sync-object'); for (let i = 0; i < await rows.count(); i++) await rows.nth(i).locator('input[type=checkbox]').uncheck(); await rows.filter({ hasText: s.nodeId }).locator('input[type=checkbox]').check();
  const status = await api(s, s.b, '/api/sync/status', { identity: s.identity }); s.head = status.objects.find(item => item.object.kind === 'node' && item.object.id === s.nodeId).view.versions; assert.equal(s.head.length, 1); s.head = s.head[0];
  // This is an explicit user-requested review. Plain Save and definitions with
  // pinned Schema baselines do not imply a current-Schema Apply refusal.
  await done(s.panel, '审阅所选对象的语义'); s.reviewPanel = s.panel.locator("section[data-sync-revalidation='review']"); await s.reviewPanel.waitFor();
  const choice = s.reviewPanel.locator(`[data-sync-revalidation-choice='node:${s.nodeId}']`); await choice.selectOption(s.head.revision); assert.equal(await choice.inputValue(), s.head.revision);
  assert.equal(await s.reviewPanel.locator("[data-sync-revalidation-choice='schema:']").inputValue(), ''); s.expectedSource = s.receivedSource;
  return s;
}
async function privateDraft(s, undo = false) {
  assert.equal(await sourceText(s.editor), s.draft);
  if (undo) {
    assert.equal(await s.editor.evaluate(element => element === window.__semanticOriginalEditor), true);
    for (const [keys, source] of [['ControlOrMeta+z', s.baseSource], ['ControlOrMeta+Shift+z', s.draft]]) {
      const saved = s.page.waitForResponse(response => response.url().endsWith('/api/draft') && response.request().postDataJSON()?.source === source && response.ok());
      await s.editor.click(); await s.editor.press(keys); await until(() => sourceText(s.editor), value => value === source, 'original editor Undo/Redo'); await saved;
    }
  }
}
async function inspect(s) {
  const next = s.page.waitForResponse(response => response.url().endsWith('/api/sync/revalidation-review'));
  await s.reviewPanel.locator("[data-sync-revalidation-action='review']").click(); const response = await next; s.review = await response.json(); assert.equal(response.status(), 200, JSON.stringify(s.review));
  await s.reviewPanel.locator("[data-sync-revalidation-summary='ready']").waitFor();
  assert.equal(s.review.resolutions.length, 1); assert.deepEqual(s.review.resolutions[0].object, s.key); assert.deepEqual(s.review.resolutions[0].revisions, [s.head.revision]);
  const schema = (await api(s, s.b, '/api/sync/status', { identity: s.identity })).objects.find(item => item.object.kind === 'schema').view.versions;
  assert.equal(schema.length, 1); assert.ok(s.review.resolutions[0].reviewedDependencies.some(dep => dep.object.kind === 'schema' && dep.revisions.length === 1 && dep.revisions[0] === schema[0].revision));
  assert.equal(await readFile(path.join(s.rb, 'Note', 'Note.typ'), 'utf8'), s.baseSource); await privateDraft(s);
  await s.reviewPanel.screenshot({ path: path.join(s.folder, 'review.png') });
}
async function tap(s, route, action, { drop = false } = {}) {
  let observed, failure;
  const handler = async intercepted => {
    try {
      const request = intercepted.request().postDataJSON(), durable = await pending(s.page); assert.ok(durable);
      if (route.endsWith('/prepare')) assert.deepEqual(durable.request, request);
      else { assert.equal(durable.attempted, true); assert.equal(durable.status, 'unknown'); assert.equal(request.operationId, durable.request.operationId); assert.equal(request.planId, durable.plan.planId); }
      const started = Date.now();
      const response = await intercepted.fetch({ timeout: 90000 }), body = await response.json(); assert.equal(response.status(), 200, JSON.stringify(body));
      s.observations.push({ route, elapsedMs: Date.now() - started });
      await appendFile(path.join(s.folder, 'timings.ndjson'), JSON.stringify(s.observations.at(-1)) + '\n');
      if (drop) await intercepted.abort('failed'); else await intercepted.fulfill({ response }); observed = { request, response: body };
    } catch (error) { failure = error; result.routeErrors.push(String(error.stack ?? error)); await intercepted.abort('failed').catch(() => {}); }
  };
  await s.page.route(`**${route}`, handler);
  try { await action(); await until(() => observed ?? failure, Boolean, route, 95000); if (failure) throw failure; return observed; }
  finally { await s.page.unroute(`**${route}`, handler); }
}
async function prepare(s, drop = false) {
  const got = await tap(s, '/api/operations/prepare', () => s.reviewPanel.locator("[data-sync-revalidation-action='prepare']").click(), { drop });
  assert.equal(got.request.intent.kind, 'revalidateSync');
  for (const field of ['expectedInstalled', 'reviewedVersions', 'resolutions']) assert.deepEqual(got.request.intent[field], s.review[field]);
  assert.equal(got.response.kind, 'revalidateSync'); s.original = got;
  await s.panel.getByRole('status').filter({ hasText: drop ? '原操作记录保留' : /^操作已完成。$/ }).waitFor(); return got;
}
async function apply(s, drop = false) {
  const got = await tap(s, '/api/operations/commit', () => s.panel.getByRole('button', { name: '确认应用此预览', exact: true }).click(), { drop }); assert.equal(got.response.status, 'committed');
  await s.panel.getByRole('status').filter({ hasText: drop ? '原操作记录保留' : /^操作已完成。$/ }).waitFor(); return got;
}
async function restart(s) {
  const port = s.b.port; await stop(s.b); s.b = await start(s, 'b-restarted', ['--data-dir', s.db, '--port', port, '--open', path.join(s.rb, 'Note', 'Note.typ'), '--root', s.rb]);
  await s.page.goto(`${s.b.base}/?restart=${randomUUID()}#token=${s.b.token}`); await editor(s); assert.equal(await sourceText(s.editor), s.draft); s.panel = await sync(s);
}
async function verify(s) {
  const current = await api(s, s.b, '/api/open', { root: s.rb, path: path.join(s.rb, 'Note', 'Note.typ') }); assert.equal(current.source, s.expectedSource); assert.equal(current.recoveredDraft, s.draft); assert.equal(current.recoveredDraftConflict, true);
  for (const [versionId, source] of Object.entries(s.history)) assert.equal((await api(s, s.b, '/api/history/read', { sessionId: current.sessionId, versionId })).source, source);
  assert.equal((await api(s, s.b, '/api/sync/file', { identity: s.identity, object: s.key, revision: s.head.revision, path: 'Note.typ' })).text, s.receivedSource);
  await api(s, s.b, '/api/sync/publish', { root: s.rb });
  const view = (await api(s, s.b, '/api/sync/status', { identity: s.identity })).objects.find(item => item.object.kind === 'node' && item.object.id === s.nodeId).view.versions;
  assert.equal(view.length, 1); assert.notEqual(view[0].revision, s.head.revision); assert.deepEqual(view[0].state.fileRevisions, s.original.request.intent.resolutions[0].state.fileRevisions);
  assert.deepEqual(await physical(path.join(s.rb, 'Note')), s.originalPhysical.directory); assert.deepEqual(await physical(path.join(s.rb, 'Note', 'Note.typ')), s.originalPhysical.source); await privateDraft(s);
}
async function compareAndSave(s) {
  await done(s.panel, '结束已确认操作'); await s.panel.getByRole('button', { name: '关闭同步', exact: true }).click(); await privateDraft(s, true);
  await s.page.getByRole('button', { name: '比较磁盘内容', exact: true }).click();
  const comparison = s.page.getByRole('dialog').filter({ has: s.page.getByRole('heading', { name: '比较草稿与磁盘内容', exact: true }) }); await comparison.waitFor();
  assert.equal(await comparison.getByLabel('草稿原始版本', { exact: true }).inputValue(), s.baseSource); assert.equal(await comparison.getByLabel('当前磁盘内容', { exact: true }).inputValue(), s.expectedSource); assert.equal(await comparison.getByLabel('合并后的草稿（可编辑）', { exact: true }).inputValue(), s.draft);
  await comparison.screenshot({ path: path.join(s.folder, 'exact-comparison.png') }); await comparison.getByRole('button', { name: '关闭并保留草稿', exact: true }).click(); await privateDraft(s);
  await s.page.getByRole('button', { name: '比较磁盘内容', exact: true }).click();
  const merged = s.expectedSource + s.draft.slice(s.baseSource.length) + '\n明确合并后继续保存\n';
  await comparison.getByLabel('合并后的草稿（可编辑）', { exact: true }).fill(merged);
  const rendered = s.page.waitForResponse(response => response.url().endsWith('/api/render') && response.request().postDataJSON()?.source === merged && response.ok());
  await comparison.getByRole('button', { name: '应用合并后的草稿', exact: true }).click(); assert.equal((await (await rendered).json()).ok, true);
  const saved = s.page.waitForResponse(response => response.url().endsWith('/api/save') && response.request().postDataJSON()?.source === merged && response.ok()); await s.page.getByRole('button', { name: '保存', exact: true }).click(); await saved;
  assert.equal(await sourceText(s.editor), merged); s.finalSource = merged; await s.page.screenshot({ path: path.join(s.folder, 'merged-current-preview.png'), fullPage: true });
}
const cases = [
  [1, 'Single head with changed Schema: exact read-only review, original Plan, preserved file revision, Draft/Undo, explicit comparison/merge and Save', async s => { await inspect(s); await prepare(s); await apply(s); await verify(s); await compareAndSave(s); }],
  [2, 'Explicit source repair edits only the selected received file; review/prepare preserves main Draft/Undo and original retained bytes', async s => {
    const details = s.reviewPanel.locator(`[data-sync-revalidation-object='node:${s.nodeId}']`);
    if (await details.getAttribute('open') === null) await details.locator(':scope > summary').click();
    const file = details.locator('details').filter({ hasText: '读取原文件以检查或修复 Note.typ' }).first();
    if (await file.getAttribute('open') === null) await file.locator(':scope > summary').click();
    await details.getByRole('button', { name: '读取原文件以检查或修复 Note.typ', exact: true }).click(); const text = details.locator("[data-sync-revalidation-repair='Note.typ']"); await text.waitFor(); assert.equal(await text.inputValue(), s.receivedSource);
    s.expectedSource = s.receivedSource + `\n#link("weftext-node://${s.identity.workspaceId}/${randomUUID()}")[ordinary unresolved link]\n明确语义修复😀\n`; await text.fill(s.expectedSource);
    assert.equal(await s.reviewPanel.locator("[data-sync-revalidation-action='review']").isDisabled(), true); assert.equal(await s.reviewPanel.locator("[data-sync-revalidation-action='prepare']").isDisabled(), true); await privateDraft(s);
    await details.locator("[data-sync-revalidation-action='repair']").click(); await inspect(s); assert.equal(s.review.resolutions[0].decision.kind, 'repair'); assert.notEqual(s.review.resolutions[0].state.fileRevisions['Note.typ'], s.head.state.fileRevisions['Note.typ']); await prepare(s); await apply(s); await verify(s); await compareAndSave(s);
  }],
  [3, 'A real later received head invalidates the prior semantic review; no stale Plan or author effect, original Draft and Undo remain', async s => {
    await inspect(s); const opened = await api(s, s.a, '/api/open', { root: s.ra, path: path.join(s.ra, 'Note', 'Note.typ') }); await api(s, s.a, '/api/save', { sessionId: opened.sessionId, baseId: opened.baseId, source: opened.source + '\nA late exact head after review\n', operationId: randomUUID() });
    await api(s, s.a, '/api/sync/export', { root: s.ra, directory: s.exchange, objects: [s.key] });
    assert.equal(await s.panel.locator("[data-sync-revalidation-summary='ready']").count(), 1);
    await done(s.panel, '接收交换目录');
    assert.equal(await s.panel.locator("[data-sync-revalidation-summary='ready']").count(), 0); assert.equal(s.requests.filter(request => request.route === '/api/operations/prepare').length, 0); assert.equal(s.requests.filter(request => request.route === '/api/operations/commit').length, 0); assert.equal(await readFile(path.join(s.rb, 'Note', 'Note.typ'), 'utf8'), s.baseSource); await s.panel.getByRole('button', { name: '关闭同步', exact: true }).click(); await privateDraft(s, true);
  }],
  [4, 'Actual prepare response is lost: durable exact revalidation request survives restart and retries the identical original Plan', async s => {
    await inspect(s); const prepared = await prepare(s, true); assert.deepEqual((await pending(s.page)).request, prepared.request); assert.equal((await pending(s.page)).plan, undefined); await restart(s);
    const retried = await tap(s, '/api/operations/prepare', () => s.panel.getByRole('button', { name: '重试原准备请求', exact: true }).click()); assert.deepEqual(retried.request, prepared.request); assert.deepEqual(retried.response, prepared.response); await s.panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor(); await apply(s); await verify(s); assert.equal(s.requests.filter(request => request.route === '/api/operations/commit').length, 1);
  }],
  [5, 'Actual commit response is lost: restart checks the original operation and exact committed receipt without a new prepare or commit', async s => {
    await inspect(s); const prepared = await prepare(s); await apply(s, true); const installed = await physical(path.join(s.rb, 'Note', 'Note.typ')); await restart(s); await done(s.panel, '检查原操作结果'); await verify(s);
    const retained = await api(s, s.b, '/api/sync/operation', { operationId: prepared.request.operationId }); assert.deepEqual(retained.request, prepared.request); assert.equal(retained.plan.planId, prepared.response.planId); assert.equal(retained.plan.status, 'committed'); assert.deepEqual(await physical(path.join(s.rb, 'Note', 'Note.typ')), installed); assert.equal(s.requests.filter(request => request.route === '/api/operations/prepare').length, 1); assert.equal(s.requests.filter(request => request.route === '/api/operations/commit').length, 1);
  }],
];
try {
  browser = await chromium.launch({ channel: 'chrome', headless: true }); result.browser = browser.version();
  for (const [id, description, action] of cases) if (selected.includes(id)) {
    const s = await setup(id);
    try { await action(s); await s.page.screenshot({ path: path.join(s.folder, 'completed.png'), fullPage: true }); }
    catch (error) { await s.page.screenshot({ path: path.join(s.folder, 'failure.png'), fullPage: true }).catch(() => {}); throw error; }
    finally { await s.context.close(); await stop(s.a); await stop(s.b); await writeFile(path.join(s.folder, 'browser-requests.json'), JSON.stringify(s.requests, null, 2)); await writeFile(path.join(s.folder, 'browser-responses.json'), JSON.stringify(s.responses, null, 2)); await writeFile(path.join(s.folder, 'fixture.json'), JSON.stringify({ identity: s.identity, head: s.head, review: s.review, original: s.original, originalPhysical: s.originalPhysical, baseSource: s.baseSource, draft: s.draft, receivedSource: s.receivedSource, expectedSource: s.expectedSource, finalSource: s.finalSource }, null, 2)); }
    assert.equal(await readFile(path.join(s.rb, 'Note', 'Note.typ'), 'utf8'), id === 3 ? s.baseSource : s.finalSource ?? s.expectedSource); result.cases.push({ id, description, status: 'passed' });
  }
  assert.deepEqual(result.pageErrors, []); assert.deepEqual(result.routeErrors, []); assert.ok(servers.every(server => server.stderr === ''), 'Owned server stderr must be empty'); result.completed = true;
} catch (error) { result.failure = String(error.stack ?? error); result.failedCase = active?.id; if (active?.page && !active.page.isClosed()) await active.page.screenshot({ path: path.join(active.folder, 'failure.png'), fullPage: true }).catch(() => {}); throw error; }
finally {
  await browser?.close(); for (const server of servers) await stop(server);
  result.ownedProcessesClosed = servers.every(server => server.child.exitCode !== null || server.child.signalCode !== null) && !browser?.isConnected(); result.serverStderrBytes = servers.reduce((sum, server) => sum + Buffer.byteLength(server.stderr), 0);
  for (const [index, server] of servers.entries()) { await writeFile(path.join(server.folder, `server-${index}-${server.label}.stdout.log`), server.stdout); await writeFile(path.join(server.folder, `server-${index}-${server.label}.stderr.log`), server.stderr); }
  await writeFile(path.join(out, 'results.json'), JSON.stringify(result, null, 2)); console.log(JSON.stringify({ out, completed: result.completed, cases: result.cases, pageErrors: result.pageErrors, routeErrors: result.routeErrors, ownedProcessesClosed: result.ownedProcessesClosed }));
}
