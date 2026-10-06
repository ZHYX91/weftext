---
source_language: zh-CN
translation_status: source
---

[English](D3-LEXICON.md)

源文档 ID：b2595435-72ce-423a-805a-09993b04d1d5。

# A2 D3 Terminology and Naming Lexicon

候选状态：A2 D3 current author candidate；未独立接受、未实现、未激活。固定 S 的 42 个 conceptId、四类 ownedNames、semanticNonaliases 顺序与 firstFreeze 全部保持；只对 current definition、producer/consumer version 与 review-state wording 做 source-qualified 更新。

## 1. 权威与控制规则

本词表只拥有 D3 身份、引用、所有权和生命周期的术语边界。D6 公共术语与真实 producer 继续由 D6 唯一拥有。fresh current D3 消费 `DependencyKey/3`、`DependencyProof/3`、`InputDescriptor/3`、`PreparedIntent/3`、`InstallationNotice/3`、`ContentCompletionProof/4` 与 `ChangeRecord/1`；`DecisionKey/2`、`CommitDomain/2`、`SourceVersion/2`、`SourceObservation/1`、`SourceVersionRef/1`、`ObservationScope/2`、`OwnerInputBinding/2`、`D3DecisionCompanion/2`、`RevisionTokenBinding/2` 及专用 `SourceRevisionPlan/1|2|3` 保持真实 owner/version。historical dependency/input/notice `/1-/2`、CP3、wire9–12 等只按 recorded decoder/bytes/pins/recovery 分派，不进入 D3 ownedNames，也不被 current successor 重编码。

每个 concept 只有一个 stable concept ID、一个 canonical 中英文 term pair 和一个 ownedNames exact set。semanticNonaliases只在 expected concept/type 已知时参与拒绝，不是全局裸词 denylist。全局 retired identifier 的 fixed-S 边界保持，不扫描用户 Document/Annotation 文本。

## 2. 完整 concept entries

### weftext.term.workspace

- 正式名：工作区 / Workspace。
- 定义：可移植聚合、授权、事务与 D3 identity namespace；不是作者内容 entity 或开放 owner。
- owner/layer：D2 domain boundary；D1 Core 是语义 authority。
- firstFreeze：frozen:D2。
- migration/deletion：R0 删除把 path/database row 当 Workspace identity 的 identifier
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "WorkspaceRef",
      "workspaceId",
      "workspace_ref"
    ],
    "codeConventions": [
      "WorkspaceId",
      "WorkspaceRef",
      "workspace_id",
      "workspace_ref"
    ],
    "localeKeys": [
      "term.workspace"
    ],
    "cliUiLabels": [
      "workspace",
      "工作区"
    ]
  },
  "semanticNonaliases": [
    "project",
    "vault",
    "folder",
    "tenant"
  ]
}
~~~

### weftext.term.node

- 正式名：节点 / Node。
- 定义：D2 content graph 中唯一长期受管、可独立引用并恰有一份 Document 的作者内容 entity；Node exact-source classification可包含lexically valid Facet memberships，ordinary与template Node均可携带非tasks/task Facet；Task只由ordinary + exact built-in tasks/task predicate成立。
- owner/layer：D2 content graph；D3 定义 NodeRef/lifecycle。
- firstFreeze：frozen:D2。
- migration/deletion：R0 删除旧 NoteId, ItemId, path-node aliases
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "NodeRef",
      "nodeId",
      "node_ref"
    ],
    "codeConventions": [
      "Node",
      "NodeId",
      "NodeRef",
      "node",
      "node_id",
      "node_ref"
    ],
    "localeKeys": [
      "term.node"
    ],
    "cliUiLabels": [
      "node",
      "节点"
    ]
  },
  "semanticNonaliases": [
    "note",
    "page",
    "item",
    "document",
    "folder",
    "data node"
  ]
}
~~~

### weftext.term.document

- 正式名：文档 / Document。
- 定义：owning Node 恰好拥有的一份 exact source 及其语义解释；以 owning NodeRef 寻址，没有第二 durable identity。
- owner/layer：D2 document content；D3只定义地址与 locator。
- firstFreeze：frozen:D2。
- migration/deletion：R0 删除独立 document ID/sidecar identity
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "Document",
      "DocumentProjection",
      "document",
      "document_owner"
    ],
    "localeKeys": [
      "term.document"
    ],
    "cliUiLabels": [
      "document",
      "文档"
    ]
  },
  "semanticNonaliases": [
    "file",
    "node",
    "page",
    "blob",
    "DocumentId",
    "DocumentRef"
  ]
}
~~~

### weftext.term.occurrence

- 正式名：出现项 / Occurrence。
- 定义：在一个明确 owner/execution/rule scope 内出现、可定位或可重建但默认没有 durable identity 的 manifestation 总称；只作元术语，不是 wire kind。
- owner/layer：D2/D3 shared meta-term。
- firstFreeze：frozen:D2 的 Document occurrence 基线 + candidate:D3 的总称收窄。
- migration/deletion：R0 禁止裸 OccurrenceId
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "OccurrenceContext"
    ],
    "localeKeys": [
      "term.occurrence"
    ],
    "cliUiLabels": [
      "occurrence",
      "出现项"
    ]
  },
  "semanticNonaliases": [
    "item",
    "instance",
    "entity"
  ]
}
~~~

### weftext.term.document-occurrence

- 正式名：文档出现项 / Document Occurrence。
- 定义：Document exact source 中的 heading、paragraph、
  list/checklist item、table row/cell、citation、bibliography placement、
  Saved Query/View definition、AttributeCarrierBlock与LexicalAttributeEntry等current-revision语法出现。
- owner/layer：D2 Document content。
- firstFreeze：frozen:D2。
- migration/deletion：R0 删除 block/heading/row durable ID
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "DocumentElement",
      "document_occurrence"
    ],
    "localeKeys": [
      "term.documentOccurrence"
    ],
    "cliUiLabels": [
      "document occurrence",
      "文档出现项"
    ]
  },
  "semanticNonaliases": [
    "block entity",
    "item entity",
    "row record",
    "note"
  ]
}
~~~

### weftext.term.task

- 正式名：任务 / Task。
- 定义：available Facet set包含exact built-in tasks/task的ordinary Node；复用该Node的identity、Document、owner与lifecycle，不形成第二kind或identity。
- owner/layer：D2 v2 built-in Facet predicate；D3只冻结NodeRef/lifecycle effect。
- firstFreeze：frozen:D2-v2。
- migration/deletion：R0 删除 checklist/task mirror、TaskId|TaskRef与旧Task specialization code
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "TaskNode",
      "task_node"
    ],
    "localeKeys": [
      "term.task"
    ],
    "cliUiLabels": [
      "task",
      "任务",
      "任务节点"
    ]
  },
  "semanticNonaliases": [
    "checklist item",
    "todo row",
    "VTODO identity",
    "TaskRef",
    "Task wire kind"
  ]
}
~~~

### weftext.term.facet

- 正式名：分面 / Facet。
- 定义：Node exact-source classification中的可组合capability membership；membership可出现在ordinary或template Node上且不创建新的entity、owner、identity、Document或lifecycle。exact tasks/task禁止出现在template上，并且只使ordinary Node满足Task predicate。
- owner/layer：D2 v2 lexical membership与built-in tasks/task marker；
  D4/D10以后拥有registry、schema与anti-spoof规则。
- firstFreeze：frozen:D2-v2。
- migration/deletion：R0删除以specialization enum代替Facet membership的Task路径
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "facetMemberships"
    ],
    "codeConventions": [
      "Facet",
      "FacetMembership",
      "facet",
      "facet_memberships"
    ],
    "localeKeys": [
      "term.facet"
    ],
    "cliUiLabels": [
      "facet",
      "分面"
    ]
  },
  "semanticNonaliases": [
    "core kind",
    "tag",
    "relation",
    "plugin",
    "node subtype"
  ]
}
~~~

### weftext.term.facet-id

