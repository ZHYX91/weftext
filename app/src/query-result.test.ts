import test from "node:test";
import assert from "node:assert/strict";
import { decodeHostedQueryResponse, decodeHostedQueryResult, decodeLocalQueryResult, queryDiagnostics } from "./query-result";
import { localQueryResult, queryDataResult } from "./query-result.test-support";
import { locale, message, uiTextValue } from "./ui-locale";

const description=()=>({explanation:message("common.unavailable"),diagnostics:[],causes:[]});
test("local Query retains every parallel diagnostic and its entity provenance across clone and translation",()=>{
    const parallels=Array.from({length:40},description);
    const raw=localQueryResult({diagnostics:[{...description(),details:{kind:"aggregate",groups:[{kind:"product",diagnostics:parallels}]}}],engineDiagnostics:[{entityKey:"entity-A",diagnostics:[{severity:"warning",origin:null,diagnostics:[{origin:"typst",text:"原文\ud800"}]}]}],productErrors:[{entityKey:"entity-B",error:{code:"failed",...description(),details:{kind:"schemaSource",source:"作者原文",roleRef:null,revision:null,cause:{code:"cause",...description()}}}}]});
    const parsed=decodeLocalQueryResult(structuredClone(raw)),cloned=structuredClone(parsed);
    assert.equal(parsed.diagnostics[0].details?.groups[0].kind,"product");
    assert.equal(queryDiagnostics(cloned).length,3);
    locale.set("en-US");const text=queryDiagnostics(cloned).map(uiTextValue).join("\n");
    assert.equal(text.match(/Unavailable/g)?.length,43);assert.match(text,/entity-A/);assert.match(text,/entity-B/);assert.match(text,/原文\ud800/);locale.set("zh-CN");
    assert.deepEqual((cloned.productErrors![0].error.details as {source:string}).source,"作者原文");
});

test("hosted Query admits its own closed scope, exact null facts and typed cells",()=>{
    const row={key:"row",entityKey:"entity",path:null,revision:null,readOnly:true,resource:"granted",canOpen:true,cells:{status:{state:"invalid" as const,value:null,diagnostics:[description()]}}};
    const result=queryDataResult({rows:[row],totalKnown:1}),response={queryId:"original",columns:["status"],scope:"authorizedProjection",readOnly:true,result};
    assert.equal(decodeHostedQueryResponse(response).rows[0].path,null);
    const {path,...missing}=row;
    for(const invalid of [{...response,readOnly:false},{...response,result:{...result,rows:[missing]}},{...response,result:{...result,diagnostics:[{...description(),details:{kind:"aggregate",groups:[]}}]}},{...response,result:{...result,queryId:"nested"}},{...response,columns:[]}])assert.throws(()=>decodeHostedQueryResponse(invalid));
    assert.equal(decodeHostedQueryResult({...result,columns:["status"],queryId:"original"},true).rows.length,1);
    assert.throws(()=>decodeHostedQueryResult({...result,columns:["status"],queryId:"original"}));
});

test("local frozen output unavailability remains a displayable result without a fabricated ID",()=>{
    const raw=localQueryResult();delete (raw as {queryId?:string}).queryId;
    const result=decodeLocalQueryResult({...raw,exportUnavailable:message("common.unavailable")});
    assert.equal(result.queryId,undefined);assert.ok(result.exportUnavailable);
    for(const invalid of [{...raw},{...raw,queryId:"id",exportUnavailable:message("common.unavailable")},{...raw,queryId:"id",diagnostics:["old message"]},{...raw,queryId:"id",totalKnown:Number.MAX_SAFE_INTEGER+1},{...raw,queryId:"id",engineDiagnostics:[{entityKey:"a",diagnostics:[{severity:"error",message:"old"}]}]}])assert.throws(()=>decodeLocalQueryResult(invalid));
});
