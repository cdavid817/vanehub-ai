use super::dto;
use crate::commands::error::CommandError;
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::sessions::infrastructure::scheduled_tasks::{self, ScheduledTaskLogDirectory};
use crate::platform::database::NativeDatabase;
use tauri::State;

#[tauri::command]
pub(crate) fn list_scheduled_tasks(
    database: State<'_, NativeDatabase>,
) -> Result<Vec<dto::ScheduledTask>, CommandError> {
    scheduled_tasks::list_scheduled_tasks(&database)
}

#[tauri::command]
pub(crate) fn list_scheduled_task_runs(
    database: State<'_, NativeDatabase>,
    task_id: String,
) -> Result<Vec<dto::ScheduledTaskRun>, CommandError> {
    scheduled_tasks::list_scheduled_task_runs(&database, &task_id)
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

/// 19.8: version-checked edit of name/content/agent/frequency. See
/// `scheduled_tasks::update_scheduled_task`'s own doc comment for the conflict contract.
#[tauri::command]
pub(crate) fn update_scheduled_task(
    database: State<'_, NativeDatabase>,
    input: dto::UpdateScheduledTaskInput,
) -> Result<dto::ScheduledTask, CommandError> {
    scheduled_tasks::update_scheduled_task(&database, input)
}

#[tauri::command]
pub(crate) fn delete_scheduled_task(
    database: State<'_, NativeDatabase>,
    log_directory: State<'_, ScheduledTaskLogDirectory>,
    task_id: String,
) -> Result<(), CommandError> {
    scheduled_tasks::delete_scheduled_task(&database, &task_id, Some(log_directory.path()))
}

/// 19.10: dispatches a task's content on demand, independent of its recurrence.
///
/// `SessionsApi` and `AgentRuntimeApi` are already registered as managed state in their own
/// right (`bootstrap/runtime.rs` `.manage()`s both, the same instances `start_scheduled_task_jobs`
/// hands to the due-task sweep), so this takes them directly as `State<'_, T>` alongside the
/// database rather than introducing a new composed API type for a single call site.
#[tauri::command]
pub(crate) fn run_scheduled_task_now(
    database: State<'_, NativeDatabase>,
    sessions: State<'_, SessionsApi>,
    agents: State<'_, AgentRuntimeApi>,
    task_id: String,
) -> Result<dto::RunScheduledTaskNowResult, CommandError> {
    scheduled_tasks::run_scheduled_task_now(&database, &sessions, &agents, &task_id)
}
