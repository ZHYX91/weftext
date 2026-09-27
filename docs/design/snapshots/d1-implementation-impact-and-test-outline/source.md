---
_weftext:
  id: "042833a6-ecb2-4909-8712-ce4ec464ff34"
---

# D1 实现影响与测试轮廓

状态：D1 冻结后的内部实施输入。本文件不授权当前任务修改代码或公开规范；实际实施仍等待 D2–D10 与 A2 门禁。

日期：2026-08-27。

权威决议：[D1 产品表面与能力边界](../d1-product-surface-and-capability-boundary/source.md)。

## 1. 实现影响图

```text
stable bootstrap major negotiation (discovery only)
  -> selected contractMajor
  -> shared capability catalog + deterministic unavailable reason
  -> Core public operation/result boundary
      -> Desktop native command allowlist + local/remote mode
      -> CLI local Core / remote Server + optional local broker
      -> Server authz/audit boundary + embedded WebUI
      -> future Mobile packaged Core / remote Server mode

single commit-holder invariant
  -> one local process per physical replica
  -> one Server instance per hosted backend across restart overlap
  -> second holder rejects writes or remains read-only
  -> D6 chooses coordination mechanism and recovery protocol

optional external capabilities
  -> isolated conversion worker
  -> Desktop/CLI or Server Agent/automation/connector broker
  -> staged output or proposed action
  -> authoritative Core/Server revalidation and commit

release support evidence
  -> native build artifact
  -> manifest/hash/SBOM/licenses/known limitations
  -> package smoke + independent clean-install evidence
  -> GitHub Release and About/capability projection
```

## 2. 组件影响

| 区域 | 冻结影响 | 禁止形成的替代权威 |
| --- | --- | --- |
| `crates/weftext-core` | 提供跨表面的规范操作、结果、诊断、能力 ID 和提交边界；不得依赖 UI、Server、worker、Agent、Provider 或网络 | UI/Server/CLI/Mobile 自算领域、权限、冲突或提交结果 |
| `apps/desktop` | 本地模式承载 Core，远端模式只调用 Server；WebView 通过窄 allowlist；可选本地 worker/broker 在 Core 外 | WebView 原始路径/字节写入；本机 Core 解释远端工作区 |
| `crates/weftext-cli` | 本地命令调用 Core，远端命令调用 Server；未来可启动可选本地 Agent broker，但无第二 Agent 权威 | CLI 专有语义、隐藏确认、broker 直接写工作区 |
| `crates/weftext-server` | 托管工作区唯一在线写入边界；同一后端跨全部实例/重启窗口只授予一个提交资格；第二实例拒写或只读 | 每个 Server 实例各自成为写入权威；控制面数据库成为内容副本 |
| Server `webui` | 只作为 Server 客户端与同源发布资产；不打开本地文件或运行 Core/worker/Agent | 浏览器本地工作区、浏览器直连 Provider/worker |
| future Mobile package | 本地 packaged Core 和远端 Server 客户端；当前 D1/G1/G1.1/G2 无转换、Agent、自动化或连接器运行/凭据管理 | WebUI 包装、移动专用领域语义、隐藏远端转换/Agent 入口 |
| `weftext-import` / worker | 工作区外隔离输入输出，只返回 staged result 或变更意图 | 可写工作区挂载、工作区内暂存、自动监听/自动提交 |
| `weftext-agent*` / broker | Desktop/CLI 或 Server 可选外部能力；主体和批准范围不扩权；最终写入经 Core | transcript/模型输出即工作区状态；Provider/Core 私有提交 |
| capability/version boundary | 固定只读 major 启动协商后使用所选 major 的能力描述；封闭 reason 与固定归一顺序 | bootstrap 成为第二业务合同；UI/平台自选 reason |
| release workflow | 逐工件证据、无生产发行签名文案、GitHub 唯一公开渠道；Mobile debug/development signing 只用于测试 | 源码可编译即 Supported；开发签名变成生产发行承诺 |

