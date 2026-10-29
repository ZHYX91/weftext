import test from "node:test";
import assert from "node:assert/strict";
import { selectedTableAxis, tableAxisDrop } from "./table-axis";

test("whole-axis selection keeps its inclusive group; a partial rectangle cannot silently enlarge it", () => {
    const table = { rows: 7, columns: 5 }, head = { row: 4, column: 3 };
    assert.deepEqual(selectedTableAxis(table, { top: 2, bottom: 4, left: 0, right: 4 }, head, "row"), { axis: "row", start: 2, end: 4 });
    assert.deepEqual(selectedTableAxis(table, { top: 0, bottom: 6, left: 1, right: 3 }, head, "column"), { axis: "column", start: 1, end: 3 });
    assert.deepEqual(selectedTableAxis(table, { top: 2, bottom: 4, left: 1, right: 3 }, head, "row"), { axis: "row", start: 4, end: 4 });
    assert.deepEqual(selectedTableAxis(table, { top: 2, bottom: 4, left: 1, right: 3 }, head, "column"), { axis: "column", start: 3, end: 3 });
});

test("group drop destinations use the final start and preserve exact before/after scope in both directions", () => {
    for (const axis of ["row", "column"] as const) {
        const group = { axis, start: 2, end: 4 };
        assert.deepEqual(tableAxisDrop(group, 0, 8), { to: 0, edge: "before" });
        assert.deepEqual(tableAxisDrop(group, 1, 8), { to: 1, edge: "before" });
        assert.deepEqual(tableAxisDrop(group, 5, 8), { to: 3, edge: "after" });
        assert.deepEqual(tableAxisDrop(group, 7, 8), { to: 5, edge: "after" });
        for (const target of [2, 3, 4, -1, 8, Number.NaN]) assert.equal(tableAxisDrop(group, target, 8), null);
        assert.equal(tableAxisDrop({ axis, start: 4, end: 2 }, 0, 8), null);
        assert.equal(tableAxisDrop({ axis, start: 2, end: 8 }, 0, 8), null);
    }
});
