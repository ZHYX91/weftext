// Mounted local style dialog in Chromium. Synthetic Core supplies only the
// inspected style; activating Restore must not call prepare or apply.
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { createServer } from "../app/node_modules/vite/dist/node/index.js";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || "playwright");
const root = fileURLToPath(new URL("../app/", import.meta.url));
const vite = await createServer({ root, configFile: false, server: { host: "127.0.0.1", port: 0, hmr: false } });
const browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE, headless: true });
const original = "#let library-style(body) = {\n  set text(size: 11pt)\n  body\n}\n";
const baseline = process.argv.includes("--baseline");
try {
  await vite.listen();
  const origin = "http://127.0.0.1:" + vite.httpServer.address().port;
  for (const [width, advanced, editable] of [[640, false, true], [390, true, true], [390, false, false]]) {
    const page = await browser.newPage({ viewport: { width, height: 740 }, locale: "zh-CN" });
    const errors = [];
    page.on("pageerror", error => { errors.push(error.message); console.error(error.message); });
    page.setDefaultTimeout(5000);
    await page.route(origin + "/", route => route.fulfill({
      contentType: "text/html",
      body: '<!doctype html><meta charset="utf-8"><html lang="zh-CN"><button id="open">Open style</button></html>',
    }));
    await page.goto(origin);
    await page.evaluate(async ({ original, editable }) => {
      const { openLibraryStylePanel } = await import("/src/style-panel.ts");
      await import("/src/style.css");
      await import("/src/style-panel.css");
      window.writeAttempts = 0;
      const write = () => { window.writeAttempts++; throw new Error("Unexpected write"); };
      document.querySelector("#open").onclick = () => void openLibraryStylePanel({
        read: async () => ({ path: "Styles/library-style.typ", source: original, baseId: "v1",
          enabled: true, entryFunction: "library-style", resourceRef: null, installed: true,
          sourceHash: "a".repeat(64), syntaxValid: true, warnings: [],
          controls: [{ key: "text.size", label: { key: "engine.style.label.bodySize", params: {} }, kind: "text", value: "11pt", editable, reason: null }] }),
        operationId: () => "offline-test", prepare: write, apply: write,
      });
    }, { original, editable });
    await page.locator("#open").click();
    const dialog = page.locator(".library-style-panel");
    await dialog.waitFor();
    const enabled = dialog.locator(".style-enabled input");
    const field = dialog.locator(".style-controls input");
    const source = dialog.locator(".style-native-source textarea");
    if (advanced) {
      await dialog.locator(".style-native-source summary").click();
      await source.fill(original + "// draft\n");
    } else if (editable) {
      await field.fill("13pt");
      await field.press("Tab");
    } else {
      await enabled.press("Space");
    }
    const restore = dialog.locator(".modal-actions button").first();
    assert.equal(await restore.isEnabled(), true);
    await restore.focus();
    await restore.press("Enter");
    assert.equal(await restore.isEnabled(), false);
    assert.equal(await field.inputValue(), "11pt");
    assert.equal(await source.inputValue(), original);
    assert.equal(await enabled.isChecked(), true);
    const expected = advanced ? source : editable ? field : enabled;
    assert.equal(await expected.evaluate(el => el === document.activeElement), !baseline,
      "Restore should return keyboard focus to the relevant editing control");
    if (!baseline) {
      if (editable) {
        await page.keyboard.press("End");
        await page.keyboard.insertText("a");
        assert.match(await expected.inputValue(), /a/);
      } else {
        await page.keyboard.press("Space");
        assert.equal(await enabled.isChecked(), false);
      }
    }
    assert.equal(await page.evaluate(() => window.writeAttempts), 0);
    assert.deepEqual(errors, []);
    await page.close();
    console.log(`${width}px ${advanced ? "native source" : editable ? "simplified" : "no editable parameter"}: ${baseline ? "baseline loses focus" : "reset returns focus without saving"}`);
  }
} finally {
  await browser.close();
  await vite.close();
}
