import test from "node:test";
import assert from "node:assert/strict";
import bundled from "../../docs/design/examples/schema.json" with { type: "json" };
import { schemaFieldLabel, schemaMemberLabel, schemaEnumLabel } from "./builtin-property-labels";
import { resolveShape, type DataSchema, type Shape } from "./native-value-form";
import { locale, uiTextValue } from "./ui-locale";

const installed = bundled as unknown as DataSchema;
const field = (schema: DataSchema, id: string) => schema.fields.find(field => field.id === id)!;
const text = uiTextValue;
const identity = { fieldId: "people/engagements", path: [{ kind: "index" as const, index: 1 }] };

test("builtin business captions follow actual field identity and preserve authored values in both languages", () => {
    const snapshot = JSON.stringify(installed);
    try {
        locale.set("zh-CN");
        const employment = schemaFieldLabel(installed, field(installed, "people/engagements"));
        const employer = schemaMemberLabel(installed, identity, "employer");
        assert.equal(text(employment), "任职记录"); assert.equal(text(employer), "任职组织");
        assert.equal(text(schemaMemberLabel(installed, { fieldId: "contacts/phones", path: [] }, "label")), "用途");
        assert.equal(text(schemaMemberLabel(installed, { fieldId: "library/publication", path: [] }, "roles")), "贡献角色");
        const phone = resolveShape(installed.definitions.Phone, installed);
        assert.equal(phone.kind, "record");
        if (phone.kind !== "record") return;
        const usage = resolveShape(phone.members.label.shape, installed);
        assert.equal(usage.kind, "enum");
        if (usage.kind !== "enum") return;
        assert.equal(text(schemaEnumLabel(installed, { fieldId: "contacts/phones", path: [] }, usage, "work")), "工作");
        assert.equal(text(schemaEnumLabel(installed, { fieldId: "contacts/phones", path: [] }, usage, "作者自定义")), "作者自定义");
        locale.set("en-US");
        assert.equal(text(employment), "Employment records"); assert.equal(text(employer), "Employer");
        assert.equal(text(schemaEnumLabel(installed, { fieldId: "contacts/phones", path: [] }, usage, "work")), "Work");
        assert.equal(JSON.stringify(installed), snapshot);
    } finally { locale.set("zh-CN"); }
});

test("custom same-name keys, renamed builtin keys, changed shapes and query parameters never borrow builtin meaning", () => {
    const custom = structuredClone(installed), original = field(custom, "people/engagements");
    original.id = "my/employment";
    assert.equal(text(schemaFieldLabel(custom, original)), "engagements");
    assert.equal(text(schemaMemberLabel(custom, { fieldId: original.id, path: [] }, "position")), "position");
    assert.equal(text(schemaMemberLabel(installed, { parameterId: "people/engagements", path: [] }, "employer")), "employer");
    const renamed = { ...field(installed, "people/engagements"), key: "my_records" };
    assert.equal(text(schemaFieldLabel(installed, renamed)), "my_records");
    const changed = structuredClone(installed);
    changed.definitions.Engagement = { kind: "record", members: { employer: { shape: { kind: "scalar", semantic: "int" }, required: true, nullable: false } }, unknown: "reject" };
    assert.equal(text(schemaFieldLabel(changed, field(changed, "people/engagements"))), "engagements");
    assert.equal(text(schemaMemberLabel(changed, identity, "employer")), "employer");
    const usage: Extract<Shape, { kind: "enum" }> = { kind: "enum", values: ["personal", "work", "other"], open: false };
    assert.equal(text(schemaEnumLabel(custom, { fieldId: original.id, path: [] }, usage, "work")), "work");
});

test("authorized expanded field profiles have captions without a whole-schema read; partial or unresolved shapes do not", () => {
    function expand(shape: Shape): Shape {
        const resolved = resolveShape(shape, installed);
        if (resolved.kind === "record") return { ...resolved, members: Object.fromEntries(Object.entries(resolved.members).map(([key, member]) => [key, { ...member, shape: expand(member.shape) }])) };
        if (resolved.kind === "list") return { ...resolved, items: expand(resolved.items) };
        if (resolved.kind === "union") return { ...resolved, variants: resolved.variants.map(expand) };
        return resolved;
    }
    const original = field(installed, "library/publication"), profile = { ...original, shape: expand(original.shape) };
    const authorized: DataSchema = { definitions: {}, fields: [profile], types: [], aliases: {} };
    assert.equal(text(schemaFieldLabel(authorized, profile)), "出版信息");
    assert.equal(text(schemaMemberLabel(authorized, { fieldId: profile.id, path: [] }, "contributors")), "作者与贡献者");
    assert.equal(text(schemaFieldLabel({ ...authorized, fields: [original] }, original)), "publication");
    const reordered = structuredClone(installed), target = reordered.definitions.Target;
    if (target.kind !== "union" || !target.discriminator) throw new Error("actual Target union");
    // A JSON producer may reorder object members without changing their meaning.
    target.discriminator = { keys: target.discriminator.keys, kind: target.discriminator.kind };
    assert.equal(text(schemaFieldLabel(reordered, field(reordered, "library/publication"))), "出版信息");
});
