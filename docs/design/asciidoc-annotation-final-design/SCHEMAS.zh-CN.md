---
source_language: zh-CN
translation_status: source
---

[English](SCHEMAS.md)

# AsciiDoc / Annotation 最终设计：闭合 Schema 与跨 owner 现行契约

状态：**candidate-design-not-implemented**。本文件是 [SPEC.zh-CN.md](SPEC.zh-CN.md) 的规范性闭合-schema伴随文件。SPEC 描述行为与算法；本文件冻结 current successor 的成员、union、排序与历史分派。两者冲突时，具名 closed shape 以本文件为准，行为约束以 SPEC 为准；任何一方都不得被解释成允许未列成员。

固定 parent：e8aa0b341630a57c786c0891d4bbd1620247441d。未在本文重新定义的嵌套类型（如 WorkspaceRef、NodeRef、Frontier/2、PinRef/2）使用 replacements 所指 fixed-parent owner 的 exact decoder；这不是“同旧版”省略成员，而是显式 import 一个未变化的具名类型。

## 0. 通用规则

- 所有 object closed：unknown/missing/duplicate member、非法 null、错误 union arm 拒绝。
- JSON integer 不接受 Boolean、浮点或指数拼写；需要跨实现稳定的任意整数值用 canonical decimal text。
- 文本不做未声明的 NFC、case-fold 或 trim。
- 有序语言序列保持顺序；set按该类型定义的 canonical key排序唯一；multiset排序但保留 multiplicity。
- D3-CJ/3 是所有本文具名 canonical bytes 的唯一 JSON canonicalizer。
- current successor 只用于 unseen/current producer；真实 saved/planned/unknown 历史按原 exact decoder、bytes、pins、recovery责任恢复，不后台迁移。
- “有 decoder/历史设计正文”不等于证明该版本曾部署。

# 1. D2 Processor Environment

```text
AsciiDocProcessorEnvironment/3 = {
  kind:"weftext_asciidoc_processor_environment",
  version:3,
  languageBaseline:
    "asciidoctor-ruby/2.0.26@0b99b39c9df884d4aec13bba45f03cdbab505769",
  documentProfile:"baseline_only/1"|"weftext_managed/1",
  backend:ProcessorBackendSemanticProfile/1,
  doctype:"unset"|"article"|"book"|"manpage"|"inline",
  safeMode:"unsafe"|"safe"|"server"|"secure",
  standalone:Boolean,
  input:ProcessorInputIdentity/2,
  output:ProcessorOutputIdentity/1,
  baseDir:ProcessorPath/1,
  ambientUserHome:ProcessorPath/1,
  attributeOverrides:[ProcessorAttributeOverride/1...],
  timeInputs:ProcessorTimeInputs/1,
  includeEnvironment:IncludeEnvironment/2,
  extensionProfiles:[AcceptedProcessorExtensionProfile/1...]
}

ProcessorPath/1 = {
  kind:"virtual_posix_absolute",
  value:text
}

ProcessorBackendSemanticProfile/1 = {
  profileId:text,
  backend:text,
  basebackend:text,
  filetype:text,
  outfilesuffix:text,
  semanticDescriptorSha256:"sha256:<64 lowercase hex>"
}

ProcessorInputIdentity/2 =
    {kind:"managed_file",ownerNodeRef:NodeRef,
     sourceObservation:SourceObservation/1,
     logicalDocfile:ProcessorPath/1,logicalDocdir:ProcessorPath/1}
  | {kind:"artifact_file",artifactPin:PinRef/2,
     logicalDocfile:ProcessorPath/1,logicalDocdir:ProcessorPath/1}
  | {kind:"string_input",logicalName:text}

ProcessorOutputIdentity/1 = {
  logicalOutfile:ProcessorPath/1|null,
  logicalOutdir:ProcessorPath/1|null
}

ProcessorAttributeOverride/1 = {
  name:text,
  action:"hard_set"|"soft_set"|"hard_unset"|"soft_unset",
  value:text|null
}

CanonicalSignedDecimal = "0" | "-"?[1-9][0-9]*

ProcessorInstant/1 = {
  utcSeconds:CanonicalSignedDecimal,
  offsetMinutes:integer(-1439..1439)
}

ProcessorTimeInputs/1 = {
  sourceDateEpochSeconds:CanonicalSignedDecimal|null,
  inputMtime:ProcessorInstant/1|null,
  clockNow:ProcessorInstant/1
}

SourceUnitBinding/1 = {
  logicalPath:ProcessorPath/1,
  source:
      {kind:"managed",ownerNodeRef:NodeRef,
       sourceObservation:SourceObservation/1}
    | {kind:"artifact",pin:PinRef/2}
    | {kind:"network_snapshot",
       requestDigest:"sha256:<64 lowercase hex>",pin:PinRef/2}
}

IncludeEnvironment/2 = {
  resolverProfile:"weftext_virtual_include/1",
  maxDepth:integer(0..65535),
  uriRead:"disabled"|"authorized_snapshot",
  sourceUnits:[SourceUnitBinding/1...]
}

AcceptedProcessorExtensionProfile/1 = {
  profileId:text,
  semanticVersion:text,
  descriptorSha256:"sha256:<64 lowercase hex>"
}
```

attributeOverrides 按 lowercase name排序唯一；extensionProfiles按profileId排序唯一；sourceUnits按logicalPath.value排序唯一。set action要求value非null；unset要求null。safe<server 且未override时 user-home来自ambientUserHome；server/secure原生default为“.”。SOURCE_DATE_EPOCH存在时local*/doc*统一取其UTC值；否则local*取clockNow，doc*优先inputMtime、再clockNow。

# 2. Core Semantic Projection

```text
UInt = nonnegative JSON integer
DecimalInteger = "0" | "-"?[1-9][0-9]*

SemanticValue =
    null | Boolean | text
  | {integer:DecimalInteger}
  | {array:[SemanticValue...]}

Property = [name:text,value:SemanticValue]

CoreSemanticProjection/1 = {
  format:"weftext.core-semantic-projection",
  version:1,
  document:CoreDocument/1,
  blocks:[CoreBlock/1...],
  catalog:CoreCatalog/1,
  indexTerms:[CoreIndexTerm/1...],
  finalState:CoreFinalState/1,
  diagnostics:[CoreDiagnostic/1...]
}

CoreDocument/1 = {
  doctype:"article"|"book"|"manpage"|"inline",
  title:null|{
    combined:CoreFlow/1,
    main:CoreFlow/1,
    subtitle:CoreFlow/1|null
  },
  authors:[{
    name:text|null,firstname:text|null,middlename:text|null,
    lastname:text|null,initials:text|null,email:text|null
  }...],
  revision:null|{number:text|null,date:text|null,remark:text|null}
}

CoreBlock/1 = {
  kind:CoreBlockKind/1,
  level:DecimalInteger|null,
  properties:[Property...],
  title:CoreFlow/1|null,
  reftext:CoreFlow/1|null,
  body:CoreFlow/1|null,
  children:[CoreBlock/1...]
}

CoreBlockKind/1 =
  preamble|section|floating_title|paragraph|
  open|example|sidebar|listing|literal|stem|pass|
  quote|verse|admonition|ulist|olist|dlist|colist|
  list_item|dlist_entry|dlist_term|dlist_description|
  table|table_head|table_body|table_foot|table_row|table_cell|
  image|audio|video|thematic_break|page_break|toc

CoreFlow/1 = {
  runs:[CoreRun/1...],
  marks:[CoreMark/1...]
}

CoreRun/1 =
    {kind:"text",value:text}
  | {kind:"soft_break"}
  | {kind:"hard_break"}
  | {kind:"atom",
     name:"image"|"icon"|"stem"|"footnote_ref"|"callout"|"kbd"|"button"|"menu",
     properties:[Property...]}
  | {kind:"raw",medium:"passthrough"|"backend_feedback",value:text}

CoreMark/1 = {
  kind:"strong"|"emphasis"|"monospace"|"mark"|
       "superscript"|"subscript"|"double_quote"|"single_quote"|
       "role"|"link"|"xref",
  start:UInt,
  end:UInt,
  properties:[Property...]
}

CoreSite/1 =
    {kind:"flow",path:SemanticPath,start:UInt,end:UInt}
  | {kind:"scalar",path:SemanticPath,field:text,start:UInt,end:UInt}
  | {kind:"node",path:SemanticPath}
  | {kind:"detached",owner:SemanticPath}

CoreIndexTerm/1 = {
  site:CoreSite/1,
  visibility:"visible"|"concealed",
  terms:[text...],
  see:text|null,
  seeAlso:[text...]
}

CoreCatalog/1 = {
  anchors:[{id:text,reftext:CoreFlow/1|null,site:CoreSite/1}...],
  footnotes:[{index:DecimalInteger,id:text|null,body:CoreFlow/1}...],
  links:[{target:text}...],
  images:[{target:text,properties:[Property...]}...],
  includes:[{target:text,properties:[Property...]}...],
  callouts:[{list:UInt,item:UInt,properties:[Property...]}...]
}

CoreFinalState/1 = {
  attributeChanges:[{
    name:text,
    result:{kind:"absent"}|{kind:"present",value:SemanticValue}
  }...],
  counters:[{name:text,value:SemanticValue}...]
}

CoreDiagnostic/1 = {
  severity:"debug"|"info"|"warn"|"error"|"fatal",
  semanticCode:text,
  location:null|{logicalFile:text|null,line:UInt|null}
}
```

Property name按UTF-8 byte order唯一。marks按start升序、end降序、kind固定枚举顺序、D3-CJ/3(properties)排序；完全相同formatting mark归一一次。一个Unicode scalar计一个flow位置；break/atom各计1。visible text只能拥有一次；link/xref label由runs覆盖，不另复制文本property。

Catalog：anchors按id；footnotes按数值index；links/images/includes按D3-CJ/3(item)排序且保留multiplicity；callouts按list/item。diagnostics按null-first logicalFile/line、severity rank、semanticCode排序并保留重复。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

# 3. CoreSemanticPropertyProfile/1

来源只有四类：A=authored_named、P=authored_positional、F=fixed_derived、I=internal。projector只能消费已经观察到的最终model/consumer事实，不重跑parser/model。

公共真实block canonical slots：

```text
id
style
roles
options
caption
numeral
subs
positional
named/<original-name>
```

已被 CoreBlock.kind/level/title/reftext/body/children 表达的事实不得重复写property。

逐类必须收录：

