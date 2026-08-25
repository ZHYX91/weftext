---
source_language: zh-CN
translation_status: source
---
[English](02-node-storage.md)

# Weftext 节点存储

状态：已接受的规范存储格式。

根目录必须包含一个内容精确为 `weftext.asciidoc.v1\n` 的 `.weftext-format`，该标记选择完整工作区中的 `X/X.adoc`。标记缺失或未知时，系统必须安全失败或进入明确的导入/接管流程，绝不能选择 Markdown。参见 [`15-weftext-asciidoc-profile.md`](15-weftext-asciidoc-profile.zh-CN.md) 和 [`../architecture/14-canonical-document-metadata-and-review.md`](../architecture/14-canonical-document-metadata-and-review.zh-CN.md)。

## 内容类别

受管节点是一个目录，其中恰好包含一个与目录同名的 AsciiDoc 节点文档：

```text
Project/
├─ Project.adoc
├─ image.png
└─ Child/
   └─ Child.adoc
```

工作区根始终是受管节点。受管节点使用小写 UUIDv4 身份，并遵守下文的修订版、父级、排序和窄范围补丁规则。

工作区存储包含四类由 Core 管理的内容：

- 受管节点：上述 `X/X.adoc` 结构，具有 `weftext.id` 和节点行为；
- 可见非受管内容：由 `.weftext-rules` 明确分类的普通目录或文件，包括非受管 Markdown；它们通过文件/资源界面显示，但不作为节点处理；
- 忽略内容：从产品清单、资源浏览、搜索、派生索引和 watcher 结果中排除的物理工作区字节；
- Trash 项权威：保留的 `.weftext-trash/_weftext.items` 存储、清单和 payload；它们不参与普通发现，但完整参与修订版、同步、备份、冲突检测和迁移。

非受管项目不获得 UUID 或 Weftext 元数据封装。非受管 Markdown 保持精确字节，除非经过明确导入器，否则绝不作为节点文档解析或重写。非受管目录构成完整的子树边界：Core 可以为文件视图枚举其中的可见文件，但不会在其下重新发现同名 AsciiDoc/UUID 对并将其作为受管节点。受管节点的规范同名 AsciiDoc 文档由受管节点项目表示，不会再次作为普通文件出现。受管目录中的其他普通文件，无论扩展名为何，都是该节点拥有的资源，包括 `.md`、`.txt`、图片、PDF 和办公文件。节点拥有的 `.md` 资源不解析为节点源文，也不贡献标题、任务、链接、属性、批注或 Query 行；它可以作为附件打开/下载，也可以明确导入为新的规范节点。只有明确的非受管规则才能移除所有者身份并把文件分类为非受管 Markdown。保留的伴随文件、节点本地存储和事务证据绝不回退为资源。即使出现位置无效，受角色约束的 `weftext.template.json` 以及表格约束的 `weftext.table.json`/`weftext.records` 名称仍然保留。

当不存在 `.weftext-rules` 时，每个可到达的内容目录都必须是受管节点，每个普通文件都属于其所在受管节点并作为资源。添加规则文件后，工作区才进入明确的非受管/忽略边界语义。若工作区已经包含这些目录，删除规则文件会产生清单诊断；任何字节都不得因此改变。

## 可移植内容规则

可选的 UTF-8 根文件 `.weftext-rules` 是可移植工作区权威。它参与工作区修订版，会随工作区同步和备份，绝不会复制到节点元数据中。Weftext 不读取、翻译或继承 `.gitignore`；VCS 配置属于设备/工具策略，不是 Weftext 产品权威。

去除空白后的整行以 `#` 开头的空行和整行注释会被忽略。不支持行内注释：模式中的字面井号必须转义。第一个非空且非注释行必须完全匹配以下内容，且不得有首尾空白：

```text
weftext-content-rules-v1
```

其后的每个非空且非注释行都必须是以下形式之一，同样不得有前导空白，操作与模式之间必须恰好有一个未转义的分隔空格：

