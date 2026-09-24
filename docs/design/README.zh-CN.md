---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# 目标设计与协作入口

## 状态与优先级

状态：accepted-design-not-implemented。本区公开 D1–D9 的完整目标设计输入，最新协调代为 D9-r04-evidence05-2026-09-24；D10 候选尚未接受。当前主分支实现仍是早期原型，产品规范中的旧 Record、信封或语法不能覆盖这些目标设计，也不能反过来据目标文档宣称已实现。

保留 D1–D9 规范正文、已激活配套修订、术语、场景和实施门。原文中的日期、候选/未激活标签和下一阶段执行范围是历史上下文；本索引和固定提交确定输入代。D9 已协调的 D3/D6/D7 修订与 D8 的 D7 修订均在相应完整文件中。仅设计已接受，D9 I01–I12 等真实实现门仍未关闭。

## 阅读与修改

完整清单在 `inputs.json`。按 D1 至 D9 读主文，再读该主题所有规范附件、术语和实施门；强制场景输入是需求与替代方案，不能当成已选方案。搜索摘要、标题或只读主文都不等于读完。大文件按行分段读取至末尾，记录覆盖范围和缺口。

导入快照保留原语言和规范文本，只改写已包含文件的相对链接，并去除未发布控制记录、私人会话地址及本地路径；这些传输变化不构成规范修改。原文中的私人验收链接不是产品协议依赖。新增候选保持中英文同步，按 candidate → accepted-design-not-implemented → implemented 或 superseded 标识状态。

候选必须绑定固定 Git 提交、完整阅读范围、假设和实际证据。更好的完整替代可以提出；涉及上游必须给出精确协调修订，独立通过及一次协调生效前不能默默改快照。评审者成为主作者时，最终验收另取独立评审。

## D10 与发布

下一任务为[Agent、自动化和外部能力](d10/TASK.zh-CN.md)。D10 之后进行 A2 全局设计审查，之后按[路线图](../../ROADMAP.zh-CN.md)推进 G1 最小可用产品。设计公开、设计接受、可运行发布是不同事件。

## 完整输入清单

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
