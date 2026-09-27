---
_weftext:
  id: "ea013942-5018-4412-b819-ac6b34d31e08"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

# D7 Implementation Impact 与 Test Outline — revision06-draft

本文件是私有架构实施义务；没有修改产品或公开文档。所有实际路径相对于 `D:/Projects/Weftext-Workspace/repos/weftext`。以下2026-09-20只读盘点不把原型当设计前提。后续实施必须刷新line位置与实际消费者。

## 1. 真实原型与替换目标

| 当前证据位置 | 真实观察 | 完整替换义务 |
|---|---|---|
| crates/weftext-core/src/query.rs:17 | QUERY_PROFILE_ID weftext.query.v1、默认limit100、最大1000、QuerySource Nodes/Tasks/Headings/Templates、QueryView混在同文件 | 唯一严格Query DAG、显式take，无默认语义截断；Task/Template走Node分类；View独立typechecker |
| 同文件 QueryValueType / QueryField / QueryLexicalContext | Uuid、Null、Record、TaskKind/TaskState、owner/path等特设公开字段，query与document/heading context混合 | 完整Ref、Optional、D4结构型Registry，受控LexicalBinding，无裸UUID或dynamic-property回退 |
| crates/weftext-core/src/query_workspace.rs:38 | QueryAccessScope保存NodeId集合+Complete/Filtered；使用filesystem扫描和旧Task projection | 由D6真实PrincipalContext/cut/source/Field gates与完整正负依赖代替；client不能自报allowedset |
| 同文件 QueryEvaluationContext:89 / validate_evaluation_context:148 | today/now/timezone/locale和Jiff宿主zone加载 | 推导context精确匹配、固定规则version，不从设备库猜版本/now，D4 exact time全域 |
| 同文件 QueryRowIdentity:178 / QueryCellValue:199 / QueryResult:250 | identity与cells/groups旧封装；CSV直接遍历该result | 六类身份分域、TerminalSchema/D6完整分页、D7delta/export current gate；CSV不得吞none/精确数值 |
| 同文件 compare_prepared_rows:2261 / collect_prepared_groups:2386 | 原查询本地排序/分组路径 | 精确batch数学、规范occurrence tie/错误与orderedness；不同partition重算一致 |
| crates/weftext-core/src/expression_language.rs；tests/expression_values.rs | 原CEL profile实现和表达式fixture | 固定D7 overload/Optional/listmacro/Unicode/Counter边界；不能沿用旧green计新语义已实现 |
| crates/weftext-core/src/task_node_action.rs、task_action_transaction.rs、task_promotion_transaction.rs、task_dependency_transaction.rs、checklist_action.rs | 特设Task状态、promotion及transaction入口 | D4 tasks/task+fields、D3真实fresh compound、D6既有源adapter、D7明确ActionSpec；所有调用面一起替换 |
| crates/weftext-server/src/lib.rs:4978 execute_query，apps/desktop/src-tauri/src/lib.rs，crates/weftext-cli/src/main.rs | Server/desktop/CLI实际调用面 | 同一Core request/result/errors，保留JS exact Counter/decimal wire；host只认证/运输 |
| prototypes/webui/app/query-surface.tsx:16,101,141,242 | 本地QueryResult类型、null或缺value显示空、index参与row key、独立groups显示 | typed schema解码、opaque rowHandle、完整加载/错误状态、View纯绑定，不把数组index作动作identity |
| crates/weftext-server/webui/app.js、apps/desktop、Mobile后续host | 现有UI与未交付host混合 | D1 capability逐surface证明；不得从Web原型推断Mobile支持 |

## 2. 删除与同步切片

实施顺序：S1 TypeSpec/CEL/strict wire与完整Registry桥；S2 D6 cut/permission/dependency Query数据入口；S3 DAG/bag/order/scalar/QueryRef与batch；S4 Result/delta/evidence/action adapters；S5 View validation/renderer/DynamicBlock；S6 六caller与exports；S7旧语法/枚举/fixture/帮助/文档清理。每切片对其发布合同同一次同步Core、caller、fixture、schema、CLIhelp、中文/英文公开规范，不能长期双读双写。

必须替换的测试入口：crates/weftext-core/tests/query_syntax.rs、query_execution.rs、query_templates.rs、query_scale_acceptance.rs、expression_values.rs、document_actions.rs、task_node_action.rs、task_action_transactions.rs、task_promotion_transactions.rs、task_dependency_transactions.rs、checklist_action.rs；tests/fixtures/query-v1、task-node-v1、checklist-v1中与新合同冲突的旧goldens。不删除与新D2合法checklist无关的保护上下文测试。

