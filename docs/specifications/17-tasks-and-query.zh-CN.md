---
source_language: zh-CN
translation_status: source
---

[English](17-tasks-and-query.md)

# Weftext 任务和 Query 视图

本规范定义规范的两级清单/任务节点模型及其 Query 视图。较早的任务元数据仅可通过显式、经审查的迁移接受；运行时源和 UI 只暴露规范模型。

## 范围和权威

本规范定义原生 AsciiDoc 清单出现、持久受管任务节点、清单到节点提升、它们的统一查询投影以及显式任务导入/转换边界。架构决策见 [`../architecture/18-task-nodes-and-checklist-promotion.md`](../architecture/18-task-nodes-and-checklist-promotion.zh-CN.md)。

精确 AsciiDoc 源和受管节点树仍是可移植权威。仅 Core 识别清单范围、验证任务节点属性、解析身份和依赖、生成查询计划/结果，并构建每一项经修订检查的编辑或工作区事务。Desktop、CLI、Server、WebUI、导出器和智能体不得独自重新发现任务、解析任务属性、修补源或写入文件。

不存在规范的尾随任务宏、任务伴随文件、任务清单、任务数据库、任意复选框状态词汇、`tasks` 代码围栏、JavaScript 求值器或客户端私有查询扩展。

## 原生待办项层

轻量任务是原生无序清单项：

```adoc
* [ ] Open
* [x] Closed
* [*] Also closed
```

`[ ]` 为未完成。`[x]` 和 `[*]` 为已完成。关闭未完成项时 Weftext 写入 `[x]`，并保留任一已接受的完成拼写，直到显式切换改变该精确标记。`%interactive` 只是 Asciidoctor 渲染选项；Weftext UI 操作仍是 Core 所有的源编辑。

清单出现没有持久任务身份。其精确操作目标为 `{ owningNodeId, documentRevision, sourceRange, parserOccurrence }`。查看、索引、查询或切换它绝不会写入 ID。移动或编辑周围源可能使该出现证据失效；调用方重新加载或报告过期目标，而不是猜测。

列表嵌套仅是作者大纲结构。它不暗示任务依赖、父任务、项目、继承的状态/日期/优先级、重复或任务节点身份。清单不能携带任务级注释、资源、依赖、日期、优先级或持久跨文档引用。需要其中任一种能力时即调用提升。

## 持久任务节点配置文件

持久任务是普通的活动受管节点。其任务身份是 `weftext.id` 中节点已有的小写 UUIDv4；身份不会重复到属性中。文档标题是任务标题，文档正文包含细节和验收标准，普通节点资源是任务资源，而 `weftext.annotations.json` 承载可移植评审。

AsciiDoc 文档头包含以下封闭配置文件：

```adoc
---
weftext:
  id: "550e8400-e29b-41d4-a716-446655440000"
---
= Publish Weftext 1.0
:weftext-task: v1
:weftext-task-state: in-progress
:weftext-task-priority: high
:weftext-task-scheduled: 2026-09-01
:weftext-task-due: 2026-09-05
:weftext-task-depends-on: 9b74c989-7bac-472f-9a8f-01f0db9f7a10
```

只有文档头中的文字属性条目是权威。正文重定义仍是处理器状态。值不会展开 AsciiDoc 属性、替换、环境变量、路径、URI、宏或可执行表达式。

| 源属性 | 解码字段 | 值和规则 |
| --- | --- | --- |
| `weftext-task` | `profile` | 必需的精确 `v1` |
| `weftext-task-state` | `state` | 必需为 `todo`、`in-progress`、`on-hold`、`completed` 或 `cancelled` |
| `weftext-task-priority` | `priority` | 可选 `highest`、`high`、`medium`、`normal`、`low` 或 `lowest`；缺失表示 `normal` |
| `weftext-task-created` | `created` | 可选 ISO 日期或显式偏移 RFC 3339 时刻 |
| `weftext-task-start` | `start` | 可选 ISO 日期或显式偏移 RFC 3339 时刻 |
| `weftext-task-scheduled` | `scheduled` | 可选 ISO 日期或显式偏移 RFC 3339 时刻 |
| `weftext-task-due` | `due` | 可选 ISO 日期或显式偏移 RFC 3339 时刻 |
| `weftext-task-closed` | `closed` | 可选；仅当状态为 `completed` 或 `cancelled` 时有效 |
| `weftext-task-depends-on` | `depends-on` | 可选，以恰好一个 ASCII 空格分隔的唯一任务节点 UUID |

