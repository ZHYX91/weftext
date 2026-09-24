---
_weftext:
  id: "3c031fa6-0f70-43ee-9388-cc007d4d2a98"
---

当前权威状态：D8 D8-r03-p2-2026-09-24，仅由总控验收（外部控制记录未随本输入发布）和committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。下面送审正文中的candidate/未激活为历史标签，不覆盖本段。架构接受不代表产品实现；D9未启动。

# D8 编辑器与跨表面交互契约

revision: D8-r03-p2-2026-09-24；状态：candidate，尚未独立接受或激活。本候选由本主稿、Editor Interfaces、Direction and Accessibility、Acceptance Matrix、Terminology、Implementation Impact及同包D7效果运输范围修订组成。架构验收不代表产品实现、平台支持或性能通过。

## 1. 选择、问题与非目标

选择一个共享编辑控制器：它管理同一作者目标的草稿、输入事务、选区和请求状态，各UI通过它生成相同Core意图。Source是exact source编辑，Write是Core可证明映射的内容编辑，Read是不可编辑的投影；它们不是三份可独立保存的内容。属性、原生表格、节点集合各有原领域目标，显示为表格不改变权威。

冻结输入为D1五表面与能力边界、D2 Profile2/wire2、D3 wire11/Result9、D4当前类型/Entry/Registry及D6/D7消费修订、D5无持久Record、D6当前Policy/2与耐久提交、D7 typed DAG/CEL1/纯View/显式Action。保持D3唯一身份、D6唯一author commit与D7完整effects。D8定义编辑适配器和消费状态，不增加document/field/row身份、协作数据库或语法解析器。

本代不交付实时共编/CRDT协议、任意富文本/HTML编辑、任意公式执行、移动端转换/Agent、翻译后的阿拉伯UI、跨设备自动同步草稿或可移植block方向语法。D9模板/转换尚未冻结时相关入口明确不可用，不能以UI拼装猜测模板。阿拉伯/希伯来内容、bidi呈现和RTL shell仍是D8必须验收的独立交互能力。

## 2. 状态所有者与共享会话

| 状态 | 所有者、绑定与期限 |
|---|---|
| 已提交作者源/Ref/版本 | Core authority，D2–D6原合同；UI不能写文件或数据库 |
| EditSession | 当前客户端、principal/session、authority连接、WorkspaceRef、完整目标Ref与读取范围；不持内容身份，不跨登录复用 |
| Base | Core已授权读取的exact source/完整Field投影/Annotation值及原revision；本会话不修改 |
| Draft | 用户提案，精确文本或typed值输入、单调draftSerial、base绑定；无author revision、无D3 locator、无Query结果权威 |
| DraftProjection | 同Core针对准确Draft及serial产生的可丢弃映射；迟到响应不得覆盖更新Draft |
| Selection/viewport | 用户当前会话可丢弃状态；含方向/焦点与对应draftSerial或author revision |
| Prepared/Submitted | Core保存的不可变计划/preview与原完整提交请求；UI保存原request供恢复，不能改tokens或效果 |

同一客户端内同principal、authority、owner的多个pane共用一个写入控制器；只读pane可绑定同Draft或明确的committed Base。属性/表格编辑与Source/Write不能同时维护互不知晓的可提交分支：有dirty Draft时其它同owner编辑入口先聚焦原编辑、明确放弃、或留在独立待合并proposal；后者永不自动覆盖前者。跨进程/设备没有隐式会话合并，由Core revision/CAS裁决。

读scope不扩张：phone-only session不能打开Source/Write全源，也不能通过另一个pane借用其他principal的cache。读取全源的主体可切换其同owner表面；没有全源读取资格只使用D7完整Field选择与窄编辑资格。部分属性投影必须标明未获权/不可用，不能显示成空值。

Draft串行号为本地非负计数；每次已完成输入事务增加一次，耗尽则关闭该session并保留proposal重新打开，不回绕。它不是D6 Counter revision。一个session只有一个正在写的输入事务；对话框持有固定target与base，不随背景焦点/排序/选择变化。

