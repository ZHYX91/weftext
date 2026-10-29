import type { ValuePath } from "./author-input";
import { decodeNativeValue, type NativeValue } from "./native-values";
import { action, emptyNativeValue, labelFor, nativeFormControls, resolveShape, unionVariant, valueEditor, ui, type DataSchema, type SchemaField, type Shape } from "./native-value-form";
import { parseStrictJson } from "./strict-json";
import { message, setUiText, type MessageKey } from "./ui-locale";

export interface ValuePresetScope { libraryId: string; schemaDigest: string }
export interface ValuePreset { id: string; name: string; value: NativeValue }
export interface ValuePresetState {
    contextKey?: string;
    name: string;
    value?: NativeValue;
    rawInputs: Record<string, string>;
    selectedId: string;
}
export const createValuePresetState = (): ValuePresetState => ({ name: "", rawInputs: {}, selectedId: "" });
export interface ValuePresetContext {
    libraryId: string;
    schemaDigest: string;
    fieldId: string;
    /** Schema slots, not a permanent identity for array occurrences. */
    path: ({ kind: "member"; key: string } | { kind: "each" })[];
    action: "replace-scalar" | "append-item";
    binding: string;
}
export interface ValuePresetLease { readonly path: readonly ValuePath[]; readonly fieldValue: string }
/** Array occurrences have no permanent item ID. A nested action owns this exact field cut. */
export function captureValuePresetLease(draft: NativeValue, path: readonly ValuePath[]): ValuePresetLease { return { path: clone(path), fieldValue: canonical(draft) }; }
type Fault = "unsupported" | "invalidValue" | "targetUnavailable" | "maxItems" | "badStorage" | "storageFailed" | "storageChanged" | "lockUnavailable" | "nameInvalid" | "nameExists" | "limit";
export class ValuePresetFault extends Error { constructor(readonly reason: Fault) { super(reason); } }
const fail = (reason: Fault): never => { throw new ValuePresetFault(reason); };
const clone = <T>(value: T): T => structuredClone(value);
const plain = (value: unknown): value is Record<string, unknown> => !!value && typeof value === "object" && !Array.isArray(value);
const closed = (value: Record<string, unknown>, keys: string[]): boolean => Object.keys(value).sort().join(",") === keys.sort().join(",");
const MAX_BYTES = 256 * 1024, MAX_PRESETS = 100;
const referenceSemantics = new Set(["node_ref", "tag_ref", "resource_ref", "schema_ref", "config_revision"]);
const scalarSemantics = new Set(["text", "nonempty_text", "phone", "email", "identifier", "doi", "url", "language_tag", "citation_key", "bool", "int", "float", "decimal", "exact_decimal", "duration", "exact_seconds", "date", "time", "local_datetime", "instant"]);
function canonical(value: unknown): string {
    return JSON.stringify(value, (_key, item) => plain(item) ? Object.fromEntries(Object.entries(item).sort(([a], [b]) => a.localeCompare(b))) : item);
}
function expanded(shape: Shape, schema: DataSchema, depth = 0): unknown {
    if (depth > 24) return fail("unsupported");
    const resolved = resolveShape(shape, schema);
    if (resolved.kind === "list") return { ...resolved, items: expanded(resolved.items, schema, depth + 1) };
    if (resolved.kind === "record") return { ...resolved, members: Object.fromEntries(Object.entries(resolved.members).map(([key, member]) => [key, { ...member, shape: expanded(member.shape, schema, depth + 1) }])) };
    if (resolved.kind === "union") return { ...resolved, variants: resolved.variants.map(item => expanded(item, schema, depth + 1)) };
    return resolved;
}
function branch(shape: Shape, value: NativeValue, schema: DataSchema): Shape {
    const result = resolveShape(shape, schema);
    if (result.kind !== "union") return result;
    const index = unionVariant(result, value, schema);
    if (index < 0) return fail("targetUnavailable");
    return branch(result.variants[index], value, schema);
}
function targetAt(field: SchemaField, schema: DataSchema, draft: NativeValue, path: readonly ValuePath[]): { shape: Shape; value: NativeValue } {
    if (path.length > 24) return fail("unsupported");
    let shape = field.shape, value = draft;
    for (const part of path) {
        shape = branch(shape, value, schema);
        if (part.kind === "member") {
            if (shape.kind !== "record" || !Object.hasOwn(shape.members, part.key) || value.kind !== "record") return fail("targetUnavailable");
            const members = value.value.filter(item => item.key === part.key);
            if (members.length !== 1) return fail("targetUnavailable");
            value = members[0].value; shape = shape.members[part.key].shape;
        } else {
            if (shape.kind !== "list" || value.kind !== "array" || !Number.isSafeInteger(part.index) || part.index < 0 || part.index >= value.value.length) return fail("targetUnavailable");
            value = value.value[part.index]; shape = shape.items;
        }
    }
    return { shape: branch(shape, value, schema), value };
}
export function valuePresetContext(scope: ValuePresetScope, schema: DataSchema, field: SchemaField, draft: NativeValue, path: readonly ValuePath[] = []): ValuePresetContext {
    if (!scope.libraryId || scope.libraryId.length > 4096 || !/^[a-f0-9]{64}$/.test(scope.schemaDigest) || !field.id || !schema.fields.some(item => item.id === field.id && canonical(item) === canonical(field))) return fail("unsupported");
    const target = targetAt(field, schema, draft, path), shape = target.shape;
    if (shape.kind !== "scalar" && shape.kind !== "enum" && shape.kind !== "list") return fail("unsupported");
    const candidate = shape.kind === "list" ? resolveShape(shape.items, schema) : shape;
    if (candidate.kind === "scalar" && !scalarSemantics.has(candidate.semantic)) return fail("unsupported");
    return {
        libraryId: scope.libraryId, schemaDigest: scope.schemaDigest, fieldId: field.id,
        path: path.map(part => part.kind === "member" ? { kind: "member", key: part.key } : { kind: "each" }),
        action: shape.kind === "list" ? "append-item" : "replace-scalar",
        binding: canonical({ field: expanded(field.shape, schema), nullable: field.nullable, target: expanded(shape, schema) }),
    };
}
export function valuePresetStorageKey(context: ValuePresetContext): string { return `weftext-value-presets:v1:${canonical(context)}`; }

