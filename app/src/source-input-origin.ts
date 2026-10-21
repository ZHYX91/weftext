import { Annotation, StateEffect, StateField, type Extension, type ChangeSet } from "@codemirror/state";
import { invertedEffects } from "@codemirror/commands";

/** A base reference contains no source text. Opaque replacements are never replayed. */
export type InputPiece = { base: number; length: number } | { text: string } | { opaque: number };
export interface PersonalText { id: string; text: string }
export function personalInputId(): string { return Array.from(crypto.getRandomValues(new Uint8Array(16)), value => value.toString(16).padStart(2, "0")).join(""); }
export interface InputOrigin { baseLength: number; pieces: InputPiece[]; detached: PersonalText[]; revision: number }
export const inputOriginKind = Annotation.define<"author" | "protected">();
export const personalTextEffect = StateEffect.define<PersonalText>();
const detachedTexts = (value: InputOrigin): PersonalText[] => [...value.detached, ...value.pieces.flatMap((piece, index) => "text" in piece && piece.text ? [{ id: `piece:${value.revision}:${index}`, text: piece.text }] : [])];
export const restoreInputOrigin = StateEffect.define<InputOrigin>({ map: (value, mapping) => ({ ...value, pieces: [{ opaque: mapping.newLength }], detached: detachedTexts(value), revision: value.revision + 1 }) });
const length = (piece: InputPiece) => "text" in piece ? piece.text.length : "base" in piece ? piece.length : piece.opaque;
export const inputLength = (value: InputOrigin) => value.pieces.reduce((sum, piece) => sum + length(piece), 0);
export function inputTexts(value: InputOrigin): string[] { return detachedTexts(value).map(entry => entry.text); }
export function initialInputOrigin(size: number): InputOrigin { return { baseLength: size, pieces: size ? [{ base: 0, length: size }] : [], detached: [], revision: 0 }; }
export function hasPersonalInput(value: InputOrigin): boolean { return !!value.detached.length || value.pieces.length !== (value.baseLength ? 1 : 0) || !!value.baseLength && !("base" in value.pieces[0] && value.pieces[0].base === 0 && value.pieces[0].length === value.baseLength); }
function slice(pieces: readonly InputPiece[], from: number, to: number): InputPiece[] {
  let at = 0; const result: InputPiece[] = [];
  for (const piece of pieces) {
    const end = at + length(piece), left = Math.max(from, at), right = Math.min(to, end);
    if (right > left) result.push("text" in piece ? { text: piece.text.slice(left - at, right - at) } : "base" in piece ? { base: piece.base + left - at, length: right - left } : { opaque: right - left });
    at = end; if (at >= to) break;
  }
  return result;
}
function compact(pieces: InputPiece[]): InputPiece[] {
  const result: InputPiece[] = [];
  for (const piece of pieces) {
    if (!length(piece)) continue;
    const last = result[result.length - 1];
    if (last && "text" in last && "text" in piece) last.text += piece.text;
    else if (last && "base" in last && "base" in piece && last.base + last.length === piece.base) last.length += piece.length;
    else if (last && "opaque" in last && "opaque" in piece) last.opaque += piece.opaque;
    else result.push({ ...piece });
  }
  return result;
}
export function changeInputOrigin(previous: InputOrigin, changes: ChangeSet, author: boolean): InputOrigin {
  let end = 0; const pieces: InputPiece[] = [], detached = [...previous.detached];
  changes.iterChanges((from, to, _fromB, _toB, inserted) => {
    pieces.push(...slice(previous.pieces, end, from));
    if (!author && inserted.length) detached.push(...slice(previous.pieces, from, to).flatMap((piece, index) => "text" in piece ? [{ id: `splice:${previous.revision}:${from}:${index}`, text: piece.text }] : []));
    if (inserted.length) pieces.push(author ? { text: inserted.toString() } : { opaque: inserted.length });
    end = to;
  }, true);
  pieces.push(...slice(previous.pieces, end, changes.length));
  return { ...previous, pieces: compact(pieces), detached, revision: previous.revision + 1 };
}
export const sourceInputOrigin = StateField.define<InputOrigin>({
  create: state => initialInputOrigin(state.doc.length),
  update(previous, transaction) {
    let value = previous;
    if (transaction.docChanged) {
      const kind = transaction.annotation(inputOriginKind);
      const author = kind === "author" || kind !== "protected" && (transaction.isUserEvent("input.type") || transaction.isUserEvent("input.paste"));
      value = changeInputOrigin(previous, transaction.changes, author);
    }
    for (const effect of transaction.effects) if (effect.is(restoreInputOrigin)) value = effect.value;
    for (const effect of transaction.effects) if (effect.is(personalTextEffect)) value = { ...value, detached: [...value.detached.filter(entry => entry.id !== effect.value.id), ...(effect.value.text ? [effect.value] : [])] };
    if (inputLength(value) !== transaction.newDoc.length) return { ...value, pieces: [{ opaque: transaction.newDoc.length }], detached: detachedTexts(value), revision: value.revision + 1 };
    return value;
  },
});
/** Ownership effects travel with CM's existing history, including grouped IME. */
export const sourceInputOrigins: Extension = [sourceInputOrigin, invertedEffects.of(transaction => transaction.docChanged ? [restoreInputOrigin.of(transaction.startState.field(sourceInputOrigin))] : [])];

export function replayPersonalInput(origin: InputOrigin, base: string): string {
  if (base.length !== origin.baseLength || origin.detached.length || origin.pieces.some(piece => "opaque" in piece)) throw new Error("输入含未能准确映射的原生修改，保留新文字，未自动恢复正文。");
  return origin.pieces.map(piece => {
    if ("text" in piece) return piece.text;
    if (!("base" in piece) || piece.base < 0 || piece.length < 0 || piece.base + piece.length > base.length) throw new Error("恢复输入的原基线范围无效，未恢复正文。");
    return base.slice(piece.base, piece.base + piece.length);
  }).join("");
}
