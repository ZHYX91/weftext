---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: b83a76f3-6d5d-463b-9f51-ed8144d24602.

# D5 Tables and Node Collections — D6-FA-r01

Candidate status: D6-FA-r01; partial coordinated candidate; not accepted, not activated, not implemented. Fixed-S D5 v1 is source and compatibility history. This complete D5 owner afterimage consumes H2 D3 wire12, H2 D6 SemanticState/SourceVersion/CommitDomain, and this batch's D4 afterimage. It creates no durable Record/RecordCollection domain.

Fixed provenance:
- S=7e18168dad3e6d120fce0dd607dc10fa7894e252
- source blob=166b1aebd43c00bb9c1152c567efbd46c7b5ffb8

## 1. Problem, choice, and upstream

D5 owns structural semantics for tables and Node collections, not Document bytes, Field schema, Node identity, parent/order, or Query execution.

The frozen choices remain:
1. D4 Field Value Occurrence is not a Record.
2. D2 Document Table Row/Cell is a current-revision Document occurrence with no durable row/cell identity.
3. Node Collection is a D7 Query result of live authorized NodeRefs, not parent, owner, stored membership list, or RecordCollection.
4. external/IR records gain no Workspace identity until explicit import/create makes fresh Nodes.
5. D5 defines no RecordRef, TableRowId, CollectionRef, ViewRef, or database row store.

P/I never becomes a second current truth for table rows, collection membership, Field occurrences, or Node parent/order.

## 2. Complete alternative comparison and row domains

| row-like thing | authoritative domain | identity/lifetime | allowed operation |
|---|---|---|---|
| D4 Field Value Occurrence | owning Document source + D4 Entry | revision-bound occurrenceKey selector; not durable identity | D4 Field edit |
| D2 Document Table Row | exact-source table occurrence | current-revision locator/ordinal | D5 native table edit |
| D2 Document Table Cell | Inline* occurrence in row | current-revision locator/column position | D5 native cell edit |
| NodeCollectionResult row | NodeRef | durable Node identity remains | D7/D5 collection Action |
| Import/worker row | external/IR | no Workspace identity | D9 preview/import; fresh Node on commit |

A tabular UI never changes these domains. Names/phones/addresses list-of-object values remain D4 typed values/occurrences instead of hidden Records.

## 3. Document Table

### 3.1 grammar and source authority

Document Table remains a D2 native AsciiDoc table occurrence whose author authority is `.adoc` exact source. D5 structured edit round-trips grammar, delimiters, cell separators, header/body/footer, ragged rows, multi-line content, block/inline boundaries, and original trivia.

Missing trailing cells are absent, not automatic empty cells. Cell content is Inline* and text such as `0012`, `true`, or a date-looking value never infers a D4 type.

Table/row/cell locators are revision-bound. Equal text/position under a new SourceVersion is not identity continuity. Parsed table state in I is rebuildable.

### 3.2 native-table local edit

Ordinary local-source operations may include a representable cell edit, row append/remove, logical-column append/remove, safe row reorder, header/title edit, and raw source-body edit. They do not require a whole-Workspace Query cut.

Every structured operation binds complete current Document source + SourceVersion/2, exact table locator/revision, D2 table parse and target row/cell/column, actual MutationFootprint, current write permission, one proposed full source, and D6 file-install qualification.

An unrelated unavailable/invalid D4 namespace in the same source remains byte-equal. D5 never rewrites it as a side effect.

### 3.3 ragged/trivia/reorder

Ragged rows are legal and are never padded. Column insertion/removal defines deterministic transformation for each row rather than relying on renderer cells.

Structured row reorder is allowed only when inter-row blank/comment/trivia semantics are preserved. Otherwise return `unsupported_table_reorder` and use Source editor. Comments/CRLF are never silently moved/lost.

### 3.4 unrepresentable cell

If a structured UI value cannot be represented losslessly in the current D2 cell grammar, return `unrepresentable_cell` with zero source write. No hidden syntax, HTML blob, or private sidecar is substituted.

## 4. D4 Field occurrences and table-like UI

D4 repeatable Field occurrences may use table/list UX, but D5 does not turn them into Document Table Rows or Records.

