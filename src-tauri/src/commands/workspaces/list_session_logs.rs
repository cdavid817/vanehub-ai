use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::workspaces::api::WorkspaceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_session_logs(
    api: State<'_, WorkspaceApi>,
    input: dto::SessionLogQuery,
) -> Result<dto::SessionLogPage, CommandError> {
    let query = mapper::session_log_query_from_dto(input);
    api.list_session_logs(&query)
        .map(mapper::session_log_page_to_dto)
        .map_err(map_command_error)
}
