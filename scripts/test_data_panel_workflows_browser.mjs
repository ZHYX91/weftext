// Actual DataPanel, body dialogs, FieldTransfer and SourceEditor history.
// Bounded protocol host only: no Core transform/durable save/native/OS IME claim.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { pathToFileURL } from "node:url";

const repo = path.resolve(import.meta.dirname, ".."), out = process.argv[2];
assert.ok(out && path.isAbsolute(out)); assert.ok(path.relative(repo, out).startsWith(".." + path.sep));
await mkdir(out);
const files = ["scripts/test_data_panel_workflows_browser.mjs", "app/src/data-panel.ts", "app/src/value-presets.ts", "app/src/field-transfer.ts", "app/src/field-transfer-state.ts", "app/src/proposal-close-guard.ts", "app/src/data-projection.ts", "app/src/data-projection-read.ts", "app/src/native-value-form.ts", "app/src/native-control-identity.ts", "app/src/editor.ts", "app/src/session.ts", "app/src/workspace-tools.ts", "app/src/ui-locale.ts", "app/src/ui-messages.ts", "assets/ui-messages.json", "app/src/data-view.css"];
const inventory = async () => Promise.all(files.map(async file => ({ path: file, sha256: createHash("sha256").update(await readFile(path.join(repo, file))).digest("hex") })));
const result = { scope: "Actual DataPanel/preset body dialog/FieldTransfer and SourceEditor; bounded exact protocol host, no Core/durable-save/native/OS IME acceptance", cases: [], completed: false, pageErrors: [], sourceBefore: await inventory(), ownership: { browserClosed: false, viteClosed: false } };
const { chromium } = createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE || "playwright");
const { createServer } = await import(pathToFileURL(path.join(repo, "app/node_modules/vite/dist/node/index.js")).href);
let vite, browser;
async function check(name, run) { const entry = { name, passed: false }; result.cases.push(entry); await run(); entry.passed = true; }
try {
    vite = await createServer({ root: path.join(repo, "app"), configFile: false, server: { host: "127.0.0.1", port: 0, hmr: false, watch: { ignored: ["**/*"] } } }); await vite.listen();
    browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE || "C:/Program Files/Google/Chrome/Application/chrome.exe", headless: true }); result.browserVersion = browser.version();
    const page = await browser.newPage({ viewport: { width: 1200, height: 1000 } }); page.on("pageerror", error => result.pageErrors.push(error.message));
    await page.route("**/panel-workflows", route => route.fulfill({ contentType: "text/html", body: '<!doctype html><meta charset="utf-8"><link rel="stylesheet" href="/src/data-view.css"><style>body{margin:0;font:14px sans-serif}main{display:grid;grid-template-columns:minmax(0,1fr) 450px;gap:16px;padding:16px}#editor{height:500px;min-width:0}.data-panel{width:450px;height:920px;box-sizing:border-box}dialog{box-sizing:border-box}dialog .data-input{box-sizing:border-box;max-width:100%}.data-field-transfer{padding:12px 0;border-top:1px solid #ddd}@media(max-width:650px){main{grid-template-columns:minmax(0,1fr)}.data-panel{width:100%;max-height:none}}</style><main><section id="editor"></section><section id="panel"></section></main>' }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/panel-workflows`);
    await page.evaluate(async () => {
        const [{ DataPanel }, { SourceEditor }, { locale, uiTextValue }, { nativeFormControls }, { DocumentSession }, { WorkspaceTools }] = await Promise.all([import("/src/data-panel.ts"), import("/src/editor.ts"), import("/src/ui-locale.ts"), import("/src/native-value-form.ts"), import("/src/session.ts"), import("/src/workspace-tools.ts")]);
        locale.set("zh-CN");
        const text = value => ({ kind: "text", value }), list = { kind: "list", items: { kind: "scalar", semantic: "text" }, min: 0, max: null };
        const encode = values => "// controlled projection fixture " + JSON.stringify(values) + "\n= Native author😀\nOriginal body retained.\n";
        const decode = source => JSON.parse(source.split("\n")[0].slice("// controlled projection fixture ".length));
        const hash = async source => [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(source)))].map(value => value.toString(16).padStart(2, "0")).join("");
        window.fixture = {
            owner: null, editor: null,
            async reset() {
                clearTimeout(this.autoTools?.refreshTimer); clearTimeout(this.ackTimer); this.autoTools = null; this.autoArmed = false;
                this.owner?.dispose(); this.editor?.destroy(); document.querySelector("#editor").replaceChildren(); localStorage.clear();
                this.calls = []; this.patches = []; this.notifications = []; this.current = true; this.editable = true; this.readOnly = false; this.failAck = false;
                this.holdTransfer = false; this.holdDelivery = false; this.transferWaiting = false; this.deliveryWaiting = false; this.holdSchema = false; this.schemaWaiting = false; this.holdProject = false; this.projectWaiting = false;
                this.schema = { definitions: {}, aliases: {}, fields: [
                    { id: "fixture/names", key: "names", shape: structuredClone(list), nullable: false },
                    { id: "fixture/phones", key: "phones", shape: structuredClone(list), nullable: false },
                    { id: "fixture/priority", key: "priority", shape: { kind: "scalar", semantic: "int" }, nullable: false },
                    { id: "fixture/title", key: "title", shape: { kind: "scalar", semantic: "text" }, nullable: false },
                ], types: [{ id: "fixture/person", fields: ["names", "phones", "priority", "title"].map(key => ({ id: "fixture/" + key, required: true })), requires: [], conflicts: [] }] };
                this.semanticDigest = "a".repeat(64); this.schemaRevision = "schema-a";
                const source = encode({ names: { kind: "array", value: [text("same"), text("same"), text("tail")] }, phones: { kind: "array", value: [text("destination")] }, priority: { kind: "int", value: "1" }, title: text("Original title") });
                this.initialSource = source;
                this.basis = { inputId: "actual-input", sessionId: "actual-session", source, serial: 1, baseId: "actual-base", path: "D:/owned-fixture/A.typ", root: "D:/owned-fixture", workspaceId: "b9dff61d-f92f-4a50-bdb1-4b3cc70403b5", nodeId: "ac4ce91b-3ec2-43f9-b11d-4c03b490c1ef" };
                this.editor = new SourceEditor(source, (next, inputId) => { this.basis = { ...this.basis, source: next, serial: this.basis.serial + 1, inputId }; }, () => {}, undefined, this.basis.inputId); this.editor.mount(document.querySelector("#editor"));
                const host = {
                    getDocument: () => this.current ? this.basis : null,
                    canEdit: () => this.editable,
                    notify: (value, error) => this.notifications.push({ value: uiTextValue(value), error: !!error }),
                    openDocument: async () => { throw new Error("No navigation in this bounded component"); },
                    applyPatch: async (patch, basis) => {
                        if (basis.inputId !== this.basis.inputId || basis.baseId !== this.basis.baseId || basis.source !== this.basis.source || basis.serial !== this.basis.serial) throw new Error("stale host patch");
                        this.patches.push(structuredClone(patch)); this.editor.apply(patch.from, patch.to, patch.text);
                        if (this.holdDelivery) { this.deliveryWaiting = true; await new Promise(resolve => { this.releaseDelivery = () => { this.deliveryWaiting = false; resolve(); }; }); }
                        if (this.failAck) throw new Error("TEST Draft ACK failed after editor.apply");
                    },
                    request: async (route, body) => {
                        this.calls.push({ route, body: structuredClone(body) });
                        const read = { schema: this.schema, source: JSON.stringify(this.schema), revision: this.schemaRevision, semanticDigest: this.semanticDigest, builtinDefault: false, canInstall: true };
                        if (route === "/api/data/schema") {
                            if (this.holdSchema) { this.schemaWaiting = true; await new Promise(resolve => { this.releaseSchema = () => { this.holdSchema = false; this.schemaWaiting = false; resolve(); }; }); }
                            return read;
                        }
                        if (route === "/api/data/references") return { nodes: [], complete: true, diagnostics: [] };
                        if (route === "/api/navigation/tags") return { tags: [], complete: true, warnings: [] };
                        if (route === "/api/data/project") {
                            const values = decode(body.source);
                            if (this.holdProject) { this.projectWaiting = true; await new Promise(resolve => { this.releaseProject = () => { this.holdProject = false; this.projectWaiting = false; resolve(); }; }); }
                            return { serial: body.serial, sourceHash: await hash(body.source), schemaHash: await hash(read.source), schemaRevision: read.revision, readOnly: this.readOnly,
                                projection: { qualification: "qualified", id: this.basis.nodeId, types: ["fixture/person"], resolvedTypes: ["fixture/person"], unknownFields: [], diagnostics: [], budgetExhausted: false,
                                    fields: this.schema.fields.map(field => ({ fieldId: field.id, key: field.key, cell: { state: "value", value: values[field.key] }, sourceRange: { from: 0, to: body.source.length }, raw: JSON.stringify(values[field.key]), writable: true, required: true, unbound: [] })) } };
                        }
                        if (route === "/api/data/edit") {
                            const values = decode(body.source); values[body.key] = body.value;
                            return { from: 0, to: body.source.length, text: encode(values) };
                        }
                        if (route === "/api/data/transfer") {
                            const values = decode(body.source), from = this.schema.fields.find(field => field.id === body.fromField), to = this.schema.fields.find(field => field.id === body.toField);
                            const moved = values[from.key].value.splice(body.start, body.end - body.start + 1); values[to.key].value.splice(body.to, 0, ...moved);
                            const source = encode(values), transfer = Object.fromEntries(["fromField", "toField", "start", "end", "to"].map(key => [key, body[key]]));
                            const wire = { source, patches: [{ from: 0, to: body.source.length, text: source, expected: body.source, byteFrom: 0, byteTo: new TextEncoder().encode(body.source).length }], patchUnit: "utf16", serial: body.serial, sourceHash: body.sourceHash, schemaRevision: body.schemaRevision, semanticDigest: body.semanticDigest, transfer };
                            if (this.holdTransfer) { this.transferWaiting = true; return await new Promise(resolve => { this.releaseTransfer = () => { this.transferWaiting = false; resolve(wire); }; }); }
                            return wire;
                        }
                        throw new Error("Unexpected controlled route " + route);
                    },
                };
                this.owner = new DataPanel(host); document.querySelector("#panel").replaceChildren(this.owner.element); await this.owner.refresh();
            },
            async automaticAck(mode) {
                await this.reset(); this.editor.destroy(); document.querySelector("#editor").replaceChildren();
                this.ackEvents = []; this.autoReadsSettled = 0; this.pendingAck = null;
                this.realSession = new DocumentSession({ ...this.basis, kind: "managed", source: this.initialSource });
                const sync = () => { const s = this.realSession; this.basis = { ...this.basis, source: s.source, serial: s.serial, inputId: s.inputId, baseId: s.baseId, inputRevision: s.acknowledgedInputRevision ?? undefined }; };
                this.acceptAck = () => {
                    const s = this.realSession, cut = this.pendingAck;
                    if (!cut) return;
                    const before = JSON.stringify([s.source, s.serial, s.inputId, s.baseId]), previous = s.acknowledgedInputRevision;
                    s.acknowledgeDraft(cut.source, cut.serial, cut.revision, cut.inputId); sync(); this.pendingAck = null;
                    this.ackEvents.push({ phase: "ack", before: previous, after: s.acknowledgedInputRevision, stableAuthorCutEqual: before === JSON.stringify([s.source, s.serial, s.inputId, s.baseId]) });
                };
                this.editor = new SourceEditor(this.initialSource, (source, inputId) => {
                    const s = this.realSession; s.edit(source, inputId); sync();
                    if (!this.autoArmed) return;
                    WorkspaceTools.prototype.documentChanged.call(this.autoTools);
                    clearTimeout(this.ackTimer);
                    this.ackTimer = setTimeout(() => {
                        this.pendingAck = { source: s.source, serial: s.serial, inputId: s.inputId, revision: s.issueDraftRevision() };
                        this.ackEvents.push({ phase: "draft", acknowledged: s.acknowledgedInputRevision, schemaWaiting: this.schemaWaiting, projectWaiting: this.projectWaiting });
                        if (mode === "schema" || mode === "project") { this.acceptAck(); if (mode === "schema") this.releaseSchema(); else this.releaseProject(); }
                    }, 450);
                }, () => {}, undefined, this.realSession.inputId);
                this.editor.mount(document.querySelector("#editor"));
                const values = decode(this.initialSource); values.names.value.push(text("previous projection"));
                this.editor.apply(0, this.editor.source.length, encode(values));
                this.pendingAck = { source: this.realSession.source, serial: this.realSession.serial, inputId: this.realSession.inputId, revision: this.realSession.issueDraftRevision() }; this.acceptAck();
                await this.owner.refresh();
                const refresh = this.owner.refresh.bind(this.owner);
                this.owner.refresh = async () => { this.ackEvents.push({ phase: "read", acknowledged: this.realSession.acknowledgedInputRevision }); try { await refresh(); } finally { this.autoReadsSettled++; } };
                this.autoTools = {
                    closeGuard: { frozen: false }, host: { getDocument: () => this.basis, notify() {} }, propertyPanel: this.owner,
                    views: { data: this.owner.element, query: document.createElement("section") }, viewVisible: kind => kind === "data",
                    knowledgePanel: { contextChanged() {}, attachSurface() {}, backgroundEnabled: () => false }, outlinePanel: { invalidate() {} },
                };
                this.holdSchema = mode === "schema"; this.holdProject = mode === "project"; this.autoArmed = true;
            },
            canFreeze() { const frozen = this.owner.freezeForClose(); if (frozen) this.owner.thawAfterClose(); return frozen; },
            authorDrafts() { return [...this.owner.drafts].map(([key, value]) => ({ key, changed: value.changed, revision: value.revision ?? 0, raw: value.rawInputs ?? {} })); },
            authorControls() { return [...this.owner.element.querySelectorAll(".data-property")].flatMap(wrapper => nativeFormControls(wrapper)).length; },
            values() { return decode(this.editor.source); },
            async undo() { this.editor.undo(); await this.owner.refresh(); },
        };
        await window.fixture.reset();
    });
    const panel = page.locator(".data-panel"), field = key => panel.locator(`.data-property[data-field-key="${key}"]`), dialog = () => page.locator("dialog.data-value-preset-dialog");
    const personalName = () => dialog().getByLabel("预设名称", { exact: true }), personalValue = () => dialog().locator(".native-value-editor").first().getByRole("textbox").first();
    const transfer = () => panel.locator(".data-field-transfer"), move = () => transfer().locator('[data-transfer-control="apply"]');
    const count = route => page.evaluate(route => window.fixture.calls.filter(call => call.route === route).length, route);
    const frozen = () => page.evaluate(() => window.fixture.canFreeze());
    async function reset() { await page.evaluate(() => window.fixture.reset()); }
    async function open(key = "names") { await field(key).locator("[data-field-preset]").click(); await dialog().waitFor(); }
    async function close() { await dialog().getByRole("button", { name: "关闭", exact: true }).click(); await dialog().waitFor({ state: "detached" }); }
    async function savePreset() {
        await dialog().getByRole("button", { name: "命名并保存预设", exact: true }).click();
        await page.waitForFunction(() => document.querySelector("dialog.data-value-preset-dialog details.data-value-presets")?.getAttribute("aria-busy") === "false");
    }
    async function save(name, value) { await personalName().fill(name); await personalValue().fill(value); await savePreset(); }
    async function settleTransfer() { await page.waitForFunction(() => window.fixture.owner.element.querySelector(".data-field-transfer")?.getAttribute("aria-busy") === "false" && window.fixture.owner.element.getAttribute("aria-busy") !== "true"); }
    await check("personal dialog inputs and IME stay outside author sampling while its full lifetime blocks native close", async () => {
        const controls = await page.evaluate(() => window.fixture.authorControls()); await open("priority");
        assert.equal(await panel.locator("[data-value-presets]").count(), 0); assert.equal(await frozen(), false);
        await personalName().fill("待保留名称"); await personalValue().fill("-");
        await personalValue().evaluate(element => element.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true, data: "中" })));
        assert.equal(await page.evaluate(() => window.fixture.owner.composing), false);
        assert.equal(await page.evaluate(() => window.fixture.authorControls()), controls);
        assert.equal(await page.evaluate(() => window.fixture.authorDrafts().some(item => item.changed || Object.keys(item.raw).length)), false);
        await close(); assert.equal(await frozen(), true); await open("priority");
        assert.equal(await personalName().inputValue(), "待保留名称"); assert.equal(await personalValue().inputValue(), "-"); await close();
    });
    await check("one dialog supports repeated explicit appends,then the original field Apply produces one real Undo", async () => {
        await reset(); await open(); await save("追加项", "same");
        const append = () => dialog().getByRole("button", { name: "追加预设项", exact: true });
        await append().click(); await append().click(); assert.equal(await count("/api/data/edit"), 0);
        assert.equal(await page.evaluate(() => window.fixture.editor.source === window.fixture.initialSource), true);
        assert.equal(await page.evaluate(() => window.fixture.owner.drafts.get("names").value.value.length), 5);
        await close(); await field("names").getByRole("button", { name: "应用到草稿", exact: true }).click();
        await page.waitForFunction(() => window.fixture.patches.length === 1 && !window.fixture.owner.element.hasAttribute("aria-busy"));
        assert.equal(await count("/api/data/edit"), 1); assert.equal(await page.evaluate(() => window.fixture.values().names.value.length), 5);
        await page.evaluate(() => window.fixture.undo()); assert.equal(await page.evaluate(() => window.fixture.editor.source === window.fixture.initialSource), true);
    });
    await check("preset staging preserves another field's invalid raw and never applies it implicitly", async () => {
        await reset(); await field("priority").locator(".native-value-editor input").fill("-"); await open(); await save("本字段", "configured");
        await dialog().getByRole("button", { name: "追加预设项", exact: true }).click(); await close();
        assert.equal(await field("priority").locator(".native-value-editor input").inputValue(), "-");
        assert.equal(await page.evaluate(() => window.fixture.owner.drafts.get("priority").changed), true);
        assert.equal(await count("/api/data/edit"), 0); assert.equal(await page.evaluate(() => window.fixture.patches.length), 0);
    });
    await check("personal configuration survives storage failure and read-only author state,but stale target revision cannot adopt it", async () => {
        await reset(); await open();
        await page.evaluate(() => { window.fixture.originalSet = Storage.prototype.setItem; Storage.prototype.setItem = function() { throw new DOMException("quota", "QuotaExceededError"); }; });
        await save("未保存", "retained😀"); await close();
        await page.evaluate(() => { Storage.prototype.setItem = window.fixture.originalSet; }); await open();
        assert.equal(await personalName().inputValue(), "未保存"); assert.equal(await personalValue().inputValue(), "retained😀");
        await savePreset();
        await page.evaluate(() => { window.fixture.owner.drafts.get("names").revision = 99; });
        await dialog().getByRole("button", { name: "追加预设项", exact: true }).click();
        assert.equal(await page.evaluate(() => window.fixture.owner.drafts.get("names").changed), false); await close();
        await page.evaluate(async () => { window.fixture.editable = false; await window.fixture.owner.refresh(); }); await open();
        await personalName().fill("只读也可配置"); await savePreset();
        assert.ok(await dialog().getByLabel("已保存的预设", { exact: true }).locator("option").filter({ hasText: "只读也可配置" }).count());
        await dialog().getByRole("button", { name: "追加预设项", exact: true }).click(); assert.equal(await count("/api/data/edit"), 0); await close();
    });
    await check("same-input refresh preserves transfer ordinals and personal input; real source and schema replacement retire old dialogs", async () => {
        await reset(); await transfer().locator('[data-transfer-control="first"]').fill("2"); await transfer().locator('[data-transfer-control="last"]').fill("2");
        await page.evaluate(() => window.fixture.owner.refresh()); assert.equal(await transfer().locator('[data-transfer-control="first"]').inputValue(), "2");
        await open(); await personalName().fill("来源更换前"); await personalValue().fill("session retained");
        await page.evaluate(() => window.fixture.owner.refresh()); assert.equal(await personalName().inputValue(), "来源更换前");
        await page.evaluate(async () => { const f = window.fixture; f.editor.apply(f.editor.source.length, f.editor.source.length, "\nChanged source"); await f.owner.refresh(); });
        assert.equal(await dialog().count(), 0); await open(); assert.equal(await personalName().inputValue(), "来源更换前"); await close();
        await open(); await page.evaluate(async () => { const f = window.fixture; f.schema = structuredClone(f.schema); f.schema.fields[0].shape.items.semantic = "nonempty_text"; f.schemaRevision = "schema-b"; f.semanticDigest = "b".repeat(64); await f.owner.refresh(); });
        assert.equal(await dialog().count(), 0); await open(); assert.equal(await personalName().inputValue(), ""); await close();
    });
    await check("a clean current document can transfer despite another document's cached proposals,and one patch has one real Undo", async () => {
        await reset(); await page.evaluate(() => { window.fixture.owner.draftCache.set("other-note", new Map([["retained", { value: { kind: "text", value: "cached" }, changed: true, rawInputs: { raw: "-" } }]])); });
        await transfer().locator('[data-transfer-control="first"]').fill("2"); await transfer().locator('[data-transfer-control="last"]').fill("3");
        await move().click(); await page.waitForFunction(() => window.fixture.patches.length === 1); await settleTransfer();
        assert.equal(await count("/api/data/transfer"), 1); assert.equal(await page.evaluate(() => window.fixture.values().names.value.length), 1);
        assert.equal(await page.evaluate(() => window.fixture.values().phones.value.length), 3);
        assert.equal(await page.evaluate(() => window.fixture.owner.draftCache.get("other-note").has("retained")), true);
        await page.evaluate(() => window.fixture.undo()); assert.equal(await page.evaluate(() => window.fixture.editor.source === window.fixture.initialSource), true);
    });
    await check("current typed/raw/type/schema proposals disable transfer without discarding or applying another field", async () => {
        for (const kind of ["typed", "raw", "type", "schema"]) {
            await reset();
            if (kind === "typed") await field("title").locator("textarea").fill("pending title");
            if (kind === "raw") await field("priority").locator(".native-value-editor input").fill("-");
            if (kind === "type") { await panel.locator(".data-type-picker > summary").click(); await panel.locator(".data-type-options input").uncheck(); }
            if (kind === "schema") { await panel.locator(".schema-settings > summary").click(); await panel.locator(".schema-source").fill("{ incomplete"); }
            assert.equal(await move().isDisabled(), true, kind); await move().evaluate(element => element.click());
            assert.equal(await count("/api/data/transfer"), 0); assert.equal(await page.evaluate(() => window.fixture.patches.length), 0);
        }
    });
    await check("late transfer preview rejects a new staged raw input and native close waits for the entire request", async () => {
        await reset(); await page.evaluate(() => window.fixture.holdTransfer = true); await move().click(); await page.waitForFunction(() => window.fixture.transferWaiting);
        assert.equal(await frozen(), false); await field("priority").locator(".native-value-editor input").fill("-");
        await page.evaluate(() => window.fixture.releaseTransfer()); await settleTransfer();
        assert.equal(await page.evaluate(() => window.fixture.patches.length), 0); assert.equal(await field("priority").locator(".native-value-editor input").inputValue(), "-");
        assert.equal(await frozen(), false);
    });
    await check("patch delivery remains busy through ACK failure,and refresh uses the actual changed source instead of repeating old ordinals", async () => {
        await reset(); await page.evaluate(() => { window.fixture.holdDelivery = true; window.fixture.failAck = true; });
        await move().click(); await page.waitForFunction(() => window.fixture.deliveryWaiting);
        assert.equal(await frozen(), false); assert.equal(await page.evaluate(() => window.fixture.patches.length), 1);
        await page.evaluate(() => window.fixture.releaseDelivery()); await settleTransfer();
        assert.equal(await page.evaluate(() => window.fixture.owner.basis.source === window.fixture.editor.source), true);
        assert.equal(await page.evaluate(() => window.fixture.owner.projectionBasis.source === window.fixture.editor.source), true);
        assert.equal(await page.evaluate(() => window.fixture.patches.length), 1); assert.equal(await count("/api/data/transfer"), 1);
        assert.equal(await frozen(), true); assert.ok(await page.evaluate(() => window.fixture.notifications.some(item => item.error && item.value.includes("TEST Draft ACK failed"))));
    });
    await check("new input pending projection cannot execute an old transfer; clear retires old author and transfer composition", async () => {
        await reset(); await page.evaluate(() => { const f = window.fixture; f.basis = { ...f.basis, inputId: "replacement-input-same-source", baseId: "replacement-base" }; f.holdSchema = true; f.pendingRefresh = f.owner.refresh(); });
        await page.waitForFunction(() => window.fixture.schemaWaiting); await move().evaluate(element => element.click()); assert.equal(await count("/api/data/transfer"), 0);
        await page.evaluate(async () => { window.fixture.releaseSchema(); await window.fixture.pendingRefresh; });
        await field("priority").locator(".native-value-editor input").evaluate(element => element.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true, data: "中" })));
        assert.equal(await frozen(), false);
        await page.evaluate(async () => { window.fixture.current = false; await window.fixture.owner.refresh(); }); assert.equal(await frozen(), true);
        await reset(); await transfer().locator('[data-transfer-control="first"]').evaluate(element => element.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true, data: "2" })));
        await page.evaluate(async () => { window.fixture.current = false; await window.fixture.owner.refresh(); }); assert.equal(await frozen(), true);
    });
    await check("transfer controls retired during a same-input pending read release only their own composition targets", async () => {
        await reset(); await page.evaluate(() => { window.fixture.holdSchema = true; window.fixture.pendingRefresh = window.fixture.owner.refresh(); });
        await page.waitForFunction(() => window.fixture.schemaWaiting);
        const old = await transfer().locator('[data-transfer-control="first"]').elementHandle();
        await old.evaluate(element => { element.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true, data: "2" })); element.value = "2"; element.dispatchEvent(new Event("input", { bubbles: true })); });
        assert.equal(await old.evaluate(element => element.isConnected), false);
        assert.equal(await page.evaluate(() => window.fixture.owner.composing), false);
        await page.evaluate(async () => { window.fixture.releaseSchema(); await window.fixture.pendingRefresh; });
        assert.equal(await move().isVisible(), true); assert.equal(await frozen(), true); assert.equal(await count("/api/data/transfer"), 0);
    });
    await check("automatic Undo refresh accepts the same author cut when its real Session ACK arrives during schema read", async () => {
        await page.evaluate(() => window.fixture.automaticAck("schema"));
        assert.equal(await field("names").locator(".native-list-item").count(), 4);
        await page.locator("#editor .cm-content[contenteditable=true]").press("Control+z");
        await page.waitForFunction(() => window.fixture.autoReadsSettled === 1 && !window.fixture.schemaWaiting && window.fixture.pendingAck === null);
        result.ackSchema = await page.evaluate(() => ({ events: window.fixture.ackEvents, sourceItems: window.fixture.values().names.value.length, shownItems: window.fixture.owner.element.querySelectorAll('.data-property[data-field-key="names"] .native-list-item').length, accepted: !!window.fixture.owner.acceptedBasis() }));
        assert.equal(result.ackSchema.sourceItems, 3); assert.equal(result.ackSchema.shownItems, 3); assert.equal(result.ackSchema.accepted, true);
    });
    await check("automatic Undo refresh accepts ACK during projection while preserving source, inputId and Base guards", async () => {
        await page.evaluate(() => window.fixture.automaticAck("project")); await page.locator("#editor .cm-content[contenteditable=true]").press("Control+z");
        await page.waitForFunction(() => window.fixture.autoReadsSettled === 1 && !window.fixture.projectWaiting && window.fixture.pendingAck === null);
        result.ackProject = await page.evaluate(() => ({ events: window.fixture.ackEvents, sourceItems: window.fixture.values().names.value.length, shownItems: window.fixture.owner.element.querySelectorAll('.data-property[data-field-key="names"] .native-list-item').length, accepted: !!window.fixture.owner.acceptedBasis() }));
        assert.equal(result.ackProject.sourceItems, 3); assert.equal(result.ackProject.shownItems, 3); assert.equal(result.ackProject.accepted, true);
    });
    await check("late and repeated same-cut ACKs leave the installed projection and exact transfer usable without an explicit refresh", async () => {
        await page.evaluate(() => window.fixture.automaticAck("installed")); await page.locator("#editor .cm-content[contenteditable=true]").press("Control+z");
        await page.waitForFunction(() => window.fixture.autoReadsSettled === 1 && !!window.fixture.pendingAck);
        await page.evaluate(() => { const f = window.fixture; f.acceptAck(); f.pendingAck = { source: f.realSession.source, serial: f.realSession.serial, inputId: f.realSession.inputId, revision: f.realSession.issueDraftRevision() }; f.acceptAck(); });
        result.ackInstalled = await page.evaluate(() => ({ events: window.fixture.ackEvents, accepted: !!window.fixture.owner.acceptedBasis(), reads: window.fixture.autoReadsSettled }));
        assert.equal(result.ackInstalled.accepted, true); assert.equal(result.ackInstalled.reads, 1);
        await page.evaluate(() => { window.fixture.autoArmed = false; }); await move().click();
        await page.waitForFunction(() => window.fixture.patches.length === 1); await settleTransfer(); assert.equal(await count("/api/data/transfer"), 1);
    });
    await reset(); await open(); await save("可读预设", "configured item");
    await page.screenshot({ path: path.join(out, "data-panel-presets-desktop.png"), fullPage: true });
    await page.setViewportSize({ width: 390, height: 850 }); await page.screenshot({ path: path.join(out, "data-panel-presets-narrow.png"), fullPage: true });
    await close(); await page.evaluate(() => { window.fixture.owner.dispose(); window.fixture.editor.destroy(); });
    assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) { result.error = String(error.stack || error); throw error; }
finally { await browser?.close(); result.ownership.browserClosed = true; await vite?.close(); result.ownership.viteClosed = true; result.sourceAfter = await inventory(); result.inputsUnchanged = JSON.stringify(result.sourceBefore) === JSON.stringify(result.sourceAfter); await writeFile(path.join(out, "result.json"), JSON.stringify(result, null, 2)); if (result.completed) assert.ok(result.inputsUnchanged); }
