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

QuerySpec/1 is not widened. A real saved QuerySpec/1, if one exists, is decoded and executed under its exact /1 grammar and error rules. Copy, fork, import, export, QueryRef and historical recovery preserve the recorded QuerySpec version. There is no automatic rewrite or deployment migration from /1 to /2. Editing and explicitly saving as /2 creates a new definition revision under the ordinary D2/D3 rules.

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

node_file_name and node_relative_path consume the actual current D6 SourceObservation/1 for the Node and its present FileObjectBinding/1. The order is closed:

1. decode QuerySpec/2 and the Ref;
2. apply D3/D6 Ref-state disclosure;
3. require current source_read; node_relative_path additionally requires current structure_state before any placement-sensitive path disclosure;
4. obtain the complete current SourceObservation/1 and present FileObjectBinding/1 from the same observer cut;
5. validate PortableRelativePath and the ordinary D6 continuity/dependency proof;
6. derive the requested text and revalidate the complete Query barrier before publication.

node_file_name is the final nonempty PortableRelativePath segment after "/". It does not strip an extension, normalize Unicode, case-fold, percent-decode or use host path rules. node_relative_path is the exact canonical PortableRelativePath text and never a host-native path.

Absent, placeholder, conflict, gapped or otherwise unproved current binding is source_unavailable/proof_unavailable under the existing owner mapping. A hidden or unauthorized Ref is not_visible. Equal bytes, digest, basename or path from an old Observation never restores qualification.

## 5. D6 Resource file-name producer

resource_file_name is valid only from ResourceRef. Ref/owner disclosure precedes the existing resource_read gate. Core then obtains the Resource's complete current SourceObservation/1 and present FileObjectBinding/1 under the same cut and derives the basename exactly as in §4.

The source grants no Resource bytes beyond what resource_read already authorizes, and the projected basename grants no Resource identity, owner change, copy right or write authority. Missing or unmaterialized bytes/binding are unavailable rather than an empty name.

## 6. Same-cut dependencies and invalidation

Every metadata read contributes the real authorization dependency and the exact source/observation dependency that produced it. D2 title/subtitle also contributes document_format and the complete D2 evaluation dependency set. File metadata binds the exact SourceObservation/1 and FileObjectBinding/1. Selector/query_scan dependencies remain separate and still prove the selected visible population and its negative range.

All bindings of one read batch are from one complete Query cut. A rename, move or external rename that changes FileObjectBinding.relativePath invalidates the old result even when the Ref and file bytes are unchanged. A D2 metadata change invalidates title/subtitle through the D2 snapshot dependency. A structure change that changes a selected subtree invalidates through the ordinary placement/query_scan dependency even if one file path is unchanged.

None of these sources turns filename or path into identity, a Locator, Provenance, SourceVersion, ActionEvidence or write capability.

## 7. Save, copy, import and consumer dispatch

SavedQueryDefinition, DynamicBlock and QueryRef preserve the embedded QuerySpec version. Definition Transfer walks typed Refs/DefinitionAddress slots according to the version-specific schema and does not scan text, paths or filenames. D9 query_json copy/export/import preserves exact QuerySpec bytes and version unless the user explicitly authors a new revision. D8 editors and the D7 Search compiler emit QuerySpec/2 for new current definitions that require these sources.

A v1 decoder never accepts subtitle, node_file_name, node_relative_path, resource_file_name or union_all. Unknown future versions fail under the existing definition/version boundary. No second Query registry, Search executor or migration ledger is introduced.

## 8. Acceptance oracles

Positive cases include a titleless Node matched by node_file_name, a Node whose native title differs from its basename, a Node move that changes node_relative_path while preserving NodeRef, an absent subtitle represented as Optional none, and a Resource matched only by resource_file_name.

Negative cases include a hidden Ref, source_read/resource_read denial, path disclosure without structure_state, placeholder or gapped Observation, stale FileBinding after rename, an old QuerySpec/1 carrying a v2-only source, and union_all inputs with unequal schemas. None may fall back to an index value, shell filter, old Observation or empty result.
