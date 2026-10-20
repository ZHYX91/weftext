import test from "node:test";
import assert from "node:assert/strict";
import { bibliographyConflictCount, ExchangeController, OutputController, OutputReadGate, type BibliographyInspection, type ExchangeRequest, type ExportRequest, type OutputTransport, type PreparedOutput } from "./export-state";
import type { DocumentBasis } from "./data-panel";

const basis = (): DocumentBasis => ({ sessionId: "session-a", source: "原生草稿", serial: 7, baseId: "base-a", path: "D:/Library/Note/note.typ", root: "D:/Library" });
const hash = "a".repeat(64);
const prepared = (request: ExportRequest, overrides: Partial<PreparedOutput> = {}): PreparedOutput => ({ id: "artifact-a", requestId: request.requestId, filename: "输出.docx", contentType: "application/vnd.openxmlformats-officedocument.wordprocessingml.document", length: 3, digest: hash, summary: "真实文件", losses: [], status: "prepared", ...overrides });
function transport(overrides: Partial<OutputTransport> = {}): OutputTransport {
  return { id: () => "request-a", prepare: async request => prepared(request), confirm: async request => ({ id: request.id, status: "confirmed", digest: hash, length: 3 }), download: async () => new Blob(["abc"]), digest: async () => hash, ...overrides };
}

test("legacy output freezes its DOCX review inputs and cancellation signals the original request without unlocking it", async () => {
  let resolve!: (value: PreparedOutput) => void; let request!: ExportRequest; const cancelled: string[] = [];
  const controller = new OutputController(transport({ prepare: input => { request = input; return new Promise(done => resolve = done); }, cancel: async input => { cancelled.push(input.requestId); } }), basis(), undefined, [{ occurrenceId: "diagram-neighbor", queryId: "query-frozen" }]);
  controller.setDraft({ format: "doc", legacyBackend: "libreoffice", reviewMode: "changes", includeResolved: true }); const pending = controller.prepare();
  assert.equal(controller.busy, true); await controller.cancel(); assert.deepEqual(cancelled, ["request-a"]); assert.equal(controller.busy, true); assert.equal(controller.locked, true);
  assert.equal(request.format, "doc"); assert.equal(request.legacyBackend, "libreoffice"); assert.equal(request.reviewMode, "changes"); assert.equal(request.includeResolved, true); assert.deepEqual(request.embeddedQueries, [{ occurrenceId: "diagram-neighbor", queryId: "query-frozen" }]);
  resolve(prepared(request)); await pending;
});
test("an unknown persistent external outcome keeps the same immutable request instead of launching a new conversion", async () => {
  const requests: ExportRequest[] = []; const controller = new OutputController(transport({ prepare: async input => { requests.push(input); return { id: input.requestId, requestId: input.requestId, status: "failed", code: "resultUnknown", message: "原转换结果未知" }; } }), basis());
  controller.setDraft({ format: "wps", legacyBackend: "configured" }); await assert.rejects(controller.prepare(), /未知/); assert.equal(controller.uncertain, "prepare");
  await assert.rejects(controller.prepare()); assert.deepEqual(requests[0], requests[1]); assert.throws(() => controller.setDraft({ legacyBackend: "libreoffice" }));
});

test("output freezes the original draft and server-retained Query; no later document or Query changes enter preparation", async () => {
  const current = basis(); const query = { queryId: "query-a", label: "部分结果", rowCount: 2, coverage: "partial" as const, freshness: "current" as const, limited: true }; let request: ExportRequest | undefined;
  const controller = new OutputController(transport({ prepare: async value => { request = value; return prepared(value); } }), current, query);
  current.source = "等待期间的新输入"; current.serial = 8; query.queryId = "query-b"; query.rowCount = 99;
  await controller.prepare();
  assert.deepEqual(request, { requestId: "request-a", sessionId: "session-a", source: "原生草稿", serial: 7, baseId: "base-a", format: "xlsx", stylePolicy: "template", queryId: "query-a" });
  assert.equal(controller.draft.query!.rowCount, 2); assert.equal(controller.draft.query!.coverage, "partial");
});

test("Office defaults to template style and exact uploaded template/bindings are cloned into the frozen request", async () => {
  const controller = new OutputController(transport(), basis());
  const template = { name: "模板.docx", bytesBase64: "eA==" }; const bindings = { 正文: { kind: "body" as const }, 日期: { kind: "field" as const, field: "due" } };
  controller.setDraft({ template, bindings }); template.bytesBase64 = "changed"; bindings.日期.field = "changed";
  let request: ExportRequest | undefined; controller.host.prepare = async value => { request = value; return prepared(value); };
  await controller.prepare(); assert.equal(request!.template!.bytesBase64, "eA=="); assert.equal(request!.bindings!.日期.field, "due"); assert.equal(request!.stylePolicy, "template");
});

test("no-template XLSX uses current note projection without inventing a Query result", async () => {
  let request: ExportRequest | undefined; const controller = new OutputController(transport({ prepare: async input => { request = input; return prepared(input); } }), basis());
  controller.setDraft({ format: "xlsx" }); await controller.prepare(); assert.equal(request!.format, "xlsx"); assert.equal(request!.queryId, undefined); assert.equal(request!.template, undefined);
});

