// Real local history dialog; data and delay come only from an isolated read-only fixture.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { createServer } from '../app/node_modules/vite/dist/node/index.js';

const require = createRequire(import.meta.url);
const baseline = process.argv.includes('--baseline');
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const server = await createServer({
  root: fileURLToPath(new URL('../app/', import.meta.url)),
  configFile: false,
  server: { host: '127.0.0.1', port: 0, hmr: false },
  plugins: [{ name: 'local-history-focus', configureServer(vite) {
    vite.middlewares.use('/history-fixture', (_request, response) => {
      response.setHeader('Content-Type', 'text/html;charset=utf-8');
      response.end('<!doctype html><html lang="zh-CN"><meta charset="UTF-8"><main></main></html>');
    });
  } }],
});

let browser;
try {
  await server.listen();
  browser = await chromium.launch({ headless: true, executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE });
  const page = await browser.newPage({ viewport: { width: 720, height: 650 } });
  page.setDefaultTimeout(6000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/history-fixture`);
  await page.evaluate(async () => {
    const { HistoryPanel } = await import('/src/history-panel.ts');
    const note = Object.freeze({
      inputId: 'owned-input', sessionId: 'session', root: '/owned', path: '/owned/note.typ',
      workspaceId: 'workspace', nodeId: 'node', source: 'Current draft', baseSource: 'Current draft',
      baseId: 'base', serial: 0, dirty: false,
    });
    const description = { explanation: 'Fixture', diagnostics: [], causes: [] };
    const version = (id, sequence, source) => ({
      versionId: id, operationId: id, revisionId: id, kind: 'save', commitSequence: sequence,
      savedAtUnixMs: sequence * 1000, timeBasis: 'commitClock', status: 'available',
      byteLength: source.length, sha256: 'fixture', source, description, readOnly: true,
      previewStatus: 'sourceOnly', previewDescription: description,
    });
    const versions = [version('second', 2, 'Saved second'), version('first', 1, 'Saved first')];
    const list = {
      ...note, currentBaseId: note.baseId, currentSavedVersionId: 'second', currentness: 'saved',
      versions, retention: { policy: 'retainedCoreJournal', complete: false, knownVersions: 2,
        availableVersions: 2, oldestSavedAtUnixMs: 1000, newestSavedAtUnixMs: 2000, description },
      warnings: [], protectedDrafts: [],
    };
    window.calls = [];
    window.notices = [];
    const panel = new HistoryPanel({
      getDocument: () => note,
      request: (path, body) => {
        window.calls.push(path);
        if (path === '/api/history/list' && window.calls.length === 1) return Promise.resolve(list);
        if (path === '/api/history/list' || path === '/api/history/compare') {
          const comparison = { selected: versions.find(item => item.versionId === body.versionId),
            current: { source: note.baseSource, baseId: note.baseId, path: note.path, serial: note.serial },
            comparison: 'currentSaved', equal: false };
          return new Promise((resolve, reject) => {
            window.finishAction = (error = false) => {
              delete window.finishAction;
              if (error) reject(Error('Synthetic read failure'));
              else resolve(path === '/api/history/list' ? list : comparison);
            };
          });
        }
        if (path === '/api/history/read') {
          const row = versions.find(item => item.versionId === body.versionId);
          return new Promise(resolve => { window.completeRead = () => {
            delete window.completeRead;
            resolve(row);
          }; });
        }
        throw Error(`Unexpected write or request: ${path}`);
      },
      restored: () => { throw Error('Read-only fixture must not restore'); },
      notify: (message, error) => { if (error) window.notices.push(message); },
    });
    await panel.open();
  });
  const first = page.locator('[data-history-version="first"]');
  const second = page.locator('[data-history-version="second"]');
  const close = page.locator('.history-header button');
  const focused = locator => locator.evaluate(node => node.ownerDocument.activeElement === node);
  async function complete() {
    await page.waitForFunction(() => typeof window.completeRead === 'function');
    await page.evaluate(() => window.completeRead());
  }

  await second.focus();
  await page.keyboard.press('Enter');
  await complete();
  await page.getByText('Saved second', { exact: true }).waitFor();
  assert.equal(await focused(second), true, 'Opening a saved version must retain its keyboard position after refresh');
  assert.equal(await second.getAttribute('aria-pressed'), 'true');
  await page.keyboard.press('Tab');
  assert.equal(await focused(first), true, 'Tab continues to the next version');

  await page.keyboard.press('Enter');
  await page.waitForFunction(() => typeof window.completeRead === 'function');
  await close.focus();
  await complete();
  await page.getByText('Saved first', { exact: true }).waitFor();
  assert.equal(await focused(close), true, 'Refreshing the rows must not reclaim focus from another control');
  assert.deepEqual(await page.evaluate(() => window.calls), [
    '/api/history/list', '/api/history/read', '/api/history/read',
  ]);
  const refresh = page.locator('.history-sidebar > button');
  const compare = page.locator('.history-actions > button').first();
  const awaitAction = () => page.waitForFunction(() => typeof window.finishAction === 'function');
  const finish = async (error = false) => {
    await page.evaluate(error => window.finishAction(error), error);
    await page.waitForFunction(() => !document.querySelector('.history-sidebar > button').disabled
      && document.querySelector('.history-sidebar > button').getAttribute('aria-disabled') !== 'true');
  };
  for (const [control, endpoint] of [[refresh, '/api/history/list'], [compare, '/api/history/compare']]) {
    const priorCount = await page.evaluate(endpoint => window.calls.filter(path => path === endpoint).length, endpoint);
    await control.focus();
    await page.keyboard.press('Enter');
    await awaitAction();
    if (baseline) {
      await control.evaluate(el => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
      assert.equal(await focused(control), false, endpoint + ' baseline loses focus');
      await finish();
      console.log(endpoint + ' reproduced focus loss on #38');
      continue;
    }
    assert.equal(await focused(control), true, endpoint + ' holds focus while pending');
    assert.equal(await control.getAttribute('aria-disabled'), 'true');
    await page.keyboard.press('Enter');
    assert.equal(await page.evaluate(endpoint => window.calls.filter(path => path === endpoint).length, endpoint), priorCount + 1);
    await finish(endpoint === '/api/history/compare');
    assert.equal(await focused(control), true, endpoint + ' holds focus after settling');
  }
  if (!baseline) {
    assert.equal(await page.evaluate(() => window.notices.length), 1);
    await page.keyboard.press('Enter');
    await awaitAction();
    await close.focus();
    await finish();
    assert.equal(await focused(close), true, 'successful comparison must not retake focus');
    await refresh.focus();
    await page.keyboard.press('Enter');
    await awaitAction();
    await finish();
    await page.keyboard.press('Tab');
    assert.equal(await focused(second), true, 'Tab continues from Refresh into the history list');
    assert.deepEqual(await page.evaluate(() => window.calls), [
      '/api/history/list', '/api/history/read', '/api/history/read', '/api/history/list',
      '/api/history/compare', '/api/history/compare', '/api/history/list',
    ]);
  }
  assert.deepEqual(errors, []);
  console.log('Local history is read-only; row navigation and read-action focus checked.');
} finally {
  await browser?.close();
  await server.close();
}
