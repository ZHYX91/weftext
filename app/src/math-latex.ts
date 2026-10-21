/** A deliberately bounded formula importer. It never expands TeX or executes input. */
export const LATEX_MATH_SUPPORT = Object.freeze([
  "分数、二项式、平方根及 n 次根",
  "上下标、积分、求和、乘积、极限及常用希腊字母／运算符",
  "成对括号及 left/right 可伸缩括号（不支持不可见边界）",
  "matrix、pmatrix、bmatrix、Bmatrix、vmatrix、Vmatrix 与 cases 环境",
  "text／operatorname 中的普通文字（含中文）及有限字面转义",
  "不支持完整文档、宏定义、宏包、外部文件、未知命令及任意 TeX 执行",
]);

export interface LatexMathDiagnostic { code: string; message: string; /** UTF-16 offsets into original, end exclusive. */ start: number; end: number }
interface LatexMathBasis { original: string; display: boolean; support: readonly string[]; diagnostics: LatexMathDiagnostic[] }
export type LatexMathConversion = (LatexMathBasis & { ok: true; body: string; typst: string }) | (LatexMathBasis & { ok: false; body: null; typst: null });
export interface LatexMathOptions { display?: boolean }

const MAX_INPUT = 16_384, MAX_DEPTH = 48, MAX_ATOMS = 4096, MAX_CELLS = 256;
const symbols: Readonly<Record<string, string>> = Object.freeze({
  alpha: "α", beta: "β", gamma: "γ", delta: "δ", epsilon: "ϵ", varepsilon: "ε", zeta: "ζ", eta: "η", theta: "θ", vartheta: "ϑ", iota: "ι", kappa: "κ", lambda: "λ", mu: "μ", nu: "ν", xi: "ξ", pi: "π", varpi: "ϖ", rho: "ρ", varrho: "ϱ", sigma: "σ", varsigma: "ς", tau: "τ", upsilon: "υ", phi: "ϕ", varphi: "φ", chi: "χ", psi: "ψ", omega: "ω",
  Gamma: "Γ", Delta: "Δ", Theta: "Θ", Lambda: "Λ", Xi: "Ξ", Pi: "Π", Sigma: "Σ", Upsilon: "Υ", Phi: "Φ", Psi: "Ψ", Omega: "Ω",
  times: "×", cdot: "⋅", div: "÷", pm: "±", mp: "∓", le: "≤", leq: "≤", ge: "≥", geq: "≥", ne: "≠", neq: "≠", approx: "≈", sim: "∼", simeq: "≃", equiv: "≡", propto: "∝", ll: "≪", gg: "≫", in: "∈", notin: "∉", ni: "∋", subset: "⊂", supset: "⊃", subseteq: "⊆", supseteq: "⊇", cup: "∪", cap: "∩", setminus: "∖", emptyset: "∅", forall: "∀", exists: "∃", neg: "¬", land: "∧", lor: "∨", wedge: "∧", vee: "∨", partial: "∂", nabla: "∇", infty: "∞", to: "→", rightarrow: "→", leftarrow: "←", leftrightarrow: "↔", Rightarrow: "⇒", Leftarrow: "⇐", Leftrightarrow: "⇔", mapsto: "↦", implies: "⇒", iff: "⇔", perp: "⊥", parallel: "∥", degree: "°", angle: "∠", ldots: "…", cdots: "⋯", vdots: "⋮", ddots: "⋱", ell: "ℓ", hbar: "ℏ", Re: "ℜ", Im: "ℑ",
  // Native operators retain their limit/script behavior.
  int: "integral", iint: "integral.double", iiint: "integral.triple", oint: "integral.cont", sum: "sum", prod: "product", lim: "lim", min: "min", max: "max", sin: "sin", cos: "cos", tan: "tan", arcsin: "arcsin", arccos: "arccos", arctan: "arctan", sinh: "sinh", cosh: "cosh", tanh: "tanh", log: "log", ln: "ln", exp: "exp", det: "det", gcd: "gcd",
});
const escaped: Readonly<Record<string, string>> = Object.freeze({ "{": "{", "}": "}", "%": "%", _: "_", "&": "&", "#": "#", $: "$", "|": "‖", " ": " " });
const forbidden = new Set(["def", "gdef", "edef", "xdef", "let", "futurelet", "newcommand", "renewcommand", "providecommand", "DeclareMathOperator", "input", "include", "includegraphics", "usepackage", "documentclass", "write", "write18", "openin", "openout", "read", "catcode", "csname", "endcsname", "special", "href", "url", "html", "require"]);
const environments = new Set(["matrix", "pmatrix", "bmatrix", "Bmatrix", "vmatrix", "Vmatrix", "cases"]);
const quote = (text: string): string => JSON.stringify(text);
class ConversionFailure extends Error { constructor(readonly diagnostic: LatexMathDiagnostic) { super(diagnostic.message); } }

