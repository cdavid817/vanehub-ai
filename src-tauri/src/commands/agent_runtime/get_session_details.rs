use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_session_details(
    api: State<'_, AgentRuntimeApi>,
) -> Result<dto::SessionDetails, CommandError> {
    api.session_details()
        .map(mapper::session_details_to_dto)
        .map_err(map_command_error)
}
