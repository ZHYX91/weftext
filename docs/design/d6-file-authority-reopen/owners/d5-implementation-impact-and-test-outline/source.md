---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: 21e90c5d-66a2-476b-b852-5a77ac4e7f13.

# D5 Implementation Impact and Test Outline — D6-FA-r01

Candidate status: D6-FA-r01; partial coordinated candidate; not accepted, not activated, not implemented. Fixed-S D5 v1 implementation/acceptance obligations remain. This file adds validation for H2 D3/D6 and this batch's D4 local-versus-complete rules, SourceVersion/2, partial indexes, and multi-replica behavior.

## 1. implementation effect graph

~~~text
D2 exact source
  -> native table parser / current-revision locators
  -> D5 structured table intent
       -> local source transformation
       -> D6 SourceVersion/2 + MutationFootprint + install
  -> D4 Field Value Occurrence editor
       -> D4 Entry/Registry proof
  -> D7 Query -> NodeCollectionResult
       -> exploratory partial presentation
       -> strong collection/bulk operation
            -> future D7 complete cut / Prepared
            -> D3/D4/D6 managed commit
~~~

There is no durable Record/RecordCollection storage. Table parse, collection result, and membership candidate in I are rebuildable.

## 2. implementation slices

### 2.1 native Document Table

Requires exact-source table parser, current SourceVersion/2, table/row/cell locator validation, ragged-row model, representable-cell encoder, exact trivia/line-ending preservation, patch-overlap detection, and D6 file-install adapter.

Native table editor is not a D4 schema editor and does not require a whole-workspace Query.

### 2.2 D4 occurrence editor

A tabular property editor invokes D4 Entry/Occurrence semantics rather than creating a TableRow/Record. Each add/remove/reorder/note binds current source and D4 Registry.

### 2.3 Node Collection

Collection viewer consumes D7 result plus explicit coverage. Strong collection action waits for new D7 complete-result/Prepared contract. It never substitutes old Prepared/1,/2, a page, or a cache.

### 2.4 conversion/import/export

Row-to-Node/import mapping/loss belongs to D9 and fresh identity to D3. D5 provides only source-row domain. Cross-Workspace transfer keeps two independent receipts.

## 3. component impacts

| area | future work | forbidden |
|---|---|---|
| table parser | native D2 grammar + locators + ragged/trivia | row ID/database row |
| table editor | exact source patch + D6 install | sidecar/table DB |
| Field list UI | D4 Entry adapter | Record inference |
| collection engine | D7 result + coverage | page as membership |
| collection actions | frozen targets + complete cut | reevaluate all at commit |
| import | D9 map + D3 fresh Node | IR row ID as NodeId |
| I | parse/result cache | authority |
| P | decision/recovery only | membership/current source |

## 4. test outline

### 4.1 native source

1. simple cell edit preserves delimiter/header/footer.
2. ragged rows stay ragged.
3. missing trailing cells stay absent.
4. CRLF/LF preserve.
5. inline formatting and escaped separator preserve.
6. inter-row comments/blanks block structured reorder.
7. representable cell round-trips exactly.
8. unrepresentable cell returns `unrepresentable_cell` with zero write.
9. old-revision table locator is stale.
10. equal row text after external edit is not old occurrence continuity.

### 4.2 row/column operations

Cover row append/remove, batches through 1000, column add/remove over ragged rows, header/title edit, safe and unsafe reorder, overlapping-patch rejection, exact post-source reparse, SourceVersion CAS, and D6 crash recovery.

### 4.3 Field occurrence UI

Cover repeated equal D4 values as separate occurrences, note-to-selector binding, D4 type/cardinality/requiredness rejection, raw display of unavailable namespaces without typed mutation, local-valid/cross-object-pending behavior, and no TableRowId/RecordRef output.

### 4.4 collection membership

Cover NodeRef deduplication, Query auth/lifecycle filtering, multi-collection membership, definition deletion without Node deletion, 200-row transport page versus membership, semantic `take`, explicit partial coverage, placeholder not meaning absent, and I rebuild without author writes.

### 4.5 Collection Creation Policy

Cover valid/stale/not-visible/trashed parent, no root fallback, defaults, Template/default facts, requireMembership=false local create, requireMembership=true complete post-query, identical definition/params/auth/dependency reevaluation, exclusion of the new Node, and D7 Prepared unavailable.

### 4.6 Remove from collection

Cover invertible explicit refs, one D4 Field edit, noninvertible arbitrary filters, UI distinction from Trash, stale preview without retarget, and authorization change.

### 4.7 delete modes

Mechanically separate table-row deletion, D4 occurrence deletion, collection removal, D3 Trash, and managed_atomic purge. A generic Delete action never changes meaning silently after View changes.

### 4.8 batch and limits

