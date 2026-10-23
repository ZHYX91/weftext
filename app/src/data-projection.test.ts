import test from "node:test";
import assert from "node:assert/strict";
import { decodeCell, decodeProjection } from "./data-projection";
import { decodeNativeValue } from "./native-values";
import { descriptionUiText } from "./ui-diagnostics";
import { locale, message, uiTextValue } from "./ui-locale";

const description = () => ({explanation:message("common.unavailable"),diagnostics:[{origin:"typst" as const,text:"原诊断\ud800\0"}],causes:[]});
const projection = () => ({qualification:"qualified",id:null,types:[],resolvedTypes:[],fields:[{fieldId:"tasks/status",key:"status",cell:{state:"invalid",value:null,diagnostics:[description()]},sourceRange:null,raw:null,writable:false,required:false,unbound:[]}],unknownFields:[],diagnostics:[description()],budgetExhausted:false});

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
    assert.equal(decodeProjection({projection:value,readOnly:true}).readOnly,true);
    assert.equal(decodeProjection({projection:{...value,readOnly:true},readOnly:false}).readOnly,true);
    for(const wrong of [{...value,diagnostics:["old"]},{...value,budgetExhausted:undefined},{...value,unknownFields:[{key:"author",raw:"original",range:{from:0,to:8},value:{kind:"text",value:"raw"},extra:true}]},{...value,fields:[{...value.fields[0],cell:{state:"invalid",value:null,diagnostics:[{...description(),extra:true}]}}]}]) assert.throws(()=>decodeProjection(wrong));
    const original={kind:"record",value:[{key:"key",value:{kind:"text",value:"common.save"}},{key:"params",value:{kind:"record",value:[]}},{key:"status",value:{kind:"text",value:"open"}}]};
    const author=decodeNativeValue(original); assert.deepEqual(author,original); locale.set("en-US"); assert.deepEqual(author,original); locale.set("zh-CN");
});
