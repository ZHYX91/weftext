---
source_language: zh-CN
translation_of: FEATURES.zh-CN.md
translation_status: synced
---

[简体中文](FEATURES.zh-CN.md)

# New Features candidate: links, queries, navigation, annotations and output

Status: coordinated design candidate, neither accepted nor implemented. This file implements the intended link, Query, navigation, business-type, annotation and output outcomes in [REQUIREMENTS](../typst-native/REQUIREMENTS.md), with necessary Agent, automation, external-effect and cost boundaries. See [REVIEW](REVIEW.md) for validation.

## 1. A few shared objects and ownership

Author facts remain one `.typ` source and independent JSON annotations. Link indexes, Query results, thumbnails, inverse relations, presentation preferences and export artifacts are projections. SOURCE owns native values and centralized schema; EDITOR owns Drafts, source mapping and constructs that can be edited safely; CORE owns identity, read qualification, logical transactions and recovery. Features creates no second field database, write entry point or authorization engine.

The shared objects are `ReadBasis`, `RevisionId`, `Plan`, `OperationJournal`, `Receipt` and `CommitManifest`. A RevisionId is a private Core allocation during planning and becomes valid only at commit; a string or equal bytes proves no permission. ReadBasis binds the actual observer incarnation, file generation, schema, authorization and consumed dependencies. A new read of equal bytes cannot revive an old write qualification. `SourceSnapshot` denotes an authorized exact source snapshot; `FieldProjection` discloses only permitted fields. Neither upgrades to the other. Core can privately locate a field and apply a narrow source patch; constructing that patch does not justify returning whole-source bytes to a field-only caller.

The logical calls below consume [CORE](CORE.md), rather than defining another wire family. [SOURCE](SOURCE.md) owns values/schema, [EDITOR](EDITOR.md) owns Draft/native writing, and [REVIEW](REVIEW.md) states evidence limits:

```text
readEntity(ref, current|revision, purpose) / readScope(scope, purpose, completeness)
  -> SourceSnapshot | FieldProjection | unavailable
compile(sourceSnapshot, environment, target, budget) -> CompileSnapshot
evaluateQuery(definitionSelection, parameters, scope, budget) -> QueryResult
readResultPage(handle, cursor) -> Page
prepare(opId, intent, draftBase, budget) -> Plan{preview, changes, losses, dependencies}
commit(opId, planHandle) -> Receipt | conflict | rejected | unknown
operationStatus(opId) -> unknown | in-progress | pending_reconciliation
                         | committed | conflict | failed
createJob / jobStatus / stopJob / inspect / confirm / download
```

prepare freezes real before images and all changes. commit rechecks current qualification and makes one logical transaction. A transaction can change source, resources and annotations, but covers only one commit authority with the required guarantees; independent servers do not gain fictitious cross-server atomicity. Core journals recover crashes and physical installation; projections expose only decided logical state. unknown retains the original opId, Draft and plan and queries operationStatus instead of creating again or sending another external request. An operation waiting for user input must not hold a long whole-library lock against unrelated notes.

## 2. Native links and four locator domains

Body links use standard Typst `link`, labels and refs. Clear application URI names are proposed; syntax need not reuse old decoders:

```typst
#link("weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654#plan")[项目计划]
#link("../资料/白云山/白云山.typ#route")[白云山]
#link("weftext-workspace://58c47256-bfce-4d29-a037-b83a3a644fb0/资料/白云山/白云山.typ#route")[白云山]
#link("file:///D:/notes/report.pdf")[报告]
```

Chinese URI text above is a readable form; generated canonical URI bytes use UTF-8 percent encoding. In-document `#link(<plan>)[View]` and `@plan` retain official semantics, including ref object/numbering requirements; neither claims cross-note identity.

| Form | Basis and guarantee |
| --- | --- |
| ID (default for creation) | Full library ID + note ID. Rename/move retains the same object; deletion never retargets to a new namesake. |
| Source-relative path | Based on the actual authored source file's directory; links inside an include use the included file. |
| Library path | Identifies the library and its root-relative path. Moving the source does not change it; moving the target may break it. |
| Local absolute path | A verified platform file URI, with a current-local-machine guarantee only. Server does not open arbitrary disk access through it. |

“Shortest valid relative path” is a computation within the source-relative domain, not a fifth protocol or title lookup. Find the longest common inventory-component prefix of source directory S and target T; emit `../` once per remaining source directory level, then the remaining target components, retaining the fragment. For the same source and target file use the exact basename. Where `.`, `..`, symlinks or case rules admit multiple interpretations, emit only a canonical form that the current Core resolver round-trips to the exact target. “The last two components are currently unique” is not path computation. Different volumes, unconnected libraries, undisclosable paths and boundary escapes yield another available form such as ID plus an accurate reason.

URI components use one UTF-8 encode/decode pass: slash is a separator, `+` is literal, `%252F` decodes once to literal `%2F`. No double decode, Unicode normalization or title-based case folding. Reject malformed escapes, control characters, encoded separators, out-of-library parents and unknown authorities. A platform library generates and rereads file URIs; do not concatenate UNC/drive syntax. Encode the resulting string through official Typst string construction while preserving the display content AST. Invalid application addresses still retain valid source and a repair entry point.

## 3. Conversion, copying and link indexes

For one current occurrence provide Convert form, Copy target address, Copy complete link and Shortest valid relative path. Literal source copying does not require the target to exist. Resolved copying requires permission to disclose the target address but no source write permission. Writing conversion requires writable source and only relevant target read/location permission.

Conversion obtains current Draft serial and ReadBasis, finds a unique authored occurrence through official AST/EDITOR mapping, resolves the exact current target and fragment, generates the requested form, rereads it through the official parser and the same Core resolver, compares identity/fragment/display content/native parameters/all unselected bytes, then produces one Draft operation/Undo and prepare/commit. A fragment must exist uniquely in exact target semantics; unavailable or duplicate labels cannot be silently discarded. Undisclosable paths cannot become location text. Rename, replacement, revocation, environment changes or later Draft edits invalidate the qualification; retain Draft and reprepare. Include/generated/shared-variable links without a unique writable argument get Source entry, not a location inferred from equal text.

Enumeration retains two origins to avoid merging repeated compiled occurrences:

```text
AuthoredLink = {sourceOwner, ReadBasis, astPath, byteRange, destinationSlot?}
RenderedLink = {projectionGeneration, occurrenceOrdinal,
                fragments[], destination, origin: unique_source | generated | multiple}
LinkResolution = resolved(target, fragment?) | missing | fragment_missing
                 | ambiguous | unavailable
```

Official AST enumerates AuthoredLink in source order. Each logical link in the final semantic projection is one RenderedLink; line/page fragments share it. Running the same include twice produces two entries even with equal spans and text. Origin traces must prove their association; span/string equality is insufficient. Generated links may navigate/export, but no unique source means no writer or stable asserted edge.

