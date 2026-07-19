use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::DesktopSettingsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn report_client_log_event(
    api: State<'_, DesktopSettingsApi>,
    event: dto::ClientLogEvent,
) -> Result<(), CommandError> {
    api.report_client_log(mapper::client_log_to_domain(event))
        .map_err(map_command_error)
}
