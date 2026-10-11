---
source_language: zh-CN
translation_status: source
---

[English](LINKS.md)

# 链接地址与形式转换

状态：未实现的设计候选；本文件唯一拥有四种地址的区别、库根地址 codec 与转换流程。D2 继续拥有原生语法，D3 拥有身份和生命周期，D6 拥有库存/权限/提交，D8 拥有 Draft，D9 拥有导出。这里不增加新的链接 parser、身份目录、权限或提交体系；原全局问题仍开放。

## 1. 四种形式和一个默认值

| 形式 | 定位依据 | 保证与适用范围 |
| --- | --- | --- |
| 稳定 ID | 原 D3 完整 NodeRef，沿原 n1.W.N 编码 | 同 Workspace 内移动、改名不改目标身份；删除不能转指同名新笔记。 |
| 源相对路径 | 原生 D2 求值上下文的路径基准 | 普通 root 笔记相对于其所在目录；源或目标移动可能改变解析。 |
| 库内完整路径 | 显式 WorkspaceId 加从库根开始的完整路径 | 源笔记移动和同一库整体迁址不改路径含义；目标移动/改名仍需更新。 |
| 本机绝对路径 | 明确设备文件 URI | 只用于当前本机实际可访问的映射；不承诺跨设备或 Server 客户端可用。 |

库在此精确指 Workspace，不是同名文件夹或一个新 identity domain。跨库地址必须带目标 WorkspaceId，不能在当前库同路径自动找替代。新建内部笔记链接的设备默认形式为稳定 ID；设置只影响此后明确创建的链接，不批量重写旧源码。标题/别名/文件名是选择器的搜索项，选中实际目标后才构造地址；显示文字独立保留。

“最短有效路径”是相对路径的简化操作，不是第五种寻址协议：由已验证目标生成同一实际基准下最短规范相对路径，验证解析结果不变；绝不在全库按 basename、标题、别名或最短唯一后缀搜索。分享短网址涉及服务和分享授权，不包含在此操作中。本机绝对路径、库内完整路径和 Web URL 的根不能混用。

## 2. 沿用原生语法与闭合目标编码

