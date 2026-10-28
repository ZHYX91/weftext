import test, { type TestContext } from "node:test";
import assert from "node:assert/strict";
import { NavigationControls } from "./query-panel";
import { CalendarPanel, PeriodNavigator } from "./period-navigation";
import type { DataSchema, PanelHost } from "./data-panel";
import { TestDocument, TestElement } from "./dom.test-support";

/** Event dispatch is the boundary under test; no layout, focus or native IME claims. */
class EventElement extends TestElement {
  inert = false;
  validity = { valid: true, badInput: false };
  returnValue = "";
  private readonly listeners = new Map<string, { run: (event: Event) => void; capture: boolean }[]>();
  override addEventListener(type?: string, run?: (event: Event) => void, options?: boolean | AddEventListenerOptions): void {
    if (!type || !run) return;
    const listeners = this.listeners.get(type) ?? [];
    listeners.push({ run, capture: options === true || typeof options === "object" && options.capture === true });
    this.listeners.set(type, listeners);
  }
  emit(type: string): void {
    let stopped = false;
    const event = { type, target: this, preventDefault() {}, stopImmediatePropagation() { stopped = true; } } as unknown as Event;
    const path: EventElement[] = [];
    for (let node: TestElement | undefined = this; node; node = node.parentNode)
      if (node instanceof EventElement) path.push(node);
    for (const node of [...path].reverse()) {
      for (const listener of node.listeners.get(type) ?? []) {
        if (listener.capture) listener.run(event);
        if (stopped) return;
      }
    }
    for (const node of path) {
      for (const listener of node.listeners.get(type) ?? []) {
        if (!listener.capture) listener.run(event);
        if (stopped) return;
      }
      if (type === "input") node.oninput?.();
      if (type === "change") node.onchange?.();
      if (type === "click") node.onclick?.();
    }
  }
  override close(): void { super.close(); this.emit("close"); }
}

class EventDocument extends TestDocument {
  override createElement(tag: string): EventElement { return new EventElement(tag, this); }
}

