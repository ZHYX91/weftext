import type { AuthorInputFactory, AuthorInputSession, AuthorInputSnapshot } from "./author-input";
import { CaptureController, type CaptureControllerState, type CaptureMutableRecord } from "./capture-controller";
import { captureString, captureText, observeCaptureText, type CaptureInput } from "./capture-draft-state";
import { LocaleScope, message, errorUiText, setUiText, UiError, type UiMessage } from "./ui-locale";

type TextSlot = "name" | "title" | "body" | "rootInput" | "parentInput" | "parametersText";
export interface CaptureFormLabels {
  name: UiMessage; authorTitle: UiMessage; bodyMode: UiMessage; plain: UiMessage; markup: UiMessage;
  advanced: UiMessage; keepClose: UiMessage; confirmOriginal: UiMessage;
  draftSaved: UiMessage;
}
export interface CaptureAdvancedControls {
  /** Synchronously sample only this draft's live, positively registered inputs. */
  collectOwnedRaw(): void;
  /** Replace rendered values only after a Core-qualified advance, preserving raw. */
  update(input: CaptureInput,consumedCut?:boolean): void;
  retireAuthorization(): void;
  dispose(): void;
}
export interface CaptureFormHost<R extends CaptureMutableRecord> {
  controller: CaptureController<R>;
  authorInputs: AuthorInputFactory;
  labels: CaptureFormLabels;
  target: HTMLElement;
  advanced(container: HTMLElement): CaptureAdvancedControls;
  submit(mode: "close" | "next" | "open"): Promise<void>;
  confirmOriginal(): Promise<void>;
  requestClose(): Promise<void>;
  canOpen?():boolean;
  canSaveNext?():boolean;
  showName?:boolean;
}

/** Presentation owns stable controls; its only storage writer is the supplied
 * Core controller. Labels/locale updates never replace producer DOM nodes. */
