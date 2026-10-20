import { action, ui } from "./data-panel";

export type CitationForm = "normal" | "prose" | "author" | "year" | "full" | "none";
export interface NativeCitationRange { from: number; to: number; utf16From: number; utf16To: number; raw: string; writable: boolean }
export interface NativeCitationSlot { key: string; bindingProven?: boolean; range: NativeCitationRange; form?: CitationForm | null; formExplicit?: boolean; supplement?: string | null; supplementExplicit?: boolean; style?: string | null; styleExplicit?: boolean; editable: boolean; reason?: string | null }
export interface NativeReferenceTarget { label: string; kind: string; range: NativeCitationRange; numbered?: boolean; editable?: boolean; reason?: string | null }
export interface NativeCitationSnapshot {
  ownerReference?: string;
  generation: string; coverage: "complete" | "partial";
  choices: { reference: string; title: string; node?: { workspaceId: string; nodeId: string }; citationReady: boolean; suggestedKey?: string; reason?: string | null }[];
  editor: { sourceHash: string; bindings: { key: string; work: string }[]; citations: NativeCitationSlot[]; targets: NativeReferenceTarget[]; crossReferences: { label: string; kind?: string; renderedKind?: string; range: NativeCitationRange }[]; renameAvailable: boolean; renameReason?: string | null };
  bindings: { key: string; work: string; state: string; authoredSlots: NativeCitationSlot[] }[];
  incoming: { title: string; reference: string; slots: NativeCitationSlot[]; visibility: string }[];
  displayStatus: string; displayExplanation: string;
  bibliographyOwnership?: "singleVirtual" | "unavailable";
  links?: { range: NativeCitationRange; target: string; title?: string | null; editable: boolean; reason?: string }[];
  unlabelledTargets?: { kind: string; range: NativeCitationRange; title: string }[];
  displayed?: { key: string; form?: string | null; ordinal: number; range?: NativeCitationRange | null; visibility?: string }[];
}
export interface NativeCitationInputs { sourceHash?: string; search: string; mode: string; work: string; key: string; form: CitationForm; supplement: string; text: string; target: string; newLabel: string; referenceForm: string; rename: Record<string, string>; slots: Record<string, { form: CitationForm | ""; supplement: string; supplementChanged: boolean; style: string; styleChanged?: boolean }> }
export function newNativeCitationInputs(): NativeCitationInputs { return { search: "", mode: "link", work: "", key: "", form: "normal", supplement: "", text: "", target: "", newLabel: "", referenceForm: "normal", rename: {}, slots: {} }; }
export function citationInputScope(context: { deployment: string; actor: string; resource: string }, ownerReference?: string): string { return JSON.stringify([context.deployment, context.actor, ownerReference ?? context.resource]); }
export function citationInsertionByte(source: string, utf16: number): number {
  if (!Number.isSafeInteger(utf16) || utf16 < 0 || utf16 > source.length || utf16 > 0 && utf16 < source.length && /[\uD800-\uDBFF]/.test(source[utf16 - 1]) && /[\uDC00-\uDFFF]/.test(source[utf16])) throw new Error("请选择准确源码字符边界后插入引用。");
  return new TextEncoder().encode(source.slice(0, utf16)).length;
}
export function citationCandidateKey(data: NativeCitationSnapshot, work: string): { key: string; reason?: string } {
  const bindings = data.editor.bindings.filter(binding => binding.work === work);
  if (bindings.length === 1) return { key: bindings[0].key };
  if (bindings.length > 1) return { key: "", reason: "此 Work 已有多个绑定键，请明确选择现有键。" };
  const suggested = data.choices.find(choice => choice.reference === work)?.suggestedKey || "work";
  if (!/^[A-Za-z][A-Za-z0-9_-]*$/.test(suggested)) return { key: "", reason: "出版数据建议键不符合原生 ASCII profile，请明确输入新键。" };
  if (data.editor.bindings.some(binding => binding.key === suggested) || data.editor.targets.some(target => target.label === suggested)) return { key: "", reason: "建议键与其它 Work 绑定或正文标签冲突，请明确输入新键。" };
  return { key: suggested };
}
const forms: [string, string][] = [["normal", "普通"], ["prose", "叙述式"], ["author", "仅作者"], ["year", "仅年份"], ["full", "完整条目"], ["none", "仅加入文献表"]];
function select(values: [string, string][], value: string, label: string): HTMLSelectElement { const control = ui("select", "data-input"); control.setAttribute("aria-label", label); for (const [key, text] of values) { const option = ui("option", "", text); option.value = key; control.append(option); } control.value = value; return control; }
function input(value: string, label: string): HTMLInputElement { const control = ui("input", "data-input"); control.value = value; control.setAttribute("aria-label", label); control.placeholder = label; return control; }
export function renderNativeCitations(container: HTMLElement, data: NativeCitationSnapshot | undefined, state: NativeCitationInputs, host: { writable: boolean; at(): number; perform(title: string, intent: Record<string, unknown>): Promise<unknown>; select(range: NativeCitationRange): void; navigate(reference: string): void; repaint(): void }): void {
  if (!data) { container.append(ui("p", "hint", "当前引用目录不可用，请刷新当前获权知识信息。")); return; }
  if(state.sourceHash!==data.editor.sourceHash){state.target="";state.sourceHash=data.editor.sourceHash;}
  const run = async (title: string, intent: Record<string, unknown>): Promise<void> => { await host.perform(title, { ...intent, generation: data.generation }); };
  container.append(ui("p", "hint", data.coverage === "partial" ? "当前获权引用目录部分可用；未发现不代表全库没有引用。" : "当前获权范围已读取。"));
  const mode = select([["link", "普通笔记链接"], ["citation", "正式文献引用"], ["reference", "本篇交叉引用"]], state.mode, "插入引用类型"); mode.onchange = () => { state.mode = mode.value; host.repaint(); }; container.append(mode);
  const form = select(forms, state.form, "引用形式"); form.onchange = () => { state.form = form.value as CitationForm; };
  const supplement = input(state.supplement, "页码或章节补充（纯文字）"); supplement.oninput = () => { state.supplement = supplement.value; };
  const display = input(state.text, "自定义显示文字（空值使用当前标题）"); display.oninput = () => { state.text = display.value; };
  if (state.mode !== "reference") {
    const search = input(state.search, "搜索获权笔记或论文"); search.oninput = () => { state.search = search.value; const term = search.value.toLocaleLowerCase(); for (const option of choice.options) option.hidden = !!option.value && !option.text.toLocaleLowerCase().includes(term); };
    const choices = data.choices.filter(choice => !state.search || choice.title.toLocaleLowerCase().includes(state.search.toLocaleLowerCase()) || choice.reference === state.work);
    const choice = select([["", "选择目标"], ...choices.map(choice => [choice.reference, choice.title] as [string, string])], state.work, "笔记或文献目标");
    const reason = ui("p", "hint"); const key = input(state.key, "原生引用键（已有绑定优先复用）"); key.oninput = () => { state.key = key.value; };
    const describe = (): void => { const candidate = data.choices.find(choice => choice.reference === state.work); reason.textContent = state.mode === "citation" ? !candidate?.citationReady ? candidate?.reason || "选择具备获权出版数据的 Work。" : citationCandidateKey(data, state.work).reason || "引用形式独立于样式；实际显示取决于当前出版数据及原生样式。" : "显示文字写入作者源；目标改名后保持原文字。"; insert.disabled = !host.writable || !candidate || state.mode === "citation" && !candidate.citationReady; };
    choice.onchange = () => { state.work = choice.value; const suggestion = citationCandidateKey(data, state.work); state.key = suggestion.key; key.value = state.key; describe(); };
    const insert = action(state.mode === "citation" ? "插入正式文献引用" : "插入普通笔记链接", () => run("插入原生引用", state.mode === "citation" ? { kind: "nativeCitationInsert", at: host.at(), work: state.work, key: state.key, form: state.form, supplement: state.supplement } : { kind: "nativeCitationLink", at: host.at(), target: state.work, text: state.text }));
    container.append(search, choice, reason); if (state.mode === "citation") container.append(key, form, supplement); else container.append(display); container.append(insert); describe();
  } else {
    const targets = select([["", "选择本篇原生目标"], ...data.editor.targets.map(target => [target.label, `${target.kind} · ${target.label}`] as [string, string]), ...(data.unlabelledTargets??[]).map(target=>[`new:${target.range.from}`,`${target.kind} · 补唯一标签：${target.title}`] as [string,string])], state.target, "本篇交叉引用目标"); targets.onchange = () => { state.target = targets.value; host.repaint(); };
    const referenceForm = select([["normal", "原生编号引用"], ["page", "页码引用（仅实际分页输出）"], ["text", "自定义文字链接"]], state.referenceForm, "交叉引用形式"); referenceForm.onchange = () => { state.referenceForm = referenceForm.value; };
    const label=input(state.newLabel,"为未标记对象明确填写唯一原生标签");label.oninput=()=>{state.newLabel=label.value;};
    const insert = action("插入本篇交叉引用", () => {const target=(data.unlabelledTargets??[]).find(target=>`new:${target.range.from}`===state.target);return run("插入本篇交叉引用",target?{kind:"nativeReferenceInsertWithLabel",at:host.at(),target:target.range,label:state.newLabel,form:state.referenceForm}:{ kind: "nativeReferenceInsert", at: host.at(), label: state.target, form: state.referenceForm, text: state.text });}); insert.disabled = !host.writable;
    container.append(targets, label, referenceForm, display, ui("p", "hint", "标签必须唯一且有准确来源；补标签与引用进入同一草稿及一次 Undo。新标记目前支持编号／页码引用，自定义文字可在已有标签上插入；流式阅读不提供页码。"), insert);
    const rename=action("预览所选标签改名范围",()=>run("重命名当前作者范围的标签",{kind:"nativeReferenceRename",old:state.target,new:state.newLabel}));rename.disabled=!host.writable||!data.editor.renameAvailable||!state.target||state.target.startsWith("new:");container.append(rename,ui("p","hint","标签改名只覆盖当前作者完整获证范围；其它笔记或外部复制的旧片段地址不保证仍有效。"));
  }
  container.append(ui("h3", "", "本篇文献绑定与源码引用"), ui("p", "hint", data.displayExplanation));
  for (const binding of data.bindings) {
    const group = ui("section", "knowledge-section"); const choice = data.choices.find(choice => choice.reference === binding.work);
    group.append(ui("strong", "", `${binding.key} · ${choice?.title || "获权目标当前不可用"}`), ui("p", "hint", binding.state === "boundOnly" ? "已绑定；未证明有正文源码引用。" : `${binding.authoredSlots.length} 个作者源码引用槽。`));
    if(data.displayStatus==="nativeCompileAvailable" && data.bibliographyOwnership==="singleVirtual") {const displayed=(data.displayed??[]).filter(item=>item.key===binding.key);group.append(ui("p","hint",`本次 HTML：${displayed.filter(item=>item.visibility!=="bibliographyOnly").length} 个实际引用组成员；${displayed.filter(item=>item.visibility==="bibliographyOnly").length} 个仅文献表成员。组成员可共享一个显示位置；生成出现项不冒作者源码槽。`));for(const item of displayed){if(item.range)group.append(action(`定位本次显示项 ${item.ordinal+1} 的主源`,()=>host.select(item.range!)));}}
    else if(data.bibliographyOwnership==="unavailable")group.append(ui("p","hint","文献表归属或动态环境未获完整证明；实际原生 key 不按同名猜接此 Work。"));
    if (choice) group.append(action("打开获权目标", () => host.navigate(binding.work)));
    const renameId=`${data.editor.sourceHash}:${binding.key}`;
    const rename = input(state.rename[renameId] ?? "", `将 ${binding.key} 改名为`); rename.oninput = () => { state.rename[renameId] = rename.value; };
    const apply = action("预览完整改键范围", () => run("重命名当前归属引用键", { kind: "nativeCitationRename", old: binding.key, new: state.rename[renameId] })); apply.disabled = !host.writable || !data.editor.renameAvailable;
    group.append(rename, apply); if (data.editor.renameReason) group.append(ui("p", "hint", data.editor.renameReason));
    for (const slot of binding.authoredSlots) {
      const id = `${data.editor.sourceHash}:${slot.range.from}:${slot.key}`, values = state.slots[id] ??= { form: "", supplement: slot.supplement ?? "", supplementChanged: false, style: slot.style ?? "" };
      const row = ui("article", "knowledge-card"); row.append(action(`定位源码：${slot.key}`, () => host.select(slot.range)), ui("p", "hint", `形式：${slot.form ?? (slot.formExplicit ? "显式原生表达（见源码）" : "继承原生 set 规则")}；样式：${slot.style ?? (slot.styleExplicit ? "显式原生表达（见源码）" : "继承原生规则 / bibliography")}${slot.form === "none" ? "；仅文献表，不显示正文标记" : ""}`));
      if (!slot.editable) row.append(ui("p", "hint", slot.reason || "此合法原生表达没有安全控件；请从源码编辑。"));
      const type = select([["", "保持当前形式 / 继承规则"], ...forms], values.form, `${slot.key} 引用形式`); const extra = input(values.supplement, `${slot.key} 定位补充`);
      const update = action("仅更新本次改动参数", () => run("更新一个引用", { kind: "nativeCitationUpdate", range: slot.range, ...(values.form ? { form: values.form } : {}), ...(values.supplementChanged ? { supplement: values.supplement } : {}) }));
      const enabled = (): void => { update.disabled = !host.writable || !slot.editable || !values.form && !values.supplementChanged; }; type.onchange = () => { values.form = type.value as CitationForm | ""; enabled(); }; extra.oninput = () => { values.supplement = extra.value; values.supplementChanged = true; enabled(); }; enabled();
      const style = input(values.style, "本引用样式（空值恢复继承；如 apa、ieee、vancouver-superscript）"); style.oninput = () => { values.style = style.value; values.styleChanged=true; }; const updateStyle = action("仅更新此引用样式", () => run("更新一个引用的原生样式", { kind: "nativeCitationStyle", range: slot.range, style: values.style })); updateStyle.disabled = !host.writable || !slot.editable;
      row.append(type, extra, update, style, updateStyle); group.append(row);
    }
    container.append(group);
  }
  container.append(ui("h3", "", "文献入引用（按来源笔记分组）"));
  if (!data.incoming.length) container.append(ui("p", "hint", data.coverage === "complete" ? "当前获权范围未证明作者 cite 入引用；动态引用仍需编译确认。" : "暂未发现；当前范围不完整。"));
  for (const source of data.incoming) { const details = ui("details"); details.append(ui("summary", "", `${source.title} · ${source.slots.length} 处源码引用`), action("打开来源笔记", () => host.navigate(source.reference))); for (const slot of source.slots) details.append(ui("pre", "knowledge-address", `${slot.key}: ${slot.range.raw}`)); container.append(details); }
  container.append(ui("h3", "", "普通链接显示文字"));
  for(const link of data.links??[]){const row=ui("article","knowledge-card");row.title=link.title?`当前获权标题：${link.title}`:"目标当前不可用";row.append(action("定位原生链接",()=>host.select(link.range)),ui("p","knowledge-address",link.target),ui("p","hint",link.title?`当前标题：${link.title}；原正文显示文字保持作者值。`:link.reason||"目标当前不可用，未显示旧标题。"));const update=action("更新此链接文字为当前标题",()=>run("更新所选链接文字",{kind:"nativeCitationLinkText",range:link.range,occurrence:{from:link.range.from,to:link.range.to,target:link.target}}));update.disabled=!host.writable||!link.editable;row.append(update);container.append(row);}
  container.append(ui("h3","","本篇交叉引用与待分类原生引用"));
  for(const reference of data.editor.crossReferences){if(reference.renderedKind==="citation")continue;container.append(action(`定位 ${reference.label}`,()=>host.select(reference.range)),ui("span","knowledge-badge",reference.renderedKind==="document"?"本次原生文内引用":reference.kind==="link"?"文内链接源码槽":"源码候选；执行类型未确认"));}
  const retained=Object.entries(state.slots).filter(([key,value])=>!key.startsWith(`${data.editor.sourceHash}:`)&&(value.form||value.supplementChanged||value.styleChanged));
  const renamed=Object.entries(state.rename).filter(([key,value])=>!key.startsWith(`${data.editor.sourceHash}:`)&&value);
  if(retained.length||renamed.length){const recovery=ui("details");recovery.append(ui("summary","","未应用的旧来源引用输入"),ui("p","hint","原文依据已变化，旧输入保留在这里；请明确选择当前引用后再填写，不自动套用到另一出现项。"),ui("pre","knowledge-address",JSON.stringify({citations:retained.map(([source,value])=>({source,form:value.form||undefined,supplement:value.supplementChanged?value.supplement:undefined,style:value.styleChanged?value.style:undefined})),renames:renamed},null,2)));container.append(recovery);}
}
