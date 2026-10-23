import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { createLatexMathPanel, type LatexMathPanel } from "./math-latex";
import { nativeElement } from "./native-node";
import { sourceBoundary } from "./editor";
import type { PanelHost } from "./data-panel";
import type { RenderNode } from "./types";
type Host = PanelHost & {
    selection?(): {
        from: number;
        to: number;
    } | undefined;
    editable?(): boolean;
};
interface Preview {
    ok: boolean;
    tree: RenderNode | null;
    source: string;
    serial: number;
    message?: string;
}
/** Conversion uses a frozen insertion basis. Preview is a separate consumer and
 * can never register the proposed source as the editor's current version. */
export class LatexInsertionPanel {
    private dialogs = new Map<string, {
        dialog: HTMLDialogElement;
        panel: LatexMathPanel;
        source: string;
    }>();
    constructor(private host: Host) { }
    clear(): void {
        for (const item of this.dialogs.values()) {
            item.panel.destroy();
            item.dialog.close();
            item.dialog.remove();
        }
        this.dialogs.clear();
    }
    async open(): Promise<void> {
        await this.host.prepareAction?.();
        const basis = this.host.getDocument();
        if (!basis || this.host.editable?.() === false)
            throw new UiError(uiMessage("latexInsertion.openEditableNoteFirst"));
        const selection = this.host.selection?.() ?? { from: basis.source.length, to: basis.source.length };
        if (!sourceBoundary(basis.source, selection.from) || !sourceBoundary(basis.source, selection.to) || selection.to < selection.from)
            throw new UiError(uiMessage("latexInsertion.selectionCharacterBoundaryInvalid"));
        const key = `${basis.sessionId}:${basis.baseId}:${basis.serial}:${selection.from}:${selection.to}`, retained = this.dialogs.get(key);
        if (retained && retained.source === basis.source) {
            if (!retained.dialog.open)
                retained.dialog.showModal();
            return;
        }
        const current = (): boolean => {
            const value = this.host.getDocument();
            return !!value && value.sessionId === basis.sessionId && value.source === basis.source && value.serial === basis.serial && value.baseId === basis.baseId && this.host.editable?.() !== false;
        };
        const proposed = (text: string): string => basis.source.slice(0, selection.from) + text + basis.source.slice(selection.to);
        const dialog = document.createElement("dialog");
        dialog.className = "modal writing-dialog latex-insertion";
        const close = document.createElement("button");
        close.type = "button";
        setUiText(close, uiMessage("common.close"));
        const panel = createLatexMathPanel({
            authorInputs: this.host.authorInputs?.(basis),
            preview: async (result) => {
                if (!current())
                    throw new UiError(uiMessage("latexInsertion.noteOrSelectionBasisChanged"));
                const candidate = proposed(result.typst);
                const resultView = await this.host.request<Preview>("/api/editor/preview", { sessionId: basis.sessionId, source: basis.source, serial: basis.serial, proposalSource: candidate });
                if (!current() || resultView.source !== candidate || resultView.serial !== basis.serial)
                    throw new UiError(uiMessage("latexInsertion.draftChangedDuringPreview"));
                if (!resultView.ok || !resultView.tree)
                    throw new UiError(resultView.message || uiMessage("latexInsertion.nativeFormulaCompileFailed"));
                const view = document.createElement("div");
                view.className = "native-proposal-preview";
                setUiAttr(view, "aria-label", uiMessage("latexInsertion.completeDocumentPreview"));
                const draw = (node: RenderNode): Node => {
                    if (node.kind === "text")
                        return uiTextNode(node.text ?? "");
                    if (node.kind === "tag")
                        return document.createDocumentFragment();
                    if (node.kind === "frame") {
                        const hint = document.createElement("p");
                        setUiText(hint, uiMessage("latexInsertion.layoutRequiresPagination"));
                        return hint;
                    }
                    const element = nativeElement(node);
                    for (const child of node.children)
                        element.append(draw(child));
                    if (node.text)
                        element.append(uiTextNode(node.text));
                    return element;
                };
                view.append(draw(resultView.tree));
                return view;
            },
            apply: async (result) => {
                if (!current())
                    throw new UiError(uiMessage("latexInsertion.noteOrEligibilityChanged"));
                await this.host.applyPatch({ from: selection.from, to: selection.to, text: result.typst }, basis);
                this.host.notify(uiMessage("latexInsertion.formulaEnteredDraft"));
            },
        });
        this.dialogs.set(key, { dialog, panel, source: basis.source });
        close.onclick = () => dialog.close();
        dialog.addEventListener("cancel", event => { event.preventDefault(); dialog.close(); });
        const position = document.createElement("p");
        setUiText(position, selection.from === selection.to ? uiMessage("latexInsertion.insertConfirmationHint") : uiMessage("latexInsertion.replaceConfirmationHint"));
        dialog.append(close, position, panel.element);
        document.body.append(dialog);
        dialog.showModal();
    }
}
