import { message, uiTextValue } from "./ui-locale";
import test from "node:test";
import assert from "node:assert/strict";
import { citationCandidateKey, citationInsertionByte, citationInputScope, decodeNativeCitationSnapshot, type NativeCitationSnapshot } from "./native-citations";
import { locale, renderUiMessage } from "./ui-locale";

const profile: NativeCitationSnapshot = { generation: "generation", coverage: "partial", choices: [{ reference: "work-a", title: "A", citationReady: true, suggestedKey: "zhang2026" }, { reference: "work-b", title: "B", citationReady: true, suggestedKey: "zhang2026" }], editor: { sourceHash: "hash", bindings: [{ key: "existing", work: "work-a" }], citations: [], targets: [], crossReferences: [], renameAvailable: true }, bindings: [], incoming: [], displayStatus: "unavailable", displayExplanation: message("common.unavailable") };
test("retained citation inputs belong to one actor and stable owner rather than a replaced session or locator",()=>{const context={deployment:"server",actor:"alice",resource:"OldPath"};const scope=citationInputScope(context,"stable-owner");assert.equal(scope,citationInputScope({...context,resource:"MovedPath"},"stable-owner"));assert.notEqual(scope,citationInputScope({...context,actor:"bob"},"stable-owner"));assert.notEqual(scope,citationInputScope(context,"another-owner"));assert.notEqual(scope,citationInputScope({...context,deployment:"elsewhere"},"stable-owner"));});
test("citation key reuses stable Work binding even when the publication suggestion changes", () => {
  assert.equal(citationCandidateKey(profile, "work-a").key, "existing");
  assert.equal(citationCandidateKey({ ...profile, choices: profile.choices.map(choice => ({ ...choice, suggestedKey: "changed" })) }, "work-a").key, "existing");
  const multiple = { ...profile, editor: { ...profile.editor, bindings: [...profile.editor.bindings, { key: "alias", work: "work-a" }] } };
  assert.equal(citationCandidateKey(multiple, "work-a").key, "");
  assert.match(uiTextValue(citationCandidateKey(multiple, "work-a").reason!), /多个绑定键/);
});
test("different Works and document labels never silently coalesce by suggested key", () => {
  const occupied = { ...profile, editor: { ...profile.editor, bindings: [{ key: "zhang2026", work: "work-a" }] } };
  assert.equal(citationCandidateKey(occupied, "work-b").key, "");
  assert.match(uiTextValue(citationCandidateKey(occupied, "work-b").reason!), /冲突/);
  const invalid = { ...profile, choices: [{ ...profile.choices[1], suggestedKey: "张 2026" }] };
  assert.equal(citationCandidateKey(invalid, "work-b").key, "");
  assert.match(uiTextValue(citationCandidateKey(invalid, "work-b").reason!), /ASCII/);
});
test("insertion offsets preserve UTF-8 bytes and reject a split supplementary scalar", () => {
  assert.equal(citationInsertionByte("中文😀\r\nbody", 4), 10);
  assert.throws(() => citationInsertionByte("中文😀\r\nbody", 3));
  assert.throws(() => citationInsertionByte("abc", -1));
  assert.throws(() => citationInsertionByte("abc", 4));
});

test("citation product descriptions are explicit while original titles and keys retain their values",()=>{
  const sourceRange={from:0,to:1,utf16From:0,utf16To:1,raw:"@",writable:false};
  const reason={explanation:message("common.unavailable"),diagnostics:[{origin:"typst",text:"原诊断\ud800"}],causes:[]};
  const slot={key:"authored-key",bindingProven:true,range:sourceRange,form:null,formExplicit:true,supplement:null,supplementExplicit:false,style:null,styleExplicit:false,editable:false,reason};
  const original={...profile,choices:[{reference:"work-a",title:null,titlePlaceholder:message("common.unavailable"),node:{workspaceId:"workspace",nodeId:"node"},citationReady:false,suggestedKey:"original-key",reason}],editor:{...profile.editor,citations:[slot],renameReason:reason},links:[{range:sourceRange,target:"original-author-target",title:null,editable:false,reason}],bibliographyOwnership:"unavailable",displayed:[]};
  const decoded=decodeNativeCitationSnapshot(JSON.parse(JSON.stringify(original)));
  assert.equal(decoded.choices[0].title,null);assert.equal(decoded.editor.citations[0].key,"authored-key");
  locale.set("en-US");assert.equal(renderUiMessage("en-US",decoded.choices[0].titlePlaceholder!),"Unavailable");assert.equal(decoded.editor.citations[0].reason!.diagnostics[0].text,"原诊断\ud800");locale.set("zh-CN");
  for(const malformed of [{...original,displayExplanation:"old message"},{...original,editor:{...original.editor,renameReason:"old reason"}},{...original,choices:[{...original.choices[0],reason:{...reason,details:{}}}]},{...original,editor:{...original.editor,citations:[{...slot,reason:undefined}]}}])assert.throws(()=>decodeNativeCitationSnapshot(malformed));
});

test("hosted citation rows admit their actual resource projection without accepting local private fields",()=>{
  const title='{"key":"common.save","params":{}} 作者原题';
  const hosted={...profile,ownerReference:"owner",choices:[{resource:"actual-resource",reference:"work-a",title,citationReady:true,suggestedKey:"key",reason:null}],editor:{...profile.editor,renameReason:null},links:[],bibliographyOwnership:"singleVirtual"};
  const decoded=decodeNativeCitationSnapshot(hosted,"hosted");assert.equal(decoded.choices[0].title,title);
  assert.throws(()=>decodeNativeCitationSnapshot(hosted));
  assert.throws(()=>decodeNativeCitationSnapshot({...hosted,choices:[{...hosted.choices[0],node:{workspaceId:"private",nodeId:"private"}}]},"hosted"));
});
