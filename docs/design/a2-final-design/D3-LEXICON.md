---
source_language: zh-CN
translation_of: D3-LEXICON.zh-CN.md
translation_status: synced
---

[简体中文](D3-LEXICON.zh-CN.md)

Source document ID: b2595435-72ce-423a-805a-09993b04d1d5.

# A2 D3 Terminology and Naming Lexicon

Candidate status: A2 D3 current author candidate; not independently accepted, implemented, or activated. All 42 fixed-S conceptIds, four ownedNames sets, semanticNonalias ordering, and firstFreeze values are retained; only current definitions, producer/consumer versions, and review-state wording are source-qualified updates.

## 1. Authority and control rules

This Lexicon owns only the D3 terminology boundary. D6 public and technical producer names remain D6-owned. Fresh current D3 consumes `DependencyKey/3`, `DependencyProof/3`, `InputDescriptor/3`, `PreparedIntent/3`, `InstallationNotice/3`, `ContentCompletionProof/4` and `ChangeRecord/1`; `DecisionKey/2`, `CommitDomain/2`, `SourceVersion/2`, `SourceObservation/1`, `SourceVersionRef/1`, `ObservationScope/2`, `OwnerInputBinding/2`, `D3DecisionCompanion/2`, `RevisionTokenBinding/2` and specialized `SourceRevisionPlan/1|2|3` retain their real owner/version. Historical dependency/input/notice `/1-/2`, CP3 and wire9-12 dispatch only through recorded decoders/bytes/pins/recovery and are never re-encoded into current types.

Each concept has one stable concept ID, one canonical Chinese/English term pair, and one exact ownedNames set. semanticNonaliases participates only when an expected concept/type is known and is not a flat denylist. The fixed-S global-retired boundary remains and never scans user Document/Annotation prose.

## 2. Complete concept entries

### weftext.term.workspace