const matches = (pattern: RegExp, value: string): boolean => pattern.exec(value)?.[0] === value;
const integer = (value: string): boolean => {
    if (!matches(/^-?(0|[1-9][0-9]*)$/, value) || value.length > 20) return false;
    const number = BigInt(value); return number >= -(1n << 63n) && number < (1n << 63n);
};
function date(value: string): boolean {
    if (!matches(/^\d{4}-\d{2}-\d{2}$/, value)) return false;
    const [year, month, day] = value.split("-").map(Number);
    const days = [31, year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0) ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    return month >= 1 && month <= 12 && day >= 1 && day <= days[month - 1];
}
const time = (value: string): boolean => matches(/^([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]$/, value);
/** Core ExactInstant uses an explicit offset and an ExactDecimal fraction, never Date/binary64. */
function instant(value: string): boolean {
    const parts = /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2}:\d{2})(?:\.(\d+))?(Z|[+-]\d{2}:\d{2})$/.exec(value);
    if (!parts || parts[0] !== value || parts[1].startsWith("0000-") || !date(parts[1]) || !time(parts[2]) || (parts[3]?.length ?? 0) > 4094 || parts[4] === "-00:00") return false;
    return parts[4] === "Z" || Number(parts[4].slice(1, 3)) <= 23 && Number(parts[4].slice(4)) <= 59;
}
function scalarValid(semantic: string, value: NativeValue): boolean {
    if (referenceSemantics.has(semantic)) return false;
    if (semantic === "bool") return value.kind === "bool";
    if (value.kind === "none" || value.kind === "bool" || value.kind === "array" || value.kind === "record") return false;
    const text = value.value;
    if (semantic === "int" || semantic === "duration") return value.kind === semantic && integer(text);
    if (semantic === "float") return value.kind === "float" && matches(/^[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?$/, text) && Number.isFinite(Number(text));
    if (semantic === "decimal") {
        if (value.kind !== "decimal" || !matches(/^[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)$/, text)) return false;
        const digits = text.replace(/^[+-]/, "").replace(".", "");
        return (text.split(".")[1]?.length ?? 0) <= 28 && digits.length <= 60 && BigInt(digits) < (1n << 96n);
    }
    if (semantic === "exact_decimal" || semantic === "exact_seconds") return value.kind === "text" && text.length <= 4096 && matches(/^-?[0-9]+(?:\.[0-9]+)?$/, text);
    if (semantic === "date") return value.kind === "date" && date(text);
    if (semantic === "time") return value.kind === "time" && time(text);
    if (semantic === "local_datetime") return value.kind === "localDatetime" && text.length === 19 && date(text.slice(0, 10)) && text[10] === "T" && time(text.slice(11));
    if (semantic === "instant") return value.kind === "text" && instant(text);
    if (value.kind !== "text") return false;
    if (semantic === "text") return true;
    if (["nonempty_text", "phone", "email", "identifier"].includes(semantic)) return text.length > 0;
    if (semantic === "citation_key") return matches(/^[A-Za-z][A-Za-z0-9_-]*$/, text);
    if (semantic === "doi") return matches(/^10\.[0-9]*\/\S+$/, text);
    if (semantic === "url") return matches(/^(https|http|ftp):\/\/\S+$/, text);
    if (semantic === "language_tag") return matches(/^[A-Za-z0-9]+(?:-[A-Za-z0-9]+)*$/, text);
    return false;
}
/** Shape/scalar candidate checking, not a replacement for Core provider/type constraints. */
export function validValuePreset(shape: Shape, value: NativeValue, schema: DataSchema, nullable = false, depth = 0): boolean {
    if (depth > 24 || JSON.stringify(value).length * 2 > MAX_BYTES / 2) return false;
    if (value.kind === "none") return nullable;
    const resolved = resolveShape(shape, schema);
    if (resolved.kind === "scalar") return scalarValid(resolved.semantic, value);
    if (resolved.kind === "enum") return value.kind === "text" && (resolved.open || resolved.values.includes(value.value));
    if (resolved.kind === "union") {
        const index = unionVariant(resolved, value, schema);
        return index >= 0 && validValuePreset(resolved.variants[index], value, schema, nullable, depth + 1);
    }
    if (resolved.kind === "list") return value.kind === "array" && value.value.length >= resolved.min && (resolved.max === null || value.value.length <= resolved.max) && value.value.every(item => validValuePreset(resolved.items, item, schema, false, depth + 1));
    if (resolved.kind !== "record" || value.kind !== "record") return false;
    const keys = new Set<string>();
    for (const member of value.value) {
        if (keys.has(member.key) || !Object.hasOwn(resolved.members, member.key)) return false;
        keys.add(member.key);
        const binding = resolved.members[member.key];
        if (!validValuePreset(binding.shape, member.value, schema, binding.nullable, depth + 1)) return false;
    }
    return Object.entries(resolved.members).every(([key, member]) => !member.required || keys.has(key));
}
function replaceAt(value: NativeValue, path: readonly ValuePath[], next: NativeValue): NativeValue {
    if (!path.length) return clone(next);
    const [part, ...rest] = path;
    if (part.kind === "index" && value.kind === "array") return { ...value, value: value.value.map((item, index) => index === part.index ? replaceAt(item, rest, next) : item) };
    if (part.kind === "member" && value.kind === "record") return { ...value, value: value.value.map(item => item.key === part.key ? { ...item, value: replaceAt(item.value, rest, next) } : item) };
    return fail("targetUnavailable");
}
export function applyValuePreset(scope: ValuePresetScope, schema: DataSchema, field: SchemaField, draft: NativeValue, preset: NativeValue, expected: ValuePresetContext, path: readonly ValuePath[] = [], lease?: ValuePresetLease): NativeValue {
    if (path.length && (!lease || canonical(lease.path) !== canonical(path) || lease.fieldValue !== canonical(draft))) return fail("targetUnavailable");
    if (canonical(valuePresetContext(scope, schema, field, draft, path)) !== canonical(expected)) return fail("targetUnavailable");
    const target = targetAt(field, schema, draft, path);
    const shape = target.shape.kind === "list" ? target.shape.items : target.shape;
    if (!validValuePreset(shape, preset, schema)) return fail("invalidValue");
    if (target.shape.kind === "list") {
        if (target.value.kind !== "array") return fail("targetUnavailable");
        if (target.shape.max !== null && target.value.value.length >= target.shape.max) return fail("maxItems");
        return replaceAt(draft, path, { kind: "array", value: [...target.value.value, clone(preset)] });
    }
    return replaceAt(draft, path, preset);
}
export interface ValuePresetStorage { getItem(key: string): string | null; setItem(key: string, value: string): void }
export interface ValuePresetMutationLock { run<T>(name: string, operation: () => T | Promise<T>): Promise<T> }
function mutationLocks(): ValuePresetMutationLock | null {
    const locks = typeof navigator === "undefined" ? undefined : navigator.locks;
    return locks?.request ? { run: async (name, operation) => await locks.request(name, { mode: "exclusive" }, operation) } : null;
}
/** Supported producers serialize under the same Web Lock, then compare the original
 * storage token. External manual storage writes are not a participating producer. */
export class ValuePresetStore {
    readonly key: string;
    private raw: string | null = null;
    private blocked = false;
    presets: ValuePreset[] = [];
    constructor(readonly context: ValuePresetContext, private readonly storage: ValuePresetStorage, private readonly accepts: (value: NativeValue) => boolean, private readonly locks: ValuePresetMutationLock | null = mutationLocks()) {
        this.key = valuePresetStorageKey(context);
        try { this.raw = storage.getItem(this.key); } catch { return fail("storageFailed"); }
        this.presets = this.decode(this.raw);
    }
    get canWrite(): boolean { return !!this.locks && !this.blocked; }
    private decode(raw: string | null): ValuePreset[] {
        if (raw === null) return [];
        try {
            if (raw.length * 2 > MAX_BYTES) return fail("badStorage");
            const value = parseStrictJson(raw);
            if (!plain(value) || !closed(value, ["version", "context", "presets"]) || value.version !== 1 || canonical(value.context) !== canonical(this.context) || !Array.isArray(value.presets) || value.presets.length > MAX_PRESETS) return fail("badStorage");
            const ids = new Set<string>(), names = new Set<string>();
            return value.presets.map(item => {
                if (!plain(item) || !closed(item, ["id", "name", "value"]) || typeof item.id !== "string" || !/^[a-f0-9-]{36}$/.test(item.id) || ids.has(item.id) || typeof item.name !== "string" || item.name !== item.name.trim() || !item.name || item.name.length > 80 || names.has(item.name)) return fail("badStorage");
                const decoded = decodeNativeValue(item.value); if (!decoded || !this.accepts(decoded)) return fail("badStorage");
                ids.add(item.id); names.add(item.name); return { id: item.id, name: item.name, value: decoded };
            });
        } catch { return fail("badStorage"); }
    }
    private async write(presets: ValuePreset[], validate?: () => boolean): Promise<void> {
        if (this.blocked) return fail("storageFailed");
        if (!this.locks) return fail("lockUnavailable");
        const expectedRaw = this.raw;
        const captured = clone(presets), raw = JSON.stringify({ version: 1, context: this.context, presets: captured });
        if (raw.length * 2 > MAX_BYTES || captured.length > MAX_PRESETS) return fail("limit");
        await this.locks.run(this.key, () => {
            if (this.blocked) return fail("storageFailed");
            try {
                if (this.raw !== expectedRaw || this.storage.getItem(this.key) !== expectedRaw) return fail("storageChanged");
                if (validate && !validate()) return fail("targetUnavailable");
                this.storage.setItem(this.key, raw);
            } catch (error) { if (error instanceof ValuePresetFault) throw error; return fail("storageFailed"); }
            try { if (this.storage.getItem(this.key) !== raw) throw new Error(); }
            catch { this.blocked = true; return fail("storageFailed"); }
            this.raw = raw; this.presets = clone(captured);
        });
    }
    async save(name: string, value: NativeValue, validate?: () => boolean): Promise<ValuePreset> {
        name = name.trim();
        if (!name || name.length > 80) return fail("nameInvalid");
        if (this.presets.some(item => item.name === name)) return fail("nameExists");
        if (!this.accepts(value)) return fail("invalidValue");
        const preset = { id: crypto.randomUUID(), name, value: clone(value) };
        await this.write([...this.presets, preset], validate); return preset;
    }
    async remove(id: string, validate?: () => boolean): Promise<void> {
        if (!this.presets.some(item => item.id === id)) return fail("storageChanged");
        await this.write(this.presets.filter(item => item.id !== id), validate);
    }
    assertCurrent(): void {
        try { if (this.storage.getItem(this.key) !== this.raw) return fail("storageChanged"); }
        catch (error) { if (error instanceof ValuePresetFault) throw error; return fail("storageFailed"); }
    }
}

export interface ValuePresetsOptions {
    scope: ValuePresetScope;
    schema: DataSchema;
    field: SchemaField;
    path?: readonly ValuePath[];
    state?: ValuePresetState;
    storage?: ValuePresetStorage;
    locks?: ValuePresetMutationLock | null;
    getDraft(): NativeValue;
    setDraft(value: NativeValue): void;
    isCurrent(): boolean;
    /** Current author raw validity, IME, basis, permission, conflict and freeze guard. */
    canApply(): boolean;
}
export function mountValuePresets(host: HTMLElement, options: ValuePresetsOptions): { refresh(): void; dispose(): void } {
    host.dataset.valuePresets = "";
    const state = options.state ?? createValuePresetState(), path = clone(options.path ?? []);
    const lease = path.length ? captureValuePresetLease(options.getDraft(), path) : undefined;
    let live = true, context: ValuePresetContext, store: ValuePresetStore | undefined, candidateShape: Shape;
    let interaction = 0, mutating = false;
    const composing = new Set<EventTarget>();
    const status = ui("p", "data-help"); status.setAttribute("role", "status");
    const fault = (error: unknown): void => { setUiText(status, message(`valuePresets.${error instanceof ValuePresetFault ? error.reason : "storageFailed"}` as MessageKey)); };
    const current = (): boolean => live && host.isConnected && !host.closest("[hidden],[inert]") && options.isCurrent();
    try {
        context = valuePresetContext(options.scope, options.schema, options.field, options.getDraft(), path);
        const target = targetAt(options.field, options.schema, options.getDraft(), path);
        candidateShape = target.shape.kind === "list" ? target.shape.items : target.shape;
        const key = valuePresetStorageKey(context);
        if (state.contextKey !== key) Object.assign(state, createValuePresetState(), { contextKey: key, value: undefined });
        if (!state.value) state.value = context.action === "replace-scalar" && validValuePreset(candidateShape, target.value, options.schema) ? clone(target.value) : emptyNativeValue(candidateShape, options.schema);
    } catch (error) {
        host.replaceChildren(status); fault(error); return { refresh() {}, dispose() { live = false; } };
    }
    const details = ui("details", "data-value-presets");
    details.append(ui("summary", "data-label", message("valuePresets.title")), ui("p", "data-help", message("valuePresets.help")), ui("p", "data-help", message(context.action === "append-item" ? "valuePresets.appendHelp" : "valuePresets.replaceHelp")));
    const name = ui("input", "data-input"); name.type = "text"; name.maxLength = 80; name.value = state.name;
    name.oninput = () => { if (current()) { state.name = name.value; interaction++; } };
    let restoring = true;
    const editor = valueEditor(candidateShape, state.value!, options.schema, value => {
        if (restoring || current()) { state.value = clone(value); if (!restoring) { state.rawInputs = {}; interaction++; } }
    }, message("valuePresets.value"), { fieldId: options.field.id, path });
    const captureRaw = (): void => { state.rawInputs = Object.fromEntries(nativeFormControls(editor).map(([key, control]) => [key, control.value])); };
    const captureInput = (): void => { if (current()) { captureRaw(); interaction++; } };
    editor.addEventListener("input", captureInput); editor.addEventListener("change", captureInput);
    const rawCut = new Map(Object.entries(state.rawInputs));
    while (rawCut.size) {
        // A union choice may rebuild its branch. Never replay into a stale snapshot.
        const entry = nativeFormControls(editor).find(([key]) => rawCut.has(key));
        if (!entry) break;
        const [key, control] = entry, raw = rawCut.get(key)!; rawCut.delete(key);
        if (control instanceof HTMLSelectElement && control.value === raw) continue;
        control.value = raw; control.dispatchEvent(new Event(control instanceof HTMLSelectElement ? "change" : "input", { bubbles: true }));
    }
    restoring = false;
    const select = ui("select", "data-input");
    select.onchange = () => { if (current()) { state.selectedId = select.value; interaction++; } };
    const selectOptions = (): void => {
        const empty = ui("option", "", message("valuePresets.choose")); empty.value = ""; select.replaceChildren(empty);
        for (const preset of store?.presets ?? []) { const option = ui("option", "", preset.name); option.value = preset.id; select.append(option); }
        select.value = store?.presets.some(item => item.id === state.selectedId) ? state.selectedId : "";
    };
    const eligible = (author = false): boolean => {
        if (!current() || author && !options.canApply() || composing.size || details.querySelector(":invalid")) { setUiText(status, message("valuePresets.guard")); return false; }
        try { if (canonical(valuePresetContext(options.scope, options.schema, options.field, options.getDraft(), path)) !== canonical(context)) return fail("targetUnavailable"); }
        catch (error) { fault(error); return false; }
        return true;
    };
    const save = action(message("valuePresets.save"), async () => {
        if (mutating || !eligible() || !store) return;
        const pendingStore = store;
        try {
            if (!validValuePreset(candidateShape, state.value!, options.schema)) return fail("invalidValue");
            captureRaw();
            const version = interaction, capturedName = name.value, candidate = clone(state.value!), raw = canonical(state.rawInputs);
            mutating = true; updateButtons(); setUiText(status, "");
            const preset = await pendingStore.save(capturedName, candidate, () => current() && pendingStore === store && interaction === version
                && name.value === capturedName && canonical(state.value) === canonical(candidate) && canonical(state.rawInputs) === raw && eligible());
            if (current()) { state.selectedId = preset.id; selectOptions(); setUiText(status, message("valuePresets.saved")); }
        } catch (error) { if (current()) fault(error); }
        finally { mutating = false; if (current()) updateButtons(); }
    });
    const apply = action(message(context.action === "append-item" ? "valuePresets.append" : "valuePresets.replace"), () => {
        if (mutating || !eligible(true) || !store) return;
        const preset = store.presets.find(item => item.id === select.value);
        if (!preset) return;
        try {
            store.assertCurrent();
            const next = applyValuePreset(options.scope, options.schema, options.field, options.getDraft(), preset.value, context, path, lease);
            captureRaw(); options.setDraft(next);
            if (live && host.isConnected) setUiText(status, message(context.action === "append-item" ? "valuePresets.appliedAppend" : "valuePresets.appliedReplace"));
        } catch (error) { fault(error); }
    });
    const remove = action(message("valuePresets.delete"), async () => {
        if (mutating || !eligible() || !store) return;
        const pendingStore = store, version = interaction, id = select.value;
        try {
            mutating = true; updateButtons(); setUiText(status, "");
            await pendingStore.remove(id, () => current() && pendingStore === store && interaction === version && select.value === id && eligible());
            if (current()) { state.selectedId = ""; selectOptions(); setUiText(status, message("valuePresets.removed")); }
        } catch (error) { if (current()) fault(error); }
        finally { mutating = false; if (current()) updateButtons(); }
    });
    const updateButtons = (): void => {
        details.setAttribute("aria-busy", String(mutating));
        save.disabled = remove.disabled = mutating || !store?.canWrite;
        apply.disabled = mutating || !store;
    };
    const start = (event: CompositionEvent): void => { if (event.target) { composing.add(event.target); interaction++; } };
    const end = (event: CompositionEvent): void => { if (event.target) { composing.delete(event.target); interaction++; } };
    details.addEventListener("compositionstart", start); details.addEventListener("compositionend", end);
    details.append(labelFor(message("valuePresets.name"), name), name, ui("p", "data-label", message("valuePresets.value")), editor, save, labelFor(message("valuePresets.available"), select), select, apply, remove, status);
    host.replaceChildren(details);
    try {
        store = new ValuePresetStore(context, options.storage ?? localStorage, value => validValuePreset(candidateShape, value, options.schema), options.locks);
        selectOptions();
        if (!store.canWrite) fault(new ValuePresetFault("lockUnavailable"));
    } catch (error) { fault(error); }
    updateButtons();
    return {
        refresh() { if (current()) { selectOptions(); updateButtons(); } },
        dispose() { if (!live) return; captureRaw(); state.name = name.value; live = false; composing.clear(); details.removeEventListener("compositionstart", start); details.removeEventListener("compositionend", end); editor.removeEventListener("input", captureInput); editor.removeEventListener("change", captureInput); save.disabled = apply.disabled = remove.disabled = true; },
    };
}
