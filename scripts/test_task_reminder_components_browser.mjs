// Real browser component checks over current product modules. No Core process
// or OS submission is simulated as passed by this focused UI test.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.WEFTEXT_PLAYWRIGHT_MODULE || 'playwright');
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const out = path.resolve(process.argv[2]);
await mkdir(out, { recursive: false });
const result = { mode: 'actual reminder settings and overview in a browser; no Core or OS acceptance', sourceHashes: {}, browser: null, cases: [], pageErrors: [], completed: false };
for (const relative of ['scripts/test_task_reminder_components_browser.mjs', 'app/src/reminder-device-control.ts', 'app/src/reminder-overview.ts', 'app/src/reminder-types.ts', 'app/src/native-control-identity.ts', 'app/src/ui-locale.ts', 'app/src/ui-diagnostics.ts', 'app/src/ui-messages.ts', 'app/src/style.css', 'app/src/data-view.css']) result.sourceHashes[relative] = createHash('sha256').update(await readFile(path.join(repo, relative))).digest('hex');
let vite, browser, page;
const translated = key => page.evaluate(async key => { const ui = await import('/src/ui-locale.ts'); return ui.t(ui.message(key)); }, key);
const clickMessage = async key => page.getByRole('button', { name: await translated(key), exact: true }).click();
async function caseCheck(name, run) { await run(); result.cases.push({ name, passed: true }); }
try {
  const { createServer } = await import(pathToFileURL(path.join(repo, 'app/node_modules/vite/dist/node/index.js')).href);
  vite = await createServer({ root: path.join(repo, 'app'), configFile: false, server: { host: '127.0.0.1', port: 0, hmr: false, watch: { ignored: ['**/*'] } }, plugins: [{ name: 'owned-reminder-test-document', configureServer(server) { server.middlewares.use('/reminder-test.html', (_request, response) => { response.setHeader('Content-Type', 'text/html;charset=utf-8'); response.end('<!doctype html><html><head><meta charset="utf-8"></head><body><main id="test"></main></body></html>'); }); } }] });
  await vite.listen();
  browser = await chromium.launch({ headless: true, ...(process.env.WEFTEXT_BROWSER_EXECUTABLE ? { executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE } : {}) }); result.browser = await browser.version();
  page = await browser.newPage(); page.on('pageerror', error => result.pageErrors.push(error.message)); await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/reminder-test.html`);
  await page.evaluate(async()=>{await import('/src/style.css');await import('/src/data-view.css');document.querySelector('#test').className='data-panel';});
  await caseCheck('overview exposes restricted scopes and retries the exact original settings request', async () => {
    await page.evaluate(async () => {
      const { ReminderOverview } = await import('/src/reminder-overview.ts'); const settings = { deviceId: 'device', settingsRevision: 2, libraries: [{ workspaceId: 'library', root: 'D:/TEST', enabled: true }], subscriptions: [] };
      window.calls = []; window.notices = []; window.libraryCalls = []; window.failure = true;
      window.overview = new ReminderOverview({ getDocument: () => null, notify: value => window.notices.push(value), request: async (route, request) => { window.calls.push(route); if (route.endsWith('/settings')) return settings; if (route.endsWith('/schedule')) return { entries: [], issues: [{ owner: { workspaceId: 'library', nodeId: 'task' }, reminderId: 'intent', code: 'unavailable', explanation: { key: 'reminder.unavailable', params: {} }, diagnostics: [{ origin: 'os', text: 'TEST library unavailable' }], causes: [] }], complete: true, next: null }; if (route.endsWith('/records')) return { records: [], next: null }; if (route.endsWith('/library')) { window.libraryCalls.push(structuredClone(request)); if (window.failure) { window.failure = false; settings.settingsRevision = 3; throw new Error('TEST lost response'); } return settings; } throw new Error(route); }, applyPatch: async () => {}, openDocument: async () => {} });
      document.querySelector('#test').append(window.overview.element); await window.overview.refresh();
    });
    await page.getByText('TEST library unavailable', { exact: false }).waitFor();await page.locator('#test').screenshot({path:path.join(out,'overview-restricted.png')});await page.locator('.reminder-overview > section').first().getByRole('button', { name: await translated('reminder.disable'), exact: true }).click(); await page.getByText(await translated('reminder.settingsPending'), { exact: true }).waitFor(); await clickMessage('reminder.retryOriginal');
    const calls = await page.evaluate(() => window.libraryCalls); assert.equal(calls.length, 2); assert.deepEqual(calls[1], calls[0]); assert.equal(calls[0].action, 'disable'); assert.equal(calls[0].workspaceId, 'library'); assert.equal(calls[0].expectedRevision, 2);
  });
  await caseCheck('closing overview does not revive UI or make new reads after a late mutation response', async () => {
    await page.evaluate(() => { window.overview.dispose(); document.querySelector('#test').replaceChildren(); });
    await page.evaluate(async () => {
      const { ReminderOverview } = await import('/src/reminder-overview.ts'); window.calls = []; window.settle = null;
      window.overview = new ReminderOverview({ getDocument: () => null, notify: () => {}, request: async (route) => { window.calls.push(route); if (route.endsWith('/settings')) return { deviceId: 'device', settingsRevision: 1, libraries: [{ workspaceId: 'library', root: 'D:/TEST', enabled: true }], subscriptions: [] }; if (route.endsWith('/schedule')) return { entries: [], issues: [], complete: true, next: null }; if (route.endsWith('/records')) return { records: [], next: null }; if (route.endsWith('/library')) return await new Promise(resolve => { window.settle = resolve; }); throw new Error(route); }, applyPatch: async () => {}, openDocument: async () => {} }); document.querySelector('#test').append(window.overview.element); await window.overview.refresh();
    });
    await clickMessage('reminder.disable'); await page.waitForFunction(() => !!window.settle); const count = await page.evaluate(() => { window.overview.dispose(); document.querySelector('#test').replaceChildren(); const count = window.calls.length; window.settle({}); return count; }); await page.waitForTimeout(100); assert.equal(await page.evaluate(() => window.calls.length), count); assert.equal(await page.locator('#test').textContent(), '');
  });
  await caseCheck('disposed connected overview buttons and old pagination never start new requests', async () => {
    await page.evaluate(async () => {
      window.overview.dispose(); document.querySelector('#test').replaceChildren();
      const { ReminderOverview } = await import('/src/reminder-overview.ts');
      window.calls = []; window.overview = new ReminderOverview({
        getDocument: () => null, notify: () => {}, applyPatch: async () => {}, openDocument: async () => {}, openResolved: async () => {},
        request: async route => { window.calls.push(route); if (route.endsWith('/settings')) return { deviceId:'device', settingsRevision:1, libraries:[{workspaceId:'library',root:'D:/TEST',enabled:true}],subscriptions:[] };
          if (route.endsWith('/schedule')) return {entries:[],issues:[],complete:false,next:{settingsRevision:1,after:'next'}};
          if (route.endsWith('/records')) return {records:[{owner:{workspaceId:'library',nodeId:'task'},reminderId:'intent',triggerInstant:'2026-10-23T08:00:12Z',status:{executionKey:'key',delivery:{state:'failedBeforeCall',description:{explanation:{key:'common.operation',params:{operationId:'original-pre-call'}},diagnostics:[],causes:[]}},submissionObservation:null,handled:false,invalidated:null}}],next:'after'};
          throw new Error(route); }
      }); document.querySelector('#test').append(window.overview.element); await window.overview.refresh();
    });
    assert.equal(await page.getByText(await translated('reminder.noScheduledTasks'),{exact:true}).count(),0);
    assert.equal(await page.getByText(await translated('reminder.failedBeforeCall'),{exact:true}).count(),1);
    assert.equal(await page.getByRole('heading',{name:await translated('reminder.deliveryRecords'),exact:true}).count(),1);
    const before=await page.evaluate(()=>{const count=window.calls.length;window.overview.dispose();for(const button of window.overview.element.querySelectorAll('button'))button.dispatchEvent(new MouseEvent('click',{bubbles:true}));return count;});
    await page.waitForTimeout(50);assert.equal(await page.evaluate(()=>window.calls.length),before);
    await page.evaluate(()=>document.querySelector('#test').replaceChildren());
  });
  await caseCheck('overview keeps replacement pending B when original A settles out of order', async () => {
    await page.evaluate(async () => {
      const { ReminderOverview } = await import('/src/reminder-overview.ts');window.calls=[];window.libraryCalls=[];window.settlers=[];
      window.overview=new ReminderOverview({getDocument:()=>null,notify:()=>{},applyPatch:async()=>{},openDocument:async()=>{},request:async(route,request)=>{
        window.calls.push(route);if(route.endsWith('/settings'))return{deviceId:'device',settingsRevision:1,libraries:[{workspaceId:'library',root:'D:/TEST',enabled:true}],subscriptions:[]};
        if(route.endsWith('/schedule'))return{entries:[],issues:[],complete:true,next:null};if(route.endsWith('/records'))return{records:[],next:null};
        if(route.endsWith('/library')){window.libraryCalls.push(structuredClone(request));return await new Promise((resolve,reject)=>window.settlers.push({resolve,reject}));}throw new Error(route);
      }});document.querySelector('#test').append(window.overview.element);await window.overview.refresh();
    });
    await clickMessage('reminder.disable');await page.waitForFunction(()=>window.settlers.length===1);
    await page.evaluate(async()=>{for(let i=0;i<2;i++)window.overview.element.querySelector('button:disabled')?.dispatchEvent(new MouseEvent('click'));await window.overview.refresh();});
    assert.equal(await page.evaluate(()=>window.settlers.length),1);assert.equal(await page.getByRole('button',{name:await translated('reminder.retryOriginal'),exact:true}).isDisabled(),true);
    await clickMessage('reminder.discardPending');await clickMessage('reminder.disable');await page.waitForFunction(()=>window.settlers.length===2);await page.evaluate(()=>window.overview.refresh());
    const before=await page.evaluate(()=>{const count=window.calls.length;window.settlers[0].resolve({});return count;});await page.waitForTimeout(50);
    assert.equal(await page.evaluate(()=>window.calls.length),before);assert.equal(await page.getByText(await translated('reminder.settingsPending'),{exact:true}).count(),1);
    await page.evaluate(()=>window.settlers[1].reject(new Error('TEST B original response unknown')));await page.getByText('TEST B original response unknown',{exact:true}).waitFor();await clickMessage('reminder.retryOriginal');await page.waitForFunction(()=>window.settlers.length===3);
    const original=await page.evaluate(()=>window.libraryCalls);assert.notEqual(original[0].mutationId,original[1].mutationId);assert.deepEqual(original[2],original[1]);
    await page.evaluate(()=>{window.overview.dispose();window.settlers[2].resolve({});document.querySelector('#test').replaceChildren();});
  });
  await caseCheck('task device buttons require exact current basis and settle only their original pending', async () => {
    await page.evaluate(async () => {
      const { ReminderDeviceControl }=await import('/src/reminder-device-control.ts');
      window.basis={sessionId:'task-session',source:'saved',serial:1,baseId:'base',path:'D:/TEST/Task/Task.typ',root:'D:/TEST',nodeId:'task',workspaceId:'library',dirty:false};
      window.calls=[];window.settlers=[];window.libraryCalls=[];
      const inspection={candidate:{root:'D:/TEST',owner:{workspaceId:'library',nodeId:'task'},taskOpen:true,resolution:{state:'absent',reminderId:null,triggerInstant:null,normalDeliveryUntil:null,definitionDigest:null,providers:{},failureKind:null,diagnostics:[]}},settings:{deviceId:'device',settingsRevision:1,libraries:[{workspaceId:'library',root:'D:/TEST',enabled:true}],subscriptions:[]},delivery:null,eligibility:null,issue:null};
      window.deviceControl=new ReminderDeviceControl({getDocument:()=>window.basis,notify:()=>{},applyPatch:async()=>{},openDocument:async()=>{},request:async(route,request)=>{window.calls.push(route);if(route.endsWith('/inspect'))return inspection;if(route.endsWith('/library')){window.libraryCalls.push(structuredClone(request));return await new Promise((resolve,reject)=>window.settlers.push({resolve,reject}));}throw new Error(route);}});
      document.querySelector('#test').append(window.deviceControl.element);await window.deviceControl.refresh(window.basis);
      window.oldDeviceButton=window.deviceControl.element.querySelector('button[data-control-key]');window.basis={...window.basis,serial:2};window.oldDeviceButton.dispatchEvent(new MouseEvent('click'));
    });
    assert.equal(await page.evaluate(()=>window.libraryCalls.length),0);
    await clickMessage('reminder.refresh');await clickMessage('reminder.disable');await page.waitForFunction(()=>window.settlers.length===1);await page.evaluate(()=>window.deviceControl.refresh(window.basis));
    await clickMessage('reminder.discardPending');await clickMessage('reminder.disable');await page.waitForFunction(()=>window.settlers.length===2);await page.evaluate(()=>window.deviceControl.refresh(window.basis));
    const before=await page.evaluate(()=>{const count=window.calls.length;window.settlers[0].reject(new Error('TEST retired A'));return count;});await page.waitForTimeout(50);assert.equal(await page.evaluate(()=>window.calls.length),before);assert.equal(await page.getByText('TEST retired A',{exact:true}).count(),0);
    assert.equal(await page.getByText(await translated('reminder.settingsPending'),{exact:true}).count(),1);
    await page.evaluate(()=>{window.deviceControl.retire();window.settlers[1].resolve({});document.querySelector('#test').replaceChildren();});
  });
  await caseCheck('UiError remains a translated descriptor in the live device error slot', async () => {
    await page.evaluate(async()=>{
      const{ReminderDeviceControl}=await import('/src/reminder-device-control.ts');const ui=await import('/src/ui-locale.ts');ui.locale.set('zh-CN');
      const basis={sessionId:'error',source:'test',serial:1,baseId:'base',path:'D:/TEST/Task/Task.typ',root:'D:/TEST'};
      window.deviceControl=new ReminderDeviceControl({getDocument:()=>basis,notify:()=>{},applyPatch:async()=>{},openDocument:async()=>{},request:async()=>{throw new ui.UiError(ui.message('reminder.saveFirst'));}});
      document.querySelector('#test').append(window.deviceControl.element);await window.deviceControl.refresh(basis);window.errorNode=[...window.deviceControl.element.querySelectorAll('p')].find(node=>node.textContent===ui.t(ui.message('reminder.saveFirst')));
    });
    await page.getByText(await translated('reminder.saveFirst'),{exact:true}).waitFor();await page.evaluate(async()=>{(await import('/src/ui-locale.ts')).locale.set('en-US');});
    assert.equal(await page.evaluate(()=>window.errorNode.isConnected),true);assert.equal(await page.evaluate(()=>window.errorNode.textContent),await translated('reminder.saveFirst'));
    await page.evaluate(()=>{window.deviceControl.retire();document.querySelector('#test').replaceChildren();});
  });
  await caseCheck('closing overview while prepareAction is pending cannot open a Core session', async () => {
    await page.evaluate(async()=>{
      const{ReminderOverview}=await import('/src/reminder-overview.ts');window.calls=[];window.prepareDone=null;window.adopted=[];
      window.overview=new ReminderOverview({getDocument:()=>null,notify:()=>{},applyPatch:async()=>{},openDocument:async()=>{},openResolved:async document=>window.adopted.push(document),prepareAction:()=>new Promise(resolve=>window.prepareDone=resolve),request:async route=>{
        window.calls.push(route);if(route.endsWith('/settings'))return{deviceId:'device',settingsRevision:1,libraries:[],subscriptions:[]};if(route.endsWith('/schedule'))return{entries:[],issues:[],complete:true,next:null};if(route.endsWith('/records'))return{records:[{owner:{workspaceId:'library',nodeId:'task'},reminderId:'intent',triggerInstant:'2026-10-23T08:00:12Z',status:{executionKey:'key',delivery:{state:'submittedToSystem',at:'2026-10-23T08:00:13Z'},submissionObservation:null,handled:true,invalidated:null}}],next:null};throw new Error(route);
      }});document.querySelector('#test').append(window.overview.element);await window.overview.refresh();
    });
    await clickMessage('reminder.openTask');await page.waitForFunction(()=>!!window.prepareDone);await page.evaluate(()=>{window.overview.dispose();window.prepareDone();});await page.waitForTimeout(50);
    assert.equal(await page.evaluate(()=>window.calls.some(route=>route.endsWith('/open'))),false);assert.deepEqual(await page.evaluate(()=>window.adopted),[]);
    await page.evaluate(()=>document.querySelector('#test').replaceChildren());
  });
  await caseCheck('empty last pages never claim the complete reminder queue or ledger is empty', async () => {
    await page.evaluate(async()=>{
      const{ReminderOverview}=await import('/src/reminder-overview.ts');window.calls=[];
      const owner={workspaceId:'library',nodeId:'task'};
      window.overview=new ReminderOverview({getDocument:()=>null,notify:()=>{},applyPatch:async()=>{},openDocument:async()=>{},request:async(route,request)=>{
        window.calls.push(route);if(route.endsWith('/settings'))return{deviceId:'device',settingsRevision:1,libraries:[],subscriptions:[]};
        if(route.endsWith('/schedule'))return request.cursor?{entries:[],issues:[],complete:true,next:null}:{entries:[{candidate:{root:'D:/TEST',owner,taskOpen:true,resolution:{state:'ready',reminderId:'intent',triggerInstant:'2026-10-23T08:00:12Z',normalDeliveryUntil:'2026-10-23T08:01:00Z',definitionDigest:'definition',providers:{},failureKind:null,diagnostics:[]}},eligibility:{executionKey:'key',timing:'upcoming',retained:null},issue:null}],issues:[],complete:false,next:{settingsRevision:1,after:'next'}};
        if(route.endsWith('/records'))return request.after?{records:[],next:null}:{records:[{owner,reminderId:'intent',triggerInstant:'2026-10-23T08:00:12Z',status:{executionKey:'key',delivery:{state:'submittedToSystem',at:'2026-10-23T08:00:13Z'},submissionObservation:null,handled:true,invalidated:null}}],next:'after'};throw new Error(route);
      }});document.querySelector('#test').append(window.overview.element);await window.overview.refresh();
      window.oldQueueMore=[...window.overview.element.children[1].querySelectorAll('button')].at(-1);
    });
    await page.locator('.reminder-overview > section').nth(1).getByRole('button',{name:await translated('reminder.more'),exact:true}).click();
    await page.locator('.reminder-overview > section').nth(2).getByRole('button',{name:await translated('reminder.more'),exact:true}).click();
    assert.equal(await page.getByText(await translated('reminder.noScheduledTasks'),{exact:true}).count(),0);assert.equal(await page.getByText(await translated('reminder.noDeliveryRecords'),{exact:true}).count(),0);
    const count=await page.evaluate(()=>{const count=window.calls.length;window.oldQueueMore.dispatchEvent(new MouseEvent('click'));return count;});await page.waitForTimeout(50);assert.equal(await page.evaluate(()=>window.calls.length),count);
    await page.evaluate(()=>{window.overview.dispose();document.querySelector('#test').replaceChildren();});
  });
  await caseCheck('safe device refresh and global overview remain available after author source changes', async () => {
    await page.evaluate(async()=>{
      const{ReminderDeviceControl}=await import('/src/reminder-device-control.ts');window.calls=[];window.basis={sessionId:'edited',source:'saved',serial:1,baseId:'base',path:'D:/TEST/Task/Task.typ',root:'D:/TEST'};
      window.deviceControl=new ReminderDeviceControl({getDocument:()=>window.basis,notify:()=>{},applyPatch:async()=>{},openDocument:async()=>{},request:async(route,request)=>{
        window.calls.push({route,request});if(route.endsWith('/inspect'))throw new Error('TEST saved task unavailable');if(route.endsWith('/settings'))return{deviceId:'device',settingsRevision:1,libraries:[],subscriptions:[]};if(route.endsWith('/schedule'))return{entries:[],issues:[],complete:true,next:null};if(route.endsWith('/records'))return{records:[],next:null};throw new Error(route);
      }});document.querySelector('#test').append(window.deviceControl.element);await window.deviceControl.refresh(window.basis);window.basis={...window.basis,source:'new author text',serial:2,dirty:true};
    });
    await clickMessage('reminder.refresh');await page.waitForFunction(()=>window.calls.filter(call=>call.route.endsWith('/inspect')).length===2);
    await page.evaluate(()=>{window.basis={...window.basis,serial:3};});await clickMessage('reminder.overview');await page.getByRole('dialog',{name:await translated('reminder.overview'),exact:true}).waitFor();
    await page.getByText(await translated('reminder.noDeliveryRecords'),{exact:true}).waitFor();await clickMessage('common.close');await page.waitForFunction(()=>!document.querySelector('dialog.reminder-overview-dialog'));
    await page.evaluate(()=>{window.deviceControl.retire();document.querySelector('#test').replaceChildren();});
  });
  await caseCheck('an unavailable saved task retains the global records and settings entry',async()=>{
    await page.evaluate(async()=>{const{ReminderDeviceControl}=await import('/src/reminder-device-control.ts');const basis={sessionId:'unavailable',source:'owned test',serial:1,baseId:'base',path:'D:/TEST/Task/Task.typ',root:'D:/TEST'};window.deviceControl=new ReminderDeviceControl({getDocument:()=>basis,notify:()=>{},request:async route=>{if(route.endsWith('/inspect'))throw new Error('TEST saved task unavailable');if(route.endsWith('/settings'))return{deviceId:'device',settingsRevision:1,libraries:[],subscriptions:[]};if(route.endsWith('/schedule'))return{entries:[],issues:[],complete:true,next:null};if(route.endsWith('/records'))return{records:[],next:null};throw new Error(route);},applyPatch:async()=>{},openDocument:async()=>{}});document.querySelector('#test').append(window.deviceControl.element);await window.deviceControl.refresh(basis);});
    await page.getByText('TEST saved task unavailable',{exact:true}).waitFor();await clickMessage('reminder.overview');await page.getByRole('dialog',{name:await translated('reminder.overview'),exact:true}).waitFor();await page.getByText(await translated('reminder.noDeliveryRecords'),{exact:true}).waitFor();await clickMessage('common.close');await page.waitForFunction(()=>!document.querySelector('dialog.reminder-overview-dialog'));await page.evaluate(()=>{window.deviceControl.retire();document.querySelector('#test').replaceChildren();});
  });
  await caseCheck('shared overview entry opens a localized modal and closes its owned component',async()=>{
    await page.evaluate(async()=>{const{openReminderOverview}=await import('/src/reminder-overview.ts');window.dialog=openReminderOverview({getDocument:()=>null,notify:()=>{},request:async route=>{if(route.endsWith('/settings'))return{deviceId:'device',settingsRevision:1,libraries:[],subscriptions:[]};if(route.endsWith('/schedule'))return{entries:[],issues:[],complete:true,next:null};if(route.endsWith('/records'))return{records:[],next:null};throw new Error(route);},applyPatch:async()=>{},openDocument:async()=>{}});});
    await page.getByRole('dialog',{name:await translated('reminder.overview'),exact:true}).waitFor();await page.getByText(await translated('reminder.noDeliveryRecords'),{exact:true}).waitFor();await page.screenshot({path:path.join(out,'overview-dialog.png'),fullPage:true});await clickMessage('common.close');await page.waitForFunction(()=>!window.dialog.isConnected);
  });
  assert.deepEqual(result.pageErrors, []); result.completed = true;
} catch (error) { result.error = error.stack || String(error); throw error; }
finally { if (browser) await browser.close(); if (vite) await vite.close(); await writeFile(path.join(out, 'result.json'), JSON.stringify(result, null, 2) + '\n'); }
