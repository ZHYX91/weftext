import test from "node:test";
import assert from "node:assert/strict";
import { applyValuePreset, captureValuePresetLease, validValuePreset, valuePresetContext, valuePresetStorageKey, ValuePresetFault, ValuePresetStore, type ValuePresetMutationLock, type ValuePresetStorage } from "./value-presets";
import type { DataSchema, NativeValue, SchemaField, Shape } from "./native-value-form";

const text = (value: string): NativeValue => ({ kind: "text", value });
const scalar = (semantic = "text"): Shape => ({ kind: "scalar", semantic });
const presetText = (value: NativeValue): boolean => value.kind === "text";
const scope = { libraryId: "actual-library-a", schemaDigest: "a".repeat(64) };
const setup = (shape: Shape) => {
    const field: SchemaField = { id: "contact/phones", key: "phones", nullable: true, shape };
    const schema: DataSchema = { definitions: {}, fields: [field], types: [], aliases: {} };
    return { field, schema };
};
function storage(initial?: string): ValuePresetStorage & { values: Map<string, string>; writes: number; fail: boolean } {
    return { values: new Map(initial ? [["initial", initial]] : []), writes: 0, fail: false,
        getItem(key) { return this.values.get(key) ?? null; },
        setItem(key, value) { this.writes++; if (this.fail) throw new Error("quota"); this.values.set(key, value); },
    };
}
function mutationLocks(): ValuePresetMutationLock & { names: string[] } {
    const tails = new Map<string, Promise<void>>(), names: string[] = [];
    return { names, run<T>(name: string, operation: () => T | Promise<T>): Promise<T> {
        names.push(name);
        const result = (tails.get(name) ?? Promise.resolve()).then(operation);
        tails.set(name, result.then(() => {}, () => {}));
        return result;
    } };
}
function deferred(): { promise: Promise<void>; resolve(): void } {
    let resolve!: () => void;
    const promise = new Promise<void>(done => { resolve = done; });
    return { promise, resolve };
}
const fault = (reason: string) => (error: unknown): boolean => error instanceof ValuePresetFault && error.reason === reason;

test("scalar replacement retains exact UTF-16 text and one explicit complete value", () => {
    const { field, schema } = setup(scalar());
    const original = text("旧值"), preset = text("\ud800原样\r\n😀");
    const context = valuePresetContext(scope, schema, field, original);
    assert.equal(context.action, "replace-scalar");
    assert.deepEqual(applyValuePreset(scope, schema, field, original, preset, context), preset);
    assert.deepEqual(original, text("旧值"));
    assert.throws(() => applyValuePreset(scope, schema, field, original, { kind: "none" }, context), fault("invalidValue"));
});

test("append preserves duplicate occurrences, unknown old members, order and the original object", () => {
    const item: Shape = { kind: "record", members: { number: { shape: scalar("phone"), required: true, nullable: false } }, unknown: "preserve" };
    const { field, schema } = setup({ kind: "list", items: item, min: 0, max: null });
    const record = (suffix: string): NativeValue => ({ kind: "record", value: [{ key: "number", value: text("00138") }, { key: "old-unbound", value: text(suffix) }] });
    const original: NativeValue = { kind: "array", value: [record("first"), record("second")] };
    const preset: NativeValue = { kind: "record", value: [{ key: "number", value: text("00138") }] };
    const context = valuePresetContext(scope, schema, field, original);
    const once = applyValuePreset(scope, schema, field, original, preset, context);
    const twice = applyValuePreset(scope, schema, field, once, preset, context);
    assert.equal(twice.kind, "array"); if (twice.kind !== "array") return;
    assert.equal(twice.value.length, 4); assert.deepEqual(twice.value.slice(0, 2), original.value);
    assert.deepEqual(twice.value.slice(2), [preset, preset]);
    assert.equal(original.value.length, 2);
    assert.equal(validValuePreset(item, record("new unknown"), schema), false);
});

test("list max and explicit none never replace or clear the old container", () => {
    const { field, schema } = setup({ kind: "list", items: scalar(), min: 0, max: 1 });
    const draft: NativeValue = { kind: "array", value: [text("kept")] };
    const context = valuePresetContext(scope, schema, field, draft);
    assert.throws(() => applyValuePreset(scope, schema, field, draft, text("second"), context), fault("maxItems"));
    assert.throws(() => applyValuePreset(scope, schema, field, { kind: "none" }, text("new"), context), fault("targetUnavailable"));
    assert.deepEqual(draft, { kind: "array", value: [text("kept")] });
});

