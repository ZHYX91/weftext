import test from "node:test";
import assert from "node:assert/strict";
import { verifiedTableCaretOffset } from "./table-cell-caret";

test("table click offsets retain Chinese, whitespace, repeated words, hard breaks and whole emoji", () => {
    const text = "相同 相同\n😀e\u0301尾";
    for (const prefix of ["", "相同", "相同 ", "相同 相同\n", "相同 相同\n😀", "相同 相同\n😀e\u0301", text]) {
        assert.equal(verifiedTableCaretOffset(text, text, text, prefix), prefix.length);
    }
    assert.equal(verifiedTableCaretOffset("", "", "", ""), 0);
});

test("a point cannot authorize a different display or partial native literal", () => {
    assert.equal(verifiedTableCaretOffset("same", "same", "another", "sa"), null);
    assert.equal(verifiedTableCaretOffset("same", "same", "same", "wrong"), null);
    assert.equal(verifiedTableCaretOffset("same", "same", "same", null), null);
    assert.equal(verifiedTableCaretOffset("#generated()", "same", "same", "sa"), null);
    assert.equal(verifiedTableCaretOffset("", "", "\u00a0", ""), null);
});

test("partial surrogate, combining and joined emoji clusters are refused rather than guessed", () => {
    for (const [text, prefix] of [["😀尾", "\ud83d"], ["e\u0301尾", "e"], ["👩‍💻尾", "👩"], ["\r\n尾", "\r"]]) {
        assert.equal(verifiedTableCaretOffset(text, text, text, prefix), null);
    }
});

test("native escaped and hard-break literals accept only proven source stops", () => {
    assert.equal(verifiedTableCaretOffset("a\\#b", "a#b", "a#b", "a#"), 2);
    assert.equal(verifiedTableCaretOffset("a\\\nb", "a\nb", "a\nb", "a\n"), 2);
    assert.equal(verifiedTableCaretOffset("a#b", "a#c", "a#c", "a#"), null);
});
