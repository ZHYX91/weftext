import type { DataSchema, NativeValue } from "./data-panel";
import type { ValuePath } from "./author-input";
import { registerNativeControl, type NativeControlAddress, type NativeControlBinding, type NativeControlIdentity } from "./native-control-identity";
import { LocaleScope, locale, message, t, type MessageKey, type MessageParams } from "./ui-locale";
import { advanceMinutes, changeAdvanceMinutes, changeClockMinute, clockFraction, composeInstantMinute, exactAdvance, exactClock, explicitOffset, instantMinute, instantOffset, newReminder, reminderMember, reminderText, setReminderMember } from "./reminder-values";

type ReminderKey = Extract<MessageKey, `reminder.${string}`>;
type LabelKey = { [K in ReminderKey]: keyof MessageParams<K> extends never ? K : never }[ReminderKey];
const text = (value: string): NativeValue => ({ kind: "text", value });
const path = (...keys: string[]): ValuePath[] => keys.map(key => ({ kind: "member", key }));
const samePath = (a: readonly ValuePath[], b: readonly ValuePath[]): boolean => a.length === b.length && a.every((part, at) => part.kind === b[at].kind && (part.kind === "member" ? part.key === (b[at] as Extract<ValuePath, { kind: "member" }>).key : part.index === (b[at] as Extract<ValuePath, { kind: "index" }>).index));

export interface ReminderRawInput { readonly path: readonly ValuePath[]; readonly role: string; readonly text: string }
export interface ReminderRawRestore {
  applied: ReminderRawInput[];
  rejected: { input: ReminderRawInput; reason: "duplicate" | "unknown" | "notText" | "unrepresentable" | "retired" }[];
}
export interface ReminderForm {
  element: HTMLElement;
  /** The owner has already qualified the same field/schema/proposal generation.
   * Copy-only or retired raw belongs in its recovery UI, never in this method. */
  restoreRaw(inputs: readonly ReminderRawInput[]): ReminderRawRestore;
  dispose(): void;
}

