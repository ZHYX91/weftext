---
source_language: zh-CN
translation_status: source
---

[English](START.md)

# D10 阅读与候选检查点

状态：candidate revision `D10-r08-joint-review-fixes-2026-09-28`；完整 R08 作者修订候选，等待 fresh 完整独立联合评审。固定 R07 已按候选18/18 + S49/49 完整评审并结论 REVISE（P0=0、P1=1、P2=10）；该历史评审不等于 R08 已接受。本文件记录交接状态，不是 Gate verdict。

固定输入基线已整合为 S=`7e18168dad3e6d120fce0dd607dc10fa7894e252`；S 包含旧 U=`f205831c848729f7ddbc3ba0cf32b689459c0c98` 的原 48 份输入和新增 D4 reference catalog。作者分支：docs/d10-start。既有 Draft PR：#2，base=docs/chat-collaboration。D1–D9 快照继续权威；D10 UPSTREAM-AMENDMENTS 只是尚未激活的配套提案。

## 1. 阅读覆盖

阅读证据按 actor/lineage 分账。**原作者 lineage** 历史完成 U48/48，随后又全文读取新增 D4 reference catalog，形成历史 S49/49。**当前接续作者**亲自完成 S16/49 全文；D9 workers/export 与 templates 只做 dependency-scoped 局部读取，不计全文。**固定 R07 的独立联合评审**另行完成 S49/49。任何一方都不能继承另一方的阅读计数。下列代码块继续只是原作者 lineage 的 U 48 文件清单：

```text
docs/design/snapshots/d1-product-surface-and-capability-boundary/source.md
docs/design/snapshots/d1-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d4-d10-a2-mandatory-scenario-inputs-2026-08-28/source.md
docs/design/snapshots/d9-conversion-templates-and-workers/source.md
docs/design/snapshots/d2-document-content-and-domain-objects/source.md
docs/design/snapshots/d3-identity-references-ownership-and-lifecycle/source.md
docs/design/snapshots/d3-terminology-and-naming-lexicon/source.md
docs/design/snapshots/d4-attribute-types-schema-and-relations/source.md
docs/design/snapshots/d4-terminology-and-naming-lexicon/source.md
docs/design/snapshots/d5-tables-and-node-collections/source.md
docs/design/snapshots/d5-terminology-and-naming-lexicon/source.md
docs/design/snapshots/d6-control-interfaces/source.md
docs/design/snapshots/d6-storage-transactions-permissions-and-sync/source.md
docs/design/snapshots/d6-terminology-and-naming-lexicon/source.md
docs/design/snapshots/d6-terminology-registry/source.json
docs/design/snapshots/d7-definition-transfer/source.md
docs/design/snapshots/d7-execution-and-action-interfaces/source.md
docs/design/snapshots/d7-narrow-field-qualification/source.md
docs/design/snapshots/d7-prepared-action-binding/source.md
docs/design/snapshots/d7-preview-and-effects-transport/source.md
docs/design/snapshots/d7-query-algebra/source.md
docs/design/snapshots/d7-query-view-action/source.md
docs/design/snapshots/d7-scenario-dispositions/source.md
docs/design/snapshots/d7-terminology-and-naming-lexicon/source.md
docs/design/snapshots/d7-terminology-registry/source.json
docs/design/snapshots/d7-value-and-cel-profile/source.md
docs/design/snapshots/d7-view-contract/source.md
docs/design/snapshots/d8-acceptance-matrix/source.json
docs/design/snapshots/d8-acceptance-matrix/source.md
docs/design/snapshots/d8-direction-and-accessibility/source.md
docs/design/snapshots/d8-editor-and-cross-surface-interaction/source.md
docs/design/snapshots/d8-editor-interfaces/source.md
docs/design/snapshots/d8-terminology-and-naming-lexicon/source.md
docs/design/snapshots/d9-acceptance-matrix/source.md
docs/design/snapshots/d9-coordinated-d3-d7-binding-amendment/source.md
docs/design/snapshots/d9-import-ir-and-mapping/source.md
docs/design/snapshots/d9-templates/source.md
docs/design/snapshots/d9-terminology-and-naming-lexicon/source.md
docs/design/snapshots/d9-workers-and-export/source.md
docs/design/snapshots/d2-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d3-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d4-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d5-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d6-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d7-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d8-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d9-implementation-impact-and-test-outline/source.md
docs/design/snapshots/d8-rtl-and-bidirectional-interaction-mandatory-review-intake-2026-09-01/source.md
```

原作者 lineage 的补充第49份输入来自 S=`7e18168dad3e6d120fce0dd607dc10fa7894e252`，完整内容为 102856 字符、3685 个内容行（文件尾换行使逐行 API 分割多一个空元素）。该目录包含 61 个 Field、7 个 Facet、22 个 value-type alias、4 个 qualifier set 和 1 个 Calendar series policy；历史 lineage 已核对它与 D7 Narrow Field Qualification 的 `people/phone` 正向构造及 relation/cross-Field 负向边界。该历史读取不能再次计入接续作者全文覆盖。

以下仓库/设计规则与任务入口由原作者 lineage 完整读取；该清单是历史 provenance，不计入接续作者 S 覆盖：

- AGENTS.zh-CN.md
- docs/DOCUMENTATION.zh-CN.md
- docs/design/AGENTS.zh-CN.md
- docs/design/README.zh-CN.md
- docs/design/inputs.json
- docs/design/d10/TASK.zh-CN.md
- scripts/check_docs.py