Outgoing links expose real authored and compiled links. Incoming links provide separate ID and “currently resolved path” groups, both navigable. Typed property refs appear as Property references, without fake body links. The path index is derived within current scope and invalidates on source/target move, replacement, permission or directory changes. Potential mentions are separate and form no actual-link or integrity edge. partial never means there are no incoming links in the whole library. Complete zero is possible only for the current authorized complete scope, without hidden titles, paths or counts. There is no automatic orphan deletion or default library-wide path rewrite.

## 4. Complete potential matching and overlap algorithm

Terms come from currently disclosable note titles and schema-designated alias/name search fields, excluding the current note. Former personal names can derive candidates from name roles without another alias fact. Deduplicate the dictionary by `(target identity, exact term)`; retain all objects with equal titles. Default matching is exact Unicode scalar literal matching, without implicit Pinyin, fuzzy matching, case folding or normalization. Additional modes require explicit choice and a stated reason.

Separate discovery from writing: accurately extracted ordinary visible logical text may discover candidates; shortcut conversion requires a unique source range and current writable mapping. Valid generated/include content may offer read-only discovery marked with its origin. Code, math, URLs, real links, attribute declarations and annotation bodies are excluded from automatic ordinary-text scanning. Do not concatenate separate logical fragments. Soft visual wraps remain the same fragment; EDITOR explicitly maps whether source line breaks belong to the same logical flow.

For n scanned scalars and z matches:

1. Build an Aho–Corasick trie from one authorized dictionary snapshot, keeping all target identities at terminal nodes. Scan each logical fragment and return all matches in `O(n + z)`. Budget exhaustion records partial per fragment; never keep only the longest match.
2. A hit is `(flow, startScalar, endScalar, targets[])`. Group only identical ranges; targets use stable full-identity order, not title merging. Record matched terms and origins. Containment/crossing are not deduplication rules.
3. Compute interval overlap separately. Only `a.start < b.end && b.start < a.end` overlaps; touching endpoints may share a lane. Retain every semantic hit.
4. A candidate visual design uses at most two separated underline lanes. Greedily assign the lowest free lane in `(start asc,end desc)` order; groups without a lane enter their connected overlap component's More matches entry. Two lanes, default-off and 20 entries per page are replaceable design choices. The requirement is complete access. Keyboard/touch/AT lists enumerate all ranges independently of visual lanes; stable cursor pages never silently truncate.
5. Hover/click/keyboard/long press open the same list. Escape restores the original selection; background scanning does not steal IME/body focus. Only choosing a target and Convert writes. Never default to the first target.
6. Reauthorize and recheck the dictionary entry and current Draft. Both endpoints of the scalar-to-exact-byte mapping must be whole grapheme boundaries, without splitting combining/ZWJ/CRLF. Partial-grapheme hits remain read-only discovery; do not expand selection. Generate a native ID link via §3 and compare target/display text/outside bytes as one Undo.

```text
Text: 白云山南路
[0,3) 白云山 -> A,B (two targets at one range)
[2,3) 山     -> C   (contained)
[2,5) 山南路 -> D   (crossing)
Greedy visuals: 白云山 lane0, 山南路 lane1, 山 overflow.
Complete list: three ranges, four targets, each independently selectable.
```

RTL drawing uses logical ranges and current layout mapping, not x-coordinate semantic order. Disabling hints does not change source. Scanning may prioritize the visible area and cancel. Revocation removes the target's candidates and displayed context. Completeness describes the authorized scan only, without a hidden-term count. An unfinished library scan does not block one independently verified local conversion.

## 5. One Query semantics with explicit states

Weftext Query accesses authorized library data; official Typst `query` remains current-compilation introspection. Plain search, navigation conditions, visual filters, saved queries and APIs produce the same typed Query AST for one evaluator. Persist structure with native dictionaries/arrays and choose a statically restricted, deterministic, side-effect-free CEL syntax profile using standard `exists/all/filter/map` comprehensions. Do not inherit old DAG wire, eight required members or contribution version trees. The CEL parser handles expression strings; the official Typst parser handles native carriers. No eval/import/arbitrary callback/network. Statically type-check and preserve advanced expressions; if the builder cannot represent them, expose a read-only advanced condition/Source rather than deleting it.

Base nodes are `scan(scope,lifecycle)`, `read(fields/products)`, `filter(expr)`, `project(columns)`, `unnest(array,binding)`, `sort(keys)`, `take(limit)`, `union_all`, `distinct(keys)`, `group(keys,aggregates)`, `related(relation)` and saved-query calls. Body/title/tables/all realized headings, including generated and arbitrarily deep headings, come from the same official semantic projection. Generated occurrences have run-local keys and are read-only without unique sources. Plain full-text search creates explicit `textMatch` nodes; UI shows body inclusion, directory/descendants/library scope and all conditions. No new Query DSL is introduced.

Query uses SOURCE's single Type/Value: text, bool, i64 int, exact decimal, explicit float, civil date/time/local datetime, offset-bearing instant, ref, array, record and schema-discriminated union. Money+currency, quantity+unit and precision dates are business records in that Shape system, not unregistered scalar codecs. Arrays preserve order and duplicates. Schema interprets record members; unknown native members remain read-only values without inferred types. No implicit float/decimal, date/instant, currency or unit conversion. Explicit conversion chooses an exact provider/version recorded as a dependency. Phones remain text. Schema conflicts/missing schema never infer a type from the first result row.

```text
Cell<T> = value(T) | absent | null | invalid(diagnostics) | unavailable(reason)
QueryResult = {handle, runId, definitionIdentity, inputBases, schema,
               coverage: complete | partial(reasons),
               freshness: current | stale(reasons), terminalKind, total?}
```

absent means not authored; null means schema-permitted explicit none; invalid means an actually read value violates its definition; unavailable means schema/product/required dependencies cannot be obtained. Public reasons must not disclose hidden-object existence; undisclosable objects are excluded from public scope. Empty strings/arrays and zero differ from these states. Predicates explicitly test `isAbsent/isNull/isInvalid/isUnavailable`; taking a member from a non-value produces a type/value error. Unknown/unavailable must not silently filter as false. If unavailable fields prevent deciding membership, mark partial with confirmed rows and undecided reasons. An invalid overall query returns error, never complete empty. FieldProjection consumes only statically isolated permitted declarations/schema and does not compile hidden body. Hidden values cannot control public success/failure/branches/timing diagnostics. A field requiring hidden dynamic computation returns uniform unavailable instead of executing a secret-dependent branch.

Coverage and freshness are orthogonal: a complete old projection may be stale; partial current rows may still be browsed. A truncated page is not partial: it has a continuation cursor. `take(200)` is an explicit author limit, not a whole-library statistic. Only complete + current + zero rows displays complete zero for that scope. A partial count is a known lower bound; aggregates over undecided input are partial/estimated, never exact totals. Never graph a page as whole-library proportions. Authorization changes invalidate run/cursor, remove protected cached cells/snippets/counts and reexecute after authorization. Other dependency changes may display readable, explicitly stale results without writing them.