公开规范目标：docs/specifications/17-tasks-and-query{.md,.zh-CN.md}、18-canonical-query-and-expression、21-board-views、12-document-actions、13-workspace-transactions；docs/architecture/07-collections-query-and-views、19-expression-query-and-template-library、21-board-views；docs/guides/03-tasks-query-and-templates两语言。当前仅列目标，等后续已授权实现及真实验收完成后才更新。

退役受控 identifiers包括旧QuerySource Tasks/Templates、QueryValueType Null/Uuid/Record与TaskKind、固定pipeline source/where/group/view结构、actionQuery/writeQuery、ViewRef/TaskRef/RecordRef、nullable-as-null、title/path/rowIndex动作寻址、provider_missing→empty fallback。没有serde alias、旧parser fallback、隐式迁移或默认limit兼容。历史证据/拒绝方案/用户正文中的旧词不受字符串禁词误报；lint仅扫描实际公开schema、exports、type符号、CLI/locales与新fixture正例。

## 3. 必须实施的 conformance

1. raw bytes strict JSON重复key、unknownenum/member/null、Counter bool/-0/exponent/2^63边界；嵌D3/D4原decoder失败不得被D7重包装接受。
2. D4全Catalog展开到D7类型：CamelCase member/union/Optional、arbitraryinteger/decimal、所有calendar/version/precision/quantity scopes、owner-local refs和author provenance。新增equipment/course定义原grammar编译，无plugin专用分支。
3. 原始source通过D2完整parse、D4Entry真实validation后read；invalid尾部、多个carrier、隐藏Field与未获全source资格两态、unknownnamespace、unsupportedrules都失败闭合，不用预建summary绕过。
4. DAG alpha/topology归一、joint cycles、unreachable、scalarempty、bagduplicates、projectlineage、QueryRef duplicate稳定sort/take、left/inner/relatednone与错误totality；exactmath随机partition和反序验证。
5. 所有Result完成/分页/空nonterminal/scalarNone/graph两段、token交叉audience/tag、撤权/expiry/epoch/gap、cache负依赖与source/lifecycle ABA；source变化不在旧页混最新内容。
6. D5全部Action实际sourcebefore/after、Field多occurrence、原生表格inert编码与trivia、samecut完整postquery top1、1000/1001、identity compound、D3 wire11 request/Result9/receipt与D6独立owner。真实permission/计划恢复/fence/commitCAS与replay不能由本地模型替代。
7. 每View全输入型/唯一键/order/domain/空态/稀疏/负值、宽表long-form、panel/series稳定、graphisolated/parallel/self/cycle、layeredrank子图、超限不截断；晚错误不发布任何chart/export。
8. 六caller精确同结果与错误；真实CLI JSON、Server API、Desktop/WebUI/Mobile transport，RTL/IME/a11y/memory/render/export验收由D8/D9配套，不给不存在的host勾pass。

## 4. 证据范围、风险与后续边界

本次有界Python模型只验证D7指定子代数的数学反例、状态机和输入判别，不执行产品、不证明完整D2/D3产品decoder；具名模型直接调用现有D2 carrier与D4 Catalog/Entry/Facet/Duration校验，范围由报告逐项限定，不复制旧corpus数量为新版本通过数；模型依赖只用标准库和随附fixture，能在独立目录运行。Policy/2 metadata、容量、窄Field资格及prepared/EffectBytes持久性是新明确实施义务；planned永久占用成本仍保留。

完整结果、保守source观察资格、remove-all/upsert-all delta、graph全量buffer和多hop关系有可见规模/延迟成本。固定list/AST/row预算可使很大的扩展搜索/colleague查询明确失败；不宣传任意规模。D7并未授予D8编辑器、D9worker、D10provider运行或产品实现的启动授权。完成架构交接后停止于D8之前，并携带原D8 RTL mandatory intake。

## 5. revision06联合替换与实际证据

D3完整replacement：wire11顶层preparationBinding及原fingerprint、十数组definitionTransfers、Result9 Q、stage14 operation_precondition_failed、legacy saved replay和同decision D7 effects。D4同步该wire/Result消费者并明确可选note/provenance的present约束，无新Field类型/Facet/derived duration算法。D6完整replacement：Policy/2两个显式metadata能力、profile/2固定bootstrap、共享commitSequence、Immutable PreparedActionBinding、complete preview/committed effects运输。全部完整文件位于upstream-replacements；原authority未写入。