Occurrence add/remove/replace/reorder/note continues through D4 Entry/1, OccurrenceKey, TypeSpec, RegistryBinding, SourceVersion, and Field permission. A row-like UI still commits a D4 source transformation.

retained_unavailable/invalid Field never becomes empty through D5 UI. A partial typed projection may show raw/status while structured typed mutation follows D4's matrix.

## 5. Node Collection

### 5.1 membership semantics

Node Collection is a deduplicated set of live authorized NodeRefs obtained by Query evaluation under exact params/context/auth/cut.

Membership is derived:
- no `memberOfCollection` author fact;
- no CollectionRef;
- no D3 parent change;
- deleting definition does not delete Nodes;
- a Node may be in multiple collections;
- Query result order is not membership identity.

Transport pagination is not membership. Query `take` changes semantic membership; semantic `take` is distinct from sending only 200 transport rows.

### 5.2 completeness

Exploration may expose explicitly partial/pending rows, but these require a future D7 complete result/cut:
- mutate all members;
- inverse remove-from-collection;
- bulk update;
- collection postcondition;
- create with requireMembership;
- Automation selecting all members.

I coverage, current page, cloud placeholder, or old cache never proves membership completeness.

Before the new D7 Prepared contract exists, strong collection/bulk entry points remain unavailable/owner_update_required. D5 invents no PreparedActionBinding/3 or bypass token.

## 6. Collection Creation Policy

CollectionCreationPolicy remains explicit construction intent in a saved definition, not parent owner or persistent Template authority. It may declare destination parent, title/body/source defaults, ordinal, optional Template/constructor, explicit author facts, and `requireMembership`.

Creation uses D3 wire12 create_node. replica_local create may reliably save local source/parent/Facet facts, but cannot claim requireMembership completed without a complete collection proof. With `requireMembership=true` the strong Action is managed/complete and reevaluates the same definition/params/auth/dependencies on proposed state, proving the fresh Node belongs to the complete result.

Invalid destination parent rejects without root fallback or sort-neighbor inference.

## 7. Remove from Node Collection

`RemoveFromNodeCollectionIntent` is a D5 semantic intent, not a D3 operation kind.

Availability requires a finite deterministic previewable author change:
- explicit refs selector may change saved definition;
- a D4 Field predicate may propose a specific Field edit;
- arbitrary noninvertible filter is unavailable.

Remove-from-collection is distinct from Trash Node. UI separates definition edit, D4 fact edit, and D3 Trash.

## 8. Native table row to Node / import

Row-to-Node conversion changes identity:
- row stays a Document occurrence with no row identity;
- target NodeRef is fresh;
- row locator is never target identity;
- Resource/Annotation copied to a new owner becomes fresh;
- preview shows source row, target source, mapping, normalization/loss;
- commit uses D3 create/import plus D9 mapping;
- source row retention/deletion is explicit, with no synchronization mirror.

Cross-Workspace transfer remains a target fresh-copy receipt plus optional independent source Trash receipt, never one atomic move.

## 9. delete, batch, and failure

### 9.1 distinct delete modes

D5 distinguishes:
1. table-row deletion as Document source edit;
2. D4 occurrence deletion as Field edit;
3. removal from Node Collection as definition/fact change;
4. D3 Trash;
5. D3 managed_atomic purge.

A generic `Delete` button never guesses from View context.

### 9.2 batch target freeze

After preview, targets are fixed exact NodeRefs/table locators/occurrence selectors plus versions. Commit never reruns current-selection/current-first-N to widen the set.

A partial result never creates all_result targets. Authorization/SourceVersion/Frontier changes stale/reprepare instead of substituting the latest page.

### 9.3 atomicity

A same-Workspace strong batch succeeds or fails as one managed decision under D3/D4/D6/D7 gates. Cross-Workspace operations have independent receipts and a coordination outcome exposing both sides.

## 10. D6-FA proof matrix

### 10.1 proof dimensions

Potential proof inputs are current Document source/SourceVersion/2, table/row/cell locator or D4 occurrence selector, actual MutationFootprint, local D2/D4/D5 validity, complete Query membership/negative range, proposed full source/post-state, current auth/policy, CommitDomain/Frontier/install, and future D7 preparation only for strong collection/bulk consumers.

### 10.2 operation matrix

