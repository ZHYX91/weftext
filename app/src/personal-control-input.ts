import { DocumentNotesInputTracker } from "./personal-input-text";
import { personalInputId } from "./source-input-origin";
export interface PersonalControlContext { audience: string; resource: string; reference?: string; epoch: number }
type TextControl = HTMLInputElement | HTMLTextAreaElement;
interface Tracked { id: string; context: PersonalControlContext; tracker: DocumentNotesInputTracker; lastValue: string; inputObserved: boolean; edit?: { from: number; to: number; inputType: string }; composition?: { tracker: DocumentNotesInputTracker; from: number; to: number }; fallback: string[] }
/** Actual form input evidence, never the initial field/schema/native result. */
export class PersonalControlInput {
  private tracked = new Map<TextControl, Tracked>();
  constructor(private document: Document, private context: () => PersonalControlContext | null, private retain: (context: PersonalControlContext, id: string, text: string) => void) {
    document.addEventListener("beforeinput", event => {
      const control = this.control(event.target), input = event as InputEvent; if (!control) return;
      const tracked = this.track(control); if (!tracked) return;
      this.protectProgrammaticValue(control, tracked);
      if (control.selectionStart !== null && control.selectionEnd !== null) tracked.edit = { from: control.selectionStart, to: control.selectionEnd, inputType: input.inputType };
    }, true);
    document.addEventListener("input", event => {
      const control = this.control(event.target); if (!control) return; const tracked = this.track(control); if (!tracked) return;
      const input = event as InputEvent, data = input.data ?? input.dataTransfer?.getData("text/plain");
      if (tracked.composition) { tracked.tracker = tracked.composition.tracker.copy(); tracked.tracker.observe(control.value, { from: tracked.composition.from, to: tracked.composition.to, inputType: "insertCompositionText" }, data); }
      else if (tracked.edit || !tracked.inputObserved || tracked.lastValue !== control.value) tracked.tracker.observe(control.value, tracked.edit, input.inputType?.startsWith("insert") ? data : undefined);
      tracked.inputObserved = true; tracked.lastValue = control.value; tracked.edit = undefined; this.publish(tracked);
    }, true);
    document.addEventListener("compositionstart", event => {
      const control = this.control(event.target); if (!control || control.selectionStart === null || control.selectionEnd === null) return;
      const tracked = this.track(control); if (tracked) { this.protectProgrammaticValue(control, tracked); tracked.composition = { tracker: tracked.tracker.copy(), from: control.selectionStart, to: control.selectionEnd }; }
    }, true);
    document.addEventListener("compositionend", event => {
      const control = this.control(event.target), tracked = control && this.tracked.get(control); if (!control || !tracked?.composition) return;
      const context = this.context(); if (!context || !this.sameContext(context, tracked.context)) { this.tracked.delete(control); return; }
      tracked.tracker = tracked.composition.tracker.copy(); tracked.tracker.observe(control.value, { from: tracked.composition.from, to: tracked.composition.to, inputType: "insertCompositionText" }, (event as CompositionEvent).data); tracked.inputObserved = true; tracked.lastValue = control.value; tracked.composition = undefined; tracked.edit = undefined; this.publish(tracked);
    }, true);
    document.addEventListener("weftext-personal-control-input", event => {
      const control = this.control(event.target), text = (event as CustomEvent<{ text?: string }>).detail?.text; if (!control || typeof text !== "string") return;
      const tracked = this.track(control); if (tracked) { tracked.fallback.push(text); this.publish(tracked); }
    });
  }
  clear(): void { this.tracked.clear(); }
  private sameContext(a: PersonalControlContext, b: PersonalControlContext): boolean { return a.audience === b.audience && a.epoch === b.epoch && a.resource === b.resource; }
  private control(target: EventTarget | null): TextControl | null {
    if (!(target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) || !target.isConnected || target.readOnly || target.disabled || target.dataset.personalInputManaged || target.id === "token" || target instanceof HTMLInputElement && ["password", "hidden", "file", "checkbox", "radio", "button", "submit"].includes(target.type)) return null;
    return target;
  }
  private track(control: TextControl): Tracked | undefined {
    const context = this.context(); if (!context) return;
    for (const old of this.tracked.keys()) if (!old.isConnected) this.tracked.delete(old);
    let tracked = this.tracked.get(control);
    if (!tracked || tracked.context.audience !== context.audience || tracked.context.epoch !== context.epoch || tracked.context.resource !== context.resource) {
      tracked = { id: personalInputId(), context, tracker: new DocumentNotesInputTracker(control.value), lastValue: control.value, inputObserved: false, fallback: [] }; this.tracked.set(control, tracked);
    }
    return tracked;
  }
  private protectProgrammaticValue(control: TextControl, tracked: Tracked): void {
    if (control.value === tracked.lastValue) return;
    const known = tracked.tracker.recoveryInput(); if (known) tracked.fallback.push(known);
    tracked.tracker = new DocumentNotesInputTracker(control.value); tracked.lastValue = control.value; tracked.edit = undefined; tracked.composition = undefined;
  }
  private publish(tracked: Tracked): void {
    const current = this.context(); if (!current || !this.sameContext(current, tracked.context)) return;
    this.retain(tracked.context, tracked.id, [...tracked.fallback, tracked.tracker.recoveryInput()].filter(Boolean).join("\n"));
  }
}
/** Explicit clipboard reads assign a value without a native input event. */
export function retainProgrammaticControlInput(control: TextControl, text: string): void { control.dispatchEvent(new CustomEvent("weftext-personal-control-input", { bubbles: true, detail: { text } })); }
