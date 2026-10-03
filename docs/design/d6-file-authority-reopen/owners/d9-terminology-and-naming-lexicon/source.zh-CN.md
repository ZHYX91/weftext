---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：fd325b06-f122-4a6e-bbb6-e484c556ae2b。

候选状态：D9 文件权威协调后像，未独立接受、未激活、未实施。以固定 S 7e18168dad3e6d120fce0dd607dc10fa7894e252 的完整本篇为来源，保留其能力、限制、场景与实际历史证据边界；历史阶段标签不决定本轮状态。当前跨域来源、D3/D6 单决议及 D7 准备/预览消费按同包真实 owner 协调。PL-IR-01 的便携 Locator 资格与尚未接受的跨 owner producer 仍为具名整合门，不因本篇已落盘而关闭。

# D9 术语与命名门

固定 S 的范围与证据说明：D3/D4/D5/D6/D7/D8现有术语权威保持；D9新增术语只在转换域，不把历史对象借名复活。

| canonical term | 唯一含义/owner | 明确禁止 |
|---|---|---|
| SourceArtifact | host取得并pin的外部输入bytes；与D3 source_artifact输入证据相衔接，D9 descriptor不是新身份类 | path/hash作为Node ID，认为相同摘要就是同一输入occurrence |
| Import IR | Weftext转换中间表示；profile定义观察与loss，没有author authority | 第三方DoclingDocument直接成为领域AST；旧ImportIr版本自动兼容 |
| SourceLocation / Observation | 输入版本内坐标与提取证据 | D3 Locator、可写target、跨版本anchor、D4 Provenance授权票据 |
| Provider / Route | 安装并审查的执行适配器 / 固定有限流水线 | 自由命令、template脚本、worker自报授权、通用fallback |
| ImportMapping / MappingProposal | Core明确转换选择/准备前完整提案 | Query operator、D7 ActionSpec新kind、已有源generic patch |
| ImportJob | 复用D6有限多batch控制记录 | 第二author ledger、全job原子或自动rollback承诺 |
| coupling group / batch | 不可拆作者组 / 一个原D3/D6原子请求 | UI page、worker进程、Query page；按1000条默拆Template |
| ConversionInput | 绑定raw inputs/IR/mapping/loss/route的产品证据artifact part | council包装哈希、持久外部identity/OriginBinding |
| Node Template / TemplateRecipe | D2 Template及显式Resource配方，Core一次fresh构造 | Office宏、持续实例绑定、自由parameter interpolation |
| Office template / placeholder / style directive / repeat band | 普通Office文本模板 / 值插入 / 可见style样板 / 单完整row或column | content control/named range/Excel Table增强层；attr/record alias |
| RenderSnapshot / D7ResultPin / ExportPlan | 有限渲染投影 / 原D7完整schema及V结果pin / 当前输出准备记录 | author snapshot import、D6 PreparedIntent替身、保存rowHandle身份 |
| StagedOutput / PublicationReceipt | 未发布完整输出 / 仅外部发布事实 | D3/D6 commit receipt、Resource创建证据 |
| LossReport / issue | 文件导入和Node Template的Core损失选择 / 原始观察问题 | 导出地址、任意跨域location、安全批准或未发现内容已完整的证明 |
| ExportInputCatalog / ExportProjection / ExportLossReport | 当前ExportPlan的完整受权输入 / 有限渲染投影 / 固定提案的导出损失报告 | 新作者source、任意客户端值、导入SourceLocation、Query rowHandle身份 |
| image_physical_size/1 / imageSizes | 可验证图片物理事实和量化 / 用户冻结的输出布局选择 | 用像素数/72或96 DPI/viewport假造源物理尺寸 |
| RegionBody / d9rg1 | 非身份profile几何事实 | 完整Locator、opaque registry identity |
| ResourceRegionLocator / l1 | 原D3完整ResourceRef/revision+geometry binding | D9新locator kind、inner重复fresh Ref |

公开名称固定用Field而不是模糊property/attr；FieldId完整namespace/path，不采用UI label。header仅D2 raw header，meta仅title/subtitle，node.id为授权显示。data.SET.COLUMN只在本次显式dataset内。paragraph/table row、Field occurrence、Node Collection、Query row四域不互换；dataset是导出输入通用称谓，不是新持久Record类型。future ICS、OriginBinding、mapping_table只在原D3/D6明确profile下才有语义，本代无隐式支持。

错误词表只有主文d9_error.code、D1原reason、D7/D3/D6原错误和Worker内部failed.code四域。Templates中的ambiguous_binding/missing等是诊断说明，外部d9_error映template_invalid或mapping_required；不能另发未定义code。Diagnostic.code只取主文d9_error闭集，feature只取IR/loss闭集。message纯文本不是可稳定分支的wire枚举。