- Canonical names: 工作区 / Workspace.
- Definition: Portable aggregate, authorization/transaction boundary and D3 identity namespace; neither an author-content entity nor an open owner.
- Owner/layer: D2 domain boundary; D1 Core semantic authority.
- firstFreeze: frozen:D2.
- Migration/deletion: R0 deletes identifiers treating paths or database rows as Workspace identity.
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
- Definition: The only long-lived, independently referencable author-content entity in the D2 content graph, owning exactly one Document. Its exact-source classification may contain lexically valid Facet memberships: ordinary and template Nodes may carry non-tasks/task Facets; only ordinary plus exact built-in tasks/task satisfies the Task predicate.
- Owner/layer: D2 content graph; D3 NodeRef and lifecycle.
- firstFreeze: frozen:D2.
- Migration/deletion: R0 deletes old NoteId, ItemId and path-node aliases.
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
- Definition: Exactly one exact source and its semantic interpretation owned by a Node, addressed by that owning NodeRef and having no second durable identity.
- Owner/layer: D2 document content; D3 address/locator only.
- firstFreeze: frozen:D2.
- Migration/deletion: R0 deletes independent document IDs and sidecar identity.
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
- Definition: A locatable or reconstructible manifestation within an explicit owner/execution/rule scope, without durable identity by default; a meta-term, not a wire kind.
- Owner/layer: D2/D3 shared meta-term.
- firstFreeze: frozen:D2 的 Document occurrence 基线 + candidate:D3 的总称收窄.
- Migration/deletion: R0 prohibits bare OccurrenceId.
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
- Definition: Current-revision syntactic occurrences in Document exact source: headings, paragraphs, list/checklist items, table rows/cells, citations, bibliography placements, Saved Query/View definitions, AttributeCarrierBlock and LexicalAttributeEntry.
- Owner/layer: D2 Document content.
- firstFreeze: frozen:D2.
- Migration/deletion: R0 deletes durable block, heading and row IDs.
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
- Definition: An ordinary Node whose available Facet set contains exact built-in tasks/task; it reuses that Node's identity, Document, owner and lifecycle and creates no second kind or identity.
- Owner/layer: D2 v2 built-in Facet predicate; D3 freezes only NodeRef/lifecycle effects.
- firstFreeze: frozen:D2-v2.
- Migration/deletion: R0 deletes checklist/task mirrors, TaskId/TaskRef and old Task-specialization code.
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
- Definition: Composable capability membership in Node exact-source classification; membership may occur on ordinary or template Nodes and creates no entity, owner, identity, Document or lifecycle. Exact tasks/task is forbidden on templates and only makes an ordinary Node satisfy the Task predicate.
- Owner/layer: D2 v2 lexical membership and built-in tasks/task marker; D4/D10 own Registry, schema and anti-spoof rules.
- firstFreeze: frozen:D2-v2.
- Migration/deletion: R0 deletes Task paths that substitute a specialization enum for Facet membership.
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
- Definition: An exact-ASCII-code-point namespace/name lexical token in D2 v2 classification identifying a Facet contract; neither an EntityRef nor resolvable content identity.
- Owner/layer: D2 v2 lexical shape; D4/D10 own Registry, namespace reservation and validation.
- firstFreeze: frozen:D2-v2 lexical contract.
- Migration/deletion: D4/D10 separately freeze the Registry; FacetId never enters the EntityRef decoder.
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
- Definition: A namespace-scoped protected lexical block occurrence in D2 v2 exact source, with only an owning Document and current-revision ranges, no durable identity and no second payload authority.
- Owner/layer: D2 v2 Document lexical projection; D3 non-durable identity and artifact-binding boundary.
- firstFreeze: frozen:D2-v2.
- Migration/deletion: R0 deletes whole-namespace blob/sidecar authority; no carrier identity is added.
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
- Definition: An opaque raw lexical entry occurrence inside an Attribute Carrier Block, projecting only raw source and a current-revision source range; it interprets no FieldId, typed value, entry identity or provenance.
- Owner/layer: D2 lexical projection; D4 inner semantics.
- firstFreeze: frozen:D2-v2.
- Migration/deletion: Any future durable Field target requires D4 to formally reopen the contract; this occurrence cannot be reused for it.
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
- Definition: A non-Node object strictly owned by one Node, with durable owner-local identity and one author byte source, but no Document or Node capability.
- Owner/layer: D2 content object; D3 owner-local ref/lifecycle.
- firstFreeze: frozen:D2.
- Migration/deletion: R0 deletes AttachmentId, FileResourceId and path-as-id.
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
- Definition: A non-Node object strictly owned by one Node with owner-local identity and the frozen same-owner target union. Fresh current portable state is complete `D3-Annotation-Value/4`; when body is present it uses the single R6 `AnnotationInlineBody/1` + `AnnotationInlineProfile/1`. Historical Value/3/plain text remains historical decoder/recovery only.
- Owner/layer: D2 content object; D3 owner/ref/reply lifecycle.
- firstFreeze: frozen:D2.
- Migration/deletion: R0 deletes global/cross-owner Annotation IDs.
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
- Definition: An authored inline Document occurrence whose target intent is a same-Workspace Node; a reference slot, not a relation entity.
- Owner/layer: D2 authored occurrence; D3 NodeRef target/resolution.
- firstFreeze: frozen:D2.
- Migration/deletion: R0 deletes link-as-entity and path targets.
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
- Migration/deletion: D4/D7 own Library citation semantics; R0 deletes citation entity IDs.
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
- Migration/deletion: R0 deletes open ObjectRef/ItemRef and entity-kind coercion.
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
- Definition: Core's durable typed identity for equality, resolution and lifecycle continuity, expressed only by a complete Workspace/owner/domain-scoped ref.
- Owner/layer: D3 identity algebra.
- firstFreeze: candidate:D3.
- Migration/deletion: R0 prohibits bare UUID equality.
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
- Definition: A closed kind plus all namespace/owner fields forming an authoritative identity value; Ref is a controlled code suffix, not a generic natural-language term.
- Owner/layer: D3 wire/API.
- firstFreeze: candidate:D3.
- Migration/deletion: R0 deletes bare UUID public parameters.
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
- Definition: A resolvable semantic fact from a controlled source/slot to a typed target, with resolution/lifecycle state; not an independent durable relation entity.
- Owner/layer: D3 reference semantics.
- firstFreeze: candidate:D3.
- Migration/deletion: R0 deletes open fields that conflate relation, link and ref.
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
- Definition: A typed domain relation owned by D4; D3 requires every ref-valued endpoint to obey typed identity/resolution and does not freeze the relation ontology.
- Owner/layer: D4 owner; D3 negative identity gate only.
- firstFreeze: candidate:D3-boundary, owner D4.
- Migration/deletion: R0 deletes symbols that call all refs relations.
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
- Definition: A non-authoritative value binding owner and exact revision/coordinates to locate a non-entity occurrence/region; it may become stale and promises no identity continuity.
- Owner/layer: D3 location semantics.
- firstFreeze: candidate:D3.
- Migration/deletion: R0 deletes locator-registry-handle identity.
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
- Definition: The domain object determining legal containment, owner-local namespace and immutable ownership; in D3 a Resource/Annotation owner is exactly one NodeRef.
- Owner/layer: D2 ownership plus D3 ref algebra.
- firstFreeze: frozen:D2 + candidate:D3 encoding.
- Migration/deletion: R0 deletes ambient-owner inference.
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
- Definition: The role or instance making the final decision and commit for a semantic state; content authority and control authority require qualifiers.
- Owner/layer: D1 Core authority plus D3 Workspace/source authority.
- firstFreeze: frozen:D1 role + candidate:D3 instance semantics.
- Migration/deletion: R0 renames master/slave and owner-as-authority symbols.
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
- Migration/deletion: R0 deletes bare source_id.
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
- Definition: A control-plane fact binding one external source instance, authorized scope/collection and mapping namespace into the comparison domain for foreign keys.
- Owner/layer: D3 identity/provenance gate; D6/D9 persistence.
- firstFreeze: candidate:D3.
- Migration/deletion: D6/D9 define wire/persistence; R0 prohibits provider tokens as binding keys.
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
- Definition: Non-authorizing evidence explaining where an object/result came from and through which operation; it creates neither identity equality nor write permission.
- Owner/layer: D3 evidence boundary; D7/D9 payloads.
- firstFreeze: candidate:D3.
- Migration/deletion: D7/D9 define payloads; R0 deletes provenance-as-authorization.
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
- Definition: A durable, auditable, source-scoped mapping from one exact ForeignIdentityKey to a Weftext authoritative ref for deterministic re-import/adoption deduplication; association does not prove identity equality.
- Owner/layer: D3 identity/provenance invariant; D6/D9/A2 persistence and retention.
- firstFreeze: candidate:D3.
- Migration/deletion: D6/D9/A2 define wire/retention; R0 prohibits title/path deduplication.
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
- Definition: A reconstructible non-durable occurrence from an external series rule, occurrence key and exact version/rule cut; it has no Node identity by default.
- Owner/layer: D3 foreign-identity boundary.
- firstFreeze: candidate:D3.
- Migration/deletion: D4/D9 decide Calendar value mapping; R0 prohibits default per-occurrence Nodes.
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
- Definition: A source-scoped lookup key comprising SourceBinding, foreign component kind, foreign persistent key and an optional occurrence discriminator; it is not Weftext content identity.
- Owner/layer: D3 foreign-identity boundary.
- firstFreeze: candidate:D3.
- Migration/deletion: D6/D9 define wire; R0 deletes UID-as-NodeId.
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
- Definition: A non-identity value or occurrence target pointing outside Weftext authority; downstream owners determine resolution/fetch capability.
- Owner/layer: D2 link boundary plus D3 non-identity gate.
- firstFreeze: candidate:D3 boundary.
- Migration/deletion: D4/D9 decide typed URI/import contracts; R0 deletes URI-as-resource-id.
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
- Definition: An identity operation preserving source and creating fresh target identity with an explicit mapping; neither same-Workspace nor cross-Workspace copy preserves Node or owner-local identity.
- Owner/layer: D3 identity operation.
- firstFreeze: candidate:D3.
- Migration/deletion: R0 deletes clone/duplicate commands or migrates them to copy.
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
- Definition: A Workspace operation creating fresh Workspace/Node/owner-local identities from a snapshot cut and returning the complete mapping; source continues to exist.
- Owner/layer: D3 Workspace activation/identity.
- firstFreeze: candidate:D3.
- Migration/deletion: R0 deletes the workspace-clone alias.
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
- Definition: Formal continuation/restore of the same Workspace identity and ledger only after authority-continuity proof.
- Owner/layer: D3 authority continuity.
- firstFreeze: candidate:D3.
- Migration/deletion: R0 deletes path/digest auto-continue.
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
- Definition: A same-Workspace Node parent/order change preserving NodeRef; a cross-Workspace user intent requires fresh-target transfer plus source disposition and is not identity-preserving move.
- Owner/layer: D3 structure/lifecycle operation.
- firstFreeze: candidate:D3.
- Migration/deletion: R0 deletes cross-Workspace move-preserve code.
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
- Definition: Artifact/source materialization creating Weftext-managed fresh identity unless an explicit formal continue/fork/restore class applies. RFC 5545 foreign first-import identity intent is exactly initial_import: only never_bound after bounded preview creates a fresh Node and the sole active OriginBinding in the same author decision; retired binding requires explicit Adopt.
- Owner/layer: D3 identity effect; D9 mapping/preview.
- firstFreeze: candidate:D3.
- Migration/deletion: D9 defines the contract; R0 deletes provider/path-derived IDs.
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
- Definition: Explicit user selection of a foreign object/occurrence as an independent Weftext object. Creating a Node requires a fresh NodeRef and the sole active OriginBinding for the same ForeignIdentityKey in the same author decision. This is the only verb allowed to explicitly rebuild that sole active binding after retirement.
- Owner/layer: D3 identity/provenance effect; D9/A2 workflow.
- firstFreeze: candidate:D3.
- Migration/deletion: D9/A2 define workflow; R0 deletes foreign-ID coercion.
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
- Definition: Explicit conversion of a managed non-durable occurrence into a fresh ordinary Node while atomically updating the original occurrence; checklist-to-Task promotion also declares exact built-in tasks/task, uses a fresh NodeRef and creates no mirror.
- Owner/layer: D2 v2 checklist-to-ordinary-Node plus tasks/task semantics; D3 fresh-identity effect.
- firstFreeze: frozen:D2 + candidate:D3 identity effect.
- Migration/deletion: D7 defines payloads; R0 deletes in-place occurrence-ID upgrades.
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
- Definition: A downstream workflow establishing continued observation of external authority; it does not turn foreign identity into Node identity by default.
- Owner/layer: D9/A2 workflow; D3 negative identity gate.
- firstFreeze: candidate:D3-boundary.
- Migration/deletion: D9/A2 define workflow; R0 deletes subscribe-as-import.
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
- Definition: A downstream state-exchange/convergence workflow under explicit authority, binding and conflict policy; the verb itself chooses neither identity nor authority.
- Owner/layer: D6/D9/A2 workflow; D3 negative identity gate.
- firstFreeze: candidate:D3-boundary.
- Migration/deletion: D6/D9/A2 define the protocol; R0 deletes identity-changing sync side effects.
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
- Owner/layer: D1 capability surface plus D9/A2 connector workflow; D3 negative gate.
- firstFreeze: candidate:D3-boundary.
- Migration/deletion: D9/A2 define workflow; R0 deletes connect-implies-import behavior.
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
- Definition: The optional wire11 top-level token wrapper binding D7's complete immutable preparation conditions to the original requestFingerprint; retained on applicable fresh-current wire13 paths; genuine historical wire11/12 records keep their recorded recovery.
- Owner/layer: D3 request/wire wrapper; D7 backing payload schema.
- firstFreeze: D7联合revision03候选.
- Migration/deletion: Explicitly introduced in wire11; old v9/v10 replay only under their original decoder. Implementation synchronizes decoder, encoder, schema, caller and fixtures without silently inserting members. Actual wire11/12 records retain their original recovery while fresh-current dispatch is wire13.
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
- Definition: The wire11-introduced tenth plan array binding a saved payload's source/result occurrences and every typed slot exactly; retained by fresh-current wire13 while genuine historical wire11/12 records keep their original decoder.
- Owner/layer: D3 identity-mutation/wire; D7 saved-definition payload schema.
- firstFreeze: D7联合revision03候选.
- Migration/deletion: Explicitly introduced in wire11; old v9/v10 replay only under their original decoder. Implementation synchronizes decoder, encoder, schema, caller and fixtures without silently inserting members. Actual wire11/12 records retain their original recovery while fresh-current dispatch is wire13.
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
- Definition: The Result/9 Q segment retaining the complete original payload and transfer, uniquely materialized by the original candidate map.
- Owner/layer: D3 Result/9 identity materialization; D7 payload decoder.
- firstFreeze: D7联合revision03候选.
- Migration/deletion: Explicitly introduced in wire11; old v9/v10 replay only under their original decoder. Implementation synchronizes decoder, encoder, schema, caller and fixtures without silently inserting members. Actual wire11/12 records retain their original recovery while fresh-current dispatch is wire13.
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

