---
_weftext:
  id: "a370f683-ad9d-468d-8a92-eb38b0fe872d"
---

当前权威状态：D8 D8-r03-p2-2026-09-24，仅由总控验收（外部控制记录未随本输入发布）和committed 协调 journal（外部控制记录未随本输入发布）共同确定生效。下面送审正文中的candidate/未激活为历史标签，不覆盖本段。架构接受不代表产品实现；D9未启动。

# D8 实现影响与测试轮廓

revision: D8-r03-p2-2026-09-24；candidate。以下是后续实施义务。本次只执行明确标注的有限模型与静态审计，不宣称真实Core、IME、renderer、AT或性能已实现。

## 1. 实现图与删除目标

```text
Desktop / WebUI / Mobile host input & accessibility adapter
              ↓ normalized edit / composition / selection events
Shared EditSession controller + private device draft journal
              ↓ explicit intent / draft projection request
Local Core or Server Core
  D8 document / annotation adapter → immutable D6 PreparedIntent
  D7 Field / native / collection / lifecycle Action adapter
              ↓ shared complete preview and effects transport
Explicit user confirmation → original D3 or D6 request
              ↓ original ledger / CAS / durable commit / replay
Committed source + receipt → invalidate / reopen projections
```

Core新增closed D8 read/project/text replacement/plain writing/prepare入口，复用D2 parser原始span、D3 Ref/Locator/Annotation值、D4完整typed验证、D6 scope/预算/footprint/ledger、D7定义/effects。必须传完整Draft Edit Map并实现grammar-complete EOF/site、普通raw region的空白与多行后继状态、独立readonly来源、异步输入队列及精确serial规则；native列/多row批量Source生成器本代全部关闭，不宣称D5完整实现。DraftProjection必须有独立类型，不允许把draft ranges或剪除locator后的对象送入D2 wire。

共享UI库实现状态机、command routing、serial控制、undo groups、target绑定、selection坐标转换、direction/layout epoch及日志恢复。host adapter只处理OS/browser/IME/clipboard/AT差异。Core与UI boundary的适配必须可headless验证。

现有原型Source/Write/Read、局部IME guards、dir=auto及物理CSS只是迁移删除/替换目标；不能原样保留旧的source autosave、直接API语义、旧Task/Record/文件权威或旧parser分支。后续实现原子更新调用方与中英公开规范，不同时维护新旧编辑状态机。品牌图形保持原样，不因RTL翻转Logo。

## 2. 延迟、预算与规模验收

这些数值是拟交付产品的目标，必须在release声称的每一host行记录CPU/内存/OS/browser/font/AT/build、输入语料、冷/热态和原始分布；当前没有测量结果。

| 场景 | 目标及测量方法 |
|---|---|
| 输入、方向键、selection feedback | reference corpus上从host已交付事件到可见feedback的p95≤50ms，连续输入不得丢失/重复字符；每组≥1000事件、冷/热各3次 |
| 命令、prepare、网络等待 | 接收命令100ms内给忙/不可用反馈；完整Core工作异步，未知提交明确显示；服务时间不硬装成固定成功时间 |
| 取消重计算/滚动 | UI收到取消100ms内更新状态，Core在已声明work-unit检查点停止；不取消已耐久commit；记录实际Core停止延迟 |
| 方向/缩放/layout重建 | 空闲时p95≤100ms恢复当前viewport可操作；composition期间延迟布局且保留宿主，不拿瞬时动画代替完成 |
| 大文档 | Desktop/WebUI reference 10MiB、100000 logical lines；Mobile本地reference 1MiB、10000 lines；另测单一1MiB logical line、深度/容量边界及10倍超预算输入 |
| 大集合/导航 | 100000 Node导航、10000-row result，D5 fetched page≤200；多页选中准确，虚拟化不把DOM行数当全集，完整Query遵D7预算 |
| 内存与磁盘 | reference editor附加内存峰值：Desktop/WebUI≤256MiB、Mobile≤128MiB；含Draft/base/undo/viewport映射，Core result spool另按D6预算明确记录；两者不可漏计到总应用限额之外 |

reference corpus必须同时有Arabic/Hebrew/CJK/Latin/emoji/CRLF与合法D2长源，不能只用ASCII空白。增加内容规模时不得改变身份或语法。超预算应在分配/解析前或累计账户边界完整拒绝/明确进入已声明的Source/Read能力模式；不能少解析一半仍声称canonical valid。若某release host达不到其承诺reference目标，不标该编辑能力Supported；低资源策略可另列可核验容量行，不掩盖失败。

解析与projections可增量，但结果必须与完整Core parse语义、primary diagnostics、source ranges及拒绝一致；不能证明则执行batch或不提供当前Write。UI viewport虚拟化独立于Core完整投影。每次viewport最多挂载可视行加有限overscan；实际焦点和composition host额外pin，不随滚动回收。日志容量按明示策略限制，不能为控内存静默丢未提交Draft；不能用自动author commit缩短日志。

## 3. 必要测试分层

1. **本次有限模型**：scalar/UTF8/UTF16与D2换行映射；固定IME事件trace的exactly-one本地事务；serial/preview/未知提交/replay状态探索；方向改变不改变typed意图；固定revision下selection/target正确。测试可以发现这些类的具体错误，不能证明真实UI/完整Unicode算法或Core权限。
2. **Core实现conformance**：真正D8 closed decoder，真实D2/D4/D7源与Registry，完整D6潜在scope/footprint；两隐藏世界差异、拒绝顺序、CAS/ABA/no-op；真实byte pins、EffectManifest全部页/epoch、lost receipt、崩溃恢复及D3新建/lifecycle路径。
3. **Unicode/layout实现**：固定Unicode18官方BidiTest/BidiCharacterTest/GraphemeBreakTest/WordBreakTest全量数据；shape、font fallback、dual caret与行折叠平台fixture；DOM UTF16/native offsets与逻辑source双向一致；Bidi_Control inspection不污染clipboard/source。
4. **真实输入与辅助技术**：Windows IME/Arabic/Hebrew layouts、macOS等实际声称平台的输入，浏览器实际composition序列；iOS/Android软键盘/死键/语音/手写/外接键盘；NVDA/VoiceOver/TalkBack及选定浏览器读屏组合。合成事件测试不能代替。
5. **端到端场景**：Acceptance Matrix全部正反例；每个已声称host、LTR/RTL、local/remote模式按适用矩阵运行。没有可用硬件/服务则标未验收，不假pass。

## 4. 后续阶段和本阶段完成门

D9须决定资源region、富剪贴板转换、TSV/CSV/Office映射与模板输出；D10须决定外部能力、认证贡献、非本代自动化。A2需复核D8新适配器和D7效果范围的完整组合，以及显式提交/Write子集的产品代价。这里不启动这些阶段。

D8阶段可接受的是完整架构和上述明示后续实现门，须有真实独立Chat GPT-6 Pro完整审查及总控逐项裁决。地方模型与静态检查不把implementation-pending改为implemented；任何未关闭架构P0/P1阻止冻结。

检查方法补充：有限oracle必须从D2 grammar输入族与操作后的后继状态生成用例，不只检查预制单行paragraph；对空白、EOL、最后非blank删除、恢复无历史、selection方向逐状态枚举可用操作。来源覆盖以每个body kind/Inline relation和重复位置为轴，分别检查navigation availability及editable eligibility，不能把null编辑映射当作已完成导航证明。真实Core仍须完整D2词法/IR/source来源、嵌套parent和保护边界测试；有限oracle不覆盖的类别逐项保留为实现门。
