import { PERSONAL_COPY_ONLY_SLOT, readPersonalCopySeed } from "./personal-input-recovery";
import { DocumentNotesInputTracker } from "./personal-input-text";
import { personalInputId } from "./source-input-origin";
import { registerAuthorInputLease, type AuthorInputContext, type AuthorInputLease, type AuthorInputOwner, type AuthorInputSession, type AuthorInputSink, type AuthorProducerCheckpoint, type AuthorProducerSnapshot, type AuthorProducerState, type ValuePath } from "./author-input";

type TextControl = HTMLInputElement | HTMLTextAreaElement;
type ChoiceControl = HTMLInputElement | HTMLSelectElement;
interface Slot { id: string; revision: number; slot: string; path: readonly ValuePath[]; state?: Tracked }
interface Producer {
  id: string; audience: string; epoch: number; owner: AuthorInputOwner; revision: number; ackRevision: number; changedRevision: number;
  retired: boolean; retiredPending: boolean; frozen: boolean; slots: Map<string, Slot>; choices: Map<string, Choice>;
  leases: Set<{ state: Tracked }>; stopped: Map<TextControl | ChoiceControl, { readOnly?: boolean; disabled?: boolean }>;
}
interface Tracked { producer: Producer; token: object; control: TextControl; context: AuthorInputContext; slot: Slot; tracker: DocumentNotesInputTracker; lastValue: string; inputObserved: boolean; fallback: string[]; current(): boolean; leaseRevision?: () => unknown; sink: AuthorInputSink; unregister(): void; edit?: { from: number; to: number; inputType: string }; composition?: { tracker: DocumentNotesInputTracker; from: number; to: number } }
interface Choice { producer: Producer; token: object; id: string; revision: number; control: ChoiceControl; context: AuthorInputContext; current(): boolean; sink?: AuthorInputSink }
const samePath = (prefix: readonly ValuePath[], path: readonly ValuePath[]) => prefix.length <= path.length && prefix.every((part, at) => JSON.stringify(part) === JSON.stringify(path[at]));
const immutable = <T>(value: T): T => { if (value && typeof value === "object") { for (const child of Object.values(value)) immutable(child); Object.freeze(value); } return value; };

/** Positive owner registry: producer proof/state only, without another raw
 * recovery store, author fact ledger or Core close acknowledgement. */
