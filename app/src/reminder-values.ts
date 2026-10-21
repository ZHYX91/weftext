import type { NativeValue } from "./data-panel";

export type ReminderKind = "absolute" | "due_before" | "due_day";
export function reminderMember(value: NativeValue, key: string): NativeValue | undefined { return value.kind === "record" ? value.value.find(member => member.key === key)?.value : undefined; }
export function reminderText(value: NativeValue, key: string): string { const member = reminderMember(value, key); return member && "value" in member && typeof member.value === "string" ? member.value : ""; }
export function setReminderMember(value: NativeValue, key: string, next: NativeValue | undefined): NativeValue {
  if (value.kind !== "record") throw new Error("Reminder requires a native record");
  const members = value.value.filter(member => member.key !== key); if (next) members.push({ key, value: next }); return { kind: "record", value: members };
}
export function newReminder(kind: ReminderKind, previous?: NativeValue): NativeValue {
  const id = previous ? reminderText(previous, "id") : "";
  const text = (value: string): NativeValue => ({ kind: "text", value });
  const value: NativeValue = { kind: "record", value: [{ key: "id", value: text(id || crypto.randomUUID()) }, { key: "kind", value: text(kind) }] };
  const fields: [string, NativeValue][] = kind === "absolute" ? [["at", text("")]] : kind === "due_before" ? [["before", text("1800")]] : [["clock", { kind: "record", value: [{ key: "time", value: { kind: "time", value: "09:00:00" } }] }], ["zone", text("")], ["zone_version", text("")]];
  value.value.push(...fields.map(([key, value]) => ({ key, value }))); return value;
}
/** An auxiliary minute edit replaces only date/hour/minute. No Date/Number,
 * implicit offset, rounding or truncation enters the author value. */
function civilMinute(value: string): boolean {
  const match = /^(\d{4})-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])T(?:[01]\d|2[0-3]):[0-5]\d$/.exec(value);
  if (!match || match[1] === "0000") return false;
  const year = BigInt(match[1]), leap = year % 4n === 0n && (year % 100n !== 0n || year % 400n === 0n);
  const lastDay = match[2] === "02" ? leap ? "29" : "28" : ["04", "06", "09", "11"].includes(match[2]) ? "30" : "31";
  return match[3] <= lastDay;
}
function instantParts(value: string): { minute: string; precision: string; offset: string } | undefined {
  const match = /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2})(:[0-5]\d(?:\.(\d+))?)(Z|[+-]\d{2}:\d{2})$/.exec(value);
  if (!match || !civilMinute(match[1]) || !explicitOffset(match[4]) || (match[3]?.length ?? 0) > 4094) return;
  return { minute: match[1], precision: match[2], offset: match[4] };
}
export function instantMinute(value: string): string | undefined { return instantParts(value)?.minute; }
export function instantOffset(value: string): string | undefined { return instantParts(value)?.offset; }
export function explicitOffset(value: string): boolean { return value !== "-00:00" && /^(?:Z|[+-](?:[01]\d|2[0-3]):[0-5]\d)$/.test(value); }
export function composeInstantMinute(minute: string, offset: string, original = ""): string {
  const parts = original ? instantParts(original) : undefined;
  if (!civilMinute(minute) || !explicitOffset(offset) || original && !parts) throw new Error("A complete minute, valid original precision and explicit offset are required");
  return minute + (parts?.precision ?? ":00") + offset;
}
export function changeInstantMinute(original: string, minute: string): string {
  const parts = instantParts(original);
  if (!parts) throw new Error("Minute helper requires an existing exact instant and explicit offset");
  return composeInstantMinute(minute, parts.offset, original);
}
export function exactClock(value: string): boolean { return /^(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d$/.test(value); }
export function changeClockMinute(original: string, minute: string): string {
  if (!exactClock(original) || !/^(?:[01]\d|2[0-3]):[0-5]\d$/.test(minute)) throw new Error("Minute helper requires an existing seconds value");
  return minute + original.slice(5);
}
function nonnegativeDecimal(value: string): { whole: bigint; fraction: string } | undefined {
  if (!value || value.length > 4096) return;
  const match = /^(-?)(\d+)(\.\d+)?$/.exec(value);
  if (!match || match[1] && /[1-9]/.test(match[2] + (match[3] ?? ""))) return;
  return { whole: BigInt(match[2]), fraction: match[3] ?? "" };
}
export function exactAdvance(value: string): boolean {
  const parts = nonnegativeDecimal(value);
  return !!parts && (parts.whole < 128849018820n || parts.whole === 128849018820n && !/[1-9]/.test(parts.fraction));
}
export function clockFraction(value: string): boolean { return value === "" || nonnegativeDecimal(value)?.whole === 0n; }
export function advanceMinutes(value: string): string | undefined {
  const parts = nonnegativeDecimal(value);
  return parts && exactAdvance(value) ? (parts.whole / 60n).toString() : undefined;
}
export function changeAdvanceMinutes(original: string, minutes: string): string {
  const parts = nonnegativeDecimal(original);
  if (!parts || !exactAdvance(original) || minutes.length > 4096 || !/^\d+$/.test(minutes)) throw new Error("Minute helper requires a nonnegative exact advance");
  const next = (BigInt(minutes) * 60n + parts.whole % 60n).toString() + parts.fraction;
  if (!exactAdvance(next)) throw new Error("Minute edit exceeds the exact advance range or precision");
  return next;
}
