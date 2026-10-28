import test, { type TestContext } from "node:test";
import assert from "node:assert/strict";
import { TestDocument, TestElement } from "./dom.test-support";
import { moveOrderedValue, orderedMoveTarget, valueEditor, nativeFormControls, type DataSchema, type NativeValue, type Shape } from "./native-value-form";
import { locale } from "./ui-locale";

const schema: DataSchema = { definitions: {}, fields: [], types: [], aliases: {} };
const text = (value: string): NativeValue => ({ kind: "text", value });
const texts: Shape = { kind: "list", items: { kind: "scalar", semantic: "text" }, min: 0, max: null };

test("display positions accept only complete in-range decimal ordinals", () => {
    assert.equal(orderedMoveTarget("1", 3000), 0);
    assert.equal(orderedMoveTarget("3000", 3000), 2999);
    for (const invalid of ["", "0", "-1", "1.5", "1e2", " 2", "2 ", "02", "9007199254740993", "3001"])
        assert.equal(orderedMoveTarget(invalid, 3000), null, invalid);
    assert.equal(orderedMoveTarget("1", 0), null);
    assert.equal(orderedMoveTarget("1", 1.5), null);
});

test("first, last and exact-index moves retain duplicate records and unknown members", () => {
    const first: NativeValue = { kind: "record", value: [{ key: "number", value: text("00138") }, { key: "unknown", value: text("第一条") }] };
    const second: NativeValue = { kind: "record", value: [{ key: "number", value: text("00138") }, { key: "unknown", value: text("第二条") }] };
    const value: NativeValue = { kind: "array", value: [first, text("middle"), second] };
    const moved = moveOrderedValue(value, 2, 0);
    assert.deepEqual(moved, { kind: "array", value: [second, first, text("middle")] });
    assert.deepEqual(moveOrderedValue(moved, 0, 2), value);
    assert.equal(value.value[0], first);
    for (const index of [NaN, Infinity, 0.5, -1, 3]) assert.equal(moveOrderedValue(value, 0, index), value);
    assert.equal(moveOrderedValue(value, 1, 1), value);
});

class Element extends TestElement {
    id = "";
    validationMessage = "";
    reportCount = 0;
    readonly listeners = new Map<string, EventListenerOrEventListenerObject[]>();
    get validity() { return { valid: !this.validationMessage }; }
    setCustomValidity(value: string): void { this.validationMessage = value; }
    reportValidity(): boolean { this.reportCount++; return this.validity.valid; }
    focus(): void { if (this.isConnected && !this.disabled) (this.ownerDocument as Document).activeElement = this; }
    override addEventListener(type?: string, listener?: EventListenerOrEventListenerObject | null): void {
        if (type && listener) this.listeners.set(type, [...this.listeners.get(type) ?? [], listener]);
    }
    emit(type: string, fields: Record<string, unknown> = {}): void {
        const event = { type, target: this, preventDefault() {}, ...fields } as unknown as Event;
        for (const listener of this.listeners.get(type) ?? []) typeof listener === "function" ? listener(event) : listener.handleEvent(event);
    }
    matches(selector: string): boolean {
        return selector.split(",").map(part => part.trim()).some(part => part === this.tag || part === `${this.tag}:invalid` && !this.validity.valid);
    }
    override querySelectorAll(selector = ""): Element[] { return this.all().slice(1).filter(node => node instanceof Element && node.matches(selector)) as Element[]; }
    querySelector(selector: string): Element | null { return this.querySelectorAll(selector)[0] ?? null; }
}
class Input extends Element {}
class Textarea extends Element {}
class Document extends TestDocument {
    activeElement: Element | null = null;
    override createElement(tag: string): Element { return tag === "input" ? new Input(tag, this) : tag === "textarea" ? new Textarea(tag, this) : new Element(tag, this); }
}
function fixture(t: TestContext, shape = texts, initial: NativeValue = { kind: "array", value: [text("first"), text("second"), text("third")] }) {
    const previous = new Map(["document", "HTMLInputElement", "HTMLTextAreaElement"].map(key => [key, Object.getOwnPropertyDescriptor(globalThis, key)]));
    const document = new Document();
    Object.defineProperties(globalThis, { document: { configurable: true, value: document }, HTMLInputElement: { configurable: true, value: Input }, HTMLTextAreaElement: { configurable: true, value: Textarea } });
    t.after(() => { for (const [key, descriptor] of previous) if (descriptor) Object.defineProperty(globalThis, key, descriptor); else Reflect.deleteProperty(globalThis, key); locale.set("zh-CN"); });
    const changes: NativeValue[] = [];
    const editor = valueEditor(shape, initial, schema, value => changes.push(value), "作者字段", { fieldId: "test/value", path: [] }) as unknown as Element;
    document.body.append(editor);
    const rows = () => editor.all().filter(node => node.className === "native-list-item") as Element[];
    const order = (row: number, action: string) => rows()[row].all().find(node => node.dataset.orderAction === action) as Element;
    return { editor, document, changes, rows, order };
}

test("actual long-distance list actions change one local value and retain focus on the moved item", t => {
    const f = fixture(t);
    f.order(2, "first").onclick!();
    assert.deepEqual(f.changes, [{ kind: "array", value: [text("third"), text("first"), text("second")] }]);
    assert.ok(f.rows()[0].all().includes(f.document.activeElement!));
    assert.equal(f.rows()[0].find("textarea").value, "third");
    const target = f.order(0, "position"); target.value = "2";
    target.emit("keydown", { key: "Enter", isComposing: false, keyCode: 13 });
    assert.deepEqual(f.changes.at(-1), { kind: "array", value: [text("first"), text("third"), text("second")] });
    assert.equal(f.document.activeElement, f.order(1, "position"));
    assert.equal(nativeFormControls(f.editor as unknown as HTMLElement).length, 6);
    locale.set("en-US");
    assert.equal(f.document.activeElement, f.order(1, "position"));
});

test("invalid target and incomplete native input survive attempted reordering without changing the value", t => {
    const shape: Shape = { kind: "list", items: { kind: "scalar", semantic: "int" }, min: 0, max: null };
    const f = fixture(t, shape, { kind: "array", value: [{ kind: "int", value: "1" }, { kind: "int", value: "2" }] });
    const position = f.order(0, "position"); position.value = "2.5";
    position.emit("keydown", { key: "Enter", isComposing: false, keyCode: 13 });
    assert.equal(position.value, "2.5"); assert.equal(position.getAttribute("aria-invalid"), "true");
    assert.deepEqual(f.changes, []); assert.equal(f.document.activeElement, position);
    const source = f.rows()[0].all().find(node => node.tag === "input" && !node.dataset.orderAction) as Element;
    source.value = "-"; source.oninput!();
    const row = f.rows()[0]; f.order(0, "last").onclick!();
    assert.deepEqual(f.changes, []); assert.equal(f.rows()[0], row);
    assert.equal(source.value, "-"); assert.equal(source.reportCount, 1); assert.equal(f.document.activeElement, source);
});

test("composition and late disconnected actions cannot reorder a live list", t => {
    const f = fixture(t), old = f.order(2, "first"), input = f.rows()[0].find("textarea");
    f.editor.emit("compositionstart", { target: input });
    old.onclick!(); assert.deepEqual(f.changes, []);
    f.order(0, "position").emit("keydown", { key: "Enter", isComposing: true, keyCode: 229 });
    assert.deepEqual(f.changes, []);
    f.editor.emit("compositionend", { target: input });
    old.onclick!(); assert.equal(f.changes.length, 1);
    old.onclick!(); assert.equal(f.changes.length, 1, "an old detached button is not a second owner");
});