Row lineage explains origin but grants no writes. Direct-field rows may propose precise intents, and prepare reacquires current ReadBasis and narrow write permission. group/distinct/computed values do not inherit a representative writer. Results from explicitly connected authorized libraries may union without implicit cross-domain reads or cross-library transaction guarantees. Trash is an explicit authorized read-only lifecycle scope, retaining tombstone/unavailability diagnostics. Results grant no restore/purge. Every view action whose call chain includes trash is read-only.

## 6. Nested collections: same-item binding, order and representative queries

Read an array as one structure. Within current ReadBasis, distinguish items by `(owner,field,valuePath,ordinal)`; ordinal is a current locator, not permanent identity. Algebraic `any(A,x=>P(x))` computes the complete predicate on one x; `all` of empty is true and `any` of empty is false. A missing array does not become empty without explicit `orEmpty`. filter/map preserve input order and duplicates. unnest outputs parent rows in their current order then item ordinal, retaining one item binding. Multiple-array expansion requires explicit Cartesian product or actual stable-key association, never guessed index pairing.

```text
phones = [
 {number:"13800138000",label:"personal",note:"Evening"},
 {number:"010-12345678",label:"work",note:"Daytime"},
 {number:"13800138000",label:"work",note:"On call"}
]
any(phones,p => p.number == "13800138000" && p.label == "work")
  -> true, matching only item 3.
Remove item 3 -> false; item 1's number cannot pair with item 2's label.
filter(phones,p => p.number == "13800138000")
  -> authored order items 1,3, each with complete label/note.
```

The proposed saved definition uses direct application metadata. This is a design schema sample, not an executed implementation fixture. Installed schema bindings interpret field names; users may use short names, while runtime must not bind fields by display labels:

```typst
#metadata((id: "f8597b15-6b7f-4a6a-9b52-c2732730fe41", types: ())) <weftext>
#metadata((
  kind: "weftext.query", key: "6b2b9e19-5599-48c3-b27b-6b006ec9c397",
  scope: (library: "current", lifecycle: "live"),
  types: ("person",),
  where: "phones.exists(p, p.label == 'work' && p.number.startsWith('138'))",
  select: (
    (name: "person", expr: "self"),
    (name: "contacts", expr: "phones.filter(p, p.label == 'work')"),
  ),
  order: ((expr: "title", direction: "asc", missing: "last"),),
)) <weftext-query-work-phones>
```

`any(A,x=>P)` above is algebraic explanation. Persisted text uses CEL `A.exists(x,P)`; all/filter/map use `A.all(x,P)`, `A.filter(x,P)`, `A.map(x,E)`. There is one expression parser/evaluator. Text is the author representation and AST is its rebuildable projection, without an independently persisted AST fact. Custom functions such as containsDate have unique static signatures registered through schema/provider; unknown functions reject the whole query rather than executing a dynamic plugin. Parameters/defaults/view columns/export all use SOURCE Type/Value, without another TypedLiteral family.

| Use | Representative Query and meaning |
| --- | --- |
| Current appointment | `engagements.exists(e, has(e.employer.ref) && e.employer.ref == param.org && containsDate(e.during,param.day))`; employer and during belong to the same item. Literal branches do not look up targets; unknown validity is not permanent/current. |
| Organization roster | `related(inverseOf(person.engagements.employer.ref),this.node)` reads authorized personal assertions; Organization stores no duplicate roster. |
| Rank selection | One engagement's `rank.system == param.system && rank.level == param.level`; no cross-system ordering of rank text. |
| Ordered authors | `publication.contributors.map(a,a)` preserves agent(ref or text), family/given/credited_as, roles/affiliations and order. Literal branches do not create people. |
| Contacts table | `unnest(phones,p)` projects `self,p.number,p.label,p.note,p.ordinal`; repeated numbers remain separate rows. |
| Open tasks | `types has task && status == 'open' && project == param.project`; sorting civil due adds no host zone. |
| Calendar tasks | A date due explicitly projects `[due,nextCivilDay(due))` as a due-date card, not another event fact/reminder. Instant due does not invent an endpoint. |
| Publication history | `publication.publication_history.filter(s,containsDate(s.during,param.day))` may return conflicting assertions without choosing the latest. |
| Deep headings | `read(headings); filter(level >= 6); project(level,text,occurrence)` includes generated headings; 10+ levels remain actual levels. |
| Money summary | `group(currency,sum(amount))` separates currencies; invalid amounts do not silently become zero totals. |

One recursive Type/Value equality applies everywhere: exact text; mathematical equality for exact numbers with authored spelling separately retained; finite float numeric equality including signed zero while preserving raw values; exact UTC instant; complete ref identity; order-sensitive arrays; record key order irrelevant but nested arrays ordered; tagged union branch+value. group/distinct compare full equality keys after any hash collision and choose representatives by stable whole-value order rather than a page's first row. They create new group rows without writers. Complex-value sorting requires an explicit scalar projection and missing-value position.

To edit the second repeated phone, UI submits current basis and item path. Core locates that item again in the exact official before AST, changes only the selected member, reparses the whole source and validates schema, as one Undo/transaction. Reordering or source changes produces conflict; show current items for selection instead of guessing an equal number. Queries, grouping, indexes and exports never rewrite array order. A domain needing cross-version item refs may explicitly add an item ID after justifying the need; every phone does not get a default UUID.

## 7. Saved queries and navigation lists (NAV01–05)

Locate a saved Query by its author note and unique application definition key; label is its display/native locator name. Direct metadata uses `kind:"weftext.query"` and a Core-allocated lowercase UUIDv4 key. Identity is owner+key, not another NodeID. Features owns scope/types/where/select/order members; SOURCE uses official AST/binding qualification consistent with root declarations and requires unique keys/labels over the whole root. Generated/include/nested/calculated declarations gain no definition qualification. Ordinary references default to **current**: open the currently authorized definition each time, show the RevisionId used and explain/reexecute a definition update. Explicit **pinned** references fix a definition RevisionId and display Pinned definition. Unavailable history or current lack of permission yields unavailable, never silent latest fallback. pinned freezes the definition, not data: each invocation still evaluates current authorized data. Capturing exact old results requires explicit result export/history snapshot, not a pinned query pretending frozen data. Old rows/cursors and pinned definitions give no current writer.

Allocate a stable application key once for a new definition in native metadata; its label may change. Read by key rather than a new same-label object. Copy allocates a new owner/key map and rewrites typed saved-definition refs through the same copy map while retaining current/pinned choice. SOURCE coordinates key declaration, uniqueness and literal-projection qualification with root rules. Valid advanced/calculated declarations remain readable/savable source, without counterfeit application-definition qualification.

