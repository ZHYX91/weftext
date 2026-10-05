---
source_language: zh-CN
translation_status: source
---

[English](SPEC.md)

# Weftext AsciiDoc / Annotation 最终设计候选：联合 owner replacement

状态：**candidate-design-not-implemented**。本规范绑定父输入 e8aa0b341630a57c786c0891d4bbd1620247441d，只允许作为其上的协调设计候选；不表示本 PR、A2、全局设计、实现或发布已接受。

## 0. 优先级、范围与不可分拆理由

本文件与 [SCHEMAS.zh-CN.md](SCHEMAS.zh-CN.md) 共同组成 replacements.json 所列 actual-owner 章节的完整 normative replacement：本文件冻结行为/算法/owner边界，SCHEMAS冻结closed current shapes、排序、cross-field与历史分派；本文件中的摘要措辞不得扩张或缩减SCHEMAS。未被 replacement router 点名的 fixed-e8aa owner正文保持原义。S=7e18168dad3e6d120fce0dd607dc10fa7894e252 的49个 snapshots 与 docs/design/inputs.json 不修改。

本候选把已经独立核销的核心 v3.10 与 FC-3.4→a→b→c→d 合成为公开、可独立审查的 current proposal。旧候选名、私有聊天、作者自评都不是协议依赖。

原本考虑拆成“AsciiDoc/format”和“Annotation/SourceTransform/trust”两批；固定 e8aa 对账后不能安全拆分：current DependencyProof/3 → PreparedActionBinding/4 → D10 ApprovalUse/2 与 EffectManifest/3 / EffectBytes/3 同时承载 managed-format currentness、D10 author recovery 与 WorkspaceBootstrapPlan/4。先发布一个临时 /3 闭集、第二批再给同版本加 bootstrap/transform arm 会破坏 closed decoder。因此本 PR 是一个联合设计 PR，但正文按 D2–D10 owner 分章。

设计原则：一套 author authority、一套 D6 planning CAS、一套 P seal；不新增第二 source truth、第二 portable ledger、第二 decision 或自由 adapter gate。Derived Index 永远可删，不能恢复 current authority。

## 1. D2：完整 AsciiDoc 语言合同

### 1.1 固定语言基线与生产实现边界

Weftext Managed Document 的 AsciiDoc core-language 语义固定为：

```text
asciidoctor-ruby/2.0.26
commit 0b99b39c9df884d4aec13bba45f03cdbab505769
```

Ruby Asciidoctor 只用于独立 conformance oracle，不进入生产依赖。生产实现必须为 Rust-native；成熟实现优先，Asciidork 79671b57923841e41fc91d2c1d2fce18cca3131c 只是候选实现，不是兼容范围上限。实现成本、富文本控件缺失或 provider 缺失都不能把合法 core-language 缩成安全子集。

完整 core contract 包括固定2.0.26的 block/inline/list/table/description/callout/checklist、attribute list、document attributes、includes、substitutions、macros、passthrough、STEM、media、footnote/indexterm/catalog、doctype/backend model、document title/author/revision、block title/reftext、native TOC/section numbering、leveloffset、diagnostics以及处理环境对可观察语义的影响。合法但当前没有结构化编辑器控件的 construct必须可解析、读取、Source编辑、保存和无损 round-trip。

标准 title separator保持原生2.0.26语义：默认 : ，显式 [separator=::] 与 :title-separator: :: 按原语义工作。**新建 Weftext 模板是否默认 :: 仍未决，本规范不选择。**

### 1.2 ProcessorEnvironment

Core Gate 的环境必须冻结而不是读取 ambient host状态。AsciiDocProcessorEnvironment/3 至少闭合：doctype、backend语义profile、safe mode、base/input/output dir、attribute hard/soft set/unset四态及顺序、user-home、locale/encoding、时间/日期/epoch输入、include resolver、网络/文件读取授权、extension registry、provider profile和所有会改变固定2.0.26 observable semantics的输入。source authored attribute events与host/builtin/include provenance分开；wf-kind/wf-facets 等 Weftext authored control只能消费 root-authored provenance，不能由 host注入或 included source反向接管 root Node classification。

### 1.3 标准标题、深层标题与 run-in

标准 AsciiDoc section状态机保持原义，包括 discrete/floating title、book part/chapter、skip warning、fragment relaxation、negative leveloffset clamp和 doctitle条件。Weftext Managed profile只增加：；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

- authored section level 6–9；
- leveloffset 继续是标准语义，effective level不因 authored 6–9而硬截断，可超过9；
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

Ruby observer是测试旁路，不是产品 parser、authority或持久状态；不得替换 selected backend、插 marker/sentinel、改 evaluator string/encoding、额外调用 content/text/title/reftext/xreftext，也不得为了补证据再次运行 parser/regex。实际 converter返回值继续原样参与后续 substitutions。

### 2.2 OracleSemanticWitness/7

Witness/7保留 Witness/6 的 document/block/collection/catalog/diagnostic/observedStrings/calls/operations/inline/content evidence，并增加：；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

```text
modelPropertyObservations:[OracleModelPropertyObservation/1...]
```

model观察只旁路复制实际 producer已经写入的 primitive field/attribute/关系，不额外求值。Section三态、caption/numeral、最终Table列宽、final Cell对象/style/alignment/span、ListItem checklist/coids、真实author named/positional/rekey、临时converter写均必须从固定producer取得，不由projector重跑Ruby model算法。

### 2.3 snapshot/reference/cut

modelPropertyObservations[].observationId 在本数组内唯一、严格随append递增。snapshot identity就是其 observationId；不存在第二snapshot ID namespace。每个subject形成 immediate-predecessor单链：首snapshot previous=null，其后必须恰指同subject直接前一snapshot。

bind/cut snapshot引用必须存在、同subject且是相应时点的 latest snapshot。cut heads按数值subjectId排序，每个live semantic subject恰一head，并满足：

```text
set(cut.heads.subjectId) == reachableSemanticClosure(cut)
```

forward discovery只沿真实header/child/dlist/table-column/final-cell/cell-inner-document等semantic关系；parent/cell_column只做反向一致性，不把stale对象重新拉入。treeprocessor返回的新Document成为真正root；旧Cell被 reinitialize 替换后不得同时live。

