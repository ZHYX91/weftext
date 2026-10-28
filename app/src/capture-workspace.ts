import { mountLocalCapture, type CaptureMountPorts, type MountedLocalCapture } from "./capture-mount";
import { CapturePersistence, emptyCaptureInput, type CaptureDraftRecord } from "./capture-draft-state";
import { captureDraftPointerKey } from "./capture-local";
import { NativeCapture } from "./native-capture";
import type { NativeSurface, CloseParticipant } from "./native-host";
import { action, ui } from "./native-value-form";
import { disposeUiTree, errorUiText, message, replaceUiChildren, setUiAttr, setUiText, uiTextValue, UiError } from "./ui-locale";

// These IDs locate one empty Core create command; no author input, capability,
// target or result is stored here. Only Core decides whether it was applied.
const newDraftPointerKey = "weftext:capture:new-draft";
interface NewDraftPointer { version: 1; draftId: string; mutationId: string; deviceId: string; wireDigest: string }
const uuid = /^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/;

/** The local Capture window's one current mount, not a second draft ledger. */
export class LocalCaptureWorkspace {
  private mounted?: MountedLocalCapture;
  private native?: NativeCapture;
  private live = true;
  private switching = false;
  private closing = false;
  private closeAttempt = 0;
  private generation = 0;
  private deviceId?: string;
  private releaseParticipant?: () => void;
  private readonly core: CapturePersistence;
  private readonly navigation = ui("section", "capture-drafts");
  private readonly drafts = ui("select", "data-input");
  private readonly diagnostic = ui("p", "data-warning");
  readonly participant: CloseParticipant;

