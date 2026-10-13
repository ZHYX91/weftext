---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# 目标设计与协作入口

当前已接受的目标设计见 [Typst 新架构](typst-architecture/README.zh-CN.md)：按 [需求与技术方案的边界](typst-native/REQUIREMENTS.zh-CN.md) 从新项目重新选择技术，恢复全类型必要结构化能力，并保留五档视图与中文默认支持。状态为 **accepted-design-not-implemented**；2026-10-13 独立全局终审、固定技术基线与证据边界见 [REVIEW](typst-architecture/REVIEW.zh-CN.md) §7。前轮 [Typst 方案](typst-native/README.zh-CN.md) 和下列 AsciiDoc 输入/评审保持原历史范围，不强制新方案继承其技术；产品重构尚未开始。

## 当前方案与历史输入

当前已接受的技术设计为上述 Typst 新架构，需求权威为 REQUIREMENTS；其设计接受状态只由新架构 REVIEW 说明。主分支仍是早期原型，不能据目标文档宣称产品已实现。

下列 D1–D9 输入在其原固定范围的状态为 accepted-design-not-implemented，原协调代为 D9-r04-evidence05-2026-09-24。该历史状态不使旧 Record、信封、存储格式、协议或实施顺序成为 Typst 重写的技术约束。历史已选业务用于核对需求，曾讨论的替代方案不自动成为当前功能；取舍以 REQUIREMENTS 与当前 owner 的具名规则为准。

保留 D1–D9 规范正文、已激活配套修订、术语、场景和实施门。原文中的日期、候选/未激活标签和下一阶段执行范围是历史上下文；本索引和固定提交确定输入代。D9 已协调的 D3/D6/D7 修订与 D8 的 D7 修订均在相应完整文件中。仅设计已接受，D9 I01–I12 等真实实现门仍未关闭。

## 阅读与修改

历史完整清单在 `inputs.json`。重写与评审先读当前 REQUIREMENTS 和相关新架构 owner，再按实际需求来源回读对应历史主文及必要附件；不要求每次修订重审全部已退休协议。声称完整审过某个历史主题时，须包含其规范附件、术语与实施门，并记录覆盖范围和缺口。强制场景输入同时包含需求与替代方案，不能全当已选方案；搜索摘要、标题或只读主文不等于全文阅读。

导入快照保留原语言和规范文本，只改写已包含文件的相对链接，并去除未发布控制记录、私人会话地址及本地路径；这些传输变化不构成规范修改。原文中的私人验收链接不是产品协议依赖。新增候选保持中英文同步，按 candidate → accepted-design-not-implemented → implemented 或 superseded 标识状态。

候选必须绑定固定 Git 提交、完整阅读范围、假设和实际证据。更好的完整替代可以提出；涉及上游必须给出精确协调修订，独立通过及一次协调生效前不能默默改快照。评审者成为主作者时，最终验收另取独立评审。

补充输入：首次公开清单遗漏了 D4 主文引用的规范性机器目录。现补入逐字保留的 [D4 参考目录](snapshots/d4-reference-catalog-registry/source.json)，清单增至 49 份。该目录包含 4 份限定信息集、22 个类型别名、61 个字段定义、7 个特征模式和 1 个日历系列策略，与已接受的上游输入副本一致。原有 48 份快照不变；基于旧固定提交的阅读记录仍只覆盖原清单，必须另行补读本目录。这是补齐输入，不是修改 D4 规范或接受 D10。

## 历史顺序与当前重构准入

[D10：Agent、自动化和外部能力](d10/TASK.zh-CN.md)及原[路线图](../../ROADMAP.zh-CN.md)保留原阶段记录，不是当前待办指令。当前顺序由 [Typst INTEGRATION](typst-architecture/INTEGRATION.zh-CN.md) 定义：协调修订、独立复核、全局终审、固定重构基线，再按功能切片实现。设计公开、设计接受、可运行发布是不同事件。

## 完整输入清单

| Topic | Input | Role |
|---|---|---|
| D4 | [D4 Reference Catalog Registry v1](snapshots/d4-reference-catalog-registry/source.json) | accepted-design-input |
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

## AsciiDoc / Annotation 最终设计候选

当前作者候选位于 [asciidoc-annotation-final-design/README.zh-CN.md](asciidoc-annotation-final-design/README.zh-CN.md)，固定父提交 `e8aa0b341630a57c786c0891d4bbd1620247441d`。它以 [replacements.json](asciidoc-annotation-final-design/replacements.json) 对 fixed-parent actual-owner sections 做显式替换，S49 snapshots 与本目录 `inputs.json` 保持不变。

该候选包含完整 AsciiDoc/必要扩展、独立 portable JSON Annotation、managed document format、SourceTransform/trust 及 D2–D10 actual-owner coordination；行为/算法见双语 SPEC，closed current schema 与历史分派见双语 SCHEMAS。双语 ACCEPTANCE 现含 438 条 core design oracle 与 322 条 coordination fixture，共 760 条，均未运行。状态仍是 design candidate，等待 exact-head 非作者独立复核；不是实现、A2/global 或发布接受。

## A2 最终设计整合候选

fixed PR4 design head 之上的 A2 candidate 包含自包含 D1–D6 作者候选、source/disposition tracking 与[复核入口](a2-final-design/REVIEW-ENTRY.zh-CN.md)。fixed-a62 已独立 CLOSED 有界 D3 P1；fixed4282 仅独立 CLOSED D4/D5 P2-02 的“六份 D10 真实来源资格”有界范围。fixed32cf 随后独立 CLOSED P2-01 R2，并通过 188/202 条 R1 映射；14 条 R1 残余已在当前作者候选修订，仍待新的 exact-stop 复核。D6 中 fixed829 只保留为 finding-origin provenance，后续独立对象为 fixed01cc：P1-01 与 P2-03 已独立 CLOSED，P1-02/P2-01/P2-02 加导航 P2 在 D6 残余修订后仍待 exact-stop 复核。D6/global A2 未 accepted。D7–D10 完整模块、Mandatory 925–1141 与 fresh global review 仍 pending。该候选未独立接受、未实现、未合并、未发布。
