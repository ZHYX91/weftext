const properties = new Set([
  "font-family", "font-size", "font-weight", "font-style", "color", "background-color",
  "border-top", "border-right", "border-bottom", "border-left", "border-collapse",
  "text-align", "vertical-align", "white-space", "display", "line-height",
  "margin-inline-end", "list-style-type",
]);

/** The native bridge supplies presentation values, never author HTML/CSS rules. */
export function nativeStyleProperties(input: unknown): [string, string][] {
  if (typeof input !== "string" || input.length > 32768) return [];
  return input.split(";").flatMap(declaration => {
    const split = declaration.indexOf(":"); if (split < 0) return [];
    const key = declaration.slice(0, split).trim().toLowerCase(), value = declaration.slice(split + 1).trim();
    if (!properties.has(key) || !value || value.length >= 4096 || /[\x00-\x1f\x7f{}<>\\]/.test(value) || /url\s*\(|expression\s*\(|!important|@import/i.test(value)) return [];
    return [[key, value] as [string, string]];
  });
}
