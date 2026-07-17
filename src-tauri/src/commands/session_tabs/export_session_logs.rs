use std::sync::Mutex;

use tauri::{AppHandle, State};

use crate::{session_tabs, AppError, RegistryStore};

#[tauri::command]
pub(crate) fn export_session_logs(
    app: AppHandle,
    state: State<'_, Mutex<RegistryStore>>,
    input: session_tabs::SessionLogQuery,
) -> Result<session_tabs::SessionLogExportResult, AppError> {
    session_tabs::export_session_logs(app, state, input)
}
