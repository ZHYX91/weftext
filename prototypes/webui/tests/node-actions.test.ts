import { describe, expect, it } from "vitest";
import {
  WORKSPACE_ACTION_REGISTRY,
  conflictingDirtyNodeIds,
  freezeCoreReviewedDraftScope,
  resolveWorkspaceActionTarget,
} from "../app/node-actions";

describe("workspace action registry and target resolver", () => {
  it("uses the focused second-pane UUID for every current-node command", () => {
    for (const action of ["create", "rename", "move", "copy", "trash_node", "chrono", "trash_resource"] as const) {
      expect(resolveWorkspaceActionTarget(action, { source: "editor_command", focusedNodeId: "split-node" })).toMatchObject({
        kind: "node",
        action,
        nodeId: "split-node",
        source: "editor_command",
      });
    }
  });

  it("freezes explicit node, resource, and Trash Item row identities", () => {
    const node = resolveWorkspaceActionTarget("move", { source: "explicit_node_row", surface: "contents", nodeId: "row-a" });
    const resource = resolveWorkspaceActionTarget("trash_resource", { source: "resource_row", ownerNodeId: "owner", resourceName: "asset.bin" });
    const item = resolveWorkspaceActionTarget("restore_item", { source: "trash_item_row", trashItemId: "item-id" });
    expect(node).toMatchObject({ kind: "node", nodeId: "row-a", surface: "contents" });
    expect(resource).toEqual({ kind: "resource", action: "trash_resource", ownerNodeId: "owner", resourceName: "asset.bin", source: "resource_row" });
    expect(item).toEqual({ kind: "trash_item", action: "restore_item", trashItemId: "item-id", source: "trash_item_row" });
    expect([node, resource, item].every(Object.isFrozen)).toBe(true);
  });

  it("keeps every action form limited to relevant fields", () => {
    expect(WORKSPACE_ACTION_REGISTRY.rename.fields).toEqual(["name"]);
    expect(WORKSPACE_ACTION_REGISTRY.move.fields).toEqual(["parentNodeId"]);
    expect(WORKSPACE_ACTION_REGISTRY.copy.fields).toEqual(["parentNodeId", "name"]);
    expect(WORKSPACE_ACTION_REGISTRY.trash_node.fields).toEqual([]);
    expect(WORKSPACE_ACTION_REGISTRY.restore_item.fields).toEqual(["restoreMode"]);
    expect(WORKSPACE_ACTION_REGISTRY.trash_resource.fields).toEqual(["resourceNames"]);
    expect(WORKSPACE_ACTION_REGISTRY.permanently_delete_item.fields).toEqual([]);
  });

  it("rejects missing or mismatched explicit identities", () => {
    expect(() => resolveWorkspaceActionTarget("copy", { source: "editor_command", focusedNodeId: " " })).toThrow(/明确 ID/u);
    expect(() => resolveWorkspaceActionTarget("restore_item", { source: "explicit_node_row", surface: "hierarchy", nodeId: "node" })).toThrow(/明确条目/u);
    expect(() => resolveWorkspaceActionTarget("trash_node", { source: "trash_item_row", trashItemId: "item" })).toThrow(/节点操作/u);
  });
});

describe("Core-reviewed dirty draft scope", () => {
  it("blocks only IDs in Core's exact draft-sensitive set", () => {
    const scope = freezeCoreReviewedDraftScope({ draftSensitiveNodeIds: ["source", "descendant", "link-owner"] });
    expect(conflictingDirtyNodeIds(scope, new Set(["unrelated", "descendant", "link-owner"]))).toEqual(["descendant", "link-owner"]);
  });

  it("rechecks the same frozen set and does not invent a target-parent dependency", () => {
    const scope = freezeCoreReviewedDraftScope({ draftSensitiveNodeIds: ["source"] });
    expect(conflictingDirtyNodeIds(scope, new Set(["target-parent"]))).toEqual([]);
    expect(conflictingDirtyNodeIds(scope, new Set(["source", "target-parent"]))).toEqual(["source"]);
  });

  it("sorts, copies, and freezes Core IDs while rejecting duplicate authority", () => {
    const ids = ["z", "a"];
    const scope = freezeCoreReviewedDraftScope({ draftSensitiveNodeIds: ids });
    ids.push("late");
    expect(scope.draftSensitiveNodeIds).toEqual(["a", "z"]);
    expect(Object.isFrozen(scope.draftSensitiveNodeIds)).toBe(true);
    expect(() => freezeCoreReviewedDraftScope({ draftSensitiveNodeIds: ["same", "same"] })).toThrow(/duplicate/u);
  });
});
