use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn send_message(
    api: State<'_, AgentRuntimeApi>,
    session_id: String,
    content: String,
    config: dto::ChatConfig,
    file_references: Option<Vec<dto::ChatFileReference>>,
) -> Result<dto::ChatMessage, CommandError> {
    api.send_message(mapper::send_message_request(
        session_id,
        content,
        config,
        file_references,
    ))
    .map(mapper::message_to_dto)
    .map_err(map_command_error)
}
