use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::permissions::api::PermissionsApi;
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_session_chat_config(
    api: State<'_, SessionsApi>,
    permissions: State<'_, PermissionsApi>,
    session_id: String,
) -> Result<dto::ChatConfig, CommandError> {
    let configuration = api
        .load_chat_configuration(&session_id)
        .map_err(map_command_error)?;
    let (principal, _) = permissions
        .find_principal(&configuration.agent_id)
        .map_err(map_command_error)?;
    mapper::chat_configuration_to_dto(configuration, principal.template())
        .map_err(map_command_error)
}