function environment(t: TestContext): EventDocument {
  const doc = new EventDocument();
  const storage = new Map<string, string>();
  for (const [key, value] of Object.entries({
    document: doc,
    localStorage: { getItem: (key: string) => storage.get(key) ?? null, setItem: (key: string, value: string) => storage.set(key, value), removeItem: (key: string) => storage.delete(key) },
  })) {
    const original = Object.getOwnPropertyDescriptor(globalThis, key);
    Object.defineProperty(globalThis, key, { configurable: true, writable: true, value });
    t.after(() => { if (original) Object.defineProperty(globalThis, key, original); else Reflect.deleteProperty(globalThis, key); });
  }
  return doc;
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

function host(request: (path: string, body: unknown) => Promise<unknown>, extra: Partial<PanelHost> = {}): PanelHost {
  return {
    request: request as PanelHost["request"],
    notify() {},
    getDocument: () => null,
    openDocument: async () => {},
    applyPatch: async () => {},
    ...extra,
  };
}

type PeriodOwner = {
  series: EventElement; key: EventElement; date: EventElement; kind: EventElement; name: EventElement; parent: EventElement; body: EventElement;
  createButton: EventElement; pending: Record<string, unknown> | null;
  result: unknown; loading: boolean; schemaAvailable: boolean;
  lookup(steps?: number, fromDate?: boolean): Promise<void>; create(): Promise<void>;
};

function period(doc: EventDocument, source: PanelHost) {
  const panel = new PeriodNavigator(source, "/library", () => "/library");
  doc.body.append(panel.element as unknown as EventElement);
  const owner = panel as unknown as PeriodOwner;
  return { panel, owner };
}

function input(control: EventElement, value: string): void { control.value = value; control.emit("input"); }
function resolved(key: string) {
  return {
    period: { kind: "record", value: [{ key: "key", value: { kind: "text", value: key } }] },
    bounds: { kind: "text", value: "range" }, policy: { multiplicity: "unique" },
    matches: [], coverage: "complete", canCreate: true,
  };
}

function periodSchema(kinds = ["day", "week", "month", "quarter", "year"]): DataSchema {
  return { fields: [{ id: "calendar/period", key: "period", nullable: false, shape: { kind: "record", members: {}, unknown: "preserve" } }], types: [], aliases: {}, definitions: {}, series_policies: [{ key: "journal", kinds, scope: { kind: "library" }, multiplicity: "unique" }] };
}

test("calendar date selection submits only a date, then steps from the Core-returned canonical key", async t => {
  const doc = environment(t), requests: Record<string, unknown>[] = [];
  const { panel, owner } = period(doc, host(async (route, request) => {
    assert.equal(route, "/api/data/period/resolve");
    requests.push(request as Record<string, unknown>);
    const result = resolved(requests.length === 1 ? "2020-W53" : "2021-W01");
    return { ...result, bounds: { kind: "record", value: [{ key: "start", value: { kind: "date", value: "2020-12-28" } }, { key: "end_exclusive", value: { kind: "date", value: "2021-01-04" } }] } };
  }));
  panel.setSchema(periodSchema());
  input(owner.series, "journal"); input(owner.kind, "week"); input(owner.date, "2021-01-01");
  await owner.lookup(0, true);
  assert.deepEqual(requests[0], { root: "/library", seriesKey: "journal", kind: "week", date: "2021-01-01" });
  assert.equal(owner.key.value, "2020-W53");
  assert.match(panel.element.textContent ?? "", /2020-12-28/);
  assert.match(panel.element.textContent ?? "", /2021-01-04/);
  await owner.lookup(1);
  assert.deepEqual(requests[1], { root: "/library", seriesKey: "journal", kind: "week", key: "2020-W53", steps: 1 });
  assert.equal(owner.key.value, "2021-W01");
  panel.assertCloseReady();
});

test("calendar retains an unsupported series kind without silently changing it or resolving", async t => {
  const doc = environment(t);
  const { panel, owner } = period(doc, host(async () => assert.fail("unsupported policy must not issue a lookup")));
  panel.setSchema(periodSchema(["day"]));
  input(owner.series, "journal"); input(owner.kind, "week"); input(owner.key, "2020-W53");
  await owner.lookup();
  assert.equal(owner.kind.value, "week");
  assert.equal(owner.key.value, "2020-W53");
  assert.equal(owner.createButton.disabled, true);
  assert.throws(() => panel.assertCloseReady());
  panel.setSchema({ ...periodSchema(), series_policies: [] });
  assert.equal(owner.series.value, "journal");
  assert.equal(owner.kind.value, "week");
});

test("calendar partial lookup never enables creation even when a malformed response suggests it", async t => {
  const doc = environment(t); let calls = 0;
  const { owner } = period(doc, host(async () => { calls++; return { ...resolved("2026-10-29"), coverage: "partial", canCreate: true }; }));
  input(owner.series, "journal"); input(owner.key, "2026-10-29");
  await owner.lookup();
  assert.equal(owner.createButton.disabled, true);
  input(owner.name, "would-be-created");
  await owner.create();
  assert.equal(calls, 1);
  assert.equal(owner.pending, null);
});

test("a late date lookup cannot adopt a new selection even when the visible key is unchanged", async t => {
  const doc = environment(t), answer = deferred<unknown>();
  const { panel, owner } = period(doc, host(async () => answer.promise));
  input(owner.series, "journal"); input(owner.date, "2021-01-01");
  const lookup = owner.lookup(0, true);
  input(owner.date, "2021-01-02");
  answer.resolve(resolved("2020-W53"));
  await lookup;
  assert.equal(owner.date.value, "2021-01-02");
  assert.equal(owner.key.value, "");
  assert.equal(owner.result, null);
  assert.throws(() => panel.assertCloseReady());
});

test("calendar docking and library switches retain exactly one periodic owner per library", async t => {
  const doc = environment(t), schema = deferred<unknown>(); let reads = 0;
  const panel = new CalendarPanel(host(async route => { assert.equal(route, "/api/data/schema"); reads++; return schema.promise; }));
  doc.body.append(panel.element as unknown as EventElement);
  panel.setLibrary({ root: "/library", workspaceId: "one" }, "/library/folder");
  const sameRead = panel.refresh();
  assert.equal(panel.freezeForClose(), false);
  schema.resolve({ schema: periodSchema() });
  await sameRead;
  assert.equal(reads, 1);
  const internals = panel as unknown as { current: { navigator: PeriodNavigator }; libraries: Map<string, unknown> };
  const original = internals.current.navigator;
  input((original as unknown as PeriodOwner).body, "retained #native(draft");
  assert.equal((original as unknown as PeriodOwner).parent.value, "/library/folder");
  panel.setLibrary({ root: "/other", workspaceId: "two" }, null);
  await panel.refresh();
  panel.element.remove();
  assert.equal(panel.freezeForClose(), false);
  panel.setLibrary({ root: "/library", workspaceId: "one" }, null);
  await panel.refresh();
  assert.equal(internals.current.navigator, original);
  assert.equal((original as unknown as PeriodOwner).body.value, "retained #native(draft");
  assert.equal(internals.libraries.size, 2);
});

test("an incomplete native date buffer cannot disappear through close or library departure", async t => {
  const doc = environment(t);
  const panel = new CalendarPanel(host(async () => ({ schema: periodSchema() })));
  doc.body.append(panel.element as unknown as EventElement);
  panel.setLibrary({ root: "/library", workspaceId: "one" }, null);
  await panel.refresh();
  const state = panel as unknown as { current: { navigator: PeriodNavigator; root: string } };
  const date = (state.current.navigator as unknown as PeriodOwner).date;
  assert.equal(date.value, "");
  date.validity = { valid: false, badInput: true };
  date.emit("input");
  assert.throws(() => panel.assertCloseReady());
  assert.equal(panel.freezeForClose(), false);
  assert.throws(() => panel.assertLibraryDeparture());
  assert.throws(() => panel.setLibrary({ root: "/other", workspaceId: "two" }, null));
  assert.equal(state.current.root, "/library");
  assert.equal((state.current.navigator as unknown as PeriodOwner).date, date);
  date.validity = { valid: true, badInput: false };
  date.emit("input");
  panel.assertCloseReady();
  panel.setLibrary({ root: "/other", workspaceId: "two" }, null);
  await panel.refresh();
  assert.equal(state.current.root, "/other");
});

test("provider-only schema changes retire displayed bounds and in-flight period results", async t => {
  const doc = environment(t), pending = deferred<unknown>(); let calls = 0;
  const { panel, owner } = period(doc, host(async () => ++calls === 1 ? resolved("2020-W53") : pending.promise));
  const schema = (version: string) => ({ ...periodSchema(), providers: [{ id: "calendar", implementation: { id: "table", abi: 1 }, data: { key: "test", version, years: [1, 2] } }] });
  panel.setSchema(schema("one"));
  input(owner.series, "journal"); input(owner.key, "2020-W53");
  await owner.lookup();
  assert.notEqual(owner.result, null);
  panel.setSchema(schema("two"));
  assert.equal(owner.result, null);
  assert.equal(owner.createButton.disabled, true);
  assert.doesNotMatch(panel.element.textContent ?? "", /range/);
  assert.equal(owner.key.value, "2020-W53");
  const reading = owner.lookup();
  panel.setSchema(schema("three"));
  pending.resolve(resolved("2021-W01"));
  await reading;
  assert.equal(owner.result, null);
  assert.equal(owner.key.value, "2020-W53");
});

test("period raw creation and lookup input belongs to the owner while detached", t => {
  const doc = environment(t);
  const { panel, owner } = period(doc, host(async () => assert.fail("unexpected request")));
  input(owner.body, "#unterminated(\u0000");
  panel.element.remove();
  assert.throws(() => panel.assertCloseReady());
  assert.equal(panel.freezeForClose(), false);
  input(owner.body, "");
  panel.assertCloseReady();
  input(owner.key, "not-a-valid-period");
  assert.throws(() => panel.assertCloseReady());
  input(owner.key, "");
  assert.equal(panel.freezeForClose(), true);
  assert.equal((panel.element as unknown as EventElement).inert, true);
  panel.thawAfterClose();
  assert.equal((panel.element as unknown as EventElement).inert, false);
});

test("late period lookup cannot erase newer raw input or release close while transport is active", async t => {
  const doc = environment(t), answer = deferred<unknown>();
  const { panel, owner } = period(doc, host(async () => answer.promise));
  input(owner.series, "journal"); input(owner.key, "2026-10-24");
  const lookup = owner.lookup();
  input(owner.key, "unfinished-period");
  assert.equal(panel.freezeForClose(), false);
  assert.equal(owner.loading, true);
  answer.resolve(resolved("2026-10-24"));
  await lookup;
  assert.equal(owner.key.value, "unfinished-period");
  assert.equal(owner.result, null);
  assert.equal(owner.loading, false);
  assert.throws(() => panel.assertCloseReady());
});

test("period close waits through committed creation delivery and its follow-up lookup", async t => {
  const doc = environment(t), delivery = deferred<void>(), deliveryStarted = deferred<void>(), refresh = deferred<unknown>(), refreshStarted = deferred<void>();
  let lookups = 0;
  const { panel, owner } = period(doc, host(async (path, body) => {
    if (path.endsWith("/resolve")) {
      if (++lookups === 1) return resolved("2026-10-24");
      refreshStarted.resolve();
      return refresh.promise;
    }
    assert.equal(path, "/api/data/period/create");
    return { operationId: (body as { operationId: string }).operationId, status: "committed", document: { path: "/library/day.typ" } };
  }, { created: async () => { deliveryStarted.resolve(); await delivery.promise; } }));
  input(owner.series, "journal"); input(owner.key, "2026-10-24");
  await owner.lookup();
  panel.assertCloseReady();
  input(owner.name, "day.typ"); input(owner.body, "authored text");
  const creating = owner.create();
  await deliveryStarted.promise;
  assert.notEqual(owner.pending, null);
  assert.equal(panel.freezeForClose(), false);
  assert.equal(owner.body.value, "authored text");
  delivery.resolve();
  await refreshStarted.promise;
  assert.equal(owner.pending, null);
  assert.equal(owner.body.value, "");
  assert.equal(panel.freezeForClose(), false);
  refresh.resolve(resolved("2026-10-24"));
  await creating;
  assert.equal(panel.freezeForClose(), true);
  panel.thawAfterClose();
});

test("failed committed period delivery retains the original operation and authored input", async t => {
  const doc = environment(t);
  const { panel, owner } = period(doc, host(async (path, body) => path.endsWith("/resolve") ? resolved("2026-10-24") : {
    operationId: (body as { operationId: string }).operationId, status: "committed", document: { path: "/library/day.typ" },
  }, { created: async () => { throw new Error("delivery failed"); } }));
  input(owner.series, "journal"); input(owner.key, "2026-10-24");
  await owner.lookup(); input(owner.name, "day.typ"); input(owner.body, "keep me");
  await owner.create();
  assert.notEqual(owner.pending, null);
  assert.equal(owner.body.value, "keep me");
  assert.throws(() => panel.assertCloseReady());
});

test("new period raw input during creation delivery survives the old successful receipt", async t => {
  const doc = environment(t), delivery = deferred<void>(), started = deferred<void>();
  const { panel, owner } = period(doc, host(async (path, body) => path.endsWith("/resolve") ? resolved("2026-10-24") : {
    operationId: (body as { operationId: string }).operationId, status: "committed", document: { path: "/library/day.typ" },
  }, { created: async () => { started.resolve(); await delivery.promise; } }));
  input(owner.series, "journal"); input(owner.key, "2026-10-24");
  await owner.lookup(); input(owner.name, "day.typ"); input(owner.body, "first input");
  const creating = owner.create();
  await started.promise;
  input(owner.body, "new invalid #raw(");
  delivery.resolve(); await creating;
  assert.equal(owner.pending, null);
  assert.equal(owner.body.value, "new invalid #raw(");
  assert.throws(() => panel.assertCloseReady());
});

test("period retry confirms its original operation even after current creation fields are cleared", async t => {
  const doc = environment(t);
  let operationId = "", creations = 0, statusReads = 0, deliveries = 0;
  const { panel, owner } = period(doc, host(async (path, body) => {
    if (path.endsWith("/resolve")) return resolved("2026-10-24");
    if (path === "/api/data/period/create") {
      creations++;
      operationId = (body as { operationId: string }).operationId;
    } else {
      assert.equal(path, "/api/operations/status");
      assert.equal((body as { operationId: string }).operationId, operationId);
      statusReads++;
    }
    return { operationId, status: "committed", document: { path: "/library/day.typ" } };
  }, { created: async () => { if (++deliveries === 1) throw new Error("first delivery failed"); } }));
  input(owner.series, "journal"); input(owner.key, "2026-10-24");
  await owner.lookup(); input(owner.name, "day.typ"); input(owner.body, "original input");
  await owner.create();
  assert.notEqual(owner.pending, null);
  input(owner.name, ""); input(owner.parent, ""); input(owner.body, "");
  await owner.create();
  assert.equal(creations, 1);
  assert.equal(statusReads, 1);
  assert.equal(deliveries, 2);
  assert.equal(owner.pending, null);
  assert.equal(owner.parent.value, "");
  // The cleared parent is new raw input; confirming the old operation must retain it.
  assert.throws(() => panel.assertCloseReady());
});

test("frozen period owner rejects new operations and external schema delivery until thaw", async t => {
  const doc = environment(t);
  let requests = 0;
  const { panel, owner } = period(doc, host(async () => { requests++; return resolved("2026-10-24"); }));
  assert.equal(panel.freezeForClose(), true);
  panel.setSchemaAvailable(false);
  await owner.lookup(); await owner.create();
  assert.equal(requests, 0);
  assert.equal(owner.schemaAvailable, true);
  panel.thawAfterClose();
  panel.setSchemaAvailable(false);
  assert.equal(owner.schemaAvailable, false);
});

type NavigationOwner = {
  createName: EventElement; parentHost: EventElement; parentRef: string | null;
  pending: { route: string; body: Record<string, unknown>; prepared?: { targetRef: string; preview: { operationId: string; planId: string } }; confirmed?: boolean; context?: unknown } | null;
  registry: unknown; loadTags(): Promise<void>; resume(): Promise<void>; move(reference: string): Promise<void>;
};

function navigation(doc: EventDocument, source: PanelHost) {
  const panel = new NavigationControls(source, "/library", "workspace", () => "/library", () => {}, () => {}, () => "tags", () => {});
  doc.body.append(panel.element as unknown as EventElement);
  return { panel, owner: panel as unknown as NavigationOwner };
}

test("navigation retains raw create-name input and its pending owner after detachment", t => {
  const doc = environment(t);
  const { panel, owner } = navigation(doc, host(async () => assert.fail("unexpected request")));
  input(owner.createName, "invalid\u0000name");
  panel.element.remove();
  assert.throws(() => panel.assertCloseReady());
  input(owner.createName, "");
  panel.assertCloseReady();
  owner.pending = { route: "/api/navigation/create-tag", body: { operationId: "original" } };
  assert.equal(panel.freezeForClose(), false);
  assert.throws(() => panel.assertCloseReady());
});

test("navigation registry read stays busy despite no mutation proposal", async t => {
  const doc = environment(t), answer = deferred<unknown>();
  const { panel, owner } = navigation(doc, host(async () => answer.promise));
  const loading = owner.loadTags();
  assert.equal(owner.pending, null);
  assert.equal(panel.freezeForClose(), false);
  answer.resolve({ tags: [], baseId: "registry", complete: true });
  await loading;
  panel.assertCloseReady();
  assert.equal(panel.freezeForClose(), true);
  const registry = owner.registry;
  await owner.loadTags();
  assert.equal(owner.registry, registry);
  panel.thawAfterClose();
});

test("navigation parent choice is authored owner state, and an open move dialog holds close", async t => {
  const doc = environment(t);
  const reference = "weftext-tag://workspace/tag";
  const { panel, owner } = navigation(doc, host(async () => ({
    tags: [{ tagRef: reference, tagId: "tag", displayName: "Tag", parentTagId: null }], baseId: "registry", complete: true,
  })));
  await owner.loadTags();
  panel.assertCloseReady();
  const select = owner.parentHost.find("select") as EventElement;
  select.value = reference; select.emit("change");
  assert.equal(owner.parentRef, reference);
  assert.equal(panel.freezeForClose(), false);
  select.value = ""; select.emit("change");
  panel.assertCloseReady();
  const moving = owner.move(reference);
  assert.equal(panel.freezeForClose(), false);
  const dialog = doc.body.find("dialog") as EventElement;
  dialog.returnValue = "cancel";
  dialog.close();
  await moving;
  panel.assertCloseReady();
});

test("navigation close spans asynchronous committed assignment delivery", async t => {
  const doc = environment(t), delivery = deferred<void>(), started = deferred<void>();
  const { panel, owner } = navigation(doc, host(async path => {
    if (path === "/api/operations/status") return { operationId: "original", planId: "plan", status: "committed" };
    assert.equal(path, "/api/navigation/tags");
    return { tags: [], baseId: "registry", complete: true };
  }, { applyOperationReceipt: async () => { started.resolve(); await delivery.promise; } }));
  owner.pending = {
    route: "/api/navigation/create-tag", body: { root: "/library", operationId: "original", displayName: "tag", parentRef: null },
    prepared: { targetRef: "weftext-tag://workspace/tag", preview: { operationId: "original", planId: "plan" } },
    confirmed: true, context: { root: "/library", path: "/library/note.typ", sessionId: "session" },
  };
  input(owner.createName, "tag");
  const resuming = owner.resume();
  await started.promise;
  assert.equal(panel.freezeForClose(), false);
  assert.notEqual(owner.pending, null);
  assert.equal(owner.createName.value, "tag");
  delivery.resolve(); await resuming;
  assert.equal(owner.pending, null);
  assert.equal(owner.createName.value, "");
  panel.assertCloseReady();
});

test("recovered navigation assignment cannot clear its pending owner without a delivery consumer", async t => {
  const doc = environment(t);
  const { panel, owner } = navigation(doc, host(async () => ({ operationId: "original", planId: "plan", status: "committed" })));
  owner.pending = {
    route: "/api/navigation/create-tag", body: { root: "/library", operationId: "original" },
    prepared: { targetRef: "weftext-tag://workspace/tag", preview: { operationId: "original", planId: "plan" } },
    confirmed: true, context: { root: "/library", path: "/library/note.typ", sessionId: "session" },
  };
  await owner.resume();
  assert.notEqual(owner.pending, null);
  assert.equal(panel.freezeForClose(), false);
});
