# 本机设备状态

本模块是 v0.7 原生宿主、提醒和个人客户端设置共享的 Core 私有基础。它不定义另一套笔记、任务、提交事实或同步身份，也不表示原生宿主或提醒已经完成验收。

Core 的一个 `instance.lock` 租约拥有 `<data-dir>/device-state`；其物理目录及祖先保留 ParentGuard。同步的 `replica_id` 属于特定 workspace/replica，不能冒充这个本机 profile 的设备身份。

## 唯一身份与复制边界

`identity.json` 使用严格版本 1 记录，包含 canonical UUIDv4、明确的保护格式和有界 sealed binding。Windows 使用非交互 DPAPI `LOCAL_MACHINE | UI_FORBIDDEN` 保护 `{version, id, user SID, original Core-directory FileIdentity}`。核验同时要求当前 Windows 保护可解、用户 SID 相同、原物理目录的 volume/index/created 相同。它不是认证凭据，也不把全机器镜像克隆推定为全球唯一新设备。

`Core::device_identity` 返回 `{id, isCurrent}`。新空状态首次生成一次；仍在同一物理 profile 的普通进程重启、升级或目录改名保留身份。复制到另一个目录、另一用户或不能解封的 Windows 环境时不自动 adopt，也不覆盖或分配另一个 ID；返回非 current 或明确证据错误，原件保留。identity 缺失但其它设备记录或原 pending 仍存在时，拒绝生成新身份。显式导入/重新绑定须另有保原件和提醒 unknown 的完整语义，不能由启动过程猜测。

## 独立记录与 CAS

合法叶只有 `identity.json`、`desktop-preferences.json`、`reminders.json`、`client-preferences.json`。三项桌面 intent 是 `resident`、`openLibraryOnLaunch`、`loginStartupRequested`，默认全部 false，按 revision CAS 原子持久。登录 intent 不是系统 StartupTask 的实际 Enabled/DisabledByUser 状态。foreign/unverified 的原设置可检查，生效值全部关闭且不允许覆盖。UI locale 由独立客户端设置 owner 管理；提醒选择、opt-in 和 ledger 由提醒 owner 管理，不塞入桌面 intent 或另建设备 ID。

内部固定 `DeviceRecordKind` 的有界读返回完整原字节和小写 SHA256；CAS 的 None 只创建缺失记录，Some 只替换相同原摘要。这是单一 Core 租约内 `&mut` 串行消费者的 CAS，不声称为任意外部同用户文件编辑提供 filesystem 原子 compare-and-swap。先写受 guard 的新临时文件、flush/close，再同卷 WRITE_THROUGH 发布，最后严格完整 readback。身份和桌面设置在写入前按各自 schema 校验，其它消费者也必须先执行自己的严格校验。失败、读回不一致或未知效果不自动重发/回滚；保留原件和临时文件。临时名只允许 `.<known-leaf>.<canonical-UUIDv4>.pending`；这不是任意后缀白名单。预算为 identity 64 KiB、桌面/客户端设置各 16 KiB、提醒 16 MiB、单目录 128 项；超预算明确失败。

## 库备份与验证

库 owner backup 明确资格核验这个设备私有根、已知叶/临时名、原对象和严格版本 schema，固定路由见证，保相同句柄；device-global payload 不成为库作者的 CapturedFile 或备份内容。未知成员、目录/别名、坏 schema、尚未链接真实严格 validator 的记录均拒绝，不泛忽略整个私有根。R1/客户端 owner 接入时须将各自真实 validator 接到唯一分类入口，不能把 `{}` 当可跳过的成功状态。

该路由见证只固定物理设备根及 `deviceGlobal:v1` 分类，不包含已排除记录的合法成员集合、payload 或 stamp。每次捕获/执行仍重新严格核验；合法提醒账本/桌面设置更新不会使无关作者库备份失效，新增未知成员仍阻止执行。

有意义的验证包括同 profile 重开、设置相互独立、过期 CAS 不改原件、物理复制不 adopt、损坏/未知/超预算保原件、目录 guard 和备份排除/拒绝。源码和受控测试不能代替真实升级卸载、跨机器保护、系统开关、完整网络隔离或断电验收。

平台依据：[DPAPI protect](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata)、[DPAPI unprotect](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptunprotectdata)。