```text
unmanaged path/pattern
ignore path/pattern
```

规则按作者顺序求值，最后一个匹配规则决定该项目的操作。默认解释是严格的受管/资源模式。所有平台都区分大小写。路径相对于工作区根，只包含由 `/` 分隔的 UTF-8 组件，且不得以 `/`、`\`、Windows 驱动器前缀、`.` 或 `..` 组件开头。`*` 在一个组件内匹配零个或多个标量，`?` 恰好匹配一个标量，`**` 只有作为完整组件时有效，并匹配零个或多个完整组件。末尾 `/` 将模式限制为目录。`\ `、`\#`、`\\`、`\*` 和 `\?` 分别编码字面空格、井号、反斜杠、星号和问号；其他转义、未转义的模式空格/井号、空组件、NUL、格式错误的 UTF-8、未知操作、缺少/未知头部、超过 4096 字节的行以及超过 1 MiB 的文件均无效。

如果 `unmanaged` 或 `ignore` 规则匹配一个目录，该目录立即成为递归边界。子孙规则不能用来重新进入受管扫描。后续规则只能覆盖 Core 在目录边界之前实际到达的项目。对 `.weftext-format` 或 `.weftext-rules` 分类、对工作区根分类、对 `.weftext-trash/_weftext.items` 或其下内容分类、对有效 `weftext.records` 存储或其后代分类，或者只对受管节点规范 AsciiDoc 文档分类而不对完整节点目录分类，都会构成边界冲突。

任何无效或冲突的规则都会使清单无效。Core 报告 `InvalidContentRules` 或 `CanonicalDocumentBoundary`，并在扩大发现范围前停止；绝不静默忽略无效行。权威文件名保留给选定根；另一个可到达的 `.weftext-rules` 会产生多权威诊断，而不是形成嵌套覆盖。Core 在信任分类前检查符号链接和 Windows junction/reparse point，绝不跟随它们，即使其词法路径看似被忽略。规范/解析后包含检查、相对组件验证和事务时重新检查，可阻止绝对路径、`..`、分隔符、链接、reparse 或嵌套权威别名越过选定根。

该格式没有通用基础设施目录、工作区级清单或中央身份注册表。唯一的工作区级可移植基础设施子树是下文定义的封闭 Trash 项存储。版本化角色可以拥有名称严格限定的节点本地伴随文件/存储（例如下述表格 profile），但这不授权通用元数据目录或跨节点注册表。`.weftext-format` 选择 profile，`.weftext-rules` 仅包含分类。保留的 VCS 目录、`weftext.annotations.json`、表格伴随文件/记录存储、Trash 清单/payload 和事务证据都不是普通资源。应用缓存和派生索引位于工作区之外。

保留命名按角色约束。根控制使用 `.weftext-*`；保留存储中的封闭子项可以使用 `_weftext.*`；可移植的节点本地伴随文件使用不带前导点或下划线的 `weftext.*`；保留的类型化文档头属性使用 `weftext-*`。不存在“每个 Weftext 拥有的 JSON 文件或目录都必须以 `_` 开头”的规则。参见 [`../architecture/14-canonical-document-metadata-and-review.md`](../architecture/14-canonical-document-metadata-and-review.zh-CN.md)。

## 多维表格节点权威

多维表格仍是一个普通受管节点，拥有其正常的节点 UUID、规范 AsciiDoc 文档、资源、子节点和节点级权限。相邻的 `weftext.table.json` 伴随文件使用 `weftext.table/v1` profile，声明字段 schema 和可移植共享视图。同级保留存储 `weftext.records` 为每个同质记录包含一个 `weftext.table-record/v1` JSON 文件，并按小写 UUID 文件名的前两个十六进制字符分片。

记录 UUID 只在所属表格节点 UUID 的限定下寻址，是表格本地身份。它不是 `weftext.id`，不获得节点文档、元数据封装、子节点、独立 ACL、批注伴随文件或提升路径。节点关系和记录关系使用类型化值；被引用的图片/文件仍是普通节点拥有的资源，位于保留存储之外。公式/汇总值和索引都是派生数据，绝不能作为竞争性的记录权威存储。

