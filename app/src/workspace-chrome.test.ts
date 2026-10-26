import test, { type TestContext } from "node:test";
import assert from "node:assert/strict";
import { WorkspaceRibbon, type RibbonTab } from "./workspace-chrome";
import { TestDocument, TestElement } from "./dom.test-support";
import { locale, message, uiTextValue } from "./ui-locale";

class Element extends TestElement {
  id = "";
  tabIndex = 0;
  mutations = 0;
  onkeydown: ((event: KeyboardEvent) => unknown) | null = null;
  override appendChild(node: TestElement): TestElement { this.mutations++; return super.appendChild(node); }
  override insertBefore(node: TestElement, before: TestElement | null): TestElement { this.mutations++; return super.insertBefore(node, before); }
  focus(): void { (this.ownerDocument as Document).activeElement = this; }
  contains(node: unknown): boolean { return this.all().includes(node as TestElement); }
}
class Document extends TestDocument {
  activeElement: TestElement | null = null;
  override createElement(tag: string): Element { return new Element(tag, this); }
}
function fixture(t: TestContext) {
  const previous = Object.getOwnPropertyDescriptor(globalThis, "document"), document = new Document(), oldLocale = locale.current;
  Object.defineProperty(globalThis, "document", { configurable: true, writable: true, value: document });
  locale.set("zh-CN");
  const ribbon = new WorkspaceRibbon(), element = ribbon.element as unknown as Element;
  document.body.append(element);
  const roles = (role: string): Element[] => element.all().filter(node => node.getAttribute("role") === role) as Element[];
  const tab = (key: RibbonTab): Element => roles("tab").find(node => node.dataset.tab === key)!;
  const panel = (key: RibbonTab): Element => roles("tabpanel").find(node => node.dataset.tab === key)!;
  const group = (key: RibbonTab, id: string): Element => panel(key).all().find(node => node.dataset.group === id)! as Element;
  t.after(() => {
    ribbon.dispose(); locale.set(oldLocale);
    if (previous) Object.defineProperty(globalThis, "document", previous); else Reflect.deleteProperty(globalThis, "document");
  });
  return { document, ribbon, element, roles, tab, panel, group };
}
function key(target: Element, value: string, options: Partial<KeyboardEvent> = {}): boolean {
  let prevented = false;
  const event = { key: value, keyCode: 0, isComposing: false, defaultPrevented: false, altKey: false, ctrlKey: false, metaKey: false,
    preventDefault: () => { prevented = true; }, ...options } as KeyboardEvent;
  target.onkeydown?.(event);
  return prevented;
}

test("Ribbon defaults to Home with six stable, independently labelled ARIA tabs and one panel", t => {
  const f = fixture(t);
  assert.equal(f.ribbon.selected, "home");
  assert.equal(f.roles("tablist").length, 1);
  assert.equal(f.roles("tab").length, 6);
  assert.equal(f.roles("tabpanel").filter(panel => !panel.hidden).length, 1);
  const ids = f.roles("tab").map(tab => tab.id);
  assert.equal(new Set(ids).size, 6);
  for (const tab of ["file", "home", "insert", "knowledge", "review", "view"] as const) {
    assert.equal(f.tab(tab).getAttribute("aria-controls"), f.panel(tab).id);
    assert.equal(f.panel(tab).getAttribute("aria-labelledby"), f.tab(tab).id);
    assert.equal(f.tab(tab).getAttribute("aria-selected"), String(tab === "home"));
    assert.equal(f.tab(tab).tabIndex, tab === "home" ? 0 : -1);
  }
  const second = new WorkspaceRibbon();
  try { assert.ok(!ids.includes(((second.element as unknown as Element).all().find(node => node.getAttribute("role") === "tab")! as Element).id)); }
  finally { second.dispose(); }
});

test("Selecting command tabs or updating the same group preserves input identity, focus and proposal values", t => {
  const f = fixture(t), input = f.document.createElement("input"), action = f.document.createElement("button");
  input.value = "未应用提案 😀";
  f.ribbon.setGroup("insert", "structure", message("common.save"), [input, action] as unknown as HTMLElement[]);
  const group = f.group("insert", "structure"), content = group.children[0] as Element, movements = content.mutations;
  f.ribbon.select("insert"); input.focus();
  f.ribbon.setGroup("insert", "structure", message("common.close"), [input, action] as unknown as HTMLElement[]);
  assert.equal(content.mutations, movements, "same command nodes are not re-appended while pointer or IME input is active");
  f.ribbon.select("view"); f.ribbon.select("insert");
  assert.equal(input.value, "未应用提案 😀");
  assert.equal(f.document.activeElement, input);
  assert.equal(group.children[0].children[0], input);
  assert.equal(f.panel("view").hidden, true);
  assert.equal(f.panel("insert").hidden, false);
  assert.equal(group.getAttribute("aria-labelledby"), (group.children[1] as Element).id);
});