| Requirement | Candidate interaction/state |
| --- | --- |
| NAV01 | Navigation/list/body panels collapse and resize; narrow screens enter/return in layers. Session stores Query AST/scope, selected identity, scroll anchor+offset, focus and per-owner Draft/Undo. Return reauthorizes and reevaluates; disappeared selection shows removed/deleted/unavailable without reviving rows. Background previews neither focus body nor commit Draft. |
| NAV02 | Optional current authorized summary/image or PDF thumbnail/named dates/key fields and density. Missing author title falls back to a disclosable filename/path, marked as fallback, without writing title. Preview states are absent/building/ready/failed/unsupported/unavailable; missing fields are distinct. Usable rows open immediately without all-library OCR/thumbnails. |
| NAV03 | Directory/tag/type/value choices produce the same removable Query condition chips. Descendants/whole-library/body inclusion stay visible. Facet counts use authorized current `count_distinct(self)` in the same scope; each note counts once per value, missing is a separate group, partial counts mark known coverage. Changing sort does not change date filters. |
| NAV04 | Shortcuts use NodeRef, DirectoryRef, TagRef or a saved-Query key, with reorder/removal leaving targets intact. Node directories use NodeRef; CORE admitDirectory registers a non-Node managed container for DirectoryRef. CORE registers tags and SOURCE typed TagRefs agree with navigation/Query. Explicit admission affects only the selected directory, not the whole library or unmanaged boundaries. Duplicate identity, observation gaps and invalid targets are explicit, never resolved through equal names. Path-only external bookmarks explicitly lack rename stability. List pin changes only eligible current-result display order, bypassing neither filters nor permission, and differs from shortcuts. |
| NAV05 | Personal local ViewPreference is keyed by directory/tag/Query identity and saves sort/group/columns/summary/thumbnail/density with reset. Invalid field/target is shown without replacing a namesake. Settings contain no result/writer and change no source, real tree order or other user's preference. |

Device settings stay in app configuration, outside business metadata. Shortcuts/view preferences/named layouts are re-resolution entry points, not stored grants/results/writers. Personal list pins may store NodeRefs by context; first query current membership, then stable-partition pinned/unpinned rows, sorting each partition by the explicit Query sort. Query-data export keeps Query order; exporting the visible list must explicitly include and label personal order.

Table/list/task list/board/calendar/timeline/gallery/metric and already selected chart layouts consume typed terminals; absent renderers are unavailable. Views perform no hidden fetch/filter/aggregate. AT has an equivalent complete accessible table. Virtualized rows are not a complete dataset. Quick capture covers location/type/template/optional title/fields/body, with field grouping/search/pinning. Only a success Receipt opens the real object and clears Add another. unknown retains opId/Draft. Creation outside current filters is not creation failure; only a complete current query establishes non-membership, otherwise membership is unknown.

Tasks are ordinary task notes with body details and shared status/due/project fields. Checking, moving lanes or changing calendar dates shows exact field before/after and requalifies at prepare; no aggregate/generated row writes. A checklist is a body item, not an automatically created task. Period navigation finds existing notes by explicit series/period/scope and creates only on explicit user intent; Core decides uniqueness under concurrent creation. Civil dates, precision, calendars, time zones and rule dependencies remain exact. Reminder delivery, snooze, repeat-after-completion and natural-language dates remain deferred.

## 8. Rich schemas: retained business scope and Query responsibilities

Every type supports necessary multi-values, nesting, time and sources. Central schema defines types/constraints/relations, with short names for ordinary UI/source. Common fields display first and optional detail expands by group. The following reconstructs selected historical capabilities without old per-value Entry/qualifier/nominal Registry versions. SOURCE owns complete field definitions; Features owns Query/view semantics.

| Type | Retained capability and representation |
| --- | --- |
| Person | Ordered names with role/language/script/validity; repeated phone/email/address/website/account with preset/custom service, usage/label/note; profession and system-qualified rank; appointments with ordinary-node or explicit literal target, position/department/validity/status; observations with quantity/time; independent nationality/legal-sex/gender-identity temporal assertions; life events/important dates with sources/conflict/explicit choice; avatar resources. parent/guardian/manager/mentor directions are explicit; spouse/sibling/general family/social/professional assertions retain separate history and roles. Names/custom descriptions imply no direction, ancestry or current validity. |
| Organization | Multiple names/identifiers/classification/status history/site/address/contact. Business guidance/territorial administration/joint leadership/supervision/subsidiaries/brand/membership/parent/governs/owns/general relations remain distinct; folders imply no business relation. Personal appointments derive the roster without duplicate author data. |
| Project/Task | Project is an ordinary note referenced by tasks; a complete Project domain schema remains a candidate rather than invented required fields. Task status is required; start/due/priority/project/completed-at/dependency retain selected meaning. Body contains detail. Completion time comes from an explicit action rather than invented history on arbitrary status edits. Full task-status history is not already selected. Existing period navigation remains; complex HR/reminder execution is not brought forward. |
| Work/Paper | Work kinds/identifiers/ordered authors and institutions with roles/publication history/venue distinct from publisher/version relation/citation key/owner-local attachments. Paper subtype includes useful volume/issue/pages/date/abstract/topics. A separately referenceable version explicitly creates a note; equal DOI never silently overwrites identity. |
| Calendar | Event ranges/sources/participants/series/exceptions/rescheduling/cancellation and existing-period-note navigation remain. Date/instant/precision/calendar/zone are explicit. Deferring reminder execution does not delete saved reminder intent. SOURCE/coordinating owners define complete schemas/providers. Task due is not automatically an event. |

Historical records may contain validity/status or selection/note/sources as native domain members, without another universal fact envelope. Sources may point to original attachment locations or refs, without read permission. Equal repeated values and overlapping periods may coexist. “Current” requires explicit validity/selection rules, not last-write-wins. Multiple preferred assertions are ambiguous and require choice instead of highest-confidence guessing.

Schema owns typed ref slots/direction. Store each fact once at one declared author owner and derive inverses. A symmetric assertion has one explicit owner; edit from either endpoint first requalifies that author source, otherwise remain read-only. Inverse rows retain exact `(sourceOwner,field,itemPath,ReadBasis)` and prepare checks them again rather than writing the displayed endpoint. Another independent assertion retains its provenance. A graph may merge display edges but must expose all assertion sources, without writing unknown origins. Literal person targets have no lookup/inverse/automatic person creation. Deleted targets retain authored facts marked suspended/dangling; cleanup needs an explicit authorized Plan, never implicit cross-source deletion. Personal appointments allow ordinary target notes without mandatory Organization upgrade. Other Organization relations validate schema domains. parent→child, guardian→ward, manager→direct-report, mentor→mentee and engagement→has-engagement inverses are explicit. General family/social/professional preset domains and custom text remain; custom text does not infer direction. Colleagues derive on demand from overlapping appointments, without O(n²) stored copies. Cycle/rank constraints apply only where schema explicitly defines them, not to every free relation.

### 8.1 Actual algorithms for periods, ranges and events

