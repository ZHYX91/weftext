import { spawnServe, serveReady, stopServe, stringifyServeJson } from './_cli_serve.mjs';
import { createdDocument } from './_creation_result.mjs';
// Owned-library browser acceptance of the embedded native authoring controls.
// Headless keyboard input does not establish real operating-system IME support.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import { createRequire } from 'node:module';
import path from 'node:path';
const require = createRequire(import.meta.url), { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]);
await mkdir(out, { recursive: false });
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const result = { exe, sha256: sha(await readFile(exe)), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(), frontend: 'embedded candidate', cases: [], pageErrors: [], completed: false, limits: ['Headless Chrome only; no actual OS IME or assistive-technology acceptance.'] };
let server, browser, page, base, token, stderr = '';
let originals = [];
const root = path.join(out, 'library'), data = path.join(out, 'state');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
// Core/Typst may briefly hold an ordinary Windows sharing lock while the
// asynchronous current-preview scan completes. Retry only the owned fixture
// original read, preserving exact-byte assertions and every HTTP failure.
async function readOwnedOriginal(file) {
  assert.ok(path.resolve(file).startsWith(root + path.sep), 'Only this test fixture library can use the sharing-lock read retry');
  for (let attempt = 0; attempt < 40; attempt++) {
    try { return await readFile(file, 'utf8'); }
    catch (error) {
      if (!['EBUSY', 'EACCES'].includes(error.code) || attempt === 39) throw error;
      (result.ownedOriginalReadRetries ??= []).push({ path: file, code: error.code, attempt: attempt + 1 });
      await pause(50);
    }
  }
}
async function start(args = []) {
  const child = spawnServe(exe, ['--data-dir', data, ...args], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  let stdout = ''; child.stdout.on('data', bytes => stdout += bytes); child.stderr.on('data', bytes => stderr += bytes);
  for (let i = 0; i < 300; i++) {
    const match = serveReady(child);
    if (match) { base = match.base; token = match.token; return child; }
    if (child.exitCode !== null) throw new Error(stderr || 'Candidate exited before startup');
    await pause(100);
  }
  await stopServe(child); throw new Error('Candidate startup timeout');
}
async function stop() { await stopServe(server); server = null; }
async function api(route, body) {
  const reply = await fetch(base + route, { method: body === undefined ? 'GET' : 'POST', headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': token }, body: body === undefined ? undefined : JSON.stringify(body) });
  const value = await reply.json(); assert.equal(reply.status, 200, JSON.stringify(value)); return value;
}
async function save(doc, source) { return api('/api/save', { sessionId: doc.sessionId, baseId: doc.baseId, source, operationId: randomUUID() }); }
const content = () => page.locator('.source-host .cm-content');
const source = async () => (await page.locator('.source-host .cm-line').allTextContents()).join('\n');
const click = (name, scope = page) => scope.getByRole('button', { name, exact: true }).click();
async function waitSource(expected) { await page.waitForFunction(expected => Array.from(document.querySelectorAll('.source-host .cm-line')).map(node => node.textContent).join('\n') === expected, expected); }
async function end() { await content().click(); await content().press('ControlOrMeta+End'); }
async function undoRedo(before, after) {
  await content().click(); await content().press('ControlOrMeta+z'); await waitSource(before);
  await content().press('ControlOrMeta+Shift+Z'); await waitSource(after);
}
async function passed(description) { result.cases.push(description); await writeFile(path.join(out, 'progress.json'), stringifyServeJson(result, 2)); process.stdout.write(description + '\n'); }
const inspector = () => page.locator('.workspace-inspector');
async function refreshCitations() {
  await page.getByText('草稿预览已更新', { exact: true }).waitFor();
  const expected = await source(), read = page.waitForRequest(request => request.url().endsWith('/api/knowledge/read'));
  await click('刷新', inspector());
  const request = await read, requested = request.postDataJSON(), reply = await request.response(), value = await reply.json();
  await writeFile(path.join(out, `knowledge-read-${result.cases.length}.json`), JSON.stringify({ request: requested, status: reply.status(), value }, null, 2));
  assert.equal(requested.context.source, expected); assert.ok(Number.isSafeInteger(requested.context.serial));
  assert.equal(reply.status(), 200, JSON.stringify(value));
  assert.equal(value.citations.editor.sourceHash, sha(Buffer.from(expected)));
  await page.getByRole('tab', { name: '插入引用', exact: true }).click();
  await page.getByLabel('插入引用类型', { exact: true }).waitFor();
}
async function apply(title, before, confirm = true) {
  await click(title, inspector());
  const dialog = page.locator('dialog.knowledge-dialog');
  await dialog.waitFor();
  assert.equal(await source(), before, 'prepared proposal must leave the Draft unchanged before confirmation');
  await click(confirm ? '确认应用' : '取消', dialog); await dialog.waitFor({ state: 'detached' });
  if (!confirm) await waitSource(before);
  else await page.waitForFunction(before => Array.from(document.querySelectorAll('.source-host .cm-line')).map(node => node.textContent).join('\n') !== before, before);
  return source();
}
try {
  server = await start(); await api('/api/library/create', { path: root, operationId: randomUUID() });
  const schema = await api('/api/data/schema', { root }), schemaOperation = randomUUID();
  const schemaPlan = await api('/api/data/schema/prepare', { root, source: schema.source, beforeSource: schema.source, beforeRevision: schema.revision, operationId: schemaOperation });
  await api('/api/data/schema/install', { operationId: schemaOperation, planId: schemaPlan.planId });
  const doc = createdDocument(await api('/api/note/create', { root, parent: root, name: 'Authoring', operationId: randomUUID() }));
  const work = createdDocument(await api('/api/note/create', { root, parent: root, name: 'Paper', operationId: randomUUID() }));
  const workSource = `#metadata((id: "${work.nodeId}", types: ("paper",), publication: (kind: "article", title: "Browser Publication", contributors: ((agent: (text: "Author Alpha"), roles: ("author",)),)))) <weftext>\n#set document(title: [Paper])\n#title()\n\nPRIVATE OWNED WORK BODY\n`;
  await save(work, workSource);
  const initial = doc.source + '\n#set heading(numbering: "1.")\n= 原生目标😀 <sec>\n\n正文保留。\n\n'; await save(doc, initial);
  originals = [{ path: doc.path, source: initial }, { path: work.path, source: workSource }];
  await stop(); server = await start(['--open', doc.path, '--root', root]);
  browser = await chromium.launch({ headless: true, ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : { channel: 'chrome' }) }); result.browser = browser.version();
  page = await browser.newPage({ viewport: { width: 1480, height: 1080 } }); page.setDefaultTimeout(40000); page.on('pageerror', error => result.pageErrors.push(String(error))); page.on('dialog', dialog => dialog.dismiss());
  result.httpErrors = []; page.on('response', async reply => { if (reply.url().includes('/api/') && reply.status() >= 400) result.httpErrors.push({ url: reply.url(), status: reply.status(), body: await reply.text().catch(() => null) }); });
  await page.goto(base + '/#token=' + token); await click('源码'); await content().waitFor(); await waitSource(initial); await end();
  await click('LaTeX 公式'); let dialog = page.locator('dialog.latex-insertion'); const input = dialog.getByLabel('原始 LaTeX（转换或取消均保留）', { exact: true });
  await input.fill(String.raw`\frac{x^2}{\sqrt{y}}+\alpha`); await click('转换并预览', dialog); await dialog.getByRole('button', { name: '确认插入原生公式', exact: true }).waitFor();
  await page.waitForFunction(() => !document.querySelector('dialog.latex-insertion button.primary').disabled);
  const converted = await dialog.locator('.output-source').innerText(); assert.ok(converted.includes('sqrt')); assert.equal(await source(), initial); assert.ok(await dialog.getByLabel('插入后的完整原生文档预览', { exact: true }).isVisible());
  await click('确认插入原生公式', dialog); await waitSource(initial + converted); await click('关闭', dialog); await undoRedo(initial, initial + converted); await content().press('ControlOrMeta+z'); await waitSource(initial);
  await passed('LaTeX toolbar conversion renders a full native preview before confirmation; exact native formula enters one Draft Undo/Redo without saving.');
  await end(); await click('LaTeX 公式'); dialog = page.locator('dialog.latex-insertion[open]'); await dialog.getByLabel('原始 LaTeX（转换或取消均保留）', { exact: true }).fill(String.raw`\unknown{a}`); await click('转换并预览', dialog); await dialog.getByRole('status').filter({ hasText: /不支持|未知/ }).waitFor(); assert.equal(await dialog.getByRole('button', { name: '确认插入原生公式', exact: true }).isEnabled(), false);
  await click('取消本次插入', dialog); assert.equal(await dialog.getByLabel('原始 LaTeX（转换或取消均保留）', { exact: true }).inputValue(), String.raw`\unknown{a}`); assert.equal(await source(), initial); await click('关闭', dialog);
  await click('LaTeX 公式'); dialog = page.locator('dialog.latex-insertion[open]'); assert.equal(await dialog.getByLabel('原始 LaTeX（转换或取消均保留）', { exact: true }).inputValue(), String.raw`\unknown{a}`); await click('关闭', dialog);
  await passed('Unsupported LaTeX cannot confirm; cancel and reopen retain the exact original input and leave the Draft unchanged.');
  await click('链接与批注'); await refreshCitations(); await end();
  await page.getByLabel('插入引用类型', { exact: true }).selectOption('link'); const target = page.getByLabel('笔记或文献目标', { exact: true });
  const paperOption = target.locator('option').filter({ hasText: /Paper/ }); assert.equal(await paperOption.count(), 1); const paperRef = await paperOption.getAttribute('value'); await target.selectOption(paperRef); await page.getByLabel('自定义显示文字（空值使用当前标题）', { exact: true }).fill('自定😀文献链接');
  await apply('插入普通笔记链接', initial, false); assert.equal(await page.getByLabel('自定义显示文字（空值使用当前标题）', { exact: true }).inputValue(), '自定😀文献链接');
  let current = await apply('插入普通笔记链接', initial); assert.equal(current, initial + `#link("${paperRef}")[#"自定😀文献链接"]`); await undoRedo(initial, current);
  await passed('Cross-note chooser inserts the selected stable native identity with explicit display text; proposal cancellation and one Undo/Redo preserve the exact source.');
  await refreshCitations(); await end(); await page.getByLabel('插入引用类型', { exact: true }).selectOption('reference'); await page.getByLabel('本篇交叉引用目标', { exact: true }).selectOption('sec'); await page.getByLabel('交叉引用形式', { exact: true }).selectOption('normal');
  let before = current; current = await apply('插入本篇交叉引用', before); assert.equal(current, before + '#ref(<sec>)'); await undoRedo(before, current);
  await refreshCitations(); await end(); await page.getByLabel('插入引用类型', { exact: true }).selectOption('reference'); await page.getByLabel('本篇交叉引用目标', { exact: true }).selectOption('sec'); await page.getByLabel('交叉引用形式', { exact: true }).selectOption('text'); await page.getByLabel('自定义显示文字（空值使用当前标题）', { exact: true }).fill('节选😀');
  before = current; current = await apply('插入本篇交叉引用', before); assert.equal(current, before + '#link(<sec>)[#"节选😀"]'); await undoRedo(before, current);
  await passed('The in-document target chooser supports numbered native references and explicit custom-text links, each as one exact Draft operation.');
  await refreshCitations(); await end(); await page.getByLabel('插入引用类型', { exact: true }).selectOption('citation'); await page.getByLabel('笔记或文献目标', { exact: true }).selectOption(paperRef); await page.getByLabel('原生引用键（已有绑定优先复用）', { exact: true }).fill('browser2026'); await page.getByLabel('引用形式', { exact: true }).selectOption('prose'); await page.getByLabel('页码或章节补充（纯文字）', { exact: true }).fill('第 12 页');
  before = current; current = await apply('插入正式文献引用', before); assert.ok(current.includes('browser2026')); assert.ok(current.includes('prose')); assert.ok(current.includes('第 12 页')); assert.ok(current.includes(paperRef)); await undoRedo(before, current);
  await refreshCitations(); await page.getByLabel('将 browser2026 改名为', { exact: true }).fill('browserRenamed'); before = current; current = await apply('预览完整改键范围', before); assert.ok(current.includes('browserRenamed')); assert.ok(!current.includes('browser2026')); await undoRedo(before, current);
  await passed('A publication-backed Work produces a native prose citation and plain-text supplement; complete key rename updates its binding and authored slot in one shared Undo/Redo.');
  await click('刷新', inspector()); await page.getByRole('tab', { name: '关系', exact: true }).click(); await page.getByLabel('关系分类', { exact: true }).selectOption('document'); await inspector().getByText('本次原生文内引用', { exact: true }).first().waitFor(); assert.equal(await inspector().getByRole('heading', { name: '出链', exact: true }).count(), 0);
  await page.getByLabel('关系分类', { exact: true }).selectOption('links'); await inspector().getByRole('button', { name: 'Paper', exact: true }).first().waitFor(); assert.equal(await inspector().getByText('本次原生文内引用', { exact: true }).count(), 0);
  await page.getByLabel('关系分类', { exact: true }).selectOption('citations'); await page.getByLabel('将 browserRenamed 改名为', { exact: true }).waitFor(); assert.equal(await inspector().getByRole('button', { name: '定位源码：browserRenamed', exact: true }).count(), 1); assert.equal(await source(), current);
  await passed('Relationship classification distinguishes native document references, ordinary note links and the actual publication-backed authored citation.');
  await click('阅读'); await page.getByRole('tab', { name: '插入引用', exact: true }).click(); await page.getByLabel('插入引用类型', { exact: true }).selectOption('citation'); assert.equal(await inspector().getByRole('button', { name: '插入正式文献引用', exact: true }).isEnabled(), false); assert.equal(await inspector().getByRole('button', { name: '预览完整改键范围', exact: true }).isEnabled(), false); assert.equal(await page.locator('.source-host').count(), 0);
  await click('源码'); await waitSource(current); assert.equal(await readOwnedOriginal(doc.path), initial); assert.equal(await readOwnedOriginal(work.path), workSource);
  await passed('Read view disables citation mutations and hides the source editor; all authoring tasks preserve both saved originals.');
  await page.screenshot({ path: path.join(out, 'authoring-complete.png'), fullPage: true }); assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) {
  result.failure = error.stack || String(error);
  if (page) { result.failureBody = await page.locator('body').innerText().catch(() => 'unavailable'); await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {}); }
  throw error;
} finally {
  await browser?.close(); await stop();
  result.originals = await Promise.all(originals.map(async original => { const bytes = await readFile(original.path); return { path: original.path, expectedSha256: sha(Buffer.from(original.source)), actualSha256: sha(bytes), unchanged: bytes.equals(Buffer.from(original.source)) }; }));
  result.stderr = stderr; await writeFile(path.join(out, 'result.json'), stringifyServeJson(result, 2));
}
process.stdout.write(stringifyServeJson(result, 2));
