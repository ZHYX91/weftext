---
source_language: zh-CN
translation_status: source
---

[English](schemes.md)

# 首批识别专用方案

`schemes.py` 只把实际识别／DOCX 证据变成未确认候选，不执行 OCR、不持久化业务字段、不执行识别文字。它是 v0.6.2 的组成模块；单元检查、真实离线 worker 检查、产品 HTTP／界面验收分别记录，不相互代替。

## 固定范围

- `invoice-cn-mainland-digital-ordinary-single-page@1`：中国大陆电子发票（普通发票）标准单页，无特定业务标签；PDF、PNG、JPEG。实际标题必须匹配，多页或多个发票标题不跨票拼接。发票号码按准确 20 位 ASCII 数字文本保留前导零；日期仅支持完整、有效的公历 `YYYY年M月D日` 或 `YYYY-MM-DD`。支持购销方名称／纳税号、合计金额／税额／价税合计及明确写出的币种。金额用准确十进制，明确支持两位小数及合法英文千位分隔，不猜小数逗号。`¥`／`￥` 不单独证明币种。只作语法提取，不判断税务真伪或业务有效性，不承诺明细结构化。
- `official-document-cn-mainland-hans@1`：中国大陆简体中文公文候选，DOCX 结构或 PDF／PNG／JPEG 实际识别。显式机关、文号、日期、主送标签产字段候选；标题／编号文字仅产待核候选。DOCX 标题、层级、原生编号只能由实际 DOCX adapter 提供证明。普通数字段首不能因此成为已证明列表。

两个方案均要求本次处理设置明确为 `language: "zh"`、`script: "Hans"`、`region: "CN"`；缺项返回未知，不从界面、汉字或执行机器猜测。地区／语言不匹配为不适用，不偷偷改选方案。

发票范围依据国家税务总局[2024 年第 11 号公告](https://fgk.chinatax.gov.cn/zcfgk/c100012/c5236067/content.html)的票种、基本字段及 20 位号码，并核对该页附件 1 的普通发票票样。公开仓库只含自有 TEST 样例，不复制真实票据。官方附件的核查不等于对样例或税务结果的认证。

## 内部输入和结果

调用 `propose(scheme_id, recognized, options)`；`recognized` 直接使用离线 worker 的 `input`、`status`、`pages -> regions -> blocks`。每个 block 保准确 `id/text/sourceMethod`、原页 `bbox`、可用 words。DOCX adapter 使用 `docxBlocks`，每个块须有原始 `blockIndex`，样式、层级和编号由真实包读取，不接受普通文字伪造结构证明。适配层在 Core 权限及准确原件冻结之后调用，不能把客户端自报的结构当作获权原件。

每个字段均为未确认候选，`status` 区分 `present`、`missing`、`duplicate`、`ambiguous`、`parse-failure`。存在唯一可解析值时 `value` 是字符串；金额、号码、日期均不变成浮点数。候选保 `raw`、`parsed` 和准确 `sources`。来源含原始摘录、block 身份、原页／区域或 DOCX 原块索引，`start/end` 明确采用 Unicode code point。坐标优先使用可与原文顺序严格对齐的 word 几何，否则仅标原 block 精度，缺坐标保持 unavailable。

购销角色仅用明确标签及可证明的标准左右栏几何；没有几何、两个角色挤在同一未分解块、跨两栏的文字均保歧义。公司名称中的“购买方／销售方”不作为角色标签。不同的两个有效发票号码无法分组时，方案拒绝输出混合字段，仅保未归组号码来源。无标签合计行仅在原页明确的金额／税额列及同一行几何可证时提取。重复值不自动去重；算术不符只产问题，不改票据。部分识别必须保覆盖警示。

公文编号选择为 `keep`／`remove`／`rebuild`，默认 keep。只有本次明确 `selectedStructureIds` 才产修改建议；旧／不存在的选择拒绝。rebuild 去掉本次选中的纯文字候选前缀，标记需由作者适配器应用一次原生编号；不会先把数字写进正文再重复编号。DOCX 原生编号与段落文字分开，结构证明不能证明正文里的数字前缀冗余，因此保留其字面正文。没有公文标记为不适用，只有编号文字则类型未知；发票不套用公文方案。原始证据与文档都不被方案模块改写。持久来源和确认字段映射遵循 SOURCE／CORE；该 JSON 不是新的可编辑作者格式或事实库。

`fields` 是以已声明字段名为键的字典。每个字段有 `valueKind`：`text`、`decimal` 或 `date`。发票号码及纳税号始终为文本，只有金额／税额／合计为十进制，开票／成文日期为日期。确认消费者须将语义类型匹配到既有 scalar schema 字段，不能把前导零号码变成数字。

`docx_reader.read_docx(original_bytes, expected_sha256)` 读取真实、有界 OOXML 段落样式、继承的层级及已解析编号定义。`blockIndex` 是 `word/document.xml` 中从零开始的段落顺序，含表格段落；摘录同时带 `part`、`paragraphIndex`。它不声称恢复渲染页坐标、求值 Word 域或完整 DOCX 交换。修订、活动／非文字内容、未解析编号明确标记为部分证据；拒绝加密、损坏、重复部件、超限及 DTD／实体包，不执行活动内容或外部关系。

结构项在完整原始来源外另带准确 `contentSource` 范围。已证明的 DOCX 编号携带定义依据及本次选择的 `nativeNumberingAction`（`preserve`、`omit`、`replace`），区分移除原生计数器和修改字面文字。

## 复现

```text
python -m unittest discover -s scripts/recognition -p test_schemes.py
python -m unittest discover -s scripts/recognition -p test_docx_reader.py
python scripts/recognition/build_scheme_fixtures.py --typst ABSOLUTE_TYPST --output-dir OWNED_OUTPUT
python scripts/recognition/test_scheme_runtime.py --runtime INDEPENDENT_RUNTIME --fixtures-dir OWNED_OUTPUT --evidence-dir OWNED_EVIDENCE
```

单元命令验证候选及真实 DOCX 结构。生成器用仓库许可 Noto 字体及明确选择的官方 Typst 生成自有 TEST 发票／公文 PDF、PNG、JPEG 和真正带样式／编号的 DOCX；Pillow 只做样例 JPEG 编码。运行检查真正调用独立安装的识别 worker 再运行方案，保原始结果与候选，逐项与样例清单核对声明字段。它仍不是产品权限、持久提交、界面或真实票据验收。缺依赖、部分结果或失败保持区分。

生成器保留未改变的原始正例像素，并为每种格式生成八类发票反例：错误标题、日期缺失／含糊／无效、算术不符、重复／竞争号码及多个标题。运行检查核声明结果、准确来源摘录、原件摘要、未确认状态和真实 DOCX 编号选择。可选 `--selections FILE` 按样例文件名记录明确原页区域及 worker 支持的分割、OCR 缩放。选区运行与自动整页识别分别记录；新样例或路线不能把原先失败输入／路线改写成通过。
