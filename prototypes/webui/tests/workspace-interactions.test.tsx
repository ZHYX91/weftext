import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import Home, { WeftextApp, extendTableAtCoreBlock, formattedBlockReplacement, headingBlockReplacement, type DocumentBlock } from "../app/page";
import {
  replaceSourceEditorValue,
  setSourceEditorScroll,
  setSourceEditorSelection,
  sourceEditorSelection,
  sourceEditorValue,
} from "../app/source-editor";
import { writeEditorValue } from "../app/write-editor";
import type { DocumentBlockKind, DocumentBlockSemantic, DocumentModel } from "../app/document-contract";

afterEach(() => {
  cleanup();
  window.location.hash = "";
  window.localStorage.clear();
  delete window.weftextDesktop;
});

function openSource() {
  fireEvent.click(screen.getByRole("button", { name: "源码" }));
  return screen.getByRole("textbox", { name: "AsciiDoc 源码" });
}

function replaceSource(editor: Element, value: string) {
  act(() => replaceSourceEditorValue(editor, value));
}

function selectSource(editor: Element, start: number, end = start) {
  act(() => setSourceEditorSelection(editor, start, end));
}

function treeNode(name: string) {
  return within(screen.getByRole("tree", { name: "工作区层级" })).getByRole("button", { name: `打开节点 ${name}` });
}

function semanticForBlock(kind: DocumentBlockKind, headingLevel: number | null, quoteDepth: number | null): DocumentBlockSemantic {
  if (kind === "heading") return { kind, level: headingLevel ?? 1 };
  if (kind === "listing" || kind === "fenced_code") return { kind: "listing", language: null };
  if (kind === "quote") return { kind, depth: quoteDepth, attribution: null, citation: null };
  if (kind === "list") return { kind, model: { kind: "unordered", depth: 1, items: [] } };
  if (kind === "table") return { kind, model: { header: null, body: [], footer: null, columnCount: 0 } };
  if (kind === "image") return { kind, target: "", alt: null };
  if (kind === "math") return { kind, notation: "ascii_math" };
  if (kind === "unsupported" || kind === "html") return { kind: "unsupported", context: kind };
  return { kind } as DocumentBlockSemantic;
}

function coreBlock(seed: Partial<DocumentBlock> & Pick<DocumentBlock, "kind" | "start" | "end">): DocumentBlock {
  const headingLevel = seed.headingLevel ?? null;
  const quoteDepth = seed.quoteDepth ?? null;
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
    semantic: seed.semantic ?? semanticForBlock(seed.kind, headingLevel, quoteDepth),
  };
}

function coreModel(blocks: DocumentBlock[] = [], diagnosticMessage?: string): DocumentModel {
  return {
    semanticModelVersion: 1,
    status: diagnosticMessage ? "degraded" : "complete",
    blocks,
    inlines: [],
    runInGroups: [],
    diagnostics: diagnosticMessage ? [{ code: "profile_warning", start: 0, end: 0, message: diagnosticMessage }] : [],
    degradations: [],
    safeHtml: "<article data-weftext-profile=\"weftext-asciidoc-v1\"></article>",
  };
}

function authorityModel(source: string, diagnosticCode?: string) {
  const length = new TextEncoder().encode(source).length;
  return coreModel([coreBlock({ kind: "paragraph", start: 0, end: length, textStart: 0, textEnd: length, text: source })], diagnosticCode);
}

function authorityWorkspace() {
  return {
    rootNodeId: "root",
    revision: "workspace-revision",
    presentation: { adjacentHeadingBody: "separate" as const },
    nodes: [
      { id: "root", name: "Workspace", parentId: null, path: "", trashed: false },
      { id: "doc", name: "Authority", parentId: "root", path: "Authority", trashed: false },
    ],
    links: { outgoing: [], backlinks: [], potentialMentions: [] },
  };
}

function authorityDocument(source: string) {
  return {
    nodeId: "doc",
    name: "Authority",
    revision: "document-revision",
    length: new TextEncoder().encode(source).length,
    source,
    model: authorityModel(source),
  };
}

function closedStructuralPlan(overrides: Record<string, unknown> = {}) {
  return {
    planId: "structural-plan",
    action: "move",
    baseRevision: "workspace-revision",
    pathChanges: [],
    documentChanges: [],
    generatedNodeIds: [],
    scopeSummary: null,
    identityMap: [],
    capturedTarget: null,
    targetNodeIds: [],
    draftSensitiveNodeIds: [],
    ...overrides,
  };
}

describe("production Core document authority", () => {
  it("keeps malformed YAML visible when the Core model does not identify a frontmatter block", async () => {
    const source = "---\nweftext:\n  id: [broken\n---\n= 可检查标题\n正文";
    const model = authorityModel(source, "malformed_yaml");
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace: authorityWorkspace(), document: { ...authorityDocument(source), model } }),
      chooseWorkspace: async () => null,
      request: async (path) => {
        if (path === "/api/document/model") return { model };
        if (path.startsWith("/api/annotations")) return { annotations: { version: 3, document_id: "doc", annotations: [] } };
        return {};
      },
    };

    render(<WeftextApp demo={null} />);
    const write = await screen.findByRole("textbox", { name: "AsciiDoc 正文" });
    expect(writeEditorValue(write)).toBe(source);
    expect(writeEditorValue(write)).toContain("id: [broken");
  });

  it("shows a Core draft-model failure and keeps exact Source available", async () => {
    const source = "---\nweftext:\n  id: [broken\n正文";
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace: authorityWorkspace(), document: authorityDocument(source) }),
      chooseWorkspace: async () => null,
      request: async (path) => {
        if (path === "/api/document/model") throw new Error("Core YAML envelope failure");
        if (path.startsWith("/api/annotations")) return { annotations: { version: 3, document_id: "doc", annotations: [] } };
        return {};
      },
    };

    render(<WeftextApp demo={null} />);
    expect((await screen.findByRole("alert")).textContent).toContain("Core YAML envelope failure");
    fireEvent.click(screen.getByRole("button", { name: "打开精确源码" }));
    expect(sourceEditorValue(screen.getByRole("textbox", { name: "AsciiDoc 源码" }))).toBe(source);
  });

  it("renders degraded passthrough as escaped inert source and never injects Core HTML", async () => {
    const source = "++++\n<img src=x onerror=alert(1)>\n++++\n";
    const model: DocumentModel = {
      ...coreModel([coreBlock({ kind: "passthrough", start: 0, end: source.length, textStart: 5, textEnd: source.length - 5, text: "<img src=x onerror=alert(1)>" })]),
      status: "degraded",
      degradations: [{
        kind: "disabled_passthrough",
        supportState: "prohibited_effect",
        range: { start: 0, end: source.length },
        fallback: "disabled_effect",
        message: "passthrough effect is disabled",
      }],
      safeHtml: "<img data-unsafe-core-html src=x onerror=alert(1)>",
    };
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace: authorityWorkspace(), document: { ...authorityDocument(source), model } }),
      chooseWorkspace: async () => null,
      request: async (path) => {
        if (path === "/api/document/model") return { model };
        if (path.startsWith("/api/annotations")) return { annotations: { version: 3, document_id: "doc", annotations: [] } };
        return {};
      },
    };

    render(<WeftextApp demo={null} />);
    await screen.findByRole("textbox", { name: "AsciiDoc 正文" });
    fireEvent.click(screen.getByRole("button", { name: "阅读" }));
    expect(screen.getByText("部分 AsciiDoc 语义采用受限显示")).toBeTruthy();
    expect(screen.getByText("passthrough effect is disabled")).toBeTruthy();
    expect(screen.getByText("<img src=x onerror=alert(1)>")).toBeTruthy();
    expect(document.querySelector("img, [data-unsafe-core-html]")).toBeNull();
  });

  it("ignores a stale async model response after a newer draft model is ready", async () => {
    const initial = "= 初始\n正文";
    const second = "= 第二版\n较慢";
    const third = "= 第三版\n最新";
    type Deferred = { resolve(value: unknown): void; promise: Promise<unknown> };
    const deferred = new Map<string, Deferred>();
    const deferredFor = (source: string) => {
      let resolve!: (value: unknown) => void;
      const promise = new Promise<unknown>((next) => { resolve = next; });
      const value = { resolve, promise };
      deferred.set(source, value);
      return value;
    };
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace: authorityWorkspace(), document: authorityDocument(initial) }),
      chooseWorkspace: async () => null,
      request: async (path, body) => {
        if (path === "/api/document/model") {
          const source = String((body as { source: string }).source);
          if (source === initial) return { model: authorityModel(initial) };
          return (deferred.get(source) ?? deferredFor(source)).promise;
        }
        if (path.startsWith("/api/annotations")) return { annotations: { version: 3, document_id: "doc", annotations: [] } };
        return {};
      },
    };

    render(<WeftextApp demo={null} />);
    await screen.findByRole("textbox", { name: "AsciiDoc 正文" });
    const sourceEditor = openSource();
    replaceSource(sourceEditor, second);
    await waitFor(() => expect(deferred.has(second)).toBe(true));
    replaceSource(sourceEditor, third);
    await waitFor(() => expect(deferred.has(third)).toBe(true));
    deferred.get(third)!.resolve({ model: authorityModel(third, "newest") });
    fireEvent.click(screen.getByRole("button", { name: "写作" }));
    const write = await screen.findByRole("textbox", { name: "AsciiDoc 正文" });
    expect(writeEditorValue(write)).toBe(third);

    deferred.get(second)!.resolve({ model: authorityModel(second, "stale") });
    await act(async () => Promise.resolve());
    expect(writeEditorValue(screen.getByRole("textbox", { name: "AsciiDoc 正文" }))).toBe(third);
    expect(screen.queryByText("正在等待 Core 解析当前精确草稿…")).toBeNull();
  });
});

describe("Workspace Trash item authority", () => {
  it("uses Core item inventory and requires an explicit target for unknown-origin restore", async () => {
    const source = "= Authority\n\nBody\n";
    const itemId = "550e8400-e29b-41d4-a716-446655440000";
    const digest = "a".repeat(64);
    const workspace = {
      ...authorityWorkspace(),
      trashItems: [{
        manifest: {
          schema: "weftext.trash-item/v1",
          trashItemId: itemId,
          operationId: "6ba7b810-9dad-41d1-80b4-00c04fd430c8",
          kind: "node",
          trashedAt: "2026-08-24T12:00:00+08:00",
          originStatus: "unknown",
          nodeId: "8f14e45f-ea8a-4f6d-8c31-8a7f3f5f6c20",
          originalParentNodeId: null,
          originalName: "Legacy note",
          ancestorNodeIds: [],
          payloadSha256: digest,
          payloadByteLength: 4096,
          payloadEntryCount: 2,
        },
        containedNodeIds: ["8f14e45f-ea8a-4f6d-8c31-8a7f3f5f6c20"],
        restore: {
          originResolution: "unknown",
          originalAvailable: false,
          withAncestorsAvailable: false,
          requiredAncestorItemIds: [],
          blockedReason: null,
        },
      }],
    };
    const requests: Array<{ path: string; body: unknown }> = [];
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace, document: authorityDocument(source) }),
      chooseWorkspace: async () => null,
      request: async (path, body) => {
        requests.push({ path, body });
        if (path.startsWith("/api/annotations")) return { annotations: { version: 3, document_id: "doc", annotations: [] } };
        if (path === "/api/trash/restore/preview") return {
          plan: closedStructuralPlan({
            planId: "restore-plan",
            action: "trash_restore",
            baseRevision: "workspace-revision",
            scopeSummary: {
              rootNode: { nodeId: "8f14e45f-ea8a-4f6d-8c31-8a7f3f5f6c20", displayName: "Renamed note" },
              descendantNodeCount: 0,
              resourceCount: 0,
              annotationSidecarCount: 0,
              byteTotal: 4096,
              affectedDocumentNodeIds: ["8f14e45f-ea8a-4f6d-8c31-8a7f3f5f6c20"],
              rewrittenDocumentNodeIds: [],
              identityPolicy: "preserve",
              trashItemCount: 1,
              operationId: null,
            },
            capturedTarget: { kind: "trash_item", trashItemId: itemId, resolvedBy: "explicit_row" },
            targetNodeIds: ["8f14e45f-ea8a-4f6d-8c31-8a7f3f5f6c20", "root"],
            draftSensitiveNodeIds: ["8f14e45f-ea8a-4f6d-8c31-8a7f3f5f6c20"],
            trashItemChanges: [{
              disposition: "restored",
              manifest: workspace.trashItems[0].manifest,
              destinationNodeId: "root",
              destinationName: "Renamed note",
            }],
          }),
        };
        return {};
      },
    };

    render(<WeftextApp demo={null} />);
    await screen.findByRole("textbox", { name: "AsciiDoc 正文" });
    fireEvent.click(screen.getByRole("button", { name: /废纸篓/ }));
    expect(screen.queryByRole("treeitem", { name: /Legacy note/ })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: /Legacy note/ }));
    expect(screen.getByText(/旧条目没有可信来源/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "恢复废纸篓条目" }));
    const preview = screen.getByRole("button", { name: "预览恢复条目" });
    expect((preview as HTMLButtonElement).disabled).toBe(true);
    fireEvent.change(screen.getByRole("combobox", { name: "废纸篓恢复目标" }), { target: { value: "root" } });
    fireEvent.change(screen.getByRole("textbox", { name: "废纸篓恢复名称" }), { target: { value: "Renamed note" } });
    fireEvent.click(preview);

    await screen.findByRole("heading", { name: "恢复废纸篓条目预览" });
    const userFacingText = [
      document.body.textContent ?? "",
      ...Array.from(document.querySelectorAll("[aria-label]")).map((element) => element.getAttribute("aria-label") ?? ""),
    ].join(" ");
    expect(userFacingText).not.toMatch(/Workspace Trash|Trash Item|managed nodes|workspace rev/u);
    const request = requests.find((entry) => entry.path === "/api/trash/restore/preview");
    expect(request?.body).toEqual({
      trashItemId: itemId,
      baseWorkspaceRevision: "workspace-revision",
      mode: "existing_target",
      resolvedBy: "explicit_row",
      targetNodeId: "root",
      name: "Renamed note",
    });
    expect(screen.getByText(itemId, { exact: false })).toBeTruthy();
    expect(screen.getByText(digest, { exact: false })).toBeTruthy();
  });
});