model_ready 恰在实际 top-level Document#parse 完成（包括restore attrs/treeprocessors及实际returned Document）后产生。成功selected-backend evaluation恰一个 evaluation_complete；异常退出不得伪造complete cut。post-ready真实semantic write进入新head；temporary物理write不能通过挑旧head绕过。

### 2.4 namespace分类与 producer-time bind

只有同属 modelPropertyObservations.observationId 的 previous/bind/cut snapshot和 model-slot observation引用使用数值先后比较。operationId、inlineEventId、valueId/observedStringId、callId 各按自己既有namespace的存在/类型/DAG规则；ModelCarrierReference.entry 是对应carrier数组的0-based index，绝不与model observationId比较。不存在global event ordinal。

bind-time chronology不能由最终wire静态证明。受控observer producer必须在真实callback中满足：目标carrier已经append，entry < targetStream.lengthAtBind，且producer仍持有exact同一Ruby object；随后才append bind。document/0同理，实际returned Document carrier必须先发布。最终decoder只能验证最终stream/index/type与object-binding closed关系，不能声称单靠最终数组证明跨stream先后。后来补齐carrier不能治愈此前非法bind。

### 2.5 catalog 与 temporary state

catalog subject的唯一ownership关系是：

```text
parent -> actual Document#register receiver Document subject
```

inner AsciiDoc cell的inner Document注册catalog时parent必须指inner Document，不指top-level root。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

DocBook root-option load-bearing物理路径按固定源码记录真实行为，例如 authored value → internal set_option temporary value → remove_attr absent/deleted；observer不得伪造restore写。latest physical snapshot与PropertyProfile的author semantic winner是两层：internal cleanup不能抹掉此前合法author provenance，也不能复活真正后来的author overwrite/delete。

## 3. CoreSemanticProjection/1 与 PropertyProfile

### 3.1 投影

CoreSemanticProjection/1 至少含 document、blocks、catalog、indexTerms、finalState、diagnostics。CoreFlow/1 用 final runs + marks表示文本/格式；坐标按 Unicode scalar计数。实际 visible text只拥有一次；hard break、footnote ref等用atom表达，不能由body/inline/break重复复制。

intermediate Ruby execution ID、call count、converter-return节点、临时dependency edge不进入 equality surface；但其对最终target、text、catalog、counter、attribute state的真实影响必须保留。authored passthrough与backend-feedback raw不能猜成普通 formatting mark。

### 3.2 Canonical PropertyProfile

属性按 A/P/F/I provenance分类：authored named、authored positional、fixed-derived semantic、internal。positional经过实际rekey只留下一个canonical slot；真正任意author named属性即便renderer未使用也保留。内部cloaked-context、cache、reader、temporary root-option等不能to_s偷渡。

公共语义至少包括 id/style/ordered roles/options/caption/numeral/explicit subs/未消费positional和 named/<name>。quote/verse attribution+citetitle、source listing language/linenums、section sectname/special/numbered、table/column/cell width/alignment/span/style、list marker/checklist/coids、media参数及mark/atom的实际语言字段按固定2.0.26 producer映射。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

显式作者 foo-option=bar 同时保留：

```text
options += "foo"
named/foo-option = "bar"
```

显式空值保留空字符串；%foo / options=foo / opts=foo 只保留membership，不虚构author named空值。真实writer顺序决定winner。

### 3.3 Canonical arrays/diagnostics

语言有序序列保持顺序；set排序唯一；multiset按D3-CJ/3(item) UTF-8 bytes排序并保留multiplicity；unique-key map冲突拒绝而非LWW。roles/columns/children/menu path等语义序列不能全局排序。

CoreDiagnostic/1 common location只包含：

```text
null | {logicalFile:text|null,line:UInt|null}
```

没有Ruby producer的column不进入cross-implementation equality；semanticCode使用共同canonical namespace，exactMessage留作Ruby evidence审计。

## 4. Provider profiles：语言有效性与renderer可用性分离

Mermaid、STEM/TeX/AsciiMath、HTML/PDF是版本化生态profile，不是AsciiDoc语法开关。合法core source在provider不可用时仍合法；BackendStatus表达unavailable/denied/incomplete，exact .adoc export仍可成功。

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

node.version必须满足>=22.13.0且同时固定可执行digest；browser/font/config缺任一exact字段时provider为unavailable。CLI 12参数按现行契约使用 size（不再width/height）、pdf-paper-format（不再pdfFit）、themeCSS（不再cssFile）。默认嵌字体不允许被解释成“任意host字体等价”。不得npx在线补包或按浮动semver取得另一个依赖树。

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

BaselineOnly是未绑定managed profile时的ordinary analysis路径，不是portable binding arm。managed current Node缺binding是 incomplete/proof_unavailable，不fallback到BaselineOnly或latest profile。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

fresh managed Node bindingRevision=1。same-Workspace copy/fork fresh identity建立新的binding revision1但保持exact profile generation；formal same-Workspace restore恢复exact historical binding/revision；普通backup bytes import没有身份continuity，重新BaselineOnly analysis→managed delta→显式admission→fresh revision1。真正managed profile migration才checked +1；source unchanged的profile-only migration有一个portable ChangeId但 sourceChanges=[]，不增加SourceVersion/H。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

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

key.ownerNodeRef==binding.ownerNodeRef；componentImage.version==binding.bindingRevision；bindingPin=portable_metadata且exact pin D3-CJ/3(binding)。ManagedDocumentSemanticQualification/1 绑定 current SourceObservation/1 + DocumentFormatCurrentQualification/1；两者任一变化都令旧projection/preparation stale。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

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

document_format={kind:"document_format",workspaceRef,ownerNodeRef}。DependencyProof/3、InputDescriptor/3、PreparedIntent/3 保持旧outer职责但使用Key/3。旧 /2 永远仍是十四臂旧rank，不回填第十五key。

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

document_format={kind:"document_format",ownerNodeRef}；component bytes strict-decode ManagedDocumentFormatBinding/1，ComponentImage/1.version=bindingRevision。PinRef/2、ComponentImage/1保持原shape。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

InstallationNotice/3 与 ContentCompletionProof/4 构成新的 current component family；旧 Notice2/CP3 的原 bytes/decoder 不原地扩臂。

