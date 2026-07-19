use super::{background, dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::operations::api::OperationTask;
use crate::contexts::tooling::sdk::api::{SdkApi, SdkOperationType};
use tauri::State;

#[tauri::command]
pub(crate) fn rollback_sdk_dependency(
    api: State<'_, SdkApi>,
    request: dto::SdkOperationRequest,
) -> Result<OperationTask, CommandError> {
    let prepared = api
        .prepare_operation(mapper::operation_request(
            request,
            SdkOperationType::Rollback,
        ))
        .map_err(map_command_error)?;
    let operation = mapper::started_operation_to_dto(&prepared.operation);
    background::spawn_operation(api.inner().clone(), prepared);
    Ok(operation)
}
