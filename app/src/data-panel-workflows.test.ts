import test from "node:test";
import assert from "node:assert/strict";
import { DataPanel, type DataSchema, type DocumentBasis, type PanelHost, type SchemaRead } from "./data-panel";
import { ProposalCloseGuard } from "./proposal-close-guard";
import type { NoteProjection } from "./data-projection";
import type { FieldTransferContext } from "./field-transfer";

function fixture() {
    const basis: DocumentBasis = { inputId: "actual-input", sessionId: "actual-session", source: "original source", serial: 1, baseId: "actual-base", root: "D:/owned-library", path: "D:/owned-library/A.typ", workspaceId: "actual-library", nodeId: "actual-node", inputRevision: 2 };
    const schema: DataSchema = { definitions: {}, fields: [], aliases: {}, types: [] };
    const schemaRead: SchemaRead = { schema, source: "schema source", revision: "schema-revision", semanticDigest: "a".repeat(64), canInstall: false, builtinDefault: false };
    const projection: NoteProjection = { qualification: "qualified", id: "actual-node", types: [], resolvedTypes: [], fields: [], unknownFields: [], diagnostics: [], budgetExhausted: false, readOnly: false };
    const state = { current: { ...basis }, editable: true };
    const drafts = new Map(), typeDrafts = new Map(), schemaDrafts = new Map();
    const owner = Object.assign(Object.create(DataPanel.prototype), {
        host: { getDocument: () => state.current, canEdit: () => state.editable } as PanelHost,
        basis: { ...basis }, projectionBasis: { ...basis }, readingProjection: false, schemaRead, schema, projection,
        drafts, draftCache: new Map(), draftIdentity: "current-note", typeDrafts, typeInstalling: new Set(), schemaDrafts, schemaInstalling: new Set(),
        closeGuard: new ProposalCloseGuard(), composing: false, rememberInputs() {},
    }) as DataPanel;
    const internals = owner as unknown as { transferContext(): FieldTransferContext | null; isTransferCurrent(context: FieldTransferContext): boolean; canTransferFields(): boolean };
    return { owner, internals, state, basis, schemaRead, drafts, typeDrafts, schemaDrafts };
}

test("transfer authority never pairs a new basis or pending read with the old accepted projection", () => {
    const f = fixture(); assert.ok(f.internals.transferContext());
    Object.assign(f.owner, { readingProjection: true }); assert.equal(f.internals.transferContext(), null);
    Object.assign(f.owner, { readingProjection: false, basis: { ...f.basis, inputId: "new-input-same-bytes" } }); assert.equal(f.internals.transferContext(), null);
    Object.assign(f.owner, { basis: { ...f.basis } });
    for (const key of ["inputId", "baseId", "workspaceId", "nodeId"] as const) {
        f.state.current = { ...f.basis, [key]: "replacement" };
        assert.equal(f.internals.transferContext(), null, key);
    }
});

test("same-cut Draft ACK metadata advances without retiring or qualifying a source-read lease", () => {
    const f = fixture(), context = f.internals.transferContext()!;
    assert.equal(Object.hasOwn(context.basis, "inputRevision"), false);
    for (const inputRevision of [undefined, 2, 3, 99]) {
        f.state.current = { ...f.basis, inputRevision };
        const renewed = f.internals.transferContext(); assert.ok(renewed);
        assert.equal(Object.hasOwn(renewed.basis, "inputRevision"), false, "a projection cannot confer ACK authority");
        assert.equal(f.internals.isTransferCurrent(context), true);
        assert.equal(f.internals.canTransferFields(), true);
    }
    f.state.current = { ...f.basis, inputId: "new-author-same-bytes", inputRevision: 2 };
    assert.equal(f.internals.isTransferCurrent(context), false, "an equal ACK cannot revive an old author input");
});

test("an accepted transfer context retires on actual schema or projection qualification replacement", () => {
    const f = fixture(), context = f.internals.transferContext()!;
    assert.equal(f.internals.isTransferCurrent(context), true);
    Object.assign(f.owner, { schemaRead: { ...f.schemaRead, semanticDigest: "b".repeat(64) } }); assert.equal(f.internals.isTransferCurrent(context), false);
    Object.assign(f.owner, { schemaRead: { ...f.schemaRead, revision: "new-revision" } }); assert.equal(f.internals.isTransferCurrent(context), false);
    Object.assign(f.owner, { schemaRead: f.schemaRead, projection: { ...context.projection, readOnly: true } }); assert.equal(f.internals.isTransferCurrent(context), false);
    Object.assign(f.owner, { projection: context.projection, projectionBasis: null }); assert.equal(f.internals.transferContext(), null);
});

test("transfer rejects only the current document's pending proposals while preserving inactive owners", () => {
    const f = fixture();
    Object.assign(f.owner, { draftCache: new Map([["another-note", new Map([["x", { changed: true, rawInputs: { value: "-" } }]])]]) });
    f.typeDrafts.set("another-note", ["retained"]); f.schemaDrafts.set("another-library", { source: "{" });
    assert.equal(f.internals.canTransferFields(), true);
    for (const proposal of ["typed", "raw", "types", "schema"]) {
        f.drafts.clear(); f.typeDrafts.delete("current-note"); f.schemaDrafts.delete(f.basis.root);
        if (proposal === "typed") f.drafts.set("field", { changed: true });
        if (proposal === "raw") f.drafts.set("field", { changed: false, rawInputs: { value: "-" } });
        if (proposal === "types") f.typeDrafts.set("current-note", ["changed"]);
        if (proposal === "schema") f.schemaDrafts.set(f.basis.root, { source: "{" });
        assert.equal(f.internals.canTransferFields(), false, proposal);
    }
    assert.equal(f.typeDrafts.has("another-note"), true); assert.equal(f.schemaDrafts.has("another-library"), true);
});
