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

`TypstLinkTarget/1` 是闭合应用投影：`{version:1,form:"node"|"relative"|"workspace"|"file"|"native",nativeDestination:<native destination>,nodeRef:NodeRef|null,authorLabel:text|null,sourceOccurrence:<SOURCE current origin>}`。nodeRef 只对严格节点 URI 由获权完整 identity resolver 生产；路径解析得到的 transient target 另留在运行时导航证据，不能填入此 asserted identity arm。nativeDestination 保留原生 string/label/location/page-coordinate 类型与实际来源，不用页面坐标补造 authorLabel。label/ref 是当前编译文档目标；URI fragment 必须在已获权目标的当前源/实际依赖中证明**准确唯一** label，重复、缺失、计算生成无法独立证明时导航报告范围不可用，无损转换禁用。

节点身份加作者 label 已支持同目标 fragment 的本轮转换，但 label 是作者可改名字，**不是** revision-bound Locator；不宣称永久锚点或自动重锚。源内无 fragment 的 ID 导航不强加 target body read；证明 fragment 则须获准确目标来源/label 的读取资格。

出链列原生真实 link/ref occurrence 与地址形式；稳定入链/asserted graph 只消费严格节点 URI 的完整 NodeRef 及 SOURCE 已验证根作者来源。generated/include/multi-origin 链接可显示获权的编译/导航结果，标为派生，不能作为稳定作者 edge 或 identityMap rewrite 槽。路径链接必须同样提供“当前解析入链”：在获权current cut按完整源occurrence/实际path resolver构建派生index并列于入链面板的独立分组，source/target改名、移动、替换、auth或负范围变化即失效，partial scope不冒完整无入链。当前path关联标transient，不升为durable identity inbound/asserted graph，但不取消其入链导航功能。潜在提及不进入真链/graph/reference integrity。标题与别名只供候选选择，重复标题不能合并对象。链接健康只在当前获权且相关范围完整时说“全库无入链”，否则说“当前可见范围未找到”；不泄隐藏数目，不自动删孤立笔记。

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

`TypstQueryValue/1` 是现任 QuerySpec/3 的**逐成员原生表示**：全部八成员必需，format/version、lifecycleScope 和 searchBindings 闭集不变；dictionary→object、array→array、bool→bool、string→text，Counter 只接受精确有界 Typst integer；D7 arbitrary integer/decimal canonical text 仍为 string，不经 float。`(:)` 是空 dictionary，`()` 是空 array，单项数组需逗号。D7 本来不允许 null 的位置不接受 Typst none；Optional.none 仍使用 D7 已规定的显式 typed Optional arm。没有 expression eval、spread、循环、导入值或 JSON 字符串 fallback。schema 声明的 NodeRef typed 位置用§2无 fragment string解析完整 Ref，其它 Resource/Annotation Ref 仍按原 typed shape 的原生 dictionary映射；不能扫描普通字符串猜 Ref。未知成员/重复 key/不适用 type 拒绝应用投影，保留 source。

现任QuerySpec3/ViewSpec1作者对象及TypedLiteral/V没有合法JSON null arm，所以此carrier内**所有**Typst `none` 都拒绝；Optional无值必须写`(state: "none")`。其它本文具名应用对象若明确有null arm（例如link的authorLabel），内部wire的null与原生参数none一一映射且仅在该槽合法；不把这条规则扩散到Query。下面是完整可编译的原生载体例，全部八Query成员和合法ViewSpec列类型/绑定齐全；实际运行仍需要Core获权cut，不以metadata编译成功代替Query执行。

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

