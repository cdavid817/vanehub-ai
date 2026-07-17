use std::sync::Mutex;

use tauri::State;

use crate::{session_tabs, AppError, RegistryStore};

#[tauri::command]
pub(crate) fn get_session_git_diff(
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
    path: String,
    source: session_tabs::GitDiffSource,
) -> Result<session_tabs::GitDiffResult, AppError> {
    session_tabs::get_session_git_diff(state, session_id, path, source)
}
