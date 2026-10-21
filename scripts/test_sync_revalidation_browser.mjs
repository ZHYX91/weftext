import { createdDocument } from './_creation_result.mjs';
// Actual semantic-review UI, original pending operation, owned replicas and headless Chrome.
import { createRequire } from 'node:module';
import { spawn, execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile, appendFile, stat, rename, readdir } from 'node:fs/promises';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import assert from 'node:assert/strict';

if (!process.argv[2] || !process.argv[3]) throw new Error('Usage: node test_sync_revalidation_browser.mjs EXE NEW_OUT [--only=1,2,3,4,5,6,7,8,9,10,11,12]');
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]);
const only = process.argv.slice(4).find(value => value.startsWith('--only='));
const selected = only ? only.slice(7).split(',').map(Number) : [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
assert.ok(selected.length && new Set(selected).size === selected.length && selected.every(id => Number.isInteger(id) && id >= 1 && id <= 12));
await mkdir(out, { recursive: false });
const sha = value => createHash('sha256').update(value).digest('hex');
const nativeGood = '#let generated = [精确标题]\n#title(generated)\n#if false [= 不存在标题]\n#heading(level:12)[实际深标题]\n#for n in (1,2) [循环#n]\n';
const nativeProvider = "nextCivilDay(title == '精确标题' && body.contains('循环2') && !body.contains('不存在标题') && headings.exists(h, h.level == 12 && h.generated) ? date('2026-01-01') : date('9999-12-31'))";
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
  const nativeCall = route === '/api/sync/revalidation-review' || route === '/api/query/run' || route === '/api/query/definition/run' || route === '/api/operations/prepare' && ['revalidateSync', 'adoptSyncExternal'].includes(request.intent?.kind);
  const response = await fetch(server.base + route, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': server.token }, body: JSON.stringify(request), signal: AbortSignal.timeout(nativeCall ? 90000 : 45000) });
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
async function exactTree(root) {
  const values = { '.': { kind: 'directory', physical: await physical(root) } };
  async function visit(directory) {
    for (const entry of (await readdir(directory, { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name))) {
      const location = path.join(directory, entry.name), relative = path.relative(root, location);
      assert.equal(entry.isSymbolicLink(), false, 'Owned fixtures contain no symbolic links');
      values[relative] = { kind: entry.isDirectory() ? 'directory' : 'file', physical: await physical(location) };
      if (entry.isDirectory()) await visit(location); else values[relative].sha256 = sha(await readFile(location));
    }
  }
  await visit(root); return values;
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
  const note = createdDocument(await api(s, s.a, '/api/note/create', { root: s.ra, parent: s.ra, name: 'Note', operationId: randomUUID() })); s.nodeId = note.nodeId; s.key = { kind: 'node', id: note.nodeId };
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
  if (id === 11 || id === 12) return setupRemovedMember(s);
  if (id >= 6) {
    const task = createdDocument(await api(s, s.a, '/api/note/create', { root: s.ra, parent: s.ra, name: 'Task', operationId: randomUUID() }));
    s.nativeKey = { kind: 'node', id: task.nodeId };
    const body = id === 7 ? nativeGood.replace('精确标题', '错误标题') : nativeGood;
    s.nativeSource = `#metadata((id: "${task.nodeId}", types: ("task",), status: "open")) <weftext>\n${body}${id === 8 ? '\n#include "../shared.typ"\n' : ''}`;
    await api(s, s.a, '/api/save', { sessionId: task.sessionId, baseId: task.baseId, source: s.nativeSource, operationId: randomUUID() });
    await transfer(s, s.a, s.ra, s.b, [s.nativeKey]);
    await commit(s, s.b, await api(s, s.b, '/api/operations/prepare', { root: s.rb, operationId: randomUUID(), intent: { kind: 'applySync', identity: s.identity, objects: [s.nativeKey] } }));
    s.nativePhysical = { directory: await physical(path.join(s.rb, 'Task')), source: await physical(path.join(s.rb, 'Task', 'Task.typ')) };
    if (id === 8) {
      s.nativeInclude = path.join(s.rb, 'shared.typ'); s.nativeIncludeBytes = Buffer.from('原始共享输入中文😀\n');
      await writeFile(path.join(s.ra, 'shared.typ'), s.nativeIncludeBytes); await writeFile(s.nativeInclude, s.nativeIncludeBytes);
    }
  }
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
  if (id <= 2 || id >= 6) {
    const defined = await api(s, s.a, '/api/query/definition/save', {
      sessionId: opened.sessionId, baseId: opened.baseId, source: authoredSource,
      sourceHash: sha(authoredSource), serial: 1, key: null,
      label: 'weftext-query-semantic', plan: id >= 6 ? { types: ['task'], ...(id === 10 ? { scope: { lifecycle: 'trash' } } : {}), select: [{ name: 'actual', expr: id === 10 ? "nextCivilDay(title == '精确标题' ? date('2026-01-01') : date('9999-12-31'))" : nativeProvider }] } : { select: [{ name: 'literal', expr: '1' }] },
      operationId: randomUUID(),
    });
    s.receivedSource = defined.document.source;
    if (id >= 6) s.nativeDefinitionKey = defined.definition.key;
    if (id === 10) {
      const retired = await api(s, s.a, '/api/operations/prepare', { root: s.ra, operationId: randomUUID(), intent: { kind: 'trashNode', source: path.join(s.ra, 'Task') } });
      await commit(s, s.a, retired); await transfer(s, s.a, s.ra, s.b, [s.nativeKey]);
      assert.equal(await readFile(path.join(s.rb, 'Task', 'Task.typ'), 'utf8'), s.nativeSource);
      s.retireRawBytes = Buffer.from('ORIGINAL LIVE TASK RAW 中文😀'); await writeFile(path.join(s.rb, 'Task', 'retire-private.raw'), s.retireRawBytes);
      s.retireRawPhysical = await physical(path.join(s.rb, 'Task', 'retire-private.raw'));
    }
  } else {
    await api(s, s.a, '/api/save', { sessionId: opened.sessionId, baseId: opened.baseId,
      source: authoredSource, operationId: randomUUID() });
    s.receivedSource = authoredSource;
  }
  s.schemaSource = await saveSchema(s, s.b, s.rb, 'semantic_current_title'); await api(s, s.b, '/api/sync/publish', { root: s.rb });
  s.exchange = path.join(folder, 'ui-exchange'); await api(s, s.a, '/api/sync/export', { root: s.ra, directory: s.exchange, objects: [s.key] });
  s.panel = await sync(s); await s.panel.getByLabel('交换目录路径（新目录或已配对目录）', { exact: true }).fill(s.exchange); await done(s.panel, '接收交换目录');
  const rows = s.panel.locator('.sync-object'); for (let i = 0; i < await rows.count(); i++) await rows.nth(i).locator('input[type=checkbox]').uncheck(); await rows.filter({ hasText: s.nodeId }).locator('input[type=checkbox]').check();
  if (id === 10) await rows.filter({ hasText: s.nativeKey.id }).locator('input[type=checkbox]').check();
  const status = await api(s, s.b, '/api/sync/status', { identity: s.identity }); s.head = status.objects.find(item => item.object.kind === 'node' && item.object.id === s.nodeId).view.versions; assert.equal(s.head.length, 1); s.head = s.head[0];
  if (id === 10) {
    const heads = status.objects.find(item => item.object.kind === 'node' && item.object.id === s.nativeKey.id).view.versions; assert.equal(heads.length, 1); s.taskHead = heads[0]; assert.equal(s.taskHead.state.retired, true);
    const current = await api(s, s.b, '/api/sync/binding', { root: s.rb }); s.taskInstalled = current.installedVersions.find(([object]) => object.kind === 'node' && object.id === s.nativeKey.id)[1]; assert.notEqual(s.taskInstalled, s.taskHead.revision);
  }
  // This is an explicit user-requested review. Plain Save and definitions with
  // pinned Schema baselines do not imply a current-Schema Apply refusal.
  await done(s.panel, '审阅所选对象的语义'); s.reviewPanel = s.panel.locator("section[data-sync-revalidation='review']"); await s.reviewPanel.waitFor();
  const choice = s.reviewPanel.locator(`[data-sync-revalidation-choice='node:${s.nodeId}']`); await choice.selectOption(s.head.revision); assert.equal(await choice.inputValue(), s.head.revision);
  if (id === 10) await s.reviewPanel.locator(`[data-sync-revalidation-choice='node:${s.nativeKey.id}']`).selectOption(s.taskHead.revision);
  assert.equal(await s.reviewPanel.locator("[data-sync-revalidation-choice='schema:']").inputValue(), ''); s.expectedSource = s.receivedSource;
  return s;
}
async function setupRemovedMember(s) {
  // Share the real two-replica/common-history bootstrap, then register the
  // resource before moving only A's physical original out of its author tree.
  s.memberBytes = Buffer.from('original registered member bytes');
  const note = await api(s, s.a, '/api/open', { root: s.ra, path: path.join(s.ra, 'Note', 'Note.typ') });
  const imported = await api(s, s.a, '/api/resources/prepare-import', { sessionId: note.sessionId, name: 'asset.bin', bytes: s.memberBytes.toString('base64'), operationId: randomUUID() }); await commit(s, s.a, imported);
  await transfer(s, s.a, s.ra, s.b, [s.key]); await commit(s, s.b, await api(s, s.b, '/api/operations/prepare', { root: s.rb, operationId: randomUUID(), intent: { kind: 'applySync', identity: s.identity, objects: [s.key] } }));
  const receiver = await api(s, s.b, '/api/open', { root: s.rb, path: path.join(s.rb, 'Note', 'Note.typ') });
  s.receiverDraft = s.draft; s.receiverHistory = s.history;
  await api(s, s.b, '/api/draft', { sessionId: receiver.sessionId, baseId: receiver.baseId, source: s.receiverDraft });
  const owner = await api(s, s.a, '/api/open', { root: s.ra, path: path.join(s.ra, 'Note', 'Note.typ') }); assert.equal(owner.source, s.baseSource);
  const timeline = await api(s, s.a, '/api/history/list', { sessionId: owner.sessionId }); s.history = {};
  for (const version of timeline.versions.filter(value => value.status === 'available')) s.history[version.versionId] = (await api(s, s.a, '/api/history/read', { sessionId: owner.sessionId, versionId: version.versionId })).source;
  assert.ok(Object.keys(s.history).length); s.draft = s.baseSource + '\nA PRIVATE MEMBER REMOVAL DRAFT 🌿'; s.expectedSource = s.baseSource;
  const status = await api(s, s.a, '/api/sync/status', { identity: s.identity }), heads = status.objects.find(item => item.object.kind === 'node' && item.object.id === s.nodeId).view.versions;
  assert.equal(heads.length, 1); s.memberPredecessor = heads[0]; assert.ok(s.memberPredecessor.state.files['asset.bin']);
  await writeFile(path.join(s.ra, 'Note', 'member-private.raw'), 'ORIGINAL SENDER RAW');
  await writeFile(path.join(s.rb, 'Note', 'member-private.raw'), 'ORIGINAL RECEIVER RAW');
  s.memberAsset = path.join(s.ra, 'Note', 'asset.bin'); s.memberIdentity = await physical(s.memberAsset); s.memberRetained = path.join(s.folder, 'retained-original-asset.bin');
  const manifestPath = path.join(s.ra, 'Note', 'weftext.resources.json'), manifestIdentity = await physical(manifestPath), manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
  assert.ok(path.resolve(s.memberAsset).startsWith(path.resolve(s.folder) + path.sep)); assert.ok(path.resolve(s.memberRetained).startsWith(path.resolve(s.folder) + path.sep));
  await assert.rejects(stat(s.memberRetained), error => error.code === 'ENOENT'); await rename(s.memberAsset, s.memberRetained); assert.deepEqual(await physical(s.memberRetained), s.memberIdentity);
  const count = manifest.resources.length; manifest.resources = manifest.resources.filter(entry => entry.path !== 'asset.bin'); assert.equal(manifest.resources.length, count - 1);
  await writeFile(manifestPath, JSON.stringify(manifest, null, 2) + '\n'); assert.deepEqual(await physical(manifestPath), manifestIdentity);
  s.memberObservedTree = await exactTree(s.ra); s.memberReceiverTree = await exactTree(s.rb);
  const port = s.a.port; await stop(s.a); s.a = await start(s, 'a-ui', ['--data-dir', s.da, '--port', port, '--open', path.join(s.ra, 'Note', 'Note.typ'), '--root', s.ra]);
  s.context = await browser.newContext({ viewport: { width: 1500, height: 1050 } }); s.page = await s.context.newPage(); s.page.setDefaultTimeout(30000);
  s.page.on('pageerror', error => result.pageErrors.push(`c${s.id}: ${String(error)}`));
  s.page.on('request', request => { const route = new URL(request.url()).pathname; if (route.startsWith('/api/')) s.requests.push({ route, body: request.postData() ? request.postDataJSON() : null }); });
  s.page.on('response', response => { const route = new URL(response.url()).pathname; if (route.startsWith('/api/')) s.responses.push({ route, status: response.status() }); });
  await s.page.goto(`${s.a.base}/#token=${s.a.token}`); await editor(s); assert.equal(await sourceText(s.editor), s.baseSource);
  const drafted = s.page.waitForResponse(response => response.url().endsWith('/api/draft') && response.request().postDataJSON()?.source === s.draft && response.ok());
  await s.editor.press('ControlOrMeta+a'); await s.page.keyboard.insertText(s.draft); await drafted; await s.editor.evaluate(element => window.__semanticOriginalEditor = element);
  s.panel = await sync(s);
  const binding = await api(s, s.a, '/api/sync/binding', { root: s.ra }), rows = s.panel.locator('.sync-object');
  for (let i = 0; i < await rows.count(); i++) await rows.nth(i).locator('input[type=checkbox]').uncheck();
  for (const object of binding.installedObjects) await rows.filter({ hasText: `${object.kind}:${object.id ?? ''}` }).locator('input[type=checkbox]').check();
  s.memberInstalledObjects = binding.installedObjects; return s;
}
async function removedOriginals(s) {
  assert.deepEqual(await exactTree(s.ra), s.memberObservedTree); assert.deepEqual(await exactTree(s.rb), s.memberReceiverTree);
  await assert.rejects(stat(s.memberAsset), error => error.code === 'ENOENT'); assert.deepEqual(await readFile(s.memberRetained), s.memberBytes); assert.deepEqual(await physical(s.memberRetained), s.memberIdentity);
  const owner = await api(s, s.a, '/api/open', { root: s.ra, path: path.join(s.ra, 'Note', 'Note.typ') }); assert.equal(owner.source, s.baseSource); assert.equal(owner.recoveredDraft, s.draft);
  for (const [versionId, source] of Object.entries(s.history)) assert.equal((await api(s, s.a, '/api/history/read', { sessionId: owner.sessionId, versionId })).source, source);
  const receiver = await api(s, s.b, '/api/open', { root: s.rb, path: path.join(s.rb, 'Note', 'Note.typ') }); assert.equal(receiver.source, s.baseSource); assert.equal(receiver.recoveredDraft, s.receiverDraft);
  for (const [versionId, source] of Object.entries(s.receiverHistory)) assert.equal((await api(s, s.b, '/api/history/read', { sessionId: receiver.sessionId, versionId })).source, source);
  const original = await api(s, s.a, '/api/sync/file', { identity: s.identity, object: s.key, revision: s.memberPredecessor.revision, path: 'asset.bin' }); assert.deepEqual(Buffer.from(original.text), s.memberBytes); await privateDraft(s);
}
async function restartMember(s) {
  const port = s.a.port; await stop(s.a); s.a = await start(s, 'a-restarted', ['--data-dir', s.da, '--port', port, '--open', path.join(s.ra, 'Note', 'Note.typ'), '--root', s.ra]);
  await s.page.goto(`${s.a.base}/?restart=${randomUUID()}#token=${s.a.token}`); await editor(s); assert.equal(await sourceText(s.editor), s.draft); s.panel = await sync(s);
}
async function receiverMemberOriginals(s) {
  assert.deepEqual(await exactTree(s.ra), s.memberObservedTree);
  assert.deepEqual(await readFile(s.memberRetained), s.memberBytes); assert.deepEqual(await physical(s.memberRetained), s.memberIdentity);
  const owner = await api(s, s.a, '/api/open', { root: s.ra, path: path.join(s.ra, 'Note', 'Note.typ') });
  assert.equal(owner.source, s.baseSource); assert.equal(owner.recoveredDraft, s.senderDraft);
  for (const [versionId, source] of Object.entries(s.senderHistory)) assert.equal((await api(s, s.a, '/api/history/read', { sessionId: owner.sessionId, versionId })).source, source);
  const after = await exactTree(s.rb), assetRelative = path.join('Note', 'asset.bin'), manifestRelative = path.join('Note', 'weftext.resources.json');
  assert.equal(after[assetRelative], undefined);
  for (const [relative, original] of Object.entries(s.memberReceiverTree)) {
    if (relative === assetRelative) continue;
    if (relative === manifestRelative) { assert.deepEqual(after[relative].physical, original.physical); assert.equal(after[relative].sha256, sha(await readFile(path.join(s.ra, relative)))); }
    else assert.deepEqual(after[relative], original, `Resource retirement preserves ${relative}`);
  }
  assert.deepEqual(await readFile(s.receiverRetained), s.memberBytes); assert.deepEqual(await physical(s.receiverRetained), s.receiverAssetIdentity);
  const receiver = await api(s, s.b, '/api/open', { root: s.rb, path: path.join(s.rb, 'Note', 'Note.typ') });
  assert.equal(receiver.source, s.baseSource); assert.equal(receiver.recoveredDraft, s.draft);
  for (const [versionId, source] of Object.entries(s.receiverHistory)) assert.equal((await api(s, s.b, '/api/history/read', { sessionId: receiver.sessionId, versionId })).source, source);
  const resources = await api(s, s.b, '/api/resources/list', { sessionId: receiver.sessionId }); assert.deepEqual(resources.resources, []);
  const unavailable = await api(s, s.b, '/api/resources/read', { sessionId: receiver.sessionId, resource: s.receiverResource.resource, revision: s.receiverResource.revision }, 409); assert.equal(unavailable.error.code, 'resourceUnavailable');
  const original = await api(s, s.b, '/api/sync/file', { identity: s.identity, object: s.key, revision: s.memberPredecessor.revision, path: 'asset.bin' }); assert.deepEqual(Buffer.from(original.text), s.memberBytes);
  const graph = await api(s, s.b, '/api/sync/status', { identity: s.identity }), versions = graph.objects.find(item => item.object.kind === 'node' && item.object.id === s.nodeId).view.versions;
  assert.deepEqual(versions, [s.memberAfter]); assert.deepEqual((await api(s, s.b, '/api/knowledge/trash', { root: s.rb })).trash, []); await privateDraft(s);
}
async function receiverMemberUndo(s) {
  await privateDraft(s); assert.equal(await s.editor.evaluate(element => element === window.__semanticOriginalEditor), true);
  for (const [keys, source] of [['ControlOrMeta+z', s.receiverOriginalDraft], ['ControlOrMeta+Shift+Z', s.draft]]) {
    const saved = s.page.waitForResponse(response => response.url().endsWith('/api/draft') && response.request().postDataJSON()?.source === source && response.ok());
    await s.editor.click(); await s.editor.press(keys); await until(() => sourceText(s.editor), value => value === source, 'receiver original Draft Undo/Redo'); await saved;
  }
}
async function memberResourcePanel(s) {
  await s.page.getByRole('button', { name: '链接与批注', exact: true }).click();
  const knowledge = s.page.getByLabel('知识与批注', { exact: true }); await knowledge.getByRole('tab', { name: '批注', exact: true }).click();
  const target = knowledge.locator('select:has(option[value="media"])');
  await target.waitFor({ state: 'visible' }); assert.equal(await target.count(), 1); await target.selectOption('media');
  return knowledge.locator('.media-viewer');
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
  const next = s.page.waitForResponse(response => response.url().endsWith('/api/sync/revalidation-review'), { timeout: s.id >= 6 ? 95000 : 30000 });
  await s.reviewPanel.locator("[data-sync-revalidation-action='review']").click(); const response = await next; s.review = await response.json(); assert.equal(response.status(), 200, JSON.stringify(s.review));
  await s.reviewPanel.locator("[data-sync-revalidation-summary='ready']").waitFor();
  assert.equal(s.review.resolutions.length, 1); assert.deepEqual(s.review.resolutions[0].object, s.key); assert.deepEqual(s.review.resolutions[0].revisions, [s.head.revision]);
  const schema = (await api(s, s.b, '/api/sync/status', { identity: s.identity })).objects.find(item => item.object.kind === 'schema').view.versions;
  assert.equal(schema.length, 1); assert.ok(s.review.resolutions[0].reviewedDependencies.some(dep => dep.object.kind === 'schema' && dep.revisions.length === 1 && dep.revisions[0] === schema[0].revision));
  if (s.nativeKey) assert.ok(s.review.reviewedVersions.some(dep => dep.object.kind === 'node' && dep.object.id === s.nativeKey.id));
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
  const resolution = s.original.request.intent.resolutions.find(item => item.object.kind === s.key.kind && item.object.id === s.key.id); assert.ok(resolution);
  assert.equal(view.length, 1); assert.notEqual(view[0].revision, s.head.revision); assert.deepEqual(view[0].state.fileRevisions, resolution.state.fileRevisions);
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
async function nativeOriginals(s) {
  assert.equal(await readFile(path.join(s.rb, 'Task', 'Task.typ'), 'utf8'), s.nativeSource);
  assert.deepEqual(await physical(path.join(s.rb, 'Task')), s.nativePhysical.directory);
  assert.deepEqual(await physical(path.join(s.rb, 'Task', 'Task.typ')), s.nativePhysical.source);
}
async function nativeRuntime(s) {
  const defined = await api(s, s.b, '/api/query/definition/run', { root: s.rb, owner: `weftext-node://${s.identity.workspaceId}/${s.nodeId}`, selection: { mode: 'current', key: s.nativeDefinitionKey }, parameters: {} });
  assert.equal(defined.result.coverage, 'complete'); assert.equal(defined.result.rows.length, 1);
  assert.equal(defined.result.rows[0].entityKey, `weftext-node://${s.identity.workspaceId}/${s.nativeKey.id}`);
  assert.deepEqual(defined.result.rows[0].cells.actual.value, { kind: 'date', value: '2026-01-02' });
  const products = await api(s, s.b, '/api/query/run', { root: s.rb, plan: { types: ['task'], select: [{ name: 'title', expr: 'title' }, { name: 'body', expr: 'body' }, { name: 'headings', expr: 'headings' }] }, parameters: {} });
  assert.equal(products.coverage, 'complete'); assert.equal(products.rows.length, 1);
  const cells = products.rows[0].cells; assert.deepEqual(cells.title.value, { kind: 'text', value: '精确标题' });
  assert.ok(cells.body.value.value.includes('循环2')); assert.ok(!cells.body.value.value.includes('不存在标题'));
  if (s.id === 9) assert.ok(cells.body.value.value.includes('外部新增循环3'));
  assert.ok(cells.headings.value.value.some(heading => heading.kind === 'record' && heading.value.some(member => member.key === 'level' && member.value.value === '12') && heading.value.some(member => member.key === 'generated' && member.value.value === true)));
  await nativeOriginals(s);
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
  [6, 'Real native title, final body and generated level-12 heading drive the exact provider; lost preparation retries one request while Draft and Undo survive', async s => {
    await inspect(s); const prepared = await prepare(s, true); assert.deepEqual((await pending(s.page)).request, prepared.request);
    const retried = await tap(s, '/api/operations/prepare', () => s.panel.getByRole('button', { name: '重试原准备请求', exact: true }).click());
    assert.deepEqual(retried.request, prepared.request); assert.deepEqual(retried.response, prepared.response);
    await s.panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor(); await apply(s); await verify(s); await nativeRuntime(s); await compareAndSave(s);
    assert.equal(s.requests.filter(request => request.route === '/api/operations/prepare').length, 2); assert.equal(s.requests.filter(request => request.route === '/api/operations/commit').length, 1);
  }],
  [7, 'A real wrong final native title reaches the provider failure and refuses review without a Plan or a source/Draft replacement', async s => {
    const response = s.page.waitForResponse(value => value.url().endsWith('/api/sync/revalidation-review'), { timeout: 95000 });
    await s.reviewPanel.locator("[data-sync-revalidation-action='review']").click(); const reply = await response, refused = await reply.json();
    assert.equal(reply.status(), 409); assert.equal(refused.error.code, 'queryInvocationUnavailable');
    await s.panel.getByRole('status').filter({ hasText: /.+/ }).waitFor();
    await until(() => s.reviewPanel.locator("[data-sync-revalidation-action='review']").isEnabled(), Boolean, 'failed native review unlock');
    assert.equal(await s.reviewPanel.locator("[data-sync-revalidation-action='prepare']").isDisabled(), true);
    assert.equal(await s.reviewPanel.locator("[data-sync-revalidation-summary='ready']").count(), 0); assert.equal(await pending(s.page), null);
    assert.equal(s.requests.filter(request => request.route === '/api/operations/prepare').length, 0); assert.equal(s.requests.filter(request => request.route === '/api/operations/commit').length, 0);
    assert.equal(await readFile(path.join(s.rb, 'Note', 'Note.typ'), 'utf8'), s.baseSource); await nativeOriginals(s);
    await s.panel.getByRole('button', { name: '关闭同步', exact: true }).click(); await privateDraft(s, true); s.noAuthorEffect = true;
  }],
  [8, 'A prepared real native include is replaced by a same-byte new file; the original commit refuses and retains its exact request and private Draft/Undo', async s => {
    await inspect(s); const prepared = await prepare(s), originalIdentity = await physical(s.nativeInclude), retained = path.join(s.folder, 'retained-original-shared.typ');
    assert.ok(path.resolve(s.nativeInclude).startsWith(path.resolve(s.folder) + path.sep)); assert.ok(path.resolve(retained).startsWith(path.resolve(s.folder) + path.sep));
    await assert.rejects(stat(retained), error => error.code === 'ENOENT'); await rename(s.nativeInclude, retained); await writeFile(s.nativeInclude, s.nativeIncludeBytes);
    assert.notDeepEqual(await physical(s.nativeInclude), originalIdentity); assert.deepEqual(await readFile(retained), await readFile(s.nativeInclude));
    const response = s.page.waitForResponse(value => value.url().endsWith('/api/operations/commit'));
    await s.panel.getByRole('button', { name: '确认应用此预览', exact: true }).click(); const reply = await response, refused = await reply.json(); assert.equal(reply.status(), 409); assert.equal(refused.error.code, 'conflict');
    await s.panel.getByRole('status').filter({ hasText: /原操作记录保留/ }).waitFor();
    assert.deepEqual((await pending(s.page)).request, prepared.request); assert.equal((await pending(s.page)).attempted, true); await done(s.panel, '检查原操作结果');
    assert.equal((await pending(s.page)).plan.planId, prepared.response.planId); assert.equal((await pending(s.page)).status, 'prepared');
    const original = await api(s, s.b, '/api/sync/operation', { operationId: prepared.request.operationId }); assert.deepEqual(original.request, prepared.request); assert.deepEqual(original.plan, prepared.response);
    assert.equal(await readFile(path.join(s.rb, 'Note', 'Note.typ'), 'utf8'), s.baseSource); await nativeOriginals(s);
    await s.panel.getByRole('button', { name: '关闭同步', exact: true }).click(); await privateDraft(s, true); s.noAuthorEffect = true;
  }],
  [9, 'Actual native external adoption consumes final after products, preserves observed bytes/FI and Draft/Undo, and recovers the same committed operation after lost response and restart', async s => {
    // Install the definition by a genuine earlier operation. The second UI
    // action adopts physically external current bytes through adoption-cut;
    // its request contains no received resolution or compiled proof flags.
    await inspect(s); await prepare(s); await apply(s); await verify(s);
    await done(s.panel, '结束已确认操作');
    await s.panel.getByRole('button', { name: '关闭同步', exact: true }).click(); await privateDraft(s, true); s.panel = await sync(s);
    s.expectedSource = s.receivedSource + '\n真实外部采纳正文 中文😀\n';
    s.nativeSource += '\n#for n in (3,) [外部新增循环#n]\n';
    await writeFile(path.join(s.rb, 'Note', 'Note.typ'), s.expectedSource);
    await writeFile(path.join(s.rb, 'Task', 'Task.typ'), s.nativeSource);
    const raw = path.join(s.rb, 'Note', 'adoption-private.raw'), rawBytes = Buffer.from('ORIGINAL NATIVE ADOPTION RAW 中文😀');
    await writeFile(raw, rawBytes); s.adoptionRawPhysical = await physical(raw);
    const exact = async () => {
      assert.equal(await readFile(path.join(s.rb, 'Note', 'Note.typ'), 'utf8'), s.expectedSource);
      assert.deepEqual(await physical(path.join(s.rb, 'Note')), s.originalPhysical.directory);
      assert.deepEqual(await physical(path.join(s.rb, 'Note', 'Note.typ')), s.originalPhysical.source);
      assert.deepEqual(await readFile(raw), rawBytes); assert.deepEqual(await physical(raw), s.adoptionRawPhysical); await nativeOriginals(s); await privateDraft(s);
    };
    const binding = await api(s, s.b, '/api/sync/binding', { root: s.rb });
    const rows = s.panel.locator('.sync-object'); for (let i = 0; i < await rows.count(); i++) await rows.nth(i).locator('input[type=checkbox]').uncheck();
    for (const object of binding.installedObjects) await rows.filter({ hasText: `${object.kind}:${object.id ?? ''}` }).locator('input[type=checkbox]').check();
    const cutResponse = s.page.waitForResponse(response => response.url().endsWith('/api/sync/adoption-cut'));
    const prepared = await tap(s, '/api/operations/prepare', () => s.panel.getByRole('button', { name: '预览采纳当前外部修改', exact: true }).click());
    const response = await cutResponse; assert.equal(response.status(), 200); const cut = await response.json(); s.adoptionCut = cut; s.adoptionOriginal = prepared;
    assert.equal(prepared.request.intent.kind, 'adoptSyncExternal'); assert.equal(prepared.response.kind, 'adoptSyncExternal');
    for (const field of ['identity', 'expectedInstalled', 'changes']) assert.deepEqual(prepared.request.intent[field], cut[field]);
    assert.equal(cut.changes.length, 2); assert.deepEqual(cut.expectedInstalled.map(dep => JSON.stringify(dep.object)).sort(), binding.installedObjects.map(object => JSON.stringify(object)).sort());
    const expected = new Map([[path.join(s.rb, 'Note', 'Note.typ'), s.expectedSource], [path.join(s.rb, 'Task', 'Task.typ'), s.nativeSource]]);
    for (const change of cut.changes) { assert.ok(change.baseId); assert.deepEqual(change.before, change.after); assert.equal(Buffer.from(change.after).toString('utf8'), expected.get(path.resolve(change.path))); }
    await s.panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor(); await exact();
    await s.panel.getByRole('button', { name: '关闭同步', exact: true }).click(); await privateDraft(s, true); s.panel = await sync(s);
    const committed = await apply(s, true); await exact(); await restart(s); await done(s.panel, '检查原操作结果');
    assert.deepEqual((await pending(s.page)).request, prepared.request); assert.equal((await pending(s.page)).status, 'committed');
    const retained = await api(s, s.b, '/api/sync/operation', { operationId: prepared.request.operationId });
    assert.deepEqual(retained.request, prepared.request); assert.equal(retained.plan.planId, prepared.response.planId); assert.equal(retained.plan.status, 'committed');
    assert.deepEqual(await commit(s, s.b, prepared.response), committed.response); await exact();
    const current = await api(s, s.b, '/api/open', { root: s.rb, path: path.join(s.rb, 'Note', 'Note.typ') });
    assert.equal(current.source, s.expectedSource); assert.equal(current.recoveredDraft, s.draft);
    for (const [versionId, source] of Object.entries(s.history)) assert.equal((await api(s, s.b, '/api/history/read', { sessionId: current.sessionId, versionId })).source, source);
    await api(s, s.b, '/api/sync/publish', { root: s.rb });
    const status = await api(s, s.b, '/api/sync/status', { identity: s.identity });
    for (const [object, relative, source] of [[s.key, 'Note.typ', s.expectedSource], [s.nativeKey, 'Task.typ', s.nativeSource]]) {
      const heads = status.objects.find(item => item.object.kind === object.kind && item.object.id === object.id).view.versions; assert.equal(heads.length, 1); assert.equal(heads[0].state.files[relative], sha(source));
      assert.equal((await api(s, s.b, '/api/sync/file', { identity: s.identity, object, revision: heads[0].revision, path: relative })).text, source);
    }
    assert.equal((await api(s, s.b, '/api/sync/file', { identity: s.identity, object: s.key, revision: s.head.revision, path: 'Note.typ' })).text, s.receivedSource);
    await nativeRuntime(s); await exact(); assert.equal(s.requests.filter(request => request.route === '/api/operations/prepare').length, 2); assert.equal(s.requests.filter(request => request.route === '/api/operations/commit').length, 2);
    await done(s.panel, '结束已确认操作'); await s.panel.getByRole('button', { name: '关闭同步', exact: true }).click(); await privateDraft(s);
  }],
  [10, 'One bound native review installs a new Trash query definition and retires its live Task in the same original request, preserving moved identities, private Draft/Undo and restart receipt', async s => {
    const response = s.page.waitForResponse(value => value.url().endsWith('/api/sync/revalidation-review'), { timeout: 95000 });
    await s.reviewPanel.locator("[data-sync-revalidation-action='review']").click(); const reply = await response; s.review = await reply.json(); assert.equal(reply.status(), 200, JSON.stringify(s.review));
    const reviewedRequest = reply.request().postDataJSON(); assert.equal(typeof reviewedRequest.operationId, 'string'); assert.ok(reviewedRequest.operationId); assert.equal(s.review.operationId, reviewedRequest.operationId);
    assert.equal(s.review.resolutions.length, 2);
    for (const [object, head] of [[s.key, s.head], [s.nativeKey, s.taskHead]]) {
      const selected = s.review.resolutions.find(item => item.object.kind === object.kind && item.object.id === object.id);
      assert.deepEqual(selected.revisions, [head.revision]); assert.deepEqual(selected.decision, { kind: 'preserve', revision: head.revision }); assert.deepEqual(selected.state, head.state);
    }
    assert.ok(s.review.expectedInstalled.some(dep => dep.object.kind === 'node' && dep.object.id === s.nativeKey.id && dep.revisions.length === 1 && dep.revisions[0] === s.taskInstalled));
    const target = path.join(s.rb, '.weftext-trash', s.review.operationId); s.retireTarget = target;
    const originalLive = async () => { await nativeOriginals(s); assert.deepEqual(await readFile(path.join(s.rb, 'Task', 'retire-private.raw')), s.retireRawBytes); assert.deepEqual(await physical(path.join(s.rb, 'Task', 'retire-private.raw')), s.retireRawPhysical); assert.equal(await readFile(path.join(s.rb, 'Note', 'Note.typ'), 'utf8'), s.baseSource); await assert.rejects(stat(target), error => error.code === 'ENOENT'); await privateDraft(s); };
    await s.reviewPanel.locator("[data-sync-revalidation-summary='ready']").waitFor(); await originalLive();
    assert.equal((await api(s, s.b, '/api/operations/status', { operationId: s.review.operationId })).status, 'unknown'); assert.equal(await pending(s.page), null);
    await s.reviewPanel.screenshot({ path: path.join(s.folder, 'same-request-retire-review.png') });
    const prepared = await prepare(s); assert.equal(prepared.request.operationId, reviewedRequest.operationId); await originalLive();
    await apply(s); await verify(s);
    const moved = async () => {
      await assert.rejects(stat(path.join(s.rb, 'Task')), error => error.code === 'ENOENT');
      assert.deepEqual(await physical(target), s.nativePhysical.directory); assert.deepEqual(await physical(path.join(target, 'Task.typ')), s.nativePhysical.source);
      assert.equal(await readFile(path.join(target, 'Task.typ'), 'utf8'), s.nativeSource); assert.deepEqual(await readFile(path.join(target, 'retire-private.raw')), s.retireRawBytes); assert.deepEqual(await physical(path.join(target, 'retire-private.raw')), s.retireRawPhysical);
    };
    await moved(); const binding = await api(s, s.b, '/api/sync/status', { identity: s.identity }), applied = binding.objects.find(item => item.object.kind === 'node' && item.object.id === s.nativeKey.id).view.versions; assert.equal(applied.length, 1); assert.equal(applied[0].state.retired, true); assert.deepEqual(applied[0].state.files, s.taskHead.state.files); assert.deepEqual(applied[0].state.fileRevisions, s.taskHead.state.fileRevisions);
    const actual = await api(s, s.b, '/api/query/definition/run', { root: s.rb, owner: `weftext-node://${s.identity.workspaceId}/${s.nodeId}`, selection: { mode: 'current', key: s.nativeDefinitionKey }, parameters: {} });
    assert.equal(actual.result.coverage, 'complete'); assert.equal(actual.result.rows.length, 1); assert.equal(actual.result.rows[0].entityKey, `weftext-node://${s.identity.workspaceId}/${s.nativeKey.id}`); assert.equal(actual.result.rows[0].lifecycle, 'trash'); assert.equal(actual.result.rows[0].readOnly, true); assert.deepEqual(actual.result.rows[0].cells.actual.value, { kind: 'date', value: '2026-01-02' });
    const receipt = await commit(s, s.b, prepared.response);
    await s.panel.getByRole('button', { name: '关闭同步', exact: true }).click(); await privateDraft(s, true); s.panel = await sync(s);
    await restart(s); await done(s.panel, '检查原操作结果'); await moved(); await verify(s);
    const retained = await api(s, s.b, '/api/sync/operation', { operationId: prepared.request.operationId }); assert.deepEqual(retained.request, prepared.request); assert.equal(retained.plan.planId, prepared.response.planId); assert.equal(retained.plan.status, 'committed'); assert.deepEqual(await commit(s, s.b, prepared.response), receipt);
    assert.equal(s.requests.filter(request => request.route === '/api/operations/prepare').length, 1); assert.equal(s.requests.filter(request => request.route === '/api/operations/commit').length, 1);
  }],
  [11, 'Explicit adoption of a physically removed registered resource keeps its original absence cut, no-write author tree, private Draft/Undo and exact operation through lost prepare and commit responses', async s => {
    const cutResponse = s.page.waitForResponse(response => response.url().endsWith('/api/sync/adoption-cut'));
    const prepared = await tap(s, '/api/operations/prepare', () => s.panel.getByRole('button', { name: '预览采纳当前外部修改', exact: true }).click(), { drop: true });
    const response = await cutResponse; assert.equal(response.status(), 200); const cut = await response.json(); s.memberCut = cut; s.memberOriginal = prepared;
    await writeFile(path.join(s.folder, 'removed-member-fixture.json'), JSON.stringify({ predecessor: s.memberPredecessor, cut, original: prepared, originalPhysicalIdentity: s.memberIdentity, retainedOriginal: s.memberRetained, observedSenderTree: s.memberObservedTree, unchangedReceiverTree: s.memberReceiverTree }, null, 2));
    assert.equal(prepared.request.intent.kind, 'adoptSyncExternal'); assert.equal(prepared.response.kind, 'adoptSyncExternal');
    for (const field of ['identity', 'expectedInstalled', 'changes', 'removedMembers']) assert.deepEqual(prepared.request.intent[field], cut[field]);
    assert.deepEqual(cut.expectedInstalled.map(dep => JSON.stringify(dep.object)).sort(), s.memberInstalledObjects.map(object => JSON.stringify(object)).sort());
    assert.equal(cut.removedMembers.length, 1); const removed = cut.removedMembers[0]; assert.equal(path.resolve(removed.path), s.memberAsset); assert.ok(removed.baseId); assert.deepEqual(Object.keys(removed).sort(), ['baseId', 'path']);
    assert.equal(cut.changes.length, 1); const changed = cut.changes[0]; assert.equal(path.resolve(changed.path), path.join(s.ra, 'Note', 'weftext.resources.json')); assert.deepEqual(changed.before, changed.after); assert.deepEqual(Buffer.from(changed.after), await readFile(changed.path));
    await s.panel.getByRole('status').filter({ hasText: /原操作记录保留/ }).waitFor(); assert.equal((await pending(s.page)).status, 'preparing'); assert.equal((await pending(s.page)).attempted, false); assert.equal((await pending(s.page)).plan, undefined); assert.deepEqual((await pending(s.page)).request, prepared.request);
    assert.match(await s.panel.innerText(), /以下资源已在外部移除/); assert.ok((await s.panel.innerText()).includes(s.memberAsset)); await removedOriginals(s);
    await s.panel.screenshot({ path: path.join(s.folder, 'removed-member-original-unknown.png') });
    await s.panel.getByRole('button', { name: '关闭同步', exact: true }).click(); await privateDraft(s, true); s.panel = await sync(s);
    await restartMember(s); assert.deepEqual((await pending(s.page)).request, prepared.request);
    const retried = await tap(s, '/api/operations/prepare', () => s.panel.getByRole('button', { name: '重试原准备请求', exact: true }).click()); assert.deepEqual(retried.request, prepared.request); assert.deepEqual(retried.response, prepared.response);
    await s.panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor(); await removedOriginals(s);
    const committed = await apply(s, true); assert.equal((await pending(s.page)).status, 'unknown'); assert.equal((await pending(s.page)).attempted, true); await removedOriginals(s); await restartMember(s); await done(s.panel, '检查原操作结果');
    assert.deepEqual((await pending(s.page)).request, prepared.request); assert.equal((await pending(s.page)).status, 'committed');
    const retained = await api(s, s.a, '/api/sync/operation', { operationId: prepared.request.operationId }); assert.deepEqual(retained.request, prepared.request); assert.equal(retained.plan.status, 'committed'); assert.equal(retained.plan.planId, prepared.response.planId);
    assert.deepEqual(await commit(s, s.a, prepared.response), committed.response); await api(s, s.a, '/api/sync/publish', { root: s.ra });
    const status = await api(s, s.a, '/api/sync/status', { identity: s.identity }), heads = status.objects.find(item => item.object.kind === 'node' && item.object.id === s.nodeId).view.versions;
    assert.equal(heads.length, 1); assert.notEqual(heads[0].revision, s.memberPredecessor.revision); assert.equal('asset.bin' in heads[0].state.files, false); assert.equal('asset.bin' in heads[0].state.fileRevisions, false);
    for (const field of ['files', 'fileRevisions']) assert.equal(heads[0].state[field]['Note.typ'], s.memberPredecessor.state[field]['Note.typ']);
    assert.equal(heads[0].state.files['weftext.resources.json'], sha(await readFile(path.join(s.ra, 'Note', 'weftext.resources.json')))); await removedOriginals(s);
    assert.equal(s.requests.filter(request => request.route === '/api/operations/prepare').length, 2); assert.equal(s.requests.filter(request => request.route === '/api/operations/commit').length, 1);
    await done(s.panel, '结束已确认操作'); await s.panel.getByRole('button', { name: '关闭同步', exact: true }).click(); await privateDraft(s);
  }],
  [12, 'Receiving a registered-resource removal through the original sync UI preserves the exact physical original, refreshes current resources and retains the open private Draft/Undo through original-operation recovery', async s => {
    const senderPrepared = await tap(s, '/api/operations/prepare', () => s.panel.getByRole('button', { name: '预览采纳当前外部修改', exact: true }).click());
    assert.equal(senderPrepared.request.intent.kind, 'adoptSyncExternal'); assert.equal(senderPrepared.request.intent.removedMembers.length, 1);
    await s.panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor(); await apply(s); await removedOriginals(s);
    await api(s, s.a, '/api/sync/publish', { root: s.ra });
    const graph = await api(s, s.a, '/api/sync/status', { identity: s.identity }); s.memberAfter = graph.objects.find(item => item.object.kind === 'node' && item.object.id === s.nodeId).view.versions[0];
    assert.notEqual(s.memberAfter.revision, s.memberPredecessor.revision); assert.equal('asset.bin' in s.memberAfter.state.files, false); assert.equal('asset.bin' in s.memberAfter.state.fileRevisions, false);
    for (const field of ['files', 'fileRevisions']) assert.equal(s.memberAfter.state[field]['Note.typ'], s.memberPredecessor.state[field]['Note.typ']);
    await done(s.panel, '结束已确认操作'); await s.panel.getByRole('button', { name: '关闭同步', exact: true }).click(); await privateDraft(s, true);
    s.senderDraft = s.draft; s.senderHistory = s.history; await s.page.close();
    const receiver = await api(s, s.b, '/api/open', { root: s.rb, path: path.join(s.rb, 'Note', 'Note.typ') }), resources = await api(s, s.b, '/api/resources/list', { sessionId: receiver.sessionId });
    assert.equal(resources.resources.length, 1); s.receiverResource = resources.resources[0]; assert.equal(s.receiverResource.name, 'asset.bin'); assert.equal(s.receiverResource.currentness, 'accepted'); s.receiverAssetIdentity = await physical(path.join(s.rb, 'Note', 'asset.bin'));
    const port = s.b.port; await stop(s.b); s.b = await start(s, 'b-resource-ui', ['--data-dir', s.db, '--port', port, '--open', path.join(s.rb, 'Note', 'Note.typ'), '--root', s.rb]);
    s.page = await s.context.newPage(); s.page.setDefaultTimeout(30000); s.receiverRequests = [];
    s.page.on('pageerror', error => result.pageErrors.push(`c12 receiver: ${String(error)}`));
    s.page.on('request', request => { const route = new URL(request.url()).pathname; if (route.startsWith('/api/')) { const value = { actor: 'receiver', route, body: request.postData() ? request.postDataJSON() : null }; s.requests.push(value); s.receiverRequests.push(value); } });
    s.page.on('response', response => { const route = new URL(response.url()).pathname; if (route.startsWith('/api/')) s.responses.push({ actor: 'receiver', route, status: response.status() }); });
    await s.page.goto(`${s.b.base}/#token=${s.b.token}`); await editor(s); assert.equal(await sourceText(s.editor), s.receiverDraft);
    s.receiverOriginalDraft = s.receiverDraft; s.draft = s.receiverDraft + '\nB active resource-removal Undo 原草稿继续编辑';
    const drafted = s.page.waitForResponse(response => response.url().endsWith('/api/draft') && response.request().postDataJSON()?.source === s.draft && response.ok());
    await s.editor.press('ControlOrMeta+End'); await s.page.keyboard.insertText(s.draft.slice(s.receiverOriginalDraft.length)); await drafted; await s.editor.evaluate(element => window.__semanticOriginalEditor = element);
    const media = await memberResourcePanel(s); await until(() => media.getByLabel('选择已登记资源', { exact: true }).locator('option').allTextContents(), values => values.includes('asset.bin'), 'registered resource shown before Apply');
    let resourceRead, resourceReadFailure;
    const readOriginalResource = async intercepted => {
      try {
        const request = intercepted.request().postDataJSON(); assert.equal(request.resource, s.receiverResource.resource); assert.equal(request.revision, s.receiverResource.revision);
        const response = await intercepted.fetch(); assert.equal(response.status(), 200); const bytes = await response.body(); assert.deepEqual(bytes, s.memberBytes);
        await intercepted.fulfill({ response }); resourceRead = { request, bytes: bytes.length, sha256: sha(bytes) };
      } catch (error) { resourceReadFailure = error; await intercepted.abort('failed').catch(() => {}); }
    };
    await s.page.route('**/api/resources/read', readOriginalResource);
    try { await media.getByLabel('选择已登记资源', { exact: true }).selectOption(s.receiverResource.resource); await until(() => resourceReadFailure ?? resourceRead, Boolean, 'actual UI resource read bytes'); if (resourceReadFailure) throw resourceReadFailure; }
    finally { await s.page.unroute('**/api/resources/read', readOriginalResource); }
    await until(() => media.locator('.media-preview .media-help').textContent(), value => value === s.receiverResource.metadata.message, 'viewer consumed the exact resource response');
    await writeFile(path.join(s.folder, 'receiver-resource-read.json'), JSON.stringify(resourceRead, null, 2));
    s.exchange = path.join(s.folder, 'receiver-resource-exchange'); await api(s, s.a, '/api/sync/export', { root: s.ra, directory: s.exchange, objects: [s.key] });
    s.panel = await sync(s); await s.panel.getByLabel('交换目录路径（新目录或已配对目录）', { exact: true }).fill(s.exchange); await done(s.panel, '接收交换目录');
    assert.deepEqual(await exactTree(s.rb), s.memberReceiverTree); await privateDraft(s);
    const rows = s.panel.locator('.sync-object'); for (let i = 0; i < await rows.count(); i++) await rows.nth(i).locator('input[type=checkbox]').uncheck(); await rows.filter({ hasText: s.nodeId }).locator('input[type=checkbox]').check();
    const prepared = await tap(s, '/api/operations/prepare', () => s.panel.getByRole('button', { name: '预览应用所选对象', exact: true }).click()); s.memberReceiverOriginal = prepared;
    assert.equal(prepared.request.intent.kind, 'applySync'); assert.deepEqual(prepared.request.intent.objects, [s.key]); assert.deepEqual(prepared.request.intent.identity, s.identity);
    const retired = prepared.response.changes.filter(change => change.action === 'retireMember'), preserved = prepared.response.changes.filter(change => change.action === 'retainOriginal');
    assert.equal(retired.length, 1); assert.equal(path.resolve(retired[0].path), path.join(s.rb, 'Note', 'asset.bin')); assert.equal(retired[0].bytes, s.memberBytes.length);
    assert.equal(preserved.length, 1); s.receiverRetained = path.resolve(preserved[0].path); assert.ok(s.receiverRetained.startsWith(path.resolve(s.rb) + path.sep)); assert.notEqual(s.receiverRetained, path.join(s.rb, 'Note', 'asset.bin')); assert.equal(preserved[0].bytes, s.memberBytes.length); await assert.rejects(stat(s.receiverRetained), error => error.code === 'ENOENT');
    await s.panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor(); assert.deepEqual(await exactTree(s.rb), s.memberReceiverTree); assert.deepEqual((await pending(s.page)).request, prepared.request);
    await s.panel.screenshot({ path: path.join(s.folder, 'receiver-resource-original-preview.png') });
    const committed = await apply(s, true); assert.equal((await pending(s.page)).status, 'unknown'); assert.equal((await pending(s.page)).attempted, true); assert.deepEqual((await pending(s.page)).request, prepared.request); await receiverMemberOriginals(s);
    await done(s.panel, '检查原操作结果'); assert.equal((await pending(s.page)).status, 'committed'); assert.deepEqual((await pending(s.page)).request, prepared.request);
    await s.panel.getByRole('button', { name: '关闭同步', exact: true }).click(); await receiverMemberUndo(s);
    const refreshed = s.page.waitForResponse(response => response.url().endsWith('/api/resources/list') && response.ok()); await media.getByRole('button', { name: '刷新资源', exact: true }).click(); assert.deepEqual((await (await refreshed).json()).resources, []);
    await until(() => media.getByLabel('选择已登记资源', { exact: true }).locator('option').allTextContents(), values => values.length === 1 && !values.includes('asset.bin'), 'removed member no longer has a current viewer role'); assert.equal(await media.locator('.media-preview').locator(':scope > *').count(), 0); assert.match(await media.locator('.media-status').innerText(), /当前笔记没有已登记资源/);
    await s.page.screenshot({ path: path.join(s.folder, 'receiver-resource-refreshed-draft.png'), fullPage: true }); await receiverMemberOriginals(s);
    const installed = await exactTree(s.rb); await restart(s); await done(s.panel, '检查原操作结果'); await receiverMemberOriginals(s); assert.deepEqual(await exactTree(s.rb), installed);
    const original = await api(s, s.b, '/api/sync/operation', { operationId: prepared.request.operationId }); assert.deepEqual(original.request, prepared.request); assert.equal(original.plan.planId, prepared.response.planId); assert.equal(original.plan.status, 'committed'); assert.deepEqual(await commit(s, s.b, prepared.response), committed.response); assert.deepEqual(await exactTree(s.rb), installed);
    assert.equal(s.receiverRequests.filter(request => request.route === '/api/operations/prepare').length, 1); assert.equal(s.receiverRequests.filter(request => request.route === '/api/operations/commit').length, 1);
    await writeFile(path.join(s.folder, 'receiver-resource-fixture.json'), JSON.stringify({ sender: senderPrepared, received: s.memberAfter, receiver: prepared, receipt: committed.response, preservedOriginal: s.receiverRetained, originalIdentity: s.receiverAssetIdentity, oldResource: s.receiverResource, before: s.memberReceiverTree, installed }, null, 2));
    await done(s.panel, '结束已确认操作'); await s.panel.getByRole('button', { name: '关闭同步', exact: true }).click(); await privateDraft(s);
  }],
];
try {
  browser = await chromium.launch({ channel: 'chrome', headless: true }); result.browser = browser.version();
  for (const [id, description, action] of cases) if (selected.includes(id)) {
    const s = await setup(id);
    try { await action(s); await s.page.screenshot({ path: path.join(s.folder, 'completed.png'), fullPage: true }); }
    catch (error) { await s.page.screenshot({ path: path.join(s.folder, 'failure.png'), fullPage: true }).catch(() => {}); throw error; }
    finally { await s.context.close(); await stop(s.a); await stop(s.b); await writeFile(path.join(s.folder, 'browser-requests.json'), JSON.stringify(s.requests, null, 2)); await writeFile(path.join(s.folder, 'browser-responses.json'), JSON.stringify(s.responses, null, 2)); await writeFile(path.join(s.folder, 'fixture.json'), JSON.stringify({ identity: s.identity, head: s.head, review: s.review, original: s.original, originalPhysical: s.originalPhysical, baseSource: s.baseSource, draft: s.draft, receivedSource: s.receivedSource, expectedSource: s.expectedSource, finalSource: s.finalSource, nativeKey: s.nativeKey, nativeSource: s.nativeSource, nativePhysical: s.nativePhysical, nativeInclude: s.nativeInclude, nativeDefinitionKey: s.nativeDefinitionKey, adoptionCut: s.adoptionCut, adoptionOriginal: s.adoptionOriginal, adoptionRawPhysical: s.adoptionRawPhysical, taskHead: s.taskHead, taskInstalled: s.taskInstalled, retireTarget: s.retireTarget, retireRawPhysical: s.retireRawPhysical }, null, 2)); }
    assert.equal(await readFile(path.join(s.rb, 'Note', 'Note.typ'), 'utf8'), id === 3 || s.noAuthorEffect ? s.baseSource : s.finalSource ?? s.expectedSource); result.cases.push({ id, description, status: 'passed' });
  }
  assert.deepEqual(result.pageErrors, []); assert.deepEqual(result.routeErrors, []); assert.ok(servers.every(server => server.stderr === ''), 'Owned server stderr must be empty'); result.completed = true;
} catch (error) { result.failure = String(error.stack ?? error); result.failedCase = active?.id; if (active?.page && !active.page.isClosed()) await active.page.screenshot({ path: path.join(active.folder, 'failure.png'), fullPage: true }).catch(() => {}); throw error; }
finally {
  await browser?.close(); for (const server of servers) await stop(server);
  result.ownedProcessesClosed = servers.every(server => server.child.exitCode !== null || server.child.signalCode !== null) && !browser?.isConnected(); result.serverStderrBytes = servers.reduce((sum, server) => sum + Buffer.byteLength(server.stderr), 0);
  for (const [index, server] of servers.entries()) { await writeFile(path.join(server.folder, `server-${index}-${server.label}.stdout.log`), server.stdout); await writeFile(path.join(server.folder, `server-${index}-${server.label}.stderr.log`), server.stderr); }
  await writeFile(path.join(out, 'results.json'), JSON.stringify(result, null, 2)); console.log(JSON.stringify({ out, completed: result.completed, cases: result.cases, pageErrors: result.pageErrors, routeErrors: result.routeErrors, ownedProcessesClosed: result.ownedProcessesClosed }));
}
