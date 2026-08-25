import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import SourceEditor, {
  replaceSourceEditorSelection,
  setSourceEditorSelection,
  sourceEditorValue,
  systemMetadataFoldRange,
} from "../app/source-editor";

afterEach(() => {
  cleanup();
  window.localStorage.clear();
});

const noOperation = () => undefined;

describe("Source editor", () => {
  it("folds one unambiguous system mapping and exposes an explicit toggle", () => {
    const source = "---\nweftext:\n  id: \"550e8400-e29b-41d4-a716-446655440000\"\n  icon: 文\n---\n= 标题\n";
    expect(systemMetadataFoldRange(source)).not.toBeNull();
    expect(systemMetadataFoldRange("---\nweftext:\n  id: first\nweftext:\n  id: duplicate\n---\n")).toBeNull();

    render(<SourceEditor
      value={source}
      selectionStart={source.length}
      selectionEnd={source.length}
      scrollTop={0}
      restoreToken={1}
      onChange={noOperation}
      onSelectionChange={noOperation}
      onScroll={noOperation}
      onKeyDown={noOperation}
      onFind={noOperation}
    />);

    const toggle = screen.getByRole("button", { name: "展开系统元数据" });
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    const editor = screen.getByRole("textbox", { name: "AsciiDoc 源码" });
    act(() => setSourceEditorSelection(editor, source.indexOf("id:")));
    const collapse = screen.getByRole("button", { name: "折叠系统元数据" });
    expect(collapse.getAttribute("aria-expanded")).toBe("true");
    fireEvent.click(collapse);
    expect(screen.getByRole("button", { name: "展开系统元数据" }).getAttribute("aria-expanded")).toBe("false");
  });

  it("preserves unrelated CRLF bytes when applying an exact source edit", () => {
    const source = "---\r\nweftext:\r\n  id: \"550e8400-e29b-41d4-a716-446655440000\"\r\n---\r\n= 标题\r\n正文\r\n";
    let changed = "";
    render(<SourceEditor
      value={source}
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

    const editor = screen.getByRole("textbox", { name: "AsciiDoc 源码" });
    act(() => {
      setSourceEditorSelection(editor, source.indexOf("正文"), source.indexOf("正文") + 2);
      replaceSourceEditorSelection(editor, "内容");
    });
    expect(changed).toBe(source.replace("正文", "内容"));
    expect(sourceEditorValue(editor)).toBe(changed);
    expect(changed.split("\r\n")).toHaveLength(7);
  });

  it("preserves unrelated mixed line endings and fails open for ambiguous metadata", () => {
    const source = "---\r\nweftext:\r\n  id: first\r\nweftext:\n  id: duplicate\n---\r\n= 标题\n第一行\r\n第二行\n";
    let changed = "";
    render(<SourceEditor
      value={source}
      selectionStart={source.indexOf("第一行")}
      selectionEnd={source.indexOf("第一行") + 3}
      scrollTop={0}
      restoreToken={1}
      onChange={(value) => { changed = value; }}
      onSelectionChange={noOperation}
      onScroll={noOperation}
      onKeyDown={noOperation}
      onFind={noOperation}
    />);
    expect(screen.queryByRole("button", { name: "展开系统元数据" })).toBeNull();
    const editor = screen.getByRole("textbox", { name: "AsciiDoc 源码" });
    act(() => {
      setSourceEditorSelection(editor, source.indexOf("第一行"), source.indexOf("第一行") + 3);
      replaceSourceEditorSelection(editor, "首行");
    });
    expect(changed).toBe(source.replace("第一行", "首行"));
    expect(changed.match(/\r\n/g)?.length).toBe(source.match(/\r\n/g)?.length);
    expect(changed.replaceAll("\r\n", "").match(/\n/g)?.length).toBe(source.replaceAll("\r\n", "").match(/\n/g)?.length);
  });

  it("restores explicit fold state by workspace and node UUID key", () => {
    const source = "---\nweftext:\n  id: \"550e8400-e29b-41d4-a716-446655440000\"\n---\n= 标题\n";
    const props = {
      value: source,
      selectionStart: source.length,
      selectionEnd: source.length,
      scrollTop: 0,
      restoreToken: 1,
      onChange: noOperation,
      onSelectionChange: noOperation,
      onScroll: noOperation,
      onKeyDown: noOperation,
      onFind: noOperation,
    };
    const mounted = render(<SourceEditor {...props} stateKey="workspace-a/node-a" />);
    fireEvent.click(screen.getByRole("button", { name: "展开系统元数据" }));
    expect(screen.getByRole("button", { name: "折叠系统元数据" })).toBeTruthy();
    mounted.unmount();
    render(<SourceEditor {...props} stateKey="workspace-a/node-a" />);
    expect(screen.getByRole("button", { name: "折叠系统元数据" })).toBeTruthy();
    cleanup();
    render(<SourceEditor {...props} stateKey="workspace-a/node-b" />);
    expect(screen.getByRole("button", { name: "展开系统元数据" })).toBeTruthy();
  });
});
