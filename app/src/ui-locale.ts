import { enUS, zhCN, messageParamTypes, type MessageKey, type MessageParamsMap } from "./ui-messages";
import { parseStrictJson } from "./strict-json";
export type UiLocale = "zh-CN" | "en-US";
export type { MessageKey } from "./ui-messages";
export type MessageParams<K extends MessageKey> = MessageParamsMap[K];
type MessageArgs<K extends MessageKey> = {
    [P in K]: [
        keyof MessageParams<P>
    ] extends [
        never
    ] ? [
    ] : [
        params: MessageParams<P>
    ];
}[K];
const UI_MESSAGE: unique symbol = Symbol("Weftext product message");
const LOCAL_DISPLAY: unique symbol = Symbol("Weftext local display");
/** A decoded envelope is opaque; keyed factories retain exact parameter correlation. */
export type UiMessage<K extends MessageKey = never> = [
    K
] extends [
    never
] ? WireDisplayMessage : {
    [P in K]: Readonly<{
        key: P;
        params: MessageParams<P>;
        [UI_MESSAGE]: true;
    }>;
}[K];
type AnyMessage = UiMessage;
/** Display consumers need the branded envelope; factories and the wire parser retain key/parameter correlation. */
interface WireDisplayMessage {
    readonly key: MessageKey;
    readonly params: object;
    readonly [UI_MESSAGE]: true;
}
export interface LocalDisplayMessage {
    readonly key: MessageKey;
    readonly params: object;
    readonly [LOCAL_DISPLAY]: true;
}
export type UiDisplayMessage = WireDisplayMessage | LocalDisplayMessage;
export type MessageSource = UiDisplayMessage | (() => UiDisplayMessage);
export type UiText = string | UiDisplayMessage | (() => UiText);
export type UiDisplayAttribute = "aria-label" | "aria-description" | "aria-valuetext" | "title" | "placeholder" | "alt";
type LocalMessageParams<K extends MessageKey> = {
    [P in keyof MessageParams<K>]: MessageParams<K>[P] extends string ? UiText : MessageParams<K>[P];
};
type LocalMessageArgs<K extends MessageKey> = {
    [P in K]: [
        keyof MessageParams<P>
    ] extends [
        never
    ] ? [
    ] : [
        params: LocalMessageParams<P>
    ];
}[K];
export function message<K extends MessageKey>(key: K, ...args: MessageArgs<K>): UiMessage<K> {
    const spec = parseUiMessage({ key, params: args[0] ?? {} });
    if (!spec)
        throw new Error(`Invalid product message parameters: ${key}`);
    return spec as UiMessage<K>;
}
/** A local display may quote exact author UTF-16. It is never a wire-message payload. */
export function displayMessage<K extends MessageKey>(key: K, ...args: LocalMessageArgs<K>): LocalDisplayMessage {
    const params = args[0] ?? {}, expected: Readonly<Record<string, string>> = messageParamTypes[key];
    if (!expected || Object.keys(params).length !== Object.keys(expected).length)
        throw new Error(`Invalid display message parameters: ${key}`);
    for (const [name, type] of Object.entries(expected)) {
        const value = (params as Readonly<Record<string, unknown>>)[name];
        if (type === "text" ? !isUiText(value) : type === "integer" ? typeof value !== "number" || !Number.isSafeInteger(value) : typeof value !== "number" || !Number.isFinite(value))
            throw new Error(`Invalid display message parameter: ${key}.${name}`);
    }
    const spec = { key, params: Object.freeze({ ...params }) };
    Object.defineProperty(spec, LOCAL_DISPLAY, { value: true });
    Object.defineProperty(spec, "toJSON", { value: () => { throw new Error("A local display message cannot be sent as a wire message"); } });
    return Object.freeze(spec) as unknown as LocalDisplayMessage;
}
function isUiText(value: unknown): value is UiText {
    return typeof value === "string" || typeof value === "function" || !!value && typeof value === "object" && !!((value as {
        [UI_MESSAGE]?: boolean;
    })[UI_MESSAGE] || (value as {
        [LOCAL_DISPLAY]?: boolean;
    })[LOCAL_DISPLAY]);
}
/** Capture raw values now while retaining explicitly branded display composition for later language changes. */
export function uiDisplayParameter(value: unknown): UiText { return isUiText(value) ? value : value === null || value === undefined ? "" : String(value); }
/** Compose display fragments without freezing their language or stringifying descriptors. */
export function joinUiText(values: readonly UiText[], separator = ""): UiText {
    const captured = [...values];
    return () => captured.map(value => uiTextValue(value)).join(separator);
}
function scalarText(value: string): boolean {
    for (let index = 0; index < value.length; index++) {
        const unit = value.charCodeAt(index);
        if (unit >= 0xd800 && unit <= 0xdbff) {
            const next = value.charCodeAt(++index);
            if (!(next >= 0xdc00 && next <= 0xdfff))
                return false;
        }
        else if (unit >= 0xdc00 && unit <= 0xdfff)
            return false;
    }
    return true;
}
/** Decode product messages explicitly; unknown/raw diagnostics have a separate display channel. */
export function parseUiMessage(value: unknown): UiMessage | undefined {
    if (!value || typeof value !== "object" || Array.isArray(value))
        return;
    if ((value as {
        [LOCAL_DISPLAY]?: boolean;
    })[LOCAL_DISPLAY])
        return;
    const wire = value as Record<string, unknown>;
    if (Object.keys(wire).sort().join(",") !== "key,params" || typeof wire.key !== "string" || !Object.hasOwn(zhCN, wire.key) || !wire.params || typeof wire.params !== "object" || Array.isArray(wire.params))
        return;
    const key = wire.key as MessageKey, params = wire.params as Record<string, unknown>;
    const expected: Readonly<Record<string, string>> = messageParamTypes[key];
    const actualKeys = Object.keys(params).sort(), expectedKeys = Object.keys(expected).sort();
    if (actualKeys.length !== expectedKeys.length || actualKeys.some((name, index) => name !== expectedKeys[index]))
        return;
    for (const [name, type] of Object.entries(expected)) {
        const item = params[name];
        if (type === "text" ? typeof item !== "string" || !scalarText(item) : type === "integer" ? typeof item !== "number" || !Number.isSafeInteger(item) : typeof item !== "number" || !Number.isFinite(item))
            return;
    }
    const parsed = { key, params: Object.freeze({ ...params }) };
    Object.defineProperty(parsed, UI_MESSAGE, { value: true });
    return Object.freeze(parsed) as UiMessage;
}
/** Use this entry point when the message itself is received as raw JSON. */
export function parseUiMessageJson(source: string): UiMessage | undefined {
    try { return parseUiMessage(parseStrictJson(source)); } catch { return; }
}
export function uiTemplateParameterNames(template: string): string[] {
    return [...template.matchAll(/\{([A-Za-z][A-Za-z0-9]*)\}/g)].map(match => match[1]);
}
export function renderUiTemplate(template: string, resolve: (name: string) => string): string {
    return template.replace(/\{([A-Za-z][A-Za-z0-9]*)\}/g, (_placeholder, name: string) => resolve(name));
}
/** Author objects cannot become product messages merely by resembling their wire shape. */
export function uiTextValue(value: unknown): string {
    return renderUiText(locale.current, value);
}
function renderUiText(language: unknown, value: unknown, translations?: Readonly<Partial<Record<MessageKey, string>>>): string {
    if (typeof value === "function")
        return renderUiText(language, (value as () => UiText)(), translations);
    if (value && typeof value === "object" && ((value as {
        [UI_MESSAGE]?: boolean;
    })[UI_MESSAGE] || (value as {
        [LOCAL_DISPLAY]?: boolean;
    })[LOCAL_DISPLAY]))
        return renderUiMessage(language, value as UiDisplayMessage, translations);
    return value === null || value === undefined ? "" : String(value);
}
export function normalizeUiLocale(value: unknown): UiLocale {
    return value === "en-US" ? "en-US" : "zh-CN";
}
/** Pure template resolution: unknown languages and absent translations use the Chinese source. */
export function renderUiMessage(locale: unknown, spec: UiDisplayMessage, translations: Readonly<Partial<Record<MessageKey, string>>> = enUS): string {
    const template = normalizeUiLocale(locale) === "en-US" ? translations[spec.key] ?? zhCN[spec.key] : zhCN[spec.key];
    return renderUiTemplate(template, (key: string) => {
        const value = (spec.params as Readonly<Record<string, UiText | number>>)[key];
        if (value === undefined)
            throw new Error(`Missing UI message parameter ${spec.key}.${key}`);
        return renderUiText(locale, value, translations);
    });
}
class UiLocaleContext {
    private value: UiLocale = "zh-CN";
    private readonly listeners = new Set<(locale: UiLocale) => void>();
    get current(): UiLocale { return this.value; }
    /** Presentation only. Preference persistence and its audience belong to the caller. */
    set(value: UiLocale): void {
        if (value !== "zh-CN" && value !== "en-US")
            throw new Error("Unsupported UI locale");
        if (this.value === value)
            return;
        this.value = value;
        const errors: unknown[] = [];
        for (const listener of [...this.listeners]) {
            try {
                listener(value);
            }
            catch (error) {
                errors.push(error);
            }
        }
        if (errors.length)
            throw new AggregateError(errors, "UI locale changed with incomplete label updates");
    }
    subscribe(listener: (locale: UiLocale) => void): () => void {
        this.listeners.add(listener);
        return () => this.listeners.delete(listener);
    }
}
export const locale = new UiLocaleContext();
export function t(spec: UiDisplayMessage): string { return renderUiMessage(locale.current, spec); }
/** Own this scope alongside the component and dispose it on removal or authorization retirement. */
export class LocaleScope {
    private readonly updates = new Set<() => void>();
    private unsubscribe?: () => void;
    private disposed = false;
    private bind(update: () => void): () => void {
        if (this.disposed)
            throw new Error("Locale scope is disposed");
        update();
        this.updates.add(update);
        if (!this.unsubscribe) {
            const reference = new WeakRef(this);
            let unsubscribe: () => void;
            unsubscribe = locale.subscribe(() => { const scope = reference.deref(); if (scope)
                scope.refresh();
            else
                unsubscribe(); });
            this.unsubscribe = unsubscribe;
        }
        return () => {
            this.updates.delete(update);
            if (!this.updates.size) {
                this.unsubscribe?.();
                this.unsubscribe = undefined;
            }
        };
    }
    bindText(node: Node, source: UiText): () => void {
        // Own one text slot, so later icons, labels or controls remain the same nodes.
        const text = node.nodeType === 3 || !node.ownerDocument ? node : [...node.childNodes].find(child => child.nodeType === 3) ?? node.insertBefore(node.ownerDocument.createTextNode(""), node.firstChild);
        return this.bind(() => { text.textContent = uiTextValue(source); });
    }
    bindAttr(node: Element, attribute: UiDisplayAttribute, source: UiText): () => void {
        return this.bind(() => { node.setAttribute(attribute, uiTextValue(source)); });
    }
    bindValidity(control: HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement, source: UiText): () => void {
        return this.bind(() => { control.setCustomValidity(uiTextValue(source)); });
    }
    refresh(): void {
        if (this.disposed)
            return;
        const errors: unknown[] = [];
        for (const update of this.updates) {
            try {
                update();
            }
            catch (error) {
                errors.push(error);
            }
        }
        if (errors.length)
            throw new AggregateError(errors, "Incomplete UI label updates");
    }
    dispose(): void { this.disposed = true; this.unsubscribe?.(); this.unsubscribe = undefined; this.updates.clear(); }
}
interface NodeLabels {
    node: Node;
    scope: LocaleScope;
    fields: Map<string, () => void>;
    reference?: WeakRef<NodeLabels>;
}
const nodeLabels = new WeakMap<Node, NodeLabels>();
const waitingForConnection = new Set<WeakRef<NodeLabels>>();
let connectionObserver: MutationObserver | undefined;
function labelsFor(node: Node): NodeLabels {
    let labels = nodeLabels.get(node);
    if (!labels) {
        labels = { node, scope: new LocaleScope(), fields: new Map() };
        nodeLabels.set(node, labels);
    }
    return labels;
}
function waitForConnection(labels: NodeLabels): void {
    labels.reference ??= new WeakRef(labels);
    waitingForConnection.add(labels.reference);
    if (!connectionObserver && typeof MutationObserver !== "undefined" && labels.node.ownerDocument) {
        connectionObserver = new MutationObserver(() => {
            if (!waitingForConnection.size)
                return;
            const errors: unknown[] = [];
            for (const reference of [...waitingForConnection]) {
                const current = reference.deref();
                if (!current)
                    waitingForConnection.delete(reference);
                else if (current.node.isConnected) {
                    try {
                        current.scope.refresh();
                        waitingForConnection.delete(reference);
                    }
                    catch (error) {
                        errors.push(error);
                    } // Keep this item available for a later retry.
                }
            }
            if (errors.length)
                throw new AggregateError(errors, "Incomplete reconnected UI label updates");
        });
        connectionObserver.observe(labels.node.ownerDocument, { childList: true, subtree: true });
    }
}
function releaseLabel(node: Node, field: string): void {
    const labels = nodeLabels.get(node);
    if (!labels)
        return;
    labels.fields.get(field)?.();
    labels.fields.delete(field);
    if (!labels.fields.size) {
        labels.scope.dispose();
        if (labels.reference)
            waitingForConnection.delete(labels.reference);
        nodeLabels.delete(node);
    }
}
/** Retire bindings and captured message parameters together with their owning display. */
export function disposeUiTree(node: Node): void {
    const pending = [node];
    while (pending.length) {
        const current = pending.pop()!;
        const labels = nodeLabels.get(current);
        if (labels) {
            labels.scope.dispose();
            labels.fields.clear();
            if (labels.reference)
                waitingForConnection.delete(labels.reference);
            nodeLabels.delete(current);
        }
        pending.push(...current.childNodes);
    }
}
/** Replacing a display retires its captured parameters while retaining explicitly reused subtrees. */
export function replaceUiChildren(parent: Node & ParentNode, ...children: (Node | string)[]): void {
    const retained = new Set(children.filter((child): child is Node => typeof child !== "string"));
    const pending = [...parent.childNodes];
    while (pending.length) {
        const node = pending.pop()!;
        if (retained.has(node))
            continue;
        const labels = nodeLabels.get(node);
        if (labels) {
            labels.scope.dispose();
            labels.fields.clear();
            if (labels.reference)
                waitingForConnection.delete(labels.reference);
            nodeLabels.delete(node);
        }
        pending.push(...node.childNodes);
    }
    parent.replaceChildren(...children);
}
/** Explicit product-label setter. Plain author text retires any former product binding. */
export function setUiText<T extends UiText | null>(node: Node, value: T): T {
    releaseLabel(node, "text");
    for (const child of node.childNodes)
        disposeUiTree(child);
    if (typeof value === "string" || value === null) {
        node.textContent = value;
        return value;
    }
    node.textContent = "";
    const slot = node.nodeType === 3 ? node : node.ownerDocument!.createTextNode("");
    if (slot !== node)
        node.appendChild(slot);
    const labels = labelsFor(node);
    let initial = true;
    const target = { get textContent() { return slot.textContent; }, set textContent(text: string | null) {
            if (initial || node.isConnected)
                slot.textContent = text;
            else
                waitForConnection(labels);
            initial = false;
        } };
    labels.fields.set("text", labels.scope.bindText(target as unknown as Text, value));
    return value;
}
export function setUiAttr<T extends UiText>(node: Element, attribute: UiDisplayAttribute, value: T): T {
    releaseLabel(node, attribute);
    if (typeof value === "string") {
        node.setAttribute(attribute, value);
        return value;
    }
    const labels = labelsFor(node);
    let initial = true;
    const target = { setAttribute(name: string, text: string) {
            if (initial || node.isConnected)
                node.setAttribute(name, text);
            else
                waitForConnection(labels);
            initial = false;
        } };
    labels.fields.set(attribute, labels.scope.bindAttr(target as Element, attribute, value));
    return value;
}
export function formatUiNumber(value: number | bigint, options?: Intl.NumberFormatOptions): string {
    return new Intl.NumberFormat(locale.current, options).format(value);
}
export function formatUiTimestamp(value: Date | number, options?: Intl.DateTimeFormatOptions): string {
    return new Intl.DateTimeFormat(locale.current, options).format(value);
}
export function formatUiPercent(value: number, options?: Intl.NumberFormatOptions): string {
    return formatUiNumber(value, { style: "percent", ...options });
}
/** Product errors retain their typed descriptor; native/compiler diagnostics stay original text. */
export class UiError extends Error {
    constructor(readonly spec: UiText) { super(); this.name = "UiError"; Object.defineProperty(this, "message", { get: () => uiTextValue(this.spec) }); }
}
export function errorUiText(error: unknown): UiText { return error instanceof UiError ? error.spec : error instanceof Error ? error.message : String(error); }
/** A standalone display slot for labels composed alongside controls or author text. */
export function uiTextNode(value: UiText): Text { const text = document.createTextNode(""); setUiText(text, value); return text; }
/** Localized browser validity text retains the exact control and validity meaning. */
export function setUiValidity(control: HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement, value: UiText): void {
    releaseLabel(control, "validation");
    if (typeof value === "string") {
        control.setCustomValidity(value);
        return;
    }
    const labels = labelsFor(control);
    labels.fields.set("validation", labels.scope.bindValidity(control, value));
}
/** Bind only explicitly declared slots in a product-owned static surface, once at construction. */
export function bindDeclaredUi(root: ParentNode): void {
    const bind = (node: Element, declaration: string, attribute?: "aria-label" | "title" | "placeholder") => {
        const spec = parseUiMessage({ key: node.getAttribute(declaration), params: {} });
        if (!spec)
            throw new Error("Invalid declared product message");
        if (attribute)
            setUiAttr(node, attribute, spec);
        else
            setUiText(node, spec);
    };
    for (const node of root.querySelectorAll("[data-ui-message]"))
        bind(node, "data-ui-message");
    for (const attribute of ["aria-label", "title", "placeholder"] as const)
        for (const node of root.querySelectorAll(`[data-ui-${attribute}]`))
            bind(node, `data-ui-${attribute}`, attribute);
}
