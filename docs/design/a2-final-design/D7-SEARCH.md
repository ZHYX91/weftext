---
source_language: zh-CN
translation_of: D7-SEARCH.zh-CN.md
translation_status: synced
---

[简体中文](D7-SEARCH.zh-CN.md)
# A2 D7 Search Semantics and SEARCH-01–08

Status: normative D7 search design inside the current author candidate. D8 interaction implementation remains a later full-module batch.

## 1. Decision and alternatives

Three complete interaction alternatives were compared.

| Alternative | Strength | Cost / risk | Decision |
| --- | --- | --- | --- |
| Plain text plus visual filters only | zero syntax learning, no lexical ambiguity | keyboard-heavy for expert use; hard to copy a complex condition | retained as the default ordinary experience |
| Automatically parse familiar field-colon text in the ordinary box | short and familiar | confuses prose containing title:, URLs, drive letters, CJK punctuation and incomplete input; error fallback can execute a different Query than intended | rejected |
| Plain text default plus visual filters plus an explicit shortcut-condition mode | no syntax required, deterministic expert path, round-trips to one Query | needs one small parser and explicit mode indication | selected |

The selected model has one execution authority: every successful search state compiles to the existing D7 Query algebra and executes through the same D6 authorization, complete-range, budget, result, paging and reset contracts. There is no second full-text executor and no durable search object distinct from saved Query.

## 2. Search object vocabulary

Current D3/D2 has no generic authored Node name that can be silently equated with all file labels. The search vocabulary therefore separates:

- document title: current D2 native document title, nullable;
- document subtitle: current D2 native subtitle, nullable;
- file name: the current D6 physical FileBinding basename where that metadata is authorized;
- path: the current D6 physical relative path where path disclosure is authorized;
- resource name: the current Resource FileBinding basename, distinct from a Document title;
- logical hierarchy: the D3 parent/order scope used to enumerate Nodes, never a stable path identity;
- body: D2 semantic body_text under the complete current D2 evaluation barrier;
- contributed text: an explicitly selected current SearchContribution Field/textPath.

A titleless Document remains titleless. A filename, path, list placeholder or first paragraph may be displayed as a UI fallback, but it never becomes title data. Duplicate titles, duplicate filenames in different allowed locations, and equal contributed strings do not merge identity.

Rename or move changes current filename/path matching while preserving NodeRef when D3 says identity is preserved. An old result therefore becomes stale under its ordinary currentness rules; saved Query semantics are re-executed against the current authorized FileBinding instead of freezing an old display path as identity.

## 3. Three product search presets

File-list filter operates over the already selected logical list scope. Default scope is the current D3 parent, nonrecursive, live Nodes only. It does not read body text, Annotation bodies, Trash, attachment extraction or OCR. A UI may explicitly enable recursive subtree or Resources; those switches compile to explicit Query scope/source choices and require their own disclosure. Empty input in this surface means browse the selected list scope; it is not an empty full-workspace search.

Quick open defaults to authorized live managed Nodes in the selected Workspace/root scope and reads title/subtitle plus separately authorized filename metadata. Body, Annotation bodies, attachment extracted text, OCR and Trash are off by default. An explicit Resources toggle includes authorized Resources and resource-name matching. Quick open is still a Query preset, not identity lookup by text.

Global text search defaults to authorized live Documents in the selected Workspace/root or explicit subtree, with title/subtitle and body enabled. Resources, attachment extraction/OCR, Annotation bodies and Trash are explicit opt-in scopes. Attachment content is available only through the real current extraction/SearchContribution dependency. Annotation text requires annotation disclosure/read before obtaining the body. Missing provider, incomplete extraction, hidden source or unmaterialized placeholder is unavailable for a complete search, never a successful empty match.

## 4. Match, Unicode, Boolean composition, and sort

The current generation preserves the retained D7 SearchContribution matching boundary: deterministic NFC comparison and exact scalar substring matching, with no automatic fuzzy matching, stemming, token-language inference, transliteration or Pinyin. Source bytes are not normalized by the comparison. Case behavior is the retained case-sensitive comparison; a future case-fold profile requires an explicit versioned semantic addition rather than a UI-only toggle.

CJK works as ordinary Unicode scalar substring matching and requires no whitespace tokenizer. RTL affects presentation only. A quoted phrase is one exact scalar sequence. Plain ordinary search text is one literal phrase; it is not implicitly split into terms. Visual filter rows default to AND, and the visual builder offers explicit OR groups and NOT where the underlying Query expression is representable.

