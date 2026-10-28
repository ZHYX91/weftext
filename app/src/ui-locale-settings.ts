import { parseStrictJson } from "./strict-json";
import { locale, message, LocaleScope, disposeUiTree, UiError, type UiLocale } from "./ui-locale";

export interface ClientPreferenceSnapshot { revision: number; preferences: { uiLocale: UiLocale } }
export interface LocalePreferenceAdapter {
  read(): Promise<ClientPreferenceSnapshot>;
  write(expectedRevision: number, uiLocale: UiLocale): Promise<ClientPreferenceSnapshot>;
}

export function decodeClientPreferences(value: unknown): ClientPreferenceSnapshot {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new UiError(message("error.invalidResponse"));
  const record = value as Record<string, unknown>, preferences = record.preferences as Record<string, unknown> | undefined;
  if (Object.keys(record).sort().join(",") !== "preferences,revision" || !Number.isSafeInteger(record.revision) || (record.revision as number) < 0 || !preferences || typeof preferences !== "object" || Array.isArray(preferences) || Object.keys(preferences).join(",") !== "uiLocale" || preferences.uiLocale !== "zh-CN" && preferences.uiLocale !== "en-US") throw new UiError(message("error.invalidResponse"));
  return { revision: record.revision as number, preferences: { uiLocale: preferences.uiLocale } };
}

function validateChoice(revision: number, uiLocale: UiLocale): void {
  if (!Number.isSafeInteger(revision) || revision < 0 || uiLocale !== "zh-CN" && uiLocale !== "en-US") throw new UiError(message("error.invalidResponse"));
}

/** Core owns this preference, so a new loopback port still reads the same profile. */
export function localLocalePreferences(request: <T>(path: string, body?: unknown) => Promise<T>): LocalePreferenceAdapter {
  return {
    read: async () => decodeClientPreferences(await request("/api/client-preferences/read", {})),
    write: async (expectedRevision, uiLocale) => { validateChoice(expectedRevision, uiLocale); return decodeClientPreferences(await request("/api/client-preferences/update", { expectedRevision, preferences: { uiLocale } })); },
  };
}

/** This is a personal display setting; the key contains no token or protected resource context. */
export function hostedLocalePreferences(storage: Storage, deployment: string, actor: string): LocalePreferenceAdapter {
  if (!deployment || !actor) throw new UiError(message("error.invalidResponse"));
  const key = `weftext-interface-locale:${encodeURIComponent(deployment)}:${encodeURIComponent(actor)}`;
  return browserLocalePreferences(storage, key);
}

export function publicLocalePreferences(storage: Storage, origin: string): LocalePreferenceAdapter {
  if (new URL(origin).origin !== origin) throw new UiError(message("error.invalidResponse"));
  return browserLocalePreferences(storage, `weftext-interface-locale-public:${encodeURIComponent(origin)}`);
}

function browserLocalePreferences(storage: Storage, key: string): LocalePreferenceAdapter {
  const read = (): ClientPreferenceSnapshot => {
    const raw = storage.getItem(key); if (raw === null) return { revision: 0, preferences: { uiLocale: "zh-CN" } };
    if (new TextEncoder().encode(raw).length > 16 * 1024) throw new UiError(message("error.clientPreferencesByteLimit"));
    let record: { version?: unknown; revision?: unknown; preferences?: unknown };
    try { record = parseStrictJson(raw) as typeof record; } catch { throw new UiError(message("error.clientPreferencesRecordUnsupported")); }
    if (!record || Object.keys(record).sort().join(",") !== "preferences,revision,version" || record.version !== 1) throw new UiError(message("error.clientPreferencesRecordUnsupported"));
    const result = decodeClientPreferences({ revision: record.revision, preferences: record.preferences }); if (result.revision === 0) throw new UiError(message("error.clientPreferencesRecordUnsupported")); return result;
  };
  return { read: async () => read(), write: async (expectedRevision, uiLocale) => {
    validateChoice(expectedRevision, uiLocale);
    const persist = () => { const original = read(); if (original.revision !== expectedRevision) throw new UiError(message("error.clientPreferencesConflict")); if (expectedRevision === Number.MAX_SAFE_INTEGER) throw new UiError(message("error.clientPreferencesRevisionExhausted")); const result = { revision: expectedRevision + 1, preferences: { uiLocale } }; storage.setItem(key, JSON.stringify({ version: 1, ...result })); const actual = read(); if (actual.revision !== result.revision || actual.preferences.uiLocale !== uiLocale) throw new UiError(message("error.clientPreferencesReadbackMismatch")); return actual; };
    // The lock serializes cooperating pages of this authenticated deployment origin.
    return typeof window !== "undefined" && navigator.locks ? navigator.locks.request(key, persist) : persist();
  } };
}