export class CaptureForm<R extends CaptureMutableRecord> {
  readonly element = document.createElement("section");
  private readonly labels = new LocaleScope();
  private readonly controls = new Map<TextSlot, HTMLInputElement | HTMLTextAreaElement>();
  private readonly status = document.createElement("p");
  private readonly diagnostic = document.createElement("pre");
  private readonly buttons: HTMLButtonElement[] = [];
  private openButton?:HTMLButtonElement;
  private nextButton?:HTMLButtonElement;
  private readonly session: AuthorInputSession;
  private readonly advanced: CaptureAdvancedControls;
  private readonly mode = document.createElement("select");
  private active = true;
  private busy = false;
  private renderedInputRevision: number;
  constructor(private readonly host: CaptureFormHost<R>) {
    this.element.className = "capture-surface";
    const state = host.controller.state;
    this.renderedInputRevision = state.draft.inputRevision;
    const session = host.authorInputs({ kind: "capture", draftId: state.draft.draftId }, () => this.active, snapshot => this.accept(snapshot), () => host.controller.editRevision);
    if (!session) throw new Error("捕获表单没有真实作者输入所有者，未挂载。 ");
    this.session = session;
    const title = document.createElement("h1"); this.labels.bindText(title, message("capture.title"));
    this.element.append(title, host.target);
    this.text("title", host.labels.authorTitle, state.input);
    this.text("body", message("capture.body"), state.input, true);
    const details = document.createElement("details"), summary = document.createElement("summary");
    this.labels.bindText(summary, host.labels.advanced); details.append(summary);
    if(host.showName!==false)this.text("name", host.labels.name, state.input, false, details);
    const modeLabel = document.createElement("label"), modeText = document.createElement("span"); this.labels.bindText(modeText, host.labels.bodyMode);
    this.mode.dataset.controlKey = JSON.stringify(["capture", state.draft.draftId, "bodyMode"]);
    for (const [value, label] of [["text", host.labels.plain], ["markup", host.labels.markup]] as const) {
      const option = document.createElement("option"); option.value = value; this.labels.bindText(option, label); this.mode.append(option);
    }
    this.mode.value = state.input.bodyMode;
    this.session.bindChoice(this.mode, "bodyMode");
    modeLabel.append(modeText, this.mode); details.append(modeLabel);
    const advanced = document.createElement("section"); details.append(advanced); this.element.append(details);
    this.advanced = host.advanced(advanced);
    const actions = document.createElement("div"); actions.className = "capture-actions";
    this.openButton=this.button(message("capture.openNote"), () => host.submit("open"));
    this.nextButton=this.button(message("capture.saveNext"),async()=>{if(host.canSaveNext?.()===false)throw new UiError(message("capture.targetRequired"));await host.submit("next");});
    actions.append(this.button(message("capture.saveClose"), () => host.submit("close")), this.nextButton, this.openButton, this.button(host.labels.confirmOriginal, () => host.confirmOriginal()), this.button(host.labels.keepClose, () => host.requestClose()), this.button(message("common.retry"), () => host.controller.retryPersistence()));
    this.status.setAttribute("aria-live", "polite"); this.diagnostic.className = "capture-diagnostic";
    this.labels.bindText(this.status, () => {
      const current = this.host.controller.state;
      return current.blocked ? message("desktop.unsavedClose") : current.persisting ? message("capture.persisting") : current.draft.pending ? message("capture.pending") : current.formReady ? host.labels.draftSaved : message("capture.rawRetained");
    });
    this.element.append(actions, this.status, this.diagnostic);
    this.element.addEventListener("compositionstart", () => { if (this.active) host.controller.beginComposition(); });
    this.element.addEventListener("compositionend", () => { host.controller.endComposition(); });
    this.update(state);
  }
  private text(slot: TextSlot, label: UiMessage, input: CaptureInput, multiline = false, parent: HTMLElement = this.element): void {
    const row = document.createElement("label"), text = document.createElement("span"); this.labels.bindText(text, label);
    const control = multiline ? document.createElement("textarea") : document.createElement("input");
    control.className = "data-input";
    if (control instanceof HTMLTextAreaElement) control.rows = 9;
    control.value = captureString(input[slot]);
    control.dataset.controlKey = JSON.stringify(["capture", this.host.controller.state.draft.draftId, slot]);
    this.controls.set(slot, control); this.session.bind(control, slot);
    row.append(text, control); parent.append(row);
  }
  private accept(snapshot: AuthorInputSnapshot): void {
    if (!this.active || snapshot.context.owner.kind !== "capture" || snapshot.context.owner.draftId !== this.host.controller.state.draft.draftId) return;
    this.host.controller.observeAuthorCut(snapshot);
    if (snapshot.kind === "choice" && snapshot.context.slot === "bodyMode") {
      if (!["text", "markup"].includes(snapshot.rawValue)) throw new Error("捕获正文模式不是当前注册选择。");
      const input = this.host.controller.state.input; input.bodyMode = input.authorBodyMode = snapshot.rawValue as "text" | "markup"; this.host.controller.edit(input); return;
    }
    const slot = snapshot.context.slot as TextSlot;
    if (!this.controls.has(slot)) throw new Error("捕获控件没有注册到当前原文 slot。");
    const input = this.host.controller.state.input; observeCaptureText(input,slot,snapshot); this.host.controller.edit(input);
  }
  private button(label: UiMessage, run: () => Promise<unknown>): HTMLButtonElement {
    const button = document.createElement("button"); button.type = "button"; button.className = "data-button"; this.labels.bindText(button, label); this.buttons.push(button);
    button.onclick = () => { if (!this.active || this.busy) return;try {this.collectOwnedRaw(); this.busy = true; this.updateButtons(); void run().catch(error => this.showError(error)).finally(() => { this.busy = false; this.update(this.host.controller.state); });}catch(error){this.busy=false;this.updateButtons();this.showError(error);} };
    return button;
  }
  collectOwnedRaw(): void {
    if (!this.active) return;
    const input = this.host.controller.state.input;
    let changed = false;
    for (const [slot, control] of this.controls) {
      const value = captureText(control.value);
      if (JSON.stringify(input[slot]) !== JSON.stringify(value)) { input[slot] = value; changed = true; }
    }
    if (changed) this.host.controller.edit(input);
    this.advanced.collectOwnedRaw();
  }
  update(state: CaptureControllerState<R>): void {
    if (state.blocked) this.showError(state.error);
    this.status.dataset.state = state.blocked ? "blocked" : state.persisting ? "persisting" : state.draft.pending ? "pending" : state.formReady ? "ready" : "recovery";
    if (this.renderedInputRevision !== state.draft.inputRevision && state.editRevision === this.host.controller.editRevision && !state.persisting) {
      // A Core advance may clear only its consumed cut. Controller.state.input
      // is the later in-memory value when newer edits exist.
      for (const [slot, control] of this.controls) { const value = captureString(state.input[slot]); if (control.value !== value) control.value = value; }
      this.mode.value = state.input.bodyMode;
      this.advanced.update(state.input,state.draft.lastMutation.kind==="advance");
      this.renderedInputRevision = state.draft.inputRevision;
    }
    this.labels.refresh();
    this.updateButtons();
  }
  private updateButtons(): void { for (const button of this.buttons) button.disabled = !this.active || this.busy; if(this.openButton)this.openButton.disabled ||= !this.host.canOpen?.();if(this.nextButton)this.nextButton.disabled ||= this.host.canSaveNext?.()===false; }
  showError(error: unknown): void { setUiText(this.diagnostic,errorUiText(error)); }
  /** Same-actor recovery remains raw/read-only; protected advanced controls are
   * retired after the synchronous final sample, before their source is hidden. */
  retireAuthorization(): void {
    if (!this.active) return;
    this.collectOwnedRaw(); this.active = false; this.session.retire(); this.advanced.retireAuthorization();
    for (const control of this.controls.values()) control.readOnly = true;
    this.mode.disabled = true; this.updateButtons();
  }
  freeze(): void { this.element.inert = true; }
  thaw(): void { if (this.active) this.element.inert = false; }
  dispose(): void { this.active = false; this.session.retire(); this.advanced.dispose(); this.labels.dispose(); }
}
