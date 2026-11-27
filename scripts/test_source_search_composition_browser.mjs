// Exercise typed DOM composition event boundaries in the actual SourceEditor, not an OS IME.
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
  const page = await browser.newPage({ viewport: { width: 640, height: 900 }, locale: 'zh-CN' })
  const errors = []
  page.on('pageerror', error => errors.push(error.message))
  await page.route(url + '/source-search-fixture', route => route.fulfill({
    contentType: 'text/html',
    body: '<!doctype html><html lang="zh-CN"><meta charset=utf-8><main id="proof"></main>',
  }))
  await page.goto(url + '/source-search-fixture')
  await page.evaluate(async () => {
    const { SourceEditor, getSearchQuery, localeApi } = await import('/src/ui-locale.browser.ts')
    localeApi.locale.set('zh-CN')
    const source = '序言\n[潮汐] 第一节\n下一节潮汐记录'
    const editor = new SourceEditor(source, () => {}, () => {})
    document.getElementById('proof').append(editor.view.dom)
    window.searchFixture = { editor, initial: source, getSearchQuery }
  })
  await page.locator('.cm-content').click()
  await page.keyboard.press('Control+f')
  const search = page.locator('.cm-search input[name=search]')

  const replacement = page.locator('.cm-search input[name=replace]')
  await search.fill('潮汐')
  await search.press('Enter')
  const snapshot = () => page.evaluate(() => {
    const { editor } = window.searchFixture
    const selected = editor.view.state.selection.main
    return { source: editor.source, from: selected.from, to: selected.to }
  })
  const key = (field, extra) => field.dispatchEvent('keydown', { key: 'Enter', code: 'Enter', bubbles: true, cancelable: true, ...extra })
  const original = await snapshot()
  assert.equal(original.to - original.from, 2, 'Initially selected a valid literal match')

  // These are synthetic DOM event shapes, not a real OS IME claim.
  await search.dispatchEvent('compositionstart', {})
  await key(search, { isComposing: false, keyCode: 13 })
  assert.deepEqual(await snapshot(), original, 'Find must not advance while an input composition is still active')
  await search.dispatchEvent('compositionend', {})
  await key(search, { isComposing: false, keyCode: 229 })
  assert.deepEqual(await snapshot(), original, 'Find must ignore the 229 confirmation even after compositionend')
  await search.press('Enter')
  const advanced = await snapshot()
  assert.equal(advanced.source, original.source)
  assert.notEqual(advanced.from, original.from, 'A distinct Enter still navigates')

  await replacement.fill('新字')
  await replacement.dispatchEvent('compositionstart', {})
  await key(replacement, { isComposing: true, keyCode: 13 })
  assert.deepEqual(await snapshot(), advanced, 'Replace ignores explicitly composing Enter')
  await key(replacement, { isComposing: false, keyCode: 13 })
  assert.deepEqual(await snapshot(), advanced, 'Replace must not modify source while an input composition is still active')
  await replacement.dispatchEvent('compositionend', {})
  await key(replacement, { isComposing: false, keyCode: 229 })
  assert.deepEqual(await snapshot(), advanced, 'Replace must ignore 229 after compositionend')
  await replacement.press('Enter')
  assert.equal((await snapshot()).source, original.source.slice(0, advanced.from) + '新字' + original.source.slice(advanced.to))
  await page.evaluate(() => window.searchFixture.editor.undo())
  assert.equal((await snapshot()).source, original.source)
  assert.deepEqual(errors, [])
  console.log('Synthetic composition events preserve Find/Replace selection and bytes, and ordinary Enter/Undo still work')
} finally {
  await browser?.close()
  await vite.close()
}
