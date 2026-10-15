---
source_language: zh-CN
translation_of: INTEGRATION.zh-CN.md
translation_status: synced
---

[简体中文](INTEGRATION.zh-CN.md)

# Contract integration and refactoring order

Status: **bounded local implementation exists; v0.6 integration and acceptance continue in batches** (2026-10-15). Independent design acceptance of the earlier fixed baseline remains historical. The refinement/experiment boundaries in [REVIEW](REVIEW.md) §8 do not describe the current product as wholly unimplemented. See [MVP](../MVP.md) and [ROADMAP](../ROADMAP.md) for actual development capabilities and remaining work; separate passing slices establish neither complete v0.6 nor publication.

## 1. One solution

[REQUIREMENTS](../REQUIREMENTS.md) defines outcomes and boundaries. SOURCE solely defines author roles and Type/Value/schema; CORE defines EntityRef, RevisionId, ReadBasis, SourceSnapshot, Plan, Receipt, CommitManifest, Environment and Job; EDITOR defines Draft, official evaluation origins and interaction; FEATURES defines Query, business actions, links, annotations and output consumers. Repeated explanations cannot create another type system or authorization entry point.

A durable revision establishes historical content; a current ReadBasis qualifies this read/write. They are not interchangeable. Read projections and result rows are not write capabilities. Actions prepare anew, and all internal persistent mutations use one Core transaction; external network effects retain separate Job responsibility. Source and indexes also differ: indexes are rebuildable, while authored content, structure, annotations and actual operation responsibility cannot be recovered from an index alone.

This solution replaces the prior SOURCE/CORE/EDITOR/FEATURES/INTEGRATION technical constraints as a whole. Prior `replacements.json`, lexicons and acceptance records describe only their fixed candidate, not the new runtime. Future upgrades of actually released new-system data still require implementation. Preserve real old materials, identify formats and explicitly import/report loss rather than expanding normal decoders by imagining that old proposals were deployed.

## 2. Requirement owners and vertical checks

These are design and implementation paths, **not executed product tests**. Original requirements remain authoritative; grouping does not remove their positive or negative scenarios.

| Requirements | Current owners | Required connected path |
| --- | --- | --- |
| RQ01–03, §5.4 | SOURCE, EDITOR, CORE | Preserve full valid source → five presentation/editing views sharing Draft → ordinary flowing reading → origin-qualified rich editing → one Draft/save; errors or ineligible controls do not discard source |
| RQ04–07 | EDITOR, FEATURES | Title/subtitle/section roles → same-level standalone/run-in numbering → outline/references → editable Word and accurate loss |
| RQ08–10, §5.2 | SOURCE, FEATURES | Historically selected capabilities → native values/schema → forms/same-item Query → ordered, exact and explainable export |
| RQ11 | SOURCE, EDITOR, CORE | Capture/properties → current draft → actual creation/update → Receipt; unknown does not create again |
| RQ12–14 | FEATURES, SOURCE, CORE | Links/path resolution/potential mentions → currently authorized selection → same-target conversion and one undo; graph indexes grant no write access |
| RQ15–17 | FEATURES, SOURCE, CORE | Search/navigation/Query → views with explicit completeness → actions prepared against real targets; task body/properties agree |
| RQ18 | FEATURES, EDITOR, CORE | Exact original target → independent annotation/discussion → fresh read/invalid state → suggestion applied in one transaction without hidden-target reads |
| RQ19–20 | FEATURES, CORE, EDITOR | Template/import/copy → frozen inputs/map → correct output/loss → confirmation and intranet delivery of the same artifact |
| RQ21–24 | CORE, all consumers | Actual local/sync/Server guarantees → current permission/revision → save/conflict/recovery/retry; local single-user use does not replace hosted multi-user needs |
| RQ25, NAV01–05 | EDITOR, FEATURES, CORE | Navigation/list/document state → personal presentation restoration → current permission/result revalidation; UI preferences do not edit source |
| RQ26 | CORE, EDITOR | Ordinary-file open → explicit dependency root → shared Draft/five views → exact save/Save As/conflict recovery; no automatic Node identity |
| RQ27 | FEATURES, CORE | Read/propose/manual apply → bounded library rules → specific connectors; separately qualify egress, author writes, external effects and costs |
| §5.7.1 / RQ18–20 | EDITOR, FEATURES, CORE; Typst/Office converters | Exact cell/list/equation selection → independent comments/pending suggestions → bidirectional real-host target verification and per-item degradation |
| §5.9 / RQ09,11,17,21–25 | SOURCE, FEATURES, EDITOR, CORE; app/packaging entry points | Tray/independent capture → shared Draft/transaction; reminder intent → authorized scheduling/activation; MSIX install/upgrade/uninstall → draft/library preservation |

