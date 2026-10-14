import { action, ui } from "./data-panel";
import { hostedCachePrefix } from "./hosted-state";
import { canCommitResourceImport, decodeResourceImportDraft, freezeResourceImport, resourceImportEnded, resourceImportInput, resourceImportReceipt, type HostedResourceCollection, type ResourceImportDraft, type ResourceImportReceipt } from "./hosted-media-import-state";
type Request = <T>(path: string, body?: unknown) => Promise<T>;
const initial = (): ResourceImportDraft => ({ collection: "", name: "", revision: 0 });
/** Own upload bytes and original operations, without reading any owner source. */
export class HostedResourceImports {
  private epoch = 0;
  private key = "";
  private draft = initial();
  private collections: HostedResourceCollection[] = [];
  private unreadable?: string;
  private original?: File;
  private reading = false;
  private busy = false;
  private picker = ui("select");
  private name = ui("input");
  private file = ui("input");
  private message = ui("p", "hint");
  private details = ui("p", "hint");
  private records = ui("section");
  private prepareButton = action("准备资源集合导入", () => this.run(() => this.prepare()));
  constructor(private host: HTMLElement, private request: Request, private changed: () => Promise<void>) { this.clear(false); }
  get hasPending() { return !!this.draft.pending && !resourceImportEnded(this.draft.pending.receipt?.status); }
  clear(removeCache = true) {
    this.epoch++; this.key = ""; this.draft = initial(); this.collections = []; this.original = undefined; this.reading = false; this.busy = false; this.unreadable = undefined;
    this.host.replaceChildren(); this.host.hidden = true; this.picker.replaceChildren(); this.name.value = ""; this.file.value = ""; this.picker.disabled = this.name.disabled = this.file.disabled = false; this.message.textContent = ""; this.details.textContent = ""; this.records.replaceChildren();
    if (removeCache) for (const key of Object.keys(sessionStorage)) if (key.startsWith("weftext-hosted-resource-import:")) sessionStorage.removeItem(key);
  }
  connect(deployment: string, actor: string, generation: number) {
    this.clear(false); this.key = hostedCachePrefix("resource-import", deployment, actor, generation) + "state";
    let raw: string | null = null;
    try { raw = sessionStorage.getItem(this.key); if (raw) this.draft = decodeResourceImportDraft(raw); } catch { this.unreadable = raw ?? ""; }
    this.picker.setAttribute("aria-label", "托管资源导入集合"); this.name.setAttribute("aria-label", "托管资源导入名称"); this.file.setAttribute("aria-label", "托管资源导入原件"); this.file.type = "file";
    this.picker.onchange = () => { this.draft.collection = this.picker.value; this.inputChanged(); };
    this.name.oninput = () => { this.draft.name = this.name.value; this.inputChanged(); };
    this.file.onchange = () => { const file = this.file.files?.[0]; if (file) void this.chooseFile(file); };
    this.host.replaceChildren(ui("h3", "", "资源集合导入"), ui("p", "hint", "集合读取、下载与导入分别授权。只上传原件；先核对准确计划再明确提交，不保存或替换正文。"), this.picker, this.name, this.file, this.details, this.prepareButton, this.message, this.records); this.render();
  }
  setCatalog(collections: HostedResourceCollection[]) {
    this.collections = collections;
    if (!this.draft.collection && !this.draft.pending) this.draft.collection = collections.find(item => item.import && item.status === "ready")?.collection ?? "";
    this.picker.replaceChildren(...collections.map(item => { const option = ui("option", "", `${item.title} · ${item.read ? "可读" : "无读取权"} / ${item.download ? "可下载" : "无下载权"} / ${item.import ? "可导入" : "无导入权"}${item.status === "unavailable" ? "（当前不可用）" : ""}${item.lifecycle === "trash" ? "（回收站只读）" : ""}`); option.value = item.collection; return option; }));
    if (this.draft.collection && !collections.some(item => item.collection === this.draft.collection)) { const old = ui("option", "", `${this.draft.collection}（原集合当前不可用，输入保留）`); old.value = this.draft.collection; this.picker.append(old); }
    this.render();
  }
  private persist(value = this.draft) { if (!this.key || this.unreadable !== undefined) throw new Error("资源导入恢复记录不可写，请先核对原记录。"); try { sessionStorage.setItem(this.key, JSON.stringify(value)); } catch { throw new Error("浏览器恢复缓存容量不足或不可写；原件 File 和输入仍在本页，未发送新的资源写入。"); } }
  private keep() { try { this.persist(); } catch (error) { this.message.textContent = (error as Error).message; } }
  private inputChanged() { this.draft.revision++; this.keep(); this.render(); }
  private async chooseFile(file: File) {
    const epoch = this.epoch; this.original = file; this.reading = true; this.draft.bytesBase64 = undefined; this.draft.bytes = file.size; this.draft.fileName = file.name; if (!this.draft.name) this.draft.name = file.name; this.draft.revision++; const revision = this.draft.revision; this.keep(); this.render();
    try { if (file.size > 16 * 1024 * 1024) throw new Error("资源导入超过 16 MiB 上限，原件仍在本页，未发起写入。"); const bytes = new Uint8Array(await file.arrayBuffer()); let binary = ""; for (let offset = 0; offset < bytes.length; offset += 32768) binary += String.fromCharCode(...bytes.subarray(offset, offset + 32768));
      if (epoch !== this.epoch || this.original !== file) return;
      this.draft.bytesBase64 = btoa(binary); this.draft.bytes = bytes.length; this.draft.revision++; this.keep(); this.message.textContent = revision !== this.draft.revision - 1 ? "读取期间的新名称或集合输入保留，原件已取得；尚未准备写入。" : "原件已完整读取；准备和提交均须先可靠保留原请求。";
    } catch (error) { if (epoch === this.epoch && this.original === file) this.message.textContent = (error as Error).message; }
    finally { if (epoch === this.epoch && this.original === file) { this.reading = false; this.render(); } }
  }
  private async run(work: () => Promise<void>) { if (this.busy) return; const epoch = this.epoch; this.busy = true; this.render(); try { await work(); } catch (error) { if (epoch === this.epoch) this.message.textContent = error instanceof Error ? error.message : "资源导入结果未确认，原请求保留，请查询原编号。"; } finally { if (epoch === this.epoch) { this.busy = false; this.render(); } } }
  private async prepare() {
    if (this.reading) throw new Error("原件仍在读取，请保留当前输入。"); const collection = this.collections.find(item => item.collection === this.draft.collection), request = freezeResourceImport(this.draft, collection, crypto.randomUUID());
    const next = { ...this.draft, pending: { request, owner: collection!.owner, input: resourceImportInput(this.draft), revision: this.draft.revision } }; this.persist(next); this.draft = next;
    const epoch = this.epoch, result = await this.request<ResourceImportReceipt>("media/import/prepare", request); if (epoch === this.epoch) this.receipt(result);
  }
  private receipt(value: ResourceImportReceipt) { const pending = this.draft.pending; if (!pending) return; pending.receipt = resourceImportReceipt(pending.request, value, pending.receipt); this.keep(); this.message.textContent = value.status === "committed" ? "原资源导入已提交；新定位符不授予读取权限，正文草稿与撤销历史保留。" : value.status === "prepared" ? "原上传计划已准备，请核对名称、字节和原编号后明确提交。" : resourceImportEnded(value.status) ? "原导入已结束，未冒作保存成功；原件和新输入保留。" : "原操作仍待核对；不会换编号或采用当前其他文件重放。"; }
  private render() {
    this.host.hidden = !this.collections.length && !this.draft.pending && !this.draft.bytesBase64 && this.unreadable === undefined;
    if (this.name.value !== this.draft.name) this.name.value = this.draft.name; this.picker.value = this.draft.collection;
    this.details.textContent = this.draft.fileName ? `原件：${this.draft.fileName} · ${this.draft.bytes ?? "未知"} 字节${this.reading ? " · 正在读取" : ""}；文件字节和名称与正文分开保留。` : "请选择要上传的原件，最多 16 MiB。";
    const collection = this.collections.find(item => item.collection === this.draft.collection); this.prepareButton.disabled = this.busy || this.reading || !!this.draft.pending || !collection?.import || collection.status !== "ready" || typeof this.draft.bytesBase64 !== "string" || this.unreadable !== undefined;
    this.records.replaceChildren();
    if (this.unreadable !== undefined) { this.picker.disabled = this.name.disabled = this.file.disabled = true; this.records.append(ui("p", "hint", "恢复记录无法解析，原文未清除，未发送新的操作。"), ui("pre", "", this.unreadable), action("明确放弃损坏的资源导入恢复记录", () => { sessionStorage.removeItem(this.key); this.unreadable = undefined; this.draft = initial(); this.picker.disabled = this.name.disabled = this.file.disabled = false; this.render(); })); return; }
    const pending = this.draft.pending; if (!pending) return; const receipt = pending.receipt, card = ui("section", "resource-import-pending");
    card.append(ui("h4", "", "原资源导入操作"), ui("p", "", `集合 ${pending.request.collection} · 原所属对象 ${pending.owner} · ${pending.request.operationId}`), ui("p", "", `状态：${receipt?.status ?? "unknown"}`));
    if (receipt?.preview) { const summary = receipt.preview.summary; card.append(ui("pre", "", Array.isArray(summary) ? summary.join("\n") : summary ?? ""), ui("p", "", `${receipt.preview.name ?? "本次上传预览当前不可用"} · ${receipt.preview.bytes ?? "未知"} 字节`)); }
    if (pending.revision !== this.draft.revision || pending.input !== resourceImportInput(this.draft)) card.append(ui("p", "hint", "后来输入或同值往返已改变依据；原计划不会提交当前新输入。请核对或明确结束原操作后再准备新上传。"));
    const recover = action("查询原资源导入操作", () => this.run(async () => { const epoch = this.epoch, value = await this.request<ResourceImportReceipt>("media/import/operation", { collection: pending.request.collection, operationId: pending.request.operationId }); if (epoch === this.epoch) { this.receipt(value); if (value.status === "committed") await this.changed(); } })); recover.disabled = this.busy;
    const retry = action("按原上传请求重试准备", () => this.run(async () => { this.persist(); const epoch = this.epoch, value = await this.request<ResourceImportReceipt>("media/import/prepare", pending.request); if (epoch === this.epoch) { this.receipt(value); if (value.status === "committed") await this.changed(); } })); retry.disabled = this.busy || resourceImportEnded(receipt?.status);
    const commit = action("确认提交原资源导入计划", () => this.run(async () => { if (!canCommitResourceImport(this.draft, collection) || this.reading) throw new Error("当前输入或集合资格已变化，未提交旧计划。"); this.persist(); const epoch = this.epoch, value = await this.request<ResourceImportReceipt>("media/import/commit", { collection: pending.request.collection, operationId: pending.request.operationId, planId: pending.receipt!.planId }); if (epoch === this.epoch) { this.receipt(value); if (value.status === "committed") await this.changed(); } })); commit.disabled = this.busy || this.reading || !canCommitResourceImport(this.draft, collection);
    const settle = action("结束未安装的原资源导入", () => this.run(async () => { const epoch = this.epoch, value = await this.request<ResourceImportReceipt>("media/import/settle", { collection: pending.request.collection, operationId: pending.request.operationId }); if (epoch === this.epoch) this.receipt(value); })); settle.disabled = this.busy || resourceImportEnded(receipt?.status); card.append(commit, recover, retry, settle);
    if (receipt?.status === "committed") { card.append(ui("p", "hint", "已提交定位符仅用于后续独立鉴权查看，不代表自动获权。"), ui("pre", "", (receipt.resourceRefs ?? []).join("\n"))); }
    if (resourceImportEnded(receipt?.status)) card.append(action("清除本页已结束导入回执，保留文件输入", () => { const next = { ...this.draft, pending: undefined }; this.persist(next); this.draft = next; this.render(); })); this.records.append(card);
  }
}