- 正式名：分面标识符 / FacetId。
- 定义：D2 v2 classification中按exact ASCII code point比较的namespace/name lexical token；它标识Facet contract但不是EntityRef或可解析content identity。
- owner/layer：D2 v2 lexical shape；D4/D10以后拥有registry、namespace reservation与validation。
- firstFreeze：frozen:D2-v2 lexical contract。
- migration/deletion：D4/D10另行冻结registry，不把FacetId加入EntityRef decoder
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "facetId"
    ],
    "codeConventions": [
      "FacetId",
      "facet_id"
    ],
    "localeKeys": [
      "term.facetId"
    ],
    "cliUiLabels": [
      "FacetId",
      "分面标识符"
    ]
  },
  "semanticNonaliases": [
    "NodeId",
    "FieldId",
    "namespaceToken",
    "provider id"
  ]
}
~~~

### weftext.term.attribute-carrier-block

- 正式名：属性载体块 / Attribute Carrier Block。
- 定义：D2 v2 exact source中的namespace-scoped protected lexical block occurrence；
  只有owning Document与current-revision ranges，无durable identity或第二payload authority。
- owner/layer：D2 v2 Document lexical projection；D3只冻结non-durable identity与artifact-binding边界。
- firstFreeze：frozen:D2-v2。
- migration/deletion：R0删除whole-namespace blob/sidecar authority，不新增carrier identity
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "attributeCarrierBlocks",
      "namespaceToken"
    ],
    "codeConventions": [
      "AttributeCarrierBlock",
      "attribute_carrier_block",
      "namespace_token"
    ],
    "localeKeys": [
      "term.attributeCarrierBlock"
    ],
    "cliUiLabels": [
      "attribute carrier block",
      "属性载体块"
    ]
  },
  "semanticNonaliases": [
    "field container",
    "record",
    "sidecar",
    "locator",
    "payload authority"
  ]
}
~~~

### weftext.term.lexical-attribute-entry

- 正式名：词法属性条目 / Lexical Attribute Entry。
- 定义：Attribute Carrier Block中的opaque raw lexical entry occurrence；
  只投影raw source与current-revision source range，
  不解释FieldId、typed value、entry identity或provenance。
- owner/layer：D2 v2 Document lexical projection；D4以后拥有inner payload semantics。
- firstFreeze：frozen:D2-v2。
- migration/deletion：D4若未来需要durable field target必须正式reopen，不能复用本occurrence
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "rawEntrySource"
    ],
    "codeConventions": [
      "LexicalAttributeEntry",
      "lexical_attribute_entry",
      "raw_entry_source"
    ],
    "localeKeys": [
      "term.lexicalAttributeEntry"
    ],
    "cliUiLabels": [
      "lexical attribute entry",
      "词法属性条目"
    ]
  },
  "semanticNonaliases": [
    "Field",
    "Record",
    "entry id",
    "reference slot",
    "Annotation target"
  ]
}
~~~

### weftext.term.resource

- 正式名：资源 / Resource。
- 定义：由一个 Node 严格拥有、具有 durable owner-local identity 与单一作者 byte source、但不拥有 Document/Node capability 的非节点对象。
- owner/layer：D2 content object；D3 owner-local ResourceRef/lifecycle。
- firstFreeze：frozen:D2。
- migration/deletion：R0 删除 AttachmentId, FileResourceId, path-as-id
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "ResourceRef",
      "resourceId",
      "resource_ref"
    ],
    "codeConventions": [
      "Resource",
      "ResourceId",
      "ResourceRef",
      "resource",
      "resource_ref"
    ],
    "localeKeys": [
      "term.resource",
      "term.resource.attachmentRole"
    ],
    "cliUiLabels": [
      "resource",
      "资源",
      "附件"
    ]
  },
  "semanticNonaliases": [
    "file",
    "blob",
    "asset",
    "attachment"
  ]
}
~~~

### weftext.term.annotation

- 正式名：批注 / Annotation。
- 定义：由一个 Node 严格拥有、具有 owner-local identity 与 frozen same-owner target union 的非节点对象；fresh current portable state 是完整 `D3-Annotation-Value/4`，body 存在时使用唯一 R6 `AnnotationInlineBody/1` + `AnnotationInlineProfile/1`。historical Value/3/plain-text 只按原 decoder/recovery 保留。
- owner/layer：D2 content object；D3 owner/ref/reply lifecycle。
- firstFreeze：frozen:D2。
- migration/deletion：R0 删除全局/cross-owner Annotation IDs
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "AnnotationRef",
      "annotationId",
      "annotation_ref"
    ],
    "codeConventions": [
      "Annotation",
      "AnnotationId",
      "AnnotationRef",
      "annotation",
      "annotation_ref"
    ],
    "localeKeys": [
      "term.annotation"
    ],
    "cliUiLabels": [
      "annotation",
      "批注"
    ]
  },
  "semanticNonaliases": [
    "comment object",
    "note",
    "thread",
    "message"
  ]
}
~~~

### weftext.term.node-link

- 正式名：节点链接 / Node Link。
- 定义：Document 中 authored inline occurrence，
  其 target intent 是 same-Workspace Node；它是 reference slot，不是 relation entity。
- owner/layer：D2 authored occurrence；D3 target NodeRef/resolution。
- firstFreeze：frozen:D2。
- migration/deletion：R0 删除 link-as-entity/path target
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "node_link"
    ],
    "codeConventions": [
      "NodeLinkOccurrence",
      "node_link"
    ],
    "localeKeys": [
      "term.nodeLink"
    ],
    "cliUiLabels": [
      "node link",
      "节点链接",
      "链接"
    ]
  },
  "semanticNonaliases": [
    "relation",
    "citation",
    "shortcut",
    "symlink"
  ]
}
~~~

### weftext.term.citation

- 正式名：引文 / Citation。
- 定义：Document 中 authored citation occurrence，target intent 是 same-Workspace Node；
  bibliography placement/render numbering不产生 identity。
- owner/layer：D2 authored occurrence；D3只冻结 target ref/locator identity；
  Library citation schema/semantics路由D4/D7。
- firstFreeze：frozen:D2。
- migration/deletion：D4/D7负责 Library citation semantics，R0删除 citation entity ID
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "citation"
    ],
    "codeConventions": [
      "CitationOccurrence",
      "citation_occurrence"
    ],
    "localeKeys": [
      "term.citation"
    ],
    "cliUiLabels": [
      "citation",
      "引文"
    ]
  },
  "semanticNonaliases": [
    "reference",
    "bibliography entry",
    "library item"
  ]
}
~~~

### weftext.term.entity

- 正式名：实体 / Entity。
- 定义：D3 中拥有 durable content identity 的 closed meta-union：Node、Resource、Annotation；仅作上位术语，不是第四种 kind。
- owner/layer：D3 identity algebra。
- firstFreeze：candidate:D3。
- migration/deletion：R0删除开放 ObjectRef/ItemRef 和 entity-kind coercion
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "EntityRef",
      "entity_ref"
    ],
    "localeKeys": [
      "term.entity"
    ],
    "cliUiLabels": [
      "entity",
      "实体"
    ]
  },
  "semanticNonaliases": [
    "item",
    "object",
    "thing",
    "record",
    "node"
  ]
}
~~~

### weftext.term.authoritative-identity

- 正式名：权威身份 / Authoritative Identity。
- 定义：Core 用于判等、解析与生命周期连续性的 durable typed identity；只由完整 Workspace/owner/domain-scoped ref 给出。
- owner/layer：D3 identity algebra。
- firstFreeze：candidate:D3。
- migration/deletion：R0禁止 bare UUID equality
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "AuthoritativeRef"
    ],
    "localeKeys": [
      "identity.authoritative"
    ],
    "cliUiLabels": [
      "authoritative identity",
      "权威身份"
    ]
  },
  "semanticNonaliases": [
    "ID",
    "key",
    "handle",
    "address"
  ]
}
~~~

### weftext.term.typed-reference

