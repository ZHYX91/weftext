// Real Outline in Chromium with an injected snapshot and a recording jump boundary.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { createServer } from '../app/node_modules/vite/dist/node/index.js';

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const vite = await createServer({
  root: fileURLToPath(new URL('../app/', import.meta.url)), configFile: false,
  server: { host: '127.0.0.1', port: 0, hmr: false },
  plugins: [{ name: 'outline-test', configureServer(server) {
    server.middlewares.use('/outline-test', (_request, response) => {
      response.setHeader('Content-Type', 'text/html;charset=utf-8');
      response.end('<!doctype html><html><main style="height:300px;width:270px"></main></html>');
    });
  } }],
});
let browser;
try {
  await vite.listen();
  browser = await chromium.launch({ headless: true, executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE });
  const page = await browser.newPage();
  page.setDefaultTimeout(6000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/outline-test`);
  await page.evaluate(async () => {
    const { HeadingStructureView } = await import('/src/heading-structure-view.ts');
    const view = new HeadingStructureView();
    window.jumps = [];
    const item = (key, text, kind = 'heading') => ({ key, text, kind, level: kind === 'heading' ? 1 : 0, generated: false });
    const items = [item('first', '章节第一'), item('figure', '章节插图', 'figure'), item('last', '章节最后')];
    window.draw = (visible = items) => view.render(document.querySelector('main'), { coverage: 'complete', truncated: false, items: visible }, h => window.jumps.push(h.key));
    window.draw();
  });
  const search = page.getByRole('searchbox');
  const next = page.locator('[data-outline-control="next-match"]');
  const previous = page.locator('[data-outline-control="previous-match"]');
  const selected = () => page.locator('.outline-row.is-selected').getAttribute('data-heading-key');
  const jumps = () => page.evaluate(() => window.jumps);
  const focused = () => search.evaluate(element => document.activeElement === element);

  await search.fill('章节');
  await search.press('ArrowDown'); assert.equal(await selected(), 'first');
  await search.press('ArrowDown'); assert.equal(await selected(), 'figure');
  await search.press('ArrowUp'); assert.equal(await selected(), 'first');
  await search.press('ArrowUp'); assert.equal(await selected(), 'last');
  assert.equal(await focused(), true);
  assert.deepEqual(await jumps(), []);
  await search.press('Enter'); assert.deepEqual(await jumps(), ['last']);

  await search.fill('章节第'); await search.press('Enter');
  assert.deepEqual(await jumps(), ['last', 'first']);
  await search.fill('章节');
  await next.click(); assert.equal(await selected(), 'first');
  await next.click(); assert.equal(await selected(), 'figure');
  await previous.click(); assert.equal(await selected(), 'first');
  await page.locator('[data-outline-control="filter"]').selectOption('caption');
  await search.press('Enter'); assert.deepEqual(await jumps(), ['last', 'first', 'figure']);

  await search.fill('没有结果');
  assert.equal(await previous.isDisabled(), true);
  assert.equal(await next.isDisabled(), true);
  await search.press('Enter'); assert.deepEqual(await jumps(), ['last', 'first', 'figure']);

  await search.fill('章节');
  await search.dispatchEvent('compositionstart');
  await search.press('Enter');
  await search.press('ArrowDown');
  assert.deepEqual(await jumps(), ['last', 'first', 'figure']);
  assert.equal(await page.locator('.outline-row.is-selected').count(), 0);
  await search.dispatchEvent('compositionend');
  await search.press('Enter'); assert.deepEqual(await jumps(), ['last', 'first', 'figure', 'figure']);
  // A background snapshot must not drop focus while a keyboard user opens Options.
  const options = page.locator('.outline-options');
  const summary = options.locator('summary');
  await summary.focus();
  await page.keyboard.press('Space');
  assert.equal(await options.evaluate(element => element.open), true);
  await page.evaluate(() => window.draw());
  assert.equal(await options.evaluate(element => element.open), true);
  assert.equal(await summary.evaluate(element => document.activeElement === element), true,
    'A fresh outline snapshot must retain focus on the disclosure');
  await page.keyboard.press('Tab');
  assert.equal(await options.locator('[data-outline-control="whole-document"]').evaluate(
    element => document.activeElement === element), true);
  await summary.focus();
  await page.keyboard.press('Space');
  await page.evaluate(() => window.draw());
  assert.equal(await options.evaluate(element => element.open), false);
  assert.equal(await summary.evaluate(element => document.activeElement === element), true);
  assert.deepEqual(errors, []);
  console.log('PASS outline search: initial result, wrap, Enter, filters, no matches, and composition ownership');
} finally {
  await browser?.close();
  await vite.close();
}