Structured-capability checks cover the actual historical D4 types and selected behavior, not just three person/task examples. Richer bibliographic, calendar and task structures cannot be deleted because a table control is missing; real deferrals remain explicit. Common and optional advanced fields have layered UI rather than making every field mandatory.

Library-style requirement §5.3 is owned by EDITOR §§8.1–8.5, CORE §9 and FEATURES §11: select an actual style resource, generate a transparent entry, preserve original note identity/source, follow native overrides and actual dependency invalidation, and distinguish an exact single file from an independent styled project. EDITOR §2.1 implements the five-view requirement in §5.4; three internal categories cannot replace five product experiences, and paged/continuous layout is separate. FEATURES §§9.1–9.6 define original annotation ranges, visible excerpts, paragraph labels and current/historical-reference boundaries.

## 3. Original 21 and later findings

The original prefix is `A2-GLOBAL-6B13-`; old CLOSED states do not transfer. The harm checks below were included in the independent global design review recorded in REVIEW §7; each still requires product implementation verification. When a removed mechanism makes its exact counterexample inapplicable, retain the general harm check without implementing nonexistent protocol branches.

| Original ID | Outcome this candidate must establish | Owner |
| --- | --- | --- |
| P1-01 | Complete copies and mutual references share one frozen identity map, matching preview and result | CORE, FEATURES |
| P1-02 | Run-in presentation saves, synchronizes and recovers with source rather than device-only state | EDITOR, CORE |
| P1-03 | Actual heading depth/numbering/references survive; output limits are accurately reported | EDITOR, FEATURES |
| P1-04 | Legal generated headings remain readable/queryable without fabricated write addresses | EDITOR, FEATURES |
| P1-05 | Explicit Trash queries are authorized, read-only and accurately scoped | FEATURES, CORE |
| P1-06 | current/pinned definition selection is visible; pinned cannot silently change; old rows regain no write authority | FEATURES, CORE |
| P2-01 | Template/copy arguments, plans, previews and actual changes correspond | FEATURES, CORE |
| P2-02 | Journal/file faults preserve original and competing drafts without false half-save success or repeated effects | CORE |
| P2-03 | Workspace, entity and dependency scopes agree, including empty scopes | CORE |
| P2-04 | Requirements, current terms and consumers have complete owners; counts do not replace semantics | All owners, this table |
| P2-05 | Phone text, exact numbers and time agree across forms/Query/export | SOURCE, FEATURES |
| P2-06 | Full legal tables preserve source, reading and output; control limits do not redefine syntax legality | EDITOR, FEATURES |
| P2-07 | Real historical materials can be identified, backed up and recovered without invented undeployed decoders | CORE |
| P2-08 | Initialization/copy/import cannot gain identity or permission through self-asserted metadata | CORE, SOURCE |
| P2-09 | Sync and Server verify their actual trust domains without mixed authorization evidence | CORE |
| P2-10 | Unselected fields, unknown content, duplicate values, comments and original information are preserved | SOURCE, EDITOR, FEATURES |
| P2-11 | Audio/video intervals and resource regions do not degrade to whole attachments | FEATURES |
| P2-12 | Every consumer uses the same actual source snapshot/revision without wrong format or permission bindings | CORE, all consumers |
| P2-13 | Stored annotation summaries follow annotation rights without extra hidden-target reads | FEATURES, CORE |
| P2-14 | new/copied/existing allocation, repeated-source instances and ordered values retain correct identities | CORE, FEATURES |
| P2-15 | Multi-object effects/dependencies have explicit ordering and are not partially committed or recovered | CORE, FEATURES |

