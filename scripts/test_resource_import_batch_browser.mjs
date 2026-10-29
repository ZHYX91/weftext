// Real headless DOM/SourceEditor checks with a controlled resource-operation
// ledger. These are component checks, not real Core, native host or OS IME QA.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
import { createServer as createPortServer } from 'node:net';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

assert.equal(process.argv.length, 3, 'Usage: node scripts/test_resource_import_batch_browser.mjs ABSOLUTE_NEW_OUTPUT');
assert.ok(path.isAbsolute(process.argv[2]), 'Use an absolute, new private output directory');
const out = path.resolve(process.argv[2]), repo = path.resolve(import.meta.dirname, '..');
assert.ok(path.relative(repo, out).startsWith('..' + path.sep), 'Keep private test evidence outside the public repository');
await mkdir(out, { recursive: false });
const require = createRequire(import.meta.url), { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
const vite = await createServer({ root: path.join(repo, 'app'), configFile: false, server: { host: '127.0.0.1', port: 0, strictPort: true, hmr: false },
  plugins: [{ name: 'resource-import-component', configureServer(server) {
    server.middlewares.use('/__resource_import_test__', (_request, response) => {
      response.setHeader('Content-Type', 'text/html');
      response.end('<!doctype html><meta charset="utf-8"><main><h1>Owned resource component</h1><div id="editor"></div></main>');
    });
  } }] });
const result = { cases: [], pageErrors: [], completed: false, ownedProcessesClosed: false,
  limits: ['Current-source headless Chrome with real DOM, SourceEditor/Undo and localStorage over controlled DTO ports; no real Core, filesystem import, native host, OS IME or authorization acceptance.'] };
let browser, page, base;
const pass = name => result.cases.push(name);
const control = (name, target = page) => target.locator(`.resource-import-batch [data-resource-import-control="${name}"]`);
const dialog = (target = page) => target.locator('.resource-import-batch');
const idle = (target = page) => target.waitForFunction(() => !document.querySelector('.resource-import-batch') || document.querySelector('.resource-import-batch').getAttribute('aria-busy') === 'false');
const calls = (route, target = page) => target.evaluate(route => window.resourceFixture.requests.filter(item => item.route === route), route);
const currentLocator = (target = page) => target.evaluate(() => {
  const f = window.resourceFixture, raw = localStorage.getItem(f.key);
  return raw === null ? null : { raw, value: JSON.parse(raw) };
});

async function install(options = {}, target = page) {
  await target.evaluate(async options => {
    const [{ ResourceImportBatch }, state, { SourceEditor }, { writingRange }, { DocumentSession }] = await Promise.all([
      import('/src/resource-import-batch.ts'), import('/src/resource-import-batch-state.ts'), import('/src/editor.ts'), import('/src/writing.ts'), import('/src/session.ts'),
    ]);
    if (options.clear !== false) localStorage.clear();
    const root = 'D:/Owned/ResourceBatch', source = 'Owned author 😀\n#endnotes()';
    const f = window.resourceFixture = { original: source, requests: options.restore?.requests ?? [], storageEvents: [], notifications: [], applies: [], applyAttempts: 0, changed: 0, acknowledgements: [], prepareActions: 0,
      editable: options.editable !== false, storageFault: options.storageFault ?? null, afterWrite: false,
      losePrepare: !!options.losePrepare, prepareMissing: !!options.prepareMissing, changedFail: !!options.changedFail, statusFail: !!options.statusFail,
      ackFail: false, rejectCommit: !!options.rejectCommit, hold: null, held: [], heldDigests: [], opensResolved: 0, openErrors: [], operations: new Map(options.restore?.operations ?? []),
      basis: { inputId: 'owned-input', sessionId: options.sessionId ?? 'owned-session', source, serial: 7, baseId: 'owned-base', inputRevision: 11,
        path: options.path ?? root + '/Note/Note.typ', root, workspaceId: '11111111-1111-4111-8111-111111111111', nodeId: '22222222-2222-4222-8222-222222222222' } };
    f.key = state.localResourceKey(state.resourceOwner(f.basis));
    f.session = options.ackPreparation ? new DocumentSession({ sessionId: f.basis.sessionId, source, baseId: f.basis.baseId, path: f.basis.path, root,
      kind: 'node', nodeId: f.basis.nodeId, workspaceId: f.basis.workspaceId, viewInputId: f.basis.inputId }) : null;
    f.ackSession = reason => {
      if (!f.session) throw Error('The real DocumentSession is required for ACK checks');
      const session = f.session, revision = session.issueDraftRevision();
      session.acknowledgeDraft(session.source, session.serial, revision, session.inputId);
      f.acknowledgements.push({ reason, revision: session.acknowledgedInputRevision, source: session.source, inputId: session.inputId, serial: session.serial, baseId: session.baseId, path: session.path });
      return session.acknowledgedInputRevision;
    };
    if (f.session) f.ackSession('initial');
    const editor = f.editor = new SourceEditor(source, (next, inputId) => {
      f.basis = { ...f.basis, source: next, inputId, serial: f.basis.serial + 1, inputRevision: f.basis.inputRevision + 1 };
      f.session?.edit(next, inputId);
    }, () => {}, undefined, f.session?.inputId ?? f.basis.inputId);
    f.currentBasis = () => f.session ? { ...f.basis, inputId: f.session.inputId, sessionId: f.session.opened.sessionId, source: f.session.source,
      serial: f.session.serial, baseId: f.session.baseId, path: f.session.path, inputRevision: f.session.acknowledgedInputRevision ?? undefined }
      : { ...f.basis, source: editor.source };
    document.querySelector('#editor').replaceChildren(editor.view.dom);
    f.files = [new File([new Uint8Array([137, 80, 78, 71])], 'original.png', { type: 'image/png' }), new File(['owned-text'], 'original.txt', { type: 'text/plain' })];
    const storage = {
      getItem(key) {
        f.storageEvents.push({ kind: 'get', key });
        if (f.storageFault === 'read') throw Error('Owned storage read failure');
        if (f.storageFault === 'readback' && f.afterWrite) { f.afterWrite = false; return 'Owned mismatching readback'; }
        return localStorage.getItem(key);
      },
      setItem(key, value) {
        f.storageEvents.push({ kind: 'set', key, value });
        if (f.storageFault === 'write') throw Error('Owned storage write failure');
        localStorage.setItem(key, value); f.afterWrite = true;
      },
      removeItem(key) {
        f.storageEvents.push({ kind: 'remove', key });
        if (f.storageFault === 'delete') throw Error('Owned storage delete failure');
        localStorage.removeItem(key);
      },
    };
    const nativeMap = async (text, at = text.indexOf('#endnotes()')) => {
      if (at < 0) at = text.length;
      const sourceHash = await state.resourceDigest(text), absent = () => ({ status: 'absent', text: null, range: null, construct: null, encoding: null });
      return { sourceHash, syntaxValid: true, literalRegions: [{ range: writingRange(text, 0, text.length), text, role: 'body', paragraphBreaks: [] }], structures: [], title: absent(), subtitle: absent(),
        bodyInsertion: writingRange(text, at, at), unavailableReason: null, notes: { sourceHash, syntaxValid: true, entries: [] },
        headingNumbering: { sourceHash, settings: null, editable: true, frozenEndnotesRequired: false, reason: null }, bodyBlocks: [] };
    };
    const request = async (route, body) => {
      const recorded = { route, body: structuredClone(body), locatorBeforeRequest: localStorage.getItem(f.key), basisAtRequest: structuredClone(f.currentBasis()) };
      f.requests.push(recorded);
      let answer;
      if (route === '/api/resources/prepare-import-batch') {
        if (f.prepareMissing) throw Error('Owned prepare did not enter Core ledger');
        const imports = body.files.map((file, index) => ({ name: file.name, resourceRef: `weftext-resource://${f.basis.workspaceId}/33333333-3333-4333-8333-${String(index + 1).padStart(12, '0')}`, exactRevision: 'owned-revision-' + index }));
        answer = { operation: { operationId: body.operationId, planId: 'owned-plan:' + body.operationId, kind: 'importResources', status: 'prepared',
          changes: [...body.files.map(file => ({ path: root + '/Note/' + file.name, action: 'import', bytes: atob(file.bytes).length })), { path: root + '/Note/weftext.resources.json', action: 'import', bytes: 320 }],
          nodeMap: { [f.basis.nodeId]: f.basis.nodeId }, newNodeId: null, revisions: {}, resourceMap: {}, losses: [] }, imports };
        f.operations.set(body.operationId, structuredClone(answer));
        if (f.losePrepare) { f.losePrepare = false; throw Error('Owned lost prepare response'); }
      } else if (route === '/api/operations/status') {
        if (f.statusFail) throw Error('Owned original status temporarily unavailable');
        answer = { operationId: body.operationId, status: f.operations.get(body.operationId)?.operation.status ?? 'unknown' };
      }
      else if (route === '/api/operations/settle') answer = { operationId: body.operationId, status: 'resolved' };
      else if (route === '/api/resources/import-batch/status') {
        answer = structuredClone(f.operations.get(body.operationId));
        if (!answer) throw Error('Owned missing import-batch operation');
      } else if (route === '/api/operations/commit') {
        if (f.rejectCommit) throw Error('Owned Core rejects changed resource path');
        const operation = f.operations.get(body.operationId);
        if (!operation || operation.operation.planId !== body.planId || operation.operation.status !== 'prepared') throw Error('Unqualified or repeated original commit');
        operation.operation.status = 'committed'; answer = { operationId: body.operationId, status: 'committed' };
      } else if (route === '/api/recovery/resolve') {
        if (body.action !== 'keepCurrent') throw Error('Unexpected recovery effect');
        f.operations.get(body.operationId).operation.status = 'resolved'; answer = { operationId: body.operationId, status: 'resolved' };
      } else if (route === '/api/editor-map') answer = { ...await nativeMap(body.source), serial: body.serial };
      else if (route === '/api/resources/import-batch/insert') {
        const operation = f.operations.get(body.operationId), map = await nativeMap(body.source), at = map.bodyInsertion.utf16From;
        const insert = '\n' + operation.imports.map(item => body.mode === 'auto' && item.name.endsWith('.png') ? `#image("${item.resourceRef}")` : `#link("${item.resourceRef}")[${item.name}]`).join('\n') + '\n';
        const next = body.source.slice(0, at) + insert + body.source.slice(at), nextMap = await nativeMap(next);
        answer = { operationId: body.operationId, imports: structuredClone(operation.imports), edit: { source: next, sourceHash: nextMap.sourceHash, serial: body.serial,
          patches: [{ from: map.bodyInsertion.from, to: map.bodyInsertion.to, utf16From: at, utf16To: at, expected: '', insert }], selection: { from: at + insert.length, to: at + insert.length }, map: nextMap } };
      } else throw Error('Unexpected resource component route ' + route);
      if (f.hold === route) await new Promise(resolve => f.held.push({ route, release: resolve }));
      return structuredClone(answer);
    };
    const host = {
      request, getDocument: f.currentBasis, canEdit: () => f.editable,
      applyPatch: async (patch, basis) => {
        f.applyAttempts++;
        const current = f.currentBasis(), sourceCutFields = ['inputId', 'sessionId', 'source', 'serial', 'baseId', 'path', 'root', 'workspaceId', 'nodeId'];
        if (!f.editable || sourceCutFields.some(key => current[key] !== basis[key])) throw Error('Harness refused stale source delivery');
        f.applies.push({ patch: structuredClone(patch), basis: structuredClone(basis) });
        editor.applyPatches(basis.source, [{ from: patch.from, to: patch.to, expected: basis.source.slice(patch.from, patch.to), insert: patch.text }], { from: patch.from + patch.text.length, to: patch.from + patch.text.length });
        if (f.ackFail) throw Error('Owned editor changed before persistence ACK rejection');
      },
      prepareAction: async () => { f.prepareActions++; if (f.session) f.ackSession('prepareAction'); }, changed: async () => { f.changed++; if (f.changedFail) throw Error('Owned resource environment refresh failure'); },
      notify: (text, error) => f.notifications.push({ text, error }), run: async operation => operation(), storage,
    };
    f.controller = new ResourceImportBatch(host);
    f.open = (files = f.files) => { f.openPromise = f.controller.open(files, f.basis.sessionId).then(() => { f.opensResolved++; }, error => { f.openErrors.push(String(error)); }); };
    f.release = () => { f.hold = null; for (const item of f.held.splice(0)) item.release(); };
    // Hold only the real digest result for this known fixture wire. The native
    // digest still runs, and its original descriptor is restored before waiting.
    f.holdOriginalWireDigest = () => {
      const expected = JSON.stringify(f.requests.find(item => item.route === '/api/resources/prepare-import-batch').body);
      const subtle = crypto.subtle, original = subtle.digest, descriptor = Object.getOwnPropertyDescriptor(subtle, 'digest');
      Object.defineProperty(subtle, 'digest', { configurable: true, writable: true, value: function (algorithm, bytes) {
        const view = ArrayBuffer.isView(bytes) ? new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength) : new Uint8Array(bytes);
        const pending = original.call(subtle, algorithm, bytes);
        if (new TextDecoder().decode(view) !== expected) return pending;
        if (descriptor) Object.defineProperty(subtle, 'digest', descriptor); else delete subtle.digest;
        return pending.then(answer => new Promise(resolve => f.heldDigests.push({ release: () => resolve(answer) })));
      } });
    };
    f.mutate = kind => {
      if (kind === 'source') editor.applyPatches(editor.source, [{ from: 0, to: 0, expected: '', insert: 'Later author input\n' }]);
      else if (kind === 'inputABA') {
        const original = editor.source;
        editor.applyPatches(original, [{ from: 0, to: 0, expected: '', insert: 'Temporary writer\n' }]);
        editor.undo();
        if (editor.source !== original) throw Error('The real SourceEditor Undo did not complete the intended ABA');
      }
      else if (kind === 'inputId' && f.session) f.session.edit(f.session.source, editor.markAuthorInput());
      else if (kind === 'baseId' && f.session) f.session.acknowledgeSave(f.session.source, f.session.baseId + '-changed', f.session.path);
      else if (kind === 'permission') f.editable = false;
      else if (kind === 'owner') f.basis = { ...f.basis, nodeId: '44444444-4444-4444-8444-444444444444', path: root + '/Other/Other.typ', sessionId: 'other-session' };
      else { const value = f.basis[kind]; f.basis = { ...f.basis, [kind]: typeof value === 'number' ? value + 1 : value + '-changed' }; }
    };
  }, options);
}