| operation | local proof | complete proof | result |
|---|---|---|---|
| raw/native table cell edit | source/table/footprint/post-state/auth/install | no workspace membership | ordinary reliable save |
| append/remove table row | same | none | ordinary reliable save |
| structured row reorder | same + trivia-safe | none | ordinary or unsupported_table_reorder |
| column edit | same | none | ordinary; no D4 type inference |
| D4 occurrence edit | complete local D4 Entry | per-Field cross-object duty | D4 complete/pending |
| D3 replica_local move/reorder | parent/sibling | does not prove collection | local success; collection pending |
| D3 replica_local Trash | local closure | complete collection/inbound may be absent | local lifecycle pending, not bulk success |
| collection local read | authorized rows + explicit coverage | no write | partial exploration allowed |
| requireMembership create | local create | complete Query cut/postcondition | unavailable before D7, then complete only |
| remove from collection | explicit invertible plan | complete membership/postcondition | complete only |
| bulk/all_result | frozen complete targets | future D7 complete cut | complete only |
| restore/purge | D3 managed_atomic | D4/D5 complete + purge Frontier | complete only |
| typed row import | D9 mapping + D3 identity | complete target plan | managed, no D7 bypass |

D6 `collection` obligation means complete collection proof is absent, not that the collection is empty. Later r6 proof does not rewrite r5.

## 11. dynamic schema, nested values, import/export

D5 never infers Record from UI shape. D4 object/list/union remains Typed Value and occurrenceKey is not RecordRef.

People names/phones/addresses/engagements remain D4 values/occurrences; the D5 alternative decision remains no persistent Record domain. Local edits use D4 occurrence/Entry semantics.

Imported table/CSV/worker rows are external IR. Fresh Nodes exist only after explicit D9 import commit. Row numbers/provider IDs are not NodeIds.

Ordinary XLSX/CSV export reads current source/Query. A partial collection export is marked partial or rejected by complete-export mode rather than being silently truncated by page size.

## 12. limits and budgets

Fixed limits remain:

| contract | maximum |
|---|---:|
| explicit Node targets in one collection mutation | 1000 |
| native table structured row targets | 1000 |
| preview detail page | 200 |
| fetched result page | 200 |
| one import batch new Nodes | 1000 |

Narrower runtime/policy budgets may reduce but never widen them. 1000 targets is not permission to scan a workspace without the needed semantic scope.

Large tables/workspaces use bounded parsing, streaming/paging, and cancellation rather than unbounded reads merely to eliminate pending.

## 13. multi-replica, I/P, and conflicts

I table parses, collection results, and membership candidates are derived/rebuildable. Rebuilding I creates no author state or complete proof.

P loss never reconstructs old batch decisions, Prepared bindings, or approval/Money from table/query output. Ordinary table source follows D6/D3 recovery.

Across replicas, source edits may conflict, disjoint-row merge still obeys file CAS, concurrent definition/member facts require a fresh complete cut, and one replica's page/placeholder is never complete membership.

Equal hash, row text, or I cache does not restore SourceVersion/locator/cut qualification.

## 14. mandatory domain dispositions

### People

People table-like editors do not create Records; duplicate phones remain distinct D4 occurrences with notes/provenance.

### Organizations

Organizations role/relation editors do not create Records and inverse edit reaches the one authored D4 relation owner.

### Calendar

Calendar derived occurrences create no row/Record identity.

### Library

Library creator/resource/citation lists remain D4 Fields/NodeRefs/Resources and Citation rows are not Records.

Mandatory scenarios are pressure obligations, not feature approval.

## 15. D7/D8/D9 boundary

D7 owns Query execution, complete result, new Prepared/Action evidence. D5 defines no new success binding.

D8 may render Node Collection as table/list/board; device paging/filter state never changes portable membership. Native Document table editor writes exact source.

D9 owns row import/export, Office template/export, and loss. D5 never makes ordinary Nodes persist export-only metadata.

## 16. legacy compatibility

Old D5 v1 semantics and saved D7 PreparedActionBinding/1,/2 retain historical decoders/bytes. Saved D3 v9/v10/v11 and D6 wire1 retain original recovery.

