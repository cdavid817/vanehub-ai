use super::{dto, session_log_mapper};
use crate::commands::error::CommandError;
use crate::contexts::operations::log_api::SessionLogApi;
use tauri::State;

/// Reads one page of session logs from the operations-owned index.
///
/// The command name and the DTO are unchanged; what moved is where the rows come from. There is no
/// fallback to scanning log files: a fallback would be a second query implementation with different
/// filters, different bounds and different coverage semantics, reached exactly when a reader is
/// least able to tell which one answered. When the index cannot answer, it says so.
#[tauri::command]
pub(crate) async fn list_session_logs(
    logs: State<'_, SessionLogApi>,
    input: dto::SessionLogQuery,
) -> Result<dto::SessionLogPage, CommandError> {
    logs.query_blocking(session_log_mapper::indexed_query_from_dto(input))
        .await
        .map(session_log_mapper::indexed_page_to_dto)
        .map_err(session_log_mapper::log_command_error)
}
