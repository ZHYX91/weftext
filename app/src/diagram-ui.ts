import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { action, ui, type DocumentBasis, type PanelHost } from "./data-panel";
import { DiagramController, diagramOccurrences, diagramPatch, type DiagramOccurrence } from "./diagram-state";
import { renderDiagramPng } from "./diagram-render";
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
        const item = this.create(basis);
        this.dialogs.set(key, item);
        document.body.append(item.dialog);
        item.dialog.showModal();
    }
    private create(basis: DocumentBasis): {
        dialog: HTMLDialogElement;
        clear(): void;
    } {
        let disposed = false;
        const authorInputs = this.host.authorInputs?.(basis);
        const dialog = ui("dialog", "modal diagram-panel");
        const title = ui("h2", "", uiMessage("hostedShell.mermaidDiagram"));
        title.id = `diagram-${crypto.randomUUID()}`;
        dialog.setAttribute("aria-labelledby", title.id);
        const close = action(uiMessage("diagramPanel.closeKeepInput"), () => { controller.cancel(); draw(); dialog.close(); }, "data-button compact");
        const header = ui("header", "diagram-header");
        header.append(title, close);
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
        const generate = action(uiMessage("diagramPanel.generatePngPreview"), async () => {
            status.className = "data-help";
            setUiText(status, uiMessage("diagramPanel.generatingOffline"));
            const pending = controller.render();
            draw();
            try {
                if (await pending)
                    setUiText(status, displayMessage("diagramPanel.pngGeneratedSummary", { p0: controller.image!.width, p1: controller.image!.height }));
            }
            catch (error) {
                status.className = "data-warning";
                setUiText(status, errorUiText(error));
            }
            finally {
                draw();
            }
        }, "data-button");
        const commit = action(uiMessage("diagramPanel.confirmImageWriteDraft"), async () => {
            if (!controller.image || disposed)
                return;
            commit.disabled = true;
            try {
                const current = this.host.getDocument();
                if (!this.editable() || !current || current.sessionId !== basis.sessionId || current.serial !== basis.serial || current.baseId !== basis.baseId || current.source !== basis.source)
                    throw new UiError(uiMessage("diagramPanel.noteOrPermissionChanged"));
                const patch = diagramPatch(basis, controller.image, selected);
                await this.host.applyPatch(patch, basis);
                if (disposed)
                    return;
                controller.cancel();
                draw();
                dialog.close();
                this.host.notify(uiMessage("diagramPanel.diagramWrittenToDraft"));
            }
            catch (error) {
                if (disposed)
                    return;
                status.className = "data-warning";
                setUiText(status, errorUiText(error));
                draw();
            }
        }, "data-button primary");
        function draw(): void {
            if (disposed)
                return;
            generate.disabled = controller.busy;
            commit.disabled = controller.busy || !controller.image;
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
        const changed = (): void => { controller.edit(source.value, Number(width.value)); draw(); setUiText(status, uiMessage("diagramPanel.sourceOrSizeChanged")); };
        authorInputs?.bind(source, `diagram:${selectionKey}:source`);
        authorInputs?.bind(width, `diagram:${selectionKey}:width`);
        source.oninput = changed;
        width.oninput = changed;
        choose.onchange = () => { inputs.set(selectionKey, { source: source.value, width: width.value }); authorInputs?.invalidate(`diagram:${selectionKey}:source`); authorInputs?.invalidate(`diagram:${selectionKey}:width`); const replacement = source.cloneNode(false) as HTMLTextAreaElement; source.replaceWith(replacement); source = replacement; source.oninput = changed; const newWidth = width.cloneNode(false) as HTMLInputElement; width.replaceWith(newWidth); width = newWidth; width.oninput = changed; selectionKey = choose.value; selected = selectionKey === "new" ? undefined : occurrences[Number(selectionKey)]; const retained = inputs.get(selectionKey); source.value = retained?.source ?? selected?.source ?? "flowchart LR\n  A[开始] --> B[完成]"; width.value = retained?.width ?? "1200"; authorInputs?.bind(source, `diagram:${selectionKey}:source`); authorInputs?.bind(width, `diagram:${selectionKey}:width`); changed(); };
        const controls = ui("div", "diagram-actions");
        controls.append(generate, commit);
        dialog.append(header, ui("p", "data-help", uiMessage("diagramPanel.sourceAndPngDescription")), choose, source, ui("label", "data-label", uiMessage("diagramPanel.pngWidthRange")), width, preview, status, controls);
        dialog.addEventListener("cancel", event => { event.preventDefault(); controller.cancel(); draw(); dialog.close(); });
        return { dialog, clear: () => {
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
            } };
    }
}
