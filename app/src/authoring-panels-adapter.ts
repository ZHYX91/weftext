import { ChangeSet } from "@codemirror/state";
import { validateWritingResult, writingRange, type WritingController, type WritingRange, type WritingRequest, type WritingResult, type WritingSnapshot } from "./writing";
import type { RenderNode } from "./types";
import type { DocumentNotesHost } from "./document-notes";
import type { HeadingNumberingIntent, HeadingNumberingPreview, NumberingHost } from "./numbering-panel";

export interface AuthoringProjection { ok: boolean; source: string; serial: number; tree: RenderNode | null }
export interface AuthoringPanelsEnvironment {
  controller: WritingController;
  projection(): AuthoringProjection | null;
  readable(): boolean;
  editable(): boolean;
  selection(): { from: number; to: number };
  requestEdit(request: WritingRequest): Promise<WritingResult>;
  previewProposal(request: { sessionId: string; source: string; serial: number; proposalSource: string }): Promise<AuthoringProjection>;
  revealSource(range: WritingRange): void;
  revealRendered?(key: string): boolean;
  ensureEndnoteModule?(): Promise<void>;
  recoverEndnoteModule?(): Promise<void>;
  retryEndnoteModule?(): Promise<void>;
  restartEndnoteModule?(): Promise<void>;
  settleEndnoteModule?(): Promise<void>;
  pendingInput?(value: string): void;
  onError?(error: Error): void;
}
export function documentNotesHost(environment: AuthoringPanelsEnvironment): DocumentNotesHost {
  const { controller } = environment;
  return {
    controller, readable: environment.readable, editable: environment.editable,
    projection: () => {
      const projection = environment.projection(), map = controller.map, basis = controller.snapshot;
      return environment.readable() && projection?.ok && projection.tree && projection.source === basis.source && projection.serial === basis.serial && map && map.notes && map.notes.sourceHash === map.sourceHash
        ? { source: basis.source, sourceHash: map.sourceHash, notes: map.notes, tree: projection.tree, structures: map.structures } : null;
    },
    insertionRange: () => {
      const basis = controller.snapshot, selection = environment.selection();
      if (selection.from === selection.to) return writingRange(basis.source, selection.from, selection.to);
      throw new Error("当前选中了正文范围；请先在需要插入注释的位置放置光标，未将标记移到文末。");
    },
    perform: (intent, basis) => controller.execute(() => {
      if (!environment.editable() || !controller.isSnapshotCurrent(basis)) throw new Error("注释提案的正文、来源或编辑权限已变化，输入保留。"); return intent;
    }),
    revealSource: environment.revealSource, revealRendered: environment.revealRendered,
    ensureEndnoteModule: environment.ensureEndnoteModule, recoverEndnoteModule: environment.recoverEndnoteModule,
    retryEndnoteModule: environment.retryEndnoteModule, settleEndnoteModule: environment.settleEndnoteModule,
    restartEndnoteModule: environment.restartEndnoteModule,
    onPendingInput: environment.pendingInput, onError: environment.onError,
  };
}
function role(node: RenderNode): string | null { return typeof node.attrs["data-weftext-role"] === "string" ? node.attrs["data-weftext-role"] as string : null; }
function all(node: RenderNode): RenderNode[] { return [node, ...node.children.flatMap(all)]; }
function visibleText(node: RenderNode, skipPrefix = false): string { return skipPrefix && role(node) === "heading-prefix" ? "" : node.kind === "text" ? node.text ?? "" : node.children.map(child => visibleText(child, skipPrefix)).join(""); }
function heading(tree: RenderNode, range: WritingRange): RenderNode | null {
  const matches = all(tree).filter(node => role(node) === "heading" && node.origin?.isMain && node.origin.utf16From >= range.utf16From && node.origin.utf16To <= range.utf16To);
  return matches.length === 1 ? matches[0] : null;
}
function prefix(node: RenderNode | null): string | null { if (!node) return null; const prefixes = all(node).filter(child => role(child) === "heading-prefix"); return prefixes.length === 1 ? visibleText(prefixes[0]).trim() || null : null; }

