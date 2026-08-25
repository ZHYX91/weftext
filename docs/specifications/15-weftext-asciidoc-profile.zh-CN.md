---
source_language: zh-CN
translation_status: source
---

[English](15-weftext-asciidoc-profile.md)

# Weftext AsciiDoc Profile v1

这是规范的受管源规范配置。受管文档使用所需标记、`weftext` 元数据封装、AsciiDoc 源和注释伴随文件。Markdown 仅可通过显式导入/导出接受，或作为可见的非受管内容接受。清单/任务节点、Query、Template Library、Citation Data、生命周期、无障碍/IME、恢复和跨平台客户端都必须保留此处及所链接架构规范中定义的契约。

## 保守超集兼容模型

Weftext AsciiDoc v1 是固定 AsciiDoc 基线的保守源语言超集，不是人工挑选的语法子集。参考基线是安全模式下、不含第三方扩展的 Asciidoctor 2.0.26；配置文件一致性语料库而非浮动的 `latest` 文档，冻结了精确的可接受行为。在该基线中有效的源构造仍是有效的 Weftext 源，保留其原生含义，并在打开/编辑/保存时逐字节存续，除非作者调用显式 Core 操作。未来上游行为只能通过经审查的配置文件版本转换进入。

源接受、语义建模、渲染以及执行效果的许可是相互独立的承诺。尤其是，识别包含、条件、直通内容、URI 或处理器声明，并不授予文件系统、网络、环境、扩展加载或可执行访问。被禁用的活动构造仍是有效的精确源，并带有能力诊断。此执行策略不是重新解释或丢弃该构造的许可。

每个构造都记录在两个独立轴上，而不是被强制归入一个兼容类别：

| 轴 | 值 |
| --- | --- |
| 语法来源 | 固定 AsciiDoc 基线；具名 Asciidoctor 兼容方言；采纳的生态扩展；Weftext 扩展 |
| Weftext 支持状态 | 完整；受限；仅保留；禁止效果 |

能力表还记录 Weftext 是否拥有任何附加语义、通用处理器降级、兼容导出行为，以及首次接受该构造的配置文件版本。因此，带有 Weftext 呈现语义的原生角色、使用 Weftext 安全渲染器的生态图表拼写，以及被保留但禁用的包含，不再会被赋予误导性的互斥标签。

扩展必须使用 AsciiDoc 扩展点，不得改变有效基线构造的含义，并且必须具有精确源语法、受保护上下文规则、诊断、通用降级和兼容降级。Desktop、CLI、Server、WebUI、导入器、导出器和智能体消费一个版本化 Core 模型，不得添加私有语法。通用 AsciiDoc 处理器是互操作性目标，而非 Weftext 存储权威。精确源始终胜过派生渲染。

`cite` 行内宏以及 `nocite` 和 `bibliography` 块宏采用一种小型、既有的 Asciidoctor 扩展拼写，而 Weftext 拥有解析、事务、作用域和呈现语义；Weftext 不加载也不声称兼容 `asciidoctor-bibtex` 或 `asciidoctor-bibliography`。规范引用记录的编写仍由一个单独接受的强类型 Citation Data Profile 构造所把关。完整的权威和接受边界在 [`16-citations-and-bibliography.md`](16-citations-and-bibliography.zh-CN.md) 中定义。

当前能力清单如下：

