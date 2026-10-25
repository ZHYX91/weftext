import { ProposalCloseGuard } from "./proposal-close-guard";
import { registerNativeProposalOwner } from "./native-proposal-owners";
import type { PanelHost } from "./data-panel";
import type { OpenedDocument } from "./types";
import type { ReminderDelivery, ReminderHostStatus, ReminderRecordsPage, ReminderScheduleCursor, ReminderSchedulePage, ReminderSettings } from "./reminder-types";
import { LocaleScope, errorUiText, message, type MessageKey, type MessageParams, type UiText } from "./ui-locale";

type ReminderKey = Extract<MessageKey, `reminder.${string}`>;
type LabelKey = { [K in ReminderKey]: keyof MessageParams<K> extends never ? K : never }[ReminderKey];
interface Pending { route: string; request: Record<string, unknown> }
const notificationLabels = {
  unavailable: "reminder.notificationUnavailable", unknown: "reminder.notificationUnknown", enabled: "reminder.notificationEnabled",
  disabledForApplication: "reminder.notificationDisabledForApplication", disabledForUser: "reminder.notificationDisabledForUser",
  disabledByPolicy: "reminder.notificationDisabledByPolicy", disabledByManifest: "reminder.notificationDisabledByManifest",
} as const satisfies Record<ReminderHostStatus["notificationState"], LabelKey>;
const schedulingLabels = {
  inactive: "reminder.schedulingInactive", running: "reminder.schedulingRunning", paused: "reminder.schedulingPaused",
  restricted: "reminder.schedulingRestricted", stopping: "reminder.schedulingStopping",
} as const satisfies Record<ReminderHostStatus["scheduling"], LabelKey>;
function isHostStatus(value: unknown): value is ReminderHostStatus {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const status = value as Partial<ReminderHostStatus>;
  return typeof status.desktopActive === "boolean" && typeof status.scanComplete === "boolean"
    && typeof status.notificationState === "string" && Object.hasOwn(notificationLabels, status.notificationState)
    && typeof status.scheduling === "string" && Object.hasOwn(schedulingLabels, status.scheduling)
    && Number.isSafeInteger(status.scopeIssueCount) && status.scopeIssueCount! >= 0;
}

/** One bounded, explicit personal status view. All queue entries are fresh
 * saved-source projections; delivery rows are the original Core effect ledger. */
export class ReminderOverview {
  private readonly closeGuard = new ProposalCloseGuard();
  assertCloseReady(): void { this.closeGuard.assertReady(this.pending.size !== 0 || this.inFlight.size !== 0); }
  assertDismissReady(): void { this.closeGuard.assertMutable(); this.assertCloseReady(); }
  bindDialog(dialog: HTMLDialogElement): void { this.closeGuard.bind(dialog); }
  freezeForClose(): boolean { return this.closeGuard.freeze(this.pending.size !== 0 || this.inFlight.size !== 0); }
  thawAfterClose(): void { this.closeGuard.thaw(); }
  readonly element = document.createElement("section");
  private readonly settingsHost = document.createElement("section");
  private readonly statusHost = document.createElement("section");
  private readonly queueHost = document.createElement("section");
  private readonly recordsHost = document.createElement("section");
  private settingsScope = new LocaleScope();
  private statusScope = new LocaleScope();
  private queueScope = new LocaleScope();
  private recordsScope = new LocaleScope();
  private generation = 0;
  private disposed = false;
  private settings: ReminderSettings | null = null;
  private readonly pending = new Map<string, Pending>();
  private readonly inFlight = new Set<Pending>();

