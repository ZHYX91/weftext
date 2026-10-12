---
source_language: zh-CN
translation_status: source
---

[English](FEATURES.md)

# Typst 链接、Query、模板与输出

状态：**协调设计候选，未实现、未独立接受**。输入为 `af6737a057c7bb69763dfaad0a57da4652e085f3`，作者分支 `design/typst-native-20261012`。本文拥有新格式的链接地址与转换、潜在链接、Query 作者承载、模板及导入/导出消费边界；[SOURCE](SOURCE.zh-CN.md) 拥有语言/数据来源，[EDITOR](EDITOR.zh-CN.md) 拥有 Draft/交互，[CORE](CORE.zh-CN.md) 拥有身份、权限、事务、闭合适配与历史分派。所有验收均为设计 oracle，产品 UNRUN。

## 1. 一份作者源与应用职责

`.typ` 是作者源；原生 link、label/ref、metadata、dictionary/array、import、函数和 set/show 直接使用。Weftext 赋予这些值应用含义，不声称官方编译器识别 NodeID、ACL、数据库或 Core Action。普通笔记不需要 JSON 字段块、双中括号链接或新的查询代码语言。高级合法 Typst 无法富编辑、编译失败或缺排版资源，都不取消已获权的准确源码读取与普通 Source 保存路径；完整业务操作另须自己的语义资格。

设备偏好、候选索引、查询结果、运行时地址和导出工件都不是第二份作者事实。Desktop/WebUI/CLI/获准 Agent 调用同一 Core；远程访问与下载经过 Server。字符串 UUID、URI、编译产物、相同标题和摘要不能授 birth、权限、当前性或写资格。

## 2. 原生链接与闭合地址 codec

### 2.1 五个选择名称、四个定位域

```typst
#set heading(numbering: "1.")
= 计划 <plan>
见 @plan。也可 #link(<plan>)[查看计划]。
#link("weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654#plan")[项目计划]
#link("../%E8%B5%84%E6%96%99/%E7%99%BD%E4%BA%91%E5%B1%B1/%E7%99%BD%E4%BA%91%E5%B1%B1.typ#route")[白云山]
#link("weftext-workspace://58c47256-bfce-4d29-a037-b83a3a644fb0/%E8%B5%84%E6%96%99/%E7%99%BD%E4%BA%91%E5%B1%B1/%E7%99%BD%E4%BA%91%E5%B1%B1.typ#route")[白云山]
#link("file:///D:/notes/report.pdf")[报告]
```

`@plan` 和 `#link(<plan>)` 是原生文内引用；不是跨笔记 ID 引用。普通 Web/mailto/tel URL 保持原生 URL 语义。正文稳定地址使用 `weftext-node://W/N`，W 和 N 都是准确 lowercase UUIDv4；可选 fragment 是目标作者 label。业务 schema 的 `node_ref` 使用同一**无 fragment**的原生 string，由 schema 在该 typed 位置生产完整 NodeRef；普通 string 恰巧长得一样仍是普通文本。不改变历史 n1.W.N、symbolicVersion1… 或旧 adapter 的解码域。

| 菜单形式 | 基准与保证 |
| --- | --- |
| ID 链接（设备新建默认） | W+N 完整身份；同库移动/改名保持对象，删除后不转指同名对象。fragment 随目标 label 改名/删除可失效。 |
| 源相对路径 | 以该 occurrence 的真实 authored source 文件目录为基准；源/目标移动可改变含义，不假定 include 位于根文档。 |
| 库内长链 | `weftext-workspace://W/<root-relative-path>`；同库整体迁址与源移动不改路径含义，目标移动需更新。 |
| 本机绝对路径 | 本机已验证 file URI 映射；没有跨设备/Server 客户端保证。 |
| 最短有效路径 | 在当前同一相对路径域生成最短规范相对路径的操作；不是标题/basename/唯一后缀搜索，也不是第五个协议或分享短网址。 |

`@` ref遵守原生编号/可引用元素要求，示例先设heading编号；无编号heading可以用显式`#link(<plan>)[...]`，不宣称任意`@label`都能渲染。示例受管目标路径是`资料/白云山/白云山.typ`，匹配唯一`X/X.typ`布局；file URI示例是普通外部PDF。

库指 Workspace。跨库必须保留目标 W 且当前已连接/获权，不猜同名库、不自动连接或发布分享。设置只影响以后明确创建的链接，不重写既有源码。可读 Node 不等于可披露父目录、库位置或本机路径。

### 2.2 URI 与 native string 两层编码

节点 URI 精确为 `weftext-node://W/N` 加可选 `#F`；authority 不允许 userinfo/port，path 恰一个 N，无 query/trailing slash/额外 segment。库 URI 精确 scheme `weftext-workspace`、authority W、起始斜杠及非空路径分量，不允许 userinfo/port/query/空路径。两者 scheme 大小写、UUID 和规范 spelling 必须准确；不接受宽松 URI 修复。片段缺席与空片段不同，应用生成器不产生且应用 codec 拒绝空片段。

每个 path segment/fragment 的准确 UTF-8 字节只保留 URI unreserved `A-Z a-z 0-9 - . _ ~`，其余用大写十六进制百分号编码；路径斜杠仅作分隔符。接收时在这一层只解码一次并要求重新编码与输入一致；拒绝畸形转义、非 UTF-8、NUL/control、反斜杠和编码的分隔斜杠。库路径还拒绝空、`.`、`..` 分量；fragment 是准确 label 字符串而非路径，不套用 dot-segment 规则。Unicode 不正规化，大小写不折叠，+ 不当空格；`%252F` 解一次得字面 `%2F`，绝不再解码。只有实际库存允许的名称可用于生成地址。

源相对 string 同样将非 unreserved 路径字节按上述规则编码，但分隔 `/` 与前导规范 `../` 保留；消费端只允许有界前导 parent 段及普通非空分量，中间 `.`/`..`、重复斜杠、反斜杠、query 和起始斜杠拒绝应用路径解释。先在原 authored source 的库存路径基准进行有界 parent 运算，再验证真实 target binding；不得走出该 Workspace、越过 ignored/unmanaged 边界或沿 symlink/alias 绕过库存。`/x`、`C:\x` 和不可识别 string 保留合法原生链接，不偷解释为库根。file URI 由受支持平台 URI/path 库生成并回读验证，禁止手拼盘符/UNC；远程客户端不获得 Server 磁盘 URI。