test("library identity, FieldId, complete schema digest and member semantics partition presets", () => {
    const { field, schema } = setup(scalar());
    const key = valuePresetStorageKey(valuePresetContext(scope, schema, field, text("x")));
    assert.notEqual(key, valuePresetStorageKey(valuePresetContext({ ...scope, libraryId: "actual-library-b" }, schema, field, text("x"))));
    assert.notEqual(key, valuePresetStorageKey(valuePresetContext({ ...scope, schemaDigest: "b".repeat(64) }, schema, field, text("x"))));
    const renamed = { ...field, id: "another/field" }, renamedSchema = { ...schema, fields: [renamed] };
    assert.notEqual(key, valuePresetStorageKey(valuePresetContext(scope, renamedSchema, renamed, text("x"))));
    assert.throws(() => valuePresetContext({ ...scope, schemaDigest: "" }, schema, field, text("x")), fault("unsupported"));
    const altered = { ...field, shape: scalar("phone") };
    assert.throws(() => valuePresetContext(scope, schema, altered, text("x")), fault("unsupported"));
});

test("actual nested member lease preserves unrelated unknown and duplicate parent content", () => {
    const { field, schema } = setup({ kind: "list", min: 0, max: null, items: { kind: "record", unknown: "preserve", members: { label: { shape: scalar(), required: true, nullable: false } } } });
    const row = (label: string): NativeValue => ({ kind: "record", value: [{ key: "label", value: text(label) }, { key: "unbound", value: text("keep") }] });
    const draft: NativeValue = { kind: "array", value: [row("first"), row("second")] };
    const path = [{ kind: "index" as const, index: 1 }, { kind: "member" as const, key: "label" }];
    const context = valuePresetContext(scope, schema, field, draft, path);
    assert.deepEqual(context.path, [{ kind: "each" }, { kind: "member", key: "label" }]);
    const lease = captureValuePresetLease(draft, path);
    const next = applyValuePreset(scope, schema, field, draft, text("replacement"), context, path, lease);
    assert.deepEqual(next, { kind: "array", value: [row("first"), row("replacement")] });
    assert.deepEqual(draft.value[1], row("second"));
    assert.throws(() => applyValuePreset(scope, schema, field, draft, text("x"), context, [{ kind: "index", index: 2 }]), fault("targetUnavailable"));
    const reordered: NativeValue = { kind: "array", value: [...draft.value].reverse() };
    assert.throws(() => applyValuePreset(scope, schema, field, reordered, text("x"), context, path, lease), fault("targetUnavailable"));
    assert.throws(() => applyValuePreset(scope, schema, field, draft, text("x"), context, [{ kind: "index", index: 0 }, { kind: "member", key: "label" }], lease), fault("targetUnavailable"));
});

test("ambiguous duplicate member positions and unknown schema paths cannot acquire a target", () => {
    const { field, schema } = setup({ kind: "record", unknown: "preserve", members: { name: { shape: scalar(), required: true, nullable: false } } });
    const value: NativeValue = { kind: "record", value: [{ key: "name", value: text("one") }, { key: "name", value: text("two") }] };
    assert.throws(() => valuePresetContext(scope, schema, field, value, [{ kind: "member", key: "name" }]), fault("targetUnavailable"));
    assert.throws(() => valuePresetContext(scope, schema, field, value, [{ kind: "member", key: "unknown" }]), fault("targetUnavailable"));
});

test("Node,Tag,Resource,Schema and unknown semantic strings cannot become preset identities", () => {
    for (const semantic of ["node_ref", "tag_ref", "resource_ref", "schema_ref", "config_revision", "future_identity"]) {
        const { field, schema } = setup(scalar(semantic));
        assert.throws(() => valuePresetContext(scope, schema, field, text("weftext-node://forged")), fault("unsupported"));
        assert.equal(validValuePreset(field.shape, text("weftext-node://forged"), schema), false);
        const list = setup({ kind: "list", items: scalar(semantic), min: 0, max: null });
        assert.throws(() => valuePresetContext(scope, list.schema, list.field, { kind: "array", value: [] }), fault("unsupported"));
    }
});

