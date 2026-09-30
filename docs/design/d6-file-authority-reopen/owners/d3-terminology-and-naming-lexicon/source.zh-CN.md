---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：b2595435-72ce-423a-805a-09993b04d1d5。

# D3 Terminology and Naming Lexicon — D6-FA-r01

候选状态：D6-FA-r01；部分联合候选；未接受、未激活、未实现。固定 S 的 42 个 concept 全部保留 conceptId、ownedNames 与 firstFreeze；本后像没有删除、重命名或转移任何既有受控名称。

## 1. 权威与控制规则

本 Lexicon 仍只拥有 D3 identity/reference/ownership/lifecycle 的术语边界。G0-A/G0-B 使用的 DecisionKey、CommitDomain、ReplicaEpoch、ChangeId、Frontier、SourceVersion、SourceObservation、SourceVersionRef、ObservationScope、DependencyProof、OwnerInputBinding、InputDescriptor、D3DecisionCompanion、WriteProtection、SemanticState、ContentGuarantee、ConflictRecord、ReliableSaveState 和 ExecutionResponsibilityRecord 仍由 D6 术语 owner 拥有；D3 只消费，不在本表新增同义 concept。

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
- 定义：由一个 Node 严格拥有、具有 owner-local identity、plain-text body 和 frozen target union 的非节点对象。
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
- 定义：从artifact/source material创建由Weftext authority管理的fresh identity，除非显式formal continue/fork/restore class；RFC 5545 foreign初次导入的identity intent精确称initial_import，只在never-bound、有界preview后创建fresh Node与sole active OriginBinding。
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
- 定义：用户显式选择一个foreign object/occurrence成为独立Weftext object；若创建Node则fresh NodeRef且必须写同一ForeignIdentityKey的OriginBinding；它是retired binding后唯一可显式重建sole active binding的verb。
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
- 定义：wire11顶层可选token绑定D7完整不可变准备条件到原requestFingerprint。
- owner/layer：D3 operation/wire；D7仅拥有payload schema。
- firstFreeze：D7联合revision03候选。
- migration/deletion：新wire11显式采用，旧v9/v10只按原decoder重放；实施时同步所有decoder/encoder/schema/caller/fixtures，不能静默补成员
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
- 定义：wire11第十数组精确绑定保存payload的源/结果occurrence和全部typed slots。
- owner/layer：D3 operation/wire；D7仅拥有payload schema。
- firstFreeze：D7联合revision03候选。
- migration/deletion：新wire11显式采用，旧v9/v10只按原decoder重放；实施时同步所有decoder/encoder/schema/caller/fixtures，不能静默补成员
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
- migration/deletion：新wire11显式采用，旧v9/v10只按原decoder重放；实施时同步所有decoder/encoder/schema/caller/fixtures，不能静默补成员
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

下列名称由 D6 terminology registry D6-Terminology/2 拥有，D3 wire12 只作为 imported type/member 使用：

| D6 concept | D3 消费位置 | 禁止解释 |
|---|---|---|
| DecisionKey/2 | wire12 ledger、replay、companion | 新 D3 identity 或第二 ledger |
| CommitDomain/2 | wire12 request、DecisionKey、receipt | 新 D3 authority/identity |
| ReplicaEpoch | replica CommitDomain | AuthorityInstanceId、device ID、continue token |
| ChangeId/1 | portable receipt、Frontier/2、conflict heads | OperationId、EntityRef、prepare reservation |
| Frontier/2 | expectedFrontier、purge causality | payload 物化、Query 全集、同步完成 |
| SourceVersion/2 | SourceObservation 内的生产版本 | 当前 observerDomain 或裸 revision |
| SourceObservation/1 | locator/evidence/current input | 第二 SourceVersion、文件 identity |
| SourceVersionRef/1 | 窄元数据/receipt 投影 | 完整 SourceVersion 或权限票据 |
| ObservationScope/2 | wire12 前门观察上界 | 写权限、实际 readSet、DependencyProof |
| DependencyProof/2 | 正负范围与 purge/strong gate | partial index、free JSON completeness |
| OwnerInputBinding/2 | protocolOwner=D3、ownerKind=d3_identity_operation/12 | D3 owned wrapper 或第二 request |
| InputDescriptor/2 | wire12 exact input | D3 私有副本 schema |
| D3DecisionCompanion/2 | 同 P seal 的 D6 companion | 第二成功 receipt 或第二 ledger |
| WriteProtection | D3 owner descriptor 固定 strict | replica_local、permission、observed_only |
| SemanticState | replica_local result qualification | D4/D5 complete proof 的替代 |
| ContentGuarantee | replica_local/managed_atomic | WriteProtection、permission 或协作模式 |
| ConflictRecord | sync conflict 定位 | D3 第二 conflict ledger |
| ReliableSaveState | D6 decision state | portable publication 已完成 |
| ExecutionResponsibilityRecord | D10 continuity | replica registration 或 content authority |