表格伴随文件与记录存储构成不可分割的角色关系。profile 无效/缺失、文件类型错误、文件名/分片/解码 ID 不一致、重复 ID 或 JSON 键、未知 schema/值字段、部分同步、冲突副本名称或链接/reparse point，都会产生表格诊断，绝不扩大资源或节点发现。内容规则不能进入该存储。普通行删除是进入表格 Deleted Records 界面的记录状态事务；删除表格节点使用普通 Workspace Trash，并保留完整表格分支。复制会重新生成目标节点及每条记录的身份；移动、Trash 和恢复保留身份。精确的封闭 schema、类型、限制、事务、动态视图分离、备份和验收契约见 [`20-multidimensional-tables-and-editor-surfaces.md`](20-multidimensional-tables-and-editor-surfaces.zh-CN.md)。

## 工作区 Trash 项权威

Workspace Trash 是位于 `.weftext-trash` 的隐藏 Core 专用特殊节点。删除的对象不会被重命名到其受管子节点层级中。每个可独立恢复的对象占用一个禁止覆盖的项目目录：

```text
.weftext-trash/_weftext.items/<trashItemId>/
├─ _weftext.trash-item.json
└─ payload/<original node directory or original resource filename>
```

`trashItemId` 是在带修订版绑定的计划中生成并检查占用情况的临时小写 UUIDv4，与所有永久节点 UUID 不同。清单文件名必须是 `_weftext.trash-item.json`，封闭 schema 是 `weftext.trash-item/v1`，其 ID 必须等于目录 basename。清单还记录一个 `operationId`、`kind`、时间戳、来源状态、精确身份/来源字段，以及 payload 长度/摘要证据。未知字段、重复 JSON 键、ID 不匹配、精确或折叠大小写后的项目冲突、缺少/多出的 payload 项、冲突副本名称、摘要被修改、链接/reparse point，或活动与已 Trash 节点 UUID 重复，都会成为协调诊断。它们绝不授权覆盖、静默加后缀、重新生成身份或清理。

删除一个节点会产生一个 `node` 项，其中包含完整的原始同名目录及其所有后代、规范 `X/X.adoc` 文件、永久节点 UUID、批注和拥有的资源，且不重命名。后代不会获得清单。节点清单记录 `nodeId`、`originalParentNodeId`、`originalName`、可选且不具权威性的 `ancestorNodeIds`，以及规范聚合清单/摘要。删除多个彼此独立的节点拥有资源文件时，每个普通文件产生一个 `resource` 项，保留原文件名并记录 `originalOwnerNodeId`、`originalName`、字节长度和 SHA-256；同一批次的所有项目共享一个 `operationId`。资源不获得永久 UUID。

非受管或忽略项目不能通过任一 API 进入 Trash。包含此类边界的受管子树整体被拒绝。规范文档、批注伴随文件、事务证据和任何保留 Trash 路径都不是可独立删除的资源。有效项目 payload 中保留的节点 UUID 属于非活动权威：普通节点扫描、导航、链接、搜索、Chrono、任务和 Query 不进入存储，但修订版、同步、备份、验证和迁移会包含每个项目目录及其每个字节。

恢复依据永久 UUID，而不是历史路径或名称来确定身份。只有记录的父级/所有者 UUID 仍活动且精确及折叠大小写后的目标都空闲时，才可恢复到原位置。如果来源本身是完整节点项目，Core 可以在选定项目之前预览一次原子父链恢复。如果来源缺失，或旧项目的 `originStatus` 为 `unknown`，项目会留在 Trash 中，直到用户明确选择现有目标；Core 绝不伪造同名父级。冲突要求明确的替代目标或可移植重命名，并始终禁止覆盖。永久删除是独立的高权限、摘要绑定且明确确认的事务。Trash 不是备份。