[`../../schemas/task-node-v1.schema.json`](../../schemas/task-node-v1.schema.json) 冻结了解码字段形状。Core 还验证日历日期、显式偏移、重复属性、状态/关闭组合、UUID 唯一性、目标任务配置文件、自依赖、重复边以及授权和循环。

仅日期值仍是日历日期。时刻要求大写 `T`、可选小数秒，以及 `Z` 或数值 `+/-HH:MM` 偏移。自然语言日期仅是 UI 输入，必须预览为规范文字值。`blocked` 由已授权的未完成依赖派生；`overdue` 由 `due` 加上显式日历/偏移上下文派生。两者均不存为状态。重复字段和自动后继不属于任务节点 v1。

未知/重复的 `weftext-task-*` 属性、不支持的规范配置版本、缺失/无效状态、非法关闭数据、畸形日期、缺失/无效依赖目标或循环，都会使任务规范配置无效。底层受管节点仍按普通节点规则可见，而任务操作/投影报告精确诊断并安全拒绝。任意非保留头部属性仍是普通文字字符串属性，不会成为任务字段。

工作区根节点、保留的回收站节点、非受管内容和被忽略内容不能携带此配置文件。任务节点的目录父级和子节点仍仅为结构组织。子节点不会自动成为子任务或依赖。

## 任务节点身份和生命周期

重命名、移动、云副本、回收站和恢复都会保留任务身份，因为它们保留节点 UUID。复制遵循普通节点复制规则，并为复制的子树重新设键。任何已接受的内部复制节点链接或依赖边仅可通过经审查的 Core 复制映射重写；外部边保留其原 UUID 目标。重复节点 UUID 仍是工作区身份冲突。

任务节点状态是一个权威字段。`todo`、`in-progress` 和 `on-hold` 为未完成；`completed` 和 `cancelled` 为已完成。状态转换使用窄的、经修订检查的头部属性计划。关闭时可以添加明确提供的有效关闭日期/时刻；重新打开时移除 `weftext-task-closed`。Core 绝不会仅因已勾选清单被提升就捏造关闭时间。

任务节点依赖替换是在一个已授权的当前图上执行的完整集合工作区计划。每个目标必须唯一解析为活动、有效的任务节点。缺失和未授权目标保持非披露；自边、重复和循环在不写入的情况下失败。任务节点可以结构性移动而不改变依赖。

## 清单到节点提升

`Promote checklist to task node` 是独立的语义工作区操作。上下文菜单、可访问行操作、命令面板、CLI、Server 和获准的智能体提案都调用相同的 Core 规划器。

操作目标仅从一个精确清单出现捕获一次。规划是只读的，并记录：

- 源节点 UUID、文档/工作区修订、精确清单范围、列表深度、标记状态、主体文本，以及完整附接的续写/后代分支；
- 生成的节点 UUID、所选活动父级、可移植目录名、文档标题、初始任务属性、精确的新文档源以及每一个创建路径；
- 包含同一深度普通无序列表项及 `node:<uuid>[<label>]` 的精确替换源；
- 受影响的注释/链接/索引、授权、数量/字节数、精确的草稿敏感节点 ID 和有序的可恢复日志步骤。

默认目标父级是清单的所属节点。预览可以选择另一现有活动父级。它显示标题、路径/名称、初始状态、被提升的内容、链接标签、受影响注释和每一项冲突。Core 绝不会静默为已占用名称加后缀或创建猜测的父级。

`[ ]` 映射至 `todo`。`[x]` 和 `[*]` 映射至 `completed`，但不合成 `weftext-task-closed`。替换结果不是清单，且不能独立切换：

```adoc
* node:550e8400-e29b-41d4-a716-446655440000[Publish Weftext 1.0]
```

UUID 是链接权威；标签是作者编写的显示文本。Weftext 可以用当前任务呈现丰富已解析链接，但不会把状态或日期缓存到引用文档中。重命名不会静默重写作者标签。

续写块和后代列表内容属于提升作用域。Core 只能通过精确结果已预览的确定性、AsciiDoc 感知转换来提升它们。若缩进、续写附接、受保护内容或重叠注释无法在不含糊的情况下转换/重新锚定，规划就会失败，而非丢失或重复内容。

提交会重新检查源和所有重写节点的精确草稿登记表、两个修订、生成身份、目标占用以及源证据。一个可恢复工作区事务创建完整任务节点并替换清单分支。成功包含两者；回滚不包含任何一者。调用后的焦点或选择变化不能重新目标该计划。