wire12 的 commitDomain、guarantee、expectedFrontier、inputDescriptor、frontierPolicy、observationScope 与 ownerInput 都是 D6 closed type/member 的消费位置；它们不在 D3 新建 ownedNames。D3 只定义 d3_identity_operation/12 canonical descriptor 的内部语义成员，并通过 D6 owner decoder + cross-field equality 消费 wrapper。

D3DecisionCompanion/2 仍由 D6 拥有；D3 primary receipt 继续由 D3 拥有。两者同 P 保存不构成术语或持久决议的双 owner。

## 5. wire12 与 legacy 名称

identity_operation_request、identity_change_receipt、identity_operation_error 等既有 D3 technical wire 名在 wire12 保持同一概念归属；版本号改变不新建 ontology concept。d3_identity_operation/12 是 D3-owned ownerKind/canonical descriptor 名称，不重新拥有 D6 的 OwnerInputBinding/2。

Preparation Binding、Definition Transfer、Definition Result Segment 三个既有 concept 的 firstFreeze 继续保留其 wire11 历史。wire12 可以复用这些名称，但 backing Prepared record 的新版本由未来 D7 afterimage拥有；D3 不把 PreparedActionBinding/2 自动重命名或升级为新 schema。

D3-CJ/3、D3-Symbolic-Result/9、Annotation Value/3、Ref/Locator 受控名称保持既有 owner。

## 6. 受控命名门

实现和后续 owner必须机械验证：

1. 42 个 inherited conceptId 全部存在且唯一。
2. 每个 ownedNames 四个 surface 集与 fixed S exact-equal。
3. 每个 firstFreeze 与 fixed S exact-equal。
4. D6 imported names不会出现在 D3 ownedNames 中。
5. 同一 public identifier不跨 concept 复用。
6. retired identifier不进入 owned set。
7. ordinary prose、用户内容、第三方 format 与历史 evidence不作为 flat denylist输入。
8. wire12 新增 technical member若没有真实 owner mapping，候选必须 fail closed，而不是实现先落名。

## 7. D6-FA-r01 术语压力案例

- “副本”指已登记 physical replica，不是 Workspace Fork。
- “接续工作区”只指 continue_workspace，不指在新设备注册 ReplicaEpoch。
- “可靠保存”属于 D6，特指 strict 路径的 reliable，不等于同步完成或 portable publication。
- durable_observed_only 也是 D6 ReliableSaveState 的分支，只属于受信人工普通单 Document source-save；它不是 D3 replica_local 的同义词。
- D3 的 replica_local 只表示局部语义证明范围；create/move/reorder/Trash 等 D3 作者安装仍全部使用 WriteProtection=strict。
- Frontier/2 是已封存因果前缀，不等于 payload 已物化、placeholder 已下载或全集证明。
- SourceVersion/2 的 commitDomain 是生产域；SourceObservation/1 的 observerDomain 是当前观察域，二者不同不等于版本错误。
- “冲突”属于 D6 ConflictRecord 的 portable control，具体 parent/lifecycle/identity 解决由 D3。
- “执行责任”属于 D6/D10 control，不等于 D3 Authority。
- “来源”仍只属于 Source；Provenance 的中文仍是“来源证据”。
- “所有者”仍不等于 Authority。
- “路径”仍不等于 Node identity 或 parent。
- semantic_pending 不是“通过全部约束”的别名。
- D3 primary receipt 与 D3DecisionCompanion/2 不是两份成功 receipt；companion 只是同 P 决议的 D6 关联记录。

## 8. 接受边界

本 Lexicon 与 G0-B D3 主后像、D3 Implementation Impact 及同批 D1 后像必须共同审查。D4/D5 仍待 C 批；D7/D8/D9/D10 名称在各自 afterimage 完成前不由 D3 预先占用。

旧 D10 B13 仍为 REVISE、术语/双语 FAIL、3 P1 + 8 P2 共 11 OPEN；本文件不关闭任何项。
