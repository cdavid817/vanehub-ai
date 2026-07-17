use std::sync::Mutex;

use tauri::State;

use crate::{session_tabs, AppError, RegistryStore};

#[tauri::command]
pub(crate) fn get_session_git_status(
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
) -> Result<session_tabs::GitStatusResult, AppError> {
    session_tabs::get_session_git_status(state, session_id)
}
