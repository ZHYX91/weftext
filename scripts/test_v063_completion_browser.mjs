// Actual final-candidate daily task chain. Every product view comes from the
// supplied executable's embedded frontend and actual HTTP/Core fixture data.
// Headless keyboard/text events are not OS IME or reference-host acceptance.
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
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
  exe, sha256: sha(await readFile(exe)), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(),
  startedAt: new Date().toISOString(), completed: false, cases: [], pageErrors: [], httpErrors: [], requests: [], snapshots: [],
  limits: ['No OS IME, assistive technology, touch, real Office host or reference-software parity acceptance.',
    'An ordinary complete-task chain, not the 5,000-tag/near-budget/over-budget scale matrix.',
    'Local owner scope; independently authorized hosted and synchronization cases run through their own product suites.'],
};
const root = path.join(out, 'TEST-library'), data = path.join(out, 'private-state');
let child, browser, context, page, base, token, stderr = '', phase = 'setup';
let image, plain, parent, farImage, savedImage, savedParent, dirtyImage, dirtyParent;
const queryReceipts = [], knowledgeReceipts = [], previewRequests = [], mutations = [];
let caseStartedAt = performance.now();

async function until(check, label = 'condition', timeout = 30000) {
  const deadline = Date.now() + timeout; let last;
  while (Date.now() < deadline) { try { const value = await check(); if (value) return value; } catch (error) { last = error; } await pause(100); }
  throw last ?? new Error('Timed out: ' + label);
}
async function start(args = []) {
  child = spawn(exe, ['--no-browser', '--data-dir', data, ...args], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  let stdout = ''; child.stdout.on('data', bytes => stdout += bytes); child.stderr.on('data', bytes => stderr += bytes);
  await until(() => { const match = stdout.match(/(http:\/\/127\.0\.0\.1:\d+)\/#token=(\w+)/); if (match) { base = match[1]; token = match[2]; return true; } if (child.exitCode !== null) throw new Error('Owned candidate exited: ' + stderr); return false; }, 'owned local candidate start');
}
async function stop() {
  if (child?.exitCode === null && child.signalCode === null) { const done = new Promise(resolve => child.once('exit', resolve)); child.kill(); await done; }
}
async function api(route, body) {
  const response = await fetch(base + route, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': token }, body: JSON.stringify(body) });
  const value = await response.json(); assert.equal(response.status, 200, `${route}: ${JSON.stringify(value)}`); return value;
}
async function commit(prepared) {
  const preview = prepared.preview ?? prepared;
  return api('/api/operations/commit', { operationId: preview.operationId, planId: preview.planId });
}
async function save(doc, source) {
  const fresh = await api('/api/open', { root, path: doc.path });
  await api('/api/save', { sessionId: fresh.sessionId, baseId: fresh.baseId, source, operationId: randomUUID() });
  return api('/api/open', { root, path: doc.path });
}
async function note(name, container = root, body = '') {
  const doc = await api('/api/note/create', { root, parent: container, name, operationId: randomUUID() });
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
  const fresh = await api('/api/open', { root, path: doc.path });
  await commit(await api('/api/resources/prepare-import', { sessionId: fresh.sessionId, name: 'TEST-pixels.png', bytes: png().toString('base64'), operationId: randomUUID() }));
  return save(doc, doc.source + '\n\n#image("TEST-pixels.png", width: 24pt)\n');
}
function observe(target) {
  target.on('pageerror', error => result.pageErrors.push({ phase, error: String(error) }));
  target.on('request', request => {
    const route = new URL(request.url()).pathname; if (!route.startsWith('/api/')) return;
    const body = request.method() === 'POST' ? request.postDataJSON() : undefined;
    if (route === '/api/navigation/preview') previewRequests.push({ phase, ...body });
    if (/\/(prepare|commit|cancel)$/.test(route) || route.startsWith('/api/navigation/') && /-tag$/.test(route)) mutations.push({ phase, route, body });
    result.requests.push({ phase, route, method: request.method() });
  });
  target.on('response', async response => {
    const route = new URL(response.url()).pathname, capturedPhase = phase;
    if (!route.startsWith('/api/')) return;
    try {
      if (response.status() >= 400) result.httpErrors.push({ phase: capturedPhase, route, status: response.status(), body: await response.text() });
      if (route === '/api/query/run' && response.ok()) queryReceipts.push({ phase: capturedPhase, request: response.request().postDataJSON(), result: await response.json() });
      if (route === '/api/knowledge/read' && response.ok()) knowledgeReceipts.push({ phase: capturedPhase, request: response.request().postDataJSON(), result: await response.json() });
    } catch (error) { if (!target.isClosed()) (result.observerErrors ??= []).push(String(error)); }
  });
}
const click = (name, host = page) => host.getByRole('button', { name, exact: true }).click();
const cm = () => page.locator('.source-host .cm-content');
const queryPanel = () => page.locator('.query-panel');
const knowledge = () => page.getByLabel('知识与批注', { exact: true });
const contents = () => page.getByRole('region', { name: '节点内容', exact: true });
async function source() { return cm().evaluate(element => { const view = element.cmTile?.root?.view; if (view?.contentDOM !== element) throw new Error('Actual retained editor unavailable'); return view.state.doc.toString(); }); }
async function expectSource(expected) { await until(async () => { assert.equal(await source(), expected); return true; }, 'exact current author Draft'); }
async function disk(file) { for (let attempt = 0; attempt < 20; attempt++) { try { return await readFile(file, 'utf8'); } catch (error) { if (!['EBUSY', 'EACCES'].includes(error.code) || attempt === 19) throw error; (result.fileReadRetries ??= []).push({ phase, file, attempt, code: error.code }); await pause(100); } } }
async function command(label) { await page.keyboard.press('Control+Shift+p'); const dialog = page.getByRole('dialog', { name: '命令', exact: true }); await dialog.getByRole('searchbox').fill(label); await dialog.getByRole('searchbox').press('Enter'); }
async function quick(name) { await page.keyboard.press('Control+p'); const dialog = page.getByRole('dialog', { name: '快捷打开', exact: true }); await dialog.getByRole('searchbox').fill(name); await dialog.getByRole('searchbox').press('Enter'); }
async function panelDetails(label, host = queryPanel()) { const summary = host.getByText(label, { exact: true }); const details = summary.locator('..'); if (!await details.evaluate(element => element.open)) await summary.click(); return details; }
async function showQuery() { if (!await queryPanel().isVisible()) await click('查询与视图'); }
async function pass(message) { const now = performance.now(); result.cases.push({ phase, message, elapsedMs: Math.round((now - caseStartedAt) * 100) / 100, input: 'Actual DOM mouse and keyboard; automation and observable waits included' }); caseStartedAt = now; await writeFile(path.join(out, 'progress.json'), JSON.stringify(result, null, 2)); console.log('PASS', message); }
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
  const start = queryReceipts.length; await click('查询', queryPanel());
  const receipt = await until(() => queryReceipts.slice(start).find(item => !predicate || JSON.stringify(item.request.plan).includes(predicate)), 'actual Query receipt');
  assert.equal(receipt.result.rows.length, count, JSON.stringify(receipt.result)); assert.equal(receipt.result.coverage, 'complete');
  await until(() => queryPanel().locator('.query-card').count().then(n => n === count), 'rendered Query results'); return receipt;
}
async function refreshKnowledge(expected) {
  await until(async () => (await page.locator('.status-right').innerText()).includes('草稿预览已更新'), 'native render ready');
  const start = knowledgeReceipts.length; await click('刷新', knowledge());
  return until(() => knowledgeReceipts.slice(start).find(item => item.request.context?.source === expected), 'actual same-Draft knowledge receipt');
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
  for (let i = 0; i < 18; i++) await note('M-' + String(i).padStart(2, '0'), parentFolder, 'Owned spacer note.');
  farImage = await attachImage(await note('Z-Image', parentFolder, 'Owned image outside the initial node-list viewport.'));
  const reference = doc => `weftext-node://${doc.workspaceId}/${doc.nodeId}`;
  parent = await save(parent, parent.source + `\n\n= TEST Duplicate\n\nRoot before.\n\n== TEST Child\n\nChild content.\n\n==== TEST Deep\n\nDeep content.\n\n= TEST Duplicate\n\n#link("${reference(image)}")[first image link]\n\n#link("${reference(image)}")[second image link]\n\n#link("${reference(plain)}")[plain link]\n`);
  savedParent = parent.source;
  await stop(); await start(['--open', image.path, '--root', root]);
  browser = await chromium.launch({ headless: true, ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : { channel: 'chrome' }) });
  result.browser = browser.version(); context = await browser.newContext({ viewport: { width: 1480, height: 1080 }, reducedMotion: 'reduce' }); page = await context.newPage(); page.setDefaultTimeout(30000); observe(page);
  await page.goto(base + '/#token=' + token); await click('源码'); await expectSource(image.source);

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

  phase = 'authored-order'; await quick('TEST-Parent'); await click('源码'); await expectSource(savedParent);
  await cm().click(); await page.keyboard.press('Control+End'); await page.keyboard.insertText(' UNSAVED PARENT DRAFT😀'); dirtyParent = savedParent + ' UNSAVED PARENT DRAFT😀'; await expectSource(dirtyParent);
  await page.getByRole('button', { name: '仅浏览 TEST-Parent 文件夹', exact: true }).click(); await contents().waitFor();
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

  phase = 'lazy-thumbnails'; assert.equal(await contents().getByLabel('节点内容缩略图', { exact: true }).isChecked(), false);
  const previewStart = previewRequests.length; await contents().getByLabel('节点内容缩略图', { exact: true }).check();
  const rowFor = doc => contents().locator(`[data-entry-path=${JSON.stringify(path.dirname(doc.path))}]`);
  await rowFor(image).scrollIntoViewIfNeeded(); await until(() => rowFor(image).locator('.node-content-thumbnail').getAttribute('data-preview-state').then(state => state === 'ready'), 'visible actual image pixels');
  assert.equal(await rowFor(image).locator('img,canvas').count(), 1);
  assert.ok(!previewRequests.slice(previewStart).some(request => request.entityKey === reference(farImage)), 'An offscreen row must not read its preview before it becomes visible');
  await rowFor(plain).scrollIntoViewIfNeeded(); await until(() => rowFor(plain).locator('.node-content-thumbnail').getAttribute('data-preview-state').then(state => state === 'absent'), 'explicit no-image row'); assert.equal(await rowFor(plain).locator('img,canvas').count(), 0);
  await rowFor(farImage).scrollIntoViewIfNeeded(); await until(() => rowFor(farImage).locator('.node-content-thumbnail').getAttribute('data-preview-state').then(state => state === 'ready'), 'newly visible far image');
  assert.ok(previewRequests.slice(previewStart).some(request => request.entityKey === reference(farImage)));
  await contents().getByLabel('节点内容缩略图', { exact: true }).uncheck(); assert.equal(await contents().locator('.node-content-thumbnail,img,canvas').count(), 0); await expectSource(dirtyParent); assert.equal(await disk(image.path), savedImage); assert.equal(await disk(parent.path), savedParent);
  await pass('Only visible rows read actual exact-basis media; visible PNG pixels and explicit absent state are distinct; scrolling starts the distant image and disabling thumbnails disposes pixels without author edits.');

  phase = 'actual-graph'; await command('打开二维关系图谱'); const receipt = await refreshKnowledge(dirtyParent), snapshot = receipt.result;
  assert.equal(snapshot.graph.scope, 'authorizedNativeSourcesAndTypedReferences'); assert.ok(snapshot.graph.generation); const parentKey = `${parent.workspaceId}/${parent.nodeId}`, imageKey = `${image.workspaceId}/${image.nodeId}`;
  const duplicateEdges = snapshot.graph.edges.filter(edge => edge.source === parentKey && edge.target === imageKey && !edge.fieldId); assert.equal(duplicateEdges.length, 2); assert.notEqual(duplicateEdges[0].key, duplicateEdges[1].key);
  const graph = () => knowledge().locator('.knowledge-graph-view'); await graph().locator('g[data-graph-key]').first().waitFor();
  await graph().getByLabel('在当前图范围搜索定位', { exact: true }).fill('A-Image'); await click('定位 A-Image', graph()); await expectSource(dirtyParent); assert.equal(await disk(parent.path), savedParent);
  await graph().getByLabel('方向', { exact: true }).selectOption('out'); await graph().getByLabel('范围', { exact: true }).selectOption('global'); await click('放大', graph()); await click('向右平移', graph()); await click('适应视口', graph());
  const graphNode = graph().locator(`g[data-graph-key=${JSON.stringify(imageKey)}]`); await graphNode.focus(); await page.keyboard.press('Enter'); await click('源码'); await expectSource(dirtyImage);
  await page.keyboard.press('Alt+ArrowLeft'); await expectSource(dirtyParent); await command('打开二维关系图谱'); await refreshKnowledge(dirtyParent);
  const edge = duplicateEdges[1]; await graph().locator(`path[data-graph-key=${JSON.stringify(edge.key)}]`).focus(); await page.keyboard.press('Space'); await click('打开准确作者来源', graph());
  await until(async () => { const actual = await cm().evaluate(element => { const range = element.cmTile.root.view.state.selection.main; return { from: range.from, to: range.to }; }); return actual.from === edge.range.utf16From && actual.to === edge.range.utf16To; }, 'accurate original graph occurrence source range'); await expectSource(dirtyParent);
  result.snapshots.push({ graph: snapshot.graph, headingStructure: snapshot.headingStructure });
  await pass('Actual same-Draft knowledge receipt reaches the embedded graph with two distinct body occurrences; search/filter/pan/zoom do not write, keyboard opens the accurate retained note, Back restores Draft, and edge navigation selects its exact author occurrence.');

  phase = 'actual-heading-structure'; await command('打开只读标题结构／大纲'); const headingsReceipt = await refreshKnowledge(dirtyParent), headings = headingsReceipt.result.headingStructure;
  assert.ok(headings); const repeated = headings.items.filter(heading => heading.text === 'TEST Duplicate'); assert.equal(repeated.length, 2); assert.ok(repeated.every(heading => heading.range)); assert.notEqual(repeated[0].key, repeated[1].key);
  const heading = repeated[1], structure = () => knowledge().locator('.knowledge-heading-structure');
  await structure().getByLabel('搜索真实标题', { exact: true }).fill('TEST Duplicate'); await structure().getByLabel('搜索真实标题', { exact: true }).press('Enter');
  const headingRow = structure().locator(`li[data-heading-key=${JSON.stringify(heading.key)}]`); await headingRow.getByRole('button').first().focus(); await page.keyboard.press('Enter');
  await until(async () => { const actual = await cm().evaluate(element => { const range = element.cmTile.root.view.state.selection.main; return { from: range.from, to: range.to }; }); return actual.from === heading.range.utf16From && actual.to === heading.range.utf16To; }, 'second duplicate heading exact source range');
  await expectSource(dirtyParent); assert.equal(await disk(parent.path), savedParent);
  await cm().click(); await page.keyboard.press('Control+z'); await expectSource(savedParent); await page.keyboard.press('Control+y'); await expectSource(dirtyParent);
  await quick('A-Image'); await expectSource(dirtyImage); await page.keyboard.press('Alt+ArrowLeft'); await expectSource(dirtyParent);
  await pass('Native duplicate headings retain distinct accurate keys/ranges; keyboard search/jump chooses the second exact heading and all new read-only views preserve the original document Undo and navigation Draft.');
  result.expectedHttpStates = []; result.unexpectedHttpErrors = [];
  for (const event of result.httpErrors) { let code; try { code = JSON.parse(event.body).error?.code; } catch { /* An unclassified response stays unexpected. */ }
    const expected = event.status === 409 && code === 'stale_draft' && ['/api/render', '/api/knowledge/read'].includes(event.route);
    (expected ? result.expectedHttpStates : result.unexpectedHttpErrors).push({ ...event, code });
  }
  assert.deepEqual(result.pageErrors, []); assert.deepEqual(result.unexpectedHttpErrors, []); assert.deepEqual(result.observerErrors ?? [], []);
  assert.equal(sha(await readFile(exe)), result.sha256); assert.equal(await disk(parent.path), savedParent); assert.equal(await disk(image.path), savedImage);
  await page.screenshot({ path: path.join(out, 'completion.png'), fullPage: true }); result.completed = true;
} catch (error) {
  result.failure = String(error.stack ?? error); result.failedPhase = phase;
  if (page && !page.isClosed()) { await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {}); await writeFile(path.join(out, 'failure-ui.txt'), await page.locator('body').innerText().catch(() => '')).catch(() => {}); }
  throw error;
} finally {
  await browser?.close(); await stop(); result.finishedAt = new Date().toISOString();
  result.ownedProcessesClosed = (!child || child.exitCode !== null || child.signalCode !== null) && (!browser || !browser.isConnected());
  result.previewRequests = previewRequests; result.mutations = mutations; result.queryReceipts = queryReceipts;
  await writeFile(path.join(out, 'results.json'), JSON.stringify(result, null, 2)); await writeFile(path.join(out, 'server.stderr.log'), stderr);
  console.log(JSON.stringify({ out, completed: result.completed, failedPhase: result.failedPhase, cases: result.cases.length, ownedProcessesClosed: result.ownedProcessesClosed }));
}
