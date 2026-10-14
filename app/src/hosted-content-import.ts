import { action, ui } from "./data-panel";
import { hostedCachePrefix } from "./hosted-state";
import { sameHostedLinksIdentity, type HostedLinksIdentity } from "./hosted-knowledge-links-state";
import { canCommitContent, contentBytes, contentEnded, contentFormats, contentInspection, contentReceipt, decodeContentDraft, freezeContentImport, type ContentDraft, type ContentFormat, type ContentInspection, type ContentMembers, type ContentReceipt, type ContentTarget } from "./hosted-content-import-state";
type Request = <T>(path: string, body?: unknown) => Promise<T>;
export interface HostedContentImportHost { identity(): HostedLinksIdentity | null; authorizationChanged(message: string, current: { actor: string; deployment: string; generation: number }): void; open(resource: string): Promise<void> }
const initial = (): ContentDraft => ({ slot: "", format: "native", revision: 0 });
const labels: Record<ContentFormat, string> = { native: "原生资料包", bibtex: "BibTeX", "csl-json": "CSL JSON", ris: "RIS" };
const sha = async (bytes: Uint8Array) => [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes as Uint8Array<ArrayBuffer>))].map(v => v.toString(16).padStart(2, "0")).join("");
/** A content producer is independent of both the current source and future readers. */
export class HostedContentImport {
  private epoch = 0;
  private key = "";
  private draft = initial();
  private targets: ContentTarget[] = [];
  private verified = false;
  private busy = false;
  private reading = false;
  private original?: File;
  private unreadable?: string;
  private inspected?: { revision: number; value: ContentInspection };
  private reviewedRevision?: number;
  private acceptedLosses = false;
  private members?: ContentMembers;
  private slot = ui("select", "data-input");
  private format = ui("select", "data-input");
  private file = ui("input", "data-input");
  private message = ui("p", "hint");
  private details = ui("p", "hint");
  private inspection = ui("section");
  private records = ui("section");
  private memberView = ui("section");
  private inspectButton = action("检查导入原件", () => { void this.run(() => this.inspect()); });
  private prepareButton = action("准备固定槽资料导入", () => { void this.run(() => this.prepare()); });
  constructor(private host: HTMLElement, private request: Request, private owner: HostedContentImportHost) { this.clear(false); }
  clear(removeCache = true): void {
    this.epoch++; this.key = ""; this.draft = initial(); this.targets = []; this.verified = false; this.busy = this.reading = false; this.original = undefined; this.unreadable = undefined; this.inspected = undefined; this.reviewedRevision = undefined; this.acceptedLosses = false; this.members = undefined;
    this.host.replaceChildren(); this.host.hidden = true; this.slot.replaceChildren(); this.format.replaceChildren(); this.file.value = ""; this.slot.disabled = this.format.disabled = this.file.disabled = false; this.message.textContent = ""; this.records.replaceChildren(); this.inspection.replaceChildren(); this.memberView.replaceChildren();
    if (removeCache) for (const key of Object.keys(sessionStorage)) if (key.startsWith("weftext-hosted-content-import:")) sessionStorage.removeItem(key);
  }
  async connect(deployment: string, actor: string, generation: number): Promise<void> {
    this.clear(false); const epoch = this.epoch, identity = this.owner.identity(); if (!identity || identity.actor !== actor || identity.deployment !== deployment || identity.generation !== generation) return;
    this.key = hostedCachePrefix("content-import", deployment, actor, generation) + "state";
    try { const raw = sessionStorage.getItem(this.key); if (raw) this.draft = decodeContentDraft(raw); } catch { this.unreadable = "恢复记录无法解析，原记录未删除；请保留原件，新的资料写入暂停。"; }
    this.slot.setAttribute("aria-label", "资料导入固定槽位"); this.format.setAttribute("aria-label", "资料导入格式"); this.file.setAttribute("aria-label", "资料导入原件"); this.file.type = "file";
    this.slot.onchange = () => { this.draft.slot = this.slot.value; this.inputChanged(); };
    this.format.onchange = () => { this.draft.format = this.format.value as ContentFormat; this.inputChanged(); };
    this.file.onchange = () => { const file = this.file.files?.[0]; if (file) void this.chooseFile(file); };
    this.host.replaceChildren(ui("h2", "", "原生资料与文献导入"), ui("p", "hint", "在管理员批准的固定空槽中新增，保留原件和未知信息。检查冲突与损失后明确提交；不更新已有 Work，不提供 typed 引用映射，不保存或替换当前正文。"), action("刷新资料导入槽位", () => { void this.run(() => this.refresh()); }), this.slot, this.format, this.file, this.details, this.inspectButton, this.prepareButton, this.message, this.inspection, this.records, this.memberView);
    this.render(); await this.refresh(); if (epoch === this.epoch && this.draft.job && !this.unreadable) await this.run(() => this.lookup());
  }
  private async authority(identity: HostedLinksIdentity | null): Promise<boolean> {
    if (!identity || !sameHostedLinksIdentity(identity, this.owner.identity())) return false;
    const current = await this.request<{ actor: string; deployment: string; generation: number }>("me");
    if (!sameHostedLinksIdentity(identity, this.owner.identity())) return false;
    if (current.actor !== identity.actor || current.deployment !== identity.deployment || current.generation !== identity.generation) { this.owner.authorizationChanged("授权策略已变化，旧导入展示已清除，请重新连接。", current); return false; } return true;
  }
  private persist(value = this.draft): void { if (!this.key || this.unreadable) throw new Error("导入恢复记录不可写，未发送新操作。"); try { sessionStorage.setItem(this.key, JSON.stringify(value)); } catch { throw new Error("浏览器不能可靠保留原件和完整原请求，未发送资料写入；原件 File 与输入仍在本页。请释放此站点缓存容量后重试。 "); } }
  private keep(): void { try { this.persist(); } catch (e) { this.message.textContent = (e as Error).message; } }
  private inputChanged(): void { this.draft.revision++; this.inspected = undefined; this.reviewedRevision = undefined; this.acceptedLosses = false; this.members = undefined; this.keep(); this.render(); }
  private async chooseFile(file: File): Promise<void> {
    const epoch = this.epoch; this.original = file; this.reading = true; this.draft.fileName = file.name; this.draft.bytes = file.size; this.draft.bytesBase64 = undefined; this.inputChanged();
    try { if (file.size > 16 * 1024 * 1024) throw new Error("原件超过 16 MiB 上限；File 保留在本页，未发送导入。"); const bytes = new Uint8Array(await file.arrayBuffer()); if (epoch !== this.epoch || this.original !== file) return; let binary = ""; for (let i = 0; i < bytes.length; i += 32768) binary += String.fromCharCode(...bytes.subarray(i, i + 32768)); this.draft.bytesBase64 = btoa(binary); this.draft.bytes = bytes.length; this.inputChanged(); this.message.textContent = "原件已完整读取，尚未准备或提交资料。"; }
    catch (e) { if (epoch === this.epoch && this.original === file) this.message.textContent = (e as Error).message; }
    finally { if (epoch === this.epoch && this.original === file) { this.reading = false; this.render(); } }
  }
  private async run(work: () => Promise<void>): Promise<void> { if (this.busy) return; const epoch = this.epoch; this.busy = true; this.render(); try { await work(); } catch (e) { if (epoch === this.epoch) { this.verified = false; this.reviewedRevision = undefined; this.members = undefined; this.message.textContent = (e as Error).message; } } finally { if (epoch === this.epoch) { this.busy = false; this.render(); } } }
  private async refresh(): Promise<void> {
    const epoch = this.epoch, identity = this.owner.identity(); const value = await this.request<{ scope: string; targets: ContentTarget[] }>("import"); if (!await this.authority(identity) || epoch !== this.epoch) return;
    if (value.scope !== "configuredImportSlots" || !Array.isArray(value.targets) || value.targets.some(t => !t || !t.id || typeof t.title !== "string" || typeof t.available !== "boolean" || !Array.isArray(t.formats) || t.formats.some(f => !contentFormats.includes(f)))) throw new Error("固定槽位目录无法确认，未采用替代目标。");
    this.members = undefined; this.targets = value.targets; this.render();
  }
  private async inspect(): Promise<void> {
    const revision = this.draft.revision, epoch = this.epoch, identity = this.owner.identity(), target = this.targets.find(t => t.id === this.draft.slot), bytesBase64 = this.draft.bytesBase64;
    if (!target?.formats.includes(this.draft.format) || !target.available || typeof bytesBase64 !== "string" || this.reading) throw new Error("请选择获权固定空槽、格式与完整原件。");
    this.persist(); const body = { slot: this.draft.slot, format: this.draft.format, bytesBase64 }, digest = await sha(contentBytes(bytesBase64));
    if (epoch !== this.epoch || revision !== this.draft.revision || !sameHostedLinksIdentity(identity!, this.owner.identity())) return;
    const value = await this.request<ContentInspection>("import/inspect", body); if (!await this.authority(identity) || epoch !== this.epoch || revision !== this.draft.revision) return;
    this.inspected = { revision, value: contentInspection(value, digest) }; this.message.textContent = value.canPrepare ? "本次原件可新增；请核对检查结果，再准备原计划。" : value.reason ?? "原件含冲突或未映射 typed 引用；不能直接新增，原件未改动。";
  }
  private accept(value: ContentReceipt): void { const job = this.draft.job; if (!job) return; job.receipt = contentReceipt(job.request, value, job.receipt); this.members = undefined; job.phase = undefined; this.verified = true; this.reviewedRevision = undefined; this.acceptedLosses = false; this.keep(); this.message.textContent = value.status === "committed" ? "原资料导入已提交；正文 Draft 与 Undo 保留，新成员仍须独立读取授权后明确打开。" : value.status === "prepared" ? "原上传计划已准备，请核对原编号、原文件与损失，再明确提交。" : "原操作事实已核对，未把结束、未登记或恢复中冒作保存成功。"; }
  private async prepare(): Promise<void> {
    const request = freezeContentImport(this.draft, this.targets.find(t => t.id === this.draft.slot), this.inspected, crypto.randomUUID()), epoch = this.epoch, identity = this.owner.identity();
    const next: ContentDraft = { ...this.draft, job: { request, fileName: this.draft.fileName!, revision: this.draft.revision, phase: "prepare" } }; this.persist(next); this.draft = next; this.verified = false;
    const value = await this.request<ContentReceipt>("import/prepare", request); if (await this.authority(identity) && epoch === this.epoch) this.accept(value);
  }
  private async lookup(): Promise<void> { const job = this.draft.job; if (!job) return; const epoch = this.epoch, identity = this.owner.identity(); const value = await this.request<ContentReceipt>("import/operation", { slot: job.request.slot, operationId: job.request.operationId }); if (await this.authority(identity) && epoch === this.epoch) { this.accept(value); if (value.status === "committed") await this.refresh(); } }
  private async retry(): Promise<void> { const job = this.draft.job; if (!job) return; this.persist(); const epoch = this.epoch, identity = this.owner.identity(); const value = await this.request<ContentReceipt>("import/prepare", job.request); if (await this.authority(identity) && epoch === this.epoch) { this.accept(value); if (value.status === "committed") await this.refresh(); } }
  private async commit(): Promise<void> {
    const job = this.draft.job; if (!job || !canCommitContent(this.draft, this.verified, this.reviewedRevision) || !this.acceptedLosses) throw new Error("请明确核对原上传及损失，未提交旧计划。");
    job.commit ??= { slot: job.request.slot, operationId: job.request.operationId, planId: job.receipt!.planId!, acceptLosses: true }; job.phase = "commit"; this.persist(); const epoch = this.epoch, identity = this.owner.identity();
    const value = await this.request<ContentReceipt>("import/commit", job.commit); if (await this.authority(identity) && epoch === this.epoch) { this.accept(value); if (value.status === "committed") await this.refresh(); }
  }
  private async settle(): Promise<void> { const job = this.draft.job; if (!job) return; job.phase = "settle"; this.persist(); const epoch = this.epoch, identity = this.owner.identity(); const value = await this.request<ContentReceipt>("import/settle", { slot: job.request.slot, operationId: job.request.operationId }); if (await this.authority(identity) && epoch === this.epoch) this.accept(value); }
  private async loadMembers(): Promise<void> { const job = this.draft.job; if (!job) return; const epoch = this.epoch, identity = this.owner.identity(); const value = await this.request<ContentMembers>("import/members", { slot: job.request.slot }); if (!await this.authority(identity) || epoch !== this.epoch) return; if (value.scope !== "configuredImportSlotMembers" || !["complete", "partial"].includes(value.coverage) || !Array.isArray(value.members) || value.members.some(m => !m.resource.startsWith(job.request.slot + "/") || typeof m.canOpen !== "boolean" || !["ready", "unavailable"].includes(m.status))) throw new Error("成员范围无法确认，未打开其他对象。"); this.members = value; }
  private render(): void {
    if (!this.key) return; this.host.hidden = false; this.slot.replaceChildren(); const selected = this.draft.slot, options = [["", "明确选择获权固定空槽"], ...this.targets.map(t => [t.id, `${t.title}${t.available ? "" : "（目标已存在或当前不可新增）"}`])]; if (selected && !this.targets.some(t => t.id === selected)) options.push([selected, "原槽位（当前不可用，输入保留）"]); for (const [id, title] of options) { const option = ui("option", "", title); option.value = id; this.slot.append(option); } this.slot.value = selected;
    this.format.replaceChildren(...contentFormats.map(f => { const option = ui("option", "", labels[f]); option.value = f; return option; })); this.format.value = this.draft.format;
    this.slot.disabled = this.format.disabled = this.file.disabled = !!this.unreadable;
    this.details.textContent = this.draft.fileName ? `当前原件：${this.draft.fileName} · ${this.draft.bytes ?? "未知"} 字节${this.reading ? " · 正在读取" : ""}` : "请选择原件，最多 16 MiB。原生资料包不包含完整恢复所需的私稿和原操作。";
    const target = this.targets.find(t => t.id === selected); this.inspectButton.disabled = this.busy || this.reading || !!this.unreadable || !target?.available || !target.formats.includes(this.draft.format) || typeof this.draft.bytesBase64 !== "string"; this.prepareButton.disabled = this.inspectButton.disabled || !!this.draft.job || this.inspected?.revision !== this.draft.revision || !this.inspected?.value.canPrepare;
    this.inspection.replaceChildren(); if (this.unreadable) this.inspection.append(ui("p", "hint", this.unreadable), action("明确放弃损坏的资料导入恢复记录", () => { void this.run(async () => { sessionStorage.removeItem(this.key); this.unreadable = undefined; this.draft = initial(); this.file.value = ""; this.original = undefined; this.inspected = undefined; this.verified = false; }); }));
    const inspected = this.inspected?.value; if (inspected) { this.inspection.append(ui("h3", "", "准确原件检查"), ui("p", "", `SHA-256：${inspected.uploadDigest} · ${inspected.recordCount ?? inspected.records?.length ?? "未知"} 条记录 · ${inspected.canPrepare ? "可准备新增" : "不能准备"}`)); if (inspected.reason) this.inspection.append(ui("p", "hint", inspected.reason)); this.inspection.append(ui("pre", "", JSON.stringify({ records: inspected.records, losses: inspected.losses }, null, 2))); }
    this.records.replaceChildren(); this.memberView.replaceChildren(); const job = this.draft.job; if (!job) return;
    const receipt = this.verified ? job.receipt : undefined, card = ui("section", "content-import-pending"); card.append(ui("h3", "", "原资料导入操作"), ui("code", "", job.request.operationId), ui("p", "", `${job.request.slot} · ${labels[job.request.format]} · ${job.fileName} · ${contentBytes(job.request.bytesBase64).length} 字节`), ui("p", "", `状态：${receipt?.status ?? "原编号保留，尚待当前授权核对"}`));
    if (job.revision !== this.draft.revision) card.append(ui("p", "hint", "后来输入或同值往返已保留。确认仅提交下方原上传计划，不使用当前其他原件；新输入不会被回执覆盖。"));
    if (receipt) { if (receipt.preview.summary) card.append(ui("p", "", receipt.preview.summary)); if (receipt.preview.recordCount !== undefined) card.append(ui("p", "", `计划记录数：${receipt.preview.recordCount}`)); if (receipt.preview.losses) card.append(ui("pre", "", JSON.stringify(receipt.preview.losses, null, 2))); }
    const consent = ui("input"); consent.type = "checkbox"; consent.setAttribute("aria-label", "已核对原上传计划并接受导入损失"); consent.checked = this.reviewedRevision === this.draft.revision && this.acceptedLosses; consent.disabled = this.busy || !receipt?.commitAllowed; consent.onchange = () => { this.reviewedRevision = consent.checked ? this.draft.revision : undefined; this.acceptedLosses = consent.checked; this.render(); }; const consentLabel = ui("label", "", "已核对原上传计划并接受明确列出的损失；后来输入继续保留"); consentLabel.prepend(consent); card.append(consentLabel);
    const commit = action("明确提交原资料导入计划", () => { void this.run(() => this.commit()); }); commit.disabled = this.busy || !canCommitContent(this.draft, this.verified, this.reviewedRevision) || !this.acceptedLosses;
    const lookup = action("查询原资料导入操作", () => { void this.run(() => this.lookup()); }); lookup.disabled = this.busy;
    const retry = action("重试完整原资料请求", () => { void this.run(() => this.retry()); }); retry.disabled = this.busy || contentEnded(receipt?.status);
    const settle = action("结束原未安装资料导入", () => { void this.run(() => this.settle()); }); settle.disabled = this.busy || contentEnded(receipt?.status); card.append(commit, lookup, retry, settle);
    if (receipt?.status === "committed") { card.append(ui("p", "hint", "原提交事实保留，节点定位符不隐含读取权。"), ui("pre", "", JSON.stringify(receipt.nodeRefs, null, 2)), action("查看独立获权的导入成员", () => { void this.run(() => this.loadMembers()); })); }
    const fresh = action("结束本页已完成导入回执，保留当前原件", () => { void this.run(async () => { if (!contentEnded(this.verified ? this.draft.job?.receipt?.status : undefined)) return; const next = { ...this.draft, job: undefined }; this.persist(next); this.draft = next; this.verified = false; this.reviewedRevision = undefined; this.members = undefined; }); }); fresh.disabled = this.busy || !contentEnded(receipt?.status); card.append(fresh); this.records.append(card);
    if (this.members) { this.memberView.append(ui("h3", "", "独立获权的导入成员"), ui("p", "hint", this.members.coverage === "partial" ? "成员部分可用，不可用对象不猜路径。" : "仅列本槽获权成员；显式打开仍会再次鉴权。")); for (const member of this.members.members) { const button = action(`打开导入成员：${member.title ?? member.resource}`, () => { void this.run(async () => { const epoch = this.epoch, identity = this.owner.identity(); if (member.canOpen && member.status === "ready" && await this.authority(identity) && epoch === this.epoch) await this.owner.open(member.resource); }); }); button.disabled = this.busy || !member.canOpen || member.status !== "ready"; this.memberView.append(button); } }
  }
}