Calendar has three selected data types: period-note/range-note/event. day/week/month/quarter/year are period kinds, not five identities. Period fields identify series/calendar/rule/period key/zone and unique/many scope; path/title does not define key. Navigate using the same Query over `(series,key,scope)`; partial cannot establish absence. A unique creation has Core validate a complete negative range and concurrent constraint in the same Plan; many permits independent notes.

A range field is `(start,end_exclusive)`, a date or instant half-open interval with at least one bounded endpoint. Open endpoints are explicit absence, never now/magic maxima. Comparable bounded endpoints require start<end, enforced by SOURCE's named `bounded_half_open_range` admission constraint. Window overlap is `start < window.end_exclusive && window.start < end_exclusive`, treating open ends as unbounded. `during(start,end)` is a separate validity period, not an event range; containsDate follows its explicit closed-interval/unknown rules. Duration is derived only; unavailable providers never fabricate seconds. Date units/precision differ from exact instant elapsed seconds. Event reuses range with status/participants/authored reminder intent rather than another time fact; participants can derive inverses, while delivery remains deferred.

Recurrence uses frozen calendar/zone/provider and anchor civil phase to generate daily/weekly/monthly/yearly candidates, applying interval/byMonth/byMonthDay/byWeekday/weekStart and count or inclusive until. Skip nonexistent civil dates and use explicit DST gap/fold rules rather than host defaults. Merge rDates by original occurrence key, exclude exDates, then apply cancel/replace at that same original key. Replacement may alter range/title/status/note; its moved endpoint is not original occurrence identity. Window evaluation considers ordinary candidates and all finite explicit replacements that may move in from outside, then filters by actual range. Budget exhaustion is partial, never complete with missing moved-in events. Occurrences do not automatically create Nodes; explicit promote/adopt does. Series edits preview keep/remap/drop of exceptions. Per-occurrence participant/reminder overrides absent from the chosen schema retain original import data and unsupported reports, without claiming full RFC compatibility.

Calendar/holiday packs provide context and retain pack/version/source/applicability in results. Multiple packs coexist without a single isHoliday fact. ICS UID/RECURRENCE-ID/SEQUENCE are source identifiers, not EntityRefs; VFREEBUSY/VTIMEZONE create no automatic note, and VTODO/VJOURNAL require explicit preview mapping. SOURCE owns complete native Period/Range/Event schema; these are concrete Features consumption obligations, not “reuse the old service.”

Optional detail is not mandatory for common records; item IDs are neither blanket-required nor blanket-forbidden. Schema/relation migration previews originals, affected fields and unknown values. Preserve unknown source with read-only display rather than deleting it and calling the result simpler.

## 9. Independent JSON annotations and atomic suggestions

Use node-owned `weftext.annotations.json`. Records have annotation ID/reply relations/attribution/time/purpose (comment/mark/suggestion)/body/labels/appearance/open-resolved state. Body may use a restricted Typst inline profile with text/emphasis/math/qualified links and no import/file/network/execution. Invalid formatting retains exact text and diagnostics instead of deleting discussion. Unknown data preserves originals; edit only understood records with safe patches. A corrupt file is never overwritten by empty JSON. Offer originals/reliable backup recovery and unreadable-state explanation. Best-effort recovered discussion is marked recovered/unverified and receives no executable-suggestion qualification.

```text
AnnotationTarget = whole_document(owner)
 | source_element(owner, revision, sourceRange, elementRole)
 | source_range(owner, revision, byteRange, sourcePreimage, visibleExcerpt)
 | whole_resource(resource)
 | pdf_region(resourceRevision, pageIndex, pageBox, rectOrTextRange)
 | image_region(resourceRevision, pixelSize, rect)
 | media_interval(resourceRevision, timebase, startTick, endTick, optionalRegion)
```

Coordinates have closed units/origin/rotation/page dimensions or timebase, bounded ranges and valid endpoint order. Never mix microseconds/frame numbers/post-crop coordinates. PDF text ranges depend on the exact revision's extraction product; normalized image coordinates must round-trip to fixed dimensions; video regions combine time and frame-local region. Missing UI support cannot downgrade to whole attachment. Flow/print selection yields the same true source target. Generated/multiple-origin content permits whole-document or explicit real-source annotation; DOM coordinates are not persistent source addresses. Commit a source Draft before requalifying a new range; a failed save retains the annotation Draft.

Target resolution is independent of discussion status: exact/mapped/stale/ambiguous/orphaned/unavailable. Automatic relocation across revisions requires Core's continuous exact edit map and an unchanged range. External change without continuity yields stale/orphaned; similar-quote search never automatically reanchors. Manual reattach selects a current target and previews it; pending suggestions require reconfirmation. Invalidated ranges/elements remain in unlocated discussion lists with replies and resolve/reopen, without deleting JSON/replacing stored summaries/attaching to namesakes.

Annotation reads disclose already stored authorized contents, including sourcePreimage/attribution, without hidden-target reads/new screenshots/context computation. Current context needs separate source/resource read. Inserting a quote requires readable origin and writable destination Draft, not writable origin. The pasted quote is destination author content and proves no target currentness.

Suggestions have explicit replacement/target basis and pending/accepted/rejected state. apply specifies annotation ID/current annotation basis, not a caller's secret replacement. Core rereads the pending suggestion/current target/source read-write permission, validates unchanged range/sourcePreimage and creates one Plan with source after + annotation accepted after. User previews both exact effects; one opId commits both in one logical decision. Conflict/revocation/unknown fault retains pending state and Draft. Recovery follows OperationJournal and never calls one installed file accepted. Concurrent apply/reject share annotation before generation and have one winner. Reject modifies only annotation without reading hidden target. Reconfirm is a distinct checked preview operation; models cannot manufacture consent/attribution time. Undo is a new full inverse transaction and conflicts if source/annotation changed, not a state-only rollback.

### 9.1. JSON stores discussion; exact source revisions/ranges locate targets

Annotations remain independent in node-owned `weftext.annotations.json`. Body, replies, attribution, resolution and target history are separate from `.typ` body. Lost/unreadable/corrupt targets never justify deleting ideas or discussion. Preserve corrupt sidecar originals and diagnostics instead of replacing them with an empty array. Unverified recovery does not qualify suggestions for execution.

Body annotation identity uses full NodeRef (workspace+note identity), a valid Core RevisionId and an exact UTF-8 half-open range in that source revision. RevisionId identifies historical source, without current read/write permission. Core creates the target on a committed authorized SourceSnapshot using official AST, final origin mapping and current selection. Draft serial/DOM path/screen coordinates/layout page/equal text are not source addresses. Whole-document, element, source-range and media-range targets are distinct.

Native labels help paragraph navigation, but are not mandatory block IDs for every annotation. Annotations do not automatically label every paragraph in the library. A surviving label does not prove an old selection/revision unchanged.

### 9.2. Raw source bytes and visible excerpts are distinct

