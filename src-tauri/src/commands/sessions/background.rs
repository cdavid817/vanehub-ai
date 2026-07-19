use super::events;
use crate::contexts::sessions::api::{PreparedNewSessionCreation, SessionsApi};
use tauri::AppHandle;

pub(super) fn spawn_creation(
    app: AppHandle,
    api: SessionsApi,
    prepared: PreparedNewSessionCreation,
) {
    tauri::async_runtime::spawn_blocking(move || {
        if let Ok(session) = api.execute_creation(prepared) {
            events::emit_active_session_changed(&app, Some(session.id()));
        }
    });
}
