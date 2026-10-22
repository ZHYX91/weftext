import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { insertWritingBlock } from "./writing-controls";
import type { WritingController } from "./writing";
import type { EverydayCommand } from "./everyday-navigation";
export function everydayWritingCommands(controller: WritingController | undefined, writable: boolean, insertQuery?: () => Promise<void>): EverydayCommand[] {
    const reason = !controller ? uiMessage("shell.openNoteFirst") : !writable ? uiMessage("everydayWriting.writableSessionRequired") : undefined;
    const actions: EverydayCommand[] = ([["paragraph", uiMessage("everydayWriting.paragraph")], ["heading", uiMessage("writing.heading")], ["bulletList", uiMessage("everydayWriting.unorderedList")], ["numberedList", uiMessage("everydayWriting.orderedList")], ["image", uiMessage("documentNotes.image")]] as const).map(([kind, label]) => ({
        id: `insert-${kind}`, label: displayMessage("documentNotes.insertNote", { p0: uiDisplayParameter(label) }), detail: uiMessage("everydayWriting.appendSharedDraftDescription"), unavailable: reason,
        run: () => insertWritingBlock(controller!, kind),
    }));
    if (insertQuery)
        actions.push({ id: "insert-query", label: uiMessage("shell.insertEmbeddedQuery"), detail: uiMessage("everydayWriting.installedSchemaDescription"), unavailable: reason, run: insertQuery });
    const selection = controller?.sourceSelection;
    const block = selection && controller?.map?.bodyBlocks?.filter(item => item.range.utf16From <= selection.from && item.range.utf16To >= selection.to).sort((a, b) => a.range.to - a.range.from - (b.range.to - b.range.from))[0];
    const snapshot = controller?.snapshot;
    const checked = async (intent: () => Parameters<WritingController["execute"]>[0]) => {
        await controller?.prepareCommand();
        if (!controller || !snapshot || !controller.isSnapshotCurrent(snapshot))
            throw new UiError(uiMessage("everydayWriting.menuBodyBasisChanged"));
        await controller.execute(intent());
    };
    const selectedBlock = () => {
        const selected = controller!.sourceSelection;
        const current = controller!.map?.bodyBlocks?.find(item => item.range.utf16From <= selected.from && item.range.utf16To >= selected.to);
        if (!current)
            throw new UiError(uiMessage("everydayWriting.uniqueRootBlockMissing"));
        return current;
    };
    for (const [target, label] of [["paragraph", uiMessage("everydayWriting.paragraph")], ["heading", uiMessage("everydayWriting.levelOneHeading")], ["bulletList", uiMessage("everydayWriting.unorderedListItem")], ["numberedList", uiMessage("everydayWriting.orderedListItem")]] as const)
        actions.push({ id: `convert-${target}`, label: displayMessage("everydayWriting.convertCurrentBlock", { p0: uiDisplayParameter(label) }), unavailable: reason ?? (controller?.map && !block ? uiMessage("everydayWriting.rootBlockCursorDescription") : undefined), run: () => checked(() => ({ kind: "convertBodyBlock", range: selectedBlock().range, target, level: 1 })) });
    for (const direction of ["up", "down"] as const)
        actions.push({ id: `move-${direction}`, label: direction === "up" ? uiMessage("everydayWriting.moveBlockUp") : uiMessage("everydayWriting.moveBlockDown"), detail: uiMessage("everydayWriting.moveBlockDescription"), unavailable: reason ?? (controller?.map && !block ? uiMessage("everydayWriting.selectRootBlockFirst") : undefined), run: () => checked(() => ({ kind: "moveBodyBlock", range: selectedBlock().range, direction })) });
    actions.push({ id: "undo", label: uiMessage("hostedEditor.undo"), unavailable: reason, run: () => controller!.undo() }, { id: "redo", label: uiMessage("hostedEditor.redo"), unavailable: reason, run: () => controller!.redo() });
    return actions;
}