/** Native author intent only. Saved-source device consent remains separate. */
export function reminderForm(initial: NativeValue, schema: DataSchema, changed: (next: NativeValue) => void, identity: NativeControlIdentity, binding?: NativeControlBinding): ReminderForm {
  const element = document.createElement("div"); element.className = "task-reminder-form";
  const capturedIdentity = structuredClone(identity);
  const addresses = new Map<HTMLElement, NativeControlAddress>();
  const validators = new Map<HTMLInputElement, () => boolean>();
  const slots: { slot: string; path: readonly ValuePath[] }[] = [];
  const unbind: (() => void)[] = [];
  let value = structuredClone(initial), scope = new LocaleScope(), live = true;
  const invalid = (): string => t(message("reminder.invalidInput"));
  const unsubscribe = locale.subscribe(() => { for (const control of validators.keys()) if (control.validity.customError) control.setCustomValidity(invalid()); });
  const current = (control: HTMLElement): boolean => live && control.isConnected && element.contains(control) && !control.matches(":disabled") && !(control instanceof HTMLInputElement && control.readOnly);
  const publish = (next: NativeValue): void => { value = next; changed(next); };
  const field = (key: string, next: NativeValue): void => publish(setReminderMember(value, key, next));
  const register = (control: HTMLElement, member: readonly ValuePath[], role: string): void => {
    const address = registerNativeControl(control, { ...capturedIdentity, path: [...capturedIdentity.path, ...member] }, role);
    addresses.set(control, address);
    if (binding && control instanceof HTMLInputElement) {
      const slot = JSON.stringify([binding.slot, role]);
      slots.push({ slot, path: address.identity.path });
      unbind.push(binding.session.bind(control, slot, address.identity.path));
    }
  };
  const validate = (control: HTMLInputElement, valid: () => boolean): (() => boolean) => {
    const check = (): boolean => { control.setCustomValidity(""); const okay = valid() && control.validity.valid; if (!okay) control.setCustomValidity(invalid()); return okay; };
    validators.set(control, check); return check;
  };
  const validateAll = (): void => { for (const check of validators.values()) check(); };
  const label = (key: LabelKey, control: HTMLElement): HTMLLabelElement => {
    const label = document.createElement("label"), caption = document.createElement("span");
    label.className = "data-label"; control.classList.add("data-input");
    scope.bindText(caption, message(key)); scope.bindAttr(control, "aria-label", message(key)); label.append(caption, control); return label;
  };
  function input(key: LabelKey, member: readonly ValuePath[], role: string, initial: string, update: (next: string) => void, valid = (next: string): boolean => !!next): HTMLInputElement {
    const control = document.createElement("input"); control.type = "text"; control.value = initial; control.required = !valid("");
    register(control, member, role); element.append(label(key, control));
    const check = validate(control, () => valid(control.value));
    control.oninput = () => { if (current(control) && check()) { try { update(control.value); } catch { control.setCustomValidity(invalid()); } } };
    return control;
  }
  const invalidateBranch = (): void => { for (const item of slots) binding?.session.invalidate(item.slot, item.path); };
  function render(): void {
    unbind.splice(0).forEach(release => release()); slots.length = 0; addresses.clear(); validators.clear();
    scope.dispose(); scope = new LocaleScope(); element.replaceChildren();
    const mode = document.createElement("select"); register(mode, path("kind"), "select-kind");
    for (const [kind, key] of [["absolute", "reminder.absolute"], ["due_before", "reminder.beforeDue"], ["due_day", "reminder.dueDay"]] as const) { const option = document.createElement("option"); option.value = kind; scope.bindText(option, message(key)); mode.append(option); }
    mode.value = reminderText(value, "kind"); element.append(label("reminder.mode", mode));
    mode.onchange = () => {
      if (!current(mode)) return;
      const kind = mode.value; if (kind !== "absolute" && kind !== "due_before" && kind !== "due_day") return;
      invalidateBranch(); publish(newReminder(kind, value)); if (live) render();
    };
    switch (reminderText(value, "kind")) {
      case "absolute": {
        const minute = document.createElement("input"); minute.type = "datetime-local"; minute.step = "60"; minute.required = true; minute.value = instantMinute(reminderText(value, "at")) ?? "";
        register(minute, path("at"), "minute-helper"); element.append(label("reminder.minuteHelper", minute));
        const offset = input("reminder.offset", path("at"), "explicit-offset", instantOffset(reminderText(value, "at")) ?? "", () => combine(), explicitOffset); offset.placeholder = "+08:00 / Z";
        const exact = input("reminder.at", path("at"), "exact-instant", reminderText(value, "at"), next => {
          field("at", text(next)); minute.value = instantMinute(next) ?? ""; offset.value = instantOffset(next) ?? "";
          validators.get(minute)?.(); validators.get(offset)?.();
        }, next => instantMinute(next) !== undefined);
        exact.placeholder = "2026-10-23T17:00:12.123456789123+08:00";
        const checkMinute = validate(minute, () => {
          try { if (!exact.value && reminderText(value, "at")) return false; composeInstantMinute(minute.value, offset.value, exact.value); return true; } catch { return false; }
        });
        function combine(): void {
          if (!checkMinute()) return;
          const next = composeInstantMinute(minute.value, offset.value, exact.value);
          exact.value = next; validators.get(exact)?.(); field("at", text(next));
        }
        minute.oninput = () => { if (current(minute)) combine(); }; break;
      }
      case "due_before": {
        const exact = input("reminder.beforeSeconds", path("before"), "exact-seconds", reminderText(value, "before"), next => {
          field("before", text(next)); minutes.value = advanceMinutes(next) ?? ""; validators.get(minutes)?.();
        }, exactAdvance);
        const minutes = input("reminder.minutes", path("before"), "minute-helper", advanceMinutes(exact.value) ?? "", next => {
          const precise = changeAdvanceMinutes(exact.value, next); exact.value = precise; validators.get(exact)?.(); field("before", text(precise));
        }, next => { try { changeAdvanceMinutes(exact.value, next); return true; } catch { return false; } });
        minutes.inputMode = "numeric"; break;
      }
      case "due_day": {
        const clock = (): NativeValue => reminderMember(value, "clock") ?? { kind: "record", value: [] };
        const clockText = (): string => reminderText(clock(), "time");
        const exact = input("reminder.clock", path("clock", "time"), "exact-clock", clockText(), next => {
          field("clock", setReminderMember(clock(), "time", { kind: "time", value: next })); minute.value = next.slice(0, 5); validators.get(minute)?.();
        }, exactClock);
        const minute = document.createElement("input"); minute.type = "time"; minute.step = "60"; minute.required = true; minute.value = exactClock(clockText()) ? clockText().slice(0, 5) : "";
        register(minute, path("clock", "time"), "minute-helper"); element.append(label("reminder.minuteHelper", minute));
        const checkMinute = validate(minute, () => { try { changeClockMinute(exact.value, minute.value); return true; } catch { return false; } });
        minute.oninput = () => { if (!current(minute) || !checkMinute()) return; const next = changeClockMinute(exact.value, minute.value); exact.value = next; validators.get(exact)?.(); field("clock", setReminderMember(clock(), "time", { kind: "time", value: next })); };
        input("reminder.fraction", path("clock", "fraction"), "exact-fraction", reminderText(clock(), "fraction"), next => field("clock", setReminderMember(clock(), "fraction", next ? text(next) : undefined)), clockFraction);
        const zone = document.createElement("select"); register(zone, path("zone"), "installed-zone-version"); zone.required = true;
        const placeholder = document.createElement("option"); placeholder.value = ""; scope.bindText(placeholder, message("reminder.zone")); zone.append(placeholder);
        const providers = (schema as DataSchema & { providers?: { data: Record<string, unknown> }[] }).providers ?? [];
        const pair = JSON.stringify([reminderText(value, "zone"), reminderText(value, "zone_version")]);
        for (const provider of providers) {
          if (provider.data.kind !== "zone-table" || typeof provider.data.key !== "string" || typeof provider.data.version !== "string") continue;
          const option = document.createElement("option"); option.value = JSON.stringify([provider.data.key, provider.data.version]); option.textContent = `${provider.data.key} · ${provider.data.version}`; zone.append(option);
        }
        if (pair !== '["",""]' && ![...zone.options].some(option => option.value === pair)) { const option = document.createElement("option"); option.value = pair; option.textContent = `${reminderText(value, "zone")} · ${reminderText(value, "zone_version")}`; zone.append(option); }
        zone.value = pair === '["",""]' ? "" : pair; element.append(label("reminder.zoneVersion", zone));
        zone.onchange = () => { if (!current(zone) || !zone.value) return; const [key, version] = JSON.parse(zone.value) as [string, string]; publish(setReminderMember(setReminderMember(value, "zone", text(key)), "zone_version", text(version))); };
        const fold = document.createElement("select"); register(fold, path("fold"), "explicit-fold");
        for (const [choice, key] of [["", "reminder.foldNone"], ["earlier", "reminder.foldEarlier"], ["later", "reminder.foldLater"]] as const) { const option = document.createElement("option"); option.value = choice; scope.bindText(option, message(key)); fold.append(option); }
        fold.value = reminderText(value, "fold"); element.append(label("reminder.fold", fold));
        fold.onchange = () => { if (current(fold)) publish(setReminderMember(value, "fold", fold.value ? text(fold.value) : undefined)); }; break;
      }
    }
    const help = document.createElement("p"); help.className = "data-help"; scope.bindText(help, message("reminder.help")); element.append(help); validateAll();
  }
  function restoreRaw(inputs: readonly ReminderRawInput[]): ReminderRawRestore {
    const result: ReminderRawRestore = { applied: [], rejected: [] };
    const key = (item: ReminderRawInput): string => JSON.stringify([item.role, item.path.map(part => part.kind === "member" ? ["member", part.key] : ["index", part.index])]);
    const counts = new Map<string, number>(); for (const item of inputs) counts.set(key(item), (counts.get(key(item)) ?? 0) + 1);
    for (const item of inputs) {
      const reject = (reason: ReminderRawRestore["rejected"][number]["reason"]): void => { result.rejected.push({ input: item, reason }); };
      if (!live) { reject("retired"); continue; }
      if (counts.get(key(item)) !== 1) { reject("duplicate"); continue; }
      const found = [...addresses].find(([, address]) => item.role === address.role && samePath(item.path, address.identity.path));
      if (!found) { reject("unknown"); continue; }
      const [control, address] = found;
      if (!(control instanceof HTMLInputElement)) { reject("notText"); continue; }
      const original = control.value; control.value = item.text;
      if (control.value !== item.text) { control.value = original; reject("unrepresentable"); continue; }
      result.applied.push({ path: address.identity.path, role: address.role, text: item.text });
    }
    validateAll(); return result;
  }
  render();
  return { element, restoreRaw, dispose: () => { if (!live) return; live = false; unbind.splice(0).forEach(release => release()); slots.length = 0; addresses.clear(); validators.clear(); scope.dispose(); unsubscribe(); } };
}
