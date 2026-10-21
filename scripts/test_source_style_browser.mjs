import { spawnServe, serveReady, stopServe, stringifyServeJson } from './_cli_serve.mjs';
import { createdDocument } from './_creation_result.mjs';
// Embedded-product source appearance regression. All libraries/processes belong
// to this run. Browser text insertion is not real Windows/OS IME acceptance.
// Usage: node scripts/test_source_style_browser.mjs EXE NEW_OUT [--hosted]
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash, randomBytes, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
assert.ok(process.argv[2] && process.argv[3], 'EXE and a new owned evidence directory are required');
const exe = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]);
await mkdir(out, { recursive: false });
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const result = { exe, sha256: sha(await readFile(exe)), harnessSha256: sha(await readFile(new URL(import.meta.url))), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(),
  frontend: 'embedded executable; no Vite, intercepted responses or mock parser', cases: [], pageErrors: [], http: [], processes: [], completed: false,
  limits: ['Browser keyboard.insertText is not real OS IME/composition/candidate-window acceptance.', 'Short owned document at two viewport widths; no large-document or assistive-technology claim.'] };
const actors = { alice: randomBytes(32).toString('hex'), admin: randomBytes(32).toString('hex') };
const root = path.join(out, 'library'), data = path.join(out, 'private-state');
let server, browser, page, base, token, stderr = '', original, doc;
const pendingResponses = new Set();
let nextHttp = 0;
async function start(args = [], hosted = false) {
  const child = spawnServe(exe, ['--data-dir', data, ...args], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  const identity = { pid: child.pid, args: child.spawnargs.slice(1), started: new Date().toISOString() }; result.processes.push(identity);
  let stdout = ''; child.stdout.on('data', bytes => stdout += bytes); child.stderr.on('data', bytes => stderr += bytes);
  child.once('exit', (code, signal) => Object.assign(identity, { exitCode: code, signal, closed: new Date().toISOString() }));
  for (let index = 0; index < 300; index++) {
    const found = serveReady(child);
    if (found) { base = found.base; token = found.token; return child; }
    if (child.exitCode !== null) throw new Error(stderr || 'Owned candidate exited before startup');
    await pause(100);
  }
  await stopServe(child); throw new Error('Owned candidate startup timeout');
}
async function stop() { await stopServe(server); server = undefined; }
async function api(route, body, actor) {
  const response = await fetch(base + route, { method: body === undefined ? 'GET' : 'POST', headers: { 'Content-Type': 'application/json', ...(actor ? { Authorization: 'Bearer ' + actors[actor] } : { 'X-Weftext-Token': token }) }, body: body === undefined ? undefined : JSON.stringify(body) });
  const value = await response.json(); assert.equal(response.status, 200, JSON.stringify(value)); return value;
}
const content = () => page.locator('.cm-content').first();
const source = async () => (await content().locator('.cm-line').allTextContents()).join('\n');
async function waitSource(expected) { await page.waitForFunction(expected => Array.from(document.querySelector('.cm-content')?.querySelectorAll('.cm-line') || []).map(line => line.textContent).join('\n') === expected, expected); }
const modeName = (hosted, styled) => hosted ? styled ? 'V02 带样式源码' : 'V01 源码' : styled ? '样式源码' : '源码';
async function mode(hosted, styled) {
  const button = page.getByRole('button', { name: modeName(hosted, styled), exact: true }); await button.click(); await button.getAttribute('aria-pressed').then(value => assert.equal(value, 'true'));
}
async function current(expected) {
  await waitSource(expected);
  await content().locator('span.cm-typst-heading').first().waitFor();
  await page.waitForFunction(() => document.querySelector('.cm-content')?.getAttribute('data-source-style-status') === 'current');
}
async function focusBody() {
  const line = content().locator('.cm-line').filter({ hasText: /^正文输入区：/ }).first(); await line.click(); await page.keyboard.press('End');
}
async function fonts() {
  return page.evaluate(async () => { await document.fonts.ready; return { status: document.fonts.status, mono: document.fonts.check('15px "Noto Sans Mono CJK SC"'), heading: document.fonts.check('20.25px "Noto Sans CJK SC"'), faces: Array.from(document.fonts).map(face => ({ family: face.family, status: face.status })) }; });
}
async function observe(label) {
  await page.evaluate(label => {
    const observed = { label, phase: 'baseline', frames: [], mutations: [], stopped: false };
    const snapshot = kind => {
      const editor = document.querySelector('.cm-content'), lines = Array.from(editor?.querySelectorAll('.cm-line') || []);
      const heading = lines.find(line => line.textContent === '= 稳定标题'), input = lines.find(line => line.textContent.startsWith('正文输入区：'));
      const rect = element => element ? { top: element.getBoundingClientRect().top, height: element.getBoundingClientRect().height } : null;
      const notice = document.querySelector('.cm-source-style-notice');
      return { at: performance.now(), kind, phase: observed.phase, status: editor?.getAttribute('data-source-style-status'), description: editor?.getAttribute('aria-description'), heading: rect(heading), input: rect(input),
        relativeInputTop: input && editor ? input.getBoundingClientRect().top - editor.getBoundingClientRect().top : null,
        officialHeadings: editor?.querySelectorAll('.cm-typst-heading').length || 0, appearances: editor?.querySelectorAll('.cm-source-appearance-heading').length || 0,
        pendingAppearances: editor?.querySelectorAll('.cm-source-appearance-heading[data-source-style=pending]').length || 0,
        headingClasses: heading?.className, headingText: heading?.textContent, inputText: input?.textContent,
        notice: !!notice && !notice.hidden && getComputedStyle(notice).display !== 'none', noticeText: notice?.textContent };
    };
    const frame = () => { if (!observed.stopped) { observed.frames.push(snapshot('animation-frame')); requestAnimationFrame(frame); } };
    const mutation = new MutationObserver(() => observed.mutations.push(snapshot('mutation')));
    mutation.observe(document.querySelector('.cm-editor'), { subtree: true, childList: true, characterData: true, attributes: true, attributeFilter: ['class', 'data-source-style', 'data-source-style-status', 'hidden'] });
    observed.mutation = mutation; window.sourceStyleObservation = observed; requestAnimationFrame(frame);
  }, label);
  await pause(80);
}
async function phase(value) { await page.evaluate(value => window.sourceStyleObservation.phase = value, value); }
async function collect(name) {
  const observed = await page.evaluate(() => { const value = window.sourceStyleObservation; value.stopped = true; value.mutation.disconnect(); const { mutation, ...serializable } = value; return serializable; });
  const file = name + '-frames.json'; await writeFile(path.join(out, file), JSON.stringify(observed, null, 2)); return { observed, file };
}
async function passed(description, evidence = {}) {
  result.cases.push({ description, ...evidence }); await writeFile(path.join(out, 'progress.json'), stringifyServeJson(result, 2)); process.stdout.write(description + '\n');
}
async function attachPage(viewport) {
  page = await browser.newPage({ viewport }); page.setDefaultTimeout(30000);
  page.on('pageerror', error => result.pageErrors.push(String(error))); page.on('dialog', dialog => dialog.dismiss());
  page.on('response', response => {
    if (!response.url().includes('/api/')) return;
    const request = response.request(), route = new URL(response.url()).pathname;
    const record = { id: ++nextHttp, route, status: response.status(), observedAt: performance.now() }; result.http.push(record);
    let body; try { body = request.postDataJSON(); } catch { /* GET */ }
    if (body?.source !== undefined) Object.assign(record, { requestSource: body.source, requestSha256: sha(Buffer.from(body.source)), requestSerial: body.serial });
    if (!/\/(?:render|editor\/render|editor-map|editor\/map)$/.test(route)) return;
    const pending = (async () => { try {
      const bytes = await response.body(), value = JSON.parse(bytes); record.bodySha256 = sha(bytes); record.bodyBytes = bytes.length;
      const file = `http-${record.id}.json`; await writeFile(path.join(out, file), bytes); record.file = file;
      record.responseSource = value.source; record.responseSerial = value.serial; record.syntaxValid = value.sourceInfo?.syntaxValid;
      record.headingTokens = value.sourceInfo?.syntaxTokens?.filter(token => token.kind.toLowerCase().includes('heading')) || [];
    } catch (error) { record.observationError = String(error); } })();
    pendingResponses.add(pending); pending.finally(() => pendingResponses.delete(pending));
  });
}
async function typingGeometry(hosted, width) {
  await page.setViewportSize({ width, height: 900 }); await mode(hosted, true); await current(original);
  const fontState = await fonts(); assert.equal(fontState.status, 'loaded'); assert.ok(fontState.mono && fontState.heading, 'Actual product fonts must finish loading before geometry observation');
  await focusBody(); await pause(100); await observe(`${hosted ? 'hosted' : 'local'}-${width}`); await phase('typing');
  let expected = original;
  for (const character of ['测', '试', '中', '文']) {
    await page.keyboard.insertText(character); expected = expected.replace(/正文输入区：[^\n]*/, text => text + character); await waitSource(expected); await pause(110);
  }
  await current(expected); await pause(100); await phase('settled'); await pause(50);
  const name = `${hosted ? 'hosted' : 'local'}-${width}`, { observed, file } = await collect(name);
  const baseline = observed.frames.find(frame => frame.phase === 'baseline' && frame.status === 'current'); assert.ok(baseline?.heading && baseline?.input, 'Baseline frame has both measured lines');
  const typed = observed.frames.filter(frame => frame.phase === 'typing'); assert.ok(typed.length >= 8, 'Observe actual paints throughout input and parse');
  assert.ok(typed.every(frame => frame.heading && frame.input && ['current', 'pending'].includes(frame.status)), 'Both measured lines and a qualified style state survive every observed input frame');
  const pending = [...typed, ...observed.mutations.filter(frame => frame.phase === 'typing')].filter(frame => frame.status === 'pending');
  assert.ok(pending.length, 'Observe a real pending interval, not only final current state');
  for (const frame of pending) { assert.equal(frame.officialHeadings, 0, 'Old official semantic classes retire immediately'); assert.ok(frame.appearances && frame.pendingAppearances, 'Unchanged heading footprint is explicitly pending'); assert.ok(frame.notice && /待|更新|暂存/.test(frame.noticeText), 'Pending appearance has a visible explanation'); }
  const geometry = [baseline, ...typed], heightRange = Math.max(...geometry.map(frame => frame.heading.height)) - Math.min(...geometry.map(frame => frame.heading.height)), inputTopRange = Math.max(...geometry.map(frame => frame.relativeInputTop)) - Math.min(...geometry.map(frame => frame.relativeInputTop)), viewportInputTopRange = Math.max(...geometry.map(frame => frame.input.top)) - Math.min(...geometry.map(frame => frame.input.top));
  assert.ok(heightRange <= 0.15, `Unchanged heading height varied by ${heightRange}px`); assert.ok(inputTopRange <= 0.15, `Input line shifted by ${inputTopRange}px`);
  assert.ok(viewportInputTopRange <= 0.15, `Input line shifted in the viewport by ${viewportInputTopRange}px`);
  await Promise.allSettled([...pendingResponses]);
  const official = result.http.findLast(record => record.responseSource === expected && record.headingTokens?.length && record.status === 200); assert.ok(official, 'Fresh official render uses the exact completed source'); assert.equal(official.responseSerial, official.requestSerial);
  const preview = page.locator(hosted ? '.hosted-flow' : '.preview-document').first(); await preview.getByText('正文输入区：测试中文', { exact: true }).waitFor();
  assert.equal(await readFile(doc.path, 'utf8'), original);
  await page.screenshot({ path: path.join(out, name + '-current.png'), fullPage: true });
  await focusBody(); await page.keyboard.press('ControlOrMeta+z'); await waitSource(original); await current(original);
  await page.keyboard.press('ControlOrMeta+Shift+Z'); await waitSource(expected); await current(expected);
  await page.keyboard.press('ControlOrMeta+z'); await waitSource(original); await current(original);
  await passed(`${name}: four ordinary browser text insertions preserve heading height and input-line position; pending visual footprint has no current semantic class; exact source/preview, one Undo/Redo, original disk bytes unchanged.`, { viewport: { width, height: 900 }, fonts: fontState, frames: file, paintedFrames: typed.length, pendingObservations: pending.length, heightRange, inputTopRange, viewportInputTopRange, officialHttp: official.id, expectedSha256: sha(Buffer.from(expected)) });
}
async function commentInvalidation(hosted) {
  await mode(hosted, true); await current(original); await content().click(); await page.keyboard.press('ControlOrMeta+Home'); await observe('unclosed-comment'); await phase('comment');
  await page.keyboard.insertText('/*\n'); const expected = '/*\n' + original; await waitSource(expected);
  await page.waitForFunction(() => document.querySelector('.cm-content')?.getAttribute('data-source-style-status') === 'current');
  await Promise.allSettled([...pendingResponses]);
  assert.equal(await content().locator('.cm-typst-heading').count(), 0, 'An invalidated heading is not current syntax');
  const official = result.http.findLast(record => record.responseSource === expected && /render$/.test(record.route)); assert.ok(official); assert.equal(official.headingTokens.length, 0);
  const { observed, file } = await collect('unclosed-comment'); const pending = observed.mutations.filter(frame => frame.status === 'pending'); assert.ok(pending.length); assert.ok(pending.every(frame => frame.officialHeadings === 0));
  await page.screenshot({ path: path.join(out, 'unclosed-comment-current.png'), fullPage: true });
  await content().click(); await page.keyboard.press('ControlOrMeta+z'); await waitSource(original); await current(original);
  await passed('An unclosed source-leading comment retires current heading semantics immediately; the next actual official parse confirms the former heading is no longer a heading; one Undo restores exact source and official style.', { frames: file, officialHttp: official.id });
}
async function inFlightModes(hosted) {
  await current(original); await focusBody(); const expected = original.replace('正文输入区：', '正文输入区：途中');
  const renderPath = hosted ? '/api/hosted/editor/render' : '/api/render';
  // Actual browser transport latency makes the transition deterministically
  // overlap a genuine response. No response interception, replay or mock data.
  const cdp = await page.context().newCDPSession(page); await cdp.send('Network.enable');
  await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 180, downloadThroughput: -1, uploadThroughput: -1 });
  const requestPromise = page.waitForRequest(request => new URL(request.url()).pathname === renderPath && request.postDataJSON()?.source === expected);
  await page.keyboard.insertText('途中'); await waitSource(expected); const request = await requestPromise;
  assert.ok(!result.http.some(record => record.route === renderPath && record.requestSource === expected), 'The actual response has not arrived before the mode transition');
  let responseObservedAt = null; const responsePromise = request.response().then(response => { responseObservedAt = performance.now(); return response; });
  const switchedAt = performance.now(); await mode(hosted, false); await mode(hosted, true); const restoredAt = performance.now();
  const response = await responsePromise; assert.equal(response.status(), 200); assert.ok(responseObservedAt >= switchedAt, 'V01 transition overlaps the actual render transport');
  const receipt = result.http.findLast(record => record.route === renderPath && record.requestSource === expected); assert.ok(receipt?.observedAt >= switchedAt, 'Passive browser response event independently follows transition start');
  await current(expected); assert.ok(await content().locator('.cm-source-appearance-heading.cm-source-appearance-rendered').count());
  await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: -1 }); await cdp.detach();
  await content().click(); await page.keyboard.press('ControlOrMeta+z'); await waitSource(original); await current(original);
  await passed('V01 transition begins while a real V02 render is in flight; switching back to V02 accepts fresh official styles and does not remain in none; source and one Undo remain exact.', { transport: 'CDP browser network latency 180ms only; unchanged backend budgets and responses', switchedAt, restoredAt, responseObservedAt, bothTransitionsInFlight: restoredAt < responseObservedAt });
}
try {
  server = await start(); await api('/api/library/create', { path: root, operationId: randomUUID() });
  doc = createdDocument(await api('/api/note/create', { root, parent: root, name: 'SourceStyle', operationId: randomUUID() }));
  original = doc.source + '\n= 稳定标题\n\n正文输入区：\n\n尾部保留。\n';
  await api('/api/save', { sessionId: doc.sessionId, baseId: doc.baseId, source: original, operationId: randomUUID() }); await stop();
  server = await start(['--open', doc.path, '--root', root]);
  browser = await chromium.launch({ headless: true, ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : { channel: 'chrome' }) }); result.browser = browser.version();
  await attachPage({ width: 1280, height: 900 }); await page.goto(base + '/#token=' + token); await content().waitFor(); await waitSource(original);
  await typingGeometry(false, 1280); await typingGeometry(false, 545); await page.setViewportSize({ width: 1280, height: 900 });
  await commentInvalidation(false); await inFlightModes(false); await page.close(); page = undefined; await stop();
  if (process.argv.includes('--hosted')) {
    const config = { deployment: 'source-style-browser', bind: '127.0.0.1:0', root, adminTokenSha256: sha(actors.admin), files: { style: { path: 'SourceStyle/SourceStyle.typ', title: '样式回归文档', editable: true } }, users: { alice: { enabled: true, tokenSha256: sha(actors.alice), grants: { style: { read: true, write: true } } } } };
    const configPath = path.join(out, 'server.json'); await writeFile(configPath, JSON.stringify(config, null, 2)); server = await start(['--server-config', configPath], true);
    await attachPage({ width: 1280, height: 900 }); await page.goto(base); await page.getByLabel('访问凭据', { exact: true }).fill(actors.alice); await page.getByRole('button', { name: '连接', exact: true }).click(); await page.getByRole('button', { name: '样式回归文档', exact: true }).click(); await content().waitFor(); await waitSource(original);
    await typingGeometry(true, 1280);
    const policy = await api('/api/hosted/admin/policy', undefined, 'admin'); policy.users.alice.grants = {}; await api('/api/hosted/admin/policy', { expectedGeneration: policy.generation, users: policy.users }, 'admin');
    await page.locator('#status').filter({ hasText: '缓存已清除' }).waitFor({ timeout: 15000 });
    await page.waitForFunction(() => Array.from(document.querySelectorAll('.cm-content')).every(editor => editor.textContent === ''));
    assert.equal(await page.locator('.cm-typst-heading,.cm-source-appearance-heading').count(), 0);
    const retired = await page.locator('.cm-content').evaluateAll(editors => editors.map(editor => ({ text: editor.textContent, status: editor.getAttribute('data-source-style-status') })));
    assert.ok(retired.every(editor => editor.text === '' && editor.status === 'none'), 'All surviving editors are empty with retired styles; complete editor destruction also retires the cache');
    assert.deepEqual(await page.evaluate(() => Object.keys(sessionStorage).filter(key => key.startsWith('weftext-hosted-draft:'))), []);
    await page.screenshot({ path: path.join(out, 'hosted-revoked.png'), fullPage: true }); await passed('Actual hosted read revocation removes source, current/pending appearance cache and personal retained draft; any surviving editor has style status none.', { survivingEditors: retired });
  }
  await Promise.allSettled([...pendingResponses]); assert.deepEqual(result.pageErrors, []); assert.ok(result.http.every(record => !record.observationError), 'All captured official response bodies were observed completely');
  assert.equal(await readFile(doc.path, 'utf8'), original); result.completed = true;
} catch (error) {
  result.failure = error.stack || String(error);
  if (page) { result.failureBody = await page.locator('body').innerText().catch(() => 'unavailable'); await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {}); }
  process.exitCode = 1;
} finally {
  await Promise.allSettled([...pendingResponses]); await browser?.close(); result.browserClosed = !!browser; await stop();
  if (doc && original) { const bytes = await readFile(doc.path); result.original = { path: doc.path, expectedSha256: sha(Buffer.from(original)), actualSha256: sha(bytes), unchanged: bytes.equals(Buffer.from(original)) }; }
  result.stderr = stderr; await writeFile(path.join(out, 'result.json'), stringifyServeJson(result, 2));
  process.stdout.write(JSON.stringify({ out, completed: result.completed, cases: result.cases.length, failure: result.failure, pageErrors: result.pageErrors }, null, 2) + '\n');
}
