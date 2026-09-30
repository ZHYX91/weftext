---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: b2595435-72ce-423a-805a-09993b04d1d5.

# D3 Terminology and Naming Lexicon — D6-FA-r01

Candidate status: D6-FA-r01; partial coordinated candidate; not accepted, not activated, not implemented. All 42 fixed-S concepts preserve conceptId, ownedNames, and firstFreeze. This afterimage deletes, renames, or transfers no existing controlled name.

## 1. Authority and control rules

This Lexicon still owns only the terminology boundary for D3 identity/reference/ownership/lifecycle. D6-FA-r01 names CommitDomain, ReplicaEpoch, ChangeId, Frontier, SourceVersion, SemanticState, ContentGuarantee, ConflictRecord, ReliableSaveState, and ExecutionResponsibilityRecord remain owned by D6 terminology. D3 consumes them and creates no synonymous concepts here.

Each concept has one stable concept ID, one canonical Chinese/English term pair, and one exact ownedNames set. semanticNonaliases participates only when an expected concept/type is known and is not a flat denylist. The fixed-S global-retired boundary remains and never scans user Document/Annotation prose.

## 2. Complete concept entries

### weftext.term.workspace

- Canonical names: 工作区 / Workspace.
- Definition: Portable aggregate, authorization/transaction boundary, and D3 identity namespace; not an author-content entity.
- Owner/layer: D2 domain boundary; D1 Core semantic authority.
- firstFreeze: frozen:D2.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 节点 / Node.
- Definition: The only long-lived independently referencable Document-bearing author-content entity in the D2 content graph.
- Owner/layer: D2 content graph; D3 NodeRef and lifecycle.
- firstFreeze: frozen:D2.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 文档 / Document.
- Definition: One exact source plus semantic interpretation existentially owned by a Node, with no second durable identity.
- Owner/layer: D2 document content; D3 address/locator only.
- firstFreeze: frozen:D2.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 出现项 / Occurrence.
- Definition: A manifestation within an explicit owner/execution/rule scope that is locatable or reconstructible but not durable identity by default.
- Owner/layer: D2/D3 shared meta-term.
- firstFreeze: frozen:D2 的 Document occurrence 基线 + candidate:D3 的总称收窄.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 文档出现项 / Document Occurrence.
- Definition: A current-revision syntactic occurrence in Document exact source, including headings, paragraphs, lists, tables, citations, saved definitions, carriers, and lexical entries.
- Owner/layer: D2 Document content.
- firstFreeze: frozen:D2.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 任务 / Task.
- Definition: An ordinary Node whose available Facet set contains exact built-in tasks/task; it reuses Node identity and lifecycle.
- Owner/layer: D2 v2 Task predicate; D3 NodeRef/lifecycle effect.
- firstFreeze: frozen:D2-v2.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 分面 / Facet.
- Definition: Composable Node exact-source capability membership that creates no second entity, owner, identity, Document, or lifecycle.
- Owner/layer: D2 lexical membership; D4/D10 registry/schema ownership.
- firstFreeze: frozen:D2-v2.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 分面标识符 / FacetId.
- Definition: Exact namespace/name lexical token identifying a Facet contract, never an EntityRef.
- Owner/layer: D2 lexical shape; D4/D10 registry ownership.
- firstFreeze: frozen:D2-v2 lexical contract.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 属性载体块 / Attribute Carrier Block.
- Definition: Namespace-scoped protected lexical block occurrence in D2 exact source with current-revision ranges only.
- Owner/layer: D2 lexical projection; D3 non-durable identity boundary.
- firstFreeze: frozen:D2-v2.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 词法属性条目 / Lexical Attribute Entry.
- Definition: Opaque raw lexical entry occurrence inside an Attribute Carrier Block, without its own durable identity.
- Owner/layer: D2 lexical projection; D4 inner semantics.
- firstFreeze: frozen:D2-v2.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 资源 / Resource.
- Definition: Owner-local durable non-Node object with one author byte source and no Document or child-Node capability.
- Owner/layer: D2 content object; D3 owner-local ref/lifecycle.
- firstFreeze: frozen:D2.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 批注 / Annotation.
- Definition: Owner-local durable non-Node auxiliary object with plain-text body, frozen target union, and reply lifecycle.
- Owner/layer: D2 content object; D3 owner/ref/reply lifecycle.
- firstFreeze: frozen:D2.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 节点链接 / Node Link.
- Definition: Authored Document inline occurrence whose target intent is a same-Workspace Node.
- Owner/layer: D2 authored occurrence; D3 NodeRef target/resolution.
- firstFreeze: frozen:D2.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 引文 / Citation.
- Definition: Authored citation occurrence whose target intent is a same-Workspace Node; bibliography presentation adds no identity.
- Owner/layer: D2 authored occurrence; D3 target identity; D4/D7 citation semantics.
- firstFreeze: frozen:D2.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 实体 / Entity.
- Definition: Closed meta-union of Node, Resource, and Annotation durable content identities; never a fourth kind.
- Owner/layer: D3 identity algebra.
- firstFreeze: candidate:D3.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 权威身份 / Authoritative Identity.
- Definition: Durable typed identity used by Core for equality, resolution, and lifecycle continuity.
- Owner/layer: D3 identity algebra.
- firstFreeze: candidate:D3.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 类型化引用 / Typed Reference.
- Definition: Closed kind plus all namespace/owner fields forming an authoritative identity value.
- Owner/layer: D3 wire/API.
- firstFreeze: candidate:D3.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 引用事实 / Reference.
- Definition: Resolvable semantic fact from a controlled source/slot to a typed target, with lifecycle and resolution state.
- Owner/layer: D3 reference semantics.
- firstFreeze: candidate:D3.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 关系 / Relation.
- Definition: D4-owned typed domain relation; D3 only enforces typed identity/resolution for ref-valued endpoints.
- Owner/layer: D4 owner; D3 negative identity gate only.
- firstFreeze: candidate:D3-boundary, owner D4.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 定位器 / Locator.
- Definition: Owner/revision/coordinate-bound non-identity value used to locate an occurrence or region and allowed to become stale.
- Owner/layer: D3 location semantics.
- firstFreeze: candidate:D3.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 所有者 / Owner.
- Definition: Domain object that determines legal containment, owner-local namespace, and immutable ownership.
- Owner/layer: D2 ownership plus D3 ref algebra.
- firstFreeze: frozen:D2 + candidate:D3 encoding.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 权威方 / Authority.
- Definition: Role or instance that makes the final decision and commit for a qualified semantic state.
- Owner/layer: D1 Core authority plus D3 Workspace/source authority.
- firstFreeze: frozen:D1 role + candidate:D3 instance semantics.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 来源 / Source.
- Definition: Original input/system that produces or carries content/evidence, always used with a qualifying domain phrase.
- Owner/layer: D2 exact source plus D3 provenance/import boundary.
- firstFreeze: frozen:D2 exact-source + candidate:D3 boundary.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 来源绑定 / Source Binding.
- Definition: Control-plane fact binding one external source instance/scope to the comparison namespace of foreign keys.
- Owner/layer: D3 identity/provenance gate; D6/D9 persistence.
- firstFreeze: candidate:D3.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 来源证据 / Provenance.
- Definition: Non-authorizing evidence explaining where an object/result came from and through which operation.
- Owner/layer: D3 evidence boundary; D7/D9 payloads.
- firstFreeze: candidate:D3.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 原始对象绑定 / Origin Binding.
- Definition: Durable auditable source-scoped mapping from ForeignIdentityKey to a Weftext authoritative ref, without identity equality.
- Owner/layer: D3 identity/provenance invariant; D6/D9 persistence.
- firstFreeze: candidate:D3.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 派生出现项 / Derived Occurrence.
- Definition: Reconstructible non-durable occurrence produced from an external series rule, occurrence key, and exact rule/version cut.
- Owner/layer: D3 foreign-identity boundary.
- firstFreeze: candidate:D3.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 外部身份键 / Foreign Identity Key.
- Definition: SourceBinding-scoped external lookup key composed from foreign component kind, persistent key, and optional occurrence discriminator.
- Owner/layer: D3 foreign-identity boundary.
- firstFreeze: candidate:D3.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 外部 URI / External URI.
- Definition: Non-identity value or occurrence target pointing outside Weftext authority.
- Owner/layer: D2 link boundary plus D3 non-identity gate.
- firstFreeze: candidate:D3 boundary.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 复制 / Copy.
- Definition: Identity operation that preserves source and creates fresh target identity plus explicit mapping.
- Owner/layer: D3 identity operation.
- firstFreeze: candidate:D3.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 工作区分叉 / Workspace Fork.
- Definition: Workspace operation producing a fresh Workspace and fresh mapped content identities from a snapshot cut.
- Owner/layer: D3 Workspace activation/identity.
- firstFreeze: candidate:D3.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 工作区接续 / Workspace Continue.
- Definition: Exclusive continuation/failover/disaster-recovery operation preserving one Workspace identity only after continuity proof.
- Owner/layer: D3 authority continuity.
- firstFreeze: candidate:D3.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 移动 / Move.
- Definition: Same-Workspace Node placement change preserving NodeRef.
- Owner/layer: D3 structure/lifecycle operation.
- firstFreeze: candidate:D3.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 导入 / Import.
- Definition: Artifact/source materialization creating Weftext-managed fresh identity unless an explicit formal identity-preserving class applies.
- Owner/layer: D3 identity effect; D9 mapping/preview.
- firstFreeze: candidate:D3.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 采纳 / Adopt.
- Definition: Explicit user choice to materialize a foreign object as fresh Weftext identity with an OriginBinding when required.
- Owner/layer: D3 identity/provenance effect; D9 workflow.
- firstFreeze: candidate:D3.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 提升 / Promote.
- Definition: Explicit conversion of a managed non-durable occurrence into a fresh ordinary Node while atomically replacing the occurrence.
- Owner/layer: D2 promotion semantics plus D3 fresh-identity effect.
- firstFreeze: frozen:D2 + candidate:D3 identity effect.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 订阅 / Subscribe.
- Definition: Downstream external-authority observation workflow that does not itself create Node identity.
- Owner/layer: D9 workflow; D3 negative identity gate.
- firstFreeze: candidate:D3-boundary.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 同步 / Sync.
- Definition: Downstream state-exchange/convergence workflow that does not itself choose identity or authority.
- Owner/layer: D6/D9 workflow; D3 negative identity gate.
- firstFreeze: candidate:D3-boundary.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 连接 / Connect.
- Definition: Provider/account/network setup operation with no direct content-identity effect.
- Owner/layer: D1 capability plus D9 connector workflow; D3 negative gate.
- firstFreeze: candidate:D3-boundary.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 准备绑定 / Preparation Binding.
- Definition: D3 request token wrapper selecting a D7-owned immutable preparation record; not content identity or permission.
- Owner/layer: D3 request/wire wrapper; D7 backing payload schema.
- firstFreeze: D7联合revision03候选.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 定义转移计划 / Definition Transfer.
- Definition: D3 identity-mutation plan binding saved-definition source/result occurrences and all typed reference slots.
- Owner/layer: D3 identity-mutation/wire; D7 saved-definition payload schema.
- firstFreeze: D7联合revision03候选.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