## 3. 依赖与发布切片

1. 共享 Core 合同完成后，G1 只发布 Windows 11 x86_64 无生产发行签名 NSIS Desktop 和 Windows x86_64 ZIP CLI。
2. G1.1 只在独立证据行通过后发布 Ubuntu 24.04 x86_64 CLI；它不阻塞 G2。
3. Server 安全基础、WebUI 和 D6 权限/事务/备份边界完成后，G2 发布 Ubuntu 24.04 x86_64 Server + 内嵌 WebUI；实时共同编辑不在 G2。
4. Mobile 本地实现不依赖 Server，但普通用户生产分发等待未来独立渠道决策；临时开发签名只进入 conformance 证据。
5. 转换依赖 D9，Agent/自动化/连接器依赖 D10；它们都不阻塞基础表面，并始终位于 Core 外。

## 4. 删除和替换目标

- 删除或重建 `prototypes/webui`；不保留原型兼容桥。
- 删除任何 UI、CLI、Server、worker 或 Agent 直接写工作区的路径。
- 删除旧 surface/capability 名、别名、fallback、双读写和作者声明依赖的入口。
- 删除把源码 CI、图标或配置存在误称为平台 Supported 的文案或门禁。
- 删除 Mobile 转换、Agent、自动化和连接器运行/凭据入口。
- 公开规范只在相应实现、fixture 和测试原子落地时更新；本 D1 任务不执行这些删除或替换。

## 5. 测试轮廓

1. **静态依赖**：Core 无 UI、Server、worker、Agent、Provider SDK 或网络依赖；Mobile 包无禁止组件；WebUI 无文件系统或 worker 直连。
2. **本地单持有者**：Desktop/CLI/其他本机进程竞争同一物理副本时只有一个可提交，其他调用失败或只读。
3. **托管单持有者**：双 Server 同后端启动、重启重叠、资格移交和故障切换全过程只有一个可提交实例；第二实例明确拒写或只读。
4. **外部变化失效**：共享文件夹部分到达或外部编辑被发现后，旧提交资格失效且停止写入；不静默合并。
5. **启动协商**：没有共同 major 时，五表面均解析 `incompatible_version` 和双方支持 major 集；协商失败前不能读取 capability 或工作区。
6. **reason 归一**：未知 ID、表面禁止、未交付、策略、用户授权、组件、配置、网络、版本、暂时故障的单因和重叠组合遵循固定顺序；`policy_denied` 不泄露被遮蔽详情。
7. **跨表面一致性**：同一 fixture/意图在 Desktop、WebUI、Server、CLI、Mobile 得到相同规范结果、诊断、计划、提交和不可用投影。
8. **远端边界**：Desktop/CLI/Mobile 本机 Core 不读取、解释、计划或提交远端工作区；浏览器不获得托管路径。
9. **worker/broker 负向**：无可写工作区挂载、无工作区内暂存、无自动提交；CLI Agent broker 不嵌入第二工具/权限/提交权威。
10. **发布证据**：每个 Supported 工件具备原生构建、摘要、SBOM、许可证、包级 smoke、独立 clean-install、已知限制和真实系统警告记录。
11. **签名边界**：公开工件和文案不声称生产已签名；Mobile 临时 debug/development signing 工件不进入 GitHub Release 或 Supported manifest。
12. **文案负向**：不出现跨平台 Desktop、Mobile 生产包、同步即协作、WebUI 本地文件、生产已签名/已验证发布者、转换/Agent 已内置等超证据声明。
13. **退役门**：仓库扫描禁止旧原型名、旧 capability、别名、fallback、双路径和 prototype bridge 回归。

## 6. 验收边界

D1 冻结只证明目标边界可机械实现，不证明当前仓库已符合。每个未来实施切片必须同时更新 Core、调用方、fixture、测试和中英文公开文档，并将真实构建/安装证据写入 `04 Acceptance`；在证据完成前不得产生新的 Supported 声明。