async function reset(options = {}, target = page) {
  await target.goto(base + '/__resource_import_test__');
  await install(options, target);
}
async function open(mode = 'importOnly', target = page) {
  await target.evaluate(() => { window.resourceFixture.open(); });
  await dialog(target).waitFor({ state: 'visible' });
  if (await control('mode', target).isEnabled()) await control('mode', target).selectOption(mode);
}
async function prepare(mode = 'importOnly', options = {}) {
  await reset(options); await open(mode);
  await control('prepare').click(); await idle();
}
async function snapshot(target = page) {
  return target.evaluate(() => {
    const f = window.resourceFixture;
    return { source: f.editor.source, original: f.original, applies: f.applies.length, applyAttempts: f.applyAttempts, applyBases: f.applies.map(item => item.basis), requests: f.requests, changed: f.changed,
      operations: [...f.operations], raw: localStorage.getItem(f.key), pending: f.controller.hasPending(), notices: f.notifications,
      acknowledgements: f.acknowledgements, prepareActions: f.prepareActions, currentBasis: f.currentBasis() };
  });
}
async function holdRealStorageLock(target) {
  await target.evaluate(() => {
    if (!navigator.locks?.request || !navigator.locks?.query) throw Error('The real Web Locks API is required for the multi-page regression');
    const f = window.resourceFixture;
    const gate = f.lockGate = { name: `weftext-resource-import:${f.key}`, held: false };
    gate.done = navigator.locks.request(gate.name, { mode: 'exclusive' }, async () => {
      gate.held = true;
      await new Promise(resolve => { gate.release = resolve; });
    });
  });
  await target.waitForFunction(() => window.resourceFixture.lockGate?.held === true);
}
async function waitRealStorageContenders(target, count) {
  await target.evaluate(count => {
    const gate = window.resourceFixture.lockGate;
    gate.queryProbe?.stop();
    const probe = gate.queryProbe = { ready: false, stopped: false, snapshot: null, lastSnapshot: null, error: null, frame: null };
    probe.stop = () => { probe.stopped = true; if (probe.frame !== null) cancelAnimationFrame(probe.frame); };
    const query = async () => {
      if (probe.stopped) return;
      try {
        const locks = await navigator.locks.query();
        if (probe.stopped) return;
        const snapshot = { name: gate.name, held: locks.held.filter(lock => lock.name === gate.name), pending: locks.pending.filter(lock => lock.name === gate.name && lock.mode === 'exclusive') };
        probe.lastSnapshot = snapshot;
        if (snapshot.pending.length >= count) { probe.snapshot = snapshot; probe.ready = true; probe.stop(); return; }
      } catch (error) { probe.error = String(error); probe.stop(); return; }
      probe.frame = requestAnimationFrame(() => { void query(); });
    };
    void query();
  }, count);
  // This predicate deliberately returns a plain boolean. The installed browser
  // runner does not poll the resolution of an async predicate's Promise.
  try { await target.waitForFunction(() => window.resourceFixture.lockGate?.queryProbe?.ready === true); }
  catch (error) {
    result.crossTabWaitFailure = await target.evaluate(() => {
      const probe = window.resourceFixture.lockGate?.queryProbe;
      return probe ? { ready: probe.ready, stopped: probe.stopped, lastSnapshot: probe.lastSnapshot, error: probe.error } : null;
    }).catch(collectionError => ({ collectionFailure: String(collectionError) }));
    throw error;
  }
  const snapshot = await target.evaluate(() => {
    const probe = window.resourceFixture.lockGate?.queryProbe;
    if (!probe?.ready || !probe.snapshot) throw Error('The real lock probe has no qualifying snapshot');
    return probe.snapshot;
  });
  assert.ok(snapshot.pending.length >= count, 'The same actual snapshot must contain every required queued contender');
  assert.ok(snapshot.pending.every(lock => typeof lock.clientId === 'string' && lock.clientId.length > 0), 'Real queued locks must identify their browser clients');
  assert.ok(new Set(snapshot.pending.map(lock => lock.clientId)).size >= count, 'Queued contenders must come from distinct real page clients');
  return snapshot;
}
async function releaseRealStorageLock(target) {
  await target.evaluate(() => { const gate = window.resourceFixture.lockGate; gate?.queryProbe?.stop(); gate?.release?.(); });
  await target.evaluate(() => window.resourceFixture.lockGate?.done);
}
async function assignedLoopbackPort() {
  const listener = createPortServer();
  let port;
  try {
    await new Promise((resolve, reject) => { listener.once('error', reject); listener.listen({ host: '127.0.0.1', port: 0, exclusive: true }, resolve); });
    const address = listener.address();
    assert.ok(address && typeof address === 'object' && address.port > 0, 'The OS did not assign a loopback port');
    port = address.port;
  } finally {
    if (listener.listening) await new Promise((resolve, reject) => listener.close(error => error ? reject(error) : resolve()));
  }
  return port;
}
async function checkRealSessionAcknowledgements() {
  const sourceCutFields = ['inputId', 'sessionId', 'source', 'serial', 'baseId', 'path', 'root', 'workspaceId', 'nodeId'];
  await prepare('autoAppend', { ackPreparation: true });
  await control('confirm').click(); await idle();
  let actual = await snapshot();
  assert.equal(actual.applyAttempts, 1, 'Repeated prepareAction ACKs on the same real DocumentSession source cut must not retire automatic insertion');
  assert.equal(actual.applies, 1); assert.equal((await calls('/api/editor-map')).length, 1); assert.equal((await calls('/api/resources/import-batch/insert')).length, 1);
  const confirmation = (await calls('/api/operations/commit'))[0].basisAtRequest, mapped = (await calls('/api/editor-map'))[0].basisAtRequest;
  assert.ok(Number.isSafeInteger(confirmation.inputRevision), 'The real Session must still expose its current acknowledged revision');
  assert.equal(mapped.inputRevision, confirmation.inputRevision + 1, 'Insertion prepareAction must really advance the acknowledged revision');
  assert.deepEqual(sourceCutFields.map(key => mapped[key]), sourceCutFields.map(key => confirmation[key]), 'An ACK-only advance must preserve every source-cut field');
  assert.equal(Object.hasOwn(actual.applyBases[0], 'inputRevision'), false, 'The delivered insertion cut must omit its transport ACK field');
  assert.deepEqual(sourceCutFields.map(key => actual.applyBases[0][key]), sourceCutFields.map(key => confirmation[key]), 'The delivered source cut must remain the captured confirmation source and writer');
  assert.ok(actual.acknowledgements.length >= 3); assert.equal(actual.source.endsWith('#endnotes()'), true);
  assert.equal((await currentLocator()).value.insertion, 'inserted');
  await page.evaluate(() => window.resourceFixture.editor.undo()); assert.equal((await snapshot()).source, actual.original);
  result.realSessionAckChecks = [{ stage: 'automatic insertion preparation', confirmationRevision: confirmation.inputRevision, mapRevision: mapped.inputRevision, applyAttempts: actual.applyAttempts }];
  pass('Real DocumentSession prepareAction acknowledgements advance N→N+1 on the identical nine-field source cut while one automatic insertion and one Undo preserve the complete author source.');

  for (const stage of ['/api/editor-map', '/api/resources/import-batch/insert']) {
    await prepare('autoAppend', { ackPreparation: true });
    await page.evaluate(stage => { window.resourceFixture.hold = stage; }, stage); await control('confirm').click();
    await page.waitForFunction(() => window.resourceFixture.held.length === 1);
    const before = (await snapshot()).currentBasis;
    await page.evaluate(() => { const f = window.resourceFixture; f.ackSession('await first ACK'); f.ackSession('await second ACK'); f.release(); });
    await idle(); actual = await snapshot();
    const acknowledged = actual.acknowledgements.at(-1);
    assert.equal(acknowledged.revision, before.inputRevision + 2);
    assert.deepEqual(['source', 'serial', 'inputId', 'baseId', 'path'].map(key => acknowledged[key]), ['source', 'serial', 'inputId', 'baseId', 'path'].map(key => before[key]));
    assert.equal(actual.applyAttempts, 1, `${stage}: same-cut ACK-only refreshes must not retire an awaited insertion`); assert.equal(actual.applies, 1);
    assert.equal(Object.hasOwn(actual.applyBases[0], 'inputRevision'), false);
    assert.deepEqual(sourceCutFields.map(key => actual.applyBases[0][key]), sourceCutFields.map(key => before[key]));
    assert.equal((await calls('/api/editor-map')).length, 1); assert.equal((await calls('/api/resources/import-batch/insert')).length, 1);
    await page.evaluate(() => window.resourceFixture.editor.undo()); assert.equal((await snapshot()).source, actual.original);
    result.realSessionAckChecks.push({ stage, beforeRevision: before.inputRevision, afterRevision: acknowledged.revision, applyAttempts: actual.applyAttempts });
  }
  pass('Two real same-source DocumentSession ACKs during either editor-map or insertion await preserve one qualified proposal delivery and a complete single Undo.');

  for (const stage of ['/api/editor-map', '/api/resources/import-batch/insert']) for (const mutation of ['inputId', 'inputABA', 'source', 'baseId', 'permission']) {
    await prepare('linksAppend', { ackPreparation: true });
    await page.evaluate(stage => { window.resourceFixture.hold = stage; }, stage); await control('confirm').click();
    await page.waitForFunction(() => window.resourceFixture.held.length === 1);
    const before = (await snapshot()).currentBasis;
    await page.evaluate(mutation => { const f = window.resourceFixture; f.mutate(mutation); f.ackSession('changed source-cut ACK'); f.release(); }, mutation);
    await idle(); actual = await snapshot();
    assert.equal(actual.applyAttempts, 0, `${stage}/${mutation}: the controller delivered a changed real Session cut`); assert.equal(actual.applies, 0);
    assert.equal((await calls('/api/resources/prepare-import-batch')).length, 1); assert.ok(actual.raw);
    if (mutation === 'inputId') assert.notEqual(actual.currentBasis.inputId, before.inputId);
    if (mutation === 'inputABA') { assert.equal(actual.source, actual.original); assert.ok(actual.currentBasis.serial > before.serial); }
    else if (mutation === 'source') assert.equal(actual.source, 'Later author input\n' + actual.original);
    else assert.equal(actual.source, actual.original);
    if (mutation === 'baseId') assert.notEqual(actual.currentBasis.baseId, before.baseId);
  }
  pass('Real Session new writers, same-bytes writer ABA, later source, changed save base and lost permission still reject both awaited delivery stages before any apply attempt.');
}

