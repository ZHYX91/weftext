import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import WriteEditor, {
  replaceWriteEditorSelection,
  setWriteEditorSelection,
  writeEditorSelection,
  writeEditorValue,
} from "../app/write-editor";
import type { DocumentBlock, DocumentModel } from "../app/document-contract";

afterEach(cleanup);
const noOperation = () => undefined;

function editorModel(seeds: Array<Partial<DocumentBlock> & Pick<DocumentBlock, "kind" | "start" | "end">>): DocumentModel {
  const blocks = seeds.map((seed): DocumentBlock => {
    const headingLevel = seed.headingLevel ?? null;
    const quoteDepth = seed.quoteDepth ?? null;
    const semantic = seed.kind === "heading"
      ? { kind: "heading" as const, level: headingLevel ?? 1 }
      : seed.kind === "quote"
        ? { kind: "quote" as const, depth: quoteDepth, attribution: null, citation: null }
        : seed.kind === "list"
          ? { kind: "list" as const, model: { kind: "unordered" as const, depth: 1, items: [] } }
          : { kind: seed.kind } as DocumentBlock["semantic"];
    return {
      textStart: seed.start,
      textEnd: seed.end,
      text: "",
      blockId: null,
      roles: [],
      title: null,
      ...seed,
      headingLevel,
      quoteDepth,
      semantic: seed.semantic ?? semantic,
    };
  });
  return { semanticModelVersion: 1, status: "complete", blocks, inlines: [], runInGroups: [], diagnostics: [], degradations: [], safeHtml: "" };
}

describe("structured Write editor", () => {
  it("hides frontmatter while exact edits preserve CRLF and the reserved mapping", () => {
    const prefix = "---\r\nweftext:\r\n  id: \"550e8400-e29b-41d4-a716-446655440000\"\r\n  icon: \"😀\"\r\n---\r\n";
    const source = `${prefix}= 标题\r\n正文\r\n`;
    const bodyStart = prefix.length;
    let changed = "";
    render(<WriteEditor
      source={source}
      model={editorModel([
        { kind: "frontmatter", start: 0, end: bodyStart, headingLevel: null },
        { kind: "heading", start: bodyStart, end: bodyStart + 6, headingLevel: 1 },
        { kind: "paragraph", start: source.indexOf("正文"), end: source.length, headingLevel: null },
      ])}
      selectionStart={source.indexOf("正文")}
      selectionEnd={source.indexOf("正文") + 2}
      scrollTop={0}
      restoreToken={1}
      onChange={(value) => { changed = value; }}
      onSelectionChange={noOperation}
      onScroll={noOperation}
      onKeyDown={noOperation}
      onFind={noOperation}
    />);
    const editor = screen.getByRole("textbox", { name: "AsciiDoc 正文" });
    expect(editor.textContent).not.toContain("weftext:");
    expect(writeEditorValue(editor)).toBe(source);
    act(() => {
      setWriteEditorSelection(editor, source.indexOf("正文"), source.indexOf("正文") + 2);
      replaceWriteEditorSelection(editor, "内容");
    });
    expect(changed).toBe(source.replace("正文", "内容"));
    expect(changed.startsWith(prefix)).toBe(true);
    expect(changed.split("\r\n")).toHaveLength(8);
    expect(writeEditorSelection(editor).start).toBe(source.indexOf("正文") + 2);
  });

  it("uses Core block kinds for semantic presentation without a frontend syntax parser", () => {
    const source = "========== Heading\n\n* item\n\n>>>>>>>>>>>> quote\n";
    render(<WriteEditor
      source={source}
      model={editorModel([
        { kind: "heading", start: 0, end: 19, headingLevel: 9 },
        { kind: "list", start: 20, end: 27, headingLevel: null },
        { kind: "quote", start: 28, end: source.length, headingLevel: null, quoteDepth: 12 },
      ])}
      selectionStart={0}
      selectionEnd={0}
      scrollTop={0}
      restoreToken={1}
      onChange={noOperation}
      onSelectionChange={noOperation}
      onScroll={noOperation}
      onKeyDown={noOperation}
      onFind={noOperation}
    />);
    const editor = screen.getByRole("textbox", { name: "AsciiDoc 正文" });
    expect(editor.closest(".write-editor")?.querySelector(".weftext-block-heading.level-9")).toBeTruthy();
    expect(editor.closest(".write-editor")?.querySelector(".weftext-block-list")).toBeTruthy();
    const quote = editor.closest(".write-editor")?.querySelector(".weftext-block-quote.quote-depth-9");
    expect(quote).toBeTruthy();
    expect(quote?.getAttribute("data-quote-depth")).toBe("12");
  });
});