| 构造 | 语法来源 | Weftext 支持状态 | 附加语义和通用降级 |
| --- | --- | --- | --- |
| `weftext` YAML frontmatter 元数据封装 | 由 Asciidoctor 跳过 frontmatter 行为识别的生态约定 | 完整，Weftext 拥有的运行元数据封装 | 通用处理器需要 skip-frontmatter 配置 |
| 文档标题/副标题和 H1–H5 | AsciiDoc 基线 | 完整 | 原生降级 |
| H6–H9 | Weftext 扩展 | 完整 | 通用处理器需要兼容降级或警告 |
| `[.run-in]` / `[.separate]` | 原生角色语法 | 完整，具有 Weftext 呈现 | 通用处理器保留普通的带角色章节 |
| `[quote] ____` | AsciiDoc 基线 | 完整 | 原生降级 |
| `>` 嵌套引文 | 具名 Asciidoctor 兼容方言 | 深度 9 以内完整支持 | 兼容导出可以降级为原生引文块 |
| 原生锚点和 `xref:` | AsciiDoc 基线 | 完整 | 原生降级 |
| `node:` / `node::` | Weftext 扩展 | 链接完整支持；块嵌入延后 | 通用处理器保留未知宏；导出显式降级 |
| 块标题、表格、源/列出/文字块、图像 | AsciiDoc 基线 | 在 Core 操作和安全渲染器内完整支持 | 原生降级 |
| STEM 和 `latexmath` | AsciiDoc 基线 | 受限的安全渲染 | 不受支持的表达式保留为源；不执行 TeX |
| `[mermaid] ....` | 采纳的图表扩展拼写 | 受限的 Weftext 渲染器 | 没有扩展时文字块源仍可见 |
| `footnote:` | AsciiDoc 基线 | 完整源语义 | 原生降级 |
| `endnote:` / `endnotes::[]` | Weftext 扩展 | 已接受，运行时延后 | 未知宏保留为源；导出显式降级 |
| `cite:`, `nocite::`, `bibliography::` | 参考文献扩展拼写 | 受限的 Weftext 语法 | Weftext 拥有数据/解析/作用域；通用导出是显式的 |
| 原生待办项 | AsciiDoc 基线 | 精确源文本待办项识别 | 原生静态待办项降级 |
| `weftext-task` 头部配置文件 | 原生文档属性上的 Weftext 扩展 | 规范任务节点配置文件 | 通用处理器保留文字文档属性和正文 |
| 尾随 `task:[...]` | 已取代的 Weftext 扩展 | 仅显式迁移输入 | 永不作为规范输出 |
| `[.weftext-query,version=1,...] ....` | Weftext 带角色文字块扩展 | 规范 Query 块 | 通用处理器保留带角色文字块 |
| `slot:name[]` / `slot::name[]` | Weftext 行内/块宏扩展 | 受限于有效 Template Root/Part 源；在已配置 Template Library 外不可用 | 普通/通用 AsciiDoc 保留未知宏而不求值 |
| 包含和条件 | AsciiDoc 基线 | 源被接受，效果受限 | 未经审查的 Core 能力不得进行路径/网络展开 |
| 直通内容和任意处理器 | AsciiDoc 基线或处理器扩展 | 源被接受，默认禁止效果 | 精确源加能力诊断；绝不自动执行 |
| 未知第三方扩展 | 生态扩展 | 仅保留 | 不动态加载；需要显式适配器/导出工作 |

## 受管文档元数据封装

唯一的受管节点形态是 `X/X.adoc`。该目录仍是当前定位符，小写 UUIDv4 则是身份。YAML frontmatter 是位于 AsciiDoc 文档头之前的 Weftext 运行元数据封装。它恰好有一个顶层 `weftext` 映射：

```adoc
---
weftext:
  id: "550e8400-e29b-41d4-a716-446655440000"
  icon: "weftext:project"
  aliases:
    - 文缕
---
= Weftext: Local-first knowledge workspace
:lang: en
:description: Local-first knowledge workspace notes
:status: draft

== First section
Body text.
```

YAML frontmatter 是 Weftext 配置文件的一部分，并非声称每个 AsciiDoc 处理器都解释 YAML。Core 拥有其精确范围和窄补丁。在 Weftext 中使用的处理器必须跳过它，而不能将其渲染为文档文本。除 `weftext` 以外的顶层键、旧的 `_weftext` 拼写以及一般用户 YAML 属性都不是规范形式。

