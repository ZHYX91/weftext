import assert from "node:assert/strict";
import test from "node:test";
import { appendBirthEvent, birthEvents, birthTimeIsLeapMonth, birthTimePrecision, birthTimeText, selectBirthEvent } from "./birthday";
import type { NativeValue } from "./native-values";
const text = (value: string): NativeValue => ({ kind: "text", value });
const record = (value: Record<string, NativeValue>): NativeValue => ({ kind: "record", value: Object.entries(value).map(([key, value]) => ({ key, value })) });
const selected: NativeValue = { kind: "bool", value: true };
test("birth projection never resolves conflicting selections or prefers the last event", () => {
    const value: NativeValue = { kind: "array", value: [record({ kind: text("birth"), selected, note: text("source A") }), record({ kind: text("death"), selected }), record({ kind: text("birth"), selected, imported: text("original") })] };
    const original = structuredClone(value);
    assert.deepEqual(birthEvents(value).map(item => [item.index, item.selected]), [[0, true], [2, true]]);
    const next = selectBirthEvent(value, 0);
    assert.deepEqual(value, original);
    assert.deepEqual(birthEvents(next).map(item => [item.index, item.selected]), [[0, true], [2, false]]);
    if (next.kind !== "array") throw new Error();
    assert.deepEqual(next.value[1], value.value[1]);
    assert.deepEqual(next.value[2], record({ kind: text("birth"), selected: { kind: "bool", value: false }, imported: text("original") }));
    assert.equal(selectBirthEvent(value, 1), value);
    assert.equal(selectBirthEvent(value, -1), value);
});
test("adding a birth record invents no date and never replaces the prior selection", () => {
    const value: NativeValue = { kind: "array", value: [record({ kind: text("birth"), selected, event_time: { kind: "date", value: "2000-02-29" } })] };
    const next = appendBirthEvent(value);
    assert.deepEqual(birthEvents(next), [{ index: 0, selected: true, time: { kind: "date", value: "2000-02-29" } }, { index: 1, selected: false, time: undefined }]);
    if (next.kind !== "array") throw new Error();
    assert.deepEqual(next.value[0], value.value[0]);
    assert.equal(value.value.length, 1);
});
test("birthday precision is projected without converting calendar, timezone or missing date parts", () => {
    for (const precision of ["year", "month", "day"] as const) {
        const time = record({ calendar: text("chinese"), version: text("original"), precision: text(precision), year: { kind: "int", value: "2020" } });
        const before = structuredClone(time);
        assert.equal(birthTimePrecision(time), precision);
        if (precision === "year") assert.equal(birthTimeText(time), "2020 (chinese / original)");
        else assert.equal(birthTimeText(time), undefined);
        assert.deepEqual(time, before);
    }
    assert.equal(birthTimePrecision(record({ month: { kind: "int", value: "2" }, day: { kind: "int", value: "29" } })), "monthDay");
    assert.equal(birthTimePrecision(undefined), "unknown");
    assert.equal(birthTimeText(undefined), undefined);
    assert.equal(birthTimeText(record({ month: { kind: "int", value: "2" }, day: { kind: "int", value: "29" } })), "2 / 29");
    assert.equal(birthTimePrecision({ kind: "text", value: "2000-01-01T00:30:00+14:00" }), "exact");
});

test("birthday leap-month marker preserves the supplied calendar distinction", () => {
    const value = record({ calendar: text("chinese"), precision: text("month"), leap_month: { kind: "bool", value: true } });
    const before = structuredClone(value);
    assert.equal(birthTimeIsLeapMonth(value), true);
    assert.equal(birthTimeIsLeapMonth(record({ leap_month: { kind: "bool", value: false } })), false);
    assert.equal(birthTimeIsLeapMonth(record({})), false);
    assert.equal(birthTimeIsLeapMonth(undefined), false);
    const monthDay = record({ month: { kind: "int", value: "2" }, day: { kind: "int", value: "29" }, leap_month: { kind: "bool", value: true } });
    assert.equal(birthTimePrecision(monthDay), "monthDay");
    assert.equal(birthTimeIsLeapMonth(monthDay), false, "unbound MonthDay members have no CalendarDate meaning");
    for (const precision of ["month", "day"]) {
        for (const flag of [true, false, undefined]) {
            const calendarDate = record({ calendar: text("chinese"), precision: text(precision), ...(flag === undefined ? {} : { leap_month: { kind: "bool" as const, value: flag } }) });
            assert.equal(birthTimeIsLeapMonth(calendarDate), flag === true);
        }
    }
    assert.deepEqual(value, before);
});
