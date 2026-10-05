---
source_language: zh-CN
translation_status: source
---

[English](SPEC.md)

# Weftext AsciiDoc / Annotation 最终设计候选：联合 owner replacement

状态：**candidate-design-not-implemented**。本规范绑定父输入 `e8aa0b341630a57c786c0891d4bbd1620247441d`，只允许作为其上的协调设计候选；不表示本 PR、A2、全局设计、实现或发布已接受。

## 0. 优先级、范围与不可分拆理由

本文件与 [SCHEMAS.zh-CN.md](SCHEMAS.zh-CN.md) 共同组成 `replacements.json` 所列 actual-owner 章节的完整 normative replacement：本文件冻结行为/算法/owner边界，SCHEMAS冻结closed current shapes、排序、cross-field与历史分派；本文件中的摘要措辞不得扩张或缩减SCHEMAS。未被 replacement router 点名的 fixed-e8aa owner正文保持原义。S=`7e18168dad3e6d120fce0dd607dc10fa7894e252` 的49个 snapshots 与 `docs/design/inputs.json` 不修改。

本候选把已经独立核销的核心 v3.10 与 FC-3.4→a→b→c→d 合成为公开、可独立审查的 current proposal。旧候选名、私有聊天、作者自评都不是协议依赖。

原本考虑拆成“AsciiDoc/format”和“Annotation/SourceTransform/trust”两批；固定 e8aa 对账后不能安全拆分：current `DependencyProof/3 → PreparedActionBinding/4 → D10 ApprovalUse/2` 与 `EffectManifest/3 / EffectBytes/3` 同时承载 managed-format currentness、D10 author recovery 与 `WorkspaceBootstrapPlan/4`。先发布一个临时 `/3` 闭集、第二批再给同版本加 bootstrap/transform arm 会破坏 closed decoder。因此本 PR 是一个联合设计 PR，但正文按 D2–D10 owner 分章。

设计原则：一套 author authority、一套 D6 planning CAS、一套 P seal；不新增第二 source truth、第二 portable ledger、第二 decision 或自由 adapter gate。Derived Index 永远可删，不能恢复 current authority。

## 1. D2：完整 AsciiDoc 语言合同

### 1.1 固定语言基线与生产实现边界

Weftext Managed Document 的 AsciiDoc core-language 语义固定为：

```text
asciidoctor-ruby/2.0.26
commit 0b99b39c9df884d4aec13bba45f03cdbab505769
```

Ruby Asciidoctor 只用于独立 conformance oracle，不进入生产依赖。生产实现必须为 Rust-native；成熟实现优先，Asciidork `79671b57923841e41fc91d2c1d2fce18cca3131c` 只是候选实现，不是兼容范围上限。实现成本、富文本控件缺失或 provider 缺失都不能把合法 core-language 缩成安全子集。

完整 core contract 包括固定2.0.26的 block/inline/list/table/description/callout/checklist、attribute list、document attributes、includes、substitutions、macros、passthrough、STEM、media、footnote/indexterm/catalog、doctype/backend model、document title/author/revision、block title/reftext、native TOC/section numbering、`leveloffset`、diagnostics以及处理环境对可观察语义的影响。合法但当前没有结构化编辑器控件的 construct必须可解析、读取、Source编辑、保存和无损 round-trip。

标准 title separator保持原生2.0.26语义：默认 `: `，显式 `[separator=::]` 与 `:title-separator: ::` 按原语义工作。**新建 Weftext 模板是否默认 `::` 仍未决，本规范不选择。**

### 1.2 ProcessorEnvironment

Core Gate 的环境必须冻结而不是读取 ambient host状态。`AsciiDocProcessorEnvironment/3` 至少闭合：doctype、backend语义profile、safe mode、base/input/output dir、attribute hard/soft set/unset四态及顺序、user-home、locale/encoding、时间/日期/epoch输入、include resolver、网络/文件读取授权、extension registry、provider profile和所有会改变固定2.0.26 observable semantics的输入。source authored attribute events与host/builtin/include provenance分开；`wf-kind`/`wf-facets` 等 Weftext authored control只能消费 root-authored provenance，不能由 host注入或 included source反向接管 root Node classification。

### 1.3 标准标题、深层标题与 run-in

标准 AsciiDoc section状态机保持原义，包括 discrete/floating title、book part/chapter、skip warning、fragment relaxation、negative leveloffset clamp和 doctitle条件。Weftext Managed profile只增加：

- authored section level 6–9；
- `leveloffset` 继续是标准语义，effective level不因 authored 6–9而硬截断，可超过9；
- stable Node link/backlink投影不借 heading text生成身份；
- run-in heading与body在 source/semantic tree中仍是两个独立节点，可在目标 renderer 同行呈现；复制、导出、Source编辑与可访问性不能把二者合并成一个不可逆字符串。

TOC、search、outline、backlink、export和editor都消费同一 section semantic projection。renderer不支持某深度时是 backend capability结果，不得改写 core tree。

### 1.4 Stable Node links 与 source provenance

Weftext Node link/citation/resource adapters建立在 native AsciiDoc link/image/attribute-list parse结果之上，不另写第二 link parser。NodeRef/ResourceRef identity由 D3提供；文本、路径、title、hash都不能猜身份。managed include span属于其真实 source owner；unmanaged/network include只能作为获准 snapshot参加一次求值，不能自动成为 durable exact annotation/write owner。

每个 semantic span保留完整 source-origin graph。合成多源文本若没有唯一 write origin，Source仍可读但结构化write必须 fail closed。

## 2. D2 测试 Oracle：Witness/7 与 CoreSemanticProjection

### 2.1 双层 Gate

对同一 source S 与冻结环境 E：

