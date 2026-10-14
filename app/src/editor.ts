import { Annotation, Compartment, EditorState, StateEffect, StateField, type Extension, type Transaction } from "@codemirror/state";
import { Decoration, EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, drawSelection, type DecorationSet } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab, isolateHistory, undo, redo } from "@codemirror/commands";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { bracketMatching, indentOnInput } from "@codemirror/language";
import type { SyntaxToken } from "./types";

export interface SourceSelection { from: number; to: number }
export interface SourcePatch { from: number; to: number; expected: string; insert: string }
export interface SourceReveal extends SourceSelection { raw: string }
export interface SourceEditorEvent {
  kind: "source" | "selection" | "composition" | "reveal";
  source: string;
  generation: number;
  selection: SourceSelection;
  composing: boolean;
}

/** Native edits use UTF-16 coordinates. Validate every splice before any change. */
export function applySourcePatches(source: string, patches: readonly SourcePatch[]): string {
  const sorted = [...patches].sort((a, b) => a.from - b.from || a.to - b.to);
  let end = -1, previous: SourcePatch | undefined;
  for (const patch of sorted) {
    if (!Number.isSafeInteger(patch.from) || !Number.isSafeInteger(patch.to) || patch.from < 0 || patch.to < patch.from || patch.to > source.length || source.slice(patch.from, patch.to) !== patch.expected) throw new Error("原生编辑范围已变化，未应用修改。");
    if (patch.from < end || previous && patch.from === previous.from) throw new Error("原生编辑范围重叠，未应用修改。");
    if (!sourceBoundary(source, patch.from) || !sourceBoundary(source, patch.to)) throw new Error("编辑范围拆开了字符或换行，未应用修改。");
    end = patch.to;
    previous = patch;
  }
  let result = source;
  for (const patch of sorted.reverse()) result = result.slice(0, patch.from) + patch.insert + result.slice(patch.to);
  return result;
}

export function sourceBoundary(source: string, at: number): boolean {
  if (at <= 0 || at >= source.length) return at >= 0 && at <= source.length;
  const before = source.charCodeAt(at - 1), after = source.charCodeAt(at);
  return !(before >= 0xd800 && before <= 0xdbff && after >= 0xdc00 && after <= 0xdfff) && !(before === 13 && after === 10);
}

export const sourceRevealEffect = StateEffect.define<SourceReveal | null>();
const nativeEdit = Annotation.define<boolean>();
const revealField = StateField.define<SourceReveal | null>({
  create: () => null,
  update(value, transaction) {
    if (value && transaction.docChanged) {
      let contained = true;
      transaction.changes.iterChangedRanges((from, to) => { if (from < value!.from || to > value!.to) contained = false; });
      // Only an edit wholly within the revealed source slot can retain it.
      // Undo/other commands outside the slot retire this display, not its bytes.
      if (!contained) value = null;
      else {
        const from = transaction.changes.mapPos(value.from, -1), to = transaction.changes.mapPos(value.to, 1);
        value = { from, to, raw: transaction.newDoc.sliceString(from, to) };
      }
    }
    for (const effect of transaction.effects) if (effect.is(sourceRevealEffect)) {
      const next = effect.value;
      value = next && next.from >= 0 && next.to >= next.from && next.to <= transaction.newDoc.length && transaction.newDoc.sliceString(next.from, next.to) === next.raw ? next : null;
    }
    return value;
  },
});
const revealDecorations = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(_value, transaction) {
    const reveal = transaction.state.field(revealField);
    if (!reveal) return Decoration.none;
    const ranges = [];
    if (reveal.from > 0) ranges.push(Decoration.replace({ inclusive: false }).range(0, reveal.from));
    if (reveal.to < transaction.newDoc.length) ranges.push(Decoration.replace({ inclusive: false }).range(reveal.to, transaction.newDoc.length));
    return Decoration.set(ranges);
  },
  provide: field => EditorView.decorations.from(field),
});

function localInput(transaction: Transaction) {
  const reveal = transaction.startState.field(revealField);
  if (!reveal || !transaction.docChanged || transaction.annotation(nativeEdit) || transaction.isUserEvent("undo") || transaction.isUserEvent("redo")) return transaction;
  let contained = true;
  transaction.changes.iterChangedRanges((from, to) => { if (from < reveal.from || to > reveal.to) contained = false; });
  return contained ? transaction : [];
}

