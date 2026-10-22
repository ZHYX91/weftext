# Weftext 结构化数据与 Query

该 crate 接收 Core 提供的冻结源码、中央 schema 和库存数据，进行纯读取、校验与源码变换；它不打开工作区文件，不分配身份，也不提交保存。

`Schema::from_author_json` 安装中央配置，拒绝重复 JSON 键、未知配置成员、重复身份、循环定义、错误 ABI、错误选择器与参数。业务类型与字段只来自配置；`Schema::bundled` 使用当前设计中的完整 authoring fixture，是否安装仍由 Core 决定。

`project` 与 `patch_field` 使用官方 Typst AST 和 Engine 的标准绑定证明。未知成员保留原值；计算字段不可通过表单覆盖。窄写保留未改动的字节、注释、字面量拼写与行尾，提交前还需由 Core 重新验证当前 basis、schema 与权限。源码偏移是 UTF-8 字节；浏览器适配层必须转换为 UTF-16。

有序列表编辑在移动、新增、删除与嵌套修改时复用已证明的作者表达式。已知记录成员可增删，union 可切换分支，同时不重写保留的未知成员。任意记录键重排需要源码编辑器。Native 源码和值处理限制为 8 MiB、100,000 个值节点、深度 64；数组变换至多进行一百万次候选比较。超限明确返回 unavailable 或错误。`search_terms` 读取已安装的永久名称／别名 FieldId，并将词典覆盖度与词条分开返回。

Query 使用实际 `cel` 0.15.0 runtime。类型检查拒绝隐式数值、日期与 instant 转换；i64、原生 decimal、扩展精确小数和 RFC3339 小数秒保持精度。CEL 的逻辑运算、集合宏和条件分支直接由该 runtime 执行。宿主预算在执行前独立检查，不能通过 `|| true` 吸收。

Query 的输入、覆盖范围与新鲜度由 Core 捕获和发布。crate 不把逐文件观察提升为完整快照，不因显示限制改变统计，也不从聚合行继承写权限。字段、关系、资源和计算产品仍需要 Core 的实际冻结依赖；应用使用同一保存入口应用返回的补丁。

高级 Query 支持同一 cut 下的保存定义调用、精确 current／pinned 定义、typed 参数、有序 union、distinct、unnest、related 遍历与 terminal 聚合。Temporal provider 根据已安装的 calendar／zone 版本执行 Gregorian 与有限表日历 Period、精确时区转换和有预算的 recurrence 展开。缺失历史、provider 覆盖、目标闭包或宿主预算仍是 partial／unavailable，不伪装成完整空结果。

产品错误的解释使用唯一消息目录生成的 `UiMessage` 与具名参数，原始 JSON／CEL／编译器诊断另有来源标记；不根据错误文字或消息键推断 invalid、unavailable、coverage 或预算状态。`Cell::Unavailable` 分开携带机器事实 `kind`、类型化摘要 `reason` 和完整并列 `diagnostics`；并列诊断不挤入有界因果链。`TypedRefClosure.completeWithoutUnboundRootFields` 是生产者独立计算的准确证明，只排除根级未知字段，已知字段的无效、计算或未绑定成员仍使该证明失败。

`expandRecurrence` 的自产 `diagnostics` 在返回的原生值／CEL ABI 中是规范结构记录，包含消息键、参数、原始诊断及因果链，不再是界面文字列表。界面语言仅用于呈现，不改变 CEL 结果、状态、发生坐标、预算或作者事件字段。每次 CEL 求值的宿主错误通道独立，保留真实类型和参数；超过 128 项或 8 MiB 诊断预算会明确拒绝该次派生结果，保留输入和已收集诊断，不能被 `|| true` 吸收。

`parse_unique_json` 是 Schema／Export／Knowledge 共用的严格 JSON 入口：重复键使用产品类型化事实，真实 serde 语法诊断另存原文。调用方负责字节／成员预算和原始输入。Query 的 `unionAll` 区分未声明生产者与显式空列表，后者由类型化预检拒绝。

相关规范：[SOURCE](../../docs/design/SOURCE.zh-CN.md)、[FEATURES](../../docs/design/FEATURES.zh-CN.md)。

验证入口：

```text
cargo test -p weftext-data
cargo clippy -p weftext-data --all-targets -- -D warnings
```

测试实际调用官方 Typst 编译器与 CEL runtime，覆盖标准绑定反例、精确数值、同项条件、顺序与重复项、错误吸收、预算、未知成员保留、窄写、周期边界及冻结结果。测试通过不等于所有平台或完整产品已经验收。
