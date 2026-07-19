use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::operations::api::{OperationTask, OperationsApi};
use tauri::State;

#[tauri::command]
pub(crate) fn list_operations(
    api: State<'_, OperationsApi>,
) -> Result<Vec<OperationTask>, CommandError> {
    api.list().map_err(map_command_error)
}
