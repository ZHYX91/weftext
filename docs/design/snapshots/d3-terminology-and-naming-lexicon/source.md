---
_weftext:
  id: "b2595435-72ce-423a-805a-09993b04d1d5"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

状态：D7 revision05联合replacement草稿，尚未接受或激活。基线接受记录仅属历史；必须以本次完整独立审查、总控验收和coordinated journal共同决定生效。稳定文档ID保持。


# D3 Terminology and Naming Lexicon — revision05


日期：2026-08-31。

## 1. 权威路径、目的与非目标

本文件是 D3 主决议的冻结 companion；[D3 主决议](../d3-identity-references-ownership-and-lifecycle/source.md)链接并约束其使用。历史聊天、旧代码名、模型措辞、Research候选和用户口语不能替代本冻结 Lexicon。

本 Lexicon 只覆盖 D3 identity/reference/ownership/lifecycle 需要的术语。它不处理 Profile、Extension、Calendar/Library 产品命名，也不冻结 D4 schema/关系类型、D5 Record ontology、D6 sync wire、D7 citation 展示/Library 语义、D8 UI 信息架构、D9 import payload 或 A2 产品包装。

## 2. 控制规则

1. 每个 concept 只有一个 stable concept ID 与一个 canonical 中英文 term pair；同一case-folded裸term或受控name跨所有surface只能指向一个concept。一个拼写可在同一concept的多个surface重复，但不能因surface不同而转移owner。
2. wire/API、code symbol/convention、CLI/UI label 与 locale key 可以不同，但必须在 entry 的`owned-names`结构化JSON中逐surface、双向、exact-set地一对一映射回同一concept ID。它们不是新的 ontology；entry prose对其他concept的引用、union member、排除边界或反例不转移名称所有权，也不进入owned set。`semantic-nonaliases`只表达“本concept不等于这些相邻concept/term”，必须携带expected concept或typed role才能裁决；它不进入flat token denylist，也不剥夺另一个concept对同形owned name的所有权。全局退役controlled identifier只由§6的`retired-controlled-identifiers`闭合集合定义。机械投影与registry只能作为可重建review evidence，不能反向改写Lexicon。
3. `frozen:D1|D2` entry 只能由显式 reopen 上游决议改变。冲突必须记录最小反例和 reopen request；D3 不得以 alias 或 code rename 绕过。
4. `candidate:D3` entry 只冻结身份/引用/生命周期含义。下游可以增加更窄 subtype，但不得复用同一 canonical term 表达不同概念。
5. 受控负向gate只扫描受控schema/wire identifier、public API/code symbol、CLI command/flag与locale key，并对每个命中保留surface与source span。分类顺序固定为：先以case-folded exact name解析`owned-names`到唯一concept；再检查§6全局`retired-controlled-identifiers`并给出其唯一replacement/deletion target；若调用点声明expected concept/typed role，则把实际owned concept与expected concept核对，concept-relative `semantic-nonaliases`只在此步产生`expected-concept-mismatch`；其余为unknown/error。合法owned name永不因出现在其他entry的semantic non-alias中被裸token拒绝。ordinary prose、用户内容、第三方format、历史evidence、migration/deletion note与closed typed counterexample不进入禁止面。独立product-conformance evidence必须证明Unicode/escaping/container等规避方式不会绕过受控surface，同时不得把证据载体、scanner实现、Unicode数据版本或宿主runtime提升为D3产品合同；依赖缺失、证据不完整或检查语义漂移一律fail closed。
6. 删除/迁移目标不是兼容承诺。Weftext 尚未发布，R0 实现直接删除退役 identifier；不加 alias parser、双读双写或 fallback。

Entry 字段顺序固定为：stable ID；正式中英文；定义；owner/layer；排除边界；wire/API；code convention；CLI/UI/locale；`owned-names`结构化JSON；简称；`semantic-nonaliases`结构化投影；人可读semantic non-alias投影；例子/反例；首次冻结与migration/deletion target。每个entry恰一`owned-names`、恰一`semantic-nonaliases`及恰一人可读投影；后两者必须按原始顺序及case-folded集合双向exact-equal。定义、排除、例子或相邻entry中的substring不得满足名称归属；本Lexicon entry与§6全局退役表是产品权威，任何machine allocation、registry或scanner都只是可丢弃、可重建的review evidence。

当前Lexicon由39个继承entry及本轮新增3个entry组成，共42个；旧独立证据只覆盖历史39个集合，本轮全部42个及其surface须由新的完整审查判断，不继承旧通过结论。当前必须证明entry与stable IDs精确相等；canonical中英文、owned wire/API name、code convention、locale key与canonical CLI/UI label逐concept一对一；所有owned name与全局retired set按case-folded exact比较不相交；每个retired identifier恰一replacement/deletion target；semantic non-alias只由expected concept/type mismatch裁决。删除、移动、协同重建或旧代次替换都fail closed。证据角色、文件名、schema、hash、运行时、Unicode数据、mutant与重放方式只属于Research/Agent Session和Design Review Execution Protocol，不由本Lexicon指定，也不能要求主决议逐文件链接这些工具。

## 3. D1/D2 frozen entries

### `weftext.term.workspace`

- 正式名：工作区 / Workspace。
- 定义：可移植聚合、授权、事务与 D3 identity namespace；不是作者内容 entity 或开放 owner。
- owner/layer：D2 domain boundary；D1 Core 是语义 authority。
- 排除：不等于目录、窗口、Server account、database、tenant 或 root Node。
- wire/API：`workspace_ref`, `WorkspaceRef`, `workspaceId`；code `WorkspaceId`, `WorkspaceRef`, variables `workspace_id|workspace_ref`。
- CLI/UI/locale：`workspace`, “工作区”, `term.workspace`; 简称 `WS` 仅允许内部图例，不用于 wire/public API。
- owned-names：`{"wireApiNames":["WorkspaceRef","workspaceId","workspace_ref"],"codeConventions":["WorkspaceId","WorkspaceRef","workspace_id","workspace_ref"],"localeKeys":["term.workspace"],"cliUiLabels":["workspace","工作区"]}`。
- semantic-nonaliases：`["project","vault","folder","tenant"]`。
- 语义非同义词（人可读投影）：`["project","vault","folder","tenant"]`。
- 例/反例：`WorkspaceRef(W)` 是 scope；`D:\notes` 不是 Workspace identity。
- 首次冻结/迁移：`frozen:D2`；R0 删除把 path/database row 当 Workspace identity 的 identifier。
### `weftext.term.node`

- 正式名：节点 / Node。
- 定义：D2 content graph 中唯一长期受管、可独立引用并恰有一份 Document 的作者内容 entity；Node exact-source classification可包含lexically valid Facet memberships，ordinary与template Node均可携带非`tasks/task` Facet；Task只由ordinary + exact built-in `tasks/task` predicate成立。
- owner/layer：D2 content graph；D3 定义 NodeRef/lifecycle。
- 排除：不等于 Document、heading/block/list item、folder、Record、Resource、Annotation、泛称 item 或 note。
- wire/API：`node_ref`, `NodeRef`, `nodeId`；code `Node`, `NodeId`, `NodeRef`, variables `node|node_id|node_ref`。
- CLI/UI/locale：`node`, “节点”, `term.node`; 简称无。
- owned-names：`{"wireApiNames":["NodeRef","nodeId","node_ref"],"codeConventions":["Node","NodeId","NodeRef","node","node_id","node_ref"],"localeKeys":["term.node"],"cliUiLabels":["node","节点"]}`。
- semantic-nonaliases：`["note","page","item","document","folder","data node"]`。
- 语义非同义词（人可读投影）：`["note","page","item","document","folder","data node"]`。
- 例/反例：ordinary + exact `tasks/task`满足Task predicate；template + `project/project`合法但不是Task；template + `tasks/task`固定冲突；paragraph不是Node；Facet membership不创建第二identity。
- 首次冻结/迁移：`frozen:D2`；R0 删除旧 `NoteId`, `ItemId`, path-node aliases。
### `weftext.term.document`

- 正式名：文档 / Document。
- 定义：owning Node 恰好拥有的一份 exact source 及其语义解释；以 owning NodeRef 寻址，没有第二 durable identity。
- owner/layer：D2 document content；D3只定义地址与 locator。
- 排除：不等于 Node、文件、DocumentElement、render、projection 或 revision token。
- wire/API：无 `DocumentRef|DocumentId`; address field必须是 `owner: NodeRef`；code `Document`, `DocumentProjection`, variables `document|document_owner`。
- CLI/UI/locale：“文档”, `term.document`; 无简称。
- owned-names：`{"wireApiNames":[],"codeConventions":["Document","DocumentProjection","document","document_owner"],"localeKeys":["term.document"],"cliUiLabels":["document","文档"]}`。
- semantic-nonaliases：`["file","node","page","blob","DocumentId","DocumentRef"]`。
- 语义非同义词（人可读投影）：`["file","node","page","blob","DocumentId","DocumentRef"]`。
- 例/反例：Node N 的 Document 地址是 N；正文 hash 不是 Document identity。
- 首次冻结/迁移：`frozen:D2`；R0 删除独立 document ID/sidecar identity。
### `weftext.term.occurrence`