- 正式名：类型化引用 / Typed Reference。
- 定义：closed kind 与全部 namespace/owner fields 组成的 authoritative identity value；Ref 是受控 code suffix，不是自然语言泛称。
- owner/layer：D3 wire/API。
- firstFreeze：candidate:D3。
- migration/deletion：R0删除裸 UUID public parameters
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "Ref",
      "_ref"
    ],
    "localeKeys": [
      "term.typedReference"
    ],
    "cliUiLabels": [
      "typed reference",
      "类型化引用"
    ]
  },
  "semanticNonaliases": [
    "pointer",
    "link",
    "ID",
    "key",
    "handle"
  ]
}
~~~

### weftext.term.reference

- 正式名：引用事实 / Reference。
- 定义：一个受控 slot/source 对一个 typed target 的可解析语义事实，带resolution/lifecycle；不是独立 durable relation entity。
- owner/layer：D3 reference semantics。
- firstFreeze：candidate:D3。
- migration/deletion：R0删除混用 relation/link/ref 的开放字段
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "ReferenceSlotAddress",
      "referenceDispositionPlan",
      "rewrittenReferences"
    ],
    "codeConventions": [
      "ReferencePlan",
      "ReferenceState",
      "reference_*"
    ],
    "localeKeys": [
      "term.reference"
    ],
    "cliUiLabels": [
      "reference",
      "引用"
    ]
  },
  "semanticNonaliases": [
    "relation",
    "link",
    "citation",
    "edge"
  ]
}
~~~

### weftext.term.relation

- 正式名：关系 / Relation。
- 定义：未来 D4 可定义的typed domain relation；D3只规定任何ref-valued endpoint必须服从typed identity/resolution，不冻结 relation ontology。
- owner/layer：owner D4; D3 negative gate only。
- firstFreeze：candidate:D3-boundary, owner D4。
- migration/deletion：R0删除把所有refs叫relations的符号
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "Relation*"
    ],
    "localeKeys": [
      "term.relation"
    ],
    "cliUiLabels": []
  },
  "semanticNonaliases": [
    "edge",
    "link",
    "reference"
  ]
}
~~~

### weftext.term.locator

- 正式名：定位器 / Locator。
- 定义：绑定owner与准确revision/coordinate、用于定位非entity occurrence/region的非权威值；可失效，不承诺identity continuity。
- owner/layer：D3 location semantics。
- firstFreeze：candidate:D3。
- migration/deletion：R0删除 locator registry handle identity
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "DocumentElementLocator",
      "DocumentRangeLocator",
      "ResourceRegionLocator",
      "l1"
    ],
    "codeConventions": [
      "Locator",
      "_locator"
    ],
    "localeKeys": [
      "term.locator"
    ],
    "cliUiLabels": [
      "locator",
      "位置",
      "定位器"
    ]
  },
  "semanticNonaliases": [
    "ref",
    "ID",
    "path",
    "anchor token"
  ]
}
~~~

### weftext.term.owner

- 正式名：所有者 / Owner。
- 定义：决定对象合法包含、owner-local namespace与不可变归属的领域对象；D3中Resource/Annotation owner恰为一个NodeRef。
- owner/layer：D2 ownership + D3 ref algebra。
- firstFreeze：frozen:D2 + candidate:D3 encoding。
- migration/deletion：R0删除ambient-owner inference
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "destinationOwnerRef",
      "owner"
    ],
    "codeConventions": [
      "owner_ref",
      "validate_owner_binding"
    ],
    "localeKeys": [
      "term.owner"
    ],
    "cliUiLabels": [
      "owner",
      "所有者节点"
    ]
  },
  "semanticNonaliases": [
    "parent",
    "authority",
    "source",
    "account"
  ]
}
~~~

### weftext.term.authority

- 正式名：权威方 / Authority。
- 定义：对某语义状态作最终判定与提交的主体/instance角色；content authority 与 control authority 必须带限定词。
- owner/layer：D1 Core authority + D3 Workspace/source authority。
- firstFreeze：frozen:D1 role + candidate:D3 instance semantics。
- migration/deletion：R0重命名master/slave与owner-as-authority符号
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "AuthorityInstanceId",
      "expectedAuthority"
    ],
    "codeConventions": [
      "Authority*",
      "authority",
      "authority_instance_id"
    ],
    "localeKeys": [
      "term.authority"
    ],
    "cliUiLabels": [
      "authority",
      "权威实例",
      "权威方"
    ]
  },
  "semanticNonaliases": [
    "owner",
    "master",
    "source-of-truth"
  ]
}
~~~

### weftext.term.source

- 正式名：来源 / Source。
- 定义：产生或承载某内容/证据的原始输入或系统；必须以exact source、foreign source、source artifact等限定词出现。
- owner/layer：D2 exact source；D3 provenance/import boundary。
- firstFreeze：frozen:D2 exact-source + candidate:D3 boundary。
- migration/deletion：R0删除裸 source_id
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "sourceArtifact",
      "sourceSpan",
      "sourceWorkspaceRef"
    ],
    "codeConventions": [
      "exact_source",
      "foreign_source",
      "source_artifact"
    ],
    "localeKeys": [
      "term.source"
    ],
    "cliUiLabels": [
      "source",
      "来源"
    ]
  },
  "semanticNonaliases": [
    "origin",
    "authority",
    "provider",
    "owner"
  ]
}
~~~

### weftext.term.source-binding

- 正式名：来源绑定 / Source Binding。
- 定义：把一个具体外部来源实例、授权scope/collection与mapping namespace绑定为foreign-key比较域的control-plane事实。
- owner/layer：D3 identity/provenance gate；persistence由D6/D9。
- firstFreeze：candidate:D3。
- migration/deletion：D6/D9定义wire/persistence，R0禁止provider token充当binding key
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "SourceBinding",
      "SourceBindingId",
      "source_binding",
      "source_binding_id"
    ],
    "localeKeys": [
      "term.sourceBinding"
    ],
    "cliUiLabels": [
      "source binding",
      "来源绑定"
    ]
  },
  "semanticNonaliases": [
    "connection",
    "account",
    "calendar ID",
    "provider token",
    "origin"
  ]
}
~~~

### weftext.term.provenance

- 正式名：来源证据 / Provenance。
- 定义：解释对象/结果从何处、经何操作产生的非授权证据集合；不产生identity equality或write permission。
- owner/layer：D3 evidence boundary；D7/D9可定义payload。
- firstFreeze：candidate:D3。
- migration/deletion：D7/D9定义payload，R0删除provenance-as-auth
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "Provenance",
      "provenance"
    ],
    "localeKeys": [
      "term.provenance"
    ],
    "cliUiLabels": [
      "provenance",
      "来源证据"
    ]
  },
  "semanticNonaliases": [
    "origin",
    "authority",
    "permission",
    "identity"
  ]
}
~~~

### weftext.term.origin-binding

- 正式名：原始对象绑定 / Origin Binding。
- 定义：从一个精确ForeignIdentityKey到一个Weftext authoritative ref的持久、可审计、source-scoped映射，用于deterministic re-import/adoption dedup；它证明关联，不证明identity相等。
- owner/layer：D3 identity/provenance invariant；persistence/retention由D6/D9/A2。
- firstFreeze：candidate:D3。
- migration/deletion：D6/D9/A2定义wire/retention，R0禁止title/path dedup
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "OriginBinding",
      "origin_binding"
    ],
    "localeKeys": [
      "term.originBinding"
    ],
    "cliUiLabels": [
      "origin binding",
      "原始对象绑定"
    ]
  },
  "semanticNonaliases": [
    "source binding",
    "foreign ID",
    "identity map",
    "sync link"
  ]
}
~~~

### weftext.term.derived-occurrence

- 正式名：派生出现项 / Derived Occurrence。
- 定义：由外部series rule、occurrence key与准确version/rule cut可重建的非durable occurrence；默认没有Node identity。
- owner/layer：D3 foreign identity boundary。
- firstFreeze：candidate:D3。
- migration/deletion：D4/D9决定calendar value mapping，R0禁止默认per-occurrence Node
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "DerivedOccurrence",
      "derived_occurrence"
    ],
    "localeKeys": [
      "term.derivedOccurrence"
    ],
    "cliUiLabels": [
      "derived occurrence",
      "派生出现项",
      "派生时次"
    ]
  },
  "semanticNonaliases": [
    "event instance",
    "occurrence Node",
    "calendar item"
  ]
}
~~~