D6-FA adds SourceVersion/2, CommitDomain/Frontier, and SemanticState consumption without adding TableRowId, RecordRef, or CollectionRef.

Before new D7 Prepared exists, strong collection Actions stay unavailable and ordinary D6 source-save never fabricates an old Action receipt.

## 17. completion conditions

Future implementation verifies:
1. local cell/row/column edit saves without unrelated complete index;
2. unrepresentable cell rejects byte-equal;
3. ragged rows/trivia preserve;
4. unsafe reorder is unsupported;
5. Field occurrence edit remains D4;
6. a 200-row page is not membership;
7. semantic take differs from transport page;
8. partial exploration cannot bulk/all_result write;
9. requireMembership create rejects without complete cut;
10. arbitrary noninvertible removal is unavailable;
11. bulk targets remain frozen;
12. cross-Workspace produces two receipts;
13. multi-replica conflict is explicit;
14. r5 replay differs from r6 current;
15. I rebuild/P loss do not restore proof;
16. legacy saved bytes remain unchanged.

This is design only, not an implementation/performance/test-success claim.

## 18. Candidate acceptance boundary

This afterimage creates no Record domain and changes no D2 grammar, D4 catalog, or D3 identity. Future immutable candidate still requires fresh independent joint review and coordinated acceptance.

Old D10 B13 remains REVISE, terminology/bilingual FAIL, eleven OPEN findings. This file closes none.

## 19. Normative D5 v1 exact-contract restoration

This section restores the fixed-S native-table, collection create/remove, bulk, import/export, budget, and stage-adapter details that remain normative. If an earlier overview conflicts with this section, this section wins. D6-FA-r01 changes only outer source/version/qualification binding and introduces no Record identity or new D7 Prepared contract.

### 19.1 native AsciiDoc table grammar and structured intent

D5 native table still follows D2 v2 exactly:

~~~text
delimiter           := |===
row                 := one logical line beginning with |
cell separator      := unescaped |
escaped pipe        := \|
escaped backslash   := \\
cell content        := Inline*
unsupported in D5 v1 native structured model:
  span
  cell block
  nested table
  header option as a schema/type declaration
~~~

The first row is never auto-promoted to schema, display numbers/dates do not infer D4 types, and reading never pads ragged rows into rectangles.

Every native structured edit binds:

~~~text
owning NodeRef
current SourceVersion/2
current Document revision represented by that SourceVersion
current table locator
exact source range
actual MutationFootprint
one explicit operation:
  insert row at explicit ordinal
  remove selected rows
  replace one existing cell with explicit Inline source
  insert/remove logical column
  reorder selected/all rows only when trivia-safe
~~~

Row insertion provides all cells explicitly and replacement requires an existing cell. Column edits over ragged rows explicitly transform every affected row rather than silently padding.

A plain-text cell intent proves that D2 parsing yields exactly the requested inert text. Advanced Inline source continues D2/D3 link/ref validation. Newline/reserved-syntax/complex content that cannot be represented losslessly returns:

~~~text
unrepresentable_cell
~~~

with zero author write. Truncation, hidden line splitting, macro execution, unfrozen escaping, and private sidecars are forbidden.

Whole-table row reorder is permitted only when no inter-row blank/comment trivia changes attachment. Otherwise:

~~~text
unsupported_table_reorder
~~~

and the exact source remains. View-only sorting never writes source.

### 19.2 row domains, Field cell, and occurrence edit

The five row-like domains remain disjoint:

~~~text
native Document table row  -> D2 table_row occurrence
Node Collection row        -> NodeRef
D4 repeatable fact row     -> Field Value Occurrence
Query aggregate/join row   -> D7 derived row
Resource preview row       -> derived view of Resource bytes
~~~

NodeCollectionResult contains NodeRefs only and never mixes Field/table/aggregate rows. Sorting, pagination, and hidden columns do not change domain or permission.

D4 Field-cell edit mode is explicit:
- zero occurrences -> append;
- exactly one -> replace that current selector;
- multiple -> select one occurrence OR explicit append OR explicit replace-all.

replace-all expands every removed occurrence, note, qualifier, provenance, and relation effect and passes the same gate. Empty display is not missing/unknown/invalid/null/empty text; D4 has no generic null. Clearing input explicitly means a legal empty text or occurrence removal.

