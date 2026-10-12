---
source_language: zh-CN
translation_of: INTEGRATION.zh-CN.md
translation_status: synced
---

[简体中文](INTEGRATION.zh-CN.md)

# Contract integration and refactoring order

Status: author candidate; see [REVIEW](REVIEW.md) for validation. This file connects requirements to the current solution without creating another transaction or field authority.

## 1. One solution

[REQUIREMENTS](../typst-native/REQUIREMENTS.md) defines outcomes and boundaries. SOURCE solely defines author roles and Type/Value/schema; CORE defines EntityRef, RevisionId, ReadBasis, SourceSnapshot, Plan, Receipt, CommitManifest, Environment and Job; EDITOR defines Draft, official evaluation origins and interaction; FEATURES defines Query, business actions, links, annotations and output consumers. Repeated explanations cannot create another type system or authorization entry point.

A durable revision establishes historical content; a current ReadBasis qualifies this read/write. They are not interchangeable. Read projections and result rows are not write capabilities. Actions prepare anew, and all internal persistent mutations use one Core transaction; external network effects retain separate Job responsibility. Source and indexes also differ: indexes are rebuildable, while authored content, structure, annotations and actual operation responsibility cannot be recovered from an index alone.

This solution replaces the prior SOURCE/CORE/EDITOR/FEATURES/INTEGRATION technical constraints as a whole. Prior `replacements.json`, lexicons and acceptance records describe only their fixed candidate, not the new runtime. Future upgrades of actually released new-system data still require implementation. Preserve real old materials, identify formats and explicitly import/report loss rather than expanding normal decoders by imagining that old proposals were deployed.

## 2. Requirement owners and vertical checks

These are design and implementation paths, **not executed product tests**. Original requirements remain authoritative; grouping does not remove their positive or negative scenarios.

| Requirements | Current owners | Required connected path |
| --- | --- | --- |
| RQ01–03 | SOURCE, EDITOR, CORE | Preserve full valid source → ordinary flowing reading → origin-qualified rich editing → one Draft/save; errors or ineligible controls do not discard source |
| RQ04–07 | EDITOR, FEATURES | Title/subtitle/section roles → same-level standalone/run-in numbering → outline/references → editable Word and accurate loss |
| RQ08–10, §5.2 | SOURCE, FEATURES | Historically selected capabilities → native values/schema → forms/same-item Query → ordered, exact and explainable export |
| RQ11 | SOURCE, EDITOR, CORE | Capture/properties → current draft → actual creation/update → Receipt; unknown does not create again |
| RQ12–14 | FEATURES, SOURCE, CORE | Links/path resolution/potential mentions → currently authorized selection → same-target conversion and one undo; graph indexes grant no write access |
| RQ15–17 | FEATURES, SOURCE, CORE | Search/navigation/Query → views with explicit completeness → actions prepared against real targets; task body/properties agree |
| RQ18 | FEATURES, EDITOR, CORE | Exact original target → independent annotation/discussion → fresh read/invalid state → suggestion applied in one transaction without hidden-target reads |
| RQ19–20 | FEATURES, CORE, EDITOR | Template/import/copy → frozen inputs/map → correct output/loss → confirmation and intranet delivery of the same artifact |
| RQ21–24 | CORE, all consumers | Actual local/sync/Server guarantees → current permission/revision → save/conflict/recovery/retry; local single-user use does not replace hosted multi-user needs |
| RQ25, NAV01–05 | EDITOR, FEATURES, CORE | Navigation/list/document state → personal presentation restoration → current permission/result revalidation; UI preferences do not edit source |

Structured-capability checks cover the actual historical D4 types and selected behavior, not just three person/task examples. Richer bibliographic, calendar and task structures cannot be deleted because a table control is missing; real deferrals remain explicit. Common and optional advanced fields have layered UI rather than making every field mandatory.

Library-style requirement §5.3 is owned by EDITOR §§8.1–8.5, CORE §9 and FEATURES §11: select an actual style resource, generate a transparent entry, preserve original note identity/source, follow native overrides and actual dependency invalidation, and distinguish an exact single file from an independent styled project. FEATURES §§9.1–9.6 define original annotation ranges, visible excerpts, paragraph labels and current/historical-reference boundaries.

## 3. Original 21 and later findings

The original prefix is `A2-GLOBAL-6B13-`; old CLOSED states do not transfer. Every check below still needs independent review and implementation verification. When a removed mechanism makes its exact counterexample inapplicable, retain the general harm check without implementing nonexistent protocol branches.

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

1. **Baseline and risks.** Verify requirements/types and form this coherent candidate; validate architecture-sensitive file backends, native data, editing origins and output choices. Independent Sol review precedes a fresh Pro global review. Collect and resolve actual findings before freezing the refactoring baseline.
2. **Small vertical loop.** Open workspace → ordinary Typst note → local Draft/source save/recovery → stable identity/links → ordinary flow reading/basic rich editing. Test external-editor races and faults early rather than after all features are implemented.
3. **Structure and organization.** Central schemas, property forms, richer types, Query/navigation/lists/task views; validate same-item collection queries, unknown/exact values and safe local edits together.
4. **Cross-object capabilities.** Independent annotations, templates/copy/import, headings and editable Word/spreadsheet output, intranet downloads, preservation and recoverable Jobs.
5. **Surfaces and deployment.** Complete hosted multi-user permissions and synchronization through the shared Core, then validate client/offline/revocation/fault combinations. Platforms may ship in stages with unsupported backends explicitly identified; a research probe is not a supported product.

Stages may overlap, but later stages cannot bypass identity, transaction or authorization contracts. Freeze public APIs, actual persistence formats, backend capabilities and environments before their implementation slice, without inventing multiple wire generations unnecessarily. Refactoring requires sufficiently concrete core mechanisms, resolved major design gaps and evidence for material choices; release additionally requires real platform acceptance, which limited experiments cannot replace.

Before transforming real materials, make recoverable backups and import old formats from copies with explicit loss and recovery reports. Do not automatically rewrite an original library without confirmation. When no real historical data has been supplied, nonexistent migration objects neither block the new design nor justify a claim that migration is complete.
