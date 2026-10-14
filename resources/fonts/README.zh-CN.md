# 离线 CJK 字体

此目录包含未修改的官方 Noto CJK 静态 OpenType 字体。原生编译与浏览器使用同一组文件，无需安装系统中文字体或连接公共 CDN。[manifest.json](manifest.json) 记录文件大小、SHA-256、官方提交、版本、地区、字重、原始下载地址和实际字体表检查结果。

| 职责 | 字体 | 交付字重 |
| --- | --- | --- |
| 正文、标题、列表、表格的汉字 | Noto Serif CJK SC / TC | Regular 400、Bold 700 |
| 无衬线文档与界面 | Noto Sans CJK SC / TC | Regular 400、Bold 700 |
| raw/code 的汉字与等宽候选 | Noto Sans Mono CJK SC / TC | Regular 400 |

SC 对应中国地区默认字形，TC 对应台湾地区默认字形。这些是完整语言字体，均覆盖基本简繁汉字、中文标点和中英混排；不是仅保留测试字符串的子集。不承诺全部 Unicode。Mono 的 ASCII advance 为 500、汉字及所检查全角标点为 1000（unitsPerEm 1000），保持半角/全角关系。实际 tab、缩进、复制与富编辑仍消费原文，不能把所有字符压成单一宽度。

Sans/Mono 固定 [Sans2.004](https://github.com/notofonts/noto-cjk/tree/523d033d6cb47f4a80c58a35753646f5c3608a78)，Serif 固定 [Serif2.003](https://github.com/notofonts/noto-cjk/tree/9b0f1436e455d902de067a2501422e5dc71ad16b)。各字体在 SIL Open Font License 1.1 下再分发；随产品同时提供 [Sans/Mono 许可](LICENSE-NotoSansCJK.txt) 与 [Serif 许可](LICENSE-NotoSerifCJK.txt)，字体内原版权信息保持原样。上游 [第三方说明](NOTICE-NotoSansCJK.md) 含历史 Apache 字样；本文所交版本的许可由固定提交的 LICENSE 与字体内元数据确定为 OFL 1.1。

数学结构继续使用有效 OpenType MATH 字体 New Computer Modern Math。Typst 内嵌字体的 [原始许可说明](NOTICE-TypstAssets.txt) 来自固定 `typst-assets 0.15.1`，包含 New Computer Modern、DejaVu Sans Mono 与 Libertinus 的实际许可。[内嵌字体清单](builtin-manifest.json) 记录这 17 个文件的实际字体版本、字重、样式、覆盖数量、数学表、SHA-256 与官方固定源；引擎将它与 CJK 清单合并交付。原生编译保留全部原始字体；浏览器除下述两个明确派生版本外共享原字节。NewCM10-Regular 的许可为 GPL 3 或更高版本加 Font Exception / Distribution Exception，其他 NewCM 字体为 GUST 1.0；不要从家族名称推断同一许可。此目录的汉字字体是正文、raw 与数学中文字串的候选，不能替代整个公式的数学字体。作者指定字体、`fallback: false` 和自定义原生规则仍按 Typst 求值；默认集合不能用来掩盖作者选择中的缺字。

Chromium 的 OpenType 校验器拒绝原 NewCM10 Regular/Italic 的 CFF 提示程序。网页只对这两个字重/样式使用改名后的 `WeftextWebSerif-*.woff2`：[网页清单](web-manifest.json) 记录原始输入、实际派生字节、变换与完整几何比较；[修改及许可说明](NOTICE-WeftextWebFonts.txt) 区分用途。派生过程展开 CFF 子程序、移除提示、改名并编码为 WOFF2，保留全部字形、cmap、轮廓、度量及其余 12 个 SFNT 表；提示移除可能改变逐像素栅格化。CSS 的 New Computer Modern 是从原生家族到同几何网页字体的明确别名，内部派生名称是 Weftext Web Serif。完整原始输入在 `web-sources`，许可条款包括[原始说明](NOTICE-TypstAssets.txt)与 [LPPL 1.3c](LICENSE-LPPL-1.3c.txt)。用 [requirements-web.txt](requirements-web.txt) 中的固定工具执行 `python resources/fonts/derive_web.py --check` 可在内存中完整重建并比较；只有字体维护需要这些额外生成工具。

浏览器通过获权同源 `/fonts/<文件名>` 加载实际资源；[fonts.css](fonts.css) 只声明字体文件和真实字重，不写文档样式事实。正文/raw/数学文字的映射属于最终原生投影的宿主适配。字体加载失败须准确显示呈现状态，保留输入与草稿，不能以 CSS family 名称代替交付成功。

使用标准 Python 检查资产：

```text
python resources/fonts/verify.py
```

默认检查比较 CJK 文件与许可的固定 SHA-256、原生 name/cmap 表和 Mono advances，并核对全部 27 个浏览器声明与实际字体清单。可额外指定 `--builtin-dir <typst-assets-0.15.1/files/fonts>`，检查固定 compiler 内嵌字体的实际字节、版本与 MATH 表。它证明测试字符映射到非零 glyph ID，不证明所有 glyph 的实际排版、浏览器加载、公式组合、PDF 嵌入或系统 IME。真实原生编译与平台验收分别运行；未运行的 Desktop、WebUI、Server/CLI、Office 与辅助技术场景不因该检查通过而关闭。
