import assert from "node:assert/strict";
import test from "node:test";
import { convertLatexMath, createLatexMathPanel, LATEX_MATH_SUPPORT, type LatexMathPanelHost } from "./math-latex";

function body(input: string): string { const result = convertLatexMath(input); assert.equal(result.ok, true, JSON.stringify(result.diagnostics)); assert.ok(result.ok); return result.body; }

test("fractions, roots and both scripts become native editable formula structures", () => {
  assert.equal(body(String.raw`\frac{x_i^2+1}{\sqrt[3]{y}}`), "frac(attach(x, t: 2, b: i) + 1, root(3, y))");
  assert.equal(body(String.raw`\frac12+x^12`), "frac(1, 2) + attach(x, t: 1) 2");
  assert.equal(body(String.raw`{a+b}^2`), "attach(a + b, t: 2)");
  assert.equal(body(String.raw`\int_0^1 x\,dx + \sum_{i=1}^{n} i`), "attach(integral, t: 1, b: 0) x #h(0.1667em) d x + attach(sum, t: n, b: i = 1) i");
});

test("Unicode variables, common operators and Latin variables cannot become code identifiers", () => {
  assert.equal(body(String.raw`\alpha\leq \beta\times x + abc + 12.34`), "α ≤ β × x + a b c + 12.34");
  assert.equal(body(String.raw`x/y, z; w: q`), 'x "/" y "," z ";" w ":" q');
  assert.equal(body(String.raw`\sin x+\operatorname{自定义}x`), 'sin x + op("自定义") x');
});

test("Chinese text and escaped metacharacters remain literal native strings", () => {
  assert.equal(body(String.raw`\text{总价} = \text{单价}\times\text{数量}`), '"总价" = "单价" × "数量"');
  const result = body(String.raw`\text{甲\#乙\$丙\_丁\{戊\}\textbackslash}`);
  assert.equal(result, JSON.stringify("甲#乙$丙_丁{戊}\\"));
  assert.equal(body(String.raw`\text{a"b}`), '"a\\"b"');
  assert.equal(convertLatexMath("总价=x").diagnostics[0].code, "text-required");
  assert.equal(convertLatexMath("𠀀=x").diagnostics[0].code, "text-required");
});

test("matrix delimiters, cells and cases preserve independent native structures", () => {
  assert.equal(body(String.raw`\begin{pmatrix}1&2\\3&4\end{pmatrix}`), 'mat(delim: "(", 1, 2; 3, 4)');
  assert.equal(body(String.raw`\begin{Vmatrix}1&2\\3&4\end{Vmatrix}`), 'mat(delim: "‖", 1, 2; 3, 4)');
  assert.equal(body(String.raw`\begin{matrix}a&\frac12\\&b\\\end{matrix}`), 'mat(delim: #none, a, frac(1, 2); "", b)');
  assert.equal(body(String.raw`f(x)=\begin{cases}x^2 & x\ge0\\-x & \text{其他}\end{cases}`), 'f "(" x ")" = cases(attach(x, t: 2) & x ≥ 0, - x & "其他")');
  assert.equal(body(String.raw`\left(\frac{a}{b}\right)^2`), "attach(lr(( frac(a, b) )), t: 2)");
  assert.equal(body(String.raw`\left\{x\right\}`), "lr({ x })");
  assert.equal(body(String.raw`\left\|x\right\|`), "lr(‖ x ‖)");
});

test("outer math delimiters determine mode while explicit mode wins and exact original survives", () => {
  const input = " \n$$\\frac{1}{2}$$\t";
  const result = convertLatexMath(input);
  assert.equal(result.original, input); assert.equal(result.display, true); assert.equal(result.typst, "$ frac(1, 2) $");
  assert.equal(convertLatexMath(input, { display: false }).typst, "$frac(1, 2)$");
  assert.equal(convertLatexMath(String.raw`\(x\)`).display, false);
  assert.equal(convertLatexMath(String.raw`\[x\]`).display, true);
  assert.equal(convertLatexMath("$x").diagnostics[0].code, "unclosed-formula");
});