/** The full buffer/history remain in state; only this proven source slot is drawn. */
export const sourceRevealExtensions: Extension = [revealField, revealDecorations, EditorState.transactionFilter.of(localInput), EditorView.atomicRanges.of(view => view.state.field(revealDecorations))];

const tokenEffect = StateEffect.define<{ tokens: SyntaxToken[]; styled: boolean }>();
const sourceDecorations = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(value, transaction) {
    // A parser result belongs to its exact source. Never map old syntax through
    // new source and present it as a current semantic decoration.
    if (transaction.docChanged) value = Decoration.none;
    for (const effect of transaction.effects) if (effect.is(tokenEffect)) {
      const marks: ReturnType<Decoration["range"]>[] = [];
      for (const token of effect.value.tokens) {
        const from = token.utf16From, to = token.utf16To;
        if (from < 0 || to > transaction.state.doc.length || from >= to) continue;
        const name = token.kind.toLowerCase();
        const kind = name.includes("heading") ? "heading" : name.includes("strong") ? "strong" : name.includes("emph") ? "emph" : name.includes("listitem") || name.includes("enumitem") ? "list" : name.includes("comment") ? "comment" : name.includes("math") ? "math" : name.includes("string") ? "string" : "";
        if (!kind) continue;
        if (kind === "heading" && effect.value.styled) {
          const line = transaction.state.doc.lineAt(from);
          marks.push(Decoration.line({ class: "cm-rendered-heading" }).range(line.from));
        }
        marks.push(Decoration.mark({ class: `cm-typst-${kind}${effect.value.styled ? " cm-rendered" : ""}` }).range(from, to));
      }
      value = Decoration.set(marks, true);
    }
    return value;
  },
  provide: field => EditorView.decorations.from(field),
});

/** Keep authored CR characters as source characters instead of normalizing EOL. */
export function createSourceState(source: string, extensions: Extension[] = []): EditorState {
  return EditorState.create({ doc: source, extensions: [EditorState.lineSeparator.of("\n"), ...extensions] });
}

export class SourceEditor {
  view: EditorView;
  private editable = new Compartment();
  private wrapped = new Compartment();
  private gutters = new Compartment();
  private listeners = new Set<(event: SourceEditorEvent) => void>();
  private inputComposition = false;
  generation = 0;

