import type { AuthorInputFactory, AuthorInputSession } from "./author-input";
import { action, ui } from "./data-panel";
import { parseQueryPlan, type QueryPlan } from "./query-panel";
import { hostedCachePrefix } from "./hosted-state";
import type { SourcePatch } from "./editor";
import type { EmbeddedQueryBasis } from "./embedded-query";
import { retainDefinitionReceipt, sameDefinitionBasis, validateDefinitionPatches, validatePreparedDefinition, type DefinitionAuthorContext, type DefinitionPrepareRequest, type DefinitionPrepared, type DefinitionProposal, type DefinitionReceipt, type DefinitionRecord } from "./hosted-query-definition-state";

type Request = <T>(path: string, body: unknown) => Promise<T>;
export interface HostedDefinitionHost {
  authorInputs?: AuthorInputFactory;
  context(): DefinitionAuthorContext | null;
  prepareBasis(): Promise<EmbeddedQueryBasis>;
  apply(patches: SourcePatch[], basis: EmbeddedQueryBasis): Promise<void>;
  reservations(resource: string, receipts: DefinitionReceipt[], includeReserved?: boolean): void;
}
const digest = async (source: string): Promise<string> => [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(source)))].map(value => value.toString(16).padStart(2, "0")).join("");
const statusText: Record<DefinitionReceipt["status"], string> = { reserved: "已预约 · 仅草稿，尚未保存定义", admitted: "已由原保存接纳", cancelled: "原预约已明确作废", recoveryPending: "原保存仍需恢复，不能再次分配", notRegistered: "原编号未登记，可重试相同请求或明确作废" };