The D6 terminology registry `D6-Terminology/2`, machine Registry, and Control producer remain the only owners of the imported D6 concepts and technical versions below. Fresh-current D3 wire13 consumes these exact current producer contracts; this Lexicon neither copies their closed JSON shapes nor creates a generic owner-defined JSON escape hatch.

| D6 producer name/version | D3 consumption boundary | Forbidden interpretation |
|---|---|---|
| `DecisionKey/2` | wire13 current ledger plus version-routed replay/saved/planned/unseen lookup and same-P companion | new D3 identity, second ledger, or a second decision |
| `CommitDomain/2` | request, `DecisionKey/2`, receipt, production/observer qualification | Workspace identity, execution-responsibility domain, or global sequence |
| `ReplicaEpoch` | replica `CommitDomain/2` writer generation | `AuthorityInstanceId`, device identity, `continue_workspace`, or execution lease |
| `ChangeId/1` | real portable effect at P seal, `Frontier/2`, conflict heads | `OperationId`, EntityRef, pre-seal reservation, or global time |
| `Frontier/2` | expected/base cut, exact policy, proved `scope_dependencies`, current CP4+ChangeRecord1 and purge causality | payload materialization, complete Query, Registry completeness, or provider “synced” |
| `SourceVersion/2` | complete managed/external production version | current observer identity, bare revision, digest, or currentness proof |
| `SourceObservation/1` | complete local current observation for locator/evidence/input | second production version, transferable sender token, or file identity |
| `SourceVersionRef/1` | narrow local projection selecting one protected current Observation | portable production-version transport, permission ticket, or bare revision |
| `SourceStamp/1` | proposed managed-after address frozen in the original plan | current `SourceVersion/2`, `ChangeId/1`, public fresh Ref, or early Locator capability |
| `SourceRevisionPlan/1` | protected P version basis only when the plan will produce a managed after | public wire, second version ledger, delete/no-op version, or mutable retry counter |
| `RevisionTokenSource/2` | closed tagged source arm for the protected revision-token binding | bare `SourceStamp/1`, bare `SourceVersion/2`, or free owner JSON |
| `RevisionTokenBinding/2` | protected stable production-address binding from one opaque token to closed `RevisionTokenSource/2`; managed canonicality comes only from the winning plan/seal + original sealed-outbox association | current observer qualification, permission, public source version, caller-selected token, or replacement for historical opaque decoders |
| `d6_source_revision/2` | new protected revision-token profile tag only | replacement tag for `d6d`, `d6r`, `d6a`, or a new D3 Locator lexical form |
| `ObservationScope/2` | authorized potential observation upper bound | write set, permission by itself, or completeness proof |
| `DependencyProof/3` | fresh-current protected complete positive/negative dependency evidence | caller read set, partial I, free JSON, “no hits”, or larger-Frontier shortcut |
| `DependencyKey/3` | fifteen closed dependency-key carriers owned/versioned by D6, including `document_format` | arbitrary owner key or a generic completeness interface |
| `OwnerInputBinding/2` | exact D3 owner input selected by `protocolOwner=D3` and owner version | second request, callback, or D3 duplicate wrapper |
| `InputDescriptor/3` | fresh-current exact protected input and pin binding | D3 private duplicate schema or hash-only equality |
| `PreparedIntent/3` | D6 fresh-current prepared-control record on applicable current paths | D3 planned decision, content identity, or automatic success |
| `D3DecisionCompanion/2` | same-P D6 association for one D3 primary decision | second success receipt, second ledger, new write scope, or source-version table |
| `WriteProtection` | installation-protection axis `strict|observed_only` | semantic guarantee, permission, or synonym for `replica_local` |
| `SemanticState` | `complete_semantics|semantic_pending` after D2-valid author semantics | permission, external-invalid state, or complete-set proof when obligations remain pending |
| `ContentGuarantee` | semantic/managed barrier axis `replica_local|managed_atomic` | `WriteProtection`, collaboration mode, or authorization |
| `InstallationNotice/3` | fresh-current immutable pre-install component record using the original base Frontier | commit proof or a new `ChangeId/1` for the not-yet-sealed decision |
| `ContentCompletionProof/4` + `ChangeRecord/1` | fresh-current post-seal portable proof/causal index with production-version `sourceChanges` | receipt permission, private `DependencyProof/3`, complete Query/Action, Approval/Money, or execution takeover |
| `ConflictRecord/2` | current new-FA portable conflict record with `Frontier/2` created cut | automatic merge, LWW, D3 bypass, or fabricated external/unknown head |
| `ConflictKey/1` | unchanged complete conflict key underlying `ConflictId` | record-version discriminator or independently mutable identity |
| `ReliableSaveState` | D6 saved-state axis including strict reliable and qualified `durable_observed_only` | Draft persistence, sync upload, or portable-publication completion |
| `ExecutionResponsibilityRecord` | durable D6/D10 continuity container where the contract actually uses it | ordinary content authority, replica registration, or replacement for D10 Run/Money/unknown contracts |

