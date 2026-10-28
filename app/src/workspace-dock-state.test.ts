import test from "node:test";
import assert from "node:assert/strict";
import { defaultDockState, decodeDockState, dockContextKey, hideDockView, moveDockView, resizeDockGroups, revealDockView, type DockState, type DockViewDefinition } from "./workspace-dock-state";

const views: DockViewDefinition[] = [
    { id: "files", side: "left", group: "navigation" },
    { id: "nodes", side: "left", group: "navigation" },
    { id: "outline", side: "left", group: "structure" },
    { id: "timeline", side: "left", group: "navigation", hidden: true },
    { id: "properties", side: "right", group: "inspection" },
    { id: "calendar", side: "right", group: "extensions" },
];
const listed = (state: DockState): string[] => [...state.left, ...state.right].flatMap(group => group.views).concat(state.hidden);
function assertLayout(state: DockState): void {
    assert.deepEqual([...listed(state)].sort(), views.map(view => view.id).sort());
    assert.equal(new Set(listed(state)).size, views.length);
    assert.equal(new Set([...state.left, ...state.right].map(group => group.id)).size, state.left.length + state.right.length);
    for (const group of [...state.left, ...state.right]) {
        assert.ok(group.views.length > 0); assert.ok(group.views.includes(group.active));
        assert.ok(group.weight >= .1 - Number.EPSILON && group.weight <= 10 + Number.EPSILON);
    }
}
test("default groups keep registration order and separately hidden views", () => {
    const state = defaultDockState(views);
    assert.deepEqual(state.left.map(group => group.views), [["files", "nodes"], ["outline"]]);
    assert.deepEqual(state.right.map(group => group.views), [["properties"], ["calendar"]]);
    assert.deepEqual(state.hidden, ["timeline"]); assertLayout(state);
});
test("the same registration group name on each side remains two stable tab groups", () => {
    const state = defaultDockState([{ id: "a", side: "left", group: "shared" }, { id: "b", side: "right", group: "shared" }, { id: "c", side: "right", group: "shared" }]);
    assert.deepEqual(state.right.map(group => group.views), [["b", "c"]]); assert.notEqual(state.left[0].id, state.right[0].id);
});
test("an empty right sidebar defaults closed but an explicit open preference survives decoding", () => {
    const leftOnly: DockViewDefinition[] = [{ id: "files", side: "left" }, { id: "properties", side: "right", hidden: true }];
    const state = defaultDockState(leftOnly); assert.equal(state.rightHidden, true);
    assert.equal(decodeDockState({ ...state, rightHidden: false }, leftOnly).rightHidden, false);
    assert.equal(decodeDockState({ version: 1, left: [], right: [] }, leftOnly).rightHidden, true);
    assert.equal(revealDockView(state, "properties", leftOnly).rightHidden, false);
    assert.equal(defaultDockState(views).rightHidden, false);
});
test("split, cross-side merge and reorder preserve unique membership and source selection", () => {
    const original = defaultDockState(views), originalBytes = JSON.stringify(original);
    const split = moveDockView(original, "nodes", { side: "left", groupId: "structure", position: "before" });
    assert.deepEqual(split.left.map(group => group.views), [["files"], ["nodes"], ["outline"]]);
    const moved = moveDockView(split, "nodes", { side: "right", groupId: "inspection", position: "center" });
    assert.deepEqual(moved.right[0].views, ["properties", "nodes"]); assert.equal(moved.right[0].active, "nodes");
    const selectedMoved = moveDockView(moved, "nodes", { side: "left", groupId: "navigation", position: "center" });
    assert.equal(selectedMoved.right[0].active, "properties"); assertLayout(selectedMoved);
    assert.equal(JSON.stringify(original), originalBytes);
});
test("hiding and revealing restores tab position, split group position and current visibility", () => {
    const original = defaultDockState(views);
    const hiddenTab = hideDockView(original, "files");
    assert.deepEqual(hiddenTab.left[0].views, ["nodes"]); assert.equal(hiddenTab.left[0].active, "nodes");
    assert.deepEqual(revealDockView(hiddenTab, "files", views).left[0].views, ["files", "nodes"]);
    const hiddenGroup = hideDockView(original, "outline"); assert.equal(hiddenGroup.left.length, 1);
    hiddenGroup.leftHidden = true;
    const restored = revealDockView(hiddenGroup, "outline", views);
    assert.deepEqual(restored.left.map(group => group.views), [["files", "nodes"], ["outline"]]);
    assert.equal(restored.leftHidden, false); assertLayout(restored);
    assertLayout(revealDockView(restored, "timeline", views));
});
test("several hidden tabs and entire groups restore their original order in either reveal order", () => {
    let hidden = defaultDockState(views);
    hidden = hideDockView(hideDockView(hideDockView(hidden, "files"), "nodes"), "outline");
    assert.deepEqual(hidden.left, []);
    for (const order of [["outline", "nodes", "files"], ["files", "outline", "nodes"]]) {
        let state = hidden;
        for (const id of order) state = revealDockView(state, id, views);
        assert.deepEqual(state.left.map(group => group.views), [["files", "nodes"], ["outline"]]); assertLayout(state);
    }
});
test("hiding an entire resized group preserves its latest height through either reveal order", () => {
    let state = resizeDockGroups(defaultDockState(views), "left", 0, .15);
    assert.equal(state.left[0].weight, 1.3);
    state = hideDockView(state, "files");
    state = resizeDockGroups(state, "left", 0, .1);
    const latestWeight = state.left[0].weight; assert.equal(latestWeight, 1.5);
    state = hideDockView(state, "nodes");
    assert.equal(state.hiddenPlacement.files.weight, latestWeight); assert.equal(state.hiddenPlacement.nodes.weight, latestWeight);
    for (const order of [["files", "nodes"], ["nodes", "files"]]) {
        let restored = state;
        for (const id of order) restored = revealDockView(restored, id, views);
        assert.equal(restored.left[0].weight, latestWeight); assert.deepEqual(restored.left[0].views, ["files", "nodes"]); assertLayout(restored);
    }
    const decoded = decodeDockState({ ...state, hiddenPlacement: { ...state.hiddenPlacement, files: { ...state.hiddenPlacement.files, weight: Infinity }, nodes: { ...state.hiddenPlacement.nodes, weight: 200 } } }, views);
    assert.equal(decoded.hiddenPlacement.files.weight, undefined); assert.equal(decoded.hiddenPlacement.nodes.weight, 10);
});
test("decode rejects arbitrary content, unknown IDs, duplicates and impossible active members", () => {
    const decoded = decodeDockState({ version: 1, source: "secret draft", token: "token", leftHidden: "false", rightHidden: true,
        left: [{ id: "same", views: ["files", "files", "missing", "nodes"], active: "missing", weight: Infinity, collapsed: "yes", source: "bad" }],
        right: [{ id: "same", views: ["files", "properties"], active: "properties", weight: -4, collapsed: true }], hidden: ["files", "timeline", "timeline", "unknown"],
        hiddenPlacement: JSON.parse('{"timeline":{"side":"left","groupId":"last","groupIndex":9999,"index":-4,"token":"bad"},"__proto__":{"side":"left","groupId":"bad"}}'), maximized: "missing" }, views);
    assert.equal(decoded.left[0].active, "files"); assert.equal(decoded.left[0].weight, 1); assert.equal(decoded.left[0].collapsed, false);
    assert.equal(decoded.right[0].weight, .1); assert.notEqual(decoded.left[0].id, decoded.right[0].id);
    assert.equal(decoded.leftHidden, false); assert.equal(decoded.rightHidden, true); assert.equal(decoded.maximized, null);
    assert.deepEqual(decoded.hiddenPlacement.timeline, { side: "left", groupId: "last", groupIndex: 29, index: 0 });
    assert.equal(Object.getPrototypeOf(decoded.hiddenPlacement), null);
    assert.equal(JSON.stringify(decoded).includes("secret"), false); assert.equal(JSON.stringify(decoded).includes("token"), false); assertLayout(decoded);
});
test("unsupported versions and corrupt layouts reset; missing new registrations are added", () => {
    assert.deepEqual(decodeDockState({ version: 2, left: [] }, views), defaultDockState(views));
    assert.deepEqual(decodeDockState(null, views), defaultDockState(views));
    const partial = decodeDockState({ version: 1, left: [{ id: "custom", views: ["nodes"], active: "nodes", weight: 2 }], right: null, hidden: [] }, views);
    assert.equal(partial.left[0].id, "custom"); assert.equal(partial.left[0].weight, 2); assertLayout(partial);
});
test("resizing preserves pair total and bounds, with keyboard-sized deltas and malformed input guards", () => {
    const original = defaultDockState(views), resized = resizeDockGroups(original, "left", 0, .1);
    assert.equal(resized.left[0].weight, 1.2); assert.equal(resized.left[1].weight, .8);
    const clamped = resizeDockGroups(original, "left", 0, 100);
    assert.ok(Math.abs(clamped.left[1].weight - .2) < 1e-10); assert.equal(clamped.left[0].weight + clamped.left[1].weight, 2);
    assert.deepEqual(resizeDockGroups(original, "left", 0, NaN), original);
    assert.deepEqual(resizeDockGroups(original, "left", -1, .1), original);
    assert.equal(original.left[0].weight, 1);
});
test("maximization cannot persist for removed or collapsed groups; reveal makes its target reachable", () => {
    const original = defaultDockState(views); original.maximized = "structure";
    assert.equal(hideDockView(original, "outline").maximized, null);
    const decoded = decodeDockState({ ...original, left: original.left.map(group => ({ ...group, collapsed: true })) }, views);
    assert.equal(decoded.maximized, null);
    const visible = revealDockView(original, "properties", views); assert.equal(visible.maximized, null);
});
test("bounded repeated moves never duplicate, lose or infer a view from stale IDs", () => {
    let state = defaultDockState(views);
    for (let at = 0; at < 150; at++) {
        const id = views[at % views.length].id;
        if (at % 3 === 0) state = hideDockView(state, id);
        else if (at % 3 === 1) state = revealDockView(state, id, views);
        else state = moveDockView(state, id, { side: at % 2 ? "right" : "left", groupId: state.left[0]?.id, position: "after" });
        assertLayout(state);
    }
    assert.deepEqual(moveDockView(state, "stale", { side: "right" }), state);
});
test("context keys preserve literal context boundaries without path or delimiter collisions", () => {
    assert.notEqual(dockContextKey("workspace:a/b"), dockContextKey("workspace:a%2Fb"));
    assert.notEqual(dockContextKey("workspace:a"), dockContextKey("workspace:b"));
    assert.equal(dockContextKey("workspace:a"), "weftext-workspace-dock:v1:workspace%3Aa");
});