未定义自动降级。任务节点可以拥有清单无法表示的正文、后代、资源、注释、依赖、权限或历史。

## 规范任务 Query 投影

节点、标题和 Template Root 域共享同一套规范 Query 能力。完整语法、`weftext.expr.v1` 类型系统、词法 `this`、显式时间上下文、作用域规则、限制和诊断，以 [`18-canonical-query-and-expression.md`](18-canonical-query-and-expression.zh-CN.md) 为准。本规范只定义任务行语义。

```adoc
.Due soon
[.weftext-query,version=1,view=task-list]
....
from tasks as task
scope subtree(this.node)
where task.closed = false
  and task.due is not null
  and task.due <= context.today + P14D
select task.id, task.title, task.state, task.due
order by task.due asc nulls last
limit 100
....
```

正文 `from tasks as task` 选择源语义；作者可以使用另一个合法的显式别名，并且每个字段仍有别名限定。`view=task-list` 仅用于呈现。该块不从 UI 焦点继承源、作用域、时间、节点、文档或标题。

任务别名是带标签联合。`kind` 是 `checklist` 或 `node`。待办项的 `id` 是 null，任务节点的则是节点 UUID。`owner_node` 是封闭的所属 Node 记录。`title` 是待办项正文，或任务节点由作者提供或系统派生的显示标题。`closed` 和 `state` 均为非 null。`checklist_depth` 仅属于待办项。优先级、`created`、`start`、`scheduled`、`due`、`closed_at` 及经权限过滤的 `blocked` 是可空任务节点字段，对待办项始终为 null。Query 结果绝不会为待办项捏造持久身份或任务节点字段值。

行携带未投影的操作证据。清单行保留所属节点 UUID/修订/精确解析器范围；任务节点行保留节点 UUID/修订和经过验证的配置文件修订。编辑一行会调用匹配的 Core 源/节点操作。查询结果、计数、分组、看板、日历和任务列表仍是可重建的派生视图。

授权会在表达式求值、计数、分组、排序、投影、限制、错误或建议之前，筛选任务候选项，并解析 `subtree(this.node)`、`descendants(this.node)` 或 `section(this.heading)`。前言或仅标题文档中的查询具有 null `this.heading`；`section(this.heading)` 返回精确的 `missing_heading_context`，而不是扩大作用域。`= Title` 仍是 Document Title，`==` 到 `==========` 仍分别为 H1 到 H9。

已取代的 Query 语法不是运行时别名。它仅由下文描述的私有一次性迁移转换器接受。

## 导入和迁移边界

Markdown 是显式导入器输入，而非规范解析器中的别名。基线语法和选定扩展仅通过有界、显式版本化的兼容配置文件识别。[`../../schemas/task-import-v1.schema.json`](../../schemas/task-import-v1.schema.json) 冻结所选配置文件及其设置证据；转换生成：

| 源概念 | 目标 |
| --- | --- |
| 普通未勾选/已勾选项目 | 原生 `* [ ]` / `* [x]` |
| 强类型生命周期、日期、优先级、依赖、持久 ID 或任务细节 | 新受管任务节点加原始位置 `node:` 链接 |
| 未完成/进行中/暂缓/已完成/已取消源状态 | 当需要多于简单开/闭含义时的任务节点 `weftext-task-state` |
| 先前任务 ID | 在计划/回执中记录的生成任务节点 UUID 映射 |
| 依赖 | 已解析任务节点 UUID 边 |
| 可安全转换的先前任务查询 | 规范 `.weftext-query` 块 |
| 重复、提醒、脚本、不支持/自定义语义 | 阻塞决定，直至存在已接受目标 |

导入规划共同清点选定文档，冻结源字节/摘要和设置，仅生成每个节点 UUID/路径一次，预览每一个新节点和源替换，验证名称/依赖/内容边界，且不写入任何内容。提交需要外部精确快照、同一经审查计划、可恢复工作区日志、回执和精确回滚。

已取代的尾随 `task:[...]` 元数据由同一显式迁移处理。[`../../schemas/task-metadata-v1.schema.json`](../../schemas/task-metadata-v1.schema.json) 定义已接受输入。其解码的 `depends-on` 值受 4,096 字节目标头部限制和最多 110 个令牌/唯一 UUID 约束；溢出是一个 `InvalidDependency`，且重复的无效、自身或重复令牌每类最多产生一个诊断。每个有效结构化出现都通过经审查的旧到新映射成为一个任务节点，该映射重写已接受的依赖和链接，替换精确源位置，并在没有无损目标决定时阻止重复或其他信息。运行时绝不把宏和任务节点视为两种规范形式。