## 3. 共享编辑状态机

状态拆成四个正交轴，避免把只读、失联和脏内容混为一谈：

- buffer=`clean|dirty`；仅Draft逐字等于Base时clean，不因renderer一样而相等。
- input=`idle|composing`；composition是一项尚未结束的本地输入事务。
- submission=`none|preparing|preview|submitted_unknown|planned|committed|rejected|terminal_failed`。
- access=`current|offline_remote|needs_revalidation|denied|authority_unavailable`。它是本地观察，只有Core可以最终授权或判定决议。

| 事件/条件 | 必须发生的变化 |
|---|---|
| 输入文本、值或结构命令 | idle且不在已提交分支中修改当前Draft；新serial；原projection/preview失效；只改变本地提案 |
| composition开始/更新 | 锁定原draftSerial与原替换范围，保存precomposition buffer/selection；更新暂存preedit，不prepare、不commit、不触发快捷动作 |
| composition结束 | 一次完整的最终文本替换，至多一个Draft事务；若文本未变则零次；以最终native输入状态确认，不能把compositionend与随后input重复应用 |
| composition取消 | 恢复precomposition buffer/selection，零author变化；焦点丢失不能被猜成取消或确认 |
| 请求预览 | 必须idle/current、目标绑定完整且没有未决提交；固定serial/intent，Core prepare；期间可继续输入，但迟到prepared只保留历史不得确认为新Draft |
| prepare成功 | 只有serial/target/access仍匹配才能进入preview；获取完整manifest，显示真实domain/作用范围和损失/不可用信息 |
| 确认提交 | 独立明确动作，检查当前serial与preview、完整效果可获取、没有composition，保存原request后发送；立即submitted_unknown，不称已保存 |
| receipt | 核对原request/OperationId/protocolOwner；显示Core实际committed。在当前有权确认原请求的实际committed后像并按后继Draft规则更新Base后，buffer始终重新按当前Draft与新Base的exact equality派生；serial/原request/generation只约束回执归属及派生更新，不是clean的附加必要条件。回执绝不覆盖当前Draft、selection或倒退serial；新baseRevision使旧map失效 |
| planned/断网/超时/响应无法判定 | 保留原request与proposal，按原协议恢复/重放；禁止新OperationId重做同次未知提交 |
| 准备/提交被拒绝 | 原Draft保留；展示实际已授权错误。只有确认原decision不会提交后才能重新prepare |
| 取消准备/关闭preview | 可以取消UI等待/丢弃未提交preview，不能等同取消已planned或回滚committed |
| 权限/authority/Registry/revision变化 | 废弃当前确认资格和相关派生结果；input可留为本地提案，Core重验证后新preview。已提交分支必须先查原决议 |
| 切换模式、方向、缩放、虚拟化窗口 | 不修改源、不自动结束composition、不提交；必要布局更新延迟至composition结束 |

submitted_unknown/planned分支内允许继续输入到明确标记的后继Draft，但不准提交后继意图直到前次有可证明结果。后继Draft基于原submitted source与本地输入日志；前次committed且实际after字节相等时，可把其作为新Base继续；拒绝/不同实际after则进入三方proposal，不静默改基线。D3 symbolic新建的结果必须根据真实receipt及后像重新打开，不猜新Ref。

撤权后的已交付字节不可远程追回。客户端应停止进一步披露、清除可清理的未交付结果/预览、锁定会话并要求重新认证；不能以“缓存还在”继续请求或宣称权限有效。用户自己输入的proposal按设备草稿政策保留；它不授读取原源/历史权。不得把本地清缓存宣称可靠安全擦除。

## 4. Source、Write、Read转换与草稿解析