### weftext.term.foreign-identity-key

- 正式名：外部身份键 / Foreign Identity Key。
- 定义：SourceBinding + foreign component kind + foreign persistent
  key (+ occurrence discriminator)形成的source-scoped lookup key；
  不是Weftext content identity。
- owner/layer：D3 foreign identity boundary。
- firstFreeze：candidate:D3。
- migration/deletion：D6/D9定义wire，R0删除UID-as-NodeId
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "ForeignIdentityKey",
      "foreign_identity_key"
    ],
    "localeKeys": [
      "term.foreignIdentityKey"
    ],
    "cliUiLabels": [
      "foreign identity key",
      "外部身份键"
    ]
  },
  "semanticNonaliases": [
    "NodeRef",
    "external ID",
    "origin",
    "dedup key"
  ]
}
~~~

### weftext.term.external-uri

- 正式名：外部 URI / External URI。
- 定义：指向Weftext authority之外资源的非身份值/occurrence target；解析/fetch能力由下游决定。
- owner/layer：D2 link boundary + D3 non-identity gate。
- firstFreeze：candidate:D3 boundary。
- migration/deletion：D4/D9决定typed URI/import，R0删除URI-as-resource-id
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "ExternalUri",
      "external_uri"
    ],
    "localeKeys": [
      "term.externalUri"
    ],
    "cliUiLabels": [
      "external URI",
      "外部链接"
    ]
  },
  "semanticNonaliases": [
    "ResourceRef",
    "file",
    "attachment",
    "source binding"
  ]
}
~~~

### weftext.term.copy

- 正式名：复制 / Copy。
- 定义：保留source，创建fresh target identity与显式mapping；同Workspace与跨Workspace都不保留Node/owner-local identity。
- owner/layer：D3 identity operation。
- firstFreeze：candidate:D3。
- migration/deletion：R0删除clone/duplicate commands或迁移到copy
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "copy_annotation",
      "copy_node_subtree",
      "copy_resource"
    ],
    "codeConventions": [
      "copy_*",
      "copy_plan",
      "copy_receipt",
      "managed_copy"
    ],
    "localeKeys": [
      "action.copy"
    ],
    "cliUiLabels": [
      "copy",
      "复制"
    ]
  },
  "semanticNonaliases": [
    "clone",
    "duplicate",
    "move-copy"
  ]
}
~~~

### weftext.term.fork

- 正式名：工作区分叉 / Workspace Fork。
- 定义：从snapshot cut创建fresh Workspace/Node/owner-local identities并返回完整mapping；source继续存在。
- owner/layer：D3 Workspace activation/identity。
- firstFreeze：candidate:D3。
- migration/deletion：R0删除workspace clone alias
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "fork_workspace"
    ],
    "codeConventions": [
      "fork_plan",
      "fork_receipt",
      "fork_workspace"
    ],
    "localeKeys": [
      "action.workspaceFork"
    ],
    "cliUiLabels": [
      "workspace fork",
      "分叉工作区"
    ]
  },
  "semanticNonaliases": [
    "clone workspace",
    "copy workspace",
    "restore"
  ]
}
~~~

### weftext.term.continue

- 正式名：工作区接续 / Workspace Continue。
- 定义：经权威连续性证明后继续同一Workspace identity与ledger；只用于formal continuation/restore path。
- owner/layer：D3 authority continuity。
- firstFreeze：candidate:D3。
- migration/deletion：R0删除path/digest auto-continue
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "continue_workspace"
    ],
    "codeConventions": [
      "continuation_cut",
      "continue_workspace"
    ],
    "localeKeys": [
      "action.workspaceContinue"
    ],
    "cliUiLabels": [
      "workspace continue",
      "接续工作区"
    ]
  },
  "semanticNonaliases": [
    "restore-as-same",
    "resume folder",
    "clone"
  ]
}
~~~

### weftext.term.move

- 正式名：移动 / Move。
- 定义：在同一Workspace内改变Node parent/order而保持NodeRef；跨Workspace用户意图必须实现为fresh target transfer加source disposition，不称identity-preserving move。
- owner/layer：D3 structure/lifecycle operation。
- firstFreeze：candidate:D3。
- migration/deletion：R0删除cross-workspace move-preserve code
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "move_node"
    ],
    "codeConventions": [
      "move_node",
      "move_plan"
    ],
    "localeKeys": [
      "action.move"
    ],
    "cliUiLabels": [
      "move",
      "移动"
    ]
  },
  "semanticNonaliases": [
    "transfer-as-same-id",
    "rename",
    "reparent Resource"
  ]
}
~~~

### weftext.term.import

- 正式名：导入 / Import。
- 定义：从artifact/source material创建由Weftext authority管理的fresh identity，除非显式formal continue/fork/restore class；RFC 5545 foreign初次导入的identity intent精确称initial_import，只在never_bound、有界preview后创建fresh Node，并在同一作者决议中写入sole active OriginBinding；retired binding必须显式Adopt。
- owner/layer：D3 identity effect；mapping/preview由D9。
- firstFreeze：candidate:D3。
- migration/deletion：D9定义contract，R0删除provider/path-derived IDs
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "import_new"
    ],
    "codeConventions": [
      "import_*",
      "import_plan",
      "import_receipt",
      "initial_import"
    ],
    "localeKeys": [
      "action.import"
    ],
    "cliUiLabels": [
      "import",
      "导入"
    ]
  },
  "semanticNonaliases": [
    "open",
    "attach",
    "ingest-as-same-id",
    "sync"
  ]
}
~~~

### weftext.term.adopt

- 正式名：采纳 / Adopt。
- 定义：用户显式选择一个foreign object/occurrence成为独立Weftext object；若创建Node则fresh NodeRef且必须在同一作者决议中写同一ForeignIdentityKey的sole active OriginBinding；它是retired binding后唯一可显式重建sole active binding的verb。
- owner/layer：D3 identity/provenance effect；workflow由D9/A2。
- firstFreeze：candidate:D3。
- migration/deletion：D9/A2定义workflow，R0删除foreign-id coercion
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "adopt_*",
      "adoption_binding"
    ],
    "localeKeys": [
      "action.adopt"
    ],
    "cliUiLabels": [
      "adopt",
      "采纳为节点"
    ]
  },
  "semanticNonaliases": [
    "promote",
    "import",
    "materialize-as-same-id"
  ]
}
~~~

### weftext.term.promote

- 正式名：提升 / Promote。
- 定义：把受管内容中的无durable identity occurrence显式转为fresh ordinary Node，并原子更新原occurrence；checklist→Task promotion同时声明exact built-in tasks/task Facet，fresh NodeRef且无mirror。
- owner/layer：frozen:D2-v2 checklist→ordinary Node + tasks/task semantics；
  D3 fresh identity effect。
- firstFreeze：frozen:D2 + candidate:D3 identity effect。
- migration/deletion：D7定义payload，R0删除in-place occurrence ID升级
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "promote_*",
      "promotion_plan"
    ],
    "localeKeys": [
      "action.promote"
    ],
    "cliUiLabels": [
      "promote",
      "提升为节点/任务"
    ]
  },
  "semanticNonaliases": [
    "adopt",
    "convert in place",
    "detach",
    "extract"
  ]
}
~~~

### weftext.term.subscribe

- 正式名：订阅 / Subscribe。
- 定义：建立external-authority持续观察的下游workflow；D3只规定它默认不把foreign identity变成Node identity。
- owner/layer：workflow owner D9/A2；D3 negative identity gate。
- firstFreeze：candidate:D3-boundary。
- migration/deletion：D9/A2定义workflow，R0删除subscribe-as-import
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "subscribe_*"
    ],
    "localeKeys": [
      "action.subscribe"
    ],
    "cliUiLabels": [
      "subscribe",
      "订阅"
    ]
  },
  "semanticNonaliases": [
    "import",
    "sync",
    "connect"
  ]
}
~~~

### weftext.term.sync

