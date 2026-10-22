// Actual original-operation delivery consumers in owned headless Chrome.
// node scripts/test_creation_delivery_browser.mjs EXE NEW_OUTPUT [all|CASE]
// Optional WEFTEXT_BROWSER_FRONTEND_URL uses an already running local Vite UI;
// WEFTEXT_API_PORT must match its proxy. This is source-UI/backend evidence,
// distinct from acceptance of the executable's embedded frontend.
// Browser abort/500 below are controlled delivery failures, not internal Core IO.
import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { createRequire } from "node:module";
import { mkdir, readFile, writeFile, readdir, rename, stat } from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
const [exeArg, outArg, selected = "all"] = process.argv.slice(2);
const repoArg = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
if (![exeArg, repoArg, outArg].every(value => value && path.isAbsolute(value))) throw Error("Executable and new output must be explicit absolute paths");
const exe = path.resolve(exeArg), repo = path.resolve(repoArg), out = path.resolve(outArg);
const frontendUrl = process.env.WEFTEXT_BROWSER_FRONTEND_URL;
const apiPort = Number(process.env.WEFTEXT_API_PORT);
if (frontendUrl && (!/^http:\/\/127\.0\.0\.1:\d+$/.test(frontendUrl)
  || !Number.isInteger(apiPort) || apiPort < 1 || apiPort > 65535)) throw Error("Source UI needs an explicit loopback origin and matching backend port");
const cases = ["note", "library", "save-as-current", "save-as-unavailable", "save-as-tab-identity", "save-malformed", "history-fence", "history-current", "history-later", "history-resolve"];
if (selected !== "all" && !cases.includes(selected)) throw Error("Unknown case " + selected);
await mkdir(out, { recursive: false });
const require = createRequire(import.meta.url);
// No search across workspaces: explicit generic runtime, or this checkout only.
const runtime = process.env.WEFTEXT_PLAYWRIGHT_MODULE || createRequire(path.join(repo, "app/package.json")).resolve("playwright");
const { chromium } = require(runtime);
const sha = bytes => createHash("sha256").update(bytes).digest("hex");
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const sourceFiles = ["app/src/main.ts", "app/src/history-panel.ts", "app/src/history-state.ts", "app/src/library-creation.ts", "app/src/session.ts"];
const result = { executable: exe, executableSha256: sha(await readFile(exe)),
  frontend: frontendUrl ? { kind: "source-vite", origin: frontendUrl, backendPort: apiPort } : { kind: "embedded" },
  head: execFileSync("git", ["-C", repo, "rev-parse", "HEAD"], { windowsHide: true }).toString().trim(),
  sourceHashes: Object.fromEntries(await Promise.all(sourceFiles.map(async name => [name, sha(await readFile(path.join(repo, name)))]))),
  cases: [], http: [], uiRequests: [], controlledFailures: [], pageErrors: [], routeErrors: [], processes: [], completed: false,
  limits: ["Controlled browser transport/read failures are not internal Core failures.", "History uses equal bytes S to exercise the real navigation Session rebind path.",
    "Owned TEST data/headless Chrome only; no system IME, accessibility, power-loss or graceful shutdown acceptance.",
    "This evidence belongs to the existing task acceptance record; not a second completion authority."] };
