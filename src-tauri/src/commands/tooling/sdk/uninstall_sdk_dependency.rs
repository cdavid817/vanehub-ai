use super::{background, dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::operations::api::OperationTask;
use crate::contexts::tooling::sdk::api::SdkApi;
use tauri::State;

#[tauri::command]
pub(crate) fn uninstall_sdk_dependency(
    api: State<'_, SdkApi>,
    sdk_id: dto::SdkId,
) -> Result<OperationTask, CommandError> {
    let prepared = api
        .prepare_operation(mapper::uninstall_request(sdk_id))
        .map_err(map_command_error)?;
    let operation = mapper::started_operation_to_dto(&prepared.operation);
    background::spawn_operation(api.inner().clone(), prepared);
    Ok(operation)
}
