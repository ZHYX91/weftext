import test from "node:test";
import assert from "node:assert/strict";
import { comparePresentedFields, decodeFieldPresentation, defaultFieldPresentation, effectiveFieldPlacement, setFieldPlacement } from "./data-field-preference";

test("field presentation accepts only current schema identities and bounded known placements", () => {
    const ids = new Set(["person/name", "person/phones", "__proto__"]);
    const decoded = decodeFieldPresentation('{"version":1,"sort":"name","placements":{"person/name":"top","person/phones":"hidden","unknown":"bottom","__proto__":"bottom"}}', ids);
    assert.deepEqual(Object.keys(decoded.placements), ["person/name", "person/phones", "__proto__"]);
    assert.equal(Object.getPrototypeOf(decoded.placements), Object.prototype);
    assert.equal(effectiveFieldPlacement(decoded, "constructor", false, false), "default");
    for (const raw of ["[]", "null", "not-json", '{"version":2,"sort":"schema","placements":{}}', '{"version":1,"sort":"schema","placements":[],"extra":true}', '{"version":1,"sort":"author","placements":{}}'])
        assert.deepEqual(decodeFieldPresentation(raw, ids), defaultFieldPresentation());
});

test("required, invalid and pending fields remain visible without clearing a stored hidden choice", () => {
    const preference = setFieldPlacement(defaultFieldPresentation(), "field", "hidden");
    assert.equal(effectiveFieldPlacement(preference, "field", false, false), "hidden");
    assert.equal(effectiveFieldPlacement(preference, "field", true, false), "default");
    assert.equal(effectiveFieldPlacement(preference, "field", false, true), "bottom");
    assert.equal(preference.placements.field, "hidden");
    assert.deepEqual(setFieldPlacement(preference, "field", "default"), defaultFieldPresentation());
});

test("local name sorting preserves equal-name order and never mutates schema order", () => {
    const fields = [{ name: "B", order: 0 }, { name: "A", order: 1 }, { name: "A", order: 2 }];
    const preference = { ...defaultFieldPresentation(), sort: "name" as const };
    assert.deepEqual([...fields].sort((a,b) => comparePresentedFields(preference,a,b)).map(item => item.order), [1,2,0]);
    assert.deepEqual(fields.map(item => item.order), [0,1,2]);
    assert.deepEqual([...fields].sort((a,b) => comparePresentedFields(defaultFieldPresentation(),a,b)).map(item => item.order), [0,1,2]);
});
