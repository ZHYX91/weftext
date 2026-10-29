/** Ephemeral selection geometry. Native source and the Engine own all writes. */
export type TableAxis = "row" | "column";
export interface TableAxisGroup { axis: TableAxis; start: number; end: number; }
interface Dimensions { rows: number; columns: number; }
interface Rectangle { top: number; bottom: number; left: number; right: number; }
interface Position { row: number; column: number; }

export function selectedTableAxis(table: Dimensions, range: Rectangle, head: Position, axis: TableAxis): TableAxisGroup {
    if (axis === "row") return range.left === 0 && range.right === table.columns - 1
        ? { axis, start: range.top, end: range.bottom } : { axis, start: head.row, end: head.row };
    return range.top === 0 && range.bottom === table.rows - 1
        ? { axis, start: range.left, end: range.right } : { axis, start: head.column, end: head.column };
}

/** A downward/rightward drop places the group's end at the pointed handle. */
export function tableAxisDrop(group: TableAxisGroup, target: number, length: number): { to: number; edge: "before" | "after" } | null {
    if (![group.start, group.end, target, length].every(Number.isSafeInteger)
        || group.start < 0 || group.end < group.start || group.end >= length || target < 0 || target >= length
        || target >= group.start && target <= group.end) return null;
    const count = group.end - group.start + 1;
    return target < group.start ? { to: target, edge: "before" } : { to: target - count + 1, edge: "after" };
}
