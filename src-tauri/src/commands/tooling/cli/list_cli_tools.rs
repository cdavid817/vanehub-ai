use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::cli::api::CliApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_cli_tools(
    api: State<'_, CliApi>,
) -> Result<Vec<dto::CliToolStatus>, CommandError> {
    api.list_tools()
        .map(|statuses| statuses.into_iter().map(mapper::status_to_dto).collect())
        .map_err(map_command_error)
}