| kind | 额外current semantic fields |
|---|---|
| section | sectname, special, numbered, actual numeral/caption |
| listing | language, linenums, start, indent, tabsize, highlight, line-comment |
| literal | indent, tabsize, line-comment |
| stem | native normalized notation/style |
| quote / verse | attribution, citetitle；verse另含真实indent/tabsize |
| admonition | name, textlabel, actual icon |
| ulist | final list style, checklist/interactive等真实options |
| olist | numbering style, actual start, reversed option |
| dlist | style, labelwidth, itemwidth, real options |
| list_item | marker, checklist state, actual coids |
| table | cols, format, separator, width, frame, grid, stripes, float, orientation, colcount, rowcount, tablepcwidth, tableabswidth(if produced), columns |；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。
| table_cell | colspan,rowspan,halign,valign,cellStyle |
| image | target,alt,width,height,format,scaledwidth,scale,link,window,float,align,fallback及真实title/imagesdir |；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。
| audio | target,start,end及公共options |
| video | target,poster,width,height,start,end,preload,float,align,hash,theme,lang,list,playlist |；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。
| toc | actual levels；title仍在CoreBlock.title |

合成的 dlist_entry/table_head/table_body/table_foot/table_row 若没有真实author对象，其properties为空。

marks/atoms：link必须target；xref必须target/refid/path；image/icon使用对应media字段；stem含notation+evaluated source；footnote_ref含index/referenceKind/target；callout含number/id/guard；kbd含keys有序数组；button含text；menu含menu/submenus/menuitem。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

显式author foo-option=bar 同时生成 options=["foo"] 与 named/foo-option="bar"；显式空值保留空字符串。由 %foo/options=foo/opts=foo 产生的物理空 foo-option 只表达membership，不伪造named author value。temporary/internal writer（例如DocBook root-option）不能按字段名冒充author。

# 4. Witness/7 retained evidence 与 model evidence

## 4.1 Oracle baseline、observer manifest 与 retained evidence closed types

以下类型全部是**测试 oracle 证据**，不是产品持久 authority、第二 parser 或 renderer wire。它们只记录固定 Ruby 2.0.26 实际求值已经产生的事实；observer 不得为了填充成员重新调用 parser/getter、改 evaluator string、替换 converter 返回值、插 marker/sentinel 或向文档语义写入状态。

```text
OracleBaseline/1 = {
  kind:"asciidoctor_ruby_baseline",
  version:1,
  repository:"asciidoctor/asciidoctor",
  release:"2.0.26",
  sourceCommit:"0b99b39c9df884d4aec13bba45f03cdbab505769",
  observerContract:"ruby_actual_evaluation_observer/1",
  observerManifestSha256:"sha256:<64 lowercase hex>"
}

OracleObserverManifest/1 = {
  kind:"ruby_actual_evaluation_observer_manifest",
  version:1,
  sourceCommit:"0b99b39c9df884d4aec13bba45f03cdbab505769",
  rubyRuntime:{
    implementation:text,
    version:text,
    buildId:text
  },
  sites:[OracleObserverSite/1...]
}

OracleObserverSite/1 = {
  siteId:UInt,
  file:text,
  fileBlobSha1:"<40 lowercase hex>",
  method:text,
  callSite:text,
  observedOperation:text,
  captureRule:text,
  rangeTransferRule:text
}

OracleDocumentState/2 = {
  backend:text,
  basebackend:text,
  doctype:"article"|"book"|"manpage"|"inline",
  safeMode:"unsafe"|"safe"|"server"|"secure",

  doctitle:{
    combined:text|null,
    main:text|null,
    subtitle:text|null
  },

  authors:[{
    name:text,
    firstname:text|null,
    middlename:text|null,
    lastname:text|null,
    initials:text|null,
    email:text|null
  }...],

  revision:null|{
    number:text|null,
    date:text|null,
    remark:text|null
  },

  attributes:[{name:text,value:text}...]
}

OracleBlockContext/1 =
    "admonition"|"audio"|"colist"|"dlist"|"document"|"example"|
    "floating_title"|"image"|"listing"|"literal"|"olist"|"open"|
    "page_break"|"paragraph"|"pass"|"preamble"|"quote"|"section"|
    "sidebar"|"stem"|"table"|"thematic_break"|"toc"|"ulist"|
    "verse"|"video"

OracleContentModel/1 =
    "compound"|"simple"|"verbatim"|"raw"|"empty"

OracleBlockEvent/2 = {
  ordinal:UInt,
  parentOrdinal:UInt|null,
  context:OracleBlockContext/1,
  contentModel:OracleContentModel/1|null,
  level:CanonicalSignedDecimal|null,
  style:text|null,
  id:text|null,
  title:text|null,
  roles:[text...],
  options:[text...],
  attributes:[{name:text,value:text}...]
}

OracleCollectionEvent/1 =
    {kind:"list_item",
     listOrdinal:UInt,itemOrdinal:UInt,
     listContext:"ulist"|"olist"|"colist"|"dlist_term"|"dlist_description",
     marker:text|null,style:text|null,text:text|null,
     checklist:"none"|"checked"|"unchecked"}

  | {kind:"table_cell",
     tableOrdinal:UInt,
     section:"head"|"body"|"foot",
     rowOrdinal:UInt,columnOrdinal:UInt,
     colspan:UInt|null,rowspan:UInt|null,
     style:text|null,text:text|null}

OracleCatalogEvent/1 =
    {kind:"ref",id:text,reftext:text|null,
     targetKind:"section"|"inline_ref"|"bibref"|"other"}
  | {kind:"footnote",index:UInt,id:text|null,text:text|null}
  | {kind:"link",target:text}
  | {kind:"image",target:text}
  | {kind:"include",target:text}
  | {kind:"callout",listOrdinal:UInt,itemOrdinal:UInt}

OracleDiagnostic/1 = {
  severity:"debug"|"info"|"warn"|"error"|"fatal",
  semanticCode:text,
  exactMessage:text,
  logicalFile:text|null,
  line:UInt|null
}

OracleObservedString/1 = {
  valueId:UInt,
  rubyEncoding:text,
  bytesBase64:text,
  byteLength:UInt
}

OracleObservedSlice/1 = {
  valueId:UInt,
  startByte:UInt,
  endByte:UInt
}

OracleEvaluationCallKind/1 =
    "parse"|"attribute_playback"|"content"|"title"|"list_text"|
    "cell_text"|"cell_content"|"apply_subs"|"substitution_stage"|
    "inline_convert"|"block_convert"|"metadata_access"|"effect"

OracleEvaluationCall/1 = {
  callId:UInt,
  parentCallId:UInt|null,
  siteId:UInt,
  nodeOrdinal:UInt|null,
  invocationOrdinal:UInt,
  kind:OracleEvaluationCallKind/1
}

OracleStringOperationKind/1 =
    "copy"|"slice"|"concat"|"join"|"regex_replace"|"split_piece"|
    "character_map"|"trim"|"delete"|"normalize"|"encode"|
    "converter_return"|"scalar_assign"

OracleOutputRunRelation/1 =
    {kind:"exact_copy",input:OracleObservedSlice/1}
  | {kind:"derived",inputs:[OracleObservedSlice/1...],operationId:UInt}
  | {kind:"generated",siteId:UInt,inlineEventId:UInt|null}

OracleOutputRun/1 = {
  startByte:UInt,
  endByte:UInt,
  relation:OracleOutputRunRelation/1
}

OracleStringOperation/1 = {
  operationId:UInt,
  callId:UInt,
  siteId:UInt,
  kind:OracleStringOperationKind/1,
  inputs:[OracleObservedSlice/1...],
  outputValueId:UInt,
  runs:[OracleOutputRun/1...],
  removedInputs:[OracleObservedSlice/1...]
}

OracleFieldPathSegment/1 =
    {kind:"field",name:text}
  | {kind:"attribute",name:text}
  | {kind:"position",index:UInt}
  | {kind:"array",index:UInt}

OracleFieldEntryKey/1 =
    {kind:"name",value:text}
  | {kind:"position",index:UInt}
  | {kind:"symbol",value:text}

OracleFieldValue/1 =
    null
  | {kind:"boolean",value:Boolean}
  | {kind:"integer",decimal:CanonicalSignedDecimal}
  | {kind:"symbol",value:text}
  | {kind:"observed_string",valueId:UInt}
  | {kind:"array",items:[OracleFieldValue/1...]}
  | {kind:"entries",
     items:[{key:OracleFieldEntryKey/1,value:OracleFieldValue/1}...]}

OracleFieldObservation/1 = {
  path:[OracleFieldPathSegment/1...],
  value:OracleFieldValue/1
}

OracleInlineContext/1 =
    "anchor"|"break"|"button"|"callout"|"footnote"|"image"|
    "indexterm"|"kbd"|"menu"|"quoted"

OracleInlineObservation/3 = {
  eventId:UInt,
  callId:UInt,
  parentCallId:UInt|null,
  producingOperationId:UInt|null,
  context:OracleInlineContext/1,
  nodeType:OracleFieldValue/1,
  fields:[OracleFieldObservation/1...],
  returnValueId:UInt|null
}

OracleContentRole/1 =
    "body"|"title"|"list_item"|"table_cell"|
    "footnote_body"|"reftext"|"raw_content"

OracleContentObservation/1 = {
  callId:UInt,
  role:OracleContentRole/1,
  resultValueIds:[UInt...],
  contributingInlineEventIds:[UInt...]
}

M37Site/1 = {
  group:
    "M37-01"|"M37-02"|"M37-03"|"M37-04"|"M37-05"|"M37-06"|
    "M37-07"|"M37-08"|"M37-09"|"M37-10"|"M37-11"|"M37-12"|
    "M37-13"|"M37-14"|"M37-15"|"M37-16"|"M37-17"|"M37-18",
  file:text,
  method:text,
  point:text
}

OracleSemanticWitness/7 = {
  format:"weftext.asciidoc-oracle-witness",
  version:7,
  baseline:OracleBaseline/1,
  processorEnvironmentSha256:"sha256:<64 lowercase hex>",
  document:OracleDocumentState/2,
  blockEvents:[OracleBlockEvent/2...],
  collectionEvents:[OracleCollectionEvent/1...],
  catalogEvents:[OracleCatalogEvent/1...],
  diagnostics:[OracleDiagnostic/1...],
  observedStrings:[OracleObservedString/1...],
  calls:[OracleEvaluationCall/1...],
  operations:[OracleStringOperation/1...],
  inlineObservations:[OracleInlineObservation/3...],
  contentObservations:[OracleContentObservation/1...],
  modelPropertyObservations:[OracleModelPropertyObservation/1...]
}

ModelSubject/1 = {
  id:UInt,
  kind:"document"|"section"|"block"|"list"|"list_item"|"table"|
       "column"|"cell"|"inline"|"attribute_buffer"|
       "table_parser_context"|"catalog_record"
}

ModelAttributeKey/1 =
    {kind:"name",value:text}
  | {kind:"position",decimal:DecimalInteger}
  | {kind:"symbol",value:text}

ModelObservedValue/1 =
    {kind:"nil"}
  | {kind:"boolean",value:Boolean}
  | {kind:"integer",decimal:DecimalInteger}
  | {kind:"symbol",name:text}
  | {kind:"float64",ieee754beHex:"<16 lowercase hex>"}
  | {kind:"string",observedStringId:UInt}
  | {kind:"array",items:[ModelObservedValue/1...]}
  | {kind:"map",entries:[{key:ModelAttributeKey/1,value:ModelObservedValue/1}...]}
  | {kind:"opaque_internal",classTag:text}

ModelSlotState/1 =
    {state:"present",value:ModelObservedValue/1,
     provenance:ModelPropertyProvenance/1}
  | {state:"absent",cause:"never_initialized"|"deleted",
     provenance:ModelPropertyProvenance/1|null}

ModelPropertyProvenance/1 = {
  class:"authored_named"|"authored_positional"|"fixed_derived"|"internal",
  form:"named_assignment"|"positional_assignment"|"style_shorthand"|
       "option_syntax"|"rekey"|"copy"|"inherit"|"computed"|"default"|
       "delete"|"temporary"|"restore",
  writerSite:M37Site/1,
  inputs:[ModelPropertyInput/1...]
}

ModelPropertyInput/1 =
    {kind:"model_slot",observationId:UInt,slot:ModelSlotAddress/1}
  | {kind:"observed_value",observedStringId:UInt}
  | {kind:"observed_slice",valueId:UInt,startByte:UInt,endByte:UInt}
  | {kind:"operation",operationId:UInt}
  | {kind:"inline_field",inlineEventId:UInt,fieldPath:[text|UInt...]}

ModelSlotAddress/1 =
    {kind:"field",name:ModelFieldName/1}
  | {kind:"attribute",key:ModelAttributeKey/1}

ModelFieldName/1 =
  context|node_name|id|content_model|level|style|caption|numeral|
  sectname|special|numbered|index|marker|colspan|rowspan|
  has_header_option|format|delimiter|type|target|imagesdir

ModelRelation/1 = {
  role:"parent"|"header"|"child"|"dlist_term"|"dlist_description"|
       "table_column"|"table_head_cell"|"table_body_cell"|"table_foot_cell"|
       "cell_column"|"cell_inner_document",
  position:[UInt...],
  targetSubjectId:UInt
}

ModelCarrierReference/1 = {
  stream:"document"|"blockEvents"|"collectionEvents"|"catalogEvents"|
         "inlineObservations"|"contentObservations"|"calls",
  entry:UInt
}

OracleModelPropertyObservation/1 =
    {kind:"snapshot",observationId:UInt,subject:ModelSubject/1,
     previousSnapshotObservationId:UInt|null,captureSite:M37Site/1,
     moment:"after_construct"|"after_write"|"after_finalize"|"before_use"|"after_use",
     fields:[{name:ModelFieldName/1,slot:ModelSlotState/1}...],
     attributes:[{key:ModelAttributeKey/1,slot:ModelSlotState/1}...],
     relations:[ModelRelation/1...],
     coverage:"complete_profile_fields_and_attribute_slots"}
  | {kind:"bind",observationId:UInt,subjectId:UInt,
     snapshotObservationId:UInt,carrier:ModelCarrierReference/1}
  | {kind:"cut",observationId:UInt,captureSite:M37Site/1,
     phase:"model_ready"|"evaluation_complete",documentSubjectId:UInt,
     heads:[{subjectId:UInt,snapshotObservationId:UInt}...]}
```

