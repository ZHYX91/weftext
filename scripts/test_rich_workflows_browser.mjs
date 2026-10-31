// Four rich-data tasks against one selected CLI's embedded UI and actual Core.
// Only response delivery is fault-injected; no Vite, mock backend or user files.
import assert from 'node:assert/strict';
import { createHash, randomUUID, randomBytes } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { spawnServe, serveReady, stopServe, stringifyServeJson } from './_cli_serve.mjs';
import { createdDocument } from './_creation_result.mjs';
import { workspaceCommand, workspaceMode } from './_workspace_browser.mjs';
import { richArtifact, fileSha256, verifyEmbeddedFrontend } from './_rich_test_artifact.mjs';
const repo = path.resolve(import.meta.dirname, '..'), exe = process.env.WEFTEXT_TEST_EXECUTABLE, out = process.env.WEFTEXT_BROWSER_REPORT;
assert.ok(exe && path.isAbsolute(exe) && out && path.isAbsolute(out));
assert.ok(path.relative(repo, out).startsWith('..' + path.sep));
await mkdir(out, { recursive: false });
const sha = value => createHash('sha256').update(value).digest('hex'), pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const artifact = await richArtifact(exe, repo, process.env.WEFTEXT_TEST_SOURCE_REF ?? 'HEAD');
const harnessFiles = ['scripts/test_rich_workflows_browser.mjs', 'scripts/_rich_test_artifact.mjs', 'scripts/_cli_serve.mjs', 'scripts/_creation_result.mjs', 'scripts/_workspace_browser.mjs'];
const inventory = () => Promise.all(harnessFiles.map(async (name) => ({ path: name, sha256: sha(await readFile(path.join(repo, name))) })));
const result = { startedUtc: new Date().toISOString(), artifact, harnessBefore: await inventory(), scope: 'Actual embedded local DataPanel/Query/Save/Undo for contacts, employment, publication and independently owned relations, saved-file reopening and original-operation lookup; actual Hosted relation fields remain read-only even with a write grant. Owned synthetic libraries/Chrome only, no native/IME/AT/Office/citation acceptance.', cases: [], services: [], uiRequests: [], http: [], pageErrors: [], routeErrors: [], completed: false };
const { chromium } = createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const root = path.join(out, 'library'), state = path.join(out, 'state'), tokens = { alice: randomBytes(32).toString('hex'), admin: randomBytes(32).toString('hex') };
let child, base, token, browser, page, phase = 'setup', ownerRecord;
const pendingResponses = new Set();
async function until(check, label, ms = 30000) { const end = Date.now() + ms; while (Date.now() < end) {
    const value = await check();
    if (value)
        return value;
    await pause(80);
} throw new Error('Timed out ' + label); }
async function start(args = []) { assert.equal(child, undefined); child = spawnServe(exe, ['--data-dir', state, ...args], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] }); let stdout = '', stderr = ''; const record = { pid: child.pid, args: child.spawnargs.slice(1), startedUtc: new Date().toISOString() }; ownerRecord = record; result.services.push(record); child.stdout.on('data', b => stdout += b); child.stderr.on('data', b => stderr += b); record.logs = () => ({ stdout, stderr }); await until(() => { const ready = serveReady(child); if (ready) {
    base = ready.base;
    token = ready.token;
    record.ownerIdentity = ready.ownerIdentity;
    return true;
} if (child.exitCode !== null)
    throw new Error(stderr); return false; }, 'owned CLI Ready'); }