普通 AsciiDoc 文档间引用用原生 xref，普通文件/URI 使用原生 link；D2 先按其完整语法解析，随后才验证目标 codec。[原生文档间引用](https://docs.asciidoctor.org/asciidoc/latest/macros/inter-document-xref/) 和 [原生文件链接](https://docs.asciidoctor.org/asciidoc/latest/macros/link-macro/) 继续适用。不增加双中括号语法，也不重解释普通以斜杠开头的路径为 Workspace root。

```adoc
xref:../资料/白云山.adoc[白云山]
link:weftext-workspace://<WorkspaceId>/<encoded-path>[白云山]
link:file:///D:/notes/report.pdf[报告]
```

第二行是语法示意，尖括号占位不属于合法存储值。库根 URI 的唯一 codec 为：scheme 精确 weftext-workspace；authority 精确 D3 lowercase UUIDv4 的 WorkspaceId；不允许 userinfo、port、query、缺 authority 或空 path。path 由一个起始斜杠和非空分量组成，分量代表 Core 库存中的真实库内路径；可选 fragment 表示同一目标的文内锚点。它是 Weftext 地址语义，不声称普通 AsciiDoc 工具能独立解析库身份。该新 codec 只在已验证 managed2 格式资格下启用；真实 managed1/BaselineOnly 仍保留普通 native link 及原文，不在读取旧记录时自动取得库根解释。启用需明确的格式转换与完整 Source 预览，原 saved/planned 记录按原 profile 恢复。跨 Workspace 的稳定 ID 仍须满足 D3 原引用/复制约束，URI 不扩张跨库写入或 identityMap 权限。

生成器将每个路径分量的精确 UTF-8 字节按 URI unreserved 集保留，其余百分号编码为大写十六进制；分隔斜杠原样保留。接收时只在 URI 这一层解码一次，拒绝畸形转义、非 UTF-8、NUL/control、空分量、dot/dot-dot、反斜杠以及编码后的路径分隔符。不会继续解码百分号所得文本、正规化 Unicode、折叠大小写或把加号当空格；非规范存储 spelling 拒绝 codec，但 Source 原文保留可修复。先原生解析/替换得到 nativeTarget，再运行此 codec，不能对源码另做百分号或宏预处理。fragment 单独按同一 UTF-8/unreserved 规则编码、一次解码为准确 author-anchor 字符串，空 fragment 与无 fragment 不混同；生成器不生成空 fragment。点分量限制只针对 path，不把合法 anchor 名重新当文件路径。

路径不得穿越 Workspace 边界或经符号链接/别名绕过原 Core inventory 规则。真实存储不支持该名称、命中已有 alias 冲突、占位文件未 materialize 或作用域不可证明时均不可生成可用转换；不能用字符串规范化证明实际文件。file URI 由受支持的本机平台 URI/path 库生成并回读验证，不手拼盘符/UNC；源和目标不在可证明的同一相对路径域时，相对形式不可用。

## 3. 一次解析不等于新的身份事实

D2ProductInline3 已有 link.nativeTarget、xref.nativeTarget/refid/path、label/nativeAttributes/slotOrigins。四种形式保留这些实际槽，不扩宽 D2IdentityAdapter1。只有原合法稳定 ID target 才可产生其 node_link arm；库根、相对或本机路径保持 adapter=null，即使本次导航已获权定位某 Node。实际导航/转换由原 Core 库存及受保护 FileObjectBinding、NodeRef、lifecycle、SourceObservation 和授权证明目标，绝不从文件名、文件里的 ID 字符串、hash 或缓存索引猜身份。

D7 core/document-link 的 asserted edges、D3 durable inbound 与 identityMap 重写仍只消费原已识别身份引用。UI 出链列表可以显示普通路径链接，但必须标明形式；当前可解析的路径关联不是稳定身份边或完整的永久反链证明。ID→路径转换预览必须说明稳定关联能力改变，保留原 effects/失效机制，不能声称只是无语义变化的缩写。未知/未获权路径保留字面普通链接，不能自动创建笔记。

当前 D2IdentityAdapter1.node_link 只有 NodeRef，没有 author-anchor 成员。因此带 fragment 的链接仅在现有目标形式能够完整表达且同目标锚点可证明时转换；没有该能力的 ID 选项明确不可用，绝不删 fragment 或私造 n1 后缀。本文不将它称为 Node+anchor 支持；具名扩展 adapter、D3 引用/复制和 D7 edge 模型属于后续单独增量。普通原生 xref 的锚点能力完整保留，自动标题锚点仍有原生改标题失效行为，不伪称 durable Locator。

## 4. 右键转换与复制

右键、触屏长按和键盘命令提供同等的“转换链接形式”“复制为”及“简化相对路径”。菜单只对当前链接 occurrence 工作，选项显示当前形式与不可用原因。未知或未获权目标使用原非披露结果，不能以禁用文字泄漏真实隐藏路径、存在性或标题。默认不修改显示文字、label 格式及仍适用的原生属性；跨宏形式无法无损保留的属性必须显示具体已获权差异并拒绝快捷转换，用户仍可完整 Source 编辑。

转换算法如下，全部依赖绑定同一实际源版本/Draft serial 与授权 cut：

1. 在原 disclosure gate 后读取确切 source occurrence 和现有 D2 投影；要求原 D8 完整 Source read/write 资格。include、generated、multi-origin、不能唯一逆映射或部分字素的 occurrence 不取得此快捷写资格。属性替换产生的地址若无唯一安全字面编辑位置，拒绝快捷转换，不把修改一个共享属性伪装成只改当前链接。
2. 沿原生 evaluation 的真实路径基准定位原目标。对可转换受管目标，证明完整 NodeRef 与当前 binding；普通外部目标则需要原允许的本机/资源路径解析资格，不擅自授予 managed ID。涉及 fragment 时另外证明同一目标源的准确唯一 author anchor；缺失/重复/无法验证时保留链接并使无损转换不可用。只比较字符串相等或同名锚点不够。
3. 生成所选形式，并使用同一原生 parser 和实际 resolver 逆向验证目标与 fragment 相同。不同形式允许未来移动行为不同，预览说明该差别。路径的生成/显示另需原位置/结构披露资格，Node 可读不等于可读父目录或本机路径；无此资格只能使用可披露的形式。库根地址跨库读取仍走原跨 Workspace 门，不能因 URI 携带 W 就授权。
4. 在同一 Draft 构造完整 proposed Source，保留未选字节、显示文字、适用属性与锚点。完整重解析/原 SourceTransform 证明其效果，使用原 D8→D6 prepare/save；不得调用禁止新增宏的窄文本 patch 分支。转换是一个 Undo 单元，取消不写文件。
5. 在最终读屏障重验源/目标/路径绑定/锚点/权限/格式/registry 与实际依赖；改名、替换同路径文件、ABA、撤权、同步缺口或 Draft 变动使准备失效。重新计算需要新准备，不能沿旧证据静默改指另一篇笔记。

“复制为”只复制目标地址或完整 AsciiDoc 链接，菜单明确两者；完整链接沿同样的原生编码和 label 保全。它不要求 source 写权、不创建 Draft 变更/receipt、不发布分享，也不自动保存。仍须有 source/目标/地址的读取披露资格，并在剪贴板写入前重验实际资格。复制相对链接明确标注其基准，粘贴到其它文档不会自动获得原目标语义；在 Weftext 内有真实来源上下文的粘贴可走显式重定位预览，纯文本剪贴板不可推断隐含身份。原样复制已获权的 source 字面是独立的普通复制动作：即使目标断开、缺失或不可读也可复制当前原文，但不附解析成功保证、不查询或追加隐藏目标信息。它不能被“转换后复制”的目标验证门误阻断。

## 5. 库与设备边界

库内路径来自 Core 的当前受控库存相对地址，不是标题组成的逻辑树，也不作为可编辑 metadata 再存一份。完整移动同一 Workspace 到别处只改变实际根绑定，URI 内 WorkspaceId 与相对路径保持；fork/copy 到新 Workspace 不伪称同一库。相对路径使用 D2 实际原生求值基准，include 或已配置 prefix 不得被 UI 重新解释为假定的根文档目录。

本机绝对形式只在调用设备确有已验证文件映射及路径披露权限时提供。WebUI/远程客户端不能将 Server 的磁盘路径交给用户作本机地址，不能假装客户端 file URI 可读取服务器文件；正常远程导航/下载仍走原 Server 入口。库根 URI 可在获权且已连接的本地或 Server Workspace 上定位；未连接库显示不可用，不自动联网连接、猜同名库或生成分享链接。

copy/template/import 的新源不能盲复制相对目标并称保持引用。所有仍要保持的路径链接先在原受权 source context 中解析为确切目标，然后在已冻结的新 source 位置重新编码/验证；map 内目标由原 identity map 确定实际新对象，再获取其真实路径。跨库 URI 的 authority 只在原操作有明确 Workspace 映射且相应目标也实际映射时改写，禁止全局替换 UUID。没有位置/目标证明、目标域不支持相对路径或无法完整保留 fragment 时进入原显式 loss/拒绝路径。所有变化进入同一完整 preview、SourceTransform、C/Q 与最终 seal；不新增自动重写后台服务，也不把路径列表当持久身份映射。

## 6. 导出边界与后续路由

普通原生 xref 的既有转换行为保持。exact-source 导出仍输出准确源字节、generationPolicy none，不因默认链接设置改写文件。已有 generation provider 只有在其声明能力、实际冻结的输入和受控输出足以证明链接目标/fragment 时才可生成对应地址；不从源文件名猜导出文件名，不重写原笔记。

将任意稳定 ID 或库根链接完整转换为离线文档集的相对地址，需要按输出集合冻结 occurrence→获权目标/anchor→受控输出名/fragment 的实际路由证据。当前 D9DocumentRenderBinding1 和 ExportPlan4 的 typed evidence union 没有承载此完整路由表；本候选不塞入自由 evidence pin、不扩宽闭合 Plan、不声称通用离线路由已完成。无法由原已定义 profile 证明的转换明确 unavailable 或进入原用户确认的损失报告；不得输出外观正常但指向错误对象的链接。后续由 D9 的版本化 render-binding/typed pin 承载、consumer 与 confirmation 联合设计。此后续项不阻止源内四形式转换或原有精确源码导出。

## 7. 设计验收

以下均为设计要求，产品 UNRUN。

| 编号 | 正向与反例 |
| --- | --- |
| LINK-01 | 同一目标在可用四形式间转换，label/适用属性不变；普通斜杠起点不被解释为库根。 |
| LINK-02 | 源移动只影响源相对形式，库整体迁址保留库根形式；目标改名/替换令路径准备失效，ID 不转指同名对象。 |
| LINK-03 | 同名笔记先显式选择；最短路径不查全库后缀；跨 Workspace 无映射不改 authority。 |
| LINK-04 | 百分号/空格/Unicode/大小写/加号保持精确语义；重复解码、encoded slash/backslash、dot traversal、userinfo/port/query、alias/symlink 越界拒绝。 |
| LINK-05 | 成功转换保留原准确 fragment；ID 无 anchor 槽、同名/缺失锚点、无逆映射时禁用，不丢锚点。 |
| LINK-06 | 可读 Node 但不可披露位置时不输出路径；Server 不泄本机盘路径；未连接库不猜替代。 |
| LINK-07 | 一次转换一次 Undo，取消/撤权/ABA/Draft 变化不提交；复制仅改剪贴板，不要求写权或发布分享。原样复制断链字面仍可用；转换后复制仍须目标资格。 |
| LINK-08 | 共享属性/include/生成来源保留原文并提供不可用原因；完整 Source 路径不借窄文本 writer 插宏。 |
| LINK-09 | ID→路径说明稳定边语义变化；D7 asserted graph/Ref inbound 不把路径瞬时解析提升为身份。 |
| LINK-10 | copy/template 新目录重定位、fresh map 与 C/Q 同决议；无证明按原 loss/拒绝，不悄悄变目标。 |
| LINK-11 | exact-source 原字节；未支持的离线路由不伪装成功，不给旧 Plan 私加成员/pin。 |