- 正式名：出现项 / Occurrence。
- 定义：在一个明确 owner/execution/rule scope 内出现、可定位或可重建但默认没有 durable identity 的 manifestation 总称；只作元术语，不是 wire kind。
- owner/layer：D2/D3 shared meta-term。
- 排除：不等于 entity、Node、Record、row handle 或 stable ref；裸 occurrence 不替代更窄 term。
- wire/API：禁止 `kind:"occurrence"` 作为开放 union；code抽象仅用 `OccurrenceContext`，变量须加限定词。
- CLI/UI/locale：通常显示具体 subtype；`term.occurrence`; 简称无。
- owned-names：`{"wireApiNames":[],"codeConventions":["OccurrenceContext"],"localeKeys":["term.occurrence"],"cliUiLabels":["occurrence","出现项"]}`。
- semantic-nonaliases：`["item","instance","entity"]`。
- 语义非同义词（人可读投影）：`["item","instance","entity"]`。
- 例/反例：DocumentOccurrence 与 DerivedOccurrence 都无默认 Node identity，但来源不同。
- 首次冻结/迁移：`frozen:D2` 的 Document occurrence 基线 + `candidate:D3` 的总称收窄；R0 禁止裸 `OccurrenceId`。
### `weftext.term.document-occurrence`

- 正式名：文档出现项 / Document Occurrence。
- 定义：Document exact source 中的 heading、paragraph、list/checklist item、table row/cell、citation、bibliography placement、Saved Query/View definition、AttributeCarrierBlock与LexicalAttributeEntry等current-revision语法出现。
- owner/layer：D2 Document content。
- 排除：不等于 Node、Document、durable entity 或 ICS Derived Occurrence。
- wire/API：D2 closed element/inline kind + D3 locator；禁止 `DocumentOccurrenceRef` durable ref。code `DocumentElement`/具体 occurrence type，变量 `document_occurrence`。
- CLI/UI/locale：显示具体元素名，fallback “文档出现项”, `term.documentOccurrence`；简称无。
- owned-names：`{"wireApiNames":[],"codeConventions":["DocumentElement","document_occurrence"],"localeKeys":["term.documentOccurrence"],"cliUiLabels":["document occurrence","文档出现项"]}`。
- semantic-nonaliases：`["block entity","item entity","row record","note"]`。
- 语义非同义词（人可读投影）：`["block entity","item entity","row record","note"]`。
- 例/反例：checklist item 是 occurrence；显式 promotion 后的新 Task Node 是另一 entity。
- 首次冻结/迁移：`frozen:D2`；R0 删除 block/heading/row durable ID。
### `weftext.term.task`

- 正式名：任务 / Task。
- 定义：available Facet set包含exact built-in `tasks/task`的ordinary Node；复用该Node的identity、Document、owner与lifecycle，不形成第二kind或identity。
- owner/layer：D2 v2 built-in Facet predicate；D3只冻结NodeRef/lifecycle effect。
- 排除：不等于 checklist item occurrence、calendar VTODO、Record、TaskRef、Task wire kind或mirror。
- wire/API：仍为 `node_ref`; code `TaskNode`, variables `task_node`; predicate必须读取exact `facetMemberships[].facetId == "tasks/task"`。
- CLI/UI/locale：“任务节点”, `term.task`; 允许 UI 短称“任务”。
- owned-names：`{"wireApiNames":[],"codeConventions":["TaskNode","task_node"],"localeKeys":["term.task"],"cliUiLabels":["task","任务","任务节点"]}`。
- 简称：无。
- semantic-nonaliases：`["checklist item","todo row","VTODO identity","TaskRef","Task wire kind"]`。
- 语义非同义词（人可读投影）：`["checklist item","todo row","VTODO identity","TaskRef","Task wire kind"]`。
- 例/反例：checklist promotion creates a fresh ordinary Node with exact `tasks/task` and no mirror；勾选 checklist 不创建 Task identity；VTODO mapping不保留foreign identity。
- 首次冻结/迁移：`frozen:D2-v2`；R0 删除 checklist/task mirror、`TaskId|TaskRef`与旧Task specialization code。
### `weftext.term.facet`

- 正式名：分面 / Facet。
- 定义：Node exact-source classification中的可组合capability membership；membership可出现在ordinary或template Node上且不创建新的entity、owner、identity、Document或lifecycle。exact `tasks/task`禁止出现在template上，并且只使ordinary Node满足Task predicate。
- owner/layer：D2 v2 lexical membership与built-in `tasks/task` marker；D4/D10以后拥有registry、schema与anti-spoof规则。
- 排除：不等于 CoreNodeKind、tag、Relation、plugin instance、UI tab或Node subtype identity。
- wire/API：classification projection字段`facetMemberships`; code `Facet|FacetMembership`, variables `facet|facet_memberships`。
- CLI/UI/locale：`facet`, “分面”, `term.facet`; 具体Facet显示名由其owner定义。
- owned-names：`{"wireApiNames":["facetMemberships"],"codeConventions":["Facet","FacetMembership","facet","facet_memberships"],"localeKeys":["term.facet"],"cliUiLabels":["facet","分面"]}`。
- 简称：无。
- semantic-nonaliases：`["core kind","tag","relation","plugin","node subtype"]`。
- 语义非同义词（人可读投影）：`["core kind","tag","relation","plugin","node subtype"]`。
- 例/反例：template + `project/project`是合法non-task Facet membership；template + `tasks/task`固定`template_task_facet_conflict`；ordinary + `tasks/task`满足Task predicate；Facet自身不是Node或Task identity。
- 首次冻结/迁移：`frozen:D2-v2`；R0删除以specialization enum代替Facet membership的Task路径。
### `weftext.term.facet-id`

- 正式名：分面标识符 / FacetId。
- 定义：D2 v2 classification中按exact ASCII code point比较的namespace/name lexical token；它标识Facet contract但不是EntityRef或可解析content identity。
- owner/layer：D2 v2 lexical shape；D4/D10以后拥有registry、namespace reservation与validation。
- 排除：不等于 NodeId、FieldId、provider id、locale key、namespaceToken或bare UUID。
- wire/API：`facetId`; code `FacetId`, variables `facet_id`。
- CLI/UI/locale：开发/诊断显示“FacetId”, `term.facetId`; 用户表面通常显示Facet label。
- owned-names：`{"wireApiNames":["facetId"],"codeConventions":["FacetId","facet_id"],"localeKeys":["term.facetId"],"cliUiLabels":["FacetId","分面标识符"]}`。
- 简称：无。
- semantic-nonaliases：`["NodeId","FieldId","namespaceToken","provider id"]`。
- 语义非同义词（人可读投影）：`["NodeId","FieldId","namespaceToken","provider id"]`。
- 例/反例：exact `tasks/task`是built-in FacetId；相同namespaceToken不自动等于FacetId owner。
- 首次冻结/迁移：`frozen:D2-v2 lexical contract`；D4/D10另行冻结registry，不把FacetId加入EntityRef decoder。
### `weftext.term.attribute-carrier-block`

- 正式名：属性载体块 / Attribute Carrier Block。
- 定义：D2 v2 exact source中的namespace-scoped protected lexical block occurrence；只有owning Document与current-revision ranges，无durable identity或第二payload authority。
- owner/layer：D2 v2 Document lexical projection；D3只冻结non-durable identity与artifact-binding边界。
- 排除：不等于 Node、Record、Facet、Field container entity、sidecar、locator或Annotation target。
- wire/API：document projection字段`attributeCarrierBlocks`与成员`namespaceToken`; code `AttributeCarrierBlock`, variables `attribute_carrier_block|namespace_token`。
- CLI/UI/locale：repair/diagnostic surface显示“属性载体块”, `term.attributeCarrierBlock`; 普通用户表面可不直接显示。
- owned-names：`{"wireApiNames":["attributeCarrierBlocks","namespaceToken"],"codeConventions":["AttributeCarrierBlock","attribute_carrier_block","namespace_token"],"localeKeys":["term.attributeCarrierBlock"],"cliUiLabels":["attribute carrier block","属性载体块"]}`。
- 简称：无。
- semantic-nonaliases：`["field container","record","sidecar","locator","payload authority"]`。
- 语义非同义词（人可读投影）：`["field container","record","sidecar","locator","payload authority"]`。
- 例/反例：block随完整exact source hash/copy；相同namespaceToken的两个blocks仍是两个current-revision occurrences且没有BlockRef。
- 首次冻结/迁移：`frozen:D2-v2`；R0删除whole-namespace blob/sidecar authority，不新增carrier identity。
### `weftext.term.lexical-attribute-entry`

- 正式名：词法属性条目 / Lexical Attribute Entry。
- 定义：Attribute Carrier Block中的opaque raw lexical entry occurrence；只投影raw source与current-revision source range，不解释FieldId、typed value、entry identity或provenance。
- owner/layer：D2 v2 Document lexical projection；D4以后拥有inner payload semantics。
- 排除：不等于 Field、Record、Annotation target、reference slot、EntityRef、locator或cross-revision entry identity。
- wire/API：entry projection字段`rawEntrySource`; code `LexicalAttributeEntry`, variables `lexical_attribute_entry|raw_entry_source`。
- CLI/UI/locale：repair/diagnostic surface显示“词法属性条目”, `term.lexicalAttributeEntry`; 普通用户表面显示D4解释后的字段时仍不得冒充entry identity。
- owned-names：`{"wireApiNames":["rawEntrySource"],"codeConventions":["LexicalAttributeEntry","lexical_attribute_entry","raw_entry_source"],"localeKeys":["term.lexicalAttributeEntry"],"cliUiLabels":["lexical attribute entry","词法属性条目"]}`。
- 简称：无。
- semantic-nonaliases：`["Field","Record","entry id","reference slot","Annotation target"]`。
- 语义非同义词（人可读投影）：`["Field","Record","entry id","reference slot","Annotation target"]`。
- 例/反例：identical rawEntrySource可在同block出现两次且保留两个ranges；任何一次都没有EntryRef或跨revision continuity。
- 首次冻结/迁移：`frozen:D2-v2`；D4若未来需要durable field target必须正式reopen，不能复用本occurrence。
### `weftext.term.resource`