async function stop() { if (!child)
    return; const owned = child, record = ownerRecord; child = undefined; record.stopAttempted = true; try {
    record.stopReceipt = await stopServe(owned);
    assert.equal(owned.exitCode, 0);
    assert.equal(owned.signalCode, null);
}
finally {
    record.actualExitCode = owned.exitCode;
    record.actualSignal = owned.signalCode;
    record.stoppedUtc = new Date().toISOString();
    const logs = record.logs();
    delete record.logs;
    await writeFile(path.join(out, `pid-${owned.pid}-stdout.log`), logs.stdout, { flag: 'wx' });
    await writeFile(path.join(out, `pid-${owned.pid}-stderr.log`), logs.stderr, { flag: 'wx' });
} }
async function api(route, body, actor, expected = 200) { const response = await fetch(base + route, { method: body === undefined ? 'GET' : 'POST', headers: { 'Content-Type': 'application/json', ...(actor ? { Authorization: 'Bearer ' + tokens[actor] } : { 'X-Weftext-Token': token }) }, body: body === undefined ? undefined : JSON.stringify(body), signal: AbortSignal.timeout(45000) }); const value = await response.json(); result.http.push({ phase, route, status: response.status, request: body, response: route.endsWith('/schema') ? { sourceSha256: sha(value.source ?? ''), revision: value.revision } : value }); assert.equal(response.status, expected, route + ': ' + JSON.stringify(value)); return value; }
const text = value => ({ kind: 'text', value }), array = (...value) => ({ kind: 'array', value }), record = values => ({ kind: 'record', value: Object.entries(values).map(([key, value]) => ({ key, value })) }), none = { kind: 'none' };
const member = (value, key) => value.value.find(item => item.key === key).value, idx = index => ({ kind: 'index', index }), mem = key => ({ kind: 'member', key });
const ref = doc => `weftext-node://${doc.workspaceId}/${doc.nodeId}`;
const date = (year, month) => record({ calendar: text('gregorian'), version: text('proleptic-gregorian.v1'), precision: text(month ? 'month' : 'year'), year: { kind: 'int', value: String(year) }, ...(month ? { month: { kind: 'int', value: String(month) } } : {}) });
async function capture(name, types, fields, body = 'TEST owned rich workflow body😀\n') { const operationId = randomUUID(); return createdDocument(await api('/api/data/capture', { root, parent: root, name, title: name, types, fields, body, operationId }), operationId); }
async function saveFixture(doc, source) { const receipt = await api('/api/save', { sessionId: doc.sessionId, baseId: doc.baseId, source, operationId: randomUUID() }); assert.equal(receipt.status, 'saved'); assert.equal(receipt.documentSourceMatchesCommit, true); return receipt.document; }
async function setFixture(doc, key, value) { const patch = await api('/api/data/edit', { sessionId: doc.sessionId, source: doc.source, serial: 1, key, path: [], value }); return saveFixture(doc, patch.source); }
// Match the existing delivery harness: a protected Windows handle can briefly
// exclude an independent reader after commit. Retry only read access, never writes.
async function readOwnedFile(file) { const deadline = Date.now() + 10000; for (;;) {
    try {
        return await readFile(file);
    }
    catch (error) {
        if (!['EBUSY', 'EPERM'].includes(error.code) || Date.now() >= deadline)
            throw error;
        result.readAccessRetries ??= [];
        result.readAccessRetries.push({ phase, file, code: error.code });
        await pause(80);
    }
} }
async function disk(file) { return (await readOwnedFile(file)).toString('utf8'); }
async function source() { return page.locator('.source-host .cm-content').evaluate(element => { const view = element.cmTile?.root?.view; if (view?.contentDOM !== element)
    throw new Error('Actual CodeMirror unavailable'); return view.state.doc.toString(); }); }
async function waitSource(expected) { await until(async () => await source() === expected, 'actual source bytes'); }
function observe(target) { target.on('pageerror', e => result.pageErrors.push(String(e))); target.on('dialog', dialog => dialog.accept()); const requests = new Map(); target.on('request', request => { const route = new URL(request.url()).pathname; if (request.method() !== 'POST' || !route.startsWith('/api/'))
    return; const entry = { phase, route, request: request.postDataJSON() }; requests.set(request, entry); result.uiRequests.push(entry); }); target.on('response', response => { const entry = requests.get(response.request()); if (!entry)
    return; const pending = (async () => { entry.status = response.status(); entry.response = await response.json(); })(); pendingResponses.add(pending); void pending.catch(() => { }).finally(() => pendingResponses.delete(pending)); }); }
