import type { DocumentBasis, PanelHost } from "./data-panel";
import { LocaleScope, errorUiText, message, type MessageKey, type UiText } from "./ui-locale";
import type { ReminderOwner as Owner, ReminderInspection as Inspection } from "./reminder-types";
import { openReminderOverview } from "./reminder-overview";

type LabelKey = Exclude<Extract<MessageKey, `reminder.${string}`>, "reminder.missed">;
interface Pending { route: string; request: Record<string, unknown> }
const ownerEqual = (a: Owner, b: Owner): boolean => a.workspaceId === b.workspaceId && a.nodeId === b.nodeId;
const sameBasis = (a: DocumentBasis | null, b: DocumentBasis): boolean => !!a && a.sessionId === b.sessionId && a.source === b.source && a.serial === b.serial && a.baseId === b.baseId && a.path === b.path && a.root === b.root && a.nodeId === b.nodeId && a.workspaceId === b.workspaceId && !!a.dirty === !!b.dirty;

/** Personal consent belongs to saved NodeRef/reminder identity and this Core
 * device, never to a copied field, an unsaved Draft or a browser timezone. */
export class ReminderDeviceControl {
  readonly element = document.createElement("section");
  private scope = new LocaleScope();
  private generation = 0;
  private activeBasis: DocumentBasis | null = null;
  private readonly pending = new Map<string, Pending>();
  private readonly inFlight = new Set<Pending>();