## 4.2 retained evidence canonical rules 与 producer manifests

`OracleBaseline/1.observerManifestSha256` 必须等于 `SHA-256(D3-CJ/3(OracleObserverManifest/1))`；`processorEnvironmentSha256` 必须等于 exact `AsciiDocProcessorEnvironment/3` canonical bytes 的 SHA-256。manifest 不是 caller 可替换的 coverage 声明：每个 siteId 唯一并按数值升序；fileBlobSha1 必须是固定 sourceCommit 下该文件的真实 Git blob；method/callSite/observedOperation/captureRule/rangeTransferRule 必须对应被独立核过的固定插桩点。缺 site、未知必要分支或 manifest/source 不匹配均为 `oracle_observation_incomplete`，不得把合法 AsciiDoc 改判不支持。

实际 evaluation observer 的完整 producer family 为：

| producer family | 必须记录的实际事实 |
|---|---|
| `AbstractBlock#convert` | attribute playback 后原转换调用、node/parent关联；不替换converter。 |
| `Block#content` | 原输入行、content model、原apply_subs调用与最终返回；包括raw/verbatim裁空行与join。 |
| `Substitutors#apply_subs` | 原 substitutions 实际顺序、每步输入/输出、passthrough提取/恢复关联。 |
| `sub_quotes / convert_quoted_text` | 原 MatchData/captures、Inline构造参数、真实converter返回值。 |
| `sub_attributes / sub_replacements` | 原match区间、替换结果、drop/drop-line以及counter/set副作用。 |
| `sub_macros` | 原macro match/captures与Inline text/target/id/refid/path/attributes。 |
| `sub_post_replacements` | 原split/slice/HardLineBreakRx输入区间及break完整text。 |
| `Inline#convert / selected converter` | 实际context/type/scalar参数与返回值；observer不得增加getter调用。 |
| `AttributeList` | 原StringScanner位置/scan/get-byte结果和产生的scalar；不重跑attribute-list parser。 |
| `ListItem#text / Cell#text/#content` | 原getter调用、实际输出和a-cell inner-document调用关系。 |
| `title / reftext` | 原第一次计算、cache hit及实际scalar返回；不为日志触发读取。 |
| footnote/counter/catalog写点 | 原操作输入和实际结果；不二次执行。 |

`OracleObservedString/1.valueId`、`OracleEvaluationCall/1.callId`、`OracleStringOperation/1.operationId`、`OracleInlineObservation/3.eventId` 分别只在各自数组namespace内唯一。相同bytes来自不同来源仍得到不同valueId；Ruby String原地修改后产生新的不可变快照。bytesBase64使用RFC4648 canonical padded Base64，decode长度必须等于byteLength。slice为对应字符串版本的字节半开区间，必须满足 `0 <= startByte <= endByte <= byteLength`；evaluation-string byte位置不冒充.adoc source range。

`OracleStringOperation/1.runs` 按startByte升序、无重叠并完整覆盖实际输出；exact_copy必须逐字复制input slice；derived保留全部真实输入，不能伪称一一source mapping；generated记录实际site及可选inline observation。removedInputs保留真实被删除输入，零输出的concealed行为不能因此消失。`OracleOutputRunRelation/1.generated.inlineEventId` 与 `OracleContentObservation/1.contributingInlineEventIds` 都只引用 `OracleInlineObservation/3.eventId`；保留的字段名不构成、也不得解码成已经退役的 `OracleInlineEvent`/marker observer。

`OracleInlineObservation/3.fields` 路径按 `D3-CJ/3(path)` 排序唯一；字符串leaf必须引用真实ObservedString。OracleFieldValue不接受Ruby object的 `to_s` 逃逸。ContentObservation的resultValueIds按原String/Array返回结构顺序，普通文本来自真实content/list/cell/title/reftext返回而不是HTML flatten。BlockEvent roles/options保留Ruby当前顺序，attributes按name排序唯一；DocumentState attributes同样按name排序唯一。CatalogEvent没有indexterm fallback，因为固定2.0.26 catalog不保存 `:indexterms`。Diagnostic exactMessage保留在evidence；跨实现CSP仍只比较已定义common diagnostic projection。

M37 writer-site闭集如下；`M37Site/1` 的 `(group,file,method,point)` 必须命中对应固定 producer family，point是manifest中的稳定分支标识而不是自由字符串：

| group | fixed producer family | capture point |
|---|---|---|
| M37-01 | `AbstractNode#initialize`, `AbstractBlock#initialize`, `Inline#initialize` | 原构造赋值完成后。 |
| M37-02 | `AttributeList#parse_attribute/#parse_into/.rekey` | named/positional/options expansion/copy/rekey真实写后。 |
| M37-03 | `Parser.parse_block_metadata_line/process_attribute_entry/store_attribute/parse_style_attribute/yield_buffered_attribute` | metadata/shorthand/set/unset真实写后。 |
| M37-04 | `Parser.next_block/build_block` | style/media/source/quote/admonition/STEM等赋值后、返回block前。 |
| M37-05 | `AbstractNode#set_attr/remove_attr/set_option/update_attributes/role=/add_role/remove_role`及真实direct Hash writes | 原mutation后，保留真实caller provenance。 |
| M37-06 | `Parser.initialize_section` | sectname/special/numbered/update_attributes完成后。 |
| M37-07 | `AbstractBlock#assign_numeral/#assign_caption`及section attach | numeral/caption/counter写完成后。 |
| M37-08 | `Table#initialize/#create_columns` | table width/orientation/columns/colcount产生后。 |
| M37-09 | `Table::Column#initialize/#assign_width`, `Table#assign_column_widths` | 每次真实width赋值后，最终组快照必须在末列balance之后。 |
| M37-10 | `Table::Cell#initialize/#reinitialize`, `Table#partition_header_footer` | final Cell对象与head/body/foot membership确定后。 |
| M37-11 | `Parser.parse_colspecs/parse_cellspec/parse_table`, `Table::ParserContext#initialize/#close_cell/#close_row/#close_table` | 原parser已得到spec/result和真实列/行/Cell关系后。 |
| M37-12 | `ListItem#initialize/#fold_first`, `Parser.parse_list_item/parse_list/parse_description_list` | marker/checklist/style/fold与term-description组装完成后。 |
| M37-13 | `Parser.parse_callout_list`, `Callouts#register/#callout_ids` 原调用返回点 | late coids真实结果写入后。 |
| M37-14 | `Document#parse`, 原 `AbstractBlock#convert`, `Inline#convert` 与collection consumer入口 | model_ready cutoff；不额外执行parse/convert。 |
| M37-15 | actual inline/converter/alt/title/reftext/media scalar consumer sites | 复用既有求值证据并绑定当时model/attr state。 |
| M37-16 | `Document#register` actual catalog insertion | catalog record primitive fields及parent→actual receiver。 |
| M37-17 | fixed converter temporary model/map writes | temporary写及真实restore或cleanup delete后。 |
| M37-18 | actual root evaluation return | evaluation_complete cutoff；只观察既有state。 |

M37-02/M37-03/M37-05还必须覆盖固定代码中的直接Hash赋值/update/delete/clear，不能只观察wrapper。具体manifest与固定源码共同构成producer-conformance，不允许projector从字段名、source文本或最终HTML补推。

