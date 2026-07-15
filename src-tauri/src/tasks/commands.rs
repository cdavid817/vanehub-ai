use crate::tasks::models::OperationTask;
use crate::tasks::registry::TaskRegistry;
use crate::AppError;
use tauri::State;

#[tauri::command]
pub fn list_operations(registry: State<'_, TaskRegistry>) -> Result<Vec<OperationTask>, AppError> {
    registry.list()
}

#[tauri::command]
pub fn get_operation_status(
    registry: State<'_, TaskRegistry>,
    operation_id: String,
) -> Result<OperationTask, AppError> {
    registry.get(&operation_id)
}
