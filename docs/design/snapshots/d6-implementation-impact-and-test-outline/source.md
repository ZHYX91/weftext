---
_weftext:
  id: "3762120f-fb70-4cf7-9699-68602e9bf9fc"
---

当前权威状态：D7 coordinated D7-coordinated-r06-terms01-2026-09-23，由总控验收（外部控制记录未随本输入发布）与committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。正文保留送审时的候选/未激活及原阶段历史描述；这些描述不覆盖本段当前状态。本次仅接受架构，D8 未启动，产品实现与发布不在本次范围。

状态：D7 revision05联合replacement草稿，尚未接受或激活。基线接受记录仅属历史；必须以本次完整独立审查、总控验收和coordinated journal共同决定生效。稳定文档ID保持。

# D6 Implementation Impact and Test Outline

本文件是后续实施义务，不是产品实施授权或已运行验收。D1/D2/D5公共语义保留；D3/D4按同包必要replacement演进；D6新增存储、控制接口与执行机制。架构接受之后仍须按独立授权实施代码/公开规范及实际host测试。

## 1. 共同迁移范围

现有原型的物理目录/sidecar、第二份Field或Record作者数据库、直接文件写入、UI/worker/provider事务捷径不能进入新模型。统一改为Core StorageDomain：完整source payload、D3 control/ledger、D4受管Registry与D6 policy/binding在一个提交域。公开API和实现同批采用新接口；不保留双写、后台同步作者副本或启动时静默批量迁移。已有用户数据的实际转换如另获授权，须独立输入artifact、预览、验证和明确激活，不能套用本架构设计授权。

## 2. 串行实施切片

| 切片 | 实施职责 | 合格证据 |
|---|---|---|
| S1 StorageDomain/backend | typed namespace隔离、source bytes/chunks、SQLite资格、OS锁/别名、WAL/FULL和故障打开 | exact source roundtrip含CRLF/trivia/未知namespace；同host第二writer；磁盘满、WAL损坏和路径替换；实际VFS/文件系统/设备证据 |
| S2 Control transaction engine | D3 A1–A11/stage1–15原序、bootstrap family/custody、D3/D6 shared ledger owner、fence、计划pins和commit/outbox | 正反向closed decoder、同key不同owner、并发winner/loser、所有崩溃点、receipt丢失、恢复授权和fence变化；不得用D6模型测试代替D3原corpus |
| S3 Authorized reads/MutationFootprint | exact-ref state-disclosure、subtree内容权限、完整源内部读、真实改动授权/typed gates | parent隐藏/spouse可见跨全部生命周期；phone-only改phone成功/夹带name或Facet失败；不可见事实参与约束但不泄露 |
| S4 Snapshot/dependencies/index | 正负范围证明、source/Registry/Calendar版本、原子失效、独立派生index | 空scope并发插入、purge与incoming、relation canonical迁移、many→unique、旧index/缺事件、cold与incremental语义相等或reset |
| S5 Results/exports | complete后分页、闭合错误、cursor/epoch/audience、每次交付当前授权、受限worker输入 | 末尾类型错误/图环、空nonterminal页、top/limit、撤权和expiry/reset竞争、导出publication/download中撤权、无身份result/schema |
| S6 Import/sync/checkout | 输入冻结、DAG/SCC分组、D3 binding矩阵、三方源比较、watermark、provider外发outbox | 10k Nodes/10批、第6批commit前后故障、大于working memory、重复续作、template后代计数、ICS多source/recurrence/retire等六组、无幂等provider禁自动双向写 |
| S7 Portability/recovery/retention | 一致snapshot/完整控制闭包、同域continue、跨域只读/fork、recent history/backup、GC pins/预算 | 旧备份不能复活burn/tombstone；授权后原bytes重放；history不可从普通purge resolver读回；pending额度耗尽不阻止管理恢复；跨域continue如未交付正确not_in_release |

每片先闭合共享Core机制，再接Desktop/CLI/Server/WebUI/Mobile适配。Mobile不因此获得D1未交付的conversion/Agent能力。Server多前端仍只有同域唯一实际提交持有者，不写“多实例”便宣称分布式存储高可用。

## 3. D4–D6宏观非回退

- D2 full-source validation与五字段attestation不变；UTF-8错误留D6 byte/repair层，不造D2新错误。
- D3的exact Ref/domain、lifecycle、proposal、fingerprint与disclosure保留，并完整验证wire11十数组/Result9/组合receipt；store schema和D6 Token不得进入内容身份代数。
- D4完整生成Catalog、Field-owned relation/qualifier/temporal rule/Calendar policy及所有operation-applicable证明逐真实源接入；copy/fork实际payload不能靠effect自证。
- D5不引入持久Record域；重复值保持独立occurrence/notes；所有collection success绑定完整post-query及固定targets，原生table编辑保留D2语法；没有隐式membership sidecar。

## 4. 场景/性能与证据层次

强制输入全文与D4/D5场景处置随架构包提供。D6场景表给机制和预期可观察结果；它不是运行报告。架构阶段的有界SQLite实验只验证所声明的极小提交模型/故障点，不能证明完整Core、D3/D4实现、性能或真实客户端验收。

