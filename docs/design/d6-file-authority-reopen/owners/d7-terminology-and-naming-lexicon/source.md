---
source_language: zh-CN
translation_of: source.zh-CN.md
translation_status: synced
---

[简体中文](source.zh-CN.md)

Source document ID: 745612fc-347a-42d9-8863-a59f37d0389b.

Candidate status: D7 file-authority coordination afterimage; not independently accepted, activated or implemented. Stage acceptance, unstarted D8/D9 labels and model counts in fixed S are historical provenance. This candidate preserves their actual semantics and evidence limits without treating those labels as current status. It consumes this package's D3 wire12/Result9, current D4 consumers and D6 Control production-version, current-observation, dependency and single-decision recovery contracts. New D3 conflict preparation and D7 /3 binding still require independent joint review of the complete package; portable Locator requalification across replicas awaits its named decision. This candidate does not claim complete closure or eligibility for activation.

# D7 Terminology and Naming Lexicon

The machine Registry and the following 34 controlled records agree byte for byte. These are machine data retaining original conceptId, bilingual names, owner, controlled spelling, localeKey and historical firstFreeze. The human explanation after each record is synchronized in both languages. Existing upstream concepts keep their owners. Historical usage, natural prose and counterexamples are not positive controlled surfaces.

Shared rules: concepts belong to D7 Query/View/Action runtime. They grant no executable manifest; D10 consumes only the explicit data contract. Code uses each exact name and applicable Query/View/Actions namespace without another public alias. This architecture freezes no CLI verb/flag; future adapters preserve exact protocols and map labels through these concepts. Each record lists Chinese/English UI labels and localeKey. Query/View/Action/CEL/DAG abbreviations are allowed only when unambiguous; Derived Period Range uses its full name on controlled surfaces. Unlisted wire/type aliases reject.

firstFreeze records fixed S provenance, not current acceptance. Unpublished prototypes may be replaced atomically in implementation, while real stored preparation/plan/receipt/unknown obligations retain original decoder/pin recovery. Impact S1–S7 lists removal of old public symbols/parser branches/fixtures/help/locales when the corresponding new contract ships; historical evidence and user prose are not removed. Complete named current-version differences and upstream consumption appear in cross-stage bindings below. A machine schema name authorizes no product activation.

## Query Specification / 查询定义

