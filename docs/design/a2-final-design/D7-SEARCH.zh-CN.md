---
source_language: zh-CN
translation_status: source
---

[English](D7-SEARCH.md)
# A2 D7 搜索语义与 SEARCH-01–08

状态：这是当前 D7 作者候选中的规范搜索设计；D8 的具体交互实现仍属于后续完整模块批次。

## 1. 方案裁决

本批比较了三种完整的交互方案。

| 方案 | 优点 | 成本 / 风险 | 裁决 |
| --- | --- | --- | --- |
| 只提供普通文本输入和可视化筛选 | 普通用户无需学习语法，且不存在词法歧义 | 熟练用户仅靠键盘操作时效率较低；复杂条件不便复制 | 保留为默认体验 |
| 在普通搜索框中自动解析常见的“字段:值”写法 | 写法短且熟悉 | 会误判正文里的 title:、URL、Windows 盘符、CJK 标点和未完成输入；解析失败后若退回普通搜索，还可能执行与用户原意不同的 Query | 拒绝 |
| 普通文本输入为默认，同时提供可视化筛选和显式的快捷条件模式 | 不要求用户学习语法，同时给熟练用户提供确定、可复制、可往返编辑的快捷路径 | 需要一个小型解析器，并且界面必须清楚显示当前模式 | 选择 |

所选方案只有一套执行权威。任何成功的搜索状态都会编译成既有的 D7 Query，并沿用相同的 D6 授权、完整范围证明、预算、结果、分页和重置规则。系统不新增第二个全文搜索执行器，也不建立区别于 saved Query 的持久搜索对象。

## 2. 搜索对象的名称与边界

当前 D3/D2 并不存在一个可以把所有“文件名称”都合并进去的通用作者字段，因此搜索界面必须区分下列概念：

- 文档标题：当前 D2 原生文档标题，可以不存在；
- 文档副标题：当前 D2 原生副标题，可以不存在；
- 文件名：只有在当前主体有权读取相应元数据时，才使用当前 D6 FileBinding 的 basename；
- 路径：只有在当前主体有权读取路径时，才使用 D6 当前相对路径；
- 资源名称：Resource 的 FileBinding basename，与 Document 标题不是同一概念；
- 逻辑层级：用于枚举 Node 的 D3 parent/order 范围，不是稳定路径身份；
- 正文：通过完整当前 D2 求值屏障取得的语义正文 body_text；
- 扩展贡献文本：由显式选中的当前 SearchContribution 提供的 Field/textPath。

无标题文档必须继续保持“无标题”。文件名、路径、列表占位文字或第一段可以作为界面回退显示，但绝不能被写成标题事实。同名标题、位于不同合法位置的同名文件、内容相同的扩展贡献文本，都不能合并身份。

若 D3 规则允许改名或移动而保持身份，则当前文件名或路径的匹配结果可以变化，但 NodeRef 不变。旧结果按正常当前性规则失效；保存的 Query 会在重新执行时读取当前获权的 FileBinding，而不会把旧显示路径冻结成身份。

## 3. 三种产品搜索入口

**文件列表筛选**只在已经选定的逻辑列表范围内运行。默认范围是当前 D3 parent 的直接子项，不递归，只包含 live Node；默认不读取正文、Annotation 正文、Trash、附件提取文本或 OCR。界面可以明确开启递归子树或 Resource 搜索；这些选项必须编译成明确的 Query 范围和数据源，并分别取得披露权限。此入口中，空输入表示“浏览当前列表范围”，绝不是一次空字符串的全 Workspace 搜索。

**快速打开**默认在选定 Workspace/root 范围内搜索获权的 live managed Node，读取标题、副标题以及另行获权的文件名元数据。正文、Annotation 正文、附件提取文本、OCR 和 Trash 默认关闭。只有用户明确开启 Resource 选项时，才加入获权 Resource 以及资源名称匹配。快速打开仍然是 Query 预设，不是按文本执行身份查找。