  constructor(private readonly host: PanelHost) { this.element.className = "reminder-device-control"; }
  retire(): void { this.generation++; this.activeBasis = null; this.scope.dispose(); this.scope = new LocaleScope(); this.element.replaceChildren(); }
  private current(generation: number, basis: DocumentBasis): boolean { return generation === this.generation && sameBasis(this.host.getDocument(), basis); }
  private text(tag: "p" | "h3" | "span", key: LabelKey, className = ""): HTMLElement {
    const element = document.createElement(tag); element.className = className; this.scope.bindText(element, message(key)); return element;
  }
  private detail(value: UiText, className = "data-help"): void {
    const node = document.createElement("p"); node.className = className; this.scope.bindText(node, value); this.element.append(node);
  }
  private button(key: LabelKey, basis: DocumentBasis, action: () => Promise<void>, requiresBasis = true): HTMLButtonElement {
    const generation = this.generation, button = document.createElement("button");
    const current = (): boolean => generation === this.generation && (!requiresBasis || sameBasis(this.host.getDocument(), basis)) && button.isConnected && this.element.contains(button);
    button.type = "button"; button.className = "data-button"; this.scope.bindText(button, message(key));
    button.onclick = () => {
      if (!current() || button.matches(":disabled")) return;
      button.disabled = true;
      void action().catch(error => { if (current()) this.detail(errorUiText(error), "data-warning"); })
        .finally(() => { if (current()) button.disabled = false; });
    };
    return button;
  }
  async refresh(input: DocumentBasis): Promise<void> {
    const basis = structuredClone(input), generation = ++this.generation;
    this.activeBasis = basis;
    if (!this.current(generation, basis)) return;
    try {
      const result = await this.host.request<Inspection>("/api/reminders/inspect", { sessionId: basis.sessionId });
      if (!this.current(generation, basis)) return;
      this.render(result, basis);
    } catch (error) {
      if (!this.current(generation, basis)) return;
      this.scope.dispose(); this.scope = new LocaleScope();
      this.element.replaceChildren(this.text("h3", "reminder.enabledDevice"), this.button("reminder.overview", basis, async () => { openReminderOverview(this.host); }, false), this.text("p", "reminder.unavailable", "data-warning"));
      this.detail(errorUiText(error)); this.element.append(this.refreshButton(basis));
    }
  }
  private render(result: Inspection, basis: DocumentBasis): void {
    this.scope.dispose(); this.scope = new LocaleScope();
    this.element.replaceChildren(this.text("h3", "reminder.enabledDevice"), this.button("reminder.overview", basis, async () => { openReminderOverview(this.host); }, false));
    const { candidate, settings, eligibility } = result;
    const target = JSON.stringify([candidate.owner.workspaceId, candidate.owner.nodeId]);
    const library = settings.libraries.find(item => item.workspaceId === candidate.owner.workspaceId);
    const subscription = settings.subscriptions.find(item => ownerEqual(item.owner, candidate.owner) && item.reminderId === candidate.resolution.reminderId);
    const pending = this.pending.get(target);
    const run = async (route: string, request: Record<string, unknown>): Promise<void> => {
      const original = this.pending.get(target) ?? { route, request: structuredClone(request) };
      this.pending.set(target, original); await this.submit(target, original, basis);
    };
    if (pending) {
      const retry = this.button("reminder.retryOriginal", basis, () => this.submit(target, pending, basis)); retry.disabled = this.inFlight.has(pending);
      this.element.append(this.text("p", "reminder.settingsPending", "data-warning"), retry, this.text("p", "reminder.pendingDiscardHelp", "data-help"), this.button("reminder.discardPending", basis, async () => {
        if (this.pending.get(target) !== pending) return;
        this.pending.delete(target); await this.refresh(basis);
      })); return;
    }
    const libraryButton = this.button(library?.enabled ? "reminder.disable" : "reminder.enable", basis, () => run("/api/reminders/library", {
      ...(library?.enabled ? { action: "disable", workspaceId: candidate.owner.workspaceId } : { action: "enable", root: candidate.root }), expectedRevision: settings.settingsRevision, mutationId: crypto.randomUUID(),
    }));
    libraryButton.dataset.controlKey = JSON.stringify(["device/reminders", candidate.owner.workspaceId, "library-enabled"]);
    const libraryRow = document.createElement("div"); libraryRow.append(this.text("span", "reminder.libraryEnabled"), libraryButton); this.element.append(libraryRow);
    if (basis.dirty) this.element.append(this.text("p", "reminder.saveFirst", "data-help"));
    if (candidate.resolution.state === "ready" && candidate.resolution.reminderId) {
      const toggle = this.button(subscription?.enabled ? "reminder.disable" : "reminder.enable", basis, () => run("/api/reminders/subscription", {
        root: candidate.root, owner: candidate.owner, reminderId: candidate.resolution.reminderId, enabled: !subscription?.enabled, expectedRevision: settings.settingsRevision, mutationId: crypto.randomUUID(),
      }));
      toggle.disabled = !subscription?.enabled && (!library?.enabled || !!basis.dirty);
      toggle.dataset.controlKey = JSON.stringify(["device/reminders", candidate.owner, candidate.resolution.reminderId, "subscription-enabled"]);
      const reminderRow = document.createElement("div"); reminderRow.append(this.text("span", "reminder.label"), toggle); this.element.append(reminderRow);
      this.detail(candidate.resolution.triggerInstant ?? "");
    }
    if (eligibility || result.delivery) {
      const retained = result.delivery ?? eligibility?.retained;
      if (retained?.invalidated) this.element.append(this.text("p", "reminder.cancelled"));
      if (retained?.handled) this.element.append(this.text("p", "reminder.handled"));
      if (retained) {
        this.element.append(this.text("p", retained.delivery.state === "submittedToSystem" ? "reminder.submitted" : retained.delivery.state === "unknown" ? "reminder.unknown" : "reminder.failedBeforeCall"));
        if (retained.delivery.diagnostic) this.detail(retained.delivery.diagnostic);
        if (!retained.handled) this.element.append(this.button("reminder.handle", basis, async () => {
          const generation = this.generation; await this.host.request("/api/reminders/handle", { executionKey: retained.executionKey });
          if (this.current(generation, basis)) await this.refresh(basis);
        }));
      } else if (eligibility?.timing === "missed") {
        const missed = document.createElement("p"); this.scope.bindText(missed, message("reminder.missed", { count: 1 })); this.element.append(missed);
      } else if (eligibility) this.element.append(this.text("p", eligibility.timing === "upcoming" ? "reminder.upcoming" : "reminder.due"));
    }
    if (!candidate.taskOpen) this.element.append(this.text("p", "reminder.cancelled"));
    if (result.issue && result.issue.code !== "reminderDisabled") this.detail(result.issue.message, "data-warning");
    for (const diagnostic of candidate.resolution.diagnostics) this.detail(diagnostic, "data-warning");
    this.element.append(this.refreshButton(basis));
  }
  private refreshButton(basis: DocumentBasis): HTMLButtonElement {
    return this.button("reminder.refresh", basis, async () => { const current = this.host.getDocument(); if (current) await this.refresh(current); }, false);
  }
  private async submit(target: string, pending: Pending, basis: DocumentBasis): Promise<void> {
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
    if (!original || !sameBasis(this.activeBasis, basis) || !sameBasis(this.host.getDocument(), basis)) return;
    const generation = this.generation;
    await this.refresh(basis);
    if (failure !== undefined && this.current(generation + 1, basis)) this.detail(failure, "data-warning");
  }
}
