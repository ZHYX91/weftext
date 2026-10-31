import { isBuiltinField, schemaPropertyPresentation } from "./builtin-property-labels";
import { appendBirthEvent, birthEvents, birthTimeIsLeapMonth, birthTimePrecision, birthTimeText, selectBirthEvent } from "./birthday";
export { schemaFieldLabel } from "./builtin-property-labels";
import {textareaSourceDraft} from "./writing";
import {decodeCoreDescription,coreDescriptionUiText} from "./ui-diagnostics";
/** Single native schema form implementation, shared by DataPanel, queries and Capture. */
import { nativeControlAddress, registerNativeControl, type NativeControlIdentity, type NativeControlBinding } from "./native-control-identity";
export { nativeControlKey, nativeControlAddress, registerNativeControl, type NativeControlAddress, type NativeControlIdentity, type NativeControlBinding } from "./native-control-identity";
import { uiDisplayParameter, message as uiMessage, displayMessage, setUiValidity, setUiText, setUiAttr, uiTextValue, joinUiText, errorUiText, UiError, type UiText } from "./ui-locale";
import type { ValuePath } from "./author-input";
import type { PanelFieldInputFactory } from "./panel-field-input";
import type { NativeValue } from "./native-values";
import type { SourceInputBasis } from "./source-input-basis";
export type { NativeValue } from "./native-values";
import type { OpenedDocument } from "./types";
import { TagTree } from "./tag-tree";
import { tagPicker } from "./tag-picker";
export type Shape = {
    kind: "scalar";
    semantic: string;
} | {
    kind: "enum";
    values: string[];
    open: boolean;
} | {
    kind: "list";
    items: Shape;
    min: number;
    max: number | null;
} | {
    kind: "record";
    members: Record<string, {
        shape: Shape;
        required: boolean;
        nullable: boolean;
    }>;
    unknown: string;
} | {
    kind: "union";
    variants: Shape[];
    discriminator: null | {
        kind: string;
        keys?: string[];
        key?: string;
        present?: number;
        absent?: number;
    };
} | {
    kind: "use";
    name: string;
};
export interface SchemaField {
    id: string;
    key: string;
    shape: Shape;
    nullable: boolean;
    default?: {
        kind: string;
        source?: string;
    };
}
export interface SchemaType {
    id: string;
    fields: {
        id: string;
        required: boolean;
    }[];
    requires: string[];
    conflicts: string[];
}
export interface DataSchema {
    definitions: Record<string, Shape>;
    fields: SchemaField[];
    types: SchemaType[];
    aliases: Record<string, string>;
    series_policies?: {
        key: string;
        kinds: string[];
        scope: {
            kind: string;
        };
        multiplicity: string;
    }[];
}
interface ReferenceChoice {
    ref: string;
    label: UiText;
}
export interface FormReferences {
    nodes: ReferenceChoice[];
    tagTree?: TagTree;
    state: "loading" | "ready" | "partial" | "unavailable";
    messages: UiText[];
}
export interface DocumentBasis extends SourceInputBasis {
    dirty?: boolean;
    /** Present only when this exact input identity, source and serial have a Core-qualified author Draft or saved-view cut. */
    inputRevision?: number;
}
export interface SourcePatch {
    from: number;
    to: number;
    text: string;
}
export interface SchemaRead {
    schema: DataSchema;
    source: string;
    /** Core digest of the complete installed schema semantics, including constraints. */
    semanticDigest: string;
    roleRef?: string;
    revision: string | null;
    builtinDefault: boolean;
    canInstall: boolean;
}
export function decodeSourcePatch(result: SourcePatch | {
    edit: SourcePatch;
} | {
    patches: SourcePatch[];
    source?: string;
}, source: string): SourcePatch {
    if ("edit" in result)
        return result.edit;
    if (!("patches" in result))
        return result;
    const patches = [...result.patches].sort((a, b) => a.from - b.from);
    if (!patches.length)
        return { from: 0, to: 0, text: "" };
    let prior = -1;
    for (const patch of patches) {
        if (!Number.isInteger(patch.from) || !Number.isInteger(patch.to) || patch.from < 0 || patch.to < patch.from || patch.to > source.length || patch.from < prior)
            throw new UiError(uiMessage("dataValidation.patchRangeInvalid"));
        prior = patch.to;
    }
    const from = patches[0].from;
    const to = patches.at(-1)!.to;
    let text = "";
    let cursor = from;
    for (const patch of patches) {
        text += source.slice(cursor, patch.from) + patch.text;
        cursor = patch.to;
    }
    text += source.slice(cursor, to);
    if (result.source !== undefined && source.slice(0, from) + text + source.slice(to) !== result.source)
        throw new UiError(uiMessage("dataValidation.patchReadbackMismatch"));
    return { from, to, text };
}
export interface PanelHost {
    fieldInputs?: PanelFieldInputFactory;
    authorInputs?(basis: DocumentBasis): import("./author-input").AuthorInputSession | undefined;
    canEdit?(): boolean;
    request<T>(path: string, body?: unknown): Promise<T>;
    requestBlob?(path: string, body: unknown): Promise<Blob>;
    getDocument(): DocumentBasis | null;
    applyPatch(patch: SourcePatch, basis: DocumentBasis): Promise<void>;
    openDocument(path: string, root?: string): Promise<void>;
    openResolved?(document: OpenedDocument): Promise<void>;
    /** Receipt from this exact navigation attempt, including a retained Draft
     * comparison; matching a later current node is not an acceptance proof. */
    openResolvedConfirmed?(document: OpenedDocument): Promise<boolean>;
    /** Only a fresh result never handed to the actual document owner. This
     * releases its disposable Session, not an author/native close proof. */
    releaseOpenedSession?(sessionId: string): Promise<void>;
    openTemplates?(): void | Promise<void>;
    notify(message: UiText, error?: boolean): void;
    /** A confirmed Core mutation changed the original library's Query environment. */
    environmentChanged?(root: string): void;
    prepareAction?(): Promise<void>;
    /** A create success must be opened by the host, using its returned Core session. */
    created?(result: unknown, operationId: string | undefined): Promise<void>;
    /** Explicitly acknowledges the Core save of a query and its owning note. */
    savedDefinition?(result: unknown, basis: DocumentBasis): Promise<void>;
    /** Reconcile one committed original Core operation without replacing later input. */
    applyOperationReceipt?(receipt: unknown, basis: DocumentBasis): Promise<void>;
}
export const fieldNames: Record<string, UiText> = { reminder: uiMessage("reminder.label"), status: uiMessage("field.status"), due: uiMessage("field.due"), start: uiMessage("field.start"), priority: uiMessage("field.priority"), project: uiMessage("field.project"), completed_at: uiMessage("field.completedAt"), title: uiMessage("queryColumn.title"), names: uiMessage("field.names"), phones: uiMessage("field.phones"), emails: uiMessage("field.emails"), addresses: uiMessage("field.addresses"), websites: uiMessage("field.websites"), accounts: uiMessage("field.accounts"), tags: uiMessage("field.tags"), publication: uiMessage("field.publication"), contributors: uiMessage("field.contributors"), resources: uiMessage("field.resources"), note: uiMessage("field.note"), notes: uiMessage("field.note"), number: uiMessage("field.number"), value: uiMessage("field.value"), text: uiMessage("field.text"), ref: uiMessage("field.nodeReference"), kind: uiMessage("field.kind"), role: uiMessage("field.role"), agent: uiMessage("field.agent"), credited_as: uiMessage("field.creditedAs"), date: uiMessage("field.date"), amount: uiMessage("field.amount"), currency: uiMessage("field.currency"), unit: uiMessage("field.unit"), during: uiMessage("field.validity"), source: uiMessage("field.source"), validity: uiMessage("field.validity"), period: uiMessage("field.period"), range: uiMessage("field.range"), event: uiMessage("field.event"), dependency: uiMessage("field.dependency") };
export const typeNames: Record<string, UiText> = { person: uiMessage("type.person"), organization: uiMessage("type.organization"), org: uiMessage("type.organization"), task: uiMessage("type.task"), project: uiMessage("field.project"), work: uiMessage("type.work"), paper: uiMessage("type.paper"), event: uiMessage("field.event"), period: uiMessage("type.periodNote"), "period-note": uiMessage("type.periodNote"), range: uiMessage("type.rangeNote"), "range-note": uiMessage("type.rangeNote") };
export const cellLabels = { value: uiMessage("nativeValue.hasValue"), absent: uiMessage("queryValue.absent"), null: uiMessage("nativeValue.explicitNone"), invalid: uiMessage("queryValue.invalid"), unavailable: uiMessage("nativeValue.unavailableComputed") };
export const commonFields = new Set(["title", "names", "status", "due", "reminder", "project", "priority", "phones", "emails", "life_events", "tags", "publication", "period", "range"]);
export function resolveShape(shape: Shape, schema: DataSchema, seen = new Set<string>()): Shape {
    if (shape.kind !== "use")
        return shape;
    if (seen.has(shape.name) || !schema.definitions[shape.name])
        throw new UiError(displayMessage("dataValidation.schemaExpansionFailed", { p0: uiDisplayParameter(shape.name) }));
    return resolveShape(schema.definitions[shape.name], schema, new Set([...seen, shape.name]));
}
export function fieldsForTypes(schema: DataSchema, tokens: string[]): {
    field: SchemaField;
    required: boolean;
}[] {
    const required = new Map<string, boolean>();
    const visited = new Set<string>();
    const visit = (token: string): void => {
        const id = schema.aliases[token] ?? token;
        if (visited.has(id))
            return;
        visited.add(id);
        const type = schema.types.find(item => item.id === id);
        if (!type)
            return;
        type.requires.forEach(visit);
        type.fields.forEach(item => required.set(item.id, !!item.required || !!required.get(item.id)));
    };
    tokens.forEach(visit);
    return schema.fields.filter(field => required.has(field.id)).map(field => ({ field, required: required.get(field.id)! }));
}
export function typeConflicts(schema: DataSchema, tokens: string[]): UiText[] {
    const expanded = new Set<string>();
    const visit = (token: string): void => {
        const id = schema.aliases[token] ?? token;
        if (expanded.has(id))
            return;
        expanded.add(id);
        schema.types.find(type => type.id === id)?.requires.forEach(visit);
    };
    tokens.forEach(visit);
    return schema.types.filter(type => expanded.has(type.id)).flatMap(type => type.conflicts.filter(id => expanded.has(id)).map(id => displayMessage("dataValidation.typeConflict", { p0: uiDisplayParameter(type.id), p1: uiDisplayParameter(id) })));
}
export function nativeMember(value: NativeValue | undefined, key: string): NativeValue | undefined { return value?.kind === "record" ? value.value.find(item => item.key === key)?.value : undefined; }
export function replaceNativeMember(value: NativeValue, key: string, replacement: NativeValue | undefined): NativeValue {
    if (value.kind !== "record")
        throw new UiError(uiMessage("dataValidation.recordValueRequired"));
    const items = value.value.map(item => ({ ...item }));
    const index = items.findIndex(item => item.key === key);
    if (replacement === undefined) {
        if (index >= 0)
            items.splice(index, 1);
    }
    else if (index >= 0)
        items[index] = { key, value: replacement };
    else
        items.push({ key, value: replacement });
    return { kind: "record", value: items };
}
export function moveOrderedValue(value: NativeValue, from: number, to: number): NativeValue {
    if (value.kind !== "array" || !Number.isSafeInteger(from) || !Number.isSafeInteger(to) || from < 0 || to < 0 || from >= value.value.length || to >= value.value.length || from === to)
        return value;
    const items = [...value.value];
    const [item] = items.splice(from, 1);
    items.splice(to, 0, item);
    return { kind: "array", value: items };
}
/** A display position is one-based, never a scalar native value or a new path. */
export function orderedMoveTarget(raw: string, length: number): number | null {
    if (!Number.isSafeInteger(length) || length < 1 || !/^[1-9][0-9]*$/.test(raw)) return null;
    const position = Number(raw);
    return Number.isSafeInteger(position) && position <= length ? position - 1 : null;
}
/** A drop gap addresses this exact ordered list, before removing its source item. */
export function orderedDropTarget(from: number, gap: number, length: number): number | null {
    if (!Number.isSafeInteger(length) || length < 1 || !Number.isSafeInteger(from) || from < 0 || from >= length
        || !Number.isSafeInteger(gap) || gap < 0 || gap > length) return null;
    const to = gap > from ? gap - 1 : gap;
    return to === from ? null : to;
}
export function unionVariant(shape: Extract<Shape, {
    kind: "union";
}>, value: NativeValue, schema: DataSchema): number {
    const discriminator = shape.discriminator;
    if (value.kind === "record" && discriminator) {
        if (discriminator.kind === "member") {
            const matching = (discriminator.keys ?? []).map((key, index) => nativeMember(value, key) ? index : -1).filter(index => index >= 0);
            return matching.length === 1 ? matching[0] : -1;
        }
        if (discriminator.kind === "record_presence")
            return nativeMember(value, discriminator.key!) ? discriminator.present! : discriminator.absent!;
        if (discriminator.kind === "tag") {
            const tag = nativeMember(value, discriminator.key!);
            if (tag?.kind !== "text")
                return -1;
            const matching = shape.variants.flatMap((variant, index) => {
                const resolved = resolveShape(variant, schema);
                if (resolved.kind !== "record")
                    return [];
                const member = resolved.members[discriminator.key!];
                if (!member)
                    return [];
                const memberShape = resolveShape(member.shape, schema);
                return memberShape.kind === "enum" && memberShape.values.includes(tag.value) ? [index] : [];
            });
            return matching.length === 1 ? matching[0] : -1;
        }
    }
    const matching = shape.variants.flatMap((variant, index) => { const resolved = resolveShape(variant, schema); const kind = resolved.kind === "enum" ? "text" : resolved.kind === "list" ? "array" : resolved.kind === "record" ? "record" : resolved.kind === "scalar" ? nativeKind(resolved.semantic) : "?"; return kind === value.kind ? [index] : []; });
    return matching.length === 1 ? matching[0] : -1;
}
function nativeKind(semantic: string): NativeValue["kind"] { return ({ local_datetime: "localDatetime", exact_decimal: "text", exact_seconds: "text" } as Record<string, NativeValue["kind"]>)[semantic] ?? (["bool", "int", "float", "decimal", "date", "time", "duration"].includes(semantic) ? semantic as NativeValue["kind"] : "text"); }
export function emptyNativeValue(shape: Shape, schema: DataSchema): NativeValue {
    const resolved = resolveShape(shape, schema);
    if (resolved.kind === "union")
        return emptyNativeValue(resolved.variants[0], schema);
    if (resolved.kind === "list")
        return { kind: "array", value: Array.from({ length: resolved.min }, () => emptyNativeValue(resolved.items, schema)) };
    if (resolved.kind === "record")
        return { kind: "record", value: Object.entries(resolved.members).filter(([, member]) => member.required).map(([key, member]) => ({ key, value: emptyNativeValue(member.shape, schema) })) };
    if (resolved.kind === "enum")
        return { kind: "text", value: resolved.values[0] ?? "" };
    if (resolved.kind !== "scalar")
        throw new UiError(uiMessage("dataValidation.shapeUnknown"));
    const kind = nativeKind(resolved.semantic);
    if (kind === "bool")
        return { kind, value: false };
    return { kind: kind as "text", value: ["int", "float", "decimal", "duration"].includes(kind) ? "0" : "" };
}
export function switchUnionValue(shape: Extract<Shape, {
    kind: "union";
}>, current: NativeValue, branch: number, schema: DataSchema): NativeValue {
    const target = resolveShape(shape.variants[branch], schema);
    let next = emptyNativeValue(target, schema);
    if (current.kind !== "record")
        return next;
    const known = new Set(shape.variants.flatMap(variant => { const resolved = resolveShape(variant, schema); return resolved.kind === "record" ? Object.keys(resolved.members) : []; }));
    const unknown = current.value.filter(item => !known.has(item.key));
    if (unknown.length && (target.kind !== "record" || target.unknown !== "preserve"))
        throw new UiError(uiMessage("dataValidation.unionUnknownMembersPreserved"));
    if (target.kind === "record" && next.kind === "record") {
        const oldBranch = unionVariant(shape, current, schema);
        const old = oldBranch >= 0 ? resolveShape(shape.variants[oldBranch], schema) : undefined;
        for (const item of current.value)
            if (!known.has(item.key) || (old?.kind === "record" && target.members[item.key] && JSON.stringify(old.members[item.key]?.shape) === JSON.stringify(target.members[item.key].shape)))
                next = replaceNativeMember(next, item.key, item.value);
    }
    return next;
}
/** Never parse exact numerics through Number. The backend performs domain validation. */
export function scalarFromInput(shape: Shape, schema: DataSchema, text: string): NativeValue {
    const resolved = resolveShape(shape, schema);
    if (resolved.kind === "enum")
        return { kind: "text", value: text };
    if (resolved.kind !== "scalar")
        throw new UiError(uiMessage("dataValidation.scalarRequired"));
    const kind = nativeKind(resolved.semantic);
    if (kind === "bool")
        return { kind, value: text === "true" };
    if (kind === "int" && !/^-?(0|[1-9][0-9]*)$/.test(text))
        throw new UiError(uiMessage("dataValidation.exactIntegerIncomplete"));
    if (["float", "decimal", "duration"].includes(kind) && !/^[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?$/.test(text))
        throw new UiError(uiMessage("dataValidation.numberIncomplete"));
    return { kind: kind as "text", value: text };
}
export function formatNativeValue(value: NativeValue | undefined): UiText {
    if (!value)
        return uiMessage("queryValue.absent");
    if (value.kind === "none")
        return "none";
    if (value.kind === "bool")
        return value.value ? uiMessage("nativeValue.booleanYes") : uiMessage("nativeValue.booleanNo");
    if (value.kind === "array")
        return () => value.value.map(item => uiTextValue(formatNativeValue(item))).join("; ");
    if (value.kind === "record")
        return () => value.value.map(item => `${item.key}: ${uiTextValue(formatNativeValue(item.value))}`).join(" · ");
    return value.value;
}
/** Native input values retain their exact serialized scalar representation. */
export function nativeInputValue(value: NativeValue): string {
    if (value.kind === "none")
        return "none";
    if (value.kind === "array" || value.kind === "record")
        throw new Error("A structured value has no scalar control value");
    return String(value.value);
}
export function diagnosticText(value: unknown): string { return typeof value === "string" ? value : JSON.stringify(value); }
export function ui<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text: UiText = ""): HTMLElementTagNameMap[K] { const element = document.createElement(tag); element.className = className; setUiText(element, text); return element; }
export function action(label: UiText, run: () => void | Promise<void>, className = "data-button"): HTMLButtonElement {
    const button = ui("button", className, label);
    button.type = "button";
    button.onclick = () => {
        if (button.isConnected === false)
            return;
        void run();
    };
    return button;
}
let nextInputId = 0;
export function labelFor(label: UiText, input: HTMLElement): HTMLLabelElement { const result = ui("label", "data-label", label); input.id ||= `data-input-${++nextInputId}`; result.htmlFor = input.id; return result; }
const cloneValue = (value: NativeValue): NativeValue => structuredClone(value);
export const emptyReferences = (): FormReferences => ({ nodes: [], state: "loading", messages: [] });
const refreshTagPickers = new WeakMap<HTMLElement, () => void>();
function referenceDescriptions(value:unknown):UiText[] {
    if(!Array.isArray(value)) throw new UiError(uiMessage("error.invalidResponse"));
    return value.map(value=>{const description=decodeCoreDescription(value);if(!description)throw new UiError(uiMessage("error.invalidResponse"));return coreDescriptionUiText(description);});
}

