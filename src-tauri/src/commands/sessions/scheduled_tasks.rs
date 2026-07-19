use super::dto;
use crate::commands::error::CommandError;
use crate::contexts::sessions::infrastructure::scheduled_tasks::{
    self, ScheduledTaskLogDirectory,
};
use crate::platform::database::NativeDatabase;
use tauri::State;

#[tauri::command]
pub(crate) fn list_scheduled_tasks(
    database: State<'_, NativeDatabase>,
) -> Result<Vec<dto::ScheduledTask>, CommandError> {
    scheduled_tasks::list_scheduled_tasks(&database)
}

#[tauri::command]
pub(crate) fn create_scheduled_task(
    database: State<'_, NativeDatabase>,
    input: dto::CreateScheduledTaskInput,
) -> Result<dto::ScheduledTask, CommandError> {
    scheduled_tasks::create_scheduled_task(&database, input)
}

#[tauri::command]
pub(crate) fn set_scheduled_task_enabled(
    database: State<'_, NativeDatabase>,
    input: dto::SetScheduledTaskEnabledInput,
) -> Result<dto::ScheduledTask, CommandError> {
    scheduled_tasks::set_scheduled_task_enabled(&database, input)
}

#[tauri::command]
pub(crate) fn delete_scheduled_task(
    database: State<'_, NativeDatabase>,
    log_directory: State<'_, ScheduledTaskLogDirectory>,
    task_id: String,
) -> Result<(), CommandError> {
    scheduled_tasks::delete_scheduled_task(&database, &task_id, Some(log_directory.path()))
}