Source可以保留暂时invalid文本，全部exact UTF-8 scalar、BOM、CRLF/CR/LF、trivia、顺序和未编辑片段必须保持。仅用户明确替换的字节可以改变；不在切换、保存、输入法、复制或方向变化时normalize NFC、换行、数字、引号或空白。malformed UTF-8属于D6物理repair envelope，无D2 payload；本编辑器不能用replacement字符悄悄解码后保存。

Write只渲染Core对当前Draft完整成功的D2投影与映射；invalid时整个Draft projection unavailable，显示Source修复和真实diagnostic，不混用旧body与新header。允许单独显示明确标注revision的“上次已提交版本”，不能当它是当前Draft。D4 typed unavailable与D2 invalid分开；保留raw的未知namespace不能因输入body而丢失。

D2 Inline只有text/link/resource_occurrence/node_link/citation，没有bold/italic/math任意格式。Write提供的文字、链接、标题、列表/原生表格命令必须落到D2合法源；未在本代冻结的格式按钮不可假装可用。公式和代码作为inert source/literal或D7显式CEL展示，打开/悬停不执行。protected payload整体可在Source编辑，但其正文不得被富编辑器二次解释。

Write映射由Core给出，渲染节点/DOM offset不是作者位置。映射不一一对应时，例如多行paragraph join、escaped cell、citation label、资源占位，显示为atomic或readonly occurrence并提供明确Source定位；不能把渲染字符串直接innerText回存。D8 Interfaces同时冻结Draft Edit Map、独立导航origins、inert片段替换和普通文本区域命令：Core提供包含全部原space/tab/EOL的plainRegion，首字、空白、换段、合段、删空与连续删除均形成精确提案。普通区域的Write宿主显示实际源换行及段间空行；单EOL在Read中按D2连接为空格。这个差异明确呈现，不隐藏换行再让UI猜回源，也不把metadata/结构语法变成普通文本。原子/escape/join只限制不能证明的区间，不把整份Write降成已有文字校对器。更多结构命令复用已有closed D7 Actions；未闭合的按钮不可用。

Source→Write先结束或取消composition（由用户/native输入确认），请求当前serial投影，成功后转模式。Write→Source保留当前Draft并映射到logical source点。Read可阅读明确的Draft预览或已提交revision，均有标签；默认打开是已提交版本。Read不提供内容直接修改；批注等独立动作仍需明确选择目标与Core计划。

本地Desktop/Mobile使用同一包内Core；远端Desktop/WebUI/Mobile仅请求Server投影/变换/prepare，客户端不另写托管parser。远端离线允许纯文本Draft输入、原已交付只读快照和草稿undo；当前Draft rich projection/Query/验证/提交不可用。重连重新授权并绑定当前revision，不能拿离线渲染当完整语义。

## 5. 光标、选区与命令路由

持久地址仍为D3。编辑选择使用`owner + author revision或draftSerial + anchor + focus + affinity`的非持久状态。anchor/focus为logical source点、plainRegion的scalar offset或准确映射的text-run点；选择方向必须保留，range执行时才取min/max。affinity只决定bidi边界/换行上同logical点的视觉光标，不改变被编辑文本。

公共源坐标严格按D2 0-based logical line、Unicode scalar column、end-exclusive。内部UTF-8 byte、DOM UTF-16和平台文本位置必须经同一准确字符串转换；不得把JS length直接当scalar。CRLF不可拆，surrogate pair不可拆；普通移动/删除以Unicode extended grapheme cluster为单位。Source另提供明确的“按码点检查/编辑”工具，使控制字符和组合字符可审查；它不改变默认按字素交互。这个显式工具仍不得产生未配对surrogate或切开CRLF。

同一布局epoch的视觉左右键按Direction规范移动；逻辑前后命令在全部设备可通过键盘/可访问命令调用。Backspace/Delete分别删除logical preceding/following grapheme，非“屏幕左/右字符”。已有selection时删除其logical range；跨atomic occurrence的选择须显式包含完整source target，否则readonly/转Source。矩形选择仅为grid target集合，不能变成正文连续range。

