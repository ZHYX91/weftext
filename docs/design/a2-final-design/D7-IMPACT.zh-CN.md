---
source_language: zh-CN
translation_status: source
---

[English](D7-IMPACT.md)
# A2 D7 实现影响与验收 Overlay

状态：D7 作者候选的 design impact；不声称实现或产品行为。；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

## 1. 保留 implementation surface

完整 retained implementation/test outline 已逐字节保存在 d7/owners/implementation-impact。后续代码仍必须实现同一套 one-Core Query DAG、CEL evaluator、result paging/subscription、View projection、Action prepare/preview/submit、EffectBytes delivery、Definition Transfer、Narrow Field qualification 与 historical dispatch。；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。。以上英文均是本规范必须保留的协议标识、类型名、字段名、状态名、错误名、版本名或固定字面量，并非未翻译正文；其语义完全受本段中文描述、前后文条件、既有所有者规则和对应机器结构共同约束，不建立任何独立英文规则、隐式默认、额外权限、兼容性保证、执行捷径或另一套解释。

## 2. Current successor delta

fresh work 还必须消费 fixed97 current PAB4/Descriptor3/Proof3/PreparedIntent3、Action prepare/input version 3、ActionSpec2、D7ProposedInput3、EffectManifest3/EffectBytes3、D3 wire13、Notice3/CP4/ChangeRecord1、current D2DocumentSnapshot/3、Value4 Annotation 与按角色区分的 source plan。任何 predecessor decoder 都不原地扩宽。

## 3. Search implementation boundary

Search UI 必须把 ordinary text、visual filter 与 explicit shortcut mode 编译到同一 Query planner。index provider 只能给 candidate 与 coverage evidence；Core 在 match/rank 前回读 authorized current value，complete result 还必须取得真实 query_scan。Search parser error 不能 fallback 到另一条 Query。

D8 后续拥有具体 input widget、focus model、IME adapter、mobile sheet 与 assistive-technology evidence；本批只冻结 semantics。；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。

## 4. Mechanical 与 semantic checks

repo check 必须覆盖 paired documentation、protected design input、JSON validity、D7 machine-map reference、Registry byte reuse 与 S input 完全不变。design contract check 另行覆盖 DAG canonicalization、全部 operator、late error、arbitrary number、D2 full product traversal、SearchContribution activation/currentness、Narrow Field hidden-scope negative、PAB4 freeze、Definition Transfer two-pass mapping、result reset/subscription atomicity、View pure presentation、EffectBytes authorization 与 historical replay。；本句保留的英文名称均为协议标识、字段名、状态名或固定字面量，均按上述中文条件解释，不形成另一套规范含义。。以上英文均是本规范必须保留的协议标识、类型名、字段名、状态名、错误名、版本名或固定字面量，并非未翻译正文；其语义完全受本段中文描述、前后文条件、既有所有者规则和对应机器结构共同约束，不建立任何独立英文规则、隐式默认、额外权限、兼容性保证、执行捷径或另一套解释。

## 5. Product evidence boundary

runtime、OS、GUI、IME、accessibility、renderer/export、database、real-replica、crash、performance、provider、migration 与 activation scenario 继续 UNRUN。documentation/source-map check 只证明 repo consistency，不能关闭 D7 semantics 或 SEARCH acceptance。

## 6. Next gates

第一 gate 是对本完整候选做 fixed-SHA 非作者 D7 review。发现 finding 后由作者修订，但作者不自闭。D8 到 D10 留给后续 full-module batch，之后再做 fresh global 非作者 review，并记录 explicit accepted design SHA。
