import type { ValueInputBinding } from "./author-input";
import { submitOriginal, TerminatedAttempt, type OriginalFact } from "./original-operation";
/** Schema forms operate on native value trees; only the source owner writes Typst. */
import type { OpenedDocument } from "./types";
import { TagTree } from "./tag-tree";
import { tagPicker } from "./tag-picker";
export type NativeValue =
  | { kind: "none" }
  | { kind: "text" | "int" | "float" | "decimal" | "date" | "time" | "localDatetime" | "duration"; value: string }
  | { kind: "bool"; value: boolean }
  | { kind: "array"; value: NativeValue[] }
  | { kind: "record"; value: { key: string; value: NativeValue }[] };
export type Shape =
  | { kind: "scalar"; semantic: string }
  | { kind: "enum"; values: string[]; open: boolean }
  | { kind: "list"; items: Shape; min: number; max: number | null }
  | { kind: "record"; members: Record<string, { shape: Shape; required: boolean; nullable: boolean }>; unknown: string }
  | { kind: "union"; variants: Shape[]; discriminator: null | { kind: string; keys?: string[]; key?: string; present?: number; absent?: number } }
  | { kind: "use"; name: string };
export interface SchemaField { id: string; key: string; shape: Shape; nullable: boolean; default?: { kind: string; source?: string } }
export interface SchemaType { id: string; fields: { id: string; required: boolean }[]; requires: string[]; conflicts: string[] }
export interface DataSchema { definitions: Record<string, Shape>; fields: SchemaField[]; types: SchemaType[]; aliases: Record<string, string>; series_policies?: { key: string; kinds: string[]; scope: { kind: string }; multiplicity: string }[] }
export interface Cell { state: "value" | "absent" | "null" | "invalid" | "unavailable"; value?: NativeValue; diagnostics?: unknown[]; reason?: string }
interface ReferenceChoice { ref: string; label: string }
export interface FormReferences { nodes: ReferenceChoice[]; tagTree?: TagTree; state: "loading" | "ready" | "partial" | "unavailable"; messages: string[] }
export interface FieldProjection { fieldId: string; key: string; cell: Cell; sourceRange?: { from: number; to: number }; writable: boolean; raw?: string | null; unbound?: string[] }
export interface NoteProjection { qualification: string; id?: string; types: string[]; fields: FieldProjection[]; unknownFields: { key: string; raw: string; range?: { from: number; to: number } }[]; diagnostics: unknown[]; readOnly?: boolean }
export interface DocumentBasis { sessionId: string; source: string; serial: number; baseId: string; path: string; root: string; nodeId?: string; workspaceId?: string; dirty?: boolean }
export interface SourcePatch { from: number; to: number; text: string }
export interface SchemaRead { schema: DataSchema; source: string; roleRef?: string; revision: string | null; builtinDefault: boolean; canInstall: boolean }
export function decodeProjection(result: NoteProjection | { projection: NoteProjection; readOnly?: boolean }): NoteProjection { return "projection" in result ? result.readOnly === undefined ? result.projection : { ...result.projection, readOnly: result.readOnly } : result; }
export function decodeSourcePatch(result: SourcePatch | { edit: SourcePatch } | { patches: SourcePatch[]; source?: string }, source: string): SourcePatch {
  if ("edit" in result) return result.edit; if (!("patches" in result)) return result;
  const patches = [...result.patches].sort((a, b) => a.from - b.from);
  if (!patches.length) return { from: 0, to: 0, text: "" };
  let prior = -1; for (const patch of patches) { if (!Number.isInteger(patch.from) || !Number.isInteger(patch.to) || patch.from < 0 || patch.to < patch.from || patch.to > source.length || patch.from < prior) throw new Error("属性补丁范围无效；草稿保持原样。"); prior = patch.to; }
  const from = patches[0].from; const to = patches.at(-1)!.to; let text = ""; let cursor = from;
  for (const patch of patches) { text += source.slice(cursor, patch.from) + patch.text; cursor = patch.to; } text += source.slice(cursor, to);
  if (result.source !== undefined && source.slice(0, from) + text + source.slice(to) !== result.source) throw new Error("属性补丁与服务端回读不一致；草稿保持原样。");
  return { from, to, text };
}
export interface PanelHost {
  authorInputs?(basis: DocumentBasis): import("./author-input").AuthorInputSession | undefined;
  canEdit?(): boolean;
  request<T>(path: string, body?: unknown): Promise<T>;
  requestBlob?(path: string, body: unknown): Promise<Blob>;
  getDocument(): DocumentBasis | null;
  applyPatch(patch: SourcePatch, basis: DocumentBasis): Promise<void>;
  openDocument(path: string, root?: string): Promise<void>;
  openResolved?(document: OpenedDocument): Promise<void>;
  openTemplates?(): void | Promise<void>;
  notify(message: string, error?: boolean): void;
  /** A confirmed Core mutation changed the original library's Query environment. */
  environmentChanged?(root: string): void;
  prepareAction?(): Promise<void>;
  /** A create success must be opened by the host, using its returned Core session. */
  created?(result: unknown): Promise<void>;
  /** Explicitly acknowledges the Core save of a query and its owning note. */
  savedDefinition?(result: unknown, basis: DocumentBasis): Promise<void>;
  /** Reconcile one committed original Core operation without replacing later input. */
  applyOperationReceipt?(receipt: unknown, basis: DocumentBasis): Promise<void>;
}

export const fieldNames: Record<string, string> = { status: "状态", due: "截止日期", start: "开始日期", priority: "优先级", project: "项目", completed_at: "完成时间", title: "标题", names: "姓名 / 名称", phones: "电话", emails: "邮箱", addresses: "地址", websites: "网站", accounts: "账号", tags: "标签", publication: "出版与引用", contributors: "作者与贡献者", resources: "资源", note: "备注", notes: "备注", label: "标签", number: "号码", value: "值", text: "文字", ref: "笔记引用", kind: "种类", role: "角色", agent: "作者", credited_as: "署名", date: "日期", amount: "金额", currency: "币种", unit: "单位", during: "有效期间", source: "来源", validity: "有效期间", period: "周期", range: "时间区间", event: "事件", dependency: "依赖任务" };
export const typeNames: Record<string, string> = { person: "人员", organization: "组织", org: "组织", task: "任务", project: "项目", work: "作品", paper: "论文", event: "事件", period: "周期笔记", "period-note": "周期笔记", range: "区间笔记", "range-note": "区间笔记" };
export const cellLabels: Record<Cell["state"], string> = { value: "已有值", absent: "未填写", null: "明确为空（none）", invalid: "值无效", unavailable: "不可用 / 计算值" };
const commonFields = new Set(["title", "names", "status", "due", "project", "priority", "phones", "emails", "tags", "publication", "period", "range"]);

