---
source_language: zh-CN
translation_of: README.zh-CN.md
translation_status: synced
---

[简体中文](README.zh-CN.md)

# Target design and collaboration entrypoint

## Status and precedence

Status: accepted-design-not-implemented. This area publishes complete D1–D9 target-design inputs through coordinated generation D9-r04-evidence05-2026-09-24; a D10 candidate has not been accepted. Main still contains an early prototype. Old Record, envelope or syntax descriptions in product specifications do not override these target designs, and target documents do not establish implementation.

The inputs preserve normative bodies, activated coordinated amendments, terminology, scenarios and implementation gates. Dates, candidate/inactive labels and next-stage execution scopes inside source text are historical context; this index and the fixed commit identify the input generation. D9 amendments to D3/D6/D7 and the D8 amendment to D7 are included in their complete files. Acceptance covers design only; actual implementation gates such as D9 I01–I12 remain open.

## Reading and editing

The complete inventory is `inputs.json`. Read main documents from D1 to D9, then every normative supplement, lexicon and implementation obligation for each topic. Mandatory scenario intake contains requirements and alternatives, not predetermined decisions. Search excerpts, titles or main documents alone do not establish complete reading. Read large files in line ranges through the end and record coverage and gaps.

Imported snapshots retain original language and normative prose. Only relative links to included files are remapped, and unpublished control records, private conversation addresses and local paths are removed. These transport changes are not normative amendments. Private acceptance links in the originals are not product-protocol dependencies. New candidates retain synchronized Chinese/English and use candidate → accepted-design-not-implemented → implemented or superseded status.

Bind candidates to a fixed Git commit, complete reading scope, assumptions and actual evidence. Better complete alternatives are welcome. Changes to upstream contracts require exact coordinated proposals and independent acceptance followed by one coordinated activation before snapshots can change. When a reviewer becomes the main author, obtain separate independent final acceptance.

## D10 and release

The next task is [Agent, automation and external capabilities](d10/TASK.md). D10 is followed by A2 global design review, then the G1 minimum usable product under the [roadmap](../../ROADMAP.md). Publishing design, accepting design and releasing runnable software are distinct events.

## Complete input inventory

