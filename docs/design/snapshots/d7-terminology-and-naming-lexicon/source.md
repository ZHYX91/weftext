---
_weftext:
  id: "745612fc-347a-42d9-8863-a59f37d0389b"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

# D7 Terminology and Naming Lexicon — revision06-draft

同包机器Registry和本Lexicon逐项一致；上游owned concepts不重新分配。历史/自然语言/反例不是受控positive surface。

## Query Specification / 查询定义
- **conceptId**: weftext.term.query-spec
- **zh**: 查询定义
- **en**: Query Specification
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 通用typed relation DAG及唯一CEL表达式的只读定义
- **exclusions**: 不是View layout、Action或每领域专用语法
- **ownedNames**: ["QuerySpec", "weftext.query"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type QuerySpec; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.query_spec
- **uiZh**: 查询定义
- **uiEn**: Query Specification
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: equipment和people复用scan/read / 禁scan_equipment
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Query Call / 查询调用
- **conceptId**: weftext.term.query-call
- **zh**: 查询调用
- **en**: Query Call
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: inline/saved query与参数/context的一次明确调用
- **exclusions**: 不是保存者权限或新内容identity
- **ownedNames**: ["QueryCall", "D7.QueryCall"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type QueryCall; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.query_call
- **uiZh**: 查询调用
- **uiEn**: Query Call
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: security-invoker / owner权限不继承
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Query Execution / 查询执行
- **conceptId**: weftext.term.query-execution
- **zh**: 查询执行
- **en**: Query Execution
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 同cut完整求值并经当前授权交付的Core执行
- **exclusions**: 不是renderer计算或partial-success stream
- **ownedNames**: ["QueryExecution", "d7_query_execute"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type QueryExecution; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.execution
- **uiZh**: 查询执行
- **uiEn**: Query Execution
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 末尾错误全失败 / 不能先发布chart
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Terminal Schema / 终端结果结构
- **conceptId**: weftext.term.terminal-schema
- **zh**: 终端结果结构
- **en**: Terminal Schema
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 完整公共结果kind、columnId和精确TypeSpec
- **exclusions**: 不是D4 FacetSchema、数据库schema或按首行推断
- **ownedNames**: ["TerminalSchema", "D7.TerminalSchema"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type TerminalSchema; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.terminal_schema
- **uiZh**: 终端结果结构
- **uiEn**: Terminal Schema
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: schema先定 / 缺列不能按label补
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Result Column / 结果列
- **conceptId**: weftext.term.result-column
- **zh**: 结果列
- **en**: Result Column
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 终端公开列的名称标识和类型
- **exclusions**: 不是D4 FieldId、source identity或持久列实体
- **ownedNames**: ["ResultColumn", "D7.columnId"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type ResultColumn; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.result_column
- **uiZh**: 结果列
- **uiEn**: Result Column
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: serials输出列 / equipment/serial-number是FieldId
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Typed Literal / 类型化字面值
- **conceptId**: weftext.term.typed-literal
- **zh**: 类型化字面值
- **en**: Typed Literal
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 在显式TypeSpec内编码的参数/常量值
- **exclusions**: 不是null、动态JSON或未绑定参数
- **ownedNames**: ["TypedLiteral", "D7.TypedLiteral"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type TypedLiteral; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.typed_literal
- **uiZh**: 类型化字面值
- **uiEn**: Typed Literal
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: Optional.none / 缺必填参数报错
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Query Value Type / 查询值类型
- **conceptId**: weftext.term.query-type
- **zh**: 查询值类型
- **en**: Query Value Type
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 可机械检查的公共值代数及D4桥
- **exclusions**: 不是运行时任意对象、Record域或host double
- **ownedNames**: ["QueryType", "D7.TypeSpec"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type QueryType; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.value_type
- **uiZh**: 查询值类型
- **uiEn**: Query Value Type
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: CamelCase D4对象保留 / 禁任意map
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Weftext CEL Profile / Weftext CEL 配置
- **conceptId**: weftext.term.cel-profile
- **zh**: Weftext CEL 配置
- **en**: Weftext CEL Profile
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 唯一CEL语法的固定overload、总性与有界宏合同
- **exclusions**: 不是D2 AsciiDoc Profile或Facet
- **ownedNames**: ["WeftextCelProfile", "weftext.cel/1"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type WeftextCelProfile; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.cel_profile
- **uiZh**: Weftext CEL 配置
- **uiEn**: Weftext CEL Profile
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 显式decimal divide / 无JS fallback
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Relation DAG / 关系有向无环图
- **conceptId**: weftext.term.relation-dag
- **zh**: 关系有向无环图
- **en**: Relation DAG
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: bag关系和非相关scalar联合依赖图
- **exclusions**: 不是领域关系图或固定pipeline
- **ownedNames**: ["RelationDag", "D7.RelationDag"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type RelationDag; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.relation_dag
- **uiZh**: 关系有向无环图
- **uiEn**: Relation DAG
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 共享global scalar / joint cycle拒绝
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## View Specification / 视图定义
- **conceptId**: weftext.term.view-spec
- **zh**: 视图定义
- **en**: View Specification
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 完整typed result到纯呈现通道的闭合绑定
- **exclusions**: 不是Query、脚本、隐式dereference或Action
- **ownedNames**: ["ViewSpec", "weftext.view"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type ViewSpec; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: view.view_spec
- **uiZh**: 视图定义
- **uiEn**: View Specification
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: line检查x顺序 / 不renderer排序
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Panel Partition / 分面板
- **conceptId**: weftext.term.panel-partition
- **zh**: 分面板
- **en**: Panel Partition
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 同一完整结果按显式key作纯展示分面
- **exclusions**: 不是D4 Facet或多Query dashboard
- **ownedNames**: ["PanelPartition", "D7.ViewSpec.options.partition"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type PanelPartition; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: view.panel_partition
- **uiZh**: 分面板
- **uiEn**: Panel Partition
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 单Query多个panel / 不生成隐藏空panel
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Graph Result / 关系图结果
- **conceptId**: weftext.term.graph-result
- **zh**: 关系图结果
- **en**: Graph Result
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 有完整节点和边、类型和端点验证的无身份结果
- **exclusions**: 不是新作者关系库、布局距离或Node identity
- **ownedNames**: ["GraphResult", "weftext.query.graph-result/1"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type GraphResult; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.graph_result
- **uiZh**: 关系图结果
- **uiEn**: Graph Result
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: isolated node保留 / hidden endpoint不造placeholder
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Search Contribution / 搜索贡献定义
- **conceptId**: weftext.term.search-contribution
- **zh**: 搜索贡献定义
- **en**: Search Contribution
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 经接纳的Field text path和role纯数据描述
- **exclusions**: 不是私有全文源、网络权限或新业务Query语法
- **ownedNames**: ["SearchContribution", "D7.SearchContribution"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type SearchContribution; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.search_contribution
- **uiZh**: 搜索贡献定义
- **uiEn**: Search Contribution
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 新设备nameField descriptor / 禁provider脚本
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Action Specification / 动作定义
- **conceptId**: weftext.term.action-spec
- **zh**: 动作定义
- **en**: Action Specification
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 一次closed显式作者修改意图
- **exclusions**: 不是Query写算子、独立实体或第三ledger
- **ownedNames**: ["ActionSpec", "weftext.action"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type ActionSpec; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: action.action_spec
- **uiZh**: 动作定义
- **uiEn**: Action Specification
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: FieldSelector精确修改 / 禁rowIndex删除
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Action Preparation / 动作准备
- **conceptId**: weftext.term.action-preparation
- **zh**: 动作准备
- **en**: Action Preparation
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 当前资格下产生完整拟议源和immutable准备结果
- **exclusions**: 不是D3 planned或已分配content identity
- **ownedNames**: ["ActionPreparation", "d7_action_prepare"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type ActionPreparation; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: action.preparation
- **uiZh**: 动作准备
- **uiEn**: Action Preparation
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: preview无author写 / commit仍D3或D6
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Action Preview / 动作预览
- **conceptId**: weftext.term.action-preview
- **zh**: 动作预览
- **en**: Action Preview
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 当前受权的完整确定或受限条件拟议效果及原协议提交请求；不是某个采样candidate map的确定结果。
- **exclusions**: 不是committed receipt或权限票据
- **ownedNames**: ["ActionPreview", "d7_action_prepared"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type ActionPreview; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: action.preview
- **uiZh**: 动作预览
- **uiEn**: Action Preview
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: preview200运输 / 目标范围仍全部
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Effects Page / 效果分页
- **conceptId**: weftext.term.effects-page
- **zh**: 效果分页
- **en**: Effects Page
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: D7 closed adapters完整effects的受权运输
- **exclusions**: 不是Query rows或通用控制JSON
- **ownedNames**: ["EffectsPage", "d7_effects_page"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type EffectsPage; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: action.effects_page
- **uiZh**: 效果分页
- **uiEn**: Effects Page
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: effects_cursor不同result_cursor / wrongtag拒绝
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Dynamic Block / 动态块
- **conceptId**: weftext.term.dynamic-block
- **zh**: 动态块
- **en**: Dynamic Block
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: D2 saved-definition occurrence中的Query/View调用绑定
- **exclusions**: 不是新D2元素、SavedView实体或结果快照
- **ownedNames**: ["DynamicBlock", "weftext.dynamic-block"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type DynamicBlock; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: view.dynamic_block
- **uiZh**: 动态块
- **uiEn**: Dynamic Block
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 保存this词法绑定 / 不保存resolved this
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Definition Address / 定义地址
- **conceptId**: weftext.term.definition-address
- **zh**: 定义地址
- **en**: Definition Address
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: NodeRef加D3 anchor/Locator寻址源内定义
- **exclusions**: 不是ViewRef或独立definitionId
- **ownedNames**: ["DefinitionAddress", "D7.DefinitionAddress"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type DefinitionAddress; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.definition_address
- **uiZh**: 定义地址
- **uiEn**: Definition Address
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: source revision变化重解析 / 不按title回退
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Result Delta / 结果增量
- **conceptId**: weftext.term.result-delta
- **zh**: 结果增量
- **en**: Result Delta
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 同授权epoch两个完整batch结果间的原子替换事件
- **exclusions**: 不是作者写入或任意增量近似
- **ownedNames**: ["ResultDelta", "d7_delta"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type ResultDelta; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.result_delta
- **uiZh**: 结果增量
- **uiEn**: Result Delta
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: gap reset / 不半应用
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Result Subscription / 结果订阅
- **conceptId**: weftext.term.result-subscription
- **zh**: 结果订阅
- **en**: Result Subscription
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 受管runtime结果更新运输及epoch/sequence
- **exclusions**: 不是ICS外部订阅、内容实体或永久token
- **ownedNames**: ["ResultSubscription", "d7_subscribe"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type ResultSubscription; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.result_subscription
- **uiZh**: 结果订阅
- **uiEn**: Result Subscription
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 旧epoch立即失效 / 不能等reset送达才拒绝
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Saved Query Definition / 保存查询载体
- **conceptId**: weftext.term.saved-query-definition
- **zh**: 保存查询载体
- **en**: Saved Query Definition
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 完整QuerySpec及可选源内创建策略的D2保存payload
- **exclusions**: 没有独立实体/身份/保存者权限
- **ownedNames**: ["SavedQueryDefinition", "weftext.saved-query"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.saved_query_definition
- **uiZh**: 保存查询载体
- **uiEn**: Saved Query Definition
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 正例：保存 QuerySpec 与可选 creationPolicy 于 D2 定义块；反例：为保存定义分配独立内容 Ref，或执行时继承保存者权限。
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Prepared Action Binding / 动作准备绑定
- **conceptId**: weftext.term.prepared-action-binding
- **zh**: 动作准备绑定
- **en**: Prepared Action Binding
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 绑定D7完整输入/条件、不可变语义预览和初始交付投影与唯一原D3/D6请求的受管证据。
- **exclusions**: 不是第三ledger或可变token目标
- **ownedNames**: ["PreparedActionBinding", "d7_prepared_action_binding"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.prepared_action_binding
- **uiZh**: 动作准备绑定
- **uiEn**: Prepared Action Binding
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 正例：同一 token 不可变地绑定完整预览和原 D3/D6 request；反例：替换既有 token 的目标，或用独立 ledger 决定提交。
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Effect Manifest / 完整效果清单
- **conceptId**: weftext.term.effect-manifest
- **zh**: 完整效果清单
- **en**: Effect Manifest
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 在首分页前完整验证的不可变语义效果，按一个交付epoch形成完整closed preview或committed运输投影；含适用的条件源预览。
- **exclusions**: 不能按页丢项或用preview冒充receipt
- **ownedNames**: ["EffectManifest", "weftext.effects"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.effect_manifest
- **uiZh**: 完整效果清单
- **uiEn**: Effect Manifest
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 正例：301项完整效果先固定全部字节slots，再形成同epoch稳定投影；反例：翻页时追加效果、换handle，或把条件源当确定修改。
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Effect Bytes / 效果字节句柄
- **conceptId**: weftext.term.effect-bytes
- **zh**: 效果字节句柄
- **en**: Effect Bytes
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 绑定phase、item字节slot、交付epoch和完整encoded pinned bytes的受控运输句柄；重签发不改变原语义效果。
- **exclusions**: 不是D6 resource_bytes/1或fresh ResourceRef
- **ownedNames**: ["EffectBytes", "effect_bytes/1", "d7_effect_bytes_read", "d7_effect_bytes_chunk"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.effect_bytes
- **uiZh**: 效果字节句柄
- **uiEn**: Effect Bytes
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 正例：preview phase 的 effect_bytes/1 运输尚未创建资源的拟议 bytes；反例：伪造 fresh ResourceRef 后借 resource_bytes/1 读取拟议内容。
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Field Selection / 字段条目选择
- **conceptId**: weftext.term.field-selection
- **zh**: 字段条目选择
- **en**: Field Selection
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 完整当前Field按原作者序的revision-bound临时选择及显式证据
- **exclusions**: 没有FieldRef、跨epoch身份或unnest写lineage
- **ownedNames**: ["FieldSelection", "d7_field_selection_open", "d7_field_selection", "d7_field_selection_evidence"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.field_selection
- **uiZh**: 字段条目选择
- **uiEn**: Field Selection
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 正例：按当前 sourceRevision 区分同值不同 key 的第二个 phone Entry；反例：拿旧 epoch 的选择或 unnest 行号直接删除当前 Field。
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Source Envelope / 源包络元数据
- **conceptId**: weftext.term.source-envelope
- **zh**: 源包络元数据
- **en**: Source Envelope
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 明确受权的源版本/容量/聚合有效性元数据投影
- **exclusions**: 不含body/其它Field/隐藏具体错误
- **ownedNames**: ["SourceEnvelope", "d7_source_envelope_read", "d7_source_envelope"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.source_envelope
- **uiZh**: 源包络元数据
- **uiEn**: Source Envelope
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 正例：获得 source_envelope_state 后读取该源 revision 与聚合有效性；反例：借元数据响应带出 body、其它 Field 值或隐藏 Field 的具体错误。
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Basis Projection / 量值基准投影
- **conceptId**: weftext.term.basis-projection
- **zh**: 量值基准投影
- **en**: Basis Projection
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 返回实际quantity或calendar值的完整已绑定基准描述供显式同基准判断
- **exclusions**: label/magnitude不是单位证明；不加载新provider
- **ownedNames**: ["quantityBasis", "dateBasis"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.basis_projection
- **uiZh**: 量值基准投影
- **uiEn**: Basis Projection
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 正例：比较两个 quantityBasis 的完整 unitId 与 dimension 后显式聚合；反例：依据同一 unitLabel 把 m 与 cm 当同单位相加。
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Fact Origin / 事实来源轨迹
- **conceptId**: weftext.term.fact-origin
- **zh**: 事实来源轨迹
- **en**: Fact Origin
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: Core内部逐值/item保留的asserted图事实来源证据
- **exclusions**: 不是公共作者provenance或implicit Action lineage
- **ownedNames**: ["FactOrigin", "D7.FactOrigin"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.fact_origin
- **uiZh**: 事实来源轨迹
- **uiEn**: Fact Origin
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 正例：保留真实关系 Entry 的内部逐值证据后输出 asserted 边；反例：把任意 project 构造的端点标成 asserted，或把作者 provenance 当写权限。
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Definition Transfer Effects / 定义转移效果
- **conceptId**: weftext.term.definition-transfer-effects
- **zh**: 定义转移效果
- **en**: Definition Transfer Effects
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 原同一decision中完整保存定义typed引用/Locator前后像证据
- **exclusions**: 不成为D3 node_link/citation slot或定义身份
- **ownedNames**: ["D7DefinitionTransferEffects", "d7_definition_transfer_effects"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.definition_transfer_effects
- **uiZh**: 定义转移效果
- **uiEn**: Definition Transfer Effects
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 正例：同一原 decision 记录 typed Ref 与真实 Locator 的转移前后像；反例：重写普通 text 中像 UUID 的内容，或把 Q 定义当 node_link slot。
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Field Entry Image / 字段条目前后像
- **conceptId**: weftext.term.field-entry-image
- **zh**: 字段条目前后像
- **en**: Field Entry Image
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 按原作者序完整Field raw Entry的明确投影
- **exclusions**: 不是删掉秘密字段后的exact SourceSnapshot
- **ownedNames**: ["FieldEntryImage", "weftext.field-entries"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.field_entry_image
- **uiZh**: 字段条目前后像
- **uiEn**: Field Entry Image
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 正例：按原作者序运输完整 people/phone Field 的 before/after Entry；反例：删掉其它字段后将投影冒充 exact SourceSnapshot。
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Derived Period Range / 派生周期区间
- **conceptId**: weftext.term.derived-period-range
- **zh**: 派生周期区间
- **en**: Derived Period Range
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 从完整已验证CalendarPeriod值产生civil日期边界或明确不可表示状态，并保留原period与seriesKey的只读投影。
- **exclusions**: 不新增作者range、Event、CalendarPeriod身份或通用CEL日期构造器。
- **ownedNames**: ["DerivedPeriodRange", "derived_period_range", "weftext.query.derived-period-range/1"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: D7 read source.kind=derived_period_range; DerivedPeriodRange<T> is an existing object/union type expansion, not a primitive.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: query.derived_period_range
- **uiZh**: 派生周期区间
- **uiEn**: Derived Period Range
- **allowedShort**: Use full name in controlled surfaces; no additional wire alias.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 2026-Q3得到2026-07-01至2026-10-01；9999年原值合法但endpoint_out_of_domain，禁止截为9999-12-31或丢行。
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Conditional Source Change / 条件源修改预览
- **conceptId**: weftext.term.conditional-source-change
- **zh**: 条件源修改预览
- **en**: Conditional Source Change
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 仅full preview中含C的existing Node完整Result/9源变换，按原同一candidate map和exact raw比较唯一确定不改版或checked加一及实际效果。
- **exclusions**: 不是已发生修改、用户脚本、预先分配的content ID或committed EffectItem。
- **ownedNames**: ["ConditionalSourceChange", "conditional_source_change"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Type ConditionalSourceChange; wire EffectItem.kind=conditional_source_change; only the exact closed result/revisionRule from D7 Transport.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: action.conditional_source_change
- **uiZh**: 条件源修改预览
- **uiEn**: Conditional Source Change
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 正例：fresh关系canonical owner两种次序使既有A保持revision7或修改到8；反例：先抽一个UUID把A显示为必改，再静默换分支。
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Effect Delivery Epoch / 效果交付世代
- **conceptId**: weftext.term.effect-delivery-epoch
- **zh**: 效果交付世代
- **en**: Effect Delivery Epoch
- **owner**: D7
- **layer**: query/view/action runtime
- **definition**: 一份完整语义清单的有限只读运输投影寿命；同epoch固定全部item/byte handles，重开可创建新epoch且不改变原decision。
- **exclusions**: 不是Workspace authority/auth世代、Query订阅epoch、原effectsToken或新的作者效果。
- **ownedNames**: ["EffectDeliveryEpoch"]
- **manifest**: No executable manifest granted; D10 consumes the explicitly defined data contract only.
- **codeConvention**: Internal type EffectDeliveryEpoch; no new public request member, command or token alias; binding carried by original effects_cursor/effect_bytes/1 records.
- **cli**: No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.
- **localeKey**: action.effect_delivery_epoch
- **uiZh**: 效果交付世代
- **uiEn**: Effect Delivery Epoch
- **allowedShort**: Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.
- **rejectedAliases**: ["unlisted controlled wire/type alias"]
- **exampleAndCounterexample**: 正例：新open重签发H2，原epoch内同cursor仍返回H1直到自身到期；反例：将旧cursor悄悄重绑到新handles或延长preview TTL。
- **firstFreeze**: D7 revision06 candidate; independent acceptance pending
- **migration**: Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.
- **deletionTarget**: S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships.

## Cross-stage bindings

[
  {
    "owner": "D3 identity algebra / D7 runtime payload",
    "names": [
      "EntityRef",
      "Locator",
      "LogicalOccurrenceKey",
      "ResultRowHandle",
      "Provenance",
      "ActionEvidence"
    ],
    "rule": "Keep inherited concept ownership; D7 defines short-lived payload and propagation, not durable identity."
  },
  {
    "owner": "D4 schema and authorship",
    "names": [
      "FacetId",
      "FieldId",
      "FieldSelector",
      "authoredProvenance"
    ],
    "rule": "FieldSelector is D5/D4 revision-bound selection, not Locator. authoredProvenance is an output column role, not a replacement name for D3 Provenance."
  },
  {
    "owner": "D6 control and transport",
    "names": [
      "AuthorizedCut",
      "DependencyProof",
      "ObservationScope",
      "PreparedIntent",
      "ResultHandle",
      "ResultCursor",
      "ByteHandle",
      "SourceVersion",
      "EffectsToken"
    ],
    "rule": "Original decoder and lifetime; same-shaped Token never implies same tag or authorization."
  },
  {
    "owner": "D2 occurrence / D7 payload",
    "names": [
      "saved_query_view_definition",
      "weftext-query",
      "weftext-view"
    ],
    "rule": "Keep Profile2 envelope unchanged. DynamicBlock format is payload only."
  },
  {
    "owner": "D3",
    "names": [
      "PreparationBinding",
      "D3.identity_operation_request.preparationBinding",
      "DefinitionTransfer",
      "D3.intent.plan.definitionTransfers",
      "D3-Symbolic-Result/9.Q"
    ],
    "rule": "Explicit D3 wire11/Result9 replacement only; no undeclared member."
  },
  {
    "owner": "D6",
    "names": [
      "SourceEnvelopeStateCapability",
      "source_envelope_state",
      "CommitSequenceStateCapability",
      "commit_sequence_state"
    ],
    "rule": "Policy/2 explicit non-implied capability; original policy/profile never silently upgraded."
  },
  {
    "owner": "D4",
    "names": [
      "DerivedDuration"
    ],
    "rule": "Consume the already-frozen exact D4 result through generic read(kind=derived_duration), no D7 temporal provider."
  },
  {
    "owner": "D5",
    "names": [
      "CollectionCreationPolicy",
      "collection_creation_policy"
    ],
    "rule": "D5-owned saved creation semantics; D7 SavedQueryDefinition.creationPolicy carries parent, explicit Action supplies source/ordinal. No alternate SavedCreationPolicy type."
  }
]