`weftext` 仅包含浅层 Weftext 操作字段 `id`、`icon`、`aliases`、`child_sort`、`child_sort_direction`、`sibling_rank`，以及仅根节点使用的 `adjacent_heading_body` 和 `template_library_root`。后者是选择唯一 Template Library 容器的小写 UUIDv4；缺失时禁用该角色投影。标题、作者、修订、语言、描述、关键词/标签、状态和自定义笔记元数据使用原生文档头及其属性。只有文档头属性条目进入稳定的 Properties 投影；正文后续重定义仍是 AsciiDoc 处理器状态。读取器使用有界文字值，且绝不通过环境、路径、URI 或可执行效果展开属性。

产品不存在竞争的受管文档类型。`.md` 仍可导入、导出并有资格被归类为非受管，但绝不会被选作规范受管节点格式。

## 文档标题、副标题和章节

原生文档标题形式具有权威性：

```adoc
= Main title: Subtitle
```

第一个有效的零级标题是文档标题。原生冒号分隔符提供副标题。Weftext 不会在 YAML 中重复任一值。受管文档可以省略标题，此时导航回退至节点名称；省略不会把稍后的章节标题变成文档标题。第二个零级标题是无效的配置文件结构，而非另一个正文标题。

文档标题和副标题是头部元数据。它们不会成为正文大纲条目，也不会占用正文标题级别。

正文标题映射如下：

| 正文级别 | 源标记 | 来源 | 支持 |
| --- | --- | --- | --- |
| 标题 1 | `==` | AsciiDoc 基线 | 完整 |
| 标题 2 | `===` | AsciiDoc 基线 | 完整 |
| 标题 3 | `====` | AsciiDoc 基线 | 完整 |
| 标题 4 | `=====` | AsciiDoc 基线 | 完整 |
| 标题 5 | `======` | AsciiDoc 基线 | 完整 |
| 标题 6 | `=======` | Weftext 扩展 | 完整 |
| 标题 7 | `========` | Weftext 扩展 | 完整 |
| 标题 8 | `=========` | Weftext 扩展 | 完整 |
| 标题 9 | `==========` | Weftext 扩展 | 完整 |

H1–H5 使用原生章节范围。H6–H9 是 Weftext 配置文件扩展。Core 通过九级暴露真实级别；大纲、标题路径、锚点、注释、搜索、Write、Read 和导出不得扁平化 H6–H9。通用处理器可能无法识别扩展级别，因此兼容导出必须明确警告或转换它们。

## 行内标题和正文呈现

Weftext 保留现有的相邻标题/正文功能，而不把标题和段落变成一个语义块。首选的逐标题形式使用带 Weftext 定义呈现语义的原生角色语法：

```adoc
[.run-in]
== Definition
Weftext is a local-first knowledge workspace.
```

Write 和 Read 可以将其呈现为一个视觉段落，标题样式为前导行。Core 仍返回独立的 Heading 块和 Paragraph 块，各自拥有独立的范围、锚点、大纲行为、链接、注释、搜索文本和无障碍语义。源保持精确。

根节点可移植默认值接受 `run_in` 或 `separate`；缺失等同于 `separate`。行内形式为：

```yaml
weftext:
  adjacent_heading_body: run_in
```

解析顺序为：

1. `[.run-in]` 强制符合条件的标题及其第一个段落一起渲染。
2. `[.separate]` 强制分离呈现。
3. 两种角色均不存在时，应用根工作区设置。
4. 在工作区 `run_in` 默认值下，只有紧随其后的物理行上的段落参与；空行会令其保持分离。

即使常规 AsciiDoc 间距包含空行，显式 `[.run-in]` 也会目标指向首个后续普通段落。移除该角色会恢复默认行为。只有正文 H1–H9 符合条件。文档标题、副标题、列表、引文、表格、代码、数学、图表、图像、分隔块和其他非段落块都不能被静默合并。未应用 Weftext 样式的通用 AsciiDoc 处理器会降级为普通的带角色章节和段落。