In shortcut mode, adjacent primary terms are AND, while OR, NOT and parentheses use the grammar below. Match ranking and role ordering remain the exact retained D7 SearchContribution rank defined in the copied Query Algebra owner text; UI code cannot add a private fuzzy score. Where the retained comparator is equal, Query appends the canonical subject key and then the selected contribution identity as an explicit stable tie so pagination is deterministic. General Query sorting remains user-defined and never follows viewport order.

## 5. Shortcut-condition mode

Shortcut parsing is opt-in. Merely typing a colon never enables it. The visible mode indicator is interaction state; once parsing succeeds, the semantic object is the compiled Query condition.

```text
Shortcut mode v1

plain-token        := escaped-token | quoted-value
field-condition    := @title:value
                    | @subtitle:value
                    | @filename:value
                    | @path:value
                    | @body:value
                    | @resource-name:value
                    | @field(field-id[,member-path]):value
value              := quoted-value | escaped-token
quoted-value       := "..." with backslash escaping for quote and backslash
boolean-expression := primary
                    | NOT primary
                    | primary AND primary
                    | primary OR primary
primary            := plain-token | field-condition | ( boolean-expression )

precedence: parentheses > NOT > AND > OR
whitespace between adjacent primary terms is AND
a colon is syntax only inside a recognized @ operator
unknown @ operator or invalid field/member path is a parse error
incomplete input is a draft parse state and executes no Query
```

The @ prefix is deliberate. Ordinary title:, https://example.test, C:\\notes\\a.adoc, time 12:30, quoted prose, and CJK full-width punctuation remain literal unless the user explicitly enters shortcut mode and uses a recognized @ operator. To search text beginning with an operator spelling inside shortcut mode, quote it or escape the leading @.

Unknown @ operators, unknown FieldIds, unavailable Field definitions, illegal member paths, unmatched quotes/parentheses, or invalid values are errors at their exact draft span. The UI executes no fallback literal Query. Incomplete input is preserved as an editable draft and likewise executes nothing. This prevents an error from silently becoming a broader search.

The first-generation built-in keyword set is title, subtitle, filename, path, body, resource-name and field. There is intentionally no generic file or name keyword because those words collapse distinct domains. field requires a stable D4 FieldId and, when present, an explicit member path validated against the same current Registry used by Query. Localized labels and aliases never replace the stable ID.

## 6. Visual filters and round-trip

Plain input, visual filters and shortcut mode all edit one condition model. A representable Query condition can round-trip losslessly to visual chips and shortcut text. Reordering presentation chips must not change semantics.

A complete Query may contain a condition outside the convenience grammar. That condition is retained as an opaque-but-editable advanced condition node in the visual model; switching to shortcut mode must show that it is not text-representable and must not drop it. The user may open the full Query editor to edit it. Saving never serializes only the visible chips.

Keyboard, pointer, touch and assistive-technology controls invoke the same condition operations. Focus remains bound to a logical condition ID, not a DOM index. Search refresh, sort or epoch reset invalidates result-row focus/selection/evidence and returns focus to a stable container rather than transferring a reused visual row to another object. IME preedit never dispatches a Query; only finalized input may update the search draft.

CJK IME, RTL text, bidi isolation, screen-reader labels, mobile sheets and hardware keyboard paths are D8 interaction obligations. This D7 batch freezes their semantic targets and acceptance requirements but does not claim platform execution.

## 7. Permission, index state, count, ranking, and snippets

Authorization and minimum disclosure precede every sensitive source, Field, contribution, attachment, Annotation or index-private read. Search does not learn hidden existence from hit counts, ranking gaps, snippets, completion suggestions or unavailable reasons.

Derived Index is only a candidate accelerator. building, partial, stale and unavailable are distinct from a proved complete zero-result Query. A complete search requires current authorized query_scan plus all real positive/negative source, Registry, contribution, extraction and authorization dependencies. A partial exploration may show only its explicitly covered range and must remain visibly incomplete; it cannot issue complete ActionEvidence or claim that the current page is the whole result.

Missing selected Field/SearchContribution/provider/extraction data is evaluated after the ordinary disclosure gate and returns the retained unavailable/reset result. It never falls back to an empty contribution. Budget exhaustion fails the complete result under the retained D7 budget error; it does not return the first N rows as complete. Cancellation has the retained explicit outcome.

