// Real SourceEditor in Chromium on a synthetic document. Composition events
// below test browser event ownership, not a native IME or candidate window.
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import { createServer } from '../app/node_modules/vite/dist/node/index.js'

const require = createRequire(import.meta.url)
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright')
const root = fileURLToPath(new URL('../app/', import.meta.url))
const vite = await createServer({ root, configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false } })
const baseline = process.argv.includes('--baseline')
let browser
try {
  await vite.listen()
  browser = await chromium.launch({ headless: true, executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE })
  const url = 'http://127.0.0.1:' + vite.httpServer.address().port
  const page = await browser.newPage({ viewport: { width: 390, height: 740 }, locale: 'zh-CN' })
  page.setDefaultTimeout(8000)
  const errors = []
  page.on('pageerror', error => errors.push(error.message))
  await page.route(url + '/goto-line-fixture', route => route.fulfill({
    contentType: 'text/html',
    body: '<!doctype html><html lang="zh-CN"><meta charset=utf-8><main id="proof"></main>',
  }))
  await page.goto(url + '/goto-line-fixture')
  await page.evaluate(async () => {
    const { SourceEditor, localeApi } = await import('/src/ui-locale.browser.ts')
    localeApi.locale.set('zh-CN')
    const initial = '甲甲甲\n乙乙乙\n丙丙丙\n丁丁丁'
    const editor = new SourceEditor(initial, () => {}, () => {})
    document.getElementById('proof').append(editor.view.dom)
    window.gotoFixture = { editor, initial, localeApi }
  })
  await page.locator('.cm-content').focus()
  const panel = page.locator('.cm-goto-line')
  const input = panel.locator('input[name=line]')
  const snapshot = () => page.evaluate(() => {
    const { editor, initial } = window.gotoFixture
    const { doc, selection } = editor.view.state, cursor = selection.main.head, line = doc.lineAt(cursor)
    const field = document.querySelector('.cm-goto-line input[name=line]')
    return {
      line: line.number, column: cursor - line.from, unchanged: editor.source === initial,
      open: Boolean(field), retained: document.activeElement === field, value: field?.value,
      message: document.getElementById(field?.getAttribute('aria-describedby'))?.textContent,
      invalid: field?.getAttribute('aria-invalid')
    }
  })
  const before = await snapshot()
  await page.keyboard.press('Control+Alt+g')
  await input.fill('第三行')
  await input.press('Enter')
  if (baseline) {
    await panel.waitFor({ state: 'detached' })
    const lost = await snapshot()
    assert.equal(lost.line, before.line)
    assert(lost.unchanged && !lost.message)
    console.log('Baseline closed invalid Go to line without a jump or explanation')
  } else {
    let actual = await snapshot()
    assert(actual.open && actual.retained && actual.unchanged && actual.line === before.line)
    assert.equal(actual.value, '第三行')
    assert.equal(actual.invalid, 'true')
    assert.match(actual.message, /请输入有效的行号/)
    await page.evaluate(() => window.gotoFixture.localeApi.locale.set('en-US'))
    actual = await snapshot()
    assert(actual.retained && actual.message?.startsWith('Enter a valid line'))
    await input.fill('??')
    await panel.locator('button[type=submit]').click()
    actual = await snapshot()
    assert(actual.open && actual.retained && actual.invalid === 'true')
    await input.fill('3:2')
    assert.equal((await snapshot()).invalid, null)
    await input.evaluate(el => {
      el.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }))
      for (const [key, keyCode] of [['Enter', 13], ['Escape', 27]])
        el.dispatchEvent(new KeyboardEvent('keydown', { key, keyCode, isComposing: true, bubbles: true, cancelable: true }))
      el.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }))
    })
    assert((await snapshot()).open && (await snapshot()).retained)
    await input.press('Enter')
    await panel.waitFor({ state: 'detached' })
    actual = await snapshot()
    assert.deepEqual([actual.line, actual.column, actual.unchanged], [3, 2, true])
    await page.keyboard.press('Control+Alt+g')
    await input.fill('-1')
    await input.press('Enter')
    await panel.waitFor({ state: 'detached' })
    assert.equal((await snapshot()).line, 2)
    await page.keyboard.press('Control+Alt+g')
    await input.fill('no')
    await input.press('Escape')
    await panel.waitFor({ state: 'detached' })
    assert.equal((await snapshot()).line, 2)
    assert.deepEqual(errors, [])
    console.log('Malformed input retained with localized guidance; click, IME ownership, absolute/relative jumps and Escape passed')
  }
} finally {
  await browser?.close()
  await vite.close()
}