export async function loadTagFormReferences(host: PanelHost, root: string, references: FormReferences, container: HTMLElement, accepts = (): boolean => true): Promise<void> {
    if (!accepts()) return;
    references.tagTree = undefined;
    try {
        const result = await host.request<{
            tags: {
                tagRef: string;
                tagId: string;
                displayName: string;
                parentTagId: string | null;
            }[];
            complete: boolean;
            warnings:unknown[];
        }>("/api/navigation/tags", { root });
        if (!accepts())
            return;
        const warnings=referenceDescriptions(result.warnings);
        references.tagTree = new TagTree(result.tags.map(tag => ({ ...tag, reference: tag.tagRef })), result.complete === true);
        references.messages.push(...warnings);
        references.state = "ready";
    }
    catch (error) {
        if (!accepts())
            return;
        references.state = "unavailable";
        references.messages.push(errorUiText(error));
    }
    for (const picker of container.querySelectorAll<HTMLElement>(".data-tag-reference-picker"))
        refreshTagPickers.get(picker)?.();
}
function populateReferenceChoices(select: HTMLSelectElement, references: FormReferences): void {
    const choices = references.nodes;
    const current = select.value;
    select.replaceChildren();
    const option = ui("option", "", references.state === "loading" ? uiMessage("referenceForm.loadingChoices") : choices.length ? uiMessage("referenceForm.chooseActual") : uiMessage("referenceForm.noChoices"));
    option.value = "";
    select.append(option);
    for (const choice of choices) {
        const item = ui("option", "", choice.label);
        item.value = choice.ref;
        select.append(item);
    }
    select.value = choices.some(choice => choice.ref === current) ? current : "";
}
function referenceStatus(references: FormReferences): UiText { return ({ loading: uiMessage("referenceForm.loadingHelp"), ready: uiMessage("referenceForm.exactChoiceHelp"), partial: uiMessage("referenceForm.partialHelp"), unavailable: uiMessage("referenceForm.unavailableHelp") })[references.state]; }
export async function loadFormReferences(host: PanelHost, root: string, references: FormReferences, container: HTMLElement, accepts = (): boolean => true): Promise<void> {
    if (!accepts()) return;
    references.state = "loading";
    const results = await Promise.allSettled([host.request<{
            nodes: {
                ref: string;
                displayLabel: string;
            }[];
            complete: boolean;
            diagnostics: unknown[];
        }>("/api/data/references", { root }).then(result=>({...result,diagnostics:referenceDescriptions(result.diagnostics)})), host.request<{
            tags: {
                tagRef: string;
                tagId: string;
                displayName: string;
                parentTagId: string | null;
            }[];
            complete: boolean;
            warnings: unknown[];
        }>("/api/navigation/tags", { root }).then(result=>({...result,warnings:referenceDescriptions(result.warnings)}))]);
    if (!accepts())
        return;
    references.messages = [];
    references.tagTree = undefined;
    if (results[0].status === "fulfilled") {
        references.nodes = results[0].value.nodes.map(node => ({ ref: node.ref, label: displayMessage("referenceForm.filenameFallback", { p0: uiDisplayParameter(node.displayLabel) }) }));
        references.messages.push(...results[0].value.diagnostics);
    }
    else
        references.messages.push(uiMessage("referenceForm.nodeListUnavailable"));
    if (results[1].status === "fulfilled") {
        try {
            references.tagTree = new TagTree(results[1].value.tags.map(tag => ({ ...tag, reference: tag.tagRef })), results[1].value.complete === true);
        }
        catch (error) {
            references.messages.push(errorUiText(error));
        }
        references.messages.push(...results[1].value.warnings);
    }
    else
        references.messages.push(uiMessage("referenceForm.tagRegistryUnavailable"));
    references.state = results.every(result => result.status === "rejected") ? "unavailable" : results.some(result => result.status === "rejected") || results.some(result => result.status === "fulfilled" && "complete" in result.value && result.value.complete === false) ? "partial" : "ready";
    for (const select of container.querySelectorAll<HTMLSelectElement>("select[data-ref-kind]"))
        populateReferenceChoices(select, references);
    for (const picker of container.querySelectorAll<HTMLElement>(".data-tag-reference-picker"))
        refreshTagPickers.get(picker)?.();
    for (const message of container.querySelectorAll<HTMLElement>(".data-reference-status"))
        setUiText(message, referenceStatus(references));
}
/** Recursive form edits a local value. Unknown record members survive unchanged. */
function bindNativeControl(control: HTMLElement, identity: NativeControlIdentity, role: string): void { registerNativeControl(control, identity, role); }
interface BoundNativeControl {
    slot: string;
    path: readonly ValuePath[];
}
function canRestructureNative(container: HTMLElement, status: HTMLElement, composing: ReadonlySet<EventTarget>, warning: UiText): boolean {
    if (composing.size) { setUiText(status, uiMessage("dataPanel.compositionPending")); return false; }
    const invalid = container.querySelector<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>("input:invalid,select:invalid,textarea:invalid");
    if (invalid) {
        setUiText(status, warning); invalid.reportValidity(); invalid.focus({ preventScroll: true }); return false;
    }
    setUiText(status, ""); return true;
}
export function valueEditor(shape: Shape, initial: NativeValue, schema: DataSchema, changed: (value: NativeValue) => void, label: UiText, identity: NativeControlIdentity, depth = 0, references?: FormReferences, binding?: NativeControlBinding, lifetime = new Map<string, BoundNativeControl>(), presentation = schemaPropertyPresentation(schema, identity)): HTMLElement {
    const retire = () => {
        for (const [key, entry] of lifetime)
            if (identity.path.every((part, index) => JSON.stringify(part) === JSON.stringify(entry.path[index]))) {
                binding?.session.invalidate(entry.slot, entry.path);
                lifetime.delete(key);
            }
    };
    const host = ui("div", "native-value-editor");
    let value = cloneValue(initial);
    let resolved: Shape;
    try {
        resolved = resolveShape(shape, schema);
    }
    catch (error) {
        host.append(ui("p", "data-warning", String(error)));
        return host;
    }
    if (depth > 18) {
        host.append(ui("p", "data-warning", uiMessage("nativeEditor.nestingLimit")));
        return host;
    }
    if (value.kind === "none") {
        host.append(ui("span", "data-empty", uiMessage("nativeValue.explicitNone")), action(uiMessage("nativeEditor.fillValue"), () => { retire(); value = emptyNativeValue(resolved, schema); changed(value); host.replaceChildren(valueEditor(resolved, value, schema, changed, label, identity, depth, references, binding, lifetime, presentation)); }));
        return host;
    }
    if (resolved.kind === "union") {
        let branch = unionVariant(resolved, value, schema);
        const select = ui("select", "data-input");
        bindNativeControl(select, identity, "variant");
        const absent = ui("option", "", uiMessage("nativeEditor.chooseBranch"));
        absent.value = "-1";
        select.append(absent);
        resolved.variants.forEach((variant, index) => { const candidate = resolveShape(variant, schema); const description = candidate.kind === "record" ? joinUiText(Object.keys(candidate.members).slice(0, 3).map(key => presentation.memberLabel(key)), " / ") : candidate.kind === "scalar" ? candidate.semantic : candidate.kind; const option = ui("option", "", () => `${index + 1}. ${uiTextValue(description)}`); option.value = String(index); select.append(option); });
        select.value = String(branch);
        host.append(labelFor(() => displayMessage("nativeEditor.branchLabel", { p0: uiDisplayParameter(uiTextValue(label)) }), select), select);
        const body = ui("div", "native-union-body");
        host.append(body);
        const show = (): void => {
            body.replaceChildren();
            if (branch >= 0)
                body.append(valueEditor(resolved.variants[branch], value, schema, next => { value = next; changed(next); }, label, { ...identity, variants: [...identity.variants ?? [], branch] }, depth + 1, references, binding, lifetime, presentation));
            else
                body.append(ui("p", "data-warning", uiMessage("nativeEditor.ambiguousBranch")));
        };
        select.onchange = () => {
            if (select.isConnected === false)
                return;
            const next = Number(select.value);
            if (next < 0)
                return;
            try {
                retire();
                value = switchUnionValue(resolved, value, next, schema);
                branch = next;
                changed(value);
                show();
            }
            catch (error) {
                select.value = String(branch);
                body.append(ui("p", "data-warning", error instanceof Error ? errorUiText(error) : String(error)));
            }
        };
        show();
        return host;
    }
    if (resolved.kind === "record") {
        if (value.kind !== "record") {
            host.append(ui("p", "data-warning", uiMessage("nativeEditor.shapeMismatch")));
            return host;
        }
        const record = ui("div", "native-record");
        const status = ui("p", "data-help"), composing = new Set<EventTarget>();
        status.setAttribute("role", "status");
        host.addEventListener("compositionstart", event => { if (event.target) composing.add(event.target); });
        host.addEventListener("compositionend", event => { if (event.target) composing.delete(event.target); });
        const canRestructure = (): boolean => canRestructureNative(record, status, composing, uiMessage("nativeEditor.finishInvalidBeforeStructure"));
        host.append(record, status);
        const render = (): void => {
            record.replaceChildren();
            for (const [key, member] of Object.entries(resolved.members)) {
                const present = nativeMember(value, key);
                const row = ui("div", "native-member");
                row.dataset.memberKey = key;
                const heading = ui("div", "native-member-heading");
                heading.append(ui("span", "", () => `${uiTextValue(presentation.memberLabel(key))}${uiTextValue(member.required ? " *" : "")}`));
                row.append(heading);
                if (!present) {
                    heading.append(ui("span", "data-empty", uiMessage("queryValue.absent")), action(uiMessage("nativeEditor.addMember"), () => { if (!canRestructure()) return; retire(); value = replaceNativeMember(value, key, emptyNativeValue(member.shape, schema)); changed(value); render(); }, "data-button compact"));
                }
                else {
                    if (!member.required)
                        heading.append(action(uiMessage("nativeEditor.removeMember"), () => { if (!canRestructure()) return; retire(); value = replaceNativeMember(value, key, undefined); changed(value); render(); }, "data-button compact"));
                    if (member.nullable && present.kind !== "none")
                        heading.append(action(uiMessage("nativeEditor.setNone"), () => { if (!canRestructure()) return; retire(); value = replaceNativeMember(value, key, { kind: "none" }); changed(value); render(); }, "data-button compact"));
                    row.append(valueEditor(member.shape, present, schema, next => { value = replaceNativeMember(value, key, next); changed(value); }, () => `${uiTextValue(label)} · ${uiTextValue(presentation.memberLabel(key))}`, { ...identity, path: [...identity.path, { kind: "member", key }] }, depth + 1, references, binding, lifetime, presentation));
                }
                record.append(row);
            }
            if (value.kind === "record")
                for (const unknown of value.value.filter(item => !resolved.members[item.key])) {
                    const row = ui("div", "native-unknown");
                    row.append(ui("strong", "", displayMessage("nativeEditor.unboundReadOnly", { p0: uiDisplayParameter(unknown.key) })), ui("p", "", formatNativeValue(unknown.value)));
                    record.append(row);
                }
        };
        render();
        return host;
    }
    if (resolved.kind === "list") {
        if (value.kind !== "array") {
            host.append(ui("p", "data-warning", uiMessage("nativeEditor.arrayRequired")));
            return host;
        }
        const items = ui("div", "native-list");
        const status = ui("p", "data-help native-list-order-status");
        status.setAttribute("role", "status");
        status.setAttribute("aria-live", "polite");
        const composing = new Set<EventTarget>();
        host.addEventListener("compositionstart", event => { if (event.target) composing.add(event.target); });
        host.addEventListener("compositionend", event => { if (event.target) composing.delete(event.target); });
        host.append(items, status);
        host.setAttribute("style", "position:relative");
        const line = ui("div", "native-list-drop-line");
        line.hidden = true;
        line.setAttribute("aria-hidden", "true");
        const lineStyle = "position:absolute;left:0;right:0;height:3px;background:var(--accent,#526b4d);pointer-events:none;z-index:1";
        line.setAttribute("style", lineStyle);
        host.append(line);
        type Drag = { handle: HTMLButtonElement; pointerId: number; from: number; value: NativeValue; x: number; y: number; latestX: number; latestY: number; gap: number | null; started: boolean; observer?: MutationObserver };
        let drag: Drag | null = null;
        const stopDrag = (cancelled = false): void => {
            const prior = drag;
            drag = null;
            line.hidden = true;
            delete host.dataset.valueDragging;
            if (prior) {
                prior.observer?.disconnect();
                prior.handle.setAttribute("style", "touch-action:none;cursor:grab");
                host.ownerDocument.defaultView?.removeEventListener("blur", blurDrag);
                host.ownerDocument.defaultView?.removeEventListener("scroll", refreshDrag, true);
                host.ownerDocument.defaultView?.removeEventListener("resize", refreshDrag);
                if (prior.handle.hasPointerCapture?.(prior.pointerId)) prior.handle.releasePointerCapture(prior.pointerId);
                if (cancelled && prior.started) setUiText(status, uiMessage("nativeEditor.dragCancelled"));
            }
        };
        const currentDrag = (): Drag | null => {
            if (!drag) return null;
            if (drag.value !== value || !drag.handle.isConnected || drag.handle.disabled || host.closest?.("[inert],[hidden]")) {
                stopDrag(true);
                return null;
            }
            return drag;
        };
        const locateDrag = (): void => {
            const current = currentDrag();
            if (!current?.started) return;
            let row = host.ownerDocument.elementFromPoint(current.latestX, current.latestY)?.closest<HTMLElement>(".native-list-item") ?? null;
            while (row && row.parentElement !== items) row = row.parentElement?.closest<HTMLElement>(".native-list-item") ?? null;
            current.gap = null;
            line.hidden = true;
            if (!row) return;
            const index = Number(row.dataset.valueOrderIndex), rect = row.getBoundingClientRect();
            const after = current.latestY >= rect.top + rect.height / 2;
            const gap = index + (after ? 1 : 0);
            if (value.kind !== "array" || orderedDropTarget(current.from, gap, value.value.length) === null) return;
            current.gap = gap;
            line.setAttribute("style", `${lineStyle};top:${(after ? rect.bottom : rect.top) - host.getBoundingClientRect().top + host.scrollTop}px`);
            line.hidden = false;
            const to = orderedDropTarget(current.from, gap, value.value.length)!;
            setUiText(status, displayMessage("nativeEditor.dragTarget", { from: current.from + 1, to: to + 1 }));
        };
        const refreshDrag = (): void => locateDrag();
        const blurDrag = (): void => stopDrag(true);
        host.addEventListener("keydown", event => {
            if (event.key !== "Escape" || !drag || event.isComposing || event.keyCode === 229) return;
            event.preventDefault(); event.stopPropagation(); stopDrag(true);
        });
        host.addEventListener("compositionstart", () => stopDrag(true));
        const canRestructure = (): boolean => canRestructureNative(items, status, composing, uiMessage("nativeEditor.finishInvalidBeforeReorder"));
        const birthdayField = identity.fieldId === "people/life_events" && identity.path.length === 0
            ? schema.fields.find(field => field.id === identity.fieldId) : undefined;
        const birthday = birthdayField && isBuiltinField(schema, birthdayField) ? ui("section", "native-birthday") : undefined;
        if (birthday) { setUiAttr(birthday, "aria-label", uiMessage("birthday.title")); host.prepend(birthday); }
        const focusBirthTime = (index: number): void => {
            const row = items.children.item(index);
            const time = row?.querySelector<HTMLElement>(":scope > .native-value-editor > .native-record > [data-member-key='event_time']");
            time?.scrollIntoView({ block: "nearest" });
            const editor = time?.querySelector<HTMLElement>(":scope > .native-value-editor");
            const control = editor
                ? editor.querySelector<HTMLElement>("input:not(:disabled),textarea:not(:disabled),select:not(:disabled),button:not(:disabled)")
                : time?.querySelector<HTMLElement>(":scope > .native-member-heading button:not(:disabled)");
            control?.focus({ preventScroll: true });
        };
        const renderBirthday = (): void => {
            if (!birthday) return;
            birthday.replaceChildren(ui("strong", "", uiMessage("birthday.title")));
            const events = birthEvents(value), selected = events.filter(event => event.selected);
            birthday.append(ui("p", "data-help", uiMessage(selected.length > 1 ? "birthday.conflict" : selected.length === 0 ? "birthday.unselected" : "birthday.selected")));
            for (const event of events) {
                const entry = ui("div", "birthday-entry");
                entry.dataset.birthIndex = String(event.index);
                const precision = birthTimePrecision(event.time);
                const precisionLabels = { unknown: uiMessage("birthday.unknown"), monthDay: uiMessage("birthday.monthDay"), year: uiMessage("birthday.year"), month: uiMessage("birthday.month"), day: uiMessage("birthday.day"), exact: uiMessage("birthday.exact") };
                entry.append(ui("span", "", displayMessage("nativeEditor.itemLegend", { p0: event.index + 1 })),
                    ui("span", "birthday-time", birthTimeText(event.time) ?? uiMessage("birthday.unknown")));
                if (precision !== "unknown") entry.append(ui("span", "data-help", precisionLabels[precision]));
                if (birthTimeIsLeapMonth(event.time)) entry.append(ui("span", "birthday-leap-month", uiMessage("builtinProperty.member.leapMonth")));
                if (event.selected) entry.append(ui("span", "data-help", uiMessage("birthday.chosen")));
                if (!(event.selected && selected.length === 1)) entry.append(action(uiMessage("birthday.choose"), () => {
                    if (!canRestructure()) return;
                    binding?.session.recordAction(`${binding.slot}:birthday-selection`, identity.path);
                    value = selectBirthEvent(value, event.index); changed(value); render(); focusBirthTime(event.index);
                }, "data-button compact"));
                entry.append(action(uiMessage("birthday.edit"), () => focusBirthTime(event.index), "data-button compact"));
                birthday.append(entry);
            }
            const add = action(uiMessage("birthday.add"), () => {
                if (value.kind !== "array" || !canRestructure()) return;
                const index = value.value.length;
                binding?.session.recordAction(`${binding.slot}:birthday-add`, identity.path);
                value = appendBirthEvent(value); changed(value); render(); focusBirthTime(index);
            }, "data-button compact");
            add.disabled = resolved.max != null && value.kind === "array" && value.value.length >= resolved.max;
            birthday.append(add, ui("p", "data-help", uiMessage("birthday.help")));
        };
        const render = (focusIndex?: number, focusRole = "position"): void => {
            stopDrag();
            renderBirthday();
            items.replaceChildren();
            if (value.kind !== "array")
                return;
            if (!value.value.length)
                items.append(ui("span", "data-empty", uiMessage("nativeEditor.emptyArray")));
            const length = value.value.length;
            value.value.forEach((item, index) => {
                const row = ui("fieldset", "native-list-item");
                row.dataset.valueOrderIndex = String(index);
                row.append(ui("legend", "", displayMessage("nativeEditor.itemLegend", { p0: index + 1 })));
                const controls = ui("div", "native-list-controls");
                const move = (to: number, role: string): boolean => {
                    if (!canRestructure()) return false;
                    const next = moveOrderedValue(value, index, to);
                    if (next === value) return false;
                    retire();
                    value = next;
                    changed(value);
                    render(to, role);
                    return true;
                };
                const up = action(uiMessage("queryShortcut.moveUp"), () => { move(index - 1, "up"); }, "data-button compact");
                up.dataset.orderAction = "up";
                up.disabled = index === 0;
                const down = action(uiMessage("nodeContents.moveDown"), () => { move(index + 1, "down"); }, "data-button compact");
                down.dataset.orderAction = "down";
                down.disabled = value.kind !== "array" || index === value.value.length - 1;
                const remove = action(uiMessage("nativeEditor.removeMember"), () => {
                    if (value.kind !== "array")
                        return;
                    if (!canRestructure()) return;
                    retire();
                    value = { kind: "array", value: value.value.filter((_, ordinal) => ordinal !== index) };
                    changed(value);
                    render();
                }, "data-button compact");
                remove.disabled = value.kind !== "array" || value.value.length <= resolved.min;
                const handle = action(displayMessage("nativeEditor.dragValue", { index: index + 1 }), () => { setUiText(status, uiMessage("nativeEditor.dragKeyboardHelp")); }, "data-button compact native-value-drag-handle");
                handle.dataset.orderAction = "drag";
                handle.setAttribute("style", "touch-action:none;cursor:grab");
                handle.disabled = length < 2;
                let suppressPointerClick = false;
                handle.addEventListener("click", event => {
                    if (suppressPointerClick && event.detail > 0) { event.preventDefault(); event.stopImmediatePropagation(); }
                }, true);
                handle.addEventListener("pointerdown", event => {
                    if (event.button !== 0 || !event.isPrimary || handle.disabled || !handle.isConnected || host.closest?.("[inert],[hidden]")) return;
                    stopDrag();
                    suppressPointerClick = true;
                    event.preventDefault();
                    if (!canRestructure()) return;
                    suppressPointerClick = false;
                    drag = { handle, pointerId: event.pointerId, from: index, value, x: event.clientX, y: event.clientY, latestX: event.clientX, latestY: event.clientY, gap: null, started: false };
                    try { handle.setPointerCapture(event.pointerId); }
                    catch { stopDrag(); return; }
                    const observer = new MutationObserver(() => { currentDrag(); });
                    drag.observer = observer;
                    observer.observe(host.ownerDocument, { childList: true, subtree: true, attributes: true, attributeFilter: ["inert", "hidden", "disabled"] });
                    host.ownerDocument.defaultView?.addEventListener("blur", blurDrag);
                    host.ownerDocument.defaultView?.addEventListener("scroll", refreshDrag, true);
                    host.ownerDocument.defaultView?.addEventListener("resize", refreshDrag);
                    handle.focus({ preventScroll: true });
                    event.preventDefault();
                });
                handle.addEventListener("pointermove", event => {
                    const current = currentDrag();
                    if (!current || current.handle !== handle || current.pointerId !== event.pointerId) return;
                    if (!(event.buttons & 1)) { stopDrag(true); return; }
                    current.latestX = event.clientX; current.latestY = event.clientY;
                    if (!current.started && Math.hypot(event.clientX - current.x, event.clientY - current.y) >= 4) {
                        current.started = true; suppressPointerClick = true; host.dataset.valueDragging = "true"; handle.setAttribute("style", "touch-action:none;cursor:grabbing");
                    }
                    locateDrag();
                    event.preventDefault();
                });
                handle.addEventListener("pointerup", event => {
                    const current = currentDrag();
                    if (!current || current.handle !== handle || current.pointerId !== event.pointerId) return;
                    current.latestX = event.clientX; current.latestY = event.clientY;
                    locateDrag();
                    const to = value.kind === "array" && current.gap !== null ? orderedDropTarget(current.from, current.gap, value.value.length) : null;
                    const started = current.started;
                    stopDrag();
                    if (started && to !== null) { if (move(to, "drag")) setUiText(status, displayMessage("nativeEditor.dragMoved", { from: index + 1, to: to + 1 })); }
                    else if (started) setUiText(status, uiMessage("nativeEditor.dragCancelled"));
                    event.preventDefault();
                });
                const cancelPointer = (event: PointerEvent): void => { if (drag?.handle === handle && drag.pointerId === event.pointerId) stopDrag(true); };
                handle.addEventListener("pointercancel", cancelPointer);
                handle.addEventListener("lostpointercapture", cancelPointer);
                controls.append(handle, up, down, remove);
                const order = ui("details", "native-list-order");
                order.append(ui("summary", "", uiMessage("nativeEditor.movePosition")));
                const first = action(uiMessage("nativeEditor.moveFirst"), () => { move(0, "first"); }, "data-button compact");
                first.dataset.orderAction = "first";
                first.disabled = index === 0;
                const last = action(uiMessage("nativeEditor.moveLast"), () => { if (value.kind === "array") move(value.value.length - 1, "last"); }, "data-button compact");
                last.dataset.orderAction = "last";
                last.disabled = value.kind !== "array" || index === value.value.length - 1;
                const position = ui("input", "data-input native-list-position");
                bindNativeControl(position, { ...identity, path: [...identity.path, { kind: "index", index }] }, "order-position");
                position.type = "text";
                position.inputMode = "numeric";
                position.size = 5;
                position.value = String(index + 1);
                position.dataset.orderAction = "position";
                setUiAttr(position, "aria-label", () => displayMessage("nativeEditor.itemLabel", { p0: uiDisplayParameter(uiTextValue(uiMessage("nativeEditor.movePosition"))), p1: index + 1 }));
                position.oninput = () => { position.removeAttribute("aria-invalid"); setUiText(status, ""); };
                const atPosition = (): void => {
                    if (value.kind !== "array" || composing.size) { canRestructure(); return; }
                    const to = orderedMoveTarget(position.value, value.value.length);
                    if (to === null) {
                        position.setAttribute("aria-invalid", "true");
                        setUiText(status, displayMessage("nativeEditor.movePositionInvalid", { count: value.value.length }));
                        position.focus({ preventScroll: true });
                        return;
                    }
                    move(to, "position");
                };
                position.addEventListener("keydown", event => {
                    if (event.key === "Enter" && !event.isComposing && event.keyCode !== 229) { event.preventDefault(); atPosition(); }
                });
                order.append(first, last, labelFor(uiMessage("nativeEditor.movePosition"), position), position,
                    action(uiMessage("nativeEditor.moveToPosition"), atPosition, "data-button compact"),
                    ui("p", "data-help", displayMessage("nativeEditor.movePositionHelp", { count: length })));
                if (index === focusIndex && !["up", "down", "drag"].includes(focusRole)) order.open = true;
                row.append(controls, order, valueEditor(resolved.items, item, schema, next => {
                    if (value.kind !== "array")
                        return;
                    const nextItems = [...value.value];
                    nextItems[index] = next;
                    value = { kind: "array", value: nextItems };
                    changed(value);
                    renderBirthday();
                }, () => displayMessage("nativeEditor.itemLabel", { p0: uiDisplayParameter(uiTextValue(label)), p1: index + 1 }), { ...identity, path: [...identity.path, { kind: "index", index }] }, depth + 1, references, binding, lifetime, presentation));
                items.append(row);
                if (index === focusIndex) {
                    const requested = [handle, up, down, first, last, position].find(control => control.dataset.orderAction === focusRole && !("disabled" in control && control.disabled));
                    const fallback = !up.disabled ? up : !down.disabled ? down : position;
                    (requested ?? fallback).focus({ preventScroll: true });
                }
            });
            const add = action(uiMessage("nativeEditor.addItem"), () => {
                if (value.kind !== "array")
                    return;
                if (!canRestructure()) return;
                retire();
                value = { kind: "array", value: [...value.value, emptyNativeValue(resolved.items, schema)] };
                changed(value);
                render();
            });
            add.disabled = resolved.max != null && value.value.length >= resolved.max;
            items.append(add);
        };
        render();
        return host;
    }
    const scalar = resolved.kind === "scalar" ? resolved.semantic : "enum";
    if (scalar === "bool") {
        const input = ui("input");
        bindNativeControl(input, identity, "value:bool");
        input.type = "checkbox";
        input.checked = value.kind === "bool" && value.value;
        const control = labelFor(label, input);
        control.prepend(input);
        host.append(control);
        input.onchange = () => {
            if (input.isConnected === false)
                return;
            changed({ kind: "bool", value: input.checked });
        };
        return host;
    }
    if (resolved.kind === "enum" && !resolved.open) {
        const input = ui("select", "data-input");
        bindNativeControl(input, identity, "value:enum");
        resolved.values.forEach(optionValue => { const option = ui("option", "", presentation.enumLabel(resolved, optionValue)); option.value = optionValue; input.append(option); });
        if (value.kind === "text" && !resolved.values.includes(value.value)) {
            const originalText = value.value;
            const unknown = ui("option", "", displayMessage("nativeEditor.unregisteredEnumChoice", { p0: originalText }));
            unknown.value = value.value;
            input.append(unknown);
        }
        input.value = nativeInputValue(value);
        setUiAttr(input, "aria-label", label);
        host.append(input);
        input.onchange = () => {
            if (input.isConnected === false) return;
            changed({ kind: "text", value: input.value });
        };
        return host;
    }
    // Open enums are authored text, with one visible value and the same raw/IME
    // owner as other text fields. Registered codes never create a second draft.
    const input = scalar === "text" || scalar === "nonempty_text" || resolved.kind === "enum" ? ui("textarea", "data-input") : ui("input", "data-input");
    bindNativeControl(input, identity, `value:${scalar}`);
    if (input instanceof HTMLTextAreaElement)
        input.rows = 2;
    input.value = nativeInputValue(value);
    setUiAttr(input, "aria-label", label);
    if (input instanceof HTMLInputElement)
        input.type = "text";
    if (["int", "float", "decimal", "exact_decimal", "duration", "exact_seconds"].includes(scalar))
        input.setAttribute("inputmode", "decimal");
    if (binding) {
        const slot = JSON.stringify([binding.slot, `value:${scalar}`, identity.variants ?? []]);
        binding.session.bind(input, slot, identity.path);
        lifetime.set(JSON.stringify([slot, identity.path]), { slot, path: identity.path });
    }
    setUiAttr(input, "placeholder", ({ date: "2026-10-13", time: "12:30:00", local_datetime: "2026-10-13T12:30:00", instant: "2026-10-13T12:30:00+08:00", node_ref: uiMessage("nativeEditor.nodeAddressPlaceholder"), phone: uiMessage("nativeEditor.phonePlaceholder"), exact_decimal: uiMessage("nativeEditor.exactDecimalPlaceholder") } as Record<string, UiText>)[scalar] ?? "");
    input.oninput = () => {
        try {
            const raw = input instanceof HTMLTextAreaElement && value.kind === "text" ? textareaSourceDraft(value.value,input.value) : input.value;
            const next = scalarFromInput(resolved, schema, raw);
            value = next;
            setUiValidity(input, "");
            changed(next);
        }
        catch (error) {
            setUiValidity(input, error instanceof Error ? errorUiText(error) : String(error));
        }
    };
    if (scalar === "tag_ref" && references) {
        const pickerHost = ui("div", "data-tag-reference-picker");
        const renderPicker = () => {
            pickerHost.replaceChildren();
            if (references.tagTree) {
                const picker = tagPicker(references.tagTree, { label: () => displayMessage("nativeEditor.tagPickerLabel", { p0: uiDisplayParameter(uiTextValue(label)) }), value: input.value, changed: ref => {
                        if (!ref)
                            return;
                        input.value = ref;
                        input.dispatchEvent(new Event("input", { bubbles: true }));
                    } });
                for (const control of picker.querySelectorAll<HTMLElement>("input,select"))
                    bindNativeControl(control, identity, control.dataset.controlRole!);
                pickerHost.append(picker);
            }
            else
                pickerHost.append(ui("p", "data-help", uiMessage("nativeEditor.tagTreeUnavailable")));
        };
        refreshTagPickers.set(pickerHost, renderPicker);
        renderPicker();
        host.append(pickerHost);
    }
    if (scalar === "node_ref" && references) {
        const picker = ui("select", "data-input data-ref-picker");
        bindNativeControl(picker, identity, "node-selection");
        picker.dataset.refKind = scalar;
        populateReferenceChoices(picker, references);
        setUiAttr(picker, "aria-label", () => displayMessage("nativeEditor.nodePickerLabel", { p0: uiDisplayParameter(uiTextValue(label)) }));
        picker.onchange = () => {
            if (!picker.value)
                return;
            input.value = picker.value;
            input.dispatchEvent(new Event("input", { bubbles: true }));
        };
        host.append(picker, ui("p", "data-help data-reference-status", referenceStatus(references)));
    }
    host.append(input);
    if (["date", "time", "local_datetime", "instant"].includes(scalar))
        host.append(ui("p", "data-help", scalar === "instant" ? uiMessage("nativeEditor.instantHelp") : uiMessage("nativeEditor.civilTimeHelp")));
    return host;
}
export function nativeFormControls(wrapper: HTMLElement): [
    string,
    HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement
][] {
    const seen = new Set<string>();
    return [...wrapper.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>("input, select, textarea")].map(control => {
        const key = nativeControlAddress(control).key;
        if (!key || seen.has(key))
            throw new Error("Native form control identity is missing or duplicated");
        seen.add(key);
        return [key, control];
    });
}
