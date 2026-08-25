---
source_language: zh-CN
translation_status: source
---

[English](18-canonical-query-and-expression.md)

# 规范 Query 与 `weftext.expr.v1`

本规范定义规范 Query 和 `weftext.expr.v1`。已交付调用方通过显式限定的 `weftext.query-expression-subset.v0` 能力，使用规范 Query 外层语法、词法上下文、强类型域/作用域、有界执行和稳定结果身份。完整求值器和可复用 Template 绑定仍是独立能力；不支持的表达式必须以精确诊断失败。已取代查询语法仅由显式一次性迁移接受，绝不是运行时别名。

## 权威与分离

规范 Query 是对可移植受管权威的派生、经权限过滤的读取。仅 Core 解析、类型检查、解析作用域、求值、排序并返回操作证据。已交付 Query 调用方传输该 Core 请求/结果，而非实现第二个解析器或求值器。导出器、模板、已保存查询存储和智能体操作不会仅因存在 Core 子集而获得隐式执行权威。

`weftext.expr.v1` 是 Query 表达式和 Template 绑定共同使用的表达式基底。Query 拥有 `from`/`scope`/投影/排序子句。Template 拥有槽位声明和伴随文件绑定。表达式不是 Query，Template 槽位也不能包含或执行 Query 子句。

`weftext.query-expression-subset.v0` 是序列化在派生 Query 计划中的诚实编译器能力标识符，因此 Core 可以拒绝伪造或过期计划。它只实现当前规范 Query 运行时已使用的表达式形式，包括显式源/上下文引用、固定谓词、null 行为和日偏移。它绝不能出现在作者 AsciiDoc、Template 绑定、已保存定义或兼容协商中，任何调用方也不得将不支持的 `weftext.expr.v1` 表达式重写为该子集。添加剩余表达式功能会在一个 Core 编译器中扩展或替换派生能力；不会添加另一种 Query 语法。

## `weftext.expr.v1` 值和字面量

封闭值类型为：

| 类型 | 字面量或构造 |
| --- | --- |
| `string` | JSON 兼容的双引号 UTF-8 字符串 |
| `bool` | `true` 或 `false` |
| `number` | 下文定义的精确可移植十进制数 |
| `null` | `null` |
| `date` | `date("YYYY-MM-DD")` |
| `instant` | `instant("RFC3339-with-explicit-offset")` |
| `duration` | 不加引号的仅日 `P1D` 至 `P36500D` |
| `UUID` | `uuid("lowercase-uuid-v4")` |
| `list<T>` | `[expr, ...]`；元素具有一种兼容类型 |
| `record` | `{"name": expr, ...}`，键为唯一文字值 |

数字字面量由可选 `-`、十进制数字和可选小数部分组成。它没有前导 `+`、指数、基数前缀、分隔符、NaN 或无穷大。去掉小数点和无意义的前导零后，其有符号系数最多有 34 位有效十进制数字；其小数位为 `0..18`。零有一位系数。值是精确数学十进制数，因此比较可以规范化尾随小数零，但绝不舍入。超出系数/小数位边界的字面量、构造器结果、传输值或操作都会返回 `numeric_overflow`；不存在隐式舍入或二进制浮点权威。

Duration v1 必须是不加引号的 `P<n>D`，其中整数 `n` 位于 `1..36500`。小时、周、月、年、小数、带符号前缀的值、零以及其他 ISO-8601 时长形式均无效。这使 `context.today + P14D` 保持为唯一规范的按日运算形式。

不存在隐式字符串/数字/日期/UUID 转换。记录成员访问对封闭标识符字段使用 `value.member`。文字属性映射使用方括号访问，例如 `node.document.properties["名称"]`；点访问不会重新解释任意属性名称。索引缺失属性返回 null。未知的封闭记录成员是类型错误。

## 运算符和纯函数

固定 v1 运算符为：

