import assert from "node:assert/strict";
import test from "node:test";
import { tagPicker } from "./tag-picker";
import { TagTree, type TreeTag } from "./tag-tree";
/** DOM event fixture; real keyboard, focus and assistive technology remain browser acceptance. */
class PickerElement {
  children: PickerElement[] = []; attributes = new Map<string, string>(); type = ""; className = ""; value = ""; textContent = ""; disabled = false;
  oninput?: () => void; onchange?: () => void; onclick?: () => void;
  constructor(readonly tag: string) {}
  append(...elements: PickerElement[]): void { this.children.push(...elements); }
  replaceChildren(...elements: PickerElement[]): void { this.children = elements; }
  setAttribute(name: string, value: string): void { this.attributes.set(name, value); }
  all(): PickerElement[] { return [this, ...this.children.flatMap(child => child.all())]; }
  find(tag: string, text?: string): PickerElement { const found = this.all().find(element => element.tag === tag && (text === undefined || element.textContent === text)); assert.ok(found); return found; }
}
const item = (i: number, parentTagId: string | null = null): TreeTag => ({ tagId: `tag-${i}`, reference: `weftext-tag://workspace/tag-${i}`, displayName: `标签${i}`, parentTagId });
function dom(run: () => void): void { const original = Object.getOwnPropertyDescriptor(globalThis, "document"); Object.defineProperty(globalThis, "document", { configurable: true, value: { createElement: (tag: string) => new PickerElement(tag) } }); try { run(); } finally { if (original) Object.defineProperty(globalThis, "document", original); else Reflect.deleteProperty(globalThis, "document"); } }
test("tag picker pages 5,000 choices, searches the real identity and writes only an explicit direct selection", () => dom(() => {
  const tree = new TagTree(Array.from({ length: 5000 }, (_, i) => item(i)), true), assigned: (string | null)[] = [], host = tagPicker(tree, { label: "直接标签", changed: reference => assigned.push(reference) }) as unknown as PickerElement;
  const select = host.find("select"), search = host.find("input"); assert.equal(select.children.length, 101); assert.deepEqual(assigned, []);
  host.find("button", "下一段").onclick!(); assert.equal(select.children[1].value, item(100).reference); assert.deepEqual(assigned, []);
  search.value = item(4999).reference; search.oninput!(); assert.equal(select.children.length, 2); select.value = item(4999).reference; select.onchange!(); assert.deepEqual(assigned, [item(4999).reference]);
  search.value = "不存在"; search.oninput!(); assert.equal(select.value, item(4999).reference); assert.equal(select.children.length, 2); assert.deepEqual(assigned, [item(4999).reference], "filtering display never edits the field or assigns ancestors");
}));
test("parent selector excludes self and descendants, retains an unavailable old identity and explicitly supports root", () => dom(() => {
  const root = item(0), child = item(1, root.tagId), other = item(2), assigned: (string | null)[] = [], tree = new TagTree([root, child, other], true), host = tagPicker(tree, { label: "父级", excludedRoot: root.tagId, root: true, value: child.reference, changed: reference => assigned.push(reference) }) as unknown as PickerElement;
  const select = host.find("select"); assert.deepEqual(select.children.filter(option => !option.disabled).map(option => option.value), ["", other.reference]); assert.ok(select.children.find(option => option.value === child.reference)?.disabled); assert.deepEqual(assigned, []);
  select.value = ""; select.onchange!(); assert.deepEqual(assigned, [null]);
}));