- 正式名：资源 / Resource。
- 定义：由一个 Node 严格拥有、具有 durable owner-local identity 与单一作者 byte source、但不拥有 Document/Node capability 的非节点对象。
- owner/layer：D2 content object；D3 owner-local ResourceRef/lifecycle。
- 排除：不等于 attachment UI role、filesystem file、path、blob digest、external URI、Document occurrence 或 Node。
- wire/API：`resource_ref`, `ResourceRef`, `resourceId`; code `Resource`, `ResourceId`, variables `resource|resource_ref`。
- CLI/UI/locale：canonical UI “资源”; 附着语境可显示“附件”但 locale key必须映射 `term.resource.attachmentRole` 而非新 kind；`term.resource`。
- owned-names：`{"wireApiNames":["ResourceRef","resourceId","resource_ref"],"codeConventions":["Resource","ResourceId","ResourceRef","resource","resource_ref"],"localeKeys":["term.resource","term.resource.attachmentRole"],"cliUiLabels":["resource","资源","附件"]}`。
- 简称：无。
- semantic-nonaliases：`["file","blob","asset","attachment"]`。
- 语义非同义词（人可读投影）：`["file","blob","asset","attachment"]`。
- 例/反例：同 Resource 可有多个 occurrence captions；`C:\a.pdf` 不是 ResourceRef。
- 首次冻结/迁移：`frozen:D2`；R0 删除 `AttachmentId`, `FileResourceId`, path-as-id。
### `weftext.term.annotation`

- 正式名：批注 / Annotation。
- 定义：由一个 Node 严格拥有、具有 owner-local identity、plain-text body 和 frozen target union 的非节点对象。
- owner/layer：D2 content object；D3 owner/ref/reply lifecycle。
- 排除：不等于 comment syntax occurrence、Document、Node、relation 或 external provider comment ID。
- wire/API：`annotation_ref`, `AnnotationRef`, `annotationId`; code `Annotation`, `AnnotationId`, variables `annotation|annotation_ref`。
- CLI/UI/locale：“批注”, `term.annotation`; 允许 UI purpose labels“评论/标记/建议”，不作为对象 alias。
- owned-names：`{"wireApiNames":["AnnotationRef","annotationId","annotation_ref"],"codeConventions":["Annotation","AnnotationId","AnnotationRef","annotation","annotation_ref"],"localeKeys":["term.annotation"],"cliUiLabels":["annotation","批注"]}`。
- semantic-nonaliases：`["comment object","note","thread","message"]`。
- 语义非同义词（人可读投影）：`["comment object","note","thread","message"]`。
- 例/反例：stale Annotation仍保留 identity；provider comment ID不是 AnnotationRef。
- 首次冻结/迁移：`frozen:D2`；R0 删除全局/cross-owner Annotation IDs。
### `weftext.term.node-link`

- 正式名：节点链接 / Node Link。
- 定义：Document 中 authored inline occurrence，其 target intent 是 same-Workspace Node；它是 reference slot，不是 relation entity。
- owner/layer：D2 authored occurrence；D3 target NodeRef/resolution。
- 排除：不等于 generic Reference、Relation、Citation、external link 或 Resource occurrence。
- wire/API：D2 `node_link` occurrence + D3 NodeRef target；code `NodeLinkOccurrence`, variables `node_link`。
- CLI/UI/locale：“节点链接”, `term.nodeLink`; 简称“链接”仅在 UI 已限定 Node 上下文。
- owned-names：`{"wireApiNames":["node_link"],"codeConventions":["NodeLinkOccurrence","node_link"],"localeKeys":["term.nodeLink"],"cliUiLabels":["node link","节点链接","链接"]}`。
- semantic-nonaliases：`["relation","citation","shortcut","symlink"]`。
- 语义非同义词（人可读投影）：`["relation","citation","shortcut","symlink"]`。
- 例/反例：A Document 的 node link 指向 B Node；不创建 LinkId。
- 首次冻结/迁移：`frozen:D2`；R0 删除 link-as-entity/path target。
### `weftext.term.citation`

- 正式名：引文 / Citation。
- 定义：Document 中 authored citation occurrence，target intent 是 same-Workspace Node；bibliography placement/render numbering不产生 identity。
- owner/layer：D2 authored occurrence；D3只冻结 target ref/locator identity；Library citation schema/semantics路由D4/D7。
- 排除：不等于 generic Reference、Node Link、BibliographyPlacement、external URI 或 Library record。
- wire/API：D2 `citation` occurrence + D3 NodeRef target；code `CitationOccurrence`, variables `citation_occurrence`。
- CLI/UI/locale：“引文”, `term.citation`; 禁止简称 `ref`。
- owned-names：`{"wireApiNames":["citation"],"codeConventions":["CitationOccurrence","citation_occurrence"],"localeKeys":["term.citation"],"cliUiLabels":["citation","引文"]}`。
- semantic-nonaliases：`["reference","bibliography entry","library item"]`。
- 语义非同义词（人可读投影）：`["reference","bibliography entry","library item"]`。
- 例/反例：删除 bibliography placement 不删除 target Node；citation没有 CitationRef。
- 首次冻结/迁移：`frozen:D2`；D4/D7负责 Library citation semantics，R0删除 citation entity ID。
## 4. D3 identity/reference entries

### `weftext.term.entity`

- 正式名：实体 / Entity。
- 定义：D3 中拥有 durable content identity 的 closed meta-union：Node、Resource、Annotation；仅作上位术语，不是第四种 kind。
- owner/layer：D3 identity algebra。
- 排除：Workspace、Document、occurrence、Record future domain、locator、evidence、operation ID。
- wire/API：`EntityRef = NodeRef | ResourceRef | AnnotationRef`; 禁止 wire `kind:"entity"`。code `EntityRef` enum，variables `entity_ref` only when union truly accepted。
- CLI/UI/locale：优先具体 kind；通用诊断“实体”, `term.entity`; 简称无。
- owned-names：`{"wireApiNames":[],"codeConventions":["EntityRef","entity_ref"],"localeKeys":["term.entity"],"cliUiLabels":["entity","实体"]}`。
- semantic-nonaliases：`["item","object","thing","record","node"]`。
- 语义非同义词（人可读投影）：`["item","object","thing","record","node"]`。
- 例/反例：Resource是entity member但不是Node；WorkspaceRef不属于EntityRef。
- 首次冻结/迁移：`candidate:D3`；R0删除开放 `ObjectRef`/`ItemRef` 和 entity-kind coercion。
### `weftext.term.authoritative-identity`

- 正式名：权威身份 / Authoritative Identity。
- 定义：Core 用于判等、解析与生命周期连续性的 durable typed identity；只由完整 Workspace/owner/domain-scoped ref 给出。
- owner/layer：D3 identity algebra。
- 排除：path、label、source span、locator、foreign key、digest、revision、provenance、provider ID。
- wire/API：具体 `WorkspaceRef|NodeRef|ResourceRef|AnnotationRef`; code `AuthoritativeRef`仅作closed trait/bound，变量优先具体 ref。
- CLI/UI/locale：不作为普通对象标签；诊断 locale `identity.authoritative`；简称“identity”只在D3章节。
- owned-names：`{"wireApiNames":[],"codeConventions":["AuthoritativeRef"],"localeKeys":["identity.authoritative"],"cliUiLabels":["authoritative identity","权威身份"]}`。
- semantic-nonaliases：`["ID","key","handle","address"]`。
- 语义非同义词（人可读投影）：`["ID","key","handle","address"]`。
- 例/反例：完整 NodeRef 是权威身份；UID/filename不是。
- 首次冻结/迁移：`candidate:D3`；R0禁止 bare UUID equality。
### `weftext.term.typed-reference`

- 正式名：类型化引用 / Typed Reference。
- 定义：closed kind 与全部 namespace/owner fields 组成的 authoritative identity value；`Ref` 是受控 code suffix，不是自然语言泛称。
- owner/layer：D3 wire/API。
- 排除：Reference occurrence/fact、Relation、Locator、handle、foreign key。
- wire/API：`WorkspaceRef|NodeRef|ResourceRef|AnnotationRef`; code type suffix `Ref`, variable suffix `_ref`。
- CLI/UI/locale：显示具体“节点引用/资源引用/批注引用”; `term.typedReference`; 允许 code简称 `Ref`，禁止 UI 裸“Ref”。
- owned-names：`{"wireApiNames":[],"codeConventions":["Ref","_ref"],"localeKeys":["term.typedReference"],"cliUiLabels":["typed reference","类型化引用"]}`。
- semantic-nonaliases：`["pointer","link","ID","key","handle"]`。
- 语义非同义词（人可读投影）：`["pointer","link","ID","key","handle"]`。
- 例/反例：ResourceRef包含owner NodeRef；`resourceId` alone不是typed ref。
- 首次冻结/迁移：`candidate:D3`；R0删除裸 UUID public parameters。
### `weftext.term.reference`