## 引文

语义或具署名的引文使用原生 quote 块：

```adoc
[quote, Ada Lovelace, Notes]
____
Quoted text.
____
```

对于快速嵌套引文，配置文件接受 Asciidoctor 兼容标记形式：

```adoc
> First level
> > Second level
> > > Third level
```

深度 1–9 获得完整编辑和呈现支持。更大深度仍为精确、深度感知的源，且不得扁平化。精确的续行、空行、署名和嵌套规则在解析器支持前需要一致性夹具；文字、列出、源或其他受保护块中的引文标记绝不会被提升。

## 锚点、普通交叉引用和受管节点链接

Weftext 保留原生 AsciiDoc 锚点和交叉引用：

```adoc
[#section-id]
== Section

xref:other-file.adoc#section-id[Display text]
```

`[[id]]` 仍是已接受的原生锚点形式。因此它不会被重用于 Weftext 节点链接触发器。

稳定的受管节点链接使用以 UUID 为目标的 Weftext 行内宏：

```adoc
node:550e8400-e29b-41d4-a716-446655440000[]
node:550e8400-e29b-41d4-a716-446655440000[文缕]
node:550e8400-e29b-41d4-a716-446655440000#section-id[Relevant section]
```

空显示使用已解析的文档 Title，随后是当前节点名称。所选别名或自定义文本明确存储在方括号中，因此之后的别名更改不会重写作者标签。未来的块嵌入会在独占一行使用对应的块宏：

```adoc
node::550e8400-e29b-41d4-a716-446655440000[]
```

Core 拥有 UUID 解析、片段、出站出现、反向链接、图边、非披露和事务式重写。`xref:` 保持普通的 AsciiDoc 文件/章节机制；`node:` 是可移植的受管身份机制。链接插入可使用命令、斜杠操作、选择器或 `node:` 补全，且不需要模仿 `[[` 交互。

## 别名和元数据所有权

AsciiDoc 引用文本可以为某个特定引用提供一个显示标签，但它不是 Weftext 节点别名的可移植列表。因此别名仍留在 Weftext 操作 YAML 中：

```yaml
weftext:
  aliases:
    - 文缕
    - Weftext Notes
```

别名是搜索和链接选择器候选项，不是身份，也不必唯一。选择别名时会将其插入为明确的节点链接显示文本。

仅在含义匹配时使用原生文档头字段：文档标题/副标题、作者/电子邮件、修订、语言、描述、关键词、版权、文档类型、目录、章节编号，以及块标题/题注。简单的自定义笔记属性同样使用文字文档头属性条目。仅将 Weftext 运行数据保留在浅层 `weftext` 元数据封装中：ID、别名、节点图标、子节点排序策略和稀疏手动排名。复杂的作者领域数据使用强类型 Profile 构造。一个语义值只有一个权威；客户端诊断冲突的导入输入，而不是静默选择或同步它们。

此元数据封装下的顶层书目映射不是规范形式。结构化引用事实需要 [`16-citations-and-bibliography.md`](16-citations-and-bibliography.zh-CN.md) 中描述的强类型 Citation Data 构造；该能力仍未满足发布门槛。

AsciiDoc 的 `icons` 属性控制文档/提示渲染，绝不会被解释为 Weftext 节点图标。

## 节点图标

v1 节点图标是 `weftext` 下的一个标量：

```yaml
weftext:
  icon: weftext:project
```

它可以是一个文字 emoji，或一个稳定的 Weftext 自有内置令牌。任意 URL、路径、嵌套图标配方、原始颜色和由处理器选择的图标集不属于 v1。未知令牌会被保留并且不产生显式图标；缺失/不支持的声明使用派生的工作区项目默认值，且绝不会仅因查看节点就写入元数据。图标渲染是装饰性的，可访问的节点名称仍然存在。AsciiDoc 的 `icons` 属性控制文档/提示渲染，绝不会被解释为此节点图标。