| Topic | Input | Role |
|---|---|---|
| D1 | [D1 Product Surface and Capability Boundary](snapshots/d1-product-surface-and-capability-boundary/source.md) | accepted-design-input |
| D2 | [D2 Document Content and Domain Objects](snapshots/d2-document-content-and-domain-objects/source.md) | accepted-design-input |
| D3 | [D3 Identity References Ownership and Lifecycle](snapshots/d3-identity-references-ownership-and-lifecycle/source.md) | accepted-design-input |
| D3 | [D3 Terminology and Naming Lexicon](snapshots/d3-terminology-and-naming-lexicon/source.md) | accepted-design-input |
| D4 | [D4 Attribute Types Schema and Relations](snapshots/d4-attribute-types-schema-and-relations/source.md) | accepted-design-input |
| D4 | [D4 Terminology and Naming Lexicon](snapshots/d4-terminology-and-naming-lexicon/source.md) | accepted-design-input |
| D5 | [D5 Tables and Node Collections](snapshots/d5-tables-and-node-collections/source.md) | accepted-design-input |
| D5 | [D5 Terminology and Naming Lexicon](snapshots/d5-terminology-and-naming-lexicon/source.md) | accepted-design-input |
| D6 | [D6 Control Interfaces](snapshots/d6-control-interfaces/source.md) | accepted-design-input |
| D6 | [D6 Storage Transactions Permissions and Sync](snapshots/d6-storage-transactions-permissions-and-sync/source.md) | accepted-design-input |
| D6 | [D6 Terminology and Naming Lexicon](snapshots/d6-terminology-and-naming-lexicon/source.md) | accepted-design-input |
| D6 | [D6 Terminology Registry](snapshots/d6-terminology-registry/source.json) | accepted-design-input |
| D7 | [D7 Definition Transfer](snapshots/d7-definition-transfer/source.md) | accepted-design-input |
| D7 | [D7 Execution and Action Interfaces](snapshots/d7-execution-and-action-interfaces/source.md) | accepted-design-input |
| D7 | [D7 Narrow Field Qualification](snapshots/d7-narrow-field-qualification/source.md) | accepted-design-input |
| D7 | [D7 Prepared Action Binding](snapshots/d7-prepared-action-binding/source.md) | accepted-design-input |
| D7 | [D7 Preview and Effects Transport](snapshots/d7-preview-and-effects-transport/source.md) | accepted-design-input |
| D7 | [D7 Query Algebra](snapshots/d7-query-algebra/source.md) | accepted-design-input |
| D7 | [D7 Query View Action](snapshots/d7-query-view-action/source.md) | accepted-design-input |
| D7 | [D7 Scenario Dispositions](snapshots/d7-scenario-dispositions/source.md) | accepted-design-input |
| D7 | [D7 Terminology and Naming Lexicon](snapshots/d7-terminology-and-naming-lexicon/source.md) | accepted-design-input |
| D7 | [D7 Terminology Registry](snapshots/d7-terminology-registry/source.json) | accepted-design-input |
| D7 | [D7 Value and CEL Profile](snapshots/d7-value-and-cel-profile/source.md) | accepted-design-input |
| D7 | [D7 View Contract](snapshots/d7-view-contract/source.md) | accepted-design-input |
| D8 | [D8 Acceptance Matrix](snapshots/d8-acceptance-matrix/source.json) | accepted-design-input |
| D8 | [D8 Acceptance Matrix](snapshots/d8-acceptance-matrix/source.md) | accepted-design-input |
| D8 | [D8 Direction and Accessibility](snapshots/d8-direction-and-accessibility/source.md) | accepted-design-input |
| D8 | [D8 Editor and Cross-Surface Interaction](snapshots/d8-editor-and-cross-surface-interaction/source.md) | accepted-design-input |
| D8 | [D8 Editor Interfaces](snapshots/d8-editor-interfaces/source.md) | accepted-design-input |
| D8 | [D8 Terminology and Naming Lexicon](snapshots/d8-terminology-and-naming-lexicon/source.md) | accepted-design-input |
| D9 | [D9 Acceptance Matrix](snapshots/d9-acceptance-matrix/source.md) | accepted-design-input |
| D9 | [D9 Conversion Templates and Workers](snapshots/d9-conversion-templates-and-workers/source.md) | accepted-design-input |
| D9 | [D9 Coordinated D3 D7 Binding Amendment](snapshots/d9-coordinated-d3-d7-binding-amendment/source.md) | accepted-design-input |
| D9 | [D9 Import IR and Mapping](snapshots/d9-import-ir-and-mapping/source.md) | accepted-design-input |
| D9 | [D9 Templates](snapshots/d9-templates/source.md) | accepted-design-input |
| D9 | [D9 Terminology and Naming Lexicon](snapshots/d9-terminology-and-naming-lexicon/source.md) | accepted-design-input |
| D9 | [D9 Workers and Export](snapshots/d9-workers-and-export/source.md) | accepted-design-input |
| D1 | [D1 Implementation Impact and Test Outline](snapshots/d1-implementation-impact-and-test-outline/source.md) | implementation-obligations |
| D2 | [D2 Implementation Impact and Test Outline](snapshots/d2-implementation-impact-and-test-outline/source.md) | implementation-obligations |
| D3 | [D3 Implementation Impact and Test Outline](snapshots/d3-implementation-impact-and-test-outline/source.md) | implementation-obligations |
| D4 | [D4 Implementation Impact and Test Outline](snapshots/d4-implementation-impact-and-test-outline/source.md) | implementation-obligations |
| D5 | [D5 Implementation Impact and Test Outline](snapshots/d5-implementation-impact-and-test-outline/source.md) | implementation-obligations |
| D6 | [D6 Implementation Impact and Test Outline](snapshots/d6-implementation-impact-and-test-outline/source.md) | implementation-obligations |
| D7 | [D7 Implementation Impact and Test Outline](snapshots/d7-implementation-impact-and-test-outline/source.md) | implementation-obligations |
| D8 | [D8 Implementation Impact and Test Outline](snapshots/d8-implementation-impact-and-test-outline/source.md) | implementation-obligations |
| D9 | [D9 Implementation Impact and Test Outline](snapshots/d9-implementation-impact-and-test-outline/source.md) | implementation-obligations |
| D4-D10-A2 | [D4-D10-A2 Mandatory Scenario Inputs 2026-08-28](snapshots/d4-d10-a2-mandatory-scenario-inputs-2026-08-28/source.md) | scenario-intake |
| D8 | [D8 RTL and Bidirectional Interaction Mandatory Review Intake 2026-09-01](snapshots/d8-rtl-and-bidirectional-interaction-mandatory-review-intake-2026-09-01/source.md) | scenario-intake |
