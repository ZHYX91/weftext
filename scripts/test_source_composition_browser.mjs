// Isolated SourceEditor DOM/queue timing regression. Synthetic composition
// events and fixture tokens do not establish OS IME or official-parser support.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
if (!process.argv[2]) throw new Error('Usage: node scripts/test_source_composition_browser.mjs NEW_OWNED_OUTPUT_DIRECTORY');
const out = path.resolve(process.argv[2]);
await mkdir(out, { recursive: false });
const fixture = path.join(out, 'fixture');
await mkdir(fixture);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const sources = ['app/src/editor.ts', 'app/src/source-history.ts', 'app/src/source-style.ts'];
const inputHashes = Object.fromEntries(await Promise.all(sources.map(async name => [name, sha(await readFile(path.join(repo, name)))])));
const result = {
  scope: 'Actual current SourceEditor and CodeMirror; isolated Vite component fixture, headless Chrome',
  limits: ['Synthetic composition events only; no actual OS input method or candidate window.', 'Fixture heading tokens only; no official Typst parser or packaged-app acceptance.', 'No user library, document, browser tab or existing server is used.'],
  node: process.version, inputHashes, cases: [], pageErrors: [], requestFailures: [], completed: false,
};
const importSource = name => '/@fs/' + path.join(repo, name).replaceAll('\\', '/');
await writeFile(path.join(fixture, 'index.html'), '<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>Owned SourceEditor composition timing regression</title><style>body{margin:20px}#host{height:420px;border:1px solid #bbb}</style><div id="host"></div><script type="module" src="/fixture.js"></script></html>');
await writeFile(path.join(fixture, 'fixture.js'), `
import { SourceEditor } from ${JSON.stringify(importSource('app/src/editor.ts'))};
import { sourceStyles, sourceTokensEffect } from ${JSON.stringify(importSource('app/src/source-style.ts'))};

const original = '= 合成时序回归标题\\n\\n正文输入区：\\n';
const host = document.querySelector('#host');
let editor, sourceEvents, changes, updates, events;
const settle = () => new Promise(resolve => setTimeout(resolve, 100));
function headingTokens() {
  const to = original.indexOf('\\n');
  return [{ kind: 'Heading', from: 0, to, utf16From: 0, utf16To: to }];
}
function snapshot() {
  const value = editor.view.state.field(sourceStyles), classes = [];
  value.decorations.between(0, editor.source.length, (_from, _to, decoration) => classes.push(...String(decoration.spec.class || '').split(' ')));
  return { source: editor.source, generation: editor.generation, basis: editor.syntaxBasis(), status: value.status, classes, decorationCount: value.decorations.size,
    domSource: Array.from(editor.view.contentDOM.querySelectorAll('.cm-line')).map(line => line.textContent).join('\\n'),
    selection: editor.selection, composing: editor.composing, changes: [...changes], sourceEvents: [...sourceEvents], updates: [...updates], events: [...events] };
}
async function reset() {
  if (editor) editor.destroy();
  host.replaceChildren(); sourceEvents = []; changes = []; updates = []; events = [];
  editor = new SourceEditor(original, source => changes.push(source), () => {});
  editor.observe(event => {
    events.push({ kind: event.kind, source: event.source, generation: event.generation, composing: event.composing });
    if (event.kind === 'source') sourceEvents.push({ source: event.source, generation: event.generation });
  });
  // Transparently record the actual view's dispatch boundary. Importing a
  // second CodeMirror entry from a private fixture can create a different
  // facet instance than Vite's optimized dependency; no foreign facet or
  // parser/input mock is installed here.
  const dispatch = editor.view.dispatch.bind(editor.view);
  editor.view.dispatch = (...specs) => {
    const before = editor.view.state;
    const tokensInstalled = specs.some(spec => (Array.isArray(spec.effects) ? spec.effects : spec.effects ? [spec.effects] : []).some(effect => effect.is(sourceTokensEffect)));
    const returned = dispatch(...specs);
    const after = editor.view.state;
    updates.push({ docChanged: !before.doc.eq(after.doc), source: after.doc.toString(), generation: after.field(sourceStyles).generation, status: after.field(sourceStyles).status, tokensInstalled });
    return returned;
  };
  editor.mount(host); editor.view.focus();
  const at = original.indexOf('正文输入区：') + '正文输入区：'.length;
  editor.select(at, at);
  editor.tokens(headingTokens(), true, editor.syntaxBasis());
  await settle();
  sourceEvents.length = 0; changes.length = 0; updates.length = 0; events.length = 0;
  return snapshot();
}
function begin() { editor.view.contentDOM.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true, data: '' })); }
function end(data = '') { editor.view.contentDOM.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true, data })); }

window.fixture = {
  original, reset, snapshot,
  async finalCommit() {
    const before = await reset(), finalText = '今天测试中文输入法😀';
    begin();
    const queued = editor.tokens([], true, editor.syntaxBasis());
    const afterQueue = snapshot();
    // Modify the real CodeMirror-owned text node and native selection in the
    // same turn as compositionend. CodeMirror's own MutationObserver must read
    // these final DOM mutations before any queued style transaction runs.
    const line = editor.view.contentDOM.querySelectorAll('.cm-line')[2];
    const text = line.firstChild;
    if (!text || text.nodeType !== Node.TEXT_NODE) throw new Error('Expected the actual plain body Text node');
    text.nodeValue += finalText;
    const selection = window.getSelection(), range = document.createRange();
    range.setStart(text, text.nodeValue.length); range.collapse(true);
    selection.removeAllRanges(); selection.addRange(range);
    end(finalText);
    const sameTurn = snapshot();
    await settle();
    return { before, queued, afterQueue, sameTurn, finalText, after: snapshot() };
  },
  async cancelled() {
    const before = await reset(); begin();
    const queued = editor.tokens([], true, editor.syntaxBasis()), afterQueue = snapshot();
    end(); await settle();
    return { before, queued, afterQueue, after: snapshot() };
  },
  async retired() {
    const before = await reset(), oldBasis = editor.syntaxBasis(); begin();
    const queued = editor.tokens([], true, oldBasis);
    editor.clearTokens(); const afterRetire = snapshot();
    end(); await settle();
    const oldAccepted = editor.tokens(headingTokens(), true, oldBasis);
    return { before, queued, afterRetire, oldAccepted, after: snapshot() };
  },
  async aba() {
    const before = await reset(), oldBasis = editor.syntaxBasis(); begin();
    const queued = editor.tokens([], true, oldBasis), at = original.indexOf('正文输入区：') + '正文输入区：'.length;
    editor.apply(at, at, '临'); editor.apply(at, at + 1, '');
    const oldAccepted = editor.tokens([], true, oldBasis);
    end(); await settle();
    return { before, queued, oldAccepted, after: snapshot() };
  },
  destroy() { if (editor) { editor.destroy(); editor = undefined; } }
};
`);

