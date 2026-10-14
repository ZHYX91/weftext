import { action, ui } from "./data-panel";
import { mountMediaViewer, type MediaViewer } from "./media-viewer";
import type { MediaRegionInput } from "./knowledge-state";
import type { ResourceOpenTarget } from "./media-selection";
import { decodeMediaDescriptor, mediaChunk, mediaFingerprint, mediaResolved, mediaTicket, mediaViewerDescriptor, type HostedMediaCatalog, type HostedMediaChunk, type HostedMediaDescriptor, type HostedMediaResolved, type HostedMediaSelection, type HostedMediaTicket, type MediaPurpose } from "./hosted-media-state";
export type HostedMediaRequest = <T>(path: string, body?: unknown, signal?: AbortSignal) => Promise<T>;

/** Exact asset bytes are independent of a note session and never persisted. */
export class HostedMedia {
  private epoch = 0;
  private version = 0;
  private catalogVersion = 0;
  private catalog: HostedMediaCatalog = { resources: [], coverage: "complete", scope: "configuredAuthorizedResources" };
  private selected?: HostedMediaDescriptor;
  private target: MediaRegionInput | null = null;
  private viewer?: MediaViewer;
  private controllers = new Set<AbortController>();
  private urls = new Set<string>();
  private picker = ui("select");
  private surface = ui("section");
  private actions = ui("section");
  private message = ui("p", "hint");
  private downloading = false;
  constructor(private host: HTMLElement, private request: HostedMediaRequest, private selectionChanged: (context: HostedMediaSelection | null) => void, private review: (context: HostedMediaSelection) => void | Promise<void>) { this.clear(); }
  context(): HostedMediaSelection | null { return this.selected ? { descriptor: structuredClone(this.selected), target: this.target && { ...this.target } } : null; }
  private stopView() {
    this.version++; for (const controller of this.controllers) controller.abort(); this.controllers.clear(); this.viewer?.dispose(); this.viewer = undefined;
    for (const url of this.urls) URL.revokeObjectURL(url); this.urls.clear(); this.selected = undefined; this.target = null; this.downloading = false; this.surface.replaceChildren(); this.actions.replaceChildren(); this.selectionChanged(null);
  }
  clear() { this.epoch++; this.catalogVersion++; this.stopView(); this.catalog = { resources: [], coverage: "complete", scope: "configuredAuthorizedResources" }; this.picker.replaceChildren(); this.message.textContent = ""; this.host.replaceChildren(); this.host.hidden = true; }
  async connect(_deployment: string, _actor: string, _generation: number) {
    this.clear(); this.host.hidden = false; this.picker.setAttribute("aria-label", "托管获权资源"); this.picker.onchange = () => { void this.openAsset(this.picker.value).catch(error => this.show(error)); };
    this.host.replaceChildren(ui("h2", "", "获权资源与媒体"), ui("p", "hint", "查看、下载与媒体批注分别授权。资源查看不打开或保存正文；回收站资源仅供只读查看。"), this.picker, action("刷新获权资源目录", () => this.refresh().catch(error => this.show(error))), this.message, this.surface, this.actions); await this.refresh();
  }
  private show(error: unknown) { this.message.textContent = error instanceof Error ? error.message : "资源当前不可用，未采用其他版本。"; }
  private stamp() { return { epoch: this.epoch, version: this.version }; }
  private current(stamp: ReturnType<HostedMedia["stamp"]>) { return stamp.epoch === this.epoch && stamp.version === this.version; }
  async refresh() {
    const epoch = this.epoch, catalogVersion = ++this.catalogVersion, result = await this.request<HostedMediaCatalog>("media"); if (epoch !== this.epoch || catalogVersion !== this.catalogVersion) return;
    if (!result || result.scope !== "configuredAuthorizedResources" || !["complete", "partial"].includes(result.coverage) || !Array.isArray(result.resources)) throw new Error("媒体目录缺少获权范围或完整性状态。");
    const previous = this.selected; this.catalog = { ...result, resources: result.resources.map(decodeMediaDescriptor) };
    if (previous) { const fresh = this.catalog.resources.find(item => item.asset === previous.asset); if (!fresh || fresh.status !== "ready" || mediaFingerprint(fresh) !== mediaFingerprint(previous)) this.stopView(); else this.selected = fresh; }
    const empty = ui("option", "", "选择获权资源…"); empty.value = ""; this.picker.replaceChildren(empty, ...this.catalog.resources.map(item => { const option = ui("option", "", `${item.title}${item.lifecycle === "trash" ? "（回收站只读）" : ""}${item.status === "unavailable" ? "（不可用）" : ""}`); option.value = item.asset; option.disabled = item.status === "unavailable"; return option; })); this.picker.value = this.selected?.asset ?? "";
    this.message.textContent = result.coverage === "partial" ? "资源目录部分可用；不可用获权条目保留，不能当作完整空集合。" : result.resources.length ? "只列显式获权资源；选择后读取准确版本。" : "本次获权目录完整，无可交付资源。"; this.renderActions();
  }
  async openReference(reference: string, revision?: string, initial?: MediaRegionInput) {
    this.stopView(); const stamp = this.stamp(); this.message.textContent = "正在独立解析获权稳定资源引用…";
    try { const resolved = mediaResolved(await this.request<HostedMediaResolved>("media/resolve", { reference, ...(revision ? { revision } : {}) }), reference, revision); if (!this.current(stamp)) return;
      this.mount(resolved, resolved.viewerTarget, initial);
    } catch (error) { if (this.current(stamp)) { this.stopView(); this.show(error); } }
  }
  async openAsset(asset: string, revision?: string, initial?: MediaRegionInput) {
    const candidate = this.catalog.resources.find(item => item.asset === asset); if (!candidate || candidate.status !== "ready") { this.stopView(); this.message.textContent = "所选资源当前不可用；未按名称或路径猜测替代项。"; return; }
    if (revision && revision !== candidate.revision) { this.stopView(); throw new Error("原资源版本已变化；目标输入保留，未改用最新版。"); }
    await this.openReference(candidate.resource, revision ?? candidate.revision, initial);
  }
  private mount(descriptor: HostedMediaDescriptor, open: ResourceOpenTarget, initial?: MediaRegionInput) {
    this.selected = descriptor; this.picker.value = descriptor.asset; const stamp = this.stamp(), viewerId = `hosted-media:${stamp.epoch}:${stamp.version}`;
    this.viewer = mountMediaViewer(this.surface, {
      context: () => this.current(stamp) ? { viewerId } : null,
      list: async () => ({ resources: [mediaViewerDescriptor(descriptor)] }),
      read: async (_lifetime, resource, revision) => { if (resource !== descriptor.resource || revision !== descriptor.revision) throw new Error("查看器请求不对应原准确资源。"); return this.readBlob(descriptor, "view", stamp); },
      notify: (message, error) => { if (this.current(stamp)) { if (error) this.stopView(); this.message.textContent = message; } },
    }, target => { if (this.current(stamp)) { this.target = target; this.selectionChanged(this.context()); } }, initial, { open, fixedResource: true, readOnly: descriptor.lifecycle === "trash" || !descriptor.annotationReady, allowSaveCopy: false });
    this.selectionChanged(this.context()); this.renderActions();
  }
  private async readBlob(descriptor: HostedMediaDescriptor, purpose: MediaPurpose, stamp = this.stamp()): Promise<Blob> {
    if (purpose === "download" && !descriptor.download) throw new Error("此资源没有独立下载资格。");
    const controller = new AbortController(); this.controllers.add(controller);
    try {
      const ticket = mediaTicket(descriptor, purpose, await this.request<HostedMediaTicket>(purpose === "view" ? "media/read" : "media/download/prepare", { asset: descriptor.asset, revision: descriptor.revision }, controller.signal));
      const chunks: Uint8Array<ArrayBuffer>[] = [];
      for (let offset = 0, done = false; !done;) {
        if (!this.current(stamp)) throw new Error("资源查看上下文已变化，停止原字节交付。");
        const response = await this.request<HostedMediaChunk>(purpose === "view" ? "media/chunk" : "media/download/chunk", { ticket: ticket.ticket, offset }, controller.signal);
        const chunk = mediaChunk(ticket, offset, response); chunks.push(new Uint8Array(chunk)); offset += chunk.length; done = response.done;
      }
      const bytes = new Uint8Array(ticket.bytes); let offset = 0; for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
      const digest = [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))].map(n => n.toString(16).padStart(2, "0")).join("");
      if (offset !== ticket.bytes || digest !== ticket.sha256 || !this.current(stamp)) throw new Error("资源长度、摘要或当前资格不符，未生成查看/下载 Blob。");
      return new Blob([bytes], { type: ticket.contentType });
    } finally { this.controllers.delete(controller); }
  }
  private async download() {
    if (!this.selected || !this.selected.download || this.downloading) return; const descriptor = this.selected, stamp = this.stamp(); this.downloading = true; this.renderActions();
    try { const blob = await this.readBlob(descriptor, "download", stamp); if (!this.current(stamp)) return; const url = URL.createObjectURL(blob); this.urls.add(url); const link = ui("a"); link.href = url; link.download = descriptor.name; link.click(); setTimeout(() => { URL.revokeObjectURL(url); this.urls.delete(url); }, 1000); this.message.textContent = "已下载独立获权、校验完整的原资源版本。"; }
    catch (error) { if (this.current(stamp)) this.show(error); }
    finally { if (this.current(stamp)) { this.downloading = false; this.renderActions(); } }
  }
  private renderActions() {
    this.actions.replaceChildren(); const descriptor = this.selected; if (!descriptor) return;
    this.actions.append(ui("p", "hint", `${descriptor.title} · ${descriptor.revision}${descriptor.lifecycle === "trash" ? " · 回收站只读，不能创建或移动批注目标" : ""}`));
    if (descriptor.download) { const download = action("下载准确获权资源", () => this.download()); download.disabled = this.downloading; this.actions.append(download); }
    const annotations = action("读取此资源的批注讨论", () => { const context = this.context(); if (context) return Promise.resolve(this.review(context)).catch(error => this.show(error)); }); this.actions.append(annotations);
  }
}
