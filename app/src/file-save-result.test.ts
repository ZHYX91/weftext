import test from "node:test";
import assert from "node:assert/strict";
import { fileSaveReceipt } from "./file-save-result";
import { submitOriginal, type OriginalRequest } from "./original-operation";
import { DocumentSession, rebindCommittedSave } from "./session";

const frozen = { operationId: "original", sessionId: "writer", source: "submitted S", sourcePath: "C:/TEST/a.typ", baseId: "before" };
const document = { sessionId: "fresh", baseId: "actual", path: frozen.sourcePath, root: "C:/TEST", source: frozen.source, kind: "ordinary" };
const receipt = { operationId: frozen.operationId, status: "committed", baseId: "original-base", path: frozen.sourcePath, message: "saved", committedSource: frozen.source, document, documentSourceMatchesCommit: true };
const malformed = [
  { baseId: undefined }, { baseId: "" }, { baseId: 12 }, { path: undefined }, { path: 12 }, { path: "C:/TEST/other.typ" },
  { operationId: "other" }, { status: "complete" }, { status: ["saved"] }, { message: undefined },
  { document: undefined }, { document: { ...document, sessionId: undefined } }, { document: { ...document, baseId: 1 } },
  { document: { ...document, kind: ["ordinary"] } }, { document: { ...document, recoveredDraftConflict: "false" } },
  { committedSource: "other submission" }, { documentSourceMatchesCommit: undefined }, { documentSourceMatchesCommit: false },
  { documentError: {} }, { documentError: { code: "io", message: "unavailable" } },
];
test("initial and queried file saves share the exact original receipt and qualified-delivery contract", () => {
  for (const status of ["saved", "committed"]) {
    assert.equal(fileSaveReceipt({ ...receipt, status }, frozen).document?.baseId, "actual");
    for (const change of malformed) assert.throws(() => fileSaveReceipt({ ...receipt, status, ...change }, frozen));
  }
  const unavailable = { ...receipt, document: undefined, documentSourceMatchesCommit: undefined, documentError: { code: "sourceIdentityChanged", message: "Original unavailable" } };
  assert.equal(fileSaveReceipt(unavailable, frozen).baseId, receipt.baseId);
  assert.equal(fileSaveReceipt({ ...receipt, document: { ...document, source: "later T" }, documentSourceMatchesCommit: false }, frozen).document?.source, "later T");
  assert.throws(() => fileSaveReceipt(receipt, { ...frozen, path: "C:/TEST/save-as.typ" }));
  assert.equal(fileSaveReceipt(receipt, { ...frozen, path: "c:\\TEST\\folder\\..\\a.typ" }).path, frozen.sourcePath);
});
test("bad initial and status file receipts preserve original attempt and input; later valid status heals without another author request", async () => {
  for (const change of malformed.filter(change => !("operationId" in change) && !("status" in change))) {
    let authors = 0, lookups = 0;
    const session = new DocumentSession({ ...document, sessionId: "writer", source: "before B", baseId: "before" });
    session.edit("submitted S + input Y");
    const request: OriginalRequest = async <T>(route: string) => {
      if (route === "/api/save") { authors++; return { ...receipt, status: "saved", ...change } as T; }
      assert.equal(route, "/api/operations/status"); lookups++;
      return (lookups === 1 ? { ...receipt, ...change } : receipt) as T;
    };
    const payload = { operationId: frozen.operationId, sessionId: frozen.sessionId, baseId: frozen.baseId, source: frozen.source };
    for (const recoverFirst of [false, true]) {
      await assert.rejects(async () => fileSaveReceipt(await submitOriginal(request, "/api/save", payload, "save", recoverFirst), frozen));
      assert.equal(session.baseId, "before"); assert.equal(session.source, "submitted S + input Y");
    }
    const good = fileSaveReceipt(await submitOriginal(request, "/api/save", payload, "save", true), frozen);
    assert.equal(rebindCommittedSave(session, frozen, good.document!), true);
    assert.equal(session.baseId, "actual"); assert.equal(session.source, "submitted S + input Y");
    assert.equal(authors, 1); assert.equal(lookups, 2);
  }
});