`SourceVersion/2` and current `SourceObservation/1` deliberately occupy different domains. Both managed and external production versions bind the full EntityRef, production `CommitDomain/2`, and production `observationEpoch`; managed additionally carries the checked production-domain revision and seal `ChangeId/1`, while external carries `externalSequence` and no managed revision/ChangeId. Current observation separately binds `observerDomain`, current `observationEpoch`, `FileObjectBinding`, and evidence pins. `SourceVersionRef/1` selects that complete local Observation. Equal revision numbers across production domains are incomparable; a foreign revision, foreign production epoch, or `externalSequence` never donates the managed after revision. `H(D,E)` is the greatest managed revision for entity E in production domain D's continuous sealed history; a production observation-generation change does not reset H, missing/gapped history is not empty, and a true raw no-op or source-unchanged structure/lifecycle effect does not advance H. Equal-byte external admission remains a real external-before/managed-after admission.

A plan that will really produce a managed after freezes one `SourceRevisionPlan/1` plus exactly one proposed `RevisionTokenBinding/2` before C/Q or other revision-bound materialization. The binding's `source` is the closed `RevisionTokenSource/2`; it carries no current observerDomain/current observationEpoch. The winning planning CAS freezes that token with the version basis and pins, and the single P seal authenticates that exact binding as the resulting managed SourceVersion's sole canonical stable address through the original sealed-outbox association, even if no Locator yet exists. Loser/aborted/unproved bindings never borrow another seal. A later observer resolves the stable address and separately proves a complete current `SourceObservation/1` whose `sourceVersion` equals it; this can qualify only a new read and never revives old selectors, PAB/Draft/map, ActionEvidence or PreparedIntent. Planning still allocates no `ChangeId/1`; deletion/no-op/source-unchanged branches retain their existing no-after-version rules. D3 Locator lexical shapes and historical decoders remain unchanged.