export function resolveShape(shape: Shape, schema: DataSchema, seen = new Set<string>()): Shape {
  if (shape.kind !== "use") return shape;
  if (seen.has(shape.name) || !schema.definitions[shape.name]) throw new Error(`无法展开 schema：${shape.name}`);
  return resolveShape(schema.definitions[shape.name], schema, new Set([...seen, shape.name]));
}
export function fieldsForTypes(schema: DataSchema, tokens: string[]): { field: SchemaField; required: boolean }[] {
  const required = new Map<string, boolean>(); const visited = new Set<string>();
  const visit = (token: string): void => {
    const id = schema.aliases[token] ?? token; if (visited.has(id)) return; visited.add(id);
    const type = schema.types.find(item => item.id === id); if (!type) return;
    type.requires.forEach(visit); type.fields.forEach(item => required.set(item.id, !!item.required || !!required.get(item.id)));
  };
  tokens.forEach(visit); return schema.fields.filter(field => required.has(field.id)).map(field => ({ field, required: required.get(field.id)! }));
}
export function typeConflicts(schema: DataSchema, tokens: string[]): string[] {
  const expanded = new Set<string>();
  const visit = (token: string): void => { const id = schema.aliases[token] ?? token; if (expanded.has(id)) return; expanded.add(id); schema.types.find(type => type.id === id)?.requires.forEach(visit); };
  tokens.forEach(visit);
  return schema.types.filter(type => expanded.has(type.id)).flatMap(type => type.conflicts.filter(id => expanded.has(id)).map(id => `${type.id} 与 ${id} 冲突`));
}
export function nativeMember(value: NativeValue | undefined, key: string): NativeValue | undefined { return value?.kind === "record" ? value.value.find(item => item.key === key)?.value : undefined; }
export function replaceNativeMember(value: NativeValue, key: string, replacement: NativeValue | undefined): NativeValue {
  if (value.kind !== "record") throw new Error("成员修改需要 record 值");
  const items = value.value.map(item => ({ ...item })); const index = items.findIndex(item => item.key === key);
  if (replacement === undefined) { if (index >= 0) items.splice(index, 1); }
  else if (index >= 0) items[index] = { key, value: replacement }; else items.push({ key, value: replacement });
  return { kind: "record", value: items };
}
export function moveOrderedValue(value: NativeValue, from: number, to: number): NativeValue {
  if (value.kind !== "array" || from < 0 || to < 0 || from >= value.value.length || to >= value.value.length) return value;
  const items = [...value.value]; const [item] = items.splice(from, 1); items.splice(to, 0, item); return { kind: "array", value: items };
}
export function unionVariant(shape: Extract<Shape, { kind: "union" }>, value: NativeValue, schema: DataSchema): number {
  const discriminator = shape.discriminator;
  if (value.kind === "record" && discriminator) {
    if (discriminator.kind === "member") { const matching = (discriminator.keys ?? []).map((key, index) => nativeMember(value, key) ? index : -1).filter(index => index >= 0); return matching.length === 1 ? matching[0] : -1; }
    if (discriminator.kind === "record_presence") return nativeMember(value, discriminator.key!) ? discriminator.present! : discriminator.absent!;
    if (discriminator.kind === "tag") {
      const tag = nativeMember(value, discriminator.key!); if (tag?.kind !== "text") return -1;
      const matching = shape.variants.flatMap((variant, index) => { const resolved = resolveShape(variant, schema); if (resolved.kind !== "record") return []; const member = resolved.members[discriminator.key!]; if (!member) return []; const memberShape = resolveShape(member.shape, schema); return memberShape.kind === "enum" && memberShape.values.includes(tag.value) ? [index] : []; });
      return matching.length === 1 ? matching[0] : -1;
    }
  }
  const matching = shape.variants.flatMap((variant, index) => { const resolved = resolveShape(variant, schema); const kind = resolved.kind === "enum" ? "text" : resolved.kind === "list" ? "array" : resolved.kind === "record" ? "record" : resolved.kind === "scalar" ? nativeKind(resolved.semantic) : "?"; return kind === value.kind ? [index] : []; });
  return matching.length === 1 ? matching[0] : -1;
}
function nativeKind(semantic: string): NativeValue["kind"] { return ({ local_datetime: "localDatetime", exact_decimal: "text", exact_seconds: "text" } as Record<string, NativeValue["kind"]>)[semantic] ?? (["bool", "int", "float", "decimal", "date", "time", "duration"].includes(semantic) ? semantic as NativeValue["kind"] : "text"); }
export function emptyNativeValue(shape: Shape, schema: DataSchema): NativeValue {
  const resolved = resolveShape(shape, schema);
  if (resolved.kind === "union") return emptyNativeValue(resolved.variants[0], schema);
  if (resolved.kind === "list") return { kind: "array", value: Array.from({ length: resolved.min }, () => emptyNativeValue(resolved.items, schema)) };
  if (resolved.kind === "record") return { kind: "record", value: Object.entries(resolved.members).filter(([, member]) => member.required).map(([key, member]) => ({ key, value: emptyNativeValue(member.shape, schema) })) };
  if (resolved.kind === "enum") return { kind: "text", value: resolved.values[0] ?? "" };
  if (resolved.kind !== "scalar") throw new Error("未知 shape");
  const kind = nativeKind(resolved.semantic); if (kind === "bool") return { kind, value: false };
  return { kind: kind as "text", value: ["int", "float", "decimal", "duration"].includes(kind) ? "0" : "" };
}
export function switchUnionValue(shape: Extract<Shape, { kind: "union" }>, current: NativeValue, branch: number, schema: DataSchema): NativeValue {
  const target = resolveShape(shape.variants[branch], schema); let next = emptyNativeValue(target, schema);
  if (current.kind !== "record") return next;
  const known = new Set(shape.variants.flatMap(variant => { const resolved = resolveShape(variant, schema); return resolved.kind === "record" ? Object.keys(resolved.members) : []; }));
  const unknown = current.value.filter(item => !known.has(item.key));
  if (unknown.length && (target.kind !== "record" || target.unknown !== "preserve")) throw new Error("所选分支不能保留未知成员；原值保留，请在源码中明确处理。");
  if (target.kind === "record" && next.kind === "record") {
    const oldBranch = unionVariant(shape, current, schema); const old = oldBranch >= 0 ? resolveShape(shape.variants[oldBranch], schema) : undefined;
    for (const item of current.value) if (!known.has(item.key) || (old?.kind === "record" && target.members[item.key] && JSON.stringify(old.members[item.key]?.shape) === JSON.stringify(target.members[item.key].shape))) next = replaceNativeMember(next, item.key, item.value);
  }
  return next;
}
/** Never parse exact numerics through Number. The backend performs domain validation. */
export function scalarFromInput(shape: Shape, schema: DataSchema, text: string): NativeValue {
  const resolved = resolveShape(shape, schema); if (resolved.kind === "enum") return { kind: "text", value: text };
  if (resolved.kind !== "scalar") throw new Error("需要标量字段"); const kind = nativeKind(resolved.semantic);
  if (kind === "bool") return { kind, value: text === "true" };
  if (kind === "int" && !/^-?(0|[1-9][0-9]*)$/.test(text)) throw new Error("整数需完整输入，保留准确值");
  if (["float", "decimal", "duration"].includes(kind) && !/^[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?$/.test(text)) throw new Error("请输入完整数值");
  return { kind: kind as "text", value: text };
}
export function formatNativeValue(value: NativeValue | undefined): string {
  if (!value) return "未填写"; if (value.kind === "none") return "none";
  if (value.kind === "bool") return value.value ? "是" : "否";
  if (value.kind === "array") return value.value.map(formatNativeValue).join("；");
  if (value.kind === "record") return value.value.map(item => `${fieldNames[item.key] ?? item.key}：${formatNativeValue(item.value)}`).join(" · ");
  return value.value;
}
export function diagnosticText(value: unknown): string { return typeof value === "string" ? value : value && typeof value === "object" && "message" in value ? String(value.message) : JSON.stringify(value); }

export function ui<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text = ""): HTMLElementTagNameMap[K] { const element = document.createElement(tag); element.className = className; element.textContent = text; return element; }
export function action(label: string, run: () => void | Promise<void>, className = "data-button"): HTMLButtonElement { const button = ui("button", className, label); button.type = "button"; button.onclick = () => { if (button.isConnected === false) return; void run(); }; return button; }
let nextInputId = 0;
function labelFor(label: string, input: HTMLElement): HTMLLabelElement { const result = ui("label", "data-label", label); input.id ||= `data-input-${++nextInputId}`; result.htmlFor = input.id; return result; }
const cloneValue = (value: NativeValue): NativeValue => structuredClone(value);
const emptyReferences = (): FormReferences => ({ nodes: [], state: "loading", messages: [] });
const refreshTagPickers = new WeakMap<HTMLElement, () => void>();
export async function loadTagFormReferences(host: PanelHost, root: string, references: FormReferences, container: HTMLElement, accepts = (): boolean => true): Promise<void> {
  references.tagTree = undefined;
  try { const result = await host.request<{ tags: { tagRef: string; tagId: string; displayName: string; parentTagId: string | null }[]; complete: boolean }>("/api/navigation/tags", { root }); if (!accepts()) return; references.tagTree = new TagTree(result.tags.map(tag => ({ ...tag, reference: tag.tagRef })), result.complete === true); references.state = "ready"; }
  catch (error) { if (!accepts()) return; references.state = "unavailable"; references.messages.push(String(error)); }
  for (const picker of container.querySelectorAll<HTMLElement>(".data-tag-reference-picker")) refreshTagPickers.get(picker)?.();
}
function populateReferenceChoices(select: HTMLSelectElement, references: FormReferences): void { const choices = references.nodes; const current = select.value; select.replaceChildren(); const option = ui("option", "", references.state === "loading" ? "正在读取实际引用对象…" : choices.length ? "选择实际对象…" : "没有可选对象（保留已有地址）"); option.value = ""; select.append(option); for (const choice of choices) { const item = ui("option", "", choice.label); item.value = choice.ref; select.append(item); } select.value = choices.some(choice => choice.ref === current) ? current : ""; }
function referenceStatus(references: FormReferences): string { return ({ loading: "引用列表读取中；已有值保留。", ready: "选择实际对象后才修改引用，也可保留准确地址。", partial: "仅显示已知引用对象，列表尚不完整；已有引用保留。", unavailable: "引用列表不可用；已有引用保留，可输入准确地址。" })[references.state]; }
async function loadFormReferences(host: PanelHost, root: string, references: FormReferences, container: HTMLElement, accepts = (): boolean => true): Promise<void> {
  references.state = "loading"; const results = await Promise.allSettled([host.request<{ nodes: { ref: string; displayLabel: string }[]; complete: boolean; diagnostics?: unknown[] }>("/api/data/references", { root }), host.request<{ tags: { tagRef: string; tagId: string; displayName: string; parentTagId: string | null }[]; complete: boolean; warnings?: unknown[] }>("/api/navigation/tags", { root })]); if (!accepts()) return; references.messages = []; references.tagTree = undefined;
  if (results[0].status === "fulfilled") { references.nodes = results[0].value.nodes.map(node => ({ ref: node.ref, label: `${node.displayLabel}（文件名回退）` })); results[0].value.diagnostics?.forEach(item => references.messages.push(diagnosticText(item))); } else references.messages.push("笔记引用列表不可用，已有引用保留。");
  if (results[1].status === "fulfilled") { try { references.tagTree = new TagTree(results[1].value.tags.map(tag => ({ ...tag, reference: tag.tagRef })), results[1].value.complete === true); } catch (error) { references.messages.push(String(error)); } results[1].value.warnings?.forEach(item => references.messages.push(diagnosticText(item))); } else references.messages.push("标签注册表不可用，已有引用保留。");
  references.state = results.every(result => result.status === "rejected") ? "unavailable" : results.some(result => result.status === "rejected") || results.some(result => result.status === "fulfilled" && "complete" in result.value && result.value.complete === false) ? "partial" : "ready";
  for (const select of container.querySelectorAll<HTMLSelectElement>("select[data-ref-kind]")) populateReferenceChoices(select, references);
  for (const picker of container.querySelectorAll<HTMLElement>(".data-tag-reference-picker")) refreshTagPickers.get(picker)?.();
  for (const message of container.querySelectorAll<HTMLElement>(".data-reference-status")) message.textContent = referenceStatus(references);
}