同subject snapshots形成immediate-predecessor单链；cut heads按subjectId数值升序且恰覆盖reachable semantic closure。只有model observation namespace里的previous/bind/cut/model_slot引用做数值先后检查；operation/inline/value/call各用自身namespace。carrier.entry是目标数组index，不与model observationId比较。

**producer-conformance gate**：真实bind callback发生时，目标carrier已append，entry < targetStream.lengthAtBind，并且producer仍持有exact同一Ruby对象；随后才能append bind。final decoder只检查最终存在/type/index，不能从最终数组假造跨stream时序。document/0同样要求actual returned Document carrier先发布。

catalog record的ownership只能是 parent -> actual Document#register receiver；inner Document不得归top-level。temporary overlay必须由真实restore或真实cleanup delete闭合；observer不得伪造restore write。

# 5. Managed format 与 D6 current successor

```text
ManagedDocumentFormatProfile/1 = {
  languageBaseline:
    "asciidoctor-ruby/2.0.26@0b99b39c9df884d4aec13bba45f03cdbab505769",
  managedProfile:"weftext_managed/1"
}

ManagedDocumentFormatBinding/1 = {
  kind:"weftext_managed_document_format",
  version:1,
  ownerNodeRef:NodeRef,
  bindingRevision:Counter,
  profile:ManagedDocumentFormatProfile/1
}

DocumentFormatDependencyKey/1 = {
  kind:"document_format",
  workspaceRef:WorkspaceRef,
  ownerNodeRef:NodeRef
}

DocumentFormatCurrentQualification/1 = {
  kind:"d2_document_format_current_qualification",
  version:1,
  key:DocumentFormatDependencyKey/1,
  stamp:{epoch:Token,revision:Counter},
  componentImage:{state:"present",version:Counter,
                  byteLength:Counter,sha256:"64-lowercase-hex"},
  binding:ManagedDocumentFormatBinding/1,
  bindingPin:PinRef/2
}

ManagedDocumentSemanticQualification/1 = {
  sourceObservation:SourceObservation/1,
  documentFormat:DocumentFormatCurrentQualification/1
}
```

BaselineOnly不是binding arm。fresh/copy/fork fresh identity用revision1；formal same-Workspace restore恢复exact historical binding；普通backup重新admission；真实managed profile migration checked +1。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

## 5.1 Dependency family

```text
DependencyKey/3 =
  source(rank0) |
  document_format(rank1) |
  lifecycle(2) | placement_range(3) | ref_inbound(4) |
  relation_incidence(5) | calendar_scope(6) | registry(7) |
  temporal_rules(8) | authorization(9) | foreign_binding(10) |
  query_scan(11) | replica_registry(12) | conflict_record(13) |
  execution_resource(14)
```

除新增document_format外，其余arm字段与fixed-e8aa DependencyKey/2 exact对应arm逐成员相同。

```text
DependencyProof/3 = {
  kind:"d6_dependency_proof",
  version:3,
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  baseFrontier:Frontier/2,
  entries:[{
    key:DependencyKey/3,
    stamp:{epoch:Token,revision:Counter},
    evidencePins:[PinRef/2...]
  }...]
}

InputDescriptor/3 = {
  kind:"d6_input_descriptor",
  version:3,
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  intentKind:text,
  saveProfile:"ordinary"|"complete"|"control_only",
  guarantee:"replica_local"|"managed_atomic",
  expectedFrontier:Frontier/2,
  frontierPolicy:"exact"|"scope_dependencies",
  observationScope:ObservationScope/2,
  sourceInputs:[{
    entityRef:EntityRef,
    observation:SourceObservation/1,
    role:"before"|"dependency"
  }...],
  controlInputs:[{
    key:DependencyKey/3,
    stamp:{epoch:Token,revision:Counter}
  }...],
  ownerInput:OwnerInputBinding/2
}

PreparedIntent/3 = {
  kind:"d6_prepared_intent",
  version:3,
  planToken:Token,
  operationId:Uuid,
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  principalAudienceToken:Token,
  inputDescriptor:InputDescriptor/3,
  beforeCut:Frontier/2,
  proposedState:SemanticState/1,
  mutationFootprint:MutationFootprint,
  dependencyProof:DependencyProof/3,
  observationProof:ObservationProof,
  budgetBinding:BudgetBinding/1,
  pinDirectory:PinDirectory,
  installationPlan:InstallationPlan,
  inputRetentionState:InputRetentionState,
  expiresAt:PreparedDeadline,
  previewBinding:PreviewBinding
}
```

最后六个嵌套名使用fixed-e8aa PreparedIntent/2的exact closed decoder和语义；本successor没有改变它们的成员，只改变Descriptor/Proof family。planToken current tag=d6_plan/3。

## 5.2 Portable component / Notice / CP

```text
PortableComponentKey/2 =
  document(rank0) |
  document_format(rank1) |
  resource(2) | annotation(3) | node_binding(4) |
  child_list(5) | lifecycle(6) | trash_membership(7) |
  policy(8) | registry(9) | period_scope(10) |
  replica_registry(11) | conflict(12)

document_format =
  {kind:"document_format",ownerNodeRef:NodeRef}

InstallationNotice/3 = {
  format:"weftext.installation-notice",
  version:3,
  decisionKey:DecisionKey/2,
  guarantee:"replica_local"|"managed_atomic",
  writeProtection:"strict"|"observed_only",
  baseFrontier:Frontier/2,
  components:[{
    key:PortableComponentKey/2,
    before:ComponentImage/1,
    after:ComponentImage/1
  }...]
}

ContentCompletionProof/4 =
    {format:"weftext.content-completion",version:4,
     outcome:"committed",
     decisionKey:DecisionKey/2,changeId:ChangeId/1,
     guarantee:"replica_local"|"managed_atomic",
     writeProtection:"strict"|"observed_only",
     semanticState:SemanticState/1,
     frontierBefore:Frontier/2,frontierAfter:Frontier/2,
     components:[{key:PortableComponentKey/2,after:ComponentImage/1}...],
     sourceChanges:[{
       entityRef:EntityRef,
       before:SourceVersion/2|"absent",
       after:SourceVersion/2|"absent"
     }...],
     receiptDigest:"sha256:64-lowercase-hex"}
  | {format:"weftext.content-completion",version:4,
     outcome:"restored",
     decisionKey:DecisionKey/2,
     baseFrontier:Frontier/2,
     components:[{key:PortableComponentKey/2,after:ComponentImage/1}...]}
```

ComponentImage/1、PinRef/2 保持 fixed-e8aa exact shape。document_format present component bytes 必须是 D3-CJ/3(binding)，ComponentImage.version=bindingRevision。

Notice3 除 component-key decoder 升为 PortableComponentKey/2 外，逐项继承 Notice2 invariant：components 非空、按 fixed-rank/canonical-key 排序且唯一，notice 在 install 前冻结并保留原 baseFrontier。CP4 除 component-key decoder 与 current ChangeRecord/1 linkage 外，逐项继承 CP3 committed/restored invariant。committed CP4 的 components 与 Notice3 key 集合及顺序严格相同，每个 after 都是实际 installed/sealed image；sourceChanges 是完整、按 EntityRef 排序且唯一的真实 source-state delta；receiptDigest 绑定原 receipt。fresh managed Document 必须在同一 plan/P/CP4 中同时带 document 与 document_format。format-only 且 source 不变时 sourceChanges=[]，不产生 SourceRevisionPlan、managed SourceVersion 或 H advance。

## 5.3 ChangeRecord

```text
ChangeRecord/1 = {
  format:"weftext.change-record",
  version:1,
  decisionKey:DecisionKey/2,
  changeId:ChangeId/1,
  installationNotice:{
    format:"weftext.installation-notice",version:3,
    byteLength:Counter,sha256:"64-lowercase-hex"
  },
  completionProof:{
    format:"weftext.content-completion",version:4,
    byteLength:Counter,sha256:"64-lowercase-hex"
  },
  frontierBefore:Frontier/2,
  frontierAfter:Frontier/2
}
```

CP4 必须 committed 且 DecisionKey/ChangeId/frontiers 与 ChangeRecord 逐项相等；Notice3/CP4 byteLength/digest 必须匹配 exact D3-CJ/3 canonical bytes。同一 P seal 固定 ChangeId、CP4 与 ChangeRecord；publication 只重发原 pin。

frontierBefore 是实际 verified pre-seal Frontier；frontierAfter 必须恰好由 frontierBefore 增加本 ChangeId，其他 domain 不回退。frontierPolicy=exact 时，frontierBefore 与原 expected/base/notice Frontier byte-equal。scope_dependencies 时，还必须从 Notice3.baseFrontier 到 frontierBefore 再到 frontierAfter 证明完整连续、已验证的 ChangeRecord/completion chain，并保留原 unrelatedness proof；vector number、provider sync 状态或当前文件均不能替代。

restored CP4 只允许 decisionKey、baseFrontier，以及每项都等于对应 Notice3 before image 的 component after images；禁止 changeId、guarantee、writeProtection、semanticState、frontierBefore、frontierAfter、sourceChanges、receiptDigest 和全部成功语义。

receiver admission 必须 strict-decode exact Notice3/CP4 version 与 canonical bytes，验证 component key 集合/顺序严格一致，取得并验证每个实际 component byte 与 owner version，验证完整 production SourceVersion changes，并证明连续 chain。present document_format component 还必须 strict-decode exact ManagedDocumentFormatBinding/1 bytes。缺失/未知 bytes 或 decoder、component-version mismatch、Notice/CP/ChangeRecord mismatch 都只能是 incomplete/proof_unavailable，不能成功 admission。

# 6. D3/D7/D8/D9 current direct holders

```text
D3ResolutionInputUse/2 = {
  kind:"d3_resolution_input_use",
  version:2,
  decisionKey:DecisionKey/2,
  principalAudienceToken:Token,
  bindingToken:Token,
  inputDescriptor:InputDescriptor/3
}
```

D3 current native wire=13，D3IdentityInput/13 的原wire12 complete member set不减少；所有嵌套 InputDescriptor/2 位置改为/3，current effects family改为EffectManifest/3。历史wire9..12保持。

## 6.1 PreparedActionBinding/4