test("unsupported input yields exact original offsets and never a partial insertable formula", () => {
  const original = String.raw`  $$x + \unknown{甲}$$  `, result = convertLatexMath(original);
  assert.equal(result.ok, false); assert.equal(result.original, original); assert.equal(result.typst, null); assert.equal(result.body, null);
  assert.equal(original.slice(result.diagnostics[0].start, result.diagnostics[0].end), String.raw`\unknown`);
  assert.deepEqual(result.support, LATEX_MATH_SUPPORT);
});

test("macros, document commands, arbitrary input and escaped code attempts are rejected", () => {
  for (const input of [String.raw`\def\x{1}\x`, String.raw`\newcommand{\x}{2}`, String.raw`\input{secret}`, String.raw`\includegraphics{secret}`, String.raw`\write18{cmd}`, String.raw`\documentclass{article}`, String.raw`\text{\input{secret}}`, String.raw`#read("secret")`, String.raw`\href{https://example.test}{x}`]) {
    const result = convertLatexMath(input); assert.equal(result.ok, false, input); assert.equal(result.typst, null); assert.equal(result.original, input);
  }
  assert.equal(body(String.raw`\text{\#read("secret")}`), JSON.stringify('#read("secret")'));
});

test("malformed and unsupported structures fail explicitly", () => {
  const cases: [string, string][] = [
    [String.raw`\frac{1}`, "missing-argument"], [String.raw`x^{2`, "unclosed-group"], [String.raw`x^2^3`, "duplicate-script"],
    [String.raw`\left(x`, "unclosed-delimiter"], [String.raw`\left.x\right|`, "unsupported-delimiter"], [String.raw`\sqrt[]{x}`, "invalid-root-index"],
    [String.raw`\begin{array}{cc}a&b\end{array}`, "unsupported-environment"], [String.raw`\begin{matrix}a&b\\c\end{matrix}`, "ragged-matrix"],
    [String.raw`\begin{matrix}x\end{pmatrix}`, "mismatched-environment"], [String.raw`\begin{matrix}x`, "unclosed-environment"],
    [String.raw`\begin{cases}a&b&c\end{cases}`, "unsupported-cases"], [String.raw`\begin{matrix}x\\[2pt]y\end{matrix}`, "unsupported-row-spacing"],
    [String.raw`\text{\bf X}`, "unsupported-text-command"], [String.raw`x&y`, "unexpected-token"], [String.raw`\right)`, "unexpected-boundary"],
    [String.raw`\text{a$x$b}`, "unsupported-text-structure"],
  ];
  for (const [input, code] of cases) { const result = convertLatexMath(input); assert.equal(result.ok, false, input); assert.equal(result.diagnostics[0].code, code, input); }
});

test("input, nesting, atom, cell and control-character bounds preserve supplied input", () => {
  for (const [input, code] of [
    ["x".repeat(16_385), "input-limit"], ["{".repeat(49) + "x" + "}".repeat(49), "depth-limit"],
    [String.raw`\sqrt`.repeat(2000) + "x", "depth-limit"],
    ["x".repeat(4097), "atom-limit"], [String.raw`\begin{matrix}` + Array(257).fill("1").join("&") + String.raw`\end{matrix}`, "cell-limit"],
    ["x\u0000", "control-character"], ["\\text{\uD800}", "invalid-unicode"], [" \n\t", "empty-formula"],
  ]) { const result = convertLatexMath(input); assert.equal(result.ok, false, code); assert.equal(result.original, input); assert.equal(result.diagnostics[0].code, code); }
});

test("comments follow TeX newline semantics and cannot smuggle commands", () => {
  assert.equal(body("x% ignored \\input{private}\n+y"), "x + y");
  assert.equal(body(String.raw`\text{a% omitted` + "\nb}"), '"ab"');
  assert.equal(body("\\text{a\n \tb}"), '"a b"');
  assert.equal(body(String.raw`\%`), '"%"');
});

