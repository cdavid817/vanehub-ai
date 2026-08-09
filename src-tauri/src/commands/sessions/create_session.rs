use super::{background, dto, mapper};
use crate::bootstrap::ScheduledTaskLogDirectory;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::operations::api::OperationTask;
use crate::contexts::retrieval::api::CodeIndexApi;
use crate::contexts::sessions::api::SessionsApi;
use tauri::{AppHandle, State};

#[tauri::command]
pub(crate) fn create_session(
    app: AppHandle,
    api: State<'_, SessionsApi>,
    code_index: State<'_, CodeIndexApi>,
    log_directory: State<'_, ScheduledTaskLogDirectory>,
    input: dto::CreateSessionInput,
) -> Result<OperationTask, CommandError> {
    let prepared = api
        .prepare_creation(mapper::creation_request(input))
        .map_err(map_command_error)?;
    let operation = mapper::creation_operation_to_dto(&prepared.operation);
    background::spawn_creation(
        app,
        api.inner().clone(),
        code_index.inner().clone(),
        log_directory.path().to_path_buf(),
        prepared,
    );
    Ok(operation)
}
