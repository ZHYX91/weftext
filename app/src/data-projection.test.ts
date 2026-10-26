import test from "node:test";
import assert from "node:assert/strict";
import { decodeCell, decodeProjection, decodeProjectionResponse } from "./data-projection";
import { decodeNativeValue } from "./native-values";
import { descriptionUiText } from "./ui-diagnostics";
import { locale, message, uiTextValue, UiError } from "./ui-locale";

const description = () => ({explanation:message("common.unavailable"),diagnostics:[{origin:"typst" as const,text:"原诊断\ud800\0"}],causes:[]});
const projection = () => ({qualification:"qualified",id:null,types:[],resolvedTypes:[],fields:[{fieldId:"tasks/status",key:"status",cell:{state:"invalid",value:null,diagnostics:[description()]},sourceRange:null,raw:null,writable:false,required:false,unbound:[]}],unknownFields:[],diagnostics:[description()],budgetExhausted:false});
const response = () => ({projection:projection(),serial:7,sourceHash:"a".repeat(64),schemaRevision:"00000000-0000-4000-8000-000000000001" as string|null,schemaHash:"b".repeat(64),readOnly:false});
const invalidResponse = (error:unknown):boolean => error instanceof UiError && error.message === uiTextValue(message("error.invalidResponse"));

test("Cells preserve invalid values, precise unavailability kinds and every typed diagnostic", () => {
    const invalid=decodeCell({state:"invalid",value:{kind:"decimal",value:"9007199254740993.0001"},diagnostics:[description()]});
    assert.equal(invalid?.state,"invalid"); assert.deepEqual(invalid!.value,{kind:"decimal",value:"9007199254740993.0001"});
    const unavailable=decodeCell({state:"unavailable",kind:"budget",reason:description(),diagnostics:[description(),description()]});
    assert.equal(unavailable?.state,"unavailable"); if(unavailable?.state!=="unavailable") return;
    assert.equal(unavailable.kind,"budget"); assert.equal(unavailable.diagnostics.length,2);
    locale.set("en-US"); assert.match(uiTextValue(descriptionUiText(unavailable.reason)),/Unavailable/); assert.ok(uiTextValue(descriptionUiText(unavailable.reason)).endsWith("原诊断\ud800\0")); locale.set("zh-CN");
    for(const value of [{state:"invalid",diagnostics:[]},{state:"unavailable",kind:"budget",reason:"old message",diagnostics:[]},{state:"unavailable",kind:"guess",reason:description(),diagnostics:[]},{state:"value",value:{kind:"none"},reason:description()},{state:"invalid",value:null,diagnostics:["old message"]}]) assert.equal(decodeCell(value),undefined);
});

test("projection decoding rejects stale shapes and retains original author-shaped records", () => {
    const value=projection(), decoded=decodeProjection(JSON.parse(JSON.stringify(value)));
    assert.equal(decoded.fields[0].cell.state,"invalid"); assert.equal(decoded.budgetExhausted,false);
    for(const wrong of [{...value,diagnostics:["old"]},{...value,budgetExhausted:undefined},{...value,unknownFields:[{key:"author",raw:"original",range:{from:0,to:8},value:{kind:"text",value:"raw"},extra:true}]},{...value,fields:[{...value.fields[0],cell:{state:"invalid",value:null,diagnostics:[{...description(),extra:true}]}}]}]) assert.throws(()=>decodeProjection(wrong));
    const original={kind:"record",value:[{key:"key",value:{kind:"text",value:"common.save"}},{key:"params",value:{kind:"record",value:[]}},{key:"status",value:{kind:"text",value:"open"}}]};
    const author=decodeNativeValue(original); assert.deepEqual(author,original); locale.set("en-US"); assert.deepEqual(author,original); locale.set("zh-CN");
});

test("raw projection decoding rejects transport envelopes and derived UI flags", () => {
    for(const value of [{projection:projection()}, {projection:projection(),readOnly:true}, response(), {...projection(),readOnly:false}])
        assert.throws(()=>decodeProjection(value),invalidResponse);
});

test("the local projection response retains all source and schema metadata independently of read-only state", () => {
    for(const value of [response(), {...response(),serial:0,schemaRevision:null,readOnly:true}, {...response(),serial:Number.MAX_SAFE_INTEGER}]) {
        const decoded=decodeProjectionResponse(JSON.parse(JSON.stringify(value)));
        assert.equal(decoded.serial,value.serial);
        assert.equal(decoded.sourceHash,value.sourceHash);
        assert.equal(decoded.schemaRevision,value.schemaRevision);
        assert.equal(decoded.schemaHash,value.schemaHash);
        assert.equal(decoded.readOnly,value.readOnly);
        assert.equal(Object.hasOwn(decoded.projection,"readOnly"),false);
        assert.equal(decoded.projection.fields[0].cell.state,"invalid");
        assert.equal(decoded.projection.diagnostics[0].diagnostics[0].text,"原诊断\ud800\0");
    }
});

test("the local projection response requires the exact six-key producer envelope", () => {
    const value=response();
    for(const key of Object.keys(value)) {
        const missing=Object.fromEntries(Object.entries(value).filter(([name])=>name!==key));
        assert.throws(()=>decodeProjectionResponse(missing),invalidResponse,`missing ${key}`);
    }
    for(const wrong of [projection(), {projection:projection(),readOnly:false}, {...value,unexpected:true},
        {resource:"resource",field:value.projection.fields[0],basis:null,qualification:"staticAuthorizedField"}])
        assert.throws(()=>decodeProjectionResponse(wrong),invalidResponse);
});

test("malformed snapshot metadata or nested projections cannot become a local response", () => {
    const value=response();
    const invalidMetadata:Record<string,unknown[]>={
        serial:[-1,0.5,Number.MAX_SAFE_INTEGER+1,NaN,Infinity,"7",null],
        sourceHash:["a".repeat(63),"a".repeat(65),"A".repeat(64),"g".repeat(64),null,7],
        schemaHash:["b".repeat(63),"b".repeat(65),"B".repeat(64),"z".repeat(64),null,7],
        schemaRevision:[undefined,7,{},[]],
        readOnly:[undefined,"false",0,null],
    };
    for(const [key,values] of Object.entries(invalidMetadata))
        for(const invalid of values) assert.throws(()=>decodeProjectionResponse({...value,[key]:invalid}),invalidResponse,`invalid ${key}`);
    for(const invalid of [null,{projection:projection()}, {...projection(),readOnly:true}, {...projection(),diagnostics:["untyped error"]}])
        assert.throws(()=>decodeProjectionResponse({...value,projection:invalid}),invalidResponse);
});
