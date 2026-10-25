import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";
import { EmbeddedQueryController, queryBasisWire } from "./embedded-query";
import { applySourcePatches } from "./editor";
import { UiError, message as uiMessage } from "./ui-locale";

function producer(file: string, name: string): string {
  const source=ts.createSourceFile(file,readFileSync(new URL(file,import.meta.url),"utf8"),ts.ScriptTarget.Latest,true,ts.ScriptKind.TS);
  let found:ts.Node|undefined;
  function visit(node:ts.Node):void{if((ts.isFunctionDeclaration(node)||ts.isMethodDeclaration(node))&&node.name?.getText(source)===name)found=node;else ts.forEachChild(node,visit);}
  visit(source);assert.ok(found);return found.getText(source);
}
const local=ts.transpileModule(producer("./main.ts","compile"),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
const hosted=ts.transpileModule(`class Producer { ${producer("./hosted-editor.ts","compile")} }`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;

test("actual local and hosted compile producers install embedded discovery with client identity and strict wire DTOs",async()=>{
 for(const kind of ["local","hosted"]){
  const source='#metadata((kind: "weftext.query")) <weftext-query-one>',inputId=crypto.randomUUID();
  const basis={sessionId:"writer",source,serial:0,inputId};
  const occurrence={id:"one",key:"one",label:"weftext-query-one",kind:"definition",callFrom:1,
   range:{from:0,to:source.length,utf16From:0,utf16To:source.length,raw:source,writable:true},writable:true,builderSupported:true,runnable:true,
   plan:null,reason:null,controlKind:null,controlReason:null,controlDiagnostics:[],call:null,binding:null};
  const paths:string[]=[];
  const request=async(path:string,wire:any)=>{
   paths.push(path);assert.deepEqual(Object.keys(wire).sort(),["serial","sessionId","source"]);
   if(path.endsWith("discover"))return {source:basis.source,serial:basis.serial,sourceHash:"hash",syntaxValid:true,occurrences:[occurrence]};
   if(path.endsWith("map"))return {sourceHash:"hash"};
   return {...queryBasisWire(basis),ok:true,failure:null,sourceInfo:{notes:null,syntaxTokens:[]},diagnostics:[]};
  };
  const queries=new EmbeddedQueryController({queryResultScope:kind,basis:()=>basis,request} as any);
  const editor={source,generation:0,syntaxBasis:()=>({}),tokens(){}};
  const session={source,serial:0,inputId,opened:{sessionId:basis.sessionId},acceptProjection:()=>false};
  const doc={editor,session,queries,renderGeneration:0,rendering:0};
  const context=vm.createContext({queryBasisWire,clearTimeout,sourceConsumersAvailable:()=>true,current:null,
   libraryCloseActivity:{begin:()=>()=>{}},api:request,decodeEditorMapResponse:(value:unknown)=>value,
   setUiText(){},uiMessage:(v:unknown)=>v,descriptionUiText:(v:unknown)=>v});
  vm.runInContext(kind==="local"?local:hosted,context);
  if(kind==="local")await vm.runInContext("compile",context)(doc);
  else await vm.runInContext("Producer.prototype.compile",context).call({editor,writing:{installMap(){}},resource:"note",sessionId:basis.sessionId,authorInputId:inputId,
   generation:0,queries,request,fonts:{ensureDefaults:async()=>{}},paint(){},error(error:Error){throw error;}});
  assert.equal(queries.occurrences.length,1);assert.ok(paths.some(path=>path.endsWith("discover")));
 }
});

test("actual local insert Query producer keeps input identity out of its strict definition DTO",async()=>{
 const session={opened:{sessionId:"writer",nodeId:"node",workspaceId:"workspace",root:"C:/library"},source:"S",serial:0,inputId:crypto.randomUUID(),baseId:"base"};
 let applied=0;const doc={session,mode:"source",editor:{applyPatches(){applied++;}}};
 const requests:any[]=[];
 const element=()=>({value:"",isConnected:true,hidden:false,disabled:false,append(){},replaceChildren(){},setAttribute(){}});
 const context=vm.createContext({current:doc,UiError,uiMessage,queryBasisWire,applySourcePatches,inventory:undefined,
  sourceConsumersAvailable:()=>true,readOnlySource:()=>false,prepareSwitch:async()=>{},operationId:()=>"operation",
  libraryCloseActivity:{begin:()=>()=>{},run:async(fn:()=>unknown)=>fn()},el:element,button:element,setUiAttr(){},setUiText(){},
  persistDraft:async()=>{},scheduleRender(){},
  modal:async(_title:unknown,_description:unknown,callback:any)=>new Promise(resolve=>{const form:any=element();callback(form,resolve);form.onsubmit({preventDefault(){}});}),
  api:async(path:string,wire:any)=>{requests.push(wire);assert.equal(path,"/api/query/definition/draft");assert.equal("inputId"in wire,false);
   assert.equal(wire.baseId,"base");assert.equal(wire.sessionId,"writer");return {draftOnly:true,source:"S",serial:0,patchUnit:"utf16",patches:[]};}});
 vm.runInContext(ts.transpileModule(producer("./main.ts","insertEverydayQuery"),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText,context);
 await vm.runInContext("insertEverydayQuery",context)();assert.equal(requests.length,1);assert.equal(applied,1);
});

test("actual hosted draft-history comparison producer projects its strict DTO while retaining the frozen client basis",async()=>{
 const basis={sessionId:"writer",source:"S",serial:0,inputId:crypto.randomUUID()},selected={versionId:"version",source:"old"};
 const controller:any={selected};let calls=0;
 const context=vm.createContext({queryBasisWire,hash:async()=>"hash",UiError,uiMessage,structuredClone});
 vm.runInContext(ts.transpileModule(`class Producer { ${producer("./hosted-history.ts","compare")} }`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText,context);
 await vm.runInContext("Producer.prototype.compare",context).call({controller,draftComparison:true,epoch:0,note:{resource:"note"},
  source:{basis:async()=>basis},perform:async(fn:()=>unknown)=>fn(),request:async(path:string,wire:any)=>{calls++;assert.equal(path,"history/compare-draft");
   assert.equal("inputId"in wire,false);assert.equal(wire.sessionId,basis.sessionId);return {comparison:"draftSnapshot",selected,
    current:{source:basis.source,serial:basis.serial,sourceHash:"hash",path:"resource:note"}};}});
 assert.equal(calls,1);assert.equal(controller.comparison.current.source,"S");assert.ok(basis.inputId);
});
