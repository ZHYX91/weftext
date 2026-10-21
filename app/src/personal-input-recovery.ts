import { RecoveryTextCache, type StoragePort } from "./recovery-text-cache";
import { hasPersonalInput, inputTexts, type InputOrigin, type InputPiece } from "./source-input-origin";

export interface PersonalInputRecord {
  id: string; resource: string; reference?: string; baseHash?: string; origin: InputOrigin;
  pending?: { operationId: string; status: "unresolved" | "committed" | "cancelled" | "resolved" };
  sequence: number;
}
type PersonalInputUpdate = Omit<PersonalInputRecord, "sequence">;
interface Records { version: 1; records: PersonalInputRecord[] }
const invalid = () => new Error("本人输入恢复记录无法完整读取；原件保留，未采用旧正文或资格。");
const integer = (value: unknown) => Number.isSafeInteger(value) && (value as number) >= 0;
function mapTexts(value: unknown, convert: (value: unknown, key: string) => unknown): Records {
  const data = value as Records;
  if (!data || data.version !== 1 || !Array.isArray(data.records) || data.records.length > 128) throw invalid();
  const ids = new Set<string>();
  return { version: 1, records: data.records.map(record => {
    if (!record || typeof record.id !== "string" || !/^[a-zA-Z0-9:-]{1,96}$/.test(record.id) || ids.has(record.id) || typeof record.resource !== "string" || !record.resource || !integer(record.sequence)) throw invalid();
    ids.add(record.id);
    if (record.reference !== undefined && (typeof record.reference !== "string" || !record.reference.startsWith("weftext-node://")) || record.baseHash !== undefined && !/^[a-f0-9]{64}$/.test(record.baseHash)) throw invalid();
    const origin = record.origin;
    if (!origin || !integer(origin.baseLength) || !integer(origin.revision) || !Array.isArray(origin.pieces) || !Array.isArray(origin.detached) || origin.pieces.length > 100000 || origin.detached.length > 10000) throw invalid();
    const pieces = origin.pieces.map((piece, index): InputPiece => {
      if (!piece || typeof piece !== "object") throw invalid();
      if ("text" in piece) return { text: convert(piece.text, `${record.id}/piece/${index}`) as string };
      if ("base" in piece && integer(piece.base) && integer(piece.length) && piece.base + piece.length <= origin.baseLength) return { base: piece.base, length: piece.length };
      if ("opaque" in piece && integer(piece.opaque)) return { opaque: piece.opaque };
      throw invalid();
    });
    const pending = record.pending;
    if (pending && (typeof pending.operationId !== "string" || !pending.operationId || !["unresolved", "committed", "cancelled", "resolved"].includes(pending.status))) throw invalid();
    return { id: record.id, resource: record.resource, ...(record.reference ? { reference: record.reference } : {}), ...(record.baseHash ? { baseHash: record.baseHash } : {}), sequence: record.sequence, origin: { baseLength: origin.baseLength, revision: origin.revision, pieces, detached: origin.detached.map(entry => { if (!entry || typeof entry.id !== "string" || !/^[a-zA-Z0-9:-]{1,96}$/.test(entry.id)) throw invalid(); return { id: entry.id, text: convert(entry.text, `${record.id}/detached/${entry.id}`) as string }; }) }, ...(pending ? { pending: { operationId: pending.operationId, status: pending.status } } : {}) };
  }) };
}

/** Actor-owned new input only. Old protected cache cleanup never deletes this namespace. */
export class PersonalInputRecovery {
  private audience = "";
  private memory = new Map<string, Records>();
  private readFailures = new Map<string, Error>();
  private issues = new Map<string, string>();
  private cache: RecoveryTextCache<Records>;
  constructor(storage: StoragePort) {
    this.cache = new RecoveryTextCache(storage, () => `weftext-personal-input:${encodeURIComponent(this.audience)}:`, mapTexts, prefix => `${prefix}text:`);
  }
  private state(audience: string): Records {
    if (!audience) throw invalid(); this.audience = audience;
    let state = this.memory.get(audience);
    if (!state) {
      try { state = this.cache.read("state") ?? { version: 1, records: [] }; }
      catch (error) { state = { version: 1, records: [] }; this.readFailures.set(audience, error instanceof Error ? error : invalid()); }
      this.memory.set(audience, state);
    }
    return state;
  }
  list(audience: string): PersonalInputRecord[] { return structuredClone(this.state(audience).records); }
  notice(audience: string): string | undefined { this.state(audience); return this.issues.get(audience) ?? (this.readFailures.has(audience) ? "原持久记录无法读取，原件未覆盖；本次输入仅保留在内存。" : undefined); }
  update(audience: string, updates: PersonalInputUpdate[]): void {
    const state = this.state(audience);
    const hadRecords = !!state.records.length;
    for (const update of updates) {
      const old = state.records.findIndex(record => record.id === update.id);
      const record = { ...structuredClone(update), ...(!update.pending && old >= 0 && state.records[old].pending ? { pending: state.records[old].pending } : {}), sequence: (old < 0 ? 0 : state.records[old].sequence) + 1 };
      if (!hasPersonalInput(record.origin) && !record.pending) { if (old >= 0) state.records.splice(old, 1); }
      else if (old < 0) state.records.push(record); else state.records[old] = record;
    }
    // Keep the newest pure input in memory even if quota/OS refuses persistence.
    if (!hadRecords && !state.records.length) return;
    if (state.records.length > 128) { const message = "本页本人输入恢复记录已达预算；当前文字仍在内存，请先取回并明确移除旧记录。"; this.issues.set(audience, message); throw new Error(message); }
    if (this.readFailures.has(audience)) throw new Error("原本人输入记录无法读取，未覆盖原件；本次新输入仅保留在内存，请先取回。");
    try { this.cache.write("state", state); this.issues.delete(audience); }
    catch (error) { this.issues.set(audience, "浏览器未能可靠写入；最新本人输入仅在内存，旧持久记录保留，请先取回。"); throw error; }
  }
  current(audience: string, record: PersonalInputRecord): PersonalInputRecord {
    const current = this.state(audience).records.find(entry => entry.id === record.id);
    if (!current || current.sequence !== record.sequence) throw new Error("恢复期间输入又发生变化，未应用较早的记录。");
    return structuredClone(current);
  }
  settle(audience: string, record: PersonalInputRecord, status: "committed" | "cancelled" | "resolved"): void {
    const current = this.current(audience, record); if (!current.pending) throw invalid();
    this.update(audience, [{ ...current, pending: { ...current.pending, status } }]);
  }
  remove(audience: string, record: PersonalInputRecord): void {
    const current = this.current(audience, record);
    if (current.pending?.status === "unresolved") throw new Error("原保存操作尚未核定，未移除其原编号。");
    const state = this.state(audience), next = { ...state, records: state.records.filter(entry => entry.id !== record.id) };
    this.memory.set(audience, next);
    if (this.readFailures.has(audience) || next.records.length > 128) { this.issues.set(audience, "本次只从内存移除；尚未达到可写条件，原持久记录未覆盖。请继续取回或移除剩余记录。"); return; }
    try { this.cache.write("state", next); this.issues.delete(audience); }
    catch { this.issues.set(audience, "本次只从内存移除；浏览器拒绝写入，旧持久记录仍保留，重新载入可能再次出现。"); }
  }
}
export function personalInputText(record: PersonalInputRecord): string { return inputTexts(record.origin).join("\n"); }
