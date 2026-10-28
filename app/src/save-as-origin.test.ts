import { strict as assert } from "node:assert";
import { test } from "node:test";
import { originalSaveAsSelection } from "./save-as-origin";

test("the SaveAs path dialog cannot redirect to a new current tab", async () => {
    const a = { session: "A" }, b = { session: "B" };
    let active = a, deliver!: (path: string) => void;
    const selected = originalSaveAsSelection(a, "A", () => new Promise<string>(resolve => { deliver = resolve; }),
        () => ({ owner: active, session: a.session, active: true }));
    active = b; deliver("chosen.typ");
    await assert.rejects(selected, /saveAsOriginChanged/);
});
test("a replaced original Session or retired author also rejects late delivery", async () => {
    const a = { session: "A", active: true };
    let release!: (path: string) => void;
    const choose = () => new Promise<string>(resolve => { release = resolve; });
    const context = () => ({ owner: a, session: a.session, active: a.active });
    const first = originalSaveAsSelection(a, "A", choose, context);
    a.session = "new"; release("one.typ");
    await assert.rejects(first, /saveAsOriginChanged/);
    a.session = "A";
    const second = originalSaveAsSelection(a, "A", choose, context);
    a.active = false; release("two.typ");
    await assert.rejects(second, /saveAsOriginChanged/);
});
test("the chosen path is delivered to the original live author", async () => {
    const a = { session: "A" };
    assert.deepEqual(await originalSaveAsSelection(a, "A", async () => "chosen.typ",
        () => ({ owner: a, session: "A", active: true })), { owner: a, path: "chosen.typ" });
    assert.equal(await originalSaveAsSelection(a, "A", async () => null,
        () => ({ owner: null, session: "B", active: false })), null);
});
