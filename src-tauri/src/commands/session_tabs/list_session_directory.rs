use std::sync::Mutex;

use tauri::State;

use crate::{session_tabs, AppError, RegistryStore};

#[tauri::command]
pub(crate) fn list_session_directory(
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
    path: String,
) -> Result<session_tabs::DirectoryListing, AppError> {
    session_tabs::list_session_directory(state, session_id, path)
}