路由优先级：native composition/输入法候选 > 已打开且聚焦的dialog/menu > 当前编辑区域的局部命令 > shell全局命令。Escape首先交native composition，随后关闭局部popup并还焦点，最后才执行未提交Draft放弃提示。Ctrl/Meta+Enter、Enter、Tab、slash、快捷Delete在composing时不能抢占IME。未聚焦编辑区的Delete无内容效果；grid Delete打开带domain的动作选择，不能默认Trash Node。

slash/command palette/context menu/inline toolbar都调用同一命令目录。目录固定能力、domain、target binding、快捷方式、可访问名称、readonly原因。打开菜单不prepare/执行；选项确认保留打开时target，若target stale则拒绝并要求重新选择。pointer与键盘访问同一意图；focus不能在异步刷新时跳到新同名行。

## 6. IME、组合输入与平台事件

宿主适配器把实际事件序列归一为begin/update/commit/cancel，controller不依赖单一浏览器事件顺序。begin保存原buffer/selection与input generation。compositionupdate/preedit允许临时文本；它不进入Core prepare、Query、历史author revision或自动持久提交。最终input可能发生在compositionend之后；适配器通过原事件事务与最终buffer核对只产生一次commit，不同时追加event.data与DOM已有文本。

composing期间不重建/卸载输入宿主，不替换value、不回写selection、不重排其虚拟行，不切方向/字体/换行宽度。异步author变更排队，结束后保留本地提案进入stale处理；权限失效立即停提交并锁定后续披露，当前native暂存可安全结束为本地proposal。后台/导航前优先请求native完成或取消；不能取得确切结果则保存precomposition Draft和明确未完成preedit供恢复，不把preedit假作最终文本。

一次输入法确认形成一个Draft undo group；拉丁死键、组合音标、emoji ZWJ、语音/手写replacement同样遵守原范围与最终文本。Spellcheck/autocorrect作为明确native replacement事务，不能越过target/serial。undo/redo在composition结束前交native自身，不操作全局Draft栈。

## 7. 复制、剪切、粘贴与拖放

提供两种明确文本复制：Source复制所选exact source；Read/Write的“复制文本”复制logical readable text，标明去除源标记，按logical顺序而非视觉glyph顺序。不复制UI为bidi隔离加的字符、hidden aria标签或不可见字段。格式丢失是明确显示转换；不声称文本复制是canonical export。复制D3引用使用单独命令，给完整typed地址的安全文本，不拿title/path当Ref。

本代粘贴默认只读取OS提供的text/plain。HTML、RTF、自定义MIME不能自动作为author syntax；普通富文本只取平台明确提供的plain text，无plain flavor则提示不可用，保留原剪贴板。Source粘贴是显式源文本提案，可暂时invalid；Write粘贴走Core inert片段或plainRegion splice适配，无法表示时完整拒绝并提供Source/Document/Resource的明确选择，不静默截断/执行宏。原生table多行/TSV批量粘贴入口本代不可用，不生成Source、不自动拆为多次单行提交。用户可自行打开通用Source编辑器输入明确源提案，但该行为不标为已实现表格批量粘贴。

剪切必须先成功写剪贴板，再在同一Draft输入事务删除固定selection；clipboard拒绝时不删。Read不能cut。跨Node对象复制/拖放走D3 fresh/owner重写与D7完整预览；文本剪贴板中的Resource token不会自动跨owner变有效，不自动下载URL或导入图片。对未完成/未知的跨Workspace copy，绝不先删源；source Trash是独立明确提交。

本地拖动正文是固定Draft内的明确range移动提案，重叠范围按无变化或拒绝，不自行猜；grid/board拖动是实际字段/结构动作。拖到视觉左/右列由columnId映射目标Field值，RTL不翻转data meaning。拖拽均有等效菜单/键盘选目标操作。

## 8. 撤销、重做与历史