上述结果再作为 Typst 原生字符串参数编码：双引号、反斜线与控制字符按官方 string 语法转义，禁止拼接用户原文为可执行代码。显示文字保留原 content AST；新 plain label 可用 `#link("...")[#text("encoded text")]` 安全构造，再经官方 parser/compiler 比较文本和结构。URI 解码发生于官方语法先得到字符串值之后，不对源码预解码，不引入第二 Typst parser。损坏 codec 只影响该应用地址解析/转换，保留源字节与修复入口。[官方 Link](https://typst.app/docs/reference/model/link/) 规定 URL string 与文内 label 的不同目标形式；这里的 URI 是 Weftext 应用协议。

### 2.3 anchor、出链、入链与图

`TypstLinkTarget/1` 是闭合应用投影：`{version:1,form:"node"|"relative"|"workspace"|"file"|"native",nativeDestination:EDITOR SemanticNativeDestination/1,nodeRef:NodeRef|null,authorLabel:text|null,sourceOccurrence:TypstAuthoredLinkKey/1|null,compiledOccurrence:TypstCompiledLinkKey/1|null}`；两个occurrence槽至少一个非null，两者非null时须§2.4准确关联。nodeRef 只对严格节点 URI 由获权完整 identity resolver 生产；路径解析得到的 transient target 另留在运行时导航证据，不能填入 asserted identity arm。nativeDestination 保留原生 string/label/location/page-coordinate 类型，不用页面坐标补造 authorLabel。label/ref 是当前编译文档目标；URI fragment 必须在获权目标的当前源/依赖中证明**准确唯一** label，重复、缺失、无法证明的生成label使范围不可用，无损转换禁用。compiled-only的nodeRef仅是派生导航投影，不因非null取得stable edge。

节点身份加作者 label 已支持同目标 fragment 的本轮转换，但 label 是作者可改名字，**不是** revision-bound Locator；不宣称永久锚点或自动重锚。源内无 fragment 的 ID 导航不强加 target body read；证明 fragment 则须获准确目标来源/label 的读取资格。

出链列原生真实 link/ref occurrence 与地址形式；稳定入链/asserted graph 只消费严格节点 URI 的完整 NodeRef 及 SOURCE 已验证根作者来源。generated/include/multi-origin 链接可显示获权的编译/导航结果，标为派生，不能作为稳定作者 edge 或 identityMap rewrite 槽。路径链接必须同样提供“当前解析入链”：在获权current cut按完整源occurrence/实际path resolver构建派生index并列于入链面板的独立分组，source/target改名、移动、替换、auth或负范围变化即失效，partial scope不冒完整无入链。当前path关联标transient，不升为durable identity inbound/asserted graph，但不取消其入链导航功能。潜在提及不进入真链/graph/reference integrity。标题与别名只供候选选择，重复标题不能合并对象。链接健康只在当前获权且相关范围完整时说“全库无入链”，否则说“当前可见范围未找到”；不泄隐藏数目，不自动删孤立笔记。

### 2.4 作者链接与编译链接 occurrence

下列closed类型由FEATURES唯一拥有，实际producer是Core官方AST/source资格与EDITOR §2.2 final realization bridge，不引用不存在的SOURCE link producer。Counter路径只在绑定源内有意义，PinRef/Observation/compiled index均不授权限。

```text
TypstAuthoredLinkKey/1={owner:NodeRef,sourceVersion:SourceVersion/2,astPath:[Counter...]}
TypstAuthoredLinkCatalog/1={kind:"typst_authored_link_catalog",version:1,
 profile:TypstProfileCarrier/1,owner:NodeRef,observation:SourceObservation/1,
 sourcePin:PinRef/2,occurrences:[TypstAuthoredLinkOccurrence/1...],complete:true}
TypstAuthoredLinkOccurrence/1={key:TypstAuthoredLinkKey/1,
 range:SOURCE TypstByteRange/1,destinationRange:SOURCE TypstByteRange/1|null,
 syntax:"native_link"|"auto_url"|"native_ref",
 destination:EDITOR SemanticNativeDestination/1|null,
 qualification:"root_literal"|"root_construct"|"nonliteral"}
TypstCompiledLinkKey/1={owner:NodeRef,sourceVersion:SourceVersion/2,
 compilationEvidencePin:PinRef/2,projectionPin:PinRef/2,index:Counter}
TypstCompiledLinkOccurrence/1={kind:"typst_compiled_link_occurrence",version:1,
 index:Counter,destination:EDITOR SemanticNativeDestination/1,
 readingNodeKeys:[Counter...],navigationKeys:[Counter...],originIds:[Counter...],
 disposition:"active"|"removed_by_show"|"unavailable"}
TypstLinkAssociation/1={compiledIndex:Counter,
 authoredKeys:[TypstAuthoredLinkKey/1...],
 state:"unique_root"|"include"|"generated"|"multi_origin"|"unavailable"}
```

AuthoredCatalog观察/sourceVersion/pin逐项同源同cut；官方AST按root source order枚举直接markup的native link调用、自动URL与ref，astPath为官方完整根children索引路径，range为整个construct准确UTF-8范围，destinationRange是唯一目标literal/构造范围，无此资格则destinationRange=null、destination=null、qualification=nonliteral；不以compile后算得字符串伪造literal来源。裸link函数须静态standard binding，字符串/raw/code中相同拼写不算link；不把include、function/loop生成输出当新的根AST occurrence。不完整AST/binding枚举只能unavailable，不能complete=true。occurrences按range.start、range.end、astPath数值字典序排序唯一；每个key恰一次。include中的managed文件可独立取得自己的catalog，但不是当前root的catalog。直接native_ref的author label无编译位置时destination保label value/finalLocationKey=null；point/location未独立证明时不得造固定坐标。生成app-link无法证明typed改写闭包时copy/fork整体拒绝或走另一个显式loss提案，不靠source字符串扫描。

Compiled occurrences只在同次成功compile的最终收敛trace枚举。EDITOR实际Projection.links使用上型，index按最终逻辑link occurrence顺序连续0..N-1；同include/show/function重复产生两个逻辑链接，即使physical span/destination相同也有两个index。同一逻辑链接的换行/分页/多个Frame碎片放在同项readingNodeKeys/navigationKeys，不按字符串或坐标合并不同occurrence。两个key数组排序唯一，逐项指同Projection真正的reading/native-link节点或navigation trace；active节点的SemanticElement.link.compiledIndex必须回指该项，所有实际active链接节点都被覆盖。removed_by_show保源意图诊断但不作为可点击输出；unavailable不可伪空。Origin IDs只能来自同origin pin，不能充当occurrence index。Projection/CompilationEvidence/world/source/授权均完整对应，无法完整枚举时拒绝“完整出链/路由”。

每个compiled index恰一个association，按compiledIndex排序；authoredKeys按完整owner/ref、sourceVersion规范bytes、astPath数值排序唯一。unique_root恰一key且owner=current root，并由官方entry→return及完整catalog独立证明唯一对应；include须真实include trace（keys可为独立managed include catalog中的key，或无managed owner的空集）；generated记录真实生成trace，不能据一个template source span变root writer；multi_origin须多个准确source keys/来源记录；unavailable不用于关联写资格。消费者独立核索引、range、目标和origin关联，不能由相等文本/span猜配。

根writer、stable asserted edge与typed source transfer只接已证明root_literal/root_construct的authored槽；compiled导航/导出按actual编译occurrence。Draft转换另用EDITOR当前mapBinding+完整Draft AST/selection资格，不能让旧committed AuthoredKey认证新draft字节。导出路由的occurrence精确为TypstCompiledLinkKey/1，冻结完整Projection后独立枚举全部active可路由app link：每项恰有一个成功route或该同key的明确loss/reject，非app/removed项不冒route。重复/缺项/额外项、错误compile pin、相同rootVersion但World变化均拒，目标/actual named destination另依§8验证；路由array按compiled index排序。源内/原字节导出不消费这份compiled路由。

## 3. 转换、复制、改名与复制实例

右键、长按和键盘同等提供“转换形式”“复制地址/完整链接”“简化相对路径”，仅作用当前 occurrence，并显示可披露的不可用原因。原样复制获权 source 字面不要求目标存在/可读，也不附解析成功保证；转换后复制须重新验证目标与地址披露，仍不要求源写权、不生成 Draft/receipt/分享。

转换步骤闭合为：先当前 disclosure/source-read → SOURCE 唯一 occurrence 与 EDITOR 同一 Base/Draft serial → 获权解析完整目标 binding 和准确 fragment → 生成所选 URI/native string → 同一官方 parser 与 Core resolver 回读证明 target/fragment 不变 → 构造完整 proposed Source，保留显示 content/适用原生属性/未选字节 → EDITOR `TypstEditOperation/1` 的闭合convert_link适配作为一个 Undo 单元生成完整源，再由 `TypstEditPrepare/1` prepare/save → 最终 barrier 重验 source、target、路径、label、schema、环境、依赖和权限。路径披露不足时仅提供可披露形式；ID→路径预览说明未来移动行为与稳定 edge 改变。无法保留 body content/label/原生参数时拒绝快捷转换并给 Source 编辑入口，不压平富内容。

include/generated/shared variable/多来源或无唯一可写 target literal 的 link 不取得快捷写资格；不因编译 Span 或相同显示文字就写回。ABA、源/目标替换、撤权、改名、label/环境改变、部分同步或 Draft 变化令准备失效，须新准备；不偷偷取 latest 或转向同名目标。取消不写，一次转换一次 Undo，保存/提交仍是 Core 原事务。

move 不改稳定 NodeRef，路径链接是否失效明确可见；不存在全库自动重写服务。copy/template/import 要保持路径含义时，先在获权前像 context 定位，再用冻结的新 source 位置重编码验证；map 内目标用 Core **同一**候选 identity map 和实际新路径。节点 URI 在 typed link 槽按完整 W+N 映射，fragment 原样保留并验证新目标唯一 label。库 URI 只有明确 Workspace/目标映射才改 authority，不能全局替换 UUID。普通文字、电话号码、CEL string、未知字段和像 UUID 的字符串不扫描替换；source 全部 typed link/Field/definition 槽一起验覆盖，无法证明则按显式 loss/拒绝，full fork 不能默默漏 Node 或删未知定义。

## 4. 潜在链接的唯一算法与交互

别名的实际producer是SOURCE §7.1/7.2已安装seed中的`core/aliases`，消费准确TypstSchemaQualification及该保留FieldId，原值是保序/重复的nonEmpty native string array。不能把任意同名key、类型alias表或插件string数组当笔记别名；候选去重只作用派生index，不修改源数组。

设备级“显示潜在链接”默认关闭，开关不改源。词来自当前获权 live Node 的原标题与 schema 明确识别 aliases，排除自身；不可用 alias source 标扫描不完整。候选去重以完整 NodeRef/准确词为依据，同标题多对象保留。匹配是精确 Unicode scalar 字面匹配，无 fuzzy/case-fold/NFC/Pinyin。仅扫描 SOURCE 有唯一 root-authored 可写映射的普通可见逻辑文本；不跨片段拼词。排除代码、公式、URL、真实链接、metadata/属性、批注、生成/include/multi-origin 内容。软换行可分段绘线，但 source join/EOL 不能拼词。

匹配携同一 Draft serial/flow 的 `[start,end)` scalar range，转换前由 fresh Draft map 映射为准确 UTF-8 byte/EOL source range，两端须为 EDITOR grapheme 边界，不拆 CRLF、combining sequence、emoji ZWJ。命中半字素只作不可转换发现，不扩大选区。相同 range 的多目标是一组，不同 range 即使嵌套/交叉都保留：“白云山” `[0,3)` 与“山” `[2,3)`；“白云山南路” `[0,3)` 与 `[2,5)`，再加“山” `[2,3)` 成为第三组，须进入overflow仍可选；“白云山2026”以别名同范围出现时保留完整目标列表。数字是 scalar 例，不是 byte offset。

视觉最多两条上下分离的提示线。按 start 升序、同 start 按 end 降序分配最小可用层，前组 end==后组 start 可共层；无可用层的组进入统一密集提示与“更多匹配”，仍保留全部语义匹配，不无限堆线、不仅靠颜色。跨行按实际片段绘制；RTL 用逻辑范围与真实排版映射，不按屏幕 x 排语义。每个弹层先最多20目标，再按完整 NodeRef 规范顺序分页；候选标题/匹配原因及获准上下文可见，多目标必须用户选定，不默认第一项。

悬停、点击、键盘命令/可访问列表和触屏长按均可打开候选；Escape 返回原选区，不抢键入/IME焦点。明确“转换为链接”才按§3修改同一 Draft，证明新原生 link 指向所选完整 NodeRef、文字与未选字节不变；只读源不因目标可读而变可写。建议、扫描结果和旧证据遇撤权、命名/schema/Base变化失效。扫描可取消、可见区优先、受预算约束；预算/截断/未完状态显式，不冒全局全集。未完成扫描仍可转换一个已完整验证的候选；候选索引可重建，不同步为事实。

## 5. 跨库 Query 的原生作者承载

### 5.1 carrier、读取、保存与引用

Typst `query` 是**编译文档内** introspection，不能查询跨库 ACL、签发 ResultHandle 或执行 Action。[官方 Query](https://typst.app/docs/reference/introspection/query/) 与 [Metadata](https://typst.app/docs/reference/introspection/metadata/) 的能力用于语言/显示；Weftext Query 保留 D7 的唯一 Core DAG、CEL、类型、scope、授权、依赖、分页、订阅、预算和错误顺序。

新保存视图在 `.typ` 中使用 root-authored、直接字面原生 metadata，标签 `weftext-view-<key>`；key 为 `[a-z][a-z0-9-]{0,63}`，该前缀由本文保留，禁止 include/generated/shadowed 声明取得应用定义资格。每个 label 在准确源中唯一；重复/缺失/未知版本保留合法源并使定义 unavailable，不取首项。carrier 是 `TypstFeatureDefinition/1`，准确四成员 `{format:"weftext.feature-definition",version:1,query:<TypstQueryValue/1>,views:(<TypstViewValue/1>,...)}`；views 非空且最多16，数组作者顺序保留。UI 生成并提供完整 Source 预览，普通作者通过筛选/列/布局编辑器创建，无需手写 DAG/JSON/DSL。

`TypstQueryValue/1` 是现任 QuerySpec/3 的**逐成员原生表示**：全部八成员必需，format/version、lifecycleScope 和 searchBindings 闭集不变；dictionary→object、array→array、bool→bool、string→text，Counter 只接受精确有界 Typst integer；D7 arbitrary integer/decimal canonical text 仍为 string，不经 float。`(:)` 是空 dictionary，`()` 是空 array，单项数组需逗号。D7 本来不允许 null 的位置不接受 Typst none；Optional.none 仍使用 D7 已规定的显式 typed Optional arm。没有任意expression eval、spread、循环、导入值或JSON字符串fallback；仅§5.4具名typed叶子复用SOURCE受控标准constructor。schema 声明的 NodeRef typed 位置用§2无 fragment string解析完整 Ref，其它 Resource/Annotation Ref 仍按原 typed shape 的原生 dictionary映射；不能扫描普通字符串猜 Ref。未知成员/重复 key/不适用 type 拒绝应用投影，保留 source。

原旧QuerySpec3/ViewSpec1/TypedLiteral/V的null禁令只属于原decoder，不能覆盖新增profile。新feature carrier的唯一schema null槽是**每一个**`TypstQueryType/1`中`kind="typst_array"`的`maxItems`：null与原生`none`一一对应，包括Parameter.type/default.type、View.inputSchema、Domain TypedLiteral.type、递归array/dictionary/union/nullable中的同槽，保留准确无schema上限，不改成随意有限N。其余Query类型/值槽仍不接受raw null/none；Optional无值用`(state: "none")`，native_nullable作者空值也用明确state arm。缺optional成员是缺席，不以none占位。本文其它closed应用对象的已列null槽（link key/range、Recipe.parentIndex、SOURCE ValueType.maxItems）只在其自己的decoder中映射，不向Query开放。下面是完整可编译的原生载体例，全部八Query成员和合法View列绑定齐全；实际运行仍需Core获权cut，metadata编译不执行Query。

```typst
#metadata((id: "12d7c4f3-9ee6-4c36-920d-ec6af27bc654", types: ())) <weftext>
#metadata((
  format: "weftext.feature-definition", version: 1,
  query: (
    format: "weftext.query", version: 3, parameters: (),
    relations: (
      (id: "n", op: "scan", domain: "nodes", selector: (kind: "workspace"), "as": "node"),
      (id: "t", op: "read", input: "n", from: "row.node",
        bindings: ((name: "title", source: (kind: "title")),)),
      (id: "p", op: "project", input: "t", fields: (
        (name: "node", expr: "row.node"),
        (name: "title", expr: "row.title"),
      )),
    ),
    scalars: (), result: (kind: "rows", relation: "p"),
    lifecycleScope: "live",
    searchBindings: (version: 1, mode: "none", selections: ()),
  ),
  views: ((
    format: "weftext.view", version: 1,
    inputSchema: (kind: "rows", ordered: false, columns: (
      (columnId: "node", type: (kind: "node_ref")),
      (columnId: "title", type: (kind: "optional", item: (kind: "text"))),
    )),
    layout: "list", bindings: (primary: "title", target: "node"),
    options: (title: "笔记", description: "当前获权笔记", labels: (:),
      textDirection: "auto", numberFormat: "canonical"),
  ),),
)) <weftext-view-notes>

#title[查询示例]
结果由Weftext阅读器在此定义入口展示，不由Typst query执行跨库查询。
```

此映射在 definition source-read 后由 SOURCE 的官方 AST 字面投影生产，consumer 为 D7 native-definition adapter；不依赖整篇排版成功。保存只写同一 occurrence 的原生作者值，走 EDITOR 全源 prepare/save；不保存当前结果、运行时 evidence、快捷输入或设备状态。无法由 basic UI 表示的已合法高级 Query 条件保留，提供 Source；不能为显示控件覆盖它。

`TypstDefinitionAddress/1` 精确为 `{version:1,owner:NodeRef,revisionToken:Token,label:text}`，label 必须上述唯一标签。revisionToken是准确43字符canonical base64url字符串，受保护tag=`d6_source_revision/2`；完整RevisionTokenBinding/2只在Core受保护record，不嵌作者值。它是 production-version 地址，不是当前权限/新内容身份/永久 ViewRef。cold read/QueryRef 先原 owner disclosure，通过真实RevisionTokenSealArtifact/1及完整受保护binding认证其production SourceVersion/2，再取得完整 fresh SourceObservation 与该准确 sourceVersion 相等，SOURCE 验唯一字面定义和 profile，最后 barrier 重验；same bytes/latest 同标签不能复活旧地址。新 native QueryRef 的 typed address 槽消费本地址，故 `TypstQueryValue/1` 是独立 profile decoder，不能塞进旧 QuerySpec3 JSON/旧 DefinitionAddress；旧 QueryRef 保留原 decoder。每个 callee 有自己的 scope/searchBindings，caller不能覆盖。

原生dictionary key遵官方关键字/identifier规则；`as`等关键字及空格/非标识符key必须用字符串key（例如`"as": "node"`），官方AST解码后才与闭合成员名比较，不自己把keyword当identifier。

feature声明有独立的binding资格，不机械套用SOURCE根声明的前缀允许集合。默认生成器在已认证`<weftext>`根声明之后、普通正文/代码之前插入连续feature metadata；此处前缀只准SOURCE §2已证明的完整静态前缀（包括合格的title-before-root）、trivia、已认证根metadata及已合格的前feature metadata，官方AST验证每个直接调用是同一未遮蔽标准metadata、一个literal dictionary、直接唯一label。前root metadata是允许的inert应用声明，因此上例第二条合法。其它位置仅在官方scope/binding证据能静态证明同一builtin、无前置let/import/条件计算影响时取得资格；不能执行前置代码猜最后值，无法证明则保留源码并definition unavailable。默认保存器不会把高级合法文档改排成这个布局来“修复”。

持久TypstDefinitionAddress/TemplateSourceAddress只签发已sealed managed生产token。external source普通当前读取仍由完整SourceObservation和准确源可生成本profile inline Query执行，不能当成有managed签名的saved地址；明确Source admission/save形成managed版本后才可签发新持久地址。旧持久地址在external修改后stale，不按相同bytes/label强改current；普通打开/源码保存不受此saved-reference门阻断。

### 5.2 保留的业务执行与来源

本native Query decoder唯一Field read arm为`{kind:"field",fieldId,shape:"native_value"}`，输出整个schema值T，不继承旧Entry `values/entries`外包list/qualifier/note/provenance。联系人phones是**一个**保重复/作者顺序的array，不二次包list。下列新typed profile使全部SOURCE日常值具有read/project/equality/sort/exact-export正向路径；它不扩旧TypeSpec/V，也不把float近似成decimal。

`TypstQueryType/1` 独立decoder递归复用原D7 TypeSpec闭合成员，所有递归type槽改为本域；另只增加下表具名arms。`TypstQueryV/1`由准确schema指定，无自由any/未知tag。`TypstTerminalSchema/1`完整沿原rows/scalar/graph成员、列顺序与key合同，仅type槽消费本域。原合法JSON Query仍原TypeSpec/V/TerminalSchema；native inline/QueryRef/View/Action/导出完整闭包选择本profile，不混页/跨profile猜值。

| SOURCE valueType | 精确Type arm与V | 比较/排序和限制 |
| --- | --- | --- |
| string/bool/int/node_ref | 原text/bool/int64/node_ref；值为原准确文字、布尔、规范带符号整数文本或完整NodeRef | 保留原精确比较规则；整数不经浮点舍入，引用仅可按完整身份判等，不按标题、文件名或显示文字补造自然排序。 |
| decimal/decimal_text | 原decimal；V是原canonical exact decimal string，由SOURCE coefficient/scale精确计算 | 原任意精度decimal比较/排序；raw scale/negative-zero来源仍在Field原pin，不改作者字节。 |
| float | `{kind:"typst_float64"}`；V为SOURCE的16 lowercase hex IEEE754 binary64 bits | 仅有限值；equality与升降序按精确binary64数值，+0/-0数值相等但V/raw bits保留，distinct代表用原canonical raw最小值；无NaN/Inf。无int/decimal隐式升型。 |
| duration/exact_seconds_text | `{kind:"typst_duration"}`的V为canonical signed i64 seconds；`{kind:"typst_exact_seconds"}`的V为canonical exact decimal seconds | 精确elapsed seconds比较/排序；不调用浮点`.seconds()`，不推民用月/年或instant，不隐式互转两种精度。 |
| date | `{kind:"typst_date"}`；V精确`{year,month,day}`，SOURCE官方Gregorian有效日期，year=1..9999 | Gregorian civil tuple比较/排序；无时区/时刻补值。Calendar/Timestamp另见下段。 |
| time | `{kind:"typst_time"}`；V精确`{hour,minute,second}`，SOURCE各分量域 | civil clock tuple比较/排序；不是UTC instant/date，午夜不偷偷跨日。 |
| local_datetime | `{kind:"typst_local_datetime"}`；V精确`{year,month,day,hour,minute,second}`，SOURCE真实日期/时间域 | civil tuple比较/排序，无host timezone；不得和instant跨类型比较。 |
| instant | `{kind:"typst_instant"}`；V为SOURCE精确RFC3339 string | 用原D4 RFC3339解析成Gregorian UTC序数日+秒+任意精度小数比较/排序；保raw offset/string，不经float/date-serial，拒无offset/-00:00/leap second。 |
| array | `{kind:"typst_array",item:TypstQueryType/1,minItems:Counter,maxItems:Counter|null}`；V是按作者顺序的递归V数组 | 保重复，equality顺序敏感，无默认整体order；显式unnest/选择scalar sort。max=null/大于65535不塞旧list.maximum，按本新arm及实际预算checked，超预算整体失败。 |
| dictionary | `{kind:"typst_dictionary",members:[{name:text,type:TypstQueryType/1,required:bool,nullable:bool}...],additional:"preserve"}`；V精确`{members:<declared-member value dictionary>,preserved:[{key:text,value:SOURCE TypstNativeValue/1}...]}` | schema members按key规范序，不受旧64上限；preserved保未知原作者顺序/精确native值，不参加可写成员或猜Field身份。equality逐成员/完整preserved值；无默认整体order。预算计全结构，不跳成员。 |
| union | `{kind:"typst_union",variants:[{name:text,type:TypstQueryType/1}...]}`；V=`{variant:text,value:<branch V>}` | 按冻结SOURCE schema ordinal生成`variant0`、`variant1`…名字，值使用同ordinal，不能猜显示名/第一匹配；允许超旧8分支但计完整预算。equality按variant+值；order须显式投影可排序branch，不擅自跨branch数值coerce。 |

required由SOURCE effective type闭包OR决定。非nullable且required直接T，缺失invalid；非nullable且optional使用原Optional<T>，missing为原`{state:"none"}`。nullable使用独立`{kind:"typst_nullable",item:TypstQueryType/1,optional:bool}`，V恰`{state:"absent"}`（仅optional）、`{state:"none"}`（作者显式none）或`{state:"value",value:<T V>}`。绝不nestedOptional或混missing/作者none。dictionary成员递归同规则。native sort key仅对该nullable type增加必填`nativeNullOrder:"absent_none_value"|"value_none_absent"`，nonnullable key禁止此成员，value内按T order；缺显式空值策略拒sort。所有Type/V都是Query内部/运输表示，不写回`.typ`。

新weftext.typst.cel/1仍以CEL为唯一表达式语言，现有text/bool/int/decimal操作精确保留。新增types支持同型`== != < <= > >=`（仅表中orderable），禁止float↔int↔decimal、time/date/local_datetime↔instant隐式coerce。float同型四则严格IEEE754 binary64 round-to-nearest-ties-even、每primitive单独舍入不FMA，divide-zero/nonfinite/overflow用原错误；duration同型加减checked i64，exact_seconds同型用原exact decimal规则。date/time/local_datetime/instant只提供精确比较与原显式temporal-provider函数，未定义运算type_mismatch/unsupported_feature；不能据此拒read/project/sort/exact export。复合类型按原明确CEL member/list规则作用于其typed值，unknown preserved只能显式取只读native值，不提供动态Field/Action。新counter/容器/每值bytes计原完整budget。

`TypstQueryResult/1`精确`{kind:"typst_query_result",version:1,resultToken:Token,cursorToken:Token,schema:TypstTerminalSchema/1}`。泛型Core ResultHandle/cursor/epoch/auth/cut/TTL仍原权威；native page row/scalar/graph成员恰原D7运输成员，cells/value由该handle的TypstQueryV decoder按准确schema解码。旧d7_query_result/wire2的schema/V不能接新arm；新producer/消费者与native result export明确分派，不能旧页刷新成native权限。

`TypstViewValue/1`是原ViewSpec1成员/布局的本profile值，type槽改TypstTerminalSchema。仅具名替代task_list的status绑定为**text**且完整值域`open|done|cancelled`，须当前Result真实SOURCE Task schema status Field/declared task类型proof；普通同名string不认证Task，renderer不补查target。title仍要求nonoptional text，Query显式`row.title.orValue('')`生成displayTitle，placeholder不写回title/filename。Calendar/Timeline允许两端同型typst_date或typst_instant，date使用固定SOURCE Gregorian/day域及显式View calendarVersion，instant原精确UTC+显式timeZone/tzdbVersion。普通Task.due的optional日期/instant可明确筛缺失/投影calendar起止，due单点不是正duration事件；日期due可由显式Query将该日与nextCivilDay(due)投影为半开一日任务范围；该具名CEL函数只接typst_date，Gregorian checked下一日，超9999-12-31返回原numeric_overflow。instant due必须由作者显式给出正duration终点和timezone/provider，不能自动造Event/reminder；无正区间只保列表/日期字段。原其它View binding/完整性/访问性/纯呈现限制不变。

保留 D7 QuerySpec/3 八成员、DAG/canonical/error/bag/order、union_all、unnest/filter/project/sort/take/aggregate/distinct/query_ref/traverse/related/scalar/graph，以及 CEL exact 值/Optional、正负范围和 complete result 规则。generic join/window/quantile 仍后置。Field 读取使用 SOURCE 原生 FieldSource 与 Registry：schema明确定义 string/object array，不恢复每值 Entry UUID/provenance；重复联系方式按当前 occurrence区分、数组下标不永久化。未知/计算不可得字段是 unavailable，绝不当 absent/空。Field-only Query不偷读正文或 Ref 目标；title 为 Optional，filename/path不补标题。

body/headings/table 读取消费 SOURCE 官方语义产品和完整环境/依赖。函数/循环/重复 include 的合法生成 heading仍可作为只读 Query行，使用该 result epoch内 logical occurrence key；无唯一physical source不伪造 Locator/写权。heading level任意合法原生深度、run-in保持语义。distinct/global aggregate的新 K由其算子定义，不能留某个代表行的parent/action lineage。编译/语义产品不可用明确返回unavailable；不用重新parser/字符串flatten获得假正文。

跨库执行是对显式已连接获权 Workspace 的 Core Query calls，结果合并须现有精确 schema/union规则与完整依赖；URI不授跨库read/write或跨authority原子事务。索引仅加速候选，building/partial/stale/预算不足与完整零结果不同，隐藏count/rank/snippet不得披露。原 Search 的plain/visual/显式shortcut presets及grammar按 D7-SEARCH §§1–10保留；不要求用户用快捷模式。

Trash/live_and_trash按D7-SEARCH §11准确只读scope及同query_scan Coverage/1；tombstone/placeholder/缺证明不当结果。scope不会恢复关系边，也不授restore/purge或Action；完整调用闭包含nonlive时写consumer拒绝，即使filter只剩live。保存searchBindings保留contributionId/version/完整descriptor/namespaceOwner；cold reopen从当前D10 Catalog逐项验证，贡献撤停/变版但Field完好仍definition_unavailable，all新增贡献不自动扩大，none普通Field Query不加ambient package门。授权变化reset result/cursor/subscription，旧页/rowHandle不能续权。

### 5.3 数据库视图、任务与录入

任务项目的实际producer是SOURCE seed `tasks/project`：optional、nonEmpty、无fragment的typed node_ref，作用于明确Task成员。读取/关系/inbound/typed copy仍同cut和独立权限，目标是普通Node，无需额外ProjectFacet；company文本或其它NodeRefField不冒项目字段。未安装准确seed/schema资格时是unavailable而非假定有同名字段。

heading scan的新闭合内部K为`{invocation:I,operator:o,kind:"typst_product_occurrence",owner:NodeRef,sourceVersion:SourceVersion/2,entry:Counter}`，entry是完整官方realized semantic introspection序列中该heading的物理index，不是筛后ordinal。每个(I,o,root owner)在受保护Result恰绑定一个完整TypstSourceSnapshot/1+semanticProjection pin及其environment/world/全部正负依赖；consumer先独立校验该basis，再查entry恰heading。重复include即使physical locator相同也有不同entry；不同QueryRef调用I不共K。K按原D3-CJ规范bytes/unsigned UTF-8比较，全结构/scan/basis计预算；缺basis/不完整projection整体失败，不跳heading。sourceVersion相同但include/environment改变仍reset/replacement，不凭相同entry复用句柄；K不是永久地址。原group/distinct按其eqKey生成新K无parent，全局aggregate keys=[]，不取代表输入的写权。

ViewSpec/1仍只呈现同一完整 Query terminal，不fetch/filter/sort/聚合、猜 title/avatar/隐藏端点或执行脚本。保留table/list/task_list/board/calendar/timeline/gallery/metric/bar/line/scatter/pie/heatmap/network闭集、显式顺序、typed column绑定、完整accessible table、Optional/zero/empty/error区别及CJK/RTL。缺renderer明确 unavailable；页面前200条不是完整membership/语义图表，不能采样冒完整。后置tree/treemap/sunburst/Gantt/统计bindings不因改格式加入。

任务是普通Node的task类型，status/due/project是SOURCE schema字段，详情是该笔记正文；表、板、历、智能清单消费同一Query。勾选、拖lane、改due必须显示准确Field变化，唯一当前写来源可证明才通过Action prepare，聚合/计算/隐藏/旧row不写。周期导航/规则预览保留当前历法/时区/range/provider义务；提醒送达、稍后、完成后重复、我的一天/自然语言日期仍后置。

快速录入复用parent/type/template/title（可无）/属性/正文面板；保存成功才能“再记一条”清空，unknown保留原request/Draft，不再create。由receipt实际resultAllocations打开真实Node；requireMembership提交前依D5验证，未要求时创建可成功。只有完整当前Query证明不包含时才显示“当前筛选之外”，partial/未完/stale只能显示membership unknown并可打开真实对象，不能重复创建或改Query。

### 5.4 新参数、常量和调用的完整分派

下列对象都由本profile选择，不给原D7 TypedLiteral/ParameterSpec/QueryCall decoder增加unknown type。未列optional成员一律必需，原参数name词法/unique、required/default错误顺序、invoker权限、完整callee闭包/调用预算和CEL scope限制保留。

```text
TypstTypedLiteral/1={type:TypstQueryType/1,value:TypstQueryV/1}
TypstParameterSpec/1={name:text,type:TypstQueryType/1,required:boolean,
 default?:TypstTypedLiteral/1}
TypstQueryCall/1={query:{kind:"inline",spec:TypstQueryValue/1}
 |{kind:"saved",definition:TypstDefinitionAddress/1},
 arguments:{parameterName:TypstTypedLiteral/1...},context:TypstQueryContext/1}
TypstQueryValueSource/1={kind:"literal",literal:TypstTypedLiteral/1}
 |{kind:"parameter",name:text}
TypstLexicalBinding/1={kind:"literal",literal:TypstTypedLiteral/1}
 |{kind:"lexical",name:"this.node"|"this.heading_title"}
TypstContextBinding/1={kind:"literal",literal:TypstTypedLiteral/1}
 |{kind:"runtime",name:"now"|"timeZone"|"calendarVersion"|"tzdbVersion"}
TypstDynamicBlock/1={format:"weftext.typst-dynamic-block",version:1,
 query:{kind:"inline",spec:TypstQueryValue/1}
 |{kind:"saved",definition:TypstDefinitionAddress/1},
 view:{kind:"inline",spec:TypstViewValue/1}
 |{kind:"saved",definition:TypstDefinitionAddress/1,viewIndex:Counter},
 arguments:{parameterName:TypstLexicalBinding/1...},
 contextBindings:{contextName:TypstContextBinding/1...}}
TypstDefinitionPayload/1=TypstFeatureDefinition/1|TypstDynamicBlock/1
```

这两个name→value对象只准静态已声明parameter/context名字，不是开放map/任意Field访问。Parameter.required=true禁止default；false必须有完整同type的default，optional parameter未提供才用default，显式none/absent值不触发default。未知/重复参数、缺required、type不同均按原错误整调用拒绝。`TypstQueryContext/1`的成员集合仍由闭包静态推导，恰为所需now/timeZone/calendarVersion/tzdbVersion：now用新TypedLiteral decoder但type必须恰原zoned_instant，其他是nonempty text；无所需context为`{}`。不把typst_instant偷偷转zoned_instant，不从主机补timezone/now。lexical this.node生成完整NodeRef的新TypedLiteral，this.heading_title生成text，均先按实际owner/当前heading资格取得再匹配callee type；SavedQuery本身没有this。runtime binding只取明确调用context，同invocation now一次冻结。

| 实际typed槽 | 唯一新producer→consumer |
| --- | --- |
| Query.parameters[].type/default、inline/saved/callee解码 | 新ParameterSpec/TypedLiteral；参数定义按name规范序，default.type与parameter.type完整相等。 |
| QueryCall.arguments、PAB/action.canonicalCallInputs、CollectionCall.call | TypstQueryCall及新TypedLiteral逐成员解码；整个闭包的参数与context来自同cut，不把新arguments交旧decoder。 |
| relation/scalar query_ref.arguments、selector literal、parameter defaults | ValueSource的literal走新TypedLiteral，parameter只映callee同型值；禁row/scalar/this相关值，保原调用隔离。 |
| recurrence.horizon、View fixed Domain.minimum/maximum及其它typed constant | 每个原schema标为TypedLiteral的真实槽都换新TypedLiteral；原horizon/domain业务type要求仍验证，未知type不能蒙混。 |
| DynamicBlock.arguments/contextBindings、saved view call | 上述Lexical/ContextBinding闭集；viewIndex仅选已认证Feature.views中的准确索引，不拿第一个猜。 |
| DefinitionTransfer静态typed roots、Result schema/V、native export、ResultPin | 全部递归选择本Type/V；包含新default/argument/lexical literal/context literal的Ref，普通CEL/text不扫描身份。 |

TypstDynamicBlock的原生作者carrier同样是合格`#metadata(...) <weftext-view-key>`，按format严格选择上述两臂；Feature.query可供saved QueryRef，DynamicBlock本身不可冒callee Query。TypstDefinitionInput.value改为TypstDefinitionPayload，typed transfer必须全枚举两臂。UI默认仍普通Query/视图builder，不要求作者手写这些内部typed对象。

源码carrier的typed value位置按准确type复用官方原生值：float64的直接Float AST映为其准确finite bits；int64直接Int或SOURCE合格标准int(Str)构造映canonical i64 string（包含i64MIN）；decimal/duration/date/time/local_datetime的SOURCE受控标准构造按实际builtin binding映本V；array逐item、dictionary/union/nullable逐声明arm递归。无界integer/decimal_text仍可canonical string；node_ref仍无fragment URI。该源映射与runtime TypedLiteral wire区分，后者严格只接受§5.2的V编码。未声明位置不解释类型，不求任意表达式；metadata的“字面声明”仅在这些typed叶子准SOURCE已定义受控constructor，不能给Query图/ID/type/schema开放计算。

新CEL profile为`weftext.typst.cel/1`，不是原weftext.cel/1的宽松模式。普通int literal仍int64、text/bool不变；仅本profile准官方CEL AST的finite double literal，合成typst_float64，保其源literal经正确IEEE754 ties-even解析的bits，-0.0保负零。不能因expected float把int literal变float，不能因expected decimal把double转decimal。具名`float64Bits("16hex")`、`durationSeconds("canonical i64")`只准literal合法payload，分别合成native float/duration；错误bits/NaN/Inf/overflow在静态阶段拒绝。expected Type只能来自准确参数/literal/已声明函数签名和父constructor规则，不来自首行数据/列label；非literal表达式须完整type相等。原optional/list/object/conditional双向规则递归换本type，native_array literal在expected完整min/max下校验，maxItems=null不偷变元素数；无expected非空array产生min=max=语法元素数，空array须expected。native dictionary/union/nullable只在完整expected type下检查closed value/state/variant，不能猜branch、flatten nullable或丢preserved值。canonical CEL AST含新literal的准确raw bits/type，调用/cache semantic identity保raw typed常量，不用下面eqKey代替其实际输入。

**QP-01完整作者fixture。** 以下两份`.typ`是高级保存Query的review fixture，普通笔记/属性作者仍用UI而不手写wire。先提交第一份callee并由Core实际签发其production token，再由caller builder填入第二份地址；这里43个A只是词法示例，实际获权正向fixture必须换成第一份真实已认证token，不能把示例token当签名。source schema已获权安装`user/amount`为float，所选实体Fieldread静态型明确为Optional<float64>；callee读取/筛非空/比较参数、显式sort/project，并将准确无schema上限array参数原样投影。下面所有schema maxItems:none都只在合法null槽，default.value实际native float/array由typed源mapper转wire bits。

```typst
#metadata((id: "12d7c4f3-9ee6-4c36-920d-ec6af27bc654", types: ())) <weftext>
#metadata((format: "weftext.feature-definition", version: 1,
  query: (format: "weftext.query", version: 3,
    parameters: (
      (name: "samples", type: (kind: "typst_array", item: (kind: "typst_float64"),
        minItems: 0, maxItems: none), required: false,
        default: (type: (kind: "typst_array", item: (kind: "typst_float64"),
          minItems: 0, maxItems: none), value: (0.5, -0.0))),
      (name: "threshold", type: (kind: "typst_float64"), required: false,
        default: (type: (kind: "typst_float64"), value: 0.5)),
    ),
    relations: (
      (id: "n", op: "scan", domain: "nodes", selector: (kind: "workspace"), "as": "node"),
      (id: "a", op: "read", input: "n", from: "row.node", bindings: (
        (name: "amount", source: (kind: "field", fieldId: "user/amount", shape: "native_value")),)),
      (id: "f", op: "filter", input: "a",
        predicate: "row.amount.hasValue() && row.amount.value() >= param.threshold"),
      (id: "s", op: "sort", input: "f", keys: (
        (expr: "row.amount.value()", direction: "asc", "none": "first"),)),
      (id: "p", op: "project", input: "s", fields: (
        (name: "amount", expr: "row.amount.value()"),
        (name: "samples", expr: "param.samples"),)),
    ), scalars: (), result: (kind: "rows", relation: "p"), lifecycleScope: "live",
    searchBindings: (version: 1, mode: "none", selections: ())),
  views: ((format: "weftext.view", version: 1,
    inputSchema: (kind: "rows", ordered: true, columns: (
      (columnId: "amount", type: (kind: "typst_float64")),
      (columnId: "samples", type: (kind: "typst_array", item: (kind: "typst_float64"),
        minItems: 0, maxItems: none)),)),
    layout: "table", bindings: (columns: ("amount", "samples")),
    options: (title: "阈值查询", description: "准确浮点与数组", labels: (:),
      textDirection: "auto", numberFormat: "canonical")),),
)) <weftext-view-threshold>
```

```typst
#metadata((id: "b7a36d54-52ec-4e63-9f0d-6579e2baee7f", types: ())) <weftext>
#metadata((format: "weftext.feature-definition", version: 1,
  query: (format: "weftext.query", version: 3,
    parameters: (
      (name: "limit", type: (kind: "typst_float64"), required: false,
        default: (type: (kind: "typst_float64"), value: 0.5)),
      (name: "samples", type: (kind: "typst_array", item: (kind: "typst_float64"),
        minItems: 0, maxItems: none), required: false,
        default: (type: (kind: "typst_array", item: (kind: "typst_float64"),
          minItems: 0, maxItems: none), value: (0.5, -0.0))),),
    relations: (
      (id: "r", op: "query_ref", definition: (version: 1,
        owner: "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654",
        revisionToken: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        label: "weftext-view-threshold"), arguments: (
        threshold: (kind: "parameter", name: "limit"),
        samples: (kind: "parameter", name: "samples"))),
      (id: "s", op: "sort", input: "r", keys: (
        (expr: "row.amount", direction: "asc", "none": "first"),)),
    ), scalars: (), result: (kind: "rows", relation: "s"), lifecycleScope: "live",
    searchBindings: (version: 1, mode: "none", selections: ())),
  views: ((format: "weftext.view", version: 1,
    inputSchema: (kind: "rows", ordered: true, columns: (
      (columnId: "amount", type: (kind: "typst_float64")),
      (columnId: "samples", type: (kind: "typst_array", item: (kind: "typst_float64"),
        minItems: 0, maxItems: none)),)),
    layout: "table", bindings: (columns: ("amount", "samples")),
    options: (title: "传递参数", description: "同型callee调用", labels: (:),
      textDirection: "auto", numberFormat: "canonical")),),
)) <weftext-view-caller>
```

调用fixture使用TypstQueryCall/1，arguments={}使limit/samples取上述真实default；显式threshold float argument可用runtime合法bits V而不是源码Float number。传到callee的有效TypedLiteral.type与参数Type逐字相同，QueryRef擦除ordered再由caller sort恢复；native result export保两个float列/array V完整schema与bits。caller改default不重写callee，换type/NaN/raw-null/旧TypedLiteral混域/unsealed token整体失败。

### 5.5 唯一native EqualityCanonical和等价类消费者

`TypstEqualityCanonical/1`精确根为`{format:"weftext.typst.equality",version:1,type:TypstQueryType/1,value:TypstEqualityTree/1}`。type使用已strict验证的完整规范type（含scope、bounds、nullable/member要求、union顺序），不只kind；原对象member按原规范序、新dictionary members按unsigned UTF-8 name、union保branch顺序。原和新profile根域不同，不跨域碰key。value树如下，全部closed；数组顺序有意义，数字以canonical文本而非host浮点表示。

| T | EqualityTree精确形状 |
| --- | --- |
| 原D7非复合类型 | bool=`{kind:"bool",value:boolean}`；text=`{kind:"text",value:text}`（仅nfc type按固定Unicode15.1 NFC）；int64/integer/decimal=`{kind:"number",value:text}`（原canonical数学值）；code=`{kind:"code",value:text}`（完整scope在外type）；Ref=`{kind:"ref",value:EntityRef}`（完整D3 canonical tuple）；calendar_date=`{kind:"calendar_date",calendarId:text,calendarVersion:text,precision:text,value:text}`（原D4 comparator的准确canonical日期lexeme）；zoned_instant用下列instant tree；quantity=`{kind:"quantity",dimensionId:text,unitId:text,magnitude:text}`（同cut验证的完整basis与canonical exact magnitude）。每臂严格由外type选，不能接受自由leaf或原raw值冒规范值。 |
| typst_float64 | `{kind:"float64",bits:text}`，合法finite非零bits原样，两个zero bits都恰写`0000000000000000`；V不改。 |
| typst_duration/typst_exact_seconds | `{kind:"duration",seconds:text}`或`{kind:"exact_seconds",seconds:text}`；i64准确文本或去末尾零/负零的exact canonical decimal。两type不混。 |
| typst_date/time/local_datetime | `{kind:"date",parts:[year,month,day]}`、`{kind:"time",parts:[hour,minute,second]}`、`{kind:"local_datetime",parts:[year,month,day,hour,minute,second]}`；分量canonical整数，保类型，无zone。 |
| typst_instant | `{kind:"instant",utcDay:text,secondOfDay:Counter,fraction:text}`；Gregorian 0001-01-01的ordinal为0，exact offset移到UTC后day可负或越原展示年；second=0..86399，fraction为0或`0.`加无尾零的任意精度digits；拒leap second，不截断小数。 |
| 原optional / native nullable | `{kind:"optional",state:"none"}`或`{kind:"optional",state:"some",value:tree}`；native为`{kind:"nullable",state:"absent"|"none"}`或`{kind:"nullable",state:"value",value:tree}`，absent仅optional许可，三个state分域。 |
| 原list/object/union及native array/union | list/array为`{kind:"sequence",values:[tree...]}`；object为`{kind:"object",members:[{name:text,value:tree}...]}`按schema序；union为`{kind:"union",variant:text,value:tree}`，native variant按冻结ordinal。外层完整type分域。 |
| typst_dictionary | `{kind:"dictionary",members:[{name:text,value:tree}...],preserved:[{key:text,value:TypstPreservedEqualityTree/1}...]}`；known按schema序，preserved按准确key unsigned UTF-8排序，**preserved原字典键顺序不参与相等**但raw V保源顺序；重复key拒绝。 |

`TypstPreservedEqualityTree/1`只递归SOURCE NativeValue：string/bool/int用对应原生kind+准确value；none为`{kind:"none"}`；float用上述float64 tree；decimal将coefficient/scale/negative精确转canonical decimal并写`{kind:"decimal",value:text}`（同值不同scale/-0同key）；duration/date/time/local_datetime用上表tree；array用sequence tree保重复/顺序；dictionary用`{kind:"dictionary",entries:[{key:text,value:TypstPreservedEqualityTree/1}...]}`按key排序。unknown preserved的string不做NFC、URI/Field/Ref推断。两字典只有键顺序不同可同eqKey，值或array顺序改变不同；raw表示始终保留。

EqKey恰D3-CJ/3 UTF-8完整根字节；byte comparator为unsigned lexicographic。hash只用`SHA-256(UTF8("weftext.typst.equality/1")||NUL||eqKey)`加速，collision仍比完整type/tree字节。新`==`、hash/contains、group/distinct/count_distinct、min/max等值代表及聚合/去重K全部调用这一唯一函数，不直接用raw bits/offset/key数组。排序的数值/时间order仍§5.2，不以CJ字节给值造自然序；相等必须与同type比较一致。raw代表在已形成的等价类中取完整raw V或整key tuple的CJ3 bytes最小者，不按第一行/拼成员；两种zero一类而通常选正零raw代表，两个offset的同UTC instant一类但保所选完整原offset。

group K只含base与按作者keys顺序的完整eqKey数组；global keys=[]；distinct K含完整column序的eqKeys；两者不含输入parent或代表行writer。输入顺序/分区/worker/页界变化不得改类数、代表或K；所有递归节点/bytes/hash/排序计原预算，预算失败不能截树/抽样。Equality树不能替代参数/raw literal、实际输出pin或SemanticStateKey中的真实值：signed zero可有同equality但不同输出bits，cache仍绑定准确raw输入/完整依赖。

## 6. 新 feature holders 与 typed transfer

native Query入口是独立`TypstQueryExecute/1={kind:"typst_query_execute",version:1,workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,expectedFrontier:Frontier/2,call:<native QueryCall>,budget:<original D7 Query budget>}`，返回上述TypstQueryResult1。旧d7_query_execute/wire2不在原JSON闭集中猜native source或加新arm。Core保护的native result profile标记决定Type/V/schema page消费；所有入口同原授权/错误顺序，不增第二执行器。新nullable值的CEL具名函数`nativePresent(x)`、`nativeIsAbsent(x)`、`nativeIsNone(x)`分别读精确state；`nativeValue(x)`只在value态返回T，否则原none_value。`nativeVariant(x)`返回冻结variant名，`nativeCase(x,"variantN")`要求literal合法name、匹配才返回该branch T，否则原type_mismatch。函数不coerce、都计原预算，前三state测试total，其它可error；unknown preserved访问只读，不创造Field身份。

以下是 CORE `FormatAdapterEnvelope/1` 的具名角色payload，不是给旧PAB4、Result9/10、ExportPlan4或closed union加未知kind。generic D6 SourceVersion/Observation、Frontier、InputDescriptor4/Proof4/PIntent4、唯一CAS/P/receipt按CORE保留，document_format携准确Typst profile pin；所有saved/planned/unknown predecessor先原record查找/恢复，不重pin/换版本。

`TypstDefinitionInput/1` 准确 `{version:1,address:TypstDefinitionAddress/1,observation:SourceObservation/1,sourcePin:PinRef/2,value:TypstDefinitionPayload/1}`，受保护record完整绑定准确occurrence、作者值和实际SOURCE环境；address/value不是caller认证。action_input/prepared_action payload逐槽采用CORE §4表：action消费下列新Spec，definitionInputs消费本Input，registryInputs消费SOURCE `TypstSchemaSnapshot/1` qualification，proposedInputs消费CORE proposed_input envelope，constructionInput消费新template role。request/proof/sourceInputs/preview/audience/expiry/budget与冻结输入逐项相等；不另建binding store/ledger，不再声明一份同形TypstActionInput。native调用槽全部选择§5.4的 `TypstQueryCall/1`：inline/saved、Parameter/default、arguments/context typed literals、QueryRef与DynamicBlock逐槽使用本新decoder；外层 `{query,arguments,context}` 成员不变，绝不让新值进入旧D7 arguments decoder。

`TypstActionSpec/1` 准确 `{format:"weftext.typst-action",version:1,intent}`，仅下面闭合集合。`TypstActionTarget/1={ref:NodeRef,observation:SourceObservation/1}`；观察必须与完整preimage/source pin、当前cut相等，不用单一revision号码。`TypstActionFieldSelector/1={target:TypstActionTarget/1,fieldId:FieldId,valuePath:[text|Counter...],preimagePin:PinRef/2}`；path从SOURCE准确Field值根出发，member按schema解码、index只针对array且当前有界，完整AST/origin/字节由Core重新核对，不接受caller的可信path。value下列均为schema准确`TypstNativeValue/1`，native Ref依typed位置处理，不能额外JSON Entry。可选成员仅表中`?`，其他缺/多成员拒绝。

| intent.kind / 准确其它成员 | 语义/替代旧动作 |
| --- | --- |
| `set_title` / target,value:text | 当前唯一native title写入；合法无title可显式创建，不按filename补title。 |
| `set_field` / target,fieldId,value | 整个Field作者值设定/替换；不存在时创建准确root key；旧单值replace与普通字段创建。 |
| `remove_field` / target,fieldId | 删除该准确root作者键，必填/关系约束仍验证，不删其它同label字段。 |
| `set_field_member` / selector,value | 精确object member/array内member替换；不可修改不存在member或改变schema类型；旧narrow phone修改消费此槽。 |
| `insert_field_item` / selector,at:Counter,value | selector终点必须array，at=0..length；添加一项，原其它item顺序/字节保留，无新事实UUID。 |
| `remove_field_item` / selector,at:Counter | selector终点array，at<length；只删当前准确item；不是永久occurrenceKey。 |
| `set_types` / target,types:[text...] | SOURCE完整类型集合变更/required约束；旧assign/remove facet改走明确完整types，不保留Entry initialEntries，field cleanup另显式remove_field。 |
| `native_edit` / edit:EDITOR TypstEditOperation/1 | 仅EDITOR闭合基本operation能力；原cell/row/heading/link等来源动作由其native adapter处理，不能作为arbitrary callback/raw patch。 |
| `annotation_action` / action:EDITOR TypstAnnotationAction/1 | 唯一闭合Annotation创建/建议应用/拒绝adapter；同原source/Annotation decision、接受状态与当前target资格，不自行扩大kind。 |
| `d3_operation` / request:CORE identity_request envelope | 保留完整操作模式、权限与对象闭包；包括创建、移动、复制、恢复、清除，只用原D3具名模式，不增加私有分支。 |
| `collection_create` / collection:<native CollectionCall>,parentPolicy:<原ParentPolicy>,source:text,ordinal:Counter,requireMembership:bool | 创建一个真正的新节点并验证完整提交后查询；准确源按SOURCE与CORE的具名构造入门，不预填可信身份，不以预览冒充创建。 |
| `collection_remove` / collection:<native CollectionCall>,change:<本表set_field或remove_field intent> | 只改准确当前targetField并证明完整postQuery不再包含它；不是隐式Trash。 |
| `bulk_field` / targets:<原D7 TargetSet，EntityTarget映为本Target>,fieldId,operation:{kind:"set",value}|{kind:"remove"} | 完整获权≤1000 Node原子操作；array append不暗含在set中，不拆批/选第一值。 |

除native values/source/definition/selector替换外，原Action target-set全量/去重、whole-call live scope、潜在dependency闭包、owner_fields窄资格、关系/类型requiredness、budget/审批/完整preview/唯一D3|D6提交语义保留。普通native checklist没有稳定Task身份；其生成/升级任务须explicit source/D3创建，不自动看文本找Task。旧promote/checklist/Entry/初始复杂provenance的wire动作不能放进新Spec；需要的native表格/列表基础操作由EDITOR矩阵承担，未选复杂事实与旧来源动作保留历史decoder/显式导入处置。Annotation建议动作仍沿EDITOR拥有的闭合annotation adapter，不借本Spec补任意intent。

`TypstDefinitionTransfer/1` 精确 `{version:1,sourceAddress:TypstDefinitionAddress/1,resultOwnerSubject:PayloadSubjectKey,resultLabel:text,slots:[{path:[TypstTransferPathSegment/1...],before:TypstTransferActualValue/1,after:<closed map/preserve/reissue_definition choice>}...]}`。producer从完整前像官方AST/native decoder枚举**所有**typed槽，consumer独立重枚举比较；无Ref定义也须slots=[]证明。path按member unsigned UTF-8/index数值排序，unique且无祖先重叠。labels、CEL strings和free text不扫描。map/preserve沿Core原identity/locality，reissue_definition只选真实同map目标definition label；最终地址用同一新owner/冻结revisionToken，不保留旧地址。

`TypstDefinitionTransferPlan/1` 精确 `{version:1,sourcePins:[{owner:NodeRef,observation:SourceObservation/1,pin:PinRef/2}...],transfers:[TypstDefinitionTransfer/1...]}`，pins按完整Ref排序唯一，所有sourceAddress恰被准确sourcePin覆盖。定义地址的完整排序键DefinitionAddressKey为`(version,RefKey(owner),revisionToken UTF-8 bytes,label UTF-8 bytes)`，version严格为1；请求transfers按`(DefinitionAddressKey(sourceAddress),subjectCanonicalKey(resultOwnerSubject),resultLabel UTF-8 bytes)`排序且全键唯一，结果另按完整`(resultOwnerSubject,resultLabel)`唯一。因此同一完整sourceAddress可以显式实例化到不同结果subject或不同结果label，不能仅因源重复拒绝，也不能由不同源重复声明同一结果。

这里的PayloadSubjectKey、subjectCanonicalKey、RefKey及其编码精确复用原D3 §4.1.3（固定owner `snapshots/d3-identity-references-ownership-and-lifecycle/source.md`）：existing/mapped按原rank及完整RefKey；new Node按原rank及数值ordinal；Resource/Annotation按原kind rank、数值ordinal和递归owner subject键；artifact按原摘要与数值partOrdinal。递归owner仍必须闭合且无环。subject对象仍用原D3-CJ/3规范编码，排序使用原递归tuple而非JSON字节、显示名、payload或数组位置；本键不分配身份，也不扩大Node结果owner的原资格。after选择精确为`{kind:"map",subject:PayloadSubjectKey}`、`{kind:"preserve"}`或`{kind:"reissue_definition",ownerSubject:PayloadSubjectKey,targetLabel:text}`；最后一臂要求真实映射的完整原definition address及新唯一label，不能改为别的同名定义。CORE construction的transferPlanPin绑定此准确完整对象及同source版本，不是自由JSON或另一个候选map。

`TypstDefinitionTransferEffects/1`及每slot不再使用未定型数组，完整closed结构如下；源/结果地址按真实记录版本解码，Node/Resource/Annotation Ref按原完整D3 decoder，不能把同名字段拼成Ref。

```text
TypstTransferPathSegment/1={kind:"member",name:text}|{kind:"index",index:Counter}
TypstTransferActualValue/1={kind:"node_ref",value:NodeRef}
 |{kind:"resource_ref",value:ResourceRef}|{kind:"annotation_ref",value:AnnotationRef}
 |{kind:"definition_address",value:TypstDefinitionAddress/1}
TypstTransferEffectSlot/1={path:[TypstTransferPathSegment/1...],
 before:TypstTransferActualValue/1,after:TypstTransferActualValue/1}
TypstDefinitionTransferEffectItem/1={sourceAddress:TypstDefinitionAddress/1,
 resultAddress:TypstDefinitionAddress/1,beforeSource:text,afterSource:text,
 slots:[TypstTransferEffectSlot/1...]}
TypstDefinitionTransferEffects/1={version:1,decisionKey:DecisionKey/2,
 transfers:[TypstDefinitionTransferEffectItem/1...]}
```

每path非空且为本profile静态schema真实typed root，按member unsigned UTF-8/index数值比较、短prefix先，unique且无祖先重叠。Transfer/TransferPlan的request path也统一该Segment，不一方用字符串/另一方用object；before改为ActualValue，after仍原map/preserve/reissue闭集。效果before/after kind必须同型，definition_address整根包括owner不另列嵌套owner；不能出现new_result_subject或caller伪分配。source/result两完整source必须逐字对应真实前像/最终after pin且包含唯一准确label，slots恰等于独立AST/type遍历所得全部槽（包括parameter/default/DynamicBlock literals）。Ref-free item仍slots=[]。

effects在同一候选map/token完成物化后，按`(DefinitionAddressKey(sourceAddress),DefinitionAddressKey(resultAddress))`重新排序且全键唯一，完整resultAddress另须唯一；两地址键均使用上述完整版本/Ref/token/label comparator，不把请求subject序误作最终Ref序。每个效果先以完整sourceAddress及resultLabel选取计划项，再通过同一map把其完整resultOwnerSubject物化为准确result owner，并验证冻结token与完整resultAddress；必须恰与一个计划项及真实receipt.resultAllocations的完整subject/Ref对应，不能按Plan/Effects/receipt数组位置关联，也不能在同源多实例中只匹配源或label。**作者Query/View/array顺序保留**。preview是同一计划私有candidate物化的完整值/源，不授公开birth；committed逐字等唯一winning map/token/afterPin及receipt分配，seal前后不能换map/source。原D7Effects2仍只原历史恢复，CORE rank4只引用这份新type。

Core materialization固定同一private candidate map、revision plan/token和新定义标签；第一遍写全部typed Ref与待重发定义地址的固定内部占位版本，官方重解析确定真实定义集合；第二遍填同一拟议revisionToken字符串/owner/label（不把seal后才产生的ChangeId嵌进源），官方重解析验证全部地址、SOURCE根声明和typed值。地址不含byte坐标，故不沿旧JSON一行Q假设；任何定义不唯一/占位残留/前后slot覆盖不等/版本改变整体拒绝，内部候选不交付。Field/link/definition/Annotation重写共享同一map与最终source，winning CAS/P才形成生产版本、ChangeId、token的真实seal artifact与有效receipt；seal不再次改写after source；loser不借另一map/seal。未知feature可源读/保存，但copy/fork的typed transfer不能默默跳过。

## 7. 原生模板、出生与导入

所有作者内嵌production地址（definition/QueryRef/recipe template与nodes.source）均使用上述opaque revisionToken，不嵌未知seal ChangeId。token不是URI片段，不百分号decode；严格原43字符token decoder拒绝padding/错tag/非规范base64url。create/copy/fork materialization在同plan私有生成并冻结随机token，两遍完整源写入同token+label后冻结afterPin；原seal artifact才将它签到实际完整SourceVersion/ChangeId，seal不回写source。消费者必须验证真实历史artifact、identity与生产源binding，再fresh currentObservation；unsealed/caller伪token、相同stamp/bytes异token和强改current地址均无资格。export等受保护内部证据仍可存真实fullSourceVersion，不能把所有证据机械token化。

排版模板用普通Typst module/import、函数参数与set/show。用户可复用成熟模板而非只能固定title/body slot；World只供应获准冻结的包/字体/资源和只读输入，函数不能读取ambient Workspace、创建Node、生成可信id/birth、授权限、写sidecar或调用Action。普通作者metadata在root独立，不由模板返回的同标签metadata认证。

应用创建模板使用下列完整native Recipe/参数定义，替代原来不存在的“SOURCE schema parameter definitions”。这是独立普通Resource的内部配方与受保护构造，不搬进普通笔记业务字段；UI参数表生成原生值，排版源码仍用普通import/函数。

```text
TypstTemplateSourceAddress/1={version:1,owner:NodeRef,revisionToken:Token}
TypstRecipeParameterDefinition/1={name:text,valueType:SOURCE TypstValueType/1,
 required:boolean,nullable:boolean,
 default:{kind:"absent"}|{kind:"literal",value:SOURCE TypstNativeValue/1}}
TypstRecipeParameterValue/1={name:text,value:SOURCE TypstNativeValue/1}
TypstRecipeBinding/1={parameter:text,slot:{kind:"title"}
 |{kind:"body",locator:SOURCE TypstRangeLocator/1,expectedText:text}
 |{kind:"field",fieldId:FieldId}}
TypstRecipeNode/1={source:TypstTemplateSourceAddress/1,
 parentIndex:Counter|null,types:[text...],bindings:[TypstRecipeBinding/1...]}
TypstRecipe/1={format:"weftext.typst-recipe",version:1,
 template:TypstTemplateSourceAddress/1,
 parameters:[TypstRecipeParameterDefinition/1...],nodes:[TypstRecipeNode/1...]}
TypstTemplateDestination/1={kind:"parent",parent:NodeRef,ordinal:Counter}
 |{kind:"collection",collection:TypstCollectionCall/1,parentPolicy:<D7 ParentPolicy>,
 ordinal:Counter,requireMembership:boolean}
TypstTemplateConstruct/1={version:1,template:SourceVersionRef/1,
 recipe:SourceVersionRef/1,parameters:[TypstRecipeParameterValue/1...],
 resources:[SourceVersionRef/1...],destination:TypstTemplateDestination/1,
 externalNodePolicy:"reject"|"preserve_current"}
```

所有member固定且无额外optional/null；name沿原`[a-z][a-z0-9_]{0,63}`，参数定义/值按name排序唯一，unknown/duplicate/缺required拒绝。required=true的default固定absent；false可absent（未提供只保原模板值）或经过valueType+nullable验证的literal（未提供才用default，实际写入必须进入完整source预览）。显式none仅nullable=true且合格，不能触发default；literal default保SOURCE native decimal scale/negative、float bits、duration exact seconds等完整表示，不用Query canonical值冒NativeValue。参数类型不等、Field enum/requiredness不合均拒，不能loss接受掉。

nodes[0].source与recipe.template/请求template的已认证完整生产版本同一对象，parentIndex=null；其它父index须早于子，source owner唯一、源为所选live template subtree，禁止多root/cycle/同Node不同revision重复。types是SOURCE短名/全名合法type集，由当前schema完整扩展，ID/types不是Field参数。bindings按parameter、slot rank(title/body/field)、准确locator/FieldId规范序唯一；同node title最多一个，FieldId只准一次，body ranges不重叠，不与control/Field/title区域混写。

title/body参数必须nonnullable string，无代码/content值。title对唯一native title作安全文字编码；body locator必须同source owner/token的唯一准确inert文本范围，官方AST当前Source读取证明expectedText、完整byte slice与paragraph/content边界；替换只作literal codec，`#link`/`#metadata`文字不执行、不生真链。跨结构/无唯一范围/readonly生成来源拒绝此binding，不全文件字符串替换。field参数以真实installed FieldId匹配完整valueType/nullable/enum，按SOURCE全Field原生容器writer设值，数组保序/重复，不旧field_append/Entry UUID。函数/import负责呈现，Recipe不执行任意宏或赋权限。

录入选择模板→参数/schema检查→获权读取完整source/resources与必要Annotation index→analysis/loss/真实Source预览→Core同一candidate map创建freshNode及必要Resource→最终验证并单一保存。只有Core分配id/出生/placement/权限；标题/文件名不猜分配。资源显式re-own；Annotation首代模板省略须获权完整index和明确omission，不扩为所有导出/复制丢Annotation。完整实例一个coupling group，不能拆绕1000 descendants/更窄预算。recipe/evidence放新template role，旧Recipe2与TemplateConstructionInput2保原历史，built-in adapter以外不生产授权construction。

### 7.1 完整构造输入、Resource/loss与同map效果

```text
TypstTemplateInputPin/1={sourceVersion:SourceVersion/2,
 observation:SourceObservation/1,
 payloadKind:"exact_source_document"|"resource_bytes",pin:PinRef/2}
TypstTemplateSubjectBinding/1={sourceRef:NodeRef|ResourceRef,
 subject:<D3 new_result_subject>}
TypstTemplateLossLocation/1={kind:"template_source",sourceVersion:SourceVersion/2}
 |{kind:"template_annotation",target:D9EntityVersionAddress/2}
TypstTemplateLossChoice/1={lossKey:Counter,choice:"accept_loss"|"reject"}
TypstTemplateConstructionInput/1={version:1,
 construction:TypstTemplateConstruct/1,recipe:TypstRecipe/1,
 inputPins:[TypstTemplateInputPin/1...],
 omittedAnnotations:[D9EntityVersionAddress/2...],lossReport:LossReport/1,
 lossChoices:[TypstTemplateLossChoice/1...],
 sourceSubjectBindings:[TypstTemplateSubjectBinding/1...]}
```

未变叶子精确复用原owner：SourceVersionRef/1是D6当前受保护观察引用（不是production token）；Resource/recipe选择、inputPins/sourceSubjectBindings及D9EntityVersionAddress/2来自`d6-file-authority-reopen/owners/d9-templates/source.zh-CN.md` §6a和workers-export §3b。construction.template必须Node，recipe必须由其owner拥有的Resource；recipe.pin独立strict解析与constructionInput.recipe逐字相等。resources按完整D3 RefKey排序唯一，全部Resource owner在recipe.nodes中，sourceToken须同cut当前获权观察，不能由路径/hash猜。父/集合destination沿原D7资格，TypstCollectionCall/1精确为`{call:TypstQueryCall/1,nodeColumn:text}`，沿原CollectionCall闭合成员只把其中call换§5.4新Call，不给collection或ParentPolicy开放新成员。

inputPins按完整RefKey排序唯一，恰覆盖实际recipe bytes、所有Node源、所选Resource；sourceVersion=observation.sourceVersion、实体/domain/payloadKind/pin字节逐项一致，Document strict UTF-8原字节，不重排语义text，Resource bytes原样。Node地址通过同sealed managed token解析到该完整sourceVersion，内部inputPins可存真实版本，不在作者源embed ChangeId。每sourceSubjectBinding恰为所选Node/复制Resource，subject严格原D3 new_result_subject同kind，Resource owner机械指所选owner的fresh Node subject；源/目标各唯一且与request.freshSubjects、proposedInputs和同private map完全闭合，未复制recipe不入bindings。此map不对外造新identity；展示仅join它与真实receipt.resultAllocations，不用数组位置/标题。

omittedAnnotations是同cut完整获权owner目录中的D9EntityVersionAddress/2按RefKey排序数组，首代template全部omit，不读body、不创建subject；无权/缺负范围不能当空。LossReport/1严格复用原Import IR/Mapping §7闭合`{version:1,items:[{lossKey,feature,locations,effect,severity,allowedChoices}...]}`，feature是原具名闭集，effect非空、locations非空且仅本TypstTemplateLossLocation：sourceVersion在inputPins中，Annotation地址在omittedAnnotations中，不混IR SourceLocation。lossKey连续，notice choices=[]，requires_choice=[accept_loss,reject]，blocking=[reject]。lossChoices按key排序唯一、恰覆盖requires_choice/blocking且不含notice；reject取消无author请求，blocking不能prepare，真正constructionInput保存无blocking的初始完整报告及所有requires_choice的accept_loss，仍保初始severity/allowedChoices。新输入/参数/资源/目录/schema/损失漂移重新analyze/confirm，不沿旧key接受。

正向构造链以三fixture明确，不把普通用户笔记写成内部wire：**TP-01**获权单Node模板含一个唯一正文文本slot和已绑定`user/report-code`字符串Field，两个required nonnullable string参数`body`/`code`对应body locator与FieldId，资源/批注/loss/definitionTransfers为空；当前inputPins恰recipe+Node，sourceSubjectBindings一项source Node→new Node subject，Core同map设fresh控制ID/目标types并安全替换body/Field，两遍全parse后冻结source，完整effect/receipt恰该Node且无D7 transfer效果。body值包含`#metadata`只呈现文字。

**TP-02**同一live template树两个Node A/B已有各一个已认证Feature定义，typed parameter/default Ref互指A/B；recipe.nodes两项父先子后，bindings可空，资源选择为空，inputPins恰recipe+A+B，sourceSubjectBindings对应两个new Node subjects；从真实before Feature payload枚举所有Ref slots，按同map变到A'/B'，production地址都用同plan各自冻结revisionToken，两遍source/Field/link/definition物化及新参数decoder验证；Effects两item的before/after slots完整为真实A/B→A'/B'，receipt实际分配一一join。模板已有互引是copy materialization正向，不能用尚不存在的caller UUID新增fresh互引；collection分支仍拒多Node/需fresh map引用。

**TP-03**单Node含无Ref的合法Feature（如§5.1列表Query），仍产生一条TypstDefinitionTransferPlan.transfer和Effects item、slots=[]；before/after完整payload/source、result地址新token均核，不能当opaque跳过。所有fixture都在完整获权cut、strict/managed_atomic、实际source/recipe/Registry pins/预算之下才正向；错参数型、重复source/subject、资源owner不对应、body locator过期/跨结构、缺typed槽或假receipt令整构造拒绝且无分配，不拆批、不换map补救。

导入以Weftext自有Rust基础设施为生产权威。原SourceArtifact/ImportIR coverage/mapping/loss/ImportJob保留；Office全sheet/hidden/formula-cache/part覆盖、精确数值与坐标、安全validator不能省。输出按目标Typst native AST构造并由官方parser/compiler及SOURCE/Registry/typed-ref完整验证；PDF能编译不等于业务metadata/Query/链接保真。旧AsciiDoc/YAML/Markdown/JSON Entry导入保留原文件，逐项映为root声明/native值/feature carrier/link，复杂事实模型不以新笔记JSON fallback恢复，必须显式loss/拒绝或作为普通附件保留。未知方言、LaTeX/Mermaid兼容只走具名成熟package/provider、支持范围和loss，不能声称native math等价任意LaTeX。

ImportJob仍由D6拥有，有限coupling groups顺序batch；已committed前缀不rollback，unknown恢复原OperationId/request，取消只阻止尚未planned后续批；同digest重导不承诺upsert/dedup。格式改变不虚构部署迁移，不读旧history时覆盖它。

## 8. 输出、离线路由、内网下载与扩展

native Query结果的准确导出使用独立`{format:"weftext.typst-query-result-export",version:1,schema:TypstTerminalSchema/1,data}`：rows的data为完整V行数组，scalar为单个V，graph精确`{nodes,edges}`各完整V行数组。包含本profile完整type/V与bag/order，不含Query定义、runtime handle/cursor/Action权；旧weftext.query-result-export1只消费旧TypeSpec/V，不加native未知arm。DOCX/XLSX等把新scalar呈现为准确文本或真正支持的typed cell，float保roundtrip显示/raw bits来源、duration按exact seconds、date/time/localdatetime不补zone；目标无法表达精度/nullable/结构时报告具体loss并需显式choice，不先把native值近似或漏字段。准确native result export仍无需renderer。

### 8.1 真实能力与封闭 render binding

准确源码导出交付原`.typ` bytes，generationPolicy=none，不依赖编译/模板/renderer，也不按链接偏好改源。准确Resource与query_json仍独立；query_json只含完整terminal schema/data/typed值，不承载Query定义。Typst官方PDF/SVG/PNG用于分页/图形输出；PNG无可点击链接不能冒同等导航。HTML在0.15.1是实验输出，流式阅读/无障碍见EDITOR的实际交付门；不可把“任何保存成网页”替代正式Word/工作簿需求。[官方导出状态](https://typst.app/docs/reference/html/)、[官方实现](https://github.com/typst/typst/tree/v0.15.1)

DOCX/ODT、XLSX/ODS、CSV/TSV是Weftext具名转换能力，不是Typst内建。Word需真实段落/style/table/资源、CJK/RTL/分页及loss检查；XLSX需真实workbook/cell typed值/顺序/公式文本安全/merge与Office/WPS/LibreOffice互操作，PDF表格不冒workbook。保留D9可见Office模板token/最短唯一native-table selector/样式/repeat准入，SOURCE新table producer替代AsciiDoc来源；无unique projection不可按标题/第一表猜。复杂合法Typst无对应转换时明确loss或route unavailable，仍可Source保存。

`TypstExportBinding/1` 精确 `{version:1,sourceInputs:[{owner:NodeRef,sourceVersion:SourceVersion/2,observation:SourceObservation/1,sourcePin:PinRef/2,snapshotPin:PinRef/2}...],environment:CORE TypstProcessorEnvironment/1,worldProof:CORE TypstWorldProof/1,presentation:"flow"|"paged",resultPins:[CORE TypstResultPin/1...],viewSpecs:[TypstViewValue/1...],route:<original D9ExportRouteBinding/1 or null>,linkRoutes:[TypstExportLinkRoute/1...],linkLosses:[TypstExportLinkLoss/1...],outputs:<original D9ExportStagedOutput/1 array>,lossReport:<original ExportLossReport/2>}`。此binding仅用于真实渲染，exact-source无binding；flow使用EDITOR `flow-semantic/1`生产renderer：官方paged realization后的语义投影→受控DOM或native可访问节点；environment.target=paged，不以实验HTML作默认。paged预览同样用paged target。实验HTML只有显式选择且具名route可用时才environment.target=html，不创造独立shared policy。sourceInputs的snapshotPin逐个绑定CORE真实TypstSourceSnapshot/1/environment/world/profile；实际消费的来源、模板、字体、asset与resource各自先授权；不为纯Field/Query导出加whole-body读权。resultPins/viewSpecs按实际配对保留完整cut/auth/epoch/schema/order，renderer不重跑Query/补查/聚合/采样。semantic chart沿原六layout export route：metric/bar/line/scatter/pie/heatmap；network源View合法但该route unavailable，不能偷降data export。

该新export role与CORE冻结confirmed plan组合，保留D9 prepare→inspect→confirm→delivery/state/cancel、受控output name/alias/prefix/reserved冲突、完整staging/验证/预算与当前授权。它不是旧ExportPlan4/D9DocumentRenderBinding1的新unknown arm；历史Plan1–4/confirmation/receipt按记录原恢复。

`TypstExportLinkLoss/1={occurrence:TypstCompiledLinkKey/1,lossKey:Counter}`关联实际完整初始ExportLossReport/2的准确loss项，key真实存在且source范围/输出位置可复算，不另造loss权威。每个§2.4 active app occurrence恰在linkRoutes或linkLosses出现一次，二者不重叠；均按compiled index排序且owner/sourceVersion/compile/projection pins与绑定basis一致。loss也不能漏链接或越权披露隐藏目标。route及loss消费完整projection后才能confirm，旧Plan4未扩这种槽。`TypstExportLinkRoute/1` 精确 `{version:1,sourceOwner:NodeRef,sourceVersion:SourceVersion/2,occurrence:TypstCompiledLinkKey/1,target:NodeRef,authorLabel:text|null,outputName:text,outputFragment:text|null}`。producer在冻结输出集合中逐occurrence证明获权目标、准确label及受控output destination；consumer验证target→唯一输出、fragment对应实际renderer named destination/HTML id，而非假定原label等于HTML id。官方可能改label为DOM id，必须验证实际映射。source occurrence/target proof/输出名冲突、漏route、重复label或未导出目标不能生成貌似可用的离线路径；明确保留应用URI或loss/拒绝选项。route不重写作者源，不用于runtime权限。输出可证明的闭合集合才获得跨文档离线链接承诺；SVG/PNG等目标无法表达的link损失逐项记录。

### 8.2 内网工件下载与发布区别

WebUI内网下载不要求外部目录或PublicationReceipt。用户选择输入/格式→完整工件生成验证→检查准确loss/输出列表→确认同一冻结plan→显示job/delivery状态及获权dataFile下载。确认绑定相同inputs/loss choices/staged bytes，不以worker exit0或preview冒成功。token绑定plan/confirmation/准确dataFile/current audience/有限retention，不能承载本机path或ambient资源读权。每次请求、每个chunk及resume range都重验当前auth与retention，撤权立刻停止后续交付，不泄隐藏file count/name/error。授权和验token先于存在性诊断。

“一次下载”指一次明确交付动作，不是一次token读取后永远免auth；重试/resume只续交仍保留的**相同文件 bytes**，不重新编译。过期/撤权/中断/unknown分别显示；过期重新生成是新plan和新确认，不自动重跑publish或交付新bytes。内容类型/受控filename/长度绑定实际dataFile；输出错误保隔离、不交worker任意额外文件。有限job与取消不删除author Draft。

外部bundle发布仍create-only staging+flush+可证明atomic rename/receipt；unknown只恢复原intent，不能换名伪成功。print是print outcome；“保存为Resource”另需Core author write/receipt，export确认不授写权。这三者及纯下载分别显示，不互代。

### 8.3 包、插件与扩展

内网显式预装/准入精确package/font/plugin/provider及版本digest/license/platform；World仅按冻结环境供应获准资源，不静默联网。Typst函数/WASM包编译能力不授Core/Server network/secret/Action或author事务；D10获权connector/action仍走原能力目录、catalog和Core预算/准备/确认。D9 Worker默认无网络、无Workspace mount/path/command权限，只消费host分配immutable slots，声明输出独立验证。未知profile/variant不任意fallback；Mobile转换执行仍沿D1不可用边界。恶意包/ZIP/XML、超budget和process tree未退出保持拒绝/quarantine，不能accept loss绕安全资格。

## 9. 具名替代与原选择保留

| 固定旧owner/section | 本文新owner/处置 |
| --- | --- |
| LINKS §§1–5、LINK-01–10；EXPERIENCE §5.2 | §§2–3替代AsciiDoc xref/codec/source writer；保留四域、默认ID、shortest操作、权限/同目标/复制。旧“ID无anchor槽”后置改为新typed Node+authorLabel，明确非durable Locator。 |
| LINKS §6/LINK-11；D9 §12/12a、D9-SCHEMAS §§9–14、D9-INTERFACES §§7–9 | §8与新export role替代语言耦合binding；exact-source/query_json/typed Office/真实staging与delivery保留，新route闭合离线目标/fragment。旧Plan不扩宽。 |
| EXPERIENCE §5.1、UX-10–13/17–18 | §4唯一保留并替换native link encoder/Source operation；EDITOR负责实际焦点/IME/RTL/AT执行验收。 |
| D7 §§4/7/9/11–14/18；D7-SCHEMAS §§3–5；D7-SEARCH §11.5；d7/owners/definition-transfer §§1–6 | §§5–6替代保存定义块、旧逐值来源、JSON位置物化与语言耦合载体；保留原查询图、表达式、动作、完整性、历史版本，以及当前贡献和回收站证明。 |
| D7-QUERY-V2 §§2–8、D7-SEARCH §§1–11；d7/owners/view §§1–9；EXPERIENCE §§3–4 | §5保留搜索、字段/正文来源分域、View闭集、scope与新carrier/cold reopen；schema基础值按SOURCE收敛。 |
| D3 §§6/8/11/14/21、D9 §6、D9-SCHEMAS §§5–7；EXPERIENCE §§3/6 | §§3/6/7与CORE新construction替代源物化槽；保留same map、fresh birth/owner、版本/currentness、receipt allocation与单一提交；旧symbolic/locator/history不改。 |
| D9 §§3–5/8–11/13–15、D9-INTERFACES §§1–6/10–12 | §§7–8替换Typst生成与环境输入；保留IR/coverage/loss/typed values/ImportJob/Worker/region/权限/原恢复，Annotation读取仍由EDITOR/Core当前owner。 |
| EXPERIENCE §§1–2/5/7–8 | 写作并排/大纲/布局/批注导航保留到EDITOR；本文件保留已选Query/链接/模板输出，后置仪表盘/复杂表单/白板/学习/朗读/同步/任意脚本/一键回退/实时光标不扩scope。 |

原21项关闭证据仍属于原固定候选；受新carrier、source/Field projection、typed transfer、Effects、query/read/save、download与格式binding影响的链按本轮重验，不能称旧CLOSED自动接受。本候选保留原可观察语义，不继承Asciidoctor 专用语法/类型/坐标证明。

## 10. 正反设计验收与限制

| ID | 正例 / 必须拒绝或明确降级 |
| --- | --- |
| TF-01 | 四地址形式同Node+label、text/content不变；普通`/x`不是库根，重复标题不默认选择。 |
| TF-02 | 空格/中文/百分号/+精确一次编码；badUTF8/lowercase逃逸/encoded slash/backslash/../越界/alias/URI query拒绝应用codec，source保留。 |
| TF-03 | Node改名ID仍同对象，label改名只片段失效；重复/缺label/无目标read不可无损转换，不删fragment。 |
| TF-04 | path不可披露只提供可披露ID；Server不输出磁盘URI，未连接跨库不猜同名target。 |
| TF-05 | 一次转换一次Undo/Source full reparse；共享变量/include/ABA/撤权/stale不写；断链原样复制可用而转换复制须验证。 |
| TF-06 | title+aliases同范围多对象、nested/cross ranges/max2/overflow/>20/keyboard/touch/RTL完整可达；关闭/取消不改source。 |
| TF-07 | combining/ZWJ/CRLF及scalar↔byte↔UTF16差异不扩选区；生成/真实link/metadata无可写提示，未完scan不冒全集。 |
| TF-08 | metadata literal定义独立读，正文编译失败仍Source读/保存；duplicate key/label、计算Query carrier、旧QuerySpec混新address/none冒null不可执行。 |
| TF-09 | 表/板/历共享Task字段+正文；computed/aggregate/旧row拒写；contact array重复值只改准确当前第二项，无事实UUID。 |
| TF-10 | generated heading行可读但无source writer；distinct/global aggregate不继承代表行权限；partial result不提供complete/action/membership外断言。 |
| TF-11 | contribution冷重开same ID/version/descriptor/owner；停用C但Field存在不替代；Trash Coverage同cut完整，hidden不漏count，Trash不授Action。 |
| TF-12 | 复制、链接、字段、定义使用同一候选身份映射、修订与收据；未知定义或完整分叉缺槽整体拒绝，表达式/普通文本UUID不替换；真实旧效果/结果按原历史解码器恢复。 |
| TF-13 | native import/function复用排版；假id/模板metadata/包不能birth或授权限；参数body不执行任意代码，unknown create不重复。 |
| TF-14 | 导入whole IR coverage/typed值/完整loss和source preview；默认转换metadata/Query/xref丢失不得因PDF成功放行；原输入保留。 |
| TF-15 | 真DOCX/style、真XLSX typed cell与Office/WPS/LibreOffice检查；PDF表格/实验HTML/无route不冒同等支持；source读写独立。 |
| TF-16 | export occurrence→target→actual输出label路由可回读；漏/重复/target不在集合明示loss或拒绝，不猜HTML id，不改source。 |
| TF-17 | 内网无外部目录/receipt可交准确confirmed dataFile；逐chunk auth、同bytes resume、过期新confirm；坏token不泄存在/path，worker额外file不交付。 |
| TF-18 | 下载、打印、资源创建和只新建发布各自展示真实结果；未知状态恢复原意图，导入任务保留已提交前缀，包和转换进程没有环境隐式权限。 |
| TF-19 / TR-01 | QP-01两份源码在真实已封印被调定义与已安装字段方案下，浮点参数及默认值、QueryRef同型传递、空实参取默认、原生浮点转准确位串、排序/投影/准确导出正向；动态块的当前节点、标题和运行时上下文都走新字面值解码器。旧字面值混新类型、整数冒浮点、坏位串/非有限数、默认类型不符、缺必选或多余参数、QueryRef越用行/标量/当前节点绑定及假修订令牌均拒绝。 |
| TF-20 / TR-02 | +0/-0同eqKey且两raw bits保留；Z与+08:00同UTC instant同key；exactseconds、native decimal不同scale、递归array/dictionary/nullable/union依定义相等/不等。distinct/group/count_distinct类数、raw最小代表与K不随输入序/分区/页界变；global K无parent。错误type、hash collision只比hash、截树、raw bits直接作EqKey、cache以eqKey替实际raw参数均拒。 |
| TF-21 / TR-03 | QP-01 parameter.type/default.type/View.inputSchema任意深度typst_array.maxItems:none准确等null且不改有限N；none仅该schema槽。raw V.none/null、default:none、缺optional成员用none补、nullable absent与none混同、向unknown schema字段放null均拒。 |
| TF-22 / TR-06 | 两个同字面root links各AuthoredKey；include/show重复同span产生两个compiled index，同一换行链接多个reading fragments仍一个index；完整AST/final trace/association重算一致。nonliteral目标可为catalog null但无writer，Draft新字节不能用旧catalog资格。 |
| TF-23 / TR-06 | 每个active app compiled key恰route或同key loss，actual target/fragment/output逐项验证；缺/多/重复项、错误projection/compile/world cut、originId或DOM key冒occurrence、重复include合并/半coverage冒complete、removed_by_show冒可点击输出均拒。 |
| TF-24 / TR-09 | TP-01 body+Field参数单Node安全物化，TP-02已存在两Node/definition typed Ref互指同map重发，TP-03无Ref transfer/effect仍slots=[]；完整pin→construction→两遍source/typed槽→Effects→唯一receipt逐项对应。同一完整源定义显式发到两个获准不同Node结果subject且label相同，按请求完整subject键得到两项；同map物化后的真实Ref序可相反，Effects重排后仍由完整源/结果键和真实收据映射准确对应两项。 |
| TF-25 / TR-09 | 参数default仅未提供时生效且保native scale/bits；资源当前Ref/owner、recipe pin、omitted Annotation目录、初始loss/accept选择全部正向匹配。body文字含执行语法仍literal；wrong parameter/valueType/nullable/FieldId、过期或跨结构body range、资源owner错、重复/缺subject、漏effect path、before/after错kind、伪candidate UUID、loss notice额外choice/blocking接受/报告漂移、receipt位置猜map均整操作拒绝。相同或不同源重复同一完整结果subject+label、effects重复完整结果地址、遗漏源修订token的键、按JSON或ordinal字符串排序、按请求位置关联重排效果、同源多实例仅按源或label配对均拒绝。 |

这些oracle尚未执行。研究只证明有限官方AST/value/编译/源splice可行，不证明ACL、全库性能、跨设备、IME、Office互操作、离线路由或生产HTML。实现时须将上述案例接入真实Core/Server与EDITOR测试，并保留原D9 I01–I12及对应安全/平台/无障碍门；设计文档不是产品通过证据。
