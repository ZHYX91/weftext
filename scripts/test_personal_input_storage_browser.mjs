/** Real IndexedDB transaction and process-restart check; no product API or
 * user browser profile. Explicitly injected immutable-row conflict tests
 * transaction abort, not actual quota exhaustion or power loss. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const out=process.env.WEFTEXT_BROWSER_REPORT;
if(!out || !path.isAbsolute(out)) throw new Error('WEFTEXT_BROWSER_REPORT must name a new absolute output directory');
await mkdir(out); // Refuse unknown existing output/profile.
const {chromium}=createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE||'playwright');
const {createServer}=await import(pathToFileURL(path.join(repo,'app/node_modules/vite/dist/node/index.js')).href);
const result={scope:'Actual IndexedDB CAS, controlled immutable-row abort and graceful browser-process restart; no Core/HTTP/native-close/power-loss claim',checks:[],source:{},ownership:{directory:out,browserProfile:path.join(out,'profile'),browserClosed:false,viteClosed:false},errors:[]};
for(const name of ['app/src/personal-input-storage.ts','app/src/personal-input-recovery.ts','app/src/recovery-text-cache.ts']) result.source[name]=createHash('sha256').update(await readFile(path.join(repo,name))).digest('hex');
let vite,context;
try {
  vite=await createServer({root:path.join(repo,'app'),configFile:false,server:{host:'127.0.0.1',port:0,hmr:false}}); await vite.listen();
  const base='http://127.0.0.1:'+vite.httpServer.address().port;
  const launch=async()=>{
    context=await chromium.launchPersistentContext(result.ownership.browserProfile,{channel:'chrome',headless:true});
    const page=await context.newPage(); await page.route('**/storage-probe',route=>route.fulfill({contentType:'text/html',body:'<!doctype html><title>Owned pure proposal persistence fixture</title>'})); await page.goto(base+'/storage-probe'); return page;
  };
  let page=await launch(); result.browser=await page.evaluate(()=>navigator.userAgent);
  result.checks.push(...await page.evaluate(async()=>{
    const {IndexedDBPersonalInputBackend}=await import('/src/personal-input-storage.ts');
    const {PersonalInputRecovery,personalInputText}=await import('/src/personal-input-recovery.ts');
    const audience=JSON.stringify(['fixture-deployment','alice']), backend=new IndexedDBPersonalInputBackend(indexedDB,'fixture-v3'), first=new PersonalInputRecovery(backend), second=new PersonalInputRecovery(new IndexedDBPersonalInputBackend(indexedDB,'fixture-v3'));
    const checks=[], assert=(value,why)=>{if(!value)throw new Error(why);};
    const record=(id,text)=>({id,owner:{kind:'registry',target:'actual-fixture'},slot:id,origin:{baseLength:0,pieces:[{text}],detached:[],revision:1}});
    await Promise.all([first.open(audience),second.open(audience)]);
    await first.update(audience,[record('first','own\ud800\u0000😀')]);
    await second.update(audience,[record('second','independent')]);
    const loaded=new PersonalInputRecovery(new IndexedDBPersonalInputBackend(indexedDB,'fixture-v3')); await loaded.open(audience);
    assert(loaded.list(audience).length===2,'different producer CAS merge'); assert(personalInputText(loaded.list(audience).find(r=>r.id==='first'))==='own\ud800\u0000😀','exact UTF16'); checks.push('real transaction CAS merges independent producers without losing lone UTF16/NUL');
    const original=await backend.read(audience), entries=new Map(original.texts), collision=[...entries.keys()][0]; entries.set(collision,'CORRUPT'); entries.set('e'.repeat(32),'new-row-before-abort');
    let refused=false;try{await backend.compareAndWrite(audience,original.commitId,'UNCOMMITTED',entries);}catch{refused=true;}
    const after=await backend.read(audience);assert(refused&&after.commitId===original.commitId&&after.metadata===original.metadata&&JSON.stringify([...after.texts].sort())===JSON.stringify([...original.texts].sort()),'failed transaction left metadata/chunks changed');checks.push('controlled immutable-row conflict aborts metadata/new chunks atomically and preserves original bytes');
    let stale=false;try{await backend.compareAndWrite(audience,0,'STALE',new Map());}catch{stale=true;}assert(stale&&(await backend.read(audience)).commitId===original.commitId,'stale CAS changed original');checks.push('stale direct CAS refuses without replacing durable row');
    await loaded.update(audience,[record('first','new own\ud800\u0000😀')]);
    const foreign=new PersonalInputRecovery(new IndexedDBPersonalInputBackend(indexedDB,'fixture-v3')); await foreign.open(JSON.stringify(['fixture-deployment','bob']));assert(foreign.list(JSON.stringify(['fixture-deployment','bob'])).length===0,'actor separation');checks.push('another actor namespace has no proposal records');
    const draftId='00000000-0000-4000-8000-000000000001',producerId='00000000-0000-4000-8000-000000000002',id=`capture:${draftId}:${producerId}`;
    const current={kind:'captureTransport',id,owner:{kind:'capture',draftId},producerId,producerRevision:1,baseRevision:3,input:{texts:{name:[],title:[],body:[{text:'A\ud800',provenance:{producerId,inputId:'a'.repeat(32),revision:1}}],rootInput:[],parentInput:[],parametersText:[]},bodyMode:'text',rawInputs:[],detachedRawInputs:[]}};
    await loaded.stageCaptureCurrent(audience,current);
    await loaded.freezeCaptureMutation(audience,id,{producerRevision:1,command:{kind:"fullInput",expectedRevision:3,mutationId:'00000000-0000-4000-8000-000000000003',fingerprint:'a'.repeat(64)},input:current.input});
    await loaded.stageCaptureCurrent(audience,{...current,producerRevision:2,input:{...current.input,texts:{...current.input.texts,body:[{text:'B\udc00\0',provenance:{producerId,inputId:'a'.repeat(32),revision:2}}]}}});
    checks.push('real IndexedDB keeps new current B separately from frozen pending A in the same pure transport record; synthetic command metadata, no Core ACK');
    return checks;
  }));
  await context.close(); context=undefined; result.ownership.firstBrowserClosed=true;
  page=await launch();
  const recovered=await page.evaluate(async()=>{
    const {IndexedDBPersonalInputBackend}=await import('/src/personal-input-storage.ts');const {PersonalInputRecovery,personalInputText}=await import('/src/personal-input-recovery.ts');
    const store=new PersonalInputRecovery(new IndexedDBPersonalInputBackend(indexedDB,'fixture-v3')),audience=JSON.stringify(['fixture-deployment','alice']); await store.open(audience);
    return {texts:store.list(audience).map(record=>({id:record.id,units:Array.from({length:personalInputText(record).length},(_,i)=>personalInputText(record).charCodeAt(i))})),capture:store.captureTransports(audience)};
  });
  assert.equal(recovered.texts.length,2); assert.deepEqual(recovered.texts.find(row=>row.id==='first').units,Array.from({length:'new own\ud800\u0000😀'.length},(_,i)=>'new own\ud800\u0000😀'.charCodeAt(i)));
  assert.equal(recovered.capture.length,1);assert.equal(recovered.capture[0].input.texts.body[0].text,'B\udc00\0');assert.equal(recovered.capture[0].pending.input.texts.body[0].text,'A\ud800');assert.equal(recovered.capture[0].pending.command.mutationId,'00000000-0000-4000-8000-000000000003');
  result.recovered=recovered;result.checks.push('separate browser process with owned profile recovers exact updated UTF16; no sessionStorage dependency');result.ok=true;
}catch(error){result.ok=false;result.errors.push(String(error?.stack||error));process.exitCode=1;}
finally{
  if(context){await context.close();result.ownership.browserClosed=true;}else result.ownership.browserClosed=true;
  if(vite){await vite.close();result.ownership.viteClosed=true;}
  await writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2));
  console.log(JSON.stringify({ok:result.ok,checks:result.checks.length,report:path.join(out,'result.json'),ownership:result.ownership}));
}