- Canonical names: 定义结果分段 / Definition Result Segment.
- Definition: Result/9 Q segment carrying one complete saved-definition payload and its transfer materialization evidence.
- Owner/layer: D3 Result/9 identity materialization; D7 payload decoder.
- firstFreeze: D7联合revision03候选.
- Migration/deletion: fixed-S migration/deletion remains; D6-FA-r01 does not weaken it.
- Controlled sets:

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

## 3. retired identifiers and semantic non-aliases

The fixed-S retired-controlled-identifiers exact set remains. Existing retired identifiers such as NodeSpecialization::Task, TaskRef, DocumentRef, AttachmentId, FileRef, ObjectRef, OccurrenceId, origin_id, and source_of_truth continue to use their one fixed replacement/deletion target.

The copy/fork/continue/move/import/adopt/promote and subscribe/sync/connect boundaries are unchanged by multi-replica storage: file copying is not Workspace Fork/Continue, and replica registration is not a new Continue alias.

## 4. D6-FA-r01 imported names and true owners

The following names are owned by D6 terminology registry D6-Terminology/2. D3 wire12 consumes them as imported types/members:

| D6 concept | D3 consumption | Forbidden interpretation |
|---|---|---|
| CommitDomain | wire12 request, ledger key, receipt | new D3 authority/identity |
| ReplicaEpoch | replica CommitDomain | AuthorityInstanceId, device ID, continue token |
| ChangeId | receipt, Frontier, conflict heads | OperationId, EntityRef |
| Frontier | expectedFrontier, purge coverage | global time or index progress |
| SourceVersion/2 | locator/evidence currentness | bare revision or content identity |
| SemanticState | replica_local qualification | substitute for complete D4/D5 proof |
| ContentGuarantee | replica_local/managed_atomic | permission or collaboration mode |
| ConflictRecord | sync-conflict address | second D3 conflict ledger |
| ReliableSaveState | D6 decision state | proof portable publication completed |
| ExecutionResponsibilityRecord | D10 continuity | replica registration or content authority |

