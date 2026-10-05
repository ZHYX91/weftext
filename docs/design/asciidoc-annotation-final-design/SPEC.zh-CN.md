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

Core Gate 的环境必须冻结而不是读取 ambient host 状态。AsciiDocProcessorEnvironment/3 的 canonical bytes 明确包含 doctype、backend semantic profile、safe mode、standalone、base/input/output identity、API attribute event 序列、user-home、UTF-8 source encoding、locale profile、时间/日期/epoch输入、include resolver、网络/文件读取授权、extension registry、provider profiles，以及其它会改变固定 2.0.26 observable semantics 的已登记输入。source authored attribute events 与 host/builtin/include provenance 分开；wf-kind/wf-facets 等 Weftext authored control 只能消费 root-authored provenance，不能由 host 注入或 included source 反向接管 root Node classification。

attributeOverrides 是固定 Ruby 2.0.26 接收 options[:attributes] 时的有序 API event 序列，不是按名字排序的集合。每个输入 pair 按固定源码的 ! / @ 语法先归一化为 lowercase name 加 hard_set、soft_set、hard_unset、soft_unset 四态；数组顺序就是 API Hash 的枚举顺序，D3-CJ/3 必须原样编码该顺序。相同 lowercase name 可以再次出现：第一次出现决定 ordered-map 的 key 位置，后续同名 event 只替换该 key 的状态而不移动位置；不得排序、去重或按最终值折叠。

这项顺序本身可观察。embedded/standalone=false 时，固定 2.0.26 在 notitle 与 showtitle 同时存在时检查 ordered attr_overrides 中二者最后出现的 key，并从该 key 派生另一个 alias。因此 API 顺序 notitle="" 后 showtitle="" 与 showtitle="" 后 notitle="" 不能得到同一 ProcessorEnvironment canonical bytes，也不能被 oracle/cache 当作语义等价环境。

本候选的 managed/source Gate 只接收 UTF-8 source bytes，sourceEncoding 因而冻结为 UTF-8；其它编码必须先经过显式 import/transcode，而不是由 oracle 隐式猜测。localeProfile 是 oracle harness 的显式受控 locale 描述，不允许回退到未记录 host locale。固定 2.0.26 当前并不会按 lang 自动加载 locale attribute 文件；lang 仍是普通 document attribute。providerProfiles 只列本次 evaluation 实际允许/选择的 provider profile，未使用 provider 时可为空；provider availability 仍与 core-language validity 分离。

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

### 2.3 snapshot、bind 与 cut 基础不变量

modelPropertyObservations[].observationId 只在本数组 namespace 内唯一，并严格随真实 append 递增。snapshot identity 就是该 snapshot 的 observationId；不存在第二 snapshot ID namespace。对每个 subject，按 observationId 取全部 snapshot 后必须形成 immediate-predecessor 单链：第一个 previousSnapshotObservationId=null，之后每一个都恰好指向同 subject 的直接前一个 snapshot。

定义 latestSnapshotBefore(subjectId,o) 为同 subject 且 observationId<o 的 snapshot 中 observationId 最大者。每个 bind 的 snapshotObservationId 必须存在、同 subject、严格早于 bind，并且恰等于 latestSnapshotBefore(subjectId,bind.observationId)。每个 cut head 同理必须指向 latestSnapshotBefore(subjectId,cut.observationId)。heads 按 subjectId 数值严格升序且 subjectId 唯一。

### 2.4 exact semantic head closure

对每个 cut C，先在 C 之前的合法 bind 中选择 carrier.stream="document" 且 carrier.entry=0 的最新一项 rootBind(C)。它必须绑定 document subject，并且 C.documentSubjectId 必须等于 rootBind(C).subjectId。treeprocessor 若真实返回新 Document，M37-14 发布的新 document/0 carrier 与 bind 决定新 root；旧 root 只保留为历史 evidence，不因曾经是 root 自动进入 cut。

structural closure 从 C.documentSubjectId 开始，以每个 subject 在 C 时刻的 latest snapshot 做 worklist traversal。ForwardSemanticRole/1 的闭集恰为 header、child、dlist_term、dlist_description、table_column、table_head_cell、table_body_cell、table_foot_cell、cell_inner_document；只有这些 relation 可以新增 subject。每个 target 必须存在且属于 semantic-head kind。parent 与 cell_column 是 backedge，只检查一致性：forward traversal 完成后其 target 必须已经在 structural closure 中，绝不能靠 backedge 扩张 closure 或把 stale Cell/Document 拉回。

supplementary subject 只按两个闭合规则加入。inline subject 必须在 cut 前有合法 inlineObservations carrier bind，且其 latest snapshot 恰有 parent 指向已经 live 的 structural subject。catalog_record 必须在 cut 前有合法 catalogEvents carrier bind，且 latest snapshot 恰有一个用于 catalog ownership 的 parent，指向 structural closure 中真实执行 Document#register 的 Document subject；不存在 owner relation，也不能把 inner Document 的 record 改挂 top-level root。attribute_buffer 与 table_parser_context 永远是 supporting-only subject，不能仅因存在 snapshot 成为 head。

令 R 为 structural closure 加上上述 live inline/catalog supplementary subjects，令 H=set(C.heads[].subjectId)。有效 cut 必须严格满足：

```text
H == R
```

并且每个 head 都指向该 subject 在 cut 时刻的 latest physical snapshot。这里的 physical membership/latest 与最终 CSP survival 是两层：temporary/internal snapshot、supporting evidence 或最终 physical absent 都不能让 verifier 回退选择旧 head；PropertyProfile/CSP 另按 A/P/F/I provenance 决定 author-semantic winner。

### 2.5 supporting evidence closure

从每个 cut head 的 snapshot、rootBind(C)、以及 C 前 subjectId 属于 R 的全部合法 bind 建立 typed worklist。snapshot 递归跟随 previousSnapshotObservationId、fields/attributes 中每个 ModelPropertyProvenance.inputs，以及 ModelObservedValue 内所有 string/array/map 子值。ModelPropertyInput/1 五个 arm 的边固定为：model_slot → 指定 model snapshot 与 slot；observed_value → OracleObservedString；observed_slice → 指定 OracleObservedString 并验证 byte half-open range；operation → OracleStringOperation；inline_field → OracleInlineObservation 并验证 fieldPath。

OracleStringOperation 继续到自己的 callId、所有 input/removed slice、outputValueId，以及每个 run：exact_copy 跟随其 slice；derived 跟随全部 slices 与 referenced operationId；generated 在 inlineEventId 非 null 时跟随该 Inline observation。operation graph 必须满足既有 DAG 合同。OracleEvaluationCall 只在 call namespace 中递归 parentCallId。OracleInlineObservation 递归 callId、parentCallId、producingOperationId、nodeType/fields 中所有 OracleFieldValue，以及 returnValueId；OracleFieldValue 的 observed_string、array、entries 逐层递归。OracleContentObservation 递归 callId、resultValueIds 与 contributingInlineEventIds。OracleObservedSlice 最终解析到其 valueId 对应的 OracleObservedString 并验证边界；ObservedString 是叶子。

每个 bind 还必须静态解析 ModelCarrierReference：document 只允许 entry=0；其它 stream 的 entry 是对应 blockEvents、collectionEvents、catalogEvents、inlineObservations、contentObservations 或 calls 数组的 0-based index，并验证目标类型。carrier 若落到 Inline、Content 或 Call，继续按上一段递归。Block/Collection/Catalog/document carrier 继续执行各自 retained closed shape/ordinal 规则，但不发明跨数组 event 顺序。

support closure 可以包含旧 intermediate snapshot、attribute_buffer、table_parser_context、旧 Cell 或 temporary writer evidence；它们不会因此进入 H。任何 dangling、wrong-kind、invalid slice/index、非法 operation/call DAG、缺失 provenance 输入都会使 Witness evidence invalid/incomplete，而不是把合法 AsciiDoc 降级为 unsupported，也不得通过再次调用 parser/getter 补证据。

### 2.6 namespace 分类、producer-time bind 与 cut cardinality

只有 modelPropertyObservations.observationId namespace 内的 previous/bind/cut snapshot 与 model_slot observation 引用使用数值 backward/latest 比较。operationId、inlineEventId、valueId/observedStringId、callId 分别只在自己的 namespace 按存在、类型和 retained DAG/reference 规则验证；subjectId 是对象身份；carrier.entry 是数组 index。禁止任何 operationId < modelObservationId、inlineEventId < modelObservationId 等跨 namespace 数值时序推断，也不新增 global ordinal。

bind-time chronology 不能由最终 wire 静态证明。完整 Witness validity 同时要求 static decoder valid 与 producer-conformance valid。受控 observer 在真实 bind callback 中必须已经发布目标 carrier，满足 entry < targetStream.lengthAtBind，并仍持有 carrier 对应的 exact 同一 Ruby object，然后才 append bind；document/0 同样要求实际 returned Document carrier 先发布。最终数组后来补齐 entry 永远不能洗白 earlier invalid bind，也禁止按 text/title/source/path/hash 事后重新寻找对象。

一个完整 top-level Witness/7 恰有一个 model_ready cut。若 selected-backend evaluation 未发生或异常退出，则没有有效 evaluation_complete；若实际 evaluation 正常返回，则恰有一个 evaluation_complete，且它晚于 model_ready 并使用同一 top-level documentSubjectId。nested inner Document 不创建第二套 top-level cut namespace。

### 2.7 catalog 与 temporary state

catalog ownership 只有 parent -> actual Document#register receiver Document subject；该 parent 只用于 supplementary live 判定与反向一致性，不会把不在 structural closure 的 stale Document 拉入 R。

temporary overlay 必须在合法 evaluation_complete 前按固定源码真实路径由实际 restore write 或实际 cleanup delete/closure 结束；observer 不得伪造 restore。DocBook root-option 的 load-bearing 物理路径例如 authored value → internal set_option temporary value → remove_attr absent/deleted，evaluation_complete head 必须是 cleanup 后 latest physical snapshot。PropertyProfile 仍可由 provenance 保留此前合法 authored winner；若之后存在真正 authored overwrite/delete，temporary cleanup 不能复活更旧值。

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

### 3.4 当前 D2 产品 projection

CoreSemanticProjection/1 继续只作为 test oracle 比较数据，绝不能成为产品 read/edit/query wire。current managed AsciiDoc 产品读取只使用 D2DocumentSnapshot/3 与 SCHEMAS §4.4 的 closed product family。exact source 始终是唯一 author authority；product projection、editor map、index、render tree、Query value 都只是可丢弃 derived state。

产品 family 的完整性按 **fixed Ruby 可观察 semantics** 判定，而不只是按 AST 外形。每个 block/inline arm 都携带该 kind 的 dedicated fixed-derived facts、允许合法可变 named native attribute 的闭合 D2NativeAttributeSet/1，以及承载不能由 named map 还原的 public block slot（style/caption/numeral/subs/positional value）的 D2BlockCommonSemantics/1。这样既禁止 free JSON escape，也不会因为 Weftext 没枚举属性名或 positional semantics 就缩小合法 AsciiDoc。

至少必须完整保存以下 fixed-derived semantics，且不限于这些举例：

- section 的 sectname/special/numbered/numeral/caption，以及 authored/effective heading level 分离；
- source/listing 的 language、linenums、start、indent、tabsize、highlight、line-comment；
- quote/verse 的 attribution 与 citetitle；
- admonition 的 name/textlabel/icon，list 的 style/start/reversed/checklist/interactive/coids，table 的 format/grid/frame/stripes/physical grid position，以及 TOC levels；；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
- quoted inline 的 id/roles，xref 的 refid/path，ref/bibref anchor 区分，visible/concealed indexterm 与 see/see-also，footnote ref/xref state、exact callout guard，以及 inline image/icon 区分；；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。
- image/audio/video 的 target-specific dimensions、timing、poster、preload、playlist/list、theme/lang、controls 等 fixed-2.0.26 converter-observable options。；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。

这些 facts 只由同一 fixed native parser/model 解析一次，再投影进一个 product family。D7/D8/D9 不能为了找回 source language、quote credit、section kind、caption/numeral/subs、icon/bibliography-anchor kind、index semantics 或 media option 而重新 parse exact source。；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。