function installPrimaryNavigationDesktop(failingDraftNodeId?: string) {
    const model = coreModel();
  const documents = {
    a: { nodeId: "a", name: "节点 A", revision: "a-revision", length: 18, source: "= 节点 A\nalpha disk\n", model },
    b: { nodeId: "b", name: "节点 B", revision: "b-revision", length: 17, source: "= 节点 B\nbeta disk\n", model },
  };
  const workspace = {
    rootNodeId: "root", revision: "workspace-revision", presentation: { adjacentHeadingBody: "separate" as const },
    nodes: [
      { id: "root", name: "Workspace", parentId: null, path: "", trashed: false },
      { id: "a", name: "节点 A", parentId: "root", path: "节点 A", trashed: false },
      { id: "b", name: "节点 B", parentId: "root", path: "节点 B", trashed: false },
    ], links: { outgoing: [], backlinks: [], potentialMentions: [] },
  };
  const savedDrafts: Array<Record<string, unknown>> = [];
  const events: string[] = [];
  let chooserCalls = 0;
  window.weftextDesktop = {
    restoreWorkspace: async () => ({ opened: true, workspace, document: documents.a }),
    chooseWorkspace: async () => {
      chooserCalls += 1;
      events.push("choose-workspace");
      return null;
    },
    request: async (path, body) => {
      if (path.startsWith("/api/document?")) {
        const id = new URL(path, "http://desktop.local").searchParams.get("nodeId") as "a" | "b";
        events.push(`open:${id}`);
        return { document: documents[id] };
      }
      if (path === "/api/document/model") return { model };
      if (path.startsWith("/api/annotations")) return { annotations: { version: 3, document_id: "a", annotations: [] } };
      if (path === "/api/draft/save") {
        const draft = body as Record<string, unknown>;
        events.push(`save:${String(draft.nodeId)}`);
        if (draft.nodeId === failingDraftNodeId) throw new Error("primary draft failure injection");
        savedDrafts.push(draft);
        return { clean: false, draftRecovery: { drafts: [], issues: [] } };
      }
      return {};
    },
  };
  return { events, savedDrafts, chooserCalls: () => chooserCalls };
}

