use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::api::CommunicationsApi;
use crate::contexts::communications::domain::RoutingSettings;
use tauri::State;

#[tauri::command]
pub(crate) fn get_im_routing(
    api: State<'_, CommunicationsApi>,
) -> Result<Option<RoutingSettings>, CommandError> {
    api.routing().map_err(map_command_error)
}