Replacement binds exactly:

~~~text
owner NodeRef
FieldId
SourceVersion/2 / current source revision
occurrenceKey
expected raw Entry
edit mode
~~~

Note-only edit preserves every other author member and every unselected source/trivia byte. Old selector never survives a revision merely because key bytes match. A Core replan preserves base/current/proposed and reproves target plus disjoint changes; FieldId+value, row number, nearest text, or key alone never carries identity.

### 19.3 Collection Creation Policy and parent priority

Node Collection membership comes from D7 evaluation over explicit Workspace, snapshot, authorization, compiled Query, and params. Deduplication uses full NodeRef. A collection is not parent, owner, or lifecycle container.

A reviewed "create in collection" plan includes:

~~~text
destinationParent: NodeRef
final sibling ordinal
explicit title
initial Document source
required initial Facet/Field author facts
optional completed Template construction input
requireMembership: Boolean
definition/query binding + params + authorization/dependencies
~~~

Parent priority is exactly:

~~~text
explicit destination in this request
  >
explicit parent in saved creation policy
  >
Node containing the saved definition
~~~

An ad-hoc Query has no containing definition and therefore requires explicit destination. Workspace root/current selected/sorted row is never guessed. Default parent is live, same-Workspace, authorized, and structurally legal. Default ordinal is parent child count in the commit-plan pre-state; concurrent change triggers replan.

Template is one-time D9 source construction, never persistent authority. Conflicts reject rather than last-wins. A caller may create without Template from explicit title/body/facts.

requireMembership=false explicitly means "create Node" and preview states that the result may not remain in the collection. requireMembership=true re-evaluates the same Query/params/auth/dependencies on the complete proposed post-state and commits only after proving the fresh Node belongs to the complete semantic result. Transport page never participates while semantic top/limit operators do. If proof cannot be obtained, the strong collection-create action is unavailable.

### 19.4 Remove from Collection, Delete, and batch

RemoveFromNodeCollectionIntent is available only when there is one finite deterministic previewable author transform:
- edit an explicit refs selector;
- edit a determinate D4 Field fact;
- edit the saved definition.
An arbitrary filter/aggregate with no unique inverse is unavailable.

UI/intent must distinguish:

~~~text
delete Document table row
delete D4 Field occurrence
remove from Node Collection
Trash Node
purge Node
~~~

Trash preview exposes real D3 subtree, Resources/Annotations, relation effects, and hidden-child closure. One NodeRef shown in several collections receives one lifecycle operation only.

Bulk preview freezes the complete target set and every target version. Commit never reruns "currently selected/current first N" to widen targets. A new result requires new preview. Page prefix is never whole-result scope.

A same-Workspace strong batch succeeds or rejects atomically. Cross-Workspace has two independent authority results:

~~~text
target fresh-copy receipt
source Trash receipt (optional and separately authorized)
~~~

Target failure never deletes source. Source-Trash failure reports copied/source retained and never pretends atomic move.

### 19.5 row->Node, import/export, and dynamic schema

Table-row to Node is a fresh identity-changing conversion. Plan includes title, parent, complete Field mapping, losses, and disposition for every original occurrence. Retaining the row is one-time copy only. Promotion uses one authorized atomic plan to replace the original row with an ordinary Node link row and never leaves a cell mirror claiming the same object fact. Node->table is export/snapshot and NodeRef never becomes row identity.

"Add column" may only choose an existing Field for display. Creating a Field is Registry evolution and never infers type from the first value. Breaking change uses fresh ID plus explicit migration. Unknown/uninstalled/incompatible namespace preserves raw and displays typed unavailable rather than empty.

CSV/XLSX mapping explicitly states:

~~~text
input format + encoding
sheet/table/range
header presence
exact header names and duplicate-column selection
space / empty cell / missing column / null distinctions
target FieldId per column
conversion per column
title construction
destination parent
initial Facets
duplicate-row policy
source/provenance/losses
formula policy
quantity/resource budgets
~~~

Display column label/locale never guesses decimal/date/ref. Formula/external link is not executed; cached value, formula text, or rejection is explicitly selected with evidence freshness. Ordinary import does not merge existing Nodes by title/path/phone/email/external UUID. SourceBinding comparison domain and active OriginBinding upsert remain separate, and SourceBinding alone never authorizes target update.