Prior Typst TR-01–09, PR618-01 and PRD5F-01 also retain counterexample value: one type model across Query slots; equality classes separate from source representation; explicit none/missing; schema changes checked against the actual predecessor; empty-body input; author links separate from generated occurrences; complete annotation saves; complete calendar dependencies; and complete template parameters/object-result mappings. Current owners reestablish these outcomes without inheriting old wires or adapters.

## 4. Implementation order and entry criteria

The sequence below retains dependencies rather than implying every stage is unstarted. Windows/NTFS local writing, structured information and cross-object features now exist. Hosted five-view editing, Query, annotations, configuration, media, structures, knowledge links, existing format interchange and owner recovery-material consumers have been connected and checked through corresponding HTTP/background-browser batches. Same-owner synchronization supports offline replicas, partial installation, explicit merges, received history and ordinary external-edit adoption; CR14 stable fragments completed a bounded fix. MVP records the latest Compound/missing-root-configuration slice, whose actual HTTP/browser configuration checks cover Rules plus two moves only. New Node/Directory parents, general directory and lifecycle/member-retirement/removal combinations, full semantic-dependency revalidation and final combined acceptance remain current-v0.6 work. Implementation or isolated testing is not availability, and subsequent enhancements do not absorb these gaps.

1. **Bounded refinements and selection prototype.** Apply current requirement/contract fixes while preserving the earlier fixed review as history. First prove real origin mapping/input under EDITOR §2.2 and compare CEL behavior in a real runtime. Repair actual failures with targeted review, without automatically restarting full Pro review.
2. **Minimum vertical slice.** Verify library-note and outside-library file opening in parallel → one Draft/source save/recovery → basic five-view/flow interaction → stable node identity/links. Test external editing races, late compilation and faults from the first slice. Prototype success covers only its explicit scope, not every view/platform.
3. **Structure and organization.** Central schemas, property forms, richer types, Query/navigation/lists/task views; validate same-item collection queries, unknown/exact values and safe local edits together.
4. **Cross-object capabilities.** Independent annotations, templates/copy/import, headings and editable Word/spreadsheet output, intranet downloads, preservation and recoverable Jobs. Complete REQUIREMENTS §5.7 in v0.5: annotation/basic-revision contracts → local proofreading/batch annotations → Word comment export → Word body/comment import → basic-revision round-trip → real Office acceptance; scope and gates belong to [ROADMAP](../ROADMAP.md).
5. **Surfaces and deployment.** Complete hosted multi-user permissions and synchronization through the shared Core, then validate client/offline/revocation/fault combinations. Platforms may ship in stages with unsupported backends explicitly identified; a research probe is not a supported product.

The version sequence added on 2026-10-14 is **v0.6 → v0.6.1 → v0.6.2 → v0.7**. Current v0.6 retains its existing multiuser/sync/authorization completion scope. Then implement REQUIREMENTS §§5.8,5.10–5.12 in v0.6.1: LaTeX equation conversion and native Word equation acceptance → Mermaid source/preview and DOCX images → optional DOC/WPS external-application adapters → native references, relationship classification, footnotes/endnotes and caption output → document language/region and import/proofreading applicability → new-consumer sync, authorization and revocation regression. EDITOR owns input, conversion confirmation and source/preview consistency; FEATURES owns format mapping, image output and losses; CORE owns original/resource persistence, transactions, synchronization and authorized artifact delivery. Completion gates remain in [ROADMAP](../ROADMAP.md). v0.7 adds UI-language switching, dependencies and execution-host acceptance on target platforms. This planning addition changes neither current v0.6 completion criteria nor implementation status.

