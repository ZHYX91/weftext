import test from "node:test";
import assert from "node:assert/strict";
import { NavigationPreferenceStore, defaultNavigationPreference, decodeNavigationPreference, navigationContextKey, sidebarResizeBounds, inspectorResizeBounds, clampPaneWidth, keyboardPaneWidth, type NavigationStorage } from "./navigation-layout";

function memoryStorage(): NavigationStorage & { records: Map<string, string> } {
  const records = new Map<string, string>(); return { records, getItem: key => records.get(key) ?? null, setItem: (key, value) => { records.set(key, value); }, removeItem: key => { records.delete(key); } };
}
const library = { root: "D:\\库", workspaceId: "workspace-a" };

test("per-library and typed scopes cannot share a geometry record by concatenation collision", () => {
  assert.notEqual(navigationContextKey({ ...library, scope: { kind: "tag", key: "a:b" } }), navigationContextKey({ ...library, scope: { kind: "query", key: "a:b" } }));
  assert.notEqual(navigationContextKey(library), navigationContextKey({ ...library, workspaceId: "workspace-b" }));
  assert.equal(navigationContextKey(library), navigationContextKey({ ...library, root: "D:\\MovedLibrary" }));
});

test("returning to a browsing scope restores only its own geometry and list position", () => {
  const storage = memoryStorage(); const store = new NavigationPreferenceStore(storage);
  const folder = { ...library, scope: { kind: "directory" as const, key: "directory-identity" } };
  const tag = { ...library, scope: { kind: "tag" as const, key: "tag-identity" } };
  store.write(folder, { ...defaultNavigationPreference(), sidebarWidth: 330, sidebarCollapsed: true, sidebarScroll: 810 });
  store.write(tag, { ...defaultNavigationPreference(), inspectorWideWidth: 720 });
  assert.equal(store.read(folder).sidebarScroll, 810); assert.equal(store.read(folder).sidebarCollapsed, true);
  assert.equal(store.read(tag).sidebarWidth, 264); assert.equal(store.read(tag).inspectorWideWidth, 720);
  store.reset(tag); assert.equal(store.read(folder).sidebarWidth, 330); assert.equal(store.read(tag).inspectorWideWidth, 680);
});

test("untrusted preference fields cannot inject author state or impossible pane widths", () => {
  const parsed = decodeNavigationPreference({ sidebarWidth: Infinity, inspectorWidth: -1, inspectorWideWidth: "680", sidebarCollapsed: "yes", sidebarScroll: NaN, source: "private", rows: ["stale"], token: "secret" });
  assert.deepEqual(parsed, defaultNavigationPreference());
  const storage = memoryStorage(); const store = new NavigationPreferenceStore(storage);
  store.write(library, { ...defaultNavigationPreference(), source: "should not survive" } as never);
  const stored = JSON.parse(storage.records.get(navigationContextKey(library))!);
  assert.equal("source" in stored, false); assert.equal("rows" in stored, false);
});

test("blocked local storage retains usable choices in memory and reports once", () => {
  let notices = 0; const unavailable: NavigationStorage = { getItem() { throw new Error("denied"); }, setItem() { throw new Error("quota"); }, removeItem() { throw new Error("denied"); } };
  const store = new NavigationPreferenceStore(unavailable, () => { notices++; });
  store.read(library); store.write(library, { ...defaultNavigationPreference(), sidebarWidth: 340 });
  assert.equal(store.read(library).sidebarWidth, 340); store.reset(library); assert.equal(notices, 1);
});

test("corrupt or oversized preference records reset without a misleading persistence warning", () => {
  const storage = memoryStorage(); storage.records.set(navigationContextKey(library), "{invalid"); let notices = 0;
  const store = new NavigationPreferenceStore(storage, () => { notices++; }); assert.deepEqual(store.read(library), defaultNavigationPreference());
  store.write(library, { ...defaultNavigationPreference(), sidebarWidth: 300 }); assert.equal(notices, 0);
  const other = { ...library, workspaceId: "oversized" }; storage.records.set(navigationContextKey(other), " ".repeat(5000)); assert.deepEqual(store.read(other), defaultNavigationPreference());
});

test("stored wide-screen sizes survive a temporary narrow viewport without overwriting preferences", () => {
  const storage = memoryStorage(); const store = new NavigationPreferenceStore(storage); store.write(library, { ...defaultNavigationPreference(), sidebarWidth: 460, inspectorWideWidth: 800 });
  const preferred = store.read(library); assert.equal(clampPaneWidth(preferred.sidebarWidth, sidebarResizeBounds(1020, true)), 380);
  assert.equal(clampPaneWidth(preferred.inspectorWideWidth, inspectorResizeBounds(1200, 820)), 500);
  assert.equal(store.read(library).sidebarWidth, 460); assert.equal(store.read(library).inspectorWideWidth, 800);
});

test("separator keyboard direction follows the physical boundary and keeps document space", () => {
  const bounds = { min: 260, max: 620 };
  assert.equal(keyboardPaneWidth(380, "sidebar", "ArrowLeft", false, bounds, 264), 368);
  assert.equal(keyboardPaneWidth(380, "inspector", "ArrowLeft", false, bounds, 380), 392);
  assert.equal(keyboardPaneWidth(600, "inspector", "ArrowLeft", true, bounds, 380), 620);
  assert.equal(keyboardPaneWidth(380, "inspector", "Home", false, bounds, 380), 260);
  assert.equal(keyboardPaneWidth(380, "inspector", "Enter", false, bounds, 380), 380);
  assert.equal(keyboardPaneWidth(380, "inspector", "Tab", false, bounds, 380), null);
});