/** Recursive form edits a local value. Unknown record members survive unchanged. */
export function valueEditor(shape: Shape, initial: NativeValue, schema: DataSchema, changed: (value: NativeValue) => void, label: string, depth = 0, references?: FormReferences, binding?: ValueInputBinding): HTMLElement {
  const retire = () => binding?.session.invalidate(binding.slot, binding.path);
  const child = (part: import("./author-input").ValuePath) => binding ? { ...binding, path: [...binding.path, part] } : undefined;
  const host = ui("div", "native-value-editor"); let value = cloneValue(initial); let resolved: Shape;
  try { resolved = resolveShape(shape, schema); } catch (error) { host.append(ui("p", "data-warning", String(error))); return host; }
  if (depth > 18) { host.append(ui("p", "data-warning", "嵌套过深，请在源码中编辑。")); return host; }
  if (value.kind === "none") { host.append(ui("span", "data-empty", "明确为空（none）"), action("填写值", () => { retire(); value = emptyNativeValue(resolved, schema); changed(value); host.replaceChildren(valueEditor(resolved, value, schema, changed, label, depth, references, binding)); })); return host; }
  if (resolved.kind === "union") {
    let branch = unionVariant(resolved, value, schema); const select = ui("select", "data-input");
    const absent = ui("option", "", "选择值的分支…"); absent.value = "-1"; select.append(absent);
    resolved.variants.forEach((variant, index) => { const candidate = resolveShape(variant, schema); const description = candidate.kind === "record" ? Object.keys(candidate.members).slice(0, 3).map(key => fieldNames[key] ?? key).join(" / ") : candidate.kind === "scalar" ? candidate.semantic : candidate.kind; const option = ui("option", "", `${index + 1}. ${description}`); option.value = String(index); select.append(option); }); select.value = String(branch); host.append(labelFor(`${label} · 值类型`, select), select);
    const body = ui("div", "native-union-body"); host.append(body);
    const show = (): void => { body.replaceChildren(); if (branch >= 0) body.append(valueEditor(resolved.variants[branch], value, schema, next => { value = next; changed(next); }, label, depth + 1, references, binding)); else body.append(ui("p", "data-warning", "当前值不能唯一确定分支，原值保留。")); };
    select.onchange = () => { if (select.isConnected === false) return; const next = Number(select.value); if (next < 0) return; try { retire(); value = switchUnionValue(resolved, value, next, schema); branch = next; changed(value); show(); } catch (error) { select.value = String(branch); body.append(ui("p", "data-warning", error instanceof Error ? error.message : String(error))); } }; show(); return host;
  }
  if (resolved.kind === "record") {
    if (value.kind !== "record") { host.append(ui("p", "data-warning", "值的形状与 schema 不符，原值保留。")); return host; }
    const record = ui("div", "native-record"); host.append(record);
    const render = (): void => {
      record.replaceChildren();
      for (const [key, member] of Object.entries(resolved.members)) {
        const present = nativeMember(value, key); const row = ui("div", "native-member"); const heading = ui("div", "native-member-heading"); heading.append(ui("span", "", `${fieldNames[key] ?? key}${member.required ? " *" : ""}`)); row.append(heading);
        if (!present) { heading.append(ui("span", "data-empty", "未填写"), action("添加", () => { retire(); value = replaceNativeMember(value, key, emptyNativeValue(member.shape, schema)); changed(value); render(); }, "data-button compact")); }
        else {
          if (!member.required) heading.append(action("移除", () => { retire(); value = replaceNativeMember(value, key, undefined); changed(value); render(); }, "data-button compact"));
          if (member.nullable && present.kind !== "none") heading.append(action("设为 none", () => { retire(); value = replaceNativeMember(value, key, { kind: "none" }); changed(value); render(); }, "data-button compact"));
          row.append(valueEditor(member.shape, present, schema, next => { value = replaceNativeMember(value, key, next); changed(value); }, `${label} · ${fieldNames[key] ?? key}`, depth + 1, references, child({ kind: "member", key })));
        }
        record.append(row);
      }
      if (value.kind === "record") for (const unknown of value.value.filter(item => !resolved.members[item.key])) { const row = ui("div", "native-unknown"); row.append(ui("strong", "", `${unknown.key} · 未绑定，只读`), ui("p", "", formatNativeValue(unknown.value))); record.append(row); }
    }; render(); return host;
  }
  if (resolved.kind === "list") {
    if (value.kind !== "array") { host.append(ui("p", "data-warning", "值不是数组，原值保留。")); return host; }
    const items = ui("div", "native-list"); host.append(items);
    const render = (): void => {
      items.replaceChildren(); if (value.kind !== "array") return;
      if (!value.value.length) items.append(ui("span", "data-empty", "空数组（0 项）"));
      value.value.forEach((item, index) => {
        const row = ui("fieldset", "native-list-item"); row.append(ui("legend", "", `第 ${index + 1} 项`)); const controls = ui("div", "native-list-controls");
        const up = action("上移", () => { retire(); value = moveOrderedValue(value, index, index - 1); changed(value); render(); }, "data-button compact"); up.disabled = index === 0;
        const down = action("下移", () => { retire(); value = moveOrderedValue(value, index, index + 1); changed(value); render(); }, "data-button compact"); down.disabled = value.kind !== "array" || index === value.value.length - 1;
        const remove = action("移除", () => { if (value.kind !== "array") return; retire(); value = { kind: "array", value: value.value.filter((_, ordinal) => ordinal !== index) }; changed(value); render(); }, "data-button compact"); remove.disabled = value.kind !== "array" || value.value.length <= resolved.min;
        controls.append(up, down, remove); row.append(controls, valueEditor(resolved.items, item, schema, next => { if (value.kind !== "array") return; const nextItems = [...value.value]; nextItems[index] = next; value = { kind: "array", value: nextItems }; changed(value); }, `${label} 第 ${index + 1} 项`, depth + 1, references, child({ kind: "index", index }))); items.append(row);
      });
      const add = action("添加一项", () => { if (value.kind !== "array") return; retire(); value = { kind: "array", value: [...value.value, emptyNativeValue(resolved.items, schema)] }; changed(value); render(); }); add.disabled = resolved.max != null && value.value.length >= resolved.max; items.append(add);
    }; render(); return host;
  }
  const scalar = resolved.kind === "scalar" ? resolved.semantic : "enum";
  if (scalar === "bool") {
    const input = ui("input"); input.type = "checkbox"; input.checked = value.kind === "bool" && value.value; const control = labelFor(label, input); control.prepend(input); host.append(control); input.onchange = () => { if (input.isConnected === false) return; changed({ kind: "bool", value: input.checked }); }; return host;
  }
  if (resolved.kind === "enum" && !resolved.open) {
    const input = ui("select", "data-input"); resolved.values.forEach(optionValue => { const option = ui("option", "", ({ open: "未完成", done: "已完成", cancelled: "已取消" } as Record<string, string>)[optionValue] ?? optionValue); option.value = optionValue; input.append(option); });
    if (value.kind === "text" && !resolved.values.includes(value.value)) { const unknown = ui("option", "", `${value.value}（未登记）`); unknown.value = value.value; input.append(unknown); }
    input.value = formatNativeValue(value); input.setAttribute("aria-label", label); host.append(input); input.onchange = () => { if (input.isConnected === false) return; changed({ kind: "text", value: input.value }); }; return host;
  }
  const input = scalar === "text" || scalar === "nonempty_text" ? ui("textarea", "data-input") : ui("input", "data-input"); if (input instanceof HTMLTextAreaElement) input.rows = 2;
  input.value = formatNativeValue(value); input.setAttribute("aria-label", label); if (input instanceof HTMLInputElement) input.type = "text";
  if (["int", "float", "decimal", "exact_decimal", "duration", "exact_seconds"].includes(scalar)) input.setAttribute("inputmode", "decimal");
  binding?.session.bind(input, binding.slot, binding.path);
  input.placeholder = ({ date: "2026-10-13", time: "12:30:00", local_datetime: "2026-10-13T12:30:00", instant: "2026-10-13T12:30:00+08:00", node_ref: "weftext-node://资料库 ID/笔记 ID", phone: "+86 138…（原样保留）", exact_decimal: "0.30（精确小数）" } as Record<string, string>)[scalar] ?? "";
  input.oninput = () => { try { const next = scalarFromInput(resolved, schema, input.value); input.setCustomValidity(""); changed(next); } catch (error) { input.setCustomValidity(error instanceof Error ? error.message : String(error)); } };
  if (scalar === "tag_ref" && references) { const pickerHost = ui("div", "data-tag-reference-picker"); const renderPicker = () => { pickerHost.replaceChildren(); if (references.tagTree) pickerHost.append(tagPicker(references.tagTree, { label: `${label} · 选择直接标签`, value: input.value, changed: ref => { if (!ref) return; input.value = ref; input.dispatchEvent(new Event("input", { bubbles: true })); } })); else pickerHost.append(ui("p", "data-help", "完整标签结构不可用；已有准确引用保留，可直接输入地址。")); }; refreshTagPickers.set(pickerHost, renderPicker); renderPicker(); host.append(pickerHost); }
  if (scalar === "node_ref" && references) { const picker = ui("select", "data-input data-ref-picker"); picker.dataset.refKind = scalar; populateReferenceChoices(picker, references); picker.setAttribute("aria-label", `${label} · 选择实际笔记`); picker.onchange = () => { if (!picker.value) return; input.value = picker.value; input.dispatchEvent(new Event("input", { bubbles: true })); }; host.append(picker, ui("p", "data-help data-reference-status", referenceStatus(references))); }
  host.append(input); if (["date", "time", "local_datetime", "instant"].includes(scalar)) host.append(ui("p", "data-help", scalar === "instant" ? "瞬间需显式时区偏移；保留输入精度。" : "民用日期 / 时间；不会自动添加本机时区。")); return host;
}

