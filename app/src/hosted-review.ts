import { AuthorInputScopes, type AuthorInputFactory } from "./author-input";
import { action, ui } from "./data-panel";
import { ProofreadingPanel, type ProofreadResult, type ProcessingDefaultsSuggestion } from "./proofreading-panel";
import type { KnowledgeAnnotation, KnowledgeContext, KnowledgeIntent } from "./knowledge-panel";
import { canApplySuggestion } from "./knowledge-state";
import { hostedCachePrefix } from "./hosted-state";
import { mediaIntentTarget, mediaAnnotationSelection, type HostedMediaSelection, type HostedMediaDescriptor } from "./hosted-media-state";
import type { MediaRegionInput } from "./knowledge-state";

type Rights = { read: boolean; write: boolean; target: boolean; accept: boolean; proofread: boolean; sourceRead: boolean; sourceWrite: boolean };
type Catalog = { resource: string; rights: Rights };
type Profile = Catalog & { basis: string | null; sourceHash?: string; annotations: KnowledgeAnnotation[]; media?: HostedMediaDescriptor };
type Preview = { summary?: string; kind?: string; body?: string; replacement?: string; count?: number; source?: { before: string; after: string } };
type Receipt = { operationId: string; status: string; planId?: string; preview?: Preview };
type Pending = { operationId: string; label: string; sourceHash?: string; planId?: string; preview?: Preview; status?: string; intent: KnowledgeIntent };
type Draft = { body: string; replacement: string; purpose: string; target: string; replies: Record<string, string>; media?: { asset: string; resource: string; revision: string; target: MediaRegionInput | null }; pending?: Pending };
type Request = <T>(path: string, body?: unknown) => Promise<T>;
type SourceContext = KnowledgeContext & { resource: string; pending: boolean };
const newDraft = (): Draft => ({ body: "", replacement: "", purpose: "comment", target: "selection", replies: {} });
const needsSource = (kind: string) => ["annotationCreate", "annotationReattach", "annotationReconfirm", "annotationAccept", "annotationProofread"].includes(kind);
const mediaKinds = new Set(["image_region", "pdf_region", "media_interval", "whole_resource"]);
interface MediaHost { context(): HostedMediaSelection | null; open(asset: string, revision?: string, initial?: MediaRegionInput): Promise<void> }

