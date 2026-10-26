import assert from "node:assert/strict";
import test, { type TestContext } from "node:test";
import type { DocumentBasis, PanelHost } from "./data-panel";
import { TestDocument, TestElement } from "./dom.test-support";
import { ReminderOverview } from "./reminder-overview";
import { ReminderDeviceControl } from "./reminder-device-control";
import type { ReminderDelivery, ReminderHostStatus, ReminderRecordsPage } from "./reminder-types";
import { locale, message, uiTextValue, type MessageKey, type MessageParams } from "./ui-locale";

class Element extends TestElement {
  contains(node: TestElement): boolean { return this.all().includes(node); }
  matches(selector: string): boolean { return selector === ":disabled" && this.disabled; }
}
class Document extends TestDocument { override createElement(tag: string): Element { return new Element(tag, this); } }
const running: ReminderHostStatus = { desktopActive: true, notificationState: "enabled", scheduling: "running", scanComplete: true, scopeIssueCount: 0 };
type ReminderKey = Extract<MessageKey, `reminder.${string}`>;
type LabelKey = { [K in ReminderKey]: keyof MessageParams<K> extends never ? K : never }[ReminderKey];
const text = (key: LabelKey): string => uiTextValue(message(key));

function fixture(t: TestContext, status: () => Promise<unknown>, settingsFailure = false, records: () => ReminderRecordsPage = () => ({ records: [], next: null })) {
  const previous = Object.getOwnPropertyDescriptor(globalThis, "document"), document = new Document(), oldLocale = locale.current;
  Object.defineProperty(globalThis, "document", { configurable: true, writable: true, value: document });
  locale.set("zh-CN");
  const calls: { route: string; body: unknown }[] = [];
  const host = { getDocument: () => null, openDocument: async () => {}, applyPatch: async () => {}, notify() {}, async request(route: string, body?: unknown) {
    calls.push({ route, body });
    if (route === "/api/desktop/status") { assert.equal(body, undefined, "Status uses the real GET route"); return status(); }
    if (route === "/api/reminders/settings") {
      if (settingsFailure) throw new Error("TEST library settings unavailable");
      return { deviceId: "device", settingsRevision: 1, libraries: [], subscriptions: [] };
    }
    if (route === "/api/reminders/schedule") return { entries: [], issues: [], complete: true, next: null };
    if (route === "/api/reminders/records") return records();
    throw new Error(`Unexpected route ${route}`);
  } } as unknown as PanelHost;
  const overview = new ReminderOverview(host), element = overview.element as unknown as Element;
  document.body.append(element);
  t.after(() => {
    overview.dispose(); locale.set(oldLocale);
    if (previous) Object.defineProperty(globalThis, "document", previous); else Reflect.deleteProperty(globalThis, "document");
  });
  const displayed = (): string => element.all().find(node => node.className === "reminder-host-status")!.textContent;
  return { overview, element, displayed, calls };
}

test("Desktop reminder status explains portable, system-disabled and restricted states alongside an empty saved queue", async t => {
  let status: ReminderHostStatus = { ...running, notificationState: "unavailable", scheduling: "restricted", scanComplete: false };
  const f = fixture(t, async () => status);
  await f.overview.refresh();
  assert.ok(f.displayed().includes(text("reminder.notificationUnavailable")));
  assert.ok(f.displayed().includes(text("reminder.schedulingRestricted")));
  assert.ok(f.displayed().includes(text("reminder.scanIncomplete")));
  assert.ok(f.element.textContent.includes(text("reminder.noScheduledTasks")), "Saved task rows remain a separate observation");
  for (const [notificationState, key] of [
    ["disabledForApplication", "reminder.notificationDisabledForApplication"], ["disabledForUser", "reminder.notificationDisabledForUser"],
    ["disabledByPolicy", "reminder.notificationDisabledByPolicy"], ["disabledByManifest", "reminder.notificationDisabledByManifest"],
    ["unknown", "reminder.notificationUnknown"],
  ] as const) {
    status = { ...running, notificationState, scheduling: "restricted", scopeIssueCount: 2 };
    await f.overview.refresh();
    assert.ok(f.displayed().includes(text(key)));
    assert.ok(f.displayed().includes(uiTextValue(message("reminder.scopeIssues", { count: 2 }))));
  }
  assert.equal(f.calls.filter(call => call.route === "/api/desktop/status").length, 6);
});

