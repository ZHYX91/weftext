import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const source = path.join(root, "assets/ui-messages.json");
const target = path.join(root, "app/src/ui-messages.ts");
const catalog = JSON.parse(fs.readFileSync(source, "utf8"));
const expectedKeys = (value, keys, description) => {
  if (!value || Array.isArray(value) || typeof value !== "object" || Object.keys(value).sort().join("\0") !== [...keys].sort().join("\0")) throw new Error(`Invalid ${description}`);
};
expectedKeys(catalog, ["version", "defaultLocale", "locales", "messages"], "UI catalog header");
if (catalog.version !== 1 || catalog.defaultLocale !== "zh-CN" || JSON.stringify(catalog.locales) !== '["zh-CN","en-US"]') throw new Error("Unsupported UI catalog version/locales");
const zh = {}, en = {}, parameterTypes = {}, params = [];
for (const [key, entry] of Object.entries(catalog.messages)) {
  if (!/^[A-Za-z][A-Za-z0-9]*(?:\.[A-Za-z][A-Za-z0-9]*)+$/.test(key)) throw new Error(`Invalid UI key ${key}`);
  expectedKeys(entry, ["zh-CN", "en-US", "params"], `UI message ${key}`);
  if (typeof entry["zh-CN"] !== "string" || !entry["zh-CN"] || typeof entry["en-US"] !== "string" || !entry["en-US"]) throw new Error(`Missing UI translation ${key}`);
  if (!entry.params || typeof entry.params !== "object" || Array.isArray(entry.params)) throw new Error(`Invalid UI parameters ${key}`);
  const names = Object.keys(entry.params).sort();
  for (const [name, type] of Object.entries(entry.params)) if (!/^[A-Za-z][A-Za-z0-9]*$/.test(name) || !["text", "integer"].includes(type)) throw new Error(`Invalid UI parameter ${key}.${name}`);
  for (const language of catalog.locales) {
    const actual = [...new Set([...entry[language].matchAll(/\{([A-Za-z][A-Za-z0-9]*)\}/g)].map(match => match[1]))].sort();
    if (JSON.stringify(actual) !== JSON.stringify(names)) throw new Error(`UI parameter mismatch ${key}/${language}`);
  }
  zh[key] = entry["zh-CN"]; en[key] = entry["en-US"];
  parameterTypes[key] = entry.params;
  params.push(`  ${JSON.stringify(key)}: { ${Object.entries(entry.params).map(([name, type]) => `${name}: ${type === "text" ? "string" : "number"}`).join("; ")} };`);
}
const generated = `// Generated from assets/ui-messages.json. Run node scripts/generate_ui_messages.mjs.\nexport const zhCN = ${JSON.stringify(zh, null, 2)} as const;\n\nexport type MessageKey = keyof typeof zhCN;\nexport interface MessageParamsMap {\n${params.join("\n")}\n}\nexport const messageParamTypes = ${JSON.stringify(parameterTypes, null, 2)} as const;\n\nexport const enUS: Readonly<Record<MessageKey, string>> = ${JSON.stringify(en, null, 2)};\n`;
if (process.argv.includes("--check")) {
  if (!fs.existsSync(target) || fs.readFileSync(target, "utf8") !== generated) throw new Error("Generated UI messages are stale; run node scripts/generate_ui_messages.mjs");
} else fs.writeFileSync(target, generated);
