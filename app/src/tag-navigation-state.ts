/** Recovery inspects the original operation; it never substitutes a fresh plan. */
export function tagOperationStatus(expected: { operationId: string; planId: string }, incoming: { operationId?: string; planId?: string; status?: string; receipt?: { status?: string } }): "prepared" | "committed" {
  if (incoming.operationId !== expected.operationId || incoming.planId !== expected.planId) throw new Error("导航恢复结果与原操作／计划不符，原记录保留。");
  const status = incoming.receipt?.status ?? incoming.status;
  if (status !== "prepared" && status !== "committed") throw new Error(`原导航操作尚不能确认：${status ?? "未知"}`);
  return status;
}
export function tagCancellationStatus(expected: { operationId: string; planId: string | null }, incoming: { operationId: string; planId: string | null; status: string }): "resolved" | "committed" {
  if (incoming.operationId !== expected.operationId || expected.planId !== null && incoming.planId !== expected.planId) throw new Error("取消回执不属于原导航操作。");
  if (incoming.planId !== null && (typeof incoming.planId !== "string" || !incoming.planId)) throw new Error("取消回执缺少准确计划状态，原操作保留。");
  if (incoming.status === "cancelled" || incoming.status === "resolved") return "resolved";
  if (incoming.status === "committed" && typeof incoming.planId === "string") return "committed";
  throw new Error(`原取消尚未确认：${incoming.status}`);
}
