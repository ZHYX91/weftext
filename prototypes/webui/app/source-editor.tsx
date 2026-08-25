"use client";

import { useLayoutEffect, useRef, useState } from "react";
import {
  foldEffect,
  foldedRanges,
  foldService,
  unfoldEffect,
} from "@codemirror/language";
import { Prec } from "@codemirror/state";
import { EditorView, keymap, type ViewUpdate } from "@codemirror/view";
import { basicSetup } from "codemirror";
import type { DocumentProfileId } from "./document-contract";

type SourceEditorProps = {
  profile?: DocumentProfileId;
  value: string;
  selectionStart: number;
  selectionEnd: number;
  scrollTop: number;
  restoreToken: number;
  onChange(value: string): void;
  onSelectionChange(start: number, end: number): void;
  onScroll(scrollTop: number): void;
  onKeyDown(event: KeyboardEvent): void;
  onFind(replace: boolean): void;
  stateKey?: string;
};

type SourceBuffer = {
  source: string;
  editorText: string;
  editorToSource: number[];
  sourceToEditor: number[];
  preferredLineEnding: "\n" | "\r\n" | "\r";
};

type SystemMetadataFold = {
  headerStart: number;
  from: number;
  to: number;
};

type EditorBinding = {
  view: EditorView;
  buffer: { current: SourceBuffer };
};

const editorBindings = new WeakMap<Element, EditorBinding>();
const foldStateStorageKey = "weftext.source-fold-state.v1";

function systemMetadataExpanded(stateKey?: string) {
  if (!stateKey) return false;
  try {
    const states = JSON.parse(window.localStorage.getItem(foldStateStorageKey) ?? "{}") as Record<string, boolean>;
    return states[stateKey] === true;
  } catch {
    return false;
  }
}

function rememberSystemMetadataExpanded(stateKey: string | undefined, expanded: boolean) {
  if (!stateKey) return;
  try {
    const states = JSON.parse(window.localStorage.getItem(foldStateStorageKey) ?? "{}") as Record<string, boolean>;
    window.localStorage.setItem(foldStateStorageKey, JSON.stringify({ ...states, [stateKey]: expanded }));
  } catch {
    // Fold continuity is best-effort device state and never changes source bytes.
  }
}

function normalizeSource(source: string): SourceBuffer {
  const editorToSource = [0];
  const sourceToEditor = new Array<number>(source.length + 1).fill(0);
  const chunks: string[] = [];
  let sourceOffset = 0;
  let editorOffset = 0;
  let preferredLineEnding: SourceBuffer["preferredLineEnding"] = "\n";
  let foundLineEnding = false;

  while (sourceOffset < source.length) {
    sourceToEditor[sourceOffset] = editorOffset;
    const character = source[sourceOffset];
    if (character === "\r") {
      const isCrLf = source[sourceOffset + 1] === "\n";
      if (!foundLineEnding) {
        preferredLineEnding = isCrLf ? "\r\n" : "\r";
        foundLineEnding = true;
      }
      chunks.push("\n");
      if (isCrLf) sourceToEditor[sourceOffset + 1] = editorOffset;
      sourceOffset += isCrLf ? 2 : 1;
    } else {
      if (character === "\n" && !foundLineEnding) {
        preferredLineEnding = "\n";
        foundLineEnding = true;
      }
      chunks.push(character);
      sourceOffset += 1;
    }
    editorOffset += 1;
    editorToSource[editorOffset] = sourceOffset;
    sourceToEditor[sourceOffset] = editorOffset;
  }

  return {
    source,
    editorText: chunks.join(""),
    editorToSource,
    sourceToEditor,
    preferredLineEnding,
  };
}

function editorOffset(buffer: SourceBuffer, sourceOffset: number) {
  return buffer.sourceToEditor[Math.max(0, Math.min(sourceOffset, buffer.source.length))] ?? 0;
}

function sourceOffset(buffer: SourceBuffer, editorPosition: number) {
  return buffer.editorToSource[Math.max(0, Math.min(editorPosition, buffer.editorText.length))] ?? buffer.source.length;
}

function replaceEditorNewlines(value: string, lineEnding: SourceBuffer["preferredLineEnding"]) {
  return lineEnding === "\n" ? value : value.replaceAll("\n", lineEnding);
}

function applyEditorChanges(buffer: SourceBuffer, update: ViewUpdate) {
  const edits: Array<{ from: number; to: number; replacement: string }> = [];
  update.changes.iterChanges((fromA, toA, _fromB, _toB, inserted) => {
    edits.push({
      from: sourceOffset(buffer, fromA),
      to: sourceOffset(buffer, toA),
      replacement: replaceEditorNewlines(inserted.toString(), buffer.preferredLineEnding),
    });
  });
  let source = buffer.source;
  for (const edit of edits.reverse()) {
    source = `${source.slice(0, edit.from)}${edit.replacement}${source.slice(edit.to)}`;
  }
  return source;
}

