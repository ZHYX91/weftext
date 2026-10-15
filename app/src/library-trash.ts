import type { KnowledgePlan, KnowledgeReceipt } from "./knowledge-panel";
import type { OpenedDocument } from "./types";

export interface LibraryTrashEntry { operationId: string; nodeId?: string; name: string; path?: string }
export interface LibraryTrashDraft { name: string; parent: string }
/** A single retired transaction may contain multiple independently restorable roots. */
export function trashRestoreDraft(drafts: Map<string, LibraryTrashDraft>, root: string, entry: LibraryTrashEntry): LibraryTrashDraft {
  const key = JSON.stringify([root, entry.operationId, entry.nodeId ?? null]);
  let draft = drafts.get(key);
  if (!draft) { draft = { name: entry.name, parent: root }; drafts.set(key, draft); }
  return draft;
}
export interface LibraryTrashHost {
  request<T>(path: string, body?: unknown): Promise<T>;
  created(document: OpenedDocument): Promise<void>;
  notify(message: string, error?: boolean): void;
}
export interface LibraryTrashController { open(root: string): Promise<void> }
export interface RestoreRequest {
  operationId: string;
  root: string;
  intent: { kind: "restoreNode"; trashOperationId: string; nodeId?: string; parent: string; name: string };
}
export interface PendingRestore {
  request: RestoreRequest;
  plan?: KnowledgePlan;
  confirmed: boolean;
  state: "preparing" | "prepared" | "committing" | "unknown" | "committed" | "failed";
  receipt?: KnowledgeReceipt;
  delivered: boolean;
  message: string;
}

const success = new Set(["committed", "saved", "complete", "success"]);
const failed = new Set(["conflict", "failed", "rejected", "failed-no-effect"]);
const terminalErrorCodes = new Set(["conflict", "operationConflict", "preimageChanged", "invalidName", "invalidPath", "trashMissing", "trashSelectionRequired", "trashSelectionMismatch", "targetExists", "destinationExists", "unsupported", "permissionDenied", "rejected"]);
function errorMessage(error: unknown): string { return error instanceof Error ? error.message : String(error); }
function errorCode(error: unknown): string { return error && typeof error === "object" && "code" in error ? String(error.code) : ""; }
function lossMessage(value: unknown): string {
  if (typeof value === "string") return value;
  if (value && typeof value === "object" && "message" in value && typeof value.message === "string") return value.message;
  return "本次恢复包含尚不能完整处理的内容，请检查准备结果。";
}
function openedDocument(value: unknown, nodeId?: string): value is OpenedDocument {
  if (!value || typeof value !== "object") return false;
  const document = value as Partial<OpenedDocument>;
  return typeof document.sessionId === "string" && typeof document.path === "string" && typeof document.root === "string" && typeof document.source === "string" && typeof document.baseId === "string" && (nodeId === undefined || document.nodeId === nodeId);
}

/** Retains the original request across UI lifetimes; status reads never execute it. */
export class LibraryTrashRestoreFlow {
  pending: PendingRestore | null = null;
  busy = false;
  constructor(private readonly host: LibraryTrashHost, private readonly newId: () => string = () => crypto.randomUUID()) {}
  get blocksNewRestore(): boolean { return Boolean(this.pending && !this.pending.delivered && this.pending.state !== "failed"); }

