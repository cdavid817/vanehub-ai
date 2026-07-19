use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::api::CommunicationsApi;
use crate::contexts::communications::domain::RoutingSettings;
use tauri::State;

#[tauri::command]
pub(crate) fn save_im_routing(
    api: State<'_, CommunicationsApi>,
    routing: RoutingSettings,
) -> Result<RoutingSettings, CommandError> {
    api.save_routing(&routing).map_err(map_command_error)
}
