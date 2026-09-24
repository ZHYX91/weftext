---
_weftext:
  id: "21e90c5d-66a2-476b-b852-5a77ac4e7f13"
---

# D5 Implementation Impact and Test Outline

接受证据：D5总控验收（外部控制记录未随本输入发布）；Pro完整原报告（外部控制记录未随本输入发布）；四项P2修正与逐字差异（外部控制记录未随本输入发布）。正文保留完整接受设计及其语义；候选称谓、完成条件或首次命名版本反映其来源，不重启已完成评审。此权威与控制索引须在协调切换journal（外部控制记录未随本输入发布）为committed后一起生效，准备中或已回滚整批不生效。

状态：已冻结的D5架构权威，revision02-editorial-01。D5已冻结revision02-editorial-01：独立Chat GPT-6 Pro对revision01明确accept/pass、P0=0/P1=0，完整阅读/语义覆盖及术语通过；四项非阻断P2已由总控作保持既有语义的编辑修正，未新增评审轮次。 本文不证明产品实现或性能；实际生效以同批committed切换journal为准。

以下是后续必须完整实现的范围，不在本次修改产品代码。

## 1. 架构影响

| 层/阶段 | 必须承接 | 明确排除 |
| --- | --- | --- |
| D2 document model | 原生table/row/cell grammar、revision locator、exact source局部编辑；Saved Query/View只作Node内occurrence | 无row identity、无新Record正文或ViewRef、无自动rectangular normalization |
| D3 identity/lifecycle | NodeRef、owner-local Resource/Annotation、copy/fork/continue、Trash/restore、revision/receipt原规则 | 不新增Record identityMap项；不使occurrenceKey跨revisionresolve |
| D4 typed facts | Registry、FieldId、完整typed value、Facet/关系公共post-state、raw source保留、Entry/schema admission | 表格列不覆盖Field schema；不重新引入people blob或Entry Annotation |
| D6 persistence/transaction/auth | 单一提交资格、source patch、完整read-set/负依赖、CAS、partial-field授权、非相交重规划、批次日志/恢复 | 不建立Record store；不把研究中pure model当物理成功；不能以whole-namespace replacement扩大冲突/写权限 |
| D7 Query/View/Action | 去重Node结果、显式值/occurrence结果、类型化可编辑列、创建策略与成员证明、固定bulk targets、分页/撤权失效 | 删除下面整套Record分支；不把join/group输出默认为可编辑Node |
| D8 UI/CLI/Mobile | 清楚区分删行/删事实/移出/Trash；位置与membership预览；多值摘要展开；保留冲突draft | 不以grid row index写入、不丢notes/历史、不以Mobile低容量改变语义 |
| D9 conversion/import/template | 显式CSV/Excel/JSON mapping、无损/损失声明、fresh import、有限batch、Office三类binding消歧 | 不执行外部公式、不隐式upsert、不要求普通Node保存export-onlyschema |
| D10 provider/connectors | 外部source authority与cache分域、SourceBinding、schema/contribution真实性和availability | connector cache不是受管Record作者库；provider不得创造另一CRUD/引用域 |

## 2. 旧 Record 分支的成套退役清单

下列定位来自尚未冻结的历史`QuerySpec-freeze-candidate-v5.zh-CN.md`。这是未来统一重构的删除/改写清单，不是保留旧候选或现在编辑历史证据的授权。D7须从冻结D2–D5重建其候选，旧文件留作研究。