候选形成前还复读了分支上的 START，核对实际进度，没有把搜索摘要、目录或部分命中当作全文阅读。

## 2. 截断与补读记录

读取过程中曾出现工具返回截断。截断调用一律不计为完整阅读，并以更小单段重新覆盖缺口。明确补读包括：

- D3 identity 主文的中段与尾段；
- D7 Query Algebra；
- D7 多文件聚合读取后逐文件补读；
- D8 Editor Interfaces；
- D8 Acceptance Matrix Markdown 与 JSON；
- Mandatory Scenario Intake 的大段聚合读取改为较小分段连续覆盖到文件末尾。

对**原作者 lineage**，上述 48/48 以及后续 S49/49 在补读后没有已知缺行。当前接续作者按实际证据仍为 S16/49 全文，剩余33份 S 文件绝不能默认为已读。搜索摘要、工具/workflow summary、目录清单和 partial range 都不计全文。

## 3. 从首批检查点到完整候选

原作者 lineage 首批只读了 4/48 个设计输入并创建 START 验证 GitHub 写入；随后该 lineage 完成余下 44 份全文阅读，并基于完整输入比较方案 A/B/C 后收敛为方案 D：Narrow Delegation Broker + Typed Contribution Catalog + Specialized Executors。

第二轮收敛进一步修正了首版方案：

- 不再假定“无需上游修改”；无人值守作者提交需要明确 D6/D7 coordinated amendment。
- standing approval 首版严格收窄到原 D7 单 owner、单 Field、当前完整 Field 恰一 Entry、一个 existing scalar member 的 set_field_member。
- D4 Registry 与 D10 Catalog 分域，但通过 Activation Binding 做完整激活；D4 累计 semantic ledger 不允许回滚指针或删除历史。
- `ToolValueProfile/1`、`ToolType/1`、`ToolValue/1` 是 D10 自有受限 closed algebra，不是 D7 alias；MCP 只作为 Tool Adapter transport。具名 Core field-member adapter 另行消费原 D7 TypedLiteral/ResolvedCodeScope 来构造准确 D7 作者意图；任意 ToolValue 绝不能成为 NodeRef、FieldId、selector 或 author request。
- package trust 固定 SHA-256 内容绑定与应用层 Ed25519 publisher 签名，同时把 PublisherIdentity 与 NamespaceClaim 分开。
- 外部效果、幂等、outcome_unknown、凭据轮换、费用 reservation、取消竞争和 audit failure 都已经有唯一规范结论。

这些是作者候选选择，不是独立 acceptance。


## 4. 当前候选材料

本目录完整候选由**九对同步中英文 Markdown（18 个 exact path）**组成：

- CANDIDATE.zh-CN.md / CANDIDATE.md
- CONTROL-CONTRACT.zh-CN.md / CONTROL-CONTRACT.md
- UPSTREAM-AMENDMENTS.zh-CN.md / UPSTREAM-AMENDMENTS.md
- TERMINOLOGY.zh-CN.md / TERMINOLOGY.md
- SCENARIO-DISPOSITIONS.zh-CN.md / SCENARIO-DISPOSITIONS.md
- IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.zh-CN.md / IMPLEMENTATION-IMPACT-AND-TEST-OUTLINE.md
- REVIEW-DISPOSITIONS.zh-CN.md / REVIEW-DISPOSITIONS.md
- TASK.zh-CN.md / TASK.md
- START.zh-CN.md / START.md

不存在第十份 supplementary record；REVIEW-DISPOSITIONS 本身就是九对之一。

作者包所称固定 S 四个补充路径是 `docs/design/README.md`、`docs/design/README.zh-CN.md`、`docs/design/inputs.json`、`docs/design/snapshots/d4-reference-catalog-registry/source.json`。前三份是 README/index，只有 catalog 是第 49 个规范源；R08 作者工作不修改其中任何文件。

CANDIDATE 是自包含主规范；CONTROL-CONTRACT 唯一拥有 D10 closed management/current/error/rule、author-adapter、external-effect、history 与 stop wire；TERMINOLOGY 关闭命名冲突；SCENARIO-DISPOSITIONS 保留并裁决 mandatory/task/race 场景；Implementation Impact 分层未来证据；UPSTREAM-AMENDMENTS 继续只是未激活配套文本；REVIEW-DISPOSITIONS 记录独立问题及作者落点；TASK 定义流程；START 记录交接。

## 5. 当前设计选择和仍未激活部分

作者推荐方案 D。Core 仍是唯一作者事务权威，D10 Broker 没有第二写路径；D4 Registry 与 D10 Capability Catalog 分域；Agent/Automation/Connector/MCP 都通过当前权限、委托、egress、secret、budget、audit 和各自 owner protocol。

CONTROL-CONTRACT 继续是唯一 wire owner。D10 ToolValue 类型保持 D10 自有受限代数；Core field-member adapter 只通过明确桥接消费原 D7 TypedLiteral/ResolvedCodeScope，绝不把 generic ToolValue 提升为作者 identity 或 author request。

D6/D7 Standing Approval amendment 尚未共同接受。因此即使候选文档完整，产品也不能把 unattended author submit 标记 available；在协调激活前，Automation 只能准备 author proposal，并沿现有 D7/D8 逐次确认。

本候选也不声称 OS sandbox、真实 MCP/model/connector 服务、credential store、费用系统、真实 Core amendment 或五端 UI 已实现。Implementation Impact 中这些证据继续 pending。

## 6. 独立终审状态