```text
PreparedActionBinding/4 = {
  kind:"d7_prepared_action_binding",
  version:4,
  bindingToken:Token,
  protocolOwner:"D3"|"D6",
  operationId:UUIDv4,
  workspaceRef:WorkspaceRef,
  principalAudienceToken:Token,
  action:ActionSpec,
  canonicalCallInputs:[QueryCall...],
  definitionInputs:[D7DefinitionInput/2...],
  registryInputs:[ValidatedCatalogContext...],
  ruleInputs:[RecurrenceReadContext...],
  sourceInputs:[{
    entityRef:EntityRef,
    observation:SourceObservation/1,
    role:"before"|"dependency"
  }...],
  constructionInput:null|TemplateConstructionInput/2,
  proposedInputs:[D7ProposedInput/2...],
  dependencyProof:DependencyProof/3,
  observationProof:<PreparedIntent/3.observationProof>,
  budgetBinding:BudgetBinding/1,
  expiresAt:<D6 protected deadline>,
  request:<D3 wire13 | d6_commit_request/2>,
  preview:<complete immutable EffectManifest/3 semantics,pins,initial delivery>,
  resolutionInput:null|D3ResolutionInput/1
}
```

MinimumMapping/3、D7DefinitionInput/2、D7ProposedInput/2、D7ResolutionAccess/1保持fixed-e8aa exact shape。

## 6.2 EffectManifest/3 / EffectBytes/3

```text
EffectManifest/3 = {
  format:"weftext.effects",
  version:3,
  phase:"preview"|"committed",
  protocolOwner:"D3"|"D6",
  operationId:UUIDv4,
  workspaceRef:WorkspaceRef,
  profile:"full"|"owner_fields",
  items:[EffectItem/3...],
  decisionKey:DecisionKey/2
}

EffectBytes/3 = {
  handleToken:Token,
  encoding:
    "exact_source_utf8"|"resource_bytes"|"d3_annotation_value4"|
    "d3_symbolic_result9"|"d4_relation_copy_effects1"|
    "d4_source_materialization_effects1"|
    "d7_definition_transfer_effects2"|"d3_canonical_effects1"|
    "d6_workspace_bootstrap_plan1"|"d6_workspace_bootstrap_plan3"|
    "d6_workspace_bootstrap_plan4"|"d7_symbolic_json3"|"field_entries2",
  byteLength:Counter
}
```

EffectItem/3 闭集与 fixed-e8aa EffectItem/2 的十四种语义分支一一对应：
source_change, conditional_source_change, entity_state_change, d3_plan, d3_receipt, semantic_extension, period_scope_change, series_configuration_change, workspace_bootstrap, authority_change, field_change, conflict_branch_source, conflict_resolution_change, canonical_plan；其中全部 EffectBytes 字段使用 /3，Annotation source image 使用 Value/4，workspace_bootstrap 现行路径允许 Plan4，不得出现 generic_json/free payload。以上英文名称均为协议标识、字段名或固定字面量，必须严格按本设计的闭合集逐项解释，不改变本句中文语义。

## 6.3 PreparedEditBinding/3

```text
PreparedEditBinding/3 = {
  kind:"d8_prepared_edit_binding",
  version:3,
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  operationId:UUIDv4,
  principalAudienceToken:Token,
  inputDescriptor:InputDescriptor/3,
  intent:<D8 current closed edit intent>,
  origin:{kind:"direct"}|{kind:"undo",originalRequest:<OriginalD6Request>},
  sourceInputs:<InputDescriptor/3.sourceInputs>,
  proposedInputs:[{entityRef:EntityRef,pin:PinRef/2}],
  registryInputs:[ValidatedCatalogContext...],
  dependencyProof:DependencyProof/3,
  observationProof:<PreparedIntent/3.observationProof>,
  budgetBinding:BudgetBinding/1,
  expiresAt:<D6 protected deadline>,
  request:d6_commit_request/2,
  preview:<complete EffectManifest/3>
}
```

proposedInputs恰一项且entityRef等于intent.target.ref；OwnerInputBinding/2保持，current ownerKind=d8_edit/3。

## 6.4 ExportPlan/3 / PublicationReceipt/3

ExportPlan/3 是protected Core record，必须同时冻结下列**全部**语义成员；未列项不得由实现自由省略：

```text
ExportPlan/3 = {
  kind:"d9_export_plan",
  version:3,
  planToken:Token,
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  principalAudienceToken:Token,
  authorizationGeneration:Token,
  inputCatalog:ExportInputCatalog/2,
  contentSelection:ExportContentSelection/1,
  projection:ExportProjection/1,
  templateBinding:null|<exact template bytes/version binding>,
  routeBinding:<exact route/profile/version binding>,
  targetFormat:<D9 closed export target>,
  initialLossReport:ExportLossReport/1,
  outputBudget:BudgetBinding/1,
  destinationIntent:<protected destination intent>,
  observationScope:ObservationScope/2,
  dependencyProof:DependencyProof/3,
  observationProof:<PreparedIntent/3.observationProof>,
  sourceRuleRoutePins:[PinRef/2...],
  stagedOutputs:[{name:text,byteLength:Counter,
                  sha256:"sha256:<64 lowercase hex>",pin:PinRef/2}...]
}
```

PublicationReceipt/3：

```text
PublicationReceipt/3 = {
  kind:"d9_publication_receipt",
  version:3,
  publicationToken:Token,
  planToken:Token,
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  outputs:[{name:text,byteLength:Counter,
            sha256:"sha256:<64 lowercase hex>"}...],
  lossReport:ExportLossReport/1,
  lossChoices:[<exact original confirmation choice>...],
  routeProfileStyleVersions:[<exact bound version>...],
  destinationDisplay:text
}
```

它只证明external publication，不是D6 author receipt。历史Export/Publication /1,/2按原decoder恢复。

# 7. Annotation closed values

```text
D3-Annotation-Value/4 = {
  kind:"d3_annotation_value",
  version:4,
  purpose:"comment"|"mark"|"suggestion",
  target:D3-Annotation-Target-Projection/1,
  replyTo:AnnotationRef|null,
  body:AnnotationInlineBody/1|null,
  appearance:AnnotationAppearance/1|null,
  labels:[text...],
  reviewState:"open"|"resolved"|"not_applicable",
  suggestion:Suggestion/3|null,
  creator:AnnotationActorSnapshot/1,
  authoredAt:AnnotationTimeSnapshot/2,
  lastEditor:AnnotationActorSnapshot/1,
  editedAt:AnnotationTimeSnapshot/2
}

D3-Annotation-Target-Projection/1 =
    {kind:"document",owner:NodeRef}
  | {kind:"document_element",locator:DocumentElementLocator}
  | {kind:"document_range",locator:DocumentRangeLocator}
  | {kind:"resource",resourceRef:ResourceRef}
  | {kind:"resource_region",locator:ResourceRegionLocator}

AnnotationInlineBody/1 = {
  format:"asciidoc-inline",
  version:1,
  languageBaseline:"asciidoctor-ruby/2.0.26",
  source:text
}

AsciiDocInlineBody/1 := AnnotationInlineBody/1

AnnotationInlineProfile/1 = {
  languageBaseline:
    "asciidoctor-ruby/2.0.26@0b99b39c9df884d4aec13bba45f03cdbab505769",
  doctype:"inline",
  processorBackend:"html5-semantic/1",
  safeMode:"secure",
  maxSourceBytes:65536,
  maxRenderedBytes:262144,
  maxInlineSemanticNodes:4096,
  managedAdapters:"disabled",
  networkEffects:"denied",
  fileEffects:"denied",
  processEffects:"denied"
}

AnnotationAppearance/1 = {
  mark:"highlight"|"underline"|"squiggle"|"strike",
  theme:"yellow"|"red"|"green"|"blue"|"purple"|"pink"|"gray"
}

AnnotationActorSnapshot/1 = {
  kind:"annotation_actor_snapshot",
  version:1,
  originWorkspaceRef:WorkspaceRef,
  authentication:"workspace_authenticated_origin"|
                 "device_local_unverified"|"imported_unverified",
  displayName:text
}

AnnotationTimeSnapshot/2 = {
  instant:RFC3339-with-offset,
  producer:"prepare_server_clock"|"prepare_device_clock"|"imported_unverified"
}

Suggestion/3 = {
  version:3,
  kind:"replace"|"delete"|"insert",
  state:"pending"|"accepted"|"rejected",
  confirmation:"confirmed"|"needs_reconfirmation"|"not_applicable",
  targetBasisSha256:"sha256:<64 lowercase hex>",
  expectedText:null|{byteLength:Counter,sha256:"sha256:<64 lowercase hex>"},
  pointAffinity:null|"left"|"right",
  replacementSource:null|text
}
```

`AnnotationInlineBody/1` 是current唯一名称并保留早期 `AsciiDocInlineBody/1` 的四成员数据shape；`AsciiDocInlineBody/1` 仅是**schema alias**，其 canonical bytes 与 `AnnotationInlineBody/1` 完全相同，不形成第二wire/version，也不声称历史部署。body只是portable source value，不能承载processor环境。求值必须使用同一节唯一 `AnnotationInlineProfile/1`；body的 `languageBaseline="asciidoctor-ruby/2.0.26"` 必须与profile所固定的2.0.26@commit版本一致。完整source必须只形成一个paragraph（允许soft wraps与trailing whitespace）；第二paragraph、heading、list、delimited block、table或block macro为 `invalid_annotation_body`，不能静默忽略。

replyTo非null强制same-owner、acyclic、purpose=comment、suggestion=null、reviewState=not_applicable。root reviewState只能open|resolved。pending confirmation只能confirmed|needs_reconfirmation；accepted/rejected terminal且confirmation=not_applicable。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

# 8. SourceTransform