`DependencyProof/3` has exactly fifteen `DependencyKey/3` kinds: `source`, `document_format`, `lifecycle`, `placement_range`, `ref_inbound`, `relation_incidence`, `calendar_scope`, `registry`, `temporal_rules`, `authorization`, `foreign_binding`, `query_scan`, `replica_registry`, `conflict_record`, and `execution_resource`. D6 owns the closed carrier kinds, ordering, stamp/pin persistence, and commit/recovery validation; the real enumeration semantics stay with their actual owners. For D3 `placement_range`, `StructureRange` has exactly nine variants: `live_children`, `trash_children`, `trash_roots`, `ancestor_chain`, `subtree`, `owner_resources`, `owner_annotations`, `reply_closure`, and `restore_membership`. Empty and non-empty proof use the same complete-enumeration standard. A placeholder, unknown decoder, missing shard, I/O failure, hidden unauthorized member, building/partial I, provider status, equal count/hash, or no hit never proves a complete empty set. D4 keeps relation/Calendar/Registry semantics and D7 keeps complete Query semantics; D3 never replaces them with one coarse Frontier prefix.

Fresh-current portable publication uses `InstallationNotice/3` + `ContentCompletionProof/4` + `ChangeRecord/1`. Its `sourceChanges` carries complete production `SourceVersion/2|absent` before/after for actual source-state changes, while same-decision current D6 effect metadata may expose only local `SourceVersionRef/1` projections. Those two evidence layers never substitute for one another. `frontierBefore` and `frontierAfter` are the actual seal cuts; exact paths retain the original equal base, while `scope_dependencies` requires the complete continuous sealed extension plus revalidation of every original dependency and durable P evidence of unrelatedness. `InstallationNotice/3.baseFrontier` is never rewritten. A receiver validates Notice/proof/components/production history and then constructs its own `SourceObservation/1`/`SourceVersionRef/1`; it never copies the sender's `sourceToken` and gains no complete Query/Action qualification merely from the current CP4/ChangeRecord chain.