class FormulaParser {
  position = 0;
  private depth = 0;
  private atoms = 0;
  private cells = 0;
  constructor(private readonly source: string, private readonly offset: number) {}
  fail(code: string, message: string, start = this.position, end = Math.min(this.source.length, start + 1)): never { throw new ConversionFailure({ code, message, start: this.offset + start, end: this.offset + end }); }
  private bounded<T>(run: () => T): T {
    if (++this.depth > MAX_DEPTH) this.fail("depth-limit", `公式嵌套超过 ${MAX_DEPTH} 层；原输入已保留。`);
    try { return run(); } finally { this.depth--; }
  }
  private whitespace(): void {
    while (this.position < this.source.length) {
      if (/\s/u.test(this.source[this.position])) this.position++;
      else if (this.source[this.position] === "%") { while (this.position < this.source.length && this.source[this.position] !== "\n") this.position++; }
      else break;
    }
  }
  private commandAhead(command: string): boolean { return this.source.startsWith(`\\${command}`, this.position) && !/[A-Za-z]/.test(this.source[this.position + command.length + 1] ?? ""); }
  private readCommand(): { name: string; start: number } {
    const start = this.position++;
    if (this.position === this.source.length) this.fail("unfinished-command", "反斜杠后缺少命令或转义字符。", start);
    const match = /^[A-Za-z]+/.exec(this.source.slice(this.position));
    const name = match ? match[0] : this.source[this.position]; this.position += name.length;
    return { name, start };
  }
  private sequence(stop: () => boolean = () => false): string {
    const result: string[] = [];
    for (;;) {
      this.whitespace(); if (this.position === this.source.length || stop()) return result.join(" ");
      let base = this.atom();
      this.whitespace();
      if (this.commandAhead("limits") || this.commandAhead("nolimits")) { const { name } = this.readCommand(); base = `${name === "limits" ? "limits" : "scripts"}(${base})`; this.whitespace(); }
      let top: string | undefined, bottom: string | undefined;
      while (this.source[this.position] === "^" || this.source[this.position] === "_") {
        const start = this.position, marker = this.source[this.position++];
        if (marker === "^" ? top !== undefined : bottom !== undefined) this.fail("duplicate-script", `同一对象有重复${marker === "^" ? "上" : "下"}标；请用花括号明确分组。`, start);
        const value = this.argument(); if (marker === "^") top = value; else bottom = value;
        this.whitespace();
      }
      if (top !== undefined || bottom !== undefined) base = `attach(${base}${top !== undefined ? `, t: ${top}` : ""}${bottom !== undefined ? `, b: ${bottom}` : ""})`;
      result.push(base);
    }
  }
  private group(): string {
    return this.bounded(() => {
      const start = this.position++;
      const value = this.sequence(() => this.source[this.position] === "}");
      if (this.source[this.position] !== "}") this.fail("unclosed-group", "花括号分组缺少右括号。", start, this.source.length);
      this.position++; return value || '""';
    });
  }
  private argument(): string {
    this.whitespace();
    if (this.position === this.source.length || "}^_&".includes(this.source[this.position]) || this.commandAhead("end") || this.source.startsWith("\\\\", this.position)) this.fail("missing-argument", "命令或上下标缺少参数；请用花括号包住公式。", this.position);
    return this.source[this.position] === "{" ? this.group() : this.bounded(() => this.atom(true));
  }
  private literalGroup(command: string): string {
    this.whitespace(); if (this.source[this.position] !== "{") this.fail("group-required", `\\${command} 的文字须放在花括号内。`);
    const start = this.position++;
    let text = "", nesting = 1;
    while (this.position < this.source.length) {
      const character = this.source[this.position++];
      if (character === "{") { if (++nesting > MAX_DEPTH) this.fail("depth-limit", `文字分组超过 ${MAX_DEPTH} 层。`, start, this.position); }
      else if (character === "}") { if (--nesting === 0) return text; }
      else if (character === "\\") {
        this.position--; const token = this.readCommand();
        if (Object.hasOwn(escaped, token.name)) text += escaped[token.name];
        else if (token.name === "textbackslash") text += "\\";
        else this.fail(forbidden.has(token.name) ? "forbidden-command" : "unsupported-text-command", `\\${command} 中不执行命令 \\${token.name}；请粘贴普通文字。`, token.start, this.position);
      } else if (character === "%") { while (this.position < this.source.length && this.source[this.position] !== "\n") this.position++; if (this.source[this.position] === "\n") this.position++; }
      else if ("$^_&#".includes(character)) this.fail("unsupported-text-structure", `\\${command} 中的 ${character} 须明确转义；文字中的嵌套数学暂不可转换。`, this.position - 1, this.position);
      else if (/\s/u.test(character)) { if (!text.endsWith(" ")) text += " "; }
      else text += character === "~" ? "\u00a0" : character;
    }
    return this.fail("unclosed-group", `\\${command} 的文字缺少右花括号。`, start, this.source.length);
  }
  private delimiter(): string {
    this.whitespace(); const start = this.position;
    if (this.source[this.position] === "\\") {
      const token = this.readCommand();
      const map: Record<string, string> = { "{": "{", "}": "}", "|": "‖", langle: "⟨", rangle: "⟩", lbrace: "{", rbrace: "}", lvert: "|", rvert: "|", vert: "|", lVert: "‖", rVert: "‖", Vert: "‖", lfloor: "⌊", rfloor: "⌋", lceil: "⌈", rceil: "⌉" };
      if (Object.hasOwn(map, token.name)) return map[token.name];
    } else if ("()[]|".includes(this.source[this.position] ?? "") && this.position < this.source.length) return this.source[this.position++];
    return this.fail("unsupported-delimiter", "仅支持可见的圆／方／花括号、竖线、角括号及上下取整边界；不可见边界暂不可转换。", start, this.position || start + 1);
  }
  private paired(): string {
    return this.bounded(() => {
      const start = this.position, left = this.delimiter();
      const content = this.sequence(() => this.commandAhead("right"));
      if (!this.commandAhead("right")) this.fail("unclosed-delimiter", "\\left 缺少对应的 \\right。", start, this.position);
      this.readCommand(); const right = this.delimiter();
      return `lr(${left} ${content} ${right})`;
    });
  }
  private environment(start: number): string {
    const name = this.literalGroup("begin");
    if (!environments.has(name)) this.fail("unsupported-environment", `环境 ${name} 尚不支持；首轮支持矩阵和 cases 分段函数。`, start, this.position);
    return this.bounded(() => {
      const rows: string[][] = [[]];
      for (;;) {
        if (++this.cells > MAX_CELLS) this.fail("cell-limit", `矩阵／分段单元超过 ${MAX_CELLS} 个。`, start, this.position);
        const row = rows[rows.length - 1];
        row.push(this.sequence(() => this.source[this.position] === "&" || this.source.startsWith("\\\\", this.position) || this.commandAhead("end")) || '""');
        if (this.source[this.position] === "&") { this.position++; continue; }
        if (this.source.startsWith("\\\\", this.position)) {
          this.position += 2; this.whitespace();
          if (this.source[this.position] === "[") this.fail("unsupported-row-spacing", "矩阵的自定义行距暂不可转换。", this.position);
          if (!this.commandAhead("end")) { rows.push([]); continue; }
        }
        if (!this.commandAhead("end")) this.fail("unclosed-environment", `环境 ${name} 缺少 \\end{${name}}。`, start, this.position);
        const ending = this.readCommand(), closing = this.literalGroup("end");
        if (closing !== name) this.fail("mismatched-environment", `环境 ${name} 不能以 ${closing} 结束。`, ending.start, this.position);
        if (name === "cases") {
          if (rows.some(items => items.length > 2)) this.fail("unsupported-cases", "cases 每行最多包含表达式与条件两列。", start, this.position);
          return `cases(${rows.map(items => items.join(" & ")).join(", ")})`;
        }
        if (rows.some(items => items.length !== rows[0].length)) this.fail("ragged-matrix", "矩阵各行列数不一致；请补齐空单元格。", start, this.position);
        const delimiter: Record<string, string> = { matrix: "#none", pmatrix: quote("("), bmatrix: quote("["), Bmatrix: quote("{"), vmatrix: quote("|"), Vmatrix: quote("‖") };
        return `mat(delim: ${delimiter[name]}, ${rows.map(items => items.join(", ")).join("; ")})`;
      }
    });
  }
  private atom(singleCharacter = false): string {
    if (++this.atoms > MAX_ATOMS) this.fail("atom-limit", `公式超过 ${MAX_ATOMS} 个元素。`);
    const start = this.position, character = this.source[this.position];
    if (character === "{") return this.group();
    if (character === "\\") {
      const { name } = this.readCommand();
      if (forbidden.has(name)) this.fail("forbidden-command", `不执行宏、外部输入或文档命令 \\${name}；原输入已保留。`, start, this.position);
      if (Object.hasOwn(symbols, name)) return symbols[name];
      if (Object.hasOwn(escaped, name)) return quote(escaped[name]);
      if (name === "frac" || name === "dfrac" || name === "tfrac" || name === "binom") {
        const numerator = this.argument(), denominator = this.argument();
        const result = `${name === "binom" ? "binom" : "frac"}(${numerator}, ${denominator})`;
        return name === "dfrac" ? `display(${result})` : name === "tfrac" ? `inline(${result})` : result;
      }
      if (name === "sqrt") {
        this.whitespace(); let index: string | undefined;
        if (this.source[this.position] === "[") {
          const begin = this.position++; index = this.bounded(() => this.sequence(() => this.source[this.position] === "]"));
          if (this.source[this.position] !== "]" || !index) this.fail("invalid-root-index", "根指数缺少内容或右方括号。", begin, this.position);
          this.position++;
        }
        const radicand = this.argument(); return index === undefined ? `sqrt(${radicand})` : `root(${index}, ${radicand})`;
      }
      if (name === "text" || name === "operatorname") { const text = this.literalGroup(name); return name === "text" ? quote(text) : `op(${quote(text)})`; }
      const styles: Record<string, string> = { mathrm: "upright", mathbf: "bold", mathit: "italic", mathbb: "bb", mathcal: "cal", overline: "overline", underline: "underline", hat: "hat", bar: "bar", vec: "arrow", dot: "dot", ddot: "dot.double" };
      if (Object.hasOwn(styles, name)) {
        // Accents use fixed Unicode accents instead of user-provided function names.
        const content = this.argument(), accents: Record<string, string> = { hat: "^", bar: "¯", vec: "→", dot: "˙", ddot: "¨" };
        return Object.hasOwn(accents, name) ? `accent(${content}, ${quote(accents[name])})` : `${styles[name]}(${content})`;
      }
      if (name === "left") return this.paired();
      if (name === "begin") return this.environment(start);
      if (name === "," || name === ":" || name === ";" || name === "!" || name === "quad" || name === "qquad") {
        const widths: Record<string, string> = { ",": "0.1667", ":": "0.2222", ";": "0.2778", "!": "-0.1667", quad: "1", qquad: "2" };
        return `#h(${widths[name]}em)`;
      }
      if (name === "right" || name === "end" || name === "\\") this.fail("unexpected-boundary", `意外出现 \\${name}；请检查括号或环境。`, start, this.position);
      this.fail("unknown-command", `未知或尚不支持的命令 \\${name}；不猜测宏的含义，原输入已保留。`, start, this.position);
    }
    if (character === "}" || character === "^" || character === "_" || character === "&") this.fail("unexpected-token", `意外出现 ${character}；请检查分组、上下标或矩阵环境。`, start);
    if (character === "$" || character === "#" || character === '"' || character === "`") this.fail("unsupported-token", `公式中的 ${character} 必须通过明确的文字或转义输入。`, start);
    const point = String.fromCodePoint(this.source.codePointAt(this.position)!);
    if (/\p{Script=Han}/u.test(point)) this.fail("text-required", "中文文字请放入 \\text{…} 中，以保留文字语义。", start, start + point.length);
    this.position += point.length;
    if (!singleCharacter && /[0-9]/.test(character)) {
      const rest = /^[0-9]*(?:\.[0-9]+)?/.exec(this.source.slice(this.position))![0]; this.position += rest.length; return character + rest;
    }
    if (character === "~") return '"\u00a0"';
    // Quote math argument separators and literal slash: Typst's unquoted / means a stacked fraction.
    if (",;:/()[]".includes(character)) return quote(character);
    if (character === "*") return "∗";
    if (!/[\p{L}\p{N}\p{Sm}\p{Po}]/u.test(point) && !"+-=<>!|.".includes(point)) this.fail("unsupported-character", `字符 ${point} 的数学含义尚未覆盖。`, start, this.position);
    return point;
  }
  parse(): string { return this.sequence(); }
}

