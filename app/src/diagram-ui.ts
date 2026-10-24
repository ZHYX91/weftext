import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { action, ui, type DocumentBasis, type PanelHost } from "./data-panel";
import { DiagramController, diagramOccurrences, diagramPatch, type DiagramOccurrence } from "./diagram-state";
import { renderDiagramPng } from "./diagram-render";
import { createNativeProposalOwner } from "./native-proposal-owners";
import "./diagram.css";
/** Host supplies the same current-draft and permission checked patch route. */
export class DiagramPanel {
    private readonly dialogs = new Map<string, {
        dialog: HTMLDialogElement;
        clear(): void;
    }>();
    constructor(private readonly host: PanelHost, private readonly editable: () => boolean = () => true) { }
    clear(): void {
        for (const item of this.dialogs.values())
            item.clear();
        this.dialogs.clear();
    }
    async open(): Promise<void> {
        const opening = createNativeProposalOwner(uiMessage("hostedShell.mermaidDiagram"));
        const endOpening = opening.begin();
        try { await this.openPrepared(); }
        finally {
            endOpening();
            opening.setPending(false);
            opening.release();
        }
    }
    private async openPrepared(): Promise<void> {
        if (!this.editable()) {
            this.host.notify(uiMessage("diagramPanel.noteNotEditable"), true);
            return;
        }
        await this.host.prepareAction?.();
        const basis = this.host.getDocument();
        if (!basis || !this.editable()) {
            this.host.notify(uiMessage("diagramPanel.openEditableNoteFirst"), true);
            return;
        }
        const key = `${basis.sessionId}/${basis.serial}/${basis.baseId}`;
        const retained = this.dialogs.get(key);
        if (retained) {
            if (!retained.dialog.open)
                retained.dialog.showModal();
            return;
        }
        const item = this.create(basis, () => this.dialogs.delete(key));
        this.dialogs.set(key, item);
        document.body.append(item.dialog);
        item.dialog.showModal();
    }
    private create(basis: DocumentBasis, forget: () => void): {
        dialog: HTMLDialogElement;
        clear(): void;
    } {
        let disposed = false, active = 0, applying = false;
        const dialog = ui("dialog", "modal diagram-panel");
        const owner = createNativeProposalOwner(uiMessage("hostedShell.mermaidDiagram"), () => {
            if (!disposed && !dialog.open) dialog.showModal();
        });
        const run = async (operation: () => Promise<void>): Promise<void> => {
            const end = owner.begin();
            active++;
            try { await operation(); }
            finally {
                active--;
                end();
                if (disposed && active === 0) owner.release();
            }
        };
        const authorInputs = this.host.authorInputs?.(basis);
        const title = ui("h2", "", uiMessage("hostedShell.mermaidDiagram"));
        title.id = `diagram-${crypto.randomUUID()}`;
        dialog.setAttribute("aria-labelledby", title.id);
        const hide = (): void => {
            if (disposed || applying) return;
            owner.assertMutable();
            controller.cancel();
            draw();
            dialog.close();
        };
        const close = action(uiMessage("diagramPanel.closeKeepInput"), hide, "data-button compact");
        const discard = action(uiMessage("common.cancel"), () => {
            if (disposed || applying) return;
            owner.assertMutable();
            const checkpoint = authorInputs?.inspect();
            clear();
            if (checkpoint) authorInputs?.acknowledge(checkpoint, "discarded");
        }, "data-button compact");
        const header = ui("header", "diagram-header");
        header.append(title, close, discard);
        let source = ui("textarea", "data-input diagram-source");
        source.rows = 10;
        source.spellcheck = false;
        setUiAttr(source, "aria-label", uiMessage("diagramPanel.mermaidSource"));
        let width = ui("input", "data-input");
        width.type = "number";
        width.min = "320";
        width.max = "2048";
        width.step = "1";
        width.value = "1200";
        setUiAttr(width, "aria-label", uiMessage("diagramPanel.pngWidth"));
        const choose = ui("select", "data-input");
        setUiAttr(choose, "aria-label", uiMessage("diagramPanel.diagramLocation"));
        const occurrences = diagramOccurrences(basis.source);
        const fresh = ui("option", "", uiMessage("diagramPanel.newDiagramAtEnd"));
        fresh.value = "new";
        choose.append(fresh);
        occurrences.forEach((item, index) => { const option = ui("option", "", displayMessage("diagramPanel.diagramChoice", { p0: index + 1, p1: uiDisplayParameter(item.source.split("\n")[0]), p2: uiDisplayParameter(item.stale ? uiMessage("diagramPanel.sourceChangedSuffix") : "") })); option.value = String(index); choose.append(option); });
        let selected: DiagramOccurrence | undefined;
        let selectionKey = "new";
        const inputs = new Map<string, {
            source: string;
            width: string;
        }>();
        const controller = new DiagramController(renderDiagramPng);
        source.value = controller.source;
        const preview = ui("div", "diagram-preview");
        const status = ui("p", "data-help");
        status.setAttribute("role", "status");
        status.setAttribute("aria-live", "polite");
        let imageUrl: string | null = null;
        const generate = action(uiMessage("diagramPanel.generatePngPreview"), () => run(async () => {
            if (disposed || applying) return;
            status.className = "data-help";
            setUiText(status, uiMessage("diagramPanel.generatingOffline"));
            const pending = controller.render();
            draw();
            try {
                if (await pending && !disposed)
                    setUiText(status, displayMessage("diagramPanel.pngGeneratedSummary", { p0: controller.image!.width, p1: controller.image!.height }));
            }
            catch (error) {
                if (disposed) return;
                status.className = "data-warning";
                setUiText(status, errorUiText(error));
            }
            finally {
                draw();
            }
        }), "data-button");
        const commit = action(uiMessage("diagramPanel.confirmImageWriteDraft"), () => run(async () => {
            if (!controller.image || disposed || applying)
                return;
            applying = true;
            draw();
            const checkpoint = authorInputs?.inspect();
            try {
                if (checkpoint?.nativePending || checkpoint?.pendingDeliveries)
                    throw new UiError(uiMessage("shell.closeAuthorInputPending"));
                const current = this.host.getDocument();
                if (!this.editable() || !current || current.sessionId !== basis.sessionId || current.serial !== basis.serial || current.baseId !== basis.baseId || current.source !== basis.source)
                    throw new UiError(uiMessage("diagramPanel.noteOrPermissionChanged"));
                const patch = diagramPatch(basis, controller.image, selected);
                await this.host.applyPatch(patch, basis);
                owner.assertMutable();
                if (disposed)
                    return;
                inputs.delete(selectionKey);
                // Other locations can contain retained, unapplied source.
                // Applying this location must not acknowledge those inputs.
                if (inputs.size === 0 && checkpoint) authorInputs?.acknowledge(checkpoint, "applied");
                controller.cancel();
                dialog.close();
                this.host.notify(uiMessage("diagramPanel.diagramWrittenToDraft"));
                if (inputs.size === 0) clear();
            }
            catch (error) {
                if (disposed)
                    return;
                status.className = "data-warning";
                setUiText(status, errorUiText(error));
            }
            finally { applying = false; draw(); }
        }), "data-button primary");
        function draw(): void {
            if (disposed)
                return;
            generate.disabled = applying || controller.busy;
            commit.disabled = applying || controller.busy || !controller.image;
            source.disabled = width.disabled = choose.disabled = close.disabled = discard.disabled = applying;
            preview.replaceChildren();
            if (imageUrl) {
                URL.revokeObjectURL(imageUrl);
                imageUrl = null;
            }
            if (controller.image) {
                imageUrl = URL.createObjectURL(new Blob([new Uint8Array(controller.image.png)], { type: "image/png" }));
                const image = ui("img");
                image.src = imageUrl;
                setUiAttr(image, "alt", uiMessage("diagramPanel.pngPreviewAlt"));
                preview.append(image);
            }
        }
        const changed = (): void => { if (disposed || applying) return; owner.assertMutable(); controller.edit(source.value, Number(width.value)); draw(); setUiText(status, uiMessage("diagramPanel.sourceOrSizeChanged")); };
        authorInputs?.bind(source, `diagram:${selectionKey}:source`);
        authorInputs?.bind(width, `diagram:${selectionKey}:width`);
        source.oninput = changed;
        width.oninput = changed;
        choose.onchange = () => { if (disposed || applying) return; owner.assertMutable(); inputs.set(selectionKey, { source: source.value, width: width.value }); authorInputs?.invalidate(`diagram:${selectionKey}:source`); authorInputs?.invalidate(`diagram:${selectionKey}:width`); const replacement = source.cloneNode(false) as HTMLTextAreaElement; source.replaceWith(replacement); source = replacement; source.oninput = changed; const newWidth = width.cloneNode(false) as HTMLInputElement; width.replaceWith(newWidth); width = newWidth; width.oninput = changed; selectionKey = choose.value; selected = selectionKey === "new" ? undefined : occurrences[Number(selectionKey)]; const retained = inputs.get(selectionKey); source.value = retained?.source ?? selected?.source ?? "flowchart LR\n  A[开始] --> B[完成]"; width.value = retained?.width ?? "1200"; authorInputs?.bind(source, `diagram:${selectionKey}:source`); authorInputs?.bind(width, `diagram:${selectionKey}:width`); changed(); };
        const controls = ui("div", "diagram-actions");
        controls.append(generate, commit);
        dialog.append(header, ui("p", "data-help", uiMessage("diagramPanel.sourceAndPngDescription")), choose, source, ui("label", "data-label", uiMessage("diagramPanel.pngWidthRange")), width, preview, status, controls);
        dialog.addEventListener("cancel", event => { event.preventDefault(); hide(); });
        const clear = (): void => {
                if (disposed) return;
                owner.assertMutable();
                disposed = true;
                authorInputs?.retire();
                controller.edit("");
                controller.cancel();
                source.value = "";
                setUiText(status, "");
                preview.replaceChildren();
                choose.replaceChildren();
                occurrences.length = 0;
                inputs.clear();
                selected = undefined;
                if (imageUrl)
                    URL.revokeObjectURL(imageUrl);
                imageUrl = null;
                dialog.close();
                dialog.remove();
                forget();
                owner.setPending(false);
                if (active === 0) owner.release();
            };
        return { dialog, clear };
    }
}
