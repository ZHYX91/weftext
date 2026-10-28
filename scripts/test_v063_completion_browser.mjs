import { spawnServe, serveReady, stopServe, stringifyServeJson } from './_cli_serve.mjs';
import { workspaceCommand, workspaceMode } from './_workspace_browser.mjs';
import { createdDocument } from './_creation_result.mjs';
// Actual final-candidate daily task chain. Every product view comes from the
// supplied executable's embedded frontend and actual HTTP/Core fixture data.
// Headless keyboard/text events are not OS IME or reference-host acceptance.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile, realpath } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import { createRequire } from 'node:module';
import { deflateSync } from 'node:zlib';
import path from 'node:path';

assert.equal(process.argv.length, 4, 'Usage: node scripts/test_v063_completion_browser.mjs CANDIDATE_EXE NEW_OWNED_EVIDENCE_DIRECTORY');
assert.ok(!process.env.WEFTEXT_DEV_FRONTEND, 'Completion acceptance must use the embedded candidate frontend');
const exe = await realpath(path.resolve(process.argv[2])), out = path.resolve(process.argv[3]);
assert.ok(out !== path.parse(out).root && out !== path.dirname(exe), 'Evidence must be a new, explicitly owned subdirectory');
await mkdir(out, { recursive: false });
const require = createRequire(import.meta.url), { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const result = {
  scope: 'Actual embedded candidate, owned local library; hierarchy/Query/shortcuts, graph/heading navigation, sibling order and lazy media',
  harness: path.resolve(process.argv[1]), harnessSha256: sha(await readFile(new URL(import.meta.url))),
  exe, sha256: sha(await readFile(exe)), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(),
  startedAt: new Date().toISOString(), completed: false, cases: [], pageErrors: [], httpErrors: [], requests: [], snapshots: [],
  limits: ['No OS IME, assistive technology, touch, real Office host or reference-software parity acceptance.',
    'An ordinary complete-task chain, not the 5,000-tag/near-budget/over-budget scale matrix.',
    'Local owner scope; independently authorized hosted and synchronization cases run through their own product suites.'],
};
const root = path.join(out, 'TEST-library'), data = path.join(out, 'private-state');
let child, browser, context, page, base, token, stderr = '', phase = 'setup';
let image, plain, parent, farImage, savedImage, savedParent, dirtyImage, dirtyParent, relationSource, relationTarget;
const queryReceipts = [], knowledgeReceipts = [], renderReceipts = [], previewRequests = [], mutations = [];
const observedRequests = new Map(), pageClients = new WeakMap(); let observedClient = 0;
let caseStartedAt = performance.now();

async function until(check, label = 'condition', timeout = 30000) {
  const deadline = Date.now() + timeout; let last;
  while (Date.now() < deadline) { try { const value = await check(); if (value) return value; } catch (error) { last = error; } await pause(100); }
  throw last ?? new Error('Timed out: ' + label);
}
async function start(args = []) {
  child = spawnServe(exe, ['--data-dir', data, ...args], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  let stdout = ''; child.stdout.on('data', bytes => stdout += bytes); child.stderr.on('data', bytes => stderr += bytes);
  await until(() => { const match = serveReady(child); if (match) { base = match.base; token = match.token; return true; } if (child.exitCode !== null) throw new Error('Owned candidate exited: ' + stderr); return false; }, 'owned local candidate start');
}
async function stop() { await stopServe(child); }
async function api(route, body) {
  const response = await fetch(base + route, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': token }, body: JSON.stringify(body) });
  const value = await response.json(); assert.equal(response.status, 200, `${route}: ${JSON.stringify(value)}`); return value;
}
async function commit(prepared) {
  const preview = prepared.preview ?? prepared;
  return api('/api/operations/commit', { operationId: preview.operationId, planId: preview.planId });
}
async function save(doc, source) {
  const operationId = randomUUID();
  const receipt = await api('/api/save', { sessionId: doc.sessionId, baseId: doc.baseId, source, operationId });
  assert.equal(receipt.operationId, operationId);
  assert.equal(receipt.status, 'saved');
  assert.equal(receipt.recoveryIssue, undefined);
  assert.equal(receipt.documentError, undefined);
  assert.equal(receipt.committedSource, source);
  assert.equal(receipt.documentSourceMatchesCommit, true);
  assert.equal(receipt.document.sessionId, doc.sessionId);
  assert.equal(receipt.document.path, doc.path);
  assert.equal(receipt.document.root, doc.root);
  assert.equal(receipt.document.nodeId, doc.nodeId);
  assert.equal(receipt.document.workspaceId, doc.workspaceId);
  assert.equal(receipt.document.source, source);
  assert.equal(receipt.document.recoveredDraftConflict, false);
  return receipt.document;
}
async function note(name, container = root, body = '') {
  const doc = createdDocument(await api('/api/note/create', { root, parent: container, name, operationId: randomUUID() }));
  return save(doc, doc.source + '\n\n' + body);
}
// Same small owned PNG geometry as the existing product HTTP fixtures.
function png() {
  function crc(bytes) { let value = 0xffffffff; for (const byte of bytes) { value ^= byte; for (let bit = 0; bit < 8; bit++) value = (value >>> 1) ^ (value & 1 ? 0xedb88320 : 0); } return (value ^ 0xffffffff) >>> 0; }
  function chunk(kind, bytes) { const type = Buffer.from(kind), length = Buffer.alloc(4), checksum = Buffer.alloc(4); length.writeUInt32BE(bytes.length); checksum.writeUInt32BE(crc(Buffer.concat([type, bytes]))); return Buffer.concat([length, type, bytes, checksum]); }
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(2, 0); ihdr.writeUInt32BE(2, 4); ihdr[8] = 8; ihdr[9] = 2;
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', deflateSync(Buffer.from([0, 255, 0, 0, 0, 255, 0, 0, 255, 0, 0, 0, 255, 0]))), chunk('IEND', Buffer.alloc(0))]);
}
async function attachImage(doc) {
  const fresh = await api('/api/open', { sessionId: doc.sessionId });
  assert.equal(fresh.sessionId, doc.sessionId);
  assert.equal(fresh.path, doc.path);
  assert.equal(fresh.baseId, doc.baseId);
  assert.equal(fresh.source, doc.source);
  assert.equal(fresh.recoveredDraftConflict, false);
  await commit(await api('/api/resources/prepare-import', { sessionId: fresh.sessionId, name: 'TEST-pixels.png', bytes: png().toString('base64'), operationId: randomUUID() }));
  return save(fresh, fresh.source + '\n\n#image("TEST-pixels.png", width: 24pt)\n');
}
function observe(target) {
  const client = ++observedClient;
  pageClients.set(target, client);
  target.on('pageerror', error => result.pageErrors.push({ phase, error: String(error) }));
  target.on('request', request => {
    const route = new URL(request.url()).pathname; if (!route.startsWith('/api/')) return;
    const body = request.method() === 'POST' ? request.postDataJSON() : undefined;
    if (route === '/api/navigation/preview') previewRequests.push({ phase, ...body });
    if (/\/(prepare|commit|cancel)$/.test(route) || route.startsWith('/api/navigation/') && /-tag$/.test(route)) mutations.push({ phase, route, body });
    const entry = { id: `${client}.${result.requests.length + 1}`, client, phase, route, method: request.method(), at: new Date().toISOString(), startedMs: performance.now(), body: body === undefined ? undefined : JSON.parse(JSON.stringify(body, (key, value) => /token|password|authorization|credential|secret/i.test(key) ? '[REDACTED]' : value)) };
    observedRequests.set(request, entry); result.requests.push(entry);
  });
  target.on('requestfinished', request => { const entry = observedRequests.get(request); if (entry) { entry.finishedAt = new Date().toISOString(); entry.elapsedMs = performance.now() - entry.startedMs; entry.timing = request.timing(); } });
  target.on('requestfailed', request => { const entry = observedRequests.get(request); if (entry) { entry.failedAt = new Date().toISOString(); entry.elapsedMs = performance.now() - entry.startedMs; entry.failure = request.failure()?.errorText; entry.timing = request.timing(); } });
  target.on('response', async response => {
    const route = new URL(response.url()).pathname, capturedPhase = phase;
    if (!route.startsWith('/api/')) return;
    const entry = observedRequests.get(response.request()); if (entry) { entry.status = response.status(); entry.responseAt = new Date().toISOString(); entry.responseElapsedMs = performance.now() - entry.startedMs; entry.responsePhase = capturedPhase; entry.timing = response.request().timing(); }
    try {
      if (response.status() >= 400) result.httpErrors.push({ phase: capturedPhase, route, status: response.status(), body: await response.text() });
      if (route === '/api/query/run' && response.ok()) { const value = await response.json(); queryReceipts.push({ phase: capturedPhase, requestId: entry?.id, request: response.request().postDataJSON(), result: value }); if (entry) entry.bodyRecordedAt = new Date().toISOString(); }
      if (route === '/api/knowledge/read' && response.ok()) {
        const value = await response.json();
        knowledgeReceipts.push({ phase: capturedPhase, client: entry?.client, requestId: entry?.id, request: response.request().postDataJSON(), result: value });
        if (entry) entry.bodyRecordedAt = new Date().toISOString();
      }
      if (route === '/api/render' && response.ok()) {
        const value = await response.json();
        renderReceipts.push({ phase: capturedPhase, requestId: entry?.id, request: response.request().postDataJSON(),
          result: { ok: value.ok, headingStructure: value.headingStructure, diagnostics: value.diagnostics } });
      }
    } catch (error) { if (!target.isClosed()) (result.observerErrors ??= []).push(String(error)); }
  });
}
async function failureSnapshot(label) {
  const receipt = { label, at: new Date().toISOString(), backendPid: child?.pid, pages: [], pendingRequests: [] };
  for (const [request, entry] of observedRequests) if (!entry.finishedAt && !entry.failedAt) { let timing; try { timing = request.timing(); } catch { timing = null; } receipt.pendingRequests.push({ ...entry, observedElapsedMs: performance.now() - entry.startedMs, timing }); }
  for (const [index, target] of (context?.pages() ?? []).entries()) {
    if (target.isClosed()) continue;
    const item = { index, url: target.url().replace(/#.*$/, '') };
    try {
      item.state = await target.evaluate(() => {
        const panel = document.querySelector('.query-panel'), source = document.querySelector('.source-host .cm-content'), view = source?.cmTile?.root?.view;
        const buttons = Array.from(panel?.querySelectorAll('button') ?? []).filter(button => ['查询', '刷新'].includes(button.textContent.trim())).map(button => ({ text: button.textContent, disabled: button.disabled, connected: button.isConnected, type: button.type, formConnected: button.form?.isConnected ?? false, formText: button.form?.innerText }));
        return { title: document.title, focused: { tag: document.activeElement?.tagName, label: document.activeElement?.getAttribute('aria-label'), text: document.activeElement?.textContent?.slice(0, 200) }, query: panel ? { visible: Boolean(panel.getClientRects().length), text: panel.innerText, buttons, chips: Array.from(panel.querySelectorAll('.query-chip')).map(chip => chip.textContent), inputs: Array.from(panel.querySelectorAll('input,select')).map(input => ({ tag: input.tagName, label: input.getAttribute('aria-label'), type: input.type, value: input.value, disabled: input.disabled })) } : null, editor: view?.contentDOM === source ? { source: view.state.doc.toString(), selection: { from: view.state.selection.main.from, to: view.state.selection.main.to } } : null, status: document.querySelector('.status-right')?.textContent };
      });
      if (item.state.editor) item.state.editor.sourceSha256 = sha(item.state.editor.source);
      await target.screenshot({ path: path.join(out, `failure-${label}-client-${index}.png`), fullPage: true });
      await writeFile(path.join(out, `failure-${label}-client-${index}.txt`), await target.locator('body').innerText());
    } catch (error) { item.observationError = String(error); }
    receipt.pages.push(item);
  }
  await writeFile(path.join(out, `failure-${label}-trace.json`), JSON.stringify(receipt, null, 2)); return receipt;
}
const click = (name, host = page) => host.getByRole('button', { name, exact: true }).click();
const cm = () => page.locator('.source-host .cm-content');
const queryPanel = () => page.locator('.query-panel');
const knowledge = () => page.getByLabel('知识与批注', { exact: true });
const nodesView = () => page.locator('#workspace-view-nodes');
const contents = () => nodesView().getByRole('region', { name: '节点内容', exact: true });
async function source() { return cm().evaluate(element => { const view = element.cmTile?.root?.view; if (view?.contentDOM !== element) throw new Error('Actual retained editor unavailable'); return view.state.doc.toString(); }); }
async function expectSource(expected) { await until(async () => { assert.equal(await source(), expected); return true; }, 'exact current author Draft'); }
async function disk(file) { for (let attempt = 0; attempt < 20; attempt++) { try { return await readFile(file, 'utf8'); } catch (error) { if (!['EBUSY', 'EACCES'].includes(error.code) || attempt === 19) throw error; (result.fileReadRetries ??= []).push({ phase, file, attempt, code: error.code }); await pause(100); } } }
async function command(label) { await page.keyboard.press('Control+Shift+p'); const dialog = page.getByRole('dialog', { name: '命令', exact: true }); await dialog.getByRole('searchbox').fill(label); await dialog.getByRole('searchbox').press('Enter'); }
async function knowledgeCommand(label, tab) { await command(label); await until(async () => { if (tab === '标题结构') return page.locator('.outline-view').isVisible(); const control = knowledge().getByRole('tab', { name: tab, exact: true }); return await control.isVisible() && await control.getAttribute('aria-selected') === 'true'; }, 'explicit knowledge command visibly selects ' + tab); }
async function quick(name) { await page.keyboard.press('Control+p'); const dialog = page.getByRole('dialog', { name: '快捷打开', exact: true }); await dialog.getByRole('searchbox').fill(name.endsWith('.typ') ? name : name + '.typ'); await dialog.getByRole('searchbox').press('Enter'); }
async function panelDetails(label, host = queryPanel()) { const summary = host.getByText(label, { exact: true }); const details = summary.locator('..'); if (!await details.evaluate(element => element.open)) await summary.click(); return details; }
async function showQuery() { if (!await queryPanel().isVisible()) await workspaceCommand(page, 'knowledge', '查询与视图'); }
async function pass(message) { const now = performance.now(); result.cases.push({ phase, message, elapsedMs: Math.round((now - caseStartedAt) * 100) / 100, input: 'Actual DOM mouse and keyboard; automation and observable waits included' }); caseStartedAt = now; await writeFile(path.join(out, 'progress.json'), stringifyServeJson(result, 2)); console.log('PASS', message); }
async function createTag(name, parentRef = '', assign = false) {
  const navigation = await panelDetails('登记标签与目录入口');
  await click('读取登记标签', navigation); await navigation.getByLabel('新标签父级', { exact: true }).waitFor();
  await navigation.getByLabel('新标签显示名称', { exact: true }).fill(name);
  await navigation.getByLabel('新标签父级 · 搜索名称或身份', { exact: true }).fill(parentRef);
  await navigation.getByLabel('新标签父级', { exact: true }).selectOption(parentRef);
  const prepared = page.waitForResponse(response => new URL(response.url()).pathname === '/api/navigation/create-tag');
  await click(assign ? '新建并赋给当前已保存笔记' : '新建登记标签', navigation);
  const proposal = await prepared; assert.equal(proposal.status(), 200, await proposal.text()); const target = await proposal.json();
  const committed = page.waitForResponse(response => new URL(response.url()).pathname === '/api/operations/commit');
  await click('执行已预览的登记', page.locator('dialog[open]'));
  const response = await committed; assert.equal(response.status(), 200, await response.text());
  await until(async () => !(await page.locator('dialog[open]').count()) && (await api('/api/navigation/tags', { root })).tags.some(tag => tag.tagRef === target.targetRef));
  return (await api('/api/navigation/tags', { root })).tags.find(tag => tag.tagRef === target.targetRef);
}
async function tagRow(tag) {
  const navigation = await panelDetails('登记标签与目录入口');
  await navigation.getByLabel('标签树搜索名称或身份', { exact: true }).fill(tag.tagRef);
  const row = navigation.locator('.registered-tag-row'); await until(() => row.count().then(n => n === 1)); return row;
}
async function moveTag(tag, parentTag, cancel = false) {
  await click('换父／移至根', await tagRow(tag)); const picker = page.locator('dialog[open]');
  await picker.getByLabel('新父标签 · 搜索名称或身份', { exact: true }).fill(parentTag.tagRef);
  await picker.getByLabel('新父标签', { exact: true }).selectOption(parentTag.tagRef);
  if (cancel) { await click('取消', picker); return; }
  const prepared = page.waitForResponse(response => new URL(response.url()).pathname === '/api/navigation/move-tag'); await click('预览换父', picker);
  const response = await prepared; assert.equal(response.status(), 200, await response.text());
  const committed = page.waitForResponse(reply => new URL(reply.url()).pathname === '/api/operations/commit'); await click('执行已预览的登记', page.locator('dialog[open]')); assert.equal((await committed).status(), 200);
  await until(async () => (await api('/api/navigation/tags', { root })).tags.find(item => item.tagId === tag.tagId)?.parentTagId === parentTag.tagId);
  await click('读取登记标签', await panelDetails('登记标签与目录入口'));
}
async function taskScope() { if (!await queryPanel().locator('.query-chips .query-chip').filter({ hasText: /^类型：任务 ×$/ }).count()) { await runQuery(22); await click('任务 · 1', queryPanel()); await runQuery(1); } }
async function clearConditions() { const chips = queryPanel().locator('.query-chips .query-chip').filter({ hasText: /^标签：/ }); while (await chips.count()) await chips.first().click(); }
async function runQuery(count, predicate) {
  const start = queryReceipts.length, startedMs = performance.now(), client = pageClients.get(page);
  assert.ok(client, 'Query must belong to an observed original browser client');
  const action = { phase, client, at: new Date().toISOString(), expectedRows: count, predicate, receiptCountAtStart: start,
    requestCountAtStart: result.requests.length, pendingQueryIds: [...observedRequests.values()]
      .filter(entry => entry.client === client && entry.route === '/api/query/run' && !entry.finishedAt && !entry.failedAt).map(entry => entry.id),
    timingScope: 'Bounded functional waits, not a responsiveness or performance qualification', waits: [], completed: false };
  (result.queryActions ??= []).push(action);
  const eligibleRequests = () => result.requests.filter((entry, index) => entry.client === client && entry.route === '/api/query/run'
    && (index >= action.requestCountAtStart || action.pendingQueryIds.includes(entry.id))
    && (!predicate || JSON.stringify(entry.body?.plan).includes(predicate)));
  const eligibleReceipts = () => {
    const ids = new Set(eligibleRequests().map(entry => entry.id));
    return queryReceipts.slice(start).filter(item => ids.has(item.requestId) && item.result.cutId
      && (!predicate || JSON.stringify(item.request.plan).includes(predicate)));
  };
  async function waitFor(check, label) {
    const wait = { label, startedAt: new Date().toISOString(), budgetMs: 30000, completed: false }, waitStarted = performance.now();
    action.waits.push(wait);
    try { const value = await until(check, label, wait.budgetMs); wait.completed = true; return value; }
    finally { wait.elapsedMs = performance.now() - waitStarted; wait.endedAt = new Date().toISOString(); }
  }
  try {
    // Schema acquisition and the actual frozen query are distinct product
    // phases. Observe the one click and its original/coalesced requests, without
    // retrying it or letting a response from another client satisfy this action.
    await click('查询', queryPanel()); action.clickedAt = new Date().toISOString();
    await waitFor(() => eligibleRequests().length > 0, 'Schema acquisition followed by original Query issuance');
    await waitFor(() => eligibleReceipts().length > 0, 'original Query response body');
    const receipt = await waitFor(async () => {
      if (await queryPanel().locator('.query-results').getAttribute('aria-busy') !== 'false') return false;
      if ((await queryPanel().locator('.query-results').innerText()).includes('先前读取的结果')) return false;
      const displayedBases = await queryPanel().locator('.query-basis').allTextContents();
      return eligibleReceipts().find(item => displayedBases.includes(`读取版本：${item.result.cutId}`));
    }, 'same original Query cutId visibly installed');
    action.matchedRequestId = receipt.requestId; action.matchedDisplayedCutId = receipt.result.cutId;
    // An admitted frozen read does not continuously confirm current filesystem
    // state. Its truthful stale marker differs from the UI's superseded result.
    assert.equal(receipt.result.rows.length, count, JSON.stringify(receipt.result)); assert.equal(receipt.result.coverage, 'complete'); assert.equal(receipt.result.snapshotMode, 'admittedFrozen'); assert.equal(receipt.result.freshness, 'stale'); assert.notEqual(receipt.result.superseded, true);
    await waitFor(() => queryPanel().locator('.query-card').count().then(n => n === count), 'same Query rendered cards');
    action.completed = true; return receipt;
  } finally {
    action.elapsedMs = performance.now() - startedMs; action.endedAt = new Date().toISOString();
    action.originalRequests = eligibleRequests().map(entry => ({ id: entry.id, at: entry.at, elapsedMs: entry.elapsedMs, status: entry.status, responseElapsedMs: entry.responseElapsedMs }));
    action.schemaRequests = result.requests.slice(action.requestCountAtStart).filter(entry => entry.client === client && entry.route === '/api/data/schema')
      .map(entry => ({ id: entry.id, at: entry.at, elapsedMs: entry.elapsedMs, status: entry.status, responseElapsedMs: entry.responseElapsedMs }));
  }
}
async function holdNextActualQuery() {
  const target = page, pattern = '**/api/query/run', tasks = new Set(), errors = []; let captured = false, heldValue, release;
  const released = new Promise(resolve => { release = resolve; });
  const handler = route => {
    const task = (async () => {
      if (captured) { await route.continue(); return; } captured = true;
      const original = await route.fetch({ timeout: 30000, maxRetries: 0 }), bytes = await original.body();
      assert.equal(original.status(), 200, bytes.toString('utf8'));
      heldValue = { request: route.request().postDataJSON(), response: JSON.parse(bytes.toString('utf8')), status: original.status(), rawBodyBytes: bytes.length, rawBodySha256: sha(bytes), heldAt: new Date().toISOString() };
      await released; await route.fulfill({ response: original }); heldValue.releasedAt = new Date().toISOString();
    })().catch(async error => { errors.push(String(error)); await route.abort('failed').catch(() => {}); }).finally(() => tasks.delete(task));
    tasks.add(task); return task;
  };
  await target.route(pattern, handler);
  return { ready: () => until(() => { if (errors.length) throw new Error(errors.join('\n')); return heldValue; }, 'original actual Query response held'), release: () => release(), close: async () => { release(); await Promise.allSettled([...tasks]); await target.unroute(pattern, handler); assert.deepEqual(errors, []); } };
}
async function refreshKnowledge(expected) {
  await until(async () => (await page.locator('.status-right').innerText()).includes('草稿预览已更新'), 'native render ready');
  const client = pageClients.get(page), startedMs = performance.now();
  assert.ok(client, 'Knowledge refresh must belong to an observed original browser client');
  assert.equal(await source(), expected);
  const currentPath = (await knowledge().locator('.knowledge-current').innerText()).trim();
  const action = { phase, client, at: new Date().toISOString(), expectedSourceSha256: sha(expected), currentPath,
    requestCountAtStart: result.requests.length, budgetMs: 30000, completed: false, candidateIdsSeen: [],
    pendingReadIds: [...observedRequests.values()].filter(entry => entry.client === client && entry.route === '/api/knowledge/read'
      && !entry.finishedAt && !entry.failedAt).map(entry => entry.id) };
  (result.knowledgeActions ??= []).push(action);
  const eligibleRequests = () => result.requests.filter((entry, index) => entry.client === client && entry.route === '/api/knowledge/read'
    && (index >= action.requestCountAtStart || action.pendingReadIds.includes(entry.id))
    && entry.body?.context?.source === expected && typeof entry.body.context.path === 'string'
    && path.normalize(entry.body.context.path) === path.normalize(currentPath));
  try {
    // A finished earlier read whose body observer runs late cannot stand in for
    // this explicit refresh. The one click may share an actually pending read;
    // any newer eligible request becomes the only candidate for delivery.
    await click('刷新', knowledge()); action.clickedAt = new Date().toISOString();
    const receipt = await until(async () => {
      const candidates = eligibleRequests();
      for (const entry of candidates) if (!action.candidateIdsSeen.includes(entry.id)) action.candidateIdsSeen.push(entry.id);
      const latest = candidates.at(-1); action.latestEligibleRequestId = latest?.id;
      if (!latest) return false;
      const observed = knowledgeReceipts.find(item => item.client === client && item.requestId === latest.id);
      if (!observed || !await knowledge().isVisible() || await source() !== expected) return false;
      if (await knowledge().getByRole('button', { name: '停止等待', exact: true }).count()) return false;
      if ((await knowledge().locator('.knowledge-status').innerText()).trim() !== '') return false;
      if (path.normalize((await knowledge().locator('.knowledge-current').innerText()).trim()) !== path.normalize(currentPath)) return false;
      if (eligibleRequests().at(-1)?.id !== latest.id) return false;
      return observed;
    }, 'actual same-Draft knowledge receipt and settled original owner', action.budgetMs);
    action.matchedRequestId = receipt.requestId; action.matchedGraphGeneration = receipt.result.graph?.generation;
    action.completed = true; return receipt;
  } finally {
    action.elapsedMs = performance.now() - startedMs; action.endedAt = new Date().toISOString();
    action.originalRequests = eligibleRequests().map(entry => ({ id: entry.id, at: entry.at, status: entry.status,
      responseElapsedMs: entry.responseElapsedMs, elapsedMs: entry.elapsedMs, bodyRecordedAt: entry.bodyRecordedAt, failure: entry.failure }));
  }
}
async function currentOutline(expected) {
  const readsBefore = result.requests.filter(entry => entry.route === '/api/knowledge/read').length;
  const receipt = await until(async () => {
    if (await source() !== expected || !await page.locator('.outline-view').isVisible()) return false;
    if (!(await page.locator('.status-right').innerText()).includes('草稿预览已更新')) return false;
    const keys = await page.locator('.outline-view [data-heading-key]').evaluateAll(rows => rows.map(row => row.dataset.headingKey));
    return renderReceipts.findLast(item => item.request.source === expected && item.result.ok && item.result.headingStructure
      && (keys.length > 0 || item.result.headingStructure.items.length === 0)
      && keys.every(key => item.result.headingStructure.items.some(heading => heading.key === key)));
  }, 'current Outline from its exact ordinary native render');
  assert.equal(result.requests.filter(entry => entry.route === '/api/knowledge/read').length, readsBefore, 'Observing the current document Outline started a whole-library knowledge read');
  return receipt;
}

try {
  await start(); await api('/api/library/create', { path: root, operationId: randomUUID() });
  const schema = await api('/api/data/schema', { root }), operationId = randomUUID();
  const schemaPlan = await api('/api/data/schema/prepare', { root, source: schema.source, beforeSource: schema.source, beforeRevision: schema.revision, operationId });
  await api('/api/data/schema/install', { operationId, planId: schemaPlan.planId });
  parent = await note('TEST-Parent'); const parentFolder = path.dirname(parent.path);
  image = await note('A-Image', parentFolder, '= TEST image note\n\nImage body stays exact.');
  assert.ok(image.source.includes('types: ()'));
  image = await save(image, image.source.replace('types: ()', 'types: ("task",), status: "open"'));
  image = await attachImage(image);
  plain = await note('B-Plain', parentFolder, '= TEST plain note\n\nNo image or PDF belongs to this note.');
  for (let i = 0; i < 18; i++) {
    const spacer = await note('M-' + String(i).padStart(2, '0'), parentFolder, 'Owned spacer note.');
    if (i === 0) relationSource = spacer;
    if (i === 1) relationTarget = spacer;
  }
  farImage = await attachImage(await note('Z-Image', parentFolder, 'Owned image outside the initial node-list viewport.'));
  const reference = doc => `weftext-node://${doc.workspaceId}/${doc.nodeId}`;
  // Use two existing spacers and the unchanged installed person schema. The
  // library still has 22 notes, one task and the original untyped B-Plain.
  relationTarget = await save(relationTarget, relationTarget.source.replace('types: ()', 'types: ("person",)'));
  relationSource = await save(relationSource, relationSource.source.replace('types: ()', `types: ("person",), parents: ((target: (ref: "${reference(relationTarget)}"),),), siblings: ((target: (ref: "${reference(relationTarget)}"),),)`));
  parent = await save(parent, parent.source + `\n\n= TEST Duplicate\n\nRoot before.\n\n== TEST Child\n\nChild content.\n\n==== TEST Deep\n\nDeep content.\n\n= TEST Duplicate\n\n#link("${reference(image)}")[first image link]\n\n#link("${reference(image)}")[second image link]\n\n#link("${reference(plain)}")[plain link]\n\n#figure(rect(width: 10pt, height: 10pt), kind: image, caption: [TEST Figure Caption])\n#figure(table(columns: 1, [Data]), caption: [TEST Table Caption])\n#figure(raw("const x = 1", block: true), caption: [TEST Code Caption])\n#set math.equation(numbering: "(1)")\n$ x = 1 $\n`);
  savedParent = parent.source;
  await stop(); await start(['--open', image.path, '--root', root]);
  browser = await chromium.launch({ headless: true, ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : { channel: 'chrome' }) });
  result.browser = browser.version(); context = await browser.newContext({ viewport: { width: 1480, height: 1080 }, reducedMotion: 'reduce' }); page = await context.newPage(); page.setDefaultTimeout(30000); observe(page);
  await page.goto(base + '/#token=' + token); await workspaceMode(page, '源码'); await expectSource(image.source);

  result.fixturePreparationMs = Math.round(performance.now() - caseStartedAt); caseStartedAt = performance.now();
  phase = 'hierarchical-tags'; await showQuery();
  const parentTag = await createTag('TEST Parent/名字'), otherTag = await createTag('TEST Other'), leaf = await createTag('TEST Leaf😀', parentTag.tagRef, true);
  assert.equal(leaf.parentTagId, parentTag.tagId); savedImage = await disk(image.path); await expectSource(savedImage);
  assert.ok(savedImage.includes(leaf.tagId)); assert.ok(!savedImage.includes(parentTag.tagId), 'Only the direct leaf is assigned; ancestors must not be authored');
  await cm().click(); await page.keyboard.press('Control+End'); await page.keyboard.insertText(' UNSAVED IMAGE DRAFT😀'); dirtyImage = savedImage + ' UNSAVED IMAGE DRAFT😀'; await expectSource(dirtyImage);
  await taskScope(); await (await tagRow(parentTag)).getByRole('button', { name: / · 此标签及子级$/ }).click(); const subtree = await runQuery(1, 'tagInSubtree');
  assert.equal(subtree.result.rows[0].entityKey, reference(image));
  await queryPanel().getByRole('button', { name: '改为仅此标签', exact: true }).focus(); await page.keyboard.press('Enter'); const exact = await runQuery(0, parentTag.tagRef); assert.ok(!JSON.stringify(exact.request.plan).includes('tagInSubtree'));
  await clearConditions(); await click('保存含子级入口', await tagRow(parentTag)); await click('保存精确入口', await tagRow(parentTag));
  const shortcuts = await page.evaluate(target => Object.entries(localStorage).filter(([key]) => key.startsWith('weftext-query-shortcuts:')).flatMap(([, raw]) => { const value = JSON.parse(raw); if (value.version !== 2) throw new Error('Shortcut collection is not v2'); return value.items.filter(item => item.kind === 'tag' && item.targetRef === target); }), parentTag.tagRef);
  assert.deepEqual([...new Set(shortcuts.map(item => item.matchMode))].sort(), ['exact', 'subtree']); result.tagShortcuts = shortcuts;
  await moveTag(leaf, otherTag, true); assert.equal((await api('/api/navigation/tags', { root })).tags.find(tag => tag.tagId === leaf.tagId).parentTagId, parentTag.tagId);
  await moveTag(leaf, otherTag); await clearConditions(); await (await tagRow(parentTag)).getByRole('button', { name: / · 此标签及子级$/ }).click(); await runQuery(0, 'tagInSubtree');
  await clearConditions(); await (await tagRow(otherTag)).getByRole('button', { name: / · 此标签及子级$/ }).click(); await runQuery(1, 'tagInSubtree');
  assert.equal(await disk(image.path), savedImage); await expectSource(dirtyImage); await moveTag(leaf, parentTag);
  await pass('Mouse-created parent/leaf and atomic first assignment, keyboard exact-vs-subtree Query, cancel/move/rerun and explicit shortcut modes preserve direct references, source bytes and the unsaved editor.');

  phase = 'reopened-shortcuts'; const originalPage = page; page = await context.newPage(); page.setDefaultTimeout(30000); observe(page); await page.goto(base + '/#token=' + token); await showQuery();
  await taskScope(); await panelDetails('本机快捷入口与最近打开');
  for (const mode of ['exact', 'subtree']) { await clearConditions(); await runQuery(1); await panelDetails('本机快捷入口与最近打开'); const item = shortcuts.find(shortcut => shortcut.matchMode === mode), label = `${item.label} · ${mode === 'subtree' ? '此标签及子级' : '仅此标签'}`; await queryPanel().getByRole('button', { name: label, exact: true }).focus(); await page.keyboard.press('Enter'); const receipt = await runQuery(mode === 'exact' ? 0 : 1, parentTag.tagRef); assert.equal(JSON.stringify(receipt.request.plan).includes('tagInSubtree'), mode === 'subtree'); }
  await page.close(); page = originalPage; await expectSource(dirtyImage); await cm().click(); await page.keyboard.press('Control+z'); await expectSource(savedImage); await page.keyboard.press('Control+y'); await expectSource(dirtyImage);
  await pass('A separately reopened embedded client restores exact/subtree shortcut meaning without implicit mode memory; the original retained document keeps its own Draft/Undo.');

  phase = 'cancel-rejected-first-assignment'; await quick('B-Plain'); await workspaceMode(page, '源码'); await expectSource(plain.source); await showQuery();
  const rejectedName = 'TEST Rejected Untyped Assignment', navigation = await panelDetails('登记标签与目录入口');
  await click('读取登记标签', navigation); await navigation.getByLabel('新标签父级', { exact: true }).waitFor();
  await navigation.getByLabel('新标签显示名称', { exact: true }).fill(rejectedName);
  await navigation.getByLabel('新标签父级 · 搜索名称或身份', { exact: true }).fill(''); await navigation.getByLabel('新标签父级', { exact: true }).selectOption('');
  const failedPrepare = page.waitForResponse(response => new URL(response.url()).pathname === '/api/navigation/create-tag');
  await click('新建并赋给当前已保存笔记', navigation); const rejection = await failedPrepare, rejectedBody = rejection.request().postDataJSON(), rejectedResult = await rejection.json();
  assert.equal(rejection.status(), 409); assert.equal(rejectedResult.error?.code, 'invalidField'); assert.equal(rejectedBody.assignment.path, plain.path);
  assert.ok(!await page.locator('dialog[open]').count(), 'A rejected preparation must never become a committable confirmation');
  const originalPrepareCount = mutations.filter(item => item.route === '/api/navigation/create-tag' && item.body.operationId === rejectedBody.operationId).length; assert.equal(originalPrepareCount, 1);
  const cancelResponse = page.waitForResponse(response => new URL(response.url()).pathname === '/api/navigation/cancel' && response.request().postDataJSON().operationId === rejectedBody.operationId);
  await click('取消原未提交导航操作', navigation); const cancellation = await cancelResponse, cancelBody = cancellation.request().postDataJSON(), cancelResult = await cancellation.json();
  assert.equal(cancellation.status(), 200); assert.deepEqual(cancelBody, { root, operationId: rejectedBody.operationId, planId: null });
  assert.deepEqual(cancelResult, { operationId: rejectedBody.operationId, planId: null, status: 'resolved' });
  await until(() => navigation.innerText().then(text => text.includes('原操作已在服务端实际结束')));
  assert.equal(mutations.filter(item => item.route === '/api/navigation/create-tag' && item.body.operationId === rejectedBody.operationId).length, originalPrepareCount, 'Cancellation must not silently reprepare the rejected operation');
  const fences = [];
  for (const [intent, body] of [['original invalid assignment', rejectedBody], ['otherwise valid creation with the same cancelled ID', Object.fromEntries(Object.entries(rejectedBody).filter(([key]) => key !== 'assignment'))]]) {
    const reply = await fetch(base + '/api/navigation/create-tag', { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': token }, body: JSON.stringify(body) }), value = await reply.json();
    assert.equal(reply.status, 409, JSON.stringify(value)); assert.equal(value.error?.code, 'operationCancelled'); fences.push({ intent, request: body, status: reply.status, response: value });
  }
  const tagsAfterCancel = (await api('/api/navigation/tags', { root })).tags; assert.ok(!tagsAfterCancel.some(tag => tag.displayName === rejectedName));
  const afterCancelledTag = await createTag('TEST After Cancel'); const newRequest = mutations.findLast(item => item.route === '/api/navigation/create-tag' && item.body.displayName === afterCancelledTag.displayName);
  assert.ok(newRequest && newRequest.body.operationId !== rejectedBody.operationId); await expectSource(plain.source); assert.equal(await disk(plain.path), plain.source);
  await page.keyboard.press('Alt+ArrowLeft'); await expectSource(dirtyImage);
  result.rejectedPrepareCancellation = { request: rejectedBody, rejectedResult, cancelBody, cancelResult, fences, newOperationId: newRequest.body.operationId };
  await pass('A real untyped-note first assignment rejects before preparation; UI cancellation sends the same ID with no plan and never reprepares. The original and changed late intents stay fenced, while a new UI operation succeeds and both source/Draft remain exact.');

  phase = 'query-refresh-coalescing'; await showQuery(); const baseline = await runQuery(0, otherTag.tagRef); assert.ok(baseline.request.plan.where.includes('tagInSubtree'));
  // The wrapping label also contains the select's option text. Match its
  // actual control instead of assuming that its accessible name is only 密度.
  const density = () => queryPanel().locator('label.query-control-label').filter({ hasText: /^密度/ }).locator('select');
  const originalDensity = await density().inputValue(), originalThumbnails = await queryPanel().getByLabel('缩略图', { exact: true }).isChecked();
  const sameEpochStart = result.requests.length, sameEpochMutations = mutations.length, sameEpochHold = await holdNextActualQuery(); let sameEpochWork, sameEpochHeld, sameEpochReceipt;
  try {
    sameEpochWork = runQuery(0, otherTag.tagRef); sameEpochWork.catch(() => {}); sameEpochHeld = await sameEpochHold.ready();
    await density().selectOption(originalDensity === 'compact' ? 'comfortable' : 'compact'); await queryPanel().getByLabel('缩略图', { exact: true }).setChecked(!originalThumbnails); await click('查询', queryPanel());
    assert.equal(mutations.length, sameEpochMutations); await expectSource(dirtyImage); sameEpochHold.release(); sameEpochReceipt = await sameEpochWork; await pause(250);
    assert.equal(result.requests.slice(sameEpochStart).filter(item => item.route === '/api/query/run').length, 1, 'Presentation changes and same-epoch repeated refresh must share one actual query');
    assert.deepEqual(sameEpochReceipt.result, sameEpochHeld.response); assert.equal(sameEpochReceipt.result.cutId, sameEpochHeld.response.cutId); assert.equal(await disk(image.path), savedImage);
  } finally { sameEpochHold.release(); if (sameEpochWork) await Promise.allSettled([sameEpochWork]); await sameEpochHold.close(); }
  const changedEpochStart = result.requests.length, changedEpochHold = await holdNextActualQuery(); let changedEpochHeld, changedEpochWork, changedEpochReceipt, installedCuts;
  try {
    await click('查询', queryPanel()); changedEpochHeld = await changedEpochHold.ready(); assert.equal(changedEpochHeld.response.rows.length, 0);
    await page.evaluate(() => { const host = document.querySelector('.query-results'), audit = { events: [] }; const observe = () => audit.events.push({ at: Date.now(), bases: Array.from(host.querySelectorAll('.query-basis')).map(item => item.textContent), busy: host.getAttribute('aria-busy') }); audit.observer = new MutationObserver(observe); audit.observer.observe(host, { childList: true, subtree: true, attributes: true, attributeFilter: ['aria-busy'] }); observe(); window.__v063QueryBasisAudit = audit; });
    await moveTag(leaf, otherTag); changedEpochWork = runQuery(1, otherTag.tagRef); changedEpochWork.catch(() => {}); changedEpochHold.release(); changedEpochReceipt = await changedEpochWork; await pause(250);
    const actualRequests = result.requests.slice(changedEpochStart).filter(item => item.route === '/api/query/run'); assert.equal(actualRequests.length, 2, 'A real registry commit must execute a new query rather than join its old environment');
    installedCuts = await page.evaluate(() => window.__v063QueryBasisAudit.events); assert.ok(!installedCuts.some(event => event.bases.includes(`读取版本：${changedEpochHeld.response.cutId}`)), 'The held obsolete environment must never become the visible result basis');
    assert.deepEqual(changedEpochHeld.request, changedEpochReceipt.request); assert.notEqual(changedEpochHeld.response.cutId, changedEpochReceipt.result.cutId); assert.equal(changedEpochReceipt.result.rows[0].entityKey, reference(image)); await expectSource(dirtyImage); assert.equal(await disk(image.path), savedImage);
  } finally { await page.evaluate(() => { window.__v063QueryBasisAudit?.observer.disconnect(); delete window.__v063QueryBasisAudit; }).catch(() => {}); changedEpochHold.release(); if (changedEpochWork) await Promise.allSettled([changedEpochWork]); await changedEpochHold.close(); }
  await moveTag(leaf, parentTag); await density().selectOption(originalDensity); await queryPanel().getByLabel('缩略图', { exact: true }).setChecked(originalThumbnails); await expectSource(dirtyImage);
  result.queryRefreshCoalescing = { sameEpochHeld, sameEpochVisibleCut: sameEpochReceipt.result.cutId, changedEpochHeld, changedEpochVisibleCut: changedEpochReceipt.result.cutId, installedCuts, hook: 'Only the first original server response is held and then forwarded without changing its body; no synthetic Query result is supplied. A read-only MutationObserver records actual visible result bases.' };
  await pass('A held actual Core response survives presentation changes and same-epoch repeated refresh as one query. A real tag-tree commit requires a second identical-plan request and a new visible cut; the old result cannot replace it. Hooks, tags and display preferences are restored.');

  phase = 'authored-order'; await quick('TEST-Parent'); await workspaceMode(page, '源码'); await expectSource(savedParent);
  await cm().click(); await page.keyboard.press('Control+End'); await page.keyboard.insertText(' UNSAVED PARENT DRAFT😀'); dirtyParent = savedParent + ' UNSAVED PARENT DRAFT😀'; await expectSource(dirtyParent);
  // Files and Nodes are independent dock views. Use their actual visible tabs
  // and existing browse action, rather than operating the hidden Files owner.
  await page.locator('[data-dock-tab="files"]').click();
  await page.locator('#workspace-view-files').waitFor({ state: 'visible' });
  await page.getByRole('button', { name: '仅浏览 TEST-Parent 文件夹', exact: true }).click();
  await nodesView().waitFor({ state: 'visible' }); await contents().waitFor();
  assert.equal(await nodesView().locator('.workspace-nodes-view').getAttribute('data-follow'), 'false');
  assert.equal(path.normalize(await nodesView().locator('.node-browser-path').getAttribute('title')), path.normalize(parentFolder));
  const captured = await api('/api/navigation/capture', { root, path: parentFolder }), before = await api('/api/navigation/order/read', { root, parentRef: captured.targetRef }); assert.equal(before.complete, true);
  const countBeforePreference = mutations.length; await contents().getByLabel('节点内容个人排序', { exact: true }).selectOption('path'); await contents().getByLabel('节点内容个人排序', { exact: true }).selectOption('name');
  assert.equal(mutations.length, countBeforePreference); assert.deepEqual((await api('/api/navigation/order/read', { root, parentRef: captured.targetRef })).orderedRefs, before.orderedRefs);
  await contents().getByLabel('节点内容个人排序', { exact: true }).selectOption('authored'); await contents().getByRole('button', { name: 'B-Plain 作者顺序上移', exact: true }).waitFor();
  const countBeforeCancel = mutations.length; await click('B-Plain 作者顺序上移', contents()); await click('取消', page.getByRole('dialog', { name: '作者同级顺序变更', exact: true })); assert.equal(mutations.length, countBeforeCancel);
  await click('B-Plain 作者顺序上移', contents()); await click('准备此顺序变更', page.getByRole('dialog', { name: '作者同级顺序变更', exact: true })); await click('取消原作者顺序计划', contents()); await click('结束已确认顺序操作', contents());
  assert.deepEqual((await api('/api/navigation/order/read', { root, parentRef: captured.targetRef })).orderedRefs, before.orderedRefs);
  await contents().getByRole('button', { name: 'B-Plain 作者顺序上移', exact: true }).focus(); await page.keyboard.press('Enter'); await click('准备此顺序变更', page.getByRole('dialog', { name: '作者同级顺序变更', exact: true })); await click('查看原顺序变更并确认', contents()); await click('确认提交原作者顺序', page.getByRole('dialog', { name: '作者同级顺序变更', exact: true }));
  const after = await until(async () => { const order = await api('/api/navigation/order/read', { root, parentRef: captured.targetRef }); return order.orderedRefs[0] === reference(plain) ? order : false; }); assert.equal(after.orderedRefs[1], reference(image));
  await contents().getByLabel('节点内容个人排序', { exact: true }).selectOption('name');
  const displayedNames = await contents().locator('[data-entry-path]').evaluateAll(rows => rows.map(row => row.dataset.entryPath)); assert.ok(displayedNames.indexOf(path.dirname(image.path)) < displayedNames.indexOf(path.dirname(plain.path)));
  assert.deepEqual((await api('/api/navigation/order/read', { root, parentRef: captured.targetRef })).orderedRefs, after.orderedRefs); await expectSource(dirtyParent); assert.equal(await disk(parent.path), savedParent);
  result.authoredOrder = { before, after }; await pass('Personal sorting is read-only; pre-prepare and durable prepared cancellation preserve the exact order; explicit keyboard preparation plus confirmation changes shared order without saving the parent Draft.');

  phase = 'lazy-thumbnails';
  // Folder preferences now enable thumbnails by default. Establish this case's
  // explicit UI starting point; disposing then remounting each node thumbnail
  // renews its visible-row resolver without changing any author/source owner.
  await contents().getByLabel('节点内容缩略图', { exact: true }).uncheck();
  assert.equal(await contents().getByLabel('节点内容缩略图', { exact: true }).isChecked(), false);
  const previewStart = previewRequests.length; await contents().getByLabel('节点内容缩略图', { exact: true }).check();
  const rowFor = doc => contents().locator(`[data-entry-path=${JSON.stringify(path.dirname(doc.path))}]`);
  await rowFor(image).scrollIntoViewIfNeeded(); await until(() => rowFor(image).locator('.node-content-thumbnail').getAttribute('data-preview-state').then(state => state === 'ready'), 'visible actual image pixels');
  assert.equal(await rowFor(image).locator('img,canvas').count(), 1);
  const decodedPixels = doc => rowFor(doc).locator('img,canvas').evaluate(element => element instanceof HTMLImageElement ? element.complete && element.naturalWidth > 0 && element.naturalHeight > 0 : element instanceof HTMLCanvasElement && element.width > 0 && element.height > 0);
  await until(() => decodedPixels(image), 'visible thumbnail decodes real pixel dimensions');
  assert.ok(!previewRequests.slice(previewStart).some(request => request.entityKey === reference(farImage)), 'An offscreen row must not read its preview before it becomes visible');
  await rowFor(plain).scrollIntoViewIfNeeded(); await until(() => rowFor(plain).locator('.node-content-thumbnail').getAttribute('data-preview-state').then(state => state === 'absent'), 'explicit no-image row'); assert.equal(await rowFor(plain).locator('img,canvas').count(), 0);
  await rowFor(farImage).scrollIntoViewIfNeeded(); await until(() => rowFor(farImage).locator('.node-content-thumbnail').getAttribute('data-preview-state').then(state => state === 'ready'), 'newly visible far image');
  await until(() => decodedPixels(farImage), 'newly visible thumbnail decodes real pixel dimensions'); await page.screenshot({ path: path.join(out, 'lazy-thumbnails.png'), fullPage: false });
  assert.ok(previewRequests.slice(previewStart).some(request => request.entityKey === reference(farImage)));
  await contents().getByLabel('节点内容缩略图', { exact: true }).uncheck(); assert.equal(await contents().locator('.node-content-thumbnail,img,canvas').count(), 0); await expectSource(dirtyParent); assert.equal(await disk(image.path), savedImage); assert.equal(await disk(parent.path), savedParent);
  await pass('Only visible rows read actual exact-basis media; visible PNG pixels and explicit absent state are distinct; scrolling starts the distant image and disabling thumbnails disposes pixels without author edits.');

  phase = 'actual-graph'; await knowledgeCommand('打开二维关系图谱', '二维图谱'); await knowledgeCommand('打开二维关系图谱', '二维图谱'); const receipt = await refreshKnowledge(dirtyParent), snapshot = receipt.result;
  assert.equal(snapshot.graph.scope, 'authorizedNativeSourcesAndTypedReferences'); assert.ok(snapshot.graph.generation); const parentKey = `${parent.workspaceId}/${parent.nodeId}`, imageKey = `${image.workspaceId}/${image.nodeId}`;
  const duplicateEdges = snapshot.graph.edges.filter(edge => edge.source === parentKey && edge.target === imageKey && !edge.fieldId); assert.equal(duplicateEdges.length, 2); assert.notEqual(duplicateEdges[0].key, duplicateEdges[1].key);
  const graph = () => knowledge().locator('.knowledge-graph-view'); await graph().locator('g[data-graph-key]').first().waitFor();
  await graph().getByLabel('在当前图范围搜索定位', { exact: true }).fill('A-Image'); await click('定位 A-Image', graph()); await expectSource(dirtyParent); assert.equal(await disk(parent.path), savedParent);
  await graph().getByLabel('方向', { exact: true }).selectOption('out'); await graph().getByLabel('范围', { exact: true }).selectOption('global'); await click('放大', graph()); await click('向右平移', graph()); await click('适应视口', graph());
  const graphNode = graph().locator(`g[data-graph-key=${JSON.stringify(imageKey)}]`); await graphNode.focus(); await page.keyboard.press('Enter'); await workspaceMode(page, '源码'); await expectSource(dirtyImage);
  await page.keyboard.press('Alt+ArrowLeft'); await expectSource(dirtyParent); await knowledgeCommand('打开二维关系图谱', '二维图谱'); await refreshKnowledge(dirtyParent);
  const edge = duplicateEdges[1]; await graph().locator(`path[data-graph-key=${JSON.stringify(edge.key)}]`).focus(); await page.keyboard.press('Space'); await click('打开准确作者来源', graph());
  await until(async () => { const actual = await cm().evaluate(element => { const range = element.cmTile.root.view.state.selection.main; return { from: range.from, to: range.to }; }); return actual.from === edge.range.utf16From && actual.to === edge.range.utf16To; }, 'accurate original graph occurrence source range'); await expectSource(dirtyParent);
  result.snapshots.push({ graph: snapshot.graph, headingStructure: snapshot.headingStructure });
  await pass('Actual same-Draft knowledge receipt reaches the embedded graph with two distinct body occurrences; search/filter/pan/zoom do not write, keyboard opens the accurate retained note, Back restores Draft, and edge navigation selects its exact author occurrence.');

  phase = 'actual-typed-graph'; await knowledgeCommand('打开二维关系图谱', '二维图谱');
  const typedReceipt = await refreshKnowledge(dirtyParent), typedGraph = typedReceipt.result.graph;
  const relationSourceKey = `${relationSource.workspaceId}/${relationSource.nodeId}`, relationTargetKey = `${relationTarget.workspaceId}/${relationTarget.nodeId}`;
  const typedEdges = typedGraph.edges.filter(edge => edge.authorSource === relationSourceKey && edge.fieldId);
  assert.equal(typedEdges.length, 3, 'Two actual authored fields produce one directed, one inverse and one symmetric edge');
  const forward = typedEdges.find(edge => edge.relation === 'people/parent'), inverse = typedEdges.find(edge => edge.relation === 'child'), symmetric = typedEdges.find(edge => edge.relation === 'people/sibling');
  assert.ok(forward && inverse && symmetric); assert.equal(new Set(typedEdges.map(edge => edge.key)).size, 3);
  for (const edge of typedEdges) {
    assert.equal(edge.sourceHash, sha(relationSource.source)); assert.equal(edge.range, null);
    assert.deepEqual(edge.itemPath.map(part => JSON.parse(part)), [{ kind: 'index', index: 0 }, { kind: 'member', key: 'target' }, { kind: 'member', key: 'ref' }]);
  }
  assert.deepEqual([forward.source, forward.target, forward.direction, forward.derived, forward.fieldId], [relationSourceKey, relationTargetKey, 'directed', false, 'people/parents']);
  assert.deepEqual([inverse.source, inverse.target, inverse.direction, inverse.derived, inverse.fieldId], [relationTargetKey, relationSourceKey, 'directed', true, 'people/parents']);
  assert.deepEqual([symmetric.source, symmetric.target, symmetric.direction, symmetric.derived, symmetric.fieldId], [relationSourceKey, relationTargetKey, 'symmetric', false, 'people/siblings']);
  const typedMutations = mutations.length, typedView = () => knowledge().locator('.knowledge-graph-view');
  await typedView().getByLabel('范围', { exact: true }).selectOption('global');
  await typedView().getByLabel('在当前图范围搜索定位', { exact: true }).fill('');
  await click('适应视口', typedView());
  for (const edge of [forward, symmetric, inverse]) {
    await typedView().getByLabel('关系类型', { exact: true }).selectOption(edge.relation);
    const visibleEdge = typedView().locator(`path[data-graph-key=${JSON.stringify(edge.key)}]`);
    // A painted vertical/horizontal SVG stroke can have a zero-width/height
    // fill box. Check its actual paint and on-screen stroke, not that box.
    await visibleEdge.waitFor({ state: 'attached' }); assert.equal(await typedView().locator('path[data-graph-key]').count(), 1);
    const drawing = typedView().locator('.knowledge-graph-canvas svg'); await drawing.waitFor({ state: 'visible' }); await drawing.scrollIntoViewIfNeeded();
    await until(async () => {
      const paint = await visibleEdge.evaluate(element => {
        const svg = element.ownerSVGElement, style = getComputedStyle(element), matrix = element.getScreenCTM(), length = element.getTotalLength();
        const rect = svg.getBoundingClientRect(), clip = { left: Math.max(0, rect.left), top: Math.max(0, rect.top), right: Math.min(innerWidth, rect.right), bottom: Math.min(innerHeight, rect.bottom) };
        const ancestors = []; let opacity = 1, displayed = true;
        for (let ancestor = element; ancestor; ancestor = ancestor.parentElement) {
          const current = getComputedStyle(ancestor); opacity *= Number(current.opacity); displayed &&= current.display !== 'none' && current.visibility === 'visible' && current.contentVisibility !== 'hidden';
          ancestors.push({ tag: ancestor.tagName, display: current.display, visibility: current.visibility, opacity: current.opacity });
          const bounds = ancestor.getBoundingClientRect(), left = bounds.left + ancestor.clientLeft, top = bounds.top + ancestor.clientTop;
          if (/(auto|scroll|hidden|clip|overlay)/.test(current.overflowX)) { clip.left = Math.max(clip.left, left); clip.right = Math.min(clip.right, left + ancestor.clientWidth); }
          if (/(auto|scroll|hidden|clip|overlay)/.test(current.overflowY)) { clip.top = Math.max(clip.top, top); clip.bottom = Math.min(clip.bottom, top + ancestor.clientHeight); }
        }
        const scale = matrix ? Math.min(Math.hypot(matrix.a, matrix.b), Math.hypot(matrix.c, matrix.d)) : 0, strokeWidth = parseFloat(style.strokeWidth), strokeOpacity = Number(style.strokeOpacity);
        const alpha = style.stroke.match(/(?:rgba|hsla)\([^)]*[,/]\s*([\d.]+)(%)?\s*\)$/i), paintedStroke = style.stroke !== 'none' && style.stroke !== 'transparent' && (!alpha || Number(alpha[1]) > 0);
        const samples = matrix && Number.isFinite(length) && length > 0 ? [0.1, 0.25, 0.5, 0.75, 0.9, 0.95].map(fraction => {
          const local = element.getPointAtLength(length * fraction), screen = new DOMPoint(local.x, local.y).matrixTransform(matrix), inside = screen.x > clip.left && screen.x < clip.right && screen.y > clip.top && screen.y < clip.bottom;
          return { fraction, x: screen.x, y: screen.y, inside, onStroke: element.isPointInStroke(local), hit: inside && document.elementFromPoint(screen.x, screen.y) === element };
        }) : [];
        return { key: element.dataset.graphKey, d: element.getAttribute('d'), stroke: style.stroke, strokeWidth, strokeOpacity, effectiveOpacity: opacity, displayed, ancestors, length, scale, screenStrokeWidth: strokeWidth * scale, clip, samples, painted: element.isConnected && displayed && opacity > 0 && paintedStroke && strokeOpacity > 0 && Number.isFinite(strokeWidth * scale) && strokeWidth * scale > 0 && samples.some(sample => sample.inside && sample.onStroke && sample.hit) };
      });
      const checks = result.typedGraphStrokeChecks ??= [], index = checks.findIndex(check => check.key === paint.key); if (index < 0) checks.push(paint); else checks[index] = paint;
      assert.equal(paint.painted, true, 'Actual SVG stroke must be painted, unclipped and hit-testable on screen: ' + JSON.stringify(paint)); return true;
    }, 'actual visible SVG relation stroke');
    assert.equal(await visibleEdge.getAttribute('marker-end') !== null, edge.direction === 'directed');
    await visibleEdge.focus(); assert.equal(await visibleEdge.evaluate(element => document.activeElement === element), true); await page.keyboard.press('Space');
    const detail = typedView().locator('.knowledge-card');
    assert.equal(await detail.getByRole('heading', { name: edge.relation, exact: true }).count(), 1);
    const text = await detail.innerText(); assert.ok(text.includes(`来源：M-00 · 字段 ${edge.fieldId}`));
    assert.ok(text.includes(edge.derived ? '派生反向；作者事实保存在原来源，不另写反边。' : '原始作者关系。'));
    assert.equal(text.includes('↔ 对称关系 ↔'), edge.direction === 'symmetric');
  }
  await click('打开准确作者来源', typedView()); await expectSource(relationSource.source);
  assert.equal(await disk(relationSource.path), relationSource.source); assert.equal(await disk(relationTarget.path), relationTarget.source);
  await page.keyboard.press('Alt+ArrowLeft'); await expectSource(dirtyParent);
  assert.equal(mutations.length, typedMutations); assert.equal(await disk(parent.path), savedParent); assert.equal(await disk(image.path), savedImage);
  result.snapshots.push({ typedGraph: { generation: typedGraph.generation, source: relationSourceKey, target: relationTargetKey, edges: typedEdges } });
  await pass('Actual Core-authored person fields reach the embedded graph as separate directed, symmetric and derived-inverse relations; type filters, arrowheads and original field provenance remain exact, and inverse source navigation opens its real author then returns without writes.');

  phase = 'actual-heading-structure'; await knowledgeCommand('打开只读标题结构／大纲', '标题结构'); await knowledgeCommand('打开只读标题结构／大纲', '标题结构'); const headingsReceipt = await currentOutline(dirtyParent), headings = headingsReceipt.result.headingStructure;
  assert.ok(headings); const repeated = headings.items.filter(heading => heading.kind === 'heading' && heading.text === 'TEST Duplicate'); assert.equal(repeated.length, 2); assert.ok(repeated.every(heading => heading.range)); assert.notEqual(repeated[0].key, repeated[1].key);
  const heading = repeated[1], structure = () => page.locator('.outline-view');
  await structure().getByLabel('搜索标题与题注', { exact: true }).fill('TEST Duplicate'); await structure().getByLabel('搜索标题与题注', { exact: true }).press('Enter');
  const headingRow = structure().locator(`li[data-heading-key=${JSON.stringify(heading.key)}]`); await headingRow.locator('.outline-jump').focus(); await page.keyboard.press('Enter');
  await until(async () => { const actual = await cm().evaluate(element => { const range = element.cmTile.root.view.state.selection.main; return { from: range.from, to: range.to }; }); return actual.from === heading.range.utf16From && actual.to === heading.range.utf16To; }, 'second duplicate heading exact source range');
  await expectSource(dirtyParent); assert.equal(await disk(parent.path), savedParent);
  await cm().click(); await page.keyboard.press('Control+z'); await expectSource(savedParent); await page.keyboard.press('Control+y'); await expectSource(dirtyParent);
  await quick('A-Image'); await expectSource(dirtyImage); await page.keyboard.press('Alt+ArrowLeft'); await expectSource(dirtyParent);
  await pass('Native duplicate headings retain distinct accurate keys/ranges; keyboard search/jump chooses the second exact heading and all new read-only views preserve the original document Undo and navigation Draft.');

  phase = 'actual-heading-subtree'; await knowledgeCommand('打开只读标题结构／大纲', '标题结构');
  const structureReceipt = await currentOutline(dirtyParent), nativeHeadings = structureReceipt.result.headingStructure;
  assert.equal(nativeHeadings.coverage, 'complete'); assert.equal(nativeHeadings.truncated, false);
  const headingItems = nativeHeadings.items.filter(item => item.kind === 'heading'), captionItems = nativeHeadings.items.filter(item => item.kind !== 'heading');
  assert.deepEqual(headingItems.map(item => [item.text, item.level]), [['TEST Duplicate', 1], ['TEST Child', 2], ['TEST Deep', 4], ['TEST Duplicate', 1]]);
  assert.deepEqual(captionItems.map(item => item.kind).sort(), ['code', 'equation', 'figure', 'table']);
  assert.ok(captionItems.every(item => item.level === 0 && item.generated === !item.range));
  const [firstRoot, childHeading, deepHeading, secondRoot] = headingItems, structureMutations = mutations.length;
  const structureView = () => page.locator('.outline-view'), structureRow = key => structureView().locator(`li[data-heading-key=${JSON.stringify(key)}]`);
  const searchHeadings = async text => { await structureView().getByLabel('搜索标题与题注', { exact: true }).fill(text); await structureView().getByLabel('搜索标题与题注', { exact: true }).press('Enter'); };
  const selectedHeading = async key => { await until(() => structureRow(key).getAttribute('class').then(value => value.includes('is-selected')), 'actual selected heading ' + key); };
  await searchHeadings(''); await structureView().locator('.outline-options > summary').click(); await click('整篇', structureView()); await click('全部展开', structureView());
  assert.equal(await structureView().locator('.knowledge-heading-list > li').count(), 8);
  assert.equal(await structureRow('@weftext-document-root').count(), 0, 'The synthetic document root is not a redundant list item');
  assert.match(await structureRow(firstRoot.key).locator('.outline-jump').getAttribute('aria-label'), /1 个直接子项／2 个后代子项$/);
  assert.match(await structureRow(childHeading.key).locator('.outline-jump').getAttribute('aria-label'), /1 个直接子项／1 个后代子项$/);
  assert.match(await structureRow(deepHeading.key).locator('.outline-jump').getAttribute('aria-label'), /0 个直接子项／0 个后代子项$/);
  await structureRow(firstRoot.key).locator('.outline-jump').click();
  await structureRow(firstRoot.key).getByRole('button', { name: '折叠分支', exact: true }).press('Enter');
  assert.equal(await page.evaluate(() => document.activeElement?.dataset.outlineControl), `fold:${firstRoot.key}`);
  assert.equal(await structureView().locator('.knowledge-heading-list > li').count(), 6);
  assert.equal(await structureRow(childHeading.key).count(), 0); assert.equal(await structureRow(deepHeading.key).count(), 0);
  assert.equal(await structureRow(firstRoot.key).getByRole('button', { name: '展开分支', exact: true }).getAttribute('aria-expanded'), 'false');
  await structureRow(firstRoot.key).getByRole('button', { name: '展开分支', exact: true }).press('Space');
  assert.equal(await page.evaluate(() => document.activeElement?.dataset.outlineControl), `fold:${firstRoot.key}`);
  assert.equal(await structureView().locator('.knowledge-heading-list > li').count(), 8);
  await structureRow(firstRoot.key).getByRole('button', { name: '折叠分支', exact: true }).press('Enter');
  assert.equal(await page.evaluate(() => document.activeElement?.dataset.outlineControl), `fold:${firstRoot.key}`);
  await searchHeadings('TEST Deep'); assert.ok((await structureView().innerText()).includes('1 个大纲项命中'));
  assert.equal(await structureRow(childHeading.key).count(), 1); assert.equal(await structureRow(deepHeading.key).count(), 1); assert.equal(await structureRow(secondRoot.key).count(), 0);
  assert.match(await structureRow(firstRoot.key).locator('.outline-jump').getAttribute('aria-label'), /1 个直接子项／2 个后代子项$/);
  await searchHeadings(''); assert.equal(await structureView().locator('.knowledge-heading-list > li').count(), 6);
  assert.equal(await structureRow(deepHeading.key).count(), 0, 'Clearing search restores the original collapsed branch');
  await searchHeadings('TEST Duplicate'); assert.ok((await structureView().innerText()).includes('2 个大纲项命中'));
  await structureView().getByRole('button', { name: '下一命中', exact: true }).press('Enter'); await selectedHeading(secondRoot.key);
  assert.equal(await page.evaluate(() => document.activeElement?.dataset.outlineControl), 'next-match');
  await structureView().getByRole('button', { name: '上一命中', exact: true }).press('Enter'); await selectedHeading(firstRoot.key);
  await structureView().getByRole('button', { name: '上一命中', exact: true }).press('Enter'); await selectedHeading(secondRoot.key);
  await structureView().getByRole('button', { name: '下一命中', exact: true }).press('Enter'); await selectedHeading(firstRoot.key);
  await searchHeadings(''); await click('全部展开', structureView()); await structureRow(childHeading.key).locator('.outline-jump').click();
  await structureView().getByRole('button', { name: '所选子树', exact: true }).press('Enter');
  assert.equal(await page.evaluate(() => document.activeElement?.dataset.outlineControl), 'selected-subtree');
  assert.equal(await structureView().locator('.knowledge-heading-list > li').count(), 2);
  assert.equal(await structureRow(childHeading.key).count(), 1); assert.equal(await structureRow(deepHeading.key).count(), 1);
  assert.equal(await structureRow('@weftext-document-root').count(), 0); assert.equal(await structureRow(firstRoot.key).count(), 0); assert.equal(await structureRow(secondRoot.key).count(), 0);
  await structureView().getByRole('button', { name: '整篇', exact: true }).press('Enter'); assert.equal(await structureView().locator('.knowledge-heading-list > li').count(), 8);
  assert.equal(await page.evaluate(() => document.activeElement?.dataset.outlineControl), 'whole-document');
  const outlineDepth = structureView().locator('[data-outline-control=depth]');
  await outlineDepth.focus(); await outlineDepth.fill('1'); await outlineDepth.press('Enter');
  assert.equal(await page.evaluate(() => document.activeElement?.dataset.outlineControl), 'depth');
  assert.equal(await structureView().locator('.knowledge-heading-list > li').count(), 2);
  await outlineDepth.fill('0'); await outlineDepth.press('Enter');
  assert.equal(await page.evaluate(() => document.activeElement?.dataset.outlineControl), 'depth');
  assert.equal(await structureView().locator('.knowledge-heading-list > li').count(), 8);
  assert.match(await structureRow(firstRoot.key).locator('.outline-jump').getAttribute('aria-label'), /1 个直接子项／2 个后代子项$/);
  await structureView().getByLabel('标题与题注', { exact: true }).selectOption('caption');
  await structureView().getByLabel('标题与题注', { exact: true }).focus();
  await structureView().getByLabel('标题与题注', { exact: true }).press('Home');
  assert.equal(await page.evaluate(() => document.activeElement?.dataset.outlineControl), 'filter');
  await structureView().getByLabel('标题与题注', { exact: true }).press('End');
  assert.equal(await page.evaluate(() => document.activeElement?.dataset.outlineControl), 'filter');
  assert.equal(await structureView().locator('.knowledge-heading-list > li').count(), 4);
  const caption = captionItems.find(item => item.kind === 'figure'); assert.ok(caption.range, 'Authored native caption has a verified exact source address');
  await structureRow(caption.key).locator('.outline-jump').press('Enter');
  await until(async () => { const actual = await cm().evaluate(element => { const range = element.cmTile.root.view.state.selection.main; return { from: range.from, to: range.to }; }); return actual.from === caption.range.utf16From && actual.to === caption.range.utf16To; }, 'caption exact source range');
  await structureView().getByLabel('标题与题注', { exact: true }).selectOption('all');
  await click('标题导图', structureView()); assert.equal(await structureView().locator('g[data-graph-key]').count(), 5, 'Heading mind map omits caption ancestry');
  await click('大纲列表', structureView());
  await expectSource(dirtyParent); assert.equal(await disk(parent.path), savedParent); assert.equal(mutations.length, structureMutations);
  result.snapshots.push({ headingSubtree: nativeHeadings });
  await pass('The same accepted native render drives actual heading counts, keyboard folding and full/subtree controls without extra knowledge reads; search reveals collapsed ancestors without changing their real counts, clears back to the saved fold and traverses duplicate matches in both directions with wraparound, without author writes.');

  phase = 'explicit-inspector-commands';
  for (let repetition = 0; repetition < 2; repetition++) {
    await command('导入或浏览图片与资源');
    await nodesView().waitFor({ state: 'visible' });
    assert.equal(path.normalize(await nodesView().locator('.node-browser-path').getAttribute('title')), path.normalize(parentFolder));
    await expectSource(dirtyParent);
  }
  await expectSource(dirtyParent); await workspaceCommand(page, 'knowledge', '链接与批注'); await until(async () => !await knowledge().isVisible(), 'toolbar still closes its own active inspector');
  await workspaceCommand(page, 'knowledge', '链接与批注'); await knowledge().waitFor({ state: 'visible' }); await expectSource(dirtyParent); assert.equal(await disk(parent.path), savedParent); assert.equal(await disk(image.path), savedImage);
  await pass('Repeated graph and heading commands keep their requested views visible, while the resource command reveals the independent Nodes owner; the knowledge toolbar still closes/reopens its own retained owner without saving or changing the author Draft.');
  result.expectedHttpStates = []; result.unexpectedHttpErrors = [];
  for (const event of result.httpErrors) { let code; try { code = JSON.parse(event.body).error?.code; } catch { /* An unclassified response stays unexpected. */ }
    const expected = event.status === 409 && (code === 'stale_draft' && ['/api/render', '/api/knowledge/read'].includes(event.route)
      || code === 'invalidField' && event.phase === 'cancel-rejected-first-assignment' && event.route === '/api/navigation/create-tag');
    (expected ? result.expectedHttpStates : result.unexpectedHttpErrors).push({ ...event, code });
  }
  assert.deepEqual(result.pageErrors, []); assert.deepEqual(result.unexpectedHttpErrors, []); assert.deepEqual(result.observerErrors ?? [], []);
  assert.equal(sha(await readFile(exe)), result.sha256); assert.equal(sha(await readFile(new URL(import.meta.url))), result.harnessSha256); assert.equal(await disk(parent.path), savedParent); assert.equal(await disk(image.path), savedImage);
  await page.screenshot({ path: path.join(out, 'completion.png'), fullPage: true }); result.completed = true;
} catch (error) {
  result.failure = String(error.stack ?? error); result.failedPhase = phase;
  if (page && !page.isClosed()) { await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {}); await writeFile(path.join(out, 'failure-ui.txt'), await page.locator('body').innerText().catch(() => '')).catch(() => {}); }
  result.failureDiagnostics = [await failureSnapshot('at-deadline')];
  if (result.queryActions?.at(-1)?.completed === false && result.queryActions.at(-1).phase === phase && String(error).includes('Timed out:')) {
    const pending = [...observedRequests.values()].filter(entry => entry.route === '/api/query/run' && !entry.bodyRecordedAt && !entry.failedAt && (entry.status === undefined || entry.status < 400));
    result.failureReadOnlyObservation = { originalAssertionFailed: true, failedWait: result.queryActions.at(-1).waits.at(-1), additionalObservationLimitMs: 35000, pendingIds: pending.map(entry => entry.id), startedAt: new Date().toISOString(), note: 'No UI input, retry or product request is sent. Late original receipts remain diagnostic and never change the failed outcome.' };
    const deadline = performance.now() + 35000; while (pending.some(entry => !entry.bodyRecordedAt && !entry.failedAt) && performance.now() < deadline) await pause(100);
    result.failureReadOnlyObservation.finishedAt = new Date().toISOString(); result.failureReadOnlyObservation.originalReceipts = pending.map(entry => ({ ...entry }));
    result.failureDiagnostics.push(await failureSnapshot('after-passive-observation'));
  }
  throw error;
} finally {
  await browser?.close(); await stop(); result.finishedAt = new Date().toISOString();
  result.ownedProcessesClosed = (!child || child.exitCode !== null || child.signalCode !== null) && (!browser || !browser.isConnected());
  result.previewRequests = previewRequests; result.mutations = mutations; result.queryReceipts = queryReceipts; result.knowledgeReceipts = knowledgeReceipts; result.renderReceipts = renderReceipts;
  await writeFile(path.join(out, 'results.json'), stringifyServeJson(result, 2)); await writeFile(path.join(out, 'server.stderr.log'), stderr);
  console.log(JSON.stringify({ out, completed: result.completed, failedPhase: result.failedPhase, cases: result.cases.length, ownedProcessesClosed: result.ownedProcessesClosed }));
}