- 正式名：引用事实 / Reference。
- 定义：一个受控 slot/source 对一个 typed target 的可解析语义事实，带resolution/lifecycle；不是独立 durable relation entity。
- owner/layer：D3 reference semantics。
- 排除：Typed Reference value、Node Link occurrence、Citation occurrence、D4 Relation、filesystem pointer。
- wire/API：`ReferenceSlotAddress`, `referenceDispositionPlan`, `rewrittenReferences`; code `ReferenceState|ReferencePlan`, variables `reference_*`。
- CLI/UI/locale：“引用”, `term.reference`; 简称 `ref`只允许指具体typed-reference code symbol，不用作此概念UI label。
- owned-names：`{"wireApiNames":["ReferenceSlotAddress","referenceDispositionPlan","rewrittenReferences"],"codeConventions":["ReferencePlan","ReferenceState","reference_*"],"localeKeys":["term.reference"],"cliUiLabels":["reference","引用"]}`。
- semantic-nonaliases：`["relation","link","citation","edge"]`。
- 语义非同义词（人可读投影）：`["relation","link","citation","edge"]`。
- 例/反例：node-link slot→NodeRef是一条Reference；NodeRef本身是Typed Reference value。
- 首次冻结/迁移：`candidate:D3`；R0删除混用 relation/link/ref 的开放字段。
### `weftext.term.relation`

- 正式名：关系 / Relation。
- 定义：未来 D4 可定义的typed domain relation；D3只规定任何ref-valued endpoint必须服从typed identity/resolution，不冻结 relation ontology。
- owner/layer：owner `D4`; D3 negative gate only。
- 排除：Reference、Node Link、Citation、parent/child ownership、Annotation reply。
- wire/API：D3不分配relation wire；保留 code namespace `Relation*` 给D4，D3禁止使用。
- CLI/UI/locale：D4未冻结前不提供generic relation command/locale；reserved `term.relation`。
- owned-names：`{"wireApiNames":[],"codeConventions":["Relation*"],"localeKeys":["term.relation"],"cliUiLabels":[]}`。
- 简称：无。
- semantic-nonaliases：`["edge","link","reference"]`。
- 语义非同义词（人可读投影）：`["edge","link","reference"]`。
- 例/反例：Node parent不是D4 Relation；未来typed `depends_on`可能是Relation。
- 首次冻结/迁移：`candidate:D3-boundary`, owner D4；R0删除把所有refs叫relations的符号。
### `weftext.term.locator`

- 正式名：定位器 / Locator。
- 定义：绑定owner与准确revision/coordinate、用于定位非entity occurrence/region的非权威值；可失效，不承诺identity continuity。
- owner/layer：D3 location semantics。
- 排除：Typed Reference、path、authoritative identity、author anchor、row handle。
- wire/API：`DocumentElementLocator|DocumentRangeLocator|ResourceRegionLocator`, `l1`; code suffix `Locator`, variables `_locator`。
- CLI/UI/locale：“定位器/位置”, `term.locator`; 简称无。
- owned-names：`{"wireApiNames":["DocumentElementLocator","DocumentRangeLocator","ResourceRegionLocator","l1"],"codeConventions":["Locator","_locator"],"localeKeys":["term.locator"],"cliUiLabels":["locator","位置","定位器"]}`。
- semantic-nonaliases：`["ref","ID","path","anchor token"]`。
- 语义非同义词（人可读投影）：`["ref","ID","path","anchor token"]`。
- 例/反例：source span locator可stale；不能送入EntityRef decoder。
- 首次冻结/迁移：`candidate:D3`；R0删除 locator registry handle identity。
### `weftext.term.owner`

- 正式名：所有者 / Owner。
- 定义：决定对象合法包含、owner-local namespace与不可变归属的领域对象；D3中Resource/Annotation owner恰为一个NodeRef。
- owner/layer：D2 ownership + D3 ref algebra。
- 排除：Authority、source、creator、ACL principal、filesystem parent、display container。
- wire/API：`owner: NodeRef`, `destinationOwnerRef`; code `owner_ref`, functions `validate_owner_binding`。
- CLI/UI/locale：“所有者节点”, `term.owner`; 简称 owner 可用于code。
- owned-names：`{"wireApiNames":["destinationOwnerRef","owner"],"codeConventions":["owner_ref","validate_owner_binding"],"localeKeys":["term.owner"],"cliUiLabels":["owner","所有者节点"]}`。
- semantic-nonaliases：`["parent","authority","source","account"]`。
- 语义非同义词（人可读投影）：`["parent","authority","source","account"]`。
- 例/反例：Resource owner决定ResourceId namespace；provider不是owner。
- 首次冻结/迁移：`frozen:D2` + `candidate:D3 encoding`；R0删除ambient-owner inference。
### `weftext.term.authority`

- 正式名：权威方 / Authority。
- 定义：对某语义状态作最终判定与提交的主体/instance角色；content authority 与 control authority 必须带限定词。
- owner/layer：D1 Core authority + D3 Workspace/source authority。
- 排除：Owner、source、identity、provenance、provider token、ACL principal。
- wire/API：`AuthorityInstanceId`, `expectedAuthority`; code `Authority*`, variables `authority|authority_instance_id`，不得简称owner。
- CLI/UI/locale：“权威方/权威实例”, `term.authority`; 允许 `Core authority`、`external authority` 限定短语。
- owned-names：`{"wireApiNames":["AuthorityInstanceId","expectedAuthority"],"codeConventions":["Authority*","authority","authority_instance_id"],"localeKeys":["term.authority"],"cliUiLabels":["authority","权威实例","权威方"]}`。
- semantic-nonaliases：`["owner","master","source-of-truth"]`。
- 语义非同义词（人可读投影）：`["owner","master","source-of-truth"]`。
- 例/反例：subscribe模式external calendar是content authority；Node仍由NodeRef识别。
- 首次冻结/迁移：`frozen:D1 role` + `candidate:D3 instance semantics`；R0重命名master/slave与owner-as-authority符号。
### `weftext.term.source`

- 正式名：来源 / Source。
- 定义：产生或承载某内容/证据的原始输入或系统；必须以`exact source`、`foreign source`、`source artifact`等限定词出现。
- owner/layer：D2 exact source；D3 provenance/import boundary。
- 排除：Authority、Owner、SourceBinding、Provenance、path。
- wire/API：无裸 `source` identity union；使用具体 `sourceWorkspaceRef|sourceArtifact|sourceSpan`。code变量必须限定 `exact_source|foreign_source|source_artifact`。
- CLI/UI/locale：“来源”, `term.source`; 禁止无上下文简称。
- owned-names：`{"wireApiNames":["sourceArtifact","sourceSpan","sourceWorkspaceRef"],"codeConventions":["exact_source","foreign_source","source_artifact"],"localeKeys":["term.source"],"cliUiLabels":["source","来源"]}`。
- semantic-nonaliases：`["origin","authority","provider","owner"]`。
- 语义非同义词（人可读投影）：`["origin","authority","provider","owner"]`。
- 例/反例：ICS feed是foreign source；它可能是authority，也可能只是一份import artifact。
- 首次冻结/迁移：`frozen:D2 exact-source` + `candidate:D3 boundary`；R0删除裸 `source_id`。
### `weftext.term.source-binding`

- 正式名：来源绑定 / Source Binding。
- 定义：把一个具体外部来源实例、授权scope/collection与mapping namespace绑定为foreign-key比较域的control-plane事实。
- owner/layer：D3 identity/provenance gate；persistence由D6/D9。
- 排除：Source、Authority、OriginBinding、provider credential、NodeRef。
- wire/API：D3只冻结概念名`SourceBinding`; 下游wire必须映射concept ID；code `SourceBinding`, `SourceBindingId`, variables `source_binding(_id)`。
- CLI/UI/locale：“来源绑定”, `term.sourceBinding`; 简称无。
- owned-names：`{"wireApiNames":[],"codeConventions":["SourceBinding","SourceBindingId","source_binding","source_binding_id"],"localeKeys":["term.sourceBinding"],"cliUiLabels":["source binding","来源绑定"]}`。
- semantic-nonaliases：`["connection","account","calendar ID","provider token","origin"]`。
- 语义非同义词（人可读投影）：`["connection","account","calendar ID","provider token","origin"]`。
- 例/反例：两个calendar scope即使UID相同也有不同binding；OAuth token不是binding identity。
- 首次冻结/迁移：`candidate:D3`；D6/D9定义wire/persistence，R0禁止provider token充当binding key。
### `weftext.term.provenance`

- 正式名：来源证据 / Provenance。
- 定义：解释对象/结果从何处、经何操作产生的非授权证据集合；不产生identity equality或write permission。
- owner/layer：D3 evidence boundary；D7/D9可定义payload。
- 排除：Source、SourceBinding、OriginBinding、Authority、audit identity、authorization evidence。
- wire/API：D3不冻结payload；code suffix `Provenance`, variables `provenance`；不得叫`source_ref`。
- CLI/UI/locale：“来源证据”, `term.provenance`; 裸“来源”只属于`weftext.term.source`，不得作为Provenance的CLI/UI label或locale翻译。
- owned-names：`{"wireApiNames":[],"codeConventions":["Provenance","provenance"],"localeKeys":["term.provenance"],"cliUiLabels":["provenance","来源证据"]}`。
- semantic-nonaliases：`["origin","authority","permission","identity"]`。
- 语义非同义词（人可读投影）：`["origin","authority","permission","identity"]`。
- 例/反例：import artifact digest可进入provenance；不能凭它批准Action。
- 首次冻结/迁移：`candidate:D3`；D7/D9定义payload，R0删除provenance-as-auth。
### `weftext.term.origin-binding`