/** Conversion has no network, file, eval, macro, or full TeX runtime dependency. */
export function convertLatexMath(original: string, options: LatexMathOptions = {}): LatexMathConversion {
  let begin = original.length - original.trimStart().length, end = original.trimEnd().length, display = options.display ?? false;
  const failure = (diagnostic: LatexMathDiagnostic): LatexMathConversion => ({ ok: false, original, display, body: null, typst: null, diagnostics: [diagnostic], support: LATEX_MATH_SUPPORT });
  if (original.length > MAX_INPUT) return failure({ code: "input-limit", message: `输入超过 ${MAX_INPUT} 个 UTF-16 字符；原输入已保留。`, start: MAX_INPUT, end: original.length });
  const controls = /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f-\u009f]/u.exec(original);
  if (controls) return failure({ code: "control-character", message: "输入含不支持的控制字符；原输入已保留。", start: controls.index, end: controls.index + controls[0].length });
  const invalidUnicode = /[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/u.exec(original);
  if (invalidUnicode) return failure({ code: "invalid-unicode", message: "输入含不完整的 Unicode 字符；原输入已保留。", start: invalidUnicode.index, end: invalidUnicode.index + 1 });
  const value = original.slice(begin, end);
  const wrappers: [string, string, boolean][] = [["$$", "$$", true], ["\\[", "\\]", true], ["\\(", "\\)", false], ["$", "$", false]];
  for (const [left, right, mode] of wrappers) {
    if (!value.startsWith(left)) continue;
    if (!value.endsWith(right) || value.length < left.length + right.length) return failure({ code: "unclosed-formula", message: "公式的外层分隔符没有闭合。", start: begin, end });
    begin += left.length; end -= right.length; display = options.display ?? mode; break;
  }
  try {
    const body = new FormulaParser(original.slice(begin, end), begin).parse();
    if (!body) return failure({ code: "empty-formula", message: "请输入需要转换的 LaTeX 公式。", start: begin, end: Math.max(begin, end) });
    return { ok: true, original, display, body, typst: display ? `$ ${body} $` : `$${body}$`, diagnostics: [], support: LATEX_MATH_SUPPORT };
  } catch (error) { if (error instanceof ConversionFailure) return failure(error.diagnostic); throw error; }
}

