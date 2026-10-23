import test from "node:test";
import assert from "node:assert/strict";
import { delimitedTable, flatClipboard, parseDelimited, writeDelimited, writeHTMLTable } from "./table-clipboard";
import { adjacentTableCell, logicalTableCells, tableCellAt, tableRectangle } from "./table-grid";
import { writingRange, type WritingTable, type WritingTableCell } from "./writing";

test("quoted TSV and CSV preserve empty, repeated, newline and escaped quote cells without partial parsing", () => {
  const values = [["重复", "重复", ""], ["两行\n中文😀", 'a"b', "c\td"]];
  for (const delimiter of ["\t", ","] as const) assert.deepEqual(parseDelimited(writeDelimited(values, delimiter), delimiter), values);
  for (const values of [[["a"], [""]], [[""], [""]], [[""]]]) for (const delimiter of ["\t", ","] as const) assert.deepEqual(parseDelimited(writeDelimited(values, delimiter), delimiter), values);
  assert.deepEqual(parseDelimited("a\tb\r\n\t\r\n", "\t"), [["a", "b"], ["", ""]]);
  for (const text of ['"未结束\t二', '"a"junk\tb', "a\tb\nc"]) assert.throws(() => delimitedTable(text, "\t"));
  assert.throws(() => parseDelimited("x\n".repeat(4097), "\t"), /预算/);
});

test("HTML serialization escapes author text and flat formats name the merged origin strategy", () => {
  const table = { rows: 2, columns: 2, cells: [{ row: 0, column: 0, rowspan: 1, colspan: 2, text: '<img src="external">\n第二行', header: true }, { row: 1, column: 0, rowspan: 1, colspan: 1, text: "", header: false }, { row: 1, column: 1, rowspan: 1, colspan: 1, text: "重复", header: false }], losses: [] };
  const html = writeHTMLTable(table); assert.ok(html.includes('<th rowspan="1" colspan="2">')); assert.ok(html.includes("&lt;img src=&quot;external&quot;&gt;<br>第二行")); assert.ok(!html.includes("<img"));
  assert.deepEqual(flatClipboard(table), [[table.cells[0].text, ""], ["", "重复"]]);
});

const body = writingRange("same", 0, 4);
function cell(row: number, column: number, rowspan = 1, colspan = 1): WritingTableCell { return { row, column, rowspan, colspan, body, range: body, header: false }; }
test("logical navigation skips merged coverage and repeated text never identifies a target", () => {
  const table: WritingTable = { columns: 3, rows: 3, cells: [], gridWritable: false, topologyWritable: true, unavailableReason: null, headerRows: 0, headerColumns: 0, logicalCells: [cell(0, 0, 2, 2), cell(0, 2), cell(1, 2), cell(2, 0), cell(2, 1), cell(2, 2)] };
  assert.equal(tableCellAt(table, { row: 1, column: 1 }), table.logicalCells![0]);
  assert.deepEqual(adjacentTableCell(table, { row: 0, column: 0 }, "next"), { row: 0, column: 2 });
  assert.deepEqual(adjacentTableCell(table, { row: 0, column: 0 }, "down"), { row: 2, column: 0 });
  assert.equal(adjacentTableCell(table, { row: 0, column: 0 }, "previous"), null);
  assert.deepEqual(tableRectangle(table, { row: 1, column: 1 }, { row: 2, column: 2 }), { top: 0, left: 0, bottom: 2, right: 2 });
});
test("native logical slots remain independent even with equal raw text", () => {
  const table: WritingTable = { columns: 2, rows: 2, cells: [body, body, body, body], logicalCells: [cell(0, 0), cell(0, 1), cell(1, 0), cell(1, 1)], gridWritable: true, topologyWritable: true, unavailableReason: null, headerRows: 0, headerColumns: 0 };
  assert.deepEqual(logicalTableCells(table).map(cell => [cell.row, cell.column]), [[0, 0], [0, 1], [1, 0], [1, 1]]);
  assert.deepEqual(tableRectangle(table, { row: 1, column: 1 }, { row: 0, column: 0 }), { top: 0, left: 0, bottom: 1, right: 1 });
});
