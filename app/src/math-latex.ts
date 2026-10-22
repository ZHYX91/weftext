import { joinUiText, uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
/** A deliberately bounded formula importer. It never expands TeX or executes input. */
export const LATEX_MATH_SUPPORT = Object.freeze([
    uiMessage("mathLatex.supportFractionsRoots"),
    uiMessage("mathLatex.supportOperators"),
    uiMessage("mathLatex.supportDelimiters"),
    uiMessage("mathLatex.supportEnvironments"),
    uiMessage("mathLatex.supportText"),
    uiMessage("mathLatex.unsupportedExecution"),
]);
export interface LatexMathDiagnostic {
    code: string;
    message: UiText; /** UTF-16 offsets into original, end exclusive. */
    start: number;
    end: number;
}
interface LatexMathBasis {
    original: string;
    display: boolean;
    support: readonly UiText[];
    diagnostics: LatexMathDiagnostic[];
}
export type LatexMathConversion = (LatexMathBasis & {
    ok: true;
    body: string;
    typst: string;
}) | (LatexMathBasis & {
    ok: false;
    body: null;
    typst: null;
});
export interface LatexMathOptions {
    display?: boolean;
}
const MAX_INPUT = 16384, MAX_DEPTH = 48, MAX_ATOMS = 4096, MAX_CELLS = 256;
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
class ConversionFailure extends UiError {
    constructor(readonly diagnostic: LatexMathDiagnostic) { super(diagnostic.message); }
}
class FormulaParser {
    position = 0;
    private depth = 0;
    private atoms = 0;
    private cells = 0;
    constructor(private readonly source: string, private readonly offset: number) { }
    fail(code: string, message: UiText, start = this.position, end = Math.min(this.source.length, start + 1)): never { throw new ConversionFailure({ code, message, start: this.offset + start, end: this.offset + end }); }
    private bounded<T>(run: () => T): T {
        if (++this.depth > MAX_DEPTH)
            this.fail("depth-limit", displayMessage("mathLatex.depthLimit", { p0: MAX_DEPTH }));
        try {
            return run();
        }
        finally {
            this.depth--;
        }
    }
    private whitespace(): void {
        while (this.position < this.source.length) {
            if (/\s/u.test(this.source[this.position]))
                this.position++;
            else if (this.source[this.position] === "%") {
                while (this.position < this.source.length && this.source[this.position] !== "\n")
                    this.position++;
            }
            else
                break;
        }
    }
    private commandAhead(command: string): boolean { return this.source.startsWith(`\\${command}`, this.position) && !/[A-Za-z]/.test(this.source[this.position + command.length + 1] ?? ""); }
    private readCommand(): {
        name: string;
        start: number;
    } {
        const start = this.position++;
        if (this.position === this.source.length)
            this.fail("unfinished-command", uiMessage("mathLatex.commandAfterBackslashMissing"), start);
        const match = /^[A-Za-z]+/.exec(this.source.slice(this.position));
        const name = match ? match[0] : this.source[this.position];
        this.position += name.length;
        return { name, start };
    }
    private sequence(stop: () => boolean = () => false): string {
        const result: string[] = [];
        for (;;) {
            this.whitespace();
            if (this.position === this.source.length || stop())
                return result.join(" ");
            let base = this.atom();
            this.whitespace();
            if (this.commandAhead("limits") || this.commandAhead("nolimits")) {
                const { name } = this.readCommand();
                base = `${name === "limits" ? "limits" : "scripts"}(${base})`;
                this.whitespace();
            }
            let top: string | undefined, bottom: string | undefined;
            while (this.source[this.position] === "^" || this.source[this.position] === "_") {
                const start = this.position, marker = this.source[this.position++];
                if (marker === "^" ? top !== undefined : bottom !== undefined)
                    this.fail("duplicate-script", displayMessage("mathLatex.duplicateScript", { p0: uiDisplayParameter(marker === "^" ? uiMessage("mathLatex.superscriptPrefix") : uiMessage("mathLatex.subscriptPrefix")) }), start);
                const value = this.argument();
                if (marker === "^")
                    top = value;
                else
                    bottom = value;
                this.whitespace();
            }
            if (top !== undefined || bottom !== undefined)
                base = `attach(${base}${top !== undefined ? `, t: ${top}` : ""}${bottom !== undefined ? `, b: ${bottom}` : ""})`;
            result.push(base);
        }
    }
    private group(): string {
        return this.bounded(() => {
            const start = this.position++;
            const value = this.sequence(() => this.source[this.position] === "}");
            if (this.source[this.position] !== "}")
                this.fail("unclosed-group", uiMessage("mathLatex.groupClosingBraceMissing"), start, this.source.length);
            this.position++;
            return value || '""';
        });
    }
    private argument(): string {
        this.whitespace();
        if (this.position === this.source.length || "}^_&".includes(this.source[this.position]) || this.commandAhead("end") || this.source.startsWith("\\\\", this.position))
            this.fail("missing-argument", uiMessage("mathLatex.argumentMissing"), this.position);
        return this.source[this.position] === "{" ? this.group() : this.bounded(() => this.atom(true));
    }
    private literalGroup(command: string): string {
        this.whitespace();
        if (this.source[this.position] !== "{")
            this.fail("group-required", displayMessage("mathLatex.textBracesRequired", { p0: uiDisplayParameter(command) }));
        const start = this.position++;
        let text = "", nesting = 1;
        while (this.position < this.source.length) {
            const character = this.source[this.position++];
            if (character === "{") {
                if (++nesting > MAX_DEPTH)
                    this.fail("depth-limit", displayMessage("mathLatex.textDepthLimit", { p0: MAX_DEPTH }), start, this.position);
            }
            else if (character === "}") {
                if (--nesting === 0)
                    return text;
            }
            else if (character === "\\") {
                this.position--;
                const token = this.readCommand();
                if (Object.hasOwn(escaped, token.name))
                    text += escaped[token.name];
                else if (token.name === "textbackslash")
                    text += "\\";
                else
                    this.fail(forbidden.has(token.name) ? "forbidden-command" : "unsupported-text-command", displayMessage("mathLatex.textCommandNotExecuted", { p0: uiDisplayParameter(command), p1: uiDisplayParameter(token.name) }), token.start, this.position);
            }
            else if (character === "%") {
                while (this.position < this.source.length && this.source[this.position] !== "\n")
                    this.position++;
                if (this.source[this.position] === "\n")
                    this.position++;
            }
            else if ("$^_&#".includes(character))
                this.fail("unsupported-text-structure", displayMessage("mathLatex.textCharacterEscapeRequired", { p0: uiDisplayParameter(command), p1: uiDisplayParameter(character) }), this.position - 1, this.position);
            else if (/\s/u.test(character)) {
                if (!text.endsWith(" "))
                    text += " ";
            }
            else
                text += character === "~" ? "\u00a0" : character;
        }
        return this.fail("unclosed-group", displayMessage("mathLatex.textClosingBraceMissing", { p0: uiDisplayParameter(command) }), start, this.source.length);
    }
    private delimiter(): string {
        this.whitespace();
        const start = this.position;
        if (this.source[this.position] === "\\") {
            const token = this.readCommand();
            const map: Record<string, string> = { "{": "{", "}": "}", "|": "‖", langle: "⟨", rangle: "⟩", lbrace: "{", rbrace: "}", lvert: "|", rvert: "|", vert: "|", lVert: "‖", rVert: "‖", Vert: "‖", lfloor: "⌊", rfloor: "⌋", lceil: "⌈", rceil: "⌉" };
            if (Object.hasOwn(map, token.name))
                return map[token.name];
        }
        else if ("()[]|".includes(this.source[this.position] ?? "") && this.position < this.source.length)
            return this.source[this.position++];
        return this.fail("unsupported-delimiter", uiMessage("mathLatex.delimiterUnsupported"), start, this.position || start + 1);
    }
    private paired(): string {
        return this.bounded(() => {
            const start = this.position, left = this.delimiter();
            const content = this.sequence(() => this.commandAhead("right"));
            if (!this.commandAhead("right"))
                this.fail("unclosed-delimiter", uiMessage("mathLatex.rightDelimiterMissing"), start, this.position);
            this.readCommand();
            const right = this.delimiter();
            return `lr(${left} ${content} ${right})`;
        });
    }
    private environment(start: number): string {
        const name = this.literalGroup("begin");
        if (!environments.has(name))
            this.fail("unsupported-environment", displayMessage("mathLatex.environmentUnsupported", { p0: uiDisplayParameter(name) }), start, this.position);
        return this.bounded(() => {
            const rows: string[][] = [[]];
            for (;;) {
                if (++this.cells > MAX_CELLS)
                    this.fail("cell-limit", displayMessage("mathLatex.cellLimit", { p0: MAX_CELLS }), start, this.position);
                const row = rows[rows.length - 1];
                row.push(this.sequence(() => this.source[this.position] === "&" || this.source.startsWith("\\\\", this.position) || this.commandAhead("end")) || '""');
                if (this.source[this.position] === "&") {
                    this.position++;
                    continue;
                }
                if (this.source.startsWith("\\\\", this.position)) {
                    this.position += 2;
                    this.whitespace();
                    if (this.source[this.position] === "[")
                        this.fail("unsupported-row-spacing", uiMessage("mathLatex.customRowSpacingUnsupported"), this.position);
                    if (!this.commandAhead("end")) {
                        rows.push([]);
                        continue;
                    }
                }
                if (!this.commandAhead("end"))
                    this.fail("unclosed-environment", displayMessage("mathLatex.environmentEndMissing", { p0: uiDisplayParameter(name), p1: uiDisplayParameter(name) }), start, this.position);
                const ending = this.readCommand(), closing = this.literalGroup("end");
                if (closing !== name)
                    this.fail("mismatched-environment", displayMessage("mathLatex.environmentEndMismatch", { p0: uiDisplayParameter(name), p1: uiDisplayParameter(closing) }), ending.start, this.position);
                if (name === "cases") {
                    if (rows.some(items => items.length > 2))
                        this.fail("unsupported-cases", uiMessage("mathLatex.casesColumnLimit"), start, this.position);
                    return `cases(${rows.map(items => items.join(" & ")).join(", ")})`;
                }
                if (rows.some(items => items.length !== rows[0].length))
                    this.fail("ragged-matrix", uiMessage("mathLatex.matrixColumnsMismatch"), start, this.position);
                const delimiter: Record<string, string> = { matrix: "#none", pmatrix: quote("("), bmatrix: quote("["), Bmatrix: quote("{"), vmatrix: quote("|"), Vmatrix: quote("‖") };
                return `mat(delim: ${delimiter[name]}, ${rows.map(items => items.join(", ")).join("; ")})`;
            }
        });
    }
    private atom(singleCharacter = false): string {
        if (++this.atoms > MAX_ATOMS)
            this.fail("atom-limit", displayMessage("mathLatex.atomLimit", { p0: MAX_ATOMS }));
        const start = this.position, character = this.source[this.position];
        if (character === "{")
            return this.group();
        if (character === "\\") {
            const { name } = this.readCommand();
            if (forbidden.has(name))
                this.fail("forbidden-command", displayMessage("mathLatex.unsafeCommandNotExecuted", { p0: uiDisplayParameter(name) }), start, this.position);
            if (Object.hasOwn(symbols, name))
                return symbols[name];
            if (Object.hasOwn(escaped, name))
                return quote(escaped[name]);
            if (name === "frac" || name === "dfrac" || name === "tfrac" || name === "binom") {
                const numerator = this.argument(), denominator = this.argument();
                const result = `${name === "binom" ? "binom" : "frac"}(${numerator}, ${denominator})`;
                return name === "dfrac" ? `display(${result})` : name === "tfrac" ? `inline(${result})` : result;
            }
            if (name === "sqrt") {
                this.whitespace();
                let index: string | undefined;
                if (this.source[this.position] === "[") {
                    const begin = this.position++;
                    index = this.bounded(() => this.sequence(() => this.source[this.position] === "]"));
                    if (this.source[this.position] !== "]" || !index)
                        this.fail("invalid-root-index", uiMessage("mathLatex.rootIndexIncomplete"), begin, this.position);
                    this.position++;
                }
                const radicand = this.argument();
                return index === undefined ? `sqrt(${radicand})` : `root(${index}, ${radicand})`;
            }
            if (name === "text" || name === "operatorname") {
                const text = this.literalGroup(name);
                return name === "text" ? quote(text) : `op(${quote(text)})`;
            }
            const styles: Record<string, string> = { mathrm: "upright", mathbf: "bold", mathit: "italic", mathbb: "bb", mathcal: "cal", overline: "overline", underline: "underline", hat: "hat", bar: "bar", vec: "arrow", dot: "dot", ddot: "dot.double" };
            if (Object.hasOwn(styles, name)) {
                // Accents use fixed Unicode accents instead of user-provided function names.
                const content = this.argument(), accents: Record<string, string> = { hat: "^", bar: "¯", vec: "→", dot: "˙", ddot: "¨" };
                return Object.hasOwn(accents, name) ? `accent(${content}, ${quote(accents[name])})` : `${styles[name]}(${content})`;
            }
            if (name === "left")
                return this.paired();
            if (name === "begin")
                return this.environment(start);
            if (name === "," || name === ":" || name === ";" || name === "!" || name === "quad" || name === "qquad") {
                const widths: Record<string, string> = { ",": "0.1667", ":": "0.2222", ";": "0.2778", "!": "-0.1667", quad: "1", qquad: "2" };
                return `#h(${widths[name]}em)`;
            }
            if (name === "right" || name === "end" || name === "\\")
                this.fail("unexpected-boundary", displayMessage("mathLatex.unexpectedCommand", { p0: uiDisplayParameter(name) }), start, this.position);
            this.fail("unknown-command", displayMessage("mathLatex.commandUnknown", { p0: uiDisplayParameter(name) }), start, this.position);
        }
        if (character === "}" || character === "^" || character === "_" || character === "&")
            this.fail("unexpected-token", displayMessage("mathLatex.unexpectedCharacter", { p0: uiDisplayParameter(character) }), start);
        if (character === "$" || character === "#" || character === '"' || character === "`")
            this.fail("unsupported-token", displayMessage("mathLatex.characterNeedsTextOrEscape", { p0: uiDisplayParameter(character) }), start);
        const point = String.fromCodePoint(this.source.codePointAt(this.position)!);
        if (/\p{Script=Han}/u.test(point))
            this.fail("text-required", uiMessage("mathLatex.chineseTextSemantics"), start, start + point.length);
        this.position += point.length;
        if (!singleCharacter && /[0-9]/.test(character)) {
            const rest = /^[0-9]*(?:\.[0-9]+)?/.exec(this.source.slice(this.position))![0];
            this.position += rest.length;
            return character + rest;
        }
        if (character === "~")
            return '"\u00a0"';
        // Quote math argument separators and literal slash: Typst's unquoted / means a stacked fraction.
        if (",;:/()[]".includes(character))
            return quote(character);
        if (character === "*")
            return "∗";
        if (!/[\p{L}\p{N}\p{Sm}\p{Po}]/u.test(point) && !"+-=<>!|.".includes(point))
            this.fail("unsupported-character", displayMessage("mathLatex.characterMathMeaningUnsupported", { p0: uiDisplayParameter(point) }), start, this.position);
        return point;
    }
    parse(): string { return this.sequence(); }
}
/** Conversion has no network, file, eval, macro, or full TeX runtime dependency. */
export function convertLatexMath(original: string, options: LatexMathOptions = {}): LatexMathConversion {
    let begin = original.length - original.trimStart().length, end = original.trimEnd().length, display = options.display ?? false;
    const failure = (diagnostic: LatexMathDiagnostic): LatexMathConversion => ({ ok: false, original, display, body: null, typst: null, diagnostics: [diagnostic], support: LATEX_MATH_SUPPORT });
    if (original.length > MAX_INPUT)
        return failure({ code: "input-limit", message: displayMessage("mathLatex.inputLimit", { p0: MAX_INPUT }), start: MAX_INPUT, end: original.length });
    const controls = /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f-\u009f]/u.exec(original);
    if (controls)
        return failure({ code: "control-character", message: uiMessage("mathLatex.controlCharactersUnsupported"), start: controls.index, end: controls.index + controls[0].length });
    const invalidUnicode = /[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/u.exec(original);
    if (invalidUnicode)
        return failure({ code: "invalid-unicode", message: uiMessage("mathLatex.unicodeIncomplete"), start: invalidUnicode.index, end: invalidUnicode.index + 1 });
    const value = original.slice(begin, end);
    const wrappers: [
        string,
        string,
        boolean
    ][] = [["$$", "$$", true], ["\\[", "\\]", true], ["\\(", "\\)", false], ["$", "$", false]];
    for (const [left, right, mode] of wrappers) {
        if (!value.startsWith(left))
            continue;
        if (!value.endsWith(right) || value.length < left.length + right.length)
            return failure({ code: "unclosed-formula", message: uiMessage("mathLatex.outerDelimiterUnclosed"), start: begin, end });
        begin += left.length;
        end -= right.length;
        display = options.display ?? mode;
        break;
    }
    try {
        const body = new FormulaParser(original.slice(begin, end), begin).parse();
        if (!body)
            return failure({ code: "empty-formula", message: uiMessage("mathLatex.formulaInputRequired"), start: begin, end: Math.max(begin, end) });
        return { ok: true, original, display, body, typst: display ? `$ ${body} $` : `$${body}$`, diagnostics: [], support: LATEX_MATH_SUPPORT };
    }
    catch (error) {
        if (error instanceof ConversionFailure)
            return failure(error.diagnostic);
        throw error;
    }
}
export type ConvertedLatexMath = Extract<LatexMathConversion, {
    ok: true;
}>;
export interface LatexMathPanelHost {
    authorInputs?: import("./author-input").AuthorInputSession;
    /** Insert through the existing editor Draft/Undo command; do not mutate a second formula authority. */
    apply(result: ConvertedLatexMath): void | Promise<void>;
    /** Optional real Typst rendering. Return only a trusted local node or plain text; input is never HTML. */
    preview?(result: ConvertedLatexMath): HTMLElement | string | Promise<HTMLElement | string>;
    cancel?(): void;
}
export interface LatexMathPanel {
    element: HTMLElement;
    setInput(value: string, display?: boolean): void;
    destroy(): void;
}
let panelNumber = 0;
/** Mountable explicit conversion → preview → confirm flow. Closing/cancelling never calls apply. */
export function createLatexMathPanel(host: LatexMathPanelHost): LatexMathPanel {
    const make = <K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text: UiText = ""): HTMLElementTagNameMap[K] => { const item = document.createElement(tag); item.className = className; setUiText(item, text); return item; };
    const element = make("section", "data-panel latex-math-panel"), heading = make("h3", "", uiMessage("mathLatex.dialogLabel"));
    const id = `latex-math-${++panelNumber}`, input = make("textarea", "data-input"), label = make("label", "data-label", uiMessage("mathLatex.originalInputLabel"));
    input.id = `${id}-input`;
    input.rows = 6;
    input.maxLength = MAX_INPUT;
    input.spellcheck = false;
    label.htmlFor = input.id;
    const mode = make("select", "data-input"), modeLabel = make("label", "data-label", uiMessage("mathLatex.insertionMode"));
    mode.id = `${id}-mode`;
    modeLabel.htmlFor = mode.id;
    for (const [value, text] of [["inline", uiMessage("mathLatex.inlineFormula")], ["display", uiMessage("mathLatex.displayFormula")]] as const) {
        const item = make("option", "", text);
        item.value = value;
        mode.append(item);
    }
    const convert = make("button", "data-button", uiMessage("mathLatex.convertPreview")), confirm = make("button", "data-button primary", uiMessage("mathLatex.confirmInsert")), cancel = make("button", "data-button", uiMessage("mathLatex.cancelInsertion"));
    for (const button of [convert, confirm, cancel])
        button.type = "button";
    const status = make("p", "data-help", uiMessage("mathLatex.nativeEditingDescription")), source = make("pre", "output-source"), rendered = make("div", "output-preview"), buttons = make("div", "output-actions");
    status.setAttribute("role", "status");
    status.setAttribute("aria-live", "polite");
    const support = make("details", "output-detail"), list = make("ul");
    support.append(make("summary", "", uiMessage("mathLatex.conversionSupport")));
    LATEX_MATH_SUPPORT.forEach(value => list.append(make("li", "", value)));
    support.append(list);
    buttons.append(convert, confirm, cancel);
    element.append(heading, label, input, modeLabel, mode, buttons, status, source, rendered, support);
    let generation = 0, current: ConvertedLatexMath | null = null, disposed = false, applying = false;
    confirm.disabled = true;
    const invalidate = (): void => { generation++; current = null; confirm.disabled = true; setUiText(source, ""); rendered.replaceChildren(); setUiText(status, uiMessage("mathLatex.inputChangedConvertAgain")); };
    host.authorInputs?.bind(input, "latex");
    input.oninput = invalidate;
    mode.onchange = invalidate;
    convert.onclick = () => {
        void (async () => {
            if (applying || disposed)
                return;
            const request = ++generation, result = convertLatexMath(input.value, { display: mode.value === "display" });
            current = null;
            confirm.disabled = true;
            rendered.replaceChildren();
            setUiText(source, result.typst ?? "");
            if (!result.ok) {
                setUiText(status, joinUiText(result.diagnostics.map(item => displayMessage("mathLatex.diagnosticCharacterRange", { p0: uiDisplayParameter(item.message), p1: item.start + 1, p2: Math.max(item.start + 1, item.end) })), "\n"));
                return;
            }
            setUiText(status, host.preview ? uiMessage("mathLatex.generatingPreview") : uiMessage("mathLatex.sourcePreviewGenerated"));
            try {
                if (host.preview) {
                    const view = await host.preview(result);
                    if (request !== generation || disposed)
                        return;
                    if (typeof view === "string")
                        setUiText(rendered, view);
                    else
                        rendered.append(view);
                }
                if (request !== generation || disposed)
                    return;
                current = result;
                confirm.disabled = false;
                setUiText(status, host.preview ? uiMessage("mathLatex.previewGenerated") : status.textContent);
            }
            catch (error) {
                if (request === generation && !disposed)
                    setUiText(status, displayMessage("mathLatex.previewFailedInputRetained", { p0: uiDisplayParameter(errorUiText(error)) }));
            }
        })();
    };
    confirm.onclick = () => {
        void (async () => {
            if (!current || applying || disposed)
                return;
            const result = current;
            applying = true;
            input.readOnly = true;
            mode.disabled = true;
            convert.disabled = true;
            cancel.disabled = true;
            confirm.disabled = true;
            try {
                await host.apply(result);
                if (!disposed) {
                    current = null;
                    setUiText(status, uiMessage("mathLatex.insertedOriginalRetained"));
                }
            }
            catch (error) {
                if (!disposed) {
                    setUiText(status, displayMessage("mathLatex.insertionFailedInputRetained", { p0: uiDisplayParameter(errorUiText(error)) }));
                    confirm.disabled = false;
                }
            }
            finally {
                applying = false;
                if (!disposed) {
                    input.readOnly = false;
                    mode.disabled = false;
                    convert.disabled = false;
                    cancel.disabled = false;
                }
            }
        })();
    };
    cancel.onclick = () => {
        if (!applying) {
            invalidate();
            setUiText(status, uiMessage("mathLatex.cancelledDraftUnchanged"));
            host.cancel?.();
        }
    };
    return { element, setInput(value, display = false) {
            if (applying)
                throw new UiError(uiMessage("mathLatex.insertionBusy"));
            input.value = value;
            mode.value = display ? "display" : "inline";
            invalidate();
        }, destroy() { disposed = true; host.authorInputs?.retire(); generation++; input.oninput = null; mode.onchange = null; convert.onclick = confirm.onclick = cancel.onclick = null; element.remove(); } };
}
