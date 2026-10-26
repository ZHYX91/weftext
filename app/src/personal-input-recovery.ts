import { RecoveryTextCache, type StoragePort } from "./recovery-text-cache";
import { hasPersonalInput, inputTexts, type InputOrigin, type InputPiece, type PendingInputCheckpoint, type PersonalText } from "./source-input-origin";
import type { AuthorInputOwner, AuthorProducerCheckpoint, ValuePath } from "./author-input";
import { IndexedDBPersonalInputBackend, PersonalInputStoreConflict, type PersonalInputBackend, type PersonalInputBundle } from "./personal-input-storage";
import type { CaptureRawProvenance, CaptureTextSlot } from "./capture-draft-state";

export type PersonalOperationRoute = { route: "known"; resource: string } | { route: "unknownOriginalRoute" };
export type PersonalPendingOperation = { operationId: string; status: "unresolved" | "committed" | "cancelled" | "resolved" } & PersonalOperationRoute;
export type PersonalOriginalRouteProof = {
  kind: "hostedDraft"; audience: string;
  pending: { operationId: string; resource: string; request: { operationId: string } };
} | {
  kind: "coreReceipt"; audience: string;
  receipt: { operationId: string; resource: string; status: "committed" | "cancelled" | "resolved" | "recoveryPending" | "notCommitted" };
};
export interface PersonalInputRecord {
  kind: "text";
  id: string; owner: Exclude<AuthorInputOwner, { kind: "capture" }>; slot?: string; path?: readonly ValuePath[];
  producer?: AuthorProducerCheckpoint;
  baseHash?: string; origin: InputOrigin; candidate?: PendingInputCheckpoint;
  pending?: PersonalPendingOperation;
  sequence: number;
}
type PersonalInputUpdate = Omit<PersonalInputRecord, "sequence" | "kind">;
export interface CaptureTransportRaw { fieldId: string; valuePath: readonly ValuePath[]; controlRole: string; text: string; provenance: CaptureRawProvenance }
export interface CaptureTransportInput {
  texts: Record<CaptureTextSlot,{text:string;provenance:CaptureRawProvenance}[]>; bodyMode: "text" | "markup" | null;
  rawInputs: CaptureTransportRaw[];
  detachedRawInputs: { retirementId: string; reason: "structure" | "schema" | "producer"; input: CaptureTransportRaw }[];
}
export interface CaptureTransportRecord {
  kind: "captureTransport"; id: string; owner: { kind: "capture"; draftId: string };
  producerId: string; producerRevision: number; baseRevision: number; sequence: number;
  pending: { producerRevision: number; command: { kind: "fullInput" | "authorInput"; expectedRevision: number; mutationId: string; fingerprint: string }; input: CaptureTransportInput } | null;
  input: CaptureTransportInput;
}
export type CaptureTransportCurrent = Omit<CaptureTransportRecord, "sequence" | "pending">;
type ProposalRecord = PersonalInputRecord | CaptureTransportRecord;
interface Records { version: 4; records: ProposalRecord[] }
const invalid = () => new Error("本人输入恢复记录无法完整读取；原件保留，未采用旧正文或资格。");
const integer = (value: unknown) => Number.isSafeInteger(value) && (value as number) >= 0;
const uuid = (value: unknown): value is string => typeof value === "string" && /^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/.test(value);
function exact(value: object, keys: string[]): void { if (Object.keys(value).sort().join("\0") !== keys.sort().join("\0")) throw invalid(); }
function captureTransport(record: CaptureTransportRecord, convert: (value: unknown, key: string) => unknown): CaptureTransportRecord {
  exact(record, ["kind", "id", "owner", "producerId", "producerRevision", "baseRevision", "sequence", "pending", "input"]);
  if (!record.owner || record.owner.kind !== "capture" || !uuid(record.owner.draftId) || !uuid(record.producerId) || !integer(record.producerRevision) || !integer(record.baseRevision) || record.baseRevision < 1) throw invalid();
  exact(record.owner, ["kind", "draftId"]);
  const pending = record.pending;
  if (pending) {
    exact(pending, ["producerRevision", "command", "input"]);
    const command = pending.command;
    if (!integer(pending.producerRevision) || pending.producerRevision > record.producerRevision || !command || !["fullInput", "authorInput"].includes(command.kind) || !integer(command.expectedRevision) || command.expectedRevision < 1 || !uuid(command.mutationId) || !/^[a-f0-9]{64}$/.test(command.fingerprint)) throw invalid();
    exact(command, ["kind", "expectedRevision", "mutationId", "fingerprint"]);
  } else if (pending !== null) throw invalid();
  const mapInput = (input: CaptureTransportInput, scope: string): CaptureTransportInput => {
  if (!input || input.bodyMode!==null && !["text", "markup"].includes(input.bodyMode) || !Array.isArray(input.rawInputs) || !Array.isArray(input.detachedRawInputs) || input.rawInputs.length + input.detachedRawInputs.length > 1024) throw invalid();
  exact(input, ["texts", "bodyMode", "rawInputs", "detachedRawInputs"]);
  const text = (value: unknown, key: string): string => {
    if (typeof value === "string" && value.length > 2 * 1024 * 1024) throw invalid();
    const mapped = convert(value, `${record.id}/${scope}/${key}`);
    if (typeof mapped === "string" && mapped.length > 2 * 1024 * 1024) throw invalid();
    return mapped as string;
  };
  const token = (value: unknown): string => { if (typeof value !== "string" || !value || value.length > 512) throw invalid(); return value; };
  const provenance = (value:CaptureRawProvenance): CaptureRawProvenance => {
    if(!value || !uuid(value.producerId) || !/^[a-f0-9]{32}$/.test(value.inputId) || !integer(value.revision)) throw invalid();
    exact(value,["producerId","inputId","revision"]);return {...value};
  };
  if(!input.texts) throw invalid();
  const slots:CaptureTextSlot[]=["name","title","body","rootInput","parentInput","parametersText"];
  exact(input.texts,slots);
  let proofCount=0;
  const texts=Object.fromEntries(slots.map(slot=>{
    const values=input.texts[slot];if(!Array.isArray(values)|| (proofCount+=values.length)>1024)throw invalid();
    const ids=new Set<string>();
    return[slot,values.map(own=>{if(!own)throw invalid();exact(own,["text","provenance"]);const proof=provenance(own.provenance),id=JSON.stringify([proof.producerId,proof.inputId]);if(ids.has(id))throw invalid();ids.add(id);return{text:text(own.text,`${slot}/${id}`),provenance:proof};})];
  })) as CaptureTransportInput["texts"];
  const raw = (value: CaptureTransportRaw, key: string): CaptureTransportRaw => {
    if (!value || !Array.isArray(value.valuePath) || value.valuePath.length > 32) throw invalid();
    exact(value, ["fieldId", "valuePath", "controlRole", "text", "provenance"]);
    if (!value.provenance || !uuid(value.provenance.producerId) || !/^[a-f0-9]{32}$/.test(value.provenance.inputId) || !integer(value.provenance.revision)) throw invalid();
    exact(value.provenance, ["producerId", "inputId", "revision"]);
    return { fieldId: token(value.fieldId), controlRole: token(value.controlRole), valuePath: value.valuePath.map(part => {
      if (part.kind === "member") { exact(part, ["kind", "key"]); return { kind: "member", key: token(part.key) }; }
      if (part.kind === "index" && integer(part.index)) { exact(part, ["kind", "index"]); return { kind: "index", index: part.index }; }
      throw invalid();
    }), text: text(value.text, key), provenance: { ...value.provenance } };
  };
  return {
    texts, bodyMode: input.bodyMode,
    rawInputs: input.rawInputs.map((value, index) => raw(value, `raw/${index}`)),
    detachedRawInputs: input.detachedRawInputs.map(value => {
      if (!value || !uuid(value.retirementId) || !["structure", "schema", "producer"].includes(value.reason)) throw invalid();
      exact(value, ["retirementId", "reason", "input"]);
      return { retirementId: value.retirementId, reason: value.reason, input: raw(value.input, `retired/${value.retirementId}`) };
    }),
  };
  };
  return { kind: "captureTransport", id: record.id, owner: { kind: "capture", draftId: record.owner.draftId }, producerId: record.producerId, producerRevision: record.producerRevision, baseRevision: record.baseRevision, sequence: record.sequence,
    input: mapInput(record.input, "current"), pending: pending ? { producerRevision: pending.producerRevision, command: { ...pending.command }, input: mapInput(pending.input, "pending") } : null,
  };
}
function mapTexts(value: unknown, convert: (value: unknown, key: string) => unknown): Records {
  const data = value as Records;
  if (!data || data.version !== 4 || !Array.isArray(data.records) || data.records.length > 128) throw invalid();
  const ids = new Set<string>(), operationRoutes = new Map<string, PersonalPendingOperation>();
  return { version: 4, records: data.records.map((record): ProposalRecord => {
    if (!record || typeof record.id !== "string" || !/^[a-zA-Z0-9:-]{1,128}$/.test(record.id) || ids.has(record.id) || !integer(record.sequence)) throw invalid();
    ids.add(record.id);
    if (record.kind === "captureTransport") return captureTransport(record, convert);
    if (record.kind !== "text") throw invalid();
    const string = (value: unknown, max = 4096): string => { if (typeof value !== "string" || !value || value.length > max) throw invalid(); return value; };
    const rawOwner = record.owner; let owner: PersonalInputRecord["owner"];
    if (!rawOwner || typeof rawOwner !== "object") throw invalid();
    switch (rawOwner.kind) {
      case "body": owner = { kind: "body", resource: string(rawOwner.resource), ...(rawOwner.reference ? { reference: string(rawOwner.reference) } : {}) }; if (owner.reference && !owner.reference.startsWith("weftext-node://")) throw invalid(); break;
      case "field": owner = { kind: "field", resource: string(rawOwner.resource), fieldId: string(rawOwner.fieldId) }; break;
      case "review": owner = { kind: "review", resource: string(rawOwner.resource), ...(rawOwner.annotationId ? { annotationId: string(rawOwner.annotationId) } : {}), ...(rawOwner.asset ? { asset: string(rawOwner.asset) } : {}), ...(rawOwner.revision ? { revision: string(rawOwner.revision) } : {}) }; break;
      case "library": owner = { kind: "library", role: string(rawOwner.role, 128) }; break;
      case "registry": owner = { kind: "registry", target: string(rawOwner.target) }; break;
      default: throw invalid();
    }
    if (record.slot !== undefined && (typeof record.slot !== "string" || record.slot.length > 4096) || record.path !== undefined && (!Array.isArray(record.path) || record.path.length > 32)) throw invalid();
    const path = record.path?.map(part => { if (part.kind === "member") return { kind: "member" as const, key: string(part.key) }; if (part.kind === "index" && integer(part.index)) return { kind: "index" as const, index: part.index }; throw invalid(); });
    if (record.baseHash !== undefined && !/^[a-f0-9]{64}$/.test(record.baseHash)) throw invalid();
    const producer = record.producer;
    if (producer) { exact(producer, ["producerId", "revision"]); if (!uuid(producer.producerId) || !integer(producer.revision)) throw invalid(); }
    const readOrigin = (origin: InputOrigin, scope: string): InputOrigin => {
    if (!origin || !integer(origin.baseLength) || !integer(origin.revision) || !Array.isArray(origin.pieces) || !Array.isArray(origin.detached) || origin.pieces.length > 100000 || origin.detached.length > 10000) throw invalid();
    const pieces = origin.pieces.map((piece, index): InputPiece => {
      if (!piece || typeof piece !== "object") throw invalid();
      if ("text" in piece) return { text: convert(piece.text, `${record.id}/${scope}/piece/${index}`) as string };
      if ("base" in piece && integer(piece.base) && integer(piece.length) && piece.base + piece.length <= origin.baseLength) return { base: piece.base, length: piece.length };
      if ("opaque" in piece && integer(piece.opaque)) return { opaque: piece.opaque };
      throw invalid();
    });
    const texts = (entries: PersonalText[] | undefined, name: string): PersonalText[] => {
      if (entries === undefined) return []; if (!Array.isArray(entries) || entries.length > 10000) throw invalid();
      return entries.map(entry => { if (!entry || typeof entry.id !== "string" || !/^[a-zA-Z0-9:-]{1,128}$/.test(entry.id) || !integer(entry.revision)) throw invalid(); return { id: entry.id, revision: entry.revision, text: convert(entry.text, `${record.id}/${scope}/${name}/${entry.id}`) as string }; });
    };
    return { baseLength: origin.baseLength, revision: origin.revision, pieces, detached: texts(origin.detached, "detached"), copyOnly: texts(origin.copyOnly, "copy") };
    };
    const pending = record.pending;
    if (pending !== undefined) {
      if (!pending || typeof pending !== "object" || Array.isArray(pending)) throw invalid();
      if (pending.route === "known") { exact(pending, ["operationId", "route", "resource", "status"]); string(pending.resource); }
      else if (pending.route === "unknownOriginalRoute") exact(pending, ["operationId", "route", "status"]);
      else throw invalid();
      if (typeof pending.operationId !== "string" || !pending.operationId) throw invalid();
      if (!["unresolved", "committed", "cancelled", "resolved"].includes(pending.status)) throw invalid();
      const previousRoute = operationRoutes.get(pending.operationId);
      if (previousRoute && !sameRoute(previousRoute, pending)) throw invalid();
      operationRoutes.set(pending.operationId, pending);
    }
    const candidate = record.candidate;
    if (candidate && (owner.kind !== "body" || candidate.operationId !== pending?.operationId || candidate.baseHash !== undefined && !/^[a-f0-9]{64}$/.test(candidate.baseHash))) throw invalid();
    return { kind: "text", id: record.id, owner, ...(record.slot !== undefined ? { slot: record.slot } : {}), ...(path ? { path } : {}), ...(record.baseHash ? { baseHash: record.baseHash } : {}), ...(producer ? { producer: { ...producer } } : {}), sequence: record.sequence, origin: readOrigin(record.origin, "confirmed"), ...(candidate ? { candidate: { operationId: candidate.operationId, baseHash: candidate.baseHash, origin: readOrigin(candidate.origin, "candidate") } } : {}), ...(pending ? { pending: pending.route === "known" ? { operationId: pending.operationId, route: "known", resource: pending.resource, status: pending.status } : { operationId: pending.operationId, route: "unknownOriginalRoute", status: pending.status } } : {}) };
  }) };
}

