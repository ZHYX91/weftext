import test from "node:test";
import assert from "node:assert/strict";
import { decodeFieldTransferPatch, fieldTransferSelection, fieldTransferSourceHash, transferContextKey, transferLists,
    type FieldTransferContext, type FieldTransferInputs } from "./field-transfer-state";
import type { NativeValue, Shape } from "./native-value-form";

const text = (value: string): NativeValue => ({ kind: "text", value });
const list: Shape = { kind: "list", items: { kind: "scalar", semantic: "text" }, min: 0, max: null };
function fixture(): FieldTransferContext {
    const source = '// 原文😀\r\n#metadata((id: "6f593c9c-1bf4-46d3-937d-81287fbd58c4", types: ("x",), a: ("same", "same", "tail"), b: ("dest",))) <weftext>\r\n正文';
    const schema = { definitions: {}, fields: [{ id: "x/a", key: "a", shape: list, nullable: false }, { id: "x/b", key: "b", shape: list, nullable: false }], types: [], aliases: {} };
    return { basis: { inputId: "input", sessionId: "session", source, serial: 7, baseId: "base", path: "D:/own/A.typ", root: "D:/own", workspaceId: "workspace", nodeId: "node" },
        schemaRead: { schema, source: JSON.stringify(schema), revision: "schema-revision", semanticDigest: "a".repeat(64), builtinDefault: false, canInstall: true },
        projection: { qualification: "qualified", id: "6f593c9c-1bf4-46d3-937d-81287fbd58c4", types: ["x"], resolvedTypes: ["x"], fields: [
            { fieldId: "x/a", key: "a", cell: { state: "value", value: { kind: "array", value: [text("same"), text("same"), text("tail")] } }, writable: true, sourceRange: { from: 1, to: 2 }, raw: '("same", "same", "tail")', unbound: [] },
            { fieldId: "x/b", key: "b", cell: { state: "value", value: { kind: "array", value: [text("dest")] } }, writable: true, sourceRange: { from: 3, to: 4 }, raw: '("dest",)', unbound: [] },
        ], unknownFields: [], diagnostics: [], budgetExhausted: false, readOnly: false } };
}
const inputs = (): FieldTransferInputs => ({ fromField: "x/a", toField: "x/b", first: "2", last: "3", position: "2" });

test("transfer selection binds permanent IDs and exact consecutive ordinals, including duplicate values and an empty destination", () => {
    const context = fixture();
    assert.deepEqual(fieldTransferSelection(context, inputs()), { fromField: "x/a", toField: "x/b", start: 1, end: 2, to: 1 });
    const empty = context.projection.fields[1].cell;
    assert.equal(empty.state, "value");
    if (empty.state === "value" && empty.value.kind === "array") empty.value.value = [];
    assert.deepEqual(fieldTransferSelection(context, { ...inputs(), first: "1", last: "1", position: "1" }), { fromField: "x/a", toField: "x/b", start: 0, end: 0, to: 0 });
    for (const value of ["", "0", "-1", "1.5", "1e0", " 1", "1 ", "01", "9007199254740993"])
        for (const key of ["first", "last", "position"] as const) assert.throws(() => fieldTransferSelection(context, { ...inputs(), position: "1", [key]: value }), /序号|位置/);
    assert.throws(() => fieldTransferSelection(context, { ...inputs(), first: "3", last: "2", position: "1" }));
    assert.throws(() => fieldTransferSelection(context, { ...inputs(), toField: "x/a" }), /不同字段/);
    assert.throws(() => fieldTransferSelection(context, { ...inputs(), fromField: "a" }), /静态列表/);
});

