import { message } from "./ui-locale";
import test from "node:test";
import assert from "node:assert/strict";
import { QueryRefreshQueue, queryRefreshKey } from "./query-refresh";
import { QueryPanel, defaultPreference, queryPlanRefreshInput, type QueryPlanInputs, type QueryResult } from "./query-panel";

const deferred = () => { let resolve!: () => void; const promise = new Promise<void>(done => { resolve = done; }); return { promise, resolve }; };
import { localQueryResult } from "./query-result.test-support";
const result=(cutId:string):QueryResult=>localQueryResult({cutId});

/** Exercise the real panel's controller paths without mounting its rendering shell. */
function panelHarness(request: (path: string, body: unknown) => Promise<unknown>) {
  const state = { root: "D:/notes", workspaceId: "workspace", environmentEpoch: 0, generation: 0, directory: null, scopeIdentity: null,
    descendants: true, lifecycle: "live", types: [], search: "", includeBody: false, conditions: [], expression: "", exactPlan: null,
    group: undefined, take: 200, preference: defaultPreference(), schema: null, result: null as QueryResult | null, savedSelection: null, savedPlan: null,
    definitionRevision: null, selected: null, savedPending: null as unknown, parameterDraft: "{}", loading: false, error: null, previewCache: new Map(),
    host: { request, notify() {} }, renderControls() {}, renderResults() {}, renderNavigation() {}, loadPreferences() {},
    refreshQueue: undefined as QueryRefreshQueue<unknown> | undefined };
  Object.setPrototypeOf(state, QueryPanel.prototype);
  const panel = state as unknown as QueryPanel, internals = state as unknown as { executeRefresh(input: unknown, current: () => boolean): Promise<void>; savePreferences(bindFields: boolean): void; saveDefinition(label: string): Promise<void>; runSaved(selection: { owner: string; key: string; label: string; mode: "current" | "pinned"; revision?: string }): Promise<void> };
  state.refreshQueue = new QueryRefreshQueue((input, current) => internals.executeRefresh(input, current));
  return { state, panel, internals };
}

test("identical in-flight Query intents share preparation and HTTP, but a later explicit refresh runs again", async () => {
  const held = deferred(), started: string[] = [];
  const queue = new QueryRefreshQueue<string>(async input => { started.push(input); await held.promise; });
  const first = queue.request("same", "original"), second = queue.request("same", "duplicate");
  assert.equal(first, second); await Promise.resolve(); assert.deepEqual(started, ["original"]);
  held.resolve(); await first;
  await queue.request("same", "explicit-after-completion");
  assert.deepEqual(started, ["original", "explicit-after-completion"]);
});

test("changing input retains only the latest frozen pending intent and rejects the active result", async () => {
  const held = deferred(), started: string[] = [], published: string[] = [];
  const queue = new QueryRefreshQueue<{ label: string }>(async (input, current) => {
    started.push(input.label); if (input.label === "A") await held.promise;
    if (current()) published.push(input.label);
  });
  const first = queue.request("A", { label: "A" }); await Promise.resolve();
  queue.request("B", { label: "B" }); const intent = { label: "C" }, last = queue.request("C", intent); intent.label = "later unrelated edit";
  assert.equal(first, last); held.resolve(); await last;
  assert.deepEqual(started, ["A", "C"]); assert.deepEqual(published, ["C"]);
});

test("environment invalidation needs a new cut even for the same plan, and leaving a root drops its waiting intent", async () => {
  const held = deferred(), started: string[] = [], published: string[] = [];
  const queue = new QueryRefreshQueue<string>(async (input, current) => { started.push(input); if (input === "root1:epoch0") await held.promise; if (current()) published.push(input); });
  const first = queue.request("root1:epoch0", "root1:epoch0"); await Promise.resolve();
  queue.invalidate(); queue.request("root1:epoch1", "root1:epoch1");
  queue.invalidate(); queue.request("root2:epoch0", "root2:epoch0"); held.resolve(); await first;
  assert.deepEqual(started, ["root1:epoch0", "root2:epoch0"]); assert.deepEqual(published, ["root2:epoch0"]);
  queue.invalidate(); await queue.request("root2:epoch1", "root2:epoch1"); assert.equal(started.at(-1), "root2:epoch1");
});

test("a superseded failed read cannot reject a newer intent or turn a completed promise into a result cache", async () => {
  const held = deferred(), started: string[] = [];
  const queue = new QueryRefreshQueue<string>(async input => { started.push(input); if (input === "failed") { await held.promise; throw new Error("original failure"); } });
  const first = queue.request("first", "failed"); await Promise.resolve();
  queue.request("second", "latest"); held.resolve(); await first;
  assert.deepEqual(started, ["failed", "latest"]); await queue.request("second", "explicit"); assert.equal(started.at(-1), "explicit");
});