function legacyKeys(value: unknown, required: string[], optional: string[] = []): void {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw invalid();
  const keys = Object.keys(value);
  if (required.some(key => !keys.includes(key)) || keys.some(key => !required.includes(key) && !optional.includes(key))) throw invalid();
}
function legacyText(value: unknown): void {
  if (typeof value === "string") return;
  legacyKeys(value, ["snapshot", "from", "to", "insert"]);
}
function legacyOrigin(value: unknown): void {
  legacyKeys(value, ["baseLength", "pieces", "detached", "revision"], ["copyOnly"]);
  const origin = value as InputOrigin;
  if (!Array.isArray(origin.pieces) || !Array.isArray(origin.detached) || origin.copyOnly !== undefined && !Array.isArray(origin.copyOnly)) throw invalid();
  for (const piece of origin.pieces) {
    if (!piece || typeof piece !== "object") throw invalid();
    if ("text" in piece) { legacyKeys(piece, ["text"]); legacyText(piece.text); }
    else if ("base" in piece) legacyKeys(piece, ["base", "length"]);
    else legacyKeys(piece, ["opaque"]);
  }
  for (const text of [...origin.detached, ...(origin.copyOnly ?? [])]) { legacyKeys(text, ["id", "revision", "text"]); legacyText(text.text); }
}
/** Decode the complete closed v3 shape before converting it. No input owner
 * proves an operation route; unverifiable pending saves remain explicit. */
