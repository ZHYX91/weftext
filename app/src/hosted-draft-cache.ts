import type { HostedDraft } from "./hosted-state";

interface StoragePort { getItem(key: string): string | null; setItem(key: string, value: string): void; removeItem(key: string): void }
interface TextRef { snapshot: string; from: number; to: number; insert: string }
type StoredText = string | TextRef;
interface Field { text: string; stored: StoredText }
interface Cached { fields: Map<string, Field>; snapshots: Map<string, string>; record: string | null }
const INLINE = 64 * 1024;
const invalid = () => new Error("浏览器恢复稿无法完整读取；原记录保留，未采用另一份正文。");
const superseded = () => new Error("浏览器恢复稿已由其他页面更新；当前输入保留，请先复制并重新载入。");
const snapshotId = /^[0-9a-f]{32}$/;

/** Lossless UTF-16 recovery encoding, independent of author/source-map authority. */
function difference(base: string, value: string, snapshot: string): TextRef {
  let from = 0, to = base.length, end = value.length;
  while (from < to && from < end && base.charCodeAt(from) === value.charCodeAt(from)) from++;
  while (to > from && end > from && base.charCodeAt(to - 1) === value.charCodeAt(end - 1)) { to--; end--; }
  return { snapshot, from, to, insert: value.slice(from, end) };
}

/** Every edit synchronously commits its latest recovery metadata. Large immutable
 * text is written once; small differences never serialize that text again.
 * The metadata is the single atomic pointer to complete source/Base/pending data.
 */
export class HostedDraftCache {
  private prefix = "";
  private retained = new Map<string, Cached>();
  constructor(private storage: StoragePort, private audiencePrefix: () => string) {}
  clearMemory(): void { this.retained.clear(); }
  private scope(): string {
    const prefix = this.audiencePrefix();
    if (prefix !== this.prefix) { this.clearMemory(); this.prefix = prefix; }
    if (!prefix.startsWith("weftext-hosted-draft:")) throw invalid();
    return prefix;
  }
  private snapshotKey(resource: string, id: string): string { return this.scope().replace(/^weftext-hosted-draft:/, "weftext-hosted-draft-text:") + encodeURIComponent(resource) + ":" + id; }
  private remember(resource: string, value: Cached): void {
    this.retained.delete(resource); this.retained.set(resource, value);
    if (this.retained.size > 8) this.retained.delete(this.retained.keys().next().value!);
  }
  read(resource: string): HostedDraft | undefined {
    const raw = this.storage.getItem(this.scope() + resource);
    if (raw === null) return undefined;
    const state: Cached = { fields: new Map(), snapshots: new Map(), record: raw };
    const text = (value: unknown, field: string): string => {
      let decoded: string;
      if (typeof value === "string") decoded = value;
      else {
        const ref = value as TextRef | null;
        if (!ref || typeof ref.snapshot !== "string" || !snapshotId.test(ref.snapshot) || !Number.isSafeInteger(ref.from) || !Number.isSafeInteger(ref.to) || typeof ref.insert !== "string") throw invalid();
        let base = state.snapshots.get(ref.snapshot);
        if (base === undefined) {
          const stored = this.storage.getItem(this.snapshotKey(resource, ref.snapshot));
          if (stored === null) throw invalid();
          base = stored; state.snapshots.set(ref.snapshot, base);
        }
        if (ref.from < 0 || ref.to < ref.from || ref.to > base.length) throw invalid();
        decoded = base.slice(0, ref.from) + ref.insert + base.slice(ref.to);
      }
      state.fields.set(field, { text: decoded, stored: value as StoredText }); return decoded;
    };
    let parsed;
    try { parsed = JSON.parse(raw); } catch { throw invalid(); }
    const value = this.mapTexts(parsed, text);
    this.remember(resource, state); return value as HostedDraft;
  }
  write(resource: string, draft: HostedDraft): void {
    const key = this.scope() + resource;
    if (!this.retained.has(resource)) this.read(resource);
    const previous = this.retained.get(resource) ?? { fields: new Map<string, Field>(), snapshots: new Map<string, string>(), record: null };
    if (this.storage.getItem(key) !== previous.record) throw superseded();
    const state: Cached = { fields: new Map(), snapshots: new Map(previous.snapshots), record: previous.record }, created: string[] = [];
    const encode = (value: unknown, field: string): StoredText => {
      if (typeof value !== "string") throw invalid();
      let stored: StoredText = value;
      const old = previous.fields.get(field);
      if (old?.text === value && (value.length <= INLINE || typeof old.stored !== "string")) stored = old.stored;
      else if (value.length > INLINE) {
        const equal = [...state.fields.values(), ...previous.fields.values()].find(entry => entry.text === value && typeof entry.stored !== "string");
        if (equal) stored = equal.stored;
        else {
          const anchor = old && typeof old.stored !== "string" ? old.stored.snapshot : [...state.snapshots.keys()][0];
          const base = anchor && state.snapshots.get(anchor);
          const delta = base === undefined ? undefined : difference(base, value, anchor!);
          if (delta && delta.insert.length <= INLINE) stored = delta;
          else {
            const id = Array.from(crypto.getRandomValues(new Uint8Array(16)), byte => byte.toString(16).padStart(2, "0")).join("");
            if (this.storage.getItem(this.snapshotKey(resource, id)) !== null) throw invalid();
            this.storage.setItem(this.snapshotKey(resource, id), value);
            created.push(id); state.snapshots.set(id, value);
            stored = { snapshot: id, from: 0, to: 0, insert: "" };
          }
        }
      }
      state.fields.set(field, { text: value, stored }); return stored;
    };
    try {
      const stored = this.mapTexts(draft, encode);
      const serialized = JSON.stringify(stored);
      if (this.storage.getItem(key) !== previous.record) throw superseded();
      this.storage.setItem(key, serialized); state.record = serialized;
    } catch (error) {
      for (const id of created) { try { this.storage.removeItem(this.snapshotKey(resource, id)); } catch { /* Preserve the previous committed pointer on quota/OS failure. */ } }
      throw error;
    }
    const used = new Set([...state.fields.values()].flatMap(field => typeof field.stored === "string" ? [] : [field.stored.snapshot]));
    this.remember(resource, state);
    for (const id of state.snapshots.keys()) if (!used.has(id)) {
      state.snapshots.delete(id);
      try { this.storage.removeItem(this.snapshotKey(resource, id)); } catch { /* Unreachable owned snapshots can remain; the new recovery record is complete. */ }
    }
  }
  private mapTexts(value: unknown, convert: (text: unknown, field: string) => unknown): unknown {
    if (!value || typeof value !== "object" || Array.isArray(value)) throw invalid();
    const draft = value as HostedDraft;
    const mapped = { ...draft, source: convert(draft.source, "source"), baseSource: convert(draft.baseSource, "baseSource") };
    if (draft.pending === undefined) return mapped;
    if (!draft.pending || typeof draft.pending !== "object" || typeof draft.pending.operationId !== "string") throw invalid();
    const pending = { ...draft.pending, source: convert(draft.pending.source, "pending") };
    if (draft.pending.request) pending.request = { ...draft.pending.request, source: convert(draft.pending.request.source, "request") } as typeof draft.pending.request;
    return { ...mapped, pending };
  }
}
