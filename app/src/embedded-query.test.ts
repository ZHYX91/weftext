import test from "node:test";
import assert from "node:assert/strict";
import { history, isolateHistory, redo, undo, undoDepth } from "@codemirror/commands";
import { createSourceState } from "./editor";
import { associateEmbeddedQueries, EmbeddedQueryController, EmbeddedQueryRunGate, embeddedQueryPolicy, exactQueryOccurrence, sameQueryBasis, validateEmbeddedEdit, type EmbeddedQueryBasis, type EmbeddedQueryOccurrence, type EmbeddedQueryRun } from "./embedded-query";
import { utf8Offset } from "./writing";
import type { RenderNode, ViewMode } from "./types";
import { message } from "./ui-locale";
import { locale, uiTextValue } from "./ui-locale";
import { decodeEmbeddedQueryDiscovery, embeddedControlDiagnostics } from "./embedded-query";

const source = "中文 😀\r\n#metadata((kind: \"weftext.query\", /* keep */ where: \"true\")) <weftext-query-first>\r\n外部字节\r\n#metadata((kind: \"weftext.query\", where: \"false\")) <weftext-query-second>\r\n";
function occurrence(label = "first"): EmbeddedQueryOccurrence {
  const from = source.indexOf("#metadata", label === "first" ? 0 : source.indexOf("外部字节")), to = source.indexOf(` <weftext-query-${label}>`, from) + ` <weftext-query-${label}>`.length;
  return { id: label, key: label, label: `weftext-query-${label}`, kind: "definition", callFrom: utf8Offset(source, from + 1), range: { from: utf8Offset(source, from), to: utf8Offset(source, to), utf16From: from, utf16To: to, raw: source.slice(from, to), writable: true }, writable: true, builderSupported: true, runnable: true, plan: null, binding: null, call: null, reason: null, controlKind: null, controlReason: null, controlDiagnostics: [] };
}
const basis: EmbeddedQueryBasis = { sessionId: "session", source, serial: 7 };
test("embedded discovery preserves complete typed diagnostics and independent machine classifications", () => {
  const description = { explanation: message("common.unavailable"), diagnostics: [{ origin: "dependency", text: "original\u0000 العربية" }], causes: [] };
  const rawOccurrence = { ...occurrence(), writable: false, builderSupported: false, runnable: false, controlKind: "invalid", controlReason: description, controlDiagnostics: Array.from({ length: 96 }, (_, index) => ({ ...description, diagnostics: [{ origin: "json", text: `parallel-${index}` }] })) };
  const raw = { serial: basis.serial, source, sourceHash: "original-hash", syntaxValid: true, occurrences: [rawOccurrence] }, frozen = JSON.stringify(raw);
  const invalid = decodeEmbeddedQueryDiscovery(JSON.parse(frozen)), budget = decodeEmbeddedQueryDiscovery({ ...raw, occurrences: [{ ...rawOccurrence, controlKind: "nativeBudget" }] });
  assert.equal(invalid.occurrences[0].controlKind, "invalid"); assert.equal(budget.occurrences[0].controlKind, "nativeBudget", "same message never selects a machine classification");
  assert.equal(invalid.occurrences[0].range.raw, occurrence().range.raw); assert.equal(invalid.occurrences[0].controlDiagnostics.length, 96);
  const notices = embeddedControlDiagnostics(invalid.occurrences[0]);
  try { locale.set("en-US"); assert.ok(uiTextValue(notices[0]).includes("Unavailable")); assert.ok(uiTextValue(notices[0]).includes("original\u0000 العربية")); assert.ok(uiTextValue(notices[96]).includes("parallel-95")); locale.set("zh-CN"); assert.ok(uiTextValue(notices[0]).includes("原始")); assert.ok(uiTextValue(notices[96]).includes("parallel-95")); } finally { locale.set("zh-CN"); }
  assert.equal(JSON.stringify(raw), frozen);
  for (const occurrence of [{ ...rawOccurrence, controlKind: undefined }, { ...rawOccurrence, controlReason: "legacy" }, { ...rawOccurrence, controlDiagnostics: undefined }, { ...rawOccurrence, controlKind: "unknown" }, { ...rawOccurrence, reason: "legacy source reason" }, { ...rawOccurrence, controlKind: null }]) assert.throws(() => decodeEmbeddedQueryDiscovery({ ...raw, occurrences: [occurrence] }));
});
test("embedded authorization projection retains only its delivered safe explanation and eligibility", () => {
  const safe = { ...occurrence(), writable: false, builderSupported: false, runnable: false, controlKind: "unavailable", controlReason: { explanation: message("error.api.hostedEmbeddedQuery.definitionReadRequired"), diagnostics: [], causes: [] }, controlDiagnostics: [] };
  const parsed = decodeEmbeddedQueryDiscovery({ serial: basis.serial, source, sourceHash: "original", syntaxValid: true, occurrences: [safe] }).occurrences[0];
  assert.equal(parsed.controlKind, "unavailable"); assert.equal(parsed.runnable, false); assert.deepEqual(parsed.controlDiagnostics, []); assert.equal(parsed.range.raw, safe.range.raw);
});
function node(key: string, item: EmbeddedQueryOccurrence, main = true): RenderNode { return { key, kind: "tag", tag: null, text: null, attrs: { "data-weftext-query-label": item.label, "data-weftext-query-occurrence": key }, children: [], origin: { path: main ? "note.typ" : "included.typ", from: item.callFrom, to: item.range.to, utf16From: item.range.utf16From + 1, utf16To: item.range.utf16To, raw: item.range.raw.slice(1), isMain: main, syntaxKind: "FuncCall" }, writable: false }; }
function tree(children: RenderNode[]): RenderNode { return { key: "root", kind: "element", tag: "div", text: null, attrs: {}, children, origin: null, writable: false }; }
const result: EmbeddedQueryRun = { serial: basis.serial, source: basis.source, sourceHash: "hash", occurrenceId: "first", draftPreview: true, result: { rows: [], coverage: "partial", freshness: "stale", diagnostics: [{ explanation:message("common.unavailable"),diagnostics:[],causes:[] }], totalKnown: 0, limited: false } };