- 正式名：原始对象绑定 / Origin Binding。
- 定义：从一个精确ForeignIdentityKey到一个Weftext authoritative ref的持久、可审计、source-scoped映射，用于deterministic re-import/adoption dedup；它证明关联，不证明identity相等。
- owner/layer：D3 identity/provenance invariant；persistence/retention由D6/D9/A2。
- 排除：Provenance、SourceBinding、IdentityMap、NodeRef、same-identity claim。
- wire/API：D3 concept `OriginBinding(ForeignIdentityKey, NodeRef)`；code `OriginBinding`, variables `origin_binding`。
- CLI/UI/locale：“原始对象绑定”, `term.originBinding`; 简称无。
- owned-names：`{"wireApiNames":[],"codeConventions":["OriginBinding","origin_binding"],"localeKeys":["term.originBinding"],"cliUiLabels":["origin binding","原始对象绑定"]}`。
- semantic-nonaliases：`["source binding","foreign ID","identity map","sync link"]`。
- 语义非同义词（人可读投影）：`["source binding","foreign ID","identity map","sync link"]`。
- 例/反例：foreign `initial_import`或`adopt`成功创建Node时必须写sole active binding；`managed_copy`与`promote`不得写OriginBinding；解绑不改变NodeRef。
- 首次冻结/迁移：`candidate:D3`；D6/D9/A2定义wire/retention，R0禁止title/path dedup。
### `weftext.term.derived-occurrence`

- 正式名：派生出现项 / Derived Occurrence。
- 定义：由外部series rule、occurrence key与准确version/rule cut可重建的非durable occurrence；默认没有Node identity。
- owner/layer：D3 foreign identity boundary。
- 排除：Document Occurrence、Node、series Node、override Record、ResultRowHandle。
- wire/API：D3不冻结typed wire；code `DerivedOccurrence`仅作为downstream value, variables `derived_occurrence`; 禁止 `DerivedOccurrenceId|Ref`。
- CLI/UI/locale：“派生时次/派生出现项”, canonical locale `term.derivedOccurrence`; 用户显示可由D4/D8选择，但必须映射concept ID。
- owned-names：`{"wireApiNames":[],"codeConventions":["DerivedOccurrence","derived_occurrence"],"localeKeys":["term.derivedOccurrence"],"cliUiLabels":["derived occurrence","派生出现项","派生时次"]}`。
- semantic-nonaliases：`["event instance","occurrence Node","calendar item"]`。
- 语义非同义词（人可读投影）：`["event instance","occurrence Node","calendar item"]`。
- 例/反例：RRULE产生的某次Occurrence默认可重建；显式adopt后新Node是另一entity。
- 首次冻结/迁移：`candidate:D3`；D4/D9决定calendar value mapping，R0禁止默认per-occurrence Node。
### `weftext.term.foreign-identity-key`

- 正式名：外部身份键 / Foreign Identity Key。
- 定义：`SourceBinding + foreign component kind + foreign persistent key (+ occurrence discriminator)`形成的source-scoped lookup key；不是Weftext content identity。
- owner/layer：D3 foreign identity boundary。
- 排除：Typed Reference、OriginBinding、UID alone、provider token、path/title。
- wire/API：D3只冻结conceptual components，不冻结JSON；code `ForeignIdentityKey`, variables `foreign_identity_key`。
- CLI/UI/locale：通常不直接显示；诊断“外部身份键”, `term.foreignIdentityKey`; 简称无。
- owned-names：`{"wireApiNames":[],"codeConventions":["ForeignIdentityKey","foreign_identity_key"],"localeKeys":["term.foreignIdentityKey"],"cliUiLabels":["foreign identity key","外部身份键"]}`。
- semantic-nonaliases：`["NodeRef","external ID","origin","dedup key"]`。
- 语义非同义词（人可读投影）：`["NodeRef","external ID","origin","dedup key"]`。
- 例/反例：同SourceBinding+VEVENT+UID+RECURRENCE-ID可lookup override；同UID跨binding不相等。
- 首次冻结/迁移：`candidate:D3`；D6/D9定义wire，R0删除UID-as-NodeId。
### `weftext.term.external-uri`

- 正式名：外部 URI / External URI。
- 定义：指向Weftext authority之外资源的非身份值/occurrence target；解析/fetch能力由下游决定。
- owner/layer：D2 link boundary + D3 non-identity gate。
- 排除：Resource、path、blob、NodeRef、SourceBinding、Authority。
- wire/API：只允许下游closed URI value，不进入EntityRef；code `ExternalUri`, variables `external_uri`。
- CLI/UI/locale：“外部链接”, `term.externalUri`; 简称 URI 可用于code/docs。
- owned-names：`{"wireApiNames":[],"codeConventions":["ExternalUri","external_uri"],"localeKeys":["term.externalUri"],"cliUiLabels":["external URI","外部链接"]}`。
- semantic-nonaliases：`["ResourceRef","file","attachment","source binding"]`。
- 语义非同义词（人可读投影）：`["ResourceRef","file","attachment","source binding"]`。
- 例/反例：`https://example.test/a`不是Resource；导入为Resource必须fresh-create并记录provenance。
- 首次冻结/迁移：`candidate:D3 boundary`; D4/D9决定typed URI/import，R0删除URI-as-resource-id。
## 5. D3 operation verbs

### `weftext.term.copy`

- 正式名：复制 / Copy。
- 定义：保留source，创建fresh target identity与显式mapping；同Workspace与跨Workspace都不保留Node/owner-local identity。
- owner/layer：D3 identity operation。
- 排除：Move、Fork、Import、Adopt、filesystem byte copy。
- wire/API：modes `copy_node_subtree|copy_resource|copy_annotation`; ICS identity intent不冻结为JSON mode；code verb `copy_*|managed_copy`, variables `copy_plan|copy_receipt`。
- CLI/UI/locale：`copy`, “复制”, `action.copy`; 简称无。
- owned-names：`{"wireApiNames":["copy_annotation","copy_node_subtree","copy_resource"],"codeConventions":["copy_*","copy_plan","copy_receipt","managed_copy"],"localeKeys":["action.copy"],"cliUiLabels":["copy","复制"]}`。
- semantic-nonaliases：`["clone","duplicate","move-copy"]`。
- 语义非同义词（人可读投影）：`["clone","duplicate","move-copy"]`。
- 例/反例：`managed_copy`只从managed live Node N产生fresh N2且不写OriginBinding；相同bytes不保留N。
- 首次冻结/迁移：`candidate:D3`; R0删除clone/duplicate commands或迁移到copy。
### `weftext.term.fork`

- 正式名：工作区分叉 / Workspace Fork。
- 定义：从snapshot cut创建fresh Workspace/Node/owner-local identities并返回完整mapping；source继续存在。
- owner/layer：D3 Workspace activation/identity。
- 排除：Copy subtree、Continue、backup Restore、Git branch。
- wire/API：`fork_workspace`; code `fork_workspace`, variables `fork_plan|fork_receipt`。
- CLI/UI/locale：`workspace fork`, “分叉工作区”, `action.workspaceFork`; 简称 fork 只在Workspace scope。
- owned-names：`{"wireApiNames":["fork_workspace"],"codeConventions":["fork_plan","fork_receipt","fork_workspace"],"localeKeys":["action.workspaceFork"],"cliUiLabels":["workspace fork","分叉工作区"]}`。
- semantic-nonaliases：`["clone workspace","copy workspace","restore"]`。
- 语义非同义词（人可读投影）：`["clone workspace","copy workspace","restore"]`。
- 例/反例：fork永不保留source WorkspaceId；raw directory copy不是fork receipt。
- 首次冻结/迁移：`candidate:D3`; R0删除workspace clone alias。
### `weftext.term.continue`

- 正式名：工作区接续 / Workspace Continue。
- 定义：经权威连续性证明后继续同一Workspace identity与ledger；只用于formal continuation/restore path。
- owner/layer：D3 authority continuity。
- 排除：Fork、Copy、ordinary import、open directory。
- wire/API：`continue_workspace`; code `continue_workspace`, variables `continuation_cut`。
- CLI/UI/locale：`workspace continue`, “接续工作区”, `action.workspaceContinue`; 无简称。
- owned-names：`{"wireApiNames":["continue_workspace"],"codeConventions":["continuation_cut","continue_workspace"],"localeKeys":["action.workspaceContinue"],"cliUiLabels":["workspace continue","接续工作区"]}`。
- semantic-nonaliases：`["restore-as-same","resume folder","clone"]`。
- 语义非同义词（人可读投影）：`["restore-as-same","resume folder","clone"]`。
- 例/反例：formal backup+validated cut可continue；ordinary export re-entry不能。
- 首次冻结/迁移：`candidate:D3`; R0删除path/digest auto-continue。
### `weftext.term.move`