export class PersonalControlInput {
  private tracked = new WeakMap<TextControl, Tracked>();
  private choices = new WeakMap<ChoiceControl, Choice>();
  private retiredNative = new WeakSet<Element>();
  private fenced = new WeakSet<Element>();
  private bindingHistory = new WeakMap<Element, string>();
  private producers = new Map<string, Producer>();
  private sessions = new Set<() => void>();
  private revision = 0;
  private frozen = false;
  constructor(private document: Document, private isCurrent: (context: AuthorInputContext) => boolean, private retain: AuthorInputSink) {
    document.addEventListener("beforeinput", event => {
      const state = this.active(event.target); if (!state) return;
      this.protectProgrammaticValue(state);
      if (state.control.selectionStart !== null && state.control.selectionEnd !== null) {
        const edit = { from: state.control.selectionStart, to: state.control.selectionEnd, inputType: (event as InputEvent).inputType };
        state.edit = edit; this.touch(state.producer, false);
        queueMicrotask(() => { if (event.defaultPrevented && state.edit === edit) { state.edit = undefined; this.revision++; } });
      }
    }, true);
    document.addEventListener("input", event => {
      const state = this.active(event.target); if (!state) return;
      const input = event as InputEvent, data = input.data ?? input.dataTransfer?.getData("text/plain");
      if (state.composition) { state.tracker = state.composition.tracker.copy(); state.tracker.observe(state.control.value, { from: state.composition.from, to: state.composition.to, inputType: "insertCompositionText" }, data); }
      else if (state.edit || !state.inputObserved || state.lastValue !== state.control.value) state.tracker.observe(state.control.value, state.edit, input.inputType?.startsWith("insert") ? data : undefined);
      state.inputObserved = true; state.lastValue = state.control.value; state.edit = undefined; state.slot.revision++; this.touch(state.producer, true); this.publish(state);
    }, true);
    document.addEventListener("compositionstart", event => {
      const state = this.active(event.target); if (!state || state.control.selectionStart === null || state.control.selectionEnd === null) return;
      this.protectProgrammaticValue(state); state.composition = { tracker: state.tracker.copy(), from: state.control.selectionStart, to: state.control.selectionEnd }; state.slot.revision++; this.touch(state.producer, false);
    }, true);
    document.addEventListener("compositionend", event => {
      const state = this.active(event.target); if (!state?.composition) return;
      state.tracker = state.composition.tracker.copy(); state.tracker.observe(state.control.value, { from: state.composition.from, to: state.composition.to, inputType: "insertCompositionText" }, (event as CompositionEvent).data);
      state.inputObserved = true; state.lastValue = state.control.value; state.composition = undefined; state.edit = undefined; state.slot.revision++; this.touch(state.producer, true); this.publish(state);
    }, true);
    document.addEventListener("change", event => {
      const control = event.target as ChoiceControl | null, state = control && this.choices.get(control);
      if (!state || !this.choiceActive(state)) return;
      state.revision++; this.touch(state.producer, true);
      // Generic text recovery never copies protected initial option values.
      state.sink?.({ kind: "choice", context: state.context, id: state.id, revision: state.revision, producerId: state.producer.id, producerRevision: state.producer.revision, text: "", rawValue: control instanceof HTMLInputElement ? String(control.checked) : control.value });
    }, true);
  }
  private touch(producer: Producer, observed: boolean): void { producer.revision++; if (observed) producer.changedRevision = producer.revision; this.revision++; }
  private state(producer: Producer): AuthorProducerState {
    return { producerId: producer.id, audience: producer.audience, epoch: producer.epoch, owner: structuredClone(producer.owner), revision: producer.revision, ackRevision: producer.ackRevision, retired: producer.retired, nativePending: producer.retiredPending || [...producer.slots.values()].some(slot => !!slot.state?.edit || !!slot.state?.composition), hasObservedChanges: producer.changedRevision > 0, unapplied: producer.changedRevision > producer.ackRevision, pendingDeliveries: producer.leases.size, frozen: producer.frozen };
  }
  inspect(): AuthorProducerSnapshot { return { collectorRevision: this.revision, producers: [...this.producers.values()].map(producer => this.state(producer)).sort((a, b) => a.producerId.localeCompare(b.producerId)) }; }
  /** The caller must represent the real owner's application or explicit
   * discard of this whole producer cut. Removing one recovery row is not that
   * action, and a transport ACK cannot call this method. */
  acknowledge(checkpoint: AuthorProducerCheckpoint, disposition: "applied" | "discarded"): boolean {
    const producer = this.producers.get(checkpoint.producerId);
    if (!producer || !Number.isSafeInteger(checkpoint.revision) || checkpoint.revision < producer.ackRevision || checkpoint.revision > producer.revision || disposition === "discarded" && checkpoint.revision !== producer.revision) return false;
    if (disposition === "applied" && producer.retiredPending) return false;
    if (disposition === "discarded") producer.retiredPending = false;
    producer.ackRevision = checkpoint.revision; this.revision++;
    const state = this.state(producer);
    if (producer.retired && !state.unapplied && !state.nativePending && !state.pendingDeliveries) this.producers.delete(producer.id);
    return true;
  }
  freeze(snapshot: AuthorProducerSnapshot): boolean {
    if (this.frozen || JSON.stringify(snapshot) !== JSON.stringify(this.inspect()) || snapshot.producers.some(producer => producer.nativePending || producer.pendingDeliveries)) return false;
    this.frozen = true;
    for (const producer of this.producers.values()) if (!this.freezeProducer(producer, { producerId: producer.id, revision: producer.revision })) { this.thaw(); return false; }
    this.revision++; return true;
  }
  thaw(): void { this.frozen = false; for (const producer of this.producers.values()) this.thawProducer(producer); this.revision++; }
  private freezeProducer(producer: Producer, checkpoint: AuthorProducerCheckpoint): boolean {
    const state = this.state(producer);
    if (producer.frozen || checkpoint.producerId !== producer.id || checkpoint.revision !== producer.revision || state.nativePending || state.pendingDeliveries) return false;
    producer.frozen = true;
    for (const slot of producer.slots.values()) if (slot.state && this.tracked.get(slot.state.control) === slot.state) { const control = slot.state.control; producer.stopped.set(control, { readOnly: control.readOnly }); control.readOnly = true; slot.state.token = {}; }
    for (const choice of producer.choices.values()) if (this.choices.get(choice.control) === choice) { producer.stopped.set(choice.control, { disabled: choice.control.disabled }); choice.control.disabled = true; choice.token = {}; }
    return true;
  }
  private thawProducer(producer: Producer): void {
    producer.frozen = false;
    for (const [control, old] of producer.stopped) {
      const text = this.tracked.get(control as TextControl), choice = this.choices.get(control as ChoiceControl);
      if (text?.producer === producer && text.current() && this.isCurrent(text.context) && old.readOnly !== undefined) (control as TextControl).readOnly = old.readOnly;
      if (choice?.producer === producer && choice.current() && this.isCurrent(choice.context) && old.disabled !== undefined) control.disabled = old.disabled;
    }
    producer.stopped.clear();
  }
  session(audience: string, epoch: number, owner: AuthorInputOwner, current: () => boolean, suppliedSink?: AuthorInputSink, leaseRevision?: () => unknown): AuthorInputSession {
    if (this.frozen) throw new Error("作者集合已冻结，不能加入新的 producer。");
    if (this.producers.size >= 512) throw new Error("未处置作者输入已达预算；原输入保留，不能静默退休。");
    if (owner.kind === "capture" && !suppliedSink) throw new Error("捕获输入必须使用自己的 Core 草稿，不能借正文恢复域。");
    const capturedOwner = immutable(structuredClone(owner));
    const producer: Producer = { id: crypto.randomUUID(), audience, epoch, owner: capturedOwner, revision: 0, ackRevision: 0, changedRevision: 0, retired: false, retiredPending: false, frozen: false, slots: new Map(), choices: new Map(), leases: new Set(), stopped: new Map() };
    this.producers.set(producer.id, producer); this.revision++;
    const sink = suppliedSink ?? this.retain, slots = producer.slots, live = () => !producer.retired && !producer.frozen && !this.frozen && current();
    const retireState = (state: Tracked) => {
      if (state.composition || state.edit) { this.retiredNative.add(state.control); producer.retiredPending = true; }
      if (this.tracked.get(state.control) === state) this.tracked.delete(state.control);
      for (const lease of producer.leases) if (lease.state === state) producer.leases.delete(lease);
      state.unregister(); this.revision++;
    };
    const collect = () => { const state = this.state(producer); if (producer.retired && !state.unapplied && !state.nativePending && !state.pendingDeliveries) this.producers.delete(producer.id); };
    const retire = () => {
      if (producer.retired) return;
      producer.retired = true;
      for (const slot of slots.values()) if (slot.state) retireState(slot.state);
      for (const choice of producer.choices.values()) { if (this.choices.get(choice.control) === choice) this.choices.delete(choice.control); this.retiredNative.add(choice.control); }
      // Only proof metadata survives retirement, never old DOM/Base/raw copies.
      slots.clear(); producer.choices.clear(); producer.stopped.clear(); producer.leases.clear(); this.sessions.delete(retire); this.revision++; collect();
    };
    this.sessions.add(retire);
    const assertBinding = (control: Element, id: string) => {
      if (!live()) throw new Error("作者输入目标已退休或冻结，不能把旧控件绑到新目标。");
      if (this.retiredNative.has(control)) throw new Error("此控件仍可能收到旧组合终止事件，新的作者目标需要新控件。");
      const history = this.bindingHistory.get(control);
      if (history && history !== id) { this.retiredNative.add(control); throw new Error("作者目标或绑定代次已改变，新目标必须使用新控件，避免迟到原生终止输入。"); }
      this.bindingHistory.set(control, id);
    };
    const guard = (control: Element) => {
      if (this.fenced.has(control) || !control.addEventListener) return;
      this.fenced.add(control);
      const handler = (event: Event) => {
        const text = this.tracked.get(control as TextControl), choice = this.choices.get(control as ChoiceControl);
        if (!control.isConnected || this.retiredNative.has(control) || !(text?.current() && this.isCurrent(text.context) || choice?.current() && this.isCurrent(choice.context))) event.stopImmediatePropagation();
      };
      for (const type of ["beforeinput", "input", "compositionstart", "compositionupdate", "compositionend", "change"]) control.addEventListener(type, handler, true);
    };
    return {
      bind: (control, slotName, path = [], options) => {
        if (!live()) throw new Error("作者输入目标已退休或冻结，不能把旧控件绑到新目标。");
        const copy = options?.personalCopy ? readPersonalCopySeed(options.personalCopy, {
          audience, owner: capturedOwner, slot: slotName, path, value: control.value,
        }) : undefined;
        if ((slotName === PERSONAL_COPY_ONLY_SLOT) !== !!copy || copy && (!suppliedSink || owner.kind === "capture")) throw new Error("本人副本绑定缺少准确的副本 owner。");
        const previousControl = this.tracked.get(control);
        if (previousControl?.composition || previousControl?.edit) { retireState(previousControl); throw new Error("原生输入尚未落定，请使用新控件承接新的作者目标。"); }
        const key = JSON.stringify([slotName, path]); let slot = slots.get(key);
        if (copy && (previousControl || slot)) throw new Error("本人副本初始绑定需要新的控件与 producer slot。");
        if (!slot) { if (slots.size + producer.choices.size >= 1024) throw new Error("作者控件超过预算。"); slot = { id: copy?.id ?? personalInputId(), revision: copy?.revision ?? 0, slot: slotName, path: immutable(structuredClone(path)) }; slots.set(key, slot); }
        assertBinding(control, slot.id);
        const old = slot.state; if (previousControl && previousControl !== old) retireState(previousControl); if (old) retireState(old);
        const sameValue = old?.lastValue === control.value;
        const initial = copy ? new DocumentNotesInputTracker("") : undefined;
        if (initial) initial.input(copy!.text, { from: 0, to: 0, inputType: "insertText" });
        const state: Tracked = { producer, token: {}, control, context: immutable({ audience, epoch, owner: capturedOwner, slot: slotName, path: slot.path }), slot, tracker: initial ?? (sameValue ? old!.tracker.copy() : new DocumentNotesInputTracker(control.value)), lastValue: control.value, inputObserved: false, fallback: old ? [...old.fallback, ...(!sameValue && old.tracker.recoveryInput() ? [old.tracker.recoveryInput()] : [])] : [], current: live, leaseRevision, sink, unregister() {} };
        slot.state = state; this.tracked.set(control, state); state.unregister = registerAuthorInputLease(control, () => this.capture(state)); guard(control); this.revision++;
        return () => { if (this.tracked.get(control) === state) retireState(state); };
      },
      bindChoice: (control, slotName, path = []) => {
        if (control instanceof HTMLInputElement && !["checkbox", "radio"].includes(control.type)) throw new Error("选择绑定只接受实际 checkbox/radio/select。");
        const key = JSON.stringify([slotName, path]), old = producer.choices.get(key), id = old?.id ?? personalInputId(); assertBinding(control, id);
        if (old) this.choices.delete(old.control);
        const state: Choice = { producer, token: {}, id, revision: old?.revision ?? 0, control, context: immutable({ audience, epoch, owner: capturedOwner, slot: slotName, path: immutable(structuredClone(path)) }), current: live, sink: owner.kind === "capture" ? suppliedSink : undefined };
        producer.choices.set(key, state); this.choices.set(control, state); guard(control); this.revision++;
        return () => { if (this.choices.get(control) === state) { this.choices.delete(control); this.revision++; } };
      },
      recordAction: (slotName, path = []) => {
        const context = immutable({ audience, epoch, owner: capturedOwner, slot: slotName, path: immutable(structuredClone(path)) });
        if (!live() || !this.isCurrent(context)) throw new Error("作者输入目标已退休、冻结或不再获准，不能记录结构操作。");
        this.touch(producer, true);
        // Capture has its own typed draft sink. An action contains no proved
        // personal text and must never enter the generic text-recovery store.
        if (owner.kind === "capture") suppliedSink!({ kind: "choice", context, id: personalInputId(), revision: 1,
          text: "", rawValue: "", producerId: producer.id, producerRevision: producer.revision });
      },
      invalidate: (name, path = []) => {
        if (producer.frozen || this.frozen) throw new Error("作者集合已冻结。");
        for (const [key, slot] of slots) if (slot.slot === name && samePath(path, slot.path)) { if (slot.state) retireState(slot.state); slots.delete(key); }
        for (const [key, choice] of producer.choices) if (choice.context.slot === name && samePath(path, choice.context.path)) { this.choices.delete(choice.control); this.retiredNative.add(choice.control); producer.choices.delete(key); this.revision++; }
      },
      capture: control => { const state = this.active(control); return state?.producer === producer ? this.capture(state) : undefined; },
      inspect: () => this.state(producer),
      acknowledge: (checkpoint, disposition) => checkpoint.producerId === producer.id && this.acknowledge(checkpoint, disposition),
      freeze: checkpoint => { const result = this.freezeProducer(producer, checkpoint); if (result) this.revision++; return result; },
      thaw: () => { if (!this.frozen) { this.thawProducer(producer); this.revision++; } },
      retire,
    };
  }
  clear(): void { for (const retire of [...this.sessions]) retire(); }
  private active(target: EventTarget | null): Tracked | undefined {
    if (!(target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) || !target.isConnected || target.readOnly || target.disabled || target.matches?.(":disabled") || target.dataset.personalInputManaged || target.id === "token" || target instanceof HTMLInputElement && ["password", "hidden", "file", "checkbox", "radio", "button", "submit", "search"].includes(target.type) || this.retiredNative.has(target)) return;
    const state = this.tracked.get(target); return state && state.current() && this.isCurrent(state.context) ? state : undefined;
  }
  private choiceActive(state: Choice): boolean { return this.choices.get(state.control) === state && state.control.isConnected && !state.control.disabled && !this.retiredNative.has(state.control) && state.current() && this.isCurrent(state.context); }
  private protectProgrammaticValue(state: Tracked): void {
    if (state.control.value === state.lastValue) return;
    const known = state.tracker.recoveryInput(); if (known) state.fallback.push(known);
    state.tracker = new DocumentNotesInputTracker(state.control.value); state.lastValue = state.control.value; state.edit = undefined; state.composition = undefined; state.slot.revision++; this.revision++;
  }
  private publish(state: Tracked): void {
    if (this.active(state.control) === state) state.sink({ kind: "text", context: state.context, id: state.slot.id, revision: state.slot.revision, producerId: state.producer.id, producerRevision: state.producer.revision, text: [...state.fallback, state.tracker.recoveryInput()].filter(Boolean).join("\n"), rawValue: state.control.value });
  }
  private capture(state: Tracked): AuthorInputLease | undefined {
    if (this.active(state.control) !== state || state.composition || state.edit || state.producer.leases.size >= 256) return;
    const token = state.token, revision = state.slot.revision, value = state.control.value, producerRevision = state.leaseRevision?.(), lease = { state };
    let released = false; state.producer.leases.add(lease); this.revision++;
    const release = () => { if (!released) { released = true; state.producer.leases.delete(lease); this.revision++; } };
    return { release, apply: (text, assign = next => { state.control.value = next; }) => {
      if (released) return false;
      const valid = this.active(state.control)?.token === token && state.slot.revision === revision && state.control.value === value && Object.is(state.leaseRevision?.(), producerRevision) && !state.composition && !state.edit && state.producer.leases.has(lease);
      release(); if (!valid) return false;
      const known = state.tracker.recoveryInput(); if (known) state.fallback.push(known); state.fallback.push(text);
      assign(text); state.tracker = new DocumentNotesInputTracker(state.control.value); state.lastValue = state.control.value; state.inputObserved = true; state.slot.revision++; this.touch(state.producer, true); this.publish(state); return true;
    } };
  }
}