```text
W = observedRuby226(S,E)
C = completedWeftextCoreEvaluation(S,E)
R = projectRuby(W)
T = projectWeftext(C)

CoreSemanticGate = D3-CJ/3(R) == D3-CJ/3(T)
```

前提还必须同时满足原输入/环境绑定、observer-neutrality、两侧projection complete。禁止以“Ruby HTML == Rust HTML”、raw trace相等或少数选中字符串相等代替。

Ruby observer是测试旁路，不是产品 parser、authority或持久状态；不得替换 selected backend、插 marker/sentinel、改 evaluator string/encoding、额外调用 `content/text/title/reftext/xreftext`，也不得为了补证据再次运行 parser/regex。实际 converter返回值继续原样参与后续 substitutions。

### 2.2 `OracleSemanticWitness/7`

Witness/7保留 Witness/6 的 document/block/collection/catalog/diagnostic/observedStrings/calls/operations/inline/content evidence，并增加：

```text
modelPropertyObservations:[OracleModelPropertyObservation/1...]
```

model观察只旁路复制实际 producer已经写入的 primitive field/attribute/关系，不额外求值。Section三态、caption/numeral、最终Table列宽、final Cell对象/style/alignment/span、ListItem checklist/coids、真实author named/positional/rekey、临时converter写均必须从固定producer取得，不由projector重跑Ruby model算法。

### 2.3 snapshot/reference/cut

`modelPropertyObservations[].observationId` 在本数组内唯一、严格随append递增。snapshot identity就是其 `observationId`；不存在第二snapshot ID namespace。每个subject形成 immediate-predecessor单链：首snapshot previous=null，其后必须恰指同subject直接前一snapshot。

bind/cut snapshot引用必须存在、同subject且是相应时点的 latest snapshot。cut heads按数值subjectId排序，每个live semantic subject恰一head，并满足：

```text
set(cut.heads.subjectId) == reachableSemanticClosure(cut)
```

forward discovery只沿真实header/child/dlist/table-column/final-cell/cell-inner-document等semantic关系；`parent`/`cell_column`只做反向一致性，不把stale对象重新拉入。treeprocessor返回的新Document成为真正root；旧Cell被 `reinitialize` 替换后不得同时live。

`model_ready` 恰在实际 top-level `Document#parse` 完成（包括restore attrs/treeprocessors及实际returned Document）后产生。成功selected-backend evaluation恰一个 `evaluation_complete`；异常退出不得伪造complete cut。post-ready真实semantic write进入新head；temporary物理write不能通过挑旧head绕过。

### 2.4 namespace分类与 producer-time bind

只有同属 `modelPropertyObservations.observationId` 的 previous/bind/cut snapshot和 model-slot observation引用使用数值先后比较。`operationId`、`inlineEventId`、`valueId/observedStringId`、`callId` 各按自己既有namespace的存在/类型/DAG规则；`ModelCarrierReference.entry` 是对应carrier数组的0-based index，绝不与model observationId比较。不存在global event ordinal。

bind-time chronology不能由最终wire静态证明。受控observer producer必须在真实callback中满足：目标carrier已经append，`entry < targetStream.lengthAtBind`，且producer仍持有exact同一Ruby object；随后才append bind。`document/0`同理，实际returned Document carrier必须先发布。最终decoder只能验证最终stream/index/type与object-binding closed关系，不能声称单靠最终数组证明跨stream先后。后来补齐carrier不能治愈此前非法bind。

### 2.5 catalog 与 temporary state

catalog subject的唯一ownership关系是：

```text
parent -> actual Document#register receiver Document subject
```

inner AsciiDoc cell的inner Document注册catalog时parent必须指inner Document，不指top-level root。

DocBook `root-option` load-bearing物理路径按固定源码记录真实行为，例如 authored value → internal `set_option` temporary value → `remove_attr` absent/deleted；observer不得伪造restore写。latest physical snapshot与PropertyProfile的author semantic winner是两层：internal cleanup不能抹掉此前合法author provenance，也不能复活真正后来的author overwrite/delete。

## 3. CoreSemanticProjection/1 与 PropertyProfile

### 3.1 投影

`CoreSemanticProjection/1` 至少含 document、blocks、catalog、indexTerms、finalState、diagnostics。`CoreFlow/1` 用 final runs + marks表示文本/格式；坐标按 Unicode scalar计数。实际 visible text只拥有一次；hard break、footnote ref等用atom表达，不能由body/inline/break重复复制。

intermediate Ruby execution ID、call count、converter-return节点、临时dependency edge不进入 equality surface；但其对最终target、text、catalog、counter、attribute state的真实影响必须保留。authored passthrough与backend-feedback raw不能猜成普通 formatting mark。

### 3.2 Canonical PropertyProfile

属性按 A/P/F/I provenance分类：authored named、authored positional、fixed-derived semantic、internal。positional经过实际rekey只留下一个canonical slot；真正任意author named属性即便renderer未使用也保留。内部`cloaked-context`、cache、reader、temporary `root-option`等不能to_s偷渡。

公共语义至少包括 id/style/ordered roles/options/caption/numeral/explicit subs/未消费positional和 `named/<name>`。quote/verse attribution+citetitle、source listing language/linenums、section sectname/special/numbered、table/column/cell width/alignment/span/style、list marker/checklist/coids、media参数及mark/atom的实际语言字段按固定2.0.26 producer映射。

显式作者 `foo-option=bar` 同时保留：

```text
options += "foo"
named/foo-option = "bar"
```

显式空值保留空字符串；`%foo` / `options=foo` / `opts=foo` 只保留membership，不虚构author named空值。真实writer顺序决定winner。

### 3.3 Canonical arrays/diagnostics

