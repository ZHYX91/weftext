import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readNoteProjection, sameProjectionInput } from "./data-projection-read";
import type { DocumentBasis, PanelHost } from "./native-value-form";

const hash = (source: string) => createHash("sha256").update(source).digest("hex");
const basis: DocumentBasis = { inputId: "input-one", sessionId: "session-one", source: "中文😀\r\n原稿", serial: 1, baseId: "base-one", root: "library", path: "note.typ" };
const schema = { source: "{\"definitions\":{}}\n", revision: null as string | null };
const projection = { qualification: "qualified", id: null, types: ["task"], resolvedTypes: ["tasks/task"], fields: [], unknownFields: [], diagnostics: [], budgetExhausted: false };
const wire = () => ({ projection, serial: basis.serial, sourceHash: hash(basis.source), schemaHash: hash(schema.source), schemaRevision: schema.revision, readOnly: false });
const reply = (value: unknown): PanelHost["request"] => async <T>() => value as T;

test("local projection read binds the exact UTF-8 source, serial and independently read schema", async () => {
    const request: PanelHost["request"] = async <T>(path: string, body?: unknown) => {
        assert.equal(path, "/api/data/project");
        assert.deepEqual(body, { sessionId: basis.sessionId, source: basis.source, serial: basis.serial });
        return { ...wire(), readOnly: true } as T;
    };
    assert.deepEqual(await readNoteProjection(request, basis, schema), { ...projection, readOnly: true });
    assert.deepEqual(await readNoteProjection(reply(wire()), basis), { ...projection, readOnly: false });
});

test("well-shaped responses for a different source, serial or schema cannot qualify the submitted input", async () => {
    for (const change of [
        { serial: 2 }, { sourceHash: hash("later input") }, { schemaHash: hash("different schema") }, { schemaRevision: "later-revision" },
    ]) await assert.rejects(readNoteProjection(reply({ ...wire(), ...change }), basis, schema));
    await assert.rejects(readNoteProjection(reply(wire()), basis, { ...schema, revision: "expected-revision" }));
    await assert.rejects(readNoteProjection(reply(wire()), basis, { source: "later schema", revision: null }));
    // A frozen export has no separate schema read; the route's exact schema
    // metadata still passes the response decoder, without claiming a pairing.
    await assert.rejects(readNoteProjection(reply({ ...wire(), schemaHash: "invalid" }), basis));
});

test("pending projection reads keep the submitted source and schema even if caller objects change", async () => {
    const original = { ...basis }, originalSchema = { ...schema };
    let finish!: (value: unknown) => void;
    const pending = new Promise<unknown>(resolve => { finish = resolve; });
    const read = readNoteProjection((async () => pending) as PanelHost["request"], original, originalSchema);
    original.source = "later"; original.serial++; originalSchema.source = "changed"; originalSchema.revision = "changed";
    finish(wire());
    assert.deepEqual(await read, { ...projection, readOnly: false });
    assert.equal(sameProjectionInput(original, basis), false);
});

test("same-byte input ABA and document/session changes invalidate a projection consumer", () => {
    assert.equal(sameProjectionInput({ ...basis }, basis), true);
    for (const change of [{ inputId: "later-input" }, { sessionId: "later-session" }, { baseId: "later-base" }, { serial: 2 }, { source: "later" }, { path: "other.typ" }, { root: "other-library" }])
        assert.equal(sameProjectionInput({ ...basis, ...change }, basis), false);
    assert.equal(sameProjectionInput(null, basis), false);
});

for (const original of ["source", "schema"] as const) for (const [kind, incomplete] of [["high", "\ud800"], ["low", "\udc00"]] as const) {
    test(`a lone ${kind} surrogate in ${original} is rejected before a request or hash and never replaced`, async context => {
        const originalBasis = { ...basis, ...(original === "source" ? { source: incomplete } : {}) };
        const originalSchema = { ...schema, ...(original === "schema" ? { source: incomplete } : {}) };
        const replacementResponse = { ...wire(), ...(original === "source" ? { sourceHash: hash("\ufffd") } : { schemaHash: hash("\ufffd") }) };
        // Ordinary UTF-8 encoders would erase the distinction; the reader must
        // reject the original code units instead of authorizing this response.
        assert.equal(hash(incomplete), hash("\ufffd"));
        let requests = 0, digests = 0, adopted = 0;
        context.mock.method(crypto.subtle, "digest", async () => { digests++; throw new Error("No hashing before scalar validation"); });
        const request: PanelHost["request"] = async <T>() => { requests++; return replacementResponse as T; };
        await assert.rejects(readNoteProjection(request, originalBasis, originalSchema).then(() => { adopted++; }));
        assert.equal(requests, 0); assert.equal(digests, 0); assert.equal(adopted, 0);
        assert.equal(originalBasis.source, original === "source" ? incomplete : basis.source);
        assert.equal(originalSchema.source, original === "schema" ? incomplete : schema.source);
        assert.equal(originalBasis.inputId, basis.inputId);
    });
}

for (const source of ["\ufffd", "😀", "中文 العربية עברית\0\r\n"]) test(`valid scalar source and schema retain exact bytes: ${JSON.stringify(source)}`, async () => {
    const originalBasis = { ...basis, source }, originalSchema = { source, revision: null };
    let requests = 0;
    const request: PanelHost["request"] = async <T>(_path: string, body?: unknown) => {
        requests++; assert.deepEqual(body, { sessionId: basis.sessionId, source, serial: basis.serial });
        return { ...wire(), sourceHash: hash(source), schemaHash: hash(source) } as T;
    };
    assert.deepEqual(await readNoteProjection(request, originalBasis, originalSchema), { ...projection, readOnly: false });
    assert.equal(requests, 1); assert.equal(originalBasis.source, source); assert.equal(originalSchema.source, source);
});

test("projection request dispatch cannot replace the schema text already validated for hashing", async () => {
    const originalSchema = { ...schema };
    const request: PanelHost["request"] = async <T>() => {
        originalSchema.source = "\ud800"; originalSchema.revision = "later";
        return wire() as T;
    };
    assert.deepEqual(await readNoteProjection(request, basis, originalSchema), { ...projection, readOnly: false });
    assert.equal(originalSchema.source, "\ud800");
});
