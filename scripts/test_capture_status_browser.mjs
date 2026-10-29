// Actual local/hosted mounts and their shared form with controlled lookup ports.
// No native window, real Capture profile, Core effect or creation acceptance.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { pathToFileURL } from "node:url";

const repo = path.resolve(import.meta.dirname, ".."), out = process.env.WEFTEXT_BROWSER_REPORT;
assert.ok(out && path.isAbsolute(out), "WEFTEXT_BROWSER_REPORT must name a new absolute directory");
await mkdir(out);
const files = ["scripts/test_capture_status_browser.mjs", "app/src/capture-form.ts", "app/src/capture-mount.ts", "app/src/capture-hosted-mount.ts", "app/src/capture-controller.ts", "app/src/capture-local.ts", "app/src/capture-hosted.ts", "app/src/ui-messages.ts", "assets/ui-messages.json"];
const inventory = async () => Promise.all(files.map(async file => ({ path: file, sha256: createHash("sha256").update(await readFile(path.join(repo, file))).digest("hex") })));
const result = { scope: "Actual local/hosted Capture mounts, controller and shared form in headless Chrome; controlled original lookup/persistence ports only", checks: [], errors: [], sourceBefore: await inventory(), ownership: { browserClosed: false, viteClosed: false } };
const { chromium } = createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE || "playwright");
const { createServer } = await import(pathToFileURL(path.join(repo, "app/node_modules/vite/dist/node/index.js")).href);
let vite, browser;
const check = async (name, run) => { const item = { name, passed: false }; result.checks.push(item); await run(); item.passed = true; };
try {
  vite = await createServer({ root: path.join(repo, "app"), configFile: false, server: { host: "127.0.0.1", port: 0, hmr: false, watch: { ignored: ["**/*"] } } });
  await vite.listen();
  browser = await chromium.launch(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE, headless: true } : { channel: "chrome", headless: true });
  const page = await browser.newPage();
  page.on("pageerror", error => result.errors.push(error.message));
  await page.route("**/capture-status-fixture", route => route.fulfill({ contentType: "text/html", body: '<!doctype html><meta charset="utf-8"><link rel="stylesheet" href="/src/capture.css"><main id="capture"></main>' }));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/capture-status-fixture`);
  result.browser = await page.evaluate(() => navigator.userAgent);
  await page.evaluate(async () => {
    const [{ mountLocalCapture }, { mountHostedCapture }, capture, { PersonalControlInput }, { PersonalInputRecovery }, { locale }] = await Promise.all([
      import("/src/capture-mount.ts"), import("/src/capture-hosted-mount.ts"), import("/src/capture-draft-state.ts"), import("/src/personal-control-input.ts"), import("/src/personal-input-recovery.ts"), import("/src/ui-locale.ts"),
    ]);
    const host = document.getElementById("capture"), held = [];
    let current;
    const memory = () => { let value; return new PersonalInputRecovery({ async read() { return structuredClone(value); }, async compareAndWrite(_audience, expected, metadata, texts) { value = { commitId: expected + 1, metadata, texts: new Map(texts) }; return expected + 1; } }); };
    const fixture = {
      locale, get current() { return current; },
      async mount(kind, pending = true) {
        current?.mounted.dispose(); current?.inputs?.clear(); host.replaceChildren(); locale.set("zh-CN");
        const draftId = crypto.randomUUID(), operationId = crypto.randomUUID(), device = crypto.randomUUID(), input = capture.emptyCaptureInput();
        input.body = capture.captureText("合成捕获原文 😀");
        let record = { version: 2, draftId, revision: 1, inputRevision: 0, inputCut: "full", lastMutation: { kind: "create", mutationId: crypto.randomUUID(), fingerprint: "a".repeat(64), resultingRevision: 1 }, currentInput: input, target: { rootHint: "fixture-library", parent: { kind: "libraryRoot" }, hostedSlot: null }, pending: pending ? { operationId, requestKind: kind === "local" ? "capture" : "hostedStructure", inputRevision: 0, wire: "{}", wireDigest: "b".repeat(64) } : null };
        const item = { kind, draftId, operationId, mode: "committed", calls: [], active: true, mounted: null, inputs: null };
        const view = () => ({ recovery: { draftId, revision: record.revision, inputRevision: record.inputRevision, inputCut: record.inputCut, lastMutation: record.lastMutation, ...capture.captureAuthorInput(record.currentInput), pending: record.pending ? { operationId: record.pending.operationId, requestKind: record.pending.requestKind, inputRevision: record.pending.inputRevision } : null }, active: { slot: "approved", textInputs: Object.fromEntries(capture.captureTextSlots.map(slot => [slot, record.currentInput[slot]])), bodyMode: record.currentInput.bodyMode, typeIds: [], fields: [], templateId: null, formBasis: null, rawInputs: [], detachedRawInputs: [] } });
        const fact = (id, status) => ({ operationId: id, status, resource: "approved", commitAllowed: false });
        const request = async (route, body) => {
          item.calls.push({ route, body: structuredClone(body) });
          if (route === "/api/capture/identity") return { id: device, isCurrent: true };
          if (route === "/api/data/schema") return { schema: { definitions: {}, aliases: {}, types: [], fields: [] }, roleRef: null, semanticDigest: "c".repeat(64) };
          if (route.endsWith("/draft/read")) return kind === "local" ? structuredClone(record) : view();
          if (["/api/operations/status", "/api/operations/settle", "capture/draft/operation", "capture/draft/settle"].includes(route)) {
            const originalId = body.operationId ?? record.pending?.operationId;
            if (item.mode === "failure") throw new Error("Controlled lookup transport failure");
            if (item.mode === "held") return new Promise((resolve, reject) => held.push({ resolve: status => resolve(fact(originalId, status)), reject, owner: item }));
            return fact(item.mode === "wrong" ? crypto.randomUUID() : originalId, item.mode === "unknown" ? "unknown" : "committed");
          }
          if (route.endsWith("/draft/input") || route.endsWith("/draft/advance")) {
            if (body.expectedRevision !== record.revision) throw new Error("Controlled Capture CAS mismatch");
            const advance = route.endsWith("/advance"), nextRevision = record.revision + 1;
            record = { ...record, revision: nextRevision, inputRevision: record.inputRevision + (advance ? 0 : 1), currentInput: advance ? record.currentInput : structuredClone(body.input), pending: advance ? null : record.pending, lastMutation: { kind: advance ? "advance" : "input", mutationId: body.mutationId, fingerprint: advance ? "d".repeat(64) : await capture.captureInputFingerprint(body.expectedRevision, body.input), resultingRevision: nextRevision } };
            return kind === "local" ? structuredClone(record) : view();
          }
          throw new Error(`Unexpected controlled Capture route ${route}`);
        };
        if (kind === "local") item.mounted = await mountLocalCapture(host, { request, options: { draftId, recovery: memory() } });
        else {
          const audience = JSON.stringify(["status-fixture", "actor"]), inputs = new PersonalControlInput(document, () => item.active, () => { throw new Error("No generic input persistence"); });
          item.inputs = inputs;
          item.mounted = await mountHostedCapture(host, { request, inputs, authorInputs: (owner, available, sink, revision) => inputs.session(audience, 1, owner, available, sink, revision), options: { deployment: "status-fixture", actor: "actor", draftId, recovery: memory(), current: () => item.active }, catalog: { scope: "configuredAuthorizedStructure", coverage: "complete", sources: [], templates: [], slots: [{ resource: "approved", title: "Synthetic allowed location", parent: "fixture-parent", create: true, receive: false, status: "ready", namePolicy: { kind: "fixed", name: "one" } }] }, async deliverCommitted() { throw new Error("No actual Library delivery in this component fixture"); } });
        }
        item.snapshot = () => ({ draft: item.mounted.session.controller.state.draft, input: item.mounted.session.controller.state.input, calls: structuredClone(item.calls) });
        item.changeOriginal = async () => { record.pending = { ...record.pending, operationId: crypto.randomUUID() }; const controller = item.mounted.session.controller; controller.edit(controller.state.input); await controller.drain(); return record.pending.operationId; };
        item.advance = async () => { await item.mounted.session.controller.advance(record.pending.operationId, false); };
        current = item;
        return { draftId, operationId };
      },
      release(status = "committed") { const entry = held.shift(); if (!entry) throw new Error("No held lookup"); entry.resolve(status); },
      get heldCount() { return held.length; },
      retire() { const item = current; if (item.kind === "hosted") { item.active = false; item.mounted.retireAuthorization(); } else item.mounted.dispose(); },
      cleanup() { current?.mounted.dispose(); current?.inputs?.clear(); },
    };
    window.captureStatus = fixture;
  });
  const form = page.locator(".capture-surface"), footer = () => form.locator(":scope > p[data-state]");
  const confirm = async () => { await form.getByRole("button", { name: /^(确认原操作|Check original operation)$/ }).click(); await page.waitForFunction(() => !window.captureStatus.current.mounted.form.pendingAction); };
  const queryRoutes = calls => calls.filter(call => /operations\/(status|settle)$|draft\/(operation|settle)$/.test(call.route));
  for (const kind of ["local", "hosted"]) {
    await check(`${kind}: queried committed original changes only presentation and retains the exact pending/input`, async () => {
      const ids = await page.evaluate(kind => window.captureStatus.mount(kind), kind);
      assert.equal(await footer().getAttribute("data-state"), "pending");
      const before = await page.evaluate(() => window.captureStatus.current.snapshot());
      await confirm();
      assert.equal(await footer().getAttribute("data-state"), "originalChecked");
      assert.equal(await footer().innerText(), "本次保存状态已查询，具体结果见上方。");
      assert.match(await page.locator("#capture").innerText(), new RegExp(`${ids.operationId}[\\s\\S]*committed`));
      const after = await page.evaluate(() => window.captureStatus.current.snapshot());
      assert.deepEqual(after.draft, before.draft); assert.deepEqual(after.input, before.input);
      assert.equal(queryRoutes(after.calls).length, 1);
      assert.equal(after.calls.some(call => /\/(submit|create|commit|advance|freeze)$/.test(call.route)), false);
    });
    await check(`${kind}: locale/update retain control identity and raw input; a same-original lookup failure retains its earlier queried fact`, async () => {
      assert.equal(await page.evaluate(() => { const item = window.captureStatus.current, control = item.mounted.form.element.querySelector("textarea"); window.originalCaptureControl = control; control.value = "未采样原输入 😀"; control.focus(); control.setSelectionRange(2, 4); window.captureStatus.locale.set("en-US"); item.mounted.form.update(item.mounted.session.controller.state); return control === item.mounted.form.element.querySelector("textarea") && control.value === "未采样原输入 😀" && document.activeElement === control && control.selectionStart === 2 && control.selectionEnd === 4; }), true);
      assert.match(await footer().innerText(), /status has been queried/);
      await page.evaluate(() => { window.captureStatus.current.mode = "failure"; });
      await confirm();
      assert.equal(await footer().getAttribute("data-state"), "originalChecked");
      assert.match(await footer().innerText(), /status has been queried/);
      assert.match(await page.locator("#capture").innerText(), /committed/);
      assert.equal(await form.locator(".capture-diagnostic").evaluate(element => element.textContent.length > 0), true);
    });
    await check(`${kind}: new original invalidates the marker; first failure/wrong ID stay pending and unknown remains explicitly unknown`, async () => {
      await page.evaluate(() => window.captureStatus.current.changeOriginal());
      assert.equal(await footer().getAttribute("data-state"), "pending");
      await confirm(); assert.equal(await footer().getAttribute("data-state"), "pending");
      await page.evaluate(() => { window.captureStatus.current.mode = "wrong"; });
      await confirm(); assert.equal(await footer().getAttribute("data-state"), "pending");
      await page.evaluate(() => { window.captureStatus.current.mode = "unknown"; });
      await confirm(); assert.equal(await footer().getAttribute("data-state"), "originalChecked");
      assert.match(await page.locator("#capture").innerText(), /unknown/);
      assert.doesNotMatch(await footer().innerText(), /saved|created|committed/i);
      const before = await page.evaluate(() => window.captureStatus.current.snapshot());
      await page.evaluate(() => window.captureStatus.current.advance());
      assert.equal(await footer().getAttribute("data-state"), "ready");
      await confirm();
      const after = await page.evaluate(() => window.captureStatus.current.snapshot());
      assert.equal(after.draft.pending, null); assert.equal(queryRoutes(after.calls).length, queryRoutes(before.calls).length);
      assert.equal(await footer().innerText(), "Recovery draft saved durably.");
    });
    await check(`${kind}: a late original response cannot mark or paint a replacement pending operation`, async () => {
      await page.evaluate(kind => window.captureStatus.mount(kind), kind);
      await page.evaluate(() => { window.captureStatus.current.mode = "held"; });
      await form.getByRole("button", { name: "确认原操作", exact: true }).click();
      await page.waitForFunction(() => window.captureStatus.heldCount === 1);
      const before = await page.evaluate(async () => { const fixture = window.captureStatus; await fixture.current.changeOriginal(); return fixture.current.snapshot(); });
      await page.evaluate(() => window.captureStatus.release());
      await page.waitForFunction(() => !window.captureStatus.current.mounted.form.pendingAction);
      assert.equal(await footer().getAttribute("data-state"), "pending");
      assert.doesNotMatch(await page.locator("#capture").innerText(), /committed/);
      const after = await page.evaluate(() => window.captureStatus.current.snapshot());
      assert.deepEqual(after.draft, before.draft); assert.deepEqual(after.input, before.input);
    });
    await check(`${kind}: switching drafts or retiring an owner rejects a late lookup without changing the new form`, async () => {
      await page.evaluate(() => { window.captureStatus.current.mode = "held"; });
      await form.getByRole("button", { name: "确认原操作", exact: true }).click();
      await page.waitForFunction(() => window.captureStatus.heldCount === 1);
      await page.evaluate(kind => window.captureStatus.mount(kind), kind);
      const before = await page.evaluate(() => window.captureStatus.current.snapshot());
      await page.evaluate(async () => { window.captureStatus.release(); await new Promise(resolve => setTimeout(resolve, 0)); });
      assert.equal(await footer().getAttribute("data-state"), "pending");
      assert.deepEqual(await page.evaluate(() => window.captureStatus.current.snapshot()), before);
      await page.evaluate(() => { window.captureStatus.current.mode = "held"; });
      await form.getByRole("button", { name: "确认原操作", exact: true }).click();
      await page.waitForFunction(() => window.captureStatus.heldCount === 1);
      await page.evaluate(async () => { window.captureStatus.retire(); window.captureStatus.release(); await new Promise(resolve => setTimeout(resolve, 0)); });
      assert.equal(await page.locator('#capture [data-state="originalChecked"]').count(), 0);
      assert.doesNotMatch(await page.locator("#capture").innerText(), /committed/);
    });
    await check(`${kind}: a draft with no pending operation remains saved and performs no original lookup`, async () => {
      await page.evaluate(kind => window.captureStatus.mount(kind, false), kind);
      await confirm(); assert.equal(await footer().getAttribute("data-state"), "ready");
      assert.equal(queryRoutes(await page.evaluate(() => window.captureStatus.current.snapshot().calls)).length, 0);
    });
  }
  await page.screenshot({ path: path.join(out, "capture-confirmation-status.png"), fullPage: true });
  await page.evaluate(() => window.captureStatus.cleanup());
  assert.deepEqual(result.errors, []);
  result.sourceAfter = await inventory(); assert.deepEqual(result.sourceAfter, result.sourceBefore);
  result.completed = true;
} catch (error) { result.failure = error.stack || String(error); process.exitCode = 1; }
finally {
  if (browser) { await browser.close(); result.ownership.browserClosed = true; }
  if (vite) { await vite.close(); result.ownership.viteClosed = true; }
  await writeFile(path.join(out, "result.json"), JSON.stringify(result, null, 2) + "\n");
}
console.log(JSON.stringify({ completed: result.completed === true, checks: result.checks.filter(check => check.passed).length, out }));
