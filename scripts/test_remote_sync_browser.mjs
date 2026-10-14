// Three actual programs and headless browser controls. No desktop interaction.
import { createRequire } from 'node:module';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { createHash, randomBytes, randomUUID } from 'node:crypto';
import assert from 'node:assert/strict';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]);
await mkdir(out, { recursive: false });
const digest = b => createHash('sha256').update(b).digest('hex');
const result = { exe, sha256: digest(await readFile(exe)), cases: [], pageErrors: [], completed: false };
const children = []; let browser, aPage, bPage, stderr = '';
const sleep = ms => new Promise(r => setTimeout(r, ms));
async function start(args, hosted = false) {
  const child = spawn(exe, args, { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] }); children.push(child);
  let output = ''; child.stdout.on('data', b => output += b); child.stderr.on('data', b => stderr += b);
  for (let i = 0; i < 300; i++) {
    const match = output.match(hosted ? /(http:\/\/127\.0\.0\.1:\d+)\/\r?\n/ : /(http:\/\/127\.0\.0\.1:\d+)\/#token=(\w+)/);
    if (match) return { child, base: match[1], token: match[2] };
    if (child.exitCode !== null) throw new Error(stderr); await sleep(100);
  }
  throw new Error('startup timeout');
}
async function stop(child) { if (child.exitCode === null && child.signalCode === null) { const done = new Promise(r => child.once('exit', r)); child.kill(); await done; } }
async function api(server, route, data, token) {
  const response = await fetch(server.base + route, { method: data === undefined ? 'GET' : 'POST', headers: { 'Content-Type': 'application/json', ...(token ? { Authorization: 'Bearer ' + token } : { 'X-Weftext-Token': server.token }) }, body: data === undefined ? undefined : JSON.stringify(data) });
  const body = await response.json(); assert.ok(response.ok, JSON.stringify(body)); return body;
}
async function sync(page) { await page.getByRole('button', { name: '同步', exact: true }).click(); const panel = page.getByRole('dialog', { name: '资料库同步' }); await panel.waitFor(); return panel; }
async function click(panel, name) { await panel.getByRole('button', { name, exact: true }).click(); await panel.getByRole('status').filter({ hasText: '操作已完成' }).waitFor(); }
async function transfer(panel, name) { await panel.getByRole('button', { name, exact: true }).click(); await panel.getByRole('status', { name: '远程传输进度' }).filter({ hasText: '传输完成' }).waitFor({ timeout: 60000 }); }
try {
  const ra = path.join(out, 'a'), rb = path.join(out, 'b'), relayRoot = path.join(out, 'relay-files'); await mkdir(relayRoot);
  const da = path.join(out, 'state-a'), db = path.join(out, 'state-b');
  let a = await start(['--no-browser', '--data-dir', da]);
  await api(a, '/api/library/create', { path: ra, operationId: randomUUID() });
  const note = await api(a, '/api/note/create', { root: ra, parent: ra, name: 'Note', operationId: randomUUID() });
  await api(a, '/api/note/create', { root: ra, parent: ra, name: 'Other', operationId: randomUUID() });
  await stop(a.child); a = await start(['--no-browser', '--data-dir', da, '--open', note.path, '--root', ra]);
  const b = await start(['--no-browser', '--data-dir', db]);
  browser = await chromium.launch({ channel: 'chrome', headless: true }); result.browser = browser.version();
  aPage = await browser.newPage({ viewport: { width: 1440, height: 1000 } }); bPage = await browser.newPage({ viewport: { width: 1280, height: 960 } });
  for (const page of [aPage, bPage]) page.on('pageerror', error => result.pageErrors.push(String(error)));
  await aPage.goto(`${a.base}/#token=${a.token}`); await bPage.goto(`${b.base}/#token=${b.token}`);
  let pa = await sync(aPage); await click(pa, '启用同步');
  const identity = JSON.parse(await pa.getByLabel('配对身份（从原副本复制）', { exact: true }).inputValue());
  const ownerToken = randomBytes(32).toString('hex'), adminToken = randomBytes(32).toString('hex');
  const config = { deployment: 'browser-sync', bind: '127.0.0.1:0', root: relayRoot, adminTokenSha256: digest(adminToken), syncIdentity: identity, files: {}, users: { owner: { enabled: true, tokenSha256: digest(ownerToken), syncOwner: true, grants: {} } } };
  const configPath = path.join(out, 'server.json'); await writeFile(configPath, JSON.stringify(config, null, 2));
  const relay = await start(['--no-browser', '--data-dir', path.join(out, 'relay-state'), '--server-config', configPath], true);
  await pa.getByLabel('远端服务地址', { exact: true }).fill(relay.base); await pa.getByLabel('完整同步凭据', { exact: true }).fill(ownerToken);
  let lostStart = false;
  await aPage.route('**/api/sync/remote/start', async route => {
    await route.fetch(); lostStart = true; await route.abort('failed');
  }, { times: 1 });
  await transfer(pa, '上传所选本机对象');
  assert.equal(lostStart, true);
  const pb = await sync(bPage);
  await pb.getByLabel('本机资料库路径', { exact: true }).fill(rb); await pb.getByLabel('配对身份（从原副本复制）', { exact: true }).fill(JSON.stringify(identity));
  await pb.getByLabel('远端服务地址', { exact: true }).fill(relay.base); await pb.getByLabel('完整同步凭据', { exact: true }).fill(ownerToken);
  await pb.getByRole('button', { name: '读取远端目录', exact: true }).click(); await pb.getByLabel(/^远端：Other /).waitFor(); await pb.getByLabel(/^远端：Other /).uncheck();
  await transfer(pb, '接收所选远端对象');
  await assert.rejects(readFile(path.join(rb, 'Note', 'Note.typ')));
  await pb.locator('.sync-object').filter({ hasText: 'Note (' }).waitFor();
  assert.equal(await pb.locator('.sync-object').filter({ hasText: 'Other (' }).count(), 0);
  await click(pb, '预览创建独立副本'); await pb.screenshot({ path: path.join(out, 'remote-clone-preview.png') }); await click(pb, '确认应用此预览'); await click(pb, '结束已确认操作');
  assert.equal((await readFile(path.join(rb, 'Note', 'Note.typ'))).toString(), note.source); await assert.rejects(readFile(path.join(rb, 'Other', 'Other.typ')));
  result.cases.push('actual UI upload with lost start response recovered by original job ID, remote selection, receive without author writes, and reviewed partial clone');
  async function save(server, root, suffix) {
    const doc = await api(server, '/api/open', { path: path.join(root, 'Note', 'Note.typ'), root }); const source = doc.source + '\n' + suffix;
    await api(server, '/api/save', { sessionId: doc.sessionId, baseId: doc.baseId, source, operationId: randomUUID() }); return source;
  }
  await save(a, ra, '甲离线 😀'); await save(b, rb, '乙离线 中文');
  await click(pa, '刷新已保存版本'); await transfer(pa, '上传所选本机对象');
  await click(pb, '刷新已保存版本');
  // Only Note was received and installed; the partial replica publishes that object.
  assert.equal(await pb.locator('.sync-object').filter({ hasText: 'Other (' }).count(), 0);
  await transfer(pb, '上传所选本机对象');
  await pb.getByRole('button', { name: '读取远端目录', exact: true }).click(); await pb.getByLabel(/^远端：Note /).waitFor(); await transfer(pb, '接收所选远端对象');
  await pb.locator('.sync-object').filter({ hasText: '冲突：保留全部分支' }).waitFor();
  await pb.screenshot({ path: path.join(out, 'remote-conflict.png') }); result.cases.push('offline saves exchanged through relay preserve both conflict branches in the actual UI');
  for (const p of [aPage, bPage]) {
    const storage = await p.evaluate(() => JSON.stringify({ local: { ...localStorage }, session: { ...sessionStorage } })); assert.equal(storage.includes(ownerToken), false);
  }
  const policy = await api(relay, '/api/hosted/admin/policy', undefined, adminToken); policy.users.owner.syncOwner = false;
  await api(relay, '/api/hosted/admin/policy', { expectedGeneration: policy.generation, users: policy.users }, adminToken);
  await pb.getByRole('button', { name: '读取远端目录', exact: true }).click(); await pb.getByRole('status', { name: '远程传输进度' }).filter({ hasText: '远端拒绝' }).waitFor();
  assert.equal(await pb.getByLabel(/^远端：/).count(), 0);
  await pb.screenshot({ path: path.join(out, 'remote-revoked.png') });
  await pa.getByRole('button', { name: '关闭同步', exact: true }).click(); pa = await sync(aPage); assert.equal(await pa.getByLabel('完整同步凭据', { exact: true }).inputValue(), '');
  result.cases.push('remote credentials absent from browser storage and cleared on close; revocation rejects fresh remote reads and clears the remote directory');
  assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) {
  result.failure = String(error.stack ?? error); await writeFile(path.join(out, 'results.json'), JSON.stringify(result, null, 2));
  for (const [name, p] of [['a', aPage], ['b', bPage]]) if (p) await p.screenshot({ path: path.join(out, `failure-${name}.png`), fullPage: true }).catch(() => {});
  throw error;
} finally {
  await browser?.close(); for (const child of children) await stop(child);
  await writeFile(path.join(out, 'results.json'), JSON.stringify(result, null, 2)); await writeFile(path.join(out, 'server.stderr.log'), stderr);
  console.log(JSON.stringify({ out, completed: result.completed, cases: result.cases, pageErrors: result.pageErrors }));
}