  constructor(source: string, onChange: (source: string) => void, save: () => void) {
    this.view = new EditorView({
      state: createSourceState(source, [
          history(), this.gutters.of(lineNumbers()), drawSelection(), highlightActiveLine(), highlightActiveLineGutter(),
          bracketMatching(), indentOnInput(), highlightSelectionMatches(), sourceDecorations, sourceRevealExtensions,
          this.editable.of(EditorView.editable.of(true)),
          this.wrapped.of(EditorView.lineWrapping),
          keymap.of([{ key: "Mod-s", run: () => { save(); return true; } }, { key: "Mod-Shift-z", run: redo, preventDefault: true }, ...defaultKeymap, ...historyKeymap, ...searchKeymap, indentWithTab]),
          EditorView.contentAttributes.of({ "aria-label": "Typst 源码编辑器", spellcheck: "false" }),
          EditorView.updateListener.of(update => {
            if (update.docChanged) { this.generation++; onChange(update.state.doc.toString()); this.emit("source"); }
            if (update.selectionSet) this.emit("selection");
            if (update.docChanged && update.startState.field(revealField) && !update.state.field(revealField)) {
              this.view.dom.classList.remove("cm-local-source");
              this.view.contentDOM.setAttribute("aria-label", "完整 Typst 源码编辑器，局部来源范围已变化");
              this.emit("reveal");
              queueMicrotask(() => { if (!this.revealed) this.view.dispatch({ effects: this.gutters.reconfigure(lineNumbers()) }); });
            }
          }),
          EditorView.domEventObservers({
            compositionstart: () => { this.inputComposition = true; this.emit("composition"); },
            compositionend: () => { this.inputComposition = false; queueMicrotask(() => this.emit("composition")); },
          }),
          EditorView.theme({
            "&": { height: "100%", fontSize: "15px" },
            ".cm-content": { fontFamily: "'Noto Sans Mono CJK SC', 'Noto Sans Mono CJK TC', 'DejaVu Sans Mono', monospace", padding: "24px 0 80px", lineHeight: "1.8", caretColor: "#426f68" },
            ".cm-line": { padding: "0 26px" },
            ".cm-gutters": { backgroundColor: "transparent", border: "none", color: "#a8aaa4", padding: "24px 0 0 10px" },
            ".cm-activeLine": { backgroundColor: "#f0f4ef80" },
            ".cm-activeLineGutter": { backgroundColor: "transparent", color: "#567a72" },
            ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": { background: "#cce2db !important" },
            "&.cm-focused": { outline: "none" },
            ".cm-scroller": { overflow: "auto" },
          }),
        ]),
    });
  }
  get source(): string { return this.view.state.doc.toString(); }
  get selection(): SourceSelection { const main = this.view.state.selection.main; return { from: main.from, to: main.to }; }
  get composing(): boolean { return this.inputComposition || this.view.compositionStarted || this.view.composing; }
  get revealed(): SourceReveal | null { return this.view.state.field(revealField); }
  observe(listener: (event: SourceEditorEvent) => void): () => void { this.listeners.add(listener); return () => this.listeners.delete(listener); }
  private emit(kind: SourceEditorEvent["kind"]): void { const event = { kind, source: this.source, generation: this.generation, selection: this.selection, composing: this.composing }; for (const listener of this.listeners) listener(event); }
  undo(): void { undo(this.view); }
  redo(): void { redo(this.view); }
  apply(from: number, to: number, text: string): void {
    this.applyPatches(this.source, [{ from, to, expected: this.source.slice(from, to), insert: text }], { from: from + text.length, to: from + text.length });
  }
  applyPatches(expectedSource: string, patches: readonly SourcePatch[], selection?: SourceSelection, userEvent = "input.native", isolated = true): void {
    if (this.source !== expectedSource) throw new Error("草稿已变化，未应用旧编辑提案。");
    const result = applySourcePatches(expectedSource, patches);
    const selected = selection ?? { from: patches[0]?.from ?? this.selection.from, to: patches[0]?.from ?? this.selection.to };
    if (!Number.isSafeInteger(selected.from) || !Number.isSafeInteger(selected.to) || selected.from < 0 || selected.to < selected.from || selected.to > result.length || !sourceBoundary(result, selected.from) || !sourceBoundary(result, selected.to)) throw new Error("原生编辑选区无效，未应用修改。");
    this.view.dispatch({ changes: [...patches].sort((a, b) => a.from - b.from).map(patch => ({ from: patch.from, to: patch.to, insert: patch.insert })), selection: { anchor: selected.from, head: selected.to }, userEvent, annotations: isolated ? [nativeEdit.of(true), isolateHistory.of("full")] : nativeEdit.of(true) });
  }
  insert(before: string, after = "", placeholder = ""): void {
    const selection = this.view.state.selection.main;
    const selected = this.view.state.sliceDoc(selection.from, selection.to);
    const body = selected || placeholder;
    this.view.dispatch({
      changes: { from: selection.from, to: selection.to, insert: before + body + after },
      selection: { anchor: selection.from + before.length, head: selection.from + before.length + body.length },
      userEvent: "input",
      annotations: isolateHistory.of("full"),
    });
    this.view.focus();
  }
  tokens(tokens: SyntaxToken[], styled: boolean): void { this.view.dispatch({ effects: tokenEffect.of({ tokens, styled }) }); }
  readOnly(value: boolean): void { this.view.dispatch({ effects: this.editable.reconfigure(EditorView.editable.of(!value)) }); }
  reveal(range: SourceReveal, expectedSource = this.source): boolean {
    if (this.composing || expectedSource !== this.source || !sourceBoundary(this.source, range.from) || !sourceBoundary(this.source, range.to) || range.from < 0 || range.to < range.from || range.to > this.source.length || this.source.slice(range.from, range.to) !== range.raw) return false;
    this.view.dispatch({ effects: [sourceRevealEffect.of(range), this.gutters.reconfigure([])], selection: { anchor: range.from, head: range.to } });
    this.view.dom.classList.add("cm-local-source");
    this.view.contentDOM.setAttribute("aria-label", "当前构造的 Typst 源码");
    return true;
  }
  clearReveal(): void {
    if (this.composing) return;
    this.view.dispatch({ effects: [sourceRevealEffect.of(null), this.gutters.reconfigure(lineNumbers())] });
    this.view.dom.classList.remove("cm-local-source");
    this.view.contentDOM.setAttribute("aria-label", "Typst 源码编辑器");
  }
  select(from: number, to: number, focus = true): void {
    this.view.dispatch({ selection: { anchor: from, head: to }, effects: EditorView.scrollIntoView(from, { y: "center" }) });
    if (focus) this.view.focus();
  }
  mount(parent: HTMLElement): void { parent.append(this.view.dom); this.view.requestMeasure(); }
}