let server, browser, page;
async function passed(name, evidence) {
  result.cases.push({ name, passed: true, evidence });
  await writeFile(path.join(out, 'progress.json'), JSON.stringify(result, null, 2));
  process.stdout.write(name + '\n');
}
function noInstalledOldTokens(after) { assert.equal(after.updates.filter(update => update.tokensInstalled).length, 0); }
try {
  const viteEntry = require.resolve('vite', { paths: [path.join(repo, 'app')] });
  const { createServer } = await import(pathToFileURL(viteEntry).href);
  server = await createServer({ configFile: false, root: fixture, cacheDir: path.join(out, 'vite-cache'), logLevel: 'error', server: { host: '127.0.0.1', port: 0, strictPort: true, fs: { allow: [repo, out] } } });
  await server.listen();
  result.url = 'http://127.0.0.1:' + server.httpServer.address().port + '/';
  browser = await chromium.launch({ headless: true, ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : { channel: 'chrome' }) });
  result.browser = browser.version();
  page = await browser.newPage({ viewport: { width: 1260, height: 800 } });
  page.on('pageerror', error => result.pageErrors.push(String(error)));
  page.on('requestfailed', request => result.requestFailures.push({ url: request.url(), failure: request.failure() }));
  await page.goto(result.url);
  await page.waitForFunction(() => !!window.fixture);
  const original = await page.evaluate(() => window.fixture.original);

  const commit = await page.evaluate(() => window.fixture.finalCommit());
  await writeFile(path.join(out, 'final-commit.json'), JSON.stringify(commit, null, 2));
  assert.equal(commit.queued, true);
  assert.equal(commit.afterQueue.composing, true);
  noInstalledOldTokens(commit.afterQueue);
  assert.equal(commit.sameTurn.source, original, 'The final real DOM mutation must still be pending when compositionend returns');
  const expected = original.replace('正文输入区：', '正文输入区：' + commit.finalText);
  assert.equal(commit.sameTurn.domSource, expected);
  assert.equal(commit.after.source, expected);
  assert.equal(commit.after.domSource, expected);
  assert.equal(commit.after.generation, 1);
  assert.deepEqual(commit.after.changes, [expected]);
  assert.deepEqual(commit.after.sourceEvents, [{ source: expected, generation: 1 }]);
  assert.equal(commit.after.status, 'pending');
  assert.ok(commit.after.classes.includes('cm-source-heading-layout'));
  assert.ok(!commit.after.classes.includes('cm-typst-heading'));
  noInstalledOldTokens(commit.after);
  await page.screenshot({ path: path.join(out, 'final-commit.png'), fullPage: true });
  await passed('Final real DOM commit reaches SourceEditor before queued styling; complete Chinese/emoji and one source event survive.', commit);

  const cancel = await page.evaluate(() => window.fixture.cancelled());
  await writeFile(path.join(out, 'cancelled.json'), JSON.stringify(cancel, null, 2));
  assert.equal(cancel.queued, true);
  assert.equal(cancel.afterQueue.decorationCount, cancel.before.decorationCount);
  noInstalledOldTokens(cancel.afterQueue);
  assert.equal(cancel.after.source, original);
  assert.equal(cancel.after.generation, 0);
  assert.deepEqual(cancel.after.sourceEvents, []);
  assert.deepEqual(cancel.after.changes, []);
  assert.equal(cancel.after.status, 'current');
  assert.equal(cancel.after.decorationCount, 0);
  assert.equal(cancel.after.updates.filter(update => update.tokensInstalled).length, 1);
  await passed('Cancelled composition without a document change installs its still-exact queued styling only after composition ends.', cancel);

  const retired = await page.evaluate(() => window.fixture.retired());
  await writeFile(path.join(out, 'retired.json'), JSON.stringify(retired, null, 2));
  assert.equal(retired.queued, true);
  assert.equal(retired.afterRetire.basis.epoch, retired.before.basis.epoch + 1);
  assert.equal(retired.oldAccepted, false);
  assert.equal(retired.after.source, original);
  assert.equal(retired.after.generation, 0);
  assert.equal(retired.after.status, 'none');
  assert.equal(retired.after.decorationCount, 0);
  noInstalledOldTokens(retired.after);
  await passed('Explicit retirement clears the composition queue and rejects the original epoch after composition ends.', retired);

  const aba = await page.evaluate(() => window.fixture.aba());
  await writeFile(path.join(out, 'aba.json'), JSON.stringify(aba, null, 2));
  assert.equal(aba.queued, true);
  assert.equal(aba.oldAccepted, false);
  assert.equal(aba.after.source, original);
  assert.equal(aba.after.generation, 2);
  assert.equal(aba.after.status, 'pending');
  assert.deepEqual(aba.after.sourceEvents.map(event => event.generation), [1, 2]);
  assert.equal(aba.after.changes.length, 2);
  assert.equal(aba.after.changes[1], original);
  assert.ok(!aba.after.classes.includes('cm-typst-heading'));
  noInstalledOldTokens(aba.after);
  await passed('Returning to equal source bytes does not resurrect the original composition styling queue or generation.', aba);

  assert.deepEqual(result.pageErrors, []);
  assert.deepEqual(result.requestFailures, []);
  result.finalInputHashes = Object.fromEntries(await Promise.all(sources.map(async name => [name, sha(await readFile(path.join(repo, name)))])));
  assert.deepEqual(result.finalInputHashes, inputHashes, 'Component sources changed during the isolated browser run');
  result.completed = true;
} catch (error) {
  result.error = String(error.stack || error);
  process.exitCode = 1;
} finally {
  if (page) await page.evaluate(() => window.fixture?.destroy()).catch(() => {});
  if (browser) await browser.close();
  if (server) await server.close();
  result.closed = { ownedBrowser: !browser || !browser.isConnected(), ownedVite: !server?.httpServer?.listening };
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2));
}
process.stdout.write(JSON.stringify({ completed: result.completed, cases: result.cases.length, closed: result.closed, error: result.error || null, result: path.join(out, 'result.json') }, null, 2) + '\n');