The source preimage is the exact raw range used to verify location/edit before state. The visible excerpt is the logical text seen/selected by the user, used for discussion/static quotation. They can differ; comparing plain excerpt text to raw source is not the same check.

Suppose `[128,139)` in a longer source is these 11 UTF-8 bytes:

```typst
甲*乙*丙
```

The user sees 甲乙丙 with 乙 emphasized: 9 UTF-8 bytes and 3 Unicode scalars of plain text. The source includes two `*` characters; the excerpt does not. Selecting only 乙 may map to the unique ordinary leaf `[132,135)`. Selecting all three may include emphasis syntax in the source range, but current AST/map must prove that span and operation. Never derive source end from the 9-byte excerpt length.

This JSON illustrates responsibilities, not a frozen wire/persistent schema. IDs/revision are illustrative; Core issues/verifies actual values. Equal text/summaries do not manufacture them:

```json
{
  "annotationId": "82b289ba-01ae-4c7b-8e51-779442316658",
  "target": {
    "kind": "source_range",
    "node": {
      "workspaceId": "58c47256-bfce-4d29-a037-b83a3a644fb0",
      "nodeId": "12d7c4f3-9ee6-4c36-920d-ec6af27bc654"
    },
    "revisionId": "202173ac-0e45-45cb-8998-7289b1688b0c",
    "sourceRange": {"unit": "utf8-byte", "start": 128, "endExclusive": 139},
    "sourcePreimage": {"utf8": "甲*乙*丙"},
    "visibleExcerpt": {"text": "甲乙丙", "kind": "copied-logical-text"}
  },
  "body": "Explain how 乙 relates to its surrounding text.",
  "reviewState": "open"
}
```

sourcePreimage may be Core's retained exact historical slice or a revision reference that allows exact retrieval. Any digest is auxiliary only, not independent location/auth/write revival. Stored visibleExcerpt is annotation content governed by annotation permissions; reading it does not reread target body or compute new hidden context. Raw source containing private material outside the excerpt cannot be disclosed merely because an annotation is saved. Core constructs from the authorized target scope under SOURCE field/body isolation.

### 9.3. Formatting, multiple paragraphs and generated content

Selections crossing strong/emph may have exact same-source ranges containing syntax delimiters. A suggestion needs EDITOR's verified safe operation rather than replacement of the entire raw range by visibleExcerpt. Apply validates exact source preimage and currently consumed semantics/dependencies separately, preserves outside bytes/native containers/target text meaning and reparses fully. Source mutation and accepted state share one Plan.

A selection crossing paragraphs may use one range only if both paragraphs and their separator are contiguous in the same exact authored source and AST/map proves it. The visible excerpt independently retains logical line breaks. Disjoint source fragments cannot become a bounding range that swallows unselected text. If the first stage lacks compound multi-range targets, retain annotation Draft and offer whole-document annotation or separately selected exact ranges, explicitly explaining the difference; never invent a root range.

Included text targets its actual source owner/revision, not root coordinates. Repeated include/function/loop display occurrences must first be distinguished. Equal source spans do not prove a root writer. Generated/multiple-origin text without unique origin permits whole-document annotation or explicit selection of readable real source, not a fabricated root slice. Retain historical excerpt while marking unavailable/ambiguous source targeting. A persistent generated-occurrence target requires separate design, not a hidden expansion here.

Explicitly convert UTF-8 bytes/UTF-16/scalars/graphemes/layout coordinates. Selections do not split combining/ZWJ/CRLF; copy uses logical reading order. PDF/image/audio/video annotations keep exact resource revision/page/pixel/region/timebase, without fictitious `.typ` text ranges.

### 9.4. Relocation requires actual edit mapping

Retain original revision/range/excerpt and separately report current exact/mapped/stale/ambiguous/orphaned/unavailable resolution. Core's continuous exact edit map may relocate unchanged ranges. Inserting text before a range moves its location without permanent text IDs. Overwrites/splits/ambiguity/watcher gaps/external edits without continuity/replacement/unavailable history retain discussion as awaiting relocation. Quote/hash/namesake-label matching never guesses automatic reanchoring.

Deleting/recreating equal bytes never revives old ReadBasis. Current reading/suggestion application reauthorizes; accessible exact history is not current writing. Manual reattach explicitly selects a current target, previews before/after and persists target history via Core. Pending suggestions require reconfirmation. Ordinary body/reply/resolve changes do not silently change targets.

### 9.5. Native paragraph labels and Copy paragraph link

Typst does not automatically allocate permanent IDs to every block. A real block can have a native label and a native link:

```typst
#block[
  这里是一段需要直接引用的说明。
] <para-method>

见 #link(<para-method>)[这段说明]。
```

The author may rename/delete the label. `@para-method` additionally depends on referenceable-object/numbering rules; ordinary paragraph navigation uses `#link(<para-method>)[...]`, without promised paragraph numbering. A cross-note link uses full ID plus exact fragment:

```typst
#link("weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654#para-method")[方法说明]
```

Copy paragraph link:

1. Currently authorize and locate the actual paragraph/block/origin. Reuse an existing exact unique label bound to that target without renaming it. Relevant source read/address-disclosure permission suffices; no source write is required.
2. Without a label, clearly disclose that this operation adds a native target label. Only writable source and EDITOR proof of a safe block wrap/existing-element label permit a source proposal. A required wrapper is part of the reviewed change, not an allegedly harmless ID.
3. Choose an ordinary label outside reserved application-role prefixes, such as `para-<random-suffix>`. Check the complete current document/dependency label set. Unproved uniqueness/generated collision/uncertain binding denies shortcut creation. This is neither NodeID nor a global block registry.
4. Construct through official AST/source writer, reparse and compare semantics, proving the real target and preserved unselected bytes. One Draft operation/Undo enters unified prepare/commit. Cancel creates no label; conflict retains Draft and requires new preparation. A wrapper that cannot preserve original flow/layout/counters is unavailable with Source repair/existing-label entry.
5. Deliver a newly created label's link as saved only after source commit and authorized current resolution. A Draft link is explicitly not effective; retain Draft without pretending others can navigate it. unknown first queries the original opId rather than allocating/recommitting another label. Report successful save and failed clipboard separately; retain the label with Retry copy instead of fictitious rollback of committed author data.

Create/reuse a label only for the explicitly chosen paragraph, not every paragraph/document/library. Read-only unlabeled origins permit static excerpt+whole-document link or an existing readable label. Quoting does not require source write and cannot fabricate a fragment absent from saved source.

### 9.6. Current navigation, fixed historical refs and static quotation