Implement §5.10 in this order: reuse SOURCE §1.2 bindings/publication data and verify native reference classification → EDITOR §2.5 unified insertion, display text and form/locator/cross-reference controls → FEATURES §10.1.1 source-note groups and citation-location views → standalone Typst dependencies and DOCX field/link/text/loss classification → ROADMAP CR01–16. Engine supplies actual semantics for the same compiled target; Knowledge distinguishes bindings, author references and displayed occurrences. Core reuses shared-Draft saves, authorization, sync and original-operation recovery. Controls create neither a second bibliography database nor a global alias identity registry; indexes do not write reciprocal links, and export adds no unproved Word automatic-update promise. Key reuse/explicit renaming and link-title updates follow FEATURES §§10.1.1/3.2 for scope, shared Draft and undo. New Web deep links require a trusted deployment origin and actual routes before enabling copy. The former hosted fragment-URI parsing gap completed its bounded CR14 fix and real HTTP/background-browser acceptance in v0.6. Later reference interactions reuse and regress that exact scope without turning application-URI resolution into deployment Web-deep-link acceptance. Existing citation defects and promised v0.6 consumer guarantees retain their original gates. New capabilities validate authorization in v0.6.1, with platform interaction in v0.7. MVP records measured capabilities only, reusing existing checks and candidate records.

v0.7 additionally implements REQUIREMENTS §5.9 and FEATURES §7.1 in order: bounded Windows tray/notification/window and MSIX prototype → single-instance residency/capture preservation → minute-level reminder intent independent of due → scheduling/invalidation/recovery summaries → MSIX installation/update and platform acceptance. SOURCE defines the optional reminder field/time semantics before implementation; FEATURES owns lifecycle, deduplication/unknown delivery and views; EDITOR reuses Draft/Undo for independent capture; CORE retains sole writes, original-operation recovery, authorization and sync while managing device state. App entry points own native bridging/single-instance activation; product packaging owns MSIX, signing, Runtime dependencies and upgrades. Validate concrete WebView2/platform mechanisms before selecting bindings, without rewriting the product for a tray. New reminder consumers validate v0.6 authorization/sync contracts in their own slice. Windows results do not establish other platforms, and packaging success does not establish installation or user-data survival after uninstall. DSK01–10 and execution records remain in ROADMAP/WORKFLOW without another acceptance ledger.

§5.7.1 joins one review implementation chain: first distinguish ordinary-comment, reply and pending-revision ranges/identities and cover partial/multiple cell/list cases; then connect bounded OMML import, math-object/symbol mappings and consistent export-loss preview in v0.6.1; finally run both directions of ROADMAP AW01–10. EDITOR §2.4 owns selection/overlapping-discussion UI. FEATURES §9.9, §10.2 and §11.3 own original/new-source/final-Word mappings, revision baseline and degradation classes. Core reuses identity/transaction/authorization/sync without another editable annotation body or an unimplemented persistent format. Add affected parsing/mapping/consumer tests with implementation; new equation scheduling cannot defer existing text-review defects. MVP records actual capability while requirements/design define targets, without relabeling old evidence as new-case passes.

Implement §5.11 after reference enhancements as a dedicated v0.6.1 slice: prove native footnote/figure/equation and the bundled document-end-note module in actual Html/Paged/standalone compilation → freeze endnote call/origin interfaces and update SOURCE → EDITOR §2.6 insertion/editing/return navigation and caption controls → FEATURES §10.1.2 DOCX note parts, bodies and SEQ/reference mappings → both directions of NC01–10. Author note/caption bodies remain source; review JSON holds only discussions. Core reuses Draft, identity, history, sync and authorization, committing confirmed import associations and all source/resource changes in one Plan. Complex modes follow capability/loss profiles and static text cannot impersonate updatable structure. Validate new consumers in this minor version rather than postponing authorization to v0.7. MVP records actual capability and WORKFLOW records execution without changing earlier passes or current v0.6 scope.