test("list choices exclude raw, absent, none, invalid, derived, readonly and unbound slots without manufacturing a list", () => {
    const mutate = [
        (context: FieldTransferContext) => context.projection.readOnly = true,
        (context: FieldTransferContext) => context.projection.budgetExhausted = true,
        (context: FieldTransferContext) => context.projection.qualification = "unqualified",
    ];
    for (const change of mutate) { const context = fixture(); change(context); assert.deepEqual(transferLists(context), []); }
    for (const change of [
        (context: FieldTransferContext) => context.projection.fields[0].writable = false,
        (context: FieldTransferContext) => context.projection.fields[0].unbound = ["unknown"],
        (context: FieldTransferContext) => context.projection.fields[0].sourceRange = null,
        (context: FieldTransferContext) => context.projection.fields[0].raw = null,
        (context: FieldTransferContext) => context.projection.fields[0].cell = { state: "absent" },
        (context: FieldTransferContext) => context.projection.fields[0].cell = { state: "null" },
        (context: FieldTransferContext) => context.projection.fields[0].cell = { state: "invalid", value: null, diagnostics: [] },
        (context: FieldTransferContext) => context.schemaRead.schema.fields[0].shape = { kind: "scalar", semantic: "text" },
    ]) { const context = fixture(); change(context); assert.deepEqual(transferLists(context).map(field => field.fieldId), ["x/b"]); }
});

test("authority keys change on document, input, schema, permission and source changes", () => {
    const original = fixture(), key = transferContextKey(original);
    for (const change of [
        (context: FieldTransferContext) => context.basis.inputId += "new",
        (context: FieldTransferContext) => context.basis.baseId += "new",
        (context: FieldTransferContext) => context.basis.serial++,
        (context: FieldTransferContext) => context.basis.source += "changed",
        (context: FieldTransferContext) => context.basis.root += "new",
        (context: FieldTransferContext) => context.basis.nodeId += "new",
        (context: FieldTransferContext) => context.basis.sessionId += "new",
        (context: FieldTransferContext) => context.basis.path += "new",
        (context: FieldTransferContext) => context.basis.workspaceId += "new",
        (context: FieldTransferContext) => context.schemaRead.revision += "new",
        (context: FieldTransferContext) => context.schemaRead.semanticDigest = "b".repeat(64),
        (context: FieldTransferContext) => context.projection.fields[1].writable = false,
    ]) { const changed = structuredClone(original); change(changed); assert.notEqual(transferContextKey(changed), key); }
    for (const inputRevision of [undefined, 2, 3]) {
        const changed = { ...original, basis: { ...original.basis, inputRevision } };
        assert.equal(transferContextKey(changed), key, "a source transfer is independent of the Core Draft ACK");
    }
});

test("one exact UTF16 patch is tied to the old source, both IDs, ordinal selection and semantic schema", async () => {
    const context = fixture(), source = context.basis.source, selection = fieldTransferSelection(context, { ...inputs(), last: "2" });
    const sourceHash = await fieldTransferSourceHash(source), from = source.indexOf('("same"'), to = source.indexOf('("dest",)') + '("dest",)'.length;
    const text = '("same", "tail",), b: ("dest", "same",)';
    const response = { source: source.slice(0, from) + text + source.slice(to), patches: [{ from, to, text, byteFrom: new TextEncoder().encode(source.slice(0, from)).length, byteTo: new TextEncoder().encode(source.slice(0, to)).length, expected: source.slice(from, to) }],
        patchUnit: "utf16", serial: context.basis.serial, sourceHash, schemaRevision: context.schemaRead.revision, semanticDigest: context.schemaRead.semanticDigest, transfer: selection };
    assert.deepEqual(decodeFieldTransferPatch(response, context, selection, sourceHash), { from, to, text });
    for (const change of [
        (wire: typeof response) => wire.serial++,
        (wire: typeof response) => wire.sourceHash = "wrong",
        (wire: typeof response) => wire.schemaRevision = "wrong",
        (wire: typeof response) => wire.semanticDigest = "wrong",
        (wire: typeof response) => wire.transfer.start--,
        (wire: typeof response) => wire.transfer.toField = "same-name-other-id",
        (wire: typeof response) => wire.patches.push(wire.patches[0]),
        (wire: typeof response) => wire.patches[0].expected = "wrong",
        (wire: typeof response) => wire.patches[0].byteFrom--,
        (wire: typeof response) => wire.source += "unrelated",
        (wire: typeof response) => wire.patchUnit = "bytes",
    ]) { const wire = structuredClone(response); change(wire); assert.throws(() => decodeFieldTransferPatch(wire, context, selection, sourceHash)); }
    assert.throws(() => decodeFieldTransferPatch({ ...response, patches: [] }, context, selection, sourceHash));
});
