import { spawnServe, serveReady, stopServe, stringifyServeJson } from './_cli_serve.mjs';
import { createdDocument } from './_creation_result.mjs';
// Real hosted Query/embedded acceptance in owned fixtures. EXE and a new evidence
// directory are required. Playwright comes from a general installation; Python
// (only ZIP verification) must be supplied as an absolute general runtime path.
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { createHash, randomUUID, randomBytes } from 'node:crypto';
import assert from 'node:assert/strict';
const require = createRequire(import.meta.url), { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]), python = process.env.WEFTEXT_PYTHON;
assert.ok(python && path.isAbsolute(python), 'WEFTEXT_PYTHON must name the absolute general Python runtime');
await mkdir(out, { recursive: false });
const sha = bytes => createHash('sha256').update(bytes).digest('hex'), pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const result = { exe, sha256: sha(await readFile(exe)), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(), cases: [], pageErrors: [], consoleErrors: [], completed: false };
const token = Object.fromEntries(['alice', 'bob', 'admin'].map(actor => [actor, randomBytes(32).toString('hex')]));
let local, server, browser, page, base, localToken, stderr = '';
// Windows can briefly deny this observer's read because of sharing
// contention. Keep the exact disk-byte assertions, bound only EBUSY observation,
// and never retry a different error or read a path outside this owned fixture.
async function readOtherFixture(file) {
  assert.equal(path.resolve(file), path.join(out, 'library', 'Other', 'Other.typ'));
  const started = performance.now(); let retries = 0;
  for (;;) {
    try {
      const source = await readFile(file, 'utf8');
      (result.diskReadObservations ??= []).push({ retries, elapsedMs: performance.now() - started, completed: true });
      return source;
    } catch (error) {
      if (error.code !== 'EBUSY' || performance.now() - started >= 5000) {
        (result.diskReadObservations ??= []).push({ retries, elapsedMs: performance.now() - started, completed: false, error: String(error) });
        throw error;
      }
      retries++; await pause(25);
    }
  }
}

