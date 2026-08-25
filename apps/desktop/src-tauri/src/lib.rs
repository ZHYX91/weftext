// Tauri's command macro emits underscore-prefixed internal bindings at the function return span.
#![cfg_attr(windows, allow(clippy::used_underscore_binding))]

#[cfg(any(windows, test))]
mod agent_lifecycle;
#[cfg(any(windows, test))]
mod draft_store;
#[cfg(any(windows, test))]
mod local_workspace;

#[cfg(windows)]
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::sync::{Arc, Mutex, MutexGuard};

#[cfg(windows)]
use serde_json::{json, Value};
#[cfg(windows)]
use tauri::{AppHandle, Manager, State};
#[cfg(windows)]
use tauri_plugin_dialog::DialogExt;
#[cfg(windows)]
use weftext_import::CancellationToken;

#[cfg(windows)]
use local_workspace::{BackupPathCapabilityKind, DesktopBackend};

#[cfg(windows)]
#[derive(Clone)]
struct DesktopState {
    backend: Arc<Mutex<DesktopBackend>>,
    active_import: Arc<Mutex<Option<CancellationToken>>>,
}

#[cfg(windows)]
impl DesktopState {
    fn new(backend: DesktopBackend) -> Self {
        Self {
            backend: Arc::new(Mutex::new(backend)),
            active_import: Arc::new(Mutex::new(None)),
        }
    }

    fn lock(&self) -> Result<MutexGuard<'_, DesktopBackend>, String> {
        self.backend
            .lock()
            .map_err(|_| "desktop workspace state is unavailable".to_owned())
    }

    fn begin_import(&self) -> Result<CancellationToken, String> {
        let mut active = self
            .active_import
            .lock()
            .map_err(|_| "desktop import cancellation state is unavailable".to_owned())?;
        if active.is_some() {
            return Err("已有一个本地导入任务正在运行；请等待或先取消".to_owned());
        }
        let token = CancellationToken::default();
        *active = Some(token.clone());
        Ok(token)
    }

    fn finish_import(&self) -> Result<(), String> {
        let mut active = self
            .active_import
            .lock()
            .map_err(|_| "desktop import cancellation state is unavailable".to_owned())?;
        *active = None;
        Ok(())
    }

    fn cancel_import(&self, body: Option<&Value>) -> Result<Value, String> {
        if body.is_some() {
            return Err("取消导入请求不接受字段".to_owned());
        }
        let active = self
            .active_import
            .lock()
            .map_err(|_| "desktop import cancellation state is unavailable".to_owned())?;
        let requested = active.as_ref().is_some_and(|token| {
            token.cancel();
            true
        });
        Ok(json!({"ok": true, "cancelRequested": requested}))
    }

    fn execute_request(&self, path: &str, body: Option<Value>) -> Result<Value, String> {
        let route = path.split('?').next().unwrap_or(path);
        if route == "/api/import/cancel" {
            return self.cancel_import(body.as_ref());
        }
        if matches!(
            route,
            "/api/import/pdf-preview" | "/api/import/markdown/preview" | "/api/import/task/preview"
        ) {
            let cancellation = self.begin_import()?;
            let result = self.lock().and_then(|mut backend| {
                backend.request_with_import_cancellation(path, body, cancellation)
            });
            let finish = self.finish_import();
            return match (result, finish) {
                (Ok(payload), Ok(())) => Ok(payload),
                (Err(error), _) | (Ok(_), Err(error)) => Err(error),
            };
        }
        self.lock()?.request(path, body)
    }
}

#[cfg(windows)]
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn restore_workspace(state: State<'_, DesktopState>) -> Result<Value, String> {
    state.lock()?.restore_workspace()
}

#[cfg(windows)]
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn open_workspace(path: String, state: State<'_, DesktopState>) -> Result<Value, String> {
    state.lock()?.open_workspace(&PathBuf::from(path))
}