test("a still-current failure is reported and a subsequent explicit retry runs", async () => {
  let failed = true, calls = 0;
  const queue = new QueryRefreshQueue<string>(async () => { calls++; if (failed) throw new Error("current failure"); });
  await assert.rejects(queue.request("same", "same"), /current failure/); failed = false;
  await queue.request("same", "same"); assert.equal(calls, 2);
});

test("Query identity preserves typed values, plan ordering, saved revisions and environment epochs", () => {
  const source = { root: "one", epoch: 2, selection: { mode: "pinned", revision: "exact", key: "key" }, parameters: { amount: { kind: "decimal", value: "9007199254740993.01" } }, plan: { select: ["title", "body"] } };
  assert.equal(queryRefreshKey(source), queryRefreshKey({ ...source, parameters: { amount: { value: "9007199254740993.01", kind: "decimal" } } }));
  for (const changed of [{ ...source, epoch: 3 }, { ...source, root: "other" }, { ...source, selection: { ...source.selection, revision: "new" } }, { ...source, parameters: { amount: { kind: "text", value: "9007199254740993.01" } } }, { ...source, plan: { select: ["body", "title"] } }]) assert.notEqual(queryRefreshKey(source), queryRefreshKey(changed));
});

test("density, thumbnails, pinning and inactive view settings cannot retire a valid in-flight Query", async () => {
  const input: QueryPlanInputs = { scope: null, descendants: true, lifecycle: "live", types: [], search: "", includeBody: false,
    conditions: [], expression: "", exactPlan: null, group: undefined, take: 200, preference: defaultPreference() };
  const key = () => queryRefreshKey(queryPlanRefreshInput(input)), original = key(), held = deferred(); let published = false, requests = 0;
  const queue = new QueryRefreshQueue<QueryPlanInputs>(async (_input, current) => { requests++; await held.promise; published = current() && key() === original; });
  const first = queue.request(original, input); await Promise.resolve();
  input.preference.density = "compact"; input.preference.thumbnails = true; input.preference.pinned = ["another-row"];
  input.preference.view = "table"; input.preference.dateField = "unused-date"; input.preference.groupField = "unused-group";
  assert.equal(key(), original); held.resolve(); await first; assert.equal(published, true); assert.equal(requests, 1);
  for (const edit of [(value: QueryPlanInputs) => { value.preference.summaries = false; }, (value: QueryPlanInputs) => { value.preference.columns.push("new-field"); }, (value: QueryPlanInputs) => { value.preference.sort = "due"; }, (value: QueryPlanInputs) => { value.preference.view = "calendar"; }]) {
    const changed = structuredClone(input); edit(changed); assert.notEqual(queryRefreshKey(queryPlanRefreshInput(changed)), original);
  }
});

test("the actual panel shares in-flight preparation, preserves presentation edits, and refreshes after completion or a confirmed write", async t => {
  const previousStorage = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
  Object.defineProperty(globalThis, "localStorage", { configurable: true, value: { setItem() {} } });
  t.after(() => { if (previousStorage) Object.defineProperty(globalThis, "localStorage", previousStorage); else Reflect.deleteProperty(globalThis, "localStorage"); });
  const held = deferred(), sent = deferred(), calls: { path: string; body: unknown }[] = []; let runs = 0;
  const { panel, state, internals } = panelHarness(async (path, body) => {
    calls.push({ path, body }); if (path === "/api/data/schema") return { definitions: {}, aliases: {}, types: [], fields: [{ id: "stable-status", key: "status", shape: { kind: "scalar", semantic: "text" }, nullable: true }] };
    assert.equal(path, "/api/query/run"); const cut = `cut-${++runs}`; if (runs === 1) { sent.resolve(); await held.promise; } return result(cut);
  });
  const first = panel.refresh(); await sent.promise; const duplicate = panel.refresh();
  state.preference.density = "compact"; state.preference.thumbnails = true; state.preference.pinned.push("selected"); internals.savePreferences(false);
  assert.equal(state.preference.fieldBindings, undefined, "display persistence must not silently add a new query binding");
  held.resolve(); await Promise.all([first, duplicate]);
  assert.equal(runs, 1); assert.equal(calls.filter(call => call.path === "/api/data/schema").length, 1); assert.equal(state.result?.cutId, "cut-1");
  await panel.refresh(); assert.equal(runs, 2); assert.equal(state.result?.cutId, "cut-2");
  panel.environmentChanged("D:\\notes"); assert.equal(state.result?.superseded, true);
  await panel.refresh(); assert.equal(runs, 3); assert.equal(state.result?.cutId, "cut-3"); assert.equal(state.result?.superseded, undefined);
});

