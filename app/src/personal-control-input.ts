import { DocumentNotesInputTracker } from "./personal-input-text";
import { personalInputId } from "./source-input-origin";
import { registerAuthorInputLease, type AuthorInputContext, type AuthorInputLease, type AuthorInputOwner, type AuthorInputSession, type AuthorInputSink, type ValuePath } from "./author-input";

type TextControl = HTMLInputElement | HTMLTextAreaElement;
interface Slot { id: string; revision: number; slot: string; path: readonly ValuePath[]; state?: Tracked }
interface Tracked { token: object; control: TextControl; context: AuthorInputContext; slot: Slot; tracker: DocumentNotesInputTracker; lastValue: string; inputObserved: boolean; fallback: string[]; current(): boolean; leaseRevision?: () => unknown; sink: AuthorInputSink; unregister(): void; edit?: { from: number; to: number; inputType: string }; composition?: { tracker: DocumentNotesInputTracker; from: number; to: number } }
const samePath = (prefix: readonly ValuePath[], path: readonly ValuePath[]) => prefix.length <= path.length && prefix.every((part, at) => JSON.stringify(part) === JSON.stringify(path[at]));
/** One payload proof implementation. Only explicitly registered producers are observed. */
export class PersonalControlInput {
  private tracked = new WeakMap<TextControl, Tracked>();
  private retiredNative = new WeakSet<TextControl>();
  private fenced = new WeakSet<TextControl>();
  private bindingHistory = new WeakMap<TextControl, string>();
  private sessions = new Set<() => void>();
  constructor(private document: Document, private isCurrent: (context: AuthorInputContext) => boolean, private retain: AuthorInputSink) {
    document.addEventListener("beforeinput", event => {
      const state = this.active(event.target); if (!state) return;
      this.protectProgrammaticValue(state);
      if (state.control.selectionStart !== null && state.control.selectionEnd !== null) state.edit = { from: state.control.selectionStart, to: state.control.selectionEnd, inputType: (event as InputEvent).inputType };
    }, true);
    document.addEventListener("input", event => {
      const state = this.active(event.target); if (!state) return;
      const input = event as InputEvent, data = input.data ?? input.dataTransfer?.getData("text/plain");
      if (state.composition) { state.tracker = state.composition.tracker.copy(); state.tracker.observe(state.control.value, { from: state.composition.from, to: state.composition.to, inputType: "insertCompositionText" }, data); }
      else if (state.edit || !state.inputObserved || state.lastValue !== state.control.value) state.tracker.observe(state.control.value, state.edit, input.inputType?.startsWith("insert") ? data : undefined);
      state.inputObserved = true; state.lastValue = state.control.value; state.edit = undefined; state.slot.revision++; this.publish(state);
    }, true);
    document.addEventListener("compositionstart", event => {
      const state = this.active(event.target); if (!state || state.control.selectionStart === null || state.control.selectionEnd === null) return;
      this.protectProgrammaticValue(state); state.composition = { tracker: state.tracker.copy(), from: state.control.selectionStart, to: state.control.selectionEnd }; state.slot.revision++;
    }, true);
    document.addEventListener("compositionend", event => {
      const state = this.active(event.target); if (!state?.composition) return;
      state.tracker = state.composition.tracker.copy(); state.tracker.observe(state.control.value, { from: state.composition.from, to: state.composition.to, inputType: "insertCompositionText" }, (event as CompositionEvent).data);
      state.inputObserved = true; state.lastValue = state.control.value; state.composition = undefined; state.edit = undefined; state.slot.revision++; this.publish(state);
    }, true);
  }
  session(audience: string, epoch: number, owner: AuthorInputOwner, current: () => boolean, suppliedSink?: AuthorInputSink, leaseRevision?: () => unknown): AuthorInputSession {
    if (owner.kind === "capture" && !suppliedSink) throw new Error("捕获输入必须使用自己的 Core 草稿，不能借正文恢复域。");
    const sink = suppliedSink ?? this.retain, slots = new Map<string, Slot>(); let live = true;
    const freeze = <T>(value: T): T => { if (value && typeof value === "object") { for (const child of Object.values(value)) freeze(child); Object.freeze(value); } return value; };
    const capturedOwner = freeze(structuredClone(owner));
    const retireState = (state: Tracked) => { if (state.composition || state.edit) this.retiredNative.add(state.control); if (this.tracked.get(state.control) === state) this.tracked.delete(state.control); state.unregister(); };
    const retire = () => { if (!live) return; live = false; for (const slot of slots.values()) if (slot.state) retireState(slot.state); this.sessions.delete(retire); };
    this.sessions.add(retire);
    return {
      bind: (control, slotName, path = []) => {
        if (!live) throw new Error("作者输入目标已退休，不能把旧控件绑到新目标。");
        const previousControl = this.tracked.get(control);
        if (previousControl?.composition || previousControl?.edit) { retireState(previousControl); throw new Error("原生输入尚未落定，请使用新控件承接新的作者目标。"); }
        if (this.retiredNative.has(control)) throw new Error("此控件仍可能收到旧组合终止事件，新的作者目标需要新控件。");
        const key = JSON.stringify([slotName, path]); let slot = slots.get(key); if (!slot) { slot = { id: personalInputId(), revision: 0, slot: slotName, path: freeze(structuredClone(path)) }; slots.set(key, slot); }
        const history = this.bindingHistory.get(control);
        if (history && history !== slot.id) { if (previousControl) retireState(previousControl); this.retiredNative.add(control); throw new Error("作者目标或绑定代次已改变，新目标必须使用新控件，避免迟到原生终止输入。"); }
        this.bindingHistory.set(control, slot.id);
        const old = slot.state;
        if (previousControl && previousControl !== old) retireState(previousControl);
        if (old) retireState(old);
        const sameValue = old?.lastValue === control.value;
        const state: Tracked = { token: {}, control, context: freeze({ audience, epoch, owner: capturedOwner, slot: slotName, path: slot.path }), slot, tracker: sameValue ? old!.tracker.copy() : new DocumentNotesInputTracker(control.value), lastValue: control.value, inputObserved: false, fallback: old ? [...old.fallback, ...(!sameValue && old.tracker.recoveryInput() ? [old.tracker.recoveryInput()] : [])] : [], current: () => live && current(), leaseRevision, sink, unregister() {} };
        slot.state = state; this.tracked.set(control, state); state.unregister = registerAuthorInputLease(control, () => this.capture(state));
        // A detached/retired node may still receive its terminal native event.
        // Fence its producer's target handlers as well as recovery publication.
        // This applies only to this positively registered control; valid program
        // restoration on a current disabled control may still update its model.
        if (!this.fenced.has(control) && control.addEventListener) {
          this.fenced.add(control);
          const guard = (event: Event) => { const current = this.tracked.get(control); if (!control.isConnected || this.retiredNative.has(control) || !current?.current() || !this.isCurrent(current.context)) event.stopImmediatePropagation(); };
          for (const type of ["beforeinput", "input", "compositionstart", "compositionupdate", "compositionend", "change"]) control.addEventListener(type, guard, true);
        }
        return () => { if (this.tracked.get(control) === state) retireState(state); };
      },
      invalidate: (name, path = []) => { for (const [key, slot] of slots) if (slot.slot === name && samePath(path, slot.path)) { if (slot.state) retireState(slot.state); slots.delete(key); } },
      capture: control => { const state = this.active(control); return state && state.current() && state.context.owner === capturedOwner ? this.capture(state) : undefined; },
      retire,
    };
  }
  clear(): void { for (const retire of [...this.sessions]) retire(); }
  private active(target: EventTarget | null): Tracked | undefined {
    if (!(target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) || !target.isConnected || target.readOnly || target.disabled || target.matches?.(":disabled") || target.dataset.personalInputManaged || target.id === "token" || target instanceof HTMLInputElement && ["password", "hidden", "file", "checkbox", "radio", "button", "submit", "search"].includes(target.type) || this.retiredNative.has(target)) return;
    const state = this.tracked.get(target); return state && state.current() && this.isCurrent(state.context) ? state : undefined;
  }
  private protectProgrammaticValue(state: Tracked): void {
    if (state.control.value === state.lastValue) return;
    const known = state.tracker.recoveryInput(); if (known) state.fallback.push(known);
    state.tracker = new DocumentNotesInputTracker(state.control.value); state.lastValue = state.control.value; state.edit = undefined; state.composition = undefined; state.slot.revision++;
  }
  private publish(state: Tracked): void { if (this.active(state.control) === state) state.sink({ context: state.context, id: state.slot.id, revision: state.slot.revision, text: [...state.fallback, state.tracker.recoveryInput()].filter(Boolean).join("\n"), rawValue: state.control.value }); }
  private capture(state: Tracked): AuthorInputLease | undefined {
    if (this.active(state.control) !== state || state.composition || state.edit) return;
    const token = state.token, revision = state.slot.revision, value = state.control.value, producerRevision = state.leaseRevision?.();
    return { apply: (text, assign = next => { state.control.value = next; }) => {
      if (this.active(state.control)?.token !== token || state.slot.revision !== revision || state.control.value !== value || !Object.is(state.leaseRevision?.(), producerRevision) || state.composition || state.edit) return false;
      const known = state.tracker.recoveryInput(); if (known) state.fallback.push(known); state.fallback.push(text);
      assign(text); state.tracker = new DocumentNotesInputTracker(state.control.value); state.lastValue = state.control.value; state.inputObserved = true; state.slot.revision++; this.publish(state); return true;
    } };
  }
}