CEL profile仍唯一，现有text/bool/int/decimal操作精确保留。新增types支持同型`== != < <= > >=`（仅表中orderable），禁止float↔int↔decimal、time/date/local_datetime↔instant隐式coerce。float同型四则严格IEEE754 binary64 round-to-nearest-ties-even、每primitive单独舍入不FMA，divide-zero/nonfinite/overflow用原错误；duration同型加减checked i64，exact_seconds同型用原exact decimal规则。date/time/local_datetime/instant只提供精确比较与原显式temporal-provider函数，未定义运算type_mismatch/unsupported_feature；不能据此拒read/project/sort/exact export。复合类型按原明确CEL member/list规则作用于其typed值，unknown preserved只能显式取只读native值，不提供动态Field/Action。新counter/容器/每值bytes计原完整budget。

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

## 6. 新 feature holders 与 typed transfer

native Query入口是独立`TypstQueryExecute/1={kind:"typst_query_execute",version:1,workspaceRef:WorkspaceRef,commitDomain:CommitDomain/2,expectedFrontier:Frontier/2,call:<native QueryCall>,budget:<original D7 Query budget>}`，返回上述TypstQueryResult1。旧d7_query_execute/wire2不在原JSON闭集中猜native source或加新arm。Core保护的native result profile标记决定Type/V/schema page消费；所有入口同原授权/错误顺序，不增第二执行器。新nullable值的CEL具名函数`nativePresent(x)`、`nativeIsAbsent(x)`、`nativeIsNone(x)`分别读精确state；`nativeValue(x)`只在value态返回T，否则原none_value。`nativeVariant(x)`返回冻结variant名，`nativeCase(x,"variantN")`要求literal合法name、匹配才返回该branch T，否则原type_mismatch。函数不coerce、都计原预算，前三state测试total，其它可error；unknown preserved访问只读，不创造Field身份。

以下是 CORE `FormatAdapterEnvelope/1` 的具名角色payload，不是给旧PAB4、Result9/10、ExportPlan4或closed union加未知kind。generic D6 SourceVersion/Observation、Frontier、InputDescriptor4/Proof4/PIntent4、唯一CAS/P/receipt按CORE保留，document_format携准确Typst profile pin；所有saved/planned/unknown predecessor先原record查找/恢复，不重pin/换版本。

`TypstDefinitionInput/1` 准确 `{version:1,address:TypstDefinitionAddress/1,observation:SourceObservation/1,sourcePin:PinRef/2,value:TypstFeatureDefinition/1}`，受保护record完整绑定准确occurrence、作者值和实际SOURCE环境；address/value不是caller认证。action_input/prepared_action payload逐槽采用CORE §4表：action消费下列新Spec，definitionInputs消费本Input，registryInputs消费SOURCE `TypstSchemaSnapshot/1` qualification，proposedInputs消费CORE proposed_input envelope，constructionInput消费新template role。request/proof/sourceInputs/preview/audience/expiry/budget与冻结输入逐项相等；不另建binding store/ledger，不再声明一份同形TypstActionInput。native QueryCall准确沿原 `{query,arguments,context}`，仅inline.spec变本profile `TypstQueryValue/1`、saved.definition变本`TypstDefinitionAddress/1`，参数/上下文严格原D7 decoder，不加字段。

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

`TypstDefinitionTransfer/1` 精确 `{version:1,sourceAddress:TypstDefinitionAddress/1,resultOwnerSubject:<CORE subject>,resultLabel:text,slots:[{path:<schema member/index array>,before:<complete typed Ref or TypstDefinitionAddress/1>,after:<closed map/preserve/reissue_definition choice>}...]}`。producer从完整前像官方AST/native decoder枚举**所有**typed槽，consumer独立重枚举比较；无Ref定义也须slots=[]证明。path按member unsigned UTF-8/index数值排序，unique且无祖先重叠。labels、CEL strings和free text不扫描。map/preserve沿Core原identity/locality，reissue_definition只选真实同map目标definition label；最终地址用同一新owner/冻结revisionToken，不保留旧地址。

