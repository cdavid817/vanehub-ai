use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::desktop::api::DesktopSettingsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_data_management_info(
    api: State<'_, DesktopSettingsApi>,
) -> Result<dto::DataManagementInfo, CommandError> {
    api.data_management_info()
        .map(mapper::data_information_to_dto)
        .map_err(map_command_error)
}