- 比较：兼容标量类型上的 `=`、`!=`、`<`、`<=`、`>` 和 `>=`；
- 布尔：`not`、`and` 和 `or`，优先级依次为括号、`not`、`and`、`or`；
- null：`is null` 和 `is not null`；
- 成员关系：在一种兼容元素类型上使用 `value in list`；以及
- 时序算术：`date|instant + duration`、`date|instant - duration`，以及同种时序相减得到 `duration`。

除 `is null` 和 `is not null` 外，任一操作数为 null 都会使普通 `=`、`!=`、`<`、`<=`、`>` 或 `>=` 比较成为 `null_comparison` 类型错误。布尔操作数必须为 bool。对于成员关系，右操作数必须是左操作数非 null 基础类型的非 null 同质列表；左值为 null 时返回 false，null 列表元素永不匹配，右操作数为 null 则是类型错误。v1 不包含除法、数字算术、正则表达式、用户定义运算符、重载字符串 `+`、赋值、变异和隐式真值。

唯一可调用函数为：

| 函数 | 结果 |
| --- | --- |
| `contains(string, string)` | 文字、区分大小写的包含关系 |
| `starts_with(string, string)` / `ends_with(string, string)` | 文字、区分大小写的前缀/后缀 |
| `format_date(date, format)` | 下文定义的与区域无关 ASCII 公历格式化 |
| `length(string|list)` | 标量/列表长度，结果为 number |
| `concat(string, ...)` | 拼接字符串，至少一个参数 |
| `coalesce(value, ...)` | 第一个非 null 的兼容值 |
| `date(string)` / `instant(string)` / `uuid(string)` | 经过验证的强类型构造 |

没有其他函数可用。尤其不存在文件系统、网络、环境、进程、秘密、环境时钟、随机数、区域查找、`eval`、动态加载、反射、扩展回调、Query 执行或模板执行。

`format_date` 仅接受标记 `YYYY`、`MM` 和 `DD`，每个最多一次，以及最多 64 字节格式字符串中的文字 ASCII `-`、`/`、`.`、`_` 和空格分隔符。标记输出零填充的前推公历年、月、日；字母、转义、名称、可变宽度字段和其他每一种标记均无效。输出受普通 4,096 字节字符串限制。该函数与区域和时区无关，也不读取环境状态：`format_date(context.today, "YYYY-MM-DD")` 只格式化显式提供的日期。区域或时区敏感的格式化将需要一个带显式参数的、单独版本化函数。

## 求值根和上下文

Query 表达式只能访问其声明的行别名、`this` 和 `context`。Template 绑定只能访问 `input` 和 `context`。每个根都是在求值前提供的封闭记录类型。

`context` 恰好具有：

| 成员 | 类型 |
| --- | --- |
| `context.today` | date |
| `context.now` | instant |
| `context.timezone` | string |
| `context.locale` | string |

调用方把全部四项作为一个不可变上下文提供。`today` 和 `now` 从不是裸标识符，也绝不从环境时钟读取。时区是显式 IANA 标识符，时刻保留其显式偏移；区域不能改变比较、排序、解析或大小写行为。

## 嵌入式 Query 语法

唯一规范的嵌入式语法为：

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

可选 AsciiDoc 块标题提供 `this.query.title`。块样式/角色必须恰为 `.weftext-query`；`version=1` 是必需的。`view` 可选，接受 `table`、`list`、`task-list`、`board`、`calendar`、`timeline` 或 `gallery`。它仅是初始呈现提示。禁止将 `source` 用作块属性，因为源语义仅属于 `from`。`task-list` 要求 `tasks` 域。

通用 AsciiDoc 处理器保留普通带角色的文字块和精确正文。它不执行查询。Core 仅在受保护范围外识别该块，并将解析与求值分离。

## Query 语法

正文为 UTF-8，使用 ASCII 小写关键词、JSON 兼容字符串和 `#` 行注释。子句最多各出现一次，且仅能按此顺序出现：

```text
from        required
scope       required
where       required
group by    optional
select      required
order by    required
limit       required
```

