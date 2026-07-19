use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::sdk::api::SdkApi;
use tauri::State;

#[tauri::command]
pub(crate) fn check_sdk_environment(
    api: State<'_, SdkApi>,
) -> Result<dto::SdkEnvironmentStatus, CommandError> {
    api.check_environment()
        .map(mapper::environment_to_dto)
        .map_err(map_command_error)
}
