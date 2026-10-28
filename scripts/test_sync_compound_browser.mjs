import { workspaceCommand, workspaceTab } from './_workspace_browser.mjs';
import { spawnServe, serveReady, stopServe, stringifyServeJson } from './_cli_serve.mjs';
import { createdDocument } from './_creation_result.mjs';
// Owned replicas and headless Chrome. Requires the compound ApplySync candidate.
// Producers use real rename/save Plans; routes only drop actual HTTP responses.
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import assert from 'node:assert/strict';

if (!process.argv[2] || !process.argv[3]) throw new Error('Usage: node scripts/test_sync_compound_browser.mjs EXE NEW_OUT [--only=1,2,...]');
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]);
const only = process.argv.slice(4).find(argument => argument.startsWith('--only='));
const selected = only ? only.slice(7).split(',').map(Number) : [1, 2, 3, 4, 5, 6, 7, 8, 9];
assert.ok(selected.length && selected.every(id => Number.isInteger(id) && id >= 1 && id <= 9) && new Set(selected).size === selected.length, 'invalid case selection');
await mkdir(out, { recursive: false });
const result = {
  exe, sha256: createHash('sha256').update(await readFile(exe)).digest('hex'),
  version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(),
  selectedCases: selected, plannedCases: 9, cases: [], pageErrors: [], routeErrors: [],
  completed: false, ownedProcessesClosed: false,
};
const servers = [], scenarios = [], taps = new Set();
let browser, activeScenario;
const pause = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds));
function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  promise.catch(() => {});
  return { promise, resolve, reject };
}
async function bounded(promise, milliseconds, label) {
  let timer;
  try { return await Promise.race([promise, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error(`${label} timeout`)), milliseconds); })]); }
  finally { clearTimeout(timer); }
}
async function until(read, predicate, label, milliseconds = 15000) {
  const deadline = Date.now() + milliseconds;
  let value;
  do {
    value = await read(); if (predicate(value)) return value;
    await pause(60);
  } while (Date.now() < deadline);
  throw new Error(`${label} timeout; last=${JSON.stringify(value)}`);
}
async function start(folder, label, args) {
  const child = spawnServe(exe, [...args], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  const server = { child, folder, label, stdout: '', stderr: '' }; servers.push(server);
  child.stdout.on('data', bytes => server.stdout += bytes); child.stderr.on('data', bytes => server.stderr += bytes);
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    const match = serveReady(child);
    if (match) return Object.assign(server, { base: match.base, token: match.token, port: new URL(match.base).port });
    if (child.exitCode !== null || child.signalCode !== null) throw new Error(`${label} startup failed: ${server.stderr}`);
    await pause(80);
  }
  throw new Error(`${label} startup timeout`);
}
async function stop(server) { await stopServe(server?.child); }
async function api(server, route, data, expected = 200) {
  const response = await fetch(server.base + route, {
    method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': server.token },
    body: JSON.stringify(data), signal: AbortSignal.timeout(45000),
  });
  const value = await response.json();
  assert.equal(response.status, expected, `${route}: ${JSON.stringify(value)}`); return value;
}
async function commit(server, plan) {
  const receipt = await api(server, '/api/operations/commit', { operationId: plan.operationId, planId: plan.planId });
  assert.equal(receipt.status, 'committed'); return receipt;
}
const sourceText = async editor => (await editor.locator('.cm-line').allTextContents()).join('\n');
const pending = page => page.evaluate(() => {
  const value = localStorage.getItem('weftext-sync-operation-v1'); return value ? JSON.parse(value) : null;
});
async function sourceEditor(page) {
  const mode = (await workspaceTab(page, 'view')).getByRole('button', { name: '源码', exact: true }); await mode.waitFor(); await mode.click();
  const editor = page.locator('.cm-content[contenteditable=true]'); await editor.waitFor(); return editor;
}
async function enterDraft(page, editor, source) {
  const persisted = page.waitForResponse(response => response.url().endsWith('/api/draft') && response.request().postDataJSON()?.source === source && response.ok(), { timeout: 30000 });
  persisted.catch(() => {});
  await editor.press('ControlOrMeta+a'); await page.keyboard.insertText(source); await persisted;
  assert.equal(await sourceText(editor), source);
}
async function sync(page) {
  await workspaceCommand(page, 'knowledge', '同步');
  const panel = page.getByRole('dialog', { name: '资料库同步' }); await panel.waitFor();
  await panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor(); return panel;
}
async function clickDone(panel, name) {
  await panel.getByRole('button', { name, exact: true }).click();
  await panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor();
}
async function closePanel(panel) { await panel.getByRole('button', { name: '关闭同步', exact: true }).click(); }
async function onlyNodes(panel, nodes) {
  const rows = panel.locator('.sync-object');
  for (let index = 0; index < await rows.count(); index++) await rows.nth(index).locator('input[type=checkbox]').uncheck();
  for (const node of nodes) await rows.filter({ hasText: node.nodeId }).locator('input[type=checkbox]').check();
  assert.equal(await rows.locator('input:checked').count(), nodes.length);
}
const objectKey = key => `${key.kind}:${key.id ?? ''}`;
const afterRelative = node => node.afterRelative ?? path.join(node.nextName, `${node.nextName}.typ`);
async function selectedObjects(panel, objects) {
  const rows = panel.locator('.sync-object');
  for (let index = 0; index < await rows.count(); index++) await rows.nth(index).locator('input[type=checkbox]').uncheck();
  for (const key of objects) {
    const row = rows.filter({ hasText: `(${objectKey(key)})` }); assert.equal(await row.count(), 1);
    await row.locator('input[type=checkbox]').check();
  }
  assert.equal(await rows.locator('input:checked').count(), objects.length);
}
async function physicalIdentity(location) {
  assert.equal(process.platform, 'win32', 'physical-object evidence uses the Windows candidate');
  const value = await stat(location, { bigint: true });
  assert.notEqual(value.ino, 0n, 'owned NTFS object must expose its real file index');
  return { dev: value.dev.toString(), ino: value.ino.toString(), birthtimeNs: value.birthtimeNs.toString() };
}
async function responseOnce(page, routeName, { drop = false, expected = 200 } = {}) {
  const observed = deferred(), finished = deferred(), pattern = `**${routeName}`;
  let captured = false;
  const tap = {
    wait: () => bounded(observed.promise, 50000, routeName),
    async dispose() {
      if (captured) await bounded(finished.promise, 50000, `${routeName} route finish`);
      await page.unroute(pattern, handler); taps.delete(tap);
    },
  };
  const handler = async route => {
    if (captured) { await route.continue(); return; }
    captured = true;
    try {
      const request = route.request().postDataJSON(), retained = await pending(page);
      assert.ok(retained, 'original request must be durable before HTTP effect');
      if (routeName.endsWith('/prepare')) assert.deepEqual(retained.request, request);
      if (routeName.endsWith('/commit')) {
        assert.equal(retained.attempted, true); assert.equal(retained.status, 'unknown');
        assert.equal(request.operationId, retained.request.operationId); assert.equal(request.planId, retained.plan.planId);
      }
      const response = await route.fetch({ timeout: 45000 }), value = await response.json();
      assert.equal(response.status(), expected, `${routeName}: ${JSON.stringify(value)}`);
      if (drop) await route.abort('failed'); else await route.fulfill({ response });
      observed.resolve({ request, retained, response: value, status: response.status() });
    } catch (error) {
      result.routeErrors.push(`${routeName}: ${String(error.stack ?? error)}`);
      observed.reject(error); await route.abort('failed').catch(() => {});
    } finally { finished.resolve(); }
  };
  await page.route(pattern, handler); taps.add(tap); return tap;
}

