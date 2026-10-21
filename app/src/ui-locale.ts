import { enUS, zhCN, messageParamTypes, type MessageKey, type MessageParamsMap } from "./ui-messages";

export type UiLocale = "zh-CN" | "en-US";
export type { MessageKey } from "./ui-messages";
export type MessageParams<K extends MessageKey> = MessageParamsMap[K];
type MessageArgs<K extends MessageKey> = { [P in K]: [keyof MessageParams<P>] extends [never] ? [] : [params: MessageParams<P>] }[K];
export type UiMessage<K extends MessageKey = MessageKey> = { [P in K]: Readonly<{ key: P; params: MessageParams<P> }> }[K];
type AnyMessage = UiMessage;
export type MessageSource = AnyMessage | (() => AnyMessage);

export function message<K extends MessageKey>(key: K, ...args: MessageArgs<K>): UiMessage<K> {
  const spec = parseUiMessage({ key, params: args[0] ?? {} });
  if (!spec) throw new Error(`Invalid product message parameters: ${key}`);
  return spec as UiMessage<K>;
}

function scalarText(value: string): boolean {
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index);
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(++index);
      if (!(next >= 0xdc00 && next <= 0xdfff)) return false;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) return false;
  }
  return true;
}

/** Decode product messages explicitly; unknown/raw diagnostics have a separate display channel. */
export function parseUiMessage(value: unknown): UiMessage | undefined {
  if (!value || typeof value !== "object" || Array.isArray(value)) return;
  const wire = value as Record<string, unknown>;
  if (Object.keys(wire).sort().join(",") !== "key,params" || typeof wire.key !== "string" || !Object.hasOwn(zhCN, wire.key) || !wire.params || typeof wire.params !== "object" || Array.isArray(wire.params)) return;
  const key = wire.key as MessageKey, params = wire.params as Record<string, unknown>;
  const expected: Readonly<Record<string, string>> = messageParamTypes[key];
  const actualKeys = Object.keys(params).sort(), expectedKeys = Object.keys(expected).sort();
  if (actualKeys.length !== expectedKeys.length || actualKeys.some((name, index) => name !== expectedKeys[index])) return;
  for (const [name, type] of Object.entries(expected)) {
    const item = params[name];
    if (type === "text" ? typeof item !== "string" || !scalarText(item) : type === "integer" ? typeof item !== "number" || !Number.isSafeInteger(item) : typeof item !== "number" || !Number.isFinite(item)) return;
  }
  return Object.freeze({ key, params: Object.freeze({ ...params }) }) as UiMessage;
}

export function normalizeUiLocale(value: unknown): UiLocale {
  return value === "en-US" ? "en-US" : "zh-CN";
}

/** Pure template resolution: unknown languages and absent translations use the Chinese source. */
export function renderUiMessage(locale: unknown, spec: UiMessage, translations: Readonly<Partial<Record<MessageKey, string>>> = enUS): string {
  const template = normalizeUiLocale(locale) === "en-US" ? translations[spec.key] ?? zhCN[spec.key] : zhCN[spec.key];
  return template.replace(/\{([A-Za-z][A-Za-z0-9_]*)\}/g, (placeholder, key: string) => {
    const value = (spec.params as Readonly<Record<string, string | number>>)[key];
    if (value === undefined) throw new Error(`Missing UI message parameter ${spec.key}.${key}`);
    return String(value);
  });
}

class UiLocaleContext {
  private value: UiLocale = "zh-CN";
  private readonly listeners = new Set<(locale: UiLocale) => void>();
  get current(): UiLocale { return this.value; }
  /** Presentation only. Preference persistence and its audience belong to the caller. */
  set(value: UiLocale): void {
    if (value !== "zh-CN" && value !== "en-US") throw new Error("Unsupported UI locale");
    if (this.value === value) return;
    this.value = value;
    const errors: unknown[] = [];
    for (const listener of [...this.listeners]) { try { listener(value); } catch (error) { errors.push(error); } }
    if (errors.length) throw new AggregateError(errors, "UI locale changed with incomplete label updates");
  }
  subscribe(listener: (locale: UiLocale) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }
}

export const locale = new UiLocaleContext();
export function t(spec: UiMessage): string { return renderUiMessage(locale.current, spec); }

/** Own this scope alongside the component and dispose it on removal or authorization retirement. */
export class LocaleScope {
  private readonly updates = new Set<() => void>();
  private unsubscribe?: () => void;
  private disposed = false;
  private bind(update: () => void): () => void {
    if (this.disposed) throw new Error("Locale scope is disposed");
    update();
    this.updates.add(update);
    this.unsubscribe ??= locale.subscribe(() => this.refresh());
    return () => {
      this.updates.delete(update);
      if (!this.updates.size) { this.unsubscribe?.(); this.unsubscribe = undefined; }
    };
  }
  bindText(node: Text | HTMLElement, source: MessageSource): () => void {
    return this.bind(() => { node.textContent = t(typeof source === "function" ? source() : source); });
  }
  bindAttr(node: Element, attribute: "aria-label" | "title" | "placeholder", source: MessageSource): () => void {
    return this.bind(() => { node.setAttribute(attribute, t(typeof source === "function" ? source() : source)); });
  }
  refresh(): void {
    if (this.disposed) return;
    const errors: unknown[] = [];
    for (const update of this.updates) { try { update(); } catch (error) { errors.push(error); } }
    if (errors.length) throw new AggregateError(errors, "Incomplete UI label updates");
  }
  dispose(): void { this.disposed = true; this.unsubscribe?.(); this.unsubscribe = undefined; this.updates.clear(); }
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
