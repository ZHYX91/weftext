// Runs the production Knowledge panel and diagnostic buttons in Chromium.
// The host provides synthetic local contexts and records the caret requests.
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { createServer } from "../app/node_modules/vite/dist/node/index.js";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || "playwright");
const root = fileURLToPath(new URL("../app/", import.meta.url));
const vite = await createServer({ root, configFile: false, server: { host: "127.0.0.1", port: 0, hmr: false } });
const browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE, headless: true });
try {
  await vite.listen();
  const url = "http://127.0.0.1:" + vite.httpServer.address().port;
  const page = await browser.newPage({ locale: "zh-CN" });
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.route(url + "/diagnostic-fixture", route => route.fulfill({
    contentType: "text/html", body: '<!doctype html><button id="other">Other note</button><main id="knowledge"></main>',
  }));
  await page.goto(url + "/diagnostic-fixture");
  await page.evaluate(async () => {
    const { mountKnowledgePanel } = await import("/src/knowledge-panel.ts");
    const { locale } = await import("/src/ui-locale.ts");
    locale.set("zh-CN");
    const { nodeSourceHash } = await import("/src/hosted-navigation-state.ts");
    const source = "正文 #link(目标) 第一处。正文 #link(目标) 第二处。";
    const owner = { sessionId: "A", nodeId: "a", workspaceId: "w", root: "r", path: "A.typ", source, serial: 0, inputId: "input-A", baseId: "base", dirty: false };
    const digest = await nodeSourceHash(source);
    const address = "#link(目标)", from = source.lastIndexOf(address);
    const item = { source: { workspaceId: "w", nodeId: "a", title: "Note A", path: "A.typ" },
      resolution: { status: "missing" }, sourceHash: digest,
      occurrence: { target: address, utf16From: from, utf16To: from + address.length } };
    const snapshot = {
      graph: null, headingStructure: null, links: null, renderedLinks: [], citations: null, mentions: null,
      annotations: [], annotationStatus: "valid", templates: [], trash: [], warnings: [], errors: [],
      annotationDiagnostics: [], propertyReferences: { coverage: "complete", items: [], diagnostics: [] },
      diagnostics: { coverage: "complete", scannedNodes: 1, nodes: [], issues: [
        item, { ...item, occurrence: { ...item.occurrence, utf16From: source.indexOf(address), utf16To: source.indexOf(address) + address.length } },
      ] },
    };
    let context = { ...owner }, held, next = null;
    const selected = [], notices = [];
    const naturalDigest = crypto.subtle.digest.bind(crypto.subtle);
    crypto.subtle.digest = (algorithm, bytes) => {
      if (!next) return naturalDigest(algorithm, bytes);
      const pending = next; next = null;
      return new Promise((resolve, reject) => { held = () => naturalDigest(algorithm, bytes).then(resolve, reject); pending(); });
    };
    const panel = mountKnowledgePanel(document.querySelector("#knowledge"), {
      context: () => context, load: async () => snapshot, navigate: () => {},
      notify: () => notices.push(context.sessionId), selectSource: (from, to) => selected.push({ session: context.sessionId, from, to }),
      prepare: async () => { throw Error("Unexpected write"); }, commit: async () => { throw Error("Unexpected write"); },
    });
    const outside = document.querySelector("#other");
    outside.onclick = () => { context = { ...owner, sessionId: "B", nodeId: "b", path: "B.typ", inputId: "input-B" }; };
    window.fixture = {
      reset() { context = { ...owner }; selected.length = notices.length = 0; held = undefined; },
      arm() { window.fixture.claimed = false; next = () => { window.fixture.claimed = true; }; },
      release() { return held(); },
      edit() { context = { ...context, serial: 1, inputId: "edited", source: "前缀。" + source }; },
      check() { return { selected, notices, from, to: from + address.length }; },
    };
    await panel.refresh();
  });
  await page.getByLabel("问题分组", { exact: true }).selectOption("occurrence");
  const actions = page.getByRole("button", { name: "定位来源", exact: true });
  assert.equal(await actions.count(), 2);
  const locate = actions.first();
  const nextLocate = actions.last();
  await locate.click();
  await page.waitForFunction(() => window.fixture.check().selected.length === 1);
  const { from, to, selected } = await page.evaluate(() => window.fixture.check());
  assert.deepEqual(selected, [{ session: "A", from, to }]);
  for (const mode of process.argv[2] ? [process.argv[2]] : ["switch", "edit"]) {
    await page.evaluate(() => { window.fixture.reset(); window.fixture.arm(); });
    await locate.click();
    await page.waitForFunction(() => window.fixture.claimed);
    const other = page.locator("#other");
    if (mode === "switch") await other.click();
    else { await other.focus(); await page.evaluate(() => window.fixture.edit()); }
    await page.evaluate(() => window.fixture.release());
    await page.waitForFunction(() => window.fixture.check().selected.length || window.fixture.check().notices.length);
    const result = await page.evaluate(() => window.fixture.check());
    assert.deepEqual(result.selected, [], mode + " must not move the caret");
    assert.equal(result.notices.length, 1);
    assert.equal(await other.evaluate(el => el === document.activeElement), true);
  }
  await page.evaluate(() => { window.fixture.reset(); window.fixture.arm(); });
  await locate.click();
  await page.waitForFunction(() => window.fixture.claimed);
  await nextLocate.click();
  await page.waitForFunction(() => window.fixture.check().selected.length === 1);
  const newest = await page.evaluate(() => window.fixture.check().selected[0]);
  assert.notEqual(newest.from, from);
  assert.equal(newest.to - newest.from, to - from);
  await page.evaluate(async () => {
    await window.fixture.release();
    await new Promise(requestAnimationFrame);
  });
  const overlap = await page.evaluate(() => window.fixture.check());
  if (process.env.EXPECT_STALE_DIAGNOSTIC_JUMP) {
    assert.deepEqual(overlap.selected, [newest, { session: "A", from, to }], "baseline reselects older occurrence");
  } else {
    assert.deepEqual(overlap.selected, [newest], "only the latest activation may select the source");
    assert.deepEqual(overlap.notices, [], "superseded successful verification is not an error");
  }
  assert.deepEqual(errors, []);
  console.log("Repeated source ranges, switch/edit and reverse delivery of two diagnostic selections checked");
} finally { await browser.close(); await vite.close(); }
