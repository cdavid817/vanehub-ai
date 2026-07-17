use std::sync::Mutex;

use tauri::State;

use crate::{session_tabs, AppError, RegistryStore};

#[tauri::command]
pub(crate) fn list_session_documents(
    state: State<'_, Mutex<RegistryStore>>,
    session_id: String,
) -> Result<session_tabs::DocumentListing, AppError> {
    session_tabs::list_session_documents(state, session_id)
}