wire12 fields commitDomain, guarantee, expectedFrontier, and inputDescriptor are D6 closed-type consumption sites; they create no new D3 ownedNames. D3 performs only owner decoding and cross-field equality over them.

## 5. wire12 and legacy names

Existing D3 technical wire names such as identity_operation_request, identity_change_receipt, and identity_operation_error keep their concept ownership under wire12; a version-number change is not a new ontology concept.

Preparation Binding, Definition Transfer, and Definition Result Segment preserve their wire11 firstFreeze history. wire12 may reuse those names, but the new backing Prepared-record version belongs to the future D7 afterimage. D3 never silently renames or upgrades PreparedActionBinding/2 into a new schema.

D3-CJ/3, D3-Symbolic-Result/9, Annotation Value/3, Ref, and Locator controlled names retain existing ownership.

## 6. Controlled naming gate

Implementation and later owners mechanically verify:

1. all 42 inherited conceptId values exist uniquely;
2. each concept's four ownedNames surface sets are exact-equal to fixed S;
3. each firstFreeze is exact-equal to fixed S;
4. D6 imported names do not enter D3 ownedNames;
5. no public identifier is owned by two concepts;
6. retired identifiers never enter an owned set;
7. ordinary prose, user content, third-party formats, and historical evidence are not flat-denylist input;
8. a new wire12 technical member without a real owner mapping fails closed rather than being named ad hoc in implementation.

## 7. D6-FA-r01 terminology stress cases

- "replica" means a registered physical replica, not Workspace Fork.
- "Workspace Continue" means continue_workspace only, not registering ReplicaEpoch on a new device.
- "Reliable Save" is D6 and does not mean sync completion or portable publication.
- "Conflict" is portable D6 ConflictRecord control; typed parent/lifecycle/identity resolution remains D3.
- "Execution Responsibility" is D6/D10 control and is not D3 Authority.
- Chinese "来源" still belongs only to Source; Provenance remains "来源证据".
- Owner remains distinct from Authority.
- Path remains distinct from Node identity and parent.
- semantic pending is not an alias for all constraints passed.

## 8. Acceptance boundary

This Lexicon is reviewed with the D3 main afterimage and D3 Implementation Impact. D3 does not pre-own future D4/D5/D7/D8/D9/D10 names before their owner afterimages exist.

The old D10 B13 result remains REVISE, terminology/bilingual FAIL, with 3 P1 and 8 P2 findings, 11 OPEN in total. This file closes none of them.