test("Groups can transfer existing nodes without cloning, hide when empty and clear without disposing caller controls", t => {
  const f = fixture(t), input = f.document.createElement("input");
  let actions = 0;
  input.onclick = () => { actions++; };
  f.ribbon.setGroup("home", "original", "Original", [input] as unknown as HTMLElement[]);
  f.ribbon.setGroup("review", "moved", "Moved", [input] as unknown as HTMLElement[]);
  assert.equal(f.group("home", "original").hidden, true);
  assert.equal(f.group("review", "moved").children[0].children[0], input);
  f.ribbon.setGroup("review", "moved", "Moved", []);
  assert.equal(f.group("review", "moved").hidden, true);
  input.onclick(); assert.equal(actions, 1);
  const firstId = (f.group("home", "original").children[1] as Element).id;
  f.ribbon.clearGroup("home", "original");
  f.ribbon.setGroup("home", "original", "New original", [input] as unknown as HTMLElement[]);
  assert.notEqual((f.group("home", "original").children[1] as Element).id, firstId, "retired group labels cannot alias a recreated group");
  const originalParent = input.parentNode;
  assert.throws(() => f.ribbon.setGroup("home", "duplicate", "Duplicate", [input, input] as unknown as HTMLElement[]));
  assert.equal(input.parentNode, originalParent);
});

test("Arrow, Home and End keys use roving selection but do not interfere with IME or modified shortcuts", t => {
  const f = fixture(t);
  assert.equal(key(f.tab("home"), "ArrowRight"), true);
  assert.equal(f.ribbon.selected, "insert"); assert.equal(f.document.activeElement, f.tab("insert"));
  assert.equal(key(f.tab("insert"), "Home"), true); assert.equal(f.ribbon.selected, "file");
  assert.equal(key(f.tab("file"), "ArrowLeft"), true); assert.equal(f.ribbon.selected, "view");
  assert.equal(key(f.tab("view"), "End"), true); assert.equal(f.ribbon.selected, "view");
  for (const options of [{ isComposing: true }, { keyCode: 229 }, { ctrlKey: true }, { metaKey: true }, { altKey: true }, { defaultPrevented: true }]) {
    assert.equal(key(f.tab("view"), "ArrowLeft", options), false);
    assert.equal(f.ribbon.selected, "view");
  }
  assert.equal(key(f.tab("view"), "Tab"), false);
});

test("Collapsing only hides panels, preserves chosen tab and exposes translated expansion state", t => {
  const f = fixture(t), input = f.document.createElement("input");
  f.ribbon.setGroup("insert", "input", "Input", [input] as unknown as HTMLElement[]);
  f.ribbon.select("insert"); input.focus();
  f.ribbon.setCollapsed(true);
  const toggle = f.element.all().find(node => node.className === "workspace-ribbon-toggle")! as Element;
  assert.equal(f.document.activeElement, toggle);
  assert.equal(toggle.getAttribute("aria-expanded"), "false");
  assert.equal(f.roles("tabpanel").filter(panel => !panel.hidden).length, 0);
  f.ribbon.select("review"); assert.equal(f.ribbon.selected, "review"); assert.equal(f.panel("review").hidden, true);
  locale.set("en-US");
  assert.equal(toggle.textContent, uiTextValue(message("workspaceRibbon.expand")));
  assert.equal(f.tab("review").textContent, uiTextValue(message("workspaceRibbon.review")));
  f.tab("insert").onclick!();
  assert.equal(f.ribbon.selected, "insert"); assert.equal(f.panel("insert").hidden, false);
  assert.equal(toggle.getAttribute("aria-expanded"), "true");
  assert.equal(input.parentNode, f.group("insert", "input").children[0]);
});

test("Unknown tabs fail before mutation and disposal retires only chrome bindings", t => {
  const f = fixture(t), input = f.document.createElement("input");
  let invoked = 0; input.onclick = () => { invoked++; };
  f.ribbon.setGroup("home", "commands", "Commands", [input] as unknown as HTMLElement[]);
  for (const value of ["unknown", "toString", "", null]) {
    assert.throws(() => f.ribbon.select(value as RibbonTab));
    assert.throws(() => f.ribbon.setGroup(value as RibbonTab, "commands", "Invalid", []));
    assert.throws(() => f.ribbon.clearGroup(value as RibbonTab, "commands"));
    assert.equal(f.ribbon.selected, "home");
  }
  f.ribbon.dispose(); f.ribbon.dispose();
  assert.equal(f.element.isConnected, false);
  assert.equal(f.tab("home").onclick, null);
  assert.equal(f.tab("home").onkeydown, null);
  input.onclick(); assert.equal(invoked, 1);
  assert.throws(() => f.ribbon.setCollapsed(false));
  assert.throws(() => f.ribbon.select("home"));
});
