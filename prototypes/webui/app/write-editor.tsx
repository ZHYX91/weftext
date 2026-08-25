"use client";

import { useLayoutEffect, useRef } from "react";
import { RangeSetBuilder, StateEffect, StateField } from "@codemirror/state";
import { Decoration, EditorView, keymap, type DecorationSet, type ViewUpdate } from "@codemirror/view";
import { basicSetup } from "codemirror";
import { Prec } from "@codemirror/state";
import type { DocumentModel, DocumentProfileId } from "./document-contract";

type WriteEditorProps = {
  profile?: DocumentProfileId;
  source: string;
  model: DocumentModel;
  selectionStart: number;
  selectionEnd: number;
  scrollTop: number;
  restoreToken: number;
  onChange(source: string): void;
  onSelectionChange(start: number, end: number): void;
  onScroll(scrollTop: number): void;
  onKeyDown(event: KeyboardEvent): void;
  onFind(replace: boolean): void;
};

type ExactBuffer = {
  source: string;
  prefix: string;
  body: string;
  editorText: string;
  editorToBody: number[];
  bodyToEditor: number[];
  lineEnding: "\n" | "\r\n" | "\r";
};

type WriteBinding = { view: EditorView; buffer: { current: ExactBuffer } };
const writeBindings = new WeakMap<Element, WriteBinding>();
const setSemanticDecorations = StateEffect.define<DecorationSet>();
const semanticDecorations = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(value, transaction) {
    for (const effect of transaction.effects) {
      if (effect.is(setSemanticDecorations)) return effect.value;
    }
    return value.map(transaction.changes);
  },
  provide: (field) => EditorView.decorations.from(field),
});

function bodyStart(model: DocumentModel) {
  return model.blocks.find((block) => block.kind === "frontmatter")?.end ?? 0;
}

function preferredLineEnding(source: string): ExactBuffer["lineEnding"] {
  const crlf = source.indexOf("\r\n");
  const lf = source.indexOf("\n");
  const cr = source.indexOf("\r");
  if (crlf !== -1 && (lf === -1 || crlf <= lf) && (cr === -1 || crlf <= cr)) return "\r\n";
  if (cr !== -1 && (lf === -1 || cr < lf)) return "\r";
  return "\n";
}

function exactBuffer(source: string, model: DocumentModel): ExactBuffer {
  const start = Math.max(0, Math.min(bodyStart(model), source.length));
  const prefix = source.slice(0, start);
  const body = source.slice(start);
  const editorToBody = [0];
  const bodyToEditor = new Array<number>(body.length + 1).fill(0);
  const chunks: string[] = [];
  let bodyOffset = 0;
  let editorOffset = 0;
  while (bodyOffset < body.length) {
    bodyToEditor[bodyOffset] = editorOffset;
    if (body[bodyOffset] === "\r") {
      const crlf = body[bodyOffset + 1] === "\n";
      chunks.push("\n");
      if (crlf) bodyToEditor[bodyOffset + 1] = editorOffset;
      bodyOffset += crlf ? 2 : 1;
    } else {
      chunks.push(body[bodyOffset]);
      bodyOffset += 1;
    }
    editorOffset += 1;
    editorToBody[editorOffset] = bodyOffset;
    bodyToEditor[bodyOffset] = editorOffset;
  }
  return {
    source,
    prefix,
    body,
    editorText: chunks.join(""),
    editorToBody,
    bodyToEditor,
    lineEnding: preferredLineEnding(source),
  };
}

function editorOffset(buffer: ExactBuffer, sourceOffset: number) {
  const bodyOffset = Math.max(0, Math.min(sourceOffset - buffer.prefix.length, buffer.body.length));
  return buffer.bodyToEditor[bodyOffset] ?? 0;
}

function sourceOffset(buffer: ExactBuffer, editorPosition: number) {
  const bodyOffset = buffer.editorToBody[Math.max(0, Math.min(editorPosition, buffer.editorText.length))] ?? buffer.body.length;
  return buffer.prefix.length + bodyOffset;
}

function applyChanges(buffer: ExactBuffer, update: ViewUpdate) {
  const edits: Array<{ from: number; to: number; replacement: string }> = [];
  update.changes.iterChanges((fromA, toA, _fromB, _toB, inserted) => {
    const replacement = buffer.lineEnding === "\n" ? inserted.toString() : inserted.toString().replaceAll("\n", buffer.lineEnding);
    edits.push({
      from: sourceOffset(buffer, fromA),
      to: sourceOffset(buffer, toA),
      replacement,
    });
  });
  let source = buffer.source;
  for (const edit of edits.reverse()) {
    source = `${source.slice(0, edit.from)}${edit.replacement}${source.slice(edit.to)}`;
  }
  return source;
}

function decorations(view: EditorView, buffer: ExactBuffer, model: DocumentModel) {
  const builder = new RangeSetBuilder<Decoration>();
  const seen = new Set<number>();
  for (const block of model.blocks) {
    if (block.kind === "frontmatter" || block.end <= buffer.prefix.length) continue;
    const position = editorOffset(buffer, Math.max(block.start, buffer.prefix.length));
    const lineStart = view.state.doc.lineAt(position).from;
    if (seen.has(lineStart)) continue;
    seen.add(lineStart);
    const level = block.kind === "heading" ? ` level-${block.headingLevel ?? 1}` : "";
    const quote = block.kind === "quote" ? ` quote-depth-${Math.min(block.quoteDepth ?? 1, 9)}` : "";
    const attributes = block.kind === "quote" ? { "data-quote-depth": String(block.quoteDepth ?? 1) } : undefined;
    builder.add(lineStart, lineStart, Decoration.line({ class: `weftext-block weftext-block-${block.kind}${level}${quote}`, attributes }));
  }
  return builder.finish();
}