Current conflicts use `ConflictRecord/2` with `Frontier/2`, while `ConflictKey/1`, `ConflictId`, ordering, and the `D6-ConflictKey/1` hash domain remain unchanged. Every head is a real continuously verified sealed `ChangeId/1` relevant to the key. Historical `ConflictRecord/1` remains under `Frontier/1` and its original decoder/bytes; a record is not silently upgraded in place and /1 and /2 do not become two current records for the same unchanged key.

## 5. wire13 current D3 operation names and historical records

Existing D3 technical names such as `identity_operation_request`, `identity_change_receipt`, and `identity_operation_error` retain their D3 concept ownership. Fresh current native request is wire13, owner-kind `d3_identity_operation/13`, using `InputDescriptor/3`; historical wire9-12 retain original decoders. `d3_identity_operation/13` is the D3-owned owner-kind/canonical-descriptor name and does not re-own `OwnerInputBinding/2`. The D3 primary receipt keeps its existing twelve effect arrays and remains distinct from same-P `D3DecisionCompanion/2`.

Preparation Binding, Definition Transfer, and Definition Result Segment preserve their fixed-S concept identity, `ownedNames`, and `firstFreeze`. The outer `preparationBinding` remains a D3 request token wrapper selecting a D7-owned record; fresh current selects `PreparedActionBinding/4`, historical PAB1-3 retain exact recovery, and D3 creates no second D7 schema. `definitionTransfers`, the Result/9 `Q` segment, and the existing B/M/N/E/S/C/Q identity-mutation partition remain D3-owned, while the SavedQuery/View/DynamicBlock payload schema and D7 effects remain D7-owned.

For copy/fork/identity-bearing import, typed Definition Transfer traverses only the actual D7 typed roots: `DefinitionAddress.owner` plus the complete recognized Locator root, TypeSpec-declared typed Ref values recursively through their typed containers, `CollectionCreationPolicy.parent`, saved View/Query calls, Query selector literals, `QueryRef` arguments, defaults, fixed View-domain `TypedLiteral`, and DynamicBlock literal/context bindings. Ordinary CEL text, ordinary prose, and unknown JSON are not scanned. A complete Locator root already contains its owner and therefore does not emit an overlapping owner slot. Every recognized typed Ref/Locator has exactly one slot; an identified payload with none still has `slots=[]`. Source/result payloads and occurrences are independently pinned and verified, and `Q` remains one-to-one with the exact saved-definition payload span. Materialization uses one private candidate map and the same `SourceRevisionPlan/1`/revision binding through both position passes; it does not create a new wire or a second definition identity.

Copy always creates fresh mapped target identity; Resource/Annotation owner never changes in place. `copy_annotation` preserves the distinction between the fresh mapped Annotation's initial reply through the reference plan and the original mode's separately admitted destination-owner existing Annotation that may be an S container. Fork maps the complete required live+trashed closure under one identity map. `continue_workspace` preserves one logical Workspace only after exclusive-continuation/failover evidence proves the old authority stopped/fenced and either proves no committed post-cut facts or carries them in an authoritative ledger sufficient to merge them; replica registration is not Continue. Partial identity-bearing import remains artifact-authoritative and offline, while ordinary import worker/IR identifiers are never content identity.

Historical compatibility is record-based, not prototype-name-based. wire9/v10/v11 and any actually existing legacy `PreparedActionBinding/1,/2`, SourceObservation/ref/token, saved/planned/unknown record, receipt, pin, charge, approval/claim, or external-effect responsibility continue only under the decoder, bytes/fingerprint, original profile, authorization, custody, clock/TTL, pin lifetime, and no-duplicate-effect obligations that actually created them. The presence of a decoder, fixture, draft, or candidate prose alone does not prove that every historical prototype was deployed or active, and no old bytes are bulk-upgraded to the current producer contract.

For a saved/committed original decision, after original request/fingerprint/continuity lookup, current delivery authorization is checked only for that decision's original actual-effect/mode or result-disclosure scope. The original receipt/error/effects bytes or original-version publication/outbox are returned/resumed. A later r6 `SourceObservation/1`, Frontier, business proof, preparation/preview TTL, or new consumer gate never retrospectively rejects or re-runs the saved decision; revocation may hide delivery without rewriting history, reinstalling the old after, reallocating H/revision/ChangeId, or charging again. A planned record resumes only its original frozen request, its recorded InputDescriptor (fresh current `/3`, historical records their recorded version), candidate map where applicable, `SourceRevisionPlan/1`/version basis, pins, reservations/write set, its recorded InstallationNotice (fresh current `/3`, historical records their recorded version), `WriteProtection`, attempt/budget, preparation lifetime, and installation state. It never re-queries, reselects a target, resamples identity/H/revision, or prepares a second decision. Unknown installation or outcome retains original pins and Approval/Money/claim/external-effect/stop responsibility; current files, I, equal hashes, reauthorization, or an empty new control store never guess success/failure or authorize blind retry/refund/reset.

## 6. Controlled naming gate

Implementation and later owners mechanically verify:

