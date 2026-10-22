import type { CaptureTransportRecord } from "./personal-input-recovery";

/** Explicit discard of a private input proposal. Its pending author creation
 * is a different authority and is never cancelled or settled here. */
export async function fenceCaptureProposal(proposal: CaptureTransportRecord, port: {
  query(): Promise<{ status: string; draft: { revision: number } } | undefined>;
  read(): Promise<{ revision: number }>;
  currentRevision(): number;
  checkpoint(): Promise<unknown>;
}): Promise<void> {
  const lookup = await port.query();
  const current = lookup?.draft ?? await port.read();
  const expected = proposal.pending?.command.expectedRevision ?? proposal.baseRevision;
  if (current.revision < expected) throw new Error("原私有草稿修订倒退，未放弃或重放输入。");
  if (current.revision > expected) return; // Every old expectedRev is already stale.
  if (port.currentRevision() !== current.revision) throw new Error("原私有草稿已变化，未以较早检查点处置输入。");
  try { await port.checkpoint(); }
  catch (error) {
    const original = await port.query();
    throw new Error(`私有输入检查点尚未确认；原 mutation 状态 ${original?.status ?? "unknown"}。原输入与原号继续保留。`, { cause: error });
  }
}