  // The optional mount seam substitutes rendering in joint lifecycle tests;
  // production always uses the real form/inputs/guard mount below.
  constructor(private readonly host: HTMLElement, private readonly ports: CaptureMountPorts, private readonly surface?: NativeSurface, private readonly mount = mountLocalCapture) {
    this.core = new CapturePersistence(ports.request);
    this.participant = {
      prepare: async request => {
        if(!this.live || this.switching || this.closing || !this.native) throw new UiError(message("desktop.unsavedClose"));
        this.closing = true; // Synchronous: handoff cannot enter during the await.
        const attempt = ++this.closeAttempt;
        const native = this.native, generation = this.generation;
        try {
          const proof = await native.participant.prepare(request);
          if(!this.live || this.switching || generation !== this.generation || attempt !== this.closeAttempt || native !== this.native) throw new UiError(message("desktop.unsavedClose"));
          return proof;
        } catch(error) { if(generation === this.generation && attempt === this.closeAttempt) this.closing = false; throw error; }
      },
      freeze: request => !this.switching && this.live && this.closing && !!this.native?.participant.freeze(request),
      thaw: () => {
        // A refused close can be cancelled while handoff owns A's frozen cut.
        // Its thaw must not release the independent handoff barrier.
        if(this.switching)return;
        this.closeAttempt++; this.closing = false; this.native?.participant.thaw();
      },
    };
    setUiAttr(this.drafts, "aria-label", message("capture.savedDrafts"));
    this.navigation.append(ui("p", "data-help", message("capture.newDraftHelp")), this.drafts,
      action(message("capture.restoreDraft"), () => this.run(() => this.restore(this.drafts.value))),
      action(message("capture.refreshDrafts"), () => this.run(() => this.refreshDrafts())),
      action(message("capture.startOtherDraft"), () => this.run(() => this.startOtherDraft())), this.diagnostic);
    host.before(this.navigation);
  }
  private async run(action: () => Promise<void>): Promise<void> {
    try { await action(); if(this.live)setUiText(this.diagnostic, ""); }
    catch(error) { if(this.live)setUiText(this.diagnostic, errorUiText(error)); }
  }
  async open(): Promise<void> {
    this.deviceId = await this.currentDevice();
    await this.publish(undefined);
    try { await this.refreshDrafts(); }
    catch(error) { setUiText(this.diagnostic, errorUiText(error)); }
    if(this.surface) { this.releaseParticipant = this.surface.register("capture", this.participant); this.surface.markReady(); }
  }
  async refreshDrafts(): Promise<void> {
    const generation = this.generation;
    const records = await this.ports.request<CaptureDraftRecord[]>("/api/capture/draft/list", {});
    if(!this.live || generation !== this.generation)return;
    const options = records.map(record => {
      const option = ui("option"); option.value = record.draftId;
      // Actual root and opaque draft ID, never translated author content.
      const labelRoot=record.target?.rootHint,marker=record.draftId.slice(0,8);
      setUiText(option, () => `${uiTextValue(message("capture.retainedDraft"))} · ${labelRoot ?? uiTextValue(message("capture.unselectedTarget"))} · ${marker}`);
      return option;
    });
    replaceUiChildren(this.drafts, ...options);
    const current = this.mounted?.session.controller.state.draft.draftId;
    if(current)this.drafts.value = current;
  }
  private async currentDevice(): Promise<string> {
    const device = await this.ports.request<{ id: string; isCurrent: boolean }>("/api/capture/identity", {});
    if(!device || typeof device !== "object" || Array.isArray(device) || Object.keys(device).sort().join(",") !== "id,isCurrent" || device.isCurrent !== true || typeof device.id !== "string" || !uuid.test(device.id) || this.deviceId && this.deviceId !== device.id)throw new UiError(message("capture.rawRetained"));
    return device.id;
  }
  private emptyCreate(value: Pick<NewDraftPointer, "draftId" | "mutationId">) {
    const input = emptyCaptureInput(); input.bodyMode = "text";
    return { draftId: value.draftId, mutationId: value.mutationId, input };
  }
  private async createDigest(value: Pick<NewDraftPointer, "draftId" | "mutationId">): Promise<string> {
    const bytes = new TextEncoder().encode(JSON.stringify(this.emptyCreate(value)));
    return [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))].map(byte => byte.toString(16).padStart(2,"0")).join("");
  }
  private reflectCurrentDraft(): void {
    const record = this.mounted?.session.controller.state.draft;
    if(!record)return;
    if(![...this.drafts.options].some(option => option.value === record.draftId)) {
      const option = ui("option"); option.value = record.draftId;
      const labelRoot=record.target?.rootHint,marker=record.draftId.slice(0,8);
      setUiText(option, () => `${uiTextValue(message("capture.retainedDraft"))} · ${labelRoot ?? uiTextValue(message("capture.unselectedTarget"))} · ${marker}`);
      this.drafts.append(option);
    }
    this.drafts.value = record.draftId;
  }
  private pendingNew(deviceId: string): NewDraftPointer | undefined {
    const raw = this.ports.options?.pointerStorage?.getItem(newDraftPointerKey);
    if(!raw)return;
    const value = JSON.parse(raw) as NewDraftPointer;
    if(!value || Object.keys(value).sort().join(",") !== "deviceId,draftId,mutationId,version,wireDigest" || value.version !== 1 || typeof value.draftId !== "string" || !uuid.test(value.draftId) || typeof value.mutationId !== "string" || !uuid.test(value.mutationId) || typeof value.deviceId !== "string" || value.deviceId !== deviceId || typeof value.wireDigest !== "string" || !/^[a-f0-9]{64}$/.test(value.wireDigest)) throw new UiError(message("capture.rawRetained"));
    return value;
  }
  private persistNew(value: NewDraftPointer): void {
    const storage = this.ports.options?.pointerStorage;
    if(!storage)throw new UiError(message("captureDraftState.persistenceUnconfirmed"));
    const wire = JSON.stringify(value);
    storage.setItem(newDraftPointerKey, wire);
    if(storage.getItem(newDraftPointerKey) !== wire)throw new UiError(message("captureDraftState.persistenceUnconfirmed"));
  }
  async startOtherDraft(): Promise<void> {
    return this.handoff(async () => {
      // Allocate and durably retain IDs BEFORE create can be dispatched.
      const deviceId = await this.currentDevice();
      let original = this.pendingNew(deviceId);
      if(!original) {
        const ids = { draftId: crypto.randomUUID(), mutationId: crypto.randomUUID() };
        original = { version: 1, ...ids, deviceId, wireDigest: await this.createDigest(ids) };
      }
      this.persistNew(original);
      let record: CaptureDraftRecord;
      try { record = await this.core.read(original.draftId); }
      catch {
        // A failed read is not evidence of absence. Reissuing the IDENTICAL
        // empty create under the SAME IDs relies on Core's original mutation
        // reconciliation; it never creates a replacement draft or a note.
        // A different empty-input encoding after an upgrade cannot replay
        // an invented wire under the old mutation. An existing draft above
        // remains readable; an unavailable one stays explicitly unresolved.
        if(original.wireDigest !== await this.createDigest(original))throw new UiError(message("captureDraftState.persistenceUnconfirmed"));
        record = await this.core.mutate("create", this.emptyCreate(original));
      }
      await this.publish(record.draftId);
      // After a mount failure this line is not reached: retry/reopen uses the
      // same retained draft ID even if its create reply was lost.
      this.ports.options?.pointerStorage?.setItem(newDraftPointerKey, "");
    });
  }
  async restore(draftId: string): Promise<void> {
    if(typeof draftId !== "string" || !uuid.test(draftId))throw new UiError(message("capture.targetRequired"));
    if(this.mounted?.session.controller.state.draft.draftId === draftId)return;
    return this.handoff(async () => { await this.core.read(draftId); await this.publish(draftId); });
  }
  private async handoff(run: () => Promise<void>): Promise<void> {
    if(!this.live || this.switching || this.closing)throw new UiError(message("desktop.unsavedClose"));
    this.native?.assertHandoffReady();
    const old = this.mounted;
    old?.assertCloseReady();
    if(old?.session.controller.state.draft.pending)throw new UiError(message("capture.pending"));
    this.switching = true; // Before the FIRST await, inverse close race is shut.
    this.navigation.inert = true;
    let frozen = false;
    try {
      if(old) {
        old.form.collectOwnedRaw();
        await old.session.settleForHandoff();
        const proof = await old.guard.prepare();
        if(!old.guard.freeze(proof))throw new UiError(message("desktop.unsavedClose"));
        frozen = true;
        // No more input may stage after this barrier; drain browser staging too.
        await old.session.settleForHandoff();
      }
      await run();
    } finally {
      if(frozen && old && old === this.mounted)old.guard.thaw();
      this.switching = false; this.navigation.inert = false;
    }
    await this.refreshDrafts();
  }
  private async publish(draftId: string | undefined): Promise<void> {
    const container = this.host.ownerDocument.createElement("section");
    let next: MountedLocalCapture | undefined;
    let ownerNative: NativeCapture | undefined;
    try {
      next = await this.mount(container, { ...this.ports, options: { ...this.ports.options, ...(draftId ? { draftId, pointerStorage: undefined } : {}) },
        deliverCommitted: this.surface ? original => { if(!ownerNative)throw new UiError(message("capture.pending")); return ownerNative.open(original); } : this.ports.deliverCommitted,
        requestClose: this.surface ? () => { if(!ownerNative)throw new UiError(message("desktop.unsavedClose")); return ownerNative.requestClose(); } : this.ports.requestClose });
      if(!this.live)throw new UiError(message("desktop.unsavedClose"));
      // Commit the mount as a unit; the stable participant delegates to exactly
      // this NativeCapture and guard. It can never return the prior A proof.
      ownerNative = this.surface ? new NativeCapture(this.surface, next) : undefined;
      this.native?.dispose(); this.mounted?.dispose();
      this.generation++; this.mounted = next;
      this.native = ownerNative;
      replaceUiChildren(this.host, container);
      this.reflectCurrentDraft();
      this.ports.options?.pointerStorage?.setItem(captureDraftPointerKey, next.session.controller.state.draft.draftId);
    } catch(error) { if(next !== this.mounted) { ownerNative?.dispose(); next?.dispose(); } throw error; }
  }
  dispose(): void {
    if(!this.live)return;
    this.live = false; this.generation++; this.closeAttempt++;
    this.releaseParticipant?.(); this.native?.dispose(); this.mounted?.dispose();
    disposeUiTree(this.navigation); this.navigation.remove();
  }
}