function upgradeVersion3(value: unknown): unknown {
  legacyKeys(value, ["version", "records"]);
  const data = value as { version: unknown; records: unknown[] };
  if (data.version !== 3 || !Array.isArray(data.records) || data.records.length > 128) throw invalid();
  return { version: 4, records: data.records.map(value => {
    if (!value || typeof value !== "object") throw invalid();
    if ((value as ProposalRecord).kind === "captureTransport") {
      captureTransport(value as CaptureTransportRecord, text => { legacyText(text); return text; });
      return value;
    }
    legacyKeys(value, ["kind", "id", "owner", "origin", "sequence"], ["slot", "path", "producer", "baseHash", "candidate", "pending"]);
    const record = value as PersonalInputRecord;
    if (record.kind !== "text" || !record.owner || typeof record.owner !== "object") throw invalid();
    const owner = record.owner;
    switch (owner.kind) {
      case "body": legacyKeys(owner, ["kind", "resource"], ["reference"]); break;
      case "field": legacyKeys(owner, ["kind", "resource", "fieldId"]); break;
      case "review": legacyKeys(owner, ["kind", "resource"], ["annotationId", "asset", "revision"]); break;
      case "library": legacyKeys(owner, ["kind", "role"]); break;
      case "registry": legacyKeys(owner, ["kind", "target"]); break;
      default: throw invalid();
    }
    legacyOrigin(record.origin);
    if (record.producer !== undefined) legacyKeys(record.producer, ["producerId", "revision"]);
    if (record.path !== undefined) {
      if (!Array.isArray(record.path)) throw invalid();
      for (const part of record.path) {
        if (part?.kind === "member") legacyKeys(part, ["kind", "key"]);
        else if (part?.kind === "index") legacyKeys(part, ["kind", "index"]);
        else throw invalid();
      }
    }
    if (record.candidate !== undefined) { legacyKeys(record.candidate, ["operationId", "origin"], ["baseHash"]); legacyOrigin(record.candidate.origin); }
    const pending = record.pending;
    if (pending === undefined) return record;
    legacyKeys(pending, ["operationId", "status"]);
    if (owner.kind !== "body" || typeof pending.operationId !== "string" || !pending.operationId || !["unresolved", "committed", "cancelled", "resolved"].includes(pending.status)) throw invalid();
    return { ...record, pending: { operationId: pending.operationId, status: pending.status, route: "unknownOriginalRoute" } };
  }) };
}
function sameTexts(left: ReadonlyMap<string, string>, right: ReadonlyMap<string, string>): boolean {
  return left.size === right.size && [...left].every(([id, text]) => right.get(id) === text);
}
function sameRoute(left: PersonalPendingOperation, right: PersonalPendingOperation): boolean {
  return left.route === right.route && (left.route !== "known" || right.route === "known" && left.resource === right.resource);
}

