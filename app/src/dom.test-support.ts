/** Small DOM boundary for pure event/state tests. Real focus, layout and composition use Chromium checks. */
export class TestDocument {
  readonly body = new TestElement("body", this);
  createElement(tag: string): TestElement { return new TestElement(tag, this); }
  createTextNode(text: string): TestElement { const node = new TestElement("#text", this, 3); node.textContent = text; return node; }
}

export class TestElement {
  readonly childNodes: TestElement[] = [];
  readonly attributes = new Map<string, string>();
  readonly dataset: Record<string, string> = {};
  parentNode?: TestElement;
  hidden = false; value = ""; disabled = false; checked = false; type = ""; className = ""; srcdoc = "";
  open = false; readOnly = false;
  oninput?: () => void; onchange?: () => void; onclick?: () => void;
  private data = "";
  constructor(readonly tag: string, readonly ownerDocument: TestDocument, readonly nodeType = 1) {}
  get children(): TestElement[] { return this.childNodes.filter(node => node.nodeType === 1); }
  get isConnected(): boolean { return this === this.ownerDocument.body || this.parentNode?.isConnected === true; }
  get textContent(): string { return this.nodeType === 3 ? this.data : this.childNodes.map(node => node.textContent).join(""); }
  set textContent(value: string | null) { this.replaceChildren(); if (this.nodeType === 3) this.data = value ?? ""; else if (value) this.append(this.ownerDocument.createTextNode(value)); }
  appendChild(node: TestElement): TestElement { node.remove(); node.parentNode = this; this.childNodes.push(node); return node; }
  append(...nodes: (TestElement | string)[]): void { for (const node of nodes) this.appendChild(typeof node === "string" ? this.ownerDocument.createTextNode(node) : node); }
  insertBefore(node: TestElement, before: TestElement | null): TestElement { if (!before) return this.appendChild(node); node.remove(); const index = this.childNodes.indexOf(before); if (index < 0) throw new Error("Missing fixture child"); node.parentNode = this; this.childNodes.splice(index, 0, node); return node; }
  replaceChildren(...nodes: (TestElement | string)[]): void { for (const node of [...this.childNodes]) node.remove(); this.append(...nodes); }
  remove(): void { const siblings = this.parentNode?.childNodes; if (siblings) siblings.splice(siblings.indexOf(this), 1); this.parentNode = undefined; }
  setAttribute(name: string, value: string): void { this.attributes.set(name, value); }
  getAttribute(name: string): string | null { return this.attributes.get(name) ?? null; }
  removeAttribute(name: string): void { this.attributes.delete(name); }
  addEventListener(): void { /* Event-only fixtures invoke their production handlers directly. */ }
  showModal(): void { this.open = true; }
  close(): void { this.open = false; }
  querySelectorAll(): TestElement[] { return []; }
  all(): TestElement[] { return [this, ...this.children.flatMap(node => node.all())]; }
  find(tag: string, text?: string): TestElement { const node = this.all().find(node => node.tag === tag && (text === undefined || node.textContent === text)); if (!node) throw new Error(`Missing fixture element: ${tag}`); return node; }
}