async function start(args, hosted = false) {
  const process = spawnServe(exe, args, { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] }); let stdout = '';
  process.stdout.on('data', bytes => stdout += bytes); process.stderr.on('data', bytes => stderr += bytes);
  for (let i = 0; i < 300; i++) { const match = serveReady(process); if (match) { base = match.base; localToken = match.token; return process; } if (process.exitCode !== null) throw new Error(stderr); await pause(100); }
  await stopServe(process); throw new Error('startup timeout');
}
async function stop(process) { await stopServe(process); }
async function api(route, body, actor) { const response = await fetch(base + route, { method: body === undefined ? 'GET' : 'POST', headers: { 'Content-Type': 'application/json', ...(actor ? { Authorization: 'Bearer ' + token[actor] } : { 'X-Weftext-Token': localToken }) }, body: body === undefined ? undefined : JSON.stringify(body) }); const value = await response.json(); assert.ok(response.ok, JSON.stringify(value)); return value; }
async function click(name, host = page) { await host.getByRole('button', { name, exact: true }).click(); }
async function connected(target, actor) { await target.goto(base); await target.getByLabel('访问凭据', { exact: true }).fill(token[actor]); await click('连接', target); await target.getByLabel('托管完整查询计划（JSON）', { exact: true }).waitFor(); await target.getByLabel('查询对象 shared', { exact: true }).waitFor(); await target.getByLabel('Office 输出格式', { exact: true }).waitFor(); }
async function plan(value, parameters = {}, target = page) { await target.getByLabel('托管完整查询计划（JSON）', { exact: true }).fill(JSON.stringify(value, null, 2)); await target.getByLabel('托管查询参数（原生值 JSON）', { exact: true }).fill(JSON.stringify(parameters, null, 2)); }
async function query(target = page) { const response = target.waitForResponse(response => response.url().endsWith('/api/hosted/query')); await click('运行托管高级查询', target); const received = await response; assert.equal(received.status(), 200); const value = await received.json(); await target.waitForFunction(() => [...document.querySelectorAll('#query button')].some(button => button.textContent === '运行托管高级查询' && !button.disabled)); await target.getByRole('table', { name: '托管高级查询冻结结果' }).waitFor(); return value; }
async function scopes(target = page) { await target.getByLabel('查询对象 shared', { exact: true }).check(); await target.getByLabel('查询对象 other', { exact: true }).check(); await target.getByLabel('获权字段 people/parents', { exact: true }).uncheck(); }
async function policy(edit) { const current = await api('/api/hosted/admin/policy', undefined, 'admin'); edit(current.users); return api('/api/hosted/admin/policy', { expectedGeneration: current.generation, users: current.users }, 'admin'); }
const column = (expr = 'status', name = 'status') => ({ name, expr });
try {
  const data = path.join(out, 'state'), root = path.join(out, 'library'); local = await start(['--data-dir', data]); await api('/api/library/create', { path: root, operationId: randomUUID() });
  const originalSchema = await api('/api/data/schema', { root }), schema = JSON.parse(originalSchema.source), schemaOp = randomUUID(); schema.fields.find(field => field.id === 'tasks/priority').shape = { kind: 'scalar', semantic: 'int' };
  const schemaPlan = await api('/api/data/schema/prepare', { root, source: JSON.stringify(schema, null, 2), beforeSource: originalSchema.source, beforeRevision: originalSchema.revision, operationId: schemaOp }); await api('/api/data/schema/install', { operationId: schemaOp, planId: schemaPlan.planId });
  const shared = createdDocument(await api('/api/note/create', { root, parent: root, name: 'Shared', operationId: randomUUID() })), other = createdDocument(await api('/api/note/create', { root, parent: root, name: 'Other', operationId: randomUUID() }));
  const inventory = await api('/api/library/open', { path: root }), sharedRef = `weftext-node://${inventory.workspaceId}/${shared.nodeId}`, otherRef = `weftext-node://${inventory.workspaceId}/${other.nodeId}`;
  const relation = schema.relations.find(item => item.field === 'people/parents').id;
  const source = (node, name, status, parents = '') => `#metadata((id: "${node.nodeId}", types: ("task", "person"), status: "${status}"${parents}, private: "PRIVATE_${name}_NEVER_QUERY")) <weftext>\n#set document(title: [文档${name}])\n#title()\n====== 深标题${name}😀\n正文${name}🌿\n`;
  await api('/api/save', { sessionId: shared.sessionId, baseId: shared.baseId, source: source(shared, 'Shared', 'open', `, parents: ((target: (ref: "${otherRef}"),),)`), operationId: randomUUID() });
  await api('/api/save', { sessionId: other.sessionId, baseId: other.baseId, source: source(other, 'Other', 'done'), operationId: randomUUID() });
  const definitionPlan = { where: 'status == "open"', select: [column()] };
  async function define(value, key) { const opened = await api('/api/open', { path: other.path, root }); return api('/api/query/definition/save', { sessionId: opened.sessionId, source: opened.source, sourceHash: sha(opened.source), serial: 1, baseId: opened.baseId, operationId: randomUUID(), label: 'weftext-query-state', ...(key ? { key } : {}), plan: value }); }
  const first = await define(definitionPlan), key = first.definition.key, oldRevision = first.definitionRevision; await define({ ...definitionPlan, where: 'status == "done"' }, key);
  const originalOther = await readOtherFixture(path.join(root, 'Other/Other.typ')); await stop(local); local = null;
  const queryRights = { definitions: true, definitionHistory: true, products: { title: true, body: true, headings: true, types: true, self: true }, relations: [relation] };
  const fields = { 'tasks/status': { read: true }, 'people/parents': { read: true } }, office = { export: true, download: true };
  const config = { deployment: 'query-browser', bind: '127.0.0.1:0', root, adminTokenSha256: sha(token.admin), files: { shared: { path: 'Shared/Shared.typ', title: '共同文档', editable: true }, other: { path: 'Other/Other.typ', title: '定义文档', editable: true } }, users: {
    alice: { enabled: true, tokenSha256: sha(token.alice), grants: { shared: { read: true, write: true, fields, query: queryRights, office }, other: { read: true, write: true, fields, query: queryRights, office } } },
    bob: { enabled: true, tokenSha256: sha(token.bob), grants: { shared: { fields, query: { ...queryRights, definitionHistory: false }, office }, other: { fields, query: { ...queryRights, definitionHistory: false }, office } } },
  } };
  const configPath = path.join(out, 'server.json'); await writeFile(configPath, JSON.stringify(config, null, 2)); server = await start(['--data-dir', data, '--server-config', configPath], true);
  browser = await chromium.launch({ channel: 'chrome', headless: true }); result.browser = browser.version();
  const context = await browser.newContext({ viewport: { width: 1440, height: 1100 }, acceptDownloads: true }), bobContext = await browser.newContext({ viewport: { width: 1280, height: 1050 } }); page = await context.newPage(); const bob = await bobContext.newPage(); let localConsumerCalls = 0;
  for (const target of [page, bob]) { target.on('pageerror', error => result.pageErrors.push(String(error))); target.on('console', message => { if (message.type() === 'error') result.consoleErrors.push(message.text()); }); target.on('dialog', dialog => dialog.accept()); target.on('request', request => { const pathname = new URL(request.url()).pathname; if (pathname.startsWith('/api/') && !pathname.startsWith('/api/hosted/')) localConsumerCalls++; }); }
  await connected(bob, 'bob'); await scopes(bob); assert.equal(await bob.locator('#documents button').count(), 0);
  const parameterPlan = { select: [column()], where: 'status == wanted', advanced: { parameters: [{ name: 'wanted', shape: { kind: 'scalar', semantic: 'text' } }] } };
  await plan(parameterPlan, { wanted: { kind: 'text', value: 'open' } }, bob); const narrow = await query(bob); assert.equal(narrow.result.rows.length, 1); assert.equal(narrow.result.rows[0].canOpen, false); assert.equal(await bob.locator('#query').getByRole('button', { name: '打开获权正文', exact: true }).count(), 0); assert.ok(!(await bob.locator('body').innerText()).includes('PRIVATE_'));
  result.cases.push('field-only actor runs native typed parameters without source access, local API requests or navigation authority');
  await connected(page, 'alice'); await scopes(); const branch = { select: [column()], where: 'status == "open"' };
  await plan({ advanced: { unionAll: [branch, branch] } }); const union = await query(); assert.equal(union.result.rows.length, 2); assert.notEqual(union.result.rows[0].key, union.result.rows[1].key);
  await plan({ advanced: { unionAll: [branch, branch], distinct: [column('status', 'uniqueStatus')] } }); const distinct = await query(); assert.equal(distinct.result.rows.length, 1); assert.equal(distinct.result.rows[0].readOnly, true); assert.equal(await page.locator('#query').getByRole('button', { name: '打开获权正文', exact: true }).count(), 0);
  for (const name of ['title', 'body', 'headings']) await page.getByLabel(`获权语义产品 ${name}`, { exact: true }).check(); await plan({ select: [column('body', 'body'), column('title', 'title'), column('headings', 'headings')] }); const semantic = await query(); assert.deepEqual(semantic.columns, ['body', 'title', 'headings']); assert.match(await page.getByRole('table', { name: '托管高级查询冻结结果' }).innerText(), /深标题Shared/); assert.ok(!JSON.stringify(semantic).includes('PRIVATE_') && !JSON.stringify(semantic).includes('#metadata'));
  for (const name of ['title', 'body', 'headings']) await page.getByLabel(`获权语义产品 ${name}`, { exact: true }).uncheck(); for (const name of ['self', 'types']) await page.getByLabel(`获权语义产品 ${name}`, { exact: true }).check(); await page.getByLabel('获权字段 people/parents', { exact: true }).check(); await page.getByLabel(`获权关系 ${relation}`, { exact: true }).check();
  const seed = { where: 'self == origin', advanced: { parameters: [{ name: 'origin', shape: { kind: 'scalar', semantic: 'node_ref' }, default: { kind: 'text', value: sharedRef } }] } };
  await plan({ select: [column('self', 'self')], advanced: { related: { relation, from: seed, direction: 'forward' } } }); const related = await query(); assert.equal(related.result.rows[0].cells.self.value.value, otherRef);
  result.cases.push('real union preserves occurrences, distinct rows stay derived, semantic products retain exact column order and deep headings, and independently granted relation follows the actual ref');
  for (const name of ['self', 'types']) await page.getByLabel(`获权语义产品 ${name}`, { exact: true }).uncheck(); await page.getByLabel('获权字段 people/parents', { exact: true }).uncheck(); await page.getByLabel(`获权关系 ${relation}`, { exact: true }).uncheck();
  await page.locator('#query details').filter({ hasText: '保存定义 · current' }).locator('summary').click(); await page.getByLabel('保存查询定义对象', { exact: true }).selectOption('other'); await page.getByLabel('保存查询定义', { exact: true }).selectOption(key); await click('读取获权保存定义'); await page.locator('#query').getByText(/当前定义已读取/).waitFor(); await click('以此保存定义创建临时调用'); const current = await query(); assert.equal(current.result.rows[0].cells.status.value.value, 'done');
  await page.getByLabel('保存定义读取方式', { exact: true }).selectOption('pinned'); await page.getByLabel('准确固定定义版本', { exact: true }).fill(oldRevision); await click('读取获权保存定义'); await page.locator('#query').getByText(new RegExp(`准确固定定义已读取.*${oldRevision}`)).waitFor(); await click('以此保存定义创建临时调用'); const pinned = await query(); assert.equal(pinned.result.rows[0].cells.status.value.value, 'open'); assert.equal(await readOtherFixture(path.join(root, 'Other/Other.typ')), originalOther);
  result.cases.push('saved current and exact pinned definitions construct the original advanced.call, show the actual selected revision and keep current author bytes untouched');
  await click('定义文档'); await page.locator('#source [data-query-label="weftext-query-state"]').waitFor(); const embedded = page.locator('#source [data-query-label="weftext-query-state"]').first(); await embedded.getByText('done', { exact: true }).waitFor();
  await click('V01 源码'); await click('查看正文查询预览'); await page.locator('#source .embedded-query').waitFor(); await click('V02 带样式源码'); assert.ok((await page.locator('#source .cm-content').innerText()).includes('weftext-query-state')); await click('V03 按需语法'); await click('展开查询源码'); await page.locator('#source .embedded-query-source-host .cm-content[contenteditable=true]').waitFor(); await click('V04 可视写作');
  const v4 = page.locator('#source [data-query-label="weftext-query-state"]').first(); await v4.getByRole('button', { name: '查询构建器', exact: true }).click(); await page.getByLabel('weftext-query-state 条件（CEL）', { exact: true }).fill('status == "open"');
  const editResponse = page.waitForResponse(response => response.url().endsWith('/api/hosted/query/embedded/edit')); await click('应用查询修改'); assert.equal((await editResponse).status(), 200);
  await page.waitForFunction(() => Object.keys(sessionStorage).filter(key => key.startsWith('weftext-hosted-draft:')).some(key => /status ==.*open/.test(JSON.parse(sessionStorage[key]).source))); assert.equal(await readOtherFixture(path.join(root, 'Other/Other.typ')), originalOther);
  await click('V01 源码'); assert.match(await page.locator('#source .cm-content').innerText(), /status ==.*open/); await page.locator('#source .cm-content').press('ControlOrMeta+z');
  await page.waitForFunction(() => Object.keys(sessionStorage).filter(key => key.startsWith('weftext-hosted-draft:')).some(key => /status ==.*done/.test(JSON.parse(sessionStorage[key]).source))); assert.match(await page.locator('#source .cm-content').innerText(), /status ==.*done/);
  await click('V04 可视写作'); await page.locator('#source .embedded-query').getByRole('button', { name: '编辑源码', exact: true }).click(); await page.getByRole('dialog').getByRole('textbox').fill('尚未应用提案😀'); await click('取消并保留共享草稿'); await click('V05 阅读'); assert.equal(await page.locator('#source .embedded-query [data-query-edit]').count(), 0); assert.equal(await readOtherFixture(path.join(root, 'Other/Other.typ')), originalOther);
  result.cases.push('embedded native definitions use all five views, builder edit joins the one source Undo, cancellation changes no source, and V05 exposes no definition writer');
  await context.setOffline(true); await page.getByLabel('托管完整查询计划（JSON）', { exact: true }).fill('{ invalid Chinese😀'); await page.getByLabel('托管查询参数（原生值 JSON）', { exact: true }).fill('{ incomplete'); await context.setOffline(false); await page.reload(); await page.getByLabel('Office 输出格式', { exact: true }).waitFor(); assert.equal(await page.getByLabel('托管完整查询计划（JSON）', { exact: true }).inputValue(), '{ invalid Chinese😀'); assert.equal(await page.getByLabel('托管查询参数（原生值 JSON）', { exact: true }).inputValue(), '{ incomplete');
  await plan(branch); let release, observed, rejectHeld; const held = new Promise((resolve, reject) => { observed = resolve; rejectHeld = reject; }), resume = new Promise(resolve => release = resolve); await page.route('**/api/hosted/query', async route => { try {
    const response = await route.fetch(); if (response.status() !== 200) { result.unexpectedQuery = { status: response.status(), request: route.request().postDataJSON(), body: await response.json() }; await route.fulfill({ response }); rejectHeld(new Error('held Query refused: ' + JSON.stringify(result.unexpectedQuery))); return; }
    observed(); await resume; await route.fulfill({ response });
  } catch (error) { rejectHeld(error); await route.abort('failed').catch(() => {}); } });
  await click('运行托管高级查询'); await held; const newer = { select: [column()], where: 'status == "done"' }; await plan(newer); const parameters = page.getByLabel('托管查询参数（原生值 JSON）', { exact: true }); await parameters.focus(); await parameters.evaluate(element => element.setSelectionRange(1, 1)); release(); await pause(500); assert.equal(await page.getByLabel('托管完整查询计划（JSON）', { exact: true }).inputValue(), JSON.stringify(newer, null, 2)); assert.equal(await parameters.evaluate(element => element.selectionStart), 1); assert.equal(await parameters.evaluate(element => element === document.activeElement), true); await page.unroute('**/api/hosted/query'); await query();
  result.cases.push('offline invalid JSON is retained across reload and a held old Query response cannot overwrite newer plan, native parameters, focus or selection');
  await click('将原冻结查询交给 Office'); assert.equal(await page.getByLabel('Office 输出格式', { exact: true }).inputValue(), 'xlsx'); const frozenResponse = page.waitForResponse(response => response.url().endsWith('/api/hosted/office/export/prepare')); await click('准备 Office 输出'); const frozen = await (await frozenResponse).json(); await page.getByLabel('接受本次输出的损失', { exact: true }).check(); await click('确认 Office 输出'); await page.waitForFunction(() => [...document.querySelectorAll('#office button')].some(button => button.textContent === '下载已确认 Office 文件' && !button.disabled)); const delivered = page.waitForEvent('download'); await click('下载已确认 Office 文件'); const download = await delivered; const output = path.join(out, 'query-frozen.xlsx'); await download.saveAs(output); const bytes = await readFile(output); assert.equal(sha(bytes), frozen.digest);
  const sheet = execFileSync(python, ['-c', 'import sys,zipfile,io\nz=zipfile.ZipFile(io.BytesIO(sys.stdin.buffer.read()));sys.stdout.buffer.write(z.read("xl/worksheets/weftext1.xml"))'], { input: bytes, windowsHide: true }).toString(); assert.match(sheet, /done/); result.cases.push('Office consumes the displayed advanced Query handle and downloads the confirmed frozen workbook with an exact digest');
  let releaseRevoked, observedRevoked, rejectRevoked; const revokedHeld = new Promise((resolve, reject) => { observedRevoked = resolve; rejectRevoked = reject; }), revokedResume = new Promise(resolve => releaseRevoked = resolve); await page.route('**/api/hosted/query', async route => { try {
    const response = await route.fetch(); if (response.status() !== 200) { result.unexpectedQuery = { status: response.status(), request: route.request().postDataJSON(), body: await response.json() }; await route.fulfill({ response }); rejectRevoked(new Error('revocation Query refused: ' + JSON.stringify(result.unexpectedQuery))); return; }
    observedRevoked(); await revokedResume; await route.fulfill({ response });
  } catch (error) { rejectRevoked(error); await route.abort('failed').catch(() => {}); } });
  await click('运行托管高级查询'); await revokedHeld; const oldKeys = await page.evaluate(() => Object.keys(sessionStorage).filter(key => /^weftext-hosted-(query|office|field|draft|review):/.test(key))); assert.ok(oldKeys.some(key => key.startsWith('weftext-hosted-office:')) && oldKeys.some(key => key.startsWith('weftext-hosted-draft:')));
  await page.evaluate(() => { window.queryOriginalRemove = Storage.prototype.removeItem; Storage.prototype.removeItem = function(key) { if (/^weftext-hosted-(query|office|field|draft|review):/.test(key)) throw new DOMException('injected removal denial', 'SecurityError'); return window.queryOriginalRemove.call(this, key); }; });
  await policy(users => users.alice.grants = {}); await page.locator('#status').filter({ hasText: '浏览器拒绝删除部分恢复缓存' }).waitFor({ timeout: 12000 }); releaseRevoked(); await pause(500); assert.equal(await page.locator('#query').isHidden(), true); assert.equal(await page.locator('#office').isHidden(), true); assert.equal(await page.locator('#review').isHidden(), true); assert.equal(await page.locator('#fields').isHidden(), true); assert.equal(await page.locator('#source .cm-content').count(), 0); assert.equal(await page.locator('#query table').count(), 0);
  assert.ok(await page.evaluate(keys => keys.every(key => sessionStorage.getItem(key) !== null), oldKeys));
  await click('全部文档'); await page.getByLabel('Office 输出格式', { exact: true }).waitFor(); assert.equal(await page.locator('#office').getByText('原 Office 输出', { exact: true }).count(), 0); assert.equal(await page.locator('#query table').count(), 0); assert.equal(await page.getByLabel('托管完整查询计划（JSON）', { exact: true }).inputValue(), JSON.stringify({ scope: { descendants: true, lifecycle: 'live' }, types: [], select: [], order: [] }, null, 2));
  // The old protected records are physically still present, yet refreshing and
  // reconnecting after the new server epoch must never select their namespace.
  await page.reload(); await page.getByLabel('Office 输出格式', { exact: true }).waitFor(); assert.equal(await page.locator('#office').getByText('原 Office 输出', { exact: true }).count(), 0); assert.equal(await page.locator('#query table').count(), 0); assert.equal(await page.locator('#source .cm-content').count(), 0);
  await page.evaluate(() => { window.queryOriginalRemove = Storage.prototype.removeItem; Storage.prototype.removeItem = function(key) { if (/^weftext-hosted-(query|office|field|draft|review):/.test(key)) throw new DOMException('injected removal denial', 'SecurityError'); return window.queryOriginalRemove.call(this, key); }; });
  await click('退出并清除本页缓存'); await page.getByLabel('访问凭据', { exact: true }).fill(token.alice); await click('连接'); await page.getByLabel('Office 输出格式', { exact: true }).waitFor(); assert.equal(await page.locator('#office').getByText('原 Office 输出', { exact: true }).count(), 0); assert.equal(await page.locator('#query table').count(), 0); assert.ok(await page.evaluate(keys => keys.every(key => sessionStorage.getItem(key) !== null), oldKeys));
  await page.evaluate(() => { Storage.prototype.removeItem = window.queryOriginalRemove; delete window.queryOriginalRemove; }); await click('退出并清除本页缓存'); assert.deepEqual(await page.evaluate(() => Object.keys(sessionStorage).filter(key => /^weftext-hosted-(query|office|field|draft|review):/.test(key))), []);
  await plan(branch, {}, bob); assert.equal((await query(bob)).result.rows[0].cells.status.value.value, 'open'); result.cases.push('injected removeItem failure clears all protected views, rejects late Query delivery, and server actor-epoch namespaces prevent old preview/source/plan restoration across refresh, reload and reconnect; unrelated actor remains authorized');
  assert.equal(localConsumerCalls, 0); assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) { result.failure = String(error.stack ?? error); if (page) await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {}); throw error; }
finally { await browser?.close(); await stop(local); await stop(server); await writeFile(path.join(out, 'results.json'), stringifyServeJson(result, 2)); await writeFile(path.join(out, 'server.stderr.log'), stderr); console.log(JSON.stringify({ out, cases: result.cases, completed: result.completed, pageErrors: result.pageErrors })); }