1. all 42 inherited `conceptId` values exist uniquely;
2. every concept's four `ownedNames` surface sets remain exact-equal to fixed S;
3. every `firstFreeze` remains exact-equal to fixed S;
4. imported D6 public names and D6 internal technical names never enter D3 `ownedNames` merely because D3 consumes them;
5. no public identifier is owned by two concepts and no retired identifier enters an owned set;
6. ordinary prose, user content, third-party formats, and historical evidence are never a flat denylist input;
7. a new technical member with no real producer/owner mapping fails closed instead of being named ad hoc in D3;
8. D6 closed types are consumed at their real producer version; “owner-defined JSON” is never accepted as a substitute for current `DependencyKey/3`, `StructureRange`, revision binding, CP4+ChangeRecord1, conflict, or another closed interface;
9. bilingual prose uses the same protocol identifiers for the same meaning; documentation formatting does not rename a wire member, split one dotted member, or create a compatibility alias.

## 7. D6-FA-r01 terminology stress cases

- “replica” means a registered ordinary physical replica, not Workspace Fork, Workspace Continue, Authority, or execution-responsibility takeover.
- Ordinary/local/complete is a semantic-evidence axis. `strict|observed_only` is the independent `WriteProtection` installation axis. Every D3 identity/structure/lifecycle author operation, including create/move/reorder/Trash, remains `strict`; D3 has no weak identity-operation mode.
- `observed_only` belongs only to D6 trusted-human `interactive_source_save` when the human selects and freezes it before planning starts for exactly one existing live Document, `ordinary+replica_local` whole-source read/replace, with author-source writes empty or limited to that Document, no applicable body/Field/node-control deny, no identity/parent/order/lifecycle/shared-policy/Registry/Calendar/other-entity mutation, and Draft Base equal to the selected complete current `SourceObservation/1`. It is never fallback after a strict plan fails and adds no per-attempt approval workflow.
- On a qualified `observed_only` save, the actually read before B and user input N remain durable. An external C not observed after the final trusted check may be overwritten by N and may have no recoverable copy; a later C may become current again without erasing B/N. Observed competition, stale Base, watcher/event gap, revocation, third state, or unknown installation is outside that relaxation and follows conflict/reprepare/paused or `recovery_unknown`.
- Prepared or `inputRetentionState=retained` means only that the proposal, actually read before, and required bindings/pins were retained; it is not Saved. After seal, `durable_observed_only` remains distinct from strict `reliable` and from portable publication state.
- `semantic_pending` means D2-valid local facts passed while only obligations explicitly allowed to remain pending are unproved. It is never a synonym for D2 invalid, typed invalid, deny, empty range, complete Query/all_result, strong Action, Automation, bulk/collection, managed restore/purge/copy/fork/import, Server checkpoint, Approval, or Money success.
- Ordinary `.adoc` source and ordinary Resource bytes remain current author authority. Portable metadata remains the owner of identity, parent/order, lifecycle, shared policy/trust, Registry-related portable control, and portable change/conflict facts. P retains non-reconstructible decision/recovery/responsibility and required pins; I is discardable/rebuildable and never proves completeness by itself.
- `SourceVersion/2` production domain/generation and current `SourceObservation/1` observer domain/generation are distinct. Two replicas starting from one production before/source cut each construct their own complete local current Observation under their own operation `CommitDomain/2`; they never copy one another's `sourceToken`.
- `d6_source_revision/2` resolves only through `RevisionTokenBinding/2` and its closed `RevisionTokenSource/2`; it does not replace historical opaque D3 revision-token lexemes or D4/D5 inner selector/revision wire.
- Current `DependencyProof/3` complete emptiness is real proof, not an I miss. Deleting/rebuilding I does not reset intact protected proof continuity; a real gap or lost correctness evidence requires a new proof epoch and equal final hash does not revive the old one.
- Restore and purge have different memberships. Restore uses the original `restore_membership` for that Node closure, so owner-local Resource/Annotation objects trashed independently earlier stay trashed. Purge must cover all currently owned trashed members, including those earlier independently trashed objects, plus the real replica/inbound/control/other-owner strong proof.
- Copy/fork/import/Definition Transfer keep one candidate map, exact preimage/result evidence, owner-local rules, complete typed slots, and `Q` occurrence/span bijection. `copy_annotation` does not turn its fresh mapped primary into an S container. No ordinary text or unknown JSON UUID string becomes a typed Ref by scanning.
- `Frontier/2` is only a verified continuous sealed causal prefix. managed-atomic strong D3 paths remain exact. The four replica-local local-structure modes may use `scope_dependencies` only when the whole base-to-current sealed chain and every original source/control/auth/positive-negative dependency remain proved unrelated; `InstallationNotice/3.baseFrontier` is never moved forward.
- Current `ContentCompletionProof/4` + `ChangeRecord/1` transports/indexes production-version history after seal; a receiver creates its own current Observation and receives no complete Query/Action/execution qualification merely from the portable proof.
- “Conflict” remains D6 portable control. Current records use `ConflictRecord/2`; `ConflictKey/1`, `ConflictId`, and `D6-ConflictKey/1` remain stable, and an unsealed external/unknown condition receives no invented head.
- A saved historical decision is replayed under its original profile and current delivery authorization for its original effect/result scope. A planned decision restores only its frozen plan. An unknown outcome retains its original continuity and no-duplicate-effect obligations. None is bulk-upgraded or re-decided from current r6 state.
- Fixed C already contains the earlier-generation D3 wire12 and D4/D5 A/B/C consumer candidates that consumed the original G0-A/G0-B SourceObservation/Frontier/WriteProtection baseline. They are real candidate history, not nonexistent work, but they are unaccepted/unactivated and do not yet consume every new P1 producer rule in this Lexicon: production-domain H/revision planning and token binding, fifteen-key/range proof including document_format, CP4+ChangeRecord1, current ConflictRecord/2 boundary, and the M5 replay/recovery split still require their real owner/consumer afterimages where applicable. Missing such a strong consumer gates only the path that depends on it; it never permanently disables a qualified ordinary `.adoc`/Resource read, Draft, human whole-source save, or local offline operation that does not depend on that strong proof.
- “Execution Responsibility” remains D6/D10 control and is not D3 Authority, `CommitDomain/2`, or replica registration. `sourceOccurrenceKey`, ApprovalUse/Money, Run/Lease/Automation/deployment and external-unknown continuity remain with D10/respective owners.
- Chinese “来源” remains Source and “来源证据” remains Provenance. Owner remains distinct from Authority, and path remains distinct from content identity and Node parent.