```text
PortableTransformCompilation/1 =
    {kind:"representable",
     events:[SourceTransformPortableEvent/3...],
     afterSourceSha256:"sha256:<64 lowercase hex>"}
  | {kind:"unavailable",
     reason:"provenance_gap"|"unsupported_transaction"|
            "invalid_utf8_boundary"|"generated_cross_anchor_edit"|
            "boundary_slot_unrepresentable"|"provenance_cycle"|
            "after_replay_mismatch"|"payload_digest_mismatch"}

SourceTransformPortableEvent/3 =
    {kind:"replace",
     startByte:Counter,endByte:Counter,
     removedByteLength:Counter,
     removedSha256:"sha256:<64 lowercase hex>",
     replacementByteLength:Counter,
     replacementSha256:"sha256:<64 lowercase hex>"}
  | {kind:"insert",
     atByte:Counter,
     replacementByteLength:Counter,
     replacementSha256:"sha256:<64 lowercase hex>"}

TransformEmissionPlan/1 =
    {kind:"disabled",
     reason:"no_exact_core_edit_plan"|"transform_profile_unavailable"}
  | {kind:"required",
     profile:"d6_source_transform_seal/1",
     expectedTrustRevision:Counter,
     expectedTrustKeyId:"sha256:<64 lowercase hex>"}

CoreSourceEditPlan/2 = {
  kind:"d6_core_source_edit_plan",
  version:2,
  decisionKey:DecisionKey/2,
  ownerNodeRef:NodeRef,
  beforeObservation:SourceObservation/1,
  coordinateProfile:"utf8-byte-half-open/1",
  edits:[SourceTransformPortableEvent/3...],
  afterPin:PinRef/2,
  transformEmission:TransformEmissionPlan/1
}

SourceTransformEvidence/2 = {
  kind:"d6_source_transform_evidence",
  version:2,
  decisionKey:DecisionKey/2,
  changeId:ChangeId/1,
  ownerNodeRef:NodeRef,
  before:SourceVersion/2,
  after:SourceVersion/2,
  beforeSourceSha256:"sha256:<64 lowercase hex>",
  afterSourceSha256:"sha256:<64 lowercase hex>",
  coordinateProfile:"utf8-byte-half-open/1",
  affinityProfile:"annotation-range-affinity/1",
  edits:[SourceTransformPortableEvent/3...]
}

SourceTransformSealSignedBody/1 = {
  format:"weftext.source-transform-seal",
  version:1,
  trustKeyId:"sha256:<64 lowercase hex>",
  evidence:SourceTransformEvidence/2
}

SourceTransformSealArtifact/1 = {
  format:"weftext.source-transform-seal",
  version:1,
  trustKeyId:"sha256:<64 lowercase hex>",
  evidence:SourceTransformEvidence/2,
  signature:"<86 ASCII unpadded base64url>"
}

SourceTransformSealKey/1 = {
  changeId:ChangeId/1,
  ownerNodeRef:NodeRef
}

SourceTransformSealOutboxItem/1 = {
  kind:"d6_source_transform_seal_outbox_item",
  version:1,
  key:SourceTransformSealKey/1,
  artifactPin:PinRef/2
}
```

plan/evidence edits 的 D3-CJ/3 必须 byte-equal；seal 不得重编译、重排或 merge。required sealed decision 恰一个 outbox item，disabled 为零；artifactPin 为 portable_metadata exact artifact canonical bytes。

Event3 必须针对 exact beforeBytes/afterBytes 做 closed 校验。replace 要求 0<=startByte<endByte<=before length、两端均为 UTF-8 scalar boundary、removedByteLength=endByte-startByte，且 removedSha256=SHA-256(beforeBytes[startByte:endByte])；insert 要求 0<=atByte<=before length、atByte 为 UTF-8 scalar boundary、replacementByteLength 非零。replacement interval 必须是两两不重叠的 maximal islands；same-point inserts 按 transaction order 合并；严格位于 replacement island 内的 insert 必须折入 island 或 compilation unavailable。canonical boundary order 为 left replacement ending p、insert@p、right replacement starting p。

generatedOutputSpan 使用 SPEC §11.1 的 replay-cursor 算法处理 exact before/after pins：对每段 unchanged gap 做 byte-compare；当前 event span 恰为 [afterCursor,afterCursor+replacementByteLength)，该 after slice 验 replacement length/hash；replace 将 beforeCursor 推到 endByte，insert 保持在 q；最后 tails 与 afterSourceSha256 都必须匹配。禁止内容搜索。mapping 定义 delta(replace)=replacementByteLength-(endByte-startByte)，并使用 checked signed arithmetic。

SourceTransformSealSignedBody/1 恰为 SourceTransformSealArtifact/1 只删除 signature。signature 必须恰为 86 个 ASCII unpadded-base64url 字符并 decode 为 64-byte Ed25519 signature。待签消息恰为 ASCII "D6-Source-Transform-Seal/1" || NUL || D3-CJ/3(SourceTransformSealSignedBody/1)。完整 transport/storage artifact bytes 恰为含 signature 的 SourceTransformSealArtifact/1 的 D3-CJ/3；alternate JSON serialization 必须拒绝。

# 9. D6 dual-profile trust

```text
SealProfileId/1 =
  "d6_revision_token_seal/1" |
  "d6_source_transform_seal/1"

WorkspaceTrustPredecessor/2 =
    {kind:"root",fingerprint:WorkspaceTrustRootFingerprint/1}
  | {kind:"declaration",revision:Counter,
     sha256:"sha256:<64 lowercase hex>"}

WorkspaceTrustDeclaration/2 = {
  kind:"d6_workspace_trust_declaration",
  version:2,
  workspaceRef:WorkspaceRef,
  revision:Counter,
  predecessor:WorkspaceTrustPredecessor/2,
  decisionKey:DecisionKey/2,
  action:
      {kind:"authorize",commitDomain:CommitDomain/2,
       profile:SealProfileId/1,trustKeyId:"sha256:<64 lowercase hex>",
       algorithm:"ed25519",publicKey:"<43 ASCII unpadded base64url>",
       possessionSignature:"<86 ASCII unpadded base64url>"}
    | {kind:"rotate",commitDomain:CommitDomain/2,
       profile:SealProfileId/1,
       oldTrustKeyId:"sha256:<64 lowercase hex>",
       newTrustKeyId:"sha256:<64 lowercase hex>",
       algorithm:"ed25519",publicKey:"<43 ASCII unpadded base64url>",
       possessionSignature:"<86 ASCII unpadded base64url>",
       continuitySignature:"<86 ASCII unpadded base64url>"|"not_required",
       mode:"ordinary"|"loss_recovery"|"compromise"}
    | {kind:"revoke",commitDomain:CommitDomain/2,
       profile:SealProfileId/1,
       trustKeyId:"sha256:<64 lowercase hex>",
       mode:"administrative"|"loss"|"compromise"}
    | {kind:"resolve_conflict",
       outcomes:[TrustConflictOutcome/2...],
       inheritedCompromises:[TrustConflictCarry/1|TrustConflictCarry/2...]},
  rootSignature:"<86 ASCII unpadded base64url>"
}

DomainSealKeyPoPBody/2 = {
  workspaceRef:WorkspaceRef,revision:Counter,
  predecessor:WorkspaceTrustPredecessor/2,decisionKey:DecisionKey/2,
  commitDomain:CommitDomain/2,profile:SealProfileId/1,
  trustKeyId:"sha256:<64 lowercase hex>",algorithm:"ed25519",
  publicKey:"<43 ASCII unpadded base64url>"
}

DomainSealKeyRotateContinuityBody/2 = {
  workspaceRef:WorkspaceRef,revision:Counter,
  predecessor:WorkspaceTrustPredecessor/2,decisionKey:DecisionKey/2,
  commitDomain:CommitDomain/2,profile:SealProfileId/1,
  oldTrustKeyId:"sha256:<64 lowercase hex>",
  newTrustKeyId:"sha256:<64 lowercase hex>",algorithm:"ed25519",
  publicKey:"<43 ASCII unpadded base64url>",
  possessionSignature:"<86 ASCII unpadded base64url>",mode:"ordinary"
}

WorkspaceTrustDeclarationSignedBody/2 :=
  WorkspaceTrustDeclaration/2 只删除 rootSignature

WorkspaceAuthorizationBundle/2 = {
  kind:"d6_workspace_authorization_bundle",
  version:2,
  workspaceRef:WorkspaceRef,
  authorizationRevision:Counter,
  policy:Policy/3,
  trustRoot:WorkspaceTrustRootDeclaration/1,
  trustRevision:Counter,
  trustDeclarations:[WorkspaceTrustDeclaration/1|WorkspaceTrustDeclaration/2...]
}

DomainSealKeyHandle/2 = {
  kind:"d6_domain_seal_key_handle",
  version:2,
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  profile:SealProfileId/1,
  trustKeyId:"sha256:<64 lowercase hex>",
  secureHandle:Token,
  state:"staged"|"usable"|"retired"|"lost"
}

SourceTransformSealVerificationKey/1 = {
  kind:"d6_source_transform_seal_verification_key",
  version:1,
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  profile:"d6_source_transform_seal/1",
  trustKeyId:"sha256:<64 lowercase hex>",
  algorithm:"ed25519",
  publicKey:"<43 ASCII unpadded base64url>"
}

TrustConflictCarry/2 = {
  kind:"d6_trust_conflict_carry",
  version:2,
  factId:"sha256:<64 lowercase hex>",
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  profile:SealProfileId/1,
  compromisedTrustKeyId:"sha256:<64 lowercase hex>",
  originAction:"revoke"|"rotate",
  originDecisionKey:DecisionKey/2,
  originDeclarationRevision:Counter,
  originDeclarationDigest:"sha256:<64 lowercase hex>",
  originActivationChangeId:ChangeId/1
}

TrustConflictOutcome/2 =
    {commitDomain:CommitDomain/2,profile:SealProfileId/1,
     state:"keep_current",trustKeyId:"sha256:<64 lowercase hex>"}
  | {commitDomain:CommitDomain/2,profile:SealProfileId/1,state:"none"}
  | {commitDomain:CommitDomain/2,profile:SealProfileId/1,
     state:"authorize_fresh",trustKeyId:"sha256:<64 lowercase hex>",
     algorithm:"ed25519",publicKey:"<43 ASCII unpadded base64url>",
     possessionSignature:"<86 ASCII unpadded base64url>"}

DomainSealKeyAddPrepare/3 = {
  wireVersion:3,kind:"d6_domain_seal_key_add_prepare",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  profile:SealProfileId/1,expectedTrustRevision:Counter,
  budget:BudgetBinding/1
}

DomainSealKeyRotatePrepare/3 = {
  wireVersion:3,kind:"d6_domain_seal_key_rotate_prepare",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  profile:SealProfileId/1,expectedTrustRevision:Counter,
  expectedTrustKeyId:"sha256:<64 lowercase hex>",
  mode:"ordinary"|"loss_recovery"|"compromise",
  budget:BudgetBinding/1
}

DomainSealKeyRevokePrepare/3 = {
  wireVersion:3,kind:"d6_domain_seal_key_revoke_prepare",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  profile:SealProfileId/1,expectedTrustRevision:Counter,
  expectedTrustKeyId:"sha256:<64 lowercase hex>",
  mode:"administrative"|"loss"|"compromise",
  budget:BudgetBinding/1
}
```

每个 publicKey 必须 decode 为 exact 32-byte Ed25519 key，并 hash 到对应 trustKeyId；每个 signature lexical value 必须 decode 为 exact 64-byte Ed25519 signature。authorize/rotate 的 possessionSignature 签 exact ASCII "D6-Domain-Seal-Key-PoP/2" || NUL || D3-CJ/3(DomainSealKeyPoPBody/2)，rotate 将 newTrustKeyId 映射到 PoP body 的 trustKeyId。authorize_fresh 用 enclosing Declaration2 的 workspaceRef/revision/predecessor/decisionKey 加该 exact outcome 的 commitDomain/profile/key tuple 构造同一 body。ordinary rotate 的 continuitySignature 签 exact ASCII "D6-Domain-Seal-Key-Rotate/2" || NUL || D3-CJ/3(DomainSealKeyRotateContinuityBody/2)；loss_recovery/compromise 必须是 literal "not_required"。rootSignature 签 exact ASCII "D6-Workspace-Trust-Declaration/2" || NUL || D3-CJ/3(WorkspaceTrustDeclarationSignedBody/2)。

