import type { DocumentBasis, PanelHost } from "./data-panel";
import { sameSourceInput } from "./source-input-basis";
import { LocaleScope, errorUiText, message, type MessageKey, type MessageParams, type UiText } from "./ui-locale";
import type { ReminderOwner as Owner, ReminderInspection as Inspection } from "./reminder-types";
import { openReminderOverview } from "./reminder-overview";
import { ProposalCloseGuard } from "./proposal-close-guard";
import { coreDescriptionUiText, coreErrorUiText, descriptionUiText } from "./ui-diagnostics";

type ReminderKey = Extract<MessageKey, `reminder.${string}`>;
type LabelKey = { [K in ReminderKey]: keyof MessageParams<K> extends never ? K : never }[ReminderKey];
interface Pending { route: string; request: Record<string, unknown>; basis: DocumentBasis }
const ownerEqual = (a: Owner, b: Owner): boolean => a.workspaceId === b.workspaceId && a.nodeId === b.nodeId;
const sameBasis = (a: DocumentBasis | null, b: DocumentBasis): boolean => sameSourceInput(a, b) && !!a!.dirty === !!b.dirty;

/** Personal consent belongs to saved NodeRef/reminder identity and this Core
 * device, never to a copied field, an unsaved Draft or a browser timezone. */
export class ReminderDeviceControl {
  readonly element = document.createElement("section");
  private readonly pendingElement = document.createElement("section");
  private scope = new LocaleScope();
  private generation = 0;
  private activeBasis: DocumentBasis | null = null;
  private readonly pending = new Map<string, Pending>();
  private readonly inFlight = new Set<Pending>();
  private readonly closeGuard = new ProposalCloseGuard();