export type ConvertedLatexMath = Extract<LatexMathConversion, { ok: true }>;
export interface LatexMathPanelHost {
  authorInputs?: import("./author-input").AuthorInputSession;
  /** Insert through the existing editor Draft/Undo command; do not mutate a second formula authority. */
  apply(result: ConvertedLatexMath): void | Promise<void>;
  /** Optional real Typst rendering. Return only a trusted local node or plain text; input is never HTML. */
  preview?(result: ConvertedLatexMath): HTMLElement | string | Promise<HTMLElement | string>;
  cancel?(): void;
}
export interface LatexMathPanel { element: HTMLElement; setInput(value: string, display?: boolean): void; destroy(): void }
let panelNumber = 0;

/** Mountable explicit conversion → preview → confirm flow. Closing/cancelling never calls apply. */
export function createLatexMathPanel(host: LatexMathPanelHost): LatexMathPanel {
  const make = <K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text = ""): HTMLElementTagNameMap[K] => { const item = document.createElement(tag); item.className = className; item.textContent = text; return item; };
  const element = make("section", "data-panel latex-math-panel"), heading = make("h3", "", "LaTeX 公式输入");
  const id = `latex-math-${++panelNumber}`, input = make("textarea", "data-input"), label = make("label", "data-label", "原始 LaTeX（转换或取消均保留）");
  input.id = `${id}-input`; input.rows = 6; input.maxLength = MAX_INPUT; input.spellcheck = false; label.htmlFor = input.id;
  const mode = make("select", "data-input"), modeLabel = make("label", "data-label", "插入位置"); mode.id = `${id}-mode`; modeLabel.htmlFor = mode.id;
  for (const [value, text] of [["inline", "行内公式"], ["display", "独立公式"]]) { const item = make("option", "", text); item.value = value; mode.append(item); }
  const convert = make("button", "data-button", "转换并预览"), confirm = make("button", "data-button primary", "确认插入原生公式"), cancel = make("button", "data-button", "取消本次插入");
  for (const button of [convert, confirm, cancel]) button.type = "button";
  const status = make("p", "data-help", "插入后编辑原生 Typst 公式；首轮不支持完整 LaTeX 文档与任意宏。"), source = make("pre", "output-source"), rendered = make("div", "output-preview"), buttons = make("div", "output-actions");
  status.setAttribute("role", "status"); status.setAttribute("aria-live", "polite");
  const support = make("details", "output-detail"), list = make("ul"); support.append(make("summary", "", "转换支持范围")); LATEX_MATH_SUPPORT.forEach(value => list.append(make("li", "", value))); support.append(list);
  buttons.append(convert, confirm, cancel); element.append(heading, label, input, modeLabel, mode, buttons, status, source, rendered, support);
  let generation = 0, current: ConvertedLatexMath | null = null, disposed = false, applying = false;
  confirm.disabled = true;
  const invalidate = (): void => { generation++; current = null; confirm.disabled = true; source.textContent = ""; rendered.replaceChildren(); status.textContent = "输入已变化，请重新转换并查看预览。"; };
  host.authorInputs?.bind(input, "latex"); input.oninput = invalidate; mode.onchange = invalidate;
  convert.onclick = () => { void (async () => {
    if (applying || disposed) return;
    const request = ++generation, result = convertLatexMath(input.value, { display: mode.value === "display" });
    current = null; confirm.disabled = true; rendered.replaceChildren(); source.textContent = result.typst ?? "";
    if (!result.ok) { status.textContent = result.diagnostics.map(item => `${item.message}（字符 ${item.start + 1}–${Math.max(item.start + 1, item.end)}）`).join("\n"); return; }
    status.textContent = host.preview ? "正在生成本次原生公式预览…" : "已生成下方原生 Typst 源码预览；确认后进入当前草稿与撤销。";
    try {
      if (host.preview) { const view = await host.preview(result); if (request !== generation || disposed) return; if (typeof view === "string") rendered.textContent = view; else rendered.append(view); }
      if (request !== generation || disposed) return;
      current = result; confirm.disabled = false; status.textContent = host.preview ? "已生成本次预览；确认后插入当前草稿，可撤销。" : status.textContent;
    } catch (error) { if (request === generation && !disposed) status.textContent = `预览失败：${error instanceof Error ? error.message : String(error)}。原输入已保留。`; }
  })(); };
  confirm.onclick = () => { void (async () => {
    if (!current || applying || disposed) return;
    const result = current; applying = true; input.readOnly = true; mode.disabled = true; convert.disabled = true; cancel.disabled = true; confirm.disabled = true;
    try { await host.apply(result); if (!disposed) { current = null; status.textContent = "已插入原生 Typst 公式，可在当前编辑器撤销。原始 LaTeX 保留在此输入框。"; } }
    catch (error) { if (!disposed) { status.textContent = `插入失败：${error instanceof Error ? error.message : String(error)}。原输入已保留。`; confirm.disabled = false; } }
    finally { applying = false; if (!disposed) { input.readOnly = false; mode.disabled = false; convert.disabled = false; cancel.disabled = false; } }
  })(); };
  cancel.onclick = () => { if (!applying) { invalidate(); status.textContent = "已取消本次插入；原始 LaTeX 已保留，草稿未改动。"; host.cancel?.(); } };
  return { element, setInput(value, display = false) { if (applying) throw new Error("正在插入公式，不能替换本次输入。"); input.value = value; mode.value = display ? "display" : "inline"; invalidate(); }, destroy() { disposed = true; host.authorInputs?.retire(); generation++; input.oninput = null; mode.onchange = null; convert.onclick = confirm.onclick = cancel.onclick = null; element.remove(); } };
}
