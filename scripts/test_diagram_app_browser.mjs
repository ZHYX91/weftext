import { spawnServe, serveReady, stopServe, stringifyServeJson } from './_cli_serve.mjs';
// Real embedded-client Mermaid acceptance over an owned ordinary document.
// No mocked host, injected application state, Vite server or native Office.
// Usage: node scripts/test_diagram_app_browser.mjs <exe> <new-owned-output-dir>
// Pure ledger checks only: node scripts/test_diagram_app_browser.mjs --self-test
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';

const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const consumerPaths = new Set(['/api/render', '/api/editor-map', '/api/query/embedded/discover']);
const isHash = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
function renderResponseBasis(requestSource, body) {
  const source = typeof body?.source === 'string' ? body.source : null;
  const sourceSha256 = source === null ? null : hash(Buffer.from(source));
  return { ok: body?.ok === true, sessionId: body?.sessionId ?? null, serial: body?.serial ?? null,
    sourceSha256, sourceBytes: source === null ? null : Buffer.byteLength(source),
    exactSourceMatchesRequest: typeof requestSource === 'string' && source === requestSource,
    declaredSourceHashMatches: body?.sourceHash === undefined || body.sourceHash === sourceSha256 };
}

// An old request is expected to fail only when its replacement is independently
// proved current. Raw errors remain in the report; this never retries an action.
function classifyRenderErrors({ trace, httpErrors, finalSourceChecks, finalSourceBytes, finalSessionId, finalUiHasPreviewFailure }) {
  const allowedSuperseded = [], rejected = [], latestBySession = [];
  const reject = (reason, error = null) => rejected.push({ requestId: error?.requestId ?? null, pathname: error?.pathname ?? null, reason });
  const hashes = ['confirmed', 'saved', 'reloaded', 'uiVerified'].map(key => finalSourceChecks?.[key]);
  const finalHash = hashes[0];
  if (!hashes.every(value => isHash(value) && value === finalHash) || !Number.isSafeInteger(finalSourceBytes) || finalSourceBytes < 1) reject('Final confirmed, saved, reloaded and UI-verified source hashes must agree.');
  if (typeof finalSessionId !== 'string' || !finalSessionId) reject('Final UI session is missing.');
  if (finalUiHasPreviewFailure || trace.some(event => event.type === 'preview-failure-toast')) reject('The UI displayed a preview failure.');
  const requests = trace.filter(event => event.type === 'request' && consumerPaths.has(event.pathname));
  const responses = trace.filter(event => event.type === 'response');
  const validRequest = request => Number.isSafeInteger(request.requestId) && request.requestId > 0 && typeof request.sessionId === 'string' && !!request.sessionId && Number.isSafeInteger(request.serial) && request.serial >= 0 && isHash(request.sourceSha256) && Number.isSafeInteger(request.sourceBytes) && request.sourceBytes > 0 && Number.isFinite(request.monotonicMs) && request.monotonicMs >= 0;
  const responseFor = request => {
    const matches = responses.filter(response => response.requestId === request.requestId);
    if (matches.length !== 1) return null;
    const response = matches[0];
    return response.pathname === request.pathname && response.sessionId === request.sessionId && response.serial === request.serial && response.sourceSha256 === request.sourceSha256 && response.sourceBytes === request.sourceBytes && Number.isFinite(response.monotonicMs) && response.monotonicMs >= request.monotonicMs ? response : null;
  };
  const cache = new Map();
  function latest(sessionId) {
    if (cache.has(sessionId)) return cache.get(sessionId);
    const session = requests.filter(request => request.sessionId === sessionId);
    const fail = reason => { const result = { reason }; cache.set(sessionId, result); return result; };
    if (!session.length || !session.every(validRequest)) return fail('The session has missing or invalid request bases.');
    if (session.some(request => requests.filter(other => other.requestId === request.requestId).length !== 1)) return fail('A consumer request ID is not unique.');
    for (const serial of new Set(session.map(request => request.serial))) {
      const bases = session.filter(request => request.serial === serial);
      if (bases.some(request => request.sourceSha256 !== bases[0].sourceSha256 || request.sourceBytes !== bases[0].sourceBytes)) return fail('The same session/serial was used for different source bytes.');
    }
    const serial = Math.max(...session.map(request => request.serial));
    const current = session.filter(request => request.serial === serial);
    if (current.some(request => request.sourceSha256 !== finalHash || request.sourceBytes !== finalSourceBytes)) return fail('The highest consumer serial does not match the final verified source.');
    if (current.some(request => responseFor(request)?.status !== 200)) return fail('A highest-serial consumer request failed or is still incomplete.');
    const renders = current.filter(request => request.pathname === '/api/render');
    if (!renders.length) return fail('The highest consumer serial has no completed render.');
    for (const request of renders) {
      const basis = responseFor(request).renderBasis;
      if (!basis?.ok || basis.sessionId !== request.sessionId || basis.serial !== request.serial || basis.sourceSha256 !== request.sourceSha256 || basis.sourceBytes !== request.sourceBytes || basis.exactSourceMatchesRequest !== true || basis.declaredSourceHashMatches !== true) return fail('The current successful render body does not exactly match its request.');
    }
    const result = { serial, requests: current, renderRequestIds: renders.map(request => request.requestId) };
    cache.set(sessionId, result); latestBySession.push({ sessionId, serial, sourceSha256: finalHash, renderRequestIds: result.renderRequestIds }); return result;
  }
  const final = latest(finalSessionId);
  if (final.reason) reject('Final UI render: ' + final.reason);
  for (const request of requests) if (!responseFor(request)) reject('A recorded consumer request has no complete, uniquely bound response.', request);
  for (const error of httpErrors) {
    if (error.pathname !== '/api/render' || error.status !== 409) { reject('Only an older /api/render HTTP 409 may be classified as superseded.', error); continue; }
    let code; try { code = JSON.parse(error.body).error?.code; } catch {}
    if (code !== 'stale_draft') { reject('The error code is not exactly stale_draft.', error); continue; }
    const matches = requests.filter(request => request.requestId === error.requestId);
    if (matches.length !== 1 || !validRequest(matches[0])) { reject('The error does not have one complete original request basis.', error); continue; }
    const request = matches[0], response = responseFor(request), current = latest(request.sessionId);
    if (request.pathname !== error.pathname || !response || response.pathname !== error.pathname || response.status !== error.status || response.error?.code !== code) { reject('The error is not bound to its exact recorded response.', error); continue; }
    if (current.reason) { reject(current.reason, error); continue; }
    if (request.serial >= current.serial) { reject('The failed request is not older than the highest serial.', error); continue; }
    if (!current.requests.every(replacement => request.monotonicMs < replacement.monotonicMs)) { reject('The failed request was not issued before the current consumer requests.', error); continue; }
    if (!requests.some(replacement => replacement.sessionId === request.sessionId && replacement.serial > request.serial && replacement.monotonicMs > request.monotonicMs && replacement.monotonicMs <= response.monotonicMs)) { reject('No newer serial had been requested when this failure arrived.', error); continue; }
    allowedSuperseded.push({ requestId: request.requestId, sessionId: request.sessionId, serial: request.serial, sourceSha256: request.sourceSha256, responseMonotonicMs: response.monotonicMs, replacementSerial: current.serial, replacementRenderRequestIds: current.renderRequestIds, reason: 'An already superseded serial was rejected; the same session highest serial rendered the exact final saved/reloaded source.' });
  }
  return { allowedSuperseded, rejected, latestBySession };
}