| 旧位置 | 删除/改写范围 | 必须保留的通用含义 |
| --- | --- | --- |
| §TypeSpec（原148–154行） | RecordCollectionRef/RecordRef及optional/list/参数/default/QueryRef/output包装 | 已授权的NodeRef与普通typed value的准确类型检查 |
| §Scan（原192–219行） | records domain、recordCollection selector、collection必须是Node、stable field UUID入口和record expectSchema | Node selector完整身份去重；授权后集合操作；D4 FieldId/RegistryBinding |
| §Relations（原227–247行） | record schema下的record/node target专属分类与relation catalogs | D4 relation ownership/direction、统一edge/target授权、禁止伪inverse |
| §Field/CEL（原323行附近） | row.record.fields.get及专属schema字段overload | 显式Field type、缺失/unavailable区分、依赖跟踪 |
| §Wire/values（原333、357–367行） | collection UUID、二元/三元record ref、records compound-ref feature、equality/group/distinct/cache/delta/export专支 | 通用object/list结构值并非受管Record，不能做全词删除 |
| §Occurrence/provenance（原367–391行） | persistent scan中Record复合identity种子、traverse target、排序决胜/增量匹配、drill-down seed | value/derived rows无durableidentity；区分新生成来源说明与继承可执行ActionEvidence |
| §Evidence/View（原413–435行） | record schema/collection权限项、read-set、lens/Action targets、Board passthrough与DynamicBlock参数 | 保留通用envelope、授权屏障、准确目标/revision重解析 |
| 旧data-model/view文档、schema/fixtures、locale/API | 与上列相关的decoder、capability、wire、测试、文案一同退役 | 历史文件保持历史；公共实现不得保留alias/双读/fallback |

特别不能继承的旧前提：Task曾被写成Node/checklist union；D2已冻结Task为ordinary Node+Facet。record collection曾被预设为Node且引用有二元/三元两种说法；本D5不保留该domain。D7对Tasks/occurrence和provenance传播规则应按当前上游重建，不能只删除`records`一行后声称已完成统一架构。

## 3. 必要检查及证据范围

1. 原生表格：行/格实际source、转义pipe/backslash、CRLF/CR/LF、重复row、ragged table、blank/comment、empty table、stale locator、合法Inline/ref、不可表示文本；未触碰字节不变，失败零作者写入。
2. 集合：属性与路径谓词的move差异；空集合默认parent；ad-hoc必须显式parent；模板冲突；append ordinal并发；filter true但top/limit排除新Node；定义/参数/schema/权限变更使旧创建计划失效。
3. 字段：两个相同phone值的不同notes、三条历史断言、同值不同keys、外部复制key碰撞、多blocks/unknownnamespace、required最后项、inverse真实owner权限；D4selectors/readset保持。
4. 并发：A改一个name note、B新增phone、C仅有phone权限；禁止整个namespace覆盖；旧revision计划不直接重放，重新绑定后不相关facts保留，不能证明时保留冲突提案；删后同key再加的ABA不得误命中。
5. Lifecycle：同一Node出现在两个集合、hidden subtree、root trash拒绝、restore原identity、copy fresh、Workspace transfer target成功source失败；只显示实际receipt结论。
6. 规模：999/1000/1001 target，199/200/201 grid页，宽row触发更窄bytes/dependencies预算；模板descendants计入import数量；不会把loaded数量当total，不产生部分成功结果。
7. 批次：10,000 Nodes分10批、workspace源大于workingmemory、第6批提交前/后故障、进程重启/断网/重复续作；每batch原子、job准确报告partial状态，不重复创建。
8. ICS：series rule/override、无限recurrence有界查询、外部subscribe/sync与adopt/import分开、跨源同UID、取消/删除/解绑/重导入/离线/时区、无Record/隐式Node物化；VFREEBUSY/VTIMEZONE无Node。
9. Query/Office：空结果仍有schema；nonterminal零行页不当EOF；join/aggregate列只读；普通Document table/Node collection/value table绑定分域；普通Node零export-only配置；旧record token拒绝。
10. 术语：受控名字唯一归属、全部相邻概念排除、退役API成套移除、合法用户/历史文字不被词表误杀；所有五表面同一语义。

本轮是架构文档审查。上述项目当前均是可复核的合同案例和未来test outline，尚无实际D5 product host通过结果。机械preflight只检查材料完整性、源副本、链接和名称清单；不编造完整parser/事务/权限执行器以获得测试数。完整Pro语义审查也不等于D6–D10或A2实现验收。