**全局正文搜索**默认在选定 Workspace/root 或明确子树中搜索获权的 live Document，并默认启用标题、副标题和正文。Resource、附件提取/OCR、Annotation 正文和 Trash 均为明确的可选范围。附件内容只有在真实的当前提取结果或 SearchContribution 依赖成立时才可搜索。Annotation 文本必须先通过 Annotation 的披露和读取权限，才能取得正文。提供方缺失、提取未完整、来源被隐藏或 placeholder 尚未物化时，完整搜索必须返回不可用状态，绝不能把它们当作“零命中”。

## 4. 匹配、Unicode、布尔组合与排序

本代保留既有 D7 SearchContribution 的匹配边界。内建 title/body 文本以及 D4 声明为 `normalization:"exact"` 的 text path 使用精确、区分大小写的 Unicode 标量比较；D4 声明为 `normalization:"nfc-for-compare"` 的 path 只在比较时分别对候选文本与 needle 做 NFC，不改写作者源字节。`contains` 在所选比较 basis 上执行精确子串匹配，equality 则提供既有的 exact-match 排名。系统不自动进行大小写折叠、模糊匹配、词干提取、按语言猜分词、转写或拼音搜索；未来若增加新的 comparison profile，必须通过明确且版本化的语义扩展，不能只靠界面开关改变结果。

CJK 文本直接按照 Unicode 标量子串匹配，不依赖空白分词。RTL 只影响呈现，不改变匹配。带引号的短语表示一条精确的标量序列。普通搜索框中的文本整体作为一个字面短语处理，不会自动拆成多个词。可视化筛选的多行条件默认用 AND 连接；当既有 Query 表达式能够表示时，界面可以显式创建 OR 分组和 NOT 条件。

快捷条件模式中，相邻的基本条件表示 AND，OR、NOT 和括号遵循下一节的语法。匹配排名和角色顺序继续以本候选复制的 Query Algebra owner 正文中既有 SearchContribution 排名规则为准，界面不得私自增加模糊分值。当既有比较器完全相等时，Query 先以规范 subject key，再以所选 contribution identity 作为稳定的最终并列顺序，从而保证分页确定。一般 Query 的排序仍由作者定义，不能跟随视口中的临时顺序。

## 5. 快捷条件模式

快捷语法必须由用户显式开启；仅仅输入冒号不会启用它。界面中的模式标记属于交互状态；解析成功后，真正的语义对象仍是编译后的 Query 条件。

```text
Shortcut mode v1

plain-token        := escaped-token | quoted-value
field-condition    := @title:value
                    | @subtitle:value
                    | @filename:value
                    | @path:value
                    | @body:value
                    | @resource-name:value
                    | @field(field-id[,member-path]):value
value              := quoted-value | escaped-token
quoted-value       := "..." with backslash escaping for quote and backslash
boolean-expression := primary
                    | NOT primary
                    | primary AND primary
                    | primary OR primary
primary            := plain-token | field-condition | ( boolean-expression )

precedence: parentheses > NOT > AND > OR
whitespace between adjacent primary terms is AND
a colon is syntax only inside a recognized @ operator
unknown @ operator or invalid field/member path is a parse error
incomplete input is a draft parse state and executes no Query
```

选择 @ 前缀是为了避免误解析普通文字。在普通模式中，title:、https://example.test、C:\\notes\\a.adoc、12:30、带引号的普通句子以及 CJK 全角标点都只是字面文本。即使已经进入快捷条件模式，也只有识别到受支持的 @ 操作符时才产生语法含义。若要在快捷模式中搜索以操作符拼写开头的文字，可以使用引号或转义开头的 @。

未知的 @ 操作符、未知 FieldId、不可用的 Field 定义、非法成员路径、未闭合的引号或括号、以及非法值，都必须在输入 Draft 的准确位置报告错误。界面不得退回到另一条字面 Query。未完成输入保持为可编辑 Draft，同样不执行 Query；因此解析错误不会静默变成更宽的搜索。