async function setup(id) {
  const folder = path.join(out, `case-${id}`); await mkdir(folder);
  const s = { id, folder, ra: path.join(folder, 'library-a'), rb: path.join(folder, 'library-b'), da: path.join(folder, 'state-a'), db: path.join(folder, 'state-b'), exchange: path.join(folder, 'exchange'), requests: [], responses: [], responseReads: new Set(), observations: [], producers: [] };
  scenarios.push(s); activeScenario = s;
  s.a = await start(folder, 'a', ['--data-dir', s.da]); s.b = await start(folder, 'b-initial', ['--data-dir', s.db]);
  await api(s.a, '/api/library/create', { path: s.ra, operationId: randomUUID() });
  s.nodes = [];
  for (const [name, title] of [['Left', '左原稿'], ['Right', '右原稿']]) {
    const note = createdDocument(await api(s.a, '/api/note/create', { root: s.ra, parent: s.ra, name, operationId: randomUUID() }));
    note.source += `\n= ${title}\n\n原始内容 ${name} 😀\n`;
    await api(s.a, '/api/save', { sessionId: note.sessionId, baseId: note.baseId, source: note.source, operationId: randomUUID() });
    s.nodes.push({ ...note, name, key: { kind: 'node', id: note.nodeId }, nextName: name === 'Left' ? '左改名' : '右改名' });
  }
  s.identity = (await api(s.a, '/api/sync/enable', { root: s.ra, operationId: randomUUID() })).identity;
  await api(s.a, '/api/sync/export', { root: s.ra, directory: s.exchange, objects: s.nodes.map(node => node.key) });
  await api(s.b, '/api/sync/receive', { identity: s.identity, directory: s.exchange });
  await commit(s.b, await api(s.b, '/api/operations/prepare', { root: s.rb, operationId: randomUUID(), intent: { kind: 'cloneSync', identity: s.identity, objects: s.nodes.map(node => node.key) } }));
  s.before = await api(s.b, '/api/sync/binding', { root: s.rb });
  if (id >= 6) {
    assert.equal(s.before.partial, true);
    assert.ok(!s.before.installedObjects.some(key => key.kind === 'rules'));
    await assert.rejects(stat(path.join(s.rb, '.weftext-rules')), { code: 'ENOENT' });
    s.rulesBytes = await readFile(path.join(s.ra, '.weftext-rules'));
    s.rulesSha256 = createHash('sha256').update(s.rulesBytes).digest('hex');
    s.originalPhysical = {};
    for (const node of s.nodes) s.originalPhysical[node.name] = {
      directory: await physicalIdentity(path.join(s.rb, node.name)),
      source: await physicalIdentity(path.join(s.rb, node.name, `${node.name}.typ`)),
    };
    s.observations.push({ point: 'real-partial-clone-without-rules', binding: s.before, rulesAbsent: true, senderRulesSha256: s.rulesSha256, physical: s.originalPhysical });
  }
  const right = s.nodes[1];
  const rightOpened = await api(s.b, '/api/open', { root: s.rb, path: path.join(s.rb, right.name, `${right.name}.typ`) });
  s.rightDraft = right.source + '\n右节点个人草稿仍保留';
  await api(s.b, '/api/draft', { sessionId: rightOpened.sessionId, baseId: rightOpened.baseId, source: s.rightDraft });
  const port = s.b.port; await stop(s.b);
  s.b = await start(folder, 'b-ui', ['--data-dir', s.db, '--port', port, '--open', path.join(s.rb, 'Left', 'Left.typ'), '--root', s.rb]);
  s.context = await browser.newContext({ viewport: { width: 1500, height: 1050 } }); s.page = await s.context.newPage(); s.page.setDefaultTimeout(30000);
  s.page.on('pageerror', error => result.pageErrors.push(`case-${id}: ${String(error)}`));
  s.page.on('request', request => {
    const route = new URL(request.url()).pathname;
    if (route.startsWith('/api/')) s.requests.push({ route, body: request.postData() ? request.postDataJSON() : null });
  });
  s.page.on('response', response => {
    const route = new URL(response.url()).pathname;
    if (!route.startsWith('/api/')) return;
    const record = { route, status: response.status(), body: response.request().postData() ? response.request().postDataJSON() : null };
    s.responses.push(record);
    if (response.status() < 400) return;
    const read = response.json().then(value => { record.error = value.error ?? value; }).catch(error => { record.bodyReadError = String(error); });
    s.responseReads.add(read); read.finally(() => s.responseReads.delete(read));
  });
  await s.page.goto(`${s.b.base}/#token=${s.b.token}`); s.editor = await sourceEditor(s.page);
  assert.equal(await sourceText(s.editor), s.nodes[0].source);
  s.leftDraft = s.nodes[0].source + '\n左节点未保存稿 中文😀';
  await enterDraft(s.page, s.editor, s.leftDraft);
  await s.editor.evaluate(element => window.__compoundOriginalEditor = element);
  return s;
}
async function produce(s) {
  for (const node of s.nodes) {
    const request = { root: s.ra, operationId: randomUUID(), intent: { kind: 'renameNode', source: path.join(s.ra, node.name), name: node.nextName } };
    const plan = await api(s.a, '/api/operations/prepare', request), receipt = await commit(s.a, plan);
    const opened = await api(s.a, '/api/open', { root: s.ra, path: path.join(s.ra, node.nextName, `${node.nextName}.typ`) });
    node.afterSource = opened.source + `\nA 保存后的准确 ${node.nextName} 内容\n`;
    const saveRequest = { sessionId: opened.sessionId, baseId: opened.baseId, source: node.afterSource, operationId: randomUUID() };
    const saved = await api(s.a, '/api/save', saveRequest);
    s.producers.push({ request, plan, receipt, saveRequest, saved });
  }
  await api(s.a, '/api/sync/export', { root: s.ra, directory: s.exchange, objects: s.selectionObjects ?? s.nodes.map(node => node.key) });
}
async function produceNewParent(s) {
  const creation = { root: s.ra, parent: s.ra, name: 'NewParent', operationId: randomUUID() };
  const created = createdDocument(await api(s.a, '/api/note/create', creation));
  const parentSource = created.source + '\n= 真实新父节点\n\nA 新父正文 中文😀\n';
  const saveRequest = { sessionId: created.sessionId, baseId: created.baseId, source: parentSource, operationId: randomUUID() };
  const saved = await api(s.a, '/api/save', saveRequest);
  s.newParent = { name: creation.name, relative: path.join(creation.name, `${creation.name}.typ`), nodeId: created.nodeId, key: { kind: 'node', id: created.nodeId }, source: parentSource };
  s.newParent.senderPhysical = {
    directory: await physicalIdentity(path.join(s.ra, creation.name)),
    source: await physicalIdentity(path.join(s.ra, s.newParent.relative)),
  };
  s.producers.push({ creation, created, saveRequest, saved });
  const left = s.nodes[0]; left.nextName = left.name; left.afterRelative = path.join(creation.name, left.name, `${left.name}.typ`);
  for (const node of s.nodes) {
    const intent = node === left
      ? { kind: 'moveNode', source: path.join(s.ra, node.name), parent: path.join(s.ra, creation.name) }
      : { kind: 'renameNode', source: path.join(s.ra, node.name), name: node.nextName };
    const request = { root: s.ra, operationId: randomUUID(), intent };
    const plan = await api(s.a, '/api/operations/prepare', request), receipt = await commit(s.a, plan);
    const opened = await api(s.a, '/api/open', { root: s.ra, path: path.join(s.ra, afterRelative(node)) });
    assert.equal(opened.nodeId, node.nodeId);
    node.afterSource = opened.source + `\nA 新父组合保存 ${node.name} 中文😀\n`;
    const saveRequest = { sessionId: opened.sessionId, baseId: opened.baseId, source: node.afterSource, operationId: randomUUID() };
    const saved = await api(s.a, '/api/save', saveRequest);
    s.producers.push({ request, plan, receipt, saveRequest, saved });
  }
  s.selectionObjects = [...s.nodes.map(node => node.key), s.newParent.key, { kind: 'rules' }];
  await api(s.a, '/api/sync/export', { root: s.ra, directory: s.exchange, objects: s.selectionObjects });
}
async function rulesInstalled(s) {
  const binding = await api(s.b, '/api/sync/binding', { root: s.rb });
  assert.ok(binding.installedObjects.some(key => key.kind === 'rules'));
  const history = await api(s.b, '/api/sync/history', { identity: s.identity, object: { kind: 'rules' } });
  assert.equal(history.coverage, 'complete'); assert.ok(history.versions.length);
  for (const version of history.versions) {
    const file = await api(s.b, '/api/sync/file', { identity: s.identity, object: { kind: 'rules' }, revision: version.revision, path: '.weftext-rules' });
    assert.deepEqual(Buffer.from(file.text, 'utf8'), s.rulesBytes);
  }
  s.observations.push({ point: 'rules-installed-in-original-compound', rulesSha256: s.rulesSha256, versions: history.versions.map(version => version.revision), operationId: s.originalRequest.operationId });
}
async function originalPhysicalObjectsRetained(s) {
  for (const node of s.nodes) {
    const source = path.join(s.rb, afterRelative(node));
    assert.deepEqual(await physicalIdentity(path.dirname(source)), s.originalPhysical[node.name].directory);
    assert.deepEqual(await physicalIdentity(source), s.originalPhysical[node.name].source);
  }
  assert.deepEqual(await readFile(path.join(s.rb, '.weftext-rules')), s.rulesBytes);
  assert.notDeepEqual(await physicalIdentity(path.join(s.rb, '.weftext-rules')), await physicalIdentity(path.join(s.ra, '.weftext-rules')));
}
async function newParentInstalled(s) {
  const parent = await api(s.b, '/api/data/open', { root: s.rb, entityKey: `weftext-node://${s.identity.workspaceId}/${s.newParent.nodeId}` });
  assert.equal(parent.nodeId, s.newParent.nodeId); assert.equal(parent.workspaceId, s.identity.workspaceId);
  assert.equal(parent.source, s.newParent.source); assert.equal(path.normalize(parent.path), path.join(s.rb, s.newParent.relative));
  const binding = await api(s.b, '/api/sync/binding', { root: s.rb });
  assert.ok(binding.installedObjects.some(key => objectKey(key) === objectKey(s.newParent.key)));
  s.observations.push({ point: 'new-parent-portable-identity', reference: `weftext-node://${s.identity.workspaceId}/${s.newParent.nodeId}`, source: parent.source, path: parent.path });
}
async function finalNewParent(s, checkTrees) {
  await checkTrees(); await originalPhysicalObjectsRetained(s);
  assert.equal(await readFile(path.join(s.rb, s.newParent.relative), 'utf8'), s.newParent.source);
  assert.notDeepEqual(await physicalIdentity(path.join(s.rb, s.newParent.name)), s.newParent.senderPhysical.directory);
  assert.notDeepEqual(await physicalIdentity(path.join(s.rb, s.newParent.relative)), s.newParent.senderPhysical.source);
  if (s.materializedNewParent) {
    assert.deepEqual(await physicalIdentity(path.join(s.rb, s.newParent.name)), s.materializedNewParent.directory);
    assert.deepEqual(await physicalIdentity(path.join(s.rb, s.newParent.relative)), s.materializedNewParent.source);
    assert.deepEqual(await physicalIdentity(path.join(s.rb, '.weftext-rules')), s.materializedNewParent.rules);
  }
}
async function receive(s) {
  const panel = await sync(s.page);
  await panel.getByLabel('交换目录路径（新目录或已配对目录）', { exact: true }).fill(s.exchange);
  await clickDone(panel, '接收交换目录');
  if (s.selectionObjects) await selectedObjects(panel, s.selectionObjects); else await onlyNodes(panel, s.nodes);
  return panel;
}
async function prepare(s, panel, options) {
  const tap = await responseOnce(s.page, '/api/operations/prepare', options);
  await panel.getByRole('button', { name: '预览应用所选对象', exact: true }).click();
  const observed = await tap.wait(); await tap.dispose();
  assert.equal(observed.request.intent.kind, 'applySync'); assert.equal(observed.request.root, s.rb);
  assert.deepEqual(new Set(observed.request.intent.objects.map(objectKey)), new Set((s.selectionObjects ?? s.nodes.map(node => node.key)).map(objectKey)));
  s.originalRequest = observed.request;
  if (!options?.drop && observed.status === 200) {
    await panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor();
    assert.equal(observed.response.operationId, observed.request.operationId); assert.equal(observed.response.status, 'prepared');
    assert.ok(observed.response.planId);
    for (const node of s.nodes) assert.ok(observed.response.changes.some(change => change.path.includes(node.nextName)), `original Plan must include ${node.nextName}`);
  } else await panel.getByRole('status').filter({ hasText: '原操作记录保留' }).waitFor();
  return observed;
}
async function apply(s, panel, options) {
  const tap = await responseOnce(s.page, '/api/operations/commit', options);
  await panel.getByRole('button', { name: '确认应用此预览', exact: true }).click();
  const observed = await tap.wait(); await tap.dispose();
  if (options?.drop || observed.status !== 200) await panel.getByRole('status').filter({ hasText: '原操作记录保留' }).waitFor();
  else await panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor();
  return observed;
}
async function restartB(s, committed) {
  const port = s.b.port; await stop(s.b);
  const left = s.nodes[0], relative = committed ? afterRelative(left) : path.join(left.name, `${left.name}.typ`);
  s.b = await start(s.folder, 'b-restarted', ['--data-dir', s.db, '--port', port, '--open', path.join(s.rb, relative), '--root', s.rb]);
  // A new hash alone is same-document navigation and does not consume the
  // restarted program's token. Change the query, preserving the true origin.
  await s.page.goto(`${s.b.base}/?restart=${randomUUID()}#token=${s.b.token}`); s.editor = await sourceEditor(s.page);
  assert.equal(await sourceText(s.editor), s.leftDraft);
  return sync(s.page);
}
async function assertDraftAndUndo(s) {
  assert.equal(await s.page.evaluate(() => window.__compoundOriginalEditor?.isConnected), true);
  assert.equal(await s.editor.evaluate(element => element === window.__compoundOriginalEditor), true);
  assert.equal(await sourceText(s.editor), s.leftDraft);
  for (const [keys, expected] of [['ControlOrMeta+z', s.nodes[0].source], ['ControlOrMeta+Shift+z', s.leftDraft]]) {
    const persisted = s.page.waitForResponse(response => response.url().endsWith('/api/draft') && response.request().postDataJSON()?.source === expected && response.ok(), { timeout: 30000 });
    persisted.catch(() => {}); await s.editor.click(); await s.editor.press(keys);
    await until(() => sourceText(s.editor), source => source === expected, `shared Undo ${keys}`); await persisted;
    assert.equal(await s.editor.evaluate(element => document.activeElement === element), true);
  }
}
async function verifyApplied(s) {
  const binding = await api(s.b, '/api/sync/binding', { root: s.rb });
  for (const node of s.nodes) {
    const reference = `weftext-node://${s.identity.workspaceId}/${node.nodeId}`;
    const current = await api(s.b, '/api/data/open', { root: s.rb, entityKey: reference });
    assert.equal(path.normalize(current.path), path.join(s.rb, afterRelative(node)));
    assert.equal(current.source, node.afterSource);
    assert.equal(current.recoveredDraft, node.name === 'Left' ? s.leftDraft : s.rightDraft);
    const installed = binding.installedVersions.find(([key]) => key.kind === 'node' && key.id === node.nodeId);
    assert.ok(installed && installed[1] !== s.before.installedVersions.find(([key]) => key.id === node.nodeId)?.[1]);
    const history = await api(s.b, '/api/sync/history', { identity: s.identity, object: node.key });
    const beforeRevision = s.before.installedVersions.find(([key]) => key.id === node.nodeId)[1];
    assert.ok(history.versions.some(version => version.revision === beforeRevision), 'original installed history must remain');
  }
}
async function sidebarHasCurrentPaths(s) {
  // The visible title is author-controlled and need not change with a leaf
  // rename. Assert the actual refreshed navigation coordinates in row titles.
  const wanted = [...s.nodes.map(node => path.join(s.rb, afterRelative(node))), ...(s.newParent ? [path.join(s.rb, s.newParent.relative)] : [])].map(value => value.replaceAll('\\', '/').toLowerCase());
  await until(() => s.page.locator('.tree-row[title]').evaluateAll(rows => rows.map(row => row.title.replaceAll('\\', '/').toLowerCase())),
    titles => wanted.every(value => titles.some(title => title.split('\n')[0] === value)), 'sidebar current paths');
}
async function finalTrees(s, applied, externalRight, occupant) {
  // Read owned physical fixtures only after both Core processes have stopped.
  for (const node of s.nodes) {
    assert.equal(await readFile(path.join(s.ra, afterRelative(node)), 'utf8'), node.afterSource);
    const name = applied ? node.nextName : node.name;
    const expected = applied ? s.receiverSources?.[node.name] ?? node.afterSource : node.name === 'Right' && externalRight ? externalRight : node.source;
    assert.equal(await readFile(path.join(s.rb, applied ? afterRelative(node) : path.join(name, `${name}.typ`)), 'utf8'), expected);
    const absent = applied ? node.name : node.nextName;
    if (!(occupant && absent === occupant.name)) await assert.rejects(stat(path.join(s.rb, absent)), { code: 'ENOENT' });
  }
  if (occupant) assert.equal(await readFile(occupant.path, 'utf8'), occupant.source);
}
function assertNoOldConsumers(s, from, sessionId) {
  const sourceRoutes = new Set(['/api/render', '/api/editor-map', '/api/paged', '/api/query/embedded/discover', '/api/query/embedded/run', '/api/knowledge/read', '/api/knowledge/proofread', '/api/data/inspect', '/api/resources/list']);
  const stale = s.requests.slice(from).filter(call => sourceRoutes.has(call.route) && (call.body?.sessionId ?? call.body?.context?.sessionId) === sessionId);
  assert.deepEqual(stale, [], 'known moved writer must not initiate old-path consumer requests before explicit comparison');
}
async function confirmedMergeAndSave(s, originalSession) {
  await s.page.getByRole('button', { name: '比较磁盘内容', exact: true }).click();
  const comparison = s.page.getByRole('dialog').filter({ has: s.page.getByRole('heading', { name: '比较草稿与磁盘内容', exact: true }) });
  await comparison.waitFor();
  const merged = s.nodes[0].afterSource + s.leftDraft.slice(s.nodes[0].source.length) + '\n明确合并后继续保存\n';
  await comparison.getByLabel('合并后的草稿（可编辑）', { exact: true }).fill(merged);
  const rendered = s.page.waitForResponse(response => response.url().endsWith('/api/render') && response.request().postDataJSON()?.source === merged && response.ok(), { timeout: 30000 }); rendered.catch(() => {});
  const knowledge = s.page.waitForResponse(response => response.url().endsWith('/api/knowledge/read') && response.request().postDataJSON()?.context?.source === merged && response.ok(), { timeout: 30000 }); knowledge.catch(() => {});
  await comparison.getByRole('button', { name: '应用合并后的草稿', exact: true }).click(); await comparison.waitFor({ state: 'detached' });
  const renderResponse = await rendered, output = await renderResponse.json(), newSession = renderResponse.request().postDataJSON().sessionId;
  assert.equal(output.ok, true); assert.notEqual(newSession, originalSession);
  assert.equal((await knowledge).request().postDataJSON().context.sessionId, newSession);
  assert.equal(await sourceText(s.editor), merged);
  assert.equal(await s.editor.evaluate(element => element === window.__compoundOriginalEditor), true);
  assert.equal(await s.page.getByRole('button', { name: '比较磁盘内容', exact: true }).count(), 0);
  const savedResponse = s.page.waitForResponse(response => response.url().endsWith('/api/save') && response.request().postDataJSON()?.source === merged && response.ok(), { timeout: 30000 }); savedResponse.catch(() => {});
  await s.page.getByRole('button', { name: '保存', exact: true }).click();
  const saved = await savedResponse; assert.equal(saved.request().postDataJSON().sessionId, newSession);
  const actual = await api(s.b, '/api/data/open', { root: s.rb, entityKey: `weftext-node://${s.identity.workspaceId}/${s.nodes[0].nodeId}` });
  assert.equal(actual.source, merged); assert.equal(await sourceText(s.editor), merged);
  s.receiverSources = { Left: merged };
  s.observations.push({ point: 'explicit-merge-restores-current-consumers-and-save', originalSession, newSession, source: merged, renderStatus: renderResponse.status(), renderOk: output.ok, saveStatus: saved.status() });
  await s.page.screenshot({ path: path.join(s.folder, 'success-compound-confirmed-merge.png'), fullPage: true });
}
async function runCase(id, description, action) {
  const s = await setup(id); let finalCheck;
  try { finalCheck = await action(s); }
  catch (error) {
    await s.page.screenshot({ path: path.join(s.folder, 'failure.png'), fullPage: true }).catch(() => {}); throw error;
  } finally {
    for (const tap of [...taps]) await tap.dispose().catch(error => result.routeErrors.push(`cleanup: ${String(error)}`));
    await bounded(Promise.all([...s.responseReads]), 5000, 'diagnostic response bodies');
    await s.context.close(); await stop(s.a); await stop(s.b);
    await writeFile(path.join(s.folder, 'fixture.json'), JSON.stringify({ nodes: s.nodes, identity: s.identity, before: s.before, producers: s.producers, originalRequest: s.originalRequest, rulesSha256: s.rulesSha256, originalPhysical: s.originalPhysical, newParent: s.newParent }, null, 2));
    await writeFile(path.join(s.folder, 'browser-requests.json'), JSON.stringify(s.requests, null, 2));
    await writeFile(path.join(s.folder, 'browser-responses.json'), JSON.stringify(s.responses, null, 2));
    await writeFile(path.join(s.folder, 'observations.json'), JSON.stringify(s.observations, null, 2));
  }
  await finalCheck?.(); result.cases.push({ id, description, status: 'passed' });
}
const cases = [
  [1, 'Two real rename+save versions installed through one original ApplySync Plan; same editor/Draft/Undo, both private Drafts and histories retained; sidebar and current stable refs refreshed', async s => {
    await (s.producer ?? produce)(s); const panel = await receive(s), prepared = await prepare(s, panel);
    await s.inspectPrepared?.(prepared);
    await panel.screenshot({ path: path.join(s.folder, 'success-compound-preview.png') });
    const committed = await apply(s, panel); assert.equal(committed.response.status, 'committed');
    const consumerStart = s.requests.length, originalSession = s.requests.find(call => call.route === '/api/draft' && call.body.source === s.leftDraft).body.sessionId;
    assert.deepEqual(committed.request, { operationId: prepared.response.operationId, planId: prepared.response.planId });
    await clickDone(panel, '结束已确认操作'); await closePanel(panel);
    assert.match(await s.page.locator('body').innerText(), /同步已改变磁盘版本/);
    s.observations.push({ point: 'committed-before-Undo', text: await s.page.locator('body').innerText() });
    await s.page.screenshot({ path: path.join(s.folder, 'success-compound-requires-comparison.png'), fullPage: true });
    await assertDraftAndUndo(s);
    assert.match(await s.page.locator('body').innerText(), /请比较磁盘内容并明确合并/);
    await workspaceCommand(s.page, 'knowledge', '链接与批注');
    await s.page.locator('.knowledge-empty').waitFor();
    assertNoOldConsumers(s, consumerStart, originalSession);
    await s.page.getByRole('button', { name: '比较磁盘内容', exact: true }).click();
    const comparison = s.page.getByRole('dialog').filter({ has: s.page.getByRole('heading', { name: '比较草稿与磁盘内容', exact: true }) });
    await comparison.waitFor();
    assert.equal(await comparison.getByLabel('草稿原始版本', { exact: true }).inputValue(), s.nodes[0].source);
    assert.equal(await comparison.getByLabel('当前磁盘内容', { exact: true }).inputValue(), s.nodes[0].afterSource);
    assert.equal(await comparison.getByLabel('合并后的草稿（可编辑）', { exact: true }).inputValue(), s.leftDraft);
    s.observations.push({ point: 'explicit-read-only-comparison', originalBase: s.nodes[0].source, currentSaved: s.nodes[0].afterSource, personalDraft: s.leftDraft });
    await comparison.screenshot({ path: path.join(s.folder, 'success-compound-exact-comparison.png') });
    await comparison.getByRole('button', { name: '关闭并保留草稿', exact: true }).click();
    await comparison.waitFor({ state: 'detached' });
    assert.equal(await sourceText(s.editor), s.leftDraft); await verifyApplied(s);
    assertNoOldConsumers(s, consumerStart, originalSession);
    await sidebarHasCurrentPaths(s);
    s.observations.push({ point: 'after-original-Undo-Redo', text: await s.page.locator('body').innerText() });
    const writes = s.requests.filter(call => call.route === '/api/operations/prepare' || call.route === '/api/operations/commit');
    assert.equal(writes.length, 2); assert.equal(s.requests.filter(call => call.route === '/api/save').length, 0);
    assert.ok(s.responses.some(response => response.route === '/api/query/run' && response.status === 200), 'library Query refresh remains independently available');
    await s.page.screenshot({ path: path.join(s.folder, 'success-compound-draft.png'), fullPage: true });
    await confirmedMergeAndSave(s, originalSession);
    return () => finalTrees(s, true);
  }],
  [2, 'Lost actual prepared response: persisted exact original request survives same-origin program/browser restart, retries original ID and obtains identical Plan before explicit commit', async s => {
    await (s.producer ?? produce)(s); let panel = await receive(s), prepared = await prepare(s, panel, { drop: true });
    await s.inspectPrepared?.(prepared);
    const before = await pending(s.page); assert.equal(before.plan, undefined); assert.equal(before.attempted, false);
    assert.deepEqual(before.request, prepared.request);
    panel = await restartB(s, false); assert.equal((await pending(s.page)).request.operationId, prepared.request.operationId);
    const tap = await responseOnce(s.page, '/api/operations/prepare');
    await panel.getByRole('button', { name: '重试原准备请求', exact: true }).click();
    const retried = await tap.wait(); await tap.dispose();
    await panel.getByRole('status').filter({ hasText: /^操作已完成。$/ }).waitFor();
    assert.deepEqual(retried.request, prepared.request); assert.deepEqual(retried.response, prepared.response);
    assert.equal((await pending(s.page)).plan.planId, prepared.response.planId);
    await apply(s, panel); await verifyApplied(s);
    assert.equal(s.requests.filter(call => call.route === '/api/operations/commit').length, 1);
    await panel.screenshot({ path: path.join(s.folder, 'success-lost-prepare-original-plan.png') });
    return () => finalTrees(s, true);
  }],
  [3, 'Lost actual compound commit response: restart checks original operation/Plan only, both target trees already installed once and personal Drafts recovered without a new commit', async s => {
    await (s.producer ?? produce)(s); let panel = await receive(s), prepared = await prepare(s, panel);
    await s.inspectPrepared?.(prepared);
    const committed = await apply(s, panel, { drop: true }); assert.equal(committed.response.status, 'committed');
    await s.inspectCommitted?.(committed);
    const before = await pending(s.page); assert.equal(before.status, 'unknown'); assert.equal(before.attempted, true);
    panel = await restartB(s, true);
    const statusResponse = s.page.waitForResponse(response => response.url().endsWith('/api/operations/status') && response.request().postDataJSON()?.operationId === prepared.request.operationId, { timeout: 30000 }); statusResponse.catch(() => {});
    await clickDone(panel, '检查原操作结果'); const status = await (await statusResponse).json();
    assert.equal(status.status, 'committed'); assert.equal(status.planId, prepared.response.planId);
    assert.equal((await pending(s.page)).request.operationId, prepared.request.operationId);
    await verifyApplied(s); assert.equal(s.requests.filter(call => call.route === '/api/operations/commit').length, 1);
    assert.equal(s.requests.filter(call => call.route === '/api/operations/prepare').length, 1);
    const recovered = await api(s.b, '/api/sync/operation', { operationId: prepared.request.operationId });
    assert.deepEqual(recovered.request, prepared.request); assert.equal(recovered.plan.planId, prepared.response.planId); assert.equal(recovered.plan.status, 'committed');
    await panel.screenshot({ path: path.join(s.folder, 'success-lost-commit-original-receipt.png') });
    return () => finalTrees(s, true);
  }],
  [4, 'A real foreign destination blocks compound prepare before any effect; both original trees, occupied object and the original editor/Draft/Undo remain', async s => {
    const name = s.nodes[1].nextName;
    const occupant = createdDocument(await api(s.b, '/api/note/create', { root: s.rb, parent: s.rb, name, operationId: randomUUID() }));
    await produce(s); const panel = await receive(s), refused = await prepare(s, panel, { expected: 409 });
    assert.ok(refused.response.error?.code); assert.notEqual(refused.response.error.code, 'syncStructureRequired');
    assert.equal((await pending(s.page)).plan, undefined);
    assert.equal(s.requests.filter(call => call.route === '/api/operations/commit').length, 0);
    await panel.screenshot({ path: path.join(s.folder, 'success-occupied-refusal.png') });
    await closePanel(panel); await assertDraftAndUndo(s);
    return () => finalTrees(s, false, undefined, { ...occupant, name });
  }],
  [5, 'Real external content after preparing invalidates the single original commit before any relocation; both original trees and later external bytes remain, with editor/Draft/Undo protected', async s => {
    await produce(s); const panel = await receive(s), prepared = await prepare(s, panel);
    const right = s.nodes[1], external = right.source + '\n真实外部第三状态，不能覆盖😀';
    await writeFile(path.join(s.rb, right.name, `${right.name}.typ`), external, 'utf8');
    const refused = await apply(s, panel, { expected: 409 }); assert.ok(refused.response.error?.code);
    assert.notEqual(refused.response.error.code, 'syncStructureRequired');
    const status = await api(s.b, '/api/operations/status', { operationId: prepared.request.operationId });
    assert.equal(status.status, 'prepared'); assert.equal(status.planId, prepared.response.planId);
    assert.equal(s.requests.filter(call => call.route === '/api/operations/prepare').length, 1);
    assert.equal(s.requests.filter(call => call.route === '/api/operations/commit').length, 1);
    await panel.screenshot({ path: path.join(s.folder, 'success-external-refusal.png') });
    await closePanel(panel); await assertDraftAndUndo(s);
    return () => finalTrees(s, false, external);
  }],
  [6, 'Real partial clone without Rules receives sender Rules and two rename/save versions; one explicit Rules+Nodes Apply installs the exact Rules and both relocations, preserving Draft/Undo and explicit comparison consumers', async s => {
    s.selectionObjects = [...s.nodes.map(node => node.key), { kind: 'rules' }];
    s.inspectPrepared = async prepared => {
      assert.ok(prepared.response.changes.some(change => change.path.endsWith('.weftext-rules')));
      await assert.rejects(stat(path.join(s.rb, '.weftext-rules')), { code: 'ENOENT' });
      for (const node of s.nodes) await assert.rejects(stat(path.join(s.rb, node.nextName)), { code: 'ENOENT' });
    };
    const checkTrees = await cases[0][2](s); await rulesInstalled(s);
    return async () => { await checkTrees(); await originalPhysicalObjectsRetained(s); };
  }],
  [7, 'Real create/save makes a missing Node parent; one Apply installs that parent and Rules, moves an original Node into it and renames/updates the other; portable IDs, physical objects, Draft/Undo and explicit compare/merge/save survive', async s => {
    s.producer = produceNewParent;
    s.inspectPrepared = async prepared => {
      assert.equal(prepared.response.nodeMap[s.newParent.nodeId], s.newParent.nodeId);
      assert.ok(prepared.response.changes.some(change => change.path.includes(s.newParent.name)));
      await assert.rejects(stat(path.join(s.rb, s.newParent.name)), { code: 'ENOENT' });
      await assert.rejects(stat(path.join(s.rb, '.weftext-rules')), { code: 'ENOENT' });
    };
    const checkTrees = await cases[0][2](s); await rulesInstalled(s);
    await newParentInstalled(s);
    return () => finalNewParent(s, checkTrees);
  }],
  [8, 'New parent and Rules compound loses its actual prepare response: original persisted request retries the identical Plan after restart, then materializes the parent once and retains both personal Drafts and history', async s => {
    s.producer = produceNewParent;
    s.inspectPrepared = async prepared => {
      assert.equal(prepared.response.nodeMap[s.newParent.nodeId], s.newParent.nodeId);
      await assert.rejects(stat(path.join(s.rb, s.newParent.name)), { code: 'ENOENT' });
      await assert.rejects(stat(path.join(s.rb, '.weftext-rules')), { code: 'ENOENT' });
    };
    const checkTrees = await cases[1][2](s); await rulesInstalled(s); await newParentInstalled(s);
    return () => finalNewParent(s, checkTrees);
  }],
  [9, 'New parent and Rules compound loses its actual commit response: the original pending operation survives restart and status recovers the same committed receipt without reprepare or recommit; parent and child identities and personal Drafts remain', async s => {
    s.producer = produceNewParent;
    s.inspectPrepared = async prepared => {
      assert.equal(prepared.response.nodeMap[s.newParent.nodeId], s.newParent.nodeId);
      await assert.rejects(stat(path.join(s.rb, s.newParent.name)), { code: 'ENOENT' });
      await assert.rejects(stat(path.join(s.rb, '.weftext-rules')), { code: 'ENOENT' });
    };
    s.inspectCommitted = async () => {
      s.materializedNewParent = {
        directory: await physicalIdentity(path.join(s.rb, s.newParent.name)),
        source: await physicalIdentity(path.join(s.rb, s.newParent.relative)),
        rules: await physicalIdentity(path.join(s.rb, '.weftext-rules')),
      };
      s.observations.push({ point: 'original-materialized-new-parent-before-committed-restart', physical: s.materializedNewParent });
    };
    const checkTrees = await cases[2][2](s); await rulesInstalled(s); await newParentInstalled(s);
    return () => finalNewParent(s, checkTrees);
  }],
];
try {
  browser = await chromium.launch({ channel: 'chrome', headless: true }); result.browser = browser.version();
  for (const [id, description, action] of cases) if (selected.includes(id)) await runCase(id, description, action);
  assert.deepEqual(result.pageErrors, []); assert.deepEqual(result.routeErrors, []); result.completed = true;
} catch (error) {
  result.failure = String(error.stack ?? error); result.failedCase = activeScenario?.id;
  if (activeScenario?.page && !activeScenario.page.isClosed()) await activeScenario.page.screenshot({ path: path.join(activeScenario.folder, 'failure.png'), fullPage: true }).catch(() => {});
  throw error;
} finally {
  await browser?.close();
  for (const server of servers) await stop(server);
  result.ownedProcessesClosed = servers.every(server => server.child.exitCode !== null || server.child.signalCode !== null) && !browser?.isConnected();
  result.serverStderrBytes = servers.reduce((bytes, server) => bytes + Buffer.byteLength(server.stderr), 0);
  for (const [index, server] of servers.entries()) {
    await writeFile(path.join(server.folder, `server-${index}-${server.label}.stdout.log`), server.stdout);
    await writeFile(path.join(server.folder, `server-${index}-${server.label}.stderr.log`), server.stderr);
  }
  await writeFile(path.join(out, 'results.json'), stringifyServeJson(result, 2));
  console.log(JSON.stringify({ out, completed: result.completed, cases: result.cases, pageErrors: result.pageErrors, routeErrors: result.routeErrors, ownedProcessesClosed: result.ownedProcessesClosed }));
}
