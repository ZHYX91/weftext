import assert from "node:assert/strict";
import test from "node:test";
import { CHINESE_FONT_SAMPLE, OfflineFontLoader, type FontDeliveryManifest, type FontReadinessHost } from "./font-readiness";

const families = ["Noto Sans CJK SC", "New Computer Modern", "Noto Serif CJK SC", "Noto Sans Mono CJK SC", "DejaVu Sans Mono", "New Computer Modern Math"];
function manifest(): FontDeliveryManifest { return { fonts: families.map((family, index) => ({ family, file: `Font${index}.otf`, url: `/fonts/Font${index}.otf`, weight: 400, role: family.endsWith("Math") ? "math" : "body", inspection: { hasMathTable: family.endsWith("Math") } })) }; }
function host(overrides: Partial<FontReadinessHost> = {}): FontReadinessHost {
  return { readManifest: async () => manifest(), fonts: { load: async font => [{ family: font.match(/"([^"]+)"$/)![1], status: "loaded" }], check: () => true }, timeoutMs: 100, ...overrides };
}

test("default readiness loads only six active regular families and actual Chinese samples", async () => {
  const calls: { font: string; text?: string }[] = [];
  const loader = new OfflineFontLoader(host({ fonts: { load: async (font, text) => { calls.push({ font, text }); return [{ family: font.match(/"([^"]+)"$/)![1], status: "loaded" }]; }, check: () => true } }));
  await loader.ensureDefaults(); assert.equal(calls.length, 6);
  assert.equal(calls.filter(call => call.text?.includes(CHINESE_FONT_SAMPLE)).length, 3);
  assert.ok(calls.every(call => call.font.startsWith("normal 400")));
});

test("a true FontFaceSet.check without any actual face never claims readiness", async () => {
  const loader = new OfflineFontLoader(host({ fonts: { load: async () => [], check: () => true } }));
  await assert.rejects(loader.ensure([{ family: "Noto Serif CJK SC" }]), /尚未成功加载/);
});

test("unavailable FontFaceSet and rejected font bytes fail explicitly", async () => {
  await assert.rejects(new OfflineFontLoader(host({ fonts: undefined })).ensureDefaults(), /没有字体加载接口/);
  await assert.rejects(new OfflineFontLoader(host({ fonts: { load: async () => { throw new Error("invalid font bytes"); }, check: () => false } })).ensure([{ family: "Noto Serif CJK SC" }]), /invalid font bytes/);
});

test("missing custom families and unconfirmed MATH data have explicit repair errors", async () => {
  await assert.rejects(new OfflineFontLoader(host()).ensure([{ family: "Custom Font" }]), /没有随本机环境交付/);
  const broken = manifest(); broken.fonts.at(-1)!.inspection = { hasMathTable: false };
  await assert.rejects(new OfflineFontLoader(host({ readManifest: async () => broken })).ensureDefaults(), /OpenType MATH/);
});

test("font timeouts are bounded, and a later retry uses a real face", async () => {
  let first = true;
  const loader = new OfflineFontLoader(host({ timeoutMs: 5, fonts: { load: async font => { if (first) { first = false; return new Promise(() => {}); } return [{ family: font.match(/"([^"]+)"$/)![1], status: "loaded" }]; }, check: () => true } }));
  await assert.rejects(loader.ensure([{ family: "Noto Serif CJK SC" }]), /加载超时/);
  await loader.ensure([{ family: "Noto Serif CJK SC" }]);
});

test("foreign delivery URLs and wrong loaded family cannot masquerade as bundled fonts", async () => {
  const foreign = manifest(); foreign.fonts[0].url = "https://example.com/font.otf";
  await assert.rejects(new OfflineFontLoader(host({ readManifest: async () => foreign })).ensure([{ family: families[0] }]), /交付地址/);
  await assert.rejects(new OfflineFontLoader(host({ fonts: { load: async () => [{ family: "Other Family", status: "loaded" }], check: () => true } })).ensure([{ family: families[0] }]), /尚未成功加载/);
});

test("a malformed delivered manifest fails with an explicit readiness error", async () => {
  const invalid = { fonts: [null] } as unknown as FontDeliveryManifest;
  await assert.rejects(new OfflineFontLoader(host({ readManifest: async () => invalid })).ensureDefaults(), /字体清单无效/);
});

test("an explicit local browser derivative retains its actual native family and rejects foreign or mismatched delivery", async () => {
  const delivered = manifest(), entry = delivered.fonts[1];
  entry.browser = { file: "WeftextWebSerif-Regular.woff2", url: "/fonts/WeftextWebSerif-Regular.woff2", family: entry.family, weight: entry.weight, style: "normal" };
  await new OfflineFontLoader(host({ readManifest: async () => delivered })).ensure([{ family: entry.family }]);
  entry.browser.url = "https://example.com/font.woff2";
  await assert.rejects(new OfflineFontLoader(host({ readManifest: async () => delivered })).ensure([{ family: entry.family }]), /网页派生交付/);
  entry.browser.url = `/fonts/${entry.browser.file}`; entry.browser.weight = 700;
  await assert.rejects(new OfflineFontLoader(host({ readManifest: async () => delivered })).ensure([{ family: entry.family }]), /网页派生交付/);
});
