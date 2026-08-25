import { describe, expect, it } from "vitest";

import { isDocumentModel, isDocumentProfile, isDocumentViewModel, isNodeMetadataProjection, type DocumentCapabilities, type DocumentModel } from "../app/document-contract";

const capabilities: DocumentCapabilities = {
  exactSource: true,
  utf8SourceEdits: true,
  yamlEnvelope: true,
  maxHeadingLevel: 9,
  actualQuoteDepth: true,
  blockIds: true,
  managedLinks: true,
  protectedRegions: true,
  typedBlocks: true,
  typedInlines: true,
  nestedLists: true,
  typedTables: true,
  safeRenderInput: true,
  degradationReports: true,
};

function validModel(): DocumentModel {
  return {
    semanticModelVersion: 1,
    status: "complete",
    blocks: [{
      kind: "heading",
      start: 0,
      end: 8,
      textStart: 2,
      textEnd: 7,
      text: "Title",
      headingLevel: 1,
      quoteDepth: null,
      blockId: null,
      roles: [],
      title: null,
      semantic: { kind: "heading", level: 1 },
    }, {
      kind: "paragraph",
      start: 9,
      end: 13,
      textStart: 9,
      textEnd: 13,
      text: "Body",
      headingLevel: null,
      quoteDepth: null,
      blockId: null,
      roles: [],
      title: null,
      semantic: { kind: "paragraph" },
    }],
    inlines: [],
    runInGroups: [{ headingBlock: 0, bodyBlock: 1 }],
    diagnostics: [],
    degradations: [],
    safeHtml: "<article><h1>Title</h1><p>Body</p></article>",
  };
}

describe("Core AsciiDoc document wire contract", () => {
  it("accepts only the exact shallow Core node-metadata projection", () => {
    const projection = {
      schema: "weftext.node-metadata.v1",
      id: "550e8400-e29b-41d4-a716-446655440000",
      icon: "weftext:future-token",
      resolvedIcon: null,
      aliases: ["文缕", "Weftext Notes"],
      childSort: "manual",
      childSortDirection: "ascending",
      siblingRank: 2048,
      adjacentHeadingBody: "run_in",
      diagnostics: [{ code: "unknown_weftext_field", field: "future", range: { start: 10, end: 20 }, message: "preserved" }],
    };
    expect(isNodeMetadataProjection(projection)).toBe(true);
    expect(isNodeMetadataProjection({ ...projection, icon: ["weftext:book"] })).toBe(false);
    expect(isNodeMetadataProjection({ ...projection, recipe: { glyph: "书" } })).toBe(false);
    expect(isNodeMetadataProjection({ ...projection, siblingRank: 0 })).toBe(false);
    expect(isNodeMetadataProjection({ ...projection, resolvedIcon: { kind: "built_in", value: "weftext:book", glyph: "书" } })).toBe(false);
  });

  it("accepts the complete semantic-model v1 payload", () => {
    expect(isDocumentModel(validModel())).toBe(true);
  });

  it("rejects the retired shallow model and unknown kinds", () => {
    expect(isDocumentModel({ blocks: [], runInGroups: [], diagnostics: [] })).toBe(false);
    const unknown = structuredClone(validModel()) as unknown as Record<string, unknown>;
    (unknown.blocks as Array<Record<string, unknown>>)[0].kind = "future_active_block";
    expect(isDocumentModel(unknown)).toBe(false);
  });

  it("rejects semantic mismatches and invalid run-in references", () => {
    const mismatch = structuredClone(validModel());
    mismatch.blocks[0].semantic = { kind: "paragraph" };
    expect(isDocumentModel(mismatch)).toBe(false);

    const invalidGroup = structuredClone(validModel());
    invalidGroup.runInGroups = [{ headingBlock: 1, bodyBlock: 99 }];
    expect(isDocumentModel(invalidGroup)).toBe(false);
  });

  it("pins the complete profile and view contracts to v2/v1", () => {
    const profile = { contractVersion: 2, profile: "ascii_doc_v1", mediaType: "text/asciidoc", canonicalExtension: "adoc", capabilities };
    expect(isDocumentProfile(profile)).toBe(true);
    expect(isDocumentProfile({ ...profile, contractVersion: 1 })).toBe(false);
    expect(isDocumentProfile({ ...profile, capabilities: { ...capabilities, typedTables: false } })).toBe(false);

    const model = validModel();
    const view = {
      contractVersion: 2,
      profile: "ascii_doc_v1",
      capabilities,
      semanticModelVersion: model.semanticModelVersion,
      status: model.status,
      blocks: model.blocks,
      inlines: model.inlines,
      runInGroups: model.runInGroups,
      degradations: model.degradations,
      safeHtml: model.safeHtml,
    };
    expect(isDocumentViewModel(view)).toBe(true);
    expect(isDocumentViewModel({ ...view, semanticModelVersion: 2 })).toBe(false);
  });
});