第一代内建关键字集合固定为 title、subtitle、filename、path、body、resource-name 和 field。本代刻意不提供笼统的 file 或 name 关键字，因为这会混淆不同数据域。field 必须使用稳定的 D4 FieldId；若给出成员路径，还必须用与 Query 相同的当前 Registry 进行验证。界面本地化标签和别名都不能替代稳定 ID。

## 6. 可视化筛选与往返编辑

普通输入、可视化筛选和快捷条件模式都编辑同一份条件模型。能被便利语法表示的 Query 条件，应当可以在可视化条件和快捷文本之间无损往返；仅调整筛选项的显示顺序不能改变语义。

完整 Query 可能包含便利语法无法表达的条件。此类条件必须作为“高级条件”节点保留在可视化模型中；切换到快捷文本时，界面应明确说明该条件无法用快捷文本表示，绝不能把它丢掉。用户仍可打开完整 Query 编辑器修改它。保存时必须序列化完整的 canonical Query，不能只保存当前看得见的筛选项。

键盘、指针、触摸和辅助技术都应调用同一组条件操作。焦点绑定逻辑条件 ID，而不是 DOM 下标。搜索刷新、排序或结果 epoch 重置后，旧结果行的焦点、选区和 evidence 都要失效，并把焦点返回稳定容器；不得把复用后的视觉行错误地赋给另一个对象。IME 预编辑阶段不得发起 Query，只有输入法完成确认后的文本才能更新搜索 Draft。

CJK 输入法、RTL 文本、双向文本隔离、读屏名称、移动端面板和硬件键盘路径均属于后续 D8 的交互验收义务。本批只冻结它们应当对应的语义目标和验收条件，不声称任何平台行为已经通过。

## 7. 权限、索引状态、命中数、排名与摘要

授权与最低披露必须先于任何敏感来源、Field、Contribution、附件、Annotation 或索引私有数据读取。Search 不得通过命中数量、排名缺口、摘要、补全提示或错误原因泄露隐藏对象是否存在。

Derived Index 只能作为候选加速器。building、partial、stale、unavailable 与“已证明完整且零结果”必须严格区分。完整搜索要求当前授权的 query_scan，并覆盖真实的正负来源范围、Registry、Contribution、提取结果和授权依赖。部分探索只能显示已经明确覆盖的范围，并持续标记为不完整；它不能签发完整 ActionEvidence，也不能把当前页冒充整个结果集。

缺失所选 Field、SearchContribution、提供方或提取数据时，应在正常披露门之后返回既有的 unavailable/reset 结果，不得退化为空 Contribution。若预算耗尽，完整结果按既有 D7 预算错误失败，不能返回前 N 条后宣称完整。取消继续使用既有明确 outcome。

命中总数只能从获权且完整的结果计算。索引仍在构建时，界面可以不显示数量，或显示“未知”，不得估算隐藏命中。排名只能使用已经获权的匹配值和既有 D7 比较器。

摘要和高亮只能从已经获权的匹配语义文本派生。它们的标量偏移仅是该结果值内部的临时呈现坐标，不是 D3 Locator。打开命中项时，必须按照真实的 Ref、provenance 或 Locator 规则重新执行一次当前解析和读取；若结果或来源已经过期，则重新验证或重置，不能把旧显示偏移套到新字节上。

## 8. 保存、重开、复制与导入

保存搜索时只保存既有 canonical Query 定义，以及明确的范围、参数、稳定 D4 FieldId、所选 SearchContribution 的 contributionId/version 依赖和作者定义的排序。快捷文本本身不是持久搜索权威。设备可以为便利而记住解析器版本或上次输入文字，但这些状态可丢弃，且不得进入 DynamicBlock、DefinitionTransfer、结果缓存或 ActionEvidence。

