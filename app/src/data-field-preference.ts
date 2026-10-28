export type FieldPlacement = "top" | "default" | "bottom" | "hidden";
export interface FieldPresentation {
    version: 1;
    sort: "schema" | "name";
    placements: Record<string, Exclude<FieldPlacement, "default">>;
}
export const defaultFieldPresentation = (): FieldPresentation => ({ version: 1, sort: "schema", placements: {} });

/** Local presentation never supplies field identities or changes author order. */
export function decodeFieldPresentation(raw: string | null, fieldIds: ReadonlySet<string>): FieldPresentation {
    if (!raw) return defaultFieldPresentation();
    try {
        const value = JSON.parse(raw);
        if (!value || typeof value !== "object" || Array.isArray(value) || value.version !== 1 ||
            !["schema", "name"].includes(value.sort) || !value.placements || typeof value.placements !== "object" || Array.isArray(value.placements) ||
            Object.keys(value).some(key => !["version", "sort", "placements"].includes(key))) return defaultFieldPresentation();
        const entries = Object.entries(value.placements);
        if (entries.length > 10000) return defaultFieldPresentation();
        const known = entries.filter((entry): entry is [string, Exclude<FieldPlacement, "default">] => fieldIds.has(entry[0]) && (entry[1] === "top" || entry[1] === "bottom" || entry[1] === "hidden"));
        return { version: 1, sort: value.sort, placements: Object.fromEntries(known) };
    } catch { return defaultFieldPresentation(); }
}

export function setFieldPlacement(preference: FieldPresentation, id: string, placement: FieldPlacement): FieldPresentation {
    const entries = Object.entries(preference.placements).filter(([key]) => key !== id);
    if (placement !== "default") entries.push([id, placement]);
    return { ...preference, placements: Object.fromEntries(entries) };
}

export function effectiveFieldPlacement(preference: FieldPresentation, id: string, protectedField: boolean, showHidden: boolean): FieldPlacement {
    const placement = Object.hasOwn(preference.placements, id) ? preference.placements[id] : "default";
    return placement === "hidden" && protectedField ? "default" : placement === "hidden" && showHidden ? "bottom" : placement;
}

export function comparePresentedFields(preference: FieldPresentation, left: { name: string; order: number }, right: { name: string; order: number }): number {
    return preference.sort === "name" ? left.name.localeCompare(right.name) || left.order - right.order : left.order - right.order;
}
