import { test } from "node:test";
import assert from "node:assert/strict";
import { advanceMinutes, changeAdvanceMinutes, changeClockMinute, changeInstantMinute, clockFraction, composeInstantMinute, exactAdvance, exactClock, explicitOffset, instantMinute, newReminder, reminderMember, reminderText, setReminderMember } from "./reminder-values";
import type { NativeValue } from "./data-panel";

test("all-day minute editing retains author record order, zone and nested precision", () => {
  const original: NativeValue = { kind: "record", value: [
    { key: "id", value: { kind: "text", value: "11111111-1111-4111-8111-111111111111" } },
    { key: "kind", value: { kind: "text", value: "due_day" } },
    { key: "clock", value: { kind: "record", value: [
      { key: "time", value: { kind: "time", value: "09:00:37" } },
      { key: "fraction", value: { kind: "text", value: "0.123456789123" } },
    ] } },
    { key: "zone", value: { kind: "text", value: "UTC" } },
    { key: "zone_version", value: { kind: "text", value: "fixed-utc.v1" } },
  ] };
  const before = structuredClone(original), expected = structuredClone(original);
  const clock = reminderMember(original, "clock")!;
  const next = setReminderMember(original, "clock", setReminderMember(clock, "time", { kind: "time", value: changeClockMinute(reminderText(clock, "time"), "09:01") }));
  const expectedClock = reminderMember(expected, "clock")!;
  assert.equal(expectedClock.kind, "record");
  if (expectedClock.kind !== "record") throw new Error("Expected authored clock record");
  expectedClock.value[0].value = { kind: "time", value: "09:01:37" };
  assert.deepEqual(next, expected);
  assert.deepEqual(original, before);
});

test("minute edits retain author seconds, arbitrary precision and the original explicit offset", () => {
  const original = "2026-10-23T17:00:12.123456789123+08:00";
  assert.equal(instantMinute(original), "2026-10-23T17:00");
  assert.equal(changeInstantMinute(original, "2026-10-24T16:30"), "2026-10-24T16:30:12.123456789123+08:00");
  assert.equal(changeClockMinute("09:00:12", "08:45"), "08:45:12");
  assert.equal(advanceMinutes("1812.123456789123"), "30");
  assert.equal(changeAdvanceMinutes("1812.123456789123", "20"), "1212.123456789123");
  assert.throws(() => changeInstantMinute("2026-10-23T17:00", "2026-10-24T16:30"));
  assert.equal(instantMinute("2026-10-23"), undefined);
});
test("initial minute selection requires an explicit offset and edits retain finer precision",()=>{
  assert.equal(composeInstantMinute("2026-10-23T16:30","+08:00"),"2026-10-23T16:30:00+08:00");
  assert.equal(composeInstantMinute("2026-10-24T08:45","+05:30","2026-10-23T17:00:12.123456789123+08:00"),"2026-10-24T08:45:12.123456789123+05:30");
  for(const offset of ["","-00:00","+24:00","+08:60"]){assert.equal(explicitOffset(offset),false);assert.throws(()=>composeInstantMinute("2026-10-23T09:00",offset));}
});
test("mode changes preserve the one reminder identity and require an explicit zone", () => {
  const absolute = newReminder("absolute"); const day = newReminder("due_day", absolute);
  assert.equal(reminderText(day, "id"), reminderText(absolute, "id"));
  assert.equal(reminderText(day, "zone"), ""); assert.equal(reminderText(day, "zone_version"), "");
  assert.notEqual(reminderText(newReminder("absolute"), "id"), reminderText(absolute, "id"));
});

test("advance helpers accept admitted signed zero and preserve the complete residual fraction", () => {
  for (const zero of ["-0", "-00.000", "000.000"]) { assert.equal(exactAdvance(zero), true); assert.equal(advanceMinutes(zero), "0"); }
  assert.equal(changeAdvanceMinutes("-00.000", "1"), "60.000");
  assert.equal(changeAdvanceMinutes("001812.123456789123", "20"), "1212.123456789123");
  for (const invalid of ["1812.", "-0.0001", "+0", "0e0", "128849018820.1"]) { assert.equal(exactAdvance(invalid), false); assert.equal(advanceMinutes(invalid), undefined); assert.throws(() => changeAdvanceMinutes(invalid, "1")); }
});

test("advance helper refuses precision or range overflow without truncating a valid original", () => {
  const original = "0." + "0".repeat(4093) + "1";
  assert.equal(original.length, 4096); assert.equal(exactAdvance(original), true);
  assert.equal(changeAdvanceMinutes(original, "0"), original);
  assert.throws(() => changeAdvanceMinutes(original, "1"));
  assert.equal(changeAdvanceMinutes("0", "2147483647"), "128849018820");
  assert.throws(() => changeAdvanceMinutes("0.1", "2147483647"));
  assert.throws(() => changeAdvanceMinutes("12.1", "2147483647"));
  assert.throws(() => changeAdvanceMinutes("0", "1".repeat(4097)));
});

test("instant and civil-clock helpers keep codec precision and reject guessing from malformed raw", () => {
  const original = "2024-02-29T17:00:12." + "1".repeat(4094) + "+08:00";
  assert.equal(instantMinute(original), "2024-02-29T17:00");
  assert.equal(composeInstantMinute("2026-10-23T08:45", "Z", original), "2026-10-23T08:45:12." + "1".repeat(4094) + "Z");
  assert.equal(instantMinute(original.replace("." + "1".repeat(4094), "." + "1".repeat(4095))), undefined);
  for (const raw of ["unfinished", "2026-02-29T09:00:00Z", "2026-10-23T09:00:60Z"]) assert.throws(() => composeInstantMinute("2026-10-23T08:45", "Z", raw));
  assert.equal(exactClock("09:00:60"), false); assert.throws(() => changeClockMinute("unfinished", "08:45"));
  for (const value of ["", "-0.000", "00.5", "0." + "1".repeat(4094)]) assert.equal(clockFraction(value), true);
  for (const value of ["1", "-0.1", "0." + "1".repeat(4095)]) assert.equal(clockFraction(value), false);
});