  async prepare(root: string, entry: LibraryTrashEntry, parent: string, name: string): Promise<void> {
    if (this.busy || this.blocksNewRestore) throw new Error("请先检查或取消原恢复操作，再准备新的恢复。");
    if (!parent.trim() || !name.trim()) throw new Error("请输入恢复位置与名称。");
    const request: RestoreRequest = Object.freeze({ operationId: this.newId(), root, intent: Object.freeze({ kind: "restoreNode" as const, trashOperationId: entry.operationId, ...(entry.nodeId !== undefined ? { nodeId: entry.nodeId } : {}), parent, name }) });
    this.pending = { request, confirmed: false, state: "preparing", delivered: false, message: "正在准备准确恢复变动…" };
    this.busy = true;
    try {
      const plan = await this.host.request<KnowledgePlan>("/api/operations/prepare", request);
      if (plan.operationId !== request.operationId || typeof plan.planId !== "string" || !plan.planId) throw new Error("恢复准备结果与原请求不一致，保留原操作供检查。");
      this.pending.plan = plan; this.pending.state = "prepared"; this.pending.message = "请核对准确恢复位置与全部文件变化。";
    } catch (error) {
      this.pending.state = terminalErrorCodes.has(errorCode(error)) ? "failed" : "unknown";
      this.pending.message = `${errorMessage(error)}${this.pending.state === "unknown" ? " 原请求已保留，检查原操作即可。" : ""}`;
      throw error;
    } finally { this.busy = false; }
  }

  cancelPrepared(): void {
    if (!this.busy && this.pending?.state === "prepared" && !this.pending.confirmed) this.pending = null;
  }

  async confirmAndCommit(): Promise<void> {
    const pending = this.pending;
    if (this.busy || !pending?.plan || pending.state !== "prepared") throw new Error("当前没有可确认的恢复准备结果。");
    pending.confirmed = true; pending.state = "committing"; pending.message = "正在执行已确认的恢复…";
    this.busy = true;
    try {
      const receipt = await this.host.request<KnowledgeReceipt>("/api/operations/commit", { operationId: pending.request.operationId, planId: pending.plan.planId });
      await this.receive(receipt);
    } catch (error) {
      this.recordCommitError(error);
      throw error;
    } finally { this.busy = false; }
  }

  async reconcile(): Promise<void> {
    const pending = this.pending; if (!pending || this.busy) return;
    this.busy = true; pending.message = "正在检查原恢复结果…";
    try {
      const result = await this.host.request<KnowledgeReceipt & Partial<KnowledgePlan>>("/api/operations/status", { operationId: pending.request.operationId });
      if (result.operationId && result.operationId !== pending.request.operationId) throw new Error("状态响应属于另一操作，原恢复保留。" );
      if (result.status === "prepared" && typeof result.planId === "string") {
        if (pending.plan && pending.plan.planId !== result.planId) throw new Error("原恢复准备标识已变化，请保留原记录检查。");
        pending.plan = result as KnowledgePlan; pending.state = "prepared";
        pending.message = pending.confirmed ? "原恢复仍处于准备状态；可明确执行同一份已确认结果。" : "原准备结果已找到；核对后明确确认。";
      } else await this.receive(result);
    } catch (error) {
      if (pending.state !== "committed") pending.state = "unknown";
      pending.message = `${errorMessage(error)} 原恢复记录已保留。`;
      throw error;
    } finally { this.busy = false; }
  }

  async deliverCommitted(): Promise<void> {
    const pending = this.pending;
    if (!pending || pending.state !== "committed" || pending.delivered) return;
    const document = pending.receipt?.document;
    if (!openedDocument(document, pending.request.intent.nodeId)) { pending.message = "恢复已完成，准确笔记的打开信息暂不可用。检查原操作以读取同一结果。"; return; }
    try { await this.host.created(document); pending.delivered = true; pending.message = "笔记已恢复并打开。"; this.host.notify(pending.message); }
    catch (error) { pending.message = `笔记已恢复；打开尚未完成：${errorMessage(error)}`; throw error; }
  }

  private async receive(receipt: KnowledgeReceipt): Promise<void> {
    const pending = this.pending; if (!pending) return;
    if (receipt.operationId && receipt.operationId !== pending.request.operationId) throw new Error("恢复结果与原请求不一致，不能交付另一操作。" );
    if (pending.plan && typeof receipt.planId === "string" && receipt.planId !== pending.plan.planId) throw new Error("恢复结果与已确认准备不一致。" );
    pending.receipt = receipt;
    if (success.has(receipt.status)) { pending.state = "committed"; pending.message = receipt.message || "恢复已完成。"; await this.deliverCommitted(); }
    else if (failed.has(receipt.status)) { pending.state = "failed"; pending.message = receipt.message || "恢复未应用，原笔记仍需检查。"; }
    else { pending.state = "unknown"; pending.message = receipt.message || "恢复结果尚未确认。原操作已保留，不会重复恢复。"; }
  }