test("query author range validates bytes, UTF16, CRLF and supplementary characters", () => {
  const item = occurrence(); assert.equal(exactQueryOccurrence(source, item), true);
  assert.equal(exactQueryOccurrence(source, { ...item, range: { ...item.range, from: item.range.utf16From } }), false);
  assert.equal(exactQueryOccurrence(source, { ...item, range: { ...item.range, raw: item.range.raw.replace("keep", "lost") } }), false);
  assert.equal(exactQueryOccurrence(source, { ...item, range: { ...item.range, utf16From: 4 } }), false);
});

test("consumed native tags preserve actual body order and separate multiple exact query origins", () => {
  const first = occurrence(), second = occurrence("second");
  const association = associateEmbeddedQueries(tree([node("second-position", second), node("first-position", first)]), [first, second]);
  assert.deepEqual([...association.keys()], ["second-position", "first-position"]);
  assert.equal(association.get("second-position")?.occurrence?.id, "second");
  assert.equal(association.get("first-position")?.occurrence?.id, "first");
  assert.equal(association.get("first-position")?.unique, true);
  const wrongOffset = node("wrong", first); wrongOffset.origin!.from++;
  assert.equal(associateEmbeddedQueries(tree([wrongOffset]), [first]).get("wrong")?.occurrence, null);
});

test("repeated output, includes and ambiguous declarations never gain a source writer", () => {
  const item = occurrence();
  const repeated = associateEmbeddedQueries(tree([node("a", item), node("b", item)]), [item]);
  assert.equal(repeated.get("a")?.unique, false); assert.equal(repeated.get("b")?.unique, false);
  assert.equal(associateEmbeddedQueries(tree([node("include", item, false)]), [item]).get("include")?.occurrence, null);
  assert.equal(associateEmbeddedQueries(tree([node("ambiguous", item)]), [item, { ...item, id: "duplicate" }]).get("ambiguous")?.occurrence, null);
});

