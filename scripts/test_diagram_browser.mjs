// Bounded product-module acceptance, without occupying desktop input.
// Usage: node scripts/test_diagram_browser.mjs <weftext.exe> <new-owned-output-dir>
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';
import assert from 'node:assert/strict';
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
const exe = path.resolve(process.argv[2]), output = path.resolve(process.argv[3]);
await mkdir(output, { recursive: false });
const report = { executable: exe, sha256: createHash('sha256').update(await readFile(exe)).digest('hex'), browser: '', completed: false, cases: [], externalRequests: [], pageErrors: [] };
let server, browser;
try {
  server = await createServer({ root: path.join(repo, 'app'), configFile: false, server: { host: '127.0.0.1', port: 0 }, plugins: [{ name: 'diagram-acceptance', configureServer(server) {
  server.middlewares.use('/__diagram_acceptance', (_, response) => { response.setHeader('content-type', 'text/html; charset=utf-8'); response.end(`<!doctype html><meta charset="utf-8"><body><script type="module">
    import { DiagramPanel } from '/src/diagram-ui.ts';
    import { diagramOccurrences } from '/src/diagram-state.ts';
    import { renderDiagramPng } from '/src/diagram-render.ts';
    window.basis = { sessionId:'owned',baseId:'base',serial:1,path:'owned.typ',root:'owned',source:'原稿\\n' };
    window.patches = [];
    window.writable=true;
    window.panel = new DiagramPanel({ getDocument:()=>window.basis, notify:()=>{}, request:async()=>{throw Error('unexpected network')}, applyPatch:async(patch,basis)=>{if(!window.writable || basis.source !== window.basis.source) throw Error('stale');window.patches.push(patch);window.basis={...window.basis,source:basis.source.slice(0,patch.from)+patch.text+basis.source.slice(patch.to),serial:basis.serial+1};} },()=>window.writable);
    window.occurrences=diagramOccurrences; window.render=renderDiagramPng; window.ready=true;
    </script>`); }); } }] });
  await server.listen(); const port = server.httpServer.address().port, base = `http://127.0.0.1:${port}`;
  browser = await chromium.launch({ headless: true, ...(process.env.WEFTEXT_BROWSER_CHANNEL ? { channel: process.env.WEFTEXT_BROWSER_CHANNEL } : {}) }); report.browser = browser.version();
  const context = await browser.newContext();
  await context.route('**/*', route => { const url = new URL(route.request().url()); if (url.origin === base || ['blob:', 'data:'].includes(url.protocol)) return route.continue(); report.externalRequests.push(url.toString()); return route.abort(); });
  const page = await context.newPage(); page.setDefaultTimeout(20000); page.on('pageerror', error => { report.pageErrors.push(String(error)); console.error('pageerror:', String(error)); }); await page.goto(base + '/__diagram_acceptance'); await page.waitForFunction(() => window.ready);
  await page.evaluate(() => window.panel.open());
  await page.getByRole('button', { name: '生成 PNG 预览', exact: true }).click(); await page.getByRole('button', { name: '确认当前图片并写入草稿', exact: true }).waitFor();
  await page.waitForFunction(() => ![...document.querySelectorAll('button')].find(button => button.textContent === '确认当前图片并写入草稿').disabled || document.querySelector('.diagram-panel[open] .data-warning'));
  assert.equal(await page.locator('.diagram-panel[open] .data-warning').count(), 0, await page.locator('.diagram-panel[open]').innerText());
  await page.screenshot({ path: path.join(output, 'chinese-preview.png') });
  await page.getByLabel('Mermaid 源码', { exact: true }).fill('flowchart LR\n  A[资料] --> B[校对] --> C[完成]');
  assert.equal(await page.locator('.diagram-preview img').count(), 0); assert.equal(await page.getByRole('button', { name: '确认当前图片并写入草稿', exact: true }).isDisabled(), true);
  report.cases.push('source edit invalidates preview and confirmation');
  await page.getByRole('button', { name: '生成 PNG 预览', exact: true }).click(); await page.waitForFunction(() => ![...document.querySelectorAll('button')].find(button => button.textContent === '确认当前图片并写入草稿').disabled);
  await page.getByRole('button', { name: '确认当前图片并写入草稿', exact: true }).click();
  const saved = await page.evaluate(() => ({ basis: window.basis, count: window.patches.length, diagrams: window.occurrences(window.basis.source).map(item=>({ ...item, png:Array.from(item.png) })) }));
  assert.equal(saved.count, 1); assert.equal(saved.diagrams.length, 1); assert.equal(saved.diagrams[0].source, 'flowchart LR\n  A[资料] --> B[校对] --> C[完成]');
  await writeFile(path.join(output, 'diagram.typ'), saved.basis.source); await writeFile(path.join(output, 'diagram.png'), Uint8Array.from(saved.diagrams[0].png));
  report.cases.push('Chinese current PNG and exact native source committed as one draft patch');
  function compile(source, paged = false) {
    const result = spawnSync(exe, ['--render-worker'], { input: JSON.stringify({ source, path: path.join(output,'diagram.typ'), root: output, frozen_files: {}, paged }), encoding: 'utf8', maxBuffer: 26*1024*1024, timeout: 25000, windowsHide: true });
    assert.equal(result.status, 0, result.stderr); return JSON.parse(result.stdout);
  }
  const html = compile(saved.basis.source); assert.equal(html.ok, true, JSON.stringify(html.diagnostics));
  assert.ok(html.html.includes('data:image/png;base64,')); const paged = compile(saved.basis.source, true); assert.equal(paged.ok, true, JSON.stringify(paged.diagnostics));
  const stale = saved.basis.source.replace('let source = "flowchart', 'let source = "graph'); const rejected = compile(stale); assert.equal(rejected.ok, false); assert.ok(rejected.diagnostics.some(item=>item.message.includes('Mermaid 图源已改变')));
  report.cases.push('real frozen Html/Paged compile accepts PNG; changing source refuses stale image');
  await page.evaluate(() => window.panel.open()); await page.locator('.diagram-panel[open]').getByLabel('Mermaid 源码', { exact: true }).fill('flowchart LR\n  BROKEN['); await page.getByRole('button', { name: '生成 PNG 预览', exact: true }).click(); await page.waitForFunction(() => document.querySelector('.diagram-panel[open] .data-warning'));
  assert.equal(await page.getByRole('button', { name: '确认当前图片并写入草稿', exact: true }).isDisabled(), true); assert.equal(await page.evaluate(() => window.patches.length), 1);
  await page.getByRole('button', { name: '关闭并保留输入', exact: true }).click(); await page.evaluate(() => window.panel.open()); assert.equal(await page.locator('.diagram-panel[open]').getByLabel('Mermaid 源码', { exact: true }).inputValue(), 'flowchart LR\n  BROKEN[');
  report.cases.push('syntax failure and dialog cancellation preserve input without writing draft');
  await page.locator('.diagram-panel[open]').getByLabel('Mermaid 源码', { exact: true }).fill('flowchart LR\nA-->B'); await page.getByRole('button', { name: '生成 PNG 预览', exact: true }).click(); await page.waitForFunction(() => ![...document.querySelectorAll('.diagram-panel[open] button')].find(button => button.textContent === '确认当前图片并写入草稿').disabled);
  await page.evaluate(() => window.basis = { ...window.basis, serial: window.basis.serial+1, source: window.basis.source+'外部草稿' }); await page.getByRole('button', { name: '确认当前图片并写入草稿', exact: true }).click();
  await page.getByText(/笔记已改变或权限已更新/).waitFor(); assert.equal(await page.evaluate(() => window.patches.length), 1);
  report.cases.push('changed draft basis refuses late confirmation and preserves input');
  await page.evaluate(() => { window.writable=false; window.panel.clear(); });
  assert.equal(await page.locator('.diagram-panel').count(), 0); await page.evaluate(() => window.panel.open()); assert.equal(await page.locator('.diagram-panel').count(), 0);
  report.cases.push('revocation purge removes source, preview and retained dialogs; read-only gate refuses reopen');
  assert.equal(report.externalRequests.length, 0); assert.equal(report.pageErrors.length, 0); report.completed = true;
} finally { if (browser) await browser.close(); if (server) await server.close(); await writeFile(path.join(output, 'result.json'), JSON.stringify(report, null, 2)); }
console.log(JSON.stringify(report, null, 2));
