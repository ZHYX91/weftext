---
source_language: zh-CN
translation_status: source
---

[English](AUTHORING.md)

# 笔记作者源：原生身份、类型与业务属性

状态：从 `6b13b03b8da3e9e3d14b8663e91442568c8917e0` 修订的设计候选；本轮独立有界复核状态、适用范围与原条款处置见 [修订入口](REVISION.zh-CN.md)。未实现，原全局审查未完成。本文件唯一拥有新作者源映射；D3 身份规则、D4 值语义、D6 提交与 D8 编辑资格继续由原模块拥有。

## 1. 文件形态与原生语言

一篇受管笔记仍是一份 `.adoc`。笔记 ID、类型与简单业务字段统一使用原生 AsciiDoc 文档头属性，正文保留完整原生语法；新笔记不生成 YAML front matter。字段的数据类型、必填、枚举、基数与关系约束统一由 schema 定义；不要求用户为每个简单值写 JSON constructor。原生属性本身是平面字符串，不声称它原生拥有数组或嵌套对象。

```adoc
:weftext-id: 12d7c4f3-9ee6-4c36-920d-ec6af27bc654
:weftext-types: task
:status: open
:due: 2026-10-20
= 准备项目报告

整理调研结果，形成项目报告。
```

控制属性和业务属性均可放在标题之前、之后或两侧；不得强制移动合法属性。属性与标题遵守原生连续文档头及空行终止规则。无标题文档仍合法。标题、文件名、目录、正文首句都不产生身份或类型。