test("Every actual scheduling state remains visible, and enabled notifications do not claim Do Not Disturb or delivery success", async t => {
  let status: ReminderHostStatus = running;
  const f = fixture(t, async () => status, true);
  for (const [scheduling, key] of [
    ["inactive", "reminder.schedulingInactive"], ["running", "reminder.schedulingRunning"],
    ["paused", "reminder.schedulingPaused"], ["restricted", "reminder.schedulingRestricted"], ["stopping", "reminder.schedulingStopping"],
  ] as const) {
    status = { ...running, scheduling, desktopActive: scheduling !== "inactive" };
    await f.overview.refresh();
    assert.ok(f.displayed().includes(text(key)));
    assert.ok(f.displayed().includes(text("reminder.notificationEnabled")));
    assert.equal(f.displayed().includes(text("reminder.submitted")), false);
    if (scheduling === "inactive") assert.ok(f.displayed().includes(text("reminder.desktopInactive")));
  }
  const chinese = f.displayed();
  locale.set("en-US");
  assert.notEqual(f.displayed(), chinese);
  assert.ok(f.displayed().includes(text("reminder.schedulingStopping")));
  assert.ok(f.element.textContent.includes("TEST library settings unavailable"), "Settings failure does not suppress host observation");
});

test("Failed or malformed host observation is explicitly unconfirmed and cannot retain a prior healthy status", async t => {
  let observation: unknown = running;
  const f = fixture(t, async () => { if (observation instanceof Error) throw observation; return observation; });
  await f.overview.refresh();
  assert.ok(f.displayed().includes(text("reminder.schedulingRunning")));
  for (const value of [new Error("TEST desktop transport failed"), null, {}, { ...running, notificationState: "futureState" }, { ...running, scopeIssueCount: -1 }]) {
    observation = value; await f.overview.refresh();
    assert.ok(f.displayed().includes(text("reminder.hostStatusUnavailable")));
    assert.equal(f.displayed().includes(text("reminder.schedulingRunning")), false);
    if (value instanceof Error) assert.ok(f.displayed().includes(value.message));
    assert.ok(f.element.textContent.includes(text("reminder.noDeliveryRecords")));
  }
});

test("A slow old host observation does not block task rows or overwrite the next refresh", async t => {
  let settle!: (value: unknown) => void, calls = 0;
  const f = fixture(t, () => ++calls === 1 ? new Promise(resolve => { settle = resolve; }) : Promise.resolve({ ...running, scheduling: "paused" }));
  const first = f.overview.refresh();
  for (let index = 0; index < 20; index++) await Promise.resolve();
  assert.ok(f.element.textContent.includes(text("reminder.noScheduledTasks")));
  assert.ok(f.displayed().includes(text("reminder.hostStatusLoading")));
  assert.equal(f.overview.freezeForClose(), false);
  await f.overview.refresh();
  assert.ok(f.displayed().includes(text("reminder.schedulingPaused")));
  settle(running); await first;
  assert.ok(f.displayed().includes(text("reminder.schedulingPaused")));
  assert.equal(f.displayed().includes(text("reminder.schedulingRunning")), false);
  assert.equal(f.overview.freezeForClose(), true); f.overview.thawAfterClose();
});

test("Retired overview ignores late desktop status without allocating a new locale scope", async t => {
  let settle!: (value: unknown) => void;
  const f = fixture(t, () => new Promise(resolve => { settle = resolve; }));
  const refresh = f.overview.refresh();
  f.overview.dispose(); const previous = f.displayed();
  settle(running); await refresh;
  assert.equal(f.displayed(), previous);
  assert.equal(f.calls.filter(call => call.route === "/api/desktop/status").length, 1);
});

function record(status: ReminderDelivery): ReminderRecordsPage {
  return { records: [{ owner: { workspaceId: "workspace", nodeId: "task" }, reminderId: "reminder", triggerInstant: "2026-10-26T09:00:59Z", status }], next: null };
}
const submitted: ReminderDelivery = {
  executionKey: "execution", delivery: { state: "submittedToSystem", at: "2026-10-26T09:01:10Z" },
  submissionObservation: {
    startedAt: "2026-10-26T09:00:59.125Z", returnedAt: "2026-10-26T09:00:59.130Z", elapsedNanoseconds: "5000000",
    result: { kind: "returnedSuccess" },
  }, handled: false, invalidated: null,
};

test("Delivery rows keep exact OS-call observations separate from ledger confirmation and switch their labels with the locale", async t => {
  const f = fixture(t, async () => running, false, () => record(structuredClone(submitted)));
  await f.overview.refresh();
  for (const language of ["zh-CN", "en-US"] as const) {
    locale.set(language);
    const displayed = f.element.textContent;
    const observation = submitted.submissionObservation!;
    assert.ok(displayed.includes(uiTextValue(message("reminder.submissionStartedAt", { at: observation.startedAt }))));
    assert.ok(displayed.includes(uiTextValue(message("reminder.submissionReturnedAt", { at: observation.returnedAt }))));
    assert.ok(displayed.includes(uiTextValue(message("reminder.submissionElapsed", { nanoseconds: "5000000" }))));
    assert.ok(displayed.includes(text("reminder.submissionReturnedSuccess")));
    assert.ok(displayed.includes(uiTextValue(message("reminder.ledgerConfirmedAt", { at: submitted.delivery.at! }))));
    assert.equal(displayed.includes(uiTextValue(message("reminder.submissionStartedAt", { at: submitted.delivery.at! }))), false);
    assert.equal(displayed.includes(text("reminder.submissionObservationMissing")), false);
  }
});