interface FieldDraft { value: NativeValue; changed: boolean; original?: string; binding?: string; rawInputs?: Record<string, string> }
export function nativeFormControls(wrapper: HTMLElement): [string, HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement][] {
  const seen = new Map<string, number>(); return [...wrapper.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>("input, select, textarea")].map(control => { const label = `${control.tagName}:${control.getAttribute("aria-label") ?? ""}`, ordinal = seen.get(label) ?? 0; seen.set(label, ordinal + 1); return [`${label}:${ordinal}`, control]; });
}
export class DataPanel {
  readonly element = ui("aside", "data-panel");
  private schema: DataSchema | null = null;
  private projection: NoteProjection | null = null;
  private basis: DocumentBasis | null = null;
  private generation = 0;
  private drafts = new Map<string, FieldDraft>();
  private filter = "";
  private pinned = new Set<string>();
  private schemaRead: SchemaRead | null = null;
  private draftCache = new Map<string, Map<string, FieldDraft>>();
  private references = emptyReferences();
  private referencesRoot = "";
  private composing = false;
  private pendingRender = false;
  private advancedOpen = false;
  private renderedIdentity: string | null = null;
  private draftIdentity: string | null = null;
  constructor(private readonly host: PanelHost) { this.element.setAttribute("aria-label", "笔记属性"); this.element.append(ui("p", "data-empty", "打开笔记后查看属性。")); this.element.addEventListener("compositionstart", () => { this.composing = true; }); this.element.addEventListener("compositionend", () => { this.composing = false; requestAnimationFrame(() => { if (!this.composing && this.pendingRender) { this.pendingRender = false; this.render(); } }); }); }
  async refresh(): Promise<void> {
    this.rememberInputs();
    const basis = this.host.getDocument(); const generation = ++this.generation;
    const identity = (doc: DocumentBasis) => JSON.stringify([doc.workspaceId, doc.nodeId ?? doc.path]);
    if (!basis) { if (this.basis) this.draftCache.set(identity(this.basis), this.drafts); this.basis = null; this.projection = null; this.draftIdentity = this.renderedIdentity = null; this.element.replaceChildren(ui("p", "data-empty", "打开笔记后查看属性。")); return; }
    if (!this.basis || identity(this.basis) !== identity(basis)) { if (this.basis) this.draftCache.set(identity(this.basis), this.drafts); this.drafts = this.draftCache.get(identity(basis)) ?? new Map(); }
    this.draftIdentity = identity(basis);
    this.basis = { ...basis }; this.element.setAttribute("aria-busy", "true");
    try {
      const schemaResult = await this.host.request<DataSchema | { schema: DataSchema }>("/api/data/schema", { root: basis.root });
      const schema = "schema" in schemaResult ? schemaResult.schema : schemaResult;
      const projectionResult = await this.host.request<NoteProjection | { projection: NoteProjection }>("/api/data/project", { sessionId: basis.sessionId, source: basis.source, serial: basis.serial }); const projection = decodeProjection(projectionResult);
      if (generation !== this.generation || this.host.getDocument()?.source !== basis.source || this.host.getDocument()?.sessionId !== basis.sessionId || this.host.getDocument()?.serial !== basis.serial) return;
      this.schema = schema; this.projection = { ...projection, readOnly: projection.readOnly || this.host.canEdit?.() === false }; this.schemaRead = "schema" in schemaResult ? schemaResult as SchemaRead : null;
      try { this.pinned = new Set(JSON.parse(localStorage.getItem(`weftext-fields:${basis.workspaceId ?? basis.root}`) ?? "[]") as string[]); } catch { this.pinned.clear(); }
      if (this.referencesRoot !== basis.root) { this.referencesRoot = basis.root; this.references = emptyReferences(); } this.render();
      if (basis.workspaceId) void loadFormReferences(this.host, basis.root, this.references, this.element, () => generation === this.generation && this.referencesRoot === basis.root);
    } catch (error) { if (generation === this.generation) this.element.replaceChildren(ui("h2", "", "属性"), ui("p", "data-warning", error instanceof Error ? error.message : String(error)), action("重试读取", () => this.refresh())); }
    finally { if (generation === this.generation) this.element.removeAttribute("aria-busy"); }
  }
  private render(): void {
    if (this.composing) { this.pendingRender = true; return; }
    if (!this.schema || !this.projection || !this.basis) return; const schema = this.schema; const projection = this.projection;
    const active = this.element.contains(document.activeElement) ? document.activeElement as HTMLElement : null, fieldKey = active?.closest<HTMLElement>("[data-field-key]")?.dataset.fieldKey, label = active?.getAttribute("aria-label"), pinFocused = active?.matches("button[aria-pressed]");
    const selection = active instanceof HTMLTextAreaElement || active instanceof HTMLInputElement && active.type === "text" ? { from: active.selectionStart, to: active.selectionEnd } : null;
    const advanced = this.element.querySelector<HTMLDetailsElement>(".data-field-group.advanced"); if (advanced) this.advancedOpen = advanced.open;
    const oldFocus = this.element.contains(document.activeElement) ? (document.activeElement as HTMLInputElement).id : "";
    this.rememberInputs(); this.element.replaceChildren(); const heading = ui("div", "data-panel-heading"); heading.append(ui("h2", "", "属性"), action("刷新", () => this.refresh(), "data-button compact")); this.element.append(heading);
    const status = ui("p", projection.qualification === "qualified" || projection.qualification === "certified" ? "data-help" : "data-warning", `源资格：${projection.qualification}`); this.element.append(status);
    this.renderTypes(schema, projection.types);
    this.renderSchemaSettings();
    const filter = ui("input", "data-input"); filter.type = "search"; filter.value = this.filter; filter.placeholder = "搜索属性…"; filter.id = "property-filter"; filter.setAttribute("aria-label", "搜索属性"); filter.oninput = () => { this.filter = filter.value; this.renderFields(); }; this.element.append(filter);
    this.element.append(ui("div", "data-fields")); this.renderFields();
    if (projection.unknownFields.length) { const unknown = ui("details", "data-unknown-fields"); unknown.append(ui("summary", "", `未知属性（${projection.unknownFields.length}）· 保留原文`)); for (const item of projection.unknownFields) { unknown.append(ui("strong", "", item.key), ui("pre", "", item.raw)); } this.element.append(unknown); }
    if (projection.diagnostics.length) { const diagnostics = ui("details", "data-diagnostics"); diagnostics.append(ui("summary", "", `${projection.diagnostics.length} 项字段提示`)); projection.diagnostics.forEach(item => diagnostics.append(ui("p", "data-warning", diagnosticText(item)))); this.element.append(diagnostics); }
    this.element.append(ui("p", "data-help", "应用属性修改只更新当前草稿；保存后写入文件。任务详情使用同一篇正文。"));
    if (oldFocus) this.element.querySelector<HTMLElement>(`#${CSS.escape(oldFocus)}`)?.focus({ preventScroll: true });
    if (fieldKey) { const field = this.element.querySelector<HTMLElement>(`[data-field-key="${CSS.escape(fieldKey)}"]`), target = label ? field?.querySelector<HTMLElement>(`[aria-label="${CSS.escape(label)}"]`) : pinFocused ? field?.querySelector<HTMLElement>("button[aria-pressed]") : null; target?.focus({ preventScroll: true }); if (selection?.from != null && selection.to != null && (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement)) target.setSelectionRange(selection.from, selection.to); }
  }
  private renderTypes(schema: DataSchema, tokens: string[]): void {
    const basis = this.basis;
    const details = ui("details", "data-type-picker"); details.append(ui("summary", "", `类型：${tokens.map(token => typeNames[token] ?? token).join("、") || "普通笔记"}`));
    const selected = new Set(tokens); const choices = ui("div", "data-type-options");
    const aliases = schema.types.map(type => { const alias = Object.entries(schema.aliases).find(([, id]) => id === type.id)?.[0] ?? type.id; return { alias, id: type.id }; });
    for (const { alias, id } of aliases) { const checkbox = ui("input"); checkbox.type = "checkbox"; checkbox.checked = tokens.includes(alias) || tokens.includes(id); const label = labelFor(typeNames[alias] ?? alias, checkbox); label.prepend(checkbox); choices.append(label); checkbox.onchange = () => { selected.delete(id); selected.delete(alias); if (checkbox.checked) selected.add(alias); warnings.textContent = typeConflicts(schema, [...selected]).join("；"); }; }
    const known = new Set(aliases.flatMap(item => [item.alias, item.id])); const unknown = tokens.filter(token => !known.has(token)); if (unknown.length) choices.append(ui("p", "data-warning", `未登记类型保留：${unknown.join("、")}`));
    const warnings = ui("p", "data-warning"); const apply = action("应用类型到草稿", () => this.applyTypes([...selected], basis)); if (this.projection?.readOnly) { apply.disabled = true; choices.querySelectorAll<HTMLInputElement>("input").forEach(input => { input.disabled = true; }); details.append(ui("p", "data-help", "当前属性只读。")); } details.append(choices, warnings, apply); this.element.append(details);
  }
  private renderFields(): void {
    if (this.composing) { this.pendingRender = true; return; }
    const target = this.element.querySelector<HTMLElement>(".data-fields"); if (!target || !this.schema || !this.projection) return; this.rememberInputs(); const oldAdvanced = target.querySelector<HTMLDetailsElement>(".advanced"); if (oldAdvanced) this.advancedOpen = oldAdvanced.open; target.replaceChildren();
    const schema = this.schema; const declared = fieldsForTypes(schema, this.projection.types); const ids = new Set(declared.map(item => item.field.id));
    const basis = this.basis;
    for (const item of this.projection.fields) if (!ids.has(item.fieldId)) { const field = schema.fields.find(field => field.id === item.fieldId); if (field) declared.push({ field, required: false }); }
    const groups = { pinned: ui("div", "data-field-group"), common: ui("div", "data-field-group"), advanced: ui("details", "data-field-group advanced") }; groups.advanced.append(ui("summary", "", "更多属性"));
    groups.advanced.open = this.advancedOpen; groups.advanced.ontoggle = () => { if (groups.advanced.isConnected) this.advancedOpen = groups.advanced.open; };
    const matching = declared.filter(({ field }) => `${field.key} ${fieldNames[field.key] ?? ""}`.toLowerCase().includes(this.filter.toLowerCase()));
    for (const { field, required } of matching) {
      const projected = this.projection.fields.find(item => item.fieldId === field.id); const cell: Cell = projected?.cell ?? { state: "absent" }; const wrapper = ui("section", "data-property");
      wrapper.dataset.fieldKey = field.key;
      const title = ui("div", "data-property-title"); title.append(ui("h3", "", `${fieldNames[field.key] ?? field.key}${required ? " *" : ""}`));
      const pin = action(this.pinned.has(field.id) ? "取消固定" : "固定", () => { if (this.pinned.has(field.id)) this.pinned.delete(field.id); else this.pinned.add(field.id); localStorage.setItem(`weftext-fields:${this.basis!.workspaceId ?? this.basis!.root}`, JSON.stringify([...this.pinned])); this.renderFields(); }, "data-button compact"); pin.setAttribute("aria-pressed", String(this.pinned.has(field.id))); title.append(pin); wrapper.append(title, ui("p", `data-cell-state ${cell.state}`, cellLabels[cell.state]));
      if (this.projection.readOnly || cell.state === "unavailable" || cell.state === "invalid" || projected?.writable === false) { wrapper.append(ui("p", "data-help", cell.reason || "当前字段没有安全写入资格。请恢复可写会话或查看原生源码。")); if (cell.value) wrapper.append(ui("p", "data-readonly-value", formatNativeValue(cell.value))); if (projected?.raw) wrapper.append(ui("pre", "data-readonly-source", projected.raw)); cell.diagnostics?.forEach(item => wrapper.append(ui("p", "data-warning", diagnosticText(item)))); }
      else if (cell.state === "absent" && !this.drafts.has(field.key)) wrapper.append(action("填写此属性", () => { this.drafts.set(field.key, { value: emptyNativeValue(field.shape, schema), changed: true, original: JSON.stringify(cell) }); this.renderFields(); }));
      else {
        const draft: FieldDraft = this.drafts.get(field.key) ?? { value: cell.value ?? { kind: "none" } as NativeValue, changed: false, original: JSON.stringify(cell) }; const binding = JSON.stringify([field.id, field.shape, schema.definitions]); draft.binding ??= binding; const schemaChanged = draft.binding !== binding; if (!draft.changed) { draft.value = cell.value ?? { kind: "none" }; draft.original = JSON.stringify(cell); draft.binding = binding; } this.drafts.set(field.key, draft); const conflicted = draft.changed && (draft.original !== JSON.stringify(cell) || schemaChanged);
        if (conflicted) wrapper.append(ui("p", "data-warning", "原属性已变化；表单输入仍保留。请核对新属性并放弃旧表单修改后重新填写。"));
        const controls = ui("div", "data-property-actions"); const apply = action("应用到草稿", async () => { if (!wrapper.querySelector<HTMLInputElement>(":invalid")) await this.applyField(field.key, draft.value, basis); else wrapper.querySelector<HTMLInputElement>(":invalid")?.reportValidity(); }, "data-button primary"); apply.disabled = !draft.changed || conflicted;
        if (schemaChanged && draft.changed) wrapper.append(ui("p", "data-warning", "原字段身份或形状已变化；未把输入绑定到同名新字段。请复制保留输入或放弃表单修改后重填。"), ui("pre", "data-readonly-source", JSON.stringify({ value: draft.value, rawInputs: draft.rawInputs }, null, 2)));
        else { wrapper.append(valueEditor(field.shape, draft.value, schema, next => { draft.value = next; draft.changed = true; apply.disabled = conflicted; }, fieldNames[field.key] ?? field.key, 0, this.references)); for (const [key, control] of this.formInputs(wrapper)) { if (draft.rawInputs && key in draft.rawInputs) { control.value = draft.rawInputs[key]; control.dispatchEvent(new Event("input", { bubbles: true })); } } }
        controls.append(apply, action("放弃表单修改", () => { this.drafts.delete(field.key); this.renderFields(); }));
        if (field.nullable && draft.value.kind !== "none") controls.append(action("明确设为 none", () => { draft.value = { kind: "none" }; draft.changed = true; this.renderFields(); })); wrapper.append(controls);
      }
      const group = this.pinned.has(field.id) ? groups.pinned : commonFields.has(field.key) || required || this.filter ? groups.common : groups.advanced; group.append(wrapper);
    }
    if (!matching.length) target.append(ui("p", "data-empty", declared.length ? "没有匹配的属性。" : "此笔记尚未选择含属性的类型。"));
    if (groups.pinned.childElementCount) { groups.pinned.prepend(ui("h3", "data-group-label", "固定属性")); target.append(groups.pinned); } target.append(groups.common); if (groups.advanced.childElementCount > 1) target.append(groups.advanced);
    this.renderedIdentity = this.draftIdentity;
  }
  private formInputs(wrapper: HTMLElement): [string, HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement][] {
    return nativeFormControls(wrapper);
  }
  private rememberInputs(): void {
    const drafts = this.renderedIdentity === this.draftIdentity ? this.drafts : this.renderedIdentity ? this.draftCache.get(this.renderedIdentity) : undefined;
    for (const wrapper of this.element.querySelectorAll<HTMLElement>(".data-property[data-field-key]")) { const draft = drafts?.get(wrapper.dataset.fieldKey!); if (!draft) continue; const raw: Record<string, string> = {}; for (const [key, control] of this.formInputs(wrapper)) if (!control.validity.valid) { raw[key] = control.value; draft.changed = true; } if (Object.keys(raw).length) draft.rawInputs = raw; else if (wrapper.querySelector("input,select,textarea")) delete draft.rawInputs; }
  }
  private async applyField(key: string, value: NativeValue, basis: DocumentBasis | null): Promise<void> { await this.patch("/api/data/edit", { key, path: [], value }, basis); }
  private async applyTypes(types: string[], basis: DocumentBasis | null): Promise<void> { if (this.schema) { const conflicts = typeConflicts(this.schema, types); if (conflicts.length) { this.host.notify(conflicts.join("；"), true); return; } } await this.patch("/api/data/types", { types }, basis); }
  private async patch(route: string, request: Record<string, unknown>, basis: DocumentBasis | null): Promise<void> {
    if (this.composing) { this.host.notify("属性仍在输入组合中，输入保留，请提交组合后再应用。", true); return; }
    if (this.host.canEdit?.() === false) { this.host.notify("当前会话只读或正在输入组合，表单输入保留。", true); return; }
    if (!basis) return;
    if (this.host.getDocument()?.source !== basis.source || this.host.getDocument()?.sessionId !== basis.sessionId || this.host.getDocument()?.serial !== basis.serial) { this.host.notify("正文草稿已变化，请刷新属性后重新应用。表单修改仍保留。", true); return; }
    this.element.setAttribute("aria-busy", "true");
    try { const result = await this.host.request<SourcePatch | { edit: SourcePatch } | { patches: SourcePatch[]; source?: string }>(route, { sessionId: basis.sessionId, source: basis.source, serial: basis.serial, ...request }); await this.host.applyPatch(decodeSourcePatch(result, basis.source), basis); if (typeof request.key === "string") this.drafts.delete(request.key); await this.refresh(); this.host.notify("属性已应用到草稿。可撤销，保存后写入文件。"); }
    catch (error) { this.host.notify(error instanceof Error ? error.message : String(error), true); } finally { this.element.removeAttribute("aria-busy"); }
  }
  private renderSchemaSettings(): void {
    const read = this.schemaRead; if (!read?.canInstall || !this.basis || this.projection?.readOnly) return; const basis = this.basis; const details = ui("details", "schema-settings"); details.append(ui("summary", "", "集中 schema（高级设置）"), ui("p", "data-help", `${read.builtinDefault ? "当前使用内建默认 schema" : "当前已安装 schema"}；安装会影响本库类型与字段校验。`));
    const source = ui("textarea", "data-input schema-source"); source.rows = 10; source.value = read.source; source.setAttribute("aria-label", "集中 schema JSON 配置"); details.append(source); const previewHost = ui("div", "schema-preview"); details.append(previewHost);
    const prepare = action("校验并预览安装", async () => {
      if (this.host.getDocument()?.root !== basis.root || this.host.canEdit?.() === false || this.composing) { this.host.notify("当前资料库或编辑资格已变化，配置输入保留。", true); return; }
      prepare.disabled = true; try { JSON.parse(source.value); const result = await this.host.request<{ planId?: string; operationId?: string; plan?: { planId?: string }; preview?: unknown; diagnostics?: unknown[] }>("/api/data/schema/prepare", { root: basis.root, source: source.value, beforeSource: read.source, beforeRevision: read.revision, operationId: crypto.randomUUID() }); previewHost.replaceChildren(ui("pre", "", diagnosticText(result.preview ?? result.diagnostics ?? result))); const planId = result.planId ?? result.plan?.planId; if (planId && result.operationId) previewHost.append(action("安装已预览的 schema", async () => { if (this.host.getDocument()?.root !== basis.root || this.host.canEdit?.() === false || this.composing) { this.host.notify("当前资料库或编辑资格已变化，原安装计划保留，请返回原库核对。", true); return; } try { await this.host.request("/api/data/schema/install", { operationId: result.operationId, planId }); this.host.environmentChanged?.(basis.root); this.host.notify("集中 schema 已安装。请刷新查询与属性读取新定义。"); await this.refresh(); } catch (error) { this.host.notify(error instanceof Error ? error.message : String(error), true); } }, "data-button primary")); else previewHost.append(ui("p", "data-warning", "尚未取得可执行安装计划，配置未安装。")); }
      catch (error) { previewHost.replaceChildren(ui("p", "data-warning", error instanceof Error ? error.message : String(error))); } finally { prepare.disabled = false; }
    }); details.append(prepare); this.element.append(details);
  }
}