test("five modes grant explicit definition commands only; V04 focus cannot reveal syntax", () => {
  assert.equal(embeddedQueryPolicy("source", true, true, true).sourcePreview, true);
  assert.equal(embeddedQueryPolicy("styled", true, true, true).sourceVisible, true);
  assert.equal(embeddedQueryPolicy("focus", true, true, true).reveal, true);
  const write = embeddedQueryPolicy("write", true, true, true);
  assert.equal(write.builder, true); assert.equal(write.sourceDialog, true); assert.equal(write.reveal, false); assert.equal(write.sourceVisible, false);
  assert.equal(embeddedQueryPolicy("write", true, true, false).builder, false);
  assert.equal(embeddedQueryPolicy("write", true, true, false).sourceDialog, true);
  const read = embeddedQueryPolicy("read", true, true, true);
  assert.deepEqual(read, { sourceVisible: false, sourcePreview: false, reveal: false, builder: false, sourceDialog: false, edit: false });
  for (const mode of ["source", "styled", "focus", "write", "read"] as ViewMode[]) assert.equal(embeddedQueryPolicy(mode, false, true, true).edit, false);
});

test("query builder patches change only the chosen occurrence and preserve all outer author bytes", () => {
  const item = occurrence(), from = source.indexOf('"true"'), patches = [{ from, to: from + 6, text: '"false"' }], updated = source.slice(0, from) + '"false"' + source.slice(from + 6);
  const validated = validateEmbeddedEdit(basis, item, { serial: basis.serial, patches, source: updated });
  assert.equal(validated[0].expected, '"true"');
  assert.equal(updated.slice(item.range.utf16From).includes("/* keep */"), true);
  assert.equal(updated.endsWith(occurrence("second").range.raw + "\r\n"), true);
  assert.throws(() => validateEmbeddedEdit(basis, occurrence("second"), { patches, source: updated }), /越过/);
  assert.throws(() => validateEmbeddedEdit(basis, item, { patches, source: updated + "lost" }), /不一致/);
  assert.throws(() => validateEmbeddedEdit(basis, item, { serial: 8, patches, source: updated }), /依据/);
});

test("source confirmation and builder edits share one CodeMirror Undo; mode changes add no content history", () => {
  const item = occurrence(), from = source.indexOf('"true"');
  let state = createSourceState(source, [history()]);
  const builder = validateEmbeddedEdit(basis, item, { patches: [{ from, to: from + 6, text: '"false"' }], source: source.slice(0, from) + '"false"' + source.slice(from + 6) });
  state = state.update({ changes: builder.map(patch => ({ from: patch.from, to: patch.to, insert: patch.insert })), userEvent: "input.query.builder", annotations: isolateHistory.of("full") }).state;
  assert.equal(undoDepth(state), 1);
  for (const mode of ["source", "styled", "focus", "write", "read"] as ViewMode[]) { embeddedQueryPolicy(mode, true, true, true); state = state.update({ selection: { anchor: from } }).state; }
  assert.equal(undoDepth(state), 1);
  state = state.update({ changes: { from, to: from + 7, insert: '"true"' }, userEvent: "input.query.source", annotations: isolateHistory.of("full") }).state;
  assert.equal(undoDepth(state), 2);
  const target = { get state() { return state; }, dispatch(transaction: { state: typeof state }) { state = transaction.state; } };
  assert.equal(undo(target), true); assert.equal(state.doc.toString().includes('where: "false"'), true);
  assert.equal(undo(target), true); assert.equal(state.doc.toString(), source);
  assert.equal(redo(target), true); assert.equal(redo(target), true); assert.equal(state.doc.toString(), source);
});

test("late Q1 result cannot replace Q2, changed parameters or a newer same-definition request", () => {
  const gate = new EmbeddedQueryRunGate(), q1 = gate.begin(basis, "first");
  assert.equal(gate.accepts(q1, basis, result), true);
  assert.equal(gate.accepts(q1, { ...basis, serial: 8 }, result), false);
  assert.equal(gate.accepts(q1, { ...basis, source: source.replace('"true"', '"false"') }, result), false);
  gate.invalidate(); assert.equal(gate.accepts(q1, basis, result), false);
  const q2 = gate.begin(basis, "second"); assert.equal(gate.accepts(q2, basis, result), false);
  const q3 = gate.begin(basis, "first"); assert.equal(gate.accepts(q1, basis, result), false); assert.equal(gate.accepts(q3, basis, result), true);
  assert.equal(result.result.coverage, "partial"); assert.equal(result.result.freshness, "stale");
});