test("An OS-call failure retains its original HRESULT and exact duration without claiming known rejection or successful delivery", async t => {
  const unknown: ReminderDelivery = {
    ...submitted, delivery: { state: "unknown" },
    submissionObservation: { ...submitted.submissionObservation!, elapsedNanoseconds: "9007199254740993", result: { kind: "returnedFailure", hresult: -2147467259 } },
  };
  const f = fixture(t, async () => running, false, () => record(unknown));
  await f.overview.refresh();
  assert.ok(f.element.textContent.includes(text("reminder.unknown")));
  assert.ok(f.element.textContent.includes(uiTextValue(message("reminder.submissionReturnedFailure", { hresult: -2147467259 }))));
  assert.ok(f.element.textContent.includes(uiTextValue(message("reminder.submissionElapsed", { nanoseconds: "9007199254740993" }))));
  assert.equal(f.element.textContent.includes(text("reminder.submitted")), false);
  assert.equal(f.element.textContent.includes(text("reminder.failedBeforeCall")), false);
  assert.equal(f.element.textContent.includes(text("reminder.submissionReturnedSuccess")), false);
});

test("Historical ledger confirmation with a null or absent observation never invents the time of the actual OS call", async t => {
  let status: ReminderDelivery = { ...submitted, submissionObservation: null };
  const f = fixture(t, async () => running, false, () => record(status));
  for (const absent of [false, true]) {
    status = { ...submitted, submissionObservation: null };
    if (absent) Reflect.deleteProperty(status, "submissionObservation");
    await f.overview.refresh();
    assert.ok(f.element.textContent.includes(text("reminder.submissionObservationMissing")));
    assert.ok(f.element.textContent.includes(uiTextValue(message("reminder.ledgerConfirmedAt", { at: submitted.delivery.at! }))));
    assert.equal(f.element.textContent.includes(text("reminder.submissionReturnedSuccess")), false);
    assert.equal(f.element.textContent.includes(uiTextValue(message("reminder.submissionStartedAt", { at: submitted.delivery.at! }))), false);
    assert.equal(f.element.textContent.includes(uiTextValue(message("reminder.submissionReturnedAt", { at: submitted.delivery.at! }))), false);
  }
});

test("Summary overflow is explicitly in-app-only and cannot appear as a failed OS call", async t => {
  const f = fixture(t, async () => running, false, () => record({ ...submitted, delivery: { state: "inAppOnly", summaryKey: "summary" }, submissionObservation: null }));
  await f.overview.refresh();
  assert.ok(f.element.textContent.includes(text("reminder.inAppOnly")));
  assert.equal(f.element.textContent.includes(text("reminder.failedBeforeCall")), false);
  assert.equal(f.element.textContent.includes(text("reminder.submissionReturnedSuccess")), false);
  assert.equal(f.element.textContent.includes(text("reminder.submissionObservationMissing")), false, "In-app-only records explicitly establish no OS submission, rather than missing timing for a possible call");
});

test("The task's device control also labels summary overflow as in-app-only", async t => {
  const f = fixture(t, async () => running);
  const basis: DocumentBasis = { inputId: "input", root: "library", path: "task.typ", source: "task", sessionId: "session", serial: 1, baseId: "base" };
  const host = { getDocument: () => basis, async request(route: string) {
    assert.equal(route, "/api/reminders/inspect");
    return {
      candidate: { root: "library", owner: { workspaceId: "workspace", nodeId: "task" }, taskOpen: true, resolution: { state: "ready", reminderId: "reminder", triggerInstant: "2026-10-26T09:00:59Z", diagnostics: [] } },
      settings: { deviceId: "device", settingsRevision: 1, libraries: [], subscriptions: [] },
      eligibility: null, issue: null, delivery: { ...submitted, delivery: { state: "inAppOnly", summaryKey: "summary" }, submissionObservation: null },
    };
  } } as unknown as PanelHost;
  const control = new ReminderDeviceControl(host);
  f.element.append(control.element as unknown as Element);
  t.after(() => control.retire());
  await control.refresh(basis);
  assert.ok(control.element.textContent!.includes(text("reminder.inAppOnly")));
  assert.equal(control.element.textContent!.includes(text("reminder.failedBeforeCall")), false);
});
