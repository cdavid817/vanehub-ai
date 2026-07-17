use std::sync::Mutex;

use tauri::State;

use crate::{session_tabs, AppError, RegistryStore};

#[tauri::command]
pub(crate) fn list_session_logs(
    state: State<'_, Mutex<RegistryStore>>,
    input: session_tabs::SessionLogQuery,
) -> Result<session_tabs::SessionLogPage, AppError> {
    session_tabs::list_session_logs(state, input)
}