/** Discussion drafts and original transactions are independent from source saves. */
export class HostedReview {
  private epoch = 0;
  private inputScopes: AuthorInputScopes;
  private replyScopes = new Map<string, object>();
  private authorSession(draft: Draft, annotationId?: string) {
    const resource = this.resource, media = draft.media, epoch = this.epoch; if (!resource) return;
    let key: object = draft;
    if (annotationId) { key = this.replyScopes.get(annotationId) ?? {}; this.replyScopes.set(annotationId, key); }
    return this.inputScopes.get(key, { kind: "review", resource, ...(annotationId ? { annotationId } : {}), ...(media ? { asset: media.asset, revision: media.revision } : {}) }, () => this.epoch === epoch && this.draft === draft && this.resource === resource && draft.media?.asset === media?.asset && draft.media?.revision === media?.revision);
  }
  private prefix = "";
  private catalog: Catalog[] = [];
  private resource?: string;
  private profile?: Profile;
  private draft?: Draft;
  private unreadable?: string;
  private busy = false;
  private panel?: ProofreadingPanel;
  private picker = ui("select");
  private editor = ui("section");
  private proofread = ui("section");
  private message = ui("p", "hint");
  constructor(private host: HTMLElement, private request: Request, private context: () => SourceContext | null, private flush: () => Promise<void>, private select: (from: number, to: number) => void, private changed: (resource: string, sourceChanged: boolean) => void, private media?: MediaHost, private processingDefaults?: () => Promise<ProcessingDefaultsSuggestion>, authorInputs?: AuthorInputFactory) { this.inputScopes = new AuthorInputScopes(authorInputs); this.clear(false); }
  mediaChanged(context: HostedMediaSelection | null) {
    if (!this.draft) return;
    if (context && context.descriptor.owner === this.resource && ["media", "mediaWhole"].includes(this.draft.target)) { this.draft.media = { asset: context.descriptor.asset, resource: context.descriptor.resource, revision: context.descriptor.revision, target: context.target && { ...context.target } }; this.keepInput(); }
    this.render();
  }
  async openMedia(context: HostedMediaSelection) {
    if (!this.catalog.some(item => item.resource === context.descriptor.owner && item.rights.read)) throw new Error("此媒体没有独立的批注讨论读取资格；查看不会授予批注权限。");
    await this.run(() => this.open(context.descriptor.owner, context.descriptor.asset));
  }
  clear(remove = true) {
    this.inputScopes.clear(); this.replyScopes.clear(); this.epoch++; this.panel?.dispose(); this.panel = undefined; this.catalog = []; this.resource = undefined; this.profile = undefined; this.draft = undefined; this.unreadable = undefined; this.busy = false;
    this.host.replaceChildren(); this.host.hidden = true; this.editor.replaceChildren(); this.proofread.replaceChildren(); this.message.textContent = ""; this.prefix = "";
    if (remove) for (const key of Object.keys(sessionStorage)) if (key.startsWith("weftext-hosted-review:")) sessionStorage.removeItem(key);
  }
  async connect(deployment: string, actor: string, generation: number) {
    this.clear(false); const epoch = this.epoch;
    const result = await this.request<{ resources: Catalog[] }>("review");
    if (epoch !== this.epoch) return;
    this.prefix = hostedCachePrefix("review", deployment, actor, generation); this.catalog = result.resources;
    this.host.hidden = !this.catalog.length;
    this.picker.setAttribute("aria-label", "选择批注对象");
    this.picker.replaceChildren(...this.catalog.map(item => { const option = ui("option", "", item.resource); option.value = item.resource; return option; }));
    this.host.replaceChildren(ui("h2", "", "批注与校对"), ui("p", "hint", "讨论单独保存。新目标、校对和应用建议使用当前已保存正文；提交前可核对准确变动。"), this.picker, action("读取批注", () => this.run(() => this.open(this.picker.value))), this.message, this.editor, this.proofread);
  }
  private key(resource = this.resource!) { return this.prefix + encodeURIComponent(resource); }
  private retain() { if (this.resource && this.draft) sessionStorage.setItem(this.key(), JSON.stringify(this.draft)); }
  private async run(work: () => Promise<void>) {
    if (this.busy) return; this.busy = true; const epoch = this.epoch; this.render();
    try { await work(); } catch (error) { if (epoch === this.epoch) this.message.textContent = error instanceof Error ? error.message : "操作未完成，输入和原操作仍保留。"; }
    finally { if (epoch === this.epoch) { this.busy = false; this.render(); } }
  }
  private async open(resource: string, asset?: string) {
    if (!resource) return;
    this.retain(); this.inputScopes.clear(); this.replyScopes.clear(); this.panel?.dispose(); this.panel = undefined; this.proofread.replaceChildren();
    this.resource = resource; this.profile = undefined; this.draft = undefined; this.unreadable = undefined;
    const raw = sessionStorage.getItem(this.key());
    try { const draft = raw ? JSON.parse(raw) as Draft : newDraft(); if (!draft || typeof draft.body !== "string" || !draft.replies) throw new Error(); this.draft = draft; }
    catch { this.unreadable = raw ?? ""; throw new Error("批注恢复记录无法解析，原文保留供复制与核对。"); }
    this.render(); const epoch = this.epoch;
    const currentMedia = this.media?.context(); const requestedAsset = asset ?? (currentMedia?.descriptor.owner === resource ? currentMedia.descriptor.asset : undefined);
    const profile = await this.request<Profile>("review/read", { resource, ...(requestedAsset ? { asset: requestedAsset } : {}) });
    if (epoch !== this.epoch) return;
    this.profile = profile; this.message.textContent = this.draft.pending ? "原批注操作待核对，请先核对结果。" : "批注已读取。";
    this.installProofreading();
  }
  private sourceContext() { const context = this.context(); return context && context.resource === this.resource ? context : null; }
  private async requireSource(expected = this.profile?.sourceHash) {
    await this.flush(); const context = this.sourceContext();
    if (!context || context.dirty || context.pending || !expected) throw new Error("请先打开并保存该正文，核对原保存结果，再重新读取批注。");
    const hash = [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(context.source)))].map(n => n.toString(16).padStart(2, "0")).join("");
    if (hash !== expected || this.sourceContext()?.source !== context.source) throw new Error("当前正文与批注依据不同，请保留草稿并重新读取核对。");
    return context;
  }
  private target() {
    if (["media", "mediaWhole"].includes(this.draft?.target ?? "")) {
      const current = this.media?.context(); if (!current || current.descriptor.owner !== this.resource) throw new Error("请先打开与讨论所属对象一致的获权媒体版本。");
      if (!this.profile?.media || this.profile.media.asset !== current.descriptor.asset || this.profile.media.revision !== current.descriptor.revision) throw new Error("媒体依据已变化，请保留批注输入并重新读取当前资源的批注依据。");
      return mediaIntentTarget(current, this.draft!.target === "mediaWhole");
    }
    const context = this.sourceContext(); if (!context) throw new Error("请打开同一对象的正文后选择目标。");
    if (this.draft?.target === "whole") return { kind: "whole_node" };
    const selection = context.selection; if (!selection) throw new Error("请先在编辑器中选择准确位置。");
    return { kind: selection.from === selection.to ? "source_point" : "source_range", from: selection.from, to: selection.to, visibleExcerpt: context.source.slice(selection.from, selection.to) };
  }
  private async prepare(label: string, intent: KnowledgeIntent): Promise<void> {
    const profile = this.profile, draft = this.draft; if (!profile?.basis || !draft || draft.pending) return;
    const epoch = this.epoch;
    const mediaTarget = intent.target && typeof intent.target === "object" && mediaKinds.has(String((intent.target as { kind?: string }).kind));
    if (mediaTarget && (intent.purpose === "suggestion" || intent.kind === "annotationReconfirm")) throw new Error("媒体目标支持讨论或标记；源文本修改建议需要准确已保存正文，原输入保留。");
    if (needsSource(intent.kind) && !mediaTarget) await this.requireSource();
    if (epoch !== this.epoch || profile !== this.profile) return;
    const pending: Pending = { operationId: crypto.randomUUID(), label, intent: structuredClone(intent), sourceHash: needsSource(intent.kind) && !mediaTarget ? profile.sourceHash : undefined };
    draft.pending = pending; this.retain();
    const result = await this.request<Receipt>("review/prepare", { resource: profile.resource, basis: profile.basis, operationId: pending.operationId, intent });
    if (epoch !== this.epoch) return; this.receipt(result);
  }
  private receipt(result: Receipt) {
    const pending = this.draft?.pending; if (!pending) return;
    if (["committed", "cancelled", "resolved"].includes(result.status)) {
      const sourceChanged = pending.intent.kind === "annotationAccept";
      delete this.draft!.pending; this.profile = undefined; this.panel?.dispose(); this.panel = undefined; this.proofread.replaceChildren(); this.retain();
      this.message.textContent = result.status === "committed" ? "原批注操作已提交。请重新读取批注；正文变更仍须与个人草稿核对。" : "原批注操作已结束，未冒作保存成功；输入保留，请重新读取。";
      if (result.status === "committed") this.changed(this.resource!, sourceChanged);
    } else {
      pending.status = result.status; pending.planId = result.planId; pending.preview = result.preview; this.retain();
      this.message.textContent = result.status === "prepared" ? "变动已准备，请核对后明确提交。" : "原操作尚待确认；输入保留，请核对原操作。";
    }
  }
  private async recover() {
    const pending = this.draft?.pending; if (!pending) return; const epoch = this.epoch;
    const request = { resource: this.resource, operationId: pending.operationId };
    let result = await this.request<Receipt>("review/operation", request);
    if (epoch !== this.epoch) return;
    if (["notRegistered", "notCommitted"].includes(result.status)) result = await this.request<Receipt>("review/settle", request);
    if (epoch === this.epoch) this.receipt(result);
  }
  private installProofreading() {
    if (!this.profile?.basis || !this.profile.rights.proofread) return;
    const epoch = this.epoch;
    this.panel = new ProofreadingPanel({ context: () => epoch === this.epoch ? this.sourceContext() : null,
      processingDefaults: this.processingDefaults,
      beforeAction: () => this.requireSource().then(() => {}),
      scan: async (_, options) => { await this.requireSource(); if (!this.profile) throw new Error("请重新读取批注"); return this.request<ProofreadResult>("review/proofread", { resource: this.resource, basis: this.profile.basis, options }); },
      perform: async (label, intent) => { await this.run(() => this.prepare(label, intent)); return false; },
      selectSource: this.select, notify: message => { this.message.textContent = message; },
    }, { storageKey: null, title: "正文校对" });
    this.panel.render(this.proofread);
  }
  private render() {
    this.editor.replaceChildren(); const draft = this.draft, profile = this.profile;
    if (this.unreadable !== undefined) { this.editor.append(ui("pre", "", this.unreadable), action("放弃损坏的批注恢复记录", () => { sessionStorage.removeItem(this.key()); this.unreadable = undefined; return this.run(() => this.open(this.resource!)); })); return; }
    if (!draft || !this.resource) return;
    const disabled = this.busy || !!draft.pending;
    const form = ui("fieldset"); form.disabled = disabled || !profile?.rights.write || !profile.basis;
    const body = ui("textarea"); body.value = draft.body; body.rows = 3; this.authorSession(draft)?.bind(body, "body"); body.setAttribute("aria-label", "批注内容"); body.oninput = () => { draft.body = body.value; this.keepInput(); };
    const purpose = ui("select"); purpose.setAttribute("aria-label", "批注类型");
    for (const [value, label] of [["comment", "讨论"], ["mark", "标记"], ["suggestion", "修改建议"]]) { const o = ui("option", "", label); o.value = value; purpose.append(o); } purpose.value = draft.purpose; purpose.onchange = () => { draft.purpose = purpose.value; this.keepInput(); };
    const target = ui("select"); target.setAttribute("aria-label", "批注目标");
    for (const [value, label] of [["selection", "当前选区或插入点"], ["whole", "整篇笔记"], ["media", "真实媒体区域或区间"], ["mediaWhole", "整份准确媒体资源"]]) { const o = ui("option", "", label); o.value = value; target.append(o); } target.value = draft.target; target.onchange = () => { draft.target = target.value; const current = this.media?.context(); if (current && current.descriptor.owner === this.resource && ["media", "mediaWhole"].includes(draft.target)) draft.media = { asset: current.descriptor.asset, resource: current.descriptor.resource, revision: current.descriptor.revision, target: current.target }; this.keepInput(); this.render(); };
    const replacement = ui("textarea"); replacement.rows = 2; replacement.value = draft.replacement; this.authorSession(draft)?.bind(replacement, "replacement"); replacement.setAttribute("aria-label", "建议替换文字"); replacement.oninput = () => { draft.replacement = replacement.value; this.keepInput(); };
    const create = action("准备新批注", () => this.run(() => this.prepare("新增批注", { kind: "annotationCreate", body: draft.body, purpose: draft.purpose, replacement: draft.replacement, target: this.target() })));
    const mediaTarget = ["media", "mediaWhole"].includes(draft.target), mediaContext = this.media?.context();
    create.disabled = !profile?.rights.target || (mediaTarget ? !mediaContext || mediaContext.descriptor.owner !== this.resource || profile.media?.asset !== mediaContext.descriptor.asset || profile.media.revision !== mediaContext.descriptor.revision || mediaContext.descriptor.lifecycle !== "live" || !(draft.target === "mediaWhole" ? mediaContext.descriptor.wholeAnnotationReady : mediaContext.descriptor.annotationReady && !!mediaContext.target) : !profile?.rights.sourceRead);
    form.append(purpose, target, body, ui("p", "hint", "修改建议使用下方替换文字；空替换表示删除。"), replacement, create); this.editor.append(form);
    if (draft.media) { this.editor.append(ui("p", "hint", `原媒体选择：${draft.media.asset} · ${draft.media.revision}。准确几何目标保留；不会按当前其他版本重定位。`)); const reopen = action("重新查看原媒体选择", () => this.run(async () => { await this.media?.open(draft.media!.asset, draft.media!.revision, draft.media!.target ?? undefined); })); reopen.disabled = this.busy; this.editor.append(reopen); }
    const reread = action("重新读取批注", () => this.run(() => this.open(this.resource!))); reread.disabled = this.busy || !!draft.pending; this.editor.append(reread);
    if (draft.pending) {
      const pending = draft.pending, card = ui("section", "review-pending"); card.append(ui("h3", "", "核对原批注操作"), ui("p", "", pending.preview?.summary ?? pending.label));
      if (pending.preview?.count !== undefined) card.append(ui("p", "", `${pending.preview.count} 项变动`));
      for (const [label, content] of [["批注", pending.preview?.body ?? pending.intent.body], ["建议", pending.preview?.replacement ?? pending.intent.replacement]] as const) if (typeof content === "string") card.append(ui("p", "", label), ui("pre", "", content));
      if (pending.preview?.source) card.append(ui("p", "", "提交前正文"), ui("pre", "", pending.preview.source.before), ui("p", "", "提交后正文"), ui("pre", "", pending.preview.source.after));
      if (pending.intent.target && mediaKinds.has(String((pending.intent.target as { kind?: string }).kind))) card.append(ui("p", "hint", "原媒体目标已冻结；移动当前选区不会更改它。确认提交的是此原计划，服务端将重新核验准确版本与权限。"), ui("pre", "", JSON.stringify(pending.intent.target, null, 2)));
      const commit = action("确认提交批注操作", () => this.run(async () => { const epoch = this.epoch; if (pending.sourceHash) await this.requireSource(pending.sourceHash); if (epoch !== this.epoch) return; const result = await this.request<Receipt>("review/commit", { resource: this.resource, operationId: pending.operationId, planId: pending.planId }); if (epoch === this.epoch) this.receipt(result); }));
      commit.disabled = this.busy || pending.status !== "prepared" || !pending.planId;
      const recover = action("查询原批注操作", () => this.run(() => this.recover())); recover.disabled = this.busy;
      const cancel = action("结束原批注操作并保留输入", () => this.run(async () => { const epoch = this.epoch; const result = await this.request<Receipt>("review/settle", { resource: this.resource, operationId: pending.operationId }); if (epoch === this.epoch) this.receipt(result); })); cancel.disabled = this.busy;
      card.append(commit, recover, cancel); this.editor.append(card);
    }
    for (const annotation of profile?.annotations ?? []) {
      const card = ui("article", "knowledge-card"); card.setAttribute("data-annotation-id", annotation.id);
      const suggestionState = annotation.suggestionState ? ({pending: "待接受", accepted: "已接受", rejected: "已拒绝"})[annotation.suggestionState] : "";
      card.append(ui("h3", "", annotation.purpose === "suggestion" ? "修改建议" : "批注"), ui("p", "", `${annotation.author ?? ""} · ${annotation.reviewState === "resolved" ? "已解决" : "未解决"}${suggestionState ? ` · ${suggestionState}` : ""}`), ui("p", "", annotation.body));
      if (annotation.visibleExcerpt) card.append(ui("blockquote", "", typeof annotation.visibleExcerpt === "string" ? annotation.visibleExcerpt : annotation.visibleExcerpt.text));
      if (annotation.replacement !== undefined) card.append(ui("p", "", `建议替换：${annotation.replacement}`));
      if (annotation.targetResolution) card.append(ui("p", "hint", `目标：${({exact: "准确定位", mapped: "已准确映射", stale: "原目标已变化", ambiguous: "不能唯一定位", orphaned: "原目标已不存在", unavailable: "当前不可用"})[annotation.targetResolution]}`));
      if (annotation.sourceLocation && this.sourceContext()) card.append(action("定位批注目标", () => this.run(async () => { const epoch = this.epoch; await this.requireSource(); if (epoch === this.epoch && profile === this.profile) this.select(annotation.sourceLocation!.from, annotation.sourceLocation!.to); })));
      const location = annotation.resourceLocation as (KnowledgeAnnotation["resourceLocation"] & { asset?: string }) | undefined;
      if (location?.asset && this.media) card.append(ui("p", "hint", `准确媒体目标：${location.asset} · ${location.revision}`), action("查看批注媒体目标", () => this.run(async () => { await this.media!.open(location.asset!, location.revision, mediaAnnotationSelection(location.target) ?? undefined); })));
      for (const reply of annotation.replies ?? []) card.append(ui("p", "", `${reply.author ?? ""}：${reply.body}`));
      if (profile?.rights.write) {
        const controls = ui("fieldset"); controls.disabled = disabled || !profile.basis;
        const reply = ui("textarea"); reply.rows = 2; reply.setAttribute("aria-label", `回复 ${annotation.id}`); reply.value = draft.replies[annotation.id] ?? ""; this.authorSession(draft, annotation.id)?.bind(reply, `reply:${annotation.id}`); reply.oninput = () => { draft.replies[annotation.id] = reply.value; this.keepInput(); };
        controls.append(reply, action("准备回复", () => this.run(() => this.prepare("回复批注", { kind: "annotationReply", annotationId: annotation.id, body: draft.replies[annotation.id] ?? "" }))), action(annotation.reviewState === "resolved" ? "准备重新打开讨论" : "准备解决讨论", () => this.run(() => this.prepare("改变讨论状态", { kind: "annotationResolve", annotationId: annotation.id, resolved: annotation.reviewState !== "resolved" }))));
        if (annotation.suggestionState === "pending") {
          controls.append(action("准备拒绝建议", () => this.run(() => this.prepare("拒绝建议", { kind: "annotationReject", annotationId: annotation.id }))));
          if (profile.rights.accept && canApplySuggestion(annotation)) controls.append(action("准备应用建议", () => this.run(() => this.prepare("应用建议到正文", { kind: "annotationAccept", annotationId: annotation.id }))));
        }
        if (profile.rights.target && (profile.rights.sourceRead || mediaTarget && mediaContext?.descriptor.owner === this.resource && mediaContext.descriptor.lifecycle === "live")) {
          if (annotation.purpose === "suggestion" && annotation.suggestionState === "pending") { if (profile.rights.sourceRead && !mediaTarget) controls.append(action("准备重新确认建议", () => this.run(() => this.prepare("确认新建议依据", { kind: "annotationReconfirm", annotationId: annotation.id, target: this.target(), replacement: draft.replacement })))); }
          else controls.append(action("准备重新定位", () => this.run(() => this.prepare("重新定位批注", { kind: "annotationReattach", annotationId: annotation.id, target: this.target() }))));
        }
        card.append(controls);
      }
      this.editor.append(card);
    }
  }
  private keepInput() { try { this.retain(); this.message.textContent = "批注输入已保留，尚未提交。"; } catch { this.message.textContent = "无法保留批注恢复信息，请先复制输入。"; } }
}