For 0,1,200,201,999,1000,1001 targets, enforce target/page limits, narrower budgets, no sampling/truncation, frozen target set at commit, all_result complete cut, and rejection of partial mutation. Import separately tests 1000/1001 new Nodes.

### 4.9 cross-Workspace

Target-copy success plus source-Trash failure is explicit partial transfer rather than atomic move. Both receipts retain separate OperationId/CommitDomain/authority.

### 4.10 multi-replica / conflicts

Cover same-cell edits, different-row file-generation conflict, explicit three-way merge, concurrent collection-definition/member-fact changes, page/placeholder, equal hash with new observationEpoch, no mtime/LWW, and no hidden row identity.

### 4.11 r5/r6, I/P

Cover r5 pending(collection), later r6 complete result, exact r5 replay, I rebuild without upgrade, P loss without decision/approval reconstruction, and new device fresh replicaEpoch without execution custody.

### 4.12 partial index / complete action

Partial metadata/query index may render a local list but never proves complete membership, requireMembership, inverse removal, bulk/all_result, negative collection constraints, or Automation targets. A full source scan may produce complete cut only under the future D7 contract.

### 4.13 domain fixtures

People duplicate phone/name rows never become Records. Organizations inverse rows reach the authored D4 owner. Calendar derived occurrences remain derived. Library citation/work/resource rows preserve domains. Native table row conversion always creates a fresh Node.

## 5. legacy and terminology

- exact-preserve the six fixed D5 conceptIds/owners/owned names/firstFreeze;
- no TableRowId/RecordRef/CollectionRef/ViewRef;
- old D5 v1 meaning preserved;
- saved D7 PreparedActionBinding/1,/2 replay under legacy only;
- no guessed new D7 Prepared;
- historical D3 v9/v10/v11 and D6 wire1 decisions unchanged.

## 6. performance and resource validation

Future benchmarks record table row/cell count, source bytes, parser time, patch size, peak RAM, index coverage, collection candidate/result counts, page count, and cancellation. This design makes no seconds-level performance guarantee.

Large tables/workspaces remain bounded and ordinary local edits do not scan unrelated content merely to appear complete.

## 7. completion boundary

A future implementation slice requires Core/adapters/fixtures/bilingual docs/legacy replay plus real execution evidence. Documentation CI or author self-review is not product conformance.

Old D10 B13 remains REVISE, terminology/bilingual FAIL, eleven OPEN findings.

## 8. Preservation of fixed-S architecture impacts and complete regression obligations

This section incorporates the fixed-S D5 architecture impacts, Record-branch retirement, and complete test cases that remain effective. D6-FA-r01 only replaces outer file authority, SourceVersion/CommitDomain/semantic-pending/multi-replica consumption and deletes none of these semantic obligations.

### 8.1 Cross-stage architecture impact

D2 continues to provide native table/row/cell grammar, revision locators, exact-source local edit, with Saved Query/View as Node-local occurrences. No row identity, Record body, ViewRef, or read-time rectangular normalization.

D3 continues to provide NodeRef, owner-local Resource/Annotation, copy/fork/continue, Trash/restore, and receipts. New D6-FA decisions use wire12 but add no Record identityMap arm and never make occurrenceKey resolve across revisions.

D4 continues to provide Registry, FieldId, TypedValue, Facet/relation common post-state, raw-source preservation, and Entry/schema admission. Table columns never override Field schema, reintroduce a people blob, or create Entry Annotation.

D6 owns durable commit qualification for a physical replica/Server backend, SourceVersion/2, source patch, complete read-set/negative dependencies, safe installation, P seal/recovery, and fine-grained authorization. It creates no Record store and never widens conflict/permission through whole-namespace replacement.

D7 owns deduplicated Node results, typed/occurrence results, editable columns, creation/membership proof, frozen bulk targets, paging/revocation. The Record branch retires as a set and join/group rows do not become editable Nodes automatically.

D8 surfaces distinguish table-row deletion, Field-fact deletion, collection removal, and Trash; expand multi-value summaries and preserve conflict Draft. Grid row index never authorizes writes and Mobile capacity never changes semantics.

D9 explicitly maps CSV/Excel/JSON with loss, fresh import, finite batches, and separated Office binding domains. It does not execute external formulas, perform implicit upsert, or require export-only schema on ordinary Nodes.

D10 keeps external-source authority separate from cache and owns SourceBinding plus schema/contribution authenticity. Connector cache is not managed Record author storage and creates no second CRUD/reference domain.

### 8.2 Legacy Record branch retires as a whole

Future D7/implementation removes together:
- RecordCollectionRef/RecordRef TypeSpec/Query arguments;
- records scan domain, recordCollection selector, record-schema identity;
- record/node relation specialization;
- row.record.fields Field/CEL overloads;
- collection UUID and record compound-ref equality/group/distinct/cache/export branches;
- persistent Record occurrence/provenance seeds;
- record-schema permission/read-set/lens/action-target/View passthrough;
- corresponding decoders, capabilities, fixtures, locale/API aliases.

