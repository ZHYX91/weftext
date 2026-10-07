---
source_language: zh-CN
translation_of: D7-QUERY-V2.zh-CN.md
translation_status: synced
---

[简体中文](D7-QUERY-V2.zh-CN.md)
# A2 D7 QuerySpec/2 and current metadata sources

Status: author repair for A2-D7-2F89-P1-02. This is a candidate current-schema successor and still requires independent review.

## 1. Version boundary

QuerySpec/2 is the current author schema for newly authored A2 queries. It keeps the QuerySpec/1 top-level members, parameter rules, DAG rules, CEL profile, result families and every v1 operator except where this document explicitly adds the current read sources and union_all. The outer execution carrier remains wireVersion2.

QuerySpec/1 is not widened. A real saved QuerySpec/1, if one exists, is decoded and executed under its exact /1 grammar and error rules. QueryRef, reopen, historical recovery and D7 Definition Transfer dispatch the recorded embedded QuerySpec version. Copy/fork/import of a saved definition therefore preserves the author payload through the existing D3/D7 Definition Transfer path. There is no automatic rewrite or deployment migration from /1 to /2. Editing and explicitly saving as /2 creates a new definition revision under the ordinary D2/D3 rules.

D9 `query_json` is deliberately **not** a QuerySpec carrier. Its owner serializes the already-complete terminal D7 result as `weftext.query-result-export/1`; it preserves TerminalSchema, typed terminal data, bag/order semantics and author Refs in values, while removing runtime handles/paging. It neither copies nor migrates a saved Query definition.

## 2. Closed QuerySpec/2 additions

Within QuerySpec/2, the read operator keeps the exact v1 shape and accepts every v1 source plus exactly these current source meanings:

| Source | Applicable from | Exact output |
| --- | --- | --- |
| {kind:"title"} | NodeRef | Optional<text>; current D2 native title, none for a valid titleless Document |
| {kind:"subtitle"} | NodeRef | Optional<text>; current D2 native subtitle, none when absent |
| {kind:"node_file_name"} | NodeRef | text; exact basename of the current present D6 FileObjectBinding relativePath |
| {kind:"node_relative_path"} | NodeRef | text; exact current D6 PortableRelativePath using "/" separators |
| {kind:"resource_file_name"} | ResourceRef | text; exact basename of the current present D6 FileObjectBinding relativePath |

All other v1 read source shapes and types are unchanged in /2. In particular body_text still consumes the D2 semantic product; resource_descriptor still exposes no path.

QuerySpec/2 also adds exactly one generic relation operator:

~~~text
{id,op:"union_all",inputs:[relationId...]}
~~~

inputs has 2..16 relation IDs. Every input public schema must be byte-equal in column order, names and TypeSpec. union_all preserves every row and duplicate, performs no coercion or deduplication, evaluates every semantically reachable input, and returns an unordered bag. Any input error fails the Query by the ordinary canonical error ordering. Internal occurrence identity adds the canonical input ordinal before the parent K, so equal public rows from different branches remain distinct. union_all does not grant Action lineage across branches; a later Action still selects an explicit Ref and performs fresh authorization.

The stable inferred feature ID for this new generic operator is query.union_all.v1. It is a QuerySpec/2 feature, not a domain or Search-only operator. Generic join, window and quantile remain unsupported.

## 3. D2 title and subtitle producer

title and subtitle are read from the same current D2DocumentMetadata/3 inside one current D2DocumentSnapshot/3. Ref-state disclosure and the existing source_read gate precede source access. The snapshot root SourceObservation/1, processor environment, document_format dependency, includes and every dependency used by the metadata projection must be current at the final D7 read barrier.

The value is the D2 semantic value from that one evaluation. Filename, path, first paragraph, placeholder text and NodeRef never synthesize title or subtitle. A valid titleless Document returns none for title; a missing subtitle returns none. Invalid or unavailable D2 projection returns the ordinary unavailable/not_visible result after the normal disclosure order, never none pretending success.