const state = path.join(out, "private-state");
let child, base, token, context, page, caseName = "", logs = { stdout: "", stderr: "" }, observed = [];
async function until(check, label) {
  const deadline = Date.now() + 30000; let last;
  while (Date.now() < deadline) { if (result.routeErrors.length) throw Error(result.routeErrors.at(-1).error);
    try { if (await check()) return; } catch (error) { last = error; } await pause(100); }
  throw Error(label + (last ? ": " + String(last) : ""));
}
async function intercept(pattern, handler) {
  await page.route(pattern, async route => {
    try { await handler(route); }
    catch (error) { result.routeErrors.push({ case: caseName, url: route.request().url(), error: String(error.stack || error) });
      await route.abort("failed").catch(() => {}); }
  });
}
async function start(opened) {
  assert.equal(child, undefined); logs = { stdout: "", stderr: "" }; base = undefined; token = undefined;
  const args = ["--no-browser", "--data-dir", state];
  if (frontendUrl) args.push("--port", String(apiPort));
  if (opened) args.push("--open", opened.path, "--root", opened.root);
  child = spawn(exe, args, { windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
  result.processes.push({ pid: child.pid, exe, args, startedAt: new Date().toISOString() });
  child.stdout.on("data", value => logs.stdout += value); child.stderr.on("data", value => logs.stderr += value);
  await until(() => { if (child.exitCode !== null) throw Error(logs.stderr);
    const match = logs.stdout.match(/(http:\/\/127\.0\.0\.1:\d+)\/#token=(\w+)/);
    if (!match) return false; base = match[1]; token = match[2]; return true; }, "Owned backend startup");
}
async function stop() {
  if (!child) return;
  const owned = child, record = result.processes.at(-1);
  if (owned.exitCode === null && owned.signalCode === null) {
    const ended = new Promise((resolve, reject) => { const timer = setTimeout(() => reject(Error("Owned child termination timeout")), 20000);
      owned.once("exit", () => { clearTimeout(timer); resolve(); }); });
    owned.kill(); await ended;
  }
  record.exitCode = owned.exitCode; record.signalCode = owned.signalCode; record.shutdown = "controlled handle termination, not graceful shutdown";
  record.listenerClosed = !base ? null : await new Promise(resolve => {
    const socket = net.createConnection({ host: "127.0.0.1", port: Number(new URL(base).port) });
    socket.once("connect", () => { socket.destroy(); resolve(false); });
    socket.once("error", () => { socket.destroy(); resolve(true); });
    socket.setTimeout(1000, () => { socket.destroy(); resolve(false); });
  });
  await writeFile(path.join(out, "pid-" + owned.pid + "-stdout.log"), logs.stdout);
  await writeFile(path.join(out, "pid-" + owned.pid + "-stderr.log"), logs.stderr);
  child = undefined; assert.equal(record.listenerClosed, true);
}
async function api(route, request, expected = 200) {
  const response = await fetch(base + route, { method: request === undefined ? "GET" : "POST",
    headers: { "X-Weftext-Token": token, "Content-Type": "application/json" }, body: request === undefined ? undefined : JSON.stringify(request) });
  const value = await response.json();
  result.http.push({ case: caseName, route, request, status: response.status, response: value });
  assert.equal(response.status, expected, JSON.stringify(value)); return value;
}
async function save(doc, source) {
  const receipt = await api("/api/save", { sessionId: doc.sessionId, baseId: doc.baseId, source, operationId: randomUUID() });
  assert.equal(receipt.status, "saved"); return api("/api/open", { root: doc.root, path: doc.path });
}
async function seed() {
  const root = path.join(out, caseName + "-library");
  await api("/api/library/create", { path: root, operationId: randomUUID() });
  const note = async name => { const receipt = await api("/api/note/create", { root, parent: root, name, operationId: randomUUID() });
    assert.equal(receipt.status, "committed"); assert.ok(receipt.document && !receipt.documentError); return receipt.document; };
  let a = await note("TEST-A"), b = await note("TEST-B");
  const sourceS = a.source + "\n\n= Owned S\n\nSubmitted S 😀\n";
  a = await save(a, sourceS);
  const versionS = (await api("/api/history/list", { sessionId: a.sessionId })).currentSavedVersionId;
  a = await save(a, sourceS + "\nIntervening historical version\n");
  a = await save(a, sourceS); // same-byte historical restore still creates a new operation
  b = await save(b, b.source + "\n\n= Navigation B\n");
  return { root, a, b, sourceS, versionS };
}
const click = (name, host = page) => host.getByRole("button", { name, exact: true }).click();
const editor = () => page.locator(".source-host .cm-content");
const timeline = () => page.getByRole("dialog", { name: "本地笔记时间线", exact: true });
async function snapshot() {
  return editor().evaluate(element => { const view = element.cmTile?.root?.view;
    if (!view || view.contentDOM !== element) throw Error("Actual CodeMirror view unavailable");
    return { source: view.state.doc.toString(), selection: { anchor: view.state.selection.main.anchor, head: view.state.selection.main.head } }; });
}
async function expectSource(source) { await until(async () => (await snapshot()).source === source, "Exact editor source"); }
async function browse(f) {
  const profile = path.join(out, caseName + "-owned-chrome");
  context = await chromium.launchPersistentContext(profile, { headless: true, viewport: { width: 1440, height: 1000 },
    ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : { channel: "chrome" }) });
  result.browser = { version: context.browser()?.version(), profile, executable: process.env.WEFTEXT_BROWSER_EXECUTABLE || "system Chrome channel" };
  page = context.pages()[0] || await context.newPage(); page.setDefaultTimeout(30000);
  page.on("pageerror", error => result.pageErrors.push({ case: caseName, error: String(error) }));
  page.on("request", request => { const route = new URL(request.url()).pathname;
    if (route.startsWith("/api/")) result.uiRequests.push({ case: caseName, route, body: request.postDataJSON() }); });
  page.on("response", response => { const route = new URL(response.url()).pathname; if (!route.startsWith("/api/")) return;
    const capturedCase = caseName; observed.push((async () => { const body = await response.json().catch(() => null);
      result.http.push({ case: capturedCase, browser: true, route, request: response.request().postDataJSON(), status: response.status(), response: body }); })()); });
  await page.goto((frontendUrl || base) + "/#token=" + token); await click("源码"); await expectSource(f.sourceS);
}
async function closeBrowser() { if (context) await context.close(); context = undefined; page = undefined;
  await Promise.allSettled(observed); observed = []; }
async function inputY() { const before = await snapshot(); await editor().click(); await page.keyboard.press("Control+End");
  await page.keyboard.press("Enter"); await page.keyboard.insertText("Waiting Y 😀");
  await expectSource(before.source + "\nWaiting Y 😀"); return { before, after: await snapshot() }; }
async function undoRedo(proof) { await editor().click(); await page.keyboard.press("Control+z"); await expectSource(proof.before.source);
  await page.keyboard.press("Control+y"); await expectSource(proof.after.source); }
async function navigation(f) {
  const before = result.uiRequests.length; await page.keyboard.press("Control+p");
  const quick = page.getByRole("dialog", { name: "快捷打开", exact: true });
  await quick.getByRole("searchbox").fill("TEST-B"); await quick.getByRole("searchbox").press("Enter");
  await quick.waitFor({ state: "hidden" }); await expectSource(f.b.source);
  await click("返回", page.getByRole("navigation", { name: "日常导航与命令", exact: true })); await expectSource(f.sourceS);
  assert.ok(result.uiRequests.slice(before).some(item => item.route === "/api/data/open" && String(item.body.entityKey).endsWith("/" + f.a.nodeId)));
}
async function openTimeline() { await click("时间线", page.locator(".top-actions")); await timeline().waitFor(); }
async function closeTimeline() { await click("关闭时间线", timeline()); await timeline().waitFor({ state: "hidden" }); }
async function prepareHistory(f) {
  await openTimeline(); await timeline().locator('[data-history-version="' + f.versionS + '"]').click();
  await click("恢复这个版本", timeline());
  const review = page.getByRole("dialog", { name: "恢复为一次新保存", exact: true });
  await review.getByRole("checkbox", { name: "保留未保存草稿", exact: true }).check(); return review;
}
const journal = (id, suffix = "plan") => path.join(state, "journal", id + "." + suffix + ".json");
const readJournal = async (id, suffix) => JSON.parse(await readFile(journal(id, suffix), "utf8"));
const planExists = async id => (await readdir(path.join(state, "journal"))).includes(id + ".plan.json");
async function failRefreshOnce(arm) {
  await intercept("**/api/library/open", async route => {
    if (!arm.active) return route.continue(); arm.active = false;
    result.controlledFailures.push({ case: caseName, kind: "HTTP 500 at consumer refresh, after real create response" });
    await route.fulfill({ status: 500, contentType: "application/json", body: JSON.stringify({ error: { code: "probeDeliveryIO", message: "Owned one-shot consumer refresh failure" } }) });
  });
}
async function noteCase() {
  const seen = [], arm = { active: false }; await failRefreshOnce(arm);
  await intercept("**/api/note/create", async route => { seen.push(route.request().postDataJSON());
    const response = await route.fetch(); assert.equal(response.status(), 200); arm.active = true; await route.fulfill({ response }); });
  await page.locator(".sidebar-quick .new-note").click();
  const dialog = page.getByRole("dialog", { name: "新建笔记", exact: true });
  await dialog.getByLabel("笔记名称", { exact: true }).fill("TEST-Created"); await click("创建并打开", dialog);
  await page.locator(".document-tab.selected .tab-name").filter({ hasText: "TEST-Created.typ" }).waitFor();
  await page.locator(".toast").filter({ hasText: "创建结果待确认" }).waitFor();
  const tabs = await page.locator(".document-tab").count(), proof = await inputY();
  const requestCount = result.uiRequests.length, responseCount = result.http.length; await page.locator(".sidebar-quick .new-note").click();
  await until(() => result.uiRequests.slice(requestCount).some(item => item.route === "/api/operations/status" && item.body.operationId === seen[0].operationId), "Original create ID lookup");
  // Wait for successful consumer refresh, rather than count tabs before callback completes.
  await until(() => result.http.slice(responseCount).some(item => item.case === caseName && item.browser && item.route === "/api/library/open" && item.status === 200), "Successful repeated delivery refresh");
  await expectSource(proof.after.source); assert.deepEqual((await snapshot()).selection, proof.after.selection);
  assert.equal(await page.locator(".document-tab").count(), tabs); assert.equal(seen.length, 1);
  assert.equal((await readJournal(seen[0].operationId)).kind, "newNode"); await undoRedo(proof);
  return { operationId: seen[0].operationId, tabs, authorPosts: seen.length, input: proof.after };
}
async function libraryCase() {
  const key = "weftext-library-creation-v1", arm = { active: false }, requests = [];
  const retained = () => page.evaluate(key => JSON.parse(localStorage.getItem(key) || "null"), key);
  await failRefreshOnce(arm);
  await intercept("**/api/library/create", async route => { requests.push(route.request().postDataJSON());
    const response = await route.fetch(); assert.equal(response.status(), 200); arm.active = true; await route.fulfill({ response }); });
  const open = async () => { await page.getByRole("button", { name: "创建资料库", exact: true }).first().click();
    const panel = page.getByRole("dialog", { name: "创建资料库与文档排版语言", exact: true }); await panel.waitFor(); return panel; };
  let panel = await open();
  await panel.getByRole("textbox", { name: "新资料库完整路径", exact: true }).fill(path.join(out, "UI-Created-Library"));
  await panel.getByRole("combobox", { name: "根节点初始内容", exact: true }).selectOption("blank");
  await click("预览创建", panel); await click("创建资料库", panel);
  await panel.getByRole("status").filter({ hasText: "原操作" }).waitFor();
  const original = await retained(); assert.ok(original.request && original.receipt);
  assert.equal(original.request.operationId, original.receipt.operationId);
  await click("关闭", panel); await panel.waitFor({ state: "hidden" });
  const tabs = await page.locator(".document-tab").count(), proof = await inputY();
  panel = await open(); assert.deepEqual((await retained()).request, original.request);
  assert.equal((await retained()).receipt.operationId, original.receipt.operationId);
  await click("确认同一创建操作", panel); await panel.waitFor({ state: "hidden" });
  assert.equal(await retained(), null); assert.equal(requests.length, 1);
  assert.equal(await page.locator(".document-tab").count(), tabs); await expectSource(proof.after.source); await undoRedo(proof);
  return { operationId: original.request.operationId, request: original.request, receipt: original.receipt, tabs, authorPosts: requests.length };
}
async function saveAsCase(f, mode) {
  const target = path.join(out, caseName + ".typ"), retained = path.join(out, caseName + "-original.typ");
  let original, receipt, held = false, release; const requests = [], before = result.uiRequests.length;
  const refresh = { active: false }; if (mode === "current") await failRefreshOnce(refresh);
  await intercept("**/api/save-as", async route => {
    original = route.request().postDataJSON(); requests.push(original);
    const response = await route.fetch(); assert.equal(response.status(), 200); receipt = await response.json();
    assert.ok(receipt.document && receipt.documentSourceMatchesCommit);
    if (mode === "unavailable") {
      await rename(target, retained); await writeFile(target, await readFile(retained));
      result.controlledFailures.push({ case: caseName, kind: "abort real SaveAs response after moving original and installing a same-byte physical substitute" });
      await route.abort("failed");
    } else {
      result.controlledFailures.push({ case: caseName, kind: "hold qualified real SaveAs response while original buffer receives Y" });
      await new Promise(resolve => { release = resolve; held = true; }); await route.fulfill({ response });
    }
  });
  await click("另存为", page.locator(".top-actions"));
  const dialog = page.getByRole("dialog", { name: "另存为", exact: true });
  await dialog.getByLabel("文件路径", { exact: true }).fill(target); await click("另存为", dialog);
  await until(() => !!original, "Original SaveAs request observed");
  if (mode === "current") await until(() => held, "Qualified original SaveAs response held");
  else await page.locator(".save-as-delivery").filter({ hasText: original.operationId }).waitFor();
  const proof = await inputY();
  if (mode === "current") {
    refresh.active = true; release(); await until(async () => (await page.locator(".document-tab.selected .tab-name").innerText()) === path.basename(target), "Original SaveAs Session adoption");
    await until(async () => await page.locator('.top-actions [data-command="save"]').isEnabled(), "SaveAs adoption finished");
    await page.locator(".save-as-delivery").filter({ hasText: original.operationId }).waitFor();
    const retryAt = result.uiRequests.length; await click("保存", page.locator(".top-actions"));
    await until(() => result.uiRequests.slice(retryAt).some(item => item.route === "/api/operations/status" && item.body.operationId === original.operationId), "Adopted SaveAs original-ID delivery retry");
    await page.locator(".save-as-delivery").waitFor({ state: "hidden" });
    await until(async () => await page.locator('.top-actions [data-command="save"]').isEnabled(), "Repeated delivery acknowledged");
  } else {
    const notice = page.locator(".save-as-delivery").filter({ hasText: original.operationId });
    // Wait for the real input's draft and projection callbacks before checking
    // ownership: transient session-error refresh must not erase this receipt.
    await until(() => result.http.some(item => item.case === caseName && item.browser && item.route === "/api/draft" && item.request?.source === proof.after.source && item.status === 200), "Input draft persisted");
    await notice.getByRole("button", { name: "查询这次另存为原结果", exact: true }).waitFor();
    await notice.getByRole("button", { name: "结束这次交付等待，继续当前稿", exact: true }).waitFor();
    assert.equal(await page.locator(".document-error.session-error").count(), 0);
    const journalBefore = Object.fromEntries(await Promise.all(["plan", "commit"].map(async suffix => [suffix, sha(await readFile(journal(original.operationId, suffix)))])));
    await click("查询这次另存为原结果", page.locator(".save-as-delivery"));
    const fact = page.getByRole("dialog", { name: "另存为原提交事实", exact: true });
    assert.match(await fact.innerText(), new RegExp(original.operationId)); assert.match(await fact.innerText(), /documentError/); await click("关闭", fact);
    await click("结束这次交付等待，继续当前稿", page.locator(".save-as-delivery"));
    await page.locator(".save-as-delivery").filter({ hasText: "本次界面交付等待已结束" }).waitFor();
    assert.deepEqual(Object.fromEntries(await Promise.all(["plan", "commit"].map(async suffix => [suffix, sha(await readFile(journal(original.operationId, suffix)))]))), journalBefore);
    assert.equal(await readFile(retained, "utf8"), f.sourceS); assert.equal(await readFile(target, "utf8"), f.sourceS);
  }
  await expectSource(proof.after.source); assert.deepEqual((await snapshot()).selection, proof.after.selection); await undoRedo(proof);
  const nextSave = page.waitForResponse(response => response.url().endsWith("/api/save"));
  await click("保存", page.locator(".top-actions")); const response = await nextSave;
  assert.equal(response.status(), 200, await response.text()); const next = response.request().postDataJSON();
  assert.equal(next.source, proof.after.source); assert.equal(requests.length, 1);
  assert.equal(next.sessionId === original.sessionId, mode === "unavailable");
  assert.equal(await readFile(mode === "unavailable" ? f.a.path : target, "utf8"), proof.after.source);
  assert.equal(await readFile(mode === "unavailable" ? target : f.a.path, "utf8"), f.sourceS);
  assert.equal(result.uiRequests.slice(before).filter(item => item.route === "/api/open" && item.body.path === target).length, 0);
  const originalFact = await api("/api/operations/status", { operationId: original.operationId });
  assert.equal(originalFact.status, "committed"); assert.equal(originalFact.baseId, receipt.baseId);
  if (mode === "unavailable") assert.ok(originalFact.documentError && !originalFact.document);
  return { original, originalReceipt: receipt, continuedSave: next, input: proof.after, endedOnlyUiDelivery: mode === "unavailable" };
}
async function tabIdentityCase(f) {
  const target = path.join(out, caseName + ".typ"), retained = path.join(out, caseName + "-original.typ");
  let original, receipt;
  await intercept("**/api/save-as", async route => {
    original = route.request().postDataJSON(); const response = await route.fetch(); assert.equal(response.status(), 200);
    receipt = await response.json(); await route.fulfill({ response });
  });
  await click("另存为", page.locator(".top-actions"));
  const dialog = page.getByRole("dialog", { name: "另存为", exact: true });
  await dialog.getByLabel("文件路径", { exact: true }).fill(target); await click("另存为", dialog);
  await until(async () => (await page.locator(".document-tab.selected .tab-name").innerText()) === path.basename(target), "Exact original SaveAs target delivery");
  await until(async () => await page.locator('.top-actions [data-command="save"]').isEnabled(), "SaveAs delivery finished");
  await page.locator(".save-as-delivery").waitFor({ state: "hidden" });
  const journalBefore = Object.fromEntries(await Promise.all(["plan", "commit"].map(async suffix => [suffix, sha(await readFile(journal(original.operationId, suffix)))])));
  const identity = await stat(target, { bigint: true });
  await page.keyboard.press("Control+p"); const quick = page.getByRole("dialog", { name: "快捷打开", exact: true });
  await quick.getByRole("searchbox").fill("TEST-B"); await quick.getByRole("searchbox").press("Enter"); await quick.waitFor({ state: "hidden" }); await expectSource(f.b.source);
  await rename(target, retained); await writeFile(target, await readFile(retained));
  const substitute = await stat(target, { bigint: true }); assert.notEqual(identity.ino, substitute.ino);
  const before = result.uiRequests.length;
  await page.locator(".document-tab .tab-name").filter({ hasText: path.basename(target) }).click(); await expectSource(f.sourceS);
  await page.locator("body").filter({ hasText: "原文件会话当前不能重新资格化" }).waitFor();
  const opens = result.uiRequests.slice(before).filter(item => item.route === "/api/open");
  assert.ok(opens.some(item => item.body.sessionId === receipt.document.sessionId)); assert.ok(opens.every(item => !item.body.path));
  const proof = await inputY(); await undoRedo(proof);
  await click("保存", page.locator(".top-actions"));
  await page.locator("body").filter({ hasText: /原稿|原文件|草稿|文件身份/ }).waitFor();
  // Direct Core writer rejection also confirms that a retained old writer has
  // not silently become the same-byte replacement after tab navigation.
  const rejected = await api("/api/save", { sessionId: receipt.document.sessionId, baseId: receipt.document.baseId, source: proof.after.source, operationId: randomUUID() }, 409);
  assert.equal(await readFile(target, "utf8"), f.sourceS); assert.equal(await readFile(retained, "utf8"), f.sourceS);
  assert.deepEqual(Object.fromEntries(await Promise.all(["plan", "commit"].map(async suffix => [suffix, sha(await readFile(journal(original.operationId, suffix)))]))), journalBefore);
  await expectSource(proof.after.source);
  return { original, originalReceipt: receipt, originalPhysicalId: String(identity.ino), substitutePhysicalId: String(substitute.ino), rejected: rejected.error, input: proof.after };
}
async function malformedSaveCase() {
  let original, actual, release, held = false, authors = 0, badIndex = 0;
  await intercept("**/api/save", async route => {
    authors++; original = route.request().postDataJSON(); const response = await route.fetch(); assert.equal(response.status(), 200); actual = await response.json();
    await new Promise(resolve => { release = resolve; held = true; });
    result.controlledFailures.push({ case: caseName, kind: "replace a real committed save response with malformed DTO; not a natural backend failure" });
    await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({ operationId: original.operationId, status: "saved" }) });
  });
  const submitted = await inputY(); await click("保存", page.locator(".top-actions")); await until(() => held, "Real save response held");
  const proof = await inputY(); release();
  await until(async () => await page.locator('.top-actions [data-command="save"]').isEnabled(), "Malformed initial save retains pending request");
  const changes = [{ baseId: null }, { path: 12 }, { document: { ...actual.document, sessionId: null } }];
  await intercept("**/api/operations/status", async route => {
    const request = route.request().postDataJSON(); if (request.operationId !== original.operationId || badIndex >= changes.length) return route.continue();
    const response = await route.fetch(); assert.equal(response.status(), 200); const good = await response.json();
    result.controlledFailures.push({ case: caseName, kind: "malformed status DTO", change: changes[badIndex] });
    await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({ ...good, ...changes[badIndex++] }) });
  });
  for (let i = 0; i < changes.length + 1; i++) {
    const response = page.waitForResponse(value => value.url().endsWith("/api/operations/status"));
    await click("保存", page.locator(".top-actions")); await response;
    await until(async () => await page.locator('.top-actions [data-command="save"]').isEnabled(), "Original status retry completes");
    assert.equal(authors, 1); await expectSource(proof.after.source);
  }
  assert.equal(badIndex, changes.length); assert.deepEqual((await snapshot()).selection, proof.after.selection); await undoRedo(proof);
  await page.unroute("**/api/save");
  const nextResponse = page.waitForResponse(value => value.url().endsWith("/api/save")); await click("保存", page.locator(".top-actions"));
  const next = await nextResponse; assert.equal(next.status(), 200, await next.text());
  assert.notEqual(next.request().postDataJSON().operationId, original.operationId);
  assert.equal(next.request().postDataJSON().source, proof.after.source);
  return { original, actualReceipt: actual, submitted: submitted.after, input: proof.after, malformedStatusCount: badIndex, originalAuthorPosts: authors };
}
async function historyCase(f, mode) {
  let original, committed, staleDraftSession, resolveHeld = false, releaseResolve, draftInjected = false;
  if (mode === "resolve") {
    // Keep a genuine older Session without changing the current buffer's bytes.
    // Its later draft is a real conflicting Core draft, not a substituted DTO.
    const current = await api("/api/open", { path: f.a.path, root: f.root });
    const older = await save(current, f.sourceS + "\nOlder independent draft basis C\n");
    staleDraftSession = await api("/api/open", { path: f.a.path, root: f.root });
    await save(older, f.sourceS);
    // The independent C -> S writes changed the real base. Reopen through the
    // actual navigation consumer before reviewing a restore from this UI.
    await navigation(f);
    await intercept("**/api/operations/status", async route => {
      if (route.request().postDataJSON().operationId === original?.operationId && !draftInjected) {
        draftInjected = true;
        await api("/api/draft", { sessionId: staleDraftSession.sessionId, source: staleDraftSession.source + "\nProtected independent D\n" });
      }
      await route.continue();
    });
    await intercept("**/api/draft/resolve", async route => {
      const response = await route.fetch(); assert.equal(response.status(), 200);
      result.controlledFailures.push({ case: caseName, kind: "hold real successful draft resolution response while author inputs Y" });
      await new Promise(resolve => { releaseResolve = resolve; resolveHeld = true; });
      await route.fulfill({ response });
    });
  }
  await intercept("**/api/history/restore", async route => {
    original = route.request().postDataJSON();
    if (mode !== "fence") { const response = await route.fetch(); assert.equal(response.status(), 200);
      committed = await response.json(); assert.equal(committed.status, "committed"); }
    result.controlledFailures.push({ case: caseName, operationId: original.operationId, kind: mode === "fence"
      ? "request aborted before server: no Core author effect" : "response aborted after actual Core commit: transport failure only" });
    await route.abort("failed");
  });
  const review = await prepareHistory(f); await click("确认恢复并新保存", review); await review.waitFor({ state: "hidden" });
  await timeline().locator(".history-operation").filter({ hasText: original.operationId }).waitFor();
  if (mode === "fence") assert.equal(await planExists(original.operationId), false);
  await closeTimeline();
  if (mode !== "resolve") await navigation(f);
  let proof = mode === "fence" || mode === "resolve" ? undefined : await inputY();
  let sourceT;
  if (mode === "later") { const doc = await api("/api/open", { path: f.a.path, root: f.root });
    sourceT = doc.source + "\nLater independently committed T\n"; await save(doc, sourceT); }
  await openTimeline(); await timeline().locator(".history-operation").filter({ hasText: original.operationId }).waitFor();
  await click("查询原恢复结果", timeline());
  if (mode === "resolve") {
    await until(() => resolveHeld, "Actual draft resolution response held");
    assert.equal(draftInjected, true); await closeTimeline(); proof = await inputY();
    releaseResolve();
    await until(async () => await page.locator('.top-actions [data-command="save"]').isEnabled(), "History delivery acknowledges after resolve");
    await expectSource(proof.after.source); await openTimeline();
  }
  if (mode === "fence") {
    await until(async () => (await api("/api/history/status", { operationId: original.operationId })).status === "resolved", "Exact missing attempt settlement");
    await closeTimeline(); assert.equal(await page.locator('.top-actions [data-command="save"]').isEnabled(), true);
    const late = await api("/api/history/restore", original, 409);
    assert.match(late.error.code, /cancel/i); assert.equal(await planExists(original.operationId), false);
    return { original, resolution: await readJournal(original.operationId, "resolution"), lateError: late.error };
  }
  if (mode === "later") {
    await timeline().locator(".history-operation").filter({ hasText: original.operationId }).waitFor();
    await closeTimeline(); await expectSource(proof.after.source); assert.equal(await readFile(f.a.path, "utf8"), sourceT);
    assert.equal(await page.locator('.top-actions [data-command="save"]').isDisabled(), true);
    await page.locator(".document-error.session-error").filter({ hasText: "磁盘内容已改变" }).waitFor();
    await undoRedo(proof);
  } else {
    // Review before use if final UI keeps an acknowledged fact visible: then
    // assert its typed pending flag/action state, not removal of all ID text.
    await until(async () => !(await timeline().locator(".history-operation").innerText()).includes(original.operationId), "Current receipt acknowledgement");
    await closeTimeline(); await expectSource(proof.after.source);
    assert.deepEqual((await snapshot()).selection, proof.after.selection); await undoRedo(proof);
    const nextSave = page.waitForResponse(response => response.url().endsWith("/api/save"));
    await click("保存", page.locator(".top-actions")); const response = await nextSave;
    assert.equal(response.status(), 200, await response.text());
    const request = response.request().postDataJSON(); assert.notEqual(request.sessionId, original.sessionId);
    assert.equal(request.source, proof.after.source); assert.equal(await readFile(f.a.path, "utf8"), proof.after.source);
    result.continuedSave = { request, response: await response.json() };
  }
  assert.equal(result.uiRequests.filter(item => item.case === caseName && item.route === "/api/history/restore").length, 1);
  const fact = await api("/api/history/status", { operationId: original.operationId });
  assert.equal(fact.status, "committed"); assert.equal(fact.receipt.baseId, committed.receipt.baseId);
  return { original, originalReceipt: committed.receipt, sourceT, input: proof.after, resolveHeld, draftInjected };
}
try {
  for (const name of cases.filter(name => selected === "all" || name === selected)) {
    caseName = name; await start(); const fixture = await seed(); await stop(); await start(fixture.a); await browse(fixture);
    try {
      const proof = name === "note" ? await noteCase() : name === "library" ? await libraryCase() : name === "save-as-tab-identity" ? await tabIdentityCase(fixture) : name === "save-malformed" ? await malformedSaveCase() : name.startsWith("save-as-") ? await saveAsCase(fixture, name.slice("save-as-".length))
        : await historyCase(fixture, name.slice("history-".length));
      result.cases.push({ name, proof }); await page.screenshot({ path: path.join(out, name + "-passed.png"), fullPage: true });
      await writeFile(path.join(out, "progress.json"), JSON.stringify(result, null, 2));
    } finally { await closeBrowser(); await stop(); }
  }
  assert.deepEqual(result.pageErrors, []); assert.deepEqual(result.routeErrors, []); result.completed = true;
} catch (error) {
  result.error = String(error.stack || error); process.exitCode = 1;
  if (page) { result.failureBody = await page.locator("body").innerText().catch(() => "unavailable");
    await page.screenshot({ path: path.join(out, "failure.png"), fullPage: true }).catch(() => {}); }
} finally {
  try { await closeBrowser(); } catch (error) { result.browserCloseError = String(error); process.exitCode = 1; }
  try { await stop(); } catch (error) { result.processCloseError = String(error); process.exitCode = 1; }
  result.ownedResourcesClosed = !child && !context && result.processes.every(item => item.listenerClosed === true);
  await writeFile(path.join(out, "result.json"), JSON.stringify(result, null, 2));
  await writeFile(path.join(out, "http.jsonl"), result.http.map(item => JSON.stringify(item)).join("\n") + "\n");
}
process.stdout.write(JSON.stringify({ completed: result.completed, ownedResourcesClosed: result.ownedResourcesClosed,
  executableSha256: result.executableSha256, cases: result.cases.map(item => item.name), evidence: path.join(out, "result.json") }, null, 2) + "\n");