Notice3 相比 Notice2 只把 portable component-key decoder 升为 PortableComponentKey/2。其余合同逐项继承：components 非空、按 fixed rank 与 canonical key 排序且唯一；notice 在第一次 portable-current install 前由原计划冻结并持久化；baseFrontier 仍是原计划基线；notice 不含尚未 seal 的 ChangeId、receipt、credential、approval、Money 或 execution authority。

CP4 相比 CP3 只把 component-key decoder 升为 PortableComponentKey/2，并增加下述 current ChangeRecord/1 关联。CP3 的其余 committed/restored 校验全部继续为规范要求：Notice↔CP key 集合与顺序严格相等、after 必须是实际 installed/sealed image、sourceChanges 完整且按 EntityRef 排序、原 receiptDigest、Frontier 单次前进、scope_dependencies 连续 chain、restored arm 禁止成功字段，以及 receiver 对真实 component bytes 与 owner version 的校验。

fresh managed Document 的 source-document component 与 document_format component 必须由同一原始 plan 产生、由同一 portable decision 安装，并且只在同一个 P seal/CP4 中一起成为 current；不得出现半激活 managed source。format-only transition 若 source 不变，只改变 document_format，sourceChanges=[]，不产生 SourceRevisionPlan/managed SourceVersion，也不推进 H(D,E)。

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

同一 P seal 固定 ChangeId、CP4 和 ChangeRecord bytes；Notice3 已在 install 前由原 plan 持久化。ChangeRecord 只索引 exact Notice/CP 与 frontiers，不复制 components/sourceChanges，不是第二 author truth。publication 失败只重发原 pinned bytes。pre-FC 记录没有真实 decoder 时，strong causal consumer 得到 proof gap，不能套 /1；ordinary source 读写不因此全局禁用。

对 committed CP4：decisionKey.workspaceRef 必须与全部 component/sourceChanges Workspace 一致，changeId.commitDomain 必须等于 decisionKey.commitDomain。CP4.components 与对应 Notice3 的 key 集合严格相等且 canonical 顺序完全一致，每个 after 都是该 key 实际 installed 且 sealed 的 after image。CP4.sourceChanges 要么为空，要么是该 decision 全部真实 source-state change 的完整、按 EntityRef canonical 排序且唯一的集合；source 不变的 portable effect 不制造假 entry。非 absent after 必须来自 winning source plan 与同一个 ChangeId 形成的 managed SourceVersion/2。receiptDigest 只认证该 decision 原始 receipt bytes，不扩大 disclosure 或 execution authority。

frontierBefore 是 seal 前实际验证的 Frontier；frontierAfter 必须恰好在 decisionKey.commitDomain 上由 frontierBefore 增加本 ChangeId，其他 domain 不回退、不被无关改写。exact policy 下 frontierBefore 与原 expected/base/notice Frontier byte-equal。scope_dependencies 下，从 Notice3.baseFrontier 到 frontierBefore 的每个新增 head，以及本 ChangeId 到 frontierAfter 的链，都必须有完整连续、已验证的 ChangeRecord/completion chain；原 frozen dependency proof 还必须证明新增 sealed effects 与全部绑定依赖无关。只比较 vector number、provider sync 状态或当前文件都不够。

restored CP4 只有在未 seal 且 Notice3 的每个 component 都已安全恢复到原 before image 时才合法。其 closed restored arm 只含 decisionKey、baseFrontier 与这些 component images；changeId、成功态 guarantee/writeProtection/semanticState、frontierBefore/frontierAfter、sourceChanges、receiptDigest 与任何成功语义均禁止出现。

receiver 只有在 strict-decode ChangeRecord 所指 exact canonical Notice3/CP4 bytes、核对版本与 cross-fields、取得全部 listed component bytes/metadata、按实际 owner decoder/version 验证每个 ComponentImage、验证完整 production SourceVersion before/after，并证明连续 causal chain 后才可 admission ChangeRecord1。document_format component 还必须 strict-decode exact ManagedDocumentFormatBinding/1。缺 bytes、未知 decoder/version、chain 不完整或 Notice/CP 不一致都只能是 incomplete/proof_unavailable，不能判 success。原 authorization、recovery、error ordering 保持不变，也不新增 ledger、CAS 或 commit point。

## 7. D3/D4/D5 current consumers

D3 current native request升级为 wire13/D3IdentityInput/13，消费 InputDescriptor/3；D3DecisionCompanion/2不变。D3ResolutionInputUse/2 exact持 InputDescriptor/3，历史 /1 仍只接受Descriptor2。

D4 current outer qualification使用Descriptor3/Proof3/15-arm key；真正解释managed Document semantics时必须同时有 source(owner)+document_format(owner)。D4 inner RelationReadContext/2、RelationReadBinding/2、OccurrenceKey、numeric sourceRevision、Calendar/Registry types都不机械升版。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

D5 table/collection/Field current outer同样消费proof3/descriptor3与document_format；format stamp变化即使SourceVersion相同也 stale/reprepare。table/row/cell locator、OccurrenceKey、RevisionTokenBinding、SourceObservation和D5 intent grammar保持原型。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

Trash保留exact managed format binding；restore保留exact historical binding；purge在同一strict plan/Notice3/P/CP4中把document_format从present改absent。历史retained bytes只服务history/recovery，不能自动成为current qualification。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

## 8. D7/D8/D9 current holders

PreparedActionBinding/4 继承PAB3业务成员并直接持 current Descriptor3/Proof3/PreparedIntent3；MinimumMapping/3不因整齐升版。Query/Action/CEL语言本身不因新dependency arm改语法；只有qualification/evidence carrier升级。

EffectManifest/3 / EffectBytes/3 是同一closed current transport，包含既有EffectBytes2 encodings及 d6_workspace_bootstrap_plan4 / d7_symbolic_json3；historical Plan1/Plan3继续旧decoder。D10 preview digest和D3 conflict preview均消费这一current family。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

D8 PreparedEditBinding/3 持 current descriptor/proof。Draft base绑定 SourceObservation+DocumentFormatCurrentQualification；format变化保留dirty Draft bytes/input log，但失效旧projection/map/preview/prepared confirmation，必须reproject/reprepare。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

D9 ExportPlan/3 / PublicationReceipt/3 冻结proof3与current semantic qualification；exact-source-only export不需要无关semantic parse，而rendered/semantic export必须列format dependency。e8aa d9_probe closed format union没有adoc/asciidoc；本设计不偷偷给旧wire加arm。新managed AsciiDoc adoption由D2/D3/D6 admission/profile流程负责。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

