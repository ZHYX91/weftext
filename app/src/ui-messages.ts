// Generated from assets/ui-messages.json. Run node scripts/generate_ui_messages.mjs.
export const zhCN = {
  "app.title": "Weftext · 写作与知识",
  "app.hostedTitle": "Weftext · 远端写作",
  "locale.label": "界面语言",
  "locale.zhCN": "简体中文",
  "locale.enUS": "English",
  "locale.unsaved": "本次界面语言尚未可靠保存；当前稿和原操作未改变。",
  "locale.unavailable": "界面语言偏好暂不可读取，使用默认语言；当前稿和原操作未改变。",
  "common.save": "保存",
  "common.cancel": "取消",
  "common.close": "关闭",
  "common.retry": "重试",
  "common.open": "打开",
  "common.unknown": "未知",
  "common.unavailable": "暂不可用",
  "common.originalDiagnostic": "原始诊断",
  "common.operation": "原操作：{operationId}",
  "desktop.library": "资料库",
  "desktop.capture": "新增碎片",
  "desktop.showLibrary": "打开资料库窗口",
  "desktop.exit": "退出 Weftext",
  "desktop.resident": "关闭窗口后继续常驻",
  "desktop.loginStartup": "登录时启动 Weftext",
  "desktop.openInterface": "启动时打开界面",
  "desktop.unsavedClose": "恢复稿尚未确认持久保存，窗口保持打开。",
  "desktop.waitingForInput": "正在等待输入完成和草稿保存。",
  "desktop.startupDisabled": "系统已禁用登录启动；Weftext 不会自行恢复此设置。",
  "capture.title": "新增碎片",
  "capture.body": "正文",
  "capture.target": "目标资料库",
  "capture.saveClose": "保存并关闭",
  "capture.saveNext": "保存并再记一条",
  "capture.openNote": "打开已创建笔记",
  "capture.saved": "碎片已保存。",
  "capture.persisting": "正在可靠保存恢复稿。",
  "capture.pending": "原操作结果尚未确认；恢复稿和原编号已保留。",
  "reminder.label": "任务提醒",
  "reminder.absolute": "独立时刻",
  "reminder.beforeDue": "截止前提醒",
  "reminder.dueDay": "截止日指定时间",
  "reminder.enabledDevice": "在本设备启用此提醒",
  "reminder.submitted": "系统已接受提醒；不代表用户已读。",
  "reminder.unknown": "系统提交结果未知，未盲目重发。",
  "reminder.unavailable": "当前条件不允许提交系统通知。",
  "reminder.missed": "有 {count} 项有效的错过提醒。",
  "editor.source": "Typst 源码编辑器",
  "editor.revealedSource": "Typst 原生源码片段",
  "editor.search": "查找",
  "editor.replace": "替换",
  "error.request": "请求未完成（{status}）。",
  "error.resourceRead": "资源读取未完成。",
  "cli.description": "正式命令行：读取、写作、查询、交换与同步。",
  "cli.requestRetained": "准确原请求已保留在 {path}；原操作：{operationId}。",
  "cli.complete": "命令已完成。",
  "cli.partial": "仅所述范围已完成，其余范围未计为成功。",
  "cli.unknown": "原操作 {operationId} 的效果尚未确认；请求保留在 {path}，仅按原编号查询。",
  "cli.error": "命令未完成，代码：{code}。",
  "cli.deliveryFailed": "本机交付未完成；原操作 {operationId} 与请求文件 {path} 已保留，不重复提交业务效果。",
  "cli.requestFileRequired": "此命令可能产生持久或外部效果，须指定新的 request-file；首次发送前保存准确输入，不包含凭据。",
  "cli.invalidInput": "输入不完整、格式或 UTF-8 无效，或超出预算；未发送请求。",
  "cli.invalidEndpoint": "远端须明确使用不含用户信息、查询参数或片段的 HTTPS origin；不接受跨 origin 重定向。",
  "cli.credentialsRequired": "远端凭据从受保护文件或明确的环境变量读取；不接受包含密钥的命令行参数。",
  "cli.requestConflict": "原请求定位器或内容不一致，未重新发送。",
  "cli.unsupported": "当前执行端或命令未提供此能力。",
  "cli.outputExists": "输出路径 {path} 已存在，拒绝覆盖原件。",
  "cli.helpCommands": "使用 --help 查看命令；使用 <命令> --help 查看该命令的参数。"
} as const;