/** Own one instance per active surface/audience and retire it before an authorization change. */
export class InterfaceLocaleControl {
  readonly element = document.createElement("div");
  private readonly scope = new LocaleScope();
  private readonly select = document.createElement("select");
  private readonly status = document.createElement("span");
  private readonly retry = document.createElement("button");
  private snapshot?: ClientPreferenceSnapshot;
  private disposed = false;
  private version = 0;
  private pending = 0;
  private unconfirmed = false;
  private unbindStatus?: () => void;
  private queue: Promise<void> = Promise.resolve();
  private readonly unsubscribe: () => void;
  private confirmedCut?: string;
  private readonly timer: ReturnType<typeof setInterval>;

  constructor(private readonly adapter: LocalePreferenceAdapter, private readonly confirmed?: () => void) {
    this.element.className = "interface-locale";
    const label = document.createElement("label"), title = document.createElement("span");
    this.scope.bindText(title, message("locale.label")); label.append(title, this.select);
    this.select.dataset.controlRole = "interface-locale";
    this.select.dataset.controlKey = JSON.stringify(["personal-preference", "uiLocale"]);
    this.scope.bindAttr(this.select, "aria-label", message("locale.label"));
    for (const [value, key] of [["zh-CN", "locale.zhCN"], ["en-US", "locale.enUS"]] as const) { const option = document.createElement("option"); option.value = value; this.scope.bindText(option, message(key)); this.select.append(option); }
    this.select.value = locale.current; this.status.setAttribute("role", "status");
    this.retry.type = "button"; this.scope.bindText(this.retry, message("common.retry")); this.retry.hidden = true;
    this.retry.onclick = () => { void this.refresh(true); };
    this.element.append(label, this.status, this.retry);
    this.unsubscribe = locale.subscribe(value => { this.select.value = value; });
    this.select.onchange = () => { const chosen = this.select.value as UiLocale, version = ++this.version; locale.set(chosen); this.unconfirmed = true; this.pending++; this.display(); this.queue = this.queue.then(() => this.persist(chosen, version)).finally(() => { this.pending--; this.display(); }); };
    void this.refresh();
    this.timer = setInterval(() => { if (!this.disposed && !this.pending && !this.unconfirmed && document.visibilityState === "visible") void this.refresh(); }, 2500);
  }

  private confirm(actual: ClientPreferenceSnapshot): void {
    const cut = JSON.stringify([actual.revision, actual.preferences.uiLocale]);
    if (cut === this.confirmedCut) return;
    this.confirmed?.();
    this.confirmedCut = cut;
  }

  private display(): void {
    if (this.disposed) return;
    this.unbindStatus?.(); this.unbindStatus = undefined; this.status.replaceChildren();
    this.status.hidden = !this.unconfirmed;
    if (this.unconfirmed) this.unbindStatus = this.scope.bindText(this.status, message(this.snapshot ? "locale.unsaved" : "locale.unavailable"));
    this.retry.hidden = !this.unconfirmed || !!this.pending;
  }

  async refresh(explicit = false): Promise<void> {
    const version = this.version;
    try {
      const actual = await this.adapter.read();
      if (this.disposed || version !== this.version || this.pending) return;
      this.snapshot = actual;
      if (!this.unconfirmed || explicit) { this.unconfirmed = false; locale.set(actual.preferences.uiLocale); this.confirm(actual); }
    } catch { if (!this.disposed && version === this.version) this.unconfirmed = true; }
    this.display();
  }

  private async persist(chosen: UiLocale, version: number): Promise<void> {
    if (this.disposed || version !== this.version) return;
    try {
      const basis = this.snapshot ?? await this.adapter.read();
      if (this.disposed || version !== this.version) return;
      const actual = await this.adapter.write(basis.revision, chosen);
      if (this.disposed) return;
      this.snapshot = actual;
      if (version === this.version) { this.unconfirmed = actual.preferences.uiLocale !== chosen; if (!this.unconfirmed) this.confirm(actual); }
    } catch {
      // A failed readback does not prove the write failed. Inspect the actual setting first.
      try { const actual = await this.adapter.read(); if (!this.disposed) { this.snapshot = actual; if (version === this.version) { this.unconfirmed = actual.preferences.uiLocale !== chosen; if (!this.unconfirmed) this.confirm(actual); } } }
      catch { if (!this.disposed && version === this.version) this.unconfirmed = true; }
    }
  }

  dispose(): void { if (this.disposed) return; this.disposed = true; this.version++; clearInterval(this.timer); this.unsubscribe(); this.scope.dispose(); disposeUiTree(this.element); this.element.remove(); this.snapshot = undefined; }
}
