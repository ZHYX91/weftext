import { createElement, type ReactNode } from "react";

import type { DocumentBlock, WorkspaceDocumentFormat } from "./document-contract";

export type DemoTreeNode = {
  id: string;
  name: string;
  depth: number;
  open?: boolean;
  kind?: "chrono" | "note" | "folder";
  parentId?: string | null;
};

export type DemoAnnotation = {
  id: string;
  author: string;
  avatar: string;
  time: string;
  body: string;
  resolved: boolean;
};

export type DemoDocument = {
  parent: string;
  title: string;
  label: string;
  lead: string;
  body: string[];
};

export type DemoHeading = {
  level: number;
  text: string;
  start: number;
  line: number;
};

export type DemoWorkspace = {
  id: string;
  workspaceName: string;
  badge: string;
  sourceLabel: string;
  initialNodeId: string;
  nodes: DemoTreeNode[];
  annotations: Record<string, DemoAnnotation[]>;
  documentFormat: WorkspaceDocumentFormat;
  messages: {
    inlineFormatted: string;
    blockFormatted: string;
    trashEmpty: string;
    structureUnavailable: string;
    sessionSaved: string;
    chronoUnavailable: string;
  };
  documentFor(id: string, fallback: string): DemoDocument;
  sourceFor(id: string, fallback: string): string;
  chronoTarget(period: "year" | "quarter" | "month" | "week" | "day"): string | null;
  bodyStart(source: string): number;
  headings(source: string): DemoHeading[];
  render(source: string, runIn: boolean): ReactNode[];
  inlineFormat(action: "bold" | "emphasis" | "inline_code" | "link"): [string, string, string];
  lineEnding(source: string, offset: number): string;
  headingReplacement(source: string, level: number): string;
  blockReplacement(source: string, action: "paragraph" | "list" | "quote_increase" | "quote_decrease" | "code", lineEnding: string): string;
  extendTable(source: string, block: DocumentBlock, cursor: number, operation: "row" | "column"): { source: string; cursor: number } | null;
};

const nodes: DemoTreeNode[] = [
  { id: "overview", name: "项目总览", depth: 0, open: true, kind: "folder" },
  { id: "design", name: "设计", depth: 1, parentId: "overview", open: true, kind: "folder" },
  { id: "direction", name: "产品方向", depth: 2, parentId: "design", kind: "note" },
  { id: "principles", name: "交互原则", depth: 2, parentId: "design", kind: "note" },
  { id: "log", name: "开发日志", depth: 1, parentId: "overview", open: true, kind: "folder" },
  { id: "year", name: "2026", depth: 2, parentId: "log", open: true, kind: "chrono" },
  { id: "month", name: "2026-08", depth: 3, parentId: "year", kind: "chrono" },
  { id: "today", name: "2026-08-21", depth: 3, parentId: "year", kind: "chrono" },
  { id: "resources", name: "资料", depth: 1, parentId: "overview", kind: "folder" },
];

const annotations: Record<string, DemoAnnotation[]> = {
  principles: [
    { id: "preview-1", author: "林然", avatar: "林", time: "10:36", body: "“操作退居其后”这个方向很好。移动节点时，可以让预览更像一张清单吗？", resolved: false },
    { id: "preview-2", author: "周宁", avatar: "周", time: "昨天", body: "三种视图之间需要保留光标位置。", resolved: true },
  ],
};