Preserved generic semantics are precise NodeRef/TypedValue typing, D4 FieldId/RegistryBinding, authorized Query envelope, target/revision re-resolution, non-durable derived rows, and ActionEvidence separation.

### 8.3 native-table complete checks

Cover pipe/backslash escaping, CRLF/CR/LF, duplicate rows, ragged tables, blank/comment trivia, empty table, stale locator, legal Inline/ref, unrepresentable text, untouched-byte equality, and zero author write on failure.

A structured patch reparses the complete proposed source; failed source CAS never partially saves.

### 8.4 Collection creation/membership

Cover path versus ordinary property predicates after move, empty-collection default parent, explicit parent for ad-hoc Query, Template conflict, append-ordinal concurrency, filter true but Query top/limit excluding the fresh Node, and definition/params/schema/auth changes invalidating an old plan.

Also cover requireMembership true/false and strict separation of transport page/cache from semantic membership.

### 8.5 Field and concurrency

Cover equal phone values with distinct notes, three historical assertions, same value with different occurrenceKeys, external-copy key collision, multiple carrier blocks, unknown namespace, deleting last required occurrence, inverse UI writing the true canonical owner, and stable D4 selector/read-set behavior.

Concurrency fixture: A edits one name note, B adds a phone, C has phone-only permission. Whole-namespace replacement is forbidden; old revision plan is not replayed directly; replan preserves unrelated facts; inability to prove yields retained conflict proposal. Delete/re-add same-key ABA never retargets automatically.

### 8.6 lifecycle and cross-Workspace

One Node in multiple collections is one identity. Hidden subtree appears in Trash preview, root Trash rejects, restore preserves identity, copy is fresh.

Cross-Workspace target copy and source Trash have two independent receipts. Target failure leaves source untouched. Source-Trash failure reports target copied / source retained. D5 never invents an atomic cross-Workspace move.

### 8.7 scale, limits, and batch recovery

Target fixtures cover 999/1000/1001, grid pages 199/200/201, wide rows hitting narrower byte/dependency budgets, and Template descendants counting toward import limits. Loaded count never means total and partial success never pretends complete.

Future large-import acceptance includes:
- 10,000 Nodes;
- 10 batches;
- workspace input larger than available working memory;
- crash before/after batch 6 commit;
- process restart;
- network loss;
- repeated resume.

Each batch is atomic; job reports exact partial committed state and never duplicates creation. This document does not claim those product tests have passed.

### 8.8 ICS and recurrence

Cover series rule/override, infinite recurrence under bounded query, external subscribe/sync versus adopt/import, same UID across different SourceBinding, cancel/delete/unbind/reimport/offline/timezone, no Record/implicit occurrence-Node materialization, VFREEBUSY/VTIMEZONE zero Node, and explicit VTODO->Task plus VJOURNAL/VEVENT mapping.

SourceBinding defines foreign-key comparison scope; active OriginBinding links ForeignIdentityKey to NodeRef. UID/RECURRENCE-ID/LogicalOccurrenceKey never becomes NodeRef.

### 8.9 Query / Office and zero-row semantics

Cover empty result with schema, nonterminal zero-row page not EOF, read-only join/aggregate columns, separate Office binding domains for ordinary Document table/Node collection/Query value table, zero export-only persistent config on ordinary Node, and explicit rejection of legacy Record token.

A partial collection export is marked partial or rejected by complete-export mode; it never silently truncates to transport page.

### 8.10 terminology and five-surface consistency

D5 controlled names have unique ownership. Document Table/row/cell, Node Collection, Field occurrence, derived row, and Resource preview stay distinct. Retired API is removed as a set while legal user content, historical prose, and third-party formats are not vocabulary-filtered.

Desktop/WebUI/Server/CLI/Mobile share the same Core semantics. Surface differences change interaction/capability only, never identity, membership, limits, or errors.

### 8.11 Additional D6-FA-r01 regression

On top of the fixed-S matrix cover:
- ordinary offline native-table edit reliably saves while unrelated I is incomplete;
- SourceVersion/2 stale/ABA;
- same/different-row multi-replica conflict;
- explicit three-way merge proposal;
- semantic_pending(collection) never feeds all_result/bulk;
- rebuilding I never restores old proof;
- losing P never restores batch decision/approval/Money;
- new replica preserves NodeRefs but gains no execution custody;
- r5 pending replay stays distinct from r6 current complete result;
- Server multi-user table/collection Drafts coexist while durable commits remain ordered by the same backend boundary.

### 8.12 acceptance boundary

These are future reviewable contract cases. Mechanical preflight, documentation CI, author self-review, and historical Pro review are not D6-D10, A2, or product-host implementation acceptance.

Old D10 B13 remains REVISE, terminology/bilingual FAIL, eleven OPEN findings.
