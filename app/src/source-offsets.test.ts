import assert from "node:assert/strict";
import test from "node:test";
import { sourceByteOffsets } from "./source-offsets";

test("snapshot byte positions match native UTF-8 prefixes, including unmatched surrogates", () => {
  const units = ["a", "\u007f", "\u0080", "\u07ff", "\u0800", "中", "\ud800", "\udc00", "😀", "\r\n", "e\u0301"];
  const source = units.flatMap(left => units.map(right => left + right)).join("");
  const offsets = sourceByteOffsets(source);
  assert.equal(offsets.length, source.length + 1);
  for (let i = 0; i <= source.length; i++) assert.equal(offsets[i], new TextEncoder().encode(source.slice(0, i)).length, `UTF-16 prefix ${i}`);
});