interface AudienceState {
  records: Records; base: Records; storage: MapStorage; cache: RecoveryTextCache<Records>; prefix: string;
  commitId: number; version: number; durableVersion: number; dirty: Set<string>; loaded: boolean;
  loading?: Promise<void>; running?: Promise<BrowserProposalAck>; failure?: Error; readFailure?: Error;
}
class MapStorage implements StoragePort {
  readonly values = new Map<string, string>();
  getItem(key: string): string | null { return this.values.get(key) ?? null; }
  setItem(key: string, value: string): void { this.values.set(key, value); }
  removeItem(key: string): void { this.values.delete(key); }
}
export interface BrowserProposalAck { kind: "browserProposal"; audience: string; commitId: number; version: number }
const empty = (): Records => ({ version: 4, records: [] });
// Codec reconstruction can reorder closed object members. Equality compares
// their meaning while preserving array order, revisions and exact UTF16 text.
const equal = (left: unknown, right: unknown) => {
  const ordered=(value:unknown):unknown=>Array.isArray(value)?value.map(ordered):value && typeof value==="object"?Object.fromEntries(Object.entries(value).sort(([a],[b])=>a<b?-1:a>b?1:0).map(([key,child])=>[key,ordered(child)])):value;
  return JSON.stringify(ordered(left))===JSON.stringify(ordered(right));
};
/** One pure-input proposal namespace. Version 4 freezes each pending operation resource independently of its input owner.
 * IndexedDB ACKs only prove browser
 * recovery transport; they never become Core/native close acknowledgements. */
