---
source_language: zh-CN
translation_of: D9-LEXICON.zh-CN.md
translation_status: synced
---

[简体中文](D9-LEXICON.zh-CN.md)

# A2 D9 Terminology and Naming Lexicon

Status: **author-resolved-pending-independent-review**. D3–D8 owners remain unchanged. D9 names conversion-control concepts only.

| Concept | Owner and meaning | Explicit exclusion |
| --- | --- | --- |
| Source Artifact | D9 input control; host-pinned external bytes | not Node, SourceBinding, path/hash identity |
| Import IR | D9 conversion; closed typed intermediate evidence | not third-party AST or author source |
| Source Location / Import Observation | D9 input-version coordinate and extraction evidence | not D3 Locator/write authority/provenance grant |
| Conversion Provider / Route | D9 host registry; reviewed adapter / finite ordered pipeline | not free command, fallback, Model Provider or worker authorization |
| Import Mapping / Mapping Proposal | D9 Core conversion choice / immutable pre-prepare proposal | not Query operator, ActionSpec or generic patch |
| Import Job | **D6-owned**, D9 operates the inherited record | not second ledger or job-wide atomic transaction |
| Coupling Group / Import Batch | indivisible author dependency group / one original atomic request | not UI/Query page or worker process |
| Conversion Input | D9 immutable raw/IR/mapping/loss/route evidence | not identity, OriginBinding or author source |
| Worker Invocation | D9 worker-control record | no paths, commands, permissions or publication authority |
| Node Template / Template Recipe | D2 Template identity / D9 one-shot construction recipe | no persistent instance binding or script interpolation |
| Office Template / Placeholder / Style Directive / Repeat Band | ordinary Office bytes and visible compiler syntax | no content-control/named-range/ExcelTable hidden layer |
| Render Snapshot / D7 Result Pin | D9 finite projection / pin of D7-owned complete result | not author snapshot or rowHandle identity |
| Export Plan | D9 current immutable output preparation, fresh type `ExportPlan/4` | not D6 PreparedIntent or author ledger |
| Annotation Content Carrier | D9 closed export input from one exact current `D8AnnotationReadResponse/1` | not annotation_index, second parser or permission carrier |
| View Render Binding | D9 finite renderer/profile/assets binding over complete D7 result + `ViewSpec/1` | not View authority, Query transform or generic chart-library config |
| Staged Output / Publication Receipt | complete unpublished bytes / external publication fact, fresh `PublicationReceipt/4` | never D3/D6 author receipt |
| Print Receipt | D9 print-only delivery outcome, `D9PrintReceipt/1` | not author receipt or external-publication proof |
| Import Loss / Export Loss | separate D9 fixed-proposal loss domains | not safety approval or cross-domain address |
| Image Physical Size / imageSizes | verifiable source physical fact / user output layout choice | never host DPI/viewport guess |
| Region Body / d9rg1 | D9 non-identity geometry | not complete Locator; outer l1 stays D3 |

## Public technical interfaces

Unique technical owners:
- Probe Interface: `d9_probe`, `d9_probe_result`.
- Conversion Start Interface: `d9_convert`, `d9_conversion_started`.
- Conversion Job Interface: `d9_conversion_state`, `d9_conversion_state_result`, `d9_conversion_cancel`.
- Import Analysis Interface: `d9_import_analyze`, `d9_import_analysis`, `d9_import_choose`.
- Import Preparation Interface: `d9_import_prepare`, `d9_import_prepared`, `d9_import_next`.
- Import Job State Interface: `d9_import_state`, `d9_import_state_result`; operates on D6 ImportJob.
- Node Template Analysis Interface: `d9_template_analyze`.
- Error Interface: `d9_error`.

These labels do not create extra domain concepts or submit paths.

## Current-version names

Final current D9 coordinates D7 `PreparedActionBinding/4`, D3 `D3IdentityOperationRequest/13`, D6 `DependencyProof/3` and Effect3, and fresh D9 `ExportPlan/4` / `PublicationReceipt/4` / `D9PrintReceipt/1`. Plan/Receipt3 remains an exact recovery family. D10's earlier naming companion references to PAB3/Plan2/Receipt2 are provenance from an intermediate owner state and are named-superseded here where final FC applies.

`D7ResultPin` owns no D7 schema/value semantics; nested TerminalSchema/V remain D7. `TemplateRecipe/2` does not own D2 Template identity. `RegionBody/d9rg1` does not own D3 Resource region identity.

## Office native selector terminology

A **visible native selector** is the exact Office-template text `native.table[...]::column[...]`. A **compiled native key** is the internal ASCII `nt_...` / `nc_...` identifier frozen in the applicable Plan3/Plan4 binding record. They are intentionally different: visible bytes are the authoring authority; the compiled key only supports canonical Plan structure. The complete `D9NativeTableSelector/1` preserves the semantic selector.

## Retirement names

Fresh current paths reject unpublished legacy ImportIr/YAML proposal, attr/record aliases, formula-reorder/broad-view template grammar, free provider command/fallback, free binding dictionaries, rowHandle identity, generic author-receipt alias and D9-private Locator kind. Historical research may retain the strings with explicit non-authoritative provenance.

## D10 direct boundary

D10 may catalog a conversion contribution and gate its external capability, but does not own D9 wire, Provider/Route semantics, Worker sandbox, template grammar, Plan, PublicationReceipt or author commit. D9 worker networking remains denied by default. Mobile conversion remains unavailable. D10 complete module is outside D9 acceptance.
