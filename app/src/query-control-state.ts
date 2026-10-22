import { message, setUiText, uiTextValue, type UiText } from "./ui-locale";
export interface QueryChoice {
    value: string;
    label: UiText;
    disabled?: boolean;
}
const retainedBases = new WeakMap<() => UiText, UiText>();
const optionLabels = new WeakMap<HTMLOptionElement, UiText>();
const baseLabel = (label: UiText): UiText => typeof label === "function" ? retainedBases.get(label) ?? label : label;
/** Permanent identity controls availability; display text is never parsed as state. */
export function retainQueryChoice(choices: QueryChoice[], selected: string, priorLabel?: UiText): {
    choices: QueryChoice[];
    unavailable: boolean;
} {
    if (!selected || choices.some(choice => choice.value === selected && !choice.disabled))
        return { choices, unavailable: false };
    const original = baseLabel(priorLabel ?? selected), label = () => message("queryChoice.unavailable", { label: uiTextValue(original) });
    retainedBases.set(label, original);
    return { choices: [{ value: selected, label, disabled: true }, ...choices.filter(choice => choice.value !== selected)], unavailable: true };
}
/** Change options in place. Neither the select nor an unchanged option is detached. */
export function updateQueryChoices(select: HTMLSelectElement, choices: QueryChoice[], selected = select.value): boolean {
    const selectedOption = Array.from(select.options).find(option => option.value === selected);
    const retained = retainQueryChoice(choices, selected, selectedOption ? optionLabels.get(selectedOption) ?? selectedOption.textContent ?? undefined : undefined);
    const existing = new Map(Array.from(select.options, option => [option.value, option])), wanted = new Set(retained.choices.map(choice => choice.value));
    for (let index = 0; index < retained.choices.length; index++) {
        const choice = retained.choices[index], option = existing.get(choice.value) ?? document.createElement("option");
        if (option.getAttribute("value") !== choice.value)
            option.value = choice.value;
        setUiText(option, choice.label);
        optionLabels.set(option, baseLabel(choice.label));
        option.disabled = !!choice.disabled;
        if (select.options[index] !== option)
            select.insertBefore(option, select.options[index] ?? null);
    }
    for (const option of Array.from(select.options))
        if (!wanted.has(option.value))
            option.remove();
    const value = selected || (choices.some(choice => choice.value === "") ? "" : choices[0]?.value ?? "");
    if (select.value !== value)
        select.value = value;
    select.setAttribute("aria-invalid", String(retained.unavailable));
    return !retained.unavailable;
}
