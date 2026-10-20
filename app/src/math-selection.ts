import type { Origin, RenderNode } from "./types";

export interface MathSelection {
  from: number;
  to: number;
  visibleExcerpt: string;
  nodeKey: string;
  kind: "whole" | "subexpression" | "symbol";
  raw: string;
}
export interface MathSelectionPlan { candidates: MathSelection[]; unavailable: string[] }
const structures = new Set(["math", "mrow", "mfrac", "msub", "msup", "msubsup", "msqrt", "mroot", "mfenced", "mtable", "munder", "mover", "munderover"]);
const symbols = new Set(["mi", "mn", "mo", "mtext", "ms"]);
const excluded = new Set(["annotation", "annotation-xml"]);

function exact(origin: Origin | null, source: string, bytes: Map<number, number>, graphemes: Set<number>): origin is Origin {
  if (!origin?.isMain || !Number.isSafeInteger(origin.from) || !Number.isSafeInteger(origin.to)
    || !Number.isSafeInteger(origin.utf16From) || !Number.isSafeInteger(origin.utf16To)
    || origin.from < 0 || origin.to <= origin.from || origin.utf16From < 0 || origin.utf16To <= origin.utf16From
    || source.slice(origin.utf16From, origin.utf16To) !== origin.raw
    || !graphemes.has(origin.utf16From) || !graphemes.has(origin.utf16To)) return false;
  return bytes.get(origin.utf16From) === origin.from && bytes.get(origin.utf16To) === origin.to;
}
function plain(node: RenderNode): string {
  if (node.kind === "tag" || excluded.has(node.tag ?? "")) return "";
  return (node.text ?? "") + node.children.map(plain).join("");
}
const originKey = (origin: Origin) => JSON.stringify([origin.path, origin.from, origin.to]);

/** A native origin identifies a source occurrence; glyph spelling never does.
 * A producer may represent one source expression with several generated child
 * objects (a cases brace plus its matrix). Only their common same-source outer
 * object represents the complete expression. Separate appearances without such
 * an object are ambiguous. The full projection is required for this test.
 */
export function planMathSelections(math: RenderNode, tree: RenderNode, source: string): MathSelectionPlan {
  const candidates: MathSelection[] = [], unavailable: string[] = [];
  if (math.tag !== "math") return { candidates, unavailable: ["所选对象不是当前原生数学树。"] };
  // Build both maps once, including for a document with many math symbols.
  // UTF-16 offsets in the middle of a surrogate have no UTF-8 boundary entry.
  const bytes = new Map<number, number>([[0, 0]]), graphemes = new Set<number>([source.length]);
  let utf16 = 0, byte = 0;
  for (const scalar of source) { const cp = scalar.codePointAt(0)!; utf16 += scalar.length; byte += cp < 0x80 ? 1 : cp < 0x800 ? 2 : cp < 0x10000 ? 3 : 4; bytes.set(utf16, byte); }
  for (const part of new Intl.Segmenter(undefined, { granularity: "grapheme" }).segment(source)) graphemes.add(part.index);
  const paths: { node: RenderNode; path: number[] }[] = [];
  const walk = (node: RenderNode, path: number[]) => {
    if (node.kind === "tag" || excluded.has(node.tag ?? "")) return;
    if (node.kind !== "text" && (structures.has(node.tag ?? "") || symbols.has(node.tag ?? ""))) paths.push({ node, path });
    node.children.forEach((child, index) => walk(child, [...path, index]));
  };
  walk(tree, []);
  if (!paths.some(item => item.node === math)) return { candidates, unavailable: ["公式已经不在当前消费投影中。"] };
  const occurrences = new Map<string, { node: RenderNode; path: number[] }[]>();
  for (const item of paths) if (item.node.origin) {
    const key = originKey(item.node.origin), values = occurrences.get(key) ?? [];
    values.push(item); occurrences.set(key, values);
  }
  const prefix = (a: number[], b: number[]) => a.length <= b.length && a.every((n, i) => b[i] === n);
  const canonical = new Map<string, RenderNode | null>();
  for (const [key, matches] of occurrences) {
    const outer = matches.reduce((a, b) => a.path.length <= b.path.length ? a : b);
    canonical.set(key, matches.every(b => prefix(outer.path, b.path)) ? outer.node : null);
  }
  const unique = (origin: Origin, node: RenderNode) => {
    return canonical.get(originKey(origin)) === node;
  };
  const seen = new Set<string>();
  const collect = (node: RenderNode) => {
    if (node.kind === "tag" || excluded.has(node.tag ?? "")) return;
    const tag = node.tag ?? "", origin = node.origin;
    if (node === math || structures.has(tag) || symbols.has(tag)) {
      if (exact(origin, source, bytes, graphemes) && unique(origin, node)) {
        const key = `${origin.from}:${origin.to}`;
        if (!seen.has(key)) {
          seen.add(key);
          candidates.push({ from: origin.utf16From, to: origin.utf16To, visibleExcerpt: plain(node), raw: origin.raw,
            nodeKey: node.key, kind: node === math ? "whole" : structures.has(tag) ? "subexpression" : "symbol" });
        }
      } else if (node === math) unavailable.push("此公式没有唯一且当前的完整源范围。可主动选择准确源码范围建立讨论。");
    }
    node.children.forEach(collect);
  };
  collect(math);
  if (!candidates.some(item => item.kind === "symbol" || item.kind === "subexpression"))
    unavailable.push("内部符号／子式没有独立的原生来源证明，未扩大为整个公式。");
  return { candidates, unavailable };
}

export interface MathSelectionControlsOptions {
  math: RenderNode;
  tree: RenderNode;
  source: string;
  /** Return false after a source, session, permission or projection change. */
  current(): boolean;
  select(selection: MathSelection): void;
  notify?(message: string): void;
}

/** Explicit element selection is available in reading mode too; selecting a
 * discussion range never grants permission to modify the mathematical source.
 */
export function createMathSelectionControls(options: MathSelectionControlsOptions): HTMLElement {
  const details = document.createElement("details"); details.className = "math-selection-controls";
  const summary = document.createElement("summary"); summary.textContent = "公式批注范围"; details.append(summary);
  const plan = planMathSelections(options.math, options.tree, options.source);
  const picker = document.createElement("select"); picker.setAttribute("aria-label", "选择整个公式、符号或子式");
  for (const [index, candidate] of plan.candidates.entries()) {
    const option = document.createElement("option"); option.value = String(index);
    const kind = candidate.kind === "whole" ? "整个公式" : candidate.kind === "symbol" ? "符号" : "子式";
    option.textContent = `${kind} · ${candidate.raw.slice(0, 80)} · 源 ${candidate.from}–${candidate.to}`;
    picker.append(option);
  }
  const preview = document.createElement("pre"); preview.setAttribute("aria-label", "实际批注源范围");
  const update = () => { preview.textContent = plan.candidates[Number(picker.value)]?.raw ?? "没有可证明的数学范围。"; };
  picker.onchange = update; update();
  const select = document.createElement("button"); select.type = "button"; select.textContent = "采用此范围"; select.disabled = !plan.candidates.length;
  select.onclick = () => {
    const candidate = plan.candidates[Number(picker.value)];
    if (!candidate || !options.current()) { options.notify?.("公式来源已变化，保留原批注输入，请重新选择当前公式范围。"); return; }
    options.select({ ...candidate });
  };
  details.append(picker, preview, select);
  for (const message of plan.unavailable) { const hint = document.createElement("p"); hint.textContent = message; details.append(hint); }
  return details;
}
