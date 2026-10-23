import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { locale, parseUiMessageJson, renderUiMessage, renderUiTemplate, uiTemplateParameterNames, type UiLocale } from "./ui-locale";

interface Oracle {
    wire: {name:string; json:string; valid:boolean; canonical?:unknown}[];
    templates: {name:string; template:string; params:[string,string][]; placeholders:string[]; expected:string}[];
    locales: {name:string; value:unknown; json:string; valid:boolean}[];
}
const oracle = JSON.parse(fs.readFileSync(new URL("../../crates/weftext-messages/tests/wire_oracles.json", import.meta.url), "utf8")) as Oracle;

test("the actual raw Node message boundary agrees with the shared Rust wire oracle", () => {
    for (const row of oracle.wire) {
        const actual = parseUiMessageJson(row.json);
        assert.equal(!!actual, row.valid, row.name);
        if (actual) assert.deepEqual(JSON.parse(JSON.stringify(actual)), row.canonical, row.name);
    }
});

test("actual UI templates agree with the shared literal-code and single-pass oracle", () => {
    for (const row of oracle.templates) {
        assert.deepEqual(uiTemplateParameterNames(row.template), row.placeholders, row.name);
        const params = new Map(row.params);
        assert.equal(renderUiTemplate(row.template, name => {
            const value = params.get(name); assert.notEqual(value, undefined, row.name); return value!;
        }), row.expected, row.name);
    }
    const wire = parseUiMessageJson('{"key":"common.operation","params":{"operationId":"{count}\\u0000😀"}}')!;
    assert.equal(renderUiMessage("en-US", wire), "Original operation: {count}\0😀");
});

test("explicit UI locale changes use the same closed locale domain as Rust", () => {
    for (const row of oracle.locales) {
        for (const value of [row.value, JSON.parse(row.json)]) {
            if (row.valid) { locale.set(value as UiLocale); assert.equal(locale.current, row.value, row.name); }
            else assert.throws(() => locale.set(value as UiLocale), /Unsupported/, row.name);
        }
    }
    locale.set("zh-CN");
});