## 块标题、题注、代码、数学和图表

Weftext 使用原生 AsciiDoc 块标题：

```adoc
.Architecture
image::architecture.svg[Architecture]

.Measurements
|===
|Name |Value
|===

.Example
[source,rust]
----
fn main() {}
----
```

点标题仍是作者文本。编号、交叉引用标签、图/表/清单列表呈现和导出都根据文档配置文件派生。

### 结构化表格语义

Weftext 表格编辑使用由配置文件一致性夹具固定的原生 AsciiDoc 表格单元格、表头单元格样式以及行/列跨度形式。它不引入 JSON 表格模型、HTML 片段或不透明编辑器元数据。现有有效的作者拼写保持精确，除非它们位于请求的编辑范围内；当 UI 操作必须添加或替换表格结构时，Core 使用一种有文档说明的规范原生拼写。

前连续 `N` 行形成列标题区域，前连续 `N` 列形成行标题区域。两者可以同时存在，因此即使原生拼写需要一个规范投影，左上交集仍在 Core 模型中携带两个语义角色。不连续或非前导的标题区域不属于 v1。Read 和导出暴露真实的表头关联，而非仅依赖粗体样式。

合并单元格是一个矩形的原生行/列跨度。合并不得跨越表格边界、切断现有跨度，或隐藏不支持的嵌套结构。如果若干选中单元格含有内容，Core 会通过可见的精确预览，按行优先顺序组合每个单元格正文。拆分会重建所表示的网格，将完整的合并正文留在前导单元格，并使其他单元格为空；精确恢复先前的单元格分布是 Undo，而非 Split。这些规则在不发明持久化合并历史的前提下避免内容丢失。

若固定基线无法无损表示已接受的表头或跨度编辑，Core 会报告该能力不可用并保留源。通用 AsciiDoc 处理器可以渲染更简单的呈现，但 Weftext 作者源仍是有效的原生 AsciiDoc，并会降级为可读的表格内容，而不需要仅限 Weftext 的预处理。

数学使用原生 AsciiDoc STEM 语法，而非 Markdown 美元分隔符。Weftext 首选的作者记法是显式 LaTeX：

```adoc
The result is latexmath:[x^2 + y^2].

.Energy
[latexmath]
++++
E = mc^2
++++
```

或者，文档可以在其原生头部设置 `:stem: latexmath`，再使用 `stem:[...]` 和 `[stem]` 块。不带该属性的无限定 `stem` 保留原生 AsciiMath 含义，绝不会被静默重新解释为 LaTeX。Weftext 渲染经审查的安全子集，不执行 TeX 或读取任意文件。不支持的表达式仍保留为带诊断的精确源。

Mermaid 是由 Weftext 渲染的块扩展，会保留文字图表源：

```adoc
.Process
[mermaid]
....
flowchart LR
  A --> B
....
```

渲染器被固定、隔离、受资源限制、经过净化、支持离线能力，并配有源/错误/无障碍回退。包括泳道行为在内的受支持图表族按能力版本化。

## 脚注和尾注

脚注使用原生行内宏形式：

```adoc
Text footnote:[A local footnote.]
Text footnote:source-note[Defined once.]
Later footnote:source-note[]
```

Weftext 添加平行的尾注宏：

```adoc
Text endnote:[A document endnote.]
Text endnote:source-note[Defined once.]
Later endnote:source-note[]
```

显式尾注放置块是：

```adoc
endnotes::[]
```

没有该块时，Weftext 会在节点/文档末尾派生尾注列表，而不重写源。在连续 Desktop 和 Web 阅读中没有页面边界：脚注引用会打开可访问的就近弹出框、旁注或窄屏工作表，而尾注会导航至尾注列表。在分页 PDF、打印或文字处理导出中，脚注会在目标格式支持时映射为页底注释，尾注会映射为文档/章节末尾注释。首个配置文件版本保留独立的派生编号序列和文档作用域。富块注释正文、跨节点注释、章节重置编号和详细导出回退仍被延后。