const documents: Record<string, DemoDocument> = {
  principles: {
    parent: "设计",
    title: "交互原则",
    label: "设计规范",
    lead: "Weftext 的界面应让内容保持在前景，操作退居其后。",
    body: ["路径不是身份。移动节点不应打断用户的思路。", "所有结构操作都先预览，再提交。", "写作、源码和阅读视图共享同一个位置。"],
  },
  direction: {
    parent: "设计",
    title: "产品方向",
    label: "产品规划",
    lead: "一个 Rust Core，连接本地桌面、内网 WebUI 与未来协作服务。",
    body: ["本地使用不要求账号。", "浏览器是内网协作的第一入口。", "同步文件夹不能替代多人协作服务器。"],
  },
  today: {
    parent: "开发日志 / 2026",
    title: "2026-08-21",
    label: "Chrono",
    lead: "今天先把交互方向变成一个可以点击、可以讨论的界面。",
    body: ["收口 Stage 1 持久语义。", "验证节点树与三种编辑视图。", "记录所有会影响数据安全的交互决定。"],
  },
  overview: {
    parent: "产品工作区",
    title: "项目总览",
    label: "工作区",
    lead: "文缕是围绕严格目录节点构建的知识工作区。",
    body: ["Stage 0：存储基础已经完成。", "Stage 1：安全的本地日常工作区。", "Stage 2：内网 Server 与浏览器 WebUI。"],
  },
};

function documentFor(id: string, fallback: string): DemoDocument {
  return documents[id] ?? {
    parent: "产品工作区",
    title: fallback,
    label: "节点",
    lead: "这个节点还没有内容。你可以从这里开始写作。",
    body: ["所有内容都保存在同名目录节点中。", "路径可以变化，节点身份保持不变。", "每次结构操作都会先展示预览。"],
  };
}

function sourceFor(id: string, fallback: string) {
  const document = documentFor(id, fallback);
  const ids: Record<string, string> = {
    principles: "11111111-1111-4111-8111-111111111111",
    direction: "22222222-2222-4222-8222-222222222222",
    today: "33333333-3333-4333-8333-333333333333",
    overview: "44444444-4444-4444-8444-444444444444",
  };
  const nodeId = ids[id] ?? "55555555-5555-4555-8555-555555555555";
  return `---\nweftext:\n  id: "${nodeId}"\n---\n= ${document.title}\n:keywords: weftext, product\n\n${document.lead}\n\n== 核心原则\n\n${document.body.map((line) => `. ${line}`).join("\n")}\n\n> 好的工具不会要求用户理解它的内部结构。`;
}

function bodyStart(source: string) {
  const normalized = source.startsWith("\uFEFF") ? source.slice(1) : source;
  if (!normalized.startsWith("---\n") && !normalized.startsWith("---\r\n")) return 0;
  const match = normalized.match(/^---\r?\n[\s\S]*?\r?\n---\r?\n/);
  if (!match) return 0;
  return (source.startsWith("\uFEFF") ? 1 : 0) + match[0].length;
}

function stripBlockId(value: string) {
  return value.replace(/\s+\^[A-Za-z0-9-]{1,128}\s*$/, "");
}

function renderHeading(level: number, text: ReactNode, key: number) {
  const normalized = Math.max(1, Math.min(level, 9));
  if (normalized <= 6) return createElement(`h${normalized}`, { key }, text);
  return <div className={`extended-heading heading-level-${normalized}`} role="heading" aria-level={normalized} key={key}>{text}</div>;
}

function renderQuote(depth: number, text: ReactNode, key: number) {
  const actualDepth = Math.max(1, Math.trunc(depth));
  const renderedDepth = Math.min(actualDepth, 32);
  let content: ReactNode = <span>{text}</span>;
  for (let level = renderedDepth; level >= 1; level -= 1) {
    content = <blockquote className={`quote-level-${Math.min(level, 9)}`} data-quote-level={level}>{content}</blockquote>;
  }
  return <div className="nested-quote" data-quote-depth={actualDepth} aria-label={`引用层级 ${actualDepth}`} key={key}>{content}</div>;
}