V1 域为 `nodes`、`tasks`、`headings` 和 `templates`。每个源声明都是 `from <domain> as <alias>`。别名是必需的，使用 ASCII snake-case，且可由作者选择，但不能遮蔽 `this`、`context`、域名或关键词。`row.*` 和裸字段名称无效。

`scope` 是以下之一：

```text
scope workspace
scope descendants(<node-reference>)
scope subtree(<node-reference>)
scope section(<heading-reference>)
```

节点引用是强类型 Node 记录，例如 `this.node`；标题引用是强类型 Heading 记录，例如 `this.heading`。不存在路径/名称/UUID 作用域，也不存在隐式当前节点。`descendants` 排除具名节点；`subtree` 包括它。`section` 选择在词法上由解析器所有的标题章节拥有的行。传入 null `this.heading` 返回精确 `missing_heading_context`。由于模板行已属于已配置库，`templates` 域在 v1 仅接受 `workspace`。

`where` 要求一个 bool 表达式；无过滤查询会明确写入 `where true`。`group by` 接受一个标量表达式和可选输出别名。`select` 接受一至 32 个以逗号分隔的表达式。直接成员路径推断其最终成员名称；`as <output-name>` 可选。计算表达式要求 `as`，且所有得到的 ASCII snake-case 输出名称必须唯一。`order by` 要求一至八个以逗号分隔的表达式，后随 `asc|desc` 及可选 `nulls first|last`。`limit` 是整数 `1..1000`。稳定的最终排序依据是域路径、适用时的行种类、适用时的解析器源起始位置以及 UUID/操作证据。

授权会在任意表达式、隐藏依赖诊断、分组、计数、排序、投影、限制、建议、导出或缓存条目之前，筛选候选行并解析作用域。隐藏和缺失的显式目标产生一个非披露的不可用结果。

## 封闭行记录

`from nodes as node` 暴露：

| 成员 | 类型和含义 |
| --- | --- |
| `node.id` | UUID |
| `node.name` | string 当前基名 |
| `node.path` | string 当前工作区相对定位符 |
| `node.depth` | number |
| `node.parent_id` | UUID 或 null |
| `node.display_title` | 派生 string：作者文档标题，否则为当前名称 |
| `node.document` | 封闭 Document 记录 |

Document 记录暴露 `title: string|null`、`subtitle: string|null`、`display_title: string` 和 `properties`。作者 `title` 和 `subtitle` 仍可空，绝不会被派生回退替代。`display_title` 是明确派生的。`properties["名称"]` 返回有界文字文档头属性字符串或 null；正文重定义和处理器展开被排除。

`tasks` 域是带标签的清单/任务节点联合。其别名暴露 `kind`、可空 `id`、`owner_node`、非空 `title`、`closed`、`state`、可空 `checklist_depth`、可空 `priority`、可空 `created`、`start`、`scheduled`、`due` 和 `closed_at`，以及可空、经权限过滤的 `blocked`。`title` 是清单主体文本或任务节点作者编写/派生的显示标题。`owner_node` 是同样封闭的 Node 记录形状。清单专有字段对于任务节点保持 null，任务节点专有字段对于清单保持 null；不会捏造任何值。

`headings` 域为每个解析器所有的正文 Heading 返回一行，绝不是 Document Title。其别名暴露 `title: string`、`level: number`、`anchor: string|null`、`parent: HeadingRef|null`、`path: list<string>`、`owning_node: Node` 和 `document: Document`。H1 至 H9 分别对应源 `==` 至 `==========`。行的 `parent` 对 H1 为 null，否则为最近的包含它的低级标题。授权在行存在之前从 `owning_node` 继承。

`templates` 域为每个 Template Root 返回一行，绝不为每个 Part 返回一行。其别名暴露 `id`、`name`、`path`、`display_title`、`part_count` 和 `parameter_count`。Template Library 根和 Template Parts 不存在于每个普通语义域中，包括 `nodes`、`tasks`、`headings`、引文和默认反向链接投影。

