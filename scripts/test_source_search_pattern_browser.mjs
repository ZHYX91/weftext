// Probe actual SourceEditor controls on a synthetic document, without Core or native IME.
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
  await search.fill('[')
  const state = () => page.evaluate(() => {
    const { editor, initial, getSearchQuery } = window.searchFixture
    const state = editor.view.state, selection = state.selection.main
    const input = editor.view.dom.querySelector('input[name=search]')
    return {
      selected: state.sliceDoc(selection.from, selection.to),
      finalUnchanged: editor.source === initial,
      queryValid: getSearchQuery(state).valid,
      focusRetained: document.activeElement === input,
      inputValid: input.validity.valid,
      invalidAria: input.getAttribute('aria-invalid'),
      descriptions: input.getAttribute('aria-describedby'),
      nextEnabled: !editor.view.dom.querySelector('.cm-search [name=next]').disabled,
      message: document.getElementById(input.getAttribute('aria-describedby'))?.textContent,
      replaceEnabled: !editor.view.dom.querySelector('.cm-search [name=replaceAll]').disabled,
    }
  })
  await search.press('Enter')
  const literal = await state()
  assert(literal.queryValid && literal.selected === '[')
  await page.locator('.cm-search input[name=re]').check()
  await search.focus()
  await search.press('Enter')
  const malformed = await state()
  assert.equal(malformed.queryValid, false)
  assert(malformed.finalUnchanged && malformed.focusRetained)
  assert(malformed.invalidAria === 'true' && !malformed.inputValid)
  assert(malformed.descriptions && malformed.message?.includes('正则表达式不完整或有误'))
  assert(!malformed.nextEnabled && !malformed.replaceEnabled)
  await page.evaluate(async () => (await import('/src/ui-locale.browser.ts')).localeApi.locale.set('en-US'))
  const translated = await state()
  assert(translated.focusRetained && translated.descriptions === malformed.descriptions)
  assert(translated.message?.startsWith('This regular expression is incomplete or invalid.'))
  await page.locator('.cm-search input[name=re]').uncheck()
  const plainAgain = await state()
  assert(plainAgain.queryValid && plainAgain.inputValid && plainAgain.nextEnabled && !plainAgain.message)
  await page.locator('.cm-search input[name=re]').check()
  await search.fill('潮汐')
  await search.press('Enter')
  const corrected = await state()
  assert(corrected.queryValid && corrected.selected === '潮汐' && corrected.finalUnchanged)
  assert(corrected.inputValid && corrected.invalidAria === null && !corrected.message && corrected.replaceEnabled)
  await search.fill('')
  const empty = await state()
  assert(empty.inputValid && !empty.message)
  assert.deepEqual(errors, [])
  console.log('Literal search, localized malformed-pattern feedback, recovery, and unchanged source passed')
} finally {
  await browser?.close()
  await vite.close()
}
