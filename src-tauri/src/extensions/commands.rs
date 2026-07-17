use crate::extensions::models::*;
use crate::extensions::service;
use crate::tasks::models::{OperationKind, OperationTask};
use crate::tasks::registry::TaskRegistry;
use crate::{logging, AppError, RegistryStore};
use rusqlite::Connection;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Child;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

#[derive(Default)]
pub struct ExtensionRuntimeManager {
    active_operations: Mutex<HashSet<ExtensionFrameworkId>>,
    children: Mutex<HashMap<ExtensionFrameworkId, Child>>,
}

impl ExtensionRuntimeManager {
    fn begin(&self, id: ExtensionFrameworkId) -> Result<(), AppError> {
        let mut active = self
            .active_operations
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?;
        if !active.insert(id) {
            return Err(AppError::Validation(format!(
                "an extension operation is already running for {}",
                id.as_str()
            )));
        }
        Ok(())
    }

    fn finish(&self, id: ExtensionFrameworkId) {
        if let Ok(mut active) = self.active_operations.lock() {
            active.remove(&id);
        }
    }

    fn running_frameworks(&self) -> Result<HashSet<ExtensionFrameworkId>, AppError> {
        let mut children = self
            .children
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?;
        children.retain(|_, child| matches!(child.try_wait(), Ok(None)));
        Ok(children.keys().copied().collect())
    }

    fn start(&self, id: ExtensionFrameworkId, port: u16) -> Result<Vec<String>, AppError> {
        if self.running_frameworks()?.contains(&id) {
            return Ok(vec!["Framework sidecar is already running".to_string()]);
        }
        let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|_| {
            AppError::Validation(format!(
                "loopback port {port} is already owned by another process"
            ))
        })?;
        drop(listener);
        let mut child = service::spawn_health_sidecar(id, port)?;
        thread::sleep(Duration::from_millis(350));
        if TcpStream::connect_timeout(
            &format!("127.0.0.1:{port}").parse().map_err(|error| {
                AppError::Validation(format!("invalid extension port: {error}"))
            })?,
            Duration::from_secs(2),
        )
        .is_err()
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::LaunchFailed(
                "extension sidecar did not become healthy".to_string(),
            ));
        }
        let pid = child.id();
        self.children
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?
            .insert(id, child);
        Ok(vec![format!(
            "Started owned loopback sidecar pid={pid} port={port}"
        )])
    }

    fn stop(&self, id: ExtensionFrameworkId) -> Result<Vec<String>, AppError> {
        let child = self
            .children
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?
            .remove(&id);
        let Some(mut child) = child else {
            return Ok(vec!["No owned sidecar process was running".to_string()]);
        };
        let pid = child.id();
        child
            .kill()
            .map_err(|error| AppError::LaunchFailed(error.to_string()))?;
        let _ = child.wait();
        Ok(vec![format!("Stopped owned sidecar pid={pid}")])
    }
}

#[tauri::command]
pub fn get_extension_overview(
    store: State<'_, Mutex<RegistryStore>>,
    runtime: State<'_, ExtensionRuntimeManager>,
) -> Result<ExtensionOverview, AppError> {
    let conn = connection(&store)?;
    service::overview(&conn, &runtime.running_frameworks()?)
}

#[tauri::command]
pub fn refresh_extension_health(
    store: State<'_, Mutex<RegistryStore>>,
    runtime: State<'_, ExtensionRuntimeManager>,
) -> Result<ExtensionOverview, AppError> {
    let conn = connection(&store)?;
    let running = runtime.running_frameworks()?;
    for definition in service::definitions() {
        if running.contains(&definition.id) {
            let _ = service::set_running(&conn, definition.id, true, None);
        }
    }
    service::overview(&conn, &running)
}

#[tauri::command]
pub fn get_extension_install_preview(
    request: ExtensionFrameworkRequest,
) -> Result<ExtensionInstallPreview, AppError> {
    service::install_preview(request.framework_id)
}

#[tauri::command]
pub fn install_extension(
    app: AppHandle,
    registry: State<'_, TaskRegistry>,
    runtime: State<'_, ExtensionRuntimeManager>,
    request: ExtensionFrameworkRequest,
) -> Result<OperationTask, AppError> {
    start_operation(
        app,
        &registry,
        &runtime,
        request.framework_id,
        ExtensionAction::Install,
    )
}

#[tauri::command]
pub fn uninstall_extension(
    app: AppHandle,
    registry: State<'_, TaskRegistry>,
    runtime: State<'_, ExtensionRuntimeManager>,
    request: ExtensionFrameworkRequest,
) -> Result<OperationTask, AppError> {
    start_operation(
        app,
        &registry,
        &runtime,
        request.framework_id,
        ExtensionAction::Uninstall,
    )
}

