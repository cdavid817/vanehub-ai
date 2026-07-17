use crate::mcp::connection;
use crate::mcp::models::*;
use crate::mcp::service;
use crate::logging;
use crate::tasks::models::OperationKind;
use crate::tasks::registry::TaskRegistry;
use crate::{AppError, RegistryStore};
use rusqlite::Connection;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn list_mcp_servers(
    state: State<'_, Mutex<RegistryStore>>,
) -> Result<Vec<McpServerConfig>, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::list_servers(&conn)
}

#[tauri::command]
pub fn add_mcp_server(
    state: State<'_, Mutex<RegistryStore>>,
    config: McpServerConfig,
) -> Result<(), AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::add_server(&conn, config)
}

#[tauri::command]
pub fn update_mcp_server(
    state: State<'_, Mutex<RegistryStore>>,
    name: String,
    config: PartialMcpServerConfig,
) -> Result<(), AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::update_server(&conn, &name, config)
}

#[tauri::command]
pub fn remove_mcp_server(
    state: State<'_, Mutex<RegistryStore>>,
    name: String,
) -> Result<(), AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::remove_server(&conn, &name)
}

#[tauri::command]
pub fn toggle_mcp_server(
    state: State<'_, Mutex<RegistryStore>>,
    name: String,
    active: bool,
) -> Result<(), AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::toggle_server(&conn, &name, active)
}

#[tauri::command]
pub async fn test_mcp_connection(
    app: AppHandle,
    state: State<'_, Mutex<RegistryStore>>,
    registry: State<'_, TaskRegistry>,
    name: String,
) -> Result<crate::tasks::models::OperationTask, AppError> {
    let config = {
        let store = state
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?;
        let conn = store.connection()?;
        service::get_server_from_db(&conn, &name)?
    };

    let task = registry.start(
        OperationKind::Mcp,
        Some(name.clone()),
        Some(format!("Testing MCP server {name}")),
    )?;
    let operation_id = task.id.clone();

    tauri::async_runtime::spawn(async move {
        let mut result = connection::test_connection(&config).await;
        result.operation_id = Some(operation_id.clone());

        let registry = app.state::<TaskRegistry>();
        let persist_result = {
            let store = app.state::<Mutex<RegistryStore>>();
            let result = (|| -> Result<(), AppError> {
                let store = store
                    .lock()
                    .map_err(|error| AppError::Storage(error.to_string()))?;
                let conn = store.connection()?;
                service::record_test_result(&conn, &name, &result)?;
                write_mcp_operation_log(
                    &conn,
                    &operation_id,
                    &name,
                    if result.success {
                        "MCP connection test passed"
                    } else {
                        result.error.as_deref().unwrap_or("MCP connection test failed")
                    },
                    if result.success {
                        logging::LogLevel::Info
                    } else {
                        logging::LogLevel::Warn
                    },
                )?;
                Ok(())
            })();
            result
        };

        if let Err(error) = persist_result {
            let _ = registry.append_log(&operation_id, error.to_string());
        }

        if result.success {
            let _ = registry.append_log(&operation_id, format!("MCP test passed for {name}"));
            let _ = registry.complete(&operation_id, serde_json::to_value(&result).ok());
        } else {
            let error = result
                .error
                .clone()
                .unwrap_or_else(|| "MCP test failed".to_string());
            let _ = registry.append_log(&operation_id, error.clone());
            let _ = registry.fail(&operation_id, error);
        }
    });

    Ok(task)
}

#[tauri::command]
pub fn get_mcp_server_status(
    state: State<'_, Mutex<RegistryStore>>,
    name: String,
) -> Result<McpServerStatus, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::get_server_status(&conn, &name)
}

#[tauri::command]
pub fn import_mcp_servers(
    state: State<'_, Mutex<RegistryStore>>,
    data: McpImportExport,
    scope: McpScope,
) -> Result<McpImportResult, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::import_servers(&conn, data, scope)
}

#[tauri::command]
pub fn export_mcp_servers(
    state: State<'_, Mutex<RegistryStore>>,
    names: Vec<String>,
) -> Result<McpImportExport, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::export_servers(&conn, names)
}

fn write_mcp_operation_log(
    conn: &Connection,
    operation_id: &str,
    server_name: &str,
    message: &str,
    level: logging::LogLevel,
) -> Result<(), AppError> {
    let mut context = BTreeMap::new();
    context.insert("operationId".to_string(), operation_id.to_string());
    context.insert("serverName".to_string(), server_name.to_string());
    logging::write_message(
        &active_log_dir_from_conn(conn),
        level,
        "mcp.operation",
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