Long import is an explicit sequence of finite atomic batches. The whole job is not one atomic commit. It stores committed batch receipts and input binding, and resume revalidates uncommitted input. D9 never deduplicates a retry merely because row text matches.

### 19.6 hard limits, budgets, and partial results

Fixed D5 hard limits:

~~~text
one Node collection mutation explicit Node targets <= 1000
one native table structured edit row targets      <= 1000
one preview detail page                           <= 200
one fetched grid/result page                      <= 200
one import batch new Nodes                        <= 1000
~~~

Exceeding a fixed limit returns:

~~~text
limit_exceeded
~~~

with no automatic truncation or silent splitting. A narrower runtime/policy budget may reject sooner but never raise the fixed limit.

Count limits are insufficient by themselves. Core also binds source bytes, decoded-value bytes, read dependency count, write bytes, time/cancellation, and materialization budget; reaching a narrower resource bound rejects the whole plan. A 200-row preview is a display slice and never makes unseen effects reviewed.

The 10,000 overlapping-engagement Person-node case must derive colleague results on demand rather than persist roughly fifty million edges. Large homogeneous import acceptance covers at least 10,000 Nodes, 10 batches, failure around batch 6, repeated resume, and working memory smaller than full workspace input. These are future D6/D7/D9 evidence duties, not current performance claims.

### 19.7 D5 intent-adapter minimum semantics

Future implementation provides one closed request/plan/error/receipt adapter per intent. D5 does not predefine the new D7 Prepared wire.

~~~text
edit native table:
  binds owner NodeRef, SourceVersion/Document revision,
        table locator, exact source ranges, explicit transforms
  success -> one complete valid proposed Document source
  reject  -> stale locator/revision, invalid/unrepresentable cell,
             limit/auth/D2/D4 full-source failure

edit Field occurrence:
  binds owner, FieldId, RegistryBinding, SourceVersion,
        occurrenceKey, expected raw Entry, explicit edit mode
  success -> D4 proposed state + relation effects
  reject  -> stale/ambiguous/unavailable schema/
             constraint/auth/relation conflict

create through collection:
  binds saved-definition locator/revision or ad-hoc Query binding,
        parent/title/source/ordinal, requireMembership,
        complete dependencies
  success -> fresh NodeRef + valid source/placement +
             membership proof when required
  reject  -> parent/stale/auth/template/schema/membership/budget

remove membership:
  binds selected NodeRef, Query+params,
        saved-definition locator/revision or ad-hoc binding,
        actual read dependencies,
        explicit author-fact/definition transform
  success -> complete post-query proves Node no longer a member
  reject  -> noninvertible/no real write target/stale/auth/
             cannot prove complete result

trash selected Nodes:
  binds dedup exact NodeRef target set + D3 closure/state
  success -> D3 lifecycle receipt
  reject  -> root/hidden/unauthorized/stale/incomplete closure

import batch:
  binds exact input, finite mapping, title/parent/facts, budgets
  success -> fresh Nodes + per-row mapping + D3/D9 receipts
  reject  -> unresolved format/loss/schema/limit/stale/auth
             with zero author writes for the batch
~~~

Row handle, column index, caption, caller-provided before-image, and cached decoded value are never write authorization. Core revalidates real source, authority, visibility, revision, and complete read-set. Preview is not commit and UI never guesses success from timeout/disconnect.

### 19.8 D6-FA-r01 current composition

Every native-table source mutation binds D6 SourceVersion/2, CommitDomain/2, current Policy, and actual FileObjectBinding/install capability. Equal hash alone never proves row/cell locator or source continuity.

A strong collection Action additionally requires the future D7 complete cut/Prepared contract. Until that owner exists it is unavailable/owner_update_required; D5 creates no replacement token. Explicit partial/pending rows are exploration-only and never feed all_result/bulk/requireMembership/Automation writes.

D6 `collection` obligation may be represented as semantic_pending(collection): complete collection proof is absent, not empty. Later r6 completeness proves r6 only; an r5 pending receipt keeps original bytes. I rebuild, P loss, a new replica, placeholder, and A->B->A never restore old membership/cut proof.