export type MessageKey = keyof typeof zhCN;
export interface MessageParamsMap {
  "app.title": {  };
  "app.hostedTitle": {  };
  "locale.label": {  };
  "locale.zhCN": {  };
  "locale.enUS": {  };
  "locale.unsaved": {  };
  "locale.unavailable": {  };
  "common.save": {  };
  "common.cancel": {  };
  "common.close": {  };
  "common.retry": {  };
  "common.open": {  };
  "common.unknown": {  };
  "common.unavailable": {  };
  "common.originalDiagnostic": {  };
  "common.operation": { operationId: string };
  "desktop.library": {  };
  "desktop.capture": {  };
  "desktop.showLibrary": {  };
  "desktop.exit": {  };
  "desktop.resident": {  };
  "desktop.loginStartup": {  };
  "desktop.openInterface": {  };
  "desktop.unsavedClose": {  };
  "desktop.waitingForInput": {  };
  "desktop.startupDisabled": {  };
  "capture.title": {  };
  "capture.body": {  };
  "capture.target": {  };
  "capture.saveClose": {  };
  "capture.saveNext": {  };
  "capture.openNote": {  };
  "capture.saved": {  };
  "capture.persisting": {  };
  "capture.pending": {  };
  "reminder.label": {  };
  "reminder.absolute": {  };
  "reminder.beforeDue": {  };
  "reminder.dueDay": {  };
  "reminder.enabledDevice": {  };
  "reminder.submitted": {  };
  "reminder.unknown": {  };
  "reminder.unavailable": {  };
  "reminder.missed": { count: number };
  "editor.source": {  };
  "editor.revealedSource": {  };
  "editor.search": {  };
  "editor.replace": {  };
  "error.request": { status: number };
  "error.resourceRead": {  };
  "cli.description": {  };
  "cli.requestRetained": { path: string; operationId: string };
  "cli.complete": {  };
  "cli.partial": {  };
  "cli.unknown": { operationId: string; path: string };
  "cli.error": { code: string };
  "cli.deliveryFailed": { operationId: string; path: string };
  "cli.requestFileRequired": {  };
  "cli.invalidInput": {  };
  "cli.invalidEndpoint": {  };
  "cli.credentialsRequired": {  };
  "cli.requestConflict": {  };
  "cli.unsupported": {  };
  "cli.outputExists": { path: string };
  "cli.helpCommands": {  };
}
export const messageParamTypes = {
  "app.title": {},
  "app.hostedTitle": {},
  "locale.label": {},
  "locale.zhCN": {},
  "locale.enUS": {},
  "locale.unsaved": {},
  "locale.unavailable": {},
  "common.save": {},
  "common.cancel": {},
  "common.close": {},
  "common.retry": {},
  "common.open": {},
  "common.unknown": {},
  "common.unavailable": {},
  "common.originalDiagnostic": {},
  "common.operation": {
    "operationId": "text"
  },
  "desktop.library": {},
  "desktop.capture": {},
  "desktop.showLibrary": {},
  "desktop.exit": {},
  "desktop.resident": {},
  "desktop.loginStartup": {},
  "desktop.openInterface": {},
  "desktop.unsavedClose": {},
  "desktop.waitingForInput": {},
  "desktop.startupDisabled": {},
  "capture.title": {},
  "capture.body": {},
  "capture.target": {},
  "capture.saveClose": {},
  "capture.saveNext": {},
  "capture.openNote": {},
  "capture.saved": {},
  "capture.persisting": {},
  "capture.pending": {},
  "reminder.label": {},
  "reminder.absolute": {},
  "reminder.beforeDue": {},
  "reminder.dueDay": {},
  "reminder.enabledDevice": {},
  "reminder.submitted": {},
  "reminder.unknown": {},
  "reminder.unavailable": {},
  "reminder.missed": {
    "count": "integer"
  },
  "editor.source": {},
  "editor.revealedSource": {},
  "editor.search": {},
  "editor.replace": {},
  "error.request": {
    "status": "integer"
  },
  "error.resourceRead": {},
  "cli.description": {},
  "cli.requestRetained": {
    "path": "text",
    "operationId": "text"
  },
  "cli.complete": {},
  "cli.partial": {},
  "cli.unknown": {
    "operationId": "text",
    "path": "text"
  },
  "cli.error": {
    "code": "text"
  },
  "cli.deliveryFailed": {
    "operationId": "text",
    "path": "text"
  },
  "cli.requestFileRequired": {},
  "cli.invalidInput": {},
  "cli.invalidEndpoint": {},
  "cli.credentialsRequired": {},
  "cli.requestConflict": {},
  "cli.unsupported": {},
  "cli.outputExists": {
    "path": "text"
  },
  "cli.helpCommands": {}
} as const;

