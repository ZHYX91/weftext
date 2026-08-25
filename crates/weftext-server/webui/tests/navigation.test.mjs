import assert from "node:assert/strict";
import test from "node:test";

import {
  INITIAL_NAVIGATION_WINDOW,
  directContents,
  incrementalItems,
  locationBreadcrumbs,
  validateBrowseLocator,
  visibleHierarchy,
} from "../navigation.js";

function projection(nodeCount = 3) {
  return {
    version: 1,
    rootNodeId: "root",
    hierarchy: Array.from({ length: nodeCount }, (_, index) => ({
      nodeId: index ? `child-${index}` : "root",
      name: index ? `Node ${index}` : "Workspace",
      parentNodeId: index ? "root" : null,
      locator: index ? `Node ${index}` : "",
      depth: index ? 1 : 0,
      childCount: index ? 0 : nodeCount - 1,
      displayIcon: { kind: index ? "default_node" : "workspace_root" },
    })),
    contents: [
      { kind: "managed_node", name: "Workspace", locator: "", parentLocator: null, nodeId: "root" },
      { kind: "managed_node", name: "Node 1", locator: "Node 1", parentLocator: "", nodeId: "child-1" },
      { kind: "unmanaged_directory", name: "Files", locator: "Files", parentLocator: "", nodeId: null },
      { kind: "unmanaged_markdown", name: "inside.md", locator: "Files/inside.md", parentLocator: "Files", nodeId: null },
      { kind: "resource", name: "asset.bin", locator: "Files/asset.bin", parentLocator: "Files", nodeId: null },
    ],
  };
}

test("Hierarchy preserves Core order while collapse and filtering retain ancestors", () => {
  const navigation = projection();
  assert.deepEqual(visibleHierarchy(navigation, [], "").map((node) => node.nodeId), ["root", "child-1", "child-2"]);
  assert.deepEqual(visibleHierarchy(navigation, ["root"], "").map((node) => node.nodeId), ["root"]);
  assert.deepEqual(visibleHierarchy(navigation, [], "Node 2").map((node) => node.nodeId), ["root", "child-2"]);
});

test("Contents browses only direct Core-visible rows and validates unmanaged locators", () => {
  const navigation = projection();
  assert.deepEqual(directContents(navigation, "Files").map((item) => item.locator), ["Files/inside.md", "Files/asset.bin"]);
  assert.equal(validateBrowseLocator(navigation, "Files"), "Files");
  assert.equal(validateBrowseLocator(navigation, "ignored"), null);
  assert.deepEqual(locationBreadcrumbs(navigation, "Files").map((crumb) => crumb.name), ["Workspace", "Files"]);
});

test("10,001-node navigation incrementally renders without counting Core scan time", () => {
  const navigation = projection(10_001);
  const rows = visibleHierarchy(navigation, [], "");
  const initial = incrementalItems(rows, INITIAL_NAVIGATION_WINDOW);
  assert.equal(rows.length, 10_001);
  assert.equal(initial.items.length, INITIAL_NAVIGATION_WINDOW);
  assert.equal(initial.remaining, 10_001 - INITIAL_NAVIGATION_WINDOW);
});
