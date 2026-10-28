// Real proofreading/Knowledge/WorkPanel components with controlled scan and
// author ports. This is not Core, HTTP, native desktop or proofreading accuracy acceptance.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { pathToFileURL } from "node:url";

assert.equal(process.argv.length, 3, "Usage: node scripts/test_proofreading_panel_browser.mjs ABSOLUTE_NEW_OUTPUT");
const repo = path.resolve(import.meta.dirname, ".."), out = process.argv[2];
assert.ok(path.isAbsolute(out) && path.relative(repo, out).startsWith(".." + path.sep));
await mkdir(out);
const inputs = ["app/src/proofreading-panel.ts", "app/src/proofreading-results-state.ts", "app/src/proofreading-results-view.ts", "app/src/proofreading-wire.ts", "app/src/knowledge-panel.ts", "app/src/knowledge-view.css", "app/src/work-activity.ts", "app/src/work-panel.ts", "app/src/ui-locale.ts", "app/src/ui-messages.ts", "assets/ui-messages.json", "scripts/test_proofreading_panel_browser.mjs"];
const inventory = async () => Promise.all(inputs.map(async name => ({ path: name, sha256: createHash("sha256").update(await readFile(path.join(repo, name))).digest("hex") })));
const result = { schemaVersion: 1, developmentOnly: true, scope: "Actual headless-browser proofreading result/list/activity/owner components; controlled scan and author ports", cases: [], pageErrors: [], completed: false, sourceBefore: await inventory(), ownership: { browserClosed: false, viteClosed: false } };
const { createServer } = await import(pathToFileURL(path.join(repo, "app/node_modules/vite/dist/node/index.js")).href);
const { chromium } = createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE || "playwright");
let server, browser;
async function check(name, run) { const item = { name, passed: false }; result.cases.push(item); await run(); item.passed = true; }
try {
    server = await createServer({ root: path.join(repo, "app"), configFile: false, server: { host: "127.0.0.1", port: 0, hmr: false, watch: { ignored: ["**/*"] } } });
    await server.listen();
    browser = await chromium.launch({ executablePath: process.env.WEFTEXT_CHROME || "C:/Program Files/Google/Chrome/Application/chrome.exe", headless: true });
    result.browserVersion = browser.version();
    const page = await browser.newPage({ viewport: { width: 1200, height: 900 } });
    page.on("pageerror", error => result.pageErrors.push(error.message));
    await page.route("**/__proofreading_check", route => route.fulfill({ contentType: "text/html", body: '<!doctype html><meta charset="utf-8"><link rel="stylesheet" href="/src/knowledge-view.css"><link rel="stylesheet" href="/src/work-panel.css"><style>body{font:14px sans-serif;margin:0}main{display:flex;height:650px}#author{flex:1}#view{width:420px;overflow:auto}footer{height:25px}.knowledge-panel{width:100%;max-width:none}</style><main><textarea id="author" aria-label="Author source"></textarea><section id="view"></section></main><footer></footer>' }));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/__proofreading_check`);
    await page.evaluate(async () => {
        const [{ ProofreadingPanel }, { mountKnowledgePanel }, { WorkPanel }, { workActivity }, { locale, uiTextValue }, { defaultProcessingSettings }] = await Promise.all([import("/src/proofreading-panel.ts"), import("/src/knowledge-panel.ts"), import("/src/work-panel.ts"), import("/src/work-activity.ts"), import("/src/ui-locale.ts"), import("/src/processing-settings.ts")]);
        const view = document.querySelector("#view"), events = [], notes = [], writes = [], basis = { sessionId: "component-doc-A", path: "fixture/A/A.typ", source: "中文原稿 😀\n".repeat(600), serial: 1, baseId: "original-base", inputId: "a".repeat(32), root: "fixture", dirty: false };
        const work = new WorkPanel({ storage: null, notify: text => notes.push(uiTextValue(text)) });
        work.setContext("proofreading-component"); document.querySelector("footer").before(work.element); document.querySelector("footer").append(work.statusButton);
        const unsubscribe = workActivity.subscribe(event => { events.push(event); if (event.kind === "task") work.reportTask(event.task); else if (event.kind === "result") work.addResult(event.result); else work.removeResult(event.id); });
        const raw = count => ({ coverage: "complete", findings: Array.from({ length: count }, (_, index) => ({ id: `finding-${index}`, category: index % 2 ? "character" : "typo", ruleId: index % 2 ? "rule-a" : "rule-b", ruleVersion: "1", excerpt: `实际显示的合成原文 ${index}`, messageOrigin: "author", authorMessage: "合成规则，请核对", replacement: null, sourceRange: { from: index * 2, to: index * 2 + 1, utf16From: index * 2, utf16To: index * 2 + 1, raw: "x" }, executable: false })), checkedCharacters: count * 2, warnings: [], existing: [], processing: defaultProcessingSettings(), processingOrigin: null, fragments: [] });
        let owner, resolveScan, resolveBefore, preflight = false, commit = false, reveals = 0, scanCount = 0, selectedSource = [];
        const fixture = { view, work, events, notes, writes, basis, locale, raw, get owner() { return owner; }, get scanCount() { return scanCount; }, get reveals() { return reveals; }, get selectedSource() { return selectedSource; }, set commit(value) { commit = value; }, resolve(count) { const accept = resolveScan; resolveScan = undefined; accept(raw(count)); }, preflightDone() { resolveBefore(); }, mount(heldBefore = false) {
            owner?.dispose(); view.hidden = false; view.replaceChildren(); preflight = heldBefore;
            owner = new ProofreadingPanel({ resultScope: "local", context: () => ({ ...basis }), beforeAction: () => preflight ? new Promise(resolve => { resolveBefore = resolve; }) : Promise.resolve(), scan: () => { scanCount++; return new Promise(resolve => { resolveScan = resolve; }); }, perform: async (_label, intent) => { writes.push(structuredClone(intent)); return commit; }, selectSource: (from, to) => selectedSource.push([from, to]), revealReport: () => { reveals++; view.hidden = false; owner.render(view); }, notify: text => notes.push(uiTextValue(text)) });
            owner.render(view); return owner;
        }, mountKnowledge() {
            owner?.dispose(); view.hidden = false; view.replaceChildren();
            owner = mountKnowledgePanel(view, { context: () => ({ ...basis }), load: async () => { throw new Error("No library scan belongs to this component fixture"); }, prepare: async () => { throw new Error("No author preparation in this owner-reveal case"); }, commit: async () => { throw new Error("No author commit in this owner-reveal case"); }, navigate: () => {}, revealPanel: () => { reveals++; view.hidden = false; }, notify: text => notes.push(uiTextValue(text)), proofread: async () => raw(2) });
            owner.selectTab("proofreading"); return owner;
        }, cleanup() { owner?.dispose(); unsubscribe(); work.dispose(); } };
        fixture.mount(); window.fixture = fixture;
    });
    const view = page.locator("#view"), author = page.getByRole("textbox", { name: "Author source" });
    await check("running and successful scan facts retain writing focus and a collapsed bottom panel", async () => {
        await view.getByRole("button", { name: "开始校对", exact: true }).click();
        await page.waitForFunction(() => window.fixture.scanCount === 1);
        await author.fill("真实浏览器输入保留 😀");
        await author.focus();
        await page.evaluate(() => window.fixture.resolve(450));
        await view.locator(".proofreading-finding").first().waitFor();
        assert.equal(await view.locator(".proofreading-finding").count(), 200);
        assert.equal(await author.evaluate(element => element === document.activeElement), true);
        assert.equal(await page.locator(".work-panel").isVisible(), false);
        assert.deepEqual(await page.evaluate(() => window.fixture.events.filter(event => event.kind === "task").map(event => event.task.status)), ["running", "succeeded"]);
    });
    await check("450 findings append in 200-item batches and preserve existing DOM, focus, scroll and locale", async () => {
        await page.evaluate(() => { window.firstFinding = document.querySelector(".proofreading-finding"); window.firstCheckbox = window.firstFinding.querySelector("input"); });
        await view.getByRole("button", { name: "再显示 200 项", exact: true }).click();
        assert.equal(await view.locator(".proofreading-finding").count(), 400);
        assert.equal(await page.evaluate(() => window.firstFinding === document.querySelector(".proofreading-finding") && window.firstCheckbox === document.querySelector(".proofreading-finding input")), true);
        await page.evaluate(() => { window.firstCheckbox.focus(); const list = document.querySelector(".proofreading-result-list"); list.scrollTop = 150; window.savedScroll = list.scrollTop; window.fixture.locale.set("en-US"); });
        assert.equal(await page.evaluate(() => document.activeElement === window.firstCheckbox && document.querySelector(".proofreading-result-list").scrollTop === window.savedScroll), true);
        await view.getByRole("button", { name: "Show 50 more", exact: true }).click();
        assert.equal(await view.locator(".proofreading-finding").count(), 450);
        assert.equal(await page.evaluate(() => window.firstFinding === document.querySelector(".proofreading-finding")), true);
        assert.match(await view.locator(".proofreading-result-count").innerText(), /450.*450.*200/);
    });
    await check("visible selection is capped at 200 and one explicit action makes one bounded author request", async () => {
        const later = view.locator('.proofreading-finding[data-finding-id="finding-300"] input');
        await later.click(); assert.equal(await later.isChecked(), false);
        assert.equal(await page.evaluate(() => window.fixture.notes.length), 1);
        await view.getByRole("button", { name: "Clear selection", exact: true }).click();
        await later.check(); assert.equal(await later.isChecked(), true);
        await view.getByRole("button", { name: "Select visible located issues (up to 200)", exact: true }).click();
        await view.getByRole("button", { name: "Add selected results as annotations", exact: true }).click();
        await page.waitForFunction(() => window.fixture.writes.length === 1);
        assert.equal(await page.evaluate(() => window.fixture.writes[0].findings.length), 200);
        assert.equal(await view.locator('.proofreading-finding[data-finding-id="finding-0"] input').isChecked(), true);
        await page.evaluate(() => { window.fixture.commit = true; });
        await view.getByRole("button", { name: "Add selected results as annotations", exact: true }).click();
        await page.waitForFunction(() => window.fixture.writes.length === 2);
        assert.deepEqual(await page.evaluate(() => window.fixture.writes.map(write => write.findings)), [Array.from({ length: 200 }, (_, index) => `finding-${index}`), Array.from({ length: 200 }, (_, index) => `finding-${index}`)]);
        assert.equal(await view.locator('.proofreading-finding[data-finding-id="finding-0"] input').isDisabled(), true);
        assert.match(await view.locator(".proofreading-result-count").innerText(), /0 of 200 selected/);
        await page.evaluate(() => { window.secondBatchRow = document.querySelector('.proofreading-finding[data-finding-id="finding-250"]'); });
        await view.getByRole("button", { name: "Select visible located issues (up to 200)", exact: true }).click();
        await view.getByRole("button", { name: "Add selected results as annotations", exact: true }).click();
        await page.waitForFunction(() => window.fixture.writes.length === 3);
        assert.deepEqual(await page.evaluate(() => window.fixture.writes[2].findings), Array.from({ length: 200 }, (_, index) => `finding-${index + 200}`));
        assert.equal(await page.evaluate(() => window.secondBatchRow === document.querySelector('.proofreading-finding[data-finding-id="finding-250"]')), true);
        assert.equal(await view.locator('.proofreading-finding[data-finding-id="finding-250"] input').isDisabled(), true);
        assert.match(await view.locator(".proofreading-result-count").innerText(), /0 of 200 selected/);
        assert.equal(await author.inputValue(), "真实浏览器输入保留 😀");
    });
    await check("filter and rule/source ordering keep semantic rows without hidden write selections", async () => {
        await view.getByRole("combobox", { name: "Sort proofreading results", exact: true }).selectOption("rule");
        assert.equal(await view.locator(".proofreading-finding").first().getAttribute("data-finding-id"), "finding-1");
        await view.getByRole("combobox", { name: "All categories", exact: true }).selectOption("character");
        assert.equal(await view.locator(".proofreading-finding").count(), 200);
        assert.equal(await view.locator(".proofreading-finding").last().getAttribute("data-finding-id"), "finding-399");
        assert.match(await view.locator(".proofreading-result-count").innerText(), /200 of 225.*450/);
        await view.getByRole("combobox", { name: "Sort proofreading results", exact: true }).selectOption("source");
        assert.equal(await page.evaluate(() => window.firstFinding.isConnected), false);
    });
    await check("a changed basis rejects an old report action and a late scan cannot publish a fresh result", async () => {
        const before = await page.evaluate(() => { window.oldReportAction = window.fixture.events.filter(event => event.kind === "result").at(-1).result.actions[0]; return window.fixture.reveals; });
        await view.getByRole("button", { name: "Start proofreading", exact: true }).click();
        await page.waitForFunction(() => window.fixture.scanCount === 2);
        await page.evaluate(() => { window.fixture.basis.serial++; window.fixture.basis.inputId = "b".repeat(32); window.fixture.resolve(3); });
        await page.waitForFunction(() => window.fixture.events.filter(event => event.kind === "task").at(-1).task.status === "failed");
        assert.equal(await page.evaluate(async () => { try { await window.oldReportAction.run(); return false; } catch { return true; } }), true);
        assert.equal(await page.evaluate(() => window.fixture.reveals), before);
        assert.equal(await view.locator(".proofreading-finding").count(), 0);
    });
    await check("retiring a running owner or an awaited preflight never publishes or starts a late report", async () => {
        await view.getByRole("button", { name: "Start proofreading", exact: true }).click();
        await page.waitForFunction(() => window.fixture.scanCount === 3);
        await page.evaluate(() => { window.fixture.owner.dispose(); window.fixture.resolve(2); });
        await page.waitForFunction(() => window.fixture.events.filter(event => event.kind === "task").at(-1).task.status === "failed");
        assert.equal(await page.evaluate(() => window.fixture.events.filter(event => event.kind === "result").length), 1);
        await page.evaluate(() => window.fixture.mount(true));
        await view.getByRole("button", { name: "Start proofreading", exact: true }).click();
        await page.evaluate(async () => { window.fixture.owner.dispose(); window.fixture.preflightDone(); await Promise.resolve(); await Promise.resolve(); });
        assert.equal(await page.evaluate(() => window.fixture.scanCount), 3);
        assert.equal(await view.locator(".proofreading-finding").count(), 0);
    });
    await check("the real Knowledge owner reveals its original proofreading tab from a bottom result", async () => {
        await page.evaluate(() => window.fixture.mountKnowledge());
        await view.getByRole("button", { name: "Start proofreading", exact: true }).click();
        await view.locator(".proofreading-finding").first().waitFor();
        await page.evaluate(() => { window.parentFirstFinding = document.querySelector(".proofreading-finding"); window.fixture.owner.selectTab("links"); window.fixture.view.hidden = true; window.fixture.work.reveal("results"); });
        await page.locator(".work-panel").getByRole("button", { name: "View proofreading report", exact: true }).click();
        assert.equal(await view.isVisible(), true);
        assert.equal(await view.getByRole("tab", { name: "Proofreading", exact: true }).getAttribute("aria-selected"), "true");
        assert.equal(await page.evaluate(() => window.parentFirstFinding === document.querySelector(".proofreading-finding")), true);
        assert.equal(await view.locator(".proofreading-result-count").evaluate(element => element === document.activeElement), true);
        await page.screenshot({ path: path.join(out, "proofreading-owner-report.png"), fullPage: true });
        await page.evaluate(() => window.fixture.cleanup());
    });
    result.sourceAfter = await inventory();
    assert.deepEqual(result.sourceAfter, result.sourceBefore);
    assert.deepEqual(result.pageErrors, []);
    result.completed = true;
} catch (error) { result.error = String(error?.stack || error); process.exitCode = 1; }
finally {
    if (browser) { await browser.close(); result.ownership.browserClosed = true; }
    if (server) { await server.close(); result.ownership.viteClosed = true; }
    await writeFile(path.join(out, "result.json"), JSON.stringify(result, null, 2) + "\n");
}
console.log(JSON.stringify({ completed: result.completed, cases: result.cases.filter(item => item.passed).length, out }));