## 清单、任务节点、规范 Query 和模板槽位

普通清单使用原生无序列表标记。Weftext 作者完成项使用 `[x]`；原生 `[*]` 拼写同样被接受并保留：

```adoc
* [ ] Open item
* [x] Completed item
* [*] Also completed
```

清单标记是可移植的开/闭权威。Weftext 通过经过修订检查的窄 Core 编辑使其可交互；Asciidoctor `%interactive` 选项是渲染器提示，且不是规范源所必需。嵌套清单项保留列表层级，不暗示任务依赖。清单仍然没有身份，不能携带强类型日期、优先级、依赖、重复、任务级注释或其他持久字段。

持久任务是由文字文档头属性标记的普通受管节点：

```adoc
---
weftext:
  id: "550e8400-e29b-41d4-a716-446655440000"
---
= Submit the paper
:weftext-task: v1
:weftext-task-state: in-progress
:weftext-task-scheduled: 2026-09-01
:weftext-task-due: 2026-09-05
:weftext-task-priority: high
```

节点 UUID 是任务身份，绝不重复到属性中。只有文档头内文字的 `weftext-task` 和 `weftext-task-*` 条目参与封闭配置文件；正文重定义属于处理器状态。该配置文件提供固定的状态、优先级、日期/时刻和任务节点依赖字段。属性引用、替换、行内标记、自然语言日期和可执行表达式都是无效值，而非待求值的输入。重复被延后，因为任务节点序列和出现历史尚未接受。

原生待办项只能由 [`../architecture/18-task-nodes-and-checklist-promotion.md`](../architecture/18-task-nodes-and-checklist-promotion.zh-CN.md) 定义的显式、可恢复工作区操作提升。该事务创建任务节点、提升任何可确定转换的续写/后代内容，并以稳定的 `node:` 链接替换原始待办项位置。它移除复选框，而非保留第二个状态源。完整的字段、查询、导入和迁移契约见 [`17-tasks-and-query.md`](17-tasks-and-query.zh-CN.md)。已取代的尾随 `task:[...]` 宏只作为经审查的迁移输入接受，绝不会被识别为规范输出。

只有一个 Query 设施。任务是版本化规范带角色文字块的一个显式源域，而不是第二个 `tasks` 围栏或语言：

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

完整的 `weftext.expr.v1`、规范 Query 语法、词法 `this`、显式 `context`、标题域以及带标签的清单/任务节点投影，冻结在 [`18-canonical-query-and-expression.md`](18-canonical-query-and-expression.zh-CN.md) 和 [`17-tasks-and-query.md`](17-tasks-and-query.zh-CN.md) 中。产品客户端可以消费共享的 Core 求值器，但不能独立求值块。Query 结果是派生视图。切换或编辑一行会调用其强类型行种类选择的精确清单源或任务节点操作；任何任务数据库或物化结果都不会成为可移植权威。JavaScript、Shell 命令、网络访问、原始文件系统读取、环境时间和客户端私有求值器都是被禁止的效果。已取代的 Query 语法仅是私有一次性迁移输入，没有运行时解析器别名。

已配置的 Template Root/Part 子树可以包含由 Template Root 固定 profile-`weftext.node-template.v1`/version-`1` `weftext.template.json` 声明的 `slot:title[]` 行内或 `slot::body[]` 块宏；每个声明由目标 Root/Part 永久 Node UUID 限定作用域。只有该有效角色/配置文件才赋予这些宏槽位语义。在普通受管文档中独立创作的相同源是惰性的并被保留。已验证的原型不能被移动/恢复到普通空间后静默降级：显式转换必须实例化/删除活动槽位并移除配置文件/伴随文件，否则阻止操作。V1 没有通用原始槽位、条件、循环或 Query 执行；生成的普通节点不得包含残余槽位/配置文件/伴随文件。完整契约见 [`19-node-template-library.md`](19-node-template-library.zh-CN.md)。

