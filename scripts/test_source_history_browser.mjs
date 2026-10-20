// Actual SourceEditor/CodeMirror DOM history regression, isolated from all
// libraries and backends. Synthetic composition is not real OS IME acceptance.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
if (!process.argv[2]) throw new Error('Usage: node scripts/test_source_history_browser.mjs NEW_OWNED_OUTPUT_DIRECTORY');
const out = path.resolve(process.argv[2]);
await mkdir(out, { recursive: false });
const fixture = path.join(out, 'fixture');
await mkdir(fixture);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const sourceFiles = ['app/src/editor.ts', 'app/src/source-history.ts', 'app/src/source-style.ts', 'app/src/writing.ts'];
const hashes = async () => Object.fromEntries(await Promise.all(sourceFiles.map(async name => [name, sha(await readFile(path.join(repo, name)))])));
const result = { scope: 'Current SourceEditor, WritingController and CodeMirror over real DOM; isolated Vite component',
  limits: ['Synthetic composition/beforeinput events only; no OS input method or candidate window.', 'No backend, library, save transaction or packaged-app acceptance.'],
  inputHashes: await hashes(), node: process.version, cases: [], pageErrors: [], requestFailures: [], completed: false };
const modulePath = name => '/@fs/' + path.join(repo, name).replaceAll('\\', '/');
await writeFile(path.join(fixture, 'index.html'), '<!doctype html><html lang="zh-CN"><meta charset="UTF-8"><title>Owned source history regression</title><div id="host" style="height:420px"></div><script type="module" src="/fixture.js"></script></html>');
await writeFile(path.join(fixture, 'fixture.js'), `
import { SourceEditor } from ${JSON.stringify(modulePath('app/src/editor.ts'))};
import { WritingController } from ${JSON.stringify(modulePath('app/src/writing.ts'))};
const base = '#metadata((id: "00000000-0000-0000-0000-000000000001", types: ())) <weftext>\\n\\n= Owned composition history\\n\\n正文输入区：保留正文\\n\\n#table(columns: 3, [普通格], table.cell(colspan: 2)[合并格], [])\\n';
let editor, controller, events, dispatches, outside, changes, requests;
const pause = () => new Promise(resolve => setTimeout(resolve, 100));
const sourceEvent = transaction => ['undo', 'redo', 'input.type.compose.start', 'input.type.compose', 'input.type', 'input.native', 'input'].find(name => transaction.isUserEvent?.(name));
function snapshot() { return { source: editor.source, domSource: Array.from(editor.view.contentDOM.querySelectorAll('.cm-line')).map(line => line.textContent).join('\\n'), generation: editor.generation, composing: editor.composing,
  events: [...events], dispatches: [...dispatches], outside: [...outside], changes: [...changes], requests }; }
async function reset() {
  if (controller) controller.dispose(); if (editor) editor.destroy();
  document.querySelector('#host').replaceChildren(); events = []; dispatches = []; outside = []; changes = []; requests = 0;
  editor = new SourceEditor(base, source => changes.push(source), () => {});
  const dispatch = editor.view.dispatch.bind(editor.view);
  editor.view.dispatch = (...specs) => { const returned = dispatch(...specs); dispatches.push({ userEvents: specs.map(spec => spec.userEvent ?? sourceEvent(spec) ?? null), source: editor.source, composing: editor.composing }); return returned; };
  editor.observe(event => events.push({ kind: event.kind, source: event.source, generation: event.generation, composing: event.composing }));
  controller = new WritingController({ editor, basis: () => ({ sessionId: 'owned-component-session', source: editor.source, serial: editor.generation }), editable: () => true,
    request: async () => { requests++; throw new Error('This source history regression must never call a backend'); } });
  editor.mount(document.querySelector('#host'));
  const at = base.indexOf('[合并格') + '[合并格'.length; editor.apply(at, at, '中文'); editor.view.focus();
  await pause(); events = []; dispatches = []; outside = []; changes = [];
  return snapshot();
}
function composition(type, data = '') { editor.view.contentDOM.dispatchEvent(new CompositionEvent(type, { bubbles: true, data })); }
function nativeHistory(inputType) { const event = new InputEvent('beforeinput', { bubbles: true, cancelable: true, inputType, isComposing: editor.composing }); editor.view.contentDOM.dispatchEvent(event); return event.defaultPrevented; }
function finalDom(insert = 'bu') {
  const marker = editor.source.includes('[合并格中文bu') ? '[合并格中文bu' : editor.source.includes('[合并格中文') ? '[合并格中文' : '[合并格';
  const start = editor.source.indexOf(marker); if (start < 0) throw new Error('Missing exact owned cell');
  const position = editor.view.domAtPos(start + marker.length), text = position.node;
  if (text.nodeType !== Node.TEXT_NODE) throw new Error('Expected the actual CodeMirror Text node at the exact source caret');
  text.nodeValue = text.nodeValue.slice(0, position.offset) + insert + text.nodeValue.slice(position.offset);
  const range = document.createRange(); range.setStart(text, position.offset + insert.length); range.collapse(true);
  const selection = window.getSelection(); selection.removeAllRanges(); selection.addRange(range);
}
window.fixture = {
  async run(kind) {
    const before = await reset();
    if (kind === 'ordinary') { const prevented = nativeHistory('historyUndo'), immediate = snapshot(); return { before, prevented, immediate, after: snapshot() }; }
    if (kind === 'readonly') {
      editor.readOnly(true); const prevented = nativeHistory('historyUndo'), afterNative = snapshot();
      // The public host command must still work for an authorized V04 owner
      // whose shared source surface itself is intentionally not editable.
      editor.undo(); return { before, prevented, afterNative, after: snapshot() };
    }
    if (kind === 'redo') { editor.undo(); await pause(); events = []; dispatches = []; changes = []; }
    const pre = snapshot(); composition('compositionstart');
    const prevented = (kind === 'after-typing' ? [] : kind === 'repeated' ? ['historyUndo', 'historyUndo', 'historyRedo'] : [kind === 'redo' ? 'historyRedo' : 'historyUndo']).map(nativeHistory);
    if (kind === 'retired') editor.clearTokens();
    if (kind === 'permission-retired') editor.readOnly(true);
    if (['commit', 'redo', 'repeated', 'retired', 'outside', 'after-typing'].includes(kind)) finalDom();
    const pending = snapshot();
    if (kind === 'outside') void controller.deferUntilCompositionEnds(() => { outside.push({ kind: 'mode', source: editor.source }); editor.readOnly(true); });
    composition('compositionend', kind === 'cancel' ? '' : 'bu');
    const immediate = snapshot();
    if (kind === 'outside') void controller.deferUntilCompositionEnds(() => { outside.push({ kind: 'save-basis', source: editor.source }); });
    if (kind === 'disposed') { controller.dispose(); editor.destroy(); }
    await pause();
    if (kind === 'after-typing') {
      const committed = snapshot();
      editor.view.contentDOM.dispatchEvent(new InputEvent('beforeinput', { bubbles: true, cancelable: true, inputType: 'insertText', data: 'x' }));
      finalDom('x');
      editor.view.contentDOM.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: 'x' }));
      await pause(); const typed = snapshot();
      editor.undo(); const afterFirst = snapshot(); editor.undo();
      return { before, pre, prevented, pending, immediate, committed, typed, afterFirst, after: snapshot() };
    }
    return { before, pre, prevented, pending, immediate, after: snapshot() };
  },
  destroy() { controller?.dispose(); editor?.destroy(); }
};
`);
let server, browser, page;
async function observe(kind) {
  const observed = await page.evaluate(kind => window.fixture.run(kind), kind);
  await writeFile(path.join(out, kind + '.json'), JSON.stringify(observed, null, 2));
  assert.equal(observed.after.requests, 0);
  return observed;
}
async function passed(name) { result.cases.push(name); await writeFile(path.join(out, 'progress.json'), JSON.stringify(result, null, 2)); process.stdout.write(name + '\n'); }
const historyEvents = snapshot => snapshot.dispatches.flatMap(item => item.userEvents).filter(event => event === 'undo' || event === 'redo');
function waiting(observed) {
  assert.ok(observed.prevented.every(Boolean));
  assert.equal(observed.immediate.source, observed.pre.source, 'Native history must not run before final composition DOM is read');
  assert.equal(observed.immediate.composing, true, 'Mode/save observers must keep waiting through the settlement task');
}
try {
  const { createServer } = await import(pathToFileURL(require.resolve('vite', { paths: [path.join(repo, 'app')] })).href);
  server = await createServer({ configFile: false, root: fixture, cacheDir: path.join(out, 'vite-cache'), logLevel: 'error', server: { host: '127.0.0.1', port: 0, strictPort: true, fs: { allow: [repo, out] } } });
  await server.listen();
  result.url = 'http://127.0.0.1:' + server.httpServer.address().port + '/';
  browser = await chromium.launch({ headless: true, ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : { channel: 'chrome' }) }); result.browser = browser.version();
  page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  page.on('pageerror', error => result.pageErrors.push(String(error))); page.on('requestfailed', request => result.requestFailures.push({ url: request.url(), failure: request.failure() }));
  await page.goto(result.url); await page.waitForFunction(() => !!window.fixture);
  const ordinary = await observe('ordinary');
  const golden = ordinary.before.source, withoutChinese = golden.replace('[合并格中文]', '[合并格]'), withBu = golden.replace('[合并格中文]', '[合并格中文bu]');
  assert.equal(ordinary.prevented, true); assert.equal(ordinary.immediate.source, withoutChinese); assert.deepEqual(historyEvents(ordinary.after), ['undo']);
  await passed('Ordinary native history remains synchronous and uses the shared CodeMirror Undo.');

  const commit = await observe('commit'); waiting(commit);
  assert.equal(commit.pending.source, golden); assert.equal(commit.pending.domSource, withBu);
  assert.equal(commit.after.source, golden); assert.equal(commit.after.domSource, golden);
  assert.deepEqual(commit.after.changes, [withBu, golden]); assert.deepEqual(historyEvents(commit.after), ['undo']);
  await passed('Final real DOM is committed before native Undo; only the new composition is undone and earlier cell Chinese survives.');

  const afterTyping = await observe('after-typing');
  assert.equal(afterTyping.committed.source, withBu); assert.equal(afterTyping.committed.composing, false);
  assert.equal(afterTyping.typed.source, withBu.replace('[合并格中文bu]', '[合并格中文bux]'));
  assert.equal(afterTyping.afterFirst.source, withBu); assert.equal(afterTyping.after.source, golden);
  assert.deepEqual(historyEvents(afterTyping.after), ['undo', 'undo']);
  await passed('Ordinary DOM typing immediately after a settled composition has its own Undo; the composition and earlier Chinese remain separate.');

  const cancel = await observe('cancel'); waiting(cancel);
  assert.equal(cancel.after.source, withoutChinese); assert.deepEqual(cancel.after.changes, [withoutChinese]);
  await passed('Cancelled composition creates no empty Undo unit and applies the queued original Undo only after settlement.');

  const redo = await observe('redo'); waiting(redo);
  assert.equal(redo.after.source, withoutChinese.replace('[合并格]', '[合并格bu]')); assert.deepEqual(historyEvents(redo.after), []);
  await passed('New composition safely invalidates an old Redo branch instead of overwriting pending input.');

  const repeated = await observe('repeated'); waiting(repeated);
  assert.equal(repeated.after.source, golden); assert.deepEqual(historyEvents(repeated.after), ['undo', 'undo', 'redo']);
  await passed('Repeated native Undo/Undo/Redo requests execute once each and in original order after the final DOM.');

  const outside = await observe('outside'); waiting(outside);
  assert.deepEqual(outside.immediate.outside, []); assert.equal(outside.after.source, golden);
  assert.deepEqual(outside.after.outside, [{ kind: 'mode', source: golden }, { kind: 'save-basis', source: golden }]);
  assert.deepEqual(historyEvents(outside.after), ['undo']);
  await passed('Actual WritingController mode and save-basis deferrals release after queued history, so switching mode cannot cancel the user Undo.');

  const retired = await observe('retired'); waiting(retired);
  assert.equal(retired.after.source, withBu); assert.deepEqual(historyEvents(retired.after), []);
  const permission = await observe('permission-retired'); waiting(permission);
  assert.equal(permission.after.source, golden); assert.deepEqual(historyEvents(permission.after), []);
  await passed('Source/session retirement and disabling the source surface cancel pending native history without reviving old requests.');

  const readonly = await observe('readonly'); assert.equal(readonly.prevented, true);
  assert.equal(readonly.afterNative.source, golden); assert.equal(readonly.after.source, withoutChinese);
  await passed('Noneditable source blocks native Undo while an explicitly authorized public host Undo still works for V04.');

  const disposed = await observe('disposed'); waiting(disposed);
  assert.equal(disposed.after.source, golden); assert.deepEqual(historyEvents(disposed.after), []);
  await passed('Destroying the owner cancels pending history and its scheduled settlement.');
  assert.deepEqual(result.pageErrors, []); assert.deepEqual(result.requestFailures, []);
  result.finalInputHashes = await hashes(); assert.deepEqual(result.finalInputHashes, result.inputHashes);
  result.completed = true;
} catch (error) { result.error = String(error.stack || error); process.exitCode = 1; }
finally {
  if (page) await page.evaluate(() => window.fixture?.destroy()).catch(() => {});
  if (browser) await browser.close(); if (server) await server.close();
  result.closed = { ownedBrowser: !browser || !browser.isConnected(), ownedVite: !server?.httpServer?.listening };
  await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2));
}
process.stdout.write(JSON.stringify({ completed: result.completed, cases: result.cases.length, closed: result.closed, error: result.error || null, result: path.join(out, 'result.json') }, null, 2) + '\n');