test("discovery installs only its actual draft; invalid intermediate source remains unchanged", async () => {
  let current = { ...basis }, finish!: (value: unknown) => void;
  const controller = new EmbeddedQueryController({ queryResultScope:"local", basis: () => current, mode: () => "write", editable: () => true, request: async <T>() => await new Promise<unknown>(resolve => { finish = resolve; }) as T, prepareEdit: async () => {}, apply: async () => { throw new Error("no edits expected"); }, reveal: () => false, closeReveal: () => {}, openRow: async () => {}, openOwner: async () => {}, notify: () => {} });
  const pending = controller.discover(basis); current = { ...basis, source: source + "#(", serial: 8 }; controller.invalidate();
  finish({ serial: 7, sourceHash: "old", syntaxValid: true, occurrences: [occurrence()] });
  assert.equal(await pending, false); assert.equal(controller.hasQueries, false); assert.equal(current.source, source + "#(");
  assert.equal(sameQueryBasis(current, basis), false);
});

test("disposing an actor/query session clears discovered writers and rejects late rediscovery", async () => {
  let finish!: (value: unknown) => void;
  const controller = new EmbeddedQueryController({ queryResultScope:"local", basis: () => basis, mode: () => "write", editable: () => true, request: async <T>() => await new Promise<unknown>(resolve => { finish = resolve; }) as T, prepareEdit: async () => {}, apply: async () => { throw new Error("no edits expected"); }, reveal: () => false, closeReveal: () => {}, openRow: async () => {}, openOwner: async () => {}, notify: () => {} });
  const initial = controller.discover(basis); finish({ serial: basis.serial, source: basis.source, sourceHash: "original", syntaxValid: true, occurrences: [occurrence()] }); assert.equal(await initial, true); assert.equal(controller.hasQueries, true);
  const late = controller.discover(basis); controller.dispose(); assert.equal(controller.hasQueries, false); assert.deepEqual(controller.outputSelections(), []);
  finish({ serial: basis.serial, source: basis.source, sourceHash: "original", syntaxValid: true, occurrences: [occurrence()] }); assert.equal(await late, false); assert.equal(controller.hasQueries, false);
});

test("native close cannot retire an embedded arrangement waiting for its author preparation", async () => {
  let rejectPreparation!: (error: Error) => void;
  const controller = new EmbeddedQueryController({ queryResultScope: "local", basis: () => basis, mode: () => "write", editable: () => true,
    request: async <T>() => ({ serial: basis.serial, source: basis.source, sourceHash: "original", syntaxValid: true, occurrences: [occurrence()] }) as T,
    prepareEdit: () => new Promise<void>((_resolve, reject) => { rejectPreparation = reject; }),
    apply: async () => { throw new Error("unqualified write must not run"); }, reveal: () => false, closeReveal() {}, openRow: async () => {}, openOwner: async () => {}, notify() {},
  });
  assert.equal(await controller.discover(basis), true);
  const pending = (controller as any).arrange({ occurrence: occurrence(), unique: true }, "remove") as Promise<void>;
  assert.equal(controller.hasUnappliedAuthorInputs, true);
  assert.equal(controller.freezeForClose(), false);
  rejectPreparation(new Error("preparation cancelled"));
  await assert.rejects(pending, /cancelled/);
  assert.equal(controller.hasUnappliedAuthorInputs, false);
  assert.equal(controller.freezeForClose(), true);
  await assert.rejects((controller as any).arrange({ occurrence: occurrence(), unique: true }, "remove"));
  controller.thawAfterClose(); controller.dispose();
});

test("same-byte source and serial cannot reuse a prior author generation",()=>{
 const basis={sessionId:"writer",source:"S",serial:0,inputId:"before"};
 assert.equal(sameQueryBasis({...basis,inputId:"after-ABA"},basis),false);assert.equal(sameQueryBasis({...basis},basis),true);
});