test("a lost preparation reuses its request ID and immutable inputs even if the transport mutates its argument", async () => {
  const requests: ExportRequest[] = []; let calls = 0;
  const controller = new OutputController(transport({ prepare: async request => { requests.push(structuredClone(request)); if (++calls === 1) { request.source = "bad mutation"; throw new Error("connection lost"); } return prepared(request); } }), basis());
  await assert.rejects(controller.prepare(), /connection lost/); assert.equal(controller.uncertain, "prepare");
  assert.throws(() => controller.setDraft({ format: "native" }), /同一操作/);
  await controller.prepare(); assert.deepEqual(requests[0], requests[1]); assert.equal(controller.uncertain, null);
});

test("a reply bound to a different request never creates a confirmable artifact", async () => {
  const controller = new OutputController(transport({ prepare: async request => prepared(request, { requestId: "other" }) }), basis());
  await assert.rejects(controller.prepare(), /不一致/); assert.equal(controller.prepared, null); assert.equal(controller.uncertain, "prepare"); await assert.rejects(controller.confirm(true), /预览/);
});

test("definite preparation refusal retains template and draft while allowing a corrected explicit request", async () => {
  const controller = new OutputController(transport({ prepare: async () => { throw Object.assign(new Error("invalid template"), { code: "templateInvalid" }); } }), basis());
  controller.setDraft({ template: { name: "原件.docx", bytesBase64: "eA==" }, bindings: { 标题: { kind: "title" } } });
  await assert.rejects(controller.prepare()); assert.equal(controller.uncertain, null); assert.equal(controller.draft.template!.name, "原件.docx");
  controller.setDraft({ stylePolicy: "source" }); assert.equal(controller.draft.bindings.标题.kind, "title");
});

test("loss acceptance is required, and an unknown confirmation repeats exactly the original artifact and decision", async () => {
  const requests: { id: string; acceptLosses: boolean }[] = []; let calls = 0;
  const controller = new OutputController(transport({ prepare: async request => prepared(request, { losses: [{ code: "loss", message: "转换限制", location: "段落 1" }] }), confirm: async request => { requests.push(request); if (++calls === 1) throw Object.assign(new Error("unknown"), { code: "resultUnknown" }); return { id: request.id, status: "confirmed" }; } }), basis());
  await controller.prepare(); await assert.rejects(controller.confirm(false), /接受/); assert.equal(requests.length, 0);
  await assert.rejects(controller.confirm(true)); assert.equal(controller.confirmed, false); await assert.rejects(controller.prepare(), /原输出/);
  await controller.confirm(false); assert.deepEqual(requests, [{ id: "artifact-a", acceptLosses: true }, { id: "artifact-a", acceptLosses: true }]);
});

test("mismatched confirmation and wrong byte lengths/digests cannot be advertised as confirmed or received", async () => {
  const controller = new OutputController(transport({ confirm: async () => ({ id: "different", status: "confirmed" }) }), basis());
  await controller.prepare(); await assert.rejects(controller.confirm(true), /不一致/); assert.equal(controller.confirmed, false); await assert.rejects(controller.download(), /确认/);
  for (const overrides of [{ download: async () => new Blob(["abcd"]) }, { digest: async () => "b".repeat(64) }]) {
    const wrong = new OutputController(transport(overrides), basis()); await wrong.prepare(); await wrong.confirm(false); await assert.rejects(wrong.download(), /字节/); assert.equal(wrong.received, false); assert.equal(wrong.uncertain, "download");
  }
});

test("download retries one confirmed artifact without compiling or confirming again", async () => {
  const ids: string[] = []; let calls = 0; let prepares = 0;
  const controller = new OutputController(transport({ prepare: async request => { prepares++; return prepared(request); }, download: async request => { ids.push(request.id); if (++calls === 1) throw new Error("connection lost"); return new Blob(["abc"]); } }), basis());
  await controller.prepare(); await controller.confirm(false); await assert.rejects(controller.download()); assert.equal(controller.received, false);
  await controller.download(); assert.equal(controller.received, true); assert.equal(prepares, 1); assert.deepEqual(ids, ["artifact-a", "artifact-a"]);
});

test("a late template response cannot replace a newer file or format selection", () => {
  const gate = new OutputReadGate(); const first = gate.begin(); const second = gate.begin();
  assert.equal(gate.accepts(first), false); assert.equal(gate.accepts(second), true); gate.invalidate(); assert.equal(gate.accepts(second), false);
});