语言有序序列保持顺序；set排序唯一；multiset按D3-CJ/3(item) UTF-8 bytes排序并保留multiplicity；unique-key map冲突拒绝而非LWW。roles/columns/children/menu path等语义序列不能全局排序。

`CoreDiagnostic/1` common location只包含：

```text
null | {logicalFile:text|null,line:UInt|null}
```

没有Ruby producer的column不进入cross-implementation equality；semanticCode使用共同canonical namespace，exactMessage留作Ruby evidence审计。

## 4. Provider profiles：语言有效性与renderer可用性分离

Mermaid、STEM/TeX/AsciiMath、HTML/PDF是版本化生态profile，不是AsciiDoc语法开关。合法core source在provider不可用时仍合法；BackendStatus表达unavailable/denied/incomplete，exact `.adoc` export仍可成功。

### 4.1 MermaidProviderProfile/1

已固定上游：

```text
@mermaid-js/mermaid-cli 12.0.0
tag commit db1ceebbe529d7975474eb0d0e9c23e9dc57cd37
package.json Git blob 883e27cc3e536e4efa9a48351626d8126ae9d36d
package-lock.json Git blob b814e336ee6af8ba265a9c29e86cb8c3ca4a923d
packageManager npm/10.8.1
engines.node >=22.13.0
peer puppeteer ^25.0.0
```

lockfile Git blob是递归npm依赖的固定输入；运行时仍不得只靠这些range。可执行provider必须注册完整：

```text
MermaidProviderProfile/1 = {
  kind:"mermaid_cli", version:1,
  cli:{version:"12.0.0",sourceCommit:"db1ceebbe529d7975474eb0d0e9c23e9dc57cd37",
       packageJsonGitBlob:"883e27cc3e536e4efa9a48351626d8126ae9d36d",
       packageLockGitBlob:"b814e336ee6af8ba265a9c29e86cb8c3ca4a923d"},
  node:{version:text,executableSha256:"sha256:<64 lowercase hex>"},
  browser:{product:text,buildId:text,executableSha256:"sha256:<64 lowercase hex>"},
  fonts:[{family:text,assetSha256:"sha256:<64 lowercase hex>",licenseId:text}...],
  config:{byteLength:Counter,sha256:"sha256:<64 lowercase hex>"},
  network:"denied"|{kind:"allowlist",origins:[text...]},
  localResources:"denied"|{kind:"explicit_pins",pins:[PinRef/2...]},
  output:"svg"|"png"|"pdf",
  outputValidator:"weftext-render-output/1"
}
```

`node.version`必须满足>=22.13.0且同时固定可执行digest；browser/font/config缺任一exact字段时provider为unavailable。CLI 12参数按当前合同使用 `size`（不再width/height）、`pdf-paper-format`（不再pdfFit）、`themeCSS`（不再cssFile）。默认嵌字体不允许被解释成“任意host字体等价”。不得`npx`在线补包或按浮动semver取得另一个依赖树。

SVG验证拒绝 script、event handler、javascript URI、DTD/entity、未授权network URI/CSS/font；unsafe输出直接拒绝，不“清洗后宣称完整”等价。

### 4.2 STEM / HTML / PDF profiles

```text
StemProviderProfile/1 = {
  kind:"stem",version:1,
  notation:"tex"|"asciimath",
  engine:{name:text,version:text,artifactSha256:"sha256:<64 lowercase hex>"},
  fonts:[{family:text,assetSha256:"sha256:<64 lowercase hex>",licenseId:text}...],
  config:{byteLength:Counter,sha256:"sha256:<64 lowercase hex>"},
  network:"denied"|{kind:"allowlist",origins:[text...]},
  localResources:"denied"|{kind:"explicit_pins",pins:[PinRef/2...]}
}

HtmlRendererProfile/1 = {
  kind:"html",version:1,
  renderer:{name:text,version:text,artifactSha256:"sha256:<64 lowercase hex>"},
  config:{byteLength:Counter,sha256:"sha256:<64 lowercase hex>"},
  resourcePolicy:"embedded_or_explicit_pins/1",
  outputValidator:"weftext-html-output/1"
}

PdfRendererProfile/1 = {
  kind:"pdf",version:1,
  renderer:{name:text,version:text,artifactSha256:"sha256:<64 lowercase hex>"},
  browser:null|{product:text,buildId:text,executableSha256:"sha256:<64 lowercase hex>"},
  fonts:[{family:text,assetSha256:"sha256:<64 lowercase hex>",licenseId:text}...],
  config:{byteLength:Counter,sha256:"sha256:<64 lowercase hex>"},
  network:"denied"|{kind:"allowlist",origins:[text...]},
  outputValidator:"weftext-pdf-output/1"
}
```

STEM native HTML路径可能默认引用CDN；offline/read/export不得暗中联网。若所选engine/resource pins不足，renderer返回unavailable，语言/CSP仍保持合法。HTML/PDF provider同样必须固定实际artifact/config/font/browser（若使用浏览器），不能从host默认值补齐。

**本PR没有安装或运行上述providers。** 当前没有已登记的exact Node/browser/font运行profile，因此不得声称Mermaid/STEM/PDF renderer测试通过；这是明确的implementation evidence gap，不是缩减语言目标。

## 5. D2/D6 Managed document format

### 5.1 唯一managed profile

```text
ManagedDocumentFormatProfile/1 = {
  languageBaseline:"asciidoctor-ruby/2.0.26@0b99b39c9df884d4aec13bba45f03cdbab505769",
  managedProfile:"weftext_managed/1"
}

ManagedDocumentFormatBinding/1 = {
  kind:"weftext_managed_document_format",
  version:1,
  ownerNodeRef:NodeRef,
  bindingRevision:Counter,
  profile:ManagedDocumentFormatProfile/1
}
```

