use crate::sdk::models::*;
use crate::sdk::service;
use crate::tasks::models::OperationKind;
use crate::tasks::registry::TaskRegistry;
use crate::{logging, AppError, RegistryStore};
use rusqlite::Connection;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

#[tauri::command]
pub fn list_sdk_definitions() -> Result<Vec<SdkDefinition>, AppError> {
    Ok(service::definitions())
}

#[tauri::command]
pub fn list_sdk_statuses() -> Result<SdkStatusMap, AppError> {
    service::list_statuses()
}

#[tauri::command]
pub fn check_sdk_environment() -> Result<SdkEnvironmentStatus, AppError> {
    Ok(service::check_environment())
}

#[tauri::command]
pub fn get_sdk_versions(sdk_id: Option<SdkId>) -> Result<SdkVersionMap, AppError> {
    Ok(service::get_versions(sdk_id))
}

#[tauri::command]
pub fn check_sdk_updates(sdk_id: Option<SdkId>) -> Result<SdkUpdateMap, AppError> {
    service::check_updates(sdk_id)
}

#[tauri::command]
pub fn install_sdk_dependency(
    registry: State<'_, TaskRegistry>,
    store: State<'_, Mutex<RegistryStore>>,
    request: SdkOperationRequest,
) -> Result<SdkOperationResult, AppError> {
    run_sdk_operation(&registry, &store, request.sdk_id, SdkOperationType::Install, || {
        service::install(request, SdkOperationType::Install)
    })
}

#[tauri::command]
pub fn update_sdk_dependency(
    registry: State<'_, TaskRegistry>,
    store: State<'_, Mutex<RegistryStore>>,
    request: SdkOperationRequest,
) -> Result<SdkOperationResult, AppError> {
    run_sdk_operation(&registry, &store, request.sdk_id, SdkOperationType::Update, || {
        service::install(request, SdkOperationType::Update)
    })
}

#[tauri::command]
pub fn rollback_sdk_dependency(
    registry: State<'_, TaskRegistry>,
    store: State<'_, Mutex<RegistryStore>>,
    request: SdkOperationRequest,
) -> Result<SdkOperationResult, AppError> {
    run_sdk_operation(
        &registry,
        &store,
        request.sdk_id,
        SdkOperationType::Rollback,
        || service::install(request, SdkOperationType::Rollback),
    )
}

#[tauri::command]
pub fn uninstall_sdk_dependency(
    registry: State<'_, TaskRegistry>,
    store: State<'_, Mutex<RegistryStore>>,
    sdk_id: SdkId,
) -> Result<SdkOperationResult, AppError> {
    run_sdk_operation(&registry, &store, sdk_id, SdkOperationType::Uninstall, || {
        service::uninstall(sdk_id)
    })
}

#[tauri::command]
pub fn get_sdk_operation_logs(sdk_id: Option<SdkId>) -> Result<Vec<SdkOperationLog>, AppError> {
    Ok(service::operation_logs(sdk_id))
}

fn run_sdk_operation(
    registry: &TaskRegistry,
    store: &Mutex<RegistryStore>,
    sdk_id: SdkId,
    operation: SdkOperationType,
    run: impl FnOnce() -> SdkOperationResult,
) -> Result<SdkOperationResult, AppError> {
    let task = registry.start(
        OperationKind::Sdk,
        Some(sdk_id.as_str().to_string()),
        Some(format!("{operation:?} SDK operation")),
    )?;
    let mut result = run();
    result.operation_id = Some(task.id.clone());

    for log in &result.logs {
        let _ = registry.append_log(&task.id, log.line.clone());
        let _ = write_sdk_operation_log(store, &task.id, log);
    }

    if result.success {
        let _ = registry.complete(&task.id, None);
    } else {
        let _ = registry.fail(
            &task.id,
            result
                .error
                .clone()
                .unwrap_or_else(|| "SDK operation failed".to_string()),
        );
    }

    Ok(result)
}

fn write_sdk_operation_log(
    store: &Mutex<RegistryStore>,
    operation_id: &str,
    log: &SdkOperationLog,
) -> Result<(), AppError> {
    let conn = store
        .lock()
        .map_err(|err| AppError::Storage(err.to_string()))?
        .connection()?;
    let mut context = BTreeMap::new();
    context.insert("operationId".to_string(), operation_id.to_string());
    context.insert("sdkId".to_string(), log.sdk_id.as_str().to_string());
    context.insert("operation".to_string(), format!("{:?}", log.operation));
    logging::write_message(
        &active_log_dir_from_conn(&conn),
        logging::LogLevel::Info,
        "sdk.operation",
        &log.line,
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
            "vanehub-sdk-command-test-{name}-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    #[test]
    fn sdk_operation_log_persists_through_unified_logging() {
        let root = temp_dir("operation-log");
        let store = Mutex::new(RegistryStore::new(root.clone()).expect("store"));
        let log = SdkOperationLog {
            sdk_id: SdkId::ClaudeSdk,
            operation: SdkOperationType::Install,
            line: "Installing token=secret".to_string(),
            timestamp: "now".to_string(),
        };

        write_sdk_operation_log(&store, "op-1", &log).expect("write operation log");

        let raw = std::fs::read_to_string(root.join("logs").join("vanehub.log")).expect("log file");
        assert!(raw.contains("sdk.operation"));
        assert!(raw.contains("claude-sdk"));
        assert!(raw.contains("[REDACTED]"));
        assert!(!raw.contains("token=secret"));
        let _ = std::fs::remove_dir_all(root);
    }
}