function restore(view: EditorView, buffer: ExactBuffer, start: number, end: number, scrollTop: number) {
  view.dispatch({ selection: { anchor: editorOffset(buffer, start), head: editorOffset(buffer, end) } });
  view.scrollDOM.scrollTop = Math.max(0, scrollTop);
  view.focus();
}

export function writeEditorValue(element: Element) {
  return writeBindings.get(element)?.buffer.current.source ?? "";
}

export function setWriteEditorSelection(element: Element, start: number, end = start) {
  const binding = writeBindings.get(element);
  if (!binding) throw new Error("Write editor is not mounted");
  restore(binding.view, binding.buffer.current, start, end, binding.view.scrollDOM.scrollTop);
}

export function replaceWriteEditorSelection(element: Element, replacement: string) {
  const binding = writeBindings.get(element);
  if (!binding) throw new Error("Write editor is not mounted");
  const selection = binding.view.state.selection.main;
  const insert = replacement.replace(/\r\n?/g, "\n");
  binding.view.dispatch({
    changes: { from: selection.from, to: selection.to, insert },
    selection: { anchor: selection.from + insert.length },
  });
}

export function writeEditorSelection(element: Element) {
  const binding = writeBindings.get(element);
  const selection = binding?.view.state.selection.main;
  if (!binding || !selection) return { start: 0, end: 0 };
  return {
    start: sourceOffset(binding.buffer.current, selection.from),
    end: sourceOffset(binding.buffer.current, selection.to),
  };
}

export default function WriteEditor(props: WriteEditorProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const bufferRef = useRef(exactBuffer(props.source, props.model));
  const callbacksRef = useRef(props);
  const applyingExternalRef = useRef(false);
  const restoredTokenRef = useRef(-1);

  useLayoutEffect(() => { callbacksRef.current = props; });

  useLayoutEffect(() => {
    if (!hostRef.current) return;
    const initial = callbacksRef.current;
    const profileName = "AsciiDoc";
    const view = new EditorView({
      parent: hostRef.current,
      doc: bufferRef.current.editorText,
      selection: {
        anchor: editorOffset(bufferRef.current, initial.selectionStart),
        head: editorOffset(bufferRef.current, initial.selectionEnd),
      },
      extensions: [
        basicSetup,
        semanticDecorations,
        EditorView.lineWrapping,
        EditorView.contentAttributes.of({
          "aria-label": `${profileName} 正文`,
          "aria-description": `由 Rust Core ${profileName} 块模型驱动的结构化写作编辑器`,
          autocapitalize: "sentences",
          spellcheck: "true",
        }),
        EditorView.theme({
          "&": { minHeight: "500px" },
          ".cm-content": { minHeight: "500px" },
          ".cm-gutters": { display: "none" },
        }),
        Prec.highest(keymap.of([
          { key: "Mod-f", run: () => { callbacksRef.current.onFind(false); return true; } },
          { key: "Mod-h", run: () => { callbacksRef.current.onFind(true); return true; } },
        ])),
        EditorView.domEventHandlers({
          keydown(event) {
            callbacksRef.current.onKeyDown(event);
            return event.defaultPrevented;
          },
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged && !applyingExternalRef.current) {
            const source = applyChanges(bufferRef.current, update);
            bufferRef.current = exactBuffer(source, callbacksRef.current.model);
            callbacksRef.current.onChange(source);
          }
          if (update.selectionSet || update.docChanged) {
            const selection = update.state.selection.main;
            callbacksRef.current.onSelectionChange(
              sourceOffset(bufferRef.current, selection.from),
              sourceOffset(bufferRef.current, selection.to),
            );
          }
        }),
      ],
    });
    viewRef.current = view;
    writeBindings.set(view.contentDOM, { view, buffer: bufferRef });
    const onScroll = () => callbacksRef.current.onScroll(view.scrollDOM.scrollTop);
    view.scrollDOM.addEventListener("scroll", onScroll, { passive: true });
    view.dispatch({ effects: setSemanticDecorations.of(decorations(view, bufferRef.current, initial.model)) });
    return () => {
      view.scrollDOM.removeEventListener("scroll", onScroll);
      writeBindings.delete(view.contentDOM);
      view.destroy();
      viewRef.current = null;
    };
  }, []);

  useLayoutEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    if (bufferRef.current.source !== props.source) {
      const next = exactBuffer(props.source, props.model);
      applyingExternalRef.current = true;
      bufferRef.current = next;
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: next.editorText } });
      applyingExternalRef.current = false;
    } else {
      bufferRef.current = exactBuffer(props.source, props.model);
    }
    view.dispatch({ effects: setSemanticDecorations.of(decorations(view, bufferRef.current, props.model)) });
  }, [props.model, props.source]);

  useLayoutEffect(() => {
    const view = viewRef.current;
    if (!view || restoredTokenRef.current === props.restoreToken) return;
    restoredTokenRef.current = props.restoreToken;
    restore(view, bufferRef.current, props.selectionStart, props.selectionEnd, props.scrollTop);
  }, [props.restoreToken, props.scrollTop, props.selectionEnd, props.selectionStart]);

  return <section className="write-editor-shell" aria-label="结构化 Write 编辑器">
    <div ref={hostRef} className="write-editor" />
  </section>;
}