- 正式名：同步 / Sync。
- 定义：在已定义authority/binding/conflict policy下交换或收敛状态的下游workflow；该动词本身不选择identity或authority。
- owner/layer：D6/D9/A2 workflow；D3 negative identity gate。
- firstFreeze：candidate:D3-boundary。
- migration/deletion：D6/D9/A2定义protocol，R0删除identity-changing sync side effects
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "sync_*"
    ],
    "localeKeys": [
      "action.sync"
    ],
    "cliUiLabels": [
      "sync",
      "同步"
    ]
  },
  "semanticNonaliases": [
    "import",
    "backup",
    "connect",
    "reconcile"
  ]
}
~~~

### weftext.term.connect

- 正式名：连接 / Connect。
- 定义：建立provider/account/network capability的下游setup动作；没有独立content identity effect。
- owner/layer：D1 capability surface + D9/A2 connector workflow；D3 negative gate。
- firstFreeze：candidate:D3-boundary。
- migration/deletion：D9/A2定义workflow，R0删除connect-implies-import行为
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [],
    "codeConventions": [
      "connect_provider"
    ],
    "localeKeys": [
      "action.connect"
    ],
    "cliUiLabels": [
      "connect",
      "连接"
    ]
  },
  "semanticNonaliases": [
    "bind source",
    "subscribe",
    "sync",
    "mount workspace"
  ]
}
~~~

### weftext.term.preparation-binding

- 正式名：准备绑定 / Preparation Binding。
- 定义：wire11顶层可选token绑定D7完整不可变准备条件到原requestFingerprint；适用 fresh-current wire13 路径继续保留；真实 historical wire11/12 record 保持 recorded recovery。
- owner/layer：D3 operation/wire；D7仅拥有payload schema。
- firstFreeze：D7联合revision03候选。
- migration/deletion：新wire11显式采用，旧v9/v10只按原decoder重放；实施时同步所有decoder/encoder/schema/caller/fixtures，不能静默补成员。fresh-current dispatch 已是 wire13；真实 wire11/12 record 仍保留其原恢复合同
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "PreparationBinding",
      "D3.identity_operation_request.preparationBinding"
    ],
    "codeConventions": [],
    "localeKeys": [
      "term.preparation_binding"
    ],
    "cliUiLabels": [
      "准备绑定",
      "Preparation Binding"
    ]
  },
  "semanticNonaliases": []
}
~~~

### weftext.term.definition-transfer

- 正式名：定义转移计划 / Definition Transfer。
- 定义：wire11 引入的第十数组精确绑定保存 payload 的源/结果 occurrence 与全部 typed slots；fresh-current wire13 继续保留，真实 historical wire11/12 record 保持原 decoder。
- owner/layer：D3 operation/wire；D7仅拥有payload schema。
- firstFreeze：D7联合revision03候选。
- migration/deletion：新wire11显式采用，旧v9/v10只按原decoder重放；实施时同步所有decoder/encoder/schema/caller/fixtures，不能静默补成员。fresh-current dispatch 已是 wire13；真实 wire11/12 record 仍保留其原恢复合同
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "DefinitionTransfer",
      "D3.intent.plan.definitionTransfers"
    ],
    "codeConventions": [],
    "localeKeys": [
      "term.definition_transfer"
    ],
    "cliUiLabels": [
      "定义转移计划",
      "Definition Transfer"
    ]
  },
  "semanticNonaliases": []
}
~~~

### weftext.term.definition-result-segment

- 正式名：定义结果分段 / Definition Result Segment。
- 定义：Result9 Q段保存完整原payload和transfer并由原candidate map唯一物化。
- owner/layer：D3 operation/wire；D7仅拥有payload schema。
- firstFreeze：D7联合revision03候选。
- migration/deletion：新wire11显式采用，旧v9/v10只按原decoder重放；实施时同步所有decoder/encoder/schema/caller/fixtures，不能静默补成员。fresh-current dispatch 已是 wire13；真实 wire11/12 record 仍保留其原恢复合同
- 受控集合：

~~~json
{
  "ownedNames": {
    "wireApiNames": [
      "D3-Symbolic-Result/9.Q"
    ],
    "codeConventions": [],
    "localeKeys": [
      "term.definition_result_segment"
    ],
    "cliUiLabels": [
      "定义结果分段",
      "Definition Result Segment"
    ]
  },
  "semanticNonaliases": []
}
~~~

## 3. retired identifiers 与 semantic non-alias

fixed-S 的 retired-controlled-identifiers exact 集合保持。NodeSpecialization::Task、TaskRef、DocumentRef、AttachmentId、
  FileRef、ObjectRef、OccurrenceId、origin_id、source_of_truth 等既有退役 identifier 仍只按原唯一 replacement/deletion target 处理。

copy/fork/continue/move/import/adopt/promote、subscribe/sync/connect 的语义边界不因多副本改变：文件复制不等于 Workspace Fork/Continue，replica registration 也不是新的 Continue alias。

## 4. D6-FA-r01 导入名称与真实 owner

D6 术语注册表 `D6-Terminology/2`、机器注册表和控制接口生产者，仍是下列 D6 导入概念与技术版本的唯一 owner。fresh-current D3 wire13 只消费这些精确的 current producer 合同；本 Lexicon 不复制其闭合 JSON 形状，也不提供“由 owner 自定义 JSON”的逃逸接口。

| D6 生产者名称/版本 | D3 消费边界 | 禁止解释 |
|---|---|---|
| `DecisionKey/2` | wire13 current 账本，加版本路由的重放/已保存/已计划/未见分流，以及同 P 伴随记录 | 新 D3 身份、第二账本或第二决议 |
| `CommitDomain/2` | 请求、`DecisionKey/2`、回执，以及生产域/观察域资格 | Workspace 身份、执行责任域或全局序号 |
| `ReplicaEpoch` | 副本 `CommitDomain/2` 的写入世代 | `AuthorityInstanceId`、设备身份、`continue_workspace` 或执行租约 |
| `ChangeId/1` | P 封存时的真实可移植效果、`Frontier/2` 与冲突头 | `OperationId`、EntityRef、封存前预留或全局时间 |
| `Frontier/2` | 预期/基线切点、精确策略、已证明的 `scope_dependencies`、current CP4+ChangeRecord1 与清除因果 | 负载已物化、完整查询、Registry 完整性或提供方“已同步” |
| `SourceVersion/2` | 完整的受管/外部生产版本 | 当前观察方身份、裸修订号、摘要或当前性证明 |
| `SourceObservation/1` | 定位器、证据和输入使用的完整本地当前观察 | 第二生产版本、可转移的发送端令牌或文件身份 |
| `SourceVersionRef/1` | 选择一份受保护当前观察的窄本地投影 | 可移植生产版本运输、权限票据或裸修订号 |
| `SourceStamp/1` | 原计划冻结的拟议受管后像地址 | 当前 `SourceVersion/2`、`ChangeId/1`、公开的新 Ref 或提前取得的 Locator 能力 |
| `SourceRevisionPlan/1` | 仅当计划将产生受管后像时使用的 P 内受保护版本依据 | 公开 wire、第二版本账本、删除/无操作版本或可变重试计数器 |
| `RevisionTokenSource/2` | 受保护修订令牌绑定的闭合带标签来源分支 | 裸 `SourceStamp/1`、裸 `SourceVersion/2` 或自由 owner JSON |
| `RevisionTokenBinding/2` | 一个不透明 token 到闭合 `RevisionTokenSource/2` 的受保护稳定生产地址绑定；managed canonicality 只来自 winning plan/seal 与 original sealed-outbox 关联 | 当前 observer 资格、权限、公开来源版本、调用方自选 token 或历史不透明 decoder 的替代 |
| `d6_source_revision/2` | 仅作为新受保护修订令牌配置的标记 | `d6d`、`d6r`、`d6a` 的替代标记，或新的 D3 定位器词法 |
| `ObservationScope/2` | 受权的潜在观察上界 | 写集、单独权限或完整性证明 |
| `DependencyProof/3` | fresh-current 受保护的完整正/负依赖证据 | 调用方读取集合、部分 I、自由 JSON、“没有命中”或较大前沿捷径 |
| `DependencyKey/3` | 由 D6 拥有并版本化的十五类闭合依赖键载体，含 `document_format` | 任意所有者键或通用完整性接口 |
| `OwnerInputBinding/2` | 由 `protocolOwner=D3` 与 owner 版本选择的精确 D3 owner input | 第二请求、回调或 D3 重复封装 |
| `InputDescriptor/3` | fresh-current 精确受保护输入及 pin 绑定 | D3 私有重复 schema 或仅凭 hash 相等 |
| `PreparedIntent/3` | 适用 current 路径的 D6 fresh-current 准备控制记录 | D3 已计划决议、内容身份或自动成功 |
| `D3DecisionCompanion/2` | 一个 D3 主决议的同 P D6 关联记录 | 第二成功回执、第二账本、新写入范围或来源版本表 |
| `WriteProtection` | `strict|observed_only` 安装保护轴 | 语义保证、权限或 `replica_local` 的同义词 |
| `SemanticState` | D2 有效作者语义后的 `complete_semantics|semantic_pending` | 权限、外部无效状态，或仍有义务待证明时的全集证明 |
| `ContentGuarantee` | `replica_local|managed_atomic` 语义/受管屏障轴 | `WriteProtection`、协作模式或授权 |
| `InstallationNotice/3` | fresh-current 使用原基线前沿的不可变安装前组件记录 | 提交证明，或这个尚未封存决议的新 `ChangeId/1` |
| `ContentCompletionProof/4` + `ChangeRecord/1` | fresh-current 封存后以生产版本 `sourceChanges` 表达的可移植证明/因果索引 | 回执读取权限、私有 `DependencyProof/3`、完整查询/动作、批准/资金或执行接管 |
| `ConflictRecord/2` | 以 `Frontier/2` 表示创建切点的当前新 FA 可移植冲突记录 | 自动合并、LWW、绕过 D3，或伪造外部/未知冲突头 |
| `ConflictKey/1` | `ConflictId` 背后的不变完整冲突 key | 记录版本判别器或可独立修改的身份 |
| `ReliableSaveState` | D6 保存状态轴，包括严格可靠保存与合格 `durable_observed_only` | 草稿持久化、同步上传或可移植发布完成 |
| `ExecutionResponsibilityRecord` | 合同实际使用时承载 D6/D10 耐久连续性的容器 | 普通内容权威、副本注册，或 D10 运行/资金/未知状态合同的替代 |