revision-1 root predecessor 的 fingerprint 是完整 WorkspaceTrustRootFingerprint/1，并与 retained root/anchor fingerprint byte-equal；rootKeyId 不能替代。Declaration/1 永远只授权 revision-token profile。Bundle2 允许 Declaration1 历史 prefix，出现首个 Declaration2 后后继都只能 /2。Declaration1 activation 继续用 CP3；Declaration2 activation 由同 DecisionKey ChangeRecord1 -> exact CP4 -> policy after-image -> Bundle2 重派生。

current unseen 普通 trust management 只使用上述 wireVersion3 prepare successors。它们不携带 caller key material，保留 fixed-parent policy_admin/root-handle/current-state gate 与原 planning/install/single-P 路径，只把 current completion family 接到 CP4。saved/planned wireVersion2 record 保留原 decoder/recovery。

```text
WorkspaceTrustGenesis/2 = {
  kind:"d6_workspace_trust_genesis",version:2,
  rootDeclaration:WorkspaceTrustRootDeclaration/1,
  initialDomainDeclarations:[
    WorkspaceTrustDeclaration/2,
    WorkspaceTrustDeclaration/2
  ]
}

WorkspaceBootstrapProfile/4 = {
  kind:"d6_bootstrap_profile",wireVersion:4,
  profileRevision:Counter,
  registrySeedBinding:RegistryBinding/1,
  newSeriesMultiplicity:"unique"|"many",
  initialPeriodScope:"workspace"
}

WorkspaceBootstrapCreatorBinding/1 = {
  issuerPrincipal:Token,
  targetPrincipal:Token,
  principalAudienceToken:Token
}

WorkspaceBootstrapTargetRegistry/1 = {
  snapshot:RegistrySnapshot/1,
  binding:RegistryBinding/1
}

WorkspaceBootstrapSeriesConfiguration/1 = {
  seriesScope:SeriesScope,
  multiplicity:"unique"|"many",
  revision:1
}

WorkspaceBootstrapPeriodScopeBinding/1 = {
  nodeRef:NodeRef,
  scope:CalendarScope,
  revision:1
}

WorkspaceBootstrapPlan/4 = {
  kind:"d6_workspace_bootstrap_plan",
  wireVersion:4,
  operationId:Uuid,
  proposalId:Uuid,
  issuerAuthorityInstanceId:Uuid,
  targetWorkspaceRef:WorkspaceRef,
  targetAuthorityInstanceId:Uuid,
  profile:WorkspaceBootstrapProfile/4,
  creatorBinding:WorkspaceBootstrapCreatorBinding/1,
  targetRegistry:WorkspaceBootstrapTargetRegistry/1,
  initialPolicy:Policy/3,
  trustGenesis:WorkspaceTrustGenesis/2,
  initialSeriesConfigurations:[WorkspaceBootstrapSeriesConfiguration/1...],
  periodScopeBindings:[WorkspaceBootstrapPeriodScopeBinding/1...]
}
```

全部 UUID 成员使用 D3 canonical lowercase UUID decoder。Genesis 恰两项：revision 1 revision-token authorize，revision 2 source-transform authorize；同 DecisionKey/activation ChangeId，rev2 predecessor hash exact rev1 canonical bytes。series configurations 按 canonical SeriesScope 排序且唯一；period bindings 按完整 NodeRef 排序且唯一，并且每个 valid prepared period 恰一项。helper members 保留 fixed-parent Plan3 字段语义；Profile4 只改变 dual-profile genesis family。

Plan4 只用于 unseen fresh create/fork，并保留原 D3 proposal/custody/CAS/P boundary。普通 copy 不使用 Plan4。saved/planned/unknown recovery 保留实际 recorded decoder/bytes；restore/continue/failover 不合成 Genesis2。本候选不声称 Plan3 已部署，也不虚构 migration。

legacy Handle1 只在 exact Declaration1-authorized key 仍 current、safe、usable 时继续 revision-token 新签名；永远不能签 transform。continuation 对两个 profile 分别求 current(K)|none|conflicted_or_unproved，任一 unproved 都禁止部分激活新 domain。declaration order 固定为 optional old revision revoke、optional old transform revoke、new revision authorize、new transform authorize，全部在同一 DecisionKey/CP4 且无可观察中间 prefix。

# 10. D10 mixed-version current outer schemas

```text
D10WorkspaceReadDependencies/2 = {
  kind:"d10_workspace_read_dependencies",
  version:2,
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  observationScope:ObservationScope/2,
  sourceInputs:[{entityRef:EntityRef,
                 observation:SourceObservation/1,
                 role:"before"|"dependency"}...],
  dependencyProof:DependencyProof/3,
  registryReads:[{binding:RegistryBinding/1,
                  snapshot:RegistrySnapshot/1,
                  snapshotPin:PinRef/2}...],
  evidencePins:[PinRef/2...]
}

D10VersionedControlRecordImage/1 =
    {schema:"d10_control_record_image/1",value:D10ControlRecordImage/1}
  | {schema:"d10_control_record_image/2",value:D10ControlRecordImage/2}

D10ControlRecordImage/2 = {
  kind:"d10_control_record_image",
  version:2,
  binding:Binding<K>/1,
  scope:Scope/1,
  usageRevision:Option<Counter>,
  view:ControlCurrentView<K>/1,
  supplement:
      {kind:"automation",subscription:ScheduleSubscription/2,
       stop:Binding<stop>/1,creator:Token}
    | {kind:"run",createdBinding:Binding<run>/1,
       principal:Token,fixedBudget:BudgetCaps/1,
       invocation:Option<AutomationInvocation/1>,
       admission:Option<LeaseRunUse/1>,
       authorSteps:[D10VersionedAuthorStepResponsibility/1...]}
}
```

Image2只允许automation/run。其它record kinds（包括planned_approval、external_approval、activation、reservation、external_effect、stop及none）继续exact Image1。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

```text
D10ControlRecordPin/2 = {
  image:D10ControlRecordImage/2,
  pin:PinRef/2
}

D10VersionedControlRecordPin/1 =
    {schema:"d10_control_record_pin/1",value:D10ControlRecordPin/1}
  | {schema:"d10_control_record_pin/2",value:D10ControlRecordPin/2}

D10ControlRange/2 =
    {kind:"records",version:2,scope:Scope/1,
     kinds:[ControlRecordKind/1],epoch:Token,revision:Counter,
     members:[ControlRef<K>/1]}
  | {kind:"cost_lineage",version:2,key:CostLayerKey/1,
     epoch:Token,revision:Counter,
     reservations:[ControlRef<reservation>/1]}
  | {kind:"occurrences",version:2,
     automation:ControlRef<automation>/1,
     epoch:Token,revision:Counter,
     records:[D10VersionedAutomationOccurrenceRecord/1...]}

D10VersionedControlRange/1 =
    {schema:"d10_control_range/1",value:D10ControlRange/1}
  | {schema:"d10_control_range/2",value:D10ControlRange/2}

D10VersionedControlPrepareBinding/1 =
    {schema:"d10_control_prepare_binding/1",value:ControlPrepareBinding/1}
  | {schema:"d10_control_prepare_binding/2",value:ControlPrepareBinding/2}
  | {schema:"d10_control_prepare_binding/3",value:ControlPrepareBinding/3}

ControlPrepareBinding/1 = {
  key:StableControlKey/1,
  canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  originalCommitRequest:PreparedCommitRequest/1,
  immutablePreview:ControlPreview/1,
  dependencyPins:ControlDependencies/1
}

ControlPrepareBinding/2 = {
  key:StableControlKey/1,
  canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  originalCommitRequest:PreparedCommitRequest/1,
  immutablePreview:ControlPreview/1,
  confirmationRequirement:ExternalConfirmationRequirement/1,
  dependencyPins:ControlDependencies/2
}

ControlPrepareBinding/3 = {
  kind:"d10_control_prepare_binding",
  version:3,
  key:StableControlKey/1,
  canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  originalCommitRequest:PreparedCommitRequest/1,
  immutablePreview:ControlPreview/1,
  confirmationRequirement:ExternalConfirmationRequirement/1,
  dependencyPins:ControlDependencies/3
}
```

ControlDependencies/3：

```text
{
  kind:"d10_control_dependencies",
  version:3,
  configBindings:[Binding<K>/1],
  usageBindings:[{
    ref:ControlRef<lease|approval|grant|cost_account>/1,
    usageRevision:Counter
  }...],
  authorityProof:Token,
  authorizationGenerations:[Token...],
  stopRefs:[ControlRef<stop>/1...],
  workspaceReads:Option<D10WorkspaceReadDependencies/2>,
  recordPins:[D10VersionedControlRecordPin/1...],
  controlRanges:[D10VersionedControlRange/1...]
}
```

D10ControlEffectPlan/2：

```text
{
  kind:"d10_control_effect_plan",
  version:2,
  changes:[{
    before:Option<D10VersionedControlRecordImage/1>,
    after:D10VersionedControlRecordImage/1
  }...],
  activationSelector:Option<{
    before:D10ActivationSelector/1,
    after:D10ActivationSelector/1
  }>,
  recordPins:[D10VersionedControlRecordPin/1...],
  registryChange:Option<{
    before:{snapshot:RegistrySnapshot/1,binding:RegistryBinding/1},
    after:{snapshot:RegistrySnapshot/1,binding:RegistryBinding/1},
    evolution:RegistryEvolutionProof/1,
    beforePin:PinRef/2,afterPin:PinRef/2
  }>
}
```

old Image1 automation/run → new Image2是合法current configure；old before不得重编码为Image2。

```text
D10VersionedAuthorStepResponsibility/1 =
    {schema:"d10_author_step_responsibility/1",
     value:D10AuthorStepResponsibility/1}
  | {schema:"d10_author_step_responsibility/2",
     value:D10AuthorStepResponsibility/2}

D10VersionedScheduleSubscription/1 =
    {schema:"d10_schedule_subscription/1",value:ScheduleSubscription/1}
  | {schema:"d10_schedule_subscription/2",value:ScheduleSubscription/2}

D10VersionedAutomationOccurrenceRecord/1 =
    {schema:"d10_automation_occurrence_record/1",
     value:AutomationOccurrenceRecord/1}
  | {schema:"d10_automation_occurrence_record/2",
     value:AutomationOccurrenceRecord/2}

D10VersionedApprovalUse/1 =
    {schema:"d10_approval_use/1",value:ApprovalUse/1}
  | {schema:"d10_approval_use/2",value:ApprovalUse/2}
```

