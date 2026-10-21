import { Annotation, Compartment, EditorState, Prec, StateEffect, StateField, type Extension, type Transaction } from "@codemirror/state";
import { Decoration, EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, drawSelection, type DecorationSet } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab, isolateHistory, undo, redo } from "@codemirror/commands";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { bracketMatching, indentOnInput } from "@codemirror/language";
import type { SyntaxToken } from "./types";
import { currentSourceStyleBasis, retireSourceStylesEffect, sourceStyleModeEffect, sourceStyles, sourceTokensEffect, type SourceStyleBasis } from "./source-style";
import { sourceCompositionHistory, SourceHistoryRequests, type SourceHistoryCommand } from "./source-history";
import { sourceCodePointCount, sourceLongLines } from "./source-statistics";

const boundedSourceShaping = EditorView.contentAttributes.compute([sourceLongLines], state => ({ class: state.field(sourceLongLines).length ? "cm-long-line" : "" }));

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

/** Keep authored CR characters as source characters instead of normalizing EOL. */
export function createSourceState(source: string, extensions: Extension[] = []): EditorState {
  return EditorState.create({ doc: source, extensions: [EditorState.lineSeparator.of("\n"), ...extensions] });
}

export class SourceEditor {
  view: EditorView;
  private editable = new Compartment();
  private gutters = new Compartment();
  private listeners = new Set<(event: SourceEditorEvent) => void>();
  private inputComposition = false;
  private pendingTokens?: { tokens: SyntaxToken[]; styled: boolean; basis: SourceStyleBasis };
  private styleNotice = document.createElement("span");
  private disposed = false;
  private sourceSnapshot?: { doc: EditorState["doc"]; source: string };
  private compositionEpoch = 0;
  private activeCompositionEpoch: number | null = null;
  private historyRequests = new SourceHistoryRequests({
    composing: () => this.inputComposition || !!this.view?.compositionStarted || !!this.view?.composing,
    execute: command => (command === "undo" ? undo : redo)(this.view),
    settled: () => {
      this.activeCompositionEpoch = null;
      // End the native unit before releasing mode/save or later ordinary
      // typing. This annotation neither changes source nor adds an Undo entry.
      this.view.dispatch({ annotations: isolateHistory.of("after") });
      this.flushTokens(); this.emit("composition");
    },
  });
  generation = 0;

