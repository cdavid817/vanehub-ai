use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 100;

#[tauri::command]
pub(crate) fn list_session_recovery_reports(
    api: State<'_, SessionsApi>,
    session_id: String,
    limit: Option<usize>,
) -> Result<Vec<dto::SessionRecoveryReport>, CommandError> {
    api.list_recovery_reports(
        &session_id,
        limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT),
    )
    .map(|reports| reports.iter().map(mapper::recovery_report_to_dto).collect())
    .map_err(map_command_error)
}