const importInput = () => ({ sessionId: "s", baseId: "b", root: "D:/Library", parent: "D:/Library", name: "导入副本", bytesBase64: "eA==", format: "native" as const });
test("unknown import preparation freezes its original and destination under one operation ID", async () => {
  const requests: ExchangeRequest[] = []; let calls = 0;
  const controller = new ExchangeController({ id: () => "operation-a", prepare: async request => { requests.push(structuredClone(request)); if (++calls === 1) { request.bytesBase64 = "mutated"; throw new Error("connection lost"); } return { operationId: request.operationId, planId: "plan-a", losses: [], changes: [] }; }, commit: async () => ({ status: "committed" }) });
  const input = importInput(); controller.setInput(input); input.name = "new"; await assert.rejects(controller.prepare()); assert.throws(() => controller.setInput(importInput()), /重复创建/); await controller.prepare(); assert.deepEqual(requests[0], requests[1]); assert.equal(requests[0].name, "导入副本");
});

test("uncertain native commit repeats one plan, and a received receipt prevents duplicate creation when opening failed", async () => {
  const requests: { operationId: string; planId: string; acceptLosses: boolean }[] = []; let calls = 0;
  const controller = new ExchangeController({ id: () => "operation-a", prepare: async request => ({ operationId: request.operationId, planId: "plan-a", losses: [] }), commit: async request => { requests.push(request); if (++calls === 1) throw Object.assign(new Error("unknown"), { code: "resultUnknown" }); return { status: "committed", operationId: request.operationId, path: "D:/Library/Imported/note.typ" }; } });
  controller.setInput(importInput()); await controller.prepare(); await assert.rejects(controller.commit(false)); await assert.rejects(controller.prepare(), /原导入/); const receipt = await controller.commit(false); assert.equal(await controller.commit(false), receipt); assert.deepEqual(requests, [{ operationId: "operation-a", planId: "plan-a", acceptLosses: false }, { operationId: "operation-a", planId: "plan-a", acceptLosses: false }]);
});

test("native loss acceptance and actual committed receipts gate the visible success state", async () => {
  let commits = 0; const controller = new ExchangeController({ id: () => "operation-a", prepare: async request => ({ operationId: request.operationId, planId: "plan-a", losses: ["本机资格不转移"] }), commit: async () => { commits++; return { status: "pending" }; } });
  controller.setInput(importInput()); await controller.prepare(); await assert.rejects(controller.commit(false), /接受/); assert.equal(commits, 0); await assert.rejects(controller.commit(true), /实际完成/); assert.equal(controller.receipt, null); assert.equal(controller.uncertain, "commit");
});

test("HTTP failure JSON is a definite terminal output failure; pending JSON retains the same request", async () => {
  const seen: ExportRequest[] = []; let calls = 0; const controller = new OutputController(transport({ prepare: async request => { seen.push(request); return ++calls === 1 ? { requestId: request.requestId, status: "pending" } : { requestId: request.requestId, status: "failed", code: "conversionUnavailable", message: "模板中缺少明确正文槽位" }; } }), basis());
  await assert.rejects(controller.prepare(), /仍在处理/); assert.equal(controller.uncertain, "prepare"); await assert.rejects(controller.prepare(), /缺少明确正文/); assert.equal(controller.uncertain, null); assert.deepEqual(seen[0], seen[1]); controller.setDraft({ format: "source" }); assert.equal(controller.draft.format, "source");
});

test("unknown lossy import commit freezes the accepted loss decision through retry", async () => {
  const decisions: boolean[] = []; let calls = 0; const controller = new ExchangeController({ id: () => "operation-a", prepare: async request => ({ operationId: request.operationId, planId: "plan-a", losses: [{ code: "native", detail: "本机资格不转移", path: "manifest" }] }), commit: async request => { decisions.push(request.acceptLosses); if (++calls === 1) throw new Error("connection lost"); return { status: "committed", path: "D:/Library/导入副本" }; } });
  controller.setInput(importInput()); await controller.prepare(); await assert.rejects(controller.commit(true)); await controller.commit(false); assert.deepEqual(decisions, [true, true]);
});

test("bibliography conflict inspection counts all explicit conflicts and never treats missing conflict data as clear", () => {
  assert.equal(bibliographyConflictCount({ records: [{ conflicts: [] }, { conflicts: [{ key: "title" }, { key: "date" }] }], losses: [] }), 2);
  assert.equal(bibliographyConflictCount({ records: [{ conflicts: [] }], losses: [] }), 0);
  assert.throws(() => bibliographyConflictCount({ records: [{}], losses: [] } as unknown as BibliographyInspection), /完整返回冲突/);
});


test("body Query output freezes each selection and never carries it into an unrelated format", async () => {
  const selected = [{ occurrenceId: "body-first", queryId: "retained-a" }];
  const calls: ExportRequest[] = [];
  const controller = new OutputController(transport({ prepare: async request => { calls.push(structuredClone(request)); return prepared(request); } }), basis(), undefined, selected);
  selected[0].queryId = "later-b";
  await controller.prepare();
  assert.deepEqual(calls[0].embeddedQueries, [{ occurrenceId: "body-first", queryId: "retained-a" }]);
  controller.setDraft({ format: "pdf" }); await controller.prepare();
  assert.deepEqual(calls[1].embeddedQueries, calls[0].embeddedQueries);
  controller.setDraft({ format: "xlsx" }); await controller.prepare();
  assert.equal(calls[2].embeddedQueries, undefined);
});