`SourceVersion/2` 与当前 `SourceObservation/1` 明确处于不同的语义域。受管和外部生产版本都绑定完整实体引用、生产 `CommitDomain/2` 与生产 `observationEpoch`；受管分支另外携带按生产域连续历史检查分配的修订号和封存 `ChangeId/1`，外部分支则携带 `externalSequence`，没有受管修订号/ChangeId。当前观察另行绑定 `observerDomain`、当前 `observationEpoch`、`FileObjectBinding` 与证据 pins。`SourceVersionRef/1` 只选择这份完整本地观察。不同生产域中的相同修订号不可比较；外部生产域的修订号、生产观察世代或 `externalSequence` 都不能贡献给新的受管后像修订号。`H(D,E)` 表示生产域 D 的连续已封存历史中实体 E 的最大受管修订号；生产观察世代变化不重置 H，缺失或有缺口的历史不是空历史，真正的原始无操作或来源未改变的结构/生命周期效果也不推进 H。即使字节相同，外部来源接纳仍是真实的外部前像/受管后像接纳。

凡计划确实会产生 managed after，都在 C/Q 或其它绑定 revision 的物化之前，为该实体冻结一份 `SourceRevisionPlan/1` 和恰好一条拟议 `RevisionTokenBinding/2`。binding 的 `source` 使用闭合 `RevisionTokenSource/2`，不携 current observerDomain/current observationEpoch。winning planning CAS 将 token、版本依据与 pins 一起冻结；唯一 P seal 通过 original sealed-outbox 关联把该 exact binding 认证为所得 managed SourceVersion 的唯一 canonical 稳定地址，即使当时还没有 Locator。loser/aborted/seal 不可证明的 binding 不能借另一 seal。后来的 observer 先解析稳定地址，再独立证明完整 current `SourceObservation/1` 的 `sourceVersion` 与之相等；这只可给一次新读取资格，绝不复活旧 selector、PAB/Draft/map、ActionEvidence 或 PreparedIntent。planning 仍不分配 `ChangeId/1`；delete/no-op/source-unchanged 保持既有不产生 after version 的规则。D3 Locator 词法形状与历史 decoder 不改。

`DependencyProof/3` 恰好包含十五类 `DependencyKey/3`：`source`、`document_format`、`lifecycle`、`placement_range`、`ref_inbound`、`relation_incidence`、`calendar_scope`、`registry`、`temporal_rules`、`authorization`、`foreign_binding`、`query_scan`、`replica_registry`、`conflict_record`、`execution_resource`。D6 拥有这些闭合载体种类、排序、stamp/pin 持久化及提交/恢复验证；具体枚举语义仍归真实 owner。对于 D3 的 `placement_range`，`StructureRange` 恰有九类：`live_children`、`trash_children`、`trash_roots`、`ancestor_chain`、`subtree`、`owner_resources`、`owner_annotations`、`reply_closure`、`restore_membership`。空范围和非空范围使用完全相同的完整枚举标准。placeholder、未知解码器、缺失分片、I/O 失败、隐藏且未授权的成员、构建中/部分 I、提供方状态、相同计数/哈希或“没有命中”都不能证明完整空集。D4 继续拥有关系/Calendar/Registry 语义，D7 继续拥有完整查询语义；D3 不能用一个粗粒度的 Frontier 前缀替代这些证明。

fresh-current 可移植发布使用 `InstallationNotice/3` + `ContentCompletionProof/4` + `ChangeRecord/1`。其中 `sourceChanges` 对真实来源状态变更运输完整 production `SourceVersion/2|absent` 前像/后像，而同一决议的当前 D6 效果元数据只可暴露本地 `SourceVersionRef/1` 投影；两层证据不得互相冒充。`frontierBefore` 与 `frontierAfter` 是实际封存切点；精确路径要求原基线保持逐字相等，`scope_dependencies` 则要求完整连续的已封存扩展、全部原依赖重验，以及 P 中耐久保存的“扩展确实无关”证据。`InstallationNotice/3.baseFrontier` 永不改写。接收端验证 Notice、proof、components 与生产历史后，必须建立自己的 `SourceObservation/1`/`SourceVersionRef/1`；不得复制发送端 `sourceToken`，也不能仅凭 current CP4/ChangeRecord chain 获得完整查询/动作资格。

当前冲突使用 `ConflictRecord/2` 与 `Frontier/2`，而 `ConflictKey/1`、`ConflictId`、排序及 `D6-ConflictKey/1` hash domain 保持不变。每个冲突头都必须是真实、连续验证并已封存、且与该 key 相关的 `ChangeId/1`。历史 `ConflictRecord/1` 继续使用 `Frontier/1` 和原解码器/字节；记录不得原地静默升级，/1 与 /2 也不能成为同一未改变 key 的两个当前记录。

## 5. wire13 current D3 操作名称与 historical record

`identity_operation_request`、`identity_change_receipt`、`identity_operation_error` 等既有 D3 技术名称继续归原概念。fresh current native request 是 wire13，owner-kind 为 `d3_identity_operation/13`，并使用 `InputDescriptor/3`；historical wire9–12 继续原 decoder。`d3_identity_operation/13` 是 D3 自有的 owner-kind/规范描述符名称，不因此重新拥有 `OwnerInputBinding/2`。D3 主回执继续保留原有十二类效果数组，并与同一 P 中的 `D3DecisionCompanion/2` 明确区分。

准备绑定、定义转移、定义结果分段三个既有概念继续保留 fixed-S 中的概念身份、`ownedNames` 和 `firstFreeze`。外层 `preparationBinding` 仍只是 D3 请求中的令牌封装；fresh current 选择 D7-owned `PreparedActionBinding/4`，historical PAB1–3 继续原恢复，D3 不复制第二套 D7 schema。`definitionTransfers`、Result/9 的 `Q` 分段，以及既有 B/M/N/E/S/C/Q 身份变更分区继续由 D3 拥有；保存查询/视图/动态块的负载模式与 D7 效果仍归 D7。

