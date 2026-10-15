import { objectKey, type ObjectKey, type ObjectState, type SyncDependency, type SyncIdentity, type SyncRequest, type SyncRevalidation, type SyncRevalidationCut, type SyncRevalidationInput, type SyncRevalidationReview } from "./sync-state";

const sameSet = (a: string[], b: string[]): boolean => a.length === new Set(a).size && b.length === new Set(b).size && a.length === b.length && [...a].sort().every((v, i) => v === [...b].sort()[i]);
const sameMap = (a: Record<string, string>, b: Record<string, string>): boolean => sameSet(Object.keys(a), Object.keys(b)) && Object.keys(a).every(k => a[k] === b[k]);
function sameState(a: ObjectState, b: ObjectState): boolean {
  return (a.parent ? objectKey(a.parent) : null) === (b.parent ? objectKey(b.parent) : null) && a.name === b.name && a.retired === b.retired && sameMap(a.files, b.files) && sameMap(a.fileRevisions, b.fileRevisions);
}
function baseRevision(r: SyncRevalidation): string { return r.decision.kind === "preserve" ? r.decision.revision : r.decision.baseRevision; }
export interface SyncExactText { object: ObjectKey; revision: string; fileRevision: string; path: string; digest: string; bytes: number; text: string | null }
export async function syncTextDigest(bytes: Uint8Array): Promise<string> { return Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", new Uint8Array(bytes)))).map(v => v.toString(16).padStart(2, "0")).join(""); }

/** An ephemeral review of immutable received inputs. Only SyncFlow persists or
 * submits the eventual request; this class never writes the current Draft. */
export class SyncRevalidationSession {
  private choices = new Map<string, SyncRevalidation>();
  private supplied = new Map<string, number[]>();
  private generation = 0;
  private retired = false;
  private accepted: SyncRevalidationReview | null = null;
  readonly cut: SyncRevalidationCut;
  readonly identity: SyncIdentity;
  constructor(readonly root: string, identity: SyncIdentity, cut: SyncRevalidationCut) { this.identity = structuredClone(identity); this.cut = structuredClone(cut); }
  get review(): SyncRevalidationReview | null { return this.accepted ? structuredClone(this.accepted) : null; }
  get selections(): SyncRevalidation[] { return structuredClone([...this.choices.values()]); }
  selection(object: ObjectKey): SyncRevalidation | undefined { const r = this.choices.get(objectKey(object)); return r ? structuredClone(r) : undefined; }
  repairedText(object: ObjectKey, path: string): string | undefined { const r = this.choice(object), bytes = this.supplied.get(r.state.files[path]); return bytes ? new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(new Uint8Array(bytes)) : undefined; }
  private changed(): void { this.generation++; this.accepted = null; }
  invalidate(): void { this.retired = true; this.changed(); }
  touch(): void { this.changed(); }
  private current(): void { if (this.retired) throw new Error("收到的版本或资料库已变化，请重新读取语义审阅。"); }
  select(object: ObjectKey, revision: string): void {
    this.current();
    const entry = this.cut.objects.find(v => objectKey(v.object) === objectKey(object)), version = entry?.view.versions.find(v => v.revision === revision);
    if (!entry || !version) throw new Error("请选择已收到的准确当前分支。");
    this.choices.set(objectKey(object), { object: structuredClone(object), revisions: entry.view.versions.map(v => v.revision), decision: { kind: "preserve", revision }, state: structuredClone(version.state), reviewedDependencies: [] });
    this.changed();
  }
  unselect(object: ObjectKey): void { this.current(); this.choices.delete(objectKey(object)); this.changed(); }
  private choice(object: ObjectKey): SyncRevalidation { this.current(); const r = this.choices.get(objectKey(object)); if (!r) throw new Error("请先明确选择此对象的一个原分支。"); return r; }
  async verifyText(object: ObjectKey, file: SyncExactText): Promise<string> {
    const r = this.choice(object), revision = baseRevision(r), original = this.cut.objects.find(v => objectKey(v.object) === objectKey(object))!.view.versions.find(v => v.revision === revision)!.state;
    const generation = this.generation;
    if (!(file.path in original.files) || objectKey(file.object) !== objectKey(object) || file.revision !== revision || file.digest !== original.files[file.path] || file.fileRevision !== original.fileRevisions[file.path] || file.text === null) throw new Error("准确版本文件无法作为文本修复依据；原文件保留。");
    const bytes = new TextEncoder().encode(file.text);
    if (bytes.length !== file.bytes || await syncTextDigest(bytes) !== file.digest) throw new Error("读取内容与原文件摘要不一致。");
    this.current(); if (generation !== this.generation) throw new Error("文件读取期间已更换选择，请重新读取。");
    return file.text;
  }
  async repairText(object: ObjectKey, path: string, text: string): Promise<void> {
    const r = this.choice(object), revision = baseRevision(r), original = this.cut.objects.find(v => objectKey(v.object) === objectKey(object))!.view.versions.find(v => v.revision === revision)!.state;
    if (!(path in original.files)) throw new Error("修复必须基于所选分支中已有的准确文件。");
    // Invalidate before hashing, so an earlier review cannot be confirmed while
    // a new repair is awaiting its digest, including equal-byte ABA edits.
    this.changed(); const generation = this.generation, bytes = new TextEncoder().encode(text), digest = await syncTextDigest(bytes);
    this.current(); if (generation !== this.generation) throw new Error("修复输入已变化，请重新确认当前内容。");
    r.state.files[path] = digest;
    if (digest === original.files[path]) r.state.fileRevisions[path] = original.fileRevisions[path];
    else { r.state.fileRevisions[path] = crypto.randomUUID(); this.supplied.set(digest, Array.from(bytes)); }
    r.decision = sameState(r.state, original) ? { kind: "preserve", revision } : { kind: "repair", baseRevision: revision };
  }
  input(): SyncRevalidationInput {
    this.current(); if (!this.choices.size) throw new Error("请明确选择至少一个要重新验证的对象。");
    const used = new Set([...this.choices.values()].flatMap(r => Object.values(r.state.files))), blobs: Record<string, number[]> = {};
    for (const [digest, bytes] of this.supplied) if (used.has(digest)) blobs[digest] = [...bytes];
    return { root: this.root, identity: structuredClone(this.identity), resolutions: this.selections, blobs };
  }
  async inspect(read: (input: SyncRevalidationInput) => Promise<SyncRevalidationReview>): Promise<boolean> {
    this.changed(); const generation = this.generation, input = this.input();
    const result = await read(structuredClone(input));
    if (this.retired || generation !== this.generation) return false;
    if (result.resolutions.length !== input.resolutions.length) throw new Error("审阅返回了不同的明确选择。");
    const seen = new Set<string>();
    for (const r of result.resolutions) {
      const key = objectKey(r.object), original = input.resolutions.find(v => objectKey(v.object) === key);
      if (seen.has(key) || !original || !sameSet(original.revisions, r.revisions) || !sameState(original.state, r.state) || original.decision.kind !== r.decision.kind || baseRevision(original) !== baseRevision(r)) throw new Error("审阅改变了原分支或修复输入；未准备操作。");
      seen.add(key);
      this.checkCut(r.reviewedDependencies);
    }
    this.checkCut(result.reviewedVersions); this.checkCut(result.expectedInstalled, false);
    for (const r of result.resolutions) if (!result.reviewedVersions.some(v => objectKey(v.object) === objectKey(r.object) && sameSet(v.revisions, r.revisions))) throw new Error("审阅缺少完整的收到版本集合。");
    this.accepted = structuredClone(result); return true;
  }
  private checkCut(cut: SyncDependency[], received = true): void {
    const seen = new Set<string>();
    for (const dep of cut) {
      const key = objectKey(dep.object);
      if (seen.has(key) || dep.revisions.length !== new Set(dep.revisions).size || (received && !this.cut.reviewedVersions.some(v => objectKey(v.object) === key && sameSet(v.revisions, dep.revisions)))) throw new Error("审阅依据已变化或包含重复对象，请重新读取。");
      seen.add(key);
    }
  }
  request(operationId: string): SyncRequest {
    this.current(); if (!this.accepted) throw new Error("请先读取并核对当前选择的实际语义依赖。");
    const input = this.input(), review = this.accepted;
    return structuredClone({ operationId, root: this.root, intent: { kind: "revalidateSync", identity: this.identity, expectedInstalled: review.expectedInstalled, reviewedVersions: review.reviewedVersions, resolutions: review.resolutions, blobs: input.blobs } });
  }
}