Draft undo/redo是会话输入日志，保存每次before/after bytes或typed输入、selection、输入时serial与group边界；恢复内容时分配新的单调serial并重新投影，不倒退计数或复用旧map；不写Core。composition/paste为各一个group，连续文字输入可在同target且没有选区/模式/命令切换时合并；不得跨prepare/提交边界或跨owner合组。新的输入清除相应Draft redo，不影响已提交历史。撤销后文本等于Base才clean。

已提交Undo是对当前cut的新Core计划、新preview、新OperationId；不回滚数据库，不复用旧receipt，不恢复旧revision。本版直接Undo由Interfaces§7限制为D8单source编辑，且完整当前版本/值必须仍等于原after；同owner任何后来编辑均拒绝直接Undo，保留后来不相关内容。可以另开明确的三方Source提案，但不自动覆盖或声称已经实现选择性跨revision Undo。历史读取、当前权限与依赖仍完整验证。清楚区分“撤销未提交输入”和“撤销已提交操作”；按钮状态和读屏名称必须一致。

Redo是对已提交Undo的新逆向意图，不重新发送原已committed请求。删除/Trash的逆向只在D3明确restore可用时提供；purge、外部副作用、不可证明历史与跨Workspace两次操作没有假原子Undo。结果未知时先恢复原decision，再提供Undo。历史内容受当前read权限控制，不能从本地cache绕过。

## 9. 过期、冲突与重定位

新revision使旧Document/Field/native selector失效；相同key、同title、同text、同digest或source A→B→A均不能延续其授权/地址。controller可以用本会话确切输入日志维护Draft内部光标，但不能把这种位置转换宣称D3跨revision locator continuity。

rebase必须保存base/current/proposed及真实差异，Core对新cut重算footprint、完整D2/D4 gate与依赖，产生新preview并要求明确确认；非重叠也不是后台自动commit。无法证明目标时保留冲突提案，禁止fuzzy最近匹配写入。readonly冲突视图只展示当前有权读取的部分；失去full-source权限时不展示原全源history作为“比较”。

引用点击遵守D3 entity/locator独立授权、生命周期与stale顺序；历史作者Ref原值可显示不等于目标可解析。Annotation exactness按下表的五种target分支判定，不能把任意source修改一律称stale；可提出同owner当前revision的新选区/element/Resource目标，明确展示旧target、候选新target和失去的上下文，用户选择后Core验证新的Annotation值及current revision。多个相同文本候选无默认选中；旧token保持原样。

D2 `targetStatus=resolved|stale`只表达目标精确性；D3 `suspended`是生命周期状态，不是D2第三个targetStatus。已精确匹配但target被Trash时仍可有resolved exactness，同时导航/动作suspended；restore在原revision及Value未变时解除suspension，不创建新locator。表中“保持”均以原合法same-owner target、当前资格和未发生其自身失效为前提：

| D3 Value/3 target | 无关Document source修改 | Resource bytes替换 | Trash与restore |
|---|---|---|---|
| document（owner） | whole-document仍resolved | 不改变该target exactness | owner Trash使D3 suspended；同Ref restore解除；不因生命周期独自改stale |
| document_element（locator） | Document revision不再精确即stale，即使同text/ABA | 无关Resource不改变Document locator | Trash/restore不修复已stale的locator；未变revision则仍exact |
| document_range（locator） | 同上；不按字符未移动推延续 | 同上 | 同上 |
| resource（resourceRef） | 无关Document编辑不改whole-resource exactness | whole-resource仍指同ResourceRef、resolved | Resource或owner Trash时suspended，同Ref restore可解除 |
| resource_region（locator） | 无关Document编辑不改region exactness | Resource revision变更使旧region stale | restore不复活旧bytes revision；只有原绑定仍精确才resolved |

Resource region profile仍须已支持；purge/不存在/权限拒绝按D3原顺序处理，不能把不可见伪报stale。D2 projection和D3 Value/3 target外形各用原协议，不混用各自字段。

