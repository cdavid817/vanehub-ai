use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn search_sessions(
    api: State<'_, SessionsApi>,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<dto::SessionSearchResult>, CommandError> {
    api.search(&query, limit)
        .and_then(mapper::search_results_to_dto)
        .map_err(map_command_error)
}
