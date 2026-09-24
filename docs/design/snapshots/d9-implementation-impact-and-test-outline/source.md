---
_weftext:
  id: "45b327bf-5eb0-45f2-a5bd-11662af244bd"
---

# D9 实现影响与测试轮廓

生效状态：D9-r04-evidence05-2026-09-24 已经总控接受，仅以总控验收（外部控制记录未随本输入发布）及committed协调journal（外部控制记录未随本输入发布）共同确定生效。以下送审正文完整保留；其中 candidate/待终审标签属于送审时状态，不覆盖本段。正文中的送审验证计数按历史证据保留；最新纠正证据与限度以总控验收为准。此状态标记不改变正文语义或实施门。


revision: D9-r04-2026-09-24；candidate。仅未来实施清单，本轮不修改产品代码/公开文档/brand，不宣布任何实现验收。现有dirty baseline622路径完整保护。

## 影响与替换顺序

1. Core仍只持D2/D4领域模型、D3/D6权限/事务、D7/D8既有动作；转换器/Office/XML依赖留在可选coordinator/worker。不在Core依赖图加入Office/Java/Python/网络。实现D9 Core mapping adapter时先完整D2/D4 admission，再D7 prepared binding＋原D3 Result9，验证全effects一致。Module边界不得因crate旧名扩大写权。
2. `crates/weftext-import/src/contract.rs`旧source ID/ImportIr/SourceFormat与`proposal.rs`旧YAML envelope不是目标合同；采用本版source occurrence、分型IR、显式mapping和D2 Profile2提案，成套替换decoder/tests，不双读旧v1。`pipeline.rs/probe.rs/docling_lite.rs/docling_process.rs/temp.rs/path.rs/limits.rs`分别对接输入、固定route、OS隔离、累积预算和output validator。保留可用算法需证明语义，不因历史测试绿灯跳过新门。
3. 当前`docling-lite-assets.lock.json`的completeForExecution=false必须保持诚实；先补实际installed-file、依赖/模型/license、worker限制证据再进入capability catalog。现存fixtures/fake runner是研究单元，不是实机安装/沙箱验收。CLI import路径也必须明确proposed与committed两个结果。
4. 导出/模板应在可选独立模块实现，当前workspace既有export相关删除状态不恢复或改写。先纯token/typed snapshot编译，再四种Office格式具体decoder/style/repeat，再实际renderer互通与external publisher。不能把文字模板直接放进Core source parser执行。
5. D6导入Job由原authority控制store持有，与原batch commit原子保存进度/receipts；不能coordinator JSON先记成功再写author。外部Publication独立控制store只记录外部输出事实，禁止承担author replay。D8 dirty Draft/conversion accept action复用原edit资格，不引入后门。
6. Desktop/CLI、Server/WebUI通过D1共同能力catalog和实际权限；Mobile无conversion运行/委托入口。Server大文件上传/下载均有限budget/current auth；WebUI不直连worker或任意path。UI明确原件/转换结果/loss/unknown，不能只给绿色完成。
7. 未来公开规范`23-office-export-template-profile.zh-CN.md`、`24-format-conversion-capability-and-provider-profile.zh-CN.md`、`guides/06-office-export-templates.zh-CN.md`需按当时实际实现替换旧Record/attr/H1–H9/公式重排/宽泛view；不能先把本架构正文搬成已实现承诺。原型清理需涵盖CLI help/API/parser/generated samples，不是只改名。

## 实施验收门（本轮均未关闭）

| 门 | 实际必需证据 |
|---|---|
| I01 IR/codec | 完整strict decoder、duplicate/unknown/Unicode/Counter、索引/来源/覆盖/预算、所有profile正反语料；独立parser与visual对照，不只同函数roundtrip。 |
| I02 hostile files | ZIP alias/traversal/links/bomb/repeated ODF rows、XML XXE/DTD、active content、encrypted/unknown variants、malicious stdout/output slot、truncated inputs、all limits；失败无author变动。 |
| I03 OS sandbox | Windows与Linux具名构建各实机：读home/DB/credentials、写workspace、network/child process、escape、OOM/timeout、kill pending、清理竞争；任何未证明平台不进available。 |
| I04 mapping/admission | D2完整parse与inert table cells、D4所有Registry/cardinality/refs/control、ordinary import fresh owner/root、source_artifact/Result9/preview/receipt byte equality；source/权限/Registry改变时拒绝。 |
| I05 real Job | 10k Nodes、10批、large-input大于内存、SCC/不可拆Template；真实D6故障注入commit前/后/receipt丢失/重启/撤权/取消/TTL/pin，逐字源与receipt/前缀一致。 |
| I06 template compiler | Word/WPS/LibreOffice普通编辑生成的真实DOCX/ODT/XLSX/ODS；run split/有效style、escape/injection、XML非法字、none/复杂值、zero/N双方向、merge/footprint、external links/macros全拒绝。 |
| I07 document output | H1–H5/body/unique bibliography/样板style/默认style冲突；具名Office版本打开保存渲染、字体fallback、中日韩/RTL/accessibility/分页，差异进入loss，不宣称像素一致。 |
| I08 typed export | D7每种result域/空页/complete/reset、bag/order/tie/none、long integer/decimal/date/instant、CSV injection与全角/whitespace、typed sheet无formula注入；主数据与loss匹配。 |
| I09 publication | create-only原子bundle的具体文件系统证明，空间不足/文件名冲突/用户移动/flush失败/rename后crash/权限撤销竞争；unknown无重复发布；Resource另走原事务。 |
| I10 region | CropBox/MediaBox/UserUnit/Rotate、EXIF、invalid/multi-frame、百万分比rounding；d9rg1 canonical与原l1绑定、fresh copy/fork、stale和授权遮蔽，键盘/屏幕阅读器实机。 |
| I11 surfaces | Desktop/CLI/Server/WebUI相同语义，Mobile禁止；D1全部重叠reason顺序；诊断不泄露部署/隐藏数据；D8 Draft conflict与provenance保留。 |
| I12 release/naming | 依赖树/SBOM/具体license与models字体、每平台安装/卸载/清理、旧alias扫描、无后门provider/config、能力catalog与实现测试逐项绑定。 |

通过某个profile只开放对应(format,operation,variant,route,platform,version)组合，不能开放整个SourceFormat枚举。Library upgrade、decoder/style/profile语义变更必须重跑受影响门；授权/transaction/public wire基础变化按总协议重新独立审查。A2整体从零审查和D10仍由后续独立阶段处理，本轮不启动。

## r02 必须联合核对的反例与证据边界

模板：重复source拒绝；self/cross-template唯一fresh subject；Resource owner严格重写；parent import与简单collection create逐字段合法；同输出但不同template/recipe revision使旧准备失效；完整membership/top-take与lost receipt原请求恢复。PreparedActionBinding/2四处镜像/消费和历史/1恢复逐项检查。

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
