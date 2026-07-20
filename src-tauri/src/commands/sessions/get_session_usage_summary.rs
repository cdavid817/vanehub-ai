use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_session_usage_summary(
    api: State<'_, SessionsApi>,
    session_id: String,
) -> Result<dto::SessionUsageSummary, CommandError> {
    api.session_usage_summary(&session_id)
        .map(mapper::session_usage_summary_to_dto)
        .map_err(map_command_error)
}
