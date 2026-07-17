use std::sync::Mutex;

use tauri::State;

use crate::{session_tabs, AppError, RegistryStore};

#[tauri::command]
pub(crate) fn read_session_file(
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
    path: String,
) -> Result<session_tabs::FileContent, AppError> {
    session_tabs::read_session_file(state, session_id, path)
}
