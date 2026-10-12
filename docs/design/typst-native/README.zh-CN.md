---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# Typst 主格式协调设计候选

状态：**需求与技术已分离，导航与列表需求已补齐，可开始技术重写；新架构未接受、产品未实现、全局终审未完成**。前轮固定输入为 `af6737a057c7bb69763dfaad0a57da4652e085f3`；作者分支为 `design/typst-native-20261012`。先读 [需求与技术方案的边界](REQUIREMENTS.zh-CN.md)：用户需求保留，旧技术不因历史通过而必须继承。下列技术 owner 仍记录前轮候选，须在协调重写中替换；前轮有界 Sol 复核及验证见 [REVIEW](REVIEW.zh-CN.md)，不自动接受新增需求的实现。

Weftext 的新笔记使用 Typst 作为唯一主格式，复用官方 Rust parser/compiler、原生语法和标准库。保留普通写作、流式阅读、富编辑、结构化属性、Query、链接、独立批注、模板及内网使用。选型不以旧实现或已有评审投入为约束，也不把产品缩减成源码加 PDF。

## 1. 先读哪几份

| 文档 | 负责内容 |
| --- | --- |
| [REQUIREMENTS](REQUIREMENTS.zh-CN.md) | 25 组需求、五项导航/列表细化、全类型结构化能力澄清、原 21 项损害映射，以及可替换技术的边界 |
| [SOURCE](SOURCE.zh-CN.md) | 原生元数据、身份/类型声明、字段与 schema 的前轮候选；精简范围按 REQUIREMENTS §5.2 重审 |
| [CORE](CORE.zh-CN.md) | `.typ` 存储、版本与环境、权限、同步、事务与恢复 |
| [EDITOR](EDITOR.zh-CN.md) | 流式阅读、富编辑/源码、普通表格、映射、批注、无障碍 |
| [FEATURES](FEATURES.zh-CN.md) | 链接各形式、潜在提示、Query/任务/视图、模板与输出 |
| [INTEGRATION](INTEGRATION.zh-CN.md) | 规范分工、继承与替代规则、跨模块正反验收 |
| [REVIEW](REVIEW.zh-CN.md) | 原 21 项影响、固定候选审查、实际验证和残余 |

[replacements.json](replacements.json) 是机器导航；各 owner 的具名处置表是语义依据。前代 [A2 修订](../a2-final-design/REVISION.zh-CN.md) 与固定输入保留，只有被明确引用的通用义务进入新候选。不能把两代格式、字段或解析 oracle 拼接使用，也不能把前代通过状态转移过来。

## 2. 笔记里的数据

普通笔记、联系人和任务共用一个源。字段类型与校验在 schema 中定义，用户写原生值；重复联系方式使用对象数组，而不是每条事实一份带身份和类型标签的 JSON。

```typst
#metadata((
  id: "12d7c4f3-9ee6-4c36-920d-ec6af27bc654",
  types: ("person",),
  name: "张三",
  company: "云山公司",
  phones: (
    (number: "+86 13800138000", label: "手机", note: "工作日联系"),
    (number: "010-12345678", label: "工作"),
  ),
)) <weftext>

#title[张三]

= 联系记录
工作日联系，其他说明直接写正文。
```

这是标准 Typst `metadata`、dictionary、array、label 与正文语法；`id`、`types` 和业务键是应用含义，不是另一门语法。完整根声明资格、位置、重复/遮蔽、错误修复及 schema 规则见 SOURCE。声明中的 UUID 仍不证明出生、权限或真实当前版本。

上述联系人是简单示例，不是人员或其他类型的字段上限。按 [REQUIREMENTS §5.2](REQUIREMENTS.zh-CN.md#52-所有笔记类型的结构化能力)，此前因 AsciiDoc 属性承载不便而删去的能力回到需求核对范围；人员、组织、任务、论文等均可按实际需求采用多值、嵌套和关系。常用界面保持简洁，必要机制重新论证，具体 schema 在技术重写中落实；SOURCE §7/§9 的旧精简处置不再限定产品范围。任务详情写正文，属性写原生值，表格/看板/日历共享同一任务。新笔记无需 YAML front matter、嵌入 JSON 属性块或私有双链标记。独立批注、配置与内部协议仍可使用 JSON。

## 3. 原生优先的边界

| 需求 | 原生机制 | 应用职责 |
| --- | --- | --- |
| 标题、1–9 层及更深合法标题、段内标题 | `title`、`heading`、标准标记与 show rules | 大纲、同段交互和准确无障碍层级 |
| 粗斜删、列表、表格、公式 | 标准元素和 math | 控件、窄回写、复杂内容的源码入口 |
| 业务数据 | 原生值、dictionary/array、准确适用的标准构造函数 | schema、缺失/错误、来源和字段权限 |
| 链接 | `link`、label、ref | ID/路径/库地址、同目标转换、出入链与可选潜在提示 |
| Query/多视图 | 原生数据载体 | 全库求值、ACL、完整性、分页和真实 Action |
| 模板与输出 | 函数/import、set/show、原生编译 | 获权输入、转换、下载和损失确认 |
| 独立批注 | 独立侧文件与原生内文配置 | 版本、范围、目标读取与写入资格 |

使用官方引擎，不另造 Typst parser 或解释器。合法复杂语法与富编辑资格分开；宏/生成内容无法安全回写时给明确来源入口，不以显示文字反推。Typst 文档内 `query` 也不代替跨库 Query。

流式阅读由 EDITOR 的明确后端和交付矩阵负责，不能等上游 HTML 自动补齐。官方 HTML 仍属实验能力，普通笔记不能只剩 PDF。LaTeX/Mermaid 兼容通过具名提供者保留范围与损失；真实 DOCX/XLSX 输出走转换能力，不把分页表格称为工作簿。

## 4. 证据与完成边界

这些文件是目标设计，并未将早期产品原型迁移为 Typst。实际完成状态以 REVIEW 为准：文档检查证明导航/双语等机械条件，官方编译示例证明所示语法，研究原型证明其有界能力；它们均不等于真实产品的编辑、权限、崩溃、同步或无障碍验收。

固定 S 的 49 份 snapshots 与 `inputs.json` 原字节保持。原 21 项按实质义务逐项重新评估，旧结论保持原固定对象。作者之间交叉讨论之后，另由非作者核固定候选；完整 Pro 终审、实施与发布不因本次修订自动启动。

## 5. 官方依据

- [官方 Rust 实现](https://github.com/typst/typst) 与 [World 接口](https://docs.rs/typst/0.15.1/typst/trait.World.html)
- [Metadata](https://typst.app/docs/reference/introspection/metadata/)、[Dictionary](https://typst.app/docs/reference/foundations/dictionary/)、[Array](https://typst.app/docs/reference/foundations/array/)
- [Title](https://typst.app/docs/reference/model/title/)、[Heading](https://typst.app/docs/reference/model/heading/)、[Link](https://typst.app/docs/reference/model/link/)
- [HTML 当前边界](https://typst.app/docs/reference/html/) 与 [文档内 Query](https://typst.app/docs/reference/introspection/query/)
