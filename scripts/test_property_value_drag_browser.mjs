// Actual ordered-value controls + pointer capture + SourceEditor history.
// Controlled host callback only: no Core, disk save, native window or OS IME acceptance.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { pathToFileURL } from "node:url";

const repo = path.resolve(import.meta.dirname, ".."), out = process.argv[2];
assert.ok(out && path.isAbsolute(out), "Pass a new absolute evidence directory");
assert.ok(path.relative(repo, out).startsWith(".." + path.sep), "Evidence stays outside the product repository");
await mkdir(out);
const files = ["scripts/test_property_value_drag_browser.mjs", "app/src/native-value-form.ts", "app/src/native-control-identity.ts", "app/src/editor.ts", "app/src/data-panel.ts", "app/src/ui-locale.ts", "app/src/ui-messages.ts", "assets/ui-messages.json"];
const inventory = async () => Promise.all(files.map(async file => ({ path: file, sha256: createHash("sha256").update(await readFile(path.join(repo, file))).digest("hex") })));
const result = { scope: "Current valueEditor pointer controls and actual SourceEditor/CodeMirror Undo; controlled host callback, no Core/native/disk-save/real-IME acceptance", cases: [], pageErrors: [], completed: false, sourceBefore: await inventory(), ownership: { browserClosed: false, viteClosed: false } };
const { chromium } = createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE || "playwright");
const { createServer } = await import(pathToFileURL(path.join(repo, "app/node_modules/vite/dist/node/index.js")).href);
let vite, browser;
async function check(name, run) { const entry = { name, passed: false }; result.cases.push(entry); await run(); entry.passed = true; }
try {
  vite = await createServer({ root: path.join(repo, "app"), configFile: false, server: { host: "127.0.0.1", port: 0, hmr: false, watch: { ignored: ["**/*"] } } });
  await vite.listen();
  browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE || "C:/Program Files/Google/Chrome/Application/chrome.exe", headless: true });
  result.browserVersion = browser.version();
  const page = await browser.newPage({ viewport: { width: 1250, height: 1100 } });
  page.on("pageerror", error => result.pageErrors.push(error.message));
  await page.route("**/property-drag-fixture", route => route.fulfill({ contentType: "text/html", body: '<!doctype html><meta charset="utf-8"><link rel="stylesheet" href="/src/data-view.css"><style>body{margin:0;font:14px sans-serif}main{display:flex;align-items:flex-start;gap:18px;padding:16px}#list,#other{width:390px}#editor{width:360px;height:300px}.native-list-item{margin:8px 0;padding:8px}.native-list-controls{display:flex;gap:4px}.native-list-item textarea{height:44px;min-height:32px}.native-list-order{margin:4px 0}</style><input id="author" value="独立写作原文"><main><section id="list"></section><section id="other"></section><section id="editor"></section></main>' }));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/property-drag-fixture`);
  await page.evaluate(async () => {
    const [{ valueEditor }, { SourceEditor }, { locale }] = await Promise.all([import("/src/native-value-form.ts"), import("/src/editor.ts"), import("/src/ui-locale.ts")]);
    const schema = { definitions: {}, fields: [], types: [], aliases: {} }, text = value => ({ kind: "text", value });
    let editor;
    window.fixture = {
      changes: [], otherChanges: [], editor: null, initialSource: "", initialValue: null, locale,
      mount(kind = "text") {
        editor?.destroy(); document.querySelector("#editor").replaceChildren(); locale.set("zh-CN");
        this.changes = []; this.otherChanges = [];
        const itemShape = kind === "int" ? { kind: "scalar", semantic: "int" } : kind === "record" ? { kind: "record", members: { number: { shape: { kind: "scalar", semantic: "phone" }, required: true, nullable: false } }, unknown: "preserve" } : { kind: "scalar", semantic: "text" };
        const record = marker => ({ kind: "record", value: [{ key: "number", value: text("00138") }, { key: "imported-extra", value: text(marker) }] });
        const initial = kind === "none" ? { kind: "none" } : { kind: "array", value: kind === "int" ? [1, 2, 3].map(value => ({ kind: "int", value: String(value) })) : kind === "record" ? [record("first"), record("second"), record("third")] : [text("first"), text("same"), text("same")] };
        this.initialValue = structuredClone(initial); this.initialSource = "= 合成原稿😀\n\n" + JSON.stringify(initial);
        editor = new SourceEditor(this.initialSource, () => {}, () => {}); editor.mount(document.querySelector("#editor")); this.editor = editor;
        const component = valueEditor({ kind: "list", items: itemShape, min: 0, max: null }, initial, schema, value => { this.changes.push(structuredClone(value)); editor.apply(0, editor.source.length, "= 合成原稿😀\n\n" + JSON.stringify(value)); }, "TEST list", { fieldId: "test/list", path: [] });
        document.querySelector("#list").replaceChildren(component); this.component = component;
        const other = valueEditor({ kind: "list", items: { kind: "scalar", semantic: "text" }, min: 0, max: null }, { kind: "array", value: [text("other one"), text("other two")] }, schema, value => this.otherChanges.push(value), "OTHER list", { fieldId: "test/other", path: [] });
        document.querySelector("#other").replaceChildren(other);
      },
    };
    document.addEventListener("pointerdown", event => { window.fixture.lastPointerId = event.pointerId; }, true);
    window.fixture.mount();
  });
  const list = page.locator("#list"), rows = () => list.locator(".native-list > .native-list-item"), handle = index => rows().nth(index).locator('[data-order-action="drag"]');
  const point = async (locator, after = false) => { const box = await locator.boundingBox(); assert.ok(box); return { x: box.x + Math.min(20, box.width / 2), y: box.y + (after ? box.height - 3 : 3) }; };
  async function start(index) { const box = await handle(index).boundingBox(); assert.ok(box); await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2); await page.mouse.down(); }
  async function target(index, after = false) { const p = await point(rows().nth(index), after); await page.mouse.move(p.x, p.y, { steps: 5 }); }
  async function drag(from, targetIndex, after = false) { await start(from); await target(targetIndex, after); await page.mouse.up(); }
  const changes = () => page.evaluate(() => window.fixture.changes);
  await check("actual pointer drop has a visible insertion line and emits one value; one real Undo/Redo restores the complete author source", async () => {
    await start(0); await target(2, true);
    assert.equal(await list.locator(".native-list-drop-line").isVisible(), true);
    assert.equal(await page.evaluate(() => window.fixture.changes.length), 0);
    await page.screenshot({ path: path.join(out, "property-value-drag-target.png"), fullPage: true });
    await page.mouse.up();
    assert.equal((await changes()).length, 1);
    assert.deepEqual((await changes())[0].value.map(item => item.value), ["same", "same", "first"]);
    assert.equal(await handle(2).evaluate(element => element === document.activeElement), true);
    const changedSource = await page.evaluate(() => window.fixture.editor.source);
    await page.evaluate(() => window.fixture.editor.undo());
    assert.equal(await page.evaluate(() => window.fixture.editor.source === window.fixture.initialSource), true);
    await page.evaluate(() => window.fixture.editor.redo());
    assert.equal(await page.evaluate(() => window.fixture.editor.source), changedSource);
    assert.equal(await page.locator("#author").inputValue(), "独立写作原文");
  });
  await check("records with duplicate known values retain each unknown member and exact occurrence order", async () => {
    await page.evaluate(() => window.fixture.mount("record")); await drag(2, 0);
    const moved = (await changes())[0];
    assert.deepEqual(moved.value.map(record => record.value.find(item => item.key === "imported-extra").value.value), ["third", "first", "second"]);
    assert.deepEqual(moved.value.map(record => record.value[0].value.value), ["00138", "00138", "00138"]);
  });
  await check("Escape, unchanged insertion gaps and dropping outside this list cancel without an author callback", async () => {
    await page.evaluate(() => window.fixture.mount());
    await start(0); await target(2, true); await page.keyboard.press("Escape"); await page.mouse.up();
    assert.equal((await changes()).length, 0); assert.equal(await list.locator(".native-list-drop-line").isVisible(), false);
    await drag(1, 1, true); assert.equal((await changes()).length, 0);
    await start(0); const p = await point(page.locator("#other .native-list-item").first(), true); await page.mouse.move(p.x, p.y, { steps: 5 }); await page.mouse.up();
    assert.deepEqual(await changes(), []); assert.deepEqual(await page.evaluate(() => window.fixture.otherChanges), []);
  });
  await check("invalid raw numeric input and active composition block the gesture without losing input", async () => {
    await page.evaluate(() => window.fixture.mount("int"));
    const numeric = rows().first().locator('input').filter({ visible: true }).first(); await numeric.fill("-");
    await drag(2, 0); assert.deepEqual(await changes(), []); assert.equal(await numeric.inputValue(), "-");
    assert.equal(await numeric.evaluate(element => element === document.activeElement), true);
    await page.evaluate(() => window.fixture.mount());
    await rows().first().locator("textarea").evaluate(element => element.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true, data: "中" })));
    await drag(2, 0); assert.deepEqual(await changes(), []);
    await rows().first().locator("textarea").evaluate(element => element.dispatchEvent(new CompositionEvent("compositionend", { bubbles: true, data: "中" })));
  });
  await check("a changed live value, freezing or hidden ownership invalidates a held gesture", async () => {
    await page.evaluate(() => window.fixture.mount()); await start(0); await target(2, true);
    await rows().nth(1).locator("textarea").evaluate(element => { element.value = "later input"; element.dispatchEvent(new Event("input", { bubbles: true })); });
    const before = await changes(); await page.mouse.up(); assert.deepEqual(await changes(), before);
    for (const attribute of ["inert", "hidden"]) {
      await page.evaluate(() => window.fixture.mount()); await start(0); await target(2, true);
      await list.evaluate((element, attribute) => element.setAttribute(attribute, ""), attribute);
      await page.mouse.up(); assert.deepEqual(await changes(), []);
      await list.evaluate((element, attribute) => element.removeAttribute(attribute), attribute);
    }
  });
  await check("retired controls and lost pointer capture cannot mutate the replacement list", async () => {
    await page.evaluate(() => window.fixture.mount()); await start(0); await target(2, true);
    await page.evaluate(() => window.fixture.mount()); await page.mouse.up(); assert.deepEqual(await changes(), []);
    await start(0); await target(2, true);
    await handle(0).evaluate(element => element.releasePointerCapture(window.fixture.lastPointerId));
    await page.mouse.up(); assert.deepEqual(await changes(), []);
  });
  await check("null never becomes a list implicitly and original keyboard ordering remains reachable", async () => {
    await page.evaluate(() => window.fixture.mount("none")); assert.equal(await handle(0).count(), 0); assert.deepEqual(await changes(), []);
    await page.evaluate(() => window.fixture.mount());
    await rows().nth(2).locator('.native-list-order > summary').click();
    const position = rows().nth(2).locator('[data-order-action="position"]'); await position.fill("1"); await position.press("Enter");
    assert.deepEqual((await changes())[0].value.map(item => item.value), ["same", "first", "same"]);
    await handle(0).focus(); await page.keyboard.press("Enter"); assert.match(await list.locator(".native-list-order-status").innerText(), /目标位置/);
    await page.evaluate(() => window.fixture.locale.set("en-US")); assert.equal(await handle(0).evaluate(element => element === document.activeElement), true);
  });
  await page.screenshot({ path: path.join(out, "property-value-drag.png"), fullPage: true });
  assert.deepEqual(result.pageErrors, []); result.sourceAfter = await inventory(); assert.deepEqual(result.sourceAfter, result.sourceBefore); result.completed = true;
} catch (error) { result.failure = error.stack || String(error); process.exitCode = 1; }
finally {
  if (browser) { await browser.close(); result.ownership.browserClosed = true; }
  if (vite) { await vite.close(); result.ownership.viteClosed = true; }
  await writeFile(path.join(out, "result.json"), JSON.stringify(result, null, 2) + "\n", { flag: "wx" });
}
console.log(JSON.stringify({ completed: result.completed, cases: result.cases, failure: result.failure, out }));