D2 把完整文件交给同一原生处理链，直接消费其 attribute events、header 边界、原字节范围、条件和 source-origin 证据；不剥离新 envelope、不归零 offset、不另写文档头 parser。Weftext 仅在原生解析之上校验保留名称和 schema 值。YAML 不再是新 profile 的身份/类型来源，也没有与属性互为备用的双源规则；外来 front matter 保留源字节和原生处理结果，若要转为受管笔记，走明确的导入/源修复预览，不能自动删除或从其中恢复 ID。schema、配置或资源文件仍可使用它们自己的既有格式。见 [原生文档头](https://docs.asciidoctor.org/asciidoc/latest/document/header/) 与 [属性名称和值](https://docs.asciidoctor.org/asciidoc/latest/attributes/names-and-values/)。

## 2. 原生控制属性与分类

控制名称是 `weftext-id` 和 `weftext-types`，按原生 canonical name 比较。ID 必须恰有一个 root 文档头直接字面 set，值精确为 D3 lowercase UUIDv4；types 可缺省，存在时也恰一个直接字面 set。控制只接受单物理行、无条件、无 attribute substitution 或续行的真实 root 声明；不将引号当字符串定界符。ID 缺失、空、重复（包括值相同和大小写名称别名）或任一 header unset，以及 types 重复/unset，使受管控制投影不可用，保留获权 Source 读取与修复。普通业务属性仍用 §5 原生重定义规则；这不是对所有原生属性施加唯一限制。

types 的一个字符串使用唯一列表约定：以 ASCII 逗号分隔 token，每项两端仅去除 SP/HTAB；整体为空或缺省表示空集合。非空列表不允许空项、尾逗号、引号、方括号或空白替代逗号。每项严格为既有 D4 单段短名或完整 namespace/name；不把显示名称猜成 token。该约定是 Weftext 的属性值协议，不是原生数组语法。原序列和空白保留，比较用展开后的集合；非法列表不得悄悄降为空分类。

控制资格来自同次 D2 header-end evaluation 的准确 root 声明与完整来源证据，而非最后一张有效属性表。Core 验证声明未受 include、条件或外部 API/environment set/unset 控制，且 header-end 对应值与该直接声明相符；这些外部来源即使写相同值也不能认证声明，缺完整证据则不可用。未激活条件分支不产生控制声明；不能因换环境而采用另一分支的 ID。正文同名 set/unset 仍保留原生渲染行为，并提示它不控制笔记；它不能改变已取出的声明或实体身份。完整声明仍须通过 §3 的受保护 metadata 资格。此验证在同一原生 evaluation 处理文档头期间完成；D2 headerAttributes 的 lexical 列表和最终 effectiveAttributes 均不能单独充当 header-end 证据。只有最终 map 而无必要 trace 的旧 snapshot 不补猜或升级认证；只能按原 pins/profile 完整重求值并取得本次实际资格，或报告不可用，不复活旧 selector。

`task` 是固定内置短名，唯一展开为 `tasks/task`。`template` 是保留的 D2 Core meta-kind token，禁止任何 facet alias 使用或重绑定该短名；完整 facet ID 不会被截短为此 token。它不能与任务等业务 facet 同列；其目标实例类型仍由 D9 template recipe 声明。其它内置或扩展短名来自 §4 的已验证表，完整 `namespace/name` 始终可用。短名与完整名展开到同一 facet 时视为重复；源顺序只影响无损编辑，不影响集合语义。Facet requires/conflicts/effective closure 仍由 D4 处理。示例中的多个类型不是多个 Node，也不产生 TaskRef。

未安装/未知的合法类型标识保留原文，并显示 schema 不可用；不得把它解释为空类型或自动改名。`task` 不能被扩展重绑定。完整 ID、显示名称、中文标签和短名不同；插件安装顺序、禁用状态与本地化不改变已存语义。普通 AsciiDoc 的 `wf-kind`/`wf-facets` 同名属性可以按原生规则存在，但在新作者源中没有身份/分类控制权，也不是控制属性的回退来源。

## 3. 笔记身份与外部元数据的分工

`weftext-id` 是笔记携带的 NodeId 声明；完整 NodeRef 仍为 WorkspaceId + NodeId。D3 分配、birth、tombstone、不复用、权限与生命周期事实仍由 D3/D6 的可移植元数据和原决议证明。文件中的字符串不是权限或出生证明，metadata 也不再另有可编辑的“文档 ID 值”。两者必须按同一次原操作一致安装；不一致是冲突/不完整，不选择一边覆盖另一边。

创建由原 D3 计划预分配唯一 UUID，并把同一值写入控制属性、文件绑定与 birth；D9 template 不能提前填最终 ID。copy/fork/import 的真正新 Node 分配新 ID；managed2 源按原 identity map 改写受管引用与控制属性，保留未知/不可证明内容及原 loss/确认规则；移动/改名保留 ID。真实 managed1 的未见 copy/fork 默认保留其精确格式代际、不插入新控制属性；只有显式选择并预览复制加格式转换才产生 managed2。restore 恢复真实历史绑定，不趁恢复转换。原计划 restart/recovery 使用原冻结 UUID，不重新随机生成。外部编辑 ID 不构成 rename、身份转移或创建授权。

只复制一份 `.adoc` 可以携带原 ID 声明和类型，不能单凭它恢复 Workspace 关系、权限、批注或执行责任。完整备份仍包括 portable metadata、所需资源与独立 `weftext.annotations.json`。同库发现两个文件声明相同 ID 时报告碰撞，不能取第一个、静默合并或重签；用户显式“作为新笔记导入”才走 fresh allocation。metadata 缺失/部分同步时保留可授权的原文读取与修复，受管身份为不完整；不能在重建索引时重新分配身份。

## 4. Schema 对源码名称的绑定

`RegistrySnapshot/2` 是本提案的唯一新注册表快照形状：完整继承 D4 §2.2 的 `RegistrySnapshot/1` 十六成员，并增加必填 `version:2` 与 `authoringBindings`；其 `kind` 保留原值。version 是精确 JSON integer 2，Boolean 不合法。新生产/当前消费只接收这个显式版本，缺 version 不回退为 Snapshot1。原无 version 的 Snapshot1 仅在真实已记录的旧 typed/recovery 域，或明确的旧→新演进 before 槽中使用；after 必须是 Snapshot2。缺 authoringBindings 的 version2 是坏对象，未知版本拒绝，禁止按成员是否存在猜测降级。实际跨模块槽位见 [作者源集成](AUTHORING-INTEGRATION.zh-CN.md)。其余旧成员、所有者、排序、完整性、tombstone/migration 规则不变。`RegistryBinding/1` 仍同时冻结 generation 与覆盖完整新对象的 digest。真实旧 Snapshot1 只按原闭合形状解码，不忽略新成员或读时升级。D10 验证贡献所有者，D4 验证 schema 和绑定；缺失或未知版本不得认作空配置。

```text
authoringBindings = {
  version: 1,
  typeAliases: [{name, facetId}...],
  fields: [{attributeName, fieldId, codec, wrappers}...]
}
codec = "text" | "boolean" | "integer" | "decimal" | "semantic_code"
      | {kind:"calendar_date", calendarId, calendarVersion, precision:"day"}
wrappers = [{variant:<declared union variant>}...]
```

所有对象闭合；数组必须真实为数组，按 `name` 或 `attributeName` 的 unsigned UTF-8 升序且唯一。短名是 D4 的单段 token；attributeName 为原生合法名称且采用原生 canonical name，不能占用任何 fixed-baseline 内置属性名、`weftext` 名称或 `weftext-`、`wf-` 保留前缀。每个 FieldId 最多一个 native binding；attributeName 在一个快照中也唯一。新绑定不是重抄 FieldDefinition；fieldId 必须解析到原已验证定义。类型短名由 facet 的已验证 owner 声明，业务名称绑定作为受授权的 Workspace schema 配置，不由启用插件自行注入。增删/改绑影响旧文件时必须预览所有受影响来源并走原 schema evolution 规则，不能静默改变既有文件含义。

`wrappers` 按外到内顺序选择原 ValueTypeSpec 中的 union；空数组表示非 union。最内层类型必须与 codec 相符，Field 最大基数必须为 1；不相符拒绝整个绑定。calendar codec 完整冻结历法、版本、精度并验证原 comparator/lexeme；不从主机区域或时区推断。bool 只接受 `true`/`false`，integer/decimal 使用 D4 既有 canonical 十进制字符串；text 保留完整字符串，semantic_code 按目标 Field 的闭合 code 域验证。类型和转换规则只存 schema，源值不附重复 type 标签。

内置 task 的初始配置包含 `task→tasks/task`；`status→tasks/status` 与 `priority→tasks/priority` 使用 semantic_code；`due→tasks/due` 选择 `wrappers=[{variant:"date"}]`，使用所选已登记 Gregorian 日精度 calendar codec。该 calendar ID/version 必须在实际 Workspace 的同一快照验证，不能靠示例字面猜测。任务 status 仍必填，值域仍为原 catalog 的 open/done/cancelled。priority 原值域、due 的 date/instant union 和复杂值能力不变；instant 值继续用 §6 复杂载体。不可变 catalog 原字节不改。

保留控制名称不是业务 Field：它们不能出现在 authoringBindings.fields、MapAttribute 的业务映射、FieldSource、FieldSelector、field_set 或其它普通字段写入器中。类型变更走原 FacetOperationRequest3 的权限/closure gate 和完整 Source 预览；ID 只由原身份生产/转换流程写入。新属性值不能绕过这些原边界。

任务等逻辑分组由已有 FacetSchema.fields/relations 和已验证 Field 绑定构造；例如 status、due、priority 可显示在“任务”组。界面分组不产生一个嵌套 task 值，不需要额外空 task 属性，也不复制 FieldDefinition。多 facet 共用字段仍是同一事实，界面可按设备偏好固定/折叠；不能因为字段前缀、分组或界面隐藏而改变数据类型、membership 或权限。是否在名字中采用 task- 前缀是 Workspace 的明确绑定选择，不能靠前缀自动推断结构；重绑定须走原演进规则。

## 5. 原生属性到业务值

只有 §4 已绑定的属性才成为 D4 Field，名称相似不认领。D2 保留完整原生属性事件；D4 读取同次 evaluation 在 root 文档头结束处的有效值及完整 provenance。正文中的后续 set/unset 继续影响原生渲染环境，但不偷偷改写 header Field。header 内重复赋值遵循原生重定义，不把重复行当多值数组；unset 表示 absence，空字符串与 absence 分开，错误值形成明确 Field diagnostic，不把错误当缺失/零/默认值。字段存在不自动添加笔记类型；Facet 必填与约束另验。

native Field 的获权来源必须是 root-authored 文档头，不能由 API hard-set、环境或 include 代写业务事实。读到的有效值若受这些来源控制，则标记 typed unavailable，保留原生渲染/Source 能力。原生 attribute substitution 可用，但其依赖、origin、环境和授权必须随同冻结；依赖非 root-author 值、多个来源或生成值时不获得窄字段写资格。未知普通属性继续是普通 AsciiDoc metadata，不被删除或报控制声明错误。

内部业务投影保持原 FieldId、TypedValue、权限与关系/Calendar/Query 语义。单值 header 的 occurrenceKey 为 UUIDv5：namespace 为标准 URL namespace，name 为 D3-CJ/3 编码的 `["weftext.header-entry/1",WorkspaceId,NodeId,FieldId]` 的 UTF-8 字节。它不写回源码，不是新 EntityRef，只在完整 owner/Field/source version/Observation/Registry 资格内使用；同值 ABA、删后重建或索引重建不能复活旧 selector。不得把这种 key 当作授权或连续性证明。

`NativeFieldOrigin/1={kind:"native_header",attributeName,rawAttributeSource,sourceRange}` 是 D4 内部 materializer 的闭合来源描述，sourceRange 使用 D2 同次 root 范围。它不扩宽历史 Entry1 JSON decoder；block 原文仍走原 decoder，header 走本节 codec。D4→D7→D8 的 typed value 相同，但 header 原文绝不能伪装成 JSON rawEntrySource 或让旧 JSON materializer 直接修补。无法证明唯一可写 root span 时使用完整 Source Draft 路径，而不是伪造窄写成功。

## 6. 多值、备注、来源与复杂字段

复杂 object、relation/ref、同值多次断言、qualifier、每值 note/provenance、非默认 union 分支继续由现有 `[weftext-attributes]` Entry1 精确表达，内部 TypedValue 不删除。本轮优先让常见简单值摆脱 JSON；不声称已为全部复杂结构找到更简单且等价的文本表示。

同一 Node + FieldId 恰有一种有效作者载体：native header 或 Entry stream。两者同时声明（即使值相等）是 carrier conflict，不相互覆盖、合并或备用。一个 Field 的全部复杂 occurrences 属于同一 Entry stream；未绑定的原生同名属性不会冒充该 Field。由 schema 判定最终语义，未选中 Field 的原字节保持。

给简单值添加逐值备注/来源或改成复杂值时，UI 先展示显式“转换属性表示”预览：准确 header span→完整 Entry、生成 occurrenceKey、原值/类型/依赖、删除 header 的全部同名赋值/unset 与受影响 native references。若这些属性仍被正文/其它值引用，必须保留等效原生行为或拒绝本次转换并留下 Draft；不能删除赋值使其它正文变化却只报告一个 Field 修改。反向转换要求基数≤1、无额外信息、codec 可逆并完整检查替换影响。不得静默丢 note/provenance、precision、relation 或重复值以获得短源码。

## 7. 编辑、复制与恢复的完整路径

D8 属性面板、快捷录入和 Source 共享同一 Draft、Undo 与 Core prepare/commit。默认只 patch 选中的原生值 span，不格式化整个文档；注释、空白、换行、未知正文、属性在标题前后的位置保持。无效或歧义控制声明保留稿件与修复入口，不自动生成第二个 ID。身份字段在常规属性面板只读；类型编辑仍须原类型约束、node_control/source 写入资格和完整变化预览。

D4 relation/materialization、D7 copy/Field Action、D8 structured edit 与 D9 import/template 构造都必须按 native/header 或 Entry 两分支读取准确原来源，并把控制声明与事实变化放入原计划的 C/Q 物化和唯一 D3/D6 提交。具体 selector、效果、模板 slot 与控制声明物化由 [作者源集成](AUTHORING-INTEGRATION.zh-CN.md) 定义；不得把内部控制构造假称为旧 C/Q 语法的现成成员。新来源形式不豁免 final read barrier、隐藏字段、完整负范围、Field-only 权限、source transform、反向关系或安装恢复。只有能够证明全部变化都在获权字段内时才允许 narrow Field；完整源编辑另走原全源授权。

新源配置称 `weftext_managed/2`，由 `ManagedDocumentFormatProfile/2` 承载：其两个成员仍为 languageBaseline（原固定值）和 managedProfile（精确新值）。`ManagedDocumentFormatBinding/2` 完整继承 Binding1 的成员，把 version 固定为 2、profile 改为 Profile2。D6 document_format 现有 key/component 承载其精确版本，原 authority/Notice/CP/CAS 不新增分支。current qualification 的 binding slot 具名接受 Binding2，已记录 Binding1 的 profile/decoder/bytes 保持；不读时升级。

此修订直接替换未部署的 managed2 YAML 候选定义，保留同一候选 Profile2/Binding2/Environment4，不增加 managed3 或虚构 YAML runtime 迁移。Git 中的前稿和其固定版本审查仍是设计历史，不是已安装 runtime 记录；不得将其局部结论自动转给新定义。真实旧 managed1 及其 recorded 解码责任保持。若真实旧 managed1 资料确须转换，显式格式转换预览必须同时覆盖 控制/属性源改写和 Binding2，source 真实变化照常产生新版本，不能套用旧 profile-only 且 `sourceChanges=[]` 捷径。原 prepared/selector/preview 不升级；saved/planned/unknown 沿原版本、原 bytes/pins/UUID/OperationId 恢复。缺 schema 可读原文，不补空 schema；新设备不能从裸文件重建权限或原执行责任。

## 8. 验收要求

以下是设计要求，未运行产品测试。每项需保留正常与拒绝/恢复路径：

| 编号 | 要求 |
| --- | --- |
| SRC-01 | 控制与业务属性在标题前/后/两边及无标题均合法；空行与 author/revision 仍按原生语义，未编辑字节保持。 |
| SRC-02 | 直接 UUID、逗号类型串、原生注释与 BOM 保留；重复/大小写别名、unset、缺 ID、条件/替换/续行、畸形列表拒绝控制投影但保留修复稿。 |
| SRC-03 | task 短名与完整名等义；两者同时出现拒绝重复；未知类型保留；安装顺序和 include/API 不能夺取分类。 |
| SRC-04 | 新建 ID 一次分配；复制生成新 ID；移动/恢复按原身份；中途 crash/retry 沿原计划，不发生 ID 与 birth 半成功。 |
| SRC-05 | 外部改 ID、两个文件同 ID、只到正文未到 metadata、缺元数据重建索引，均不静默合并/重签/扩权。 |
| SRC-06 | status/due 与 schema 精确转换；空、unset、错误、正文重定义、header 重定义、API hard-set 与 include 分开；日期 precision/calendar 不猜。 |
| SRC-07 | Registry 同 generation 换 digest、名称碰撞、builtin 属性冲突、unknown member、codec/union 不相符及旧 Snapshot1 误读全部拒绝。 |
| SRC-08 | 同一 Field header+Entry 冲突；两个同值 phone/不同来源仍是两条事实；简单→复杂→简单不能丢数据或悄改其它引用。 |
| SRC-09 | Query/View 读取同一 typed 值；窄字段编辑不能暴露全源；撤权、ABA、gap、format/schema 变化不复活旧 selector；普通 Source 保存仍有独立路径。 |
| SRC-10 | template/import/copy/关系反向物化完整覆盖两种载体；source/body/ID/Binding 同原提交；真正旧 saved/planned/unknown bytes 不重编码。 |
| SRC-11 | 外来 YAML 不提供身份或备用值；未知普通属性保留；schema/config 格式不被此笔记规则修改；不伪造已部署 YAML 迁移。 |
| SRC-12 | 任务属性组由 schema 生成，多类型共用字段不复制；分组/前缀不造嵌套对象；控制名被全部普通 Field 构造/写入入口排除。 |