function responseFor(predicate) { const pending = page.waitForResponse(predicate); void pending.catch(() => { }); return pending; }
async function localPage(doc) { await start(['--open', doc.path, '--root', root]); const context = await browser.newContext({ viewport: { width: 1480, height: 1100 } }); page = await context.newPage(); observe(page); page.setDefaultTimeout(30000); await page.goto(base + '/#token=' + token); await workspaceMode(page, '源码'); await waitSource(doc.source); await workspaceCommand(page, 'knowledge', '属性'); await page.locator(`.data-property[data-field-key="${doc.fieldKey}"]`).waitFor({ state: 'attached' }); const advanced = page.locator('.data-panel .data-field-group.advanced'); if (await advanced.count() && !await advanced.evaluate(e => e.open))
    await advanced.locator(':scope > summary').click(); return context; }
async function control(rootSelector, fieldId, pathValue, role) { const key = await page.locator(rootSelector + ' [data-control-key]').evaluateAll((elements, { fieldId, pathValue, role }) => { const matches = elements.filter(element => { const key = JSON.parse(element.dataset.controlKey); return key[0] === fieldId && JSON.stringify(key[1]) === JSON.stringify(pathValue) && key[2][0] === role; }); if (matches.length !== 1)
    throw new Error('Expected exact actual native control: ' + JSON.stringify({ fieldId, pathValue, role, count: matches.length })); return matches[0].dataset.controlKey; }, { fieldId, pathValue, role }); return page.locator(rootSelector).locator(`[data-control-key=${JSON.stringify(key)}]`); }
async function move(rootSelector, id, pathValue, to) { const input = await control(rootSelector, id, pathValue, 'order-position'), details = input.locator('xpath=..'); if (!await details.evaluate(e => e.open))
    await details.locator(':scope > summary').click(); await input.fill(String(to)); await input.press('Enter'); }
async function applyField(doc, expected) { const rootSelector = `.data-property[data-field-key="${doc.fieldKey}"]`, response = responseFor(r => new URL(r.url()).pathname === '/api/data/edit' && r.request().postDataJSON()?.key === doc.fieldKey); await page.locator(rootSelector).getByRole('button', { name: '应用到草稿', exact: true }).click(); const actual = await response; assert.equal(actual.status(), 200, await actual.text()); const request = actual.request().postDataJSON(); assert.deepEqual(request.value, expected); const wire = await actual.json(); await waitSource(wire.source); return wire.source; }
async function undoRedo(before, after) { await page.locator('.source-host .cm-content').click(); await page.keyboard.press('Control+z'); await waitSource(before); await page.keyboard.press('Control+Shift+z'); await waitSource(after); }
async function lostLocalSave(expected) { let original, actual, resolve; const delivered = new Promise(done => resolve = done), pattern = '**/api/save'; const before = result.uiRequests.length; const handler = async (route) => { try {
    original = route.request().postDataJSON();
    assert.equal(original.source, expected);
    const reply = await route.fetch({ maxRetries: 0 });
    actual = await reply.json();
    assert.equal(reply.status(), 200, JSON.stringify(actual));
    assert.equal(actual.status, 'saved');
    await route.abort('failed');
}
catch (error) {
    result.routeErrors.push(String(error));
    await route.abort('failed').catch(() => { });
}
finally {
    resolve();
} }; await page.route(pattern, handler); const status = responseFor(r => new URL(r.url()).pathname === '/api/operations/status' && r.request().postDataJSON()?.operationId === original?.operationId); await page.keyboard.press('Control+s'); await delivered; assert.deepEqual(result.routeErrors, []); const response = await status; assert.equal(response.status(), 200); const recovered = await response.json(); assert.equal(recovered.operationId, original.operationId); assert.equal(recovered.status, 'committed'); await until(async () => await page.locator('.save-state').first().innerText() === '已保存', 'saved UI original receipt'); await page.unroute(pattern, handler); await waitSource(expected); const posts = result.uiRequests.slice(before).filter(entry => entry.route === '/api/save'); assert.equal(posts.length, 1); return { request: original, actualCommittedReply: actual, recoveredReply: recovered, authorPosts: posts.length, deliveryFault: 'actual committed Save response dropped, then product queried same original operation' }; }
async function queryUI(doc, condition, expectedRecords, rows) { const panel = page.locator('.query-panel'); if (!await panel.isVisible())
    await workspaceCommand(page, 'knowledge', '查询与视图'); const advanced = panel.locator('.query-advanced-plan'); if (!await advanced.evaluate(e => e.open))
    await advanced.locator(':scope > summary').click(); const where = `self == nodeRef(${JSON.stringify(ref(doc))}) && (${condition})`, plan = { types: [doc.type], where, select: [{ name: 'records', expr: doc.fieldKey }] }; await advanced.locator('.query-plan-input').fill(JSON.stringify(plan)); await advanced.getByLabel('本次调用的原生参数 JSON', { exact: true }).fill('{}'); const pending = responseFor(r => new URL(r.url()).pathname === '/api/query/run' && [where, `(${where})`].includes(r.request().postDataJSON()?.plan?.where)); await advanced.getByRole('button', { name: '运行完整计划', exact: true }).click(); const response = await pending; assert.equal(response.status(), 200, await response.text()); const result = await response.json(); assert.equal(result.coverage, 'complete', JSON.stringify(result.diagnostics)); assert.equal(result.rows.length, rows); if (rows)
    assert.deepEqual(result.rows[0].cells.records.value, expectedRecords); await panel.locator('.query-basis').filter({ hasText: result.cutId }).waitFor({ state: 'visible' }); assert.equal(await source(), doc.savedSource); return { request: response.request().postDataJSON(), response: result, displayedCutId: result.cutId }; }
