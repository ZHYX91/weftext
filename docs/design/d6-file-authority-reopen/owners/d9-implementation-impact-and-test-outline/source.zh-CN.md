---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：45b327bf-5eb0-45f2-a5bd-11662af244bd。

候选状态：D9 文件权威 PL-IR-01 修复 consumer，未独立接受、未激活、未实施。固定 S 的能力/限制/历史证据边界保持；本文消费稳定生产地址/current Observation 作者修复，但该新 exact candidate 仍待独立复核，不自行关闭 P1。

# D9 实现影响与测试轮廓

固定 S 的范围与证据说明：这是未来实施清单；本文只更新设计后像，不修改实现代码、品牌或用户修改，不宣布任何实现验收。下列具体实现路径和研究计数来自固定 S，不表示本轮重新检查当前实现。

## 影响与替换顺序

1. Core仍只持D2/D4领域模型、D3/D6权限/事务、D7/D8既有动作；转换器/Office/XML依赖留在可选coordinator/worker。不在Core依赖图加入Office/Java/Python/网络。实现D9 Core mapping adapter时先完整D2/D4 admission，再D7 prepared binding＋原D3 Result9，验证全effects一致。Module边界不得因crate旧名扩大写权。
2. `crates/weftext-import/src/contract.rs`旧source ID/ImportIr/SourceFormat与`proposal.rs`旧YAML envelope不是目标合同；采用本版source occurrence、分型IR、显式mapping和D2 Profile2提案，成套替换decoder/tests，不双读旧v1。`pipeline.rs/probe.rs/docling_lite.rs/docling_process.rs/temp.rs/path.rs/limits.rs`分别对接输入、固定route、OS隔离、累积预算和output validator。保留可用算法需证明语义，不因历史测试绿灯跳过新门。
3. 当前`docling-lite-assets.lock.json`的completeForExecution=false必须保持诚实；先补实际installed-file、依赖/模型/license、worker限制证据再进入capability catalog。现存fixtures/fake runner是研究单元，不是实机安装/沙箱验收。CLI import路径也必须明确proposed与committed两个结果。
4. 导出/模板应在可选独立模块实现，不得恢复或改写用户当前修改。先纯token/typed snapshot编译，再四种Office格式具体decoder/style/repeat，再实际renderer互通与external publisher。不能把文字模板直接放进Core source parser执行。
5. D6导入Job由原authority控制store持有，与原batch commit原子保存进度/receipts；不能coordinator JSON先记成功再写author。外部Publication独立控制store只记录外部输出事实，禁止承担author replay。D8 dirty Draft/conversion accept action复用原edit资格，不引入后门。
6. Desktop/CLI、Server/WebUI通过D1共同能力catalog和实际权限；Mobile无conversion运行/委托入口。Server大文件上传/下载均有限budget/current auth；WebUI不直连worker或任意path。UI明确原件/转换结果/loss/unknown，不能只给绿色完成。
7. 未来公开规范`23-office-export-template-profile.zh-CN.md`、`24-format-conversion-capability-and-provider-profile.zh-CN.md`、`guides/06-office-export-templates.zh-CN.md`需按当时实际实现替换旧Record/attr/H1–H9/公式重排/宽泛view；不能先把本架构正文搬成已实现承诺。原型清理需涵盖CLI help/API/parser/generated samples，不是只改名。

## 实施验收门（本轮均未关闭）