function selfTestRenderErrorGate() {
  const source = '当前中文😀', oldSource = '旧稿', finalHash = hash(Buffer.from(source));
  const request = (requestId, serial, text, monotonicMs, pathname = '/api/render', sessionId = 'session-a') => ({ type: 'request', requestId, pathname, sessionId, serial, sourceSha256: hash(Buffer.from(text)), sourceBytes: Buffer.byteLength(text), monotonicMs });
  const response = (req, status, monotonicMs, body) => ({ ...req, type: 'response', status, monotonicMs, ...(status === 200 && req.pathname === '/api/render' ? { renderBasis: renderResponseBasis(req.serial === 1 ? oldSource : source, body ?? { ok: true, source, serial: req.serial, sessionId: req.sessionId }) } : {}), ...(status >= 400 ? { error: { code: body?.code ?? 'stale_draft' } } : {}) });
  const older = request(1, 1, oldSource, 1), current = request(2, 2, source, 2);
  const good = () => structuredClone({ trace: [older, current, response(older, 409, 3), response(current, 200, 4)], httpErrors: [{ requestId: 1, pathname: '/api/render', status: 409, body: JSON.stringify({ error: { code: 'stale_draft' } }) }], finalSourceChecks: { confirmed: finalHash, saved: finalHash, reloaded: finalHash, uiVerified: finalHash }, finalSourceBytes: Buffer.byteLength(source), finalSessionId: 'session-a', finalUiHasPreviewFailure: false });
  let count = 0;
  const check = (name, mutate, allowed = false) => { const input = good(); const baseline = classifyRenderErrors(input); assert.equal(baseline.rejected.length, 0, name + ': fresh baseline'); assert.equal(baseline.allowedSuperseded.length, 1, name + ': fresh baseline'); mutate?.(input); const classified = classifyRenderErrors(input); assert.equal(classified.rejected.length === 0, allowed, name); if (allowed) assert.equal(classified.allowedSuperseded.length, input.httpErrors.length, name); count++; };
  check('older render with exact current successful body', null, true);
  check('no HTTP errors still requires exact final render', input => { input.httpErrors = []; }, true);
  check('reload session resets serial without proving another session error', input => { const req = request(3, 0, source, 5, '/api/render', 'session-reloaded'); input.trace.push(req, response(req, 200, 6)); input.finalSessionId = req.sessionId; }, true);
  check('two superseded generations require an issued replacement for each error', input => { const req = request(3, 3, source, 3.5); input.trace[3] = response(current, 409, 4); input.trace.push(req, response(req, 200, 5)); input.httpErrors.push({ ...input.httpErrors[0], requestId: 2 }); }, true);
  check('current serial failure', input => { input.httpErrors[0].requestId = 2; input.trace[3] = response(current, 409, 4); });
  check('cross-session success', input => { input.trace[1].sessionId = 'session-b'; input.trace[3].sessionId = 'session-b'; input.trace[3].renderBasis.sessionId = 'session-b'; input.finalSessionId = 'session-b'; });
  check('highest source differs from final', input => { input.trace[1].sourceSha256 = hash(Buffer.from('别稿')); input.trace[3].sourceSha256 = input.trace[1].sourceSha256; });
  check('resource collection error is never accepted', input => { input.httpErrors[0].body = JSON.stringify({ error: { code: 'resourceCollectionChanged' } }); input.trace[2].error.code = 'resourceCollectionChanged'; });
  check('HTTP 422 is never accepted', input => { input.httpErrors[0].status = 422; input.trace[2].status = 422; });
  check('an error appended after an earlier green ledger invalidates the gate', input => { input.httpErrors.push({ requestId: 99, pathname: '/api/draft', status: 422, body: JSON.stringify({ error: { code: 'lateFailure' } }) }); });
  check('non-render 409 is never accepted', input => { input.httpErrors[0].pathname = '/api/editor-map'; });
  check('raw error cannot borrow a different consumer path', input => { input.trace[0].pathname = '/api/editor-map'; input.trace[2].pathname = '/api/editor-map'; });
  for (const field of ['sourceSha256', 'sourceBytes', 'serial', 'sessionId', 'ok', 'exactSourceMatchesRequest', 'declaredSourceHashMatches']) check('success response mismatch: ' + field, input => { input.trace[3].renderBasis[field] = field === 'sourceSha256' ? hash(Buffer.from('错源')) : field === 'sessionId' ? 'wrong-session' : field === 'sourceBytes' || field === 'serial' ? 999 : false; });
  check('new highest consumer is pending', input => { input.trace.push(request(3, 3, source, 5, '/api/editor-map')); });
  check('new highest consumer failed', input => { const req = request(3, 3, source, 5, '/api/query/embedded/discover'); input.trace.push(req, response(req, 409, 6)); input.httpErrors.push({ requestId: 3, pathname: req.pathname, status: 409, body: input.httpErrors[0].body }); });
  check('higher successful render uses a different source', input => { const req = request(3, 3, '别稿', 5); input.trace.push(req, response(req, 200, 6, { ok: true, source: '别稿', serial: 3, sessionId: req.sessionId })); });
  check('highest sibling consumer failed', input => { const req = request(3, 2, source, 2.5, '/api/editor-map'); input.trace.push(req, response(req, 409, 6)); });
  check('higher request issued after failure cannot excuse it', input => { input.trace[1].monotonicMs = 3.5; });
  check('same serial with two source hashes', input => { input.trace.push(request(3, 1, '冲突稿', 1.5, '/api/editor-map')); });
  check('missing successful response body', input => { delete input.trace[3].renderBasis; });
  check('lower-serial request still in flight', input => { input.trace.push(request(3, 0, oldSource, 0.5, '/api/editor-map')); });
  check('invalid request timestamp', input => { input.trace[0].monotonicMs = -1; });
  check('missing request binding', input => { input.httpErrors[0].requestId = 99; });
  check('missing saved hash', input => { delete input.finalSourceChecks.saved; });
  check('wrong reloaded source', input => { input.finalSourceChecks.reloaded = hash(Buffer.from('错源')); });
  check('preview failure toast', input => { input.trace.push({ type: 'preview-failure-toast', monotonicMs: 3.1 }); });
  check('current UI preview failure', input => { input.finalUiHasPreviewFailure = true; });
  check('real response source is independently hashed', input => { input.trace[3].renderBasis = renderResponseBasis(source, { ok: true, source: '错误返回', serial: 2, sessionId: 'session-a' }); });
  check('declared response hash mismatch', input => { input.trace[3].renderBasis = renderResponseBasis(source, { ok: true, source, sourceHash: hash(Buffer.from('错误返回')), serial: 2, sessionId: 'session-a' }); });
  return count;
}
if (process.argv[2] === '--self-test') {
  process.stdout.write(JSON.stringify({ completed: true, pureMemoryCases: selfTestRenderErrorGate(), executableStarted: false, browserStarted: false }) + '\n'); process.exit(0);
}
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const exe = path.resolve(process.argv[2]), output = path.resolve(process.argv[3]);
const executableSha256 = hash(await readFile(exe));
await mkdir(output, { recursive: false });
const authorRoot = path.join(output, 'author-input'), stateRoot = path.join(output, 'private-state');
await mkdir(authorRoot);
const file = path.join(authorRoot, 'Mermaid-owned.typ');
const initialSource = '= Mermaid 原生图表验收\n\n原稿中文😀，确认前保持不变。\n';
const mermaidSource = 'flowchart LR\n  A[收集资料] --> B[校对中文] --> C[保存完成]';
await writeFile(file, initialSource);
const result = { completed: false, executable: exe, executableSha256,
  version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(),
  frontend: 'Actual embedded executable assets; no Vite or mock host',
  node: process.version, browser: '', browserChannel: process.env.WEFTEXT_BROWSER_CHANNEL || 'msedge',
  authorRoot, stateRoot, file, originalSha256: hash(Buffer.from(initialSource)),
  mermaidSource, cases: [], pageErrors: [], externalRequests: [], httpErrors: [],
  limits: ['Headless browser input; no desktop input, native Office, real OS IME or assistive-technology acceptance.',
    'This script verifies the actual local app client; module/browser-only and DOCX HTTP evidence remain separate.'] };
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const draftWrites = [], saveWrites = [], openReads = [], responseParsers = [];
const traceStarted = performance.now(), requestTrace = new WeakMap();
let currentStep = 'setup', nextRequestId = 0;
result.trace = [];
function trace(event) { const entry = { monotonicMs: Number((performance.now() - traceStarted).toFixed(3)), step: currentStep, ...event }; result.trace.push(entry); return entry; }
function step(name) { currentStep = name; trace({ type: 'step' }); }
function tracedBasis(request) {
  let body; try { body = request.postDataJSON(); } catch { body = null; }
  return { sessionId: body?.sessionId ?? null, serial: body?.serial ?? null,
    sourceSha256: typeof body?.source === 'string' ? hash(Buffer.from(body.source)) : null,
    sourceBytes: typeof body?.source === 'string' ? Buffer.byteLength(body.source) : null };
}
let server, browser, page, base, token, stdout = '', stderr = '', committedSource, finalGateContext;
async function readOwnedFile() {
  for (let attempt = 0; ; attempt++) {
    try { return await readFile(file); }
    catch (error) { if (!['EBUSY', 'EPERM'].includes(error.code) || attempt >= 30) throw error; await pause(50); }
  }
}
async function start() {
  server = spawnServe(exe, ['--data-dir', stateRoot, '--open', file, '--root', authorRoot],
    { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  result.serverPid = server.pid;
  server.stdout.on('data', bytes => stdout += bytes);
  server.stderr.on('data', bytes => stderr += bytes);
  for (let count = 0; count < 400; count++) {
    const found = serveReady(server);
    if (found) { base = found.base; token = found.token; result.loopbackBase = base; return; }
    if (server.exitCode !== null) throw new Error(stderr || 'Owned executable exited before startup');
    await pause(100);
  }
  throw new Error('Owned executable startup timeout');
}
async function stop() { await stopServe(server);  result.serverStopped = !!server && server.exitCode === 0 && server.signalCode === null; }
const click = (name, scope = page) => scope.getByRole('button', { name, exact: true }).click();
const surface = () => page.locator('.source-host .cm-content');
const dialog = () => page.locator('dialog.diagram-panel[open]');
async function waitDraft(predicate, after = 0) {
  for (let count = 0; count < 600; count++) {
    const found = draftWrites.slice(after).find(write => write.status === 200 && write.ok && predicate(write.source));
    if (found) return found;
    await pause(50);
  }
  throw new Error('Actual app did not persist the expected native Draft: ' + JSON.stringify(draftWrites.map(write => ({ sourceSha256: hash(Buffer.from(write.source)), status: write.status, ok: write.ok }))));
}
async function unchangedOriginal() { assert.equal((await readOwnedFile()).toString('utf8'), initialSource); }
async function readFinalUiSource(expected) {
  const domSource = await surface().evaluate(content => [...content.querySelectorAll('.cm-line')].map(line => line.textContent).join('\n'));
  const dom = { sourceSha256: hash(Buffer.from(domSource)), sourceBytes: Buffer.byteLength(domSource) };
  if (domSource === expected) return { source: domSource, method: 'Complete visible CodeMirror DOM', dom };
  // Normal editor selection/copy handlers provide virtualized long lines without
  // reading CodeMirror state or touching the system clipboard. Copy does not edit.
  await surface().click(); await surface().press('ControlOrMeta+a');
  const copied = await surface().evaluate(content => {
    const transfer = new DataTransfer();
    const event = new ClipboardEvent('copy', { clipboardData: transfer, bubbles: true, cancelable: true });
    content.dispatchEvent(event);
    return { source: transfer.getData('text/plain'), handled: event.defaultPrevented };
  });
  assert.equal(copied.handled, true, 'The real editor must handle the in-memory copy event');
  assert.equal(copied.source, expected, 'The full source read from the real editor must equal the saved/reopened native source');
  return { source: copied.source, method: 'Actual editor select-all and in-memory clipboard copy; no system clipboard', dom };
}
async function passed(message) {
  result.cases.push(message); await writeFile(path.join(output, 'progress.json'), stringifyServeJson(result, 2));
  process.stdout.write(message + '\n');
}
async function renderCurrent() {
  await click('生成 PNG 预览', dialog());
  await page.waitForFunction(() => {
    const open = document.querySelector('dialog.diagram-panel[open]');
    const confirm = [...open.querySelectorAll('button')].find(button => button.textContent === '确认当前图片并写入草稿');
    return !confirm.disabled || !!open.querySelector('.data-warning');
  });
  assert.equal(await dialog().locator('.data-warning').count(), 0, await dialog().innerText());
  assert.equal(await dialog().getByRole('button', { name: '确认当前图片并写入草稿', exact: true }).isEnabled(), true);
  return dialog().locator('.diagram-preview img').evaluate(async image => {
    await image.decode(); const canvas = document.createElement('canvas'); canvas.width = image.naturalWidth; canvas.height = image.naturalHeight;
    const context = canvas.getContext('2d'); context.drawImage(image, 0, 0);
    const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
    const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', pixels));
    return { pngBase64: canvas.toDataURL('image/png').split(',')[1], pixelSha256: [...digest].map(byte => byte.toString(16).padStart(2, '0')).join(''), width: image.naturalWidth, height: image.naturalHeight, alt: image.alt };
  });
}
async function pngPixelHash(png) {
  return page.evaluate(async encoded => {
    const image = new Image(); image.src = 'data:image/png;base64,' + encoded; await image.decode();
    const canvas = document.createElement('canvas'); canvas.width = image.naturalWidth; canvas.height = image.naturalHeight;
    const context = canvas.getContext('2d'); context.drawImage(image, 0, 0);
    const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
    return [...new Uint8Array(await crypto.subtle.digest('SHA-256', pixels))].map(byte => byte.toString(16).padStart(2, '0')).join('');
  }, png.toString('base64'));
}
try {
  await start();
  browser = await chromium.launch({ headless: true, ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : { channel: result.browserChannel }) });
  result.browser = browser.version();
  const context = await browser.newContext({ viewport: { width: 1480, height: 1080 } });
  await context.addInitScript(() => {
    const seen = new WeakSet();
    const inspect = node => {
      if (!(node instanceof Element)) return;
      for (const item of [...(node.matches('.toast') ? [node] : []), ...node.querySelectorAll('.toast')]) {
        const text = item.textContent || '';
        if (!seen.has(item) && text.includes('预览失败')) {
          seen.add(item); console.debug('WEFTEXT_ACCEPTANCE_TRACE:' + JSON.stringify({ type: 'preview-failure-toast', text, browserMonotonicMs: performance.now() }));
        }
      }
    };
    new MutationObserver(records => { for (const record of records) for (const node of record.addedNodes) inspect(node); }).observe(document, { childList: true, subtree: true });
  });
  await context.route('**/*', route => {
    const url = new URL(route.request().url());
    if (url.origin === base || ['blob:', 'data:'].includes(url.protocol)) return route.continue();
    result.externalRequests.push({ url: url.href, resourceType: route.request().resourceType() }); return route.abort();
  });
  page = await context.newPage(); page.setDefaultTimeout(40000);
  page.on('console', message => {
    const prefix = 'WEFTEXT_ACCEPTANCE_TRACE:';
    if (message.text().startsWith(prefix)) { try { trace(JSON.parse(message.text().slice(prefix.length))); } catch {} }
  });
  page.on('pageerror', error => result.pageErrors.push(String(error)));
  page.on('dialog', async prompt => { result.unexpectedNativeDialog = prompt.message(); await prompt.dismiss(); });
  page.on('request', request => {
    const pathname = new URL(request.url()).pathname;
    if (['/api/render', '/api/editor-map', '/api/query/embedded/discover', '/api/draft', '/api/save'].includes(pathname)) {
      const entry = trace({ type: 'request', requestId: ++nextRequestId, pathname, ...tracedBasis(request) }); requestTrace.set(request, entry);
    }
    if (pathname === '/api/draft') { const body = request.postDataJSON(); draftWrites.push({ request, source: body.source, sessionId: body.sessionId }); }
    if (pathname === '/api/save') { const body = request.postDataJSON(); saveWrites.push({ request, source: body.source, operationId: body.operationId }); }
  });
  page.on('response', response => { responseParsers.push((async () => {
    const pathname = new URL(response.url()).pathname;
    const requestEntry = requestTrace.get(response.request());
    let responseEntry;
    if (requestEntry) {
      responseEntry = trace({ type: 'response', requestId: requestEntry.requestId, requestStep: requestEntry.step, pathname, status: response.status(), sessionId: requestEntry.sessionId, serial: requestEntry.serial, sourceSha256: requestEntry.sourceSha256, sourceBytes: requestEntry.sourceBytes });
    }
    if (pathname.startsWith('/api/') && response.status() >= 400) {
      const body = await response.text().catch(() => ''); result.httpErrors.push({ requestId: requestEntry?.requestId ?? null, pathname, status: response.status(), body });
      if (responseEntry) {
        responseEntry.responseBodySha256 = hash(Buffer.from(body)); responseEntry.responseBodyBytes = Buffer.byteLength(body);
        try { const error = JSON.parse(body).error; if (error) responseEntry.error = { code: error.code, message: error.message }; } catch {}
      }
    }
    if (pathname === '/api/render' && response.status() === 200 && responseEntry) {
      const body = await response.json().catch(() => null);
      let requestSource; try { requestSource = response.request().postDataJSON()?.source; } catch {}
      responseEntry.renderBasis = renderResponseBasis(requestSource, body);
    }
    if (pathname === '/api/draft') {
      const found = draftWrites.find(write => write.request === response.request());
      if (found) { found.status = response.status(); found.ok = (await response.json().catch(() => ({}))).ok === true; }
    }
    if (pathname === '/api/save') {
      const found = saveWrites.find(write => write.request === response.request());
      if (found) { found.status = response.status(); found.receipt = await response.json().catch(() => ({})); }
    }
    if (pathname === '/api/open' && response.status() === 200) {
      const opened = await response.json().catch(() => null); if (opened) openReads.push({ source: opened.source, recoveredDraft: opened.recoveredDraft, sessionId: opened.sessionId });
    }
  })().catch(error => { (result.responseObserverErrors ??= []).push(String(error)); })); });
  step('open-original');
  await page.goto(base + '/#token=' + token);
  await click('源码'); await surface().waitFor();
  await page.waitForFunction(expected => [...document.querySelectorAll('.source-host .cm-line')].map(line => line.textContent).join('\n') === expected, initialSource);
  await page.getByText('草稿预览已更新', { exact: true }).waitFor();
  await unchangedOriginal();
  await passed('Actual embedded app opens an owned ordinary Typst document with its original source and live toolbar.');

  step('cancel-preview-render');
  await click('Mermaid 图表'); await dialog().waitFor();
  await dialog().getByLabel('Mermaid 源码', { exact: true }).fill(mermaidSource);
  await dialog().getByLabel('PNG 图片宽度（像素）', { exact: true }).fill('960');
  const cancelledPreview = await renderCurrent();
  result.cancelledPreview = { width: cancelledPreview.width, height: cancelledPreview.height, pixelSha256: cancelledPreview.pixelSha256 };
  await page.screenshot({ path: path.join(output, 'preview-before-cancel.png'), fullPage: true });
  await unchangedOriginal();
  step('cancel-preview-close');
  await click('关闭并保留输入', dialog());
  assert.equal(await page.locator('dialog.diagram-panel[open]').count(), 0);
  assert.equal(draftWrites.every(write => write.source === initialSource), true, 'Preview/cancel must not change the real app Draft');
  await unchangedOriginal();
  step('reopen-cancelled-input');
  await click('Mermaid 图表');
  await dialog().waitFor();
  assert.equal(await dialog().getByLabel('Mermaid 源码', { exact: true }).inputValue(), mermaidSource);
  assert.equal(await dialog().locator('.diagram-preview img').count(), 0);
  assert.equal(await dialog().getByRole('button', { name: '确认当前图片并写入草稿', exact: true }).isDisabled(), true);
  await passed('Real rendering and cancellation preserve the authored input while leaving both native Draft and saved original unchanged; reopening requires a fresh preview.');

  step('confirm-preview-render');
  const confirmedPreview = await renderCurrent();
  await writeFile(path.join(output, 'observed-preview-reencoded.png'), Buffer.from(confirmedPreview.pngBase64, 'base64'));
  const beforeConfirm = draftWrites.length;
  step('confirm-native-draft');
  await click('确认当前图片并写入草稿', dialog());
  await page.waitForFunction(() => !document.querySelector('dialog.diagram-panel[open]'));
  const nativeDraft = await waitDraft(source => source.includes('// weftext-mermaid v1 '), beforeConfirm);
  committedSource = nativeDraft.source;
  assert.equal((committedSource.match(/^  let source = /gm) || []).length, 1);
  assert.equal((committedSource.match(/^  let confirmed = /gm) || []).length, 1);
  assert.equal((committedSource.match(/^\/\/ weftext-mermaid v1 /gm) || []).length, 1);
  const literal = /^  let source = ("(?:\\.|[^"\\])*")$/m.exec(committedSource);
  const witness = /^  let confirmed = ("(?:\\.|[^"\\])*")$/m.exec(committedSource);
  assert.equal(JSON.parse(literal[1]), mermaidSource); assert.equal(JSON.parse(witness[1]), mermaidSource);
  const embedded = /image\(bytes\("([a-f0-9]+)"\.matches/.exec(committedSource);
  const png = Buffer.from(embedded[1], 'hex');
  assert.deepEqual([...png.subarray(0, 8)], [137, 80, 78, 71, 13, 10, 26, 10]);
  assert.equal(png.readUInt32BE(16), confirmedPreview.width); assert.equal(png.readUInt32BE(20), confirmedPreview.height);
  assert.equal(await pngPixelHash(png), confirmedPreview.pixelSha256);
  await writeFile(path.join(output, 'confirmed-native.png'), png);
  assert.ok(committedSource.startsWith(initialSource));
  await writeFile(path.join(output, 'confirmed-native-draft.typ'), committedSource);
  result.confirmed = { sourceSha256: hash(Buffer.from(committedSource)), sourceBytes: Buffer.byteLength(committedSource), pngSha256: hash(png), pngBytes: png.length, width: confirmedPreview.width, height: confirmedPreview.height, pixelSha256: confirmedPreview.pixelSha256, previewAndNativePixelsMatch: true, editableSourceBindings: 1, frozenWitnessBindings: 1 };
  await unchangedOriginal();
  await passed('Toolbar confirmation writes the exact current PNG and single editable Mermaid source through the real native Draft route without saving the original.');

  let after = draftWrites.length;
  step('undo-once');
  await surface().click(); await surface().press('ControlOrMeta+z');
  await waitDraft(source => source === initialSource, after); await unchangedOriginal();
  result.undoActions = 1;
  after = draftWrites.length;
  step('redo-once');
  await surface().press('ControlOrMeta+Shift+Z');
  await waitDraft(source => source === committedSource, after); await unchangedOriginal();
  result.redoActions = 1;
  await page.getByText('草稿预览已更新', { exact: true }).waitFor();
  await passed('One actual editor Undo removes the whole source/PNG Draft change; one Redo restores its exact bytes, while the saved original stays unchanged.');

  const saveResponse = page.waitForResponse(response => new URL(response.url()).pathname === '/api/save' && response.request().postDataJSON()?.source === committedSource);
  step('save-native-file');
  await page.locator('[data-command="save"]').click();
  const savedResponse = await saveResponse;
  assert.equal(savedResponse.status(), 200); const saveReceipt = await savedResponse.json(); assert.equal(saveReceipt.status, 'saved');
  for (let count = 0; count < 200 && !(await readOwnedFile()).equals(Buffer.from(committedSource)); count++) await pause(50);
  assert.deepEqual(await readOwnedFile(), Buffer.from(committedSource));
  await page.locator('.save-state').filter({ hasText: /^已保存$/ }).waitFor();
  await writeFile(path.join(output, 'saved-native.typ'), await readOwnedFile());
  const opensBefore = openReads.length;
  step('reload-saved-file');
  await page.reload(); await click('源码'); await surface().waitFor();
  await page.getByText('草稿预览已更新', { exact: true }).waitFor();
  step('reopen-saved-editable-source');
  await click('Mermaid 图表');
  await dialog().waitFor();
  const choose = dialog().getByLabel('图表位置', { exact: true });
  assert.equal(await choose.locator('option').count(), 2);
  await choose.selectOption('0');
  assert.equal(await dialog().getByLabel('Mermaid 源码', { exact: true }).inputValue(), mermaidSource);
  assert.equal(await dialog().getByRole('button', { name: '确认当前图片并写入草稿', exact: true }).isDisabled(), true);
  await page.screenshot({ path: path.join(output, 'reopened-editable-source.png'), fullPage: true });
  await click('关闭并保留输入', dialog());
  assert.deepEqual(await readOwnedFile(), Buffer.from(committedSource));
  result.saved = { sourceSha256: hash(await readOwnedFile()), sourceBytes: (await readOwnedFile()).length, saveOperationId: saveReceipt.operationId, uiReloadCompleted: true, actualOpenResponsesAfterReload: openReads.length - opensBefore, sameExistingSourceEditable: true };
  await passed('Actual Save writes the native source/PNG to disk; reloading the embedded app and selecting the existing diagram exposes the exact same editable source.');
  step('final-error-gate');
  // Settle only response-observation work; UI actions and their waits above are
  // unchanged. Requests still in flight have no response and fail classification.
  for (let index = 0; index < responseParsers.length; index++) await responseParsers[index];
  const reloadedDraft = draftWrites.findLast(write => write.status === 200 && write.ok && write.sessionId !== nativeDraft.sessionId && write.source === committedSource);
  assert.ok(reloadedDraft, 'The reopened real app must acknowledge the exact saved native Draft in its new session');
  result.saved.reloadedSessionId = reloadedDraft.sessionId;
  result.saved.reloadedDraftSourceSha256 = hash(Buffer.from(reloadedDraft.source));
  const uiSource = await readFinalUiSource(committedSource);
  result.finalUiSource = { sourceSha256: hash(Buffer.from(uiSource.source)), sourceBytes: Buffer.byteLength(uiSource.source), method: uiSource.method, observedDom: uiSource.dom };
  result.finalUiHasPreviewFailure = (await page.locator('body').innerText()).includes('预览失败');
  for (let index = 0; index < responseParsers.length; index++) await responseParsers[index];
  finalGateContext = { finalSourceChecks: { confirmed: result.confirmed.sourceSha256, saved: result.saved.sourceSha256, reloaded: result.saved.reloadedDraftSourceSha256, uiVerified: result.finalUiSource.sourceSha256 },
    finalSourceBytes: Buffer.byteLength(committedSource), finalSessionId: reloadedDraft.sessionId, finalUiHasPreviewFailure: result.finalUiHasPreviewFailure };
  result.renderErrorClassification = classifyRenderErrors({ trace: result.trace, httpErrors: result.httpErrors, ...finalGateContext });
  assert.deepEqual(result.externalRequests, []); assert.deepEqual(result.pageErrors, []); assert.deepEqual(result.responseObserverErrors ?? [], []);
  assert.equal(result.finalUiHasPreviewFailure, false); assert.deepEqual(result.renderErrorClassification.rejected, []);
  result.completed = true;
} catch (error) {
  result.failure = error.stack || String(error);
  if (page) { result.failureBody = await page.locator('body').innerText().catch(() => 'unavailable'); await page.screenshot({ path: path.join(output, 'failure.png'), fullPage: true }).catch(() => {}); }
  throw error;
} finally {
  step('close-owned-processes');
  await browser?.close(); await stop();
  for (let index = 0; index < responseParsers.length; index++) await responseParsers[index];
  let lateGateFailure;
  if (finalGateContext) {
    // No response can be silently appended after an earlier green gate. Preserve
    // the raw ledger and reclassify after the owned browser has fully closed.
    result.renderErrorClassification = classifyRenderErrors({ trace: result.trace, httpErrors: result.httpErrors, ...finalGateContext });
    result.finalLedgerSealedAfterBrowserClose = true;
    if (result.completed && (result.renderErrorClassification.rejected.length || result.externalRequests.length || result.pageErrors.length || result.responseObserverErrors?.length)) {
      lateGateFailure = new Error('The final response ledger contains a late or unclassified failure.');
      result.completed = false; result.failure = lateGateFailure.stack;
    }
  }
  result.finalFileSha256 = hash(await readOwnedFile());
  result.draftWrites = draftWrites.map(write => ({ sourceSha256: hash(Buffer.from(write.source)), sourceBytes: Buffer.byteLength(write.source), sessionId: write.sessionId, status: write.status, ok: write.ok }));
  result.saveWrites = saveWrites.map(write => ({ sourceSha256: hash(Buffer.from(write.source)), operationId: write.operationId, status: write.status, receiptStatus: write.receipt?.status }));
  await writeFile(path.join(output, 'server.stdout.log'), stdout.replace(/#token=\w+/g, '#token=<redacted>'));
  await writeFile(path.join(output, 'server.stderr.log'), stderr);
  await writeFile(path.join(output, 'result.json'), stringifyServeJson(result, 2));
  if (lateGateFailure) throw lateGateFailure;
}
process.stdout.write(stringifyServeJson(result, 2));
