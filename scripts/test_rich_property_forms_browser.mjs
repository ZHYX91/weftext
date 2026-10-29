// Installed business schema + actual shared forms and SourceEditor history.
// The host callback is controlled: no Core save, persistence, authorization or native acceptance.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { pathToFileURL } from "node:url";
const repo = path.resolve(import.meta.dirname, ".."), out = process.argv[2];
assert.ok(out && path.isAbsolute(out), "Pass a new absolute output directory");
assert.ok(path.relative(repo, out).startsWith(".." + path.sep), "Output stays outside the product repository");
await mkdir(out);
const files = ["scripts/test_rich_property_forms_browser.mjs", "docs/design/examples/schema.json", "app/src/native-value-form.ts", "app/src/data-view.css", "app/src/builtin-property-labels.ts", "app/src/native-control-identity.ts", "app/src/editor.ts", "assets/ui-messages.json", "app/src/ui-messages.ts"];
const inventory = async () => Promise.all(files.map(async file => ({ path: file, sha256: createHash("sha256").update(await readFile(path.join(repo, file))).digest("hex") })));
const result = { scope: "Installed builtin schema, actual shared native forms and SourceEditor Undo; controlled source callback only, no Core/save/recovery/permissions/native-window/OS-IME acceptance", cases: [], pageErrors: [], completed: false, sourceBefore: await inventory() };
const { chromium } = createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE || "playwright");
const { createServer } = await import(pathToFileURL(path.join(repo, "app/node_modules/vite/dist/node/index.js")).href);
let vite, browser;
async function check(name, run) { const item = { name, passed: false }; result.cases.push(item); await run(); item.passed = true; }
try {
    vite = await createServer({ root: path.join(repo, "app"), configFile: false, server: { host: "127.0.0.1", port: 0, hmr: false, watch: { ignored: ["**/*"] } } });
    await vite.listen();
    browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE || "C:/Program Files/Google/Chrome/Application/chrome.exe", headless: true });
    result.browserVersion = browser.version();
    const page = await browser.newPage({ viewport: { width: 1300, height: 1000 } });
    page.on("pageerror", error => result.pageErrors.push(error.message));
    await page.route("**/rich-property-fixture", route => route.fulfill({ contentType: "text/html", body: '<!doctype html><meta charset="utf-8"><link rel="stylesheet" href="/src/data-view.css"><style>body{font:14px sans-serif;margin:16px}main{display:grid;grid-template-columns:1fr 1fr;gap:20px}#editor{height:300px;position:sticky;top:8px}.native-member{padding:5px;margin:4px;border-left:1px solid #ccc}.native-list-item{margin:8px 0}.data-input{max-width:95%}</style><main><section id="forms"></section><section id="editor"></section></main>' }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/rich-property-fixture`);
    const builtin = JSON.parse(await readFile(path.join(repo, "docs/design/examples/schema.json"), "utf8"));
    await page.evaluate(async builtin => {
        const [{ valueEditor, schemaFieldLabel, nativeFormControls, nativeControlAddress }, { SourceEditor }, { locale, uiTextValue }] = await Promise.all([
            import("/src/native-value-form.ts"), import("/src/editor.ts"), import("/src/ui-locale.ts"),
        ]);
        const text = value => ({ kind: "text", value }), record = value => ({ kind: "record", value: Object.entries(value).map(([key, value]) => ({ key, value })) }), list = (...value) => ({ kind: "array", value });
        const ref = id => `weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/${id}`;
        const a = ref("12d7c4f3-9ee6-4c36-920d-ec6af27bc654"), b = ref("8aa10c08-3e7d-4d10-9488-92bc7f393050");
        const date = (year, month) => record({ calendar: text("gregorian"), version: text("proleptic-gregorian.v1"), precision: text(month ? "month" : "year"), year: { kind: "int", value: String(year) }, ...(month ? { month: { kind: "int", value: String(month) } } : {}) });
        const serialize = value => value.kind === "record" ? "(" + value.value.map(member => `${member.key}: ${serialize(member.value)}`).join(", ") + ",)" : value.kind === "array" ? "(" + value.value.map(serialize).join(", ") + (value.value.length ? "," : "") + ")" : value.kind === "text" ? JSON.stringify(value.value) : value.kind === "decimal" ? `decimal(${JSON.stringify(value.value)})` : value.kind === "none" ? "none" : String(value.value);
        let editor;
        window.fixture = {
            locale, changes: [], values: {}, schema: builtin, a, b,
            source() { return '#metadata((id: "32eb9a12-e752-4bfa-8cdf-65e00f23ac26", types: ("person", "paper"), ' + Object.entries(this.values).map(([key, value]) => `${key}: ${serialize(value)}`).join(", ") + ')) <weftext>\n\n正文😀 /* retained body */\r\n'; },
            mount(custom = false) {
                locale.set("zh-CN"); editor?.destroy(); document.querySelector("#editor").replaceChildren(); document.querySelector("#forms").replaceChildren(); this.changes = [];
                this.schema = structuredClone(builtin);
                if (custom) this.schema.fields.find(field => field.id === "contacts/phones").id = "my/phones";
                this.values = {
                    phones: list(record({ number: text("00138"), label: text("work"), note: text("first note"), imported: { kind: "decimal", value: "0.30" } }), record({ number: text("00138"), label: text("personal"), note: text("second note") })),
                    engagements: list(record({ employer: record({ ref: text(a) }), position: text("Engineer"), during: record({ start: date(2020), end: { kind: "none" } }) }), record({ employer: record({ ref: text(b) }), position: text("Editor"), during: record({ start: date(2024, 3), end: { kind: "none" } }) })),
                    publication: record({ kind: text("article"), title: text("Owned paper"), contributors: list(record({ agent: record({ ref: text(a) }), credited_as: text("同名"), roles: list(text("author")) }), record({ agent: record({ ref: text(b) }), credited_as: text("同名"), roles: list(text("editor")) })) }),
                };
                this.initial = structuredClone(this.values);
                editor = new SourceEditor(this.source(), () => {}, () => {}); editor.mount(document.querySelector("#editor")); this.editor = editor;
                this.renderForms();
            },
            renderForms() {
                document.querySelector("#forms").replaceChildren();
                for (const key of ["phones", "engagements", "publication"]) {
                    const field = this.schema.fields.find(field => field.key === key), section = document.createElement("section"), title = document.createElement("h2"); section.id = key;
                    title.textContent = uiTextValue(schemaFieldLabel(this.schema, field)); section.append(title);
                    section.append(valueEditor(field.shape, this.values[key], this.schema, value => { this.values[key] = structuredClone(value); this.changes.push({ key, value: structuredClone(value) }); editor.apply(0, editor.source.length, this.source()); }, schemaFieldLabel(this.schema, field), { fieldId: field.id, path: [] }));
                    document.querySelector("#forms").append(section);
                    nativeFormControls(section);
                }
            },
            control(fieldId, path, role) {
                const matches = [...document.querySelectorAll("#forms [data-control-key]")].filter(control => {
                    const address = nativeControlAddress(control);
                    return address.identity.fieldId === fieldId && JSON.stringify(address.identity.path) === JSON.stringify(path) && address.role === role;
                });
                if (matches.length !== 1) throw new Error(`Expected one actual control, got ${matches.length}: ${fieldId}/${JSON.stringify(path)}/${role}`);
                return matches[0].dataset.controlKey;
            },
        };
        window.fixture.mount();
    }, builtin);
    const member = key => ({ kind: "member", key }), index = index => ({ kind: "index", index });
    async function control(field, path, role) { const key = await page.evaluate(({ field, path, role }) => window.fixture.control(field, path, role), { field, path, role }); return page.locator(`[data-control-key=${JSON.stringify(key)}]`); }
    const values = () => page.evaluate(() => window.fixture.values);
    const read = (value, key) => value.value.find(member => member.key === key).value;
    async function move(field, path, to) { const position = await control(field, path, "order-position"); await position.locator("xpath=..").locator("summary").click(); await position.fill(String(to)); await position.press("Enter"); }
    await check("builtin duplicate contacts have real usage captions and preserve notes, custom source and exact order through one shared Undo", async () => {
        assert.match(await page.locator("#phones").innerText(), /用途/);
        assert.doesNotMatch(await page.locator("#phones").innerText(), /标签/);
        const usage = await control("contacts/phones", [index(0), member("label")], "value:enum");
        assert.equal(await usage.locator('option[value="work"]').innerText(), "工作");
        await (await control("contacts/phones", [index(0), member("note")], "value:text")).fill("changed first note");
        const before = await page.evaluate(() => window.fixture.editor.source);
        await move("contacts/phones", [index(1)], 1);
        const current = (await values()).phones.value;
        assert.deepEqual(current.map(record => read(record, "number").value), ["00138", "00138"]);
        assert.deepEqual(current.map(record => read(record, "note").value), ["second note", "changed first note"]);
        assert.deepEqual(current.map(record => read(record, "label").value), ["personal", "work"]);
        assert.equal(read(current[1], "imported").value, "0.30");
        const after = await page.evaluate(() => window.fixture.editor.source);
        await page.evaluate(() => window.fixture.editor.undo()); assert.equal(await page.evaluate(() => window.fixture.editor.source), before);
        await page.evaluate(() => window.fixture.editor.redo()); assert.equal(await page.evaluate(() => window.fixture.editor.source), after);
        await page.locator("#phones").screenshot({ path: path.join(out, "builtin-contact-zh.png") });
    });
    await check("employment edits and reordering retain organization, position and year/month precision on the same occurrence", async () => {
        await (await control("people/engagements", [index(0), member("position")], "value:text")).fill("Principal Engineer");
        await move("people/engagements", [index(1)], 1);
        const current = (await values()).engagements.value;
        assert.deepEqual(current.map(record => read(record, "position").value), ["Editor", "Principal Engineer"]);
        assert.deepEqual(current.map(record => read(read(record, "employer"), "ref").value), await page.evaluate(() => [window.fixture.b, window.fixture.a]));
        const year = read(read(current[1], "during"), "start");
        assert.equal(read(year, "precision").value, "year"); assert.equal(year.value.some(item => item.key === "month" || item.key === "day"), false);
        assert.equal(current.every(record => read(read(record, "during"), "end").kind === "none"), true);
    });
    await check("same-name contributors reorder without merging their distinct references or authored role strings", async () => {
        await move("library/publication", [member("contributors"), index(1)], 1);
        const contributors = read((await values()).publication, "contributors").value;
        assert.deepEqual(contributors.map(record => read(record, "credited_as").value), ["同名", "同名"]);
        assert.deepEqual(contributors.map(record => read(record, "roles").value[0].value), ["editor", "author"]);
        assert.deepEqual(contributors.map(record => read(read(record, "agent"), "ref").value), await page.evaluate(() => [window.fixture.b, window.fixture.a]));
    });
    await check("locale changes captions without replacing focused author input or modifying source; open enums retain one authored text value", async () => {
        const note = await control("contacts/phones", [index(0), member("note")], "value:text"); await note.focus();
        const before = await page.evaluate(() => ({ source: window.fixture.editor.source, changes: window.fixture.changes.length }));
        await page.evaluate(() => window.fixture.locale.set("en-US"));
        assert.equal(await note.evaluate(node => document.activeElement === node), true);
        assert.match(await page.locator("#phones").innerText(), /Usage label/);
        assert.deepEqual(await page.evaluate(() => ({ source: window.fixture.editor.source, changes: window.fixture.changes.length })), before);
        const kind = await control("library/publication", [member("kind")], "value:enum");
        assert.equal(await kind.evaluate(control => control.tagName), "TEXTAREA");
        await kind.fill("作者自定义\nsecond line😀");
        assert.equal(read((await values()).publication, "kind").value, "作者自定义\nsecond line😀");
        await kind.fill("book"); assert.equal(read((await values()).publication, "kind").value, "book");
        assert.equal(await page.locator('#publication [data-control-key]').evaluateAll(controls => controls.some(control => JSON.parse(control.dataset.controlKey)[2][0] === "value:enum-custom")), false);
        await page.locator("#phones").screenshot({ path: path.join(out, "builtin-contact-en.png") });
    });
    await check("a custom schema with the same phones/label keys keeps literal names and enum codes", async () => {
        await page.evaluate(() => window.fixture.mount(true));
        assert.equal(await page.locator("#phones h2").innerText(), "phones");
        const usage = await control("my/phones", [index(0), member("label")], "value:enum");
        assert.equal(await usage.locator('option[value="work"]').innerText(), "work");
        assert.match(await page.locator("#phones").innerText(), /customLabel/);
        assert.doesNotMatch(await page.locator("#phones").innerText(), /自定义用途/);
        assert.equal(await page.evaluate(() => window.fixture.changes.length), 0);
    });
    await page.screenshot({ path: path.join(out, "rich-property-forms.png") });
    assert.deepEqual(result.pageErrors, []); result.sourceAfter = await inventory(); assert.deepEqual(result.sourceAfter, result.sourceBefore); result.completed = true;
} catch (error) { result.failure = error.stack || String(error); process.exitCode = 1; }
finally {
    if (browser) await browser.close();
    if (vite) await vite.close();
    await writeFile(path.join(out, "result.json"), JSON.stringify(result, null, 2) + "\n", { flag: "wx" });
}
console.log(JSON.stringify({ completed: result.completed, cases: result.cases, failure: result.failure, out }));
