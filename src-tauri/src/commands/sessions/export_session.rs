use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::{SessionExportRequest, SessionsApi};
use tauri::State;

#[tauri::command]
pub(crate) fn export_session(
    api: State<'_, SessionsApi>,
    session_id: String,
    format: dto::SessionExportFormat,
    destination_directory: Option<String>,
) -> Result<dto::SessionExportResult, CommandError> {
    api.export(SessionExportRequest {
        session_id,
        format: mapper::export_format(format),
        destination_directory,
    })
    .map(mapper::export_result_to_dto)
    .map_err(map_command_error)
}