test("schema changes revalidate native kind, closed enum and exact decimal strings", () => {
    const { schema } = setup(scalar());
    assert.equal(validValuePreset({ kind: "enum", values: ["open"], open: false }, text("retired"), schema), false);
    assert.equal(validValuePreset(scalar("int"), text("2"), schema), false);
    assert.equal(validValuePreset(scalar("int"), { kind: "int", value: "9223372036854775807" }, schema), true);
    assert.equal(validValuePreset(scalar("int"), { kind: "int", value: "9223372036854775808" }, schema), false);
    assert.equal(validValuePreset(scalar("exact_decimal"), text("0.30000000000000000000000000000000001"), schema), true);
    assert.equal(validValuePreset(scalar("exact_decimal"), text("-"), schema), false);
    assert.equal(validValuePreset(scalar("date"), { kind: "date", value: "2026-02-29" }, schema), false);
    assert.equal(validValuePreset(scalar("date"), { kind: "date", value: "2024-02-29" }, schema), true);
});

test("named typed presets save, reload and delete independently of author data", async () => {
    const { field, schema } = setup(scalar());
    const context = valuePresetContext(scope, schema, field, text("author")), backend = storage();
    const locks = mutationLocks(), store = new ValuePresetStore(context, backend, presetText, locks), exact = text("name\r\n😀");
    const preset = await store.save("  工作  ", exact);
    assert.equal(preset.name, "工作"); assert.deepEqual(new ValuePresetStore(context, backend, presetText, locks).presets[0].value, exact);
    await assert.rejects(store.save("工作", text("replacement")), fault("nameExists"));
    await store.remove(preset.id); assert.deepEqual(store.presets, []);
    assert.equal(backend.writes, 2); assert.deepEqual(exact, text("name\r\n😀"));
});

test("bad,duplicate-key,wrong-context and invalid NativeValue JSON never become an empty writable store", () => {
    const { field, schema } = setup(scalar());
    const context = valuePresetContext(scope, schema, field, text("author")), key = valuePresetStorageKey(context);
    for (const raw of ["{", '{"version":1,"version":1,"context":{},"presets":[]}', JSON.stringify({ version: 1, context: { ...context, libraryId: "wrong" }, presets: [] }), JSON.stringify({ version: 1, context, presets: [{ id: "a".repeat(36), name: "bad", value: { kind: "text", value: 123 } }] })]) {
        const backend = storage(); backend.values.set(key, raw);
        assert.throws(() => new ValuePresetStore(context, backend, presetText, mutationLocks()), fault("badStorage"));
        assert.equal(backend.getItem(key), raw); assert.equal(backend.writes, 0);
    }
});

test("quota failure retains the old bytes and in-memory proposed value without success", async () => {
    const { field, schema } = setup(scalar());
    const context = valuePresetContext(scope, schema, field, text("author")), backend = storage(), store = new ValuePresetStore(context, backend, presetText, mutationLocks());
    await store.save("old", text("keep")); const old = backend.getItem(store.key); backend.fail = true;
    const candidate = text("unsaved exact\r\n😀");
    await assert.rejects(store.save("new", candidate), fault("storageFailed"));
    assert.equal(backend.getItem(store.key), old); assert.deepEqual(store.presets.map(item => item.name), ["old"]); assert.deepEqual(candidate, text("unsaved exact\r\n😀"));
});

test("a different producer's exact raw update blocks saving,deletion and stale adoption", async () => {
    const { field, schema } = setup(scalar());
    const context = valuePresetContext(scope, schema, field, text("author")), backend = storage(), locks = mutationLocks(), first = new ValuePresetStore(context, backend, presetText, locks);
    const item = await first.save("first", text("first")), second = new ValuePresetStore(context, backend, presetText, locks);
    await second.save("other tab", text("other")); const latest = backend.getItem(first.key);
    await assert.rejects(first.save("mine", text("mine")), fault("storageChanged"));
    await assert.rejects(first.remove(item.id), fault("storageChanged"));
    assert.throws(() => first.assertCurrent(), fault("storageChanged"));
    assert.equal(backend.getItem(first.key), latest);
});

test("uncertain readback never triggers a compensating overwrite or a second write", async () => {
    const { field, schema } = setup(scalar());
    const context = valuePresetContext(scope, schema, field, text("author"));
    let raw: string | null = null, reads = 0, writes = 0;
    const backend: ValuePresetStorage = { getItem() { if (++reads >= 3) throw new Error("read denied after write"); return raw; }, setItem(_key, value) { writes++; raw = value; } };
    const store = new ValuePresetStore(context, backend, presetText, mutationLocks());
    await assert.rejects(store.save("pending", text("kept candidate")), fault("storageFailed"));
    await assert.rejects(store.save("new", text("new")), fault("storageFailed"));
    assert.equal(writes, 1); assert.ok(raw); assert.deepEqual(store.presets, []);
});

