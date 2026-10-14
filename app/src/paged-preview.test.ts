import assert from "node:assert/strict";
import test from "node:test";
import { decodePagedPdf, pagedFileName, pagedResultMatches, samePagedBasis, type PagedBasis, type PagedOutput } from "./paged-preview";

const basis: PagedBasis = { sessionId: "session-a", serial: 2, source: "当前草稿", environment: "style-v1" };
const result: PagedOutput = {
  ok: true, target: "paged", pdfBase64: null, pageCount: 1, diagnostics: [], dependencies: [], fontFamilies: [],
  compilerVersion: "0.15.1", serial: basis.serial, source: basis.source, environment: basis.environment,
};

test("paged output requires the exact Draft and current environment", () => {
  assert.equal(samePagedBasis(basis, { ...basis }), true);
  assert.equal(samePagedBasis(basis, null), false);
  assert.equal(samePagedBasis(basis, { ...basis, serial: 3 }), false);
  assert.equal(samePagedBasis(basis, { ...basis, source: "新草稿" }), false);
  assert.equal(samePagedBasis(basis, { ...basis, sessionId: "session-b" }), false);
  assert.equal(samePagedBasis(basis, { ...basis, environment: "style-v2" }), false);
  assert.equal(pagedResultMatches(basis, result), true);
  assert.equal(pagedResultMatches(basis, { ...result, environment: "style-v2" }), false);
  assert.equal(pagedResultMatches(basis, { ...result, serial: 1 }), false);
  assert.equal(pagedResultMatches(basis, { ...result, source: "旧草稿" }), false);
  assert.equal(pagedResultMatches(basis, { ...result, compilerVersion: "0.14.0" }), false);
});

test("native preview accepts PDF bytes and rejects HTML or arbitrary output", () => {
  const bytes = decodePagedPdf(Buffer.from("%PDF-1.7\nfixture bytes\n%%EOF").toString("base64"));
  assert.equal(new TextDecoder().decode(bytes).startsWith("%PDF-"), true);
  assert.throws(() => decodePagedPdf(Buffer.from("<html><script>unsafe</script></html>").toString("base64")), /PDF/);
  assert.throws(() => decodePagedPdf("data:application/pdf;base64,AAAA"), /PDF/);
  assert.throws(() => decodePagedPdf("%%%="), /PDF/);
  assert.throws(() => decodePagedPdf(""), /PDF/);
});

test("download filenames preserve the actual document name without host paths", () => {
  assert.equal(pagedFileName("D:\\笔记\\研究.typ"), "研究.pdf");
  assert.equal(pagedFileName("notes/with.dots.typ"), "with.dots.pdf");
  assert.equal(pagedFileName(""), "草稿.pdf");
});
