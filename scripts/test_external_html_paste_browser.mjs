// External browser HTML producer: native copy and trusted default paste, without
// manufactured ClipboardEvent/DataTransfer or replaying HTML into the clipboard.
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import path from 'node:path';

const dev = process.argv.includes('--dev'), hosted = process.argv.includes('--hosted');
assert.equal(process.argv.length, 4 + Number(dev) + Number(hosted), 'Usage: node scripts/test_external_html_paste_browser.mjs EXE NEW_OUTPUT [--dev] [--hosted]');
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]);
await mkdir(out, { recursive: false });
const { chromium } = createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const result = { exe, sha256: sha(await readFile(exe)), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(), frontend: dev ? 'current Vite frontend with stated backend' : 'embedded candidate', cases: [], pageErrors: [], completed: false, limits: ['A separate HTML page in real headless Chrome is the external producer; this does not establish Word/Excel clipboard producers, OS IME or full AT.'] };
const root = path.join(out, 'library'), data = path.join(out, 'state');
const hostedToken = randomUUID(), adminToken = randomUUID();
result.client = hosted ? 'controlled hosted writer' : 'local owner';
let child, browser, page, vite, base, token, stderr = '';
async function start(args = [], server = false) {
  child = spawn(exe, ['--no-browser', '--data-dir', data, ...args], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  let stdout = ''; child.stdout.on('data', bytes => stdout += bytes); child.stderr.on('data', bytes => stderr += bytes);
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    const match = stdout.match(server ? /(http:\/\/127\.0\.0\.1:\d+)\/\r?\n/ : /(http:\/\/127\.0\.0\.1:\d+)\/#token=(\w+)/);
    if (match) { base = match[1]; token = match[2]; return; }
    if (child.exitCode !== null) throw new Error(stderr);
    await new Promise(resolve => setTimeout(resolve, 80));
  }
  throw new Error('Owned server startup timeout');
}
async function stop() { if (child?.exitCode === null && child.signalCode === null) { const done = new Promise(resolve => child.once('exit', resolve)); child.kill(); await done; } }
async function api(route, body) { const reply = await fetch(base + route, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': token }, body: JSON.stringify(body) }), value = await reply.json(); assert.equal(reply.status, 200, route + ': ' + JSON.stringify(value)); return value; }
const click = label => page.getByRole('button', { name: label, exact: true }).click();
const mode = label => click(hosted ? label === '源码' ? 'V01 源码' : 'V04 可视写作' : label);
const sourceSelector = hosted ? '#source .cm-content' : '.source-host .cm-content';
const surface = () => page.locator('.writing-table-surface').first();
async function source() { return page.locator(sourceSelector).evaluate(element => { const view = element.cmTile?.root?.view; if (view?.contentDOM !== element) throw new Error('Actual source editor unavailable'); return view.state.doc.toString(); }); }
async function verifySource(expected) { await mode('源码'); await page.waitForFunction(({ expected, selector }) => { const element = document.querySelector(selector), view = element?.cmTile?.root?.view; return view?.contentDOM === element && view.state.doc.toString() === expected; }, { expected, selector: sourceSelector }); }
async function target() {
  await mode('可视编辑'); await surface().locator('.writing-table-tools[aria-busy="false"]').waitFor();
  await surface().locator('[data-cell-row="1"][data-cell-column="0"] .table-cell-display').click();
  await surface().getByLabel('第 2 行第 1 列文字', { exact: true }).press('Escape');
  await surface().getByRole('grid').focus();
}
try {
  await start(); await api('/api/library/create', { path: root, operationId: randomUUID() });
  const doc = await api('/api/note/create', { root, parent: root, name: 'External Paste', operationId: randomUUID() });
  const initial = doc.source + '\n表前保留。\n\n#table(columns: 3, [头甲], [头乙], [头丙], [旧甲], [旧乙], [右边保留], [旧丙], [旧丁], [末格保留])\n\n表后保留。\n';
  await api('/api/save', { sessionId: doc.sessionId, baseId: doc.baseId, source: initial, operationId: randomUUID() }); await stop();
  if (hosted) {
    const config = { deployment: 'external-paste', bind: '127.0.0.1:0', root, adminTokenSha256: sha(adminToken), files: { table: { path: path.relative(root, doc.path).replaceAll('\\', '/'), title: '外部粘贴文档', editable: true } }, users: { writer: { enabled: true, tokenSha256: sha(hostedToken), grants: { table: { read: true, write: true } } } } };
    const configPath = path.join(out, 'server.json'); await writeFile(configPath, JSON.stringify(config)); await start(['--server-config', configPath], true);
  } else await start(['--open', doc.path, '--root', root]);
  let frontend = base;
  if (dev) {
    const app = path.resolve(import.meta.dirname, '../app'), { createServer } = await import(pathToFileURL(path.join(app, 'node_modules/vite/dist/node/index.js')).href);
    const proxy = { target: base, changeOrigin: true, configure(instance) { instance.on('proxyReq', (request, incoming) => { if (incoming.headers.origin) request.setHeader('Origin', base); }); } };
    vite = await createServer({ root: app, configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false, watch: { ignored: ['**/*'] }, proxy: { '/api': proxy, '/fonts': proxy } } });
    await vite.listen(); frontend = 'http://127.0.0.1:' + vite.httpServer.address().port;
  }
  browser = await chromium.launch({ channel: 'chrome', headless: true }); result.browser = browser.version();
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, permissions: ['clipboard-read', 'clipboard-write'] });
  page = await context.newPage(); page.setDefaultTimeout(30000); page.on('pageerror', error => result.pageErrors.push(String(error)));
  await page.goto(frontend + (hosted ? '/hosted.html' : '/#token=' + token));
  if (hosted) { await page.getByLabel('访问凭据', { exact: true }).fill(hostedToken); await click('连接'); await page.getByRole('button', { name: '外部粘贴文档', exact: true }).waitFor(); await page.waitForLoadState('networkidle'); await click('外部粘贴文档'); await page.locator('#source .cm-content[contenteditable=true]').waitFor(); }
  await verifySource(initial);
  await page.evaluate(() => { window.__externalPasteEvents = []; document.addEventListener('paste', event => window.__externalPasteEvents.push({ trusted: event.isTrusted, types: [...event.clipboardData.types], html: event.clipboardData.getData('text/html'), text: event.clipboardData.getData('text/plain') }), true); });
  const producer = await context.newPage();
  await producer.goto('data:text/html;charset=utf-8,' + encodeURIComponent('<!doctype html><meta charset="utf-8"><title>Owned external HTML table</title><table id="external"><tr><td colspan="2">外部甲<br>外部乙</td></tr><tr><td>重复</td><td></td></tr></table>'));
  await producer.evaluate(() => { const range = document.createRange(); range.selectNode(document.querySelector('table')); const selection = getSelection(); selection.removeAllRanges(); selection.addRange(range); window.__copyEvents = []; document.addEventListener('copy', event => window.__copyEvents.push({ trusted: event.isTrusted }), true); });
  await producer.keyboard.press('Control+c');
  result.producer = { url: producer.url(), copyEvents: await producer.evaluate(() => window.__copyEvents) };
  assert.equal(result.producer.copyEvents.length, 1); assert.equal(result.producer.copyEvents[0].trusted, true);
  await page.bringToFront(); await target(); await surface().getByRole('grid').press('Control+v');
  let dialog = page.getByRole('dialog', { name: '粘贴表格范围', exact: true }); await dialog.waitFor();
  await dialog.getByRole('button', { name: '取消', exact: true }).click(); await verifySource(initial); assert.equal(await readFile(doc.path, 'utf8'), initial);
  result.cases.push('Native external HTML copy and trusted paste preview can be canceled without changing Draft or disk.');
  await target(); await surface().getByRole('grid').press('Control+v'); dialog = page.getByRole('dialog', { name: '粘贴表格范围', exact: true }); await dialog.waitFor();
  await dialog.getByRole('button', { name: '粘贴', exact: true }).click();
  await surface().locator('[data-cell-row="1"][data-cell-column="0"][colspan="2"]').waitFor();
  result.pastedCells = await surface().locator('[data-cell-row][data-cell-column]').evaluateAll(cells => cells.map(cell => ({ row: Number(cell.dataset.cellRow), column: Number(cell.dataset.cellColumn), rowspan: cell.rowSpan, colspan: cell.colSpan, text: cell.querySelector('textarea')?.value })));
  assert.deepEqual(result.pastedCells, [
    { row: 0, column: 0, rowspan: 1, colspan: 1, text: '头甲' }, { row: 0, column: 1, rowspan: 1, colspan: 1, text: '头乙' }, { row: 0, column: 2, rowspan: 1, colspan: 1, text: '头丙' },
    { row: 1, column: 0, rowspan: 1, colspan: 2, text: '外部甲\n外部乙' }, { row: 1, column: 2, rowspan: 1, colspan: 1, text: '右边保留' },
    { row: 2, column: 0, rowspan: 1, colspan: 1, text: '重复' }, { row: 2, column: 1, rowspan: 1, colspan: 1, text: '' }, { row: 2, column: 2, rowspan: 1, colspan: 1, text: '末格保留' },
  ]);
  await mode('源码'); const pasted = await source();
  for (const text of ['外部甲', '外部乙', '重复', '右边保留', '末格保留', '表前保留', '表后保留']) assert.ok(pasted.includes(text), text);
  for (const text of ['旧甲', '旧乙', '旧丙', '旧丁']) assert.ok(!pasted.includes(text), text);
  assert.equal(await readFile(doc.path, 'utf8'), initial);
  await page.locator(sourceSelector).press('Control+z'); await verifySource(initial);
  await page.locator(sourceSelector).press('Control+Shift+Z'); await verifySource(pasted);
  result.cases.push('One native paste preserves merged cells, line breaks, empty values and untouched cells; one Undo/Redo restores the exact Draft.');
  result.pasteEvents = await page.evaluate(() => window.__externalPasteEvents);
  assert.equal(result.pasteEvents.length, 2);
  for (const event of result.pasteEvents) { assert.equal(event.trusted, true); assert.ok(event.types.includes('text/html')); assert.match(event.html, /colspan="2"/); assert.match(event.html, /外部甲/); }
  assert.equal(await readFile(doc.path, 'utf8'), initial); assert.deepEqual(result.pageErrors, []);
  result.completed = true;
} catch (error) { result.failure = String(error.stack ?? error); if (page) { await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {}); await writeFile(path.join(out, 'failure-ui.txt'), await page.locator('body').innerText().catch(() => 'unavailable')); } throw error;
} finally { await browser?.close(); await vite?.close(); await stop(); result.ownedProcessesClosed = (!child || child.exitCode !== null || child.signalCode !== null) && (!browser || !browser.isConnected()); await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2)); await writeFile(path.join(out, 'server.stderr.log'), stderr); }
console.log(JSON.stringify({ out, completed: result.completed, cases: result.cases.length, ownedProcessesClosed: result.ownedProcessesClosed }));
