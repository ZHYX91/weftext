import assert from "node:assert/strict";
import test from "node:test";
import { decodeLibraryDraft, freezeLibraryPrepare, libraryCanCommit, libraryChanges, libraryInput, libraryReceipt, type LibraryDraft, type LibraryProfile } from "./hosted-library-state";
import { locale, message, uiTextValue } from "./ui-locale";
const profile: LibraryProfile = { role: "style", basis: "opaque", source: "#let library-style(body) = { set text(size: 12pt); body }", installed: true, enabled: true, warnings: [], controls: [{ key: "text.size", label: message("common.save"), kind: "text", value: "12pt", editable: true, reason: null }] };
const draft = (): LibraryDraft => ({ source: profile.source, baseSource: profile.source, enabled: true, baseEnabled: true, changes: { "text.size": "14pt" }, revision: 2 });
test("hosted library freezes native expressions and preserves unknown/custom source bytes", () => {
  const input = draft(), request = freezeLibraryPrepare("style", profile, input, "original"); assert.deepEqual(request, { role: "style", basis: "opaque", operationId: "original", changes: { "text.size": "14pt" } }); input.changes["text.size"] = "18pt"; assert.equal(request.changes?.["text.size"], "14pt");
  const schema = '{"unknown":{"complex":[1,"😀"]}, "fields": []}\n', json: LibraryProfile = { role: "schema", basis: "j", source: "{}", installed: true }; assert.equal(freezeLibraryPrepare("schema", json, { source: schema, baseSource: "{}", changes: {}, revision: 0 }, "json").source, schema);
});
test("hosted library incomplete/ambiguous controls retain raw input and never prepare old values", () => {
  const input = draft(); input.source += "\n// custom"; assert.throws(() => freezeLibraryPrepare("style", profile, input, "x"), /同时有改动/); input.source = profile.source; input.changes["old.parameter"] = "bad original"; assert.throws(() => freezeLibraryPrepare("style", profile, input, "x"), /原控件输入/); assert.equal(input.changes["old.parameter"], "bad original");
  assert.deepEqual(libraryChanges(profile.controls!, { "text.size": " 12.5pt " }), { "text.size": " 12.5pt " });
  assert.throws(() => libraryChanges(profile.controls!, { "text.size": false } as unknown as Record<string, string>), /恢复输入无效/);
  assert.throws(() => freezeLibraryPrepare("schema", { role: "schema", source: "{}", basis: "j", installed: true }, { source: '{"unfinished":', baseSource: "{}", revision: 0, changes: {} }, "x"), /尚未完成/);
});
test("hosted library original receipts are immutable and permission-filtered replies drop old preview", () => {
  const request = freezeLibraryPrepare("style", profile, draft(), "original"), receipt = { role: "style" as const, operationId: "original", status: "prepared", planId: "p", commitAllowed: true, preview: { beforeSource: profile.source, afterSource: "new", summary: message("common.save") } };
  assert.throws(() => libraryReceipt(request, { ...receipt, operationId: "other" })); assert.throws(() => libraryReceipt(request, { ...receipt, role: "schema" })); assert.throws(() => libraryReceipt(request, { ...receipt, planId: "other" }, receipt));
  const committed = { role: "style", operationId: "original", status: "committed", planId: null, preview: {}, commitAllowed: false };
  assert.deepEqual(libraryReceipt(request, committed, receipt), committed);
  assert.throws(() => libraryReceipt(request, { ...receipt, preview: {} }));
  assert.throws(() => libraryReceipt(request, receipt, undefined, "another basis"));
  assert.throws(() => libraryReceipt({ ...request, source: "exact proposal" }, receipt));
});
test("hosted library new input and ABA block commit without changing original request", () => {
  const input = draft(), request = freezeLibraryPrepare("style", profile, input, "original"); input.pending = { request, input: libraryInput(input), revision: input.revision, planId: "p", receipt: { role: "style", operationId: "original", status: "prepared", planId: "p", commitAllowed: true, preview: { beforeSource: profile.source, afterSource: "new", summary: message("common.save") } } }; assert.equal(libraryCanCommit(input), true);
  input.changes["text.size"] = "18pt"; input.revision++; assert.equal(libraryCanCommit(input), false); input.changes["text.size"] = "14pt"; input.revision++; assert.equal(libraryCanCommit(input), false); assert.equal(request.changes?.["text.size"], "14pt"); assert.deepEqual(decodeLibraryDraft(JSON.stringify(input), "style"), input); assert.throws(() => decodeLibraryDraft(JSON.stringify(input), "schema"));
});
test("hosted library explicitly installs an absent baseline without inventing an input edit", () => {
  const missing: LibraryProfile = { role: "schema", basis: "missing", source: '{"fields":[]}', installed: false }, input: LibraryDraft = { source: missing.source, baseSource: missing.source, changes: {}, revision: 0 };
  assert.equal(freezeLibraryPrepare("schema", missing, input, "install-original").source, missing.source); assert.throws(() => freezeLibraryPrepare("schema", { ...missing, installed: true }, input, "x"), /没有待提交/);
});

test("cached typed summaries are rebranded and a redacted reply cannot replace the pinned original plan", () => {
  const input = draft(), request = freezeLibraryPrepare("style", profile, input, "original");
  input.pending = { request, input: libraryInput(input), revision: input.revision, planId: "p", receipt: { role: "style", operationId: "original", status: "prepared", planId: "p", commitAllowed: true, preview: { beforeSource: profile.source, afterSource: "new", summary: message("common.save") } } };
  const raw = JSON.stringify(input), restored = decodeLibraryDraft(raw, "style");
  locale.set("en-US"); assert.equal(uiTextValue(restored.pending!.receipt!.preview.summary), "Save"); locale.set("zh-CN");
  const redacted = libraryReceipt(request, { role: "style", operationId: "original", status: "prepared", planId: null, preview: {}, commitAllowed: false }, restored.pending!.receipt, undefined, "p");
  assert.deepEqual(redacted.preview, {}); assert.equal(redacted.planId, null);
  assert.throws(() => libraryReceipt(request, { ...input.pending!.receipt, planId: "replacement" }, redacted, undefined, "p"));
  assert.throws(() => libraryReceipt(request, { role: "style", operationId: "original", status: "committed", planId: null, preview: { beforeSource: profile.source, afterSource: "substituted source", summary: message("common.save") }, commitAllowed: false }, restored.pending!.receipt, profile.source, "p"));
  const malformed = JSON.parse(raw); malformed.pending.receipt.preview.summary = "old protocol"; assert.throws(() => decodeLibraryDraft(JSON.stringify(malformed), "style"));
  assert.equal(input.source, profile.source); assert.equal(input.changes["text.size"], "14pt");
});
