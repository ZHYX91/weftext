// Actual birthday buttons, CaptureFields, LocalCaptureSession/full-input outbox,
// IndexedDB and fresh controller/producer registry/close guard after recreation.
// The Core transport is controlled: no real Core, filesystem save, native or IME claim.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { pathToFileURL } from "node:url";

const repo = path.resolve(import.meta.dirname, ".."), out = process.argv[2];
assert.equal(process.argv.length, 3, "Usage: node scripts/test_birthday_capture_recovery_browser.mjs NEW_ABSOLUTE_OUTPUT");
assert.ok(out && path.isAbsolute(out));
assert.ok(path.relative(repo, out).startsWith(".." + path.sep), "Keep private evidence outside the product repository");
await mkdir(out);
const inputs = ["scripts/test_birthday_capture_recovery_browser.mjs", "app/src/birthday.ts", "app/src/capture-fields.ts", "app/src/capture-local.ts", "app/src/capture-controller.ts", "app/src/capture-close.ts", "app/src/capture-local-persistence.ts", "app/src/capture-input-persistence.ts", "app/src/capture-author-outbox.ts", "app/src/capture-draft-state.ts", "app/src/capture-private-disposition.ts", "app/src/personal-control-input.ts", "app/src/personal-input-recovery.ts", "app/src/personal-input-storage.ts", "app/src/author-input.ts", "app/src/native-value-form.ts", "app/src/native-control-identity.ts", "app/src/builtin-property-labels.ts", "app/src/ui-locale.ts", "app/src/ui-messages.ts", "assets/ui-messages.json", "docs/design/examples/schema.json"];
const sha = value => createHash("sha256").update(value).digest("hex");
const inventory = async () => Promise.all(inputs.map(async file => ({ path: file, sha256: sha(await readFile(path.join(repo, file))) })));
const result = {
    schemaVersion: 1, scope: "Current-source actual birthday button -> CaptureFields -> LocalCaptureSession/Controller -> full-input outbox/real IndexedDB -> fresh recovery/session/controller/registry/fields/close guard; controlled Core transport",
    limits: ["Core mutation/status/flush responses are controlled, not real Core or durable author-file acceptance.", "Destroy/rebuild recreates every frontend owner and rereads real IndexedDB in the same browser page; it is not OS crash, full browser restart or native close acceptance.", "Unacknowledged non-text choices retain the original full-input mutation locator/fingerprint and close barrier, not a fabricated full candidate in the generic personal-text store."],
    cases: [], pageErrors: [], completed: false, sourceBefore: await inventory(), ownership: { browserClosed: false, viteClosed: false },
};
const { chromium } = createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE || "playwright");
const { createServer } = await import(pathToFileURL(path.join(repo, "app/node_modules/vite/dist/node/index.js")).href);
let vite, browser, page;
async function check(name, run) {
    const item = { name, passed: false }; result.cases.push(item);
    try { item.observation = await run(); item.passed = true; }
    catch (error) {
        item.failure = String(error.stack || error);
        if (page) item.observation = await page.evaluate(() => window.fixture?.snapshot()).catch(() => null);
        throw error;
    }
}
try {
    vite = await createServer({ root: path.join(repo, "app"), configFile: false, server: { host: "127.0.0.1", port: 0, hmr: false, watch: { ignored: ["**/*"] } } });
    await vite.listen();
    browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE || "C:/Program Files/Google/Chrome/Application/chrome.exe", headless: true });
    result.browser = browser.version();
    page = await browser.newPage({ viewport: { width: 1100, height: 850 } });
    page.on("pageerror", error => result.pageErrors.push(error.message));
    await page.route("**/birthday-capture-recovery", route => route.fulfill({ contentType: "text/html", body: '<!doctype html><meta charset="utf-8"><link rel="stylesheet" href="/src/data-view.css"><style>body{font:14px sans-serif;margin:12px}main{max-width:850px}fieldset{min-width:0}input,select,textarea{max-width:100%;box-sizing:border-box}</style><main id="capture"></main>' }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/birthday-capture-recovery`);
    const schema = JSON.parse(await readFile(path.join(repo, "docs/design/examples/schema.json"), "utf8"));
    await page.evaluate(async schema => {
        const [{ CaptureFields }, { LocalCaptureSession }, { CaptureCloseGuard }, capture, { PersonalControlInput }, { PersonalInputRecovery }, birthday, { locale, t, message }] = await Promise.all([
            import("/src/capture-fields.ts"), import("/src/capture-local.ts"), import("/src/capture-close.ts"), import("/src/capture-draft-state.ts"), import("/src/personal-control-input.ts"), import("/src/personal-input-recovery.ts"), import("/src/birthday.ts"), import("/src/ui-locale.ts"),
        ]);
        locale.set("en-US");
        const fieldId = "people/life_events", basis = { schemaRef: "weftext-config://recovery-fixture/schema", semanticDigest: "a".repeat(64) };
        const text = value => ({ kind: "text", value }), bool = value => ({ kind: "bool", value });
        const record = value => ({ kind: "record", value: Object.entries(value).map(([key, value]) => ({ key, value })) });
        const initial = { kind: "array", value: [
            record({ kind: text("birth"), selected: bool(true), note: text("Protected original A 😀\r\n"), sources: { kind: "array", value: [record({ reference: text("https://example.invalid/birth-a"), note: text("Retain source A") })] } }),
            record({ kind: text("death"), selected: bool(true), note: text("Independent death remains selected") }),
            record({ kind: text("birth"), selected: bool(true), note: text("Protected original B"), unknownEvidence: record({ code: text("Retain unknown evidence") }) }),
        ] };
        const host = document.querySelector("#capture");
        const fixture = window.fixture = {
            initial, fieldId, calls: [], dead: [], current: null, genericRetains: 0,
            labels: { choose: t(message("birthday.choose")), add: t(message("birthday.add")) },
            candidate(input) { const entry = input.fields.find(item => item.fieldId === fieldId); return entry && capture.captureFieldCandidate(entry); },
            async reset() {
                this.destroy(); host.replaceChildren(); this.calls = []; this.dead = []; this.genericRetains = 0;
                this.deviceId = crypto.randomUUID(); this.draftId = crypto.randomUUID(); this.mode = "normal"; this.lookup = "normal";
                const input = capture.emptyCaptureInput(); input.typeIds = ["people/person"]; input.formBasis = structuredClone(basis);
                input.fields = [{ fieldId, valueState: { kind: "valid", value: structuredClone(initial) } }];
                this.durable = { version: 2, draftId: this.draftId, revision: 1, inputRevision: 1, inputCut: "full", lastMutation: { kind: "create", mutationId: crypto.randomUUID(), fingerprint: "b".repeat(64), resultingRevision: 1 }, currentInput: input, pending: null, target: null };
                this.audience = JSON.stringify(["local-device", this.deviceId]);
                await this.open();
                // Preserve the existing protected raw cut before the button-only
                // action. This is normal Capture collection, not authored text.
                this.current.fields.collectOwnedRaw(); await this.current.session.controller.drain();
                await this.current.recovery.flush(this.audience);
                this.baseline = structuredClone(this.durable); this.calls = [];
            },
            async request(route, body) {
                this.calls.push({ route, body: structuredClone(body) });
                if (route === "/api/capture/identity") return { id: this.deviceId, isCurrent: true };
                if (route === "/api/capture/draft/read") { if (body.draftId !== this.draftId) throw Error("Wrong draft"); return structuredClone(this.durable); }
                if (route === "/api/capture/draft/input") {
                    if (this.mode === "reject") throw Error("CONTROLLED original input did not apply");
                    if (body.draftId !== this.draftId || body.expectedRevision !== this.durable.revision) throw Error("Controlled Core input CAS failed");
                    const revision = this.durable.revision + 1;
                    this.durable = { ...this.durable, revision, inputRevision: this.durable.inputRevision + 1, currentInput: structuredClone(body.input), lastMutation: { kind: "input", mutationId: body.mutationId, fingerprint: await capture.captureInputFingerprint(body.expectedRevision, body.input), resultingRevision: revision } };
                    if (this.mode === "lost") throw Error("CONTROLLED response lost after full input applied");
                    return structuredClone(this.durable);
                }
                if (route === "/api/capture/draft/mutation" || route === "/api/capture/draft/input-mutation") {
                    if (this.lookup === "unavailable") throw Error("CONTROLLED original lookup unavailable");
                    const matches = body.draftId === this.draftId && body.mutationId === this.durable.lastMutation.mutationId;
                    let fingerprint = body.fingerprint;
                    if (body.command) {
                        if (body.command.kind !== "input") throw Error("Unexpected non-input original lookup");
                        fingerprint = await capture.captureInputFingerprint(body.command.expectedRevision, body.command.input);
                    }
                    const expectedRevision = body.command?.expectedRevision ?? body.expectedRevision;
                    const status = this.lookup === "unknown" || !matches ? "supersededOrUnknown"
                        : fingerprint === this.durable.lastMutation.fingerprint && expectedRevision + 1 === this.durable.revision ? "applied" : "differentCommand";
                    const draft = structuredClone(this.durable);
                    // The actual input-mutation route returns pure recovery
                    // metadata, not the protected full candidate. The session
                    // must separately read that candidate after proof recovery.
                    const recovered = route.endsWith("/input-mutation") ? {
                        draftId: draft.draftId, revision: draft.revision, inputRevision: draft.inputRevision,
                        inputCut: draft.inputCut, lastMutation: draft.lastMutation, pending: null,
                        ...capture.captureAuthorInput(draft.currentInput),
                    } : draft;
                    if (this.lookup === "wrong-proof") { recovered.lastMutation.fingerprint = "f".repeat(64); return { status: "applied", draft: recovered }; }
                    return { status, draft: recovered };
                }
                if (route === "/api/capture/draft/flush") {
                    if (body.draftId !== this.draftId || body.recordRevision !== this.durable.revision || body.inputRevision !== this.durable.inputRevision || body.lastMutationId !== this.durable.lastMutation.mutationId) throw Error("Flush must name exact current full cut");
                    return { draftId: this.draftId, recordRevision: this.durable.revision, inputRevision: this.durable.inputRevision, inputCut: "full", lastMutationId: this.durable.lastMutation.mutationId, pendingOperationId: null, recordDigest: "c".repeat(64) };
                }
                throw Error("Unexpected route: " + route);
            },
            async open() {
                if (this.current) throw Error("Dispose all old owners before rebuilding");
                const recovery = new PersonalInputRecovery();
                const session = await LocalCaptureSession.open(this.request.bind(this), { draftId: this.draftId, recovery });
                let live = true;
                const inputs = new PersonalControlInput(document, () => live, () => { this.genericRetains++; throw Error("A choice must never enter generic personal text recovery"); });
                const authorInputs = (owner, current, sink, revision) => inputs.session("birthday-capture-recovery", 1, owner, () => live && current(), sink, revision);
                const fields = new CaptureFields(host, session.controller, authorInputs, { schema, ...basis, allowedFields: [fieldId] });
                let inert = false;
                const guard = new CaptureCloseGuard(session.controller, inputs, { assertResolved: () => session.assertRecovered(), collectOwnedRaw: () => fields.collectOwnedRaw(), freeze: () => { inert = true; host.inert = true; }, thaw: () => { inert = false; host.inert = false; } });
                this.current = { recovery, session, inputs, fields, guard, live: () => live, inert: () => inert, dispose: () => { live = false; fields.dispose(); session.controller.retire(); inputs.clear(); host.inert = false; host.replaceChildren(); } };
            },
            destroy() {
                if (!this.current) return;
                const old = this.current; old.dispose();
                this.dead.push({ ownerGeneration: old.session.controller.ownerGeneration, producers: old.inputs.inspect(), controller: old.session.controller, session: old.session, inputs: old.inputs, recovery: old.recovery, fields: old.fields });
                this.current = null;
            },
            async settleFailure() {
                let failed = false;
                try { await this.current.session.controller.drain(); } catch { failed = true; }
                await this.current.recovery.flush(this.audience);
                if (!failed) throw Error("The original input was expected to remain unconfirmed");
                return this.snapshot();
            },
            async close() {
                try { const proof = await this.current.guard.prepare(); return { accepted: true, proof }; }
                catch (error) { return { accepted: false, error: String(error) }; }
            },
            supersedeWithoutChangingBytes() {
                const revision = this.durable.revision + 1;
                this.durable = { ...this.durable, revision, lastMutation: { kind: "input", mutationId: crypto.randomUUID(), fingerprint: "d".repeat(64), resultingRevision: revision } };
            },
            snapshot() {
                const active = this.current;
                return { durable: structuredClone(this.durable), baseline: structuredClone(this.baseline), ownerGeneration: active?.session.controller.ownerGeneration,
                    state: active?.session.controller.state, registry: active?.inputs.inspect(), unconfirmed: active?.session.unconfirmed,
                    transports: active?.recovery.captureTransports(this.audience, this.draftId), calls: structuredClone(this.calls), genericRetains: this.genericRetains,
                    dead: this.dead.map(old => ({ ownerGeneration: old.ownerGeneration, producers: old.producers })) };
            },
            distinctOwners() {
                const old = this.dead.at(-1), fresh = this.current;
                return !!old && old.session !== fresh.session && old.controller !== fresh.session.controller && old.inputs !== fresh.inputs && old.recovery !== fresh.recovery && old.fields !== fresh.fields && old.ownerGeneration !== fresh.session.controller.ownerGeneration;
            },
        };
    }, schema);
    const fieldId = "people/life_events";
    const candidate = input => input.fields.find(field => field.fieldId === fieldId).valueState.value;
    const inputsOnly = observation => observation.calls.filter(call => call.route === "/api/capture/draft/input");
    const originalOf = observation => {
        const requests = inputsOnly(observation); assert.equal(requests.length, 1, "one original full-input mutation before recovery"); return requests[0].body;
    };
    async function action(kind, mode, lookup) {
        await page.evaluate(async ({ mode, lookup }) => { const f = window.fixture; await f.reset(); f.mode = mode; f.lookup = lookup; }, { mode, lookup });
        const labels = await page.evaluate(() => window.fixture.labels);
        const panel = page.locator("#capture .native-birthday");
        if (kind === "choose") await panel.locator('[data-birth-index="2"]').getByRole("button", { name: labels.choose, exact: true }).click();
        else await panel.getByRole("button", { name: labels.add, exact: true }).click();
        return page.evaluate(() => window.fixture.settleFailure());
    }
    async function rebuild(lookup) {
        return page.evaluate(async lookup => { const f = window.fixture; f.destroy(); f.lookup = lookup; f.mode = "normal"; await f.open(); return { ...f.snapshot(), distinctOwners: f.distinctOwners() }; }, lookup);
    }
    function assertPureRetained(row, original) {
        assert.equal(row.pending.command.kind, "fullInput"); assert.equal(row.pending.command.mutationId, original.mutationId);
        assert.equal(row.pending.command.expectedRevision, original.expectedRevision);
        assert.deepEqual(row.pending.input.rawInputs, []); assert.deepEqual(row.pending.input.detachedRawInputs, []);
        assert.ok(Object.values(row.pending.input.texts).every(texts => texts.length === 0));
        assert.doesNotMatch(JSON.stringify(row), /Protected original|Retain source A|unknownEvidence|formBasis/);
    }
    for (const kind of ["choose", "add"]) {
        await check(`${kind}: unapplied input failure retains the original action cut through fresh owners and blocks close`, async () => {
            const failed = await action(kind, "reject", "unknown"), original = originalOf(failed);
            assert.notDeepEqual(candidate(failed.state.input), candidate(failed.baseline.currentInput), "the real button changed its owned typed draft");
            assert.deepEqual(failed.durable, failed.baseline); assert.equal(failed.state.blocked, true);
            assert.ok(failed.registry.producers.some(producer => producer.unapplied && producer.hasObservedChanges));
            const before = await page.evaluate(() => window.fixture.close()); assert.equal(before.accepted, false);
            const reopened = await rebuild("unknown"); assert.equal(reopened.distinctOwners, true);
            assert.equal(reopened.unconfirmed.length, 1); assertPureRetained(reopened.unconfirmed[0], original);
            assert.deepEqual(candidate(reopened.state.input), candidate(failed.baseline.currentInput), "unproved candidate is not silently installed after recreation");
            const after = await page.evaluate(() => window.fixture.close()); assert.equal(after.accepted, false);
            const final = await page.evaluate(() => window.fixture.snapshot());
            assert.equal(inputsOnly(final).length, 1); assert.equal(final.calls.some(call => call.route.endsWith("/flush")), false);
            assert.equal(final.genericRetains, 0);
            return { original, before, after, fresh: final };
        });
        await check(`${kind}: applied response loss resolves the original full-input fingerprint before a fresh exact close proof`, async () => {
            const failed = await action(kind, "lost", "unavailable"), original = originalOf(failed);
            assert.deepEqual(candidate(failed.durable.currentInput), candidate(failed.state.input));
            assert.equal(failed.durable.lastMutation.mutationId, original.mutationId);
            assert.ok(failed.registry.producers.some(producer => producer.unapplied));
            const reopened = await rebuild("normal"); assert.equal(reopened.distinctOwners, true);
            assert.equal(reopened.unconfirmed.length, 0); assert.deepEqual(reopened.transports, []);
            assert.equal(inputsOnly(reopened).length, 1, "recovery queries rather than resubmitting the input");
            const query = reopened.calls.findLast(call => call.route === "/api/capture/draft/input-mutation");
            assert.deepEqual(query.body, { draftId: original.draftId, expectedRevision: original.expectedRevision, mutationId: original.mutationId, fingerprint: failed.durable.lastMutation.fingerprint });
            assert.deepEqual(candidate(reopened.state.input), candidate(failed.durable.currentInput));
            assert.equal(reopened.registry.producers.some(producer => producer.hasObservedChanges), false, "loading a recovered cut invents no new author action");
            const close = await page.evaluate(async () => { const f = window.fixture, result = await f.close(); if (result.accepted) { result.frozen = f.current.guard.freeze(result.proof); result.inert = f.current.inert(); f.current.guard.thaw(); } return result; });
            assert.equal(close.accepted, true); assert.equal(close.frozen, true); assert.equal(close.inert, true);
            const final = await page.evaluate(() => window.fixture.snapshot());
            assert.equal(close.proof.recordRevision, final.durable.revision); assert.equal(close.proof.lastMutationId, final.durable.lastMutation.mutationId);
            assert.equal(close.proof.ownerGeneration, final.ownerGeneration); assert.equal(close.proof.editRevision, final.state.editRevision);
            assert.equal(final.registry.producers.some(producer => producer.unapplied), false); assert.equal(final.genericRetains, 0);
            assert.notEqual(close.proof.ownerGeneration, failed.ownerGeneration);
            return { original, recoveryQuery: query, close, fresh: final };
        });
        await check(`${kind}: same-byte superseded/unknown input stays unresolved over repeated fresh owners without a replacement mutation`, async () => {
            const failed = await action(kind, "lost", "unavailable"), original = originalOf(failed);
            await page.evaluate(() => window.fixture.supersedeWithoutChangingBytes());
            let prior;
            for (let attempt = 0; attempt < 2; attempt++) {
                const reopened = await rebuild("unknown"); assert.equal(reopened.distinctOwners, true);
                assert.deepEqual(candidate(reopened.durable.currentInput), candidate(failed.durable.currentInput));
                assert.notEqual(reopened.durable.lastMutation.mutationId, original.mutationId);
                assert.equal(reopened.unconfirmed.length, 1); assertPureRetained(reopened.unconfirmed[0], original);
                if (prior) assert.deepEqual(reopened.unconfirmed, prior, "reconstruction cannot rewrite the unresolved original locator");
                prior = reopened.unconfirmed;
                assert.equal((await page.evaluate(() => window.fixture.close())).accepted, false);
            }
            const final = await page.evaluate(() => window.fixture.snapshot());
            assert.equal(inputsOnly(final).length, 1); assert.equal(final.calls.some(call => call.route.endsWith("/flush")), false);
            assert.equal(final.calls.some(call => /\/(create|submit|author-input|checkpoint)$/.test(call.route)), false);
            assert.equal(final.genericRetains, 0);
            return { original, fresh: final };
        });
    }
    await check("an applied label with a mismatched original fingerprint retains the choice proposal and cannot grant close", async () => {
        const failed = await action("choose", "lost", "unavailable"), original = originalOf(failed);
        const wrong = await rebuild("wrong-proof"); assert.equal(wrong.unconfirmed.length, 1); assertPureRetained(wrong.unconfirmed[0], original);
        assert.equal((await page.evaluate(() => window.fixture.close())).accepted, false);
        assert.equal(inputsOnly(await page.evaluate(() => window.fixture.snapshot())).length, 1);
        const corrected = await rebuild("normal"); assert.equal(corrected.unconfirmed.length, 0);
        assert.deepEqual(corrected.transports, []); assert.equal(inputsOnly(corrected).length, 1);
        assert.equal((await page.evaluate(() => window.fixture.close())).accepted, true);
        return { original, wrong, corrected };
    });
    assert.deepEqual(result.pageErrors, []);
    await page.evaluate(() => window.fixture.destroy());
    result.completed = true;
} catch (error) {
    result.failure = String(error.stack || error);
    if (page) await page.screenshot({ path: path.join(out, "failure.png"), fullPage: true }).catch(() => {});
    process.exitCode = 1;
} finally {
    try { await browser?.close(); result.ownership.browserClosed = true; } catch (error) { result.browserCloseFailure = String(error); }
    try { await vite?.close(); result.ownership.viteClosed = true; } catch (error) { result.viteCloseFailure = String(error); }
    result.sourceAfter = await inventory(); result.inputsUnchanged = JSON.stringify(result.sourceBefore) === JSON.stringify(result.sourceAfter);
    await writeFile(path.join(out, "result.json"), JSON.stringify(result, null, 2));
    console.log(JSON.stringify({ out, completed: result.completed, cases: result.cases.length, inputsUnchanged: result.inputsUnchanged, ownership: result.ownership, failure: result.failure }));
    if (!result.inputsUnchanged || !result.ownership.browserClosed || !result.ownership.viteClosed) process.exitCode = 1;
}
