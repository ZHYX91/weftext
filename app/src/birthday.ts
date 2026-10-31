import type { NativeValue } from "./native-values";

const member = (value: NativeValue | undefined, key: string): NativeValue | undefined =>
    value?.kind === "record" ? value.value.find(item => item.key === key)?.value : undefined;
const text = (value: NativeValue | undefined): string | undefined => value?.kind === "text" ? value.value : undefined;

/** Ordinals belong to this exact list, not a durable identity or a merged fact. */
export function birthEvents(value: NativeValue): { index: number; selected: boolean; time?: NativeValue }[] {
    return value.kind !== "array" ? [] : value.value.flatMap((item, index) => {
        if (text(member(item, "kind")) !== "birth") return [];
        const selection = member(item, "selected");
        return [{ index, selected: selection?.kind === "bool" && selection.value, time: member(item, "event_time") }];
    });
}

/** Explicitly choose one birth occurrence, retaining every event and its evidence. */
export function selectBirthEvent(value: NativeValue, index: number): NativeValue {
    if (value.kind !== "array" || !birthEvents(value).some(item => item.index === index)) return value;
    return { kind: "array", value: value.value.map((item, ordinal) => {
        if (item.kind !== "record" || text(member(item, "kind")) !== "birth") return item;
        const prior = member(item, "selected");
        if (ordinal !== index && !(prior?.kind === "bool" && prior.value)) return item;
        const selected: NativeValue = { kind: "bool", value: ordinal === index };
        return { kind: "record", value: prior
            ? item.value.map(entry => entry.key === "selected" ? { ...entry, value: selected } : entry)
            : [...item.value, { key: "selected", value: selected }] };
    }) };
}

/** A new event has an unknown date until the author supplies it in the real form. */
export function appendBirthEvent(value: NativeValue): NativeValue {
    if (value.kind !== "array") return value;
    return { kind: "array", value: [...value.value, { kind: "record", value: [
        { key: "kind", value: { kind: "text", value: "birth" } },
        { key: "selected", value: { kind: "bool", value: false } },
    ] }] };
}

export function birthTimePrecision(value: NativeValue | undefined): "unknown" | "monthDay" | "year" | "month" | "day" | "exact" {
    if (!value || value.kind === "none") return "unknown";
    if (value.kind !== "record") return "exact";
    if (!member(value, "calendar")) return "monthDay";
    const precision = text(member(value, "precision"));
    return precision === "year" || precision === "month" || precision === "day" ? precision : "unknown";
}

export function birthTimeIsLeapMonth(value: NativeValue | undefined): boolean {
    const leap = member(value, "leap_month");
    return leap?.kind === "bool" && leap.value;
}

/** Display only supplied date parts. In particular a year never becomes Jan 1. */
export function birthTimeText(value: NativeValue | undefined): string | undefined {
    if (!value || value.kind === "none") return;
    if (value.kind !== "record") return "value" in value && typeof value.value === "string" ? value.value : undefined;
    const precision = birthTimePrecision(value);
    const keys = precision === "monthDay" ? ["month", "day"] : precision === "year" ? ["year"]
        : precision === "month" ? ["year", "month"] : precision === "day" ? ["year", "month", "day"] : [];
    if (!keys.length) return;
    const parts = keys.map(key => member(value, key));
    if (parts.some(part => part?.kind !== "int")) return;
    const date = parts.map(part => part?.kind === "int" ? part.value : "").join(" / ");
    const calendar = text(member(value, "calendar")), version = text(member(value, "version"));
    return calendar ? `${date} (${calendar}${version ? ` / ${version}` : ""})` : date;
}
