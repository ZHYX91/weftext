---
source_language: zh-CN
translation_status: source
---

[English](README.md)

# 编辑来源与保存闭环：有界原型

本原型验证“一份 Typst 源 → 同次最终 Html 文字的真实来源 → 局部修改 → 同一 Draft/Undo → 独立实验文件保存”。它不是完整编辑器、正式 Core 或五档产品验收。2026-10-13 的 [results.json](results.json) 记录 **15 个真实编译/Windows 文件案例通过**；[test_session.mjs](test_session.mjs) 另验共享历史、过期结果/范围拒绝与模拟 composition 分组。后者不是真实系统输入法测试。

## 范围与结构

- [03-source-origin.patch](03-source-origin.patch) 从 Typst 0.15.1 同次最终 Html text 的原生 Span，通过官方 Source AST 取准确字节范围、祖先与全树语法种类。应用不从 DOM 文本相等反猜地址。Span 本身不是写权限。
- [server.py](server.py) 仅允许独立实验目录的 `note.typ`。可写 profile 限于整篇没有代码且来源唯一的字面正文/标题/强调/列表；重复 include、show、生成函数、嵌套代码和表格保源码入口。代码旁边的普通字面内容也保守降为只读投影，这是原型限制。
- [session.mjs](session.mjs) 只持有一份源、serial、Undo/Redo；迟到编译只在源和 serial 都相等时接纳。[app.mjs](app.mjs) 实现五个有界入口：源码、保语法的简单样式、点击显示局部源码、字面文字回写、只读阅读。V03 用显式应用按钮；V04 只替换非空单行文字，转义原生标点；不是完整结构编辑。
- 保存用 Windows share-zero 同一句柄核对原文件身份/准确 Base，写入前保存 before/after，再写入/flush 与提交记录；响应重试保 operation。注入异常后重新建立 Probe 检查未决记录，第三方新稿不被覆盖。此处未覆盖生产 ancestor/reparse/hardlink 保证、目录 durability、OS 断电或进程硬杀，不能替代 CORE 后端验收。
- 浏览器另做了 V02 语法常显、V04 中文回写、切 V01 验源码、Undo/Redo、V03 局部修改和保存检查；最终版本的阅读只读与错误输入边界见 [ui-results.json](ui-results.json)。页面使用宿主中文字体，不证明字体离线打包。实验数据不进入公开仓库。

## 重现

先按 [html-flow](../html-flow/README.zh-CN.md) 在干净官方 v0.15.1 源码应用 01、02 补丁及其 Cargo.lock。只在这个隔离源码目录继续应用本目录 03 补丁，另设 target；不修改原证据的源码或二进制。

```powershell
$editorProbe = (Get-Location).Path
$typstSource = 'C:\typst-editor-source'
$editorTarget = 'C:\typst-editor-target'
git -C $typstSource apply --check (Join-Path $editorProbe '03-source-origin.patch')
git -C $typstSource apply (Join-Path $editorProbe '03-source-origin.patch')
cargo build --manifest-path (Join-Path $typstSource 'Cargo.toml') --target-dir $editorTarget -p typst-cli --locked
$editorBinary = Join-Path $editorTarget 'debug\typst.exe'
python test_probe.py --binary $editorBinary --output C:\temp\editor-results-new.json
node test_session.mjs
python server.py --binary $editorBinary --data C:\temp\weftext-editor-demo --port 8791
```

`--data` 必须是空的专用实验目录或本实验自己标记的目录，不传真实笔记库。打开 `http://127.0.0.1:8791/`；Ctrl+C 停服务。测试只使用临时副本；输出目录须先存在。浏览器草稿储存在该本机 origin 的 localStorage，恢复时仍保原 Base；它不是加密、多用户或跨设备存储。

## 尚未证明

真实 CJK IME/AT、跨格式选区、完整光标/选区保持、V02 Enter/结构语法编辑、V03 无按钮的完整按需体验、V04 空文档/删除最后文字/结构插入、表格和代码混合文档的富编辑、样式视觉一致性、长文性能、正式 Save As、托管 Node/权限/同步、Office 输出均未在这里完成。结构或不可证文字编辑仍走源码。主产品应选择成熟编辑会话组件并逐步实现官方 AST 逆编辑；不能把本原型 DOM 代码直接当产品后端。

本实验支持继续实现首个纵向切片，不能据此宣布所有来源都可写或五档已经完成；实际失败按具体机制修复，不自动重开全套设计终审。
