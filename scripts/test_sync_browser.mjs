import { spawnServe, serveReady, stopServe, stringifyServeJson } from './_cli_serve.mjs';
import { createdDocument } from './_creation_result.mjs';
// Headless, owned fixtures, two independent actual programs. No desktop input.
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import assert from 'node:assert/strict';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]);
await mkdir(out, { recursive: false });
const result = { exe, sha256: createHash('sha256').update(await readFile(exe)).digest('hex'), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(), cases: [], pageErrors: [], completed: false };
const servers = []; let browser, pageA, pageB, stderr = '';
const pause = ms => new Promise(r => setTimeout(r, ms));
async function start(args) {
  const child = spawnServe(exe, args, { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] }); servers.push(child);
  let output = ''; child.stdout.on('data', b => output += b); child.stderr.on('data', b => stderr += b);
  for (let i = 0; i < 300; i++) {
    const m = serveReady(child);
    if (m) return { child, base: m.base, token: m.token };
    if (child.exitCode !== null) throw new Error(stderr); await pause(100);
  }
  throw new Error('startup timeout');
}
async function stop(child) { await stopServe(child); }
async function api(server, route, data) {
  const response = await fetch(server.base + route, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': server.token }, body: JSON.stringify(data) });
  const body = await response.json(); assert.ok(response.ok, JSON.stringify(body)); return body;
}
async function sync(page) { await page.getByRole('button', { name: '同步', exact: true }).click(); const panel = page.getByRole('dialog', { name: '资料库同步' }); await panel.waitFor(); await panel.getByRole('button', { name: '读取本机状态', exact: true }).waitFor(); return panel; }
async function click(panel, name) {
  const button = panel.getByRole('button', { name, exact: true });
  await button.click(); await panel.getByRole('status').filter({ hasText: '操作已完成' }).waitFor();
}
async function close(panel) { await panel.getByRole('button', { name: '关闭同步', exact: true }).click(); }
try {
  const ra = path.join(out, 'library-a'), rb = path.join(out, 'library-b'), da = path.join(out, 'state-a'), db = path.join(out, 'state-b'), exchange = path.join(out, 'exchange');
  let a = await start(['--data-dir', da]);
  await api(a, '/api/library/create', { path: ra, operationId: randomUUID() });
  const note = createdDocument(await api(a, '/api/note/create', { root: ra, parent: ra, name: 'Note', operationId: randomUUID() }));
  await stop(a.child);
  a = await start(['--data-dir', da, '--open', note.path, '--root', ra]);
  const b = await start(['--data-dir', db]);
  browser = await chromium.launch({ channel: 'chrome', headless: true }); result.browser = browser.version();
  const ca = await browser.newContext({ viewport: { width: 1440, height: 1000 } }), cb = await browser.newContext({ viewport: { width: 1280, height: 960 } });
  pageA = await ca.newPage(); pageB = await cb.newPage();
  for (const p of [pageA, pageB]) p.on('pageerror', e => result.pageErrors.push(String(e)));
  await pageA.goto(`${a.base}/#token=${a.token}`); await pageB.goto(`${b.base}/#token=${b.token}`);
  let pa = await sync(pageA); await click(pa, '启用同步');
  const identity = await pa.getByLabel('配对身份（从原副本复制）', { exact: true }).inputValue();
  assert.ok(JSON.parse(identity).genesis);
  await pa.getByLabel('交换目录路径（新目录或已配对目录）', { exact: true }).fill(exchange);
  await click(pa, '导出所选对象'); await pa.screenshot({ path: path.join(out, 'origin.png') }); await close(pa);
  let pb = await sync(pageB);
  await pb.getByLabel('本机资料库路径', { exact: true }).fill(rb);
  await pb.getByLabel('配对身份（从原副本复制）', { exact: true }).fill(identity);
  await pb.getByLabel('交换目录路径（新目录或已配对目录）', { exact: true }).fill(exchange);
  await click(pb, '接收交换目录');
  await click(pb, '预览创建独立副本'); assert.match(await pb.innerText(), /cloneSync/);
  await assert.rejects(readFile(path.join(rb, 'Note', 'Note.typ')));
  await pb.screenshot({ path: path.join(out, 'clone-preview.png') });
  // Simulate a lost committed response in the actual browser, then reload.
  let accepted; const received = new Promise(r => accepted = r);
  await pageB.route('**/api/operations/commit', async route => { const response = await route.fetch(); assert.equal(response.status(), 200); accepted(); await route.abort('failed'); });
  await pb.getByRole('button', { name: '确认应用此预览', exact: true }).click(); await received;
  await pb.getByRole('status').filter({ hasText: '原操作记录保留' }).waitFor();
  await pageB.unroute('**/api/operations/commit'); await pageB.reload(); pb = await sync(pageB);
  await click(pb, '检查原操作结果'); assert.match(await pb.innerText(), /原同步操作：committed/);
  await click(pb, '结束已确认操作'); await close(pb);
  result.cases.push('UI enables/pairs/exports/receives and previews independent clone; lost commit response recovered after browser reload');

  // Two disconnected replicas commit independently; transport is only later exchanged.
  async function save(server, root, suffix) {
    const opened = await api(server, '/api/open', { root, path: path.join(root, 'Note', 'Note.typ') });
    const source = opened.source + '\n' + suffix;
    await api(server, '/api/save', { sessionId: opened.sessionId, baseId: opened.baseId, source, operationId: randomUUID() }); return source;
  }
  const sourceA = await save(a, ra, '甲中文分支😀'), sourceB = await save(b, rb, '乙离线分支');
  await pageA.getByRole('button', { name: '源码', exact: true }).click();
  const editor = pageA.locator('.cm-content[contenteditable=true]');
  const unsaved = sourceA + '\n浏览器未保存草稿'; await editor.fill(unsaved);
  // Open the cloned library through its normal UI so the sync entry has a root.
  await pageB.locator('.library-button').click();
  const open = pageB.getByRole('dialog'); await open.getByLabel('文件夹路径', { exact: true }).fill(rb); await open.getByRole('button', { name: '打开', exact: true }).click();
  pb = await sync(pageB); await click(pb, '刷新已保存版本');
  await pb.getByLabel('交换目录路径（新目录或已配对目录）', { exact: true }).fill(exchange); await click(pb, '导出所选对象'); await close(pb);
  pa = await sync(pageA); await pa.getByLabel('交换目录路径（新目录或已配对目录）', { exact: true }).fill(exchange);
  await click(pa, '接收交换目录'); await click(pa, '刷新已保存版本');
  await pa.locator('.sync-object').filter({ hasText: '冲突：保留全部分支' }).waitFor();
  const object = pa.locator('.sync-object').filter({ hasText: '冲突：保留全部分支' });
  const branchButtons = object.getByRole('button', { name: /^查看版本 / }); assert.equal(await branchButtons.count(), 2);
  const seen = [];
  for (let i = 0; i < 2; i++) {
    await branchButtons.nth(i).click(); await pa.locator('summary').filter({ hasText: /^Note.typ$/ }).click(); await click(pa, '读取 Note.typ');
    const source = await pa.locator('details').filter({ hasText: 'Note.typ' }).locator('pre').innerText(); seen.push(source);
  }
  assert.deepEqual(new Set(seen), new Set([sourceA, sourceB]));
  await pa.screenshot({ path: path.join(out, 'concurrent-branches.png') });
  await click(pa, '预览采用此完整分支'); await click(pa, '确认应用此预览');
  assert.equal((await readFile(note.path)).toString(), seen[1]);
  await pa.screenshot({ path: path.join(out, 'merged.png') });
  const originalId = await pa.locator('section').filter({ hasText: '原同步操作：committed' }).locator('code').innerText();
  await click(pa, '结束已确认操作'); await close(pa);
  assert.equal((await editor.locator('.cm-line').allTextContents()).join('\n'), unsaved);
  const retained = await api(a, '/api/open', { root: ra, path: note.path }); assert.equal(retained.recoveredDraft, unsaved);
  assert.match(await pageA.locator('body').innerText(), /磁盘内容已改变。当前草稿与撤销记录保留，请比较后继续保存。/);
  pa = await sync(pageA);
  await pa.getByLabel('原同步操作编号（地址变化或浏览器记录丢失时）', { exact: true }).fill(originalId);
  await click(pa, '读取原同步操作'); assert.match(await pa.innerText(), /原同步操作：committed/);
  await pa.screenshot({ path: path.join(out, 'original-lookup.png') });
  result.cases.push('UI shows both concurrent source versions and common bases, then explicitly previews and commits a complete branch selection');
  result.cases.push('applying preserves the actual editor draft and warns about disk changes; original-ID lookup recovers a completed proposal without browser pending state');
  assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (e) {
  result.failure = String(e.stack ?? e);
  await writeFile(path.join(out, 'results.json'), stringifyServeJson(result, 2));
  for (const [name, p] of [['a', pageA], ['b', pageB]]) if (p) await p.screenshot({ path: path.join(out, `failure-${name}.png`), fullPage: true }).catch(() => {});
  throw e;
} finally {
  await browser?.close(); for (const server of servers) await stop(server);
  await writeFile(path.join(out, 'results.json'), stringifyServeJson(result, 2)); await writeFile(path.join(out, 'server.stderr.log'), stderr);
  console.log(JSON.stringify({ out, completed: result.completed, cases: result.cases, pageErrors: result.pageErrors }));
}