`TypstDefinitionTransferPlan/1` 精确 `{version:1,sourcePins:[{owner:NodeRef,observation:SourceObservation/1,pin:PinRef/2}...],transfers:[TypstDefinitionTransfer/1...]}`，pins按完整Ref排序唯一，transfers按source owner/ref+label排序唯一，所有sourceAddress恰被准确sourcePin覆盖；same source可多次显式实例化但resultOwnerSubject+resultLabel唯一。after选择精确为`{kind:"map",subject:PayloadSubjectKey}`、`{kind:"preserve"}`或`{kind:"reissue_definition",ownerSubject:PayloadSubjectKey,targetLabel:text}`；最后一臂要求真实映射的完整原definition address及新唯一label，不能改为别的同名定义。CORE construction的transferPlanPin绑定此准确完整对象及同source版本，不是自由JSON或另一个候选map。

`TypstDefinitionTransferEffects/1` 准确 `{version:1,decisionKey:DecisionKey/2,transfers:[{sourceAddress,resultAddress,beforeSource:text,afterSource:text,slots:<actual before/after values array>}...]}`；这是新role内typed效果，原D7Effects2字节只留原恢复，不换tag。前后完整源与实际覆盖槽独立验证，结果数组按source owner/ref+label规范排序，**普通Query/View作者数组仍保留作者顺序**。

Core materialization固定同一private candidate map、revision plan/token和新定义标签；第一遍写全部typed Ref与待重发定义地址的固定内部占位版本，官方重解析确定真实定义集合；第二遍填同一拟议revisionToken字符串/owner/label（不把seal后才产生的ChangeId嵌进源），官方重解析验证全部地址、SOURCE根声明和typed值。地址不含byte坐标，故不沿旧JSON一行Q假设；任何定义不唯一/占位残留/前后slot覆盖不等/版本改变整体拒绝，内部候选不交付。Field/link/definition/Annotation重写共享同一map与最终source，winning CAS/P才形成生产版本、ChangeId、token的真实seal artifact与有效receipt；seal不再次改写after source；loser不借另一map/seal。未知feature可源读/保存，但copy/fork的typed transfer不能默默跳过。

## 7. 原生模板、出生与导入

所有作者内嵌production地址（definition/QueryRef/recipe template与nodes.source）均使用上述opaque revisionToken，不嵌未知seal ChangeId。token不是URI片段，不百分号decode；严格原43字符token decoder拒绝padding/错tag/非规范base64url。create/copy/fork materialization在同plan私有生成并冻结随机token，两遍完整源写入同token+label后冻结afterPin；原seal artifact才将它签到实际完整SourceVersion/ChangeId，seal不回写source。消费者必须验证真实历史artifact、identity与生产源binding，再fresh currentObservation；unsealed/caller伪token、相同stamp/bytes异token和强改current地址均无资格。export等受保护内部证据仍可存真实fullSourceVersion，不能把所有证据机械token化。

排版模板用普通Typst module/import、函数参数与set/show。用户可复用成熟模板而非只能固定title/body slot；World只供应获准冻结的包/字体/资源和只读输入，函数不能读取ambient Workspace、创建Node、生成可信id/birth、授权限、写sidecar或调用Action。普通作者metadata在root独立，不由模板返回的同标签metadata认证。

应用创建模板使用 `TypstRecipe/1` 精确 `{format:"weftext.typst-recipe",version:1,template:<authorized Node production address>,parameters:<SOURCE schema parameter definitions>,nodes:[{source:<authorized Node production address>,parentIndex:<earlier index or none for root>,types:<explicit type array>,bindings:[{parameter:text,slot:"title"|"body"|"field",fieldId?:<SOURCE FieldId>}...]}]}`；仅slot=field要求fieldId，其余禁止。nodes source唯一、一个根、父先子后，参数具名unique且全部bindings类型匹配；body参数为内容数据，不能插入任意代码。函数/import参与**呈现**，应用recipe负责哪些获权source/资源实例化，不把Typst执行当Core动作。