## 9. 独立 portable JSON Annotation

Annotation 是 D3 AnnotationRef identity 下的 portable current value，不嵌进 Document source，不建立第二 MessageId。每个 root/reply 都是独立 AnnotationRef；thread 机械等于 same-owner reply_closure，cycle 拒绝。并发 fresh replies 可并存；同一 Annotation 并发 edit 通过 revision/CAS/ConflictRecord，不用 timestamp LWW。

### 9.1 D3-Annotation-Value/4

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

unknown/missing/duplicate member、非法 null、非 UTF-8 均拒绝。labels 最多64项，每项1..128 UTF-8 bytes、exact unique；body source最多65536 bytes。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

purpose invariants：

- root comment：body required/nonempty，suggestion=null，reviewState=open|resolved；
- root mark：appearance required，suggestion=null，reviewState=open|resolved；
- root suggestion：suggestion required，reviewState=open|resolved，body可作为review rationale；
- replyTo!=null：same owner、acyclic、purpose=comment、suggestion=null、reviewState=not_applicable；；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。
- resolve/reopen只改thread root；target resolution与reviewState正交。

### 9.2 D3-Annotation-Target-Projection/1

唯一 identity/locator projection 是 closed union：

```text
document       {kind:"document",owner:NodeRef}
document_element{kind:"document_element",locator:DocumentElementLocator}
document_range {kind:"document_range",locator:DocumentRangeLocator}
resource       {kind:"resource",resourceRef:ResourceRef}
resource_region{kind:"resource_region",locator:ResourceRegionLocator}
```

每个 variant 只允许列出的字段。owner/locator/resourceRef 必须与 AnnotationRef.owner 相同；不允许 AnnotationRef、裸NodeRef、AuthorAnchorAddress或跨owner locator/ref替代target。outer wire与 Projection/1 必须双向唯一materialize；path/title/hash/ambient owner不得补字段。

### 9.3 Annotation 行内正文与唯一求值配置