## 包含、处理器和活动内容

没有 UI shell 或第三方 AsciiDoc 处理器可以获得不受限制的工作区、文件系统、网络、环境变量、URI 或命令访问。包含、条件、解析路径的属性、扩展、图表处理器和直通内容，都是由 Core 和经审查的渲染器边界控制的能力。不支持的活动行为仍是源文本或可见诊断；绝不会仅因打开文档就执行。

未来包含功能必须使用经验证的工作区相对定位符、内容边界和权限检查、循环/深度/大小限制、确定性依赖修订以及非披露规则。仅有通用安全模式不足以构成完整沙箱。

## 精确源、编辑和派生状态

Core 保留精确 UTF-8 字节、YAML 格式、行尾、受保护块以及用于操作的每个范围。Write 和 Read 是投影；Source 不是从规范化 AST 重新生成。所有补丁仍经过修订检查且范围狭窄。搜索、大纲、链接、反向链接、图、题注、注释编号、渲染图表和索引均是可重建的派生状态。

Core 为规范 AsciiDoc 行内、标题、段落/列表/引文/代码、表格、普通链接和图像资源引用命令暴露一个 `DocumentFormatPlan` 边界。输入和返回的选择范围均为 UTF-8 字节偏移。受保护范围、无效边界、缺失的语义块和畸形表格均安全拒绝。共享 React shell 将浏览器字符串位置映射到此契约，且绝不从文件扩展名选择语法。Markdown 行为被限制在显式导入器/导出器边界内，而不是运行时对等规范配置。

`weftext.annotations.json` 将每条可移植评审消息存为受约束的精确 `weftext.asciidoc.inline.v1` 源。导入的正文和锚点只会通过带唯一确定性目标映射的显式预览进行转换。设备草稿绑定 AsciiDoc 配置文件和修订；不兼容草稿是恢复证据，绝不自动应用。

解析器选择必须证明精确源映射、畸形输入行为、由 Core 控制的扩展钩子、Windows/macOS/Linux 打包、性能、许可和已接受的 Rust 最低版本。仅产生 HTML 的处理器不是足够的存储权威。

## 延后语法

以下内容仍是独立决策，客户端不得静默发明：

- 托管/非 Owner 任务调用方、规范 Query 产品视图、可编辑派生视图，以及超出 [`17-tasks-and-query.md`](17-tasks-and-query.zh-CN.md) 和 [`18-canonical-query-and-expression.md`](18-canonical-query-and-expression.zh-CN.md) 中契约的 Saved Query 存储；
- [`19-node-template-library.md`](19-node-template-library.zh-CN.md) 定义的 Template 角色清单、固定伴随文件解析器、引擎、Designer、实例化和角色转换事务；
- 词汇表/索引术语以及出版级跨文档编号；
- 富多块脚注/尾注；
- 超出 UUID 块嵌入的组件/转引参数；
- 不受限制的 AsciiDoc 扩展、任意 Ruby 处理器、远程包含和可执行内容；
- 超出已接受的显式 Markdown 导入/导出映射和诊断的兼容转换。

## 验收边界

工程验收要求 Desktop、CLI、Server 和 WebUI 的 Core 模型一致；精确源和畸形源夹具；索引重建；共享规范编辑器行为；CJK/UTF-8/混合行尾覆盖；注释/草稿协调；以及证明没有处理器绕过 Core 事务或 Server 授权。发布验收还要求通用 Asciidoctor 降级检查、Markdown 导入/导出兼容报告、已签名打包生命周期和恢复演练、实体无障碍/IME 覆盖，以及受支持 GUI 平台矩阵。