export class PersonalInputRecovery {
  private audiences = new Map<string, AudienceState>();
  constructor(private readonly backend: PersonalInputBackend = new IndexedDBPersonalInputBackend()) {}
  private make(audience: string, bundle?: PersonalInputBundle): AudienceState {
    if (!audience || audience.length > 4096) throw invalid();
    const prefix = `weftext-personal-input:${encodeURIComponent(audience)}:`, storage = new MapStorage();
    if (bundle) { storage.setItem(`${prefix}state`, bundle.metadata); for (const [id, text] of bundle.texts) storage.setItem(`${prefix}text:state:${id}`, text); }
    const cache = new RecoveryTextCache<Records>(storage, () => prefix, mapTexts, value => `${value}text:`);
    const records = bundle ? cache.read("state") ?? empty() : empty();
    return { records, base: structuredClone(records), storage, cache, prefix, commitId: bundle?.commitId ?? 0, version: 0, durableVersion: 0, dirty: new Set(), loaded: !!bundle };
  }
  private async migrateVersion3(audience: string, original: PersonalInputBundle): Promise<PersonalInputBundle> {
    if (!integer(original.commitId) || original.commitId < 1) throw invalid();
    const converted = { ...original, metadata: JSON.stringify(upgradeVersion3(JSON.parse(original.metadata))), texts: new Map(original.texts) };
    // Decode every original text reference and the full converted contract before
    // keeping a backup or changing the active pointer.
    this.make(audience, converted);
    const archiveAudience = `weftext-personal-input-original:${JSON.stringify([3, audience, original.commitId])}`;
    // The archive coordinate binds the original audience and commit. Keep the
    // metadata bytes directly: wrapping them would double-escape a full bundle.
    const metadata = original.metadata;
    let archived = await this.backend.read(archiveAudience);
    if (!archived) {
      try { await this.backend.compareAndWrite(archiveAudience, 0, metadata, original.texts); }
      catch (error) { if (!(error instanceof PersonalInputStoreConflict)) throw error; }
      archived = await this.backend.read(archiveAudience);
    }
    if (!archived || archived.metadata !== metadata || !sameTexts(archived.texts, original.texts)) throw invalid();
    const commitId = await this.backend.compareAndWrite(audience, original.commitId, converted.metadata, converted.texts);
    return { ...converted, commitId };
  }
  private async load(audience: string): Promise<AudienceState> {
    for (let conflicts = 0; ; conflicts++) {
      const bundle = await this.backend.read(audience);
      if (!bundle) return this.make(audience);
      if (JSON.parse(bundle.metadata)?.version !== 3) return this.make(audience, bundle);
      try { return this.make(audience, await this.migrateVersion3(audience, bundle)); }
      catch (error) { if (!(error instanceof PersonalInputStoreConflict) || conflicts >= 3) throw error; }
    }
  }
  async open(audience: string): Promise<void> {
    let state = this.audiences.get(audience);
    if (!state) { state = this.make(audience); this.audiences.set(audience, state); }
    if (state.loading) return state.loading;
    if (state.loaded) { if (state.readFailure) throw state.readFailure; return; }
    const selected = state;
    selected.loading = this.load(audience).then(loaded => {
      loaded.loaded = true; loaded.loading = selected.loading; this.audiences.set(audience, loaded);
    }).catch(error => { selected.loaded = true; selected.readFailure = error instanceof Error ? error : invalid(); throw selected.readFailure; });
    return selected.loading;
  }
  private state(audience: string): AudienceState {
    const state = this.audiences.get(audience);
    if (!state?.loaded) throw new Error("本人恢复域尚未完成持久读取，输入未借用另一份旧状态。");
    return state;
  }
  list(audience: string): PersonalInputRecord[] { return structuredClone(this.state(audience).records.records.filter((record): record is PersonalInputRecord => record.kind === "text")); }
  captureTransports(audience: string, draftId?: string): CaptureTransportRecord[] { return structuredClone(this.state(audience).records.records.filter((record): record is CaptureTransportRecord => record.kind === "captureTransport" && (!draftId || record.owner.draftId === draftId))); }
  notice(audience: string): string | undefined { const state = this.state(audience); return state.readFailure?.message ?? state.failure?.message; }
  pending(audience?: string): boolean {
    const pending=(state:AudienceState)=>!!state.readFailure || !!state.failure || !!state.loading && !state.loaded || state.version>state.durableVersion || state.records.records.some(record => record.kind === "text" && record.pending?.route === "unknownOriginalRoute" && record.pending.status === "unresolved");
    return audience===undefined?[...this.audiences.values()].some(pending):pending(this.state(audience));
  }
  update(audience: string, updates: PersonalInputUpdate[]): Promise<BrowserProposalAck> {
    const state = this.state(audience);
    for (const update of updates) {
      const index = state.records.records.findIndex(record => record.id === update.id), old = state.records.records[index];
      if (old && (old.kind !== "text" || !equal(old.owner, update.owner))) throw invalid();
      let next: PersonalInputRecord = { ...structuredClone(update), kind: "text", ...(!update.pending && old?.kind === "text" && old.pending ? { pending: old.pending } : {}), sequence: (old?.sequence ?? 0) + 1 };
      if (old?.kind === "text" && old.pending?.route === "unknownOriginalRoute" && old.pending.status === "unresolved" && next.pending?.operationId !== old.pending.operationId) throw invalid();
      if (next.pending?.route === "unknownOriginalRoute" && !(old?.kind === "text" && old.pending?.route === "unknownOriginalRoute" && old.pending.operationId === next.pending.operationId)) throw invalid();
      if (next.pending && state.records.records.some(record => record.kind === "text" && record.pending?.operationId === next.pending!.operationId && !sameRoute(record.pending, next.pending!))) throw invalid();
      if (old?.kind === "text" && old.pending?.operationId === next.pending?.operationId && old.pending && old.pending.status !== "unresolved") next = settledRecord(next, old.pending.status);
      next = mapTexts({ version: 4, records: [next] }, value => { if (typeof value !== "string") throw invalid(); return value; }).records[0] as PersonalInputRecord;
      if (!hasPersonalInput(next.origin) && !next.pending) { if (index >= 0) state.records.records.splice(index, 1); }
      else if (index < 0) state.records.records.push(next); else state.records.records[index] = next;
      state.dirty.add(update.id);
    }
    state.version++; return this.flush(audience);
  }
  stageCaptureCurrent(audience: string, proposal: CaptureTransportCurrent): Promise<BrowserProposalAck> {
    const state = this.state(audience), index = state.records.records.findIndex(record => record.id === proposal.id), old = state.records.records[index];
    if (old && (old.kind !== "captureTransport" || old.owner.draftId !== proposal.owner.draftId || old.producerId !== proposal.producerId || proposal.producerRevision < old.producerRevision)) throw invalid();
    if (old?.kind === "captureTransport" && proposal.producerRevision === old.producerRevision && !equal(old.input, proposal.input)) throw invalid();
    const next = captureTransport({ ...structuredClone(proposal), baseRevision: Math.max(proposal.baseRevision, old?.kind === "captureTransport" ? old.baseRevision : 0), pending: old?.kind === "captureTransport" ? old.pending : null, sequence: (old?.sequence ?? 0) + 1 }, value => { if (typeof value !== "string") throw invalid(); return value; });
    if (index < 0) state.records.records.push(next); else state.records.records[index] = next;
    state.dirty.add(next.id); state.version++; return this.flush(audience);
  }
  freezeCaptureMutation(audience: string, id: string, pending: NonNullable<CaptureTransportRecord["pending"]>): Promise<BrowserProposalAck> {
    const state = this.state(audience), index = state.records.records.findIndex(record => record.id === id), current = state.records.records[index];
    if (current?.kind !== "captureTransport" || current.pending && !equal(current.pending, pending)) throw new Error("另一原 input mutation 尚未核定，未覆盖其冻结 cut。");
    const next = captureTransport({ ...structuredClone(current), pending: structuredClone(pending), sequence: current.sequence + 1 }, value => { if (typeof value !== "string") throw invalid(); return value; });
    state.records.records[index] = next; state.dirty.add(id); state.version++; return this.flush(audience);
  }
  current(audience: string, record: PersonalInputRecord): PersonalInputRecord {
    const current = this.state(audience).records.records.find(entry => entry.id === record.id);
    if (current?.kind !== "text" || current.sequence !== record.sequence) throw new Error("恢复期间输入又发生变化，未采用较早的记录。");
    return structuredClone(current);
  }
  settle(audience: string, record: PersonalInputRecord, status: "committed" | "cancelled" | "resolved"): Promise<BrowserProposalAck> {
    const current = this.current(audience, record); if (!current.pending || current.pending.route !== "known") throw invalid(); return this.update(audience, [settledRecord(current, status)]);
  }
  /** Only an explicit same-audience original request or Core receipt may supply
   * a route missing from v3. Ordinary input updates cannot promote it. */
  proveOriginalRoute(audience: string, record: PersonalInputRecord, proof: PersonalOriginalRouteProof): Promise<BrowserProposalAck> {
    const current = this.current(audience, record), pending = current.pending;
    if (!pending || pending.route !== "unknownOriginalRoute" || current.owner.kind !== "body" || !proof || proof.audience !== audience) throw invalid();
    let resource: string;
    if (proof.kind === "hostedDraft") {
      if (proof.pending?.operationId !== pending.operationId || proof.pending.request?.operationId !== pending.operationId) throw invalid();
      resource = proof.pending.resource;
    } else if (proof.kind === "coreReceipt") {
      if (proof.receipt?.operationId !== pending.operationId || !["committed", "cancelled", "resolved", "recoveryPending", "notCommitted"].includes(proof.receipt.status)) throw invalid();
      resource = proof.receipt.resource;
    } else throw invalid();
    if (typeof resource !== "string" || !resource || resource.length > 4096) throw invalid();
    const state = this.state(audience);
    for (const existing of state.records.records) if (existing.kind === "text" && existing.pending?.operationId === pending.operationId && existing.pending.route === "known" && existing.pending.resource !== resource) throw invalid();
    state.records.records = state.records.records.map(existing => {
      if (existing.kind !== "text" || existing.pending?.operationId !== pending.operationId || existing.pending.route !== "unknownOriginalRoute") return existing;
      const next = { ...existing, sequence: existing.sequence + 1, pending: { ...existing.pending, route: "known" as const, resource } };
      state.dirty.add(next.id); return next;
    });
    state.version++; return this.flush(audience);
  }
  remove(audience: string, record: PersonalInputRecord): Promise<BrowserProposalAck> {
    const current = this.current(audience, record); if (current.pending?.status === "unresolved") throw new Error("原保存操作尚未核定，未移除其原编号。");
    return this.removeRecord(audience, current.id);
  }
  acknowledgeCapture(audience: string, proposal: CaptureTransportRecord, proof: { draftId: string; mutationId: string; fingerprint: string; recordRevision: number }): Promise<BrowserProposalAck> {
    const current = this.state(audience).records.records.find(record => record.id === proposal.id);
    if (current?.kind !== "captureTransport" || !current.pending || !proposal.pending || !equal(current.pending, proposal.pending) || current.owner.draftId !== proof.draftId || current.pending.command.mutationId !== proof.mutationId || current.pending.command.fingerprint !== proof.fingerprint || current.pending.command.expectedRevision + 1 !== proof.recordRevision) throw new Error("Core 回执不对应原 transport cut，未移除未确认本人输入。");
    if (current.producerRevision === current.pending.producerRevision) return this.removeRecord(audience, current.id);
    current.pending = null; current.baseRevision = Math.max(current.baseRevision, proof.recordRevision); current.sequence++;
    const state = this.state(audience); state.dirty.add(current.id); state.version++; return this.flush(audience);
  }
  /** An explicit whole-proposal discard, not an inferred Core acknowledgement.
   * This never removes or settles the draft's separate pending author effect. */
  discardCapture(audience: string, proposal: CaptureTransportRecord): Promise<BrowserProposalAck> {
    const current = this.state(audience).records.records.find(record => record.id === proposal.id);
    if (current?.kind !== "captureTransport" || current.sequence !== proposal.sequence || !equal(current.owner, proposal.owner) || !equal(current.pending, proposal.pending)) throw new Error("待确认输入已变化，未放弃另一份 proposal。");
    return this.removeRecord(audience, current.id);
  }
  private removeRecord(audience: string, id: string): Promise<BrowserProposalAck> {
    const state = this.state(audience); state.records.records = state.records.records.filter(record => record.id !== id); state.dirty.add(id); state.version++; state.failure = undefined; return this.flush(audience);
  }
  flush(audience: string): Promise<BrowserProposalAck> {
    const state = this.state(audience);
    if (state.readFailure) return Promise.reject(state.readFailure);
    if (state.failure) return Promise.reject(state.failure);
    const version = state.version;
    state.running ??= Promise.resolve().then(() => this.pump(audience, state)).catch(error => { state.failure = error instanceof Error ? error : invalid(); throw state.failure; }).finally(() => { state.running = undefined; });
    return state.running.then(ack => state.durableVersion >= version ? ack : this.flush(audience));
  }
  retry(audience: string): Promise<BrowserProposalAck> { const state = this.state(audience); state.failure = undefined; return this.flush(audience); }
  private async pump(audience: string, state: AudienceState): Promise<BrowserProposalAck> {
    let conflicts = 0;
    while (state.version > state.durableVersion) {
      if (state.records.records.length > 128) throw new Error("本人输入恢复记录已达预算；最新输入仍在内存，旧记录未覆盖。");
      const version = state.version, cut = structuredClone(state.records);
      state.cache.write("state", cut);
      const metadata = state.storage.getItem(`${state.prefix}state`); if (metadata === null) throw invalid();
      const textPrefix = `${state.prefix}text:state:`, texts = new Map([...state.storage.values].filter(([key]) => key.startsWith(textPrefix)).map(([key, value]) => [key.slice(textPrefix.length), value]));
      try { state.commitId = await this.backend.compareAndWrite(audience, state.commitId, metadata, texts); }
      catch (error) {
        if (!(error instanceof PersonalInputStoreConflict) || conflicts++ >= 3) throw error;
        const bundle = await this.backend.read(audience); if (!bundle) throw error;
        const remote = this.make(audience, bundle), merged = structuredClone(remote.records);
        for (const id of state.dirty) {
          const before = state.base.records.find(record => record.id === id), latest = remote.records.records.find(record => record.id === id), local = state.records.records.find(record => record.id === id);
          if (!equal(before, latest) && !equal(local, latest)) throw new PersonalInputStoreConflict();
          merged.records = merged.records.filter(record => record.id !== id); if (local) merged.records.push(structuredClone(local));
        }
        const checked = mapTexts(merged, value => { if (typeof value !== "string") throw invalid(); return value; });
        state.records = checked; state.base = remote.base; state.storage = remote.storage; state.cache = remote.cache; state.commitId = remote.commitId;
        continue;
      }
      state.base = cut; state.durableVersion = version;
      for (const id of state.dirty) if (equal(state.records.records.find(record => record.id === id), cut.records.find(record => record.id === id))) state.dirty.delete(id);
    }
    return { kind: "browserProposal", audience, commitId: state.commitId, version: state.durableVersion };
  }
}
export function personalInputText(record: PersonalInputRecord): string { return inputTexts(record.origin).join("\n"); }
function settledRecord<T extends Omit<PersonalInputRecord, "sequence">>(record: T, status: "committed" | "cancelled" | "resolved"): T {
  if (!record.pending) throw invalid();
  const { candidate, ...retained } = record;
  return { ...retained, ...(status === "committed" && candidate ? { baseHash: candidate.baseHash, origin: candidate.origin } : {}), pending: { ...record.pending, status } } as T;
}