`AnnotationInlineBody/1`、其兼容 schema alias `AsciiDocInlineBody/1`，以及唯一 `AnnotationInlineProfile/1` 的 exact closed shape 由 [SCHEMAS.zh-CN.md §7](SCHEMAS.zh-CN.md#7-annotation-closed-values) 规范。SPEC 不再维护第二份同名 profile 字面量。

`AnnotationInlineBody/1` 保留四个成员：format、version、languageBaseline、source；它只是可移植正文值，不是处理器配置。求值配置固定 Ruby 2.0.26 的确切提交，并固定 doctype 为 inline、processorBackend 为 `html5-semantic/1`、safe mode 为 secure，同时应用已闭合的资源与效果限制。正文中的短 languageBaseline 字面量只标识可移植 source 数据版本，必须对应求值配置固定的 2.0.26 提交，不能用于选择另一实现版本。

完整 source 必须只形成一个段落（允许软换行与末尾空白）；第二段、标题、列表、定界块、表格或块宏均为 `invalid_annotation_body`，不能被 inline doctype 静默忽略。固定2.0.26允许的行内 strong/emphasis/mono/mark/role、URL link、xref、STEM、footnote 等仍可使用；n1/r1/weftext-cite/carrier/query/view/deep-heading/run-in 这些 managed adapter 在此配置中不激活。

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

### 9.5 Suggestion/3

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

pending 的 confirmation 为 confirmed|needs_reconfirmation；accepted/rejected 必须 not_applicable 且 terminal。targetBasis唯一为 SHA256("Weftext-Suggestion-Target-Basis/1" || NUL || D3-CJ/3(complete stored target))；不含quote/prefix/context/display text。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

- replace：nonempty document_range；expectedText required；replacementSource required（可空）；pointAffinity=null；
- delete：nonempty document_range；expectedText required；replacementSource=null；pointAffinity=null；
- insert：zero-width document_range；expectedText=null；replacementSource nonempty；pointAffinity left|right。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

## 10. Annotation targets/current qualification

### 10.1 source target

source span target绑定真实 owner、basis production SourceVersion、byte range、point affinity、expected bytes/context及source provenance。root-authored span owner=root Node；managed include span owner=included Node。unmanaged/network/synthetic multi-origin不能直接制造durable exact target；可annotate directive或先adopt/reanchor。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

resolution状态必须区分：

```text
exact
mapped
candidate
ambiguous
orphaned
unavailable
```

candidate/fuzzy/context search永不取得正文write authority。重复文本多个候选必须ambiguous，不选first/nearest。

### 10.2 media target

PDF/image region绑定exact ResourceVersion + normalized rectangle；zoom/UI shell变化不改region。audio/video使用rational timebase/ticks与 [start,end)。decoder/profile不可用时已有raw annotation保留，新exact region creation unavailable。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

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

SourceTransformPortableEvent/3 只有 replace 与 insert，坐标全部使用 original-before UTF-8 byte。delete 是 replacementByteLength=0 的 replace。每个 replace 必须满足 0 <= startByte < endByte <= beforeByteLength，且 startByte/endByte 都是 UTF-8 scalar boundary；removedByteLength 必须等于 endByte-startByte，removedSha256 必须等于 beforeBytes[startByte:endByte] 的 SHA-256。每个 insert 必须满足 0 <= atByte <= beforeByteLength 且 atByte 是 UTF-8 scalar boundary；零长度 insert 不是 canonical event，必须省略。generated edit 在可表达时折回原 provenance event；source replacement overlap 必须形成一个 maximal replacement island，严格落在 island 内部的 insert 必须折入该 island，否则 compilation unavailable。same-point inserts 按真实 transaction order 合并，所以一个 point 最多保留一个 insert。

canonical event array 按 original coordinate 排序。interval 在 p 结束的 replacement 位于 p 处 event 之前；同一个起点 p 上，唯一 insert@p 位于从 p 开始的 replacement 之前。因此 boundary order 固定为：left replacement ending p → unique insert at p → right replacement starting p。maximal-island 形成后，各 replacement source interval 两两不重叠。

`generatedOutputSpan(E)` 禁止内容搜索，必须用 exact before/after pin 做一次 replay cursor 计算。初始化 beforeCursor=0、afterCursor=0。依 canonical 顺序处理 E：replace 时 q=E.startByte，insert 时 q=E.atByte；要求 q>=beforeCursor，并要求 beforeBytes[beforeCursor:q] 与 afterBytes[afterCursor:afterCursor+(q-beforeCursor)] byte-equal，然后两个 cursor 都推进这段 unchanged 长度。E 的 generatedOutputSpan 恰为 [afterCursor, afterCursor+E.replacementByteLength)，必须落在 afterBytes 范围内，其 exact slice 同时满足 replacementByteLength 与 replacementSha256，随后 afterCursor 到 span end；replace 再令 beforeCursor=endByte，insert 则保持 beforeCursor=q。最后 beforeBytes[beforeCursor:] 与 afterBytes[afterCursor:] 必须 byte-equal，完整 after bytes 还必须匹配 afterSourceSha256。任何 boundary、length、digest、ordering、unchanged segment 或 final replay 失败都令 compilation/evidence 无效；receiver 不得用冗余字段替代真实 bytes。

### 11.2 mapping

对 replacement R=[a,b)，设 replacement length 为 Lr，定义 delta(R)=Lr-(b-a)，它是可为负数的 signed integer；对 q 处 insertion I，len(I)=replacementByteLength。以下求和必须用 checked signed arithmetic，最终 mapped coordinate 必须仍在 replay 后 after-source byte range 内。

非零 target [s,e)：任何 replace 与 target 真实 overlap 或 insert 严格落在内部都停止 exact mapping。否则一次性使用 original-before 坐标累计：

```text
s' = s + Σδ(R where b<=s) + ΣL(I where q<=s)
e' = e + Σδ(R where b<=e) + ΣL(I where q<e)
```

point p被replacement a<=p<b覆盖则停止；否则：

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

SourceTransformSealSignedBody/1 恰好是完整 SourceTransformSealArtifact/1 删除唯一 signature 成员后的 closed object，因此仍含 format、version、trustKeyId 与完整 SourceTransformEvidence/2，不能增删其它字段。signature 的 lexical form 恰为 86 个 ASCII unpadded-base64url 字符，decode 后必须是 64-byte Ed25519 signature。选中的 verification public key 恰为 43 个 ASCII unpadded-base64url 字符，decode 后必须是 32 bytes；trustKeyId 必须恰为 "sha256:" + lowercase_hex(SHA-256(raw public-key bytes))。

待签消息两语完全相同，固定为 `ASCII "D6-Source-Transform-Seal/1" || NUL || D3-CJ/3(SourceTransformSealSignedBody/1)`。transport/store 的 artifact bytes 必须恰为含 signature 的完整 SourceTransformSealArtifact/1 的 D3-CJ/3；即使其它 JSON serialization 解出相同值，也必须拒绝。signature 不覆盖自身、outbox address 或后来的 trust cut。

outbox key 固定为 {changeId,ownerNodeRef}；artifactPin 保存 portable_metadata 的精确规范 artifact bytes。required 决策密封后恰有一个 item，disabled 则为零；同一 key 出现两份不同 artifact 属于 integrity conflict。
required seal 在原 P seal 中重新验证冻结的 profile/trustRevision/trustKeyId 与 usable handle。
publication/recovery 只重发原 pin bytes，永不重编译 event，也不按当前 source/hash/trust 搜索或重新签名。

SourceTransform artifact不是PortableComponent，不进入Notice/CP component set；它与source decision共用同一P seal/ChangeId。

## 13. D6 trust profile / activation / history

一个 Workspace 恰有一个保留的 WorkspaceTrustRootDeclaration/1 和一个受保护的 WorkspaceTrustAnchor/1。closed seal profile 只有 d6_revision_token_seal/1 与 d6_source_transform_seal/1。历史 WorkspaceTrustDeclaration/1 继续保持 byte-exact revision-token-only。WorkspaceTrustDeclaration/2 是 current profile-discriminated successor；WorkspaceAuthorizationBundle/2 可以包含 byte-exact /1 prefix 再接 /2 suffix，出现第一个 /2 后不得回到 /1。

Declaration2 的 revision 1 predecessor 必须是 `{kind:"root",fingerprint:WorkspaceTrustRootFingerprint/1}`，其中 fingerprint 是完整 retained object，不能降成裸 digest。该 fingerprint 必须与 retained root declaration 重算值以及唯一 protected WorkspaceTrustAnchor/1 中保存的值 byte-equal。后续 predecessor 使用前一 declaration 的 exact revision 与其 D3-CJ/3 bytes 的 SHA-256。rootKeyId 永远不能代替 predecessor fingerprint。

Declaration2 的 publicKey 恰为 43 个 ASCII unpadded-base64url 字符，解码后必须是 32-byte Ed25519 key；所有 Ed25519 signature 成员恰为 86 个 ASCII unpadded-base64url 字符，解码后必须是 64 bytes。
每个 trustKeyId/newTrustKeyId 必须等于 `"sha256:" + lowercase_hex(SHA-256(raw_32_byte_public_key))`。uppercase hex、padded base64、其它 serialization 或 closed prepare protocol 之外的 caller key bytes 一律拒绝。

authorize 与 rotate 的 possessionSignature 必须认证 `ASCII "D6-Domain-Seal-Key-PoP/2" || NUL || D3-CJ/3(DomainSealKeyPoPBody/2)`。
PoP body 取外层 declaration 的 workspaceRef、revision、predecessor 与 decisionKey。
随后加入 action 的 commitDomain/profile 与新 key tuple。
rotate 把 newTrustKeyId 规范化到 body 的 trustKeyId。
TrustConflictOutcome/2 的 authorize_fresh 使用同一 PoP domain/body，并从外层 Declaration2 的 common fields 重建。
它再绑定该 outcome 的 commitDomain/profile/key tuple；PoP 不得搬到另一 declaration revision、DecisionKey、domain、profile 或 key。

ordinary rotate 的 mode 必须是 ordinary；continuitySignature 是被替换旧 key 对 `ASCII "D6-Domain-Seal-Key-Rotate/2" || NUL || D3-CJ/3(DomainSealKeyRotateContinuityBody/2)` 的 Ed25519 signature。
该 closed body 包含 declaration common fields 与完整 rotate action。
除 continuitySignature/rootSignature 外，还必须包含 old/new key IDs、publicKey、possessionSignature 与 mode="ordinary"。
mode 为 loss_recovery 或 compromise 时，continuitySignature 必须使用 literal `"not_required"`，不要求也不接受旧 key signature；revoke 使用独立的 closed mode 集 administrative|loss|compromise。

rootSignature 必须认证 `ASCII "D6-Workspace-Trust-Declaration/2" || NUL || D3-CJ/3(WorkspaceTrustDeclarationSignedBody/2)`；WorkspaceTrustDeclarationSignedBody/2 就是完整 Declaration2 只删除 rootSignature。因此 root 会绑定完整 action、PoP/continuity、conflict outcomes/carries、predecessor 与 DecisionKey。

当前未见过的普通 trust administration 只通过 SCHEMAS §9 的闭合 wireVersion3 add/rotate/revoke prepare successor。profile 使用 SealProfileId/1，因此 revision-token 与 source-transform 两个 profile 都有真实 public producer。
request 不携带 public/private key material。
add/rotate 的 key 与 PoP 由 Core 在 admitted secure store 内生成。
gate 保持当前 workspace policy_admin 与精确 expected trust revision。
还必须保留 current disclosure、anchored root 与 usable root handle。
rotate 的 mode 为 ordinary|loss_recovery|compromise；revoke 独立为 administrative|loss|compromise。
操作只通过原 planning/install/single-P/CP4 路径更新现有 policy/WorkspaceAuthorizationBundle component，不新增 trust ledger、Boolean validator、CAS 或 commit point。
真实 saved/planned wireVersion2 trust request 保留 exact decoder/recovery，绝不改写为 /3。

Declaration1 activation 继续走原 DecisionKey -> CP3/public-history 规则。Declaration2 activation 必须由其 DecisionKey -> committed CP4 + ChangeRecord/1 -> 首次追加该 declaration sequence 的 exact policy after-image 重派生。共享同一 DecisionKey 的 declarations 共用一个 activation ChangeId，在 history_at(C) 中全进或全不进。TrustConflictCarry/2 必须从原 Declaration2 与原 CP4/ChangeRecord 重派生 origin；后来的 resolver cut 不能替代 origin cut。

普通 SourceTransform receiver 验证 producing ChangeRecord/CP4，并用 C=CP4.frontierBefore 做 historical key verification。后续 ordinary rotation 不会让在 C 合法的 artifact 失效。compromise 必须比较 artifact seal ChangeId 与原 compromise activation ChangeId 的 causal order；causal-concurrent 或更晚的 old-key seal 无论 arrival order 都失败。DomainSealKeyHandle/1 只在 exact Declaration1-authorized key 仍 current、safe、usable 时继续 revision-token 新签名；它永远不获得 transform authority，也不改编码成 Handle2。

WorkspaceBootstrapProfile/4 与 Profile3 使用同一闭合成员集合和 issuer semantics，只把 fresh-target trust genesis family 改为 WorkspaceTrustGenesis/2。
WorkspaceBootstrapPlan/4 保留 D3 allocation chain 的 canonical lowercase UUID proposalId 以及其它 UUID 成员。
creator binding 与 target Registry binding 使用 SCHEMAS §9 的闭合 helper type。
series configuration 与 period-scope binding 同样使用闭合 helper type，不留描述性 placeholder。

Plan4 只用于 current unseen fresh create_workspace/fork_workspace bootstrap。原 D3 proposal authenticity、issuer/target-custody ordering、一个 planning CAS、一个 DecisionKey、一个 P seal 全部保持。create 从 frozen eligible seed 派生 target Registry；fork 在 fixed source cut 携带并映射完整 source Registry/configuration history。fresh bootstrap 只有一个 root，并严格按顺序生成两个 Declaration2 authorize：revision 1 revision-token，revision 2 source-transform；两者同 DecisionKey/activation ChangeId，rev2 predecessor hash exact rev1 canonical bytes。两个 staged handle 只有在这一个 seal commit 后才变 usable，不存在可观察的一-profile prefix。

普通 managed copy 不是 bootstrap，继续走既有 scope/configuration mapping 规则。restore/recovery 按实际 saved decoder/plan 分派，不注入 Genesis2，也不升级 old bytes。continue_workspace/failover 保留 current policy、principal mappings、Registry/configuration 与 profile trust state，不能重新跑 bootstrap。本设计候选尚未部署，因此不声称从 Plan3 有 migration 或 dual-write；任何确实存在的历史 record 只按其 recorded contract 继续。

replica registration 继续使用 specialized same-record producer；current FC operation 为 dual-profile，generic trust prepare 不能冒充 replica_register authority。continuation 对 revision 与 transform profile 分别计算 current(K)|none|conflicted_or_unproved；任一 profile unproved 都禁止新 signing domain 部分激活。其余情况下完整顺序固定为 optional old revision revoke、optional old transform revoke、new revision authorize、new transform authorize，全部在一个 DecisionKey/CP4 中且无可观察中间 prefix。

## 14. D10 current/historical mixed holders

### 14.1 current owner 路由与历史分派

对于当前尚未建立决议的 D10 Workspace control 操作，协调后的链固定为 D10WorkspaceReadDependencies/2 与 DependencyProof/3 → ControlDependencies/3 → D10ControlInput/2 → ControlPrepareBinding/3。新的自动 author step 使用 PreparedActionBinding/4 与 EffectManifest/3、EffectBytes/3 构造 ApprovalUse/2。新的交互 author step 使用其 owner 合同选定的 PreparedActionBinding/4 或 PreparedEditBinding/3。新的调度记录使用 ScheduleSubscription/2 与 AutomationOccurrenceRecord/2。

这只接管 replacements.json 所列 fixed-parent D10 CANDIDATE、CONTROL-CONTRACT、IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE、TERMINOLOGY 与 UPSTREAM-AMENDMENTS 中的 fresh/current 语句，不对版本名做全局替换。真实已保存、已规划或状态未知的记录，继续按实际写入时的精确 decoder 与恢复合同处理。若 recorded format 是历史 PreparedActionBinding/3、EffectManifest/2、EffectBytes/2、PreparedEditBinding/2、ApprovalUse/1、ScheduleSubscription/1、AutomationOccurrenceRecord/1、ControlPrepareBinding/1 或 /2，则其原 bytes、pins、canonical request、OperationId 与恢复证据都保持不变。current mixed holder 只能通过显式 versioned carrier 引用这些旧责任，不能迁移或重新 pin。

fixed-parent D6 direct consumer 采用同一分流。当前 unseen D10 control 使用 D10ControlInput/2、ControlDependencies/3 与 D10ControlEffectPlan/2，不再走 fixed-parent 的 /1-/2 current producer 语句；当前 unseen 自动 author use 消费 ApprovalUse/2 与 PAB4/Effect3。saved/planned 历史关联必须先按原版本恢复，不能仅因存在 current successor 就改写。

### 14.2 image、pin、range 与 effect plan

D10ControlRecordImage/2 只用于 automation 和 run，因为只有这两类的嵌套 current schema 发生了变化。planned_approval、external_approval、activation、reservation、external_effect、stop 以及 supplement=none 的记录继续使用精确 Image1。Image2 不得降级为 Image1；旧 Image1 也不能为了进入 mixed holder 而重新编码。

Pin1 继续认证 fixed-parent D10-Control-Record/1 payload。Pin2 认证 D10-Control-Record/2 前缀、一个 NUL 与 D3-CJ/3(Image2) 的精确组合；payload kind 为 artifact，retention 保持原 recovery 或 approval_money，byteLength 与 SHA-256 必须对应完整前缀 payload。Pin2 永远不能重新 pin Image1。

mixed record-pin 数组先取 carrier.value.image，再依次按 binding.ref 的 canonical bytes、binding.revision 数值、usageRevision 的 none 先于 some、some 时的 usageRevision 数值、最后按完整 canonical image bytes 排序。同一个 cut 的逻辑身份是 binding.ref、binding.revision 与 usageRevision。若该身份出现两项且 image bytes 相同，属于重复责任并拒绝；若 bytes 不同，则是 integrity_conflict。schema 版本不能成为同一 cut 保留两项的理由。

range kind rank 固定为 records=0、cost_lineage=1、occurrences=2。逻辑 range identity 分别是 scope 加完整已排序 kinds、CostLayerKey、Automation Ref。mixed range 数组先按 kind rank，再按逻辑 identity 的 canonical bytes 排序；同一 logical identity 跨 Range1/Range2 最多一项。occurrences range 的内部记录按 AutomationOccurrenceKey 排序，同一个 occurrence key 不能同时保留 V1 与 V2。

D10ControlEffectPlan/2 的 changes 按 after.value.binding.ref 的完整 canonical Ref 排序且唯一。before 非 none 时，before 与 after 的 binding.ref 必须相等，并且必须有一份与真实存储 before schema 完全匹配的 versioned record pin。Image1 before 配 Pin2 非法。当前 Automation configure 可以合法地从 Image1+Subscription1 变成 Image2+Subscription2。若 D10 scheduling owner 判定为 continue，则普通 configure 可以保留同一 subscription generation；只有 owner 规则要求 replace 时才换 generation。mixed-holder uniqueness 是 snapshot 规则，不要求把所有旧 subscription 一律 replace。

### 14.3 ControlDependencies/3 与单一 cut

ControlDependencies/3 完整保留实际读到的 configBindings、usageBindings、authority proof、authorization generations、stopRefs、可选 Workspace reads、versioned record pins 与 versioned ranges。config binding 按完整 Ref 规范排序；usage binding 按完整 Ref；authorization-generation token 按 canonical token bytes；stop ref 按完整 Ref；record pin 与 range 使用上一节的 mixed 排序。

每个 configBinding 都必须有精确匹配的 record image/pin。每个 usageBinding 都必须有同一 usageRevision 的匹配 image。每个 stopRef 都必须有精确 stop Image1/Pin1，因为 stop 不机械升版。一次 preparation 所使用的 current range fence、binding、usage revision、record image 与受保护 Workspace evidence 必须来自同一个真实 Authority Store barrier。不能把 barrier A 的 V1 value 与 barrier B 的 V2 value 拼成“完整”快照。

被 current responsibility 引用的 historical image 仍是 immutable evidence，不会因为进入 holder 就变成 current configuration。缺少所需旧 bytes、pin 或 decoder 时，在原 disclosure gate 之后返回 state_unavailable；同 cut 的矛盾证据是 integrity_conflict。要求旧 Pin1 时不能用当前 Image2 代替。

### 14.4 Claims、Money 与 Inventory canonicality

D10ExecutionClaims/2 必须完整保存全部 mixed responsibility。recordPins 使用 mixed pin 顺序。prepareBindings 按 inner StableControlKey 的 canonical bytes 排序，而且该 key 在 Binding1、Binding2、Binding3 之间全局唯一。leaseRuns 按完整 Run ControlRef 排序唯一。authorSteps 以 run Ref 加 stepId 为 identity，Step1/Step2 跨版本唯一。subscriptions 以 Automation Ref 加 generation 为 identity，Subscription1/2 跨版本唯一。occurrenceRecords 以 AutomationOccurrenceKey 为 identity。ranges 使用 mixed range 顺序。continuityPins 按 pinToken 排序唯一。wrapper 只选择精确 decoder；不能给 Binding1 增加 kind、version 或 confirmation 字段，不能 LWW，也不能改变原 pins 或 dependencies。

D10MoneyResponsibility/2 的 reservations 按完整 Binding<reservation>/1 canonical key 排序唯一；layers 按 CostLayerKey canonical bytes 排序唯一；recordPins 与 ranges 使用 mixed 规则；evidencePins 按 pinToken 排序唯一。同一个 reservation identity 的矛盾值不能伪装成两份 liability。CostLayerTotal 仍是完整 reservation/attribution 集合的验证投影，不能从当前配置余额反推。

D10ExecutionInventory/2 的 ApprovalUse carrier 按 DecisionKey canonical bytes 跨版本唯一；externalUnknowns 按 binding.ref canonical bytes 排序唯一；stopState 同样按 binding.ref canonical bytes 排序唯一。所有仍被 pending work、recovery、deduplication 或不可逆 stop 引用的责任都必须保留，其中包括已完成但仍被引用的 external attempt。

Inventory2 只有在旧 holder 的 admission、planning、send 与 schedule writer 都在一个真实 store barrier 停止后才能捕获。来自不同 barrier 的 old/new value 不能拼成一个 complete inventory。

### 14.5 Inventory pin、Record3 equality、store incarnation 与 StopCapacity

Inventory2 artifact 的精确 payload 是 UTF8 D6-Execution-Inventory/2、一个 NUL byte，再接 D3-CJ/3(D10ExecutionInventory/2)。对应 PinRef/2 的 payloadKind=artifact、retentionClass=recovery，byteLength 与 SHA-256 都覆盖完整前缀 payload。Inventory1 保留历史 D6-Execution-Inventory/1 domain，绝不重新 pin 或重编码成 Inventory2。

ExecutionResponsibilityRecord/3 与其 pinned Inventory2 必须逐项一致。workspaceRef 相等；五类 responsibility payload——approvalUses、claims、moneyLineage、externalUnknowns、stopState——必须是 byte-equal canonical values。ExecutionContinuityProof/2.inventoryPin 必须选择这份精确 Inventory2。

Inventory2.storeIncarnation 必须与 ExecutionContinuityProof/2.storeIncarnation byte-equal；受保护的 birth、barrier 或 fence token 映射还必须证明这一个实际 store 与同一 capture barrier。本批不引入另一个 caller-provided store-incarnation proof object。

Inventory2.stopCapacity 必须是在同一 barrier 从 authoritative safety store 取得的精确 StopCapacity/1。StopCapacity/1 继续是 fixed-parent 的 issued、reserved 两个 Counter，不作为 Record3 的重复成员。Workspace-only custody move 不能把这组 shared counter 复制到第二个 active store。要么原 authoritative safety store 继续作为可达的 serialized owner，要么完整 store handoff 必须 fence 所有受影响 writer，并保留全部 target/latch reservation 与 capacity。store continuity 或 safety capacity 无法证明时 takeover unavailable；不能接受零值、可见子集或 rebuild 值。

Record2、Proof1、Inventory1 保留其历史 decoder 与 payload domain。Record3 只在真实 responsibility mutation、checkpoint 或 custody handoff 时形成。该 transition 必须 strict-decode 每项 retained historical responsibility，在一个 barrier 捕获完整 mixed Inventory2，只 pin 一次，建立 Proof2，在适用时 checked-increment record revision，并保持 executionDomainId。不得 mint 替代 ControlRef、request、approval、reservation、claim，也不得创建第二 execution ledger。

## 15. Annotation suggestion生命周期

Suggestion与reviewState分离；唯一生命周期 authority 是 §9.5 的 Suggestion/3.state 与 confirmation，不得再定义第二个 SuggestionState 类型。

reject只需先取得annotation state disclosure + annotation_read/write；不需要读取target source，也不建立blind-write profile。它以同一 Annotation current-revision CAS 将 Suggestion/3.state 从 pending 改为 rejected，并令 confirmation=not_applicable；其它已授权修改按 Value/4 规则处理。accept必须取得annotation和target disclosure、fresh current target qualification、current expected bytes/point与新的D7 prepare；mapped geometry只是fresh qualification输入，不复活旧 PreparedIntent/ActionEvidence。

显式reanchor保持 state=pending，将 confirmation 改为 needs_reconfirmation；必须在fresh current exact target重新确认expected bytes/point、更新target basis后才回 confirmed。accept在**一个D6 seal**同时写Document after与 Annotation 的 Suggestion/3.state=accepted, confirmation=not_applicable；accept/reject race只有一个current Annotation revision winner。geometry可映射但expected bytes变化仍不能accept。accepted/rejected均为terminal，不提供reopen。

## 16. Copy/import/export/backup Annotation

Node copy产生fresh AnnotationRefs，完整rewrite reply graph并通过真实source/resource identityMap重发targets；creator/authoredAt/lastEditor/editedAt作为attribution保留，但不能复制旧locator或按文本猜position。copy owner/target mismatch、mandatory reply/target无法重建时整个typed copy失败；raw byte copy不冒充typed success。

import将可证明的managed identity/source/resource target映射到fresh refs；无法证明owner/provenance的target保留为需要用户处理的candidate/unavailable，不自动fuzzy写入。export/Review Bundle按当前权限决定是否包含source excerpt/history/mediaregion；HTML/PDF输出不能因为renderer convenience升级target resolution。portable backup包含Annotation JSON和其portable identity/value；消费型execution authority不随普通文件copy传播。

## 17. Error、recovery与版本边界

共同顺序保持：closed decode → minimum disclosure/capability → authority/domain/backend/trust → stable key与saved/planned/unseen → current target/source/control → dependency/semantic/budget → one planning CAS → install → one P seal → output authorization。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

saved结果先按原owner/version重放；planned恢复原request/descriptor/proof/prepared/pins/Notice/install/version basis，不能就地升级；unknown保留原Approval/Money/claim/external-effect/stop责任。只有unseen current使用Descriptor3/PAB4/Edit3/ExportPlan3/Notice3/CP4/Declaration2等。存在decoder或历史设计文字不等于某prototype曾部署；只有实际可证明存在的record承担其历史decoder义务。

缺SourceTransform artifact、Index行、provider或新strong proof不能被永久 owner_update_required 当成通用禁用。协调已落库的current正向路径应按其具体 proof_unavailable/state_unavailable/dependency_conflict 等closed error恢复；raw/source/Draft/repair等不依赖缺失strong authority的路径继续按原资格。

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

明确保持的关键类型包括：PinRef/2、ComponentImage/1、OwnerInputBinding/2、ObservationScope/2、D3DecisionCompanion/2、D4/D5 inner selector/locator types、WorkspaceTrustRootDeclaration/1、WorkspaceTrustAnchor/1、RevisionTokenSealVerificationKey/1、LeaseRunUse/1、D10ExternalResponsibility/1、D10StopResponsibility/1、StopCapacity/1。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

历史版本按其真实decoder保留，不被current closed union原地扩臂。

## 19. 验收与证据边界

逐项设计义务见 ACCEPTANCE.zh-CN.md / ACCEPTANCE.md：437条core design oracles +117条actual-owner coordination fixtures，共554条。它们是**未运行的设计验收义务**，不是实现测试结果。

本PR没有安装或运行 Ruby oracle、Asciidork、Mermaid CLI、浏览器/Puppeteer、STEM/PDF providers，也没有运行产品SQLite/replica/crash/crypto/Automation handoff测试。作者自检只能证明文档/JSON/路由的一致性；独立接受必须绑定本PR停止写入后的exact head SHA。