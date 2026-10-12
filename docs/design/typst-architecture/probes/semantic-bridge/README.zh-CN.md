---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# Typst 0.15.1 语义桥有限原型

状态：2026-10-12 实际研究实验；不是产品实现、完整 E1 关闭或编辑器/renderer 验收。固定 Weftext 输入 `d167ae8c1d0346faf62e8e0887cf789de0bf0b89`；产品实现未修改。观察对象为[官方 v0.15.1 源码](https://github.com/typst/typst/tree/v0.15.1)。

实际执行 11 个受控样例，分别用 observer on、observer off（仅 World 请求和最终 Frame 对照）、官方发布 CLI `typst 0.15.1 (9dfd3a08)` 编译。每例三份 PDF 逐字节相等；on/off 的 World source/file/font 请求序列逐项相等，最终 Frame（原生遍历序列、Tag、glyph span、坐标）相等。结果见 executed-summary.json。固定时间戳 0，忽略系统字体，用官方嵌入字体；这不覆盖用户字体或任意项目。

朴素 visit_show_rules/realize 的直接日志不能提供完整最终轮：默认样例第 3 个稳定轮只实际执行 1 个 realization，其余内层结果来自缓存。补丁新增独立的 tracked `Sink.probe(EcoString)` push-only 观察槽，并在原有 Sink.extend/parallelize 顺序中合并；comemo 在缓存命中时重放这些观察效果。稳定轮 subsink 得到 4 个 realization（其中 3 个来自缓存重放），并实际捕获 strong 内层输出。repeat-include 稳定轮得到 7 个，组合样例得到 42 个。未禁用缓存、未重新执行 show、未改变 Content/Style/计数算法。该旁路会增加内存和 CPU，未测性能。

捕获点：evaluate Content；prepare 后带原生 Location 的元素及真实 effective level/outlined/numbering/direction；recipe/builtin 返回 Content 和 recipe span；realization 返回的有序原生 Content/Style 数组；原生 Tag start/end 内容；每轮 subsink；成功编译最终 Introspector 与原生 Frame。源范围由已返回的 Source/官方 parser 查出，未新增 World 读取。样例 final trace 保留在执行输出目录；只有 stable=true 的成功轮被提取为 .final-trace.json，早轮仍留诊断日志。

样例覆盖 default heading/paragraph/strong；heading 加 PREFIX/SUFFIX；heading 被 show 删除；strong 被 show 去除；同文件两次 include；循环生成 heading；逐 label 同行 heading/list/table/inline+display math；变量正文重复；Paged target；SVG 资源；计算字符串拼接。show 删除 heading 时最终 query 仍有 heading、显示无标题；自定义 show 新增文字完整捕获。两次 include 保同 physical span、独立 Location、最终 counter [1]/[2]，没有 root writer。默认 heading/strong 有原生 tag、builtin return 和有效样式关联；任意 recipe 保实际返回内容，不自动冒默认 role。

明确的来源反例：`left="FromA"; right="FromB"; text(left + right)` 得到 `FromAFromB`，Content/Frame 唯一 span 只指调用 `text(left + right)`。这层观察不能恢复两个字符串的值流来源。可保守标 generated/no writer；需要完整来源依赖时必须增加官方 eval 的构造/值流观察，并使事件同样随 memoized 结果重放。Span 唯一、字串相同或一次出现均不授写资格。

此实验证明有限案例中，稳定 post-show 的原生阅读结构、样式、tag 与物理来源可以在保留官方编译结果和缓存的情况下观察。它不证明任意值流/多来源闭包、完整依赖与授权、表格全部拓扑/数学朗读、任意布局 reading order、透明自定义规则的语义恢复、富编辑逆映射、HTML/AT/320px renderer、IME/Undo、性能或跨版本维护成本。普通段落仍有原生 paragraph/ordered children 可供流式适配，不把 AST/query 或 PDF 抽文冒 post-show 阅读树；typed adapter 和局部 island 策略仍须实现与验收。

可复现文件：observer.patch（仅 12 个官方文件/新增观察模块）、实际 Cargo.lock、fixtures/、纯 Python 标准库 run_probe.py、version.json、精简 `executed-summary.json`。不附官方 source tree、target、下载缓存或海量 trace。

复现：下载固定源码，校验 version.json 的 archive SHA256；在干净源码根执行 `git apply /path/to/observer.patch`，复制本目录 `Cargo.lock` 覆盖官方 lock。排除的仅为 docs/tests workspace 与三个 test-only typst-dev-assets devdep，避免无关 Git 测试资源下载；runtime依赖固定于附带 lock。构建命令如下（引号按 shell 调整）：

```text
cargo build --locked -p typst-cli --no-default-features --features embedded-fonts --jobs 6 --config 'profile.dev.package."*".opt-level=0' --config 'profile.dev.debug=0'
python run_probe.py --instrumented-cli /path/to/source/target/debug/typst --official-cli /path/to/official-0.15.1/typst --output /path/to/probe-results
```

Windows 可执行文件为 typst.exe。公开 runner 已用同一实际 binary 与独立 fixtures 路径再运行，11 例通过；patch 在从官方 zip 提取的干净原文件上 git apply --check/git apply 成功，结果逐文件等于实际执行源码。完整 source archive/patch/lock pins 在 `version.json`。