// Minimal DOM harness exercises event/race behavior without a browser or an extra runtime package.
class ElementStub {
  readonly children: ElementStub[] = [];
  textContent = ""; className = ""; id = ""; htmlFor = ""; value = ""; disabled = false; readOnly = false;
  onclick: (() => void) | null = null; oninput: (() => void) | null = null; onchange: (() => void) | null = null;
  constructor(readonly tag: string) {}
  append(...nodes: ElementStub[]): void { this.children.push(...nodes); }
  replaceChildren(...nodes: ElementStub[]): void { this.children.splice(0, this.children.length, ...nodes); }
  setAttribute(): void { /* Attribute rendering is outside this event-only harness. */ }
  remove(): void { /* Ownership removal does not affect the asynchronous generation guard. */ }
  find(tag: string, text?: string): ElementStub { const found = [this, ...this.descendants()].find(item => item.tag === tag && (text === undefined || item.textContent === text)); assert.ok(found, `${tag} ${text ?? ""}`); return found; }
  private descendants(): ElementStub[] { return this.children.flatMap(child => [child, ...child.descendants()]); }
}
async function panelFixture(run: (panel: ReturnType<typeof createLatexMathPanel>, root: ElementStub) => Promise<void>, host: LatexMathPanelHost): Promise<void> {
  const previous = Object.getOwnPropertyDescriptor(globalThis, "document");
  Object.defineProperty(globalThis, "document", { configurable: true, value: { createElement: (tag: string) => new ElementStub(tag) } });
  const panel = createLatexMathPanel(host);
  try { await run(panel, panel.element as unknown as ElementStub); }
  finally { panel.destroy(); if (previous) Object.defineProperty(globalThis, "document", previous); else Reflect.deleteProperty(globalThis, "document"); }
}
const settle = (): Promise<void> => new Promise(resolve => setImmediate(resolve));

test("panel requires conversion and confirmation, and cancellation retains original without applying", async () => {
  const applied: string[] = [];
  await panelFixture(async (panel, root) => {
    panel.setInput(String.raw`\frac12`);
    const confirm = root.find("button", "确认插入原生公式"), convert = root.find("button", "转换并预览");
    assert.equal(confirm.disabled, true); convert.onclick!(); await settle(); assert.equal(confirm.disabled, false); assert.deepEqual(applied, []);
    root.find("button", "取消本次插入").onclick!(); assert.equal(confirm.disabled, true); assert.equal(root.find("textarea").value, String.raw`\frac12`); assert.deepEqual(applied, []);
    convert.onclick!(); await settle(); confirm.onclick!(); confirm.onclick!(); await settle(); assert.deepEqual(applied, ["$frac(1, 2)$"]); assert.equal(confirm.disabled, true);
  }, { apply(result) { applied.push(result.typst); } });
});

test("late preview cannot confirm changed input, and a failed preview retains its original", async () => {
  let resolvePreview!: (value: string) => void, calls = 0;
  await panelFixture(async (panel, root) => {
    panel.setInput("x"); const convert = root.find("button", "转换并预览"), confirm = root.find("button", "确认插入原生公式");
    convert.onclick!(); panel.setInput("y"); resolvePreview("stale x preview"); await settle(); assert.equal(confirm.disabled, true); assert.equal(root.find("textarea").value, "y");
    convert.onclick!(); await settle(); assert.equal(confirm.disabled, true); assert.match(root.find("p").textContent, /预览失败/); assert.equal(root.find("textarea").value, "y");
  }, { apply() { assert.fail("Changed or failed previews must not insert"); }, preview() { if (++calls === 1) return new Promise(resolve => { resolvePreview = resolve; }); throw new Error("render unavailable"); } });
});

test("failed Draft application can retry exact confirmed source while pending calls cannot duplicate", async () => {
  let rejectApply!: (error: Error) => void, calls = 0;
  await panelFixture(async (panel, root) => {
    panel.setInput("x", true); root.find("button", "转换并预览").onclick!(); await settle(); const confirm = root.find("button", "确认插入原生公式");
    confirm.onclick!(); confirm.onclick!(); assert.equal(calls, 1); assert.equal(root.find("textarea").readOnly, true);
    rejectApply(new Error("draft changed")); await settle(); assert.equal(confirm.disabled, false); assert.match(root.find("p").textContent, /draft changed/); assert.equal(root.find("textarea").value, "x");
    confirm.onclick!(); await settle(); assert.equal(calls, 2); assert.equal(confirm.disabled, true);
  }, { apply(result) { assert.equal(result.typst, "$ x $"); return ++calls === 1 ? new Promise<void>((_resolve, reject) => { rejectApply = reject; }) : undefined; } });
});
