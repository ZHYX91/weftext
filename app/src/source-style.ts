import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { StateEffect, StateField, type EditorState, type Transaction } from "@codemirror/state";
import { Decoration, EditorView, type DecorationSet } from "@codemirror/view";
import type { SyntaxToken } from "./types";
export interface SourceStyleBasis {
    source: string;
    generation: number;
    epoch: number;
}
export interface SourceTokens {
    tokens: readonly SyntaxToken[];
    styled: boolean;
    basis: SourceStyleBasis;
}
export type SourceStyleStatus = "none" | "current" | "pending";
type AppearanceKind = "heading" | "strong" | "emph" | "list" | "comment" | "math" | "string";
interface Appearance {
    kind: AppearanceKind;
    styled: boolean;
    from: number;
    to: number;
    // The complete covered lines must survive byte-for-byte. This is a visual
    // footprint, never a certified syntax range or an editing capability.
    lineFrom: number;
    lineTo: number;
}
export interface SourceStyleState {
    decorations: DecorationSet;
    status: SourceStyleStatus;
    generation: number;
    epoch: number;
    appearances: readonly Appearance[];
}
export const sourceTokensEffect = StateEffect.define<SourceTokens>();
/** A personal display-mode change does not certify or retire parser results. */
export const sourceStyleModeEffect = StateEffect.define<boolean>();
/** Authority/session retirement also invalidates equal-byte queued results. */
export const retireSourceStylesEffect = StateEffect.define<null>();
function appearanceKind(token: SyntaxToken): AppearanceKind | null {
    const name = token.kind.toLowerCase();
    return name.includes("heading") ? "heading" : name.includes("strong") ? "strong" : name.includes("emph") ? "emph" : name.includes("listitem") || name.includes("enumitem") ? "list" : name.includes("comment") ? "comment" : name.includes("math") ? "math" : name.includes("string") ? "string" : null;
}
function parsedAppearances(transaction: Transaction, effect: SourceTokens): Appearance[] {
    const appearances: Appearance[] = [];
    for (const token of effect.tokens) {
        const from = token.utf16From, to = token.utf16To, kind = appearanceKind(token);
        if (!kind || !Number.isSafeInteger(from) || !Number.isSafeInteger(to) || from < 0 || to > transaction.newDoc.length || from >= to)
            continue;
        appearances.push({ kind, styled: effect.styled, from, to, lineFrom: transaction.newDoc.lineAt(from).from, lineTo: transaction.newDoc.lineAt(to - 1).to });
    }
    return appearances;
}
/** Preserve only unchanged display geometry; old parser classes never survive. */
function pendingAppearances(appearances: readonly Appearance[], transaction: Transaction): Appearance[] {
    const guards = new Map<string, {
        from: number;
        to: number;
    } | null>();
    const retained: Appearance[] = [];
    for (const appearance of appearances) {
        const key = `${appearance.lineFrom}:${appearance.lineTo}`;
        let guard = guards.get(key);
        if (guard === undefined) {
            let touched = false;
            transaction.changes.iterChangedRanges((from, to) => {
                if (from < appearance.lineTo && to > appearance.lineFrom || from === to && from > appearance.lineFrom && from < appearance.lineTo)
                    touched = true;
            });
            const from = transaction.changes.mapPos(appearance.lineFrom, 1), to = transaction.changes.mapPos(appearance.lineTo, -1);
            // Boundary insertions may shift a complete line (for example a new line
            // before a heading). Same-line prefixes/suffixes or joined lines do not.
            guard = !touched && from <= to && transaction.newDoc.lineAt(from).from === from && transaction.newDoc.lineAt(to).to === to && transaction.startState.doc.sliceString(appearance.lineFrom, appearance.lineTo) === transaction.newDoc.sliceString(from, to) ? { from, to } : null;
            guards.set(key, guard);
        }
        if (!guard)
            continue;
        const from = transaction.changes.mapPos(appearance.from, 1), to = transaction.changes.mapPos(appearance.to, -1);
        if (from >= to || transaction.startState.doc.sliceString(appearance.from, appearance.to) !== transaction.newDoc.sliceString(from, to))
            continue;
        retained.push({ ...appearance, from, to, lineFrom: guard.from, lineTo: guard.to });
    }
    return retained;
}
function draw(appearances: readonly Appearance[], status: SourceStyleStatus): DecorationSet {
    const marks: ReturnType<Decoration["range"]>[] = [], headingLines = new Set<number>();
    const current = status === "current";
    for (const appearance of appearances) {
        if (appearance.kind === "heading" && appearance.styled && !headingLines.has(appearance.lineFrom)) {
            headingLines.add(appearance.lineFrom);
            marks.push(Decoration.line({ class: `cm-source-heading-layout${current ? " cm-rendered-heading" : ""}`, attributes: { "data-source-style": status } }).range(appearance.lineFrom));
        }
        const classes = `cm-source-appearance-${appearance.kind}${appearance.styled ? " cm-source-appearance-rendered" : ""}${current ? ` cm-typst-${appearance.kind}${appearance.styled ? " cm-rendered" : ""}` : ""}`;
        marks.push(Decoration.mark({ class: classes, attributes: { "data-source-style": status } }).range(appearance.from, appearance.to));
    }
    return Decoration.set(marks, true);
}
/** Source presentation is deliberately independent of maps and write authority. */
export const sourceStyles = StateField.define<SourceStyleState>({
    create: () => ({ decorations: Decoration.none, status: "none", generation: 0, epoch: 0, appearances: [] }),
    update(value, transaction) {
        let { appearances, status, epoch } = value;
        const generation = value.generation + (transaction.docChanged ? 1 : 0);
        if (transaction.docChanged) {
            appearances = pendingAppearances(appearances, transaction);
            if (status !== "none")
                status = "pending";
        }
        let source: string | undefined;
        for (const effect of transaction.effects) {
            if (effect.is(retireSourceStylesEffect)) {
                epoch++;
                appearances = [];
                status = "none";
            }
            else if (effect.is(sourceStyleModeEffect)) {
                if (appearances.some(appearance => appearance.styled !== effect.value))
                    appearances = appearances.map(appearance => ({ ...appearance, styled: effect.value }));
            }
            else if (effect.is(sourceTokensEffect)) {
                const basis = effect.value.basis;
                if (basis.generation !== generation || basis.epoch !== epoch || basis.source !== (source ??= transaction.newDoc.toString()))
                    continue;
                appearances = parsedAppearances(transaction, effect.value);
                status = "current";
            }
        }
        if (appearances === value.appearances && status === value.status && generation === value.generation && epoch === value.epoch)
            return value;
        return { appearances, status, generation, epoch, decorations: draw(appearances, status) };
    },
    provide: field => [
        EditorView.decorations.from(field, value => value.decorations),
        EditorView.contentAttributes.from(field, value => ({
            "data-source-style": value.status,
            "data-source-style-status": value.status,
        })),
    ],
});
export function sourceStyleDescription(state: EditorState): UiText {
    const status = state.field(sourceStyles).status;
    return uiMessage(status === "pending" ? "sourceStyle.pendingTitle" : status === "current" ? "sourceStyle.currentTitle" : "sourceStyle.notReadyTitle");
}
export function currentSourceStyleBasis(state: EditorState): SourceStyleBasis {
    const value = state.field(sourceStyles);
    return { source: state.doc.toString(), generation: value.generation, epoch: value.epoch };
}
