import { action, ui } from "./data-panel";
import { hostedCachePrefix } from "./hosted-state";
import { sameHostedLinksIdentity, type HostedLinksIdentity } from "./hosted-knowledge-links-state";
import { backupChunk, backupEnded, backupReceipt, canCommitBackup, canDownloadBackup, decodeBackupJob, type BackupCatalog, type BackupChunk, type BackupJob, type BackupReceipt } from "./hosted-owner-backup-state";
type Request = <T>(path: string, body?: unknown) => Promise<T>;
export interface HostedOwnerBackupHost { identity(): HostedLinksIdentity | null; authorizationChanged(message: string, current: { actor: string; deployment: string; generation: number }): void }
/** Recovery material delivery is separate from native content export. */
export class HostedOwnerBackup {
  private epoch = 0;
  private key = "";
  private job?: BackupJob;
  private catalog?: BackupCatalog;
  private busy = false;
  private verified = false;
  private unreadable?: string;
  private reviewed = false;
  private message = ui("p", "hint");
  private panel = ui("section");
  private url?: string;
  constructor(private host: HTMLElement, private request: Request, private owner: HostedOwnerBackupHost) { this.clear(false); }
  clear(removeCache = true): void {
    this.epoch++; this.key = ""; this.job = undefined; this.catalog = undefined; this.busy = this.verified = this.reviewed = false; this.unreadable = undefined; this.message.textContent = ""; this.panel.replaceChildren(); this.host.replaceChildren(); this.host.hidden = true; this.releaseUrl();
    if (removeCache) for (const key of Object.keys(sessionStorage)) if (key.startsWith("weftext-hosted-owner-backup:")) sessionStorage.removeItem(key);
  }
  private releaseUrl(): void { if (this.url) URL.revokeObjectURL(this.url); this.url = undefined; }
  async connect(deployment: string, actor: string, generation: number): Promise<void> {
    this.clear(false); const epoch = this.epoch, identity = this.owner.identity(); if (!identity || identity.actor !== actor || identity.deployment !== deployment || identity.generation !== generation) return;
    this.key = hostedCachePrefix("owner-backup", deployment, actor, generation) + "state";
    try { const raw = sessionStorage.getItem(this.key); if (raw) this.job = decodeBackupJob(raw); } catch { this.unreadable = "完整备份恢复记录无法解析；原记录保留，生成与下载暂停。"; }
    this.render(); await this.refresh(); if (epoch === this.epoch && this.job && !this.unreadable) await this.run(() => this.lookup());
  }
  private async authority(identity: HostedLinksIdentity | null): Promise<boolean> {
    if (!identity || !sameHostedLinksIdentity(identity, this.owner.identity())) return false;
    const current = await this.request<{ actor: string; deployment: string; generation: number }>("me"); if (!sameHostedLinksIdentity(identity, this.owner.identity())) return false;
    if (current.actor !== identity.actor || current.deployment !== identity.deployment || current.generation !== identity.generation) { this.owner.authorizationChanged("授权策略已变化，旧完整备份展示已清除，请重新连接。", current); return false; } return true;
  }
  private persist(value: BackupJob | null | undefined = this.job): void { if (!this.key || this.unreadable) throw new Error("备份恢复记录不可写，未发送操作。"); try { if (value) sessionStorage.setItem(this.key, JSON.stringify(value)); else sessionStorage.removeItem(this.key); } catch { throw new Error("浏览器不能可靠保留原备份请求；原编号保留，未发送新的生成或确认操作。"); } }
  private keep(): void { try { this.persist(); } catch (e) { this.message.textContent = (e as Error).message; } }
  private async run(work: () => Promise<void>): Promise<void> { if (this.busy) return; const epoch = this.epoch; this.busy = true; this.render(); try { await work(); } catch (e) { if (epoch === this.epoch) { this.verified = this.reviewed = false; this.releaseUrl(); this.message.textContent = (e as Error).message; } } finally { if (epoch === this.epoch) { this.busy = false; this.render(); } } }
  private async refresh(): Promise<void> {
    const epoch = this.epoch, identity = this.owner.identity(); const value = await this.request<BackupCatalog>("owner-backup"); if (!await this.authority(identity) || epoch !== this.epoch) return;
    if (!value || typeof value.enabled !== "boolean" || value.scope !== "completeSelectedLibraryRecoveryMaterials" || value.automaticRestore !== false || value.rights && [value.rights.read, value.rights.export, value.rights.download].some(v => typeof v !== "boolean")) throw new Error("完整恢复备份资格无法确认，未借原生导出权限生成备份。"); this.catalog = value; this.render();
  }
  private accept(value: BackupReceipt): void { if (!this.job) return; this.job.receipt = backupReceipt(this.job.request, value, this.job.receipt); this.job.phase = undefined; this.verified = true; this.reviewed = false; this.keep(); this.message.textContent = value.status === "committed" ? value.artifact ? "原完整备份已生成；可按独立下载权限交付同一工件。" : "原完整备份确已提交，但本次交付工件已过期、关闭或重启丢失；不能重执行原编号，请保留原操作事实。" : value.status === "prepared" ? "原完整备份计划已准备，请核对范围后明确生成。" : "原请求事实已核对；结束或未登记不代表已生成备份。"; }
  private async prepare(): Promise<void> {
    if (this.job || !this.catalog?.enabled || !this.catalog.rights?.export) throw new Error("当前没有独立完整备份生成权限，或原请求仍待核对。");
    const job: BackupJob = { request: { requestId: crypto.randomUUID() }, phase: "prepare" }; this.persist(job); this.job = job; this.verified = false; const epoch = this.epoch, identity = this.owner.identity(); const value = await this.request<BackupReceipt>("owner-backup/prepare", job.request); if (await this.authority(identity) && epoch === this.epoch) this.accept(value);
  }
  private async lookup(): Promise<void> { const job = this.job; if (!job) return; const epoch = this.epoch, identity = this.owner.identity(); const value = await this.request<BackupReceipt>("owner-backup/status", job.request); if (await this.authority(identity) && epoch === this.epoch) this.accept(value); }
  private async retry(): Promise<void> { const job = this.job; if (!job) return; this.persist(); const epoch = this.epoch, identity = this.owner.identity(); const value = await this.request<BackupReceipt>("owner-backup/prepare", job.request); if (await this.authority(identity) && epoch === this.epoch) this.accept(value); }
  private async commit(): Promise<void> {
    const job = this.job; if (!job || !this.reviewed || !canCommitBackup(job, this.verified, this.catalog)) throw new Error("请核对原完整备份计划，未生成旧缓存计划。");
    job.commit ??= { requestId: job.request.requestId, planId: job.receipt!.planId! }; job.phase = "commit"; this.persist(); const epoch = this.epoch, identity = this.owner.identity(); const value = await this.request<BackupReceipt>("owner-backup/commit", job.commit); if (await this.authority(identity) && epoch === this.epoch) this.accept(value);
  }
  private async settle(): Promise<void> { const job = this.job; if (!job) return; job.phase = "settle"; this.persist(); const epoch = this.epoch, identity = this.owner.identity(); const value = await this.request<BackupReceipt>("owner-backup/settle", job.request); if (await this.authority(identity) && epoch === this.epoch) this.accept(value); }
  private async close(): Promise<void> {
    const job = this.job; if (!job || !this.verified || job.receipt?.status !== "committed") return; job.phase = "close"; this.persist(); const epoch = this.epoch, identity = this.owner.identity();
    const value = await this.request<{ requestId: string; delivery: string; originalOperationRetained: boolean }>("owner-backup/close", job.request); if (!await this.authority(identity) || epoch !== this.epoch) return; if (value.requestId !== job.request.requestId || value.delivery !== "closed" || value.originalOperationRetained !== true) throw new Error("关闭回执无法确认，原备份事实保留。"); this.releaseUrl(); await this.lookup();
  }
  private async download(): Promise<void> {
    const job = this.job; if (!job || !canDownloadBackup(job, this.verified, this.catalog)) throw new Error("原工件尚未按当前授权核对，或没有独立下载权限。");
    const epoch = this.epoch, identity = this.owner.identity(), artifact = structuredClone(job.receipt!.artifact!), chunks: Uint8Array<ArrayBuffer>[] = []; this.persist(); this.releaseUrl(); let offset = 0;
    do { const value = await this.request<BackupChunk>("owner-backup/download", { requestId: job.request.requestId, digest: artifact.digest, offset }); if (!await this.authority(identity) || epoch !== this.epoch || this.job !== job) return; const bytes = backupChunk(job.request, artifact, offset, value); chunks.push(bytes as Uint8Array<ArrayBuffer>); offset = value.nextOffset; if (value.done) break; } while (offset < artifact.length);
    const blob = new Blob(chunks, { type: artifact.contentType }); if (blob.size !== artifact.length) throw new Error("备份下载长度不完整，未交付文件。"); const digest = [...new Uint8Array(await crypto.subtle.digest("SHA-256", await blob.arrayBuffer()))].map(v => v.toString(16).padStart(2, "0")).join(""); if (digest !== artifact.digest) throw new Error("备份完整摘要不匹配，未交付文件。");
    if (!await this.authority(identity) || epoch !== this.epoch || this.job !== job) return; this.url = URL.createObjectURL(blob); const link = ui("a"); link.href = this.url; link.download = artifact.filename; document.body.append(link); link.click(); link.remove(); this.releaseUrl(); this.message.textContent = `同一完整备份已按 ${blob.size} 字节与 SHA-256 校验交付；不提供自动恢复。`;
  }
  private render(): void {
    if (!this.key) return; this.host.hidden = !this.catalog?.enabled && !this.job && !this.unreadable; if (this.host.hidden) return;
    this.host.replaceChildren(ui("h2", "", "完整恢复资料备份"), ui("p", "hint", "包括所选库的原始文件、忽略项、回收内容、所有人的私有草稿和原操作记录。它需要管理员独立授权，区别于原生资料包；不提供自动跨设备恢复。生成与下载均不保存、替换当前正文。"), this.message, this.panel); this.panel.replaceChildren();
    if (this.unreadable) { this.panel.append(ui("p", "hint", this.unreadable), action("明确放弃损坏的完整备份恢复记录", () => { void this.run(async () => { sessionStorage.removeItem(this.key); this.unreadable = undefined; this.job = undefined; }); })); return; }
    const prepare = action("准备完整恢复资料备份", () => { void this.run(() => this.prepare()); }); prepare.disabled = this.busy || !!this.job || !this.catalog?.rights?.export; const refresh = action("刷新独立备份资格", () => { void this.run(() => this.refresh()); }); refresh.disabled = this.busy; this.panel.append(prepare, refresh);
    const job = this.job; if (!job) return; const receipt = this.verified ? job.receipt : undefined, card = ui("section", "owner-backup-pending"); card.append(ui("h3", "", "原完整备份请求"), ui("code", "", job.request.requestId), ui("p", "", `状态：${receipt?.status ?? "原编号保留，尚待当前授权核对"}`)); if (receipt?.summary) card.append(ui("p", "", receipt.summary));
    if (receipt?.status === "committed" && !receipt.artifact) card.append(ui("p", "hint", "原提交已完成，本次交付工件不可用（过期、关闭或重启）；不能自动生成替代备份。"));
    const consent = ui("input"); consent.type = "checkbox"; consent.setAttribute("aria-label", "确认完整备份包含所有人的私稿与原操作"); consent.checked = this.reviewed; consent.disabled = this.busy || !canCommitBackup(job, this.verified, this.catalog); consent.onchange = () => { this.reviewed = consent.checked; this.render(); }; const label = ui("label", "", "已核对原计划；确认完整备份包括所有人的私稿与原操作，不是原生资料包"); label.prepend(consent); card.append(label);
    const commit = action("明确生成原完整备份计划", () => { void this.run(() => this.commit()); }); commit.disabled = this.busy || !this.reviewed || !canCommitBackup(job, this.verified, this.catalog);
    const lookup = action("查询原完整备份请求", () => { void this.run(() => this.lookup()); }); lookup.disabled = this.busy;
    const retry = action("重试原完整备份准备请求", () => { void this.run(() => this.retry()); }); retry.disabled = this.busy || backupEnded(receipt?.status);
    const settle = action("结束原未安装备份计划", () => { void this.run(() => this.settle()); }); settle.disabled = this.busy || backupEnded(receipt?.status) || !this.catalog?.rights?.export; card.append(commit, lookup, retry, settle);
    if (receipt?.artifact) card.append(ui("p", "", `${receipt.artifact.filename} · ${receipt.artifact.length} 字节`), ui("code", "", receipt.artifact.digest));
    const download = action("下载原完整恢复备份", () => { void this.run(() => this.download()); }); download.disabled = this.busy || !canDownloadBackup(job, this.verified, this.catalog); const close = action("关闭本次备份交付工件", () => { void this.run(() => this.close()); }); close.disabled = this.busy || receipt?.status !== "committed" || !receipt.artifact; card.append(download, close);
    const fresh = action("明确结束本页已完成备份回执", () => { void this.run(async () => { if (!backupEnded(this.verified ? this.job?.receipt?.status : undefined)) return; this.persist(null); this.job = undefined; this.verified = this.reviewed = false; this.releaseUrl(); }); }); fresh.disabled = this.busy || !backupEnded(receipt?.status); card.append(fresh); this.panel.append(card);
  }
}
