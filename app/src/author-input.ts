import type { PersonalCopySeed } from "./personal-input-recovery";

export interface AuthorInputBindOptions { readonly personalCopy?: PersonalCopySeed }

/** Native ValuePath identities, shared with field and capture owners. */
export type ValuePath = { kind: "member"; key: string } | { kind: "index"; index: number };
export type AuthorInputOwner =
  | { kind: "body"; resource: string; reference?: string }
  | { kind: "field"; resource: string; fieldId: string }
  | { kind: "review"; resource: string; annotationId?: string; asset?: string; revision?: string }
  | { kind: "library"; role: string }
  | { kind: "registry"; target: string }
  | { kind: "capture"; draftId: string; library?: { workspaceId: string; genesis: string }; approvedSlot?: string };
export interface AuthorInputContext { audience: string; epoch: number; owner: AuthorInputOwner; slot: string; path: readonly ValuePath[] }
export interface AuthorInputSnapshot {
  kind: "text" | "choice";
  context: AuthorInputContext; id: string; revision: number;
  /** Proved new text only: the personal recovery store may retain this. */
  text: string;
  /** Explicit owner sinks (e.g. Core CaptureDraft) may consume their raw slot.
   * This must never be copied into the generic personal-input store. */
  rawValue: string;
  producerId: string;
  producerRevision: number;
}
export type AuthorInputSink = (input: AuthorInputSnapshot) => void;
export interface AuthorProducerCheckpoint { producerId: string; revision: number }
export interface AuthorProducerState extends AuthorProducerCheckpoint {
  audience: string; epoch: number; owner: AuthorInputOwner; ackRevision: number;
  retired: boolean; nativePending: boolean; hasObservedChanges: boolean;
  unapplied: boolean; pendingDeliveries: number; frozen: boolean;
}
export interface AuthorProducerSnapshot { collectorRevision: number; producers: AuthorProducerState[] }
export interface AuthorInputLease { apply(text: string, assign?: (text: string) => void): boolean; release(): void }
export interface AuthorInputSession {
  bind(control: HTMLInputElement | HTMLTextAreaElement, slot: string, path?: readonly ValuePath[], options?: AuthorInputBindOptions): () => void;
  /** Positive choice registration tracks unapplied intent without treating an
   * untouched protected option as personally typed text. */
  bindChoice(control: HTMLInputElement | HTMLSelectElement, slot: string, path?: readonly ValuePath[]): () => void;
  /** Record an explicit non-text edit after its guards, before publishing its typed
   * value. Throws if this exact producer is no longer writable; never a save ACK. */
  recordAction(slot: string, path?: readonly ValuePath[]): void;
  /** Structure/branch/reorder changes retire previous slot identities. */
  invalidate(slot: string, path?: readonly ValuePath[]): void;
  capture(control: HTMLInputElement | HTMLTextAreaElement): AuthorInputLease | undefined;
  inspect(): AuthorProducerState;
  /** Actual application/explicit owner discard only; never a browser storage ACK. */
  acknowledge(checkpoint: AuthorProducerCheckpoint, disposition: "applied" | "discarded"): boolean;
  /** Producer stop only; this is never a Core durability or close proof. */
  freeze(checkpoint: AuthorProducerCheckpoint): boolean;
  thaw(): void;
  retire(): void;
}
export type AuthorInputFactory = (owner: AuthorInputOwner, current: () => boolean, sink?: AuthorInputSink, leaseRevision?: () => unknown) => AuthorInputSession | undefined;

/** Re-rendering the same proposal keeps its slot IDs; a new proposal gets a new lease. */
export class AuthorInputScopes {
  private entries = new WeakMap<object, { owner: string; session: AuthorInputSession }>();
  private sessions = new Set<AuthorInputSession>();
  constructor(private factory?: AuthorInputFactory) {}
  get(proposal: object, owner: AuthorInputOwner, current: () => boolean, leaseRevision?: () => unknown): AuthorInputSession | undefined {
    const key = JSON.stringify(owner), old = this.entries.get(proposal);
    if (old?.owner === key) return old.session;
    if (old) { old.session.retire(); this.sessions.delete(old.session); }
    const session = this.factory?.(owner, current, undefined, leaseRevision);
    if (session) { this.entries.set(proposal, { owner: key, session }); this.sessions.add(session); }
    return session;
  }
  clear(): void { for (const session of this.sessions) session.retire(); this.sessions.clear(); this.entries = new WeakMap(); }
}

const controls = new WeakMap<HTMLInputElement | HTMLTextAreaElement, () => AuthorInputLease | undefined>();
export function registerAuthorInputLease(control: HTMLInputElement | HTMLTextAreaElement, capture: () => AuthorInputLease | undefined): () => void { controls.set(control, capture); return () => { if (controls.get(control) === capture) controls.delete(control); }; }
/** Capture before clipboard/file/network awaits, never after the payload returns. */
export function captureAuthorInput(control: HTMLInputElement | HTMLTextAreaElement): AuthorInputLease | undefined { return controls.get(control)?.(); }
