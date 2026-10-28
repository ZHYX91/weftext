import test from "node:test";
import assert from "node:assert/strict";
import {
    WorkPanelPreferenceStore, clampWorkPanelHeight, decodeWorkPanelPreference, defaultWorkPanelPreference,
    keyboardWorkPanelHeight, workPanelPreferenceKey, workTaskNeedsAttention, workTaskProgress, type WorkPanelStorage,
} from "./work-panel-state";

function memoryStorage(): WorkPanelStorage & { records: Map<string, string> } {
    const records = new Map<string, string>();
    return { records, getItem: key => records.get(key) ?? null, setItem: (key, value) => { records.set(key, value); }, removeItem: key => { records.delete(key); } };
}

test("work panel is collapsed by default and accepts only bounded display preferences", () => {
    assert.equal(defaultWorkPanelPreference().collapsed, true);
    assert.deepEqual(decodeWorkPanelPreference({ collapsed: "false", height: Infinity, selected: "logs", tasks: ["private"] }), defaultWorkPanelPreference());
    assert.deepEqual(decodeWorkPanelPreference({ collapsed: false, height: 350.2, selected: "results" }), { collapsed: false, height: 350, selected: "results" });
    assert.deepEqual(decodeWorkPanelPreference([]), defaultWorkPanelPreference());
});

test("library layouts persist independently without storing diagnostic or task content", () => {
    const storage = memoryStorage(), store = new WorkPanelPreferenceStore(storage);
    store.write("workspace:a", { collapsed: false, height: 420, selected: "tasks", diagnostics: ["private"] } as never);
    store.write("workspace:b", { collapsed: true, height: 180, selected: "results" });
    const read = store.read("workspace:a"); read.height = 700;
    assert.equal(store.read("workspace:a").height, 420);
    assert.equal(store.read("workspace:b").height, 180);
    assert.deepEqual(JSON.parse(storage.records.get(workPanelPreferenceKey("workspace:a"))!), { collapsed: false, height: 420, selected: "tasks" });
    store.reset("workspace:b");
    assert.equal(store.read("workspace:a").height, 420);
    assert.deepEqual(store.read("workspace:b"), defaultWorkPanelPreference());
});

test("blocked storage preserves session geometry and reports once", () => {
    let notices = 0;
    const storage = { getItem() { throw new Error("denied"); }, setItem() { throw new Error("quota"); }, removeItem() { throw new Error("denied"); } };
    const store = new WorkPanelPreferenceStore(storage, () => { notices++; });
    store.read("a"); store.write("a", { collapsed: false, height: 320, selected: "tasks" });
    assert.equal(store.read("a").height, 320);
    store.reset("a"); assert.equal(notices, 1);
});

test("corrupt and oversized preference records use defaults", () => {
    const storage = memoryStorage();
    storage.records.set(workPanelPreferenceKey("broken"), "{");
    storage.records.set(workPanelPreferenceKey("oversized"), " ".repeat(2049));
    const store = new WorkPanelPreferenceStore(storage);
    assert.deepEqual(store.read("broken"), defaultWorkPanelPreference());
    assert.deepEqual(store.read("oversized"), defaultWorkPanelPreference());
});

test("resizer preserves document space and maps Up to a taller bottom panel", () => {
    assert.equal(keyboardWorkPanelHeight(220, "ArrowUp", false, 1000), 232);
    assert.equal(keyboardWorkPanelHeight(220, "ArrowDown", true, 1000), 172);
    assert.equal(keyboardWorkPanelHeight(220, "Home", false, 1000), 96);
    assert.equal(keyboardWorkPanelHeight(220, "End", false, 1000), 600);
    assert.equal(keyboardWorkPanelHeight(500, "Enter", false, 1000), 220);
    assert.equal(keyboardWorkPanelHeight(220, "Tab", false, 1000), null);
    assert.equal(clampWorkPanelHeight(700, 600), 360);
    assert.equal(clampWorkPanelHeight(220, 100), 96);
});

test("waiting and failure remain attention states; a task can report only finite bounded progress", () => {
    assert.equal(workTaskNeedsAttention("waiting"), true);
    assert.equal(workTaskNeedsAttention("failed"), true);
    assert.equal(workTaskNeedsAttention("running"), true);
    assert.equal(workTaskNeedsAttention("succeeded"), false);
    assert.equal(workTaskNeedsAttention("cancelled"), false);
    assert.equal(workTaskProgress(72.8), 73);
    for (const bad of [NaN, Infinity, -1, 101, "5", null]) assert.equal(workTaskProgress(bad), undefined);
});
