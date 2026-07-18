use crate::plugin_integrations::models::*;
use crate::plugin_integrations::service;
use crate::{logging, AppError, RegistryStore};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

#[tauri::command]
pub fn get_plugin_integration_overview() -> Result<PluginIntegrationOverview, AppError> {
    Ok(service::overview())
}

#[tauri::command]
pub fn refresh_plugin_integrations() -> Result<PluginIntegrationOverview, AppError> {
    Ok(service::overview())
}

#[tauri::command]
pub fn test_plugin_integration(
    store: State<'_, Mutex<RegistryStore>>,
    request: PluginIntegrationRequest,
) -> Result<PluginIntegrationTestResult, AppError> {
    let log_dir = active_log_dir_from_store(&store)?;
    service::test_readiness(request.integration_id, log_dir)
}

fn active_log_dir_from_store(store: &Mutex<RegistryStore>) -> Result<PathBuf, AppError> {
    let conn = store
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?
        .connection()?;
    Ok(active_log_dir_from_conn(&conn))
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