field/carrier出现项没有AnnotationTarget。其note用D4 Entry note；不允许把raw source range包装成document range绕过此禁令。尚未提交Draft的选区只能保存局部待批注proposal，不能发D3 locator或新Annotation；先明确提交文档，再在真实revision重新选择并创建批注。D7 apply_suggestion继续其精确target/revision、Document+Annotation原子计划，replacement不可因显示方向被重排。

Resource region profiles未冻结的格式显示明确unsupported，允许whole-resource已有target；不靠DOM像素坐标发明regionToken。D9后续profile必须绑定exact Resource revision、几何/页/方向和可访问替代，不能以当前D8契约声称任意PDF/image区域重定位已支持。

## 10. 属性、表格、集合与动作映射

| 用户意图 | 唯一目标/入口 | 必须保留 |
|---|---|---|
| Source完整文本、Query/View定义编辑 | D8 document prepare → D6 | 当前完整source版本、实际footprint、D2/D4与适用D7定义门，完整full preview |
| 普通文字Write | D8片段或最小普通正文命令，随后D8 document prepare | 完整Draft Edit Map与导航origins、Core区域/树变化证明、输入队列、一个undo group |
| 属性append/replace/remove/member | D7 Field Actions/selection | 完整FieldId、raw Entry、revision、Registry、duplicates及notes/provenance |
| 原生cell/row/checklist及既有行重排 | D7各既有具体Native Action，按原EntityTarget/NativeSelector/tableLocator等形状逐项调用 | 原locator/range、trivia、D2完整源、无行身份 |
| 尚无closed adapter的原生column/多row批量生成/ragged自动修补 | 当前不可用；关闭这些UI自动生成Source路线；既有D7行重排等不属此行 | 未来closed Core适配器需D5每行精确效果/1000row上限；通用Source不证明生成意图或冒称该命令 |
| 集合新建/移出 | D7 collection_create/remove | 明确parent/title/source、requireMembership、完整post-query；top/take参与 |
| Board列拖动 | 一个明确D7 Field或collection意图 | 无唯一可写字段时不可用；column caption不当FieldId，inverse/aggregate不写 |
| 批量字段、表单 | D7 bulk_field的已closed同型意图 | 1000显式targets，实际Field/null/empty/类型语义；不转成D8 Source绕过限额，无任意混合batch |
| Trash/restore/copy/promotion | D7 d3_operation或具体promotion → D3 | 完整闭包、实际owner/Ref、原wire11与Result9、distinct intent |
| 批注新建/生命周期 | 原D3 create_annotation/lifecycle | plain body、同owner目标、reply acyclic |
| 批注修改/明确重定位 | D8 annotation prepare → D6 | 同AnnotationRef、exact旧revision、合法完整新Value/3；不创建身份 |
| 接受建议 | 原D7 apply_suggestion | exact当前range、原replacement、同D6原子effects |
| 模板实例化 | D9结果已合法提供时交D7 create/collection_create | 当前尚无D9结果能力时不可用；不在D8发明模板解析 |

既有Native能力明确保留：`set_native_cell`、`insert_native_row`、`remove_native_row`、满足原trivia保持证明的`reorder_native_rows`、`toggle_checklist`及`promote_native_row`/`promote_checklist`。每项使用原D7完整参数、权限、D2/D4复验和D3/D6协议归属，不能统一套单个NativeSelector。insert_native_row使用原owner/expectedRevision/tableLocator/at/cells；reorder_native_rows使用原owner/expectedRevision/tableLocator/rows完整无重复排列，trivia不能证明保持时按原合同拒绝。两行或更多行并不使这个既有动作自动成为“未闭合多row生成”。禁止的是尚无closed adapter的UI批量生成/TSV/列/自动ragged修补及其Source替代，不禁止用户独立明确调用原有Action；不得自动拆分来冒充这些未交付批量能力。

