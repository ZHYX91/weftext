// Real Chromium and production picker; synthetic DOM composition events, not native IME.
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import { createServer } from '../app/node_modules/vite/dist/node/index.js'

const require = createRequire(import.meta.url)
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright')
const root = fileURLToPath(new URL('../app/', import.meta.url))
const vite = await createServer({ root, configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false } })
let browser
try {
  await vite.listen()
  browser = await chromium.launch({ headless: true, executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE })
  const url = 'http://127.0.0.1:' + vite.httpServer.address().port
  const page = await browser.newPage({ viewport: { width: 390, height: 780 }, locale: 'zh-CN' })
  const errors = []
  page.on('pageerror', error => errors.push(error.message))
  await page.route(url + '/picker-fixture', route => route.fulfill({
    contentType: 'text/html', body: '<!doctype html><html lang="zh-CN"><meta charset=utf-8><button id="open">打开</button>',
  }))
  await page.goto(url + '/picker-fixture')
  await page.evaluate(async () => {
    const { openEverydayPicker } = await import('/src/everyday-ui.ts')
    const { localeApi } = await import('/src/ui-locale.browser.ts')
    localeApi.locale.set('zh-CN')
    const opened = []
    const items = ['北岸记录', '南岸记录'].map((title, i) => ({
      key: String(i), title, detail: '合成条目', async open() { opened.push(title) },
    }))
    document.getElementById('open').onclick = () => openEverydayPicker('快捷打开验收', items, () => {})
    window.pickerOpened = opened
  })
  const trigger = page.locator('#open')
  const dialog = page.getByRole('dialog', { name: '快捷打开验收', exact: true })
  const search = dialog.getByRole('searchbox')
  const activeSearch = () => search.evaluate(element => document.activeElement === element)
  const opened = () => page.evaluate(() => window.pickerOpened)
  const sendKey = (key, keyCode) => search.dispatchEvent('keydown', {
    key, keyCode, bubbles: true, cancelable: true, isComposing: false,
  })

  await trigger.focus()
  await trigger.press('Enter')
  await search.waitFor()
  await search.dispatchEvent('compositionstart', {})
  await search.fill('南岸')
  assert.equal(await dialog.getByRole('button', { name: /北岸记录/ }).count(), 1,
    'The list must not render the uncommitted text')
  await sendKey('Enter', 13)
  if (process.env.EXPECT_STALE_ACTIVATION) {
    assert.deepEqual(await opened(), ['北岸记录'])
    assert.equal(await dialog.count(), 0)
    console.log('Stock picker ran the old first result while the new candidate was composing.')
  } else {
    assert.deepEqual(await opened(), [])
    assert.ok(await activeSearch())
    await sendKey('ArrowDown', 40)
    assert.ok(await activeSearch(), 'The active input keeps its ArrowDown')
    await sendKey('Escape', 27)
    await dialog.dispatchEvent('cancel', { cancelable: true })
    await page.keyboard.press('Escape')
    assert.equal(await dialog.count(), 1, 'a real key cannot dismiss the picker during DOM composition')
    assert.ok(await activeSearch(), 'The composition Escape cannot close the dialog')
    // The compositor is synthetic; a real Escape in an HTML search field
    // may clear its value in Chromium without an OS candidate window.
    // Model the subsequent committed IME text explicitly before settling.
    await search.fill('南岸')
    await search.dispatchEvent('compositionend', {})
    await dialog.getByRole('button', { name: /南岸记录/ }).waitFor()
    assert.equal(await dialog.getByRole('button', { name: /北岸记录/ }).count(), 0)
    await sendKey('Enter', 229)
    assert.deepEqual(await opened(), [])
    await search.press('Enter')
    assert.deepEqual(await opened(), ['南岸记录'])
    assert.equal(await dialog.count(), 0)
    assert.ok(await trigger.evaluate(element => document.activeElement === element))
    await trigger.press('Enter')
    await search.press('ArrowUp')
    assert.ok(await dialog.getByRole('button', { name: /南岸记录/ }).evaluate(el => document.activeElement === el))
    await page.keyboard.press('Escape')
    assert.equal(await dialog.count(), 0)
    assert.deepEqual(await opened(), ['南岸记录'])
    console.log('Composition keeps text/focus; commit refreshes results; distinct Enter opens the correct item.')
  }
  assert.deepEqual(errors, [])
} finally {
  await browser?.close()
  await vite.close()
}
