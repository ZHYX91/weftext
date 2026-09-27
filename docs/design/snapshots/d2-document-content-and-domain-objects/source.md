---
_weftext:
  id: "993f6232-c8ae-476b-8ab3-9e071b55cd29"
---

# D2 文档内容与领域对象

状态：冻结决议。本规范只有在控制索引记录 D2 与其对应 D3 amendment 以同一世代原子切换时才成为当前产品权威；单独存在、单独复制或仅作为未激活 staging bytes 时不激活。

决议日期：2026-08-31。

## 1. 范围、冻结上游、权威与非目标

本决议回答 D2：Workspace、Node、Document、DocumentMetadata、DocumentBody、DocumentElement、CoreNodeKind、Facet membership、Task、Template、Resource、Annotation、Citation/Bibliography、NodeCollectionResult、非节点记录和 UnmanagedItem 的本体边界；同时冻结 Weftext AsciiDoc Profile v2、namespace-scoped lexical attribute carrier、最小 wire v2、错误矩阵以及 D3–D10 可以依赖的不变量。

冻结上游是 2026-08-27 D1：

- Desktop、WebUI、Server、CLI、Mobile 使用同一 Core 领域语义，不得出现表面专属同名对象。
- 本机 Core 或 Server 内 Core 是唯一工作区权威；UI、worker、Agent、Provider、缓存和控制面状态不是工作区对象。
- 受管作者源是一份合法 Weftext AsciiDoc；普通文件不按扩展名获得特殊地位。
- 项目没有用户和兼容义务；不为现有目录、sidecar、类型名、语法、别名或迁移路径保留设计。

D2 同时冻结两项长期消歧：

1. canonical document-content algebra 不包含持久 Record、RecordCollection 或 RecordRef；Document table 的行、单元格只作为 DocumentElement occurrence。D5 仍可从零选择与 D2 content graph 分离的 record domain，但不得冒充 Document/Node、共享 NodeRef 或成为同一正文的第二作者权威。
2. 保存的 Query/View 定义只能是某个 Node Document 内的 authored occurrence，地址为 owning NodeRef 加 document-local locator/anchor；不存在独立 durable View identity、ViewRef 或 workspace-control 内容 owner。

本次重开冻结三个正交结论：A+ reserved AsciiDoc header 保存 Node control；Task 是 ordinary Node 上的 built-in `tasks/task` Facet；持久属性使用 namespace-scoped lexical carrier。旧冻结 D2 与旧冻结 D3 在原子切换前继续共同构成唯一产品权威；不得先激活本 D2 再补 D3。

明确非目标：

- D3：NodeRef/ResourceRef/AnnotationRef 的编码，一般 copy/move/delete/recovery/tombstone，以及 locator 生命周期；本次只要求受限 amendment 重证本决议改变的七个切片。
- D4：canonical FieldId、typed value、schema、repeatable entry key、note/source/provenance、关系方向/基数/删除与 Facet registry。
- D5：独立 Record/RecordCollection 是否存在及其 persistence/owner/identity/schema/CRUD/规模与表格式 UX。
- D6：物理目录/sidecar/数据库布局、事务、revision、权限、同步、冲突、索引、strict UTF-8 byte envelope 与 repair。
- D7：QuerySpec/ViewSpec/ActionEvidence 的内部语法、执行与呈现。
- D8：编辑器状态机、交互、IME、a11y 和设备差异。
- D9：Template target plan、参数、slot、实例化、转换 worker、导入导出。
- D10：Facet/provider/package/connector、Agent、自动化、approval 与 audit。

D2 冻结的是这些主题必须服从的对象、作者权威、语法和 wire 边界，不冻结其内部协议。

## 2. 冻结结论与核心原则

本决议冻结三个正交结论：

1. **Node control = A+ reserved AsciiDoc header**：完整作者源仍是一份合法 Weftext AsciiDoc。`wf-kind`保存封闭Core meta-kind，`wf-facets`保存0..N composable FacetIds；没有YAML envelope、comment preamble或sidecar authority。
2. **Task ontology = B2**：Task是包含built-in `tasks/task` facet的ordinary Document-bearing Node；没有TaskRef、Task wire kind或第二lifecycle。Tasks UI/module可禁用，但source meaning随Core contract分发。
3. **persisted attribute syntax = namespace-scoped lexical carrier**：D2 v2 冻结全局reserved block opener `[weftext-attributes]` + protected `....` payload，只冻结outer framing、lexical entry/ranges、raw preservation、limits与plain-export loss。Canonical FieldId、typed value、entry ID、note/source schema不在D2冻结。

Template独立保留为`wf-kind: template`的Core meta-object。Task Template在D9-owned target plan声明目标`tasks/task` facet；Template Node本身不因此成为Task。

核心原则：

- Node 是 D2 content graph 中唯一可独立存在、可独立引用并拥有 Document 的受管作者内容实体；D2 不声称所有未来 Workspace domain extension 都必须是 Node。
- 每个 Node 恰好拥有一个 Document；完整 exact source 是 Document、CoreNodeKind、Facet membership、metadata、carrier 与 body 的唯一作者权威。
- Document 内的 heading、paragraph、list item、table row、citation、bibliography placement、saved Query/View definition、AttributeCarrierBlock 与 LexicalAttributeEntry 都是非 durable occurrence。
- Resource 与 Annotation 是由一个 Node 严格拥有的非节点对象；它们各有单一作者源，但不拥有 Document 或 Node 能力。
- Task 不新增 identity、owner、wire kind 或 lifecycle；它是 available Facet set 包含 exact built-in `tasks/task` 的 ordinary Node。
- Template 是 `coreKind=template` 的 Core meta-kind，不是 Task Facet，也不是隐藏库存。
- NodeCollectionResult 是无身份、无 owner 的求值结果；membership 不形成第二 structural parent。
- exact source 之外的 projection、classification、validity attestation、index、cache、render 与 typed view 都是可丢弃派生状态，不能成为第二作者权威。

## 3. 领域代数与对象关系

```text
Workspace (aggregate boundary; not content)
  └─ root Node
      ├─ ordered child Node*
      ├─ Document exactly 1
      │   ├─ DocumentMetadata exactly 1 when projection is available
      │   ├─ AttributeCarrierBlock* when projection is available
      │   │   └─ LexicalAttributeEntry+
      │   └─ DocumentBody exactly 1 when projection is available
      │       ├─ closed DocumentElement tree
      │       ├─ Citation occurrence*
      │       ├─ BibliographyPlacement occurrence*
      │       └─ SavedQueryViewDefinition occurrence*
      ├─ Resource*
      └─ Annotation*

CoreNodeKind := ordinary | template
TaskNode := ordinary Node where available Facet set contains exact `tasks/task`
NodeCollectionResult ──references──> live authorized Node*
Record / RecordCollection / RecordRef ∉ D2 content graph
UnmanagedItem ∉ Workspace domain graph
```
### 3.1 Workspace

Workspace 是可移植聚合、授权和事务边界，不是内容实体或开放的“杂物 owner”。D2 只定义 Workspace 的 contentRootNode 指针；全部 D2 document-bearing content 必须经该 root 进入 Node 树。D5 若选择独立 record domain，必须自行冻结单独的 Workspace entry point/owner，而不能复用 contentRootNode、Node parent 或 Document authority。

账户、ACL 存储、设备草稿、窗口、缓存、索引、presence 和在线状态不是工作区内容。D2 v2 不允许 contentRootNode 之外的 workspace-owned Resource、Annotation、SavedView、NodeCollection 或 AuxiliaryRecord。Workspace control metadata 只能承载控制事实，不能承载作者内容。D5 的独立 record root（若有）不属于 Workspace control metadata，也不属于 D2 content graph。

### 3.2 Node

Node 是 D2 content graph 中唯一长期受管、Document-bearing 的作者内容实体：

- 可被独立引用；
- root 之外恰有一个 structural parent；
- 同一 parent 下具有唯一、连续、从 0 开始的 sibling ordinal，因而 Node tree 是有序树；
- 恰好拥有一个规范 Document；
- 可拥有零个或多个 Resource、Annotation 和 child Node；
- 可参与 D4 属性/关系与 D7 Query/Action；
- 生命周期独立于父/兄弟的正文编辑。

“容器 Node”也必须拥有 Document；body 可以为空，但 title 必须由作者显式给出。不存在无 Document 的 folder/data Node。title 是 DocumentMetadata 派生的非身份显示字段，Workspace 内允许重名。外部损坏使 projection unavailable 时，Node 仍按 D3 identity 出现在 repair inventory，UI 不得把路径或文件名回写成 title。

Node parent 与 sibling ordinal 是 Node control state，不来自 heading、saved view 或 collection membership。Node move 不改 Document heading；heading move 不改 Node parent/order。

### 3.3 Document

Document 是 owning Node 恰好拥有的一份 Weftext AsciiDoc Profile v2 exact source 及其语义解释，不具有第二个独立长期身份。跨边界地址使用 owning Node 的 D3 NodeRef。

Document 逻辑结果分为：

- exact source：唯一 lexical-lossless 作者权威，始终可在 repair surface 读取；
- parse result：valid 或 invalid，加有序 D2 document diagnostics；
- projection：valid 时为 available，包含 DocumentMetadata 与 DocumentBody；invalid 时整体 unavailable；
- D2 commit eligibility：`d2CommitEligibility`；valid 为 eligible，invalid 为 reject。它只回答 D2 source/profile/projection gate，不预判 D4/D6/D7 的下游 gate。

DocumentMetadata、DocumentBody tree、render、outline、search text、bibliography、classification、typed properties、Query rows、索引和编辑器模型全部由 exact source 派生，不是可单独写入的第二权威。

### 3.4 DocumentElement、Block 与 Inline

DocumentElement 是 parse 后可定位的语法 occurrence，不是领域实体。v2 只支持第 4 节和第 5 节列出的封闭 block/inline kind；没有省略的“其他同类”类别。

Element 可具有 D3 locator token 和可选 document-local author anchor。AttributeCarrierBlock 与 LexicalAttributeEntry 只有 current-revision source range，不取得 locator token、EntityRef、Annotation target 或跨 revision continuity。anchor 只在 owning Document 内唯一；不同 Document 可同名。跨文档地址必须先给 NodeRef，再给 local anchor/locator。locator 或 anchor 永远不能被 EntityRef decoder 接受，且 D2 不保证 locator 跨 revision 有效。

### 3.5 Heading、Paragraph、ListItem、Table 与 checklist

Document title 是 metadata，不是 heading element。body 中 section heading、paragraph、list/list item、table/row/cell 都是 occurrence。

checklist item 是 list item 上的 null/unchecked/checked marker，不是 Task；toggle 是 Document edit。只有当某项需要独立状态、日期、依赖、关系、Resource ownership、独立引用或生命周期时才提升为 Task Node。

Promotion 是显式 Action：创建 fresh ordinary Node，在 exact source 中声明 built-in `tasks/task` Facet，并在同一原子计划中以普通 Node link 替换原 checklist occurrence，并冻结新 Node 的 structural parent。调用方未指定 parent 时，默认 parent 为包含 occurrence 的 Node。原位置不保留 checkbox/task mirror。D2 不冻结源文本如何映射为新 Node 的 title/body，也不冻结替换 link 的 label；D7 ActionSpec 必须显式携带这些 source transformation inputs，不能由不同表面各自猜测。

D2 v2 的 Document table row/cell 不具有 RecordRef、独立 owner、Resource、Annotation 或生命周期。D5 可以在 occurrence 边界内增加原生表格编辑，也可以另行定义独立 JSON record dataset；两者之间若有导入、链接或提升，必须是显式 Action，不能让同一 row 同时成为 Document occurrence 与独立 Record 的双身份/双权威。


### 3.6 CoreNodeKind、Facet、Task 与 Template

`node_classification` 只在同一 owner、同一 exact-source revision/digest、同一 D2 parser contract 已有 full-parse-valid proof 时 available。其语义为：

```text
CoreNodeKind := ordinary | template
FacetId := closed lexical namespace/name token
NodeFacetSet := finite unordered set<FacetId>
TaskNode := Node where CoreNodeKind=ordinary and NodeFacetSet contains exact `tasks/task`
```