- 正式名：移动 / Move。
- 定义：在同一Workspace内改变Node parent/order而保持NodeRef；跨Workspace用户意图必须实现为fresh target transfer加source disposition，不称identity-preserving move。
- owner/layer：D3 structure/lifecycle operation。
- 排除：Copy、Reorder、cross-Workspace identity preservation、filesystem rename。
- wire/API：`move_node`; cross-Workspace使用transfer/copy receipt；code `move_node`, variables `move_plan`。
- CLI/UI/locale：`move`, “移动”, `action.move`; UI跨Workspace必须提示fresh identity。
- owned-names：`{"wireApiNames":["move_node"],"codeConventions":["move_node","move_plan"],"localeKeys":["action.move"],"cliUiLabels":["move","移动"]}`。
- semantic-nonaliases：`["transfer-as-same-id","rename","reparent Resource"]`。
- 语义非同义词（人可读投影）：`["transfer-as-same-id","rename","reparent Resource"]`。
- 例/反例：N在W内P→Q仍是N；W1→W2不能保留NodeRef。
- 首次冻结/迁移：`candidate:D3`; R0删除cross-workspace move-preserve code。
### `weftext.term.import`

- 正式名：导入 / Import。
- 定义：从artifact/source material创建由Weftext authority管理的fresh identity，除非显式formal continue/fork/restore class；RFC 5545 foreign初次导入的identity intent精确称`initial_import`，只在never-bound、有界preview后创建fresh Node与sole active OriginBinding。
- owner/layer：D3 identity effect；mapping/preview由D9。
- 排除：Subscribe、Sync、Adopt、Continue、filesystem copy。
- wire/API：`import_new`与artifact class；ICS identity intent不冻结为JSON mode；code `import_*|initial_import`, variables `import_plan|import_receipt`。
- CLI/UI/locale：`import`, “导入”, `action.import`; 无简称。
- owned-names：`{"wireApiNames":["import_new"],"codeConventions":["import_*","import_plan","import_receipt","initial_import"],"localeKeys":["action.import"],"cliUiLabels":["import","导入"]}`。
- semantic-nonaliases：`["open","attach","ingest-as-same-id","sync"]`。
- 语义非同义词（人可读投影）：`["open","attach","ingest-as-same-id","sync"]`。
- 例/反例：ICS `initial_import`只在never-bound创建fresh Node并写OriginBinding；retired binding必须显式Adopt，不能再次Import；title变化不靠path去重。
- 首次冻结/迁移：`candidate:D3`; D9定义contract，R0删除provider/path-derived IDs。
### `weftext.term.adopt`

- 正式名：采纳 / Adopt。
- 定义：用户显式选择一个foreign object/occurrence成为独立Weftext object；若创建Node则fresh NodeRef且必须写同一ForeignIdentityKey的OriginBinding；它是retired binding后唯一可显式重建sole active binding的verb。
- owner/layer：D3 identity/provenance effect；workflow由D9/A2。
- 排除：Import batch、Promote managed occurrence、Subscribe、Sync、UID identity preservation。
- wire/API：D3不冻结mode；future API/code verb `adopt_*`, variables `adoption_binding`必须映射OriginBinding。
- CLI/UI/locale：`adopt`, “采纳为节点”, `action.adopt`; 简称无。
- owned-names：`{"wireApiNames":[],"codeConventions":["adopt_*","adoption_binding"],"localeKeys":["action.adopt"],"cliUiLabels":["adopt","采纳为节点"]}`。
- semantic-nonaliases：`["promote","import","materialize-as-same-id"]`。
- 语义非同义词（人可读投影）：`["promote","import","materialize-as-same-id"]`。
- 例/反例：adopt never-bound或retired ICS occurrence→fresh Node+sole OriginBinding；active live只返回already-active，UID不成为NodeId。
- 首次冻结/迁移：`candidate:D3`; D9/A2定义workflow，R0删除foreign-id coercion。
### `weftext.term.promote`

- 正式名：提升 / Promote。
- 定义：把受管内容中的无durable identity occurrence显式转为fresh ordinary Node，并原子更新原occurrence；checklist→Task promotion同时声明exact built-in `tasks/task` Facet，fresh NodeRef且无mirror。
- owner/layer：`frozen:D2-v2` checklist→ordinary Node + `tasks/task` semantics；D3 fresh identity effect。
- 排除：Adopt foreign object、Import、in-place kind mutation、adding a label。
- wire/API：D2/D7 future action; code verb `promote_*`, variables `promotion_plan`。
- CLI/UI/locale：`promote`, “提升为节点/任务”, `action.promote`; 无简称。
- owned-names：`{"wireApiNames":[],"codeConventions":["promote_*","promotion_plan"],"localeKeys":["action.promote"],"cliUiLabels":["promote","提升为节点/任务"]}`。
- semantic-nonaliases：`["adopt","convert in place","detach","extract"]`。
- 语义非同义词（人可读投影）：`["adopt","convert in place","detach","extract"]`。
- 例/反例：managed locator resolved的checklist promotion产生fresh ordinary NodeRef、exact `tasks/task` membership、activeBindingWrites=0且无mirror；foreign object不能借Promote绕过Adopt。
- 首次冻结/迁移：`frozen:D2` + `candidate:D3 identity effect`; D7定义payload，R0删除in-place occurrence ID升级。
### `weftext.term.subscribe`

- 正式名：订阅 / Subscribe。
- 定义：建立external-authority持续观察的下游workflow；D3只规定它默认不把foreign identity变成Node identity。
- owner/layer：workflow owner D9/A2；D3 negative identity gate。
- 排除：Import、Sync protocol、Connect setup、Adopt。
- wire/API：D3不冻结；future code `subscribe_*`, locale `action.subscribe`必须映射本concept。
- CLI/UI/locale：`subscribe`, “订阅”, `action.subscribe`; 无简称。
- owned-names：`{"wireApiNames":[],"codeConventions":["subscribe_*"],"localeKeys":["action.subscribe"],"cliUiLabels":["subscribe","订阅"]}`。
- semantic-nonaliases：`["import","sync","connect"]`。
- 语义非同义词（人可读投影）：`["import","sync","connect"]`。
- 例/反例：subscribe calendar保持external authority；不默认每VEVENT建Node。
- 首次冻结/迁移：`candidate:D3-boundary`; D9/A2定义workflow，R0删除subscribe-as-import。
### `weftext.term.sync`

- 正式名：同步 / Sync。
- 定义：在已定义authority/binding/conflict policy下交换或收敛状态的下游workflow；该动词本身不选择identity或authority。
- owner/layer：D6/D9/A2 workflow；D3 negative identity gate。
- 排除：Subscribe、Connect、Import、Continue、automatic merge。
- wire/API：D3不冻结；future code `sync_*`, locale `action.sync`必须有显式authority mode。
- CLI/UI/locale：`sync`, “同步”, `action.sync`; 无简称。
- owned-names：`{"wireApiNames":[],"codeConventions":["sync_*"],"localeKeys":["action.sync"],"cliUiLabels":["sync","同步"]}`。
- semantic-nonaliases：`["import","backup","connect","reconcile"]`。
- 语义非同义词（人可读投影）：`["import","backup","connect","reconcile"]`。
- 例/反例：sync可更新OriginBinding target内容；不能因UID相等合并跨SourceBinding Node。
- 首次冻结/迁移：`candidate:D3-boundary`; D6/D9/A2定义protocol，R0删除identity-changing sync side effects。
### `weftext.term.connect`

- 正式名：连接 / Connect。
- 定义：建立provider/account/network capability的下游setup动作；没有独立content identity effect。
- owner/layer：D1 capability surface + D9/A2 connector workflow；D3 negative gate。
- 排除：SourceBinding、Subscribe、Sync、Import、Authority activation。
- wire/API：D3不冻结；future code `connect_provider`, locale `action.connect`; credential identifiers不映射content refs。
- CLI/UI/locale：`connect`, “连接”, `action.connect`; 无简称。
- owned-names：`{"wireApiNames":[],"codeConventions":["connect_provider"],"localeKeys":["action.connect"],"cliUiLabels":["connect","连接"]}`。
- semantic-nonaliases：`["bind source","subscribe","sync","mount workspace"]`。
- 语义非同义词（人可读投影）：`["bind source","subscribe","sync","mount workspace"]`。
- 例/反例：连接provider后仍需显式SourceBinding/subscribe/import选择；OAuth成功不创建Node。
- 首次冻结/迁移：`candidate:D3-boundary`; D9/A2定义workflow，R0删除connect-implies-import行为。
## 6. Semantic non-alias and retired controlled identifier registry

当前42个entry中的`semantic-nonaliases`是concept-relative排除边界：只有当调用点提供expected concept/typed role时，才可用它拒绝“把A当作B”。它们不是flat denylist；例如`NodeRef`始终是Node的合法owned name，而“把ForeignIdentityKey声明为NodeRef”因expected-concept mismatch失败。

全局retired controlled identifier由以下唯一结构化闭集定义。每个`name`按case-folded exact token匹配、恰出现一次，并有唯一`replacement`；`delete-without-replacement`也是明确且唯一的R0删除目标。该集合必须与全部entry的owned names不相交：

- retired-controlled-identifiers：`[{"name":"AnnotationThreadId","replacement":"delete-without-replacement"},{"name":"AttachmentId","replacement":"ResourceId"},{"name":"BlobId","replacement":"delete-without-replacement"},{"name":"CitationRef","replacement":"delete-without-replacement"},{"name":"DocumentId","replacement":"NodeId"},{"name":"DocumentOccurrenceRef","replacement":"delete-without-replacement"},{"name":"DocumentRef","replacement":"NodeRef"},{"name":"FileRef","replacement":"ResourceRef"},{"name":"FileResourceId","replacement":"ResourceId"},{"name":"ItemId","replacement":"delete-without-replacement"},{"name":"ItemRef","replacement":"delete-without-replacement"},{"name":"LinkId","replacement":"delete-without-replacement"},{"name":"NoteId","replacement":"NodeId"},{"name":"NoteRef","replacement":"NodeRef"},{"name":"NodeSpecialization::Task","replacement":"tasks/task Facet membership"},{"name":"ObjectRef","replacement":"EntityRef"},{"name":"OccurrenceId","replacement":"delete-without-replacement"},{"name":"TaskId","replacement":"NodeId"},{"name":"TaskRef","replacement":"NodeRef"},{"name":"attachment_ref","replacement":"resource_ref"},{"name":"origin_id","replacement":"delete-without-replacement"},{"name":"source_of_truth","replacement":"delete-without-replacement"}]`。