| Form | Meaning and qualification |
| --- | --- |
| Current paragraph link | NodeRef+label resolves current authorized data. Node rename/move retains identity; label rename/deletion/duplication/generated-binding change may break it without approximate guessing. It does not guarantee unchanged wording. |
| Historical annotation target | NodeRef+RevisionId+exact source range describes the original target. Proven current mapping is separate; original history is retained. |
| Explicit fixed reference | Opens exact historical revision/range through Core's historical read/reference capabilities. Missing/revoked history is unavailable, never current fallback. External encoding/copyable historical URI needs separate specification; ordinary NodeID#label is not fixed. |
| Static quotation | Copies authorized visible text into writable destination body with source link/optional stored historical description. Text becomes destination author content and does not follow source changes. Current link content may differ; UI explains copy time/historical selection. |

Static quotation needs readable origin and writable destination Draft, not writable origin. Creating a source paragraph label is a distinct authorized source write. Reading/copying grants no label creation. Across separate authorities show source-label save and destination-edit outcomes separately, without cross-authority atomicity.

This scope covers static excerpts/source navigation/explicit historical annotation only. Live transclusion requires separate design for current reads/cache/revocation/cycles/dependencies/origin boundaries/export. This addendum neither claims it completed nor adds liveembed scope.

## 10. Templates, copies and bibliography import

Layout templates use standard Typst modules/imports/functions/set/show. Packages see frozen authorized inputs/fonts/resources, without ambient file/network/trusted identity/author writes. Application creation templates define parameter types/defaults and precise title/body/field slots; ordinary UI does not require recipe/wire authoring. Missing/null/explicit input differ and defaults apply only to missing input. Text parameters are safe literals, so `#metadata` input does not execute. Preview includes all new notes/resources/typed refs/annotation preservation or omissions/specific losses and binds the same Plan as execution.

Copy enumerates the complete authorized selection closure, allocates one fresh note/resource/annotation/definition-key/candidate-RevisionId map, traverses every typed ref/link/definition slot via official AST/schema, resolves paths in the old context and encodes them in new source locations through that same map, validates targets/fragments/owners/definition selection, freezes all after images/losses and atomically commits. Do not replace free text/unknown fields/CEL strings/UUID-like phone text. If unknown application definitions prevent complete typed-ref proof, reject full copy or offer an explicit original-preserving/loss plan. Mutual note refs, ref-free definitions and multiple instances of one source associate by full source/result keys, not array position. Resources explicitly re-own rather than share private assets because paths match. Proven annotation targets update through the map; unprovable ones retain discussion with stale state or an explicit omission choice.

Weftext-owned Rust import first retains originals, safely parses complete parts/sheets/resources/unknown data, builds exact native values/semantic structures, locates every unrepresentable feature and describes effect/options, previews and creates via Core. PDF compilation is not business fidelity; renaming extensions is not import. Formula/hidden sheet/cache results remain distinct. Macros/malicious ZIP/XML/extra worker output do not execute. Large jobs may commit bounded groups with a queryable committed prefix. Cancel stops later groups; unknown checks the original group opId without reimporting the prefix.

A paper note's `publication` record is the single editable authority for formal bibliographic data; there is no separately editable Bib database. BibTeX/CSL/RIS/DOI/provider import maps to that record, retaining original-record attachments and field source mappings. Citations refer to note identity. Bib/CSL data for Typst compilation is a frozen projection from current authorized literature snapshots. publication.title is the formal work title, distinct from document title; if used as document title, generate it from publication.title rather than writing it twice. Refresh presents a change preview and leaves author-modified fields as conflicts for choice rather than overwriting from provider latest. DOI/ISBN/arXiv identify works externally, not Nodes; repeated import offers explicit matching/choice, not blind upsert.

```typst
#metadata((
  id: "8aa10c08-3e7d-4d10-9488-92bc7f393050", types: ("paper",),
  publication: (
    kind: "article", title: "原生笔记中的结构化数据", citation_key: "chen2026native",
    contributors: (
      (agent: (ref: "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654"), credited_as: "陈明", family: "陈", given: "明", roles: ("author",)),
      (agent: (text: "李华"), family: "李", given: "华", roles: ("author",)),
      (agent: (text: "原生数据研究组"), roles: ("author",)),
    ),
    identifiers: ((scheme: "doi", value: "10.0000/example"),),
    issued: (calendar: "gregorian", precision: "month", year: 2026, month: 9),
    publication_history: (
      (state: "accepted", during: (start: datetime(year: 2026, month: 7, day: 20))),
      (state: "published", during: (start: datetime(year: 2026, month: 9, day: 1))),
    ),
    container: "知识工具研究", volume: "12", issue: "3", pages: "101–118",
    abstract: "A sample paper for checking the complete chain.",
    keywords: ("Typst", "Structured notes"),
  ),
)) <weftext>
#context {
  let record = query(<weftext>).first().value
  title(record.publication.title)
}

= Reading notes
The body records questions/discussion. PDF is a resource of this note, not a second work.
```

This is fictional; DOI is not a real external record. A positive fixture must use authorized resolvable Person, optional venue(Work|Org), PDF ResourceRefs and installed paper schema. Container text is not a venue ref and creates no journal. Formal data uses SOURCE's single publication shape. The generated body title has no counterfeit writer; the form edits publication.title once, and EDITOR supplies a true field-edit entry point. agent has exactly a ref or text branch and roles is an array; unknown/corporate authors need no forced Person creation. Check contributor order in form→same-item ordered Query→CSL/Bib→citation numbering under one frozen database→DOCX bibliography. Retain unknown date precision without day 1. Unknown roles/provider members preserve originals/diagnostics rather than becoming absent.

## 11. Editable output and intranet downloads

Exact-source export returns original `.typ` bytes without layout/templates. Exact structured-result export returns complete schema/Cell/value/order without runtime handles/writers. PDF/SVG/PNG use available real Typst outputs; experimental capabilities stay labeled experimental. DOCX/XLSX have separate Weftext converters and actual verification routes.

DOCX contains real paragraphs/runs/styles/tables/images/relationships/refs, not page images posing as editable text. Title/subtitle retain SOURCE roles; titleless notes gain no author title. A run-in heading defaults to the whole paragraph in `Heading N`, participating in numbering. Body runs reset inherited character properties to template body formatting while retaining explicit authored strong/emph. Preserve DOCX Heading 1–9 within the native target range; 10+ preserves source and reports unsupported/loss without collapsing to 9. A normal Word TOC may contain the whole paragraph as the stated default; title-only TOC is separately optional. Verify numbering/TOC/refs in actual Word/WPS/LibreOffice and state each application's support rather than treating valid OOXML as interaction proof.

XLSX is a real workbook/sheets/typed cells with explicit table/Query order. Phone/identifier values are text. Values exceeding numeric precision use exact text or explicit loss choice, without prior rounding. Civil date/time/local datetime adds no host zone; instant retains explicit offset or exact text. Nested arrays/records default to readable structure text plus exact subsidiary tables/structured exports; explicit unnest retains source ordinal/duplicates. Strings are string cells, so `=...` does not execute as a formula. Imported formulas use explicit permitted formula slots only. Merge/header/empty/null/missing distinctions are not silently flattened. A PDF table is not a workbook.