Hit count is computed only from the authorized complete result. A UI may omit a count or say unknown while building; it cannot estimate hidden matches. Ranking uses only authorized matched values and the retained D7 comparator.

Snippets and highlights are derived only from the already authorized matched semantic text. Their scalar offsets are ephemeral presentation offsets inside that result value; they are not D3 Locator coordinates. Opening a hit performs a fresh current resolution/read through the actual Ref/provenance/Locator rules. If the result or source is stale, open/search revalidates or resets instead of mapping a display offset onto new bytes.

## 8. Persistence, reopen, copy, and import

Saving a search stores the existing canonical Query definition and its explicit scope, parameters, stable D4 FieldIds, selected SearchContribution contributionId/version dependencies, and author-defined ordering. The shortcut string is not durable search authority. A non-author device may remember parser version or the user's last text for convenience, but this state is discardable and never enters DynamicBlock, DefinitionTransfer, result cache or ActionEvidence.

Reopen decodes the saved Query under its recorded author schema and then qualifies current definitions, Registry, contributions and permissions. Field deletion or incompatible type change follows the Registry migration/unavailability contract. SearchContribution removal/version change follows the D10 activation dependency rule. Permission changes reset current results. Rename/move changes current filename/path values but not stable content identity.

Copy/fork/import uses existing D7 Definition Transfer over the canonical Query payload. Typed Refs and DefinitionAddress roots are mapped by the D3 rules; FieldIds and SearchContribution stable identities remain semantic dependencies rather than being guessed from labels. Unknown payloads never convert themselves into a new search syntax. Old result rows, snippets and evidence are not copied as current authority.

## 9. Hit equivalence across list, full search, and saved Query

When file-list, global search and a saved Query are configured with byte-equivalent canonical conditions, scope, contribution set, ordering and current dependency cut, they have the same D7 result semantics. Surface pagination or virtualization does not change membership or ordering.

Capability differences are explicit. A surface that cannot obtain a required provider, complete range, secure snippet, bidi interaction or assistive navigation reports that capability unavailable/incomplete; it does not run a different hidden query. Cross-device equality is semantic equality of the canonical Query and qualified dependencies, not pixel equality.

## 10. SEARCH-01–08 acceptance

| ID | Normative D7 closure and later D8 acceptance |
| --- | --- |
| SEARCH-01 | File-list filter, quick open and global text search use the scopes in §3; browsing empty input is separate from explicit search; recursion, Resource, Annotation, Trash, body and attachment/OCR scope are explicit. |
| SEARCH-02 | Title/subtitle/filename/path/resource-name/hierarchy remain separate; titleless stays titleless; duplicate display values preserve distinct Refs; rename/move invalidates old result qualification without changing identity by text. |
| SEARCH-03 | Current match is case-sensitive NFC exact substring over Unicode scalars; CJK and RTL are deterministic; Boolean AND/OR/NOT is explicit; no fuzzy/tokenizer/Pinyin/stemming claim; unsupported future modes do not silently run. |
| SEARCH-04 | Shortcut mode is explicit and uses @ operators, quoting, escaping, precedence and errors from §5; ordinary colon text, URLs, drive letters and title: remain literal; invalid/incomplete shortcut text executes no alternate Query. |
| SEARCH-05 | Visual and shortcut conditions edit one model; nonrepresentable advanced conditions are retained, not dropped; focus/keyboard/touch/AT/IME/CJK/RTL semantics follow §6 and later D8 platform evidence. |
| SEARCH-06 | Permission precedes sensitive reads; hidden counts/ranks/snippets do not leak; index building/partial/stale/unavailable differs from complete zero; missing contribution/provider and budget have explicit unavailable/error outcomes; ordinary open/edit/save remains independent. |
| SEARCH-07 | Save/reopen/copy/import persists canonical Query and stable semantic dependencies, not UI state; Field/contribution/version/permission changes requalify or reset; old results never acquire new current eligibility. |
| SEARCH-08 | A hit opens through a fresh current authorized source resolution; snippet/highlight offsets are not Locators; equivalent canonical conditions produce equivalent membership/order across list/full/saved surfaces while capability differences remain explicit. |

These rows are design acceptance obligations. No product search, platform interaction, tokenizer, provider, accessibility, performance, or cross-device test is claimed PASS in this batch.