try {
  const assignedPort = await assignedLoopbackPort();
  result.frontendPort = { requestedPort: 0, assignedPort, strict: true };
  // Vite's current listen(0) treats zero as its default-port request. Pass the
  // OS-assigned nonzero port through Vite's official initialization instead;
  // strictPort refuses an intervening owner without choosing another port.
  await vite.listen(assignedPort);
  const actualPort = vite.httpServer.address().port;
  result.frontendPort.actualPort = actualPort;
  assert.equal(actualPort, assignedPort);
  base = 'http://127.0.0.1:' + actualPort;
  browser = await chromium.launch({ headless: true, ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : { channel: 'chrome' }) });
  result.browserVersion = browser.version();
  const context = await browser.newContext({ viewport: { width: 1100, height: 900 } });
  page = await context.newPage(); page.setDefaultTimeout(15000);
  page.on('pageerror', error => result.pageErrors.push(String(error.stack || error)));

  await checkRealSessionAcknowledgements();

  await reset(); await open('linksAppend');
  await page.locator('[data-resource-import-name="0"]').fill('renamed-image.png');
  await page.locator('[data-resource-import-name="1"]').fill('renamed-document.txt');
  assert.deepEqual(await dialog().locator('tbody tr td:nth-child(2)').allTextContents(), ['original.png', 'original.txt']);
  assert.ok((await dialog().locator('[data-resource-import-total]').textContent()).includes('14'));
  assert.equal((await calls('/api/resources/prepare-import-batch')).length, 0);
  await control('prepare').click(); await idle();
  const first = (await calls('/api/resources/prepare-import-batch'))[0], locator = JSON.parse(first.locatorBeforeRequest);
  assert.deepEqual(first.body.files.map(file => file.name), ['renamed-image.png', 'renamed-document.txt']);
  assert.equal(locator.mode, 'linksAppend'); assert.equal(locator.operationId, first.body.operationId); assert.equal(locator.requestKind, 'operation');
  assert.equal(JSON.stringify(locator).includes('Owned author'), false); assert.equal(Object.hasOwn(locator, 'source'), false); assert.equal(Object.hasOwn(locator, 'uploads'), false);
  assert.deepEqual(Object.keys(locator.files[0]).sort(), ['bytes', 'fileName', 'name', 'sha256']);
  assert.ok((await dialog().locator('[data-resource-import-changes]').textContent()).includes('renamed-image.png'));
  assert.equal((await calls('/api/operations/commit')).length, 0);
  await control('confirm').click(); await idle(); assert.equal((await calls('/api/operations/commit')).length, 1);
  pass('Ordered names, mode, byte preview and complete prepared impacts precede explicit confirmation; the original locator is durably present before prepare and contains metadata without source or upload bytes.');

  await prepare('importOnly', { editable: false }); await control('confirm').click(); await idle();
  let actual = await snapshot(); assert.equal(actual.source, actual.original); assert.equal(actual.applies, 0); assert.equal(actual.applyAttempts, 0);
  assert.equal((await calls('/api/editor-map')).length, 0); assert.equal((await calls('/api/resources/import-batch/insert')).length, 0);
  await control('finish').click(); await idle(); assert.equal((await snapshot()).pending, false); assert.equal(await currentLocator(), null);
  pass('Read-only bodies can import-only, with no editor map or insertion, and a qualified explicit finish removes the locator.');

  for (const mode of ['autoAppend', 'linksAppend']) {
    await prepare(mode); await control('confirm').click(); await idle(); actual = await snapshot();
    assert.equal(actual.applies, 1); assert.equal(actual.applyAttempts, 1); assert.equal((await calls('/api/editor-map')).length, 1); assert.equal((await calls('/api/resources/import-batch/insert')).length, 1);
    assert.equal(actual.source.endsWith('#endnotes()'), true); assert.notEqual(actual.source, actual.original);
    assert.equal((await calls('/api/resources/import-batch/insert'))[0].body.mode, mode === 'autoAppend' ? 'auto' : 'links');
    assert.equal((await currentLocator()).value.insertion, 'inserted');
    await page.evaluate(() => window.resourceFixture.editor.undo());
    assert.equal((await snapshot()).source, actual.original, 'One real SourceEditor Undo must remove the whole imported batch insertion');
  }
  pass('Auto and links modes each use one exact native map/proposal and one real SourceEditor patch; a single Undo restores the complete original including trailing author content.');

  await prepare('importOnly', { losePrepare: true });
  assert.equal((await calls('/api/resources/prepare-import-batch')).length, 1); assert.ok((await calls('/api/operations/status')).length >= 1);
  assert.equal(await control('confirm').count(), 1); const lostId = (await currentLocator()).value.operationId;
  await control('recover').click(); await idle(); assert.equal((await currentLocator()).value.operationId, lostId); assert.equal((await calls('/api/resources/prepare-import-batch')).length, 1);
  await control('cancel-plan').click(); await idle(); assert.equal((await calls('/api/recovery/resolve'))[0].body.action, 'keepCurrent');
  assert.equal(await currentLocator(), null);
  pass('A lost prepare response recovers the original prepared plan without retransmitting uploads; explicit cancellation resolves that same operation with keepCurrent.');

  await prepare('importOnly');
  const oldConfirm = await control('confirm').elementHandle(); assert.ok(oldConfirm);
  assert.equal(await oldConfirm.evaluate(element => element.disabled), false);
  const retiredBatchId = (await currentLocator()).value.operationId;
  await control('cancel-plan').click(); await idle(); assert.equal(await currentLocator(), null);
  await open('importOnly'); await control('prepare').click(); await idle();
  const nextBatchRaw = (await currentLocator()).raw, nextBatchId = JSON.parse(nextBatchRaw).operationId;
  assert.notEqual(nextBatchId, retiredBatchId);
  assert.equal(await oldConfirm.evaluate(element => element.disabled), false, 'The stale-confirm regression must not be satisfied by disabling the old element');
  await oldConfirm.evaluate(element => element.click()); await idle();
  assert.equal((await calls('/api/operations/commit')).length, 0, 'An enabled detached confirm callback must not commit the current replacement batch');
  assert.equal((await currentLocator()).raw, nextBatchRaw); assert.equal((await snapshot()).applyAttempts, 0);
  await control('confirm').click(); await idle();
  assert.equal((await calls('/api/operations/commit')).length, 1); assert.equal((await calls('/api/operations/commit'))[0].body.operationId, nextBatchId);
  await control('finish').click(); await idle(); assert.equal(await currentLocator(), null); await oldConfirm.dispose();
  pass('An enabled confirm from a cancelled batch cannot commit a later prepared batch or change its locator; only the current connected confirmation commits the new original.');

  await prepare('importOnly', { losePrepare: true, statusFail: true });
  const retryLocatorRaw = (await currentLocator()).raw, retryWire = await page.evaluate(() => window.resourceFixture.controller.originalWire);
  const staleRetry = await control('retry').elementHandle(); assert.ok(staleRetry); assert.equal(await control('retry').isEnabled(), true);
  await page.evaluate(() => window.resourceFixture.holdOriginalWireDigest()); await control('retry').click();
  await page.waitForFunction(() => window.resourceFixture.heldDigests.length === 1);
  await page.evaluate(() => {
    const f = window.resourceFixture;
    f.basis = { ...f.basis, sessionId: 'same-node-runtime-new-session', path: 'D:/Owned/ResourceBatch/Moved/Moved.typ' };
    for (const item of f.heldDigests.splice(0)) item.release();
  });
  await idle(); assert.equal(await control('retry').isDisabled(), true);
  assert.equal((await calls('/api/resources/prepare-import-batch')).length, 1, 'A route change during the real digest must stop the old wire before transport');
  await staleRetry.evaluate(element => element.click()); await idle(); await staleRetry.dispose();
  assert.equal((await calls('/api/resources/prepare-import-batch')).length, 1, 'An enabled detached callback from the old route must not retransmit uploads');
  assert.equal((await currentLocator()).raw, retryLocatorRaw); assert.equal(await page.evaluate(() => window.resourceFixture.controller.originalWire), retryWire);
  const statusAtStart = (await calls('/api/resources/import-batch/status')).length;
  await page.evaluate(() => { window.resourceFixture.statusFail = false; }); await control('recover').click(); await idle();
  const recoveredRoutes = (await calls('/api/resources/import-batch/status')).slice(statusAtStart);
  assert.ok(recoveredRoutes.length > 0); assert.ok(recoveredRoutes.every(item => item.body.sessionId === 'same-node-runtime-new-session'));
  assert.equal((await currentLocator()).raw, retryLocatorRaw); assert.equal(await page.evaluate(() => window.resourceFixture.controller.originalWire), retryWire);
  assert.equal((await calls('/api/resources/prepare-import-batch')).length, 1); assert.equal((await snapshot()).applyAttempts, 0);
  await control('cancel-plan').click(); await idle(); assert.equal(await currentLocator(), null);
  pass('Same-page session/path changes retire the original upload route even during its real digest and through an old retry callback; recovery qualifies only the new session while preserving the original wire, locator and operation.');

  await prepare('importOnly', { prepareMissing: true });
  const missing = (await currentLocator()).value, settled = (await calls('/api/operations/settle'))[0].body;
  assert.deepEqual(settled, { operationId: missing.operationId, requestKind: 'operation', wireDigest: missing.wireDigest });
  assert.equal((await calls('/api/resources/prepare-import-batch')).length, 1); assert.equal((await calls('/api/operations/commit')).length, 0);
  await control('finish').click(); await idle(); assert.equal(await currentLocator(), null);
  pass('A missing original enters settle with requestKind operation and the original wire digest, then finishes without a replacement ID or import.');

  await prepare('linksAppend'); const beforeReload = await snapshot(), beforeReloadLocator = (await currentLocator()).raw;
  await page.reload(); await install({ clear: false, sessionId: 'same-node-new-session', restore: { requests: beforeReload.requests, operations: beforeReload.operations } });
  await page.locator('.cm-content').focus();
  await page.evaluate(() => window.resourceFixture.controller.recover());
  assert.equal(await dialog().isVisible(), false); assert.equal(await page.locator('.cm-content').evaluate(element => document.activeElement === element), true);
  assert.equal((await currentLocator()).raw, beforeReloadLocator);
  await page.evaluate(() => { const f = window.resourceFixture; f.open([new File(['must not replace'], 'new-file.txt')]); }); await dialog().waitFor({ state: 'visible' }); await idle();
  assert.deepEqual(await dialog().locator('tbody tr td:nth-child(2)').allTextContents(), ['original.png', 'original.txt']);
  assert.equal((await calls('/api/resources/prepare-import-batch')).length, 1);
  assert.equal((await calls('/api/resources/import-batch/status')).at(-1).body.sessionId, 'same-node-new-session');
  pass('A real page reload retains only its locator; background recovery keeps focus, qualifies the same node’s new session and never adopts newly offered files.');

  const beforeMoveReload = await snapshot(), beforeMoveRaw = (await currentLocator()).raw;
  await page.reload(); await install({ clear: false, path: 'D:/Owned/ResourceBatch/Moved/Moved.typ', sessionId: 'same-node-moved-session', rejectCommit: true, restore: { requests: beforeMoveReload.requests, operations: beforeMoveReload.operations } });
  await open(); await idle();
  assert.equal((await currentLocator()).raw, beforeMoveRaw, 'The stable workspace/node key must discover and retain the locator from the old path');
  assert.equal((await calls('/api/resources/prepare-import-batch')).length, 1);
  assert.equal((await calls('/api/resources/import-batch/status')).at(-1).body.sessionId, 'same-node-moved-session');
  assert.equal(await control('prepare').count(), 0);
  await control('confirm').click(); await idle();
  assert.equal((await currentLocator()).raw, beforeMoveRaw); assert.equal((await calls('/api/operations/commit')).at(-1).body.operationId, JSON.parse(beforeMoveRaw).operationId);
  assert.equal((await calls('/api/resources/prepare-import-batch')).length, 1); assert.equal((await snapshot()).applyAttempts, 0);
  await control('cancel-plan').click(); await idle(); assert.equal(await currentLocator(), null);
  await prepare('linksAppend'); await page.evaluate(() => { window.resourceFixture.editable = false; }); await control('confirm').click(); await idle();
  const movedCommitted = await snapshot(), movedCommittedRaw = (await currentLocator()).raw;
  await page.reload(); await install({ clear: false, path: 'D:/Owned/ResourceBatch/Moved/Moved.typ', sessionId: 'same-node-committed-moved-session', restore: { requests: movedCommitted.requests, operations: movedCommitted.operations } });
  await page.locator('.cm-content').focus(); await page.evaluate(() => window.resourceFixture.controller.recover());
  assert.equal(await dialog().isVisible(), false); assert.equal((await currentLocator()).raw, movedCommittedRaw);
  await open(); await idle(); await control('insert').click(); await idle();
  assert.equal((await snapshot()).applyAttempts, 1); assert.equal((await calls('/api/resources/prepare-import-batch')).length, 1);
  assert.equal((await calls('/api/editor-map')).at(-1).body.sessionId, 'same-node-committed-moved-session');
  assert.equal((await currentLocator()).value.owner.path, JSON.parse(movedCommittedRaw).owner.path, 'Runtime route qualification must not rewrite the frozen locator owner');
  await control('finish').click(); await idle(); assert.equal(await currentLocator(), null);
  pass('A same-node move discovers the original stable-key locator and qualifies its current session; a Core-rejected old-path commit stays original, while qualified committed resources insert on the latest exact map without rewriting the locator or uploading again.');

  await prepare('linksAppend'); await page.evaluate(() => { window.resourceFixture.hold = '/api/editor-map'; }); await control('confirm').click();
  await page.waitForFunction(() => window.resourceFixture.held.length === 1);
  await page.evaluate(() => { const f = window.resourceFixture; f.mutate('owner'); f.release(); }); await idle();
  actual = await snapshot(); assert.equal(actual.applies, 0); assert.equal(actual.applyAttempts, 0); assert.equal(actual.source, actual.original); assert.ok(actual.raw);
  assert.equal(JSON.parse(actual.raw).owner.nodeId, '22222222-2222-4222-8222-222222222222');
  pass('A→B navigation while a native map is pending cannot apply an old proposal and retains the original A locator.');

  for (const stage of ['/api/editor-map', '/api/resources/import-batch/insert']) for (const mutation of ['source', 'inputId', 'baseId', 'serial', 'permission']) {
    await prepare('linksAppend'); await page.evaluate(stage => { window.resourceFixture.hold = stage; }, stage); await control('confirm').click();
    await page.waitForFunction(() => window.resourceFixture.held.length === 1);
    await page.evaluate(mutation => { const f = window.resourceFixture; f.mutate(mutation); f.release(); }, mutation); await idle();
    actual = await snapshot(); assert.equal(actual.applies, 0, `${stage}/${mutation} applied a stale proposal`); assert.equal(actual.applyAttempts, 0, `${stage}/${mutation} delivered a stale proposal to the host`); assert.ok(actual.raw);
    assert.equal((await calls('/api/resources/prepare-import-batch')).length, 1);
    if (mutation !== 'source') assert.equal(actual.source, actual.original);
    else assert.equal(actual.source, 'Later author input\n' + actual.original);
  }
  pass('Later source, input ID, base, serial and permission changes retire both pending-map and pending-insertion deliveries without altering later input or repeating import.');

  await prepare('importOnly', { changedFail: true }); await control('confirm').click(); await idle();
  assert.equal((await calls('/api/operations/commit')).length, 1); assert.equal((await calls('/api/resources/prepare-import-batch')).length, 1); assert.ok(await currentLocator());
  assert.equal(await control('finish').isDisabled(), true, 'An unrefreshed committed outcome cannot be discarded as finished');
  await page.evaluate(() => { window.resourceFixture.changedFail = false; }); await control('recover').click(); await idle();
  assert.equal((await calls('/api/operations/commit')).length, 1); assert.equal((await calls('/api/resources/prepare-import-batch')).length, 1);
  assert.equal(await control('confirm').count(), 0);
  pass('Committed import plus a failed environment refresh retains the committed original; recovery refreshes it without a second import or commit.');

  await prepare('linksAppend'); await page.evaluate(() => { window.resourceFixture.editable = false; }); await control('confirm').click(); await idle();
  await page.evaluate(() => { const f = window.resourceFixture; f.editable = true; f.ackFail = true; f.controller.refresh(); });
  const staleInsert = await control('insert').elementHandle(); assert.ok(staleInsert);
  await control('insert').click(); await idle(); actual = await snapshot(); assert.equal(actual.applies, 1); assert.equal(actual.applyAttempts, 1); assert.notEqual(actual.source, actual.original);
  assert.ok(['uncertain', 'applying'].includes((await currentLocator()).value.insertion)); assert.equal(await control('insert').count(), 0);
  await control('recover').click(); await idle(); await staleInsert.evaluate(element => element.click()); await idle();
  assert.equal((await snapshot()).applies, 1); assert.equal((await snapshot()).applyAttempts, 1); assert.equal((await calls('/api/resources/import-batch/insert')).length, 1); await staleInsert.dispose();
  pass('An editor mutation followed by an ACK rejection is consumed as uncertain; recovery and a stale insert callback cannot replay the proposal.');

  for (const storageFault of ['read', 'write', 'readback']) {
    await reset({ storageFault }); await open();
    if (storageFault !== 'read') { await control('prepare').click(); await idle(); }
    assert.equal((await calls('/api/resources/prepare-import-batch')).length, 0); assert.equal((await snapshot()).pending, true);
    if (storageFault === 'readback') assert.ok(await currentLocator(), 'A mismatching readback must preserve the original stored record');
    const attempted = await page.evaluate(() => window.resourceFixture.storageEvents.filter(event => event.kind === 'set').at(-1)?.value ?? null);
    await page.evaluate(() => { window.resourceFixture.storageFault = null; }); await control('recover-storage').click(); await idle();
    if (storageFault === 'read') { await control('close').click(); assert.equal((await snapshot()).pending, false); }
    else {
      const recovered = (await currentLocator()).value;
      assert.equal(recovered.operationId, JSON.parse(attempted).operationId, 'Transient storage recovery must reuse the exact attempted operation');
      assert.equal((await calls('/api/operations/settle')).at(-1).body.operationId, recovered.operationId);
      assert.equal((await calls('/api/resources/prepare-import-batch')).length, 0, 'Storage recovery cannot invent or send another import');
      await control('finish').click(); await idle(); assert.equal(await currentLocator(), null);
    }
  }
  await prepare(); await control('confirm').click(); await idle();
  const beforeDelete = (await currentLocator()).raw; await page.evaluate(() => { window.resourceFixture.storageFault = 'delete'; }); await control('finish').click(); await idle();
  assert.equal((await currentLocator()).raw, beforeDelete); assert.equal((await snapshot()).pending, true);
  await page.evaluate(() => { window.resourceFixture.storageFault = null; }); await control('recover-storage').click(); await idle();
  assert.equal((await currentLocator()).raw, beforeDelete); await control('finish').click(); await idle(); assert.equal(await currentLocator(), null);
  assert.equal((await calls('/api/resources/prepare-import-batch')).length, 1); assert.equal((await calls('/api/operations/commit')).length, 1);
  await prepare(); await control('confirm').click(); await idle();
  const replacementRaw = await page.evaluate(() => { const f = window.resourceFixture, newer = JSON.parse(localStorage.getItem(f.key)); newer.operationId = '99999999-9999-4999-8999-999999999999'; const raw = JSON.stringify(newer); localStorage.setItem(f.key, raw); return raw; });
  await control('finish').click(); await idle(); assert.equal((await currentLocator()).raw, replacementRaw); assert.equal((await snapshot()).pending, true);
  await control('recover-storage').click(); await idle(); assert.equal((await currentLocator()).raw, replacementRaw); assert.equal(await control('finish').count(), 0);
  pass('Storage read/write/readback/delete failures preserve originals and recover only the exact original or attempted token after a transient fault; old tokens cannot adopt, finish or remove a newer locator.');

  // Two real pages share the browser context, origin, localStorage and native
  // LockManager. A real externally held lock aligns both mutation contenders;
  // neither controller's lock nor its storage/authorization guard is mocked.
  let secondPage;
  const staleFinishes = [];
  try {
    await reset(); await open('linksAppend');
    secondPage = await page.context().newPage(); secondPage.setDefaultTimeout(15000);
    secondPage.on('pageerror', error => result.pageErrors.push(String(error.stack || error)));
    assert.equal(secondPage.context(), page.context());
    await reset({ clear: false, sessionId: 'second-page-initial-session' }, secondPage); await open('linksAppend', secondPage);
    assert.equal(new URL(secondPage.url()).origin, new URL(page.url()).origin);
    const peers = [page, secondPage];
    assert.equal(await currentLocator(page), null); assert.equal(await currentLocator(secondPage), null);
    await page.locator('[data-resource-import-name="0"]').fill('first-racer.png');
    await secondPage.locator('[data-resource-import-name="0"]').fill('second-racer.png');
    await holdRealStorageLock(page);
    await Promise.all(peers.map(target => control('prepare', target).click()));
    const prepareLocks = await waitRealStorageContenders(page, 2);
    await releaseRealStorageLock(page); await Promise.all(peers.map(target => idle(target)));
    let ports = await Promise.all(peers.map(target => snapshot(target)));
    const prepareCounts = ports.map(port => port.requests.filter(request => request.route === '/api/resources/prepare-import-batch').length);
    assert.equal(prepareCounts.reduce((sum, count) => sum + count, 0), 1, 'Only one empty-record contender can reach the resource prepare endpoint');
    let winnerIndex = prepareCounts.findIndex(count => count === 1), loserIndex = 1 - winnerIndex;
    let winner = peers[winnerIndex], loser = peers[loserIndex];
    const wonRaw = (await currentLocator(winner)).raw;
    assert.equal((await currentLocator(loser)).raw, wonRaw);
    assert.equal(JSON.parse(wonRaw).operationId, ports[winnerIndex].requests.find(request => request.route === '/api/resources/prepare-import-batch').body.operationId);
    assert.equal(ports.reduce((sum, port) => sum + port.applyAttempts, 0), 0);
    await control('recover-storage', loser).click(); await idle(loser);
    assert.equal((await currentLocator(loser)).raw, wonRaw, 'The losing initial write must not overwrite the winner when retried');

    // Leave the winning import committed but with no body insertion. Both new
    // pages then recover that exact same locator and independent source views.
    await winner.evaluate(() => { window.resourceFixture.editable = false; });
    await control('confirm', winner).click(); await idle(winner);
    const committedOwner = await snapshot(winner);
    assert.equal(committedOwner.applyAttempts, 0); assert.equal((await currentLocator(winner)).value.insertion, 'pending');
    await reset({ clear: false, sessionId: 'first-page-insert-session', restore: { operations: committedOwner.operations } }, page);
    await reset({ clear: false, sessionId: 'second-page-insert-session', restore: { operations: committedOwner.operations } }, secondPage);
    await Promise.all(peers.map(target => open('linksAppend', target))); await Promise.all(peers.map(target => idle(target)));
    assert.equal((await currentLocator(page)).raw, (await currentLocator(secondPage)).raw);
    for (const target of peers) { const finish = await control('finish', target).elementHandle(); assert.ok(finish); staleFinishes.push(finish); }
    await holdRealStorageLock(page);
    await Promise.all(peers.map(target => control('insert', target).click()));
    const insertLocks = await waitRealStorageContenders(page, 2);
    await releaseRealStorageLock(page); await Promise.all(peers.map(target => idle(target)));
    ports = await Promise.all(peers.map(target => snapshot(target)));
    assert.equal(ports.reduce((sum, port) => sum + port.applyAttempts, 0), 1, 'Competing original insertion claims must deliver at most one proposal to SourceEditor');
    winnerIndex = ports.findIndex(port => port.applyAttempts === 1); loserIndex = 1 - winnerIndex;
    winner = peers[winnerIndex]; loser = peers[loserIndex];
    assert.notEqual(ports[winnerIndex].source, ports[winnerIndex].original); assert.equal(ports[loserIndex].source, ports[loserIndex].original);
    assert.equal((await currentLocator(winner)).value.insertion, 'inserted');
    assert.equal(ports.reduce((sum, port) => sum + port.requests.filter(request => request.route === '/api/resources/prepare-import-batch').length, 0), 0);
    const originalOperation = (await currentLocator(winner)).value.operationId;
    await control('finish', winner).click(); await idle(winner); assert.equal(await currentLocator(winner), null);
    await open('importOnly', winner); await control('prepare', winner).click(); await idle(winner);
    const nextRaw = (await currentLocator(winner)).raw; assert.notEqual(JSON.parse(nextRaw).operationId, originalOperation);
    await staleFinishes[loserIndex].evaluate(element => element.click()); await idle(loser);
    assert.equal((await currentLocator(loser)).raw, nextRaw, 'The losing owner\'s stale finish callback must not delete a later operation');
    await control('recover-storage', loser).click(); await idle(loser); assert.equal((await currentLocator(loser)).raw, nextRaw);
    assert.equal((await snapshot(loser)).applyAttempts, 0);
    await control('cancel-plan', winner).click(); await idle(winner); assert.equal(await currentLocator(winner), null);
    result.crossTabOwnership = { sameBrowserContext: true, sameOrigin: true, prepareCounts, applyAttempts: ports.map(port => port.applyAttempts), prepareLocks, insertLocks };
    pass('Real same-origin pages queue on the native per-node Web Lock: one initial locator wins one prepare, one original insertion claim reaches SourceEditor, and a stale peer cannot remove a later record.');
  } finally {
    await releaseRealStorageLock(page).catch(() => {});
    for (const handle of staleFinishes) await handle.dispose().catch(() => {});
    await secondPage?.close();
  }

  await reset();
  assert.equal(await page.evaluate(() => window.resourceFixture.controller.freezeForClose()), true);
  await page.evaluate(() => { window.resourceFixture.open(); }); await page.waitForFunction(() => window.resourceFixture.openErrors.length === 1);
  assert.equal(await dialog().isVisible(), false); await page.evaluate(() => window.resourceFixture.controller.thawAfterClose()); await open();
  const nameInput = page.locator('[data-resource-import-name="0"]');
  await nameInput.focus(); await nameInput.dispatchEvent('compositionstart', { data: '中' });
  await control('prepare').click(); await control('close').click();
  assert.equal((await calls('/api/resources/prepare-import-batch')).length, 0); assert.equal(await dialog().isVisible(), true);
  assert.equal(await page.evaluate(() => window.resourceFixture.controller.freezeForClose()), false);
  await nameInput.dispatchEvent('compositionend', { data: '中文' });
  await page.evaluate(() => { window.resourceFixture.hold = '/api/resources/prepare-import-batch'; }); await control('prepare').click();
  await page.waitForFunction(() => window.resourceFixture.held.length === 1);
  assert.equal(await control('close').isDisabled(), true); await page.keyboard.press('Escape'); assert.equal(await dialog().isVisible(), true);
  assert.equal(await page.evaluate(() => { try { window.resourceFixture.controller.assertCloseReady(); return true; } catch { return false; } }), false);
  await page.evaluate(() => window.resourceFixture.release()); await idle(); await control('close').click();
  assert.equal(await dialog().isVisible(), false); assert.equal((await snapshot()).pending, true);
  await page.evaluate(() => window.resourceFixture.open()); await dialog().waitFor({ state: 'visible' }); await idle(); await control('cancel-plan').click(); await idle();
  assert.equal(await page.evaluate(() => { window.resourceFixture.controller.assertCloseReady(); return window.resourceFixture.controller.freezeForClose(); }), true);
  pass('Frozen, busy and composing owners preserve their close boundaries; hiding a prepared dialog retains the original, and explicit cancellation releases the owner.');

  assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) {
  result.failure = String(error.stack || error);
  if (page) { await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {}); await writeFile(path.join(out, 'failure.html'), await page.content().catch(() => '')).catch(() => {}); }
} finally {
  try { if (page && !page.isClosed()) result.finalPorts = await page.evaluate(() => { const f = window.resourceFixture; return f ? { routes: f.requests.map(item => item.route), applies: f.applies.length, pending: f.controller.hasPending() } : null; }); } catch (error) { result.portCollectionFailure = String(error); }
  try { await browser?.close(); } catch (error) { result.browserCloseFailure = String(error); }
  try { await vite.close(); } catch (error) { result.frontendCloseFailure = String(error); }
  result.ownedProcessesClosed = !result.browserCloseFailure && !result.frontendCloseFailure;
  await writeFile(path.join(out, 'results.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify({ out, completed: result.completed, cases: result.cases.length, failure: result.failure, ownedProcessesClosed: result.ownedProcessesClosed }));
}
if (!result.completed || !result.ownedProcessesClosed) process.exitCode = 1;
