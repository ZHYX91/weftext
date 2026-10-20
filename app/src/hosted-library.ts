import { action, ui } from "./data-panel";
import { nativeStyleControlInput, nativeStyleControlReason, type NativeStyleControl, type NativeStyleValue } from "./style-panel";
import { decodeProcessingDocument, defaultProcessingDocument } from "./processing-settings-panel";
import { parseProcessingLanguages, processingLanguageTag } from "./processing-settings";
import { hostedCachePrefix } from "./hosted-state";
import { nodeSourceHash } from "./hosted-navigation-state";
import { movedSibling } from "./node-contents-state";
import { decodeLibraryDraft, freezeLibraryPrepare, libraryCanCommit, libraryInput, libraryReceipt, libraryRoles, libraryTerminal, type LibraryDraft, type LibraryProfile, type LibraryReceipt, type LibraryRole } from "./hosted-library-state";

type Request = <T>(path: string, body?: unknown) => Promise<T>;
type Catalog = { roles: { role: LibraryRole; title: string; read: boolean; write: boolean }[]; styleUse: boolean; reorderNodes?: boolean; orderParents?: {reference:string;title:string}[] };
interface HostedOrder { parentRef:string;settingsSourceHash:string;orderedRefs:string[];members:{targetRef:string;name:string}[];complete:boolean }
const labelSource = "托管库配置完整源码";
/** Role drafts are proposals; only the original Core operation installs bytes. */
export class HostedLibrary {
  private epoch = 0;
  private prefix = "";
  private catalog: Catalog = { roles: [], styleUse: false };
  private role?: LibraryRole;
  private profile?: LibraryProfile;
  private draft?: LibraryDraft;
  private unreadable?: string;
  private busy = false;
  private pendingRoles = new Set<LibraryRole>();
  private picker = ui("select");
  private editor = ui("section");
  private actions = ui("section");
  private message = ui("p", "hint");
  private source = ui("textarea");
  private enabled = ui("input");
  private comparison?: HTMLButtonElement;
  private inputs: { control: NativeStyleControl; input: HTMLInputElement | HTMLSelectElement }[] = [];
  private orderParent = "";
  private order?: HostedOrder;
  private orderHost = ui("section");
  constructor(private host: HTMLElement, private request: Request, private changed: (role: LibraryRole) => void | Promise<void>) { this.clear(false); }
  get hasPending() { return !!this.draft?.pending || this.pendingRoles.size > 0; }
  clear(remove = true) {
    this.epoch++; this.catalog = { roles: [], styleUse: false }; this.role = undefined; this.profile = undefined; this.draft = undefined; this.unreadable = undefined; this.busy = false; this.pendingRoles.clear(); this.prefix = ""; this.order = undefined; this.orderParent = ""; this.orderHost.replaceChildren();
    this.host.replaceChildren(); this.host.hidden = true; this.editor.replaceChildren(); this.actions.replaceChildren(); this.picker.replaceChildren(); this.message.textContent = ""; this.source.value = ""; this.enabled.checked = false; this.inputs = []; this.comparison = undefined;
    const failures: unknown[] = [];
    if (remove) { try { for (const key of Object.keys(sessionStorage)) if (key.startsWith("weftext-hosted-library:")) { try { sessionStorage.removeItem(key); } catch (error) { failures.push(error); } } } catch (error) { failures.push(error); } }
    if (failures.length) throw new Error("浏览器拒绝删除部分库配置恢复缓存；受保护展示已清除。");
  }
  async connect(deployment: string, actor: string, generation: number) {
    this.clear(false); const epoch = this.epoch, catalog = await this.request<Catalog>("library"); if (epoch !== this.epoch) return;
    if (!Array.isArray(catalog.roles) || catalog.roles.some(item => !libraryRoles.includes(item.role) || typeof item.title !== "string" || typeof item.read !== "boolean" || typeof item.write !== "boolean") || typeof catalog.styleUse !== "boolean") throw new Error("库配置目录缺少准确角色资格。");
    this.catalog = catalog; this.prefix = hostedCachePrefix("library", deployment, actor, generation); this.host.hidden = !catalog.roles.length && !catalog.styleUse;
    for (const item of catalog.roles) { const raw = sessionStorage.getItem(this.prefix + item.role); if (raw) { try { if (decodeLibraryDraft(raw, item.role).pending) this.pendingRoles.add(item.role); } catch { /* The selected role shows the original unreadable record. */ } } }
    this.picker.setAttribute("aria-label", "托管库配置角色"); this.picker.replaceChildren(...catalog.roles.filter(item => item.read).map(item => { const option = ui("option", "", item.title); option.value = item.role; return option; }));
    this.picker.onchange = () => { void this.run(async () => this.select(this.picker.value as LibraryRole)); };
    this.host.replaceChildren(ui("h2", "", "库配置与默认样式"), ui("p", "hint", "配置与正文草稿分别保存。原生源码保留，先准备并核对变更，再明确提交原操作；不会自动保存笔记。"), ui("p", "hint", catalog.styleUse ? "具备独立库样式执行资格；执行资格不交付样式源码。" : "没有独立库样式执行资格。"), this.picker, this.message, this.editor, this.actions);
    const savedRole = sessionStorage.getItem(this.prefix + "selection"), role = catalog.roles.find(item => item.read && item.role === savedRole)?.role ?? catalog.roles.find(item => item.read)?.role;
    if (role) this.select(role); else { this.picker.hidden = true; this.editor.replaceChildren(); this.actions.replaceChildren(); }
  }
  private key() { return this.prefix + this.role; }
  private retain() { if (this.role && this.draft) { sessionStorage.setItem(this.key(), JSON.stringify(this.draft)); if (this.draft.pending) this.pendingRoles.add(this.role); else this.pendingRoles.delete(this.role); } }
  private select(role: LibraryRole) {
    if (!this.catalog.roles.some(item => item.role === role && item.read)) throw new Error("配置角色当前不可读取。");
    try { this.retain(); } catch (error) { if (this.role) this.picker.value = this.role; throw error; }
    this.role = role; this.picker.hidden = false; this.picker.value = role; this.profile = undefined; this.draft = undefined; this.unreadable = undefined; this.editor.replaceChildren(); this.actions.replaceChildren(); this.source.value = ""; this.inputs = []; this.comparison = undefined; this.order=undefined;
    sessionStorage.setItem(this.prefix + "selection", role); const raw = sessionStorage.getItem(this.key());
    if (raw !== null) { try { this.draft = decodeLibraryDraft(raw, role); } catch { this.unreadable = raw; } }
    this.message.textContent = this.draft?.pending ? "原配置操作待核对；无需当前读取依据也可查询原编号。" : "选择角色后读取准确配置。恢复输入不会被读取覆盖。"; this.render();
  }
  private async run(work: () => void | Promise<void>) {
    if (this.busy) return; const epoch = this.epoch; this.busy = true; this.renderActions(); this.picker.disabled = true;
    try { await work(); } catch (error) { if (epoch === this.epoch) this.message.textContent = error instanceof Error ? error.message : "配置操作未完成，原输入与原操作保留。"; }
    finally { if (epoch === this.epoch) { this.busy = false; this.picker.disabled = false; this.renderActions(); if(this.role==="settings")this.renderOrder(); } }
  }
  private async read() {
    const role = this.role, epoch = this.epoch, revision = this.draft?.revision; if (!role || this.unreadable !== undefined) return;
    const profile = await this.request<LibraryProfile>("library/read", { role }); if (epoch !== this.epoch || role !== this.role) return;
    if (profile.role !== role || typeof profile.source !== "string" || !profile.basis || typeof profile.installed !== "boolean") throw new Error("配置读取不对应原角色或准确源码，输入保留。");
    const draft = this.draft;
    this.profile = profile;
    this.order = undefined;
    if (!draft || draft.revision === revision && !draft.pending && draft.source === draft.baseSource && !Object.keys(draft.changes).length && draft.enabled === draft.baseEnabled) this.draft = { source: profile.source, baseSource: profile.source, changes: {}, enabled: profile.enabled, baseEnabled: profile.enabled, revision: (draft?.revision ?? 0) + 1, lastReceipt: draft?.lastReceipt };
    else if (draft.source === profile.source && !Object.keys(draft.changes).length && draft.enabled === profile.enabled && !draft.pending) { draft.baseSource = profile.source; draft.baseEnabled = profile.enabled; }
    this.message.textContent = this.needsComparison() ? "服务端配置依据已变化；原输入保留，请比较双方再确认新依据。" : "已读取准确配置，改动尚未提交。"; this.render();
    try { this.retain(); } catch { throw new Error("准确配置已读取，本页原文可复制；浏览器无法保留恢复缓存，尚未发送写操作。"); }
  }
  private needsComparison() { return !!this.draft && !!this.profile && (this.draft.baseSource !== this.profile.source || this.draft.baseEnabled !== this.profile.enabled); }
  private edited(keepOrder = false) {
    if (!this.draft) return; this.draft.revision++;
    if(this.role==="settings"&&!keepOrder){this.order=undefined;this.renderOrder();}
    try { this.retain(); this.message.textContent = this.draft.pending ? "新配置输入已保留；原操作请求冻结，不能提交为这份新输入。请先核对或结束原操作。" : "配置输入已保留，尚未准备或提交。"; }
    catch { this.message.textContent = "浏览器无法保留配置输入，请先复制；原操作发送前必须成功保留恢复记录。"; }
    const advanced = this.draft.source !== this.draft.baseSource; for (const { control, input } of this.inputs) input.disabled = !this.writable() || !control.editable || advanced;
    this.renderActions();
  }
  private writable() { return this.catalog.roles.some(item => item.role === this.role && item.write); }
  private async prepare() {
    const draft = this.draft, profile = this.profile, role = this.role; if (!draft || !profile || !role || !this.writable()) throw new Error("请先读取当前可写配置。");
    const request = freezeLibraryPrepare(role, profile, draft, crypto.randomUUID());
    draft.pending = { request, input: libraryInput(draft), revision: draft.revision };
    try { this.retain(); } catch (error) { delete draft.pending; throw new Error(`浏览器无法保留原配置操作，尚未发送：${error instanceof Error ? error.message : String(error)}`); }
    this.renderActions(); await this.sendPrepare();
  }
  private async sendPrepare() {
    const draft = this.draft, pending = draft?.pending, epoch = this.epoch; if (!pending) return;
    this.retain(); const result = await this.request<LibraryReceipt>("library/prepare", structuredClone(pending.request)); if (epoch === this.epoch && this.draft === draft) await this.install(result);
  }
  private async operation(settle = false) {
    const draft = this.draft, pending = draft?.pending, epoch = this.epoch; if (!pending) return;
    this.retain(); const result = await this.request<LibraryReceipt>(settle ? "library/settle" : "library/operation", { role: pending.request.role, operationId: pending.request.operationId }); if (epoch === this.epoch && this.draft === draft) await this.install(result);
  }
  private async commit() {
    const draft = this.draft, pending = draft?.pending, epoch = this.epoch; if (!draft || !pending || !libraryCanCommit(draft) || !this.writable() || this.needsComparison()) throw new Error("请核对原准备预览；新输入或当前资格已变化，尚未提交。");
    pending.commit = { role: pending.request.role, operationId: pending.request.operationId, planId: pending.receipt!.planId! }; this.retain();
    const result = await this.request<LibraryReceipt>("library/commit", pending.commit); if (epoch === this.epoch && this.draft === draft) await this.install(result);
  }
  private async install(incoming: LibraryReceipt) {
    const draft = this.draft, pending = draft?.pending; if (!draft || !pending) return;
    const receipt = libraryReceipt(pending.request, incoming, pending.receipt, draft.baseSource); pending.receipt = receipt;
    if (!libraryTerminal(receipt.status)) { this.retain(); this.message.textContent = receipt.status === "prepared" ? "原配置计划已准备，请核对准确原文后明确提交。" : `原配置操作状态：${receipt.status}。未冒作保存成功，请查询原编号或明确结束。`; this.renderActions(); return; }
    const current = pending.revision === draft.revision && JSON.stringify(pending.input) === JSON.stringify(libraryInput(draft));
    if (receipt.status === "committed" && current && typeof receipt.preview?.afterSource === "string") { draft.source = receipt.preview.afterSource; draft.changes = {}; if (receipt.preview.enabled !== undefined) draft.enabled = receipt.preview.enabled; }
    draft.lastReceipt = { role: receipt.role, operationId: receipt.operationId, status: receipt.status }; delete draft.pending; this.profile = undefined; let storageError: unknown; try { this.retain(); } catch (error) { storageError = error; }
    this.message.textContent = receipt.status === "committed" ? "原配置操作已提交；后续输入保留。请重新读取准确依据；正文未保存。" : "原配置操作已结束，未冒作已保存；输入保留，请重新读取准确依据。"; this.render();
    if (receipt.status === "committed") await this.changed(receipt.role);
    if (storageError) throw new Error(`原配置事实 ${receipt.status} 已收到，但浏览器无法更新恢复缓存；请保留原编号核对：${pending.request.operationId}`);
  }
  private render() {
    const focused = document.activeElement === this.source, selection = focused ? [this.source.selectionStart, this.source.selectionEnd] : undefined;
    this.editor.replaceChildren(); this.inputs = []; this.comparison = undefined; const draft = this.draft, profile = this.profile;
    if (this.unreadable !== undefined) { this.editor.append(ui("p", "hint", "恢复记录无法解析，未套用到当前角色；原文保留。"), ui("pre", "", this.unreadable), action("放弃损坏的配置恢复记录", () => this.run(() => { sessionStorage.removeItem(this.key()); this.unreadable = undefined; this.draft = undefined; this.render(); }))); this.renderActions(); return; }
    if (!draft) { this.renderActions(); return; }
    if (profile) { this.editor.append(ui("p", "hint", profile.installed ? "当前配置已安装。" : "此角色尚未安装；准备与提交是明确创建操作。")); for (const warning of profile.warnings ?? []) this.editor.append(ui("p", "hint", warning)); }
    if (this.role === "style") {
      const enabledLabel = ui("label", "", "自动应用库默认样式"); this.enabled.type = "checkbox"; this.enabled.checked = draft.enabled ?? false; this.enabled.disabled = !this.writable(); this.enabled.setAttribute("aria-label", "托管库样式启用"); this.enabled.onchange = () => { draft.enabled = this.enabled.checked; this.edited(); }; enabledLabel.append(this.enabled); this.editor.append(enabledLabel);
      const controls = ui("section"); controls.setAttribute("aria-label", "托管库原生样式控件");
      for (const control of profile?.controls ?? []) {
        const raw = draft.changes[control.key], value: NativeStyleValue = raw === undefined ? control.value : control.kind === "select" ? control.options?.[Number(raw)]?.value ?? control.value : raw;
        const label = ui("label", "", control.label + (control.unit ? `（${control.unit}）` : "")), input = nativeStyleControlInput(control, value); input.setAttribute("aria-label", `托管库样式 ${control.key}`);
        if (raw !== undefined && control.kind !== "boolean" && control.kind !== "select") input.value = String(raw);
        input.disabled = !this.writable() || !control.editable || draft.source !== draft.baseSource; input.oninput = () => { draft.changes[control.key] = control.kind === "boolean" ? (input as HTMLInputElement).checked : input.value; this.edited(); }; label.append(input); if (control.reason) label.append(ui("span", "hint", nativeStyleControlReason(control.reason))); controls.append(label); this.inputs.push({ control, input });
      }
      this.editor.append(controls, ui("p", "hint", "控件修改准确原生参数；完整源码保留未知规则。编辑完整源码时控件暂停，不能同时提交两份输入。"));
      for (const [key, value] of Object.entries(draft.changes)) if (!profile?.controls?.some(control => control.key === key && control.editable)) this.editor.append(ui("p", "hint", `未映射原控件 ${key}：${String(value)}（保留，不能静默提交）`), action(`放弃原控件输入 ${key}`, () => { delete draft.changes[key]; this.edited(); this.render(); }));
      if (Object.keys(draft.changes).length) this.editor.append(action("放弃控件输入，使用完整源码", () => { draft.changes = {}; this.edited(); this.render(); }));
    }
    if (this.role === "settings") {
      this.editor.append(ui("p", "hint", this.catalog.reorderNodes ? "具备独立作者同级重排资格；navigationOrders 的变更按整份库设置预览和提交。个人列表排序不会写入此处。" : "没有独立作者同级重排资格；普通库设置写权不能修改 navigationOrders。"));
      this.editor.append(this.orderHost); this.renderOrder();
      const processing = ui("section"); processing.setAttribute("aria-label", "共享处理默认建议");
      processing.append(ui("h3", "共享处理默认与词表"), ui("p", "hint", "这里是导入／校对的共享建议，只有用户在本次处理明确采用才启用；文档语言仍由原生 Typst 决定。保存复用当前库设置的权限、历史与同步。"));
      try {
        const settings = JSON.parse(draft.source) as Record<string, unknown>;
        if (!settings || Array.isArray(settings) || settings.version !== 1) throw new Error("原设置版本不受支持");
        if (settings.processing === undefined) {
          const create = action("添加共享通用处理建议", () => { settings.processing = defaultProcessingDocument(); draft.source = JSON.stringify(settings, null, 2); this.edited(); this.render(); }); create.disabled = !this.writable(); processing.append(create);
        } else {
          const document = decodeProcessingDocument(settings.processing);
          const label = ui("label", "", "默认建议语言／文字体系／地区（可多选，未知地区省略）"), input = ui("input", "knowledge-input"); input.value = document.defaults.languages.map(processingLanguageTag).join(", "); input.disabled = !this.writable(); input.onchange = () => { try { document.defaults.languages = parseProcessingLanguages(input.value); settings.processing = document; draft.source = JSON.stringify(settings, null, 2); this.edited(); this.render(); } catch (error) { this.message.textContent = error instanceof Error ? error.message : String(error); } }; label.append(input); processing.append(label, ui("p", "hint", `通用方案；${document.rules.length} 条词典规则、${document.scopedAllowlist.length} 条有范围白名单。词条、适用范围及准确版本在下方同一 JSON 中编辑，未知规则保源。`));
        }
      } catch { processing.append(ui("p", "hint", "当前共享处理设置不能由友好控件安全解析，原 JSON 已保留；请在源码中核对，未覆盖未知规则。")); }
      this.editor.append(processing);
    }
    const sourceLabel = ui("label", "", this.role === "style" ? "完整原生 Typst 样式源码" : "完整 JSON 配置源码"); this.source.rows = 14; this.source.spellcheck = false; this.source.setAttribute("aria-label", labelSource); if (this.source.value !== draft.source) this.source.value = draft.source; this.source.readOnly = !this.writable(); this.source.oninput = () => { draft.source = this.source.value; this.edited(); }; sourceLabel.append(this.source); this.editor.append(sourceLabel);
    if (this.needsComparison()) { this.editor.append(ui("h3", "", "配置依据已变化，保留双方原文"), ui("p", "", "原读取依据"), ui("pre", "", draft.baseSource), ui("p", "", "当前服务端配置"), ui("pre", "", profile!.source)); const compare = action("已核对配置，以当前原文作为新依据", () => this.run(() => { if (draft.pending) throw new Error("先核对或结束原操作，再确认新依据。"); draft.baseSource = profile!.source; draft.baseEnabled = profile!.enabled; this.edited(); this.render(); })); this.comparison = compare; this.editor.append(compare); }
    this.renderActions(); if (selection) { this.source.focus(); this.source.setSelectionRange(selection[0], selection[1]); }
  }
  private renderOrder() {
    this.orderHost.replaceChildren(); const draft=this.draft, profile=this.profile;
    if(this.role!=="settings" || !this.catalog.reorderNodes || !draft || !profile)return;
    this.orderHost.setAttribute("aria-label","托管作者同级顺序");this.orderHost.append(ui("h3","","作者同级顺序"),ui("p","hint","仅列独立获权的导航父级。读取完整直接子级后调整顺序，再通过下方同一库设置准备与确认；本控件不移动目录、不保存正文。"));
    const parents=this.catalog.orderParents??[], picker=ui("select");picker.setAttribute("aria-label","作者顺序父级");
    for(const parent of parents){const option=ui("option","",`${parent.title} · ${parent.reference}`);option.value=parent.reference;picker.append(option);}
    if(!parents.some(parent=>parent.reference===this.orderParent))this.orderParent=parents[0]?.reference??"";picker.value=this.orderParent;
    picker.disabled=this.busy||!!draft.pending;picker.onchange=()=>{this.orderParent=picker.value;this.order=undefined;this.renderOrder();};
    const read=action("读取完整作者子级",()=>this.run(async()=>{
      const parent=this.orderParent,epoch=this.epoch,basis=profile.basis;
      const result=await this.request<HostedOrder>("library/order/read",{parentRef:parent});
      if(epoch!==this.epoch||this.profile?.basis!==basis||this.draft!==draft||this.orderParent!==parent)return;
      if(result.parentRef!==parent||!Array.isArray(result.members)||!Array.isArray(result.orderedRefs)||typeof result.complete!=="boolean"||result.settingsSourceHash!==await nodeSourceHash(profile.source))throw new Error("子级或库设置依据已变化，请保留输入并重新读取配置。");
      this.order=result;this.renderOrder();
    }));read.disabled=this.busy||!!draft.pending||!this.orderParent;this.orderHost.append(picker,read);
    if(!parents.length)this.orderHost.append(ui("p","hint","没有独立获权的导航父级；作者重排权不授予全库成员读取。"));
    const order=this.order;if(!order||order.parentRef!==this.orderParent)return;
    if(!order.complete){this.orderHost.append(ui("p","hint","直接子级未完整接纳或观察，不能准备作者重排。"));return;}
    let settings:Record<string,unknown>;try{settings=JSON.parse(draft.source);if(!settings||Array.isArray(settings)||settings.version!==1)throw Error();}catch{this.orderHost.append(ui("p","hint","当前JSON尚未完成，原输入保留。"));return;}
    const orders=settings.navigationOrders as Record<string,unknown>|undefined,stored=orders?.[order.parentRef];
    if(stored!==undefined&&(!Array.isArray(stored)||stored.some(ref=>typeof ref!=="string"))){this.orderHost.append(ui("p","hint","当前父级顺序不是完整引用数组，请核对原JSON。"));return;}
    const actual=new Set(order.orderedRefs),refs=(stored as string[]|undefined)?.filter(ref=>actual.has(ref))??[...order.orderedRefs];
    for(const ref of order.orderedRefs)if(!refs.includes(ref))refs.push(ref);
    for(const[ordinal,ref]of refs.entries()){
      const row=ui("div","hosted-order-row"),name=order.members.find(member=>member.targetRef===ref)?.name??"原稳定对象";row.append(ui("span","",`${ordinal+1}. ${name} · ${ref}`));
      for(const[direction,label]of[[-1,"上移"],[1,"下移"]]as const){const move=action(label,()=>{
        if(this.draft!==draft||this.order!==order||draft.pending||this.busy)return;
        const latest=JSON.parse(draft.source)as Record<string,unknown>;if(!latest||Array.isArray(latest)||latest.version!==1)throw new Error("JSON已变化，未覆盖原输入");
        if(latest.navigationOrders===undefined)latest.navigationOrders={};
        if(!latest.navigationOrders||typeof latest.navigationOrders!=="object"||Array.isArray(latest.navigationOrders))throw new Error("原navigationOrders格式无效，未覆盖原输入");
        (latest.navigationOrders as Record<string,unknown>)[order.parentRef]=movedSibling(refs,ref,direction);draft.source=JSON.stringify(latest,null,2);this.source.value=draft.source;this.edited(true);this.renderOrder();
      });move.setAttribute("aria-label",`${name} 作者顺序${label}`);move.disabled=this.busy||!!draft.pending||direction===-1&&ordinal===0||direction===1&&ordinal===refs.length-1;row.append(move);}this.orderHost.append(row);
    }
  }
  private renderActions() {
    if (this.comparison) this.comparison.disabled = this.busy || !!this.draft?.pending;
    this.actions.replaceChildren(); if (!this.role) return; const draft = this.draft, pending = draft?.pending;
    const read = action("读取托管库配置", () => this.run(() => this.read())); read.disabled = this.busy || this.unreadable !== undefined; this.actions.append(read);
    const prepare = action("准备库配置变更", () => this.run(() => this.prepare())); prepare.disabled = this.busy || !this.writable() || !draft || !this.profile || !!pending || this.needsComparison(); this.actions.append(prepare);
    if (draft?.lastReceipt) this.actions.append(ui("p", "hint", `原配置回执：${draft.lastReceipt.operationId} · ${draft.lastReceipt.status}${draft.lastReceipt.status === "committed" ? "（提交事实，不代表当前读取依据）" : "（未冒作已保存）"}`));
    if (!pending) return;
    const card = ui("section"); card.setAttribute("aria-label", "核对原库配置操作"); card.append(ui("h3", "", "核对原库配置操作"), ui("p", "", `${pending.request.role} · ${pending.request.operationId} · ${pending.receipt?.status ?? "unknown"}`));
    const preview = pending.receipt?.preview;
    for (const line of Array.isArray(preview?.summary) ? preview.summary : preview?.summary ? [preview.summary] : []) card.append(ui("p", "", line));
    for (const [label, value] of [["原计划提交前配置", preview?.beforeSource], ["原计划提交后配置", preview?.afterSource]] as const) if (typeof value === "string") { const area = ui("textarea"); area.rows = 8; area.readOnly = true; area.value = value; area.setAttribute("aria-label", label); card.append(ui("p", "", label), area); }
    if (preview?.enabled !== undefined) card.append(ui("p", "hint", `原计划库样式：${preview.enabled ? "启用" : "禁用"}`));
    if (draft && pending.revision !== draft.revision) card.append(ui("p", "hint", "当前输入已不同于原冻结提案；原回执不会覆盖它。先核对或结束原计划，再准备新输入。"));
    const query = action("查询原库配置操作", () => this.run(() => this.operation())); query.disabled = this.busy;
    const retry = action("重试原配置准备请求", () => this.run(() => this.sendPrepare())); retry.disabled = this.busy || !this.writable() || !!pending.commit;
    const settle = action("结束原库配置操作并保留输入", () => this.run(() => this.operation(true))); settle.disabled = this.busy || !this.writable();
    const commit = action(pending.commit ? "重试原配置提交请求" : "确认提交原库配置计划", () => this.run(() => this.commit())); commit.disabled = this.busy || !draft || !libraryCanCommit(draft) || !this.writable() || this.needsComparison();
    card.append(query, retry, settle, commit); this.actions.append(card);
  }
}