function lines(source: string) {
  const result: Array<{ from: number; to: number; text: string }> = [];
  let from = 0;
  while (from <= source.length) {
    const newline = source.indexOf("\n", from);
    const to = newline === -1 ? source.length : newline;
    result.push({ from, to, text: source.slice(from, to) });
    if (newline === -1) break;
    from = newline + 1;
  }
  return result;
}

export function systemMetadataFoldRange(source: string): SystemMetadataFold | null {
  const sourceLines = lines(source);
  if (!sourceLines.length || sourceLines[0].text.replace(/^\uFEFF/, "") !== "---") return null;

  let candidate: SystemMetadataFold | null = null;
  for (let index = 1; index < sourceLines.length; index += 1) {
    const line = sourceLines[index];
    if (line.text === "---" || line.text === "...") break;
    if (!/^weftext:[ \t]*(?:#.*)?$/.test(line.text)) continue;
    if (candidate) return null;

    let childEnd = line.to;
    let hasNestedContent = false;
    for (let childIndex = index + 1; childIndex < sourceLines.length; childIndex += 1) {
      const child = sourceLines[childIndex];
      if (child.text === "---" || child.text === "...") break;
      if (child.text.length > 0 && !/^[ \t]/.test(child.text)) break;
      childEnd = child.to;
      if (/^[ \t]+\S/.test(child.text)) hasNestedContent = true;
    }
    if (hasNestedContent && childEnd > line.to) {
      candidate = { headerStart: line.from, from: line.to, to: childEnd };
    }
  }
  return candidate;
}

function isSystemFolded(view: EditorView) {
  const range = systemMetadataFoldRange(view.state.doc.toString());
  if (!range) return false;
  let folded = false;
  foldedRanges(view.state).between(range.from, range.to, (from, to) => {
    if (from <= range.from && to >= range.to) folded = true;
  });
  return folded;
}

function foldSystemMetadata(view: EditorView) {
  const range = systemMetadataFoldRange(view.state.doc.toString());
  if (range && !isSystemFolded(view)) {
    view.dispatch({ effects: foldEffect.of({ from: range.from, to: range.to }) });
  }
}

function restoreSelection(view: EditorView, buffer: SourceBuffer, start: number, end: number, scrollTop: number) {
  const range = systemMetadataFoldRange(view.state.doc.toString());
  const anchor = editorOffset(buffer, start);
  const head = editorOffset(buffer, end);
  if (range && ((anchor > range.from && anchor <= range.to) || (head > range.from && head <= range.to))) {
    view.dispatch({ effects: unfoldEffect.of({ from: range.from, to: range.to }) });
    return restoreSelectionAfterUnfold(view, anchor, head, scrollTop, true);
  }
  return restoreSelectionAfterUnfold(view, anchor, head, scrollTop, false);
}

function restoreSelectionAfterUnfold(view: EditorView, anchor: number, head: number, scrollTop: number, unfolded: boolean) {
  view.dispatch({ selection: { anchor, head } });
  view.scrollDOM.scrollTop = Math.max(0, scrollTop);
  view.focus();
  return unfolded;
}

/** Test-facing helpers exercise the real editor transaction boundary without DOM text fabrication. */
export function sourceEditorValue(element: Element) {
  return editorBindings.get(element)?.buffer.current.source ?? "";
}

export function replaceSourceEditorValue(element: Element, value: string) {
  const binding = editorBindings.get(element);
  if (!binding) throw new Error("Source editor is not mounted");
  binding.view.dispatch({
    changes: { from: 0, to: binding.view.state.doc.length, insert: normalizeSource(value).editorText },
  });
}

export function setSourceEditorSelection(element: Element, start: number, end = start) {
  const binding = editorBindings.get(element);
  if (!binding) throw new Error("Source editor is not mounted");
  restoreSelection(binding.view, binding.buffer.current, start, end, binding.view.scrollDOM.scrollTop);
}

export function replaceSourceEditorSelection(element: Element, replacement: string) {
  const binding = editorBindings.get(element);
  if (!binding) throw new Error("Source editor is not mounted");
  const selection = binding.view.state.selection.main;
  binding.view.dispatch({
    changes: {
      from: selection.from,
      to: selection.to,
      insert: normalizeSource(replacement).editorText,
    },
  });
}

export function sourceEditorSelection(element: Element) {
  const binding = editorBindings.get(element);
  if (!binding) return { start: 0, end: 0 };
  const selection = binding.view.state.selection.main;
  return {
    start: sourceOffset(binding.buffer.current, selection.from),
    end: sourceOffset(binding.buffer.current, selection.to),
  };
}

export function setSourceEditorScroll(element: Element, scrollTop: number) {
  const binding = editorBindings.get(element);
  if (!binding) throw new Error("Source editor is not mounted");
  binding.view.scrollDOM.scrollTop = scrollTop;
  binding.view.scrollDOM.dispatchEvent(new Event("scroll"));
}

export default function SourceEditor(props: SourceEditorProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const bufferRef = useRef(normalizeSource(props.value));
  const callbacksRef = useRef(props);
  const applyingExternalRef = useRef(false);
  const restoredTokenRef = useRef(-1);
  const [systemFolded, setSystemFolded] = useState(false);
  const hasSystemMetadata = systemMetadataFoldRange(props.value.replace(/\r\n?/g, "\n")) !== null;

  useLayoutEffect(() => {
    callbacksRef.current = props;
  });

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
        EditorView.lineWrapping,
        EditorView.contentAttributes.of({
          "aria-label": `${profileName} 源码`,
          "aria-description": `精确 ${profileName} 源码编辑器，系统元数据默认折叠`,
          autocapitalize: "off",
          autocomplete: "off",
          spellcheck: "false",
        }),
        EditorView.theme({
          "&": { minHeight: "500px" },
          ".cm-content": { minHeight: "500px" },
        }),
        foldService.of((state, lineStart, lineEnd) => {
          const range = systemMetadataFoldRange(state.doc.toString());
          return range && lineStart <= range.headerStart && lineEnd >= range.headerStart
            ? { from: range.from, to: range.to }
            : null;
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
            const nextSource = applyEditorChanges(bufferRef.current, update);
            const nextBuffer = normalizeSource(nextSource);
            bufferRef.current = nextBuffer;
            callbacksRef.current.onChange(nextSource);
          }
          if (update.selectionSet || update.docChanged) {
            const selection = update.state.selection.main;
            callbacksRef.current.onSelectionChange(
              sourceOffset(bufferRef.current, selection.from),
              sourceOffset(bufferRef.current, selection.to),
            );
          }
          setSystemFolded((current) => {
            const next = isSystemFolded(update.view);
            return current === next ? current : next;
          });
        }),
      ],
    });
    viewRef.current = view;
    editorBindings.set(view.contentDOM, { view, buffer: bufferRef });
    const onScroll = () => callbacksRef.current.onScroll(view.scrollDOM.scrollTop);
    view.scrollDOM.addEventListener("scroll", onScroll, { passive: true });
    if (!systemMetadataExpanded(initial.stateKey)) foldSystemMetadata(view);
    setSystemFolded(isSystemFolded(view));
    return () => {
      view.scrollDOM.removeEventListener("scroll", onScroll);
      editorBindings.delete(view.contentDOM);
      view.destroy();
      viewRef.current = null;
    };
  }, []);

  useLayoutEffect(() => {
    const view = viewRef.current;
    if (!view || bufferRef.current.source === props.value) return;
    const nextBuffer = normalizeSource(props.value);
    applyingExternalRef.current = true;
    bufferRef.current = nextBuffer;
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: nextBuffer.editorText } });
    applyingExternalRef.current = false;
    if (!systemMetadataExpanded(props.stateKey)) foldSystemMetadata(view);
  }, [props.stateKey, props.value]);

  useLayoutEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    const range = systemMetadataFoldRange(view.state.doc.toString());
    if (!range) return;
    if (systemMetadataExpanded(props.stateKey)) {
      view.dispatch({ effects: unfoldEffect.of({ from: range.from, to: range.to }) });
    } else {
      foldSystemMetadata(view);
    }
  }, [props.stateKey]);

  useLayoutEffect(() => {
    const view = viewRef.current;
    if (!view || restoredTokenRef.current === props.restoreToken) return;
    restoredTokenRef.current = props.restoreToken;
    if (restoreSelection(view, bufferRef.current, props.selectionStart, props.selectionEnd, props.scrollTop)) {
      rememberSystemMetadataExpanded(props.stateKey, true);
    }
  }, [props.restoreToken, props.scrollTop, props.selectionEnd, props.selectionStart, props.stateKey]);

  function toggleSystemMetadata() {
    const view = viewRef.current;
    if (!view) return;
    const range = systemMetadataFoldRange(view.state.doc.toString());
    if (!range) return;
    view.dispatch({ effects: (systemFolded ? unfoldEffect : foldEffect).of({ from: range.from, to: range.to }) });
    rememberSystemMetadataExpanded(props.stateKey, systemFolded);
    view.focus();
  }

  const profileName = "AsciiDoc";
  return <section className="source-editor-shell" aria-label={`${profileName} Source 编辑器`}>
    <header className="source-editor-heading">
      <span>{profileName} Source</span>
      {hasSystemMetadata && <button type="button" onClick={toggleSystemMetadata} aria-expanded={!systemFolded}>
        {systemFolded ? "展开系统元数据" : "折叠系统元数据"}
      </button>}
    </header>
    <div ref={hostRef} className="source-editor" />
  </section>;
}