命名检查：无新durable EntityRef/Record/FieldRef；D9数字Counter与D4 numeric lexeme不同；originClass是host输入渠道，不是D3外部身份origin；outputSlot/resourceKey/sourceIndex是局部ordinal。新profile/version未知一律拒绝，不保留旧公开原型alias。未来实施同步删除旧解析器、帮助、示例和测试中的有效旧名称，历史研究可保留并明确非权威。

总控词表检查须在最终candidate与实际gate之间再次执行；此候选正文不是已完成独立命名门的声明。

当前 PreparedActionBinding/3 是 D7 唯一 owner 的受管准备记录，constructionInput只存来源与编译证据；不是新Action、Ref、D3 mode或commit入口。resultAllocations与identityMap按原mode区分，来源到实例显示只是sourceSubjectBindings与原receipt的机械join。hiddenPolicy是固定mapping输入，accept_loss只是确认固定损失，不执行include/omit程序。

TemplateLossLocation只为模板输入pin或明确omitted Annotation提供证据位置，分域于外部IR SourceLocation；不成为D3 Locator、写目标或新durable地址。

ExportLossLocation所有ordinal只在同一不可变Plan目录/完整结果/projection内解释；重复同值行不合并；报告摘要不是新的身份和授权。source_range的UTF-8 byte边界与template_range的Unicode scalar边界明确不同。Export确认、外部PublicationReceipt与原D3/D6作者receipt分工不变。

ExportContentSelection/1是导出Plan的内部消费选择证据；bodyInput/bibliographyInput只绑定本目录中的准确Document输入，null表示未请求。它不是作者schema、权限授予或新增公共wire；input pin存在不等于已选择全部内容。

## 当前版本的来源与责任词表

| canonical term | 当前唯一含义/owner | 明确禁止 |
|---|---|---|
| SourceVersion/2 | D6 封存的 managed 生产版本，或明确 external 生产观察版本；携原生产域/世代 | 当作当前读取 token；externalSequence 冒充 managed revision |
| SourceObservation/1 / SourceVersionRef/1 | D6 当前观察的完整受保护对象 / {entityRef,sourceToken} 窄投影 | 搬用发送端观察、相同 Counter/bytes 重签、冲突安装输入当 current source |
| DecisionKey/2 | 原 D3/D6 的完整 Workspace、CommitDomain、OperationId 单决议地址 | 跨域合并同 OperationId；worker 重跑创建第二决议 |
| TemplateRecipe/2 / TemplateSourceAddress/2 | node-template/2 的持久配方 / 精确固定 managed 生产地址 | follow latest、保存当前 sourceToken、猜 Recipe/1 的生产域、自动改写 body Locator |
| TemplateConstruct/2 / TemplateConstructionInput/2 | 当前源选择的请求 / 完整当前观察、真实 source pins 与编译证据 | 任意调用方 AST、第二 ActionSpec 或提交、省略目录扩大到 Annotation body |
| D9EntityVersionAddress/2 | 同 cut 受权目录中完整 Ref 与生产 SourceVersion/2 的证据位置 | 单独当当前 Observation、SourceVersionRef 或写能力 |
| ConversionInput/2 | 当前完整输入及构造投影的不可变产品 ZIP 证据 | 用 partOrdinal/摘要重建丢失用途 pin；把外部工件称为 managed source |
| ExportPlan/2 / ExportInputCatalog/2 / PublicationReceipt/2 | 当前域完整输出准备 / 真实来源目录 / 仅外部发布事实 | D6 CP3、作者 receipt、重新渲染补原 bytes、unknown 换名重发 |

ImportIR/1、WorkerInvocation/1、Office template/1、原 SourceArtifact/IR 坐标、ExportContentSelection/1、ExportProjection/1、ExportLossReport/1、query-result-export/1 与 bundle/1 没有因状态行整体升版。各自新版/旧版只按实际 decoder 和具名载体决定，不能在旧 shape 暗添字段或按缺字段猜。

“完整来源”指本操作真正消费或明确枚举省略的有限受权范围，不是默认读取全部 Workspace/body。“当前”属于当前观察/授权 cut，“生产版本”属于源历史，“外部发布”属于原 publication intent；三者不得因一个相同摘要混为同一寿命。semantic_pending 不能放行本篇需要完整证明的导入/构造/Query 导出，也不掩盖 typed/source invalid。

PL-IR-01、D3/D6 IR-06/07 和 D7 当前完整 producer 的具名独立门保持；不存在作者自称命名检查通过即整包接受的规则。真实历史 saved/planned/unknown 的原 decoder、最小披露、pins 与不可重复责任依旧存在，不凭旧原型文字虚构已经部署的兼容层。