- `wf-kind`缺失投影 ordinary；唯一显式值是 template。
- `wf-facets`缺失投影 empty set；membership 是 exact-code-point unordered set，source order仅用于 lossless edit 与 range。
- Task 是 ordinary Document-bearing Node，不形成 TaskRef、Task wire kind、第二 owner、第二 lifecycle 或 Task/checklist mirror。
- `tasks/task` 的最小语义 marker 随 Core contract 分发；Tasks UI/module 可以禁用，但 source、NodeRef、generic read/export 与 Facet predicate 仍成立。
- Template 是 Core meta-object。Template Node 禁止同时声明 `tasks/task`；Task Template 只能在 D9 target plan 中要求新实例采用 ordinary + `tasks/task`，Template 自身不变为 Task。
- title 仍来自 DocumentMetadata，prose 仍来自 DocumentBody，附件仍是 owner-local Resource；portable structured facts 只能经 canonical exact source 承载。
- full parse invalid、proof missing/stale 或 owner/revision/digest/profile 不匹配时 classification 必须 unavailable；不得从旧 cache 或 header-only candidate 推断 ordinary/Task/Template。

### 3.7 Resource 与 Attachment

Resource 是由一个 Node 恰好拥有的 opaque byte object：

- 不是 Node，不拥有 Document、child Node、Node tree position 或 Node 属性集；
- bytes 是作者权威；length、digest、detected media type/dimensions 是派生技术事实；
- 只可被 owning Node 的 Document 或 Annotation 以 owner-local ResourceRef 直接引用；
- 不因扩展名、MIME 或文件名自动成为 Weftext content。

Attachment/Image 是 Resource occurrence 的 presentation role，不是实体种类。caption、alt、page、width 和 height 属于 occurrence，不是 Resource 全局事实。

跨 Node 复用必须复制为目标 Node 的新 Resource，或使用普通 Node link 指向 owner；不得直接嵌入另一 Node 的 ResourceRef。普通 HTTPS link 是 inert link occurrence；parse/render 不 fetch。raw path、URL 或任意非 owner-local token 不能被 ResourceRef decoder 接受。

### 3.8 Annotation

Annotation 是 Node 直接拥有的非节点辅助记录。`purpose` 是封闭判别值 `comment | mark | suggestion`；thread 不是第四种 purpose，而是由同 owner 的 `replyToAnnotationRef` 形成的 reply 关系。它只可 target owning Node 的整个 Document、可地址 DocumentElement、Document range 或 owner-local Resource/region；跨 Node target 不允许。

Annotation body 在 v2 只能是 inert plain UTF-8 text。它不解析 AsciiDoc、HTML、Markdown、macro、attribute reference 或 active content；render 时按纯文本转义。Annotation 没有第二套 inline profile。comment/mark 的 `suggestion` 必须为 null；suggestion purpose 只允许 document_range target，并要求一个封闭的 `replace_plain_text` payload、预期 Document revision token 与 replacement text。

target locator 在预期 revision 无法解析时，Annotation 可保留为 stale；accept/replay 必须 fail closed。suggestion 接受还必须匹配 payload 的预期 Document revision，生成显式 D7 Document Action，且永不直接写 source；Document+Annotation 原子提交由 D6。AnnotationRef、author、reply 生命周期、delete/recover 和 locator/reanchor 的内部协议由 D3/D6，但本节冻结其可观察 purpose/reply/payload 外形。

### 3.9 Citation、bibliographic reference 与 bibliography

Citation 是 DocumentBody inline occurrence，不是实体。v2 citation target 只有一种明确域：同一 Workspace 内的 Node。target envelope 为 kind=node 加 opaque node token；D3 冻结 token 编码，D4 冻结目标 Node 中 reference facts 的 schema。Citation 可以跨 Node，因此 Resource 的 owner-local 规则不适用于 citation。

BibliographyPlacement 是 body block occurrence，决定 bibliography 在正文中的位置。renderer 从当前 Document citation occurrences 解析目标 Node 并派生 entries、编号、排序和显示；这些都是 presentation，不回写 source。placement 删除不删除目标 Node 或 reference facts。

D2 v2 不支持 Record bibliographic target、document-local key 或裸字符串 target。未来增加新 target kind 必须重开 D2；不得把 RecordRef coercion 为 NodeRef。

### 3.10 Node Collection、Saved Query/View 与 D5 record domain 边界

NodeCollectionResult 是 Node-only 的无身份 evaluation result：

- 没有 collectionRef、owner、Document、Resource 或生命周期；
- members 只包含求值时仍存活且已授权的 NodeRef；
- 不拥有成员，不改变 parent/order，不复制 Document；
- 一个 Node 可出现在多个结果，移除 membership 不删除或移动 Node。

若用户保存 Query/View，其定义必须写入某个 Node Document，作为 SavedQueryViewDefinition occurrence。它的 durable address 只能是 NodeRef 加 document-local locator/anchor，不存在 ViewRef、独立 owner 或成员快照。D7 冻结 occurrence 的 query/view payload、evaluation、sorting、pagination 和 UI，但不能改变该作者权威或创建第二持久对象。

D2 的 document-content algebra 与 wire 没有 Record、RecordCollection、RecordRef 或 record collection result；table row/cell 是非节点 occurrence。D5 可以不重开 D2 就定义独立 record domain，但必须同时冻结其 Workspace entry point、owner、authority、identity 和 wire，并满足：Record 不是 Node、没有 Document/Node parent；RecordRef 与 NodeRef 不复用或 coercion；RecordCollectionResult 与 NodeCollectionResult 使用不同 domain/discriminant；任何 Document row ↔ Record 转换是显式 Action，不是同一 identity 原地变种。若 D5 要让 Record 成为 Document-bearing content、复用 Node identity 或进入 D2 content tree，才必须重开 D2。

### 3.11 UnmanagedItem

Unmanaged file/directory 是 Core inventory 暴露的物理入口，不是 Workspace 领域对象。它没有 NodeRef、Document、属性、关系、CoreNodeKind/Facet classification、Annotation、Query entity row 或 Node lifecycle。扩展名不改变此结论。采用/导入必须创建新的受管对象，不能在读取时静默升级。

## 4. Weftext AsciiDoc Profile v2：封闭且可执行

### 4.1 Exact source、parse pipeline 与安全不变量

D2 parser 的固定阶段为 logical lines → header/control → attribute-carrier discovery → body block/inline → ref/anchor validation。`document_payload.source`只在D6对完整physical byte envelope完成strict UTF-8 decode后才构造；decode失败时没有D2 payload，也没有D2 diagnostic。进入D2的exact source是从scalar 0到EOF的完整decoded Document string，并可无损重编码为同一valid UTF-8 byte sequence；它包含BOM scalar（如有）、title/header/carrier/body/trivia/line endings。D2不normalize Unicode、LF/CRLF/CR、comment、order、spaces或raw entry。physical byte handle、decode/repair与malformed byte provenance仍由D6拥有。

固定phase：logical lines→header/control→attribute carrier prefix→body block/inline→ref/anchor validation。任一phase都不fetch/include/execute/read environment/open path/run provider code。

### 4.2 A+ header grammar

Header grammar在本文内完整闭合：

- blank logical line只含ASCII space/tab或为空；line comment只允许column 0的`//`或`// `后任意文本，并作为source trivia。
- 首个non-blank/non-comment logical line必须匹配`= TITLE`，等号后恰一个ASCII space；TITLE至少一个非空白字符，首尾不得是space/tab。title中的colon不拆分subtitle，也不运行Inline lexer。
- title之后到首个blank line只允许line comment或header attribute；首个blank结束header。若无blank，EOF结束header且body为空。
- ordinary header attribute只允许精确形状`:name: value`。attribute name匹配`[a-z][a-z0-9-]{0,63}`并按exact ASCII-lower比较；duplicate name拒绝。value是第二colon和一个ASCII space后的exact source substring，允许空字符串，不trim、不展开、不重新lex。
- `subtitle`是唯一subtitle source；不接受implicit author/revision line。`wf-kind`与`wf-facets`是dedicated Node-control projections，不进入`headerAttributes`；其余合法attributes按source order投影raw name/value/ranges。
- control/header values、comments、line endings、order与spaces都只由完整exact source权威承载，不存在header sidecar、YAML或provider fallback。

```abnf
lower         = %x61-7A
digit         = %x30-39
segment-tail  = lower / digit / ("-" (lower / digit))
lower-segment = lower *segment-tail
namespace-id  = lower-segment *("." lower-segment)
facet-name    = lower-segment
facet-id      = namespace-id "/" facet-name
kind-line     = ":wf-kind: template"
facets-line   = ":wf-facets: " facet-id *(SP facet-id)
```

closed constraints：

- `wf-kind`缺失投影`ordinary`；唯一显式值`template`。空/其他value invalid。
- `wf-facets`缺失投影empty set；存在时1..32 tokens，tokens间恰一个ASCII space。由32×127-byte tokens + 31 SP可达的exact semantic maximum为4,095 bytes；4,095合法，4,096必然违反token/count grammar。
- `namespace-id`与`facet-name`都只含ASCII。每个`lower-segment`必须以`lower`开头；digit可在首字符之后出现；hyphen只可出现在segment内部且必须紧跟lower或digit，因此leading/trailing/consecutive hyphen都非法。dot只分隔`namespace-id`中的两个非空segment；`facet-name`不允许dot。
- 完整`namespace-id`（包含dot）最多63 ASCII bytes；完整`facet-name`最多63 ASCII bytes；完整`facet-id`（包含唯一slash）最多127 ASCII bytes。所有边界都按exact UTF-8 bytes计数；因grammar为ASCII，byte count等于scalar count。
- FacetId与namespace lexical comparison只按exact ASCII code point进行；禁止case-fold、Unicode normalization、percent decoding、trim或任何canonical rewrite。`Vendor/task`与`vendor/task`不等，且前者直接lexical-invalid。
- source order保留用于range与lossless edit；semantic membership是exact code-point unordered set，无precedence。
- duplicate FacetId在第二token精确span报错；unknown但lexically valid FacetId是D2-valid并raw-preserved。
- control lines各最多一次，位于所有普通attributes之前，relative order kind→facets；任何其他`wf-` name、legacy `wf-specialization`或late control line invalid。
- `template` + `tasks/task`是`template_task_facet_conflict`；Task Template必须使用独立target plan。

### 4.3 Namespace-scoped lexical carrier

carrier只允许位于header结束之后、首个ordinary body element之前。为使查找carrier本身可有界，D2冻结一个**attribute-carrier discovery prefix**：

- 起点是header prefix的physical end-exclusive；若header以terminating blank结束，就是该blank的EOL之后；若header在EOF结束，起点就是EOF。
- 从该起点开始，blank logical line、column-0 comment、完整carrier block及它们之间/之后的blank/comment trivia都连续属于该prefix；所有被包含logical line的CRLF/CR/LF bytes都计数。
- 终点是首个ordinary body logical line的physical start，若没有body则是EOF。首个ordinary body开始后，exact opener再出现是`misplaced_attribute_carrier`；protected literal/source raw text中的任何同形内容都inert。
- 即使Document最终没有carrier，只要scanner仍在跳过可能领先于carrier的blank/comment trivia，这些bytes也属于discovery prefix并受1MiB限制；不能用unbounded trivia寻找未来opener。

normative outer source：

```adoc
[weftext-attributes]
....
namespace people
entry <opaque one-line payload owned by D4>
entry <opaque one-line payload owned by D4>
....
```

closed D2 grammar：

```abnf
lower           = %x61-7A
digit           = %x30-39
segment-tail    = lower / digit / ("-" (lower / digit))
lower-segment   = lower *segment-tail
namespace-token = lower-segment *("." lower-segment)
carrier-opener  = "[weftext-attributes]" EOL "...." EOL
namespace-line  = "namespace " namespace-token EOL
entry-line      = "entry " 1*utf8-non-line-break EOL
carrier         = carrier-opener namespace-line 1*entry-line closer-line
closer-line     = "...." (EOL / EOF)
```