Offline cross-document routes cover every RenderedLink occurrence in the frozen projection: actual target→unique output file→actual renderer named destination. Do not assume a source label is a Word bookmark/HTML ID or merge repeated include entries. Each occurrence has exactly one route or explicit loss; omitted targets cannot be claimed navigable offline. Routes change artifacts only, not source. External URLs retain target-format behavior without granting network/resource permissions.

Intranet flow is inputs/format→Job generates and validates exact artifacts→inspect outputs/losses→confirm the same frozen plan/bytes→download. No external publication directory or publication receipt is required. Artifacts bind inputs/environment/schema/current delivery permission/digest/length/output name. Every request/chunk/resume checks current qualification, and resume returns the same bytes without recompilation. Revocation stops further delivery; expiry explicitly requires new generation/confirmation instead of silently changing content. Authorize before existence diagnostics. Filename/ContentType come from controlled output; extra worker files are quarantined. Generated/confirmed/delivered/printed/saved-as-Resource/published/unknown are separate outcomes. Download changes no author facts; saving a Resource is another Core transaction.

## 12. Agents, automation and external effects

Retain selected agent/automation/connector permission and outcome semantics using current Plan/Job/OperationJournal; lack of a new requirement number does not delete D10. Agent reads/author writes use identical Core scopes. Models/documents/web/email/tool output are data, never approvals. Bounded delegation states principal/scope/actions/count/time/budget. Field grants do not become body access, and package execution does not get ambient workspace/network/secret. Exact package/font/provider admission supports offline intranet without silent networking.

Automatic author changes freshly prepare complete effects and match an authorized finite rule or interactive confirmation. Out-of-rule effects stop for the user instead of widening scope. Rules are not model summaries and approvals bind exact Plans. Scheduler uses stable occurrence/run keys so restarts do not redo handled occurrences; rule changes do not replace unknown Runs. stop/revocation is checked with an atomic fence before first irreversible send/commit: if it wins, execution stops. Preserve already-sent/committed facts rather than fictitious refunds/rollback. Cancel stops unstarted work; unknown read-only reconciliation retains the original request.

External mutation first freezes exact account/target/payload/idempotency key, displays the full real request and receives authorized user confirmation. Broad grants/read permission/export confirmation do not approve sending. Distinguish prepared/submitting/succeeded/failed-no-effect/unknown. Automatically retry unknown only with reliable same-request idempotency or no-effect evidence, never a new ID. Credential rotation cannot redo an old unknown operation against another account. Compensation is a new authorized effect. Report Core author transactions and external mutations separately, without a fictitious combined atomic Receipt.

Before potentially charged model/tool/network calls, atomically reserve a bounded amount under the actual account/currency/pricing version to prevent concurrent overspend. Distinguish settled/released-no-charge/uncertain. unknown or stop does not erase potentially incurred cost; grant renewal does not reset history. Cost and author effects are separate rollback states shown separately. Secrets enter no public logs/Query/source/artifacts. Workers read limited input/output slots without direct workspace mounts/arbitrary commands. The first stage adds no automatic whiteboard/persistent multi-Query dashboard/reminder execution/complex workflow/real-time collaboration.

## 13. Coverage, prototype risks and design checks

These are unrun design oracles. This author draft inherits no old CLOSED result or implementation acceptance from AST experiments/compilation.

| ID | Positive case and critical counterexample |
| --- | --- |
| F01 / RQ12–13 | Four-form conversion preserves target/fragment/content, shortest path round-trips and literal broken-link copy works. Duplicate label/boundary/revocation/shared AST denies shortcut writes while preserving source. |
| F02 / RQ14 | 白云山/山/山南路, same-range A/B, RTL/overflow/pages are completely accessible. Partial-grapheme discovery is read-only; partial is not complete; no default target. |
| F03 / RQ08–09/15 | Item 1 number cannot pair with item 2 label; duplicates/order/notes survive. Numeric/date/currency types do not implicitly coerce; absent/null/invalid/unavailable differ from complete empty. |
| F04 / RQ06/15 | Generated deep headings remain queryable at real level and duplicate includes are distinct occurrences. Groups have no inherited writer; partial aggregates are not exact totals. |
| F05 / RQ16 | current explains updates and pinned fixes definition with current data. Missing history never falls back latest; trash is read-only; old rows have no writer. |
| F06 / NAV01–05/RQ11/25 | Wide/narrow return retains Draft/filter/anchor, asynchronous preview preserves focus, shortcuts/invalidation/pins/preferences honor their scope; field-only reads do not fetch body/counts. |
| F07 / RQ17 | Task body/fields/board/calendar agree, civil due adds no zone, periods find existing and enforce concurrent uniqueness. No generated/aggregate row actions. |
| F08 / RQ18 | Accurate PDF/image/media coordinates, corrupt JSON retained, orphaned discussion preserved, hidden targets not freshly read. Source+accepted state share one logical decision and recovery/races preserve one winner. |
| F09 / RQ19/24 | Mutual links/multiple instances/ref-free definitions/resources/annotations share one map. Unknown typed slots/loss drift block execution; unknown queries original opId. |
| F10 / RQ10 | Paper form/Query/CSL/citations/Word retain contributor order, refresh conflicts preserve author changes, external IDs are not identity and unknown precision/fields retain originals. |
| F11 / RQ04–07/20 | Actual editable DOCX, run-in body formatting/emphasis, Heading1–9 and 10+ loss; real XLSX without mistaken formulas/rounding. Validate actual Office applications. |
| F12 / RQ20/23 | Intranet download needs no publish, resumes confirmed same bytes and stops on revocation. Offline occurrence routes fully cover links and extra worker output is withheld. |
| F13 / RQ21–24/D10 | Test authorization/external approvals/cost/stop/unknown together; same-document conflicts retain both Drafts, unrelated notes continue and cross-authority atomicity is not invented. |

Prototype priorities: complete official final origin traces for links/headings without counterfeit writers; Aho–Corasick/dense overlap accessibility; nested same-item/empty-state/completeness Query and field-only authorization; navigation state preservation during revocation/Draft switches; multi-file annotation recovery; editable Word run-in headings and real XLSX precision; chunk-delivery authorization boundaries. Literal metadata projection need not wait for whole-document compilation, but declarations and shortcut writes require official AST/exact binding. Preserve source/Draft with explicit capability state where proof is absent, without a Features string parser bypass.

Ownership: this document directly handles RQ08–20 consumers and NAV01–05; SOURCE/EDITOR provide RQ01–07 language/editor/title behavior with output continued here. CORE/EDITOR and Features jointly check RQ21–25 surfaces/storage/auth/transactions/context. Stable directory/tag shortcuts, a single Type/Value, definition-key creation and full Calendar schema are explicit coordination interfaces; old protocol family names do not deliver them. There are no new compatibility decoders for undeployed candidates. Real historical data gets a separately scoped explicit import with original preservation.