  constructor(source: string, onChange: (source: string) => void, save: () => void) {
    this.view = new EditorView({
      state: createSourceState(source, [
          history(), sourceCodePointCount, sourceLongLines, this.gutters.of(lineNumbers()), drawSelection(), highlightActiveLine(), highlightActiveLineGutter(),
          bracketMatching(), indentOnInput(), highlightSelectionMatches(), sourceStyles, sourceRevealExtensions,
          sourceCompositionHistory(() => this.activeCompositionEpoch, transaction => !!transaction.annotation(nativeEdit)),
          this.editable.of(EditorView.editable.of(true)),
          EditorView.lineWrapping, boundedSourceShaping,
          keymap.of([{ key: "Mod-s", run: () => { save(); return true; } }, { key: "Mod-Shift-z", run: () => this.requestHistory("redo", true), preventDefault: true }, ...defaultKeymap,
            ...historyKeymap.map(binding => ({ ...binding, run: binding.run === undo ? () => this.requestHistory("undo", true) : binding.run === redo ? () => this.requestHistory("redo", true) : binding.run })), ...searchKeymap, indentWithTab]),
          EditorView.contentAttributes.of({ "aria-label": "Typst 源码编辑器", spellcheck: "false" }),
          Prec.highest(EditorView.domEventHandlers({
            beforeinput: (event, view) => {
              const command = event.inputType === "historyUndo" ? "undo" : event.inputType === "historyRedo" ? "redo" : null;
              if (!command) return false;
              // history() otherwise dispatches immediately, even while the
              // final composition DOM mutation is still waiting to be read.
              event.preventDefault();
              if (!view.state.readOnly && view.state.facet(EditorView.editable)) {
                if (event.isComposing && this.activeCompositionEpoch === null) this.activeCompositionEpoch = ++this.compositionEpoch;
                this.requestHistory(command, true, event.isComposing);
              }
              return true;
            },
          })),
          EditorView.domEventHandlers({
            beforeinput: (event, view) => {
              // A large native contenteditable insertion builds thousands of
              // DOM lines before CodeMirror can read it back. Use the normal
              // selection transaction while retaining the complete author text.
              // Composition still belongs to the browser/input method.
              if (event.inputType !== "insertText" || !event.data || event.data.length < 64 * 1024 || !event.cancelable || event.defaultPrevented || event.isComposing || this.composing || view.state.readOnly || !view.state.facet(EditorView.editable)) return false;
              event.preventDefault();
              view.dispatch({ ...view.state.replaceSelection(event.data), userEvent: "input.type", scrollIntoView: true, annotations: isolateHistory.of("full") });
              return true;
            },
          }),
          EditorView.updateListener.of(update => {
            const styleStatus = update.state.field(sourceStyles).status;
            this.styleNotice.hidden = styleStatus !== "pending";
            if (update.docChanged) { this.pendingTokens = undefined; this.generation++; onChange(this.source); this.emit("source"); }
            if (update.selectionSet) this.emit("selection");
            if (update.docChanged && update.startState.field(revealField) && !update.state.field(revealField)) {
              this.view.dom.classList.remove("cm-local-source");
              this.view.contentDOM.setAttribute("aria-label", "完整 Typst 源码编辑器，局部来源范围已变化");
              this.emit("reveal");
              queueMicrotask(() => { if (!this.revealed) this.view.dispatch({ effects: this.gutters.reconfigure(lineNumbers()) }); });
            }
          }),
          EditorView.domEventObservers({
            compositionstart: () => { this.inputComposition = true; this.activeCompositionEpoch = ++this.compositionEpoch; this.historyRequests.compositionStart(); this.emit("composition"); },
            compositionend: () => {
              this.inputComposition = false;
              // Keep outside mode/save actions waiting through CM's final DOM
              // microtask, then apply queued history before releasing them.
              this.historyRequests.compositionEnd();
            },
          }),
          EditorView.theme({
            "&": { height: "100%", fontSize: "15px" },
            ".cm-content": { fontFamily: "'Noto Sans Mono CJK SC', 'Noto Sans Mono CJK TC', 'DejaVu Sans Mono', monospace", padding: "24px 0 80px", lineHeight: "1.8", caretColor: "#426f68" },
            ".cm-line": { padding: "0 26px" },
            ".cm-content.cm-long-line": { fontKerning: "none" },
            ".cm-gutters": { backgroundColor: "transparent", border: "none", color: "#a8aaa4", padding: "24px 0 0 10px" },
            ".cm-activeLine": { backgroundColor: "#f0f4ef80" },
            ".cm-activeLineGutter": { backgroundColor: "transparent", color: "#567a72" },
            ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": { background: "#cce2db !important" },
            "&.cm-focused": { outline: "none" },
            ".cm-scroller": { overflow: "auto" },
            ".cm-source-style-notice": { position: "absolute", top: "3px", right: "12px", fontSize: "11px", lineHeight: "18px", color: "#687263", backgroundColor: "#fafcf8", pointerEvents: "none", zIndex: "2" },
          }),
        ]),
    });
    this.styleNotice.className = "cm-source-style-notice";
    this.styleNotice.textContent = "样式更新中 · 暂存上次外观";
    this.styleNotice.setAttribute("role", "note");
    this.styleNotice.hidden = true;
    this.view.dom.append(this.styleNotice);
  }
  get source(): string {
    const doc = this.view.state.doc;
    if (this.sourceSnapshot?.doc !== doc) this.sourceSnapshot = { doc, source: doc.toString() };
    return this.sourceSnapshot.source;
  }
  get characterCount(): number { return this.view.state.field(sourceCodePointCount); }
  get selection(): SourceSelection { const main = this.view.state.selection.main; return { from: main.from, to: main.to }; }
  get composing(): boolean { return this.inputComposition || this.view.compositionStarted || this.view.composing || this.historyRequests.composing; }
  get revealed(): SourceReveal | null { return this.view.state.field(revealField); }
  observe(listener: (event: SourceEditorEvent) => void): () => void { this.listeners.add(listener); return () => this.listeners.delete(listener); }
  private emit(kind: SourceEditorEvent["kind"]): void { const event = { kind, source: this.source, generation: this.generation, selection: this.selection, composing: this.composing }; for (const listener of this.listeners) listener(event); }
  undo(): void { this.requestHistory("undo"); }
  redo(): void { this.requestHistory("redo"); }
  private requestHistory(command: SourceHistoryCommand, sourceSurface = false, composingHint = false): boolean {
    if (this.disposed || sourceSurface && (this.view.state.readOnly || !this.view.state.facet(EditorView.editable))) return false;
    return this.historyRequests.request(command, composingHint);
  }
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
  syntaxBasis(): SourceStyleBasis { return currentSourceStyleBasis(this.view.state); }
  tokens(tokens: SyntaxToken[], styled: boolean, basis: SourceStyleBasis): boolean {
    const current = this.syntaxBasis();
    if (this.disposed || basis.source !== current.source || basis.generation !== current.generation || basis.epoch !== current.epoch) return false;
    if (this.composing) { this.pendingTokens = { tokens, styled, basis }; return true; }
    this.pendingTokens = undefined;
    this.view.dispatch({ effects: sourceTokensEffect.of({ tokens, styled, basis }) });
    return true;
  }
  clearTokens(): void { this.pendingTokens = undefined; this.historyRequests.retire(); this.view.dispatch({ effects: retireSourceStylesEffect.of(null) }); }
  styleMode(styled: boolean): void {
    if (this.pendingTokens) this.pendingTokens.styled = styled;
    this.view.dispatch({ effects: sourceStyleModeEffect.of(styled) });
  }
  private flushTokens(): void {
    const pending = this.pendingTokens;
    if (pending && !this.composing && !this.disposed) this.tokens(pending.tokens, pending.styled, pending.basis);
  }
  destroy(): void { this.disposed = true; this.pendingTokens = undefined; this.historyRequests.dispose(); this.view.destroy(); }
  readOnly(value: boolean): void { if (value && this.view.state.facet(EditorView.editable)) this.historyRequests.retire(); this.view.dispatch({ effects: this.editable.reconfigure(EditorView.editable.of(!value)) }); }
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