/** Compare native heading products only after mapping exact source ranges. */
export function numberingPreviewHeadings(before: RenderNode, after: RenderNode, source: string, result: WritingResult, beforeRanges: WritingRange[]): NonNullable<HeadingNumberingPreview["headings"]> {
  const patches = validateWritingResult(source, result), changes = ChangeSet.of(patches.map(patch => ({ from: patch.from, to: patch.to, insert: patch.insert })), source.length, "\n");
  return beforeRanges.flatMap(range => {
    if (patches.some(patch => patch.from < range.utf16To && patch.to > range.utf16From)) return [];
    const from = changes.mapPos(range.utf16From, 1), to = changes.mapPos(range.utf16To, -1), nextRange = writingRange(result.source, from, to), old = heading(before, range), next = heading(after, nextRange);
    if (!old || !next) return [];
    return [{ text: visibleText(next, true).trim(), before: prefix(old), after: prefix(next) }];
  });
}
export function numberingHost(environment: AuthoringPanelsEnvironment, personal: { defaultKey: string; noteKey: string; display(enabled: boolean): void }): NumberingHost {
  const { controller } = environment, basis: WritingSnapshot = controller.snapshot, map = controller.map, before = environment.projection();
  let approved: { action: string; source: string } | null = null;
  const current = () => environment.readable() && controller.isSnapshotCurrent(basis) && environment.projection()?.source === basis.source && environment.projection()?.serial === basis.serial && controller.map?.sourceHash === map?.sourceHash;
  const requireCurrent = () => { if (!current() || !environment.editable() || controller.state.composing) throw new Error("编号提案的正文、来源或写权限已变化；请先完成输入并重新预览。"); };
  return {
    source: () => controller.snapshot.source,
    inspection: () => ({ ...(controller.map?.headingNumbering ?? { sourceHash: "", settings: null, editable: false, reason: "当前原生编号地图未就绪" }), editable: !!controller.map?.headingNumbering?.editable && current() && environment.editable(), reason: !current() ? "正文或授权已变化，请重新打开当前笔记编号控制" : !environment.editable() ? "当前没有修改本篇正文的权限" : controller.map?.headingNumbering?.reason ?? null }),
    preview: async action => {
      approved = null; requireCurrent(); await controller.flush(); requireCurrent();
      if (!map || !before?.ok || !before.tree) throw new Error("当前草稿还没有可比较的真实标题投影，请等待预览。");
      const result = await environment.requestEdit({ sessionId: basis.sessionId, source: basis.source, serial: basis.serial, sourceHash: map.sourceHash, intent: { kind: "headingNumbering", action } }); requireCurrent(); validateWritingResult(basis.source, result);
      const rendered = await environment.previewProposal({ sessionId: basis.sessionId, source: basis.source, serial: basis.serial, proposalSource: result.source }); requireCurrent();
      if (!rendered.ok || !rendered.tree || rendered.source !== result.source || rendered.serial !== basis.serial) throw new Error("编号提案未取得准确原生编译预览；原稿与编号输入保留。");
      approved = { action: JSON.stringify(action), source: result.source };
      return { beforeSource: basis.source, afterSource: result.source, headings: numberingPreviewHeadings(before.tree, rendered.tree, basis.source, result, map.structures.filter(item => item.kind === "heading").map(item => item.range)) };
    },
    apply: async (action: HeadingNumberingIntent) => {
      requireCurrent(); if (!approved || approved.action !== JSON.stringify(action)) throw new Error("请先预览这次准确编号设置，未应用未预览的提案。");
      const afterSource = approved.source;
      await controller.execute(() => { requireCurrent(); return { kind: "headingNumbering", action }; }, afterSource);
      approved = null; return true;
    },
    personalDefaultKey: personal.defaultKey, personalNoteKey: personal.noteKey, displayPersonal: personal.display,
    readable: environment.readable, composing: () => controller.state.composing,
    binding: () => { const now = controller.snapshot; return JSON.stringify([now.sessionId, now.serial, now.generation]); },
    observeState: (element: HTMLElement, update: () => void) => controller.observeSurface(element, update),
  };
}

export function personalNumberingKeys(user: string, library: string, note: string): { defaultKey: string; noteKey: string } {
  if (![user, library, note].every(value => typeof value === "string" && value.length > 0)) throw new Error("个人辅助编号尚无稳定用户、资料库与笔记身份。");
  const base = `weftext-personal-heading-number:${encodeURIComponent(user)}`;
  return { defaultKey: `${base}:default`, noteKey: `${base}:${encodeURIComponent(library)}:${encodeURIComponent(note)}` };
}
export function personalNumberingVisible(keys: { defaultKey: string; noteKey: string }, storage: Pick<Storage, "getItem"> = localStorage): boolean {
  const own = storage.getItem(keys.noteKey); return own === "show" || own !== "hide" && storage.getItem(keys.defaultKey) === "show";
}
/** App-created badges are explicitly personal; author numbers remain intact. */
export function markPersonalHeading(element: Element, ordinal: number): void { element.setAttribute("data-personal-heading-order", String(ordinal)); }
export function displayPersonalHeadings(container: Element, enabled: boolean): void {
  for (const heading of Array.from(container.querySelectorAll<Element>("[data-personal-heading-order]"))) {
    heading.querySelector(":scope > .personal-heading-number")?.remove(); if (!enabled) continue;
    const badge = document.createElement("span"); badge.className = "personal-heading-number"; badge.textContent = `辅助 ${heading.getAttribute("data-personal-heading-order")} `; badge.setAttribute("aria-label", `个人辅助序号 ${heading.getAttribute("data-personal-heading-order")}，不属于文档编号`); heading.prepend(badge);
  }
}
