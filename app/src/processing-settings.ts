/** Operation settings. Native Typst continues to own document language. */
export interface ProcessingLanguage { language: string; script: string | null; region: string | null }
export interface ProcessingDependency { id: string; version: string }
export interface SharedProcessingOrigin { source: "shared-library"; baseId: string; revision?: string }
export interface ProcessingSettings { version: string; languages: ProcessingLanguage[]; scheme: string; inputFormat: string; documentType: string; dependencies: ProcessingDependency[] }
export interface ProcessingRuleScope { universal?: boolean; languages?: string[]; scripts?: string[]; regions?: string[]; inputFormats?: string[]; documentTypes?: string[]; dependencies?: ProcessingDependency[] }
export const defaultProcessingSettings = (): ProcessingSettings => ({ version: "1", languages: [], scheme: "general", inputFormat: "typst", documentType: "prose", dependencies: [] });
export function parseProcessingLanguages(input: string): ProcessingLanguage[] {
  const result = input.split(/[,，\s]+/).filter(Boolean).map(tag => {
    if (!/^[A-Za-z]{2,8}(?:-[A-Za-z]{4})?(?:-(?:[A-Za-z]{2}|\d{3}))?$/.test(tag)) throw new Error("请用语言代码及可选文字体系、地区，例如 zh-Hans-CN、en-GB；未知地区可省略。");
    const [language, ...parts] = tag.split("-");
    const script = parts.find(p => p.length === 4) ?? null;
    const region = parts.find(p => p.length !== 4)?.toUpperCase() ?? null;
    return { language: language.toLowerCase(), script, region };
  });
  if (result.length > 32) throw new Error("每次最多选择 32 种处理语言。");
  return result;
}
export function processingLanguageTag(value: ProcessingLanguage): string { return [value.language, value.script, value.region].filter(Boolean).join("-"); }
export function validateProcessingScope(value: unknown): asserts value is ProcessingRuleScope {
  const s = value as ProcessingRuleScope;
  if (!s || typeof s !== "object" || Array.isArray(s) || s.universal !== undefined && typeof s.universal !== "boolean") throw new Error("规则需要明确的适用范围。");
  for (const key of ["languages", "scripts", "regions", "inputFormats", "documentTypes"] as const) {
    if (s[key] !== undefined && (!Array.isArray(s[key]) || s[key]!.length > 100 || s[key]!.some(v => typeof v !== "string" || !/^[A-Za-z0-9_.-]{1,100}$/.test(v)))) throw new Error("规则范围代码无效。");
  }
  if (s.universal && [s.languages, s.scripts, s.regions].some(v => v?.length)) throw new Error("通用规则不能同时限定语言、文字体系或地区。");
  if (s.dependencies !== undefined && (!Array.isArray(s.dependencies) || s.dependencies.length > 2000 || s.dependencies.some(v => !v || !/^[A-Za-z0-9_.-]{1,100}$/.test(v.id) || !/^[A-Za-z0-9_.-]{1,100}$/.test(v.version)))) throw new Error("规则依赖需要标识与准确版本。");
}
export function decodeProcessingSettings(input: unknown): ProcessingSettings {
  if (input === undefined) return defaultProcessingSettings();
  const v = input as ProcessingSettings;
  if (!v || typeof v !== "object" || !/^[A-Za-z0-9_.-]{1,100}$/.test(v.version) || !Array.isArray(v.languages) || v.languages.length > 32 || v.scheme !== "general" || !/^[A-Za-z0-9_.-]{1,50}$/.test(v.inputFormat) || !/^[A-Za-z0-9_.-]{1,100}$/.test(v.documentType) || !Array.isArray(v.dependencies)) throw new Error("处理设置需要版本、语言列表及当前支持的通用方案。");
  for (const l of v.languages) { if (!l || !/^[A-Za-z]{2,8}$/.test(l.language) || !(l.script === null || typeof l.script === "string" && /^[A-Za-z]{4}$/.test(l.script)) || !(l.region === null || typeof l.region === "string" && /^(?:[A-Za-z]{2}|\d{3})$/.test(l.region))) throw new Error("处理语言格式无效。"); parseProcessingLanguages(processingLanguageTag(l)); }
  validateProcessingScope({ universal: true, dependencies: v.dependencies });
  return structuredClone(v);
}
export function scopeLabel(scope?: ProcessingRuleScope): string {
  if (!scope) return "旧规则：待确认适用范围";
  const language = scope.universal ? "通用" : [...scope.languages ?? [], ...scope.scripts ?? [], ...scope.regions ?? []].join("／") || "范围未知";
  return language + (scope.dependencies?.length ? `；依赖 ${scope.dependencies.map(d => `${d.id}@${d.version}`).join("、")}` : "");
}
