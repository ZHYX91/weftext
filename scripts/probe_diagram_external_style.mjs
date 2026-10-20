// Read-only product-module probe; no desktop input and no dependency requests
// are allowed to leave the headless browser. A successful probe records both
// acceptance and rejection; inspect result.json for actual renderer behavior.
// Usage: node scripts/probe_diagram_external_style.mjs <new-owned-output-dir> [source-file]
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const output = path.resolve(process.argv[2]);
await mkdir(output, { recursive: false });
const report = { completed: false, source: 'flowchart LR\n A-->B\n style A fill:url(/external.svg)', browser: '', dependencyAttempts: [], pageErrors: [], scope: 'Current repository diagram modules served by Vite; not packaged-executable UI or native Office acceptance', rendererSha256: createHash('sha256').update(await readFile(path.join(repo, 'app/src/diagram-render.ts'))).digest('hex') };
if (process.argv[3]) report.source = await readFile(path.resolve(process.argv[3]), 'utf8');
let server, browser;
try {
  const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
  server = await createServer({ root: path.join(repo, 'app'), cacheDir: path.join(output, 'vite-cache'), configFile: false, server: { host: '127.0.0.1', port: 0 }, plugins: [{ name: 'diagram-style-probe', configureServer(server) {
    server.middlewares.use('/__diagram_style_probe', (_, response) => { response.setHeader('content-type', 'text/html; charset=utf-8'); response.end(`<!doctype html><meta charset="utf-8"><body><script type="module">
import { validateDiagramSource } from '/src/diagram-state.ts';
import { renderDiagramPng } from '/src/diagram-render.ts';
const parse = DOMParser.prototype.parseFromString;
DOMParser.prototype.parseFromString = function(...args) { const doc = parse.apply(this, args); if (args[1] === 'image/svg+xml' && doc.documentElement.localName === 'svg') window.capturedSvg = args[0]; return doc; };
window.validate = validateDiagramSource; window.render = renderDiagramPng; window.ready = true;
</script>`); });
  } }] });
  await server.listen(); const base = `http://127.0.0.1:${server.httpServer.address().port}`;
  browser = await chromium.launch({ headless: true, ...(process.env.WEFTEXT_BROWSER_CHANNEL ? { channel: process.env.WEFTEXT_BROWSER_CHANNEL } : {}) }); report.browser = browser.version();
  const context = await browser.newContext();
  await context.route('**/*', route => {
    const url = new URL(route.request().url());
    if (url.pathname === '/external.svg' || ![base].includes(url.origin) && !['blob:', 'data:'].includes(url.protocol)) { report.dependencyAttempts.push({ url: url.href, resourceType: route.request().resourceType(), action: 'aborted before delivery' }); return route.abort(); }
    return route.continue();
  });
  const page = await context.newPage(); page.setDefaultTimeout(25000); page.on('pageerror', error => report.pageErrors.push(String(error)));
  await page.goto(base + '/__diagram_style_probe'); await page.waitForFunction(() => window.ready);
  const observed = await page.evaluate(async source => {
    let sourceAccepted = false, sourceError = '', renderAccepted = false, renderError = '', png = [], width, height;
    try { window.validate(source); sourceAccepted = true; } catch (error) { sourceError = String(error); }
    try { const result = await window.render(source, 1200); renderAccepted = true; png = Array.from(result.png); width = result.width; height = result.height; } catch (error) { renderError = String(error); }
    const svg = window.capturedSvg || '';
    const parsed = new DOMParser().parseFromString(svg || '<svg/>', 'image/svg+xml');
    const styles = [...parsed.querySelectorAll('*')].flatMap(element => [...element.attributes].filter(attribute => /url\s*\(/i.test(attribute.value)).map(attribute => ({ element: element.localName, attribute: attribute.name, value: attribute.value })));
    return { sourceAccepted, sourceError, renderAccepted, renderError, png, width, height, svg, styles };
  }, report.source);
  const { png, svg, ...summary } = observed; Object.assign(report, summary);
  if (svg) await writeFile(path.join(output, 'observed.svg'), svg);
  if (png.length) { const bytes = Uint8Array.from(png); await writeFile(path.join(output, 'observed.png'), bytes); report.pngBytes = bytes.length; report.pngSha256 = createHash('sha256').update(bytes).digest('hex'); }
  await page.screenshot({ path: path.join(output, 'probe.png'), fullPage: true }); report.completed = true;
} finally {
  if (browser) await browser.close(); if (server) await server.close();
  await writeFile(path.join(output, 'result.json'), JSON.stringify(report, null, 2));
}
console.log(JSON.stringify(report, null, 2));
