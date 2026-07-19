use super::{background, dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::operations::api::OperationTask;
use crate::contexts::sessions::api::SessionsApi;
use tauri::{AppHandle, State};

#[tauri::command]
pub(crate) fn create_session(
    app: AppHandle,
    api: State<'_, SessionsApi>,
    input: dto::CreateSessionInput,
) -> Result<OperationTask, CommandError> {
    let prepared = api
        .prepare_creation(mapper::creation_request(input))
        .map_err(map_command_error)?;
    let operation = mapper::creation_operation_to_dto(&prepared.operation);
    background::spawn_creation(app, api.inner().clone(), prepared);
    Ok(operation)
}