## 4. D6 Node file metadata producer

`node_file_name` and `node_relative_path` consume only the D6-owned `D6FileBindingMetadataObservation/1` from D6 §19 / D6-SCHEMAS §11. D7 does not read FileObjectBinding directly and does not invent a path capability.

The order is closed:

1. decode QuerySpec/2 and the NodeRef;
2. apply the ordinary D3/D6 Ref-state disclosure;
3. D6 requires the existing `entity_state` and `locator_state` grants for that exact Ref, with deny-before-allow/default deny, before reading FileBinding locator metadata;
4. D6 establishes the complete current protected SourceObservation/1 and present FileObjectBinding/1 in one observer cut and returns only the protected metadata observation/value;
5. D7 validates the requested projection and binds that exact observation plus the current authorization dependency;
6. revalidate the complete Query barrier before publication.

Neither source requires `source_read`, `resource_read`, `structure_state` or `source_envelope_state` merely to disclose the FileBinding locator. A Query that separately reads body/source/Resource bytes still needs the original content capability for that separate read. The metadata producer may internally validate protected SourceObservation/FileObjectBinding evidence for correctness; those protected bytes/tokens are not exposed as Query cells.

`node_file_name` is the final nonempty PortableRelativePath segment after "/". It does not strip an extension, normalize Unicode, case-fold, percent-decode or use host path rules. `node_relative_path` is the exact canonical PortableRelativePath text and never a host-native path.

Absent, placeholder, conflict, gapped or otherwise unproved current binding is source_unavailable/proof_unavailable under the D6 owner mapping after authorization. A hidden or unauthorized Ref is not_visible before locator metadata is read. Equal bytes, digest, basename or path from an old Observation never restores qualification.

## 5. D6 Resource file-name producer

`resource_file_name` is valid only from ResourceRef and consumes the same D6-owned metadata producer with projection=basename. Ref/owner disclosure and the exact `entity_state` + `locator_state` gates precede FileBinding metadata access. It does **not** require `resource_read` merely to reveal the authorized locator label; a separate Resource byte/descriptor read still uses its original gate.

The projected basename grants no Resource identity, owner change, copy right, ByteHandle or write authority. Missing/unmaterialized/gapped binding is unavailable rather than an empty name.

## 6. Same-cut dependencies, direct consumers and union identity

Every metadata read contributes the real authorization dependency and the exact D6 protected SourceObservation/FileObjectBinding that produced it. D2 title/subtitle also contributes document_format and the complete D2 evaluation dependency set. Selector/query_scan dependencies remain separate and still prove the selected visible population and its negative range.

All bindings of one read batch are from one complete Query cut. A Policy/auth-generation change, rename, move or external rename that changes FileObjectBinding.relativePath invalidates the old result even when the Ref and file bytes are unchanged. A D2 metadata change invalidates title/subtitle through the D2 snapshot dependency. A structure change that changes a selected subtree invalidates through the ordinary placement/query_scan dependency even if one FileBinding path is unchanged.

None of these sources turns filename or path into identity, a D3 Locator, Provenance, SourceVersion, ActionEvidence or write capability.

### 6.1 QuerySpec/2 title consumers

Fresh QuerySpec/2 `title` is Optional<text>. Any retained current consumer that needs a nonoptional display label must adapt explicitly rather than reinterpreting the source type.

For the Temporal Query/View positive chain, the fresh /2 witness reads `title:Optional<text>`, retains that optional column in the terminal/a11y data, and derives `displayTitle = row.title.orValue('')` before the Calendar/Timeline projection. The View binds the nonoptional `displayTitle` column. Empty display text is only a presentation value: D8 may render a localized "untitled" placeholder while preserving the separate Optional title state. Neither the empty value nor a placeholder is persisted back as author title, participates in identity, or falls back to filename/path.

