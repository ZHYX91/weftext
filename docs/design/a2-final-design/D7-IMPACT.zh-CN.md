---
source_language: zh-CN
translation_status: source
---

[English](D7-IMPACT.md)
# A2 D7 实现影响与验收补充

状态：这是 D7 作者候选的实现影响说明；不声称任何实现或产品行为已经完成。

## 1. 保留的实现范围

完整的既有实现与测试大纲已经逐字节保存在 d7/owners/implementation-impact。后续代码仍必须实现同一套 Core Query DAG、CEL 求值器、结果分页与订阅、View 投影、Action 的准备/预览/提交、EffectBytes 交付、Definition Transfer、Narrow Field 资格以及历史版本分派。

## 2. 当前 successor 的增量

新的未见请求还必须消费 fixed97 的当前 PAB4、Descriptor3、Proof3、PreparedIntent3、Action prepare/input version 3、ActionSpec2、D7ProposedInput3、EffectManifest3/EffectBytes3、D3 wire13、Notice3/CP4/ChangeRecord1、当前 D2DocumentSnapshot/3、Value4 Annotation，以及按真实角色区分的 source plan。任何旧版 decoder 都不能为了 current 数据而在原标签下被扩宽。

## 3. 搜索实现边界

搜索界面必须把普通文字、可视化筛选和显式快捷模式编译到同一个 Query planner。索引提供方只能提供候选项与覆盖证据；Core 在匹配和排名前必须重新读取已经获权的当前值，完整结果还必须取得真实的 query_scan。搜索解析错误不得退回到另一条 Query。

具体输入控件、焦点模型、IME 适配器、移动端面板以及辅助技术证据归后续 D8；本批只冻结语义。

## 4. 机械检查与语义检查

仓库级检查必须覆盖中英配对文档、受保护设计输入、JSON 有效性、D7 machine map 引用、Registry blob 复用以及固定 S 输入完全不变。设计级规范检查还应分别覆盖 DAG 规范化、全部操作符、迟发错误、任意精度数值、D2 完整产品树遍历、SearchContribution 的激活与当前性、Narrow Field 对隐藏范围的负例、PAB4 冻结、Definition Transfer 两遍映射、结果重置与订阅原子替换、View 的纯呈现、EffectBytes 授权以及历史重放。

## 5. 产品证据边界

runtime、OS、GUI、IME、辅助技术、renderer/export、database、真实 replica、crash、performance、provider、migration 与 activation 场景继续记为 UNRUN。文档和 source map 检查只能证明仓库一致性，不能关闭 D7 语义或 SEARCH 验收义务。

## 6. 后续门槛

第一道门槛是对这份完整候选进行固定 SHA 的非作者 D7 复核。若发现问题，由作者修订，但作者不能自行关闭 finding。D8–D10 继续作为后续完整模块批次；完成后还需要一次全新的全局非作者复核，并记录明确的 accepted design SHA。