describe("workspace document sessions", () => {
  it("keeps the built-in AsciiDoc demo path preview while live paths come from Core", () => {
    render(<Home />);
    fireEvent.click(screen.getByRole("button", { name: "新建节点" }));
    const dialog = screen.getByRole("dialog", { name: "新建节点" });
    expect(within(dialog).getByText("会议记录/会议记录.adoc")).toBeTruthy();
    fireEvent.change(within(dialog).getByPlaceholderText("例如：会议记录"), { target: { value: "R1A" } });
    expect(within(dialog).getByText("R1A/R1A.adoc")).toBeTruthy();
  });

  it("switches the controlled Source value with the selected node", () => {
    render(<Home />);
    const source = openSource();
    expect(sourceEditorValue(source)).toContain("= 交互原则");

    fireEvent.click(treeNode("产品方向"));
    expect(sourceEditorValue(source)).toContain("= 产品方向");
    expect(sourceEditorValue(source)).not.toContain("= 交互原则");
  });

  it("keeps an unsaved draft when switching away and back", () => {
    render(<Home />);
    const source = openSource();
    replaceSource(source, `${sourceEditorValue(source)}\n会话草稿标记`);

    fireEvent.click(treeNode("产品方向"));
    expect(sourceEditorValue(source)).not.toContain("会话草稿标记");
    fireEvent.click(treeNode("交互原则"));
    expect(sourceEditorValue(source)).toContain("会话草稿标记");
  });

  it("applies exact cursor-local AsciiDoc and table edits", async () => {
    render(<Home />);
    const source = openSource();
    replaceSource(source, "= 标题\n正文");
    const start = sourceEditorValue(source).indexOf("正文");
    selectSource(source, start, start + 2);
    fireEvent.click(screen.getByRole("button", { name: "加粗" }));
    expect(sourceEditorValue(source)).toBe("= 标题\n*正文*");

    selectSource(source, sourceEditorValue(source).length);
    fireEvent.click(screen.getByRole("button", { name: "插入表格" }));
    expect(sourceEditorValue(source)).toContain("|===\n|列 1 |列 2\n| |\n|===");
  });

  it("applies H1-H9 and quote depth operations without normalizing exact line endings", () => {
    expect(headingBlockReplacement("  === 中文标题\r\n", 9)).toBe("  ========== 中文标题\r\n");
    expect(headingBlockReplacement("========== H9\n", 1)).toBe("== H9\n");
    const mixed = "> 一级\r\n>> 二级\n>>> 三级\r";
    const deeper = formattedBlockReplacement(mixed, "quote_increase", "\r\n");
    expect(deeper).toBe("> > 一级\r\n> >> 二级\n> >>> 三级\r");
    expect(formattedBlockReplacement(deeper, "quote_decrease", "\r\n")).toBe(mixed);
    const fenced = "----\r\n== 不是标题\n> 不是引用\r\n----\r\n";
    expect(formattedBlockReplacement(fenced, "code", "\r\n")).toBe("== 不是标题\n> 不是引用\r\n");
  });

  it("appends AsciiDoc table rows and columns only inside the Core block while preserving mixed line endings", () => {
    const prefix = "---\r\nweftext:\r\n  id: \"11111111-1111-4111-8111-111111111111\"\r\n---\r\n";
    const table = ".Metrics\r\n|===\n| A | B\r\n\r| 1 | 2\n|===\r\n";
    const source = `${prefix}${table}尾段\n`;
    const block: DocumentBlock = {
      kind: "table",
      start: prefix.length,
      end: prefix.length + table.length,
      textStart: prefix.length,
      textEnd: prefix.length + table.length,
      text: table,
      headingLevel: null,
      blockId: null,
    };
    const bodyCursor = block.start + table.indexOf("1");
    const column = extendTableAtCoreBlock(source, block, bodyCursor, "column");
    expect(column?.source).toBe(`${prefix}.Metrics\r\n|===\n| A | B |  \r\n\r| 1 | 2 |  \n|===\r\n尾段\n`);
    expect(column?.cursor).toBe(bodyCursor + " |  ".length);

    const row = extendTableAtCoreBlock(source, block, block.start + table.indexOf("A"), "row");
    expect(row?.source).toBe(`${prefix}.Metrics\r\n|===\n| A | B\r\n\r| 1 | 2\n|  |  \r\n|===\r\n尾段\n`);
    expect(row?.cursor).toBe(block.start + table.lastIndexOf("|==="));

    const gfm = "| A | B |\n| --- | --- |\n| 1 | 2 |\n";
    const gfmBlock = { ...block, start: 0, end: gfm.length, textStart: 0, textEnd: gfm.length, text: gfm };
    expect(extendTableAtCoreBlock(gfm, gfmBlock, 1, "row")).toBeNull();
  });

  it("keeps Chinese IME composition text and handles a large draft", () => {
    render(<Home />);
    const source = openSource();
    const large = Array.from({ length: 2_000 }, (_, index) => `第 ${index + 1} 行 中文内容`).join("\n");
    fireEvent.compositionStart(source);
    replaceSource(source, `${large}\n正在输入中文`);
    fireEvent.compositionEnd(source, { data: "中文" });
    expect(sourceEditorValue(source).endsWith("正在输入中文")).toBe(true);
    expect(sourceEditorValue(source).split("\n")).toHaveLength(2_001);
  });

  it("persists selection, scroll and view per node identity", async () => {
    render(<Home />);
    const source = openSource();
    const offset = sourceEditorValue(source).indexOf("核心原则");
    selectSource(source, offset);
    expect(JSON.parse(window.localStorage.getItem("weftext.editor-state.v1") ?? "{}")["demo/principles"].selectionStart).toBe(offset);
    act(() => setSourceEditorScroll(source, 240));
    fireEvent.click(treeNode("产品方向"));
    fireEvent.click(treeNode("交互原则"));
    const restored = screen.getByRole("textbox", { name: "AsciiDoc 源码" });
    await waitFor(() => expect(sourceEditorSelection(restored).start).toBe(offset));
    expect(JSON.parse(window.localStorage.getItem("weftext.editor-state.v1") ?? "{}")["demo/principles"].scrollTop).toBe(240);
  });

  it("stores an explicit high-contrast theme as device-local UI state", () => {
    render(<Home />);
    fireEvent.click(screen.getByRole("button", { name: "工作区设置与连接" }));
    fireEvent.change(screen.getByRole("combobox", { name: "界面主题" }), { target: { value: "contrast" } });
    expect(document.querySelector(".app-shell")?.getAttribute("data-theme")).toBe("contrast");
    expect(window.localStorage.getItem("weftext.theme.v1")).toBe("contrast");
  });

  it("shows annotations only for the node that owns them", () => {
    render(<Home />);
    expect(screen.getByText("林然")).toBeTruthy();
    fireEvent.click(treeNode("产品方向"));
    expect(screen.queryByText("林然")).toBeNull();
    expect(screen.getByText("当前节点没有批注。")).toBeTruthy();
  });

  it("renders annotation v3 and previews only the canonical typed request", async () => {
    const actorId = "00000000-0000-4000-8000-000000000002";
    window.localStorage.setItem("weftext.annotation.actor.v1", JSON.stringify({ id: actorId, name: "Local reviewer" }));
    const source = "= Workspace\n正文\n";
    const model = coreModel();
    const workspace = {
      rootNodeId: "root",
      revision: "workspace-revision",
      presentation: { adjacentHeadingBody: "separate" as const },
      nodes: [{ id: "root", name: "Workspace", parentId: null, path: "", trashed: false }],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
    };
    const annotations = {
      version: 3,
      document_id: "root",
      annotations: [{
        id: "11111111-1111-4111-8111-111111111111",
        kind: "comment",
        target: { kind: "block", heading_path: ["Workspace"] },
        appearance: { mark: "highlight", theme: "purple" },
        labels: ["verify"],
        thread: [{
          id: "22222222-2222-4222-8222-222222222222",
          author_id: "33333333-3333-4333-8333-333333333333",
          author_name: "Review Alice",
          body: { format: "weftext.asciidoc.inline.v1", source: "请核对 *正文*。" },
          created_at: "2026-08-24T00:00:00Z",
          updated_at: "2026-08-24T00:00:00Z",
        }],
        state: "open",
        created_at: "2026-08-24T00:00:00Z",
        updated_at: "2026-08-24T00:00:00Z",
      }],
    };
    const previewBodies: Array<Record<string, unknown>> = [];
    window.weftextDesktop = {
      restoreWorkspace: async () => ({
        opened: true,
        workspace,
        document: { nodeId: "root", name: "Workspace", revision: "a".repeat(64), length: source.length, source, model },
      }),
      chooseWorkspace: async () => null,
      request: async (path, body) => {
        if (path === "/api/document/model") return { model };
        if (path.startsWith("/api/annotations")) return { annotations };
        if (path === "/api/annotation/preview") {
          previewBodies.push(body as Record<string, unknown>);
          return { plan: { planId: "annotation-plan", baseRevision: "a".repeat(64), action: "annotation" } };
        }
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(screen.getAllByText("Review Alice")).toHaveLength(2));
    expect(screen.getByText("请核对 *正文*。")).toBeTruthy();
    expect(screen.getByText("highlight")).toBeTruthy();
    expect(screen.getByText("verify")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "＋ 添加批注或建议" }));
    const dialog = screen.getByRole("dialog", { name: "添加批注或建议" });
    fireEvent.change(within(dialog).getByRole("textbox", { name: "批注内容" }), { target: { value: "新的 _评论_" } });
    fireEvent.change(within(dialog).getByRole("textbox", { name: "批注标签" }), { target: { value: "question, verify" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "生成事务预览" }));

    await waitFor(() => expect(previewBodies).toHaveLength(1));
    expect(previewBodies[0]).toEqual({
      action: "create",
      nodeId: "root",
      timestamp: expect.any(String),
      kind: "comment",
      target: { kind: "block_at", sourceOffset: 0 },
      appearance: { mark: "highlight", theme: "yellow" },
      labels: ["question", "verify"],
      bodySource: "新的 _评论_",
      authorId: actorId,
      authorName: "Local reviewer",
    });
    expect(previewBodies[0]).not.toHaveProperty("sourceOffset");
    expect(previewBodies[0]).not.toHaveProperty("mark");
    expect(previewBodies[0]).not.toHaveProperty("color");
  });

  it("opens the selected search result with the keyboard", () => {
    render(<Home />);
    fireEvent.click(screen.getByRole("button", { name: "搜索工作区" }));
    const input = screen.getByPlaceholderText("搜索节点、用户属性或正文…");
    fireEvent.change(input, { target: { value: "产品方向" } });
    fireEvent.keyDown(input, { key: "Enter" });

    const source = openSource();
    expect(sourceEditorValue(source)).toContain("= 产品方向");
  });

  it("restores UUID-only tabs, history, breadcrumbs, bookmarks, recents and split session", async () => {
    render(<Home />);
    fireEvent.click(screen.getByRole("button", { name: "新标签快速打开节点" }));
    const search = screen.getByPlaceholderText("搜索节点、用户属性或正文…");
    fireEvent.change(search, { target: { value: "产品方向" } });
    fireEvent.click(within(screen.getByRole("dialog", { name: "搜索工作区" })).getByRole("button", { name: /产品方向/ }));
    await waitFor(() => expect(document.querySelectorAll(".tab-item")).toHaveLength(2));
    expect(document.querySelector(".tab-item.active .tab")?.textContent).toContain("产品方向");

    fireEvent.click(screen.getByRole("button", { name: "收藏当前节点" }));
    expect(within(screen.getByRole("region", { name: "书签" })).getByRole("button", { name: "产品方向" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "打开双栏分屏" }));
    expect(screen.getByRole("complementary", { name: "第二编辑栏" })).toBeTruthy();

    fireEvent.click(treeNode("交互原则"));
    expect(document.querySelector(".tab-item.active .tab")?.textContent).toContain("交互原则");
    expect([...document.querySelectorAll(".tab-item .tab")].filter((tab) => tab.textContent?.includes("交互原则"))).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: "后退" }));
    await waitFor(() => expect(document.querySelector(".tab-item.active .tab")?.textContent).toContain("产品方向"));
    fireEvent.click(screen.getByRole("button", { name: "前进" }));
    await waitFor(() => expect(document.querySelector(".tab-item.active .tab")?.textContent).toContain("交互原则"));

    const breadcrumbs = screen.getByRole("navigation", { name: "面包屑" });
    fireEvent.click(within(breadcrumbs).getByRole("button", { name: "项目总览" }));
    await waitFor(() => expect(document.querySelector(".tab-item.active .tab")?.textContent).toContain("项目总览"));

    const stored = JSON.parse(window.localStorage.getItem("weftext.navigation.v1") ?? "{}") as Record<string, unknown>;
    expect(JSON.stringify(stored)).not.toContain("产品/方向");
    expect(JSON.stringify(stored)).toContain("direction");
    cleanup();
    render(<Home />);
    expect(document.querySelectorAll(".tab-item")).toHaveLength(2);
    expect(screen.getByRole("complementary", { name: "第二编辑栏" })).toBeTruthy();
    expect(within(screen.getByRole("region", { name: "书签" })).getByRole("button", { name: "产品方向" })).toBeTruthy();

    const close = screen.getByRole("button", { name: "关闭标签 项目总览" });
    fireEvent.click(close);
    await waitFor(() => expect(document.querySelectorAll(".tab-item")).toHaveLength(1));
  });

  it("keeps second-pane find, replace, view and selection on its own draft", async () => {
    const model = coreModel();
    const documents = {
      root: { nodeId: "root", name: "Workspace", revision: "a".repeat(64), source: "= Workspace\nroot token\n", model },
      a: { nodeId: "a", name: "节点 A", revision: "b".repeat(64), source: "= 节点 A\nalpha token\n", model },
      b: { nodeId: "b", name: "节点 B", revision: "c".repeat(64), source: "= 节点 B\nbeta token\n", model },
    };
    const workspace = {
      rootNodeId: "root",
      revision: "workspace-revision",
      presentation: { adjacentHeadingBody: "separate" as const },
      nodes: [
        { id: "root", name: "Workspace", parentId: null, path: "", trashed: false },
        { id: "a", name: "节点 A", parentId: "root", path: "节点 A", trashed: false },
        { id: "b", name: "节点 B", parentId: "root", path: "节点 B", trashed: false },
      ],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
    };
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace, document: documents.root }),
      chooseWorkspace: async () => null,
      request: async (path) => {
        if (path === "/api/document/model") return { model };
        if (path.startsWith("/api/document?")) {
          const id = new URL(path, "http://desktop.local").searchParams.get("nodeId") as "root" | "a" | "b";
          return { document: documents[id] };
        }
        if (path === "/api/annotations") return { annotations: { version: 3, document_id: "root", annotations: [] } };
        if (path === "/api/draft/save") return { clean: false, draftRecovery: { drafts: [], issues: [] } };
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(treeNode("节点 A")).toBeTruthy());
    fireEvent.click(treeNode("节点 A"));
    await waitFor(() => expect(document.querySelector(".tab-item.active .tab")?.textContent).toContain("节点 A"));
    fireEvent.click(screen.getByRole("button", { name: "打开双栏分屏" }));
    const pane = await screen.findByRole("complementary", { name: "第二编辑栏" });
    fireEvent.click(within(pane).getByRole("button", { name: "源码" }));
    await waitFor(() => expect(within(pane).getByRole("textbox", { name: "AsciiDoc 源码" })).toBeTruthy());
    fireEvent.click(within(pane).getByRole("button", { name: "查找第二编辑栏" }));
    fireEvent.change(within(pane).getByRole("textbox", { name: "查找第二编辑栏文本" }), { target: { value: "root token" } });
    fireEvent.click(within(pane).getByRole("button", { name: "第二栏下一个匹配" }));
    const source = within(pane).getByRole("textbox", { name: "AsciiDoc 源码" });
    await waitFor(() => expect(sourceEditorSelection(source).end - sourceEditorSelection(source).start).toBe("root token".length));
    fireEvent.click(within(pane).getByRole("button", { name: "替换" }));
    fireEvent.change(within(pane).getByRole("textbox", { name: "第二栏替换文本" }), { target: { value: "replaced" } });
    fireEvent.click(within(pane).getByRole("button", { name: "替换当前" }));
    await waitFor(() => expect(sourceEditorValue(source)).toContain("replaced"));
    expect(document.querySelector(".tab-item.active .tab")?.textContent).toContain("节点 A");
    fireEvent.change(within(pane).getByRole("combobox", { name: "切换第二编辑栏节点" }), { target: { value: "b" } });
    await waitFor(() => expect(sourceEditorValue(within(pane).getByRole("textbox", { name: "AsciiDoc 源码" }))).toContain("beta token"));
    fireEvent.change(within(pane).getByRole("combobox", { name: "切换第二编辑栏节点" }), { target: { value: "root" } });
    await waitFor(() => expect(sourceEditorValue(within(pane).getByRole("textbox", { name: "AsciiDoc 源码" }))).toContain("replaced"));
  });

  it("persists an outgoing dirty primary document before immediately opening another node", async () => {
    const fixture = installPrimaryNavigationDesktop();
    render(<Home />);
    await waitFor(() => expect(document.querySelector(".tab-item.active .tab")?.textContent).toContain("节点 A"));
    const source = openSource();
    const edited = `${sourceEditorValue(source)}primary immediate edit\n`;
    replaceSource(source, edited);
    fireEvent.click(treeNode("节点 B"));

    await waitFor(() => expect(document.querySelector(".tab-item.active .tab")?.textContent).toContain("节点 B"));
    expect(fixture.events.indexOf("save:a")).toBeGreaterThanOrEqual(0);
    expect(fixture.events.indexOf("save:a")).toBeLessThan(fixture.events.indexOf("open:b"));
    expect(fixture.savedDrafts).toContainEqual({ nodeId: "a", revision: "a-revision", source: edited });
  });

  it("keeps the dirty primary node active when its outgoing draft cannot be persisted", async () => {
    const fixture = installPrimaryNavigationDesktop("a");
    render(<Home />);
    await waitFor(() => expect(document.querySelector(".tab-item.active .tab")?.textContent).toContain("节点 A"));
    const source = openSource();
    replaceSource(source, `${sourceEditorValue(source)}must remain visible\n`);
    fireEvent.click(treeNode("节点 B"));

    await waitFor(() => expect(screen.getByRole("heading", { name: "草稿恢复中心" })).toBeTruthy());
    expect(document.querySelector(".tab-item.active .tab")?.textContent).toContain("节点 A");
    expect(sourceEditorValue(screen.getByRole("textbox", { name: "AsciiDoc 源码" }))).toContain("must remain visible");
    expect(fixture.events).toContain("save:a");
    expect(fixture.events).not.toContain("open:b");
  });

  it("keeps the outgoing primary draft durable when switching the whole workspace after A to B", async () => {
    const fixture = installPrimaryNavigationDesktop();
    render(<Home />);
    await waitFor(() => expect(document.querySelector(".tab-item.active .tab")?.textContent).toContain("节点 A"));
    const source = openSource();
    const edited = `${sourceEditorValue(source)}durable before workspace switch\n`;
    replaceSource(source, edited);
    fireEvent.click(treeNode("节点 B"));
    await waitFor(() => expect(document.querySelector(".tab-item.active .tab")?.textContent).toContain("节点 B"));

    fireEvent.click(screen.getByRole("button", { name: "工作区设置与连接" }));
    fireEvent.click(screen.getByRole("button", { name: "切换工作区" }));
    await waitFor(() => expect(fixture.chooserCalls()).toBe(1));
    expect(fixture.savedDrafts).toContainEqual({ nodeId: "a", revision: "a-revision", source: edited });
    expect(fixture.events.indexOf("save:a")).toBeLessThan(fixture.events.indexOf("choose-workspace"));
  });

  it("flushes an immediately edited second pane, and both dirty panes, before switching desktop workspaces", async () => {
    async function runSwitch(primaryDirty: boolean, failingNodeId?: string) {
    const model = coreModel();
      const documents = {
        root: { nodeId: "root", name: "Workspace", revision: "root-revision", length: 24, source: "= Workspace\nprimary disk\n", model },
        a: { nodeId: "a", name: "节点 A", revision: "a-revision", length: 18, source: "= 节点 A\nsplit disk\n", model },
      };
      const workspace = {
        rootNodeId: "root", revision: "workspace-revision", presentation: { adjacentHeadingBody: "separate" as const },
        nodes: [
          { id: "root", name: "Workspace", parentId: null, path: "", trashed: false },
          { id: "a", name: "节点 A", parentId: "root", path: "节点 A", trashed: false },
        ], links: { outgoing: [], backlinks: [], potentialMentions: [] },
      };
      const nextWorkspace = {
        rootNodeId: "next-root", revision: "next-workspace-revision", presentation: { adjacentHeadingBody: "separate" as const },
        nodes: [{ id: "next-root", name: "Next Workspace", parentId: null, path: "", trashed: false }],
        links: { outgoing: [], backlinks: [], potentialMentions: [] },
      };
      const nextDocument = { nodeId: "next-root", name: "Next Workspace", revision: "next-revision", length: 16, source: "= Next Workspace", model };
      const savedDrafts: Array<Record<string, unknown>> = [];
      let draftsSeenByChooser = 0;
      window.localStorage.setItem("weftext.navigation.v1", JSON.stringify({ root: {
        version: 1, tabs: [{ id: "tab-root", nodeId: "root" }], activeTabId: "tab-root", back: [], forward: [],
        recent: ["root", "a"], bookmarks: [], split: { nodeId: "a", selectionStart: 0, selectionEnd: 0, scrollTop: 0, view: "source" },
      } }));
      window.weftextDesktop = {
        restoreWorkspace: async () => ({ opened: true, workspace, document: documents.root }),
        chooseWorkspace: async () => {
          draftsSeenByChooser = savedDrafts.length;
          return { opened: true, workspace: nextWorkspace, document: nextDocument };
        },
        request: async (path, body) => {
          if (path.startsWith("/api/document?")) return { document: documents.a };
          if (path === "/api/document/model") return { model };
          if (path === "/api/annotations") return { annotations: { version: 3, document_id: "root", annotations: [] } };
          if (path === "/api/draft/save") {
            if ((body as Record<string, unknown>).nodeId === failingNodeId) throw new Error("draft failure injection");
            savedDrafts.push(body as Record<string, unknown>);
            return { clean: false, draftRecovery: { drafts: [], issues: [] } };
          }
          return {};
        },
      };

      render(<Home />);
      const pane = await screen.findByRole("complementary", { name: "第二编辑栏" });
      const splitSource = await within(pane).findByRole("textbox", { name: "AsciiDoc 源码" });
      replaceSource(splitSource, `${sourceEditorValue(splitSource)}split immediate edit\n`);
      if (primaryDirty) {
        const primary = document.querySelector("article.document-surface");
        if (!primary) throw new Error("primary editor not rendered");
        fireEvent.click(screen.getAllByRole("button", { name: "源码" })[0]);
        const primarySource = within(primary as HTMLElement).getByRole("textbox", { name: "AsciiDoc 源码" });
        replaceSource(primarySource, `${sourceEditorValue(primarySource)}primary immediate edit\n`);
      }
      fireEvent.click(screen.getByRole("button", { name: "工作区设置与连接" }));
      fireEvent.click(screen.getByRole("button", { name: "切换工作区" }));

      const expectedCount = primaryDirty ? 2 : 1;
      if (failingNodeId) {
        await waitFor(() => expect(screen.getByRole("heading", { name: "草稿恢复中心" })).toBeTruthy());
        expect(draftsSeenByChooser).toBe(0);
        cleanup();
        window.localStorage.clear();
        delete window.weftextDesktop;
        return;
      }
      await waitFor(() => expect(treeNode("Next Workspace")).toBeTruthy());
      expect(draftsSeenByChooser).toBe(expectedCount);
      expect(savedDrafts).toEqual(expect.arrayContaining([
        expect.objectContaining({ nodeId: "a", revision: "a-revision", source: expect.stringContaining("split immediate edit") }),
      ]));
      if (primaryDirty) {
        expect(savedDrafts).toEqual(expect.arrayContaining([
          expect.objectContaining({ nodeId: "root", revision: "root-revision", source: expect.stringContaining("primary immediate edit") }),
        ]));
      }
      cleanup();
      window.localStorage.clear();
      delete window.weftextDesktop;
    }

    await runSwitch(false);
    await runSwitch(true);
    await runSwitch(true, "a");
  });

  it("keeps a stale restored second-pane draft out of the disk view and offers comparison", async () => {
    const model = coreModel();
    const staleDraft = {
      nodeId: "a", name: "节点 A", baseRevision: "old", currentRevision: "new", stale: true,
      length: 17, updatedAtUnixMs: 1_787_300_000_000, source: "= 节点 A\n过期草稿",
    };
    const documents = {
      root: { nodeId: "root", name: "Workspace", revision: "root-revision", length: 11, source: "= Workspace", model },
      a: { nodeId: "a", name: "节点 A", revision: "new", length: 17, source: "= 节点 A\n磁盘新版本", model, recoveryDraft: staleDraft },
    };
    const workspace = {
      rootNodeId: "root", revision: "workspace-revision", presentation: { adjacentHeadingBody: "separate" as const },
      nodes: [
        { id: "root", name: "Workspace", parentId: null, path: "", trashed: false },
        { id: "a", name: "节点 A", parentId: "root", path: "节点 A", trashed: false },
      ], links: { outgoing: [], backlinks: [], potentialMentions: [] },
    };
    window.localStorage.setItem("weftext.navigation.v1", JSON.stringify({ root: {
      version: 1, tabs: [{ id: "tab-root", nodeId: "root" }], activeTabId: "tab-root", back: [], forward: [],
      recent: ["root", "a"], bookmarks: [], split: { nodeId: "a", selectionStart: 0, selectionEnd: 0, scrollTop: 0, view: "source" },
    } }));
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace, document: documents.root, draftRecovery: { drafts: [staleDraft], issues: [] } }),
      chooseWorkspace: async () => null,
      request: async (path) => {
        if (path.startsWith("/api/document?")) return { document: documents.a };
        if (path === "/api/document/model") return { model };
        if (path === "/api/annotations") return { annotations: { version: 3, document_id: "root", annotations: [] } };
        return {};
      },
    };

    render(<Home />);
    const pane = await screen.findByRole("complementary", { name: "第二编辑栏" });
    const source = await within(pane).findByRole("textbox", { name: "AsciiDoc 源码" });
    expect(sourceEditorValue(source)).toContain("磁盘新版本");
    expect(sourceEditorValue(source)).not.toContain("过期草稿");
    expect(within(pane).getByRole("alert").textContent).toContain("未静默套用草稿");
    fireEvent.click(within(pane).getByRole("button", { name: "在主栏比较草稿" }));
    await waitFor(() => expect(screen.getByText("基于旧 revision，需要选择")).toBeTruthy());
    expect(screen.getByText((_, element) => element?.tagName === "PRE" && element.textContent?.includes("过期草稿") === true)).toBeTruthy();
  });

  it("previews, revision-checks, commits and conflicts a second-pane draft explicitly", async () => {
    const model = coreModel();
    const documents = {
      root: { nodeId: "root", name: "Workspace", revision: "root-revision", length: 23, source: "= Workspace\nroot source\n", model },
      a: { nodeId: "a", name: "节点 A", revision: "a-revision", length: 18, source: "= 节点 A\nalpha\n", model },
    };
    const workspace = {
      rootNodeId: "root", revision: "workspace-revision", presentation: { adjacentHeadingBody: "separate" as const },
      nodes: [
        { id: "root", name: "Workspace", parentId: null, path: "", trashed: false },
        { id: "a", name: "节点 A", parentId: "root", path: "节点 A", trashed: false },
      ], links: { outgoing: [], backlinks: [], potentialMentions: [] },
    };
    let committedBody: Record<string, unknown> | null = null;
    let failPreview = false;
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace, document: documents.root }),
      chooseWorkspace: async () => null,
      request: async (path, body) => {
        if (path.startsWith("/api/document?")) {
          const id = new URL(path, "http://desktop.local").searchParams.get("nodeId") as "root" | "a";
          return { document: documents[id] };
        }
        if (path === "/api/document/model") return { model };
        if (path === "/api/annotations") return { annotations: { version: 3, document_id: "root", annotations: [] } };
        if (path === "/api/draft/save") return { clean: false, draftRecovery: { drafts: [], issues: [] } };
        if (path === "/api/document/preview") {
          if (failPreview) throw new Error("stale document revision");
          return { plan: { baseRevision: "root-revision", nextRevision: "next-revision", oldLength: 23, newLength: 31, changed: true } };
        }
        if (path === "/api/document/commit") {
          committedBody = body as Record<string, unknown>;
          return { commit: { revision: "committed-revision", length: String(committedBody.source).length }, draftRecovery: { drafts: [], issues: [] } };
        }
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(treeNode("节点 A")).toBeTruthy());
    fireEvent.click(treeNode("节点 A"));
    await waitFor(() => expect(document.querySelector(".tab-item.active .tab")?.textContent).toContain("节点 A"));
    fireEvent.click(screen.getByRole("button", { name: "打开双栏分屏" }));
    const pane = await screen.findByRole("complementary", { name: "第二编辑栏" });
    fireEvent.click(within(pane).getByRole("button", { name: "源码" }));
    const source = await within(pane).findByRole("textbox", { name: "AsciiDoc 源码" });
    replaceSource(source, `${sourceEditorValue(source)}second pane edit\n`);
    await waitFor(() => expect((within(pane).getByRole("button", { name: "保存第二编辑栏" }) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(within(pane).getByRole("button", { name: "保存第二编辑栏" }));
    await waitFor(() => expect(screen.getByRole("heading", { name: "保存第二栏文档预览" })).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "确认提交" }));
    await waitFor(() => expect(committedBody).toMatchObject({ nodeId: "root", revision: "root-revision" }));
    expect(String(committedBody?.source)).toContain("second pane edit");

    failPreview = true;
    replaceSource(source, `${sourceEditorValue(source)}conflicting edit\n`);
    fireEvent.click(within(pane).getByRole("button", { name: "保存第二编辑栏" }));
    await waitFor(() => expect(screen.getByRole("heading", { name: "Core 已拒绝这次操作" })).toBeTruthy());
    expect(screen.getByText("stale document revision")).toBeTruthy();
  });

  it("collapses and restores descendants from the node tree", () => {
    render(<Home />);
    expect(treeNode("产品方向")).toBeTruthy();
    const overview = treeNode("项目总览");
    const row = overview.closest('[role="treeitem"]')!;
    const disclosure = within(row as HTMLElement).getByRole("button", { name: "折叠 项目总览" });
    expect(disclosure.querySelector(".tree-caret")?.classList.contains("expanded")).toBe(true);
    expect(overview.textContent).not.toMatch(/[▸▾]/);
    fireEvent.click(disclosure);
    expect(within(row as HTMLElement).getByRole("button", { name: "展开 项目总览" }).querySelector(".tree-caret")?.classList.contains("collapsed")).toBe(true);
    expect(within(screen.getByRole("tree", { name: "工作区层级" })).queryByRole("button", { name: "打开节点 产品方向" })).toBeNull();
    fireEvent.click(within(row as HTMLElement).getByRole("button", { name: "展开 项目总览" }));
    expect(treeNode("产品方向")).toBeTruthy();
  });

  it("finds and narrowly replaces visible document text", async () => {
    render(<Home />);
    const source = openSource();
    const systemPrefix = sourceEditorValue(source).slice(0, sourceEditorValue(source).indexOf("= 交互原则"));
    fireEvent.click(screen.getByRole("button", { name: "查找" }));
    const find = screen.getByRole("textbox", { name: "查找文本" });
    fireEvent.change(find, { target: { value: "路径" } });
    fireEvent.click(screen.getByRole("button", { name: "下一个匹配" }));

    await waitFor(() => {
      const selection = sourceEditorSelection(source);
      expect(sourceEditorValue(source).slice(selection.start, selection.end)).toBe("路径");
    });
    fireEvent.click(screen.getByRole("button", { name: "替换" }));
    fireEvent.change(screen.getByRole("textbox", { name: "替换文本" }), { target: { value: "节点路径" } });
    fireEvent.click(screen.getByRole("button", { name: "替换当前" }));

    expect(sourceEditorValue(source)).toContain("节点路径不是身份");
    expect(sourceEditorValue(source).startsWith(systemPrefix)).toBe(true);
    expect(screen.getByText("已替换当前匹配；草稿尚未提交")).toBeTruthy();
  });

  it("wraps document matches and replaces all exact occurrences", async () => {
    render(<Home />);
    const source = openSource();
    replaceSource(source, "= 标题\n目标和目标");
    fireEvent.click(screen.getByRole("button", { name: "查找" }));
    fireEvent.change(screen.getByRole("textbox", { name: "查找文本" }), { target: { value: "目标" } });
    fireEvent.click(screen.getByRole("button", { name: "下一个匹配" }));
    await waitFor(() => expect(sourceEditorSelection(source).start).toBe(sourceEditorValue(source).indexOf("目标")));
    fireEvent.click(screen.getByRole("button", { name: "下一个匹配" }));
    await waitFor(() => expect(sourceEditorSelection(source).start).toBe(sourceEditorValue(source).lastIndexOf("目标")));
    fireEvent.click(screen.getByRole("button", { name: "下一个匹配" }));
    await waitFor(() => expect(sourceEditorSelection(source).start).toBe(sourceEditorValue(source).indexOf("目标")));
    fireEvent.click(screen.getByRole("button", { name: "替换" }));
    fireEvent.change(screen.getByRole("textbox", { name: "替换文本" }), { target: { value: "结果" } });
    fireEvent.click(screen.getByRole("button", { name: "全部替换" }));
    expect(sourceEditorValue(source)).toBe("= 标题\n结果和结果");
  });

  it("opens document find from the keyboard and closes it with Escape", () => {
    render(<Home />);
    fireEvent.keyDown(window, { key: "f", ctrlKey: true });
    expect(screen.getByRole("search", { name: "查找当前文档" })).toBeTruthy();
    fireEvent.keyDown(screen.getByRole("textbox", { name: "查找文本" }), { key: "Escape" });
    expect(screen.queryByRole("search", { name: "查找当前文档" })).toBeNull();
  });

  it("finds frontmatter only when Source explicitly exposes it", () => {
    render(<Home />);
    openSource();
    fireEvent.click(screen.getByRole("button", { name: "查找" }));
    const findPanel = screen.getByRole("search", { name: "查找当前文档" });
    fireEvent.change(within(findPanel).getByRole("textbox", { name: "查找文本" }), { target: { value: "weftext:" } });
    expect(within(findPanel).getByText("1 / 1")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "写作" }));
    expect(within(findPanel).getByText("0 / 0")).toBeTruthy();
  });

  it("excludes fenced pseudo-headings from the prototype outline fallback", () => {
    render(<Home />);
    const source = openSource();
    replaceSource(source, "= 可见标题\n\n[source,adoc]\n----\n== 围栏内标题\n----\n\n== 第二个可见标题");
    fireEvent.click(screen.getByRole("button", { name: "大纲" }));
    const outline = screen.getByRole("navigation", { name: "文档大纲" });
    expect(within(outline).queryByRole("button", { name: /^可见标题/ })).toBeNull();
    expect(within(outline).getByRole("button", { name: /^第二个可见标题/ })).toBeTruthy();
    expect(within(outline).queryByRole("button", { name: /围栏内标题/ })).toBeNull();
  });

  it("keeps the source position across Write and Source and navigates from the outline", async () => {
    render(<Home />);
    const source = openSource();
    const sourceOffset = sourceEditorValue(source).indexOf("核心原则");
    selectSource(source, sourceOffset);

    fireEvent.click(screen.getByRole("button", { name: "写作" }));
    const write = screen.getByRole("textbox", { name: "AsciiDoc 正文" }) as HTMLTextAreaElement;
    const bodyOffset = write.value.indexOf("核心原则");
    await waitFor(() => expect(write.selectionStart).toBe(bodyOffset));
    fireEvent.click(screen.getByRole("button", { name: "源码" }));
    const reopenedSource = screen.getByRole("textbox", { name: "AsciiDoc 源码" });
    await waitFor(() => expect(sourceEditorSelection(reopenedSource).start).toBe(sourceOffset));

    fireEvent.click(screen.getByRole("button", { name: "大纲" }));
    const outline = screen.getByRole("navigation", { name: "文档大纲" });
    fireEvent.click(within(outline).getByRole("button", { name: /核心原则/ }));
    await waitFor(() => expect(sourceEditorSelection(reopenedSource).start).toBe(sourceEditorValue(reopenedSource).lastIndexOf("== 核心原则")));
    expect(within(outline).getByRole("button", { name: /核心原则/ }).getAttribute("aria-current")).toBe("location");
  });

  it("uses Core UTF-8 block ranges for a live Unicode outline", async () => {
    const source = "---\nweftext:\n  id: \"550e8400-e29b-41d4-a716-446655440000\"\n---\n:keywords: 中文\n\n== 中文标题\n\n正文\n\n=== 第二节\n内容";
    const firstStart = source.indexOf("== 中文标题");
    const secondStart = source.indexOf("=== 第二节");
    const byteAt = (offset: number) => new TextEncoder().encode(source.slice(0, offset)).length;
    const model = coreModel([
      coreBlock({ kind: "frontmatter", start: 0, end: byteAt(firstStart), textStart: 0, textEnd: 0, text: "" }),
      coreBlock({ kind: "heading", start: byteAt(firstStart), end: byteAt(firstStart + "== 中文标题".length), textStart: byteAt(firstStart + 3), textEnd: byteAt(firstStart + "== 中文标题".length), text: "中文标题", headingLevel: 1 }),
      coreBlock({ kind: "heading", start: byteAt(secondStart), end: byteAt(secondStart + "=== 第二节".length), textStart: byteAt(secondStart + 4), textEnd: byteAt(secondStart + "=== 第二节".length), text: "第二节", headingLevel: 2 }),
    ]);
    const workspace = {
      rootNodeId: "root",
      revision: "workspace-revision",
      presentation: { adjacentHeadingBody: "separate" as const },
      nodes: [{ id: "root", name: "中文", parentId: null, path: "", trashed: false }],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
    };
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace, document: { nodeId: "root", name: "中文", revision: "a".repeat(64), length: source.length, source, model } }),
      chooseWorkspace: async () => null,
      request: async (path, body) => {
        if (path === "/api/document/model") return { model };
        if (path === "/api/document/format") {
          expect(body).toMatchObject({
            start: byteAt(secondStart),
            end: byteAt(secondStart),
            command: { kind: "heading", level: 9 },
          });
          const next = source.replace("=== 第二节", "========== 第二节");
          return { plan: {
            profile: "ascii_doc_v1",
            source: next,
            selectionStart: byteAt(secondStart),
            selectionEnd: byteAt(secondStart + "========== 第二节".length),
            changed: true,
          } };
        }
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(screen.getByText("Core revision 已同步")).toBeTruthy());
    const write = screen.getByRole("textbox", { name: "AsciiDoc 正文" });
    expect(write.textContent).not.toContain("weftext:");
    expect(writeEditorValue(write)).toBe(source);
    const editor = openSource();
    fireEvent.click(screen.getByRole("button", { name: "大纲" }));
    const outline = screen.getByRole("navigation", { name: "文档大纲" });
    fireEvent.click(within(outline).getByRole("button", { name: /第二节/ }));
    await waitFor(() => expect(sourceEditorSelection(editor).start).toBe(secondStart));
    fireEvent.change(screen.getByRole("combobox", { name: "标题级别" }), { target: { value: "9" } });
    await waitFor(() => expect(sourceEditorValue(editor)).toContain("========== 第二节"));
    expect(sourceEditorValue(editor)).toContain("== 中文标题");
  });

  it("edits a live AsciiDoc profile only through the Core format plan", async () => {
    const source = "---\nweftext:\n  id: \"550e8400-e29b-41d4-a716-446655440000\"\n---\n== 标题\n正文 😀\n";
    const emoji = source.indexOf("😀");
    const byteAt = (offset: number, value = source) => new TextEncoder().encode(value.slice(0, offset)).length;
    const profile = {
      contractVersion: 2,
      profile: "ascii_doc_v1" as const,
      mediaType: "text/asciidoc" as const,
      canonicalExtension: "adoc" as const,
      capabilities: { exactSource: true, utf8SourceEdits: true, yamlEnvelope: true, maxHeadingLevel: 9, actualQuoteDepth: true, blockIds: true, managedLinks: true, protectedRegions: true, typedBlocks: true, typedInlines: true, nestedLists: true, typedTables: true, safeRenderInput: true, degradationReports: true },
    };
    const model = coreModel();
    const workspace = {
      rootNodeId: "root",
      revision: "workspace-revision",
      documentFormat: { generation: "ascii_doc_v1" as const, canonicalExtension: "adoc" as const, mediaType: "text/asciidoc" as const },
      presentation: { adjacentHeadingBody: "separate" as const },
      nodes: [{ id: "root", name: "AsciiDoc", parentId: null, path: "", trashed: false }],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
    };
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace, document: { nodeId: "root", name: "AsciiDoc", revision: "b".repeat(64), length: source.length, source, profile, model } }),
      chooseWorkspace: async () => null,
      request: async (path, body) => {
        if (path === "/api/document/model") return { profile, model };
        if (path.startsWith("/api/annotations")) return { annotations: { version: 3, document_id: "root", annotations: [] } };
        if (path === "/api/document/format") {
          expect(body).toMatchObject({
            start: byteAt(emoji),
            end: byteAt(emoji + "😀".length),
            command: { kind: "bold" },
          });
          const next = source.replace("😀", "*😀*");
          const selected = next.indexOf("😀");
          return { plan: {
            profile: "ascii_doc_v1",
            source: next,
            selectionStart: byteAt(selected, next),
            selectionEnd: byteAt(selected + "😀".length, next),
            changed: true,
          } };
        }
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(screen.getByText("Core revision 已同步")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "源码" }));
    const editor = screen.getByRole("textbox", { name: "AsciiDoc 源码" });
    selectSource(editor, emoji, emoji + "😀".length);
    expect(screen.getByRole("toolbar", { name: "光标处 AsciiDoc 格式" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "加粗" }));
    await waitFor(() => expect(sourceEditorValue(editor)).toContain("正文 *😀*"));
    expect(screen.getByText("AsciiDoc", { selector: ".statusbar span" })).toBeTruthy();
  });

  it("renders Weftext H1-H9 and actual quote depth with accessible Read semantics", async () => {
    const source = "===== H4\n======= H6\n======== H7\n========== H9\nrun in body\n>>>>>>>>> depth nine\n>>>>>>>>>>>> depth twelve\n";
    const blocks = [
      { kind: "heading", text: "H4", headingLevel: 4, quoteDepth: null },
      { kind: "heading", text: "H6", headingLevel: 6, quoteDepth: null },
      { kind: "heading", text: "H7", headingLevel: 7, quoteDepth: null },
      { kind: "heading", text: "H9", headingLevel: 9, quoteDepth: null },
      { kind: "paragraph", text: "run in body", headingLevel: null, quoteDepth: null },
      { kind: "quote", text: "depth nine", headingLevel: null, quoteDepth: 9 },
      { kind: "quote", text: "depth twelve", headingLevel: null, quoteDepth: 12 },
    ].map((block, index) => coreBlock({ ...block, kind: block.kind as DocumentBlockKind, start: index, end: index + 1, textStart: index, textEnd: index + 1 }));
    const model = { ...coreModel(blocks), runInGroups: [{ headingBlock: 3, bodyBlock: 4 }] };
    const workspace = {
      rootNodeId: "root", revision: "workspace-revision", presentation: { adjacentHeadingBody: "run_in" as const },
      nodes: [{ id: "root", name: "Heading document", parentId: null, path: "", trashed: false }],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
    };
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace, document: { nodeId: "root", name: "Heading document", revision: "a".repeat(64), length: source.length, source, model } }),
      chooseWorkspace: async () => null,
      request: async (path) => path === "/api/document/model" ? { model } : {},
    };

    render(<Home />);
    await waitFor(() => expect(screen.getByText("Core revision 已同步")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "阅读" }));
    expect(screen.getByRole("heading", { level: 4, name: "H4" }).tagName).toBe("H4");
    expect(screen.getByRole("heading", { level: 6, name: "H6" }).tagName).toBe("H6");
    expect(screen.getByRole("heading", { level: 7, name: "H7" }).tagName).toBe("DIV");
    expect(screen.getByRole("heading", { level: 9, name: "H9" }).tagName).toBe("DIV");
    expect(document.querySelector("h7, h8, h9")).toBeNull();
    expect(document.querySelector('[data-quote-depth="9"] blockquote blockquote')).toBeTruthy();
    expect(document.querySelector('[data-quote-depth="12"]')?.getAttribute("aria-label")).toContain("12");
  });

  it("opens the restored active-tab UUID instead of the backend remembered document", async () => {
    const model = coreModel();
    const documents = {
      root: { nodeId: "root", name: "Workspace", revision: "root-revision", length: 10, source: "= Workspace", model },
      a: { nodeId: "a", name: "Restored A", revision: "a-revision", length: 17, source: "= Restored active", model },
    };
    const workspace = {
      rootNodeId: "root", revision: "workspace-revision", presentation: { adjacentHeadingBody: "separate" as const },
      nodes: [
        { id: "root", name: "Workspace", parentId: null, path: "", trashed: false },
        { id: "a", name: "Restored A", parentId: "root", path: "Restored A", trashed: false },
      ],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
    };
    window.localStorage.setItem("weftext.navigation.v1", JSON.stringify({ root: {
      version: 1, tabs: [{ id: "tab-root", nodeId: "root" }, { id: "tab-a", nodeId: "a" }], activeTabId: "tab-a",
      back: [], forward: [], recent: ["a", "root"], bookmarks: [], split: null,
    } }));
    const documentRequests: string[] = [];
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace, document: documents.root }),
      chooseWorkspace: async () => null,
      request: async (path) => {
        if (path.startsWith("/api/document?")) {
          documentRequests.push(path);
          return { document: documents.a };
        }
        if (path === "/api/document/model") return { model };
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(document.querySelector(".tab-item.active .tab")?.textContent).toContain("Restored A"));
    expect(sourceEditorValue(openSource())).toBe("= Restored active");
    expect(documentRequests).toEqual(["/api/document?nodeId=a&remember=false"]);
  });

  it("commits one canonical icon scalar through a stored single-use metadata plan", async () => {
    const rootId = "550e8400-e29b-41d4-a716-446655440000";
    let revision = "a".repeat(64);
    let source = "---\nweftext:\n  id: \"550e8400-e29b-41d4-a716-446655440000\"\n  icon: \"weftext:project\"\n---\n= Workspace\n";
    let icon: { kind: "built_in"; value: string; glyph: string } | null = {
      kind: "built_in",
      value: "weftext:project",
      glyph: "项",
    };
    const metadata = () => ({ schema: "weftext.node-metadata.v1" as const, id: rootId, icon: icon?.value ?? null, resolvedIcon: icon, aliases: [], childSort: "name" as const, childSortDirection: "ascending" as const, siblingRank: null, adjacentHeadingBody: "separate" as const, diagnostics: [] });
    const model = coreModel([
      coreBlock({ kind: "frontmatter", start: 0, end: source.indexOf("= Workspace"), textStart: 0, textEnd: 0, text: "" }),
      coreBlock({ kind: "heading", start: source.indexOf("= Workspace"), end: source.length, textStart: source.indexOf("Workspace"), textEnd: source.length - 1, text: "Workspace", headingLevel: 1 }),
    ]);
    let workspace = {
      rootNodeId: rootId,
      revision: "workspace-revision",
      presentation: { adjacentHeadingBody: "separate" as const },
      nodes: [{ id: rootId, name: "Workspace", parentId: null, path: "", trashed: false, icon }],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
      iconCatalog: [{ id: "weftext:book", label: "书籍", glyph: "书" }, { id: "weftext:project", label: "项目", glyph: "项" }],
    };
    const metadataBodies: Array<Record<string, unknown>> = [];
    let pendingPlan = "";
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace, document: { nodeId: rootId, name: "Workspace", revision, length: source.length, source, model, metadata: metadata() } }),
      chooseWorkspace: async () => null,
      request: async (path, body) => {
        if (path === "/api/document/model") return { model };
        if (path === "/api/node/metadata/preview") {
          const request = body as Record<string, unknown>;
          metadataBodies.push(request);
          pendingPlan = `plan-${metadataBodies.length}`;
          return { plan: closedStructuralPlan({ planId: pendingPlan, action: "node_metadata", baseRevision: workspace.revision, documentChanges: [{ path: "Workspace.adoc", editCount: 1 }], targetNodeIds: [rootId], draftSensitiveNodeIds: [rootId] }) };
        }
        if (path === "/api/workspace/action/commit") {
          const request = body as { planId: string };
          expect(request.planId).toBe(pendingPlan);
          const intent = metadataBodies.at(-1)!;
          const iconProperty = /^ {2}icon:[^\n]*\n/m;
          if (intent.remove) {
            source = source.replace(iconProperty, "");
            icon = null;
          } else {
            source = source.replace(iconProperty, `  icon: ${JSON.stringify(intent.icon)}\n`);
            icon = { kind: "built_in", value: String(intent.icon), glyph: String(intent.icon) === "weftext:book" ? "书" : "项" };
          }
          revision = metadataBodies.length === 1 ? "b".repeat(64) : "c".repeat(64);
          workspace = { ...workspace, revision: `workspace-${metadataBodies.length}`, nodes: [{ ...workspace.nodes[0], icon }] };
          pendingPlan = "";
          return { workspace, commit: { revision } };
        }
        if (path.startsWith("/api/document?")) return { document: { nodeId: rootId, name: "Workspace", revision, length: source.length, source, model, metadata: metadata() } };
        if (path === "/api/draft/save") return { clean: false, draftRecovery: { drafts: [], issues: [] } };
        if (path.startsWith("/api/annotations")) return { annotations: { version: 3, document_id: rootId, annotations: [] } };
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(screen.getByText("Core revision 已同步")).toBeTruthy());
    expect(treeNode("Workspace").querySelector(".node-icon")?.textContent).toBe("项");
    fireEvent.click(screen.getByRole("button", { name: "设置节点图标" }));
    expect(screen.getByText(/只接受单个 emoji/)).toBeTruthy();
    fireEvent.change(screen.getByRole("textbox", { name: "搜索图标" }), { target: { value: "书籍" } });
    fireEvent.click(screen.getByRole("button", { name: "选择图标 书籍" }));
    await waitFor(() => expect(metadataBodies[0]).toMatchObject({ action: "icon", icon: "weftext:book", nodeId: rootId, revision: "a".repeat(64) }));
    expect(screen.getByRole("dialog", { name: "修改节点元数据预览" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "确认提交事务" }));
    await waitFor(() => expect(treeNode("Workspace").querySelector(".node-icon")?.textContent).toBe("书"));
    await waitFor(() => expect(screen.getByText("rev bbbbbbbb")).toBeTruthy());
    expect(source).toContain('  icon: "weftext:book"');

    fireEvent.click(screen.getByRole("button", { name: "设置节点图标" }));
    fireEvent.change(screen.getByRole("combobox", { name: "节点图标列表位置" }), { target: { value: "after" } });
    const recentButton = within(screen.getByRole("region", { name: "最近节点" })).getByRole("button", { name: "Workspace" });
    expect(recentButton.firstChild?.textContent).toContain("Workspace");
    expect(recentButton.lastElementChild?.textContent).toBe("书");
    fireEvent.change(screen.getByRole("combobox", { name: "节点图标列表位置" }), { target: { value: "hidden" } });
    expect(treeNode("Workspace").querySelector(".node-icon")).toBeNull();
    expect(document.querySelector(".title-node-icon")?.textContent).toBe("书");
    fireEvent.click(screen.getByRole("checkbox", { name: "在文档标题显示图标" }));
    expect(document.querySelector(".title-node-icon")).toBeNull();
    expect(JSON.parse(window.localStorage.getItem("weftext.icon-preferences.v1") ?? "{}").placement).toBe("hidden");
    fireEvent.click(screen.getByRole("button", { name: "清除便携图标" }));
    await waitFor(() => expect(metadataBodies.at(-1)).toMatchObject({ action: "icon", remove: true, nodeId: rootId, revision: "b".repeat(64) }));
    fireEvent.click(screen.getByRole("button", { name: "确认提交事务" }));
    await waitFor(() => expect(source).not.toContain("  icon:"));
  });

  it("keeps CJK alias IME input exact and leaves metadata commit disabled in Safe Mode", async () => {
    const rootId = "11111111-1111-4111-8111-111111111111";
    const revision = "a".repeat(64);
    const source = `---\nweftext:\n  id: "${rootId}"\n  aliases:\n    - "旧别名"\n---\n= Workspace\n:status: review\n`;
    const model = coreModel();
    const metadata = {
      schema: "weftext.node-metadata.v1" as const,
      id: rootId,
      icon: "😀",
      resolvedIcon: { kind: "emoji" as const, value: "😀", glyph: "😀" },
      aliases: ["旧别名"],
      childSort: "name" as const,
      childSortDirection: "descending" as const,
      siblingRank: null,
      adjacentHeadingBody: "separate" as const,
      diagnostics: [],
    };
    const properties = {
      properties: [{ name: "status", value: "review", kind: "custom" as const, range: { start: 90, end: 106 }, nameRange: { start: 91, end: 97 }, valueRange: { start: 99, end: 105 } }],
      diagnostics: [],
      headerRange: { start: 81, end: 106 },
    };
    const workspace = {
      rootNodeId: rootId,
      revision: "workspace-revision",
      presentation: { adjacentHeadingBody: "separate" as const },
      nodes: [{ id: rootId, name: "Workspace", parentId: null, path: "", trashed: false, icon: metadata.resolvedIcon }],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
      iconCatalog: [],
    };
    const previews: Array<Record<string, unknown>> = [];
    let commitCalls = 0;
    window.weftextDesktop = {
      restoreWorkspace: async () => ({
        opened: true,
        safeMode: true,
        workspace,
        document: { nodeId: rootId, name: "Workspace", revision, length: source.length, source, model, metadata, properties },
      }),
      chooseWorkspace: async () => null,
      request: async (path, body) => {
        if (path === "/api/document/model") return { model, properties };
        if (path.startsWith("/api/annotations")) return { annotations: { version: 3, document_id: rootId, annotations: [] } };
        if (path === "/api/node/metadata/preview") {
          previews.push(body as Record<string, unknown>);
          return { plan: closedStructuralPlan({ planId: "aliases-safe-mode", action: "node_metadata", baseRevision: workspace.revision, documentChanges: [{ path: "Workspace.adoc", editCount: 1 }], targetNodeIds: [rootId], draftSensitiveNodeIds: [rootId] }) };
        }
        if (path === "/api/workspace/action/commit") {
          commitCalls += 1;
          return {};
        }
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(screen.getByText("Core revision 已同步")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "Properties" }));
    const systemMetadata = screen.getByRole("group", { name: "节点系统元数据" });
    expect(within(systemMetadata).getByText(rootId)).toBeTruthy();
    expect(within(systemMetadata).getByText("😀")).toBeTruthy();
    expect((within(systemMetadata).getByRole("combobox", { name: "直接子节点排序" }) as HTMLSelectElement).value).toBe("name");
    expect((within(systemMetadata).getByRole("combobox", { name: "子节点排序方向" }) as HTMLSelectElement).value).toBe("descending");
    expect(within(systemMetadata).getByText("分开")).toBeTruthy();
    expect(within(screen.getByRole("region", { name: "文档属性" })).getByText("status")).toBeTruthy();

    const aliases = within(systemMetadata).getByRole("textbox", { name: "节点别名（每行一个）" });
    fireEvent.compositionStart(aliases);
    fireEvent.change(aliases, { target: { value: "中文别名\n😀 团队" } });
    fireEvent.keyDown(aliases, { key: "Enter", ctrlKey: true, isComposing: true });
    expect(previews).toHaveLength(0);
    fireEvent.compositionEnd(aliases, { data: "团队" });
    fireEvent.keyDown(aliases, { key: "Enter", ctrlKey: true, isComposing: false });
    await waitFor(() => expect(previews).toHaveLength(1));
    expect(previews[0]).toEqual({
      action: "aliases",
      aliases: ["中文别名", "😀 团队"],
      nodeId: rootId,
      revision,
    });
    const dialog = screen.getByRole("dialog", { name: "修改节点元数据预览" });
    const commit = within(dialog).getByRole("button", { name: "安全模式：提交已暂停" });
    expect((commit as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(commit);
    expect(commitCalls).toBe(0);
  });

  it("ignores a late metadata preview after node navigation and keeps the child rank projection", async () => {
    const rootId = "22222222-2222-4222-8222-222222222222";
    const childId = "33333333-3333-4333-8333-333333333333";
    const model = coreModel();
    const projection = (id: string, root: boolean) => ({
      schema: "weftext.node-metadata.v1" as const,
      id,
      icon: null,
      resolvedIcon: null,
      aliases: [],
      childSort: "name" as const,
      childSortDirection: "ascending" as const,
      siblingRank: root ? null : 2048,
      adjacentHeadingBody: root ? "separate" as const : null,
      diagnostics: [],
    });
    const documents = {
      root: { nodeId: rootId, name: "Workspace", revision: "a".repeat(64), length: 11, source: "= Workspace", model, metadata: projection(rootId, true) },
      child: { nodeId: childId, name: "节点 B", revision: "b".repeat(64), length: 7, source: "= 节点 B", model, metadata: projection(childId, false) },
    };
    const workspace = {
      rootNodeId: rootId,
      revision: "workspace-revision",
      presentation: { adjacentHeadingBody: "separate" as const },
      nodes: [
        { id: rootId, name: "Workspace", parentId: null, path: "", trashed: false, icon: null },
        { id: childId, name: "节点 B", parentId: rootId, path: "节点 B", trashed: false, icon: null },
      ],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
      iconCatalog: [],
    };
    let resolvePreview!: (value: unknown) => void;
    const pendingPreview = new Promise((resolve) => { resolvePreview = resolve; });
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace, document: documents.root }),
      chooseWorkspace: async () => null,
      request: async (path) => {
        if (path === "/api/document/model") return { model };
        if (path === "/api/node/metadata/preview") return pendingPreview;
        if (path.startsWith("/api/document?")) return { document: documents.child };
        if (path.startsWith("/api/annotations")) return { annotations: { version: 3, document_id: rootId, annotations: [] } };
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(screen.getByText("Core revision 已同步")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "Properties" }));
    fireEvent.click(screen.getByRole("button", { name: "预览别名事务" }));
    fireEvent.click(treeNode("节点 B"));
    await waitFor(() => expect(document.querySelector(".tab-item.active .tab")?.textContent).toContain("节点 B"));
    await act(async () => {
      resolvePreview({ plan: closedStructuralPlan({ planId: "late", action: "node_metadata", baseRevision: workspace.revision, documentChanges: [{ path: "Workspace.adoc", editCount: 1 }], targetNodeIds: [rootId], draftSensitiveNodeIds: [rootId] }) });
      await Promise.resolve();
    });
    expect(screen.queryByRole("dialog", { name: "修改节点元数据预览" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Properties" }));
    expect((screen.getByRole("textbox", { name: "同级稀疏 rank" }) as HTMLInputElement).value).toBe("2048");
  });

  it("shows a current metadata preview failure without invoking workspace commit", async () => {
    const rootId = "44444444-4444-4444-8444-444444444444";
    const model = coreModel();
    const metadata = {
      schema: "weftext.node-metadata.v1" as const,
      id: rootId,
      icon: null,
      resolvedIcon: null,
      aliases: [],
      childSort: "name" as const,
      childSortDirection: "ascending" as const,
      siblingRank: null,
      adjacentHeadingBody: "run_in" as const,
      diagnostics: [],
    };
    const workspace = {
      rootNodeId: rootId,
      revision: "workspace-revision",
      presentation: { adjacentHeadingBody: "run_in" as const },
      nodes: [{ id: rootId, name: "Workspace", parentId: null, path: "", trashed: false, icon: null }],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
      iconCatalog: [],
    };
    let commitCalls = 0;
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace, document: { nodeId: rootId, name: "Workspace", revision: "a".repeat(64), length: 11, source: "= Workspace", model, metadata } }),
      chooseWorkspace: async () => null,
      request: async (path) => {
        if (path === "/api/document/model") return { model };
        if (path === "/api/node/metadata/preview") throw new Error("stale metadata revision");
        if (path === "/api/workspace/action/commit") commitCalls += 1;
        if (path.startsWith("/api/annotations")) return { annotations: { version: 3, document_id: rootId, annotations: [] } };
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(screen.getByText("Core revision 已同步")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "Properties" }));
    fireEvent.click(screen.getByRole("button", { name: "预览排序事务" }));
    const dialog = await screen.findByRole("dialog", { name: "Core 已拒绝这次操作" });
    expect(within(dialog).getByText("stale metadata revision")).toBeTruthy();
    expect(commitCalls).toBe(0);
  });

  it("ignores an older node response that arrives after the latest click", async () => {
    const emptyModel = coreModel();
    const workspace = {
      rootNodeId: "root",
      revision: "workspace-revision",
      presentation: { adjacentHeadingBody: "separate" as const },
      nodes: [
        { id: "root", name: "Workspace", parentId: null, path: "", trashed: false },
        { id: "a", name: "节点 A", parentId: "root", path: "节点 A", trashed: false },
        { id: "b", name: "节点 B", parentId: "root", path: "节点 B", trashed: false },
      ],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
    };
    const pending = new Map<string, (value: unknown) => void>();
    window.weftextDesktop = {
      restoreWorkspace: async () => ({
        opened: true,
        workspace,
        document: { nodeId: "root", name: "Workspace", revision: "root-revision", length: 0, source: "= Workspace", model: emptyModel },
      }),
      chooseWorkspace: async () => null,
      request: async (path) => {
        if (path === "/api/document/model") return { model: emptyModel };
        if (path.startsWith("/api/document?")) {
          const id = new URL(path, "http://desktop.local").searchParams.get("nodeId") ?? "";
          return new Promise((resolve) => pending.set(id, resolve));
        }
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(treeNode("节点 A")).toBeTruthy());
    fireEvent.click(treeNode("节点 A"));
    fireEvent.click(treeNode("节点 B"));

    await act(async () => {
      pending.get("b")?.({ document: { nodeId: "b", name: "节点 B", revision: "b-revision", length: 8, source: "= 节点 B", model: emptyModel } });
      await Promise.resolve();
      pending.get("a")?.({ document: { nodeId: "a", name: "节点 A", revision: "a-revision", length: 8, source: "= 节点 A", model: emptyModel } });
      await Promise.resolve();
    });

    expect(sourceEditorValue(openSource())).toContain("= 节点 B");
  });

  it("restores a current device draft after a desktop restart", async () => {
    const emptyModel = coreModel();
    const workspace = {
      rootNodeId: "root",
      revision: "workspace-revision",
      presentation: { adjacentHeadingBody: "separate" as const },
      nodes: [{ id: "root", name: "Workspace", parentId: null, path: "", trashed: false }],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
    };
    const recoveryDraft = {
      nodeId: "root",
      name: "Workspace",
      baseRevision: "a".repeat(64),
      currentRevision: "a".repeat(64),
      stale: false,
      length: 18,
      updatedAtUnixMs: 1_787_300_000_000,
      source: "= Workspace\n恢复草稿",
    };
    const draftRecovery = { drafts: [recoveryDraft], issues: [] };
    window.weftextDesktop = {
      restoreWorkspace: async () => ({
        opened: true,
        workspace,
        document: { nodeId: "root", name: "Workspace", revision: "a".repeat(64), length: 11, source: "= Workspace", model: emptyModel, recoveryDraft },
        draftRecovery,
        safeMode: false,
      }),
      chooseWorkspace: async () => null,
      request: async (path) => {
        if (path === "/api/document/model") return { model: emptyModel };
        if (path === "/api/draft/save") return { clean: false, draftRecovery };
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(screen.getByRole("heading", { name: "草稿恢复中心" })).toBeTruthy());
    expect(screen.getByText("基于当前 revision")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "继续编辑草稿" }));
    expect(sourceEditorValue(openSource())).toContain("恢复草稿");
    await waitFor(() => expect(screen.getByText("恢复草稿已存本机 · 预览提交")).toBeTruthy());
  });

  it("compares a stale recovery draft before replacing the disk view", async () => {
    const emptyModel = coreModel();
    const workspace = {
      rootNodeId: "root",
      revision: "workspace-revision",
      presentation: { adjacentHeadingBody: "separate" as const },
      nodes: [{ id: "root", name: "Workspace", parentId: null, path: "", trashed: false }],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
    };
    const recoveryDraft = {
      nodeId: "root",
      name: "Workspace",
      baseRevision: "a".repeat(64),
      currentRevision: "b".repeat(64),
      stale: true,
      length: 19,
      updatedAtUnixMs: 1_787_300_000_000,
      source: "= Workspace\n本地草稿",
    };
    const draftRecovery = { drafts: [recoveryDraft], issues: [] };
    const calls: string[] = [];
    window.weftextDesktop = {
      restoreWorkspace: async () => ({
        opened: true,
        workspace,
        document: { nodeId: "root", name: "Workspace", revision: "b".repeat(64), length: 19, source: "= Workspace\n磁盘外部修改", model: emptyModel, recoveryDraft },
        draftRecovery,
        safeMode: false,
      }),
      chooseWorkspace: async () => null,
      request: async (path) => {
        calls.push(path);
        if (path === "/api/document/model") return { model: emptyModel };
        if (path === "/api/draft/save") return { clean: false, draftRecovery: { drafts: [{ ...recoveryDraft, stale: false, currentRevision: "b".repeat(64) }], issues: [] } };
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(screen.getByText("基于旧 revision，需要选择")).toBeTruthy());
    expect(screen.getByText((_, element) => element?.tagName === "PRE" && element.textContent?.includes("磁盘外部修改") === true)).toBeTruthy();
    expect(screen.getByText((_, element) => element?.tagName === "PRE" && element.textContent?.includes("本地草稿") === true)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "恢复草稿继续编辑" }));
    expect(sourceEditorValue(openSource())).toContain("本地草稿");
    expect(sourceEditorValue(openSource())).not.toContain("磁盘外部修改");
    await waitFor(() => expect(calls).toContain("/api/draft/save"));
  });

  it("keeps device drafts writable while Safe Mode blocks workspace commits", async () => {
    const emptyModel = coreModel();
    const workspace = {
      rootNodeId: "root",
      revision: "workspace-revision",
      presentation: { adjacentHeadingBody: "separate" as const },
      nodes: [{ id: "root", name: "Workspace", parentId: null, path: "", trashed: false }],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
    };
    const calls: string[] = [];
    window.weftextDesktop = {
      restoreWorkspace: async () => ({
        opened: true,
        workspace,
        document: { nodeId: "root", name: "Workspace", revision: "a".repeat(64), length: 11, source: "= Workspace", model: emptyModel },
        draftRecovery: { drafts: [], issues: [] },
        safeMode: true,
      }),
      chooseWorkspace: async () => null,
      request: async (path) => {
        calls.push(path);
        if (path === "/api/document/model") return { model: emptyModel };
        if (path === "/api/draft/save") return { clean: false, draftRecovery: { drafts: [], issues: [] } };
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(screen.getByText("安全模式")).toBeTruthy());
    const source = openSource();
    replaceSource(source, `${sourceEditorValue(source)}\n设备草稿`);
    await waitFor(() => expect(calls).toContain("/api/draft/save"));
    fireEvent.click(screen.getByText("恢复草稿已存本机 · 预览提交"));
    expect(await screen.findByText("安全模式已启用；草稿已保留，但不会提交到工作区")).toBeTruthy();
    expect(calls).not.toContain("/api/document/preview");
    expect(calls).not.toContain("/api/document/commit");
  });

  it("moves a stale save into disk-versus-draft recovery comparison", async () => {
    const emptyModel = coreModel();
    const workspace = {
      rootNodeId: "root",
      revision: "workspace-revision",
      presentation: { adjacentHeadingBody: "separate" as const },
      nodes: [{ id: "root", name: "Workspace", parentId: null, path: "", trashed: false }],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
    };
    const oldRevision = "a".repeat(64);
    const newRevision = "b".repeat(64);
    const localSource = "= Workspace\n本地待保存";
    const diskSource = "= Workspace\n外部已经保存";
    const recoveryDraft = {
      nodeId: "root",
      name: "Workspace",
      baseRevision: oldRevision,
      currentRevision: newRevision,
      stale: true,
      length: localSource.length,
      updatedAtUnixMs: 1_787_300_000_000,
      source: localSource,
    };
    const draftRecovery = { drafts: [recoveryDraft], issues: [] };
    window.weftextDesktop = {
      restoreWorkspace: async () => ({
        opened: true,
        workspace,
        document: { nodeId: "root", name: "Workspace", revision: oldRevision, length: 11, source: "= Workspace", model: emptyModel },
        draftRecovery: { drafts: [], issues: [] },
        safeMode: false,
      }),
      chooseWorkspace: async () => null,
      request: async (path) => {
        if (path === "/api/document/model") return { model: emptyModel };
        if (path === "/api/draft/save") return { clean: false, draftRecovery };
        if (path === "/api/document/preview") throw new Error("stale document revision");
        if (path.startsWith("/api/document?")) {
          return { document: { nodeId: "root", name: "Workspace", revision: newRevision, length: diskSource.length, source: diskSource, model: emptyModel, recoveryDraft } };
        }
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(screen.getByText("Core revision 已同步")).toBeTruthy());
    const source = openSource();
    replaceSource(source, localSource);
    await waitFor(() => expect(screen.getByText("恢复草稿已存本机 · 预览提交")).toBeTruthy());
    fireEvent.click(screen.getByText("恢复草稿已存本机 · 预览提交"));
    await waitFor(() => expect(screen.getByRole("heading", { name: "Core 已拒绝这次操作" })).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "读取磁盘版本并比较" }));
    await waitFor(() => expect(screen.getByText("基于旧 revision，需要选择")).toBeTruthy());
    expect(screen.getByText((_, element) => element?.tagName === "PRE" && element.textContent?.includes("外部已经保存") === true)).toBeTruthy();
    expect(screen.getByText((_, element) => element?.tagName === "PRE" && element.textContent?.includes("本地待保存") === true)).toBeTruthy();
  });

  it("shares Hierarchy and Contents semantics without turning unmanaged browsing into document navigation", async () => {
    const model = coreModel();
    const revision = "a".repeat(64);
    const documents = {
      root: { nodeId: "root", name: "Workspace", revision, length: 11, source: "= Workspace", model },
      a: { nodeId: "a", name: "节点 A", revision, length: 12, source: "= 节点 A\n正文", model },
      b: { nodeId: "b", name: "节点 B", revision, length: 12, source: "= 节点 B\n正文", model },
    };
    const hierarchy = [
      { nodeId: "root", name: "Workspace", parentNodeId: null, locator: "", depth: 0, childCount: 1, displayIcon: { kind: "workspace_root" as const } },
      { nodeId: "a", name: "节点 A", parentNodeId: "root", locator: "节点 A", depth: 1, childCount: 1, displayIcon: { kind: "default_node" as const } },
      { nodeId: "b", name: "节点 B", parentNodeId: "a", locator: "节点 A/节点 B", depth: 2, childCount: 0, displayIcon: { kind: "default_node" as const } },
    ];
    const contents = [
      ...hierarchy.map((node) => ({ kind: "managed_node" as const, name: node.name, locator: node.locator, parentLocator: node.locator.includes("/") ? node.locator.slice(0, node.locator.lastIndexOf("/")) : node.locator ? "" : null, nodeId: node.nodeId, ownerNodeId: null, displayIcon: node.displayIcon })),
      { kind: "unmanaged_directory" as const, name: "Files", locator: "节点 A/Files", parentLocator: "节点 A", nodeId: null, ownerNodeId: null, displayIcon: { kind: "folder" as const } },
      { kind: "unmanaged_markdown" as const, name: "inside.md", locator: "节点 A/Files/inside.md", parentLocator: "节点 A/Files", nodeId: null, ownerNodeId: null, displayIcon: { kind: "markdown_file" as const } },
      { kind: "resource" as const, name: "asset.bin", locator: "节点 A/Files/asset.bin", parentLocator: "节点 A/Files", nodeId: null, ownerNodeId: null, displayIcon: { kind: "file" as const } },
    ];
    const workspace = {
      rootNodeId: "root", revision: "workspace", presentation: { adjacentHeadingBody: "separate" as const },
      nodes: hierarchy.map((node) => ({ id: node.nodeId, name: node.name, parentId: node.parentNodeId, path: node.locator, trashed: false, icon: null, displayIcon: node.displayIcon })),
      content: contents.map((item) => ({ kind: item.kind, name: item.name, path: item.locator, parentPath: item.parentLocator, nodeId: item.nodeId, ownerNodeId: null, displayIcon: item.displayIcon })),
      navigation: { version: 1 as const, rootNodeId: "root", hierarchy, contents },
      links: { outgoing: [], backlinks: [], potentialMentions: [] }, iconCatalog: [],
    };
    const opened: string[] = [];
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace, document: documents.a, draftRecovery: { drafts: [], issues: [] } }),
      chooseWorkspace: async () => null,
      request: async (path) => {
        if (path === "/api/document/model") return { model };
        if (path === "/api/draft/save") return { clean: true, draftRecovery: { drafts: [], issues: [] } };
        if (path.startsWith("/api/document?")) {
          const id = new URL(path, "http://desktop.local").searchParams.get("nodeId") as keyof typeof documents;
          opened.push(id);
          return { document: documents[id] };
        }
        if (path.startsWith("/api/annotations")) return { annotations: { version: 3, document_id: "a", annotations: [] } };
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(screen.getByText("Core revision 已同步")).toBeTruthy());
    const tree = screen.getByRole("tree", { name: "工作区层级" });
    const disclosure = within(tree).getByRole("button", { name: "折叠 节点 A" });
    fireEvent.click(disclosure);
    expect(opened).toEqual([]);
    expect(screen.getByRole("heading", { name: "节点 A" })).toBeTruthy();
    fireEvent.click(within(tree).getByRole("button", { name: "展开 节点 A" }));
    fireEvent.keyDown(treeNode("节点 A"), { key: "ArrowDown" });
    await waitFor(() => expect(document.activeElement).toBe(treeNode("节点 B")));

    fireEvent.click(screen.getByRole("tab", { name: "内容" }));
    expect(screen.getByText(/跟随主编辑栏：节点 A/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "浏览文件夹 Files" }));
    expect(screen.getByText("节点 A/Files")).toBeTruthy();
    expect(screen.getByLabelText("inside.md，非托管 Markdown，只读")).toBeTruthy();
    expect(screen.getByLabelText("asset.bin，节点资源，只读")).toBeTruthy();
    expect(opened).toEqual([]);
    expect(screen.getByRole("heading", { name: "节点 A" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "回到当前节点" }));
    expect(screen.queryByRole("button", { name: "回到当前节点" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "打开双栏分屏" }));
    const split = await screen.findByRole("complementary", { name: "第二编辑栏" });
    const splitSelector = within(split).getByRole("combobox", { name: "切换第二编辑栏节点" });
    fireEvent.change(splitSelector, { target: { value: "b" } });
    fireEvent.focus(splitSelector);
    await waitFor(() => expect(screen.getByText(/跟随第二编辑栏：节点 B/)).toBeTruthy());
  });

  it("projects Core citation rich text and submits UUID-based macro edit intent", async () => {
    const source = "= Main\n\nSee cite:[@smith].\n\nbibliography:: cited[]\n";
    const citationStart = source.indexOf("cite:[@smith]");
    const bibliographyStart = source.indexOf("bibliography:: cited[]");
    const model = coreModel([
      coreBlock({ kind: "heading", start: 0, end: 7, textStart: 2, textEnd: 6, text: "Main", headingLevel: 1 }),
      coreBlock({ kind: "paragraph", start: 8, end: 27, textStart: 8, textEnd: 27, text: "See cite:[@smith]." }),
      coreBlock({ kind: "paragraph", start: bibliographyStart, end: source.length - 1, textStart: bibliographyStart, textEnd: source.length - 1, text: "bibliography:: cited[]" }),
    ]);
    const workspace = {
      rootNodeId: "root", revision: "workspace-revision", presentation: { adjacentHeadingBody: "separate" as const },
      documentFormat: { generation: "ascii_doc_v1" as const, canonicalExtension: "adoc" as const, mediaType: "text/asciidoc" as const },
      nodes: [
        { id: "root", name: "Workspace", parentId: null, path: "", trashed: false },
        { id: "doc", name: "Main", parentId: "root", path: "Main", trashed: false },
        { id: "ref-uuid", name: "Smith 2024", parentId: "root", path: "Smith 2024", trashed: false },
        { id: "ref-jones", name: "Jones 2025", parentId: "root", path: "Jones 2025", trashed: false },
      ],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
    };
    const document = { nodeId: "doc", name: "Main", revision: "a".repeat(64), length: source.length, source, model };
    const richStyle = { italic: false, smallCaps: false, weight: "normal" as const, underline: false, verticalAlign: "none" as const };
    const draft = {
      authoring: { reference: { citationData: null, diagnostics: [] }, citations: { diagnostics: [] } },
      analysis: { diagnostics: [], clusters: [{ form: "parenthetical" as const, range: { start: citationStart, end: citationStart + "cite:[@smith]".length }, items: [{ label: "page", locator: null, prefix: null, suffix: null, reference: { nodeId: "ref-uuid", citationData: { key: "smith", itemType: "book", title: "Test Book", fields: {} } } }] }], nocites: [], bibliography: { range: { start: bibliographyStart, end: bibliographyStart + "bibliography:: cited[]".length }, inclusion: "cited" as const } }, presentationFailure: null,
      presentation: {
        providerId: "weftext-offline-csl", providerVersion: "1", profile: { styleId: "apa", locale: "en-US" },
        components: [{
          componentNodeId: "doc", revision: "a".repeat(64),
          citations: [{ sourceRange: { start: citationStart, end: citationStart + "cite:[@smith]".length }, form: "parenthetical" as const, noteNumber: null, referenceNodeIds: ["ref-uuid"], content: { runs: [{ text: "(Smith, 2024)", style: richStyle, link: null, referenceNodeId: "ref-uuid" }] } }],
          bibliography: { sourceRange: { start: bibliographyStart, end: bibliographyStart + "bibliography:: cited[]".length }, hangingIndent: true, secondFieldAlign: null, lineSpacing: 1, entrySpacing: 1, entries: [{ referenceNodeId: "ref-uuid", firstField: null, content: { runs: [{ text: "Smith. Test Book. 2024.", style: richStyle, link: null, referenceNodeId: "ref-uuid" }] } }] },
        }],
      },
    };
    let macroBody: { target: { range: { start: number; end: number } }; intent: unknown } | null = null;
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace, document, draftRecovery: { drafts: [], issues: [] } }),
      chooseWorkspace: async () => null,
      request: async (path, body) => {
        if (path === "/api/document/model") return { model };
        if (path.startsWith("/api/annotations")) return { annotations: { version: 3, document_id: "doc", annotations: [] } };
        if (path === "/api/citation/analyze") return draft;
        if (path.startsWith("/api/citation/search?")) return path.includes("Jones")
          ? { references: [{ nodeId: "ref-jones", key: "jones", itemType: "article-journal", title: "Second Study", contributors: ["Jones"], identifiers: {}, selectable: true, matchedFields: ["title"] }] }
          : { references: [{ nodeId: "ref-uuid", key: "smith", itemType: "book", title: "Test Book", contributors: ["Smith"], identifiers: {}, selectable: true, matchedFields: ["title"] }] };
        if (path === "/api/citation/macro-edit-preview") {
          macroBody = body as { target: { range: { start: number; end: number } }; intent: unknown };
          const target = macroBody.target;
          const replacement = "cite:[@smith,label=page,locator=9]+[jones]";
          return { plan: { proposedSource: `${source.slice(0, target.range.start)}${replacement}${source.slice(target.range.end)}`, edit: { ...target.range, replacement } } };
        }
        if (path === "/api/draft/save") return { clean: false, draftRecovery: { drafts: [], issues: [] } };
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(screen.getByText("Core revision 已同步")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "阅读" }));
    expect(await screen.findByText("(Smith, 2024)")).toBeTruthy();
    expect(screen.getByText("Smith. Test Book. 2024.")).toBeTruthy();

    const editor = openSource();
    selectSource(editor, citationStart, citationStart + "cite:[@smith]".length);
    fireEvent.click(screen.getByRole("button", { name: "引用" }));
    const dialog = screen.getByRole("dialog", { name: "插入引用" });
    fireEvent.click(within(dialog).getByRole("button", { name: "移除 Test Book" }));
    const citationSearch = within(dialog).getByRole("combobox", { name: "搜索参考文献" });
    fireEvent.change(citationSearch, { target: { value: "Smith" } });
    const smithOption = await within(dialog).findByRole("option", { name: /Test Book/ });
    fireEvent.keyDown(citationSearch, { key: "Enter", isComposing: true });
    expect(smithOption.getAttribute("aria-selected")).toBe("false");
    fireEvent.keyDown(citationSearch, { key: "Enter" });
    expect(smithOption.getAttribute("aria-selected")).toBe("true");
    fireEvent.change(within(dialog).getByPlaceholderText("例如 42–44"), { target: { value: "9" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "加入引用项" }));
    fireEvent.change(within(dialog).getByRole("combobox", { name: "搜索参考文献" }), { target: { value: "Jones" } });
    fireEvent.click(await within(dialog).findByRole("option", { name: /Second Study/ }));
    fireEvent.click(within(dialog).getByRole("button", { name: "替换当前 Core 引用面" }));
    await waitFor(() => expect(macroBody).not.toBeNull());
    expect(macroBody?.intent).toEqual({ kind: "citation", cluster: { form: "parenthetical", items: [{ referenceNodeId: "ref-uuid", label: "page", locator: "9", prefix: null, suffix: null }, { referenceNodeId: "ref-jones", label: null, locator: null, prefix: null, suffix: null }] } });
    expect(sourceEditorValue(editor)).toContain("cite:[@smith,label=page,locator=9]+[jones]");
  });

  it("commits a task plan from the shared inspector and reloads the authoritative document", async () => {
    const source = "= Tasks\n\n* [ ] Ship the release\n";
    const committedSource = source.replace("[ ]", "[x]");
    const model = coreModel();
    const workspace = {
      rootNodeId: "root", revision: "workspace-revision", presentation: { adjacentHeadingBody: "separate" as const },
      documentFormat: { generation: "ascii_doc_v1" as const, canonicalExtension: "adoc" as const, mediaType: "text/asciidoc" as const },
      nodes: [{ id: "root", name: "Workspace", parentId: null, path: "", trashed: false }, { id: "task-doc", name: "Tasks", parentId: "root", path: "Tasks", trashed: false }],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
    };
    const nextWorkspace = { ...workspace, revision: "next-workspace-revision" };
    const document = { nodeId: "task-doc", name: "Tasks", revision: "a".repeat(64), length: source.length, source, model };
    const nextDocument = { ...document, revision: "b".repeat(64), length: committedSource.length, source: committedSource };
    const taskRange = { start: source.indexOf("* [ ]"), end: source.length - 1 };
    const requests: Array<{ path: string; body?: unknown }> = [];
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace, document, draftRecovery: { drafts: [], issues: [] } }),
      chooseWorkspace: async () => null,
      request: async (path, body) => {
        requests.push({ path, body });
        if (path === "/api/document/model") return { model };
        if (path.startsWith("/api/document?")) return { document: nextDocument };
        if (path.startsWith("/api/annotations")) return { annotations: { version: 3, document_id: "task-doc", annotations: [] } };
        if (path === "/api/citation/analyze") return { authoring: { reference: { citationData: null, diagnostics: [] }, citations: { diagnostics: [] } }, analysis: { diagnostics: [] }, presentation: null, presentationFailure: null };
        if (path.startsWith("/api/task/inspect?")) return {
          nodeId: "task-doc",
          occurrences: [{ nodeId: "task-doc", revision: document.revision, task: { state: "open", description: "Ship the release", listDepth: 0, range: taskRange, metadata: null, valid: true } }],
          diagnostics: [],
        };
        if (path === "/api/task/edit-preview") return {
          plan: {
            planId: "task-plan", kind: "edit", baseWorkspaceRevision: workspace.revision, nodeId: "task-doc",
            authoring: { proposedSource: committedSource, assignedId: null, target: { state: "closed", description: "Ship the release", listDepth: 0, range: taskRange, metadata: null, valid: true } },
            documentChanges: [{}],
          },
        };
        if (path === "/api/task/transaction/commit") return { workspace: nextWorkspace, searchIndexWarning: null };
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(screen.getByText("Core revision 已同步")).toBeTruthy());
    fireEvent.click(within(screen.getByRole("navigation", { name: "Inspector 面板" })).getByRole("button", { name: "Tasks" }));
    fireEvent.click(await screen.findByRole("button", { name: "完成任务：Ship the release" }));
    expect(await screen.findByRole("textbox", { name: "精确拟议源" })).toHaveProperty("value", committedSource);
    fireEvent.click(screen.getByRole("button", { name: "确认并提交" }));

    await waitFor(() => expect(requests.some(({ path }) => path.startsWith("/api/document?nodeId=task-doc&remember=false"))).toBe(true));
    expect(requests.find(({ path }) => path === "/api/task/edit-preview")?.body).toEqual({
      nodeId: "task-doc",
      baseWorkspaceRevision: "workspace-revision",
      baseRevision: document.revision,
      target: { kind: "occurrence", range: taskRange },
      intent: { kind: "toggle" },
    });
    expect(requests.find(({ path }) => path === "/api/task/transaction/commit")?.body).toEqual({ planId: "task-plan" });
    expect(sourceEditorValue(openSource())).toBe(committedSource);
  });

  it("opens the shared query surface, renders the authorized Core result, and follows row identity", async () => {
    const rootSource = "= Workspace\n\nRoot document.\n";
    const childSource = "= Research\n\nSelected from the Core query.\n";
    const model = coreModel();
    const workspace = {
      rootNodeId: "root", revision: "workspace-revision", presentation: { adjacentHeadingBody: "separate" as const },
      documentFormat: { generation: "ascii_doc_v1" as const, canonicalExtension: "adoc" as const, mediaType: "text/asciidoc" as const },
      nodes: [{ id: "root", name: "Workspace", parentId: null, path: "", trashed: false }, { id: "research", name: "Research", parentId: "root", path: "Research", trashed: false }],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
    };
    const documents = {
      root: { nodeId: "root", name: "Workspace", revision: "a".repeat(64), length: rootSource.length, source: rootSource, model },
      research: { nodeId: "research", name: "Research", revision: "b".repeat(64), length: childSource.length, source: childSource, model },
    };
    const requests: Array<{ path: string; body?: unknown }> = [];
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace, document: documents.root, draftRecovery: { drafts: [], issues: [] } }),
      chooseWorkspace: async () => null,
      request: async (path, body) => {
        requests.push({ path, body });
        if (path === "/api/document/model") return { model };
        if (path.startsWith("/api/document?")) return { document: documents.research };
        if (path.startsWith("/api/annotations")) return { annotations: { version: 3, document_id: path.includes("research") ? "research" : "root", annotations: [] } };
        if (path === "/api/citation/analyze") return { authoring: { reference: { citationData: null, diagnostics: [] }, citations: { diagnostics: [] } }, analysis: { diagnostics: [] }, presentation: null, presentationFailure: null };
        if (path === "/api/query/execute") return {
          execution: {
            blockIndex: 0,
            analysis: { blocks: [{ source: "nodes", view: "table", body: "", range: { start: 0, end: 80 }, valid: true }], diagnostics: [] },
            result: {
              source: "nodes", columns: [{ outputName: "name", path: "name", field: "name", propertyKey: null, valueType: "string", nullable: false }, { outputName: "path", path: "path", field: "path", propertyKey: null, valueType: "string", nullable: false }], groups: [], totalBeforeLimit: 1, truncated: false,
              rows: [{ identity: { kind: "node", nodeId: "research", revision: documents.research.revision }, cells: [{ column: { outputName: "name", path: "name", field: "name", propertyKey: null, valueType: "string", nullable: false }, value: { kind: "text", value: "Research" } }, { column: { outputName: "path", path: "path", field: "path", propertyKey: null, valueType: "string", nullable: false }, value: { kind: "text", value: "/Research" } }] }],
            },
            csv: "name,path\r\nResearch,/Research\r\n",
          },
        };
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(screen.getByText("Core revision 已同步")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "查询与视图" }));
    const dialog = screen.getByRole("dialog", { name: "查询与派生表格" });
    fireEvent.click(within(dialog).getByRole("button", { name: "运行 Core 查询" }));
    expect(await within(dialog).findByText("/Research")).toBeTruthy();
    const queryRequest = requests.find(({ path }) => path === "/api/query/execute");
    expect(queryRequest?.body).toMatchObject({
      source: "[.weftext-query,version=1,view=table]\n....\nfrom nodes as node\nscope workspace\nwhere true\nselect node.name, node.path\norder by node.path asc\nlimit 100\n....\n",
      blockIndex: 0,
      context: { binding: { nodeId: "root", heading: null }, now: expect.any(String), timezone: expect.any(String), locale: expect.any(String) },
    });

    fireEvent.click(within(dialog).getByRole("button", { name: "打开节点" }));
    await waitFor(() => expect(requests.some(({ path }) => path === "/api/document?nodeId=research")).toBe(true));
    expect(screen.queryByRole("dialog", { name: "查询与派生表格" })).toBeNull();
    expect(sourceEditorValue(openSource())).toBe(childSource);
  });

  it("keeps historical reference records read-only and exposes only occurrence authoring", async () => {
    const source = "---\nweftext:\n  id: \"550e8400-e29b-41d4-a716-446655440000\"\n---\n= Smith\n";
    const model = coreModel();
    const workspace = {
      rootNodeId: "root", revision: "workspace-revision", presentation: { adjacentHeadingBody: "separate" as const },
      documentFormat: { generation: "ascii_doc_v1" as const, canonicalExtension: "adoc" as const, mediaType: "text/asciidoc" as const },
      nodes: [{ id: "root", name: "Workspace", parentId: null, path: "", trashed: false }, { id: "ref-uuid", name: "Smith", parentId: "root", path: "Smith", trashed: false }],
      links: { outgoing: [], backlinks: [], potentialMentions: [] },
    };
    const document = { nodeId: "ref-uuid", name: "Smith", revision: "a".repeat(64), length: source.length, source, model };
    const requests: string[] = [];
    window.weftextDesktop = {
      restoreWorkspace: async () => ({ opened: true, workspace, document, draftRecovery: { drafts: [], issues: [] } }),
      chooseWorkspace: async () => null,
      request: async (path) => {
        requests.push(path);
        if (path === "/api/document/model") return { model };
        if (path.startsWith("/api/annotations")) return { annotations: { version: 3, document_id: "ref-uuid", annotations: [] } };
        if (path === "/api/citation/analyze") return { authoring: { reference: { citationData: { key: "smith", itemType: "book", title: "Test Book", fields: {} }, diagnostics: [] }, citations: { diagnostics: [] } }, analysis: { diagnostics: [] }, presentation: null, presentationFailure: null };
        return {};
      },
    };

    render(<Home />);
    await waitFor(() => expect(screen.getByText("Core revision 已同步")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "Citations" }));
    expect(await screen.findByText("历史参考文献记录（只读）")).toBeTruthy();
    expect(screen.getByText(/创建、字段编辑和引用键重命名没有写入口/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: /创建参考文献|更新标题|更新类型|重命名|管理字段/ })).toBeNull();

    fireEvent.click(within(screen.getByRole("region", { name: "引用与参考文献" })).getByRole("button", { name: "插入引用" }));
    const dialog = screen.getByRole("dialog", { name: "插入引用" });
    expect(within(dialog).getByRole("button", { name: "正文引用" })).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: "仅加入书目" })).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: "参考文献表" })).toBeTruthy();
    expect(within(dialog).queryByLabelText("引用键")).toBeNull();
    expect(requests.every((path) => !path.includes("/reference/") && !path.includes("rename") && !path.includes("transaction"))).toBe(true);
  });
});
