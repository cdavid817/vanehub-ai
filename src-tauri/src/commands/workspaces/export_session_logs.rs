use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) async fn export_session_logs(
    api: State<'_, WorkspaceApi>,
    input: dto::SessionLogQuery,
) -> Result<dto::SessionLogExportResult, CommandError> {
    let query = mapper::session_log_query_from_dto(input);
    api.export_session_logs_blocking(query)
        .await
        .map(mapper::session_log_export_to_dto)
        .map_err(map_command_error)
}