async function localTask(doc, edit, positive, negative, expected) {
    phase = doc.testId ?? doc.fieldKey;
    const context = await localPage(doc), before = doc.source;
    await edit(`.data-property[data-field-key="${doc.fieldKey}"]`);
    assert.equal(await source(), before, 'form is staged until Apply');
    const after = await applyField(doc, expected);
    await undoRedo(before, after);
    const saved = await lostLocalSave(after);
    assert.equal(await disk(doc.path), after);
    doc.savedSource = after;
    const positiveQuery = await queryUI(doc, positive, expected, 1), negativeQuery = await queryUI(doc, negative, expected, 0);
    await writeFile(path.join(out, (doc.testId ?? doc.fieldKey) + '-saved.typ'), after, { flag: 'wx' });
    await context.close();
    page = undefined;
    await stop();
    await start(['--open', doc.path, '--root', root]);
    const reopened = await api('/api/open', { path: doc.path, root });
    assert.equal(reopened.source, after);
    const original = await api('/api/operations/status', { operationId: saved.request.operationId });
    assert.equal(original.status, 'committed');
    assert.equal(original.operationId, saved.request.operationId);
    const projected = await api('/api/data/project', { sessionId: reopened.sessionId, source: reopened.source, serial: 1, sourceHash: sha(reopened.source) });
    assert.deepEqual(projected.projection.fields.find(f => f.key === doc.fieldKey).cell.value, expected);
    // A fresh browser and Core process must reopen the same authored records.
    const reopenedContext = await browser.newContext({ viewport: { width: 1480, height: 1100 } });
    page = await reopenedContext.newPage();
    observe(page);
    await page.goto(base + '/#token=' + token);
    await workspaceMode(page, '源码');
    await waitSource(after);
    await workspaceCommand(page, 'knowledge', '属性');
    const selector = `.data-property[data-field-key="${doc.fieldKey}"]`;
    await page.locator(selector).waitFor({ state: 'attached' });
    const advanced = page.locator('.data-panel .data-field-group.advanced');
    if (await advanced.count() && !await advanced.evaluate(e => e.open))
        await advanced.locator(':scope > summary').click();
    const checks = [];
    if (doc.fieldKey === 'phones')
        for (const [i, value] of expected.value.entries())
            for (const [key, role] of [['number', 'value:phone'], ['label', 'value:enum'], ['note', 'value:text']])
                checks.push({ path: [idx(i), mem(key)], role, value: member(value, key).value });
    if (doc.fieldKey === 'engagements')
        for (const [i, value] of expected.value.entries()) {
            checks.push({ path: [idx(i), mem('employer'), mem('ref')], role: 'value:node_ref', value: member(member(value, 'employer'), 'ref').value });
            checks.push({ path: [idx(i), mem('position')], role: 'value:text', value: member(value, 'position').value });
            checks.push({ path: [idx(i), mem('during'), mem('start'), mem('precision')], role: 'value:enum', value: member(member(member(value, 'during'), 'start'), 'precision').value });
        }
    if (doc.fieldKey === 'publication')
        for (const [i, value] of member(expected, 'contributors').value.entries()) {
            checks.push({ path: [mem('contributors'), idx(i), mem('agent'), mem('ref')], role: 'value:node_ref', value: member(member(value, 'agent'), 'ref').value });
            checks.push({ path: [mem('contributors'), idx(i), mem('credited_as')], role: 'value:text', value: member(value, 'credited_as').value });
            checks.push({ path: [mem('contributors'), idx(i), mem('roles'), idx(0)], role: 'value:nonempty_text', value: member(value, 'roles').value[0].value });
        }
    if (doc.fieldKey === 'siblings')
        for (const [i, value] of expected.value.entries()) {
            checks.push({ path: [idx(i), mem('target'), mem('ref')], role: 'value:node_ref', value: member(member(value, 'target'), 'ref').value });
            checks.push({ path: [idx(i), mem('note')], role: 'value:text', value: member(value, 'note').value });
        }
    for (const check of checks)
        assert.equal(await (await control(selector, doc.fieldId, check.path, check.role)).inputValue(), check.value);
    const reopenedQuery = await queryUI(doc, positive, expected, 1);
    await reopenedContext.close();
    page = undefined;
    await stop();
    const entry = { flow: doc.fieldKey === 'siblings' ? 'independent-relations' : doc.fieldKey, ...(doc.order ? { order: doc.order } : {}), sourceSha256: sha(after), saved, positiveQuery, negativeQuery, reopen: { sourceSha256: sha(reopened.source), nodeId: reopened.nodeId, visibleControls: checks }, reopenedQuery, originalAfterProcessRestart: original };
    result.cases.push(entry);
    await writeFile(path.join(out, 'progress.json'), stringifyServeJson({ cases: result.cases, phase }, 2));
    console.log('PASS local ' + (doc.testId ?? doc.fieldKey));
}
let fixture;
try {
    await start();
    result.embeddedFrontendVerified = await verifyEmbeddedFrontend(base, artifact.frontend);
    delete artifact.frontend;
    await api('/api/library/create', { path: root, operationId: randomUUID() });
    const initialSchema = await api('/api/data/schema', { root }), schemaOperation = randomUUID();
    const prepared = await api('/api/data/schema/prepare', { root, source: initialSchema.source, beforeSource: initialSchema.source, beforeRevision: initialSchema.revision, operationId: schemaOperation });
    await api('/api/data/schema/install', { operationId: schemaOperation, planId: prepared.planId });
    const orgA = await capture('Org A', ['organization'], {}), orgB = await capture('Org B', ['organization'], {}), authorA = await capture('Author A', ['person'], {}), authorB = await capture('Author B', ['person'], {});
    const phones = array(record({ number: text('00138'), label: text('work'), note: text('work original'), imported: text('keep first') }), record({ number: text('00138'), label: text('personal'), note: text('personal original') }));
    const contact = Object.assign(await capture('Contacts', ['person'], { 'contacts/phones': phones }), { fieldKey: 'phones', fieldId: 'contacts/phones', type: 'person' });
    const expectedPhones = structuredClone(phones);
    member(expectedPhones.value[0], 'note').value = 'work revised';
    expectedPhones.value.reverse();
    const employments = array(record({ employer: record({ ref: text(ref(orgA)) }), position: text('Engineer'), during: record({ start: date(2020), end: none }) }), record({ employer: record({ ref: text(ref(orgB)) }), position: text('Editor'), during: record({ start: date(2024, 3), end: none }) }));
    const employment = Object.assign(await capture('Employment', ['person'], { 'people/engagements': employments }), { fieldKey: 'engagements', fieldId: 'people/engagements', type: 'person' });
    const expectedEmployment = structuredClone(employments);
    member(expectedEmployment.value[0], 'position').value = 'Principal Engineer';
    expectedEmployment.value.reverse();
    const publication = record({ kind: text('article'), title: text('Owned paper'), issued: date(2026), contributors: array(record({ agent: record({ ref: text(ref(authorA)) }), credited_as: text('同名'), roles: array(text('author')), note: text('author assertion') }), record({ agent: record({ ref: text(ref(authorB)) }), credited_as: text('同名'), roles: array(text('editor')), note: text('editor assertion') })) });
    const paper = Object.assign(await capture('Paper', ['paper'], { 'library/publication': publication }), { fieldKey: 'publication', fieldId: 'library/publication', type: 'paper' });
    const expectedPaper = structuredClone(publication);
    const contributors = member(expectedPaper, 'contributors');
    member(contributors.value[1], 'note').value = 'editor revised';
    contributors.value.reverse();
    const pairs = [];
    for (const order of ['higher', 'lower']) {
        const notes = [await capture('Relation ' + order + ' 1', ['person'], {}), await capture('Relation ' + order + ' 2', ['person'], {})].sort((a, b) => a.nodeId.localeCompare(b.nodeId));
        let [a, b] = order === 'higher' ? [notes[1], notes[0]] : notes;
        a = await setFixture(a, 'siblings', array(record({ target: record({ ref: text(ref(b)) }), note: text(order + ' author original') })));
        b = await setFixture(b, 'siblings', array(record({ target: record({ ref: text(ref(a)) }), note: text(order + ' target independent') })));
        pairs.push({ order, a, b });
    }
    fixture = { contact, employment, paper, pairs, orgA, orgB, authorA, authorB };
    await writeFile(path.join(out, 'fixture.json'), stringifyServeJson(fixture, 2), { flag: 'wx' });
    await stop();
    browser = await chromium.launch({ headless: true, ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : { channel: 'chrome' }) });
    result.browser = browser.version();
    await localTask(contact, async (selector) => { await (await control(selector, contact.fieldId, [idx(0), mem('note')], 'value:text')).fill('work revised'); await move(selector, contact.fieldId, [idx(1)], 1); }, "phones.exists(p, p.number == '00138' && p.label == 'work' && p.note == 'work revised')", "phones.exists(p, p.label == 'personal' && p.note == 'work revised')", expectedPhones);
    await localTask(employment, async (selector) => { await (await control(selector, employment.fieldId, [idx(0), mem('position')], 'value:text')).fill('Principal Engineer'); await move(selector, employment.fieldId, [idx(1)], 1); }, `engagements.exists(e, e.employer.ref == nodeRef(${JSON.stringify(ref(orgA))}) && e.position == 'Principal Engineer' && e.during.start.year == 2020)`, `engagements.exists(e, e.employer.ref == nodeRef(${JSON.stringify(ref(orgA))}) && e.position == 'Editor' && e.during.start.year == 2024)`, expectedEmployment);
    await localTask(paper, async (selector) => { await (await control(selector, paper.fieldId, [mem('contributors'), idx(1), mem('note')], 'value:text')).fill('editor revised'); await move(selector, paper.fieldId, [mem('contributors'), idx(1)], 1); }, `publication.contributors.exists(c, c.agent.ref == nodeRef(${JSON.stringify(ref(authorB))}) && c.credited_as == '同名' && c.roles.exists(r, r == 'editor') && c.note == 'editor revised')`, `publication.contributors.exists(c, c.agent.ref == nodeRef(${JSON.stringify(ref(authorA))}) && c.roles.exists(r, r == 'editor'))`, expectedPaper);
    // Each relation declaration is authored on its owning local note. A shared
    // graph edge must never merge the two independent assertion records.
    for (const pair of pairs) {
        const beforeB = await readOwnedFile(pair.b.path), doc = Object.assign(pair.a, { fieldKey: 'siblings', fieldId: 'people/siblings', type: 'person', order: pair.order, testId: 'siblings-' + pair.order });
        const expected = array(record({ target: record({ ref: text(ref(pair.b)) }), note: text(pair.order + ' author revised') }));
        pair.expected = expected;
        await localTask(doc, async (selector) => { await (await control(selector, doc.fieldId, [idx(0), mem('note')], 'value:text')).fill(pair.order + ' author revised'); }, `siblings.exists(r, r.target.ref == nodeRef(${JSON.stringify(ref(pair.b))}) && r.note == '${pair.order} author revised')`, `siblings.exists(r, r.target.ref == nodeRef(${JSON.stringify(ref(pair.b))}) && r.note == '${pair.order} target independent')`, expected);
        assert.deepEqual(await readOwnedFile(pair.b.path), beforeB, 'the independent target assertion bytes must not be rewritten');
        Object.assign(result.cases.at(-1), { sourceOwner: ref(doc), target: ref(pair.b), targetBeforeSha256: sha(beforeB), targetAfterSha256: sha(await readOwnedFile(pair.b.path)) });
    }
    // Current field-only authorization deliberately excludes relation writers
    // (SERVER resource/relation boundary). Exercise that real negative boundary;
    // this is not a Hosted relation-write acceptance claim.
    phase = 'hosted-relation-readonly';
    const files = {}, grants = {};
    for (const pair of pairs) {
        for (const [role, doc] of [['A', pair.a], ['B', pair.b]]) {
            const resource = pair.order + role;
            files[resource] = { path: path.relative(root, doc.path).replaceAll('\\', '/'), title: resource, editable: true };
            grants[resource] = { fields: { 'people/siblings': { read: true, write: role === 'A' } } };
        }
    }
    const config = { deployment: 'rich-relations-' + randomUUID(), bind: '127.0.0.1:0', root, adminTokenSha256: sha(tokens.admin), files, users: { alice: { enabled: true, tokenSha256: sha(tokens.alice), grants } } }, configPath = path.join(out, 'hosted.json');
    await writeFile(configPath, JSON.stringify(config, null, 2), { flag: 'wx' });
    await start(['--server-config', configPath]);
    const context = await browser.newContext({ viewport: { width: 1480, height: 1100 } });
    page = await context.newPage();
    observe(page);
    await page.goto(base);
    await page.getByLabel('访问凭据', { exact: true }).fill(tokens.alice);
    await page.getByRole('button', { name: '连接', exact: true }).click();
    await workspaceCommand(page, 'home', '属性');
    await page.getByLabel('选择获权字段', { exact: true }).waitFor();
    const boundary = { flow: 'hosted-relation-readonly-boundary', profiles: [], queries: [], limit: 'Current narrow resource/relation write boundary; explicit field write grant does not supply an authorized relation producer. No Hosted relation write passed.' };
    for (const pair of pairs) {
        for (const [role, doc] of [['A', pair.a], ['B', pair.b]]) {
            const resource = pair.order + role, beforeBytes = await readOwnedFile(doc.path);
            await page.getByLabel('选择获权字段', { exact: true }).selectOption({ label: resource + ' · people/siblings' });
            const reading = responseFor(r => new URL(r.url()).pathname === '/api/hosted/fields/read' && r.request().postDataJSON()?.resource === resource);
            await page.locator('#fields').getByRole('button', { name: '读取字段', exact: true }).click();
            const profile = await (await reading).json();
            assert.equal(profile.field.writable, false);
            assert.equal(profile.basis, null);
            await until(() => page.locator('#fields fieldset').first().evaluate(element => element.disabled === true).catch(() => false), 'read-only relation field surface');
            assert.equal(await (await control('#fields', 'people/siblings', [idx(0), mem('note')], 'value:text')).inputValue(), pair.order + (role === 'A' ? ' author revised' : ' target independent'));
            assert.equal(await page.locator('#fields').getByRole('button', { name: '保存字段', exact: true }).isDisabled(), true);
            if (role === 'A')
                assert.deepEqual(profile.field.cell.value, pair.expected);
            const denial = await api('/api/hosted/fields/write', { resource, fieldId: 'people/siblings', basis: randomUUID(), value: profile.field.cell.value, operationId: randomUUID() }, 'alice', role === 'A' ? 409 : 404);
            if (role === 'A') assert.equal(denial.error.code, 'fieldBasisChanged');
            assert.deepEqual(await readOwnedFile(doc.path), beforeBytes);
            boundary.profiles.push({ resource, explicitWriteGrant: role === 'A', profile, denial, sourceSha256: sha(beforeBytes) });
        }
        await page.getByLabel('查询字段', { exact: true }).selectOption('people/siblings');
        await page.getByLabel('CEL 筛选条件', { exact: true }).fill(`siblings.exists(r, r.note == '${pair.order} author revised')`);
        const query = responseFor(r => new URL(r.url()).pathname === '/api/hosted/query');
        await page.locator('#fields').getByRole('button', { name: '运行查询', exact: true }).click();
        const queryResult = await (await query).json();
        assert.equal(queryResult.result.coverage, 'complete');
        assert.equal(queryResult.result.rows.length, 1);
        assert.equal(queryResult.result.rows[0].entityKey, pair.order + 'A');
        await workspaceCommand(page, 'home', '属性');
        await page.getByRole('table', { name: '获权字段查询结果', exact: true }).waitFor({ state: 'visible' });
        boundary.queries.push(queryResult);
    }
    assert.equal(result.uiRequests.filter(entry => entry.phase === phase && entry.route === '/api/hosted/fields/write').length, 0);
    await context.close();
    page = undefined;
    await stop();
    await start(['--server-config', configPath]);
    for (const entry of boundary.profiles) {
        const reopened = await api('/api/hosted/fields/read', { resource: entry.resource, fieldId: 'people/siblings' }, 'alice');
        assert.deepEqual(reopened.field.cell, entry.profile.field.cell);
        assert.equal(reopened.field.writable, false);
        assert.equal(reopened.basis, null);
        entry.afterProcessRestart = reopened;
    }
    result.cases.push(boundary);
    console.log('PASS hosted relation read-only boundary');
    await stop();
    await until(() => pendingResponses.size === 0, 'response observers');
    assert.deepEqual(result.pageErrors, []);
    assert.deepEqual(result.routeErrors, []);
    assert.equal(await fileSha256(exe), artifact.executableSha256);
    result.harnessAfter = await inventory();
    assert.deepEqual(result.harnessAfter, result.harnessBefore);
    result.completed = true;
}
catch (error) {
    result.failure = { phase, message: error.stack || String(error) };
    process.exitCode = 1;
    if (page && !page.isClosed()) {
        await page.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => { });
        await writeFile(path.join(out, 'failure-body.txt'), await page.locator('body').innerText().catch(() => ''), { flag: 'wx' });
    }
}
finally {
    if (browser) {
        try {
            await browser.close();
            result.browserClosed = true;
        }
        catch (error) {
            result.browserCloseFailure = String(error);
            process.exitCode = 1;
        }
    }
    try {
        await stop();
    }
    catch (error) {
        result.shutdownFailure = String(error.stack || error);
        process.exitCode = 1;
    }
    result.finishedUtc = new Date().toISOString();
    await writeFile(path.join(out, 'result.json'), stringifyServeJson(result, 2), { flag: 'wx' });
}
console.log(JSON.stringify({ completed: result.completed, cases: result.cases.map(c => ({ flow: c.flow, order: c.order })), failure: result.failure, shutdownFailure: result.shutdownFailure, out }));
