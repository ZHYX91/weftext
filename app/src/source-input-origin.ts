import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { Annotation, StateEffect, StateField, Transaction, type Extension, type EditorState } from "@codemirror/state";
import { invertedEffects } from "@codemirror/commands";
/** Serializable recovery is a projection, never the live document or its Undo. */
export type InputPiece = {
    base: number;
    length: number;
} | {
    text: string;
} | {
    opaque: number;
};
export interface PersonalText {
    id: string;
    revision: number;
    text: string;
}
export interface InputOrigin {
    baseLength: number;
    pieces: InputPiece[];
    detached: PersonalText[];
    copyOnly?: PersonalText[];
    revision: number;
}
export interface PendingInputCheckpoint {
    operationId: string;
    baseHash?: string;
    origin: InputOrigin;
}
export interface PersonalInputProjection {
    baseHash?: string;
    origin: InputOrigin;
    candidate?: PendingInputCheckpoint;
}
export function personalInputId(): string { return Array.from(crypto.getRandomValues(new Uint8Array(16)), value => value.toString(16).padStart(2, "0")).join(""); }
interface Atom {
    id: string;
    at: number;
    length: number;
    kind: "protected" | "author" | "opaque";
    text?: string;
}
export interface SourceInputOrigin {
    pieces: Atom[];
    detached: PersonalText[];
    copyOnly?: PersonalText[];
    revision: number;
}
interface CheckpointInterval {
    id: string;
    at: number;
    length: number;
    base: number;
}
export interface InputCheckpoint {
    readonly id: string;
    readonly baseHash?: string;
    readonly length: number;
    readonly intervals: readonly CheckpointInterval[];
    readonly personal: readonly {
        id: string;
        revision: number;
    }[];
    readonly index: ReadonlyMap<string, readonly CheckpointInterval[]>;
}
interface Checkpoints {
    confirmed: InputCheckpoint;
    pending?: {
        operationId: string;
        checkpoint: InputCheckpoint;
    };
}
type CheckpointCommand = {
    kind: "initialHash";
    id: string;
    hash: string;
} | {
    kind: "begin";
    operationId: string;
    checkpoint: InputCheckpoint;
} | {
    kind: "settle";
    operationId: string;
    committed: boolean;
};
export const inputOriginKind = Annotation.define<"author" | "protected">();
export const personalTextEffect = StateEffect.define<PersonalText>();
const checkpointEffect = StateEffect.define<CheckpointCommand>();
const pureAtoms = (origin: SourceInputOrigin): PersonalText[] => [...origin.detached, ...origin.pieces.flatMap(piece => piece.kind === "author" && piece.text ? [{ id: `atom:${piece.id}:${piece.at}`, revision: 0, text: piece.text }] : [])];
export const restoreInputOrigin = StateEffect.define<SourceInputOrigin>({ map: (origin, mapping) => ({ pieces: mapping.newLength ? [{ id: personalInputId(), at: 0, length: mapping.newLength, kind: "opaque" }] : [], detached: pureAtoms(origin), copyOnly: origin.copyOnly, revision: origin.revision + 1 }) });
function atomSlice(pieces: readonly Atom[], from: number, to: number): Atom[] {
    let at = 0;
    const result: Atom[] = [];
    for (const piece of pieces) {
        const left = Math.max(from, at), right = Math.min(to, at + piece.length);
        if (left < right)
            result.push({ ...piece, at: piece.at + left - at, length: right - left, ...(piece.kind === "author" ? { text: piece.text!.slice(left - at, right - at) } : {}) });
        at += piece.length;
        if (at >= to)
            break;
    }
    return result;
}
function compactAtoms(pieces: Atom[]): Atom[] {
    const result: Atom[] = [];
    for (const piece of pieces) {
        if (!piece.length)
            continue;
        const previous = result.at(-1);
        if (previous && previous.id === piece.id && previous.kind === piece.kind && previous.at + previous.length === piece.at) {
            previous.length += piece.length;
            if (previous.kind === "author")
                previous.text += piece.text!;
        }
        else
            result.push({ ...piece });
    }
    return result;
}
function compactProjection(pieces: InputPiece[]): InputPiece[] {
    const result: InputPiece[] = [];
    for (const piece of pieces) {
        const previous = result.at(-1);
        if (previous && "text" in previous && "text" in piece)
            previous.text += piece.text;
        else if (previous && "base" in previous && "base" in piece && previous.base + previous.length === piece.base)
            previous.length += piece.length;
        else if (previous && "opaque" in previous && "opaque" in piece)
            previous.opaque += piece.opaque;
        else
            result.push({ ...piece });
    }
    return result;
}
export const sourceInputOrigin = StateField.define<SourceInputOrigin>({
    create: state => ({ pieces: state.doc.length ? [{ id: personalInputId(), at: 0, length: state.doc.length, kind: "protected" }] : [], detached: [], revision: 0 }),
    update(previous, transaction) {
        let value = previous;
        if (transaction.docChanged) {
            const kind = transaction.annotation(inputOriginKind), author = kind === "author" || kind !== "protected" && (transaction.isUserEvent("input.type") || transaction.isUserEvent("input.paste"));
            let end = 0;
            const pieces: Atom[] = [], detached = [...previous.detached];
            transaction.changes.iterChanges((from, to, _fromB, _toB, inserted) => {
                pieces.push(...atomSlice(previous.pieces, end, from));
                if (!author && inserted.length)
                    for (const displaced of atomSlice(previous.pieces, from, to))
                        if (displaced.kind === "author" && displaced.text)
                            detached.push({ id: `displaced:${displaced.id}:${displaced.at}:${displaced.length}`, revision: 0, text: displaced.text });
                if (inserted.length)
                    pieces.push({ id: personalInputId(), at: 0, length: inserted.length, kind: author ? "author" : "opaque", ...(author ? { text: inserted.toString() } : {}) });
                end = to;
            }, true);
            pieces.push(...atomSlice(previous.pieces, end, transaction.changes.length));
            value = { ...previous, pieces: compactAtoms(pieces), detached, revision: previous.revision + 1 };
        }
        for (const effect of transaction.effects)
            if (effect.is(restoreInputOrigin))
                value = effect.value;
        for (const effect of transaction.effects)
            if (effect.is(personalTextEffect))
                value = { ...value, detached: [...value.detached.filter(entry => entry.id !== effect.value.id), ...(effect.value.text ? [{ ...effect.value }] : [])] };
        if (value.pieces.reduce((sum, piece) => sum + piece.length, 0) !== transaction.newDoc.length)
            return { ...value, pieces: transaction.newDoc.length ? [{ id: personalInputId(), at: 0, length: transaction.newDoc.length, kind: "opaque" }] : [], detached: pureAtoms(value), revision: value.revision + 1 };
        return value;
    },
});
export function inputCheckpoint(state: EditorState, baseHash?: string): InputCheckpoint {
    const origin = state.field(sourceInputOrigin), index = new Map<string, CheckpointInterval[]>();
    let base = 0;
    const intervals = origin.pieces.map(piece => { const interval = { id: piece.id, at: piece.at, length: piece.length, base }; base += piece.length; const list = index.get(piece.id) ?? []; list.push(interval); index.set(piece.id, list); return interval; });
    for (const list of index.values()) {
        list.sort((left, right) => left.at - right.at);
        if (list.some((entry, at) => at > 0 && list[at - 1].at + list[at - 1].length > entry.at))
            throw new UiError(uiMessage("sourceInputOrigin.overlappingIdentity"));
    }
    return { id: personalInputId(), baseHash, length: base, intervals, index, personal: origin.detached.map(({ id, revision }) => ({ id, revision })) };
}
export const sourceInputCheckpoints = StateField.define<Checkpoints>({
    create: state => ({ confirmed: inputCheckpoint(state) }),
    update(previous, transaction) {
        let value = previous;
        for (const effect of transaction.effects)
            if (effect.is(checkpointEffect)) {
                const command = effect.value;
                if (command.kind === "initialHash") {
                    if (value.confirmed.id === command.id && !value.confirmed.baseHash)
                        value = { ...value, confirmed: { ...value.confirmed, baseHash: command.hash } };
                }
                else if (command.kind === "begin") {
                    if (value.pending) {
                        if (value.pending.operationId !== command.operationId || value.pending.checkpoint.baseHash !== command.checkpoint.baseHash)
                            throw new UiError(uiMessage("sourceInputOrigin.saveUnverifiedBaselineFrozen"));
                        continue;
                    }
                    value = { ...value, pending: { operationId: command.operationId, checkpoint: command.checkpoint } };
                }
                else {
                    if (value.pending?.operationId !== command.operationId)
                        throw new UiError(uiMessage("sourceInputOrigin.saveReceiptMismatch"));
                    value = { confirmed: command.committed ? value.pending.checkpoint : value.confirmed };
                }
            }
        return value;
    },
});
/** Metadata never enters Undo. History restores atoms; projection uses this current basis. */
export const sourceInputOrigins: Extension = [sourceInputOrigin, sourceInputCheckpoints, invertedEffects.of(transaction => transaction.docChanged ? [restoreInputOrigin.of(transaction.startState.field(sourceInputOrigin))] : [])];
export const initializeInputHash = (id: string, hash: string) => ({ effects: checkpointEffect.of({ kind: "initialHash", id, hash }), annotations: Transaction.addToHistory.of(false) });
export const beginInputCheckpoint = (operationId: string, checkpoint: InputCheckpoint) => ({ effects: checkpointEffect.of({ kind: "begin", operationId, checkpoint }), annotations: Transaction.addToHistory.of(false) });
export const settleInputCheckpoint = (operationId: string, committed: boolean) => ({ effects: checkpointEffect.of({ kind: "settle", operationId, committed }), annotations: Transaction.addToHistory.of(false) });
function project(origin: SourceInputOrigin, checkpoint: InputCheckpoint): InputOrigin {
    const pieces: InputPiece[] = [];
    const unmapped = (piece: Atom, from: number, length: number) => {
        if (length)
            pieces.push(piece.kind === "author" ? { text: piece.text!.slice(from - piece.at, from - piece.at + length) } : { opaque: length });
    };
    for (const piece of origin.pieces) {
        const list = checkpoint.index.get(piece.id);
        if (!list) {
            unmapped(piece, piece.at, piece.length);
            continue;
        }
        let low = 0, high = list.length;
        while (low < high) {
            const middle = (low + high) >>> 1;
            if (list[middle].at + list[middle].length <= piece.at)
                low = middle + 1;
            else
                high = middle;
        }
        let at = piece.at;
        const end = piece.at + piece.length;
        for (let slot = low; slot < list.length && list[slot].at < end; slot++) {
            const interval = list[slot], left = Math.max(at, interval.at), right = Math.min(end, interval.at + interval.length);
            if (at < left)
                unmapped(piece, at, left - at);
            if (left < right)
                pieces.push({ base: interval.base + left - interval.at, length: right - left });
            at = Math.max(at, right);
        }
        if (at < end)
            unmapped(piece, at, end - at);
    }
    const included = (entry: PersonalText) => checkpoint.personal.some(old => old.id === entry.id && old.revision === entry.revision);
    return { baseLength: checkpoint.length, pieces: compactProjection(pieces), detached: origin.detached.filter(entry => !included(entry)), copyOnly: [...(origin.copyOnly ?? []), ...origin.detached.filter(included)], revision: origin.revision };
}
export function personalInputProjection(state: EditorState): PersonalInputProjection {
    const checkpoints = state.field(sourceInputCheckpoints), origin = state.field(sourceInputOrigin);
    return { baseHash: checkpoints.confirmed.baseHash, origin: project(origin, checkpoints.confirmed), ...(checkpoints.pending ? { candidate: { operationId: checkpoints.pending.operationId, baseHash: checkpoints.pending.checkpoint.baseHash, origin: project(origin, checkpoints.pending.checkpoint) } } : {}) };
}
export function restoredInputOrigin(origin: InputOrigin, basis: SourceInputOrigin): SourceInputOrigin {
    if (origin.baseLength !== basis.pieces.reduce((sum, piece) => sum + piece.length, 0))
        throw new UiError(uiMessage("sourceInputOrigin.recoveryBaselineLengthMismatch"));
    return { pieces: compactAtoms(origin.pieces.flatMap((piece): Atom[] => "base" in piece ? atomSlice(basis.pieces, piece.base, piece.base + piece.length) : "text" in piece ? [{ id: personalInputId(), at: 0, length: piece.text.length, kind: "author", text: piece.text }] : [{ id: personalInputId(), at: 0, length: piece.opaque, kind: "opaque" }])), detached: [...origin.detached], copyOnly: origin.copyOnly, revision: basis.revision + 1 };
}
export const inputLength = (origin: InputOrigin) => origin.pieces.reduce((sum, piece) => sum + ("text" in piece ? piece.text.length : "base" in piece ? piece.length : piece.opaque), 0);
export function inputTexts(origin: InputOrigin): string[] { return [...origin.pieces.flatMap(piece => "text" in piece && piece.text ? [piece.text] : []), ...origin.detached.map(entry => entry.text), ...(origin.copyOnly ?? []).map(entry => entry.text)]; }
export function hasPersonalInput(origin: InputOrigin): boolean { return !!origin.detached.length || !!origin.copyOnly?.length || origin.pieces.length !== (origin.baseLength ? 1 : 0) || !!origin.baseLength && !("base" in origin.pieces[0] && origin.pieces[0].base === 0 && origin.pieces[0].length === origin.baseLength); }
export function replayPersonalInput(origin: InputOrigin, base: string): string {
    if (base.length !== origin.baseLength || origin.detached.length || origin.pieces.some(piece => "opaque" in piece))
        throw new UiError(uiMessage("sourceInputOrigin.nativeChangesUnmapped"));
    return origin.pieces.map(piece => {
        if ("text" in piece)
            return piece.text;
        if (!("base" in piece) || !Number.isSafeInteger(piece.base) || !Number.isSafeInteger(piece.length) || piece.base < 0 || piece.length < 0 || piece.base + piece.length > base.length)
            throw new UiError(uiMessage("sourceInputOrigin.originalBaselineRangeInvalid"));
        return base.slice(piece.base, piece.base + piece.length);
    }).join("");
}