  constructor(private readonly host: PanelHost) {
    this.closeGuard.bind(this.element);
    this.element.className = "reminder-overview quick-capture";
    this.statusHost.className = "reminder-host-status";
    this.statusHost.setAttribute("aria-live", "polite");
    this.element.append(this.settingsHost, this.queueHost, this.recordsHost);
  }
  dispose(): void { this.disposed = true; this.generation++; this.settingsScope.dispose(); this.statusScope.dispose(); this.queueScope.dispose(); this.recordsScope.dispose(); }
  private current(generation: number): boolean { return !this.disposed && !this.closeGuard.frozen && generation === this.generation; }
  private text(scope: LocaleScope, key: LabelKey, tag: "p" | "h2" | "h3" | "span" = "p"): HTMLElement {
    const node = document.createElement(tag); scope.bindText(node, message(key)); return node;
  }
  private detail(host: HTMLElement, scope: LocaleScope, value: UiText): void {
    const node = document.createElement("p"); node.className = "data-help"; scope.bindText(node, value); host.append(node);
  }
  private button(host: HTMLElement, scope: LocaleScope, key: LabelKey, action: (current: () => boolean) => Promise<void>): HTMLButtonElement {
    const generation = this.generation, node = document.createElement("button");
    const current = (): boolean => this.current(generation) && node.isConnected && this.element.contains(node);
    node.type = "button"; node.className = "data-button"; scope.bindText(node, message(key));
    node.onclick = () => {
      if (!current() || node.matches(":disabled")) return;
      node.disabled = true;
      void this.closeGuard.run(async () => {
        try { await action(current); }
        catch (error) { if (current()) this.detail(host, scope, errorUiText(error)); }
        finally { if (current()) node.disabled = false; }
      });
    };
    return node;
  }
  async refresh(): Promise<void> {
    return this.closeGuard.run(async () => {
      if (this.disposed) return;
      const generation = ++this.generation;
      this.settingsScope.dispose(); this.settingsScope = new LocaleScope();
      this.statusScope.dispose(); this.statusScope = new LocaleScope();
      this.statusHost.replaceChildren(this.text(this.statusScope, "reminder.hostStatus", "h3"), this.text(this.statusScope, "reminder.hostStatusLoading"));
      this.settingsHost.replaceChildren(this.text(this.settingsScope, "reminder.overview", "h2"), this.button(this.settingsHost, this.settingsScope, "reminder.refresh", () => this.refresh()), this.statusHost);
      await Promise.allSettled([this.loadHostStatus(generation), (async () => {
        await this.loadSettings(generation);
        if (this.current(generation)) await Promise.allSettled([this.loadQueue(null, generation), this.loadRecords(null, generation)]);
      })()]);
    });
  }
  private async loadSettings(generation: number): Promise<void> {
    try {
      const settings = await this.host.request<ReminderSettings>("/api/reminders/settings", {});
      if (!this.current(generation)) return;
      this.settings = settings; this.renderSettings(settings);
    } catch (error) {
      if (!this.current(generation)) return;
      this.settings = null; this.settingsHost.append(this.text(this.settingsScope, "reminder.unavailable"));
      this.detail(this.settingsHost, this.settingsScope, errorUiText(error));
      for (const [target, pending] of this.pending) {
        const row = document.createElement("section"); this.pendingControls(row, target, pending); this.settingsHost.append(row);
      }
    }
  }
  private async loadHostStatus(generation: number): Promise<void> {
    try {
      const status = await this.host.request<unknown>("/api/desktop/status");
      if (!this.current(generation)) return;
      this.statusScope.dispose(); this.statusScope = new LocaleScope();
      this.statusHost.replaceChildren(this.text(this.statusScope, "reminder.hostStatus", "h3"));
      if (!isHostStatus(status)) {
        this.statusHost.append(this.text(this.statusScope, "reminder.hostStatusUnavailable"));
        return;
      }
      if (!status.desktopActive) this.statusHost.append(this.text(this.statusScope, "reminder.desktopInactive"));
      this.statusHost.append(this.text(this.statusScope, notificationLabels[status.notificationState]), this.text(this.statusScope, schedulingLabels[status.scheduling]));
      if (!status.scanComplete) this.statusHost.append(this.text(this.statusScope, "reminder.scanIncomplete"));
      if (status.scopeIssueCount !== 0) this.detail(this.statusHost, this.statusScope, message("reminder.scopeIssues", { count: status.scopeIssueCount }));
    } catch (error) {
      if (!this.current(generation)) return;
      this.statusScope.dispose(); this.statusScope = new LocaleScope();
      this.statusHost.replaceChildren(this.text(this.statusScope, "reminder.hostStatus", "h3"), this.text(this.statusScope, "reminder.hostStatusUnavailable"));
      this.detail(this.statusHost, this.statusScope, errorUiText(error));
    }
  }
  private pendingControls(row: HTMLElement, target: string, pending: Pending): void {
    const retry = this.button(row, this.settingsScope, "reminder.retryOriginal", () => this.submit(target, pending));
    retry.disabled = this.inFlight.has(pending);
    row.append(this.text(this.settingsScope, "reminder.settingsPending"), retry, this.text(this.settingsScope, "reminder.pendingDiscardHelp"), this.button(row, this.settingsScope, "reminder.discardPending", async () => {
      if (this.pending.get(target) !== pending) return;
      this.pending.delete(target); await this.refresh();
    }));
  }
  private renderSettings(settings: ReminderSettings): void {
    for (const library of settings.libraries) {
      const row = document.createElement("section"); row.className = "data-property";
      this.detail(row, this.settingsScope, library.root); row.append(this.text(this.settingsScope, "reminder.libraryEnabled", "span"));
      const pending = this.pending.get(library.workspaceId);
      if (pending) this.pendingControls(row, library.workspaceId, pending);
      else row.append(this.button(row, this.settingsScope, library.enabled ? "reminder.disable" : "reminder.enable", async () => {
        const pending: Pending = { route: "/api/reminders/library", request: { ...(library.enabled ? { action: "disable", workspaceId: library.workspaceId } : { action: "enable", root: library.root }), expectedRevision: settings.settingsRevision, mutationId: crypto.randomUUID() } };
        this.pending.set(library.workspaceId, pending); await this.submit(library.workspaceId, pending);
      }));
      this.settingsHost.append(row);
    }
    for (const [target, pending] of this.pending) {
      if (settings.libraries.some(library => library.workspaceId === target)) continue;
      const row = document.createElement("section"); this.pendingControls(row, target, pending); this.settingsHost.append(row);
    }
  }
  private async submit(target: string, pending: Pending): Promise<void> {
    return this.closeGuard.run(async () => {
      if (this.inFlight.has(pending) || this.pending.get(target) !== pending) return;
      this.inFlight.add(pending);
      let failure: UiText | undefined;
      let original = false;
      try {
        await this.host.request(pending.route, structuredClone(pending.request));
        original = this.pending.get(target) === pending;
        if (original) this.pending.delete(target);
      } catch (error) { original = this.pending.get(target) === pending; failure = errorUiText(error); }
      finally { this.inFlight.delete(pending); }
      if (!original || this.disposed) return;
      const generation = this.generation;
      await this.refresh();
      if (failure !== undefined && this.current(generation + 1)) this.detail(this.settingsHost, this.settingsScope, failure);
    });
  }
  private delivery(host: HTMLElement, scope: LocaleScope, status: ReminderDelivery): void {
    if (status.invalidated) host.append(this.text(scope, "reminder.cancelled"));
    if (status.handled) host.append(this.text(scope, "reminder.handled"));
    host.append(this.text(scope, status.delivery.state === "submittedToSystem" ? "reminder.submitted" : status.delivery.state === "unknown" ? "reminder.unknown" : "reminder.failedBeforeCall"));
    if (status.delivery.at) this.detail(host, scope, status.delivery.at);
    if (status.delivery.diagnostic) this.detail(host, scope, status.delivery.diagnostic);
    if (!status.handled) host.append(this.button(host, scope, "reminder.handle", async current => {
      await this.host.request("/api/reminders/handle", { executionKey: status.executionKey }); if (current()) await this.refresh();
    }));
  }
  private async loadQueue(cursor: ReminderScheduleCursor | null, generation: number): Promise<void> {
    return this.closeGuard.run(async () => {
      if (!this.current(generation)) return;
      try {
        const page = await this.host.request<ReminderSchedulePage>("/api/reminders/schedule", { cursor });
        if (!this.current(generation)) return;
        this.queueScope.dispose(); this.queueScope = new LocaleScope();
        this.queueHost.replaceChildren(this.text(this.queueScope, "reminder.label", "h3"));
        if (cursor === null && page.complete && !page.next && !page.entries.length && !page.issues.length) this.queueHost.append(this.text(this.queueScope, "reminder.noScheduledTasks"));
        for (const entry of page.entries) {
          const row = document.createElement("section"); row.className = "data-property";
          this.detail(row, this.queueScope, entry.candidate.root);
          if (entry.candidate.resolution.triggerInstant) this.detail(row, this.queueScope, entry.candidate.resolution.triggerInstant);
          if (entry.eligibility?.retained) this.delivery(row, this.queueScope, entry.eligibility.retained);
          else if (entry.eligibility?.timing === "missed") { const text = document.createElement("p"); this.queueScope.bindText(text, message("reminder.missed", { count: 1 })); row.append(text); }
          else if (entry.eligibility) row.append(this.text(this.queueScope, entry.eligibility.timing === "due" ? "reminder.due" : "reminder.upcoming"));
          if (entry.issue) { row.append(this.text(this.queueScope, "reminder.unavailable")); this.detail(row, this.queueScope, entry.issue.message); }
          for (const diagnostic of entry.candidate.resolution.diagnostics) this.detail(row, this.queueScope, diagnostic);
          this.queueHost.append(row);
        }
        for (const issue of page.issues) {
          const row = document.createElement("section"); row.className = "data-warning";
          row.append(this.text(this.queueScope, "reminder.unavailable")); this.detail(row, this.queueScope, issue.message);
          if (this.settings) {
            const revision = this.settings.settingsRevision;
            row.append(this.button(row, this.queueScope, "reminder.disable", async () => {
              const target = JSON.stringify([issue.owner, issue.reminderId]);
              const pending = this.pending.get(target) ?? { route: "/api/reminders/subscription", request: { root: "", owner: structuredClone(issue.owner), reminderId: issue.reminderId, enabled: false, expectedRevision: revision, mutationId: crypto.randomUUID() } };
              this.pending.set(target, pending); await this.submit(target, pending);
            }));
          }
          this.queueHost.append(row);
        }
        if (page.next) { const next = page.next; this.queueHost.append(this.button(this.queueHost, this.queueScope, "reminder.more", () => this.loadQueue(next, generation))); }
      } catch (error) {
        if (!this.current(generation)) return;
        this.queueScope.dispose(); this.queueScope = new LocaleScope();
        this.queueHost.replaceChildren(this.text(this.queueScope, "reminder.unavailable")); this.detail(this.queueHost, this.queueScope, errorUiText(error));
      }
    });
  }
  private async loadRecords(after: string | null, generation: number): Promise<void> {
    return this.closeGuard.run(async () => {
      if (!this.current(generation)) return;
      try {
        const page = await this.host.request<ReminderRecordsPage>("/api/reminders/records", { after });
        if (!this.current(generation)) return;
        this.recordsScope.dispose(); this.recordsScope = new LocaleScope();
        this.recordsHost.replaceChildren(this.text(this.recordsScope, "reminder.deliveryRecords", "h3"));
        if (after === null && !page.records.length) this.recordsHost.append(this.text(this.recordsScope, "reminder.noDeliveryRecords"));
        for (const record of page.records) {
          const row = document.createElement("section"); row.className = "data-property"; row.dataset.executionKey = record.status.executionKey;
          const library = this.settings?.libraries.find(library => library.workspaceId === record.owner.workspaceId);
          if (library) this.detail(row, this.recordsScope, library.root);
          this.detail(row, this.recordsScope, record.triggerInstant); this.delivery(row, this.recordsScope, record.status);
          if (this.host.openResolved) row.append(this.button(row, this.recordsScope, "reminder.openTask", async current => {
            await this.host.prepareAction?.(); if (!current()) return;
            const document = await this.host.request<OpenedDocument>("/api/reminders/open", { executionKey: record.status.executionKey });
            await this.host.openResolved!(document);
          }));
          this.recordsHost.append(row);
        }
        if (page.next) { const next = page.next; this.recordsHost.append(this.button(this.recordsHost, this.recordsScope, "reminder.more", () => this.loadRecords(next, generation))); }
      } catch (error) {
        if (!this.current(generation)) return;
        this.recordsScope.dispose(); this.recordsScope = new LocaleScope();
        this.recordsHost.replaceChildren(this.text(this.recordsScope, "reminder.unavailable")); this.detail(this.recordsHost, this.recordsScope, errorUiText(error));
      }
    });
  }
}

/** Shared entry for the native menu and the open task's properties. */
export function openReminderOverview(host: PanelHost): HTMLDialogElement {
  const dialog = document.createElement("dialog"); dialog.className = "capture-dialog reminder-overview-dialog";
  const scope = new LocaleScope(); scope.bindAttr(dialog, "aria-label", message("reminder.overview"));
  const close = document.createElement("button"); close.type = "button"; close.className = "data-button capture-close"; scope.bindText(close, message("common.close"));
  const overview = new ReminderOverview(host), unregister = registerNativeProposalOwner(overview);
  overview.bindDialog(dialog);
  dialog.append(close, overview.element);
  const dismiss = (): void => {
    try { overview.assertDismissReady(); }
    catch (error) { host.notify(errorUiText(error), true); return; }
    dialog.close();
  };
  close.onclick = dismiss;
  dialog.addEventListener("cancel", event => { event.preventDefault(); dismiss(); });
  dialog.addEventListener("close", () => {
    try { overview.assertDismissReady(); }
    catch (error) { host.notify(errorUiText(error), true); dialog.showModal(); return; }
    unregister(); overview.dispose(); scope.dispose(); dialog.remove();
  });
  document.body.append(dialog); dialog.showModal(); void overview.refresh(); return dialog;
}
