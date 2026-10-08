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
  sourceEncoding:"UTF-8",
  localeProfile:ProcessorLocaleProfile/1,
  attributeOverrides:[ProcessorAttributeOverride/1...],
  timeInputs:ProcessorTimeInputs/1,
  includeEnvironment:IncludeEnvironment/2,
  extensionProfiles:[AcceptedProcessorExtensionProfile/1...],
  providerProfiles:[AcceptedProcessorProviderProfile/1...]
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

ProcessorLocaleProfile/1 = {
  profileId:text,
  lang:text|null,
  lcAll:text|null,
  lcCtype:text|null,
  lcTime:text|null,
  descriptorSha256:"sha256:<64 lowercase hex>"
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

AcceptedProcessorProviderProfile/1 = {
  providerKind:"stem"|"mermaid"|"syntax_highlighter"|"html"|"pdf"|"docx",
  profileId:text,
  semanticVersion:text,
  descriptorSha256:"sha256:<64 lowercase hex>"
}
```

attributeOverrides 是有序 sequence，不排序、不去重。每个 event 的 name 必须是固定 Ruby key syntax 处理后得到的 non-empty lowercase name；hard_set/soft_set 要求 value 非 null，hard_unset/soft_unset 要求 null。canonical decoder 按数组顺序重建 ordered attr_overrides：某 name 首次出现时把 key 追加到尾部，后续同名 event 只替换状态而不移动 key。D3-CJ/3 因而直接保留 API enumeration order；两个仅交换 notitle/showtitle 首次 key 顺序的环境必须有不同 canonical bytes。embedded 模式按该 ordered key 顺序执行固定 2.0.26 的 notitle/showtitle alias 派生，不允许先排序再派生。

sourceEncoding 恰为 UTF-8；其它 source encoding 不属于本 Gate 的直接输入。localeProfile 必须是受控 oracle harness 的完整已登记描述，profileId+descriptorSha256 与四个显式 locale environment 值共同冻结，不允许读取未记录 host locale。extensionProfiles 按 profileId 排序唯一，providerProfiles 按 (providerKind,profileId) 排序唯一，sourceUnits 按 logicalPath.value 排序唯一。providerProfiles 为空表示本次 evaluation 没有调用 provider，不表示 provider 不存在。safe<server 且未 override 时 user-home 来自 ambientUserHome；server/secure 原生 default 为“.”。SOURCE_DATE_EPOCH 存在时 local*/doc* 统一取其 UTC 值；否则 local* 取 clockNow，doc* 优先 inputMtime、再 clockNow。

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

SemanticHeadSubjectKind/1 =
  "document"|"section"|"block"|"list"|"list_item"|"table"|
  "column"|"cell"|"inline"|"catalog_record"

SupportingOnlySubjectKind/1 =
  "attribute_buffer"|"table_parser_context"

ForwardSemanticRole/1 =
  "header"|"child"|"dlist_term"|"dlist_description"|
  "table_column"|"table_head_cell"|"table_body_cell"|"table_foot_cell"|
  "cell_inner_document"

BackedgeSemanticRole/1 =
  "parent"|"cell_column"

ModelRelation/1 = {
  role:ForwardSemanticRole/1|BackedgeSemanticRole/1,
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

### 4.3 Witness/7 exact closure verifier

**A. observation / snapshot / bind 静态层。** modelPropertyObservations 按数组顺序要求 observationId 严格递增。对每个 subject，将全部 snapshot 按 observationId 排序：首项 previousSnapshotObservationId 必须 null，后续项必须恰指同 subject 直接前一项。same subjectId 在全部 snapshot 中 kind byte-equal；真实 Ruby object→subjectId 一一关系属于 producer-conformance。定义 latestSnapshotBefore(s,o) 为 subject=s 且 observationId<o 的最大 snapshot。每个 bind 必须引用存在、同 subject 的 latestSnapshotBefore(subjectId,bind.observationId)。

ModelCarrierReference 的静态分派恰为：
- document：entry 必须 0，并解析 Witness.document；
- blockEvents / collectionEvents / catalogEvents / inlineObservations / contentObservations / calls：entry 是对应数组 0-based index，必须在范围内且目标类型与 stream 完全匹配。

这里不比较 carrier.entry 与 model observationId。

**B. root 与 physical semantic closure。** 对 cut C，在 observationId<C.observationId 的合法 bind 中取 carrier=(document,0) 的最新 bind 为 rootBind(C)；必须存在，subject kind=document，且 C.documentSubjectId=rootBind.subjectId。对任何候选 subject s，latest[s]=latestSnapshotBefore(s,C.observationId)。

structural worklist 从 C.documentSubjectId 开始，只沿 latest[s].relations 中 ForwardSemanticRole/1 的九个闭合 role 扩张；target subject 必须存在并属于 SemanticHeadSubjectKind/1。完成 forward traversal 后，Rstruct 内每个 latest snapshot 的 parent/cell_column backedge target 必须已经在 Rstruct；backedge 永不新增 target。old root、old Cell 或其它 stale object 不能靠 backedge 复活。

随后只做两个 supplementary pass：
1. inline：kind=inline；cut 前至少一个合法 bind 指向 inlineObservations；latest snapshot 恰有一个 parent，且 parent target 已在 Rstruct。满足则加入 R。
2. catalog_record：cut 前至少一个合法 bind 指向 catalogEvents；latest snapshot 恰有一个 catalog ownership parent；target kind=document 且 target 已在 Rstruct。满足则加入 R。不存在 owner role，也不得把 target 不在 Rstruct 的 Document 加入 R。

SupportingOnlySubjectKind/1 永不加入 R。加入 supplementary 后再次检查 R 中全部 backedge；target 必须已经在 R。令 H=set(C.heads[].subjectId)，要求 heads 按 subjectId 严格升序、subjectId 唯一、H==R，并且每个 head.snapshotObservationId=latest[head.subjectId].observationId。缺 live subject、extra stale subject、duplicate、stale head 或 relation conflict 都使 Witness invalid/incomplete。

**C. supporting evidence closure。** seed 恰为：所有 cut head snapshots、rootBind(C)、以及 observationId<C.observationId 且 subjectId∈R 的全部合法 bind。按以下 typed edge 递归直到不再增加节点：
- snapshot → previousSnapshotObservationId；每个 field/attribute slot 的 provenance.inputs；ModelObservedValue 的 string/array/map 子值；
- ModelPropertyInput.model_slot 分支指向 modelPropertyObservations 中指定的 snapshot，并要求目标 slot 真实存在；observed_value 分支指向 observedStrings[valueId]；observed_slice 分支指向 observedStrings[valueId] 并验证对应 byte range；operation 分支指向 operations[operationId]；inline_field 分支指向 inlineObservations[inlineEventId] 并验证 fieldPath；
- OracleStringOperation 继续指向 calls[callId]、inputs/removedInputs 中的每个 slice 与 observedStrings[outputValueId]；run.exact_copy 继续指向对应 slice；run.derived 继续指向全部 slices 与它引用的 operationId；run.generated 在 inlineEventId 非 null 时继续指向该 Inline observation；
- OracleEvaluationCall → 非 null parentCallId；
- OracleInlineObservation → callId、非 null parentCallId、非 null producingOperationId、nodeType 与 fields 中递归 OracleFieldValue、非 null returnValueId；
- OracleFieldValue.observed_string → observedStrings[valueId]；array/entries → 每个 nested value；
- OracleContentObservation → calls[callId]、全部 resultValueIds、全部 contributingInlineEventIds；
- OracleObservedSlice → observedStrings[valueId] 并验证 0<=startByte<=endByte<=byteLength；OracleObservedString 为叶子；
- bind → 其 carrier；carrier 若为 inlineObservations/contentObservations/calls，继续上述递归；其它 carrier 执行各自 retained closed validation，无额外跨数组时间边。

同 namespace 的 model refs 使用 backward/latest；operation/value/inline/call 各只在自身 namespace 验 existence/type/DAG；subject 只按 identity/relation；禁止跨 namespace 数值比较。任何 dangling/wrong-kind、invalid slice/index、operation/call cycle 或缺失 provenance input 都失败。support closure 可含 SupportingOnlySubjectKind/1、旧 snapshot/Cell 与 temporary evidence，但不会因此加入 H。

**D. producer-time 与 cardinality。** static decoder 只能证明最终 stream/index/type 与上述 reference/closure；完整 Witness validity 还要求 producerConformanceValid。每个真实 bind callback 发生时 carrier 已实际 append，entry < targetStream.lengthAtBind，callback 仍持有 exact 同一 Ruby object，并在其后 append bind。document/0 只能在 actual returned top-level Document carrier 发布后绑定。未来补齐 carrier 永不治愈 earlier invalid bind；不得靠 text/title/source/path/hash 事后匹配。

完整 top-level Witness/7 恰一个 model_ready cut。selected-backend evaluation 未发生或异常退出时为零个有效 evaluation_complete；正常返回时恰一个，且其 observationId 大于 model_ready、documentSubjectId byte-equal。inner Document 不创建第二 cut namespace。model_ready/evaluation_complete 的 head 都是对应 cut 的 latest physical snapshot；physical cut membership 与 CoreSemanticPropertyProfile/1 的 semantic survival/winner 分离。

catalog ownership仍只能是 parent -> actual Document#register receiver。temporary overlay 必须由固定源码真实 restore write 或 cleanup delete/closure 在合法 evaluation_complete 前结束；observer不得伪造 restore。

## 4.4 D2 current product Document projection

这一 family 是产品数据，不是 OracleSemanticWitness/7 或 CoreSemanticProjection/1。当前 /3 仍是未实现候选，因此本节直接完成同一 /3 contract，不为未部署候选制造 /4 迁移。

```text
D2ProductInteger := CanonicalSignedDecimal
// effective section levels require a nonnegative value; there is no upper language clamp.

D2NativeSemanticValue/1 =
    null | Boolean | text
  | {integer:CanonicalSignedDecimal}
  | {textArray:[text...]}

D2NativeAttributeEntry/1 = {
  name:text,
  value:D2NativeSemanticValue/1,
  provenance:"authored_named"|"authored_positional"|"fixed_derived",
  sourceOrigins:D2SourceOriginSet/2
}

D2NativeAttributeSet/1 = {
  entries:[D2NativeAttributeEntry/1...]
}

D2BlockCommonSemantics/1 = {
  style:text|null,
  caption:text|null,
  numeral:text|null,
  subs:[text...],
  positional:[D2NativeSemanticValue/1...],
  origins:{
    style:D2SourceOriginSet/2,
    caption:D2SourceOriginSet/2,
    numeral:D2SourceOriginSet/2,
    subs:[D2SourceOriginSet/2...],
    positional:[D2SourceOriginSet/2...]
  }
}

D2SourceOwner/1 =
    {kind:"root_document",logicalPath:ProcessorPath/1,
     ownerNodeRef:NodeRef,sourceObservation:SourceObservation/1}
  | {kind:"managed_include",logicalPath:ProcessorPath/1,
     ownerNodeRef:NodeRef,sourceObservation:SourceObservation/1}
  | {kind:"artifact_include",logicalPath:ProcessorPath/1,pin:PinRef/2}
  | {kind:"network_snapshot",logicalPath:ProcessorPath/1,
     requestDigest:"sha256:<64 lowercase hex>",pin:PinRef/2}

D2SourceRange/2 = {
  source:D2SourceOwner/1,
  startByte:Counter,endByte:Counter,
  startLine:Counter,startColumn:Counter,
  endLine:Counter,endColumn:Counter
}

D2WritableSource/1 =
    {kind:"unique",range:D2SourceRange/2}
  | {kind:"none",
     reason:"generated"|"multi_origin"|"ambiguous"|"non_author_source"}

D2SourceTransformKind/1 =
  "attribute"|"specialchars"|"quotes"|"macros"|"replacements"|
  "post_replacements"|"include"|"parser_derived"|"model_derived"|
  "weftext_adapter"

D2SourceOrigin/2 =
    {id:Counter,kind:"authored",range:D2SourceRange/2,
     inputs:[],writableSource:D2WritableSource/1}
  | {id:Counter,kind:"reference",range:D2SourceRange/2,
     inputs:[Counter...],writableSource:D2WritableSource/1}
  | {id:Counter,kind:"substitution",operation:D2SourceTransformKind/1,
     range:D2SourceRange/2|null,inputs:[Counter...],
     writableSource:D2WritableSource/1}
  | {id:Counter,kind:"generated",operation:D2SourceTransformKind/1,
     range:D2SourceRange/2|null,inputs:[Counter...],
     writableSource:D2WritableSource/1}
  | {id:Counter,kind:"synthetic",inputs:[Counter...],
     writableSource:{kind:"none",reason:"generated"|"multi_origin"|"ambiguous"}}
  | {id:Counter,kind:"multi_origin",inputs:[Counter...],
     writableSource:{kind:"none",reason:"multi_origin"|"ambiguous"}}

D2SourceOriginGraph/2 = {
  origins:[D2SourceOrigin/2...]
}

D2SourceOriginSet/2 = {
  originIds:[Counter...],
  writableSource:D2WritableSource/1
}

D2ProductReadBarrier/1 = {
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  principalAudienceToken:Token,
  frontier:Frontier/2
}

D2ProductEvaluationBinding/1 = {
  processorEnvironment:AsciiDocProcessorEnvironment/3,
  processorEnvironmentSha256:"sha256:<64 lowercase hex>",
  rootSource:SourceUnitBinding/1,
  includeSources:[SourceUnitBinding/1...],
  readBarrier:D2ProductReadBarrier/1,
  observationScope:ObservationScope/2,
  dependencyProof:DependencyProof/3
}

D2ProductDiagnostic/1 = {
  severity:"debug"|"info"|"warn"|"error",
  semanticCode:text,
  message:text,
  sourceRange:D2SourceRange/2|null
}

D2TextProjection/2 = {
  text:text,
  sourceOrigins:D2SourceOriginSet/2
}

D2DocumentBackendState/1 = {
  backend:text,
  basebackend:text,
  filetype:text,
  outfilesuffix:text,
  origins:{
    backend:D2SourceOriginSet/2,
    basebackend:D2SourceOriginSet/2,
    filetype:D2SourceOriginSet/2,
    outfilesuffix:D2SourceOriginSet/2
  }
}

D2DocumentAuthor/1 = {
  name:text,
  firstname:text|null,
  middlename:text|null,
  lastname:text|null,
  initials:text|null,
  email:text|null,
  origins:{
    name:D2SourceOriginSet/2,
    firstname:D2SourceOriginSet/2,
    middlename:D2SourceOriginSet/2,
    lastname:D2SourceOriginSet/2,
    initials:D2SourceOriginSet/2,
    email:D2SourceOriginSet/2
  }
}

D2DocumentRevision/1 = {
  number:text|null,
  date:text|null,
  remark:text|null,
  origins:{
    number:D2SourceOriginSet/2,
    date:D2SourceOriginSet/2,
    remark:D2SourceOriginSet/2
  }
}

D2EffectiveDocumentAttribute/1 = {
  name:text,
  value:D2NativeSemanticValue/1,
  sourceOrigins:D2SourceOriginSet/2
}

D2DocumentMetadata/3 = {
  backend:D2DocumentBackendState/1,
  doctype:"article"|"book"|"manpage"|"inline",
  safeMode:"unsafe"|"safe"|"server"|"secure",
  doctypeOrigins:D2SourceOriginSet/2,
  safeModeOrigins:D2SourceOriginSet/2,
  title:D2TextProjection/2|null,
  mainTitle:D2TextProjection/2|null,
  subtitle:D2TextProjection/2|null,
  authors:[D2DocumentAuthor/1...],
  revision:D2DocumentRevision/1|null,
  effectiveAttributes:[D2EffectiveDocumentAttribute/1...],
  headerAttributes:[{name:text,value:text|null,
                    sourceRange:D2SourceRange/2,
                    sourceOrigins:D2SourceOriginSet/2}...]
}

D2AttributeCarrierBlock/2 = {
  kind:"attribute_carrier_block",
  namespaceToken:text,
  sourceRange:D2SourceRange/2,
  namespaceRange:D2SourceRange/2,
  entries:[{kind:"lexical_attribute_entry",rawEntrySource:text,
            sourceRange:D2SourceRange/2}...]
}

D2IdentityAdapter/1 =
    {kind:"node_link",target:NodeRef}
  | {kind:"resource",target:ResourceRef}
  | {kind:"citation",target:NodeRef,citationKey:text|null}

D2InlineMediaSemantics/1 =
    {kind:"image",
     width:text|null,height:text|null,format:text|null,
     scaledwidth:text|null,scale:text|null,link:text|null,window:text|null,
     float:text|null,align:text|null,fallback:text|null,imagesdir:text|null,
     origins:{width:D2SourceOriginSet/2,height:D2SourceOriginSet/2,
              format:D2SourceOriginSet/2,scaledwidth:D2SourceOriginSet/2,
              scale:D2SourceOriginSet/2,link:D2SourceOriginSet/2,
              window:D2SourceOriginSet/2,float:D2SourceOriginSet/2,
              align:D2SourceOriginSet/2,fallback:D2SourceOriginSet/2,
              imagesdir:D2SourceOriginSet/2}}
  | {kind:"icon",
     size:text|null,flip:text|null,rotate:text|null,title:text|null,
     width:text|null,height:text|null,
     origins:{size:D2SourceOriginSet/2,flip:D2SourceOriginSet/2,
              rotate:D2SourceOriginSet/2,title:D2SourceOriginSet/2,
              width:D2SourceOriginSet/2,height:D2SourceOriginSet/2}}

D2ProductInline/3 =
    {kind:"text",text:text,sourceOrigins:D2SourceOriginSet/2}
  | {kind:"quoted",style:"strong"|"emphasis"|"monospaced"|"mark"|
                         "superscript"|"subscript"|"double"|"single"|"unquoted",
     id:text|null,roles:[text...],children:[D2ProductInline/3...],
     slotOrigins:{style:D2SourceOriginSet/2,id:D2SourceOriginSet/2,
                  roles:[D2SourceOriginSet/2...]},
     nativeAttributes:D2NativeAttributeSet/1,
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"link",nativeTarget:text,label:[D2ProductInline/3...],
     slotOrigins:{nativeTarget:D2SourceOriginSet/2},
     adapter:D2IdentityAdapter/1|null,nativeAttributes:D2NativeAttributeSet/1,
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"xref",nativeTarget:text,refid:text|null,path:text|null,
     label:[D2ProductInline/3...],adapter:D2IdentityAdapter/1|null,
     slotOrigins:{nativeTarget:D2SourceOriginSet/2,
                  refid:D2SourceOriginSet/2,path:D2SourceOriginSet/2},
     nativeAttributes:D2NativeAttributeSet/1,
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"anchor",referenceKind:"ref"|"bibref",
     id:text,reftext:text|null,
     slotOrigins:{referenceKind:D2SourceOriginSet/2,id:D2SourceOriginSet/2,
                  reftext:D2SourceOriginSet/2},
     nativeAttributes:D2NativeAttributeSet/1,
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"image",nativeTarget:text,alt:text|null,
     semantics:D2InlineMediaSemantics/1,
     slotOrigins:{nativeTarget:D2SourceOriginSet/2,alt:D2SourceOriginSet/2},
     adapter:D2IdentityAdapter/1|null,
     nativeAttributes:D2NativeAttributeSet/1,
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"footnote",id:text|null,index:CanonicalSignedDecimal|null,
     referenceKind:null|"ref"|"xref",target:text|null,
     children:[D2ProductInline/3...],
     slotOrigins:{id:D2SourceOriginSet/2,index:D2SourceOriginSet/2,
                  referenceKind:D2SourceOriginSet/2,target:D2SourceOriginSet/2},
     nativeAttributes:D2NativeAttributeSet/1,
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"indexterm",visibility:"visible"|"concealed",
     text:text|null,terms:[text...],see:text|null,seeAlso:[text...],
     slotOrigins:{visibility:D2SourceOriginSet/2,text:D2SourceOriginSet/2,
                  terms:[D2SourceOriginSet/2...],see:D2SourceOriginSet/2,
                  seeAlso:[D2SourceOriginSet/2...]},
     nativeAttributes:D2NativeAttributeSet/1,
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"stem",notation:"tex"|"asciimath",source:text,
     slotOrigins:{notation:D2SourceOriginSet/2,source:D2SourceOriginSet/2},
     nativeAttributes:D2NativeAttributeSet/1,
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"kbd",keys:[text...],
     slotOrigins:{keys:[D2SourceOriginSet/2...]},
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"menu",path:[text...],
     slotOrigins:{path:[D2SourceOriginSet/2...]},
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"button",label:text,
     slotOrigins:{label:D2SourceOriginSet/2},
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"callout",label:text,number:CanonicalSignedDecimal|null,
     id:text|null,
     guard:null|text|{before:text,after:text},
     slotOrigins:{label:D2SourceOriginSet/2,number:D2SourceOriginSet/2,
                  id:D2SourceOriginSet/2,guard:D2SourceOriginSet/2},
     sourceOrigins:D2SourceOriginSet/2}
  | {kind:"line_break",sourceOrigins:D2SourceOriginSet/2}
  | {kind:"passthrough",text:text,sourceOrigins:D2SourceOriginSet/2}

D2Heading/3 = {
  kind:"heading",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,
  sourceOrigins:D2SourceOriginSet/2,
  authoredLevel:integer(1..9),
  effectiveLevel:D2ProductInteger,
  inlines:[D2ProductInline/3...],
  anchor:text|null,
  roles:[text...],
  options:[text...],
  slotOrigins:{authoredLevel:D2SourceOriginSet/2,
               effectiveLevel:D2SourceOriginSet/2,
               anchor:D2SourceOriginSet/2,
               roles:[D2SourceOriginSet/2...],
               options:[D2SourceOriginSet/2...]},
  nativeAttributes:D2NativeAttributeSet/1
}

D2ParagraphBlock/3 = {
  kind:"paragraph",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  anchor:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  slotOrigins:{anchor:D2SourceOriginSet/2,
               roles:[D2SourceOriginSet/2...],
               options:[D2SourceOriginSet/2...]},
  common:D2BlockCommonSemantics/1,
  inlines:[D2ProductInline/3...],
  nativeAttributes:D2NativeAttributeSet/1
}

D2SectionSemantics/1 = {
  sectname:text,
  special:Boolean,
  numbered:false|true|"chapter",
  numeral:text|null,
  caption:text|null,
  origins:{sectname:D2SourceOriginSet/2,special:D2SourceOriginSet/2,
           numbered:D2SourceOriginSet/2,numeral:D2SourceOriginSet/2,
           caption:D2SourceOriginSet/2}
}

D2SectionBlock/3 = {
  kind:"section",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  heading:D2Heading/3,
  common:D2BlockCommonSemantics/1,
  semantics:D2SectionSemantics/1,
  nativeAttributes:D2NativeAttributeSet/1,
  children:[D2ProductBlock/3...]
}

D2ListItemSemantics/1 = {
  coids:[text...],
  coidOrigins:[D2SourceOriginSet/2...],
  nativeAttributes:D2NativeAttributeSet/1
}

D2ListItem/3 = {
  kind:"list_item",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  marker:text|null,checked:Boolean|null,
  slotOrigins:{marker:D2SourceOriginSet/2,checked:D2SourceOriginSet/2},
  terms:[[D2ProductInline/3...]...],
  inlines:[D2ProductInline/3...],
  semantics:D2ListItemSemantics/1,
  children:[D2ProductBlock/3...]
}

D2ListSemantics/1 =
    {kind:"unordered",style:text|null,checklist:Boolean,interactive:Boolean,
     origins:{style:D2SourceOriginSet/2,checklist:D2SourceOriginSet/2,
              interactive:D2SourceOriginSet/2}}
  | {kind:"ordered",style:text|null,start:CanonicalSignedDecimal|null,
     reversed:Boolean,
     origins:{style:D2SourceOriginSet/2,start:D2SourceOriginSet/2,
              reversed:D2SourceOriginSet/2}}
  | {kind:"description",style:text|null,labelwidth:text|null,
     itemwidth:text|null,
     origins:{style:D2SourceOriginSet/2,labelwidth:D2SourceOriginSet/2,
              itemwidth:D2SourceOriginSet/2}}
  | {kind:"callout",style:text|null,
     origins:{style:D2SourceOriginSet/2}}

D2ListBlock/3 = {
  kind:"list",listKind:"unordered"|"ordered"|"description"|"callout",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  anchor:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  slotOrigins:{anchor:D2SourceOriginSet/2,
               roles:[D2SourceOriginSet/2...],
               options:[D2SourceOriginSet/2...]},
  common:D2BlockCommonSemantics/1,
  semantics:D2ListSemantics/1,
  nativeAttributes:D2NativeAttributeSet/1,
  items:[D2ListItem/3...]
}

D2TableColumn/3 = {
  ordinal:Counter,width:text|null,halign:text|null,valign:text|null,
  style:text|null,sourceOrigins:D2SourceOriginSet/2,
  slotOrigins:{width:D2SourceOriginSet/2,halign:D2SourceOriginSet/2,
               valign:D2SourceOriginSet/2,style:D2SourceOriginSet/2},
  nativeAttributes:D2NativeAttributeSet/1
}

D2TableCell/3 = {
  kind:"table_cell",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  columnStart:Counter,colspan:Counter,rowspan:Counter,style:text|null,
  halign:text|null,valign:text|null,
  slotOrigins:{columnStart:D2SourceOriginSet/2,colspan:D2SourceOriginSet/2,
               rowspan:D2SourceOriginSet/2,style:D2SourceOriginSet/2,
               halign:D2SourceOriginSet/2,valign:D2SourceOriginSet/2},
  nativeAttributes:D2NativeAttributeSet/1,
  content:
      {kind:"inline",inlines:[D2ProductInline/3...]}
    | {kind:"blocks",children:[D2ProductBlock/3...]}
}

D2TableRow/3 = {
  kind:"table_row",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  section:"head"|"body"|"foot",
  sectionOrigins:D2SourceOriginSet/2,
  cells:[D2TableCell/3...]
}

D2TableSemantics/1 = {
  cols:text|null,format:text|null,separator:text|null,width:text|null,
  frame:text|null,grid:text|null,stripes:text|null,float:text|null,
  orientation:text|null,colcount:Counter,rowcount:Counter,
  tablepcwidth:text|null,tableabswidth:text|null,
  origins:{cols:D2SourceOriginSet/2,format:D2SourceOriginSet/2,
           separator:D2SourceOriginSet/2,width:D2SourceOriginSet/2,
           frame:D2SourceOriginSet/2,grid:D2SourceOriginSet/2,
           stripes:D2SourceOriginSet/2,float:D2SourceOriginSet/2,
           orientation:D2SourceOriginSet/2,colcount:D2SourceOriginSet/2,
           rowcount:D2SourceOriginSet/2,tablepcwidth:D2SourceOriginSet/2,
           tableabswidth:D2SourceOriginSet/2}
}

D2TableBlock/3 = {
  kind:"table",locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  anchor:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  slotOrigins:{anchor:D2SourceOriginSet/2,
               roles:[D2SourceOriginSet/2...],
               options:[D2SourceOriginSet/2...]},
  common:D2BlockCommonSemantics/1,
  semantics:D2TableSemantics/1,
  nativeAttributes:D2NativeAttributeSet/1,
  columns:[D2TableColumn/3...],
  rows:[D2TableRow/3...]
}

D2DelimitedSemantics/1 =
    {kind:"listing"|"source",language:text|null,linenums:Boolean,
     start:CanonicalSignedDecimal|null,indent:CanonicalSignedDecimal|null,
     tabsize:CanonicalSignedDecimal|null,highlight:text|null,
     lineComment:text|null,
     origins:{language:D2SourceOriginSet/2,linenums:D2SourceOriginSet/2,
              start:D2SourceOriginSet/2,indent:D2SourceOriginSet/2,
              tabsize:D2SourceOriginSet/2,highlight:D2SourceOriginSet/2,
              lineComment:D2SourceOriginSet/2}}
  | {kind:"literal",indent:CanonicalSignedDecimal|null,
     tabsize:CanonicalSignedDecimal|null,lineComment:text|null,
     origins:{indent:D2SourceOriginSet/2,tabsize:D2SourceOriginSet/2,
              lineComment:D2SourceOriginSet/2}}
  | {kind:"quote",attribution:text|null,citetitle:text|null,
     origins:{attribution:D2SourceOriginSet/2,citetitle:D2SourceOriginSet/2}}
  | {kind:"verse",attribution:text|null,citetitle:text|null,
     indent:CanonicalSignedDecimal|null,tabsize:CanonicalSignedDecimal|null,
     origins:{attribution:D2SourceOriginSet/2,citetitle:D2SourceOriginSet/2,
              indent:D2SourceOriginSet/2,tabsize:D2SourceOriginSet/2}}
  | {kind:"stem",notation:text|null,
     origins:{notation:D2SourceOriginSet/2}}
  | {kind:"pass"}

D2DelimitedBlock/3 = {
  kind:"delimited",
  blockKind:"listing"|"literal"|"source"|"pass"|"stem"|"quote"|"verse",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  anchor:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  style:text|null,
  slotOrigins:{anchor:D2SourceOriginSet/2,
               roles:[D2SourceOriginSet/2...],
               options:[D2SourceOriginSet/2...],
               style:D2SourceOriginSet/2,
               contentText:D2SourceOriginSet/2|null},
  common:D2BlockCommonSemantics/1,
  semantics:D2DelimitedSemantics/1,
  nativeAttributes:D2NativeAttributeSet/1,
  content:
      {kind:"text",text:text,sourceOrigins:D2SourceOriginSet/2}
    | {kind:"inline",inlines:[D2ProductInline/3...]}
    | {kind:"blocks",children:[D2ProductBlock/3...]}
}

D2ContainerSemantics/1 =
    {kind:"admonition",name:text,textlabel:text|null,icon:text|null,
     origins:{name:D2SourceOriginSet/2,textlabel:D2SourceOriginSet/2,
              icon:D2SourceOriginSet/2}}
  | {kind:"example"|"sidebar"|"open"|"preamble"|"abstract"|"partintro"}

D2ContainerBlock/3 = {
  kind:"container",
  blockKind:"example"|"sidebar"|"open"|"admonition"|"preamble"|
            "abstract"|"partintro",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  anchor:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  slotOrigins:{anchor:D2SourceOriginSet/2,
               roles:[D2SourceOriginSet/2...],
               options:[D2SourceOriginSet/2...]},
  common:D2BlockCommonSemantics/1,
  semantics:D2ContainerSemantics/1,
  nativeAttributes:D2NativeAttributeSet/1,
  children:[D2ProductBlock/3...]
}

D2MediaSemantics/1 =
    {kind:"image",width:text|null,height:text|null,format:text|null,
     scaledwidth:text|null,scale:text|null,link:text|null,window:text|null,
     float:text|null,align:text|null,fallback:text|null,imagesdir:text|null,
     origins:{width:D2SourceOriginSet/2,height:D2SourceOriginSet/2,
              format:D2SourceOriginSet/2,scaledwidth:D2SourceOriginSet/2,
              scale:D2SourceOriginSet/2,link:D2SourceOriginSet/2,
              window:D2SourceOriginSet/2,float:D2SourceOriginSet/2,
              align:D2SourceOriginSet/2,fallback:D2SourceOriginSet/2,
              imagesdir:D2SourceOriginSet/2}}
  | {kind:"audio",start:text|null,end:text|null,
     autoplay:Boolean,controls:Boolean,loop:Boolean,
     origins:{start:D2SourceOriginSet/2,end:D2SourceOriginSet/2,
              autoplay:D2SourceOriginSet/2,controls:D2SourceOriginSet/2,
              loop:D2SourceOriginSet/2}}
  | {kind:"video",poster:text|null,width:text|null,height:text|null,
     start:text|null,end:text|null,preload:text|null,float:text|null,
     align:text|null,hash:text|null,theme:text|null,lang:text|null,
     list:text|null,playlist:text|null,autoplay:Boolean,loop:Boolean,
     muted:Boolean,controls:Boolean,fullscreen:Boolean,modest:Boolean,
     related:Boolean,
     origins:{poster:D2SourceOriginSet/2,width:D2SourceOriginSet/2,
              height:D2SourceOriginSet/2,start:D2SourceOriginSet/2,
              end:D2SourceOriginSet/2,preload:D2SourceOriginSet/2,
              float:D2SourceOriginSet/2,align:D2SourceOriginSet/2,
              hash:D2SourceOriginSet/2,theme:D2SourceOriginSet/2,
              lang:D2SourceOriginSet/2,list:D2SourceOriginSet/2,
              playlist:D2SourceOriginSet/2,autoplay:D2SourceOriginSet/2,
              loop:D2SourceOriginSet/2,muted:D2SourceOriginSet/2,
              controls:D2SourceOriginSet/2,fullscreen:D2SourceOriginSet/2,
              modest:D2SourceOriginSet/2,related:D2SourceOriginSet/2}}

D2MediaBlock/3 = {
  kind:"media",mediaKind:"image"|"audio"|"video",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  nativeTarget:text,alt:text|null,title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  slotOrigins:{nativeTarget:D2SourceOriginSet/2,alt:D2SourceOriginSet/2,
               roles:[D2SourceOriginSet/2...],
               options:[D2SourceOriginSet/2...]},
  common:D2BlockCommonSemantics/1,
  semantics:D2MediaSemantics/1,
  nativeAttributes:D2NativeAttributeSet/1,
  adapter:D2IdentityAdapter/1|null
}

D2AtomicSemantics/1 =
    {kind:"floating_title"}
  | {kind:"page_break"}
  | {kind:"thematic_break"}
  | {kind:"toc",levels:CanonicalSignedDecimal|null,
     origins:{levels:D2SourceOriginSet/2}}

D2AtomicBlock/3 = {
  kind:"atomic",
  blockKind:"floating_title"|"page_break"|"thematic_break"|"toc",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,sourceOrigins:D2SourceOriginSet/2,
  anchor:text|null,
  level:D2ProductInteger|null,
  title:[D2ProductInline/3...],
  roles:[text...],options:[text...],
  slotOrigins:{anchor:D2SourceOriginSet/2,level:D2SourceOriginSet/2,
               roles:[D2SourceOriginSet/2...],
               options:[D2SourceOriginSet/2...]},
  common:D2BlockCommonSemantics/1,
  semantics:D2AtomicSemantics/1,
  nativeAttributes:D2NativeAttributeSet/1
}

D2SavedDefinitionBlock/3 = {
  kind:"saved_query_view_definition",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,
  sourceOrigins:D2SourceOriginSet/2,
  definitionKind:"query"|"view"|"dynamic_block",
  payload:text,
  slotOrigins:{definitionKind:D2SourceOriginSet/2,payload:D2SourceOriginSet/2}
}

D2BibliographyPlacementBlock/3 = {
  kind:"bibliography_placement",
  locator:DocumentElementLocator,
  sourceRange:D2SourceRange/2,
  sourceOrigins:D2SourceOriginSet/2
}

D2ProductBlock/3 =
    D2SectionBlock/3 | D2ParagraphBlock/3 | D2ListBlock/3 |
    D2TableBlock/3 | D2DelimitedBlock/3 | D2ContainerBlock/3 |
    D2MediaBlock/3 | D2AtomicBlock/3 |
    D2SavedDefinitionBlock/3 | D2BibliographyPlacementBlock/3

D2DocumentBody/3 = {
  kind:"document_body",
  children:[D2ProductBlock/3...]
}

D2DocumentPayload/3 =
    {source:text,
     parse:{status:"valid",diagnostics:[D2ProductDiagnostic/1...]},
     projection:{state:"available",
                 metadata:D2DocumentMetadata/3,
                 attributeCarrierBlocks:[D2AttributeCarrierBlock/2...],
                 body:D2DocumentBody/3},
     d2CommitEligibility:"eligible",
     repairVisibility:"not_required"}
  | {source:text,
     parse:{status:"invalid",diagnostics:[D2ProductDiagnostic/1...]},
     projection:{state:"unavailable"},
     d2CommitEligibility:"reject",
     repairVisibility:"exact_source_only"}

D2DocumentSnapshot/3 = {
  wireVersion:3,
  kind:"document_snapshot",
  ownerNodeRef:NodeRef,
  evaluation:D2ProductEvaluationBinding/1,
  originGraph:D2SourceOriginGraph/2,
  document:D2DocumentPayload/3
}

D7HeadingProjection/2 = {
  owner:NodeRef,
  title:text,
  level:CanonicalSignedDecimal
}
```

全部 product union 都 strict：unknown/missing member、unknown arm、duplicate JSON key、非法 null 与跨 arm member 一律拒绝。D2ProductBlock/3 与 D2ProductInline/3 是固定 baseline 加具名 Weftext extension 的 current 完整产品 family；不存在 generic/opaque block、inline、property JSON 或第二 AST escape。

D2NativeAttributeSet/1 是唯一允许可变命名属性的 native attribute carrier；value 只能来自闭合 D2NativeSemanticValue/1，entries 保留固定 parser 最终 named attribute map 的真实 enumeration order，name 唯一。D2BlockCommonSemantics/1 单独承载无法可靠从 named map 恢复的 public AbstractBlock slots：实际 style、caption、numeral、subs 以及原 positional order。这样既允许合法 arbitrary author attribute name，也不会打开自由 JSON。各 kind 的 dedicated semantic member 不是可省略 shortcut：同一个 fixed-Ruby fact 若同时出现在 nativeAttributes/common 与专用字段，经固定语义转换后必须 byte-equal。只有 fixed public model/converter 真正可观察的 internal derived attribute 才能保留。inline image 还必须区分 image/icon；reference anchor 区分 ref/bibref；footnote type 只允许 null|ref|xref；callout guard 精确保留 scalar 或两段 comment guard。

字段级 provenance 是 closed product value 的一部分，而不是第二次 parse。D2NativeAttributeEntry/1 的每个 entry 直接携 sourceOrigins；D2BlockCommonSemantics/1、所有具名 per-kind semantics 与 block/inline slotOrigins 对象逐字段镜像其 semantic member。sequence-valued slot 的 origin array 长度必须与 value array 完全相等并按相同顺序配对；null/derived scalar 仍有自己的 D2SourceOriginSet/2。title/label/children/terms 等递归 D2ProductInline/3 序列由其每个 child 自身的字段级 origin 承载，不允许父 node 的 coarse sourceOrigins 冒充其独立 slot。unknown/missing origin member、长度错位、把一个 slot 的 origin 复用给另一个独立 slot，或仅凭 sourceRange/path/text 重建映射都 strict-fail。

structured write gate 只看正在编辑的那个 slot 的 D2SourceOriginSet/2.writableSource，再叠加该 exact range.source owner 的当前写权限与最终 evaluation/read barrier currentness。kind=unique 且 source owner 可写/current 时必须允许原 owner 已定义的 structured edit 正向路径；generated、multi_origin、ambiguous、non_author_source 或缺写权限时该 slot 仍可读但 structured-readonly。managed include 的 unique authored range 也只授予其自己 source owner 的写权限，绝不从 root Document 权限继承。比如 `[source,ruby]` 的 block style、language 与 body text 分别绑定各自 span；`[#foo.red]` 的 id、每个 role 与 child text 分别绑定各自 span。reference/substitution/environment-derived 值保留真实 graph 输入但不能靠 node-level range 猜 writable target；任一 bound source/environment/dependency 在 final barrier stale 都使旧 slot write 失败并重新准备。

D2DocumentMetadata/3 是固定 Ruby 2.0.26 最终 Document public/converter observable state 的产品 carrier，而不是 header 的近似摘要。backend 四元组与 safeMode/doctype 来自同一次 evaluation；title=完整 doctitle，mainTitle/subtitle 是同一次固定 title partition。authors 按 Document#authors 顺序保存 name/firstname/middlename/lastname/initials/email 六个成员，D9 或 UI 禁止从 full name/source 重新拆分。effectiveAttributes 按固定 Document#attributes 最终实际 enumeration order 保存完整 present map 且 name 唯一，包括 converter 可见的 builtin/generated/environment-derived attribute；这些 entry 通过 sourceOrigins 表示 authored/substitution/generated provenance，绝不为没有 physical author span 的值伪造 sourceRange。headerAttributes 只保留真实 physical header lexical entries及其 range，不承担 effective attribute authority。revision 的 number/date/remark 和全部 document-level独立成员同样保留字段级 origin。D9/D8 consumer只能消费这些 exact product members，不能从 raw source、fullname、默认 backend 或 ambient environment 补猜。

effectiveAttributes[].value 精确复用已经闭合的 D2NativeSemanticValue/1 域。必须保留 final fixed-Ruby Hash 的 key 是否存在以及真实 Ruby value class：key absent 就没有 effectiveAttributes entry；present nil 编码为 null；TrueClass/FalseClass 编码 Boolean；String 编码 text；Integer 编码 {integer:CanonicalSignedDecimal}，使用无界且无多余前导零的规范十进制；Array 只有在每个元素都是 String 时才编码 {textArray:[text...]}，并完整保留元素顺序、重复项与空串。fixed core 的 safe-mode-level、max-include-depth、authorcount 因此保持 integer，mannames 保持 fixed HTML5/DocBook converter 实际消费的有序 text array。任何 producer 都不得 to_s、JSON-stringify、join、normalize、sort 或重 parse 这些值。ProcessorAttributeOverride/1 本身只允许 text set-value 与 null unset action，不能借 API 注入其它 Ruby class。accepted extension profile 只有在每个 present final Document#attributes value 都落入这个 closed domain 时才与 rich product 兼容；Symbol、Float、Hash、nested/mixed/non-String Array 或其它 opaque object 必须使 rich product projection unavailable，不能成为 free JSON。typed value 与 sourceOrigins 必须来自同一次 fixed evaluation；generated/non-author value 不伪造 physical range，D7/D8/D9 直接消费 typed value。

D2SourceOriginGraph/2 只在一个 snapshot 内有效。origin id 必须从0连续无洞；每个 input id 都小于引用它的 node id，因此 graph 无环且不形成第二 identity namespace。authored 恰有一个 physical source range；reference 记录真实 authored reference site 与其 input；substitution/generated 记录固定 transformation family 与全部真实 inputs；multi_origin 至少两个 inputs。sourceOrigins.originIds 排序唯一且都存在。writableSource 从这些 origin 机械导出：只有所有 retained path 最终收敛到同一个 byte-equal authored writable range 才是 unique；generated、non-author、ambiguous 或 multi-source 都只能 structured-read。exact Source read/save 继续由原 authorization 独立提供。

D2SourceOwner/1 必须 version-exact。root_document 与 managed_include 携带求值实际使用的 SourceObservation/1；artifact/network source unit 携带 exact immutable pin；全部 variant 都携带 processor environment 中的 logicalPath。不得由 path/title/hash 推导 source unit。current managed D2DocumentSnapshot/3 强制 processorEnvironment.input.kind=managed_file，其 ownerNodeRef 与 snapshot.ownerNodeRef byte-equal；rootSource.kind=managed 且 owner/SourceObservation/logicalPath 全部相等。includeSources 与 processorEnvironment.includeEnvironment.sourceUnits byte-equal。processorEnvironmentSha256 唯一为：；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。

```text
"sha256:" + lowercase_hex(
  SHA-256(
    UTF8("D2-Processor-Environment/3") || NUL ||
    D3-CJ/3(processorEnvironment)
  )
)
```

readBarrier 的 Workspace/CommitDomain 必须等于 dependencyProof，readBarrier.frontier 等于 dependencyProof.baseFrontier，observationScope 就是取得该 proof 使用的 scope。proof 必须包含 root source key、每个 managed include source key、managed root 的 document_format，以及 native/Weftext projection 实际消费的其它 Registry/foreign/authorization dependency；未实际使用的 source unit/environment 不能为了扩大 proof 而塞入。每个 SourceUnitBinding 的 managed observation 或 immutable pin 都必须与 originGraph 中对应 source owner byte-equal。

current D7/D8/D9 consumer 在消费 projection bytes 前，必须在最终 authorized read barrier 验证完整 evaluation binding。完全相同 cut 可直接使用；later cut 只能沿既有 continuous scope_dependencies 规则，证明全部 bound source unit、environment-relevant control、authorization 与 negative dependency 均未改变。managed include SourceObservation、artifact/network pin、processor-environment bytes 或任一实际 dependency 变化，都使旧 tree stale，即使 root source 与 document_format byte-equal。snapshot pin 证明“当时生成了哪棵 tree”；evaluation binding 证明“这棵 tree 现在是否仍 current”。

projection available 时，每个可寻址 occurrence 都带该类型要求的 exact current D3 DocumentElementLocator。D2SourceRange/2 byte interval 为半开区间，line/column endpoint 必须属于 exact versioned source owner，start 不得晚于 end。semantic sequence 保持 parser order；roles/options 保持语言定义顺序；只有 owner 明确声明为 set 的字段才排序唯一。columnStart 是应用此前 row/column span 后的 zero-based physical table-grid column，同一 row 内每个 occupied cell origin 唯一。

BaselineOnly 只接受固定 Ruby baseline 真正 parse 的 heading；WeftextManaged 额外允许 authoredLevel 6–9。effectiveLevel 使用 nonnegative CanonicalSignedDecimal 承载 fixed Ruby Integer 在仅做 lower clamp 后的结果，没有 Counter/int64 upper language limit。处理巨大合法 level 时资源/work budget 可以返回 budget_exceeded，但不能把源文档改判 syntax-invalid 或 numeric_overflow。D7 headings 使用既有 TypeSpec {kind:"integer"} 与其 canonical decimal V encoding 暴露该值；旧 int64 heading projection 只作历史输入。

D2IdentityAdapter/1 只能在 enclosing native occurrence 已成功 parse 后出现。node_link/citation target 必须是 D3-qualified stable identity；resource target 是 owner-local，并满足产生该 occurrence 的 Document/source-owner 规则。adapter 只是附加 typed identity，不能替代 native parser，也不能成为 writable source target。

完整 current product snapshot 的 canonical protected pin 为 artifact/recovery：

```text
UTF8("D2-Document-Snapshot/3") || NUL ||
D3-CJ/3(complete D2DocumentSnapshot/3)
```

历史 document_snapshot wireVersion2 与有限 BodyBlock/Inline decoder 只作 historical，绝不扩成 /3。


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

ComponentImage/1 与 PinRef/2 保持 fixed-e8aa 的精确结构。document_format 的 present component bytes 必须是 D3-CJ/3(binding)，且 ComponentImage.version=bindingRevision。

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

D3IdentityInput/13 = {
  kind:"d3_identity_input",version:13,
  mode:D3Mode,
  writeProtection:"strict",
  expectedAuthority?:ExpectedAuthority,
  workspaceProposal?:WorkspaceAllocationProposal,
  intent:D3Intent
}

D3IdentityOperationRequest/13 = {
  wireVersion:13,
  kind:"identity_operation_request",
  operationId:UUIDv4,
  boundWorkspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  guarantee:"replica_local"|"managed_atomic",
  expectedFrontier:Frontier/2,
  inputDescriptor:InputDescriptor/3,
  mode:D3Mode,
  expectedAuthority?:ExpectedAuthority,
  workspaceProposal?:WorkspaceAllocationProposal,
  preparationBinding?:PreparationBinding,
  intent:D3Intent
}
```

D3IdentityInput/13 精确保留 D3IdentityInput/12 的语义成员集合；只有 outer request boundary 的嵌套 D6 descriptor family 更新为 current InputDescriptor/3。expectedAuthority、workspaceProposal、preparationBinding 都是真正 optional member，绝不是 nullable placeholder。replica_local 只允许 create_node/move_node/reorder_node/trash，并要求这三个 member 全部 absent。managed_atomic 严格继承 fixed-parent matrix：create/fork 要求 expectedAuthority=create 及 required proposal；continue 要求 expectedAuthority=continue；其它 managed_atomic mode 要求 expectedAuthority=existing；preparationBinding 只在继承的 D7-mediated mode 允许时出现。禁止的 member 即使 value 本身可解码也必须 invalid_request，JSON null 一律非法。D3IdentityOperationRequest/13 仅把 wire12 top-level 的 wireVersion 改为13并改用 InputDescriptor/3。descriptor/request 的 mode、authority、proposal 在存在时逐字节相等；guarantee/frontier policy、ownerInput、requestFingerprint、DecisionKey/OperationId 单一 ledger 顺序、saved/planned/unseen 分支和 error/disclosure order 都保持。historical wire9–12 不扩 decoder、不重编码。

## 6.1 PreparedActionBinding/4

current D7 action successor 使用精确继承，不是自由扩展。D7ActionSpec/2 的顶层对象等于 fixed-parent ActionSpec/1 仅把 version 改为2；其 intent decoder 恰为 fixed-parent intent union 删除 historical apply_suggestion arm 后，再加入下面三个 closed Annotation arms。其它 fixed-parent arm 的成员与语义逐字节沿用。

```text
D7CreateAnnotationIntent/2 = {
  kind:"create_annotation",
  destinationOwnerRef:NodeRef,
  value:AnnotationEditableProposal/1
}

D7ApplySuggestionIntent/2 = {
  kind:"apply_suggestion",
  annotation:AnnotationRef,
  expectedAnnotationRevisionToken:AnnotationRevisionToken/1
}

D7RejectSuggestionIntent/2 = {
  kind:"reject_suggestion",
  annotation:AnnotationRef,
  expectedAnnotationRevisionToken:AnnotationRevisionToken/1
}

D7ActionSpec/2 :=
  ActionSpec/1 的 top-level version 改为2，
  删除 fixed-parent apply_suggestion arm，
  再加入 D7CreateAnnotationIntent/2、
         D7ApplySuggestionIntent/2 与 D7RejectSuggestionIntent/2

D7ActionPrepareRequest/3 = {
  wireVersion:3,kind:"d7_action_prepare",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  expectedFrontier:Frontier/2,
  action:D7ActionSpec/2,
  selectedSources:[SourceVersionRef/1...],
  budget:BudgetBinding/1,
  evidenceToken?:Token
}

D7ActionInput/3 = {
  kind:"d7_action_input",version:3,
  action:D7ActionSpec/2,
  canonicalCallInputs:[QueryCall...],
  definitionInputs:[D7DefinitionInput/2...],
  registryInputs:[ValidatedCatalogContext...],
  ruleInputs:[RecurrenceReadContext...],
  proposedInputs:[D7ProposedInput/3...]
}

PreparedActionBinding/4 = {
  kind:"d7_prepared_action_binding",
  version:4,
  bindingToken:Token,
  protocolOwner:"D3"|"D6",
  operationId:UUIDv4,
  workspaceRef:WorkspaceRef,
  principalAudienceToken:Token,
  action:D7ActionSpec/2,
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
  proposedInputs:[D7ProposedInput/3...],
  dependencyProof:DependencyProof/3,
  observationProof:<PreparedIntent/3.observationProof>,
  budgetBinding:BudgetBinding/1,
  expiresAt:<D6 protected deadline>,
  request:D3IdentityOperationRequest/13|d6_commit_request/2,
  preview:<complete immutable EffectManifest/3 semantics,pins,initial delivery>,
  resolutionInput:null|D3ResolutionInput/1
}
```

MinimumMapping/3、D7DefinitionInput/2 与 D7ResolutionAccess/1 保持 fixed-e8aa exact shape。D7ProposedInput/2 仅保留给真实 historical d7_action/2 / PAB3 decoder，绝不原地扩展。

```text
D7ProposedInput/3 = {
  subject:PayloadSubjectKey,
  payloadKind:
    "exact_source_document"|"resource_bytes"|"annotation_value",
  encoding:
    "exact_source_utf8"|"resource_bytes"|
    "d3_annotation_value4"|"d3_symbolic_result9",
  pin:PinRef/2
}
```

current `/3` cross-field 唯一合法组合是 exact_source_document↔exact_source_utf8、resource_bytes↔resource_bytes，以及 annotation_value↔d3_annotation_value4|d3_symbolic_result9。protocolOwner=D6 的 current Annotation concrete after 必须使用 d3_annotation_value4，并且 pin bytes 恰为 D3-CJ/3(完整 D3-Annotation-Value/4)；d3_symbolic_result9 只允许继承的 current D3 symbolic-result 分支，并按 Result/9 的真实 subject/pin 规则绑定。`d3_annotation_value3` 在 `/3` 中非法。OwnerInputBinding/2.pinRefs 必须精确覆盖 `/3` 中全部 proposed pins 与实际受保护 evidence；historical PAB3/Input2 继续逐字节使用 D7ProposedInput/2 与 d3_annotation_value3，不能 repin/reencode。

protocolOwner=D6 的 current D7 action 使用 InputDescriptor/3.ownerInput：protocolOwner=D7，ownerKind=intentKind=d7_action/3；canonicalDescriptorBytes 恰为 D3-CJ/3(D7ActionInput/3)。D7ActionInput/3 的六个成员与 PreparedActionBinding/4 对应成员逐项相等，ownerInput.pinRefs 继续恰覆盖 proposed pins 与实际受保护 source/definition/rule evidence，并按 D6 规则排序唯一。protocolOwner=D3 的 create_annotation 等 identity action 仍使用 D3 自己的 d3_identity_operation/13 owner descriptor；PAB4 只绑定 cross-owner preparation，不创建第二 D3 request authority。historical d7_action/2/PAB3 保留原 decoder、bytes 与 recovery。

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

EffectItem/3 保留 fixed-parent 的十四个 arm，并只新增一个 current owner-specific arm：presentation_policy_change:D8PresentationPolicyEffect/1。它与 series_configuration_change 属于同一类 typed shared-configuration owner effect，不是新的 PortableComponentKey、Policy/3 mutation、Registry value、author source 或 generic JSON。全部 byte slot 使用 EffectBytes/3，Annotation source image 使用 Value/4，current workspace bootstrap 允许 Plan4。historical EffectItem/1-/2 decoder 绝不扩张。；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。

## 6.3 PreparedEditBinding/3

```text
D8EditIntent/3 =
    {kind:"document",
     target:D8SourceTarget/2,
     source:text}
  | {kind:"annotation",
     target:D8SourceTarget/2,
     expectedAnnotationRevisionToken:AnnotationRevisionToken/1,
     value:AnnotationEditableProposal/1,
     targetPolicy:"preserve"|"replace_current"}
  | {kind:"annotation_reconfirm_suggestion",
     target:D8SourceTarget/2,
     expectedAnnotationRevisionToken:AnnotationRevisionToken/1}

D8PinnedEditIntent/3 =
    {kind:"document",
     target:D8SourceTarget/2,
     proposedSource:PinRef/2}
  | {kind:"annotation",
     target:D8SourceTarget/2,
     expectedAnnotationRevisionToken:AnnotationRevisionToken/1,
     proposedValue:PinRef/2,
     targetPolicy:"preserve"|"replace_current"}
  | {kind:"annotation_reconfirm_suggestion",
     target:D8SourceTarget/2,
     expectedAnnotationRevisionToken:AnnotationRevisionToken/1,
     proposedValue:PinRef/2}

D8EditInput/3 = {
  kind:"d8_edit_input",version:3,
  invocationClass:"interactive_source_save"|"noninteractive",
  writeProtection:"strict"|"observed_only",
  intent:D8PinnedEditIntent/3,
  origin:{kind:"direct"}|{kind:"undo",originalRequest:OriginalD6Request}
}

D8EditPrepareRequest/3 = {
  wireVersion:3,kind:"d8_edit_prepare",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  saveProfile:"ordinary"|"complete",
  guarantee:"replica_local"|"managed_atomic",
  writeProtection:"strict"|"observed_only",
  intent:D8EditIntent/3,
  budget:BudgetBinding/1
}

D8AnnotationDraftProjection/1 =
    {kind:"d8_annotation_draft",version:1,
     annotationRef:AnnotationRef,
     baseObservation:SourceObservation/1,
     baseRevisionToken:AnnotationRevisionToken/1,
     draftSerial:Counter,
     access:"editable"|"readonly",
     value:AnnotationEditableValue/1,
     targetResolution:"exact"|"mapped"|"candidate"|"ambiguous"|"orphaned"|"unavailable",
     body:{state:"valid",source:text}}
  | {kind:"d8_annotation_draft",version:1,
     annotationRef:AnnotationRef,
     baseObservation:SourceObservation/1,
     baseRevisionToken:AnnotationRevisionToken/1,
     draftSerial:Counter,
     access:"editable"|"readonly",
     value:AnnotationEditableValue/1,
     targetResolution:"exact"|"mapped"|"candidate"|"ambiguous"|"orphaned"|"unavailable",
     body:{state:"invalid",source:text,diagnostics:[CoreDiagnostic/1...]}}

PreparedEditBinding/3 = {
  kind:"d8_prepared_edit_binding",version:3,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  operationId:UUIDv4,principalAudienceToken:Token,
  inputDescriptor:InputDescriptor/3,
  intent:D8EditIntent/3,
  origin:{kind:"direct"}|{kind:"undo",originalRequest:OriginalD6Request},
  sourceInputs:InputDescriptor/3.sourceInputs,
  proposedInputs:[{entityRef:EntityRef,pin:PinRef/2}],
  registryInputs:[ValidatedCatalogContext...],
  dependencyProof:DependencyProof/3,
  observationProof:PreparedIntent/3.observationProof,
  budgetBinding:BudgetBinding/1,expiresAt:PreparedDeadline,
  request:d6_commit_request/2,preview:EffectManifest/3
}
```

document intent 的 proposedInputs.pin 选择精确 UTF-8 source。annotation intent 的 proposedInputs 恰一项，entityRef 等于 target.ref，且 PinRef/2 payloadKind=annotation_value；pin bytes 必须是 Core 构造的完整 D3-Annotation-Value/4 的 D3-CJ/3，而不是只 pin AnnotationEditableValue/1。target.ref 必须是 AnnotationRef，expectedAnnotationRevisionToken 必须等于 prepare 时 current PortableAnnotationRecord/4 token。Annotation 强制 saveProfile=complete 与 writeProtection=strict；observed_only 仍只允许原已资格化 interactive Document 路径。

OwnerInputBinding/2 的 current ownerKind/intentKind=d8_edit/3，canonicalDescriptorBytes=D3-CJ/3(D8EditInput/3)。pinRefs 恰为 D8PinnedEditIntent/3 命名的 pins 与 retained origin evidence 的排序去重集合。D8EditInput/3.intent 必须从 PreparedEditBinding/3.intent 机械派生，只把 source/value bytes 替换为对应 proposed pin；任何 request shape 都没有 actor/time 或 trust flag。historical d8_edit_prepare wire1/2 与 PreparedEditBinding/1/2 继续原 intent/value decoder 与 recovery。

### 6.3.1 Current Annotation read / Draft producer 与 operation-class input

```text
D8AnnotationReadRequest/1 = {
  wireVersion:1,kind:"d8_annotation_read",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  annotationRef:AnnotationRef
}

D8AnnotationBodyRead/1 =
    {state:"absent",exactSource:null,semanticText:""}
  | {state:"valid",exactSource:text,semanticText:text}
  | {state:"invalid",exactSource:text,semanticText:null,
     diagnostics:[CoreDiagnostic/1...]}

D8AnnotationReadResponse/1 = {
  kind:"d8_annotation_read",version:1,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  annotationRef:AnnotationRef,
  sourceObservation:SourceObservation/1,
  annotationRevisionToken:AnnotationRevisionToken/1,
  value:D3-Annotation-Value/4,
  targetResolution:"exact"|"mapped"|"candidate"|"ambiguous"|"orphaned"|"unavailable",
  body:D8AnnotationBodyRead/1
}

D8AnnotationDraftOpenRequest/1 = {
  wireVersion:1,kind:"d8_annotation_draft_open",
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  annotationRef:AnnotationRef
}

D8AnnotationDraftOpenResponse/1 = {
  kind:"d8_annotation_draft_open",version:1,
  read:D8AnnotationReadResponse/1,
  draft:D8AnnotationDraftProjection/1
}
```

这两个是真实 Core/D8 entry，不是 response-only shape。`d8_annotation_read` 在最终 authorized read barrier 返回完整 Value/4，因此 creator/authoredAt/lastEditor/editedAt 可显示；Draft 仍只保存/编辑 AnnotationEditableValue/1，绝不是第二 author authority。`d8_annotation_draft_open` 复用同一次 current read：有 annotation_write 时 draft.access=editable，否则为 readonly；readonly 不能进入 D8EditPrepareRequest/3。Draft 的 baseObservation/baseRevisionToken 必须分别等于 read.sourceObservation/read.annotationRevisionToken，serial=0 时 editable projection 必须等于 read.value 的八个 editable fields。local Draft 后续可以变化，但 prepare 与最终 read barrier 必须再次验证原 base token/Observation。

D8EditIntent/3 的操作类别由封闭的 intent 分支与 targetPolicy 机械决定，绝不能由调用方提交可信标志：annotation+preserve 对应 ordinary_edit，annotation+replace_current 对应 manual_reattach，annotation_reconfirm_suggestion 对应 reconfirm_suggestion。调用方的 AnnotationEditableProposal/1 不携带 Suggestion 的生命周期状态或证明；Core 先对 current before + proposal 执行 closed operation-class gate，展开一份保留 before attribution 的完整 candidate Value/4，然后才做 canonical candidate-vs-before no-op 比较。只有 candidate 真正不同才写入 fresh Core lastEditor/editedAt、fresh revision token 与唯一 planned/pinned final Value/4。reconfirm 分支没有 caller value，其 proposedValue 完全由 Core 重新读取真实目标并重算产生；成功 reconfirm 是真实状态转换。

## 6.4 D8 workspace presentation policy 与 render binding

presentation preference 是 existing Portable Workspace Metadata 中的 shared Workspace configuration，但它是 D8-owned complex configuration，和 SeriesScopeConfiguration 同类。本设计明确不新增 PortableComponentKey arm，也不复用 Policy/3 或 Registry。currentness 由完整 D8 head set 与写入 owner effect 的同一个 D6 planning CAS/P decision 证明。

```text
D8PresentationPolicyAddress/1 = {
  workspaceRef:WorkspaceRef,
  revision:Counter,
  recordSha256:"sha256:<64 lowercase hex>"
}

D8WorkspacePresentationPolicy/1 = {
  kind:"d8_workspace_presentation_policy",version:1,
  workspaceRef:WorkspaceRef,
  revision:Counter,
  defaultPresentation:"separate"|"run_in"
}

D8WorkspacePresentationPolicy/2 = {
  kind:"d8_workspace_presentation_policy_record",version:2,
  workspaceRef:WorkspaceRef,
  revision:Counter,
  parents:[D8PresentationPolicyAddress/1...],
  defaultPresentation:"separate"|"run_in",
  activationChangeId:ChangeId/1
}

D8PresentationPolicyHeadSet/1 = {
  kind:"d8_presentation_policy_heads",version:1,
  workspaceRef:WorkspaceRef,
  stamp:{epoch:Token,revision:Counter},
  heads:[D8PresentationPolicyAddress/1...]
}

D8WorkspacePresentationPolicyBinding/2 = {
  policy:D8WorkspacePresentationPolicy/1,
  record:D8WorkspacePresentationPolicy/2,
  address:D8PresentationPolicyAddress/1,
  headSet:D8PresentationPolicyHeadSet/1,
  pin:PinRef/2
}

D8PresentationPolicyProposal/1 = {
  workspaceRef:WorkspaceRef,
  parents:[D8PresentationPolicyAddress/1...],
  revision:Counter,
  defaultPresentation:"separate"|"run_in"
}

D8PresentationPolicyBootstrapInit/1 = {
  kind:"d8_presentation_policy_bootstrap_init",version:1,
  before:D8PresentationPolicyHeadSet/1,
  proposal:D8PresentationPolicyProposal/1
}

D8PresentationPolicySetRequest/2 = {
  wireVersion:2,
  kind:"d8_presentation_policy_set",
  workspaceRef:WorkspaceRef,
  commitDomain:CommitDomain/2,
  expectedFrontier:Frontier/2,
  expectedHeads:[D8PresentationPolicyAddress/1...],
  defaultPresentation:"separate"|"run_in",
  budget:BudgetBinding/1
}

D8PresentationPolicyInput/1 = {
  kind:"d8_presentation_policy_input",version:1,
  workspaceRef:WorkspaceRef,
  headSet:D8PresentationPolicyHeadSet/1,
  defaultPresentation:"separate"|"run_in"
}

D8PresentationPolicyHeadEvidence/1 = {
  address:D8PresentationPolicyAddress/1,
  recordPin:PinRef/2
}

D8PresentationPolicyPrepareResult/2 =
    {kind:"d8_presentation_policy_no_change",version:2,
     workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
     headSet:D8PresentationPolicyHeadSet/1}
  | {kind:"d8_presentation_policy_prepared",version:2,
     operationId:UUIDv4,
     input:D8PresentationPolicyInput/1,
     proposal:D8PresentationPolicyProposal/1,
     headEvidence:[D8PresentationPolicyHeadEvidence/1...],
     prepared:PreparedIntent/3,
     request:d6_commit_request/2,
     preview:EffectManifest/3}

D8PresentationPolicyOutboxItem/1 = {
  kind:"d8_presentation_policy_outbox",version:1,
  decisionKey:DecisionKey/2,
  address:D8PresentationPolicyAddress/1,
  recordPin:PinRef/2
}

D8PresentationPolicyEffect/1 = {
  kind:"presentation_policy_change",version:1,
  before:D8PresentationPolicyHeadSet/1,
  proposal:D8PresentationPolicyProposal/1,
  committed:null|D8WorkspacePresentationPolicy/2
}

D8PresentationDecision/1 =
    {kind:"explicit",
     sourceOverride:"run_in"|"separate"|"conflict",
     effective:"run_in"|"separate",
     diagnostic:null|"role_conflict"}
  | {kind:"no_eligible_body",effective:"separate"}
  | {kind:"workspace_default",
     policy:D8WorkspacePresentationPolicyBinding/2,
     effective:"run_in"|"separate"}

D8PresentationResult/1 = {
  heading:DocumentElementLocator,
  body:DocumentElementLocator|null,
  decision:D8PresentationDecision/1
}

D8DocumentRenderBinding/1 = {
  sourceObservation:SourceObservation/1,
  documentFormat:DocumentFormatCurrentQualification/1,
  documentSnapshotPin:PinRef/2,
  presentation:D8PresentationDecision/1,
  rendererProfileSha256:"sha256:<64 lowercase hex>"
}
```

D8WorkspacePresentationPolicy/2 record bytes 唯一为：

```text
UTF8("D8-Workspace-Presentation-Policy/2") || NUL ||
D3-CJ/3(complete policy)
```

recordSha256 覆盖上述完整 prefixed bytes。PinRef/2 选择完全相同 bytes，payloadKind=portable_metadata、retention=recovery；address.workspaceRef/revision 与 record 相等。D8WorkspacePresentationPolicyBinding/2.policy 必须机械构造成 {kind:"d8_workspace_presentation_policy",version:1,workspaceRef:record.workspaceRef,revision:record.revision,defaultPresentation:record.defaultPresentation} 这个唯一 logical current view，不单独持久化；这样保留 Stage4A /1 current-value contract 而不制造第二 authority。parents 按 (revision,recordSha256) 排序唯一且均属于同一 Workspace，每个 parent record 都必须 retained/valid。初始 record revision=1、parents=[]、defaultPresentation=separate。后继 parents 必须等于 prepare/seal 时完整 protected current head set，revision=checked(max(parent.revision)+1)，activationChangeId 必须等于该 portable D6 P decision 唯一分配的 ChangeId。因此两个 offline writer 都可能形成 revision2，但不同 hash/ChangeId 会保留为两个 heads；revision number、arrival order、LWW 都不能选 winner。

D8PresentationPolicyHeadSet/1 是 D8 owner 对 Portable Workspace Metadata 中全部 maximal valid policy record 的 current observation。stamp 是这个 exact Workspace configuration 的 protected Core state：完整 head enumeration 首次建立一段 continuous proof epoch 时 revision=1；correctness evidence continuity 丢失/重建时换 epoch，之后每次已证明的 head-set transition 都 checked-increment revision；两个数字都不能选 branch。它不是 caller evidence 或 index scan。heads 排序唯一；一个 head 才是 current，多个 head 是 conflict。heads=[] 是**已经证明的 uninitialized owner state**，绝不能从“看不到记录”推断：只有真实 producer 绑定了完整受保护 empty-set observation 时才合法，即仍未 activation 且由原 D3 target-custody 证明为 fresh 的 D8PresentationPolicyBootstrapInit/1.before，或 SetRequest 明确接纳的 current proved-uninitialized state。missing/corrupt metadata、unknown parent decoder、缺 retained record、unproved continuity gap 或仅仅 backing file 不存在都必须 unavailable，绝不能变成 [] 或 ambient default。成功 activation 的 current Plan4 fresh Workspace 不可能以 [] 被观察到，因为 initial record/head 与 activation 原子提交。current binding 要求 headSet.heads == [address]，record/pin/address byte-equal，policy 等于 record 的机械 /1 view，并且完整 ancestry retained。

D8PresentationPolicyBootstrapInit/1 不是公开的变更请求。before.workspaceRef 与 proposal.workspaceRef 必须和全新 target Workspace 按字节完全相等；before.heads=[]，before.stamp 是 D8 owner 的受保护全新观测，由原 D3 target-custody 对目标历史确为空的证明派生，且 before.stamp.revision=1。
proposal 必须精确为 parents=[]、revision=1、defaultPresentation=separate。该 helper 不携带 activationChangeId、已提交 record、address、hash、pin、outbox item 或第二个 authorization decision，而且只能用于 current unseen WorkspaceBootstrapPlan/4。

D8PresentationPolicySetRequest/2 固定 managed_atomic/strict，并有一个实际 current producer。错误顺序是 closed decode → presentation-state disclosure → policy_admin → expectedFrontier/domain qualification → protected head-set read → expectedHeads exact equality → retained head record/pin/ancestry availability → semantic/budget。任何更早阶段失败都不得暴露 hidden head/value。一个 current head 且 defaultPresentation 与请求相同是 d8_presentation_policy_no_change：返回已授权的 exact headSet 后结束，零 PreparedIntent、零 P、零 ChangeId/record/outbox；[] 初始化与 multi-head conflict 即使所有 head value 相同也绝不是 no-op。

改变路径构造 D8PresentationPolicyInput/1 和 Proposal/1：proposal.parents 等于完整 frozen headSet.heads，revision 在 [] 时为1，否则 checked(max(parent.revision)+1)，并固定请求的 defaultPresentation；proposal 本身永远没有 activationChangeId。headEvidence 与 heads 一一同序且 address byte-equal，recordPin 选择 retained parent record 的 exact canonical bytes。Core 随后构造一个现有 PreparedIntent/3：InputDescriptor/3 使用 protocolOwner=D8、ownerKind=intentKind=d8_presentation_policy/1、saveProfile=control_only、guarantee=managed_atomic、frontierPolicy=exact、observationScope=既有 control_only Workspace scope、sourceInputs=[]，canonicalDescriptorBytes=D3-CJ/3(input)，controlInputs/DependencyProof 精确包含实际读取的 authorization 及其它真实 control dependency，pinRefs 精确覆盖 headEvidence 加实际 authorization/dependency evidence。这个 P-only shared configuration producer 不取得 author source 或 SourceRevisionPlan，也绝不把 P 变成作者内容 store；唯一 ChangeId 仍是下文只在 final P 分配的 ordinary portable decision ChangeId。preview 恰含一个 presentation_policy_change，其 before=frozen headSet、proposal=frozen proposal、committed=null。D8PresentationPolicyPrepareResult/2 的 prepared/request 分别是这一个 current PreparedIntent/3 与由同一 operationId/planToken 产生的原 d6_commit_request/2；Core 在返回前原子保存这份 outside association。saved/planned/unknown 只恢复这些 original bytes/pins/head stamp/proposal/request，绝不 fresh-resample heads。

winning planning CAS 冻结 exact Input、headSet stamp、headEvidence、Proposal、authorization/dependency/budget、preview 和 D6 plan责任。由于 current record 必须包含尚未分配的 activationChangeId，prepare、planning、staging、InstallationNotice、install 与 verify 阶段一律不得构造/编码 D8WorkspacePresentationPolicy/2，不得计算其 recordSha256，不得创建它的 portable_metadata pin 或 D8PresentationPolicyOutboxItem/1，也不得预留 ChangeId。presentation policy 继续复用 fixed-parent EffectManifest 的真实 rank-6 series_configuration_change 所在的 complex-owner-effect 类：EffectItem/3 使用 typed presentation_policy_change，但不增加 PortableComponentKey；任何该 decision 既有 component install/verify 仍只描述原 D6 component，绝不塞一个虚构 policy component。

final P 是唯一 materialization 点。P 先按原 D6 顺序重验 policy_admin/authorization、domain fence、expectedFrontier、完整 frozen headSet stamp/heads、parent records/ancestry、dependency 与 budget；old-head 预验后若另一个 writer 已改变 head set，当前 plan conflict/reprepare，且尚无新 ChangeId。全部通过后，同一个 P transaction 才 checked-allocate唯一 ChangeId C，然后从 frozen Proposal+C 构造唯一 D8WorkspacePresentationPolicy/2；严格 D3-CJ/3 编码带固定前缀，计算 recordSha256，建立 exact recovery portable_metadata PinRef/2/address，并把 committed presentation_policy_change（before/proposal/committed）、原 decision/receipt/effects、ChangeRecord association 与 D8PresentationPolicyOutboxItem/1 一起落入原 P/outbox，同时把 owner head graph 原子转为这个 successor。record.activationChangeId、ChangeRecord.changeId、receipt/effect decision 与 outbox decisionKey 必须全等；任一步失败都在 P commit 前整体失败，不能留下半个 record/pin/head。

两个 offline writer 可从同一 parent 分别 seal，sync 后形成两个 maximal heads；禁止 arrival/hash/revision-number/LWW 选胜者。显式 resolution 必须以完整当前多 head set 为 parents 走同一 producer，并产生唯一 multi-parent successor。P 已提交而 publication/transport 失败时，重试只从 D8PresentationPolicyOutboxItem/1 解引用并重放 exact retained record bytes；不得重新编码、重新分配 ChangeId、重算 proposal 或改 head。receiver 只有在 strict canonical record/address/pin、activationChangeId、原 EffectManifest/receipt/ChangeRecord decision association、parent ancestry 与 head-set continuity 全部验证后才 admission；相同 hash、trusted sender、布尔“已验证”或 provider latest 都不能替代。final P 前 revocation 使该 plan 按原 authorization 错误停止且零 ChangeId；P 后 revocation 只影响当前可见/后续管理资格，不改历史 decision/outbox。这里没有第二 Policy store、ledger、CAS 或迁移层。


sync/admission 必须验证每个 immutable record 的 domain/canonical bytes、activationChangeId、exact retained ChangeRecord/receipt/EffectManifest presentation_policy_change association、ancestry 与 current head-set stamp continuity，再把它放入 portable head graph。record 的 historical decoder 未知、activation decision 无法证明或 head-set observation 不连续时只能 unavailable，绝不能 current。conflicting heads 始终保持 conflict，直到授权的显式 multi-parent successor 解决。只有所有 descendant/recovery last-reference 都不再需要某 non-head record 时才可 compaction。device-local preference 或 host default 永远不能替代这个 Workspace-wide record。

run-in decision 是条件依赖：role_conflict 使用 explicit+Separate，不读 policy；显式 separate 不读 policy；显式 run-in 也不读 policy，并且只有既有 explicit-role semantic-adjacency body eligible 时才 RunIn，否则 no_eligible_body。两个 role 都没有时，若无 eligible body 同样 no_eligible_body；只有满足 implicit-default physical-adjacency 的 body 才使用 workspace_default，因此才要求 current one-head policy binding。policy missing/conflicted 只使这个 default-dependent presentation unavailable。

D8 cache identity 包含完整 D8DocumentRenderBinding/1。policy 改变只失效 presentation.kind=workspace_default 且原 policy binding 已不 current 的 cache；显式 role、no-body、conflict fallback 在其 source/product dependency 不变时不因无关 policy 修改而失效。


## 6.5 ExportPlan/3 / PublicationReceipt/3

本节冻结真实记录的 /3 前身 shape，并定义 §6.6 的 /4 successor 对应成员继续继承的共享 export 规则。fresh unseen current export 使用 /4。/3 decoder、token tag、bytes、pins、confirmation 与 saved/planned/unknown recovery 都保持精确原义，绝不原地扩宽。除非 §6.6 明确扩展某个封闭成员集合，/4 继续继承 §6.5 的 generation-policy、受控名称、canonical ordering、template/route/style/document-render、proof/pin 与 publication durability 规则。

current /3 继续使用 fixed-parent ExportInputCatalog/2、ExportContentSelection/1、ExportProjection/1、ExportLossReport/1。per-plan generation choices 本来就是有限的 immutable Plan 参数，因此直接进入闭合 Plan；不建立独立 generation-policy registry，也不存在 hash-only descriptor authority。

```text
D9ExportPlanToken/3 := D6 Token tagged "d9_export_plan/3"
D9PublicationToken/3 := D6 Token tagged "d9_publication/3"

D9ExportInputDomain/1 =
  "document"|"native_table"|"node_collection"|"query_rows"|"query_json"|"resource"

D9ExportTarget/1 =
    {kind:"asciidoc_source"}
  | {kind:"resource_exact"}
  | {kind:"html",profileId:text}
  | {kind:"pdf",profileId:text}
  | {kind:"docx",profileId:text}
  | {kind:"odt",profileId:text}
  | {kind:"csv_utf8"}
  | {kind:"tsv_utf8"}
  | {kind:"xlsx",profileId:text}
  | {kind:"ods",profileId:text}
  | {kind:"query_json"}

D9ExportTemplateBinding/1 = {
  inputIndex:Counter,
  profileId:text,
  profileVersion:text,
  pin:PinRef/2
}

D9ExportRouteStepBinding/1 = {
  step:Counter,
  providerId:text,
  providerVersion:text,
  inputProfileId:text,
  outputProfileId:text,
  optionsSha256:"sha256:<64 lowercase hex>",
  stepBudget:BudgetBinding/1,
  evidencePins:[PinRef/2...]
}

D9ExportRouteBinding/1 = {
  routeId:text,
  routeRevision:Token,
  profileId:text,
  profileVersion:text,
  steps:[D9ExportRouteStepBinding/1...]
}

D9ExportStyleBundleBinding/1 = {
  styleBundleId:text,
  version:Counter,
  pin:PinRef/2
}

D9TemplateBindingChoice/1 = {
  templatePath:text,
  inputIndex:Counter
}

D9MissingPolicyChoice/1 = {
  templatePath:text,
  action:"empty"
}

D9ImageSizeChoice/1 = {
  resource:ResourceRef,
  widthMicrometres:Counter,
  heightMicrometres:Counter
}

D9LayoutChoice/1 = {
  resource:ResourceRef,
  choice:"preserve_aspect_within_box"|"use_exact_dimensions"
}

D9NativeTableSelector/1 = {
  inputIndex:Counter,
  tableLocator:DocumentElementLocator,
  tableQualifier:
      {kind:"unqualified"}
    | {kind:"title",text:text}
    | {kind:"occurrence",title:text|null,ordinal:Counter},
  columnQualifier:
      {kind:"leaf",text:text}
    | {kind:"header_suffix",segments:[text...]}
    | {kind:"occurrence",segments:[text...],ordinal:Counter}
}

D9NativeTableTokenBinding/1 = {
  setName:text,
  columnName:text,
  selector:D9NativeTableSelector/1
}

D9ExportGenerationPolicy/1 =
    {kind:"none"}
  | {kind:"render",
     bindingChoices:[D9TemplateBindingChoice/1...],
     missingPolicy:[D9MissingPolicyChoice/1...],
     imageSizes:[D9ImageSizeChoice/1...],
     layoutChoices:[D9LayoutChoice/1...],
     nativeTableBindings:[D9NativeTableTokenBinding/1...]}

D9DocumentRenderBinding/1 = {
  ownerNodeRef:NodeRef,
  sourceObservation:SourceObservation/1,
  documentSnapshotPin:PinRef/2,
  semanticQualification:ManagedDocumentSemanticQualification/1,
  presentation:D8PresentationDecision/1
}

D9ExportDestinationIntent/1 =
    {kind:"external_bundle",destinationHandle:Token,basename:text,createOnly:true}
  | {kind:"server_download",downloadToken:Token,basename:text}
  | {kind:"resource_handoff",ownerNodeRef:NodeRef,resourceName:text}

D9ExportLossChoice/1 = {
  lossKey:Counter,
  choice:"accept_loss"|"reject"
}

D9ExportConfirmation/1 = {
  planToken:D9ExportPlanToken/3,
  lossChoices:[D9ExportLossChoice/1...]
}

D9ControlledRelativeOutputName/1 := text satisfying the §6.5 closed output-name domain

D9ExportStagedOutput/1 = {
  name:D9ControlledRelativeOutputName/1,
  byteLength:Counter,
  sha256:"sha256:<64 lowercase hex>",
  pin:PinRef/2
}

D9PublishedOutput/1 = {
  name:D9ControlledRelativeOutputName/1,
  byteLength:Counter,
  sha256:"sha256:<64 lowercase hex>"
}

ExportPlan/3 = {
  kind:"d9_export_plan",version:3,
  planToken:D9ExportPlanToken/3,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  principalAudienceToken:Token,authorizationGeneration:Token,
  inputDomain:D9ExportInputDomain/1,
  inputCatalog:ExportInputCatalog/2,
  contentSelection:ExportContentSelection/1,
  projection:ExportProjection/1,
  documentRenderBinding:D9DocumentRenderBinding/1|null,
  templateBinding:D9ExportTemplateBinding/1|null,
  routeBinding:D9ExportRouteBinding/1|null,
  styleBundles:[D9ExportStyleBundleBinding/1...],
  generationPolicy:D9ExportGenerationPolicy/1,
  target:D9ExportTarget/1,
  initialLossReport:ExportLossReport/1,
  outputBudget:BudgetBinding/1,
  destination:D9ExportDestinationIntent/1,
  observationScope:ObservationScope/2,
  dependencyProof:DependencyProof/3,
  observationProof:ObservationProof,
  recoveryPins:[PinRef/2...],
  evidencePins:[PinRef/2...],
  stagedOutputs:[D9ExportStagedOutput/1...]
}

PublicationReceipt/3 = {
  kind:"d9_publication_receipt",version:3,
  publicationToken:D9PublicationToken/3,
  planToken:D9ExportPlanToken/3,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  outputs:[D9PublishedOutput/1...],
  lossReport:ExportLossReport/1,
  lossChoices:[D9ExportLossChoice/1...],
  target:D9ExportTarget/1,
  templateBinding:D9ExportTemplateBinding/1|null,
  routeBinding:D9ExportRouteBinding/1|null,
  styleBundles:[D9ExportStyleBundleBinding/1...],
  generationPolicy:D9ExportGenerationPolicy/1,
  presentation:D8PresentationDecision/1|null,
  destinationDisplay:text
}
```

D9ControlledRelativeOutputName/1 是非空 Unicode scalar 字符串，其原始 UTF-8 编码就是协议名称。唯一分隔符是 U+002F SOLIDUS（“/”）。名称必须由一个或多个非空 component 组成，不能以“/”开头或结尾，也不能出现空 component。component 不能是“.”或“..”；不能含 U+0000..U+001F、U+007F、U+005C（反斜杠）以及 ASCII 字符 : * ? " < > |；末尾不能是 U+0020 SPACE 或 U+002E FULL STOP。因此 Unix rooted name、重复分隔符、Windows drive/root 形式与 UNC/backslash 形式都直接非法，不交给 host 决定 alias。除这些排除项外允许 Unicode scalar，本协议不是 ASCII-only。

设备名检查只取每个 component 在第一个 U+002E 前的 stem，把 ASCII A-Z 折成 a-z，然后拒绝 con、prn、aux、nul、clock$、conin$、conout$、com1..com9 与 lpt1..lpt9。若 ASCII 字母部分相同而末位分别是 U+00B9、U+00B2、U+00B3，也拒绝 COM¹/COM²/COM³ 与 LPT¹/LPT²/LPT³。host device table 或 locale 不能扩张或削弱这个协议检查；destination capability 只能额外拒绝。

PortableAlias(name) 只用于冲突拒绝。先按“/”拆分已经合法的名称；每个 component 依次执行 Unicode 15.1.0 NFC、Unicode 15.1.0 CaseFolding.txt 的 full default case fold（使用 C/F mapping，不使用 locale-specific T mapping），再执行一次 NFC，最后用“/”连接。normalization 算法固定为 Unicode 15.1.0 的 Unicode Standard Annex #15。实现必须得到与这些固定版本 Unicode data 相同的 scalar 结果；ambient locale 或未固定版本的 host Unicode library 不能作为 authority。PortableAlias 永远不替换 stored name，也不参加原始 UTF-8 排序。

同一 fresh-current bundle 中，exact duplicate name 非法；不同名称若 PortableAlias 相等也非法。把 PortableAlias 看成 component vector：一个成员的 vector 若是另一个成员的 proper prefix，就形成 file/directory conflict，必须拒绝。根级 loss-report.json 与 manifest.json 是系统保留成员，由 Core 精确生成；所有 dataFile 都必须与二者做 exact name、alias 与 prefix 检查。因此 Report.txt/report.txt 冲突，canonical-equivalent 的 NFC/NFD 拼写冲突；dir/file 是合法嵌套形式，而包含 dir\\file 的名称直接非法。真实文件系统若发现更多 collision，可以拒绝 destination，但不能改变这套 portable relation。

对真实记录的 ExportPlan/3，stagedOutputs 仍是完整 canonical staged member set：全部已验证 dataFiles 加 exact loss-report.json 与 manifest.json，并各自绑定真实 bytes/pin。PublicationReceipt/3.outputs 继续表示该 /3 Plan 实际发布 bytes 的同一完整 name/digest 集合。fresh ExportPlan/4 继承相同的完整 staged-member 规则；PublicationReceipt/4 与 D9PrintReceipt/1 使用 /4 Plan 中对应的受控输出成员。external manifest 的 reportFile 必须精确命名 loss-report.json，其 length/digest 与 staged/published member 相等；manifest.json 继续继承不自哈希规则。server-download 与 Resource-handoff 只能按这个 exact controlled name 选择原 staged dataFile bytes，不能合成替代名称。安全、alias、prefix 或 reserved 失败属于 invalid preparation，绝不能变成 ExportLossChoice。

exact-source/resource-exact/query-json plan 强制 generationPolicy={kind:"none"}、templateBinding=null、routeBinding=null、documentRenderBinding=null，projection 不得含 renderer-derived value。即使 template/provider/generation registry unavailable，它们仍有唯一可编码正向路径，因为本合同根本不消费这些依赖。rendered HTML/PDF/DOCX/ODT 与有限 table output 使用 generationPolicy.render；某一 policy 类未使用时对应 array 可以为空。

D9TemplateBindingChoice/1 只在真实 ambiguous binding 且 authorized template/profile 允许用户选择时出现；inputIndex 必须选择 exact frozen catalog item，不能借 choice 新增 read。D9MissingPolicyChoice/1 只能命中已存在的 exact template path，且 authorized projection value 确为 none；action=empty 不能掩盖 unknown path、unreadable input、type error 或 unavailable schema。imageSizes 按 ResourceRef key 排序唯一，两个 dimension 均为 positive，resource 必须等于 authorized resource catalog input。layoutChoices 同样按 ResourceRef key 排序唯一，必须存在相同 resource 的 imageSizes，而且只在 fixed Templates rule 真正要求 explicit layout choice 时出现。preserve_aspect_within_box 取不超过两个 chosen dimension 的最大同宽高比尺寸；use_exact_dimensions 使用两个 chosen dimension 并记录 required layout loss。

/3 前身为每个 set-like array 定义唯一 comparator contract；fresh ExportPlan/4 对所有未改变的对应成员继续继承这些 comparator。routeBinding.steps 的 array position i 必须对应 step=i，也就是完整 0..N-1；每个 step.evidencePins 按 pinToken 排序且唯一。styleBundles 按 styleBundleId 的 UTF-8 bytes 排序且唯一，同一 ID 若 version/pin 不同就是 conflict。stagedOutputs 与 PublicationReceipt/3-/4 outputs 必须先满足 D9ControlledRelativeOutputName/1 以及上述完整 exact-name、PortableAlias、file/directory-prefix、reserved-system-name 冲突规则，然后才按 stored protocol name 的原始无符号 UTF-8 bytes 字典序排序并保持唯一；排序本身不执行 Unicode normalization、case folding、locale collation、host/path-library collation 或 separator rewriting，完全相同 byte prefix 的较短者在前。exact duplicate 非法，portable alias/prefix/reserved conflict 非法，同一 exact name 若 byteLength/sha256/pin 不同则是 integrity conflict，绝不 LWW。generationPolicy.render.bindingChoices 与 missingPolicy 继续共用 templatePath Unicode-scalar comparator：按 exact decoded scalar value 字典序比较，normalization=none、case-sensitive、禁止 locale/case folding，完全相同 scalar prefix 的较短者在前。canonical-equivalent 但 scalar sequence 不同的 template path 仍可保持不同 key，除非既有 Templates 规则另行拒绝；这个 template comparator 与 output-name alias relation 明确分离。各 array 分别按自己的 comparator 排序且唯一；同一 exact templatePath 不能有两个 inputIndex/action body，也不能同时出现在 bindingChoice 与 missingPolicy。nativeTableBindings 按 controlled ASCII (setName,columnName) tuple 排序且唯一；同 key 不同 token/selector 必须失败。imageSizes/layoutChoices 的 ResourceRef、lossChoices 的 lossKey、plan evidencePins/recoveryPins 的 pinToken 继续原 comparator，implementation 不得自选替代顺序。

D9 fresh prepare 必须 strict-decode 全部 choices，并验证完整 current output-name 域与 bundle conflict set，再检测 duplicate/conflicting keys，最后在冻结任何 ExportPlan/4 bytes、/4 plan token、protected pin、staged manifest 或 confirmation basis 之前执行唯一 canonical sort。因此同一合法 set 的任意 permutation 必须得到 byte-for-byte 相同 Plan。duplicate key 即使 body byte-equal 也因集合要求唯一而拒绝；body 不同还构成 integrity conflict。冻结的 fresh ExportPlan/4 或 PublicationReceipt/4 必须已经 canonical，且全部 current output name 必须满足 D9ControlledRelativeOutputName/1。真实记录的 ExportPlan/3、PublicationReceipt/3、recovery record 或 received /3 record 继续使用精确 /3 decoder 与 protected bytes；admission 绝不能通过排序、normalization、rename、repin 或 re-encode 把它修成 /4。ExportPlan/1-/2 与 PublicationReceipt/1-/2 同样继续原 decoder、bytes 与 name rule。fresh-current predicate 不追溯迁移任何已记录 /1-/3 value。receipt 的 route/style/generation-policy selection 与 protected Plan byte-equal，outputs 继续使用上面的独立 raw-name comparator。

D9ExportTemplateBinding/1.inputIndex 必须选择 inputCatalog.items[index] 中恰一个 payload.kind=template 项；pin 与该 item.pin byte-equal。profileId/profileVersion 必须选择能成功解码这些 exact bytes 的 accepted decoder；profile mismatch/unavailable 是 template unavailable，不得 fallback。不存在第二 template registry 或 filename lookup。

nonnull route 的 steps 必须非空并从 0 连续编号。每一步的 input/output profile 都必须被该 exact provider/version 接受。
terminal output profile 必须与 target 完全一致。
对于六种文档或 Office 目标 html/pdf/docx/odt/xlsx/ods，目标配置必须使用 target.profileId 中已经冻结的 profile。
csv_utf8 的 terminal profile 固定为 "text/csv-utf8/1"，不得改成其它值。
tsv_utf8 的 terminal profile 固定为 "text/tsv-utf8/1"，同样不得改成其它值。
routeBinding.profileId/profileVersion 必须指向包含这条精确 terminal chain 的 accepted route profile。target 与 terminal 不匹配时直接拒绝 prepare，不能自动改换 route。

document render 时，documentSnapshotPin 必须选择 ownerNodeRef/sourceObservation 对应的 exact D2-Document-Snapshot/3 bytes。snapshot evaluation 的 root SourceObservation 与 ManagedDocumentSemanticQualification.sourceObservation 都必须和 sourceObservation 逐字相等。
document_format 必须为 current；§4.4 的全部 include/environment/dependency 也必须在最终 export read barrier 重新验证。presentation 必须是实际生成 staged output 时消费的 D8PresentationDecision/1。
explicit/no-body/conflict-fallback decision 不包含 Workspace policy pin；只有 workspace_default 才携带实际消费的 current D8 policy binding。后续 source/include/policy 变化不能修改或重新渲染已经 prepared 的 Plan；fresh prepare 才读取新的 current binding。

Pins(X) 表示只沿 closed typed value X 中 schema 真正声明为 PinRef/2 的 member（或含这些 member 的 closed type）递归取得 PinRef/2；text/digest/handle 都不算。对真实记录的 ExportPlan/3，推导精确 /3 evidencePins 时不得递归 plan 自身的 evidencePins member；其 evidencePins 继续严格等于下面这些前身成员的 pinToken 排序唯一并集。fresh ExportPlan/4 继承同一递归 PinRef 规则，只通过 §6.6 明确列出的 typed projection 与 viewRenderBinding 成员扩展该并集：

```text
Pins(inputCatalog) +
Pins(documentRenderBinding) +
Pins(templateBinding) +
Pins(routeBinding) +
Pins(styleBundles) +
Pins(dependencyProof) +
Pins(observationProof) +
Pins(stagedOutputs) +
recoveryPins
```

recoveryPins 自身按 pinToken 排序唯一，只允许 fixed-parent recovery contract 实际要求、且上列 typed path 尚未覆盖的 original route/rule/destination/unknown-publication pins；它不是 extension point。漏掉任何 reachable typed pin、加入 unrelated pin，或用不同 pin 重复同一 semantic evidence，Plan 都 invalid。这样 FC4A-EXP-05 的 byte equality 有唯一可执行 derivation，而 evidencePins 不成为第二 authority。

### 6.5.1 Native table → Office SET/COLUMN naming

native table dataset name 只能从已授权的 D2TableBlock/3 product projection 与 Office template 推导，ordinary Node 不增加任何 export configuration。
文本比较必须使用精确 Unicode scalar sequence：不做 normalization、区分大小写并保留 whitespace。因此 CJK、RTL 与 combining-character 都遵循同一条不依赖 host 的规则。

对每个 physical table column，按 head-row source order 构造 headerPath。一个 head cell 的 [columnStart,columnStart+colspan) 覆盖该 column 时，其 semantic inline text 贡献一次；rowspan 在后续 logical row 不重复同一 cell。空 header cell 贡献 empty string。column leaf 是最后一个 segment，没有 head segment 时是 empty。table title 是完整 semantic title text 或 null。

selector 只允许按以下固定次序选择最短唯一 qualifier：

1. candidate tables 中 leaf text alone；
2. 以 leaf 结尾的最短 headerPath suffix；
3. 同一 header suffix 加 exact table title；
4. 在同一组 facts 基础上，再加入 zero-based table occurrence，用于区分 title/path 完全相同的候选；如仍需要，再加入 zero-based column occurrence，用于区分同一 table 内 full header path 完全相同的列。

每一级：0 match 为 mapping_required；1 个即选中；>1 才进入下一层。最终 occurrence 层仍不是1个时 ambiguous_binding。禁止 first/last winner、猜 suffix、current UI order、filename、rowHandle、path guess 或强迫修改 author source。同一 repeat SET 的全部 binding 必须解析到同一 table/rowset。

SET/COLUMN external name 继续使用现有 ASCII [a-z][a-z0-9_]{0,63}。unqualified native-table SET 固定 "native_table"。leaf-only COLUMN 若本身满足 ASCII grammar 且 unique，直接用 exact leaf。所有 qualified 或 non-ASCII selector 使用唯一 deterministic ASCII token：；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。

```text
SET    = "nt_" + base32hex_lower(SHA-256(
           UTF8("D9-Native-Table-Set/1") || NUL ||
           D3-CJ/3(tableQualifier)))
COLUMN = "nc_" + base32hex_lower(SHA-256(
           UTF8("D9-Native-Table-Column/1") || NUL ||
           D3-CJ/3({tableQualifier,columnQualifier})))
```

base32hex_lower 是完整256-bit digest 的52字符 lowercase RFC4648 base32hex，无 padding，因此两个名字都满足63-byte grammar。Plan 保存 D9NativeTableTokenBinding/1，所以 token 不作为 identity，也不能靠反推猜 selector；Core 从 current product projection 重新计算全部 candidate token，并要求 stored selector 是唯一 match。两个 non-byte-equal selector 若 digest collision，返回 ambiguous_binding/integrity failure，不能视为相等。

后来新增 same-name column/table 可以让旧 short selector 变成不唯一；fresh prepare 必须 ambiguous_binding，直到 template 使用新 required qualified token。old prepared ExportPlan 已冻结 table projection/selector/staged bytes，因此保持 immutable。

ExportInputCatalog/2、ExportContentSelection/1、ExportProjection/1、ExportLossReport/1 保持 fixed-parent 的封闭 shape 与原语义。
D9NativeTableTokenBinding/1 只记录既有 dataset SET/COLUMN name 的推导结果，不新增第二套 dataset schema。每个 native-table dataset column/cell 的 projection origin 必须指向 binding 选择的同一个 inputIndex/tableLocator；table grid facts 只取 D2TableCell/3。

D9ExportConfirmation/1 继续是记录型 ExportPlan/3 的精确 confirmation：不得改变 catalog、projection、route、target、destination、generation policy、report、budget、presentation 或 staged bytes。fresh ExportPlan/4 使用 §6.6 的 D9ExportConfirmation/2；它继承相同不可变性，并额外冻结新的 Annotation/View selection 与 renderer 成员。lossChoices 继续按 lossKey 排序唯一，并完整覆盖 requires_choice/blocking。PublicationReceipt/3 与自己的 protected /3 Plan/confirmation byte-match；PublicationReceipt/4 与 protected /4 Plan/Confirmation2 byte-match，并记录实际 published output digest。

真实记录的 ExportPlan/3 只使用 d9_export_plan/3 token，真实记录的 PublicationReceipt/3 使用 d9_publication/3。fresh unseen ExportPlan/4 使用 d9_export_plan/4，fresh external publication 使用 d9_publication/4，print-only delivery 使用 d9_print/1。lookup、inspect、confirmation 与 unknown recovery 必须先按 token tag 分派，再严格解码 record.version。ExportPlan/1-/2-/3 与 PublicationReceipt/1-/2-/3 都保留各自记录的 tag、精确 bytes、pins、permissions、confirmation、unknown-publication state 与 recovery；任何已记录 token 都不得 repin 或重新编码成 /4。


## 6.6 fresh-current D9 export /4 继任族

引入本继任族后，上述 /3 export family 冻结为历史/现有恢复输入，绝不原地扩宽。fresh unseen export 使用下面的 /4 family。没有改变 closed member/union 的嵌套类型继续沿用原 decoder；只有闭合集合真实变化的类型才升版。

~~~text
D9ExportPlanToken/4 := D6 Token tagged "d9_export_plan/4"
D9PublicationToken/4 := D6 Token tagged "d9_publication/4"
D9PrintToken/1 := D6 Token tagged "d9_print/1"

D9ExportInputDomain/2 =
  "document"|"native_table"|"node_collection"|"query_rows"|"query_json"|"resource"
  | "annotation"|"view"

D9AnnotationContentInput/1 = {
  kind:"annotation_content",
  annotationRef:AnnotationRef,
  sourceObservation:SourceObservation/1,
  annotationRevisionToken:AnnotationRevisionToken/1,
  record:PortableAnnotationRecord/4,
  recordPin:PinRef/2,
  body:D8AnnotationBodyRead/1,
  targetResolution:"exact"|"mapped"|"candidate"|"ambiguous"|"orphaned"|"unavailable"
}

ExportInputCatalog/3 = {
  version:3,
  items:[{index:Counter,label:text,payload:D9ExportCatalogPayload/3}...]
}

D9ExportCatalogPayload/3 =
    {kind:"document",sourceVersion:SourceVersion/2,observation:SourceObservation/1,pin:PinRef/2}
  | {kind:"resource",sourceVersion:SourceVersion/2,observation:SourceObservation/1,pin:PinRef/2}
  | {kind:"field",ownerVersion:SourceVersion/2,ownerObservation:SourceObservation/1,
     fieldId:FieldId,entries:[Entry/1...]}
  | {kind:"annotation_index",ownerVersion:SourceVersion/2,
     targets:[D9EntityVersionAddress/2...]}
  | D9AnnotationContentInput/1
  | {kind:"template",origin:D9ExportTemplateOrigin/1,pin:PinRef/2}
  | {kind:"query_result",result:D7ResultPin}


D9EntityVersionAddress/2 = {
  ref:EntityRef,
  sourceVersion:SourceVersion/2
}

D9ExportTemplateOrigin/1 =
    {kind:"artifact",source:SourceArtifact}
  | {kind:"resource",sourceVersion:SourceVersion/2,observation:SourceObservation/1}
  | {kind:"route_asset",routeRevision:Token,assetId:text,assetVersion:text}

D9ExportBindingProjection/1 = {
  path:text,
  value:RenderSnapshot,
  origins:[ExportInputLocation/2...]
}

D9ExportCellProjection/1 = {
  value:RenderSnapshot|null,
  origins:[ExportInputLocation/2...]
}

D9ExportColumnProjection/1 = {
  name:text,
  valueKind:text,
  nullable:Boolean
}

D9ExportRowProjection/1 = {
  cells:[D9ExportCellProjection/1...]
}

D9ExportDatasetProjection/1 = {
  name:text,
  columns:[D9ExportColumnProjection/1...],
  rows:[D9ExportRowProjection/1...]
}

D9AnnotationBackupFile/1 = {
  format:"weftext.annotation-backup",
  version:1,
  records:[PortableAnnotationRecord/4...]
}

D9AnnotationSelection/1 = {
  inputIndex:Counter,
  mode:"portable_backup"|"review_bundle_r6",
  includeSourceHistory:Boolean,
  includeTargetContext:Boolean
}

ExportContentSelection/2 = {
  version:2,
  bodyInput:Counter|null,
  bibliographyInput:Counter|null,
  annotationInputs:[D9AnnotationSelection/1...],
  viewInput:Counter|null
}

D9AnnotationDisclosureProjection/1 =
    {state:"not_requested"}
  | {state:"unavailable"}
  | {state:"disclosed",
     fragments:[{value:RenderSnapshot,origins:[ExportInputLocation/2...]}...]}

D9AnnotationExportProjection/1 =
    {kind:"portable_backup",inputIndex:Counter,recordPin:PinRef/2}
  | {kind:"review_bundle_r6",inputIndex:Counter,
     purpose:"comment"|"mark"|"suggestion",
     semanticBody:text|null,
     appearance:AnnotationAppearance/1|null,
     labels:[text...],
     reviewState:"open"|"resolved"|"not_applicable",
     suggestion:Suggestion/3|null,
     replyTo:AnnotationRef|null,
     creator:AnnotationActorSnapshot/1,
     authoredAt:AnnotationTimeSnapshot/2,
     lastEditor:AnnotationActorSnapshot/1,
     editedAt:AnnotationTimeSnapshot/2,
     targetResolution:"exact"|"mapped"|"candidate"|"ambiguous"|"orphaned"|"unavailable",
     sourceHistory:D9AnnotationDisclosureProjection/1,
     targetContext:D9AnnotationDisclosureProjection/1}

D9ViewAssetBinding/1 = {
  role:"font"|"color_profile"|"page_profile"|"accessibility_profile",
  assetId:text,
  assetVersion:text,
  pin:PinRef/2
}

D9ViewRendererBinding/1 = {
  rendererId:text,
  rendererVersion:text,
  profileId:text,
  profileVersion:text,
  layout:"metric"|"bar"|"line"|"scatter"|"pie"|"heatmap",
  targetKind:"docx"|"xlsx"|"pdf"|"svg"|"png"|"print",
  accessibilityProfile:"same-data-table-alt-text/1",
  assets:[D9ViewAssetBinding/1...],
  evidencePins:[PinRef/2...]
}

D9ViewOutputScope/1 := "complete_data"

D9ViewRenderBinding/1 = {
  resultInput:Counter,
  querySemanticSha256:"sha256:<64 lowercase hex>",
  snapshotResultSha256:"sha256:<64 lowercase hex>",
  resultEpoch:Token,
  authorizationGeneration:Token,
  viewSpec:ViewSpec/1,
  viewSpecSha256:"sha256:<64 lowercase hex>",
  renderer:D9ViewRendererBinding/1,
  outputScope:D9ViewOutputScope/1
}

D9ViewExportProjection/1 = {
  resultInput:Counter,
  viewSpecSha256:"sha256:<64 lowercase hex>",
  outputScope:D9ViewOutputScope/1,
  accessibility:"same_data_table_required"
}

ExportProjection/2 = {
  version:2,
  bindings:[D9ExportBindingProjection/1...],
  datasets:[D9ExportDatasetProjection/1...],
  annotations:[D9AnnotationExportProjection/1...],
  view:D9ViewExportProjection/1|null
}

ExportInputLocation/2 =
    {kind:"input",inputIndex:Counter}
  | {kind:"source_range",inputIndex:Counter,start:Counter,end:Counter}
  | {kind:"annotation",inputIndex:Counter,targetIndex:Counter}
  | {kind:"annotation_record",inputIndex:Counter}
  | {kind:"annotation_body",inputIndex:Counter}
  | {kind:"annotation_disclosure",inputIndex:Counter,
     scope:"source_history"|"target_context",fragment:Counter}
  | {kind:"query_cell",inputIndex:Counter,table:"rows"|"nodes"|"edges",
     row:Counter,column:Counter}
  | {kind:"query_scalar",inputIndex:Counter}
  | {kind:"template_range",inputIndex:Counter,part:text,
     elementPath:[Counter...],start:Counter,end:Counter}
  | {kind:"view",inputIndex:Counter,
     aspect:"chart"|"accessible_table"|"font"|"color"|"page"|"alt_text"}

ExportLossLocation/2 =
    ExportInputLocation/2
  | {kind:"binding",path:text}
  | {kind:"dataset_cell",set:text,row:Counter,column:Counter}
  | {kind:"block",path:text,blockIndex:Counter}

ExportLossReport/2 = {
  format:"weftext.export-loss",version:2,
  planToken:D9ExportPlanToken/4,
  inputs:[{index:Counter,label:text,kind:text}...],
  items:[{lossKey:Counter,feature:text,locations:[ExportLossLocation/2...],
          effect:text,severity:"notice"|"requires_choice"|"blocking",
          allowedChoices:["accept_loss"|"reject"...]}...]
}

D9ExportConfirmation/2 = {
  version:2,
  planToken:D9ExportPlanToken/4,
  lossChoices:[D9ExportLossChoice/1...]
}

D9ExportTarget/2 =
    {kind:"asciidoc_source"}
  | {kind:"resource_exact"}
  | {kind:"annotation_backup"}
  | {kind:"html",profileId:text}
  | {kind:"pdf",profileId:text}
  | {kind:"docx",profileId:text}
  | {kind:"odt",profileId:text}
  | {kind:"csv_utf8"}
  | {kind:"tsv_utf8"}
  | {kind:"xlsx",profileId:text}
  | {kind:"ods",profileId:text}
  | {kind:"query_json"}
  | {kind:"svg",profileId:text}
  | {kind:"png",profileId:text}
  | {kind:"print",profileId:text}

D9ExportDestinationIntent/2 =
    {kind:"external_bundle",destinationHandle:Token,basename:text,createOnly:true}
  | {kind:"server_download",downloadToken:Token,basename:text}
  | {kind:"resource_handoff",ownerNodeRef:NodeRef,resourceName:text}
  | {kind:"print",printIntentToken:Token}

D9ExportBundleManifest/2 = {
  format:"weftext.export-bundle",version:2,
  planToken:D9ExportPlanToken/4,
  dataFiles:[D9PublishedOutput/1...],
  reportFile:D9PublishedOutput/1
}

ExportPlan/4 = {
  kind:"d9_export_plan",version:4,
  planToken:D9ExportPlanToken/4,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  principalAudienceToken:Token,authorizationGeneration:Token,
  inputDomain:D9ExportInputDomain/2,
  inputCatalog:ExportInputCatalog/3,
  contentSelection:ExportContentSelection/2,
  projection:ExportProjection/2,
  documentRenderBinding:D9DocumentRenderBinding/1|null,
  viewRenderBinding:D9ViewRenderBinding/1|null,
  templateBinding:D9ExportTemplateBinding/1|null,
  routeBinding:D9ExportRouteBinding/1|null,
  styleBundles:[D9ExportStyleBundleBinding/1...],
  generationPolicy:D9ExportGenerationPolicy/1,
  target:D9ExportTarget/2,
  initialLossReport:ExportLossReport/2,
  outputBudget:BudgetBinding/1,
  destination:D9ExportDestinationIntent/2,
  observationScope:ObservationScope/2,
  dependencyProof:DependencyProof/3,
  observationProof:ObservationProof,
  recoveryPins:[PinRef/2...],
  evidencePins:[PinRef/2...],
  stagedOutputs:[D9ExportStagedOutput/1...]
}

PublicationReceipt/4 = {
  kind:"d9_publication_receipt",version:4,
  publicationToken:D9PublicationToken/4,
  planToken:D9ExportPlanToken/4,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  outputs:[D9PublishedOutput/1...],
  lossReport:ExportLossReport/2,
  lossChoices:[D9ExportLossChoice/1...],
  target:D9ExportTarget/2,
  templateBinding:D9ExportTemplateBinding/1|null,
  routeBinding:D9ExportRouteBinding/1|null,
  styleBundles:[D9ExportStyleBundleBinding/1...],
  generationPolicy:D9ExportGenerationPolicy/1,
  viewRenderBinding:D9ViewRenderBinding/1|null,
  presentation:D8PresentationDecision/1|null,
  destinationDisplay:text
}

D9PrintReceipt/1 = {
  kind:"d9_print_receipt",version:1,
  printToken:D9PrintToken/1,
  planToken:D9ExportPlanToken/4,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  output:D9PublishedOutput/1,
  lossReport:ExportLossReport/2,
  lossChoices:[D9ExportLossChoice/1...],
  viewRenderBinding:D9ViewRenderBinding/1,
  destinationDisplay:text
}
~~~

### 6.6.1 Plan/4 跨字段准入与 canonical collection

上面的 /4 JSON shape 都是封闭类型，但以下跨字段关系同样属于严格准入。即使 evidencePins 完整，也不能替代这些关系约束。

全部导出路径共用同一段公共入口门禁：（1）严格封闭解码 wire/tag/version/union/member/null，授权前仅返回不披露受保护信息的错误；（2）D1 静态 surface/release capability；（3）现任 audience/entry authorization；（4）适用的 Workspace/domain/P 连续性。完成这些步骤后，Core 才按原操作的真实状态分派。若属于已有 saved/planned/unknown 责任，或 inspect/confirmation/publish/state/receipt，则按真实 token tag、record version、原控制存储与现任披露资格查找原始且经认证的受保护 ExportPlan，再复验原 pin/dependency/currentness、confirmation 与 owner 恢复责任，绝不改写记录字节。若是 fresh unseen prepare，此时原本就没有 ExportPlan/4；凭当前权限启动私有候选构造，不能把查找一份已存在的受保护 Plan 当成首次准备的前置条件。通过原资格门禁后，两条路径仍必须验证所有适用的 D9 封闭关系、canonical collection、route、loss 与 budget，才可返回受保护输出。已失权且输入又有坏 index/layout 时，必须先给出原不披露的 not_visible/reset，不可泄露目录、布局、profile 或隐藏 dependency 详情；封闭解码/version 和 D1 静态拒绝仍排在现任授权之前。

首次 unseen prepare 先经过共同入口的现任授权，再按 D9ExportInputDomain/2 确定原有潜在 ObservationScope；在读取任何敏感目录或来源前必须固定该范围。Core 只取得所选域实际消费的同一 cut 原 owner 输入，绝不能默认要求 D8 Annotation 或 D7 Query。现任八个封闭输入域的完整分派如下（不新增公共 wire，也不另建 catalog 权威）：

| inputDomain | 实际来源 owner 与所消费 catalog 分支 | Query、渲染器及最小授权范围 | 必须拒绝的最小反例 |
| --- | --- | --- | --- |
| document | 原 D6/D2 Document source_read；完整 SourceVersion/2、相符的现任 SourceObservation/1 与精确 Document 原文字节 PinRef/2；只有真实渲染文档才消费 D2 snapshot 和 D8 presentation | asciidoc_source 不要求 Query 或 Annotation；generationPolicy=none 不需要 template、route、renderer；显式选正文或参考文献仍遵守原资格 | 缺 source_read、Observation 过期或 source pin 错误均拒绝；不得虚构 Query、补空正文或扩读整个 Workspace |
| native_table | 经原 Document source/pin 和精确 table range 获权的 D2TableBlock/3、D2TableCell/3；只在实际选用时加入窄 D4 Field 或 Resource 输入 | 保留 D2 table/Field 原规则，不额外强加全 Workspace Query；只有确实选用 Office template/route 才核该依赖 | 表格结构不能证明、借窄 Field 偷读隐藏 Document 正文、来源/Field 未获权都拒绝 |
| node_collection | 真实获权且完整的 Collection/Query 结果，按实际选择附加原窄 D4 Field entries 等输入；依赖 Query 的集合冻结完整 D7ResultPin | 实际消费 Query 时必须覆盖完整正负 query_scan、schema/bag/order/epoch/auth/cut 和全部依赖；不得只用 cursor 或额外读整篇正文 | Query 缺失/部分到达、Field 未获权、虚构空集合都拒绝；窄 Field 不自动扩大成 Document source_read |
| query_rows | 由 query_result 承载真实完整 D7 Query 结果及所有来源、依赖证据 | 必须完成 Query 并核实际目标；只有所选输出确实消费渲染器或模板时才核它们 | 首个分页、不完整结果、旧授权世代或只有 cursor 均不能冒充完整 rows |
| query_json | 只消费一份真实完整 D7ResultPin，按原 TerminalSchema.kind 分派：rows 保留完整列/type 与每行 typed V；scalar 恰保留一个 typed V（包括 none）；graph 完整保留 nodes、edges 两份 RowsSchema、nodeRef/sourceRef/targetRef/relationColumn/derivationColumn 列绑定，以及精确 data={nodes:[[V...],...],edges:[[V...],...]}。实际存在的每列（例如 nodes.displayName）、typed V、bag occurrence 及 D7 原行/表顺序都不能丢。 | 要求真实完整 Query/result/cut 与正负依赖；generationPolicy=none 不需要 renderer/template/provider、ViewSpec 或 network ViewSpec.nodeDetails 可选绑定。 | 已获权 graph 有孤立 node、合法端点/关系 schema 且 edges 为空，哪怕没有 ViewSpec/nodeDetails 也必须成功；若丢掉实际 node/edge 列或 V、重复 occurrence、截断分页、改原顺序或缺真实 nodes/edges 表则拒绝。不能要求额外 nodeDetails Query 数据成员，也不能改作 rows/chart。 |
| resource | 原 Resource resource_read；完整 SourceVersion/2、现任 SourceObservation/1 与精确 resource_bytes PinRef/2 | resource_exact 使用 generationPolicy=none，无需 Query、Annotation、document renderer 或其它无关 provider | 缺 resource_read、字节不可得、生产版本/Observation/pin 错误都拒绝，不得虚构 Query |
| annotation | 一次真实现任 D8AnnotationReadResponse/1 所形成的 annotation_content：精确 AnnotationRef、Observation、revision、Value/4、R6 body、canonical PortableAnnotationRecord/4 pin | 遵循 D8 当前 Annotation 资格；历史与目标上下文分别额外授权；backup 使用 generationPolicy=none，不要求 D7 Query | annotation_index 只能证明省略目录；隐藏 context、旧 revision 或 recordPin 不对应不得替换正文，也不能再次解析 |
| view | 实际选中的 query_result D7ResultPin 与精确 ViewSpec/1，随后执行原 D7 View §7；合法目标需具名已接受的 D9 六图表 renderer/profile | 需要完整 Query、原 result 授权/epoch/完整性，以及 complete_data 同数据可访问表；不得借 D8 Annotation 或文档 presentation 绕过 | 获权但 viewInput=99 不存在或选中非 query_result 时根本无 D7 result；真实 D7 错误不能被提前的 renderer 诊断覆盖 |

ExportInputCatalog/3 的封闭 payload 一共且仅有七种：document、resource、field、annotation_index、annotation_content、template、query_result。field 继续由原窄 D4 Field 的生产版本、ownerObservation、完整 entries 资格控制，不是第九个公开 inputDomain，也不能据此读取隐藏 Document 正文；annotation_index 只作为另行获权的完整省略目录。template 仅在实际消费时才要求：artifact 保留原 lease/字节；resource 来源另行满足 resource_read、SourceVersion/Observation 与精确字节/pin；route_asset 必须符合已审查安装 revision/profile。route/style/asset/ForeignBinding/Resource 依赖也只在实际消费时逐项授权并 pin。catalog kind/label、template 的存在、pin 并集和无关 provider 安装，都不能扩大所选域的原潜在范围。无 Query 的 Source/Resource/Field 读取走原现任读取 profile；实际消费 Query 的集合/查询则必须保留完整的 D7 正负依赖证明。

完成获权的输入域分派后，在进入 D7 View owner 前只允许检查最小路由条件：所选 View index 存在、payload.kind 恰为 query_result，并且有真实对应 D7ResultPin。若请求方选择了缺失的 viewInput=99，或选择了 field/annotation_content 而不是 query_result，这属于已获权请求自身的选择错误：D9 当前 Workspace 路径使用原封闭 wire2 d9_error {wireVersion:2,kind:"d9_error",code:"invalid_request"}；因为没有 result，不能虚构 D7 error，也不能借用 worker 的 invalid_output。这里 invalid_request 只适用于请求方选错 index/type，不可归咎于已认证的受保护 Plan 损坏或尚不能证明的 producer。已有 saved/frozen Plan 若其可达原受保护记录与 pin 在获权后确实证明 index/kind/value 互相矛盾，才可返回既有 wire2 integrity_conflict；必须有可达且证实的矛盾证据，不能因缺证据就声称确定损坏。真实 source/Observation 不可得时走原 source_unavailable；pin/证明不可取得或不能证明时走 proof_unavailable；domain 连续性缺失则沿用适用的 domain_unavailable 或原 owner 不可用/reset，不假定 integrity_conflict，更不得填空目录。上面已获权请求方选错 View 目录 index/kind 应用 invalid_request，不能用 invalid_output；这个限制只针对该 selector 错误，绝不缩小 D9 原有 invalid_output 职责。原 D9 Import IR §2 规定 Host/Core 必须在 IR schema 与 worker 状态之外独立验证格式覆盖率：worker 成功且 IR schema 合格，但独立检查发现 XLSX 遗漏了所选 sheet 或隐藏对象时，仍按原 invalid_output 处置（D9 Acceptance S33）；确实发现而无法表示的对象应进入 issue/loss。原 D9 Resource Region §8 的新签发还可按原格式/几何校验和入口资格使用已有 invalid_output 或 unsupported_profile。Host、Core、worker 各自输出校验职责仍在；也不得把任何来源/格式/Region 错误一律归为 invalid_output。现任 Workspace D9/2 错误仅使用原 §4a wire2 d9_error 闭集；历史 D9/1 与进入 D1/D3/D6/D7/D8 后的原错误信封及顺序不变。入口失权必须先返回不披露的 not_visible 或原 reset，不泄露私有路由细节；只有真实所选 D7 result 才进入 View §7 全部原序：封闭解码 → result 授权/epoch → 完整 result → schema → layout/binding → 全量结构/顺序 → domain/numbers → budget → 交付。经过原 owner 后，D9 selection/projection、recordPin、ViewSpec hash、target/receipt、canonical、budget/loss 关系仍须全部校验，最小路由不构成豁免。

fresh prepare 仍按 §6.5 与原 D9 workers/export owner §3a，在输出排序及分配 token 前检查完整候选 dataFile 名称和 report/manifest 保留名冲突；随后 Core 随机分配尚不公开的唯一 d9_export_plan/4 planToken，先于构造引用该 token 的 report/manifest/Plan 字节，验证完整 dataFiles，冻结精确 loss/staged bytes/pin，原子保存完整受保护 Plan 后才披露。准备失败不公开 token，也不增加第二 store/CAS/authority。已有 saved/planned/unknown/inspect/receipt 始终按原 token/version/Plan 分派，保留精确恢复。

跨输入域的排他关系同样强制执行：inputDomain 不是 annotation 时，contentSelection.annotationInputs 和 projection.annotations 都必须为空；inputDomain 不是 view 时，contentSelection.viewInput、viewRenderBinding 与 projection.view 都必须为 null。catalog 中存在某个 input，并不代表可在未选择、未授权的情况下消费它，也不得通过无关 projection 偷带内容。这些关系必须在输出准备前的严格准入阶段检查。

当 inputDomain=annotation：
- 必须至少选中一个 Annotation 内容输入（annotationInputs 非空）。普通文档正文与参考文献的选择均必须为空，即 bodyInput=null 和 bibliographyInput=null；同时不得选中 View（viewInput=null），不得附带 View 渲染绑定（viewRenderBinding=null）、View 投影（projection.view=null）或文档渲染绑定（documentRenderBinding=null）。
- annotationInputs 是以 inputIndex 为 key 的 set-like array，按 Counter 递增排序且唯一。同一个 catalog input 在一个 Plan 中最多出现一次，因此只能选择一个 mode。同一 Plan 混用 portable_backup 与 review_bundle_r6 必须拒绝；需要两种输出时使用两个 Plan。
- 每个选中的 inputIndex 必须精确指向一个 inputCatalog.items[index]，且 payload 必须是 annotation_content。
- `projection.annotations` 的数量与 `annotationInputs` 完全一致，且 `inputIndex` 均按严格递增顺序对应。每个位置 i 的选择 mode 必须与 projection mode 相同。对于 portable_backup，`projection.annotations[i].recordPin` 必须与 `inputCatalog.items[contentSelection.annotationInputs[i].inputIndex].payload.recordPin` 逐字相等；被选中条目的 payload 必须恰好是 annotation_content，D9AnnotationSelection/1 自身没有 recordPin 成员。payload.record 必须是由同一次合法 D8 read 中的 AnnotationRef、revision token 与完整 Value/4 构成的精确 PortableAnnotationRecord/4，原有 PinRef/2 必须校验 D3-CJ/3(record) 字节。获权 A/PA 且选择 A、投影 PA 时通过；若 A 的同一 index 却投影 PB，即使 B/PB 也单独获权、PA 和 PB 的 pin 都在递归证据并集中，仍必须拒绝。review_bundle_r6 只能匹配相同 index 的审阅投影。缺失或额外 projection、重复 index、跨 mode 配对或 `[0,1,0]` 均拒绝；pin 并集完整不能代替逐 index 的对应关系。
- portable_backup selection 要求 includeSourceHistory=false 且 includeTargetContext=false；两个 flag 对 backup 不适用，true 必须拒绝，不能静默忽略。此模式还要求 target.kind=annotation_backup 且 generationPolicy.kind=none。反过来，review_bundle_r6 不得选择 annotation_backup 目标；必须使用具名且已接受的 Review Bundle 渲染目标与 profile，不能以 review 语义写入备份文件。
- 对 review_bundle_r6：未请求 source history 时，includeSourceHistory=false 且 sourceHistory.state=not_requested；请求时 includeSourceHistory=true，结果只能是 disclosed 或 unavailable。target context 独立遵循同一规则。已经请求但无权披露时必须记录 unavailable，不能伪装成 not_requested 或空事实。
- D9AnnotationDisclosureProjection/1.fragments 是有顺序的 sequence，保持获权 producer/context 的原顺序，不能排序。state=disclosed 时至少一个 fragment。每个 fragment 的 origins 是非空 set-like array，按 canonical D3-CJ/3(ExportInputLocation/2) bytes 排序并去重；重复 origin 必须拒绝。

当 inputDomain=view：
- View 输入不得选择文档正文或参考文献，因此 bodyInput=null、bibliographyInput=null、annotationInputs=[]；viewInput 必须非 null，documentRenderBinding=null，viewRenderBinding 与 projection.view 均必须非 null。文档导出仍走独立 inputDomain，并继续由 D9DocumentRenderBinding/1.presentation 决定文档呈现；D8 的文档呈现决定不能拿来表示 View 的数据范围。
- contentSelection.viewInput、viewRenderBinding.resultInput、projection.view.resultInput 必须逐字相等，并精确选择一个 query_result catalog item。binding 中全部 hash/epoch/auth 字段都必须从这一个选中的 D7ResultPin 推导。
- `projection.view.viewSpecSha256` 必须与 `viewRenderBinding.viewSpecSha256` 逐字相等，而且两者都必须等于由精确 `viewRenderBinding.viewSpec`（ViewSpec/1）按 D3-CJ/3 编码后计算的 SHA-256。这里只复验已有的真实 ViewSpec，不新增 hash authority。即使三个 resultInput、layout 与 complete_data scope 全都一致，只要 projection 的 hash 来自另一个合法 ViewSpec，也必须拒绝。
- viewRenderBinding.renderer.layout 必须逐字等于 viewRenderBinding.viewSpec.layout。
- viewRenderBinding.outputScope 与 projection.view.outputScope 都只能是 complete_data。首批 profile 导出完整获权 D7 result 与 ViewSpec 中的全部 series、panel、item；设备本地 legend hide/show 状态不是 author data，不能改变 export scope。same-data accessible table 使用完全相同的 complete-data scope。未来若支持 current_display，必须增加 versioned successor，并冻结 stable hidden-series keys 与合格 local-state source，不能用 Boolean 或字段缺失来猜。
- 外层 target.kind 必须是 docx|xlsx|pdf|svg|png|print 之一，且逐字等于 renderer.targetKind；target.profileId 必须逐字等于 renderer.profileId。当且仅当 target.kind=print 时 destination.kind=print；非 print View target 一律拒绝 print destination。
- renderer.assets 是 set-like array，按 (role rank font<color_profile<page_profile<accessibility_profile, UTF8(assetId), UTF8(assetVersion), pin.pinToken) 排序且唯一。同一 (role,assetId) 若出现不同 version/pin 就是 conflict；不同 assetId 的多个 font/color asset 合法。只有 accepted renderer/profile 明确证明不消费这些外部 asset 时，该数组才可为空。
- renderer.evidencePins 是按 pinToken 排序且唯一的 set-like array，只含该 renderer route 实际要求的 installation/profile evidence，不得加入无关 pin。
- 现任 `D9ViewRendererBinding/1.layout` 仍只有六种封闭图表布局。合法的 D7 `network` 视图在当前 D9 导出路径上固定返回 `renderer_unavailable`；仅安装具名渲染配置不能扩展这一封闭联合类型。网络图形导出必须等待真正具名且升版的渲染器与模式继任合同。

交付与回执的兼容关系同样是封闭集合：
- destination.kind=external_bundle 才可生成 PublicationReceipt/4；所有重复 Plan member（包括 viewRenderBinding）必须逐字相等。PublicationReceipt/4.presentation 只有在 documentRenderBinding 非 null 时才逐字等于该 document binding 的 presentation，否则必须为 null；它绝不承载 View output scope。
- destination.kind=print 只生成 D9PrintReceipt/1。回执的 `planToken` 必须在原 Workspace/domain、现任 audience disclosure 和已确认的交付责任下解析到同一份原始、经认证的受保护 ExportPlan/4 记录，绝不能只凭 token 文本匹配另一份任意 Plan。该 Plan 的 target.kind=print 且 destination.kind=print。回执 viewRenderBinding 必须与 Plan 逐字相等；output 必须来自该 Plan 已冻结且验证过的 staged bytes 的真实受控 print 输出，name/digest/size 与经确认的 lossReport/lossChoices 都要对应，不能换 output 或 renderer。D9PrintReceipt/1 没有 target 成员：不得比较不存在的 receipt.target，也不得新增 wire 成员。Plan 关联、binding、output 或 loss 错配时，即使回执自身结构合法也必须拒绝。
- destination.kind=resource_handoff 只产生既有独立 D7/D3 author result，消费 exact staged bytes；不得伪造 PublicationReceipt/4 或 D9PrintReceipt/1。
- destination.kind=server_download 只走既有服务器下载交付与状态查询路径；它不产生可携带的外部发布回执，也不产生打印回执。

canonicalization 只允许在 Plan/4 freeze 前执行一次。已经 frozen、received、inspect、confirmation、saved/planned/unknown 或 recovery 的 record 必须本来就满足这些顺序与关系；noncanonical array 或关系不匹配直接拒绝，不能读取时排序或修复。既有 D7 row order、Annotation disclosure fragment order 与其它 owner-defined ordered sequence 必须保留，不能全局排序。

D9ViewRenderBinding/1 与 Plan/4 仍是设计候选，产品执行 UNRUN；本次修正不迁移任何已部署或已记录的 View-binding bytes。将来一旦 /4 family 真正 accepted/deployed，若要增加新的 View output scope 或 renderer layout member，必须新增 versioned successor，不能原地扩宽 /1。

ExportInputCatalog/3 逐字保留 /2 的全部 arm，只新增 annotation_content。该 arm 必须从一次真实现任 D8AnnotationReadResponse/1 构造：annotationRef、sourceObservation、annotationRevisionToken、value、body 与 targetResolution 都与该 read 逐字相等。record 精确为对应 PortableAnnotationRecord/4；recordPin 在既有 PinRef/2 完整性规则下选择 D3-CJ/3(record) 的精确字节。Plan 的 dependencyProof 与 observationProof 覆盖同一 cut 的 Annotation read，以及任何独立获权的 context read。最终 export barrier 同时复验 sourceObservation 与 annotationRevisionToken；正文文本相同不能替代已经变化的 revision。

annotation_index 继续只作为 omission-directory evidence，绝不能填充 annotation_content、annotationInputs 或 Annotation 正文/context projection。portable backup 要求 inputDomain=annotation、target.kind=annotation_backup、一个或多个 mode=portable_backup selection、generationPolicy=none，并生成精确 D9AnnotationBackupFile/1：records 按完整 canonical AnnotationRef bytes 排序且唯一，每个 record 必须逐字等于对应已选 recordPin 的 D3-CJ/3 解码值；整个 backup file 使用 D3-CJ/3(D9AnnotationBackupFile/1) 的 canonical UTF-8 bytes。它不序列化当前 permission、SourceObservation capability、revision-signing capability、PAB 或 ActionEvidence。Review Bundle 要求 mode=review_bundle_r6，且只消费已经由 D8AnnotationBodyRead/1 产生的结果：valid 使用其 R6 semantic text，absent 使用 null，invalid 则 renderer unavailable，绝不再选第二 parser。purpose、appearance、labels、reviewState、suggestion、reply 与 attribution 都来自同一完整 Value/4。

Annotation 正文/署名与 target/source context 独立做资格校验。includeSourceHistory 与 includeTargetContext 只是选择是否尝试，不代表 permission。target hidden/unavailable 时写成 targetContext.state=unavailable，不能压掉原本合法的 Review Bundle 正文、署名或 reply。任何 disclosed fragment 都必须来自同一 cut 中另行获权的 catalog input，并带非空 ExportInputLocation/2 origins。缺少 context 只能由相应 projection state 与完整 loss item 表示，不得用 display label、annotation_index 或猜测 target bytes 替代。

View export 要求 inputDomain=view、恰一个 viewInput 指向 query_result catalog item、没有 Annotation selection，并且 viewRenderBinding 非 null。只有在公共入口完成授权，并由内部候选目录定位出真实所选 query_result D7ResultPin 后，Core 才对这份完整结果执行 D7 View §7 原顺序，首先是 ViewSpec 静态封闭解码。所选 index 不存在或 payload 不是 query_result 时不得进入 D7，而是拒绝已获权候选的结构错误，不得伪造 D7 结果错误。tree、treemap、sunburst、Gantt、boxplot、quantile 等不属于现任 D7 ViewSpec/1 封闭集合的布局，必须先返回 unsupported_layout，不能进入 D9 渲染器选择。对于合法现任 D7 布局，首个封闭 D9 chart profile 只渲染 metric、bar、line、scatter、pie、heatmap；network 等其它合法 D7 布局，或六图表缺少具名 backend/profile 时，当前 route 返回 renderer_unavailable。不得把数据行偷换成图表；renderer 不能重跑 Query、补读、排序、聚合、分箱、采样或修改 ViewSpec 语义。

querySemanticSha256 是 D3-CJ/3(该 D7ResultPin 保留的精确 D7 SemanticStateKey) 的 SHA-256；snapshotResultSha256 是 D3-CJ/3(该 pin 保留的精确 D7 SnapshotResultKey) 的 SHA-256。resultEpoch 与 authorizationGeneration 必须与同一 result evidence 逐字相等；viewSpecSha256 是 D3-CJ/3(精确 ViewSpec/1) 的 SHA-256。这些 hash 只做冻结交叉校验，不是 identity，也不形成第二 cache authority；完整 result/cut/dependency authority 仍是所选 D7ResultPin。

View renderer binding 只能绑定一个具名已安装 renderer/profile/version 与一个精确 target kind；canonical asset/evidence collection 与 target/profile 关系由 §6.6.1 定义。固定 complete_data scope 要求同一完整数据表、title/description alt-text 语义、Query/panel 顺序、CJK/RTL 保全与非纯颜色编码。PDF、SVG、PNG 与 print profile 可以直接渲染这六个 layout；DOCX/XLSX 只有在其具名 profile 能证明同一 View 语义时才可用。若 Office route 使用 template，§6.5.1 的可见模板 authority 与全部 §14 规则仍然适用。backend/profile/layout 组合不支持时稳定 unavailable，绝不能静默替换为 data table。

Plan/4 的 `evidencePins` 必须精确等于 `Pins(inputCatalog)`、`Pins(projection)`、`Pins(documentRenderBinding)`、`Pins(viewRenderBinding)`、`Pins(templateBinding)`、`Pins(routeBinding)`、`Pins(styleBundles)`、`Pins(dependencyProof)`、`Pins(observationProof)`、`Pins(stagedOutputs)` 与 `recoveryPins` 的 pinToken 排序唯一递归并集，并且不得递归 Plan 自己的 `evidencePins` 成员。因此，实际消费的 Annotation record/context pin，以及 View 的 renderer、asset 与 result dependency，仍全部进入这一份唯一 pin 并集。

D9ExportConfirmation/2 不能改变 catalog、Annotation/View selection、projection、ViewSpec、renderer/profile/assets、route/template、target、destination、loss report 或 staged bytes。external publication 继续 create-only，并生成 PublicationReceipt/4；Resource handoff 继续走原 D7/D3 对精确 staged bytes 的独立 author 路径。print destination 使用同一个 frozen/staged/confirmed Plan，只生成 D9PrintReceipt/1；它不授予 author capability，也不能声称 external publication。现任 Plan/4 bundle 使用 D9ExportBundleManifest/2。

真实的 ExportPlan/1-/2-/3 与 PublicationReceipt/1-/2-/3，以及各自的 plan/publication token tag、ExportInputCatalog/2、ExportContentSelection/1、ExportProjection/1、ExportLossReport/1、D9ExportConfirmation/1 和 bundle manifest/1，都保留精确的历史 decoder；pins、confirmation bytes 与 saved/planned/unknown recovery 也保持原义。不得把 /3 record 重新 pin、重新编码、排序、改名或升级成 /4。


# 7. Annotation closed values

```text
AnnotationRevisionToken/1 := nonempty opaque JSON string

PortableAnnotationRecord/4 = {
  kind:"portable_annotation",version:4,
  annotationRef:AnnotationRef,
  annotationRevisionToken:AnnotationRevisionToken/1,
  value:D3-Annotation-Value/4
}

AnnotationEditableValue/1 = {
  purpose:"comment"|"mark"|"suggestion",
  target:D3-Annotation-Target-Projection/1,
  replyTo:AnnotationRef|null,
  body:AnnotationInlineBody/1|null,
  appearance:AnnotationAppearance/1|null,
  labels:[text...],
  reviewState:"open"|"resolved"|"not_applicable",
  suggestion:Suggestion/3|null
}

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

PortableAnnotationRecord/4 是现有 Node 内 annotations JSON authority 中的当前逻辑 Portable Metadata record；annotationRef.owner 必须等于所属 Node。D3-Annotation-Value/4 的 canonical bytes 恰为 D3-CJ/3(value)；当前 annotation_value payload binding 与 PinRef/2 hash/pin 的是这些字节，不是外层 PortableAnnotationRecord/4。annotationRevisionToken 是不透明 token，不能由 value digest 推导；同一 AnnotationRef 的每个已提交 Value/4 state 都必须唯一。

当前 wire13 D3 mutation 使用 Value/4 基础 decoder，并保留继承的逻辑 slots：annotation_target ordinal0、annotation_reply ordinal1。target 永远是 reference slot；replyTo 非 null 时也是 reference slot；existing Annotation 的 identity-preserving reply change 使用继承的 structural S，禁止重复产生 reply reference result。其它 Value/4 成员都是 nonreference bytes。任何已经改变的当前 Value/4 只有一个新的 final AnnotationRevisionToken/1；target/reply 的 toSource address、annotation_reply_change evidence、SourceRevisionPlan/result pin、source change 与 receipt 都必须使用同一 token。Caller AnnotationEditableProposal/1 bytes 绝不直接和 AnnotationEditableValue/1 比较；Core 先通过 operation-class gate 展开保留 before attribution 的完整 candidate Value/4。candidate Value/4 与 before Value/4 canonical bytes 相等才是 no-op，并完整保留 Suggestion evidence、token/SourceVersion/H/attribution，且不建立 SourceRevisionPlan；只有 candidate 真正不同才写 fresh lastEditor/editedAt 与 fresh final token。

当前 payloadBindings 的 payloadKind=annotation_value 必须按真实 request family 分派：wire13 当前 mutation 严格解码 Value/4；真正历史 wire9–12 继续使用 Value/3 decoder。D3-Symbolic-Result/9 framing 不变，但 Annotation 基础字节与 slot span 必须由该 request family 选择的 decoder 计算；禁止扩大历史 decoder。

当前 Portable Metadata 不再把 D2 Annotation-v2 outer wire 物化为作者状态。历史 D2 v2 Annotation snapshot、Value/3、plain_text 正文、replace_plain_text suggestion 与 targetStatus resolved/stale 只保留真实历史 decoder/recovery。当前正文 bytes 只能是 AnnotationInlineBody/1，并使用唯一 AnnotationInlineProfile/1；不存在 plain-text 兼容回退或第二 parser。

## 7.1 Caller proposal 与 Node-local physical aggregate

```text
SuggestionAuthorProposal/1 = {
  kind:"replace"|"delete"|"insert",
  replacementSource:null|text
}

AnnotationEditableProposal/1 = {
  purpose:"comment"|"mark"|"suggestion",
  target:D3-Annotation-Target-Projection/1,
  replyTo:AnnotationRef|null,
  body:AnnotationInlineBody/1|null,
  appearance:AnnotationAppearance/1|null,
  labels:[text...],
  reviewState:"open"|"resolved"|"not_applicable",
  suggestion:SuggestionAuthorProposal/1|null
}

AnnotationAggregate/1 = {
  format:"weftext.annotations",version:1,
  ownerNodeRef:NodeRef,
  records:[PortableAnnotationRecord/4...]
}

AnnotationAggregateObservation/1 = {
  kind:"d6_annotation_aggregate_observation",version:1,
  workspaceRef:WorkspaceRef,
  ownerNodeRef:NodeRef,
  observerDomain:CommitDomain/2,
  fileObjectBinding:FileObjectBinding/1,
  aggregateBytesPin:PinRef/2|null
}

AnnotationAggregateInstall/1 = {
  kind:"d6_annotation_aggregate_install",version:1,
  ownerNodeRef:NodeRef,
  before:AnnotationAggregateObservation/1,
  after:"absent"|PinRef/2,
  installCapability:FileInstallCapability/2,
  changedAnnotationRefs:[AnnotationRef...]
}
```

SuggestionAuthorProposal/1 只承载作者可提议的 kind/replacementSource。replace 必须带 replacementSource（允许空串），delete 必须为 null，insert 必须带非空内容。state、confirmation、targetBasisSha256、expectedText、pointAffinity 永远不属于 caller proposal。AnnotationEditableProposal/1 的 purpose、reply、body、appearance、labels 与 review 约束继续与 AnnotationEditableValue/1 相同；Core 通过操作类别 gate 决定 Suggestion/3 受控的 before→after。gate 使用派生 current-state class，而不把 null 当成 fresh absence：fresh create=`absent_annotation`；已有 suggestion=null=`no_suggestion`；pending=`pending_confirmed|pending_needs_reconfirmation`；terminal=`terminal_accepted|terminal_rejected`。interactive_create 接纳合法 no-suggestion comment/mark/reply 与合法 root suggestion；ordinary_edit 接纳 no_suggestion→no_suggestion、真实目标资格后的 no_suggestion→pending+needs_reconfirmation、pending→pending、pending→no_suggestion，以及 terminal→同一 terminal only；manual_reattach 只接纳 no_suggestion→no_suggestion 或 pending→pending+needs_reconfirmation，不能和类别转换合并。target 不变的 no-suggestion 编辑以及 pending→no_suggestion 清除不会凭空获得 target-source-read 权限；真正消费目标内容的转换/操作只在各自具名 qualification/read 阶段读取。完成 gate 后 Core 才展开保留 before attribution 的完整 candidate Value/4 并与 before 比较 canonical bytes；Proposal bytes 永远不是 no-op comparator。

`weftext.annotations.json` 的 current physical bytes 恰为 D3-CJ/3(完整 AnnotationAggregate/1)，无 BOM、无额外换行或第二 envelope。records 必须非空，按完整 AnnotationRef canonical bytes 升序且唯一；每个 annotationRef.owner 必须等于 ownerNodeRef，nonnull replyTo 必须同 owner 且整张 records reply graph 无环。空集合唯一物理表示是 sidecar absent，禁止同时保留空 aggregate。unknown/missing member、duplicate JSON key、unknown version、owner mismatch、duplicate Ref 或 reply cycle 都使整个 aggregate strict-decode 失败；normal current read 不能部分信任其中“看起来合法”的记录。

AnnotationAggregateObservation/1 是物理文件观察，不是 Annotation 的 SourceVersion/SourceObservation、revision token 或 identity。这里的 `FileObjectBinding/1` 与 `FileInstallCapability/2` **逐字复用 fixed D6 Control Interfaces §2**，不是本候选新版本：absent binding=`{kind:"absent",backendToken,relativePath,observationEpoch,parentGenerationToken}`；present binding=`{kind:"present",backendToken,relativePath,observationEpoch,objectGenerationToken,byteLength,sha256}`；capability 仍只有 create_only(parentGenerationToken)、conditional_replace(expectedObjectGenerationToken)、exclusive_write_window(windowToken,expectedObjectGenerationToken)、observed_replace(observedObjectGenerationToken)。PortableRelativePath、Token/Counter、64-lowercase-hex 及全部 containment/continuity 规则保持该 owner 原义。fileObjectBinding.kind=absent 时 aggregateBytesPin 必须 null；kind=present 时 aggregateBytesPin 必须是 payloadKind=portable_metadata 且 bytes 等于上述完整 physical JSON 的 PinRef/2。digest 单独永远不是 CAS。AnnotationAggregateInstall/1 只是现有 D6 安装计划的一个具名 file-write coordination binding，不创建新 ledger/CAS/author store；before 必须是 fresh Observation，而不是裸 `"absent"`。after 为 PinRef 时指向完整 after aggregate，为 `"absent"` 时表示删除最后一个 record。installCapability 必须与 before FileObjectBinding 精确交叉：absent→create_only 且 parentGenerationToken 相等；present→原 D6 允许的 replace capability 且 generation/window token 精确匹配。managed_atomic strong path 仍只接受该 owner 的 strict capability；observed_replace 仅在 D6 §4.1 原本 observed_only eligibility 已成立时有效，绝不升级成 CAS。changedAnnotationRefs 按 Ref canonical bytes 排序唯一，并恰等于 before/after logical record 差集。一个 Node 在一个 DecisionKey 下最多一个 AnnotationAggregateInstall/1。

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

plan/evidence edits 的 D3-CJ/3 必须逐字节相等；seal 不得重编译、重排或合并。required 决策密封后恰有一个 outbox item，disabled 则为零；artifactPin 保存 portable_metadata 的精确规范 artifact bytes。

Event3 必须针对精确的 beforeBytes/afterBytes 做闭合校验。replace 要求 0<=startByte<endByte<=before length，两端都位于 UTF-8 scalar boundary；removedByteLength=endByte-startByte，且 removedSha256=SHA-256(beforeBytes[startByte:endByte])。
insert 要求 0<=atByte<=before length、atByte 位于 UTF-8 scalar boundary，且 replacementByteLength 非零。
replacement interval 必须组成两两不重叠的 maximal island；同一点的 insert 按 transaction order 合并。严格位于 replacement island 内的 insert 必须折入该 island，否则 compilation unavailable。
规范边界顺序固定为：在 p 结束的左 replacement、insert@p、从 p 开始的右 replacement。

SourceTransformEvidence/2.beforeSourceSha256 必须恰为 "sha256:" + lowercase_hex(SHA-256(evidence.before 与 ownerNodeRef 对应的完整精确 before-source bytes))。receiver 必须从 producing CP/history 与 retained version evidence 取得这份历史 bytes，重新计算 whole-source digest，并在接受 artifact 前要求相等。Event removedSha256/length、slice replay 成功、SourceVersion 字段相等或 current-file bytes 都不能替代。缺 exact-before bytes 时沿用既有 proof/state-unavailable 边界；已证明 digest 矛盾则属于 integrity failure。该交叉字段不新增成员，也不提升 SourceTransformEvidence/2 版本。

generatedOutputSpan 使用 SPEC §11.1 的 replay-cursor 算法处理精确的 before/after pins。每段未变化间隙都要逐字节比较，当前 event span 恰为 [afterCursor,afterCursor+replacementByteLength)，并用该 after slice 验证 replacement 的长度与 hash。
replace 将 beforeCursor 推到 endByte；insert 保持在 q。最后剩余字节与 afterSourceSha256 都必须匹配，禁止内容搜索。
mapping 定义 delta(replace)=replacementByteLength-(endByte-startByte)，并使用有溢出检查的有符号运算。

SourceTransformSealSignedBody/1 恰为 SourceTransformSealArtifact/1 只删除 signature。signature 必须恰为 86 个 ASCII unpadded-base64url 字符，解码后是 64-byte Ed25519 signature。
待签消息恰为 ASCII "D6-Source-Transform-Seal/1" || NUL || D3-CJ/3(SourceTransformSealSignedBody/1)。完整传输/存储 artifact bytes 恰为含 signature 的 SourceTransformSealArtifact/1 的 D3-CJ/3；其它 JSON serialization 必须拒绝。

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
       conflictId:ConflictId,
       resolvedHeads:[ChangeId/1...],
       selected:WorkspaceAuthorizationBundleAddress/1,
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

公开 conflict request 继续使用 fixed-parent d6_conflict_prepare wireVersion3 与 ConflictResolution/2。
对当前尚未建立决议的 policy_bundle_choice，同一 selected/policy/freshAuthorizations JSON arm 使用下面的 current inner type 与受保护 descriptor。
source_merge 与 choose_source_head 继续使用 fixed-parent ConflictResolutionInput/2 与 ConflictResolutionDerivedPlan/1。

```text
FreshDomainAuthorizationSpec/2 = {
  commitDomain:CommitDomain/2,
  profile:SealProfileId/1
}

PolicyBundleHeadEvidence/2 =
    {head:ChangeId/1,completionProofVersion:3,bundleVersion:1}
  | {head:ChangeId/1,completionProofVersion:4,bundleVersion:1|2}

TrustConflictCarryValidationHop/1 =
    {declarationVersion:1,declarationRevision:Counter,
     declarationDigest:"sha256:<64 lowercase hex>",
     activationChangeId:ChangeId/1,
     completionProofVersion:3,bundleVersion:1,
     changeRecordPin:PinRef/2,
     completionProofPin:PinRef/2,
     policyBundlePin:PinRef/2}
  | {declarationVersion:2,declarationRevision:Counter,
     declarationDigest:"sha256:<64 lowercase hex>",
     activationChangeId:ChangeId/1,
     completionProofVersion:4,bundleVersion:2,
     changeRecordPin:PinRef/2,
     completionProofPin:PinRef/2,
     policyBundlePin:PinRef/2}

TrustConflictCarryValidationEvidence/1 = {
  head:ChangeId/1,
  factId:"sha256:<64 lowercase hex>",
  origin:TrustConflictCarryValidationHop/1,
  carriers:[TrustConflictCarryValidationHop/1...]
}

ConflictResolutionPolicyDerivedPlan/2 = {
  kind:"policy_bundle",
  selected:WorkspaceAuthorizationBundleAddress/1,
  headEvidence:[PolicyBundleHeadEvidence/2...],
  selectedBundleVersion:1|2,
  selectedBundlePin:PinRef/2,
  carryEvidence:[TrustConflictCarryValidationEvidence/1...],
  effectiveCompromises:[
    TrustConflictCarry/1|TrustConflictCarry/2...
  ],
  inheritedCompromises:[
    TrustConflictCarry/1|TrustConflictCarry/2...
  ],
  outcomes:[TrustConflictOutcome/2...],
  resultBundleVersion:1|2,
  resultBundlePin:PinRef/2
}

ConflictResolutionInput/3 = {
  kind:"d6_conflict_resolution_input",version:3,
  conflictId:ConflictId,expectedKey:ConflictKey/1,
  resolution:{
    kind:"policy_bundle_choice",
    selected:WorkspaceAuthorizationBundleAddress/1,
    policy:Policy/3,
    freshAuthorizations:[FreshDomainAuthorizationSpec/2...]
  },
  branchEvidence:[ConflictResolutionBranchEvidence/1...],
  derivedPlan:ConflictResolutionPolicyDerivedPlan/2
}

ConflictResolutionPreview/2 = {
  kind:"d6_conflict_resolution_preview",version:2,
  conflictId:ConflictId,expectedKey:ConflictKey/1,
  resolution:{
    kind:"policy_bundle_choice",
    selected:WorkspaceAuthorizationBundleAddress/1,
    policy:Policy/3,
    freshAuthorizations:[FreshDomainAuthorizationSpec/2...]
  },
  branchEvidenceDigest:"sha256:<64 lowercase hex>",
  derivedPlan:ConflictResolutionPolicyDerivedPlan/2
}
```

FreshDomainAuthorizationSpec/2 的 JSON 成员仍只有原来的两个，但 profile 扩为完整 SealProfileId/1 union。数组按 D3-CJ/3 规范排序且唯一，不含任何 key material。current ConflictResolutionInput/3 只作为 policy_bundle_choice 的 protected owner-descriptor successor；它不提升公开 request 版本、不新增 submit path，也不改变 conflict_resolve、policy_admin、disclosure 或 error order。任何可证明真实 saved/planned 的旧 owner descriptor 都保留 recorded decoder、preview 与 pins。

PolicyBundleHeadEvidence/2 必须完整、按 ChangeId 排序唯一，并与 expectedKey.heads 的集合逐项相等。version=3 表示 branchEvidence.completionProofPin 必须严格解码 ContentCompletionProof/3，且精确 policy after-image 必须严格解码 WorkspaceAuthorizationBundle/1。version=4 表示 completionProofPin 严格解码 ContentCompletionProof/4，branchEvidence.changeRecordPin 严格解码匹配的 ChangeRecord/1 与精确 Notice3/CP4 chain，policy after-image 再严格解码为声明的 Bundle1 或 Bundle2。两种 arm 都要求 policyBundlePin 存在并 pin 精确 canonical bundle bytes；WorkspaceAuthorizationBundleAddress/1 的 authorizationRevision、trustRevision、byteLength、sha256 必须匹配。policy_bundle_choice 的每个 branchEvidence.sourcePins 必须为空。CP3+Bundle2、未知版本、CP4 缺 ChangeRecord、tag/bytes 不一致或 decoder fallback 一律拒绝。

TrustConflictCarryValidationEvidence/1 是某个 effective fact 在某个 expectedKey head 上实际消费的完整 retained validation path。数组先按 head ChangeId，再按 ASCII factId 排序唯一；resolver 的逐 head effective compromise fold 使用的每个 (head,factId) 恰有一项。origin 是 Carry 所命名的直接 compromise declaration；carriers 按 declaration revision 递增，逐项列出该 head 上从 origin 之后真实经过的所有 resolve_conflict declaration。version-1 hop 严格解码其真实历史 ChangeRecord/CP3/Bundle1 activation evidence；version-2 hop 严格解码 ChangeRecord/1 + CP4 + Bundle2。declarationDigest 与 activationChangeId 必须逐字节等于这些精确 pins 所证明的 declaration 和 activation cut。origin hop 必须匹配 Carry 的 originDeclarationRevision、originDeclarationDigest 与 originActivationChangeId，并重新验证 action/key 映射。每个 carrier hop 都必须是实际携带该 fact 的合法 resolver。缺少任何实际 traversed carrier、把 origin cut 换成 resolver 时间，或用相同 current bytes 替换历史 evidence 都失败。

对 current policy arm，OwnerInputBinding/2.protocolOwner 固定为 D6，ownerKind 精确为 d6_conflict_resolution/3；InputDescriptor/3.intentKind 必须与该 ownerKind byte-equal。canonicalDescriptorBytes 精确等于 D3-CJ/3(ConflictResolutionInput/3)。OwnerInputBinding/2.pinRefs 是按 canonical PinRef 排序且去重后的精确并集，只包含：每个 branchEvidence.changeRecordPin；每个 branchEvidence.completionProofPin；每个存在的 branchEvidence.policyBundlePin；selectedBundlePin；resultBundlePin；以及 carryEvidence 中每个 origin/carrier 的 changeRecordPin、completionProofPin、policyBundlePin。hash-only declaration reference、current bundle、Derived Index row 或未列出的 pin 都不能替代这些条目。若 selected bundle pin 与对应 branch policyBundlePin 相同，set 去重后只出现一次。

ConflictResolutionPreview/2 是 current policy 的 immutable owner preview。branchEvidenceDigest 精确为 "sha256:" + lowercase_hex(SHA-256(D3-CJ/3(完整 ConflictResolutionInput/3.branchEvidence array)))。preview 的 conflictId、expectedKey、resolution 必须与 Input3 byte-equal；derivedPlan 必须与完整 Plan2 byte-equal，包括 headEvidence、carryEvidence、mixed Carry1/2、Outcome2 以及 resultBundleVersion/pin。PreparedIntent/3.previewBinding 必须绑定这份精确 Preview2 与其 canonical preview pin。对这个 control_only policy arm，PreparedIntent/3.pinDirectory 必须是以下集合的 canonical duplicate-free union：OwnerInputBinding.pinRefs、DependencyProof/3 的全部 evidencePin、previewBinding 所选精确 preview pin、以及 fixed installationPlan 实际命名的全部 proposal/before/after/recovery PinRef。sourceInputs 为空，因此这里没有 SourceObservation evidence pin。相同 pin 不能重新绑定到另一份 protected record。

两个 current source arm 不使用这些 policy successor。source_merge 与 choose_source_head 继续保留 ConflictResolutionInput/2、ConflictResolutionDerivedPlan/1、ConflictResolutionPreview/1。OwnerInputBinding.ownerKind 与 InputDescriptor/3.intentKind 都固定为 d6_conflict_resolution/2；canonicalDescriptorBytes 必须精确等于 D3-CJ/3(Input2)。
原有完整 pin union、source semantic evidence、saveProfile=complete、scope selection 与 Preview1 规则继续有效。只有外层尚未建立决议的 D6 carrier 改用 current InputDescriptor/3 + DependencyProof/3 + PreparedIntent/3 family；planToken 为 d6_plan/3。

Carry2 的 originDeclarationDigest 恰为 "sha256:" + lowercase_hex(SHA-256(D3-CJ/3(包含 rootSignature 的完整原 WorkspaceTrustDeclaration/2)))。直接 Declaration2 compromise 映射固定为 revoke -> action.trustKeyId，rotate -> action.oldTrustKeyId。Carry2 fact body 是按 D3-CJ/3 闭合编码的九个字段：workspaceRef、commitDomain、profile、compromisedTrustKeyId、originAction、originDecisionKey、originDeclarationRevision、originDeclarationDigest、originActivationChangeId。factId 固定为：

```text
factId =
  "sha256:" + lowercase_hex(
    SHA-256(
      ASCII "D6-Trust-Compromise-Fact/2" || NUL ||
      D3-CJ/3(Carry2 fact body)
    )
  )
```

originActivationChangeId 只能从原 Declaration2 DecisionKey，经 committed CP4 + ChangeRecord/1 与首次追加它的精确 Bundle2 after-image 重派生。Carry1 保持 fixed-parent /1 fact domain 与 Declaration1/CP3 origin 规则。Declaration2 resolve_conflict 可以继承 Carry1/Carry2；每项都必须递归回溯并重验其 direct original compromise declaration。effectiveCompromises 与 inheritedCompromises 都按 ASCII factId 排序唯一；重复 factId 必须 canonical bytes 相等，否则 integrity_conflict。effective union 同时包含 selected 与 losing branches，因此 branch choice 不能丢 compromise fact。

Bundle1 的 source-transform normalized state 是 none。affectedDomainProfiles 是所有 head 间 normalized domain/profile state 不同的 pair，与 effective compromise union 指向的所有 pair 的并集。需要 trust-resolution declaration 时，TrustConflictOutcome/2 按 D3-CJ/3(commitDomain,profile) 排序唯一，并完整覆盖每个 affected pair。keep_current 只允许 selected current key 在完整 effective union 下仍安全。显式请求的 affected pair 可以 authorize_fresh，其新 key 使用 DomainSealKeyHandle/2 与 PoP/2。若无需 trust resolution 且没有 fresh authorization，policy-only 结果保留 selectedBundleVersion/trustRevision；否则恰追加一条 WorkspaceTrustDeclaration/2 resolve_conflict，resultBundleVersion=2。selected Bundle1 的精确 Declaration1 prefix 必须原样保留，再追加新的 Declaration2。

ConflictResolutionPolicyDerivedPlan/2 冻结每个 head 的 proof/bundle 分派与 selected bundle pin，还要完整冻结逐 head carry validation evidence、完整 carry union、inherited subset、Outcome2 以及精确 result bundle pin/version。resultBundlePin 必须严格解码 derived result 并复现 canonical bytes。
原唯一 planning CAS 会同时冻结 InputDescriptor/3、OwnerInputBinding/2、Preview2、pinDirectory、staged fresh-handle association 与 installationPlan。final submit 仍只有 d6_commit_request/2。唯一 final P 通过 Notice3/CP4/ChangeRecord1 发布当前 policy component 与同一 conflict-record transition；staged authorize_fresh handle 只能在该 commit 中变 usable。
planned recovery 恢复精确的 Input3/Plan2/Preview2 bytes 与 pins，绝不从当前 history 重建。receiver 必须重算每个 head 的分派、Carry1/Carry2 facts、全部 retained carry-validation hop、mixed recursive union、PoP/2 与 root signatures；还必须验证精确 result bundle、preview/descriptor 交叉字段，以及同一 DecisionKey 的 conflict-record transition。

每个 publicKey 必须解码为精确的 32-byte Ed25519 key，并哈希到对应 trustKeyId；每个 signature 值必须解码为精确的 64-byte Ed25519 signature。
authorize/rotate 的 possessionSignature 必须按精确消息签署。
签名消息为 ASCII "D6-Domain-Seal-Key-PoP/2" || NUL || D3-CJ/3(DomainSealKeyPoPBody/2)。
rotate 把 newTrustKeyId 映射到 PoP body 的 trustKeyId。
authorize_fresh 使用外层 Declaration2 的 common fields；再加入该 outcome 的 commitDomain/profile/key tuple，构造同一 PoP body。
ordinary rotate 的 continuitySignature 必须按精确消息签署。
签名消息为 ASCII "D6-Domain-Seal-Key-Rotate/2" || NUL || D3-CJ/3(DomainSealKeyRotateContinuityBody/2)。
loss_recovery/compromise 必须使用 literal "not_required"。
rootSignature 签署 exact ASCII "D6-Workspace-Trust-Declaration/2" || NUL || D3-CJ/3(WorkspaceTrustDeclarationSignedBody/2)。

revision-1 root predecessor 的 fingerprint 是完整 WorkspaceTrustRootFingerprint/1，并与 retained root/anchor fingerprint byte-equal；rootKeyId 不能替代。Declaration/1 永远只授权 revision-token profile。Bundle2 允许 Declaration1 历史 prefix，出现首个 Declaration2 后后继都只能 /2。Declaration1 activation 继续用 CP3；Declaration2 activation 由同 DecisionKey ChangeRecord1 -> exact CP4 -> policy after-image -> Bundle2 重派生。

当前未见过的普通 trust management 只使用上述 wireVersion3 prepare successor，不携带 caller key material。
它保留 fixed-parent 的 policy_admin、root-handle、current-state gate 与原 planning/install/single-P 路径，只把当前 completion family 接到 CP4。
已保存或已规划的 wireVersion2 record 继续使用原 decoder/recovery。

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
  initialPresentationPolicy:D8PresentationPolicyBootstrapInit/1,
  initialSeriesConfigurations:[WorkspaceBootstrapSeriesConfiguration/1...],
  periodScopeBindings:[WorkspaceBootstrapPeriodScopeBinding/1...]
}
```

全部 UUID 成员使用 D3 的 canonical lowercase UUID decoder。Genesis 恰两项：revision 1 是 revision-token authorize，revision 2 是 source-transform authorize；两者使用同一 DecisionKey/activation ChangeId，且 rev2 predecessor 必须哈希精确的 rev1 canonical bytes。
series configuration 按 canonical SeriesScope 排序且唯一；period binding 按完整 NodeRef 排序且唯一，每个有效 prepared period 恰有一项。
这些 helper member 保留 fixed-parent Plan3 的字段语义；Profile4 只改变 dual-profile genesis family。

Plan4 只用于 unseen fresh create/fork，并保留原 D3 proposal/custody/CAS/P boundary。普通 copy 不使用 Plan4。saved/planned/unknown recovery 保留实际 recorded decoder/bytes；restore/continue/failover 不合成 Genesis2。本候选不声称 Plan3 已部署，也不虚构 migration。

initialPresentationPolicy 是必填字段，并在同一个 Plan4 内闭合 D8 的 fresh-target state。before/proposal 的 WorkspaceRef 必须等于 targetWorkspaceRef；before 是 §6.4 定义的受保护 fresh empty-head observation，且 stamp.revision=1；proposal 精确为 parents=[]、revision=1、defaultPresentation=separate。
准备、规划和暂存阶段必须使用原 create/fork 的 OperationId、planning CAS 和 DecisionKey 冻结该 helper，并冻结一个 committed=null 的 typed presentation_policy_change preview。这不是 D8 SetRequest，不要求尚未激活的 target 具备 policy_admin，不新增第二 CAS/ledger，也不在最终 P 之前创建已提交的 policy record/hash/address/pin/outbox/ChangeId。

只有原 bootstrap 的全部最终检查成功后，同一个 P 才按 checked allocation 分配唯一的 bootstrap ChangeId C。Core 在这个原子 P 事务中根据 frozen proposal+C 构造 canonical D8WorkspacePresentationPolicy/2，计算 fixed-prefix hash、用于 recovery 的精确 portable_metadata pin/address 与 D8PresentationPolicyOutboxItem/1，提交 presentation_policy_change 及原 receipt/effects/ChangeRecord 关联，把 D8 head graph 从 frozen empty before 推进到 heads=[address]（epoch 不变，stamp revision 经 checked increment 到 2），并同时提交 target activation/custody 与其余全部 bootstrap effect。
record.activationChangeId 和 ChangeRecord.changeId 都必须是 C，outbox decisionKey 仍是原 create/fork DecisionKey。planning loser、abort、final check 失败或 P commit 失败都必须保持 target 未激活，并且不留下 partial D8 record/pin/head/outbox。Plan4 target 成功激活后，必须立即具备 revision=1/defaultPresentation=separate 的 /1 logical current view。P 后 publication retry 只能使用 retained outbox 的 exact bytes，并执行正常的 receiver same-decision/ancestry admission。

普通 managed copy 不使用 Plan4，也不能合成这项初始化。真实 saved/planned/unknown Plan4 必须恢复原 initialPresentationPolicy、OperationId/proposal/custody mapping、pins 与 decoder，绝不能重新采样 [] 或 current branch。restore/continue/failover 保留其 recorded/current policy state，不重跑 bootstrap。真正历史 Plan1/Plan3 bytes 保持 actual decoder，不注入该字段。本候选不声称已经 active 的 Workspace 需要 migration 或 dual-write。

legacy Handle1 只在 exact Declaration1-authorized key 仍 current、safe、usable 时继续 revision-token 新签名；永远不能签 transform。continuation 对两个 profile 分别求 current(K)|none|conflicted_or_unproved，任一 unproved 都禁止部分激活新 domain。declaration order 固定为 optional old revision revoke、optional old transform revoke、new revision authorize、new transform authorize，全部在同一 DecisionKey/CP4 且无可观察中间 prefix。

# 10. D10 mixed-version current outer schemas

以下均为 current outer-holder schema。每个 versioned carrier 都必须按 tag 严格解码对应的精确 inner type，并保留其原 canonical bytes 与 pins；未知 tag 必须 fail closed。carrier 只负责选择 decoder，不增加 authority、不建立 decision/CAS，也不迁移 inner record。

```text
D10WorkspaceReadDependencies/2 = {
  kind:"d10_workspace_read_dependencies",version:2,
  workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,
  observationScope:ObservationScope/2,
  sourceInputs:[{entityRef:EntityRef,observation:SourceObservation/1,
                 role:"before"|"dependency"}...],
  dependencyProof:DependencyProof/3,
  registryReads:[{binding:RegistryBinding/1,snapshot:RegistrySnapshot/1,
                  snapshotPin:PinRef/2}...],
  evidencePins:[PinRef/2...]
}

D10VersionedControlRecordImage/1 =
    {schema:"d10_control_record_image/1",value:D10ControlRecordImage/1}
  | {schema:"d10_control_record_image/2",value:D10ControlRecordImage/2}

D10ControlRecordImage/2 = {
  kind:"d10_control_record_image",version:2,
  binding:Binding<K>/1,scope:Scope/1,
  usageRevision:Option<Counter>,view:ControlCurrentView<K>/1,
  supplement:
      {kind:"automation",subscription:ScheduleSubscription/2,
       stop:Binding<stop>/1,creator:Token}
    | {kind:"run",createdBinding:Binding<run>/1,
       principal:Token,fixedBudget:BudgetCaps/1,
       invocation:Option<AutomationInvocation/1>,
       admission:Option<LeaseRunUse/1>,
       authorSteps:[D10VersionedAuthorStepResponsibility/1...]}
}

D10ControlRecordPin/2 = {
  image:D10ControlRecordImage/2,pin:PinRef/2
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
     automation:ControlRef<automation>/1,epoch:Token,revision:Counter,
     records:[D10VersionedAutomationOccurrenceRecord/1...]}

D10VersionedControlRange/1 =
    {schema:"d10_control_range/1",value:D10ControlRange/1}
  | {schema:"d10_control_range/2",value:D10ControlRange/2}

D10VersionedControlPrepareBinding/1 =
    {schema:"d10_control_prepare_binding/1",value:ControlPrepareBinding/1}
  | {schema:"d10_control_prepare_binding/2",value:ControlPrepareBinding/2}
  | {schema:"d10_control_prepare_binding/3",value:ControlPrepareBinding/3}

ControlPrepareBinding/1 = {
  key:StableControlKey/1,canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  originalCommitRequest:PreparedCommitRequest/1,
  immutablePreview:ControlPreview/1,
  dependencyPins:ControlDependencies/1
}

ControlPrepareBinding/2 = {
  key:StableControlKey/1,canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  originalCommitRequest:PreparedCommitRequest/1,
  immutablePreview:ControlPreview/1,
  confirmationRequirement:ExternalConfirmationRequirement/1,
  dependencyPins:ControlDependencies/2
}

ControlPrepareBinding/3 = {
  kind:"d10_control_prepare_binding",version:3,
  key:StableControlKey/1,canonicalIntentBytes:Bytes,
  allocatedControlRefs:[ControlRef<K>/1],
  originalCommitRequest:PreparedCommitRequest/1,
  immutablePreview:ControlPreview/1,
  confirmationRequirement:ExternalConfirmationRequirement/1,
  dependencyPins:ControlDependencies/3
}

对 current unseen external-consent preparation，confirmationRequirement 仍是 fixed-parent attended-confirmation protocol 使用的同一个闭合 ExternalConfirmationRequirement/1，并且必须在原 commit request 交付前冻结于 ControlPrepareBinding/3。可变 confirmation fact 仍只属于 ExternalConfirmationRecord/1，不能塞进 Binding3 或 ControlDependencies/3。current confirmation eligibility 因而消费 Binding3 + Dependencies3，同时保留原 trusted event、principal、time window、immutable intent/preview、disclosure、approval_unavailable/preflight、saved-result replay 与 result-redaction 规则。已经证明的历史 ControlPrepareBinding/2 保留其精确 requirement、Dependencies2、canonical intent bytes、confirmation association 与 recovery decoder。

ControlDependencies/3 = {
  kind:"d10_control_dependencies",version:3,
  configBindings:[Binding<K>/1],
  usageBindings:[{ref:ControlRef<lease|approval|grant|cost_account>/1,
                  usageRevision:Counter}...],
  authorityProof:Token,authorizationGenerations:[Token...],
  stopRefs:[ControlRef<stop>/1...],
  workspaceReads:Option<D10WorkspaceReadDependencies/2>,
  recordPins:[D10VersionedControlRecordPin/1...],
  controlRanges:[D10VersionedControlRange/1...]
}

D10ControlEffectPlan/2 = {
  kind:"d10_control_effect_plan",version:2,
  changes:[{before:Option<D10VersionedControlRecordImage/1>,
            after:D10VersionedControlRecordImage/1}...],
  activationSelector:Option<{before:D10ActivationSelector/1,
                             after:D10ActivationSelector/1}>,
  recordPins:[D10VersionedControlRecordPin/1...],
  registryChange:Option<{
    before:{snapshot:RegistrySnapshot/1,binding:RegistryBinding/1},
    after:{snapshot:RegistrySnapshot/1,binding:RegistryBinding/1},
    evolution:RegistryEvolutionProof/1,
    beforePin:PinRef/2,afterPin:PinRef/2}>
}

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

D10ExecutionClaims/2 = {
  kind:"d10_execution_claims",version:2,
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
  kind:"d10_money_responsibility",version:2,
  reservations:[{binding:Binding<reservation>/1,
                 value:CostReservation/1,
                 attribution:CostBudgetAttribution/1,
                 settlements:[CostSettlementDecision/1...]}...],
  layers:[CostLayerTotal/1...],
  recordPins:[D10VersionedControlRecordPin/1...],
  ranges:[D10VersionedControlRange/1...],
  evidencePins:[PinRef/2...]
}

StopCapacity/1 = {
  issued:Counter,reserved:Counter
}

D10ExecutionInventory/2 = {
  kind:"d10_execution_inventory",version:2,
  workspaceRef:WorkspaceRef,storeIncarnation:Uuid,
  approvalUses:[D10VersionedApprovalUse/1...],
  claims:D10ExecutionClaims/2,
  moneyLineage:D10MoneyResponsibility/2,
  externalUnknowns:[D10ExternalResponsibility/1...],
  stopState:[D10StopResponsibility/1...],
  stopCapacity:StopCapacity/1
}

ExecutionResponsibilityRecord/3 = {
  kind:"d6_execution_responsibility",version:3,
  workspaceRef:WorkspaceRef,executionDomainId:Uuid,
  holder:
      {kind:"local_replica",replicaEpoch:Uuid}
    | {kind:"server",authorityInstanceId:Uuid,deploymentId:Uuid},
  revision:Counter,status:"active"|"paused"|"transferring",
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

Image2 只允许 automation/run；其它 record kind 继续使用精确 Image1。Pin1 保留历史 D10-Control-Record/1 前缀与 decoder。Pin2 的完整内容是 UTF8 D10-Control-Record/2、NUL 与 D3-CJ/3(Image2)；PinRef/2 的 payloadKind 是 artifact，retention 沿用 recovery 或 approval_money，byteLength 与 SHA-256 必须覆盖完整前缀内容。schema tag 与 payload domain 不一致必须失败，Pin2 不得重新 pin Image1。

对 D10VersionedControlRecordPin/1 数组，先令 I=carrier.value.image。排序依次使用 D3-CJ/3(I.binding.ref)、binding.revision 数值、usageRevision 的 none 先于 some、some 时的 usageRevision 数值、最后 D3-CJ/3(I)。同 cut identity 是 I.binding.ref、I.binding.revision、I.usageRevision。相同 identity 的第二个 byte-equal image 属于重复并拒绝；bytes 不同则是 integrity_conflict。version 不能拆分该 identity。

Range kind rank 固定为 records=0、cost_lineage=1、occurrences=2。跨版本 logical identity 分别是 scope 加完整排序后的 kinds、CostLayerKey、Automation Ref。mixed range 先按 rank 再按 identity canonical bytes 排序；每个 identity 只能出现一次。occurrences range 的内部记录按 AutomationOccurrenceKey 排序；一个 key 不能同时出现 V1 与 V2。

D10ControlEffectPlan/2.changes 按完整 after.value.binding.ref 的 canonical bytes 排序唯一。before 存在时，before.binding.ref 与 after.binding.ref 必须相等，recordPins 还必须含有与真实存储 before schema 完全匹配的 versioned pin。Image1 before 配 Pin2 非法。Image1 Automation+Subscription1 → Image2+Subscription2 是合法 current configure。若 scheduling owner 的 continue 规则成立，普通 configure 可以让旧 subscription 保持同一 generation；mixed 支持不能强迫 replace 或后台 migration。

ControlDependencies/3 的数组保持 owner 排序：configBindings 与 usageBindings 按完整 Ref；authorizationGenerations 按 token；stopRefs 按完整 Ref；recordPins/ranges 按上述 mixed 顺序。每个 config binding 都有精确匹配 image/pin；每个 usage binding 都有相同 usageRevision 的 image；每个 stopRef 都有精确 stop Image1/Pin1。current binding、usage revision、range fence、pin 与 Workspace evidence 必须来自同一个真实 Authority Store barrier。barrier A 的 V1 evidence 与 barrier B 的 V2 evidence 不能拼成 complete dependency snapshot。缺失必要历史 bytes、decoder 或 pin 时，在 disclosure 之后返回 state_unavailable；同 cut 矛盾是 integrity_conflict。

D10ExecutionClaims/2 的 canonical identity 固定如下：recordPins 使用上面的规则；prepareBindings 在 Binding1/2/3 之间统一按 inner StableControlKey；leaseRuns 按完整 Run ControlRef；authorSteps 在版本间统一按 run Ref 加 stepId；subscriptions 在版本间统一按 Automation Ref 加 generation；occurrenceRecords 按 AutomationOccurrenceKey；ranges 使用上面的规则；continuityPins 按 pinToken。每个 identity 跨版本唯一。Binding1(K) 与 Binding3(K) 即使 canonicalIntentBytes 和 originalCommitRequest byte-equal 也不能共存。Binding1 保留历史六成员精确 shape；不能增加 kind、version、confirmationRequirement 或 /2-/3 dependency field，也不能 LWW、repin。

D10MoneyResponsibility/2 的 reservation 按完整 Binding<reservation>/1 canonical bytes 排序唯一；layer 按 CostLayerKey canonical bytes 排序唯一；recordPins/ranges 使用 mixed 规则；evidencePins 按 pinToken 排序唯一。D10ExecutionInventory/2 的 approvalUses 按 DecisionKey canonical bytes 跨版本唯一，externalUnknowns 与 stopState 均按 binding.ref canonical bytes 排序唯一。所有 pending、unknown、dedup、stop responsibility，以及仍被引用的已完成 external attempt 都必须保留。只有在 admission、planning、send、schedule writer 都停在同一个真实 store barrier 后才能捕获 inventory。

Inventory2 artifact 的精确 payload 是 UTF8 D6-Execution-Inventory/2、NUL、D3-CJ/3(D10ExecutionInventory/2)。PinRef/2 为 artifact/recovery，byteLength 与 SHA-256 覆盖完整前缀 bytes。Inventory1 保留 D6-Execution-Inventory/1，绝不重新 pin 成 /2。

Record3.workspaceRef 必须等于 Inventory2.workspaceRef。Record3 的 approvalUses、claims、moneyLineage、externalUnknowns、stopState 五类 payload 分别与 Inventory2 对应成员 byte-equal。Proof2.inventoryPin 必须选择这一份精确 Inventory2，且 Inventory2.storeIncarnation 等于 Proof2.storeIncarnation。受保护的 birth/barrier/fence token mapping 必须证明同一实际 store 与 capture barrier；不新增独立 caller-supplied store-incarnation proof object。

Inventory2.stopCapacity 必须是同一 authoritative safety-store barrier 下的精确 fixed-parent StopCapacity/1。StopCapacity/1 仍只有 issued、reserved，不在 Record3 中重复。Workspace-only handoff 不能把 shared safety counter 复制到第二个 active store。完整 store handoff 必须 fence 全部受影响 writer，并保留所有 target/latch reservation 与 safety capacity；边界不可证明时 takeover unavailable。

Record2、Proof1、Inventory1 都保留历史 decoder/domain。Record3 只在真实 responsibility mutation、checkpoint 或 custody handoff 时产生，必须保持 executionDomainId；unchanged holder 不做后台 migration。

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

ScheduleContinuityInvalidation/2 = {
  kind:"d6_schedule_continuity_invalidation",
  version:2,
  automation:ControlRef<automation>/1,
  subscriptionGeneration:Counter,
  expectedWitnessRevision:Counter,
  producerEpoch:Token,
  transition:Counter,
  status:"binding_changed"|"gap",
  evidencePins:[PinRef/2...]
}
```

当前 continuity artifact 的编码按版本精确分域：

```text
Witness2ArtifactBytes =
  UTF8("D6-Schedule-Continuity/2") || NUL ||
  D3-CJ/3(完整 ScheduleContinuityWitness/2)

Step2ArtifactBytes =
  UTF8("D6-Schedule-Step/2") || NUL ||
  D3-CJ/3(完整 ScheduleContinuityStep/2)

Invalidation2ArtifactBytes =
  UTF8("D6-Schedule-Invalidation/2") || NUL ||
  D3-CJ/3(完整 ScheduleContinuityInvalidation/2)
```

三者对应的 PinRef/2 都使用 payloadKind=artifact、retentionClass=recovery。byteLength 与 SHA-256 必须覆盖完整 domain prefix、唯一的 NUL byte 和完整 canonical object bytes。digest 本身不认证 continuity；仍必须验证受保护 producer/subscription provenance、producerEpoch、精确 generation、retained transition chain，以及原 authorization/disclosure gate。

decoder 必须先按 authenticated artifact domain 闭合分派，再解 inner object。D6-Schedule-Continuity/1 只能解 historical ScheduleContinuityWitness/1；D6-Schedule-Step/1 只能解 historical ScheduleContinuityStep/1；D6-Schedule-Invalidation/1 只能解 historical ScheduleContinuityInvalidation/1。D6-Schedule-Continuity/2 只能解 ScheduleContinuityWitness/2；D6-Schedule-Step/2 只能解 ScheduleContinuityStep/2；D6-Schedule-Invalidation/2 只能解 ScheduleContinuityInvalidation/2。unknown domain、已知 domain 搭配错误 kind/version、其它 prefix、缺失 NUL 或非 canonical object bytes 都必须按原 disclosure/error boundary fail closed。禁止 fallback decoder、扩展 /1 decoder、repin 或重新编码历史 bytes。

```text
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
         {protocolOwner:"D3",request:D3IdentityOperationRequest/13}
       | {protocolOwner:"D6",request:d6_commit_request/2},
     preparedFormat:
       "d7_prepared_action_binding4"|"d8_prepared_edit_binding3",
     preparedRecordPin:PinRef/2,recoveryPins:[PinRef/2...]}
```

对 fresh current core_field_member step，D10AuthorPreparationLink/2.preparedBindingToken 必须选择精确 PAB4，且该 PAB4 的原 request 必须等于 link.request。Core 必须在返回 prepared step 或允许 submission 前，原子保存 Link2、完整 PAB4 与所需 preview/effect/recovery pins。ApprovalUse/2.preparedBindingToken 选择同一个 PAB4。其 preview digest 固定为
SHA-256(UTF8("D10-Author-Preview/2") || NUL || D3-CJ/3(normalized complete EffectManifest/3))；
EffectBytes/3 slot 只投影为 {encoding,byteLength,payloadDigest}，不按成员名递归猜测。fresh current automatic author qualification 使用 DependencyProof/3；只要语义解析 managed Document，就必须包含 document_format，并且只有在完整验证 EffectManifest/3、EffectBytes/3 与 MutationFootprint 后才能构造 ApprovalUse/2。历史 Link1/PAB3/ApprovalUse1 的 saved 或 planned association 保留原 decoder、bytes、pins、request 与 OperationId。

对 fresh ScheduleSubscription/2 registration，current D6 Storage producer 只有在 selected source/Field/Registry/current scheduling gates 通过、有限 retention 已预留、且真实持续维护的 Core source/control transition producer 已在同一 configuration transaction 注册后，才能创建 ScheduleContinuityWitness/2。initial 与 checkpoint 使用 subscription 的精确 ScheduleRecurrenceEvidence/2，revision=1、consumedTransition=0，并生成 fresh producerEpoch。retained witness pin 必须精确使用上面的 Witness2ArtifactBytes。current 正向 transition 使用 ScheduleContinuityStep/2 与 DependencyProof/3；新的 current portable transition 使用真实 ChangeRecord/1、InstallationNotice/3 与 ContentCompletionProof/4 pins，每个 retained current step pin 都必须精确使用上面的 Step2ArtifactBytes。retained history 中的历史 transition 保持原 /1 artifact domain 与精确 decoder。相关 P-only control/rule transition 仍从真实 protected before/after state 捕获。

fold、compaction、receiver admission、D10 continuityPins consumption 与 recovery 都必须先按每个 protected schedule-continuity artifact 的精确 domain 分派，再解 inner object。current Witness2/Step2 不能放在 /1 domain 下接受，historical Witness1/Step1 也不能放在 /2 domain 下接受。continuityPins 是按 pinToken 排序且唯一的精确 PinRef/2；真实历史若跨过显式 same-generation bridge，可以保留 version-mixed original typed chain，但每个元素都保留自己的精确 bytes/domain/decoder。另一条合法 full retained-chain 路径同样保留每个 original typed artifact 与 source/control evidence，不能把整条 chain 归一化成一个版本。typed evidence 缺失或 unknown 时，在原 authorization/disclosure check 后沿用原 gap/unavailable 行为；已证明 domain/object mismatch 时不得用相同 digest/current state 修复。

当不存在合法 current after evidence 时，current producer 必须产生 ScheduleContinuityInvalidation/2，不能伪造 ScheduleRecurrenceEvidence/2。binding_changed 需要完整可信的 selected-business discontinuity 证据；after unavailable/unknown、missing history、unknown decoder、observer/producer gap 或无法保留必要 transition 均为 gap。Invalidation 比较同一 current witness/registration，原子推进下一 checked transition/revision，保留最后合法 checkpoint，并对该 generation 永久不可 reset。其 artifact pin 必须精确使用上面的 Invalidation2ArtifactBytes。fixed-parent inbox/capacity/final-counter reservation、authorization、compaction 与无关 source 可用性规则保持不变。

Schedule current proof 中真实解析 managed Document 时必须含 source + document_format dependency。source/profile bytes 未变但 format proof continuity gap 得到 gap；binding 发生真实改变得到 binding_changed，即使最终 recurrence/range 值碰巧相同。已有 Subscription1 继续作为 historical retention owner，并配套 Witness1/Step1/Invalidation1 及其精确 /1 artifact domain。它只有通过 explicit continue + complete retained history 证明没有 intervening format/rule/business discontinuity，并建立 current Evidence2/Proof3 cut，才可变成 same-generation Subscription2；否则必须 replace。这个 bridge 保留每个旧 pin 与 producer association；只有 bridge 之后新产生的 Witness2/Step2/Invalidation2 才使用 /2 domain。绝不把 version-1 witness/step/invalidation repin 或重新编码成 version 2，也绝不 reset invalid generation。

# 11. 历史分派与单一 authority

saved先按原 owner/version重放；planned恢复原descriptor/proof/prepared/pins/Notice/install/version basis；unknown保留原Approval/Money/claim/external/stop责任；只有unseen走current successors。旧bytes/pins绝不因进入mixed wrapper而重编码。

所有本文 successor继续使用原单一 DecisionKey、planning CAS、installation、P seal、receipt/outbox。document_format不是第二Document authority；SourceTransform不是source authority；ChangeRecord不是第二portable truth；D10 versioned wrapper不赋新authority；Derived Index永远不能重建current proof。

# 12. Provider profiles

Provider closed shapes见 SPEC §4。它们是生态renderer资格，不改变core-language validity。Mermaid fixed source是 CLI 12.0.0 commit db1ceebbe529d7975474eb0d0e9c23e9dc57cd37，并固定package.json/blob与package-lock/blob；Node/browser/fonts/config仍必须用实际version+digest登记。没有已登记runtime就只能unavailable，不能用浮动semver、host browser/font或网络下载补齐。

# 13. 验收引用

所有上述schema与cross-field规则的正负设计义务逐项列在 [ACCEPTANCE.zh-CN.md](ACCEPTANCE.zh-CN.md)：438 core +322 coordination，共760，全部未运行。该文件的ID正文是规范的一部分，不允许用计数替代。