| 门 | 实际必需证据 |
|---|---|
| I01 IR/codec | 完整严格解码器：重复或未知成员、Unicode、Counter、索引、来源、覆盖与预算，以及所有配置的正反语料；必须有独立解析器和视觉对照，不只用同函数往返验证。 |
| I02 hostile files | ZIP 名称别名、路径穿越、链接、解压炸弹、ODF 重复行，XML XXE/DTD、活动内容、加密或未知变体、恶意标准输出和输出槽、截断输入及全部限额；失败时作者内容不变。 |
| I03 OS sandbox | Windows 与 Linux 各具名构建的实机证据：尝试读取用户目录、数据库和凭据，写入工作区、联网、创建子进程、逃逸、内存耗尽、超时、终止待定及清理竞争；任何未证明平台都不能标为 available。 |
| I04 mapping/admission | 完整 D2 解析及惰性表格单元格，D4 全部 Registry、基数、引用与控制准入，普通导入的 fresh owner/root，以及 source_artifact、Result9、预览和回执的逐字节一致；来源、权限或 Registry 改变时拒绝。 |
| I05 real Job | 10k Nodes、10 批、大于内存的真实输入、SCC 与不可拆 Template；真实 D6 故障注入覆盖提交前后、回执丢失、重启、撤权、取消和 TTL/pin，源字节、回执与前缀逐项一致。 |
| I06 template compiler | 由 Word/WPS/LibreOffice 普通编辑产生的真实 DOCX/ODT/XLSX/ODS；验证文本段拆分和有效样式、转义与注入、XML 非法字符、none/复杂值、零项或多项的双方向重复、合并和写入范围，以及外部链接与宏的完整拒绝。 |
| I07 document output | H1–H5、正文、唯一书目、样板样式和默认样式冲突；具名 Office 版本的打开、保存与渲染，字体回退、中日韩文字、RTL、无障碍及分页；差异计入损失，不宣称像素一致。 |
| I08 typed export | D7 各结果域、空页、完整性与重置，重复行、次序、平局和 none，长整数、小数、日期与瞬间，包含全角和空白的 CSV 注入，以及无公式注入的有类型工作表；主数据须与损失报告匹配。 |
| I09 publication | 具体文件系统对 create-only 原子目录包的证明；空间不足、文件名冲突、用户移动、flush 失败、rename 后崩溃与撤权竞争；unknown 不重复发布，Resource 另走原事务。 |
| I10 region | CropBox/MediaBox/UserUnit/Rotate、EXIF、非法和多帧输入、百万分比舍入；验证 d9rg1 规范编码与原 l1 绑定、fresh copy/fork、过期与授权遮蔽，以及键盘和屏幕阅读器实机使用。 |
| I11 surfaces | Desktop/CLI/Server/WebUI 语义相同，Mobile 禁止转换；D1 所有重叠原因的优先顺序；诊断不泄露部署或隐藏数据；保留 D8 Draft 冲突与来源标记。 |
| I12 release/naming | 依赖树、SBOM、具体许可证、模型与字体，每个平台的安装、卸载和清理，旧别名扫描，提供方与配置无后门，能力目录逐项绑定真实实现测试。 |

通过某个profile只开放对应(format,operation,variant,route,platform,version)组合，不能开放整个SourceFormat枚举。Library upgrade、decoder/style/profile语义变更必须重跑受影响门；授权/transaction/public wire基础变化按总协议重新独立审查。A2 与全局终审仍是独立阶段；本篇不自行接受其它阶段。

## r02 必须联合核对的反例与证据边界

模板：重复source拒绝；self/cross-template唯一fresh subject；Resource owner严格重写；parent import与简单collection create逐字段合法；同输出但不同template/recipe revision使旧准备失效；完整membership/top-take与lost receipt原请求恢复。固定 S 的 PreparedActionBinding/2 四处镜像/消费与真实历史 /1 恢复证据逐项保留；当前 /3 则以 D7 唯一 owner 和本候选消费关系重新检查，不能把旧镜像当当前结构。

隐藏内容：同range的exclude/include、隐藏中间行、隐藏列绑定不移位、被隐藏header拒绝、隐藏sheet、空axis、merge相交、manifest/groups重算，全部按typed字段和显式mapping，不能用message或choice猜。

导出与模板：NodeRef/复杂object/quantity/scalar none/graph保留原D7型；RenderSnapshot拒绝未投影复杂值；auth generation reset；普通data token只在同SET item scope，0/1/N和跨SET都不隐式first。

总控补充：IR与预期D2 AST一致、D4真实calendar_date/zoned_instant、固定style资产版本、d9rg1与原D3授权/stale顺序。新增有限模型证据单独记录；原57项只是原有研究子集，任何新用例不能被计作实际D3/D6产品、OS隔离、Office互通或完整格式conformance通过。

## r03 导出组合与单位检查

逐生产者/消费者构造完整ExportInputCatalog、ExportProjection、ExportLossReport、确认和staged bundle；覆盖Optional<Text>→CSV、Document/Field/Annotation省略、Office缺值、重复同值行、错误pin/列/来源域、权限换代、确认后bytes/报告/模板变化、空结果/graph/scalar原V。图片覆盖密度有/无/非法/冲突、方向交换、exact有理换算、半偶量化、显式尺寸有无及物理事实与布局policy分离。有限模型源码/实际输出独立留存；真实decoder、当前权限、Office renderer与OS发布仍是未完成实施门。

新增validate_r03_export.py实际为82/82，独立保留validation-r03/export-results.json、export-witnesses.json和consumer-matrix.json。实际最小DOCX只验证ZIP/XML中的位置，未运行Office；两份PNG验证尺寸/密度元数据，JPEG仅为单位算术模型。native-table与bibliography案例只验证来源位置/块索引，不冒称完整D2 table parser或citation renderer。与原63/63、59/59分别报告，不合并为产品scenario通过数。

## r04 消费适用条件与完整有限链

