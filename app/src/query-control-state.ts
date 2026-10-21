export interface QueryChoice { value: string; label: string; disabled?: boolean }

/** A removed permanent identity remains visible; a same-label successor is not a selection. */
export function retainQueryChoice(choices: QueryChoice[], selected: string, priorLabel?: string): { choices: QueryChoice[]; unavailable: boolean } {
  if (!selected || choices.some(choice => choice.value === selected && !choice.disabled)) return { choices, unavailable: false };
  return { choices: [{ value: selected, label: `${priorLabel?.replace(/（当前不可用）$/, "") ?? selected}（当前不可用）`, disabled: true }, ...choices.filter(choice => choice.value !== selected)], unavailable: true };
}

/** Change options in place. Neither the select nor an unchanged option is detached. */
export function updateQueryChoices(select: HTMLSelectElement, choices: QueryChoice[], selected = select.value): boolean {
  const retained = retainQueryChoice(choices, selected, Array.from(select.options).find(option => option.value === selected)?.textContent ?? undefined);
  const existing = new Map(Array.from(select.options, option => [option.value, option])), wanted = new Set(retained.choices.map(choice => choice.value));
  for (let index = 0; index < retained.choices.length; index++) {
    const choice = retained.choices[index], option = existing.get(choice.value) ?? document.createElement("option");
    if (option.getAttribute("value") !== choice.value) option.value = choice.value;
    if (option.textContent !== choice.label) option.textContent = choice.label;
    option.disabled = !!choice.disabled;
    if (select.options[index] !== option) select.insertBefore(option, select.options[index] ?? null);
  }
  for (const option of Array.from(select.options)) if (!wanted.has(option.value)) option.remove();
  const value = selected || (choices.some(choice => choice.value === "") ? "" : choices[0]?.value ?? "");
  if (select.value !== value) select.value = value;
  select.setAttribute("aria-invalid", String(retained.unavailable));
  return !retained.unavailable;
}
