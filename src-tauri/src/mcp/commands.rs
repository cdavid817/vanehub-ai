use crate::mcp::connection;
use crate::mcp::models::*;
use crate::mcp::service;
use crate::{AppError, RegistryStore};
use std::sync::Mutex;
use tauri::State;

#[tauri::command]
pub fn list_mcp_servers(
    state: State<'_, Mutex<RegistryStore>>,
) -> Result<Vec<McpServerConfig>, AppError> {
    let store = state.lock().map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::list_servers(&conn)
}

#[tauri::command]
pub fn add_mcp_server(
    state: State<'_, Mutex<RegistryStore>>,
    config: McpServerConfig,
) -> Result<(), AppError> {
    let store = state.lock().map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::add_server(&conn, config)
}

#[tauri::command]
pub fn update_mcp_server(
    state: State<'_, Mutex<RegistryStore>>,
    name: String,
    config: PartialMcpServerConfig,
) -> Result<(), AppError> {
    let store = state.lock().map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::update_server(&conn, &name, config)
}

#[tauri::command]
pub fn remove_mcp_server(
    state: State<'_, Mutex<RegistryStore>>,
    name: String,
) -> Result<(), AppError> {
    let store = state.lock().map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::remove_server(&conn, &name)
}

#[tauri::command]
pub fn toggle_mcp_server(
    state: State<'_, Mutex<RegistryStore>>,
    name: String,
    active: bool,
) -> Result<(), AppError> {
    let store = state.lock().map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::toggle_server(&conn, &name, active)
}

#[tauri::command]
pub async fn test_mcp_connection(
    state: State<'_, Mutex<RegistryStore>>,
    name: String,
) -> Result<McpTestResult, AppError> {
    let config = {
        let store = state.lock().map_err(|error| AppError::Storage(error.to_string()))?;
        let conn = store.connection()?;
        service::get_server_from_db(&conn, &name)?
    };

    let result = connection::test_connection(&config).await;

    {
        let store = state.lock().map_err(|error| AppError::Storage(error.to_string()))?;
        let conn = store.connection()?;
        service::record_test_result(&conn, &name, &result)?;
    }

    Ok(result)
}

#[tauri::command]
pub fn get_mcp_server_status(
    state: State<'_, Mutex<RegistryStore>>,
    name: String,
) -> Result<McpServerStatus, AppError> {
    let store = state.lock().map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::get_server_status(&conn, &name)
}

#[tauri::command]
pub fn import_mcp_servers(
    state: State<'_, Mutex<RegistryStore>>,
    data: McpImportExport,
    scope: McpScope,
) -> Result<McpImportResult, AppError> {
    let store = state.lock().map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::import_servers(&conn, data, scope)
}

#[tauri::command]
pub fn export_mcp_servers(
    state: State<'_, Mutex<RegistryStore>>,
    names: Vec<String>,
) -> Result<McpImportExport, AppError> {
    let store = state.lock().map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    service::export_servers(&conn, names)
}
