// Actual Data panel and typed form over a synthetic Core. No native-host claim.
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
  await page.route(url + '/native-list-fixture', route => route.fulfill({
    contentType: 'text/html',
    body: '<!doctype html><meta charset=utf-8><main id=owned-fixture></main>',
  }))
  await page.goto(url + '/native-list-fixture')
  await page.evaluate(async () => {
    const { DataPanel } = await import('/src/data-panel.ts')
    const { locale } = await import('/src/ui-locale.ts')
    await import('/src/style.css')
    await import('/src/data-view.css')
    locale.set('zh-CN')
    const hash = async text => [...new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text)))].map(byte => byte.toString(16).padStart(2, '0')).join('')
    const int = value => ({ kind: 'int', value })
    const fieldId = 'owned/values', key = 'values', typeId = 'owned/test'
    const schema = {
      definitions: {}, aliases: {},
      fields: [{ id: fieldId, key, shape: { kind: 'list', items: { kind: 'scalar', semantic: 'int' }, min: 0, max: null }, nullable: false }],
      types: [{ id: typeId, fields: [{ id: fieldId, required: true }], requires: [], conflicts: [] }],
    }
    const schemaSource = JSON.stringify(schema)
    let values = ['1', '2', '3'].map(int)
    let basis = { inputId: 'one', sessionId: 'fixture', source: 'Synthetic property note', serial: 1, baseId: 'base', path: '/owned/note.typ', root: '/owned' }
    let projected = values
    const fixture = window.testFixture = { edits: [], installed: [], notices: [] }
    const panel = new DataPanel({
      getDocument: () => basis,
      canEdit: () => true,
      notify: (notice, error) => fixture.notices.push(error),
      applyPatch: async patch => {
        fixture.installed.push(patch)
        values = projected
        basis = { ...basis, serial: basis.serial + 1, inputId: 'next', source: patch.text }
      },
      request: async (path, body) => {
        if (path === '/api/data/schema') return { schema, source: schemaSource, revision: null, canInstall: false, builtinDefault: true }
        if (path === '/api/data/project') return { serial: body.serial, sourceHash: await hash(body.source), schemaHash: await hash(schemaSource), schemaRevision: null, readOnly: false,
          projection: { qualification: 'qualified', id: null, types: [typeId], resolvedTypes: [typeId], fields: [{
            fieldId, key, cell: { state: 'value', value: { kind: 'array', value: values } }, sourceRange: null, raw: null, writable: true, required: true, unbound: [],
          }], unknownFields: [], diagnostics: [], budgetExhausted: false } }
        if (path === '/api/data/edit') {
          fixture.edits.push(body)
          projected = body.value.value
          return { from: 0, to: body.source.length, text: JSON.stringify(projected) }
        }
        throw Error('Unexpected fixture request ' + path)
      },
    })
    document.querySelector('#owned-fixture').append(panel.element)
    await panel.refresh()
  })
  const field = page.locator('[data-field-key=values]')
  const rows = field.locator('.native-list-item')
  const inputs = () => rows.locator('input').evaluateAll(nodes => nodes.map(input => [input.value, input.validity.valid]))
  const input = i => rows.nth(i).locator('input')
  const move = (i, name) => rows.nth(i).getByRole('button', { name, exact: true })
  const apply = field.getByRole('button', { name: '应用到草稿', exact: true })
  const focused = control => control.evaluate(element => element === document.activeElement)
  await input(2).fill('4')
  await input(0).fill('1e')
  assert.deepEqual(await inputs(), [['1e', false], ['2', true], ['4', true]])
  await move(1, '上移').press('Enter')
  if (process.env.WEFTEXT_LIST_BASELINE) {
    assert.deepEqual(await inputs(), [['2', true], ['1', true], ['4', true]])
    assert.equal(await focused(move(0, '下移')), false)
    await apply.press('Enter')
    await page.waitForFunction(() => window.testFixture.edits.length === 1)
    assert.deepEqual(await page.evaluate(() => window.testFixture.edits[0].value.value.map(x => x.value)), ['2', '1', '4'])
    console.log('Old properties silently restore the invalid sibling and send a patch without it')
  } else {
    assert.deepEqual(await inputs(), [['1e', false], ['2', true], ['4', true]])
    assert.equal(await focused(input(0)), true)
    await field.getByRole('button', { name: '添加一项', exact: true }).press('Enter')
    assert.equal(await focused(input(0)), true)
    await apply.press('Enter')
    assert.equal(await page.evaluate(() => window.testFixture.edits.length), 0)
    await input(0).fill('7')
    await move(1, '上移').press('Enter')
    assert.deepEqual(await inputs(), [['2', true], ['7', true], ['4', true]])
    assert.equal(await focused(move(0, '下移')), true)
    await apply.press('Enter')
    await page.waitForFunction(() => window.testFixture.installed.length === 1)
    assert.deepEqual(await page.evaluate(() => window.testFixture.edits[0].value.value.map(x => x.value)), ['2', '7', '4'])
    await field.getByRole('button', { name: '添加一项', exact: true }).press('Enter')
    assert.equal(await focused(input(3)), true)
    await rows.nth(3).getByRole('button', { name: '移除', exact: true }).press('Enter')
    assert.equal(await focused(rows.nth(2).getByRole('button', { name: '移除', exact: true })), true)
    assert.equal(await page.evaluate(() => window.testFixture.edits.length), 1)
    console.log('Property draft, rejection without a write, valid keyboard move/add/remove focus, and exact Apply passed')
  }
  assert.deepEqual(errors, [])
} finally {
  await browser?.close()
  await vite.close()
}