录入选择模板→参数/schema检查→获权读取完整source/resources与必要Annotation index→analysis/loss/真实Source预览→Core同一candidate map创建freshNode及必要Resource→最终验证并单一保存。只有Core分配id/出生/placement/权限；标题/文件名不猜分配。资源显式re-own；Annotation首代模板省略须获权完整index和明确omission，不扩为所有导出/复制丢Annotation。完整实例一个coupling group，不能拆绕1000 descendants/更窄预算。recipe/evidence放新template role，旧Recipe2与TemplateConstructionInput2保原历史，built-in adapter以外不生产授权construction。

本节Node生产地址精确为`TypstTemplateSourceAddress/1={version:1,owner:NodeRef,revisionToken:Token}`，读取按§5生产版本认证/currentObservation/profile原则，完整源不是相同hash或filename推断。`TypstTemplateConstructionInput/1` 精确 `{version:1,recipe:TypstRecipe/1,parameters:<validated native dictionary>,inputPins:[{address:TypstTemplateSourceAddress/1,observation:SourceObservation/1,pin:PinRef/2}...],resources:<original explicit authorized Resource selections>,omittedAnnotations:<exact authorized version-address array>,lossReport:<original LossReport/1>,lossChoices:<complete original choices>,sourceSubjectBindings:<unique source-to-CORE-subject array>}`。集合规范排序/unique，sourceSubjectBindings与实际request fresh graph恰对应；instance展示只join该mapping与真实receipt allocation，不另造身份map。

导入以Weftext自有Rust基础设施为生产权威。原SourceArtifact/ImportIR coverage/mapping/loss/ImportJob保留；Office全sheet/hidden/formula-cache/part覆盖、精确数值与坐标、安全validator不能省。输出按目标Typst native AST构造并由官方parser/compiler及SOURCE/Registry/typed-ref完整验证；PDF能编译不等于业务metadata/Query/链接保真。旧AsciiDoc/YAML/Markdown/JSON Entry导入保留原文件，逐项映为root声明/native值/feature carrier/link，复杂事实模型不以新笔记JSON fallback恢复，必须显式loss/拒绝或作为普通附件保留。未知方言、LaTeX/Mermaid兼容只走具名成熟package/provider、支持范围和loss，不能声称native math等价任意LaTeX。

ImportJob仍由D6拥有，有限coupling groups顺序batch；已committed前缀不rollback，unknown恢复原OperationId/request，取消只阻止尚未planned后续批；同digest重导不承诺upsert/dedup。格式改变不虚构部署迁移，不读旧history时覆盖它。

## 8. 输出、离线路由、内网下载与扩展

native Query结果的准确导出使用独立`{format:"weftext.typst-query-result-export",version:1,schema:TypstTerminalSchema/1,data}`：rows的data为完整V行数组，scalar为单个V，graph精确`{nodes,edges}`各完整V行数组。包含本profile完整type/V与bag/order，不含Query定义、runtime handle/cursor/Action权；旧weftext.query-result-export1只消费旧TypeSpec/V，不加native未知arm。DOCX/XLSX等把新scalar呈现为准确文本或真正支持的typed cell，float保roundtrip显示/raw bits来源、duration按exact seconds、date/time/localdatetime不补zone；目标无法表达精度/nullable/结构时报告具体loss并需显式choice，不先把native值近似或漏字段。准确native result export仍无需renderer。

### 8.1 真实能力与封闭 render binding