  private recordCommitError(error: unknown): void {
    const pending = this.pending; if (!pending || pending.state === "committed") return;
    pending.state = terminalErrorCodes.has(errorCode(error)) ? "failed" : "unknown";
    pending.message = `${errorMessage(error)}${pending.state === "unknown" ? " 恢复结果未知，请检查原操作。" : ""}`;
  }
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text = ""): HTMLElementTagNameMap[K] { const result = document.createElement(tag); result.className = className; result.textContent = text; return result; }
function field(title: string, value: string): { label: HTMLLabelElement; input: HTMLInputElement } { const label = el("label", "knowledge-field"); label.append(el("span", "", title)); const input = el("input", "knowledge-input"); input.value = value; label.append(input); return { label, input }; }

/** Library entry point remains usable when there is no active note. */
export function createLibraryTrashController(host: LibraryTrashHost): LibraryTrashController {
  const flow = new LibraryTrashRestoreFlow(host);
  const drafts = new Map<string, LibraryTrashDraft>();
  let active: { dialog: HTMLDialogElement; root: string; promise: Promise<void>; close(): void } | null = null;
  return {
    async open(root: string): Promise<void> {
      if (active) { active.dialog.focus(); return active.promise; }
      const dialog = el("dialog", "knowledge-dialog library-trash-dialog"); dialog.setAttribute("aria-label", "资料库回收站");
      dialog.append(el("h2", "", "资料库回收站"), el("p", "knowledge-help", "选择已回收笔记，核对恢复名称、目标位置及完整文件变动。"), el("p", "knowledge-address", root));
      const status = el("p", "knowledge-help"); status.setAttribute("aria-live", "polite");
      const pendingHost = el("section", "knowledge-section"), list = el("section", "knowledge-section"), actions = el("div", "knowledge-actions");
      dialog.append(status, pendingHost, list, actions);
      let entries: LibraryTrashEntry[] = [], closed = false, loaded = false;
      let resolve!: () => void;
      const promise = new Promise<void>(r => { resolve = r; });
      const close = (): void => { if (closed) return; closed = true; dialog.close(); dialog.remove(); if (active?.dialog === dialog) active = null; resolve(); };
      active = { dialog, root, promise, close };
      const button = (label: string, run: () => void | Promise<void>, primary = false): HTMLButtonElement => {
        const value = el("button", `knowledge-button${primary ? " primary" : ""}`, label); value.type = "button";
        value.onclick = () => { void Promise.resolve(run()).catch(error => { status.textContent = errorMessage(error); host.notify(errorMessage(error), true); }).finally(() => { if (!closed) render(); }); }; return value;
      };
      const lock = (): void => { dialog.querySelectorAll<HTMLInputElement | HTMLButtonElement>("input,button").forEach(control => { if (control.dataset.close !== "true") control.disabled = true; }); };
      const load = async (): Promise<void> => {
        status.textContent = "正在读取当前资料库回收记录…";
        const result = await host.request<{ trash: LibraryTrashEntry[] }>("/api/knowledge/trash", { root });
        if (closed) return;
        entries = result.trash; loaded = true; status.textContent = flow.pending?.delivered ? flow.pending.message : ""; render();
      };
      const renderPreview = (pending: PendingRestore): void => {
        pendingHost.append(el("h3", "", pending.confirmed ? "已确认的原恢复" : "恢复预览"), el("p", "knowledge-help", `恢复名称：${pending.request.intent.name}`), el("p", "knowledge-address", `目标父目录：${pending.request.intent.parent}`));
        if (pending.request.root !== root) pendingHost.append(el("p", "knowledge-warning", `这份原操作属于 ${pending.request.root}；检查完成前不能准备新的恢复。`));
        const changes = pending.plan?.changes ?? [];
        if (changes.length) {
          pendingHost.append(el("p", "knowledge-help", `本次恢复涉及 ${changes.length} 个文件变化。`));
          const paths = el("ul", "knowledge-change-list");
          for (const change of changes) { const item = el("li"); item.append(el("strong", "", ({ create: "恢复到", copy: "恢复副本到", move: "移至", rename: "重新命名", relocateFrom: "移出回收位置", relocateTo: "恢复到", remove: "移出回收位置", delete: "移出回收位置", write: "恢复内容到" } as Record<string, string>)[change.action ?? ""] ?? "恢复变动"), el("span", "knowledge-address", change.path ?? "")); if (change.bytes !== undefined) item.append(el("small", "knowledge-help", `${change.bytes.toLocaleString()} 字节`)); paths.append(item); } pendingHost.append(paths);
        }
        for (const loss of pending.plan?.losses ?? []) pendingHost.append(el("p", "knowledge-warning", lossMessage(loss)));
        pendingHost.append(el("p", pending.state === "unknown" || pending.state === "failed" ? "knowledge-warning" : "knowledge-help", pending.message));
      };
      const render = (): void => {
        if (closed) return;
        pendingHost.replaceChildren(); list.replaceChildren();
        const pending = flow.pending;
        if (pending && !pending.delivered) {
          renderPreview(pending); const controls = el("div", "knowledge-actions"); pendingHost.append(controls);
          if (pending.state === "prepared") {
            if (!pending.confirmed) controls.append(button("取消本次准备", () => { flow.cancelPrepared(); render(); }));
            controls.append(button(pending.confirmed ? "执行已确认的原恢复" : "确认恢复", async () => { lock(); await flow.confirmAndCommit(); if (!closed && flow.pending?.state === "committed") await load(); }, true));
          } else if (pending.state === "committed") {
            controls.append(button(openedDocument(pending.receipt?.document, pending.request.intent.nodeId) ? "打开已恢复笔记" : "检查原操作", async () => { lock(); if (openedDocument(flow.pending?.receipt?.document, flow.pending?.request.intent.nodeId)) await flow.deliverCommitted(); else await flow.reconcile(); }));
          } else if (pending.state === "unknown") controls.append(button("检查原操作", async () => { lock(); await flow.reconcile(); if (!closed && flow.pending?.state === "committed") await load(); }));
        }
        if (!entries.length) list.append(el("p", "knowledge-empty", loaded ? "当前资料库暂无可读回收记录。" : "回收记录尚未读取完成。"));
        for (const entry of entries) {
          const draft = trashRestoreDraft(drafts, root, entry);
          const card = el("article", "knowledge-card"); card.append(el("strong", "", entry.name)); if (entry.path) card.append(el("p", "knowledge-address", entry.path));
          const name = field("恢复名称", draft.name), parent = field("目标父目录（完整路径）", draft.parent); name.input.oninput = () => { draft!.name = name.input.value; }; parent.input.oninput = () => { draft!.parent = parent.input.value; };
          const prepare = button("准备恢复", async () => { lock(); await flow.prepare(root, entry, parent.input.value, name.input.value); }); prepare.disabled = flow.busy || flow.blocksNewRestore; name.input.disabled = parent.input.disabled = flow.busy;
          card.append(name.label, parent.label, prepare); list.append(card);
        }
        if (flow.busy) lock();
      };
      const refresh = button("刷新回收记录", load), done = button("关闭", close); done.dataset.close = "true"; actions.append(refresh, done);
      dialog.addEventListener("cancel", event => { event.preventDefault(); close(); }); document.body.append(dialog); dialog.showModal(); status.textContent = "正在读取当前资料库回收记录…";
      try { if (flow.pending && !flow.pending.delivered && flow.pending.state !== "failed") await flow.reconcile(); await load(); }
      catch (error) { if (!closed) { status.textContent = errorMessage(error); host.notify(errorMessage(error), true); render(); } }
      return promise;
    },
  };
}