在复制、分叉和身份承载导入中，类型化定义转移只遍历真实 D7 类型化根：`DefinitionAddress.owner` 与其中完整、已识别的 Locator 根；由 TypeSpec 声明的类型化 Ref，并递归进入其类型化容器；`CollectionCreationPolicy.parent`；已保存 View/Query 的调用地址、Query selector 字面量、`QueryRef` 参数、参数默认值、固定 View 域 `TypedLiteral`，以及 DynamicBlock 的字面量/上下文绑定。普通 CEL 表达式文本、普通自然语言文本和未知 JSON 都不扫描。完整 Locator 根已经包含 owner，因此不得再为它的 owner 生成重叠槽。每个已识别的类型化 Ref/Locator 恰有一个槽；确认没有任何 Ref/Locator 的合法负载仍必须产生 `slots=[]`。来源/结果负载和出现项分别由真实 pin 独立绑定与验证，`Q` 继续与精确的保存定义负载跨度一一对应。两遍位置物化始终使用同一份私有候选映射，以及同一 `SourceRevisionPlan/1`/修订绑定；它不创造新 wire，也不创造第二种定义身份。

复制始终创建新的映射目标身份；Resource/Annotation 的所有者不原地改变。`copy_annotation` 继续区分新映射 Annotation 的初始回复（由引用计划表达）与原模式另行允许修改的目标所有者下既有 Annotation（只有后者才可作为 S 载体）。分叉在同一身份映射中处理要求的完整 live/trashed 两态闭包。`continue_workspace` 只有在排他接续/故障转移证据证明旧权威方已停止或被有效栅栏隔离，并且要么证明切点后没有已提交事实、要么取得足以完整承接并合并这些事实的权威账本时，才保留同一逻辑 Workspace；副本注册不是工作区接续。部分身份承载导入继续以离线制品为前像权威，普通导入中的工作进程/IR 标识永远不是内容身份。

历史兼容以真实存在的记录为准，而不是以原型名称为准。wire9/v10/v11，以及任何真实存在的旧 `PreparedActionBinding/1,/2`、SourceObservation/ref/token、已保存/已计划/未知记录、回执、pin、计费、批准/claim 或外部效果责任，只按实际生成它们的解码器、原字节/指纹、原 profile、授权、保管连续性、时钟/TTL、pin 生命周期和不得重复效果的责任继续履约。仅存在解码器、fixture、草稿或候选文字，并不能证明所有历史原型已经部署或处于活动状态，也不能把旧字节批量升级到当前生产者合同。

对于已保存/已提交的原决议，在原请求/指纹/连续性查账成功后，只按该决议原来的实际效果/模式或结果披露范围检查当前交付授权，然后返回原回执/错误/效果字节，或恢复原版本发布/outbox。后来的 r6 `SourceObservation/1`、Frontier、业务证明、准备/预览 TTL 或新的消费者门，都不能倒追拒绝或重新执行这个已保存决议；撤权只能遮蔽当前交付，不能重写历史、重装旧后像、重新分配 H/修订号/ChangeId，也不能再次收费。已计划记录只恢复原冻结请求、其 recorded InputDescriptor（fresh current 为 `/3`，historical 使用其 recorded version）、适用时的候选映射、`SourceRevisionPlan/1`/版本依据、pins、预留/写集、其 recorded InstallationNotice（fresh current 为 `/3`，historical 使用其 recorded version）、`WriteProtection`、尝试/预算、准备期限和安装状态；不得重新查询、重选目标、重采样身份/H/修订号，也不得准备第二个决议。安装或结果未知时，原 pins 与 Approval/Money/claim/外部效果/停止责任继续保留；当前文件、I、相同哈希、重新授权或新的空控制库，都不能猜测成功/失败，也不能授权盲目重试、退款或重置。

## 6. 受控命名门

实现和后续 owner 必须机械验证：

1. 42 个 inherited `conceptId` 全部存在且唯一；
2. 每个概念的四类 `ownedNames` 表面集合与 fixed S 完全相等；
3. 每个 `firstFreeze` 与 fixed S 完全相等；
4. D6 导入的公共名称和 D6 内部技术名称，不会因为 D3 消费它们而进入 D3 `ownedNames`；
5. 同一个公共标识符不能由两个概念共同拥有，退役标识符也不能进入受控集合；
6. 普通说明文字、用户内容、第三方格式和历史证据，不作为平面拒绝词表输入；
7. 新技术成员如果不能证明存在真实生产者/owner 映射，就必须拒绝或停用依赖该成员的路径，不能由 D3 临时造名；
8. D6 闭合类型必须按真实生产者版本消费；“由 owner 自定义 JSON”不得代替 current `DependencyKey/3`、`StructureRange`、修订绑定、CP4+ChangeRecord1、冲突或其它闭合接口；
9. 双语正文对同一语义必须使用相同的协议标识符；文档排版不能重命名 wire 成员、拆开一个点号成员，或制造兼容别名。

## 7. D6-FA-r01 术语压力案例