BaselineOnly是未绑定managed profile时的ordinary analysis路径，不是portable binding arm。managed current Node缺binding是 incomplete/proof_unavailable，不fallback到BaselineOnly或latest profile。

fresh managed Node bindingRevision=1。same-Workspace copy/fork fresh identity建立新的binding revision1但保持exact profile generation；formal same-Workspace restore恢复exact historical binding/revision；普通backup bytes import没有身份continuity，重新BaselineOnly analysis→managed delta→显式admission→fresh revision1。真正managed profile migration才checked +1；source unchanged的profile-only migration有一个portable ChangeId但 `sourceChanges=[]`，不增加SourceVersion/H。

### 5.2 current qualification

```text
DocumentFormatCurrentQualification/1 = {
  kind:"d2_document_format_current_qualification",version:1,
  key:DocumentFormatDependencyKey/1,
  stamp:{epoch:Token,revision:Counter},
  componentImage:{state:"present",version:Counter,byteLength:Counter,sha256:"64-lowercase-hex"},
  binding:ManagedDocumentFormatBinding/1,
  bindingPin:PinRef/2
}
```

key.ownerNodeRef==binding.ownerNodeRef；componentImage.version==binding.bindingRevision；bindingPin=portable_metadata且exact pin `D3-CJ/3(binding)`。`ManagedDocumentSemanticQualification/1` 绑定 current `SourceObservation/1 + DocumentFormatCurrentQualification/1`；两者任一变化都令旧projection/preparation stale。

## 6. D6 dependency/component successors

### 6.1 DependencyKey/3

十五臂rank固定：

```text
0 source
1 document_format
2 lifecycle
3 placement_range
4 ref_inbound
5 relation_incidence
6 calendar_scope
7 registry
8 temporal_rules
9 authorization
10 foreign_binding
11 query_scan
12 replica_registry
13 conflict_record
14 execution_resource
```

`document_format={kind:"document_format",workspaceRef,ownerNodeRef}`。`DependencyProof/3`、`InputDescriptor/3`、`PreparedIntent/3` 保持旧outer职责但使用Key/3。旧 `/2` 永远仍是十四臂旧rank，不回填第十五key。

### 6.2 PortableComponentKey/2

rank固定：

```text
0 document
1 document_format
2 resource
3 annotation
4 node_binding
5 child_list
6 lifecycle
7 trash_membership
8 policy
9 registry
10 period_scope
11 replica_registry
12 conflict
```

`document_format={kind:"document_format",ownerNodeRef}`；component bytes strict-decode `ManagedDocumentFormatBinding/1`，`ComponentImage/1.version=bindingRevision`。`PinRef/2`、`ComponentImage/1`保持原shape。

`InstallationNotice/3` 与 `ContentCompletionProof/4` 只因 component-key union和current production family升级；CP4 committed/restored责任、frontier、sourceChanges及strict install沿e8aa CP3算法。旧Notice2/CP3原bytes/decoder不扩臂。

### 6.3 ChangeRecord/1

本设计首次规范闭合当前ChangeRecord；不宣称未知历史bytes属于该版本：

```text
ChangeRecord/1 = {
  format:"weftext.change-record",version:1,
  decisionKey:DecisionKey/2,
  changeId:ChangeId/1,
  installationNotice:{format:"weftext.installation-notice",version:3,byteLength:Counter,sha256:"64-lowercase-hex"},
  completionProof:{format:"weftext.content-completion",version:4,byteLength:Counter,sha256:"64-lowercase-hex"},
  frontierBefore:Frontier/2,
  frontierAfter:Frontier/2
}
```

同一P seal固定ChangeId、CP4和ChangeRecord bytes；Notice3已在install前由原plan持久化。ChangeRecord只索引exact Notice/CP和frontiers，不复制components/sourceChanges，不是第二author truth。publication失败只重发原pinned bytes。pre-FC记录没有真实decoder时，strong causal consumer得到proof gap，不能套 `/1`；ordinary source读写不因此全局禁用。

## 7. D3/D4/D5 current consumers

D3 current native request升级为 wire13/`D3IdentityInput/13`，消费 `InputDescriptor/3`；`D3DecisionCompanion/2`不变。`D3ResolutionInputUse/2` exact持 `InputDescriptor/3`，历史 `/1` 仍只接受Descriptor2。

D4 current outer qualification使用Descriptor3/Proof3/15-arm key；真正解释managed Document semantics时必须同时有 `source(owner)+document_format(owner)`。D4 inner `RelationReadContext/2`、`RelationReadBinding/2`、OccurrenceKey、numeric sourceRevision、Calendar/Registry types都不机械升版。

D5 table/collection/Field current outer同样消费proof3/descriptor3与document_format；format stamp变化即使SourceVersion相同也 stale/reprepare。table/row/cell locator、OccurrenceKey、RevisionTokenBinding、SourceObservation和D5 intent grammar保持原型。

Trash保留exact managed format binding；restore保留exact historical binding；purge在同一strict plan/Notice3/P/CP4中把document_format从present改absent。历史retained bytes只服务history/recovery，不能自动成为current qualification。

## 8. D7/D8/D9 current holders

`PreparedActionBinding/4` 继承PAB3业务成员并直接持 current Descriptor3/Proof3/PreparedIntent3；`MinimumMapping/3`不因整齐升版。Query/Action/CEL语言本身不因新dependency arm改语法；只有qualification/evidence carrier升级。

`EffectManifest/3` / `EffectBytes/3` 是同一closed current transport，包含既有EffectBytes2 encodings及 `d6_workspace_bootstrap_plan4` / `d7_symbolic_json3`；historical Plan1/Plan3继续旧decoder。D10 preview digest和D3 conflict preview均消费这一current family。

