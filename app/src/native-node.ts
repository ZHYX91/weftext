import { nativeStyleProperties } from "./native-style";
import type { RenderNode } from "./types";

/** Shared allowlist for local and hosted projections; never inject author HTML. */
export function nativeElement(node: RenderNode): HTMLElement | MathMLElement {
    const allowed = new Set(["div", "p", "span", "strong", "em", "s", "del", "u", "h1", "h2", "h3", "h4", "h5", "h6", "ul", "ol", "li", "dl", "dt", "dd", "blockquote", "pre", "code", "table", "thead", "tbody", "tfoot", "tr", "td", "th", "br", "hr", "a", "img", "sup", "sub", "section", "article", "figure", "figcaption"]);
    const math = new Set(["math", "mrow", "mi", "mn", "mo", "mtext", "ms", "mspace", "msup", "msub", "msubsup", "mfrac", "msqrt", "mroot", "munder", "mover", "munderover", "mtable", "mtr", "mtd", "mstyle", "mpadded", "mphantom", "menclose", "semantics", "annotation"]);
    const tag = node.tag?.toLowerCase() ?? "span";
    const element: HTMLElement | MathMLElement = math.has(tag) ? document.createElementNS("http://www.w3.org/1998/Math/MathML", tag) : document.createElement(allowed.has(tag) ? tag : "div");
    element.classList.add("native-node");
    // Only individual semantic attributes are consumed. Author HTML/CSS is never injected.
    for (const attr of ["colspan", "rowspan", "start", "value", "aria-level"]) {
      const raw = node.attrs[attr]; const value = typeof raw === "string" && /^\d+$/.test(raw) ? Number(raw) : raw; if (typeof value === "number" && Number.isSafeInteger(value) && value > 0 && value <= 10000) element.setAttribute(attr, String(value));
    }
    for (const attr of ["title", "lang", "dir", "role", "aria-label", "display", "mathvariant", "stretchy", "fence", "separator", "accent", "accentunder", "lspace", "rspace", "encoding"]) {
      const value = node.attrs[attr]; if (typeof value === "string" && value.length < 4096 && !value.includes("\0")) element.setAttribute(attr, value);
    }
    if (typeof node.attrs.id === "string") element.id = `native-${node.attrs.id}`;
    for (const [target, attribute] of [["mfrac", "linethickness"], ["mspace", "width"]]) {
      const raw = node.attrs[attribute];
      if (tag === target && typeof raw === "string" && raw.length <= 40 && /^(?:[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:em|ex|px|pt|pc|cm|mm|in|%|mu)?|thin|medium|thick)$/.test(raw)) element.setAttribute(attribute, raw);
    }
    const styles = node.attrs.style;
    for (const [key, value] of nativeStyleProperties(styles)) element.style.setProperty(key, value);
    if (styles && typeof styles === "object") {
      const map = styles as Record<string, unknown>;
      for (const [source, css] of [["font-weight", "fontWeight"], ["font-style", "fontStyle"], ["text-align", "textAlign"]]) {
        const value = map[source]; if (typeof value === "string" && /^(normal|bold|[1-9]00|italic|oblique|left|right|center|justify|start|end)$/.test(value)) (element.style as unknown as Record<string, string>)[css] = value;
      }
    }
    if (tag === "img") { const source = node.attrs.src; if (typeof source === "string" && /^data:image\/(png|jpeg|gif|webp);base64,[a-zA-Z0-9+/=]+$/.test(source) && source.length < 12 * 1024 * 1024) element.setAttribute("src", source); if (typeof node.attrs.alt === "string") element.setAttribute("alt", node.attrs.alt); }
    return element;
}