#[tauri::command]
pub fn set_extension_enabled(
    app: AppHandle,
    registry: State<'_, TaskRegistry>,
    runtime: State<'_, ExtensionRuntimeManager>,
    request: ExtensionEnableRequest,
) -> Result<OperationTask, AppError> {
    start_operation(
        app,
        &registry,
        &runtime,
        request.framework_id,
        if request.enabled {
            ExtensionAction::Enable
        } else {
            ExtensionAction::Disable
        },
    )
}

#[tauri::command]
pub fn start_extension(
    app: AppHandle,
    registry: State<'_, TaskRegistry>,
    runtime: State<'_, ExtensionRuntimeManager>,
    request: ExtensionFrameworkRequest,
) -> Result<OperationTask, AppError> {
    start_operation(
        app,
        &registry,
        &runtime,
        request.framework_id,
        ExtensionAction::Start,
    )
}

#[tauri::command]
pub fn stop_extension(
    app: AppHandle,
    registry: State<'_, TaskRegistry>,
    runtime: State<'_, ExtensionRuntimeManager>,
    request: ExtensionFrameworkRequest,
) -> Result<OperationTask, AppError> {
    start_operation(
        app,
        &registry,
        &runtime,
        request.framework_id,
        ExtensionAction::Stop,
    )
}

#[tauri::command]
pub fn test_extension(
    app: AppHandle,
    registry: State<'_, TaskRegistry>,
    runtime: State<'_, ExtensionRuntimeManager>,
    request: ExtensionFrameworkRequest,
) -> Result<OperationTask, AppError> {
    start_operation(
        app,
        &registry,
        &runtime,
        request.framework_id,
        ExtensionAction::SelfTest,
    )
}

fn start_operation(
    app: AppHandle,
    registry: &TaskRegistry,
    runtime: &ExtensionRuntimeManager,
    id: ExtensionFrameworkId,
    action: ExtensionAction,
) -> Result<OperationTask, AppError> {
    runtime.begin(id)?;
    let task = match registry.start(
        OperationKind::Extension,
        Some(id.as_str().to_string()),
        Some(format!("{action:?} local extension")),
    ) {
        Ok(task) => task,
        Err(error) => {
            runtime.finish(id);
            return Err(error);
        }
    };
    let operation_id = task.id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        run_operation(app, operation_id, id, action);
    });
    Ok(task)
}

fn run_operation(
    app: AppHandle,
    operation_id: String,
    id: ExtensionFrameworkId,
    action: ExtensionAction,
) {
    let registry = app.state::<TaskRegistry>();
    let runtime = app.state::<ExtensionRuntimeManager>();
    let store = app.state::<Mutex<RegistryStore>>();
    let result = match connection(&store) {
        Ok(conn) => {
            let _ = service::set_transition(&conn, id, transition_for(action), &operation_id);
            execute(&conn, &runtime, id, action)
        }
        Err(error) => ExtensionOperationResult {
            success: false,
            framework_id: id,
            action,
            message: error.to_string(),
            logs: Vec::new(),
            error: Some(error.to_string()),
        },
    };

    for line in &result.logs {
        let _ = registry.append_log(&operation_id, line.clone());
        let _ = write_operation_log(
            &store,
            &operation_id,
            id,
            action,
            line,
            logging::LogLevel::Info,
        );
    }
    if result.success {
        let _ = registry.complete(&operation_id, serde_json::to_value(&result).ok());
    } else {
        let error = result
            .error
            .clone()
            .unwrap_or_else(|| result.message.clone());
        if let Ok(conn) = connection(&store) {
            let _ = service::mark_error(&conn, id, &error);
        }
        let _ = write_operation_log(
            &store,
            &operation_id,
            id,
            action,
            &error,
            logging::LogLevel::Error,
        );
        let _ = registry.fail(&operation_id, error);
    }
    runtime.finish(id);
}

fn execute(
    conn: &Connection,
    runtime: &ExtensionRuntimeManager,
    id: ExtensionFrameworkId,
    action: ExtensionAction,
) -> ExtensionOperationResult {
    match action {
        ExtensionAction::Install => service::install(conn, id),
        ExtensionAction::Uninstall => {
            if runtime
                .running_frameworks()
                .map(|items| items.contains(&id))
                .unwrap_or(true)
            {
                failure(id, action, "Stop the framework before uninstalling")
            } else {
                service::uninstall(conn, id)
            }
        }
        ExtensionAction::Enable => service::set_enabled(conn, id, true),
        ExtensionAction::Disable => service::set_enabled(conn, id, false),
        ExtensionAction::SelfTest => service::self_test(conn, id),
        ExtensionAction::Start => start_runtime(conn, runtime, id),
        ExtensionAction::Stop => stop_runtime(conn, runtime, id),
    }
}