function render(source: string, runIn: boolean) {
  const lines = source.slice(bodyStart(source)).split(/\r?\n/);
  const rendered: ReactNode[] = [];
  for (let index = 0; index < lines.length; index += 1) {
    const heading = lines[index].match(/^(={1,10})(?:[ \t]+|$)(.*?)\s*$/);
    const quote = lines[index].match(/^((?:>[ \t]?)+)(.*)$/);
    const next = lines[index + 1];
    if (heading && runIn && heading[1].length > 1 && next && !/^(?:=|>|\.|\*|-)\s/.test(next)) {
      rendered.push(<div className="run-in-paragraph" key={index}>{renderHeading(heading[1].length - 1, stripBlockId(heading[2]), index)}<span>{stripBlockId(next)}</span></div>);
      index += 1;
    } else if (heading) {
      rendered.push(renderHeading(Math.max(1, heading[1].length - 1), stripBlockId(heading[2]), index));
    } else if (quote) {
      const depth = (quote[1].match(/>/g) ?? []).length;
      rendered.push(renderQuote(depth, quote[2], index));
    } else if (lines[index].trim()) {
      rendered.push(<p key={index}>{stripBlockId(lines[index])}</p>);
    }
  }
  return rendered;
}

function sourcePosition(source: string, offset: number) {
  const lines = source.slice(0, offset).split("\n");
  return { line: lines.length };
}

function headings(source: string): DemoHeading[] {
  const start = bodyStart(source);
  const result: DemoHeading[] = [];
  let bodyOffset = 0;
  let fence: "----" | "...." | null = null;
  for (const line of source.slice(start).split("\n")) {
    const fenceMatch = line.match(/^\s{0,3}(----|\.\.\.\.)/);
    if (fenceMatch) {
      const marker = fenceMatch[1] as "----" | "....";
      fence = fence === marker ? null : fence ?? marker;
    } else if (!fence) {
      const heading = line.match(/^\s{0,3}(={2,10})[\t ]+(.+?)[\t ]*$/);
      if (heading) {
        const headingStart = start + bodyOffset;
        result.push({ level: heading[1].length - 1, text: stripBlockId(heading[2]).trim(), start: headingStart, line: sourcePosition(source, headingStart).line });
      }
    }
    bodyOffset += line.length + 1;
  }
  return result;
}

function exactLines(source: string) {
  const lines: Array<{ text: string; ending: string; start: number }> = [];
  const matcher = /\r\n|\r|\n/g;
  let start = 0;
  for (let match = matcher.exec(source); match; match = matcher.exec(source)) {
    lines.push({ text: source.slice(start, match.index), ending: match[0], start });
    start = match.index + match[0].length;
  }
  if (start < source.length) lines.push({ text: source.slice(start), ending: "", start });
  return lines;
}

function preferredLineEnding(source: string, offset: number) {
  const before = source.slice(0, offset).match(/(?:\r\n|\r|\n)(?![\s\S]*(?:\r\n|\r|\n))/)?.[0];
  return before ?? source.match(/\r\n|\r|\n/)?.[0] ?? "\n";
}

function stripSemanticBlockPrefix(source: string) {
  return source.replace(/^([ ]{0,3})(?:={2,10}(?:[ \t]+|$)|(?:>[ \t]?)+|[.*-] )/, "$1");
}

export function headingBlockReplacement(source: string, level: number) {
  const normalized = Math.max(1, Math.min(Math.trunc(level), 9));
  const indent = source.match(/^ {0,3}/)?.[0] ?? "";
  const body = stripSemanticBlockPrefix(source).slice(indent.length);
  return `${indent}${"=".repeat(normalized + 1)} ${body}`;
}

export function formattedBlockReplacement(source: string, action: "paragraph" | "list" | "quote_increase" | "quote_decrease" | "code", lineEnding: string) {
  const mapLines = (transform: (line: string) => string) => exactLines(source).map((line) => `${transform(line.text)}${line.ending}`).join("");
  if (action === "paragraph") return mapLines(stripSemanticBlockPrefix);
  if (action === "list") {
    return mapLines((line) => {
      const indent = line.match(/^ {0,3}/)?.[0] ?? "";
      return `${indent}* ${stripSemanticBlockPrefix(line).slice(indent.length)}`;
    });
  }
  if (action === "quote_increase") return mapLines((line) => line.replace(/^([ ]{0,3})/, "$1> "));
  if (action === "quote_decrease") return mapLines((line) => line.replace(/^([ ]{0,3})>[ \t]?/, "$1"));
  const lines = exactLines(source);
  const opening = lines[0]?.text.match(/^(----|\.\.\.\.)/)?.[1];
  if (opening && lines.length >= 2 && new RegExp(`^${opening}[ \\t]*$`).test(lines.at(-1)?.text ?? "")) {
    return lines.slice(1, -1).map((line) => `${line.text}${line.ending}`).join("");
  }
  return `----${lineEnding}${source}${source.endsWith(lineEnding) ? "" : lineEnding}----${lineEnding}`;
}