## 8. Acceptance and activation boundary

This Lexicon is only the paired P2 D3 terminology afterimage. The two private D3 main files and this Lexicon are author candidates, not independent acceptance, implementation, activation, or release. The 42 fixed-S concept snapshots, their `conceptId`/`ownedNames`/`firstFreeze`, and fixed-S snapshots/inputs/catalog remain immutable read-only historical inputs. P2 synchronizes the real owner afterimages plus the three routing metadata files; it does not authorize rewriting those historical sources.

Fixed C already completed the prior-generation D4/D5 candidate consumer work for the original G0-A/G0-B baseline. The current P1 producer additions named above still require the applicable D3/D4 P2 and D5 P3 consumer afterimages plus the full D7 set and the established D8/D9/D10 owner/consumer set before any dependent managed/strong path can be coordinated. Their absence is not permission for partial activation and is not a global ban on unrelated qualified ordinary/local/offline content operations.

The coordinated D6 Control/Registry candidates now recognize the present native D3 descriptor/companion and state the exact InstallationNotice boundary: baseFrontier may contain historical sealed ChangeIds, while no new ChangeId for this unsealed decision is present. These producer repairs are candidate text subject to fresh joint acceptance; this Lexicon neither edits those producers nor activates a consumer.

The historical R08 three-P1/eight-P2 result remains review provenance; later named repairs and fixed-SHA bounded/limited reviews are evidence only within exact scopes. PL-IR-01 has prior bounded/limited closure at PR3 fixed e8aa within that accumulated review scope, but this is not fresh A2/global acceptance. A completed A2 still requires a new non-author global review on one fixed final SHA. This author self-closes no A2 finding and claims no product implementation, activation, release, or deployment. The fixed-446 D3 P1-01/P1-02 repair disposition in this head is resolved-pending-independent-review only; D1/D2 bounded closures from that report are not reopened.

## 9. Closed D3 adapter projections

The following are technical projections of existing controlled concepts, not new content identities or replacement firstFreeze values. D3ResolverInput/12 is a protected, immutable Ref/Locator read context with explicit Workspace/domain/exact Frontier and closed outcomes (main §11). The wire12 transfer observation uses complete target/source DecisionKeys and preserves the two independent Copy/Trash decisions (main §8.4.1); it never creates cross-Workspace Move identity.

D3ConflictResolutionPrepare/1 continues to prepare the existing Move/lifecycle/Copy semantics. Fresh current resolution uses `PreparedActionBinding/4`, `D3ResolutionInputUse/2` and `InputDescriptor/3` to retain complete ConflictKey/head/selection/branch evidence; final native request is wire13. Historical PAB3/Use1/wire12 preparation retains exact recovery. Full preview remains real D7 manifest/page/byte transport, not a PinRef access promise.

### 9.1 Canonical materialization and lifecycle selection

The current resolution binding explicitly composes selected canonical materialization with the original native operation. D3CanonicalInput/1 binds the selected production source and D6 ConflictInstallInput/1 actual installed before; it is not current source observation or ordinary write authority. Open-record retain-live/retain-Trash has an empty lifecycle transition but a portable conflict effect, with no opposite-head membership or fictitious inverse transition. A Resource's retained canonical claim may be materialized while a different head is copied fresh in the same plan; raw copy mode has no new overwrite slot. Exact net receipt effects and H/version rules are main §10.1.1.1–.2. The typed installation wrapper and conflict-only SourceRevisionPlan/2 are D6 producers; D3ResolutionInput/1 canonicalInputs and unremovable usage guard are D3 consumption, with D7 /3 persistence. No controlled concept JSON or firstFreeze changes.

### 9.2 Canonical evidence projection

D3CanonicalEffectPlan/1 is a protected whole-source-selection evidence namespace, not a raw native mode extension. D3CanonicalPlanProjection/1 publicly projects the exact plan with mandatory complete byte transport. D3CanonicalEffects/1 is a same-seal public semantic extension, not another receipt or decision: primary twelve arrays describe only native effects; canonical reference/S/lifecycle evidence has its own original comparator scope. Cross-component shared history is explicit, while physical source effects and H are deduplicated only by exact equality. Slot ordinal/kind pairing grants neither persistent slot identity nor Locator reanchoring. D7 transports both components completely and never treats unknown/missing extension as full success; saved commit remains historical fact even when current delivery fails. No controlled concept JSON or firstFreeze is changed.