`templates` 是受识别的稳定 v1 域名。当 Template 角色清单不可用时，求值返回精确 `domain_unavailable`；它不会返回空的成功结果、回退至 `nodes`，或投机地检查路径/伴随文件。当清单可用时，普通授权和非披露会在任何 Template Root 行或计数存在之前运行。

## 词法 `this`

`this` 只从作者块位置解析：

- `this.node`：所属 Node 记录；
- `this.document`：所属 Document 记录；
- `this.heading`：最近的解析器所有、包含当前块的正文 Heading 记录，或 null；它不同于任意 `headings` 行别名；以及
- `this.query`：具有来自块标题的可空 `title` 的记录。

不存在 `this.title` 或 `this.subtitle`。请使用 `this.document.title`、`this.document.subtitle` 或明确派生的 `this.document.display_title`。

Heading 记录具有 `title: string`、`level: number`、`anchor: string|null`、`parent: HeadingRef|null` 和 `path: list<string>`。`HeadingRef` 包含相同的非递归 title/level/anchor/path 字段。原生 `= Title` 是 Document Title。`==` 至 `==========` 是正文 H1 至 H9。只有标题的文档和前言会把 `this.heading` 解析为 null。H1 的 `parent` 为 null；被包含的 H2 报告其 H1 父级。`this.heading` 为 null 时访问标题成员产生精确 `missing_heading_context`，绝不会回退为文档标题。

焦点、选择、活动选项卡、Explorer 行、URL、当前请求节点和显示路径绝不会影响 `this`。引用 `this` 的 Saved Query 必须持久化或接收一个显式、不可变的嵌入绑定。若没有其中之一而执行，求值返回 `missing_context`。Saved Query 存储被延后；此执行规则未被延后。

## 界限和诊断

超过 16,384 UTF-8 字节、2,048 个令牌、256 个表达式节点、深于 32 的嵌套、任何列表中的 64 项、任何记录中的 64 个字段、八个排序键、32 个选定字段或高于 1,000 的限制的 Query 正文源，会在求值前被拒绝。该限制不限制包含它的受管 AsciiDoc 文档。每个解码字符串最多 4,096 UTF-8 字节。一次求值最多有 65,536 个表达式步骤。结果最多有 1,000 行以及最多 4 MiB 规范序列化强类型数据，包括分组和操作证据。编码和解码大小会在分配放大前检查；超出任一界限返回 `resource_limit`，没有部分计划或部分结果。求值不能递归。

诊断携带精确源范围，并具有用于语法、重复/有序子句、未知域/别名/成员/函数、`domain_unavailable`、类型不匹配、`null_comparison`、`numeric_overflow`、无效字面量、无效作用域、缺失上下文、缺失标题上下文、不可用目标、`resource_limit` 和禁止效果的稳定代码。解析器失败绝不执行部分计划。

## 迁移和运行时边界

已取代的 `[query,source=...]` 属性语法、旧正文 `scope` 形式、正文 `sort`、裸 `today`、裸字段以及任意 `row.*` 拼写，只由私有、只读、一次性转换器接受。转换器必须输出精确规范块，并证明已授权总体、null 行为、排序、投影和时间上下文等价；否则阻止整个迁移。产品运行时、普通导入输出、文档示例和新写入器绝不同时识别两种语法。

## 验收

验收覆盖显式源别名；版本/源/view 分离；精确子句顺序；没有裸字段或 `row.*`；所有类型/运算符/函数和禁止效果；精确十进制溢出/不舍入；仅日时长；普通 null 比较错误和固定的可空成员关系；确定性上下文；仅文档标题和前言标题 null；H1/H2 所有权和 H1 null 父级；作者文档标题/副标题与派生显示标题；不存在 `this.title`；可空查询块标题；Saved Query `missing_context`；强类型作用域引用和隐藏目标非披露；清单之前 `templates` 域不可用；带 Unicode 键的属性；通用 AsciiDoc 文字块降级；畸形/受保护/CJK/RTL/CRLF 输入；稳定排序；精确资源上限；以及每个已交付调用方中相同的 Core 结果。
