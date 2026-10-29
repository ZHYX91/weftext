// Actual preset controls and SourceEditor history; controlled local Draft host.
// No Core, disk save, native window, real OS IME or whole-version acceptance.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { pathToFileURL } from "node:url";

const repo = path.resolve(import.meta.dirname, ".."), out = process.argv[2];
assert.ok(out && path.isAbsolute(out), "Pass a new absolute evidence directory");
assert.ok(path.relative(repo, out).startsWith(".." + path.sep), "Evidence stays outside the public repository");
await mkdir(out);
const files = ["scripts/test_value_presets_browser.mjs", "app/src/value-presets.ts", "app/src/native-value-form.ts", "app/src/native-values.ts", "app/src/native-control-identity.ts", "app/src/strict-json.ts", "app/src/editor.ts", "app/src/ui-locale.ts", "app/src/ui-messages.ts", "assets/ui-messages.json"];
const inventory = async () => Promise.all(files.map(async file => ({ path: file, sha256: createHash("sha256").update(await readFile(path.join(repo, file))).digest("hex") })));
const result = { scope: "Actual preset controls and explicit controlled host field application/SourceEditor Undo only; no Core/native/disk-save/OS IME acceptance", cases: [], pageErrors: [], completed: false, sourceBefore: await inventory(), ownership: { browserClosed: false, viteClosed: false } };
const { chromium } = createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE || "playwright");
const { createServer } = await import(pathToFileURL(path.join(repo, "app/node_modules/vite/dist/node/index.js")).href);
let vite, browser, page;
async function check(name, run) {
    const entry = { name, passed: false }; result.cases.push(entry);
    try { await run(); entry.passed = true; }
    catch (error) {
        entry.error = String(error.stack || error);
        if (page && !page.isClosed()) {
            entry.screenshot = `failed-case-${result.cases.length}.png`;
            await page.screenshot({ path: path.join(out, entry.screenshot), fullPage: true });
        }
        throw error;
    }
}
try {
    vite = await createServer({ root: path.join(repo, "app"), configFile: false, server: { host: "127.0.0.1", port: 0, hmr: false, watch: { ignored: ["**/*"] } } });
    await vite.listen();
    browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE || "C:/Program Files/Google/Chrome/Application/chrome.exe", headless: true });
    result.browserVersion = browser.version();
    const browserContext = await browser.newContext({ viewport: { width: 1100, height: 900 } });
    page = await browserContext.newPage();
    page.on("pageerror", error => result.pageErrors.push(error.message));
    await browserContext.route("**/presets-fixture", route => route.fulfill({ contentType: "text/html", body: '<!doctype html><meta charset="utf-8"><link rel="stylesheet" href="/src/data-view.css"><style>body{font:14px sans-serif;margin:16px}main{display:grid;grid-template-columns:minmax(0,450px) minmax(0,1fr);gap:20px}#presets{max-width:450px;min-width:0}#editor{width:100%;height:300px;min-width:0}main>section{min-width:0}@media(max-width:700px){main{grid-template-columns:minmax(0,1fr)}}input,select,textarea{max-width:100%;box-sizing:border-box}details>*{margin:8px 0}.data-button{margin-right:8px}</style><input id="authorRaw" aria-label="Original author raw" value="valid"><main><section id="presets"></section><section><button id="applyField">Explicit host field apply</button><div id="editor"></div></section></main>' }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/presets-fixture`);
    const initializeFixture = async () => {
        const [{ mountValuePresets, createValuePresetState, valuePresetContext, valuePresetStorageKey, ValuePresetStore, validValuePreset }, { SourceEditor }, { locale }, { nativeFormControls }] = await Promise.all([import("/src/value-presets.ts"), import("/src/editor.ts"), import("/src/ui-locale.ts"), import("/src/native-value-form.ts")]);
        locale.set("zh-CN");
        window.fixtureApi = { ValuePresetStore, valuePresetContext, validValuePreset };
        const text = value => ({ kind: "text", value }), scalar = semantic => ({ kind: "scalar", semantic });
        window.fixture = {
            component: null, state: createValuePresetState(), generation: 0, stages: [], editor: null, current: true, permission: true, authorComposing: false, failStore: false,
            mount(kind = "list", { preserveState = false, library = "actual-library-a", digest = "a".repeat(64), detached = false, lockMode = "native" } = {}) {
                this.component?.dispose(); this.editor?.destroy();
                document.querySelector("#editor").replaceChildren();
                this.kind = kind; this.current = true; this.permission = true; this.authorComposing = false; this.failStore = false; this.stages = [];
                document.querySelector("#authorRaw").setCustomValidity("");
                if (!preserveState) this.state = createValuePresetState();
                const recordShape = { kind: "record", unknown: "preserve", members: { number: { shape: scalar("phone"), required: true, nullable: false } } };
                const guardedRecordShape = { kind: "record", unknown: "preserve", members: {
                    n: { shape: scalar("int"), required: true, nullable: false },
                    label: { shape: scalar("text"), required: false, nullable: false },
                    nullable: { shape: scalar("text"), required: true, nullable: true },
                } };
                const unionShape = { kind: "union", variants: [
                    { kind: "record", unknown: "preserve", members: { n: { shape: scalar("int"), required: true, nullable: false } } },
                    { kind: "record", unknown: "preserve", members: { label: { shape: scalar("text"), required: true, nullable: false } } },
                ], discriminator: { kind: "member", keys: ["n", "label"] } };
                const record = suffix => ({ kind: "record", value: [{ key: "number", value: text("00138") }, { key: "old-unbound", value: text(suffix) }] });
                const item = kind === "int" ? scalar("int") : kind === "record" ? recordShape : kind === "record-guard" ? guardedRecordShape : kind === "union" ? unionShape : kind === "node" ? scalar("node_ref") : scalar("text");
                const rootScalar = ["scalar", "instant", "exact_decimal"].includes(kind);
                const shape = rootScalar ? scalar(kind === "scalar" ? "text" : kind) : { kind: "list", items: item, min: 0, max: kind === "max" ? 1 : null };
                this.field = { id: kind === "instant" ? "task.completed_at" : "contact/test", key: kind === "instant" ? "completed_at" : "test", shape, nullable: true }; this.schema = { definitions: {}, fields: [this.field], types: [], aliases: {} }; this.scope = { libraryId: library, schemaDigest: digest };
                this.draft = rootScalar ? text(kind === "scalar" ? "author original" : kind === "instant" ? "2026-10-13T12:30:00+08:00" : "0.30") : kind === "none" ? { kind: "none" } : { kind: "array", value: kind === "record" ? [record("first"), record("second")] : kind === "record-guard" ? [] : kind === "int" ? [{ kind: "int", value: "1" }] : [text("same")] };
                this.originalDraft = structuredClone(this.draft); this.initialSource = "= Original author😀\n" + JSON.stringify(this.draft);
                this.editor = new SourceEditor(this.initialSource, () => {}, () => {}); this.editor.mount(document.querySelector("#editor"));
                this.detached = detached; this.lockMode = lockMode; this.redraw();
            },
            redraw() {
                this.component?.dispose(); const old = document.querySelector("#presets"), host = document.createElement("section"); host.id = "presets";
                if (!this.detached) old.replaceWith(host);
                const epoch = ++this.generation;
                this.component = mountValuePresets(host, {
                    scope: this.scope, field: this.field, schema: this.schema, state: this.state,
                    ...(this.lockMode === "none" ? { locks: null } : {}),
                    storage: { getItem: key => localStorage.getItem(key), setItem: (key, value) => { if (this.failStore) throw new DOMException("quota", "QuotaExceededError"); localStorage.setItem(key, value); } },
                    getDraft: () => this.draft,
                    setDraft: value => { this.stages.push(structuredClone(value)); this.draft = value; this.redraw(); },
                    isCurrent: () => this.current && epoch === this.generation,
                    canApply: () => this.permission && !this.authorComposing && document.querySelector("#authorRaw").validity.valid,
                });
                if (this.detached) old.replaceWith(host);
                host.querySelector("details")?.setAttribute("open", "");
            },
            key() { return valuePresetStorageKey(valuePresetContext(this.scope, this.schema, this.field, this.draft)); },
            corrupt(raw) { localStorage.setItem(this.key(), raw); this.redraw(); },
            storage() { return localStorage.getItem(this.key()); },
            replay(raw) { const control = document.querySelector("#presets .native-value-editor"); const [key] = nativeFormControls(control)[0]; this.component.dispose(); this.state.value = text("old hidden value"); this.state.rawInputs = { [key]: raw }; this.detached = true; this.redraw(); },
        };
        document.querySelector("#applyField").onclick = () => { const f = window.fixture; f.editor.apply(0, f.editor.source.length, "= Original author😀\n" + JSON.stringify(f.draft)); };
        window.fixture.mount();
    };
    await page.evaluate(initializeFixture);
    const host = page.locator("#presets"), name = () => host.getByLabel("预设名称", { exact: true });
    const presetValue = () => host.locator(".native-value-editor").first().getByRole("textbox").first();
    const button = label => host.getByRole("button", { name: label, exact: true });
    const select = () => host.getByLabel("已保存的预设", { exact: true });
    async function mount(kind = "list", options) { await page.evaluate(({ kind, options }) => { localStorage.clear(); window.fixture.mount(kind, options); }, { kind, options }); }
    async function save(label, value) {
        await name().fill(label); await presetValue().fill(value); await button("命名并保存预设").click();
        await page.waitForFunction(label => document.querySelector("#presets details")?.getAttribute("aria-busy") === "false"
            && document.querySelector("#presets > details > [role=status]")?.textContent === "预设已保存在此浏览器。"
            && JSON.parse(window.fixture.storage() || "null")?.presets.some(item => item.name === label), label);
    }
    async function waitForStatus(text) { await page.waitForFunction(text => document.querySelector("#presets > details > [role=status]")?.textContent === text, text); }
    const stages = () => page.evaluate(() => window.fixture.stages);
    await check("record candidate structural actions preserve incomplete sibling raw instead of rebuilding from an older typed value", async () => {
        for (const operation of ["add", "remove", "none"]) {
            await mount("record-guard");
            const rows = host.locator(".native-record").first().locator(":scope > .native-member");
            if (operation === "remove") await rows.nth(1).getByRole("button").click();
            const number = rows.nth(0).getByRole("textbox");
            await number.fill("-");
            const before = await page.evaluate(() => ({ value: structuredClone(window.fixture.state.value), raw: structuredClone(window.fixture.state.rawInputs) }));
            await number.evaluate(element => { window.rawCandidateControl = element; });
            await rows.nth(operation === "none" ? 2 : 1).getByRole("button", { name: operation === "none" ? "设为 none" : operation === "add" ? "添加" : "移除", exact: true }).click();
            assert.equal(await rows.nth(0).getByRole("textbox").inputValue(), "-", operation);
            assert.equal(await rows.nth(0).getByRole("textbox").evaluate(element => element === window.rawCandidateControl), true, `${operation} must not rebuild the editor`);
            assert.equal(await rows.nth(0).getByRole("textbox").evaluate(element => element.validity.valid), false);
            assert.deepEqual(await page.evaluate(() => ({ value: window.fixture.state.value, raw: window.fixture.state.rawInputs })), before, operation);
            await name().fill("incomplete record"); await button("命名并保存预设").click();
            assert.equal(await page.evaluate(() => window.fixture.storage()), null);
            assert.deepEqual(await stages(), []);
        }
    });
    await check("record candidate structural actions reject active composition independently of invalid-raw guards", async () => {
        for (const operation of ["add", "remove", "none"]) {
            await mount("record-guard");
            const rows = host.locator(".native-record").first().locator(":scope > .native-member");
            if (operation === "remove") await rows.nth(1).getByRole("button").click();
            const number = rows.nth(0).getByRole("textbox"); await number.fill("7");
            assert.equal(await number.evaluate(element => element.validity.valid), true);
            await number.evaluate(element => { window.rawCandidateControl = element; element.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true, data: "七" })); });
            const before = await page.evaluate(() => ({ value: structuredClone(window.fixture.state.value), raw: structuredClone(window.fixture.state.rawInputs) }));
            await rows.nth(operation === "none" ? 2 : 1).getByRole("button", { name: operation === "none" ? "设为 none" : operation === "add" ? "添加" : "移除", exact: true }).click();
            assert.equal(await rows.nth(0).getByRole("textbox").evaluate(element => element === window.rawCandidateControl && element.isConnected), true, operation);
            assert.equal(await number.inputValue(), "7");
            assert.deepEqual(await page.evaluate(() => ({ value: window.fixture.state.value, raw: window.fixture.state.rawInputs })), before, operation);
            assert.equal(await page.evaluate(() => window.fixture.storage()), null); assert.deepEqual(await stages(), []);
            await number.evaluate(element => element.dispatchEvent(new CompositionEvent("compositionend", { bubbles: true, data: "七" })));
        }
    });
    await check("task completed_at instant presets preserve twelve fraction digits through save,reload and explicit apply", async () => {
        const instant = "2026-10-13T12:30:00.123456789012+08:00";
        await mount("instant"); await save("准确完成时间", instant);
        assert.deepEqual(await page.evaluate(() => JSON.parse(window.fixture.storage()).presets[0].value), { kind: "text", value: instant });
        await page.evaluate(() => { window.fixture.state = { name: "", rawInputs: {}, selectedId: "" }; window.fixture.redraw(); });
        assert.deepEqual(await page.evaluate(() => JSON.parse(window.fixture.storage()).presets[0].value), { kind: "text", value: instant });
        await select().selectOption({ label: "准确完成时间" }); await button("用预设替换此值").click();
        assert.deepEqual(await stages(), [{ kind: "text", value: instant }]);
        assert.equal(await page.evaluate(() => window.fixture.editor.source === window.fixture.initialSource), true);
        await page.locator("#applyField").click(); assert.equal(await page.evaluate(instant => window.fixture.editor.source.includes(instant), instant), true);
    });
    await check("exact decimal candidate grammar rejects leading plus,missing integer or fraction and the 4097-character boundary", async () => {
        await mount("exact_decimal"); await save("有效准确小数", "-12345678901234567890.0300");
        const original = await page.evaluate(() => window.fixture.storage());
        for (const raw of ["+1", ".5", "1.", "1".repeat(4097)]) {
            await name().fill("invalid exact decimal"); await presetValue().fill(raw); await button("命名并保存预设").click();
            await waitForStatus("预设值不符合当前类型；请完成或修正输入。最终字段约束仍由原应用流程校验。");
            assert.equal(await page.evaluate(() => window.fixture.storage()), original, `invalid raw ${raw.length > 20 ? `${raw.length} characters` : raw}`);
            assert.equal(await presetValue().inputValue(), raw);
        }
        await save("4096字合法边界", "1".repeat(4096));
        assert.equal(await page.evaluate(() => JSON.parse(window.fixture.storage()).presets[1].value.value.length), 4096);
    });
    const peer = await browserContext.newPage();
    peer.on("pageerror", error => result.pageErrors.push(`peer: ${error.message}`));
    await peer.goto(page.url());
    await check("two real same-origin producers serialize writes with navigator.locks and reject the second stale record inside its lock", async () => {
        await mount("scalar");
        const context = await page.evaluate(() => window.fixtureApi.valuePresetContext(window.fixture.scope, window.fixture.schema, window.fixture.field, window.fixture.draft));
        await page.evaluate(context => {
            const { ValuePresetStore } = window.fixtureApi;
            window.producerState = { reads: [], writes: [], lockNames: [], held: false };
            const storage = {
                getItem: key => { const raw = localStorage.getItem(key); window.producerState.reads.push(raw); return raw; },
                setItem: (key, raw) => { window.producerState.writes.push(raw); localStorage.setItem(key, raw); },
            };
            const gate = new Promise(resolve => { window.releaseProducer = resolve; });
            window.producerStore = new ValuePresetStore(context, storage, value => value.kind === "text", {
                run(name, operation) {
                    window.producerState.lockNames.push(name);
                    return navigator.locks.request(name, async () => { window.producerState.held = true; await gate; return operation(); });
                },
            });
        }, context);
        await peer.evaluate(async context => {
            const { ValuePresetStore } = await import("/src/value-presets.ts");
            window.producerState = { reads: [], writes: [] };
            const storage = {
                getItem: key => { const raw = localStorage.getItem(key); window.producerState.reads.push(raw); return raw; },
                setItem: (key, raw) => { window.producerState.writes.push(raw); localStorage.setItem(key, raw); },
            };
            window.producerStore = new ValuePresetStore(context, storage, value => value.kind === "text");
        }, context);
        const before = await Promise.all([page.evaluate(() => window.producerState), peer.evaluate(() => window.producerState)]);
        assert.deepEqual(before.map(item => item.reads), [[null], [null]], "both Stores must read the same old bytes before either producer saves");
        await page.evaluate(() => { window.producerOutcome = window.producerStore.save("first producer", { kind: "text", value: "first exact😀" }).then(preset => ({ saved: preset.name }), error => ({ reason: error.reason })); });
        await page.waitForFunction(() => window.producerState.held);
        await peer.evaluate(() => { window.producerOutcome = window.producerStore.save("second producer", { kind: "text", value: "second exact😀" }).then(preset => ({ saved: preset.name }), error => ({ reason: error.reason })); });
        const key = await page.evaluate(() => window.producerStore.key);
        await peer.waitForFunction(async key => (await navigator.locks.query()).pending.some(lock => lock.name === key), key);
        const queued = await peer.evaluate(async () => ({ locks: await navigator.locks.query(), state: window.producerState }));
        assert.deepEqual(queued.state.reads, [null], "second producer must not check/write storage before acquiring its actual lock");
        assert.deepEqual(queued.state.writes, []);
        await page.evaluate(() => window.releaseProducer());
        const outcomes = await Promise.all([page.evaluate(() => window.producerOutcome), peer.evaluate(() => window.producerOutcome)]);
        assert.deepEqual(outcomes, [{ saved: "first producer" }, { reason: "storageChanged" }]);
        const after = await Promise.all([page.evaluate(() => window.producerState), peer.evaluate(() => window.producerState)]);
        assert.deepEqual(after[0].lockNames, [key]); assert.equal(after[0].writes.length, 1); assert.equal(after[1].writes.length, 0);
        const raw = await page.evaluate(key => localStorage.getItem(key), key);
        assert.equal(await peer.evaluate(key => localStorage.getItem(key), key), raw);
        assert.deepEqual(JSON.parse(raw).presets.map(item => ({ name: item.name, value: item.value })), [{ name: "first producer", value: { kind: "text", value: "first exact😀" } }]);
        result.crossTabStorageLock = { sameOrigin: new URL(page.url()).origin === new URL(peer.url()).origin, before, queued, outcomes, after, raw };
    });
    async function holdMutationLock(key) {
        await peer.evaluate(key => {
            window.mutationLockHeld = false;
            window.mutationLockPromise = navigator.locks.request(key, async () => {
                window.mutationLockHeld = true;
                await new Promise(resolve => { window.releaseMutationLock = resolve; });
            });
        }, key);
        await peer.waitForFunction(() => window.mutationLockHeld);
    }
    async function releaseMutationLock(key) {
        await peer.evaluate(async () => { window.releaseMutationLock(); await window.mutationLockPromise; });
        await page.waitForFunction(async key => {
            const { held, pending } = await navigator.locks.query();
            return ![...held, ...pending].some(lock => lock.name === key);
        }, key);
    }
    await check("mounted save and delete recheck disposal,field retirement and IME after waiting for an actual browser mutation lock", async () => {
        result.queuedMutationGuards = [];
        for (const operation of ["save", "delete"]) {
            for (const guard of ["disposed", "field-retired", "composition-active", "composition-ended"]) {
                await mount("scalar"); await save("existing personal record", "old personal value");
                await name().fill("queued candidate"); await presetValue().fill("kept queued😀");
                const cut = await page.evaluate(() => ({ key: window.fixture.key(), raw: window.fixture.storage() }));
                await holdMutationLock(cut.key);
                await button(operation === "save" ? "命名并保存预设" : "删除所选预设").click();
                await page.waitForFunction(async key => (await navigator.locks.query()).pending.some(lock => lock.name === key), cut.key);
                await page.evaluate(guard => {
                    if (guard === "disposed") window.fixture.component.dispose();
                    if (guard === "field-retired") window.fixture.schema.fields = [];
                    if (guard.startsWith("composition")) {
                        const control = document.querySelector("#presets .native-value-editor input,#presets .native-value-editor textarea");
                        control.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true, data: "中" }));
                        if (guard === "composition-ended") control.dispatchEvent(new CompositionEvent("compositionend", { bubbles: true, data: "中" }));
                    }
                }, guard);
                await releaseMutationLock(cut.key);
                assert.equal(await page.evaluate(key => localStorage.getItem(key), cut.key), cut.raw, `${operation} after ${guard}`);
                assert.equal(await name().inputValue(), "queued candidate"); assert.equal(await presetValue().inputValue(), "kept queued😀");
                assert.deepEqual(await stages(), []); result.queuedMutationGuards.push({ operation, guard, bytesUnchanged: true });
            }
        }
    });
    await check("missing navigator.locks and an explicit null provider disable only personal mutations while loading and applying saved values", async () => {
        await mount("scalar"); await save("readable preset", "saved before locks unavailable");
        const original = await page.evaluate(() => window.fixture.storage());
        await page.evaluate(() => { window.fixture.lockMode = "none"; window.fixture.redraw(); });
        const noLocksPage = await browserContext.newPage();
        noLocksPage.on("pageerror", error => result.pageErrors.push(`no-locks: ${error.message}`));
        await noLocksPage.addInitScript(() => Object.defineProperty(navigator, "locks", { value: undefined, configurable: true }));
        await noLocksPage.goto(page.url()); await noLocksPage.evaluate(initializeFixture);
        await noLocksPage.evaluate(() => window.fixture.mount("scalar"));
        for (const [target, mode] of [[page, "injected-null"], [noLocksPage, "navigator-unavailable"]]) {
            const controls = target.locator("#presets"), targetSelect = controls.getByLabel("已保存的预设", { exact: true });
            assert.equal(await controls.getByRole("button", { name: "命名并保存预设", exact: true }).isDisabled(), true, mode);
            assert.equal(await controls.getByRole("button", { name: "删除所选预设", exact: true }).isDisabled(), true, mode);
            await controls.getByLabel("预设名称", { exact: true }).fill("still editable");
            await controls.locator(".native-value-editor").first().getByRole("textbox").fill("unsaved with no lock😀");
            await targetSelect.selectOption({ label: "readable preset" });
            assert.equal(await controls.getByRole("button", { name: "用预设替换此值", exact: true }).isEnabled(), true, mode);
            await controls.getByRole("button", { name: "用预设替换此值", exact: true }).click();
            assert.deepEqual(await target.evaluate(() => window.fixture.stages), [{ kind: "text", value: "saved before locks unavailable" }], mode);
            assert.equal(await target.evaluate(() => window.fixture.storage()), original, mode);
            const direct = await target.evaluate(async mode => {
                const f = window.fixture, { ValuePresetStore, valuePresetContext } = window.fixtureApi;
                const store = new ValuePresetStore(valuePresetContext(f.scope, f.schema, f.field, f.draft), localStorage, value => value.kind === "text", ...(mode === "injected-null" ? [null] : []));
                store.assertCurrent();
                const reason = async operation => { try { await operation(); return "unexpected write"; } catch (error) { return error.reason; } };
                return { canWrite: store.canWrite, names: store.presets.map(item => item.name), save: await reason(() => store.save("blocked", { kind: "text", value: "blocked" })), remove: await reason(() => store.remove(store.presets[0].id)) };
            }, mode);
            assert.deepEqual(direct, { canWrite: false, names: ["readable preset"], save: "lockUnavailable", remove: "lockUnavailable" }, mode);
            assert.equal(await target.evaluate(() => window.fixture.storage()), original, mode);
        }
        await noLocksPage.close();
    });
    await peer.close();
    await check("a named scalar candidate replaces only the staged field; explicit host apply creates one real Undo", async () => {
        await mount("scalar"); await save("常用说明", "configured😀");
        assert.equal(await page.evaluate(() => window.fixture.editor.source === window.fixture.initialSource), true);
        await button("用预设替换此值").focus(); await page.keyboard.press("Space");
        assert.deepEqual(await stages(), [{ kind: "text", value: "configured😀" }]);
        assert.equal(await page.evaluate(() => window.fixture.editor.source === window.fixture.initialSource), true);
        await page.locator("#applyField").click();
        assert.equal(await page.evaluate(() => window.fixture.editor.source.includes("configured😀")), true);
        await page.evaluate(() => window.fixture.editor.undo());
        assert.equal(await page.evaluate(() => window.fixture.editor.source === window.fixture.initialSource), true);
    });
    await check("the actual append action retains duplicates and each explicit application appends exactly one item", async () => {
        await mount(); await save("相同值", "same");
        await button("追加预设项").click(); await button("追加预设项").click();
        assert.equal((await stages()).length, 2);
        assert.deepEqual(await page.evaluate(() => window.fixture.draft.value.map(item => item.value)), ["same", "same", "same"]);
        assert.equal(await name().inputValue(), "相同值"); assert.equal(await presetValue().inputValue(), "same");
        assert.equal(await page.evaluate(() => window.fixture.editor.source === window.fixture.initialSource), true);
    });
    await check("typed record items append without copying,discarding or deduplicating old unknown members", async () => {
        await mount("record"); await save("办公电话", "00138"); await button("追加预设项").click();
        const draft = await page.evaluate(() => window.fixture.draft);
        assert.deepEqual(draft.value.slice(0, 2), await page.evaluate(() => window.fixture.originalDraft.value));
        assert.deepEqual(draft.value[2], { kind: "record", value: [{ key: "number", value: { kind: "text", value: "00138" } }] });
    });
    await check("browser storage reloads named presets; deleting the preset never deletes author values", async () => {
        await mount(); await save("保留作者", "saved item");
        await page.evaluate(() => { window.fixture.state = { name: "", rawInputs: {}, selectedId: "" }; window.fixture.redraw(); });
        await select().selectOption({ label: "保留作者" }); await button("删除所选预设").click();
        await waitForStatus("所选预设已删除。");
        assert.equal(await select().locator("option").count(), 1); assert.deepEqual(await stages(), []);
        assert.equal(await page.evaluate(() => JSON.stringify(window.fixture.draft) === JSON.stringify(window.fixture.originalDraft)), true);
    });
    await check("invalid numeric raw and synthetic active composition block actions and survive a same-scope redraw", async () => {
        await mount("int"); await save("数字", "7"); await presetValue().fill("-");
        await button("追加预设项").click(); assert.deepEqual(await stages(), []);
        await page.evaluate(() => window.fixture.redraw()); assert.equal(await presetValue().inputValue(), "-");
        await button("命名并保存预设").click(); assert.deepEqual(await stages(), []);
        await presetValue().fill("8");
        await presetValue().evaluate(element => element.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true, data: "中" })));
        await button("追加预设项").click(); assert.deepEqual(await stages(), []);
        await presetValue().evaluate(element => element.dispatchEvent(new CompositionEvent("compositionend", { bubbles: true, data: "中" })));
    });
    await check("storage write failure retains the old bytes,name and typed candidate across a redraw", async () => {
        await mount(); await save("旧预设", "old"); const old = await page.evaluate(() => window.fixture.storage());
        await name().fill("未保存名称"); await presetValue().fill("未保存😀"); await page.evaluate(() => window.fixture.failStore = true);
        await button("命名并保存预设").click();
        await waitForStatus("浏览器未确认预设存储成功；当前输入保留，请重新读取后核对。");
        assert.equal(await page.evaluate(() => window.fixture.storage()), old);
        await page.evaluate(() => window.fixture.redraw());
        assert.equal(await name().inputValue(), "未保存名称"); assert.equal(await presetValue().inputValue(), "未保存😀");
        assert.deepEqual(await stages(), []);
    });
    await check("union choice restoration keeps invalid raw in the visible live branch and blocks a false Save", async () => {
        await mount("union"); await name().fill("分支输入"); await presetValue().fill("-");
        await page.evaluate(() => { window.fixture.detached = true; window.fixture.redraw(); });
        assert.equal(await presetValue().inputValue(), "-");
        assert.equal(await presetValue().evaluate(element => element.validity.valid), false);
        await button("命名并保存预设").click(); assert.equal(await page.evaluate(() => window.fixture.storage()), null);
        assert.equal(await presetValue().inputValue(), "-");
    });
    await check("bad records and external manual preference changes stay untouched and cannot apply a stale choice", async () => {
        await mount(); await page.evaluate(() => window.fixture.corrupt('{"version":1,"version":1}'));
        await name().fill("still editable"); await presetValue().fill("kept");
        assert.equal(await button("命名并保存预设").isDisabled(), true);
        assert.equal(await page.evaluate(() => window.fixture.storage()), '{"version":1,"version":1}');
        await mount(); await save("旧候选", "old"); await page.evaluate(() => { const key = window.fixture.key(); const record = JSON.parse(localStorage.getItem(key)); record.presets = []; localStorage.setItem(key, JSON.stringify(record)); });
        const latest = await page.evaluate(() => window.fixture.storage()); await button("追加预设项").click();
        assert.deepEqual(await stages(), []); assert.equal(await page.evaluate(() => window.fixture.storage()), latest);
    });
    await check("the host's invalid raw,composition,permission,current-owner,hidden and retired guards prevent adoption", async () => {
        for (const guard of ["raw", "composition", "permission", "current", "hidden", "retired"]) {
            await mount(); await save("候选", "candidate");
            await page.evaluate(guard => {
                if (guard === "raw") document.querySelector("#authorRaw").setCustomValidity("incomplete author raw");
                if (guard === "composition") window.fixture.authorComposing = true;
                if (guard === "permission") window.fixture.permission = false;
                if (guard === "current") window.fixture.current = false;
                if (guard === "hidden") document.querySelector("#presets").hidden = true;
                if (guard === "retired") window.fixture.component.dispose();
            }, guard);
            await host.getByRole("button", { name: "追加预设项", exact: true, includeHidden: true }).evaluate(element => element.click()); assert.deepEqual(await stages(), [], guard);
        }
    });
    await check("personal configuration can save while the author is read-only or has incomplete raw,while adoption remains guarded", async () => {
        await mount();
        await page.evaluate(() => { window.fixture.permission = false; window.fixture.authorComposing = true; document.querySelector("#authorRaw").setCustomValidity("author unfinished"); });
        await save("本人配置", "configured independently");
        assert.equal(await page.evaluate(() => JSON.parse(window.fixture.storage()).presets[0].name), "本人配置");
        await button("追加预设项").click(); assert.deepEqual(await stages(), []);
        await button("删除所选预设").click(); await waitForStatus("所选预设已删除。");
        assert.equal(await select().locator("option").count(), 1);
    });
    await check("library and schema partitions clear a reused personal draft state rather than leaking another scope", async () => {
        await mount(); await save("A库", "secret candidate");
        await page.evaluate(() => window.fixture.mount("list", { preserveState: true, library: "actual-library-b" }));
        assert.equal(await name().inputValue(), ""); assert.equal(await select().locator("option").count(), 1); assert.equal(await presetValue().inputValue(), "");
        await page.evaluate(() => window.fixture.mount("list", { preserveState: true, digest: "b".repeat(64) }));
        assert.equal(await select().locator("option").count(), 1); assert.deepEqual(await stages(), []);
    });
    await check("detached construction replays exact raw into the typed candidate before its first Save", async () => {
        await mount("scalar"); await page.evaluate(() => window.fixture.replay("visible restored new😀"));
        assert.equal(await presetValue().inputValue(), "visible restored new😀");
        await name().fill("恢复后保存"); await button("命名并保存预设").click();
        await waitForStatus("预设已保存在此浏览器。");
        assert.equal(await page.evaluate(() => JSON.parse(window.fixture.storage()).presets[0].value.value), "visible restored new😀");
    });
    await check("none,max-size and reference-shaped targets do not silently clear or synthesize an identity", async () => {
        await mount("none"); await save("候选", "candidate"); await button("追加预设项").click(); assert.deepEqual(await stages(), []);
        assert.deepEqual(await page.evaluate(() => window.fixture.draft), { kind: "none" });
        await mount("max"); await save("候选", "candidate"); await button("追加预设项").click(); assert.deepEqual(await stages(), []);
        await mount("node"); assert.equal(await host.locator("input,select,textarea").count(), 0); assert.equal(await host.getByRole("status").textContent(), "此位置或引用类型暂不支持值预设；请沿原字段控件输入。");
    });
    await mount(); await save("截图预设", "configured item");
    await page.screenshot({ path: path.join(out, "value-presets-desktop.png"), fullPage: true });
    await page.setViewportSize({ width: 390, height: 850 });
    await check("narrow personal controls retain readable width and keep horizontal overflow local", async () => {
        assert.ok((await presetValue().boundingBox()).width >= 250);
        assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
    });
    await page.screenshot({ path: path.join(out, "value-presets-narrow.png"), fullPage: true });
    assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) { result.error = String(error.stack || error); throw error; }
finally {
    await browser?.close(); result.ownership.browserClosed = true;
    await vite?.close(); result.ownership.viteClosed = true;
    result.sourceAfter = await inventory(); result.inputsUnchanged = JSON.stringify(result.sourceBefore) === JSON.stringify(result.sourceAfter);
    await writeFile(path.join(out, "result.json"), JSON.stringify(result, null, 2));
    if (result.completed) assert.ok(result.inputsUnchanged, "Preset component input files changed during the original run");
}
