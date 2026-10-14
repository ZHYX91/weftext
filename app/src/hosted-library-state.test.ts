import assert from "node:assert/strict";
import test from "node:test";
import { decodeLibraryDraft, freezeLibraryPrepare, libraryCanCommit, libraryChanges, libraryInput, libraryReceipt, type LibraryDraft, type LibraryProfile } from "./hosted-library-state";
const profile: LibraryProfile = { role: "style", basis: "opaque", source: "#let library-style(body) = { set text(size: 12pt); body }", installed: true, enabled: true, controls: [{ key: "text.size", label: "字体", kind: "text", value: "12pt", editable: true }] };
const draft = (): LibraryDraft => ({ source: profile.source, baseSource: profile.source, enabled: true, baseEnabled: true, changes: { "text.size": "14pt" }, revision: 2 });
test("hosted library freezes native expressions and preserves unknown/custom source bytes", () => {
  const input = draft(), request = freezeLibraryPrepare("style", profile, input, "original"); assert.deepEqual(request, { role: "style", basis: "opaque", operationId: "original", changes: { "text.size": "14pt" } }); input.changes["text.size"] = "18pt"; assert.equal(request.changes?.["text.size"], "14pt");
  const schema = '{"unknown":{"complex":[1,"😀"]}, "fields": []}\n', json: LibraryProfile = { role: "schema", basis: "j", source: "{}", installed: true }; assert.equal(freezeLibraryPrepare("schema", json, { source: schema, baseSource: "{}", changes: {}, revision: 0 }, "json").source, schema);
});
test("hosted library incomplete/ambiguous controls retain raw input and never prepare old values", () => {
  const input = draft(); input.source += "\n// custom"; assert.throws(() => freezeLibraryPrepare("style", profile, input, "x"), /同时有改动/); input.source = profile.source; input.changes["old.parameter"] = "bad original"; assert.throws(() => freezeLibraryPrepare("style", profile, input, "x"), /原控件输入/); assert.equal(input.changes["old.parameter"], "bad original");
  const controls = [{ key: "number", label: "数值", kind: "number" as const, value: 2, editable: true, min: 1 }]; assert.throws(() => libraryChanges(controls, { number: "" }), /原数值无效/); assert.throws(() => libraryChanges(controls, { number: "NaN" }), /原数值无效/);
  assert.throws(() => freezeLibraryPrepare("schema", { role: "schema", source: "{}", basis: "j", installed: true }, { source: '{"unfinished":', baseSource: "{}", revision: 0, changes: {} }, "x"), /尚未完成/);
});
test("hosted library original receipts are immutable and permission-filtered replies drop old preview", () => {
  const request = freezeLibraryPrepare("style", profile, draft(), "original"), receipt = { role: "style" as const, operationId: "original", status: "prepared", planId: "p", commitAllowed: true, preview: { beforeSource: profile.source, afterSource: "new", summary: ["change"] } };
  assert.throws(() => libraryReceipt(request, { ...receipt, operationId: "other" })); assert.throws(() => libraryReceipt(request, { ...receipt, role: "schema" })); assert.throws(() => libraryReceipt(request, { ...receipt, planId: "other" }, receipt));
  assert.deepEqual(libraryReceipt(request, { role: "style", operationId: "original", status: "committed", commitAllowed: false }, receipt), { role: "style", operationId: "original", status: "committed", commitAllowed: false });
  assert.throws(() => libraryReceipt(request, { ...receipt, preview: {} }));
  assert.throws(() => libraryReceipt(request, receipt, undefined, "another basis"));
  assert.throws(() => libraryReceipt({ ...request, source: "exact proposal" }, receipt));
});
test("hosted library new input and ABA block commit without changing original request", () => {
  const input = draft(), request = freezeLibraryPrepare("style", profile, input, "original"); input.pending = { request, input: libraryInput(input), revision: input.revision, receipt: { role: "style", operationId: "original", status: "prepared", planId: "p", commitAllowed: true } }; assert.equal(libraryCanCommit(input), true);
  input.changes["text.size"] = "18pt"; input.revision++; assert.equal(libraryCanCommit(input), false); input.changes["text.size"] = "14pt"; input.revision++; assert.equal(libraryCanCommit(input), false); assert.equal(request.changes?.["text.size"], "14pt"); assert.deepEqual(decodeLibraryDraft(JSON.stringify(input), "style"), input); assert.throws(() => decodeLibraryDraft(JSON.stringify(input), "schema"));
});
test("hosted library explicitly installs an absent baseline without inventing an input edit", () => {
  const missing: LibraryProfile = { role: "schema", basis: "missing", source: '{"fields":[]}', installed: false }, input: LibraryDraft = { source: missing.source, baseSource: missing.source, changes: {}, revision: 0 };
  assert.equal(freezeLibraryPrepare("schema", missing, input, "install-original").source, missing.source); assert.throws(() => freezeLibraryPrepare("schema", { ...missing, installed: true }, input, "x"), /没有待提交/);
});