实际实施验收必须覆盖至少10000 Person Nodes overlapping engagements的按需colleague查询，不物化约5000万关系；10000 Nodes/10批导入与大于working-memory完整输入；完整语义query/cancel/error/输出等价；多实体修改与撤权/恢复组合；实际候选host包和版本绑定。D5的1000/200是动作/展示边界，不是全Workspace或普通Query的总量上限，更不是无卡顿证明。benchmark及磁盘/内存/延迟目标必须单独呈现真实数据。

## 5. 下游冻结义务

D7：closed Query/Action/View schema、typed result/delta wire、operator incremental等价、完整post-query证明、无身份row与ActionEvidence。D8：有限预览、真实partial import状态、冲突base/current/proposed、明确可移植性与not_in_release呈现。D9：格式parser/IR/映射与loss、template后代计数、输入稳定性及import job wire。D10：principal/贡献者认证、trusted rule和Registry来源、provider版本/条件写与幂等、受限worker/broker。各方只能调用Core冻结语义入口，不能用“adapter以后补”绕过D6原子性或权限。

D6不授权D7开始，也不以本文件为任何产品代码、发布、部署或生产数据变更的许可。


## 6. 本次新协议非回退检查

本地revision02-model-report.json明确列出18项有界检查及未覆盖范围，包含真实SQLite进程在fresh+existing+watermark commit前/后退出和精确重放；只有具名结果是已运行证据。C模型使用有限NodeRef根及模型Registry占位，不是完整D4可信context。完整产品decoder、全部C事实图/顺序/locator/S、所有D3 modes、真实认证与host性能均仍须S2–S7真实实施验证。

系列配置范围为series+scope跨全部periodKey，unique按各完整D4 key分组，版本读取→准备→管理commit复用closed接口。所有资源消耗/补充改变resource-policy revision并CAS exact旧counter，避免覆盖消耗。永久paused计划可能持续占用pins直至总容量耗尽，这是明确成本；管理读取及新增实际容量不等于改变原语义预算或获得abandon权限。

D6 revision03增加跨阶段consumer库存与ByteHandle验收：固定R1而非latest、完整ResourceRef/版本/descriptor/ByteHandle域、跨tag/audience、当前授权与撤权线性化、expiry/reset/range/pin优先级、真正EOF/short read/I/O失败、binary/0字节/非ASCII与跨chunk无洞、Counter及共享预算并发CAS。Resource读取不验收Document repair或上传运输；Effects运输须在实现接口冻结前独立closed，不得D7 row fallback。永久paused pins须明确主体/Workspace持久占用准入和管理保留容量；不新增abandon或释放原pins。

联合恢复验收补充：trashed Annotation restore加target reference/S reply改写时，存在的typed preimage slot取non_live_source；null旧reply单独取absent；target与reply共用一次最终revision。ByteHandle当前授权缓存因generation变化失效，但仍获权时旧版本pin可继续，lifecycle/continuity reset与auth重验严格分开。

D4 RelationReadContext/2、RelationReadBinding/2及FacetOperationRequest/2为本代状态/源分域的受信接口，D6绑定既有lifecycle或prepared-origin依赖；不会扩大D3最小tombstone或新增content ref。完整消费者与C44/C45正负验收按D4和D6主稿执行，旧运行语料保持历史范围。

受信D3 fresh Create conformance须验证结果revision=1、expectedOwnerRevision=1、request声明/initialEntries与完整暂存结果精确一致，成功不二次append/increment；revision0假前像、结果2、缺失/重复initial Entry、现存owner冒用fresh origin都拒绝。普通existing实际修改仍加一次。

## 当前观察与初始化实施义务

接入D6接口§13固定潜在观察scope、D3 stage3与D6最小授权定位、空范围前置权限、当前policy/Registry CAS、replay/recovery/delivery；成对隐藏状态在无权时必须同一not_visible且零业务读取/decision，有权时仍完整检查真实约束。纯policy管理保留control_only恢复路径。实现§14 issuer首次信任与可撤销grant、family固定profile、独立target principal、完整Registry seed/source重绑、初始policy/Calendar配置和scope绑定一次activation；完整fork保留multiplicity且不复制ACL。实现§8.1 scope选择与迁移、空配置删除、控制inbound和ABA版本。模型不是完整decoder/host验收，需在对应实施入口冻结并实际验证所有运输、认证与故障前提。

## D7联合revision03消费补充（未接受）

当前协议基线为D3 wire11/Result9；D4 C及全部领域类型语义不变。新增受控名称和exact位置：PreparationBinding=D3.identity_operation_request.preparationBinding；DefinitionTransfer=D3.intent.plan.definitionTransfers[]；D3-Symbolic-Result/9.Q（定义结果分段）=D3-Symbolic-Result/9的Q；PreparedActionBinding=D7受管不可变准备输入；SourceEnvelopeStateCapability/CommitSequenceStateCapability=D6 Policy/2.capabilities；EffectManifest/EffectBytes=D7只读效果运输。它们均非内容实体或第二ledger。完整定义以同包D3主稿§21、D6 Control Interfaces §15–16及D7专项正文为准。

必须新增实际版本解码/legacy replay、fingerprint差异、planned恢复、保存定义typed引用转移/Locator、完整效果及窄Field正向outcome测试；历史wire10与Result8测试保留历史归属，不以数字替换声称新版本通过。其它原实现影响和必需检查不因本补充被删除。
