import type { AuthorInputFactory } from "./author-input";
import { CaptureCloseGuard } from "./capture-close";
import { captureString, type CaptureInput } from "./capture-draft-state";
import { CaptureFields, type CaptureSchemaView } from "./capture-fields";
import { CaptureForm, type CaptureAdvancedControls } from "./capture-form";
import { HostedCaptureSession, type HostedCaptureOptions, type HostedCaptureRecord, type HostedCaptureRequest } from "./capture-hosted";
import { action, ui } from "./native-value-form";
import { PersonalControlInput } from "./personal-control-input";
import { decodeStructureCatalog, type StructureCatalog, type StructureReceipt, type StructureSlot } from "./hosted-structure-state";
import { disposeUiTree, errorUiText, message, replaceUiChildren, setUiAttr, setUiText, UiError } from "./ui-locale";

export interface HostedCaptureMountPorts {
  request: HostedCaptureRequest;
  options: HostedCaptureOptions;
  catalog: StructureCatalog;
  /** The containing page owns exactly one registry/real factory. */
  inputs:PersonalControlInput;
  authorInputs:AuthorInputFactory;
  /** Resolves only after the real library adopted this independently authorized result. */
  deliverCommitted(resource: string): Promise<void>;
  /** Called after Core flush and actual producer freeze; reject a refused close. */
  requestClose?(): Promise<void>;
}
export interface MountedHostedCapture {
  readonly session: HostedCaptureSession;
  readonly form: CaptureForm<HostedCaptureRecord>;
  readonly guard?: CaptureCloseGuard<HostedCaptureRecord>;
  retireAuthorization(): void;
  dispose(): void;
}

/** Hosted presentation consumes an authorized slot and the shared Capture forms.
 * A pure owner projection is displayed only for copying, never as a full form. */