每个 semantic value 还引用 D2SourceOriginGraph/2。graph 必须区分 authored range、authored reference site、substitution、generated value、synthetic value 与 multi-origin result；只有全部 retained provenance path 最终收敛到同一个 exact authored range 时，才导出 single writable source。flat range list 不足以表达这层关系。generated/ambiguous/multi-origin 值的 structured edit 必须 fail closed，但 authorized exact Source read/save 独立继续可用。

document-level product 同样按 fixed Ruby 2.0.26 的实际 public/converter observable state 闭合。D2DocumentMetadata/3 保存 backend/basebackend/filetype/outfilesuffix、safeMode、doctype、完整 doctitle/main/subtitle、revision，以及 Document#authors 的 name/firstname/middlename/lastname/initials/email；initials 不从 fullname 猜。这里完整保留最终文档状态，不允许任何二次推断。它还保存最终完整 Document#attributes present map，而不是把 physical header sourceRange 列表冒充 effective state。builtin/generated/environment-derived attribute 通过 generated/substitution origin 表示，绝不能伪造 authored physical range；headerAttributes 只保留真实 lexical header entry。该 domain 对齐固定 0b99b39c9df884d4aec13bba45f03cdbab505769 的 Document 与 converter consumption；D9/D8 直接消费 product，不从 full name、raw Source、ambient backend 或 host defaults 重建。

final attribute map 还必须保留 fixed Ruby value 的真实**类型**，而不只是字符串表面值。D2EffectiveDocumentAttribute/1 复用 closed D2NativeSemanticValue/1：Hash key absent 就没有 entry；present nil/Boolean/String/Integer 分别映射 null/Boolean/text/canonical integer；Array 只有全部元素都是 String 时才成为保持原顺序的 textArray。因此 safe-mode-level、max-include-depth、authorcount 保持 integer，mannames 保持 HTML5 join 与 DocBook map 实际消费的 ordered Array。ProcessorEnvironment API override 已经只允许 text set-value 或 null unset action。accepted extension 也不能获得 arbitrary-object escape：final Symbol/Float/Hash/nested 或 mixed Array/opaque object 必须使 rich product unavailable，除非未来显式 versioned product domain 真正接纳。Core 禁止 to_s/join/JSON flatten，也不能从 attribute name 或 source 猜回类型；D7/D8/D9 取得的是 typed value 与同一次 evaluation 的 origins。

