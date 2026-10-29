// Real HostedKnowledgeLinks DOM with a bounded request host; no Core or author writes.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

assert.equal(process.argv.length, 3, 'Usage: node scripts/test_hosted_relation_preferences_browser.mjs ABSOLUTE_NEW_OUTPUT');
assert.ok(path.isAbsolute(process.argv[2]));
const repo = path.resolve(import.meta.dirname, '..'), out = path.resolve(process.argv[2]);
assert.ok(path.relative(repo, out).startsWith('..' + path.sep), 'Evidence belongs outside the product repository');
await mkdir(out, { recursive: false });
const inputs = ['app/src/hosted-knowledge-links.ts', 'app/src/hosted-knowledge-links-state.ts', 'app/src/hosted-relation-preferences.ts', 'app/src/relationship-diagnostics.ts', 'app/src/relationship-diagnostics-state.ts', 'app/src/ui-messages.ts', 'scripts/test_hosted_relation_preferences_browser.mjs'];
const inventory = async () => Promise.all(inputs.map(async name => { const bytes = await readFile(path.join(repo, name)); return { path: name, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') }; }));
const result = { startedUtc: new Date().toISOString(), developmentOnly: true,
  scope: 'Current HostedKnowledgeLinks and personal-rule dialogs in headless Chrome with browser localStorage and a bounded request host.',
  limits: ['No actual Core/server authorization or durable author save, native desktop, real OS IME or assistive-technology acceptance.'], cases: [], pageErrors: [], completed: false, sourceBefore: await inventory() };
const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
const require = createRequire(import.meta.url), { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
let server, browser;
const fixtureHtml = `<!doctype html><meta charset="utf-8"><style>body{font:14px sans-serif;margin:20px}#links{max-width:800px}</style><main id="links"></main><script type="module">
import {HostedKnowledgeLinks} from '/src/hosted-knowledge-links.ts';import {locale,uiTextValue,message} from '/src/ui-locale.ts';
const workspace='10000000-0000-4000-8000-000000000001',sourceId='10000000-0000-4000-8000-000000000002',targetId='10000000-0000-4000-8000-000000000003';
const state={workspace,actor:'alice',deployment:'server',generation:1,revoked:false,reads:0,authorWrites:0};window.fixture=Object.assign(state,{locale,uiTextValue,message});
const context=()=>({inputId:'input',actor:state.actor,deployment:state.deployment,generation:state.generation,credential:'TEST-never-persist-token',epoch:state.generation,authorization:state.generation,resource:'SameAlias',sessionId:'session',source:'chosen term',serial:1,writable:true,pending:false,compare:false});
const identity=()=>{const c=context();return {actor:c.actor,deployment:c.deployment,generation:c.generation,credential:c.credential,authorization:c.authorization};};
const node=(nodeId,resource)=>({resource,title:nodeId===sourceId?'Current note':'Authorized target',nodeRef:{workspaceId:state.workspace,nodeId},reference:'weftext-node://'+state.workspace+'/'+nodeId,canOpen:true});
const profile=body=>{const source=node(sourceId,'SameAlias'),target=node(targetId,'Other'),root=state.workspace+'/'+sourceId;return {
 basis:{actor:state.actor,actorEpoch:state.generation,deployment:state.deployment,resource:'SameAlias',sessionId:body.sessionId,serial:body.serial,sourceHash:body.sourceHash,snapshot:'a'.repeat(64)},scope:'configuredAuthorizedManagedSources',writable:true,fileConversion:false,
 graph:{root,generation:'graph',scope:'authorizedNativeSourcesAndTypedReferences',coverage:'complete',truncated:false,nodes:[],edges:[]},
 diagnostics:{coverage:'complete',scannedNodes:state.revoked?1:2,nodes:(state.revoked?[source]:[source,target]).map(node=>({node,incoming:0,outgoing:0,invalidOutgoing:0,state:'isolated'})),issues:[]},
 links:{coverage:'complete',outgoing:[],incomingId:[],incomingPath:[]},renderedLinks:[],propertyReferences:{scope:'explicitAuthorizedFields',coverage:'complete',items:[]},
 mentions:{scope:'nativeTitlesAndAuthorizedNameFields',coverage:'complete',fragments:state.revoked?[]:[{id:'fragment',text:'chosen term',coverage:'complete',hits:[{startScalar:0,endScalar:11,startUtf16:0,endUtf16:11,terms:['chosen term'],writable:true,component:0,targets:[target],candidates:[{target,term:'chosen term',provenance:'title'}]}]}]}};};
const links=new HostedKnowledgeLinks(document.querySelector('#links'),async(path,body)=>{if(path==='me')return {actor:state.actor,deployment:state.deployment,generation:state.generation};if(path==='knowledge-links/read'){state.reads++;return profile(body);}throw Error('Unexpected bounded request '+path);},{context,identity,flush:async()=>{},apply:async()=>{state.authorWrites++;throw Error('No author write is expected');},navigate:async()=>{},select:()=>{},authorizationChanged:()=>links.clear()});
state.configure=values=>{Object.assign(state,values);links.connect();};state.links=links;links.connect();window.fixtureReady=true;
</script>`;
async function check(name, run) { const entry = { name, passed: false }; result.cases.push(entry); await run(); entry.passed = true; }
try {
  server = await createServer({ root: path.join(repo, 'app'), configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false }, plugins: [{ name: 'owned-hosted-rule-fixture', configureServer(vite) { vite.middlewares.use('/owned-hosted-rule-fixture', (_request, response) => { response.setHeader('Content-Type', 'text/html; charset=utf-8'); response.end(fixtureHtml); }); } }] });
  await server.listen();
  browser = await chromium.launch({ headless: true, ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : { channel: 'chrome' }) });
  result.browser = browser.version();
  const page = await browser.newPage({ viewport: { width: 1100, height: 900 } }); page.setDefaultTimeout(15000); page.on('pageerror', error => result.pageErrors.push(String(error)));
  await page.goto('http://127.0.0.1:' + server.httpServer.address().port + '/owned-hosted-rule-fixture');
  await page.waitForFunction(() => window.fixtureReady);
  const text = key => page.evaluate(key => window.fixture.uiTextValue(window.fixture.message(key)), key);
  const button = async (key, root = page) => root.getByRole('button', { name: await text(key), exact: true });
  const read = async () => { await (await button('hostedLinks.readLinksMentions')).click(); await page.waitForFunction(label => [...document.querySelectorAll('button')].some(button => button.textContent === label && !button.disabled), await text('hostedLinks.readLinksMentions')); await page.getByText(await text('hostedLinks.personalRulesBrowserScope'), { exact: true }).waitFor(); await page.getByLabel(await text('hostedLinks.showPotentialMentions'), { exact: true }).check(); };
  const chooser = async () => button('knowledge.ignoreTerm');
  const preview = async () => page.getByRole('dialog', { name: await text('relationshipDiagnostics.personalRulePreview'), exact: true });
  const saveWord = async () => { await (await chooser()).click(); await (await button('relationshipDiagnostics.savePersonalRule', await preview())).click(); };
  const openRules = async () => { const label = await page.evaluate(() => window.fixture.uiTextValue(window.fixture.message('relationshipDiagnostics.personalRuleCount', { p0: 1 }))); const details = page.locator('details').filter({ has: page.getByText(label, { exact: true }) }).last(); await details.locator('summary').click(); return details; };
  const keys = () => page.evaluate(() => Object.keys(localStorage).filter(key => key.startsWith('weftext-hosted-relations-v1:')));

  await check('saving a note-scoped choice survives page reload and a confirmed Undo persists the empty preference list', async () => {
    await read(); assert.equal(await (await chooser()).count(), 1); await saveWord(); assert.equal(await (await chooser()).count(), 0); assert.equal((await keys()).length, 1);
    const raw = await page.evaluate(() => localStorage.getItem(Object.keys(localStorage).find(key => key.startsWith('weftext-hosted-relations-v1:')))); assert.ok(!raw.includes('TEST-never-persist-token')); assert.ok(!raw.includes('Authorized target')); assert.ok(!raw.includes('sourceHash'));
    await page.reload(); await page.waitForFunction(() => window.fixtureReady); await read(); assert.equal(await (await chooser()).count(), 0);
    const rules = await openRules(); await (await button('relationshipDiagnostics.removeRule', rules)).click(); assert.equal(await (await chooser()).count(), 1); await saveWord(); const active = await openRules(); await (await button('relationshipDiagnostics.undoPreferenceChange', active)).click(); assert.equal(await (await chooser()).count(), 1);
    await page.reload(); await page.waitForFunction(() => window.fixtureReady); await read(); assert.equal(await (await chooser()).count(), 1);
  });
  await check('quota refusal retains the same unapplied preview and candidate, then retry and Undo work after storage recovers', async () => {
    await page.evaluate(() => { window.originalRuleSet = Storage.prototype.setItem; Storage.prototype.setItem = function(key, value) { if(key.startsWith('weftext-hosted-relations-v1:'))throw new DOMException('owned refusal', 'QuotaExceededError'); return window.originalRuleSet.call(this,key,value); }; });
    await (await chooser()).click(); const dialog = await preview(); await (await button('relationshipDiagnostics.savePersonalRule', dialog)).click(); await dialog.getByText(await text('relationshipDiagnostics.ruleSaveFailedPreviewRetained'), { exact: true }).waitFor(); assert.equal(await (await chooser()).count(), 1);
    await page.evaluate(() => { Storage.prototype.setItem = window.originalRuleSet; }); await (await button('relationshipDiagnostics.savePersonalRule', dialog)).click(); assert.equal(await (await chooser()).count(), 0); const rules = await openRules(); await (await button('relationshipDiagnostics.undoPreferenceChange', rules)).click(); assert.equal(await (await chooser()).count(), 1);
  });
  await check('a successful reread in the same audience and library retains the diagnostic search, grouping and sorting', async () => {
    const search = page.getByLabel(await text('relationshipDiagnostics.searchDiagnostics'), { exact: true }); await search.fill('retained diagnostic search');
    const grouping = page.getByLabel(await text('relationshipDiagnostics.issueGrouping'), { exact: true }); await grouping.selectOption('sourceFolder');
    const sort = page.getByLabel(await text('relationshipDiagnostics.resultSort'), { exact: true }); await sort.selectOption('problems');
    await read(); assert.equal(await search.inputValue(), 'retained diagnostic search'); assert.equal(await grouping.inputValue(), 'sourceFolder'); assert.equal(await sort.inputValue(), 'problems');
    await (await button('relationshipDiagnostics.clearSearch')).click();
  });
  await check('same resource alias under another actor, deployment or real library never imports the previous choice', async () => {
    await saveWord();
    for (const change of [{ actor: 'bob' }, { actor: 'alice', deployment: 'other-server' }, { deployment: 'server', workspace: '20000000-0000-4000-8000-000000000001' }]) {
      await page.evaluate(change => window.fixture.configure(change), change); assert.equal(await (await preview()).count(), 0); await read(); assert.equal(await (await chooser()).count(), 1);
    }
    await page.evaluate(() => window.fixture.configure({ workspace: '10000000-0000-4000-8000-000000000001' })); await read(); assert.equal(await (await chooser()).count(), 0);
  });
  await check('known authorization shrink closes a pending rule preview and hides old target and term rules while retaining the private preferences', async () => {
    await page.getByRole('tab', { name: await text('relationshipDiagnostics.tabIsolatedNodes'), exact: true }).click();
    const target = page.locator('.relationship-diagnostics article').filter({ has: page.getByRole('button', { name: 'Authorized target', exact: true }) });
    await (await button('relationshipDiagnostics.markExpectedIsolation', target)).click(); await (await button('relationshipDiagnostics.savePersonalRule', await preview())).click();
    const source = page.locator('.relationship-diagnostics article').filter({ has: page.getByRole('button', { name: 'Current note', exact: true }) });
    await (await button('relationshipDiagnostics.markExpectedIsolation', source)).click(); assert.equal(await (await preview()).count(), 1);
    await page.evaluate(() => window.fixture.configure({ revoked: true, generation: 2 })); assert.equal(await (await preview()).count(), 0); await read(); assert.equal(await page.getByText('Authorized target', { exact: true }).count(), 0); assert.equal(await page.getByText('chosen term', { exact: true }).count(), 0);
    assert.ok(!(await page.locator('#links').textContent()).includes('10000000-0000-4000-8000-000000000003'));
    const raw = await page.evaluate(key => localStorage.getItem(key), (await keys())[0]); assert.equal(JSON.parse(raw).rules.length, 2);
    await page.evaluate(() => window.fixture.configure({ revoked: false })); await read(); assert.equal(await (await chooser()).count(), 0);
  });
  await check('a corrupt browser record is retained, explained and protected from empty overwrite', async () => {
    const key = (await keys())[0]; await page.evaluate(key => { localStorage.setItem(key, '{"version":1,"version":2,"rules":[]}'); window.fixture.links.connect(); }, key);
    await (await button('hostedLinks.readLinksMentions')).click(); await page.getByText(await text('hostedLinks.personalRulesReadFailed'), { exact: true }).waitFor(); await page.getByLabel(await text('hostedLinks.showPotentialMentions'), { exact: true }).check();
    await (await chooser()).click(); const dialog = await preview(); await (await button('relationshipDiagnostics.savePersonalRule', dialog)).click(); await dialog.getByText(await text('relationshipDiagnostics.ruleSaveFailedPreviewRetained'), { exact: true }).waitFor(); assert.equal(await page.evaluate(key => localStorage.getItem(key), key), '{"version":1,"version":2,"rules":[]}'); await dialog.press('Escape');
  });
  result.authorWrites = await page.evaluate(() => window.fixture.authorWrites); assert.equal(result.authorWrites, 0); assert.deepEqual(result.pageErrors, []); result.sourceAfter = await inventory(); assert.deepEqual(result.sourceAfter, result.sourceBefore); result.completed = true;
} finally {
  result.endedUtc = new Date().toISOString(); await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2)); await browser?.close(); await server?.close();
}
console.log(JSON.stringify({ completed: result.completed, cases: result.cases.length, evidence: path.join(out, 'result.json') }));