/** The proposal cache retains recovery identity; the original source remains the only author buffer. */
export class HostedQueryDefinitions {
  readonly element = ui("section");
  private epoch = 0;
  private key = "";
  private label = "weftext-query-new";
  private records: DefinitionRecord[] = [];
  private selected = "";
  private busy = false;
  private ownerSession = "";
  private inputOwner = "";
  private inputSession?: AuthorInputSession;
  private ownerGeneration = 0;
  private operationGeneration = new Map<string, number>();
  private message = ui("p", "hint");
  private controls = ui("section");
  private preview = ui("section");
  private unreadable?: string;
  constructor(private request: Request, private author: HostedDefinitionHost, private plan: () => string, private eligible: (resource: string) => boolean) { this.element.append(ui("h3", "", "新建保存定义 · 正文草稿提案"), this.message, this.controls, this.preview); }
  clear() { this.inputSession?.retire(); this.inputSession = undefined; this.inputOwner = ""; this.epoch++; this.ownerGeneration++; this.operationGeneration.clear(); this.key = ""; this.records = []; this.selected = ""; this.ownerSession = ""; this.busy = false; this.unreadable = undefined; this.label = "weftext-query-new"; this.controls.replaceChildren(); this.preview.replaceChildren(); this.message.textContent = ""; }
  async connect(deployment: string, actor: string, generation: number) {
    this.clear(); this.key = hostedCachePrefix("query", deployment, actor, generation) + "definition-proposals";
    let raw: string | null = null;
    try { raw = sessionStorage.getItem(this.key); if (raw) { const value = JSON.parse(raw) as { label: string; records: DefinitionRecord[] }; if (typeof value.label !== "string" || !Array.isArray(value.records) || value.records.some(item => !item.operationId || !item.resource || item.request && (item.request.operationId !== item.operationId || item.request.resource !== item.resource))) throw new Error(); this.label = value.label; this.records = value.records.map(item => ({ ...item, receipt: undefined })); this.selected = this.records[0]?.operationId ?? ""; } } catch { this.unreadable = raw ?? ""; }
    this.render();
    const epoch = this.epoch;
    for (const record of [...this.records]) { try { await this.lookup(record); } catch (error) { if (epoch === this.epoch) this.message.textContent = error instanceof Error ? error.message : String(error); } if (epoch !== this.epoch) return; }
    await this.authorChanged();
  }
  private persist(records = this.records, beforePrepare = false) { if (!this.key || this.unreadable !== undefined) throw new Error("定义恢复缓存尚不可用，请先保护原提案。"); try { sessionStorage.setItem(this.key, JSON.stringify({ label: this.label, records })); } catch { throw new Error(beforePrepare ? "浏览器无法保留原定义提案编号和准确请求，未发起新的预约；请先复制输入。" : "浏览器无法保留原定义提案编号和准确请求；原操作可能已登记，请保留本页并查询原编号，正文保存状态仍以原保存回执为准。"); } }
  async authorChanged(force = false) {
    this.renderControls(); this.renderPreview();
    const context = this.author.context(), ownerSession = context && this.eligible(context.resource) ? `${context.resource}:${context.sessionId}` : "";
    if (!ownerSession || !force && ownerSession === this.ownerSession) return;
    this.ownerSession = ownerSession; const epoch = this.epoch, generation = ++this.ownerGeneration;
    try {
      const response = await this.request<{ resource: string; reservations: DefinitionReceipt[] }>("query/definition/reservations", { sessionId: context!.sessionId });
      if (epoch !== this.epoch || generation !== this.ownerGeneration || this.ownerSession !== ownerSession || this.author.context()?.sessionId !== context!.sessionId || this.author.context()?.baseId !== context!.baseId) return;
      if (response.resource !== context!.resource || !Array.isArray(response.reservations)) throw new Error("定义预约列表与当前正文不一致，未借用另一对象的编号。");
      for (const receipt of response.reservations) {
        const existing = this.records.find(item => item.operationId === receipt.operationId);
        if (existing) Object.assign(existing, retainDefinitionReceipt(existing, receipt));
        else this.records.push(retainDefinitionReceipt({ operationId: receipt.operationId, resource: response.resource }, receipt));
      }
      // This list intentionally contains only pending reservations. Its absence
      // is not evidence of admission; query the exact IDs after every save/open.
      this.author.reservations(response.resource, response.reservations); this.persist(); if (!this.selected) this.selected = this.records.find(item => item.resource === response.resource)?.operationId ?? ""; this.render();
    } catch (error) { if (epoch === this.epoch && this.ownerSession === ownerSession) this.message.textContent = error instanceof Error ? error.message : String(error); }
    if (force) {
      const ids = new Set([...(context!.definitionReservations ?? []), ...this.records.filter(item => item.resource === context!.resource && (item.request || item.applied) && (!item.receipt || ["reserved", "recoveryPending", "notRegistered"].includes(item.receipt.status))).map(item => item.operationId)]);
      for (const id of ids) {
        if (epoch !== this.epoch || generation !== this.ownerGeneration || this.author.context()?.sessionId !== context!.sessionId) return;
        let record = this.records.find(item => item.operationId === id);
        if (!record) { record = { operationId: id, resource: context!.resource }; this.records.push(record); }
        try { await this.lookup(record); } catch (error) { if (epoch === this.epoch && generation === this.ownerGeneration) this.message.textContent = error instanceof Error ? error.message : String(error); }
      }
    }
  }
  private render() { this.renderControls(); this.renderPreview(); }
  private unknown(resource: string) { return this.records.some(record => record.resource === resource && (!record.receipt || ["notRegistered", "recoveryPending"].includes(record.receipt.status))); }
  private renderControls() {
    const active = document.activeElement, focused = active instanceof HTMLInputElement && active.getAttribute("aria-label") === "新保存定义原生标签", selection = focused ? { from: active.selectionStart, to: active.selectionEnd } : null;
    this.controls.replaceChildren();
    if (this.unreadable !== undefined) { this.controls.append(ui("p", "hint", "原定义提案缓存无法解析，保留原记录；尚未分配新编号。"), ui("pre", "", this.unreadable)); return; }
    const context = this.author.context(), label = ui("input", "data-input"); label.setAttribute("aria-label", "新保存定义原生标签"); label.value = this.label; const inputOwner = context ? JSON.stringify([context.resource, context.sessionId]) : "";
    if (inputOwner !== this.inputOwner) { this.inputSession?.retire(); this.inputSession = undefined; this.inputOwner = inputOwner; }
    if (context && !this.inputSession) { const epoch = this.epoch, selected = context; this.inputSession = this.author.authorInputs?.({ kind: "body", resource: context.resource }, () => this.epoch === epoch && this.author.context()?.sessionId === selected.sessionId && this.author.context()?.resource === selected.resource); }
    this.inputSession?.bind(label, "definition-label"); label.oninput = () => { this.label = label.value; try { this.persist(); } catch (error) { this.message.textContent = String(error instanceof Error ? error.message : error); } };
    const create = action("将临时计划提为当前正文的新保存定义", () => this.prepare()); create.disabled = this.busy || !context?.writable || context.pending || context.compare || !this.eligible(context.resource) || this.unknown(context.resource) || (context.definitionReservations?.length ?? 0) >= 32;
    this.controls.append(ui("p", "hint", context ? `当前定义所有者：${context.resource}；提案只进入此正文的同一份草稿和 Undo。保存按钮才提交。` : "先打开可写正文才能新建定义。原提案可独立查询，无需先打开来源。"), label, create);
    if (context && this.unknown(context.resource)) this.controls.append(ui("p", "hint", "此正文有原提案尚待核对。请查询、按原请求重试或明确作废，不能分配替代编号。"));
    if (context) this.controls.append(ui("p", "hint", `下次保存关联 ${context.definitionReservations?.length ?? 0} 个已进入此草稿的预约，上限 32；服务端找回的未应用预约只列为恢复入口，不会全部加入保存。`));
    const select = ui("select", "data-input"); select.setAttribute("aria-label", "原保存定义提案"); select.append(Object.assign(ui("option", "", "选择原提案编号"), { value: "" }));
    for (const record of this.records) select.append(Object.assign(ui("option", "", `${record.resource} · ${record.operationId}`), { value: record.operationId })); select.value = this.selected; select.onchange = () => { if (select.isConnected === false) return; this.selected = select.value; this.render(); }; this.controls.append(select);
    const record = this.records.find(item => item.operationId === this.selected), check = action("查询原定义提案", () => this.perform(() => this.lookup(record!))), retry = action("按原编号重试相同定义提案", () => this.perform(() => this.send(record!, false))), settle = action("明确作废原定义预约", () => this.perform(async () => { const epoch = this.epoch, generation = this.beginOperation(record!); const receipt = await this.request<DefinitionReceipt>("query/definition/settle", { resource: record!.resource, operationId: record!.operationId }); if (!this.currentOperation(record!, epoch, generation)) return; this.install(record!, receipt); this.message.textContent = statusText[receipt.status]; }));
    check.disabled = this.busy || !record; retry.disabled = this.busy || !record?.request || !!record.receipt && !["notRegistered", "reserved"].includes(record.receipt.status); settle.disabled = this.busy || !record || !!record.receipt && !["notRegistered", "reserved"].includes(record.receipt.status);
    this.controls.append(check, retry, settle, ui("p", "hint", "未知结果按原编号核对；Undo 或删除定义不会作废预约。明确作废后不能重用该编号或 key。"));
    if (focused) { label.focus({ preventScroll: true }); if (selection?.from !== null && selection?.from !== undefined && selection.to !== null) label.setSelectionRange(selection.from, selection.to); }
  }
  private renderPreview() {
    this.preview.replaceChildren(); const record = this.records.find(item => item.operationId === this.selected); if (!record) return;
    this.preview.append(ui("p", "hint", `原提案 ${record.operationId} · ${record.receipt ? statusText[record.receipt.status] : "结果尚未核对；保留原编号，不分配替代 key"}`));
    if (record.receipt?.proposalUnavailable) this.preview.append(ui("p", "hint", "原提案源码当前不可交付或验证；仅保留原编号与操作状态，未按旧缓存恢复补丁。"));
    const proposal = record.receipt?.proposal; if (!proposal || !["reserved", "admitted"].includes(record.receipt!.status)) return;
    this.preview.append(ui("p", "hint", `${proposal.definition.label} · 服务端 key ${proposal.definition.key}`));
    const details = ui("details"); details.append(ui("summary", "", "比较原依据、原提案与当前草稿"), ui("h4", "", "原依据"), ui("pre", "", proposal.basisSource ?? record.request?.source ?? "原依据不可恢复"), ui("h4", "", "原提案源码（保存状态以上方原操作为准）"), ui("pre", "", proposal.source));
    const context = this.author.context(); if (context?.resource === record.resource) details.append(ui("h4", "", "当前个人草稿"), ui("pre", "", context.source)); this.preview.append(details);
    if (!context?.writable || context.resource !== record.resource || context.pending || context.compare || record.receipt!.status !== "reserved") return;
    if (context.source === proposal.source) { const link = action("将原定义预约纳入下次保存", () => this.perform(async () => { const current = this.author.context(); if (!current || current.resource !== record.resource || current.source !== proposal.source || current.pending || current.compare || !current.writable || (current.definitionReservations?.length ?? 0) >= 32) throw new Error("当前草稿、保存状态或预约配额已变化，未借用旧编号。"); this.author.reservations(record.resource, [record.receipt!], true); record.applied = true; this.persist(); this.message.textContent = "原预约已关联当前准确草稿，尚未保存定义。"; })); link.disabled = this.busy || (context.definitionReservations?.length ?? 0) >= 32 || !!context.definitionReservations?.includes(record.operationId); this.preview.append(ui("p", "hint", "当前草稿已含原提案；不会再次插入。关联原预约后点击原保存按钮才提交。"), link); return; }
    const sameSource = context.source === (proposal.basisSource ?? record.request?.source), approval = ui("input"); approval.type = "checkbox"; approval.setAttribute("aria-label", "已比较并接受恢复原定义提案到当前草稿"); const label = ui("label", "data-label", sameSource ? "已比较相同原文，恢复原定义补丁到当前草稿" : "已比较双方稿，允许将当前草稿替换为原提案（可 Undo）"); label.prepend(approval);
    const apply = action(sameSource ? "恢复原定义补丁到当前草稿" : "以原定义提案替换当前草稿（可撤销）", () => this.perform(async () => {
      const current = this.author.context(); if (!approval.checked || !current || current.resource !== context.resource || current.sessionId !== context.sessionId || current.baseId !== context.baseId || current.source !== context.source || current.serial !== context.serial || current.pending || current.compare || !current.writable) throw new Error("比较后草稿或资格已变化，请重新比较；未贴入旧提案。");
      const epoch = this.epoch, basis = await this.author.prepareBasis(); if (epoch !== this.epoch || !this.records.includes(record)) return; if (basis.source !== context.source || basis.sessionId !== context.sessionId || basis.serial !== context.serial) throw new Error("恢复登记期间草稿已变化，未贴入旧提案。");
      const patches: SourcePatch[] = sameSource ? validateDefinitionPatches(context.source, proposal) : [{ from: 0, to: context.source.length, expected: context.source, insert: proposal.source }];
      if ((current.definitionReservations?.length ?? 0) >= 32 && !current.definitionReservations?.includes(record.operationId)) throw new Error("此草稿已关联 32 个预约，请先保存或明确处理原预约，未添加另一编号。");
      this.author.reservations(record.resource, [record.receipt!], true); await this.author.apply(patches, basis); if (epoch !== this.epoch) return; record.applied = true; this.persist(); this.message.textContent = "已显式恢复到同一份正文草稿和 Undo；尚未保存定义。";
    })); apply.disabled = true; approval.onchange = () => apply.disabled = !approval.checked || this.busy; this.preview.append(label, apply);
  }
  private async perform(run: () => Promise<void>) { if (this.busy) return; const epoch = this.epoch; this.busy = true; this.renderControls(); try { await run(); } catch (error) { if (epoch === this.epoch) this.message.textContent = error instanceof Error ? error.message : String(error); } finally { if (epoch === this.epoch) { this.busy = false; this.render(); } } }
  private async prepare() { await this.perform(async () => {
    const epoch = this.epoch, plan: QueryPlan = parseQueryPlan(this.plan()), label = this.label, before = this.author.context(); if (!before?.writable || before.pending || before.compare || !this.eligible(before.resource) || this.unknown(before.resource) || (before.definitionReservations?.length ?? 0) >= 32) throw new Error("当前正文没有新建定义资格、原提案尚待核对或已达 32 个预约，未分配替代编号。");
    const basis = await this.author.prepareBasis(); const context = this.author.context(); if (epoch !== this.epoch || !context || context.resource !== before.resource || context.baseId !== before.baseId || context.sessionId !== basis.sessionId || context.serial !== basis.serial || context.source !== basis.source) throw new Error("登记期间当前正文已变化，未分配编号。");
    const sourceHash = await digest(context.source); if (epoch !== this.epoch || !this.author.context() || this.author.context()!.source !== context.source || this.author.context()!.serial !== context.serial) throw new Error("原文已变化，未发起预约。");
    const operationId = crypto.randomUUID(), request: DefinitionPrepareRequest = { resource: context.resource, sessionId: context.sessionId, baseId: context.baseId, source: context.source, serial: context.serial, sourceHash, operationId, label, plan }, record: DefinitionRecord = { operationId, resource: context.resource, request };
    const records = [...this.records, record]; this.persist(records, true); this.records = records; this.selected = operationId; this.render(); await this.send(record, true);
  }); }
  private async send(record: DefinitionRecord, allowApply: boolean) {
    const epoch = this.epoch, generation = this.beginOperation(record); if (!record.request) throw new Error("没有原请求，只能查询原编号，不能制造新的重试输入。");
    this.persist(); const incoming = await this.request<DefinitionPrepared>("query/definition/prepare", record.request); if (!this.currentOperation(record, epoch, generation)) return;
    if (incoming.status !== "reserved") { this.install(record, { operationId: incoming.operationId, resource: incoming.resource, status: incoming.status, key: incoming.key, binding: incoming.binding, saveOperationId: incoming.saveOperationId }); this.message.textContent = statusText[incoming.status]; return; }
    const proposal = validatePreparedDefinition(record.request, incoming); this.install(record, { operationId: incoming.operationId, resource: incoming.resource, status: incoming.status, key: incoming.key, binding: incoming.binding, saveOperationId: incoming.saveOperationId, proposal });
    if (allowApply && incoming.canApply === true && incoming.recoveryOnly === false && sameDefinitionBasis(this.author.context(), record.request)) {
      this.author.reservations(record.resource, [record.receipt!], true);
      await this.author.apply(validateDefinitionPatches(record.request.source, proposal), record.request); if (!this.currentOperation(record, epoch, generation)) return; record.applied = true; this.persist(); this.message.textContent = "新定义提案已插入当前正文草稿，共用 Undo；尚未保存，原 key 只分配一次。";
    } else this.message.textContent = "原定义提案已核对。当前依据或会话不同，请明确比较后恢复；未自动贴回正文。";
  }
  private install(record: DefinitionRecord, receipt: DefinitionReceipt) { Object.assign(record, retainDefinitionReceipt(record, receipt)); this.author.reservations(record.resource, [receipt], record.applied === true); this.persist(); this.render(); }
  private beginOperation(record: DefinitionRecord) { const generation = (this.operationGeneration.get(record.operationId) ?? 0) + 1; this.operationGeneration.set(record.operationId, generation); return generation; }
  private currentOperation(record: DefinitionRecord, epoch: number, generation: number) { return epoch === this.epoch && this.records.includes(record) && this.operationGeneration.get(record.operationId) === generation; }
  private async lookup(record: DefinitionRecord) { const epoch = this.epoch, generation = this.beginOperation(record), receipt = await this.request<DefinitionReceipt>("query/definition/operation", { resource: record.resource, operationId: record.operationId }); if (!this.currentOperation(record, epoch, generation)) return; this.install(record, receipt); this.message.textContent = statusText[receipt.status]; }
}