export interface CaptureContext { root: string; parent?: string }
export class QuickCapture {
  readonly element = ui("section", "quick-capture");
  private operationId: string | null = null;
  private pending: Record<string, unknown> | null = null;
  private schema: DataSchema | null = null;
  private references = emptyReferences();
  constructor(private readonly host: PanelHost, private context: CaptureContext) { this.element.setAttribute("aria-label", "快速记录"); }
  async initialize(): Promise<void> { try { const result = await this.host.request<DataSchema | { schema: DataSchema }>("/api/data/schema", { root: this.context.root }); this.schema = "schema" in result ? result.schema : result; this.render(); void loadFormReferences(this.host, this.context.root, this.references, this.element); } catch (error) { this.element.replaceChildren(ui("p", "data-warning", String(error)), action("重试", () => this.initialize())); } }
  updateContext(): void { const template = this.element.querySelector<HTMLButtonElement>(".capture-template-entry"); const help = this.element.querySelector<HTMLElement>(".capture-template-help"); if (!template || !help) return; const document = this.host.getDocument(); template.disabled = !this.host.openTemplates || !document?.nodeId || document.root !== this.context.root; help.textContent = template.disabled ? "打开本库笔记后可进入原生模板向导。当前可按类型创建第一篇笔记。" : "原生模板在同一资料库的模板向导中预览并创建；当前快速记录输入保留。"; }
  private render(): void {
    if (!this.schema) return; const schema = this.schema; const form = ui("form", "capture-form"); form.append(ui("h2", "", "快速记录"));
    const template = action("选择原生模板并预览", async () => { try { await this.host.openTemplates?.(); } catch (error) { this.host.notify(error instanceof Error ? error.message : String(error), true); } }, "data-button capture-template-entry"); form.append(template, ui("p", "data-help capture-template-help"));
    const name = ui("input", "data-input"); name.required = true; name.placeholder = "笔记文件夹名称"; const parent = ui("input", "data-input"); parent.value = this.context.parent ?? this.context.root; parent.required = true;
    const title = ui("input", "data-input"); title.placeholder = "可不填写"; const body = ui("textarea", "data-input capture-body"); body.rows = 7; body.placeholder = "正文（原生 Typst）";
    form.append(labelFor("保存位置", parent), parent, labelFor("名称 *", name), name, labelFor("作者标题（可选）", title), title);
    const type = ui("select", "data-input"); const ordinary = ui("option", "", "普通笔记"); ordinary.value = ""; type.append(ordinary); schema.types.forEach(item => { const alias = Object.entries(schema.aliases).find(([, id]) => id === item.id)?.[0] ?? item.id; const option = ui("option", "", typeNames[alias] ?? alias); option.value = alias; type.append(option); }); form.append(labelFor("笔记类型", type), type);
    const fields = ui("div", "capture-fields"); let values = new Map<string, NativeValue>(); form.append(fields);
    type.onchange = () => {
      fields.replaceChildren(); values = new Map(); for (const { field, required } of fieldsForTypes(schema, type.value ? [type.value] : [])) {
        const container = required || commonFields.has(field.key) ? ui("section", "capture-property") : ui("details", "capture-property"); const heading = container instanceof HTMLDetailsElement ? ui("summary", "", fieldNames[field.key] ?? field.key) : ui("h3", "", `${fieldNames[field.key] ?? field.key}${required ? " *" : ""}`); container.append(heading);
        const addEditor = (): void => { const value = emptyNativeValue(field.shape, schema); values.set(field.key, value); container.append(valueEditor(field.shape, value, schema, next => values.set(field.key, next), fieldNames[field.key] ?? field.key, 0, this.references)); };
        if (required) addEditor(); else { const add = action("填写", () => { add.remove(); addEditor(); }); container.append(add); } fields.append(container);
      }
    };
    form.append(labelFor("正文", body), body); const feedback = ui("p", "data-help", "成功创建并打开真实笔记后才清空输入。当前筛选不包含新笔记时也会打开它。"); feedback.setAttribute("aria-live", "polite"); const submit = ui("button", "data-button primary", "创建并打开"); submit.type = "submit"; form.append(feedback, submit); this.element.replaceChildren(form); this.updateContext();
    form.onsubmit = event => {
      event.preventDefault(); if (!form.reportValidity()) return;
      void (async () => {
        const retry = this.pending !== null;
        submit.disabled = true; this.operationId ??= crypto.randomUUID(); this.pending ??= { root: this.context.root, parent: parent.value, name: name.value, title: title.value || undefined, types: type.value ? [type.value] : [], fields: Object.fromEntries(values), body: body.value, operationId: this.operationId };
        const submitted = this.pending; let receiptReceived = false;
        try {
          const result = await submitOriginal<OriginalFact & { document?: OpenedDocument; documentError?: { message?: string } }>(this.host.request, "/api/data/capture", this.pending, "capture", retry);
          if (result.status && !["saved", "created", "committed", "complete", "success"].includes(result.status)) throw new Error(`创建尚未确认：${result.status}`);
          receiptReceived = true;
          const document = result.document;
          if (!document) throw new Error(`记录已提交；打开尚未完成：${result.documentError?.message ?? "请确认原操作"}`);
          if (this.host.created) await this.host.created(document); else await this.host.openDocument(document.path, document.root ?? this.context.root);
          const unchanged = name.value === submitted.name && parent.value === submitted.parent && (title.value || undefined) === submitted.title && body.value === submitted.body && JSON.stringify(type.value ? [type.value] : []) === JSON.stringify(submitted.types) && JSON.stringify(Object.fromEntries(values)) === JSON.stringify(submitted.fields);
          this.operationId = null; this.pending = null; if (unchanged) { name.value = ""; title.value = ""; body.value = ""; values.clear(); type.value = ""; fields.replaceChildren(); } feedback.className = "data-help"; feedback.textContent = unchanged ? "已创建并打开。可继续记录下一条。" : "提交的记录已创建并打开。等待期间的新输入仍保留，可核对名称后继续记录。"; this.updateContext(); void loadFormReferences(this.host, this.context.root, this.references, this.element);
        } catch (error) {
          if (!receiptReceived && error instanceof TerminatedAttempt) { this.operationId = null; this.pending = null; }
          feedback.className = "data-warning"; feedback.textContent = `${error instanceof Error ? error.message : String(error)}${this.pending ? " 再次创建将查询同一操作结果；原输入保留。" : " 原输入保留。"}`;
        } finally { submit.disabled = false; }
      })();
    };
  }
}
