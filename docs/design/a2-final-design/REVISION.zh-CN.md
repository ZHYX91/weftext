---
source_language: zh-CN
translation_status: source
---

[English](REVISION.md)

# 作者源与使用流程修订入口

状态：设计候选；本轮原生属性与链接增量在 `e703f9e7bb9908d8b6a322620b59b184d0f7bb5f` 完成独立有界复核，原全局审查仍未完成，不构成全局接受或实施启动。修订起点为 `6b13b03b8da3e9e3d14b8663e91442568c8917e0`，作者分支 `design/yaml-properties-ux-20261011`。新规范由 [AUTHORING](AUTHORING.zh-CN.md) 、[作者源集成](AUTHORING-INTEGRATION.zh-CN.md) 、[EXPERIENCE](EXPERIENCE.zh-CN.md) 和 [LINKS](LINKS.zh-CN.md) 自包含定义。其它模块只引用这些唯一 owner，不复制第二份配置/语法。

## 1. 本次修改

- 取消新笔记的 YAML front matter；ID、类型统一为原生 `weftext-id` 与 `weftext-types`。任务短名为 `task`，逗号列表只定义值协议，内部仍展开为原 facet。
- 简单业务值使用原生 AsciiDoc 属性，允许标题前后位置；类型由 schema 定义。复杂事实仍保留完整能力，显式转换不损失来源/备注/重复值。
- 协调 ID 分配、复制、恢复、完整文件坐标、Registry 名称映射、Field 读写与格式绑定，不只修改示例。
- 纳入写作阅读、快速记录、任务/Query 多视图、批注引用/潜在链接、模板到内网下载五条完整流程；其它研究候选逐项处置。
- 增加稳定 ID、源相对、库根完整、本机绝对四种链接形式及右键转换/复制。最短路径不做全库猜名；ID+anchor 与通用离线路由仍明确后置。

本增量以已完成前稿 `35cf55f95da45ccab910adb85c2e7276197b47f6` 为修改前像；直接替换未部署 managed2 的源映射，不制造新 runtime 历史代际。

## 2. 具名替代与保留

仅在本修订候选内，以下精确关切由新条款替代；未列出的原义务继续成立。旧固定来源与原报告不改写，读完/有界接受不被冒充为对新设计的接受。

| 原 owner / 关切 | 新 owner | 处置 |
| --- | --- | --- |
| D2 / root `wf-kind`、`wf-facets` 控制及禁止 YAML 的前代输入 | AUTHORING §§1–2 | 原生保留控制声明取代前稿 YAML 和旧控制；完整来源资格，不恢复旧 AsciiDoc 缩水规则。 |
| D3 / Node 身份的源携带与 file binding | AUTHORING §3 | ID 声明内嵌；原 WorkspaceId/NodeId、birth、不复用、collision、lifecycle/授权仍保留。 |
| D4 §2.2 / RegistrySnapshot1 闭合形状 | AUTHORING §4 | Snapshot2 增加完整 authoringBindings；原 Snapshot1 decoder 不扩宽，catalog 原字节不改。 |
| D4 §16.1 / 所有 Field 必须 Entry JSON、普通属性只可经 MapAttribute 导入 | AUTHORING §§4–6 | 已绑定单值原生属性可直接成为 Field；复杂 Entry1 保留；同 Field 双载体冲突而非两份真相。 |
| D6 §1、§2 / 文档身份声明物理位置 | AUTHORING §3 | 原生 ID 声明与原 metadata birth/file binding 同决议一致；不将结构/权限写入源属性。 |
| fixed97 SPEC §5 / 唯一 managed1 格式 | AUTHORING §7 | 新 Profile2/Binding2 的精确范围；原记录保持原版本；源变化不得冒充 profile-only 变更。 |
| D7/D8/D9 / 只识别 Entry 的 Field materializer、copy/template source | AUTHORING §§5–7 | 必须区分 header/Entry 原来源，完整预览/物化和原授权；不制造 JSON 原文。 |
| D10 / 向 D4 提供 RegistrySnapshot1 的 fresh 握手 | AUTHORING §4 | 新源配置提供完整 Snapshot2，完整 digest/owner 验证；真实旧计划保持原快照。 |
| D8 / 基本呈现与现有动作入口 | EXPERIENCE §§1–6 | 明确五流程、可选潜在提示与访问性，不新增写入权威。 |

链接规范 [LINKS](LINKS.zh-CN.md) §§1–7 具名协调 D2 的 nativeTarget/adapter、D3 的路径与引用身份边界、D8 的完整 Source 转换与剪贴板、D9 的 copy/template 重定位及准确导出能力；现有闭合形状不暗增成员。

机器入口 [AUTHORING-REVISION.json](AUTHORING-REVISION.json) 对应这份处置。前代 A2 source maps 仍定位其固定原版本及未变义务；遇本表明确改变的语义必须沿新 owner，不把旧 map 的 retain 解释为恢复被替代的限制。本次不为全部旧行重新复制一份来源目录。

## 3. 验证和未解决范围

先审 [AUTHORING §8](AUTHORING.zh-CN.md#8-验收要求) 的 SRC-01–12、[EXPERIENCE §8](EXPERIENCE.zh-CN.md#8-设计验收) 的 UX-01–18 和 [集成 §7](AUTHORING-INTEGRATION.zh-CN.md#7-集成验收与剩余边界) 的 INT-01–06，以及 [LINKS §7](LINKS.zh-CN.md#7-设计验收) 的 LINK-01–11，检查每条正常、错误、撤权/并发/恢复及真实原 source/producer/consumer。它们是设计案例，产品未运行；文档检查或 reviewer 数量不证明实现可用。

原固定 6b13 全局审查的 6 项 P1 与 15 项 P2 继续未关闭。这些涉及符号 Document 输入、独立 presentation policy 生产、原生 heading/block 产品域、Trash/保存搜索、版本/来源/解码、批注时间域/隐藏摘要及效果排序。本轮新交互不自动修复它们；确有影响的修订须单独给出完整新反例与关闭证据。原未完成 D7–D10、全局与 Mandatory/来源阅读范围也不自动获得完成信用。

固定 S 的 49 份 snapshots 和 `docs/design/inputs.json` 不得改变。中文/英文同步；无产品实现、运行时 migration、合并或部署声明。新独立审查先读修改及受影响依赖，再讨论分歧与修正；所有局部通过仍不能替代最终完整全局终审。