- “副本”只指已登记的普通物理副本，不是工作区分叉、工作区接续、Authority 或执行责任接管。
- ordinary/local/complete 表示语义证据范围；`strict|observed_only` 是独立的 `WriteProtection` 安装保护轴。所有 D3 身份/结构/生命周期作者操作，包括创建、移动、重排和 Trash，始终使用 `strict`；D3 不存在弱化的身份操作模式。
- `observed_only` 只属于 D6 受信人工 `interactive_source_save`：人工必须在 planning 开始前显式选择并冻结，目标恰为一个既有 live Document，执行 `ordinary+replica_local` 完整来源读/替换；作者来源写集为空或仅包含该 Document；不存在适用的正文/Field/节点控制拒绝；不修改身份、父级/顺序、生命周期、共享策略、Registry、Calendar 或其它实体；Draft Base 等于选定的完整当前 `SourceObservation/1`。它绝不是严格计划失败后的后备路径，也不新增逐次批准流程。
- 合格 `observed_only` 保存的实际读取前像 B 与用户输入 N 必须耐久保留。最终受信检查后未观察到的外部 C 可能被 N 覆盖，并且可能没有可恢复副本；后来 C 可以再次成为当前文件，但不能抹掉 B/N。已经观察到的竞争、过期 Base、watcher/event 缺口、撤权、第三状态或安装未知都不在该豁免内，继续走冲突/重新准备/暂停或 `recovery_unknown`。
- 已准备状态或 `inputRetentionState=retained` 只表示提案、实际读取前像以及所需绑定/pins 已保留，并不等于“已保存”；封存后的 `durable_observed_only` 仍与严格 `reliable` 以及可移植发布状态区分。
- `semantic_pending` 只表示 D2 有效的局部事实已经通过，而合同明确允许继续待定的义务尚未证明。它绝不是 D2 无效、类型化值无效、权限拒绝、空范围、完整查询/全结果、强动作、自动化、批量/集合、受管恢复/清除/复制/分叉/导入、服务器检查点、批准或资金成功的同义词。
- 普通 `.adoc` 来源与普通 Resource 字节仍是当前作者权威。可移植元数据继续拥有身份、父级/顺序、生命周期、共享策略/信任、Registry 相关可移植控制，以及可移植变更/冲突事实。P 保留不可从文件重建的决议/恢复/责任和必要 pins；I 可删除并重建，绝不能单独证明完整性。
- `SourceVersion/2` 的生产域/世代与当前 `SourceObservation/1` 的观察域/世代分离。两个副本从同一生产前像/来源切点出发时，各自在自己的操作 `CommitDomain/2` 下构造完整的本地当前观察；不得复制另一副本的 `sourceToken`。
- `d6_source_revision/2` 只能通过 `RevisionTokenBinding/2` 及其闭合 `RevisionTokenSource/2` 解析；它不替换历史不透明的 D3 修订令牌词法，也不改变 D4/D5 内层选择器/修订号线格式。
- current `DependencyProof/3` 的完整空集必须来自真实证明，而不是 I 未命中。删除/重建 I 不会在受保护证明连续性仍完整时重置该连续性；真实缺口或正确性证据丢失则必须进入新的证明世代，相同最终哈希不能复活旧证明。
- 恢复与清除的成员资格不同。恢复只使用原 Node 闭包的 `restore_membership`，因此更早独立进入 Trash 的所有者局部 Resource/Annotation 继续保持在 Trash；清除则必须覆盖全部当前仍由该所有者拥有且处于 Trash 的成员，包括这些更早独立进入 Trash 的对象，并同时取得真实副本、入站引用、控制和其它所有者的强证明。
- 复制/分叉/导入/定义转移继续使用一份候选映射、精确前像/结果证据、所有者局部规则、完整类型化槽，以及 `Q` 的出现项/跨度双射。`copy_annotation` 不把新映射主 Annotation 变成 S 载体。普通文本或未知 JSON 中看似 UUID 的字符串，不会因为扫描而变成类型化 Ref。
- `Frontier/2` 只表示已验证的连续封存因果前缀。managed-atomic 的强 D3 路径继续使用 exact。四个 replica-local 局部结构模式只有在完整的基线到当前封存链，以及全部原来源/控制/授权/正负依赖都证明无关时，才可使用 `scope_dependencies`；`InstallationNotice/3.baseFrontier` 永不前移。
- current `ContentCompletionProof/4` + `ChangeRecord/1` 在封存后运输/索引生产版本历史；接收端建立自己的当前观察，仅凭可移植证明并不能取得完整 Query/Action/执行资格。
- “冲突”仍属于 D6 可移植控制。当前记录使用 `ConflictRecord/2`；`ConflictKey/1`、`ConflictId` 与 `D6-ConflictKey/1` 保持稳定，未封存的外部/未知条件不得伪造冲突头。
- 历史已保存决议按原 profile 和原效果/结果范围的当前交付授权重放；已计划决议只恢复冻结计划；未知结果保留原连续性与不得重复效果的责任。三者都不批量升级，也不由当前 r6 状态重新裁决。
- 固定 C 已经包含较早一代的 D3 wire12 与 D4/D5 A/B/C 消费者候选，它们消费了原 G0-A/G0-B 的 SourceObservation/Frontier/WriteProtection 基线。这些是真实候选历史，不是“尚不存在”，但仍未接受、未激活，也尚未消费本词表所列全部 P1 新生产者规则：适用处仍需真实所有者/消费者后像承接生产域 H/修订规划与令牌绑定、十五类依赖键/范围证明（含 document_format）、CP4+ChangeRecord1、当前 ConflictRecord/2 边界和 M5 重放/恢复分流。缺少这种强消费者只门控真正依赖它的路径，绝不能永久取消不依赖该强证明且已合格的普通 `.adoc`/Resource 读取、草稿、人工完整来源保存或本地离线操作。
- “执行责任”继续属于 D6/D10 控制，不是 D3 权威方、`CommitDomain/2` 或副本注册。`sourceOccurrenceKey`、批准使用/资金、运行/租约/自动化/部署以及外部未知连续性仍归 D10/各真实所有者。
- 中文“来源”仍只对应本词表的来源概念，“来源证据”仍只对应来源证据概念；所有者仍不等于权威方，路径仍不等于内容身份或 Node 父级。

## 8. 接受与激活边界

本词表只是 P2 D3 术语的中英配对作者后像。两份私人 D3 主文与本词表都只是作者候选，不构成独立接受、实现、激活或发布。固定 S 的 42 个概念快照、其 `conceptId`/`ownedNames`/`firstFreeze`，以及 fixed-S 的快照、输入和目录全部保持不可变、只读的历史输入。P2 只同步真实所有者后像与三份路由元数据，不授权改写这些历史来源。

固定 C 已经完成原 G0-A/G0-B 基线对应的上一代 D4/D5 候选消费者工作。本轮上文列出的 P1 生产者新增规则，仍需适用的 D3/D4 P2 与 D5 P3 消费者后像，以及 D7 全套和既定 D8/D9/D10 owner/consumer 集合完成后，任何依赖它们的受管/强路径才能协调。缺项既不是部分激活的许可，也不是对无关且合格的普通/局部/离线内容操作的全局禁用。

已协调的 D6 Control/Registry 候选现已承认真实存在的 native D3 descriptor/companion，并精确表述 InstallationNotice 边界：baseFrontier 可含历史 sealed ChangeIds，但不含本尚未封存决议的新 ChangeId。这些生产者修订仍是等待全新联合接受的候选文字；本词典不编辑其生产者，也不激活消费者。

historical R08 的 3 P1 + 8 P2 继续作为 review provenance；后续具名 repair 与 fixed-SHA bounded/limited review 只在 exact scope 内有效。PL-IR-01 在 PR3 fixed e8aa 的 accumulated review scope 内已有 prior bounded/limited closure，但这不是 fresh A2/global acceptance。完整 A2 最终仍需全新非作者对一个 fixed final SHA 做 global review。本作者不自行关闭 A2 finding，也不声称产品测试、实现、激活、发布或部署通过。本 head 对 fixed-446 D3 P1-01/P1-02 的 repair disposition 仅为 resolved-pending-independent-review；不重新打开该报告已经有界独立关闭的 D1/D2 项。

## 9. 闭合 D3 适配器投影

以下是既有受控概念的技术投影，不是新内容身份或替代firstFreeze。D3ResolverInput/12是受保护不可变Ref/Locator读上下文，绑定显式Workspace/domain/精确Frontier与闭合outcomes（主文§11）。wire12 transfer观察绑定完整target/source DecisionKeys，保留两个独立Copy/Trash决议（主文§8.4.1），不创建跨Workspace Move身份。

D3ConflictResolutionPrepare/1 继续准备既有的移动、生命周期和复制语义。新的当前冲突处理使用 `PreparedActionBinding/4`、`D3ResolutionInputUse/2` 与 `InputDescriptor/3` 保留完整的 ConflictKey、冲突头、选择和分支证据；最终原生请求仍为 wire13。历史 PAB3/Use1/wire12 准备流程保持精确恢复。完整预览仍通过 D7 的清单、分页和字节传输交付，不是 PinRef 读取承诺。

### 9.1 Canonical 物化与生命周期选择

当前 resolution binding 显式合成所选 canonical 物化与原 native operation。D3CanonicalInput/1 绑定所选生产 source 和 D6 ConflictInstallInput/1 的真实 installed before，不是 current source observation 或普通写权。open record 的保留 live/Trash 不产生 lifecycle transition，但有 portable conflict effect，不借相反 head membership 或虚构反向转换。可在同一 plan 物化 Resource 保留的 canonical claim，同时 fresh-copy 另一 head；raw copy mode 没有新 overwrite slot。精确净 receipt effects 与 H/version 规则见主文 §10.1.1.1–.2。typed installation wrapper/conflict-only SourceRevisionPlan/2 归 D6 producer；D3ResolutionInput/1 canonicalInputs 与不可剥离用途 guard 是 D3 消费，D7 /3 持久保存。controlled concept JSON 与 firstFreeze 不变。

### 9.2 Canonical evidence projection

D3CanonicalEffectPlan/1 是受保护的完整 source 选择证据命名空间，不是 raw native mode 扩展。D3CanonicalPlanProjection/1 公开投影 exact plan，并必须完整运输对应 bytes。D3CanonicalEffects/1 是同 seal 公共语义扩展，不是另一 receipt/decision：primary 十二数组只表达 native，canonical reference/S/lifecycle 证据各自遵守原 comparator 范围。跨分量共享历史来源显式表达，物理 source effect 与 H 只按精确相等去重。ordinal/kind 配对既不提供持久 slot identity，也不重锚 Locator。D7 完整运输两个分量，未知/缺失扩展不冒作完整成功；当前交付失败也不改变 saved commit 的历史事实。controlled concept JSON 和 firstFreeze 均不改。
