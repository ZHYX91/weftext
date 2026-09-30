---
source_language: zh-CN
translation_status: source
---

[English](INTAKE.md)

# D6-FA-r01 产品输入登记

状态：candidate intake；仅登记已确认需求与候选研究输入，不激活功能，不代表插件实现已审计。

## 1. D8 Source / Live / Read 输入

已确认的编辑界面需求：

- Source：编辑 exact source。
- Live：由 Core 对当前 Draft 提供映射后的可编辑表示。
- Read：只读表示，可绑定明确 Draft 预览或 committed revision。
- Live 的源标记显示策略闭集：`always_show | contextual | always_hide`，中文分别为“始终显示 / 按需显示 / 始终隐藏”；默认 `contextual`。
- 标记显示策略属于 presentation preference，切换不得修改 source、不得提交作者变更、不得结束或确认 IME composition。
- Source↔Live↔Read 切换必须保持同一 Draft、Selection/anchor/focus/affinity、Undo/Redo 输入日志及 IME 连续性；映射无法证明时进入 Source 或明确 readonly/atomic 区，不猜 DOM offset。
- 外部文件变化使旧 map/preview/cut 失效时，当前 Draft 仍作为本地提案保留；重新绑定使用新 SourceVersion 与三方 proposal，不因渲染文字相同静默 rebase。
- 完整 D8 afterimage 后续编写；本批 D6 只冻结其所需的文件版本、可靠保存/冲突和 Server collaboration 输入。

## 2. 插件候选 intake

以下仅根据给定固定 README 引用登记候选语义方向；**本批没有读取七个插件源码、release 包或研究附件，也没有做实现审计**。README 不是 Weftext 能力证明。

| 候选 | intake 主题 | 预计 owner | 固定 README |
|---|---|---|---|
| Property Order | 属性重复值重排、候选顺序 | D4/D5/D8 | https://github.com/ZHYX91/%6Fbsidian-property-order/blob/137668dfc239bd7e0757032bf84093fab09ca6aa/README.md |
| Folder Nodes | 正文/子节点导航、资源视图、人工顺序 | D3/D5/D8 | https://github.com/ZHYX91/%6Fbsidian-folder-nodes/blob/27f6e0b6f24641eab0d23d2f1f255b133482213e/README.md |
| Chrono Notes | 周期/日期导航、日历任务投影 | D4/D7/D8 | https://github.com/ZHYX91/%6Fbsidian-chrono-notes/blob/0116aaa6397aa2b32bc57889b951c25114a3bbad/README.md |
| Link Integrity | 引用诊断/可靠性 | D3/D7/D8 | https://github.com/ZHYX91/%6Fbsidian-link-integrity/blob/902b881585c346e599656c3d5629017165c5da9c/README.md |
| Number Suite | 编号/题注/脚注/交叉引用、大纲/导出 | D2/D8/D9 | https://github.com/ZHYX91/%6Fbsidian-number-suite/blob/5ecdcf88040b5b5caaf7b2759410cf2d115fcfd8/README.md |
| Structural Tables | 复杂表格、行提升 Node 集合 | D2/D5/D8/D9 | https://github.com/ZHYX91/%6Fbsidian-structural-tables/blob/2cd5a1c05a22bce801fa59a56295ee97e68b66e7/README.md |
| Assistant Workflow | 快照、能力匹配、校对、进度、恢复工作流 | D8/D9/D10 | https://github.com/ZHYX91/%6Fbsidian-%64ocwen-assistant/blob/65481ec5771fc44ad87a5433b10804a555e50590/README.md |

## 3. 明确未批准项

本 intake 不批准：

- 新 Weftext 正文语法；
- 为 heading/table row/property occurrence 新增长期稳定片段身份；
- 每键 author commit / 自动提交；
- 绕过 Core 的插件直接文件写或数据库写；
- 把外部 runtime、转换程序、Provider 或 源应用 API 变成 Weftext 生产语义权威；
- 因 README 展示某平台而扩大 Weftext release/support 矩阵；
- 把候选插件的私有实现、历史 bug 或市场排名导入规范。

这些项目也不是永久禁止；以后若产品需要，必须由实际 owner 提案、版本化、完整权限/事务/失败合同和独立审查决定。

## 4. 与 D6-FA-r01 的直接关系

本次 D6 文件权威只接收以下跨域输入：

1. 外部文件变化必须能使旧 SourceVersion/map/preview 失效，并保留 Draft/冲突分支。
2. 人工顺序不能只存在可删 index；当前 parent/order 的逻辑 owner 必须可移植。
3. Snapshot/进度/恢复 UI 不能把 building index、prepared、preview、worker success、HTTP success 或同步 provider 状态显示成 author committed。
4. 大库初开应先允许活动文档和普通可靠保存，再渐进完成 metadata/search/OCR；完整 Action 仍要求完整范围。
5. Server 多用户必须允许并发会话；后台提交顺序不等于前台单用户。

其它插件语义留给后续真实 owner 后像。