```json
{
  "conceptId": "weftext.term.query-spec",
  "zh": "查询定义",
  "en": "Query Specification",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "通用typed relation DAG及唯一CEL表达式的只读定义",
  "exclusions": "不是View layout、Action或每领域专用语法",
  "ownedNames": [
    "QuerySpec",
    "weftext.query"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type QuerySpec; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.query_spec",
  "uiZh": "查询定义",
  "uiEn": "Query Specification",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "equipment和people复用scan/read / 禁scan_equipment",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

A read-only generic typed relation DAG with the sole CEL expression language. Not a View layout, Action or per-domain syntax. Equipment and people reuse scan/read; scan_equipment is forbidden.

## Query Call / 查询调用

```json
{
  "conceptId": "weftext.term.query-call",
  "zh": "查询调用",
  "en": "Query Call",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "inline/saved query与参数/context的一次明确调用",
  "exclusions": "不是保存者权限或新内容identity",
  "ownedNames": [
    "QueryCall",
    "D7.QueryCall"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type QueryCall; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.query_call",
  "uiZh": "查询调用",
  "uiEn": "Query Call",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "security-invoker / owner权限不继承",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

One explicit inline/saved Query call with arguments and context. Not the saving principal's authority or a new content identity. Execution uses security-invoker rights; it inherits no owner permission.

## Query Execution / 查询执行

```json
{
  "conceptId": "weftext.term.query-execution",
  "zh": "查询执行",
  "en": "Query Execution",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "同cut完整求值并经当前授权交付的Core执行",
  "exclusions": "不是renderer计算或partial-success stream",
  "ownedNames": [
    "QueryExecution",
    "d7_query_execute"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type QueryExecution; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.execution",
  "uiZh": "查询执行",
  "uiEn": "Query Execution",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "末尾错误全失败 / 不能先发布chart",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Complete Core evaluation at one cut, delivered under current authorization. Not renderer computation or partial-success streaming. A final-row error fails the whole result; a chart cannot publish first.

## Terminal Schema / 终端结果结构

```json
{
  "conceptId": "weftext.term.terminal-schema",
  "zh": "终端结果结构",
  "en": "Terminal Schema",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "完整公共结果kind、columnId和精确TypeSpec",
  "exclusions": "不是D4 FacetSchema、数据库schema或按首行推断",
  "ownedNames": [
    "TerminalSchema",
    "D7.TerminalSchema"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type TerminalSchema; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.terminal_schema",
  "uiZh": "终端结果结构",
  "uiEn": "Terminal Schema",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "schema先定 / 缺列不能按label补",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Complete public result kind, columnIds and exact TypeSpecs. Not D4 FacetSchema, database schema or first-row inference. Fix schema first; a missing column cannot be reconstructed by label.

## Result Column / 结果列

```json
{
  "conceptId": "weftext.term.result-column",
  "zh": "结果列",
  "en": "Result Column",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "终端公开列的名称标识和类型",
  "exclusions": "不是D4 FieldId、source identity或持久列实体",
  "ownedNames": [
    "ResultColumn",
    "D7.columnId"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type ResultColumn; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.result_column",
  "uiZh": "结果列",
  "uiEn": "Result Column",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "serials输出列 / equipment/serial-number是FieldId",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

The terminal public column's name identity and type. Not D4 FieldId, source identity or a durable column entity. serials is an output column; equipment/serial-number is a FieldId.

## Typed Literal / 类型化字面值

```json
{
  "conceptId": "weftext.term.typed-literal",
  "zh": "类型化字面值",
  "en": "Typed Literal",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "在显式TypeSpec内编码的参数/常量值",
  "exclusions": "不是null、动态JSON或未绑定参数",
  "ownedNames": [
    "TypedLiteral",
    "D7.TypedLiteral"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type TypedLiteral; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.typed_literal",
  "uiZh": "类型化字面值",
  "uiEn": "Typed Literal",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "Optional.none / 缺必填参数报错",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

A parameter or constant encoded inside an explicit TypeSpec. Not null, dynamic JSON or an unbound parameter. Optional.none is explicit; a missing required parameter errors.

## Query Value Type / 查询值类型

```json
{
  "conceptId": "weftext.term.query-type",
  "zh": "查询值类型",
  "en": "Query Value Type",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "可机械检查的公共值代数及D4桥",
  "exclusions": "不是运行时任意对象、Record域或host double",
  "ownedNames": [
    "QueryType",
    "D7.TypeSpec"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type QueryType; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.value_type",
  "uiZh": "查询值类型",
  "uiEn": "Query Value Type",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "CamelCase D4对象保留 / 禁任意map",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Mechanically checked public value algebra and its D4 bridge. Not arbitrary runtime objects, a Record domain or host double. Preserve CamelCase D4 objects; arbitrary maps are forbidden.

## Weftext CEL Profile / Weftext CEL 配置

```json
{
  "conceptId": "weftext.term.cel-profile",
  "zh": "Weftext CEL 配置",
  "en": "Weftext CEL Profile",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "唯一CEL语法的固定overload、总性与有界宏合同",
  "exclusions": "不是D2 AsciiDoc Profile或Facet",
  "ownedNames": [
    "WeftextCelProfile",
    "weftext.cel/1"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type WeftextCelProfile; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.cel_profile",
  "uiZh": "Weftext CEL 配置",
  "uiEn": "Weftext CEL Profile",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "显式decimal divide / 无JS fallback",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Fixed overloads, totality and bounded macros for the sole CEL grammar. Not D2 AsciiDoc Profile or a Facet. Decimal division is explicit; no JS fallback exists.

## Relation DAG / 关系有向无环图

```json
{
  "conceptId": "weftext.term.relation-dag",
  "zh": "关系有向无环图",
  "en": "Relation DAG",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "bag关系和非相关scalar联合依赖图",
  "exclusions": "不是领域关系图或固定pipeline",
  "ownedNames": [
    "RelationDag",
    "D7.RelationDag"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type RelationDag; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.relation_dag",
  "uiZh": "关系有向无环图",
  "uiEn": "Relation DAG",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "共享global scalar / joint cycle拒绝",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

The joint dependency graph of bag relations and noncorrelated scalars. Not the business relationship graph or a fixed pipeline. Global scalars can be shared; joint cycles reject.

## View Specification / 视图定义

```json
{
  "conceptId": "weftext.term.view-spec",
  "zh": "视图定义",
  "en": "View Specification",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "完整typed result到纯呈现通道的闭合绑定",
  "exclusions": "不是Query、脚本、隐式dereference或Action",
  "ownedNames": [
    "ViewSpec",
    "weftext.view"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type ViewSpec; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "view.view_spec",
  "uiZh": "视图定义",
  "uiEn": "View Specification",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "line检查x顺序 / 不renderer排序",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Closed bindings from complete typed results to presentation-only channels. Not Query, script, implicit dereference or Action. line verifies x order; the renderer does not sort it.

## Panel Partition / 分面板

```json
{
  "conceptId": "weftext.term.panel-partition",
  "zh": "分面板",
  "en": "Panel Partition",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "同一完整结果按显式key作纯展示分面",
  "exclusions": "不是D4 Facet或多Query dashboard",
  "ownedNames": [
    "PanelPartition",
    "D7.ViewSpec.options.partition"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type PanelPartition; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "view.panel_partition",
  "uiZh": "分面板",
  "uiEn": "Panel Partition",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "单Query多个panel / 不生成隐藏空panel",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Presentation-only partition of one complete result by explicit key. Not D4 Facet or a multi-Query dashboard. One Query can have several panels; hidden empty panels are not invented.

## Graph Result / 关系图结果

```json
{
  "conceptId": "weftext.term.graph-result",
  "zh": "关系图结果",
  "en": "Graph Result",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "有完整节点和边、类型和端点验证的无身份结果",
  "exclusions": "不是新作者关系库、布局距离或Node identity",
  "ownedNames": [
    "GraphResult",
    "weftext.query.graph-result/1"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type GraphResult; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.graph_result",
  "uiZh": "关系图结果",
  "uiEn": "Graph Result",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "isolated node保留 / hidden endpoint不造placeholder",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

An identity-free result with complete nodes/edges and verified types/endpoints. Not a new author relationship database, layout-distance fact or Node identity. Keep isolated nodes; do not create placeholders for hidden endpoints.

## Search Contribution / 搜索贡献定义

```json
{
  "conceptId": "weftext.term.search-contribution",
  "zh": "搜索贡献定义",
  "en": "Search Contribution",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "经接纳的Field text path和role纯数据描述",
  "exclusions": "不是私有全文源、网络权限或新业务Query语法",
  "ownedNames": [
    "SearchContribution",
    "D7.SearchContribution"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type SearchContribution; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.search_contribution",
  "uiZh": "搜索贡献定义",
  "uiEn": "Search Contribution",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "新设备nameField descriptor / 禁provider脚本",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Accepted pure-data descriptions of Field text paths and search roles. Not a private full-text source, network permission or business-specific Query grammar. New equipment nameField descriptors are allowed; provider scripts are not.

## Action Specification / 动作定义

```json
{
  "conceptId": "weftext.term.action-spec",
  "zh": "动作定义",
  "en": "Action Specification",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "一次closed显式作者修改意图",
  "exclusions": "不是Query写算子、独立实体或第三ledger",
  "ownedNames": [
    "ActionSpec",
    "weftext.action"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type ActionSpec; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "action.action_spec",
  "uiZh": "动作定义",
  "uiEn": "Action Specification",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "FieldSelector精确修改 / 禁rowIndex删除",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

One explicit closed author-mutation intention. Not a Query write operator, separate entity or third ledger. Use exact FieldSelector modification, never rowIndex deletion.

## Action Preparation / 动作准备

```json
{
  "conceptId": "weftext.term.action-preparation",
  "zh": "动作准备",
  "en": "Action Preparation",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "当前资格下产生完整拟议源和immutable准备结果",
  "exclusions": "不是D3 planned或已分配content identity",
  "ownedNames": [
    "ActionPreparation",
    "d7_action_prepare"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type ActionPreparation; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "action.preparation",
  "uiZh": "动作准备",
  "uiEn": "Action Preparation",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "preview无author写 / commit仍D3或D6",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Producing complete proposed sources and an immutable preparation under current qualification. Not D3 planned state or an allocated content identity. Preview writes no author data; D3 or D6 still commits.

## Action Preview / 动作预览

```json
{
  "conceptId": "weftext.term.action-preview",
  "zh": "动作预览",
  "en": "Action Preview",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "当前受权的完整确定或受限条件拟议效果及原协议提交请求；不是某个采样candidate map的确定结果。",
  "exclusions": "不是committed receipt或权限票据",
  "ownedNames": [
    "ActionPreview",
    "d7_action_prepared"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type ActionPreview; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "action.preview",
  "uiZh": "动作预览",
  "uiEn": "Action Preview",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "preview200运输 / 目标范围仍全部",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Currently authorized complete determined or narrowly conditional proposed effects and the original submit request, not the determined outcome of a sampled candidate map. Not a committed receipt or permission ticket. A 200-item preview page is transport; targets remain complete.

## Effects Page / 效果分页

```json
{
  "conceptId": "weftext.term.effects-page",
  "zh": "效果分页",
  "en": "Effects Page",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "D7 closed adapters完整effects的受权运输",
  "exclusions": "不是Query rows或通用控制JSON",
  "ownedNames": [
    "EffectsPage",
    "d7_effects_page"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type EffectsPage; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "action.effects_page",
  "uiZh": "效果分页",
  "uiEn": "Effects Page",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "effects_cursor不同result_cursor / wrongtag拒绝",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Authorized transport of complete effects from closed D7 adapters. Not Query rows or generic control JSON. effects_cursor differs from result_cursor; a wrong tag rejects.

## Dynamic Block / 动态块

```json
{
  "conceptId": "weftext.term.dynamic-block",
  "zh": "动态块",
  "en": "Dynamic Block",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "D2 saved-definition occurrence中的Query/View调用绑定",
  "exclusions": "不是新D2元素、SavedView实体或结果快照",
  "ownedNames": [
    "DynamicBlock",
    "weftext.dynamic-block"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type DynamicBlock; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "view.dynamic_block",
  "uiZh": "动态块",
  "uiEn": "Dynamic Block",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "保存this词法绑定 / 不保存resolved this",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Query/View invocation bindings inside a D2 saved-definition occurrence. Not a new D2 element, SavedView entity or result snapshot. Save lexical this binding, never resolved this.

## Definition Address / 定义地址

```json
{
  "conceptId": "weftext.term.definition-address",
  "zh": "定义地址",
  "en": "Definition Address",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "NodeRef加D3 anchor/Locator寻址源内定义",
  "exclusions": "不是ViewRef或独立definitionId",
  "ownedNames": [
    "DefinitionAddress",
    "D7.DefinitionAddress"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type DefinitionAddress; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.definition_address",
  "uiZh": "定义地址",
  "uiEn": "Definition Address",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "source revision变化重解析 / 不按title回退",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

NodeRef plus a D3 anchor/Locator addressing a definition inside source. Not ViewRef or a separate definitionId. Reparse after source-version change; do not fall back by title.

## Result Delta / 结果增量

```json
{
  "conceptId": "weftext.term.result-delta",
  "zh": "结果增量",
  "en": "Result Delta",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "同授权epoch两个完整batch结果间的原子替换事件",
  "exclusions": "不是作者写入或任意增量近似",
  "ownedNames": [
    "ResultDelta",
    "d7_delta"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type ResultDelta; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.result_delta",
  "uiZh": "结果增量",
  "uiEn": "Result Delta",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "gap reset / 不半应用",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

An atomic replacement event between two complete batch results in the same authorization epoch. Not an author write or arbitrary incremental approximation. A gap resets; never apply half a delta.

## Result Subscription / 结果订阅

```json
{
  "conceptId": "weftext.term.result-subscription",
  "zh": "结果订阅",
  "en": "Result Subscription",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "受管runtime结果更新运输及epoch/sequence",
  "exclusions": "不是ICS外部订阅、内容实体或永久token",
  "ownedNames": [
    "ResultSubscription",
    "d7_subscribe"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type ResultSubscription; namespace Weftext.Core.Query / View / Actions according to owner. Wire uses only the exact listed spelling.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.result_subscription",
  "uiZh": "结果订阅",
  "uiEn": "Result Subscription",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "旧epoch立即失效 / 不能等reset送达才拒绝",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Managed runtime transport for result updates with epoch/sequence. Not external ICS subscription, content entity or permanent token. Old epoch invalidation is immediate, not delayed until reset delivery.

## Saved Query Definition / 保存查询载体

```json
{
  "conceptId": "weftext.term.saved-query-definition",
  "zh": "保存查询载体",
  "en": "Saved Query Definition",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "完整QuerySpec及可选源内创建策略的D2保存payload",
  "exclusions": "没有独立实体/身份/保存者权限",
  "ownedNames": [
    "SavedQueryDefinition",
    "weftext.saved-query"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.saved_query_definition",
  "uiZh": "保存查询载体",
  "uiEn": "Saved Query Definition",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "正例：保存 QuerySpec 与可选 creationPolicy 于 D2 定义块；反例：为保存定义分配独立内容 Ref，或执行时继承保存者权限。",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

A D2 saved payload containing complete QuerySpec and optional source-local creation policy. It has no separate entity, identity or saving-principal authority. Store QuerySpec/optional creationPolicy in the definition block; do not allocate a content Ref or inherit the saver's rights.

## Prepared Action Binding / 动作准备绑定

```json
{
  "conceptId": "weftext.term.prepared-action-binding",
  "zh": "动作准备绑定",
  "en": "Prepared Action Binding",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "绑定D7完整输入/条件、不可变语义预览和初始交付投影与唯一原D3/D6请求的受管证据。",
  "exclusions": "不是第三ledger或可变token目标",
  "ownedNames": [
    "PreparedActionBinding",
    "d7_prepared_action_binding"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.prepared_action_binding",
  "uiZh": "动作准备绑定",
  "uiEn": "Prepared Action Binding",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "正例：同一 token 不可变地绑定完整预览和原 D3/D6 request；反例：替换既有 token 的目标，或用独立 ledger 决定提交。",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Managed evidence binding complete D7 inputs/conditions, immutable semantic preview and initial delivery projection to one original D3/D6 request. Not a third ledger or mutable token target. A token immutably binds full preview/request; redirecting it or deciding commit in another ledger is forbidden.

## Effect Manifest / 完整效果清单

```json
{
  "conceptId": "weftext.term.effect-manifest",
  "zh": "完整效果清单",
  "en": "Effect Manifest",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "在首分页前完整验证的不可变语义效果，按一个交付epoch形成完整closed preview或committed运输投影；含适用的条件源预览。",
  "exclusions": "不能按页丢项或用preview冒充receipt",
  "ownedNames": [
    "EffectManifest",
    "weftext.effects"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.effect_manifest",
  "uiZh": "完整效果清单",
  "uiEn": "Effect Manifest",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "正例：301项完整效果先固定全部字节slots，再形成同epoch稳定投影；反例：翻页时追加效果、换handle，或把条件源当确定修改。",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Complete immutable semantic effects validated before the first page and projected into one closed preview/committed delivery epoch, including applicable conditional sources. Items cannot disappear per page and preview cannot impersonate a receipt. Fix all byte slots for 301 effects before a stable epoch projection; do not append effects, change handles while paging or label conditional source as certain change.

## Effect Bytes / 效果字节句柄

```json
{
  "conceptId": "weftext.term.effect-bytes",
  "zh": "效果字节句柄",
  "en": "Effect Bytes",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "绑定phase、item字节slot、交付epoch和完整encoded pinned bytes的受控运输句柄；重签发不改变原语义效果。",
  "exclusions": "不是D6 resource_bytes/1或fresh ResourceRef",
  "ownedNames": [
    "EffectBytes",
    "effect_bytes/1",
    "effect_bytes/2",
    "d7_effect_bytes_read",
    "d7_effect_bytes_chunk"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.effect_bytes",
  "uiZh": "效果字节句柄",
  "uiEn": "Effect Bytes",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "正例：preview 的 effect_bytes/2 运输尚未创建资源的完整拟议字节；历史 /1 仅恢复原记录。反例：伪造 fresh ResourceRef 后借 resource_bytes/1 读取拟议内容。",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Controlled transport bound to phase, item byte slot, delivery epoch and complete encoded pinned bytes; reissuance does not change original effects. Not D6 resource_bytes/1 or a fresh ResourceRef. Preview effect_bytes/2 carries complete proposed bytes before Resource creation; historical /1 restores real records only. Never invent a fresh ResourceRef to read proposed bytes through resource_bytes/1.

## Field Selection / 字段条目选择

```json
{
  "conceptId": "weftext.term.field-selection",
  "zh": "字段条目选择",
  "en": "Field Selection",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "按完整当前观察及真实生产版本绑定的整个 Field 作者序临时选择与显式证据；保留原 inner revision-bound selector。",
  "exclusions": "没有FieldRef、跨epoch身份或unnest写lineage",
  "ownedNames": [
    "FieldSelection",
    "d7_field_selection_open",
    "d7_field_selection",
    "d7_field_selection_evidence"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.field_selection",
  "uiZh": "字段条目选择",
  "uiEn": "Field Selection",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "正例：同 cut 的 SourceVersionRef 与精确 occurrenceKey/raw 选择第二个同值电话；反例：A:1/B:1 只比 Counter、旧 epoch 选择或 unnest 行号直接删当前条目。",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Temporary whole-Field selection in author order, bound to complete current observation and real production version, with explicit evidence and original inner revision-bound selector. No FieldRef, cross-epoch identity or unnest write lineage. A same-cut SourceVersionRef plus exact occurrenceKey/raw selects the second equal phone. Counter-only A:1/B:1 matching, old epoch selection or unnest row index cannot delete current Entries.

## Source Envelope / 源包络元数据

```json
{
  "conceptId": "weftext.term.source-envelope",
  "zh": "源包络元数据",
  "en": "Source Envelope",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "明确受权的完整 SourceVersion/2、容量和聚合有效性元数据投影；有效响应的窄 SourceVersionRef/1 仅选择真实当前观察，不授予 body 或写权限。",
  "exclusions": "不含body/其它Field/隐藏具体错误",
  "ownedNames": [
    "SourceEnvelope",
    "d7_source_envelope_read",
    "d7_source_envelope"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.source_envelope",
  "uiZh": "源包络元数据",
  "uiEn": "Source Envelope",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "正例：获得 source_envelope_state 后由真实 producer 读取完整源版本、聚合有效性与合格窄引用；反例：返回隐藏正文或字段错误，或将无效源的 Counter 补签为观察。",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Explicitly authorized complete SourceVersion/2, capacity and aggregate-validity metadata projection. A valid response's narrow SourceVersionRef/1 selects only an actual current observation and grants no body/write rights. No body, other Field values or hidden detailed errors. With source_envelope_state, the real producer returns complete version, aggregate validity and qualified narrow reference. Hidden text/Field errors or signing an invalid source's Counter into Observation is forbidden.

## Basis Projection / 量值基准投影

```json
{
  "conceptId": "weftext.term.basis-projection",
  "zh": "量值基准投影",
  "en": "Basis Projection",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "返回实际quantity或calendar值的完整已绑定基准描述供显式同基准判断",
  "exclusions": "label/magnitude不是单位证明；不加载新provider",
  "ownedNames": [
    "quantityBasis",
    "dateBasis"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.basis_projection",
  "uiZh": "量值基准投影",
  "uiEn": "Basis Projection",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "正例：比较两个 quantityBasis 的完整 unitId 与 dimension 后显式聚合；反例：依据同一 unitLabel 把 m 与 cm 当同单位相加。",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

The complete bound basis of an actual quantity/calendar value for explicit same-basis comparison. Label/magnitude is not unit proof; no new provider is loaded. Compare complete unitId/dimension from quantityBasis before aggregation; equal unitLabel cannot justify adding metres and centimetres as one unit.

## Fact Origin / 事实来源轨迹

```json
{
  "conceptId": "weftext.term.fact-origin",
  "zh": "事实来源轨迹",
  "en": "Fact Origin",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "Core内部逐值/item保留的asserted图事实来源证据",
  "exclusions": "不是公共作者provenance或implicit Action lineage",
  "ownedNames": [
    "FactOrigin",
    "D7.FactOrigin"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.fact_origin",
  "uiZh": "事实来源轨迹",
  "uiEn": "Fact Origin",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "正例：保留真实关系 Entry 的内部逐值证据后输出 asserted 边；反例：把任意 project 构造的端点标成 asserted，或把作者 provenance 当写权限。",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Core-internal per-value/item provenance proving asserted graph facts. Not public author provenance or implicit Action lineage. Retain a real relation Entry's internal evidence for an asserted edge; arbitrary projected endpoints cannot assert facts and author provenance is not write permission.

## Definition Transfer Effects / 定义转移效果

```json
{
  "conceptId": "weftext.term.definition-transfer-effects",
  "zh": "定义转移效果",
  "en": "Definition Transfer Effects",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "原同一decision中完整保存定义typed引用/Locator前后像证据",
  "exclusions": "不成为D3 node_link/citation slot或定义身份",
  "ownedNames": [
    "D7DefinitionTransferEffects",
    "d7_definition_transfer_effects"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.definition_transfer_effects",
  "uiZh": "定义转移效果",
  "uiEn": "Definition Transfer Effects",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "正例：同一原 decision 记录 typed Ref 与真实 Locator 的转移前后像；反例：重写普通 text 中像 UUID 的内容，或把 Q 定义当 node_link slot。",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Complete typed-reference/Locator before/after evidence for saved definitions in the same original decision. Not D3 node_link/citation slots or definition identity. Record real typed Ref/Locator transfers; do not rewrite UUID-looking plain text or treat Q definitions as node_link slots.

## Field Entry Image / 字段条目前后像

```json
{
  "conceptId": "weftext.term.field-entry-image",
  "zh": "字段条目前后像",
  "en": "Field Entry Image",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "按原作者序完整Field raw Entry的明确投影",
  "exclusions": "不是删掉秘密字段后的exact SourceSnapshot",
  "ownedNames": [
    "FieldEntryImage",
    "weftext.field-entries"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Types/functions use the exact ownedNames; namespace Weftext.Core.Query/View/Actions as applicable; no alternate public alias.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.field_entry_image",
  "uiZh": "字段条目前后像",
  "uiEn": "Field Entry Image",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "正例：按原作者序运输完整 people/phone Field 的 before/after Entry；反例：删掉其它字段后将投影冒充 exact SourceSnapshot。",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Explicit complete Field raw-Entry projection in original author order. Not an exact SourceSnapshot with secret Fields removed. Transport the whole people/phone Field before/after; deleting other Fields cannot make the projection an exact source.

## Derived Period Range / 派生周期区间

```json
{
  "conceptId": "weftext.term.derived-period-range",
  "zh": "派生周期区间",
  "en": "Derived Period Range",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "从完整已验证CalendarPeriod值产生civil日期边界或明确不可表示状态，并保留原period与seriesKey的只读投影。",
  "exclusions": "不新增作者range、Event、CalendarPeriod身份或通用CEL日期构造器。",
  "ownedNames": [
    "DerivedPeriodRange",
    "derived_period_range",
    "weftext.query.derived-period-range/1"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "D7 read source.kind=derived_period_range; DerivedPeriodRange<T> is an existing object/union type expansion, not a primitive.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "query.derived_period_range",
  "uiZh": "派生周期区间",
  "uiEn": "Derived Period Range",
  "allowedShort": "Use full name in controlled surfaces; no additional wire alias.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "2026-Q3得到2026-07-01至2026-10-01；9999年原值合法但endpoint_out_of_domain，禁止截为9999-12-31或丢行。",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Read-only projection from a fully validated CalendarPeriod to civil date boundaries or explicit unrepresentability, preserving original period and seriesKey. Adds no author range, Event, CalendarPeriod identity or generic CEL date constructor. 2026-Q3 yields 2026-07-01 through 2026-10-01. A legal year9999 period can be endpoint_out_of_domain; never clip to 9999-12-31 or drop its row.

## Conditional Source Change / 条件源修改预览

```json
{
  "conceptId": "weftext.term.conditional-source-change",
  "zh": "条件源修改预览",
  "en": "Conditional Source Change",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "仅完整预览中含 C 的既有 Node 的完整 Result/9 源变换；由原唯一 candidate map、完整字节与 source-state 比较及 owner 推导的 forceAdmission，唯一决定保留完整生产版本或消费当前生产域 H+1 的拟议版本。",
  "exclusions": "不是已发生修改、用户脚本、预先分配的content ID或committed EffectItem。",
  "ownedNames": [
    "ConditionalSourceChange",
    "conditional_source_change"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Type ConditionalSourceChange; wire EffectItem.kind=conditional_source_change; only the exact closed result/revisionRule from D7 Transport.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "action.conditional_source_change",
  "uiZh": "条件源修改预览",
  "uiEn": "Conditional Source Change",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "正例：同一生产域 H=7 时真正无改变保留原版本，真实字节或 source-state admission 使用版本8；反例：从 foreign revision 或历史 head 捐号，或让调用方选择 forceAdmission。",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Complete Result/9 transformation of an existing Node containing C, only in full preview. The original unique candidate map, full bytes/source-state comparison and owner-derived forceAdmission uniquely choose retention of full production version or the current production domain's proposed H+1. Not an actual change, user script, preallocated content ID or committed EffectItem. Within one production domain at H=7, true no-change retains its version and real byte/source-state admission uses8. Foreign revisions or historical heads cannot donate numbering; callers cannot choose forceAdmission.

## Effect Delivery Epoch / 效果交付世代

```json
{
  "conceptId": "weftext.term.effect-delivery-epoch",
  "zh": "效果交付世代",
  "en": "Effect Delivery Epoch",
  "owner": "D7",
  "layer": "query/view/action runtime",
  "definition": "一份完整语义清单的有限只读运输投影寿命；同epoch固定全部item/byte handles，重开可创建新epoch且不改变原decision。",
  "exclusions": "不是Workspace authority/auth世代、Query订阅epoch、原effectsToken或新的作者效果。",
  "ownedNames": [
    "EffectDeliveryEpoch"
  ],
  "manifest": "No executable manifest granted; D10 consumes the explicitly defined data contract only.",
  "codeConvention": "Internal type EffectDeliveryEpoch; no new public request member, command or token alias; current binding uses effects_cursor/effect_bytes/2 records, while real historical effect_bytes/1 keeps its original decoder.",
  "cli": "No CLI verb or flag frozen by this architecture; the future adapter must preserve the exact protocol and map labels through this concept.",
  "localeKey": "action.effect_delivery_epoch",
  "uiZh": "效果交付世代",
  "uiEn": "Effect Delivery Epoch",
  "allowedShort": "Query/View/Action/CEL/DAG only where unambiguous; no new wire aliases.",
  "rejectedAliases": [
    "unlisted controlled wire/type alias"
  ],
  "exampleAndCounterexample": "正例：新open重签发H2，原epoch内同cursor仍返回H1直到自身到期；反例：将旧cursor悄悄重绑到新handles或延长preview TTL。",
  "firstFreeze": "D7 revision06 candidate; independent acceptance pending",
  "migration": "Unpublished prototype has no compatibility obligation; replace affected controlled identifiers atomically with implementation. Historical evidence and user prose exempt.",
  "deletionTarget": "S1-S7 implementation inventory in D7 Impact; remove old exposed symbols/parser branches/fixtures/help/locales when the corresponding new contract ships."
}
```

Finite read-only delivery lifetime of a complete semantic manifest. One epoch fixes all item/byte handles; reopening may create another without changing the decision. Not Workspace authority/auth generation, Query subscription epoch, original effectsToken or new author effects. A new open may issue H2 while an old epoch's cursor still returns H1 until its own expiry. Never redirect an old cursor to new handles or extend preview TTL.

## Cross-stage bindings

```json
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
      "EffectsToken",
      "SourceObservation/1",
      "SourceVersionRef/1",
      "CommitDomain/2",
      "Frontier/2",
      "OwnerInputBinding/2"
    ],
    "rule": "Current SourceVersion/2 records stable production history; actual SourceObservation/1 and SourceVersionRef/1 separately bind current observer domain/epoch/file/pins and use-specific authorization. DependencyProof/2 and ObservationScope/2 retain all actual positive/negative proofs. No unsigned Counter, historical pin or conflict wrapper is an ordinary source observation; real historical decoders retain original recovery."
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
    "rule": "Original D3 wire12/Result9 and preparationBinding remain the sole identity submit. D3 typed resolution uses protected PreparedActionBinding/3 and the same planning CAS/P; native receipt covers native only, with mandatory public D3CanonicalEffects/1 for the named committed profile. No D6 planToken or new submit envelope is added."
  },
  {
    "owner": "D6",
    "names": [
      "SourceEnvelopeStateCapability",
      "source_envelope_state",
      "CommitSequenceStateCapability",
      "commit_sequence_state"
    ],
    "rule": "Current Policy/3 retains explicit, non-implied source_envelope_state and commit_sequence_state. SourceEnvelope/2 uses actual current-observation production; domainCommitSequence is scoped to the complete specified CommitDomain. Original policy/profile and saved recovery are never silently upgraded."
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
  },
  {
    "owner": "D7 current protected record and delivery",
    "names": [
      "PreparedActionBinding/3",
      "MinimumMapping/3",
      "D7DefinitionInput/2",
      "D7ProposedInput/2",
      "D7ResolutionAccess/1",
      "D7ProposedVersion/1",
      "EffectManifest/2",
      "EffectBytes/2",
      "FieldEntryImage/2",
      "D7DefinitionTransferEffects/2",
      "d7_action/2"
    ],
    "rule": "Complete runtime wire2 producers, typed D6 descriptor, one original D3/D6 request and mandatory full preview/committed semantics. MinimumMapping3 registryBinding is null iff no actual Registry use and registryInputs=[]; typed C/Q/Field/classification/relation use requires actual immutable contexts. No new ledger, ActionSpec arm or receipt. Historical record1/2 and transport1 recover original bytes/pins."
  },
  {
    "owner": "D3 resolution / D6 installation / D7 read-only effects",
    "names": [
      "D3ResolutionInput/1",
      "D3CanonicalEffectPlan/1",
      "D3CanonicalPlanProjection/1",
      "D3CanonicalEffects/1",
      "ConflictInstallInput/1",
      "D7CanonicalState/1",
      "D7RestoreMembershipImage/1",
      "canonical_plan",
      "conflict_branch_source",
      "conflict_resolution_change"
    ],
    "rule": "D3 uniquely owns typed resolution and canonical plans/extensions; D6 alone produces purpose-bound installation input, never current observation. D7 fully transports actual installed/selected/result bytes, metadata and restore membership. Preview has projection and bytes without committed extension; same-P committed requires public canonical extension plus native receipt/companion. Exact-overlap physical effects deduplicate once; unknown recovery does not reselect heads or H."
  },
  {
    "owner": "D8 edit consumer / D7 full transport",
    "names": [
      "PreparedEditBinding/2",
      "d8_edit_prepared/2"
    ],
    "rule": "Actual D8 protected binding embeds D6 PreparedIntent2 preview using Manifest2/EffectBytes2, protocolOwner=D6/profile=full; no D7 bindingToken, ActionSpec impersonation or ConflictInstallInput."
  },
  {
    "owner": "D9 template consumer / D7 preparation",
    "names": [
      "TemplateRecipe/2",
      "TemplateConstruct/2",
      "TemplateConstructionInput/2",
      "D9EntityVersionAddress/2"
    ],
    "rule": "D9 owns persistent fixed managed production addresses separately from current runtime SourceVersionRef and complete observation/pins. ConstructionInput2 enters PAB3 only through the built-in typed adapter. Omitted Annotation directory/state evidence does not require body sourceInputs. PL-IR-01 remains a precise unresolved Locator qualification gate."
  },
  {
    "owner": "D6 bootstrap / D7 display / D10 consumer",
    "names": [
      "WorkspaceBootstrapPlan/1",
      "WorkspaceBootstrapPlan/2",
      "WorkspaceBootstrapProfile/3",
      "Policy/3",
      "d6_workspace_bootstrap_plan1",
      "d6_workspace_bootstrap_plan2",
      "d7_symbolic_json2",
      "d7_planned_preview_open",
      "d7_planned_preview_opened"
    ],
    "rule": "D7 consumes actual complete D6 Plan2/Profile3/Policy3, while real already-issued families fixed to profile1/2 retain original Plan1 for their authorized replacement, uncommitted create/fork and recovery. Current symbolic-effect version2 payloadFormat is the closed Plan1|Plan2 encoding union selected by the protected family and actual decoder, not caller downgrade; no relabeling or Policy3 injection. D10 planned-preview2 restores original DecisionKey2 and actual Manifest2/EffectBytes2, not another producer or decision. Historical Plan1/symbolic1/Manifest1 real records retain exact decoding and custody; issuer A never replaces inactive target W/B domain. D7 Preview Transport section8 owns the exact wire2 D6/PAB3 planned-preview producer; D10 only consumes it and its complete original pinned semantics. D10AuthorPreparationLink is a separate atomic protected association, not a D7 member or second submit."
  }
]
```

These bindings preserve original D3/D4/D5/D6 ownership. D7 owns only its runtime selections, preparation records and read-only effects transport. Current /2,/3 and real historical /1 decoders/retention obligations are explicit and separate; equal-shaped tokens or Counters infer no cross-version or cross-replica qualification.