test("same-key concurrent producers serialize before comparing their original raw token", async () => {
    const { field, schema } = setup(scalar());
    const context = valuePresetContext(scope, schema, field, text("author")), backend = storage(), locks = mutationLocks();
    const first = new ValuePresetStore(context, backend, presetText, locks), second = new ValuePresetStore(context, backend, presetText, locks);
    const firstSave = first.save("first tab", text("keep first"));
    const secondSave = assert.rejects(second.save("second tab", text("must not overwrite")), fault("storageChanged"));
    const saved = await firstSave; await secondSave;
    assert.equal(backend.writes, 1);
    assert.deepEqual(locks.names, [first.key, second.key]);
    assert.deepEqual(new ValuePresetStore(context, backend, presetText, locks).presets, [saved]);
    assert.deepEqual(first.presets, [saved]); assert.deepEqual(second.presets, []);
});

test("two queued mutations on one store cannot silently reuse its newly advanced raw token", async () => {
    const { field, schema } = setup(scalar());
    const context = valuePresetContext(scope, schema, field, text("author")), backend = storage(), locks = mutationLocks();
    const store = new ValuePresetStore(context, backend, presetText, locks), entered = deferred(), release = deferred();
    const holder = locks.run(store.key, async () => { entered.resolve(); await release.promise; });
    await entered.promise;
    const firstSave = store.save("first proposal", text("keep first"));
    const secondSave = assert.rejects(store.save("stale proposal", text("must not write")), fault("storageChanged"));
    release.resolve(); await holder;
    const saved = await firstSave; await secondSave;
    assert.equal(backend.writes, 1); assert.deepEqual(store.presets, [saved]);
    assert.deepEqual(new ValuePresetStore(context, backend, presetText, locks).presets, [saved]);
});

test("queued saving and deletion revalidate their owner inside the lock before any write", async () => {
    const { field, schema } = setup(scalar());
    const context = valuePresetContext(scope, schema, field, text("author")), backend = storage(), locks = mutationLocks();
    const store = new ValuePresetStore(context, backend, presetText, locks), saved = await store.save("existing", text("keep"));
    const original = backend.getItem(store.key), lockName = locks.names[0];
    for (const mutation of ["save", "remove"]) {
        const entered = deferred(), release = deferred();
        const holder = locks.run(lockName, async () => { entered.resolve(); await release.promise; });
        await entered.promise;
        let current = true, validations = 0;
        const validate = (): boolean => { validations++; return current; };
        const rejected = assert.rejects(mutation === "save" ? store.save("late", text("late"), validate) : store.remove(saved.id, validate), fault("targetUnavailable"));
        current = false;
        assert.equal(backend.writes, 1); release.resolve();
        await Promise.all([holder, rejected]);
        assert.ok(validations > 0); assert.equal(backend.writes, 1);
        assert.equal(backend.getItem(store.key), original); assert.deepEqual(store.presets, [saved]);
    }
});

test("missing mutation locks preserve readable and applicable presets while refusing all writes", async () => {
    const { field, schema } = setup(scalar());
    const original = text("author"), context = valuePresetContext(scope, schema, field, original), backend = storage(), locks = mutationLocks();
    const saved = await new ValuePresetStore(context, backend, presetText, locks).save("readable", text("configured"));
    const bytes = backend.getItem(valuePresetStorageKey(context)), store = new ValuePresetStore(context, backend, presetText, null);
    assert.equal(store.canWrite, false);
    assert.deepEqual(store.presets, [saved]); assert.doesNotThrow(() => store.assertCurrent());
    assert.deepEqual(applyValuePreset(scope, schema, field, original, store.presets[0].value, context), saved.value);
    await assert.rejects(store.save("blocked", text("must not write")), fault("lockUnavailable"));
    await assert.rejects(store.remove(saved.id), fault("lockUnavailable"));
    assert.equal(backend.writes, 1); assert.equal(backend.getItem(store.key), bytes); assert.deepEqual(store.presets, [saved]);
});

