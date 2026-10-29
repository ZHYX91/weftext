// Actual CaptureFields/Controller, DataPanel and HostedFields lifecycle boundaries.
// Controlled persistence/transport only: no Core save, real ACL or native-host acceptance.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { pathToFileURL } from "node:url";
const repo = path.resolve(import.meta.dirname, ".."), out = process.argv[2];
const fieldId = process.argv[3] ?? "library/publication";
assert.ok(["library/publication", "contacts/names"].includes(fieldId), "Select one actual builtin open-enum field");
const initialText = fieldId === "contacts/names" ? "ordinary" : "article", registeredText = fieldId === "contacts/names" ? "legal" : "book";
assert.ok(out && path.isAbsolute(out)); assert.ok(path.relative(repo, out).startsWith(".." + path.sep));
await mkdir(out);
const files = ["scripts/test_open_enum_consumers_browser.mjs", "docs/design/examples/schema.json", "app/src/native-value-form.ts", "app/src/native-control-identity.ts", "app/src/builtin-property-labels.ts", "app/src/ui-locale.ts", "app/src/ui-messages.ts", "assets/ui-messages.json", "app/src/capture-fields.ts", "app/src/capture-controller.ts", "app/src/capture-draft-state.ts", "app/src/data-panel.ts", "app/src/hosted-fields.ts", "app/src/personal-control-input.ts", "app/src/hosted.ts", "app/src/editor.ts"];
const inventory = async () => Promise.all(files.map(async file => ({ path: file, sha256: createHash("sha256").update(await readFile(path.join(repo, file))).digest("hex") })));
const result = { scope: "Actual Capture submission field gate, DataPanel refresh/Apply/Undo, HostedFields re-read/write and the unchanged hosted beforeunload handler with real PersonalControlInput; controlled transports, no Core/durable-save/ACL/native acceptance", fieldId, cases: [], observations: {}, pageErrors: [], completed: false, sourceBefore: await inventory() };
const { chromium } = createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE || "playwright");
const { createServer } = await import(pathToFileURL(path.join(repo, "app/node_modules/vite/dist/node/index.js")).href);
let vite, browser;
async function check(name, run) { const item = { name, passed: false }; result.cases.push(item); try { await run(); item.passed = true; } catch (error) { item.failure = error.stack || String(error); process.exitCode = 1; } }
try {
    vite = await createServer({ root: path.join(repo, "app"), configFile: false, server: { host: "127.0.0.1", port: 0, hmr: false, watch: { ignored: ["**/*"] } } }); await vite.listen();
    browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE || "C:/Program Files/Google/Chrome/Application/chrome.exe", headless: true });
    result.browser = browser.version();
    const page = await browser.newPage({ viewport: { width: 1250, height: 1000 } }); page.on("pageerror", error => result.pageErrors.push(error.message));
    await page.route("**/enum-consumers", route => route.fulfill({ contentType: "text/html", body: '<!doctype html><meta charset="utf-8"><link rel="stylesheet" href="/src/data-view.css"><style>body{font:14px sans-serif}section{margin:12px}.data-panel{width:650px;max-height:none}#source{height:250px}</style><main><section id="capture"></section><section id="data"></section><section id="source"></section><section id="hosted"></section></main>' }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/enum-consumers`);
    const builtin = JSON.parse(await readFile(path.join(repo, "docs/design/examples/schema.json"), "utf8"));
    const hostedSource = await readFile(path.join(repo, "app/src/hosted.ts"), "utf8"), hookStart = hostedSource.indexOf('window.addEventListener("beforeunload", e => {');
    assert.ok(hookStart >= 0);
    const beforeUnloadHook = hostedSource.slice(hookStart, hostedSource.indexOf("\n});", hookStart) + 4);
    await page.evaluate(async ({ builtin, beforeUnloadHook, fieldId }) => {
        const [{ CaptureFields }, { CaptureController }, capture, { PersonalControlInput }, { DataPanel }, { HostedFields }, { SourceEditor }, native, { locale, t, message }] = await Promise.all([
            import("/src/capture-fields.ts"), import("/src/capture-controller.ts"), import("/src/capture-draft-state.ts"), import("/src/personal-control-input.ts"), import("/src/data-panel.ts"), import("/src/hosted-fields.ts"), import("/src/editor.ts"), import("/src/native-value-form.ts"), import("/src/ui-locale.ts"),
        ]);
        locale.set("en-US");
        const text = value => ({ kind: "text", value }), record = values => ({ kind: "record", value: Object.entries(values).map(([key, value]) => ({ key, value })) });
        const field = builtin.fields.find(field => field.id === fieldId), isName = fieldId === "contacts/names";
        const initial = isName ? { kind: "array", value: [record({ text: text("Owned name"), role: text("ordinary") })] } : record({ kind: text("article"), title: text("Owned paper") });
        const valuePath = [...(isName ? [{ kind: "index", index: 0 }] : []), { kind: "member", key: isName ? "role" : "kind" }];
        const kind = value => (isName ? value.value[0] : value).value.find(member => member.key === (isName ? "role" : "kind")).value.value;
        const typeId = isName ? "people/person" : "library/work", typeAlias = isName ? "person" : "work";
        const hash = async source => [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(source)))].map(value => value.toString(16).padStart(2, "0")).join("");
        const fixture = window.fixture = { kind, locale, initial, labels: { apply: t(message("queryEdit.applyDraft")), reread: t(message("hostedFields.rereadField")), rebase: t(message("hostedFields.adoptCurrentBasis")), save: t(message("hostedFields.saveField")) } };
        const captureInputs = new PersonalControlInput(document, () => true, () => { throw new Error("Capture must retain through its own sink"); });
        const captureFactory = (owner, current, sink, revision) => captureInputs.session("capture-fixture", 1, owner, current, sink, revision);
        const basis = { schemaRef: "weftext-config://fixture/schema", semanticDigest: "a".repeat(64) }, input = capture.emptyCaptureInput();
        input.typeIds = [typeId]; input.formBasis = basis; input.fields = [{ fieldId: field.id, valueState: { kind: "valid", value: structuredClone(initial) } }];
        let durable = { version: 2, draftId: crypto.randomUUID(), revision: 1, inputRevision: 1, inputCut: "full", currentInput: input, pending: null, lastMutation: { kind: "create", mutationId: crypto.randomUUID(), fingerprint: "b".repeat(64), resultingRevision: 1 } };
        const controller = new CaptureController({ async mutate(route, command) {
            if (route !== "input" || command.expectedRevision !== durable.revision) throw new Error("Controlled Capture input CAS only");
            durable = { ...durable, revision: durable.revision + 1, inputRevision: durable.inputRevision + 1, currentInput: structuredClone(command.input), lastMutation: { kind: "input", mutationId: command.mutationId, fingerprint: "b".repeat(64), resultingRevision: durable.revision + 1 } }; return structuredClone(durable);
        }, async flush() { throw new Error("No Core close proof"); } }, durable);
        fixture.capture = { controller, inputs: captureInputs, fields: new CaptureFields(document.querySelector("#capture"), controller, captureFactory, { schema: builtin, ...basis, allowedFields: [field.id] }), async rebuild() {
            this.fields.collectOwnedRaw(); await controller.drain(); this.fields.dispose(); document.querySelector("#capture").replaceChildren();
            this.fields = new CaptureFields(document.querySelector("#capture"), controller, captureFactory, { schema: builtin, ...basis, allowedFields: [field.id] }); await controller.drain();
        }, gate() { this.fields.collectOwnedRaw(); this.fields.assertSubmittable(); return kind(capture.captureFieldCandidate(controller.state.input.fields[0])); } };
        const encode = value => "// controlled field projection " + JSON.stringify(value) + "\n正文😀\r\n", decode = source => JSON.parse(source.split("\n")[0].slice("// controlled field projection ".length));
        fixture.basis = { inputId: "source-input", sessionId: "session", source: encode(initial), serial: 1, baseId: "base", root: "D:/owned-fixture", path: "D:/owned-fixture/Work.typ", workspaceId: "58c47256-bfce-4d29-a037-b83a3a644fb0", nodeId: "32eb9a12-e752-4bfa-8cdf-65e00f23ac26" };
        fixture.editor = new SourceEditor(fixture.basis.source, (source, inputId) => { fixture.basis = { ...fixture.basis, source, inputId, serial: fixture.basis.serial + 1 }; }, () => {}, undefined, fixture.basis.inputId); fixture.editor.mount(document.querySelector("#source"));
        fixture.localRequests = [];
        fixture.data = new DataPanel({ getDocument: () => fixture.basis, canEdit: () => true, notify() {}, async openDocument() { throw new Error("No navigation"); }, async applyPatch(patch, basis) {
            if (basis.source !== fixture.basis.source) throw new Error("Stale controlled patch"); fixture.editor.apply(patch.from, patch.to, patch.text);
        }, async request(route, body) {
            fixture.localRequests.push({ route, body: structuredClone(body) });
            const schemaSource = JSON.stringify(builtin), read = { schema: builtin, source: schemaSource, revision: "schema", semanticDigest: basis.semanticDigest, builtinDefault: false, canInstall: false };
            if (route === "/api/data/schema") return read;
            if (route === "/api/data/references") return { nodes: [], complete: true, diagnostics: [] };
            if (route === "/api/navigation/tags") return { tags: [], complete: true, warnings: [] };
            if (route === "/api/data/project") return { serial: body.serial, sourceHash: await hash(body.source), schemaHash: await hash(schemaSource), schemaRevision: read.revision, readOnly: false, projection: { qualification: "qualified", id: fixture.basis.nodeId, types: [typeAlias], resolvedTypes: [typeId], unknownFields: [], diagnostics: [], budgetExhausted: false, fields: [{ fieldId: field.id, key: field.key, cell: { state: "value", value: decode(body.source) }, sourceRange: { from: 0, to: body.source.length }, raw: JSON.stringify(decode(body.source)), writable: true, required: false, unbound: [] }] } };
            if (route === "/api/data/edit") return { from: 0, to: body.source.length, text: encode(body.value) };
            throw new Error("Unexpected local route " + route);
        } }); document.querySelector("#data").append(fixture.data.element); await fixture.data.refresh();
        const expand = shape => { const value = native.resolveShape(shape, builtin); return value.kind === "record" ? { ...value, members: Object.fromEntries(Object.entries(value.members).map(([key, member]) => [key, { ...member, shape: expand(member.shape) }])) } : value.kind === "list" ? { ...value, items: expand(value.items) } : value.kind === "union" ? { ...value, variants: value.variants.map(expand) } : value; };
        const personalControls = new PersonalControlInput(document, () => true, () => {}), authorInputs = (owner, current, sink, revision) => personalControls.session("hosted-fixture", 1, owner, current, sink, revision);
        let server = structuredClone(initial); fixture.hostedWrites = []; fixture.hostedRegistry = personalControls;
        fixture.hosted = new HostedFields(document.querySelector("#hosted"), async (route, body) => {
            if (route === "fields") return { fields: [{ resource: "work", fieldId: field.id, write: true }] };
            if (route === "fields/read") return { resource: "work", basis: "field-basis", qualification: "staticAuthorizedField", field: { fieldId: field.id, key: field.key, shape: expand(field.shape), nullable: false, required: false, writable: true, cell: { state: "value", value: structuredClone(server) } } };
            if (route === "navigation/registry") throw new Error("No registry rights in this controlled field fixture");
            if (route === "fields/write") { fixture.hostedWrites.push(structuredClone(body)); server = structuredClone(body.value); return { status: "committed", cell: { state: "value", value: server } }; }
            throw new Error("Unexpected hosted route " + route);
        }, undefined, undefined, authorInputs);
        await fixture.hosted.connect("enum-fixture", "alice", 1); await fixture.hosted.openGranted("work", field.id);
        // Execute the unchanged actual hosted consumer, with clean body and every
        // unrelated operation false. The real field registry is its only dirty input.
        new Function("personalInputs", "personalControls", "structures", "media", "library", "history", "draft", beforeUnloadHook)({ pending: () => false }, personalControls, { hasPending: false }, { hasPending: false }, { hasPending: false }, { hasPending: () => false }, { source: "clean body", baseSource: "clean body", pending: null });
        fixture.beforeUnload = () => { const event = new Event("beforeunload", { cancelable: true }); window.dispatchEvent(event); return event.defaultPrevented; };
        fixture.readData = () => kind(fixture.data.drafts.get(field.key).value);
        fixture.readHosted = () => kind(fixture.hosted.draft.value);
        fixture.readSource = () => kind(decode(fixture.editor.source));
        fixture.enumKey = root => [...document.querySelectorAll(root + " [data-control-key]")].filter(control => { if (JSON.parse(control.dataset.controlKey)[0] !== field.id) return false; const address = native.nativeControlAddress(control); return address.identity.fieldId === field.id && JSON.stringify(address.identity.path) === JSON.stringify(valuePath); }).map(control => ({ key: control.dataset.controlKey, role: native.nativeControlAddress(control).role, tag: control.tagName, value: control.value, hidden: control.hidden }));
        await controller.drain();
    }, { builtin, beforeUnloadHook, fieldId });
    const control = async root => {
        const entries = await page.evaluate(root => window.fixture.enumKey(root), root); assert.equal(entries.length, 1, "one current author control per open-enum value");
        return page.locator(`[data-control-key=${JSON.stringify(entries[0].key)}]`).filter({ visible: true }).and(page.locator(root + " input," + root + " textarea"));
    };
    const labels = await page.evaluate(() => window.fixture.labels);
    await check("Capture rebuild reaches the real submission field gate with exactly its current author text", async () => {
        const input = await control("#capture"); await input.fill("custom publication😀"); await input.fill(registeredText);
        await page.evaluate(() => window.fixture.capture.rebuild());
        result.observations.capture = await page.evaluate(() => ({ controls: window.fixture.enumKey("#capture"), value: window.fixture.kind(window.fixture.capture.controller.state.input.fields[0].valueState.value), gate: (() => { try { return window.fixture.capture.gate(); } catch (error) { return String(error); } })() }));
        assert.equal(await page.evaluate(() => window.fixture.capture.gate()), registeredText);
        await (await control("#capture")).fill("未知出版类型\nsecond line😀"); await page.evaluate(() => window.fixture.capture.rebuild());
        assert.equal(await page.evaluate(() => window.fixture.capture.gate()), "未知出版类型\nsecond line😀");
        assert.equal(await (await control("#capture")).inputValue(), "未知出版类型\nsecond line😀");
    });
    await check("DataPanel refresh retains the current legal open text; explicit Apply alone changes shared source and Undo", async () => {
        await (await control("#data")).fill(registeredText); await page.evaluate(() => window.fixture.data.refresh());
        assert.equal(await page.evaluate(() => window.fixture.readData()), registeredText);
        assert.equal(await page.evaluate(() => window.fixture.readSource()), initialText);
        await (await control("#data")).fill("未知自定义\n保留😀"); await page.evaluate(() => window.fixture.data.refresh());
        assert.equal(await (await control("#data")).inputValue(), "未知自定义\n保留😀");
        await page.locator(`#data .data-property[data-field-id="${fieldId}"]`).getByRole("button", { name: labels.apply, exact: true }).click();
        await page.waitForFunction(() => window.fixture.readSource() === "未知自定义\n保留😀");
        await page.evaluate(() => window.fixture.editor.undo()); assert.equal(await page.evaluate(() => window.fixture.readSource()), initialText);
        await page.evaluate(() => window.fixture.editor.redo()); assert.equal(await page.evaluate(() => window.fixture.readSource()), "未知自定义\n保留😀");
    });
    await check("HostedFields ordinary text edits register unapplied author input and trigger the actual clean-body beforeunload condition", async () => {
        assert.equal(await page.evaluate(() => window.fixture.beforeUnload()), false);
        await (await control("#hosted")).fill(registeredText);
        result.observations.hostedRegistry = await page.evaluate(() => ({ registry: window.fixture.hostedRegistry.inspect(), blocksLeave: window.fixture.beforeUnload(), value: window.fixture.readHosted() }));
        assert.equal(await page.evaluate(() => window.fixture.beforeUnload()), true);
        assert.ok((await page.evaluate(() => window.fixture.hostedRegistry.inspect())).producers.some(producer => producer.owner.kind === "field" && producer.unapplied && producer.hasObservedChanges));
        await page.locator("#hosted").getByRole("button", { name: labels.reread, exact: true }).click();
        await page.waitForFunction(() => !window.fixture.hosted.busy);
        assert.equal(await (await control("#hosted")).inputValue(), registeredText);
        await (await control("#hosted")).fill("自定义出版\nactual text😀");
        await page.locator("#hosted").getByRole("button", { name: labels.reread, exact: true }).click(); await page.waitForFunction(() => !window.fixture.hosted.busy);
        assert.equal(await (await control("#hosted")).inputValue(), "自定义出版\nactual text😀");
        await page.locator("#hosted").getByRole("button", { name: labels.rebase, exact: true }).click();
        await page.locator("#hosted").getByRole("button", { name: labels.save, exact: true }).click(); await page.waitForFunction(() => window.fixture.hostedWrites.length === 1 && !window.fixture.hosted.busy);
        assert.equal(await page.evaluate(() => window.fixture.kind(window.fixture.hostedWrites[0].value)), "自定义出版\nactual text😀");
    });
    assert.deepEqual(result.pageErrors, []); result.sourceAfter = await inventory(); assert.deepEqual(result.sourceAfter, result.sourceBefore); result.completed = result.cases.every(item => item.passed);
} catch (error) { result.failure = error.stack || String(error); process.exitCode = 1; }
finally { if (browser) await browser.close(); if (vite) await vite.close(); await writeFile(path.join(out, "result.json"), JSON.stringify(result, null, 2) + "\n", { flag: "wx" }); }
console.log(JSON.stringify({ completed: result.completed, cases: result.cases, observations: result.observations, failure: result.failure, out }));