正文/书目由ExportContentSelection/1明确选择，不能由Document pin存在或body slot缺失推导。纯Query、窄Field/集合、native_table不添加正文读取或虚构省略；请求正文却无权则失败。body、bibliography、样式回退、已知placement省略和报告位置采用同一选择条件。新验证必须从实际fixture和明确选择生成全部必需报告及准确data/report/manifest bytes，检验确认后不变及三个出口的当前资格；不可只分别证明几个地址存在。图片模型先校验绝对/相对声明一致，再进入有/无显式尺寸分支。

r03的82个断言和历史源码继续原样保留，但它们不验证完整Office消费义务、真实值派生或完整初始Plan与报告bytes的一致性。r04的新有限组合另行执行和记账；仍不声称完整D2/Office decoder、citation renderer、真实授权/CAS、OS隔离或publication故障门通过。

当前r04有限证据：validation-r04/export-results.json为90/90，validation-r04-consumers/results.json为130/130，后者包含12个连贯场景和独立结构反例；均与63/63、59/59及历史82/82分别记账。实际最小DOCX只验证协议样本的ZIP/XML和输出字节，未完成Office互通；非空书目/style只作结构模型。

## 当前文件权威协调的实现顺序与新增 oracle

上述 I01–I12 全部保持开放。历史 57、63/63、59/59、82/82、90/90、130/130 分属原各有限模型，不能相加或计作本轮通过；本轮没有运行原模型、实际 worker、Office 或 OS 故障测试。实现读取路径和安装版本同样是原来源记录，启用时必须重新验证。

1. 先实现真实版本 decoder、当前入口 scope/授权/CommitDomain/P 连续性与 SourceObservation 生产，再读取完整 source、Registry、Entry 或业务对象。D9/2 workspace 入口与纯工件 D9/1 分开；strong 路径始终 strict，不能借 D8 人工 ordinary/observed_only 或 semantic_pending 放行。缺某个强 producer 只门控其依赖路径，不把普通外部工件处理永久关闭。
2. TemplateRecipe/2 保存精确 managed 生产版本；TemplateConstruct/2 消费实际 SourceVersionRef；TemplateConstructionInput/2 保存真实观察与完整 PinRef。验证 A:1/B:1 不混同、生产域与观察域可以不同、epoch gap/外部替换不能靠 equal bytes 复活、固定来源变版拒绝。真正读取的源与仅省略目录项按各自先验用途授权，后者不扩成 Annotation body 读取。
3. 显式旧配方升级完整保留参数/树/slots/资源/损失，先展示准确来源映射，再经原 create_resource 独立准备和确认另存。未知旧生产版本不能以同 Counter 自动迁移；普通未提交旧模板仍可明确选择当前来源。PL-IR-01 现使用稳定地址/current Observation 作者算法：按 D6 PL13 验证 `body_text`，按 PL14 验证 Resource region，包括纯同步正向读取、变版 dependency_conflict/stale、不取 latest 重签，以及不改写既有 analysis/PAB/ExportPlan。本要求仍待独立复核，不再是未定义算法，也不自行关闭 P1。
4. D7 /3 是唯一准备 owner；移除 current schema 镜像重复，保留真实 /1、/2 最小映射、pins、原 request 和恢复。D3 原 wire12 无 planToken；普通 source plan 使用真实 H/空历史及唯一 candidate map，fresh/version 不在 preview 提前分配。当前 D9 不消费 conflict-only 安装 wrapper 或 source plan /2。
5. Job 每批绑定完整 DecisionKey/2。验证同 Workspace/OperationId 不同域不互认、未知先恢复原请求、先恢复准确 committedPrefix 才准备下一批、任何域变更不自动接管。原不可拆 group/SCC/1000 descendants/10k 实测义务保留；预算/charge 不因 worker 重跑、换 token 或 P 丢失清零。
6. ExportInputCatalog/2 与 Plan 的实际当前证明和完整 pins 一起生成；source/Field/annotation 目录及 Query proof 按实际用途、同 cut 和正负范围验证。无关连续 sealed Frontier 扩展不代替 source currentness，也不凭一个整体 Frontier 变化丢弃真正合法的 scoped 读取。导出选择三分、bag/order、真实 projection origins、初始 loss/确认和 exact staged bytes 保持同一记录。
7. inspect/confirm/publish/chunk 当前授权持续有效；Query epoch 变化 reset。原 publishing/unknown 先按原 destination、占有/rename 和 P 责任恢复，不重渲染同摘要或换名重发。真正 Resource author decision、CP3 与外部 PublicationReceipt/2 分开；一侧失败不补造另一侧 receipt。

新增正反 oracle 与原 S01–S34 一同列于当前接受矩阵。文档配对/哈希/差异检查只验证交付，不替代上述实现、独立语义接受或任何第三方许可证/格式现状调查。