test("saving snapshots the complete candidate before waiting for its mutation lock", async () => {
    const item: Shape = { kind: "record", members: { number: { shape: scalar("phone"), required: true, nullable: false } }, unknown: "preserve" };
    const { field, schema } = setup({ kind: "list", items: item, min: 0, max: null });
    const context = valuePresetContext(scope, schema, field, { kind: "array", value: [] }), backend = storage(), locks = mutationLocks();
    const store = new ValuePresetStore(context, backend, value => validValuePreset(item, value, schema), locks);
    assert.equal(store.canWrite, true);
    const candidate: NativeValue = { kind: "record", value: [{ key: "number", value: text("00138") }] }, snapshot = structuredClone(candidate);
    const entered = deferred(), release = deferred();
    const holder = locks.run(valuePresetStorageKey(context), async () => { entered.resolve(); await release.promise; });
    await entered.promise;
    const pending = store.save("  captured name  ", candidate);
    candidate.value[0].value = text("later caller edit"); candidate.value.push({ key: "unknown", value: text("later caller member") });
    assert.equal(backend.writes, 0); release.resolve(); await holder;
    const saved = await pending;
    assert.equal(saved.name, "captured name"); assert.deepEqual(saved.value, snapshot);
    assert.deepEqual(new ValuePresetStore(context, backend, value => validValuePreset(item, value, schema), locks).presets[0].value, snapshot);
});

test("ExactDecimal candidates follow Core closed spelling and total input-length bounds", () => {
    const { schema } = setup(scalar());
    for (const semantic of ["exact_decimal", "exact_seconds"]) {
        for (const accepted of ["0", "-0.001", "0001.2300", "1".repeat(4096), `0.${"1".repeat(4094)}`]) {
            assert.equal(validValuePreset(scalar(semantic), text(accepted), schema), true, `${semantic}: ${accepted.length} bytes`);
        }
        for (const rejected of ["", "+1", ".5", "1.", "-", "1e3", "1\n", "1 ", "١", "1".repeat(4097), `0.${"1".repeat(4095)}`]) {
            assert.equal(validValuePreset(scalar(semantic), text(rejected), schema), false, `${semantic}: ${JSON.stringify(rejected.slice(0, 20))} (${rejected.length} bytes)`);
        }
    }
});

test("ExactInstant candidates require closed explicit-offset spelling and preserve 4094 fractional digits", () => {
    const { schema } = setup(scalar());
    for (const accepted of ["2026-10-29T12:34:56Z", "2024-02-29T23:59:59.0000000012345678900+23:59", "0001-01-01T00:00:00+00:00", `2026-10-29T12:34:56.${"1".repeat(4094)}-08:30`]) {
        assert.equal(validValuePreset(scalar("instant"), text(accepted), schema), true, `${accepted.slice(0, 32)} (${accepted.length} bytes)`);
    }
    for (const rejected of [
        "2026-10-29T12:34:56", "2026-10-29T12:34:56-00:00", "2026-10-29t12:34:56Z", "2026-10-29T12:34:56z", "2026-10-29 12:34:56Z",
        "0000-01-01T00:00:00Z", "2026-02-29T12:34:56Z", "2024-02-30T12:34:56Z", "2026-10-29T24:00:00Z", "2026-10-29T12:60:00Z", "2026-10-29T12:34:60Z",
        "2026-10-29T12:34:56+24:00", "2026-10-29T12:34:56+23:60", "2026-10-29T12:34:56+0:00", "2026-10-29T12:34:56+0000",
        "2026-10-29T12:34:56.Z", "2026-10-29T12:34:56.1.2Z", "2026-10-29T12:34:56.١Z", "2026-10-29T12:34:56Z\n", "2026-10-29T12:34:56Z ",
        `2026-10-29T12:34:56.${"1".repeat(4095)}Z`,
    ]) {
        assert.equal(validValuePreset(scalar("instant"), text(rejected), schema), false, `${JSON.stringify(rejected.slice(0, 36))} (${rejected.length} bytes)`);
    }
});

test("exact temporal and decimal preset storage and adoption preserve the authored spelling byte for byte", async () => {
    for (const [semantic, spelling] of [
        ["instant", `2024-02-29T23:59:59.${"1234567890".repeat(409)}1234+08:30`],
        ["exact_decimal", `-0.${"1234567890".repeat(409)}123`],
        ["exact_seconds", "-0000.0012300"],
    ]) {
        const { field, schema } = setup(scalar(semantic)), original = text(spelling), context = valuePresetContext(scope, schema, field, original);
        const backend = storage(), locks = mutationLocks(), accepts = (value: NativeValue): boolean => validValuePreset(field.shape, value, schema);
        const saved = await new ValuePresetStore(context, backend, accepts, locks).save("exact", original);
        const reloaded = new ValuePresetStore(context, backend, accepts, locks).presets[0];
        assert.deepEqual(saved.value, original); assert.deepEqual(reloaded.value, original);
        assert.equal(JSON.stringify(reloaded.value), JSON.stringify(original));
        assert.deepEqual(applyValuePreset(scope, schema, field, text("replaced"), reloaded.value, context), original);
    }
});
