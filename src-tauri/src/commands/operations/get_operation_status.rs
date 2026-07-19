use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::operations::api::{OperationTask, OperationsApi};
use tauri::State;

#[tauri::command]
pub(crate) fn get_operation_status(
    api: State<'_, OperationsApi>,
    operation_id: String,
) -> Result<OperationTask, CommandError> {
    api.get(&operation_id).map_err(map_command_error)
}