使用已取代任务字段或语法的旧私有 Query 块，只能作为一次性迁移输入。只有在类型、null、已授权总体、作用域、排序和投影保持精确时，只读转换器才可把 `phase`/`resolution` 谓词映射到统一 `state`，把 `structured` 映射到 `kind`，并把旧所有者字段映射为规范 `owner_node` 记录。它会把已取代源属性、裸字段、裸 `today`、正文 `sort` 和旧作用域拼写重写为唯一规范语法。`recurring` 以及任何会在目标任务节点块中丢失含义的字段或谓词，都会阻止迁移。同一份经过审阅的工作区计划会一并变更任务出现项、依赖、链接和受影响 Query 块；运行时绝不保留两个解析器，也不静默求值不同总体。

## 迁移执行边界

封闭的 [`../../schemas/task-rebaseline-v1.schema.json`](../../schemas/task-rebaseline-v1.schema.json) 契约定义完整本地工作区迁移预览。不透明本地捕获绑定修订且经路径脱敏，但它既不证明身份也不授予 Owner 权限；ACL/Owner 授权先于规划。规划器清点活动宏输入，冻结文档/工作区修订和精确源预览，一次性分配新的任务节点 UUID/名称/路径映射，仅在完整映射已存在后映射已接受字段，并在不重新生成的情况下重新验证经审查 ID。`conversionReady` 意味着预览未发现强类型语义阻塞项；它仅由专用的经审查权威提交，绝非通用提交。

迁移事务权威绑定经审查摘要和完整值、一个工作区租约、精确根身份、旧到新映射、源/新节点字节、注释证据、活动草稿、外部快照和暂存恢复证据。提交重新检查当前 Owner 授权和草稿。恢复验证精确端点，要么完成经审查结果，要么恢复经审查前状态，并在不进行无关写入的情况下保留未知状态。

回滚权威仅从经审查的前向计划和精确已提交结果派生。它重新验证完整结果和原始快照，要求当前 Owner 审查和第二次确认，逆转源更改，将生成的任务节点树移动至日志暂存区，并在完成任一精确端点前验证每个恢复工件。它是恢复证据，而非持久产品回执。

不可信序列化预览在权威重新验证前使用 Core 的有界 JSON 解码器。直接 serde 反序列化仅是数据模型便利，不能建立字节上限、规范经审查文本、摘要完整性或当前工作区证据。

Core 提供精确的原生/结构化行内解析、任务 UUID 与依赖索引、强类型行内编辑、循环任务完成、依赖替换、可恢复任务事务、导入预览、CLI/Owner Server/本地桥接契约，以及共享的本地任务检查器。Query 调用方使用规范外层语法、显式任务别名、词法 `this`、显式 `context`、稳定输出名称和有边界的派生计划能力 `weftext.query-expression-subset.v0`。完整共享的 `weftext.expr.v1` 求值器、迁移提交权威、托管授权、持久回执、批注迁移、调用方集成、无障碍/IME 和大工作区操作，必须在公开前满足已定义契约。任何产品端都不得在写入任务节点后继续写入已取代元数据。

## 验收边界

可移植夹具覆盖清单识别/切换、受保护上下文、混合行尾、CJK/RTL/emoji、每个任务节点字段、无效组合、重复/未知属性、日期/偏移、依赖失败/循环、内容边界、复制/重新设键、回收站/恢复、同步冲突和权限过滤。

提升夹具覆盖叶和嵌套清单分支、续写、生成的名称/UUID、替代父级、冲突、精确源替换、节点链接标签转义、注释重新锚定/阻止、源/目标过期、未保存草稿、每个崩溃边界、已验证回滚以及不存在残留的复选框镜像。

Query 夹具覆盖带标签行联合、`task.title`、可空性、显式别名、词法节点/文档/标题/Query 上下文、只有标题或位于前言时缺少正文标题、强类型作用域、`context.today + P14D`、分组、稳定排序、投影、限制、信息不披露、畸形输入和资源上限。跨产品端验收证明 Desktop、本地 WebUI、托管 WebUI、CLI、Server、导入/导出、备份/恢复、同步和获准智能体操作使用一致的 Core 计划与结果。迁移验收证明宏到节点和旧 Query 的预览、提交、重启恢复、回执与精确回滚都由工作区外完整备份保护，并且运行时不存在双实现。
