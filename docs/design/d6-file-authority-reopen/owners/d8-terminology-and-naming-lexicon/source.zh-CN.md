---
source_language: zh-CN
translation_status: source
---

[English](source.md)

源文档 ID：38d4f1f1-958a-4b10-a3e0-7d1627f87f3e。

# D8 术语与命名

候选状态：D8-FA-r01；完整 owner 后像，尚未独立接受、激活或实现。固定来源 S 为 7e18168dad3e6d120fce0dd607dc10fa7894e252。原验收、模型与平台证据仅保留其原有范围；本候选的存在不是实现证明。D3 冲突适配器、D7 当前效果生产端与跨副本 Locator 当前资格的联合协调仍须分别完成并独立接受；这些整合门不追溯取消真实历史决议的恢复义务。

revision: D8-FA-r01；candidate。沿用D1–D7既有concept/Ref/Locator/Field/Action/PreparedIntent/EffectManifest定义，不重命名其领域。新概念仅用于下表明确层级；公共wire名字列出的全集属于Editor Interfaces/Direction合同，不能把review工具、文件名或测试计数混入产品术语。

| concept | canonical English / 中文 | 所有者与命名 | 禁止混称 |
|---|---|---|---|
| weftext.term.edit_session | Edit Session / 编辑会话 | UI状态；EditSession | Workspace、Document identity、提交权威 |
| weftext.term.edit_draft | Edit Draft / 编辑草稿 | 非权威用户提案；Draft / draftSerial | 已保存作者源、sourceRevision |
| weftext.term.draft_projection | Draft Projection / 草稿投影 | Core对本次proposal的可丢弃投影；d8_draft_projection | D2 document_payload、合法D3 locator |
| weftext.term.draft_edit_map | Draft Edit Map / 草稿编辑映射 | Core输出的flow/path/segment/plainRegion/site；仅绑定一次完整proposal | D3 Locator、持久段落身份、客户端parser |
| weftext.term.prepared_edit_binding | Prepared Edit Binding / 编辑准备绑定 | Core 保存的不可变 D8 编辑准备记录；d8_prepared_edit_binding | 不能混称 D7 PreparedActionBinding、ActionSpec，不能成为第三份决议账本 |
| weftext.term.composition_transaction | Composition Transaction / 组合输入事务 | 界面中的开始、更新、确认或取消输入组 | 不能混称 D6 作者内容事务或操作回执 |
| weftext.term.caret_affinity | Caret Affinity / 光标亲和位置 | 同logical点的visual side，upstream/downstream | source偏移、永久定位器 |
| weftext.term.layout_epoch | Layout Epoch / 布局代 | 同一次字形布局、折行与命中测试，客户端可以丢弃 | 不能混称结果的代、授权代或作者内容版本 |
| weftext.term.direction_preference | Direction Preference / 方向偏好 | 设备或会话中的展示偏好，取值为 ltr/rtl/auto | 不能混称语言区域设置、可携带的作者方向或查询排序 |

D8新请求/响应kind闭集：`d8_document_read`、`d8_document`、`d8_draft_project`、`d8_draft_projection`、`d8_draft_text_replace`、`d8_draft_text_replaced`、`d8_draft_write`、`d8_draft_written`、`d8_edit_prepare`、`d8_edit_prepared`、`d8_undo_prepare`、`d8_editor_error`。内部binding kind独立为`d8_prepared_edit_binding`。`document|annotation`只是D8 intent的局部discriminator，不能增加D3实体kind。成员与closed error集合以Editor Interfaces为唯一源。

Source / Write / Read是模式名称；“只读”是是否允许当前交互的状态，不是第四份文档。“草稿已保存于此设备”“待提交”“结果待确认”“已提交”分别使用，不把autosave Draft写作Workspace已保存。`planned`仅表示Core已存原计划，`preview`不能称receipt。选区是selection，作用于源的连续范围为source range；视觉矩形不自动成为原范围。

“RTL支持”不得作为单一空泛标签：分别列Unicode内容保存、bidi交互、RTL shell、RTL语言UI翻译。`dir=auto`只是一种具体呈现机制，不是完整能力名称。阿拉伯引文locale不等于阿拉伯UI本地化。

新旧受控名称核验范围：本候选的上述九个概念及十三个kind声明与实际使用，D7运输修订对两种Prepared Binding的名称使用，D3/D6版本号消费者。无需扫描用户正文/代码样例普通词并宣称自然语言一词一义已机械证明。独立评审需明确术语是否通过。

BodyPath、flow、segment、regionIndex及siteIndex均为Draft Edit Map局部坐标；命令splice_plain/break_plain/insert_at_site是草稿变换，不是D7 ActionSpec或D6 author transaction。inputSerial是原请求回传，projection.draftSerial是下一个proposal序号，不能混称author revision。

origins是Draft Projection内独立的只读来源成员，elements/flows及relation不授编辑权，不是新的领域身份。plainRegion是Draft Edit Map的局部raw文本区域，不是D2 paragraph；空白区域不制造D2空实体。上游D2 Inline闭集严格为text/link/resource_occurrence/node_link/citation；kind:"resource"不是合法Inline。D3 Annotation target的resource分支仍是其原协议的合法名称，不可全局替换。


当前消费者命名：新 D8 外层为 wire2，D8SourceTarget/2 绑定完整 SourceObservation/1；Draft 的 baseObservation 是当前观察，不是生产 revision 别名。PreparedEditBinding/2 消费 D6 PreparedIntent/2、d6_plan/2、InputDescriptor/2、DependencyProof/2、Policy/3 和实际 D7 EffectManifest/2 producer；真实旧 /1 记录按原 decoder 恢复，不补成员。D3 原生 wire12 与 D6 wire2 的提交字段分别归其 owner。上述版本消费不增加第十个身份概念，也不更改九个 concept ID。

ReliableSaveState、InputRetention、WriteProtection、SemanticState、SourceStamp、SourceRevisionPlan、CommitDomain 继续由 D6 定义。界面必须区分 retained、reliable、durable_observed_only 与 portable 发布，不能把 semantic_pending 当 invalid 的别名或把 externalSequence 叫 sourceRevision。D8 TextReplace 响应没有 caret；只有 d8_draft_written 有闭合 caret 成员，排队定位不能以术语含混新增字段。