For link display, an explicit source label still wins. Otherwise a freshly authorized target-title read yields Optional<text>: some(v) projects v; none projects a titleless/empty display label. A target that is not readable continues to use the call site's already-known unavailable-link presentation state. Filename/path are never consulted to synthesize target title.

A genuine QuerySpec/1 definition keeps its exact /1 title type/decoder. It is not reinterpreted as Optional and receives no automatic migration merely because the current D2 product permits titleless Documents.

### 6.2 `union_all` LogicalOccurrenceKey and order

QuerySpec/2 extends the retained closed K tagged tree by exactly one constructor:

~~~text
{kind:"union_all",
 base:{invocation:I,operator:o},
 inputOrdinal:Counter,
 parent:K}
~~~

`inputOrdinal` is the zero-based index 0..15 of the corresponding entry in the author `inputs` array. Array order is semantic and therefore retained by canonicalization. Duplicate relation IDs in `inputs` are legal: `union_all [r,r]` emits two copies of every parent occurrence, tagged with inputOrdinal 0 and 1, so bag duplicates remain distinct even when public cells and parent K are byte-equal.

The shared upstream relation node is evaluated under the ordinary DAG once; each union input occurrence copies its complete rows into the union branch and charges the ordinary checked union output row/byte/work budgets. Parent relation errors keep their original canonical operator/error ordering; a union schema mismatch is a static QuerySpec/2 type/graph error at the union node. `union_all` itself is unordered, so `take` is invalid until an explicit `sort`. After sort keys compare equal, the full K tree including `inputOrdinal` supplies the retained deterministic internal tie; paging therefore cannot collapse or reorder duplicate branches.

`union_all` still erases implicit Action lineage. A later Action must select an explicit Ref and obtain fresh authorization; the new K is never public identity or ActionEvidence.

## 7. Save, copy, import/export and consumer dispatch

SavedQueryDefinition, DynamicBlock and QueryRef preserve the embedded QuerySpec version. Definition Transfer walks typed Ref/DefinitionAddress slots according to the version-specific schema and does not scan text, paths or filenames. Saved-definition copy/fork/import uses that D7 Definition Transfer plus the ordinary D3 author operation and preserves the embedded QuerySpec bytes/version unless the user authors a new definition revision.

D9 `query_json` remains a terminal-result export profile only: it serializes the complete result's TerminalSchema and data and never carries QuerySpec author bytes. Importing that result file therefore cannot create, migrate or refresh a saved Query definition.

D8 editors and the D7 Search compiler emit QuerySpec/2 for new current definitions that require these sources. A v1 decoder never accepts subtitle, node_file_name, node_relative_path, resource_file_name or union_all. Unknown future versions fail under the existing definition/version boundary. No second Query registry, Search executor or migration ledger is introduced.

## 8. Acceptance oracles

Positive cases include: a principal with `entity_state+locator_state` but no `source_read` matching a titleless Node by node_file_name; a Resource basename match without `resource_read`; a Node whose native title differs from its basename; a Node move that changes node_relative_path while preserving NodeRef; an absent subtitle represented as Optional none; a titleless temporal item whose QuerySpec/2 Calendar witness derives nonauthoritative displayTitle without writing a title; a titleless readable link target that never falls back to filename; and `union_all [r,r]` preserving two occurrences through explicit sort/paging.

Negative cases include a hidden Ref, missing `locator_state`, placeholder or gapped Observation, stale FileBinding after rename, content-read denial on a separate body/Resource-byte read, an old QuerySpec/1 carrying a v2-only source, union_all inputs with unequal schemas, and `take` applied directly to unordered union_all. None may fall back to an index value, shell filter, stronger content permission, filename-as-title, old Observation or empty result.

Saved-definition copy/fork/import must retain the recorded embedded QuerySpec version through Definition Transfer. D9 query_json must remain a terminal result serialization and must never be accepted as QuerySpec author source.