`utf8-non-line-break`是一个完整、严格UTF-8解码的Unicode scalar，排除仅% x0D（CR）和% x0A（LF）；它不是Python/host的“line separator”抽象，因此U+0085、U+2028、U+2029、VT与FF都允许在entry payload内。

- logical line break只认CRLF、CR、LF；scanner必须先把CRLF作为一个indivisible EOL匹配，再匹配lone CR或LF。U+0085、U+2028、U+2029、VT、FF及其他Unicode/Python line separators都只是普通non-line-break scalar。
- namespace-token与§4.2 `namespace-id`使用完全相同grammar、1..63-byte整体上限与exact-case comparison；不存在第二套dot/hyphen/digit/normalization规则。D2不裁决它是不是canonical semantic namespace或谁拥有它。
- 每个entry只投影exact `rawEntrySource`与normative `source_range`（0-based logical line + Unicode scalar column，end-exclusive）；D2不解释JSON、FieldId、value、entry key、note、source、provenance、ref或business cardinality。验证工具可额外输出明确标注为test-local的UTF-8 byte span，但它不进入产品wire。
- 一个Document最多32carrier blocks、总计8192entries；第33个exact opener在其完整logical line产生rank 150；第8193条entry在完整固定`entry` marker一经判定时产生rank 150，range只覆盖该marker，之后payload不读取或验证。
- carrier discovery冻结的是**inspection budget**，不是事后按semantic prefix长度追算的limit：从header end起，任何为了判定blank/comment/reserved opener/ordinary body或carrier framing而消费的byte都计入1,048,576-byte budget。ordinary body的首个已判定non-prefix byte立即结束discovery，semantic carrier prefix可为0；但若leading space/tab、comment、reserved stem或framing line在budget内无法完整判定，scanner fail-closed，不访问budget外source。为完成CRLF/strict UTF-8 scalar判定，最多只可再inspect4个sentinel bytes，总access上限为1,048,580；rank150指向第一个会越界的indivisible sequence前zero-width point。测试输出必须同时记录actual carrier/entry access与此upper bound。
- 每条`entry-line`的physical byte span从`entry `的`e`开始，到其required EOL的physical end-exclusive结束，包含literal prefix、opaque payload及CRLF/CR/LF bytes，最多65,536 bytes，恰好N合法。N+1同样在首个会越界的完整scalar/EOL sequence之前返回zero-width rank 150。entry payload仍至少一个Unicode scalar。
- 所有block/count/inspection/entry limits都incremental执行；第8193条entry在其`entry` marker一被判定就产生start-earlier rank150，不读取同一行payload后再用末端overflow替代它。在更早source point越界后不得继续到更晚malformed closer并用rank120覆盖。block无recursion/nesting/macro/substitution/secondary lex。
- 同namespace允许多个blocks；每个block与entry保持source order，但block没有identity、whole-object value、precedence、permission或transaction semantics。
- identical raw entries都保留为两个lexical occurrences；D2不dedupe/last-win。D4以后可基于其own grammar返回duplicate semantic entry diagnostic。
- unknown namespace/provider仍可split block、preserve exact raw entries、copy/export。D2 parse不依赖registry。
- exact top-level line`[weftext-attributes]`是唯一合法opener。任何column-0 top-level logical line以reserved stem`[weftext-attributes`开头但不与exact opener相等（例如`[weftext-attributes ]`、`[weftext-attributes-extra]`、缺`]`）都是`invalid_attribute_carrier_structure@120`，range为完整malformed logical line；它不能降级成ordinary body。protected literal/source raw text例外且永远不重新识别。
- malformed namespace/entry prefix/closer fail closed；entry payload内部unknown不属于D2 syntax error。
- `[weftext-attributes]`是全局reserved top-level opener，因此不存在与普通`[source,LANG]` language token的同形冲突。作者要展示该syntax时，必须放进现有literal/source protected block；protected raw text永远不被重新识别。top-level body中late opener仍是`misplaced_attribute_carrier`。

### 4.3.1 Protected-payload wrong-closer preservation

literal、source、quote、saved query与saved view protected payload的active delimiter分别由其已识别opener确定。在尚未到达active closer前，遇到另外一种recognized protected delimiter logical line（集合`....`、`____`、`----`中不等于active delimiter者）立即生成`invalid_document_structure` candidate；range覆盖该offending logical line的完整content，不含line break。若随后还有active closer，该更早wrong closer仍然使Document invalid；foreign delimiter绝不因后续active closer而变成inert raw。只有在整个payload中既无wrong closer candidate、又无active closer时，才在EOF生成zero-width missing-closer candidate。

Body evaluator不得用“先找active closer，再检查其前内容”的control flow裁决语义。对每个已识别protected block或table，它必须在bounded body interval内收集可确定的active closer、recognized wrong closer、table non-row与missing-closer candidates，并按全局`(source start, rank)`规则裁决。该规则不增加delimiter、nesting、escape、family、rank或产品wire字段。

### 4.4 Body block grammar

所有结构 marker 必须从 column 0 开始；不支持 tab/space indentation 改变结构。非 protected body 中 column 0 的 line comment 是全局 trivia：它终止当前 paragraph 和 list run，像 blank line 一样不生成 element；在 table 中不生成 row；在 anchor 与目标 block 之间允许并保持附着；在 protected block 内只是 raw text。`[source,LANG]` 与紧随的 `----` 之间不允许 comment 或 blank，因为 delimiter 必须是下一逻辑行。

封闭 block allowlist：