#[cfg(windows)]
#[allow(clippy::needless_pass_by_value, clippy::used_underscore_binding)]
#[tauri::command]
async fn choose_markdown_export_destination(
    suggested_name: String,
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<Option<Value>, String> {
    let extension = Path::new(&suggested_name)
        .extension()
        .and_then(|value| value.to_str());
    if suggested_name.is_empty()
        || suggested_name.len() > 255
        || suggested_name.chars().any(char::is_control)
        || suggested_name
            .chars()
            .any(|character| matches!(character, '/' | '\\' | ':'))
        || extension.is_none_or(|value| {
            !value.eq_ignore_ascii_case("md") && !value.eq_ignore_ascii_case("markdown")
        })
    {
        return Err("Markdown 导出建议名称不是安全的单一文件名".to_owned());
    }
    let selected = app
        .dialog()
        .file()
        .set_title("导出 unmanaged Markdown")
        .set_file_name(suggested_name)
        .add_filter("Markdown", &["md", "markdown"])
        .blocking_save_file();
    let Some(selected) = selected else {
        return Ok(None);
    };
    let destination = selected
        .into_path()
        .map_err(|error| format!("系统选择器返回了无效路径：{error}"))?;
    state
        .lock()?
        .register_markdown_export_destination(destination)
        .map(Some)
}

#[cfg(windows)]
#[allow(clippy::needless_pass_by_value, clippy::used_underscore_binding)]
#[tauri::command]
async fn choose_task_import_receipt_destination(
    suggested_name: String,
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<Option<Value>, String> {
    let extension = Path::new(&suggested_name)
        .extension()
        .and_then(|value| value.to_str());
    if suggested_name.is_empty()
        || suggested_name.len() > 255
        || suggested_name.chars().any(char::is_control)
        || suggested_name
            .chars()
            .any(|character| matches!(character, '/' | '\\' | ':'))
        || extension.is_none_or(|value| !value.eq_ignore_ascii_case("json"))
    {
        return Err("Task import receipt 建议名称不是安全的单一 JSON 文件名".to_owned());
    }
    let selected = app
        .dialog()
        .file()
        .set_title("保存 task import receipt")
        .set_file_name(suggested_name)
        .add_filter("JSON receipt", &["json"])
        .blocking_save_file();
    let Some(selected) = selected else {
        return Ok(None);
    };
    let destination = selected
        .into_path()
        .map_err(|error| format!("系统选择器返回了无效路径：{error}"))?;
    state
        .lock()?
        .register_task_import_receipt_destination(destination)
        .map(Some)
}

#[cfg(windows)]
#[allow(clippy::needless_pass_by_value, clippy::used_underscore_binding)]
#[tauri::command]
async fn choose_backup_directory(
    kind: BackupPathCapabilityKind,
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<Option<Value>, String> {
    let title = match kind {
        BackupPathCapabilityKind::BackupParent => "选择备份存放目录",
        BackupPathCapabilityKind::Snapshot => "选择 Weftext 备份快照目录",
        BackupPathCapabilityKind::RestoreParent => "选择 alternate restore 父目录",
        BackupPathCapabilityKind::DrillParent => "选择恢复演练工作目录",
        BackupPathCapabilityKind::DrillResultsParent => "选择恢复演练结果目录",
    };
    let selected = app.dialog().file().set_title(title).blocking_pick_folder();
    let Some(selected) = selected else {
        return Ok(None);
    };
    let selected = selected
        .into_path()
        .map_err(|error| format!("系统选择器返回了无效路径：{error}"))?;
    state
        .lock()?
        .register_backup_path_capability(kind, &selected)
        .map(Some)
}

#[cfg(windows)]
#[allow(clippy::needless_pass_by_value, clippy::used_underscore_binding)]
#[tauri::command]
async fn desktop_request(
    path: String,
    body: Option<Value>,
    state: State<'_, DesktopState>,
) -> Result<Value, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.execute_request(&path, body))
        .await
        .map_err(|error| format!("desktop request worker failed: {error}"))?
}

/// Starts the native Weftext desktop shell.
///
/// # Panics
///
/// Panics when the native application runtime cannot be initialized or exits
/// with an unrecoverable error.
#[cfg(windows)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config_dir = app
                .path()
                .app_config_dir()
                .map_err(|error| format!("could not locate desktop settings: {error}"))?;
            let docling_installation_root = app
                .path()
                .resource_dir()
                .map_err(|error| format!("could not locate desktop resources: {error}"))?
                .join("docling-lite");
            app.manage(DesktopState::new(
                DesktopBackend::new_with_docling_installation(
                    config_dir,
                    docling_installation_root,
                ),
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            restore_workspace,
            open_workspace,
            choose_markdown_export_destination,
            choose_task_import_receipt_destination,
            choose_backup_directory,
            desktop_request
        ])
        .run(tauri::generate_context!())
        .expect("Weftext desktop runtime failed");
}

/// Reports that the current Stage 1C package is Windows-only.
#[cfg(not(windows))]
pub fn run() {
    eprintln!("Weftext Desktop Alpha is currently packaged for Windows only");
}

#[cfg(all(test, windows))]
mod tests {
    use serde_json::json;
    use tempfile::tempdir;

    use super::{DesktopBackend, DesktopState};

    #[test]
    fn active_import_cancellation_is_independent_bounded_and_closed() {
        let config = tempdir().expect("temp config");
        let state = DesktopState::new(DesktopBackend::new(config.path().to_path_buf()));
        let token = state.begin_import().expect("first import token");
        assert!(!token.is_cancelled());
        assert!(state.begin_import().is_err(), "a second worker is refused");

        let cancelled = state.cancel_import(None).expect("cancel request");
        assert_eq!(cancelled["cancelRequested"], true);
        assert!(token.is_cancelled());
        let unexpected_body = json!({});
        assert!(state.cancel_import(Some(&unexpected_body)).is_err());

        state.finish_import().expect("finish import");
        let idle = state.cancel_import(None).expect("idle cancellation");
        assert_eq!(idle["cancelRequested"], false);
        state.begin_import().expect("slot was released");
    }
}
