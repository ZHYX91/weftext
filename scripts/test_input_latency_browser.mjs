// Trusted source-editor key events on an explicit embedded candidate.
// The second animation-frame receipt is a conservative feedback bound, not an
// OS keyboard/IME measurement. Chrome Layout tracing and JS heap have separate scopes.
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { access, mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const args = process.argv.slice(2), hosted = args.includes('--hosted'), smoke = args.includes('--smoke'), dev = args.includes('--dev'), collapsePreview = args.includes('--collapse-preview'), recoveryCheck = args.includes('--recovery-check'), fixtureArg = args.find(arg => arg.startsWith('--fixture='));
assert.equal(args.length, 2 + Number(hosted) + Number(smoke) + Number(dev) + Number(collapsePreview) + Number(recoveryCheck) + Number(!!fixtureArg), 'Usage: node scripts/test_input_latency_browser.mjs EXE NEW_OUTPUT [--hosted] [--dev] [--smoke [--fixture=ordinary|100k-lines|1MiB-line] [--collapse-preview] [--recovery-check]]');
assert.ok(!fixtureArg || smoke && ['ordinary', '100k-lines', '1MiB-line'].includes(fixtureArg.slice(10)), 'Fixture selection is diagnostic only');
assert.ok(!collapsePreview || smoke && !hosted, 'Collapsed preview is a local diagnostic only');
assert.ok(!recoveryCheck || smoke && hosted && fixtureArg === '--fixture=100k-lines', 'Recovery roundtrip requires hosted 100k-line smoke');
const exe = path.resolve(args[0]), out = path.resolve(args[1]);
const { chromium } = createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const events = smoke ? 20 : 1000, rounds = smoke ? 1 : 3;
const fixtures = smoke ? [fixtureArg?.slice(10) ?? 'ordinary'] : ['ordinary', '100k-lines', '1MiB-line'];
const selector = hosted ? '#source .cm-content' : '.source-host .cm-content';
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const result = {
  executable: exe, executableSha256: sha(await readFile(exe)), version: execFileSync(exe, ['--version'], { windowsHide: true }).toString().trim(),
  client: hosted ? 'controlled hosted writer' : 'local owner', frontend: dev ? 'current Vite workspace frontend with supplied backend; development integration only' : 'embedded supplied candidate', preview: collapsePreview ? 'collapsed for diagnostic isolation' : 'default product presentation', surface: 'V01 source editor', protocol: smoke ? 'diagnostic smoke; no full acceptance claim' : '1000 trusted key events; cold/warm, three runs per fixture',
  eventsPerCondition: events, rounds, fixtures, targets: { feedbackUpperBoundP95Ms: 50, maximumLayoutTaskMs: 100 },
  environment: { platform: process.platform, release: os.release(), arch: process.arch, cpus: os.cpus().length, cpu: os.cpus()[0]?.model, physicalMemoryBytes: os.totalmem(), node: process.version },
  scope: [
    'Trusted DOM keydown arrival to the second animation-frame receipt; both editor state and visible DOM are checked at that receipt. Transport/OS keyboard/IME latency is excluded.',
    'A fresh owned browser/backend for cold; reload in the same context/backend for warm. OS caches remain. The end caret is scrolled fully into both page/editor viewports before timing, without text warmup. Inputs are paced by completed feedback receipts.',
    'Chrome devtools.timeline tracing is enabled during measurement and may add overhead. Layout task duration is distinct from feedback latency.',
    'Renderer JS heap is diagnostic only; it excludes the browser/backend/workers and does not establish the whole-product steady-memory target.',
    'The 100k-line fixture approaches the current 2 MiB Core source limit. The 10 MiB desktop byte target is not met by this scope. No mobile/AT/real IME or reference parity claim.',
  ],
  measurements: [], pageErrors: [], pageCrashes: [], ownedRootProcesses: [], completed: false,
};
async function readOwnedOriginal(file) {
  assert.ok(path.resolve(file).startsWith(out + path.sep), 'Sharing-lock retry is limited to this owned fixture');
  for (let attempt = 0; attempt < 200; attempt++) {
    try { return await readFile(file, 'utf8'); }
    catch (error) {
      if (!['EBUSY', 'EACCES'].includes(error.code) || attempt === 199) throw error;
      (result.ownedOriginalReadRetries ??= []).push({ path: file, code: error.code, attempt: attempt + 1 });
      await pause(50);
    }
  }
}
await mkdir(out, { recursive: false });
const harness = await readFile(fileURLToPath(import.meta.url)); result.harnessSha256 = sha(harness);
await writeFile(path.join(out, 'executed-harness.mjs'), harness, { flag: 'wx' });
let child, browser, browserServer, page, base, frontend, vite, ownerToken, startupTrace, stderr = '';
async function bounded(promise, milliseconds, label) {
  let timer;
  try { return await Promise.race([promise, new Promise((_, reject) => { timer = setTimeout(() => reject(Error(label + ' timeout')), milliseconds); })]); }
  finally { clearTimeout(timer); }
}
function recordProcess(process, kind) {
  const entry = { kind, pid: process.pid, executable: process.spawnfile, arguments: process.spawnargs, observedSpawnAt: new Date().toISOString(), exitCode: null, signal: null }; result.ownedRootProcesses.push(entry);
  process.once('exit', (code, signal) => { entry.exitCode = code; entry.signal = signal; entry.exitObservedAt = new Date().toISOString(); });
  return entry;
}

async function start(data, extra = [], server = false) {
  child = spawn(exe, ['--no-browser', '--data-dir', data, ...extra], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  recordProcess(child, server ? 'hosted backend' : 'local backend');
  let stdout = ''; child.stdout.on('data', bytes => stdout += bytes); child.stderr.on('data', bytes => stderr += bytes);
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    const match = stdout.match(server ? /(http:\/\/127\.0\.0\.1:\d+)\/\r?\n/ : /(http:\/\/127\.0\.0\.1:\d+)\/#token=(\w+)/);
    if (match) { base = match[1]; ownerToken = match[2]; return; }
    if (child.exitCode !== null) throw Error(stderr); await pause(80);
  }
  throw Error('Owned backend startup timeout');
}
async function stop() {
  if (child?.exitCode === null && child.signalCode === null) { const ended = new Promise(resolve => child.once('exit', resolve)); child.kill(); await ended; }
  child = undefined;
}
async function api(route, body) {
  const response = await fetch(base + route, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Weftext-Token': ownerToken }, body: JSON.stringify(body), signal: AbortSignal.timeout(120000) });
  const value = await response.json(); assert.ok(response.ok, route + ': ' + JSON.stringify(value)); return value;
}
const click = name => page.getByRole('button', { name, exact: true }).click();
async function openPage(token, reload = false) {
  if (reload) await page.reload(); else await page.goto(frontend + (hosted ? '/hosted.html' : '/#token=' + ownerToken));
  if (hosted) {
    if (!reload) { await page.getByLabel('访问凭据', { exact: true }).fill(token); await click('连接'); }
    await page.getByRole('button', { name: '输入测量', exact: true }).waitFor(); await click('输入测量');
  }
  if (collapsePreview) { const collapse = page.getByRole('button', { name: '收起预览', exact: true }); if (await collapse.count()) await collapse.click(); }
  await page.locator(selector + '[contenteditable=true]').waitFor();
  await click(hosted ? 'V01 源码' : '源码');
}
async function readSource() { return page.locator(selector).evaluate(element => { const view = element.cmTile?.root?.view; assertView(view, element); return view.state.doc.toString(); function assertView(value, target) { if (value?.contentDOM !== target) throw Error('Actual CodeMirror editor missing'); } }); }
function summary(values) { const sorted = [...values].sort((a, b) => a - b); return { count: sorted.length, p50: sorted[Math.ceil(sorted.length * .5) - 1], p95: sorted[Math.ceil(sorted.length * .95) - 1], maximum: sorted.at(-1) }; }
async function traceEnd(cdp) {
  const complete = new Promise(resolve => cdp.once('Tracing.tracingComplete', resolve)); await bounded(cdp.send('Tracing.end'), 30000, 'Trace end RPC');
  const { stream } = await bounded(complete, 30000, 'Trace completion'); assert.ok(stream, 'Actual trace stream required'); let text = '';
  try { for (;;) { const block = await bounded(cdp.send('IO.read', { handle: stream, size: 1024 * 1024 }), 30000, 'Trace stream'); text += block.base64Encoded ? Buffer.from(block.data, 'base64').toString('utf8') : block.data; if (block.eof) break; } }
  finally { await cdp.send('IO.close', { handle: stream }); }
  return text;
}
async function measure(dir, original, phase) {
  await page.locator(selector).focus(); await page.keyboard.press('Control+End');
  assert.equal(await readSource(), original); await pause(100);
  const viewportSetup = await page.locator(selector).evaluate(element => {
    const view = element.cmTile?.root?.view; if (view?.contentDOM !== element) throw Error('Actual source editor missing');
    const position = view.domAtPos(view.state.doc.length), node = position.node.nodeType === Node.TEXT_NODE ? position.node.parentElement : position.node;
    node.closest('.cm-line').scrollIntoView({ block: 'center', inline: 'nearest' });
    view.scrollDOM.scrollTop = view.scrollDOM.scrollHeight;
    return { setup: 'Actual last line scrolled into page and editor; no text warmup', windowY: window.scrollY, editorY: view.scrollDOM.scrollTop };
  });
  await pause(1000);
  const visibleCaret = await page.locator(selector).evaluate(element => {
    const view = element.cmTile.root.view, caret = view.coordsAtPos(view.state.doc.length), viewport = view.scrollDOM.getBoundingClientRect();
    return { caret, viewportTop: viewport.top, viewportBottom: viewport.bottom, windowHeight: window.innerHeight, fullyVisible: !!caret && caret.top >= Math.max(0, viewport.top) && caret.bottom <= Math.min(window.innerHeight, viewport.bottom) };
  });
  assert.ok(visibleCaret.fullyVisible, 'Source caret must be fully visible before timing');
  const cdp = await page.context().newCDPSession(page); await cdp.send('Performance.enable');
  const metricsBefore = (await cdp.send('Performance.getMetrics')).metrics;
  await cdp.send('Tracing.start', { categories: 'devtools.timeline', transferMode: 'ReturnAsStream' });
  let tracing = true;
  const receipt = { phase, viewportSetup, visibleCaret, startedAt: new Date().toISOString(), sourceBytes: Buffer.byteLength(original), sourceUtf16: original.length, sourceLines: original.split('\n').length, sourceSha256: sha(original), eventsPlanned: events, inputSequenceDeadlineMs: 120000, feedbackReceiptDeadlineMs: 5000, completed: false };
  try {
    await page.bringToFront(); await page.locator(selector).focus();
    receipt.focusAtEntry = await page.locator(selector).evaluate(element => { const view = element.cmTile.root.view; return { active: element.contains(document.activeElement) || document.activeElement === element, documentFocused: document.hasFocus(), connected: element.isConnected, selectionHead: view.state.selection.main.head, length: view.state.doc.length }; });
    assert.ok(receipt.focusAtEntry.active && receipt.focusAtEntry.documentFocused && receipt.focusAtEntry.connected && receipt.focusAtEntry.selectionHead === original.length, 'Measured input must start on the focused end caret');
    await page.locator(selector).evaluate((element, expectedLength) => {
      const view = element.cmTile?.root?.view; if (view?.contentDOM !== element) throw Error('Actual CodeMirror editor missing');
      const state = { samples: [], pending: Promise.resolve(), originalLength: expectedLength, keysReceived: 0, documentKeys: [] }; window.__inputLatency = state;
      const documentListener = event => { if (event.isTrusted && ['x', 'Backspace'].includes(event.key)) { state.documentKeys.push({ key: event.key, target: event.target?.tagName, className: event.target?.className }); if (state.documentKeys.length > 16) state.documentKeys.shift(); } };
      document.addEventListener('keydown', documentListener, true);
      const listener = event => {
        if (!event.isTrusted || !['x', 'Backspace'].includes(event.key)) return;
        state.keysReceived++;
        const index = state.samples.length, expectedKey = index % 2 ? 'Backspace' : 'x';
        if (event.key !== expectedKey) throw Error('Unexpected key sequence');
        const start = event.timeStamp, entered = performance.now(), originalRect = element.getBoundingClientRect(), originalViewport = view.scrollDOM.getBoundingClientRect(), originalScroll = { windowY: window.scrollY, editorY: view.scrollDOM.scrollTop };
        state.pending = new Promise((resolve, reject) => requestAnimationFrame(() => {
          const firstFrame = performance.now(); requestAnimationFrame(() => {
            try {
              const doc = view.state.doc, rect = element.getBoundingClientRect();
              const expected = expectedLength + Number(event.key === 'x');
              if (doc.length !== expected || event.key === 'x' && doc.sliceString(doc.length - 1) !== 'x') throw Error('Draft feedback does not match trusted input');
              const dom = view.domAtPos(doc.length), connected = element.isConnected && dom.node.isConnected;
              const lastLine = doc.lineAt(doc.length), from = view.domAtPos(lastLine.from), visibleRange = document.createRange();
              visibleRange.setStart(from.node, from.offset); visibleRange.setEnd(dom.node, dom.offset);
              const visibleLastLine = visibleRange.toString();
              if (visibleLastLine !== lastLine.text) throw Error('Visible DOM text does not match Draft feedback');
              const focused = element.contains(document.activeElement) || document.activeElement === element;
              const viewport = view.scrollDOM.getBoundingClientRect(), caret = view.coordsAtPos(doc.length);
              state.lastObservation = { index, key: event.key, connected, focused, topBefore: originalRect.top, topAfter: rect.top, topDelta: rect.top - originalRect.top, viewportBefore: { top: originalViewport.top, bottom: originalViewport.bottom }, viewportAfter: { top: viewport.top, bottom: viewport.bottom }, scrollBefore: originalScroll, scrollAfter: { windowY: window.scrollY, editorY: view.scrollDOM.scrollTop }, caret, activeElement: document.activeElement?.outerHTML.slice(0, 300), selectionHead: view.state.selection.main.head, documentLength: doc.length, visibleLastLine };
              if (!connected || !focused || Math.abs(rect.top - originalRect.top) > 1) throw Error('Input surface/focus/vertical anchor changed: ' + JSON.stringify(state.lastObservation));
              const end = performance.now();
              state.samples.push({ index, key: event.key, trusted: true, handlerEntryMs: entered - start, firstFrameMs: firstFrame - start, feedbackUpperBoundMs: end - start, focused, connected, visibleLastLine, documentLength: doc.length }); resolve();
            } catch (error) { reject(error); }
          });
        }));
      };
      element.addEventListener('keydown', listener, true); state.remove = () => { element.removeEventListener('keydown', listener, true); document.removeEventListener('keydown', documentListener, true); };
    }, original.length);
    const started = performance.now();
    for (let index = 0; index < events; index++) {
      if (performance.now() - started > receipt.inputSequenceDeadlineMs) throw Error('120-second input-sequence deadline exceeded');
      if (index % 50 === 0) {
        const stopRequested = await access(path.join(out, 'stop.request')).then(() => true, error => { if (error.code === 'ENOENT') return false; throw error; });
        if (stopRequested) throw Error('Measurement stopped by its owner; no completed-condition claim');
      }
      await page.keyboard.press(index % 2 ? 'Backspace' : 'x'); await bounded(page.evaluate(async expected => { const state = window.__inputLatency; if (state.keysReceived !== expected) throw Error('Trusted event did not reach source editor: ' + JSON.stringify({ expected, received: state.keysReceived, documentKeys: state.documentKeys, activeElement: document.activeElement?.outerHTML.slice(0, 200) })); await state.pending; if (state.samples.length !== expected) throw Error('Visible feedback receipt missing'); }, index + 1), receipt.feedbackReceiptDeadlineMs, 'Visible feedback receipt');
      if ((index + 1) % 250 === 0) console.log(JSON.stringify({ phase, events: index + 1, planned: events }));
    }
    receipt.wallClockMs = performance.now() - started;
    receipt.samples = await page.evaluate(() => { const state = window.__inputLatency; state.remove(); return state.samples; });
    assert.equal(receipt.samples.length, events); assert.equal(await readSource(), original);
    const metricsAfter = (await cdp.send('Performance.getMetrics')).metrics;
    tracing = false; const trace = await traceEnd(cdp);
    await writeFile(path.join(dir, phase + '-trace.json'), trace, { flag: 'wx' }); receipt.traceSha256 = sha(trace);
    const layouts = JSON.parse(trace).traceEvents.filter(event => event.name === 'Layout' && event.ph === 'X' && Number.isFinite(event.dur)).map(event => event.dur / 1000);
    assert.ok(layouts.length, 'Actual browser Layout tasks must be observed');
    receipt.feedbackUpperBoundMs = summary(receipt.samples.map(sample => sample.feedbackUpperBoundMs)); receipt.layoutTaskMs = summary(layouts);
    receipt.metricsBefore = metricsBefore; receipt.metricsAfter = metricsAfter;
    receipt.rendererJsHeapUsedBytes = metricsAfter.find(metric => metric.name === 'JSHeapUsedSize')?.value;
    receipt.feedbackP95Met = receipt.feedbackUpperBoundMs.p95 <= result.targets.feedbackUpperBoundP95Ms;
    receipt.layoutMaximumMet = receipt.layoutTaskMs.maximum <= result.targets.maximumLayoutTaskMs;
    receipt.completed = true; receipt.finishedAt = new Date().toISOString(); return receipt;
  } finally {
    receipt.finalObservation = await page.evaluate(() => ({ samples: window.__inputLatency?.samples, lastObservation: window.__inputLatency?.lastObservation, keysReceived: window.__inputLatency?.keysReceived, documentKeys: window.__inputLatency?.documentKeys, activeElement: document.activeElement?.outerHTML.slice(0, 300) })).catch(error => ({ error: String(error) }));
    if (!receipt.completed) { await page.screenshot({ path: path.join(dir, phase + '-failure.png') }).catch(error => { receipt.screenshotError = String(error); }); }
    if (tracing) await traceEnd(cdp).then(trace => writeFile(path.join(dir, phase + '-incomplete-trace.json'), trace)).catch(error => { receipt.traceCleanupError = String(error); });
    await cdp.detach(); await writeFile(path.join(dir, phase + '-result.json'), JSON.stringify(receipt, null, 2));
  }
}
try {
  for (const fixture of fixtures) for (let round = 1; round <= rounds; round++) {
    const dir = path.join(out, fixture + '-' + round); await mkdir(dir); const root = path.join(dir, 'lib'), data = path.join(dir, 'state');
    await start(data); await api('/api/library/create', { path: root, operationId: randomUUID() });
    const note = await api('/api/note/create', { root, parent: root, name: 'Input', operationId: randomUUID() });
    const body = fixture === 'ordinary' ? '普通输入与草稿保持。\n'.repeat(160) : fixture === '100k-lines' ? '0123456789abcdefghi\n'.repeat(100000) : 'A'.repeat(1024 * 1024) + '\n';
    const original = note.source + '\n' + body;
    assert.ok(Buffer.byteLength(original) < 2 * 1024 * 1024);
    await api('/api/save', { sessionId: note.sessionId, baseId: note.baseId, source: original, operationId: randomUUID() }); await stop();
    const token = randomUUID(); let configPath;
    if (hosted) {
      const config = { deployment: 'input-latency', bind: '127.0.0.1:0', root, adminTokenSha256: sha(randomUUID()), files: { input: { path: path.relative(root, note.path).replaceAll('\\', '/'), title: '输入测量', editable: true } }, users: { writer: { enabled: true, tokenSha256: sha(token), grants: { input: { read: true, write: true } } } } };
      configPath = path.join(dir, 'server.json'); await writeFile(configPath, JSON.stringify(config));
    }
    await start(data, hosted ? ['--server-config', configPath] : ['--open', note.path, '--root', root], hosted);
    frontend = base;
    if (dev) {
      const app = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../app'), { createServer } = await import(pathToFileURL(path.join(app, 'node_modules/vite/dist/node/index.js')).href);
      const proxy = { target: base, changeOrigin: true, configure(instance) { instance.on('proxyReq', (request, incoming) => { if (incoming.headers.origin) request.setHeader('Origin', base); }); } };
      vite = await createServer({ root: app, configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false, watch: { ignored: ['**/*'] }, proxy: { '/api': proxy, '/fonts': proxy } } });
      await vite.listen(); frontend = 'http://127.0.0.1:' + vite.httpServer.address().port;
    }
    browserServer = await chromium.launchServer({ headless: true, ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : { channel: 'chrome' }) });
    recordProcess(browserServer.process(), 'owned Chrome');
    browser = await chromium.connect(browserServer.wsEndpoint());
    if (dev && smoke) { startupTrace = await browser.newBrowserCDPSession(); await startupTrace.send('Tracing.start', { categories: 'devtools.timeline,disabled-by-default-v8.cpu_profiler', transferMode: 'ReturnAsStream' }); }
    result.environment.chrome = browser.version(); const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
    page = await context.newPage(); page.setDefaultTimeout(30000); page.on('pageerror', error => result.pageErrors.push(String(error))); page.on('crash', () => result.pageCrashes.push({ fixture, round, at: new Date().toISOString() }));
    if (recoveryCheck) {
      result.recovery = { completed: false, scope: 'Large unsaved browser recovery with server draft delivery blocked; not a latency-target or real IME claim', blockedDraftRequests: 0 };
      await page.route('**/api/hosted/draft', route => { result.recovery.blockedDraftRequests++; return route.abort('internetdisconnected'); });
    }
    if (collapsePreview) await page.addInitScript(() => { document.addEventListener('DOMContentLoaded', () => { const style = document.createElement('style'); style.textContent = '.preview-pane { display: none !important; }'; document.head.append(style); }); });
    await openPage(token);
    if (startupTrace) { await writeFile(path.join(dir, 'startup-trace.json'), await traceEnd(startupTrace)); await startupTrace.detach(); startupTrace = undefined; }
    for (const phase of ['cold-client', 'warm-reload']) {
      if (phase === 'warm-reload') await openPage(token, true);
      const measurement = await measure(dir, original, phase); measurement.fixture = fixture; measurement.round = round;
      // The native preview worker may hold a Windows sharing lease throughout
      // the measurement. Verify exact original bytes after the owned backend
      // closes, rather than mistaking an active read lease for author changes.
      measurement.originalCheck = 'deferred until owned backend releases file leases';
      result.measurements.push(measurement); await writeFile(path.join(out, 'progress.json'), JSON.stringify(result, null, 2));
      console.log(JSON.stringify({ fixture, round, phase, p95Ms: measurement.feedbackUpperBoundMs.p95, maximumLayoutMs: measurement.layoutTaskMs.maximum, feedbackP95Met: measurement.feedbackP95Met, layoutMaximumMet: measurement.layoutMaximumMet }));
    }
    if (recoveryCheck) {
      const expected = original + '浏览器恢复😀';
      await page.locator(selector).focus(); await page.keyboard.press('Control+End'); await page.keyboard.insertText('浏览器恢复😀');
      assert.equal(await readSource(), expected);
      await pause(1200); assert.ok(result.recovery.blockedDraftRequests > 0, 'Server draft delivery must actually be blocked');
      result.recovery.storageBeforeReload = await page.evaluate(() => {
        const keys = Object.keys(sessionStorage), key = keys.find(key => key.startsWith('weftext-hosted-draft:'));
        return { metadataChars: sessionStorage.getItem(key)?.length, snapshots: keys.filter(key => key.startsWith('weftext-hosted-draft-text:')).map(key => ({ chars: sessionStorage.getItem(key)?.length })) };
      });
      assert.ok(result.recovery.storageBeforeReload.metadataChars < 4096);
      assert.equal(result.recovery.storageBeforeReload.snapshots.length, 1);
      await openPage(token, true);
      assert.equal(await readSource(), expected, 'Reload must recover the complete unsaved browser Draft');
      Object.assign(result.recovery, { sourceBytes: Buffer.byteLength(expected), sourceSha256: sha(expected), completeDraftAfterReload: true });
    }
    await browser.close(); browser = undefined; await browserServer.close(); browserServer = undefined; await vite?.close(); vite = undefined; await stop();
    assert.equal(await readOwnedOriginal(note.path), original);
    for (const measurement of result.measurements.filter(item => item.fixture === fixture && item.round === round)) measurement.diskUnchanged = true;
    if (recoveryCheck) {
      // This recovery-only case reloads an unsaved Draft and starts compilation.
      // Check the unchanged original once its owned backend releases file leases.
      Object.assign(result.recovery, { completed: true, diskUnchanged: true }); console.log(JSON.stringify(result.recovery));
    }
  }
  assert.equal(result.measurements.length, fixtures.length * rounds * 2); assert.deepEqual(result.pageErrors, []);
  result.completed = true; result.measuredSourceTargetsMet = result.measurements.every(item => item.feedbackP95Met && item.layoutMaximumMet);
  if (!result.measuredSourceTargetsMet && !smoke) process.exitCode = 1;
} catch (error) {
  result.failure = String(error.stack ?? error);
  if (page) {
    result.failureUi = await bounded(page.evaluate(selector => ({ url: location.href, sourceCount: document.querySelectorAll(selector).length, bodyText: document.body.innerText.slice(0, 5000), activeElement: document.activeElement?.outerHTML.slice(0, 300) }), selector), 5000, 'Failure UI receipt').catch(error => ({ error: String(error) }));
    await page.screenshot({ path: path.join(out, 'failure.png'), timeout: 5000 }).catch(error => { result.failureScreenshotError = String(error); });
  }
  throw error;
}
finally {
  result.cleanupErrors = [];
  if (startupTrace) { await page?.close({ runBeforeUnload: false }).catch(() => {}); await traceEnd(startupTrace).then(trace => writeFile(path.join(out, 'incomplete-startup-trace.json'), trace)).catch(error => { result.startupTraceError = String(error); }); await startupTrace.detach().catch(() => {}); }
  for (const close of [() => browser?.close(), () => browserServer?.close(), async () => { await vite?.close(); vite = undefined; }, () => stop()]) { try { await close(); } catch (error) { result.cleanupErrors.push(String(error)); } }
  result.ownedProcessesClosed = result.ownedRootProcesses.every(item => item.exitObservedAt) && result.cleanupErrors.length === 0 && !vite;
  if (!result.ownedProcessesClosed) process.exitCode = 1;
  await writeFile(path.join(out, 'server.stderr.log'), stderr); await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2));
}