export function extendTableAtCoreBlock(source: string, block: DocumentBlock, cursor: number, operation: "row" | "column") {
  if (block.kind !== "table" || block.start < 0 || block.end > source.length || block.start >= block.end || cursor < block.start || cursor > block.end) return null;
  const original = source.slice(block.start, block.end);
  const lines = exactLines(original);
  let opening = -1;
  let closing = -1;
  lines.forEach((line, index) => {
    if (line.text.trim() !== "|===") return;
    if (opening < 0) opening = index;
    closing = index;
  });
  if (opening < 0 || closing <= opening) return null;

  if (operation === "row") {
    const columns = lines.slice(opening + 1, closing).find((line) => line.text.includes("|"))?.text.match(/\|/g)?.length ?? 2;
    const row = Array.from({ length: Math.max(1, columns) }, () => "|  ").join("");
    const ending = preferredLineEnding(source, block.start);
    const insertionOffset = lines[closing].start;
    const updated = `${original.slice(0, insertionOffset)}${row}${ending}${original.slice(insertionOffset)}`;
    return { source: `${source.slice(0, block.start)}${updated}${source.slice(block.end)}`, cursor: block.start + insertionOffset };
  }

  const suffix = " |  ";
  const localCursor = cursor - block.start;
  let insertedBeforeCursor = 0;
  const updated = lines.map((line, index) => {
    const append = index > opening && index < closing && Boolean(line.text.trim());
    const insertionOffset = line.start + line.text.length;
    if (append && insertionOffset <= localCursor) insertedBeforeCursor += suffix.length;
    return `${line.text}${append ? suffix : ""}${line.ending}`;
  }).join("");
  return { source: `${source.slice(0, block.start)}${updated}${source.slice(block.end)}`, cursor: cursor + insertedBeforeCursor };
}

function inlineFormat(action: "bold" | "emphasis" | "inline_code" | "link"): [string, string, string] {
  if (action === "bold") return ["*", "*", "加粗文本"];
  if (action === "emphasis") return ["_", "_", "强调文本"];
  if (action === "inline_code") return ["+", "+", "代码"];
  return ["https://example.invalid[", "]", "链接文本"];
}

export const DEMO_WORKSPACE: DemoWorkspace = {
  id: "demo",
  workspaceName: "产品工作区",
  badge: "交互原型",
  sourceLabel: "隔离的原型草稿",
  initialNodeId: "principles",
  nodes,
  annotations,
  documentFormat: { generation: "ascii_doc_v1", canonicalExtension: "adoc", mediaType: "text/asciidoc" },
  messages: {
    inlineFormatted: "已在演示草稿光标位置应用 AsciiDoc；草稿尚未提交",
    blockFormatted: "已更新演示 AsciiDoc 草稿",
    trashEmpty: "演示工作区的 Trash 为空",
    structureUnavailable: "演示模式不执行结构写入。连接 Desktop 或本机 Core 后才能生成真实事务预览。",
    sessionSaved: "草稿已保存到本次原型会话",
    chronoUnavailable: "季度和周节点需要连接 Desktop 或本机 Core 后创建",
  },
  documentFor,
  sourceFor,
  chronoTarget: (period) => period === "year" ? "year" : period === "month" ? "month" : period === "day" ? "today" : null,
  bodyStart,
  headings,
  render,
  inlineFormat,
  lineEnding: preferredLineEnding,
  headingReplacement: headingBlockReplacement,
  blockReplacement: formattedBlockReplacement,
  extendTable: extendTableAtCoreBlock,
};
