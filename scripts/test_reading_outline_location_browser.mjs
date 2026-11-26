// Actual workspace geometry with synthetic Core; no installed native host claim.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { createServer } from "../app/node_modules/vite/dist/node/index.js";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || "playwright");
const root = fileURLToPath(new URL("../app/", import.meta.url));
const vite = await createServer({ root, configFile: false, server: { host: "127.0.0.1", port: 0, hmr: false } });
const browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE, headless: true });
const source = "= 同名章节\n\n== 小节\n\n= 同名章节";
const sourceHash = createHash("sha256").update(source).digest("hex");
const note = { sessionId: "outline-location", viewInputId: "one", path: "/tmp/synthetic/outline.typ", root: "/tmp/synthetic", kind: "ordinary", source, baseId: sourceHash };
const inventory = { root: note.root, status: "ordinary", entries: [], warnings: [], complete: true };
const heading = (key, text, level, generated = false) => ({ key, kind: "heading", text, level, generated, range: null });
const headings = [heading("begin", "同名章节", 1), heading("child", "小节", 2), heading("repeat", "同名章节", 1, true), heading("end", "短结尾", 1)];
const node = (key, tag, children, attrs = {}) => ({ key, kind: "element", tag, children, attrs, text: null, origin: null, writable: false });
const text = (key, value) => ({ key, kind: "text", text: value, tag: null, children: [], attrs: {}, origin: null, writable: false });
const filler = prefix => Array.from({ length: 36 }, (_, index) => node(prefix + index, "p", [text(prefix + "-text-" + index, "合成正文，用于浏览器滚动验证。")]));
const rendered = node("doc", "div", headings.flatMap(item => [
  node(item.key, "h" + item.level, [text(item.key + "-text", item.text)], { "data-weftext-role": "heading", "data-weftext-level": String(item.level) }),
  ...(item.key === "end" ? [node("end-body", "p", [text("end-text", "结束。")])] : filler("paragraph-" + item.key)),
]));
const role = { status: "absent", text: null, range: null, construct: null, encoding: null };
const map = { sourceHash, syntaxValid: true, literalRegions: [], structures: [], title: role, subtitle: role, bodyInsertion: null,
  unavailableReason: null, notes: { sourceHash, syntaxValid: true, entries: [] },
  headingNumbering: { sourceHash, settings: null, editable: true, frozenEndnotesRequired: false, reason: null }, bodyBlocks: [] };

try {
  await vite.listen();
  const url = "http://127.0.0.1:" + vite.httpServer.address().port;
  for (const width of [1120, 640, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 760 }, locale: "zh-CN" });
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.addInitScript(() => localStorage.setItem("weftext-view", "read"));
    await page.route("**/api/**", async route => {
      const path = new URL(route.request().url()).pathname;
      const body = route.request().postDataJSON() ?? {};
      const fixtures = {
        "/api/bootstrap": { inventory, initialDocument: note, recoveries: [] },
        "/api/library/open": inventory,
        "/api/render": { ok: true, html: "", tree: rendered, diagnostics: [], sourceInfo: { syntaxValid: true, identityStatus: "ordinary", types: [], headings: [], links: [], syntaxTokens: [] },
          dependencies: [], fontFamilies: [], compilerVersion: "injected reading location fixture", serial: body.serial, source: body.source,
          headingStructure: { coverage: "complete", truncated: false, items: headings } },
        "/api/editor-map": { ...map, serial: body.serial },
        "/api/query/embedded/discover": { serial: body.serial, source: body.source, sourceHash, syntaxValid: true, occurrences: [] },
      };
      return route.fulfill(path in fixtures ? { json: fixtures[path] } : { status: 404, json: {} });
    });
    await page.goto(url);
    const flow = page.locator(".mode-read .flow-pane");
    await flow.locator('[data-document-node-key="begin"]').waitFor();
    if (width <= 780) await page.locator(".sidebar-overlay-close").click();
    const ribbon = page.locator(".workspace-ribbon");
    await ribbon.locator('[data-tab="view"][role="tab"]').click();
    await ribbon.getByRole("button", { name: "大纲／标题结构", exact: true }).click();
    const outline = page.locator("#workspace-view-outline");
    const row = key => outline.locator(`[data-heading-key="${key}"]`);
    await row("begin").locator('[aria-current="location"]').waitFor();
    await row("begin").locator(".outline-fold").click();
    assert.equal(await row("child").count(), 0);
    const focused = outline.locator('[data-outline-control="search"]');
    await focused.focus();
    const setReading = key => flow.evaluate((el, key) => {
      const heading = el.querySelector(`[data-document-node-key="${key}"]`);
      const start = el.getBoundingClientRect().top + el.clientTop;
      el.scrollTop += heading.getBoundingClientRect().top - start;
    }, key);
    await setReading("child");
    await outline.locator('[data-heading-key="begin"][data-reading-location="contains"]').waitFor();
    assert.equal(await outline.locator('[aria-current="location"]').count(), 0);
    assert.equal(await focused.evaluate(el => el === document.activeElement), true);
    assert.equal(await row("child").count(), 0);

    await setReading("repeat");
    await row("repeat").locator('[aria-current="location"]').waitFor();
    assert.equal(await focused.evaluate(el => el === document.activeElement), true);
    assert.equal(await row("begin").locator('[aria-current="location"]').count(), 0);
    await focused.fill("小节");
    await row("child").waitFor();
    assert.equal(await focused.evaluate(el => el === document.activeElement), true);
    assert.equal(await outline.locator('[aria-current="location"]').count(), 0);
    await focused.fill("");
    await row("repeat").locator('[aria-current="location"]').waitFor();
    assert.equal(await row("child").count(), 0);
    await flow.evaluate(el => { el.scrollTop = el.scrollHeight; });
    await row("end").locator('[aria-current="location"]').waitFor();
    assert.equal(await focused.evaluate(el => el === document.activeElement), true);
    if (width === 1120) {
      await page.locator(".read-edit-shortcut").click();
      await page.locator(".mode-source .cm-content").waitFor();
      assert.equal(await outline.locator("[data-reading-location]").count(), 0);
    }
    assert.deepEqual(errors, []);
    console.log(`width ${width}: chapter position, identical title identity, folded parent, search and focus preserved`);
    await page.close();
  }
} finally {
  await browser.close();
  await vite.close();
}