重开时先按保存时记录的作者 schema 解码 Query，再对当前 definition、Registry、Contribution 和权限重新资格化。Field 被删除或类型不兼容时，遵循 Registry 的迁移/不可用规则；SearchContribution 被移除或版本变化时，遵循 D10 激活依赖规则；权限变化会重置当前结果。改名或移动只改变当前文件名/路径值，不会按文本改变稳定内容身份。

复制、fork 或导入搜索定义时，对 canonical Query payload 使用既有 D7 Definition Transfer。typed Ref 与 DefinitionAddress root 按 D3 规则映射；FieldId 与 SearchContribution 的稳定身份继续作为语义依赖，不能从显示标签猜测。未知 payload 不会自动转换成新搜索语法。旧结果行、摘要和 evidence 也不会被复制成当前权限。

## 9. 列表搜索、全文搜索与 saved Query 的结果等价

当文件列表、全局搜索和 saved Query 使用逐字节等价的 canonical 条件、范围、Contribution 集、排序和当前依赖 cut 时，它们必须具有相同的 D7 结果语义。不同界面的分页或虚拟化不能改变成员集合和顺序。

能力差异必须明确呈现。如果某个界面无法取得所需提供方、完整范围、安全摘要、双向文本交互或辅助技术导航，就应把相应能力标为不可用或不完整，而不是偷偷执行另一条 Query。跨设备的一致性要求是 canonical Query 与已资格化依赖的语义一致，不要求像素布局一致。

## 10. SEARCH-01–08 验收义务

| ID | D7 规范闭合与后续 D8 验收 |
| --- | --- |
| SEARCH-01 | 文件列表筛选、快速打开和全局正文搜索使用 §3 的范围；空输入浏览与显式搜索分开；递归、Resource、Annotation、Trash、正文及附件/OCR 范围都必须明确选择。 |
| SEARCH-02 | 标题、副标题、文件名、路径、资源名和逻辑层级彼此分域；无标题不得用文件名补造；显示值相同的对象仍保留不同 Ref；改名/移动使旧结果资格失效，但不按文字改变身份。 |
| SEARCH-03 | 当前匹配按 §4 保留 exact 与 `nfc-for-compare` 两种比较 basis：二者都区分大小写，substring/equality 只使用该字段声明的 Unicode 标量比较规则，并且比较不会改写 source bytes；CJK 与 RTL 的结果语义保持确定，AND/OR/NOT 必须显式；不声称模糊、分词、拼音或词干能力；未来未支持模式不能静默执行。 |
| SEARCH-04 | 快捷模式必须显式开启，并采用 §5 的 @ 操作符、引号、转义、优先级和错误规则；普通冒号文本、URL、Windows 盘符和 title: 保持字面含义；非法或未完成快捷输入不执行替代 Query。 |
| SEARCH-05 | 可视化条件与快捷条件编辑同一模型；无法表达的高级条件必须保留；焦点、键盘、触摸、辅助技术、IME、CJK 和 RTL 语义按 §6 固定，平台证据留给后续 D8。 |
| SEARCH-06 | 权限先于敏感读取；隐藏命中数、排名和摘要不得泄露；索引构建中、部分、过期、不可用与完整零结果分开；Contribution/提供方缺失和预算耗尽有明确不可用或错误结果；普通打开、编辑和保存不受无关完整搜索证明阻断。 |
| SEARCH-07 | 保存、重开、复制和导入持久化 canonical Query 与稳定语义依赖，而不是设备 UI 状态；Field、Contribution、版本或权限变化会重新资格化或重置；旧结果不会获得新的当前资格。 |
| SEARCH-08 | 打开命中项时重新执行当前获权的来源解析；摘要/高亮偏移不是 Locator；相同 canonical 条件在列表、全文搜索和 saved Query 中产生相同成员和顺序，同时允许能力差异明确呈现。 |

这些都是设计验收义务。本批不宣称任何产品搜索、平台交互、分词器、提供方、辅助技术、性能或跨设备测试已经通过。