```text
D10ExecutionClaims/2 = {
  kind:"d10_execution_claims",
  version:2,
  recordPins:[D10VersionedControlRecordPin/1...],
  prepareBindings:[D10VersionedControlPrepareBinding/1...],
  leaseRuns:[LeaseRunUse/1...],
  authorSteps:[D10VersionedAuthorStepResponsibility/1...],
  subscriptions:[D10VersionedScheduleSubscription/1...],
  occurrenceRecords:[D10VersionedAutomationOccurrenceRecord/1...],
  ranges:[D10VersionedControlRange/1...],
  continuityPins:[PinRef/2...]
}

D10MoneyResponsibility/2 = {
  kind:"d10_money_responsibility",
  version:2,
  reservations:[{
    binding:Binding<reservation>/1,
    value:CostReservation/1,
    attribution:CostBudgetAttribution/1,
    settlements:[CostSettlementDecision/1...]
  }...],
  layers:[CostLayerTotal/1...],
  recordPins:[D10VersionedControlRecordPin/1...],
  ranges:[D10VersionedControlRange/1...],
  evidencePins:[PinRef/2...]
}

D10ExecutionInventory/2 = {
  kind:"d10_execution_inventory",
  version:2,
  workspaceRef:WorkspaceRef,
  storeIncarnation:Uuid,
  approvalUses:[D10VersionedApprovalUse/1...],
  claims:D10ExecutionClaims/2,
  moneyLineage:D10MoneyResponsibility/2,
  externalUnknowns:[D10ExternalResponsibility/1...],
  stopState:[D10StopResponsibility/1...],
  stopCapacity:StopCapacity/1
}
```

prepareBindings统一按inner StableControlKey canonical bytes排序唯一，跨版本同key拒绝。其它mixed arrays按SPEC §14的semantic identity跨版本唯一，不能LWW。

D6 current direct holder：

```text
ExecutionResponsibilityRecord/3 = {
  kind:"d6_execution_responsibility",
  version:3,
  workspaceRef:WorkspaceRef,
  executionDomainId:Uuid,
  holder:
      {kind:"local_replica",replicaEpoch:Uuid}
    | {kind:"server",authorityInstanceId:Uuid,deploymentId:Uuid},
  revision:Counter,
  status:"active"|"paused"|"transferring",
  approvalUses:[D10VersionedApprovalUse/1...],
  claims:D10ExecutionClaims/2,
  moneyLineage:D10MoneyResponsibility/2,
  externalUnknowns:[D10ExternalResponsibility/1...],
  stopState:[D10StopResponsibility/1...],
  lastContinuityProof:ExecutionContinuityProof/2
}

ExecutionContinuityProof/2 =
    {kind:"initial",storeIncarnation:Uuid,
     inventoryPin:PinRef/2,birthProofToken:Token}
  | {kind:"checkpoint",storeIncarnation:Uuid,
     inventoryPin:PinRef/2,barrierToken:Token}
  | {kind:"handoff",storeIncarnation:Uuid,
     inventoryPin:PinRef/2,fromHolder:ExecutionHolder,
     toHolder:ExecutionHolder,oldRevision:Counter,
     barrierToken:Token,oldHolderFenceToken:Token}
```

inventoryPin exact payload domain：
UTF8("D6-Execution-Inventory/2") || NUL || D3-CJ/3(D10ExecutionInventory/2)。
Record2/Proof1/Inventory1历史domain不变；只有真实responsibility mutation/checkpoint/handoff才形成Record3。

## 10.1 D10 current schedule / author-step 直接类型

以下类型是 §10 mixed wrappers 所引用的 current exact inner values；不是 wrapper 自行发明的自由对象。

```text
D10ControlInput/2 = {
  kind:"d10_control",
  version:2,
  key:StableControlKey/1,
  operationId:Uuid,
  canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  preview:ControlPreview/1,
  dependencies:ControlDependencies/3,
  effectPlan:D10ControlEffectPlan/2,
  confirmationRequirement:ExternalConfirmationRequirement/1
}

ScheduleRecurrenceEvidence/2 = {
  kind:"d10_schedule_recurrence_evidence",
  version:2,
  observation:SourceObservation/1,
  sourcePin:PinRef/2,
  metadataPin:PinRef/2,
  dependencyProof:DependencyProof/3,
  registrySnapshot:RegistrySnapshot/1,
  registryEvolution:Option<RegistryEvolutionProof/1>,
  recurrenceContext:RecurrenceReadContext/1
}

ScheduleSourceBinding/2 =
    {kind:"once",atUtcSeconds:CanonicalDecimal}
  | {kind:"recurrence",
     ownerNodeRef:NodeRef,
     recurrenceOccurrenceKey:occurrenceKey,
     rangeOccurrenceKey:occurrenceKey,
     initial:ScheduleRecurrenceEvidence/2,
     checkpoint:ScheduleRecurrenceEvidence/2,
     continuityPins:[PinRef/2...]}

ScheduleSubscription/2 = {
  kind:"d10_schedule_subscription",
  version:2,
  automation:ControlRef<automation>/1,
  generation:Counter,
  lowerOriginalStartUtcSeconds:CanonicalDecimal,
  activeDefinition:Binding<automation>/1,
  definitionRevision:Counter,
  source:ScheduleSourceBinding/2
}

ScheduleOccurrenceProof/2 =
    {version:2,kind:"once",at:zoned_instant}
  | {version:2,kind:"recurrence",
     evidence:ScheduleRecurrenceEvidence/2,
     projection:<complete accepted D4 recurrence projection outcome>,
     originalStart:<the selected outcome row's D4 originalStart>}

AutomationOccurrenceRecord/2 = {
  kind:"d10_automation_occurrence_record",
  version:2,
  key:AutomationOccurrenceKey/1,
  definition:Binding<automation>/1,
  definitionRevision:Counter,
  dueUtcSeconds:CanonicalDecimal,
  proof:ScheduleOccurrenceProof/2,
  disposition:AutomationOccurrenceDisposition/1
}

ScheduleContinuityWitness/2 = {
  kind:"d6_schedule_continuity",
  version:2,
  automation:ControlRef<automation>/1,
  subscriptionGeneration:Counter,
  revision:Counter,
  initial:ScheduleRecurrenceEvidence/2,
  checkpoint:ScheduleRecurrenceEvidence/2,
  status:"continuous"|"binding_changed"|"gap",
  producerEpoch:Token,
  consumedTransition:Counter
}

ScheduleContinuityStep/2 = {
  kind:"d6_schedule_continuity_step",
  version:2,
  automation:ControlRef<automation>/1,
  subscriptionGeneration:Counter,
  expectedWitnessRevision:Counter,
  producerEpoch:Token,
  transition:Counter,
  before:ScheduleRecurrenceEvidence/2,
  after:ScheduleRecurrenceEvidence/2,
  portableChanges:[{
    changeRecordPin:PinRef/2,
    installationNoticePin:PinRef/2,
    completionProofPin:PinRef/2
  }...],
  dependencyBefore:DependencyProof/3,
  dependencyAfter:DependencyProof/3,
  retainedInputs:[PinRef/2...]
}

D10AuthorPreparationLink/2 = {
  kind:"d10_author_preparation_link",
  version:2,
  run:ControlRef<run>/1,
  stepId:Counter,
  automation:Binding<automation>/1,
  definitionRevision:Counter,
  taskDigest:Sha256,
  preparedBindingToken:Token,
  request:d6_commit_request/2
}

ApprovalUse/2 = {
  version:2,
  approval:Binding<approval>/1,
  run:ControlRef<run>/1,
  stepId:Counter,
  request:d6_commit_request/2,
  decisionKey:DecisionKey/2,
  preparedBindingToken:Token,
  previewSemanticDigest:Sha256,
  delegationBinding:Binding<lease>/1,
  activationBinding:ActivationBinding/1,
  count:ApprovalCountReservation/1,
  budgetReservations:[ControlRef<reservation>/1...]
}

D10AuthorStepResponsibility/2 =
    {version:2,kind:"core_field_member",
     link:D10AuthorPreparationLink/2,
     decisionKey:DecisionKey/2,protocolOwner:"D6",
     preparedRecordPin:PinRef/2,recoveryPins:[PinRef/2...]}
  | {version:2,kind:"interactive",
     run:ControlRef<run>/1,stepId:Counter,decisionKey:DecisionKey/2,
     authorRequest:
         {protocolOwner:"D3",request:<complete identity_operation_request wire13>}
       | {protocolOwner:"D6",request:d6_commit_request/2},
     preparedFormat:
       "d7_prepared_action_binding4"|"d8_prepared_edit_binding3",
     preparedRecordPin:PinRef/2,recoveryPins:[PinRef/2...]}
```

ApprovalUse/2.preparedBindingToken 必须选择 exact PAB4。其 preview digest 固定为
SHA-256(UTF8("D10-Author-Preview/2") || NUL || D3-CJ/3(normalized complete EffectManifest/3))；
EffectBytes/3 slot只替换为 {encoding,byteLength,payloadDigest}，不按成员名递归猜测。

Schedule current proof中真实解析 managed Document 时必须含 source + document_format dependency。source/profile bytes未变但format proof continuity gap得到 gap；binding发生真实改变得到 binding_changed，即使最终recurrence/range值碰巧相同。旧 Subscription1 只有在 explicit continue 且完整历史证明无format/rule discontinuity时才可同generation形成Subscription2；否则要求replace。

# 11. 历史分派与单一 authority

saved先按原 owner/version重放；planned恢复原descriptor/proof/prepared/pins/Notice/install/version basis；unknown保留原Approval/Money/claim/external/stop责任；只有unseen走current successors。旧bytes/pins绝不因进入mixed wrapper而重编码。

所有本文 successor继续使用原单一 DecisionKey、planning CAS、installation、P seal、receipt/outbox。document_format不是第二Document authority；SourceTransform不是source authority；ChangeRecord不是第二portable truth；D10 versioned wrapper不赋新authority；Derived Index永远不能重建current proof。

# 12. Provider profiles

Provider closed shapes见 SPEC §4。它们是生态renderer资格，不改变core-language validity。Mermaid fixed source是 CLI 12.0.0 commit db1ceebbe529d7975474eb0d0e9c23e9dc57cd37，并固定package.json/blob与package-lock/blob；Node/browser/fonts/config仍必须用实际version+digest登记。没有已登记runtime就只能unavailable，不能用浮动semver、host browser/font或网络下载补齐。

# 13. 验收引用

所有上述schema与cross-field规则的正负设计义务逐项列在 [ACCEPTANCE.zh-CN.md](ACCEPTANCE.zh-CN.md)：437 core +117 coordination，共554，全部未运行。该文件的ID正文是规范的一部分，不允许用计数替代。