Implement §5.12 in v0.6.1 as follows: verify current native styles and DOCX language declarations → define actual personal-preference/shared-processing storage and authorization contracts, updating affected SOURCE/CORE sections → EDITOR §8.7 initialization/settings controls → FEATURES §10.3 profile applicability, mixed-language handling and four proofreading categories → frozen settings/dictionary versions, language interchange and LC01–08. Show source, processing and output languages separately; preserve complex native rules, retain unknown regions and mark incomplete support as partial. Close shared-dictionary, preview, artifact and revocation paths in this version. v0.7 adds personal UI switching, translation fallback and LC10 platform regression. PDF/image OCR and initial public-document/invoice optimization reuse this mechanism in v0.6.2 under §5.13, with LC09/IM01–10; OFD/XPS remain deferred. They add no current v0.6 work, and settings controls cannot substitute for recognition capability. This revision updates requirements/design/plans and MVP status only, without changing persistent schemas or product implementation.

§5.13 follows v0.6.1 as v0.6.2: freeze independent backends/models/licenses, language combinations, initial regional profiles/invoice types/fields and quality fixtures → PDF text layers, page status and provenance → PNG/JPEG and scanned/mixed-page OCR, deduplication and basic structure → public-document numbering/fields and exact invoice candidates → EDITOR §8.8 comparison/review and Core atomic import → IM01–09/LC09. FEATURES §10.4 owns routing, rule applicability and coverage/loss; Engine/import adapters provide exact text, geometry and provenance. SOURCE/CORE freeze provenance evidence, Job results and applicable field mappings before implementation without another editable recognition body. Validate authorization, offline execution, cancellation, partial failures and original-operation recovery at each stage, initially in independent Windows-local/self-hosted environments. v0.7 uses IM10 for platform model installation/upgrades, resource budgets and interaction. OFD/XPS, scanner acquisition, complex tables, handwriting/formula OCR and all invoice types are outside the initial scope. Product-owned rules, dependency inventories and authorized fixtures reuse WORKFLOW/MVP to record actual completion.

Stages may overlap, but later stages cannot bypass identity, transaction or authorization contracts. Freeze public APIs, actual persistence formats, backend capabilities and environments before their implementation slice, without inventing multiple wire generations unnecessarily. Refactoring requires sufficiently concrete core mechanisms, resolved major design gaps and evidence for material choices; release additionally requires real platform acceptance, which limited experiments cannot replace.

Before transforming real materials, make recoverable backups and import old formats from copies with explicit loss and recovery reports. Do not automatically rewrite an original library without confirmation. When no real historical data has been supplied, nonexistent migration objects neither block the new design nor justify a claim that migration is complete.

## 5. Targeted checks for this refinement

- `Inbox/loose.typ` and directories without a primary file remain visible/expandable and Unmanaged. Explicit unmanaged subtrees stop Node discovery, not ordinary browsing. Missing admitted primaries retain identity diagnostics; ignored content never silently becomes Nodes.
- Outside-library `.typ` without ID opens/saves/Save-As with correct relative dependencies and no new control files beside originals. Preserve both drafts on external change; WebUI copy-download is not in-place save.
- CEL swaps/exists/all/filter/conditional guards and host aborts follow FEATURES §5.4; authorization/budget failures cannot be absorbed by true.
- Copy A current r2 plus B pinned to A/Q@r1: retain the exact historical target and preview its dependency; independent copy explicitly rejects instead of changing to r2.
- Compact schema rejects signature overrides, unknown operations and wrong ABIs; resolution preserves all business fields/constraints/value rules.
- Check Agent read/egress/write/external effects/cost separately; a free read-only model request still needs the proper egress scope.

These are design requirements. REVIEW §8 and probe READMEs record actual execution; unrun operations are not passes.
