use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_session_recovery_summary(
    api: State<'_, SessionsApi>,
    session_id: String,
) -> Result<dto::SessionRecoverySummary, CommandError> {
    api.recovery_summary(&session_id)
        .and_then(mapper::recovery_summary_to_dto)
        .map_err(map_command_error)
}
