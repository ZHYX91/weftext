import { StateField, type EditorState, type Text, type Transaction } from "@codemirror/state";
import { LONG_TEXT_RUN } from "./text-layout";

const highSurrogate = (unit: number) => unit >= 0xd800 && unit <= 0xdbff;
const lowSurrogate = (unit: number) => unit >= 0xdc00 && unit <= 0xdfff;

/** Match JavaScript string iteration, including unmatched UTF-16 surrogates. */
export function countSourceCodePoints(source: string): number {
  let pairs = 0;
  for (let i = 1; i < source.length; i++) if (highSurrogate(source.charCodeAt(i - 1)) && lowSurrogate(source.charCodeAt(i))) pairs++;
  return source.length - pairs;
}

interface Range { from: number; to: number }
function surrogatePairs(doc: Text, ranges: Range[]): number {
  let total = 0;
  const count = (from: number, to: number) => {
    const cursor = doc.iterRange(from, to); let previousHigh = false;
    while (!cursor.next().done) for (let i = 0; i < cursor.value.length; i++) {
      const unit = cursor.value.charCodeAt(i);
      if (previousHigh && lowSurrogate(unit)) total++;
      previousHigh = highSurrogate(unit);
    }
  };
  let from = -1, to = -1;
  for (const range of ranges) {
    if (from < 0) { from = range.from; to = range.to; }
    else if (range.from <= to) to = Math.max(to, range.to);
    else { count(from, to); from = range.from; to = range.to; }
  }
  if (from >= 0) count(from, to);
  return total;
}

function updateCodePoints(value: number, transaction: Transaction): number {
  if (!transaction.docChanged) return value;
  const before = transaction.startState.doc, after = transaction.newDoc, oldRanges: Range[] = [], newRanges: Range[] = [];
  // One unchanged unit at either side includes every pair a splice can create
  // or split. Merge overlapping windows so adjacent edits cannot double count.
  transaction.changes.iterChangedRanges((fromA, toA, fromB, toB) => {
    oldRanges.push({ from: Math.max(0, fromA - 1), to: Math.min(before.length, toA + 1) });
    newRanges.push({ from: Math.max(0, fromB - 1), to: Math.min(after.length, toB + 1) });
  });
  return value + after.length - before.length - surrogatePairs(after, newRanges) + surrogatePairs(before, oldRanges);
}

export const sourceCodePointCount = StateField.define<number>({
  create: state => state.doc.length - surrogatePairs(state.doc, [{ from: 0, to: state.doc.length }]),
  update: updateCodePoints,
});

// Keep wrapping but bound native kerning work while a huge logical line exists.
export const LONG_SOURCE_LINE = LONG_TEXT_RUN;
function initialLongLines(state: EditorState): readonly number[] {
  const positions: number[] = [], lines = state.doc.iterLines(); let from = 0;
  while (!lines.next().done) { if (lines.value.length > LONG_SOURCE_LINE) positions.push(from); from += lines.value.length + 1; }
  return positions;
}
export const sourceLongLines = StateField.define<readonly number[]>({
  create: initialLongLines,
  update: (positions, transaction) => {
    if (!transaction.docChanged) return positions;
    const doc = transaction.newDoc, next = new Set<number>();
    const inspect = (at: number) => { const line = doc.lineAt(at); if (line.length > LONG_SOURCE_LINE) next.add(line.from); };
    // Existing long lines can shrink, split, move or disappear. Only they and
    // lines touched by the actual new ranges need inspection after an edit.
    for (const position of positions) inspect(transaction.changes.mapPos(position));
    transaction.changes.iterChangedRanges((_fromA, _toA, fromB, toB) => {
      const first = doc.lineAt(fromB).number, last = doc.lineAt(toB).number;
      for (let number = first; number <= last; number++) inspect(doc.line(number).from);
    });
    return [...next].sort((a, b) => a - b);
  },
});