实际运行报告：d7-inherited-model-report.json（29项继承有界检查）；d7-revision05-semantic-model-report.json（12项数学/语义检查，含真实calendar/event→range-note目录的declared/effective区分）；d7-d4-bridge-witness-report.json（59个原Entry、18构造、全部作者provenance与7个Family Entry）；d7-narrow-phone-model-report.json（22项具体Field source/权限/提交模型）；d7-binding-transfer-model-report.json（14项binding/typed transfer/Q framing/运输检查，包含typed slot comparator反例）；Facet/checklist及跨阶段术语检查各有具名报告。

新增d7-durable-recovery-model-report.json包含13项真实SQLite有界事务检查：关闭重开、saved plan与完整依赖范围再证明、相关变更/负范围插入、无关变更、撤权及连续性暂停、SQL fence比较、两connection争用、source/ledger同事务回滚、lost receipt精确重放，以及208行完整post-query与仅前200条preview得到相反结果的反例。去掉最终范围复核的mutant会错误提交，不能把该反例算作安全通过。该模型没有完整D3 wire/stages、通用CEL、OS崩溃/锁或完整Policy代数，不宣称产品事务已实现。以上模型及时间链、构造类型、搜索模板、恢复位置、完整结构效果、交付epoch、作者引用、条件源/Trash/家谱模型共16个入口；全部源码及实际import/数据依赖随包并在独立目录复跑。通过数量本身不建立架构接受。


## 6. 周期投影与完整消费链义务

Algebra§11在D7增加共同DerivedPeriodRange read语义，不改D4作者类型/period身份或Registry schema。实现须为任意结构匹配Field执行完整CalendarPeriod语义验证，保留每Entry及全部依赖；CalendarDate超域给明确状态，不能靠宿主日期exception。与period相邻的date/instant range、Event、recurrence、DerivedDuration也须从合法源走到实际消费者；类型构造覆盖或单独算术pass不足以证明可表达。

新增d7_temporal_chain_models.py及d7-temporal-chain-model-report.json提供逐阶段有界证据，完整Query/View/TypeSpec和原始source放在Temporal Query View Witness。报告明确区分实际D2外层carrier/D4 Entry gate、有限CEL/Query/View子集、原D4 recurrence/duration调用与未执行的完整产品/parser/授权/renderer。故意去掉period adapter或把point改为普通object必须被consumer类型门抓住；超域/缺provider/撤权等不能由正常样本数量遮盖。所有报告与16个模型实际依赖须独立目录复跑，独立架构接受仍另外取得。


## 7. 构造、原值、效果和恢复的有界证据

当前合同区分作者保存的Ref原值与当前目标状态/解析，全部read/Field/search/action/effects接收端遵循相同界限；原值可披露不授予target resolve能力。Search的Optional NFC fallback保持同型；双向构造类型给literal紧上界，外部expected type不隐式扩大非literal。现存C结果的conditional_source_change完整保留符号源与两种版本分支；committed仅列实际变化。每次committed open建立完整独立交付epoch，原decision语义与pins不变。D3明确独立删除成员的restore membership与owner purge closure；D7闭合restoreLocation和效果排序；家谱文本亲属来自同一graph.nodes的显式nodeDetails，保持不可展开。

新增报告分别为：d7-constructor-typing-model-report.json（39个有限AST类型案例，不是完整CEL parser）；d7-search-template-model-report.json（12种模板、54个实际D4值和4个反例）；d7-restore-location-model-report.json（24个映射/拒绝案例）；d7-effect-completeness-model-report.json（11个集合完整性案例，301个实际 sibling ordinal变化分页200+101）；d7-delivery-epoch-model-report.json（17个案例、202个generic pinned slots，有限内存模型）；d7-authored-reference-model-report.json（31个案例、28个作者域组合，调用实际D2/D4 gate，有限phone/CAS/receipt重放）；d7-revision06-effects-model-report.json（37个条件源两种Ref顺序、独立Trash恢复/清除、家谱递归详情呈现案例）。

最后一模型仅用单fact抽象C物化器，具体输出通过原D4 Entry和D2 prefix/carrier gate；不是完整Result/9或C decoder、所有mode、完整Policy、公开EffectBytes decoder、耐久事务或D8 renderer。Trash模型只覆盖一个owner的Resource/Annotation集合，没有完整reply链或嵌套Node。delivery epoch模型不证明完整EffectItem或生产授权服务。作者引用模型的receipt重放不等于效果读取端点已实现。原17个Facet案例调用保留的旧unversioned RelationReadContext decoder；它们不证明当前RelationReadContext/2的lifecycle/ABA、masked、sourceless或stateToken边界。上述未实现边界仍是规范及后续产品conformance义务，不能将历史模型通过数扩大成新协议全域通过。
