// Real headless pointer/keyboard checks of the shared graph and heading views.
// Uses public display fixtures only: no Core/HTTP/OS-IME acceptance is implied.
import { createRequire } from 'node:module';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import assert from 'node:assert/strict';
const require = createRequire(import.meta.url), { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..'), out = path.resolve(process.argv[2]);
await mkdir(out, { recursive: false });
const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
const vite = await createServer({ root: path.join(repo, 'app'), configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false } });
let browser, page; const result = { cases: [], measurements: [], pageErrors: [], completed: false, scope: 'shared frontend actual browser display fixtures; no backend or real-user data' };
try {
  await vite.listen(); browser = await chromium.launch({ channel: 'chrome', headless: true }); result.browser = browser.version(); page = await browser.newPage({ viewport: { width: 1440, height: 1100 }, reducedMotion: 'reduce' }); page.on('pageerror', error => result.pageErrors.push(String(error)));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/`);
  await page.evaluate(async () => {
    const NativeResizeObserver = ResizeObserver, nativeRaf = requestAnimationFrame, nativeCancelRaf = cancelAnimationFrame;
    window.canvasResizeRecords = []; window.canvasFrames = { scheduled: 0, cancelled: 0 };
    window.ResizeObserver = class {
      constructor(callback) { this.record = { active: true, observes: 0, disconnects: 0, targets: [] }; window.canvasResizeRecords.push(this.record); this.native = new NativeResizeObserver(entries => callback(entries, this)); }
      observe(target, options) { this.record.observes++; this.record.targets.push(target); this.native.observe(target, options); }
      unobserve(target) { this.native.unobserve(target); }
      disconnect() { this.record.active = false; this.record.disconnects++; this.native.disconnect(); }
    };
    window.requestAnimationFrame = callback => { window.canvasFrames.scheduled++; return nativeRaf.call(window, callback); };
    window.cancelAnimationFrame = id => { window.canvasFrames.cancelled++; return nativeCancelRaf.call(window, id); };
    const { RelationshipGraphView } = await import('/src/relationship-graph-view.ts'), { HeadingStructureView } = await import('/src/heading-structure-view.ts');
    document.head.append(Object.assign(document.createElement('link'), { rel: 'stylesheet', href: '/src/knowledge-view.css' }));
    document.body.innerHTML = '<main id="graph"></main><main id="headings"></main>'; window.graphEvents = [];
    const node = key => ({ key, title: key, reference: key, directory: 'notes', tags: key === 'B' ? ['weftext-tag://w/child'] : [], tagsKnown: true });
    const edge = (key, source, target, relation, direction = 'directed', derived = false) => ({ key, source, target, relation, direction, derived, authorSource: 'A', fieldId: key.startsWith('body') ? null : 'tasks/dependencies', itemPath: [], sourceHash: 'basis', range: null });
    window.graphFixture = { generation: 'same-cut', scope: 'authorizedNativeSourcesAndTypedReferences', coverage: 'partial', truncated: false, tagTree: [{ reference: 'weftext-tag://w/root', tagId: 'root', displayName: '父标签', parentTagId: null }, { reference: 'weftext-tag://w/child', tagId: 'child', displayName: '直接标签', parentTagId: 'root' }], nodes: ['A', 'B', 'C'].map(node), edges: [edge('body1', 'A', 'B', '正文链接'), edge('body2', 'A', 'B', '正文链接'), edge('typed', 'A', 'B', 'tasks/dependency'), edge('inverse', 'B', 'A', 'dependent', 'directed', true), edge('sym', 'B', 'C', 'sibling', 'symmetric')] };
    window.originalGraph = JSON.stringify(window.graphFixture);
    window.RelationshipGraphView = RelationshipGraphView; window.graphView = new RelationshipGraphView(); window.graphView.render(document.querySelector('#graph'), window.graphFixture, 'A', { open: node => window.graphEvents.push(['open', node.key]), source: (edge, node) => window.graphEvents.push(['source', node.key, edge.derived]), error: error => { throw error; } });
    const headings = ['文档根', '深层', '命中', '命中', '生成标题'].map((text, index) => ({ kind: 'heading', key: `h${index}`, text, number: index === 0 ? '一' : null, level: [1, 4, 5, 2, 1][index], generated: index === 4, range: index === 4 ? null : { utf16From: index * 10, utf16To: index * 10 + 4 } }));
    headings.splice(1, 0, { kind: 'figure', key: 'caption', text: '题注', number: '1', nativeKind: 'figure', level: 0, generated: true, range: null });
    window.headingFixture = { coverage: 'complete', truncated: false, items: headings }; window.originalHeadings = JSON.stringify(window.headingFixture);
    window.HeadingStructureView = HeadingStructureView; window.headingView = new HeadingStructureView(); window.renderHeadings = () => window.headingView.render(document.querySelector('#headings'), window.headingFixture, heading => window.graphEvents.push(['heading', heading.key, heading.generated]), undefined, '文档根'); window.renderHeadings();
  });
  const graph = page.locator('#graph'), headings = page.locator('#headings');
  await graph.getByText(/已展示 2 个节点、4 条关系/).waitFor();
  await graph.getByLabel('在当前图范围搜索定位', { exact: true }).fill('C'); await graph.getByText('当前范围没有匹配；需要扩大范围时请明确修改范围或筛选。', { exact: true }).waitFor(); assert.equal(await graph.getByLabel('范围', { exact: true }).inputValue(), 'local');
  await graph.getByLabel('范围', { exact: true }).selectOption('global'); await graph.getByText(/已展示 3 个节点、5 条关系/).waitFor(); await graph.getByRole('button', { name: '定位 C', exact: true }).click(); await graph.getByRole('heading', { name: 'C', exact: true }).waitFor();
  await graph.getByText('键盘与完整结果列表', { exact: true }).click(); await graph.locator('details').getByRole('button', { name: 'B → A · dependent（派生反向）', exact: true }).click(); await graph.getByText(/派生反向；作者事实保存在原来源/).waitFor(); await graph.getByRole('button', { name: '打开准确作者来源', exact: true }).click(); assert.deepEqual(await page.evaluate(() => window.graphEvents.pop()), ['source', 'A', true]); result.cases.push('local/global, search without implicit range expansion and parallel/derived-edge author provenance');
  const nodeB = graph.locator('g.knowledge-graph-node[data-graph-key="B"]');
  await nodeB.focus(); await nodeB.press(' '); await graph.getByRole('heading', { name: 'B', exact: true }).waitFor(); await nodeB.press('Enter'); assert.deepEqual(await page.evaluate(() => window.graphEvents.pop()), ['open', 'B']);
  await graph.getByRole('button', { name: '适应视口', exact: true }).click(); await nodeB.dblclick(); assert.deepEqual(await page.evaluate(() => window.graphEvents.pop()), ['open', 'B']);
  await page.setViewportSize({ width: 480, height: 900 });
  await page.waitForFunction(() => { const svg = document.querySelector('#graph svg'), style = getComputedStyle(svg); return Math.abs(svg.viewBox.baseVal.width - (svg.getBoundingClientRect().width - parseFloat(style.borderLeftWidth) - parseFloat(style.borderRightWidth))) < .1; });
  const beforeDrag = await nodeB.getAttribute('transform'), box = await nodeB.boundingBox(); await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2); await page.mouse.down(); await page.mouse.move(box.x + box.width / 2 + 35, box.y + box.height / 2 + 24, { steps: 4 }); await page.mouse.up(); assert.notEqual(await nodeB.getAttribute('transform'), beforeDrag); const moved = await nodeB.boundingBox(); assert.ok(Math.abs(moved.x - box.x - 35) < 2 && Math.abs(moved.y - box.y - 24) < 2, 'SVG viewBox/letterbox drag stays under pointer'); assert.equal(await page.evaluate(() => JSON.stringify(window.graphFixture) === window.originalGraph), true); await page.setViewportSize({ width: 1440, height: 1100 }); result.cases.push('mouse double-open and keyboard selection/open preserve live canvas; narrow viewBox dragging follows pointer and changes only personal layout');
  assert.equal(await graph.getByLabel('标签匹配模式', { exact: true }).inputValue(), 'subtree'); await graph.getByLabel('图谱标签条件', { exact: true }).selectOption('weftext-tag://w/root'); await graph.getByText(/已展示 1 个节点、0 条关系/).waitFor(); await graph.getByLabel('标签匹配模式', { exact: true }).selectOption('exact'); await graph.getByText(/已展示 0 个节点、0 条关系/).waitFor(); await graph.getByRole('button', { name: '清除标签条件', exact: true }).click(); await graph.getByText(/已展示 3 个节点、5 条关系/).waitFor(); assert.equal(await page.evaluate(() => JSON.stringify(window.graphFixture) === window.originalGraph), true); result.cases.push('shared complete TagTree picker defaults to subtree; exact switch and clearing only change personal graph conditions');
  await page.evaluate(() => { const host = document.createElement('main'); host.id = 'solo-graph'; host.style.width = '264px'; host.style.display = 'none'; document.body.append(host); window.soloGraphFixture = { ...window.graphFixture, coverage: 'complete', nodes: [window.graphFixture.nodes[0]], edges: [] }; window.originalSoloGraph = JSON.stringify(window.soloGraphFixture); window.soloGraphView = new window.RelationshipGraphView(); const render = () => window.soloGraphView.render(host, window.soloGraphFixture, 'A', { open() {}, source() {}, error(error) { throw error; } }); render(); render(); host.style.display = ''; });
  const solo = page.locator('#solo-graph');
  await page.waitForFunction(() => window.soloGraphView.canvas.viewport?.width > 0);
  const measureSolo = async phase => { const entry = await solo.locator('.knowledge-graph-node text').evaluate(element => { const matrix = element.getScreenCTM(), point = new DOMPoint(0, 0).matrixTransform(matrix).matrixTransform(element.ownerSVGElement.getScreenCTM().inverse()), viewport = element.ownerSVGElement.viewBox.baseVal; return { scale: Math.hypot(matrix.a, matrix.b), effectiveFontCssPx: parseFloat(getComputedStyle(element).fontSize) * Math.hypot(matrix.a, matrix.b), x: point.x, y: point.y, width: viewport.width, height: viewport.height }; }); result.measurements.push({ phase, ...entry }); return entry; };
  const soloInitial = await measureSolo('generic-initial-hidden-264'); assert.ok(soloInitial.effectiveFontCssPx >= 11.9); assert.ok(Math.abs(soloInitial.x - soloInitial.width / 2) < .1 && Math.abs(soloInitial.y - soloInitial.height / 2) < .1);
  await solo.getByRole('button', { name: '100%', exact: true }).click(); await solo.locator('svg').focus(); for (let index = 0; index < 10; index++) await solo.locator('svg').press('ArrowRight');
  await solo.getByLabel('在当前图范围搜索定位', { exact: true }).fill('A'); await solo.getByRole('button', { name: '定位 A', exact: true }).click();
  await page.waitForFunction(() => document.querySelector('#solo-graph svg')?.viewBox.baseVal.width === window.soloGraphView.canvas.viewport?.width);
  const soloLocated = await measureSolo('generic-locate-264'); assert.ok(Math.abs(soloLocated.scale - 1) < .01 && soloLocated.effectiveFontCssPx >= 11.9); assert.ok(Math.abs(soloLocated.x - soloLocated.width / 2) < .1 && Math.abs(soloLocated.y - soloLocated.height / 2) < .1);
  await solo.evaluate(element => { element.style.width = '420px'; });
  await page.waitForFunction(() => window.soloGraphView.canvas.viewport?.width > 400);
  const soloResized = await measureSolo('generic-resize-420'); assert.ok(Math.abs(soloResized.scale - 1) < .01); assert.ok(Math.abs(soloResized.x - soloResized.width / 2) < .1 && Math.abs(soloResized.y - soloResized.height / 2) < .1);
  await solo.screenshot({ path: path.join(out, 'generic-readable-420.png') });
  assert.equal(await page.evaluate(() => JSON.stringify(window.soloGraphFixture) === window.originalSoloGraph), true); await page.evaluate(() => { window.soloGraphView.clear(); document.querySelector('#solo-graph').remove(); }); result.cases.push('generic graph first hidden Fit, readable 264px Locate and 420px resize use the same CSS viewport without moving author facts');
  assert.equal(await headings.locator('.outline-row').count(), 6);
  const firstRow = headings.locator('li[data-heading-key="h0"]'); await firstRow.getByRole('button', { name: '折叠分支', exact: true }).click(); assert.equal(await headings.locator('.outline-row').count(), 2);
  const search = () => headings.locator('[data-outline-control="search"]');
  await search().fill('命中'); assert.equal(await headings.locator('.outline-row').count(), 4); await headings.getByText('2 个大纲项命中', { exact: true }).waitFor();
  const eventsBeforeSearch = await page.evaluate(() => window.graphEvents.length);
  await search().press('Enter'); assert.equal(await headings.locator('.outline-row.is-selected').getAttribute('data-heading-key'), 'h2'); await headings.getByText('第 1 / 2 个命中', { exact: true }).waitFor();
  await search().press('Shift+Enter'); assert.equal(await headings.locator('.outline-row.is-selected').getAttribute('data-heading-key'), 'h3');
  await search().press('Enter'); assert.equal(await headings.locator('.outline-row.is-selected').getAttribute('data-heading-key'), 'h2');
  await search().evaluate(element => { element.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true })); element.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', keyCode: 229, bubbles: true, cancelable: true })); window.renderHeadings(); });
  assert.equal(await headings.locator('.outline-row.is-selected').getAttribute('data-heading-key'), 'h2');
  await search().evaluate(element => element.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true })));
  assert.equal(await page.evaluate(() => window.graphEvents.length), eventsBeforeSearch);
  await search().press('Escape'); assert.equal(await headings.locator('.outline-row').count(), 2); result.cases.push('Enter and Shift+Enter cycle exact duplicate matches; IME and projection refresh preserve search, and clearing restores branches without navigation');
  await headings.locator('.outline-options > summary').click(); await headings.getByRole('button', { name: '全部展开', exact: true }).click();
  await headings.locator('[data-outline-control="map"]').click();
  const card = key => headings.locator(`g.heading-map-card[data-graph-key="${key}"]`), branch = key => headings.locator(`[data-heading-branch="${key}"]`);
  assert.equal(await card('@weftext-document-root').count(), 1); assert.equal(await card('h0').count(), 1); assert.equal(await headings.locator('g.heading-map-card').count(), 6);
  assert.ok((await card('h0').locator('[data-outline-control="map-item:h0"]').getAttribute('aria-label')).includes('一 文档根'));
  assert.equal(await branch('@weftext-document-root').getAttribute('aria-label'), '折叠 2 个直接子标题'); assert.equal(await branch('h0').getAttribute('aria-label'), '折叠 2 个直接子标题');
  await branch('h0').click(); assert.equal(await headings.locator('g.heading-map-card').count(), 3); assert.equal(await branch('h0').getAttribute('aria-expanded'), 'false');
  await branch('h0').press(' '); assert.equal(await headings.locator('g.heading-map-card').count(), 6); assert.equal(await branch('h0').getAttribute('aria-expanded'), 'true');
  await branch('h0').evaluate(element => element.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', keyCode: 229, bubbles: true, cancelable: true }))); assert.equal(await headings.locator('g.heading-map-card').count(), 6);
  assert.equal(await page.evaluate(() => window.graphEvents.length), eventsBeforeSearch); result.cases.push('same-name document root and H1 remain separate; native number and caption-free direct child handles fold only their own branch by pointer or keyboard');
  const horizontal = await card('h1').getAttribute('transform'); await headings.locator('[data-outline-control="map-vertical"]').click(); assert.notEqual(await card('h1').getAttribute('transform'), horizontal);
  const drawing = headings.locator('svg'); await drawing.focus(); for (let index = 0; index < 25; index++) await drawing.press('ArrowRight');
  await search().fill('命中'); await search().press('Enter');
  const selectedPosition = await card('h2').evaluate(element => { const box = element.getBBox(), point = new DOMPoint(box.x + box.width / 2, box.y + box.height / 2).matrixTransform(element.getScreenCTM()).matrixTransform(element.ownerSVGElement.getScreenCTM().inverse()), viewport = element.ownerSVGElement.viewBox.baseVal; return { x: point.x, y: point.y, width: viewport.width, height: viewport.height }; });
  assert.ok(selectedPosition.x >= 12 && selectedPosition.x <= selectedPosition.width - 12 && selectedPosition.y >= 12 && selectedPosition.y <= selectedPosition.height - 12, 'the visited match is brought into the actual map viewBox independently of CSS scaling');
  await search().press('Shift+Enter'); assert.equal(await card('h3').getAttribute('class').then(value => value.includes('is-selected')), true);
  await search().press('Escape'); await card('h4').locator('[data-outline-control="map-item:h4"]').press('Enter'); assert.deepEqual(await page.evaluate(() => window.graphEvents.pop()), ['heading', 'h4', true]);
  assert.equal(await page.evaluate(() => JSON.stringify(window.headingFixture) === window.originalHeadings), true); result.cases.push('map directions, search camera reveal and generated-heading Enter navigation preserve original native facts');
  const longNumber = '12.3.4.5.6.7.8.9.10.11.12', longTitle = '任意深层级标题的完整中文标题与原生编号';
  await page.evaluate(({ longNumber, longTitle }) => { window.headingFixture = structuredClone(window.headingFixture); const item = window.headingFixture.items.find(item => item.key === 'h0'); item.number = longNumber; item.text = longTitle; window.renderHeadings(); window.longHeadingOriginal = JSON.stringify(window.headingFixture); }, { longNumber, longTitle });
  assert.equal(await card('h0').locator('[data-outline-control="map-item:h0"]').getAttribute('aria-label'), `${longNumber} ${longTitle}`);
  assert.equal(await card('h0').locator('.heading-map-number > title').textContent(), longNumber);
  assert.ok((await card('h0').locator('.heading-map-number').evaluate(element => [...element.childNodes].filter(node => node.nodeType === Node.TEXT_NODE).map(node => node.textContent).join(''))).endsWith('…'));
  const clips = await card('h0').evaluate(element => { const number = element.querySelector('.heading-map-number'), title = element.querySelector('[data-outline-control] > text:not(.heading-map-number)'); return [number, title].map(item => { const id = /url\(#(.+)\)/.exec(item.getAttribute('clip-path'))[1], clip = document.getElementById(id).querySelector('rect'); return Number(clip.getAttribute('width')); }); });
  assert.deepEqual(clips, [40, 159]); assert.equal(await page.evaluate(() => JSON.stringify(window.headingFixture) === window.longHeadingOriginal), true); result.cases.push('long native numbering and titles stay in fixed card regions while their full original values remain accessible');
  const measure = async (width, phase) => {
    const entry = await headings.locator('.heading-map-card [data-outline-control^="map-item:"] > text:not(.heading-map-number)').first().evaluate(element => {
      const matrix = element.getScreenCTM(), svg = element.ownerSVGElement, rectangle = svg.getBoundingClientRect(), font = parseFloat(getComputedStyle(element).fontSize), viewport = svg.viewBox.baseVal;
      const card = element.closest('.heading-map-card'), body = card.querySelector('[data-outline-control^="map-item:"] > rect'), corners = [new DOMPoint(Number(body.getAttribute('x')), -23), new DOMPoint(Number(body.getAttribute('x')) + Number(body.getAttribute('width')), 23)].map(point => point.matrixTransform(body.getScreenCTM()));
      return { fontSize: font, effectiveFontCssPx: font * Math.hypot(matrix.a, matrix.b), transformScale: Math.hypot(matrix.a, matrix.b), cssWidth: rectangle.width, cssHeight: rectangle.height, viewBoxWidth: viewport.width, viewBoxHeight: viewport.height, cardInsideViewport: corners.every(point => point.x >= rectangle.left && point.x <= rectangle.right && point.y >= rectangle.top && point.y <= rectangle.bottom), activeObservers: window.canvasResizeRecords.filter(record => record.active).length };
    });
    result.measurements.push({ width, phase, ...entry });
    assert.ok(Math.abs(entry.transformScale - 1) < .01, `${phase}: 100% is one actual CSS pixel per world unit`);
    assert.ok(entry.effectiveFontCssPx >= 11.9, `${phase}: native heading title is readable at 12 CSS px`);
    return entry;
  };
  await page.evaluate(() => { const host = document.createElement('main'); host.id = 'initial-hidden-heading'; host.style.width = '264px'; host.style.display = 'none'; document.body.append(host); window.hiddenHeadingView = new window.HeadingStructureView(); const render = () => window.hiddenHeadingView.render(host, window.headingFixture, () => {}, undefined, '文档根'); render(); host.querySelector('[data-outline-control="map"]').click(); render(); host.style.display = ''; });
  await page.waitForFunction(() => window.hiddenHeadingView.canvas.viewport?.width > 0);
  const firstHidden = await page.locator('#initial-hidden-heading .heading-map-card').first().evaluate(element => { const body = element.querySelector('[data-outline-control^="map-item:"] > rect'), rect = body.getBoundingClientRect(), svg = element.ownerSVGElement.getBoundingClientRect(), text = element.querySelector('[data-outline-control^="map-item:"] > text'), matrix = text.getScreenCTM(); return { effectiveFontCssPx: parseFloat(getComputedStyle(text).fontSize) * Math.hypot(matrix.a, matrix.b), rootInsideViewport: rect.left >= svg.left && rect.right <= svg.right && rect.top >= svg.top && rect.bottom <= svg.bottom }; });
  result.measurements.push({ width: 264, phase: 'initial-hidden-rerender', ...firstHidden }); assert.ok(firstHidden.effectiveFontCssPx >= 11.9); assert.equal(firstHidden.rootInsideViewport, true);
  await page.evaluate(() => { window.hiddenHeadingView.clear(); document.querySelector('#initial-hidden-heading').remove(); });
  for (const width of [264, 420, 1100]) {
    await headings.evaluate((element, width) => { element.style.width = `${width}px`; }, width);
    await headings.locator('[data-outline-control="map-horizontal"]').click();
    await page.waitForFunction(() => { const svg = document.querySelector('#headings svg'), style = getComputedStyle(svg); return Math.abs(svg.viewBox.baseVal.width - (svg.getBoundingClientRect().width - parseFloat(style.borderLeftWidth) - parseFloat(style.borderRightWidth))) < .1; });
    const initial = await measure(width, 'initial'); assert.equal(initial.cardInsideViewport, true, 'document root and both controls fit in the real sidebar');
    await headings.getByRole('button', { name: '适应视口', exact: true }).click();
    const fitScale = await page.evaluate(() => window.headingView.canvas.camera.scale);
    await card('@weftext-document-root').locator('[data-outline-control="map-item:@weftext-document-root"]').focus();
    await branch('@weftext-document-root').focus();
    const focusScale = await page.evaluate(() => window.headingView.canvas.camera.scale); assert.equal(focusScale, fitScale, 'ordinary keyboard focus preserves an explicitly fitted overview');
    result.measurements.push({ width, phase: 'explicit-fit-focus', fitScale, focusScale });
    await headings.getByRole('button', { name: '100%', exact: true }).click(); await measure(width, '100-percent');
    await search().fill('命中'); await search().press('Enter');
    const searchEntry = await card('h2').locator('[data-outline-control="map-item:h2"] > text:not(.heading-map-number)').evaluate(element => { const m = element.getScreenCTM(); return { transformScale: Math.hypot(m.a, m.b), effectiveFontCssPx: parseFloat(getComputedStyle(element).fontSize) * Math.hypot(m.a, m.b) }; });
    result.measurements.push({ width, phase: 'search', ...searchEntry }); assert.ok(searchEntry.effectiveFontCssPx >= 11.9); assert.ok(Math.abs(searchEntry.transformScale - 1) < .01);
    await search().press('Escape');
    await headings.locator('[data-outline-control="map-vertical"]').click(); const vertical = await measure(width, 'vertical'); assert.equal(vertical.cardInsideViewport, true);
    await headings.screenshot({ path: path.join(out, `heading-readable-${width}.png`) });
  }
  await page.evaluate(() => { window.beforeHiddenCamera = JSON.stringify(window.headingView.canvas.camera); window.beforeHiddenViewport = JSON.stringify(window.headingView.canvas.viewport); document.querySelector('#headings').style.display = 'none'; });
  await page.waitForFunction(() => document.querySelector('#headings svg').getBoundingClientRect().width === 0);
  assert.equal(await page.evaluate(() => JSON.stringify(window.headingView.canvas.camera) === window.beforeHiddenCamera && JSON.stringify(window.headingView.canvas.viewport) === window.beforeHiddenViewport), true);
  await headings.evaluate(element => { element.style.display = ''; });
  await page.waitForFunction(() => document.querySelector('#headings svg').getBoundingClientRect().width > 0);
  await measure(1100, 'hidden-restored');
  await headings.locator('[data-outline-control="map-horizontal"]').click(); await headings.screenshot({ path: path.join(out, 'heading-readable-restored.png') });
  await page.evaluate(async () => {
    const { mountKnowledgePanel } = await import('/src/knowledge-panel.ts');
    const dock = document.createElement('div'), host = document.createElement('section'); dock.id = 'knowledge-owner-dock'; host.id = 'knowledge-owner'; dock.append(host); document.body.append(dock);
    const basis = { sessionId: 'owned-session', serial: 1, inputId: 'owned-input', inputRevision: 1, baseId: 'owned-basis', path: 'D:/Owned/Note/Note.typ', source: '= Owned source', root: 'D:/Owned', workspaceId: 'owned', nodeId: 'A', dirty: false };
    const snapshot = { graph: { generation: 'owned-graph', scope: 'authorizedNativeSourcesAndTypedReferences', coverage: 'complete', truncated: false, nodes: [{ key: 'owned/A', title: 'Panel owner', reference: 'weftext-node://owned/A', directory: 'notes', tags: [], tagsKnown: true }], edges: [] }, headingStructure: null, links: { generation: 'owned-links', coverage: 'complete', outgoing: [], incomingId: [], incomingPath: [] }, diagnostics: { coverage: 'complete', scannedNodes: 1, issues: [], nodes: [] }, renderedLinks: [], propertyReferences: { coverage: 'complete', items: [], diagnostics: [] }, citations: null, mentions: null, annotations: [], annotationStatus: 'valid', annotationDiagnostics: [], templates: [], trash: [], warnings: [], errors: [] };
    const owner = mountKnowledgePanel(host, { context: () => ({ ...basis }), load: async () => structuredClone(snapshot), prepare: async () => { throw new Error('No author preparation belongs to this owner-lifetime fixture'); }, commit: async () => { throw new Error('No author commit belongs to this owner-lifetime fixture'); }, navigate: () => {}, notify: () => {} });
    window.knowledgeOwnerFixture = { host, dock, owner }; owner.selectTab('graph'); await owner.refresh();
  });
  const ownerPanel = page.locator('#knowledge-owner'); await ownerPanel.locator('svg').waitFor();
  await page.evaluate(() => { const f = window.knowledgeOwnerFixture; f.oldSvg = f.host.querySelector('svg'); f.oldRecord = window.canvasResizeRecords.find(record => record.targets.includes(f.oldSvg)); f.dock.hidden = true; });
  await page.waitForFunction(() => window.knowledgeOwnerFixture.oldSvg.getBoundingClientRect().width === 0);
  assert.equal(await page.evaluate(() => window.knowledgeOwnerFixture.oldRecord.active && window.knowledgeOwnerFixture.oldSvg.isConnected), true, 'hiding a live Dock view does not retire its canvas');
  await page.evaluate(() => { const f = window.knowledgeOwnerFixture; f.dock.hidden = false; });
  await page.waitForFunction(() => window.knowledgeOwnerFixture.oldSvg.getBoundingClientRect().width > 0);
  await page.evaluate(() => window.knowledgeOwnerFixture.owner.selectTab('links'));
  assert.equal(await page.evaluate(() => window.knowledgeOwnerFixture.oldRecord.active), false, 'switching tabs retires the replaced canvas immediately');
  assert.equal(await ownerPanel.locator('.knowledge-graph-canvas').count(), 0);
  await page.evaluate(() => window.knowledgeOwnerFixture.owner.selectTab('graph')); await ownerPanel.locator('svg').waitFor();
  const panelLifetime = await page.evaluate(() => { const f = window.knowledgeOwnerFixture, nextSvg = f.host.querySelector('svg'), nextRecord = window.canvasResizeRecords.find(record => record.targets.includes(nextSvg)); const restored = nextSvg !== f.oldSvg && nextRecord.active; f.owner.dispose(); f.dock.remove(); return { restored, replacedObserverRetired: !f.oldRecord.active, disposedObserverRetired: !nextRecord.active }; });
  result.measurements.push({ phase: 'knowledge-panel-owner', ...panelLifetime }); assert.deepEqual(panelLifetime, { restored: true, replacedObserverRetired: true, disposedObserverRetired: true });
  result.cases.push('the real KnowledgePanel keeps a hidden Dock canvas live, retires it on graph-to-links replacement and recreates it safely before final owner disposal');
  const retirement = await page.evaluate(() => {
    const oldHeadingState = window.headingView.canvas, oldGraphState = window.graphView.canvas, oldHeading = document.querySelector('#headings .knowledge-graph-canvas'), oldGraph = document.querySelector('#graph .knowledge-graph-canvas');
    const cameraBefore = [JSON.stringify(oldHeadingState.camera), JSON.stringify(oldGraphState.camera)], framesBefore = { ...window.canvasFrames };
    document.querySelector('#headings').style.display = 'none'; window.headingView.clear(); window.graphView.clear();
    for (const canvas of [oldHeading, oldGraph]) { for (const button of canvas.querySelectorAll('button')) button.click(); canvas.querySelector('svg').dispatchEvent(new WheelEvent('wheel', { deltaY: -120, bubbles: true, cancelable: true })); }
    return { activeObservers: window.canvasResizeRecords.filter(record => record.active).length, createdObservers: window.canvasResizeRecords.length, disconnectedObservers: window.canvasResizeRecords.filter(record => record.disconnects > 0).length, framesBefore, framesAfter: { ...window.canvasFrames }, retiredEventsInert: cameraBefore[0] === JSON.stringify(oldHeadingState.camera) && cameraBefore[1] === JSON.stringify(oldGraphState.camera) };
  });
  result.measurements.push({ phase: 'hidden-retirement', ...retirement }); assert.equal(retirement.activeObservers, 0); assert.equal(retirement.createdObservers, retirement.disconnectedObservers); assert.equal(retirement.retiredEventsInert, true); assert.deepEqual(retirement.framesAfter, retirement.framesBefore);
  await headings.evaluate(element => { element.style.display = ''; }); await page.evaluate(() => window.renderHeadings()); await headings.locator('[data-outline-control="map"]').click();
  result.cases.push('264/420/wide CSS viewports keep 12px headings at 100% and search, restore hidden views, and explicitly retire observers and old controls');
  await page.screenshot({ path: path.join(out, 'knowledge-structure.png'), fullPage: true });
  const finalActiveObservers = await page.evaluate(() => { window.headingView.clear(); window.graphView.clear(); return window.canvasResizeRecords.filter(record => record.active).length; });
  assert.equal(finalActiveObservers, 0); result.measurements.push({ phase: 'final-retirement', activeObservers: finalActiveObservers });
  assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) { result.failure = String(error.stack ?? error); await page?.screenshot({ path: path.join(out, 'failure.png'), fullPage: true }).catch(() => {}); throw error; }
finally { await browser?.close(); await vite.close(); await writeFile(path.join(out, 'results.json'), JSON.stringify(result, null, 2)); console.log(JSON.stringify(result)); }