D8 `PreparedEditBinding/3` 持 current descriptor/proof。Draft base绑定 SourceObservation+DocumentFormatCurrentQualification；format变化保留dirty Draft bytes/input log，但失效旧projection/map/preview/prepared confirmation，必须reproject/reprepare。

D9 `ExportPlan/3` / `PublicationReceipt/3` 冻结proof3与current semantic qualification；exact-source-only export不需要无关semantic parse，而rendered/semantic export必须列format dependency。e8aa `d9_probe` closed format union没有adoc/asciidoc；本设计不偷偷给旧wire加arm。新managed AsciiDoc adoption由D2/D3/D6 admission/profile流程负责。

## 9. 独立 portable JSON Annotation

Annotation 是 D3 `AnnotationRef` identity 下的 portable current value，不嵌进 Document source，不建立第二 MessageId。每个 root/reply 都是独立 AnnotationRef；thread 机械等于 same-owner `reply_closure`，cycle 拒绝。并发 fresh replies 可并存；同一 Annotation 并发 edit 通过 revision/CAS/ConflictRecord，不用 timestamp LWW。

### 9.1 `D3-Annotation-Value/4`

current closed value 固定为：

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
```

unknown/missing/duplicate member、非法 null、非 UTF-8 均拒绝。labels 最多64项，每项1..128 UTF-8 bytes、exact unique；body source最多65536 bytes。

purpose invariants：

- root `comment`：body required/nonempty，suggestion=null，reviewState=open|resolved；
- root `mark`：appearance required，suggestion=null，reviewState=open|resolved；
- root `suggestion`：suggestion required，reviewState=open|resolved，body可作为review rationale；
- replyTo!=null：same owner、acyclic、purpose=comment、suggestion=null、reviewState=not_applicable；
- resolve/reopen只改thread root；target resolution与reviewState正交。

### 9.2 `D3-Annotation-Target-Projection/1`

唯一 identity/locator projection 是 closed union：

```text
document       {kind:"document",owner:NodeRef}
document_element{kind:"document_element",locator:DocumentElementLocator}
document_range {kind:"document_range",locator:DocumentRangeLocator}
resource       {kind:"resource",resourceRef:ResourceRef}
resource_region{kind:"resource_region",locator:ResourceRegionLocator}
```

每个 variant 只允许列出的字段。owner/locator/resourceRef 必须与 AnnotationRef.owner 相同；不允许 AnnotationRef、裸NodeRef、AuthorAnchorAddress或跨owner locator/ref替代target。outer wire与 Projection/1 必须双向唯一materialize；path/title/hash/ambient owner不得补字段。

### 9.3 `AnnotationInlineProfile/1`

```text
AnnotationInlineProfile/1 = {
  languageBaseline:"asciidoctor-ruby/2.0.26@0b99b39c9df884d4aec13bba45f03cdbab505769",
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
```

完整 source 必须只形成一个 paragraph（允许 soft wraps 与 trailing whitespace）；第二paragraph、heading、list、delimited block、table、block macro均 `invalid_annotation_body`，不能被 inline doctype 静默忽略。允许固定2.0.26 inline strong/emphasis/mono/mark/roles/URL links/xref/STEM/footnote等；n1/r1/weftext-cite/carrier/query/view/deep-heading/run-in managed adapters在此profile不激活。

### 9.4 Appearance、Actor 与时间

```text
AnnotationAppearance/1 = {
  mark:"highlight"|"underline"|"squiggle"|"strike",
  theme:"yellow"|"red"|"green"|"blue"|"purple"|"pink"|"gray"
}

AnnotationActorSnapshot/1 = {
  kind:"annotation_actor_snapshot",version:1,
  originWorkspaceRef:WorkspaceRef,
  authentication:"workspace_authenticated_origin"|"device_local_unverified"|"imported_unverified",
  displayName:text
}

AnnotationTimeSnapshot/2 = {
  instant:RFC3339-with-offset,
  producer:"prepare_server_clock"|"prepare_device_clock"|"imported_unverified"
}
```

actor/time 是历史归因，不参与LWW。creation时 caller不能填写四个归因字段；Core在trusted preparation注入 creator/authoredAt，并令 lastEditor=creator、editedAt=authoredAt，随后冻结在原PreparedIntent，replay不重新采样。later mutation必须 byte-equal 保留 creator/authoredAt，由Core重写lastEditor/editedAt。same-Workspace copy保留原review attribution；cross-Workspace trusted transfer保留 originWorkspaceRef；ordinary untrusted import使用 imported_unverified。

### 9.5 `Suggestion/3`

```text
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

pending 的 confirmation 为 confirmed|needs_reconfirmation；accepted/rejected 必须 not_applicable 且 terminal。targetBasis唯一为 `SHA256("Weftext-Suggestion-Target-Basis/1" || NUL || D3-CJ/3(complete stored target))`；不含quote/prefix/context/display text。

- replace：nonempty document_range；expectedText required；replacementSource required（可空）；pointAffinity=null；
- delete：nonempty document_range；expectedText required；replacementSource=null；pointAffinity=null；
- insert：zero-width document_range；expectedText=null；replacementSource nonempty；pointAffinity left|right。

## 10. Annotation targets/current qualification

### 10.1 source target

source span target绑定真实 owner、basis production SourceVersion、byte range、point affinity、expected bytes/context及source provenance。root-authored span owner=root Node；managed include span owner=included Node。unmanaged/network/synthetic multi-origin不能直接制造durable exact target；可annotate directive或先adopt/reanchor。

resolution状态必须区分：

```text
exact
mapped
candidate
ambiguous
orphaned
unavailable
```

`candidate`/fuzzy/context search永不取得正文write authority。重复文本多个候选必须ambiguous，不选first/nearest。

### 10.2 media target

PDF/image region绑定exact ResourceVersion + normalized rectangle；zoom/UI shell变化不改region。audio/video使用rational timebase/ticks与 `[start,end)`。decoder/profile不可用时已有raw annotation保留，新exact region creation unavailable。

### 10.3 history与权限

在线读取old quote/context必须同时有annotation disclosure和对应historical source disclosure。raw backup已经合法交付的bytes不能被未来revoke“收回”；当前API仍按current permissions。Review Bundle没有history权限就不能偷偷带historical excerpts。

## 11. SourceTransform compiler / exact mapping

### 11.1 total compilation

```text
PortableTransformCompilation/1 =
  representable{events:[SourceTransformPortableEvent/3...],afterSourceSha256}
| unavailable{reason:
    provenance_gap|unsupported_transaction|invalid_utf8_boundary|
    generated_cross_anchor_edit|boundary_slot_unrepresentable|
    provenance_cycle|after_replay_mismatch|payload_digest_mismatch}
```

普通save遇unavailable仍可继续，只是不能产生transform evidence。

`SourceTransformPortableEvent/3` 只有 replace与insert。坐标全部是original-before UTF-8 byte half-open；delete是replacementLength=0的replace。generated replacement/insertion内部后续edit折回同provenance event；same-point inserts按真实transaction order合并；真实overlap可形成maximal island；boundary insert不能被错误吞进neighbor replacement。same-point canonical order：left replacement ending p → unique insert@p → right replacement starting p。

receiver从before/after pins机械计算每个event在after中的generatedOutputSpan并切片replacement payload，再验证length/hash；禁止内容搜索。replay必须精确得到afterPin与afterSourceSha256。

### 11.2 mapping

非零target `[s,e)`：任何replace与target真实overlap或insert严格落在内部都停止exact mapping。否则一次性使用original-before坐标累计：

```text
s' = s + Σδ(R where b<=s) + ΣL(I where q<=s)
e' = e + Σδ(R where b<=e) + ΣL(I where q<e)
```

point p被replacement `a<=p<b`覆盖则停止；否则：

```text
p' = p + Σδ(R where b<=p) + ΣL(I where q<p)
     + (ΣL(I where q=p) when affinity=right else 0)
```

因此 replacement end==p保持连续、start==p停止；insert@p left保持插入前、right到插入后。multi-transform chain逐段重新验证SourceVersion、signature、history cut与mapping，不跳中间transition。

### 11.3 frozen plan

```text
CoreSourceEditPlan/2 = {
  kind:"d6_core_source_edit_plan",version:2,
  decisionKey,ownerNodeRef,beforeObservation,
  coordinateProfile:"utf8-byte-half-open/1",
  edits:[SourceTransformPortableEvent/3...],
  afterPin:PinRef/2,
  transformEmission:TransformEmissionPlan/1
}

TransformEmissionPlan/1 =
  disabled{reason:"no_exact_core_edit_plan"|"transform_profile_unavailable"}
| required{profile:"d6_source_transform_seal/1",expectedTrustRevision,expectedTrustKeyId}
```

arm由Core在planning前机械决定，caller不可选。winning plan冻结后required不能降级、disabled不能升级；required seal时必须重验exact profile/revision/key和usable handle。seal不得重新compile/reorder/merge events。

## 12. SourceTransform signed evidence/outbox

```text
SourceTransformEvidence/2 = {
  kind:"d6_source_transform_evidence",version:2,
  decisionKey,changeId,ownerNodeRef,
  before:managed SourceVersion/2,
  after:managed SourceVersion/2,
  beforeSourceSha256,afterSourceSha256,
  coordinateProfile:"utf8-byte-half-open/1",
  affinityProfile:"annotation-range-affinity/1",
  edits:[SourceTransformPortableEvent/3...]
}
```

required plan必须：

```text
D3-CJ/3(plan.edits) == D3-CJ/3(evidence.edits)
```

`SourceTransformSealArtifact/1` 以 `D6-Source-Transform-Seal/1 || NUL || D3-CJ/3(body_without_signature)` 做Ed25519签名。outbox key唯一为 `{changeId,ownerNodeRef}`；`SourceTransformSealOutboxItem/1` 的artifactPin=portable_metadata exact canonical artifact bytes。required sealed decision恰一item；disabled零item；同key不同bytes为integrity conflict。publication crash只按exact key重发原pin，不按current trust/source/hash搜索或重签。

SourceTransform artifact不是PortableComponent，不进入Notice/CP component set；它与source decision共用同一P seal/ChangeId。

## 13. D6 trust profile / activation / history

一个Workspace只有一个 `WorkspaceTrustRootDeclaration/1`/anchor。closed seal profile只有：

```text
d6_revision_token_seal/1
d6_source_transform_seal/1
```

historical `WorkspaceTrustDeclaration/1` 永远只授权revision-token。current `WorkspaceTrustDeclaration/2` 使用profile-discriminated签名域并允许两profile；`WorkspaceAuthorizationBundle/2` 可含exact `/1` historical prefix + `/2` suffix，首个 `/2` 之后不得回到 `/1`。predecessor hash总是前一条真实版本canonical bytes。

Declaration1 activation仍从其原 DecisionKey→CP3/ChangeRecord证据派生；Declaration2 activation必须从同DecisionKey committed CP4 + ChangeRecord/1 + policy after-image第一次追加exact declaration sequence派生。同DecisionKey连续declarations共享一个activation ChangeId，在`history_at(C)` all-in/all-out。

`TrustConflictCarry/2.originActivationChangeId` 不能自证：receiver重载原 Declaration2、验证root/predecessor/profile/mode、重新找到original DecisionKey→CP4/ChangeRecord，再比较派生ChangeId。Carry1保持原Declaration1+CP3算法。later resolver cut绝不替代origin cut。

SourceTransform普通receiver先验证producing ChangeRecord/CP4，然后固定 `C=CP4.frontierBefore`，调用historical validation(profile, trustKeyId, producing CommitDomain,C)再验signature。之后ordinary rotate不使旧合法artifact失效；compromise用artifact seal ChangeId与**原 compromise activation ChangeId**做因果判定，不能用arrival/current state/CP4.frontierAfter替代。

`DomainSealKeyHandle/1` 若exact revision profile/domain/trustKey仍在current safe interval且usable，可以继续签新的revision-token，即使bundle已经是v2；永远不能签transform或被静默包装成Handle2。由Declaration2新建/rotate的key使用Handle2。

fresh bootstrap有一个root和恰两个profile declarations：rev1 revision-token，rev2 source-transform，固定profile rank顺序，共一DecisionKey/activation ChangeId，无合法中间prefix；两个staged handles仅同P commit后usable。replica registration同理固定为双profile。continuation对old domain的两个profile分别计算current(K)|none|conflicted/unproved；任一unproved整体不能激活新domain。全部可证后按 revision revoke? → transform revoke? → new revision authorize → new transform authorize 的固定顺序在一个DecisionKey/CP4中完成。

## 14. D10 current/historical mixed holders

D10 current Workspace read使用 `D10WorkspaceReadDependencies/2`（DependencyProof/3）；`ControlDependencies/3`、`D10ControlInput/2`、`ControlPrepareBinding/3`接到D6 Descriptor3/Intent3。

### 14.1 record images

`D10ControlRecordImage/2` **只**表示真实嵌套变化的automation/run：automation含ScheduleSubscription/2；run含versioned AuthorStep responsibilities。planned_approval/external_approval/activation/reservation/external_effect/stop及supplement=none种类继续可用exact Image1，不为了数字整齐升级。

`D10VersionedControlRecordImage/1`、`D10VersionedControlRecordPin/1`、`D10VersionedControlRange/1`显式以schema tag承载V1/V2 exact value。Pin1 payload仍 `D10-Control-Record/1 || NUL || D3-CJ/3(Image1)`；Pin2用 `/2` domain，历史pin不重编码。Range2三臂records/cost_lineage/occurrences；occurrences可装versioned Occurrence1/2，同semantic key跨版本最多一项。

`D10ControlEffectPlan/2` 的before可以是actual Image1或Image2，after是exact current successor；因此 `Image1(Automation+Subscription1) → Image2(Automation+Subscription2)` 是合法current configure正路径，不强迫先做后台migration。recordPins允许old/new exact pin。

### 14.2 prepare binding三代

```text
D10VersionedControlPrepareBinding/1 =
  {schema:"d10_control_prepare_binding/1",value:ControlPrepareBinding/1}
| {schema:"d10_control_prepare_binding/2",value:ControlPrepareBinding/2}
| {schema:"d10_control_prepare_binding/3",value:ControlPrepareBinding/3}
```

historical `/1` exact decoder来自真实历史设计：

```text
{key:StableControlKey/1,canonicalIntentBytes:Bytes,
 allocatedControlRefs:[ControlRef<K>/1],
 originalCommitRequest:PreparedCommitRequest/1,
 immutablePreview:ControlPreview/1,
 dependencyPins:ControlDependencies/1}
```

e8aa明确承诺true saved `/1`记录按原decoder恢复；这不声称所有环境都曾部署它。Claims2 `prepareBindings`统一按inner StableControlKey canonical bytes排序唯一，跨版本同key拒绝；wrapper只标decoder，不赋authority、不后台迁移、不改原pins/dependencies/saved-planned-unknown责任。

### 14.3 complete execution responsibility

`D10MoneyResponsibility/2`、`D10ExecutionClaims/2`、`D10ExecutionInventory/2` 都使用version-dispatched record pins/ranges/preparations/author steps/subscriptions/occurrences/ApprovalUses。ExternalResponsibility/1、StopResponsibility/1、StopCapacity/1本身没有嵌改变类型，保持 `/1`，但Inventory2必须完整携带。

一个合法Inventory2可以同时包含 old ApprovalUse1/PAB3/Subscription1/Occurrence1/Pin1/PrepareBinding1或2，与 new ApprovalUse2/PAB4/Subscription2/Occurrence2/Pin2/PrepareBinding3，只要各semantic identity不同。跨版本同DecisionKey、same StableControlKey、same (Automation,generation)、same OccurrenceKey或same record cut均拒绝，不LWW。

D6 current successor为 `ExecutionResponsibilityRecord/3 + ExecutionContinuityProof/2`，inventory pin exact domain `D6-Execution-Inventory/2`。Record2/Proof1/Inventory1历史domain保持。只有真实responsibility mutation/checkpoint/handoff才从Record2形成Record3；不因runtime升级后台迁移。handoff必须在同一store barrier停止old holder admission/planning/send/schedule writers，完整捕获old+new liabilities并不可逆fence old holder；缺任一old pin/PAB/subscription/stop/external unknown就pause/unavailable，不能构造部分inventory。普通source读写不因此全局阻塞。

## 15. Annotation suggestion生命周期

Suggestion与reviewState分离；唯一生命周期 authority 是 §9.5 的 `Suggestion/3.state` 与 `confirmation`，不得再定义第二个 SuggestionState 类型。

reject只需先取得annotation state disclosure + annotation_read/write；不需要读取target source，也不建立blind-write profile。它以同一 Annotation current-revision CAS 将 `Suggestion/3.state` 从 `pending` 改为 `rejected`，并令 `confirmation=not_applicable`；其它已授权修改按 Value/4 规则处理。accept必须取得annotation和target disclosure、fresh current target qualification、current expected bytes/point与新的D7 prepare；mapped geometry只是fresh qualification输入，不复活旧 PreparedIntent/ActionEvidence。

显式reanchor保持 `state=pending`，将 `confirmation` 改为 `needs_reconfirmation`；必须在fresh current exact target重新确认expected bytes/point、更新target basis后才回 `confirmed`。accept在**一个D6 seal**同时写Document after与 Annotation 的 `Suggestion/3.state=accepted, confirmation=not_applicable`；accept/reject race只有一个current Annotation revision winner。geometry可映射但expected bytes变化仍不能accept。accepted/rejected均为terminal，不提供reopen。

## 16. Copy/import/export/backup Annotation

Node copy产生fresh AnnotationRefs，完整rewrite reply graph并通过真实source/resource identityMap重发targets；creator/authoredAt/lastEditor/editedAt作为attribution保留，但不能复制旧locator或按文本猜position。copy owner/target mismatch、mandatory reply/target无法重建时整个typed copy失败；raw byte copy不冒充typed success。

import将可证明的managed identity/source/resource target映射到fresh refs；无法证明owner/provenance的target保留为需要用户处理的candidate/unavailable，不自动fuzzy写入。export/Review Bundle按当前权限决定是否包含source excerpt/history/mediaregion；HTML/PDF输出不能因为renderer convenience升级target resolution。portable backup包含Annotation JSON和其portable identity/value；消费型execution authority不随普通文件copy传播。

## 17. Error、recovery与版本边界

共同顺序保持：closed decode → minimum disclosure/capability → authority/domain/backend/trust → stable key与saved/planned/unseen → current target/source/control → dependency/semantic/budget → one planning CAS → install → one P seal → output authorization。

saved结果先按原owner/version重放；planned恢复原request/descriptor/proof/prepared/pins/Notice/install/version basis，不能就地升级；unknown保留原Approval/Money/claim/external-effect/stop责任。只有unseen current使用Descriptor3/PAB4/Edit3/ExportPlan3/Notice3/CP4/Declaration2等。存在decoder或历史设计文字不等于某prototype曾部署；只有实际可证明存在的record承担其历史decoder义务。

缺SourceTransform artifact、Index行、provider或新strong proof不能被永久 `owner_update_required` 当成通用禁用。协调已落库的current正向路径应按其具体 `proof_unavailable/state_unavailable/dependency_conflict` 等closed error恢复；raw/source/Draft/repair等不依赖缺失strong authority的路径继续按原资格。

## 18. Schema version清单

Current新增/继任：

```text
ManagedDocumentFormatProfile/1
ManagedDocumentFormatBinding/1
DocumentFormatCurrentQualification/1
ManagedDocumentSemanticQualification/1
OracleSemanticWitness/7
CoreSemanticProjection/1
DependencyKey/3
DependencyProof/3
InputDescriptor/3
PreparedIntent/3
PortableComponentKey/2
InstallationNotice/3
ContentCompletionProof/4
ChangeRecord/1
D3IdentityInput/13
D3ResolutionInputUse/2
PreparedActionBinding/4
EffectManifest/3
EffectBytes/3
PreparedEditBinding/3
ExportPlan/3
PublicationReceipt/3
PortableTransformCompilation/1
SourceTransformPortableEvent/3
CoreSourceEditPlan/2
TransformEmissionPlan/1
SourceTransformEvidence/2
SourceTransformSealArtifact/1
SourceTransformSealOutboxItem/1
WorkspaceTrustDeclaration/2
WorkspaceAuthorizationBundle/2
DomainSealKeyHandle/2
WorkspaceTrustGenesis/2
WorkspaceBootstrapPlan/4
D10WorkspaceReadDependencies/2
ControlDependencies/3
D10ControlInput/2
ControlPrepareBinding/3
D10ControlRecordImage/2
D10ControlRecordPin/2
D10ControlRange/2
D10ControlEffectPlan/2
ApprovalUse/2
D10AuthorStepResponsibility/2
ScheduleRecurrenceEvidence/2
ScheduleSubscription/2
ScheduleOccurrenceProof/2
AutomationOccurrenceRecord/2
D10MoneyResponsibility/2
D10ExecutionClaims/2
D10ExecutionInventory/2
ExecutionResponsibilityRecord/3
ExecutionContinuityProof/2
```

明确保持的关键类型包括：`PinRef/2`、`ComponentImage/1`、`OwnerInputBinding/2`、`ObservationScope/2`、`D3DecisionCompanion/2`、D4/D5 inner selector/locator types、`WorkspaceTrustRootDeclaration/1`、`WorkspaceTrustAnchor/1`、`RevisionTokenSealVerificationKey/1`、`LeaseRunUse/1`、`D10ExternalResponsibility/1`、`D10StopResponsibility/1`、`StopCapacity/1`。

历史版本按其真实decoder保留，不被current closed union原地扩臂。

## 19. 验收与证据边界

逐项设计义务见 `ACCEPTANCE.zh-CN.md` / `ACCEPTANCE.md`：437条core design oracles +117条actual-owner coordination fixtures，共554条。它们是**未运行的设计验收义务**，不是实现测试结果。

本PR没有安装或运行 Ruby oracle、Asciidork、Mermaid CLI、浏览器/Puppeteer、STEM/PDF providers，也没有运行产品SQLite/replica/crash/crypto/Automation handoff测试。作者自检只能证明文档/JSON/路由的一致性；独立接受必须绑定本PR停止写入后的exact head SHA。