多值Field空格/空文本/缺项/none/invalid分别呈现；清空输入不默认remove或replace-all。unknown namespace保留source并标明不可用。日期、数字、单位的显示locale不改typed输入：可提供明确可预览的输入转换，但提交值必须D4 canonical，阿拉伯数字不能未经显式规则冒充canonical ASCII数值。D8本代只承诺canonical输入，localized值输入转换未冻结则不可用。

Query/Read cell默认只读；用户明确Action及target column后才申请evidence。sort/page/refresh不触发作者写入；result reset废弃selection/evidence，不按屏幕index选新行。跨页“全选”显式标明当前完整result epoch；prepare固定去重targets，不能在commit重算扩大范围。隐藏列/筛选条件不删数据。

## 11. 离线、草稿、后台与恢复

本地合格Core可以离线正常提交。远端失联只保存非权威Draft，状态文案固定区分“草稿已保存在此设备”“待提交”“结果待确认”“已提交”。source是否已同步不能从输入空闲、预览成功、网络HTTP成功或磁盘Draft写成成功推断。

Draft存储位于设备私有区，按principal/authority/Workspace/完整target和base版本隔离，不进入portable作者source/DynamicBlock。默认不跨设备同步；容量/TTL由显式设备政策展示；dirty未提交proposal不因普通cache清理静默丢弃。发生quota/磁盘失败立即提示未保存草稿，并允许用户复制/显式导出自己有权的数据。关闭含dirty或未知提交的窗口提供留存/返回/明确放弃提案；放弃提案不取消已提交请求。

应用崩溃恢复先加载原request和状态再重连认证，查询/重放同request；未知OperationId不可自动造新request。后台挂起不得让deadline/TTL变无限；恢复以Core当前状态为准。planned的Core pins及ledger不受设备Draft清理影响。无local draft persistence能力时明确显示会话内草稿，不能承诺重启恢复。

## 12. 平台、可访问性与大规模

Direction and Accessibility文件定义视觉/逻辑命令、方向权威、镜像边界、屏幕阅读顺序与平台矩阵，属于本主稿规范。Desktop、Server WebUI与Mobile具有相同Core结果/错误/权限/commit，输入设备差异只影响affordance。CLI/Server headless消费相同业务请求，无虚构光标或IME要求。

性能目标及规模分级在Implementation Impact定义；它们是后续实测验收门，不是当前测量。大文档必须保持exact source与revision，解析/全scope/Query/effects在有预算的Core异步执行。虚拟化只影响viewport，selection/IME/焦点不能依DOM复用位置；查询完整性和D5页/动作限额不变。若不能安全渲染一份已valid完整投影，呈现明确大文档Source/Read能力限制，不交付假的partial canonical projection。

## 13. 替代方案与取舍

| 完整方案 | 裁决 |
|---|---|
| DOM/富文本AST成为可编辑作者源，再导出AsciiDoc | 第二权威、不可表示语法及原字节漂移；拒绝 |
| 每种表面独立save/undo/selection，并共享API名称 | 无法确保目标/IME/未知提交状态一致；拒绝 |
| 每次keypress直接author commit/自动提交 | 临时invalid、IME、预览与scope无法闭合；本代不选择，自动保存仅Draft |
| 一律Source textarea，取消Write/属性/集合 | 语义简单但不能满足结构编辑、重复事实与跨表面需求；保留Source为完整兜底，不作唯一体验 |
| CRDT operation log成为长期权威 | 越过D6事务/权限/关系边界；本代拒绝，后续协作只形成Core检查点 |
| 方向另存portable sidecar/隐藏属性，复制时自动跟随 | 未经D2/D4冻结的作者事实；拒绝。本代可丢弃个人presentation偏好和源内Unicode足够，代价是没有portable block方向设置 |

当前候选的限制是明确的：Write安全子集、source编辑全源读取成本、revision-bound批注可能因其绑定revision变更stale、显式提交、无阿拉伯UI翻译、无资源区域profile、无实时共同编辑和性能实测证明。独立评审需判断这些取舍能否满足本阶段硬要求，不能把它们藏在实现细节中。