test("the actual panel rebases only its latest frozen pending parameters after an author write", async () => {
  const held = deferred(), sent = deferred(), parameters: unknown[] = [], installed: string[] = []; let runs = 0;
  const { panel, state } = panelHarness(async (path, body) => {
    if (path === "/api/data/schema") return { definitions: {}, aliases: {}, types: [], fields: [] };
    const request = body as { parameters: unknown }; parameters.push(request.parameters); const cut = `cut-${++runs}`;
    if (runs === 1) { sent.resolve(); await held.promise; } return result(cut);
  });
  state.renderResults = () => { if (state.result && !state.result.superseded) installed.push(state.result.cutId!); };
  const first = panel.refresh(); await sent.promise;
  state.parameterDraft = '{"value":{"kind":"text","value":"second"}}'; panel.refresh();
  state.parameterDraft = '{"value":{"kind":"decimal","value":"9007199254740993.01"}}'; const latest = panel.refresh();
  panel.environmentChanged("D:/notes"); held.resolve(); await Promise.all([first, latest]);
  assert.equal(runs, 2); assert.deepEqual(parameters, [{}, { value: { kind: "decimal", value: "9007199254740993.01" } }]);
  assert.equal(state.result?.cutId, "cut-2"); assert.ok(installed.every(cut => cut === "cut-2")); assert.equal(state.loading, false);
});

test("saved current and pinned reads use the same panel scheduler, while a different pinned revision remains a new intent", async () => {
  const held = deferred(), sent = deferred(), selections: unknown[] = []; let runs = 0;
  const { state, internals } = panelHarness(async (path, body) => {
    if (path === "/api/data/schema") return { definitions: {}, aliases: {}, types: [], fields: [] };
    assert.equal(path, "/api/query/definition/run"); selections.push((body as { selection: unknown }).selection);
    const cut = `saved-${++runs}`; if (runs === 1) { sent.resolve(); await held.promise; } return { result: result(cut), definitionRevision: `revision-${runs}` };
  });
  const selection = { owner: "weftext-node://workspace/owner", key: "saved", label: "Original label", mode: "current" as const };
  const first = internals.runSaved(selection); await sent.promise;
  const same = internals.runSaved({ ...selection, label: "Display-only rename" }); held.resolve(); await Promise.all([first, same]);
  assert.equal(runs, 1); assert.equal(state.result?.cutId, "saved-1");
  await internals.runSaved({ ...selection, mode: "pinned", revision: "exact-a" });
  await internals.runSaved({ ...selection, mode: "pinned", revision: "exact-b" });
  assert.equal(runs, 3); assert.deepEqual(selections, [{ mode: "current", key: "saved" }, { mode: "pinned", key: "saved", revision: "exact-a" }, { mode: "pinned", key: "saved", revision: "exact-b" }]);
});

test("a saved definition keeps its frozen original through ordinary errors and delivers its committed fact", async () => {
  const calls: { path: string; body: unknown }[] = [];
  const { state, internals } = panelHarness(async (path, body) => {
    calls.push({ path, body });
    if (path === "/api/query/definition/save") throw Object.assign(new Error("owner unavailable"), { code: "permissionDenied" });
    const id = (body as { operationId: string }).operationId;
    assert.equal(path, "/api/operations/status");
    return { operationId: id, status: "committed", documentError: { code: "committedResultUnavailable", explanation: message("queryDefinition.saveReceiptUnavailable"), diagnostics: [], causes: [] } };
  });
  let delivered = 0;
  Object.assign(state.host, { getDocument: () => ({ root: "D:/notes", nodeId: "owner", sessionId: "original-session", source: "author draft", serial: 7, baseId: "original-base", path: "D:/notes/A.typ" }), savedDefinition: async () => { delivered++; throw new Error("document delivery unavailable"); } });
  state.result = result("old-cut"); await internals.saveDefinition("Saved");
  assert.equal(delivered, 1); assert.ok(state.savedPending);
  const original = structuredClone((state.savedPending as { body: unknown }).body);
  await internals.saveDefinition("Changed display input");
  assert.equal(delivered, 2); assert.deepEqual((state.savedPending as { body: unknown }).body, original);
  assert.deepEqual(calls.map(call => call.path), ["/api/query/definition/save", "/api/operations/status", "/api/operations/status"]);
});