下表只投影常见semantic non-alias与迁移说明，不扩大上述全局retired set：

| Bare term / identifier | Disposition | Canonical replacement | Controlled deletion target |
| --- | --- | --- | --- |
| `note`, 笔记 | concept-relative non-alias | `Node`，或未来下游定义的窄subtype | exact `NoteId|NoteRef`按全局表替换；裸自然语言不扫描 |
| `NodeSpecialization::Task` | global retired controlled identifier | `TaskNode` predicate over exact `tasks/task` Facet membership | 从owned set删除；按§6唯一replacement迁移，不提供compat alias |
| `item`, 项目项 | concept-relative ambiguous term | concrete `Node|Resource|Annotation|DocumentOccurrence|DerivedOccurrence` | exact `ItemId|ItemRef`删除；typed role必须具体化 |
| `object`, 对象 | concept-relative ambiguous term | concrete kind；generic code只用closed `EntityRef` | exact `ObjectRef`→`EntityRef` |
| `clone` | concept-relative operation non-alias | `copy`、`workspace fork`或`workspace continue`三选一 | 因replacement依赖intent，不进入flat retired set |
| `attachment` as kind | concept-relative domain non-alias | `Resource`; attachment只可作mapped UI role | exact `AttachmentId|attachment_ref`按全局表替换 |
| `file`/`blob` as Resource | concept-relative carrier non-alias | `Resource`、`ExternalUri`或D6 byte envelope具体词 | 仅全局表中的exact identifier可裸token删除/替换 |
| `source-of-truth`, `master` | concept-relative authority non-alias | qualified `Core authority|external authority` | 无唯一裸term replacement；typed role裁决 |
| `origin` | concept-relative ambiguous term | `OriginBinding`或`Provenance`按expected concept选择 | exact `origin_id`删除；裸词不扫描 |
| `ref` in UI | concept-relative ambiguous label | concrete “节点引用/资源引用/批注引用” | locale/CLI expected role裁决 |
| `relation` for any link/ref | concept-relative conflation | `Reference|Node Link|Citation|Relation`按expected concept选择 | schema typed role裁决 |
| `instance` for occurrence | concept-relative ambiguous term | `Document Occurrence|Derived Occurrence|AuthorityInstance` | expected concept/type裁决 |

历史 evidence、quoted counterexample 与 migration note 可以提及这些词；negative gate不得改写用户Document/Annotation文本。只有§6结构化全局表可生成flat retired-token拒绝，entry的semantic non-alias只能生成expected-concept/type mismatch。

## 7. Rejected alternatives

| Alternative | Disposition | Reason |
| --- | --- | --- |
| Node 与 Document 都叫 note/page | rejected | 抹掉1:1 ownership与无第二Document identity边界 |
| Document occurrence 与 Derived Occurrence 共用 `OccurrenceId` | rejected | 两者都默认无durable identity且scope/reconstruction不同 |
| Reference/Relation/Link/Citation统一成一个wire kind | rejected | authored occurrence、typed target fact与未来D4 relation职责不同 |
| Resource/attachment/file/blob统一为FileRef | rejected | owner-local domain object、UI role、storage carrier、digest/URI混同 |
| owner/authority/source/origin统一成source | rejected | 归属、决策权、输入来源、映射证据四轴不可观察 |
| copy/clone/fork/continue均叫clone | rejected | fresh identity、Workspace identity continuity与source disposition不可区分 |
| import/adopt/promote均叫import | rejected | batch authority transition、foreign selected adoption与managed occurrence promotion不同 |
| subscribe/sync/connect自动选择authority/identity | rejected | workflow动词不能替代显式identity effect和conflict policy |

## 8. Machine-readable controlled mapping contract

本节冻结可机械投影的映射语义，不冻结任何review artifact的文件名、schema代次、hash、运行时、scanner或sandbox实现。产品权威为本Lexicon当前完整42个entry（39个继承及本轮3个新增）；mapping/allocation/surface inventory必须能从entry与Candidate重建，但都是review/implementation evidence，不新增concept：

- 每个Lexicon concept ID恰一registry entry，canonical中文、英文与本entry逐字相等；
- `wireApiNames`穷尽登记该concept独占的closed wire/API identifier；空数组表示D3刻意不分配wire/API名，不能由实现自行补名；
- `codeConventions`穷尽登记独占的type/function/variable或namespace convention；它是命名门禁，不承诺具体语言ABI；
- `localeKeys`与`cliUiLabels`登记canonical controlled display mapping；同一label不得跨concept复用；
- 一个name可以在同一concept的不同surface重复，例如`fork_workspace`同时是wire mode与code function，但不得跨concept复用；
- `semanticNonaliases`逐entry机械投影`semantic-nonaliases`，只在expected concept/typed role已知时参与判定；它不生成flat deny token；
- `retiredControlledIdentifiers`只机械投影§6的唯一结构化列表；每个name恰一replacement，且与全部owned name case-folded disjoint；
- review evidence必须独立重解析Lexicon与D3 authority target，逐surface记录identifier、source span、actual concept、expected concept与classification，并证明任何name跨concept移动、retired set漂移、未知public identifier或target新增受控名称都fail closed。
- 当前同代registry必须exact包含全部42个entries；`facetMemberships→Facet`、`facetId→FacetId`、`attributeCarrierBlocks|namespaceToken→Attribute Carrier Block`、`rawEntrySource→Lexical Attribute Entry`。`NodeSpecialization::Task`不得属于任何owned set，只能命中§6唯一global-retired rule。合法产品变更必须显式升级Lexicon并接受新的独立Gate；工具代次变化本身不得迫使产品Lexicon升级。

## 9. Controlled naming gate and migration contract

冻结后生成 `D3-Terminology-Negative-Gate/2` 需求清单：

- owned-name 输入：本 Lexicon 的 stable concept IDs、wire/API names、code conventions、CLI labels、locale keys；case-folded exact name必须先映射到唯一concept；
- global retired 输入：只取§6的`retired-controlled-identifiers`；必须与owned names disjoint，每项唯一replacement，不得由builder/validator另维护手写子集或从entry semantic non-alias反向扩大；
- concept-relative 输入：每个entry的`semantic-nonaliases`；只有调用点提供expected concept/typed role时才检查，实际owned concept不同则`expected-concept-mismatch`。正例`NodeRef`映射`weftext.term.node`；ForeignIdentityKey schema/type冒充`NodeRef`时以expected concept mismatch拒绝，而不是因裸token拒绝；
- 扫描面：controlled schema/wire identifiers、public type/function/variable declarations、CLI commands/flags、locale keys、规范性decision/implementation headings及non-JSON fence中的identifier-shaped token；所有surface共用closed extraction与total classification。顺序固定为exact owned-name → global retired → expected-concept/type check → unknown/error；只允许owned-name中显式尾随单个`*`的case-folded literal-prefix convention；
- 明确排除：普通inline只允许closed `[D3-TERM-EXCLUDE:<class>]` marker精确覆盖紧邻单个span；Document/Annotation/user content、历史review/evidence、migration/deletion note、quoted counterexample与第三方format分别使用对应class。JSON fence、non-JSON fence、显式声明值、schema table每个cell与normative heading都禁止marker并始终扫描；exact ICS `UID`等仅token级排除。`不得|不能|禁止|UID|VEVENT`不得豁免相邻或同线D3 identifier；
- 结果：每个命中必须报告file、controlled surface、matched identifier、actual concept ID、expected concept ID（若有）、classification与canonical replacement/deletion target；不能自动重命名或给自然语言报错；
- R0删除/替换目标：只采用§6 exact全局表；`NodeSpecialization::Task`必须唯一迁移为exact `tasks/task` Facet membership predicate且不得保留alias；`clone*`、`master`、裸`origin`、`kind:"entity"|kind:"occurrence"`等需要typed context的表达不得作为flat token规则，必须由schema/expected concept的闭合检查裁决；
- 下游新增术语必须先扩展Lexicon并通过one-term-one-meaning、排除边界、中英文、wire/code/surface与alias审查，不能在实现中先落名再倒逼ontology。

## 10. Gate checklist