export async function mountHostedCapture(host: HTMLElement, ports: HostedCaptureMountPorts): Promise<MountedHostedCapture> {
  const catalog = decodeStructureCatalog(ports.catalog);
  let live = true, authorizationRetired = false, protectedRetired = false, retiringProtected = false;
  let form: CaptureForm<HostedCaptureRecord> | undefined;
  let fields: CaptureFields<HostedCaptureRecord> | undefined;
  let guard: CaptureCloseGuard<HostedCaptureRecord> | undefined;
  let renderState: (state: ReturnType<HostedCaptureSession["snapshot"]>) => void = () => {};
  const current = () => live && !authorizationRetired && ports.options.current();
  const assertCurrent = () => { if (!current()) throw new UiError(message("capture.rawRetained")); };
  const session = await HostedCaptureSession.open(ports.request, {
    ...ports.options,
    current,
    changed: state => {
      if (!current()) return;
      renderState(state);
      ports.options.changed?.(state);
    },
  });
  assertCurrent();
  const draftId = session.controller.state.draft.draftId;
  const inputs=ports.inputs;
  const authorInputs:AuthorInputFactory=(owner,available,sink,revision)=>ports.authorInputs(owner,()=>current()&&!protectedRetired&&available(),sink,revision);
  const target = ui("section", "capture-target"), status = ui("p", "data-help"), original = ui("section", "capture-original"), recovery = ui("section", "capture-recovery"), diagnostic = ui("pre", "capture-diagnostic");
  status.setAttribute("aria-live", "polite");
  const slot = (): StructureSlot | undefined => {
    const active = session.controller.state.draft.active;
    return active ? catalog.slots.find(item => item.resource === active.slot) : undefined;
  };
  const full = () => current() && !protectedRetired && session.controller.state.draft.inputCut === "full" && !!session.controller.state.draft.active;
  const assertResolved = () => { assertCurrent(); if (session.unconfirmed.length) throw new UiError(message("capture.rawRetained")); };
  const canSaveNext = () => full() && slot()?.create === true && slot()?.status === "ready" && slot()?.namePolicy.kind === "uniqueChildren";
  const copy = (container: HTMLElement, text: string) => {
    const control = ui("textarea", "data-input"); control.readOnly = true; control.value = text;
    setUiAttr(control, "aria-label", message("capture.rawRetained")); container.append(control);
  };
  const showFact = (fact: StructureReceipt | null) => {
    replaceUiChildren(original);
    if (!fact) return;
    setUiText(status, fact.status === "committed" ? message("capture.saved") : message("capture.pending"));
    original.append(ui("code", "", fact.operationId), ui("p", "", fact.status));
  };
  const showError = (error: unknown) => setUiText(diagnostic, errorUiText(error));
  const run = (operation: () => Promise<void>) => async () => {
    try { assertCurrent(); await operation(); assertCurrent(); }
    catch (error) { if (current()) showError(error); }
  };
  function renderRecovery(): void {
    replaceUiChildren(recovery);
    if (!current()) return;
    const state = session.controller.state;
    if (!full()) {
      recovery.append(ui("h2", "", message("capture.restoreDraft")), ui("p", "data-help", message("capture.rawRetained")));
      for (const proofs of Object.values(state.input.authorTexts)) for (const proof of proofs) copy(recovery, captureString(proof.text));
      for (const raw of [...state.input.rawInputs, ...state.input.detachedRawInputs.map(entry => entry.input)]) if (raw.author) copy(recovery, captureString(raw.author.text));
      if (state.draft.pending) {
        recovery.append(ui("code", "", state.draft.pending.operationId));
        recovery.append(action(message("capture.confirmOriginal"), run(async () => { showFact(await session.confirmOriginal()); })));
        recovery.append(action(message("capture.openNote"), run(async () => {
          const fact = await session.confirmOriginal(); showFact(fact);
          if (fact?.status !== "committed" || !fact.result?.canOpen || fact.result.status !== "available") throw new UiError(message("capture.pending"));
          assertCurrent(); await ports.deliverCommitted(fact.result.resource); assertCurrent();
          // A copy-only view cannot advance the protected form. Keep the
          // original committed operation available for a later authorized open.
        })));
      }
    }
    for (const proposal of session.unconfirmed) {
      const section = ui("section"); section.append(ui("p", "data-help", message("capture.rawRetained")));
      if (proposal.pending) section.append(ui("code", "", proposal.pending.command.mutationId));
      for (const proofs of Object.values(proposal.input.texts)) for (const proof of proofs) copy(section, proof.text);
      for (const raw of [...proposal.input.rawInputs, ...proposal.input.detachedRawInputs.map(entry => entry.input)]) copy(section, raw.text);
      section.append(action(message("common.cancel"), run(async () => { await session.discardUnconfirmed(proposal); renderRecovery(); })));
      recovery.append(section);
    }
  }
  function retireProtected(): void {
    if (protectedRetired || retiringProtected) return;
    // Sampling precedes producer retirement. Only proved author fragments can
    // enter the transport outbox if the full Core command has lost its grant.
    retiringProtected = true;
    try { form?.retireAuthorization(); }
    catch (error) { if (current()) showError(error); }
    finally {
      protectedRetired = true; retiringProtected = false;
      if (form) form.element.hidden = true;
    }
  }
  function assertNewSubmission(): void {
    assertResolved();
    if (!full()) throw new UiError(message("capture.rawRetained"));
    const approved = slot(), input = session.controller.state.input;
    if (!approved?.create || approved.status !== "ready") throw new UiError(message("capture.targetRequired"));
    // This mounted slice edits ordinary/typed notes. Existing template effects
    // remain recoverable; no template/default consumer is fabricated here.
    if (input.templateId || captureString(input.name)) throw new UiError(message("capture.rawRetained"));
    fields?.assertSubmittable();
    if (!fields && (input.fields.length || input.typeIds.length || input.rawInputs.length || input.bodyMode !== "text")) throw new UiError(message("capture.rawRetained"));
  }
  const close = async () => {
    assertResolved();
    if (!guard || !full()) throw new UiError(message("desktop.unsavedClose"));
    const proof = await guard.prepare(); assertCurrent();
    if (!guard.freeze(proof)) throw new UiError(message("desktop.unsavedClose"));
    try {
      if (ports.requestClose) await ports.requestClose();
      else host.ownerDocument.defaultView?.close();
      if (current() && host.isConnected && host.ownerDocument.defaultView?.closed !== true) guard.thaw();
    } finally {
      // Embedded panel disposal must release the containing page's registry.
      if(host.ownerDocument.defaultView?.closed!==true)guard.thaw();
    }
  };
  let advancedHost: HTMLElement | undefined;
  form = new CaptureForm({
    controller: session.controller, authorInputs, target, showName: false,
    labels: { name: message("capture.name"), authorTitle: message("capture.authorTitle"), bodyMode: message("capture.bodyMode"), plain: message("capture.plain"), markup: message("capture.markup"), advanced: message("capture.advanced"), keepClose: message("capture.keepClose"), confirmOriginal: message("capture.confirmOriginal"), draftSaved: message("capture.draftSaved") },
    advanced: container => {
      advancedHost = container;
      const controls: CaptureAdvancedControls = {
        collectOwnedRaw: () => { if (!protectedRetired) fields?.collectOwnedRaw(); },
        update: (input: CaptureInput, consumed) => fields?.update(input, consumed),
        retireAuthorization: () => fields?.retireAuthorization(),
        dispose: () => fields?.dispose(),
      };
      return controls;
    },
    canOpen: () => current(), canSaveNext,
    submit: async mode => {
      assertCurrent();
      if (mode === "next" && !canSaveNext()) throw new UiError(message("capture.rawRetained"));
      if (!session.controller.state.draft.pending) assertNewSubmission();
      await session.save(mode, catalog, async resource => { assertCurrent(); await ports.deliverCommitted(resource); assertCurrent(); }, close);
    },
    confirmOriginal: async () => { assertCurrent(); showFact(await session.confirmOriginal()); },
    requestClose: close,
  });
  guard = new CaptureCloseGuard(session.controller, inputs, {
    assertResolved: () => { assertResolved(); if (!full()) throw new UiError(message("desktop.unsavedClose")); },
    collectOwnedRaw: () => form!.collectOwnedRaw(), freeze: () => { assertCurrent(); form!.freeze(); }, thaw: () => { if (current()) form!.thaw(); },
  });
  replaceUiChildren(host, form.element, original, recovery, status, diagnostic);
  const approved = slot();
  if (full() && approved) {
    target.append(ui("h2", "", message("capture.target")), ui("p", "", approved.title), ui("p", "", approved.parentTitle ?? approved.parent));
    const schema = approved.capture;
    if (approved.captureStatus !== "unavailable" && schema?.roleRef && /^[a-f0-9]{64}$/.test(schema.semanticDigest)) {
      fields = new CaptureFields(advancedHost!, session.controller, authorInputs, {
        schema, schemaRef: schema.roleRef, semanticDigest: schema.semanticDigest,
        allowedTypes: approved.types ?? [], allowedFields: schema.fields.map(field => field.id),
      } satisfies CaptureSchemaView);
    }
  } else retireProtected();
  renderState = state => {
    if (!state.draft.active || state.draft.inputCut !== "full") retireProtected();
    if (!protectedRetired && !retiringProtected) form!.update(state);
    renderRecovery();
  };
  renderRecovery();
  const unloading = (event: BeforeUnloadEvent) => {
    const state = session.controller.state;
    if (ports.options.recovery.pending() || state.persisting || state.blocked || session.unconfirmed.length || inputs.inspect().producers.some(producer => producer.nativePending || producer.pendingDeliveries || producer.unapplied)) event.preventDefault();
  };
  host.ownerDocument.defaultView?.addEventListener("beforeunload", unloading);
  return {
    session, form,
    get guard() { return full() ? guard : undefined; },
    retireAuthorization() {
      if (authorizationRetired || !live) return;
      try { form!.retireAuthorization(); }
      finally {
        authorizationRetired = protectedRetired = true;
        session.controller.retire();
        replaceUiChildren(host, ui("p", "data-help", message("capture.rawRetained")));
      }
    },
    dispose() {
      if (!live) return;
      live = false; form!.dispose(); session.controller.retire();
      host.ownerDocument.defaultView?.removeEventListener("beforeunload", unloading);
      disposeUiTree(host);
    },
  };
}
