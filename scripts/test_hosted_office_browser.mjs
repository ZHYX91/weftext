// Optional real-browser acceptance. Supply EXE, a new owned evidence directory,
// WEFTEXT_PLAYWRIGHT_MODULE when necessary, and an installed Python 3 runtime.
import { createRequire } from 'node:module';
import { spawn, execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile, readdir, access } from 'node:fs/promises';
import path from 'node:path';
import { createHash, randomUUID, randomBytes } from 'node:crypto';
import assert from 'node:assert/strict';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]);
await mkdir(out, { recursive: false });
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const result = { exe, sha256: sha(await readFile(exe)), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(), cases: [], pageErrors: [], consoleErrors: [], completed: false };
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const token = Object.fromEntries(['alice', 'bob', 'writer', 'admin'].map(actor => [actor, randomBytes(32).toString('hex')]));
let local, server, browser, page, base, localToken, stderr = '';
async function start(args, hosted = false) {
  const process = spawn(exe, args, { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] }); let text = '';
  process.stdout.on('data', bytes => text += bytes); process.stderr.on('data', bytes => stderr += bytes);
  for (let i = 0; i < 300; i++) {
    const match = text.match(hosted ? /(http:\/\/127\.0\.0\.1:\d+)\/\r?\n/ : /(http:\/\/127\.0\.0\.1:\d+)\/#token=(\w+)/);
    if (match) { base = match[1]; localToken = match[2]; return process; }
    if (process.exitCode !== null) throw new Error(stderr); await pause(100);
  }
  process.kill(); throw new Error('startup timeout');
}
async function stop(process) { if (process && process.exitCode === null && process.signalCode === null) { const done = new Promise(resolve => process.once('exit', resolve)); process.kill(); await done; } }
async function api(route, data, actor) {
  const response = await fetch(base + route, { method: data === undefined ? 'GET' : 'POST', headers: { 'Content-Type': 'application/json', ...(actor ? { Authorization: 'Bearer ' + token[actor] } : { 'X-Weftext-Token': localToken }) }, body: data === undefined ? undefined : JSON.stringify(data) });
  const body = await response.json(); assert.ok(response.ok, JSON.stringify(body)); return body;
}
async function click(name) { await page.getByRole('button', { name, exact: true }).click(); }
async function idle(name) { await page.waitForFunction(label => [...document.querySelectorAll('#office button')].some(button => button.textContent === label && !button.disabled), name); }
async function cache() { return page.evaluate(() => JSON.parse(sessionStorage[Object.keys(sessionStorage).find(key => key.startsWith('weftext-hosted-office:'))])); }
async function reload() { await page.reload(); await page.getByLabel('Office 输出格式', { exact: true }).waitFor(); await page.waitForFunction(() => document.querySelector('#office [aria-label="Word 导入目标"] option[value="imported"]')); }
async function changePolicy(edit) { const current = await api('/api/hosted/admin/policy', undefined, 'admin'); edit(current.users); return api('/api/hosted/admin/policy', { expectedGeneration: current.generation, users: current.users }, 'admin'); }
async function prepareOutput() {
  const response = page.waitForResponse(response => response.url().endsWith('/api/hosted/office/export/prepare'));
  await click('准备 Office 输出'); const received = await response; assert.equal(received.status(), 200); const receipt = await received.json();
  await idle('确认 Office 输出'); return receipt;
}
async function confirmOutput() {
  await page.getByLabel('接受本次输出的损失', { exact: true }).check();
  const response = page.waitForResponse(response => response.url().endsWith('/api/hosted/office/export/confirm'));
  await click('确认 Office 输出'); assert.equal((await response).status(), 200); await idle('下载已确认 Office 文件');
}
async function downloadOutput(receipt, name) {
  const event = page.waitForEvent('download'); await click('下载已确认 Office 文件'); const download = await event;
  assert.equal(download.suggestedFilename(), receipt.filename); assert.equal(await download.failure(), null);
  const target = path.join(out, name); await download.saveAs(target); const bytes = await readFile(target);
  assert.equal(bytes.length, receipt.length); assert.equal(sha(bytes), receipt.digest); assert.equal(bytes.subarray(0, 2).toString(), 'PK'); await idle('明确准备一份新 Office 输出'); return bytes;
}
function parts(bytes) {
  return JSON.parse(execFileSync(process.env.WEFTEXT_PYTHON || 'python', ['-c', 'import sys,io,json,zipfile\nz=zipfile.ZipFile(io.BytesIO(sys.stdin.buffer.read()))\nprint(json.dumps({n:z.read(n).decode("utf-8") for n in z.namelist() if n.endswith(".xml")}))'], { input: bytes, windowsHide: true, maxBuffer: 40 * 1024 * 1024 }).toString());
}
function templateFrom(bytes) {
  return execFileSync(process.env.WEFTEXT_PYTHON || 'python', ['-c', `import sys,io,zipfile
source=zipfile.ZipFile(io.BytesIO(sys.stdin.buffer.read())); output=io.BytesIO()
with zipfile.ZipFile(output,'w',zipfile.ZIP_DEFLATED) as target:
 for entry in source.infolist():
  data=source.read(entry.filename)
  if entry.filename=='word/document.xml':
   xml=data.decode('utf-8'); start=xml.index('<w:body>')+len('<w:body>'); end=xml.index('<w:sectPr>')
   controls=''.join('<w:sdt><w:sdtPr><w:tag w:val="'+name+'"/></w:sdtPr><w:sdtContent><w:p><w:r><w:t>OLD_'+name+'</w:t></w:r></w:p></w:sdtContent></w:sdt>' for name in ['bodySlot','noteSlot'])
   data=(xml[:start]+controls+xml[end:]).encode('utf-8')
  target.writestr(entry.filename,data)
sys.stdout.buffer.write(output.getvalue())`], { input: bytes, windowsHide: true, maxBuffer: 40 * 1024 * 1024 });
}
try {
  const data = path.join(out, 'state'), root = path.join(out, 'library'); local = await start(['--no-browser', '--data-dir', data]);
  await api('/api/library/create', { path: root, operationId: randomUUID() });
  const schema = await api('/api/data/schema', { root }), operationId = randomUUID();
  const installed = JSON.parse(schema.source); installed.fields.find(field => field.id === 'tasks/priority').shape = { kind: 'scalar', semantic: 'int' };
  const plan = await api('/api/data/schema/prepare', { root, source: JSON.stringify(installed, null, 2), beforeSource: schema.source, beforeRevision: schema.revision, operationId });
  await api('/api/data/schema/install', { operationId, planId: plan.planId });
  const note = await api('/api/note/create', { root, parent: root, name: 'Shared', operationId: randomUUID() });
  const initial = note.source.replace('types: ()', 'types: ("task",), status: "open"') + '\n共同正文中文😀\n';
  await api('/api/save', { sessionId: note.sessionId, baseId: note.baseId, source: initial, operationId: randomUUID() }); await stop(local); local = null;
  const config = { deployment: 'office-browser', bind: '127.0.0.1:0', root, adminTokenSha256: sha(token.admin), files: {
    shared: { path: 'Shared/Shared.typ', title: '共同文档', editable: true },
    imported: { path: 'Imported/document/document.typ', title: 'Word 导入槽位', editable: true, importable: true },
  }, users: {
    alice: { enabled: true, tokenSha256: sha(token.alice), grants: { shared: { read: true, write: true, fields: { 'tasks/status': { read: true, write: true } }, annotations: { read: true, target: true }, office: { export: true, download: true } }, imported: { read: true, write: true, office: { import: true } } } },
    bob: { enabled: true, tokenSha256: sha(token.bob), grants: { shared: { fields: { 'tasks/status': { read: true } }, office: { export: true, download: true } } } },
    writer: { enabled: true, tokenSha256: sha(token.writer), grants: { shared: { read: true, write: true } } },
  } };
  const configPath = path.join(out, 'server.json'); await writeFile(configPath, JSON.stringify(config, null, 2)); server = await start(['--no-browser', '--data-dir', data, '--server-config', configPath], true);
  browser = await chromium.launch({ channel: 'chrome', headless: true }); result.browser = browser.version();
  const context = await browser.newContext({ viewport: { width: 1440, height: 1100 }, acceptDownloads: true }); page = await context.newPage();
  page.on('pageerror', error => result.pageErrors.push(String(error))); page.on('console', message => { if (message.type() === 'error') result.consoleErrors.push(message.text()); }); page.on('dialog', dialog => dialog.accept());
  let outputCalls = 0, importCalls = 0; page.on('request', request => { if (request.url().endsWith('/api/hosted/office/export/prepare')) outputCalls++; if (request.url().endsWith('/api/hosted/office/import/prepare')) importCalls++; });
  await page.goto(base); await page.getByLabel('访问凭据', { exact: true }).fill(token.alice); await click('连接'); await page.getByLabel('Office 输出格式', { exact: true }).waitFor(); await page.getByLabel('Word 导入目标', { exact: true }).waitFor();
  await click('共同文档'); await page.locator('#source .cm-content[contenteditable=true]').waitFor(); await click('V01 源码');
  const source = page.locator('#source .cm-content[contenteditable=true]'); await source.click(); await source.press('ControlOrMeta+End'); await source.press('Enter'); await source.pressSequentially('UNSAVEDWORDBODY987 中文😀草稿');
  await page.locator('#source [role=status]').filter({ hasText: '预览对应当前草稿' }).waitFor({ timeout: 30000 });
  await page.getByLabel('Word 审阅输出', { exact: true }).selectOption('comments'); await click('准备 Office 输出');
  await page.locator('#office').getByText(/带批注或修订的输出须使用已保存正文/).waitFor(); assert.equal(outputCalls, 0); assert.equal(await readFile(path.join(root, 'Shared/Shared.typ'), 'utf8'), initial);
  await page.getByLabel('Word 审阅输出', { exact: true }).selectOption('none'); let originalOutput, originalRequest;
  await page.route('**/api/hosted/office/export/prepare', async route => { originalRequest = route.request().postDataJSON(); const response = await route.fetch(); assert.equal(response.status(), 200); originalOutput = await response.json(); await route.abort('failed'); });
  await click('准备 Office 输出'); await idle('核对原 Office 输出'); await page.unroute('**/api/hosted/office/export/prepare');
  assert.equal(outputCalls, 1); assert.match(originalRequest.source, /UNSAVEDWORDBODY987/); assert.equal((await cache()).output.request.requestId, originalRequest.requestId);
  await reload(); assert.equal(await page.locator('#source .cm-content').count(), 0);
  await click('核对原 Office 输出'); await idle('确认 Office 输出'); assert.equal((await cache()).output.receipt.requestId, originalRequest.requestId); assert.equal(outputCalls, 1);
  result.cases.push('Office entry blocks dirty review export without autosave; lost plain DOCX prepare and reload recover the original frozen draft without opening source');
  if (originalOutput.losses.length) { await click('确认 Office 输出'); await page.locator('#office').getByText('请先阅读并明确接受本次输出的损失。', { exact: true }).waitFor(); assert.equal((await cache()).output.receipt.status, 'prepared'); }
  await page.getByLabel('接受本次输出的损失', { exact: true }).check(); await reload(); assert.equal(await page.getByLabel('接受本次输出的损失', { exact: true }).isChecked(), true);
  await confirmOutput(); const originalDocx = await downloadOutput(originalOutput, 'frozen.docx');
  assert.match(parts(originalDocx)['word/document.xml'], /UNSAVEDWORDBODY987/); assert.equal(await readFile(path.join(root, 'Shared/Shared.typ'), 'utf8'), initial);
  result.cases.push('explicit loss consent survives redraw/reload; actual DOCX download matches the original digest and contains unsaved captured text without saving source');
  await click('明确准备一份新 Office 输出');
  await page.getByLabel('查询字段', { exact: true }).selectOption('tasks/status'); await page.getByLabel('CEL 筛选条件', { exact: true }).fill('status == "open"');
  const queryResponse = page.waitForResponse(response => response.url().endsWith('/api/hosted/query')); await click('运行查询'); const query = await (await queryResponse).json();
  await page.getByRole('table', { name: '获权字段查询结果' }).waitFor(); assert.equal(await page.getByLabel('Office 输出格式', { exact: true }).inputValue(), 'xlsx');
  const current = await api('/api/hosted/open', { resource: 'shared' }, 'writer'); await api('/api/hosted/save', { sessionId: current.sessionId, baseId: current.baseId, source: current.source.replace('status: "open"', 'status: "done"'), operationId: randomUUID() }, 'writer');
  await page.getByLabel('Office 输出格式', { exact: true }).selectOption('docx'); await page.getByLabel('Word 审阅输出', { exact: true }).selectOption('changes'); await page.getByLabel('包含已解决批注', { exact: true }).check(); await page.getByLabel('Office 输出格式', { exact: true }).selectOption('xlsx');
  const xlsx = await prepareOutput(), xlsxRequest = (await cache()).output.request; assert.equal(xlsxRequest.queryId, query.queryId); assert.equal(xlsxRequest.reviewMode, 'none'); assert.equal(xlsxRequest.includeResolved, false); assert.ok(!('source' in xlsxRequest)); assert.ok(!('resource' in xlsxRequest));
  await page.locator('#office details').first().evaluate(element => element.open = true); assert.match(await page.getByRole('table', { name: '冻结查询输出预览' }).innerText(), /open/);
  await confirmOutput(); let invalidDownloads = 0; const invalidDelivered = () => invalidDownloads++; page.on('download', invalidDelivered);
  await page.route('**/api/hosted/office/export/chunk', async route => { const response = await route.fetch(); const chunk = await response.json(); await route.fulfill({ response, json: { ...chunk, sha256: '0'.repeat(64) } }); });
  await click('下载已确认 Office 文件'); await page.locator('#office').getByText(/下载块与已确认工件不一致/).waitFor(); assert.equal(invalidDownloads, 0); page.off('download', invalidDelivered); await page.unroute('**/api/hosted/office/export/chunk');
  await click('核对原 Office 输出'); await idle('下载已确认 Office 文件'); const workbook = await downloadOutput(xlsx, 'frozen.xlsx'); assert.match(parts(workbook)['xl/worksheets/weftext1.xml'], /open/);
  result.cases.push('typed Query exports the displayed frozen open value after source changes; format switch drops Word-only options; inconsistent chunks are blocked before verified XLSX delivery');
  await click('明确准备一份新 Office 输出'); await page.getByLabel('Office 输出格式', { exact: true }).selectOption('docx'); await page.getByLabel('Word 审阅输出', { exact: true }).selectOption('none'); await click('共同文档');
  await page.locator('#source .cm-content[contenteditable=true]').waitFor(); await page.locator('#source [role=status]').filter({ hasText: '预览对应当前草稿' }).waitFor({ timeout: 30000 });
  const template = templateFrom(originalDocx); await page.getByLabel('Office 输出模板', { exact: true }).setInputFiles({ name: 'owned-template.docx', mimeType: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', buffer: template });
  await page.getByLabel('模板槽位 bodySlot', { exact: true }).waitFor(); await page.getByLabel('模板槽位 bodySlot', { exact: true }).selectOption('body'); await page.getByLabel('模板槽位 noteSlot', { exact: true }).selectOption('text'); await page.getByLabel('模板固定文字 noteSlot', { exact: true }).fill('模板字面文本😀<&');
  const templated = await prepareOutput(); assert.equal((await cache()).output.request.template.bytesBase64, template.toString('base64')); assert.ok(!('queryId' in (await cache()).output.request)); await confirmOutput(); const templatedBytes = await downloadOutput(templated, 'template.docx');
  const xml = parts(templatedBytes)['word/document.xml']; assert.match(xml, /模板字面文本😀/); assert.ok(!xml.includes('OLD_bodySlot') && !xml.includes('OLD_noteSlot'));
  await page.screenshot({ path: path.join(out, 'office-output.png'), fullPage: true }); result.cases.push('actual template inspection and explicit body/text slot binding preserve exact upload and literal non-BMP text; an unbound old query is not silently appended');
  await click('明确准备一份新 Office 输出'); await page.getByLabel('Word 导入目标', { exact: true }).selectOption('imported'); await page.getByLabel('Word 导入原件', { exact: true }).setInputFiles({ name: 'frozen-original.docx', mimeType: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', buffer: originalDocx });
  await idle('准备 Word 导入'); const beforeImport = importCalls;
  await page.evaluate(() => { window.officeOriginalSetItem = Storage.prototype.setItem; Storage.prototype.setItem = function(key, value) { if (key.startsWith('weftext-hosted-office:')) throw new DOMException('injected cache quota failure', 'QuotaExceededError'); return window.officeOriginalSetItem.call(this, key, value); }; });
  await click('准备 Word 导入'); await page.locator('#office').getByText(/浏览器恢复缓存容量不足或不可写/).waitFor(); assert.equal(importCalls, beforeImport); assert.match(await page.locator('#office').innerText(), /frozen-original.docx/); await assert.rejects(access(path.join(root, 'Imported')));
  await page.evaluate(() => { Storage.prototype.setItem = window.officeOriginalSetItem; delete window.officeOriginalSetItem; });
  await click('核对原 Word 导入操作'); await idle('明确准备另一份 Word 导入'); assert.equal((await cache()).importJob.receipt.status, 'cancelled'); await click('明确准备另一份 Word 导入'); assert.equal((await cache()).importDraft.file.bytesBase64, originalDocx.toString('base64'));
  result.cases.push('injected session-storage failure prevents all import prepare writes, keeps the original file, and safely settles the unsent original ID before an explicit new proposal');
  const importResponse = page.waitForResponse(response => response.url().endsWith('/api/hosted/office/import/prepare')); await click('准备 Word 导入'); const preparedImport = await importResponse; assert.equal(preparedImport.status(), 200); const imported = await preparedImport.json(); await idle('确认提交 Word 导入'); await assert.rejects(access(path.join(root, 'Imported')));
  assert.equal((await cache()).importJob.request.bytesBase64, originalDocx.toString('base64')); await page.getByLabel('接受本次 Word 导入的损失', { exact: true }).check();
  let commitRequest; await page.route('**/api/hosted/office/import/commit', async route => { commitRequest = route.request().postDataJSON(); const response = await route.fetch(); assert.equal(response.status(), 200); assert.equal((await response.json()).status, 'committed'); await route.abort('failed'); });
  await click('确认提交 Word 导入'); await idle('核对原 Word 导入操作'); await page.unroute('**/api/hosted/office/import/commit'); assert.equal(commitRequest.operationId, imported.operationId); const countAtCommit = importCalls;
  await reload(); assert.equal(await page.locator('#source .cm-content').count(), 0); await click('核对原 Word 导入操作'); await page.locator('#office').getByText(/原 Word 导入已提交，原件与回执保留/).waitFor();
  assert.equal((await cache()).importJob.receipt.status, 'committed'); assert.equal((await cache()).importJob.request.operationId, imported.operationId); assert.equal(importCalls, countAtCommit); assert.equal(await page.getByRole('button', { name: '确认提交 Word 导入', exact: true }).isDisabled(), true);
  assert.deepEqual(await readFile(path.join(root, 'Imported/document/original.docx')), originalDocx); assert.match(await readFile(path.join(root, 'Imported/document/document.typ'), 'utf8'), /UNSAVEDWORDBODY987/); assert.equal((await readdir(root)).filter(name => name === 'Imported').length, 1);
  await click('共同文档'); await page.locator('#source .cm-content[contenteditable=true]').waitFor(); assert.match(await page.locator('#source .cm-content').innerText(), /UNSAVEDWORDBODY987/);
  result.cases.push('lost import commit and reload query the original committed operation once, preserve the DOCX bytes and native tree, and do not replace the separate personal source draft');
  // A request aborted before reaching the server is genuinely notRegistered;
  // this is different from injecting a terminal receipt or simulating expiry.
  let unsent; await page.route('**/api/hosted/office/export/prepare', route => { unsent = route.request().postDataJSON(); return route.abort('failed'); });
  await click('准备 Office 输出'); await idle('核对原 Office 输出'); await page.unroute('**/api/hosted/office/export/prepare'); await reload(); await click('核对原 Office 输出'); await idle('明确准备一份新 Office 输出');
  const unregistered = (await cache()).output; assert.equal(unregistered.request.requestId, unsent.requestId); assert.deepEqual(unregistered.receipt, { requestId: unsent.requestId, status: 'notRegistered' }); await click('明确准备一份新 Office 输出');
  result.cases.push('a never-delivered output request is recovered as the original notRegistered terminal ID without stale artifact preview or automatic regeneration');
  await click('共同文档'); await page.locator('#source .cm-content[contenteditable=true]').waitFor(); await page.locator('#source [role=status]').filter({ hasText: '预览对应当前草稿' }).waitFor({ timeout: 30000 }); const finalOutput = await prepareOutput(); assert.notEqual(finalOutput.requestId, unsent.requestId); await confirmOutput();
  let release, observed; const held = new Promise(resolve => observed = resolve), resume = new Promise(resolve => release = resolve); let lateDownloads = 0; page.on('download', () => lateDownloads++);
  await page.route('**/api/hosted/office/export/chunk', async route => { const response = await route.fetch(); assert.equal(response.status(), 200); observed(); await resume; await route.fulfill({ response }); });
  await click('下载已确认 Office 文件'); await held; await changePolicy(users => users.alice.grants = {}); await page.locator('#status').filter({ hasText: '缓存已清除' }).waitFor({ timeout: 12000 }); release(); await pause(500);
  assert.equal(lateDownloads, 0); assert.equal(await page.locator('#office').isHidden(), true); assert.deepEqual(await page.evaluate(() => Object.keys(sessionStorage).filter(key => key.startsWith('weftext-hosted-office:'))), []);
  assert.equal((await api('/api/hosted/fields/read', { resource: 'shared', fieldId: 'tasks/status' }, 'bob')).field.cell.value.value, 'done');
  result.cases.push('revocation clears output/import recovery data and rejects held late artifact delivery while the other actor remains authorized');
  assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) { result.failure = String(error.stack ?? error); if (page) await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {}); throw error; }
finally { await browser?.close(); await stop(local); await stop(server); await writeFile(path.join(out, 'results.json'), JSON.stringify(result, null, 2)); await writeFile(path.join(out, 'server.stderr.log'), stderr); console.log(JSON.stringify({ out, cases: result.cases, completed: result.completed, pageErrors: result.pageErrors })); }