| kind | source shape | children/content |
| --- | --- | --- |
| section | heading line == 到 ======，marker 后恰一个 space和非空 title | heading + ordered nested BodyBlock；title 使用与 paragraph 相同的 Inline lexer；level 从 1 开始且只能同级、上移或增加 1 |
| paragraph | 一组连续、未匹配其他 block 的非空行 | Inline*；逻辑行以一个 space连接 |
| unordered_list | 一组连续的一个或多个 * 加一个 space的行 | ListItem*；marker 数是 nesting level，不能跳级 |
| ordered_list | 一组连续的一个或多个 . 加一个 space的行 | ListItem*；marker 数是 nesting level，不能跳级 |
| table | 独占行 \|=== 开始和结束 | TableRow*；中间每个非 comment/nonblank 行必须以 \| 开始 |
| literal_block | 独占行 .... 开始和结束 | raw text；完全 inert，不解析 inline/anchor/macro |
| source_block | 独占行 [source,LANG] 后紧接 ----，以 ---- 结束 | language + raw text；LANG 匹配 [a-z0-9][a-z0-9-]{0,31}；内容 inert |
| quote_block | 独占行 ____ 开始和结束 | raw text；完全 inert |
| thematic_break | 独占行 ''' | 无 children |
| resource_block | 独占 wf-resource::TOKEN[OPTIONS] | 一个 ResourceOccurrence |
| bibliography_placement | 独占 wf-bibliography::[] | 无 children |
| saved_query_view_definition | 独占 [weftext-query] 或 [weftext-view]，后接 .... payload block | payload 是 inert UTF-8；D7 解释但不创建独立 identity |

table 内非 blank/noncomment 且不以 `|` 开始的 logical line 唯一产生 `invalid_document_structure`。`[weftext-query]` / `[weftext-view]` 后下一 logical line不是 `....`，或 EOF 前没有该 required immediate opener，也唯一产生 `invalid_document_structure`。

Anchor line只允许精确 [[anchor]]，anchor 匹配 [a-z][a-z0-9-]{0,63}。它必须紧邻并附着到下一 addressable block，中间只能有 comment；dangling、多 anchor 或 duplicate 都无效。

ListItem 的文本用 Inline*。unordered marker 后若紧接 [ ] 或 [x] 再加一个 space，分别投影 checked=false/true；其他 item 的 checked=null。list item 只能包含其 inline content和按 marker level派生的 nested list，不支持 continuation block。

List stack 使用唯一算法：首项 depth 必须为 1；depth 每次最多增加 1，增加时新 list 成为紧邻前一 ListItem 的 child，ordered/unordered kind 可不同；同 depth kind 改变时关闭当前 list 并打开相邻 sibling list；depth 降低时逐层 pop，到达目标 depth 后若 kind 不同，同样关闭并打开 sibling。blank 或 comment 清空 stack。因而 `* a` 后的 `.. b` 是 a 下的 ordered child，而文档首行 `.. b` 是 `invalid_document_structure`。

Table row 是一条以 | 开始的逻辑行；每个未转义 | 开始新 cell，`\|` 表示 cell 内 literal pipe，`\\` 表示 literal backslash。cell 只解析 Inline*；不支持 span、cell block、attribute、header option或嵌套 table。blank/comment 不创建 row。

合法且已识别的 opener 若缺少/错用 closer，或 source delimiter 没有立即跟随，是 `invalid_document_structure`；recognized wrong-closer 必须遵守 §4.3.1；未知或词法 malformed 的 reserved opener 是 `unsupported_document_feature`。section/list 跳级、dangling anchor 和 body 中 column 0 的 `= TITLE` 分别是 `invalid_document_structure`、`invalid_document_structure`、`duplicate_document_title`，不存在“二选一” family。

### 4.5 Inline grammar 与禁止项

Inline lexer 只产生以下 kind：

| kind | source shape | 语义 |
| --- | --- | --- |
| text | 不属于下列 token 的 UTF-8 text | inert text |
| link | link:HTTPS_TARGET[LABEL] | inert HTTPS navigation intent；parse/render 不 fetch |
| resource_occurrence | wf-resource:TOKEN[OPTIONS] | owner-local ResourceRef occurrence |
| node_link | wf-node:TOKEN[LABEL] | Node target intent；D3 冻结 token |
| citation | wf-cite:node/TOKEN[] | same-Workspace Node bibliographic target |

TOKEN 匹配 [A-Za-z0-9._~-]+。HTTPS_TARGET 匹配 https:// 后至少一个 ASCII URI 字符 [A-Za-z0-9._~:/?#@!$&'()*+,;=%-]，不得含 whitespace、[ 或 ]。LABEL 是不含未转义 `]` 的 inert text。

Escape 使用一次 left-to-right pass，且在 delimiter 识别前执行：反斜杠只可吞掉当前上下文允许的下一字符，被吞字符作为 literal 且永不重新参与 delimiter 或 macro 识别；其他反斜杠是相应 token malformed。LABEL 与 alt/caption 允许 `\]`、`\;`、`\\`；table cell 允许 `\|`、`\\`。因此 `\\|` 先得到一个 literal backslash，随后的 `|` 仍是 cell delimiter；不存在递归 unescape。

Resource OPTIONS 是以未转义分号分隔的固定 key=value 列表；key 只允许 role、alt、caption、page、width、height，且不得重复。role 只允许 attachment 或 image；page/width/height 是正十进制整数；alt/caption 使用 LABEL escape。任何 options 未给 role 时默认 attachment；未给 alt/caption/page/width/height 时各自为 null。空 options 因而投影为 role=attachment、其余全 null；显式 `alt=` 或 `caption=` 投影为空字符串而不是 null。未知 key、空 numeric 或 malformed option 是 invalid_resource_occurrence。

非 protected body 中：

- 任何 {name} 形状的 attribute reference 都是 unsupported_document_feature；正文 attribute interpolation 完全禁用。
- column 0 的 :name: value 是 unsupported_document_feature，不改变 header。
- column 0 的 `= TITLE` 是 duplicate_document_title，不作为 paragraph text。
- 任何其他 ASCII identifier 开头并形成 name:target[...] 或 name::target[...] 的 macro-shaped token 都是 unsupported_document_feature。由此 include、image、pass 和 extension macro 全部 fail closed。
- ifdef、ifndef、ifeval、endif 等 directive line、任意 passthrough delimiter/construct、block attribute line、leveloffset 和 executable extension 均不在 allowlist。
- 不在 allowlist 的 inline markup不会被二次解释；如果不形成 reserved macro/attribute/structural token，则只是 text。需要展示任意原始 syntax 时使用 literal/source block。

未知构造没有“实现定义”或 permissive fallback：reserved 形状一律拒绝，其余文字按 text 处理。

### 4.6 文档语义映射

| source/事实 | owner/projection | body content | durable entity |
| --- | --- | --- | --- |
| title line | DocumentMetadata.title | 否 | 否 |
| subtitle | DocumentMetadata.subtitle | 否 | 否 |
| `wf-kind` | Node classification coreKind | 否 | 否 |
| `wf-facets` token | Node classification facetMembership | 否 | 否 |
| 其他合法 header attribute | DocumentMetadata.headerAttributes | 否 | 否；D4 可派生 typed view |
| AttributeCarrierBlock/LexicalAttributeEntry | Document lexical projection | 否 | 否；只有 current-revision range |
| Node identity/parent/sibling ordinal | Node control state | 否 | Node 组成 |
| section heading/paragraph/list/item/table/row/cell | DocumentElement occurrence | 是 | 否 |
| saved Query/View definition | DocumentElement occurrence | 是 | 否；无 ViewRef |
| Resource bytes/derived facts | Resource | 否 | 是，非 Node |
| Resource caption/alt/page/size | Resource occurrence | 是 | 否 |
| Citation/BibliographyPlacement | inline/block occurrence | 是 | 否 |
| Annotation/thread/body | Node-owned Annotation | 否 | 是，非 Node |
| Query result/render/cache/index/validity attestation | derived state | 否 | 否 |

Document body 可以为空。bibliography 是 body placement 加 derived entries，不是 metadata。subtitle 只有显式 header source；title 中的 colon 始终是 inert title text。carrier block/entry 不进入 BodyBlock union，也不成为可独立提交、引用、标注或授权的对象。

### 4.7 Non-normative D4 stress prototype

下例只证明outer carrier能承载独立entries，不冻结任何inner spelling：

```adoc
[weftext-attributes]
....
namespace people
entry {"id":"example-a","field":"name","value":"张三"}
entry {"id":"example-b","field":"phone","value":"+86...","note":"工作号码"}
....
```

`id`、`field`、JSON、`value`、`note`全部是D4压力示意。D2 wire v2只看见两条raw entry strings与ranges。D4必须以后证明独立canonical fields、stable entry addressing、note/source、typed value与unknown-provider round-trip；没有D4 Gate前这些名字没有兼容承诺。

## 5. 最小稳定 wire v2

本节完整冻结 wire v2；不存在对 Research companion、旧 wire v1 或省略字段/default 的规范依赖。

### 5.1 closed-world rules

- 所有可独立传输的D2对象都必须含`wireVersion:2`与closed`kind`。未知version/kind、known kind的unknown/missing field、JSON duplicate key、非法null、重复logical-set identity全部decoder reject；没有ignore/last-win/default/coercion。
- 非顶层`document_payload`不带wireVersion/kind/owner，且只能完整嵌在`node_snapshot.document`或`document_snapshot.document`。
- nullable字段必须显式写null；nonnullable不能省略。数组只有明确写ordered才可观察顺序；logical set的array order不可观察且duplicate identity invalid。
- D3拥有NodeRef/ResourceRef/AnnotationRef与locator token编码；D6拥有byte handle、physical storage、strict UTF-8 decode/repair、revision/CAS。只有D6 strict decode成功才构造D2 `document_payload`；malformed bytes没有D2 payload。D2只冻结其已解码source及D3/D6对象在D2 wire中的typed slots、owner/cardinality与closed union。
- `sourceRevision`、exact-source digest/hash、filesystem path、mtime、cache/index key、`headerClassificationCandidate`与`documentValidityAttestation`不在D2 wire。需要artifact/evidence binding时由D3/D6 envelope绑定整个D2 exact source。

### 5.2 shared scalar objects

#### 5.2.1 source_range

闭对象字段恰为：

```json
{
  "startLine": 0,
  "startColumn": 0,
  "endLine": 0,
  "endColumn": 1
}
```

- line/column都是nonnegative JSON integers。
- 0-based logical line，column以Unicode scalar计数，end exclusive。
- 只有CRLF、CR、LF划分logical lines且不进入column；CRLF是一个indivisible EOL并必须在lone CR之前匹配。U+0085、U+2028、U+2029、VT、FF等其他separator只是当前logical line内的普通scalar。BOM若存在是source中的Unicode scalar，按实际位置计数。Parser不normalize source。
- range owner隐式为containing `document_payload.source`。range不能脱离该payload、owner Node与其D3/D6 revision binding作为durable address传输。
- start不得在end之后；end不得超出exact source。zero-width insertion point合法。

#### 5.2.2 diagnostic

闭对象字段恰为`family,orderRank,sourceRange`：

```json
{
  "family": "missing_document_title",
  "orderRank": 10,
  "sourceRange": {
    "startLine": 0,
    "startColumn": 0,
    "endLine": 0,
    "endColumn": 0
  }
}
```

family必须来自§6.1 closed diagnostic table；orderRank必须是该family唯一rank。不能由surface改rank/range。`invalid_utf8`不在该表中，任何D2 decoder/producer都不得输出它。

limit与header primary ranges额外闭合如下；它们仍只使用上面的`source_range`对象，不新增byte-offset public field：

- parser只接收D6已strict-decoded source，按Unicode scalar与CRLF/CR/LF indivisible sequence及其确定性valid UTF-8重编码长度推进。header-prefix byte overflow在首个sequence若被包含就会使prefix大于65,536 bytes时，返回该sequence之前的zero-width point；绝不指向scalar编码内部或CRLF中间。
- wrong continuation、overlong、surrogate、U+10FFFF以上与truncated physical bytes由D6在构造payload前拒绝；它们没有D2 `source_range`、family或orderRank。byte0 BOM若成功decode则是source scalar，不被normalize；title recognition可消费它但wire source完整保留它。
- 第257条header logical line仅在其完整logical line仍位于byte bound内时覆盖该行完整content；若该行先跨byte bound，则只产生前述zero-width byte-overflow range。
- carrier discovery inspection-budget或entry physical-line byte overflow同样返回首个crossing scalar/EOL sequence之前的zero-width point。第33个block opener覆盖完整logical line；第8193条entry覆盖固定`entry` marker span，故count 本决议可在不读取同一行unbounded payload时胜过later overflow。
- malformed top-level reserved-stem opener覆盖完整logical line；`wf-facets` extra space/tab覆盖实际一个malformed scalar；attribute-shaped invalid name只覆盖两个colon之间的name substring。
- header/carrier phase必须先生成所有**complete**可判定本决议 ranges，再按range start、随后唯一rank选择primary。crossing logical line的truncated prefix不是complete structural 本决议。更晚的rank30/120不能覆盖更早的template/task conflict或carrier limit。
- inherited body/inline/ref families are closed as `invalid_document_structure@200`, `unsupported_document_feature@210`, `duplicate_document_anchor@220`, `invalid_resource_occurrence@230`, `invalid_node_link@240`, `invalid_citation_target@250`, and `unresolved_bibliographic_target@260`; each rejects with unavailable projection and `d2CommitEligibility=reject`. A recognized foreign protected delimiter before the active closer is `invalid_document_structure@200` over the offending full logical line even if an active closer follows; only absence of both wrong closer and active closer uses EOF zero-width. A table non-row line likewise beats a later missing-closer EOF candidate. `invalid_node_specialization` is retired: legacy `wf-specialization` is `unknown_node_control_attribute@60`.

#### 5.2.3 raw_header_attribute

闭对象字段恰为`name,value,sourceRange,nameRange,valueRange`。name/value是exact decoded substrings，ordered array按source order输出。`wf-kind`与`wf-facets`不进入该array。

```json
{
  "name": "owner",
  "value": "team-a",
  "sourceRange": {"startLine":2,"startColumn":0,"endLine":2,"endColumn":14},
  "nameRange": {"startLine":2,"startColumn":1,"endLine":2,"endColumn":6},
  "valueRange": {"startLine":2,"startColumn":8,"endLine":2,"endColumn":14}
}
```

### 5.3 document_payload closed union

字段恰为`source,parse,projection,d2CommitEligibility,repairVisibility`。

#### 5.3.1 exact source authority

`source`是D6 strict UTF-8 decode成功后构造的authoritative exact Document string，包括BOM scalar（如有）、title、header、attribute carriers、body、comments、spaces与原line breaks。JSON transport escaping不改变解码后的scalar sequence；确定性UTF-8重编码恢复同一valid byte sequence。D2 projection、classification与diagnostics全部可由该source重建；它们不成为第二authority。若D6 decode失败，此对象不存在，D6 byte-envelope/repair结果也不得伪装成invalid D2 branch。

#### 5.3.2 valid branch

valid分支固定：

```json
{
  "source": "<exact UTF-8 source>",
  "parse": {"status":"valid","diagnostics":[]},
  "projection": {
    "state":"available",
    "metadata":"<document_metadata>",
    "attributeCarrierBlocks":[],
    "body":{"kind":"document_body","children":[]}
  },
  "d2CommitEligibility":"eligible",
  "repairVisibility":"not_required"
}
```

- `parse`字段恰为status/diagnostics；valid diagnostics恰为空array。
- projection字段恰为state/metadata/attributeCarrierBlocks/body。
- d2CommitEligibility只能eligible；repairVisibility只能not_required。

#### 5.3.3 invalid branch

invalid分支固定：

```json
{
  "source": "broken exact source",
  "parse": {
    "status":"invalid",
    "diagnostics":[{
      "family":"missing_document_title",
      "orderRank":10,
      "sourceRange":{"startLine":0,"startColumn":0,"endLine":0,"endColumn":0}
    }]
  },
  "projection":{"state":"unavailable"},
  "d2CommitEligibility":"reject",
  "repairVisibility":"exact_source_only"
}
```

- diagnostics恰有一个primary。
- projection字段恰为state，无metadata/carrier/body/oldcache/partial tree。
- d2CommitEligibility只能reject；repairVisibility只能exact_source_only。
- 对已成功decode但D2 syntax invalid的source，exact source始终可见给已获授权的repair surface；D1/D6可以拒绝整个snapshot访问，但不能在已交付invalid D2 payload中用old/partial projection替代。malformed physical bytes仍停留在D6 repair surface，不产生本分支。

#### 5.3.4 impossible combinations

下列decoder/producer invalid：valid+nonempty diagnostics、valid+unavailable projection、invalid+available projection、invalid+eligible、valid+exact_source_only、invalid+not_required、projection unavailable仍携带metadata/body/carrier。

### 5.4 classification projection

`node_classification`是closed tagged union：

available字段恰为`state,coreKind,facetMemberships`：

```json
{
  "state":"available",
  "coreKind":"ordinary",
  "facetMemberships":[
    {
      "facetId":"tasks/task",
      "sourceRange":{"startLine":1,"startColumn":12,"endLine":1,"endColumn":22}
    }
  ]
}
```

- coreKind只能ordinary/template。
- facetMembership闭对象字段恰为facetId/sourceRange。
- facetMemberships按source order传输；semantic membership是logical set，duplicateFacetId invalid。
- facetId、namespaceToken与`tasks/task` predicate都按exact ASCII code point比较；decoder/producer不得case-fold、Unicode-normalize、trim或percent-decode它们。

unavailable字段恰为`state`：

```json
{"state":"unavailable"}
```

Document full parse invalid时classification必须unavailable；不能从旧source/cache推断ordinary/Task/Template。只有同一owner Node、同一exact-source revision/digest、同一D2 profile/parser contract的full-parse-valid proof存在时，classification才可available。proof missing、stale、不匹配或invalid一律unavailable。

100k bounded header/control scan的internal `headerClassificationCandidate`不是本union的available branch。它不可进入wire、不可单独证明Document valid、不可直接驱动Task/Template public predicate或commit。实现可以保存revision/hash/profile-bound、可丢弃/可重建、non-authoritative `documentValidityAttestation` cache；其内部closed shape恰为`ownerNodeRef`、`sourceRevisionToken`、`sourceSha256`、`profileId`、`fullParseStatus`五字段，且只接受`fullParseStatus=valid`，缺失/extra/non-valid/mismatch均unavailable。header-only scanner消费调用方预先绑定的digest，不读取或hash carrier/body。attestation只证明同一exact source已full-parse valid，不保存替代source、classification、projection或第二classification authority。warm exact-match可与当前header candidate一起发布available；cold/missing必须unavailable并可触发full parse；stale必须拒绝；rebuild只从current exact source重算。

两个header完全相同的sources不能只凭header得到相同public result：`= T\n:wf-facets: tasks/task\n\nBody.\n`在current full-parse-valid proof后available；把body换为`:name: value\n`会得到`unsupported_document_feature@210`、Document invalid、classification unavailable。任何同header优化都必须保留该区分。

在完整`node_snapshot`中，`document.parse.status=valid`本身表示producer已完成current full parse，因此classification必须available；`document.parse.status=invalid`时必须unavailable。header-only cold path不得伪造完整`node_snapshot`或`document_payload`，也不得把“尚未full parse”编码成D2 syntax invalid。

Task不是classification kind。Task predicate是available facet set contains exact`tasks/task`。Template是coreKind=template且本决议 grammar已禁止同Node含tasks/task。

### 5.5 metadata 与 lexical carrier projection

#### 5.5.1 document_metadata

字段恰为`title,subtitle,headerAttributes`。

- title闭对象字段恰为`value,sourceRange`。
- subtitle为null或相同`{value,sourceRange}`闭对象。
- headerAttributes是ordered`raw_header_attribute`array。

#### 5.5.2 attribute_carrier_block

闭对象字段恰为`kind,namespaceToken,sourceRange,namespaceRange,entries`：

```json
{
  "kind":"attribute_carrier_block",
  "namespaceToken":"people",
  "sourceRange":{"startLine":3,"startColumn":0,"endLine":7,"endColumn":4},
  "namespaceRange":{"startLine":5,"startColumn":10,"endLine":5,"endColumn":16},
  "entries":[{
    "kind":"lexical_attribute_entry",
    "rawEntrySource":"{\"future\":true}",
    "sourceRange":{"startLine":6,"startColumn":6,"endLine":6,"endColumn":21}
  }]
}
```

- blocks/entries均按source order；同namespace可多blocks，没有block identity/precedence。
- entry字段恰为kind/rawEntrySource/sourceRange。range只覆盖`entry `后的opaque payload，不含prefix或line break。
- rawEntrySource不被D2解释；不能额外输出FieldId、typedvalue、entryId/note/source、Record、permission、merge或Annotation target。
- identical rawEntrySource合法并输出两个occurrences/ranges；D2不dedupe。
- `attributeCarrierBlocks`只在完整discovery与所有carrier limits均valid时输出。semantic discovery prefix从header physical end-exclusive开始，到首个ordinary body line start或EOF结束，包含leading/inter-block/trailing blank/comment trivia与所有EOL；其识别使用本决议冻结的1MiB inspection budget（ordinary body的首个判定byte结束discovery，无法在budget内判定则fail-closed）。entry-line limits按增量检查；wire不携带prefix byte length或test-local byte spans，但mechanical manifest必须记录actual carrier/entry access及upper bound。

### 5.6 full valid node_snapshot example

示例exact source：

```adoc
= Ship release
:wf-facets: tasks/task

[weftext-attributes]
....
namespace tasks
entry {"future":true}
....

Done.
```

完整snapshot：

```json
{
  "wireVersion":2,
  "kind":"node_snapshot",
  "nodeRef":"<D3:NodeRef>",
  "parentRef":null,
  "siblingOrdinal":null,
  "classification":{
    "state":"available",
    "coreKind":"ordinary",
    "facetMemberships":[{
      "facetId":"tasks/task",
      "sourceRange":{"startLine":1,"startColumn":12,"endLine":1,"endColumn":22}
    }]
  },
  "document":{
    "source":"= Ship release\n:wf-facets: tasks/task\n\n[weftext-attributes]\n....\nnamespace tasks\nentry {\"future\":true}\n....\n\nDone.\n",
    "parse":{"status":"valid","diagnostics":[]},
    "projection":{
      "state":"available",
      "metadata":{
        "title":{
          "value":"Ship release",
          "sourceRange":{"startLine":0,"startColumn":2,"endLine":0,"endColumn":14}
        },
        "subtitle":null,
        "headerAttributes":[]
      },
      "attributeCarrierBlocks":[{
        "kind":"attribute_carrier_block",
        "namespaceToken":"tasks",
        "sourceRange":{"startLine":3,"startColumn":0,"endLine":7,"endColumn":4},
        "namespaceRange":{"startLine":5,"startColumn":10,"endLine":5,"endColumn":15},
        "entries":[{
          "kind":"lexical_attribute_entry",
          "rawEntrySource":"{\"future\":true}",
          "sourceRange":{"startLine":6,"startColumn":6,"endLine":6,"endColumn":21}
        }]
      }],
      "body":{
        "kind":"document_body",
        "children":[{
          "kind":"paragraph",
          "locatorToken":"<D3:Locator>",
          "anchor":null,
          "inlines":[{"kind":"text","text":"Done."}]
        }]
      }
    },
    "d2CommitEligibility":"eligible",
    "repairVisibility":"not_required"
  },
  "resources":[],
  "annotations":[]
}
```

`node_snapshot`字段恰为wireVersion/kind/nodeRef/parentRef/siblingOrdinal/classification/document/resources/annotations。root parentRef/siblingOrdinal都null；nonroot都nonnull且ordinal为nonnegativeinteger，D3 workspace integrity rules另行约束。

resources是按resourceRef identity的logical set；annotations按annotationRef identity的logical set；array order不可观察。它们不因Document invalid自动消失。

### 5.7 full invalid snapshots

invalid `document_snapshot`：

```json
{
  "wireVersion":2,
  "kind":"document_snapshot",
  "ownerNodeRef":"<D3:NodeRef>",
  "document":{
    "source":"---\n_weftext:\n---\n",
    "parse":{
      "status":"invalid",
      "diagnostics":[{
        "family":"missing_document_title",
        "orderRank":10,
        "sourceRange":{"startLine":0,"startColumn":0,"endLine":0,"endColumn":0}
      }]
    },
    "projection":{"state":"unavailable"},
    "d2CommitEligibility":"reject",
    "repairVisibility":"exact_source_only"
  }
}
```

invalid `node_snapshot`同样完整携带identity/tree/resources/annotations，但classification unavailable且document invalid：

```json
{
  "wireVersion":2,
  "kind":"node_snapshot",
  "nodeRef":"<D3:NodeRef>",
  "parentRef":null,
  "siblingOrdinal":null,
  "classification":{"state":"unavailable"},
  "document":{
    "source":"broken exact source",
    "parse":{
      "status":"invalid",
      "diagnostics":[{
        "family":"missing_document_title",
        "orderRank":10,
        "sourceRange":{"startLine":0,"startColumn":0,"endLine":0,"endColumn":0}
      }]
    },
    "projection":{"state":"unavailable"},
    "d2CommitEligibility":"reject",
    "repairVisibility":"exact_source_only"
  },
  "resources":[],
  "annotations":[]
}
```

### 5.8 BodyBlock/Inline closed unions

D2 v2的body algebra在本文内完整列出；attribute carrier是metadata prefix，不进入BodyBlock union，也不能被body locator冒充。

BodyBlock closed kinds/fields：

| kind | exact fields |
| --- | --- |
| section | kind,locatorToken,anchor,heading,children |
| paragraph | kind,locatorToken,anchor,inlines |
| unordered_list | kind,locatorToken,anchor,items |
| ordered_list | kind,locatorToken,anchor,items |
| table | kind,locatorToken,anchor,rows |
| literal_block | kind,locatorToken,anchor,text |
| source_block | kind,locatorToken,anchor,language,text |
| quote_block | kind,locatorToken,anchor,text |
| thematic_break | kind,locatorToken,anchor |
| resource_block | kind,locatorToken,anchor,occurrence |
| bibliography_placement | kind,locatorToken,anchor |
| saved_query_view_definition | kind,locatorToken,anchor,definitionKind,payload |

Heading字段恰为`kind,locatorToken,level,inlines`；kind=heading，level为1..5，inlines ordered。ListItem字段恰为`kind,locatorToken,checked,inlines,nestedLists`；kind=list_item，checked只能null/false/true，nestedLists只含ordered_list/unordered_list。TableRow字段恰为`kind,locatorToken,cells`；TableCell字段恰为`kind,locatorToken,inlines`。

可地址document element kinds封闭为section/heading/paragraph/unordered_list/ordered_list/list_item/table/table_row/table_cell/literal_block/source_block/quote_block/thematic_break/resource_block/bibliography_placement/saved_query_view_definition。Inline无locatorToken；inline/citation annotation只能用containing flow的document_range。

Inline closed union：

| kind | exact fields/constraints |
| --- | --- |
| text | kind,text |
| link | kind,httpsTarget,label |
| resource_occurrence | 本文§9完整ResourceOccurrence fields |
| node_link | kind,target,label；target恰为`{kind:"node",targetToken:string}` |
| citation | kind,target；target恰为`{kind:"node",targetToken:string}` |

reserved `[weftext-attributes]`出现在literal/source protected raw text时不重新lex；top-level body late occurrence不是source_block，而是D2 framing diagnostic。

Protected payload delimiter语义不扩充BodyBlock union：`....`、`____`、`----`只是现有literal/source/quote/saved query/view grammar的recognized delimiters。已知opener确定active delimiter；在active closer前出现另一recognized delimiter时，producer必须返回`invalid_document_structure@200`，sourceRange为该foreign delimiter完整logical line。不得先跳到后续active closer并把foreign line序列化进`text`/`payload`。只有无foreign delimiter且无active closer时，sourceRange才是EOF zero-width。Table在active `|===`前的首个non-row logical line按相同`(start,rank)`原则胜过EOF missing closer。

### 5.9 Resource snapshots

`resource_snapshot`字段恰为wireVersion/kind/resourceRef/ownerNodeRef/byteEnvelope/derived：

```json
{
  "wireVersion":2,
  "kind":"resource_snapshot",
  "resourceRef":"<D3:OwnerLocalResourceRef>",
  "ownerNodeRef":"<D3:NodeRef>",
  "byteEnvelope":{"kind":"d6_byte_handle","handleToken":"<D6:ByteHandle>"},
  "derived":{
    "length":null,
    "digest":null,
    "mediaType":null,
    "width":null,
    "height":null
  }
}
```

ResourceOccurrence字段恰为kind/ownerNodeRef/resourceRef/presentation：

```json
{
  "kind":"resource_occurrence",
  "ownerNodeRef":"<D3:NodeRef>",
  "resourceRef":"<D3:OwnerLocalResourceRef>",
  "presentation":{
    "role":"attachment",
    "alt":null,
    "caption":null,
    "page":null,
    "width":null,
    "height":null
  }
}
```

presentation字段恰为role/alt/caption/page/width/height；role只能attachment/image；page/width/height为null或positiveinteger，alt/caption为null或string。resourceRef必须解析到ownerNodeRef。Resource bytes authority与occurrence presentation不合并。

### 5.10 Annotation snapshots and target union

`annotation`顶层字段恰为wireVersion/kind/annotationRef/ownerNodeRef/purpose/replyToAnnotationRef/target/body/suggestion/targetStatus。

- purpose只能comment/mark/suggestion。
- body恰为`{format:"plain_text",text:string}`；不解释HTML/AsciiDoc/macro/ref。
- replyToAnnotationRef为null或same-owner existing AnnotationRef；reply graph必须acyclic。
- targetStatus只能resolved/stale；后续无法解析时对象保留为stale，accept/replay failclosed。
- comment/mark的suggestion必须null。
- suggestion purpose的target必须document_range；suggestion恰为`{kind:"replace_plain_text",expectedDocumentRevisionToken:"<D6:DocumentRevision>",replacementText:string}`。空replacementText合法。revision/token不匹配时source不变。

AnnotationTarget v2保持五成员closed union，不因attribute carrier新增target：

| kind | exact additional fields |
| --- | --- |
| document | none |
| document_element | locatorToken |
| document_range | startLocatorToken,endLocatorToken |
| resource | resourceRef |
| resource_region | resourceRef,regionLocatorToken |

不存在`attribute_entry`/`field_occurrence`target。若D4以后提出cross-revision durable target，必须正式重开D2/D3 target union；不能把raw sourceRange或namespace/value猜成target。

### 5.11 document_snapshot/node_collection_result

- `document_snapshot`字段恰为wireVersion/kind/ownerNodeRef/document；document是完整document_payload。
- `node_collection_result`字段恰为wireVersion/kind/members；members是ordered、无重复NodeRef array，order由D7 query决定。无collectionRef/ViewRef/member snapshot。
- decoder不接受RecordRef/record_collection_result进入D2 Node/Document APIs；D5保留独立entrypoint选择。

### 5.12 decoder and operation errors

| family | condition | result |
| --- | --- | --- |
| `unsupported_wire_version` | wireVersion不是2 | decoder reject |
| `unsupported_wire_kind` | tag完全不在v2 kinds | decoder reject |
| `domain_kind_mismatch` | known valid kind置于错误union/domain或ref/locator coercion | decoder/operation reject |
| `invalid_wire_object` | unknown/missing field、duplicate JSON key、illegal null/type/range、impossible valid-invalid combination | decoder reject |
| `duplicate_wire_identity` | logical set出现duplicate ref/FacetId | decoder reject |
| `unmanaged_not_addressable` | UnmanagedItem送入Node/Resource/Annotation API | operation reject |
| `cross_owner_resource_reference` | resource owner mismatch | operation reject |

D2 document syntax diagnostics与wire decoder errors不混用。UTF-8 physical decoding/repair、total byte envelope、I/O、permission、revision、transaction、network/path是D6 envelope；D2 parser-local control/carrier limits仍产生本决议定义的 D2 family。D6 decode failure的结果不是`document_payload{parse.status:"invalid"}`。

### 5.13 commit and downstream gates

`d2CommitEligibility`只来自本文D2 parse/profile/projection。最终commit仍是：

```text
D2 eligibility
AND applicable D4 semantic gate
AND applicable D6 authority/transaction gate
AND applicable D7 payload gate
```

downstream reject不能把D2-invalid变valid、不能返回old/partialprojection、不能另存typed carrier为第二source。unknown provider可以让D2 lexical projection valid，但D4/D10 typed capability unavailable/reject；rawsource仍唯一authority。

## 6. Diagnostics、spans、repair 与 commit gate

### 6.1 Document parse matrix

D2 v2仍是`valid|invalid`；invalid只有一个primary，projection unavailable、new commit rejected、exactsource repair visible。unknown provider或D4 semantic invalid不能冒充D2 parse family。

phase为header→carrier→body；每个phase先生成该phase所有可判定candidate ranges，再取source start最早，同start取rank最小。不能因parser control flow把更晚error提前返回。只有先前已有合法`wf-kind: template`时，`tasks/task` token完成才立即生成`template_task_facet_conflict` candidate；facets-before-kind不会回溯生成conflict，后续kind固定为`misplaced_node_control_attribute@70`。新/替换families：

| rank | family | exact span |
| ---: | --- | --- |
| 10 | `missing_document_title` | required title insertion point |
| 20 | `duplicate_document_title` | second complete document-title logical line |
| 30 | `invalid_document_header` | complete malformed header logical line |
| 40 | `invalid_attribute_name` | complete attribute-shaped line中两个colon之间的name substring |
| 50 | `duplicate_header_attribute` | second complete attribute logical line |
| 55 | `control_header_limit_exceeded` | byte overflow：首个会使65,536-byte prefix越界的完整UTF-8 scalar或CRLF/CR/LF sequence之前的zero-width point；line overflow：若第257条header logical line在byte bound内完整结束，覆盖该行完整content range；若该行自身先跨byte bound，使用byte-overflow point |
| 60 | `unknown_node_control_attribute` | reserved `wf-*` name |
| 70 | `misplaced_node_control_attribute` | complete late control line |
| 80 | `invalid_core_node_kind` | value或empty insertion point |
| 90 | `invalid_facet_membership_list` | first malformed token；extra ASCII space/tab覆盖该实际单个space/tab scalar，missing token使用required insertion point |
| 100 | `duplicate_facet_membership` | second duplicate token |
| 110 | `template_task_facet_conflict` | `tasks/task` token |
| 120 | `invalid_attribute_carrier_structure` | wrong/missing opener/closer/entry prefix；reserved-stem malformed opener覆盖完整logical line；absence at insertion point |
| 130 | `misplaced_attribute_carrier` | late opener complete line |
| 140 | `invalid_attribute_namespace_token` | namespace token或missing insertion point |
| 150 | `attribute_carrier_limit_exceeded` | discovery-inspection/entry byte overflow用first crossing scalar/EOL sequence之前zero-width point；第33个block opener覆盖完整logical line；第8193条entry覆盖其固定`entry` marker span（无需读取其unbounded payload） |
| 200 | `invalid_document_structure` | structural marker；dangling/multiple/错误附着anchor、recognized wrong closer或table non-row覆盖offending完整logical line；missing closer使用EOF zero-width；required delimiter absence使用规范插入点 |
| 210 | `unsupported_document_feature` | 完整reserved directive/block line或inline occurrence |
| 220 | `duplicate_document_anchor` | 第二个duplicate anchor完整logical line |
| 230 | `invalid_resource_occurrence` | grammar/options malformed覆盖完整occurrence；owner-local ResourceRef无法解析只覆盖TOKEN |
| 240 | `invalid_node_link` | grammar malformed覆盖完整occurrence；same-Workspace Node intent无法解析只覆盖TOKEN |
| 250 | `invalid_citation_target` | malformed/错误target kind覆盖完整occurrence；foreign target intent只覆盖TOKEN |
| 260 | `unresolved_bibliographic_target` | citation TOKEN |


继承的 body/inline/ref family 没有外部规范依赖；其候选生成与唯一 range 在本文内闭合如下：

| family | 唯一 sourceRange |
| --- | --- |
| `duplicate_document_title@20` | body 中第二个 column-0 `= TITLE` 的完整 logical line |
| `invalid_document_structure@200` | 跳级覆盖 marker token；dangling/multiple/错误附着 anchor、recognized wrong closer、table non-row 覆盖 offending完整logical line；missing closer覆盖EOF zero-width；source/saved-definition opener后缺 required immediate delimiter覆盖下一逻辑行column 0 zero-width或EOF；其他presence violation覆盖最早使grammar失败的完整logical line，absence violation覆盖规范插入点 |
| `unsupported_document_feature@210` | 完整 reserved directive/block line 或完整 inline reserved occurrence；unclosed inline occurrence到logical line末尾 |
| `duplicate_document_anchor@220` | 第二次 anchor 的完整logical line |
| `invalid_resource_occurrence@230` | grammar/options malformed覆盖完整`wf-resource` occurrence；合法occurrence的ResourceRef无法owner-local resolve只覆盖TOKEN |
| `invalid_node_link@240` | grammar malformed覆盖完整occurrence；合法token无法same-Workspace resolve只覆盖TOKEN |
| `invalid_citation_target@250` | malformed/错误target kind覆盖完整occurrence；合法shape但target intent非same-Workspace Node只覆盖TOKEN |
| `unresolved_bibliographic_target@260` | citation TOKEN |

所有候选 range 先于 primary 选择生成；logical line span不含line break，token/occurrence span从introducer首scalar到闭合delimiter后一个scalar，缺闭合则到logical line末尾；zero-width insertion point的start=end。recognized foreign protected delimiter即使后面存在active closer也保持更早的`invalid_document_structure@200`；只有没有wrong closer且没有active closer时才使用EOF zero-width。`invalid_utf8`不是D2 family，也没有D2 rank。

repair：

- 不自动改source，不返回old/partial projection，不从YAML/sidecar/legacyheader恢复。
- repair preview可删第二facet token、删除unknowncontrol、移动latecarrier、补closer；确认后仍expected revision+完整reparse。
- D2 generic repair不得改写inner entry payload、生成FieldId/entryId、mergevalues或attachnote；这些属于后续owner。
- lexical-valid unknown namespace/facet不是repair error；source继续可见。

header attribute先按attribute-shaped line识别，再验证name；例如`:Topic: x`固定为`invalid_attribute_name@40`并只覆盖`Topic`，不得因valid-name regex未命中而降为rank 30。Logical-line scanner只认CRLF/CR/LF；任何其他Unicode separator不得改变line number、phase或primary family。

byte-bound crossing logical line在其CRLF/CR/LF或complete scalar被完全判定前不是complete line，不能产生rank 20/30/40/50、control/facet或body structural candidate；只有crossing zero-width limit candidate参与arbitration。`invalid_node_specialization`已retire，不再是合法diagnostic family；legacy `wf-specialization`按`unknown_node_control_attribute@60`处理。

### 6.2 Annotation 与 operation envelope

Annotation 不改变 Document parse status。`targetStatus`只有resolved或stale：合法且可解析为resolved；已提交对象因后续revision无法解析时保留为stale且accept/replay reject；新提交的body/target/owner/purpose/payload/reply非法时commit reject且不产生portable object。

Annotation family封闭为`invalid_annotation_body`、`invalid_annotation_target`、`invalid_annotation_purpose`、`invalid_annotation_payload`、`invalid_annotation_reply`、`stale_annotation_target`；Resource owner mismatch使用`cross_owner_resource_reference`。新Annotation exactly-one fail-fast顺序固定为purpose → target/owner → body → purpose-specific payload → reply owner/acyclic；target/owner内部先验证closed target shape，再把structurally-valid cross-owner Resource单独映射为`cross_owner_resource_reference`。

Operation/query/decoder family：

| family | 条件 | 结果 |
| --- | --- | --- |
| `unmanaged_not_addressable` | 将UnmanagedItem当Node/Resource/Annotation | operation reject |
| `domain_kind_mismatch` | 已知合法D2 kind/ref/locator进入错误union/domain或发生coercion | decoder/operation/query reject |
| `cross_owner_resource_reference` | ResourceRef owner mismatch | operation reject |
| `unsupported_wire_version` | wireVersion不是2 | decoder reject |
| `unsupported_wire_kind` | tag完全不在wire v2 kinds | decoder reject |
| `invalid_wire_object` | closed object字段/type/null/range/duplicate JSON key/impossible combination非法 | decoder reject |
| `duplicate_wire_identity` | logical set出现duplicate ref或FacetId | decoder reject |

UTF-8 physical decode、总byte envelope、I/O、permission、revision、transaction、network与physical path属于D6，不是D2 document diagnostic。

### 6.3 Downstream gate composition

`d2CommitEligibility`只由本文D2 parse/profile/projection产生。最终authoritative commit gate为`d2CommitEligibility AND applicableD4Gate AND applicableD6Gate AND applicableD7Gate`；未适用gate视为pass，各下游错误使用自己的namespace/envelope。

下游reject不能把D2-invalid变valid、不能改变tree/metadata/classification、不能返回partial/旧projection，也不能把typed carrier、cache、sidecar或provider database变为第二作者权威。已存外部损坏由D6 repair envelope暴露exact source；feature可以unavailable，但不能制造另一份D2 projection。

## 7. Task、Template、external edit 与 export

### 7.1 portable owner 与 disable

- `tasks/task`的minimal semantic marker随Core runtime/protocol分发；Tasks UI/module可禁用，但parser、facet predicate、NodeRef、raw carrier与genericread/export仍成立。
- UI disabled时Tasks-specificform/view/reminder/promotion入口返回D1closed unavailable reason，不删除或降级facet。
- unknown future taskfacet/version不被猜成built-inTask；sourcepreserve、typed capability failclosed。

### 7.2 Assign/Remove Task

- ordinary Node→Task是显式Assign Facet Action：preview冲突与D4-required initial facts，expected revision，atomic commit；NodeRef保持。
- Task→ordinary Node是Remove Facet Action。D2只移除membership；是否保留/cleanup Tasks-owned entries由D4 action plan明确，不能silent delete。
- 同名raw header attribute或看起来像task的body不能启发式promote。

### 7.3 checklist toggle/promote

- toggle只修改checklist occurrence token，不创建或查找Task Node。
- promote preflight绑定owner NodeRef、expected source revision、exact occurrence locator、write capability、target placement与完整D4/D6gate，preflight不分配identity。
- successful commit创建fresh ordinary NodeRef、source中声明`tasks/task`、写caller/D4 plan给出的initial content，并把原checkbox occurrence替换为ordinary Node link；没有mirror。
- 任一失败零source mutation、零allocation、零receipt。两个并发promote最多一个成功；失败方stale/retry。

### 7.4 composition/query/VTODO

- Task+Project+Calendar是同一unordered facet set；owner facts以后由canonicalD4fields保存一次，consumer只derived projection。
- D7若提供`tasks`selector，normative result仍是Node rows filtered byfacet；没有TaskRef或Task entity collection。
- VTODO import显式preview后创建fresh ordinary Node+`tasks/task`并使用D3OriginBinding；UID/RECURRENCE-ID/SEQUENCE不是Node identity。field mapping/loss归D4/D9。

### 7.5 Template independence

- `wf-kind: template`是Core meta-kind；Template Node禁止同时声明`tasks/task`。
- Task Template target plan另行声明targetCoreKind=ordinary与targetfacets含`tasks/task`；Template自身不因此成为Task。
- instance获得freshNodeRef并遵循D3ref/slotrewrite；target business fields由D4/D9定义。

### 7.6 External edit、operations 与 export

| operation | identity | D2 v2 source rule |
| --- | --- | --- |
| move/rename/reorder same Workspace | NodeRef preserve | exact bytes/control/carrier unchanged |
| whole-Node copy/fork | fresh NodeRef | exactsource payload可复制；D3 slot/owner rewrite另行验证 |
| canonical import | fresh NodeRef | v2 bytes完整validate，无legacyfallback |
| ordinary AsciiDoc import | fresh NodeRef | 无control时ordinary；不得猜Task/Template/facets |
| canonical export | artifact binding | retain exact bytes/hash/control/carrier |
| plain/content-only export | new unmanaged artifact | explicit loss report；不能回写为same canonical Node |

普通AsciiDoc renderer会把reserved style附着到protected literal block并显示其raw payload；canonical source仍是一份合法AsciiDoc。plain exporter可以：

- retain block as visible literal并声明typed semantics unavailable；或
- strip blocks/control，loss report列namespace tokens、rawentry counts与strippedfacet/kind。

两者都是新artifact，不是canonical save。external formatter改变bytes会形成新revision；valid则新projection，invalid则repair，无sidecar/oldprojection fallback。

D6以后拥有permission、semantic partial merge、CAS/read-set/receipt；D2只要求任何committed result是一份完整valid exact source，失败无partial mutation。

## 8. Security、100k 与 five-surface parity

- `control/header prefix`精确定义为byte 0起，到第一个header-terminating blank logical line的physical end-exclusive；若Document在header结束即EOF，则到最后一个header logical line的physical end。它包括leading blank/comment trivia、title、header lines、所有CRLF/CR/LF bytes与terminating blank，admission limit为65,536 UTF-8 bytes；恰好N合法，N+1拒绝。
- D2 bounded header scanner只接收D6已strict-decoded source，并按decoded logical-line/scalar边界及其确定性UTF-8重编码长度推进：先把CRLF作为一个sequence，再认lone CR/LF，其余每次一个Unicode scalar。为区分offset 65,535处的lone CR与跨界CRLF，并完成最多4-byte valid scalar sentinel，header candidate scan从重编码byte 0最多inspect 65,540 bytes；sentinel仅用于D2 limit decision，不得把header界外内容lex或interpret为body。若下一个valid sequence的end-exclusive会大于65,536，scanner登记该sequence start的zero-width rank 55，再与已确定header candidates按`(start,rank)`裁决。
- malformed UTF-8 lead/continuation、overlong、surrogate、U+10FFFF以上与truncated bytes在D6 strict decode阶段拒绝并进入D6 byte-envelope/repair evidence；不会构造`document_payload.source`，也不会进入D2 scanner、D2 arbitration或D2 public scalar-column range。BOM若由D6成功decode则是source byte0对应的一个scalar；D2 title recognition可消费它但不normalize、删除或另存。
- 因此对两个D6-valid sources，前65,536 UTF-8 bytes相同且末byte为CR时仍可区别：下一byte为LF时，CRLF作为整体跨界并在CR前报rank55；下一byte非LF或EOF时，CR是N内完整EOL。valid multibyte scalar跨界同理，range只能落在scalar之前。
- title之后、terminating blank之前最多256 header logical lines（comment与attribute都计数）；因每个attribute恰占一行，这也限制最多256 attributes。第257行若在byte limit内完整结束，rank 55覆盖其完整content；若该行在完整结束前先触发byte overflow，则bounded byte point是唯一rank 55 range，scanner不追到unbounded line terminator。
- 其他control limits：1..32facets；namespace-id/facet-name分别63 bytes；FacetId 127 bytes；32个最大FacetId加31个SP的facet list semantic maximum是4,095 bytes（N合法）；任何4,096-byte list必已违反token/count grammar并在first malformed token报rank 90，不保留不可达的“4,096 valid”声明。
- attribute-carrier discovery inspection budget为1,048,576 bytes、最多32blocks/8192entries、每条physical entry line最多65,536 bytes；inspection与entry byte scans各最多多access4-byte sentinel并incremental fail closed。linear scan，无recursion/JSON/YAML/alias/tag/merge/substitution/extension。
- 100k header/control path只运行上述post-D6 header scanner；每Node最多inspect 65,540 re-encoded valid UTF-8 bytes，不读carrier/body、不加载provider或sidecar。其输出必须严格命名为internal `headerClassificationCandidate`，只是从同一exact-source revision的header推导出的`coreKind/facetMemberships`候选，**不是**public `node_classification`、不是validity proof、不能直接序列化、不能使commit eligible，也不进入D3 Terminology Lexicon或public wire。
- public `node_classification.available`只可在同一owner Node、同一exact-source revision/digest、同一D2 profile/parser contract已有full-parse-valid proof时由该header candidate发布。proof missing、stale、digest/revision/profile不匹配或full parse invalid时一律`node_classification.unavailable`；不得借用旧revision的available result。最小同header pair必须区分：`= T\n:wf-facets: tasks/task\n\nBody.\n` full-parse valid后available Task；同header而body为`:name: value\n`时`unsupported_document_feature@210`、Document invalid、classification unavailable。
- 允许一个revision/hash-bound、可丢弃/可重建、non-authoritative `documentValidityAttestation` cache保存“该exact revision已由该profile完成full parse且valid”的事实。该内部fact的closed shape恰为`ownerNodeRef`、`sourceRevisionToken`、`sourceSha256`、`profileId`、`fullParseStatus`五字段，且可接受值要求`fullParseStatus=valid`；字段缺失、unknown extra field、非valid状态、owner/revision/digest/profile任一不匹配均只得unavailable。header-only调用方预先绑定digest，scanner自身不得读取或hash carrier/body。attestation不得保存替代source、classification或projection，不得改变parse结果、不得成为第二authoritative index；cold/missing返回unavailable并可排队full parse，warm exact-match可避免重读body，stale必须拒绝，rebuild必须只从current exact source重算。D6 physical decode/byte envelope必须先成功，但不进入D2 family/rank表。
- Desktop、CLI、Server/WebUI、Mobileoffline共享同parser、limits、family/rank/ranges与wirev2。UI capability可以unavailable，但source meaning不变。
- server permission filtering在任何D4/D7typed projection前执行；D2diagnostic不能泄露inner payload beyond caller-authorized repair surface。


## 9. 替代方案与从零裁决

### 9.1 Node control placement

| option | best property | decisive burden/counterexample | disposition |
| --- | --- | --- | --- |
| A current header | 一种AsciiDoc grammar、one-file、普通工具可读、100k prefix scan最简单 | 需为set token、第二duplicate span、unknown provider与reserved prefix给出闭合规则 | **A+ accepted** |
| B closed `_weftext` YAML | control/domain层级清楚、native list | 新增YAML lexical/security与lossless editing surface；byte-0/title、comments/order/quotes、plain AsciiDoc、truncation、locator/hash/copy全部扩大 | reject；未证明不可约能力 |
| C sidecar/control plane | content/control物理分域 | 只复制`.adoc`会丢classification；missing pair、非原子后端、backup/sync/fork/import/Mobile均依赖第二payload；D6尚未冻结 | reject；portable one-file失败 |
| D comment preamble | 比YAML简单且视觉分层，普通renderer忽略 | current comments是trivia；formatters可删除；要求每个ordinary Node携带空preamble增加噪音；仍需一套第二subgrammar而没有超过reserved native attributes的能力 | reject but retain as reviewed alternative |

A+的layer separation不是靠第二文件语法，而是：

- 唯一reserved names是`wf-kind`、`wf-facets`；任何其他`wf-` attribute拒绝。
- control lines必须位于普通header attributes之前，固定相对顺序`wf-kind`→`wf-facets`。
- ordinary raw header attributes不因同名被provider认领，也不自动成为typed field。
- legacy `wf-specialization`、YAML与sidecar不提供compatibility read、precedence或fallback。

### 9.2 Task B1/B2

| question | B1 Core Task specialization | B2 ordinary Node + built-in facet |
| --- | --- | --- |
| identity/lifecycle | Task仍复用NodeRef | 相同；只复用NodeRef |
| composition | Task例外分类 + future facets两套机制 | Task/Project/Calendar为同一unordered facet set |
| portable semantics | Core直接硬编码 | `tasks/task` contract随Core分发，UI可禁用 |
| query/wire | 容易产生Task subset/kind | Node facet predicate；无Task wire union |
| checklist promote | fresh Task-specialized Node | fresh ordinary Node + task facet；no mirror不变 |
| irreducible difference | 未发现独立owner/identity/lifecycle | 不需要把业务capability升级为Core entity kind |

选择B2。它不蕴含YAML B；六格cross-product必须逐格证明。

### 9.3 persisted attributes

| option | D2 can freeze safely | pressure deferred to later owner | disposition |
| --- | --- | --- | --- |
| flat raw header | title/header grammar与raw values | repeatable structured entry、note、stable target、typed validation仍无闭合 | 保留为generic metadata，不作为canonical typed carrier |
| UI-only grouping | 不改source/authority | 只改善UI，不解决source organization | allowed UX later, not persisted decision |
| single profile blob | outer attribute可解析 | whole-object conflict/permission/migration/query blast radius | reject strong counterexample |
| namespace-scoped carrier | closed block placement、lexical entries/ranges、raw round-trip | FieldId/value/entry ID/note/permission/merge/target分别由D4/D5/D6/D3 | **accepted within D2 boundary** |


## 10. 下游最小稳定接口与非回退边界

### 10.1 Later-owner closure requirements

本决议保留以下mandatory scenarios，但只作后续owner Gate输入：

- D4：canonicalFieldId/namespace owner/anti-spoof、typed value、repeatable entry key、note/source/provenance、unknownprovider semantic validation、flat↔carrier explicit mapping。
- D4/D10 Facet registry与anti-spoof Gate必须把frozen built-in `tasks/task`的`tasks` namespace保留给Core，禁止third-party/user在该namespace masquerade；`wf`与`core`也不得由third-party/user claim，后续owner须裁决它们是Core-usable还是永久禁用。除`tasks/task`外，本决议不冻结registry spellings，lexically-valid unknown FacetId仍按D2 raw-preserved。
- D4/D10必须显式裁决carrier `namespaceToken`与FacetId namespace是共享registry、closed mapping还是完全分离；仅因字符串相同不得推断semantic owner/equality。Gate需覆盖collision、anti-spoof、provider removal与migration，不得引入隐式fallback。
- D5：repeatable entry是否只是value occurrence或取得Record domain；不得暗建identity。
- D6：per-field permission、concurrent merge、delete-vs-edit、entryreorder、CAS/receipt；不得whole-namespace blob比较。
- D3：只有D4选择跨revision stable entry target或Annotation target时，才新增closed locator/target/rewrite/ABA amendment；D2 lexical range本身不承诺continuity。
- D7/D8/D9：Query/API canonical IDs、UI grouping/labels、import/exportmapping与loss。

两个相同phone、不同notesource；A改name/B加phone；provider removal；copy/reorder/delete；same`status/date`跨namespace等场景必须被这些owners重放，但不被本文伪装成已冻结wire。

### 10.2 D1/D3 non-regression

#### D1

`D1 reopen required=no`：五表面、Core唯一提交、Mobileoffline同parser、bundledTasks作为可禁用capability均保持。若实现让provider直写、某表面另解source或sidecar成为authority，立即停止并重开D1 boundary。

#### D3

`D3 amendment required=yes, impact-scoped`：

1. 绑定本文件最终D2 digest、outer grammar evolution、对应D3 wireVersion与golden corpus。
2. 在D3 non-durable Document occurrence inventory中显式加入`AttributeCarrierBlock`与`LexicalAttributeEntry`：二者只有owner Document/current-revision range，无EntityRef、独立lifecycle、cross-revision continuity、locator token或Annotation target。
3. 重证exact-source payload/hash、newcontrol/carrier spans、copy/fork/import/export artifact binding；carrier/entry随authoritative source被hash/copy，但不成为新payload authority或symbolic ref slot。
4. 把Task specialization依赖改为ordinaryNode + built-in`tasks/task`facet：checklist promotion、VTODO mapping、copy/import classification与fresh NodeRef/no-mirror不变量保持。
5. D3 Terminology Lexicon与negative Gate必须新增`Facet`、`FacetId`、`attribute carrier block`、`lexical attribute entry`，绑定owned names `facetId`、`facetMemberships`、`attributeCarrierBlocks`、`namespaceToken`、`rawEntrySource`；重写`weftext.term.node`、`weftext.term.task`与`Promote`，删除`NodeSpecialization::Task`和“frozen header specialization派生Task”的controlled meanings。
6. 重证Template meta-kind与Task Template target plan的freshidentity/refrewrite边界。
7. **不在本amendment自动新增field occurrence identity/Annotation target**；只有D4以后选择跨revisiondurable target时再提出独立closed amendment。

不重开WorkspaceRef、NodeRef、ResourceRef、AnnotationRef本体；move/rename/trash/restore/tombstone一般lifecycle；foreignidentity一般代数；SavedQuery/View与Record边界，除非D3 task出现具体新反例。

### 10.3 D3–D10 owner matrix

| 下游 | D2强制输入 | 仍未决 |
| --- | --- | --- |
| D3 | Node durable；Document由Node地址；carrier/entry non-durable；Resource/Annotation owner-local；Task=ordinary Node+`tasks/task`；Template=Core meta-kind；wire v2 | final D2 digest、对应D3 wireVersion/goldens、copy/move/delete/recovery、locator与七切片amendment |
| D4 | title/subtitle/raw header、Facet lexical memberships、carrier raw entries来自唯一source；D2不解释inner payload | FieldId、typed value、schema、Task facts、relation、entry key/note/source/provenance、Facet registry/validation |
| D5 | table row/cell与carrier entry均非Record；Record不得冒充Node/Document或复用NodeRef | 独立Record domain与表格UX |
| D6 | exact source唯一；strict decode前置；parse矩阵固定；invalid无partial projection；Resource/Annotation唯一owner/source | storage、transaction、permission、revision、sync、byte envelope、merge |
| D7 | NodeCollectionResult无identity；saved Query/View仅Document occurrence；Task selector是Node facet predicate | payload、evaluation、sort/page、Action evidence |
| D8 | UI分离Node tree、Document tree、classification、collection与unmanaged inventory | editor/IME/a11y/performance |
| D9 | Template是Core meta-kind；Task Template使用target plan；source只来自Document/Resource | slots/parameters/instantiation/import/export/worker |
| D10 | provider/package/Agent/automation不是D2 object；unknown lexical IDs raw-preserved | registry、capability、approval、audit、credential |

## 11. 实现影响与退役闭环

最终受控切换后的实现目标为：Core profile parser/control/carrier/wirev2/diagnostics、Task predicate/promotion；storage exactsource payload与artifact binding；five-surface source/repair/facet actions；controlled retirement of `wf-specialization: task|template`、Task wire kind/TaskRef、YAML/sidecar fallback与single-profile blob。

本节只描述冻结后的未来实现目标，不授权修改`repos/weftext`。

Retirement traceability matrix：

| retirement target | replacement | negative fixture/deletion assertion |
| --- | --- | --- |
| mixed YAML/header/sidecar control authority | A+ reserved `wf-kind`/`wf-facets` + one exact source | YAML/sidecar/body-attr无compat decoder或precedence |
| legacy `wf-specialization: task|template` | `coreKind=template`或ordinary+Facet set | legacy name固定rank60；无specialization wire/selector |
| Task wire kind/TaskRef/checklist mirror | NodeRef + exact `tasks/task` Facet + explicit promotion | toggle无TaskRef；promotion无mirror；NodeRef保持/新建规则唯一 |
| hidden Template inventory/ancestor suppression | `coreKind=template`; templates仍是nodes | nodes selector无Template排除分支 |
| single-profile/whole-namespace blob | independent lexical entries；D4-owned typed mapping | 无whole-object last-win/permission/migration入口 |
| durable Block/field occurrence identity | current-revision occurrence/range only | locator/entry送EntityRef或AnnotationTarget decoder拒绝 |
| open AsciiDoc parser/include/body interpolation | closed Profile v2；no fetch/execute | unknown macro/include/pass/body attr fail closed |
| resource-global presentation/cross-owner ref | owner-local ResourceOccurrence | same resource多caption；cross-owner reject |
| AsciiDoc Annotation body | plain_text body | HTML/pass/macro保持inert |
| persistent NodeCollection/SavedView | result无identity；definition occurrence | ViewRef/collectionRef/workspace owner拒绝 |
| Record branches masquerading asD2 content | Document occurrences；D5独立domain | D2 decoder无Record branch或coercion |
| partial/old projection on invalid source | exact source + full unavailable | invalid无metadata/body/classification/carrier |
| unordered Node snapshot | siblingOrdinal | `[A,B]`与`[B,A]` wire不同 |
| wire v1 compatibility decoder | closed wire v2 only | version1/unknown/alias全部reject |

Future implementation retirement gate必须为每行提供旧API/type/fixture删除清单、replacement测试、negative fixture和全文search deletion assertion；新测试通过不能替代旧兼容分支删除。

## 12. 测试轮廓

Minimum test groups：

1. header/control positive/negative、duplicate/unknown/span/rank；D2 post-decode scanner必须覆盖LF、CR、CRLF各自N/N+1、CRLF恰在boundary split、valid BOM、valid 4-byte scalar straddle、unterminated long header line及每个zero-width wire range；crossing partial line不得生成结构candidate；line scanner必须证明U+0085/U+2028/U+2029/VT/FF不分行。wrong continuation、overlong、surrogate、U+10FFFF以上与truncated bytes另列为pre-D2/D6 boundary evidence：每例必须证明strict decode失败、D2 payload未构造、D2 runner未调用。
2. FacetId/carrier namespace共享lexical matrix：dotted namespace、internal hyphen、digits-after-first positive；empty dot segment、leading digit、leading/trailing/consecutive hyphen、facet-name dot negative；namespace 63/64、facet-name 63/64、FacetId 127/128、namespace-token 63/64及facet-list 4,095/4,096 exact bytes。
3. 256 header logical lines N/N+1，valid 256-attribute、257-comment；leading-trivia/title/terminator旁路；attribute-shaped invalid name→rank40/name span；kind-after-facets→rank70；template/task earlier than later BROKEN；malformed facet extra-space覆盖实际space scalar。
4. carrier framing/placement/raw roundtrip/unknown namespace/duplicate raw entries；1MiB discovery inspection budget的leading/inter-block/trailing trivia、多block accumulation与closer EOL N/N+1、long indented ordinary line budget failure、actual access/upper-bound instrumentation；entry physical line64KiB N/N+1（LF/CR/CRLF）、8193×N+1 count-vs-overflow collision；第33 block/8193 entry；malformed reserved opener variants；earlier prefix/entry crossing + later structural error。确认每个limit incremental、innerpayload不被D2解释。另对literal/source/quote/saved-query/view逐一覆盖foreign recognized delimiter后有active closer与无active closer：两者都在foreign delimiter完整logical line报rank200；只有完全无foreign delimiter且active closer缺失才报EOF。table non-row + missing closer必须由更早non-row line胜出。
5. six-cell `(A|B|C)×(B1|B2)`除了binding/oracle-shape checker，还必须有独立semantic evaluator从每个source/control/target-plan/capability artifact推导classification、illegal family、provider/version状态、artifact-count与promote invariants；不能读取`expected`字段来生成actual结果。Template/invalid combos/providerdisabled/copy/export均需semantic replay。
6. Task+Project+Calendar、toggle/concurrentpromote、VTODOOriginBinding、noTaskRef/no mirror。
7. move/copy/fork/import/export exact hashes与plainloss。
8. inherited body diagnostic table必须重放duplicate document title、invalid structure、unsupported feature、duplicate anchor、invalid resource occurrence、invalid Node link、invalid citation target、unresolved bibliography target；每例绑定family/rank/range/unavailable projection/commit reject。100k测试必须把header candidate与public classification分栏，并覆盖valid/invalid same-header pair、cold/missing attestation、warm exact-match、rebuild、stale revision/digest/profile拒绝；header-only instrumented maximum source access=65,540 bytes，public actual必须同时断言Document projection、commit eligibility与repair visibility。
9. five-surface golden parity与negative retirement scan。
10. Domain properties：每个Node恰一Document；root外parent+ordinal唯一连续；Resource/Annotation owner-local；Task predicate不产生新identity；D2 content graph无Record/View entity。
11. Body/inline：sections/list/checklist/table/protected blocks/query-view occurrence；wrong closer、non-row、missing delimiter、unknown macro/body attr/include/pass均按closed family/range；no fetch。
12. Resource/Annotation：presentation per occurrence、cross-owner reject；Annotation五成员target union、plain text、reply acyclic、stale与suggestion revision fail closed；不存在attribute-entry target。
13. Citation/Bibliography：same-Workspace Node target；placement删除不删目标；render numbering不回写；Record/string target reject。
14. Collections/Record：NodeCollectionResult无identity；saved definition只有NodeRef+locator；D2 decoder拒绝RecordRef，独立D5 domain不进入content tree。
15. Wire/gates：unknown version/kind/field、duplicate JSON key、illegal null、duplicate logical identity、known-wrong-union全部fail closed；D2/D4/D6/D7 gate AND不产生partial/旧projection。
16. Retirement：第11节逐项验证legacy specialization、wire v1、YAML/sidecar、TaskRef/mirror、whole-namespace blob和兼容decoder已删除。

## 13. 冻结证据、激活条件与后续边界

- D1 reopen：no。
- Fresh DeepSeek stage：`opencode-go/deepseek-v4-pro`，session `ses_facb7ed24ffeRVLbrHbuwDHBbQ`；P0=0，全部P1/P2已关闭；证据为`05 Research/D2 Replacement Review 2026-08-30/D2 Replacement DeepSeek Evidence and Adjudication v1.md`。
- Final replacement Gate：真正全新的GPT-5.6 Sol + Pro 5/5；URL （原评审会话地址不公开），标题`Review Sol Pro Gate`，UI elapsed `59m 55s`；package SHA-256 `6D476C610CDBCECC72F324DCB2E39F172FF883D628510E9446957F260E291535`。
- Final verdict：pass；P0=0、P1=0、blocking P2=0、terminology gate=pass、top-level material change=no。唯一nonblocking P2是brief中的replay script basename与唯一实际payload的`_r2`后缀不一致；按anti-churn规则不重跑。
- 本规范不单独激活。必须先生成并通过impact-scoped D3 amendment：绑定本文件最终SHA、wireVersion/goldens、carrier/entry non-durable inventory、exact-source/artifact binding、Task Facet、Lexicon/negative Gate、Template boundary，并明确不扩张field occurrence identity/AnnotationTarget。
- D3 amendment必须经过一份fresh DeepSeek V4 Pro与一份fresh Sol Pro；P0/P1=0后，由controller在单一受控世代中原子替换D2、D3、D3 Lexicon、D2/D3 Implementation Impact及全部索引。禁止D2新/D3旧的中间authority；rollback必须成对恢复旧D2/D3 bytes与索引。
- Gate与本规范只证明architecture closure，不证明产品实现或retirement已完成；本轮没有修改`repos/weftext`或`brand/`。
- 原子切换完成后仍停在D4 paused；没有用户新的明确恢复指令时，不创建或启动D4。