fn start_runtime(
    conn: &Connection,
    runtime: &ExtensionRuntimeManager,
    id: ExtensionFrameworkId,
) -> ExtensionOperationResult {
    let status = match service::status_for(conn, id) {
        Ok(status) => status,
        Err(error) => return failure(id, ExtensionAction::Start, &error.to_string()),
    };
    if !status.installed {
        return failure(
            id,
            ExtensionAction::Start,
            "Framework must be installed before starting",
        );
    }
    match runtime.start(id, status.port) {
        Ok(logs) => {
            if let Err(error) = service::set_running(conn, id, true, None) {
                return failure(id, ExtensionAction::Start, &error.to_string());
            }
            ExtensionOperationResult {
                success: true,
                framework_id: id,
                action: ExtensionAction::Start,
                message: "Framework started".to_string(),
                logs,
                error: None,
            }
        }
        Err(error) => failure(id, ExtensionAction::Start, &error.to_string()),
    }
}

fn stop_runtime(
    conn: &Connection,
    runtime: &ExtensionRuntimeManager,
    id: ExtensionFrameworkId,
) -> ExtensionOperationResult {
    match runtime.stop(id) {
        Ok(logs) => {
            if let Err(error) = service::set_running(conn, id, false, None) {
                return failure(id, ExtensionAction::Stop, &error.to_string());
            }
            ExtensionOperationResult {
                success: true,
                framework_id: id,
                action: ExtensionAction::Stop,
                message: "Framework stopped".to_string(),
                logs,
                error: None,
            }
        }
        Err(error) => failure(id, ExtensionAction::Stop, &error.to_string()),
    }
}

fn transition_for(action: ExtensionAction) -> ExtensionLifecycleStatus {
    match action {
        ExtensionAction::Install => ExtensionLifecycleStatus::Installing,
        ExtensionAction::Uninstall => ExtensionLifecycleStatus::Uninstalling,
        ExtensionAction::Start => ExtensionLifecycleStatus::Starting,
        ExtensionAction::Stop => ExtensionLifecycleStatus::Stopping,
        _ => ExtensionLifecycleStatus::Installed,
    }
}

fn failure(
    id: ExtensionFrameworkId,
    action: ExtensionAction,
    message: &str,
) -> ExtensionOperationResult {
    ExtensionOperationResult {
        success: false,
        framework_id: id,
        action,
        message: message.to_string(),
        logs: Vec::new(),
        error: Some(message.to_string()),
    }
}

fn connection(store: &Mutex<RegistryStore>) -> Result<Connection, AppError> {
    store
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?
        .connection()
}

fn write_operation_log(
    store: &Mutex<RegistryStore>,
    operation_id: &str,
    id: ExtensionFrameworkId,
    action: ExtensionAction,
    message: &str,
    level: logging::LogLevel,
) -> Result<(), AppError> {
    let conn = connection(store)?;
    let mut context = BTreeMap::new();
    context.insert("operationId".to_string(), operation_id.to_string());
    context.insert("frameworkId".to_string(), id.as_str().to_string());
    context.insert("action".to_string(), format!("{action:?}"));
    logging::write_message(
        &active_log_dir_from_conn(&conn),
        level,
        "extension.operation",
        message,
        context,
    )
}

fn active_log_dir_from_conn(conn: &Connection) -> PathBuf {
    PathBuf::from(
        super::super::load_setting_value(conn, "logDirectory")
            .ok()
            .flatten()
            .unwrap_or_else(|| {
                conn.path()
                    .and_then(|path| PathBuf::from(path).parent().map(logging::default_log_dir))
                    .unwrap_or_else(super::super::fallback_log_dir)
                    .to_string_lossy()
                    .to_string()
            }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "vanehub-extension-command-test-{name}-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    #[test]
    fn operation_lock_rejects_concurrent_mutation_for_framework() {
        let manager = ExtensionRuntimeManager::default();
        manager
            .begin(ExtensionFrameworkId::Paddleocr)
            .expect("first operation");
        assert!(manager.begin(ExtensionFrameworkId::Paddleocr).is_err());
        manager.finish(ExtensionFrameworkId::Paddleocr);
        assert!(manager.begin(ExtensionFrameworkId::Paddleocr).is_ok());
    }

    #[test]
    fn start_refuses_foreign_loopback_listener() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener");
        let port = listener.local_addr().expect("address").port();
        let manager = ExtensionRuntimeManager::default();
        let error = manager
            .start(ExtensionFrameworkId::Paddleocr, port)
            .expect_err("foreign listener");
        assert!(error.to_string().contains("owned by another process"));
    }

    #[test]
    fn operation_logs_use_unified_redaction_without_feature_log_file() {
        let root = temp_dir("logging");
        let store = Mutex::new(RegistryStore::new(root.clone()).expect("store"));
        write_operation_log(
            &store,
            "op-1",
            ExtensionFrameworkId::Paddleocr,
            ExtensionAction::Install,
            "Downloading token=secret password:hunter2",
            logging::LogLevel::Info,
        )
        .expect("write log");

        let raw = std::fs::read_to_string(root.join("logs").join("vanehub.log"))
            .expect("unified log file");
        assert!(raw.contains("extension.operation"));
        assert!(raw.contains("paddleocr"));
        assert!(raw.contains("[REDACTED]"));
        assert!(!raw.contains("hunter2"));
        assert!(!root.join("extensions.log").exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