export const enUS: Readonly<Record<MessageKey, string>> = {
  "app.title": "Weftext · Writing and knowledge",
  "app.hostedTitle": "Weftext · Remote writing",
  "locale.label": "Interface language",
  "locale.zhCN": "简体中文",
  "locale.enUS": "English",
  "locale.unsaved": "The interface language has not been durably saved. Your draft and original operation are unchanged.",
  "locale.unavailable": "The interface language preference is unavailable. Using the default language; your draft and original operation are unchanged.",
  "common.save": "Save",
  "common.cancel": "Cancel",
  "common.close": "Close",
  "common.retry": "Retry",
  "common.open": "Open",
  "common.unknown": "Unknown",
  "common.unavailable": "Unavailable",
  "common.originalDiagnostic": "Original diagnostic",
  "common.operation": "Original operation: {operationId}",
  "desktop.library": "Library",
  "desktop.capture": "New capture",
  "desktop.showLibrary": "Open library window",
  "desktop.exit": "Exit Weftext",
  "desktop.resident": "Keep running after closing windows",
  "desktop.loginStartup": "Start Weftext at login",
  "desktop.openInterface": "Open the interface at startup",
  "desktop.unsavedClose": "The recovery draft is not confirmed durable. The window remains open.",
  "desktop.waitingForInput": "Waiting for input to finish and the draft to be saved.",
  "desktop.startupDisabled": "The system has disabled login startup. Weftext will not re-enable it automatically.",
  "capture.title": "New capture",
  "capture.body": "Body",
  "capture.target": "Target library",
  "capture.saveClose": "Save and close",
  "capture.saveNext": "Save and capture another",
  "capture.openNote": "Open created note",
  "capture.saved": "Capture saved.",
  "capture.persisting": "Saving the recovery draft durably.",
  "capture.pending": "The original operation is unconfirmed. Its draft and original ID are retained.",
  "reminder.label": "Task reminder",
  "reminder.absolute": "Independent instant",
  "reminder.beforeDue": "Before the due time",
  "reminder.dueDay": "Time on the due day",
  "reminder.enabledDevice": "Enable this reminder on this device",
  "reminder.submitted": "The system accepted the reminder. This does not mean it was read.",
  "reminder.unknown": "The system submission result is unknown. It was not blindly resubmitted.",
  "reminder.unavailable": "System notifications cannot be submitted under the current conditions.",
  "reminder.missed": "Valid missed reminders: {count}.",
  "editor.source": "Typst source editor",
  "editor.revealedSource": "Native Typst source fragment",
  "editor.search": "Find",
  "editor.replace": "Replace",
  "error.request": "The request did not complete ({status}).",
  "error.resourceRead": "The resource could not be read.",
  "cli.description": "Command-line access for reading, writing, querying, interchange and synchronization.",
  "cli.requestRetained": "The exact original request is retained at {path}; original operation: {operationId}.",
  "cli.complete": "Command completed.",
  "cli.partial": "Only the reported scope is complete. Other scope is not counted as successful.",
  "cli.unknown": "The effect of original operation {operationId} is unconfirmed. Its request is retained at {path}; inspect the original ID.",
  "cli.error": "The command did not complete. Code: {code}.",
  "cli.deliveryFailed": "Local delivery is incomplete. Original operation {operationId} and request file {path} are retained; the business effect is not resubmitted.",
  "cli.requestFileRequired": "This command can have a persistent or external effect. Provide a new request-file to retain the exact input before its first submission, without credentials.",
  "cli.invalidInput": "Input is incomplete, has invalid format or UTF-8, or exceeds its budget. No request was sent.",
  "cli.invalidEndpoint": "A remote endpoint requires an explicit HTTPS origin without user information, query or fragment. Cross-origin redirects are rejected.",
  "cli.credentialsRequired": "Read remote credentials from a protected file or an explicit environment variable. Secret command-line arguments are not accepted.",
  "cli.requestConflict": "The original request locator or content differs. It was not resubmitted.",
  "cli.unsupported": "This endpoint or command does not provide this capability.",
  "cli.outputExists": "Output path {path} already exists. The original is not overwritten.",
  "cli.helpCommands": "Use --help to list commands, or <command> --help for that command's arguments."
};
