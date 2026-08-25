import { describe, expect, it } from "vitest";

import {
  INITIAL_NAVIGATION_WINDOW,
  directContents,
  incrementalWindow,
  locationBreadcrumbs,
  navigationProjection,
  readExplorerState,
  validatedBrowseLocator,
  visibleHierarchy,
  writeExplorerState,
  type SharedNavigationProjection,
} from "../app/shared-navigation";

function projection(count = 3): SharedNavigationProjection {
  const managedContents = Array.from({ length: Math.max(0, count - 1) }, (_, index) => ({
    kind: "managed_node" as const,
    name: `Node ${index + 1}`,
    locator: `Node ${index + 1}`,
    parentLocator: "",
    nodeId: `child-${index + 1}`,
    ownerNodeId: null,
    displayIcon: { kind: "default_node" },
  }));
  return {
    version: 1,
    rootNodeId: "root",
    hierarchy: Array.from({ length: count }, (_, index) => ({
      nodeId: index ? `child-${index}` : "root",
      name: index ? `Node ${index}` : "Workspace",
      parentNodeId: index ? "root" : null,
      locator: index ? `Node ${index}` : "",
      depth: index ? 1 : 0,
      childCount: index ? 0 : count - 1,
      displayIcon: { kind: index ? "default_node" : "workspace_root" },
    })),
    contents: [
      { kind: "managed_node", name: "Workspace", locator: "", parentLocator: null, nodeId: "root", ownerNodeId: null, displayIcon: { kind: "workspace_root" } },
      ...managedContents,
      { kind: "unmanaged_directory", name: "Files", locator: "Files", parentLocator: "", nodeId: null, ownerNodeId: null, displayIcon: { kind: "folder" } },
      { kind: "unmanaged_markdown", name: "inside.md", locator: "Files/inside.md", parentLocator: "Files", nodeId: null, ownerNodeId: null, displayIcon: { kind: "markdown_file" } },
      { kind: "resource", name: "asset.bin", locator: "Files/asset.bin", parentLocator: "Files", nodeId: null, ownerNodeId: null, displayIcon: { kind: "file" } },
    ],
  };
}

describe("shared Explorer projection", () => {
  it("preserves Core hierarchy order and only filters direct Contents rows", () => {
    const navigation = projection();
    expect(visibleHierarchy(navigation, new Set(), "").map((node) => node.nodeId)).toEqual(["root", "child-1", "child-2"]);
    expect(visibleHierarchy(navigation, new Set(["root"]), "").map((node) => node.nodeId)).toEqual(["root"]);
    expect(directContents(navigation, "Files").map((item) => item.locator)).toEqual(["Files/inside.md", "Files/asset.bin"]);
    expect(validatedBrowseLocator(navigation, "Files")).toBe("Files");
    expect(validatedBrowseLocator(navigation, "ignored/secret")).toBeNull();
    expect(locationBreadcrumbs(navigation, "Files").map((item) => item.name)).toEqual(["Workspace", "Files"]);
  });

  it("uses a versioned Core projection instead of rebuilding caller order", () => {
    const navigation = projection();
    const normalized = navigationProjection("root", navigation, [
      { id: "child-1", name: "wrong caller order" },
      { id: "root", name: "wrong caller order" },
    ]);
    expect(normalized).toBe(navigation);
    expect(normalized.hierarchy[0].nodeId).toBe("root");
  });

  it("restores mode, expansion, filter, width, scroll and validated browse locator as device state", () => {
    writeExplorerState("workspace", {
      version: 1,
      activity: "explorer",
      mode: "contents",
      collapsedNodeIds: ["root"],
      filter: "中文",
      width: 360,
      scrollTop: 125,
      browseLocator: "Files",
    });
    expect(readExplorerState("workspace")).toMatchObject({ mode: "contents", collapsedNodeIds: ["root"], filter: "中文", width: 360, scrollTop: 125, browseLocator: "Files" });
  });

  it("incrementally renders the 10,001-node UI fixture", () => {
    const rows = visibleHierarchy(projection(10_001), new Set(), "");
    const initial = incrementalWindow(rows, INITIAL_NAVIGATION_WINDOW);
    expect(rows).toHaveLength(10_001);
    expect(initial.items).toHaveLength(INITIAL_NAVIGATION_WINDOW);
    expect(initial.remaining).toBe(10_001 - INITIAL_NAVIGATION_WINDOW);
  });

  it("records separate 10,001-node UI projection and windowing latencies", () => {
    const navigation = projection(10_001);
    const measure = (operation: () => unknown) => {
      const samples = Array.from({ length: 21 }, () => {
        const startedAt = performance.now();
        operation();
        return performance.now() - startedAt;
      }).sort((left, right) => left - right);
      return Number(samples[10].toFixed(3));
    };
    const visible = visibleHierarchy(navigation, new Set(), "");
    const initialRender = measure(() => incrementalWindow(visibleHierarchy(navigation, new Set(), ""), INITIAL_NAVIGATION_WINDOW));
    const expand = measure(() => visibleHierarchy(navigation, new Set(["root"]), ""));
    const modeSwitch = measure(() => incrementalWindow(directContents(navigation, ""), INITIAL_NAVIGATION_WINDOW));
    const localRefresh = measure(() => {
      const refreshed = navigationProjection("root", navigation, []);
      incrementalWindow(visibleHierarchy(refreshed, new Set(), ""), INITIAL_NAVIGATION_WINDOW);
    });
    const keyboardMove = measure(() => visible.slice(0, INITIAL_NAVIGATION_WINDOW).findIndex((node) => node.nodeId === "child-120"));
    const timings = { initialRender, expand, modeSwitch, localRefresh, keyboardMove };
    console.info("shared-navigation-10001-ui-ms", timings);
    for (const duration of Object.values(timings)) {
      expect(Number.isFinite(duration)).toBe(true);
      expect(duration).toBeLessThan(1_000);
    }
  });
});