字段级 provenance 由 SCHEMAS §4.4 的 closed carrier 机械实现，而不是靠 node-level sourceOrigins 的说明性约定：每个 D2NativeAttributeEntry/1 自带 sourceOrigins；common/per-kind semantic 成员有逐字段 origins；block/inline 独立 slot 有 slotOrigins，sequence slot 与 value array 一一同序。structured edit 只能使用被编辑 slot 自己的 writableSource，并在 final barrier 重验该 exact SourceOwner 的 write authorization/current Observation 与 evaluation dependencies。unique authored 正例不能因实现偷懒而一律 readonly；generated、multi-origin、ambiguous、non-author 或没有该 SourceOwner write 权限的值仍可读但不能 structured-write。[source,ruby] 的 style、language、body 与 [#foo.red] 的 id、role、child content 因此有不同可机械关联的 origin；consumer 禁止从 coarse range、path、text 或第二次 parse 反推。

root/include source unit 必须 version-exact。current managed D2DocumentSnapshot/3 只能来自 owner 与 snapshot owner 相同的 managed_file processor input，root SourceUnitBinding/SourceObservation 必须与这个 exact input 相等。managed include 保留 evaluation 实际读取的 SourceObservation/1；artifact/network input 保留 exact immutable pin。D2DocumentSnapshot/3 冻结 exact AsciiDocProcessorEnvironment/3、canonical digest、root/include SourceUnitBinding、ObservationScope 和同一个 authorized read barrier 上的 complete DependencyProof/3。proof 必须包含每个 managed source unit，以及 projection 实际消费的 Registry/foreign/authorization dependency。include Node SourceObservation、artifact/network pin、processor environment 或真实 used dependency 任一变化，都让旧 semantic tree stale，即使 root source 与 document_format 没变化。；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。

因此 snapshot pin 只回答“当时生成了哪棵 tree”，evaluation binding 才回答“这棵 tree 现在是否仍 current”。D7/D8/D9 在自己的 final read barrier 消费 projection 前必须验证完整 evaluation binding。later barrier 只有沿既有 scope_dependencies continuity proof，证明所有 bound source/environment/control/negative dependency 未变时才合法。无关且未被 closed dependency set 消费的 global failure 不得阻塞该 consumer。

D2Heading/3 保持 authoredLevel/effectiveLevel 分离。WeftextManaged 允许 authored 6–9；fixed Ruby leveloffset state machine 只有 lower clamp，因此 effectiveLevel 使用 arbitrary-precision nonnegative canonical integer，没有 Counter/int64 language ceiling。D7 headings 通过既有 TypeSpec {kind:"integer"} 与 canonical decimal value 暴露它。处理巨大合法 level 时资源预算可以 budget_exceeded，但不能 syntax invalid 或 numeric overflow。普通 1–9 heading 继续走同一 Query/outline/render 正向路径。

native link/xref/image/citation grammar 必须先由唯一 AsciiDoc parser 解析。只有 native occurrence 已存在且 Weftext target 独立验证通过后才应用 D2IdentityAdapter/1。adapter 可附 stable NodeRef、owner-local ResourceRef 或 citation identity，但不能从 path/title/text/hash 推 identity。managed include 保留自己的 versioned SourceOwner/Origin facts；include 不改变 root Node identity，也不会给 including Document 对 included source 的写权限。

旧 Profile-v2 对合法 open/include/pass/extension 的禁令只保留 historical implementation input。current fixed-baseline legality 由上述完整 parser/product contract 决定。若 rich editor 没有某合法 construct 的 structural control，该 control unavailable，但该 construct 仍可 read，并可通过 exact Source 无损保存；不能改判 syntax invalid 或 flatten。

invalid source 继续保留 authorized exact source 与 ordered diagnostics，product projection unavailable，D2 commit eligibility reject。physical decode/source-envelope failure 仍是 D6 error，不得伪装成 D2 invalid syntax；repair 与 Source surface 按原 authorization 独立存在。；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。


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

D3 current native request 使用 wire13 / D3IdentityInput/13 + InputDescriptor/3。expectedAuthority 继续是真正 inherited optional member：replica_local create_node/move_node/reorder_node/trash 必须完全省略 expectedAuthority/workspaceProposal/preparationBinding；managed_atomic 严格按 fixed-parent mode matrix 使用 create/continue/existing。JSON null 不能代替 absence，禁止的 member 直接 invalid。D3DecisionCompanion/2 不升级；D3ResolutionInputUse/2 保存 exact InputDescriptor3，历史 /1 只保存 Descriptor2。；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。

D4 current outer qualification使用Descriptor3/Proof3/15-arm key；真正解释managed Document semantics时必须同时有 source(owner)+document_format(owner)。D4 inner RelationReadContext/2、RelationReadBinding/2、OccurrenceKey、numeric sourceRevision、Calendar/Registry types都不机械升版。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

D5 table/collection/Field current outer同样消费proof3/descriptor3与document_format；format stamp变化即使SourceVersion相同也 stale/reprepare。table/row/cell locator、OccurrenceKey、RevisionTokenBinding、SourceObservation和D5 intent grammar保持原型。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

Trash保留exact managed format binding；restore保留exact historical binding；purge在同一strict plan/Notice3/P/CP4中把document_format从present改absent。历史retained bytes只服务history/recovery，不能自动成为current qualification。；其中英文名称均为协议标识、字段名或固定字面量，不改变本句中文语义。

## 8. D7/D8/D9 current holders

PreparedActionBinding/4 直接持 current Descriptor3/Proof3/PreparedIntent3；MinimumMapping/3 不为整齐而升版。Query/Action/CEL 的 author syntax 保持不变，升级的是 qualification/evidence 以及 source adapter 所消费的产品 projection。EffectManifest/3 与 EffectBytes/3 继续是唯一 current transport family；historical Plan1/Plan3 均保持原 decoder。

### 8.1 D7 product-projection consumers

D7 outer runtime 保持 wireVersion2，QuerySpec/ViewSpec author schema 保持 version1。current headings scan 必须 strict-decode D2DocumentSnapshot/3，并在 final D7 read barrier 验证完整 evaluation binding；projection available 后，输出 owner/title 与 level，其中 level 使用既有 D7 arbitrary-precision TypeSpec {kind:"integer"} 与 canonical decimal value。内部 position 仍是 exact current DocumentElementLocator。old saved result 若 schema 记录 level:int64，只属于 historical result data；current product scan 不得为了 int64 截断、饱和或拒绝 fixed Ruby 合法的大 effective level。；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。

existing body_text source 在同一 evaluation-currentness gate 后递归消费 D2DocumentBody/3。它从完整 D2ProductInline/3 family 取得 semantic text，保持 source/list/table order，并使用 product semantic profiles，而不是重 parse exact source。合法 arm 若 body_text 没有 mapping，该 adapter unavailable；不得缩小 D2 syntax 或用 to_s 静默 flatten。

D7 definition-transfer 保留完整 definitionTransfers/Result9 semantics。fresh current D3 submission 使用 D3IdentityOperationRequest/13 及其 exact mode matrix；真正 saved/planned wire12 request、effects、Locators 与 recovery 继续原 decoder。；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。

### 8.2 D8 document、Draft、visual presentation 与 run-in policy

D8 outer document entry 仍为 wireVersion2。current d8_document.snapshot 是 root observation 相同的 D2DocumentSnapshot/3，并携带 exact product evaluation binding/origin graph。D8 必须在 projection/map 作为 current 之前验证完整 root/include/environment dependency set；managed include 变化即使 root bytes 不变也必须失效旧 tree。D2 invalid source 继续既有 exact Draft/repair 行为，不暴露 partial projection。

Workspace run-in default 是 Portable Workspace Metadata 中的 D8-owned shared configuration；它不是 Document source、Policy/3 authorization、Registry data、PortableComponentKey 或 device-local preference。current immutable record/head-set contract 在 SCHEMAS §6.4。单 head 时 D8WorkspacePresentationPolicy/1 仍是唯一 logical current value；/2 record 只是其 immutable portable history carrier，不是第二 value authority。head-set 同时携带 owner-specific current observation stamp。普通 mutation/conflict resolution 都要求 policy_admin、exact observed head-set/stamp、一个 D6 planning CAS 与一个 P decision；同一 portable decision 分配 ordinary ChangeId/ChangeRecord 并提交恰一个 presentation_policy_change owner effect，不新增 ledger/CAS 或 Policy/Registry revision。两个 offline successor 即使 numeric revision 相同，只要 canonical record bytes/ChangeId 不同就保留两个 heads；禁止 revision-number winner、arrival order 与 LWW。多个 heads 只让真正依赖 default 的 presentation unavailable，直到授权的 explicit multi-parent successor 解决。sync/admission 必须验证 retained ChangeRecord/receipt/effect association 与 ancestry 后才能把 head 当 current。saved/planned/unknown 恢复 frozen head-set stamp，绝不重新采样 branch。；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。

SCHEMAS §6.4 把真实 mutation producer 固定到原 D6 阶段。SetRequest 先按 closed decode→presentation-state disclosure→policy_admin→frontier/domain→head-set→exact expectedHeads→retained parent pin/ancestry→semantic/budget 验证；change 路径返回 D8PresentationPolicyPrepareResult/2，内含同一 operation 的 D8PresentationPolicyInput/1、Proposal/1、parent headEvidence、PreparedIntent/3、d6_commit_request/2 与 preview EffectManifest/3。该 PreparedIntent 使用现有 control_only Workspace observation scope、saveProfile=control_only、sourceInputs=[] 与 managed_atomic/strict，只绑定实际 authorization/control dependencies 和 parent record pins；它不读取/安装 author source，不创建 SourceRevisionPlan 或 author-source mutation；唯一 ChangeId 是只在 final P 分配的 ordinary portable decision ChangeId。planning CAS 只冻结 Proposal/Input/head stamp/pins/dependencies；因为 committed record 必含尚未分配的 activationChangeId，prepare/planning/staging/Notice/install/verify 都禁止预造 committed record、record hash、portable pin、outbox 或 ChangeId。final P 重验 authorization 与 frozen heads 后才 checked-allocate C，并在同一 P transaction 由 frozen Proposal+C 构造/编码 D8WorkspacePresentationPolicy/2、hash/pin/address、committed owner effect、ChangeRecord/receipt association 与 protected D8PresentationPolicyOutboxItem/1。P 后 publication/crash retry 只能重放该 pin 的 exact bytes；receiver 必须验证 record/pin、activation ChangeId、effect/receipt/ChangeRecord 与完整 ancestry。equal hash、trusted sender、boolean verified 或 provider latest 都不是 admission proof。相同单 head/value 是零计划零P的 no_change；multi-head 即使值相同也必须显式 resolution；saved/planned/unknown 只恢复 original frozen bytes/pins/request，不重新采样。这条链只处理共享展示配置，不读取作者正文，也不把控制元数据当成正文内容。

fresh unseen create_workspace/fork_workspace 不等待以后再调用 SetRequest。WorkspaceBootstrapPlan/4 必须携 D8PresentationPolicyBootstrapInit/1，其中 before 是由 target custody 证明的 protected empty state，proposal 固定为 parents=[]、revision=1、defaultPresentation=separate。它使用原 create/fork authority、OperationId、planning CAS、DecisionKey 与 final P，不要求尚未 active target 的 policy_admin，也不增加第二 control transaction。P 前只冻结 init/proposal 与 committed=null owner effect；同一个 bootstrap P ChangeId 才原子物化 canonical /2 record/hash/pin/address、committed presentation_policy_change、原 receipt/ChangeRecord association、outbox 与 one-head transition，并与 Workspace activation 同时提交。任何 loser/abort 都不能留下 active Workspace 或 partial policy state。成功 activation 的 Plan4 target 因而立即可用 one-head default。proved [] 仍是明确真实的 initialization state；missing/corrupt/unproved policy evidence 必须 unavailable，绝不能 silent separate fallback。

presentation **条件消费** Workspace record。显式 .run-in、显式 .separate、role_conflict→Separate、no-eligible-body 都不读取或绑定 Workspace policy。两个 role 都没有时，只有满足 implicit-default physical-adjacency 的 eligible body 才消费一个 current policy head。因此 policy missing/corrupt/conflicted 不会阻断显式 separate/run-in 文档，但 Use Default 真正需要 default 时必须 unavailable。

Enable 删除 separate 并确保 run-in；Disable 删除 run-in 并确保 separate；Use Default 删除两者。它们都是普通 source-role edit，不修改 Workspace policy。heading 与 first paragraph 始终是独立 D2 node/source range；RunIn 只是 presentation，绝不重新 parse 成 author source。

D8DocumentRenderBinding/1 冻结 exact D2 snapshot pin 与实际使用的 D8PresentationDecision/1。cache invalidation 只跟真实 dependency consumption：Workspace policy 改变会使旧 workspace_default decision stale；explicit/no-body/conflict-fallback 不得凭空获得一个从未读取的 policy dependency。

### 8.3 D9 semantic/rendered export 与 exact preparation

D9 current export 使用 SCHEMAS §6.5 ExportPlan/3。exact AsciiDoc source、exact Resource 与 query_json 强制 generationPolicy=none，并且没有 template/route/document-render binding；它们不依赖 generation-policy registry、renderer、provider 或 run-in policy，所以无关 provider/configuration failure 不能阻断这些 exact path。；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。

rendered document target 冻结 exact D2DocumentSnapshot/3 及其 evaluation binding、ManagedDocumentSemanticQualification/1、实际消费的 exact D8PresentationDecision/1 与 accepted route/profile chain。HTML 与 D8 使用同一 product/run-in decision。DOCX/ODT 在 selected target profile 支持时可把 effective 1–9 映射到显式 heading style；更大的 arbitrary-precision level 与 target limit 只能成为 explicit loss/unavailability，不能使 D2 syntax invalid。prepare 前必须重新验证 include/environment dependency；Plan 冻结后，later include/source/presentation-policy change 不得 rerender 或修改该 Plan。

generation policy 是有限 **per-plan** closed value：binding choice、missingPolicy=empty、explicit Resource image size、layout choice、native-table token binding。没有 external generation-policy registry/descriptor。missing policy 不能绕 permission/type/unknown-path error；image size/layout 始终按 exact authorized ResourceRef 与 fixed Templates rule。；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。

所有 set-like collection 的 canonical comparator 同样属于 Plan contract：route steps 的 array position 必须等于 step=0..N-1，step.evidencePins 按 pinToken；styleBundles 按 styleBundleId。bindingChoices/missingPolicy 的 templatePath 按 exact Unicode scalar sequence lexicographic 比较，normalization=none、case-sensitive，精确 scalar prefix 较短者在前；禁止 locale/case-folding，两个 array 必须共用这一 comparator 完成跨集合互斥。对于 stagedOutputs/receipt.outputs，fresh-current 名称必须先满足 SCHEMAS §6.5 的 D9ControlledRelativeOutputName/1，完整 bundle 还必须通过确定性的 exact-name、portable alias、文件/目录前缀和系统保留名称冲突检查；随后才按原始 protocol name 的无符号 UTF-8 字节做字典序比较，完全相同的字节前缀较短者在前。这个 comparator 本身仍不执行 normalization、case folding、locale collation、host/path-library ordering 或 separator rewrite。nativeTableBindings 继续以 (setName,columnName) 为 key；已有 imageSizes/layoutChoices/lossChoices/top-level evidencePins/recoveryPins 继续各自既定 key。D9 prepare 必须在冻结 Plan bytes/token/pins/staged manifest/confirmation basis 前先检测 duplicate/conflict 再 canonical sort，使合法 input permutation 得到 byte-for-byte 相同 Plan。冻结、接收或恢复的 current record 若已乱序、名称 duplicate/conflict 或名称不属于闭合 output-name 域，必须失败；读路径不能排序、normalize、rename 或用 LWW 修复 protected bytes。

D9ControlledRelativeOutputName/1 把 fixed-parent 的“controlled relative name”要求闭合为 fresh current ExportPlan/3 的唯一输出名称协议。最终 output name 由 Core 决定，worker 不选择文件系统路径。external-bundle 的 dataFiles 名称、每个 ExportPlan/3 stagedOutputs.name、每个 PublicationReceipt/3 outputs.name、bundle manifest 中的 reportFile 名称，以及 server-download 交付所消费的名称都使用同一协议域。根级系统成员固定为 loss-report.json 与 manifest.json；二者由 Core 生成，必须出现在 stagedOutputs 与 receipt.outputs 中，并在 Plan 冻结前与全部 dataFile 一起执行同一 alias/prefix 冲突检查。普通 dataFile 可以使用 assets/图/附件.png 这类嵌套名称，但不得占用或 alias 根级系统名称，也不得把它们当目录前缀。Resource handoff 消费所选 dataFile 的原始已验证 bytes/name；独立的 resourceName 仍是 D3 author intent，不能通过重命名 export output 推导。

协议名称保持原字节。Unicode normalization 与 case folding 只用于派生 SCHEMAS 定义的拒绝用 portable alias key，绝不改写 stored name、manifest、pin、digest、confirmation、receipt 或 published bytes。选定 destination 的文件系统或 storage API 若不能无损创建某个协议合法名称，可以额外判 destination unavailable；这只是 host capability 检查，不是协议 authority。它不能让协议非法名称变合法、改变 alias 关系、normalize/rename 已冻结名称、覆盖已有目标或把失败 bundle 降级成 partial copy。名称安全、alias、prefix、reserved 冲突属于结构安全失败，不能由 ExportLossChoice 接受；它们也不能反向把合法 AsciiDoc/Resource/Query source 判 invalid，或改变稳定 source identity。

current prepare 必须把完整候选 dataFile 名称集合与固定 report/manifest 名称一起验证，再进行 output 排序、token 分配、pin、staging 或 confirmation。inspect/confirmation/publication/receipt 必须保持这些原始名称。saved/planned/unknown current work 恢复原 names、bytes、pins、OperationId/token 与 installation/publication evidence；unknown publication 不得换名重试。真正 historical ExportPlan/1-/2 与 PublicationReceipt/1-/2 继续使用记录时的 decoder、name、ordering、bytes、pins 与 recovery，即使某个旧名称按 D9ControlledRelativeOutputName/1 会失败，也不得读时 migration、resort、repin 或 re-encode。

template binding inputIndex 必须选择 exact template catalog item，pin/profile decoder 必须 byte-equal/accepted。route steps 冻结真实 provider/version/profile transition，terminal profile 必须匹配 target。Plan3 只使用 protected d9_export_plan/3 token；PublicationReceipt3 使用 d9_publication/3。historical Plan/Receipt1-/2 保留原 tag/bytes/pins/recovery；confirmation/inspect/unknown-publication lookup 必须先按 token tag，再按 record version strict dispatch。；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。

Plan evidencePins 只有一个 derivation：input catalog、document render binding、template、route、styles、dependency/observation proof、staged outputs 中递归可达的 typed PinRef，加 explicit retained recoveryPins，再按 pinToken 排序去重。漏 reachable pin 或加入 unrelated evidence 都改变/破坏 Plan。PublicationReceipt3 必须重复 exact target/template/route/styles/generation-policy/presentation selection 与实际 output digest。；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。

native table→Office dataset name 完全按 SCHEMAS §6.5.1，不给 ordinary Node 增加 export metadata。Core 从 D2TableBlock/3/D2TableCell/3 导出 physical column，包括 multi-row head 与 colspan/rowspan。text comparison 固定 exact Unicode scalar sequence、normalization=none、case-sensitive、保留 whitespace。resolver 依次尝试 leaf、最短更长 header suffix、exact table title、确定性 table/column occurrence ordinal。例如唯一 lowercase-ASCII leaf `amount` 可直接用 `data.native_table.amount`；Plan/Actual 下都有 Amount 时，bare Amount 必须 ambiguous，只能选 Plan/Amount 或 Actual/Amount 的 qualified selector/token；两张同标题表 full path 也相同则必须再加 frozen occurrence qualifier。0 candidate 为 mapping_required；最终仍多 candidate 为 ambiguous_binding；禁止 first/last、猜 suffix 或改 author source。简单唯一 lowercase-ASCII leaf 保留 short COLUMN token；qualified/CJK/RTL/combining selector 使用 SCHEMAS 固定 ASCII digest token。后来新增同名 table/column 可以让 fresh short binding 变 ambiguous；already prepared Plan 因已冻结原 projection/selector/staged bytes 而保持 immutable。；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。

原 D9 permission/state machine 继续 load-bearing：potential scope/authorization 在敏感读取前；inspect/confirmation/publish/delivery 重验 current authorization；unreadable 不能变 none；不同 Query authorization generation 不混用；create-only external publication 保留 durability/unknown-outcome；Save-as-Resource 是独立 current D3 create_resource preparation/receipt。fresh current D3 request 服从修正后的 optional expectedAuthority mode matrix。；本句保留的英文仅表示固定协议标识、字段名、状态名或字面量，均按上述中文条件解释，不形成另一套规范含义。


### 8.4 fixed-parent direct-holder dispatch 与历史边界

replacement router 必须显式接管 D2 implementation impact、D7 Definition Transfer、D9 Import IR、D9 Acceptance Matrix 的 current statements。D2 历史 Profile-v2 implementation prohibition 保持 immutable snapshot evidence，但 current implementation obligation 使用 D2DocumentSnapshot/3 与上述完整 native-baseline product family。D9 Import IR 保留 ImportIR/1、Mapping/1、ConversionInput/2 及所有 file safety/mapping/loss 规则；只有 current outer author submission 改为 D3IdentityOperationRequest/13，真实 saved/planned wire12 import decision 仍精确恢复。D9 Acceptance S12 只在旧“五级 heading”前提上被 supersede：WeftextManaged authored 6–9 为合法，必须贯穿 D2/D7/D8/D9；target 不支持某深度时走显式 loss/degradation，不能拒绝 source。

禁止全局字符串替换去升级 historical wire12、document_snapshot wire2 或 ExportPlan/1-/2 record。saved/planned/unknown recovery 必须先于 current producer gate，并保持原 bytes、permissions、errors、confirmation 与 publication responsibility。

Stage4A 当时明确把 Annotation mutation/alias 留给独立后续批；current candidate 已在随后 Stage4B 的 §§9–16 与 SCHEMAS §§6–7 完成 Value/4/D8/D7/alias chain。本次 A-residual repair 不重写该链；直接共享合同触碰只包括 generic wire13 expectedAuthority optionality 修正，以及 current EffectItem/3 新增 typed presentation_policy_change owner effect，后续都应与本 fixed head 一起做直接增量非作者复核。fixed19f Annotation 的三个独立残余 finding 与 physical-JSON aggregate source gap 仍明确留在本批范围外。

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

### 9.6 Portable Metadata authority、canonical bytes、revision 与 CAS

当前 Annotation 的唯一可移植作者权威，是现有 Node 内 `weftext.annotations.json` 的 Portable Metadata 条目。SCHEMAS §7 的当前逻辑记录是 `PortableAnnotationRecord/4`：持久 `annotationRef`、不透明 `annotationRevisionToken` 与完整 `D3-Annotation-Value/4`。D2 历史 Annotation-v2 外层 snapshot 不再是当前作者值，只保留历史解码和投影。它的 plain-text 正文、`replace_plain_text` 建议以及 resolved/stale 两态 targetStatus 都不能约束当前 Value/4。

current annotation payload 的唯一规范 bytes 为：

```text
annotationValueBytes = D3-CJ/3(complete D3-Annotation-Value/4)
annotationValueSha256 =
  "sha256:" + lowercase_hex(SHA-256(annotationValueBytes))
```

current `payloadBindings` 中 payloadKind=`annotation_value` 的条目、Annotation 的 D6 `SourceRevisionPlan/1.afterPin`、D8 proposed-value pin、current EffectBytes 的 `d3_annotation_value4` encoding，以及 current materialized postimage，都必须指向同一组 canonical bytes。Portable Metadata envelope、AnnotationRef 与 revision token 不进入 `annotationValueSha256`。相同 digest 不能证明 identity、currentness 或 CAS。

`annotationRevisionToken` 继续是 nonempty opaque JSON string，不参与 content identity。已有 Annotation 在 planning 前必须同时匹配 exact current token 与 exact current SourceObservation。`AnnotationEditableProposal/1` 与 current/Draft 的 `AnnotationEditableValue/1` projection 是两个不同的 closed shape，绝不直接互相比字节。完成 closed request decode 及继承的授权/currentness 检查后，Core 先执行 §9.10 operation-class gate，用 caller proposal + complete current before 展开唯一完整 candidate `D3-Annotation-Value/4`，此时四个 before attribution 字段保持逐字节不变；随后只比较 D3-CJ/3(candidate Value/4) 与 D3-CJ/3(current before Value/4)。只有二者相等才是真正 value no-op：完整 current Suggestion evidence/state、attribution、annotationRevisionToken、SourceVersion 与 H 全部保留，不建立 SourceRevisionPlan/source_change，也不得隐式 reconfirm 或刷新 lifecycle evidence。只有 canonical Value/4 bytes 真正不同时，Core 才写入 fresh lastEditor/editedAt、构造 final complete Value/4、为该 Annotation state 分配从未使用过的新 token，并冻结唯一 SourceRevisionPlan/plan；target/reply slot address、structural evidence、source change、receipt 与 materialization 全部使用同一个 final token。显式 reconfirm 成功时本身就是 lifecycle state transition，不能被折成 no-op。即使之后 Value/4 bytes 回到旧值，也必须使用 fresh token，禁止 ABA 复用。

可由作者修改的 value fields 恰为 purpose、target、replyTo、body、appearance、labels、reviewState、suggestion。任何其中字段的实际 committed change 都是 Value/4 mutation，必须推进 Annotation revision。四个 attribution fields 不是 caller mutation input。interactive/current creation 时，trusted Core 从实际执行 principal 与可信时钟生成 creator/authoredAt，并令 lastEditor=creator、editedAt=authoredAt。later actual mutation 必须 byte-preserve creator/authoredAt，并由 Core 根据真实执行 principal/time 重写 lastEditor/editedAt。caller 不能提交 trusted/verified、人类来源、自报 actor/time 等字段来取得真实性；closed request shape 必须拒绝这些字段。

actor/time 仅作显示与历史归因，不参与 authentication、permission、排序或 LWW。trusted same-Workspace copy 可以保留既有四个 attribution snapshots；trusted cross-Workspace transfer 保留其 originWorkspaceRef 作为历史来源。ordinary untrusted import 若要保留 display/time 文本，importer 必须把对应 snapshots materialize 为 `imported_unverified`，不能伪装成 `workspace_authenticated_origin` 或 `prepare_server_clock`。此后本地真实 mutation 继续 byte-preserve imported creator/authoredAt，并用当前实际执行者/时间重写 lastEditor/editedAt。

### 9.7 Value/4 的 D3 mutation binding、slots、receipt 与 lifecycle

当前 wire13 Annotation 修改继续复用固定 D3 mutation algebra，但基础解码器改为当前 Value/4；历史 wire9–12 继续按真实 Value/3 字节解码。两个逻辑 slot 不变：

```text
annotation_target  slotOrdinal=0  reference
annotation_reply   slotOrdinal=1  nonnull时是reference；
                                  existing identity reply变化时只走structural S
```

span 必须从实际 D3-CJ/3(Value/4) member value 计算，因此 Value/4 新字段可以改变 byte offset，却不能改变 logical ordinal。purpose、body、appearance、labels、reviewState、suggestion、creator、authoredAt、lastEditor、editedAt 对 D3 identity mutation 都属于 nonreference bytes。AnnotationInlineBody/1 内的 inline AsciiDoc 继续使用既有 R6 profile，不产生隐藏 D3 slots；target identity 与 reply structure 只由上面两个 slot 表达。

current existing-Annotation result 因而必须有一对完整 Value/4 preimage/result payload。target@0 始终保留完整 reference plan/evidence。nonnull 且未改变的 reply 使用普通 reference evidence；同一 existing Annotation 的 P→Q、P→null、null→Q 恰使用一个 annotation_reply structural plan/S segment 加一个 `annotation_reply_change`，reply slot 禁止再出现 reference result。fresh 或 mapped fresh Annotation 的初始 nonnull reply 仍由 reference plan/result slot 表达，绝不能作为 S container。任何 current Value/4 result，包括只改 reviewState 或 labels，都按 retained payload-binding rule hash 完整 Value/4 base 或完整 Symbolic Result bytes；不能因为 target/reply 未改就漏掉 nonreference mutation。

固定 D3 mode-admission 矩阵继续承担约束。`copy_resource` 不能分配 fresh Annotation；ordinary import 的仅新 owner 规则不能让新导入 Annotation 成为 existing Annotation 的同 owner fresh reply；copy_node_subtree、fork 与 partial identity-bearing import 都不能因为 Value4 successor 而新增 existing-Annotation 的 S 副作用。通用 subject union 不得扩大这些模式。

若 restore trashed Annotation A 的同时把 reply P→Q，target@0 与 nonnull reply@1 的 prestate 都是 non_live_source，poststate 都是 resolved；所有 A 的 toSource、reply structural change 与 receipt reference 必须使用唯一 final annotationRevisionToken。若旧 reply=null，只有 reply prestate=absent，target 仍是 non_live_source。只要 Value/4 有变化，就不能用 lifecycle-only 替代 typed target preimage 或 reply S。独立 Trash/restore 若 Value/4 byte-equal，不得生成新 Annotation revision；lifecycle 是独立 portable metadata。永久 Annotation purge 继续继承 D3 tombstone/no-reuse closure：它从 current Portable Metadata authority 中移除该 live/trashed object，不制造 replacement Value/4 或 revision token；只有既有 history/backup retention 规则要求的 bytes 可以继续保留，而且同一 AnnotationRef 永远不能复用。

copy/fork/import materialization 只能按既有 identityMap/candidate-map 规则改写 target/reply，然后物化唯一 final Value/4 与唯一 final revision token。receipt 的 source version、target/reply toSource address 与 `annotation_reply_change.toAnnotationRevisionToken` 必须全部指向这一个 final revision。backup/export 只按既有 disclosure 携带 portable identity/value，不携带 current permission、ActionEvidence 或 executable preparation。

### 9.8 D8 current Annotation edit surface

current D8 edit preparation 使用 SCHEMAS §6.3 的 version-3 successor。`D8EditIntent/3` 保留 document arm；current annotation mutation arm 接受 `AnnotationEditableProposal/1` + exact expected current Annotation revision token + targetPolicy，显式 reconfirm arm只带 Annotation target/token。caller 不能提交完整 Value/4 attribution，也不能提交 Suggestion state/confirmation/basis/expectedText/pointAffinity。Core 独立读取完整 current PortableAnnotationRecord/4、SourceObservation 与权限，按 §9.10 operation-class gate 构造唯一完整 proposed Value/4，再 pin exact D3-CJ/3(proposed Value/4)。`PreparedEditBinding/3.intent` 必须恰为 `D8EditIntent/3`；`D8EditInput/3` 是 current OwnerInputBinding descriptor。不得再保留 `<D8 current intent>` 占位，也不存在 caller-provided actor/time/lifecycle evidence。

Annotation edit 必须使用 complete profile 与 strict write protection。它先建立 Annotation disclosure、annotation_read/write 与 exact current Annotation CAS。targetPolicy=preserve 要求 stored target bytes/identity byte-equal，但允许 freshly requalify exact/mapped 状态而不写 stored target；targetPolicy=replace_current 要求在 current target disclosure/qualification 下显式选择合法 same-owner target。candidate/ambiguous/fuzzy location 只能 read-only，直到用户 explicit manual reattach 选定一个 exact target，并以新的 Value/4 mutation prepare。pure synchronization 或 stable locator 新近重新取得资格，只要 Value/4 bytes 不变，就不是 edit，不能刷新 lastEditor/editedAt、不能推进 token，也不能复活旧 PAB/EditBinding/ActionEvidence。

body editor 只有一个 source truth：`AnnotationInlineBody/1.source`。visual mode 只是同一 R6 `AnnotationInlineProfile/1` 的可丢弃 render，不得保存 HTML 或第二套 rich body。local Draft 中 inline source invalid 时，必须保留 exact draft source 与 diagnostics，同时 visual rendering 与 prepare unavailable。只有 annotation_read 没有 annotation_write 的 principal 获得 authorized current value 的 read-only surface，不能因为 render 成功就得到 writable Draft。Portable Metadata corrupt/undecodable 时禁止 partial projection：normal read 走现有 unavailable/integrity boundary；有权限的 repair/backup surface 可以暴露 exact raw portable bytes，但不得补造 Value/4 members。

historical D8 wire1/wire2、PreparedEditBinding/1-/2、Value/3 proposed pins 与真实 saved/planned/unknown requests 都按其原 decoder/pins 恢复。current editor 支持 Value/4 不得把它们转换成 EditBinding3/Value4。

### 9.9 D7 current Annotation creation 与 suggestion actions

当前交互式 Annotation 创建使用专用 `D7CreateAnnotationIntent/2` 分支。调用方只能提交 destinationOwnerRef 与 `AnnotationEditableProposal/1`；proposal 不含 Suggestion terminal/lifecycle/evidence。通过原有目标 owner 状态披露与创建授权后，Core 验证 target/reply 的 owner 约束和 R6 正文，执行 §9.10 interactive_create gate（首版 suggestion 只可 pending，confirmed evidence 只能 fresh target recompute），再按 §9.6 注入 creator/authoredAt/lastEditor/editedAt，随后构造完整当前 D3IdentityOperationRequest/13 mode=create_annotation：沿用既有 fresh primary Annotation subject、Value/4 结果 payload binding、target@0 reference plan 与可选初始 reply@1 reference plan。PAB4 绑定这份精确生成的 request 与 Value/4 pin。调用方不能通过 generic d3_operation 或非交互构造入口自行嵌入 actor/time/lifecycle snapshots 来冒充可信当前交互创建；copy/import/create-member 路径继续使用各自具名 owner preparation 与 attribution 规则。

其它 current new D7 author preparation 继续使用 SCHEMAS §6.1 的 `D7ActionSpec/2` 与 `D7ActionPrepareRequest/3`。fixed-parent 非 suggestion ActionSpec/1 arms 全部 byte-for-byte 继承，只为上述实际创建需求增加 create_annotation arm。旧 `apply_suggestion(annotation:EntityTarget,targetLocator)` 只作 historical。current closed suggestion arms 为：

```text
apply_suggestion  -> accept stored pending Suggestion/3
reject_suggestion -> reject stored pending Suggestion/3
```

两者只携带 AnnotationRef 与 exact expectedAnnotationRevisionToken；caller 不能另传 targetLocator、replacement bytes、expectedText、point affinity、actor/time 或预先算好的 source patch。

apply_suggestion 准备必须重新读取当前 Annotation 与其中保存的 Suggestion/3，要求 pending+confirmed，再重新资格化保存的 target 和真实目标 source，并执行唯一 kind 映射：replace 对应使用 replacementSource 的一个 SourceTransform replace；delete 对应 replacement bytes 长度为0的 replace；insert 对应保存的零宽 point，并使用保存的 pointAffinity 生成一个 SourceTransform insert。replace/delete 必须用重新读取的精确字节验证当前 expectedText；insert 必须验证 point/basis。mapped/candidate geometry 只能作为重新取得精确资格的输入。target SourceOrigin 决定真实可写 owner；AnnotationRef owner 与 reply structure 本身不授予 target 写权限。

accept 建立一个 immutable D6 plan，同时包含 target source after-image 与同一 Annotation 的 Value/4（Suggestion.state=accepted、confirmation=not_applicable，并由 Core 写入 lastEditor/editedAt）。两个 source changes 使用同一 DecisionKey、一个 planning CAS、一个 P seal；只改 target 不改 accepted state，或只改 accepted state 不改 target，都不是合法 success。target Document branch 在可表达时使用既有 CoreSourceEditPlan/2/SourceTransform evidence；旧 PreparedIntent/PAB 不能复活。

reject_suggestion 只要求 Annotation 状态披露、annotation_read/write 与精确当前 Annotation token；它不读取 target source，也不要求 target-read 权限。它只把 Annotation Value/4 改成 rejected/not_applicable，并使用同一 CAS、revision 与 actor-time 规则。accept 与 reject，或其它正文、review、label、appearance、reply、suggestion 编辑发生竞争时，只能有一个 token 胜者；失败方按 stale/conflict 重新读取并准备。

### 9.10 Suggestion operation-class transition gate

`AnnotationEditableValue/1` 仍是 current/Draft editable projection，但不再直接作为 current caller mutation wire；D7 create 与 D8 edit 使用 SCHEMAS §7.1 的 `AnnotationEditableProposal/1`，其中没有 state/confirmation/basis/expectedText/pointAffinity。存在 current value 时 Core 必须先读取 complete current before，再从真实 entry 机械确定 operation class，并执行下面唯一 before+operation+proposal→after 规则。派生 before class 是闭合的：fresh create 为 `absent_annotation`；已有 Value/4 且 suggestion=null 为 `no_suggestion`；pending Suggestion/3 分为 `pending_confirmed` / `pending_needs_reconfirmation`；accepted/rejected 分为 `terminal_accepted` / `terminal_rejected`。fresh absence 绝不能和已有 no-suggestion Value 混同；不存在 generic trusted flag、第二 decision 或 UI-only gate。

| producer / operation | 允许的 Suggestion class before→after | 必须的 Core 行为 |
|---|---|---|
| D7 create_annotation / interactive_create | absent_annotation→no_suggestion（合法 root comment/root mark/reply）；或 absent_annotation→pending（合法 root suggestion） | caller 只交 Proposal。comment/mark/reply 创建继续满足既有 purpose/reply/body/appearance/review cross-field，产生 suggestion=null。suggestion 创建只能从 caller 取得 kind/replacementSource；Core 对实际 selected target 做 fresh qualification，只有自己读取/验证真实 target 并重算 targetBasis/expectedText/point 后才可产生 confirmed，否则只能 pending+needs_reconfirmation。首版 accepted/rejected 仍不可能。 |
| D8 annotation+preserve / ordinary_edit | no_suggestion→no_suggestion；no_suggestion→pending+needs_reconfirmation；pending→pending；pending→no_suggestion；terminal→同一 terminal only | body/appearance/labels/review 与满足既有 Value/4 cross-field 的合法 purpose/root↔reply 变化继续可用。已有 no_suggestion 且 target 不变的普通编辑不新增 target-source-content 读取。no_suggestion→pending 只有在当前 Annotation 授权/token 检查通过、真实 stored target 被 fresh qualification 后才合法；Core 自行生成 lifecycle evidence，但本次类别转换强制 needs_reconfirmation，不能制造 confirmed/accepted/rejected，之后必须走显式 reconfirm 才能 confirmed。pending 可通过合法 non-suggestion root/reply proposal 清为 no_suggestion，丢弃 pending evidence 但不声称目标被修改。pending→pending 时，author suggestion 输入逐字不变就完整保留原 confirmed/needs evidence；需要重新目标资格的作者变化强制 needs_reconfirmation。terminal Suggestion/3 必须 byte-equal，不能清空、改 purpose/reply、重开或切成另一 terminal。 |
| D8 annotation+replace_current / manual_reattach | no_suggestion→no_suggestion；pending→pending+needs_reconfirmation；terminal 禁止 | no-suggestion comment/mark/reply 可重新挂到一个 exact 合法同 owner target，after 仍 suggestion=null，不伪造 suggestion evidence。pending reattach 重算 targetBasis 与该 kind 所需 expectedText/point，并强制 needs_reconfirmation；本操作不能同时做 suggestion 类别转换。terminal Suggestion 不得 reattach。 |
| D8 annotation_reconfirm_suggestion / reconfirm_suggestion | pending_needs_reconfirmation→pending_confirmed | 调用方不传 value/evidence。Core 必须重新读取已保存目标与真实目标 source/point，重算全部 targetBasis/expectedText/pointAffinity；任何当前性、权限或字节不符都保持零变更。成功 reconfirm 是真实状态转换，不是 no-op。 |
| D7 apply_suggestion | pending_confirmed→terminal_accepted only | 继续执行 §9.9 的新鲜目标读取、三类建议变换，以及目标 after 与 Annotation after 共同进入同一个 D6 plan、一次 planning CAS 和一个 P；不得只提交其中一侧。 |
| D7 reject_suggestion | pending_confirmed\|pending_needs_reconfirmation→terminal_rejected only | 只要求 Annotation 状态披露、读取/写入权限及 exact token，不读取目标；目标隐藏或不可用时仍允许 reject，并且只改变 Annotation 的终态。 |
| D3 copy/import owner、Trash/restore、recovery | 不把携带的 terminal display state 解释为 fresh apply/reject decision | 按真实 identityMap/归因/历史 decoder 保留既有正向功能；copy/import 可保存 terminal 展示状态与 attribution，但没有当前 Workspace 的 apply receipt/target source change 就不能声称“曾提交目标修改”。fresh ordinary import 的归因按 imported_unverified；pure Trash/restore byte-equal 保留 token/Value，不产生新 decision。 |

对已有 Annotation，gate 必须先把合法 Proposal 展开成完整 candidate Value/4，**之后**才做 no-op 比较。ordinary_edit 在 author-visible suggestion 输入与 target-dependent 输入均不变时必须完整保留 current Suggestion lifecycle/evidence，绝不能隐式 reconfirm pending 或重写 terminal evidence。candidate 初始继续携带 before 的 creator/authoredAt/lastEditor/editedAt；只有 candidate 与 before 的 canonical bytes 真正不同，才写 fresh lastEditor/editedAt 并分配 fresh revision token/SourceRevisionPlan。no-op 因此不会授予额外能力，也不能绕过 stale/currentness/final-barrier 检查。

继承的错误/披露顺序按操作需要保持：closed decode → Annotation 状态披露 → annotation_read → annotation_write（发生变更时）→ exact token/current Annotation，总是在任何 target-source 披露之前。ordinary_edit 若保持已有 no_suggestion 且 stored target content 不变，不能仅因新增 gate 就平白要求 target-source-read；pending 清为 no_suggestion 时同理。no_suggestion→pending、manual reattach、显式 reconfirm 与 apply 只有在各自具名 target qualification/read 阶段才消费目标信息。目标隐藏时，必须在读出 target bytes/expectedText 前返回 not_visible；provider 或 source 不可用不能伪装成 orphaned。reconfirm 失败时保留原 pending+needs_reconfirmation。permission revocation、并发 token/Observation 变化、stale Draft/base binding 或 final read barrier 失败都按继承规则零变更/重新准备，即使展开后的 candidate 原本会相等。非交互/generic constructor 以及旧 selector/PAB/Draft/recovery artifact 都不得重新开放同一旁路。

### 9.11 Current proposed carrier 与 Annotation read chain

`D7ProposedInput/3` 是 D7ActionInput/3/PAB4 的唯一 current proposed-input carrier。D6-owned apply/reject/其它 concrete current Annotation after 必须 `payloadKind=annotation_value, encoding=d3_annotation_value4`，pin 完整 Value/4 canonical bytes；D3 合法 symbolic-result 分支继续使用 `d3_symbolic_result9`。真实 PAB3/D7ProposedInput/2 保持 d3_annotation_value3 的原 decoder/bytes/pins，current decoder 不接受把 Value4 塞进 /2。

fixed D7 Query Algebra §2 的 `{kind:"annotation_body"}` 当前句由本段替换：from 仍是 AnnotationRef、输出仍是 text，但 producer 必须在最终 authorized query read barrier 读取 current PortableAnnotationRecord/4，并以唯一 AnnotationInlineProfile/1 求值。body=null 投影为 `""`；valid body 输出 **R6 semantic text**，不是 exact source。因此 source `*Alice*` 的 query text 是 `Alice`，而 D8 read/Draft surface 返回的 exact source 是 `*Alice*`。invalid R6 在 annotation_read/currentness 成功后使整次 read 按 existing D7 `source_unavailable` 失败；禁止退回历史 D2 plain_text、strip-markup 猜测或 partial semantic text。

SCHEMAS §6.3.1 的 `d8_annotation_read` 与 `d8_annotation_draft_open` 是实际 Core/D8 producer。read 先做 Annotation disclosure/annotation_read，再取得 current aggregate-backed record、Annotation SourceObservation/token 与 target-resolution；它返回完整 Value/4（含 creator/authoredAt/lastEditor/editedAt）以及 body exactSource+semanticText/diagnostics。target 本身 hidden/unavailable 时，Annotation 本体若可读仍可返回，targetResolution=unavailable 且不泄露 target source bytes。Draft-open 同时返回该 complete read 与 discardable editable projection；无 annotation_write 时 access=readonly，不能 prepare。editable prepare 必须绑定 base token/Observation，并在 planning 前和最终 read barrier 重验 current record、aggregate observation、authorization 与 dependency cut；任何一项移动都 stale/reprepare，不能靠相同 Value hash 或 locator 继续。

### 9.12 `weftext.annotations.json` physical aggregate 与 D6 安装

逻辑 authority 仍只有 Portable Workspace Metadata；Node-local `weftext.annotations.json` 是它的 Annotation physical carrier，不是第二 metadata root、DB、ledger 或 CAS。当前物理格式只有 SCHEMAS §7.1 `AnnotationAggregate/1`。sidecar 位置由当前 Node FileBinding/managed-node boundary 与固定 basename 机械确定，path 不存进 aggregate；协调 rename 只更新 FileBinding/physical observation，ownerNodeRef、AnnotationRef、Value4 与 revision token 不变。其它 `.weftext-meta` portable facts 保持 D6 原 owner；未来 sharding 只有 closed layout successor 能改变物理布局，并必须给每个 AnnotationRef 唯一 deterministic shard owner，绝不双写。

每条改变的 Annotation 仍各自拥有一个 logical `PortableAnnotationRecord/4`、一个 final AnnotationRevisionToken、一个 Value4/source-revision result；整文件 canonical bytes/FileObjectBinding/AnnotationAggregateObservation/安装 pin 是另一个**物理层**。一个 record 改变会让同文件旧 aggregate observation 与依赖其 FileObjectBinding 的 current read observation stale，但不会给其它 byte-equal record 分配新 AnnotationRevisionToken、Value4、managed SourceVersion 或 H。合法 fresh read 重新读取/strict-decode 完整 aggregate 并给未改 logical record 重新建立 current observation；这是 observation refresh，不是 source mutation。

准备写入时，Storage 从 fresh AnnotationAggregateObservation/1 开始；其 FileObjectBinding 可按 fixed D6 Control §2 为 absent 或 present，不能用裸缺失/猜测代替。把本 DecisionKey 的全部 logical record changes 应用到同一内存 aggregate，保持未改 records 的 logical bytes，然后只生成一个 AnnotationAggregateInstall/1 after pin。managed_atomic strong path 的 strict 安装沿用 D6 §6 与 Control §2 的 create_only/conditional_replace/exclusive_write_window 资格；before FileObjectBinding/object generation 不匹配就 stale/paused，read-hash-then-rename 或 digest equality 不是 CAS。D6 §4.1 已合法的 observed_only 路径仍可使用 observed_replace，但它只保留该 owner 的弱保证，不能声称 managed atomic/CAS。两个并发 managed 操作改同一 sidecar 时最多一个 physical CAS 先成功；失败方 fresh re-read 后重新构造包含胜者 records 的 after，不能 LWW 覆盖。

同一决策若修改同一 Node 的多个 Annotation，InstallationNotice3/CP4 仍逐项列出每个实际 `PortableComponentKey.annotation` logical component after，并与各 Annotation sourceChanges/final revision 对齐，但 InstallationPlan 对 sidecar 恰一个 physical after。P seal 前验证完整 aggregate after、每个 changed logical component、SourceRevisionPlan/value pin 与 file install 一致；一个失败则整组不 seal。sealed production history/旧 PreparedIntent 永远不因后来重写 aggregate container 而更新、复活或 repin。

同步/接纳必须先按原 D6 的 ChangeRecord/Notice/CP 与当前冲突连续链完成验证，再严格解码整个 aggregate；到达顺序、mtime、修订号、相同 hash 或 provider 的 “latest” 声明都不能选定胜者。重复 Ref、owner 错误、reply 环、重复 JSON key、截断、未知 format/version 或 component/aggregate 不匹配都禁止部分信任：强路径必须进入 incomplete/integrity_conflict，获授权的原始修复/备份只能取得精确原始字节，不能补造成员。外部直接编辑 JSON 即使语法有效也不是可信 Core 决策；只有已验证的 portable transition 或显式 import/admission 才能建立新的当前状态，调用方提供的 actor/lifecycle 字段不能因此变成可信的 apply/reject 证据，普通导入的 attribution 仍为 imported_unverified。

backup 在显式 Frontier 保存 ordinary files + exact portable sidecar bytes，不携带 current permission/PAB/ActionEvidence。Node copy/import 通过既有 identityMap/candidate-map 生成 fresh AnnotationRefs、改写 target/reply，并输出新 owner 的唯一 aggregate；terminal Suggestion 可作为历史展示数据保留，但不会制造 destination target mutation receipt。Trash/restore 若 Value4 不变只移动原 lifecycle/portable ownership 状态并保留 token；restore 后 fresh read 重新绑定物理 observation。purge 删除相应 record；最后一个 record 被 purge 时 canonical after 是 sidecar absent。reply graph、history retention 与 no-reuse 仍按 §9.7/D3 原规则，container rewrite 不改变它们。

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

beforeSourceSha256 是整份 source 的交叉字段，不是自由 digest，也不是 Event slice 的摘要。它必须由 evidence.before 所标识的完整精确 source bytes 计算：

```text
beforeSourceSha256 =
  "sha256:" + lowercase_hex(SHA-256(exact_before_source_bytes))
```

exact_before_source_bytes 是 ownerNodeRef 在 evidence.before 这份 managed SourceVersion/2 上的完整字节。receiver 必须从 producing CP/history 与 retained version evidence 取得这份历史字节，而不是使用当前文件。接收验证必须重新计算 digest 并逐字节相等后才能接受 signed evidence。removedSha256、removedByteLength、SourceVersion 字段相等或各 slice replay 成功都不能替代整文件 hash。无法取得 exact-before bytes 时沿用既有 proof/state-unavailable 边界；已经证明 digest 矛盾则属于 integrity failure。SourceTransformEvidence 不因此升 schema/version。

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

### 13.1 双 profile policy conflict resolution

公开的 d6_conflict_prepare request 继续使用 fixed-parent wireVersion3；policy_bundle_choice 的 JSON 成员名仍是 selected、policy、freshAuthorizations。当前 unseen policy arm 使用 SCHEMAS §9 的 FreshDomainAuthorizationSpec/2，其 profile 是 SealProfileId/1。source_merge 与 choose_source_head 继续沿用 fixed-parent 合同。这里不新增 submit surface：conflict_resolve、workspace policy_admin、subject disclosure、精确 expectedKey、原 error order、唯一 winning planning CAS、原 d6_commit_request/2 与唯一 final P 都保持不变。任何能证明真实保存或规划于旧 owner descriptor 的 policy resolution，都必须恢复原 request、descriptor、pins、handles 与 recovery state，不能重新按 current successor 解析。本候选不声称存在需要虚构 migration 的部署历史。

每个 expectedKey head 都必须先完成验证，才能信任 branch 内容。retained completion proof 严格解码为 ContentCompletionProof/3 的 head 是历史 CP3 head，其 policy after-image 必须严格解码 WorkspaceAuthorizationBundle/1。retained completion proof 严格解码为 ContentCompletionProof/4 的 head 是 current CP4 head：必须验证精确 ChangeRecord/1、Notice3/CP4 linkage、policy ComponentImage bytes/version 与连续链，然后才按 bundle 自身精确 version 分派为 WorkspaceAuthorizationBundle/1 或 /2。保持不变的 WorkspaceAuthorizationBundleAddress/1 仍必须只选 expectedKey.heads 中恰一个 head，其 authorizationRevision、trustRevision、byteLength 与 SHA-256 必须匹配所选 bundle 的精确 bytes。未知 proof/bundle version、CP3 携 Bundle2、CP4 缺 ChangeRecord、address 不匹配、Workspace/root fingerprint 不同或共同历史不完整，都沿用现有 unavailable/integrity 边界失败；禁止 decoder fallback、current-host 猜测、arrival order 或 LWW。

resolver 对每个已验证 head 都要规范化两个 SealProfileId/1 profile。Bundle1 只贡献 revision-token 历史，source-transform 状态视为 none。Bundle2 按精确 Declaration1 prefix 与 Declaration2 suffix 折叠。effective compromise set 从最长公共 trust prefix 开始，并递归合并 selected 与全部合法 losing branch 的 compromise fact。Carry1 必须完全按 fixed-parent Declaration1/CP3 合同验证。Carry2 必须从原 Declaration2 bytes 与原 CP4 activation cut 验证。直接 Declaration2 revoke 且 mode=compromise 时，compromisedTrustKeyId 映射 action.trustKeyId；直接 Declaration2 rotate 且 mode=compromise 时，映射 action.oldTrustKeyId。originDeclarationDigest 哈希包含 rootSignature 的完整 canonical Declaration2。Carry2 的 factId 固定为：

```text
body = {
  workspaceRef,commitDomain,profile,compromisedTrustKeyId,
  originAction,originDecisionKey,originDeclarationRevision,
  originDeclarationDigest,originActivationChangeId
}

factId =
  "sha256:" + lowercase_hex(
    SHA-256(
      ASCII "D6-Trust-Compromise-Fact/2" || NUL || D3-CJ/3(body)
    )
  )
```

originActivationChangeId 永远从原 Declaration2 的 DecisionKey，经 committed CP4 + ChangeRecord/1 与首次追加该 declaration 的精确 Bundle2 after-image 重派生；后来的 resolver ChangeId 不能替代。继承的 Carry1 或 Carry2 只有在重新计算其版本对应 factId、验证具名原 declaration/root signature、直接 compromise action/key 映射、重派生原 activation cut，并验证每一层 carrying resolver 后才能接受。折叠必须递归。effective union 按 ASCII factId 规范排序；byte-equal duplicate 合并，同 factId 但 canonical bytes 不同则 integrity_conflict。未选 branch 的 fact 仍留在 union 中。resolver 不能通过选择另一 branch 或改用 resolver 时间让 compromised key 恢复安全。

affectedDomainProfiles 是所有 head 间 normalized state 不同的 domain/profile pair，加上 effective compromise union 指向的全部 pair。freshAuthorizations 按 canonical bytes 排序唯一，可以指定任一 seal profile，但只能请求 affected pair，且 selected state 必须为 none，或其 selected current key 在 effective union 下不安全。安全且无关的 current key 不能借 conflict resolution 旋转。若无需 trust resolution 且 freshAuthorizations 为空，policy-only 结果保持所选 bundle family 与 trustRevision。否则 current resolver 必须恰产生一条 WorkspaceTrustDeclaration/2 resolve_conflict declaration。若 selected 是 Bundle1，结果以其精确 Declaration1 prefix 为前缀并追加这条 Declaration2，从而成为 Bundle2；若 selected 已是 Bundle2，则直接追加 Declaration2。declaration revision 为 selected.trustRevision+1；conflictId 与 request conflictId byte-equal；selected 与 request 的 WorkspaceAuthorizationBundleAddress/1 byte-equal；predecessor 哈希所选链最后一条 declaration 的精确 bytes；resolvedHeads 等于完整排序后的 expectedKey.heads；inheritedCompromises 等于 canonical effective union 减去 selected chain 已经 effective 的 facts；outcomes 对每个 affected pair 完整。TrustConflictOutcome/2 只有在 selected current key 未被 effective facts 判为不安全时才能 keep_current；否则为 none，或在该 exact affected pair 被显式请求时 authorize_fresh。

每个 authorize_fresh outcome 都使用新生成且受保护的 DomainSealKeyHandle/2，并使用前文冻结的精确 PoP/2 body/message。外层 Declaration2 的 root signature 因此绑定 profile、新 key、PoP、mixed inherited carries、完整 outcomes 与 predecessor。proposed Policy/3 必须完整；若与 selected.policy byte-equal，则 policy revision 保持；否则必须按原 policy_admin 规则成为合法 checked successor。result bundle 的 authorizationRevision 只递增一次；只有实际追加 Declaration2 时 trustRevision 才递增一次。current portable resolution 继续使用原 current Notice3/CP4/ChangeRecord 路径与唯一 final P；staged fresh handle 只能随同一 committed transition 变为 usable。任何 losing branch bytes 都不能改写。

current unseen policy arm 必须形成完整的 owner-input、preview 与 recovery 链，不能只定义 Plan2。OwnerInputBinding/2.protocolOwner 固定为 D6，ownerKind 固定为 d6_conflict_resolution/3；current InputDescriptor/3.intentKind 与其相同，guarantee=managed_atomic、frontierPolicy=exact、saveProfile=control_only，且 sourceInputs 为空。canonicalDescriptorBytes 精确等于 D3-CJ/3(ConflictResolutionInput/3)。source_merge 与 choose_source_head 不使用该 owner version：两者的 ownerKind/intentKind 继续是 d6_conflict_resolution/2，精确 owner descriptor、derived plan 与 preview 继续分别使用 ConflictResolutionInput/2、ConflictResolutionDerivedPlan/1、ConflictResolutionPreview/1；只有 outer unseen D6 carrier 使用 current InputDescriptor/3 + DependencyProof/3 + PreparedIntent/3 family。

ConflictResolutionInput/3 包含完整 branchEvidence 与 ConflictResolutionPolicyDerivedPlan/2。Plan2 包含逐 head CP3/CP4 与 Bundle1/2 分派、selected bundle version/pin、完整逐 head TrustConflictCarryValidationEvidence/1、effective/inherited Carry1|Carry2 集、完整 Outcome2 array，以及 result bundle version/pin。对每个 expected head 和该 head effective fold 使用的每个 fact，carryEvidence 都要保留 direct original compromise activation 以及之后实际经过的每个 resolver carrier。每个 hop 都 pin 验证该 declaration 实际使用的精确 ChangeRecord、completion proof 与 policy bundle；version-1 hop 保留其历史 CP3/Bundle1 activation evidence，version-2 hop 使用 ChangeRecord1/CP4/Bundle2。因此原 Carry activation cut 或中间 carrier 不能从 later current bundle、裸 digest、Derived Index 或 resolver 时间重建。

current policy arm 的 OwnerInputBinding/2.pinRefs 必须是 canonical sorted/unique union，精确包含：每个 branchEvidence.changeRecordPin、completionProofPin 与存在的 policyBundlePin；selectedBundlePin；resultBundlePin；以及 carryEvidence 中每个 origin/carrier hop 的 changeRecordPin、completionProofPin、policyBundlePin。相同 PinRef 去重后只出现一次。policy arm 不允许 branch sourcePins。DependencyProof/3 evidence 仍由其真实 owner 保存，不能因为未重复写进 OwnerInputBinding.pinRefs 就从 PreparedIntent retention 中省略。

immutable owner preview 使用闭合 ConflictResolutionPreview/2。其 conflictId、expectedKey、resolution 必须与 Input3 逐字节相等；branchEvidenceDigest 是完整 branchEvidence array 的 D3-CJ/3 bytes 的 SHA-256。derivedPlan 必须与完整 Plan2 逐字节相等，并完整包含 headEvidence、carryEvidence、mixed carries、Outcome2 与 result bundle version/pin。
PreparedIntent/3.previewBinding 必须绑定该 Preview2 与其受保护的 canonical preview pin。对这个 control_only policy arm，pinDirectory 必须由 OwnerInputBinding.pinRefs、DependencyProof/3 的全部 evidence pin、previewBinding 选择的精确 preview pin，以及 installationPlan 实际命名的 proposal/before/after/recovery PinRef 组成，并按 canonical 规则去重。sourceInputs 为空，因此不产生 SourceObservation evidence pin；同一个 pin 不能重新绑定到另一份 protected record。

唯一 planning CAS 必须同时冻结 InputDescriptor3、OwnerInputBinding2、Preview2、pinDirectory、installationPlan、staged fresh-key association 与 result bytes；final submit 仍只有 d6_commit_request/2，唯一 final P 仍是唯一 commit point。receiver admission 重放冻结的 Input3/Plan2/Preview2/pin set，验证每个 head proof/bundle 分派、每个 retained Carry origin/carrier hop、全部 root/declaration signature 与 predecessor，重算 Carry1/Carry2 union 与 factId，验证 authorize_fresh PoP/2，要求 result-bundle bytes 相等，并验证同一 DecisionKey 的 CP4/ChangeRecord/conflict-record transition。planned recovery 精确恢复这些 descriptor、preview、pins 与 handles，绝不从 current state 派生替代值。能够证明属于旧 owner/outer carrier 的 saved/planned/unknown record，必须在 unseen-current dispatch 前恢复原 request、Input2/Plan1/Preview1、pins、handles 与 OperationId。本候选不因存在 current successor 就推定部署 migration。证据与权限完整的合法 current dual-profile conflict 因而具有正向路径，同时不新增第二 ledger、CAS 或 submit。

必要反例是 transform KT1 并发分叉：ordinary KT1→KT2 与 compromise KT1→KT3。选择 ordinary branch 也不能丢掉 losing branch 中 KT1 在原 cut 的 compromise fact，更不能把该 cut 移到 resolver。只有在完整 fold 下可独立证明安全时 KT2 才可保留。若 selected transform state 为 none，或 selected current key 在完整 fold 下不安全，则显式请求 source-transform FreshDomainAuthorizationSpec/2 必须生成 fresh transform key 与合法 Outcome2，而不是让 Workspace 永久无法 resolution。

### 13.2 Replica registration 当前 producer

d6_replica_register_prepare 保留 fixed-parent wireVersion2 request 与专用 replica_register 权限；不新增 profile 成员，普通 dual-profile trust add/rotate/revoke 不能代替这项 replica 权限。
对当前尚未建立决议的 registration，winning prepare 生成一个 fresh ReplicaEpoch；joining secure store 为该 replica CommitDomain 恰生成两个 fresh DomainSealKeyHandle/2，两者初始均为 staged，profile 顺序固定为 revision-token 后 source-transform。调用方 JSON 不提供任何一把 key。

同一个 frozen plan 在一个 DecisionKey 下按上述 profile 顺序恰追加两条 WorkspaceTrustDeclaration/2 authorize declaration。第一条 revision 为 selected trustRevision+1，第二条为 +2；第二条 predecessor 哈希第一条完整 Declaration2 bytes。每条 declaration 都有自己的 PoP/2 与 rootSignature。plan 同时冻结 expectedReplicaRegistryRevision、selected current bundle/trustRevision、新 ReplicaRecord、两个 staged handle association、两条 declarations 与精确 Bundle2 after-image。

恰一个 final P seal 在同一个 current CP4/ChangeRecord transition 与同一 ChangeId 中同时发布 replica_registry after-image 和 policy/Bundle2 after-image。只有该 commit 被 admission 后，两把 handle 才能一起 staged→usable；任何单 profile prefix 或仅复制 public bytes 都不能使其中一把 usable。receiver admission 必须由同一 CP4 证明两项 component transition、精确 ReplicaEpoch/CommitDomain、固定双 profile 顺序、Declaration2 predecessor chain、同一 DecisionKey、root/PoP signatures、精确 Bundle2 result，并且不存在 unresolved policy/trust conflict。

planned/recovery 必须从 winning plan 恢复同一对 staged handles 与 declarations，绝不重新生成 key。任何确实可证明的历史 replica-registration decision 继续按其 recorded decoder/bytes 恢复，不能事后补造第二把 key。原 retire transition admission 后，该 replica 的两个 profile 都不得再用于新签名。

WorkspaceBootstrapProfile/4 与 Profile3 使用同一闭合成员集合和 issuer semantics，只把 fresh-target trust genesis family 改为 WorkspaceTrustGenesis/2。
WorkspaceBootstrapPlan/4 保留 D3 allocation chain 的 canonical lowercase UUID proposalId 以及其它 UUID 成员。
creator binding 与 target Registry binding 使用 SCHEMAS §9 的闭合 helper type。
series configuration 与 period-scope binding 同样使用闭合 helper type，不留描述性 placeholder。

Plan4 只用于 current unseen fresh create_workspace/fork_workspace bootstrap。原 D3 proposal authenticity、issuer/target-custody ordering、一个 planning CAS、一个 DecisionKey、一个 P seal 全部保持。create 从 frozen eligible seed 派生 target Registry；fork 在 fixed source cut 携带并映射完整 source Registry/configuration history。fresh bootstrap 只有一个 root，并严格按顺序生成两个 Declaration2 authorize：revision 1 revision-token，revision 2 source-transform；两者同 DecisionKey/activation ChangeId，rev2 predecessor hash exact rev1 canonical bytes。两个 staged handle 只有在这一个 seal commit 后才变 usable，不存在可观察的一-profile prefix。

同一 Plan4 还必须冻结 mandatory initialPresentationPolicy。prepare 从原 issuer-proved empty target history 派生 D8 before stamp，绝不能从 missing file 猜空，并冻结 parents=[]、revision=1、separate，以及 committed=null 的 typed presentation_policy_change preview。planning/staging 在原 operation 下携带这些 exact bytes，不提前创建依赖 ChangeId 的 record/hash/pin/outbox。唯一 final P 在全部原 proposal/custody/Registry/Policy/trust/source checks 后分配原 bootstrap ChangeId C，并在同一个 atomic transition 中物化 D8 /2 record、exact prefixed hash/pin/address、committed effect、原 receipt/ChangeRecord association、outbox 与 head transition，同时 activation W/B 和其它全部 bootstrap state。planning CAS loser、abort 或 P 失败都不能留下 half-active Workspace，也不能留下没有 activation 的 policy record。P 成功后，依赖 default 的 presentation 立即以 revision1/separate 工作；publication unknown 只能从 exact outbox bytes 重试，receiver admission 必须核原 bootstrap decision association。saved/planned/unknown Plan4 只能从 recorded init bytes 续接，不重新采样 state。

普通 managed copy 不是 bootstrap，继续走既有 scope/configuration mapping 规则。restore/recovery 按实际 saved decoder/plan 分派，不注入 Genesis2，也不升级 old bytes。continue_workspace/failover 保留 current policy、principal mappings、Registry/configuration 与 profile trust state，不能重新跑 bootstrap。本设计候选尚未部署，因此不声称从 Plan3 有 migration 或 dual-write；任何确实存在的历史 record 只按其 recorded contract 继续。

replica registration 继续使用 specialized same-record producer；current FC operation 为 dual-profile，generic trust prepare 不能冒充 replica_register authority。continuation 对 revision 与 transform profile 分别计算 current(K)|none|conflicted_or_unproved；任一 profile unproved 都禁止新 signing domain 部分激活。其余情况下完整顺序固定为 optional old revision revoke、optional old transform revoke、new revision authorize、new transform authorize，全部在一个 DecisionKey/CP4 中且无可观察中间 prefix。

### 13.3 D6 Storage §9.1 / §9.3 current consumer

fixed-parent D6 Storage §9.1 的单 profile registration/admission 文字只在 current unseen replica registration 上被本候选接管。Storage consumer 必须接收 §13.2 的精确 transition：一个 fresh ReplicaEpoch；按 revision-token 后 source-transform 固定顺序恰两个 fresh staged DomainSealKeyHandle/2；同一 DecisionKey 下恰两条 Declaration2 authorize；replica_registry 与 policy/Bundle2 after-image 位于同一个 Notice3/CP4/ChangeRecord1 与同一 final P/ChangeId；只有完整 transition admission 后，两把 handle 才能同时 staged→usable。receiver 必须交叉验证 active ReplicaRecord、两条 Declaration2 的 predecessor/signature/PoP chain、固定 profile 顺序、同一 DecisionKey 与精确 Bundle2 bytes。retention/planned recovery 保留精确 staged pair、declarations、component pins 与原 request，绝不重新生成 key。可证明存在的历史 registration 保留 recorded single-profile decoder/bytes。replica_register 仍是 specialized authority，generic trust administration 不能替代。replica_retire 的原 transition 被 admission 后阻止两个 current profile 的未来 signing，但不删除历史 authorization，也不接管 ApprovalUse、claim、Money、external unknown、Automation lease 或 execution custody。普通 replica content 与 execution-responsibility takeover 继续是两个独立合同。

fixed-parent Storage §9.3 的 conflict direct consumer 同样按 arm 分派。尚未建立决议的 source_merge 与 choose_source_head 使用 current outer InputDescriptor/3 + DependencyProof/3 + PreparedIntent/3，但仍保留精确 ConflictResolutionInput/2、Plan1、Preview1、source pins 与原 source semantics。尚未建立决议的 policy_bundle_choice 使用 ConflictResolutionInput/3、Plan2、Preview2、mixed Carry1/2、精确 OwnerInputBinding pin union，以及上述 PreparedIntent3 的 preview/pinDirectory 闭环。
Storage 必须保留每个未选 branch 的原 bytes、精确 bundle/activation evidence、result bundle pin 与受保护 preview pin，直到原 last-reference 规则允许释放。final write 仍使用原 D6 typed request 与唯一 P seal。saved/planned/unknown 必须先恢复 recorded outer carrier、request、owner descriptor、preview、pins、fresh-handle association 与 OperationId；current outer type 不得重新解释或迁移这些 bytes。这里仅接管 Storage 的 current direct-consumer 语句，不改变无关的历史 conflict、transport 与 recovery 语义。

## 14. D10 current/historical mixed holders

### 14.1 current owner 路由与历史分派

对于当前尚未建立决议的 D10 Workspace control 操作，协调后的链固定为 D10WorkspaceReadDependencies/2 与 DependencyProof/3 → ControlDependencies/3 → D10ControlInput/2 → ControlPrepareBinding/3。新的自动 author step 使用 PreparedActionBinding/4 与 EffectManifest/3、EffectBytes/3 构造 ApprovalUse/2。新的交互 author step 使用其 owner 合同选定的 PreparedActionBinding/4 或 PreparedEditBinding/3。新的调度记录使用 ScheduleSubscription/2 与 AutomationOccurrenceRecord/2。

这一 current 路由除了已经纳入 replacement 的 D10 sections 外，还明确接管 fixed-parent D10 CONTROL-CONTRACT §4、§7 与 D6 Storage §7.2.1 中的 fresh producer 语句。这里不是全局替换版本名。真实 saved/planned/unknown 记录继续按实际写入时的精确 decoder 与恢复合同处理；若 recorded format 是历史 D10AuthorPreparationLink/1、PreparedActionBinding/3、EffectManifest/2、EffectBytes/2、PreparedEditBinding/2、ApprovalUse/1、ScheduleSubscription/1、ScheduleContinuityWitness/1、ScheduleContinuityStep/1、ScheduleContinuityInvalidation/1、AutomationOccurrenceRecord/1、ControlPrepareBinding/1 或 /2，则其原 pins、canonical request、OperationId、confirmation fact 与 recovery evidence 都保持逐字节不变。current mixed holder 只能通过显式 versioned carrier 引用这些旧责任，不能迁移、重新 pin 或重新生成。

对 fresh core_field_member author preparation，D10 CONTROL §4 的 recovery link 使用 D10AuthorPreparationLink/2。其 preparedBindingToken 选择精确 current PreparedActionBinding/4，request 是该 preparation 产生的原 D6 d6_commit_request/2。Core 必须在返回 prepared author step 或允许 submission 之前，原子保存 Link2、完整 PAB4、EffectManifest/3 与所需 EffectBytes/3 semantic evidence，以及必要 recovery pins。当前 §7 mapping 继续执行既有 fresh Field/Entry selection，但使用 DependencyProof/3；只要语义解析 managed Document，就必须包含 document_format。随后调用 current D7 prepare owner、保留 PAB4、验证完整 EffectManifest3/EffectBytes3 与实际 MutationFootprint，再构造 ApprovalUse/2。Standing Approval 仍不能选择 target、Entry、member path 或 requested value；planning、final seal、saved replay 与原 request 仍归 D6。只要历史 Link1/PAB3/ApprovalUse1 association 已被证明，就必须精确恢复，不能重新准备为 current。

对 fresh 或其它尚未决议的 current external-consent control preparation，ControlPrepareBinding/3 保存 fixed-parent 语义要求的同一个 ExternalConfirmationRequirement/1；独立的 protected ExternalConfirmationRecord/1 继续拥有当前 confirmed fact。trusted attended confirmation event、完整 immutable preview/intent binding、current principal、trusted time、consent interval、dependency revalidation、non-disclosure 与 approval_unavailable/preflight 顺序全部保持。confirmation 不得修改 Binding3。已经保存或已准备的 ControlPrepareBinding/2 保留其原 requirement、canonical intent bytes、Dependencies2、confirmation association、result lookup 与 recovery；不能因为 current unseen prepare 使用 Binding3 就改写。

fixed-parent D6 direct consumer 使用同一分流。current unseen D10 control 使用 D10ControlInput/2、ControlDependencies/3 与 D10ControlEffectPlan/2，不再走 fixed-parent /1-/2 current producer 语句；current unseen automatic author use 消费 ApprovalUse/2 与 Link2/PAB4/Effect3。saved/planned 历史关联必须先按原版本恢复，不能仅因存在 current successor 就改写。

D6 Storage §7.2.1 的 current producer 同样只为 fresh ScheduleSubscription/2 建立新 scheduling registration。当前 D10 scheduling gates 与有限 retention reservation 成功后，同一个 configuration transaction 必须向真实 Core source/control producer 注册，并创建 ScheduleContinuityWitness/2；其 initial/checkpoint 使用 ScheduleRecurrenceEvidence/2，revision=1、consumedTransition=0，并生成 fresh producerEpoch。current witness artifact bytes 精确为 UTF8("D6-Schedule-Continuity/2") || NUL || D3-CJ/3(完整 ScheduleContinuityWitness/2)。当前正向 transition 使用 ScheduleContinuityStep/2、DependencyProof/3，以及真实 current ChangeRecord/1 + Notice3 + CP4 pins；每个 current step artifact 精确为 UTF8("D6-Schedule-Step/2") || NUL || D3-CJ/3(完整 ScheduleContinuityStep/2)。current source 若 vanished、conflicted、invalid、unavailable 或历史出现 gap，则使用已经闭合 D6-Schedule-Invalidation/2 domain 的 ScheduleContinuityInvalidation/2。三种 current pin 都使用 PinRef/2 payloadKind=artifact、retentionClass=recovery，byteLength 与 SHA-256 覆盖完整 domain + NUL + canonical object bytes。digest 只作 comparison evidence；真正认证仍依赖 protected producer/subscription provenance，以及精确 generation、epoch 与 transition continuity。binding_changed 仍必须有已证明的 selected-business discontinuity，gap 仍表示 unavailable/unknown history。producer 继续保留 fixed-parent 的 atomic inbox、retention、counter、capacity、authorization、compaction、error 与 no-reset 规则；这个 successor 只改变 current typed evidence/dependency/component family 与精确 current artifact domain。

D6 producer、fold、compaction、receiver、recovery 与 D10 §16 continuityPins consumer 都必须先按 authenticated domain 分派，再解 inner record。D6-Schedule-Continuity/1 与 D6-Schedule-Step/1 只接受 historical Witness1/Step1；D6-Schedule-Continuity/2 与 D6-Schedule-Step/2 只接受 Witness2/Step2。Invalidation1/Invalidation2 同样严格按 /1 与 /2 domain 分派。unknown domain、domain/object version mismatch、其它 prefix、缺失 NUL 或非 canonical bytes 都必须沿原 authorization/disclosure 与 unavailable/integrity boundary fail closed；禁止 fallback decoder 或扩展 /1 decoder。continuityPins 可以保留精确 current /2 typed pins、精确 historical /1 typed pins，或在真实 retained history 跨越 bridge 时保留完整 version-mixed original typed chain，但每个元素都必须保持原 bytes/domain/decoder。只有 retained exact typed witness/step chain 仍完整证明 fixed-parent continuity invariants 后，compaction 才能释放中间 payload。

已有或 retired ScheduleSubscription/1 仍是合法 historical recovery-retention owner，并继续配套 Witness1/Step1/Invalidation1 与原 /1 pins/producer obligations；绝不后台迁移。已经定义的 same-generation Subscription1 → Subscription2 显式 continue，只有在完整 retained history 证明没有 intervening format/rule/business discontinuity，并建立 current Evidence2/Proof3 cut 后才合法；否则必须 replace 并创建新 generation。合法 continue 保持 semantic generation 与每个旧 typed artifact pin；只有 bridge 之后新产生的 Witness2/Step2/Invalidation2 才使用 /2 domain。旧 /1 bytes 绝不 repin 或重新编码，也不能 reset invalid generation。

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

D10ExecutionClaims/2 必须完整保存全部 mixed responsibility。recordPins 使用 mixed pin 顺序。prepareBindings 按 inner StableControlKey 的 canonical bytes 排序，而且该 key 在 Binding1、Binding2、Binding3 之间全局唯一。leaseRuns 按完整 Run ControlRef 排序唯一。authorSteps 以 run Ref 加 stepId 为 identity，Step1/Step2 跨版本唯一。subscriptions 以 Automation Ref 加 generation 为 identity，Subscription1/2 跨版本唯一。occurrenceRecords 以 AutomationOccurrenceKey 为 identity。ranges 使用 mixed range 顺序。continuityPins 按 pinToken 排序唯一。每个 continuity pin 都是精确 typed D6 schedule artifact，必须先按 retained D6-Schedule-Continuity/{1|2}、D6-Schedule-Step/{1|2} 或 D6-Schedule-Invalidation/{1|2} domain 分派，再解 inner record；禁止 free proof-map、schema 猜测、扩展 /1 decoder 或跨版本 repin。wrapper 只选择精确 decoder；不能给 Binding1 增加 kind、version 或 confirmation 字段，不能 LWW，也不能改变原 pins 或 dependencies。

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

Suggestion 与 reviewState 继续正交；唯一生命周期 authority 是完整 Value/4 中的 Suggestion/3。pending 使用 confirmed|needs_reconfirmation；accepted/rejected 都是 terminal 且 confirmation=not_applicable。current executable actions 只有 §9.9 的 D7ActionSpec/2 apply_suggestion 与 reject_suggestion。

explicit manual reattach/reanchor 是普通 Value/4 target mutation：写入一个新的 exact same-owner target，把 pending suggestion 改成 needs_reconfirmation，推进 Annotation revision，并记录实际 Core editor/time。reconfirmation 是另一个显式 current mutation：必须 fresh qualify 该 exact target，重新计算 targetBasisSha256 与 expectedText/point，再以新的 Annotation revision 回到 pending+confirmed。mapped/candidate geometry 与 pure sync requalification 都不能自动改变 Value/4。

accept/reject 均以 exact current Annotation CAS 为前提。accept 按 §9.9 的三种 kind mapping 和 one-seal Document+Annotation atomicity 执行。reject 存在明确的 target-unreadable 正向路径：只要 Annotation disclosure 与 annotation_read/write 成功，即使 target hidden/unavailable 也不阻断 reject，因为该路径不读取或写入 target bytes。并发 body/review/label/appearance/reply/suggestion edit 都改变同一 revision token，因此旧 suggestion action 必须 stale。accepted/rejected均为terminal，不提供reopen。

## 16. Copy/import/export/backup Annotation

Node copy 产生 fresh AnnotationRefs，完整 rewrite reply graph，并只通过真实 source/resource identityMap 重建 targets。fresh/mapped Annotation 的 target@0 与初始 nonnull reply@1 继续使用继承的 reference-plan/result 规则，不是 existing-Annotation structural S。每个 materialized copied Value/4 只有一个 final Annotation revision；attribution 按 §9.6 处理。旧 locator 不能复制，position 不能靠文本猜测。mandatory target/reply 无法重建时 typed copy 整体失败；raw byte copy 不冒充 typed success。

ordinary import 只按继承 owner rules 创建 fresh imported Annotation identity；可证明的 target/reply 通过真实 import mapping 改写。无法证明的 target 只有在 import profile 明确允许时才能作为 imported-unverified/candidate repair 表示保留，并且不能授权 navigation/write；不得按文本自动重绑。D3 mode matrix 禁止时，fresh imported Annotation 不能借机给 existing Annotation 增加 structural reply mutation。

portable backup 携带当前 PortableAnnotationRecord/4 JSON value/identity 与历史归因展示数据，但不携带当前 permission、SourceObservation、revision-token signing capability、PAB、ActionEvidence 或可执行授权。restore/recovery 必须先按真实记录版本分派：真正旧 D2 Annotation-v2/Value3 record 使用历史 decoder；当前 Value/4 record 严格解码 Value/4。损坏或未知字节只作为 repair/backup 数据保留，不能成为部分可信的当前 Annotation。

export/Review Bundle 只有在每项对应的披露权限成功时，才能包含 Annotation 正文、source/history 摘录与 media-region 上下文。resource-region Annotation 必须保留 target 合同要求的精确 resource identity/version/profile/geometry。渲染方便性不能把 candidate/ambiguous/orphaned/unavailable 状态升级，也不能把隐藏 source 变成导出上下文。

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
D7ProposedInput/3
EffectManifest/3
EffectBytes/3
PreparedEditBinding/3
PortableAnnotationRecord/4
AnnotationEditableValue/1
AnnotationEditableProposal/1
SuggestionAuthorProposal/1
AnnotationAggregate/1
AnnotationAggregateObservation/1
AnnotationAggregateInstall/1
D8AnnotationReadRequest/1
D8AnnotationReadResponse/1
D8AnnotationBodyRead/1
D8AnnotationDraftOpenRequest/1
D8AnnotationDraftOpenResponse/1
D8EditIntent/3
D8EditInput/3
D7CreateAnnotationIntent/2
D7ActionSpec/2
D7ActionPrepareRequest/3
D7ActionInput/3
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
TrustConflictCarry/2
TrustConflictOutcome/2
FreshDomainAuthorizationSpec/2
PolicyBundleHeadEvidence/2
TrustConflictCarryValidationHop/1
TrustConflictCarryValidationEvidence/1
ConflictResolutionPolicyDerivedPlan/2
ConflictResolutionInput/3
ConflictResolutionPreview/2
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
D10AuthorPreparationLink/2
D10AuthorStepResponsibility/2
ScheduleRecurrenceEvidence/2
ScheduleSubscription/2
ScheduleContinuityWitness/2
ScheduleContinuityStep/2
ScheduleContinuityInvalidation/2
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

逐项设计义务见 ACCEPTANCE.zh-CN.md / ACCEPTANCE.md：438条core design oracles +322条actual-owner coordination fixtures，共760条。它们是**未运行的设计验收义务**，不是实现测试结果。

本PR没有安装或运行 Ruby oracle、Asciidork、Mermaid CLI、浏览器/Puppeteer、STEM/PDF providers，也没有运行产品SQLite/replica/crash/crypto/Automation handoff测试。作者自检只能证明文档/JSON/路由的一致性；独立接受必须绑定本PR停止写入后的exact head SHA。