  constructor(private readonly host: PanelHost) { this.element.className = "reminder-device-control"; this.closeGuard.bind(this.element); }
  assertCloseReady(): void { this.closeGuard.assertReady(this.pending.size > 0 || this.inFlight.size > 0); }
  freezeForClose(): boolean { return this.closeGuard.freeze(this.pending.size > 0 || this.inFlight.size > 0); }
  thawAfterClose(): void { this.closeGuard.thaw(); }
  hasPending(): boolean { return this.pending.size > 0; }
  retire(): void { if (this.closeGuard.frozen) return; this.generation++; this.activeBasis = null; this.scope.dispose(); this.scope = new LocaleScope(); this.element.replaceChildren(); this.renderPending(); }
  private current(generation: number, basis: DocumentBasis): boolean { return !this.closeGuard.frozen && generation === this.generation && sameBasis(this.host.getDocument(), basis); }
  private text(tag: "p" | "h3" | "span", key: LabelKey, className = ""): HTMLElement {
    const element = document.createElement(tag); element.className = className; this.scope.bindText(element, message(key)); return element;
  }
  private detail(value: UiText, className = "data-help"): void {
    const node = document.createElement("p"); node.className = className; this.scope.bindText(node, value); this.element.append(node);
  }
  private button(key: LabelKey, basis: DocumentBasis, action: () => Promise<void>, requiresBasis = true): HTMLButtonElement {
    const generation = this.generation, button = document.createElement("button");
    const current = (): boolean => !this.closeGuard.frozen && generation === this.generation && (!requiresBasis || sameBasis(this.host.getDocument(), basis)) && button.isConnected && this.element.contains(button);
    button.type = "button"; button.className = "data-button"; this.scope.bindText(button, message(key));
    button.onclick = () => {
      if (!current() || button.matches(":disabled")) return;
      button.disabled = true;
      void this.closeGuard.run(async () => {
        try { await action(); }
        catch (error) { if (current()) this.detail(errorUiText(error), "data-warning"); }
        finally { if (current()) button.disabled = false; }
      });
    };
    return button;
  }
  private renderPending(): void {
    if (this.closeGuard.frozen) return;
    this.pendingElement.replaceChildren();
    for (const [target, pending] of this.pending) {
      const name = document.createElement("p"); name.textContent = pending.basis.path;
      const retry = this.button("reminder.retryOriginal", pending.basis, () => this.submit(target, pending, pending.basis), false);
      const discard = this.button("reminder.discardPending", pending.basis, async () => {
        if (this.inFlight.has(pending) || this.pending.get(target) !== pending) return;
        this.pending.delete(target); this.renderPending();
      }, false);
      retry.disabled = discard.disabled = this.inFlight.has(pending);
      this.pendingElement.append(name, this.text("p", "reminder.settingsPending", "data-warning"), retry, this.text("p", "reminder.pendingDiscardHelp", "data-help"), discard);
    }
    if (this.pending.size && !this.element.contains(this.pendingElement)) this.element.append(this.pendingElement);
  }
  async refresh(input: DocumentBasis): Promise<void> {
    if (this.closeGuard.frozen) return;
    return this.closeGuard.run(() => this.refreshOwned(input));
  }
  private async refreshOwned(input: DocumentBasis): Promise<void> {
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
      this.renderPending();
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
      const original = this.pending.get(target) ?? { route, request: structuredClone(request), basis: structuredClone(basis) };
      this.pending.set(target, original); await this.submit(target, original, basis);
    };
    if (pending) {
      this.renderPending(); return;
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
        this.element.append(this.text("p", retained.delivery.state === "submittedToSystem" ? "reminder.submitted" : retained.delivery.state === "unknown" ? "reminder.unknown" : retained.delivery.state === "inAppOnly" ? "reminder.inAppOnly" : "reminder.failedBeforeCall"));
        if (retained.delivery.state === "unknown" || retained.delivery.state === "failedBeforeCall") this.detail(coreDescriptionUiText(retained.delivery.description));
        if (!retained.handled) this.element.append(this.button("reminder.handle", basis, async () => {
          const generation = this.generation; await this.host.request("/api/reminders/handle", { executionKey: retained.executionKey });
          if (this.current(generation, basis)) await this.refresh(basis);
        }));
      } else if (eligibility?.timing === "missed") {
        const missed = document.createElement("p"); this.scope.bindText(missed, message("reminder.missed", { count: 1 })); this.element.append(missed);
      } else if (eligibility) this.element.append(this.text("p", eligibility.timing === "upcoming" ? "reminder.upcoming" : "reminder.due"));
    }
    if (!candidate.taskOpen) this.element.append(this.text("p", "reminder.cancelled"));
    if (result.issue && result.issue.code !== "reminderDisabled") this.detail(coreErrorUiText(result.issue), "data-warning");
    for (const diagnostic of candidate.resolution.diagnostics) this.detail(descriptionUiText(diagnostic), "data-warning");
    this.element.append(this.refreshButton(basis));
    this.renderPending();
  }
  private refreshButton(basis: DocumentBasis): HTMLButtonElement {
    return this.button("reminder.refresh", basis, async () => { const current = this.host.getDocument(); if (current) await this.refresh(current); }, false);
  }
  private async submit(target: string, pending: Pending, basis: DocumentBasis): Promise<void> {
    if (this.inFlight.has(pending) || this.pending.get(target) !== pending) return;
    this.inFlight.add(pending);
    this.renderPending();
    let failure: UiText | undefined;
    let original = false;
    try {
      await this.host.request(pending.route, structuredClone(pending.request));
      original = this.pending.get(target) === pending;
      if (original) this.pending.delete(target);
    } catch (error) { original = this.pending.get(target) === pending; failure = errorUiText(error); }
    finally { this.inFlight.delete(pending); this.renderPending(); }
    if (!original || !sameBasis(this.activeBasis, basis) || !sameBasis(this.host.getDocument(), basis)) return;
    const generation = this.generation;
    await this.refresh(basis);
    if (failure !== undefined && this.current(generation + 1, basis)) this.detail(failure, "data-warning");
  }
}