完整的清单形状、payload 摘要、事务、同步、迁移和调用方规则，以 [`../architecture/17-workspace-trash-item-store.md`](../architecture/17-workspace-trash-item-store.zh-CN.md) 和 [`13-workspace-transactions.md`](13-workspace-transactions.zh-CN.md) 为权威。

## 身份

每个节点文档都包含一个小写 UUIDv4：

```yaml
---
weftext:
  id: "550e8400-e29b-41d4-a716-446655440000"
---
```

frontmatter 必须恰好包含一个顶层 `weftext` mapping。`id` 是持久身份。路径是当前定位器；父级和名称由目录树派生。路径、父级、名称、节点类型、子路径、文件系统时间戳和派生状态绝不能持久化到元数据封装中。

移动和重命名保留 ID。Weftext 复制会为复制的子树重新生成身份。云副本保留 ID。删除后在同一路径新建会产生新 ID。同一工作区内出现重复 ID 是身份冲突。

根节点 ID 标识初始工作区 lineage。完整根的分叉与复制是明确的操作；原始文件系统复制在用户选择前会被报告为未解决的副本/分叉状态。

## 排序

没有配置时使用升序自然名称排序。父节点控制其直接子节点的排序方式：

```yaml
weftext:
  child_sort: manual
```

或：

```yaml
weftext:
  child_sort: name
  child_sort_direction: ascending
```

子节点只存储其稀疏排名：

```yaml
weftext:
  sibling_rank: 2048
```

排名通常是以 1024 为间隔的正整数，只有当实际父节点使用 manual 模式时才有意义。缺少排名的项目排在已有排名的子节点之后；排名相同则使用规范化 basename，再使用路径排序。切换到 manual 会物化稀疏排名；name 模式忽略休眠排名。

## 源文权威

文件系统是结构权威。`weftext` 元数据封装是身份、导航图标、别名、最小排序策略和仅根节点的模板库选择权威。可重建的本地索引把 ID 映射到当前路径。不存在通用节点 JSON 清单，也不存在中央身份注册表。唯一固定的 `weftext.template.json` 契约只在派生的 Template Root 上有效，不能推广到普通节点。

元数据编辑只能补丁化所需的 YAML 范围；遇到含义不明确或不支持的 frontmatter 必须安全失败。整段 frontmatter 重新序列化不是可接受的普通编辑路径。

## 可引用节点

书目引用仍是普通受管节点，不获得节点类型、清单或并行数据库。顶层 YAML `reference` mapping 不属于目标元数据封装。类型化 Citation Data 为引用创建/编辑提供结构化的作者书目事实。`weftext.id` 仍是稳定的节点/引用身份；可变的 citation key 绝不能替代它。参见 [`16-citations-and-bibliography.md`](16-citations-and-bibliography.zh-CN.md)。

## 任务源文权威

简单任务是规范文档正文中的原生待办项出现位置。它们只能由所属节点 UUID、精确文档修订、精确源文本范围和解析器确认的出现位置标识；查看、索引或切换任务绝不写入身份或类型化元数据。

持久任务是普通受管节点，在字面 AsciiDoc 文档头属性中携带封闭的 `weftext-task` v1 profile。它已有的 `weftext.id` 就是任务身份。它不在 YAML 元数据封装中获得节点类型、任务清单、任务伴随文件、独立任务 UUID 或数据库行。需要日期、优先级、依赖关系、正文/资源、任务级批注或其他持久行为的清单，必须通过可恢复工作区事务显式提升；其原始清单位置变成稳定的 `node:` 链接，不保留复选框镜像。

任务索引、Query 结果、看板、日历、依赖图和任务计数都是可重建的派生数据。原生待办项切换会精确修补所属源文本；任务节点变更只修补窄范围的字面头属性；提升会创建节点并原子替换源文出现位置。task-node v1 不定义循环。完整权威见 [`../architecture/18-task-nodes-and-checklist-promotion.md`](../architecture/18-task-nodes-and-checklist-promotion.zh-CN.md) 与 [`17-tasks-and-query.md`](17-tasks-and-query.zh-CN.md)。尾部 `task:[...]` 宏只作为迁移输入接受，不是节点权威。