1. 每个stable concept ID恰一entry，canonical中英文term不重复映射到不同ID。
2. D1/D2 frozen entries与上游定义相容；任何修改都必须显式reopen，不允许D3 alias覆盖。
3. Node/Document/Entity/DocumentOccurrence/DerivedOccurrence的identity正负例完整。
4. Reference/Typed Reference/Node Link/Citation/Relation边界可由wire/code/surface映射机械区分。
5. Resource/attachment/file/blob/External URI没有coercion。
6. Owner/Authority/Source/SourceBinding/Provenance/OriginBinding互斥且都有最小反例。
7. Copy/Fork/Continue/Move/Import/Adopt/Promote的fresh/preserve/provenance effect唯一；ICS `initial_import|adopt|promote|managed_copy`分别一对一映射Import/Adopt/Promote/Copy且binding effect唯一；Subscribe/Sync/Connect不暗选identity。
8. 全局retired identifier gate只作用受控identifier，不扫描用户内容或抹除历史证据；entry semantic non-alias只在expected concept/typed role存在时裁决。
9. 所有entry-owned name、entry semantic non-alias与§6 global retired table经machine-readable registry双向exact-set映射。owned names与global retired set case-folded disjoint；每个retired name唯一replacement；正例`NodeRef`映射Node，负例ForeignIdentityKey-as-NodeRef以expected-concept mismatch失败。删除owned name、遗漏任一entry projection、移动name/retired scope并协同重建registry均必须fail。自动遍历每条global retired rule×surface，并覆盖同线`不得|不能|禁止|UID|VEVENT`；受控surface中的`NoteRef|ObjectRef|origin_id|FutureRef`分别按owned、retired或unknown规则唯一分类。自然语言显示变化不改变wire/code identity。
10. `Source`的裸中文label恰为“来源”，`Provenance`恰为“来源证据”；两者不得通过context重新共享裸label。
11. 本次冻结的terminology conformance必须明确给出pass、最强一词多义反例为none并覆盖migration/deletion；冻结后任何unknown controlled name、跨concept owned-name复用、global retired命中或expected-concept mismatch仍必须fail closed。
12. 四个D2 v2 entries与五个新增owned wire names精确归属且跨surface无冲突；carrier/entry无EntityRef/locator/Annotation target controlled meaning。
13. Node/Facet coverage必须接受template + lexically valid non-`tasks/task` Facet（positive `project/project`），并拒绝template + exact `tasks/task`；Facet membership自身不分配或改变NodeRef。
14. Task/Promote coverage必须证明Task只等于ordinary + exact `tasks/task`，existing Node的Facet assign/remove保持NodeRef，checklist promote产生fresh NodeRef且无mirror/OriginBinding；`NodeSpecialization::Task`只在global-retired table与本migration说明中出现。
15. 本companion不单独激活；必须与final D2/D3/Impacts/control indexes原子切换，D4保持paused。

## D6组合修订的技术投影归属

保持上述所有concept ID、正式双语名、locale及排除边界。D3的existingPayloadEdits、potentialChanges、artifactBinding、companion_effect、fresh_annotation_reply和Result/9 C是Operation/identity-preserving effect与Payload Subject的内部closed计划技术投影；不新增内容identity或用户实体术语。C内FieldId/OccurrenceKey/typed根与carrier沿D4 Field Value Occurrence和D2 lexical carrier概念归属；不能把C或Entry称为新的Reference Slot/Entity/Locator。

D4SourceMaterializationEffects/1及d4_source_materialization_effects是D4 Field Value Occurrence现有概念的原子源效果技术投影，与D4RelationCopyEffects/1并列；准确schema由D4完整决议§15拥有，D3 v11 §4.1.3a拥有前验绑定。代码类型用D4SourceMaterializationEffects，内部member sourceMaterializationEffects；无新CLI/UI/locale alias，用户仍看到原字段值与操作效果。新技术名称首次适用本次D6 coordinated replacement，既有v9/Result7历史证据不删除；实施发布前当前closed schemas统一迁至wire11/Result9；原v9/v10仅按各自原decoder重放或恢复已保存决议，不接受新旧版decision。负向门禁应拒绝把这些技术计划句柄导出为RecordRef/EntryRef或第二作者源。


Primary/member是一个fresh graph内的角色，不是Owner别名或同owner承诺；reply的互斥以annotation_reply@1 slot为单位，不是Annotation identity互斥。S-origin生命周期证据是原referenceLifecycleTransitions的事实来源，不新增内容实体、引用target或独立receipt概念。

D4 RelationReadContext/2、RelationReadBinding/2及FacetOperationRequest/2为本代状态/源分域的受信接口，D6绑定既有lifecycle或prepared-origin依赖；不会扩大D3最小tombstone或新增content ref。完整消费者与C44/C45正负验收按D4和D6主稿执行，旧运行语料保持历史范围。

当前D6消费引用ObservationScope、IssuerControlPolicy、WorkspaceBootstrapPlan、WorkspaceBootstrapProfile、CalendarPeriodScopeBinding，概念及所有权以同包D6完整机器术语表为准；scope不是新Field/Node identity，bootstrap不是第二decision，四种profile不得成为裸通用Profile产品别名。D3 observationScope已有局部conformance计数成员仍保持其原小写局部含义，不作为新公共类型导出；D6计划内该成员由明确schema分域。

## D7联合revision03消费补充（未接受）

当前协议基线为D3 wire11/Result9；D4 C及全部领域类型语义不变。新增受控名称和exact位置：PreparationBinding=D3.identity_operation_request.preparationBinding；DefinitionTransfer=D3.intent.plan.definitionTransfers[]；D3-Symbolic-Result/9.Q（定义结果分段）=D3-Symbolic-Result/9的Q；PreparedActionBinding=D7受管不可变准备输入；SourceEnvelopeStateCapability/CommitSequenceStateCapability=D6 Policy/2.capabilities；EffectManifest/EffectBytes=D7只读效果运输。它们均非内容实体或第二ledger。完整定义以同包D3主稿§21、D6 Control Interfaces §15–16及D7专项正文为准。

必须新增实际版本解码/legacy replay、fingerprint差异、planned恢复、保存定义typed引用转移/Locator、完整效果及窄Field正向outcome测试；历史wire10与Result8测试保留历史归属，不以数字替换声称新版本通过。其它原实现影响和必需检查不因本补充被删除。

### `weftext.term.preparation-binding`

- 正式名：准备绑定 / Preparation Binding。
- 定义：wire11顶层可选token绑定D7完整不可变准备条件到原requestFingerprint。
- owner/layer：D3 operation/wire；D7仅拥有payload schema。
- 排除：非内容identity、第二ledger、任意JSON patch或权限token。
- wire/API：PreparationBinding, D3.identity_operation_request.preparationBinding. 独立code convention无新增。
- CLI/UI/locale：无新命令/flag；UI使用正式中英文；locale `term.preparation_binding`。
- owned-names：`{"wireApiNames": ["PreparationBinding", "D3.identity_operation_request.preparationBinding"], "codeConventions": [], "localeKeys": ["term.preparation_binding"], "cliUiLabels": ["准备绑定", "Preparation Binding"]}`。
- 简称：无；不接受未列alias。
- semantic-nonaliases：`[]`。
- 语义非同义词（人可读投影）：`[]`。
- 例/反例：wire11的preparationBinding携带bindingToken，使原requestFingerprint绑定已保存的D7准备条件；以同一token改指另一组条件不合法。
- 首次冻结/迁移：D7联合revision03候选；新wire11显式采用，旧v9/v10只按原decoder重放；实施时同步所有decoder/encoder/schema/caller/fixtures，不能静默补成员。

### `weftext.term.definition-transfer`

- 正式名：定义转移计划 / Definition Transfer。
- 定义：wire11第十数组精确绑定保存payload的源/结果occurrence和全部typed slots。
- owner/layer：D3 operation/wire；D7仅拥有payload schema。
- 排除：非内容identity、第二ledger、任意JSON patch或权限token。
- wire/API：DefinitionTransfer, D3.intent.plan.definitionTransfers. 独立code convention无新增。
- CLI/UI/locale：无新命令/flag；UI使用正式中英文；locale `term.definition_transfer`。
- owned-names：`{"wireApiNames": ["DefinitionTransfer", "D3.intent.plan.definitionTransfers"], "codeConventions": [], "localeKeys": ["term.definition_transfer"], "cliUiLabels": ["定义转移计划", "Definition Transfer"]}`。
- 简称：无；不接受未列alias。
- semantic-nonaliases：`[]`。
- 语义非同义词（人可读投影）：`[]`。
- 例/反例：保存定义中由TypeSpec确认的NodeRef按显式slots参与copy映射；普通CEL/text中的同UUID字符串不构成typed slot，必须原样保留。
- 首次冻结/迁移：D7联合revision03候选；新wire11显式采用，旧v9/v10只按原decoder重放；实施时同步所有decoder/encoder/schema/caller/fixtures，不能静默补成员。

### `weftext.term.definition-result-segment`

- 正式名：定义结果分段 / Definition Result Segment。
- 定义：Result9 Q段保存完整原payload和transfer并由原candidate map唯一物化。
- owner/layer：D3 operation/wire；D7仅拥有payload schema。
- 排除：非内容identity、第二ledger、任意JSON patch或权限token。
- wire/API：D3-Symbolic-Result/9.Q. 独立code convention无新增。
- CLI/UI/locale：无新命令/flag；UI使用正式中英文；locale `term.definition_result_segment`。
- owned-names：`{"wireApiNames": ["D3-Symbolic-Result/9.Q"], "codeConventions": [], "localeKeys": ["term.definition_result_segment"], "cliUiLabels": ["定义结果分段", "Definition Result Segment"]}`。
- 简称：无；不接受未列alias。
- semantic-nonaliases：`[]`。
- 语义非同义词（人可读投影）：`[]`。
- 例/反例：Q承载一个完整SavedQueryViewDefinition payload span，并由真实inputPayload和transfer唯一物化；普通正文或D4 carrier不能使用Q分段。
- 首次冻结/迁移：D7联合revision03候选；新wire11显式采用，旧v9/v10只按原decoder重放；实施时同步所有decoder/encoder/schema/caller/fixtures，不能静默补成员。
