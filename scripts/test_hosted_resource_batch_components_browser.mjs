// Actual HostedResourceImports UI/state with controlled transport; not Core or HTTP acceptance.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { pathToFileURL } from "node:url";
const repo = path.resolve(import.meta.dirname, ".."), out = process.env.WEFTEXT_BROWSER_REPORT;
assert.ok(out && path.isAbsolute(out)); await mkdir(out);
const sources = ["app/src/hosted-media-import.ts", "app/src/hosted-media-import-state.ts", "app/src/hosted-import-wire.ts", "app/src/ui-messages.ts", "assets/ui-messages.json", "scripts/test_hosted_resource_batch_components_browser.mjs"];
const inventory = async () => Promise.all(sources.map(async name => ({ name, sha256: createHash("sha256").update(await readFile(path.join(repo, name))).digest("hex") })));
const result = { scope: "Actual HostedResourceImports batch UI/locator/retirement with controlled request ports; no real import, ACL, Core, native or whole-product acceptance", checks: [], errors: [], sourceBefore: await inventory(), ownership: { browserClosed: false, viteClosed: false }, completed: false };
const { chromium } = createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE || "playwright");
const { createServer } = await import(pathToFileURL(path.join(repo, "app/node_modules/vite/dist/node/index.js")).href);
let vite, browser;
const check = async (name, work) => { const item = { name, passed: false }; result.checks.push(item); await work(); item.passed = true; };
try {
    vite = await createServer({ root: path.join(repo, "app"), configFile: false, server: { host: "127.0.0.1", port: 0, hmr: false, watch: { ignored: ["**/*"] } } }); await vite.listen();
    browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE, headless: true });
    const page = await browser.newPage(); page.on("pageerror", error => result.errors.push(error.message)); page.setDefaultTimeout(15000);
    await page.route("**/resource-batch-fixture", route => route.fulfill({ contentType: "text/html", body: '<!doctype html><meta charset="utf-8"><main id="import"></main>' }));
    await page.goto("http://127.0.0.1:" + vite.httpServer.address().port + "/resource-batch-fixture");
    await page.evaluate(async () => {
        const [{ HostedResourceImports }, { message, locale }, { hostedCachePrefix }] = await Promise.all([import("/src/hosted-media-import.ts"), import("/src/ui-locale.ts"), import("/src/hosted-state.ts")]);
        const host = document.getElementById("import"), operations = new Map(), calls = [];
        let actor = "alice", generation = 1, hold = false, release, changed = 0, originalBeforeEffect = false;
        const catalogs = writable => ["one", "two"].map((collection, index) => ({ collection, owner: index ? "Other" : "Shared", title: collection, lifecycle: "live", read: false, download: false, import: writable, status: "ready" }));
        const cacheKey = () => hostedCachePrefix("resource-import", "component-deployment", actor, generation) + "state";
        const key = (operationId, capturedActor = actor) => capturedActor + ":" + operationId;
        const response = (operationId, collection, status = "notRegistered") => ({ collection, owner: collection === "one" ? "Shared" : "Other", operationId, status, planId: null, preview: {}, resourceRefs: [], assets: [], results: [], commitAllowed: false });
        const owner = new HostedResourceImports(host, async (route, body) => {
            const capturedActor = actor; calls.push({ route, body: structuredClone(body), actor: capturedActor });
            const identity = key(body.operationId, capturedActor);
            if (route === "media/import-batch/prepare") {
                const raw = sessionStorage.getItem(cacheKey()), locator = raw && JSON.parse(raw);
                if (!locator?.pending || locator.pending.request.operationId !== body.operationId || raw.includes("bytesBase64")) throw new Error("No small original locator before effect");
                originalBeforeEffect = true;
                const receipt = { ...response(body.operationId, body.collection, "prepared"), planId: "plan-" + body.operationId, preview: { summary: message("common.save"), totalBytes: body.files.reduce((sum, file) => sum + atob(file.bytesBase64).length, 0), files: body.files.map(file => ({ name: file.name, bytes: atob(file.bytesBase64).length, contentType: "image/png", metadata: { kind: "image", width: 1, height: 1 } })) }, commitAllowed: true };
                operations.set(identity, receipt);
                if (hold) return new Promise(resolve => { release = () => resolve(receipt); });
                return receipt;
            }
            if (route === "media/import-batch/operation") return structuredClone(operations.get(identity) ?? response(body.operationId, body.collection));
            if (route === "media/import-batch/settle") { const stopped = response(body.operationId, body.collection, "cancelled"); operations.set(identity, stopped); return stopped; }
            if (route === "media/import-batch/commit") {
                const previous = operations.get(identity);
                if (previous?.planId !== body.planId || !previous.commitAllowed) throw new Error("Wrong original batch plan");
                const refs = ["weftext-resource://workspace/z", "weftext-resource://workspace/a"].slice(0, previous.preview.files.length);
                const committed = { ...response(body.operationId, body.collection, "committed"), resourceRefs: [...refs].sort(), assets: [...refs].sort().map(ref => body.collection + "/" + ref.split("/").at(-1)), results: previous.preview.files.map((file, index) => ({ name: file.name, resourceRef: refs[index], exactRevision: crypto.randomUUID() })) };
                operations.set(identity, committed); return committed;
            }
            throw new Error("Unexpected resource batch route " + route);
        }, async () => { changed++; });
        const fixture = { owner, locale, calls, get changed() { return changed; }, get beforeEffect() { return originalBeforeEffect; }, get original() { return JSON.parse(sessionStorage.getItem(cacheKey()) ?? "null"); },
            connect(nextActor = "alice", nextGeneration = 1, writable = true) { actor = nextActor; generation = nextGeneration; owner.connect("component-deployment", actor, generation); owner.setCatalog(catalogs(writable)); },
            held() { hold = true; release = undefined; }, release() { if (!release) throw new Error("No held request"); hold = false; release(); }, get holding() { return !!release; },
            setStatus(status) { const original = this.original.pending.request; operations.set(key(original.operationId), response(original.operationId, original.collection, status)); },
            retireCollection() { owner.setCatalog(catalogs(false)); }, clear() { owner.clear(); },
        };
        fixture.connect(); window.resourceBatch = fixture;
    });
    const host = page.locator("#import"), names = () => host.locator(".resource-import-files input"), card = () => host.locator(".resource-import-pending");
    const select = async (files = [{ name: "z-first.png", mimeType: "image/png", buffer: Buffer.from([1, 2, 3]) }, { name: "a-second.png", mimeType: "image/png", buffer: Buffer.from([4, 5]) }]) => {
        await host.getByLabel("托管资源导入原件", { exact: true }).setInputFiles(files);
        await page.waitForFunction(() => [...document.querySelectorAll("#import button")].some(button => button.textContent === "准备资源集合导入" && !button.disabled));
    };
    const click = name => host.getByRole("button", { name, exact: true }).click();
    const idle = () => page.waitForFunction(() => [...document.querySelectorAll("#import button")].some(button => button.textContent === "查询原资源导入操作" && !button.disabled));
    await check("multiple real Files keep exact order and editable per-file names, stable focus and no cached upload bytes", async () => {
        assert.equal(await host.getByLabel("托管资源导入原件", { exact: true }).getAttribute("multiple"), "");
        await select(); assert.equal(await names().count(), 2);
        await page.evaluate(() => { window.nameNode = document.querySelectorAll(".resource-import-files input")[1]; });
        await names().nth(1).fill("renamed-second.png");
        assert.equal(await page.evaluate(() => window.nameNode === document.querySelectorAll(".resource-import-files input")[1] && document.activeElement === window.nameNode), true);
        const cache = await page.evaluate(() => window.resourceBatch.original);
        assert.deepEqual(cache.files.map(file => file.name), ["z-first.png", "renamed-second.png"]); assert.ok(cache.files.every(file => /^[a-f0-9]{64}$/.test(file.sha256)));
        assert.doesNotMatch(JSON.stringify(cache), /bytesBase64|AQID/); assert.match(await host.innerText(), /2 项，共 5 字节/);
    });
    await check("one stored original precedes prepare; full ordered preview and committed identities do not insert document text", async () => {
        await click("准备资源集合导入"); await idle();
        assert.equal(await page.evaluate(() => window.resourceBatch.beforeEffect), true);
        assert.equal(await card().locator("ol li").count(), 2);
        await click("确认提交原资源导入计划"); await idle();
        const cache = await page.evaluate(() => window.resourceBatch.original), calls = await page.evaluate(() => window.resourceBatch.calls);
        assert.equal(new Set(calls.map(call => call.body.operationId)).size, 1);
        assert.deepEqual(cache.pending.receipt.results.map(file => file.name), ["z-first.png", "renamed-second.png"]);
        assert.match(await host.innerText(), /本批资源尚未插入正文/); assert.equal(await page.evaluate(() => window.resourceBatch.changed), 1);
        await click("清除本页已结束导入回执，保留文件输入");
    });
    await check("reload restores only its locator and queries the same ID; another upload cannot replace an unresolved original", async () => {
        await select(); await click("准备资源集合导入"); await idle();
        const original = await page.evaluate(() => window.resourceBatch.original.pending.request.operationId), before = await page.evaluate(() => window.resourceBatch.calls.filter(call => call.route.endsWith("/prepare")).length);
        await page.evaluate(() => window.resourceBatch.connect()); await idle();
        assert.equal(await host.getByRole("button", { name: "按原上传请求重试准备", exact: true }).isDisabled(), true);
        assert.equal(await page.evaluate(() => window.resourceBatch.original.pending.request.operationId), original);
        await host.getByLabel("托管资源导入原件", { exact: true }).setInputFiles({ name: "later.png", buffer: Buffer.from([9]) });
        await page.waitForFunction(() => window.resourceBatch.original.files[0]?.sha256);
        assert.equal(await host.getByRole("button", { name: "准备资源集合导入", exact: true }).isDisabled(), true);
        assert.equal(await page.evaluate(() => window.resourceBatch.calls.filter(call => call.route.endsWith("/prepare")).length), before);
        await click("结束未安装的原资源导入"); await idle(); await click("清除本页已结束导入回执，保留文件输入");
    });
    await check("metadata storage refusal sends no effect and retains the actual file configuration", async () => {
        await select(); const before = await page.evaluate(() => window.resourceBatch.calls.length);
        await page.evaluate(() => { window.originalStorageSet = Storage.prototype.setItem; Storage.prototype.setItem = function(key, value) { if (key.startsWith("weftext-hosted-resource-import:")) throw new DOMException("controlled quota", "QuotaExceededError"); return window.originalStorageSet.call(this, key, value); }; });
        await click("准备资源集合导入"); await host.getByText(/恢复缓存容量不足或不可写/).waitFor();
        assert.equal(await page.evaluate(() => window.resourceBatch.calls.length), before);
        assert.equal(await host.getByLabel("托管资源导入原件", { exact: true }).evaluate(input => input.files.length), 2);
        await page.evaluate(() => { Storage.prototype.setItem = window.originalStorageSet; });
    });
    await check("invalid name failure preserves both real Files for correction and does not register an original", async () => {
        await names().first().fill("../invalid.png"); const before = await page.evaluate(() => window.resourceBatch.calls.length);
        await click("准备资源集合导入"); await host.getByText(/当前批次尚未完整/).waitFor();
        assert.equal(await page.evaluate(() => window.resourceBatch.calls.length), before);
        assert.equal(await names().first().inputValue(), "../invalid.png");
        await names().first().fill("fixed.png");
    });
    await check("later names and collection ABA cannot authorize the frozen batch; same-tab retry retains its exact original wire", async () => {
        await page.evaluate(() => window.resourceBatch.held()); await click("准备资源集合导入"); await page.waitForFunction(() => window.resourceBatch.holding);
        await names().first().fill("newer.png"); await names().first().fill("fixed.png");
        await page.evaluate(() => window.resourceBatch.release()); await idle();
        assert.equal(await host.getByRole("button", { name: "确认提交原资源导入计划", exact: true }).isDisabled(), true);
        await click("按原上传请求重试准备"); await idle();
        const requests = await page.evaluate(() => window.resourceBatch.calls.filter(call => call.route.endsWith("/prepare")).slice(-2).map(call => call.body)); assert.deepEqual(requests[0], requests[1]);
        await host.getByLabel("托管资源导入集合", { exact: true }).selectOption("two");
        assert.equal(await card().locator("ol li").count(), 0);
        await click("结束未安装的原资源导入"); await idle(); await click("清除本页已结束导入回执，保留文件输入");
    });
    await check("held old actor/generation result and revoked collection cannot refill a replacement owner", async () => {
        await select(); await page.evaluate(() => window.resourceBatch.held()); await click("准备资源集合导入"); await page.waitForFunction(() => window.resourceBatch.holding);
        const beforeChanged = await page.evaluate(() => window.resourceBatch.changed);
        await page.evaluate(() => window.resourceBatch.connect("reader", 2, false)); await page.evaluate(() => window.resourceBatch.release());
        await page.waitForFunction(() => !document.querySelector(".resource-import-pending"));
        assert.equal(await names().count(), 0); assert.equal(await page.evaluate(() => window.resourceBatch.changed), beforeChanged);
        assert.equal(await host.getByRole("button", { name: "准备资源集合导入", exact: true }).isDisabled(), true);
        await page.evaluate(() => window.resourceBatch.connect("new-actor", 3)); await select();
        await page.evaluate(() => window.resourceBatch.held()); await click("准备资源集合导入"); await page.waitForFunction(() => window.resourceBatch.holding);
        await page.evaluate(() => { window.resourceBatch.retireCollection(); window.resourceBatch.release(); }); await idle();
        assert.equal(await card().locator("ol li").count(), 0);
        assert.equal(await host.getByRole("button", { name: "确认提交原资源导入计划", exact: true }).isDisabled(), true);
    });
    await check("over-count File selection is retained visibly and rejected before any request", async () => {
        await page.evaluate(() => window.resourceBatch.connect("over-count", 4));
        const before = await page.evaluate(() => window.resourceBatch.calls.length);
        await host.getByLabel("托管资源导入原件", { exact: true }).setInputFiles(Array.from({ length: 65 }, (_, index) => ({ name: "file-" + index + ".bin", buffer: Buffer.from([index]) })));
        await host.getByText(/请选择 1–64 个文件/).waitFor();
        assert.equal(await names().count(), 65);
        assert.equal(await host.getByLabel("托管资源导入原件", { exact: true }).evaluate(input => input.files.length), 65);
        assert.equal(await page.evaluate(() => window.resourceBatch.calls.length), before);
        assert.equal(await host.getByRole("button", { name: "准备资源集合导入", exact: true }).isDisabled(), true);
    });
    await check("old single or corrupt closed records stay explicit unreadable input without a parallel operation path", async () => {
        await page.evaluate(() => { window.resourceBatch.clear(); sessionStorage.setItem("weftext-hosted-resource-import:component-deployment:legacy:g4:state", JSON.stringify({ collection: "one", name: "old single", revision: 1, bytesBase64: "AQID" })); window.resourceBatch.connect("legacy", 4); });
        assert.equal(await host.getByLabel("托管资源导入原件", { exact: true }).isDisabled(), true);
        assert.match(await host.innerText(), /old single/); assert.equal(await names().count(), 0);
        await page.evaluate(() => window.resourceBatch.connect("fresh", 5));
        assert.equal(await host.getByLabel("托管资源导入原件", { exact: true }).isEnabled(), true);
    });
    assert.equal((await page.evaluate(() => window.resourceBatch.calls)).some(call => call.route.startsWith("media/import/")), false);
    await page.screenshot({ path: path.join(out, "resource-batch-components.png"), fullPage: true });
    await page.evaluate(() => window.resourceBatch.clear());
    assert.deepEqual(result.errors, []); result.sourceAfter = await inventory(); assert.deepEqual(result.sourceAfter, result.sourceBefore); result.completed = true;
} catch (error) { result.failure = error.stack ?? String(error); process.exitCode = 1; }
finally { if (browser) { await browser.close(); result.ownership.browserClosed = true; } if (vite) { await vite.close(); result.ownership.viteClosed = true; } await writeFile(path.join(out, "result.json"), JSON.stringify(result, null, 2) + "\n"); }
console.log(JSON.stringify({ completed: result.completed, checks: result.checks.filter(check => check.passed).length, out }));
