// Real browser component checks over current product modules. No Core process
// or OS submission is simulated as passed by this focused UI test.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const out = path.resolve(process.argv[2]);
await mkdir(out, { recursive: false });
const schema = JSON.parse(await readFile(path.join(repo, 'docs/design/examples/schema.json'), 'utf8'));
const result = { mode: 'actual reminder form and B1 controls in a browser; no Core or OS acceptance', sourceHashes: {}, browser: null, cases: [], pageErrors: [], completed: false };
for (const relative of ['scripts/test_task_reminder_form_browser.mjs', 'app/src/reminder-form.ts', 'app/src/reminder-values.ts', 'app/src/native-control-identity.ts', 'app/src/author-input.ts', 'app/src/personal-control-input.ts', 'app/src/ui-locale.ts', 'app/src/ui-messages.ts', 'app/src/style.css', 'app/src/data-view.css', 'docs/design/examples/schema.json']) result.sourceHashes[relative] = createHash('sha256').update(await readFile(path.join(repo, relative))).digest('hex');
let vite, browser, page, controlKeys;
const key = (members, role) => { const value = controlKeys[JSON.stringify([members, role])]; assert.ok(value, 'known actual producer address'); return value; };
const control = (members, role) => page.locator(`[data-control-key=${JSON.stringify(key(members, role))}]`);
const current = () => page.evaluate(() => window.reminderValue);
const translated = key => page.evaluate(async key => { const ui = await import('/src/ui-locale.ts'); return ui.t(ui.message(key)); }, key);
const clickMessage = async key => page.getByRole('button', { name: await translated(key), exact: true }).click();
async function caseCheck(name, run) { await run(); result.cases.push({ name, passed: true }); }
try {
  const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
  vite = await createServer({ root: path.join(repo, 'app'), configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false, watch: { ignored: ['**/*'] } }, plugins: [{ name: 'owned-reminder-test-document', configureServer(server) { server.middlewares.use('/reminder-test.html', (_request, response) => { response.setHeader('Content-Type', 'text/html;charset=utf-8'); response.end('<!doctype html><html><head><meta charset="utf-8"></head><body><main id="test"></main></body></html>'); }); } }] });
  await vite.listen();
  browser = await chromium.launch({ headless: true, ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : {}) }); result.browser = await browser.version();
  page = await browser.newPage(); page.on('pageerror', error => result.pageErrors.push(error.message)); await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/reminder-test.html`);
  await page.evaluate(async()=>{await import('/src/style.css');await import('/src/data-view.css');document.querySelector('#test').className='data-panel';});
  controlKeys = await page.evaluate(async () => { const { nativeControlKey } = await import('/src/native-control-identity.ts'); const roles = [[['kind'], 'select-kind'], [['at'], 'minute-helper'], [['at'], 'explicit-offset'], [['at'], 'exact-instant'], [['before'], 'exact-seconds'], [['before'], 'minute-helper'], [['clock', 'time'], 'exact-clock'], [['clock', 'time'], 'minute-helper'], [['clock', 'fraction'], 'exact-fraction'], [['zone'], 'installed-zone-version'], [['fold'], 'explicit-fold']]; return Object.fromEntries(roles.map(([members, role]) => [JSON.stringify([members, role]), nativeControlKey({ fieldId: 'tasks/reminder', path: members.map(key => ({ kind: 'member', key })) }, role)])); });
  await page.evaluate(async schema => { const { reminderForm } = await import('/src/reminder-form.ts'); const { newReminder } = await import('/src/reminder-values.ts'); window.reminderValue = newReminder('absolute'); window.form = reminderForm(window.reminderValue, schema, value => { window.reminderValue = value; }, { fieldId: 'tasks/reminder', path: [] }); document.querySelector('#test').append(window.form.element); }, schema);
  await caseCheck('new absolute minute selection requires an explicit offset', async () => {
    const originalId = (await current()).value.find(member => member.key === 'id').value.value;
    await control(['at'], 'minute-helper').fill('2026-10-23T16:30'); assert.equal((await current()).value.find(member => member.key === 'at').value.value, '');
    await control(['at'], 'explicit-offset').fill('+08:00'); assert.equal((await current()).value.find(member => member.key === 'at').value.value, '2026-10-23T16:30:00+08:00');
    assert.equal((await current()).value.find(member => member.key === 'id').value.value, originalId);
  });
  await caseCheck('minute and offset edits preserve arbitrary fractional seconds', async () => {
    await control(['at'], 'exact-instant').fill('2026-10-23T17:00:12.123456789123+08:00'); await control(['at'], 'minute-helper').fill('2026-10-24T08:45'); await control(['at'], 'explicit-offset').fill('+05:30');
    assert.equal((await current()).value.find(member => member.key === 'at').value.value, '2026-10-24T08:45:12.123456789123+05:30');
    await page.locator('#test').screenshot({ path: path.join(out, 'absolute-precise.png') });
  });
  await caseCheck('locale changes retain exact input, selection, invalid raw state and identity', async () => {
    const original = await current(); await control(['at'], 'explicit-offset').fill('unfinished offset'); await control(['at'], 'explicit-offset').evaluate(input => { input.focus(); input.setSelectionRange(2, 6); });
    const before = await control(['at'], 'explicit-offset').evaluate(input => ({ value: input.value, from: input.selectionStart, to: input.selectionEnd, message: input.validationMessage }));
    await page.evaluate(async () => { (await import('/src/ui-locale.ts')).locale.set('en-US'); });
    const after = await control(['at'], 'explicit-offset').evaluate(input => ({ value: input.value, from: input.selectionStart, to: input.selectionEnd, message: input.validationMessage, focused: document.activeElement === input }));
    assert.equal(after.value, before.value); assert.equal(after.from, before.from); assert.equal(after.to, before.to); assert.equal(after.focused, true); assert.notEqual(after.message, before.message); assert.deepEqual(await current(), original);
    await control(['at'], 'explicit-offset').fill('+05:30');
  });
  await caseCheck('relative-minute and all-day controls share one intent and preserve finer precision', async () => {
    const originalId = (await current()).value.find(member => member.key === 'id').value.value;
    await control(['kind'], 'select-kind').selectOption('due_before'); await control(['before'], 'exact-seconds').fill('1812.123456789123'); await control(['before'], 'minute-helper').fill('20'); assert.equal((await current()).value.find(member => member.key === 'before').value.value, '1212.123456789123');
    await control(['kind'], 'select-kind').selectOption('due_day'); assert.equal((await current()).value.find(member => member.key === 'zone').value.value, '');
    await control(['clock', 'time'], 'exact-clock').fill('09:00:12'); await control(['clock', 'fraction'], 'exact-fraction').fill('0.123456789123'); await control(['clock', 'time'], 'minute-helper').fill('08:45'); await control(['zone'], 'installed-zone-version').selectOption(JSON.stringify(['UTC', 'fixed-utc.v1']));
    const value = await current(), clock = value.value.find(member => member.key === 'clock').value; assert.equal(clock.value.find(member => member.key === 'time').value.value, '08:45:12'); assert.equal(clock.value.find(member => member.key === 'fraction').value.value, '0.123456789123'); assert.equal(value.value.find(member => member.key === 'id').value.value, originalId);
    await page.evaluate(async () => { (await import('/src/ui-locale.ts')).locale.set('zh-CN'); }); assert.deepEqual(await current(), value);
    await page.locator('#test').screenshot({ path: path.join(out, 'civil-day-precise.png') });
  });
  await page.evaluate(() => { window.form.dispose(); document.querySelector('#test').replaceChildren(); });
  await caseCheck('before-due minute edits retain invalid raw and refuse codec or range overflow', async () => {
    const original = '0.' + '0'.repeat(4093) + '1';
    await page.evaluate(async ({ schema, original }) => { const { reminderForm } = await import('/src/reminder-form.ts'); const { newReminder, setReminderMember } = await import('/src/reminder-values.ts'); window.reminderValue = setReminderMember(newReminder('due_before'), 'before', { kind: 'text', value: original }); window.form = reminderForm(window.reminderValue, schema, value => { window.reminderValue = value; }, { fieldId: 'tasks/reminder', path: [] }); document.querySelector('#test').append(window.form.element); }, { schema, original });
    await control(['before'], 'minute-helper').fill('1');
    assert.equal(await control(['before'], 'exact-seconds').inputValue(), original); assert.equal((await current()).value.find(item => item.key === 'before').value.value, original); assert.equal(await control(['before'], 'minute-helper').evaluate(input => input.validity.valid), false);
    await control(['before'], 'exact-seconds').fill('-0.000'); await control(['before'], 'minute-helper').fill('1'); assert.equal((await current()).value.find(item => item.key === 'before').value.value, '60.000');
    await control(['before'], 'exact-seconds').fill('1812.'); await control(['before'], 'minute-helper').fill('20'); assert.equal(await control(['before'], 'exact-seconds').inputValue(), '1812.'); assert.equal((await current()).value.find(item => item.key === 'before').value.value, '60.000');
    await control(['before'], 'exact-seconds').fill('0.1'); await control(['before'], 'minute-helper').fill('2147483647'); assert.equal(await control(['before'], 'exact-seconds').inputValue(), '0.1'); assert.equal((await current()).value.find(item => item.key === 'before').value.value, '0.1'); assert.equal(await control(['before'], 'minute-helper').evaluate(input => input.validity.valid), false);
    await control(['before'], 'exact-seconds').fill('1812.123456789123'); await control(['before'], 'minute-helper').fill('20'); assert.equal((await current()).value.find(item => item.key === 'before').value.value, '1212.123456789123');
    await page.evaluate(() => { window.form.dispose(); document.querySelector('#test').replaceChildren(); });
  });
  await caseCheck('actual B1 bindings restore exact raw without proof and retain semantic addresses across locale changes', async () => {
    const actual = await page.evaluate(async schema => {
      const { reminderForm } = await import('/src/reminder-form.ts'), { newReminder, setReminderMember } = await import('/src/reminder-values.ts');
      const { nativeControlAddress, nativeControlKey } = await import('/src/native-control-identity.ts'), { PersonalControlInput } = await import('/src/personal-control-input.ts');
      window.captureSnapshots = []; window.genericProof = []; window.bindingOrder = []; window.formChanges = 0; window.authorCurrent = true;
      window.proofs = new PersonalControlInput(document, () => true, input => window.genericProof.push(structuredClone(input)));
      window.realSession = window.proofs.session('owned-ui-test', 1, { kind: 'capture', draftId: 'owned-ui-draft' }, () => window.authorCurrent, input => window.captureSnapshots.push(structuredClone(input)));
      window.recordingSession = { bind: (control, slot, path) => { window.bindingOrder.push({ type: 'bind', slot, path: structuredClone(path) }); return window.realSession.bind(control, slot, path); }, invalidate: (slot, path) => { window.bindingOrder.push({ type: 'invalidate', slot, path: structuredClone(path) }); window.realSession.invalidate(slot, path); }, capture: control => window.realSession.capture(control), retire: () => { window.bindingOrder.push({ type: 'retire' }); window.realSession.retire(); } };
      window.bindingIdentity = { fieldId: 'tasks/reminder', path: [{ kind: 'member', key: 'nested' }, { kind: 'index', index: 2 }], variants: [2, 1] };
      window.reminderValue = setReminderMember(newReminder('absolute'), 'at', { kind: 'text', value: '2026-10-23T17:00:12.123456789123+08:00' });
      window.form = reminderForm(window.reminderValue, schema, value => { window.reminderValue = value; window.formChanges++; window.bindingOrder.push({ type: 'changed' }); }, window.bindingIdentity, { session: window.recordingSession, slot: 'capture-reminder' }); document.querySelector('#test').append(window.form.element);
      window.boundExact = [...window.form.element.querySelectorAll('input')].find(control => nativeControlAddress(control).role === 'exact-instant');
      const address = nativeControlAddress(window.boundExact), raw = { path: address.identity.path, role: address.role, text: 'unfinished\ud800raw' };
      const duplicate = window.form.restoreRaw([raw, { ...raw, text: 'other' }, { path: address.identity.path, role: 'not-a-control', text: 'unknown' }]);
      const restored = window.form.restoreRaw([raw]); const minute = [...window.form.element.querySelectorAll('input')].find(control => nativeControlAddress(control).role === 'minute-helper');
      const unrepresentable = window.form.restoreRaw([{ path: nativeControlAddress(minute).identity.path, role: 'minute-helper', text: 'invalid\ud800minute' }]);
      window.boundExact.focus(); window.boundExact.setSelectionRange(2, 6);
      return { key: address.key, expectedKey: nativeControlKey(address.identity, address.role), path: address.identity.path, variants: address.identity.variants, rawUnits: Array.from({ length: window.boundExact.value.length }, (_, at) => window.boundExact.value.charCodeAt(at)), valid: window.boundExact.validity.valid, duplicate: duplicate.rejected.map(item => item.reason), applied: restored.applied.length, unrepresentable: unrepresentable.rejected.map(item => item.reason), changes: window.formChanges, capture: window.captureSnapshots.length, generic: window.genericProof.length, bindingCount: window.bindingOrder.length, validation: window.boundExact.validationMessage };
    }, schema);
    assert.equal(actual.key, actual.expectedKey); assert.deepEqual(actual.path, [{ kind: 'member', key: 'nested' }, { kind: 'index', index: 2 }, { kind: 'member', key: 'at' }]); assert.deepEqual(actual.variants, [2, 1]); assert.deepEqual(actual.rawUnits, [...'unfinished'].map(char => char.charCodeAt(0)).concat([0xd800], [...'raw'].map(char => char.charCodeAt(0)))); assert.equal(actual.valid, false); assert.deepEqual(actual.duplicate, ['duplicate', 'duplicate', 'unknown']); assert.equal(actual.applied, 1); assert.deepEqual(actual.unrepresentable, ['unrepresentable']); assert.equal(actual.changes, 0); assert.equal(actual.capture, 0); assert.equal(actual.generic, 0);
    await page.evaluate(async () => { (await import('/src/ui-locale.ts')).locale.set('en-US'); });
    const after = await page.evaluate(() => ({ same: document.activeElement === window.boundExact, from: window.boundExact.selectionStart, to: window.boundExact.selectionEnd, units: Array.from({ length: window.boundExact.value.length }, (_, at) => window.boundExact.value.charCodeAt(at)), changes: window.formChanges, capture: window.captureSnapshots.length, count: window.bindingOrder.length, validation: window.boundExact.validationMessage }));
    assert.equal(after.same, true); assert.equal(after.from, 2); assert.equal(after.to, 6); assert.deepEqual(after.units, actual.rawUnits); assert.equal(after.changes, 0); assert.equal(after.capture, 0); assert.equal(after.count, actual.bindingCount); assert.notEqual(after.validation, actual.validation);
    const exact = page.locator(`[data-control-key=${JSON.stringify(actual.key)}]`); await exact.press('Control+A'); await exact.press('Backspace'); await exact.pressSequentially('2026-10-23T17:00:12.123456789123+08:00');
    const authored = await page.evaluate(() => ({ snapshot: window.captureSnapshots.at(-1), rawWasProof: window.captureSnapshots.some(input => input.text.includes('\ud800')), generic: window.genericProof.length, before: window.bindingOrder.filter(item => item.type === 'bind').map(item => ({ slot: item.slot, path: item.path })) }));
    assert.equal(authored.snapshot.context.owner.kind, 'capture'); assert.deepEqual(authored.snapshot.context.path, actual.path); assert.equal(authored.snapshot.context.slot, JSON.stringify(['capture-reminder', 'exact-instant'])); assert.equal(authored.snapshot.rawValue, '2026-10-23T17:00:12.123456789123+08:00'); assert.equal(authored.rawWasProof, false); assert.equal(authored.generic, 0);
  });
  await caseCheck('branch changes invalidate old leases first and retired selects cannot mutate a connected form', async () => {
    const modeKey = await page.evaluate(async () => { const { nativeControlAddress } = await import('/src/native-control-identity.ts'); window.oldMode = [...window.form.element.querySelectorAll('select')].find(control => nativeControlAddress(control).role === 'select-kind'); window.oldExact = window.boundExact; window.oldLease = window.realSession.capture(window.oldExact); window.bindingOrder = []; return nativeControlAddress(window.oldMode).key; });
    await page.locator(`[data-control-key=${JSON.stringify(modeKey)}]`).selectOption('due_day');
    const branch = await page.evaluate(() => { const first = window.bindingOrder.findIndex(item => item.type === 'changed'); const prior = window.bindingOrder.slice(0, first); const before = JSON.stringify(window.reminderValue), snapshots = window.captureSnapshots.length; const applied = window.oldLease?.apply('late clipboard'); window.oldMode.value = 'due_before'; window.oldMode.dispatchEvent(new Event('change', { bubbles: true })); window.oldExact.value = 'late terminal'; window.oldExact.dispatchEvent(new InputEvent('input', { bubbles: true, data: 'late terminal', inputType: 'insertCompositionText' })); return { first, prior, applied, same: JSON.stringify(window.reminderValue) === before, snapshots: window.captureSnapshots.length === snapshots }; });
    assert.ok(branch.first > 0); assert.equal(branch.prior.length, 3); assert.ok(branch.prior.every(item => item.type === 'invalidate')); assert.equal(branch.applied, false); assert.equal(branch.same, true); assert.equal(branch.snapshots, true);
    await page.evaluate(async () => { const { nativeControlAddress } = await import('/src/native-control-identity.ts'); window.oldZone = [...window.form.element.querySelectorAll('select')].find(control => nativeControlAddress(control).role === 'installed-zone-version'); window.oldFold = [...window.form.element.querySelectorAll('select')].find(control => nativeControlAddress(control).role === 'explicit-fold'); });
    await page.locator(`[data-control-key=${JSON.stringify(modeKey)}]`).selectOption('absolute');
    const retired = await page.evaluate(async () => { const { nativeControlAddress } = await import('/src/native-control-identity.ts'); const before = JSON.stringify(window.reminderValue), changes = window.formChanges; window.oldZone.value = JSON.stringify(['UTC', 'fixed-utc.v1']); window.oldZone.dispatchEvent(new Event('change', { bubbles: true })); window.oldFold.value = 'earlier'; window.oldFold.dispatchEvent(new Event('change', { bubbles: true })); const mode = [...window.form.element.querySelectorAll('select')].find(control => nativeControlAddress(control).role === 'select-kind'); window.form.dispose(); const connected = mode.isConnected; mode.value = 'due_before'; mode.dispatchEvent(new Event('change', { bubbles: true })); const result = { connected, same: JSON.stringify(window.reminderValue) === before, changes: window.formChanges === changes, sharedSessionRetired: window.bindingOrder.some(item => item.type === 'retire') }; window.authorCurrent = false; window.realSession.retire(); window.proofs.clear(); document.querySelector('#test').replaceChildren(); (await import('/src/ui-locale.ts')).locale.set('zh-CN'); return result; });
    assert.equal(retired.connected, true); assert.equal(retired.same, true); assert.equal(retired.changes, true); assert.equal(retired.sharedSessionRetired, false);
  });
  assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) { result.error = error.stack || String(error); throw error; }
finally { if (browser) await browser.close(); if (vite) await vite.close(); await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n'); }