## 模板库角色

工作区根可以持久化一个 `weftext.template_library_root` 小写 UUIDv4。唯一解析到的活动受管节点是模板库根，仅作为容器。每个直接受管子节点都是一个 Template Root；该子节点下的每个后代都是由该 Root 拥有的 Template Part。Part 不能独立实例化，v1 没有嵌套分类层。

这些仍是 `X/X.adoc` 受管节点，但其角色由配置的根和当前树派生。模板库根、Root 和 Part 被排除在所有普通语义投影之外：节点、任务、标题/大纲、引用/书目、搜索、图、Chrono、默认链接/反向链接、最近项目以及规范 `nodes|tasks|headings` Query 行。它们只通过专用模板库投影出现；只有 Template Root 出现在明确的 `templates` Query 域中，在角色感知清单可用前返回 `domain_unavailable`。

只有 Template Root 可以携带相邻的 `weftext.template.json`，其固定 profile 对为 `weftext.node-template.v1` 和版本 `1`。伴随文件定义整个 Root/Part 子树的封闭参数/slot 契约，每个 slot 范围都是其 Root 或 Part 的永久节点 UUID。库根、Part、普通节点或其他位置出现同名文件时都只是诊断，不是资源或替代角色声明。独立写入的 `slot:name[]` 和 `slot::name[]` 在普通 AsciiDoc 中不起作用，只有在经过验证的角色/profile 中才获得语义；已有效的 prototype 不能移出角色边界后静默失效。

模板实例化目标会在一次经过审阅的可恢复事务中创建全新 UUID 的普通子树，通过完整映射重写内部链接，复制拥有的资源，默认省略设计批注，绝不复制伴随文件或角色。跨越模板角色边界的移动是带精确角色/源文/伴随文件验证和草稿门槛的转换事务，不是只按路径移动；离开模板空间时必须物化/删除所有活动 slot，且不得留下 profile/伴随文件残留，否则阻止操作。普通 Trash 拒绝当前配置的库根，除非同一显式事务同时清除或重新绑定根配置。Trash payload 保持封闭的精确字节，恢复绝不猜测或重新绑定配置。参见 [`19-node-template-library.md`](19-node-template-library.zh-CN.md)。

## 元数据封装与文档属性

`weftext` 是唯一允许的顶层 YAML key。v1 字段为 `id`、`icon`、`aliases`、`child_sort`、`child_sort_direction`、`sibling_rank`，以及仅根节点可用的 `adjacent_heading_body` 和 `template_library_root`。图标是受支持的 Weftext token 或字面 emoji 标量。别名是用于节点查找和链接的有序字符串列表，不是标题、标签或身份。`adjacent_heading_body` 取 `run_in` 或 `separate`，缺少时等同于 `separate`。`template_library_root` 是可选的小写 UUIDv4，绝不从文件夹名称或伴随文件推断。重复字段、未知顶层 key、结构无效的值、YAML alias/tag 或含义不明确的元数据封装都会安全失败。未知内部字段为前向兼容而保留，并产生诊断。

标题、副标题、作者、修订版、语言、描述、关键词/标签、状态和自定义笔记属性使用 AsciiDoc 文档头及头属性。只有头属性条目进入 Properties 投影；正文后续重新定义仍是处理状态。任意自定义属性保持字面字符串。像 `weftext-task` 这样的封闭保留 profile 可以在版本化契约下独立定义自己的 `weftext-task-*` 字段，但不能据此推断无关属性的类型。复杂结构化数据使用版本化类型 Profile 构造，而不是 YAML 或临时属性编码。

图标、别名和排序 UI 操作提交类型化意图，并执行窄范围的修订版检查补丁。仅查看节点绝不写入默认值。Core 派生的备用图标只用于呈现。在 Write 或 Read 中隐藏元数据封装，不会把它从精确 Source 中移除，也不会使其成为安全边界。