准确源码导出交付原`.typ` bytes，generationPolicy=none，不依赖编译/模板/renderer，也不按链接偏好改源。准确Resource与query_json仍独立；query_json只含完整terminal schema/data/typed值，不承载Query定义。Typst官方PDF/SVG/PNG用于分页/图形输出；PNG无可点击链接不能冒同等导航。HTML在0.15.1是实验输出，流式阅读/无障碍见EDITOR的实际交付门；不可把“任何保存成网页”替代正式Word/工作簿需求。[官方导出状态](https://typst.app/docs/reference/html/)、[官方实现](https://github.com/typst/typst/tree/v0.15.1)

DOCX/ODT、XLSX/ODS、CSV/TSV是Weftext具名转换能力，不是Typst内建。Word需真实段落/style/table/资源、CJK/RTL/分页及loss检查；XLSX需真实workbook/cell typed值/顺序/公式文本安全/merge与Office/WPS/LibreOffice互操作，PDF表格不冒workbook。保留D9可见Office模板token/最短唯一native-table selector/样式/repeat准入，SOURCE新table producer替代AsciiDoc来源；无unique projection不可按标题/第一表猜。复杂合法Typst无对应转换时明确loss或route unavailable，仍可Source保存。

`TypstExportBinding/1` 精确 `{version:1,sourceInputs:[{owner:NodeRef,sourceVersion:SourceVersion/2,observation:SourceObservation/1,sourcePin:PinRef/2,snapshotPin:PinRef/2}...],environment:CORE TypstProcessorEnvironment/1,worldProof:CORE TypstWorldProof/1,presentation:"flow"|"paged",resultPins:[CORE TypstResultPin/1...],viewSpecs:[TypstViewValue/1...],route:<original D9ExportRouteBinding/1 or null>,linkRoutes:[TypstExportLinkRoute/1...],outputs:<original D9ExportStagedOutput/1 array>,lossReport:<original ExportLossReport/2>}`。此binding仅用于真实渲染，exact-source无binding；flow使用EDITOR `flow-semantic/1`生产renderer：官方paged realization后的语义投影→受控DOM或native可访问节点；environment.target=paged，不以实验HTML作默认。paged预览同样用paged target。实验HTML只有显式选择且具名route可用时才environment.target=html，不创造独立shared policy。sourceInputs的snapshotPin逐个绑定CORE真实TypstSourceSnapshot/1/environment/world/profile；实际消费的来源、模板、字体、asset与resource各自先授权；不为纯Field/Query导出加whole-body读权。resultPins/viewSpecs按实际配对保留完整cut/auth/epoch/schema/order，renderer不重跑Query/补查/聚合/采样。semantic chart沿原六layout export route：metric/bar/line/scatter/pie/heatmap；network源View合法但该route unavailable，不能偷降data export。

该新export role与CORE冻结confirmed plan组合，保留D9 prepare→inspect→confirm→delivery/state/cancel、受控output name/alias/prefix/reserved冲突、完整staging/验证/预算与当前授权。它不是旧ExportPlan4/D9DocumentRenderBinding1的新unknown arm；历史Plan1–4/confirmation/receipt按记录原恢复。

`TypstExportLinkRoute/1` 精确 `{version:1,sourceOwner:NodeRef,sourceVersion:SourceVersion/2,occurrence:<SOURCE deterministic occurrence key>,target:NodeRef,authorLabel:text|null,outputName:text,outputFragment:text|null}`。producer在冻结输出集合中逐occurrence证明获权目标、准确label及受控output destination；consumer验证target→唯一输出、fragment对应实际renderer named destination/HTML id，而非假定原label等于HTML id。官方可能改label为DOM id，必须验证实际映射。source occurrence/target proof/输出名冲突、漏route、重复label或未导出目标不能生成貌似可用的离线路径；明确保留应用URI或loss/拒绝选项。route不重写作者源，不用于runtime权限。输出可证明的闭合集合才获得跨文档离线链接承诺；SVG/PNG等目标无法表达的link损失逐项记录。

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

这些oracle尚未执行。研究只证明有限官方AST/value/编译/源splice可行，不证明ACL、全库性能、跨设备、IME、Office互操作、离线路由或生产HTML。实现时须将上述案例接入真实Core/Server与EDITOR测试，并保留原D9 I01–I12及对应安全/平台/无障碍门；设计文档不是产品通过证据。
